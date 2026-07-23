//! The `config init` scaffold: resolves the project content directories and
//! the tmp directory, then applies the idempotent scaffold through the port.

use config::{ConfigAccess, ConfigError, Key, Scaffold};

/// The content-directory keys `init` creates. Each resolves through
/// `paths.<key>`, falling back to the catalogue default.
const CONTENT_KEYS: &[&str] = &[
    "plans",
    "research_codebase",
    "decisions",
    "prs",
    "validations",
    "review_plans",
    "review_prs",
    "review_work",
    "work",
    "notes",
    "research_design_inventories",
    "research_design_gaps",
    "global",
    "research_issues",
];

/// Resolves the scaffold directories and applies them.
///
/// # Errors
///
/// A [`ConfigError`] when a path cannot be read or a directory created.
pub fn run(
    config: &dyn ConfigAccess,
    scaffold: &dyn Scaffold,
) -> Result<(), ConfigError> {
    let mut dirs = Vec::with_capacity(CONTENT_KEYS.len());
    for key in CONTENT_KEYS {
        dirs.push(resolve_path(config, key)?);
    }
    let tmp = resolve_path(config, "tmp")?;
    scaffold.init(&dirs, &tmp)
}

fn resolve_path(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<String, ConfigError> {
    let parsed = Key::parse(&format!("paths.{key}"))?;
    Ok(config.effective(&parsed, None)?.rendered())
}

#[cfg(test)]
mod tests {
    use config::{catalogue, render_value};

    /// The `(key, dir)` pairs `init.sh`'s `DIR_DEFAULTS` array hard-coded,
    /// captured so the catalogue coincidence is pinned once the shell script is
    /// gone.
    const LEGACY_DIR_DEFAULTS: &[(&str, &str)] = &[
        ("plans", "meta/plans"),
        ("research_codebase", "meta/research/codebase"),
        ("decisions", "meta/decisions"),
        ("prs", "meta/prs"),
        ("validations", "meta/validations"),
        ("review_plans", "meta/reviews/plans"),
        ("review_prs", "meta/reviews/prs"),
        ("review_work", "meta/reviews/work"),
        ("work", "meta/work"),
        ("notes", "meta/notes"),
        (
            "research_design_inventories",
            "meta/research/design-inventories",
        ),
        ("research_design_gaps", "meta/research/design-gaps"),
        ("global", "meta/global"),
        ("research_issues", "meta/research/issues"),
    ];

    #[test]
    fn each_dir_default_matches_the_catalogue() {
        for (key, expected) in LEGACY_DIR_DEFAULTS {
            let full = format!("paths.{key}");
            let rendered =
                catalogue::default_for(&full).map(|value| render_value(&value));
            assert_eq!(rendered.as_deref(), Some(*expected), "paths.{key}");
        }
    }
}
