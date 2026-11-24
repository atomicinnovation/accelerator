use super::is_top_level_domain;

const IANA_ROOT_ZONE: &str = include_str!("tlds-alpha-by-domain.txt");

#[test]
fn the_snapshot_is_iana_shaped() {
    let mut lines = IANA_ROOT_ZONE.lines();
    let version = lines.next().unwrap_or_default();
    assert!(version.starts_with("# Version"), "{version}");
    let labels: Vec<&str> = lines.collect();
    for label in &labels {
        assert!(
            !label.is_empty()
                && label.bytes().all(|byte| byte.is_ascii_uppercase()
                    || byte.is_ascii_digit()
                    || byte == b'-'),
            "{label:?}"
        );
    }
    assert!(labels.len() >= 1_400, "{} labels", labels.len());
}

#[test]
fn top_level_domains_ignore_case() {
    for label in ["io", "IO", "rs", "md", "pid"] {
        assert!(is_top_level_domain(label), "{label}");
    }
    for label in ["toml", "yaml", "g", "3"] {
        assert!(!is_top_level_domain(label), "{label}");
    }
}
