//! Interactive prompt flow for `springup new`.
//!
//! Wraps a [`PlanBuilder`](crate::commands::new::PlanBuilder) with `dialoguer` prompts.
//! Each step pre-fills / skips itself when the corresponding flag was already supplied, so
//! partial flags + wizard is a first-class UX (spec §4).

use std::io::IsTerminal;

use color_eyre::eyre;
use dialoguer::{Confirm, Input, MultiSelect, Select};

use springup_core::config::Config;
use springup_core::initializr::InitializrMetadata;
use springup_core::plan::{
    ArchitectureKind, BuildTool, ExtraFeature, Language, Packaging, ProjectPlan,
};

use crate::cli::NewArgs;
use crate::commands::new::PlanBuilder;
use crate::ui::theme;

/// Run the interactive wizard and return a fully-resolved [`ProjectPlan`].
pub fn run(
    mut b: PlanBuilder,
    metadata: &InitializrMetadata,
    args: &NewArgs,
    cfg: &Config,
) -> color_eyre::Result<ProjectPlan> {
    // 1. Project metadata: name, group, artifact, description, package
    if b.name.is_none() {
        let default_name = if metadata.name.default.is_empty() {
            "demo".to_string()
        } else {
            metadata.name.default.clone()
        };
        let name: String = Input::new()
            .with_prompt("Project name")
            .default(default_name)
            .interact_text()?;
        b.name = Some(name);
    }

    if b.artifact_id.is_none() {
        let default = b.name.clone().unwrap_or_default();
        let artifact: String = Input::new()
            .with_prompt("Artifact id")
            .default(default)
            .interact_text()?;
        b.artifact_id = Some(artifact);
    }

    if b.group_id.is_none() {
        let default = cfg.group_id.clone();
        let group: String = Input::new()
            .with_prompt("Group id")
            .default(default)
            .interact_text()?;
        b.group_id = Some(group);
    }

    if b.description.is_none() {
        let desc: String = Input::new()
            .with_prompt("Description (optional)")
            .allow_empty(true)
            .default(String::new())
            .interact_text()?;
        b.description = Some(desc);
    }

    if b.package_name.is_none() {
        let group = b.group_id.clone().unwrap_or_default();
        let artifact = b.artifact_id.clone().unwrap_or_default();
        let default_pkg = format!("{}.{}", group, artifact.replace('-', "."));
        let pkg: String = Input::new()
            .with_prompt("Package name")
            .default(default_pkg)
            .interact_text()?;
        b.package_name = Some(pkg);
    }

    // 2. Spring Boot version
    if b.spring_boot_version.is_none() {
        let versions: Vec<&str> = metadata
            .boot_version
            .values
            .iter()
            .map(|v| v.id.as_str())
            .collect();
        let default_idx = metadata
            .boot_version
            .values
            .iter()
            .position(|v| v.id == metadata.boot_version.default)
            .unwrap_or(0);
        let selection = Select::new()
            .with_prompt("Spring Boot version")
            .items(&versions)
            .default(default_idx)
            .interact()?;
        b.spring_boot_version = Some(versions[selection].to_string());
    }

    // 3. Build tool
    if b.build_tool.is_none() {
        let items = ["Maven", "Gradle (Groovy)", "Gradle (Kotlin)"];
        let selection = Select::new()
            .with_prompt("Build tool")
            .items(&items)
            .default(0)
            .interact()?;
        b.build_tool = Some(match selection {
            0 => BuildTool::Maven,
            1 => BuildTool::GradleGroovy,
            2 => BuildTool::GradleKotlin,
            _ => BuildTool::Maven,
        });
    }

    // 4. Language
    if b.language.is_none() {
        let items = ["Java", "Kotlin"];
        let selection = Select::new()
            .with_prompt("Language")
            .items(&items)
            .default(0)
            .interact()?;
        b.language = Some(match selection {
            0 => Language::Java,
            1 => Language::Kotlin,
            _ => Language::Java,
        });
    }

    // 5. Java version
    if b.java_version.is_none() {
        let versions: Vec<&str> = metadata
            .java_version
            .values
            .iter()
            .map(|v| v.id.as_str())
            .collect();
        let default_idx = metadata
            .java_version
            .values
            .iter()
            .position(|v| v.id == metadata.java_version.default)
            .unwrap_or(0);
        let selection = Select::new()
            .with_prompt("Java version")
            .items(&versions)
            .default(default_idx)
            .interact()?;
        b.java_version = Some(versions[selection].to_string());
    }

    // 6. Packaging
    if b.packaging.is_none() {
        let items = ["Jar", "War"];
        let selection = Select::new()
            .with_prompt("Packaging")
            .items(&items)
            .default(0)
            .interact()?;
        b.packaging = Some(match selection {
            0 => Packaging::Jar,
            1 => Packaging::War,
            _ => Packaging::Jar,
        });
    }

    // 7. Dependencies (multi-select, grouped by category)
    if b.dependencies.is_empty() {
        b.dependencies = prompt_dependencies(metadata)?;
    }

    // 8. Architecture skeleton
    if b.architecture.is_none() && !args.yes {
        let items = ["none", "layered", "hexagonal"];
        let selection = Select::new()
            .with_prompt("Architecture skeleton")
            .items(&items)
            .default(0)
            .interact()?;
        b.architecture = match selection {
            0 => None,
            1 => Some(ArchitectureKind::Layered),
            2 => Some(ArchitectureKind::Hexagonal),
            _ => None,
        };
    }

    // 9. Extras
    if b.extras.is_empty() && !args.yes {
        b.extras = prompt_extras()?;
    }

    // Output dir, git, commit — only prompt if not supplied and not --yes
    if b.output_dir.is_none() {
        let artifact = b.artifact_id.clone().unwrap_or_else(|| "demo".into());
        let default = format!("./{artifact}");
        let out: String = Input::new()
            .with_prompt("Output directory")
            .default(default)
            .interact_text()?;
        b.output_dir = Some(std::path::PathBuf::from(out));
    }
    if b.git_init.is_none() {
        b.git_init = Some(
            Confirm::new()
                .with_prompt("Initialize a git repository?")
                .default(true)
                .interact()?,
        );
    }
    if b.initial_commit.is_none() && b.git_init == Some(true) {
        b.initial_commit = Some(
            Confirm::new()
                .with_prompt("Make an initial commit?")
                .default(false)
                .interact()?,
        );
    }

    // Build the final plan
    let _is_tty = std::io::stdin().is_terminal();
    let _ = theme::accent(); // touch theme to avoid dead_code warnings if branch skipped

    let plan = ProjectPlan {
        group_id: b
            .group_id
            .ok_or_else(|| eyre::eyre!("internal: group_id not resolved"))?,
        artifact_id: b
            .artifact_id
            .ok_or_else(|| eyre::eyre!("internal: artifact_id not resolved"))?,
        name: b
            .name
            .ok_or_else(|| eyre::eyre!("internal: name not resolved"))?,
        description: b.description.unwrap_or_default(),
        package_name: b
            .package_name
            .ok_or_else(|| eyre::eyre!("internal: package_name not resolved"))?,
        spring_boot_version: b
            .spring_boot_version
            .ok_or_else(|| eyre::eyre!("internal: spring_boot_version not resolved"))?,
        build_tool: b
            .build_tool
            .ok_or_else(|| eyre::eyre!("internal: build_tool not resolved"))?,
        language: b
            .language
            .ok_or_else(|| eyre::eyre!("internal: language not resolved"))?,
        java_version: b
            .java_version
            .ok_or_else(|| eyre::eyre!("internal: java_version not resolved"))?,
        packaging: b
            .packaging
            .ok_or_else(|| eyre::eyre!("internal: packaging not resolved"))?,
        dependencies: b.dependencies,
        architecture: b.architecture,
        extras: b.extras,
        output_dir: b
            .output_dir
            .ok_or_else(|| eyre::eyre!("internal: output_dir not resolved"))?,
        git_init: b.git_init.unwrap_or(true),
        initial_commit: b.initial_commit.unwrap_or(false),
    };
    plan.validate()?;
    Ok(plan)
}

fn prompt_dependencies(metadata: &InitializrMetadata) -> color_eyre::Result<Vec<String>> {
    // Flatten dependencies with category headers as interstitial items.
    let mut items: Vec<String> = Vec::new();
    let mut selectable: Vec<Option<String>> = Vec::new(); // None = header, Some = dep id
    let mut defaults: Vec<bool> = Vec::new();

    // Sort: web first (Initializr convention), then alphabetical by group name.
    let mut groups = metadata.dependencies.values.clone();
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    for g in &groups {
        if g.values.is_empty() {
            continue;
        }
        let header = format!("── {} ──", g.name);
        items.push(header.clone());
        selectable.push(None);
        defaults.push(false);
        for d in &g.values {
            let label = if d.description.is_empty() {
                format!("{} ({})", d.name, d.id)
            } else {
                format!("{} — {} ({})", d.name, d.description, d.id)
            };
            items.push(label);
            selectable.push(Some(d.id.clone()));
            defaults.push(false);
        }
    }

    let selection = MultiSelect::new()
        .with_prompt("Dependencies (space to toggle, enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    let mut chosen = Vec::new();
    for idx in selection {
        if let Some(id) = &selectable[idx] {
            chosen.push(id.clone());
        }
    }
    Ok(chosen)
}

fn prompt_extras() -> color_eyre::Result<Vec<ExtraFeature>> {
    let items: Vec<String> = ExtraFeature::ALL
        .iter()
        .map(|f| format!("{} — {}", f.slug(), extra_description(f)))
        .collect();
    let defaults: Vec<bool> = ExtraFeature::ALL
        .iter()
        .map(|f| matches!(f, ExtraFeature::Readme | ExtraFeature::EditorConfig))
        .collect();
    let selection = MultiSelect::new()
        .with_prompt("Extras")
        .items(&items)
        .defaults(&defaults)
        .interact()?;
    let mut out = Vec::new();
    for idx in selection {
        out.push(ExtraFeature::ALL[idx]);
    }
    Ok(out)
}

fn extra_description(f: &ExtraFeature) -> &'static str {
    match f {
        ExtraFeature::Dockerfile => "multi-stage Dockerfile + .dockerignore",
        ExtraFeature::DockerCompose => "docker-compose.yml with auto-detected services",
        ExtraFeature::GithubActionsCi => ".github/workflows/ci.yml build+test workflow",
        ExtraFeature::ConfigProfiles => "application-dev.yml + application-prod.yml",
        ExtraFeature::EditorConfig => ".editorconfig",
        ExtraFeature::Readme => "README.md with run instructions",
    }
}
