//! CLI smoke tests: --help, --version, no-args behavior.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_prints_pkg_version() {
    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("springup "))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_lists_subcommands() {
    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("completions"))
        .stdout(predicate::str::contains("update-metadata"));
}

#[test]
fn new_help_lists_flags() {
    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.args(["new", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--group-id"))
        .stdout(predicate::str::contains("--artifact-id"))
        .stdout(predicate::str::contains("--boot-version"))
        .stdout(predicate::str::contains("--build-tool"))
        .stdout(predicate::str::contains("--deps"))
        .stdout(predicate::str::contains("--extras"))
        .stdout(predicate::str::contains("--architecture"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--yes"));
}
