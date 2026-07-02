//! Color theme — single source of truth for ANSI styling.
//!
//! Respects `NO_COLOR`, `--color never`, and the `console` crate's TTY detection.

use console::Style;
use once_cell::sync::Lazy;

/// Global color preference, initialized from `--color` / `NO_COLOR` / TTY detection.
pub static COLOR_ENABLED: Lazy<bool> = Lazy::new(|| {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Some(c) = std::env::var_os("CLICOLOR_FORCE") {
        if c != "0" {
            return true;
        }
    }
    console::colors_enabled()
});

/// Helper to apply style only when colors are enabled.
fn s(fg: Option<u8>, bold: bool) -> Style {
    if *COLOR_ENABLED {
        let mut style = Style::new();
        if let Some(c) = fg {
            style = style.fg(console::Color::Color256(c));
        }
        if bold {
            style = style.bold();
        }
        style
    } else {
        Style::new().force_styling(false)
    }
}

/// Accent color (cyan).
pub fn accent() -> Style {
    s(Some(39), false)
}

/// Bold heading.
pub fn heading() -> Style {
    s(Some(39), true)
}

/// Success / OK (green).
pub fn success() -> Style {
    s(Some(35), false)
}

/// Warning (yellow).
pub fn warning() -> Style {
    s(Some(179), false)
}

/// Error (red).
pub fn error() -> Style {
    s(Some(124), true)
}

/// Dim / muted (gray).
pub fn dim() -> Style {
    s(Some(242), false)
}

/// Section title (purple).
pub fn section() -> Style {
    s(Some(141), true)
}
