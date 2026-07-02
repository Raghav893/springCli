//! End-to-end `springup new` integration tests against a mocked Initializr server.
//!
//! These tests NEVER hit the real network. We spin up a `wiremock` mock server that returns
//! canned metadata + a placeholder starter zip, point `springup` at it via
//! `--base-url <mock_url>`, and assert on the generated file tree.

mod common;

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{fake_starter_zip_bytes, FAKE_INITIALIZR_METADATA};

/// Spin up a mock Initializr that responds to `/` (metadata) and `/starter.zip` (project zip).
async fn spawn_mock_initializr() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(FAKE_INITIALIZR_METADATA)
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/starter.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_starter_zip_bytes()))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn new_yes_generates_project_files() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("my-service");

    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.args([
        "new",
        "my-service",
        "--yes",
        "--base-url",
        &server.uri(),
        "--group-id",
        "dev.test",
        "--artifact-id",
        "my-service",
        "--boot-version",
        "3.5.0",
        "--build-tool",
        "maven",
        "--language",
        "java",
        "--java-version",
        "21",
        "--packaging",
        "jar",
        "--deps",
        "web,data-jpa,postgresql",
        "--architecture",
        "layered",
        "--extras",
        "docker,ci,config-profiles,readme",
        "--output",
        out.to_str().unwrap(),
        "--no-git",
    ])
    .assert()
    .success();

    let _ = cmd; // silence unused mut warning

    // Base project from Initializr (pom.xml came from the fake zip).
    assert!(
        out.join("pom.xml").exists(),
        "pom.xml should be extracted from starter zip"
    );

    // Custom layer: layered architecture.
    assert!(out
        .join("src/main/java/dev/test/my/service/exception/GlobalExceptionHandler.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/dto/ApiResponse.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/controller/HealthController.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/entity/SampleEntity.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/repository/SampleRepository.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/service/SampleService.java")
        .exists());
    assert!(out
        .join("src/main/java/dev/test/my/service/controller/SampleController.java")
        .exists());

    // Extras.
    assert!(out.join("Dockerfile").exists());
    assert!(out.join(".dockerignore").exists());
    assert!(out.join(".github/workflows/ci.yml").exists());
    assert!(out.join("src/main/resources/application-dev.yml").exists());
    assert!(out.join("src/main/resources/application-prod.yml").exists());
    assert!(out.join("README.md").exists());

    // Manifest.
    assert!(out.join("springup.toml").exists());
    let manifest = std::fs::read_to_string(out.join("springup.toml")).unwrap();
    assert!(manifest.contains("[project]"));
    assert!(manifest.contains("group_id = \"dev.test\""));
    assert!(manifest.contains("artifact_id = \"my-service\""));
    assert!(manifest.contains("build_tool = \"maven\""));
    assert!(manifest.contains("[architecture]"));
    assert!(manifest.contains("kind = \"layered\""));
    assert!(manifest.contains("[dependencies]"));
    // TOML may serialize arrays single-line or multi-line; just check each dep is present.
    assert!(manifest.contains("\"web\""));
    assert!(manifest.contains("\"data-jpa\""));
    assert!(manifest.contains("\"postgresql\""));
}

#[tokio::test]
async fn new_dockerfile_contains_correct_java_version() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("svc");

    Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "svc",
            "--yes",
            "--base-url",
            &server.uri(),
            "--java-version",
            "21",
            "--build-tool",
            "maven",
            "--extras",
            "docker",
            "--output",
            out.to_str().unwrap(),
            "--no-git",
        ])
        .assert()
        .success();

    let dockerfile = std::fs::read_to_string(out.join("Dockerfile")).unwrap();
    assert!(
        dockerfile.contains("maven:3.9-eclipse-temurin-21"),
        "Dockerfile should reference Java 21 maven image; got:\n{dockerfile}"
    );
    assert!(
        dockerfile.contains("eclipse-temurin:21-jre"),
        "Dockerfile should reference Java 21 JRE runtime image; got:\n{dockerfile}"
    );
}

#[tokio::test]
async fn new_unknown_dependency_fails_with_suggestion() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();

    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.args([
        "new",
        "svc",
        "--yes",
        "--base-url",
        &server.uri(),
        "--deps",
        "wbe", // typo for "web" — should not be suggested (jaro_winkler < 0.7)
        "--output",
        tmp.path().join("svc").to_str().unwrap(),
        "--no-git",
    ])
    .assert()
    .failure();
    let _ = cmd;
}

#[tokio::test]
async fn new_typo_dependency_suggests_correction() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();

    let output = Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "svc",
            "--yes",
            "--base-url",
            &server.uri(),
            "--deps",
            "wev", // typo for "web" — should be suggested
            "--output",
            tmp.path().join("svc").to_str().unwrap(),
            "--no-git",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("wev") && stderr.contains("web"),
        "stderr should mention both the unknown id and the suggestion; got:\n{stderr}"
    );
}

#[tokio::test]
async fn new_output_dir_not_empty_refuses() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("existing");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("blocking-file.txt"), b"hello").unwrap();

    Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "svc",
            "--yes",
            "--base-url",
            &server.uri(),
            "--output",
            out.to_str().unwrap(),
            "--no-git",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not empty"));
}

#[tokio::test]
async fn new_gradle_build_tool_uses_gradle_templates() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("grad");

    Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "grad",
            "--yes",
            "--base-url",
            &server.uri(),
            "--build-tool",
            "gradle",
            "--extras",
            "ci",
            "--output",
            out.to_str().unwrap(),
            "--no-git",
        ])
        .assert()
        .success();

    let ci_yml = std::fs::read_to_string(out.join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci_yml.contains("./gradlew build"),
        "Gradle CI should use ./gradlew build; got:\n{ci_yml}"
    );
}

#[tokio::test]
async fn new_hexagonal_architecture_writes_ports_and_adapters() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("hex");

    Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "hex",
            "--yes",
            "--base-url",
            &server.uri(),
            "--group-id",
            "com.example",
            "--artifact-id",
            "hex",
            "--deps",
            "web,data-jpa",
            "--architecture",
            "hexagonal",
            "--output",
            out.to_str().unwrap(),
            "--no-git",
        ])
        .assert()
        .success();

    let pkg_base = "src/main/java/com/example/hex";
    assert!(out
        .join(format!("{pkg_base}/domain/model/Sample.java"))
        .exists());
    assert!(out
        .join(format!("{pkg_base}/domain/port/in/GetSampleUseCase.java"))
        .exists());
    assert!(out
        .join(format!("{pkg_base}/domain/port/out/SampleRepository.java"))
        .exists());
    assert!(out
        .join(format!("{pkg_base}/application/GetSampleService.java"))
        .exists());
    assert!(out
        .join(format!("{pkg_base}/adapter/in/web/SampleController.java"))
        .exists());
    assert!(out
        .join(format!(
            "{pkg_base}/adapter/out/persistence/SampleEntity.java"
        ))
        .exists());
}

#[tokio::test]
async fn new_docker_compose_detects_postgres() {
    let server = spawn_mock_initializr().await;
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("pg");

    Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "pg",
            "--yes",
            "--base-url",
            &server.uri(),
            "--deps",
            "data-jpa,postgresql",
            "--extras",
            "docker-compose",
            "--output",
            out.to_str().unwrap(),
            "--no-git",
        ])
        .assert()
        .success();

    let compose = std::fs::read_to_string(out.join("docker-compose.yml")).unwrap();
    assert!(
        compose.contains("postgres:"),
        "expected postgres service; got:\n{compose}"
    );
    assert!(compose.contains("5432:5432"));
}

#[test]
fn add_stub_returns_not_implemented() {
    let mut cmd = Command::cargo_bin("springup").unwrap();
    cmd.args(["add", "flyway"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not yet implemented"));
}

/// Sanity: the test harness itself can locate the binary.
#[test]
fn binary_path_exists() {
    let p = common::springup_binary();
    assert!(
        Path::new(&p).exists(),
        "binary should exist at {}",
        p.display()
    );
}
