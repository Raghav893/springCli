//! `springup new` — scaffold a new Spring Boot project.
//!
//! Mode resolution (spec §4):
//! - If stdin is not a TTY, OR `-y`/`--yes` is passed, OR enough flags are present to fully
//!   resolve a [`ProjectPlan`]: skip the wizard.
//! - If flags are partially given but insufficient: run the wizard but pre-fill/skip the
//!   answered steps.

use std::io::IsTerminal;
use std::path::PathBuf;

use color_eyre::eyre::{self, Context};
use tracing::{debug, info};

use springup_core::config::Config;
use springup_core::initializr::{InitializrClient, InitializrConfig, MetadataCache};
use springup_core::manifest::ProjectManifest;
use springup_core::plan::{
    ArchitectureKind, BuildTool, ExtraFeature, Language, Packaging, ProjectPlan,
};
use springup_core::template::{AppliedFiles, TemplateRenderer};

use crate::cli::NewArgs;
use crate::ui::{messages, summary, theme};
use crate::wizard;

/// Entry point invoked by `cli::Cli::parse_and_dispatch`.
pub fn run(args: NewArgs) -> color_eyre::Result<i32> {
    println!("{}", theme::heading().apply_to(messages::TAGLINE));
    println!();

    // Load global config (best-effort; fall back to defaults on error).
    let cfg = Config::load().unwrap_or_else(|e| {
        eprintln!("warning: could not load global config ({e}); using defaults");
        Config::default()
    });

    // Decide wizard vs non-interactive mode up front.
    let non_interactive = args.yes || !std::io::stdin().is_terminal();

    // Build a ProjectPlan from flags + config defaults.
    let plan_builder = plan_from_flags(&args, &cfg)?;

    // If --dry-run, print the plan (resolved as far as flags + config can take us) and exit.
    // For dry-run we still fetch metadata to fully validate, but skip the wizard.
    if args.dry_run {
        return dry_run(plan_builder, &args);
    }

    // Set up Initializr client config.
    let initializr_cfg = InitializrConfig {
        base_url: args.base_url.clone().unwrap_or_else(|| {
            cfg.initializr_base_url
                .clone()
                .unwrap_or_else(|| springup_core::initializr::DEFAULT_BASE_URL.into())
        }),
        offline: args.offline,
        refresh: args.refresh,
        timeout: None,
    };

    // The actual generation runs inside a tokio runtime.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let result: eyre::Result<(ProjectPlan, AppliedFiles, PathBuf)> = rt.block_on(async move {
        let cache = MetadataCache::new().context("creating metadata cache")?;
        let client = InitializrClient::new(initializr_cfg, cache)?;

        // Fetch metadata (cached or fresh). In non-interactive mode we fetch up front; in
        // interactive mode the wizard fetches concurrently while prompting.
        let metadata = if non_interactive {
            let pb = summary::spinner(messages::FETCHING_METADATA);
            let m = client
                .fetch_metadata()
                .await
                .context("fetching Initializr metadata")?;
            pb.finish_with_message("Metadata ready.");
            m
        } else {
            // Wizard will handle it.
            client
                .fetch_metadata()
                .await
                .context("fetching Initializr metadata")?
        };

        // Resolve the plan, optionally via the wizard.
        let plan = if non_interactive {
            plan_builder.resolve(&metadata, &client)?
        } else {
            println!("{}", theme::accent().apply_to(messages::WIZARD_INTRO));
            println!();
            wizard::run(plan_builder, &metadata, &args, &cfg)?
        };

        // Validate dependencies against metadata.
        client.validate_dependencies(&metadata, &plan)?;

        // Download + extract starter zip.
        let pb = summary::spinner(messages::DOWNLOADING_STARTER);
        let zip_bytes = client
            .download_starter_zip(&plan)
            .await
            .context("downloading starter zip")?;
        pb.finish_with_message("Starter downloaded.");

        // Determine output dir + ensure it doesn't already exist (or is empty).
        let output_dir = plan.output_dir.clone();
        if output_dir.exists() && output_dir.is_dir() {
            let count = std::fs::read_dir(&output_dir)
                .map(|d| d.count())
                .unwrap_or(0);
            if count > 0 {
                return Err(eyre::eyre!(
                    "output directory '{}' is not empty ({} entries); refusing to overwrite",
                    output_dir.display(),
                    count
                ));
            }
        }
        std::fs::create_dir_all(&output_dir)?;

        // Extract base project.
        let pb = summary::spinner("Extracting base project…");
        springup_core::initializr::client::extract_zip(&zip_bytes, &output_dir)
            .context("extracting starter zip")?;
        pb.finish_with_message("Base project extracted.");

        // Apply custom template layer.
        let pb = summary::spinner(messages::APPLYING_EXTRAS);
        let renderer = TemplateRenderer::new();
        let applied = renderer.apply_extras(&plan, &output_dir)?;
        pb.finish_with_message("Custom layer applied.");

        // Write springup.toml manifest.
        let manifest = ProjectManifest::from_plan(&plan);
        manifest.write_to_dir(&output_dir)?;
        debug!("wrote springup.toml");

        // Git init + initial commit (optional).
        if plan.git_init {
            git_init(&output_dir, plan.initial_commit)?;
        }

        Ok((plan, applied, output_dir))
    });

    let (plan, applied, output_dir) = result?;

    summary::print_summary(&plan, &applied, &output_dir);
    info!("done");
    Ok(0)
}

/// Run `git init` (and optionally an initial commit) in the output directory.
fn git_init(dir: &std::path::Path, initial_commit: bool) -> eyre::Result<()> {
    use std::process::Command;
    let pb = summary::spinner("Initializing git repository…");
    let r = Command::new("git").arg("init").current_dir(dir).output();
    match r {
        Ok(o) if o.status.success() => {
            pb.finish_with_message("Git initialized.");
            if initial_commit {
                let pb2 = summary::spinner("Creating initial commit…");
                Command::new("git")
                    .args(["add", "."])
                    .current_dir(dir)
                    .output()?;
                let commit = Command::new("git")
                    .args(["commit", "-m", "Initial commit (scaffolded by springup)"])
                    .current_dir(dir)
                    .output()?;
                if commit.status.success() {
                    pb2.finish_with_message("Initial commit created.");
                } else {
                    pb2.abandon_with_message(
                        "Could not create commit (git user.name / user.email not set?).",
                    );
                }
            }
        }
        Ok(o) => {
            pb.abandon_with_message("git init failed — skipping git setup.");
            debug!("git init stderr: {}", String::from_utf8_lossy(&o.stderr));
        }
        Err(_) => {
            pb.abandon_with_message("git binary not found — skipping git setup.");
        }
    }
    Ok(())
}

/// Run --dry-run: fetch metadata, resolve the plan fully, print as JSON, exit.
fn dry_run(builder: PlanBuilder, args: &NewArgs) -> color_eyre::Result<i32> {
    let initializr_cfg = InitializrConfig {
        base_url: args
            .base_url
            .clone()
            .unwrap_or_else(|| springup_core::initializr::DEFAULT_BASE_URL.into()),
        offline: args.offline,
        refresh: args.refresh,
        timeout: None,
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let plan = rt.block_on(async move {
        let cache = MetadataCache::new()?;
        let client = InitializrClient::new(initializr_cfg, cache)?;
        let metadata = client.fetch_metadata().await?;
        builder.resolve(&metadata, &client)
    })?;
    let json = serde_json::to_string_pretty(&plan)?;
    println!("{json}");
    Ok(0)
}

/// Partially-resolved plan: built from flags + config defaults, missing fields are `Option`.
#[derive(Debug, Clone, Default)]
pub struct PlanBuilder {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub spring_boot_version: Option<String>,
    pub build_tool: Option<BuildTool>,
    pub language: Option<Language>,
    pub java_version: Option<String>,
    pub packaging: Option<Packaging>,
    pub dependencies: Vec<String>,
    pub architecture: Option<ArchitectureKind>,
    pub extras: Vec<ExtraFeature>,
    pub output_dir: Option<PathBuf>,
    pub git_init: Option<bool>,
    pub initial_commit: Option<bool>,
}

impl PlanBuilder {
    /// Resolve into a fully-specified [`ProjectPlan`], filling gaps from `metadata` defaults.
    pub fn resolve(
        self,
        metadata: &springup_core::initializr::InitializrMetadata,
        _client: &InitializrClient,
    ) -> eyre::Result<ProjectPlan> {
        let group_id = self
            .group_id
            .unwrap_or_else(|| metadata.group_id.default.clone());
        let artifact_id = self
            .artifact_id
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| metadata.artifact_id.default.clone());
        let name = self.name.clone().unwrap_or_else(|| artifact_id.clone());
        let description = self
            .description
            .unwrap_or_else(|| metadata.description.default.clone());
        let package_name = self
            .package_name
            .unwrap_or_else(|| format!("{}.{}", group_id, artifact_id.replace('-', ".")));
        let spring_boot_version = self
            .spring_boot_version
            .or_else(|| metadata.latest_stable_boot_version().map(str::to_string))
            .ok_or_else(|| eyre::eyre!("could not determine a Spring Boot version"))?;
        let build_tool = self.build_tool.unwrap_or(BuildTool::Maven);
        let language = self.language.unwrap_or(Language::Java);
        let java_version = self
            .java_version
            .or_else(|| metadata.java_version.values.first().map(|v| v.id.clone()))
            .unwrap_or_else(|| "21".into());
        let packaging = self.packaging.unwrap_or(Packaging::Jar);
        let output_dir = self
            .output_dir
            .unwrap_or_else(|| PathBuf::from(&artifact_id));
        let git_init = self.git_init.unwrap_or(true);
        let initial_commit = self.initial_commit.unwrap_or(false);

        let plan = ProjectPlan {
            group_id,
            artifact_id,
            name,
            description,
            package_name,
            spring_boot_version,
            build_tool,
            language,
            java_version,
            packaging,
            dependencies: self.dependencies,
            architecture: self.architecture,
            extras: self.extras,
            output_dir,
            git_init,
            initial_commit,
        };
        plan.validate()?;
        Ok(plan)
    }
}

/// Build a [`PlanBuilder`] from CLI flags + global config defaults.
fn plan_from_flags(args: &NewArgs, cfg: &Config) -> color_eyre::Result<PlanBuilder> {
    let architecture = args.architecture.as_deref().and_then(|s| {
        if s.eq_ignore_ascii_case("none") {
            None
        } else {
            ArchitectureKind::from_slug(s)
        }
    });

    let extras = match &args.extras {
        Some(items) => crate::cli::parse_extras_list(items)?,
        None => Vec::new(),
    };

    let git_init = if args.git {
        Some(true)
    } else if args.no_git {
        Some(false)
    } else {
        None
    };
    let initial_commit = if args.commit {
        Some(true)
    } else if args.no_commit {
        Some(false)
    } else {
        None
    };

    Ok(PlanBuilder {
        // group id: flag > config > Initializr default (resolved later)
        group_id: args.group_id.clone().or_else(|| Some(cfg.group_id.clone())),
        // artifact id: from --artifact-id, else from positional NAME, else None (Initializr default)
        artifact_id: args.artifact_id.clone().or_else(|| args.name.clone()),
        // name: from positional NAME or --name
        name: args.name.clone().or_else(|| args.artifact_id.clone()),
        description: args.description.clone(),
        // package: not exposed via flags in v1 — derived from group+artifact in resolve()
        package_name: None,
        spring_boot_version: args
            .boot_version
            .clone()
            .or_else(|| cfg.spring_boot_version.clone()),
        build_tool: args
            .build_tool
            .or_else(|| BuildTool::from_slug(&cfg.build_tool)),
        language: args.language,
        java_version: args
            .java_version
            .clone()
            .or_else(|| Some(cfg.java_version.clone())),
        packaging: args.packaging,
        dependencies: args.deps.clone().unwrap_or_default(),
        architecture,
        extras,
        output_dir: args.output.clone(),
        git_init,
        initial_commit,
    })
}
