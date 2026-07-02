//! Strongly-typed error model for `springup-core`.
//!
//! Each failure mode that a user can encounter is a distinct enum variant, so the CLI layer can
//! render precise, actionable messages ("unknown dependency 'web', did you mean 'web'?")
//! instead of dumping raw strings. Library callers can also match on variants programmatically.

use std::path::PathBuf;

/// The error type returned by all fallible `springup-core` operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Initializr service is unreachable, returned a non-2xx status, or timed out.
    #[error("Spring Initializr is unreachable: {message}")]
    InitializrUnreachable {
        /// Human-readable description of the underlying transport / HTTP failure.
        message: String,
        /// True if the failure is plausibly transient and a retry might help.
        retryable: bool,
    },

    /// The Initializr returned a body we could not parse as the expected metadata document.
    #[error("Spring Initializr returned malformed metadata: {message}")]
    InvalidMetadata {
        /// Description of what failed during parsing.
        message: String,
    },

    /// A dependency id passed by the user does not exist in Initializr metadata.
    #[error("Unknown dependency '{id}'{}", suggestion.as_ref().map(|s| format!(", did you mean '{s}'?")).unwrap_or_default())]
    InvalidDependency {
        /// The dependency id the user supplied.
        id: String,
        /// Best-effort fuzzy-match suggestion, if one was found.
        suggestion: Option<String>,
    },

    /// A combination of plan values is internally inconsistent (e.g. Java 8 + Spring Boot 4).
    #[error("Invalid project plan: {message}")]
    InvalidProjectPlan {
        /// What is wrong and (where possible) how to fix it.
        message: String,
    },

    /// A template failed to render. Wrapped message contains the template name and minijinja error.
    #[error("Template render error in '{template}': {message}")]
    TemplateRenderError {
        /// Name of the template that failed.
        template: String,
        /// The underlying render error message.
        message: String,
    },

    /// A required embedded template asset is missing from the binary.
    #[error("Missing template asset: {0}")]
    MissingTemplateAsset(String),

    /// A path inside a downloaded zip attempted to escape the target directory (zip-slip).
    #[error("Refusing to extract entry '{entry}' outside target directory (zip-slip)")]
    ZipSlip {
        /// The offending entry path.
        entry: String,
    },

    /// The downloaded starter zip was corrupt or empty.
    #[error("Invalid or corrupt project zip: {message}")]
    InvalidZip {
        /// Why the zip was rejected.
        message: String,
    },

    /// An IO error from the filesystem (cache read, project write, etc.).
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// A TOML parse/serialize error (manifest or config file).
    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// A TOML serialization error.
    #[error("TOML serialization error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),

    /// A JSON parse error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// A URL parse error.
    #[error("URL error: {0}")]
    UrlError(#[from] url::ParseError),

    /// The HTTP client itself failed to be constructed.
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// A path that should exist was not found.
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    /// A catch-all for errors that don't fit a more specific variant.
    /// Used sparingly — prefer adding a new variant when a recurring failure mode appears.
    #[error("{0}")]
    Other(String),
}

/// Convenience `Result` alias used everywhere in this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Returns `true` when retrying the same operation might succeed.
    ///
    /// Used by the Initializr client to decide whether to back off and retry.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::InitializrUnreachable { retryable, .. } => *retryable,
            _ => false,
        }
    }

    /// Construct an [`Error::InvalidDependency`] with a suggestion computed via fuzzy matching.
    pub fn invalid_dependency_with_suggestion(
        id: impl Into<String>,
        candidates: &[String],
    ) -> Self {
        let id = id.into();
        let suggestion = best_suggestion(&id, candidates);
        Error::InvalidDependency { id, suggestion }
    }
}

/// Pick the best fuzzy-match suggestion for `id` from `candidates` using Jaro-Winkler similarity.
///
/// Returns `None` when no candidate reaches the 0.7 similarity threshold — better to suggest
/// nothing than to suggest a wildly wrong id. The 0.7 threshold catches single-character typos
/// (which typically score 0.8+) while avoiding most false positives.
pub fn best_suggestion(id: &str, candidates: &[String]) -> Option<String> {
    use strsim::jaro_winkler;
    candidates
        .iter()
        .map(|c| (c.clone(), jaro_winkler(id, c)))
        .filter(|(_, score)| *score >= 0.7)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_finds_close_match() {
        let candidates = vec![
            "web".to_string(),
            "data-jpa".to_string(),
            "security".to_string(),
        ];
        let s = best_suggestion("wev", &candidates);
        assert_eq!(s.as_deref(), Some("web"));
    }

    #[test]
    fn suggestion_returns_none_when_nothing_close() {
        let candidates = vec!["web".to_string(), "data-jpa".to_string()];
        let s = best_suggestion("zzzzzzz", &candidates);
        assert!(s.is_none());
    }

    #[test]
    fn invalid_dependency_includes_suggestion() {
        // "wev" is a single-character typo of "web" — a realistic misspelling.
        let candidates = vec!["web".to_string(), "data-jpa".to_string()];
        let err = Error::invalid_dependency_with_suggestion("wev", &candidates);
        let msg = format!("{}", err);
        assert!(msg.contains("wev"));
        assert!(msg.contains("web"));
    }
}
