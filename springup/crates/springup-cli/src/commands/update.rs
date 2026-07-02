//! `springup update` — self-update stub for v1.
//!
//! In v1, self-update is intentionally not bundled (we'd pull in a separate async release
//! fetcher). Documented manual install paths live in the README.

use crate::ui::{messages, theme};

pub fn run() -> color_eyre::Result<i32> {
    eprintln!(
        "{}",
        theme::warning().apply_to(messages::UPDATE_NOT_IMPLEMENTED)
    );
    eprintln!();
    eprintln!("To update, reinstall via your original install path:");
    eprintln!("  cargo install springup --locked");
    eprintln!("  brew upgrade springup    # (if installed via Homebrew)");
    eprintln!("  scoop update springup    # (if installed via Scoop)");
    Ok(1)
}
