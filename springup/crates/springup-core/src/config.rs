//! Global user configuration (`~/.config/springup/config.toml`).
//!
//! Stores user-level defaults: default group id, default Java version, default build tool, color
//! preference, telemetry opt-in (default OFF — see spec §6.7), and an optional Initializr base
//! URL override.
//!
//! Precedence (most specific wins):
//! 1. CLI flags
//! 2. Project-local `.springuprc.toml` (if present in cwd)
//! 3. Global user config (this file)
//! 4. Hardcoded defaults ([`Config::default`])

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::plan::BuildTool;

/// Global user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default Maven group id (e.g. `com.example`).
    #[serde(default = "default_group_id")]
    pub group_id: String,
    /// Default author/org name (used in README and LICENSE).
    #[serde(default)]
    pub author: Option<String>,
    /// Default Java version (e.g. `21`).
    #[serde(default = "default_java_version")]
    pub java_version: String,
    /// Default build tool slug.
    #[serde(default = "default_build_tool")]
    pub build_tool: String,
    /// Default Spring Boot version, or `None` to always use Initializr's latest stable.
    #[serde(default)]
    pub spring_boot_version: Option<String>,
    /// Optional Initializr base URL override (for mirrors / air-gapped networks).
    #[serde(default)]
    pub initializr_base_url: Option<String>,
    /// Color output preference: `"auto"`, `"always"`, or `"never"`.
    #[serde(default = "default_color")]
    pub color: String,
    /// Telemetry opt-in flag. Always defaults to `false` — see spec §6.7.
    #[serde(default)]
    pub telemetry: bool,
}

fn default_group_id() -> String {
    "com.example".into()
}
fn default_java_version() -> String {
    "21".into()
}
fn default_build_tool() -> String {
    "maven".into()
}
fn default_color() -> String {
    "auto".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            group_id: default_group_id(),
            author: None,
            java_version: default_java_version(),
            build_tool: default_build_tool(),
            spring_boot_version: None,
            initializr_base_url: None,
            color: default_color(),
            telemetry: false,
        }
    }
}

impl Config {
    /// Returns the platform-standard config file path.
    pub fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("dev", "springup", "springup")
            .ok_or_else(|| Error::Other("could not determine platform config directory".into()))?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Load the global config, falling back to defaults if the file does not exist.
    ///
    /// Returns an error only if the file exists but is corrupt (so the user can decide what to do
    /// rather than silently losing their settings).
    pub fn load() -> Result<Self> {
        Self::load_from(Self::config_path()?.as_path())
    }

    /// Load config from a specific path (used in tests and the `config` subcommand).
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&data)?;
        Ok(cfg)
    }

    /// Save the config to the standard location, creating parent dirs as needed.
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::config_path()?)
    }

    /// Save the config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }

    /// Set a single key by name. Returns `Err` for unknown keys.
    ///
    /// Used by `springup config set <key> <value>`.
    pub fn set_field(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "group-id" | "group_id" => self.group_id = value.into(),
            "author" => self.author = Some(value.into()),
            "java-version" | "java_version" => self.java_version = value.into(),
            "build-tool" | "build_tool" => {
                if BuildTool::from_slug(value).is_none() {
                    return Err(Error::InvalidProjectPlan {
                        message: format!(
                            "unknown build tool '{value}' (expected: maven, gradle, gradle-kotlin)"
                        ),
                    });
                }
                self.build_tool = value.into();
            }
            "spring-boot-version" | "spring_boot_version" => {
                self.spring_boot_version = Some(value.into());
            }
            "initializr-base-url" | "initializr_base_url" => {
                self.initializr_base_url = Some(value.into());
            }
            "color" => {
                let v = value.to_ascii_lowercase();
                if !matches!(v.as_str(), "auto" | "always" | "never") {
                    return Err(Error::InvalidProjectPlan {
                        message: format!(
                            "color must be one of: auto, always, never (got '{value}')"
                        ),
                    });
                }
                self.color = v;
            }
            "telemetry" => {
                self.telemetry = match value.to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => {
                        return Err(Error::InvalidProjectPlan {
                            message: format!(
                                "telemetry must be a boolean (true/false), got '{value}'"
                            ),
                        })
                    }
                };
            }
            other => {
                return Err(Error::InvalidProjectPlan {
                    message: format!(
                        "unknown config key '{other}'. Known keys: group-id, author, java-version, build-tool, spring-boot-version, initializr-base-url, color, telemetry"
                    ),
                })
            }
        }
        Ok(())
    }

    /// Get a single field's value as a string (for `springup config get <key>`).
    pub fn get_field(&self, key: &str) -> Result<String> {
        match key {
            "group-id" | "group_id" => Ok(self.group_id.clone()),
            "author" => Ok(self.author.clone().unwrap_or_default()),
            "java-version" | "java_version" => Ok(self.java_version.clone()),
            "build-tool" | "build_tool" => Ok(self.build_tool.clone()),
            "spring-boot-version" | "spring_boot_version" => {
                Ok(self.spring_boot_version.clone().unwrap_or_default())
            }
            "initializr-base-url" | "initializr_base_url" => {
                Ok(self.initializr_base_url.clone().unwrap_or_default())
            }
            "color" => Ok(self.color.clone()),
            "telemetry" => Ok(self.telemetry.to_string()),
            other => Err(Error::InvalidProjectPlan {
                message: format!("unknown config key '{other}'"),
            }),
        }
    }

    /// List all `(key, value)` pairs in stable order (for `springup config list`).
    pub fn list_fields(&self) -> Vec<(&'static str, String)> {
        vec![
            ("group-id", self.group_id.clone()),
            ("author", self.author.clone().unwrap_or_default()),
            ("java-version", self.java_version.clone()),
            ("build-tool", self.build_tool.clone()),
            (
                "spring-boot-version",
                self.spring_boot_version.clone().unwrap_or_default(),
            ),
            (
                "initializr-base-url",
                self.initializr_base_url.clone().unwrap_or_default(),
            ),
            ("color", self.color.clone()),
            ("telemetry", self.telemetry.to_string()),
        ]
    }
}

/// Look for a `.springuprc.toml` in the cwd and return its raw text if present.
///
/// For v1, project-local overrides are intentionally minimal: we parse but only surface the
/// `group_id`, `java_version`, and `build_tool` fields, matching the global config schema. This
/// keeps the precedence story honest without committing to a richer per-project schema yet.
pub fn read_local_override(cwd: &Path) -> Result<Option<Config>> {
    let path = cwd.join(".springuprc.toml");
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let cfg: Config = toml::from_str(&data)?;
    Ok(Some(cfg))
}

/// Merge `base` with `override_cfg`, with `override_cfg` taking precedence only for non-default
/// (explicitly set) fields. Since we don't track "was-this-set" state in v1, the override simply
/// replaces matching fields if they are non-empty / non-None.
pub fn merge(base: Config, override_cfg: Config) -> Config {
    Config {
        group_id: if override_cfg.group_id.is_empty() {
            base.group_id
        } else {
            override_cfg.group_id
        },
        author: override_cfg.author.or(base.author),
        java_version: if override_cfg.java_version.is_empty() {
            base.java_version
        } else {
            override_cfg.java_version
        },
        build_tool: if override_cfg.build_tool.is_empty() {
            base.build_tool
        } else {
            override_cfg.build_tool
        },
        spring_boot_version: override_cfg
            .spring_boot_version
            .or(base.spring_boot_version),
        initializr_base_url: override_cfg
            .initializr_base_url
            .or(base.initializr_base_url),
        color: if override_cfg.color.is_empty() {
            base.color
        } else {
            override_cfg.color
        },
        telemetry: override_cfg.telemetry || base.telemetry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("missing.toml");
        let cfg = Config::load_from(&path).unwrap();
        assert_eq!(cfg.group_id, "com.example");
        assert_eq!(cfg.java_version, "21");
        assert!(!cfg.telemetry);
    }

    #[test]
    fn round_trip_config() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        let mut cfg = Config {
            group_id: "dev.raghavarora".into(),
            author: Some("Raghu".into()),
            ..Config::default()
        };
        cfg.set_field("telemetry", "false").unwrap();
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.group_id, "dev.raghavarora");
        assert_eq!(loaded.author.as_deref(), Some("Raghu"));
    }

    #[test]
    fn set_unknown_key_fails() {
        let mut cfg = Config::default();
        assert!(cfg.set_field("bogus", "x").is_err());
    }

    #[test]
    fn set_invalid_color_fails() {
        let mut cfg = Config::default();
        assert!(cfg.set_field("color", "rainbow").is_err());
    }

    #[test]
    fn set_invalid_build_tool_fails() {
        let mut cfg = Config::default();
        assert!(cfg.set_field("build-tool", "bazel").is_err());
    }

    #[test]
    fn merge_takes_override_when_present() {
        let base = Config {
            group_id: "base.example".into(),
            ..Config::default()
        };
        let ovr = Config {
            group_id: "override.example".into(),
            ..Config::default()
        };
        let m = merge(base, ovr);
        assert_eq!(m.group_id, "override.example");
    }

    #[test]
    fn merge_keeps_base_when_override_empty() {
        let base = Config {
            group_id: "base.example".into(),
            ..Config::default()
        };
        let ovr = Config {
            group_id: String::new(), // empty override should NOT replace base
            ..Config::default()
        };
        let m = merge(base, ovr);
        assert_eq!(m.group_id, "base.example");
    }

    #[test]
    fn list_fields_contains_all_keys() {
        let cfg = Config::default();
        let keys: Vec<_> = cfg.list_fields().into_iter().map(|(k, _)| k).collect();
        assert!(keys.contains(&"group-id"));
        assert!(keys.contains(&"telemetry"));
    }
}
