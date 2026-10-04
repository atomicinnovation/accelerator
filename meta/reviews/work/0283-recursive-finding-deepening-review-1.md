---
type: "work-item-review"
id: "0283-recursive-finding-deepening-review-1"
title: "Work Item Review: Recursive Finding Deepening"
date: "2026-09-25T14:17:32+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0283"
work_item_id: "0283"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-26T00:55:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Recursive Finding Deepening

**Verdict:** REVISE

The work item is structurally complete and unusually precise: cap arithmetic,
lineage paths, guard rules and the resume/depth-change matrix are consistent
across Summary, Requirements, Acceptance Criteria and Technical Notes, and the
design rationale (conduct-orchestrated recursion, notes as source of truth) is
well argued. It needs revision because the acceptance criteria lag the
requirements — several requirements have no criterion, several criteria are
weaker than their requirement or pass trivially, and two criteria use
"returns follow-ups" wording that contradicts the content-free rule. Separately,
the concurrency cap is bundled into a gated, droppable item it does not
technically depend on, and the 0278 coupling is uncaptured.

### Cross-Cutting Themes

- **Concurrency cap bundling** (flagged by: scope, dependency) — the cap
  applies at `depth: 1`, needs nothing from the recursion engine, yet inherits
  0280's gate and 0283's descope fate.
- **Concurrency semantics under-specified** (flagged by: clarity, testability)
  — "batch" is undefined (barrier vs sliding window), and the default-24
  criterion has no over-cap workload, so neither the meaning nor the default is
  verifiable.
- **Acceptance criteria lag requirements** (flagged by: completeness,
  testability) — quarantine, `.1.md.invalid` index retention, `round_count` /
  `synthesise` scoping, notes kept, composer config key, `configure help`
  content and the step 8 reason/next-step have no criterion.
- **"Returns follow-ups" wording** (flagged by: clarity, testability) — AC 2
  and AC 8 describe follow-ups as a researcher's return, contradicting the
  content-free orchestration requirement and AC 7.
- **Weak bounds in orchestration criteria** (flagged by: testability, clarity)
  — "at most" caps and batches pass on degenerate implementations; the guard
  criterion does not test the one-injected-path rule.

### Findings

#### Major

- 🟡 **Clarity / Testability**: ACs say researchers "return" follow-ups,
  contradicting content-free orchestration
  **Location**: Acceptance Criteria
  AC 2 ("whose level-1 researcher returns 4 follow-up questions") and AC 8 ("a
  node returning `follow_ups: []`") frame follow-ups as a return value, while
  the requirement and AC 7 make the note the sole source.
- 🟡 **Scope / Dependency**: Concurrency cap is a separate concern bundled into
  a droppable, gated item
  **Location**: Requirements: Concurrency
  The cap governs every `conduct` batch including `depth: 1`, could ship
  independently, yet is blocked on 0280's gate and lost if 0283 is descoped.
- 🟡 **Testability / Completeness**: Stated requirements have no corresponding
  acceptance criterion
  **Location**: Acceptance Criteria
  Quarantine as `.<lineage>.md.invalid`, index retention via `.1.md.invalid`,
  `round_count`/`synthesise` scoping, notes kept after composition,
  tier-tagging, `accelerator config agent composer`, `configure help` content,
  and the step 8 reason/next-step are unverified.
- 🟡 **Clarity**: "Batch" semantics of the concurrency cap are ambiguous
  **Location**: Requirements: Concurrency
  Nothing says whether a batch is a barrier-synchronised group or a sliding
  window of N in flight, or whether levels and composers may share one.
- 🟡 **Testability**: Default concurrency criterion has no precondition that
  exceeds the cap
  **Location**: Acceptance Criteria
  "Spawns at most 24 at a time" passes trivially for any workload of 24 or
  fewer.
- 🟡 **Testability**: Researcher write-guard criterion is weaker than the
  one-injected-path requirement
  **Location**: Acceptance Criteria
  A guard accepting any well-formed lineage path would pass, letting a
  researcher overwrite a sibling's note.
- 🟡 **Testability**: Eval-level verification of conduct orchestration has no
  defined observation method
  **Location**: Technical Notes
  Spawn counts, batch sizes and prompt contents are "verified at eval level"
  with no named stub, fixture or Task-call log.
- 🟡 **Dependency**: Uncaptured dependency on 0278 (umbrella doc type and
  nested indexing)
  **Location**: Dependencies
  Level notes register under the umbrella `topic-research` type and rely on
  the indexer skipping `findings/`, both delivered by in-progress 0278, which
  is not a transitive blocker via 0280.

#### Minor

- 🔵 **Scope**: Re-running at a larger `--depth` could be a follow-up story
  **Location**: Requirements: Depth changes on resume
  Extending a tree at a larger depth is additive to the core recursion; the
  smaller-depth escape hatch is the load-bearing part.
- 🔵 **Dependency**: 0281 `ask`/`report` not captured as a consumer of the
  `.levels/` layout
  **Location**: Dependencies
  0281 reads every finding of a focus area; nothing records that it must
  exclude level notes.
- 🔵 **Dependency**: 0284's enumeration must exclude `.levels/`, but is only
  recorded as out of scope
  **Location**: Dependencies
  Level notes carry `question` and `source_profile`, the fields 0284 groups
  on.
- 🔵 **Dependency**: arXiv rate-limit coupling missing
  **Location**: Assumptions
  0280 serialises arXiv at 1 req/3s and assumes ≤2 min per focus area; 13
  nodes per pair at `depth: 3` breaks that.
- 🔵 **Dependency**: 0280 blocker covers only the gate, not 0280's code
  artefacts
  **Location**: Dependencies
  The item extends 0280's pair model, profiles and guard; "sign-off, not
  merge" could read as permitting a start before that code settles.
- 🔵 **Testability**: Halving and batching criteria use "at most" bounds that
  degenerate implementations satisfy
  **Location**: Acceptance Criteria
  Spawning nothing satisfies AC 3; fully serial execution satisfies AC 10.
- 🔵 **Testability**: Composer prose quality and follow-up ranking are
  unverifiable; citation criterion ignores resolved depth
  **Location**: Requirements
  A `--depth 2` recompose citing a level-3 note would pass.
- 🔵 **Testability**: Final check criterion excludes the test suite
  **Location**: Acceptance Criteria
  `mise run check` is format/lint/types only; the promised unit tests are not
  part of done.
- 🔵 **Clarity**: Lineage encoding reads ambiguously
  **Location**: Requirements: Level notes
  The leading segment is the level, not a position, but the sentence implies
  every segment is a position.
- 🔵 **Clarity**: Dedupe tie-break and "keeps its position" unclear
  **Location**: Requirements: Duplicate avoidance
  Which duplicate survives between same-level siblings or cousins is
  unstated.
- 🔵 **Clarity**: Resume-at-smaller-depth ACs leave unstated which notes exist
  **Location**: Acceptance Criteria
  "Left outstanding" could mean a missing level-2 node, which would need a
  researcher.
- 🔵 **Clarity**: Passive "is quarantined" hides which component quarantines
  **Location**: Requirements: Resume
  The actor (conduct, `outstanding`, validator) and timing are unstated.
- 🔵 **Clarity**: Vague referents in individual ACs
  **Location**: Acceptance Criteria
  "None is spawned" (AC 2), "level 1 spawns" (AC 3), and "its subtree" for a
  limit-level node at `depth: 2`.
- 🔵 **Completeness**: Some Surface and Composer requirements have no matching
  AC
  **Location**: Acceptance Criteria
  Merged into the major "Stated requirements have no corresponding acceptance
  criterion" above.

#### Suggestions

- 🔵 **Completeness**: Context explains constraints but not why
  single-level findings fall short
  **Location**: Context
  A descope candidate should state the value being weighed against its cost.
- 🔵 **Testability**: Over-cap warning and dedupe lineage criteria lack a
  concrete observable
  **Location**: Acceptance Criteria
  Name the warning channel and give a worked dedupe example.
- 🔵 **Clarity**: Unexplained "breadth above 8" threshold
  **Location**: Assumptions
  Tie 8 to 0282's default `breadth`.
- 🔵 **Clarity**: Undefined local referents
  **Location**: Requirements: Surface; Requirements: Concurrency
  "Step 8", "misplaced flag", "the ceiling", and three notations for the notes
  directory.

### Strengths

- ✅ Cap arithmetic (4, then `ceil(parent/2)`) and worst-case node counts
  (5/13/21) agree across every section.
- ✅ Context derives the conduct-orchestrated design from a stated constraint
  (researchers cannot nest subagents).
- ✅ AC 7 turns "notes are the source of truth" into a falsifiable scenario.
- ✅ The resume and depth-change matrix covers `--depth 1/2/3`, a composed pair,
  a missing node and an all-notes-no-finding state.
- ✅ Technical Notes enumerate every walker of `findings/`, turning hidden
  couplings into visible ones, and split unit-testable from eval-level
  verification.
- ✅ The 0280 blocker names a precise unblock condition (gate sign-off, not
  merge); 0282 is marked satisfied; 0284 is recorded as out of scope.
- ✅ Accepted risks (OpenAlex budget, uncapped breadth on descope,
  near-paraphrase siblings) and the descope fallback are explicit.

### Recommended Changes

1. **Rephrase AC 2 and AC 8 preconditions in terms of the note** (addresses:
   ACs say researchers "return" follow-ups)
   "whose level-1 note records 4 `follow_ups`"; "a node whose note records
   `follow_ups: []`". Also name "no level-3 researcher" in AC 2.
2. **Decide the concurrency cap's home and define "batch"** (addresses:
   Concurrency cap bundling; batch semantics)
   Either split the cap into its own story (or under 0282) that 0283 depends
   on, or record in Drafting Notes why it shares the gate and descope fate.
   Either way, define batch as barrier group or sliding window and whether
   composers share it.
3. **Add criteria for the uncovered requirements** (addresses: Stated
   requirements have no AC; Surface/Composer ACs)
   Quarantine and subtree re-spawn; `.1.md.invalid` index retention;
   `round_count`/`synthesise` ignore `.levels/` and notes survive; composer
   agent config override; `configure help` names `research.topic.concurrency`
   (default 24) and the `composer` key; step 8 names reason and next step.
4. **Tighten orchestration criteria with over-cap fixtures and exact
   expectations** (addresses: default concurrency AC; "at most" bounds)
   e.g. level-1 note with 6 follow-ups → exactly 4 level-2 researchers told cap
   2; 5 pairs at concurrency 2 → batches 2, 2, 1; >24 pending spawns with no
   config → no batch above 24, composers included.
5. **Enumerate guard cases** (addresses: write-guard criterion)
   Allowed: the injected `2-3.md`. Rejected: an uninjected sibling, another
   pair's `.levels/`, nested paths, non-`.md`, malformed lineage, dot-prefixed
   quarantine names. Mirror for the composer.
6. **Name the eval mechanism** (addresses: eval-level verification)
   State the stub researcher, fixture location and how Task calls and prompts
   are observed, or tag each orchestration AC with its eval.
7. **Extend Dependencies** (addresses: 0278; 0281; 0284; 0280 artefacts;
   arXiv)
   Add 0278 as blocker or coordination edge; add 0281 and extend 0284 with the
   "top-level `<nn>-*.md` only" contract; note that the item builds on 0280's
   merged pair model, profiles and guard; record the arXiv wall-clock risk.
8. **Clarify lineage, dedupe order, quarantine actor and resume
   preconditions** (addresses: clarity minors)
   Worked lineage example for a level-3 node; state which duplicate survives;
   name who quarantines and when; state which notes exist in the
   smaller-depth resume ACs; fix "its subtree" at `depth: 2`.
9. **Tighten composer citation criterion and definition of done**
   (addresses: citation ignores resolved depth; final check excludes tests)
   Cited sources must appear in notes with `level` ≤ stamped `depth`; replace
   `mise run check` with the bare `mise run` default task.
10. **Minor polish** (addresses: suggestions)
    Context sentence on the value of depth; tie "breadth above 8" to 0282;
    replace "step 8", "the ceiling" and the three stem placeholders with one
    term each; name the over-cap warning channel; consider deferring the
    larger-`--depth` extension.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Dense but mostly precise. Core terms are defined or anchored to
concrete paths, and the cap-halving arithmetic is consistent. The main clarity
problems are internal inconsistencies: ACs that describe researchers as
"returning" follow-ups, ambiguous concurrency "batch" semantics, a lineage
encoding that is easy to misread, resume ACs with unstated preconditions, and
passive constructions that hide the quarantining actor.

**Strengths**:
- Cap arithmetic consistent across Summary, Requirements, AC 3 and Technical
  Notes (5/13/21).
- "Pair" defined explicitly before use.
- Context derives the conduct-orchestrated design from a stated constraint.
- AC 7 makes the notes-as-truth rule concrete.
- Most requirements name their actor.

**Findings**:
- 🟡 major / high — **Acceptance Criteria** — ACs say researchers "return"
  follow-ups, contradicting content-free orchestration. AC 2 and AC 8 frame
  follow-ups as a return value; express preconditions as what the note
  records.
- 🟡 major / medium — **Requirements: Concurrency** — "Batch" semantics
  ambiguous: whole level, barrier group, or sliding window of N in flight.
  Define once, including whether levels and composers share a batch.
- 🔵 minor / medium — **Requirements: Level notes** — Lineage encoding reads
  ambiguously; state that the first segment is the level and the rest are
  ancestor-to-self positions, with a level-3 example.
- 🔵 minor / medium — **Requirements: Duplicate avoidance** — "Already in the
  tree" tie-break and "keeps its position" unclear; state which duplicate
  survives and that siblings are not renumbered.
- 🔵 minor / medium — **Acceptance Criteria** — Smaller-depth resume ACs leave
  unstated which notes exist; name them.
- 🔵 minor / medium — **Requirements: Resume** — Passive "is quarantined" hides
  the actor and timing.
- 🔵 minor / high — **Acceptance Criteria** — Vague referents: "none is
  spawned", "level 1 spawns", "its subtree" for a limit-level node.
- 🔵 suggestion / medium — **Assumptions** — Unexplained "breadth above 8";
  tie to 0282's default.
- 🔵 suggestion / medium — **Requirements: Surface; Concurrency** — "Step 8",
  "misplaced flag", "the ceiling", and three notations for the notes
  directory.

### Completeness

**Summary**: Very complete. Every expected section is present and substantive:
user-framed Summary, motivating Context, detailed Requirements, 25 Given/When/
Then ACs, and populated Dependencies, Assumptions, Technical Notes, Drafting
Notes and References. Frontmatter is intact. Remaining gaps: a few Surface and
Composer requirements lack criteria, and Context leans towards constraints over
the user problem.

**Strengths**:
- User-story Summary with mechanism and honest descope framing.
- Context explains why conduct owns recursion.
- Requirements are specific enough to start without clarification.
- ACs cover nearly every requirement.
- Dependencies state a precise unblock condition.
- Accepted risks are explicit; Open Questions explains its "None".
- References map the implementation surface.

**Findings**:
- 🔵 minor / medium — **Acceptance Criteria** — Some Surface and Composer
  requirements (concurrency key and default, composer key, step 8 reason and
  next step, `accelerator config agent composer`) have no criterion.
- 🔵 suggestion / low — **Context** — Does not state why single-level findings
  fall short, which a descope decision needs.

### Dependency

**Summary**: Primary blocking edges are well captured: 0280's gate with a
precise unblock condition, 0282 satisfied, 0284 related. Uncaptured: 0278
(umbrella type, nested indexing), sibling consumers 0281 and 0284 of the
`.levels/` layout, and the arXiv rate-limit interaction with multiplied node
counts.

**Strengths**:
- Precise 0280 unblock condition and current status.
- 0282 edge marked satisfied, with forward-only `blocks` convention mirrored.
- 0284 relationship recorded with out-of-scope note.
- Technical Notes enumerate every walker of `findings/`.
- OpenAlex budget exhaustion named as an accepted risk.

**Findings**:
- 🟡 major / medium — **Dependencies** — Uncaptured dependency on 0278, which
  delivers the umbrella `topic-research` type and nested indexing and is not a
  transitive blocker via 0280. Add as blocker or coordination edge.
- 🔵 minor / medium — **Dependencies** — 0281 `ask`/`report` not captured as a
  consumer that must exclude `.levels/`.
- 🔵 minor / medium — **Dependencies** — 0284's enumeration must exclude
  `.levels/` / `kind: level-note`; record the contract reciprocally.
- 🔵 minor / medium — **Assumptions** — arXiv's serialised 1 req/3s limit and
  0280's ≤2 min/focus-area assumption break at depth > 1.
- 🔵 minor / medium — **Dependencies** — 0280 blocker covers only the gate, not
  its pair model, profiles and guard code that this item extends.
- 🔵 suggestion / low — **Requirements** — Concurrency cap for `depth: 1` is
  ordered behind a gate it does not technically depend on; record this
  explicitly.

### Scope

**Summary**: Largely one coherent unit: conduct-orchestrated recursion within a
pair, with notes, composer, guard, resume and index retention all needed for
it. The main tension is the concurrency cap, an orthogonal concern that changes
`depth: 1` behaviour yet shares this item's gating and descope fate. The item
is large for a story, but mostly irreducibly so.

**Strengths**:
- Explicit boundaries: visualiser to 0284, knob to 0282, `breadth` separate
  from recursion.
- Apparent extras are shown to be forced by adding `.levels/` under
  `findings/`.
- Descope status and fallback are explicit.
- Summary, Requirements and ACs describe the same scope.

**Findings**:
- 🟡 major / high — **Requirements: Concurrency** — Concurrency cap is a
  separate concern bundled into a droppable, gated item. Extract to its own
  story (or under 0282) that 0283 depends on, or record why it shares the
  fate.
- 🔵 minor / medium — **Requirements: Depth changes on resume** — Extending at
  a larger `--depth` could be a follow-up story; keep the smaller-depth escape
  hatch here.

### Testability

**Summary**: Largely testable: concrete Given/When/Then with depths, caps,
paths and flags, plus a unit-vs-eval verification split. Gaps: "at most"
bounds that degenerate implementations pass; guard and concurrency criteria
looser than their requirements; several requirements with no criterion; and no
defined observation method for eval-level orchestration checks.

**Strengths**:
- Concrete preconditions and observable outcomes throughout.
- Thorough resume/depth-change state matrix.
- Clear unit-testable vs eval-level split.
- AC 7 falsifies the return-vs-note distinction.
- Concrete two-focus-area index-retention fixture.

**Findings**:
- 🟡 major / high — **Acceptance Criteria** — Stated requirements have no
  criterion: quarantine, `.1.md.invalid` retention, `round_count`/`synthesise`
  scoping, notes kept, tier-tagging, composer config, `configure help`
  content, step 8 reason and next step.
- 🟡 major / high — **Acceptance Criteria** — Default concurrency criterion has
  no over-cap workload; add one where pending researchers and, separately,
  composers exceed 24.
- 🟡 major / medium — **Acceptance Criteria** — Researcher guard criterion is
  weaker than the one-injected-path rule; enumerate allowed and rejected
  paths, and mirror for the composer.
- 🟡 major / medium — **Technical Notes** — Eval-level verification has no
  defined observation method (stub researcher, fixtures, Task-call log).
- 🔵 minor / high — **Acceptance Criteria** — "At most" bounds in AC 3 and
  AC 10 pass on degenerate implementations; use exact expectations with
  over-cap fixtures.
- 🔵 minor / high — **Acceptance Criteria** — AC 2's "researcher returns"
  precondition contradicts content-free return.
- 🔵 minor / medium — **Requirements** — Composer prose shape and follow-up
  ranking unverifiable; citation criterion ignores resolved depth.
- 🔵 minor / medium — **Acceptance Criteria** — `mise run check` excludes the
  test suite; use the bare `mise run`.
- 🔵 suggestion / medium — **Acceptance Criteria** — Over-cap warning channel
  and a worked dedupe lineage example are unspecified.

## Re-Review (Pass 2) — 2026-09-25T15:46:13+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: ACs say researchers "return" follow-ups — Resolved
- 🟡 **Scope / Dependency**: Concurrency cap bundled into a gated item — Resolved by recorded decision (scope re-raises it as minor)
- 🟡 **Testability / Completeness**: Requirements with no criterion — Resolved; residual gaps below (legacy finding without `depth`, composer prose shape)
- 🟡 **Clarity**: "Batch" undefined — Partially resolved; batch is defined but now conflicts with "level by level"
- 🟡 **Testability**: Default concurrency AC trivially satisfied — Resolved
- 🟡 **Testability**: Guard criterion weaker than requirement — Resolved
- 🟡 **Testability**: Eval observation method undefined — Partially resolved; evals deferred to 0161, but attended-run preconditions and evidence remain unstated
- 🟡 **Dependency**: Uncaptured 0278 dependency — Resolved
- 🔵 **Scope**: Larger `--depth` extension — Kept by recorded decision
- 🔵 **Dependency**: 0281, 0284, 0280 artefacts, arXiv — Resolved
- 🔵 **Testability**: "At most" bounds; final check excludes tests — Resolved
- 🔵 **Testability**: Composer prose unverifiable; citation ignores depth — Partially resolved (citation fixed; prose shape still has no criterion)
- 🔵 **Clarity**: Lineage encoding; resume preconditions; quarantine actor; vague referents; local referents — Resolved
- 🔵 **Clarity**: Dedupe tie-break — Partially resolved; "lineage order" itself is undefined
- 🔵 **Clarity**: "Breadth above 8" — Partially resolved; still reads as `breadth` rather than concurrency being uncapped
- 🔵 **Testability**: Over-cap warning observable — Partially resolved; requirement names stderr, AC does not
- 🔵 **Completeness**: Context motivation — Resolved

### New Issues Introduced

- 🟡 **Clarity**: "Level by level" conflicts with batches filled from everything outstanding — unclear whether `outstanding` re-runs per batch or per level, and whether a batch mixes levels.
- 🟡 **Clarity / Testability**: "Lineage order" undefined — level-major vs depth-first give different surviving duplicates; no expected outcome for cross-branch or same-note duplicates.
- 🟡 **Testability**: Orchestration preconditions (over-cap notes, divergent returns) cannot be produced by an attended run on demand — state that they are established by seeding `.levels/` notes and running `conduct` as a resume.
- 🟡 **Testability**: No stated evidence for spawn-level attended checks (batch grouping, caps and known questions in prompts).
- 🟡 **Scope**: Story sized like an epic slice (17 requirements, 28 criteria) — consider splitting or recording why it is indivisible.
- 🔵 **Testability**: Concurrency tested only at `depth: 1`; invalid concurrency values unspecified; next-step wording and source-matching key undefined; legacy finding without `depth` has no AC; `mise run` AC names areas, not behaviours.
- 🔵 **Clarity**: "Below" means both deeper and shallower; composer guard rule uses `<name>` where `<stem>` is meant; unclear whether a directly written `depth: 1` finding is stamped and complete; "pairs needing composition only" ambiguous; unanchored terms ("node", "the engine", `budget_exhausted`, "dzhng halving").
- 🔵 **Completeness**: Level-note body content unspecified; dzhng prior-art link missing.
- 🔵 **Dependency**: 0161 has no reciprocal record of the eval handoff; 0280's sibling contract wants the per-profile cost multiplier stated; attended verification needs OpenAlex/arXiv availability and budget.

### Assessment

The first pass's structural issues are resolved: the criteria now track the requirements, use exact counts, enumerate guard paths, and the dependency map is two-sided. Pass 2 surfaces a second layer of precision issues, two of which the batch and dedupe edits introduced (level-vs-batch scheduling, undefined lineage order), plus the practical question of how attended runs establish and observe orchestration preconditions. These are small, targeted edits; the scope re-sizing finding contests a decision already made and can be closed by a Drafting Notes line.

## Re-Review (Pass 3) — 2026-09-25T17:55:29+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "Level by level" vs batch filling — Resolved (per-pair level barrier)
- 🟡 **Clarity / Testability**: Undefined lineage order — Resolved
- 🟡 **Testability**: Attended-run preconditions — Resolved for seeded cases; the depth-3 fan-out criterion still mixes seeded and model-produced notes (see below)
- 🟡 **Testability**: Evidence for spawn-level checks — Resolved
- 🟡 **Scope**: Story sized like an epic slice — Closed by recorded decision; scope re-raises it
- 🔵 Pass-2 minors (below/deeper, `<stem>`, depth stamping, invalid concurrency, warning channel, legacy finding, level-note body, 0161 reciprocal, per-profile multiplier, dzhng link) — Resolved

### New Issues Introduced

- 🟡 **Clarity**: Lineage example `3-2-4` contradicts the level-2 cap of 2 (introduced in pass-1 edits).
- 🟡 **Testability**: Depth-3 fan-out criterion is self-contradictory — seeded `1.md` means no level-1 cap is observed, and level-2 notes "recording 3" contradict their injected cap of 2.
- 🟡 **Scope**: Concurrency cap bundling and story size re-raised — both recorded decisions in Drafting Notes.
- 🔵 **Clarity / Testability**: Pair A/B batching criterion omits B's follow-on composer batch; ceil formula applies per `outstanding` re-run, not per run.
- 🔵 **Clarity**: Note `depth` field meaning after depth changes; "expected node" undefined; trim-before-dedupe ordering implied only.
- 🔵 **Testability**: Composer "first paragraph answers" is subjective; smaller-depth and containment checks pass vacuously without distinguishing sources; failed-node setup and reason strings undefined; unit-test roll-up omits composition-only category, concurrency resolution and whitespace normalisation; limit-level follow-up recording and content-free return loosely checked.
- 🔵 **Dependency**: Config snapshot files (key-count test, `dump.golden`, `public-api.txt`) not recorded; 0161 recursion-eval portion should be blocked by 0283; OpenAlex budget as a verification prerequisite not in Dependencies; re-check code references after 0280 merges.
- ✅ **Completeness**: No findings.

### Assessment

Of four majors, two restate recorded scope decisions and two are defects introduced by earlier review edits (the `3-2-4` example and the depth-3 fan-out criterion); both are one-line fixes. The remaining minors are precision refinements to attended-run criteria and dependency bookkeeping. Once the two defects are fixed, the item is ready for planning; the scope findings stand as accepted.

## Re-Review (Pass 4) — 2026-09-26T00:52:26+00:00

Lenses re-run: clarity, testability.

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: `3-2-4` lineage example — Resolved
- 🟡 **Testability**: Depth-3 fan-out criterion — Resolved (split into seeded `outstanding` case and attended cap check)
- 🔵 **Clarity**: A/B batching, note `depth` meaning, "expected node", trim-before-dedupe — Resolved
- 🔵 **Testability**: Composer shape, vacuous containment, failed-node setup, unit-test roll-up, limit-level prompt, content-free return — Resolved or partially resolved (see below)

### New Issues Introduced

- 🟡 **Clarity**: "Missing node" definition excludes the level-1 node, so an unstarted or root-quarantined pair has no reported missing node.
- 🟡 **Clarity**: "`depth: 1` means no composer" (Summary, first AC) contradicts the `--depth 1` resume path that composes from `1.md`.
- 🟡 **Testability**: Breadth criterion conflicts with 0282's outline-only `breadth` enforcement and does not state the outline size.
- 🟡 **Testability**: No criterion shows a limit-level node recording follow-ups end to end.
- 🔵 **Clarity**: Actor behind "validated" in `outstanding` rules; plural/singular mismatch in the fetch-failure AC; placeholder `k` reused for different positions; `breadth` used for dzhng's fan-out; shorthand terms from siblings; "match the note's `follow_ups`" imprecise.
- 🔵 **Testability**: Dedupe examples omit their `--depth`; cap-injection AC depends on model output; fetch-failure setup not reproducible; composer shape check degenerates for single-note pairs; follow-up leak check has no match rule; attended-run evidence not recorded.

### Assessment

The pass-3 defects are fixed. Pass 4 surfaces edge-of-definition issues — the root node's place in "missing nodes", the `depth: 1` qualifier, and two criteria that can pass vacuously. Each is a one- or two-line edit. Returns are diminishing: each pass now finds issues at a finer grain than the last, several introduced by prior review edits.

### Edits after Pass 4

All four pass-4 majors and most minors were addressed without a further pass: the level-1 node joins the missing set when no validated `1.md` exists; the `depth: 1` shape is qualified to pairs with no `.levels/`; the breadth criterion fixes its outline and asserts recursion does not truncate it; limit-level follow-up recording is listed as a prose contract. `outstanding` now validates the notes it reads; cap-injection, dedupe and fetch-failure criteria use seeded, concrete preconditions and lineages. Skipped by decision: sibling-shorthand glosses and attended-run evidence recording (left to `validate-plan`). The concurrency bundling and story size stand as recorded decisions.

## Verdict Change — 2026-09-26T00:55:27+00:00

**Verdict:** APPROVE

Changed by the reviewer after the pass-4 edits, without a further pass.
