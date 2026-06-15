---
type: work-item-review
id: "0051-sync-work-items-skill-review-1"
title: "Work Item Review: Sync Work Items Skill"
date: "2026-06-15T19:23:10+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0051"
relates_to: ["work-item-review:0047-core-skills-sync-integration-review-1"]
work_item_id: "0051"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-06-15T21:12:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Sync Work Items Skill

**Verdict:** REVISE

The newly-added scope — extending `/list-work-items` to render the three
baseline-dependent sync states once `last-sync.json` exists — is clean and
well-formed: its requirement, four acceptance criteria, and the five-state
distinctness invariant were each cited as precisely verifiable, and the scope
boundary with sibling 0047 reads consistently from both stories. The REVISE
verdict is driven almost entirely by **pre-existing** content in the core
`/sync-work-items` deliverable (this is 0051's first review): a frontmatter vs
prose contradiction over which integrations actually block it, an unspecified
conflict path under the directional modes, and three core sync behaviours
(resumability, the conflict-diff default, content-equivalence) whose
verification procedures aren't pinned down.

### Cross-Cutting Themes

- **`blocked_by` frontmatter vs Dependencies prose** (flagged by: clarity,
  dependency) — The frontmatter hard-blocks on all three integration stories
  (0048/0049/0050), but the prose, the Drafting Notes, and epic 0045 all say
  only *one* integration is needed (Jira is already complete). The
  machine-readable graph and the human intent disagree, which would
  artificially serialise the capstone behind every integration.
- **Core sync behaviours lack pinned verification procedures** (flagged by:
  testability, clarity) — The most consequential behaviours — the
  conflict-resolution gate before an unrecoverable overwrite, crash
  resumability, and the content-equivalence comparison that drives every
  push/pull/conflict decision — are each described qualitatively ("safely",
  "highlighted as default", "logically equivalent") without an observable test,
  and the conflict path under `--push-only`/`--pull-only` is unspecified.

### Findings

#### Critical

- (none)

#### Major

- 🟡 **Clarity / Dependency**: Frontmatter `blocked_by` contradicts the "at least
  one integration" Dependencies prose
  **Location**: Frontmatter / Dependencies
  Frontmatter lists 0048, 0049, and 0050 all as hard blockers, but the prose,
  Drafting Notes ("blocked by all other planned stories"), and epic 0045 ("at
  least one integration… can be developed in parallel") say a single integration
  (Jira, complete) suffices. A scheduler reading the frontmatter would block the
  capstone behind Linear AND Trello AND GitHub unnecessarily.

- 🟡 **Clarity**: Conflict-item behaviour under `--push-only` / `--pull-only` is
  unspecified
  **Location**: Requirements
  The push and pull branches are mode-gated ("and mode permits push/pull"), but
  the "When both have changed" conflict branch carries no mode qualifier, and the
  directional ACs are silent on conflicts. An implementer cannot tell whether a
  conflict item is skipped, warned, or forced into a resolution the mode forbids.

- 🟡 **Dependency**: Empty `Blocks` field omits 0047's deferred states, which
  consume 0051's `last-sync.json`
  **Location**: Dependencies
  0047 records a forward data dependency on 0051's `last-sync.json` baseline
  (deliberately non-blocking, to avoid a cycle), but 0051's `Blocks` field is
  `—`, so the downstream consumer of its key output is invisible from 0051's own
  record.

- 🟡 **Testability**: Resumability criterion "resumes safely" has no measurable
  pass/fail procedure
  **Location**: Acceptance Criteria
  The interrupted-sync AC names two failure modes but doesn't enumerate the
  post-resume invariants or give a procedure to induce the interruption (where
  the loop is cut, what partial `last-sync.json` results), so two engineers could
  disagree on whether a resumed run passed.

- 🟡 **Testability**: Conflict-diff "highlighted as default" under-specifies the
  verifiable signal
  **Location**: Acceptance Criteria
  "Section-by-section" is pinned in Assumptions, but "remote version highlighted
  as default" is a presentation property with no observable check — the most
  consequential UX (the gate before an unrecoverable overwrite) could pass
  regardless of what is actually shown.

- 🟡 **Testability**: Normalised content-equivalence drives every sync decision
  but has no testable definition
  **Location**: Assumptions
  Every push/pull/conflict outcome depends on deciding whether local and remote
  are "equal", but the normalisation rules are deferred wholesale to the plan and
  no AC anchors an example of equal-vs-changed, so the core reconciliation logic
  is only partially testable and risks spurious conflicts on cosmetic diffs.

#### Minor

- 🔵 **Clarity**: Technical Notes uses the legacy guard-banned `meta/integrations/`
  path
  **Location**: Technical Notes
  Requirements say "the configured integrations path", but the Technical Notes
  example is `meta/integrations/jira/last-sync.json` — the legacy path epic 0045
  flags as guard-banned (resolved default `.accelerator/state/integrations/`).

- 🔵 **Completeness**: Open Questions is empty despite unresolved decisions noted
  elsewhere
  **Location**: Open Questions
  Genuine deferred decisions live inline (per-integration normalisation rules,
  per-item commit semantics, override-log format) but Open Questions shows only a
  dash.

- 🔵 **Dependency**: Live per-item remote read at list time adds an external API
  coupling with no availability note
  **Location**: Requirements
  The `/list-work-items` extension makes a previously local-only command depend on
  a reachable, rate-limited remote; the degradation path (e.g. fall back to
  synced/unsynced when unreachable) isn't captured.

- 🔵 **Dependency**: Conflict-resolution UX coupling to 0047's confirmation-prompt
  style not in Dependencies
  **Location**: Technical Notes
  The `blocked_by: 0047` edge actually covers two reuses — the status-slot seam
  and the confirmation-prompt precedent — but only the seam is explained in
  Dependencies.

- 🔵 **Scope**: `/list-work-items` rendering extension is a separable concern
  bundled into the sync-skill story
  **Location**: Requirements
  The extension targets a different skill surface and could be specified,
  delivered, and rolled back as a follow-on; the single-source-of-derivation
  justification is a code-reuse argument, not evidence of an indivisible unit.

- 🔵 **Scope**: Story is large even before the folded-in extension
  **Location**: Acceptance Criteria
  The core sync deliverable spans four modes, bidirectional reconciliation,
  conflict UX, numeric-ID batch push, untracked-issue pulling with filters/`--all`,
  persistence, and resumability — eleven sync-only ACs, several added beyond the
  epic.

- 🔵 **Testability**: Override criterion's "choice is logged" lacks an observable
  destination
  **Location**: Acceptance Criteria
  No statement of where/in what form the override is logged (stdout,
  `last-sync.json`, audit record), so the logging half can't be checked.

- 🔵 **Testability**: Filter-narrowing requirement defers the exact accepted flags
  **Location**: Requirements
  Filters are "the same options as the integration's `search-*` skills (e.g.
  assignee, label, state)"; with only "e.g." examples there's no closed list to
  verify, leaving integration-specific filters unconfirmed.

- 🔵 **Testability**: Numeric-ID push criterion bundles four behaviours into one
  check
  **Location**: Acceptance Criteria
  Per-item offer, batch offer, accepted-and-rewritten, declined-unchanged are one
  bullet; the batch path has no separate scenario and could go untested.

#### Suggestions

- 🔵 **Clarity**: Restate (or cross-reference) the numeric vs remote-format
  `work_item_id` rule, which lives only in 0047
  **Location**: Requirements
  A reader of 0051 alone must infer the boundary (e.g. how a Trello `AbCd1234` or
  GitHub `owner/repo#42` classifies).

- 🔵 **Completeness**: Add a Size/effort line as sibling 0047 carries
  **Location**: Technical Notes
  The capstone story has no size estimate, conspicuous against 0047's explicit
  "Size: M".

### Strengths

- ✅ The scope boundary with sibling 0047 is explicit and reciprocal — 0051
  produces `last-sync.json` and completes the three baseline-dependent states;
  0047 ships synced/unsynced and leaves the status-slot seam; both documents tell
  the same story with no gap or overlap.
- ✅ The three baseline-dependent label criteria each specify the exact
  local/remote-vs-baseline state that triggers them, and the five-state
  uniqueness invariant is a precise, mechanically checkable constraint.
- ✅ Actors are named consistently ("the user", "/sync-work-items"); the
  conflict-resolution policy (remote-default, explicit confirmation,
  override-to-push) is stated identically across Summary, Requirements, and ACs.
- ✅ The `--preview` criterion is strongly testable — it names every change
  category and a definite negative outcome (no local writes, no remote writes,
  `last-sync.json` not updated).
- ✅ Structurally complete and densely populated; mode flags, numeric-ID push,
  and `--all` are coherent variations on the single sync verb; out-of-scope items
  (multi-system mirroring, SHA diffing, three-way merge) are explicitly named.
- ✅ The acyclic-dependency reasoning is sound — because 0051 is `blocked_by`
  0047, placing the baseline-dependent rendering downstream (here) rather than in
  0047 correctly avoids a cycle.

### Recommended Changes

1. **Reconcile `blocked_by` with the "at least one integration" intent**
   (addresses: blocked_by-vs-prose contradiction) — Model the OR-relationship:
   drop 0048/0049/0050 from the hard `blocked_by` (Jira, already complete,
   satisfies the "any one integration" gate) and keep the prose note that any
   additional integration extends coverage, or — if all three are genuinely
   required — change the prose, Drafting Notes, and epic to match. Make the
   frontmatter and prose express one condition.

2. **Specify conflict handling under the directional modes** (addresses:
   conflict-mode unspecified) — State what a conflict item does in `--push-only`
   and `--pull-only` (e.g. reported and skipped, not prompted), and add/extend the
   directional ACs to cover the conflict case.

3. **Pin the three core sync verification procedures** (addresses: resumability,
   conflict-diff default, content-equivalence) — Rewrite the resumability AC as a
   concrete interrupt-and-resume scenario with explicit per-item invariants; give
   "remote is the default" an observable signal (e.g. the confirmation prompt's
   default answer accepts remote); and add at least one equivalence AC anchoring
   what counts as unchanged (e.g. trailing-whitespace + remote-managed `updated_at`
   differences are not a conflict).

4. **Record the 0047 data relationship in `Blocks`** (addresses: empty Blocks) —
   Add a non-blocking note mirroring 0047's wording: 0047's three deferred states
   consume the `last-sync.json` baseline 0051 produces, intentionally not modelled
   as a blocking edge to avoid a cycle.

5. **Fix the legacy path example and the remote-availability note** (addresses:
   guard-banned path, live-remote-read coupling) — Change the Technical Notes
   example to `.accelerator/state/integrations/jira/last-sync.json` (or a
   placeholder), and add an Assumption that the baseline-dependent `/list-work-items`
   states degrade to synced/unsynced when the remote is unreachable.

6. **Tidy the smaller testability and completeness gaps** (addresses: override
   log destination, filter flags, numeric-ID bundling, Open Questions, size) —
   Name the override-log target; restate the filter AC as a parity check; split
   the numeric-ID push AC into per-item and batch scenarios; surface the deferred
   decisions in Open Questions; and add a Size line.

7. **Decide whether to keep the `/list-work-items` extension in this story**
   (addresses: bundled separable concern, story-size) — Either keep it with a
   one-line Summary justification for why both surfaces must ship together, or
   split the three-state rendering into its own small story `blocked_by` 0051
   (mirroring how 0047 deferred it). The single-source-of-derivation argument
   supports keeping it together; the call is yours.

## Per-Lens Results

### Clarity

**Summary**: 0051 is generally clear and well-structured: actors are named, the
five-state model is explicitly cross-referenced against 0047, and the scope seam
between the two stories is carefully narrated in both Requirements and Drafting
Notes. The main weaknesses are (1) a contradiction between the frontmatter
`blocked_by` and the Dependencies prose, (2) unspecified conflict behaviour under
`--push-only`/`--pull-only`, and (3) a path inconsistency where Technical Notes
uses the legacy guard-banned `meta/integrations/` path.

**Strengths**:
- The scope boundary with 0047 is stated unambiguously and consistently in both
  directions.
- Actors are consistently named — no passive-voice ambiguity over who writes or
  confirms.
- The conflict-resolution policy is stated identically across Summary,
  Requirements, and ACs.

**Findings**:
- 🟡 major (high): **Frontmatter `blocked_by` contradicts Dependencies prose on
  integration blockers** (Dependencies) — all-of vs at-least-one; scheduler
  cannot tell when 0051 unblocks.
- 🟡 major (high): **Conflict-item behaviour under `--push-only`/`--pull-only`
  unspecified** (Requirements) — conflict branch has no mode qualifier; directional
  ACs silent.
- 🔵 minor (high): **Technical Notes uses the legacy guard-banned integrations
  path** (Technical Notes) — example contradicts "configured integrations path".
- 🔵 suggestion (medium): **"remote-format" vs "numeric" rely on 0047's definition
  without restating it** (Requirements).

### Completeness

**Summary**: 0051 is exceptionally complete — every expected section is present
and densely populated, frontmatter carries all required fields with a recognised
kind, and the scope boundary with 0047 is explicit and cross-referenced. Story
needs (motivation, the served system, a rich testable criteria set) are
satisfied. The only gaps are an empty Open Questions that understates real
unresolved decisions, and the absence of a size estimate that sibling 0047
carries.

**Strengths**:
- All structural sections present and substantive.
- Fifteen Given/When/Then ACs covering each mode and edge case plus the three
  list-work-items states.
- The scope boundary with 0047 is explicit and reciprocal.
- Frontmatter complete and coherent.
- Assumptions meaningfully scope the work.

**Findings**:
- 🔵 minor (medium): **Open Questions empty despite unresolved decisions noted
  elsewhere** (Open Questions).
- 🔵 suggestion (low): **No size/effort estimate, unlike sibling 0047** (Technical
  Notes).

### Dependency

**Summary**: 0051's upstream blockers are mostly well-captured — 0046 and 0047
appear in frontmatter and prose, and the 0047 status-slot-seam consumption is
documented. But there is a material inconsistency between the hard `blocked_by`
edges (all of 0048/0049/0050) and the prose (only one integration required), and
the downstream consumer represented by 0047's deferred states is missing from
0051's empty `Blocks` field.

**Strengths**:
- The upstream dependency on 0047's status-slot seam is captured as both a
  frontmatter edge and a Technical Note.
- The external remote-tracker coupling is named in Context and Requirements, and
  reuses the integrations' existing `search-*` skills.
- Ordering against 0046 and the one-active-integration constraint are captured,
  plus the `last-sync.json` write-ordering semantics.

**Findings**:
- 🟡 major (high): **`blocked_by` hard-blocks on all three integration stories,
  contradicting the "at least one" prose** (Dependencies).
- 🟡 major (high): **Empty `Blocks` field omits 0047's deferred states, which
  consume 0051's `last-sync.json`** (Dependencies).
- 🔵 minor (medium): **Live per-item remote read adds an external API coupling
  with no availability note** (Requirements).
- 🔵 minor (medium): **Conflict-resolution UX coupling to 0047's confirmation
  style not in Dependencies** (Technical Notes).

### Scope

**Summary**: 0051 describes the `/sync-work-items` skill plus a folded-in
`/list-work-items` rendering extension. The sync skill is a large but coherent
capstone. The rendering extension is a second, separable deliverable targeting a
different skill surface; the bundling justification (single-source state
derivation) is a code-reuse argument rather than evidence of an indivisible unit.
The story is also notably large even setting the extension aside.

**Strengths**:
- The bundling decision is explicitly reasoned (the fold-in from 0047), not
  silent scope creep.
- Mode flags, numeric-ID push, and `--all` are coherent variations on one verb.
- Out-of-scope items are explicitly named.
- The acyclic-dependency reasoning for placing the rendering downstream is sound.

**Findings**:
- 🔵 minor (medium): **`/list-work-items` rendering extension is a separable
  concern bundled into the sync-skill story** (Requirements).
- 🔵 minor (low): **Story is large for a single story even before the folded-in
  extension** (Acceptance Criteria).

### Testability

**Summary**: The ACs are largely well-formed Given/When/Then pairs and the
five-state label criteria are precisely verifiable. But several criteria lean on
terms with no concrete verification procedure — the conflict diff's "highlighted
as default", the resumability "safely" guarantee, and the normalised-equivalence
comparison that drives every push/pull decision. The override-log and
content-equivalence behaviours are under-specified for a tester.

**Strengths**:
- Most criteria have concrete preconditions and definite outcomes (file
  written/not, `last-sync.json` updated/not).
- The three baseline-dependent label criteria each specify their exact trigger
  state.
- The five-state uniqueness invariant is mechanically checkable.
- The `--preview` criterion names every change category and a definite negative
  outcome.

**Findings**:
- 🟡 major (high): **Resumability "resumes safely" has no measurable pass/fail
  procedure** (Acceptance Criteria).
- 🟡 major (medium): **Conflict-diff "highlighted as default" under-specifies the
  verifiable signal** (Acceptance Criteria).
- 🟡 major (medium): **Normalised content-equivalence drives every sync decision
  but has no testable definition** (Assumptions).
- 🔵 minor (medium): **Override criterion's "choice is logged" lacks an observable
  destination** (Acceptance Criteria).
- 🔵 minor (medium): **Filter-narrowing requirement defers the exact accepted
  flags** (Requirements).
- 🔵 minor (low): **Numeric-ID push criterion bundles four behaviours into one
  un-segmented check** (Acceptance Criteria).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** REVISE

Five of the six Pass 1 majors are fully resolved, and every Pass 1 minor and
suggestion is cleared. The verdict remains REVISE on three majors that cluster
tightly: a single substantive design question (the change-detection /
content-equivalence mechanism is described two incompatible ways and its
normalisation rule is deferred), plus a missing acceptance criterion for the
graceful-degradation behaviour added during the Pass 1 fixes. The story is much
closer to ready — the residual work is narrow.

### Previously Identified Issues

#### Major (5 of 6 resolved)

- 🟡 → ✅ **Clarity / Dependency**: `blocked_by` vs prose contradiction —
  **Resolved.** Frontmatter is now `[0046, 0047]`; the dependency lens confirms
  frontmatter and prose match, with Jira satisfying the integration requirement
  and 0048-0050 non-blocking. Cited as a strength.
- 🟡 → ✅ **Clarity**: conflict behaviour under `--push-only`/`--pull-only`
  unspecified — **Resolved.** Requirements and a dedicated AC now define
  report-and-skip in directional modes.
- 🟡 → ✅ **Dependency**: empty `Blocks` omits 0047 data relationship —
  **Resolved.** The non-blocking `last-sync.json` relationship and cycle-avoidance
  rationale are now recorded; cited as a strength.
- 🟡 → ✅ **Testability**: resumability not measurable — **Resolved.** The
  concrete interrupt-after-A-before-B scenario is now cited as a strength.
- 🟡 → ✅ **Testability**: conflict-diff "default" unobservable — **Resolved.**
  The AC now pins the default answer accepting remote; cited as a strength.
- 🟡 → 🟡 **Testability**: content-equivalence undefined — **Partially resolved.**
  An equivalence AC was added (trailing-whitespace + remote-managed fields →
  unchanged), but the underlying normalisation rule remains deferred to the plan,
  so the AC still has no definitive pass/fail oracle, and the clarity lens
  surfaces a related tension (see New Issues). Still major.

#### Minor / Suggestion (all resolved)

- 🔵 → ✅ **Clarity**: legacy guard-banned path — **Resolved** (example now
  `.accelerator/state/integrations/...`).
- 🔵 → ✅ **Completeness**: Open Questions empty — **Resolved** (three deferred
  decisions listed).
- 🔵 → ✅ **Dependency**: live-remote-read availability note — **Resolved**
  (graceful-degradation Assumption added).
- 🔵 → ✅ **Dependency**: conflict-UX coupling to 0047 not in Dependencies —
  **Resolved** (now named in the `blocked_by: 0047` rationale).
- 🔵 → 🔵 **Scope**: extension bundled — **Resolved as a finding;** the lens now
  judges the bundling coherent given the stated single-source-derivation
  rationale and raises only optional suggestions.
- 🔵 → ✅ **Scope**: story large — **Resolved** (explicit L size now declared and
  justified).
- 🔵 → ✅ **Testability**: override log destination — **Resolved** (sync summary +
  `work_item_id`/direction).
- 🔵 → ✅ **Testability**: filter flags deferred — **Resolved** (restated as a
  parity check; a residual oracle nuance remains, see New Issues).
- 🔵 → ✅ **Testability**: numeric-ID push bundled — **Resolved** (split into
  per-item and batch ACs).
- 🔵 → ✅ **Clarity / Completeness** suggestions (cross-ref rule, size) —
  **Resolved.**

### New Issues Introduced

All cluster on the change-detection mechanism or are easy AC additions.

- 🔴 **Clarity (major)**: Two conflicting "has it changed?" mechanisms —
  timestamp-based (Assumptions/Requirements: file mtime + remote `updated_at` vs
  `last-sync.json`) and content-normalised (the new equivalence AC) — are
  described without stating which is authoritative or how they compose. The core
  reconciliation logic has two divergent interpretations.
- 🟡 **Testability (major)**: The normalisation-tolerance AC can't produce a
  definitive pass/fail because the ignored-field set and whitespace policy are
  deferred to the plan — the example is illustrative, not exhaustive.
- 🟡 **Testability (major)**: The graceful-degradation behaviour added to
  Assumptions (remote unreachable → show only synced/unsynced, don't fail) has no
  acceptance criterion. Also flagged by completeness (minor).
- 🔵 **Clarity (minor)**: Given the `last-sync.json` schema stores only a per-item
  *remote* timestamp, how the *local* side is compared "against last-sync.json"
  is underspecified (which field — the global `timestamp`?).
- 🔵 **Clarity (minor/suggestion)**: "default to remote; require explicit user
  confirmation" reads as two competing defaults in the Requirement (the AC
  resolves it); `--all` is "pull everything" in Requirements but "bypass project
  filter" in the AC.
- 🔵 **Testability (minor)**: filter parity "identically to that skill" and the
  numeric-ID remote-key rewrite each need a concrete oracle; the default-mode
  happy-path AC bundles four outcomes under one unspecified fixture.
- 🔵 **Dependency (minor/suggestion)**: the integration `search-*` read-skill
  reuse is an Assumption, not a captured dependency; clarify the 0047 edge is hard
  (status-slot seam) vs the UX-consistency preference.

### Assessment

Substantial progress — the dependency graph, conflict-mode behaviour,
resumability, and conflict-resolution UX are all now sound, and the scope
bundling reads as coherent. The remaining REVISE rests on essentially one
decision and one omission:

1. **Define the change-detection contract** — state whether the authoritative
   "changed?" signal is timestamp-based, content-normalised, or a timestamp
   pre-filter confirmed by a normalised-content comparison; pin a minimum
   equivalence rule (ignored fields + whitespace policy) rather than deferring it
   wholesale; and clarify what the local side compares against in the
   `last-sync.json` schema. This resolves the clarity major and the normalisation
   testability major together.
2. **Add the graceful-degradation AC** — the remote-unreachable path the
   Assumptions promise needs a criterion (trivial; resolves the second
   testability major and the completeness minor).

The remaining minors are polish. Pinning the change-detection contract is the one
genuine design decision left before this story is plan-ready.

## Re-Review (Pass 3) — 2026-06-15

**Verdict:** COMMENT

The change-detection contract was defined (timestamp pre-filter + authoritative
normalised-content comparison, with the pre-filter able only to short-circuit to
*unchanged*), a minimum normalisation rule was pinned in Assumptions, the
`last-sync.json` schema was extended with a per-item `local_hash` baseline, and a
graceful-degradation AC was added. Both remaining Pass 2 majors are resolved, and
no new majors were introduced. **Zero major findings across all five lenses;
completeness returned no findings at all.** The story is plan-ready.

### Previously Identified Issues

- 🔴 → ✅ **Clarity**: two conflicting change-detection mechanisms — **Resolved.**
  The lens confirms the mechanism is now described one consistent way across the
  Change-detection contract, Requirements, and the schema, and that the contract
  explicitly states the reconciling invariant (pre-filter may only short-circuit
  to *unchanged*). A residual minor: the Summary's "timestamp-based" shorthand
  understates the now-authoritative content comparison.
- 🟡 → ✅ **Testability**: normalisation AC had no definitive oracle —
  **Resolved.** The pinned minimum normalisation rule (per-line whitespace
  trimming + ignored remote-managed/non-local-schema fields) gives the
  equivalence AC a deterministic pass/fail oracle.
- 🟡 → ✅ **Testability**: graceful-degradation behaviour had no AC — **Resolved.**
  A dedicated AC now covers remote-read failure (every item still renders
  synced/unsynced; clean exit; no hang). The completeness lens, which also
  flagged it, now returns no findings.
- 🔵 → ✅ **Clarity (minor)**: local-side comparison vs schema — **Resolved** by
  the `local_hash` baseline field, though the lens suggests stating the local
  side is re-normalised-and-re-hashed for symmetry with the remote side.
- 🔵 → ✅ **Clarity (minor)**: `--all` "everything" vs "bypass project filter" —
  **Resolved** (now: bypasses only the project scope; user filters still apply).
- 🔵 → ✅ **Clarity (minor)**: "default to remote; require confirmation" wording —
  **Resolved** (prompt default answer is remote; no write without explicit
  confirmation), with a residual nit on whether accepting the default *is* the
  confirming action.
- 🔵 → ✅ **Testability (minor)**: filter-parity and numeric-rewrite oracles —
  **Resolved** (parity restated as set-equality vs the `search-*` skill; rewrite
  tied to the create operation's returned key, no longer `^[0-9]+$`).
- 🔵 → ✅ **Completeness (suggestion)**: untracked-creation AC — **Resolved** (a
  dedicated criterion was added).
- 🔵 → ✅ **Dependency (minor/suggestion)**: search-* reuse + hard-vs-soft 0047
  edge — **Resolved** (read/search capability and the hard seam vs consistency
  tie are now stated).

### New Issues Introduced

All minor or suggestion; none block planning.

- 🔵 **Clarity**: the Summary's "timestamp-based" line understates the contract —
  reword to mention the authoritative content comparison.
- 🔵 **Clarity**: name `local_hash` as a digest used purely for two-way equality
  (distinct from the out-of-scope SHA three-way merge); state the local side is
  re-normalised and re-hashed for symmetry with the remote side.
- 🔵 **Clarity**: clarify whether accepting the conflict prompt's default answer
  is itself the confirming action that authorises the local write.
- 🔵 **Dependency**: frame the filter-flag coupling to each integration's
  `search-*` skill as a maintenance coupling; optionally add a "related (coverage
  extension): 0048/0049/0050" note.
- 🔵 **Scope**: ensure the implementing plan treats the `/list-work-items`
  extension as a separately-landable slice so a sync-engine stall doesn't block
  the lower-risk rendering change.
- 🔵 **Testability**: filter-parity verification needs a fixed remote fixture +
  recorded expected set; enumerate the five states inline in the distinctness AC;
  the conflict AC's "labelled as default choice" clause is presentational (the
  functional default-answer assertion already covers it).

### Assessment

0051 is plan-ready. The change-detection contract is now coherent and bounded,
the dependency graph is acyclic and fully captured, the scope bundling reads as a
justified single unit, and every acceptance criterion has a determinable oracle.
The residual findings are all optional polish — most cheaply, rewording the
Summary's change-detection line and glossing `local_hash`. None require another
review pass before `/create-plan`.

## Approval — 2026-06-15

**Verdict elevated to APPROVE** by the reviewer. The two cheapest Pass 3
suggestions (Summary change-detection wording and the `local_hash` gloss) were
applied after the pass; the remaining suggestions are optional polish. The work
item is accepted for implementation planning.
