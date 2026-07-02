# springup

> Scaffold a production-ready Spring Boot backend in seconds. `springup` is what `npm create`,
> `bun create`, and `cargo generate` are to their ecosystems, but for Spring Boot.

`springup` is a fast, single static binary that interactively (or non-interactively)
scaffolds a production-ready Spring Boot backend by combining the official Spring Initializr
API with an opinionated custom template layer (architecture skeletons, Docker, CI, config
profiles) — written in Rust, distributed as a real CLI tool.

---

## Highlights

- **Sub-second startup**, smooth interactive prompts, zero Java/Maven/Gradle required to
  *generate* a project.
- **Two first-class modes**: an interactive TUI wizard (default) and fully flag-driven
  non-interactive mode (for CI, scripting, muscle-memory power users).
- **Real, buildable projects**: delegates dependency/version resolution to the official
  Spring Initializr API, then layers custom extras on top.
- **Production-grade code**: typed errors, structured logging, retry-with-backoff, zip-slip
  protection, fuzz-matched dependency suggestions, snapshot tests, no telemetry.
- **Self-contained binary**: all custom templates are embedded via `rust-embed` — only the
  Initializr base project requires network.
- **Future-proof**: a `springup.toml` manifest is written into every project so a future
  `springup add <module>` command works without a rewrite.

---

## Install

### `cargo install` (Rust users)

```bash
cargo install springup --locked
```

### `curl | sh` (Linux / macOS)

```bash
curl -fsSL https://github.com/springup-cli/springup/raw/main/dist/install.sh | sh
```

### Homebrew (macOS)

```bash
brew install springup-cli/tap/springup
```

### Scoop (Windows)

```powershell
scoop bucket add springup-cli https://github.com/springup-cli/scoop-bucket.git
scoop install springup
```

### Shell completions

```bash
springup completions bash    > /etc/bash_completion.d/springup
springup completions zsh     > "${fpath[1]}/_springup"
springup completions fish    > ~/.config/fish/completions/springup.fish
springup completions powershell | Out-File springup.ps1
```

Verify:

```bash
springup --version
```

---

## Quickstart

### Interactive (default)

```bash
springup new
```

`springup` will walk you through project metadata, Spring Boot version, build tool, language,
Java version, packaging, dependencies (searchable, grouped by category), an optional
architecture skeleton, and extras. Then it fetches the base project from
`https://start.spring.io`, layers your choices on top, writes a `springup.toml` manifest,
and prints next steps.

### Non-interactive (CI / scripting)

```bash
springup new my-service \
  --group-id dev.raghavarora \
  --artifact-id my-service \
  --boot-version 3.5.0 \
  --build-tool maven \
  --language java \
  --java-version 21 \
  --packaging jar \
  --deps web,data-jpa,postgresql,validation,security \
  --architecture layered \
  --extras docker,docker-compose,ci,config-profiles,readme \
  --no-git \
  --yes
```

Or, accept every default:

```bash
springup new my-service -y
```

### Dry-run (peek at the resolved plan)

```bash
springup new my-service --dry-run --yes
```

Prints the fully-resolved `ProjectPlan` as JSON and exits without writing anything.

---

## Command reference

```
springup new [NAME]                 Scaffold a new project (interactive if flags omitted)
springup add <MODULE>                [stub — designed for v2]
springup config get|set|list         Manage global user config
springup completions <SHELL>         Generate shell completion script (bash/zsh/fish/powershell)
springup update-metadata             Refresh the cached Initializr metadata
springup update                      Self-update (stub in v1 — reinstall via your package manager)
springup --version / -V
springup --help / -h
```

### `springup new` flags

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
    --extras <EXTRA,EXTRA,...>         docker,docker-compose,ci,config-profiles,editorconfig,readme
-o, --output <DIR>                     default: ./<artifact-id>
    --git / --no-git                   default: --git
    --commit / --no-commit             default: --no-commit
-y, --yes                              accept all defaults, skip wizard entirely
    --dry-run                          print the resolved ProjectPlan as JSON, generate nothing
    --offline                          use cached metadata only; never hit the network
    --refresh                          force-refresh the cached Initializr metadata
    --base-url <URL>                   Initializr base URL override (env: SPRINGUP_INITIALIZR_BASE_URL)
```

**Mode resolution rule**: if stdin is not a TTY, OR `--yes` is passed, OR enough flags are
present to fully resolve a `ProjectPlan`, skip the wizard. If flags are partially given but
insufficient, the wizard runs but pre-fills / skips the steps already answered by flags —
the best of both worlds.

---

## Architecture skeletons

Choose `none`, `layered`, or `hexagonal`:

### `layered`

```
src/main/java/<package>/
├── controller/    # REST controllers
├── service/       # Business logic
├── repository/    # Spring Data JPA repositories
├── dto/           # Request / response DTOs (incl. ApiResponse<T>)
├── entity/        # JPA entities
└── exception/     # GlobalExceptionHandler + custom exceptions
```

Comes with a working CRUD vertical slice (`SampleEntity` → `SampleRepository` →
`SampleService` → `SampleController`) when `data-jpa` is selected.

### `hexagonal` (ports-and-adapters)

```
src/main/java/<package>/
├── domain/                  # Pure domain: model + ports
│   ├── model/               # Domain entities (no framework annotations)
│   └── port/
│       ├── in/              # Inbound ports (use cases)
│       └── out/             # Outbound ports (driven by adapters)
├── application/             # Use-case implementations
└── adapter/
    ├── in/web/              # Driving adapter: REST controllers
    └── out/persistence/     # Driven adapter: JPA persistence
```

The domain layer has zero framework dependencies. Adapters wire it to Spring.

---

## Extras

| Extra | What it adds |
| --- | --- |
| `docker` | Multi-stage `Dockerfile` matching your Java version + build tool, plus `.dockerignore`. |
| `docker-compose` | `docker-compose.yml` with the app + auto-detected backing services (postgres / mysql / redis / mongo) based on selected dependencies. |
| `ci` | `.github/workflows/ci.yml` — build + test workflow matching the chosen build tool, with dependency caching. |
| `config-profiles` | `application-dev.yml` + `application-prod.yml` split out from the default properties, with sensible dev-vs-prod defaults (H2/local DB vs. env-var-driven). |
| `editorconfig` | `.editorconfig` for consistent editor formatting. |
| `readme` | `README.md` with the project name, run instructions matching the build tool, Docker instructions, and a tuned `.gitignore`. |

---

## Configuration

Global user config lives at `~/.config/springup/config.toml` (Linux/macOS) or
`%APPDATA%\springup\config.toml` (Windows).

```toml
group_id = "com.example"
author = "Your Name"
java_version = "21"
build_tool = "maven"
spring_boot_version = ""   # empty = always use Initializr's latest stable
initializr_base_url = ""    # empty = use https://start.spring.io
color = "auto"              # auto | always | never
telemetry = false           # always off by default — see Privacy below
```

Manage via subcommand:

```bash
springup config set group-id dev.raghavarora
springup config get java-version
springup config list
```

### Precedence (most specific wins)

1. CLI flags
2. Project-local `.springuprc.toml` (if present in cwd)
3. Global user config (`~/.config/springup/config.toml`)
4. Hardcoded defaults

---

## Privacy

`springup` collects **no telemetry**. There is no analytics SDK, no phone-home beacon, no
crash reporter. The only network calls are to `https://start.spring.io` (or your configured
mirror) and only when you run `springup new` / `springup update-metadata`. Metadata is cached
locally at `~/.cache/springup/initializr-metadata.json` with a 24h TTL.

This is a stated principle of the project, not an oversight — see `CONTRIBUTING.md` if you
ever want to add opt-in telemetry.

---

## How it works

1. **Mode detection**: flags + TTY detection decide wizard vs. non-interactive.
2. **Metadata fetch**: `GET https://start.spring.io` with `Accept: application/json` returns
   the full metadata catalogue. Cached for 24h. Fetched concurrently with the first wizard
   prompts so perceived latency is hidden.
3. **Plan resolution**: a `ProjectPlan` struct is built (by either the wizard or flag
   parsing) and validated.
4. **Base project download**: `GET https://start.spring.io/starter.zip?type=...&...` returns
   a real, buildable Spring Boot project as a zip. Streamed to disk and extracted with
   zip-slip protection.
5. **Custom template layer**: embedded `minijinja` templates (architecture skeletons, Docker,
   CI, profiles, README) are rendered against the `ProjectPlan` and written into the project.
6. **Manifest**: `springup.toml` is written so a future `springup add` knows the project's
   shape.
7. **Optional `git init`** + initial commit.
8. **Summary**: colored output showing what was generated, file tree, and next-step commands.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the module layout and how to add a new template.

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). PRs welcome — `cargo fmt --check`,
`cargo clippy -- -D warnings`, and `cargo test --all-features` must pass.

---

## License

Dual-licensed under MIT OR Apache-2.0, at your option.
