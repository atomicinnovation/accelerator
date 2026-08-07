//! The typed-linkage inner-value shape check. Hand-rolled (no `regex`),
//! matching `corpus::linkage`'s dependency discipline.
//!
//! Mirrors `FM_TYPED_REF_RE`, `^(${FM_SOURCE_TYPE_RE}):[A-Za-z0-9.-]+$` —
//! **not** the same grammar as `corpus::typed_ref::parse_typed_ref` (which
//! recognises only four prefixes and special-cases path-shaped values).

use crate::frontmatter_validation::schema::LINKAGE_SOURCE_TYPES;

/// Whether `inner` is a well-formed `<type>:<id>` typed-linkage reference.
///
/// `inner` is the value already stripped of its surrounding quotes;
/// `<type>` must be in the vocabulary, `<id>` one or more
/// `[A-Za-z0-9.-]` characters.
#[must_use]
pub fn is_well_formed(inner: &str) -> bool {
    let Some((source_type, id)) = inner.split_once(':') else {
        return false;
    };
    if !LINKAGE_SOURCE_TYPES.contains(&source_type) {
        return false;
    }
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

#[cfg(test)]
mod tests {
    use super::is_well_formed;

    #[test]
    fn a_known_type_with_a_simple_id_is_well_formed() {
        assert!(is_well_formed("work-item:0042"));
        assert!(is_well_formed("adr:ADR-0042"));
        assert!(is_well_formed("pr:42"));
    }

    #[test]
    fn a_dotted_version_numbered_id_is_well_formed() {
        assert!(is_well_formed("plan:2026-01-01-changelog-1.21.0-cleanup"));
    }

    #[test]
    fn an_unknown_type_is_rejected() {
        assert!(!is_well_formed("bogus-type:0042"));
    }

    #[test]
    fn a_bare_number_with_no_type_prefix_is_rejected() {
        assert!(!is_well_formed("0042"));
    }

    #[test]
    fn a_path_shaped_value_is_rejected() {
        assert!(!is_well_formed("meta/work/0042-x.md"));
    }

    #[test]
    fn an_empty_id_is_rejected() {
        assert!(!is_well_formed("work-item:"));
    }

    #[test]
    fn an_id_carrying_a_hash_is_rejected() {
        assert!(!is_well_formed("work-item:0042#anchor"));
    }
}
