//! A `conduct` run's memory between plans: the batch it was offered, the
//! indexes it pinned, and the spawns it attempted.

use std::fmt;

use crate::pinned_indexes::PinnedIndexes;
use crate::round::Round;
use crate::spawn_window::Attempts;
use crate::spawn_window::SpawnRef;
use crate::spawn_window::Window;

const LONGEST_RUN_ID: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(String);

impl RunId {
    /// The suffix keeps two runs started in the same second apart.
    #[must_use]
    pub fn mint(timestamp: &str, suffix: u32) -> Self {
        Self(format!("{timestamp}-{suffix}"))
    }

    /// Accepts `[A-Za-z0-9_-]{1,64}`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let well_formed = (1..=LONGEST_RUN_ID).contains(&text.len())
            && text
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        well_formed.then(|| Self(text.to_owned()))
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLedger {
    run: RunId,
    pending: PendingBatch,
    pins: PinnedIndexes,
    attempts: Attempts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Continuation {
    Continued(RunLedger),
    Superseded { stored: RunId },
}

impl RunLedger {
    /// Everything `round` already shows counts as seen, so a resumed set's
    /// existing notes and findings are never reported as unexpected.
    #[must_use]
    pub fn start(run: RunId, round: &Round) -> Self {
        Self {
            run,
            pending: PendingBatch::default(),
            pins: PinnedIndexes::default(),
            attempts: Attempts {
                notes_seen: round.accepted_notes.clone(),
                answered_seen: round.answered.clone(),
                ..Attempts::default()
            },
        }
    }

    /// Acknowledging a batch forgets the digests of its nodes' notes, so a
    /// note re-researched in that batch is taken afresh rather than compared
    /// with the note it replaced.
    #[must_use]
    pub fn continue_as(
        stored: Self,
        run: &RunId,
        spawned: Option<u32>,
    ) -> Continuation {
        if stored.run != *run {
            return Continuation::Superseded { stored: stored.run };
        }
        let mut ledger = stored;
        let acknowledged = spawned
            .map(|batch| ledger.pending.acknowledge(batch))
            .unwrap_or_default();
        for spawn in &acknowledged {
            if let SpawnRef::Node(at) = spawn {
                ledger.attempts.notes_seen.remove(at);
            }
        }
        ledger
            .attempts
            .attempted
            .extend(acknowledged.iter().cloned());
        ledger.attempts.just_acknowledged = acknowledged.into_iter().collect();
        Continuation::Continued(ledger.with_pending_synced())
    }

    /// Pending spawns a changed offer drops count as attempted: an offer
    /// normally changes because those spawns wrote.
    #[must_use]
    pub fn record(mut self, window: &Window) -> Self {
        self.pins.pin_all(&window.round.indexes);
        for (at, digest) in &window.round.accepted_notes {
            self.attempts
                .notes_seen
                .entry(at.clone())
                .or_insert(*digest);
        }
        self.attempts
            .answered_seen
            .extend(window.round.answered.iter().cloned());
        let displaced = self.pending.replace(window.offered.clone());
        self.attempts.attempted.extend(displaced);
        self.attempts.just_acknowledged.clear();
        self.with_pending_synced()
    }

    #[must_use]
    pub fn from_parts(
        run: RunId,
        pending: PendingBatch,
        pins: PinnedIndexes,
        attempts: Attempts,
    ) -> Self {
        Self {
            run,
            pending,
            pins,
            attempts,
        }
        .with_pending_synced()
    }

    #[must_use]
    pub const fn run(&self) -> &RunId {
        &self.run
    }

    #[must_use]
    pub const fn pending(&self) -> &PendingBatch {
        &self.pending
    }

    #[must_use]
    pub const fn pins(&self) -> &PinnedIndexes {
        &self.pins
    }

    #[must_use]
    pub const fn attempts(&self) -> &Attempts {
        &self.attempts
    }

    fn with_pending_synced(mut self) -> Self {
        self.attempts.pending = self.pending.spawns.iter().cloned().collect();
        self
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;

    use super::Continuation;
    use super::PendingBatch;
    use super::RunId;
    use super::RunLedger;
    use crate::pinned_indexes::PinnedIndexes;
    use crate::question::FakeUnicode;
    use crate::question::NormalisedQuestion;
    use crate::round::Pair;
    use crate::round::Round;
    use crate::round::RoundInputs;
    use crate::round::Stage;
    use crate::spawn_window::window;
    use crate::spawn_window::Attempts;
    use crate::spawn_window::SpawnRef;
    use crate::spawn_window::Window;
    use crate::tree::Digest;
    use crate::tree::NoteRef;
    use corpus::topic_research::Lineage;
    use corpus::topic_research::Stem;

    const SEEN: Digest = Digest::new([1; 32]);
    const CHANGED: Digest = Digest::new([2; 32]);

    fn run(text: &str) -> RunId {
        RunId::parse(text).expect("a run id")
    }

    fn spawn(text: &str) -> SpawnRef {
        SpawnRef::parse(text).expect("a spawn ref")
    }

    fn spawns(texts: &[&str]) -> Vec<SpawnRef> {
        texts.iter().map(|text| spawn(text)).collect()
    }

    fn set(texts: &[&str]) -> BTreeSet<SpawnRef> {
        spawns(texts).into_iter().collect()
    }

    fn note_ref(text: &str) -> NoteRef {
        let (stem, at) = text.split_once(':').expect("a note ref");
        NoteRef {
            stem: Stem::parse(stem).expect("a stem"),
            lineage: Lineage::parse(at).expect("a lineage"),
        }
    }

    fn empty_round() -> Round {
        Round::plan(&RoundInputs::default(), &FakeUnicode)
    }

    fn research(stem: &str) -> Pair {
        Pair {
            question: "Q?".into(),
            profile: "web".into(),
            stem: Stem::parse(stem).expect("a stem"),
            path: format!("findings/{stem}.md"),
            stage: Stage::Research,
        }
    }

    fn offering(stems: &[&str]) -> Window {
        let mut round = empty_round();
        round.pairs = stems.iter().map(|stem| research(stem)).collect();
        window(round, None, None)
    }

    fn continued(ledger: RunLedger, spawned: Option<u32>) -> RunLedger {
        let run = ledger.run().clone();
        match RunLedger::continue_as(ledger, &run, spawned) {
            Continuation::Continued(ledger) => ledger,
            Continuation::Superseded { stored } => {
                RunLedger::start(stored, &empty_round())
            }
        }
    }

    fn started() -> RunLedger {
        RunLedger::start(run("r1"), &empty_round())
    }

    fn question(text: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(text, &FakeUnicode)
    }

    #[test]
    fn a_started_ledger_snapshots_the_accepted_notes_and_pins_nothing() {
        let mut round = empty_round();
        round.accepted_notes.insert(note_ref("01-a:1"), SEEN);
        round.answered.insert(Stem::parse("02-b").expect("a stem"));
        let ledger = RunLedger::start(run("r1"), &round);
        assert_eq!(ledger.attempts().notes_seen, round.accepted_notes);
        assert_eq!(ledger.attempts().answered_seen, round.answered);
        assert_eq!(ledger.pins(), &PinnedIndexes::default());
        assert!(ledger.attempts().attempted.is_empty());
        assert!(ledger.attempts().just_acknowledged.is_empty());
    }

    #[test]
    fn a_started_ledger_holds_batch_zero_with_no_spawns() {
        let ledger = started();
        assert_eq!(ledger.pending().number(), 0);
        assert!(ledger.pending().spawns().is_empty());
        assert!(ledger.attempts().pending.is_empty());
    }

    #[test]
    fn a_minted_run_id_carries_its_timestamp_and_suffix_and_parses() {
        let minted = RunId::mint("2026-09-27-153000", 42);
        assert_eq!(minted.to_string(), "2026-09-27-153000-42");
        assert_eq!(RunId::parse(&minted.to_string()), Some(minted));
    }

    #[test]
    fn a_run_id_is_refused_outside_its_alphabet() {
        for text in ["", "a/b", "..", "a:b", "a\nb", "a b"] {
            assert_eq!(RunId::parse(text), None, "{text:?}");
        }
        assert!(RunId::parse("Run_1-x").is_some());
    }

    #[test]
    fn a_run_id_longer_than_64_characters_is_refused() {
        assert!(RunId::parse(&"a".repeat(64)).is_some());
        assert_eq!(RunId::parse(&"a".repeat(65)), None);
    }

    #[test]
    fn continuing_another_runs_ledger_is_superseded() {
        assert_eq!(
            RunLedger::continue_as(started(), &run("r2"), Some(0)),
            Continuation::Superseded { stored: run("r1") }
        );
    }

    #[test]
    fn acknowledging_the_offered_batch_moves_it_to_attempted() {
        let ledger = started().record(&offering(&["01-a", "02-b"]));
        let ledger = continued(ledger, Some(0));
        assert_eq!(ledger.attempts().attempted, set(&["01-a", "02-b"]));
        assert_eq!(ledger.attempts().just_acknowledged, set(&["01-a", "02-b"]));
        assert!(ledger.pending().spawns().is_empty());
        assert!(ledger.attempts().pending.is_empty());
    }

    #[test]
    fn acknowledging_a_batch_advances_its_number() {
        let ledger = continued(started().record(&offering(&["01-a"])), Some(0));
        assert_eq!(ledger.pending().number(), 1);
        let ledger = ledger.record(&offering(&["02-b"]));
        assert_eq!(ledger.pending().number(), 1);
        assert_eq!(ledger.pending().spawns(), spawns(&["02-b"]));
    }

    #[test]
    fn a_repeated_plan_without_acknowledgement_re_offers_the_same_batch() {
        let ledger = started().record(&offering(&["01-a"]));
        let ledger = continued(ledger, None);
        let again =
            window(offering(&["01-a"]).round, None, Some(ledger.attempts()));
        assert_eq!(again.offered, spawns(&["01-a"]));
        let ledger = ledger.record(&again);
        assert_eq!(ledger.pending().number(), 0);
        assert_eq!(ledger.pending().spawns(), spawns(&["01-a"]));
        assert!(ledger.attempts().attempted.is_empty());
    }

    #[test]
    fn a_stale_acknowledgement_moves_nothing() {
        let mut batch = PendingBatch::new(1, spawns(&["01-a"]));
        assert!(batch.acknowledge(0).is_empty());
        assert!(batch.acknowledge(2).is_empty());
        assert_eq!(batch, PendingBatch::new(1, spawns(&["01-a"])));
    }

    #[test]
    fn a_replayed_acknowledgement_after_the_next_offer_moves_nothing() {
        let ledger = continued(started().record(&offering(&["01-a"])), Some(0));
        let ledger = ledger.record(&offering(&["02-b"]));
        let ledger = continued(ledger, Some(0));
        assert_eq!(ledger.attempts().attempted, set(&["01-a"]));
        assert!(ledger.attempts().just_acknowledged.is_empty());
        assert_eq!(ledger.pending().spawns(), spawns(&["02-b"]));
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
    fn recording_pins_every_index_and_advances_the_batch_only_when_the_offer_changes(
    ) {
        let mut offer = offering(&["01-a"]);
        offer.round.indexes.pin(question("A?"), 1);
        offer.round.indexes.pin(question("B?"), 2);
        let ledger = started().record(&offer);
        assert_eq!(ledger.pins(), &offer.round.indexes);
        assert_eq!(ledger.pending().number(), 0);
        let ledger = continued(ledger, None).record(&offer);
        assert_eq!(ledger.pending().number(), 0);
        let ledger = continued(ledger, None).record(&offering(&["02-b"]));
        assert_eq!(ledger.pending().number(), 1);
        assert_eq!(ledger.pins().index_for(&question("A?")), Some(1));
    }

    #[test]
    fn acknowledging_a_batch_drops_its_nodes_seen_digests() {
        let mut round = empty_round();
        round.accepted_notes.insert(note_ref("01-a:1"), SEEN);
        round.accepted_notes.insert(note_ref("02-b:1"), SEEN);
        let ledger = RunLedger::start(run("r1"), &round);
        let mut offer = window(round, None, None);
        offer.offered = spawns(&["01-a:1"]);
        let ledger = continued(ledger.record(&offer), Some(0));
        let seen: Vec<String> = ledger
            .attempts()
            .notes_seen
            .keys()
            .map(ToString::to_string)
            .collect();
        assert_eq!(seen, ["02-b:1"]);
    }

    #[test]
    fn replacing_an_unacknowledged_batch_after_its_notes_land_attempts_it() {
        let ledger = started().record(&offering(&["01-a", "02-b"]));
        let ledger = continued(ledger, None).record(&offering(&["02-b"]));
        assert_eq!(ledger.attempts().attempted, set(&["01-a"]));
        assert_eq!(ledger.pending().spawns(), spawns(&["02-b"]));
    }

    #[test]
    fn recording_adds_newly_accepted_note_digests_without_replacing_seen_ones()
    {
        let mut round = empty_round();
        round.accepted_notes.insert(note_ref("01-a:1"), SEEN);
        let ledger = RunLedger::start(run("r1"), &round);
        round.accepted_notes.insert(note_ref("01-a:1"), CHANGED);
        round.accepted_notes.insert(note_ref("01-a:2-1"), CHANGED);
        let ledger = ledger.record(&window(round, None, None));
        assert_eq!(
            ledger.attempts().notes_seen.get(&note_ref("01-a:1")),
            Some(&SEEN)
        );
        assert_eq!(
            ledger.attempts().notes_seen.get(&note_ref("01-a:2-1")),
            Some(&CHANGED)
        );
    }

    #[test]
    fn recording_clears_just_acknowledged() {
        let ledger = continued(started().record(&offering(&["01-a"])), Some(0));
        assert!(!ledger.attempts().just_acknowledged.is_empty());
        let ledger = ledger.record(&offering(&[]));
        assert!(ledger.attempts().just_acknowledged.is_empty());
    }

    #[test]
    fn a_pair_answered_in_one_batch_is_not_unexpected_two_plans_later() {
        let answered = |ledger: &RunLedger| {
            let mut round = empty_round();
            round.answered.insert(Stem::parse("01-a").expect("a stem"));
            window(round, None, Some(ledger.attempts()))
        };
        let ledger = continued(started().record(&offering(&["01-a"])), Some(0));
        let first = answered(&ledger);
        assert!(first.unexpected.is_empty());
        let ledger = continued(ledger.record(&first), None);
        let second = answered(&ledger);
        assert!(second.unexpected.is_empty());
        let ledger = continued(ledger.record(&second), None);
        assert!(answered(&ledger).unexpected.is_empty());
    }

    #[test]
    fn a_ledger_rebuilt_from_its_parts_syncs_its_pending_attempts() {
        let rebuilt = RunLedger::from_parts(
            run("r1"),
            PendingBatch::new(2, spawns(&["01-a:1"])),
            PinnedIndexes::default(),
            Attempts::default(),
        );
        assert_eq!(rebuilt.attempts().pending, set(&["01-a:1"]));
        assert_eq!(rebuilt.pending().number(), 2);
    }
}
