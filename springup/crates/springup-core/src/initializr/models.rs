//! Typed models for the Spring Initializr metadata document.
//!
//! The Initializr root document is a JSON object with several top-level keys (`dependencies`,
//! `type`, `bootVersion`, `language`, `packaging`, `javaVersion`, `groupId`, `artifactId`, ...).
//! We model only what `springup` actually uses and let unknown fields pass through (`#[serde
//! (default)]` everywhere) so the client doesn't break when Initializr adds new keys.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The full Initializr metadata document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializrMetadata {
    /// Top-level bootVersion list — all Spring Boot versions Initializr knows about.
    #[serde(default)]
    pub boot_version: MetadataValues,
    /// Java version options.
    #[serde(default)]
    pub java_version: MetadataValues,
    /// Language options.
    #[serde(default, alias = "language")]
    pub languages: MetadataValues,
    /// Packaging options.
    #[serde(default)]
    pub packaging: MetadataValues,
    /// Project type options (Maven, Gradle Groovy, Gradle Kotlin).
    #[serde(default, alias = "type")]
    pub types: ProjectTypeValues,
    /// Dependency catalogue, grouped by category.
    #[serde(default)]
    pub dependencies: DependencyGroups,
    /// Default group id (e.g. `com.example`).
    #[serde(default)]
    pub group_id: MetadataDefault,
    /// Default artifact id (e.g. `demo`).
    #[serde(default)]
    pub artifact_id: MetadataDefault,
    /// Default project name.
    #[serde(default)]
    pub name: MetadataDefault,
    /// Default package name.
    #[serde(default)]
    pub package_name: MetadataDefault,
    /// Default description.
    #[serde(default)]
    pub description: MetadataDefault,
    /// Default packaging value.
    #[serde(default)]
    pub packaging_default: MetadataDefault,
}

impl InitializrMetadata {
    /// Convenience: returns the latest stable (non-snapshot, non-milestone) boot version id.
    ///
    /// Falls back to whatever is first in the list if no stable version is found.
    pub fn latest_stable_boot_version(&self) -> Option<&str> {
        self.boot_version
            .values
            .iter()
            .find(|v| v.stable())
            .or_else(|| self.boot_version.values.first())
            .map(|v| v.id.as_str())
    }

    /// Flatten every dependency id across all groups.
    pub fn all_dependency_ids(&self) -> Vec<String> {
        self.dependencies
            .values
            .iter()
            .flat_map(|g| g.values.iter().map(|d| d.id.clone()))
            .collect()
    }

    /// Look up a dependency by id.
    pub fn find_dependency(&self, id: &str) -> Option<&Dependency> {
        self.dependencies
            .values
            .iter()
            .flat_map(|g| g.values.iter())
            .find(|d| d.id == id)
    }

    /// Returns the group name containing a given dependency id (for nicer UI grouping).
    pub fn group_name_for(&self, id: &str) -> Option<&str> {
        self.dependencies
            .values
            .iter()
            .find(|g| g.values.iter().any(|d| d.id == id))
            .map(|g| g.name.as_str())
    }
}

/// A list of selectable metadata values (boot versions, java versions, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValues {
    /// The selectable entries.
    #[serde(default)]
    pub values: Vec<MetadataValue>,
    /// Initializr's default choice.
    #[serde(default)]
    pub default: String,
}

/// A single selectable metadata entry (one boot version, one java version, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataValue {
    /// The id used in API requests (e.g. `3.5.0`).
    #[serde(default)]
    pub id: String,
    /// Human-readable name (e.g. `3.5.0 (SNAPSHOT)`).
    #[serde(default)]
    pub name: String,
    /// Optional hyperlink to docs / release notes.
    #[serde(default)]
    pub action: Option<String>,
}

impl MetadataValue {
    /// True if this entry is a stable release (no SNAPSHOT, no M/RC suffix).
    pub fn stable(&self) -> bool {
        let n = self.id.to_ascii_uppercase();
        !(n.contains("SNAPSHOT") || n.contains("-M") || n.contains("-RC") || n.contains("PRE"))
    }
}

/// Project-type list (Maven / Gradle Groovy / Gradle Kotlin).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTypeValues {
    /// The selectable entries.
    #[serde(default)]
    pub values: Vec<ProjectType>,
    /// Initializr's default choice.
    #[serde(default)]
    pub default: String,
}

/// A single project type entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectType {
    /// The id used in API requests (e.g. `maven-build`).
    #[serde(default)]
    pub id: String,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// The action URL for this type (typically `/starter.zip`).
    #[serde(default)]
    pub action: Option<String>,
    /// Optional tags (e.g. `build=maven`).
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

/// A category of dependencies (`Web`, `SQL`, `NoSQL`, ...).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGroups {
    /// All category groups.
    #[serde(default)]
    pub values: Vec<DependencyGroup>,
}

/// A category group (e.g. "Web", "SQL").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGroup {
    /// Category name.
    #[serde(default)]
    pub name: String,
    /// Dependencies in this category.
    #[serde(default)]
    pub values: Vec<Dependency>,
}

/// A single Initializr dependency entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    /// Dependency id (e.g. `web`, `data-jpa`).
    #[serde(default)]
    pub id: String,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// One-line description.
    #[serde(default)]
    pub description: String,
    /// Optional compatibility version range (e.g. `Spring Boot >=2.0.0`).
    #[serde(default)]
    pub version_range: Option<String>,
    /// Optional hyperlinks to docs.
    #[serde(default)]
    pub links: Vec<Link>,
}

/// A documentation link attached to a dependency.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Link {
    /// Link relation (e.g. `reference`, `guide`).
    #[serde(default)]
    pub rel: String,
    /// Title text.
    #[serde(default)]
    pub title: String,
    /// Target URL.
    #[serde(default)]
    pub href: String,
}

/// Wrapper used for default-value fields in the metadata document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataDefault {
    /// The default text value.
    #[serde(default)]
    pub default: String,
}

/// What kind of metadata a `MetadataValue` represents — used for error reporting and UI labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataType {
    /// Spring Boot version.
    BootVersion,
    /// Java version.
    JavaVersion,
    /// Source language.
    Language,
    /// Packaging format.
    Packaging,
}

impl MetadataType {
    /// The Initializr JSON field name to look up for this metadata kind.
    pub fn field(&self) -> &'static str {
        match self {
            MetadataType::BootVersion => "bootVersion",
            MetadataType::JavaVersion => "javaVersion",
            MetadataType::Language => "language",
            MetadataType::Packaging => "packaging",
        }
    }
}

/// A version range constraint — used to express compatibility (`Spring Boot >=2.0.0 and <4.0.0`).
///
/// Initializr's `versionRange` syntax is a compact string like `2.0.0.M1` or `[2.0.0,4.0.0-M1)`.
/// We parse it loosely into a `[low, high)` representation; the spec does not require us to
/// enforce these constraints, only to *display* them when relevant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    /// Lower bound (inclusive), if any.
    pub low: Option<String>,
    /// Upper bound (exclusive), if any.
    pub high: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_metadata() {
        let json = r#"{
            "bootVersion": {"default": "3.5.0", "values": [
                {"id": "3.5.0", "name": "3.5.0"},
                {"id": "3.5.0-SNAPSHOT", "name": "3.5.0 (SNAPSHOT)"}
            ]},
            "javaVersion": {"default": "17", "values": [
                {"id": "17", "name": "17"},
                {"id": "21", "name": "21"}
            ]},
            "dependencies": {"values": [
                {"name": "Web", "values": [
                    {"id": "web", "name": "Spring Web", "description": "Build RESTful web apps"}
                ]},
                {"name": "SQL", "values": [
                    {"id": "data-jpa", "name": "Spring Data JPA", "description": "JPA persistence"}
                ]}
            ]}
        }"#;
        let m: InitializrMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(m.latest_stable_boot_version(), Some("3.5.0"));
        assert_eq!(m.all_dependency_ids(), vec!["web", "data-jpa"]);
        assert!(m.find_dependency("web").is_some());
        assert_eq!(m.group_name_for("data-jpa"), Some("SQL"));
    }

    #[test]
    fn stable_flag_excludes_snapshots() {
        let v = MetadataValue {
            id: "3.5.0-SNAPSHOT".into(),
            name: "3.5.0 (SNAPSHOT)".into(),
            action: None,
        };
        assert!(!v.stable());

        let v = MetadataValue {
            id: "3.5.0".into(),
            name: "3.5.0".into(),
            action: None,
        };
        assert!(v.stable());
    }
}
