//! A legacy-to-canonical status mapping, by type. Only `plan` and
//! `plan-review` carry any rows.

/// The canonical status for a legacy `(type, status)` pair, or `None` when
/// unmapped.
pub fn map_status(linkage_type: &str, legacy: &str) -> Option<&'static str> {
    match (linkage_type, legacy) {
        (
            "plan",
            "accepted" | "complete" | "implemented" | "final" | "revised",
        ) => Some("done"),
        ("plan", "approved" | "reviewed") => Some("ready"),
        ("plan-review", "accepted") => Some("complete"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::map_status;

    #[test]
    fn plan_legacy_statuses_map_to_done_or_ready() {
        assert_eq!(map_status("plan", "accepted"), Some("done"));
        assert_eq!(map_status("plan", "complete"), Some("done"));
        assert_eq!(map_status("plan", "implemented"), Some("done"));
        assert_eq!(map_status("plan", "final"), Some("done"));
        assert_eq!(map_status("plan", "revised"), Some("done"));
        assert_eq!(map_status("plan", "approved"), Some("ready"));
        assert_eq!(map_status("plan", "reviewed"), Some("ready"));
    }

    #[test]
    fn plan_review_accepted_maps_to_complete() {
        assert_eq!(map_status("plan-review", "accepted"), Some("complete"));
    }

    #[test]
    fn an_unmapped_pair_is_none() {
        assert_eq!(map_status("plan", "bogus"), None);
        assert_eq!(map_status("work-item", "accepted"), None);
    }
}
