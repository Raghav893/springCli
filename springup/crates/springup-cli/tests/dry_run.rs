//! `springup new --dry-run` integration tests.

mod common;

use assert_cmd::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::FAKE_INITIALIZR_METADATA;

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
    server
}

#[tokio::test]
async fn dry_run_prints_resolved_plan_as_json() {
    let server = spawn_mock_initializr().await;
    let output = Command::cargo_bin("springup")
        .unwrap()
        .args([
            "new",
            "demo",
            "--yes",
            "--dry-run",
            "--base-url",
            &server.uri(),
            "--group-id",
            "com.example",
            "--artifact-id",
            "demo",
            "--deps",
            "web",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"group_id\": \"com.example\""));
    assert!(stdout.contains("\"artifact_id\": \"demo\""));
    assert!(stdout.contains("\"dependencies\""));
    assert!(stdout.contains("\"spring_boot_version\": \"3.5.0\""));
}
