//! The address of one node of one pair's research tree, and where its level
//! note lives.

use std::fmt;

use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::stem::Stem;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteRef {
    pub stem: Stem,
    pub lineage: Lineage,
}

impl NoteRef {
    /// The set-relative path of the level note at this address.
    #[must_use]
    pub fn path(&self) -> String {
        format!("findings/{}.levels/{}.md", self.stem, self.lineage)
    }

    /// Scoped by `set_slug`, because stems repeat across sets and level-note
    /// ids must be unique across the corpus.
    #[must_use]
    pub fn id(&self, set_slug: &str) -> String {
        format!("{set_slug}.{}.{}", self.stem, self.lineage)
    }
}

impl fmt::Display for NoteRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.stem, self.lineage)
    }
}

#[cfg(test)]
mod tests {
    use super::NoteRef;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::stem::Stem;

    fn note_ref(stem: &str, lineage: &str) -> Option<NoteRef> {
        Some(NoteRef {
            stem: Stem::parse(stem)?,
            lineage: Lineage::parse(lineage)?,
        })
    }

    #[test]
    fn a_notes_path_and_id_share_its_stem_and_lineage() {
        let at = note_ref("01-a-web", "3-2-1");
        assert_eq!(
            at.as_ref().map(NoteRef::path).as_deref(),
            Some("findings/01-a-web.levels/3-2-1.md")
        );
        assert_eq!(
            at.map(|at| at.id("attention")).as_deref(),
            Some("attention.01-a-web.3-2-1")
        );
    }

    #[test]
    fn a_note_ref_reads_as_its_stem_and_lineage() {
        assert_eq!(
            note_ref("01-a-web", "2-1")
                .map(|at| at.to_string())
                .as_deref(),
            Some("01-a-web:2-1")
        );
    }
}
