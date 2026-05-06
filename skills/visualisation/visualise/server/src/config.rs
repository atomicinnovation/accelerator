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
    /// Work-item ID pattern configuration. Absent from pre-Phase-2
    /// configs; treated as the numeric default when missing.
    #[serde(default)]
    pub work_item: Option<RawWorkItemConfig>,
    /// Kanban column keys. Absent → seven template-status defaults.
    /// An empty list is rejected at boot.
    #[serde(default)]
    pub kanban_columns: Option<Vec<String>>,
}

/// A single kanban board column, as resolved at boot.
#[derive(Debug, Clone)]
pub struct KanbanColumn {
    pub key: String,
    pub label: String,
}

/// Deserializable form of the work-item ID configuration. The launcher
/// emits this under the `work_item` key in config.json.
#[derive(Debug, Clone, Deserialize)]
pub struct RawWorkItemConfig {
    pub scan_regex: String,
    #[serde(default = "default_id_pattern")]
    pub id_pattern: String,
    #[serde(default)]
    pub default_project_code: Option<String>,
}

fn default_id_pattern() -> String {
    "{number:04d}".to_string()
}

/// Runtime work-item ID configuration with the scan regex compiled once
/// at boot. All downstream code accepts `&WorkItemConfig` and never
/// re-compiles.
#[derive(Debug)]
pub struct WorkItemConfig {
    pub scan_regex: regex::Regex,
    pub scan_regex_raw: String,
    pub id_pattern: String,
    pub default_project_code: Option<String>,
}

impl WorkItemConfig {
    pub fn from_raw(raw: RawWorkItemConfig) -> Result<Self, ConfigError> {
        let scan_regex = regex::Regex::new(&raw.scan_regex).map_err(|source| {
            ConfigError::InvalidScanRegex {
                pattern: raw.scan_regex.clone(),
                source,
            }
        })?;
        Ok(Self {
            scan_regex,
            scan_regex_raw: raw.scan_regex,
            id_pattern: raw.id_pattern,
            default_project_code: raw.default_project_code,
        })
    }

    /// Fallback used when `work_item` is absent from config (pre-Phase-2
    /// launchers). Behaves identically to the default `{number:04d}` pattern.
    pub fn default_numeric() -> Self {
        let raw = "^([0-9]+)-".to_string();
        Self {
            scan_regex: regex::Regex::new(&raw).unwrap(),
            scan_regex_raw: raw,
            id_pattern: "{number:04d}".to_string(),
            default_project_code: None,
        }
    }

    /// Extract the full-string work-item ID from a filename.
    ///
    /// Two-pass admission:
    /// 1. Primary: apply the configured scan regex. Capture group 1 is the
    ///    digit run; the full ID is `<project_code>-<digits>` when a project
    ///    code is configured, or just `<digits>` for the numeric-only pattern.
    /// 2. Fallback (project-prefixed pattern only): if the primary regex fails
    ///    and a `default_project_code` is set, try the bare-numeric form.
    ///    On match, key the file as `<project_code>-<digits>` so legacy
    ///    bare-numeric files remain reachable during a pattern-config rollout.
    pub fn extract_id(&self, filename: &str) -> Option<String> {
        if let Some(cap) = self.scan_regex.captures(filename) {
            let digits = cap.get(1)?.as_str();
            return Some(match &self.default_project_code {
                Some(code) => format!("{code}-{digits}"),
                None => digits.to_string(),
            });
        }
        // Fallback: only when a project code is configured.
        let code = self.default_project_code.as_deref()?;
        let dash = filename.find('-')?;
        let prefix = &filename[..dash];
        if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(format!("{code}-{prefix}"))
    }
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
        let bytes = std::fs::read(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

const DEFAULT_KANBAN_COLUMN_KEYS: &[&str] = &[
    "draft", "ready", "in-progress", "review", "done", "blocked", "abandoned",
];

fn label_from_key(key: &str) -> String {
    let spaced = key.replace('-', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
    }
}

impl Config {
    /// Resolves the raw `kanban_columns` field (list of key strings, or None for
    /// "use defaults") into a validated `Vec<KanbanColumn>` with derived labels.
    ///
    /// Semantics:
    /// - `None` (field absent from config) → seven template-status defaults.
    /// - `Some(keys)` where `keys` is non-empty → one `KanbanColumn` per key.
    /// - `Some([])` (empty list) → `ConfigError::EmptyKanbanColumns` (reject at boot).
    pub fn resolve_kanban_columns(&self) -> Result<Vec<KanbanColumn>, ConfigError> {
        match &self.kanban_columns {
            None => Ok(DEFAULT_KANBAN_COLUMN_KEYS
                .iter()
                .map(|k| KanbanColumn { key: k.to_string(), label: label_from_key(k) })
                .collect()),
            Some(keys) if keys.is_empty() => Err(ConfigError::EmptyKanbanColumns),
            Some(keys) => Ok(keys
                .iter()
                .map(|k| KanbanColumn { key: k.clone(), label: label_from_key(k) })
                .collect()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid work-item scan regex '{pattern}': {source}")]
    InvalidScanRegex {
        pattern: String,
        source: regex::Error,
    },
    #[error("visualiser.kanban_columns must not be empty")]
    EmptyKanbanColumns,
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
        assert_eq!(c.templates.len(), 8);
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
        let err = serde_json::from_str::<Config>(json).expect_err("unknown field must fail");
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
        assert_eq!(
            adr.config_override.as_deref().unwrap(),
            std::path::Path::new("/custom/adr.md")
        );
    }

    #[test]
    fn work_item_config_from_raw_compiles_valid_regex() {
        let raw = RawWorkItemConfig {
            scan_regex: "^([0-9]+)-".to_string(),
            id_pattern: "{number:04d}".to_string(),
            default_project_code: None,
        };
        assert!(WorkItemConfig::from_raw(raw).is_ok());
    }

    #[test]
    fn work_item_config_from_raw_rejects_invalid_regex() {
        let raw = RawWorkItemConfig {
            scan_regex: "([unclosed".to_string(),
            id_pattern: "{number:04d}".to_string(),
            default_project_code: None,
        };
        let err = WorkItemConfig::from_raw(raw).expect_err("invalid regex must fail");
        assert!(matches!(err, ConfigError::InvalidScanRegex { .. }));
    }

    #[test]
    fn work_item_config_extract_id_default_pattern() {
        let cfg = WorkItemConfig::default_numeric();
        assert_eq!(cfg.extract_id("0001-foo.md").as_deref(), Some("0001"));
        assert_eq!(cfg.extract_id("0042-bar-baz.md").as_deref(), Some("0042"));
        assert_eq!(cfg.extract_id("malformed.md"), None);
        assert_eq!(cfg.extract_id("ADR-0001-foo.md"), None);
    }

    #[test]
    fn work_item_config_extract_id_project_pattern() {
        let cfg = WorkItemConfig::from_raw(RawWorkItemConfig {
            scan_regex: "^PROJ-([0-9]+)-".to_string(),
            id_pattern: "{project}-{number:04d}".to_string(),
            default_project_code: Some("PROJ".to_string()),
        })
        .unwrap();
        assert_eq!(cfg.extract_id("PROJ-0042-foo.md").as_deref(), Some("PROJ-0042"));
        assert_eq!(cfg.extract_id("PROJ-1-short.md").as_deref(), Some("PROJ-1"));
        assert_eq!(cfg.extract_id("PROJ-0042.md"), None);
        assert_eq!(cfg.extract_id("malformed.md"), None);
    }

    #[test]
    fn work_item_config_extract_id_fallback_for_legacy_files() {
        let cfg = WorkItemConfig::from_raw(RawWorkItemConfig {
            scan_regex: "^PROJ-([0-9]+)-".to_string(),
            id_pattern: "{project}-{number:04d}".to_string(),
            default_project_code: Some("PROJ".to_string()),
        })
        .unwrap();
        // Bare-numeric file that doesn't match the project pattern:
        // admitted via fallback keyed as PROJ-<digits>.
        assert_eq!(cfg.extract_id("0042-foo.md").as_deref(), Some("PROJ-0042"));
        assert_eq!(cfg.extract_id("0001-bar.md").as_deref(), Some("PROJ-0001"));
    }

    #[test]
    fn work_item_config_extract_id_no_fallback_without_project_code() {
        let cfg = WorkItemConfig::default_numeric();
        // Default pattern: bare-numeric files go through primary, not fallback.
        // Non-numeric prefixes are rejected.
        assert_eq!(cfg.extract_id("ADR-0001-foo.md"), None);
    }

    fn bare_config_json() -> &'static str {
        r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0", "project_root": "/r",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {}
        }"#
    }

    #[test]
    fn kanban_columns_missing_field_falls_back_to_defaults() {
        let cfg: Config = serde_json::from_str(bare_config_json()).unwrap();
        let cols = cfg.resolve_kanban_columns().unwrap();
        assert_eq!(
            cols.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
            vec!["draft", "ready", "in-progress", "review", "done", "blocked", "abandoned"]
        );
    }

    #[test]
    fn kanban_columns_labels_derived_from_keys() {
        let cfg: Config = serde_json::from_str(bare_config_json()).unwrap();
        let cols = cfg.resolve_kanban_columns().unwrap();
        let draft = cols.iter().find(|c| c.key == "draft").unwrap();
        assert_eq!(draft.label, "Draft");
        let ip = cols.iter().find(|c| c.key == "in-progress").unwrap();
        assert_eq!(ip.label, "In progress");
    }

    #[test]
    fn kanban_columns_read_from_config() {
        let json = r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0", "project_root": "/r",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {},
            "kanban_columns": ["ready", "in-progress", "done"]
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        let cols = cfg.resolve_kanban_columns().unwrap();
        assert_eq!(
            cols.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
            vec!["ready", "in-progress", "done"]
        );
    }

    #[test]
    fn kanban_columns_empty_list_rejected_at_boot() {
        let json = r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0", "project_root": "/r",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {},
            "kanban_columns": []
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        let err = cfg.resolve_kanban_columns().unwrap_err();
        assert!(matches!(err, ConfigError::EmptyKanbanColumns));
    }
}
