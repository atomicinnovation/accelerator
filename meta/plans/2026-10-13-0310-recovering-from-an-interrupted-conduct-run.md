---
type: "plan"
id: "2026-10-13-0310-recovering-from-an-interrupted-conduct-run"
title: "Recovering from an Interrupted Conduct Run Implementation Plan"
date: "2026-10-13T13:03:28+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0310"
parent: "work-item:0310"
derived_from: ["codebase-research:2026-10-13-0310-recovering-from-an-interrupted-conduct-run"]
relates_to: ["work-item:0304", "work-item:0307", "work-item:0308", "plan:2026-10-10-0303-research-concurrency-subagent-limit"]
tags: ["research", "conduct", "run-ledger", "outstanding", "recovery"]
revision: "01e6fce34d8ad51297dc39e8197b98c20d4bf456"
repository: "accelerator"
last_updated: "2026-10-13T20:45:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Recovering from an Interrupted Conduct Run Implementation Plan

## Overview

When `outstanding --start` replaces another run's readable ledger, the plan
gains a `replaced_run` key: the earlier run's id, its last offered batch
classed by output file as valid, invalid, unwritten or unasked, and the
invalid findings to quarantine. The plan is drawn as if those findings were
already quarantined. `conduct` shows the report before its first batch and
renames the listed findings straight after it, and the failure-reason table
gains an "unfinished run" row. The "replaced the ledger of run" warning
leaves `warnings`.

## Current State Analysis

- `replaced_ledger_notice` (`cli/research-cli/src/topic_command.rs:386-405`)
  reads the old `RunLedger`, keeps only `stored.run()` for a notice, and drops
  `pending.spawns`, the last offered batch, and `claims`, the index each
  question held.
- It runs inside the `RunMode::Start` arm of `run_outstanding`
  (`topic_command.rs:254-273`), before `read_round_inputs` (`:281-290`) and
  `RoundPlan::of` (`:291`). Classification needs the inputs, so the stored
  ledger has to outlive the notice.
- `RoundInputs` (`cli/research/src/topic/plan.rs:23-37`) holds every
  `Finding`, `LevelsDirectory` and quarantine `IndexClaim`. Classification
  needs nothing else read.
- `PlanOutput::render` (`topic_command.rs:433-526`) adds `run`, `batch` and
  `unexpected` only when a ledger exists (`:515-524`). A key added only on a
  replaced ledger follows that pattern and leaves the three whole-plan
  equality tests untouched.
- An invalid finding is never at an offered path on `--start`: `claim_index`
  (`plan.rs:279-309`) runs with empty claims, the invalid file raises the
  index floor (`:120-134`), and the pair lands at a fresh stem. Step 3
  (`skills/research/research-topic/SKILL.md:297-310`) therefore never renames
  it, and the `finding_count` rule (`SKILL.md:611-618`) counts it.
- `outstanding`, its run flags and the run ledger have not shipped: they sit
  under `[Unreleased] ### Added` in `CHANGELOG.md` (`:118-133`), whose ledger
  sentence still calls the ledger "safe to delete after an interrupted run".
- 0304 is still `draft`, so the new row carries the wait-or-raise step inline.

## Desired End State

On `--start` over a readable ledger, the plan JSON carries:

```jsonc
"replaced_run": {
  "run": "<earlier run id>",
  "valid": [{"spawn": "01-a-web", "question": "A?", "profile": "web"}],
  "invalid": [{"spawn": "03-c-web", "question": "C?", "profile": "web"}],
  "unwritten": [{"spawn": "05-e-web:1", "question": "E?", "profile": "web"}],
  "unasked": [{"spawn": "04-d-web", "question": "Old D?", "profile": "web"}],
  "quarantine": ["/abs/.../findings/03-c-web.md"]
}
```

Each entry carries the outline pair that owns its stem; an unasked finding
carries the pair it names, and any other spawn with no owning pair carries
`null` question and profile. Each list keeps the order of the earlier run's
pending batch. The plan around it reads the `quarantine` findings as
quarantine markers, so under this run's claims it is the plan the set yields
once they are renamed; a consumer must rename them before acting on it. The
key is absent with no ledger, after `end-run`, on a corrupt ledger, and on
every plan other than `--start`. A stderr line names it; no `warnings` entry
accompanies it. The corrupt-ledger warning is unchanged.

`conduct` opens with "run {run} did not end; if it is still going, it stops
at its next batch, and what it writes from now on may show as unexpected",
then one group per non-empty class, before step 3 of the first batch.
Straight after the report it renames each `quarantine` path to a `.invalid`
marker, whether or not the plan offers a spawn. The failure-reason table has
an "unfinished run" row.

Verify with `mise run` (the bare default task) exiting 0, plus the manual
interrupted-run walkthrough under Testing Strategy.

### Key Discoveries:

- The classification is pure over the replaced `RunLedger`, the round's
  inputs and the one plan the CLI draws, and belongs in `research::conduct`
  beside `observed` and `window`; the domain crate stays I/O-free
  (`cli/pup.ron`). Pair verdicts need no plan and decide the quarantine;
  node verdicts need the plan drawn after it. So `ReplacedBatch` judges the
  findings and resolves every owner, reads the inputs as quarantined, and
  judges the nodes against the caller's one plan, never a private second
  plan.
- A stem's owning pair comes from the replaced ledger's `claims()`
  (`cli/research/src/conduct/ledger.rs:136`), which `record` fills from every
  plan's claims (`:101-115`). `Planner::plan` claims every item with an
  outstanding pair at the index it allocated, whatever the index's source,
  so every offered spawn's stem is claimed. The owner is the outline item
  whose normalised question the run claimed at the stem's index, under the
  profile whose stem rebuilds it. The slug alone cannot say:
  `QuestionSlug::from` (`topic/layout/slug.rs:18-33`) keeps only ASCII
  letters and digits, so every CJK question slugs to `focus-area`.
- A pair's finding is valid when it answers its owning pair
  (`Finding::answers`, `cli/research/src/topic/evidence.rs:92-100`), and
  invalid when it fails validation or answers anything else. This is step
  3's "does not complete its pair" test, and deliberately stricter than step
  5 and the planner's `answer` (`plan.rs:265-274`), which match by question
  alone and credit a misdirected file to the pair it names.
- Ownership is decided first, the same way for pairs and nodes. A spawn with
  no owning pair (its question or profile left the outline) is unasked and
  left in place, whatever it wrote, unless it left a finding that fails
  validation or answers a pair (question and profile) the outline still
  asks: that finding is invalid and quarantined, but not offered again,
  since no pair owns its stem.
- An owned node is unwritten when its directory holds no note at its
  lineage, accepted or refused. A written node is invalid when its
  directory's `rejected()` holds the lineage, the plan rejects it, or its
  directory's `root_question()` is not its owning pair's question (an
  orphaned tree); otherwise valid. The plan's `QuestionDisagreesWithCandidate`
  (`cli/research/src/topic/tree.rs:190-217`) catches a disagreeing note only
  while the run's claims keep its stem. On `--start` the claims are empty, so
  an off-question root note gives its directory's index claim to the wrong
  question and the pair moves to a fresh stem (`plan.rs:965-981`); the
  orphaned rule catches that.
- `RunLedger::record` counts every pending spawn a changed offer drops as
  attempted (`ledger.rs:101-115`), so a second plan in the same run after the
  quarantine would strand never-spawned spawns behind a saturated window.
  Reading the quarantine into the first plan (`RoundInputs::with_findings_quarantined`)
  gives the post-rename plan with no second call.
- A quarantine marker claims its index for the question its file names
  (`research-adapters/src/topic.rs:185-193`), and `claimed_by_a_file`
  (`plan.rs:292-297`) takes the lowest file claim for a question, marker or
  tree alike. Recovery moves a pair to a fresh index while its marker keeps
  the old one, so a later claims-free `--start` would pull the question back
  to the marker and orphan the tree grown at the fresh index. The planner
  therefore tries a tree's claim before a marker's: a tree holds research, a
  marker only reserves an index. Today step 5 makes a marker only at its
  pair's own stem, so nothing relies on a marker outranking a tree except
  `the_lowest_index_a_marker_or_levels_directory_holds_wins` (`plan.rs:844`),
  whose fixture orphans `04-a-openalex.levels/` under the current rule.
- A set-aside finding claims its index as its marker will, for the question
  it answers. One failing validation claims for no question, where its real
  marker may name one; this run's claims pin its stem, and the tree rule
  keeps a later plan from pulling the question back.
- ⚠️ A compose pair whose `.levels/` directory survives keeps its index: the
  directory is a file claim (`plan.rs:116-121`), so an invalid composed
  finding sits at the offered path and both the report's quarantine and the
  offered-path rule name it. The report's quarantine runs before any step 3
  and the offered-path rule acts only on a file that still exists, so it is
  renamed once. The same holds whenever a file claim holds the question's
  index, including a quarantine marker from an earlier failure
  (`claimed_by_a_file`, `plan.rs:292-297`). The "offered at a fresh index"
  acceptance criterion holds for a single-pass pair with no such claim; its
  test uses depth 1 and no markers.
- `RunLedger::from_parts` (`cli/research/src/conduct/ledger.rs:116-127`) and
  `PendingBatch::new` build a replaced ledger in unit tests.
- `research` is a pinned crate (`tasks/public_api.py:15-30`); new public types
  need `mise run public-api:update`, snapshot at
  `cli/research/tests/fixtures/public-api.txt`. Helpers only the crate calls
  are `pub(crate)`, as in `conduct/concurrency.rs`.

## What We're NOT Doing

- Reporting spawns from earlier acknowledged batches (`attempted`): the report
  covers the last offered batch only.
- Reporting writes that land after `--start` from the earlier run's agents
  still in flight; 0308 owns them, and they stay under Unexpected notes.
- Quarantining invalid findings outside the replaced batch (hand edits,
  leftovers behind a corrupt ledger), or unasked findings in it.
- Quarantining an orphaned `.levels/` directory. Its notes are reported
  invalid, but its pair is offered at another stem, so step 3 never reaches
  them; the directory holds its index and counts towards no `finding_count`.
- Re-checking a quarantine path before renaming it. An earlier run still in
  flight could rewrite a listed finding between `--start` and the rename; the
  marker keeps its bytes and the pair is researched again. This mid-edit case
  is not one of the after-start writes 0308 owns.
- Renaming any file from the CLI; the skill, or a hand run, keeps every
  quarantine.
- Carrying the quarantine list past its one plan. A recovering run that stops
  between `--start` returning and the renames, or a hand run that skips them,
  leaves the list unrecorded: the next `--start` reports the new run's batch.
  A leftover that fails validation stays counted in `finding_count`; a
  misdirected one is counted as the answer to the question it names, and the
  next `--run` reports it under `unexpected`, because the ledger's `seen`
  never held it. Carrying the list would need a new ledger field.
- Recognising a usage limit, or any cause, in the next run.
- Reasons for an invalid finding: finding validation discards its violations
  (`cli/research-adapters/src/topic.rs:222-248`).
- Changing the ledger's on-disk schema. The report reads `run`, `pending` and
  `claims`, but decoding (`cli/research-adapters/src/conduct/ledger.rs:140-168`)
  needs every current field in its current encoding and ignores unknown ones.
  A later change, as 0307 plans, must keep existing fields' encoding (or keep
  decoding the old one) and default new fields when absent, for a ledger
  written before it to keep yielding the report.

## Implementation Approach

Inside out, each phase mergeable alone:

1. The domain classification and the quarantine reading of round inputs,
   unit-tested and unused.
2. The CLI emits `replaced_run` additively; the old notice stays and the plan
   is unchanged, so a skill that ignores the key loses nothing.
3. In one change, the CLI plans with the listed findings read as quarantined
   and moves the notice out of `warnings` to stderr alone, and the skill
   shows the report and renames the listed findings. No release plans around
   a quarantine nobody performs, and none shows neither signal.
4. The recovery row and the docs-site wording.

Every phase is red-green-refactor: the failing test lands first in each step
below. Guards, which pin behaviour the phase must not change, are listed
apart and pass before and after.

## Phase 1: Classify the replaced run's last batch

### Overview

`OutlineItem` gains both directions of the stem rule; the planner prefers a
tree to a marker for a question's index; `RoundPlan` answers whether it
rejects a note; `RoundInputs` reads findings as their markers;
`research::conduct::replaced_run` judges a replaced batch's findings, reads
the inputs as quarantined, and judges its nodes against the caller's one
plan.

### Changes Required:

#### 1. The stem rule in both directions

**File**: `cli/research/src/topic/outline.rs`
**Changes**: tests first:

- `an_item_allocates_its_stem_from_its_slug_and_profile`
- `an_item_names_the_pair_whose_stem_it_rebuilds`
- `an_item_names_no_pair_for_a_stem_under_a_profile_it_lacks`
- `an_item_asks_a_pair_only_under_a_profile_it_names`

Then, `pub(crate)` since only the crate calls them:

```rust
impl OutlineItem {
    pub(crate) fn stem_at(&self, index: u32, profile: &str) -> Option<Stem> {
        let slug = QuestionSlug::from(self.question.as_str());
        Stem::allocated(index, slug.as_str(), profile)
    }

    pub(crate) fn pair_at(&self, stem: &Stem) -> Option<Pair> {
        self.profiles()
            .into_iter()
            .find(|profile| {
                self.stem_at(stem.index(), profile).as_ref() == Some(stem)
            })
            .map(|profile| self.pair(profile))
    }
}
```

Refactor, with the planner tests as the net: `Planner::plan_item`
(`cli/research/src/topic/plan.rs:209-215`) allocates through
`item.stem_at(index, profile)`.

#### 2. A tree outranks a marker for its question's index

**File**: `cli/research/src/topic/plan.rs`
**Changes**: test first. `the_lowest_index_a_marker_or_levels_directory_holds_wins`
(`:844-852`) pins a marker at `02` beating `04-a-openalex.levels/`, which
plans `02-a-openalex` and orphans that tree. It becomes
`a_levels_directory_outranks_a_marker_for_its_question`, expecting
`["04-a-web", "04-a-openalex"]` in both halves. Beside it, two new guards
pin the order within each kind: `the_lowest_index_among_markers_wins` and
`the_lowest_index_among_levels_directories_wins`. Then:

- `RoundInputs::file_claims(&self, unicode) -> Vec<IndexClaim>`
  (`pub(crate)`): the quarantine markers' claims and each `.levels/`
  directory's, the expression `Planner::new` builds today (`:114-119`).
  `Planner::new` calls it for the index floor.
- `Planner` keeps the two kinds apart, and `claim_index` (`:279-309`) tries
  a `.levels/` directory's claim before a marker's: the run's claim, then a
  retained finding's index, then a tree's, then a marker's, then a fresh
  one. A tree holds research; a marker only reserves an index.

#### 3. Plan rejection and quarantine reading

**File**: `cli/research/src/topic/plan.rs`
**Changes**: tests first:

- `a_plan_rejects_only_the_node_it_lists_rejected` (lineage `1` rejected in
  one pair beside an accepted lineage `1` in another, and `2-1` rejected
  beside an accepted `2-2` in one pair)
- `a_misdirected_finding_read_as_quarantined_plans_as_its_marker_would`: a
  valid `02-b-web.md` naming `A?` plans exactly as the same inputs with
  `IndexClaim::quarantined(".02-b-web.md.invalid", Some("A?"), …)` in place
  of the finding.
- `a_misdirected_finding_below_its_named_questions_tree_leaves_it_on_its_tree`:
  `01-b-web.md` naming `A?` with `02-a-web.levels/` present; `A?` stays at
  `02-a-web`.
- `a_finding_failing_validation_read_as_quarantined_holds_its_index_for_no_question`
- `reading_findings_as_quarantined_leaves_every_other_finding`

Then:

```rust
impl RoundPlan {
    pub(crate) fn rejects(&self, at: &NoteRef) -> bool {
        self.pairs
            .iter()
            .filter(|pair| pair.stem == at.stem)
            .any(|pair| match &pair.stage {
                Stage::ResearchNodes(nodes) => nodes.iter().any(|node| {
                    node.lineage == at.lineage && node.rejected.is_some()
                }),
                Stage::SinglePass | Stage::Compose(_) => false,
            })
    }
}

impl RoundInputs {
    pub(crate) fn with_findings_quarantined(
        mut self,
        stems: &[Stem],
        unicode: &dyn UnicodeText,
    ) -> Self {
        let (set_aside, kept): (Vec<Finding>, Vec<Finding>) =
            self.findings.into_iter().partition(|finding| {
                finding.stem().is_some_and(|stem| stems.contains(stem))
            });
        self.findings = kept;
        self.quarantined.extend(
            set_aside
                .iter()
                .filter_map(|finding| finding.marker_claim(unicode)),
        );
        self
    }
}
```

**File**: `cli/research/src/topic/evidence.rs`
**Changes**: tests first, `a_finding_claims_its_index_for_the_question_it_answers`,
`a_levels_directory_holds_a_lineage_it_noted_or_refused` and
`a_levels_directory_refuses_only_the_lineages_it_rejected`; then:

```rust
impl Finding {
    pub(crate) fn marker_claim(
        &self,
        unicode: &dyn UnicodeText,
    ) -> Option<IndexClaim> {
        let named = self.answer().map(|answer| answer.pair.question.as_str());
        Some(IndexClaim::new(self.index()?, named, unicode))
    }
}

impl LevelsDirectory {
    pub(crate) fn refused(&self, lineage: &Lineage) -> bool {
        self.rejected.contains_key(lineage)
    }

    pub(crate) fn holds(&self, lineage: &Lineage) -> bool {
        self.refused(lineage)
            || self.notes.iter().any(|note| note.lineage == *lineage)
    }
}
```

`marker_claim` mirrors the adapter's reading of a real marker
(`research-adapters/src/topic.rs:185-193`) for a finding that validates. One
failing validation claims for no question, where its real marker may name
one; this run's claims pin its stem, and the tree rule above keeps a later
claims-free plan from pulling the question back.

#### 4. Module registration

**File**: `cli/research/src/conduct.rs`
**Changes**: add `pub mod replaced_run;`.

#### 5. The classification

**File**: `cli/research/src/conduct/replaced_run.rs` (new)
**Changes**: tests first, in the module's `#[cfg(test)]` block, built from
`Finding::retained`, `Finding::invalid`, `LevelsDirectory::new`, and
`RunLedger::from_parts` with the claims the earlier run held, as
`observed.rs:52-133` does. Each test judges findings, reads the inputs as
quarantined, plans once with `RoundPlan::of(&inputs, &FakeUnicode)`, and
reports, as the CLI will:

- `a_pair_whose_finding_answers_its_owning_pair_is_valid`
- `a_pair_whose_finding_fails_validation_is_invalid`
- `a_pair_whose_finding_names_a_question_no_outline_pair_has_is_invalid`
- `a_pair_whose_finding_names_a_profile_its_item_lacks_is_invalid`
- `a_pair_whose_finding_answers_another_outline_pair_is_invalid`
- `a_pair_whose_finding_differs_from_its_question_only_in_normalisation_is_valid`
  (full-width `？` and folded whitespace under `FakeUnicode`)
- `a_pair_sharing_its_slug_with_another_is_judged_against_the_question_its_run_claimed`
  (two CJK questions at `01-focus-area-web` and `02-focus-area-web`; the
  finding at `02` answering the first is invalid)
- `a_pair_with_no_finding_is_unwritten`
- `a_valid_finding_whose_question_left_the_outline_is_unasked_with_the_pair_it_names`
- `a_valid_finding_whose_profile_left_its_item_is_unasked`
- `a_finding_at_an_unowned_stem_answering_an_outline_pair_is_invalid`
- `a_finding_failing_validation_at_an_unowned_stem_is_invalid`
- `an_unwritten_spawn_whose_question_left_the_outline_is_unasked`
- `a_node_whose_note_the_plan_accepts_is_valid`
- `a_node_whose_note_was_refused_is_invalid`
- `a_node_whose_note_the_plan_rejects_against_its_candidate_is_invalid`
- `a_node_with_no_note_is_unwritten`
- `a_node_no_longer_planned_is_classed_by_its_note` (pair moved to compose)
- `a_node_whose_tree_grew_from_another_question_is_invalid` (root note names
  `Other?`; claims hold `A?` at the stem's index)
- `a_deeper_node_in_an_orphaned_tree_is_invalid`
- `a_lineage_with_no_note_in_an_orphaned_tree_is_unwritten` (including a tree
  whose root question comes from a quarantined root marker)
- `a_node_whose_tree_root_matches_only_after_normalisation_is_valid`
- `a_node_in_a_tree_with_no_root_question_is_classed_by_its_note`
- `a_node_whose_question_left_the_outline_is_unasked_whatever_its_note`
  (accepted, refused and missing)
- `a_misdirected_finding_does_not_hide_the_rejections_of_the_pair_it_names`
  (the finding at `02-b-web` answers `A?`, whose `2-1` note disagrees with
  its candidate; `01-a-web:2-1` is invalid once the inputs are read as
  quarantined)
- `each_leftover_carries_its_owning_pair`
- `each_class_keeps_the_order_of_the_last_batch` (batch built as `03-c`,
  `01-a`, `02-b:1`, so a sorted class fails)
- `an_empty_last_batch_names_its_run_and_classes_nothing`
- `only_invalid_findings_are_quarantined` (an invalid node and an unasked
  finding name none)
- `the_report_quarantines_what_the_batch_read_as_quarantined`

Then the minimum to pass:

```rust
//! What the run whose ledger a starting run replaced left of the batch it
//! last offered: each spawn classed by the one file it writes.

use crate::conduct::ledger::RunId;
use crate::conduct::ledger::RunLedger;
use crate::conduct::spawn::SpawnRef;
use crate::topic::claims::ClaimedIndexes;
use crate::topic::evidence::Finding;
use crate::topic::evidence::LevelsDirectory;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::outline::Outline;
use crate::topic::outline::Pair;
use crate::topic::plan::RoundInputs;
use crate::topic::plan::RoundPlan;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::UnicodeText;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedBatch {
    run: RunId,
    quarantine: Vec<Stem>,
    spawns: Vec<Pending>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Judged(Verdict),
    AwaitingPlan { at: NoteRef, owner: Option<Pair> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict {
    leftover: Leftover,
    outcome: SpawnOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedRun {
    run: RunId,
    valid: Vec<Leftover>,
    invalid: Vec<Leftover>,
    unwritten: Vec<Leftover>,
    unasked: Vec<Leftover>,
    quarantine: Vec<Stem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    spawn: SpawnRef,
    pair: Option<Pair>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnOutcome {
    Valid,
    Invalid,
    Unwritten,
    Unasked,
}

impl ReplacedBatch {
    #[must_use]
    pub fn judging_findings(
        replaced: &RunLedger,
        inputs: &RoundInputs,
        unicode: &dyn UnicodeText,
    ) -> Self {
        let judge = Judge {
            outline: &inputs.outline,
            claims: replaced.claims(),
            unicode,
        };
        let mut quarantine = Vec::new();
        let spawns = replaced
            .memory()
            .pending
            .spawns()
            .iter()
            .map(|spawn| match spawn {
                SpawnRef::Pair(stem) => {
                    let verdict = judge.finding(stem, inputs);
                    if verdict.outcome == SpawnOutcome::Invalid {
                        quarantine.push(stem.clone());
                    }
                    Pending::Judged(verdict)
                }
                SpawnRef::Node(at) => Pending::AwaitingPlan {
                    at: at.clone(),
                    owner: judge.owner(&at.stem),
                },
            })
            .collect();
        Self {
            run: replaced.run().clone(),
            quarantine,
            spawns,
        }
    }

    #[must_use]
    pub fn quarantining(
        &self,
        inputs: RoundInputs,
        unicode: &dyn UnicodeText,
    ) -> RoundInputs {
        inputs.with_findings_quarantined(&self.quarantine, unicode)
    }

    #[must_use]
    pub fn report(
        self,
        inputs: &RoundInputs,
        plan: &RoundPlan,
        unicode: &dyn UnicodeText,
    ) -> ReplacedRun {
        let mut report = ReplacedRun {
            run: self.run,
            valid: Vec::new(),
            invalid: Vec::new(),
            unwritten: Vec::new(),
            unasked: Vec::new(),
            quarantine: self.quarantine,
        };
        for pending in self.spawns {
            let verdict = match pending {
                Pending::Judged(verdict) => verdict,
                Pending::AwaitingPlan { at, owner } => {
                    let outcome =
                        note_outcome(&at, owner.as_ref(), inputs, plan, unicode);
                    Verdict {
                        leftover: Leftover {
                            spawn: SpawnRef::Node(at),
                            pair: owner,
                        },
                        outcome,
                    }
                }
            };
            report.class_of(verdict.outcome).push(verdict.leftover);
        }
        report
    }
}

impl ReplacedRun {
    #[must_use]
    pub const fn run(&self) -> &RunId {
        &self.run
    }

    #[must_use]
    pub fn valid(&self) -> &[Leftover] {
        &self.valid
    }

    #[must_use]
    pub fn invalid(&self) -> &[Leftover] {
        &self.invalid
    }

    #[must_use]
    pub fn unwritten(&self) -> &[Leftover] {
        &self.unwritten
    }

    #[must_use]
    pub fn unasked(&self) -> &[Leftover] {
        &self.unasked
    }

    #[must_use]
    pub fn quarantine(&self) -> &[Stem] {
        &self.quarantine
    }

    const fn class_of(&mut self, outcome: SpawnOutcome) -> &mut Vec<Leftover> {
        match outcome {
            SpawnOutcome::Valid => &mut self.valid,
            SpawnOutcome::Invalid => &mut self.invalid,
            SpawnOutcome::Unwritten => &mut self.unwritten,
            SpawnOutcome::Unasked => &mut self.unasked,
        }
    }
}

impl Leftover {
    #[must_use]
    pub const fn spawn(&self) -> &SpawnRef {
        &self.spawn
    }

    #[must_use]
    pub const fn pair(&self) -> Option<&Pair> {
        self.pair.as_ref()
    }
}

struct Judge<'a> {
    outline: &'a Outline,
    claims: &'a ClaimedIndexes,
    unicode: &'a dyn UnicodeText,
}

impl Judge<'_> {
    fn owner(&self, stem: &Stem) -> Option<Pair> {
        self.outline
            .items
            .iter()
            .filter(|item| {
                let question = NormalisedQuestion::of(&item.question, self.unicode);
                self.claims.index_for(&question) == Some(stem.index())
            })
            .find_map(|item| item.pair_at(stem))
    }

    fn finding(&self, stem: &Stem, inputs: &RoundInputs) -> Verdict {
        let finding = inputs
            .findings
            .iter()
            .find(|finding| finding.stem() == Some(stem));
        let (outcome, pair) = self.owner(stem).map_or_else(
            || self.unowned_finding(finding),
            |owner| (self.owned_finding(finding, &owner), Some(owner)),
        );
        Verdict {
            leftover: Leftover {
                spawn: SpawnRef::Pair(stem.clone()),
                pair,
            },
            outcome,
        }
    }

    fn owned_finding(
        &self,
        finding: Option<&Finding>,
        owner: &Pair,
    ) -> SpawnOutcome {
        let question = NormalisedQuestion::of(&owner.question, self.unicode);
        match finding {
            None => SpawnOutcome::Unwritten,
            Some(finding) if finding.answers(&question, &owner.profile) => {
                SpawnOutcome::Valid
            }
            Some(_) => SpawnOutcome::Invalid,
        }
    }

    fn unowned_finding(
        &self,
        finding: Option<&Finding>,
    ) -> (SpawnOutcome, Option<Pair>) {
        match finding.map(Finding::answer) {
            None => (SpawnOutcome::Unasked, None),
            Some(None) => (SpawnOutcome::Invalid, None),
            Some(Some(answer)) if self.asks(&answer.pair) => {
                (SpawnOutcome::Invalid, None)
            }
            Some(Some(answer)) => {
                (SpawnOutcome::Unasked, Some(answer.pair.clone()))
            }
        }
    }

    fn asks(&self, pair: &Pair) -> bool {
        let question = NormalisedQuestion::of(&pair.question, self.unicode);
        self.outline.items.iter().any(|item| {
            NormalisedQuestion::of(&item.question, self.unicode) == question
                && item.profiles().contains(&pair.profile.as_str())
        })
    }
}

fn note_outcome(
    at: &NoteRef,
    owner: Option<&Pair>,
    inputs: &RoundInputs,
    plan: &RoundPlan,
    unicode: &dyn UnicodeText,
) -> SpawnOutcome {
    let Some(owner) = owner else {
        return SpawnOutcome::Unasked;
    };
    let Some(directory) = inputs.levels.iter().find(|directory| {
        *directory.stem() == at.stem && directory.holds(&at.lineage)
    }) else {
        return SpawnOutcome::Unwritten;
    };
    if directory.refused(&at.lineage)
        || plan.rejects(at)
        || !grew_from(directory, owner, unicode)
    {
        SpawnOutcome::Invalid
    } else {
        SpawnOutcome::Valid
    }
}

fn grew_from(
    directory: &LevelsDirectory,
    owner: &Pair,
    unicode: &dyn UnicodeText,
) -> bool {
    directory.root_question().is_none_or(|root| {
        NormalisedQuestion::of(root, unicode)
            == NormalisedQuestion::of(&owner.question, unicode)
    })
}
```

The method names carry the order: `judging_findings`, then `quarantining`
the inputs, then `report` against the plan drawn from them. Phase 2 skips
the middle step, and `report` judges nodes against whatever plan the caller
emits.

#### 6. Public API snapshot

**File**: `cli/research/tests/fixtures/public-api.txt`
**Changes**: regenerate with `mise run public-api:update`; it gains
`ReplacedBatch`, `ReplacedRun` and `Leftover`. Every other addition is
`pub(crate)`.

### Success Criteria:

#### Automated Verification:

- [x] The new unit tests, and the reversed marker-or-tree test, fail before
      each body exists, then pass: `cargo test -p research` (from `cli/`)
- [x] Guard: the planner's other tests pass unchanged after `stem_at` and
      `file_claims` replace the inline allocation and claim list
- [x] Public API pin matches: `mise run public-api:check`
- [x] Domain dependency rule holds: `mise run pup:check`
- [x] Workspace lint and format: `mise run cli:check`
- [x] Full local CI: `mise run`

#### Manual Verification:

- [x] Test names read as the classification rules in the work item, with no
      comment needed to explain any of them

---

## Phase 2: Emit `replaced_run` from `outstanding --start`

### Overview

`run_outstanding` keeps a readable replaced ledger, judges its findings,
plans as today, judges its nodes against that plan, and renders
`replaced_run`. The plan itself and the existing notice are unchanged until
Phase 3.

### Changes Required:

#### 1. CLI integration tests first

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**: add, beside `starting_over_a_stale_ledger_replaces_it_with_a_warning`.
Every test below asserts `replaced_run` and fails first:

- `starting_over_a_stale_ledger_reports_its_last_batch_by_class`: depth 2,
  `--concurrency 5`, outline `C?`, `D?`, `E?`, `A?`, `B?`. Seed complete
  one-note trees at `01-a-web` and `02-b-web`, so the allocation floor leaves
  `03`–`05` for the rest. Assert the first `--start` offers, in outline
  order, `03-c-web:1`, `04-d-web:1`, `05-e-web:1`, `01-a-web`, `02-b-web`. Then write a valid finding for `02-b-web`, a schema-invalid
  finding for `01-a-web`, an accepted root note for `03-c-web`, a refused
  root note for `04-d-web` (level mismatch, as
  `a_refused_note_is_reported_on_its_node_with_its_reason` builds it), and
  nothing for `05-e-web`. Assert on the second `--start`:

  ```rust
  assert_eq!(
      plan["replaced_run"],
      json!({
          "run": first,
          "valid": [left("03-c-web:1", "C?"), left("02-b-web", "B?")],
          "invalid": [left("04-d-web:1", "D?"), left("01-a-web", "A?")],
          "unwritten": [left("05-e-web:1", "E?")],
          "unasked": [],
          "quarantine": [absolute(&project, &format!("{TOPICS}/s/findings/01-a-web.md"))?],
      })
  );
  ```

  `left(spawn, question)` is a test helper building
  `{"spawn", "question", "profile": "web"}`. The refused node `04-d-web:1` is
  invalid but names no quarantine path. The offer order is not lexical, so
  the literals also catch a class that sorts its entries. Also assert the
  second plan's top-level keys are the first plan's plus `replaced_run`, so
  the key leaves the rest of the plan's shape alone.
- `an_invalid_or_misdirected_finding_from_a_replaced_batch_is_listed_for_quarantine`:
  depth 1, `A?` and `B?`. After the first `--start`, write `01-a-web.md`
  failing validation (a required field missing) while its frontmatter still
  parses with `question: A?`, and `02-b-web.md` valid but naming `Z?`.
  Assert both spawns are `invalid` with their owning pairs `A?` and `B?`,
  both absolute paths are in `quarantine`, and `spawns_of` is exactly
  `["03-a-web", "04-b-web"]`.
- `a_finding_answering_a_sibling_pair_is_listed_for_quarantine`: depth 1,
  `A?` and `B?`. After the first `--start`, write `02-b-web.md` valid but
  naming `A?`, and nothing at `01-a-web`. Assert `02-b-web` is `invalid`, its
  absolute path is in `quarantine`, and `01-a-web` is `unwritten`.
- `a_finding_whose_question_left_the_outline_is_left_in_place`: depth 1,
  `A?`. After the first `--start`, write a valid `01-a-web.md`, then reword
  the outline item to `A2?`. Assert `replaced_run.unasked ==
  [{"spawn": "01-a-web", "question": "A?", "profile": "web"}]` and
  `quarantine == []`.
- `an_unwritten_spawn_whose_question_left_the_outline_names_no_pair`: as
  above with nothing written; assert `replaced_run.unasked ==
  [{"spawn": "01-a-web", "question": null, "profile": null}]`.
- `a_valid_finding_from_a_replaced_batch_counts_its_pair_answered`: depth 1,
  `A?`; write a valid `01-a-web.md`; assert `replaced_run.valid ==
  [left("01-a-web", "A?")]`, `pairs == []` and the item is `complete`.
- `a_stale_ledger_with_an_empty_last_batch_still_names_its_run`: write a
  ledger with `"pending": {"number": 0, "spawns": []}` through a first
  `--start` on a set with nothing outstanding; assert `replaced_run` equals
  `{"run": first, "valid": [], "invalid": [], "unwritten": [], "unasked": [],
  "quarantine": []}`.
- `a_stale_ledger_with_fields_it_does_not_know_still_reports_its_last_batch`:
  after a first `--start`, add an unknown top-level key and an unknown key
  inside `pending` to the stored ledger; assert `replaced_run` names the run
  and classes its batch.
- `a_stale_ledger_in_todays_shape_reports_its_last_batch`: write a literal
  ledger fixture in today's exact encoding, not one this binary produced,
  its `claims` keyed by normalised question (NFKC with whitespace folded,
  case kept, so `"A?"`);
  assert `replaced_run` names its run and classes its batch.

Guards, green before and after:

- `no_replaced_run_without_a_ledger_to_replace`: a first `--start`, and a
  `--start` after `end-run`, each lack the key.
- `no_replaced_run_on_a_plain_or_continued_plan`: a plain plan and a `--run`
  plan lack the key.
- Extend `starting_over_a_corrupt_ledger_replaces_it_with_a_warning` to
  assert the key is absent.
- `outline_and_counts_match_disk_after_a_replaced_batch`: depth 1, `A?` and
  `B?`; after a first `--start`, write a valid `01-a-web.md` only, `--start`
  again, `end-run`, then a plain plan; assert `items` shows `A?` complete and
  `B?` not. This guards the step 7 tick, which reads exactly this plan.

#### 2. Keep the replaced ledger

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: replace `replaced_ledger_notice` with a reading that keeps the
ledger:

```rust
enum ReplacedLedger {
    Absent,
    Readable(RunLedger),
    Corrupt,
}

fn replaced_ledger<F: FileReader>(
    set_root: &Path,
    fs: &F,
) -> Result<ReplacedLedger, String> {
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(None) => Ok(ReplacedLedger::Absent),
        Ok(Some(stored)) => Ok(ReplacedLedger::Readable(stored)),
        Err(LedgerError::Corrupt) => Ok(ReplacedLedger::Corrupt),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

impl ReplacedLedger {
    fn notice(&self) -> Option<String> {
        const CONSEQUENCE: &str = "stops at its next batch, and notes its \
                                   last batch writes may show as unexpected";
        match self {
            Self::Absent => None,
            Self::Readable(stored) => Some(format!(
                "replaced the ledger of run {}; if that run is still going \
                 it {CONSEQUENCE}",
                stored.run()
            )),
            Self::Corrupt => Some(format!(
                "replaced a corrupt run ledger; if its run is still going \
                 it {CONSEQUENCE}"
            )),
        }
    }

    fn judging_findings(&self, inputs: &RoundInputs) -> Option<ReplacedBatch> {
        match self {
            Self::Readable(stored) => Some(ReplacedBatch::judging_findings(
                stored,
                inputs,
                &UnicodeTables,
            )),
            Self::Absent | Self::Corrupt => None,
        }
    }
}
```

The Start arm's concurrency notices move into a `start_notices(concurrency)`
helper, and the existing mode match returns the replaced ledger beside the
continued one, so no further match over `RunMode` appears:

```rust
let (continued, replaced) = match &mode {
    RunMode::Continue { run, spawned } => (
        Some(continued_ledger(set_root, run, *spawned, fs)?),
        ReplacedLedger::Absent,
    ),
    RunMode::Start(_) => {
        notices.extend(start_notices(concurrency));
        let replaced = replaced_ledger(set_root, fs)?;
        notices.extend(replaced.notice());
        (None, replaced)
    }
    RunMode::Query => (None, ReplacedLedger::Absent),
};
```

After `read_round_inputs`, `let replaced_batch =
replaced.judging_findings(&reading.inputs);`; after `RoundPlan::of`, `let
replaced_run = replaced_batch.map(|replaced_batch|
replaced_batch.report(&reading.inputs, &plan, &UnicodeTables));`. Nodes are judged
against the plan the CLI emits. The ledger write stays ahead of `render()`.

#### 3. Render the key

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: add `replaced_run: Option<&'a ReplacedRun>` to `PlanOutput`; in
`render()` after the ledger keys, `if let Some(replaced) = self.replaced_run
{ rendered["replaced_run"] = self.replaced_run(replaced); }`, with a helper
beside `fn pair`:

```rust
fn replaced_run(&self, replaced: &ReplacedRun) -> Value {
    let entries = |leftovers: &[Leftover]| -> Vec<Value> {
        leftovers
            .iter()
            .map(|leftover| {
                json!({
                    "spawn": leftover.spawn().to_string(),
                    "question": leftover.pair().map(|pair| &pair.question),
                    "profile": leftover.pair().map(|pair| &pair.profile),
                })
            })
            .collect()
    };
    json!({
        "run": replaced.run().to_string(),
        "valid": entries(replaced.valid()),
        "invalid": entries(replaced.invalid()),
        "unwritten": entries(replaced.unwritten()),
        "unasked": entries(replaced.unasked()),
        "quarantine": replaced
            .quarantine()
            .iter()
            .map(|stem| self.absolute(&stem.finding_path()))
            .collect::<Vec<_>>(),
    })
}
```

From the ledger only `RunId`, `SpawnRef` and claimed indexes shape the
output; each question and profile comes from the outline or a validated
finding, so strict ledger decoding still keeps the file's free text out of
it.

#### 4. Docs and changelog

**File**: `docs-site/src/content/docs/research.md`
**Changes**: in the run-flags paragraph (`:503-510`), add that `--start` over
another run's readable ledger also carries `replaced_run`: that run's `run`,
its last offered batch split into `valid`, `invalid`, `unwritten` and
`unasked`, each entry a spawn ref with the outline question and profile that
own its stem (or, for an unasked finding, the pair it names), and the
absolute `quarantine` paths of its invalid findings.

**File**: `CHANGELOG.md`
**Changes**: `outstanding` has not shipped, so extend its existing
`[Unreleased] ### Added` entry (`:122-133`) rather than adding one: `--start`
over another run's readable ledger reports that run's last batch as
`replaced_run`.

### Success Criteria:

#### Automated Verification:

- [x] Each test asserting `replaced_run` fails before the CLI change, then
      passes: `cargo test -p research-cli --test topic_outstanding` (from
      `cli/`)
- [x] Guards (key absence, the tick plan) pass before and after the change
- [x] The existing stale-ledger and notice-order tests still pass unchanged
- [x] Workspace lint and format: `mise run cli:check`
- [x] Docs lane: `mise run docs:check`
- [x] Full local CI: `mise run`

#### Manual Verification:

- [x] A hand `--start` over a stale ledger in a scratch set prints
      `replaced_run` with the expected classes

---

## Phase 3: The interrupted-run report in `conduct`

### Overview

The CLI plans with the listed findings read as quarantined and moves the
readable-ledger notice to stderr alone; the skill shows the report before
the first batch and renames the listed findings straight after it.

### Changes Required:

#### 1. Structure tests first

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**:

- Update `test_research_topic_passes_the_concurrency_flag_through` to expect
  "a corrupt run ledger or a clamped concurrency".
- `test_conduct_reads_the_replaced_run`: the fields-read list names
  `replaced_run` with `run`, `valid`, `invalid`, `unwritten`, `unasked` and
  `quarantine`, each entry's `spawn`, `question` and `profile`, and the
  opaque-data sentence covers its refs and questions.
- `test_conduct_ignores_stderr`: the fields-read prose contains "ignore
  stderr, which repeats the trims and, after a replaced ledger, names the
  replaced run for a hand run".
- `test_the_report_groups_each_class_under_its_heading`: the report
  instruction pairs "written by run {run}; kept" with `valid`, "written but
  invalid; offered again" with `invalid`, "not written; offered again" with
  `unwritten`, and "for a question the outline no longer asks; left in
  place" with `unasked`, marks each `invalid` entry whose finding is in
  `quarantine` with "(quarantined)", and reads an `invalid` entry with a
  `null` question as "(quarantined; no outline pair asks it now)".
- `test_conduct_opens_the_report_with_the_fixed_sentence`: `_prose` of the
  conduct section contains "run {run} did not end; if it is still going, it
  stops at its next batch, and what it writes from now on may show as
  unexpected".
- `test_the_report_never_asserts_the_earlier_run_stopped`: the report
  instruction, cut out with `_section`, contains none of "stopped",
  "crashed", "died", "was interrupted", "has ended".
- `test_the_report_shows_the_opening_alone_for_an_empty_batch`: it states
  that empty classes are omitted and the opening sentence still shows.
- `test_the_report_quarantines_listed_findings_before_any_batch`: the report
  instruction names `quarantine` and `.<name>.invalid`, contains "whether or
  not the plan offers a spawn", and sits before the step 3 heading; step 3's
  offered-path bullet still says "an existing `path`".
- `test_a_failed_quarantine_lists_what_is_left_and_stops_conduct`: the
  report instruction contains "attempt every rename", "each path still not
  renamed" and "stop `conduct`".

#### 2. Plan with the listed findings quarantined

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**, tests first:

- Extend `a_finding_answering_a_sibling_pair_is_listed_for_quarantine`: the
  second `--start` offers exactly `["02-a-web", "03-b-web"]` at `batch` `0`.
- `a_quarantine_never_strands_an_offered_spawn`: the sibling fixture, with
  the first `--start` at the default concurrency so both `01-a-web` and
  `02-b-web` are in the replaced batch, and the second at `--concurrency 1`.
  The second `--start` offers `["02-a-web"]`; rename `02-b-web.md` as the
  report would, write `02-a-web.md`, and plan `--run {new} --spawned 0`.
  Assert it offers `["03-b-web"]` and `unfinished` is empty.
- `the_first_plan_after_a_replaced_batch_matches_the_plan_once_quarantined`:
  over the sibling fixture, rename each `quarantine` path to its marker and
  plan `--run {new}`; assert its `pairs` equal the second `--start`'s.
  Before the change the `--start` offers `["03-b-web"]` alone, so it fails
  first.

Guards, green before and after:

- `quarantined_leftovers_keep_their_fresh_stems_for_the_rest_of_the_run`:
  continue the misdirected-finding fixture by renaming both files to
  `.<name>.invalid`, then plan with `--run {new}`; assert `spawns_of` is
  exactly `["03-a-web", "04-b-web"]` and `replaced_run` is absent. The
  failing-validation file's marker names `A?`, so this pins that the run's
  claim outranks the marker's. Then plan a plain round: `warnings` holds no
  unmatched entry for `Z?`, and `A?` and `B?` are both incomplete, so the
  markers no longer read as findings.

Then in `run_outstanding`, between `read_round_inputs` and `RoundPlan::of`:

```rust
let replaced_batch = replaced.judging_findings(&reading.inputs);
let inputs = match &replaced_batch {
    Some(replaced_batch) => {
        replaced_batch.quarantining(reading.inputs, &UnicodeTables)
    }
    None => reading.inputs,
};
```

`RoundPlan::of`, `Observed::of`, `replaced_batch.report` and everything
after read `inputs`.

#### 3. Skill prose

**File**: `skills/research/research-topic/SKILL.md`
**Changes**:

- First-plan warnings (`:262-267`): "report the first plan's `warnings` as
  soon as it returns — a corrupt run ledger or a clamped concurrency".
- After that sentence, the report instruction: when the first plan carries
  `replaced_run`, show before step 3 "run {run} did not end; if it is still
  going, it stops at its next batch, and what it writes from now on may show
  as unexpected", then one group per non-empty class, each entry as
  `{stem}: {question} ({profile})` or `{stem} {lineage}: {question}
  ({profile})`, as the summary's Failures section leads with the stem, or
  `{stem}` / `{stem} {lineage}` alone when `question` is `null`:
  - "written by run {run}; kept": `valid`;
  - "written but invalid; offered again": `invalid`, each entry whose finding
    is in `quarantine` marked "(quarantined)". An entry whose `question` is
    `null` reads "(quarantined; no outline pair asks it now)" and is not
    offered again;
  - "not written; offered again": `unwritten`;
  - "for a question the outline no longer asks; left in place": `unasked`.

  With every class empty, show the opening sentence alone.

  Straight after the report, whether or not the plan offers a spawn, attempt
  every rename of a `quarantine` path that still exists to `.<name>.invalid`
  beside it, under the unique-suffix rule; the plan already reads them as
  quarantined. If any rename fails, report each failure verbatim, then each
  path still not renamed with the `.<name>.invalid` name to give it by hand
  before re-running `conduct`, and stop `conduct`.
- Fields read (`:269-291`): add `replaced_run` and its six fields, and each
  entry's `spawn`, `question` and `profile`; add its refs and questions to
  the opaque-data sentence (`:292-295`). The trims sentence's "ignore the
  stderr copy" becomes "ignore stderr, which repeats the trims and, after a
  replaced ledger, names the replaced run for a hand run".
- Step 3 Clear (`:297-310`) is unchanged: the report's quarantine has already
  run, and its offered-path bullet acts only on an existing file.

#### 4. Move the readable-ledger notice to stderr alone

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: the ledger notices move to a new `ledger_wording` module beside
`plan_wording` and `concurrency_wording`. `ReplacedLedger::notice` becomes
`ledger_wording::corrupt_notice`, feeding `notices` and so both `warnings`
and stderr, with `CONSEQUENCE` inlined into its one use. The
readable-ledger line is drawn from the report, for a hand run, which also
has to perform the quarantine itself:

```rust
pub fn corrupt_notice(replaced: &ReplacedLedger) -> Option<String> {
    match replaced {
        ReplacedLedger::Corrupt => Some(
            "replaced a corrupt run ledger; if its run is still going it \
             stops at its next batch, and notes its last batch writes may \
             show as unexpected"
                .to_owned(),
        ),
        ReplacedLedger::Absent | ReplacedLedger::Readable(_) => None,
    }
}

pub fn replaced_notice(replaced: &ReplacedRun) -> String {
    let notice = format!(
        "replaced the ledger of run {}; if it is still going it stops at \
         its next batch",
        replaced.run()
    );
    if replaced.quarantine().is_empty() {
        notice
    } else {
        format!(
            "{notice}; rename each replaced_run quarantine path to \
             .<name>.invalid beside it, never deleting it, before acting \
             on this plan"
        )
    }
}
```

`ReplacedLedger` keeps only its states and `judging_findings`. The stderr
loop chains `replaced_run.as_ref().map(ledger_wording::replaced_notice)` after
`notices` and before the trims.

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**, tests first:

- `starting_over_a_stale_ledger_replaces_it_with_a_warning` becomes
  `starting_over_a_stale_ledger_reports_it_without_a_warning`: `warnings` is
  `[]`, stderr is exactly the short `replaced_notice` line for `{first}`
  (its batch quarantines nothing), and the stored ledger is the new run's.
- `a_stale_ledger_with_findings_to_quarantine_asks_a_hand_run_to_rename_them`:
  over the misdirected fixture, stderr is the long line.
- `stderr_orders_a_clamp_a_replaced_ledger_and_a_trim`: a readable stale
  ledger, a clamped concurrency and an over-cap note; assert the stderr lines
  in that order.
- Remove `replaced_warning`.

Guard, green before and after:

- `concurrency_warnings_precede_a_replaced_ledger_notice` becomes
  `concurrency_warnings_precede_a_corrupt_ledger_notice`, over a corrupt
  ledger.

#### 5. Docs and changelog

**File**: `docs-site/src/content/docs/research.md`
**Changes**: state that a readable replaced ledger yields `replaced_run`, a
plan drawn as if its `quarantine` findings were renamed, and a stderr line
asking a hand run to rename them, with no `warnings` entry; a corrupt one
yields the warning alone. Qualify the additive-only sentence (`:470-471`):
consumers ignore fields they do not know, except that on `--start` they must
rename the `replaced_run.quarantine` paths before acting on the plan.

**File**: `CHANGELOG.md`
**Changes**: extend the same `### Added` entry: `conduct` reports what an
unfinished earlier run left of its last batch and quarantines its invalid
findings, and `--start` plans as if they were quarantined.

### Success Criteria:

#### Automated Verification:

- [x] New and updated structure tests fail before the prose change, then
      pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] The extended sibling test, the stranding test, the equivalence test,
      the stale-ledger-without-warning rename, the hand-run rename line and
      the stderr-order test fail before the CLI change, then pass:
      `cargo test -p research-cli --test topic_outstanding` (from `cli/`)
- [x] Guards (fresh stems with the marker proxy, the corrupt-ledger order)
      pass before and after
- [x] Read-only CI set: `mise run check`
- [x] Full local CI: `mise run`

#### Manual Verification:

- [ ] The interrupted-run walkthrough below shows the report before the
      first batch, with its groups and no claim that the earlier run stopped
- [ ] After the report, the invalid leftover is a `.invalid` marker and the
      manifest's `finding_count` excludes it
- [ ] An empty-batch stale ledger shows the opening sentence alone

---

## Phase 4: The "unfinished run" recovery row

### Overview

The failure-reason table gains the row; the report states its next step
inline; the docs-site leftover-ledger paragraph stops asserting an
interruption and stops offering deletion as an equal choice.

### Changes Required:

#### 1. Structure tests first

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**: `_unfinished_run_row()` beside `_capacity_row()`, asserting one
row starting `| an unfinished run |`, and:

- `test_the_unfinished_run_row_names_re_running_conduct`: contains "re-run
  `conduct <slug>`" and "offers what the earlier run left unfinished".
- `test_the_unfinished_run_row_carries_the_usage_limit_step`: contains "agent
  usage limit", "after the reset time in the message", "after raising the
  limit when it gives none".
- `test_the_unfinished_run_row_is_its_own_row`: it is distinct from the
  `rate_limited` row and the capacity row, and neither of those mentions
  "usage limit".
- `test_the_report_states_the_next_step_if_this_run_stops`: the report
  instruction, cut out with `_section`, contains "if this run stops before
  it ends, re-run `conduct <slug>`" and "if this session shows an agent
  usage limit, first wait until the reset time in the message, or raise the
  limit when it gives none", and shows them only when `invalid` or
  `unwritten` is non-empty.

#### 2. The row and the report's next step

**File**: `skills/research/research-topic/SKILL.md`
**Changes**: after the capacity row (`:485`):

```markdown
| an unfinished run | re-run `conduct <slug>`, which offers what the earlier run left unfinished; when the earlier session showed an agent usage limit, first wait: re-run after the reset time in the message, or after raising the limit when it gives none |
```

The report instruction from Phase 3 gains, after the groups and only when
`invalid` or `unwritten` is non-empty: "this run offers the invalid and
unwritten spawns again; if this run stops before it ends, re-run `conduct
<slug>`; if this session shows an agent usage limit, first wait until the
reset time in the message, or raise the limit when it gives none". The user
never sees the table, so the report states the row's step inline, phrased
for this run.

#### 3. Docs and changelog

**File**: `docs-site/src/content/docs/research.md`
**Changes**: replace "A leftover ledger means a run was interrupted, and is
safe to delete; consider ignoring `**/.conduct-run.json`. A person clears a
leftover ledger by deleting it or by re-running `conduct`." (`:516-519`)
with "A leftover ledger means a run did not end. If its session showed an
agent usage limit, wait until the reset time in the message, or raise the
limit when it gives none. Then re-run `conduct`, which reports what that run
left of its last batch and quarantines the invalid findings it wrote.
Deleting the ledger instead skips both and leaves those findings counted.
Consider ignoring `**/.conduct-run.json`." The table is not mirrored in the
docs, so this is where a user whose session died finds the wait step.

**File**: `CHANGELOG.md`
**Changes**: in the same `### Added` entry, replace "removed on completion
and safe to delete after an interrupted run" with "removed on completion; a
leftover one means a run did not end, and re-running `conduct` recovers what
it left, where deleting the ledger would skip that", and add the
"unfinished run" recovery row.

### Success Criteria:

#### Automated Verification:

- [ ] Row and report tests fail before the row exists, then pass:
      `uv run pytest tests/unit/tasks/test_research_structure.py`
- [ ] Docs lane: `mise run docs:check`
- [ ] Full local CI: `mise run`

#### Manual Verification:

- [ ] The row reads correctly beside `rate_limited` and the capacity row in
      the rendered skill
- [ ] The rendered docs recommend re-running `conduct`, with the wait step
      first, and no longer offer deleting as an equal choice

---

## Testing Strategy

### Unit Tests:

- `topic::outline`: both directions of the stem rule.
- `topic::plan`: a tree outranks a marker for its question's index; plan
  rejection matching; findings read as quarantined plan as their markers
  would, without pulling a question off its tree.
- `topic::evidence`: a finding's marker claim; which lineages a levels
  directory holds and refuses.
- `research::conduct::replaced_run`: every class for pairs and nodes,
  ownership by claimed index under slug collisions, unowned spawns and the
  misdirected ones among them, orphaned trees and unwritten lineages in
  them, an unplanned node, order, an empty batch, and the quarantine list.

### Integration Tests:

- `topic_outstanding.rs`: the mixed batch and the plan's unchanged shape;
  misdirected, sibling-answering and invalid findings with fresh stems kept
  across the run and the markers no longer read as findings; findings and
  unwritten spawns whose question left the outline; a valid finding
  answering its pair; the empty batch; unknown ledger fields and today's
  ledger encoding; key absence with no ledger, after `end-run`, on plain and
  continued plans, and on a corrupt ledger; the first plan matching the
  post-quarantine plan; no spawn stranded at concurrency 1; both stderr
  lines and their order; the tick plan matching disk.

### Manual Testing Steps:

1. In a scratch set at depth 2, run `/accelerator:research-topic conduct
   <slug>` and stop the session after the first batch is offered and some
   agents have written.
2. Hand-edit one written finding to break its frontmatter.
3. Re-run `conduct <slug>`; confirm the report opens with the fixed sentence,
   lists the groups correctly, and appears before any spawn.
4. Confirm the broken finding is now `.<name>.invalid` and its pair is
   researched again.
5. Confirm the manifest's `finding_count` and the outline ticks match disk
   after step 7.
6. Repeat with a corrupt `.conduct-run.json`; confirm only the corrupt-ledger
   warning shows.
7. Repeat with a replaced batch whose only leftover is a finding answering a
   sibling pair, and nothing else outstanding; confirm the file is
   quarantined, the first plan offers its pairs, and they are researched.

## Performance Considerations

⏱️ Judging the replaced batch is linear in the batch times the findings,
directories and outline items already in memory, and draws no plan of its
own; there is no extra I/O.

## Migration Notes

None. The ledger format is unchanged, so a ledger written before this lands
yields a report. `replaced_run` is additive under the plan JSON's
additive-only rule.

Going forward, a later ledger change must keep existing fields' encoding (or
keep decoding the old one) and default new fields when absent from a stored
ledger; otherwise a run interrupted before an upgrade decodes as corrupt and
yields the corrupt-ledger warning instead of the report. The report also
reads `claims` through question normalisation and the stem rule: a change to
either must keep older claims decoding and matching, or runs interrupted
across it lose their owning pairs and report spawns as unasked.

## References

- Original work item: `meta/work/0310-recovering-from-an-interrupted-conduct-run.md`
- Codebase research: `meta/research/codebase/2026-10-13-0310-recovering-from-an-interrupted-conduct-run.md`
- Plan-field-plus-skill-row precedent: `meta/plans/2026-10-10-0303-research-concurrency-subagent-limit.md` (Phase 5)
- Conditional-key pattern: `cli/research-cli/src/topic_command.rs:515-524`
- Classification model: `cli/research/src/conduct/observed.rs`
