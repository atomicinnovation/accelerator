//! Which paths below the research topics directory name a finding.

/// Whether `relative`, a `/`-separated path below the topics directory, is
/// exactly `<set>/findings/<name>.md` with no leading dot on the set or name.
pub fn is_finding_path(relative: &str) -> bool {
    let segments: Vec<&str> = relative.split('/').collect();
    let [set, "findings", name] = segments.as_slice() else {
        return false;
    };
    is_visible(set)
        && is_visible(name)
        && name
            .strip_suffix(".md")
            .is_some_and(|stem| !stem.is_empty())
}

fn is_visible(segment: &str) -> bool {
    !segment.is_empty() && !segment.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::is_finding_path;

    #[test]
    fn a_markdown_file_directly_in_a_sets_findings_is_a_finding() {
        assert!(is_finding_path("s/findings/01-x-web.md"));
        assert!(is_finding_path("2026-09-24-attention/findings/x.md"));
    }

    #[test]
    fn findings_must_be_the_immediate_parent() {
        assert!(!is_finding_path("findings/x.md"));
        assert!(!is_finding_path("s/findings/sub/x.md"));
        assert!(!is_finding_path("s/sub/findings/x.md"));
        assert!(!is_finding_path("s/brief.md"));
        assert!(!is_finding_path("s/notfindings/x.md"));
    }

    #[test]
    fn a_leading_dot_is_refused() {
        assert!(!is_finding_path("s/findings/.01-x-web.md.invalid"));
        assert!(!is_finding_path("s/findings/.x.md"));
        assert!(!is_finding_path(".s/findings/x.md"));
    }

    #[test]
    fn only_markdown_names_are_findings() {
        assert!(!is_finding_path("s/findings/x.txt"));
        assert!(!is_finding_path("s/findings/x.md.bak"));
        assert!(!is_finding_path("s/findings/x"));
    }

    #[test]
    fn empty_segments_are_refused() {
        assert!(!is_finding_path("/findings/x.md"));
        assert!(!is_finding_path("s//findings/x.md"));
        assert!(!is_finding_path("s/findings/"));
        assert!(!is_finding_path(""));
    }
}
