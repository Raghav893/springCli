//! Embedded template assets for `springup`'s custom layer.
//!
//! Every file under `assets/` is embedded into the binary at compile time via [`rust-embed`], so
//! the shipped binary is fully self-contained — no runtime fetch needed for the custom layer
//! (only the Initializr base project requires network).
//!
//! Templates use [`minijinja`] syntax and are registered into a `minijinja::Environment` by
//! [`crate::template::TemplateRenderer`] using their asset path as the lookup key.
//!
//! ## Asset layout
//!
//! ```text
//! assets/
//! ├── architectures/
//! │   ├── layered/      # GlobalExceptionHandler, ApiResponse, sample CRUD slice
//! │   └── hexagonal/    # domain / application / adapter skeleton
//! ├── docker/           # Dockerfile + docker-compose.yml + .dockerignore
//! ├── ci/github-actions/# ci.yml for Maven and Gradle
//! ├── config-profiles/  # application-dev.yml, application-prod.yml
//! ├── editorconfig/     # .editorconfig
//! ├── git/              # .gitignore
//! └── readme/           # README.md
//! ```

#![forbid(unsafe_code)]

use rust_embed::RustEmbed;

/// Embedded asset bundle. Use [`Asset::iter`] to enumerate asset paths and [`Asset::get`] to
/// fetch a specific asset's bytes.
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Asset;
