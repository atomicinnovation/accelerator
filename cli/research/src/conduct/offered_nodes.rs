//! What each node a `conduct` run offered was told when it was first
//! offered, kept so its researcher reads that and not a later tree.

use std::collections::BTreeMap;

use crate::conduct::spawn::SpawnRef;
use crate::topic::layout::note_ref::NoteRef;

/// The context of one offered node that its spawn ref cannot derive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeContext {
    pub profile: String,
    pub question: String,
    pub known_questions: Vec<String>,
}

/// The contexts of a batch's pending nodes, or none at all when the stored
/// snapshot could not be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfferedNodes {
    contexts: BTreeMap<NoteRef, NodeContext>,
    unreadable: bool,
}

impl OfferedNodes {
    #[must_use]
    pub fn of(
        contexts: impl IntoIterator<Item = (NoteRef, NodeContext)>,
    ) -> Self {
        Self {
            contexts: contexts.into_iter().collect(),
            unreadable: false,
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            contexts: BTreeMap::new(),
            unreadable: false,
        }
    }

    #[must_use]
    pub const fn unreadable() -> Self {
        Self {
            contexts: BTreeMap::new(),
            unreadable: true,
        }
    }

    #[must_use]
    pub const fn is_unreadable(&self) -> bool {
        self.unreadable
    }

    #[must_use]
    pub fn retained_for(mut self, spawns: &[SpawnRef]) -> Self {
        self.contexts
            .retain(|at, _| spawns.contains(&SpawnRef::Node(at.clone())));
        self
    }

    #[must_use]
    pub fn context(&self, at: &NoteRef) -> Option<&NodeContext> {
        self.contexts.get(at)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NoteRef, &NodeContext)> {
        self.contexts.iter()
    }
}
