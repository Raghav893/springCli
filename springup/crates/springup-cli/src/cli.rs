//! Command-line interface definition for `springup`.
//!
//! Uses `clap` v4 derive macros. Designed so that the command tree is coherent from day one,
//! even though only `new` (and `config`, `completions`) are fully implemented in v1.

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand};

use springup_core::plan::{BuildTool, ExtraFeature, Language, Packaging};

use crate::commands;

/// Top-level CLI parsed from `argv`.
#[derive(Parser, Debug)]
#[command(
    name = "springup",
    bin_name = "springup",
    version,
    about = "Scaffold a production-ready Spring Boot backend in seconds.",
    long_about = "springup is what `npm create` and `cargo generate` are to their ecosystems, \
                  but for Spring Boot: a fast, single static binary that interactively (or \
                  non-interactively) scaffolds a production-ready Spring Boot backend.",
    propagate_version = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Increase logging verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all logging output (overrides -v).
    #[arg(short, long, global = true, action = ArgAction::SetTrue)]
    pub quiet: bool,

    /// Force enable / disable colored output. `auto` detects TTY.
    #[arg(
        long,
        global = true,
        env = "CLICOLOR",
        default_value = "auto",
        help_heading = "Global options"
    )]
    pub color: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level subcommands. Designed today; `add` is a stub.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new Spring Boot project (interactive if flags omitted).
    New(Box<NewArgs>),
    /// Add a module / extra to an existing project (not yet implemented in v1).
    #[command(name = "add")]
    Add(AddArgs),
    /// Manage global user configuration.
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Generate shell completion scripts.
    Completions(CompletionsArgs),
    /// Refresh the cached Spring Initializr metadata.
    UpdateMetadata,
    /// Self-update the binary to the latest release.
    Update,
}

/// Arguments for `springup new`.
#[derive(Args, Debug, Clone)]
pub struct NewArgs {
    /// Project name (and default artifact id). If omitted, the wizard will prompt.
    pub name: Option<String>,

    /// Maven group id.
    #[arg(long, help_heading = "Project metadata")]
    pub group_id: Option<String>,

    /// Maven artifact id.
    #[arg(long, help_heading = "Project metadata")]
    pub artifact_id: Option<String>,

    /// One-line project description.
    #[arg(long, help_heading = "Project metadata")]
    pub description: Option<String>,

    /// Spring Boot version. Defaults to latest stable per Initializr metadata.
    #[arg(long, help_heading = "Project metadata")]
    pub boot_version: Option<String>,

    /// Build tool: maven | gradle | gradle-kotlin.
    #[arg(long, value_parser = build_tool_parser, help_heading = "Build settings")]
    pub build_tool: Option<BuildTool>,

    /// Source language: java | kotlin.
    #[arg(long, value_parser = language_parser, help_heading = "Build settings")]
    pub language: Option<Language>,

    /// Java version (e.g. 17, 21).
    #[arg(long, help_heading = "Build settings")]
    pub java_version: Option<String>,

    /// Packaging: jar | war.
    #[arg(long, value_parser = packaging_parser, help_heading = "Build settings")]
    pub packaging: Option<Packaging>,

    /// Comma-separated Initializr dependency ids (e.g. web,data-jpa,postgresql).
    #[arg(
        short = 'd',
        long,
        value_delimiter = ',',
        help_heading = "Dependencies"
    )]
    pub deps: Option<Vec<String>>,

    /// Architecture skeleton: none | layered | hexagonal.
    #[arg(long, help_heading = "Custom layer")]
    pub architecture: Option<String>,

    /// Comma-separated extras: docker,docker-compose,ci,config-profiles,editorconfig,readme.
    #[arg(long, value_delimiter = ',', help_heading = "Custom layer")]
    pub extras: Option<Vec<String>>,

    /// Output directory. Defaults to `./<artifact-id>`.
    #[arg(short = 'o', long, help_heading = "Output")]
    pub output: Option<std::path::PathBuf>,

    /// Run `git init` in the output directory (default).
    #[arg(long, conflicts_with = "no_git", action = ArgAction::SetTrue, help_heading = "Output")]
    pub git: bool,

    /// Skip `git init`.
    #[arg(long, conflicts_with = "git", action = ArgAction::SetTrue, help_heading = "Output")]
    pub no_git: bool,

    /// Make an initial commit after `git init`.
    #[arg(long, conflicts_with = "no_commit", action = ArgAction::SetTrue, help_heading = "Output")]
    pub commit: bool,

    /// Skip the initial commit (default).
    #[arg(long, conflicts_with = "commit", action = ArgAction::SetTrue, help_heading = "Output")]
    pub no_commit: bool,

    /// Accept all defaults, skip the wizard entirely (CI / scripting mode).
    #[arg(short = 'y', long, action = ArgAction::SetTrue, help_heading = "Output")]
    pub yes: bool,

    /// Print the resolved ProjectPlan and exit without generating anything.
    #[arg(long, action = ArgAction::SetTrue, help_heading = "Output")]
    pub dry_run: bool,

    /// Use cached metadata only; never hit the network.
    #[arg(long, action = ArgAction::SetTrue, help_heading = "Network")]
    pub offline: bool,

    /// Force-refresh the cached Initializr metadata.
    #[arg(long, action = ArgAction::SetTrue, help_heading = "Network")]
    pub refresh: bool,

    /// Initializr base URL override (e.g. for a mirror).
    #[arg(long, env = "SPRINGUP_INITIALIZR_BASE_URL", help_heading = "Network")]
    pub base_url: Option<String>,
}

/// Arguments for `springup add` (stub in v1).
#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    /// Name of the module / extra to add.
    pub module: String,
}

/// `springup config` subcommand tree.
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    /// Get a single config value by key.
    Get { key: String },
    /// Set a single config value.
    Set { key: String, value: String },
    /// List all config values.
    List,
}

/// `springup completions` arguments.
#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    /// Target shell.
    #[arg(value_parser = ["bash", "elvish", "fish", "powershell", "zsh"])]
    pub shell: String,
}

impl Cli {
    /// Parse `argv` and dispatch to the appropriate command handler.
    ///
    /// Returns the process exit code (0 = success, non-zero = failure).
    pub fn parse_and_dispatch(args: &[String]) -> color_eyre::Result<i32> {
        let cli = Self::parse_from(args.iter());
        crate::ui::logging::init(cli.verbose, cli.quiet, &cli.color);

        let code = match cli.command {
            None => {
                // No subcommand: print help.
                Self::command().print_help()?;
                0
            }
            Some(Command::New(args)) => commands::new::run(*args)?,
            Some(Command::Add(args)) => commands::add::run(args)?,
            Some(Command::Config(cmd)) => commands::config::run(cmd)?,
            Some(Command::Completions(args)) => commands::completions::run(args)?,
            Some(Command::UpdateMetadata) => commands::update_metadata::run()?,
            Some(Command::Update) => commands::update::run()?,
        };
        Ok(code)
    }
}

fn build_tool_parser(s: &str) -> Result<BuildTool, String> {
    BuildTool::from_slug(s)
        .ok_or_else(|| format!("unknown build tool '{s}' (expected: maven, gradle, gradle-kotlin)"))
}

fn language_parser(s: &str) -> Result<Language, String> {
    match s.to_ascii_lowercase().as_str() {
        "java" => Ok(Language::Java),
        "kotlin" | "kt" => Ok(Language::Kotlin),
        _ => Err(format!("unknown language '{s}' (expected: java, kotlin)")),
    }
}

fn packaging_parser(s: &str) -> Result<Packaging, String> {
    match s.to_ascii_lowercase().as_str() {
        "jar" => Ok(Packaging::Jar),
        "war" => Ok(Packaging::War),
        _ => Err(format!("unknown packaging '{s}' (expected: jar, war)")),
    }
}

/// Parse a comma-separated list of extras slugs into a `Vec<ExtraFeature>`.
pub fn parse_extras_list(items: &[String]) -> color_eyre::Result<Vec<ExtraFeature>> {
    let joined = items.join(",");
    Ok(ExtraFeature::parse_list(&joined)?)
}
