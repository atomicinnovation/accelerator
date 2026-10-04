//! What a `conduct` run remembers between plans: the batch it last offered,
//! the spawns it attempted, and what it has seen the set show.

use std::collections::BTreeSet;

use crate::conduct::observed::Observed;
use crate::conduct::spawn::SpawnRef;

/// The spawns last offered and not yet acknowledged, under the number the
/// acknowledgement must quote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingBatch {
    number: u32,
    spawns: Vec<SpawnRef>,
}

impl PendingBatch {
    #[must_use]
    pub const fn new(number: u32, spawns: Vec<SpawnRef>) -> Self {
        Self { number, spawns }
    }

    /// Takes the batch when `spawned` is its number, so a stale or replayed
    /// acknowledgement moves nothing.
    pub fn acknowledge(&mut self, spawned: u32) -> Vec<SpawnRef> {
        if spawned != self.number {
            return Vec::new();
        }
        self.number = self.number.saturating_add(1);
        std::mem::take(&mut self.spawns)
    }

    /// Offers `spawns` in place of the pending batch, returning the pending
    /// spawns the new offer drops.
    pub fn replace(&mut self, spawns: Vec<SpawnRef>) -> Vec<SpawnRef> {
        let displaced = self
            .spawns
            .iter()
            .filter(|spawn| !spawns.contains(spawn))
            .cloned()
            .collect();
        if !self.spawns.is_empty() && self.spawns != spawns {
            self.number = self.number.saturating_add(1);
        }
        self.spawns = spawns;
        displaced
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunMemory {
    pub pending: PendingBatch,
    pub attempted: BTreeSet<SpawnRef>,
    pub just_acknowledged: BTreeSet<SpawnRef>,
    pub seen: Observed,
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
    use super::PendingBatch;
    use super::RunMemory;
    use crate::conduct::spawn::SpawnRef;

    fn spawns(texts: &[&str]) -> Vec<SpawnRef> {
        texts
            .iter()
            .map(|text| SpawnRef::parse(text).expect("a spawn ref"))
            .collect()
    }

    #[test]
    fn a_stale_acknowledgement_moves_nothing() {
        let mut batch = PendingBatch::new(1, spawns(&["01-a"]));
        assert!(batch.acknowledge(0).is_empty());
        assert!(batch.acknowledge(2).is_empty());
        assert_eq!(batch, PendingBatch::new(1, spawns(&["01-a"])));
    }

    #[test]
    fn replacing_returns_only_spawns_absent_from_the_new_offer() {
        let mut batch = PendingBatch::new(3, spawns(&["01-a", "02-b"]));
        assert_eq!(batch.replace(spawns(&["02-b", "03-c"])), spawns(&["01-a"]));
        assert_eq!(batch.number(), 4);
        assert!(batch.replace(spawns(&["02-b", "03-c"])).is_empty());
        assert_eq!(batch.number(), 4);
        let mut empty = PendingBatch::new(2, Vec::new());
        assert!(empty.replace(spawns(&["01-a"])).is_empty());
        assert_eq!(empty.number(), 2);
    }

    #[test]
    fn a_pending_or_just_acknowledged_spawn_is_excused() {
        let memory = RunMemory {
            pending: PendingBatch::new(0, spawns(&["01-a"])),
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
