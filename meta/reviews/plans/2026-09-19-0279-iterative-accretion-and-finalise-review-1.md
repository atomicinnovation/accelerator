---
type: "plan-review"
id: "2026-09-19-0279-iterative-accretion-and-finalise-review-1"
title: "Plan Review: Iterative Accretion and Finalise"
date: "2026-09-20T12:59:22+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-19-0279-iterative-accretion-and-finalise"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["correctness", "architecture", "test-coverage", "code-quality", "safety", "standards", "compatibility", "usability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-20T14:45:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Iterative Accretion and Finalise

**Verdict:** REVISE

The design is sound and the reviewers are near-unanimous on its strengths:
disk-derived counters, staleness-as-lifecycle-position, content-first /
manifest-last crash discipline, a minimal single-file blast radius, and a
contract that 0278 fully provisioned (compatibility verified every "no change
required" claim against the actual files). The plan does not need
re-architecting. What it needs is a specification pass: because the deliverable
is `SKILL.md` prose executed by a model, every ambiguity or un-patched
contradiction in that prose is a latent execution bug, and the review surfaced
several — a surviving hardcoded `round: 1`, a reopen rule that contradicts a
retained sentence, the `finalise` gate specified twice with conflicting message
conventions, and an undefined breadth ceiling once rounds accrete. Twelve major
findings (no critical) cluster into a handful of concrete edits; fixing them is
mechanical and the plan is otherwise ready.

### Cross-Cutting Themes

- **The `finalise` gate is specified twice with conflicting refusal wording**
  (flagged by: code-quality, standards, usability) — Phase 5 §1 adds `finalise
  requires synthesised` to the Shared Preamble, governed by the generic
  "name the expected prior state" refusal, while Phase 5 §3 mandates a refusal
  that names the *current* status (AC9). One verb, two contradictory refusal
  conventions in one file.
- **The multi-round golden proves shape, not count correctness — the marquee
  0279 logic has no automated guard** (flagged by: test-coverage, architecture,
  safety) — the validator presence-checks extras only, so a fixture whose
  `round_count` disagrees with its findings validates clean. Test-coverage and
  architecture independently propose the same cheap fix: a deterministic Rust
  cross-check in `frontmatter_goldens.rs`.
- **`round_count` derivation is undefined at the edges** (flagged by:
  correctness, architecture, code-quality) — "highest `round` stamped on any
  finding on disk" is a maximum over a possibly-empty set (all researchers fail
  → zero findings), and it does not say whether quarantined `.invalid` markers
  (which still carry a `round:` stamp) count.
- **The iterative loop is invisible on the skill surface and silent at runtime**
  (flagged by: usability) — the reopen of a finished set is unannounced, the
  append-vs-revise branch gives no feedback, and the `description`/`argument-hint`
  still read as a linear pipeline rather than a growable, closable, reopenable
  loop.

### Tradeoff Analysis

- **Test-first rigour vs the no-harness reality**: the plan makes an explicit,
  honest carve-out from the repo's test-first rule because the executor is a
  model running prose with no eval harness. Reviewers accept the carve-out but
  converge on two mitigations that do not require a harness: (1) a deterministic
  Rust cross-check that locks the *fixture's* counts against drift (encoding the
  derivation rule as executable documentation), and (2) tracking the deferred
  eval suite as a real work item sequenced before the next `conduct`-touching
  slice. Neither closes the behavioural gap; both raise the floor cheaply.
- **`finalise` fail-safe simplicity vs freshness certainty** (safety): the
  single-condition gate (`status == synthesised`) is elegant, but because
  `finalise` alone is not reconcile-first, a mid-`conduct` crash under a
  still-`synthesised` manifest lets `finalise` certify a stale, count-frozen set
  as `complete`. A corroborating freshness check (reject if any finding's `round`
  exceeds `synthesis.md`'s `rounds_covered`) buys certainty at the cost of the
  gate's one-line simplicity. Recommend accepting the extra check — the research
  already names `rounds_covered` as exactly this corroborating signal.

### Findings

#### Critical

None.

#### Major

- 🟡 **Code Quality**: conduct's hardcoded `round: 1` survives the real-round-injection change
  **Location**: Phase 2, Change 3 (SKILL.md:159-164)
  Phase 2 Change 2 injects the real round N, but Change 3's diff stops at "never
  the raw focus-area count" and leaves the next sentence (`SKILL.md:162-164`,
  "Each finding carries … `round: 1` …") unpatched. The finished file would tell
  the model both to inject round N and that every finding carries `round: 1`.

- 🟡 **Correctness**: `round_count` derivation and outline's conducted-check don't exclude quarantined invalid findings
  **Location**: Phase 2 Change 3 & Phase 3 Change 2
  A `.<nn>-<slug>.md.invalid` quarantine marker still carries a readable
  `round: N` stamp. A round that produced only a quarantined finding would raise
  `round_count` and make `outline` append `## Round N+1` past a round that
  yielded no valid research. `finding_count` is carefully qualified "retained,
  validated"; the round derivations are not.

- 🟡 **Code Quality**: reopen regression is added as prose that contradicts Phase 3's retained "leave status unchanged" sentence
  **Location**: Phase 3 Change 2 & Phase 5 Change 2
  Phase 3 writes "otherwise leave `status` unchanged". Phase 5 adds reopen
  (synthesised/complete → researching) as free prose with no diff revising that
  sentence. Applied literally, the file carries both, so the model gets
  contradictory instructions for a synthesised/complete input and the reopen
  (AC8) may not fire.

- 🟡 **Code Quality / Standards / Usability**: `finalise` refusal is duplicated with conflicting message wording
  **Location**: Phase 5 Change 1 (Shared Preamble) & Change 3 (finalise body)
  The preamble's generic refusal names the "expected prior state"; the finalise
  body names the "current status" (AC9). The verb is governed by two conflicting
  conventions — a model could emit the generic message and violate AC9, or emit
  both.

- 🟡 **Code Quality**: breadth-8 ceiling is ambiguous once rounds can be appended (per-round vs total)
  **Location**: Phase 3 Change 2 & Change 3
  The append prose keeps "the breadth ceiling of 8" while Change 3 leaves the
  "at most eight focus areas, ever" framing at `SKILL.md:48` unchanged. With N
  appended rounds it is undefined whether 8 is per-round (set grows to 8×N) or a
  hard total — undefined at the exact point accretion is introduced.

- 🟡 **Safety**: `finalise` is not reconcile-first, so a mid-conduct crash can certify a stale, count-frozen set as `complete`
  **Location**: Phase 5 Change 2 & Change 3
  The crash-safety argument relies on `conduct`'s reconcile-first repair, but
  `finalise` is a pure status flip checking only `status == synthesised`. Sequence:
  synthesised set → `conduct` crashes after findings land but before its final
  manifest edit → `finalise` passes the gate on the stale `synthesised` status
  and stamps `complete` with frozen counts and an uncovered finding. Recoverable
  (findings survive on disk) but silently violates "synthesised implies a current
  dossier".

- 🟡 **Test Coverage / Architecture / Safety**: the multi-round golden proves shape, not count correctness — no automated guard for the aggregate-root invariants
  **Location**: Phase 1 Change 2; Testing Strategy; Key Discoveries
  The validator presence-checks extras only, so the fixture would validate clean
  even if `round_count` were `5` against two findings. The subtlest, highest-value
  0279 logic (count derivation, gap-fill vs new round) has zero automated coverage
  and the fixture that anchors it does not assert its own counts.

- 🟡 **Test Coverage**: Phase 1's automated verification can pass vacuously
  **Location**: Phase 1 Change 2; Success Criteria (Automated)
  `cargo test <filter>` exits 0 when the filter matches zero tests, so a typo, a
  missing test, or a differently-named test all yield green. The plan compounds
  this: the new test name appears only in the command, while the prose says only
  "a test mirroring `the_committed_topic_research_set_validates_clean`" (the
  existing single-round test).

- 🟡 **Test Coverage / Safety**: the deferred eval suite is the sole future behavioural guard but is untracked, and the next slices amend `conduct`
  **Location**: What We're NOT Doing; Testing Strategy (final paragraph)
  All behavioural correctness is verified once, manually, during 0279; thereafter
  no automated net until a `research-topic` eval suite lands "separately" — with
  no tracking work-item ID. 0282 and 0283 both amend `conduct` next, so the verb
  most likely to change soon has no behavioural guard.

- 🟡 **Usability**: reopen of a synthesised/complete set is silent — the user is not told the subject reopened
  **Location**: Phase 5 Change 2
  A plain `outline SLUG` / `conduct SLUG` on a finished set silently drops it to
  `researching`, and because `primary` stays on `synthesis.md`, the reader's
  landing document is now a stale dossier with no indication it is stale. The
  status regression is the sole staleness signal, yet it is invisible when it
  happens.

- 🟡 **Usability**: `finalise` refusal names the current status but not the recovery path
  **Location**: Phase 5 Change 3
  The refusal tells the user where the set is but not what to do next — nothing
  points them to running `synthesise` first to reach `synthesised`.

- 🟡 **Usability**: the skill surface still reads as a linear pipeline, not a growable/closable/reopenable loop
  **Location**: Phase 5 Change 4 (description / argument-hint)
  The frontmatter enumerates five verbs flatly. From the surface alone a user
  cannot tell that `outline`/`conduct` repeat to grow a subject, that running
  them on a finished set reopens it, or that `finalise` requires a prior
  `synthesise` — the entire point of 0279 is invisible on the listed surface.

#### Minor

- 🔵 **Architecture / Correctness / Code Quality**: `round_count` derivation is undefined when a conduct lands zero findings
  **Location**: Phase 2 Change 3
  "Highest `round` on disk" is a maximum over a possibly-empty set (all
  researchers fail → no findings), yet `status` still advances to `researching`.
  0277 floored this at `round_count: 1`; the derivation removes the floor without
  specifying the empty-set result.

- 🔵 **Test Coverage**: Phase 2 and Phase 4 success criteria omit the AC7 status transitions they are mapped to
  **Location**: Phase 2 & Phase 4 Manual Verification; phase-to-criterion table
  The table maps AC7's `conduct`→`researching` and `synthesise`→`synthesised` to
  Phases 2 and 4, but neither phase's manual verification asserts the resulting
  `status`. Phases 3 and 5 do assert theirs.

- 🔵 **Test Coverage**: manual checks lean on undefined ad-hoc scratch sets and a non-reproducible live-web run
  **Location**: Testing Strategy; Phase 2 & Phase 5 Manual Verification
  "A pre-seeded scratch set" is invoked without a defined seed state, and the
  immutability guarantee (AC3) is only exercisable via a non-deterministic
  attended live-web `conduct`. Anchoring web-free checks to the committed
  multi-round fixture and confirming "mutates nothing" via a clean VCS diff would
  make them repeatable.

- 🔵 **Safety**: in-place manifest edits are non-atomic; a torn write during the final edit is uncovered
  **Location**: Validate every write; Phase 5
  The content-first / manifest-last argument protects state *before* the final
  edit, but the manifest edits themselves are direct in-place YAML (not brief's
  temp-then-rename). A crash *during* the edit can leave torn YAML as the
  aggregate root the indexer keys on. 0279 adds more such edits (finalise,
  reopen) run more often.

- 🔵 **Safety**: iterative accretion widens the exposure of the deferred write-scope assertion
  **Location**: What We're NOT Doing; Deferred hardening
  The sole in-tool detector of a stray subagent write is deferred; the refuse-to-
  overwrite guard acts only at index allocation, not on where the subagent writes.
  0279 makes `conduct` re-runnable across many rounds, multiplying the runs during
  which this detector is missing. Deferring is defensible for a VCS-recoverable
  tool, but the attended-only constraint should stay prominent.

- 🔵 **Correctness**: a no-op `conduct` still regresses a synthesised set to researching
  **Location**: Phase 5 Change 2
  Reopen is unconditional, but a `conduct` on a fully-synthesised set with no
  outstanding areas spawns nothing and adds nothing — yet still un-synthesises the
  dossier, contradicting the work item's framing that reopen marks staleness
  *because* something was added. Matches AC8/AC10 as written; confirm it is
  intended.

- 🔵 **Correctness**: crash recovery is argued for conduct's reopen window but not outline's
  **Location**: Phase 5 Change 2
  `outline` appends `## Round N+1` before its final status edit, so a crash
  between leaves an appended pending round under a still-`synthesised` manifest,
  and `outline` has no reconcile-first step. It is in fact self-healing (the next
  `outline` revises N+1 in place), but the asymmetry could mislead an implementer.

- 🔵 **Code Quality**: synthesise reconciling `round_count` adds a second writer of a conduct-owned field
  **Location**: Phase 4 Change 2
  Phase 4 has `synthesise` reconcile `round_count`/`finding_count`, but the design
  invariant is that only `conduct` advances `round_count`, and the shared "Validate
  every write" section mentions only `finding_count`. Two verbs now derive
  `round_count` — a DRY/drift risk.

- 🔵 **Code Quality**: "revise in place" wording is awkward for the first outline where nothing exists yet
  **Location**: Phase 3 Change 2
  On a `briefed` set there is no `outline.md` and no `## Round N`, so "the highest
  `## Round N`" is undefined and "revise … in place" describes editing a checklist
  that does not exist. Splitting the first-outline case out would remove the
  inference.

- 🔵 **Usability**: the append-vs-revise outline outcome is unannounced and hinges on invisible disk state
  **Location**: Phase 3 Change 2
  The same command either appends a new round or revises the highest one in place,
  branching on disk state the user cannot see, with no output telling them which
  fired. Reporting "appended Round 3" vs "revised Round 2 in place" makes it
  observable.

- 🔵 **Usability**: precondition refusals still name a single "expected prior state" after widening
  **Location**: Phases 2-4 (Shared Preamble refusal wording)
  After widening, `conduct` accepts four states, but the preamble refusal is left
  as "naming the expected prior state" (singular), under-describing the contract
  and leaving the recovery path incomplete.

- 🔵 **Compatibility**: the newly-written `complete` status has no positive validation golden
  **Location**: Phase 1; What We're NOT Doing
  0279 is the first slice to write `status: complete` to a real manifest, but the
  corpus golden suite pins a positive test only for `briefed` (and the new fixture
  is `synthesised`). The value's acceptance is pinned on the frontend side only,
  not the Rust validator side.

- 🔵 **Standards**: golden test name breaks the established helper/fixture naming word order
  **Location**: Phase 1 Change 2; Automated Verification
  The helper is `topic_research_multiround_set()` and the fixture dir is
  `topic-research-multiround-set` ("multiround" after "topic-research"), but the
  pinned test name is `the_committed_multiround_topic_research_set_validates_clean`
  ("multiround" moved to the front). Rename to
  `the_committed_topic_research_multiround_set_validates_clean`.

- 🔵 **Standards**: allowed-tools justification omits the `metadata derive` command finalise uses
  **Location**: Phase 5 Change 4
  §4 justifies "no new allowed-tools" by citing `corpus resolve` and
  `frontmatter validate`, but §3 also derives metadata via `corpus metadata
  derive`. The conclusion holds (all three are already granted); the rationale
  under-enumerates.

#### Suggestions

- 🔵 **Architecture**: the 0279×0283 conduct seam is unexamined
  **Location**: What We're NOT Doing; Implementation Approach
  The design accommodates 0282/0283 additively, but the load-bearing hinge for
  0283 — that every finding, at any future recursion depth, carries a `round`
  stamp so max-round derivation still holds — is left implicit. A sentence in
  What We're NOT Doing asserting that invariant would de-risk whichever slice
  lands second.

- 🔵 **Architecture**: "independently mergeable" slightly overstates the phase relationship
  **Location**: Implementation Approach
  Phases 2-5 all edit the same Shared Preamble block, each on top of the prior
  phase's text, and the ordering constraints force a fixed sequence. "Sequentially
  mergeable in a fixed order, each leaving a consistent state" matches the two
  documented constraints and removes the implication of order-independence.

- 🔵 **Compatibility**: Phase 4 narrows (rather than widens) the synthesise precondition
  **Location**: Phase 4 Change 1
  Adding a status gate to 0277's findings-only precondition removes previously-
  accepted inputs (a findings-bearing `complete` set). The practical break surface
  is essentially nil, but stating that the narrowing is intentional — and that a
  `complete` set must be reopened before re-synthesis — avoids it reading as a
  regression.

- 🔵 **Standards**: `last_updated` advancement is specified only for finalise, not the other in-place edits
  **Location**: Phase 5 Change 2 & Phases 2-4
  The finalise section explicitly advances `last_updated`/`last_updated_by`, but
  the reopen and widened edits do not mention it, even though "Populate
  frontmatter" already governs all writes. Treat provenance refresh uniformly or
  confirm the existing convention covers it and drop the finalise-only mention.

### Strengths

- ✅ The frontmatter contract genuinely admits everything 0279 writes:
  compatibility verified the five-state `status_vocab`, the `round` and
  `rounds_covered` extras, the templates, the frontend chip map and colour
  tokens, and the drift fixture — every "no schema/Rust/template/vocab change"
  claim holds against the actual files.
- ✅ The "derived-from-disk, never stored" principle is applied consistently to
  `round_count`, `finding_count`, and staleness, so gap-fill-vs-new-round and
  crash recovery fall out without stateful bookkeeping, and no dual source of
  truth is reintroduced.
- ✅ Content-first / manifest-last discipline is preserved and correctly extended:
  folding reopen into the verb's final manifest edit keeps a crash-before-commit
  leaving a prior-consistent state.
- ✅ `finalise` and reopen add no new CLI mutator, no new spawn point, and reuse
  only read-only tools already granted, so the permission, dispatch-coherence,
  and bare-invocation lints stay green and the trust boundary is not widened.
- ✅ The five-state machine has no dead or unreachable states; the append-vs-revise
  branch keys on finding-existence (defending against a checkbox that lies after
  a reopen) and covers both states of the highest round.
- ✅ The Testing Strategy draws its automated/manual boundary with rare honesty —
  it states outright that the validator is presence-only and that a count
  disagreeing with disk validates clean, and correctly declines a
  count-mismatch negative test that would pass misleadingly.
- ✅ The Phase-3-to-4 "non-corrupting intermediate" is correctly argued: a
  premature `rounds_covered: 1` mislabels reader-facing provenance only; it loses
  no finding and freezes no authoritative count.
- ✅ The sibling multi-round fixture is the right backward-compatible call —
  growing the existing set in place would disturb the ~9 negative-case goldens
  that string-replace its exact single-round shape.
- ✅ Domain language is rich and matches the epic vocabulary end to end
  (accretion, gap-fill, round, focus area, dossier, reopen, aggregate root).

### Recommended Changes

1. **Patch every prose contradiction so the file is internally consistent**
   (addresses: conduct's hardcoded `round: 1` survives; reopen contradicts Phase
   3's "leave status unchanged"; finalise refusal duplicated; breadth-8 per-round
   vs total).
   These are the highest-leverage fixes — each is a latent execution bug in a
   model-run file. Extend the Phase 2 Change 3 diff to rewrite the "Each finding
   carries … `round: 1`" sentence to the injected round. Give Phase 5 an explicit
   diff that rewrites the Phase 3 status sentence into the full mapping
   (briefed→outlined; outlined/researching unchanged; synthesised/complete→
   researching). Reconcile the finalise gate to a single authority (see #2). State
   the breadth ceiling explicitly (per-round or whole-set) and update the "ever"
   wording at `SKILL.md:48` to match.

2. **Make the `finalise` gate a single authority with one refusal convention**
   (addresses: finalise duplicated with conflicting wording; finalise refusal no
   recovery path; precondition refusals name a single expected state).
   Either exempt `finalise` from the generic preamble refusal or drop it from the
   preamble list and let the finalise body be the sole gate. Make the message name
   both the current status and the recovery verb (e.g. "status is researching;
   finalise requires synthesised — run `synthesise SLUG` first"). While there,
   generalise the preamble refusal to enumerate all accepted prior states for the
   widened verbs.

3. **Define the `round_count` derivation totally** (addresses: undefined for
   zero-finding case; quarantined invalids not excluded).
   At both derivation sites, qualify the count as "retained, validated findings
   (the visible `<nn>-*.md` files, excluding `.invalid` markers)" — mirroring
   `finding_count` — and specify the empty-set result (e.g. "unchanged; the
   brief-time default 0 when no finding is on disk").

4. **Add a deterministic Rust cross-check for the fixture's counts** (addresses:
   golden proves shape not count correctness; Phase 1 verification passes
   vacuously; no positive `complete` golden).
   In `frontmatter_goldens.rs`, parse the multi-round fixture frontmatter and
   assert `round_count == max(round across findings)` and `finding_count ==
   number of finding files`. State the exact test name once in the Changes
   Required prose and rename it to
   `the_committed_topic_research_multiround_set_validates_clean`. Add a one-line
   positive golden that flips a fixture manifest to `status: "complete"` and
   asserts clean validation. Rely on the whole-suite run plus a visual test-count
   increase, not a name-filtered invocation that passes on zero matches.

5. **Close the `finalise` freshness gap** (addresses: finalise not reconcile-first
   can certify a stale set complete).
   Give `finalise` a corroborating check before flipping to `complete`: refuse if
   any finding's `round` exceeds `synthesis.md`'s `rounds_covered` (or reconcile
   disk counts and reject a mismatch), so a crash-frozen stale set cannot be
   certified complete.

6. **Make the loop discoverable and its outcomes observable** (addresses: silent
   reopen; append-vs-revise unannounced; surface reads as a pipeline).
   Have `outline`/`conduct` emit an explicit notice when they regress a finished
   set ("this set was complete; adding a round reopened it — re-run synthesise to
   refresh"), and report which append-vs-revise branch fired. Extend the
   `description` to convey the growth-and-closure loop, and add a manual-verification
   check that the reopen is announced.

7. **Track the deferred eval suite and tighten the manual checks** (addresses:
   untracked eval suite; Phase 2/4 omit AC7 status checks; manual checks lean on
   undefined seeds).
   Capture the deferred `research-topic` eval suite as an explicit work item,
   referenced from this plan and sequenced to land before or with the next
   `conduct`-touching slice. Add explicit "manifest `status` becomes researching /
   synthesised" bullets to Phase 2 and Phase 4. Anchor web-free manual checks to
   the committed multi-round fixture and confirm "mutates nothing" via a clean VCS
   diff.

8. **Address the remaining minors and suggestions as spec polish** (addresses:
   non-atomic manifest edits; deferred write-scope exposure; no-op conduct
   regression; outline crash window; synthesise as second round_count writer;
   first-outline wording; allowed-tools rationale; 0283 seam invariant;
   "independently mergeable" wording; Phase 4 narrowing note; uniform
   `last_updated`).
   None blocks the slice; each is a one-line clarification. Note the non-atomic
   in-place edit window (or adopt brief's temp-then-rename), keep the attended-only
   constraint prominent, decide whether a no-op conduct should reopen, note
   outline's window is self-healing, scope `round_count` ownership to `conduct`,
   split out the first-outline case, list all three reused commands in the
   allowed-tools rationale, assert the every-finding-carries-a-round invariant for
   0283, reword "independently mergeable" to "sequentially mergeable in a fixed
   order", state the Phase 4 narrowing is intentional, and treat provenance
   refresh uniformly.

## Per-Lens Results

### Correctness

**Summary**: The core design — deriving `round_count`/`finding_count` as
projections of disk rather than blind counters, and folding reopen-regression
into the content-first/manifest-last final edit — is logically sound and makes
gap-fill vs new-round detection and crash recovery fall out correctly without
stateful bookkeeping. The append-vs-revise branch has complete case coverage
and the state machine has no dead or unreachable states. Two gaps remain: the
count/round derivations do not say whether quarantined `.invalid` findings are
excluded, and the "highest round on disk" derivation is undefined when zero
findings exist.

**Strengths**:
- Counts derived from disk, not incremented, so gap-fill vs new-round and crash
  recovery are correct by construction.
- Append-vs-revise keys on finding-existence, not the checkbox, defending against
  a checkbox that lies after a reopen or hand-edit; the two branches jointly cover
  both states of the highest round.
- Reopen folded into the final manifest edit; a mid-conduct crash is provably
  absorbed by conduct's reconcile-first step and self-heals across a verb switch.
- Five-state machine has no dead or unreachable states; the single finalise gate
  covers both an absent and a stale synthesis.
- Single-model sequential executor means no genuine concurrency/TOCTOU exposure.

**Findings**:
- 🔴 (major, medium) *round_count derivation and outline's conducted-check don't
  exclude quarantined invalid findings* — Phase 2 Change 3 & Phase 3 Change 2. A
  `.invalid` marker still carries a `round:` stamp; a round yielding only a
  quarantined finding would raise `round_count` and drive a spurious append.
  Qualify both derivations "retained, validated" as `finding_count` already is.
- 🔵 (minor, medium) *round_count derivation undefined when a conduct lands zero
  findings* — Phase 2 Change 3. Maximum over an empty set when all researchers
  fail; `status` still advances to `researching`. Specify the empty-set answer.
- 🔵 (minor, low) *a no-op conduct still regresses a synthesised set to
  researching* — Phase 5 Change 2. Unconditional reopen un-synthesises a dossier
  even when nothing was added, contradicting the work item's "adds findings or
  focus areas" framing. Confirm intended.
- 🔵 (minor, low) *crash recovery argued for conduct's reopen window but not
  outline's* — Phase 5 Change 2. Outline's append-then-manifest window is in fact
  self-healing; the plan should say so to avoid misleading the implementer.

### Architecture

**Summary**: Architecturally sound — preserves the skill-mutates / CLI-validates
trust boundary, applies "derived-from-disk, never stored" consistently to the
counters and to staleness, and keeps the content-first / manifest-last aggregate-
root discipline through every new transition (including a crash-safe folding of
reopen). The two ordering constraints are correctly identified and justified, and
the contract-first/behaviour-second split gives a minimal single-file blast radius
that accommodates 0282/0283 additively. The one systemic exposure — candidly
acknowledged — is that all aggregate-root invariants now live in unverifiable
prose with a presence-only validator.

**Strengths**:
- Trust boundary preserved: no new CLI mutator, no new spawn point, read-only
  tools only.
- "Derived-from-disk, never stored" applied consistently to round_count,
  finding_count, and staleness; eliminates the dual source of truth the 0278
  collapse removed.
- Aggregate-root discipline preserved through every new transition; reopen folded
  into the final edit is crash-safe.
- The two ordering constraints correctly reasoned; the one tolerated intermediate
  correctly classified as reader-facing provenance, not data loss.
- Finalise's single-condition gate makes the wrong thing structurally hard;
  `complete` stays cleanly reopenable.
- Domain language matches the epic vocabulary end to end.

**Findings**:
- 🟡 (major, high) *all aggregate-root invariants enforced only in model prose;
  the validator is presence-only* — Key Discoveries / Testing Strategy. 0279
  concentrates more invariant-bearing logic into the untestable layer. The plan
  justifies the tradeoff appropriately (need not block), but the durable home for
  these invariants is the validator (a numeric cross-check) or the deferred eval
  suite.
- 🔵 (minor, medium) *round_count projection undefined for the zero-finding case*
  — Phase 2 Change 3. Reachable when every researcher fails. Specify the empty-set
  result so the rule is total.
- 🔵 (suggestion, medium) *the 0279×0283 conduct seam is unexamined* — the
  load-bearing hinge is that every future recursive finding still carries a round
  stamp; assert it in What We're NOT Doing.
- 🔵 (suggestion, low) *"independently mergeable" overstates the phase
  relationship* — phases share the same preamble block and must merge in a fixed
  order; reword to "sequentially mergeable in a fixed order".

### Test Coverage

**Summary**: Unusually honest about its automated/manual split — the Testing
Strategy states outright that the validator is presence-only, that a count
disagreeing with disk validates clean, and that reopen/finalise/count-derivation
have no automated coverage. Pre-seeding scratch sets to make most behavioural
checks web-free is the right isolation move. The residual risk is under-mitigated
in two cheap-to-fix ways: the sole Phase 1 guard is a single static positive
golden whose counts are unasserted, and the subtle count-derivation logic plus
reopen/finalise rest entirely on non-reproducible manual runs with no tracked
follow-up suite.

**Strengths**:
- The ⚠️ Testing Strategy callout draws the automated/manual boundary with rare
  honesty and correctly declines a count-mismatch negative test that would pass
  misleadingly.
- Isolating conduct's live-web coupling and pre-seeding scratch sets maximises the
  deterministic manual surface.
- Manual steps are largely anchored to specific observable signals, not vague
  "works correctly".
- Phase ordering justified on data-integrity grounds — sound risk-based
  sequencing given weak automated guards.

**Findings**:
- 🔴 (major, high) *the highest-risk behaviour (count derivation) has no automated
  guard and the fixture's counts are unasserted* — Phase 1 Change 2. Convert the
  manual cross-check into a deterministic Rust assertion parsing the fixture and
  asserting `round_count == max(round)` and `finding_count == file count`.
- 🔴 (major, high) *Phase 1's automated verification cannot fail if the golden is
  missing or misnamed* — `cargo test <filter>` exits 0 on zero matches, and the
  new test name appears only in the command. State the exact name once in prose;
  rely on the whole-suite run.
- 🔴 (major, medium) *the deferred eval suite is the sole future guard but is
  untracked, and the next slices amend conduct* — capture it as a tracked work
  item sequenced before the next conduct-touching slice.
- 🔵 (minor, high) *Phase 2 and Phase 4 success criteria omit the AC7 status
  transitions they are mapped to* — add explicit "status becomes researching /
  synthesised" bullets.
- 🔵 (minor, medium) *manual checks lean on ad-hoc scratch sets and a
  non-reproducible live-web run; immutability is only attended-web-verifiable* —
  anchor web-free checks to the committed fixture; confirm "mutates nothing" via a
  clean VCS diff; isolate a reconcile-only run to prove byte-identical immutability
  deterministically.

### Code Quality

**Summary**: Well-structured, with sound phase ordering and honest reuse of the
review-adr/create-adr patterns. From a maintainability-of-the-prose standpoint,
several diffs are internally inconsistent with the text they patch or with each
other: a hardcoded `round: 1` survives, the outline status-write logic and the
Phase 5 reopen are competing statements with no reconciling diff, and the finalise
refusal is duplicated with conflicting wording. Two derivation edge cases
(breadth-8 per-round vs total, round_count with zero findings) are left undefined
enough to make model execution unreliable.

**Strengths**:
- Phase ordering justified against a concrete corruption risk; the one tolerated
  intermediate is called out explicitly.
- New behaviour modelled on existing named patterns, keeping the prose idiomatic.
- Finalise reuses existing allowed-tools — a clean minimal surface.
- The finalise refusal signal and the reopen crash-safety framing match the ACs.
- The Phase 1 Rust snippet carries no comments, respecting the comment policy.

**Findings**:
- 🔴 (major, high) *conduct's hardcoded `round: 1` survives the real-round-injection
  change* — `SKILL.md:162-164` is never patched, so the file contradicts itself.
- 🟡 (major, high) *reopen regression is added as prose that contradicts Phase 3's
  retained "leave status unchanged" sentence* — give Phase 5 an explicit diff that
  rewrites the Phase 3 sentence to the full status mapping.
- 🟡 (major, medium) *finalise refusal duplicated with conflicting message wording*
  — reconcile to a single gate/convention.
- 🟡 (major, medium) *breadth-8 ceiling ambiguous once rounds can be appended
  (per-round vs total)* — state the semantics and fix the "ever" wording at
  `SKILL.md:48`.
- 🔵 (minor, medium) *round_count derivation undefined when no findings are on disk*
  — specify the empty-disk fallback.
- 🔵 (minor, low) *synthesise reconciling round_count adds a second writer of a
  conduct-owned field* — scope synthesise to `rounds_covered`, or lift the
  reconciliation rule into "Validate every write".
- 🔵 (minor, low) *"revise in place" wording is awkward for the first outline* —
  split the first-outline case out explicitly.

### Safety

**Summary**: A low-criticality, developer-facing skill whose entire state lives in
a VCS-tracked tree, so the blast radius is bounded and recovery is fast. The
data-safety story is genuinely strong: immutable findings, refuse-to-overwrite,
quarantine-not-delete, content-first/manifest-last, and disk-derived counts all
preserved and extended, with phased ordering that avoids corrupting intermediates.
The one real gap: the crash-safety claim leans on conduct's reconcile-first repair,
but finalise is deliberately not reconcile-first, so a mid-conduct crash under a
still-synthesised manifest followed by finalise can certify a stale, count-frozen
set as complete.

**Strengths**:
- Finding immutability preserved end to end; all state VCS-recoverable.
- Content-first / manifest-last preserved and extended; reopen folded into the
  final edit.
- Counts are projections of disk; Phase 4 additionally reconciles them.
- finalise fails safe on a non-synthesised set.
- The Phase-3-to-4 non-corrupting intermediate is correctly argued.
- Accepted risks documented transparently rather than left implicit.

**Findings**:
- 🔴 (major, medium) *finalise is not reconcile-first, so a mid-conduct crash can
  certify a stale, count-frozen set as complete* — give finalise a corroborating
  freshness gate (reject if any finding's `round` exceeds `rounds_covered`).
- 🔵 (minor, medium) *in-place manifest edits are non-atomic; a torn write during
  the final edit is uncovered* — note the window or adopt brief's
  temp-then-rename.
- 🔵 (minor, medium) *iterative accretion widens the exposure of the deferred
  write-scope assertion* — keep the attended-only constraint prominent; prioritise
  the assertion before any hosted/unattended use.
- 🔵 (minor, medium) *the data-safety invariants have no automated regression net;
  the multi-round golden exercises no verb* — prioritise the deferred eval suite
  and consider a committed quarantine-marker negative fixture.

### Standards

**Summary**: Largely convention-consistent. The five-verb framing edits mirror
existing formats, finalise reuses only granted tools (so the bare-invocation,
skill-permissions, dispatch-coherence, and skill-invocation lints stay green as
claimed), and the finalise section is placed and shaped consistently with the
cited ADR patterns while correctly adapting them to topic-research's single-
frontmatter-status convention. A handful of internal naming/spec inconsistencies
should be tightened.

**Strengths**:
- Five-verb framing edits follow existing formats; the long argument-hint is
  consistent with other skills' hints.
- The claim that the convention lints stay green holds under inspection (no new
  `!`-site, no new fenced invocation, no new dispatched token).
- finalise correctly reuses the three read-only corpus commands already granted.
- The finalise section preserves verb sequencing and heading conventions and does
  not copy the ADR dual-status body line.
- The added Rust carries no comments, stays within 80 cols, and its crate/path/
  invocation all agree.

**Findings**:
- 🔵 (minor, high) *golden test name breaks the established helper/fixture naming
  word order* — rename to
  `the_committed_topic_research_multiround_set_validates_clean`.
- 🔵 (minor, medium) *finalise carries two conflicting refusal-message conventions*
  — reconcile the preamble and the finalise body to one authority.
- 🔵 (minor, high) *allowed-tools justification omits the metadata-derive command
  finalise uses* — list all three reused commands in the §4 rationale.
- 🔵 (suggestion, low) *last_updated advancement specified only for finalise, not
  the other in-place manifest edits* — treat provenance refresh uniformly.

### Compatibility

**Summary**: A strongly backward-compatible, additive slice. The central claim —
the contract already admits every state and field 0279 writes, so no Rust/schema/
template/vocab change is needed — holds under direct verification of all cited
files. The one new enum value written to live data (`status: complete`) was fully
provisioned by 0278 across every consumer checked (Rust vocab, drift fixture,
frontend chip map, colour tokens, lifecycle-chip test). The only gaps are minor
and test-side.

**Strengths**:
- Verified the five-state `status_vocab`, the `round` and `rounds_covered` extras,
  the templates, the status-vocab JSON fixture, and its drift pin — every "no
  change" claim is correct.
- Verified forward-compat for `complete` across the frontend variant map, colour
  tokens, and the lifecycle-chip test.
- Verified extras are presence-checked only and no consumer numerically compares
  the counts, so the intermediate `rounds_covered: 1` breaks nothing.
- Verified the sibling-fixture decision protects the ~9 negative-case goldens.
- Verified Migration Notes: every 0277 leave-state falls within the widened
  preconditions; all 0277 findings are `round: 1`, consistent with the derivation.
- Verified the new verb and reopen edges are additive; no consumer pins the
  four-verb surface.

**Findings**:
- 🔵 (minor, medium) *the newly-written `complete` status has no positive
  validation golden* — add a one-line golden that flips a fixture manifest to
  `status: "complete"` and asserts clean validation.
- 🔵 (suggestion, medium) *Phase 4 narrows rather than widens the synthesise
  precondition* — state the narrowing is intentional and that a `complete` set
  must be reopened before re-synthesis.

### Usability

**Summary**: The plan extends a five-verb surface consistently — finalise reuses
the existing slug resolution and read-only tools, and the per-verb description
gloss is preserved — but the iterative loop and reopen semantics are
under-communicated to the user. The most significant DX gaps: a silent reopen of a
finished subject with a now-stale dossier still fronted by `primary`, a finalise
refusal that names only the current status with no recovery path, and a
description that still reads as a linear pipeline. Smaller: append-vs-revise gives
no feedback, the widened preconditions' refusal still names a single state, and
finalise's precondition is specced twice.

**Strengths**:
- finalise reuses the Shared Preamble resolution and existing read-only tools,
  adding no new flags — the surface grows minimally.
- The description gloss pattern is extended consistently.
- Widening synthesise to permit an idempotent re-run is a forgiving, low-surprise
  choice.
- The single finalise gate is conceptually simple and easy to explain.

**Findings**:
- 🟡 (major, high) *reopen of a synthesised/complete set is silent* — emit a notice
  when a finished set is reopened; add a manual-verification check that it is
  announced.
- 🟡 (major, high) *finalise refusal names the current status but not the recovery
  path* — name the current status, the required prior state, and the recovery verb.
- 🟡 (major, medium) *the skill surface still reads as a linear pipeline* — extend
  the description to convey the growable/closable/reopenable loop.
- 🔵 (minor, medium) *append-vs-revise outcome is unannounced and hinges on
  invisible disk state* — report which branch fired.
- 🔵 (minor, medium) *precondition refusals still name a single expected prior
  state after widening* — enumerate the full accepted set.
- 🔵 (minor, medium) *finalise precondition specced twice with divergent
  conventions* — choose one gate and one message convention.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-20

**Verdict:** REVISE

The revision resolved all 12 major findings and every minor and suggestion from
the initial review — verified lens by lens. It introduced three new major
findings, each a quick fix: a `reconcile` wording collision that would invert the
new `finalise` freshness gate, and two test-coverage gaps where the count
cross-check under-delivers on what the plan claims (a degenerate fixture, and
unencoded edge arms). Three new majors meet the REVISE threshold, so the verdict
holds — but the plan is one short edit pass from APPROVE.

### Previously Identified Issues

- 🟡 **Code Quality**: conduct's hardcoded `round: 1` survives — Resolved (Phase 2 §3 diff now rewrites the sentence to the injected round).
- 🟡 **Correctness**: round_count / append-check don't exclude quarantined invalids — Resolved (both sites now exclude `.invalid` markers).
- 🟡 **Code Quality**: reopen contradicts Phase 3's "leave status unchanged" sentence — Resolved (Phase 5 §2 rewrites that exact sentence via diff).
- 🟡 **Code Quality / Standards / Usability**: finalise refusal duplicated with conflicting wording — Resolved (finalise section is the single authority; preamble exempts it).
- 🟡 **Code Quality**: breadth-8 per-round vs total ambiguity — Resolved (stated per-round; `SKILL.md:48` "ever" rewritten).
- 🟡 **Safety**: finalise not reconcile-first → crash certifies stale complete — Resolved (freshness gate added; closes both crash cases).
- 🟡 **Test Coverage / Architecture / Safety**: golden proves shape not counts — Resolved in structure (cross-check added), but see New Issues on the cross-check's adequacy.
- 🟡 **Test Coverage**: Phase 1 verification passes vacuously — Resolved (whole-suite run; confirm test count rose by three).
- 🟡 **Test Coverage / Safety**: deferred eval suite untracked — Partially resolved (plan now calls for a dedicated work item, but the item is not yet created; re-review downgraded it to minor).
- 🟡 **Usability**: silent reopen — Resolved (explicit reopen notice added).
- 🟡 **Usability**: finalise refusal no recovery path — Resolved (names current status and recovery verb).
- 🟡 **Usability**: skill surface reads as a linear pipeline — Resolved (description conveys the loop).
- 🔵 **Minors and suggestions (18)** — All resolved, with one standing exception: the deferred `conduct` write-scope assertion (Safety) remains an accepted, restated deferral, re-flagged as minor because 0279 widens its exposure.

### New Issues Introduced

- 🟡 **Correctness**: the finalise gate says "reconcile counts against disk", but `reconcile` means write-to-repair everywhere else in the file (`SKILL.md:122`, `:216`, Phase 4). Under that meaning finalise would repair-then-complete and the gate would never fire — the exact failure it was added to prevent. Fix: compare-only wording that forbids writing.
- 🟡 **Test Coverage**: the count cross-check fixture is degenerate — `round_count`, `finding_count`, and `rounds_covered` all equal `2` — so it cannot distinguish a `max(round)` derivation from a file count. Fix: reshape the fixture (e.g. Round 1 with two findings, Round 2 with one → `finding_count: 3`, `round_count: 2`).
- 🟡 **Test Coverage**: the derivation rule's tricky arms (quarantine exclusion, gap-fill, empty→0) are encoded nowhere, so "encodes the count rule as executable documentation" overstates the guard. Fix: add a sibling fixture with a `.invalid` marker stamped at a higher round, or narrow the claim.
- 🔵 **Architecture**: the count-derivation rule is now restated across `conduct`/`synthesise`/`finalise` plus the cross-check with no canonical home; the cross-check encodes a simplified (terminal-state) version; the 0283 seam covers the round stamp but not the flat `<nn>-*.md` layout the scan assumes.
- 🔵 **Usability**: the canned recovery guidance ("re-run synthesise") misleads on the outline-append path (needs `conduct`) and the never-synthesised path (`synthesise` itself refuses); the freshness-gate refusal carries no recovery guidance; and no manual-verification bullet confirms the new reopen notice / append-vs-revise report is emitted.
- 🔵 **Code Quality**: §1's precondition list ("finalise requires synthesised") understates the actual gate, since §3's freshness gate can refuse a synthesised set.
- 🔵 **Safety**: the freshness gate's `finding_count` reconcile is ambiguous (all visible `<nn>-*.md` vs validated-only), leaving a narrow window where an un-quarantined invalid finding slips past; and iterative accretion widens the deferred write-scope exposure.
- 🔵 **Compatibility**: the Phase 4 synthesise narrowing changes behaviour in a narrow mid-`conduct`-crash window (manifest still `outlined` with a finding on disk) — benign and self-healing.
- 🔵 **Standards**: the two other new Phase 1 tests (the count cross-check and the `complete` golden) are left unnamed against the file's descriptive-sentence convention.
- 🔵 **Correctness**: a `finalise` run in the mid-`outline` crash window closes a set with a dangling, un-conducted pending round — benign and recoverable; confirm `finalise` asserts currency over findings, not exhaustion of planned rounds.

### Assessment

The plan is in good shape and close to ready. Every originally-identified issue
is addressed. The three new majors are self-inflicted by the revision and each is
a small, well-specified edit: change the `reconcile` wording to compare-only,
reshape the multi-round fixture so its three counts diverge, and add one
`.invalid`-marker sibling fixture (or narrow the claim). With those plus the
minor recovery-guidance, `finding_count`-scope, and test-naming polish, the plan
reaches APPROVE. No structural or design concern remains.

## Re-Review (Pass 3, confirmation) — 2026-09-20

**Verdict:** COMMENT

Scoped confirmation pass on the two lenses that carried the pass-2 majors
(correctness, test-coverage). All three pass-2 majors are confirmed resolved:
the `finalise` freshness gate is now unambiguously a read-only compare that
cannot repair-then-complete and fires correctly on a crash-frozen set without
false-refusing a fresh one; the multi-round fixture is no longer degenerate
(`finding_count: 3` vs `round_count: 2`), so the cross-check genuinely
distinguishes a `max(round)` derivation from a file count; and the
quarantine-exclusion arm is now statically pinned by a dedicated `.invalid`-marker
fixture. No new major appeared. Two minors surfaced and were fixed in the same
pass; the verdict moves from REVISE to COMMENT — the plan is implementation-ready.

### Previously Identified Issues (pass-2 majors)

- 🟡 **Correctness**: `reconcile` wording would invert the freshness gate — Resolved (Phase 5 §3 is now a read-only compare that writes nothing; verified it still fires on a stale set and does not false-refuse a fresh one).
- 🟡 **Test Coverage**: degenerate cross-check fixture — Resolved (fixture reshaped to `finding_count: 3`, `round_count: 2`; a file-count derivation would now fail the assertion).
- 🟡 **Test Coverage**: tricky derivation arms unencoded — Resolved (a `.invalid`-marker sibling fixture pins the quarantine-exclusion arm; the claim is narrowed; gap-fill/empty-set correctly left to manual).

### New Issues Introduced (all fixed this pass)

- 🔵 **Correctness**: Phase 2 §4's canonical-rule statement grouped `finalise` with the write-to-repair verbs, in tension with its read-only gate — Fixed (§4 now distinguishes `finalise`'s read-only compare-before-edit from `conduct`/`synthesise`'s write-on-edit).
- 🔵 **Test Coverage**: the AC6 manual check still read "both findings" after the fixture grew to three, skipping the cross-round `03-third-focus` — Fixed (now "all three findings").
- 🔵 **Test Coverage** (suggestion): the two cross-checks each re-derive `max(round)` — Applied (plan now factors a shared `highest_round(dir, exclude_invalid)` helper).

### Assessment

The plan is implementation-ready. Every actionable finding from all three passes
is addressed. Two items remain open by choice, not oversight: the deferred
`research-topic` eval-suite work (owned by 0161, a catalogue-wide hardening that
lands after the topic-research epic completes and deliberately gates no slice)
and the accepted deferral of the `conduct` write-scope assertion (compensated by
attended-only runs and human commit review). Neither blocks implementing 0279.
Verdict COMMENT: acceptable as-is, with those two accepted tradeoffs recorded.
