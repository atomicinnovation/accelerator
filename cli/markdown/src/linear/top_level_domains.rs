use std::collections::HashSet;
use std::sync::LazyLock;

const IANA_ROOT_ZONE: &str = include_str!("tlds-alpha-by-domain.txt");

static TOP_LEVEL_DOMAINS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| {
        IANA_ROOT_ZONE
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect()
    });

/// Whether `label` is a top-level domain in the pinned IANA root zone
/// snapshot, ignoring ASCII case.
#[must_use]
pub fn is_top_level_domain(label: &str) -> bool {
    TOP_LEVEL_DOMAINS.contains(label.to_ascii_uppercase().as_str())
}
