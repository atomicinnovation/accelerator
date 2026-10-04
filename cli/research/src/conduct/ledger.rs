//! A `conduct` run's ledger between plans: whose run it is, the indexes it
//! claimed, and what it remembers.

use std::fmt;

use crate::conduct::memory::RunMemory;
use crate::conduct::observed::Observed;
use crate::conduct::spawn::SpawnRef;
use crate::conduct::window::Window;
use crate::topic::claims::ClaimedIndexes;
use crate::topic::plan::RoundPlan;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLedger {
    run: RunId,
    claims: ClaimedIndexes,
    memory: RunMemory,
}

/// Another run owns the ledger a run tried to continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superseded {
    pub by: RunId,
}

impl RunLedger {
    /// Everything `observed` shows counts as seen, so a resumed set's
    /// existing notes and findings are never reported as unexpected.
    #[must_use]
    pub fn start(run: RunId, observed: &Observed) -> Self {
        Self {
            run,
            claims: ClaimedIndexes::default(),
            memory: RunMemory {
                seen: observed.clone(),
                ..RunMemory::default()
            },
        }
    }

    /// Acknowledging a batch forgets the digests of its nodes' notes, so a
    /// note re-researched in that batch is taken afresh rather than compared
    /// with the note it replaced.
    ///
    /// # Errors
    ///
    /// [`Superseded`] when `stored` belongs to another run.
    pub fn continue_as(
        stored: Self,
        run: &RunId,
        spawned: Option<u32>,
    ) -> Result<Self, Superseded> {
        if stored.run != *run {
            return Err(Superseded { by: stored.run });
        }
        let mut ledger = stored;
        let memory = &mut ledger.memory;
        let acknowledged = spawned
            .map(|batch| memory.pending.acknowledge(batch))
            .unwrap_or_default();
        for spawn in &acknowledged {
            if let SpawnRef::Node(at) = spawn {
                memory.seen.notes.remove(at);
            }
        }
        memory.attempted.extend(acknowledged.iter().cloned());
        memory.just_acknowledged = acknowledged.into_iter().collect();
        Ok(ledger)
    }

    /// Pending spawns a changed offer drops count as attempted: an offer
    /// normally changes because those spawns wrote.
    #[must_use]
    pub fn record(
        mut self,
        plan: &RoundPlan,
        observed: &Observed,
        window: &Window,
    ) -> Self {
        self.claims.claim_all(&plan.claims);
        self.memory.seen.absorb(observed);
        let displaced = self.memory.pending.replace(window.offered.clone());
        self.memory.attempted.extend(displaced);
        self.memory.just_acknowledged.clear();
        self
    }

    #[must_use]
    pub const fn from_parts(
        run: RunId,
        claims: ClaimedIndexes,
        memory: RunMemory,
    ) -> Self {
        Self {
            run,
            claims,
            memory,
        }
    }

    #[must_use]
    pub const fn run(&self) -> &RunId {
        &self.run
    }

    #[must_use]
    pub const fn claims(&self) -> &ClaimedIndexes {
        &self.claims
    }

    #[must_use]
    pub const fn memory(&self) -> &RunMemory {
        &self.memory
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;

    use super::RunId;
    use super::RunLedger;
    use super::Superseded;
    use crate::conduct::observed::Observed;
    use crate::conduct::spawn::SpawnRef;
    use crate::conduct::window::window;
    use crate::conduct::window::Window;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::evidence::Digest;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Pair;
    use crate::topic::plan::OutstandingPair;
    use crate::topic::plan::RoundInputs;
    use crate::topic::plan::RoundPlan;
    use crate::topic::question::FakeUnicode;
    use crate::topic::question::NormalisedQuestion;
    use crate::topic::tree::Stage;

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

    fn empty_plan() -> RoundPlan {
        RoundPlan::of(&RoundInputs::default(), &FakeUnicode)
    }

    fn single_pass(stem: &str) -> OutstandingPair {
        OutstandingPair {
            pair: Pair {
                question: "Q?".into(),
                profile: "web".into(),
            },
            stem: Stem::parse(stem).expect("a stem"),
            stage: Stage::SinglePass,
        }
    }

    fn planning(stems: &[&str]) -> RoundPlan {
        let mut plan = empty_plan();
        plan.pairs = stems.iter().map(|stem| single_pass(stem)).collect();
        plan
    }

    fn offering(stems: &[&str]) -> Window {
        window(&planning(stems), &Observed::default(), None, None)
    }

    fn recorded(ledger: RunLedger, stems: &[&str]) -> RunLedger {
        ledger.record(&planning(stems), &Observed::default(), &offering(stems))
    }

    fn showing(notes: &[(&str, Digest)]) -> Observed {
        Observed {
            notes: notes
                .iter()
                .map(|(at, digest)| (note_ref(at), *digest))
                .collect(),
            ..Observed::default()
        }
    }

    fn continued(ledger: RunLedger, spawned: Option<u32>) -> RunLedger {
        let run = ledger.run().clone();
        RunLedger::continue_as(ledger, &run, spawned).unwrap_or_else(
            |superseded| RunLedger::start(superseded.by, &Observed::default()),
        )
    }

    fn started() -> RunLedger {
        RunLedger::start(run("r1"), &Observed::default())
    }

    fn question(text: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(text, &FakeUnicode)
    }

    #[test]
    fn a_started_ledger_remembers_what_the_set_shows_and_claims_nothing() {
        let mut observed = showing(&[("01-a:1", SEEN)]);
        observed
            .answered
            .insert(Stem::parse("02-b").expect("a stem"));
        let ledger = RunLedger::start(run("r1"), &observed);
        assert_eq!(ledger.memory().seen, observed);
        assert_eq!(ledger.claims(), &ClaimedIndexes::default());
        assert!(ledger.memory().attempted.is_empty());
        assert!(ledger.memory().just_acknowledged.is_empty());
    }

    #[test]
    fn a_started_ledger_holds_batch_zero_with_no_spawns() {
        let ledger = started();
        assert_eq!(ledger.memory().pending.number(), 0);
        assert!(ledger.memory().pending.spawns().is_empty());
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
            Err(Superseded { by: run("r1") })
        );
    }

    #[test]
    fn acknowledging_the_offered_batch_moves_it_to_attempted() {
        let ledger = recorded(started(), &["01-a", "02-b"]);
        let ledger = continued(ledger, Some(0));
        assert_eq!(ledger.memory().attempted, set(&["01-a", "02-b"]));
        assert_eq!(ledger.memory().just_acknowledged, set(&["01-a", "02-b"]));
        assert!(ledger.memory().pending.spawns().is_empty());
    }

    #[test]
    fn acknowledging_a_batch_advances_its_number() {
        let ledger = continued(recorded(started(), &["01-a"]), Some(0));
        assert_eq!(ledger.memory().pending.number(), 1);
        let ledger = recorded(ledger, &["02-b"]);
        assert_eq!(ledger.memory().pending.number(), 1);
        assert_eq!(ledger.memory().pending.spawns(), spawns(&["02-b"]));
    }

    #[test]
    fn a_repeated_plan_without_acknowledgement_re_offers_the_same_batch() {
        let ledger = continued(recorded(started(), &["01-a"]), None);
        let again = window(
            &planning(&["01-a"]),
            &Observed::default(),
            None,
            Some(ledger.memory()),
        );
        assert_eq!(again.offered, spawns(&["01-a"]));
        let ledger =
            ledger.record(&planning(&["01-a"]), &Observed::default(), &again);
        assert_eq!(ledger.memory().pending.number(), 0);
        assert_eq!(ledger.memory().pending.spawns(), spawns(&["01-a"]));
        assert!(ledger.memory().attempted.is_empty());
    }

    #[test]
    fn a_replayed_acknowledgement_after_the_next_offer_moves_nothing() {
        let ledger = continued(recorded(started(), &["01-a"]), Some(0));
        let ledger = recorded(ledger, &["02-b"]);
        let ledger = continued(ledger, Some(0));
        assert_eq!(ledger.memory().attempted, set(&["01-a"]));
        assert!(ledger.memory().just_acknowledged.is_empty());
        assert_eq!(ledger.memory().pending.spawns(), spawns(&["02-b"]));
    }

    #[test]
    fn recording_claims_every_index_and_advances_the_batch_only_when_the_offer_changes(
    ) {
        let mut plan = planning(&["01-a"]);
        plan.claims.claim(question("A?"), 1);
        plan.claims.claim(question("B?"), 2);
        let offer = offering(&["01-a"]);
        let ledger = started().record(&plan, &Observed::default(), &offer);
        assert_eq!(ledger.claims(), &plan.claims);
        assert_eq!(ledger.memory().pending.number(), 0);
        let ledger =
            continued(ledger, None).record(&plan, &Observed::default(), &offer);
        assert_eq!(ledger.memory().pending.number(), 0);
        let ledger = recorded(continued(ledger, None), &["02-b"]);
        assert_eq!(ledger.memory().pending.number(), 1);
        assert_eq!(ledger.claims().index_for(&question("A?")), Some(1));
    }

    #[test]
    fn acknowledging_a_batch_drops_its_nodes_seen_digests() {
        let observed = showing(&[("01-a:1", SEEN), ("02-b:1", SEEN)]);
        let ledger = RunLedger::start(run("r1"), &observed);
        let mut offer = window(&empty_plan(), &observed, None, None);
        offer.offered = spawns(&["01-a:1"]);
        let ledger =
            continued(ledger.record(&empty_plan(), &observed, &offer), Some(0));
        let seen: Vec<String> = ledger
            .memory()
            .seen
            .notes
            .keys()
            .map(ToString::to_string)
            .collect();
        assert_eq!(seen, ["02-b:1"]);
    }

    #[test]
    fn replacing_an_unacknowledged_batch_after_its_notes_land_attempts_it() {
        let ledger = recorded(started(), &["01-a", "02-b"]);
        let ledger = recorded(continued(ledger, None), &["02-b"]);
        assert_eq!(ledger.memory().attempted, set(&["01-a"]));
        assert_eq!(ledger.memory().pending.spawns(), spawns(&["02-b"]));
    }

    #[test]
    fn recording_adds_newly_accepted_note_digests_without_replacing_seen_ones()
    {
        let ledger = RunLedger::start(run("r1"), &showing(&[("01-a:1", SEEN)]));
        let observed = showing(&[("01-a:1", CHANGED), ("01-a:2-1", CHANGED)]);
        let ledger = ledger.record(
            &empty_plan(),
            &observed,
            &window(&empty_plan(), &observed, None, None),
        );
        assert_eq!(
            ledger.memory().seen.notes.get(&note_ref("01-a:1")),
            Some(&SEEN)
        );
        assert_eq!(
            ledger.memory().seen.notes.get(&note_ref("01-a:2-1")),
            Some(&CHANGED)
        );
    }

    #[test]
    fn recording_clears_just_acknowledged() {
        let ledger = continued(recorded(started(), &["01-a"]), Some(0));
        assert!(!ledger.memory().just_acknowledged.is_empty());
        let ledger = recorded(ledger, &[]);
        assert!(ledger.memory().just_acknowledged.is_empty());
    }

    #[test]
    fn a_pair_answered_in_one_batch_is_not_unexpected_two_plans_later() {
        let mut observed = Observed::default();
        observed
            .answered
            .insert(Stem::parse("01-a").expect("a stem"));
        let answered = |ledger: &RunLedger| {
            window(&empty_plan(), &observed, None, Some(ledger.memory()))
        };
        let ledger = continued(recorded(started(), &["01-a"]), Some(0));
        let first = answered(&ledger);
        assert!(first.unexpected.is_empty());
        let ledger =
            continued(ledger.record(&empty_plan(), &observed, &first), None);
        let second = answered(&ledger);
        assert!(second.unexpected.is_empty());
        let ledger =
            continued(ledger.record(&empty_plan(), &observed, &second), None);
        assert!(answered(&ledger).unexpected.is_empty());
    }
}
