//! Own-identity predicate and the `id`/`work_item_id` alias fallback,
//! operating on already-extracted values.

/// True iff either own-identity field carries a non-empty value.
#[must_use]
pub fn is_work_item_file(id: Option<&str>, work_item_id: Option<&str>) -> bool {
    id.is_some_and(|v| !v.is_empty())
        || work_item_id.is_some_and(|v| !v.is_empty())
}

/// The alternate own-identity field name to fall back to, or `None` when
/// `field` is not an own-identity field at all.
#[must_use]
pub fn own_identity_alias(field: &str) -> Option<&'static str> {
    match field {
        "id" => Some("work_item_id"),
        "work_item_id" => Some("id"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_work_item_file, own_identity_alias};

    #[test]
    fn is_work_item_file_true_when_either_field_present() {
        assert!(is_work_item_file(Some("work-item:0042"), None));
        assert!(is_work_item_file(None, Some("0042")));
        assert!(!is_work_item_file(None, None));
        assert!(!is_work_item_file(Some(""), Some("")));
    }

    #[test]
    fn own_identity_alias_maps_both_directions() {
        assert_eq!(own_identity_alias("id"), Some("work_item_id"));
        assert_eq!(own_identity_alias("work_item_id"), Some("id"));
        assert_eq!(own_identity_alias("status"), None);
    }
}
