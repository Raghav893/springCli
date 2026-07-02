# Build Spec: `springup` — A Production-Grade Spring Boot Project Scaffolder in Rust

> Hand this entire document to an AI coding assistant (Claude Code, Cursor, etc.) as the
> primary instruction set. It describes a complete, installable CLI tool — not a toy script.
> Sections are ordered so an agent can work top-to-bottom and produce a working `v0.1.0`,
> then iterate toward the full feature set.

---

## 0. One-line pitch

`springup` is what `npm create`, `bun create`, and `cargo generate` are to their ecosystems,
but for Spring Boot: a fast, single static binary that interactively (or non-interactively)
scaffolds a production-ready Spring Boot backend by combining the official Spring Initializr
API with an opinionated custom template layer (architecture skeletons, Docker, CI, config
profiles) — written in Rust, distributed as a real CLI tool.

---

## 1. Goals & non-goals

**Goals**
- Feel as polished and fast as `bun create` / `create-t3-app`: sub-second startup, smooth
  interactive prompts, sensible defaults, zero Java/Maven/Gradle required to just *generate*
  a project.
- Two first-class modes: interactive TUI wizard (default) and fully flag-driven non-interactive
  mode (for CI, scripting, muscle-memory power users).
- Generate real, buildable Spring Boot projects by delegating dependency/version resolution to
  the official Spring Initializr API (`https://start.spring.io`), then layering custom
  extras on top (architecture skeleton, Dockerfile, GitHub Actions, application.yml profiles).
- Be genuinely production-grade as a piece of software: proper error handling, logging,
  tests, CI, versioned releases, cross-platform binaries, shell completions, self-update.
- Support an "add" workflow later (`springup add module`) — so today's design must not paint
  the tool into a one-shot-generator corner. Use a lightweight project config file
  (`springup.toml`) written into generated projects to enable this later without a rewrite.

**Non-goals (explicitly out of scope for v1)**
- No frontend/monorepo scaffolding (no Next.js/React generation). Backend + Docker/CI only.
- No bundling of a JVM, Maven, or Gradle. The tool never requires Java to be installed to
  *generate* a project (it may shell out to `./mvnw`/`./gradlew` only for optional post-gen
  steps like "run tests now?", which are opt-in and gracefully skipped if unavailable).
- No GUI. Terminal only.
- No plugin marketplace/registry in v1 (design for it, don't build it yet).

---

## 2. Prior art to study before writing code

The agent should mentally (or actually, if it has web access) benchmark against:
- `cargo-generate` (Rust) — template mechanics, `{{placeholder}}` substitution patterns.
- `create-t3-app` (TS) — interactive prompt UX, layered "optional add-on" selection.
- `starship` (Rust) — how a beloved Rust CLI handles config file precedence, cross-platform
  installs, and shell completions.
- `bun create` / `npm create` — speed expectations and flag ergonomics.
- Spring Initializr itself (`start.spring.io`) — this tool is a client of its REST API, not a
  replacement for it. Read its API docs (`https://start.spring.io` returns machine-readable
  metadata at the root when queried with `Accept: application/json`).

---

## 3. Core architecture

### 3.1 High-level flow

```
User runs `springup new`
        │
        ▼
[Mode detection] ── flags present & non-interactive? ──► skip wizard, validate flags
        │ no
        ▼
[Interactive Wizard] (ratatui or dialoguer-based prompts)
   1. Project metadata (group, artifact, name, package, description)
   2. Spring Boot version (fetched live from Initializr metadata, cached)
   3. Build tool: Maven | Gradle (Groovy) | Gradle (Kotlin)
   4. Language: Java | Kotlin
   5. Java version
   6. Packaging: Jar | War
   7. Dependencies (multi-select, searchable, grouped by category — from Initializr metadata)
   8. [Custom layer] Architecture skeleton? (none / layered / hexagonal) — optional, user choice
   9. [Custom layer] Extras multi-select: Dockerfile, docker-compose.yml, GitHub Actions CI,
      application-{dev,prod}.yml profiles, .editorconfig, pre-commit hook, README badges
        │
        ▼
[Resolve & Plan] — build a ProjectPlan struct (see §3.3), validate combos
        │
        ▼
[Fetch base project from Initializr] (zip stream) ──► extract to temp dir
        │
        ▼
[Apply custom template layer] — inject architecture folders, Docker/CI files,
   rewrite application.yml, write springup.toml manifest
        │
        ▼
[Post-gen] — git init (optional), initial commit (optional), print summary + next steps
```

### 3.2 Crate/module layout

```
springup/
├── Cargo.toml
├── crates/
│   ├── springup-cli/          # binary crate — arg parsing, wizard orchestration, output
│   │   ├── src/main.rs
│   │   ├── src/cli.rs         # clap definitions
│   │   ├── src/wizard/        # interactive prompt flows
│   │   ├── src/commands/      # new.rs, add.rs, config.rs, completions.rs, update.rs
│   │   └── src/ui/            # terminal rendering helpers, spinners, theming
│   ├── springup-core/         # library crate — pure logic, fully unit-testable, no I/O-heavy CLI concerns
│   │   ├── src/plan.rs        # ProjectPlan struct + validation
│   │   ├── src/initializr/    # Spring Initializr API client + response models
│   │   ├── src/template/      # custom template engine (rendering, placeholder substitution)
│   │   ├── src/manifest.rs    # springup.toml read/write (project-level state)
│   │   └── src/config.rs      # global user config (~/.config/springup/config.toml)
│   └── springup-templates/    # embedded custom templates (architecture skeletons, CI, Docker)
│       └── assets/
│           ├── architectures/layered/
│           ├── architectures/hexagonal/
│           ├── docker/
│           ├── ci/github-actions/
│           └── config-profiles/
├── xtask/                     # cargo-xtask pattern: release automation, template asset checks
└── tests/                     # integration tests (see §8)
```

Rationale for splitting `springup-core` out: it must be usable without a terminal (unit
tests, potential future library consumers, WASM playground, etc.) and keeps `main.rs` thin.

### 3.3 Core data model

```rust
/// The fully-resolved, validated description of what to generate.
/// Built by EITHER the wizard OR flag parsing — both paths converge here.
pub struct ProjectPlan {
    pub group_id: String,
    pub artifact_id: String,
    pub name: String,
    pub description: String,
    pub package_name: String,
    pub spring_boot_version: String,
    pub build_tool: BuildTool,       // Maven | GradleGroovy | GradleKotlin
    pub language: Language,          // Java | Kotlin
    pub java_version: String,
    pub packaging: Packaging,        // Jar | War
    pub dependencies: Vec<DependencyId>,   // Initializr dependency ids, e.g. "web", "data-jpa"
    pub architecture: Option<ArchitectureKind>, // None | Layered | Hexagonal
    pub extras: Vec<ExtraFeature>,   // Docker, DockerCompose, GithubActionsCi, ConfigProfiles, ...
    pub output_dir: PathBuf,
    pub git_init: bool,
    pub initial_commit: bool,
}
```

Keep this struct serializable (`serde`). It doubles as the schema for:
- Non-interactive flags (`clap` derives directly into a builder for this struct).
- A future `--from-file plan.toml` mode (batch/reproducible generation — trivial to add
  later since the plan is already data, not scattered logic).

### 3.4 Spring Initializr integration

- Base URL: `https://start.spring.io`.
- On startup (or on a `springup update-metadata` command / cached with TTL), fetch
  `GET https://start.spring.io` with `Accept: application/json` to get the full metadata
  document: available Spring Boot versions, dependency list with categories/descriptions/
  compatibility ranges, Java versions, packaging options, language options.
- Cache this metadata locally (`~/.cache/springup/initializr-metadata.json` or platform
  equivalent via the `directories` crate) with a TTL (e.g. 24h) so the wizard is instant on
  repeat runs and works offline using stale-but-usable cached data (warn the user it's stale).
- To generate the actual project, call
  `GET https://start.spring.io/starter.zip?type=...&language=...&bootVersion=...&groupId=...&artifactId=...&name=...&packageName=...&packaging=...&javaVersion=...&dependencies=...`
  and stream the zip response to a temp file, then extract.
- Handle Initializr downtime/errors gracefully: clear error message, suggest retry, never
  panic. Consider a `--offline` flag that only works if a previously-downloaded base template
  cache exists (stretch goal, not required for v1).
- Validate dependency IDs against fetched metadata *before* hitting the generate endpoint, so
  users get a fast, friendly "unknown dependency 'foo', did you mean 'flyway'?" instead of a
  raw API error. Implement fuzzy matching (`strsim` crate, e.g. Jaro-Winkler) for suggestions.

### 3.5 Custom template layer (the "extras" system)

This is what differentiates `springup` from a bare Initializr CLI wrapper. Runs *after* the
base project is extracted.

**Architecture skeletons** (optional, chosen interactively per §3.1 step 8):
- `layered`: generates `controller/`, `service/`, `repository/`, `dto/`, `entity/`,
  `exception/` packages under the base package, with:
  - A `GlobalExceptionHandler` using `@RestControllerAdvice`
  - A sample `ApiResponse<T>` wrapper DTO
  - A sample health-check controller wired to the chosen dependencies (e.g. if `data-jpa` is
    selected, include a minimal `Entity`/`Repository`/`Service`/`Controller` CRUD vertical
    slice as a working example, not just empty folders)
- `hexagonal`: `domain/`, `application/`, `adapter/in/web/`, `adapter/out/persistence/` —
  ports-and-adapters skeleton with a sample use case.
- `none`: skip — leave Initializr's default single-package layout untouched.

**Extras** (multi-select, independent of architecture choice):
- `Dockerfile` — multi-stage build (build stage with the right Maven/Gradle image matching
  chosen Java version, slim JRE runtime stage), respects `packaging` (Jar/War).
- `docker-compose.yml` — service block for the app + conditionally a `postgres`/`mysql`/
  `redis` service *if* the corresponding Initializr dependency was selected (detect from
  `dependencies` list: `data-jpa`+`postgresql` driver → add postgres service, etc.).
- GitHub Actions CI (`.github/workflows/ci.yml`) — build + test workflow matching the chosen
  build tool (`mvn -B verify` or `./gradlew build`), with dependency caching configured
  correctly per build tool.
- `application-dev.yml` / `application-prod.yml` — Spring profiles split out from the default
  `application.properties`, with profile activation wired via `SPRING_PROFILES_ACTIVE` and a
  sensible dev-vs-prod default (e.g. dev uses H2/local DB, prod reads from env vars).
- `.editorconfig`, a `.gitignore` tuned for the chosen build tool (if Initializr's default is
  insufficient), and a generated `README.md` with actual project name, run instructions
  matching the build tool, and Docker instructions if Docker extra was selected.

**Template engine mechanics**:
- Use a real templating crate (`tera` or `minijinja` — prefer `minijinja` for smaller
  footprint and no unsafe macro magic) rather than hand-rolled string replace, since files
  need conditionals (include a block only if a dependency is present) and loops (dependency
  list in README).
- All templates live in `springup-templates/assets/` and are embedded into the binary at
  compile time via `rust-embed`, so the shipped binary is fully self-contained — no runtime
  fetch needed for the custom layer (only the Initializr base project requires network).
- Template context = a serializable view derived from `ProjectPlan` (package name, whether
  each relevant dependency is present, build tool, etc.) — pass this into every render call.

### 3.6 `springup.toml` — the project manifest

Written to the root of every generated project. This is the `package.json` equivalent —
it's what makes a future `springup add <module>` possible without re-deriving context.

```toml
[project]
springup_version = "0.1.0"
generated_at = "2026-07-02T10:00:00Z"
group_id = "dev.raghavarora"
artifact_id = "my-service"
package_name = "dev.raghavarora.myservice"
spring_boot_version = "3.5.0"
build_tool = "maven"
language = "java"
java_version = "21"

[architecture]
kind = "layered"

[extras]
enabled = ["docker", "docker-compose", "github-actions-ci", "config-profiles"]

[dependencies]
initializr = ["web", "data-jpa", "postgresql", "validation", "security"]
```

This file is inert metadata for v1 (the tool doesn't read it back for anything beyond a
future `add` command), but it must be written correctly and documented, since it's the
extension point for everything that comes after v1.

---

## 4. CLI surface (commands & flags)

Use `clap` v4 with derive macros. Design the command tree now even though only `new` is
required for v1, so help text and completions are coherent from day one.

```
springup new [NAME]                 Scaffold a new project (interactive if flags omitted)
springup add <MODULE>                [v2+, stub with "not yet implemented" for v1]
springup config get|set|list         Manage global user config
springup completions <SHELL>         Generate shell completion script (bash/zsh/fish/powershell)
springup update                      Self-update the binary to latest release
springup --version / -V
springup --help / -h
```

### `springup new` flags (non-interactive mode)

```
-n, --name <NAME>
    --group-id <GROUP_ID>              default from global config or "com.example"
    --artifact-id <ARTIFACT_ID>
    --description <DESC>
    --boot-version <VERSION>           default: latest stable per Initializr metadata
    --build-tool <maven|gradle|gradle-kotlin>
    --language <java|kotlin>
    --java-version <VERSION>
    --packaging <jar|war>
-d, --deps <DEP,DEP,...>               comma-separated Initializr dependency ids
    --architecture <none|layered|hexagonal>
    --extras <EXTRA,EXTRA,...>         docker,docker-compose,ci,config-profiles
-o, --output <DIR>                     default: ./<artifact-id>
    --git / --no-git                   default: --git
    --commit / --no-commit             default: --no-commit
-y, --yes                              accept all defaults, skip wizard entirely
    --dry-run                          print the resolved ProjectPlan as JSON/TOML, generate nothing
```

**Interaction mode resolution rule** (this must be unambiguous, write it as actual logic, not
vibes): if stdin is not a TTY, OR any of `--yes`/`-y` is passed, OR enough required flags are
present to fully resolve a `ProjectPlan` with no ambiguity, skip the wizard and go straight to
generation, filling any *non-required* gaps with global-config defaults or hardcoded sane
defaults. If flags are partially given but insufficient, run the wizard but pre-fill/skip the
steps already answered by flags (this is the "best of both" UX — don't force an all-or-nothing
choice between flags and wizard).

---

## 5. Interactive wizard UX requirements

- Library choice: `dialoguer` (simpler, more standard for this style of prompt-by-prompt
  wizard than a full `ratatui` app) is likely the right call — this is a sequence of
  select/multi-select/input prompts, not a persistent dashboard. Use `ratatui` only if you
  want a genuinely different, more visual experience (e.g. a dependency-browser split pane);
  otherwise `dialoguer` + `indicatif` (spinners/progress bars) is simpler and battle-tested.
- Every prompt needs: clear question text, sensible pre-selected default, `?`-triggerable
  inline help/description where non-obvious (e.g. showing each dependency's one-line
  description from Initializr metadata next to it in the multi-select).
- Dependency multi-select must be **searchable/filterable** (type to filter the list) — with
  60+ Initializr dependencies, an unfiltered scroll list is a bad experience. Group by
  Initializr's own categories (Web, SQL, NoSQL, Security, Messaging, Ops, etc.) with category
  headers in the list.
- Show a spinner during network calls (metadata fetch, project generation download) with
  elapsed time, and a clear failure state with retry guidance.
- End with a colorized summary: what was generated, file tree (top 2 levels), and copy-paste
  next-step commands (`cd`, build command matching build tool, run command).
- Respect `NO_COLOR` env var and `--no-color` flag. Respect `CLICOLOR_FORCE`.
- All user-facing strings centralized (not scattered `println!`) so the tool could add i18n
  later without a rewrite — a simple `messages.rs` module is enough for v1, no need for a
  full i18n crate.

---

## 6. Production-grade engineering requirements

These are non-negotiable for calling this "production grade" — treat as a checklist.

### 6.1 Error handling
- `thiserror` for typed library errors in `springup-core` (each failure mode is a distinct
  variant: `InitializrUnreachable`, `InvalidDependency { id, suggestion }`,
  `TemplateRenderError`, `IoError`, `InvalidProjectMetadata`, etc.).
- `anyhow` (or `color-eyre` for prettier terminal output) at the `springup-cli` boundary for
  ergonomic error propagation and nicely formatted terminal error reports with backtraces in
  `--verbose` mode.
- Never `unwrap()`/`panic!()` on any path reachable from user input or network I/O. `unwrap()`
  is acceptable only on genuinely-infallible internal invariants, and even then prefer
  `expect("reason")` documenting why it's infallible.
- Every network call has a timeout (`reqwest` client configured with sane connect/read
  timeouts) and a bounded retry with backoff for transient failures (`backoff` crate or
  hand-rolled exponential backoff, 3 attempts max).

### 6.2 Logging & diagnostics
- `tracing` + `tracing-subscriber` for structured logging. `--verbose`/`-v` (repeatable, `-vv`
  for debug, `-vvv` for trace) controls log level. Default: only user-facing progress output,
  no log noise.
- `--dry-run` and `--verbose` should make it possible to fully understand what the tool *would
  do* without needing to read source.

### 6.3 Config management (global user config)
- Location via the `directories` crate (`ProjectDirs::from("dev", "springup", "springup")`)
  → e.g. `~/.config/springup/config.toml` on Linux, correct platform equivalents on
  macOS/Windows.
- Stores: default group-id, default author/org for READMEs, default Java version, default
  build tool, preferred package manager mirror (if ever needed), telemetry opt-in flag
  (default OFF — see §6.7), color preference.
- `springup config set group-id dev.raghavarora`, `springup config get group-id`,
  `springup config list` — implement properly, not just a TODO.
- Precedence, most specific wins: CLI flags > project-local `.springuprc.toml` (if present in
  cwd) > global user config > hardcoded defaults. Document this precedence in `--help` and
  README.

### 6.4 Testing
- Unit tests in `springup-core` for: `ProjectPlan` validation logic, template context
  derivation, dependency-id fuzzy matching, manifest serialization round-trip.
- Integration tests in `tests/` that actually run the compiled binary (`assert_cmd` crate)
  against a **mocked** Initializr server (`wiremock` or `mockito` crate) — never hit the real
  network in CI. Assert on generated file tree structure and key file contents (e.g. Dockerfile
  contains the right base image tag for the chosen Java version).
- Golden-file / snapshot tests (`insta` crate) for generated file contents (Dockerfile,
  GitHub Actions YAML, README) so template regressions are caught immediately on any template
  edit.
- A real (non-mocked) opt-in "smoke test" behind a feature flag or separate CI job that hits
  the live Initializr API once, to catch upstream API drift — run on a schedule (e.g. nightly
  GitHub Actions cron), not on every PR.

### 6.5 CI/CD for the tool itself
- GitHub Actions workflow(s):
  - `ci.yml`: on every PR/push — `cargo fmt --check`, `cargo clippy -- -D warnings`,
    `cargo test --all-features`, run on a matrix of `ubuntu-latest`, `macos-latest`,
    `windows-latest`.
  - `release.yml`: on tag push (`v*`) — build release binaries for at minimum:
    `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` (static, for broad Linux compat),
    `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
    `x86_64-pc-windows-msvc`. Use `cargo-dist` (recommended — it generates the release
    workflow, install scripts, and Homebrew/Scoop manifests for you) rather than hand-rolling
    cross-compilation matrix YAML from scratch.
- `cargo-dist` also generates the `curl | sh` install script and can publish a Homebrew tap
  and Scoop manifest automatically — use it rather than reinventing this.
- Also publish to crates.io (`cargo publish`) as a secondary install path for Rust users who
  prefer `cargo install springup`.

### 6.6 Documentation
- Top-level `README.md`: install instructions for every distribution channel, quickstart GIF
  or asciinema-style terminal recording reference, full command reference, architecture
  overview linking to `ARCHITECTURE.md`.
- `ARCHITECTURE.md`: the module layout from §3.2, the `ProjectPlan` data flow, how to add a
  new "extra" template.
- `CONTRIBUTING.md`: how to run tests, how to add a new architecture skeleton or extra, PR
  checklist (fmt/clippy/tests must pass).
- Every public function/struct in `springup-core` has doc comments (`cargo doc` should
  produce something genuinely useful, not empty stubs).

### 6.7 Privacy / telemetry
- If any telemetry is ever added (usage analytics), it must be **opt-in**, off by default,
  clearly disclosed in README, and disableable via `springup config set telemetry false` and
  an env var (`SPRINGUP_TELEMETRY=0`). For v1, ship with **no telemetry at all** — simplest
  and most trustworthy default; mention this explicitly in README as a stated principle.

### 6.8 Security
- Never execute arbitrary code from downloaded templates (the Initializr zip is data, not
  code — treat it as such; don't `eval`/shell-exec anything extracted from it).
- Validate zip extraction against path traversal (zip-slip vulnerability) — use a
  well-maintained zip crate (`zip` crate) and explicitly sanitize/verify every extracted
  path stays within the target directory before writing.
- Pin dependency versions in `Cargo.lock`, commit it (this is a binary, not a library others
  depend on — lockfile should always be committed).
- Run `cargo audit` in CI as a scheduled job to catch known vulnerabilities in dependencies.

### 6.9 Performance
- Cold start (no cache) to first wizard prompt should feel instant (<100ms) — metadata fetch
  happens *after* the prompts that don't need it (project name, output dir) start, not
  before, so perceived latency is hidden. Fetch metadata concurrently with the first 1-2
  wizard prompts using an async task, `await` it only when the wizard reaches the
  dependency-selection step.
- Use `tokio` (async runtime, since we're doing concurrent network + prompt work) with a
  minimal feature set (`rt-multi-thread`, `macros`, `time` — not the full `full` feature
  flag, to keep binary size and compile time down).
- Release profile: enable LTO and strip symbols in `Cargo.toml` `[profile.release]`
  (`lto = true`, `strip = true`, `codegen-units = 1`) for smaller, faster binaries — the
  build-time cost is worth it for a tool distributed as prebuilt binaries.

---

## 7. Key crate choices (recommended, justify any deviation)

| Concern | Crate |
|---|---|
| CLI parsing | `clap` v4 (derive) |
| Interactive prompts | `dialoguer` |
| Progress/spinners | `indicatif` |
| Async runtime | `tokio` (minimal features) |
| HTTP client | `reqwest` (rustls-tls feature, not native-tls, for easier musl static builds) |
| Templating | `minijinja` |
| Embedded assets | `rust-embed` |
| Zip handling | `zip` |
| Serialization | `serde` + `serde_json` + `toml` |
| Error types (lib) | `thiserror` |
| Error handling (bin) | `color-eyre` |
| Logging | `tracing` + `tracing-subscriber` |
| Config dirs | `directories` |
| Fuzzy matching | `strsim` |
| Colored output | `owo-colors` or `console` (pick one, `dialoguer` already pulls in `console`) |
| Testing (CLI) | `assert_cmd` + `predicates` |
| Testing (mocking) | `wiremock` |
| Testing (snapshots) | `insta` |
| Release automation | `cargo-dist` |

---

## 8. Milestone plan (build in this order)

1. **Skeleton**: workspace with the three crates, `clap` CLI parsing `new` command with all
   flags (no logic yet — just parse and print the resolved struct). CI running fmt/clippy/test
   on empty test suite.
2. **Initializr client**: metadata fetch + cache, `starter.zip` fetch + extraction to temp
   dir, with mocked-server integration tests.
3. **Non-interactive `new`**: full flag-driven path works end-to-end — generates a real,
   buildable Spring Boot project with zero prompts, zero custom extras yet.
4. **Custom template layer**: architecture skeletons + extras, `minijinja` rendering,
   `springup.toml` manifest writing. Snapshot tests for every template.
5. **Interactive wizard**: full `dialoguer` flow wrapping the same `ProjectPlan` builder,
   with searchable dependency multi-select.
6. **Config system**: global config file, precedence resolution, `config` subcommand.
7. **Polish pass**: colorized summary output, `--dry-run`, shell completions, `--verbose`
   logging, error message quality pass (every error should suggest a fix).
8. **Release infrastructure**: `cargo-dist` setup, crates.io publish, README/docs finalized.
9. **v1.0.0 tag.**

Stop and produce a working, tested artifact at the end of *every* milestone — don't build all
modules half-finished in parallel. Each milestone should be independently `cargo test`-green
and independently demoable.

---

## 9. Definition of done for v1.0.0

- [ ] `cargo install springup` and `curl -fsSL <install-script-url> | sh` both work
- [ ] `springup new my-service -y` generates a buildable Spring Boot project with zero prompts
- [ ] `springup new` (no args, TTY present) runs the full interactive wizard end-to-end
- [ ] Generated project builds successfully with the chosen build tool with no manual edits
      (verify with `mvnw`/`gradlew` in integration tests where feasible, or documented manual
      verification if sandboxing Java in CI is out of scope)
- [ ] Dockerfile extra produces an image that builds successfully via `docker build`
- [ ] All six target platform binaries build in release CI
- [ ] `cargo clippy -- -D warnings` and `cargo fmt --check` clean
- [ ] Test coverage on `springup-core` logic paths (plan validation, template rendering,
      dependency matching) — no formal % target required, but every non-trivial function has
      at least one test
- [ ] README covers install (all channels), quickstart, full flag reference, architecture
      choice explanation, extras explanation
- [ ] Shell completions generate valid, loadable scripts for bash/zsh/fish
- [ ] No panics reachable from any documented user input (fuzz the flag parser lightly if
      time allows)

---

## 10. Naming note

`springup` is a placeholder name used throughout this spec for consistency — rename freely
(check crates.io / npm-adjacent naming collisions before final choice; something in the
"spring-*" or "*-init" space that isn't already a popular crates.io package is worth a quick
search before committing).
