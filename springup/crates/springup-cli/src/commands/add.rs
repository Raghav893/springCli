//! `springup add` — stub for v1.
//!
//! Designed in v1, built in v2. The on-disk `springup.toml` manifest is the extension point.

use crate::cli::AddArgs;
use crate::ui::{messages, theme};

pub fn run(args: AddArgs) -> color_eyre::Result<i32> {
    eprintln!(
        "{}",
        theme::warning().apply_to(messages::ADD_NOT_IMPLEMENTED)
    );
    eprintln!("  requested module: {}", args.module);
    eprintln!();
    eprintln!(
        "The project manifest (springup.toml) is already in place to support this in a future release."
    );
    Ok(1)
}
