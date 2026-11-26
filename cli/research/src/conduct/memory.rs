//! What a `conduct` run remembers between plans.
//!
//! That is the batch it last offered, the spawns it attempted, what it has
//! seen the set show, the findings it still owes quarantining, and the
//! profiles whose researchers it withholds.

use std::collections::BTreeSet;

use crate::conduct::observed::Observed;
use crate::conduct::offered_nodes::NodeContext;
use crate::conduct::offered_nodes::OfferedNodes;
use crate::conduct::spawn::SpawnRef;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    spawns: Vec<SpawnRef>,
    nodes: OfferedNodes,
}

impl Offer {
    #[must_use]
    pub fn new(spawns: Vec<SpawnRef>, nodes: OfferedNodes) -> Self {
        let nodes = nodes.retained_for(&spawns);
        Self { spawns, nodes }
    }
}

/// The spawns last offered and not yet acknowledged, under the number the
/// acknowledgement must quote, with the context each pending node was first
/// offered with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingBatch {
    number: u32,
    spawns: Vec<SpawnRef>,
    nodes: OfferedNodes,
}

impl PendingBatch {
    /// A pending node may lack a context, as in a batch offered before
    /// contexts were kept.
    #[must_use]
    pub fn restored(number: u32, offer: Offer) -> Self {
        Self {
            number,
            spawns: offer.spawns,
            nodes: offer.nodes,
        }
    }

    /// Takes the batch when `spawned` is its number, so a stale or replayed
    /// acknowledgement moves nothing.
    pub fn acknowledge(&mut self, spawned: u32) -> Vec<SpawnRef> {
        if spawned != self.number {
            return Vec::new();
        }
        self.number = self.number.saturating_add(1);
        self.nodes = OfferedNodes::empty();
        std::mem::take(&mut self.spawns)
    }

    /// Offers `offer` in place of the pending batch, returning the pending
    /// spawns it drops. A node still pending keeps the context it was first
    /// offered with.
    pub fn replace(&mut self, offer: Offer) -> Vec<SpawnRef> {
        let displaced = self
            .spawns
            .iter()
            .filter(|spawn| !offer.spawns.contains(spawn))
            .cloned()
            .collect();
        if !self.spawns.is_empty() && self.spawns != offer.spawns {
            self.number = self.number.saturating_add(1);
        }
        let first_offered = std::mem::take(&mut self.nodes);
        self.nodes =
            OfferedNodes::of(offer.nodes.iter().map(|(at, offered)| {
                let context = first_offered.context(at).unwrap_or(offered);
                (at.clone(), context.clone())
            }));
        self.spawns = offer.spawns;
        displaced
    }

    #[must_use]
    pub fn context(&self, at: &NoteRef) -> Option<&NodeContext> {
        self.nodes.context(at)
    }

    #[must_use]
    pub const fn nodes(&self) -> &OfferedNodes {
        &self.nodes
    }

    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    #[must_use]
    pub fn spawns(&self) -> &[SpawnRef] {
        &self.spawns
    }
}

/// Profiles whose researcher spawns a run no longer offers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Withheld(BTreeSet<String>);

impl Withheld {
    #[must_use]
    pub fn of(profiles: impl IntoIterator<Item = String>) -> Self {
        Self(profiles.into_iter().collect())
    }

    #[must_use]
    pub fn joined(mut self, other: &Self) -> Self {
        self.0.extend(other.0.iter().cloned());
        self
    }

    #[must_use]
    pub fn contains(&self, profile: &str) -> bool {
        self.0.contains(profile)
    }

    pub fn profiles(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.iter().map(String::as_str)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunMemory {
    pub pending: PendingBatch,
    pub attempted: BTreeSet<SpawnRef>,
    pub just_acknowledged: BTreeSet<SpawnRef>,
    pub seen: Observed,
    pub owed_quarantine: BTreeSet<Stem>,
    pub withheld: Withheld,
}

impl RunMemory {
    /// Whether a write by `spawn` is accounted for: its batch was just
    /// acknowledged, or is offered and not yet acknowledged.
    #[must_use]
    pub fn excuses(&self, spawn: &SpawnRef) -> bool {
        self.just_acknowledged.contains(spawn)
            || self.pending.spawns().contains(spawn)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::Offer;
    use super::PendingBatch;
    use super::RunMemory;
    use crate::conduct::offered_nodes::NodeContext;
    use crate::conduct::offered_nodes::OfferedNodes;
    use crate::conduct::spawn::SpawnRef;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;

    fn spawns(texts: &[&str]) -> Vec<SpawnRef> {
        texts
            .iter()
            .map(|text| SpawnRef::parse(text).expect("a spawn ref"))
            .collect()
    }

    fn node(text: &str) -> NoteRef {
        let (stem, lineage) = text.split_once(':').expect("a node ref");
        NoteRef {
            stem: Stem::parse(stem).expect("a stem"),
            lineage: Lineage::parse(lineage).expect("a lineage"),
        }
    }

    fn asking(question: &str) -> NodeContext {
        NodeContext {
            profile: "web".into(),
            question: question.into(),
            known_questions: Vec::new(),
        }
    }

    fn contexts(asked: &[(&str, &str)]) -> OfferedNodes {
        OfferedNodes::of(
            asked
                .iter()
                .map(|(at, question)| (node(at), asking(question))),
        )
    }

    fn offer(texts: &[&str]) -> Offer {
        Offer::new(spawns(texts), OfferedNodes::empty())
    }

    fn pending(number: u32, texts: &[&str]) -> PendingBatch {
        PendingBatch::restored(number, offer(texts))
    }

    #[test]
    fn retained_for_keeps_only_contexts_of_node_spawns_held() {
        let held = contexts(&[("01-a:2-1", "A?"), ("01-a:2-2", "B?")])
            .retained_for(&spawns(&["01-a:2-1", "01-a"]));
        assert_eq!(held, contexts(&[("01-a:2-1", "A?")]));
    }

    #[test]
    fn a_batch_keeps_no_context_for_a_spawn_it_does_not_hold() {
        let batch = PendingBatch::restored(
            0,
            Offer::new(
                spawns(&["01-a:2-1"]),
                contexts(&[("01-a:2-1", "A?"), ("01-a:2-2", "B?")]),
            ),
        );
        assert_eq!(batch.context(&node("01-a:2-1")), Some(&asking("A?")));
        assert_eq!(batch.context(&node("01-a:2-2")), None);
    }

    #[test]
    fn a_batch_may_hold_pending_nodes_without_context() {
        let batch = PendingBatch::restored(0, offer(&["01-a:2-1"]));
        assert_eq!(batch.spawns(), spawns(&["01-a:2-1"]));
        assert_eq!(batch.context(&node("01-a:2-1")), None);
    }

    #[test]
    fn acknowledging_a_batch_clears_its_contexts() {
        let mut batch = PendingBatch::restored(
            0,
            Offer::new(spawns(&["01-a:2-1"]), contexts(&[("01-a:2-1", "A?")])),
        );
        batch.acknowledge(0);
        assert_eq!(batch.nodes(), &OfferedNodes::empty());
    }

    #[test]
    fn the_next_offer_replaces_an_unreadable_snapshot() {
        let mut batch = PendingBatch::restored(
            0,
            Offer::new(spawns(&["01-a:2-1"]), OfferedNodes::unreadable()),
        );
        assert!(batch.nodes().is_unreadable());
        batch.replace(Offer::new(
            spawns(&["01-a:2-1"]),
            contexts(&[("01-a:2-1", "A?")]),
        ));
        assert!(!batch.nodes().is_unreadable());
        assert_eq!(batch.context(&node("01-a:2-1")), Some(&asking("A?")));
    }

    #[test]
    fn a_stale_acknowledgement_keeps_the_contexts() {
        let mut batch = PendingBatch::restored(
            0,
            Offer::new(spawns(&["01-a:2-1"]), contexts(&[("01-a:2-1", "A?")])),
        );
        batch.acknowledge(1);
        assert_eq!(batch.context(&node("01-a:2-1")), Some(&asking("A?")));
    }

    #[test]
    fn a_stale_acknowledgement_moves_nothing() {
        let mut batch = pending(1, &["01-a"]);
        assert!(batch.acknowledge(0).is_empty());
        assert!(batch.acknowledge(2).is_empty());
        assert_eq!(batch, pending(1, &["01-a"]));
    }

    #[test]
    fn replacing_returns_only_spawns_absent_from_the_new_offer() {
        let mut batch = pending(3, &["01-a", "02-b"]);
        assert_eq!(batch.replace(offer(&["02-b", "03-c"])), spawns(&["01-a"]));
        assert_eq!(batch.number(), 4);
        assert!(batch.replace(offer(&["02-b", "03-c"])).is_empty());
        assert_eq!(batch.number(), 4);
        let mut empty = pending(2, &[]);
        assert!(empty.replace(offer(&["01-a"])).is_empty());
        assert_eq!(empty.number(), 2);
    }

    #[test]
    fn a_pending_or_just_acknowledged_spawn_is_excused() {
        let memory = RunMemory {
            pending: pending(0, &["01-a"]),
            just_acknowledged: spawns(&["02-b"]).into_iter().collect(),
            ..RunMemory::default()
        };
        let excused =
            |text: &str| spawns(&[text]).iter().all(|s| memory.excuses(s));
        assert!(excused("01-a"));
        assert!(excused("02-b"));
        assert!(!excused("03-c"));
    }
}
