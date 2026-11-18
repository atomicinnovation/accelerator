---
type: "work-item-review"
id: "0307-pass-conduct-spawn-context-by-reference-review-1"
title: "Work Item Review: Pass Conduct Spawn Context by Reference"
date: "2026-11-18T18:34:43+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0121"
target: "work-item:0307"
work_item_id: "0307"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["research", "deep-research", "conduct", "cli"]
last_updated: "2026-11-18T20:00:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Pass Conduct Spawn Context by Reference

**Verdict:** REVISE

A well-motivated, precise and unusually thorough story: the Context
quantifies the problem, the snapshot semantics and failure outcomes are
pinned down, and dependencies on 0303, 0304 and 0310 are mapped in
detail. It needs revision because the contract for the node query's
output (notably `id`) is inconsistent across sections, several stated
requirements (backward-compatible ledger decoding, replace-on-offer,
the untrusted-question move) have no acceptance criterion, and the
criteria about LLM-generated prompts don't say which artefact a
verifier checks. Scope is large but coherent; the template-by-path and
search-budget threads could be justified or split.

### Cross-Cutting Themes

- **The node query's output contract is scattered and `id` is
  undefined** (flagged by: clarity, completeness, testability) — the
  field list differs between **Node context query**, **Search budget by
  query** and the first AC; `id` appears only in the AC, collides with
  the `--node` "node id", and has no format.
- **Requirements and ACs don't line up one-to-one** (flagged by:
  completeness, testability, clarity) — some behaviours exist only in
  ACs/Technical Notes (plan-output change, template path resolution,
  allowed-tools grant, failure outcomes, query error contract); others
  exist only in Requirements/Dependencies (snapshot decoding default,
  replace-on-offer, untrusted-question statement, inline composer
  questions, re-issued spawns); and the outputter-path list differs.
- **Re-issued spawns (0303) are ambiguous** (flagged by: clarity,
  testability, dependency) — unclear whether new behaviour is needed,
  what "its per-response ceiling" refers to, and 0303's landing status
  is unstated.

### Findings

#### Major

- 🟡 **Testability**: Prompt-content and agent-behaviour criteria lack a
  defined verification artefact
  **Location**: Acceptance Criteria
  Criteria about what a spawn prompt "carries exactly", inlined
  templates, budget stamping and summary outcomes describe runtime LLM
  output without saying whether SKILL.md step 4, the agent definition,
  profile/outputter text or a live run is checked.
- 🟡 **Testability**: Snapshot backward-compatible decoding and
  replace-on-offer have no criteria
  **Location**: Requirements: Offered-node snapshot
  An implementation could break decoding of pre-existing ledgers (the
  0310 hazard) or accumulate snapshots across offers and still pass
  every AC.
- 🟡 **Clarity / Testability / Completeness**: `id` is overloaded and
  its value undefined
  **Location**: Acceptance Criteria; Requirements: Node context query
  The `--node` "node id" is the spawn ref, but the printed `id` is
  "computed from the slug and spawn ref" with no format, and it is
  missing from the requirement's field list.

#### Minor

- 🔵 **Completeness**: Several AC behaviours have no matching
  requirement
  **Location**: Requirements
  Plan-output field removal, template path resolution and its
  path-printing mode, the allowed-tools grant, the three failure
  outcomes with troubleshooting/docs rows, and the query's error
  contract appear only in ACs or Technical Notes.
- 🔵 **Clarity**: Node query output fields are listed differently in
  three places
  **Location**: Requirements
  No single section gives the complete output shape (base fields,
  `id`, conditional `search_budget`).
- 🔵 **Clarity**: One outputter path in the requirement, two in the AC
  **Location**: Requirements: Research-node spawns by reference
  The requirement's "only" list omits the finding outputter path the
  AC requires.
- 🔵 **Scope**: Template-by-path is a separately deliverable concern
  **Location**: Requirements: Template by path
  It touches composer and single_pass spawns, has no dependency on the
  node query or ledger, and accounts for four ACs.
- 🔵 **Testability**: Untrusted-question relocation has no criterion
  **Location**: Requirements: Untrusted questions
  The safety statement could be lost in the move without failing any
  AC.
- 🔵 **Testability**: Nothing pins composer and single_pass spawns
  keeping their inline question
  **Location**: Requirements: Unchanged spawns
  Converting every spawn to references would fail no AC.
- 🔵 **Testability / Clarity**: Re-issued capacity-refused spawns
  (0303) are untested and ambiguously worded
  **Location**: Dependencies
  No AC covers a re-issued spawn querying its node; "its per-response
  ceiling" has an unclear referent; it is unclear whether new work is
  needed.
- 🔵 **Dependency**: No stated order with 0308 for the shared
  research-guard change
  **Location**: Dependencies
  Both items edit the guard allowlist; "any other subcommand stays
  blocked" may conflict.
- 🔵 **Dependency**: Landing status of 0303 and 0310 not stated
  **Location**: Dependencies
  0304 is marked landed; the others aren't, leaving ownership of
  interactions unclear if the order is reversed.
- 🔵 **Clarity**: Project terms used without definition or link
  **Location**: Summary
  `conduct` vs `research-topic`, `outstanding`, pair, round, cap,
  lineage and bare "step 4"/"step 2".
- 🔵 **Clarity**: "Takes no `path` key" reads as contradicting the
  printed `path`
  **Location**: Drafting Notes
  It means no `--path` argument, but "key" suggests an output field.
- 🔵 **Clarity**: One snapshot ledger field or several?
  **Location**: Requirements: Offered-node snapshot
  Singular "field" vs plural "snapshot fields" and the
  `pending.spawns`-as-objects alternative.

#### Suggestions

- 🔵 **Scope**: Moving the search budget into the node query is
  optional extra scope
  **Location**: Requirements: Search budget by query / leaves the prompt
  Not required by the stated problem and not mentioned in the Summary;
  justify (reference-only prompt invariant) or defer.
- 🔵 **Dependency**: 0283 is linked but its coupling is not described
  **Location**: Frontmatter: relates_to
  Every other related item has a Dependencies entry; 0283 does not.
- 🔵 **Testability**: Error-path and guard criteria leave small
  ambiguities
  **Location**: Acceptance Criteria
  Unspecified exit code/stderr shape, unlisted blocked subcommands, no
  per-spawn size bound.
- 🔵 **Clarity**: Command name open in one place, fixed in another;
  absent `search_budget` omitted vs null
  **Location**: Requirements
  "Settled in implementation" vs the fixed allow rule
  `Bash(accelerator research topic node *)`.

### Strengths

- ✅ The Context quantifies the problem (spawn counts, prompt sizes,
  output-token cost, per-response ceiling) and separates the real
  cost (orchestrator generation) from the non-issue (agents reading).
- ✅ Snapshot semantics are precise: what's recorded, when replaced,
  how it defaults, and why sibling writes can't change
  `known_questions`.
- ✅ Edge cases are enumerated: unknown node, superseded run, withheld
  web node after a quota halt vs one in the halting batch, web vs
  non-web budgets.
- ✅ Failure outcomes have exact names, so they can't be paraphrased
  inconsistently.
- ✅ Dependencies are unusually thorough: 0304's landed status and the
  exact tests that change, 0310's decoding constraint, 0303's
  re-issued spawns, and permission couplings.
- ✅ Explicit boundaries (composer and single_pass spawns unchanged
  bar template path) and Drafting Notes recording the user's design
  decisions prevent scope drift.
- ✅ Assumptions record accepted trade-offs (custom researcher
  breakage, override-outside-project cost, ledger growth).

### Recommended Changes

1. **Make Node context query the single authoritative output
   contract** (addresses: `id` overloaded; output fields listed in
   three places; command name open vs fixed)
   List every printed field there, including `id` with its format and
   an example (distinct name from the `--node` "node id" if they
   differ) and `search_budget` for web nodes only (state omitted vs
   null). Fix the command name or note the allow rule follows it.
2. **Reframe prompt/behaviour ACs against the producing artefacts**
   (addresses: verification artefact)
   E.g. "step 4's research-node spawn bullet in `research-topic`
   SKILL.md injects exactly …"; "the researcher agent definition
   instructs reporting 'node query failed' when …". Note whether a
   live-run smoke check is also expected.
3. **Add ACs for uncovered requirements** (addresses: snapshot
   decoding/replace-on-offer; untrusted questions; unchanged spawns;
   re-issued spawns)
   An old ledger without the snapshot decodes with an empty snapshot;
   a second offer replaces the first (an id only in the first batch
   then fails); the researcher definition carries the untrusted-question
   statement and step 4 no longer does; composer/single_pass prompts
   keep an inline question and no node id; a re-issued spawn's query
   answers from the snapshot.
4. **Add requirement bullets for AC-only behaviours** (addresses:
   ACs without requirements; outputter paths)
   Plan output drops `question`/`known_questions`; template path
   resolution (override vs plugin default) via a path-printing mode
   plus the allowed-tools grant; the three failure outcomes with
   troubleshooting rows and fallback allow rule; the query's error
   contract. Name both level-note and finding outputter paths in the
   research-node reference list.
5. **Tighten Dependencies** (addresses: 0308 order; 0303/0310 status;
   0303 wording; 0283)
   State each sibling's status, the order relative to 0308 and who
   reconciles the guard allowlist, whether re-issued spawns need new
   behaviour (and name the ceiling's subject), and add or drop 0283.
6. **Justify or split the secondary threads** (addresses:
   template-by-path; search budget scope)
   Either split template-by-path into a sibling story under 0121 and
   defer the budget move, or add a sentence to the Summary or Drafting
   Notes explaining why they ship together (e.g. a reference-only
   prompt invariant).
7. **Small clarity fixes** (addresses: project terms; `path` key;
   snapshot field count)
   Say once that `conduct` is a verb of `research-topic` and that
   "step 4" is in its SKILL.md; reword "takes no `path` key" as "no
   `--path` argument"; decide singular field vs extended
   `pending.spawns` or leave it explicitly to implementation.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The item is precise for its size. The problems are local:
`id` means two different things, the output fields and outputter paths
are listed differently in Requirements and ACs, one pronoun in
Dependencies is unclear, and several project terms are undefined.

**Strengths**:
- Named requirements, referred to by name from the ACs.
- The Context separates the real problem from the non-problem.
- Snapshot semantics are fully pinned.
- Failure outcomes have exact names.
- Drafting Notes record why alternatives were rejected.

**Findings**:
- 🟡 major / high — Acceptance Criteria — `id` means two different
  things. The `--node` id is the spawn ref; the printed `id` is
  computed from slug and ref, undefined, and missing from the
  requirement.
- 🔵 minor / high — Requirements — The node query's output fields are
  listed differently in three places.
- 🔵 minor / high — Requirements — One outputter path in the
  requirement, two in the AC.
- 🔵 minor / medium — Dependencies — Unclear pronoun and obligation in
  the 0303 entry.
- 🔵 minor / medium — Summary — Project terms used without definition
  or link.
- 🔵 minor / medium — Drafting Notes — "Takes no `path` key" reads as
  contradicting the printed `path`.
- 🔵 minor / medium — Requirements — One snapshot ledger field, or
  several?
- 🔵 suggestion / medium — Requirements — Command name open vs fixed;
  absent `search_budget` omitted vs null.

### Completeness

**Summary**: Very complete. Every section is present and has real
content, and the frontmatter is fully populated. The one gap is that
several AC behaviours have no matching requirement.

**Strengths**:
- A quantified Context.
- Complete frontmatter.
- Thorough Dependencies.
- Explicit Assumptions.
- Open Questions resolved.

**Findings**:
- 🔵 minor / high — Requirements — Several AC behaviours have no
  matching requirement (plan output, template path resolution,
  allowed-tools grant, failure outcomes and docs, the query's error
  contract, the `id` field).

### Dependency

**Summary**: The dependency mapping is unusually thorough (0304 fully
captured, 0310 decoding constraint, no upstream blockers). The gaps
are order and status among sibling items, plus an unexplained link to
0283.

**Strengths**:
- 0304 captured in full.
- The 0310 constraint is precise.
- The re-issued-spawn obligation from 0303 is explicit.
- The custom researcher breakage is recorded.
- Permission couplings are captured.

**Findings**:
- 🔵 minor / medium — Dependencies — No stated order with 0308 for the
  shared research-guard change.
- 🔵 minor / medium — Dependencies — Landing status of 0303 and 0310
  not stated.
- 🔵 suggestion / medium — Frontmatter: relates_to — 0283 is linked
  but its coupling is not described.

### Scope

**Summary**: One clear purpose, with tightly interdependent core
requirements. It is at the large end for a story (15 ACs). The
template-by-path thread could be delivered on its own, and the
search-budget move is optional extra scope.

**Strengths**:
- An explicit boundary for composer and single_pass spawns.
- Precise edges with 0303, 0304, 0308 and 0310.
- Drafting Notes record the scope decisions.
- A story under epic 0121 is an appropriate kind.

**Findings**:
- 🔵 minor / medium — Requirements: Template by path — a separately
  deliverable concern; split it or justify bundling.
- 🔵 suggestion / low — Requirements: Search budget — optional extra
  scope; defer it or justify it in the Summary.

### Testability

**Summary**: Most criteria are precondition/action/outcome with
concrete names and edge cases. The gaps are LLM-prompt criteria with
no defined artefact, requirements with no criterion, and an undefined
`id`.

**Strengths**:
- Concrete preconditions, actions and outcomes.
- A deterministic snapshot oracle.
- Enumerated edge cases.
- A named regression map of the tests that change.

**Findings**:
- 🟡 major / high — Acceptance Criteria — Prompt-content and
  agent-behaviour criteria lack a defined verification artefact.
- 🟡 major / high — Requirements: Offered-node snapshot — Backward-
  compatible decoding and replace-on-offer have no criteria.
- 🔵 minor / high — Acceptance Criteria — Expected value of `id` is
  undefined.
- 🔵 minor / high — Requirements: Untrusted questions — no criterion.
- 🔵 minor / medium — Requirements: Unchanged spawns — no criterion
  pins the inline question.
- 🔵 minor / medium — Dependencies — Re-issued spawns (0303) have no
  criterion.
- 🔵 suggestion / medium — Acceptance Criteria — Exit code, blocked
  subcommands and the size bound are unspecified.

## Re-Review (Pass 2) — 2026-11-18T18:49:19+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Testability**: Prompt-content and agent-behaviour criteria lack a
  defined verification artefact — Resolved
- 🟡 **Testability**: Snapshot backward-compatible decoding and
  replace-on-offer have no criteria — Resolved
- 🟡 **Clarity / Testability / Completeness**: `id` is overloaded and its
  value undefined — Resolved
- 🔵 **Completeness**: Several AC behaviours have no matching requirement
  — Resolved
- 🔵 **Clarity**: Node query output fields listed differently in three
  places — Resolved
- 🔵 **Clarity**: One outputter path in the requirement, two in the AC —
  Resolved
- 🔵 **Scope**: Template-by-path is a separately deliverable concern —
  Partially resolved (bundling justified in Drafting Notes; scope still
  notes it could ship alone)
- 🔵 **Testability**: Untrusted-question relocation has no criterion —
  Resolved
- 🔵 **Testability**: Nothing pins composer and single_pass inline
  questions — Resolved
- 🔵 **Testability / Clarity**: Re-issued spawns (0303) untested and
  ambiguous — Resolved
- 🔵 **Dependency**: No stated order with 0308 — Resolved
- 🔵 **Dependency**: Landing status of 0303 and 0310 not stated —
  Resolved
- 🔵 **Clarity**: Project terms used without definition — Partially
  resolved (`conduct` and "step 4" defined; `lineage`, `cap`, "offer",
  "outputter" still unglossed)
- 🔵 **Clarity**: "Takes no `path` key" contradiction — Resolved
- 🔵 **Clarity**: One snapshot ledger field or several — Resolved
- 🔵 **Scope**: Search budget move is optional scope — Resolved
  (justified), though the Summary still omits it
- 🔵 **Dependency**: 0283 linked but undescribed — Resolved
- 🔵 **Testability**: Error-path and guard ambiguities — Partially
  resolved (stderr/stdout and an example blocked subcommand added; no
  ledger / unknown slug not covered)
- 🔵 **Clarity**: Command name open vs fixed — Resolved (now explicitly
  open, with dependents following it)

### New Issues Introduced

- 🟡 **Testability**: Snapshot criteria compare against a plan that no
  longer carries the values — the first and sibling-write ACs expect
  values "as offered" by the plan, but **Plan without node context**
  removes them, so a test has no independent oracle.
- 🔵 **Testability**: Query criterion does not assert the exact key set.
- 🔵 **Testability**: The search-budget oracle is defined against
  pre-change rendering as well as `SearchBudgets::at(level)`.
- 🔵 **Testability**: The template-path criterion doesn't name the
  path-printing invocation or its output, nor that step 4 uses it.
- 🔵 **Testability**: No criterion for a non-web note omitting
  `search_budget`, or for a query against an unknown slug / no ledger.
- 🔵 **Clarity**: What counts as a "new offer" (a repeat `outstanding`
  before `--spawned`) is undefined.
- 🔵 **Clarity**: Unclear whether the plan's node entries keep
  `search_budget`.
- 🔵 **Clarity**: The **Unchanged spawns** heading contradicts its body.
- 🔵 **Scope**: The story is at the upper bound of size (20 ACs); a
  snapshot-plus-query / switch-to-references split is possible.
- 🔵 **Dependency**: 0308's possible use of the snapshot is not recorded
  as a conditional block.
- 🔵 **Dependency**: Accepting the custom-researcher break depends on
  landing before the first release, which is untracked.
- 🔵 Suggestions: name the beneficiary (completeness); list the open
  command name under Open Questions (completeness); mention the budget
  move in the Summary (scope); align "superseded" vs "not current" run
  wording (clarity); list 0283-era tests pinning inlined context
  (dependency); a manual smoke check of query-first (testability).

### Assessment

All three original major findings are resolved and the verdict moves
from REVISE to COMMENT. One new major remains: the snapshot criteria
need an expected value that doesn't come from the plan, for example a
fixture tree whose `known_questions` are fixed before a sibling write.
The new minors are small tightening edits. The work item is acceptable
for planning, but fixing the snapshot oracle and the "new offer"
definition first would make implementation unambiguous.

## Re-Review (Pass 3) — 2026-11-18T19:12:02+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Testability**: Snapshot criteria compare against a plan that no
  longer carries the values — Resolved (fixture literals)
- 🔵 **Testability**: Query criterion does not assert the exact key set —
  Resolved (a suggestion remains to pin the web fixture's eight keys and
  budget value)
- 🔵 **Testability**: Search-budget oracle defined against pre-change
  rendering — Resolved
- 🔵 **Testability**: Template-path criterion doesn't name the invocation
  — Resolved (the mode's flag is still unnamed; see new issues)
- 🔵 **Testability**: Non-web note omission and missing-ledger errors —
  Resolved
- 🔵 **Clarity**: What counts as a "new offer" — Resolved
- 🔵 **Clarity**: Whether the plan keeps `search_budget` — Resolved
- 🔵 **Clarity**: **Unchanged spawns** heading contradicts its body —
  Resolved
- 🔵 **Scope**: Template-by-path is a separable slice — Still present
  (bundled by decision; scope asks why composer and single_pass template
  moves belong here)
- 🔵 **Scope**: Story size at the upper bound — Partially resolved (the
  rejected split is recorded; the template / node-context seam is not)
- 🔵 **Dependency**: 0308's possible use of the snapshot — Partially
  resolved (a conditional block is recorded; who decides is not)
- 🔵 **Dependency**: Release-timing condition untracked — Partially
  resolved (stated, but not tied to a release or item)
- 🔵 Suggestions from pass 2 (beneficiary, command name in Open
  Questions, budget move in Summary, run wording, pinned tests, smoke
  check) — Resolved

### New Issues Introduced

- 🔵 **Testability**: No criterion for a node newly added by a repeat
  offer getting a fresh snapshot.
- 🔵 **Testability**: The plan criterion doesn't assert that node entries
  keep every other field.
- 🔵 **Testability**: The query-error criterion bundles five cases a
  tester could cover partially.
- 🔵 **Testability**: The guard's negative case relies on "any other"
  subcommand.
- 🔵 **Clarity**: The web profile's budget source is stated
  unconditionally, though single_pass web spawns keep an inline budget.
- 🔵 **Clarity**: Which template each spawn type receives is not stated.
- 🔵 **Clarity**: Why the orchestrator still reads the plan's
  `search_budget` is unclear.
- 🔵 **Clarity / Completeness**: The path-printing mode is unnamed and,
  like the snapshot's shape, missing from Open Questions.
- 🔵 **Completeness**: The step 2 and docs plan-description updates appear
  only in Technical Notes.
- 🔵 **Dependency**: Who decides 0308's path source is untracked.
- 🔵 Suggestions: the "and so holds only the current batch" non-sequitur;
  glosses for composer, pair modes, research guard, withheld and quota;
  name the plan's other consumers; cover custom profiles and outputters
  in the release-timing assumption; a defined extraction rule for
  "injects exactly" structure tests; consider the template / node-context
  split.

### Assessment

No critical or major findings remain, and every earlier major is
resolved. The work item is ready for planning. The remaining minors
sharpen wording and close small gaps in the criteria; they can be fixed
now or during planning.

## Re-Review (Pass 4) — 2026-11-18T19:51:31+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Testability**: Repeat offer of a newly offered node — Resolved
- 🔵 **Testability**: Plan criterion doesn't assert that other fields are
  kept — Partially resolved (asserted against an unlisted baseline)
- 🔵 **Testability**: Bundled query-error cases — Resolved
- 🔵 **Testability**: Guard negative case is "any other" — Resolved
- 🔵 **Clarity**: Web profile budget source stated unconditionally —
  Resolved
- 🔵 **Clarity**: Which template each spawn receives — Resolved
- 🔵 **Clarity**: Why the plan keeps `search_budget` — Resolved
- 🔵 **Clarity / Completeness**: Path-printing mode and snapshot shape
  missing from Open Questions — Resolved
- 🔵 **Completeness**: Step 2 and docs updates only in Technical Notes —
  Partially resolved (now a requirement, but no criterion)
- 🔵 **Dependency**: Who decides 0308's path source — Resolved (a
  suggestion remains to mirror it on 0308)
- 🔵 **Dependency**: Release timing untracked — Partially resolved (tied
  to 0121, but not in **Blocks**)
- 🔵 **Scope**: Template-by-path bundling and story size — Resolved by
  recorded decision (scope now suggests only planning internal phases)
- 🔵 Suggestions from pass 3 — Resolved

### New Issues Introduced

- 🔵 **Testability**: The repeat-plan criterion depends on fixture
  topology (pending, dropped and new nodes, and the sibling's question)
  the item doesn't define.
- 🔵 **Testability**: Researchers acting on the query result is checked
  only through instruction text; the smoke run has no pass condition.
- 🔵 **Clarity**: Spawn ref, offer, run ledger, pending batch and the
  `outstanding` / `--spawned` cycle are used before they are defined,
  or never defined.
- 🔵 **Clarity**: "Halting batch" is undefined, as is whether the halt
  leaves it as the current offered batch.
- 🔵 **Clarity**: Whether an ended (`end-run`) run is still "current" for
  the query.
- 🔵 **Clarity**: The channel failure outcomes are reported through is
  implicit.
- 🔵 Suggestions: the elliptical Summary clause; phrase the successive-offer
  snapshot check through the query rather than the ledger shape; land the
  path-printing mode as a prerequisite chore; plan internal phases;
  mirror the 0308 coupling on 0308.

### Assessment

No critical or major findings for a second pass running. Each pass's
minors are now finer-grained refinements of criteria and definitions
rather than gaps. The work item is ready for planning; the remaining
minors are optional polish.

## Verdict Change — 2026-11-18T20:00:26+00:00

**Verdict:** APPROVE

After pass 4, the remaining minor findings were addressed in one edit
without a further review pass: fixture topology for the repeat-plan
criteria, explicit plan-entry keys, a criterion for the step 2 and docs
updates, definitions (run ledger, offer, pending batch, spawn ref, halting
batch), ended-run and halting-batch semantics, the channel that carries
failure outcomes, 0121's release under **Blocks**, the accepted
agent-compliance risk with smoke-run pass conditions, and planning phases.
The reviewer marked the work item approved and ready for planning.
