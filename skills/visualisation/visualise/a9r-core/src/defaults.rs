//! The path-key defaults table.
//!
//! Ports `PATH_KEYS` / `PATH_DEFAULTS` from
//! [`scripts/config-defaults.sh`](../../../../scripts/config-defaults.sh).
//! Keys here are the *bare* keys (the bash table stores them
//! `paths.`-prefixed; `config-read-path` prepends `paths.` to the user key
//! before matching, so matching on the bare key is equivalent).

/// `(bare_key, default_path)` rows, in the same order as the bash arrays.
/// Every default is non-empty, which is load-bearing: `config-read-path`
/// only emits the "unknown key" warning when the resolved table default is
/// empty (i.e. the key was *not* in this table).
pub const PATH_DEFAULTS: &[(&str, &str)] = &[
    ("plans", "meta/plans"),
    ("research_codebase", "meta/research/codebase"),
    ("decisions", "meta/decisions"),
    ("prs", "meta/prs"),
    ("validations", "meta/validations"),
    ("review_plans", "meta/reviews/plans"),
    ("review_prs", "meta/reviews/prs"),
    ("review_work", "meta/reviews/work"),
    ("templates", ".accelerator/templates"),
    ("work", "meta/work"),
    ("notes", "meta/notes"),
    ("tmp", ".accelerator/tmp"),
    ("integrations", ".accelerator/state/integrations"),
    (
        "research_design_inventories",
        "meta/research/design-inventories",
    ),
    ("research_design_gaps", "meta/research/design-gaps"),
    ("global", "meta/global"),
    ("research_issues", "meta/research/issues"),
];

/// The plugin-standard default for a bare path key, or `None` if the key is
/// not in the table.
pub fn path_default(key: &str) -> Option<&'static str> {
    PATH_DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_returns_table_default() {
        assert_eq!(path_default("plans"), Some("meta/plans"));
        assert_eq!(path_default("templates"), Some(".accelerator/templates"));
        assert_eq!(
            path_default("research_design_inventories"),
            Some("meta/research/design-inventories"),
        );
    }

    #[test]
    fn unknown_key_returns_none() {
        assert_eq!(path_default("nope"), None);
        // The legacy bare keys were renamed; they are NOT in the table.
        assert_eq!(path_default("design_inventories"), None);
        assert_eq!(path_default("design_gaps"), None);
    }

    #[test]
    fn table_has_the_seventeen_rows_and_no_empty_defaults() {
        assert_eq!(PATH_DEFAULTS.len(), 17);
        assert!(PATH_DEFAULTS
            .iter()
            .all(|(k, v)| !k.is_empty() && !v.is_empty()));
    }
}
