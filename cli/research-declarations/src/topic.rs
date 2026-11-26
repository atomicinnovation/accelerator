//! How a topic research set names its documents and directories on disk.

pub const BRIEF: &str = "brief.md";
pub const OUTLINE: &str = "outline.md";
pub const MANIFEST: &str = "manifest.md";
pub const SYNTHESIS: &str = "synthesis.md";
pub const FINDINGS: &str = "findings";
pub const REPORTS: &str = "reports";
pub const LEVELS_SUFFIX: &str = ".levels";

#[cfg(test)]
mod tests {
    use crate::topic::BRIEF;
    use crate::topic::FINDINGS;
    use crate::topic::LEVELS_SUFFIX;
    use crate::topic::MANIFEST;
    use crate::topic::OUTLINE;
    use crate::topic::REPORTS;
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
}
