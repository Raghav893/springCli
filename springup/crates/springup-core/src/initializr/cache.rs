//! On-disk cache for Initializr metadata.
//!
//! The cache lives at `<cache_dir>/springup/initializr-metadata.json` (platform-correct via the
//! `directories` crate). It is a thin wrapper around a single JSON file containing the serialized
//! [`InitializrMetadata`] plus a `cached_at` timestamp and the base URL it was fetched from (so
//! the cache is never accidentally used against a different Initializr host).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{Error, Result};
use crate::initializr::InitializrMetadata;

/// Default cache TTL: 24 hours. After this, the cached metadata is considered stale.
pub const DEFAULT_TTL: Duration = Duration::hours(24);

/// On-disk metadata cache.
pub struct MetadataCache {
    cache_path: PathBuf,
    ttl: Duration,
}

impl std::fmt::Debug for MetadataCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetadataCache")
            .field("cache_path", &self.cache_path)
            .field("ttl", &self.ttl)
            .finish()
    }
}

/// The serialized cache envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEnvelope {
    /// The base URL the metadata was fetched from.
    base_url: String,
    /// When the metadata was fetched.
    cached_at: DateTime<Utc>,
    /// The metadata document itself.
    metadata: InitializrMetadata,
}

impl MetadataCache {
    /// Construct a cache rooted at the standard platform cache dir.
    pub fn new() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("dev", "springup", "springup")
            .ok_or_else(|| Error::Other("could not determine platform cache directory".into()))?;
        let cache_dir = dirs.cache_dir();
        std::fs::create_dir_all(cache_dir)?;
        let cache_path = cache_dir.join("initializr-metadata.json");
        Ok(Self {
            cache_path,
            ttl: DEFAULT_TTL,
        })
    }

    /// Construct a cache at an explicit path (used in tests).
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self {
            cache_path: path.into(),
            ttl: DEFAULT_TTL,
        }
    }

    /// Override the TTL (mainly for tests).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Returns the cache file path (for inspection / cleanup).
    pub fn path(&self) -> &Path {
        &self.cache_path
    }

    /// Try to read fresh cached metadata for the given base URL.
    ///
    /// Returns `Ok(Some(_))` if a non-stale entry exists for `base_url`, `Ok(None)` otherwise.
    pub fn read_fresh(&self, base_url: &str) -> Result<Option<InitializrMetadata>> {
        let Some(envelope) = self.read_envelope()? else {
            return Ok(None);
        };
        if envelope.base_url != base_url {
            debug!(
                "cache hit but base URL mismatch ({} vs {}), ignoring",
                envelope.base_url, base_url
            );
            return Ok(None);
        }
        let age = Utc::now().signed_duration_since(envelope.cached_at);
        if age > self.ttl {
            debug!("cache stale (age = {})", age);
            return Ok(None);
        }
        Ok(Some(envelope.metadata))
    }

    /// Try to read stale cached metadata, regardless of TTL. Used for offline fallback.
    pub fn read_stale(&self, base_url: &str) -> Result<Option<(InitializrMetadata, Duration)>> {
        let Some(envelope) = self.read_envelope()? else {
            return Ok(None);
        };
        if envelope.base_url != base_url {
            return Ok(None);
        }
        let age = Utc::now().signed_duration_since(envelope.cached_at);
        Ok(Some((envelope.metadata, age)))
    }

    fn read_envelope(&self) -> Result<Option<CacheEnvelope>> {
        if !self.cache_path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&self.cache_path)?;
        let envelope: CacheEnvelope = serde_json::from_str(&data)?;
        Ok(Some(envelope))
    }

    /// Write metadata to the cache, atomically (write to temp + rename).
    pub fn write(&self, base_url: &str, metadata: &InitializrMetadata) -> Result<()> {
        let envelope = CacheEnvelope {
            base_url: base_url.to_string(),
            cached_at: Utc::now(),
            metadata: metadata.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&envelope)?;
        let parent = self
            .cache_path
            .parent()
            .ok_or_else(|| Error::Other("cache path has no parent".into()))?;
        std::fs::create_dir_all(parent)?;
        let tmp = self.cache_path.with_extension("json.tmp");
        std::fs::write(&tmp, &serialized)?;
        std::fs::rename(&tmp, &self.cache_path)?;
        Ok(())
    }

    /// Delete the cache file (used by `springup update-metadata`).
    pub fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            std::fs::remove_file(&self.cache_path)?;
        }
        Ok(())
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            cache_path: PathBuf::from("initializr-metadata.json"),
            ttl: DEFAULT_TTL,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> InitializrMetadata {
        serde_json::from_str(
            r#"{
            "bootVersion": {"default": "3.5.0", "values": [{"id": "3.5.0", "name": "3.5.0"}]}
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn round_trip_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let cache = MetadataCache::at_path(&path);
        assert!(cache.read_fresh("https://x").unwrap().is_none());
        cache.write("https://x", &sample_metadata()).unwrap();
        let m = cache.read_fresh("https://x").unwrap().unwrap();
        assert_eq!(m.boot_version.default, "3.5.0");
    }

    #[test]
    fn different_base_url_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let cache = MetadataCache::at_path(&path);
        cache.write("https://a", &sample_metadata()).unwrap();
        assert!(cache.read_fresh("https://b").unwrap().is_none());
    }

    #[test]
    fn stale_age_returned() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache.json");
        let cache = MetadataCache::at_path(&path).with_ttl(Duration::zero());
        cache.write("https://a", &sample_metadata()).unwrap();
        // fresh should be None because TTL is 0
        assert!(cache.read_fresh("https://a").unwrap().is_none());
        // stale should still return data
        let (m, _age) = cache.read_stale("https://a").unwrap().unwrap();
        assert_eq!(m.boot_version.default, "3.5.0");
    }
}
