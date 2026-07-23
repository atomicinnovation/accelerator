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
    /// Idle auto-shutdown window as a humantime duration string
    /// (`"8h"`, `"30m"`, `"1h30m"`), or a disable token (`"never"`, `0`,
    /// or any zero-length duration). Absent → the built-in 8h default.
    /// Resolved + validated at boot by `resolve_idle_limit_ms`.
    #[serde(default)]
    pub idle_timeout: Option<String>,
    /// Editor deep-link selection: a preset key (e.g. `vscode`, `cursor`,
    /// `idea`, `web-storm`) or a custom URL template containing `://` or an
    /// `{abs}`/`{rel}` placeholder. Absent → `Open in editor` renders disabled.
    /// Passed through verbatim; the frontend resolves presets/templates.
    #[serde(default)]
    pub editor: Option<String>,
    /// `JetBrains` project name for the `{project}` placeholder. Absent →
    /// server defaults to the basename of `project_root`. Ignored by
    /// non-JetBrains presets.
    #[serde(default)]
    pub editor_project: Option<String>,
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
        let scan_regex =
            regex::Regex::new(&raw.scan_regex).map_err(|source| {
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

    /// True iff `token` is exactly a canonical work-item id under this
    /// configuration. The width is parsed from `id_pattern`'s
    /// `{number:0Nd}` segment; tokens with the wrong digit count
    /// (or a missing/incorrect project prefix) are rejected.
    ///
    /// This is distinct from `extract_id` (which uses the more permissive
    /// `scan_regex` and requires a trailing `-`) and from `normalise_id`
    /// (which pads bare digits to the canonical width). The token
    /// predicate admits only canonical-form strings, with no padding and
    /// no surrounding context — the right tool for slug-prefix stripping.
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        let width = self.canonical_digit_width();
        let digits = match &self.default_project_code {
            Some(code) => match token.strip_prefix(&format!("{code}-")) {
                Some(rest) => rest,
                None => return false,
            },
            None => token,
        };
        if width == 0 {
            // No width specifier: admit any non-empty digit run.
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        } else {
            digits.len() == width && digits.chars().all(|c| c.is_ascii_digit())
        }
    }

    fn canonical_digit_width(&self) -> usize {
        let s = &self.id_pattern;
        let Some(i) = s.find("{number") else {
            return 0;
        };
        let rest = &s[i + "{number".len()..];
        let Some(end) = rest.find('}') else {
            return 0;
        };
        let spec = &rest[..end];
        let trimmed = spec.trim_start_matches(':').trim_end_matches('d');
        if trimmed.is_empty() {
            return 0;
        }
        let digits = trimmed.trim_start_matches('0');
        if digits.is_empty() {
            // Was "0" or "00…" — admit any digit count.
            return 0;
        }
        digits.parse::<usize>().ok().unwrap_or(0)
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

    /// Validate and normalise a work-item ID from any source (frontmatter,
    /// API input, etc.). Accepts:
    /// - Bare digits (`"42"` or `"0042"`): when `default_project_code` is
    ///   set, prefixes with the code (`"ENG-42"`); otherwise returns the
    ///   digits unchanged.
    /// - Prefixed form (`"ENG-0042"`, `"OPS-7"`): returns the value
    ///   verbatim — the workspace's `default_project_code` is NOT
    ///   re-applied, so multi-prefix coexistence under remote sync works.
    ///
    /// Returns `None` for any other shape (`"ENG0042"`, `"PROJ-1.2"`,
    /// `""`, whitespace).
    pub fn normalise_id(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some((prefix, digits)) = trimmed.split_once('-') {
            if prefix.is_empty()
                || !prefix.chars().all(|c| c.is_ascii_alphabetic())
                || digits.is_empty()
                || !digits.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            return Some(trimmed.to_string());
        }
        if !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(match &self.default_project_code {
            Some(code) => format!("{code}-{trimmed}"),
            None => trimmed.to_string(),
        })
    }
}

impl Default for WorkItemConfig {
    fn default() -> Self {
        Self::default_numeric()
    }
}

#[cfg(test)]
impl WorkItemConfig {
    pub fn with_pattern_for_test(prefix: &str, width: usize) -> Self {
        let raw = format!("^({}-[0-9]{{{}}})-", regex::escape(prefix), width);
        Self {
            scan_regex: regex::Regex::new(&raw).unwrap(),
            scan_regex_raw: raw,
            // Use the literal `{project}` placeholder so `id_pattern.contains
            // ("{project}")` lookups behave like production configs.
            id_pattern: format!("{{project}}-{{number:0{width}d}}"),
            default_project_code: Some(prefix.to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateTiers {
    pub config_override: Option<PathBuf>,
    pub user_override: PathBuf,
    pub plugin_default: PathBuf,
    /// Path of the config file (relative to the project root) in which
    /// `config_override` is declared, when known to the launcher. Used
    /// by the templates view's Tier 1 description text. `None` when
    /// there is no config-override or the launcher does not surface
    /// this information.
    #[serde(default)]
    pub config_override_source: Option<String>,
}

impl TemplateTiers {
    /// Iterate over the three tier paths (config-override, user-override,
    /// plugin-default). Skips the config-override slot when it is `None`.
    pub fn iter_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.config_override
            .iter()
            .cloned()
            .chain(std::iter::once(self.user_override.clone()))
            .chain(std::iter::once(self.plugin_default.clone()))
    }
}

impl Config {
    pub fn from_path(path: &std::path::Path) -> Result<Self, ConfigError> {
        let bytes =
            std::fs::read(path).map_err(|source| ConfigError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        serde_json::from_slice(&bytes).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

// Runtime fallback for the kanban columns when config omits them. The
// authoritative declaration is `visualiser.kanban_columns` in the config
// catalogue (`cli/config`); this crate can't depend on it, so keep them in sync.
const DEFAULT_KANBAN_COLUMN_KEYS: &[&str] = &[
    "draft",
    "ready",
    "in-progress",
    "review",
    "done",
    "blocked",
    "abandoned",
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
    pub fn resolve_kanban_columns(
        &self,
    ) -> Result<Vec<KanbanColumn>, ConfigError> {
        match &self.kanban_columns {
            None => Ok(DEFAULT_KANBAN_COLUMN_KEYS
                .iter()
                .map(|k| KanbanColumn {
                    key: (*k).to_string(),
                    label: label_from_key(k),
                })
                .collect()),
            Some(keys) if keys.is_empty() => {
                Err(ConfigError::EmptyKanbanColumns)
            }
            Some(keys) => Ok(keys
                .iter()
                .map(|k| KanbanColumn {
                    key: k.clone(),
                    label: label_from_key(k),
                })
                .collect()),
        }
    }
}

/// Runtime fallback for the idle window when config omits it. The authoritative
/// declaration is `visualiser.idle_timeout` in the config catalogue
/// (`cli/config`); this crate can't depend on it, so keep the two in sync.
const DEFAULT_IDLE_TIMEOUT: &str = "8h";

/// Sentinel meaning "idle auto-shutdown disabled". Inert against the
/// `idle >= idle_limit_ms` comparison in lifecycle.rs (the production loop
/// just compares the value it is handed; the sentinel never appears there
/// literally).
///
/// The disable tests (the owner-death test in `tests/lifecycle_owner.rs` and the
/// new `disabled_idle_never_fires`) reference this exported constant rather than a
/// bare `i64::MAX`, so the disable contract is named in one place and shared by
/// import — a future change to the idle comparison cannot silently break the
/// disable assumption without the named constant showing up in the diff.
pub const DISABLED_IDLE_LIMIT_MS: i64 = i64::MAX;

/// Largest finite idle window we store: one below the disable sentinel,
/// so an over-large configured duration clamps here and can never be
/// mistaken for "disabled".
const MAX_IDLE_LIMIT_MS: i64 = DISABLED_IDLE_LIMIT_MS - 1;

impl Config {
    /// Resolve the idle window into milliseconds, or the disable sentinel.
    ///
    /// Semantics:
    /// - Absent field → the built-in `"8h"` default, parsed through the
    ///   same path as user input.
    /// - `"never"` (case-insensitive), the bare `"0"`, or *any* zero-length
    ///   duration (`"0s"`, `"0ms"`, …) → `DISABLED_IDLE_LIMIT_MS`, so the
    ///   "zero idle window" case is uniform regardless of spelling.
    /// - Any other value → parsed by `humantime`; an unparseable value is
    ///   rejected (`ConfigError::InvalidIdleTimeout`, carrying the
    ///   underlying parse error) so the server fails fast at boot rather
    ///   than silently defaulting.
    pub fn resolve_idle_limit_ms(&self) -> Result<i64, ConfigError> {
        let raw = self.idle_timeout.as_deref().unwrap_or(DEFAULT_IDLE_TIMEOUT);
        let trimmed = raw.trim();
        // Disable tokens handled before parsing: the textual "never" and the
        // bare "0" (which humantime cannot parse, lacking a unit).
        if trimmed.eq_ignore_ascii_case("never") || trimmed == "0" {
            return Ok(DISABLED_IDLE_LIMIT_MS);
        }
        let dur = humantime::parse_duration(trimmed).map_err(|source| {
            ConfigError::InvalidIdleTimeout {
                value: raw.to_string(),
                source,
            }
        })?;
        // A zero-length window ("0s", "0ms", …) also disables, matching the
        // bare-"0" token above.
        if dur.is_zero() {
            return Ok(DISABLED_IDLE_LIMIT_MS);
        }
        // Saturate in u128 *before* the i64 cast (an over-large duration must
        // clamp to MAX_IDLE_LIMIT_MS, never wrap negative), and floor at 1ms so a
        // sub-millisecond-but-non-zero value ("1ns", "500us") — which is NOT
        // is_zero() yet truncates to 0 ms — stays a tiny finite window (fires on
        // the next tick) rather than collapsing to `idle_limit_ms == 0`, which
        // the loop would treat as fire-on-first-tick.
        Ok(dur.as_millis().min(MAX_IDLE_LIMIT_MS as u128).max(1) as i64)
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
    // The accepted-format guidance is duplicated in write-visualiser-config.sh's
    // pre-flight error — keep the two messages in sync.
    #[error("invalid visualiser.idle_timeout '{value}': expected a duration like \"8h\", \"30m\", \"1h30m\", or \"never\"/0 to disable: {source}")]
    InvalidIdleTimeout {
        value: String,
        source: humantime::DurationError,
    },
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
        let c =
            Config::from_path(&fixture("config.valid.json")).expect("valid");
        assert_eq!(c.plugin_version, "1.19.0-pre.2");
        assert_eq!(c.host, "127.0.0.1");
        assert_eq!(c.owner_pid, 0);
        assert_eq!(c.doc_paths.len(), 12);
        assert!(c.doc_paths.contains_key("decisions"));
        assert!(c.doc_paths.contains_key("review_plans"));
        assert!(c.doc_paths.contains_key("review_prs"));
        assert_eq!(c.templates.len(), 13);
        let adr = c.templates.get("adr").expect("adr tier");
        assert!(adr.config_override.is_none());
        assert!(adr.user_override.ends_with("adr.md"));
        assert!(adr.plugin_default.ends_with("adr.md"));
        assert!(adr.config_override_source.is_none());
        // The previously-renamed and newly-added names deserialise and are
        // tier-shaped (incl. the fourth key, config_override_source).
        assert!(c.templates.contains_key("codebase-research"));
        let rca = c.templates.get("rca").expect("rca tier");
        assert!(rca.config_override.is_none());
        assert!(rca.config_override_source.is_none());
        assert!(rca.plugin_default.ends_with("rca.md"));
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
        let err =
            Config::from_path(std::path::Path::new("/nonexistent/config.json"))
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
        let err =
            WorkItemConfig::from_raw(raw).expect_err("invalid regex must fail");
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
        assert_eq!(
            cfg.extract_id("PROJ-0042-foo.md").as_deref(),
            Some("PROJ-0042")
        );
        assert_eq!(
            cfg.extract_id("PROJ-1-short.md").as_deref(),
            Some("PROJ-1")
        );
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

    #[test]
    fn normalise_id_passes_prefixed_form_through_unchanged() {
        let cfg = WorkItemConfig::default_numeric();
        assert_eq!(cfg.normalise_id("ENG-0042").as_deref(), Some("ENG-0042"));
        assert_eq!(cfg.normalise_id("OPS-7").as_deref(), Some("OPS-7"));
    }

    #[test]
    fn normalise_id_applies_project_code_to_bare_digits() {
        let cfg = WorkItemConfig::from_raw(RawWorkItemConfig {
            scan_regex: "^ENG-([0-9]+)-".to_string(),
            id_pattern: "{project}-{number:04d}".to_string(),
            default_project_code: Some("ENG".to_string()),
        })
        .unwrap();
        assert_eq!(cfg.normalise_id("42").as_deref(), Some("ENG-42"));
        assert_eq!(cfg.normalise_id("0042").as_deref(), Some("ENG-0042"));
    }

    #[test]
    fn normalise_id_preserves_foreign_prefix_when_default_code_is_set() {
        // Multi-prefix coexistence: a frontmatter `work_item_id: "OPS-7"`
        // in a workspace whose `default_project_code` is "ENG" passes
        // through verbatim — the workspace's code is NOT re-applied.
        let cfg = WorkItemConfig::from_raw(RawWorkItemConfig {
            scan_regex: "^ENG-([0-9]+)-".to_string(),
            id_pattern: "{project}-{number:04d}".to_string(),
            default_project_code: Some("ENG".to_string()),
        })
        .unwrap();
        assert_eq!(cfg.normalise_id("OPS-7").as_deref(), Some("OPS-7"));
    }

    #[test]
    fn normalise_id_returns_bare_digits_when_no_default_code() {
        let cfg = WorkItemConfig::default_numeric();
        assert_eq!(cfg.normalise_id("42").as_deref(), Some("42"));
        assert_eq!(cfg.normalise_id("0042").as_deref(), Some("0042"));
    }

    #[test]
    fn normalise_id_rejects_shape_invalid_values() {
        let cfg = WorkItemConfig::default_numeric();
        // Prefix without dash:
        assert_eq!(cfg.normalise_id("ENG0042"), None);
        // Dotted suffix:
        assert_eq!(cfg.normalise_id("PROJ-1.2"), None);
        // Empty / whitespace:
        assert_eq!(cfg.normalise_id(""), None);
        assert_eq!(cfg.normalise_id("   "), None);
        // Non-alphabetic prefix:
        assert_eq!(cfg.normalise_id("123-456"), None);
        // Trailing letters:
        assert_eq!(cfg.normalise_id("ENG-42abc"), None);
    }

    #[test]
    fn is_canonical_id_token_under_default_numeric() {
        let cfg = WorkItemConfig::default_numeric();
        assert!(cfg.is_canonical_id_token("0040"));
        assert!(!cfg.is_canonical_id_token("40"));
        assert!(!cfg.is_canonical_id_token("00040"));
        assert!(!cfg.is_canonical_id_token("100"));
        assert!(!cfg.is_canonical_id_token("004A"));
        assert!(!cfg.is_canonical_id_token(""));
    }

    #[test]
    fn is_canonical_id_token_under_project_prefixed_pattern() {
        let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
        assert!(cfg.is_canonical_id_token("PROJ-0040"));
        assert!(!cfg.is_canonical_id_token("0040"));
        assert!(!cfg.is_canonical_id_token("PROJ-40"));
        assert!(!cfg.is_canonical_id_token("OTHER-0040"));
    }

    #[test]
    fn default_impl_matches_default_numeric() {
        let a: WorkItemConfig = WorkItemConfig::default();
        let b = WorkItemConfig::default_numeric();
        assert_eq!(a.scan_regex_raw, b.scan_regex_raw);
        assert_eq!(a.id_pattern, b.id_pattern);
        assert_eq!(a.default_project_code, b.default_project_code);
    }

    #[test]
    fn normalise_id_trims_surrounding_whitespace() {
        let cfg = WorkItemConfig::default_numeric();
        assert_eq!(cfg.normalise_id("  0042  ").as_deref(), Some("0042"));
        assert_eq!(cfg.normalise_id(" ENG-7 ").as_deref(), Some("ENG-7"));
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
            vec![
                "draft",
                "ready",
                "in-progress",
                "review",
                "done",
                "blocked",
                "abandoned"
            ]
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

    /// Build a `Config` from `bare_config_json` with a single `idle_timeout`
    /// value spliced in, so each resolver case is a one-liner.
    fn config_with_idle_timeout(value: &str) -> Config {
        let json = format!(
            r#"{{
                "plugin_root": "/p", "plugin_version": "0.0.0", "project_root": "/r",
                "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
                "log_path": "/l", "doc_paths": {{}}, "templates": {{}},
                "idle_timeout": {value}
            }}"#
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn idle_timeout_absent_field_defaults_to_8h() {
        let cfg: Config = serde_json::from_str(bare_config_json()).unwrap();
        // Absolute assertion pins the unit (8h in ms).
        assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 28_800_000);
        // Relative drift guard ties the "8h" string default to the const.
        assert_eq!(
            cfg.resolve_idle_limit_ms().unwrap(),
            crate::lifecycle::Settings::DEFAULT.idle_limit_ms
        );
    }

    #[test]
    fn idle_timeout_simple_minutes() {
        let cfg = config_with_idle_timeout(r#""30m""#);
        assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 30 * 60 * 1000);
    }

    #[test]
    fn idle_timeout_compound_duration() {
        let cfg = config_with_idle_timeout(r#""1h30m""#);
        assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 90 * 60 * 1000);
    }

    #[test]
    fn idle_timeout_never_token_is_case_insensitive() {
        for token in [r#""never""#, r#""Never""#, r#""NEVER""#] {
            let cfg = config_with_idle_timeout(token);
            assert_eq!(
                cfg.resolve_idle_limit_ms().unwrap(),
                DISABLED_IDLE_LIMIT_MS,
                "token {token} must disable"
            );
        }
    }

    #[test]
    fn idle_timeout_bare_zero_disables() {
        // The bare "0" arrives as a JSON string "0" from config-read-value.sh.
        let cfg = config_with_idle_timeout(r#""0""#);
        assert_eq!(
            cfg.resolve_idle_limit_ms().unwrap(),
            DISABLED_IDLE_LIMIT_MS
        );
    }

    #[test]
    fn idle_timeout_zero_length_durations_disable() {
        for token in [r#""0s""#, r#""0ms""#] {
            let cfg = config_with_idle_timeout(token);
            assert_eq!(
                cfg.resolve_idle_limit_ms().unwrap(),
                DISABLED_IDLE_LIMIT_MS,
                "zero-length {token} must disable"
            );
        }
    }

    #[test]
    fn idle_timeout_sub_millisecond_floors_to_one_ms() {
        for token in [r#""1ns""#, r#""500us""#] {
            let cfg = config_with_idle_timeout(token);
            assert_eq!(
                cfg.resolve_idle_limit_ms().unwrap(),
                1,
                "sub-ms {token} must floor to 1ms, not 0 and not disabled"
            );
        }
    }

    #[test]
    fn idle_timeout_over_large_saturates_below_sentinel() {
        let cfg = config_with_idle_timeout(r#""100000000000years""#);
        let resolved = cfg.resolve_idle_limit_ms().unwrap();
        assert_eq!(resolved, MAX_IDLE_LIMIT_MS);
        assert_ne!(resolved, DISABLED_IDLE_LIMIT_MS);
        assert!(resolved > 0, "must not wrap negative or to zero");
    }

    #[test]
    fn idle_timeout_surrounding_whitespace_trimmed() {
        let cfg = config_with_idle_timeout(r#""  8h  ""#);
        assert_eq!(cfg.resolve_idle_limit_ms().unwrap(), 28_800_000);
    }

    #[test]
    fn idle_timeout_invalid_values_fail_fast() {
        for token in [r#""soon""#, r#""00""#, r#""0.0""#, r#""   ""#] {
            let cfg = config_with_idle_timeout(token);
            let err = cfg
                .resolve_idle_limit_ms()
                .expect_err(&format!("{token} must be rejected"));
            assert!(
                matches!(err, ConfigError::InvalidIdleTimeout { .. }),
                "expected InvalidIdleTimeout for {token}, got {err:?}"
            );
        }
    }

    #[test]
    fn editor_fields_absent_resolve_to_none() {
        // The `#[serde(default)]` fields are absent from `bare_config_json`;
        // assert they parse to `None` at runtime (the disabled-button contract).
        let cfg: Config = serde_json::from_str(bare_config_json()).unwrap();
        assert_eq!(cfg.editor, None);
        assert_eq!(cfg.editor_project, None);
    }

    #[test]
    fn editor_fields_parse_when_present() {
        let json = r#"{
            "plugin_root": "/p", "plugin_version": "0.0.0", "project_root": "/r",
            "tmp_path": "/t", "host": "127.0.0.1", "owner_pid": 0,
            "log_path": "/l", "doc_paths": {}, "templates": {},
            "editor": "cursor", "editor_project": "myrepo"
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.editor.as_deref(), Some("cursor"));
        assert_eq!(cfg.editor_project.as_deref(), Some("myrepo"));
    }
}
