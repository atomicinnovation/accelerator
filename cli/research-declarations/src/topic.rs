//! How a topic research set names its documents and directories on disk.

use corpus::set::MemberKind;
use corpus::set::MemberSlot;
use corpus::set::Presentation;
use corpus::set::SetDeclaration;
use corpus::set::SetShape;

pub const BRIEF: &str = "brief.md";
pub const OUTLINE: &str = "outline.md";
pub const MANIFEST: &str = "manifest.md";
pub const SYNTHESIS: &str = "synthesis.md";
pub const FINDINGS: &str = "findings";
pub const REPORTS: &str = "reports";
pub const LEVELS_SUFFIX: &str = ".levels";

// `const` string concatenation cannot join `FINDINGS`, `LEVELS_SUFFIX` or
// `REPORTS` into a wildcard slot, so those slots are spelled out and
// `research`'s path builders are tested against them.
pub static SET: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: MANIFEST,
        member_slots: &[
            document(BRIEF),
            document(OUTLINE),
            document(SYNTHESIS),
            document("findings/*.md"),
            document("findings/*.levels/*.md"),
            document("reports/*.md"),
        ],
    },
    presentation: Presentation::SetPage,
};

const fn document(pattern: &'static str) -> MemberSlot {
    MemberSlot {
        pattern,
        kind: MemberKind::Document,
    }
}

#[cfg(test)]
mod tests {
    use corpus::set::MemberKind;
    use corpus::set::Presentation;
    use corpus::set::SetMatcher;
    use corpus::set::SetPlace;
    use corpus::set::SlotPatternError;

    use crate::topic::BRIEF;
    use crate::topic::FINDINGS;
    use crate::topic::LEVELS_SUFFIX;
    use crate::topic::MANIFEST;
    use crate::topic::OUTLINE;
    use crate::topic::REPORTS;
    use crate::topic::SET;
    use crate::topic::SYNTHESIS;

    #[test]
    fn a_topic_set_names_its_documents_and_directories() {
        assert_eq!(
            [
                MANIFEST,
                BRIEF,
                OUTLINE,
                SYNTHESIS,
                FINDINGS,
                REPORTS,
                LEVELS_SUFFIX
            ],
            [
                "manifest.md",
                "brief.md",
                "outline.md",
                "synthesis.md",
                "findings",
                "reports",
                ".levels"
            ]
        );
    }

    type Outcome = Result<(), (&'static str, SlotPatternError)>;

    const DOCUMENT: Option<SetPlace> =
        Some(SetPlace::Member(MemberKind::Document));

    #[test]
    fn a_topic_set_holds_its_manifest_and_its_findings_notes() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        assert_eq!(set.classify(MANIFEST), Some(SetPlace::Root));
        assert_eq!(set.classify("findings/01-a.levels/2-1.md"), DOCUMENT);
        Ok(())
    }

    #[test]
    fn a_topic_set_holds_no_working_or_unslotted_file() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        for path in [
            "findings/sub/x.md",
            "notes.txt",
            "findings/.01-x-web.md.invalid",
            ".conduct-run.json",
        ] {
            assert_eq!(set.classify(path), None, "{path}");
        }
        Ok(())
    }

    #[test]
    fn a_topic_set_matches_only_extensions_in_any_case() -> Outcome {
        let set = SetMatcher::new(&SET)?;
        assert_eq!(set.classify("findings/01-a.MD"), DOCUMENT);
        assert_eq!(set.classify("Findings/01-a.md"), None);
        assert_eq!(set.classify("findings/01-a.LEVELS/2-1.md"), None);
        Ok(())
    }

    #[test]
    fn a_topic_set_is_presented_as_a_set_page() {
        assert_eq!(SET.presentation, Presentation::SetPage);
    }
}
