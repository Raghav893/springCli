# Contributing to springup

Thanks for your interest in improving `springup`! This document covers everything you need to
get started.

## Project status

`springup` is pre-1.0. The CLI surface, manifest format, and template output may change
between minor versions. Once we hit 1.0, the manifest format and the `springup new` flag
set will be considered stable.

## Development prerequisites

- Rust 1.75+ (run `rustup update stable`)
- `cargo install cargo-insta` (for snapshot test review)
- `cargo install cargo-deny` (optional, for license audits)
- No Java/Maven/Gradle required — tests use mocked Initializr

## Getting started

```bash
git clone https://github.com/springup-cli/springup
cd springup
cargo test --workspace
cargo run -- new my-test-project --yes --no-git --output /tmp/springup-test
```

## Development workflow

### Run the CLI from source

```bash
cargo run -- new my-service --yes
cargo run -- --help
cargo run -- config list
```

### Run all tests

```bash
cargo test --workspace
```

This runs:
- Unit tests in `springup-core` (plan validation, fuzzy match, manifest round-trip, config
  merge, cache TTL, metadata parsing)
- Integration tests in `springup-cli/tests/` (mocked Initializr via `wiremock`)
- Snapshot tests in `springup-core/tests/snapshot_tests.rs`

### Snapshot tests

Snapshots live in `crates/springup-core/tests/snapshots/`. When you intentionally change a
template:

1. Run `cargo insta test` to see the diff.
2. Run `cargo insta accept` to update the snapshots.
3. Commit the updated `.snap` files alongside your template change.

If you see a snapshot diff you didn't expect, that's a regression — investigate before
accepting.

### Lint checks (must pass before merging)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### Template asset checks

```bash
cargo run -p xtask -- check-templates
cargo run -p xtask -- check-assets
```

`check-templates` verifies every `.j2` file parses as valid `minijinja` source.
`check-assets` verifies every asset is non-empty and (where applicable) valid UTF-8.

## How to add things

### Add a new Initializr dependency mapping

No code change needed — `springup` accepts any Initializr dependency id. The wizard
discovers them dynamically from Initializr's metadata. If you want a new "convenience
boolean" on `TemplateContext` (e.g. `has_kafka`), add it to `TemplateContext::from_plan`
in `template.rs` and use it in your template.

### Add a new extra

1. Add a variant to `ExtraFeature` in `crates/springup-core/src/plan.rs`. Update `ALL`,
   `slug()`, `from_slug()`.
2. Add the template file under `crates/springup-templates/assets/<category>/`.
3. Add an `apply_<extra>` method to `TemplateRenderer` in
   `crates/springup-core/src/template.rs` and a case in `apply_extras`.
4. Add a snapshot test in `crates/springup-core/tests/snapshot_tests.rs`.
5. Add an integration test in `crates/springup-cli/tests/new_command.rs` that exercises
   the new extra end-to-end.
6. Update `README.md`'s extras table.

### Add a new architecture skeleton

1. Add a variant to `ArchitectureKind` in `plan.rs` (update `from_slug`, `ALL`).
2. Create the template files under
   `crates/springup-templates/assets/architectures/<name>/`.
3. Add an `apply_<arch>` method to `TemplateRenderer` and a case in `apply_extras`.
4. Snapshot test + integration test.

### Add a new CLI flag

1. Add the field to `NewArgs` in `crates/springup-cli/src/cli.rs`.
2. Wire it into `plan_from_flags` in `commands/new.rs`.
3. If it changes `ProjectPlan`, update `ProjectPlan::validate` and add a unit test.
4. Add an integration test that exercises the flag.

## PR checklist

Before opening a PR:

- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo test --workspace` passes
- [ ] New tests added for any new logic
- [ ] Snapshots updated via `cargo insta accept` (and the diff was intentional)
- [ ] `README.md` / `ARCHITECTURE.md` updated if behavior changed
- [ ] No new `unwrap()` / `panic!()` on paths reachable from user input or network I/O
- [ ] No telemetry added (see Privacy section of README)

## Commit message conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add Kafka Streams starter template
fix: respect NO_COLOR env var in summary output
docs: clarify flag precedence in README
test: add snapshot for gradle-kotlin Dockerfile
chore: bump minijinja to 2.5
```

## Release process

Releases are automated via `cargo-dist`:

1. Bump version in `Cargo.toml` (workspace).
2. Update `CHANGELOG.md`.
3. Open a PR titled `release: vX.Y.Z`.
4. After merge, tag `vX.Y.Z` and push: `git tag vX.Y.Z && git push --tags`.
5. CI builds binaries for all 6 targets, publishes to crates.io, and creates a GitHub
   Release with the install script.

## Code of conduct

Be kind. Disagreements are fine; personal attacks are not. We follow the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## License

By contributing, you agree that your contributions will be dual-licensed under MIT OR
Apache-2.0, at the option of the user.
