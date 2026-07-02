//! The custom template layer that runs *after* the Initializr base project is extracted.
//!
//! Responsibilities:
//! - Resolve and render embedded templates (from [`springup_templates`]) using [`minijinja`].
//! - Derive a [`TemplateContext`] from a [`ProjectPlan`] so every template sees a consistent view
//!   of "what's been chosen", including which Initializr dependencies are present.
//! - Provide the high-level [`TemplateRenderer::apply_extras`] entry point that walks a plan's
//!   extras + architecture and writes the appropriate files into the output directory.

use std::path::{Path, PathBuf};

use minijinja::{Environment, Value};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{Error, Result};
use crate::plan::{ArchitectureKind, ExtraFeature, Language, Packaging, ProjectPlan};

/// Render context derived from a [`ProjectPlan`]. Passed into every template render call.
///
/// Template fields use `snake_case` so they read naturally inside `{{ }}` blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Maven group id.
    pub group_id: String,
    /// Maven artifact id.
    pub artifact_id: String,
    /// Project name.
    pub name: String,
    /// Project description.
    pub description: String,
    /// Base Java package.
    pub package_name: String,
    /// Package path (`dev.foo.bar` -> `dev/foo/bar`).
    pub package_path: String,
    /// Spring Boot version.
    pub spring_boot_version: String,
    /// Build tool slug (`maven` | `gradle` | `gradle-kotlin`).
    pub build_tool: String,
    /// Language slug (`java` | `kotlin`).
    pub language: String,
    /// File extension for the chosen language.
    pub language_ext: String,
    /// Java version.
    pub java_version: String,
    /// Packaging slug (`jar` | `war`).
    pub packaging: String,
    /// Initializr dependency ids, lowercased.
    pub dependencies: Vec<String>,
    /// Architecture kind slug (`none` | `layered` | `hexagonal`), or `None`.
    pub architecture: Option<String>,
    /// Enabled extra slugs.
    pub extras: Vec<String>,
    /// Current year (for copyright headers).
    pub year: i32,
    /// `springup` version (for manifest / badges).
    pub springup_version: String,
    // --- Dependency booleans (convenience for `{% if has_web %}` in templates) ---
    /// True if `web` selected.
    pub has_web: bool,
    /// True if `data-jpa` selected.
    pub has_data_jpa: bool,
    /// True if `data-mongodb` selected.
    pub has_data_mongodb: bool,
    /// True if `data-redis` selected.
    pub has_data_redis: bool,
    /// True if `postgresql` selected.
    pub has_postgresql: bool,
    /// True if `mysql` selected.
    pub has_mysql: bool,
    /// True if `security` selected.
    pub has_security: bool,
    /// True if `validation` selected.
    pub has_validation: bool,
    /// True if `actuator` selected.
    pub has_actuator: bool,
    /// True if `flyway` selected.
    pub has_flyway: bool,
    /// True if `liquibase` selected.
    pub has_liquibase: bool,
}

impl TemplateContext {
    /// Build a context from a [`ProjectPlan`].
    pub fn from_plan(plan: &ProjectPlan) -> Self {
        let deps_lower: Vec<String> = plan
            .dependencies
            .iter()
            .map(|d| d.to_ascii_lowercase())
            .collect();
        let has = |id: &str| deps_lower.iter().any(|d| d == id);
        let pkg_path = plan.package_path().to_string_lossy().to_string();
        let year = chrono::Utc::now()
            .format("%Y")
            .to_string()
            .parse()
            .unwrap_or(2026);
        Self {
            group_id: plan.group_id.clone(),
            artifact_id: plan.artifact_id.clone(),
            name: plan.name.clone(),
            description: plan.description.clone(),
            package_name: plan.package_name.clone(),
            package_path: pkg_path,
            spring_boot_version: plan.spring_boot_version.clone(),
            build_tool: plan.build_tool.slug().to_string(),
            language: match plan.language {
                Language::Java => "java",
                Language::Kotlin => "kotlin",
            }
            .to_string(),
            language_ext: plan.language.ext().to_string(),
            java_version: plan.java_version.clone(),
            packaging: match plan.packaging {
                Packaging::Jar => "jar",
                Packaging::War => "war",
            }
            .to_string(),
            dependencies: plan.dependencies.clone(),
            architecture: plan.architecture.map(|a| match a {
                ArchitectureKind::Layered => "layered".to_string(),
                ArchitectureKind::Hexagonal => "hexagonal".to_string(),
            }),
            extras: plan.extras.iter().map(|e| e.slug().to_string()).collect(),
            year,
            springup_version: env!("CARGO_PKG_VERSION").to_string(),
            has_web: has("web"),
            has_data_jpa: has("data-jpa"),
            has_data_mongodb: has("data-mongodb"),
            has_data_redis: has("data-redis"),
            has_postgresql: has("postgresql"),
            has_mysql: has("mysql"),
            has_security: has("security"),
            has_validation: has("validation"),
            has_actuator: has("actuator"),
            has_flyway: has("flyway"),
            has_liquibase: has("liquibase"),
        }
    }
}

/// High-level template renderer that wraps a `minijinja` environment with embedded templates.
pub struct TemplateRenderer {
    env: Environment<'static>,
}

impl std::fmt::Debug for TemplateRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateRenderer")
            .field("env", &"<minijinja::Environment>")
            .finish()
    }
}

impl TemplateRenderer {
    /// Construct a renderer with all embedded templates loaded.
    pub fn new() -> Self {
        let mut env = Environment::new();
        // Register every embedded template by name. Names are the asset paths in
        // springup-templates, prefixed with a namespace for clarity.
        for path in springup_templates::Asset::iter() {
            let key = path.to_string();
            if let Some(asset) = springup_templates::Asset::get(&path) {
                let source = String::from_utf8_lossy(asset.data.as_ref()).to_string();
                // Best-effort registration; invalid templates will surface on render.
                let _ = env.add_template_owned(key.clone(), source);
            }
        }
        Self { env }
    }

    /// Render a named template with a context.
    pub fn render(&self, name: &str, ctx: &TemplateContext) -> Result<String> {
        let value = Value::from_serialize(ctx);
        self.env
            .get_template(name)
            .map_err(|e| Error::TemplateRenderError {
                template: name.into(),
                message: format!("template not found: {e}"),
            })?
            .render(value)
            .map_err(|e| Error::TemplateRenderError {
                template: name.into(),
                message: e.to_string(),
            })
    }

    /// Apply the entire custom template layer (architecture + extras) to `output_dir`.
    ///
    /// This is the main entry point called by the `new` command after the Initializr base
    /// project has been extracted.
    pub fn apply_extras(&self, plan: &ProjectPlan, output_dir: &Path) -> Result<AppliedFiles> {
        let ctx = TemplateContext::from_plan(plan);
        let mut applied = AppliedFiles::default();
        // 1. Architecture skeleton
        if let Some(arch) = plan.architecture {
            match arch {
                ArchitectureKind::Layered => {
                    self.apply_layered(&ctx, output_dir, &mut applied)?;
                }
                ArchitectureKind::Hexagonal => {
                    self.apply_hexagonal(&ctx, output_dir, &mut applied)?;
                }
            }
        }
        // 2. Extras
        for extra in &plan.extras {
            match extra {
                ExtraFeature::Dockerfile => {
                    self.apply_dockerfile(&ctx, output_dir, &mut applied)?
                }
                ExtraFeature::DockerCompose => {
                    self.apply_docker_compose(&ctx, output_dir, &mut applied)?
                }
                ExtraFeature::GithubActionsCi => {
                    self.apply_github_actions(&ctx, output_dir, &mut applied)?
                }
                ExtraFeature::ConfigProfiles => {
                    self.apply_config_profiles(&ctx, output_dir, &mut applied)?
                }
                ExtraFeature::EditorConfig => {
                    self.apply_editorconfig(&ctx, output_dir, &mut applied)?
                }
                ExtraFeature::Readme => self.apply_readme(&ctx, output_dir, &mut applied)?,
            }
        }
        Ok(applied)
    }

    fn write_rendered(
        &self,
        rel_path: &str,
        output_dir: &Path,
        template: &str,
        ctx: &TemplateContext,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        let rendered = self.render(template, ctx)?;
        let path = output_dir.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, rendered)?;
        debug!("wrote {}", path.display());
        applied.files.push(PathBuf::from(rel_path));
        Ok(())
    }

    fn write_raw(
        &self,
        rel_path: &str,
        output_dir: &Path,
        template: &str,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        let asset = springup_templates::Asset::get(template)
            .ok_or_else(|| Error::MissingTemplateAsset(template.into()))?;
        let path = output_dir.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, asset.data.as_ref())?;
        debug!("wrote {}", path.display());
        applied.files.push(PathBuf::from(rel_path));
        Ok(())
    }

    fn apply_layered(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        // Architecture files live under src/main/java/<package>/{controller,service,...}.
        let pkg = &ctx.package_path;
        let lang_ext = &ctx.language_ext;
        let lang_dir = match ctx.language.as_str() {
            "kotlin" => "kotlin",
            _ => "java",
        };
        let main = format!("src/main/{lang_dir}/{pkg}");

        // Global exception handler
        self.write_rendered(
            &format!("{main}/exception/GlobalExceptionHandler.{lang_ext}"),
            output_dir,
            "architectures/layered/exception/GlobalExceptionHandler.j2",
            ctx,
            applied,
        )?;
        // ApiResponse wrapper DTO
        self.write_rendered(
            &format!("{main}/dto/ApiResponse.{lang_ext}"),
            output_dir,
            "architectures/layered/dto/ApiResponse.j2",
            ctx,
            applied,
        )?;
        // Health check controller (always present)
        self.write_rendered(
            &format!("{main}/controller/HealthController.{lang_ext}"),
            output_dir,
            "architectures/layered/controller/HealthController.j2",
            ctx,
            applied,
        )?;
        // Sample CRUD slice, only if data-jpa is present
        if ctx.has_data_jpa {
            self.write_rendered(
                &format!("{main}/entity/SampleEntity.{lang_ext}"),
                output_dir,
                "architectures/layered/entity/SampleEntity.j2",
                ctx,
                applied,
            )?;
            self.write_rendered(
                &format!("{main}/repository/SampleRepository.{lang_ext}"),
                output_dir,
                "architectures/layered/repository/SampleRepository.j2",
                ctx,
                applied,
            )?;
            self.write_rendered(
                &format!("{main}/service/SampleService.{lang_ext}"),
                output_dir,
                "architectures/layered/service/SampleService.j2",
                ctx,
                applied,
            )?;
            self.write_rendered(
                &format!("{main}/controller/SampleController.{lang_ext}"),
                output_dir,
                "architectures/layered/controller/SampleController.j2",
                ctx,
                applied,
            )?;
        }
        Ok(())
    }

    fn apply_hexagonal(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        let pkg = &ctx.package_path;
        let lang_ext = &ctx.language_ext;
        let lang_dir = match ctx.language.as_str() {
            "kotlin" => "kotlin",
            _ => "java",
        };
        let main = format!("src/main/{lang_dir}/{pkg}");
        self.write_rendered(
            &format!("{main}/domain/model/Sample.{lang_ext}"),
            output_dir,
            "architectures/hexagonal/domain/model/Sample.j2",
            ctx,
            applied,
        )?;
        self.write_rendered(
            &format!("{main}/domain/port/in/GetSampleUseCase.{lang_ext}"),
            output_dir,
            "architectures/hexagonal/domain/port/in/GetSampleUseCase.j2",
            ctx,
            applied,
        )?;
        self.write_rendered(
            &format!("{main}/domain/port/out/SampleRepository.{lang_ext}"),
            output_dir,
            "architectures/hexagonal/domain/port/out/SampleRepository.j2",
            ctx,
            applied,
        )?;
        self.write_rendered(
            &format!("{main}/application/GetSampleService.{lang_ext}"),
            output_dir,
            "architectures/hexagonal/application/GetSampleService.j2",
            ctx,
            applied,
        )?;
        self.write_rendered(
            &format!("{main}/adapter/in/web/SampleController.{lang_ext}"),
            output_dir,
            "architectures/hexagonal/adapter/in/web/SampleController.j2",
            ctx,
            applied,
        )?;
        if ctx.has_data_jpa {
            self.write_rendered(
                &format!("{main}/adapter/out/persistence/SampleEntity.{lang_ext}"),
                output_dir,
                "architectures/hexagonal/adapter/out/persistence/SampleEntity.j2",
                ctx,
                applied,
            )?;
            self.write_rendered(
                &format!("{main}/adapter/out/persistence/SampleJpaRepository.{lang_ext}"),
                output_dir,
                "architectures/hexagonal/adapter/out/persistence/SampleJpaRepository.j2",
                ctx,
                applied,
            )?;
            self.write_rendered(
                &format!("{main}/adapter/out/persistence/SamplePersistenceAdapter.{lang_ext}"),
                output_dir,
                "architectures/hexagonal/adapter/out/persistence/SamplePersistenceAdapter.j2",
                ctx,
                applied,
            )?;
        }
        Ok(())
    }

    fn apply_dockerfile(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        let template = match ctx.build_tool.as_str() {
            "gradle" | "gradle-kotlin" => "docker/Dockerfile.gradle.j2",
            _ => "docker/Dockerfile.maven.j2",
        };
        self.write_rendered("Dockerfile", output_dir, template, ctx, applied)?;
        self.write_raw(".dockerignore", output_dir, "docker/.dockerignore", applied)?;
        Ok(())
    }

    fn apply_docker_compose(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        self.write_rendered(
            "docker-compose.yml",
            output_dir,
            "docker/docker-compose.yml.j2",
            ctx,
            applied,
        )
    }

    fn apply_github_actions(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        let template = match ctx.build_tool.as_str() {
            "gradle" | "gradle-kotlin" => "ci/github-actions/ci.gradle.yml.j2",
            _ => "ci/github-actions/ci.maven.yml.j2",
        };
        self.write_rendered(
            ".github/workflows/ci.yml",
            output_dir,
            template,
            ctx,
            applied,
        )
    }

    fn apply_config_profiles(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        self.write_rendered(
            "src/main/resources/application-dev.yml",
            output_dir,
            "config-profiles/application-dev.yml.j2",
            ctx,
            applied,
        )?;
        self.write_rendered(
            "src/main/resources/application-prod.yml",
            output_dir,
            "config-profiles/application-prod.yml.j2",
            ctx,
            applied,
        )?;
        Ok(())
    }

    fn apply_editorconfig(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        self.write_rendered(
            ".editorconfig",
            output_dir,
            "editorconfig/.editorconfig.j2",
            ctx,
            applied,
        )
    }

    fn apply_readme(
        &self,
        ctx: &TemplateContext,
        output_dir: &Path,
        applied: &mut AppliedFiles,
    ) -> Result<()> {
        self.write_rendered("README.md", output_dir, "readme/README.md.j2", ctx, applied)?;
        // .gitignore — we use a raw asset (not templated) for stability
        self.write_raw(".gitignore", output_dir, "git/gitignore", applied)?;
        Ok(())
    }
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks every file written by [`TemplateRenderer::apply_extras`] — used for summary output
/// and snapshot tests.
#[derive(Debug, Default, Clone)]
pub struct AppliedFiles {
    /// Relative paths (relative to the output dir) of every file written.
    pub files: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::BuildTool;
    use std::path::PathBuf;

    fn sample_plan(arch: Option<ArchitectureKind>, extras: Vec<ExtraFeature>) -> ProjectPlan {
        ProjectPlan {
            group_id: "com.example".into(),
            artifact_id: "demo".into(),
            name: "demo".into(),
            description: "A demo service".into(),
            package_name: "com.example.demo".into(),
            spring_boot_version: "3.5.0".into(),
            build_tool: BuildTool::Maven,
            language: Language::Java,
            java_version: "21".into(),
            packaging: Packaging::Jar,
            dependencies: vec!["web".into(), "data-jpa".into(), "postgresql".into()],
            architecture: arch,
            extras,
            output_dir: PathBuf::from("./demo"),
            git_init: false,
            initial_commit: false,
        }
    }

    #[test]
    fn context_has_correct_flags() {
        let p = sample_plan(None, vec![]);
        let ctx = TemplateContext::from_plan(&p);
        assert!(ctx.has_web);
        assert!(ctx.has_data_jpa);
        assert!(ctx.has_postgresql);
        assert!(!ctx.has_security);
    }

    #[test]
    fn apply_layered_with_jpa_writes_full_slice() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = sample_plan(Some(ArchitectureKind::Layered), vec![]);
        let r = TemplateRenderer::new();
        let applied = r.apply_extras(&plan, tmp.path()).unwrap();
        assert!(applied.files.iter().any(|p| p
            .to_string_lossy()
            .contains("exception/GlobalExceptionHandler.java")));
        assert!(applied
            .files
            .iter()
            .any(|p| p.to_string_lossy().contains("entity/SampleEntity.java")));
        assert!(applied.files.iter().any(|p| p
            .to_string_lossy()
            .contains("controller/SampleController.java")));
    }

    #[test]
    fn apply_docker_extra_writes_dockerfile() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = sample_plan(None, vec![ExtraFeature::Dockerfile]);
        let r = TemplateRenderer::new();
        let applied = r.apply_extras(&plan, tmp.path()).unwrap();
        assert!(applied
            .files
            .iter()
            .any(|p| p.to_string_lossy() == "Dockerfile"));
        assert!(applied
            .files
            .iter()
            .any(|p| p.to_string_lossy() == ".dockerignore"));
        let dockerfile = std::fs::read_to_string(tmp.path().join("Dockerfile")).unwrap();
        assert!(dockerfile.contains("FROM"));
        assert!(dockerfile.contains("21")); // java version
    }

    #[test]
    fn apply_config_profiles_writes_yml() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = sample_plan(None, vec![ExtraFeature::ConfigProfiles]);
        let r = TemplateRenderer::new();
        let applied = r.apply_extras(&plan, tmp.path()).unwrap();
        assert!(applied
            .files
            .iter()
            .any(|p| p.to_string_lossy() == "src/main/resources/application-dev.yml"));
        assert!(applied
            .files
            .iter()
            .any(|p| p.to_string_lossy() == "src/main/resources/application-prod.yml"));
    }

    #[test]
    fn apply_readme_includes_project_name() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = sample_plan(None, vec![ExtraFeature::Readme]);
        let r = TemplateRenderer::new();
        r.apply_extras(&plan, tmp.path()).unwrap();
        let readme = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
        assert!(readme.contains("demo"));
        assert!(readme.contains("mvnw") || readme.contains("mvn"));
    }

    #[test]
    fn apply_github_actions_writes_ci_yml() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = sample_plan(None, vec![ExtraFeature::GithubActionsCi]);
        let r = TemplateRenderer::new();
        r.apply_extras(&plan, tmp.path()).unwrap();
        let ci = std::fs::read_to_string(tmp.path().join(".github/workflows/ci.yml")).unwrap();
        assert!(ci.contains("on:"));
        assert!(ci.contains("verify") || ci.contains("build"));
    }
}
