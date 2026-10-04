//! Which agent one `conduct` spawn is.

use std::fmt;

use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;

/// A pair's single-pass researcher or composer, or the researcher of one
/// node in a pair's tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpawnRef {
    Pair(Stem),
    Node(NoteRef),
}

impl SpawnRef {
    /// Reads `<stem>` or `<stem>:<lineage>`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.split_once(':') {
            Some((stem, lineage)) => Some(Self::Node(NoteRef {
                stem: Stem::parse(stem)?,
                lineage: Lineage::parse(lineage)?,
            })),
            None => Stem::parse(text).map(Self::Pair),
        }
    }
}

impl fmt::Display for SpawnRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pair(stem) => write!(f, "{stem}"),
            Self::Node(at) => write!(f, "{at}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpawnRef;

    #[test]
    fn a_spawn_ref_round_trips_for_a_pair_and_a_node() {
        for text in ["03-a-web", "03-a-web:1", "03-a-web:3-2-1"] {
            assert_eq!(
                SpawnRef::parse(text).map(|s| s.to_string()).as_deref(),
                Some(text)
            );
        }
        assert!(matches!(
            SpawnRef::parse("03-a-web"),
            Some(SpawnRef::Pair(_))
        ));
        assert!(matches!(
            SpawnRef::parse("03-a-web:2-1"),
            Some(SpawnRef::Node(_))
        ));
    }

    #[test]
    fn a_spawn_ref_is_refused_when_malformed() {
        for text in ["03-a:2-0", ":1", "03-A", "", "03-a:", "03-a:1:1"] {
            assert_eq!(SpawnRef::parse(text), None, "{text:?}");
        }
    }
}
