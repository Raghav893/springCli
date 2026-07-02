//! Shared helpers for integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

/// Locate the `springup` binary built by `cargo`. Used by `assert_cmd::Command` indirectly via
/// `cargo_bin`, but we expose a helper for cases that need the path explicitly.
pub fn springup_binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("springup")
}

/// A minimal but realistic Initializr metadata document for tests.
///
/// Mirrors the shape of the real `GET https://start.spring.io` response (just trimmed to the
/// fields springup actually reads). Keep this in sync with the
/// `springup_core::initializr::InitializrMetadata` schema.
pub const FAKE_INITIALIZR_METADATA: &str = r#"{
    "bootVersion": {
        "default": "3.5.0",
        "values": [
            {"id": "3.5.0", "name": "3.5.0"},
            {"id": "3.4.0", "name": "3.4.0"},
            {"id": "3.5.0-SNAPSHOT", "name": "3.5.0 (SNAPSHOT)"}
        ]
    },
    "javaVersion": {
        "default": "21",
        "values": [
            {"id": "17", "name": "17"},
            {"id": "21", "name": "21"}
        ]
    },
    "language": {
        "default": "java",
        "values": [
            {"id": "java", "name": "Java"},
            {"id": "kotlin", "name": "Kotlin"}
        ]
    },
    "packaging": {
        "default": "jar",
        "values": [
            {"id": "jar", "name": "Jar"},
            {"id": "war", "name": "War"}
        ]
    },
    "type": {
        "default": "maven-build",
        "values": [
            {"id": "maven-build", "name": "Maven", "action": "/starter.zip", "tags": {"build": "maven"}},
            {"id": "gradle-build", "name": "Gradle (Groovy)", "action": "/starter.zip", "tags": {"build": "gradle"}},
            {"id": "gradle-build-kotlin", "name": "Gradle (Kotlin)", "action": "/starter.zip", "tags": {"build": "gradle-kotlin"}}
        ]
    },
    "groupId": {"default": "com.example"},
    "artifactId": {"default": "demo"},
    "name": {"default": "demo"},
    "packageName": {"default": "com.example.demo"},
    "description": {"default": ""},
    "dependencies": {
        "values": [
            {
                "name": "Web",
                "values": [
                    {"id": "web", "name": "Spring Web", "description": "Build RESTful web apps"},
                    {"id": "actuator", "name": "Spring Boot Actuator", "description": "Production-ready features"}
                ]
            },
            {
                "name": "SQL",
                "values": [
                    {"id": "data-jpa", "name": "Spring Data JPA", "description": "JPA persistence"},
                    {"id": "postgresql", "name": "PostgreSQL Driver", "description": "PostgreSQL JDBC driver"},
                    {"id": "mysql", "name": "MySQL Driver", "description": "MySQL JDBC driver"}
                ]
            },
            {
                "name": "NoSQL",
                "values": [
                    {"id": "data-redis", "name": "Spring Data Redis", "description": "Redis caching"},
                    {"id": "data-mongodb", "name": "Spring Data MongoDB", "description": "MongoDB persistence"}
                ]
            },
            {
                "name": "Migration",
                "values": [
                    {"id": "flyway", "name": "Flyway Migration", "description": "Versioned DB migrations"}
                ]
            },
            {
                "name": "Validation",
                "values": [
                    {"id": "validation", "name": "Validation", "description": "Bean validation"}
                ]
            },
            {
                "name": "Security",
                "values": [
                    {"id": "security", "name": "Spring Security", "description": "Auth & authz"}
                ]
            }
        ]
    }
}"#;

/// A tiny placeholder "starter zip" — just enough bytes for `extract_zip` to handle. We use a
/// real (single-file) zip so the extractor doesn't reject it.
pub fn fake_starter_zip_bytes() -> Vec<u8> {
    use std::io::Write;
    // Build an in-memory zip containing a single file: `pom.xml`.
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("pom.xml", options).unwrap();
    zip.write_all(b"<project><!-- placeholder --></project>")
        .unwrap();
    zip.finish().unwrap().into_inner()
}
