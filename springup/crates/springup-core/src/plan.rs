//! The `ProjectPlan` data model — the single source of truth for what gets generated.
//!
//! A `ProjectPlan` is the fully-resolved, validated description of a Spring Boot project to
//! scaffold. It is built by EITHER the interactive wizard OR non-interactive flag parsing, then
//! funneled through the same validation pipeline, so the two modes can never drift apart in what
//! they consider a legal configuration.
//!
//! The struct is `serde`-serializable so it doubles as the schema for a future `--from-file`
//! mode (reproducible batch generation) and as the on-disk [`crate::manifest`] format.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The dependency identifiers Spring Initializr understands (e.g. `web`, `data-jpa`, `security`).
///
/// We keep these as opaque strings rather than an exhaustive enum because Initializr's dependency
/// catalogue evolves between Spring Boot releases; an opaque id lets us pass new entries through
/// without a core-crate release. Validation against the live metadata catalogue happens in
/// [`crate::initializr`].
pub type DependencyId = String;

/// Build tool selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildTool {
    /// Apache Maven (`pom.xml` + `mvnw`).
    Maven,
    /// Gradle with Groovy DSL (`build.gradle`).
    GradleGroovy,
    /// Gradle with Kotlin DSL (`build.gradle.kts`).
    GradleKotlin,
}

impl BuildTool {
    /// The Initializr string code for this build tool.
    pub fn initializr_code(&self) -> &'static str {
        match self {
            BuildTool::Maven => "maven-build",
            BuildTool::GradleGroovy => "gradle-build",
            BuildTool::GradleKotlin => "gradle-build-kotlin",
        }
    }

    /// The lowercase slug used in config files / manifests.
    pub fn slug(&self) -> &'static str {
        match self {
            BuildTool::Maven => "maven",
            BuildTool::GradleGroovy => "gradle",
            BuildTool::GradleKotlin => "gradle-kotlin",
        }
    }

    /// The wrapper script filename shipped with generated projects.
    pub fn wrapper(&self) -> &'static str {
        match self {
            BuildTool::Maven => "./mvnw",
            BuildTool::GradleGroovy | BuildTool::GradleKotlin => "./gradlew",
        }
    }

    /// Command for the "verify" build goal.
    pub fn verify_command(&self) -> &'static str {
        match self {
            BuildTool::Maven => "verify",
            BuildTool::GradleGroovy | BuildTool::GradleKotlin => "build",
        }
    }

    /// Parse from a slug, returning `None` on miss (case-insensitive).
    pub fn from_slug(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "maven" => Some(Self::Maven),
            "gradle" | "gradle-groovy" => Some(Self::GradleGroovy),
            "gradle-kotlin" | "gradle_kotlin" => Some(Self::GradleKotlin),
            _ => None,
        }
    }
}

impl std::fmt::Display for BuildTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

impl std::str::FromStr for BuildTool {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_slug(s).ok_or_else(|| Error::InvalidProjectPlan {
            message: format!(
                "unknown build tool '{s}' (expected one of: maven, gradle, gradle-kotlin)"
            ),
        })
    }
}

/// Source language for generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Java.
    Java,
    /// Kotlin.
    Kotlin,
}

impl Language {
    /// Initializr string code.
    pub fn initializr_code(&self) -> &'static str {
        match self {
            Language::Java => "java",
            Language::Kotlin => "kotlin",
        }
    }

    /// File extension (without leading dot).
    pub fn ext(&self) -> &'static str {
        match self {
            Language::Java => "java",
            Language::Kotlin => "kt",
        }
    }
}

impl std::str::FromStr for Language {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "java" => Ok(Self::Java),
            "kotlin" | "kt" => Ok(Self::Kotlin),
            other => Err(Error::InvalidProjectPlan {
                message: format!("unknown language '{other}' (expected: java | kotlin)"),
            }),
        }
    }
}

/// Packaging format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Packaging {
    /// Executable JAR (default for Spring Boot).
    Jar,
    /// WAR for traditional servlet containers.
    War,
}

impl Packaging {
    /// Initializr string code.
    pub fn initializr_code(&self) -> &'static str {
        match self {
            Packaging::Jar => "jar",
            Packaging::War => "war",
        }
    }
}

impl std::str::FromStr for Packaging {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "jar" => Ok(Self::Jar),
            "war" => Ok(Self::War),
            other => Err(Error::InvalidProjectPlan {
                message: format!("unknown packaging '{other}' (expected: jar | war)"),
            }),
        }
    }
}

/// Architecture skeleton style for the custom template layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchitectureKind {
    /// Layered: `controller / service / repository / dto / entity / exception`.
    Layered,
    /// Hexagonal / ports-and-adapters: `domain / application / adapter/{in,out}`.
    Hexagonal,
}

impl ArchitectureKind {
    /// Parse from a slug, returning `None` on miss (case-insensitive).
    pub fn from_slug(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "layered" => Some(Self::Layered),
            "hexagonal" | "hex" | "ports-and-adapters" => Some(Self::Hexagonal),
            _ => None,
        }
    }

    /// All variants in spec order — useful for prompt UIs and `--help` text.
    pub const ALL: &'static [Self] = &[Self::Layered, Self::Hexagonal];
}

impl std::str::FromStr for ArchitectureKind {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_slug(s).ok_or_else(|| Error::InvalidProjectPlan {
            message: format!("unknown architecture '{s}' (expected: none | layered | hexagonal)"),
        })
    }
}

/// Optional extras layered on top of the Initializr base project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtraFeature {
    /// Multi-stage `Dockerfile` matching the chosen Java version + build tool.
    Dockerfile,
    /// `docker-compose.yml` with app + (optional) database service.
    DockerCompose,
    /// `.github/workflows/ci.yml` build-and-test workflow.
    GithubActionsCi,
    /// `application-dev.yml` / `application-prod.yml` profile split.
    ConfigProfiles,
    /// `.editorconfig` for consistent editor formatting.
    EditorConfig,
    /// Generated `README.md` with run instructions and badges.
    Readme,
}

impl ExtraFeature {
    /// Slug used in CLI flags, manifests, and config files.
    pub fn slug(&self) -> &'static str {
        match self {
            ExtraFeature::Dockerfile => "docker",
            ExtraFeature::DockerCompose => "docker-compose",
            ExtraFeature::GithubActionsCi => "ci",
            ExtraFeature::ConfigProfiles => "config-profiles",
            ExtraFeature::EditorConfig => "editorconfig",
            ExtraFeature::Readme => "readme",
        }
    }

    /// All variants in spec order.
    pub const ALL: &'static [Self] = &[
        Self::Dockerfile,
        Self::DockerCompose,
        Self::GithubActionsCi,
        Self::ConfigProfiles,
        Self::EditorConfig,
        Self::Readme,
    ];

    /// Parse a comma-separated list of slugs into a `Vec<ExtraFeature>`, returning an error on
    /// any unknown slug.
    pub fn parse_list(s: &str) -> Result<Vec<Self>> {
        let mut out = Vec::new();
        for tok in s.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let f = Self::from_slug(tok).ok_or_else(|| Error::InvalidProjectPlan {
                message: format!(
                    "unknown extra '{tok}' (expected one of: {})",
                    Self::ALL
                        .iter()
                        .map(ExtraFeature::slug)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })?;
            if !out.contains(&f) {
                out.push(f);
            }
        }
        Ok(out)
    }

    /// Parse a single slug, returning `None` on miss.
    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|f| f.slug().eq_ignore_ascii_case(s))
    }
}

/// The fully-resolved, validated description of what to generate.
///
/// Built by EITHER the wizard OR flag parsing — both paths converge here. Serialized form is the
/// schema for `springup.toml` manifests and a future `--from-file plan.toml` mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPlan {
    /// Maven group id, e.g. `dev.raghavarora`.
    pub group_id: String,
    /// Maven artifact id, e.g. `my-service`. Also used as the default output dir name.
    pub artifact_id: String,
    /// Human-readable project name. Defaults to the artifact id.
    pub name: String,
    /// One-line description for README / POM metadata.
    pub description: String,
    /// Base Java package, e.g. `dev.raghavarora.myservice`.
    pub package_name: String,
    /// Spring Boot version (e.g. `3.5.0`). Must match a version advertised by Initializr.
    pub spring_boot_version: String,
    /// Build tool selection.
    pub build_tool: BuildTool,
    /// Source language.
    pub language: Language,
    /// Java version (e.g. `17`, `21`).
    pub java_version: String,
    /// Packaging format.
    pub packaging: Packaging,
    /// Initializr dependency ids. Validated against live metadata before generation.
    pub dependencies: Vec<DependencyId>,
    /// Optional architecture skeleton.
    pub architecture: Option<ArchitectureKind>,
    /// Extras layered on top.
    pub extras: Vec<ExtraFeature>,
    /// Where the project should be written. Defaults to `./<artifact-id>`.
    pub output_dir: PathBuf,
    /// Whether to `git init` the output directory.
    pub git_init: bool,
    /// Whether to make an initial commit after `git init`.
    pub initial_commit: bool,
}

impl ProjectPlan {
    /// Validate the plan against static invariants (cross-field consistency).
    ///
    /// Dynamic validation against live Initializr metadata (e.g. "is `data-jpa` actually a real
    /// dependency id for Spring Boot 3.5.0?") is performed in [`crate::initializr`].
    pub fn validate(&self) -> Result<()> {
        validate_maven_coordinate(&self.group_id, "group-id")?;
        validate_maven_coordinate(&self.artifact_id, "artifact-id")?;

        if self.name.trim().is_empty() {
            return Err(Error::InvalidProjectPlan {
                message: "project name must not be empty".into(),
            });
        }

        validate_package_name(&self.package_name)?;

        if self.spring_boot_version.trim().is_empty() {
            return Err(Error::InvalidProjectPlan {
                message: "spring_boot_version must not be empty".into(),
            });
        }

        validate_java_version(&self.java_version)?;

        // Kotlin + Gradle Kotlin DSL is fine; Kotlin + Maven is technically supported by
        // Initializr but discouraged — warn via the plan? For now we accept it.

        if self.output_dir.as_os_str().is_empty() {
            return Err(Error::InvalidProjectPlan {
                message: "output_dir must not be empty".into(),
            });
        }

        // Deduplicate dependencies while preserving order.
        // (Not an error — just normalize.)
        Ok(())
    }

    /// Returns the default output directory for a given artifact id (`./<artifact-id>`).
    pub fn default_output_dir(artifact_id: &str) -> PathBuf {
        PathBuf::from(artifact_id)
    }

    /// True if the plan requests a given extra.
    pub fn has_extra(&self, f: ExtraFeature) -> bool {
        self.extras.contains(&f)
    }

    /// True if the plan includes the given Initializr dependency id.
    pub fn has_dep(&self, id: &str) -> bool {
        self.dependencies.iter().any(|d| d == id)
    }

    /// Returns the source main path under the output dir, e.g. `src/main/java/dev/foo/bar`.
    pub fn main_source_dir(&self) -> PathBuf {
        let lang_dir = match self.language {
            Language::Java => "java",
            Language::Kotlin => "kotlin",
        };
        let mut p = PathBuf::from("src").join("main").join(lang_dir);
        for seg in self.package_name.split('.') {
            p = p.join(seg);
        }
        p
    }

    /// Returns the source test path under the output dir.
    pub fn test_source_dir(&self) -> PathBuf {
        let lang_dir = match self.language {
            Language::Java => "java",
            Language::Kotlin => "kotlin",
        };
        let mut p = PathBuf::from("src").join("test").join(lang_dir);
        for seg in self.package_name.split('.') {
            p = p.join(seg);
        }
        p
    }

    /// Convert the package name to a relative path (`dev.foo.bar` -> `dev/foo/bar`).
    pub fn package_path(&self) -> PathBuf {
        let mut p = PathBuf::new();
        for seg in self.package_name.split('.') {
            p = p.join(seg);
        }
        p
    }

    /// Normalize: dedupe dependencies, sort extras deterministically.
    pub fn normalized(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        self.dependencies.retain(|d| seen.insert(d.clone()));

        self.extras.sort_by_key(ExtraFeature::slug);
        self.extras.dedup();
        self
    }
}

fn validate_maven_coordinate(s: &str, field: &'static str) -> Result<()> {
    if s.trim().is_empty() {
        return Err(Error::InvalidProjectPlan {
            message: format!("{field} must not be empty"),
        });
    }
    // Maven coordinate rules: letters, digits, dot, dash, underscore. No spaces, no slashes.
    let valid = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !valid {
        return Err(Error::InvalidProjectPlan {
            message: format!(
                "{field} '{s}' contains illegal characters (allowed: A-Z a-z 0-9 . - _)"
            ),
        });
    }
    Ok(())
}

fn validate_package_name(s: &str) -> Result<()> {
    if s.trim().is_empty() {
        return Err(Error::InvalidProjectPlan {
            message: "package_name must not be empty".into(),
        });
    }
    if s.starts_with('.') || s.ends_with('.') {
        return Err(Error::InvalidProjectPlan {
            message: format!("package_name '{s}' must not start or end with a dot"),
        });
    }
    for seg in s.split('.') {
        if seg.is_empty() {
            return Err(Error::InvalidProjectPlan {
                message: format!("package_name '{s}' contains an empty segment"),
            });
        }
        // Java reserved-word check would be overkill here; Initializr itself doesn't enforce it.
        if !seg.chars().next().unwrap().is_ascii_alphabetic() && !seg.starts_with('_') {
            return Err(Error::InvalidProjectPlan {
                message: format!(
                    "package_name segment '{seg}' must start with a letter or underscore"
                ),
            });
        }
        if !seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        {
            return Err(Error::InvalidProjectPlan {
                message: format!(
                    "package_name segment '{seg}' contains illegal characters (allowed: A-Z a-z 0-9 _ $)"
                ),
            });
        }
    }
    Ok(())
}

fn validate_java_version(s: &str) -> Result<()> {
    if s.trim().is_empty() {
        return Err(Error::InvalidProjectPlan {
            message: "java_version must not be empty".into(),
        });
    }
    // Accept numeric (8, 11, 17, 21) or `1.N` form.
    let ok = s
        .split('.')
        .all(|part| part.chars().all(|c| c.is_ascii_digit()))
        && !s.starts_with('.');
    if !ok {
        return Err(Error::InvalidProjectPlan {
            message: format!("java_version '{s}' must be numeric (e.g. 17, 21, 1.8)"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> ProjectPlan {
        ProjectPlan {
            group_id: "com.example".into(),
            artifact_id: "demo".into(),
            name: "demo".into(),
            description: "".into(),
            package_name: "com.example.demo".into(),
            spring_boot_version: "3.5.0".into(),
            build_tool: BuildTool::Maven,
            language: Language::Java,
            java_version: "21".into(),
            packaging: Packaging::Jar,
            dependencies: vec!["web".into()],
            architecture: None,
            extras: vec![],
            output_dir: PathBuf::from("./demo"),
            git_init: true,
            initial_commit: false,
        }
    }

    #[test]
    fn valid_plan_passes() {
        plan().validate().unwrap();
    }

    #[test]
    fn rejects_bad_group_id() {
        let mut p = plan();
        p.group_id = "com example".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn rejects_bad_package_name() {
        let mut p = plan();
        p.package_name = ".leading.dot".into();
        assert!(p.validate().is_err());

        p.package_name = "trailing.dot.".into();
        assert!(p.validate().is_err());

        p.package_name = "double..dot".into();
        assert!(p.validate().is_err());

        p.package_name = "1numeric.start".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn rejects_bad_java_version() {
        let mut p = plan();
        p.java_version = "abc".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn parse_extras_list() {
        let e = ExtraFeature::parse_list("docker,ci,config-profiles").unwrap();
        assert_eq!(
            e,
            vec![
                ExtraFeature::Dockerfile,
                ExtraFeature::GithubActionsCi,
                ExtraFeature::ConfigProfiles,
            ]
        );
    }

    #[test]
    fn parse_extras_dedupes() {
        let e = ExtraFeature::parse_list("docker,docker").unwrap();
        assert_eq!(e, vec![ExtraFeature::Dockerfile]);
    }

    #[test]
    fn parse_extras_rejects_unknown() {
        assert!(ExtraFeature::parse_list("docker,bogus").is_err());
    }

    #[test]
    fn build_tool_round_trip() {
        for t in [
            BuildTool::Maven,
            BuildTool::GradleGroovy,
            BuildTool::GradleKotlin,
        ] {
            assert_eq!(BuildTool::from_slug(t.slug()), Some(t));
        }
    }

    #[test]
    fn normalized_dedupes_deps_and_sorts_extras() {
        let mut p = plan();
        p.dependencies = vec!["web".into(), "web".into(), "data-jpa".into()];
        p.extras = vec![
            ExtraFeature::Readme,
            ExtraFeature::Dockerfile,
            ExtraFeature::Dockerfile,
        ];
        let n = p.normalized();
        assert_eq!(n.dependencies, vec!["web", "data-jpa"]);
        assert_eq!(
            n.extras,
            vec![ExtraFeature::Dockerfile, ExtraFeature::Readme]
        );
    }

    #[test]
    fn package_path_segments() {
        let p = plan();
        assert_eq!(p.package_path(), PathBuf::from("com/example/demo"));
    }
}
