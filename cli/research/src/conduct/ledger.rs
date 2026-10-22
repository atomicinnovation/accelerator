//! A `conduct` run's ledger between plans: whose run it is, the indexes it
//! claimed, and what it remembers.

use std::fmt;

use crate::conduct::memory::RunMemory;
use crate::conduct::memory::Withheld;
use crate::conduct::observed::Observed;
use crate::conduct::search_budget::SearchBudgets;
use crate::conduct::spawn::SpawnRef;
use crate::conduct::window::Window;
use crate::topic::claims::ClaimedIndexes;
use crate::topic::layout::stem::Stem;
use crate::topic::plan::RoundInputs;
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
    search_budgets: Option<SearchBudgets>,
}

/// Another run owns the ledger a run tried to continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superseded {
    pub by: RunId,
}

/// Findings a run was started owing to quarantine that are still on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwedQuarantine {
    pub stems: Vec<Stem>,
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
            search_budgets: None,
        }
    }

    /// Budgets are sized once per run, by its first plan to find them
    /// missing, so later plans keep them rather than resize to what remains.
    #[must_use]
    pub const fn awaits_search_budgets(&self) -> bool {
        self.search_budgets.is_none()
    }

    #[must_use]
    pub fn budgeting_searches(mut self, budgets: SearchBudgets) -> Self {
        self.search_budgets = Some(budgets);
        self
    }

    /// Withheld profiles stay withheld for the rest of the run, so a plan
    /// that omits them withholds them still.
    #[must_use]
    pub fn withholding(mut self, profiles: &Withheld) -> Self {
        self.memory.withheld =
            std::mem::take(&mut self.memory.withheld).joined(profiles);
        self
    }

    /// The plan that started this run already reads `stems` as quarantined,
    /// so the run owes their renames until they leave the set.
    #[must_use]
    pub fn owing_quarantine(mut self, stems: &[Stem]) -> Self {
        self.memory.owed_quarantine.extend(stems.iter().cloned());
        self
    }

    /// # Errors
    ///
    /// [`OwedQuarantine`] when a finding the run owes quarantining is still
    /// among `inputs`.
    pub fn settling_quarantine(
        mut self,
        inputs: &RoundInputs,
    ) -> Result<Self, OwedQuarantine> {
        let remaining: Vec<Stem> = inputs
            .findings
            .iter()
            .filter_map(|finding| finding.stem())
            .filter(|stem| self.memory.owed_quarantine.contains(stem))
            .cloned()
            .collect();
        if !remaining.is_empty() {
            return Err(OwedQuarantine { stems: remaining });
        }
        self.memory.owed_quarantine.clear();
        Ok(self)
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
    /// normally changes because those spawns wrote. Withholding drops them
    /// without their having written, so held-back spawns stay unattempted.
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
        self.memory.attempted.extend(
            displaced
                .into_iter()
                .filter(|spawn| !window.withheld_spawns.contains(spawn)),
        );
        self.memory.just_acknowledged.clear();
        self
    }

    #[must_use]
    pub const fn from_parts(
        run: RunId,
        claims: ClaimedIndexes,
        memory: RunMemory,
        search_budgets: Option<SearchBudgets>,
    ) -> Self {
        Self {
            run,
            claims,
            memory,
            search_budgets,
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

    #[must_use]
    pub const fn search_budgets(&self) -> Option<&SearchBudgets> {
        self.search_budgets.as_ref()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;

    use super::OwedQuarantine;
    use super::RunId;
    use super::RunLedger;
    use super::Superseded;
    use crate::conduct::concurrency::test_support::unbounded;
    use crate::conduct::memory::Withheld;
    use crate::conduct::observed::Observed;
    use crate::conduct::search_budget::SearchBudgets;
    use crate::conduct::spawn::SpawnRef;
    use crate::conduct::window::window;
    use crate::conduct::window::Window;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::Finding;
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
    use crate::topic::tree::WorstCaseTree;

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
            worst_case: WorstCaseTree::default(),
        }
    }

    fn planning(stems: &[&str]) -> RoundPlan {
        let mut plan = empty_plan();
        plan.pairs = stems.iter().map(|stem| single_pass(stem)).collect();
        plan
    }

    fn offering(stems: &[&str]) -> Window {
        window(&planning(stems), &Observed::default(), unbounded(), None)
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
            unbounded(),
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
        let mut offer = window(&empty_plan(), &observed, unbounded(), None);
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
            &window(&empty_plan(), &observed, unbounded(), None),
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
            window(&empty_plan(), &observed, unbounded(), Some(ledger.memory()))
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

    fn stems(texts: &[&str]) -> Vec<Stem> {
        texts
            .iter()
            .map(|text| Stem::parse(text).expect("a stem"))
            .collect()
    }

    fn owing(texts: &[&str]) -> RunLedger {
        started().owing_quarantine(&stems(texts))
    }

    fn holding_findings(names: &[&str]) -> RoundInputs {
        RoundInputs {
            findings: names.iter().map(|name| Finding::invalid(name)).collect(),
            ..RoundInputs::default()
        }
    }

    #[test]
    fn a_started_ledger_owes_no_quarantine() {
        assert!(started().memory().owed_quarantine.is_empty());
    }

    #[test]
    fn a_run_cannot_settle_an_owed_quarantine_whose_finding_remains() {
        let settled = owing(&["01-a-web", "02-b-web"]).settling_quarantine(
            &holding_findings(&["02-b-web.md", "03-c-web.md"]),
        );
        assert_eq!(
            settled,
            Err(OwedQuarantine {
                stems: stems(&["02-b-web"])
            })
        );
    }

    #[test]
    fn a_run_whose_owed_findings_are_gone_owes_nothing() {
        let settled = owing(&["01-a-web"])
            .settling_quarantine(&holding_findings(&["03-c-web.md"]))
            .expect("nothing left to quarantine");
        assert!(settled.memory().owed_quarantine.is_empty());
    }

    #[test]
    fn recording_a_plan_keeps_the_owed_quarantine() {
        let ledger = recorded(owing(&["01-a-web"]), &["03-a-web"]);
        assert_eq!(
            ledger.memory().owed_quarantine,
            stems(&["01-a-web"]).into_iter().collect()
        );
    }

    fn budgets() -> SearchBudgets {
        let levels = [(1, 3), (2, 2), (3, 1)]
            .into_iter()
            .map(|(level, budget)| {
                (level, NonZeroU32::new(budget).expect("a budget"))
            })
            .collect::<BTreeMap<_, _>>();
        SearchBudgets::from_parts(levels, 190)
    }

    #[test]
    fn a_started_ledger_has_no_search_budgets() {
        let ledger = started();
        assert_eq!(ledger.search_budgets(), None);
        assert!(ledger.awaits_search_budgets());
    }

    #[test]
    fn budgeting_searches_stores_the_budgets() {
        let ledger = started().budgeting_searches(budgets());
        assert_eq!(ledger.search_budgets(), Some(&budgets()));
        assert!(!ledger.awaits_search_budgets());
    }

    #[test]
    fn continuing_and_recording_keep_the_search_budgets() {
        let ledger =
            recorded(started().budgeting_searches(budgets()), &["01-a"]);
        let ledger = recorded(continued(ledger, Some(0)), &["02-b"]);
        assert_eq!(ledger.search_budgets(), Some(&budgets()));
    }

    fn withheld(profiles: &[&str]) -> Withheld {
        Withheld::of(profiles.iter().map(|profile| (*profile).to_owned()))
    }

    #[test]
    fn a_started_ledger_withholds_nothing() {
        assert_eq!(started().memory().withheld, Withheld::default());
    }

    #[test]
    fn withholding_adds_to_what_the_run_already_withholds() {
        let ledger = started()
            .withholding(&withheld(&["web"]))
            .withholding(&withheld(&["web", "x"]));
        assert_eq!(ledger.memory().withheld, withheld(&["web", "x"]));
        let ledger = started()
            .withholding(&withheld(&["web"]))
            .withholding(&withheld(&["web"]));
        assert_eq!(ledger.memory().withheld, withheld(&["web"]));
    }

    #[test]
    fn withholding_an_unacknowledged_batch_leaves_its_spawns_unattempted() {
        let ledger = recorded(started(), &["01-a"]);
        let ledger = continued(ledger, None).withholding(&withheld(&["web"]));
        let plan = planning(&["01-a"]);
        let offer = window(
            &plan,
            &Observed::default(),
            unbounded(),
            Some(ledger.memory()),
        );

        let ledger = ledger.record(&plan, &Observed::default(), &offer);

        assert_eq!(ledger.memory().attempted, set(&[]));
        assert!(ledger.memory().pending.spawns().is_empty());
    }

    #[test]
    fn continuing_and_recording_keep_the_withheld_profiles() {
        let ledger =
            recorded(started().withholding(&withheld(&["web"])), &["01-a"]);
        let ledger = recorded(continued(ledger, Some(0)), &["02-b"]);
        assert_eq!(ledger.memory().withheld, withheld(&["web"]));
    }
}
