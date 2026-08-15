//! Whether every substantive H2 section of a design-gap document carries a
//! cue phrase.
//!
//! The patterns are extended regular expressions, and this crate may not
//! depend on a regex engine, so the compiled matcher is injected. The pattern
//! slice stays here as the source the adapter compiles, and a drift test pins
//! it against the on-disk file that documents itself as canonical for both
//! this audit and `extract-work-items`.

/// The canonical cue-phrase patterns, one ERE alternative per entry.
///
/// The first three are matched case-insensitively. The fourth is matched
/// case-sensitively, so `implement Foo` (a proper-noun feature name) matches
/// while `implement foo` does not.
pub const CUE_PHRASE_PATTERNS: [&str; 3] =
    ["we need", "users? need", "the system must"];

/// The one pattern whose case carries meaning.
pub const CASE_SENSITIVE_CUE_PHRASE_PATTERN: &str = "[Ii]mplement [A-Z]";

/// The compiled-regex seam.
///
/// The domain states which patterns matter; the adapter owns compiling and
/// running them.
pub trait CuePhraseMatcher {
    /// Whether `text` carries any cue phrase.
    fn matches(&self, text: &str) -> bool;
}

/// An H2 section whose prose carries no cue phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncuedSection {
    pub name: String,
}

/// Splits `document` into H2 sections and reports every substantive one
/// carrying no cue phrase.
///
/// A section holding only whitespace is skipped: there is no prose to cue. An
/// H1 closes the section above it without opening one of its own.
#[must_use]
pub fn audit(
    document: &str,
    matcher: &dyn CuePhraseMatcher,
) -> Vec<UncuedSection> {
    sections(document)
        .into_iter()
        .filter(|(_, body)| !body.trim().is_empty())
        .filter(|(_, body)| !matcher.matches(body))
        .map(|(name, _)| UncuedSection { name })
        .collect()
}

fn sections(document: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut open: Option<(String, String)> = None;

    for line in document.lines() {
        if let Some(name) = line.strip_prefix("## ") {
            sections.extend(open.take());
            open = Some((name.to_owned(), String::new()));
        } else if line.starts_with("# ") {
            sections.extend(open.take());
        } else if let Some((_, body)) = open.as_mut() {
            body.push('\n');
            body.push_str(line);
        }
    }
    sections.extend(open);
    sections
}

#[cfg(test)]
mod tests {
    use super::audit;
    use super::CuePhraseMatcher;

    /// Stands in for the compiled ERE, matching the same phrases by the
    /// simplest means that cannot itself be the thing under test.
    struct Phrases;

    impl CuePhraseMatcher for Phrases {
        fn matches(&self, text: &str) -> bool {
            let lowered = text.to_lowercase();
            ["we need", "user need", "users need", "the system must"]
                .iter()
                .any(|phrase| lowered.contains(phrase))
                || text.split("implement ").skip(1).any(|rest| {
                    rest.chars().next().is_some_and(char::is_uppercase)
                })
        }
    }

    struct NeverMatches;

    impl CuePhraseMatcher for NeverMatches {
        fn matches(&self, _text: &str) -> bool {
            false
        }
    }

    fn uncued(document: &str) -> Vec<String> {
        audit(document, &Phrases)
            .into_iter()
            .map(|section| section.name)
            .collect()
    }

    #[test]
    fn a_fully_cued_document_reports_nothing() {
        let document = "# Title\n\n## Alpha\n\nWe need a thing.\n\n\
                        ## Beta\n\nThe system must do it.\n";
        assert!(uncued(document).is_empty());
    }

    #[test]
    fn an_uncued_section_is_named() {
        let document = "## Alpha\n\nWe need a thing.\n\n\
                        ## Beta\n\nJust some prose.\n";
        assert_eq!(uncued(document), vec!["Beta"]);
    }

    #[test]
    fn every_uncued_section_is_named_not_only_the_first() {
        let document = "## Alpha\n\nprose\n\n## Beta\n\nmore prose\n";
        assert_eq!(uncued(document), vec!["Alpha", "Beta"]);
    }

    #[test]
    fn a_whitespace_only_section_has_no_prose_to_cue() {
        let document = "## Empty\n\n   \n\n## Beta\n\nWe need a thing.\n";
        assert!(uncued(document).is_empty());
    }

    #[test]
    fn an_h1_closes_the_section_above_it_without_opening_one() {
        let document = "## Alpha\n\nprose\n\n# New Title\n\nloose prose\n\n\
             ## Beta\n\nWe need a thing.\n";
        assert_eq!(uncued(document), vec!["Alpha"]);
    }

    #[test]
    fn the_last_section_is_audited_too() {
        let document = "## Alpha\n\nWe need a thing.\n\n## Omega\n\nprose\n";
        assert_eq!(uncued(document), vec!["Omega"]);
    }

    #[test]
    fn a_document_with_no_h2_sections_reports_nothing() {
        assert!(audit("# Title\n\njust prose\n", &NeverMatches).is_empty());
    }

    #[test]
    fn the_matcher_is_what_decides_not_the_sectioning() {
        let document = "## Alpha\n\nWe need a thing.\n";
        assert_eq!(
            audit(document, &NeverMatches)
                .into_iter()
                .map(|section| section.name)
                .collect::<Vec<_>>(),
            vec!["Alpha"]
        );
    }
}
