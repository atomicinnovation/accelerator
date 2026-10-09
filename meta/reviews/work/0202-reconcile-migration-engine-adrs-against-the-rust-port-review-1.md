---
type: "work-item-review"
id: "0202-reconcile-migration-engine-adrs-against-the-rust-port-review-1"
title: "Work Item Review: Reconcile Migration-Engine ADRs Against the Rust Port"
date: "2026-10-08T22:08:33+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0202"
work_item_id: "0202"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-08T23:25:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Reconcile Migration-Engine ADRs Against the Rust Port

**Verdict:** REVISE

The item has one coherent purpose and a well-reasoned disposition taxonomy
(Supersede / Deprecate / Retain) bounded by ADR-0031, and its Drafting Notes
pre-empt the obvious alternatives. It is not yet implementable as written:
the Context attributes to ADR-0037 content the ADR does not contain, the
ADR-0038 disposition is governed by two contradictory rules while ADR-0038's
own text names a deleted bash path, and several requirements (the ADR-0023
successor's content, the cross-reference sweep) have no acceptance criterion
or no defined boundary.

### Cross-Cutting Themes

- **ADR-0037 is misdescribed** (flagged by: clarity, completeness,
  testability) — ADR-0037 defines an abstract contract (trigger predicate,
  three display elements, resumability artefact, accept/edit/skip, §5
  supplement clause) and explicitly declines to commit to a callback ABI,
  process model, or persistence format. The `# INTERACTIVE: yes` header,
  `migration_*` callbacks, FIFO protocol, write-ahead log, sticky skip, and
  callback determinism come from the bash implementation, not the ADR. Its
  only concrete stale reference is `skills/config/migrate/scripts/run-migrations.sh`.
- **ADR-0038 disposition is contradictory and likely wrong** (flagged by:
  clarity, completeness, testability, scope) — Requirements/Assumptions say
  "turns solely on whether its text names changed machinery"; the last
  Drafting Note says "retain unless a second interactive migration exists".
  ADR-0038 line 122 ties the migration ID to "its file in
  `skills/config/migrate/migrations/`" and "the migration script", and
  line 124 specifies line-delimited JSON, so under AC1 it cannot remain
  `accepted`.
- **Cross-reference sweep is unbounded and unverified** (flagged by:
  testability, scope, clarity, dependency) — "any skill or doc prose" and
  "work items" have no stated boundary (done/historical items?) and only the
  SKILL.md Cross-references section has an acceptance criterion.
- **Staleness test and AC1 scope are inconsistent** (flagged by: clarity,
  testability) — Requirements key on "the port deleted or changed"; AC1 keys
  on "no longer exists in the codebase", and "no ADR left in `accepted`"
  could mean the three ADRs or the whole corpus.

### Findings

#### Major

- 🟡 **Clarity / Completeness / Testability**: ADR-0037's contents are
  described inaccurately, and the successor's required content has no stated
  source
  **Location**: Context, Requirements, Technical Notes, AC3
  The Context lists the opt-in header, four callbacks, FIFO protocol,
  write-ahead-log, sticky skip, and callback determinism as ADR-0037
  content; none appear in its text. AC3 still requires the successor to
  cover them, so the successor is codifying implementation decisions for
  the first time, which the item should say explicitly, naming the source
  documents.
- 🟡 **Testability / Clarity / Completeness / Scope**: Two contradictory
  decision rules for ADR-0038, and its stale migration-file path is missed
  **Location**: Requirements, Assumptions, Drafting Notes
  The rules diverge whenever ADR-0038 names changed machinery while 0007
  remains the only interactive migration, which ADR-0038 line 122 already
  does. Following the Drafting Notes rule fails AC1.
- 🟡 **Clarity**: Two-band model is assigned to both the generic successor
  ADR and ADR-0038
  **Location**: Acceptance Criteria (AC3)
  ADR-0037 says band design is a migration concern; AC3 requires the generic
  interactive-contract successor to describe the two-band model, which would
  elevate a migration-0007 parameter into the framework contract.
- 🟡 **Testability**: No criterion verifies the ADR-0023 successor's content
  **Location**: Acceptance Criteria
  The ledger path, compiled-in registration, atomic writes, idempotency
  self-checks, and no-dry-run requirement are untested; a successor missing
  any of them passes every AC.
- 🟡 **Testability / Scope / Clarity / Dependency**: Cross-reference update
  is unbounded and has a criterion only for SKILL.md
  **Location**: Requirements (last bullet), AC6
  No search scope, no rule for done/historical work items, plans, or
  research, and AC6's "or explicitly explains why" escape clause admits
  nearly any stale link.
- 🟡 **Testability**: AC3's "reader can correctly describe" has no reference
  answer
  **Location**: Acceptance Criteria (AC3)
  Pass/fail depends on who reads; replace with an enumerated content check
  against the named Rust sources.

#### Minor

- 🔵 **Clarity / Testability**: Staleness test differs between Requirements
  and AC1, and AC1's corpus scope is ambiguous
  **Location**: Requirements, AC1
  Pre-port drift (ADR-0023's ledger path) passes "the port changed" but fails
  "no longer exists"; scope AC1 to the three named ADRs or state it is
  corpus-wide.
- 🔵 **Clarity**: §5 recursive supplement clause conflated with callback
  determinism, and supersession-vs-supplement not justified
  **Location**: Context, Summary, Technical Notes
  §5 requires extensions be recorded in a new supplementary ADR; the item
  discharges it by supersession without saying why that suffices.
- 🔵 **Clarity / Scope**: Summary implies a supersede-only exercise and omits
  the new decisions and cross-reference sweep
  **Location**: Summary
- 🔵 **Dependency**: Ordering between the ADR-0023 and ADR-0037 successors is
  not stated
  **Location**: Requirements
  Drafting the ADR-0037 successor first risks it citing the superseded
  ADR-0023, which cannot be fixed later without another supersession.
- 🔵 **Dependency**: ADR-0047/0052/0053, named in 0172's plan, are not
  addressed
  **Location**: Context
- 🔵 **Dependency**: 0070, 0214, 0157 absent from Dependencies and
  `relates_to`
  **Location**: Dependencies, Frontmatter: relates_to
  This item resolves 0070's ledger-path flag; no edge records that.
- 🔵 **Testability**: AC5 "reaches `status: accepted`" is reviewer
  discretion and always satisfiable
  **Location**: AC5
- 🔵 **Completeness**: Open Questions is an empty heading
  **Location**: Open Questions

#### Suggestions

- 🔵 **Completeness**: No designated place to record each ADR's disposition
  **Location**: Requirements, AC1
  Add a Dispositions table (ADR, disposition, trigger text, successor id).
- 🔵 **Clarity**: Bare numeric references (0214, 0157, 0099, 0070) and
  undefined terms (sticky skip, callback determinism)
  **Location**: Context
  0099 is also missing from References.
- 🔵 **Dependency**: Downstream consumers of the ADR identifiers and any
  Blocks edge under 0136 are not listed
  **Location**: Dependencies

### Strengths

- ✅ Single coherent purpose; pre-port ledger drift brought in scope with a
  stated rationale.
- ✅ Closed disposition taxonomy with an observable test and named skill for
  each option.
- ✅ Drafting Notes record rejected alternatives (Amend, errata block,
  standalone timeout ADRs) and the reasoning, preventing relitigation.
- ✅ ADR-0031 constraint stated with precedents; other ADRs' bodies
  deliberately excluded from the sweep.
- ✅ Technical Notes give a one-to-one trait-method ↔ callback mapping.
- ✅ AC2 and AC4 are directly verifiable; AC1 converts the goal into an
  auditable outcome.
- ✅ Blocker 0172 named, done, with the Phase 10 cutover as stabilisation
  point.

### Recommended Changes

1. **Rewrite ADR-0037's Context entry to match its text** (addresses:
   ADR-0037 misdescribed, §5 conflation)
   State that ADR-0037's only concrete stale reference is
   `run-migrations.sh`, and that the successor additionally codifies
   implementation-level decisions (callbacks → trait methods, write-ahead
   log, sticky skip, determinism, the 30s timeout, `verify_applied`,
   `finalise`), naming the source for each (0172's plan, the bash-era
   SKILL.md, `cli/migrate/src/*.rs`). Add one sentence on why supersession
   discharges §5.
2. **Pre-judge ADR-0038 as superseded and delete the conflicting rule**
   (addresses: ADR-0038 contradictory rules, Summary understates scope)
   Cite lines 122 and 124 as triggers, extend AC2 to cover it, sequence it
   after the ADR-0037 successor, and remove the "second interactive
   migration" Drafting Note and Assumption.
3. **Place the two-band model** (addresses: two-band model assigned to both)
   Keep it in the ADR-0038 successor; have the ADR-0037 successor state only
   that band design is migration-owned, and drop it from AC3's list.
4. **Replace AC3 with an enumerated content check and add an ADR-0023
   successor AC** (addresses: AC3 no reference answer, ADR-0023 successor
   untested)
   Mirror AC4's style; include "no statement refers to a bash script, FIFO,
   or `# INTERACTIVE` header".
5. **Bound the cross-reference sweep and make it a search-result AC**
   (addresses: unbounded sweep, AC6 escape clause, downstream consumers)
   Scope to `skills/`, `docs-site/`, and non-done work items; state that
   done work items, plans, and research are historical and untouched;
   tighten AC6's escape clause to links labelled historical alongside a
   successor link.
6. **Unify the staleness test and scope AC1** (addresses: staleness test
   inconsistent)
   Use "names anything that no longer holds in the codebase, whatever the
   cause" and scope AC1 to the three ADRs (or state the corpus-wide audit,
   including ADR-0047/0052/0053).
7. **Add sequencing, a Dispositions table, and linkage edges** (addresses:
   ordering, disposition location, 0070/0214/0157 edges, empty Open
   Questions, bare numbers)
   Order: ADR-0023 → ADR-0037 → ADR-0038 successors. Add `relates_to` for
   0070; prefix bare numbers with `work-item:`; fill or remove Open
   Questions.
8. **Fold AC5 into a completion step or give it teeth** (addresses: AC5
   always satisfiable)

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Intent is clear (one recorded disposition per ADR, supersession
as default), but internal consistency fails: ADR-0037's content is
misattributed, ADR-0038 has two contradictory decision rules, and the
two-band model is assigned to both the generic contract and ADR-0038.

**Strengths**:
- Disposition options each have an explicit test and an executing skill.
- Drafting Notes remove ambiguity about dropped Amend and changed review
  criterion.
- Trait ↔ callback mapping makes "restate against the Rust-native shape"
  concrete.
- ADR-0023 claims are concrete and verified against its text.

**Findings**:
- major / high — Context — ADR-0037's contents are described inaccurately.
  None of the header, callbacks, FIFO, write-ahead-log, sticky skip, or
  determinism appear in ADR-0037; it declines to commit to a callback ABI.
  Separate ADR text from implementation additions.
- major / high — Drafting Notes — Two contradictory rules for ADR-0038's
  disposition. Choose one and state it consistently.
- major / medium — Acceptance Criteria — Two-band model is assigned to both
  the generic successor ADR and ADR-0038. Decide where it belongs.
- minor / medium — Summary — How the recursive supplement clause relates to
  the chosen mechanism is unclear; §5 is conflated with callback determinism
  and supersession-vs-supplement is unjustified.
- minor / high — Summary — Summary implies every outcome is a supersession.
- minor / medium — Requirements — Staleness test is defined in different
  terms ("port deleted or changed" vs "no longer exists"); AC1 scope
  ambiguous.
- minor / medium — Requirements — Cross-reference requirement mixes
  exclusions and inclusions; unclear whether done work items are rewritten.
- suggestion / medium — Context — Bare numeric references (0214, 0157, 0099,
  0070) and undefined terms; 0099 missing from References.

### Completeness

**Summary**: Structurally complete for a task with substantive sections, but
the Context's description of ADR-0037/0038 diverges from their text, leaving
the successor's newly-recorded decisions under-enumerated; Open Questions is
empty and there is no disposition record location.

**Strengths**:
- Frontmatter complete and correct.
- Context establishes motivation, quotes 0172's scoping-out, and explains
  ADR-0031 with precedents.
- Closed disposition taxonomy with decision rules and executing skills.
- Drafting Notes capture rejected alternatives.
- Blocker 0172 named and its stabilisation point given.

**Findings**:
- major / medium — Requirements — Successor-ADR requirements omit decisions
  ADR-0037 never actually recorded (write-ahead log, sticky skip, callback
  determinism) yet AC3 requires; name the source of each.
- minor / medium — Drafting Notes — ADR-0038 guidance omits the
  `skills/config/migrate/migrations/` path it names, and how ADR-0038 treats
  a superseded ADR-0037.
- minor / high — Open Questions — Empty heading.
- suggestion / medium — Requirements — No designated place to record each
  ADR's disposition; add a Dispositions section.

### Dependency

**Summary**: Primary blocker 0172 and the ADR-0031 constraint are captured.
Gaps: successor ordering, related items 0070/0214/0157 not linked, other
migration-engine ADRs (0047/0052/0053) unaddressed, downstream consumers
unlisted.

**Strengths**:
- 0172 named, done, with stabilisation rationale.
- ADR-0031 process constraint explicit with precedents.
- SKILL.md Cross-references consumer named; other ADR bodies excluded with
  rationale.

**Findings**:
- minor / medium — Requirements — Ordering between ADR-0023 and ADR-0037
  successors not captured; supersede 0023 first, then 0037, then settle 0038.
- minor / medium — Context — ADR-0047/0052/0053 named in 0172's plan are not
  addressed.
- minor / high — Dependencies — 0070, 0214, 0157 absent from Dependencies and
  `relates_to`; 0070's flag is resolved by this item.
- suggestion / low — Requirements — Downstream consumers of the ADR
  identifiers not enumerated.
- suggestion / low — Dependencies — No Blocks entry under parent 0136.

### Scope

**Summary**: Single coherent purpose, correct kind, carefully argued
boundaries. Main risk: ADR-0038 left open though its text suggests a third
supersession; the cross-reference sweep has a soft boundary.

**Strengths**:
- Every requirement serves the reconciliation; pre-port drift included with
  rationale.
- Errata-block convention rejected as disproportionate.
- New contract facets folded into the ADR-0037 successor.
- Other ADRs' bodies excluded consistently with ADR-0031.
- Single owner, named parent, blocker done.

**Findings**:
- minor / medium — Requirements / Assumptions / Drafting Notes — ADR-0038
  disposition left open though its `skills/config/migrate/migrations/`
  reference suggests a third supersession; reconcile the conflicting rules.
- minor / medium — Requirements — Cross-reference requirement has an
  open-ended reach; bound to living documents.
- suggestion / medium — Summary — Summary understates actual scope.

### Testability

**Summary**: Most ACs are mechanically checkable (AC1, AC2, AC4). Gaps: the
ADR-0023 successor content and the corpus sweep lack criteria, AC3 has no
reference answer, and the ADR-0038 rule conflict would cause AC1 to fail.

**Strengths**:
- AC2 directly verifiable.
- AC1 is an auditable enumerate-and-check procedure.
- AC4 names three concrete facts.
- Disposition taxonomy gives each option an observable test.
- Review criterion deliberately changed to an observable state.

**Findings**:
- major / high — Requirements / Assumptions / Drafting Notes — Two
  conflicting rules for ADR-0038; one guarantees AC1 failure. Keep the
  "names changed machinery" rule.
- major / high — Acceptance Criteria — No criterion verifies the ADR-0023
  successor's required content; add an AC4-style criterion.
- major / high — Acceptance Criteria — Corpus-wide cross-reference update has
  a criterion only for SKILL.md; phrase as a search result over a defined
  scope.
- major / medium — AC3 — "Reader can correctly describe" has no reference
  answer; replace with an enumerated content check against named Rust
  sources.
- minor / medium — AC5 — "Reaches `status: accepted`" is reviewer discretion,
  so it is always satisfiable.
- minor / medium — AC1 / AC6 — AC1's corpus scope ambiguous; AC6's escape
  clause loosens the pass condition.

## Re-Review (Pass 2) — 2026-10-08T22:19:26+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Completeness / Testability**: ADR-0037's contents described
  inaccurately — Resolved
- 🟡 **Testability / Clarity / Completeness / Scope**: Contradictory ADR-0038
  rules and missed stale path — Resolved
- 🟡 **Clarity**: Two-band model assigned to both successors — Resolved
- 🟡 **Testability**: No criterion for the ADR-0023 successor's content —
  Partially resolved (named subset only; see carry-forward finding below)
- 🟡 **Testability / Scope / Clarity / Dependency**: Cross-reference sweep
  unbounded — Resolved (scope bounded; AC7 self-match and
  Requirement/criterion mismatch remain as minors)
- 🟡 **Testability**: AC3 had no reference answer — Resolved
- 🔵 **Clarity / Testability**: Staleness test inconsistent, AC1 scope
  ambiguous — Resolved
- 🔵 **Clarity**: §5 conflated with callback determinism — Resolved
- 🔵 **Clarity / Scope**: Summary supersede-only framing — Resolved
- 🔵 **Dependency**: Successor ordering not stated — Resolved (wording of the
  rule still unclear; see below)
- 🔵 **Dependency**: ADR-0047/0052/0053 not addressed — Resolved
- 🔵 **Dependency**: 0070/0214/0157 not linked — Resolved
- 🔵 **Testability**: AC5 always satisfiable — Resolved
- 🔵 **Completeness**: Open Questions empty — Resolved
- 🔵 **Completeness**: No disposition record location — Resolved (Successor
  column has no fill step; see below)
- 🔵 **Clarity**: Bare numeric references — Resolved
- 🔵 **Dependency**: No Blocks entry — Partially resolved (0136's closure gate
  not recorded)

### New Issues Introduced

- 🟡 **Completeness / Testability**: Successors' required content omits
  still-valid decisions — the per-successor lists read as exhaustive but drop
  ADR-0023's clean-tree pre-flight, preview, VCS-revert rollback, skipped
  ledger and discoverability replacement; ADR-0037's accept/edit/skip
  semantics and three resumability guarantees; ADR-0038's hybrid
  application, extra display elements, edit targets and `Source:`→`parent`
  rule. Superseding retires every decision, so add a carry-forward criterion
  (each Decision-section item restated or explicitly dropped with reason).
- 🟡 **Testability**: ADR-0038 successor's session-log record shape has no
  criterion — AC5 omits the record-field comparison the Requirements demand.
- 🔵 **Scope / Testability**: Three-way disposition rule is vestigial — the
  Dispositions table and AC2 fix Supersede for all three, so Deprecate/Retain
  can never pass.
- 🔵 **Clarity**: Session-log record shape ownership (engine vs migration
  0007) unassigned between the ADR-0037 and ADR-0038 successors.
- 🔵 **Clarity**: Context says Phase 10 "deleted every bash mechanism listed
  above", which reads as deleting the invariants carried into Rust.
- 🔵 **Clarity**: "its predecessor's successor" collides with supersession
  vocabulary.
- 🔵 **Clarity**: Repointing Requirement says "repoint" while AC7 also admits
  labelled-historical citations.
- 🔵 **Testability**: AC7's search scope matches 0202 itself.
- 🔵 **Testability**: AC6 mixes literal and conceptual tokens, omits
  `hooks/migrate-discoverability.sh`, the `*.sh` glob, and `migration_*`
  names, and its exception needs judgement.
- 🔵 **Completeness**: No step fills the Dispositions Successor column.
- 🔵 **Dependency**: 0136's closure gate is not a Blocks edge; sibling
  migrate-crate items 0241 and 0264 not checked for overlap; timing of the
  `create-adr --supersedes` status flip not captured.
- 🔵 **Scope**: ADR-0037 successor is the dominant, judgement-heavy effort;
  "task" kind may undersell it.
- 🔵 **Clarity / Testability / Completeness** (suggestions): gloss sticky
  skip / source drift / predicate routing; tighten AC4 to the Requirements'
  definitions; add Rust sources, docs-site page and `create-adr` to
  References.

### Assessment

All six first-pass majors are resolved or reduced to minors. Two new majors
remain, both about the successor ADRs silently dropping content: no
carry-forward criterion for the still-valid decisions each superseded ADR
makes, and no criterion for the ADR-0038 successor's record shape. Both are
small, mechanical additions; the remaining minors are wording and
verification-precision fixes. One more editing pass should reach COMMENT or
APPROVE.

## Re-Review (Pass 3) — 2026-10-08T22:47:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Completeness / Testability**: Successors omit still-valid decisions —
  Partially resolved (carry-forward requirement and AC added; checklist is
  open-ended and mis-keyed to Decision sections, see below)
- 🟡 **Testability**: ADR-0038 record shape has no criterion — Resolved
- 🔵 **Scope / Testability**: Vestigial three-way rule — Resolved
- 🔵 **Clarity**: Record-shape ownership unassigned — Resolved
- 🔵 **Clarity**: "Deleted every bash mechanism" ambiguity — Resolved
- 🔵 **Clarity**: "Predecessor's successor" wording — Resolved
- 🔵 **Clarity**: Repoint requirement vs AC7 mismatch — Resolved
- 🔵 **Testability**: AC7 matched 0202 — Resolved
- 🔵 **Testability**: AC6 conceptual tokens — Resolved (now conflicts with the
  carried-forward section, see below)
- 🔵 **Completeness**: Successor column fill step — Resolved
- 🔵 **Dependency**: 0136 gate, 0241/0264 overlap, create-adr timing — Resolved
- 🔵 **Scope**: ADR-0037 successor effort — Resolved (Summary)
- 🔵 Suggestions (glosses, AC4 precision, References) — Resolved

### New Issues Introduced

- 🟡 **Testability / Completeness**: Carried-forward checklist is open-ended
  ("include") and mis-keyed — ADR-0023's no-dry-run, VCS-revert rollback and
  SessionStart choices sit in Considered Options, not Decision; the list
  omits `# DESCRIPTION:`, abort-on-failure, the summary table, the first
  migration, the force bypass; ADR-0037's non-opt-in mechanical path,
  uniform/hybrid freedom, and edit-validity ownership. Replace with a closed
  checklist keyed to each ADR's Decision subheadings plus chosen options.
- 🟡 **Testability / Clarity / Completeness**: Literal-search AC conflicts
  with the carried-forward section — dropped-decision entries naturally name
  the literals; "Superseded design" appears only in the AC. Permit the
  carried-forward section, name its heading in Requirements, state case
  sensitivity.
- 🔵 **Testability / Clarity**: Force bypass required but unchecked; name
  `ACCELERATOR_MIGRATE_FORCE`; define foreign dirt; align guarded resume with
  the unchanged-run-base condition.
- 🔵 **Clarity**: "Callbacks" in the determinism rule over-reaches
  (`apply_decision` mutates, `finalise` runs once); name the covered methods.
- 🔵 **Testability**: `verify_applied`/`finalise` purposes and record fields
  have no reference answer; "covers" display elements/controls is loose.
- 🔵 **Clarity**: "Supplements"/"parameterises" lack an observable marker.
- 🔵 **Clarity**: "Drift" overloaded (ledger-path staleness vs source drift).
- 🔵 **Testability**: Overturned-disposition branch has no verifiable end
  state.
- 🔵 **Dependency**: In-flight 0298 (SessionStart hook consolidation) not
  captured; 0115/0116/0117 (decisions bridge, stall) not linked or decided.
- 🔵 **Scope**: Three serially gated acceptance cycles in one task; optional
  split into three children. Title undersells first-time decision recording.
- 🔵 Suggestions: AC1 triggering-text clause already met; AC7 phrased so
  "replaced" hits cannot exist; frontmatter linkage values and "non-done"
  undefined; meaning of "hold" and §5 carry-forward unclear.

### Assessment

Both pass-2 majors are addressed in substance, but the carry-forward fix
introduced two related majors: its checklist is open-ended and keyed to the
wrong ADR sections, and it collides with the literal-search criterion. Both
are resolved by one change — a closed per-ADR checklist under a named
heading that the literal-search AC permits. The minors are precision fixes.

## Re-Review (Pass 4) — 2026-10-08T23:05:56+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Completeness**: Open-ended, mis-keyed carried-forward
  checklist — Resolved (closed, keyed to Decision headings and chosen
  options; completeness confirmed item-for-item against all three ADRs)
- 🟡 **Testability / Clarity / Completeness**: Literal-search vs
  carried-forward conflict — Resolved (list still narrower than the rule;
  see below)
- 🔵 **Testability / Clarity**: Force bypass, foreign dirt, run base —
  Resolved
- 🔵 **Clarity**: "Callbacks" over-reach — Resolved
- 🔵 **Testability**: `verify_applied`/`finalise`/record-field reference
  answers — Resolved
- 🔵 **Clarity**: Supplements/parameterises marker — Resolved
- 🔵 **Clarity**: Overloaded "drift" — Partially resolved (`verify_applied`
  failure still called source drift)
- 🔵 **Testability**: Overturned-disposition end state — Partially resolved
  (branch added but does not cascade; see below)
- 🔵 **Dependency**: 0298, 0115/0116/0117 — Resolved
- 🔵 **Scope**: Three serial cycles; title — Resolved (kept as one item by
  decision; title changed)
- 🔵 Suggestions (AC1 clause, AC7 phrasing, frontmatter, "hold", §5) —
  Resolved

### New Issues Introduced

- 🟡 **Completeness**: ADR-0023 successor brief omits shipped base-framework
  decisions no ADR records — migration-level `--skip`/`--unskip` and the
  `migrations-skipped` ledger (which ADR-0037 §4 already cites as "ADR-0023's"
  mechanism), the `NoOpPending` soft skip, the run-level advisory lock, and
  the pre-flight's `.accelerator/` scan.
- 🟡 **Clarity**: Dispositions marking is ambiguous — which column takes
  Confirmed / Not superseded, and where the follow-up ID goes.
- 🔵 **Scope / Testability / Clarity**: Not-superseded branch does not
  cascade — successor-content, chain, and repointing criteria assume every
  successor exists.
- 🔵 **Testability**: ADR-0038 line-124 accounting open-ended; close it over
  the eight named fields and define "folded into a composite value".
- 🔵 **Testability**: Accept/edit/skip criterion checks presence, not the
  expected effects and `outcome` literals.
- 🔵 **Testability**: Repointing AC does not check MMMM is the matching
  successor, nor that citations were not simply deleted.
- 🔵 **Testability**: Literal list narrower than the "bash-era names" rule
  (`# DESCRIPTION`, `PROJECT_ROOT`, `hooks/` paths).
- 🔵 **Clarity**: "Proposed value" display element vs ADR-0037's "proposed
  transformation" and the `proposed_value` field.
- 🔵 **Clarity**: "What 0007 puts into each record" underspecified.
- 🔵 **Dependency**: Approver for the three acceptances unnamed.
- 🔵 Suggestions: define path manifest, stall, migration lag; reword "not
  the hook that delivers it"; note that 0241/0298 should cite successors;
  route disputes over new-decision substance to a follow-up.

### Assessment

Both pass-3 majors are resolved. Two new majors remain: one substantive (the
base-framework mechanics shipped since ADR-0023 — notably migration-level
skip — have no home), one mechanical (Dispositions marking). Three lenses
independently flag the Not-superseded branch as the source of residual
ambiguity; removing it, since review has already confirmed all three
triggering texts no longer hold, would dissolve the clarity major and three
minors at once.

## Re-Review (Pass 5) — 2026-10-08T23:13:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Completeness**: Base-framework mechanics unhomed — Resolved
  (completeness now reports no findings)
- 🟡 **Clarity**: Dispositions marking ambiguous — Resolved
- 🔵 **Scope / Testability / Clarity**: Not-superseded branch — Resolved
- 🔵 **Testability**: Line-124 accounting, accept/edit/skip effects,
  repoint-to-matching-successor, literal list — Resolved
- 🔵 **Clarity**: "Proposed value" display element — Partially resolved
  (accept semantics still say "proposed value" in plain text)
- 🔵 **Clarity**: `verify_applied` as source drift; undefined terms;
  hook wording — Resolved
- 🔵 **Dependency**: Approver; sibling citations — Resolved (obligations on
  0241/0298 are recorded only here)

### New Issues Introduced

- 🟡 **Clarity**: "Migration lag" Terms entry (neither applied nor skipped)
  contradicts the shipped rule — `cli/migrate-cli/src/discoverability.rs`
  still compares highest applied against highest bundled ID, as ADR-0023
  states, with a legacy-ledger fallback. Introduced by the pass-4 edit.
- 🟡 **Testability**: Content criteria check presence and checklist
  partition, not accuracy against the shipped Rust; a wrong field
  classification or a mis-filed Restated item passes.
- 🔵 **Clarity**: Banned literals name checklist items that may belong
  under Restated (`# DESCRIPTION:`, `PROJECT_ROOT`).
- 🔵 **Clarity**: `verify_applied` neither in nor explicitly out of the
  repeatable-callbacks set.
- 🔵 **Clarity**: Summary's list of new decisions omits migration-level
  skip and 0007's field filling.
- 🔵 **Testability**: Replace-vs-annotate rule never fails; "not the
  predecessor" negations lack a failing condition; process constraints
  (ordering, verbatim predecessor bodies) unverified; invocation surfaces,
  trait mapping, and vocabulary bound looser in ACs than Requirements.
- 🔵 **Scope**: Repoint sweep boundary moves with sibling work; repointing
  not tied to per-successor checkpoints; sibling guidance sits here.
- 🔵 **Dependency**: create-adr/review-adr behaviour and the
  SKILL.md/docs-site mirror relationship not listed as couplings.

### Assessment

Pass-4 majors are resolved and completeness is clean. One new major is a
factual error introduced by the previous edit and is a one-line fix. The
other asks the work item to pin the successors' correct content in advance,
which shades into drafting the ADRs inside the work item; a cited-source
criterion addresses it more proportionately. Findings are now at a fine
grain and successive passes surface new ones at a similar rate, so further
passes have diminishing returns.

## Verdict Change — 2026-10-08T23:24:11+00:00

**Verdict:** COMMENT (changed from REVISE by Toby Clemson)

Pass 5's two majors and its minors were addressed after the pass without a
further lens run:

- "Migration lag" corrected to ADR-0023's unchanged highest-ID rule.
- Accuracy: every "Restated" entry must cite the Rust type, method, or
  `SKILL.md` section that shows it holds; predecessor diffs are limited
  to frontmatter.
- Checklist items renamed free of banned literals; `verify_applied`
  added to the repeatable callbacks; Summary lists every successor's new
  decisions; negation test for "supplements … not" defined.
- Repointing is per-successor with a closing sweep; known citations
  replaced, not annotated; the generated docs-site page is regenerated
  rather than edited; create-adr/review-adr reliance listed.

The major-count threshold applied to pass 5 alone would still yield
REVISE. The override reflects that the residual findings were resolved
and that successive passes were surfacing new findings at diminishing
grain.

## Verdict Change — 2026-10-08T23:25:59+00:00

**Verdict:** APPROVE (changed from COMMENT by Toby Clemson)

The work item is approved as ready for implementation.
