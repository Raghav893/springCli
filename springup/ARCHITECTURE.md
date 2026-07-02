# Architecture

This document describes `springup`'s internal architecture for contributors.

## Workspace layout

```
springup/
├── Cargo.toml                      # workspace root + shared [workspace.dependencies]
├── crates/
│   ├── springup-cli/               # binary crate — arg parsing, wizard, output
│   │   ├── src/
│   │   │   ├── main.rs             # thin shim: calls springup_cli::run()
│   │   │   ├── lib.rs              # library form for assert_cmd tests
│   │   │   ├── cli.rs              # clap derive definitions + dispatch
│   │   │   ├── commands/           # new.rs, add.rs, config.rs, completions.rs, update.rs, update_metadata.rs
│   │   │   ├── wizard/             # dialoguer-based interactive prompts
│   │   │   └── ui/                 # logging, theme, summary, messages
│   │   └── tests/                  # integration tests with mocked Initializr (wiremock)
│   ├── springup-core/              # library crate — pure logic, fully unit-testable
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── plan.rs             # ProjectPlan + BuildTool/Language/Packaging/ExtraFeature
│   │   │   ├── error.rs            # thiserror error model + fuzzy suggestion
│   │   │   ├── initializr/         # client.rs (HTTP) + cache.rs (disk) + models.rs (typed JSON)
│   │   │   ├── template.rs         # minijinja renderer + TemplateContext
│   │   │   ├── manifest.rs         # springup.toml read/write
│   │   │   └── config.rs           # global user config + precedence merge
│   │   └── tests/                  # snapshot tests (insta)
│   └── springup-templates/         # embedded asset crate (rust-embed)
│       ├── src/lib.rs
│       └── assets/                 # *.j2 templates + raw assets (.dockerignore, .gitignore)
├── xtask/                          # cargo-xtask: template-asset health checks
└── .github/workflows/              # ci.yml + release.yml + audit.yml
```

### Why three crates?

- **`springup-core`** is pure logic — no TTY, no terminal colors, no `clap`. Every function
  is unit-testable and could in principle be reused by a WASM playground or a non-CLI tool.
- **`springup-cli`** wraps core with prompts, spinners, colored output, and arg parsing.
  `main.rs` is intentionally a thin shim.
- **`springup-templates`** is a tiny crate whose only job is to embed asset files at compile
  time via `rust-embed`. Splitting it out means `springup-core` doesn't need to own the asset
  list — templates can be added without touching core logic.

## Data flow

```
User runs `springup new`
        │
        ▼
[Mode detection] ── flags + TTY? ──► skip wizard
        │ no TTY or partial flags
        ▼
[Interactive Wizard] (dialoguer)
   1. Project metadata (group, artifact, name, package, description)
   2. Spring Boot version (from cached metadata)
   3. Build tool
   4. Language
   5. Java version
   6. Packaging
   7. Dependencies (searchable multi-select, grouped by category)
   8. Architecture skeleton (none / layered / hexagonal)
   9. Extras multi-select (docker, docker-compose, ci, config-profiles, editorconfig, readme)
        │
        ▼
[Resolve & Plan] — ProjectPlan struct + validation
        │
        ▼
[Fetch base project from Initializr] (zip stream) ──► extract to output dir (zip-slip safe)
        │
        ▼
[Apply custom template layer]
   - Architecture skeleton (layered / hexagonal) files
   - Dockerfile / docker-compose.yml / .dockerignore
   - .github/workflows/ci.yml
   - application-{dev,prod}.yml
   - .editorconfig, .gitignore, README.md
        │
        ▼
[Write springup.toml manifest]
        │
        ▼
[Optional git init + initial commit]
        │
        ▼
[Print summary + next-step commands]
```

Both paths (wizard and flag-driven) converge on the same `ProjectPlan` struct, which is
serialized as `springup.toml` and as the schema for a future `--from-file plan.toml` mode.

## The `ProjectPlan` data model

Defined in `springup-core/src/plan.rs`. Both the wizard and the flag parser populate a
`PlanBuilder`, which is then resolved into a `ProjectPlan` using Initializr metadata
defaults. The plan is validated before any network call to download the starter zip.

Key fields:
- `group_id`, `artifact_id`, `name`, `description`, `package_name` — Maven coordinates.
- `spring_boot_version`, `build_tool`, `language`, `java_version`, `packaging` — Spring Boot
  build settings.
- `dependencies: Vec<DependencyId>` — Initializr dependency ids (validated against metadata).
- `architecture: Option<ArchitectureKind>` — `None` / `Layered` / `Hexagonal`.
- `extras: Vec<ExtraFeature>` — Docker, DockerCompose, GithubActionsCi, ConfigProfiles,
  EditorConfig, Readme.
- `output_dir`, `git_init`, `initial_commit` — output behavior.

`ProjectPlan` derives `Serialize`/`Deserialize` so it can be persisted and re-loaded.

## The Initializr client

Lives in `springup-core/src/initializr/`:

- `models.rs` — typed JSON schema for the Initializr metadata document. Uses
  `#[serde(rename_all = "camelCase")]` to match the API's casing.
- `cache.rs` — disk cache at `~/.cache/springup/initializr-metadata.json`. 24h TTL. Atomic
  write (tmp + rename). Stale-but-usable fallback for `--offline` mode.
- `client.rs` — `reqwest`-based HTTP client with connect/read timeouts and bounded
  exponential-backoff retry (3 attempts) for transient failures. Validates dependency ids
  against metadata with fuzzy-match suggestions.

## The template engine

`springup-core/src/template.rs` wraps a `minijinja::Environment` populated from
`springup-templates::Asset::iter()`. Every `.j2` file under `assets/` is registered at
construction time. `TemplateContext::from_plan(plan)` derives a serializable view of the plan
that templates render against.

The `apply_extras(plan, output_dir)` entry point walks the plan's architecture + extras and
writes each file. Use `AppliedFiles` to inspect what was written (used for summary output
and snapshot tests).

### Adding a new template

1. Drop the `.j2` file (or raw asset) under
   `crates/springup-templates/assets/<category>/<name>`.
2. If templated, use `minijinja` syntax (`{{ var }}`, `{% if cond %}…{% endif %}`). Context
   fields are documented on `TemplateContext` in `template.rs`.
3. If the template should be wired into the extras system, add a new `ExtraFeature` variant
   in `plan.rs`, an `apply_*` method in `template.rs`, and a case in `apply_extras`.
4. Add a snapshot test in `crates/springup-core/tests/snapshot_tests.rs`.
5. Run `cargo insta review` to accept the new snapshot.

### Escaping GitHub Actions `${{ }}` syntax

GitHub Actions uses `${{ expr }}` for expression substitution, which collides with
`minijinja`'s `{{ expr }}`. Inside CI YAML templates, escape with:

```jinja
${{ "{{" }} github.ref {{ "}}" }}
```

This emits the literal `${{ github.ref }}` string.

## The `springup.toml` manifest

Written to the root of every generated project. Defined in `manifest.rs` as
`ProjectManifest` with `[project]`, `[architecture]`, `[extras]`, `[dependencies]` sections.

Inert metadata for v1 (the tool doesn't read it back) — but it's the extension point for
`springup add <module>` in v2. Every field is `serde`-serializable so it round-trips through
TOML cleanly.

## Error handling

- `springup-core` uses `thiserror` with one enum variant per failure mode (`Error` in
  `error.rs`). Each variant carries enough context for the CLI to render an actionable
  message (e.g. `InvalidDependency { id, suggestion }`).
- `springup-cli` uses `color_eyre` at the boundary for ergonomic propagation and pretty
  backtraces in `--verbose` mode.
- `unwrap()`/`expect()` only on genuinely-infallible internal invariants. Every network path
  has a timeout and bounded retry.

## Logging

`tracing` + `tracing-subscriber`. Default level: warn. `-v` → info, `-vv` → debug,
`-vvv` → trace. `--quiet` → errors only. Honors `NO_COLOR` and `--color never`.

## Testing

Three layers:

1. **Unit tests** (`#[cfg(test)] mod tests` inside each core module) — plan validation,
   fuzzy match, manifest round-trip, config merge, cache TTL behavior, metadata parsing.
2. **Integration tests** (`crates/springup-cli/tests/*.rs`) — `assert_cmd` drives the
   compiled binary against a `wiremock`-mocked Initializr. Never hits the real network.
3. **Snapshot tests** (`crates/springup-core/tests/snapshot_tests.rs`) — `insta` snapshots
   for every rendered template. Run `cargo insta accept` after intentionally changing a
   template.

A separate opt-in "smoke test" job in CI hits the live Initializr API on a nightly schedule
to catch upstream API drift — never on every PR.

## Release automation

`cargo-dist` generates the release workflow, install scripts, and Homebrew/Scoop manifests
from `dist-workspace.toml`. Tag a release with `git tag v0.1.0 && git push --tags` and CI
builds binaries for:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl` (static)
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Also published to crates.io as `cargo install springup`.

## Performance

- Cold start to first wizard prompt < 100ms — metadata fetch happens *after* the prompts
  that don't need it (project name, output dir), so perceived latency is hidden.
- `tokio` (minimal features: `rt-multi-thread`, `macros`, `time`, `fs`, `io-util`, `sync`).
- Release profile: `lto = true`, `strip = true`, `codegen-units = 1` for smaller, faster
  binaries. `panic = "abort"` to drop the unwind tables.
