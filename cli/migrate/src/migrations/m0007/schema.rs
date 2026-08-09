//! The cross-cutting facts `scripts/templates-schema.tsv` alone does not
//! carry: the own-identity key per type (distinct from a type's *forbidden*
//! own-id keys — the rename this drives always pre-empts the drop rule from
//! ever seeing the same key), the legacy type-alias fold, and the
//! typed-linkage key vocabulary/cardinality.

/// The legacy own-identity key a type renames into `id:` — `work-item`/`adr`
/// only. Distinct from
/// [`corpus::frontmatter_validation::schema::SchemaRow::forbidden_own_id_keys`]:
/// the rename here always runs first, so the drop rule never actually sees
/// `work_item_id`/`adr_id` on a work-item/adr file.
pub fn own_id_key_for_type(linkage_type: &str) -> Option<&'static str> {
    match linkage_type {
        "work-item" => Some("work_item_id"),
        "adr" => Some("adr_id"),
        _ => None,
    }
}

/// Legacy artifact-type alias folded to its canonical type. Work-item
/// kind-values (story/bug/…) are not handled here — migration 0005 owns
/// those.
pub fn canonical_type(linkage_type: &str) -> &str {
    if linkage_type == "validation" {
        "plan-validation"
    } else {
        linkage_type
    }
}

/// A typed-linkage key's cardinality — `parent`/`target`/`source`/
/// `superseded_by` are single-valued; every other vocabulary key is a list.
/// `None` for a key outside the nine-key vocabulary.
pub fn linkage_cardinality(key: &str) -> Option<Cardinality> {
    match key {
        "parent" | "target" | "source" | "superseded_by" => {
            Some(Cardinality::Single)
        }
        "supersedes" | "blocks" | "blocked_by" | "derived_from"
        | "relates_to" => Some(Cardinality::List),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    Single,
    List,
}

/// Whether `key` is a typed-linkage key (any cardinality) — the value
/// normalisation rule's own dispatch gate.
pub fn is_linkage_key(key: &str) -> bool {
    linkage_cardinality(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_type, is_linkage_key, linkage_cardinality,
        own_id_key_for_type, Cardinality,
    };

    #[test]
    fn work_item_and_adr_have_own_id_keys() {
        assert_eq!(own_id_key_for_type("work-item"), Some("work_item_id"));
        assert_eq!(own_id_key_for_type("adr"), Some("adr_id"));
        assert_eq!(own_id_key_for_type("plan"), None);
    }

    #[test]
    fn validation_folds_to_plan_validation() {
        assert_eq!(canonical_type("validation"), "plan-validation");
        assert_eq!(canonical_type("plan"), "plan");
    }

    #[test]
    fn single_valued_keys_are_recognised() {
        for key in ["parent", "target", "source", "superseded_by"] {
            assert_eq!(linkage_cardinality(key), Some(Cardinality::Single));
        }
    }

    #[test]
    fn list_valued_keys_are_recognised() {
        for key in [
            "supersedes",
            "blocks",
            "blocked_by",
            "derived_from",
            "relates_to",
        ] {
            assert_eq!(linkage_cardinality(key), Some(Cardinality::List));
        }
    }

    #[test]
    fn an_unknown_key_has_no_cardinality() {
        assert_eq!(linkage_cardinality("bogus"), None);
        assert!(!is_linkage_key("bogus"));
    }
}
