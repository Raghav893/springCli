//! `tracing` initialization driven by `--verbose` / `--quiet` / `--color` flags.

use tracing_subscriber::EnvFilter;

/// Initialize the global tracing subscriber.
///
/// - `verbose == 0` (default): warn level + only springup's own crates
/// - `verbose == 1` (-v): info level
/// - `verbose == 2` (-vv): debug level
/// - `verbose == 3+` (-vvv): trace level
///
/// `quiet` overrides everything to errors-only.
///
/// Honors `NO_COLOR` and `--color never` by disabling ANSI output.
pub fn init(verbose: u8, quiet: bool, color: &str) {
    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("springup={level},springup_core={level}")));

    let use_ansi = match color.to_ascii_lowercase().as_str() {
        "never" => false,
        "always" => true,
        _ => {
            // auto: respect NO_COLOR and TTY
            std::env::var_os("NO_COLOR").is_none() && atty_stderr()
        }
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_ansi(use_ansi)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(unix)]
fn atty_stderr() -> bool {
    use std::os::fd::AsRawFd;
    let fd = std::io::stderr().as_raw_fd();
    unsafe { libc_isatty(fd) }
}

#[cfg(not(unix))]
fn atty_stderr() -> bool {
    // Best-effort on non-unix: assume TTY if stderr is not redirected.
    // The `console` crate (already in our dep tree via dialoguer) provides cross-platform TTY
    // detection; we use it as a fallback.
    console::Term::stderr().features().is_atty()
}

#[cfg(unix)]
unsafe extern "C" {
    fn isatty(fd: std::os::fd::RawFd) -> i32;
}

#[cfg(unix)]
unsafe fn libc_isatty(fd: std::os::fd::RawFd) -> bool {
    unsafe { isatty(fd) != 0 }
}
