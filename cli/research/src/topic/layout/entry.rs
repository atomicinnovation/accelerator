//! Which names directly in a set's `findings/` and `reports/` a reader takes
//! as a finding or a report.
//!
//! A reader admits any visible numbered Markdown name;
//! `finding_path::is_finding_path` is the stricter rule for the stems a
//! writer may create.

use std::path::Path;

use crate::topic::layout::stem::Stem;

#[must_use]
pub fn is_finding_entry(name: &str) -> bool {
    is_visible_markdown(name) && Stem::index_of(name).is_some()
}

/// The stem whose `.levels/` tree a finding entry was composed from, when
/// its name is one the planner could have allocated.
#[must_use]
pub fn finding_stem(name: &str) -> Option<Stem> {
    if !is_finding_entry(name) {
        return None;
    }
    let (stem, _) = name.rsplit_once('.')?;
    Stem::parse(stem)
}

#[must_use]
pub fn is_report_entry(name: &str) -> bool {
    is_visible_markdown(name)
}

fn is_visible_markdown(name: &str) -> bool {
    !name.starts_with('.')
        && Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
mod tests {
    use super::finding_stem;
    use super::is_finding_entry;
    use super::is_report_entry;
    use crate::topic::layout::stem::Stem;

    #[test]
    fn a_numbered_visible_markdown_entry_is_a_finding() {
        assert!(is_finding_entry("01-a-web.md"));
        assert!(is_finding_entry("07-a-web.MD"));
    }

    #[test]
    fn an_unnumbered_or_hidden_entry_is_no_finding() {
        for name in ["notes.md", ".03-c-web.md.invalid", "01-a-web.levels"] {
            assert!(!is_finding_entry(name), "{name:?}");
        }
    }

    #[test]
    fn a_visible_markdown_entry_is_a_report() {
        assert!(is_report_entry("a.md"));
        assert!(!is_report_entry(".c.md"));
        assert!(!is_report_entry("notes.txt"));
    }

    #[test]
    fn a_finding_entry_names_its_stem_whatever_its_extension_s_case() {
        for name in ["07-a-web.md", "07-a-web.MD"] {
            assert_eq!(finding_stem(name), Stem::parse("07-a-web"), "{name:?}");
        }
    }

    #[test]
    fn an_entry_outside_the_stem_alphabet_names_no_stem() {
        for name in ["notes.md", "01-A-Web.md", ".01-a-web.md", "01-a-web"] {
            assert_eq!(finding_stem(name), None, "{name:?}");
        }
    }
}
