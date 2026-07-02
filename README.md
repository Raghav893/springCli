<p align="center">
  <strong>springup</strong><br>
  <em>Scaffold a production-ready Spring Boot backend in seconds.</em>
</p>

<p align="center">
  <a href="https://github.com/Raghav893/springCli/releases"><img src="https://img.shields.io/github/v/release/Raghav893/springCli?style=flat-square&color=blue" alt="Release"></a>
  <a href="https://crates.io/crates/springup-cli"><img src="https://img.shields.io/crates/v/springup-cli?style=flat-square&color=orange" alt="crates.io"></a>
  <a href="https://github.com/Raghav893/springCli/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Raghav893/springCli/ci.yml?style=flat-square&label=CI" alt="CI"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat-square" alt="License"></a>
</p>

---

`springup` is what `npm create`, `bun create`, and `cargo generate` are to their ecosystems —
but for **Spring Boot**. A fast, single static binary that interactively (or non-interactively)
scaffolds a production-ready Spring Boot backend by combining the official Spring Initializr
API with an opinionated custom template layer.

**Written in Rust. Zero Java required to generate projects. Sub-second startup.**

---

## ⚡ Quick Install

### Linux / macOS (one command)

```bash
curl -fsSL https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.ps1 | iex
```

### Then just run:

```bash
springup new
```

That's it. You'll get an interactive wizard that walks you through creating a fully configured
Spring Boot project with architecture, Docker, CI, and more.

---

## 📦 All Install Methods

| Method | Command |
| --- | --- |
| **curl (Linux/macOS)** | `curl -fsSL https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.sh \| sh` |
| **PowerShell (Windows)** | `irm https://raw.githubusercontent.com/Raghav893/springCli/main/springup/dist/install.ps1 \| iex` |
| **Cargo (Rust users)** | `cargo install springup --locked` |
| **Homebrew (macOS)** | `brew install springup-cli/tap/springup` |
| **Scoop (Windows)** | `scoop bucket add springup-cli https://github.com/springup-cli/scoop-bucket.git && scoop install springup` |

After installing, verify it works:

```bash
springup --version
```

### Shell Completions

```bash
# Bash
springup completions bash > /etc/bash_completion.d/springup

# Zsh
springup completions zsh > "${fpath[1]}/_springup"

# Fish
springup completions fish > ~/.config/fish/completions/springup.fish

# PowerShell
springup completions powershell | Out-File springup.ps1
```

---

## 🚀 Usage

### Interactive Mode (recommended for first-time users)

```bash
springup new
```

The interactive wizard will guide you through:

1. **Project name & metadata** — group ID, artifact ID, description
2. **Spring Boot version** — fetched live from Spring Initializr
3. **Build tool** — Maven, Gradle (Groovy), or Gradle (Kotlin)
4. **Language** — Java or Kotlin
5. **Java version** — 17, 21, etc.
6. **Packaging** — JAR or WAR
7. **Dependencies** — searchable, grouped by category (Web, SQL, Security, etc.)
8. **Architecture skeleton** — none, layered, or hexagonal
9. **Extras** — Dockerfile, docker-compose, GitHub Actions CI, config profiles, README

Then it generates a **real, buildable** Spring Boot project and prints next steps.

### Non-Interactive Mode (CI / scripting / power users)

```bash
springup new my-service \
  --group-id com.example \
  --artifact-id my-service \
  --boot-version 3.5.0 \
  --build-tool maven \
  --language java \
  --java-version 21 \
  --packaging jar \
  --deps web,data-jpa,postgresql,validation,security \
  --architecture layered \
  --extras docker,docker-compose,ci,config-profiles,readme \
  --yes
```

### Quick Scaffold with Defaults

```bash
springup new my-service -y
```

Creates a project with sensible defaults — latest Spring Boot, Maven, Java 21, JAR packaging.

### Dry Run (preview without generating)

```bash
springup new my-service --dry-run --yes
```

Prints the fully-resolved project plan as JSON without writing any files.

---

## 📋 Command Reference

```
USAGE:
    springup <COMMAND>

COMMANDS:
    new [NAME]                  Scaffold a new Spring Boot project
    add <MODULE>                Add a module to existing project [coming in v2]
    config get|set|list         Manage global user configuration
    completions <SHELL>         Generate shell completion script
    update-metadata             Refresh cached Spring Initializr metadata
    update                      Self-update springup binary
    --version, -V               Print version
    --help, -h                  Print help
```

### `springup new` — Full Flag Reference

```
USAGE:
    springup new [NAME] [OPTIONS]

ARGUMENTS:
    [NAME]    Project directory name (defaults to artifact-id)

OPTIONS:
    -n, --name <NAME>                  Project name
        --group-id <GROUP_ID>          Maven group ID [default: com.example]
        --artifact-id <ARTIFACT_ID>    Maven artifact ID
        --description <DESC>           Project description
        --boot-version <VERSION>       Spring Boot version [default: latest stable]
        --build-tool <TOOL>            maven | gradle | gradle-kotlin
        --language <LANG>              java | kotlin
        --java-version <VERSION>       Java version (17, 21, etc.)
        --packaging <TYPE>             jar | war
    -d, --deps <DEP,DEP,...>           Comma-separated dependency IDs
        --architecture <ARCH>          none | layered | hexagonal
        --extras <EXTRA,EXTRA,...>     docker,docker-compose,ci,config-profiles,
                                       editorconfig,readme
    -o, --output <DIR>                 Output directory [default: ./<artifact-id>]
        --git / --no-git               Initialize git repo [default: --git]
        --commit / --no-commit         Create initial commit [default: --no-commit]
    -y, --yes                          Accept all defaults, skip wizard
        --dry-run                      Print resolved plan as JSON, generate nothing
        --offline                      Use cached metadata only, no network
        --refresh                      Force-refresh cached Initializr metadata
        --base-url <URL>               Override Initializr base URL
                                       [env: SPRINGUP_INITIALIZR_BASE_URL]
```

**Mode resolution**: If stdin is not a TTY, OR `--yes` is passed, OR enough flags fully
resolve a project plan → wizard is skipped. If flags are partially given, the wizard runs
but pre-fills the answered steps.

### `springup config` — Global Configuration

```bash
# Set your default group ID
springup config set group-id com.yourcompany

# View a config value
springup config get java-version

# List all config values
springup config list
```

Config file location:
- **Linux/macOS**: `~/.config/springup/config.toml`
- **Windows**: `%APPDATA%\springup\config.toml`

#### Config Options

```toml
group_id = "com.example"           # Default Maven group ID
author = "Your Name"               # Author for generated README
java_version = "21"                # Default Java version
build_tool = "maven"               # Default build tool
spring_boot_version = ""           # Empty = always use latest stable
initializr_base_url = ""           # Empty = https://start.spring.io
color = "auto"                     # auto | always | never
telemetry = false                  # Always off — no telemetry, ever
```

#### Config Precedence (most specific wins)

1. CLI flags
2. Project-local `.springuprc.toml` (if present in cwd)
3. Global user config (`~/.config/springup/config.toml`)
4. Hardcoded defaults

---

## 🏗 Architecture Skeletons

Choose your project structure during scaffolding:

### `layered` (traditional)

```
src/main/java/<package>/
├── controller/    # REST controllers
├── service/       # Business logic
├── repository/    # Spring Data JPA repositories
├── dto/           # Request/response DTOs (incl. ApiResponse<T>)
├── entity/        # JPA entities
└── exception/     # GlobalExceptionHandler + custom exceptions
```

Includes a working CRUD vertical slice (`SampleEntity` → `SampleRepository` →
`SampleService` → `SampleController`) when `data-jpa` is selected.

### `hexagonal` (ports and adapters)

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

The domain layer has **zero framework dependencies**. Adapters wire it to Spring.

### `none`

Leaves the default Spring Initializr single-package layout untouched.

---

## 🧩 Extras

| Extra | What It Adds |
| --- | --- |
| `docker` | Multi-stage `Dockerfile` matching your Java version + build tool, plus `.dockerignore` |
| `docker-compose` | `docker-compose.yml` with app + auto-detected backing services (Postgres/MySQL/Redis/Mongo) based on selected dependencies |
| `ci` | `.github/workflows/ci.yml` — build + test workflow matching your build tool with dependency caching |
| `config-profiles` | `application-dev.yml` + `application-prod.yml` with sensible dev-vs-prod defaults |
| `editorconfig` | `.editorconfig` for consistent formatting |
| `readme` | `README.md` with project name, run instructions, Docker instructions |

---

## 🔥 Examples

### Create a REST API with PostgreSQL

```bash
springup new user-service \
  --deps web,data-jpa,postgresql,validation \
  --architecture layered \
  --extras docker,docker-compose,ci \
  --yes
```

### Create a Kotlin microservice with Gradle

```bash
springup new order-service \
  --language kotlin \
  --build-tool gradle-kotlin \
  --deps web,data-jpa,postgresql,security,actuator \
  --architecture hexagonal \
  --extras docker,docker-compose,ci,config-profiles,readme \
  --yes
```

### Create a minimal service with all defaults

```bash
springup new my-api -y
```

### Preview what would be generated

```bash
springup new my-api --dry-run -y
```

### Use in a CI pipeline

```bash
# Non-interactive — no TTY needed
springup new payment-service \
  --group-id com.acme \
  --boot-version 3.5.0 \
  --deps web,data-jpa,postgresql \
  -y --no-git
```

---

## ✨ Highlights

- **Sub-second startup** — smooth interactive prompts, zero Java required to generate
- **Two modes** — interactive TUI wizard (default) + fully flag-driven for CI/scripting
- **Real, buildable projects** — delegates to the official Spring Initializr API, then layers custom extras on top
- **Production-grade code** — typed errors, structured logging, retry-with-backoff, zip-slip protection, fuzz-matched dependency suggestions, snapshot tests, no telemetry
- **Self-contained binary** — all custom templates embedded via `rust-embed`; only the Initializr base project requires network
- **Future-proof** — `springup.toml` manifest in every project enables future `springup add <module>` without a rewrite
- **Cross-platform** — pre-built binaries for Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows

---

## 🔒 Privacy

`springup` collects **no telemetry**. There is no analytics SDK, no phone-home beacon, no
crash reporter. The only network calls are to `https://start.spring.io` (or your configured
mirror) and only when you run `springup new` or `springup update-metadata`. Metadata is cached
locally with a 24-hour TTL.

This is a stated principle of the project, not an oversight.

---

## 🛠 How It Works

1. **Mode detection** — flags + TTY detection decide wizard vs. non-interactive
2. **Metadata fetch** — `GET https://start.spring.io` returns the full metadata catalogue (cached for 24h, fetched concurrently with the first wizard prompts)
3. **Plan resolution** — a `ProjectPlan` struct is built and validated
4. **Base project download** — `GET https://start.spring.io/starter.zip?...` returns a buildable project as a zip, streamed and extracted with zip-slip protection
5. **Custom template layer** — embedded `minijinja` templates are rendered and written into the project
6. **Manifest** — `springup.toml` is written for future `springup add` support
7. **Optional `git init`** + initial commit
8. **Summary** — colored output with file tree and next-step commands

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the full module layout.

---

## 🤝 Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). PRs welcome!

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all-features
```

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.
