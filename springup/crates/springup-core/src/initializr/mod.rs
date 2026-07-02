//! Spring Initializr REST client and metadata cache.
//!
//! Responsibilities:
//! - Fetch the Initializr metadata document (`GET /` with `Accept: application/json`), which lists
//!   available Spring Boot versions, dependency ids grouped by category, Java versions, packaging
//!   and language options.
//! - Cache that document on disk with a TTL (default 24h) so repeat runs are instant and the
//!   wizard works offline using stale-but-usable cached data (with a warning).
//! - Download a generated starter zip (`GET /starter.zip?...`) and stream it to a temp file.
//! - Validate user-supplied dependency ids against the live metadata, returning a fuzzy-match
//!   suggestion when an id is misspelled.
//!
//! Network calls use a configured `reqwest` client with connect/read timeouts and a bounded
//! retry with exponential backoff for transient failures.

pub mod cache;
pub mod client;
pub mod models;

pub use cache::MetadataCache;
pub use client::{extract_zip, InitializrClient, InitializrConfig, DEFAULT_BASE_URL};
pub use models::{
    Dependency, DependencyGroup, InitializrMetadata, Link, MetadataType, ProjectType, VersionRange,
};
