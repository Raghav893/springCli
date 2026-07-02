//! Library entry point for the `springup` binary.
//!
//! The actual binary (`src/main.rs`) is a thin shim that calls [`run`] and converts the result
//! into an [`std::process::ExitCode`]. This split makes the CLI logic testable via `assert_cmd`.

pub mod cli;
pub mod commands;
pub mod ui;
pub mod wizard;

use color_eyre::Result;

/// Entry point invoked by `main.rs`.
pub fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let exit = cli::Cli::parse_and_dispatch(&args)?;
    if exit == 0 {
        Ok(())
    } else {
        std::process::exit(exit);
    }
}
