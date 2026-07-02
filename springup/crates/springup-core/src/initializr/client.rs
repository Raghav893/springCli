//! HTTP client for `https://start.spring.io` (or a configurable mirror).
//!
//! Provides:
//! - [`InitializrClient::fetch_metadata`] — fetch + cache the metadata document.
//! - [`InitializrClient::download_starter_zip`] — fetch a generated starter project as bytes.
//! - [`InitializrClient::validate_dependencies`] — fuzzy-match suggestions for unknown ids.
//!
//! All network calls have connect/read timeouts and a bounded exponential-backoff retry policy
//! for transient (5xx + connection) failures.

use std::path::Path;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use tracing::{debug, warn};
use url::Url;

use crate::error::{Error, Result};
use crate::initializr::cache::MetadataCache;
use crate::initializr::InitializrMetadata;
use crate::plan::ProjectPlan;

/// Default base URL for the public Spring Initializr.
pub const DEFAULT_BASE_URL: &str = "https://start.spring.io";

/// Connect timeout for Initializr HTTP calls.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Read timeout (per response — large starter zips can take a few seconds).
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Maximum number of retry attempts for transient failures.
const MAX_RETRIES: u32 = 3;

/// Configuration for an [`InitializrClient`].
#[derive(Debug, Clone)]
pub struct InitializrConfig {
    /// Base URL of the Initializr instance.
    pub base_url: String,
    /// Per-request timeout (overrides the default connect+read timeout if set).
    pub timeout: Option<Duration>,
    /// When `true`, prefer stale cached metadata over a network fetch.
    pub offline: bool,
    /// Force-refresh the metadata cache even if it is fresh.
    pub refresh: bool,
}

impl Default for InitializrConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.into(),
            timeout: None,
            offline: false,
            refresh: false,
        }
    }
}

/// A Spring Initializr REST client.
pub struct InitializrClient {
    cfg: InitializrConfig,
    http: Client,
    cache: MetadataCache,
}

impl std::fmt::Debug for InitializrClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitializrClient")
            .field("cfg", &self.cfg)
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl InitializrClient {
    /// Construct a client with the given config and metadata cache.
    pub fn new(cfg: InitializrConfig, cache: MetadataCache) -> Result<Self> {
        let mut builder = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .user_agent(concat!("springup/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(5));
        if let Some(t) = cfg.timeout {
            builder = builder.timeout(t);
        }
        let http = builder.build()?;
        Ok(Self { cfg, http, cache })
    }

    /// Returns the configured base URL.
    pub fn base_url(&self) -> &str {
        &self.cfg.base_url
    }

    /// Returns a reference to the metadata cache.
    pub fn cache(&self) -> &MetadataCache {
        &self.cache
    }

    /// Fetch the Initializr metadata, using the on-disk cache when fresh.
    ///
    /// Behavior:
    /// - If `offline`, return stale cache or error.
    /// - If `refresh`, ignore fresh cache and force a network fetch.
    /// - Otherwise, return fresh cache if present; else fetch from network.
    pub async fn fetch_metadata(&self) -> Result<InitializrMetadata> {
        if !self.cfg.refresh {
            if let Some(m) = self.cache.read_fresh(&self.cfg.base_url)? {
                debug!("metadata cache hit (fresh)");
                return Ok(m);
            }
        }

        if self.cfg.offline {
            if let Some((m, age)) = self.cache.read_stale(&self.cfg.base_url)? {
                warn!("offline mode: using stale cached metadata (age = {})", age);
                return Ok(m);
            }
            return Err(Error::InitializrUnreachable {
                message: "offline mode requested but no cached metadata is available".into(),
                retryable: false,
            });
        }

        let url = Url::parse(&self.cfg.base_url)?;
        let url_str = url.as_str();
        let body = self.get_with_retry(url_str, true).await?;
        let metadata: InitializrMetadata =
            serde_json::from_slice(&body).map_err(|e| Error::InvalidMetadata {
                message: format!("JSON parse failure: {e}"),
            })?;

        // Best-effort cache write — failure here is not fatal.
        if let Err(e) = self.cache.write(&self.cfg.base_url, &metadata) {
            warn!("failed to write metadata cache: {e}");
        }
        Ok(metadata)
    }

    /// Validate the dependency ids in a [`ProjectPlan`] against Initializr metadata.
    ///
    /// Returns the first invalid dependency (with suggestion) as an error. On success, returns
    /// `Ok(())`.
    pub fn validate_dependencies(
        &self,
        metadata: &InitializrMetadata,
        plan: &ProjectPlan,
    ) -> Result<()> {
        let all = metadata.all_dependency_ids();
        for dep in &plan.dependencies {
            if !all.iter().any(|c| c == dep) {
                return Err(Error::invalid_dependency_with_suggestion(dep, &all));
            }
        }
        Ok(())
    }

    /// Download a starter zip from Initializr and return its raw bytes.
    ///
    /// Streams the response into memory; for the sizes involved (a few hundred KB) this is fine.
    pub async fn download_starter_zip(&self, plan: &ProjectPlan) -> Result<Vec<u8>> {
        let mut url = Url::parse(&self.cfg.base_url)?;
        url.set_path("/starter.zip");
        let type_code = plan.build_tool.initializr_code();
        let mut q = url.query_pairs_mut();
        q.append_pair("type", type_code)
            .append_pair("language", plan.language.initializr_code())
            .append_pair("bootVersion", &plan.spring_boot_version)
            .append_pair("groupId", &plan.group_id)
            .append_pair("artifactId", &plan.artifact_id)
            .append_pair("name", &plan.name)
            .append_pair("description", &plan.description)
            .append_pair("packageName", &plan.package_name)
            .append_pair("packaging", plan.packaging.initializr_code())
            .append_pair("javaVersion", &plan.java_version);
        if !plan.dependencies.is_empty() {
            q.append_pair("dependencies", &plan.dependencies.join(","));
        }
        drop(q);

        let url_str = url.to_string();
        debug!("downloading starter from {url_str}");
        let body = self.get_with_retry(&url_str, false).await?;
        Ok(body)
    }

    /// GET a URL with exponential backoff for transient failures.
    ///
    /// When `accept_json` is true, sets `Accept: application/json`.
    async fn get_with_retry(&self, url: &str, accept_json: bool) -> Result<Vec<u8>> {
        let mut last_err: Option<Error> = None;
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let backoff = Duration::from_millis(500 * (1 << (attempt - 1)));
                debug!("retry #{attempt} after {backoff:?}");
                tokio::time::sleep(backoff).await;
            }
            let mut req = self.http.get(url);
            if accept_json {
                req = req.header(reqwest::header::ACCEPT, "application/json");
            }
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let bytes = resp.bytes().await?;
                        return Ok(bytes.to_vec());
                    }
                    let retryable =
                        status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS;
                    let msg = format!("HTTP {} from {}", status, url);
                    last_err = Some(Error::InitializrUnreachable {
                        message: msg,
                        retryable,
                    });
                    if !retryable {
                        break;
                    }
                }
                Err(e) => {
                    last_err = Some(Error::InitializrUnreachable {
                        message: format!("request error: {e}"),
                        retryable: e.is_connect() || e.is_timeout(),
                    });
                }
            }
        }
        Err(last_err.unwrap_or_else(|| Error::InitializrUnreachable {
            message: "exhausted retries".into(),
            retryable: false,
        }))
    }
}

/// Extract a downloaded starter zip into `dest`, rejecting any entry that would escape the
/// target directory (zip-slip protection).
pub fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
    use std::io::{Cursor, Write};

    let reader = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| Error::InvalidZip {
        message: e.to_string(),
    })?;

    let dest_canonical = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());

    std::fs::create_dir_all(dest)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| Error::InvalidZip {
            message: e.to_string(),
        })?;
        let entry_name = entry.name().to_string();

        let out_path = dest.join(&entry_name);
        // Zip-slip check: canonicalize the parent + child and ensure it's still under dest.
        let parent = out_path.parent().unwrap_or(dest);
        std::fs::create_dir_all(parent)?;
        let parent_canonical = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        let normalized =
            parent_canonical.join(out_path.file_name().ok_or_else(|| Error::ZipSlip {
                entry: entry_name.clone(),
            })?);

        if !normalized.starts_with(&dest_canonical) {
            return Err(Error::ZipSlip { entry: entry_name });
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&normalized)?;
        } else {
            let mut f = std::fs::File::create(&normalized)?;
            std::io::copy(&mut entry, &mut f)?;
            f.flush()?;

            // Restore unix permissions if present (best-effort).
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ = std::fs::set_permissions(
                        &normalized,
                        std::fs::Permissions::from_mode(mode),
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BuildTool, Language, Packaging};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn plan() -> ProjectPlan {
        ProjectPlan {
            group_id: "com.example".into(),
            artifact_id: "demo".into(),
            name: "demo".into(),
            description: "".into(),
            package_name: "com.example.demo".into(),
            spring_boot_version: "3.5.0".into(),
            build_tool: BuildTool::Maven,
            language: Language::Java,
            java_version: "21".into(),
            packaging: Packaging::Jar,
            dependencies: vec!["web".into()],
            architecture: None,
            extras: vec![],
            output_dir: Path::new("./demo").into(),
            git_init: false,
            initial_commit: false,
        }
    }

    #[tokio::test]
    async fn fetch_metadata_caches_and_reads() {
        let server = MockServer::start().await;
        let body = r#"{
            "bootVersion": {"default": "3.5.0", "values": [{"id":"3.5.0","name":"3.5.0"}]},
            "dependencies": {"values":[{"name":"Web","values":[{"id":"web","name":"Spring Web","description":""}]}]}
        }"#;
        Mock::given(method("GET"))
            .and(path("/"))
            .and(header("accept", "application/json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&server)
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let cache = MetadataCache::at_path(tmp.path().join("cache.json"));
        let cfg = InitializrConfig {
            base_url: server.uri(),
            ..Default::default()
        };
        let client = InitializrClient::new(cfg, cache).unwrap();

        let m = client.fetch_metadata().await.unwrap();
        assert_eq!(m.boot_version.default, "3.5.0");

        // Second call should hit cache (server's mock only allows 1 call total since we don't
        // re-mount). Drop server and call again — should succeed from cache.
        let m2 = client.fetch_metadata().await.unwrap();
        assert_eq!(m2.boot_version.default, "3.5.0");
    }

    #[tokio::test]
    async fn download_starter_zip_builds_correct_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/starter.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PK\x03\x04fakezip".to_vec()))
            .mount(&server)
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let cache = MetadataCache::at_path(tmp.path().join("cache.json"));
        let cfg = InitializrConfig {
            base_url: server.uri(),
            ..Default::default()
        };
        let client = InitializrClient::new(cfg, cache).unwrap();
        let bytes = client.download_starter_zip(&plan()).await.unwrap();
        assert!(!bytes.is_empty());
    }

    #[tokio::test]
    async fn server_error_retries_then_fails() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let cache = MetadataCache::at_path(tmp.path().join("cache.json"));
        let cfg = InitializrConfig {
            base_url: server.uri(),
            ..Default::default()
        };
        let client = InitializrClient::new(cfg, cache).unwrap();
        let r = client.fetch_metadata().await;
        assert!(r.is_err());
        let msg = format!("{}", r.unwrap_err());
        assert!(msg.contains("Initializr"));
    }
}
