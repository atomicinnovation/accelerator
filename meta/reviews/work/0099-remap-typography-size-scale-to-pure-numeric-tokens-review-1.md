---
type: work-item-review
id: "0099-remap-typography-size-scale-to-pure-numeric-tokens-review-1"
title: "Work Item Review: Remap Typography Size Scale To Pure-Numeric Tokens"
date: "2026-06-13T07:16:27+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0099"
work_item_id: "0099"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-13T08:55:33+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Remap Typography Size Scale To Pure-Numeric Tokens

**Verdict:** COMMENT

Work item 0099 is a thoroughly populated, coherent, single unit of work: a
value-preserving rename of the entire `--size-*` typography scale to a
pure-numeric scheme, with the accompanying ADR supersession and consumer/test
updates shipped in lockstep. Every section is present and substantive, the full
19-token rename mapping leaves no guesswork, and the boundaries (px-vs-rem
deferred to 0091; non-size tokens out of scope) are drawn cleanly. The findings
are all polish rather than blockers — they cluster on the Acceptance Criteria
(undefined test-harness cross-references and outcomes stated without a stated
verification procedure) and on the successor ADR being a first-class deliverable
that lacks a stable handle and a Dependencies entry. Work item is acceptable but
could be improved — see the major finding below.

### Cross-Cutting Themes

- **Acceptance Criteria lean on undefined test-harness names** (flagged by:
  clarity, testability) — "AC5 ratchet", "EXCEPTIONS hygiene check", and "AC4"
  appear in the ACs, but the ACs are unnumbered checkboxes so the "ACn"
  references have no in-document referent, and the harness terms are never
  glossed. Clarity reads them as unresolvable referents; testability reads AC5
  as bundling several checks behind one undefined gate. Both force the reader
  into `migration.test.ts` to understand the completion gates.
- **Outcomes stated without a verification procedure** (flagged by:
  testability, clarity) — AC1 (no old names remain) and AC2 (comment rewrite)
  state target states but no concrete check, even though sibling 0090 supplies
  exact `rg` patterns for the analogous gate. AC4's "every affected surface" is
  verified only by incidental screenshot coverage.
- **Successor ADR underspecified as a deliverable** (flagged by: completeness,
  dependency) — the new ADR is a hard deliverable and the linchpin connecting
  this remap to 0091's later unit decision, yet it has no placeholder handle
  (unlike 0090's "the new radius ADR") and no explicit Dependencies bullet.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: "AC5 ratchet" references an undefined numbered criterion
  **Location**: Acceptance Criteria
  The final AC bullet and the Migration approach refer to "the AC5 ratchet", but
  this work item's ACs are unnumbered checkboxes, so "AC5" has no resolvable
  referent within the document — it points at an external test the reader cannot
  identify, so they cannot confirm what the gate checks without hunting through
  `migration.test.ts`.

#### Minor

- 🔵 **Clarity**: "EXCEPTIONS hygiene check" and "AC4" used without definition
  **Location**: Acceptance Criteria
  The ACs cite "the EXCEPTIONS hygiene check" and lean on harness concepts that
  originate in ADR-0036 without defining them here; a reader who has not read
  ADR-0036's migration-test internals cannot tell what EXCEPTIONS is or why this
  rename must keep its check green.

- 🔵 **Dependency**: 0091's dependence on 0099's successor ADR is not captured
  from 0099's side
  **Location**: Dependencies
  0099 frames 0091 only as "orthogonal", but 0091's own Dependencies and AC3
  treat 0099's successor ADR as a conditional prerequisite and prefer to
  "sequence after 0099". That makes 0091 a downstream consumer enabled by 0099's
  successor ADR, yet 0099 lists no Blocks/enables entry for it.

- 🔵 **Testability**: AC2 comment-rewrite criterion has no objective pass/fail
  condition
  **Location**: Acceptance Criteria
  AC2 requires the naming-convention comment be "rewritten to describe the
  pure-numeric scheme" but gives no condition a verifier could check — "describes
  the scheme" is subjective and could be claimed met by any non-trivial edit.

- 🔵 **Clarity**: "prototype drift fixture" referent is unexplained
  **Location**: Requirements
  Requirements instruct keeping "the prototype drift fixture green (it pins only
  `--code-*`/`--tk-*`/`--atomic-*`, not `--size-*`)" but never say what the
  fixture is or what drift it guards; the reader cannot judge whether the
  requirement is a genuine constraint or a no-op here.

- 🔵 **Completeness**: Successor ADR is never given a placeholder identifier
  **Location**: Requirements / Acceptance Criteria / Technical Notes
  The work item repeatedly references creating "a new ADR" but never assigns it
  a placeholder handle the way 0090 does ("referred to throughout as 'the new
  radius ADR'"). Without a stable name, downstream references (notably 0091's
  AC3) and the implementer have no consistent handle when both ADRs are in play.

- 🔵 **Dependency**: "In-flight typography work" churn coupling names no concrete
  work item
  **Location**: Dependencies
  The bullet "Touches the same global.css / tokens.ts / migration.test.ts
  surfaces as any in-flight typography work; coordinate sequencing" names a real
  merge coupling but identifies no specific work item, so a reviewer cannot
  confirm whether a conflicting item exists or has been sequenced.

- 🔵 **Dependency**: Successor-ADR creation is a required output coupling but not
  surfaced in Dependencies
  **Location**: Requirements
  Creating the superseding ADR is a hard deliverable and an artefact coupling
  (it must carry px-anchoring forward pointing at 0091), but this lives only in
  Requirements/Migration approach prose and is not mirrored as a Dependencies
  entry, making the ADR-chain easy to overlook when scheduling against 0091.

- 🔵 **Scope**: `kind: task` may under-signal the work relative to 0075/0090
  precedent
  **Location**: Frontmatter: kind
  The item is declared `kind: task`, but its scope — 19 renamed tokens, ~100
  consumer references, an ADR supersession, a convention rewrite, and guardrail
  updates — matches two near-identical in-repo precedents (0075, 0090) that were
  both delivered as `kind: story`. A sizing/labelling judgement, not a
  delivery-blocking split; the unit of work itself is coherent and atomic.

- 🔵 **Testability**: AC1 states the outcome but not the verification procedure
  for "no old names remain"
  **Location**: Acceptance Criteria
  AC1 defines the target state but, unlike 0090's AC3 (which supplies exact `rg`
  patterns), specifies no procedure. The `global.css` declaration set is not
  covered by the AC3 `var()`-resolves test, which checks consumer references —
  so verifying "no old names remain among declarations" falls to manual reading.

- 🔵 **Testability**: AC4 visual-invariance has no per-surface anchor; relies
  solely on unchanged baselines
  **Location**: Acceptance Criteria
  AC4 discharges its "every affected surface" claim entirely through existing
  screenshot baselines remaining byte-identical. 0090 additionally specified
  per-selector `getComputedStyle` assertions; without an equivalent, a surface
  whose token is mis-renamed but lacks baseline coverage would not be caught.

- 🔵 **Testability**: AC5 bundles multiple distinct ADR checks under one
  criterion
  **Location**: Acceptance Criteria
  AC5 bundles six independently-verifiable conditions into one checkbox (ADR
  exists, supersedes ADR-0036, documents the scheme, records migration, ADR-0036
  marked superseded, carries px-anchoring forward pointing at 0091). It can be
  marked done when only the easy checks are met, while the load-bearing
  px-anchoring carry-forward is weakly specified.

#### Suggestions

- 🔵 **Clarity**: "tween" / "tween suffix" jargon used without definition
  **Location**: Context
  `-sm`/`-lg` names are called "tween suffixes"; "tween" (an interpolated
  half-step between integer tiers) is borrowed jargon a new team member would
  likely need explained. The surrounding table makes intent largely recoverable.

- 🔵 **Completeness**: The px×10 encoding rule for sub-100 values is exemplified
  but not stated
  **Location**: Requirements / Technical Notes (Full rename mapping)
  The rule `--size-<px×10>` is shown via examples; the general behaviour for
  whole-number px below 10 (9.5→95, two-digit; vs 11→110, three-digit) is left
  to infer from the table. Inert today; matters only if the scale is later
  extended below 10px.

- 🔵 **Clarity**: Dense pronouns ("its" / "this") in the px-anchoring
  carry-forward requirement
  **Location**: Requirements
  The carry-forward sentence stacks "this remap", "the successor ADR", and "the
  px-vs-rem trade-off" with several pronouns across clause boundaries; recoverable
  but easy to misattribute which ADR amends which.

### Strengths

- ✅ Summary, Context, Requirements, Acceptance Criteria, and the Technical Notes
  rename table all agree on the same scope (the entire scale, hero → 4xs) and
  the same invariant (byte-identical computed px) — no cross-section scope drift.
- ✅ The "Full rename mapping" table enumerates all 19 old→new token pairs with
  px values, asserts the px×10 encoding produces no collisions, and gives the
  implementer the exact expected token set with no follow-up questions.
- ✅ Potential contradictions around ADR supersession (work item vs successor ADR
  owning the `supersedes` edge) and 0091 coordination (supersede vs amend) are
  explicitly identified and reconciled in Open Questions, Dependencies, and
  Drafting Notes, leaving a single coherent reading.
- ✅ Clean orthogonality boundary with 0091: the naming axis (this item) and the
  unit axis (0091, px-vs-rem) are explicitly separated, and 0091's later decision
  is directed to amend rather than fork the successor ADR — matching 0091's own
  recommendation.
- ✅ Decoupled from 0094 with rationale ("should NOT block 0094"); the lockstep
  artefact coupling (global.css declarations, tokens.ts mirror, consumers,
  migration.test.ts guardrails) is named with the `var()`-resolves-to-declared-token
  test designated as the completeness gate.
- ✅ Open Questions is explicitly closed ("None outstanding") with each formerly-open
  decision named and cross-referenced to Drafting Notes, and scope is delimited
  (`--lh-*`/`--tracking-caps` out; `--size-h4` flagged in).

### Recommended Changes

1. **Make the Acceptance Criteria self-contained and individually verifiable**
   (addresses: "AC5 ratchet" undefined referent, "EXCEPTIONS hygiene check"
   undefined, AC5 bundling, AC1/AC2 missing procedures) — Number the ACs so
   "ACn" cross-references resolve within the document; gloss "AC5 ratchet" and
   "EXCEPTIONS hygiene check" on first use (or link ADR-0036's definitions); split
   AC5 into separately-checkable bullets; and add concrete sweeps — e.g. a grep
   over `--size-*` declarations in `global.css` for any name not matching
   `--size-[0-9]+` returns zero matches (AC1), and a minimum-content check for the
   convention comment (AC2), mirroring 0090's AC3 `rg`-pattern gates.

2. **Give the successor ADR a stable handle and a Dependencies entry**
   (addresses: no placeholder identifier, successor-ADR not in Dependencies) —
   Adopt 0090's convention: state once that the successor ADR's ID is allocated
   at creation and is referred to throughout as "the successor ADR", and add a
   Dependencies bullet naming it as a required same-PR artefact that 0091 will
   later amend.

3. **Capture 0091's downstream dependence and name the in-flight coupling**
   (addresses: 0091 coupling not captured from 0099's side, unnamed in-flight
   work) — Add a Dependencies note that 0099's successor ADR is the artefact 0091
   prefers to chain off (per 0091 AC3), and either name the specific in-flight
   typography items touching these surfaces or state explicitly that none are open
   beyond 0091.

4. **Resolve the kind: task vs story sizing question** (addresses: kind
   under-signal) — Either relabel to `kind: story` to match the 0075/0090
   precedent, or add a one-line Drafting Note justifying why this is intentionally
   a task despite the larger surface (e.g. purely mechanical find-replace, zero
   conceptual decisions remaining).

5. **Strengthen the visual-invariance guarantee and minor glosses** (addresses:
   AC4 per-surface anchor, "prototype drift fixture", "tween", px×10 sub-100 rule,
   pronoun density) — Either scope AC4's claim to baseline-covered surfaces or add
   a positive computed-value check per the 0075/0090 `*-resolved-*` pattern; and
   add brief one-line glosses for "prototype drift fixture" and "tween", state the
   px×10 encoding is literal with no zero-padding, and split the dense
   px-anchoring sentence naming each ADR explicitly.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its core intent — a pure-numeric rename
of the entire `--size-*` scale — with strong internal consistency: Summary,
Context, Requirements, and Acceptance Criteria all describe the same whole-scale,
value-preserving rename, and the Drafting Notes explicitly reconcile the
ADR-supersession and 0091-coordination wording that could otherwise contradict.
The main clarity gaps are a cluster of undefined cross-references and harness
jargon ("AC5 ratchet", "EXCEPTIONS hygiene check", "AC4", "prototype drift
fixture", "tween") whose meaning the reader cannot recover from this document
alone. None rise to a contradiction; they are referent/jargon-resolution issues
that force a reader to consult external test code.

**Strengths**:
- Summary, Context, Requirements, Acceptance Criteria, and the Technical Notes
  rename table all agree on the same scope and invariant — no cross-section scope
  drift.
- The full rename mapping table makes every token's before/after name and px value
  unambiguous.
- Potential contradictions around ADR supersession and 0091 coordination are
  explicitly identified and reconciled in Open Questions, Dependencies, and
  Drafting Notes.
- The pure-numeric naming convention (`--size-<px×10>`) is defined inline with
  worked examples on first use.

**Findings**:
- 🟡 **major** (high) — Acceptance Criteria — "AC5 ratchet" references an
  undefined numbered criterion. The final AC bullet and the Migration approach
  refer to "the AC5 ratchet", but the ACs are unnumbered checkboxes, so "AC5"
  has no resolvable referent — it points at an external test the reader cannot
  identify. Suggestion: define "AC5" on first use or replace it with a
  self-contained description of what the ratchet enforces.
- 🔵 **minor** (high) — Acceptance Criteria — "EXCEPTIONS hygiene check" and
  "AC4" used without definition. A reader who has not read ADR-0036's
  migration-test internals cannot tell what EXCEPTIONS is. Suggestion: add a
  one-line gloss on first use or link ADR-0036's definition.
- 🔵 **minor** (medium) — Requirements — "prototype drift fixture" referent is
  unexplained. The requirement to keep it green never says what the fixture is
  or what drift it guards. Suggestion: add a brief gloss or link the
  artefact/test that defines it.
- 🔵 **suggestion** (medium) — Context — "tween" / "tween suffix" jargon used
  without definition. Borrowed design jargon carrying the specific meaning "a
  0.5px half-step between integer tiers". Suggestion: gloss on first use.
- 🔵 **suggestion** (low) — Requirements — Ambiguous "its" / "this" in the
  px-anchoring carry-forward requirement. Several pronouns and noun phrases stack
  in one long sentence. Suggestion: split the sentence and name the ADR explicitly
  each time.

### Completeness

**Summary**: Work item 0099 is a thoroughly populated task: Summary, Context,
Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
Technical Notes, Drafting Notes, and References are all present and substantively
filled, and the frontmatter (kind: task, status: draft, priority: medium) is
valid. As a mechanical rename task it carries the content its kind demands — a
clear definition of the work, a complete old→new token mapping, an enumerated
surface list, and a completeness gate. The only gaps are minor: the successor-ADR
deliverable is described but never given a placeholder identifier, and the
half-step encoding rule has an implicit edge that goes unstated.

**Strengths**:
- Every expected section for a task is present and substantively populated — no
  empty or placeholder-only sections.
- The "Full rename mapping" table gives the complete old→new mapping for all 19
  tokens.
- Acceptance Criteria contains six specific criteria, well above the two-criterion
  minimum.
- Open Questions is explicitly closed ("None outstanding") with each formerly-open
  decision named and cross-referenced.
- Frontmatter is complete and valid.

**Findings**:
- 🔵 **minor** (medium) — Requirements / Acceptance Criteria / Technical Notes —
  The successor ADR is never given a placeholder identifier the way sibling 0090
  does ("referred to throughout as 'the new radius ADR'"). Without a consistent
  handle, downstream references (0091's AC3) and the implementer have no stable
  name. Suggestion: adopt 0090's convention — state once that the successor ADR's
  ID is allocated at creation and is referred to throughout as "the successor ADR".
- 🔵 **suggestion** (low) — Requirements / Technical Notes — The encoding rule
  `--size-<px×10>` is given with examples, but the general rule for whole-number
  px below 10 (95 two-digit vs 110 three-digit) is left to infer from the table.
  Inert today. Suggestion: add one sentence noting the encoding is literally px×10
  with no zero-padding.

### Dependency

**Summary**: This task is unusually well dependency-mapped: it explicitly
decouples from 0094 (done plan, no rework), declares the ADR-0036 supersession
coupling and its modelling (successor ADR owns the edge), and flags the
orthogonal-but-coordinated relationship with 0091 plus the lockstep coupling
across global.css/tokens.ts/migration.test.ts. The principal gap is an asymmetric
downstream coupling: 0091's own Dependencies treat 0099's successor ADR as a
conditional prerequisite and prefer to sequence after 0099, yet 0099 frames 0091
only as "orthogonal" and never lists it as a Blocks/enabled-by entry. A secondary
gap is the unnamed "in-flight typography work" churn-coordination note.

**Strengths**:
- The Dependencies section explicitly states 0094 is decoupled and this work
  "should NOT block 0094", with rationale.
- The ADR-0036 supersession coupling is captured precisely and consistently
  across Requirements, AC, Migration approach, and Drafting Notes, including the
  deliberate decision that the successor ADR owns the `supersedes` edge.
- The lockstep artefact coupling is named in Requirements, Technical Notes, and
  Migration approach, with the var()-resolves-to-declared-token test designated as
  the completeness gate.
- The cross-coupling with 0091 is addressed from this side: framed as orthogonal,
  with a preferred sequencing and the expectation that 0091 amends rather than
  forks the successor ADR.

**Findings**:
- 🔵 **minor** (high) — Dependencies — 0091's dependence on 0099's successor ADR
  is not captured as a downstream coupling from 0099's side. 0091's own
  Dependencies and AC3 treat the successor ADR as a conditional prerequisite and
  prefer to "sequence after 0099", yet 0099 lists no Blocks/enables entry.
  Suggestion: add a note that 0099's successor ADR is the artefact 0091 prefers to
  chain off (per 0091 AC3).
- 🔵 **minor** (medium) — Dependencies — The "in-flight typography work" churn
  coupling names no concrete work item, so the coordination instruction is not
  trackable. Suggestion: name the specific items or state explicitly that none are
  open beyond 0091.
- 🔵 **minor** (medium) — Requirements — Successor-ADR creation is a required
  output coupling but not surfaced in Dependencies; the ADR-chain coupling lives
  only in prose and is easy to overlook when scheduling against 0091. Suggestion:
  add a Dependencies bullet naming the successor ADR as a required same-PR artefact
  that 0091 will later amend.

### Scope

**Summary**: Work item 0099 is a coherent, well-bounded single unit of work: a
whole-scale rename of the typography `--size-*` tokens to a pure-numeric scheme,
with the accompanying ADR supersession and consumer/test updates shipped together.
The Summary, Requirements, and Acceptance Criteria all describe the same scope,
and the boundaries (px-vs-rem deferred to 0091; non-size typography tokens out of
scope) are stated clearly. The one notable scope signal is the `kind: task`
declaration against a scope — ~100 consumer references, 19 renamed tokens, an ADR
supersession, and guardrail-test updates — that two near-identical in-repo
precedents (0075, 0090) both delivered as stories.

**Strengths**:
- Single unified purpose: every requirement serves the one rename concern; no
  "and also" bundling.
- Clean orthogonality boundary with 0091 (naming axis vs unit axis), directing
  0091's later decision to amend rather than fork the successor ADR.
- The ADR supersession is part of the same coherent unit, mirroring the 0075/0090
  token-rule precedent.
- Scope is explicitly delimited: whole scale justified by internal consistency;
  `--lh-*`/`--tracking-caps` out of scope; `--size-h4` flagged in-scope.
- Decoupled from 0094, so the item has standalone value and clear sequencing.

**Findings**:
- 🔵 **minor** (medium) — Frontmatter: kind — Declared `kind: task`, but its scope
  (19 tokens, ~100 references, ADR supersession, convention rewrite, guardrail
  updates) matches two near-identical precedents (0075, 0090) both delivered as
  `kind: story`. A sizing/labelling judgement, not a delivery-blocking split.
  Suggestion: relabel to `kind: story`, or add a Drafting Note justifying the task
  label (purely mechanical find-replace, zero conceptual decisions remaining).

### Testability

**Summary**: The Acceptance Criteria are largely verifiable: most criteria point
to concrete mechanical checks (the var()-resolves-to-declared-token test, the
suite pass, byte-identical computed font sizes) and the Technical Notes supply a
complete 19-row rename mapping that pins the exact expected token set. The
principal weaknesses are AC1 and the comment-rewrite criterion (AC2), which state
outcomes without specifying the verification procedure — notably absent given the
sibling 0090 work item provides exact ripgrep patterns for the analogous gate. The
Summary's intent is covered by the criteria collectively, though no per-surface
enumeration anchors the visual-invariance claim the way 0090's AC2 does.

**Strengths**:
- AC3 names a concrete, existing mechanical gate (the var()-resolves test) and
  ties completeness to it passing with no stale references.
- AC4 expresses visual invariance as a measurable outcome — byte-identical computed
  font sizes and unchanged screenshot baselines — rather than a subjective "looks
  the same".
- The "Full rename mapping" enumerates all 19 old→new pairs, giving a verifier the
  exact expected token set to check AC1 against.
- The Assumptions section states the condition under which the find-replace +
  var()-resolves test is a complete gate, making the verification strategy's
  validity inspectable.

**Findings**:
- 🔵 **minor** (high) — Acceptance Criteria — AC2 comment-rewrite criterion has no
  objective pass/fail condition; "describes the scheme" is subjective.
  Suggestion: state the minimum content the comment must contain (the px×10
  encoding, one whole-step and one half-step example, no retired names).
- 🔵 **minor** (medium) — Acceptance Criteria — AC1 states the outcome but not the
  verification procedure for "no old names remain"; unlike 0090's AC3 it supplies
  no `rg` patterns, and the declaration set is not covered by the AC3 var()-resolves
  test. Suggestion: add a concrete sweep over the `--size-*` declarations.
- 🔵 **minor** (medium) — Acceptance Criteria — AC4 visual-invariance has no
  per-surface anchor; it relies solely on unchanged baselines, so a mis-renamed
  surface lacking baseline coverage would not be caught. Suggestion: scope the
  claim to baseline-covered surfaces or add positive computed-value checks.
- 🔵 **minor** (medium) — Acceptance Criteria — AC5 bundles multiple distinct ADR
  checks under one criterion, including a presence-only check for the px-anchoring
  carry-forward that 0091 depends on. Suggestion: split AC5 into separately-checkable
  bullets and make the carry-forward objective (a Neutral consequence restating the
  px-anchored stance, linking `work-item:0091`).

## Re-Review (Pass 2) — 2026-06-13

**Verdict:** REVISE

Pass-1 edits resolved nearly every structural finding: the ADR handle, the
EXCEPTIONS/drift-fixture/tween glosses, the px×10 encoding rule, all three
dependency couplings, and the kind-justification all landed cleanly, and the
scope finding was downgraded to "no change required". However, numbering the
acceptance criteria and adding the AC4 fallback each introduced a new **major**
finding — so the verdict moves to REVISE on the two-major threshold. Both majors
are narrow, mechanical refinements of the pass-1 edits, not structural problems.

### Previously Identified Issues

- 🟡 **Clarity**: "AC5 ratchet" references an undefined numbered criterion —
  **Partially resolved**. The inline disambiguation helps and EXCEPTIONS is now
  glossed, but numbering the ACs created this work item's own AC5, so the token
  "AC5" now carries two meanings (see new issue below).
- 🔵 **Clarity**: "EXCEPTIONS hygiene check" / "AC4" undefined — **Resolved**.
  AC8 now glosses EXCEPTIONS as the per-occurrence admitted-literal ledger guard.
- 🔵 **Clarity**: "prototype drift fixture" referent unexplained — **Resolved**.
  Requirements now glosses it as the design-prototype token-drift guard pinning
  only `--code-*`/`--tk-*`/`--atomic-*`.
- 🔵 **Clarity**: "tween" jargon — **Resolved**. Glossed inline in the Summary.
- 🔵 **Clarity**: dense pronouns in the px-anchoring requirement — **Resolved**.
  Sentence split, each ADR named.
- 🔵 **Completeness**: successor ADR has no placeholder identifier — **Resolved**.
  Requirements coins "the successor ADR" as the stable handle.
- 🔵 **Completeness**: px×10 sub-100 encoding unstated — **Resolved**. Requirements
  states the encoding is literal px×10 with no zero-padding.
- 🔵 **Dependency**: 0091's dependence not captured from 0099's side — **Resolved**.
  A "Downstream coupling" note now records it; re-review confirms it bidirectionally.
- 🔵 **Dependency**: in-flight typography work names no item — **Resolved**.
  Now states no competing item is open beyond 0091.
- 🔵 **Dependency**: successor-ADR creation not in Dependencies — **Resolved**.
  A "Required deliverable — the successor ADR" bullet was added.
- 🔵 **Scope**: `kind: task` may under-signal — **Resolved**. Drafting Notes now
  justify the task label; re-review rates it "no change required".
- 🔵 **Testability**: AC2 has no objective pass/fail — **Resolved**. AC2 now states
  required content plus a retired-name grep (one residual ellipsis nit, below).
- 🔵 **Testability**: AC1 states outcome, not procedure — **Partially resolved**.
  A grep is now described, but in prose rather than a runnable, precedent-style pattern.
- 🔵 **Testability**: AC4 has no per-surface anchor — **Partially resolved & escalated**.
  The added fallback introduced an unverifiable antecedent (see new major below).
- 🔵 **Testability**: AC5 bundles multiple ADR checks — **Resolved**. Split into
  AC5/AC6/AC7 (AC8 still bundles test conditions — minor, below).

### New Issues Introduced

- 🔴 **Clarity** (major): The token "AC5" now refers to two things — this work
  item's own AC5 ("Successor ADR created") and the `migration.test.ts` "AC5"
  coverage ratchet named in AC8. The inline flag helps but the bare "AC5" and the
  unnamed "earlier work item" leave the referent fragile. *Fix:* drop the "AC5"
  label from the work item's prose entirely and refer to the test by function (the
  aggregate `var(--*)` coverage ratchet in `migration.test.ts`), or name the
  earlier work item.
- 🔴 **Testability** (major): AC4's fallback ("if a renamed token is consumed
  solely by an un-baselined surface, add a positive computed-value assertion")
  depends on first determining which tokens are consumed only by un-baselined
  surfaces, and no procedure is given — so a token with no baseline coverage could
  pass AC4 vacuously. *Fix:* anchor byte-identical to the declaration level (each
  renamed token declares the identical px value per the mapping table) plus AC3's
  consumer-resolution gate, so invariance is provable by construction; or require
  enumerating each token's consumer selectors and confirming baseline-or-spec
  coverage of each.
- 🔵 **Clarity** (minor): Open Questions defers each resolution to Drafting Notes
  while Drafting Notes re-describes them — a cross-section round-trip (pre-existing).
- 🔵 **Clarity** (suggestion): AC8's "the ADR-0036 font-size ban (now enforced by
  the successor ADR)" leaves "now" temporally ambiguous and doesn't say whether the
  test's own ADR reference is updated as part of this work.
- 🔵 **Dependency** (minor): the 0091 enabling edge is in prose, not a scannable
  structured "Enables/Blocks" entry.
- 🔵 **Dependency** (minor): 0099 reads "sequence this remap first" as firm, but
  0091 records a sanctioned fallback where it decides first and chains off ADR-0036
  — the two should match strengths.
- 🔵 **Testability** (minor): AC2's retired-name grep list ends with an ellipsis,
  leaving the ban set undefined; enumerate the 19 old names or restate positively.
- 🔵 **Testability** (minor): AC1's grep is prose, not a runnable pattern like the
  0075/0090 precedents give.
- 🔵 **Testability** (minor): AC8's "full suites pass" is satisfiable by skipped or
  deleted tests; add a no-skip/no-regression guard on the named guardrails.
- 🔵 **Completeness** (suggestion): Requirement 5 says the successor ADR's ID is
  "allocated at creation time" but points at no ADR-ID allocation convention.
- 🔵 **Scope** (suggestion ×2): kind rationale is sound (no change required); minor
  tension between "purely mechanical" and the small ADR-authoring judgement — could
  be acknowledged in the kind rationale.

### Assessment

The work item is substantively stronger than at pass 1 — every structural and
dependency finding is resolved, and the residual issues are all confined to the
acceptance-criteria wording. The verdict is REVISE only because two narrow majors
appeared as a direct side-effect of the pass-1 AC edits (the "AC5" double-meaning
and the AC4 fallback's unverifiable antecedent). Both have a one-edit fix and do
not require rethinking the work. Applying the two major fixes (plus, optionally,
the AC1/AC2/AC8 testability nits) would clear the work item for implementation.

## Re-Review (Pass 3) — 2026-06-13

**Verdict:** COMMENT

Both pass-2 majors are resolved. Clarity no longer flags the "AC5" double-meaning
(the label was dropped from the work item's prose), and testability now lists AC4
as a strength ("verifiable by construction" via the mapping table + AC3). The
verdict improves REVISE → COMMENT. One new major surfaced — a direct artefact of
the pass-2 AC2 enumeration fix — and was fixed immediately as a follow-up (see
closing note).

### Previously Identified Issues (pass-2 majors)

- 🔴 → ✅ **Clarity**: "AC5" referred to two things — **Resolved**. The `AC5`
  label was removed from AC8 and the Technical Notes; the test is now named purely
  by function (the aggregate `var(--*)` coverage ratchet in `migration.test.ts`),
  leaving one "AC5" in the document.
- 🔴 → ✅ **Testability**: AC4 unverifiable coverage antecedent — **Resolved**.
  AC4 was re-anchored to the declaration level (each renamed token declares its
  old px value per the mapping table + AC3's consumer gate ⇒ byte-identical by
  construction); screenshots demoted to secondary confirmation. The testability
  agent now cites this as a strength.

### New Issues Introduced

- 🟡 **Testability** (major): AC2's enumerated retired-name grep used bare,
  unanchored substrings (`lg`, `sm`, `md`, `row`, `body`, `xs`, `h1`–`h4`), which
  match ordinary prose in a rewritten comment — so the check could never reliably
  "return nothing". A direct side-effect of the pass-2 enumeration fix. *Fixed*
  in the closing edits below.
- 🔵 **Testability** (minor): AC1's regex ran over the whole file, conflating the
  convention comment with declarations. *Fixed* (scoped to declaration lines).
- 🔵 **Testability** (minor): AC3 delegated to the resolve test without restating
  its pass condition. *Fixed* (pass condition + test location added).
- 🔵 **Testability** (suggestion): AC4's secondary screenshot check is
  platform-sensitive and states no tolerance. *Left* — it is explicitly secondary
  to the by-construction guarantee.
- 🔵 **Clarity** (suggestion): Summary's "superseding ADR-0036" under-specified
  that an ADR (not the work item) enacts it. *Fixed* ("via a new successor ADR").
- 🔵 **Clarity** (suggestion ×2): px×10 rule exemplified before stated in the
  Summary; "ratchet" / "carried into" jargon ungloss in AC8. *Left* — stylistic,
  the rule is stated in Requirements and the terms are standard CI/ADR vocabulary.

### Assessment

The work item is implementation-ready. Verdict COMMENT (one major, below the
two-major REVISE threshold), and that major plus the two coupled minors were
fixed as closing edits. Remaining open items are low-value stylistic suggestions
that do not affect verifiability. No pass-4 review was run: the closing edits are
mechanically self-evident (anchoring greps to the `--size-` token form, scoping
AC1 to declaration lines, restating AC3's pass condition), and further passes
would yield only diminishing stylistic suggestions.

### Closing edits applied after pass 3

- AC1 grep scoped to declaration lines:
  `rg -nP -- '^\s*--size-(?![0-9]+\s*:)[\w-]+\s*:' src/styles/global.css`.
- AC2 retired-name grep anchored to the `--size-` prefix
  (`--size-(hero|h1|…|4xs)\b`), so it matches token names only, never prose.
- AC3 now names the test location and restates its pass condition (every
  `var(--size-*)` resolves to a declared key, zero unresolved).
- Summary attributes supersession to the new successor ADR.

## Reviewer Decision — 2026-06-13

**Verdict:** APPROVE

Reviewer (Toby Clemson) accepted the work item after the pass-3 closing edits.
The lens-computed pass-3 verdict was COMMENT (one major, below the two-major
REVISE threshold) — that major and its coupled minors were fixed in the closing
edits, and the only remaining items are low-value stylistic suggestions with no
verifiability impact. The work item is approved and ready for implementation
planning.

