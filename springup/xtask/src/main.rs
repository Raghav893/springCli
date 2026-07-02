//! Workspace automation tasks for `springup`.
//!
//! Run via `cargo xtask <task>`. Available tasks:
//! - `cargo xtask check-templates` — verify every embedded template asset parses as valid
//!   minijinja source and that every asset referenced from the core crate exists.
//! - `cargo xtask check-assets` — verify embedded asset integrity (non-empty, valid UTF-8 for
//!   `.j2` files).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Workspace automation for springup")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Verify embedded template assets are valid.
    CheckTemplates,
    /// Verify every asset is non-empty and (where applicable) UTF-8.
    CheckAssets,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::CheckTemplates => check_templates(),
        Cmd::CheckAssets => check_assets(),
    }
}

fn templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("crates/springup-templates/assets")
}

fn check_templates() -> anyhow::Result<()> {
    let dir = templates_dir();
    let mut count = 0usize;
    let mut errors = 0usize;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(&dir).unwrap();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "j2" {
            continue;
        }
        count += 1;
        let src = std::fs::read_to_string(path)?;
        let mut env = minijinja::Environment::new();
        if let Err(e) = env.add_template_owned(rel.to_string_lossy().to_string(), src) {
            eprintln!("FAIL {} — {e}", rel.display());
            errors += 1;
        }
    }
    println!("Checked {count} templates, {errors} errors");
    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn check_assets() -> anyhow::Result<()> {
    let dir = templates_dir();
    let mut count = 0usize;
    let mut errors = 0usize;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        count += 1;
        let path = entry.path();
        let meta = std::fs::metadata(path)?;
        if meta.len() == 0 {
            eprintln!("FAIL {} — empty asset", path.display());
            errors += 1;
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(ext, "j2" | "yml" | "yaml" | "md") && std::fs::read_to_string(path).is_err() {
            eprintln!("FAIL {} — not valid UTF-8", path.display());
            errors += 1;
        }
    }
    println!("Checked {count} assets, {errors} errors");
    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
