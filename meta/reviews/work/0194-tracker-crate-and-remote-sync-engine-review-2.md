---
type: work-item-review
id: "0194-tracker-crate-and-remote-sync-engine-review-2"
title: "Work Item Review: Tracker Crate and Remote Sync Engine"
date: "2026-08-10T12:22:08+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0194"
relates_to: ["work-item-review:0194-tracker-crate-and-remote-sync-engine-review-1"]
work_item_id: "0194"
reviewer: Toby Clemson
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 2
review_pass: 3
tags: []
last_updated: "2026-08-10T16:34:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Tracker Crate and Remote Sync Engine

**Verdict:** REVISE

0194 is a strong, precise work item — the classification vocabulary,
decision-table signature, baseline storage contract and crate-boundary
invariants are all enumerated explicitly, sixteen acceptance criteria are
anchored to named bash oracles, and the 2026-08-10 correction pass visibly
raised its accuracy by reading the source scripts rather than trusting the
split-time summary. The findings below are almost all consequences of that
same pass: it added `--preview`, conflict resolution and the projection
seam without fully reconciling them against the sections written earlier,
and it left the item's dependency record stale. Two structural problems
carry the verdict — the bash parity suites are simultaneously the
acceptance oracle and the removal target, and the SKILL half of the
conflict flow is declared in scope but has no requirement, no criterion and
no named file.

### Cross-Cutting Themes

- **The bash suites are both the gate and the thing removed** (flagged by:
  clarity, testability, completeness) — the parity criterion runs against
  `test-work-item-sync-*.sh` and the classify/decide sections of
  `test-work-item-scripts.sh`, which another requirement deletes in the
  same change; "ported", "repointed" and "removed" are used for the same
  artefacts.
- **The SKILL half of the conflict flow is committed but invisible**
  (flagged by: completeness, testability, scope) — Phase C states the
  prompting and re-invocation land here, yet no requirement names the SKILL
  file, no criterion covers the round trip, and the conflict report's
  output contract — the one interface crossing a process edge — is
  unspecified.
- **Work that cannot complete or be verified inside this boundary**
  (flagged by: dependency, scope, testability) — composition-root provider
  wiring, the live contract suite, and per-provider projection fidelity all
  need 0171's adapters, while Dependencies asserts "no remaining
  blockers".
- **The dependency record is stale** (flagged by: dependency, completeness,
  scope) — 0170 completed 2026-08-07, three days before this item's last
  edit, yet `blocked_by` still names it, Phase D still reads as gated, and
  a Drafting Note's revisit trigger can never fire.
- **Undefined outcomes at the `--push` boundary** (flagged by: clarity,
  testability) — `decline` and `confirmed-local-fallback` have no trigger,
  the interaction model for `create --push` is never stated, and the
  `update --push` criterion asks the implementer to define the outcome it
  is meant to verify.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability / Clarity / Completeness**: Parity oracles named in the
  classification and parity criteria are the same bash suites the removal
  criterion deletes
  **Location**: Acceptance Criteria (classification parity, sync parity,
  removal); Requirements (script removal, coverage gap)
  The classification criterion verifies against "the bash
  `work-item-sync-classify.sh` parity fixtures" and the parity criterion
  runs "against the repointed `skills/work/scripts/test-work-item-sync-*.sh`
  gates, the classify/decide sections of `test-work-item-scripts.sh`" —
  while Requirements and the removal criterion delete exactly those
  artefacts in the same change. "Repointed" (bash harness re-aimed at the
  Rust binary) and "ported" (rewritten in Rust) are different
  implementations and the item asserts both. A verifier reaching the end
  state cannot run the suite the parity criterion names.

- 🟡 **Completeness / Testability / Scope**: The SKILL's prompting half is a
  named Phase C deliverable with no requirement, no criterion, no named
  file, and no report contract
  **Location**: Requirements; Acceptance Criteria; Technical Notes (Phasing)
  Technical Notes puts the conversation-side prompt and re-invocation in
  Phase C "not an afterthought", but no Requirements bullet describes the
  SKILL change, no criterion mentions it, and the SKILL is never named by
  path — while `skills/work/list-work-items/SKILL.md` is named precisely
  elsewhere. Separately, the conflict report the SKILL must parse is
  specified only as "a form the SKILL can render and prompt from": no
  format, no machine-parseability, no exit code.

- 🟡 **Dependency / Completeness / Scope**: `blocked_by: work-item:0170` and
  the Dependencies narrative are stale — 0170 is done
  **Location**: Frontmatter: blocked_by; Dependencies; Drafting Notes
  0170 carries `status: done` (validated 2026-08-07), three days before
  this item's `last_updated`. Technical Notes already contradicts the
  blocker ("already exist and are substantially built"), yet Dependencies
  still gates Phase D on it and a 2026-08-10 Drafting Note reasons about
  "if Phase D stalls waiting on 0170" — a revisit trigger that can never
  fire.

- 🟡 **Dependency / Scope**: The reverse coupling on 0171 — composition-root
  wiring and the live contract suite — is uncaptured, and Dependencies
  asserts the opposite
  **Location**: Dependencies; Requirements (composition root); Acceptance
  Criteria (contract suite)
  Requirements ask for the pipeline to run against "the active provider
  client, wired at the work binary's composition root", and a criterion
  requires a tagged suite exercising "real remote calls" — but no provider
  client exists until 0171, which this item blocks. Assumptions concede
  this obliquely; Dependencies states "no remaining blockers". The same
  applies to the projection-behind-the-port requirement, whose per-provider
  recipes are 0171's deliverable.

- 🟡 **Dependency**: Phase E cannot start before B, C and D — the claim that
  E is unblocked is wrong
  **Location**: Dependencies; Technical Notes (Phasing)
  Dependencies says "Phases A–C and E are unblocked and can start
  immediately", but Phase E deletes `work-item-create-remote.sh`,
  `work-item-update-remote.sh` and `work-item-push-decide.sh` — the three
  scripts whose replacements land in Phase D — and the sync-stage scripts B
  and C replace. Taking that note at face value breaks the sync and
  `--push` paths mid-migration, against the epic's stay-functional rule.

- 🟡 **Dependency**: Skill-level consumers of the nine removed scripts are
  never audited or required to be repointed
  **Location**: Requirements (script removal); Acceptance Criteria
  (removal)
  The item does a careful consumer audit for the two scripts it keeps —
  naming three live callers of `work-item-sync-label.sh` — but none for the
  nine it deletes. The removal requirement and criterion mention only
  `test-*.sh` suites, the superseded sections, and the suite floor; the
  SKILL.md files that shell out to `work-item-sync-*.sh`,
  `work-item-create-remote.sh` and `work-item-update-remote.sh` at runtime
  are never named. Any surviving caller breaks in the same commit that
  removes the tests which would have caught it.

- 🟡 **Completeness / Clarity / Testability**: The anti-drift gate for the
  ported `work-item-sync-label.sh` cites golden fixtures that do not cover
  labelling
  **Location**: Acceptance Criteria (sync parity); Technical Notes
  (fixtures)
  The no-shim decision is held safe "by testing **both** implementations
  against one shared set of golden fixtures", but the item's fixture
  inventory lists only `work-item-normalise.golden` and
  `work-item-project-remote.golden` — and `project-remote`'s bash side is
  deleted here, so it is not one of the surviving duplicated pair. Label
  coverage is described as a *section inside* `test-work-item-scripts.sh`,
  which the removal requirement deletes. The label port is also unassigned
  to any phase.

- 🟡 **Clarity / Testability**: `decline` and `confirmed-local-fallback`
  imply an interaction model never stated for `create --push`
  **Location**: Requirements (`--push` wiring); Acceptance Criteria (create
  `--push`)
  Both terms presuppose someone answering a question, yet the only
  interaction-model statement scopes non-interactivity narrowly to
  `accelerator work sync`, leaving unstated whether `create --push` may
  prompt or whether the SKILL owns that prompt too. Neither outcome has a
  trigger a test could produce, and "never silently duplicates a create on
  retry" is an unbounded negative with no scenario attached.

- 🟡 **Testability**: The `update --push` criterion asks the implementer to
  define the outcome it is meant to verify
  **Location**: Acceptance Criteria (update `--push`); Requirements
  The criterion requires the retryable-vs-terminal distinction to be
  surfaced "with the corresponding local-file outcome for each case", and
  the requirement instructs the implementer to "define" that outcome. The
  pass condition is therefore authored by the person being verified, and
  the highest-risk case (`E_DISPATCH_TERMINAL`, mutation state uncertain)
  has no stated expected local-file state at all.

- 🟡 **Testability**: The `--preview` no-mutation check verifies only the
  baseline document
  **Location**: Acceptance Criteria (`--preview`)
  A preview's dangerous failure modes are writing pulled content to local
  work-item files and issuing remote `create`/`update` calls — both pass a
  baseline-only byte-comparison. The criterion's other half, "the reported
  plan matches the actions a real run would take", names no procedure.

- 🟡 **Testability / Completeness**: The `finalise` run-start-epoch rule is
  required but no criterion verifies it
  **Location**: Requirements (resumability); Acceptance Criteria
  Requirements state "the global timestamp (run-START epoch) advances only
  on clean completion" and flag that a mis-advanced timestamp poisons the
  next run's mtime pre-filter. The resumability criterion covers only
  per-item write ordering and crash-then-rerun determinism; nothing asserts
  the timestamp is the run-*start* epoch, nor that a failed run leaves it
  unadvanced. The failure is silent and delayed.

- 🟡 **Clarity / Completeness**: Technical Notes defers to an Open Questions
  section that declares nothing outstanding
  **Location**: Technical Notes (shared scripts); Open Questions
  The shared-scripts bullet ends "Both stay; see Open Questions on whether
  to shim them", but Open Questions says "None outstanding" and Drafting
  Notes records the settled answer ("no shim"). A reader following the
  pointer cannot tell whether the decision is made or dropped.

- 🟡 **Scope**: The item has grown well past its post-split size
  **Location**: Requirements; Technical Notes (Phasing)
  0194 now carries 17 Requirements bullets and 16 Acceptance Criteria
  across five phases — a new crate, a state machine split over two crates,
  a command with three modes and a two-invocation protocol, `--push` wiring
  onto another story's commands, a SKILL flow, a characterization test, and
  nine script removals plus a suite-floor decrement. Sibling 0170 — itself
  carved out of this story for being epic-scale — shipped with 3
  Requirements and 8 criteria. The epic has already split 0169 into four
  and abandoned 0173 into three for the same reason, and this pass finding
  four previously-recorded claims wrong is the early signal of that drift.

#### Minor

- 🔵 **Clarity / Testability**: The no-provider-types criterion conflates a
  dependency-graph check with a signature check, and is not mechanised
  **Location**: Acceptance Criteria (tracker public API)
  "Verified by its dependency graph carrying no `reqwest` or provider-crate
  types in public signatures" blends two checks with different failure
  modes — and Assumptions explicitly accepts `reqwest` workspace-wide, so
  they are not interchangeable. Its sibling criterion for the `work`
  dependency is backed by a named cargo-pup rule; this one is left to
  inspection.

- 🔵 **Clarity**: "port" is used for both the hexagonal port and the
  bash-to-Rust translation
  **Location**: Requirements; Assumptions; Drafting Notes
  "`work-item-push-decide.sh` … needs a characterization test before the
  port replaces it" and "the first run after the port" both mean the Rust
  migration, not the `RemoteTracker` trait — after a dozen uses establishing
  the opposite reading.

- 🔵 **Clarity**: "the four bridge scripts" is never enumerated and no
  source is named for `show`
  **Location**: Requirements (tracker crate)
  Technical Notes names three bridge scripts, leaving `show` and
  `fetch_all` with at most one identified source between them — so the
  parity check the rest of the item leans on cannot be made for two of the
  four port operations.

- 🔵 **Clarity**: Several terms are used as if already defined
  **Location**: Requirements; Assumptions; Acceptance Criteria
  "presence-only", "mtime pre-filter" (the entire justification for the
  `--preview` invariant, mentioned nowhere else), "ADF", "renderable
  states", "the full contract", and "the configured pattern" for ID
  allocation all appear without gloss — unlike `work.integration` and
  `E_DISPATCH_*`, which are glossed well.

- 🔵 **Completeness / Testability**: The parity criterion enumerates fewer
  suites than Requirements say must be ported, and the bulk-fetch path has
  no criterion
  **Location**: Acceptance Criteria (sync parity)
  Requirements list nine covered scripts needing porting; the parity
  criterion names the `sync-*` gates, the classify/decide sections and two
  goldens — omitting baseline, label and `fetch-remote`. The port defines
  `fetch_all()` and Requirements assign bulk-vs-`show` orchestration to the
  caller, but nothing exercises it.

- 🔵 **Dependency**: Jira and Linear as external systems — and the
  credentials their contract suite needs — are absent from Dependencies
  **Location**: Dependencies; Assumptions
  Both providers are named throughout, with exact projection recipes and a
  criterion requiring real remote calls, yet neither appears in
  Dependencies and nothing records what the live suite needs: API
  credentials, a sandbox project or workspace, or tolerance for rate limits
  on the bulk fetch.

- 🔵 **Dependency**: The deferred removal of `work-item-sync-label.sh` and
  `work-item-normalise.sh` has no captured successor
  **Location**: Requirements (do-not-remove); Drafting Notes (scoped out)
  A knowingly-created duplication is left with no owner in the dependency
  graph — nothing connects it to 0174 (Retire Shell Tooling and CI Guards)
  or to a `/list-work-items` port.

- 🔵 **Dependency**: The `push-decide` characterization test is unassigned
  to a phase
  **Location**: Requirements (coverage gap); Technical Notes (Phasing)
  Requirements establish a hard ordering ("before the port replaces it"),
  but the five-phase breakdown never says which phase carries it, so the
  one genuine characterize-then-port constraint can slip into the slice it
  is meant to precede.

- 🔵 **Dependency**: Dual ownership of the `E_DISPATCH_*` taxonomy across
  the Rust port and the surviving bash bridges is uncaptured
  **Location**: Technical Notes (exit-code taxonomy)
  The remaining bridges live in the Jira and Linear skills and stay in bash
  until 0171, so the taxonomy has two implementations with no fixture,
  shared test or Blocks entry holding them in step — and it is exactly the
  retryable-vs-terminal semantics the `--push` criteria depend on.

- 🔵 **Testability**: Composition-root provider selection and the
  classifier's never-fetch rule carry no criteria
  **Location**: Requirements
  Nothing verifies that changing `work.integration` selects a different
  client, or what happens when it is absent or unknown; and nothing catches
  an accidental per-item fetch inside the classifier, which would surface
  as a rate-limit problem in production rather than a test failure.

- 🔵 **Clarity**: "the same golden fixtures" has no resolvable referent for
  the label implementation
  **Location**: Acceptance Criteria (sync parity)
  Covered by the label-fixture major finding above; recorded here as the
  clarity lens saw it independently.

#### Suggestions

- 🔵 **Testability**: No criterion checks the post-port first run for
  projection parity
  **Location**: Assumptions
  Assumptions name the sharpest risk — a whitespace difference reclassifies
  every synced item as `remotely-modified` — but the golden fixture checks
  the projection function in isolation, not the property the risk is about.
  A fixture corpus whose `remote_hash` baselines came from the bash path
  should still classify as `synced` after the port.

- 🔵 **Testability**: The gated contract suite's content is unspecified and
  nothing binds the fake to the real implementations
  **Location**: Acceptance Criteria (test partitioning); Assumptions
  The suite is defined only negatively. Every unit-level criterion is
  verified against a fake whose fidelity is never checked — deferred in
  review 1, re-raised because Phase C now routes conflict resolution
  through the same boundary.

- 🔵 **Completeness**: Context explains provenance but not why the migration
  is worth doing
  **Location**: Context
  Almost the whole section is split-and-revision history. Sibling 0170
  carries the beneficiary and outcome ("plugin maintainers inherit a typed,
  bash-3.2-independent, characterization-tested CLI"); this item does not.

- 🔵 **Scope**: The Phase D bundling decision's revisit trigger is dead
  **Location**: Drafting Notes; Dependencies
  "Revisit if Phase D stalls waiting on 0170" can never fire now 0170 is
  done, so a decision explicitly held open is frozen by default rather than
  by judgement.

- 🔵 **Clarity**: Drafting Notes cite acceptance criteria by ordinal, and
  the ordinals are wrong
  **Location**: Drafting Notes
  "AC1's resumability contract", "AC4's test-partitioning" and "AC6's
  removal/floor-decrement" point at the classification, dirty-pull and
  `--resolve` criteria respectively in the current sixteen-item list.

- 🔵 **Clarity**: The command synopsis omits `--resolve`
  **Location**: Requirements (sync command)
  `accelerator work sync [--push-only|--pull-only] [--preview]` reads as
  the authoritative flag list, yet `--resolve <id>=<remote|local>` is part
  of the delivered surface.

- 🔵 **Clarity**: The first Drafting Note asserts a dependency direction
  later notes reverse
  **Location**: Drafting Notes
  "this item precedes both rather than depending on either" is stated in the
  present tense and superseded two notes later, with no marker.

### Strengths

- ✅ The classification vocabulary is enumerated in full in both
  Requirements and Acceptance Criteria, with the two caller-handled states
  called out, and the decision table is given as an explicit signature
  (mode × state × local-dirty → six outcomes) — no reader has to guess the
  shape of the logic.
- ✅ Acceptance Criteria are sixteen Given/When/Then statements each
  anchored to a named external oracle — a specific bash script, a golden
  fixture, a cargo-pup rule — rather than restating Requirements.
- ✅ The resumability criterion gives a genuinely two-sided procedure for
  the engine's highest-risk property: write-order assertions via a fake
  store plus the ported crash-injection seam with a same-terminal-state
  re-run assertion.
- ✅ The `tracker`-must-not-depend-on-`work` invariant is made mechanically
  falsifiable — a named `Cargo.toml` absence plus a cargo-pup whole-crate
  `RestrictImports` rule that fails `cli:check`, with the permitted set
  spelled out.
- ✅ Adversarial cases are enumerated rather than hand-waved:
  first-sync-on-dirty surfacing as `conflict`, the unrecognised-token safe
  default, the unparseable baseline degrading to empty, and the
  stdin-closed non-interactivity check.
- ✅ Non-goals are stated positively with reasons attached — the two
  non-removable scripts are named, their live consumers listed, and the
  duplication's safety mechanism recorded.
- ✅ Drafting Notes carry a dated decision trail including rejected
  alternatives (port in `work`; a full `tracker` + `tracker-adapters`
  pair) and a frank record of which earlier claims were wrong and why.
- ✅ Blocking is recorded at slice granularity, and 0171's unblocking
  milestone is pinned at the port signature rather than this item's full
  acceptance gate.
- ✅ Requirements carefully distinguish porting work from wiring work —
  `work-adapters/src/project_remote.rs` is already ported and only needs
  wiring — which is a rare and genuinely useful form of completeness.

### Recommended Changes

1. **Separate the transient parity oracle from the retained gate**
   (addresses: "Parity oracles … are the same bash suites the removal
   criterion deletes") — state the ordering explicitly: the bash suites
   must pass against the Rust implementation before Phase E removes them,
   and name what survives as the permanent regression gate (e.g. the
   classify/decide fixture tables lifted into Rust test data). Pick one of
   "ported" or "repointed" and use it consistently.

2. **Give the SKILL half a requirement, a criterion, a filename and a
   report contract** (addresses: "The SKILL's prompting half is a named
   Phase C deliverable with no requirement…") — name the sync SKILL by
   path, describe what it must do, specify the conflict report's shape
   (machine-parseable line per unresolved item carrying id and state, plus
   a distinct exit code), and add a criterion covering the
   report → prompt → `--resolve` round trip.

3. **Clear the stale 0170 blocker** (addresses: "`blocked_by:
   work-item:0170` and the Dependencies narrative are stale") — drop it
   from frontmatter, record 0170 as discharged (done 2026-08-07) in the
   same form used for 0166 and 0187, and drop the dead "revisit if Phase D
   stalls" trigger.

4. **Record the reverse coupling on 0171 and narrow what this item claims
   to verify** (addresses: "The reverse coupling on 0171 … is uncaptured";
   "Composition-root provider selection … carry no criteria") — add a
   Dependencies bullet stating that composition-root wiring, the live
   contract suite and per-provider projection fidelity are satisfiable only
   once 0171 lands, and restate the affected criteria as "the seam exists
   and is exercised against a fake".

5. **State the intra-item phase ordering** (addresses: "Phase E cannot
   start before B, C and D") — correct Dependencies to "Phases A–C are
   independently startable; E depends on B, C and D", and assign the
   `push-decide` characterization test to a phase ahead of D.

6. **Audit and repoint the SKILL callers of the nine removed scripts**
   (addresses: "Skill-level consumers of the nine removed scripts are never
   audited") — enumerate them in Requirements and add a criterion requiring
   they be repointed at `accelerator work …` in the same change, mirroring
   the audit already done for `work-item-sync-label.sh`.

7. **Name the label anti-drift fixture and assign the label port to a
   phase** (addresses: "The anti-drift gate for the ported
   `work-item-sync-label.sh` cites golden fixtures that do not cover
   labelling") — say whether `test-fixtures/work-item-sync-label.golden`
   must be created, and drop `work-item-project-remote.golden` from the
   dual-implementation clause since only its Rust side survives.

8. **Fill in the `--push` outcome tables** (addresses: "`decline` and
   `confirmed-local-fallback` imply an interaction model never stated";
   "The `update --push` criterion asks the implementer to define the
   outcome") — state each outcome's trigger, state whether
   `create`/`update --push` inherit sync's non-interactivity, and write the
   two local-file outcomes for `E_DISPATCH_RETRYABLE` and
   `E_DISPATCH_TERMINAL` into the item rather than deferring them. Replace
   "never silently duplicates" with a concrete re-run scenario.

9. **Broaden the `--preview` no-mutation assertion and add a `finalise`
   criterion** (addresses: "The `--preview` no-mutation check verifies only
   the baseline document"; "The `finalise` run-start-epoch rule … no
   criterion") — assert three observables for preview (baseline
   byte-identical, local files unchanged, zero remote write calls on the
   fake) and add a criterion that a failed run leaves the global timestamp
   unadvanced while a clean run persists the run-start epoch.

10. **Re-take the size decision** (addresses: "The item has grown well past
    its post-split size") — split along the seams already drawn (sync
    engine A–C, `--push` wiring D, retirement E) or promote 0194 to an epic
    with the phases as children.

11. **Fix the dangling Open Questions pointer and gloss the remaining
    undefined terms** (addresses: "Technical Notes defers to an Open
    Questions section that declares nothing outstanding"; "Several terms
    are used as if already defined") — replace the pointer with the settled
    no-shim answer, and gloss "presence-only", "mtime pre-filter", "ADF",
    "renderable states", "the full contract" and "the configured pattern"
    in the style already used for `work.integration`.

12. **Split the no-provider-types criterion into its two checks**
    (addresses: "The no-provider-types criterion conflates a
    dependency-graph check with a signature check") — one on the manifest
    (mechanisable by the same cargo-pup rule), one on public signatures.

## Per-Lens Results

### Clarity

**Summary**: This is an unusually precise work item: the classification
vocabulary, decision-table inputs and outputs, baseline storage contract,
and crate-boundary invariants are all enumerated explicitly, and a
documented 2026-08-10 correction pass removed several earlier inaccuracies.
The main clarity weaknesses are cross-section contradictions left behind by
that revision — Technical Notes still defers a resolved question to an Open
Questions section that now says "none outstanding", and the parity
Acceptance Criterion asks bash test suites to be "repointed" while
Requirements order the same suites removed. Secondary issues are a heavily
overloaded use of "port" (hexagonal port vs. bash-to-Rust port), an
implied-but-unstated interaction model for `create --push`, and a cluster
of terms used as if already defined (`ADF`, "presence-only", "mtime
pre-filter", "confirmed-local-fallback").

**Strengths**:
- The classification vocabulary is enumerated in full in both Requirements
  and Acceptance Criteria, with the two caller-handled states called out.
- The decision table is stated as an explicit signature, so the shape of
  the logic has exactly one reading.
- `work.integration` and `E_DISPATCH_RETRYABLE`/`E_DISPATCH_TERMINAL` are
  glossed inline at their use sites.
- Acceptance Criteria name the actor and trigger rather than using passive
  constructions.
- Drafting Notes record which earlier claims were wrong and why, and
  explain the deliberate deviation from the source research doc's Open
  Question 2.
- Non-goals are stated positively with the reason attached.

**Findings**:
- 🟡 Major (high confidence) — Technical Notes defers to an Open Questions
  section that declares nothing outstanding. Location: Technical Notes /
  Open Questions.
- 🟡 Major (high confidence) — Bash test suites are simultaneously
  described as removed, ported, and repointed. Location: Acceptance
  Criteria (sync parity suite) / Requirements.
- 🟡 Major (medium confidence) — "decline" and "confirmed-local-fallback"
  imply an interaction model that is never stated for `create --push`.
  Location: Requirements (`--push` wiring) / Acceptance Criteria.
- 🔵 Minor (medium confidence) — "port" is used for both the hexagonal port
  and the bash-to-Rust translation. Location: Requirements / Technical
  Notes / Drafting Notes.
- 🔵 Minor (medium confidence) — The no-provider-types check conflates two
  different verifications. Location: Acceptance Criteria (tracker public
  API).
- 🔵 Minor (medium confidence) — "the four bridge scripts" is never
  enumerated and no source script is named for `show`. Location:
  Requirements (tracker crate).
- 🔵 Minor (high confidence) — Several terms are used as if already
  defined. Location: Requirements / Assumptions / Acceptance Criteria.
- 🔵 Minor (medium confidence) — "the same golden fixtures" has no
  resolvable referent for `work-item-sync-label.sh`. Location: Acceptance
  Criteria (sync parity suite).
- 🔵 Suggestion (high confidence) — Acceptance Criteria references in
  Drafting Notes point at the wrong criteria. Location: Drafting Notes.
- 🔵 Suggestion (high confidence) — The stated command synopsis omits
  `--resolve`. Location: Requirements (sync command synopsis).
- 🔵 Suggestion (medium confidence) — First Drafting Note asserts a
  dependency direction that later notes reverse. Location: Drafting Notes.

### Completeness

**Summary**: 0194 is an exceptionally complete work item: every expected
section is present and densely populated, the sixteen acceptance criteria
are Given/When/Then statements anchored to named bash oracles and fixtures,
and the Drafting Notes carry a dated decision trail including rejected
alternatives. Frontmatter is intact and kind-appropriate for a story, and
the 2026-08-10 correction pass visibly tightened Requirements against the
real source scripts. The remaining gaps are coverage asymmetries rather
than missing sections: a Phase C deliverable (the sync SKILL's prompting
half) that no Requirement or criterion names, an anti-drift gate for the
label port that cites a fixture the item's own inventory doesn't list, and
a `blocked_by` entry plus Dependencies narrative that have gone stale now
that 0170 is done.

**Strengths**:
- Every expected section is present and substantively populated — no
  placeholder or token-content sections.
- Acceptance Criteria are anchored to named external oracles rather than
  restating Requirements.
- Frontmatter is complete and kind-appropriate for a story.
- Open Questions is explicitly closed out and says where the answers live.
- Requirements distinguish porting work from wiring work, and Drafting
  Notes record which earlier claims were wrong.
- Technical Notes enumerate the source-bash inventory, a five-slice
  phasing breakdown, and an explicit non-goal.

**Findings**:
- 🟡 Major (high confidence) — SKILL-side prompting and re-invocation is a
  named phase deliverable with no Requirement, no acceptance criterion, and
  no named file. Location: Requirements / Acceptance Criteria.
- 🟡 Major (medium confidence) — The anti-drift gate for the ported
  `work-item-sync-label.sh` cites a shared golden fixture the item's own
  inventory does not list. Location: Acceptance Criteria.
- 🟡 Major (high confidence) — `blocked_by: work-item:0170` and the
  Dependencies narrative are stale — 0170 is done. Location: Frontmatter:
  blocked_by / Dependencies.
- 🔵 Minor (high confidence) — Technical Notes points the reader at an Open
  Question that no longer exists. Location: Technical Notes.
- 🔵 Minor (medium confidence) — The parity-suite criterion enumerates
  fewer suites than Requirements say must be ported, and no criterion
  covers the bulk fetch path. Location: Acceptance Criteria.
- 🔵 Minor (medium confidence) — The `finalise` half of the resumability
  contract has no acceptance criterion outside `--preview`. Location:
  Acceptance Criteria.
- 🔵 Suggestion (medium confidence) — Context explains the item's
  provenance but not why the sync engine is being migrated. Location:
  Context.

### Dependency

**Summary**: The work item is unusually disciplined about dependency
granularity — it names which single phase (D) is blocked by 0170, and pins
0171's unblocking milestone at the port signature rather than the full
acceptance gate — and its frontmatter edges are bidirectionally consistent
with 0170 and 0171. However, the Dependencies section is now stale (0170
completed on 2026-08-07, three days before this item's last edit, yet it is
still recorded as an active blocker in both prose and frontmatter), and
three couplings the body implies are absent from it: a reverse dependency
on 0171 for the composition-root provider wiring and the live-remote
contract suite, the intra-item ordering that makes Phase E depend on Phase
D, and the skill-level consumers of the nine scripts Phase E deletes. The
external tracker services themselves (Jira, Linear) and the credentials
their contract suite needs are named throughout Requirements and
Assumptions but appear nowhere in Dependencies.

**Strengths**:
- Dependencies records blocking at slice granularity rather than item
  granularity.
- The Blocks entry for 0171 names the actual blocking milestone (the port
  signature compiling at end of Phase A).
- Discharged blockers (0166, 0187) are recorded as resolved with dates
  rather than silently deleted.
- Frontmatter edges are bidirectionally consistent with 0170 and 0171.
- The two non-removable scripts have their three external consumers named
  explicitly.
- Assumptions captures the cross-story risk that moving projection behind
  the port could change persisted `remote_hash` values.

**Findings**:
- 🟡 Major (high confidence) — Blocked-by 0170 is stale; 0170 completed
  before this item's last edit. Location: Dependencies / Frontmatter:
  blocked_by.
- 🟡 Major (high confidence) — Reverse coupling on 0171 for provider wiring
  and the live-remote contract suite is uncaptured. Location: Requirements
  / Acceptance Criteria.
- 🟡 Major (high confidence) — Phase E cannot start before Phase D; the
  claim that E is unblocked is wrong. Location: Technical Notes: Phasing /
  Dependencies.
- 🟡 Major (medium confidence) — Skill-level consumers of the nine removed
  scripts are not identified as needing repointing. Location: Requirements
  (script removal) / Acceptance Criteria.
- 🔵 Minor (medium confidence) — Jira and Linear as external systems, and
  the credentials their contract suite needs, are absent from Dependencies.
  Location: Dependencies / Assumptions.
- 🔵 Minor (medium confidence) — Deferred removal of `sync-label` and
  `normalise` has no captured successor. Location: Requirements /
  Drafting Notes.
- 🔵 Minor (medium confidence) — The push-decide characterization test is
  unassigned to a phase. Location: Technical Notes: Phasing /
  Requirements.
- 🔵 Minor (medium confidence) — Dual ownership of the `E_DISPATCH_*`
  taxonomy across the Rust port and the surviving bash bridges is
  uncaptured. Location: Technical Notes.

### Scope

**Summary**: 0194 is a coherent capability — everything in it serves "work
items sync with the remote tracker from the Rust CLI" — and its boundaries
are unusually well drawn, with explicit non-goals, a weighed-and-rejected
alternatives log for the crate split, and a five-phase internal
decomposition that names each phase's blocker. The scope concern is size
rather than coherence: since the 2026-08-05 split from 0170 the item has
absorbed `--push` wiring from its sibling and, on 2026-08-10, three further
behaviours discovered in the source bash (`--preview`, interactive conflict
resolution, the projection seam), reaching 17 Requirements bullets and 16
Acceptance Criteria across three Rust crates, a new binary surface, the
bash suite, the build-system suite floor and the SKILL surface. Two smaller
boundary issues: the SKILL-side prompting half is declared in scope but
appears in no Acceptance Criterion, and the projection-behind-the-port
requirement cannot be fully realised or verified inside this item.

**Strengths**:
- Non-goals are stated explicitly with reason and consequence recorded.
- The five-phase structure makes the item's internal seams visible and
  identifies which phase is blocked.
- Provider-specific concerns are held outside the boundary and the
  invariant is enforced mechanically rather than by convention.
- Drafting Notes record the alternatives considered and rejected for the
  crate boundary.
- The split from 0170 and the reciprocal move of `--push` are documented
  consistently on both items.

**Findings**:
- 🟡 Major (medium confidence) — The item now carries 17 Requirements and
  16 Acceptance Criteria across five phases and four distinct deliverables,
  matching the shape the epic has already split twice. Location:
  Requirements / Technical Notes: Phasing.
- 🔵 Minor (medium confidence) — The SKILL's prompting half is declared in
  scope in Technical Notes but appears in no Acceptance Criterion.
  Location: Technical Notes: Phasing / Acceptance Criteria.
- 🔵 Minor (medium confidence) — The projection-behind-the-port requirement
  completes outside this item's boundary and its stated gate cannot
  exercise a real provider until 0171 lands. Location: Requirements /
  Assumptions.
- 🔵 Suggestion (medium confidence) — The Phase D bundling decision's
  revisit trigger can never fire now that 0170 is done. Location: Drafting
  Notes / Dependencies.

### Testability

**Summary**: 0194's Acceptance Criteria are unusually strong for their
genre: most anchor verification to named bash oracles and golden fixtures,
several name the exact mechanism (recording fake store, ported fault seam,
stdin-closed run, cargo-pup rule), and the adversarial cases
(first-sync-on-dirty, unrecognised resolve token, unparseable baseline) are
enumerated rather than hand-waved. The weaknesses cluster around three
areas: an internal contradiction where the parity oracles named in the
classification and parity criteria are the same bash suites the removal
criterion requires deleted; criteria whose expected outcome is deferred to
the implementer (the `update --push` local-file outcome) or verified by a
proxy too narrow to catch the failure mode (`--preview` checks only the
baseline document); and several Requirements — the `finalise`
run-start-epoch rule, the conflict report's output contract, the SKILL's
prompting half, composition-root provider selection — that carry no
criterion at all.

**Strengths**:
- The resumability criterion gives a genuinely two-sided verification of
  the highest-risk property: write-order assertions plus a ported
  crash-injection seam with a same-terminal-state re-run assertion.
- The `tracker`-not-depending-on-`work` criterion is mechanically
  falsifiable via `Cargo.toml` plus a named cargo-pup rule that fails
  `cli:check`.
- The `--resolve` criterion enumerates the full token truth table including
  the safe-default branch and the normalisation rules.
- The non-interactivity criterion converts an abstract property into an
  operational test (run with stdin closed).
- The characterization criterion bounds its obligation concretely rather
  than asking for "adequate coverage".
- The classification criterion names a specific adversarial case rather
  than only the happy path.
- The default-run/tagged-suite partition is a binary, mechanically
  checkable property.

**Findings**:
- 🟡 Major (high confidence) — Parity oracles named in the classification
  and parity criteria are the same bash suites the removal criterion
  deletes. Location: Acceptance Criteria / Requirements.
- 🟡 Major (high confidence) — The `update --push` criterion asks the
  implementer to define the expected outcome it is meant to verify.
  Location: Acceptance Criteria / Requirements.
- 🟡 Major (medium confidence) — The `--preview` no-mutation check verifies
  only the baseline document, not local files or remote calls. Location:
  Acceptance Criteria.
- 🟡 Major (medium confidence) — The conflict report's output contract is
  unspecified, and the SKILL half of the two-invocation flow has no
  criterion. Location: Acceptance Criteria / Requirements.
- 🟡 Major (medium confidence) — The `finalise` global-timestamp rule is
  required but no criterion verifies it. Location: Acceptance Criteria /
  Requirements.
- 🔵 Minor (medium confidence) — The dual-implementation gate for
  `work-item-sync-label.sh` names no golden fixture. Location: Acceptance
  Criteria / Technical Notes.
- 🔵 Minor (medium confidence) — `decline` and `confirmed-local-fallback`
  have no stated trigger, and "never silently duplicates" has no
  procedure. Location: Acceptance Criteria.
- 🔵 Minor (medium confidence) — The no-provider-types criterion conflates
  a dependency check with a signature check and, unlike its sibling, is not
  mechanically enforced. Location: Acceptance Criteria.
- 🔵 Minor (low confidence) — Composition-root provider selection and the
  classifier's never-fetch/bulk-orchestration rules carry no criteria.
  Location: Requirements.
- 🔵 Suggestion (medium confidence) — The projection-parity risk is named
  but no criterion checks the post-port first run. Location: Assumptions.
- 🔵 Suggestion (low confidence) — The gated contract/integration suite's
  content is unspecified, and nothing binds the fake to the real
  implementations. Location: Acceptance Criteria / Assumptions.


## Re-Review (Pass 2) — 2026-08-10T16:04:28+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability/Clarity/Completeness**: Parity oracles are the same bash
  suites the removal criterion deletes — **Partially resolved**. The
  contradiction is gone: the bash suites are now an explicit transient
  pre-removal oracle and the fixture tables lift into Rust. But
  "each Rust replacement passes against the bash suite" has no defined
  procedure now that no harness is repointed — testability cannot tell
  whether that means a differential test shelling out to the script, an
  early table lift run against both, or nothing compared at all.
- 🟡 **Completeness/Testability/Scope**: The SKILL's prompting half has no
  requirement, criterion, filename or report contract — **Partially
  resolved**. `sync-work-items/SKILL.md` is now named with a requirement,
  a round-trip criterion and a machine-parseable report contract. Two gaps
  remain: `create`/`update --push` defer their judgment to "the SKILL"
  with no SKILL named and no criterion, and the sync round-trip criterion
  is the one criterion in the item with no verification procedure.
- 🟡 **Dependency/Completeness/Scope**: `blocked_by: work-item:0170` is
  stale — **Resolved**. Frontmatter cleared and Dependencies rewritten with
  the 2026-08-07 completion date. Residue: Summary, Context, Requirements
  and Assumptions still speak of 0170's commands in the future tense.
- 🟡 **Dependency/Scope**: The reverse coupling on 0171 is uncaptured —
  **Partially resolved, and it uncovered worse**. The Dependencies bullet
  now distinguishes seams from clients, but the contract-suite criterion
  still ends "and against each real client once 0171 lands", which three
  lenses independently flag as contradicting "nothing in this item's
  acceptance gate waits on 0171". The deeper problem is new and critical —
  see below.
- 🟡 **Dependency**: Phase E cannot start before B, C and D — **Resolved**.
  Stated explicitly in Dependencies and the phasing note. One wrinkle: the
  preceding bullet's "All five phases are now startable" contradicts it.
- 🟡 **Dependency**: SKILL callers of the nine removed scripts are never
  audited — **Mostly resolved**. All four callers are now enumerated in
  Requirements and the removal criterion. But `/list-work-items`' target is
  unnamed: no `accelerator work` verb exposes classification outside
  `sync`, and the script it calls is the one being retained.
- 🟡 **Completeness/Clarity/Testability**: The label anti-drift gate cites
  fixtures that do not cover labelling — **Resolved for the fixture,
  displaced elsewhere**. `work-item-sync-label.golden` is now required with
  its own criterion. Three lenses now flag that the `label` stage itself
  has no crate home, no behavioural requirement, and sits in Phase E while
  Phases B–C need it working.
- 🟡 **Clarity/Testability**: `decline` and `confirmed-local-fallback`
  imply an unstated interaction model — **Resolved as stated, replaced by
  a mechanism gap**. Both terms are gone and non-interactivity now covers
  every remote-touching command. But the replacement criterion ("re-run for
  the same work item issues no second `create`") has no constructible
  precondition: the failed run writes a file *without* `external_id`, and
  no pending-push marker or idempotency key is specified.
- 🟡 **Testability**: The `update --push` criterion defers its own outcome
  — **Resolved**. Both failure paths now have concrete, asserted local-file
  and baseline outcomes.
- 🟡 **Testability**: `--preview` no-mutation verifies only the baseline —
  **Resolved**. Three independent observables now. The added plan-fidelity
  criterion, however, compares against a preview report that no requirement
  defines.
- 🟡 **Testability/Completeness**: The `finalise` rule has no criterion —
  **Resolved**. Both halves now asserted against an injected clock. Minor
  new risk: the strict inequality spans an injected clock and real
  filesystem mtimes, and the failure-injection point is unnamed.
- 🟡 **Clarity/Completeness**: Technical Notes defers to an empty Open
  Questions section — **Resolved**.
- 🟡 **Scope**: The item has grown past its post-split size — **Not
  resolved, by decision**. Kept whole with the rationale recorded. Scope
  re-raises it as major and notes the item grew from 17/16 to 22/25 in the
  course of fixing the other findings, and that the stated rationale
  ("splitting D out would put the `--push` retirement in a different story
  from the sync retirement") is contradicted by 0170's own precedent of
  partitioned retirement.

### New Issues Introduced

- 🔴 **Dependency**: **Phase E deletes the only working remote path while
  no real `RemoteTracker` implementation exists.** Phase E removes the
  create/update/fetch bridge scripts and repoints the SKILLs at
  `accelerator work …`, but the real clients are 0171's deliverable — and
  0171 is blocked by this story. Between this item landing and 0171
  landing, `sync` and `create/update --push` resolve only fakes, so the
  user-facing flows are non-functional. This breaks epic 0136's
  stay-functional-at-every-step rule, the same rule this item invokes to
  justify not splitting Phase D out. Not introduced by the edits — the
  edits made the sequencing explicit enough to see it.
- 🟡 **Clarity**: The decision-table requirement and its criterion still
  use `remote-ahead` and `local-ahead` — names the item's own Drafting
  Notes say "were never the vocabulary". Pre-existing; surfaced now that
  the surrounding text uses the corrected names.
- 🟡 **Dependency**: The obligations this item delegates to 0171 (contract
  suite against real clients, per-provider projection fidelity) appear
  nowhere in 0171, which is still `status: draft`.
- 🟡 **Completeness/Testability**: The `E_DISPATCH_*` anti-drift fixture is
  promised in Technical Notes but has no requirement and no criterion —
  unlike its exact analogue, the label golden.
- 🟡 **Clarity/Testability**: "bulk mode" versus "per-item mode" is
  asserted by a new criterion but no flag, threshold or config key selects
  between them, so the precondition cannot be constructed.
- 🟡 **Clarity**: The test-partitioning requirement says the default
  invocation runs the parity suite "only"; the contract-suite criterion
  says the same suite runs against the fake in the default invocation.
- 🔵 **Clarity**: "dirty" drives a decision-table input and a safety
  guarantee but is never defined — VCS working-copy dirtiness or
  divergence from the baseline `local_hash`.
- 🔵 **Scope/Dependency**: 0171 is blocked by a fragment of this item (end
  of Phase A), a coupling the dependency graph cannot express.

### Assessment

The edits did what they were asked to do: eleven of the thirteen major
findings are resolved or substantially resolved, and the two structural
contradictions that carried the REVISE verdict — the parity suites being
both oracle and removal target, and the SKILL half being invisible — are
gone in substance. The verdict stays REVISE for a different reason: making
the sequencing explicit exposed a **critical** gap that the vaguer earlier
text concealed. Phase E retires the bash bridges before any real client
exists to replace them, and the story that supplies those clients is
blocked by this one. That is a genuine ordering defect in the migration,
not a documentation problem, and it needs a decision — gate Phase E's
bridge removal on 0171, or ship an interim adapter that shells out to the
existing bridges — before this item is implementable.

The remaining majors are a consistent pattern worth naming: every place the
specification crosses a process edge (preview output, the create-retry
marker, the bulk/per-item selector, the `E_DISPATCH_*` fixture) now has a
criterion asserting behaviour that no requirement defines. The fixes added
criteria faster than they added contracts. A further pass should close
those contracts rather than add more criteria — the item is at 22
requirements and 25 criteria, and scope's size finding is now stronger than
when it was first raised.


## Re-Review (Pass 3) — 2026-08-10T16:34:11+00:00

**Verdict:** REVISE

Run after the cutover — script removal, SKILL repointing, the sync SKILL's
conversational conflict flow, and the suite-floor decrement — moved out of
0194 and into 0171.

### Previously Identified Issues

- 🔴 **Dependency**: Phase E deletes the only working remote path while no
  real `RemoteTracker` implementation exists — **Resolved**. The cutover is
  0171's. 0194 now ships the binary beside the live bash path, Dependencies
  states the handover as four named obligations, a criterion asserts no
  script is removed and no SKILL repointed, and 0171 carries matching
  requirements and criteria. The dependency lens confirms the handover is
  accepted on both sides rather than asserted on one.
- 🟡 **Testability**: "Passes against the bash suite" had no defined
  procedure — **Superseded rather than resolved**. The pre-removal-oracle
  framing is gone, but its replacement has a new hole: "both held to one
  oracle" is realised as two independent copies of the fixture data, each
  checked against its own implementation, with nothing asserting the copies
  still agree.
- 🟡 **Completeness/Clarity**: `create`/`update --push` defer to an unnamed
  "the SKILL" — **Still present**. The bare definite reference remains, and
  its referent shifts between the sync skill and the create skill.
- 🟡 **Testability**: The SKILL round-trip criterion had no verification
  procedure — **Resolved**. A test harness now drives the two-invocation
  loop inside this story; the conversational half went to 0171 with a
  criterion there.
- 🟡 **Dependency**: Obligations delegated to 0171 appear nowhere in 0171 —
  **Resolved**. 0171 now carries all three plus the cutover.
- 🟡 **Completeness/Testability**: The `E_DISPATCH_*` fixture is promised in
  Technical Notes with no requirement or criterion — **Still present**, and
  now worse: 0171 has an acceptance criterion to delete a fixture 0194 is
  not obliged to create.
- 🟡 **Clarity/Testability**: "bulk mode" has no selector — **Still
  present**.
- 🟡 **Clarity**: The test-partitioning requirement and the contract-suite
  criterion contradicted each other — **Resolved**. The criterion now
  scopes this story to the fake and hands the real-client run to 0171.
- 🔵 **Clarity**: "dirty" is never defined — **Still present**, and the
  dependency lens adds that its source is unnamed: 0170 ported
  `work-item-file-dirty.sh` as a private helper inside `accelerator-work`,
  while classify and decide are sited in the `work` library crate.
- 🟡 **Clarity**: Retired state names `remote-ahead` / `local-ahead` in the
  decision-table requirement — **Still present**.
- 🟡 **Scope**: The item exceeds a story-sized unit — **Partially
  addressed**. Moving the cutover removed the largest slice, but the item
  still carries 23 requirements and 26 criteria across four phases, and
  scope notes the risk was relocated rather than reduced: 0171 is now the
  oversized item.

### New Issues Introduced

- 🔴 **Clarity/Completeness**: **Three passages still say bash scripts are
  removed in this story**, contradicting the no-removal requirement and its
  criterion — Technical Notes "Source bash" (`project-remote` "needs the
  wiring and the removal"), Technical Notes "Parity fixtures"
  (`project-remote`'s bash side "is deleted here"), and the Requirements
  coverage-gap bullet ("need porting and removal only"). An implementer
  following Technical Notes would delete a script the live bash sync path
  still calls. Introduced by the cutover edit failing to sweep every
  removal phrase.
- 🟡 **Clarity**: The Context section still promises the binary replaces
  the bash path and that users get a new conflict experience — the opposite
  of what the final criterion now asserts.
- 🟡 **Clarity/Completeness**: Phase labelling is inconsistent — Drafting
  Notes still say "five explicit phases", site the label golden "in Phase
  E", and state "E depends on B–D", while Technical Notes now lists four
  slices and Dependencies says there is no removal phase.
- 🟡 **Clarity/Dependency**: `fetch_all()` is the only port operation with
  no signature, and the port signature is what unblocks 0171 at the end of
  Phase A.
- 🟡 **Testability/Clarity/Dependency**: The `create --push` retry
  criterion still has no constructible precondition — the failed run writes
  no `external_id` and no marker is specified, so nothing records that a
  remote issue may exist. The dependency lens adds that fixing it may need
  port surface, which would break the signature promised to 0171.
- 🟡 **Clarity**: The decision outcome named `prompt` contradicts the
  strict non-interactivity rule, and `skip-conflict` is never mapped to any
  input cell.
- 🟡 **Testability**: The classification-stability corpus has no specified
  size, provider span or state coverage — one linear item would satisfy it
  while leaving the jira recipe unguarded.
- 🟡 **Dependency**: The residual duplication is handed to 0174 on a
  precondition — porting `/list-work-items` — that no work item owns, and
  0174 carries no matching requirement.
- 🟡 **Clarity/Completeness**: Label coverage is five states in Requirements
  and seven in the fixture criterion, with no stated output for the two
  caller-handled states.
- 🔵 **Completeness**: The `unsynced` state — the most common sync path —
  has no criterion covering first-time push and `external_id` write-back;
  and no criterion drives a successful end-to-end run at all.
- 🔵 **Clarity**: "the binary ships alongside it, unwired" conflicts with
  the three internal wirings the Requirements mandate.
- 🔵 **Testability**: Non-interactivity is verified for `sync` and `create
  --push` but not `update --push`.

### Assessment

The critical is resolved, and resolved well — the handover is documented
symmetrically, the interval is explicitly a no-behaviour-change interval,
and the ordering constraint that the baseline corpus must be captured
before its generator disappears was caught and written down. Every lens
recognised the fix.

It was replaced by a smaller but real one of the same kind: the edit
changed the decision without sweeping the prose that encoded the old one,
so three passages still instruct an implementer to delete a script the
story now forbids removing. That is mechanical to fix and carries no design
question.

Underneath both sits the pattern this item keeps reproducing. Each pass
resolves its predecessor's findings and introduces a comparable number of
new ones, because the document is large enough that a decision taken in one
section leaves residue in three others — 23 requirements, 26 criteria, four
phases, and a decision log that now contradicts the phasing it describes.
Three lenses independently reached the same recommendation this round:
extract Phase A, the `tracker` port, as its own item. It is a signature with
no logic, it is the only thing 0171 actually waits on, and the current
arrangement blocks a downstream story on a fragment the dependency graph
cannot express. That would also stop the port churning under 0171 while the
rest of this item is still being edited.

---
*Review generated by /accelerator:review-work-item*
