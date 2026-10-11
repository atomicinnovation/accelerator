---
type: "work-item-review"
id: "0296-sync-round-trip-defects-between-local-work-items-and-the-tracker-review-1"
title: "Work Item Review: Sync Round-Trip Defects Between Local Work Items and the Tracker"
date: "2026-10-10T17:15:21+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0296"
work_item_id: "0296"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-10T20:44:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Sync Round-Trip Defects Between Local Work Items and the Tracker

**Verdict:** REVISE

Every section is present and the reproductions are precise. Equivalence is defined as a deterministic comparison of parsed event streams, and the scope boundaries name a sibling for each exclusion. Most of the major findings are inconsistencies between sections that make two or more acceptance criteria impossible to satisfy together. They concern what `--preview` writes, which Linear rewrites count as noise, what an old-recipe "pull" does, and what a Jira pull produces. The rest are about scale: the item bundles four separately shippable pieces of work, and it does not record that it blocks 0320 or that it shares the baseline schema with 0290.

### Cross-Cutting Themes

- **Preview baseline writes contradict each other** (flagged by: clarity, testability). The Defect 2 noise-only reproduction expects `--preview` to record a baseline. The migration criterion lists `--preview` among the runs that do not rewrite entries.
- **The noise category list drifts between sections** (flagged by: clarity, testability). Context, Equivalence, the noise criterion and Assumptions each list Linear's rewrites differently. Autolinked URLs and bold hoisted off code spans are covered by no rule and no criterion.
- **The old-recipe "harmless pull" is undefined** (flagged by: clarity, testability). It is unclear whether this pull writes the file, which would conflict with the rule that a sync never rewrites a local file whose body is equivalent to the remote one.
- **Jira is under-specified for its size** (flagged by: scope, clarity, testability). Jira scope is absent from the title and Summary. The pull criteria use Linear-only wording ("remote `description`"), and no criterion covers a Jira pull.
- **Nobody owns title handling** (flagged by: testability, dependency, scope). A title-only remote edit has no expected outcome. 0290 does not cover the title, and the dossier's title-difference line anticipates field-level presentation.
- **Undeclared prerequisites** (flagged by: completeness, dependency). Open Questions says "None", but the parser choice, the Linear read-back fixture capture and the corpus history check are all still open.

### Findings

#### Major

- 🟡 **Clarity + Testability**: Preview both records and does not record a baseline
  **Location**: Requirements: Defect 2 / Acceptance Criteria (unmarked entries not rewritten)
  The Defect 2 noise-only reproduction runs `work sync --preview` and expects "a recorded baseline". The final migration criterion lists `--preview` among the runs that do not rewrite an entry. Tests written from the two cannot both pass.
- 🟡 **Clarity + Testability**: Noise categories inconsistent; autolinked URLs and hoisted bold uncovered
  **Location**: Context / Requirements: Equivalence / Acceptance Criteria / Assumptions
  The four lists of Linear rewrites disagree. No equivalence rule normalises an autolinked URL (a text event becomes a link event) or bold hoisted off code spans. 0320 nevertheless says 0296 absorbs both.
- 🟡 **Clarity + Testability**: Old-recipe "harmless" pull conflicts with the no-write rule
  **Location**: Requirements: Digest migration
  The item does not say whether a one-sided noise change under the old recipe writes the local file. "Harmless" is not observable, and no criterion covers noise on an unmarked entry.
- 🟡 **Testability + Clarity**: No criterion for a Jira pull; Linear-only wording
  **Location**: Acceptance Criteria / Requirements: Defect 1
  "The local body equals the remote `description`" cannot hold for Jira, where the description is ADF JSON. Nothing pins the summary-line strip or `document_to_markdown` output on a Jira pull.
- 🟡 **Testability**: Dossier criterion can pass without the fix
  **Location**: Acceptance Criteria (dossier lists only the edited section)
  The untouched list section is not required to differ by Linear noise. A byte-identical fixture therefore passes against today's code.
- 🟡 **Testability**: A title-only remote edit has no expected outcome
  **Location**: Requirements: Out of scope
  "No longer registers as a body change" is a behavioural change, but the reported status, the baseline effect and the local file effect are all unspecified.
- 🟡 **Scope**: Several separately deliverable pieces of work bundled into one bug
  **Location**: Requirements
  The item contains four pieces of work: the title-line strip, the equivalence engine with the digest split and migration, Jira ADF equivalence, and the dossier presentation. The ordering requirement is better met by splitting Defect 1 out as a blocker than by merging it into the rest.
- 🟡 **Scope**: Jira equivalence goes beyond the Linear-only title and Summary
  **Location**: Requirements
  Jira adds a crate-restructuring change and converter failure modes, but no Jira defect is reproduced.
- 🟡 **Dependency**: 0320 depends on 0296's equivalence, but "Blocks: none"
  **Location**: Dependencies
  0320's acceptance criteria use "0296's definition" of equivalence. Neither item records the blocking edge.
- 🟡 **Dependency**: 0296 and 0290 both change the baseline `Entry` and digest recipe
  **Location**: Requirements: Digest migration
  No landing order is recorded. Without one, the two items risk incompatible schema changes or a double migration.

#### Minor

- 🔵 **Clarity**: "Authoritative for markdown representation" versus pulls writing the tracker serialisation
  **Location**: Context / Required behaviour
  The authority rule's boundary (it applies only when bodies are equivalent) is implied rather than stated. "Symmetric for content" is undefined.
- 🔵 **Clarity**: Jira converter steps named two different ways
  **Location**: Requirements / Acceptance Criteria / Technical Notes
  The item says both `markdown_to_document`/`document_to_markdown` and `assemble`/`render`, with no mapping between the two pairs.
- 🔵 **Clarity**: "Ignoring blank lines" and "frontmatter keeps its existing comparison" are ambiguous
  **Location**: Requirements: Equivalence
  Blank lines are not CommonMark events. No separate frontmatter comparison exists today.
- 🔵 **Clarity**: Undefined status terms and loose phrasing
  **Location**: Acceptance Criteria / Required behaviour
  `write-once`, `remote-absent`, `indeterminate` and "digest recipe" are not defined. "Content-changing" and "meaning-changing" are used as apparent synonyms.
- 🔵 **Testability**: The failed read-back criterion only rules out one outcome
  **Location**: Acceptance Criteria (create whose read-back failed)
  It asserts only that the item is "not reported as a conflict". It needs a positive outcome: `synced`, a non-empty `remote_hash` and no write.
- 🔵 **Testability**: Several recovery cases have no criterion
  **Location**: Requirements: Recovery for the existing corpus
  Two cases are missing: a non-equivalent body with a blank `local_hash` (0136, 0158 and others) and a non-equivalent body with an empty `remote_hash`.
- 🔵 **Testability**: Rewrite categories have no concrete fixture pairs; the renumbering boundary is unclear
  **Location**: Acceptance Criteria
  "Ignoring list-marker style" could be read as covering ordered start numbers, which would contradict the renumbering criterion.
- 🔵 **Testability**: The golden-digest criterion passes trivially, and nothing enforces the recipe-marker bump
  **Location**: Acceptance Criteria (golden digest)
  The fixture does not record the parser version and recipe marker, so it cannot fail on an upgrade without a bump.
- 🔵 **Scope**: The `bug` kind understates a change that includes a schema migration
  **Location**: Frontmatter: kind
- 🔵 **Scope**: The dossier title-difference line is new capability that overlaps title sync
  **Location**: Requirements: Required behaviour
- 🔵 **Dependency**: Recovery of 0276 and 0293 needs 0320
  **Location**: Context
  Their meaning-changing rewrites keep them conflicted after 0296 alone.
- 🔵 **Dependency**: Linear's server-side re-serialisation is an external coupling missing from Dependencies
  **Location**: Assumptions
- 🔵 **Dependency**: Dossier shape changes affect the `sync-work-items` skill and 0213's evals
  **Location**: Requirements: Required behaviour
- 🔵 **Dependency**: Title sync is deferred to 0290, but 0290 does not cover the title
  **Location**: Requirements: Out of scope
- 🔵 **Completeness**: Open Questions says "None" while the parser choice and verifications remain open
  **Location**: Open Questions
- 🔵 **Completeness**: `relates_to` and References leave out 0320 and 0321
  **Location**: Frontmatter: relates_to

#### Suggestions

- 🔵 **Completeness**: The Summary does not say what the work delivers
  **Location**: Summary
- 🔵 **Scope**: The old digest recipe has no retirement point
  **Location**: Requirements: Digest migration
- 🔵 **Dependency**: The new parser crate and crate-boundary moves are not listed as prerequisites
  **Location**: Technical Notes
- 🔵 **Dependency**: 0321 may revise the representation rule; the no-wait decision is not recorded here
  **Location**: Dependencies
- 🔵 **Testability**: Nothing re-measures the corpus symptom from the Summary
  **Location**: Summary
- 🔵 **Testability**: The ordering requirement cannot be checked by a behavioural test
  **Location**: Requirements: Required behaviour

### Strengths

- ✅ Both defects use a Reproduction / Expected / Actual structure with exact commands and the dossier path. Defect 2 separates the noise-only case from the real-conflict case.
- ✅ Equivalence is defined once as a comparison of pinned-parser CommonMark event streams with a closed list of normalisations. This makes it a deterministic, unit-testable function.
- ✅ Negative criteria (meaning-changing rewrites, a `*`/`-` swap in fenced code, bodies the Jira converter rejects) stop an equivalence that is too permissive from passing.
- ✅ Outcomes are observable: file bytes, reported status, tracker updates and dossier file existence.
- ✅ Out of scope points each exclusion at a named sibling (0290, 0320, 0321), and "normalising anything beyond the equivalence definition" closes the rule set.
- ✅ The ordering constraint between the title strip and new pull behaviour is a stated requirement with a recorded rationale.
- ✅ The Drafting Notes make every split-or-keep and design choice auditable.

### Recommended Changes

1. **Decide whether `--preview` writes baselines and make Defect 2 and the migration criterion agree** (addresses: Preview both records and does not record a baseline). The likely fix is for preview to report what it would record, with a non-preview run doing the recording.
2. **Write one table of the 12 rewrite categories and map each to an equivalence rule or to "real difference"** (addresses: Noise categories inconsistent; Rewrite categories have no concrete fixture pairs; "Ignoring blank lines" is ambiguous). Add rules or reclassify autolinked URLs and hoisted bold. Pin "list-marker style" to the bullet character and the delimiter, not the start number. Give each row a fixture pair and refer to the table from Context and the criteria.
3. **Replace "harmless" with observable old-recipe outcomes** (addresses: Old-recipe "harmless" pull). State that no file write happens on equivalence, and assert the new marker after the rewrite.
4. **Resolve the Jira position** (addresses: Jira equivalence goes beyond the Linear-only title and Summary; No criterion for a Jira pull; Jira converter steps named two different ways). Either split Jira into a sibling item, or retitle the item to be tracker-neutral, use tracker-neutral body wording, and add a Jira pull criterion that unifies the converter names.
5. **Consider splitting Defect 1 out as a blocking bug** (addresses: Several separately deliverable pieces of work bundled; The `bug` kind understates). Keep 0296 as the equivalence, digest and migration story, or make it a parent.
6. **Sharpen the weak criteria** (addresses: Dossier criterion can pass without the fix; The failed read-back criterion only rules out one outcome; Several recovery cases have no criterion; The golden-digest criterion passes trivially). Require noise in the untouched section. Assert positive outcomes for a failed read-back. Express recovery as a table across all three states and both outcomes. Record the parser version and marker in the golden fixture.
7. **Specify title-only remote edits and their owner** (addresses: A title-only remote edit has no expected outcome; Title sync is deferred to 0290 but 0290 does not cover the title; The dossier title-difference line overlaps title sync). Add a `synced` and no-write criterion, and name an owner for title sync.
8. **Fix the Dependencies section** (addresses: 0320 depends on 0296; 0290 baseline coordination; Recovery of 0276 and 0293 needs 0320; Linear serialisation as an external coupling; Dossier consumers; Parser crate; 0321 no-wait). Add "Blocks: 0320" and a coordinated-change entry for 0290 with a landing order. Name Linear's serialiser and the `sync-work-items` skill as couplings, and note the 0321 no-wait decision.
9. **Housekeeping** (addresses: Open Questions says "None"; `relates_to` leaves out 0320 and 0321; The Summary does not say what the work delivers; The old digest recipe has no retirement point; Nothing re-measures the corpus symptom). Move the parser choice and the fixture capture into Open Questions or record them as decisions. Add 0320 and 0321 to `relates_to` and References. Add one sentence to the Summary describing the remedy. Name a retirement trigger for the old recipe. Add a corpus re-check.

## Per-Lens Results

### Clarity

**Summary**: The item is dense but precise. Ambiguity remains in four places: the noise category list differs between sections; the preview baseline expectation contradicts the migration criterion; the old-recipe "harmless pull" conflicts with the no-write rule; and the Jira converter naming and Linear-only wording drift.

**Strengths**: precise reproductions; equivalence defined once; named actors and observable outcomes; the Drafting Notes explain choices that look contradictory.

**Findings**:
- major / high — Requirements: Defect 2 / AC — Preview both records and does not record a baseline.
- major / high — Context / Equivalence / AC / Assumptions — The list of noise categories differs between sections.
- major / medium — Digest migration — The "harmless pull" under the old recipe conflicts with the no-write rule.
- minor / medium — Context / Required behaviour — "Authoritative for markdown representation" versus pulls writing the tracker serialisation.
- minor / high — AC (Jira) / Equivalence / Technical Notes — Jira converter steps named two different ways.
- minor / medium — Defect 1 / Required behaviour / AC — Linear-only wording ("remote description") used for behaviour that also covers Jira.
- minor / medium — Equivalence — "Ignoring blank lines" and "frontmatter keeps its existing comparison" allow more than one reading.
- minor / medium — AC / Required behaviour — Undefined status terms and loose phrasing.

### Completeness

**Summary**: Very complete for a bug. Every section is populated, and 18 criteria map to the requirement groups. The gaps are bookkeeping: Open Questions says "None", 0320 and 0321 are missing from `relates_to`, and the Summary does not describe the remedy.

**Strengths**: valid frontmatter; a reproduction structure for both defects; a Context that explains the forces; Requirements well beyond the symptoms; rationale recorded in the Drafting Notes.

**Findings**:
- minor / high — Open Questions — "None" while unverified assumptions and an undecided parser choice remain.
- minor / high — Frontmatter: relates_to — 0320 and 0321 are missing from `relates_to` and References.
- suggestion / medium — Summary — Describes the defects but not what the work delivers.

### Dependency

**Summary**: The ordering constraint inside the item and the 0321 boundary are well handled. However, "Blocks: none" hides 0320's dependency on the equivalence definition. The shared baseline schema with 0290 has no recorded order, and the external couplings (Linear's serialiser, the parser crate, the dossier consumers) are not named.

**Strengths**: the ordering constraint is a requirement; the authority claim is limited to a representation rule; the crate-boundary moves are visible in Technical Notes.

**Findings**:
- major / high — Dependencies — 0320 depends on 0296's equivalence definition, but "Blocks" is empty.
- major / medium — Digest migration — 0296 and 0290 both change the baseline `Entry` and digest recipe, with no order recorded.
- minor / high — Context — Recovery of 0276 and 0293 needs 0320.
- minor / medium — Assumptions — Linear's server-side re-serialisation is an external coupling missing from Dependencies.
- minor / medium — Required behaviour — Dossier shape changes affect the `sync-work-items` skill and 0213's evals.
- minor / medium — Out of scope — Title sync is deferred to 0290, but 0290 does not cover the title.
- suggestion / medium — Technical Notes — The new pinned parser crate and crate-boundary moves are not listed as prerequisites.
- suggestion / low — Dependencies — 0321 may revise the representation rule; the no-wait decision is recorded only in 0321.

### Scope

**Summary**: The item is coherent around one purpose, and its boundaries against its siblings are clear. But it bundles several separately shippable pieces of work, closer in size to a story or small epic than a bug, and the Jira part goes beyond the Linear-only title and Summary.

**Strengths**: a precise Out of scope section with named siblings; a closed equivalence definition; recovery tied to the same mechanism; split decisions recorded.

**Findings**:
- major / medium — Requirements — Several separately deliverable pieces of work bundled into one bug.
- major / medium — Requirements — Jira equivalence goes beyond the Linear-only title and Summary.
- minor / medium — Frontmatter: kind — The `bug` kind understates a change that includes a schema migration.
- minor / medium — Requirements — The dossier title-difference line is new capability that overlaps title sync.
- suggestion / low — Requirements — The old digest recipe has no retirement point.

### Testability

**Summary**: Unusually strong for a bug. The outcomes are observable, the negative criteria are present, and the comparison is deterministic. The gaps are at the edges: two noise categories are uncovered, the preview contradiction, no Jira pull criterion, no criterion for a title-only edit, and several criteria whose preconditions let them pass without the fix.

**Strengths**: precise reproductions; negative criteria; observable outcomes; deterministic equivalence; enumerated non-rewriting runs.

**Findings**:
- major / high — Equivalence / AC — Autolinked URLs and hoisted bold are neither in the equivalence rules nor in any criterion.
- major / high — Defect 2 / Digest migration — The noise-only reproduction expects `--preview` to record a baseline, but migration says preview does not rewrite.
- major / high — AC — No criterion covers stripping the title line on a Jira pull or what a Jira pulled body should contain.
- major / high — AC (dossier) — The dossier criterion's setup does not require noise in the untouched section, so it can pass without the fix.
- major / medium — Out of scope — The changed behaviour for a title-only remote edit has no expected outcome.
- minor / high — AC (failed read-back) — The criterion only rules out one outcome.
- minor / medium — Recovery — Several recovery cases have no criterion.
- minor / medium — Digest migration — "Harmless" is not measurable, and noise on unmarked entries has no criterion.
- minor / medium — AC — The rewrite categories have no concrete fixture pairs, and the renumbering boundary is unclear.
- minor / medium — AC (golden digest) — The criterion passes trivially, and nothing enforces the recipe-marker bump.
- suggestion / medium — Summary — Nothing re-measures the corpus symptom.
- suggestion / low — Required behaviour — The ordering requirement cannot be checked by a behavioural test.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-10T19:09:20+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity + Testability**: Preview both records and does not record a baseline — Resolved
- 🟡 **Clarity + Testability**: Noise categories inconsistent — Resolved (13-row Equivalence table, 0320 aligned)
- 🟡 **Clarity + Testability**: Old-recipe "harmless" pull — Partially resolved (outcomes stated; the "falling back" clause now conflicts)
- 🟡 **Testability + Clarity**: No Jira pull criterion; Linear-only wording — Partially resolved (pull criterion added; other criteria still Linear-worded)
- 🟡 **Testability**: Dossier criterion passes without the fix — Resolved
- 🟡 **Testability**: Title-only remote edit outcome — Resolved (title added to 0290)
- 🟡 **Scope**: Bundled deliverables — Still present (author decision: kept whole, kind changed to `story`)
- 🟡 **Scope**: Jira beyond Linear-only title — Resolved (retitled, file renamed)
- 🟡 **Dependency**: 0320 blocked by 0296 — Resolved
- 🟡 **Dependency**: Shared baseline with 0290 — Resolved
- 🔵 Minors and suggestions — Resolved, except the old-recipe follow-up (still untracked) and the ordering rule (still review-only)

### New Issues Introduced

- 🟡 **Testability + Dependency**: The corpus criterion depends on the live repo and on Linear state, so it can't be repeated, and it needs credentialed access to the PP workspace
- 🟡 **Testability + Completeness**: `--preview` never writing `last-sync.json`, and the noise-only preview outcome, have no criterion
- 🟡 **Testability + Clarity**: Jira coverage is thin; the re-sync, recovery and migration criteria are still worded for Linear only
- 🟡 **Clarity**: Old-recipe rules conflict on when equivalence applies ("falling back … when both sides changed" against remote-only equivalence)
- 🟡 **Clarity**: The outcome of `locally-modified` for a change to frontmatter only is undefined (push or not, and what is sent)
- 🟡 **Dependency**: Nobody owns updating the `sync-work-items` skill and 0213's evals for the dossier changes
- 🔵 **Testability**: Recovery-table not-equivalent rows say "unchanged", which conflicts with `finalise_baseline` blanking `local_hash`; "File written" doesn't say which file
- 🔵 **Testability**: The format of the title-difference line is unspecified
- 🔵 **Testability**: Ordered-list start numbers, local-only noise under the old recipe, and an example for inserted blank lines have no criterion
- 🔵 **Clarity**: Near-synonyms are used for the same concepts ("noise" and "cosmetic", blank and empty hash, the conflict state names); the old recipe is never named
- 🔵 **Dependency**: Jira ADF and the `adf` converter are not named as couplings; the sibling items that touch the pull path (0285, 0291, 0290) are not named
- 🔵 **Completeness**: The story doesn't name its beneficiary; Jira's `underline` loss has no criterion
- 🔵 Suggestions: 0146's Stories list omits 0296, 0320 and 0321; the 0290 Blocks entry doesn't mention the title-detection gap; the meaning of "fails on drift" is unclear

### Assessment

All ten major findings from pass 1 are resolved or settled by an author decision. The new major findings are narrower, and most come from edits in this pass, in places that are now precise enough for edge cases to show. None of them is structural. Five of the six need a sentence or a criterion each. The sixth, the owner of the skill and eval update, needs a decision. The item is close to ready, but under the major-count threshold it remains REVISE.

## Re-Review (Pass 3) — 2026-10-10T20:25:19+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability + Dependency**: Corpus criterion depends on live data — Partially resolved (the fixture split is in place; the merge-time check can pass trivially)
- 🟡 **Testability + Completeness**: No criterion for the preview rules — Resolved
- 🟡 **Testability + Clarity**: Jira coverage thin — Resolved (minor gaps remain in the Jira equivalence criterion)
- 🟡 **Clarity**: Old-recipe rules conflict — Partially resolved (the rule for a local-only equivalent change is still inconsistent)
- 🟡 **Clarity**: Outcome of a frontmatter-only `locally-modified` change — Resolved (its criterion asserts only the reported state)
- 🟡 **Dependency**: Owner of the dossier consumer update — Resolved
- 🟡 **Scope**: Bundled deliverables — Still present (accepted by the author)
- 🔵 Pass-2 minors — Resolved, except the review-only ordering rule (accepted)

### New Issues Introduced

- 🟡 **Clarity + Testability**: The recovery table and conflict note still name a single `local_hash` and "both hashes" after the digest split
- 🟡 **Clarity + Testability**: Undefined outcome for a local-only change whose body is equivalent to the remote; no criterion covers local-only noise under the new recipe
- 🟡 **Testability**: The merge-time corpus check passes trivially, because equivalent sections are omitted, so a noise-only conflict produces an empty dossier
- 🔵 **Clarity**: "Locally changed" doesn't say which split hash it covers; "as the tracker serialises it" is ambiguous for Jira; Context asserts the cause of the missing baselines as fact; the Summary uses terms before defining them and overstates the migration; "differs from the baseline" compares a body with a digest
- 🔵 **Testability**: Thin near-miss coverage for over-normalisation; the Jira equivalence criterion only covers equality and rejection; the frontmatter-only criterion asserts only the reported state
- 🔵 **Dependency**: 0203 is also gated on 0320, and nothing tracks re-pushing the gated items; the crate restructuring and its layering constraint aren't listed in Dependencies
- 🔵 **Completeness**: 0285 and 0291 are missing from `relates_to` and References; the relationship to 0213 isn't explained
- 🔵 Suggestions: the parser-approval gate, a thin title-detection follow-up, making the ordering rule a permanent regression test

### Assessment

The findings are narrowing at every pass. All remaining major findings sit in the area where the digest split meets recovery and local-side equivalence. None of them is structural, and each needs a defined rule or a criterion. The item remains REVISE under the major-count threshold, but one more focused edit should leave it ready.

## Re-Review (Pass 4) — 2026-10-10T20:33:37+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity + Testability**: Single `local_hash` after the split — Resolved
- 🟡 **Clarity + Testability**: Local-only equivalent change undefined — Resolved (one rule: baseline, update only on a title difference); the old-recipe edge is still open (see below)
- 🟡 **Testability**: Merge-time corpus check could pass trivially — Partially resolved (no longer trivial, but its equivalence test is circular)
- 🟡 **Scope**: Bundled deliverables — Still present (accepted; new argument about 0291 waiting on the whole item)
- 🔵 Pass-3 minors — Resolved, except that the 0285 ordering is stale (0285 is `done`)

### New Issues Introduced

- 🟡 **Clarity**: Several criteria don't say which digest recipe they assume, though their outcome depends on it
- 🟡 **Clarity**: "Local frontmatter hash changed" is undefined for old-recipe entries, which store one combined hash
- 🟡 **Testability**: No criterion would fail if digests didn't absorb noise, because the equivalence fallback gives the same outcomes
- 🟡 **Testability**: The Jira cosmetic-rewrite criterion has no input that can be built, because the table catalogues only Linear's rewrites
- 🟡 **Scope**: The reason for keeping Defect 1 in the item makes 0291 wait for the whole effort
- 🔵 **Clarity**: `conflict` and `unresolved conflict`; which recovery row the five corpus items fall into; the referents of "Defect 1 criteria" and "every pull entry point"; "the only field a push carries"; "seen only in a conflict dossier"; Jira "cosmetic"; "either way"
- 🔵 **Testability**: Stale-dossier precondition; negative or missing reported states; the title-difference branch untested; "still classified under the old recipe" isn't observable; untested behaviours (converter-rejected digests, preview agreeing with apply); near-misses missing for the loosened rules; provenance of the corpus fixtures
- 🔵 **Dependency**: The 0285 ordering is stale; 0291's ordering is recorded on one side only; the old-recipe removal follow-up is untracked; 0213's skill lint isn't recorded; the crate decision is outside the pre-work gate; 0230 writes baselines
- 🔵 **Completeness**: No Jira reproduction in Defect 2

### Assessment

The findings have stopped converging. Each pass resolves the previous majors and turns up roughly as many new ones, and each new one lies deeper in the edge cases. The item has grown to about 500 lines and 33 criteria. That supports the scope lens: the item now carries the detail of an implementation plan, and the review keeps finding gaps in that detail. Splitting the item or moving to planning would make progress faster than another review pass.

## Re-Review (Pass 5) — 2026-10-10T20:41:46+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Criteria don't say which recipe they assume — Resolved (new-recipe default stated)
- 🟡 **Clarity**: Old-recipe frontmatter test — Resolved
- 🟡 **Testability**: No test that digests absorb noise — Resolved
- 🟡 **Testability**: Jira cosmetic input couldn't be built — Resolved (clause dropped; Jira marked preventive)
- 🟡 **Scope**: Bundling and the Defect 1 split — Still present (accepted)
- 🔵 Pass-4 minors — Resolved, except the old-recipe removal follow-up (untracked)

### New Issues Introduced

- 🟡 **Clarity**: The `synced` outcomes in the recovery table contradict the reporting rule ("a missing digest counts as changed", so an old-recipe entry with an empty `local_hash` would be `locally-modified`)
- 🟡 **Clarity**: "Old recipe" becomes ambiguous once 0290 or a parser bump supersedes this item's recipe
- 🟡 **Testability**: Which outcomes "rewrite" an entry is undefined, so whether an unchanged unmarked entry migrates can't be decided
- 🟡 **Testability + Clarity + Dependency**: The merge-time corpus check still depends on judgement against moving live state
- 🟡 **Dependency + Testability**: The recovery fixtures come from dossiers that exist only on this machine and can be overwritten
- 🟡 **Dependency**: 0230 (in progress) changes the same pull and baseline code with no ordering recorded
- 🟡 **Scope**: Preventive Jira work bundled in (accepted earlier; new angle: it is now explicitly preventive)
- 🔵 Minors: the recipe-marker field is unnamed; the open questions that block the start aren't marked; two criteria omit their reported state; Jira coverage of the other pull entry points and of the dossier; near-misses for task-marker state, heading level and paragraph breaks; the source of the golden fixture's parser version; `blocks:` frontmatter for 0320 and 0290; whether title overwrites count as content; the `remotely-modified` precondition; projection terminology; the order of the Jira digest pipeline; the old-recipe follow-up is untracked. One claim (0213's skill lint and evals) was discarded after checking: neither exists.

### Assessment

The count of major findings has gone 10 → 6 → 3 → 6 → 6 across five passes. Each pass resolves the previous set and turns up new edge cases at about the same rate, now around migration semantics, recovery precedence and fixture provenance. The item is at 540 lines and 39 criteria. More review passes are unlikely to reach APPROVE. The open questions are now implementation-design questions, better settled in a plan or by splitting the item than in more work-item text.

## Verdict Change — 2026-10-10T20:44:49+00:00

**Verdict:** COMMENT

The pass-5 majors were applied after that pass without a further review:

- recovery precedence (a missing local hash counts as unknown);
- the legacy recipe, named;
- every classified entry is rewritten under the new recipe;
- a mechanical corpus check;
- fixtures captured with `accelerator linear show`;
- coordination with 0230.

The author changed the verdict to COMMENT. Further review passes were turning up implementation-design edge cases at a steady rate, and these are better settled in `/create-plan`. Accepted and outstanding:

- the scope bundling, including the preventive Jira work;
- the untracked work item to remove the legacy recipe.
