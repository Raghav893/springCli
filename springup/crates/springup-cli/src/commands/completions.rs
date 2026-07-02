//! `springup completions` — generate shell completion scripts.

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli::{Cli, CompletionsArgs};

pub fn run(args: CompletionsArgs) -> color_eyre::Result<i32> {
    let mut cmd = Cli::command();
    let shell = match args.shell.as_str() {
        "bash" => Shell::Bash,
        "elvish" => Shell::Elvish,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "zsh" => Shell::Zsh,
        other => {
            eprintln!("unsupported shell: {other}");
            return Ok(1);
        }
    };
    generate(shell, &mut cmd, "springup", &mut std::io::stdout());
    Ok(0)
}
