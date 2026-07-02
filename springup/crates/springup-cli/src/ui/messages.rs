//! Centralized user-facing strings.
//!
//! Kept in one place so the tool can later add i18n without scattering `println!` calls across
//! the codebase. Each constant is a short, single-line message; multi-line output is composed
//! by the caller.

/// Used at the top of `new` command output.
pub const TAGLINE: &str = "springup — scaffold a Spring Boot backend in seconds";

/// Printed before the wizard starts.
pub const WIZARD_INTRO: &str =
    "Let's scaffold your Spring Boot project. Answer a few prompts and you'll be up and running.";

/// Printed when a network fetch is in flight.
pub const FETCHING_METADATA: &str = "Fetching Spring Initializr metadata…";

/// Printed when downloading the starter zip.
pub const DOWNLOADING_STARTER: &str = "Downloading starter project from start.spring.io…";

/// Printed when applying the custom template layer.
pub const APPLYING_EXTRAS: &str = "Applying custom template layer…";

/// Printed on successful generation.
pub const DONE: &str = "Project generated successfully.";

/// Final hint to the user.
pub const NEXT_STEPS_HINT: &str = "Next steps:";

/// Used when offline mode returned stale metadata.
pub const STALE_METADATA_WARN: &str =
    "Warning: using stale cached metadata (offline mode). Run `springup update-metadata` when online.";

/// Used when `add` is invoked (it's a stub in v1).
pub const ADD_NOT_IMPLEMENTED: &str =
    "`springup add` is not yet implemented in v1. Track the issue on GitHub.";

/// Used when `update` is invoked (it's a stub in v1).
pub const UPDATE_NOT_IMPLEMENTED: &str =
    "`springup update` is not yet implemented in v1. Reinstall via `cargo install springup` or your package manager.";
