//! `springup config` — get / set / list global user config.

use clap::CommandFactory;
use color_eyre::eyre;

use springup_core::config::Config;

use crate::cli::ConfigCommand;
use crate::ui::theme;

pub fn run(cmd: ConfigCommand) -> color_eyre::Result<i32> {
    match cmd {
        ConfigCommand::Get { key } => get(&key),
        ConfigCommand::Set { key, value } => set(&key, &value),
        ConfigCommand::List => list(),
    }
}

fn get(key: &str) -> color_eyre::Result<i32> {
    let cfg = Config::load().map_err(|e| eyre::eyre!("could not load config: {e}"))?;
    match cfg.get_field(key) {
        Ok(v) => {
            println!("{v}");
            Ok(0)
        }
        Err(e) => {
            eprintln!("{}: {e}", theme::error().apply_to("error"));
            Ok(1)
        }
    }
}

fn set(key: &str, value: &str) -> color_eyre::Result<i32> {
    let mut cfg = Config::load().map_err(|e| eyre::eyre!("could not load config: {e}"))?;
    if let Err(e) = cfg.set_field(key, value) {
        eprintln!("{}: {e}", theme::error().apply_to("error"));
        return Ok(1);
    }
    if let Err(e) = cfg.save() {
        eprintln!(
            "{}: could not save config: {e}",
            theme::error().apply_to("error")
        );
        return Ok(1);
    }
    println!("{} {} = {}", theme::success().apply_to("✓"), key, value);
    Ok(0)
}

fn list() -> color_eyre::Result<i32> {
    let cfg = Config::load().map_err(|e| eyre::eyre!("could not load config: {e}"))?;
    let path = Config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "<unknown>".into());
    println!("{} {}", theme::dim().apply_to("# config file:"), path);
    for (k, v) in cfg.list_fields() {
        println!("{k} = {v}");
    }
    Ok(0)
}

/// Hint used by the completions command: returns the full clap command for completions.
#[allow(dead_code)]
pub fn clap_command() -> clap::Command {
    crate::cli::Cli::command()
}
