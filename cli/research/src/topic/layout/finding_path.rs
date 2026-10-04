//! Which paths below the research topics directory name a finding or a
//! level note.

use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::stem::Stem;

/// Whether `relative`, a `/`-separated path below the topics directory, is
/// exactly `<set>/findings/<stem>.md` with a visible set.
pub fn is_finding_path(relative: &str) -> bool {
    let segments: Vec<&str> = relative.split('/').collect();
    let [set, "findings", name] = segments.as_slice() else {
        return false;
    };
    is_visible(set) && name.strip_suffix(".md").is_some_and(is_stem)
}

/// Whether `relative` is exactly
/// `<set>/findings/<stem>.levels/<lineage>.md` with a visible set.
pub fn is_level_note_path(relative: &str) -> bool {
    let segments: Vec<&str> = relative.split('/').collect();
    let [set, "findings", levels, name] = segments.as_slice() else {
        return false;
    };
    is_visible(set)
        && levels.strip_suffix(".levels").is_some_and(is_stem)
        && name
            .strip_suffix(".md")
            .is_some_and(|lineage| Lineage::parse(lineage).is_some())
}

fn is_stem(text: &str) -> bool {
    Stem::parse(text).is_some()
}

fn is_visible(segment: &str) -> bool {
    !segment.is_empty() && !segment.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::is_finding_path;
    use super::is_level_note_path;

    #[test]
    fn a_markdown_file_directly_in_a_sets_findings_is_a_finding() {
        assert!(is_finding_path("s/findings/01-x-web.md"));
        assert!(is_finding_path("2026-09-24-attention/findings/01-x.md"));
        assert!(!is_finding_path("s/findings/x.md"));
    }

    #[test]
    fn a_finding_stem_carries_its_index() {
        assert!(is_finding_path("s/findings/3-x-web.md"));
        assert!(is_finding_path("s/findings/100-x-web.md"));
        assert!(!is_finding_path("s/findings/x-web.md"));
        assert!(!is_finding_path("s/findings/-x-web.md"));
    }

    #[test]
    fn a_finding_stem_stays_inside_the_allocated_alphabet() {
        for name in ["03-A.md", "03-a b.md", "03-a\nb.md", "03-a:1.md"] {
            assert!(
                !is_finding_path(&format!("s/findings/{name}")),
                "{name:?}"
            );
        }
    }

    #[test]
    fn every_level_note_path_the_work_item_names_gets_its_verdict() {
        let levels = "s/findings/03-a-web.levels";
        for note in ["1.md", "3-2-1.md", "2-10.md"] {
            assert!(is_level_note_path(&format!("{levels}/{note}")), "{note}");
        }
        for note in [
            "2.md",
            "2-1-1.md",
            "2-0.md",
            "2-a.md",
            "0.md",
            ".2-1.md.invalid",
            "2-1.txt",
            "2-1/x.md",
            "1.levels/1.md",
            ".md",
            "",
        ] {
            assert!(
                !is_level_note_path(&format!("{levels}/{note}")),
                "{note:?}"
            );
        }
        for path in [
            "s/findings/a-web.levels/1.md",
            "s/findings/03-A.levels/1.md",
            "s/findings/03-a b.levels/1.md",
            "s/findings/03-a-web/1.md",
            "s/findings/.03-a-web.levels/1.md",
            ".s/findings/03-a-web.levels/1.md",
            "/findings/03-a-web.levels/1.md",
            "s/notfindings/03-a-web.levels/1.md",
            "findings/03-a-web.levels/1.md",
        ] {
            assert!(!is_level_note_path(path), "{path:?}");
        }
    }

    #[test]
    fn a_level_note_is_never_a_finding_and_a_finding_is_never_a_level_note() {
        let note = "s/findings/03-a-web.levels/1.md";
        let finding = "s/findings/03-a-web.md";
        assert!(is_level_note_path(note) && !is_finding_path(note));
        assert!(is_finding_path(finding) && !is_level_note_path(finding));
    }

    #[test]
    fn findings_must_be_the_immediate_parent() {
        assert!(!is_finding_path("findings/01-x.md"));
        assert!(!is_finding_path("s/findings/sub/01-x.md"));
        assert!(!is_finding_path("s/sub/findings/01-x.md"));
        assert!(!is_finding_path("s/brief.md"));
        assert!(!is_finding_path("s/notfindings/01-x.md"));
    }

    #[test]
    fn a_leading_dot_is_refused() {
        assert!(!is_finding_path("s/findings/.01-x-web.md.invalid"));
        assert!(!is_finding_path("s/findings/.01-x.md"));
        assert!(!is_finding_path(".s/findings/01-x.md"));
    }

    #[test]
    fn only_markdown_names_are_findings() {
        assert!(!is_finding_path("s/findings/01-x.txt"));
        assert!(!is_finding_path("s/findings/01-x.md.bak"));
        assert!(!is_finding_path("s/findings/01-x"));
    }

    #[test]
    fn empty_segments_are_refused() {
        assert!(!is_finding_path("/findings/01-x.md"));
        assert!(!is_finding_path("s//findings/01-x.md"));
        assert!(!is_finding_path("s/findings/"));
        assert!(!is_finding_path(""));
    }
}
