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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfferedNodes(BTreeMap<NoteRef, NodeContext>);

impl OfferedNodes {
    #[must_use]
    pub fn of(
        contexts: impl IntoIterator<Item = (NoteRef, NodeContext)>,
    ) -> Self {
        Self(contexts.into_iter().collect())
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// A context belongs only to a node spawn among `spawns`.
    #[must_use]
    pub fn retained_for(mut self, spawns: &[SpawnRef]) -> Self {
        self.0
            .retain(|at, _| spawns.contains(&SpawnRef::Node(at.clone())));
        self
    }

    #[must_use]
    pub fn context(&self, at: &NoteRef) -> Option<&NodeContext> {
        self.0.get(at)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NoteRef, &NodeContext)> {
        self.0.iter()
    }
}
