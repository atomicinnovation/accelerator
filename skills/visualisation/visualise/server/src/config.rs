//! Typed view over the config.json produced by launch-server.sh.
//!
//! Schema decided in the Phase 2 plan (research Gap 2). Any
//! change here is a breaking change against the preprocessor —
//! keep fields in sync with scripts/launch-server.sh's JSON
//! writer.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub plugin_root: PathBuf,
    pub plugin_version: String,
    pub project_root: PathBuf,
    pub tmp_path: PathBuf,
    pub host: String,
    pub owner_pid: i32,
    /// Start-time of the owner process, seconds-since-epoch.
    /// `None` disables the owner-PID identity cross-check in the
    /// lifecycle watch (the bare PID probe still runs).
    #[serde(default)]
    pub owner_start_time: Option<u64>,
    pub log_path: PathBuf,
    pub doc_paths: HashMap<String, PathBuf>,
    pub templates: HashMap<String, TemplateTiers>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateTiers {
    pub config_override: Option<PathBuf>,
    pub user_override: PathBuf,
    pub plugin_default: PathBuf,
}

impl Config {
    pub fn from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let bytes = std::fs::read(path)
            .map_err(|source| ConfigError::Read { path: path.to_path_buf(), source })?;
        serde_json::from_slice(&bytes)
            .map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("failed to parse config {path}: {source}")]
    Parse { path: PathBuf, source: serde_json::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        p.push(name);
        p
    }

    #[test]
    fn parses_valid_config() {
        let c = Config::from_path(&fixture("config.valid.json")).expect("valid");
        assert_eq!(c.plugin_version, "1.19.0-pre.2");
        assert_eq!(c.host, "127.0.0.1");
        assert_eq!(c.owner_pid, 0);
        assert_eq!(c.doc_paths.len(), 9);
        assert!(c.doc_paths.contains_key("decisions"));
        assert!(c.doc_paths.contains_key("review_plans"));
        assert!(c.doc_paths.contains_key("review_prs"));
        assert_eq!(c.templates.len(), 5);
        let adr = c.templates.get("adr").expect("adr tier");
        assert!(adr.config_override.is_none());
        assert!(adr.user_override.ends_with("adr.md"));
        assert!(adr.plugin_default.ends_with("adr.md"));
    }

    #[test]
    fn rejects_missing_required_field() {
        let err = Config::from_path(&fixture("config.missing-required.json"))
            .expect_err("missing plugin_root must fail");
        assert!(
            matches!(err, ConfigError::Parse { .. }),
            "expected Parse error, got {err:?}"
        );
    }

    #[test]
    fn rejects_nonexistent_path() {
        let err = Config::from_path(std::path::Path::new("/nonexistent/config.json"))
            .expect_err("missing file must fail");
        assert!(matches!(err, ConfigError::Read { .. }));
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        // deny_unknown_fields guards against preprocessor/server
        // schema drift — a typo like `doc_path` (singular) would
        // otherwise silently produce an empty map.
        let json = r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0",
            "project_root": "/r",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {},
            "doc_path": {"decisions": "/typo"}
        }"#;
        let err = serde_json::from_str::<Config>(json)
            .expect_err("unknown field must fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn config_override_can_be_populated() {
        let json = r#"{
            "plugin_root": "/p",
            "plugin_version": "0.0.0",
            "project_root": "/r",
            "tmp_path": "/t",
            "host": "127.0.0.1",
            "owner_pid": 1,
            "log_path": "/l",
            "doc_paths": {},
            "templates": {
                "adr": {
                    "config_override": "/custom/adr.md",
                    "user_override": "/u/adr.md",
                    "plugin_default": "/d/adr.md"
                }
            }
        }"#;
        let c: Config = serde_json::from_str(json).expect("parse");
        let adr = c.templates.get("adr").unwrap();
        assert_eq!(adr.config_override.as_deref().unwrap(), std::path::Path::new("/custom/adr.md"));
    }
}
