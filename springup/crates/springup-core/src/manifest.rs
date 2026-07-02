//! The `springup.toml` project manifest.
//!
//! Written to the root of every generated project. This is the `package.json` equivalent — it is
//! what makes a future `springup add <module>` possible without re-deriving context.
//!
//! For v1 the file is inert metadata (the tool doesn't read it back beyond a future `add`
//! command), but it must be written correctly and round-trip cleanly through TOML.

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::plan::{ArchitectureKind, Language, Packaging, ProjectPlan};

/// The current manifest schema version.
pub const MANIFEST_SCHEMA_VERSION: &str = "0.1.0";

/// The on-disk manifest structure. Mirrors the example in the spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    /// The `[project]` table.
    pub project: ProjectSection,
    /// The `[architecture]` table.
    #[serde(default)]
    pub architecture: ArchitectureSection,
    /// The `[extras]` table.
    #[serde(default)]
    pub extras: ExtrasSection,
    /// The `[dependencies]` table.
    #[serde(default)]
    pub dependencies: DependenciesSection,
}

/// `[project]` section of the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSection {
    /// Version of `springup` that generated this project.
    pub springup_version: String,
    /// ISO-8601 timestamp at which the project was generated.
    pub generated_at: String,
    /// Maven group id.
    pub group_id: String,
    /// Maven artifact id.
    pub artifact_id: String,
    /// Project name.
    #[serde(default)]
    pub name: String,
    /// Project description.
    #[serde(default)]
    pub description: String,
    /// Base Java package.
    pub package_name: String,
    /// Spring Boot version.
    pub spring_boot_version: String,
    /// Build tool slug.
    pub build_tool: String,
    /// Language slug.
    pub language: String,
    /// Java version.
    pub java_version: String,
    /// Packaging slug.
    pub packaging: String,
}

/// `[architecture]` section.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchitectureSection {
    /// Architecture kind (`none`, `layered`, `hexagonal`).
    #[serde(default)]
    pub kind: String,
}

/// `[extras]` section.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtrasSection {
    /// Enabled extra slugs.
    #[serde(default)]
    pub enabled: Vec<String>,
}

/// `[dependencies]` section.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependenciesSection {
    /// Initializr dependency ids.
    #[serde(default)]
    pub initializr: Vec<String>,
}

impl ProjectManifest {
    /// Build a manifest from a fully-resolved [`ProjectPlan`].
    pub fn from_plan(plan: &ProjectPlan) -> Self {
        let kind = match plan.architecture {
            None => "none".to_string(),
            Some(ArchitectureKind::Layered) => "layered".to_string(),
            Some(ArchitectureKind::Hexagonal) => "hexagonal".to_string(),
        };
        Self {
            project: ProjectSection {
                springup_version: env!("CARGO_PKG_VERSION").to_string(),
                generated_at: Utc::now().to_rfc3339(),
                group_id: plan.group_id.clone(),
                artifact_id: plan.artifact_id.clone(),
                name: plan.name.clone(),
                description: plan.description.clone(),
                package_name: plan.package_name.clone(),
                spring_boot_version: plan.spring_boot_version.clone(),
                build_tool: plan.build_tool.slug().to_string(),
                language: match plan.language {
                    Language::Java => "java",
                    Language::Kotlin => "kotlin",
                }
                .to_string(),
                java_version: plan.java_version.clone(),
                packaging: match plan.packaging {
                    Packaging::Jar => "jar",
                    Packaging::War => "war",
                }
                .to_string(),
            },
            architecture: ArchitectureSection { kind },
            extras: ExtrasSection {
                enabled: plan.extras.iter().map(|f| f.slug().to_string()).collect(),
            },
            dependencies: DependenciesSection {
                initializr: plan.dependencies.clone(),
            },
        }
    }

    /// Serialize to a TOML string.
    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write the manifest to `<dir>/springup.toml`.
    pub fn write_to_dir(&self, dir: &Path) -> Result<()> {
        let path = dir.join("springup.toml");
        std::fs::write(path, self.to_toml()?).map_err(Into::into)
    }
}

/// Read a manifest from a TOML string (used in tests and the future `add` command).
pub fn parse_manifest(toml_str: &str) -> Result<ProjectManifest> {
    Ok(toml::from_str(toml_str)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BuildTool, ExtraFeature, ProjectPlan};
    use std::path::PathBuf;

    fn plan() -> ProjectPlan {
        ProjectPlan {
            group_id: "dev.raghavarora".into(),
            artifact_id: "my-service".into(),
            name: "my-service".into(),
            description: "A demo service".into(),
            package_name: "dev.raghavarora.myservice".into(),
            spring_boot_version: "3.5.0".into(),
            build_tool: BuildTool::Maven,
            language: Language::Java,
            java_version: "21".into(),
            packaging: Packaging::Jar,
            dependencies: vec!["web".into(), "data-jpa".into()],
            architecture: Some(ArchitectureKind::Layered),
            extras: vec![ExtraFeature::Dockerfile, ExtraFeature::GithubActionsCi],
            output_dir: PathBuf::from("./my-service"),
            git_init: true,
            initial_commit: false,
        }
    }

    #[test]
    fn manifest_round_trips() {
        let manifest = ProjectManifest::from_plan(&plan());
        let toml_str = manifest.to_toml().unwrap();
        let parsed = parse_manifest(&toml_str).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn manifest_has_correct_extras_and_deps() {
        let manifest = ProjectManifest::from_plan(&plan());
        assert_eq!(manifest.extras.enabled, vec!["docker", "ci"]);
        assert_eq!(manifest.dependencies.initializr, vec!["web", "data-jpa"]);
        assert_eq!(manifest.architecture.kind, "layered");
        assert_eq!(manifest.project.build_tool, "maven");
    }

    #[test]
    fn manifest_for_none_architecture() {
        let mut p = plan();
        p.architecture = None;
        let m = ProjectManifest::from_plan(&p);
        assert_eq!(m.architecture.kind, "none");
    }

    #[test]
    fn write_to_dir_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = ProjectManifest::from_plan(&plan());
        manifest.write_to_dir(tmp.path()).unwrap();
        let written = std::fs::read_to_string(tmp.path().join("springup.toml")).unwrap();
        assert!(written.contains("[project]"));
        assert!(written.contains("springup_version"));
    }
}
