# Changelog

All notable changes to `springup` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial public release candidate.

## [0.1.0] — YYYY-MM-DD

### Added
- `springup new` command with interactive wizard (`dialoguer`) and non-interactive flag-driven mode.
- Spring Initializr REST client with metadata fetch, 24h disk cache, and bounded retry-with-backoff.
- Custom template layer with embedded `minijinja` templates:
  - `layered` architecture skeleton (controller / service / repository / dto / entity / exception)
  - `hexagonal` ports-and-adapters skeleton (domain / application / adapter/{in,out})
  - `docker` extra: multi-stage Dockerfile + `.dockerignore`, Java-version-aware
  - `docker-compose` extra: app + auto-detected postgres / mysql / redis / mongo services
  - `ci` extra: GitHub Actions workflow matching the chosen build tool (Maven or Gradle)
  - `config-profiles` extra: `application-dev.yml` + `application-prod.yml` with env-var-driven prod config
  - `editorconfig` extra: `.editorconfig` for consistent editor formatting
  - `readme` extra: `README.md` with run instructions + tuned `.gitignore`
- `springup.toml` project manifest written to every generated project (extension point for v2 `add`).
- `springup config get|set|list` for global user config at `~/.config/springup/config.toml`.
- `springup completions <shell>` for bash, zsh, fish, PowerShell, elvish.
- `springup update-metadata` to refresh the cached Initializr metadata.
- `springup add <module>` and `springup update` stubs (designed in v1, implemented in v2).
- Fuzzy-match dependency suggestions ("unknown dependency 'wev', did you mean 'web'?").
- Zip-slip protection during Initializr zip extraction.
- `--dry-run` mode that prints the resolved `ProjectPlan` as JSON.
- `--offline` mode that prefers stale cached metadata over the network.
- `--refresh` flag to force-refresh the metadata cache.
- `--verbose` / `-v` (repeatable) and `--quiet` for log level control.
- `--color auto|always|never` plus `NO_COLOR` env var support.
- 39 unit tests, 18 integration tests (mocked Initializr via `wiremock`), 13 snapshot tests
  (`insta`). All passing.
- `cargo clippy -- -D warnings` and `cargo fmt --check` clean.
- CI workflows: `ci.yml` (fmt / clippy / test / xtask / doc on ubuntu + macOS + Windows),
  `release.yml` (`cargo-dist` build for 6 targets + crates.io publish), `audit.yml`
  (nightly `cargo audit` + live Initializr smoke test).
- `README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`.

### Privacy
- Zero telemetry. The only network calls are to `https://start.spring.io` (or your
  configured mirror) when running `springup new` or `springup update-metadata`. No
  analytics, no crash reporting, no phone-home beacons.

[Unreleased]: https://github.com/springup-cli/springup/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/springup-cli/springup/releases/tag/v0.1.0
