//! `springup config get|set|list` integration tests.

use assert_cmd::Command;
use predicates::prelude::*;

mod common;

#[test]
fn config_get_unknown_key_exits_nonzero() {
    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.args(["config", "get", "bogus-key"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown config key"));
}

#[test]
fn config_set_and_get_round_trip() {
    // Use a custom HOME so we don't clobber the dev's real config.
    let tmp = tempfile::tempdir().unwrap();
    let home_env = if cfg!(windows) { "USERPROFILE" } else { "HOME" };

    let mut cmd_set = Command::cargo_bin("springup").unwrap();
    cmd_set
        .env(home_env, tmp.path())
        .args(["config", "set", "group-id", "dev.test.integration"])
        .assert()
        .success()
        .stdout(predicate::str::contains("group-id = dev.test.integration"));

    let mut cmd_get = Command::cargo_bin("springup").unwrap();
    cmd_get
        .env(home_env, tmp.path())
        .args(["config", "get", "group-id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dev.test.integration"));
}

#[test]
fn config_list_includes_known_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let home_env = if cfg!(windows) { "USERPROFILE" } else { "HOME" };

    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.env(home_env, tmp.path())
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("group-id"))
        .stdout(predicate::str::contains("java-version"))
        .stdout(predicate::str::contains("build-tool"))
        .stdout(predicate::str::contains("telemetry"));
}

#[test]
fn config_set_invalid_value_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let home_env = if cfg!(windows) { "USERPROFILE" } else { "HOME" };

    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.env(home_env, tmp.path())
        .args(["config", "set", "build-tool", "bazel"])
        .assert()
        .failure();
}
