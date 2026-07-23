//! The `review` view assembly.
//!
//! Resolves the effective review settings for a mode, discovers custom lenses
//! and filters them by `applies_to`, and builds the unified lens catalogue. All
//! validation warnings are collected for stderr.

use config::{
    catalogue, ConfigAccess, ConfigError, Key, ReadLensCatalogue, Resolved,
    Value,
};

/// Which review the settings are rendered for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Pr,
    Plan,
    WorkItem,
}

impl Mode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pr => "pr",
            Self::Plan => "plan",
            Self::WorkItem => "work-item",
        }
    }
}

/// One labelled threshold value and its default (for the override annotation).
pub struct ValueLine {
    pub label: &'static str,
    pub value: String,
    pub default: String,
}

/// One custom lens row for the catalogue.
pub struct CustomRow {
    pub name: String,
    pub path: String,
    pub always_include: bool,
}

/// The resolved verdict thresholds, formatted into output lines by the renderer.
pub enum Verdict {
    /// The pr-mode `REQUEST_CHANGES` escalation severity.
    Pr { severity: String },
    /// The plan/work-item `REVISE` escalation severity and major count, with the
    /// count's default (for the override annotation).
    Revise {
        severity: String,
        count: String,
        count_default: String,
    },
}

/// The assembled review view.
pub struct ReviewView {
    pub values: Vec<ValueLine>,
    pub core_lenses: Vec<String>,
    pub filtered_core_lenses: Vec<String>,
    pub disabled_lenses: Vec<String>,
    pub verdict: Verdict,
    pub builtin_lenses: Vec<&'static str>,
    pub custom_rows: Vec<CustomRow>,
    pub warnings: Vec<String>,
    /// Work-item built-in lenses absent from a non-empty `core_lenses`; the
    /// renderer turns a non-empty set into the informational note.
    pub missing_builtin_lenses: Vec<String>,
}

const SEVERITIES: &[&str] = &["critical", "major", "none"];

/// # Errors
///
/// A [`ConfigError`] when a config level or lens directory cannot be read.
pub fn assemble(
    config: &dyn ConfigAccess,
    lenses: &dyn ReadLensCatalogue,
    mode: Mode,
) -> Result<ReviewView, ConfigError> {
    let mut warnings = Vec::new();
    let min_default = if mode == Mode::WorkItem {
        "3".to_owned()
    } else {
        catalogue_default("review.min_lenses")
    };
    let max_default = catalogue_default("review.max_lenses");

    let min = positive(config, "min_lenses", &min_default, &mut warnings)?;
    let max = positive(config, "max_lenses", &max_default, &mut warnings)?;
    let (min_lenses, max_lenses) = if int(&min) > int(&max) {
        warnings.push(format!(
            "review.min_lenses ({min}) > review.max_lenses ({max}) — using \
             defaults ({min_default}, {max_default})"
        ));
        (min_default.clone(), max_default.clone())
    } else {
        (min, max)
    };

    let mut values = threshold_lines(config, mode, &mut warnings)?;
    values.push(ValueLine {
        label: "min lenses",
        value: min_lenses.clone(),
        default: min_default,
    });
    values.push(ValueLine {
        label: "max lenses",
        value: max_lenses,
        default: max_default,
    });

    let core_lenses = resolve_list(config, "review.core_lenses")?;
    let disabled_lenses = resolve_list(config, "review.disabled_lenses")?;
    for lens in &core_lenses {
        if disabled_lenses.contains(lens) {
            warnings.push(format!(
                "Lens '{lens}' appears in both core_lenses and \
                 disabled_lenses — disabled_lenses takes precedence"
            ));
        }
    }

    let discovered = discover_custom(lenses, &mut warnings)?;
    let active = filter_by_mode(&discovered, mode, &mut warnings);

    let builtin_lenses = builtins_for(mode);
    let filtered = split_core_lenses(
        &core_lenses,
        &discovered,
        &builtin_lenses,
        &active,
        &mut warnings,
    );
    lens_count_warnings(
        &builtin_lenses,
        &active,
        &disabled_lenses,
        int(&min_lenses),
        &mut warnings,
    );

    let verdict = resolve_verdict(config, mode, &mut warnings)?;

    let missing_builtin_lenses = missing_core_lenses(
        mode,
        &builtin_lenses,
        &core_lenses,
        &disabled_lenses,
    );

    Ok(ReviewView {
        values,
        core_lenses,
        filtered_core_lenses: filtered,
        disabled_lenses,
        verdict,
        builtin_lenses,
        custom_rows: active
            .iter()
            .map(|lens| CustomRow {
                name: lens.name.clone(),
                path: lens.path.clone(),
                always_include: lens.auto_detect.is_none(),
            })
            .collect(),
        warnings,
        missing_builtin_lenses,
    })
}

/// In work-item mode, when the user sets `core_lenses` to a subset of the
/// built-in work-item lenses, the built-ins that will still be added up to
/// `max_lenses` (empty when the note does not apply).
fn missing_core_lenses(
    mode: Mode,
    builtins: &[&str],
    core_lenses: &[String],
    disabled_lenses: &[String],
) -> Vec<String> {
    if mode != Mode::WorkItem || core_lenses.is_empty() {
        return Vec::new();
    }
    builtins
        .iter()
        .copied()
        .filter(|lens| {
            !disabled_lenses.iter().any(|d| d == lens)
                && !core_lenses.iter().any(|c| c == lens)
        })
        .map(str::to_owned)
        .collect()
}

/// The mode-specific threshold value lines (before `min`/`max`).
fn threshold_lines(
    config: &dyn ConfigAccess,
    mode: Mode,
    warnings: &mut Vec<String>,
) -> Result<Vec<ValueLine>, ConfigError> {
    Ok(match mode {
        Mode::Pr => vec![
            non_negative_line(
                config,
                "max inline comments",
                "max_inline_comments",
                &catalogue_default("review.max_inline_comments"),
                warnings,
            )?,
            non_negative_line(
                config,
                "dedup proximity",
                "dedup_proximity",
                &catalogue_default("review.dedup_proximity"),
                warnings,
            )?,
            severity_line(
                config,
                "pr request changes severity",
                "pr_request_changes_severity",
                warnings,
            )?,
        ],
        Mode::Plan => vec![
            severity_line(
                config,
                "plan revise severity",
                "plan_revise_severity",
                warnings,
            )?,
            positive_line(
                config,
                "plan revise major count",
                "plan_revise_major_count",
                &catalogue_default("review.plan_revise_major_count"),
                warnings,
            )?,
        ],
        Mode::WorkItem => vec![
            severity_line(
                config,
                "work-item revise severity",
                "work_item_revise_severity",
                warnings,
            )?,
            positive_line(
                config,
                "work-item revise major count",
                "work_item_revise_major_count",
                &catalogue_default("review.work_item_revise_major_count"),
                warnings,
            )?,
        ],
    })
}

fn builtins_for(mode: Mode) -> Vec<&'static str> {
    if mode == Mode::WorkItem {
        catalogue::BUILTIN_WORK_ITEM_LENSES.to_vec()
    } else {
        catalogue::BUILTIN_CODE_LENSES.to_vec()
    }
}

struct DiscoveredLens {
    name: String,
    path: String,
    auto_detect: Option<String>,
    applies_to: Option<String>,
}

fn discover_custom(
    lenses: &dyn ReadLensCatalogue,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredLens>, ConfigError> {
    let mut discovered = Vec::new();
    for lens in lenses.custom_lenses()? {
        let Some(fields) = lens.fields else {
            warnings.push(format!(
                "Custom lens at {} has invalid frontmatter — skipping",
                lens.dir
            ));
            continue;
        };
        let name = match fields.name {
            None => {
                warnings.push(format!(
                    "Custom lens at {} missing 'name' in frontmatter — \
                     skipping",
                    lens.dir
                ));
                continue;
            }
            Some(name) if name.is_empty() => {
                warnings.push(format!(
                    "Custom lens at {} has empty 'name' in frontmatter — \
                     skipping",
                    lens.dir
                ));
                continue;
            }
            Some(name) => name,
        };
        if is_builtin(&name) {
            warnings.push(format!(
                "Custom lens '{name}' conflicts with built-in lens name — \
                 skipping"
            ));
            continue;
        }
        discovered.push(DiscoveredLens {
            name,
            path: lens.path,
            auto_detect: fields.auto_detect.filter(|value| !value.is_empty()),
            applies_to: fields.applies_to,
        });
    }
    Ok(discovered)
}

fn filter_by_mode<'a>(
    discovered: &'a [DiscoveredLens],
    mode: Mode,
    warnings: &mut Vec<String>,
) -> Vec<&'a DiscoveredLens> {
    let mut active = Vec::new();
    for lens in discovered {
        match &lens.applies_to {
            None => active.push(lens),
            Some(raw) => {
                if applies_to_modes(&lens.name, raw, warnings)
                    .contains(&mode.label().to_owned())
                {
                    active.push(lens);
                }
            }
        }
    }
    active
}

/// Parses an `applies_to` value into its recognised modes, warning on empty or
/// unrecognised entries, matching bash `validate_applies_to`.
fn applies_to_modes(
    lens_name: &str,
    raw: &str,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let stripped = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(raw);
    if stripped.chars().all(char::is_whitespace) {
        warnings.push(format!(
            "Custom lens '{lens_name}' has empty applies_to — lens will not \
             appear in any mode"
        ));
        return Vec::new();
    }
    let mut modes = Vec::new();
    for entry in stripped.split(',') {
        let mode: String =
            entry.chars().filter(|c| !c.is_whitespace()).collect();
        if mode.is_empty() {
            continue;
        }
        if !["pr", "plan", "work-item"].contains(&mode.as_str()) {
            warnings.push(format!(
                "Custom lens '{lens_name}' declares applies_to containing \
                 unrecognised mode '{mode}' — ignoring that entry"
            ));
            continue;
        }
        if !modes.contains(&mode) {
            modes.push(mode);
        }
    }
    modes
}

fn is_builtin(name: &str) -> bool {
    catalogue::BUILTIN_CODE_LENSES.contains(&name)
        || catalogue::BUILTIN_WORK_ITEM_LENSES.contains(&name)
}

/// The valid-cross-mode core lenses dropped for this mode (the `Filtered core
/// lenses` block), warning on unrecognised entries.
fn split_core_lenses(
    core_lenses: &[String],
    discovered: &[DiscoveredLens],
    builtins: &[&str],
    active: &[&DiscoveredLens],
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let all_valid = |name: &str| {
        is_builtin(name) || discovered.iter().any(|lens| lens.name == name)
    };
    let active_mode = |name: &str| {
        builtins.contains(&name) || active.iter().any(|lens| lens.name == name)
    };
    let mut filtered = Vec::new();
    for lens in core_lenses {
        if !all_valid(lens) {
            warnings.push(format!(
                "review.core_lenses contains unrecognised lens '{lens}'"
            ));
        } else if !active_mode(lens) {
            filtered.push(lens.clone());
        }
    }
    filtered
}

fn lens_count_warnings(
    builtins: &[&str],
    active: &[&DiscoveredLens],
    disabled: &[String],
    min_lenses: i64,
    warnings: &mut Vec<String>,
) {
    for lens in disabled {
        if !is_builtin(lens)
            && !active.iter().any(|active| &active.name == lens)
        {
            warnings.push(format!(
                "review.disabled_lenses contains unrecognised lens '{lens}'"
            ));
        }
    }
    let active_names: Vec<String> = builtins
        .iter()
        .map(|b| (*b).to_owned())
        .chain(active.iter().map(|lens| lens.name.clone()))
        .collect();
    let available = active_names
        .iter()
        .filter(|name| !disabled.contains(name))
        .count();
    if i64::try_from(available).unwrap_or(i64::MAX) < min_lenses {
        warnings.push(format!(
            "Only {available} lenses available after disabling, but \
             min_lenses is {min_lenses}"
        ));
    }
}

fn resolve_verdict(
    config: &dyn ConfigAccess,
    mode: Mode,
    warnings: &mut Vec<String>,
) -> Result<Verdict, ConfigError> {
    match mode {
        Mode::Pr => Ok(Verdict::Pr {
            severity: severity(
                config,
                "pr_request_changes_severity",
                warnings,
            )?,
        }),
        Mode::Plan => resolve_revise(
            config,
            "plan_revise_severity",
            "plan_revise_major_count",
            "3",
            warnings,
        ),
        Mode::WorkItem => resolve_revise(
            config,
            "work_item_revise_severity",
            "work_item_revise_major_count",
            "2",
            warnings,
        ),
    }
}

fn resolve_revise(
    config: &dyn ConfigAccess,
    severity_key: &str,
    count_key: &str,
    count_default: &str,
    warnings: &mut Vec<String>,
) -> Result<Verdict, ConfigError> {
    Ok(Verdict::Revise {
        severity: severity(config, severity_key, warnings)?,
        count: positive(config, count_key, count_default, warnings)?,
        count_default: count_default.to_owned(),
    })
}

/// The catalogue default for a full `review.*` key, rendered to its scalar
/// string (empty when the key carries none).
fn catalogue_default(key: &str) -> String {
    catalogue::default_for(key)
        .map(|value| config::render_value(&value))
        .unwrap_or_default()
}

fn resolve(
    config: &dyn ConfigAccess,
    key: &str,
    default: &str,
) -> Result<String, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.get(&parsed, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => default.to_owned(),
    })
}

fn resolve_list(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Vec<String>, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.get(&parsed, None)? {
        Resolved::Found(Value::Sequence(items)) => items
            .iter()
            .map(|item| config::render_value(&Value::Scalar(item.clone())))
            .filter(|item| !item.is_empty())
            .collect(),
        _ => Vec::new(),
    })
}

fn int(value: &str) -> i64 {
    value.parse().unwrap_or(0)
}

fn is_non_negative(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

fn non_negative(
    config: &dyn ConfigAccess,
    key: &str,
    default: &str,
    warnings: &mut Vec<String>,
) -> Result<String, ConfigError> {
    let value = resolve(config, &format!("review.{key}"), default)?;
    if is_non_negative(&value) {
        Ok(value)
    } else {
        warnings.push(format!(
            "review.{key} must be a non-negative integer, got '{value}' — \
             using default ({default})"
        ));
        Ok(default.to_owned())
    }
}

fn positive(
    config: &dyn ConfigAccess,
    key: &str,
    default: &str,
    warnings: &mut Vec<String>,
) -> Result<String, ConfigError> {
    let value = resolve(config, &format!("review.{key}"), default)?;
    if is_non_negative(&value) && value != "0" {
        Ok(value)
    } else {
        warnings.push(format!(
            "review.{key} must be a positive integer, got '{value}' — using \
             default ({default})"
        ));
        Ok(default.to_owned())
    }
}

fn severity(
    config: &dyn ConfigAccess,
    key: &str,
    warnings: &mut Vec<String>,
) -> Result<String, ConfigError> {
    let default = catalogue_default(&format!("review.{key}"));
    let value = resolve(config, &format!("review.{key}"), &default)?;
    if SEVERITIES.contains(&value.as_str()) {
        Ok(value)
    } else {
        warnings.push(format!(
            "review.{key} must be 'critical', 'major', or 'none', got \
             '{value}' — using default (critical)"
        ));
        Ok(default)
    }
}

fn non_negative_line(
    config: &dyn ConfigAccess,
    label: &'static str,
    key: &str,
    default: &str,
    warnings: &mut Vec<String>,
) -> Result<ValueLine, ConfigError> {
    Ok(ValueLine {
        label,
        value: non_negative(config, key, default, warnings)?,
        default: default.to_owned(),
    })
}

fn positive_line(
    config: &dyn ConfigAccess,
    label: &'static str,
    key: &str,
    default: &str,
    warnings: &mut Vec<String>,
) -> Result<ValueLine, ConfigError> {
    Ok(ValueLine {
        label,
        value: positive(config, key, default, warnings)?,
        default: default.to_owned(),
    })
}

fn severity_line(
    config: &dyn ConfigAccess,
    label: &'static str,
    key: &str,
    warnings: &mut Vec<String>,
) -> Result<ValueLine, ConfigError> {
    Ok(ValueLine {
        label,
        value: severity(config, key, warnings)?,
        default: catalogue_default(&format!("review.{key}")),
    })
}
