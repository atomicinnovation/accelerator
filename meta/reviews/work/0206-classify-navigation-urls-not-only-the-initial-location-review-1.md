---
type: "work-item-review"
id: "0206-classify-navigation-urls-not-only-the-initial-location-review-1"
title: "Work Item Review: Classify Navigation URLs, Not Only The Initial Location"
date: "2026-08-31T17:19:16+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0206"
work_item_id: "0206"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-31T20:16:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Classify Navigation URLs, Not Only The Initial Location

**Verdict:** COMMENT

This story is ready to implement. Every section is present and substantively
populated, scope is a single coherent guarantee with explicit boundaries, and
the acceptance criteria are unusually verifiable. The findings are polish: one
major note about coordinating with sibling 0209 on the shared daemon surface,
and a cluster of minor wording refinements on the acceptance criteria and two
overloaded terms.

### Cross-Cutting Themes

- **AC6 (doc-wording criterion) is under-specified** (flagged by: testability,
  clarity) — testability finds no defined pass condition ("replaced" is
  tautologically passable), and clarity finds the front-door/boundary
  distinction that motivates the change relies on outside context. Both resolve
  with the same edit: name the exact phrases that must disappear and state that
  "boundary" means the check now applies to every navigation.

### Findings

#### Critical

None.

#### Major

- 🟡 **Dependency**: Coordination/ordering coupling with 0209 captured only as loose 'relates to'
  **Location**: Dependencies
  Both 0206 and 0209 modify the same long-lived daemon's navigation surface and
  both plumb new per-request information through the Rust executor. Recorded only
  as "Relates to: 0209" with no ordering note, so concurrent scheduling collides
  on `lib/daemon.js`'s navigate handler and the executor's per-request args.

#### Minor

- 🔵 **Dependency**: 'Blocked by: none' rationale omits the executor/daemon integration surface it extends
  **Location**: Dependencies
  The readiness rationale names only the three delivered domain functions of
  0196, but the surface this work extends is `Command::Executor` (which "has no
  allowance flags today") and the daemon's per-request channel — a later 0196
  phase, whose delivery a scheduler cannot verify from the section alone.

- 🔵 **Testability**: First criterion asserts an implementation path rather than a purely observable outcome
  **Location**: Acceptance Criteria
  AC1's "same code path `validate-source` uses" is an implementation assertion
  confirmable only by reading source; the paired "refuses what that would
  refuse" clause carries the verifiable content.

- 🔵 **Testability**: Documentation-wording criterion lacks a defined pass condition
  **Location**: Acceptance Criteria
  AC6 says advisory wording is "replaced" but not what the replacement must say
  or which phrases must no longer be present, so any edit could be argued to
  satisfy it.

- 🔵 **Clarity**: same_origin: false is overloaded to mean two distinct things
  **Location**: Requirements
  Repurposing `same_origin: false` as a reachability-refusal skip conflates a
  genuinely cross-origin destination with a refused internal one, so a `links`
  consumer cannot tell the two skip reasons apart from the field alone.

- 🔵 **Clarity**: --allow-insecure-scheme scope exceeds the Summary's reachability framing
  **Location**: Requirements
  The Summary frames the change around reachability, but Requirements introduce
  `--allow-insecure-scheme` alongside `--allow-internal` without explaining what
  an insecure scheme is here or whether scheme classification is enforced per
  navigation; only `--allow-internal` is tested.

#### Suggestions

- 🔵 **Scope**: Two enforcement surfaces (navigate and links) are technically separable slices within one story
  **Location**: Requirements
  `navigate` (error envelope) and `links` (`same_origin: false` skip) plus
  redirect interception are cohesive under the single guarantee but technically
  deliverable independently. Keep together; the `links` classification is the
  most naturally separable slice if delivery pressure ever forces a split.

- 🔵 **Testability**: Refusal input set that defines pass/fail is only in Context, not in the criterion
  **Location**: Acceptance Criteria
  The encodings that give "refuses what that would refuse" its teeth (decimal,
  hex, octal, IPv6-transition, IPv4-mapped) live in Context, so a tester reading
  only the criteria could under-test the refusal surface.

- 🔵 **Clarity**: 'front door' vs 'boundary' distinction relies on outside context
  **Location**: Acceptance Criteria
  AC6's "since the check is then a boundary" contrast is developed in the
  referenced migration plan but only implied here.

### Strengths

- ✅ Every expected section is present and substantively populated, with
  frontmatter carrying all required fields at recognised values (kind: story,
  status: ready, priority: high).
- ✅ Scope is a single coherent guarantee — no navigation or followed link
  escapes the reachability policy — with explicit, defensible boundaries (DNS
  rebinding accepted as residual; cross-origin auth-header stripping deferred to
  0209; not folded into the parent migration).
- ✅ Acceptance criteria are outcome-oriented: the refused-navigate envelope
  shape (`retryable: false`, reach classification in `details`, URL never
  loaded), the per-request non-leakage scenario, and the named redirect test all
  admit a definitive pass/fail.
- ✅ Open Questions is resolved rather than dangling — both prior decisions
  (refusal shape; per-request vs daemon-lifetime allowances) are decided and
  cross-referenced from Requirements.
- ✅ Actors are named consistently (operator, driving agent, implementer) and
  obscure jargon is glossed at point of use (DNS rebinding defined inline).

### Recommended Changes

1. **Add a coordination note for 0209 in Dependencies** (addresses: Coordination/ordering coupling with 0209)
   Promote "Relates to: 0209" to an explicit statement that both stories extend
   a shared per-request-forwarding and route-handler seam — either sequence one
   after the other, or note that the second rebases onto the first rather than
   reinventing the plumbing.

2. **Give AC6 a concrete pass condition** (addresses: Documentation-wording criterion lacks a defined pass condition; 'front door' vs 'boundary' distinction relies on outside context)
   Name the phrases that must disappear (e.g. "front door, not a boundary" and
   equivalent advisory caveats in `host_reach`/`access_policy` docs and the
   design page) and state that "boundary" means the check now applies to every
   navigation, not just the entry point.

3. **Reframe AC1 behaviourally** (addresses: First criterion asserts an implementation path)
   Replace "same code path `validate-source` uses" with an observable
   equivalence over the refused input classes (decimal, hex, octal,
   IPv6-transition, IPv4-mapped encodings of private/link-local/reserved hosts).

4. **Note the executor/daemon surface in the 'Blocked by' rationale** (addresses: 'Blocked by: none' rationale omits the integration surface)
   Confirm `Command::Executor` and the daemon's per-request forwarding channel
   are delivered, not only the three pure domain functions.

5. **Clarify the two overloaded terms in Requirements** (addresses: same_origin: false is overloaded; --allow-insecure-scheme scope exceeds the framing)
   State that `same_origin: false` is deliberately overloaded (and whether any
   consumer must distinguish the skip reasons), and add one sentence on why
   `--allow-insecure-scheme` travels with `--allow-internal` and whether scheme
   classification is enforced per navigation or merely carried.

## Per-Lens Results

### Clarity

**Summary**: Unusually clear and internally consistent — actors named, problem
maps cleanly onto Requirements and Acceptance Criteria, specialised terms
defined inline. The few concerns concentrate on two overloaded/under-explained
pieces of vocabulary: the repurposed `same_origin: false` signal and the
`--allow-insecure-scheme` flag whose role is broader than the Summary's
reachability framing.

**Strengths**:
- The "front door" metaphor is explicitly defined in Context.
- Actors are consistently named (operator, driving agent, implementer).
- Obscure jargon defined at point of use (DNS rebinding glossed in Assumptions).
- Stated problem maps directly and completely onto Requirements and AC.

**Findings**:
- 🔵 minor (confidence: medium) — *same_origin: false is overloaded to mean two
  distinct things* (Requirements). Repurposing `same_origin: false` as a
  reachability-refusal skip conflates a genuinely cross-origin destination with
  a refused internal one; per AC4 even a same-origin internal endpoint is
  reported as `same_origin: true` suppressed. A `links` consumer cannot tell the
  two skip reasons apart from the field alone. Suggestion: state the overload
  explicitly and clarify whether any consumer needs to distinguish them.
- 🔵 minor (confidence: medium) — *--allow-insecure-scheme scope exceeds the
  Summary's reachability framing* (Requirements). The Summary frames the change
  around reachability, but Requirements introduce a second flag without
  explaining what an insecure scheme is here or whether scheme classification is
  enforced per navigation; only `--allow-internal` is tested. Suggestion: one
  sentence on why the flag travels with `--allow-internal` and whether scheme
  enforcement is in scope.
- 🔵 suggestion (confidence: low) — *'front door' vs 'boundary' distinction
  relies on outside context* (Acceptance Criteria). AC6's contrast is developed
  in the referenced migration plan but only implied here. Suggestion: add a
  half-sentence clarifying that "boundary" means the check applies to every
  navigation.

### Completeness

**Summary**: Exceptionally complete — every expected section is present and
substantively populated, and the frontmatter carries all required fields with
recognised values (kind: story, status: ready, priority: high). The story
identifies the actor, explains why the work is needed, and provides seven
concrete acceptance criteria. No structural or informational gaps.

**Strengths**:
- Summary uses a clear "As an operator... I want... so that..." form.
- Context explains the forces behind the work, not just a restatement.
- Acceptance Criteria has seven specific, outcome-oriented bullets.
- Open Questions resolved and cross-referenced from Requirements.
- Dependencies, Assumptions, Technical Notes all populated with relevant content.
- Frontmatter complete and internally consistent.

**Findings**: None.

### Dependency

**Summary**: Well-populated Dependencies section — names the blocking parent
(0196) with a "delivered" rationale, states no known downstream blocks, records
the relationship to 0209. The gaps: the 0209 coupling is recorded only as a
loose "relates to" despite both stories modifying the same daemon surface and
executor plumbing; and the "Blocked by: none" rationale enumerates only the
three domain functions while the actual integration surface is the executor and
daemon, whose delivery status is left implicit.

**Strengths**:
- Upstream blocker (0196) captured with an explicit readiness rationale.
- Residual coupling to 0209 named in both Dependencies and References.
- Out-of-scope residual couplings (DNS rebinding, pre-resolution limitation)
  stated explicitly in Assumptions.

**Findings**:
- 🟡 major (confidence: medium) — *Coordination/ordering coupling with 0209
  captured only as loose 'relates to'* (Dependencies). Both items modify the
  same long-lived daemon's navigation surface and both carry new per-request
  info through the Rust executor. Recorded only as "Relates to: 0209" with no
  ordering note; concurrent scheduling collides on `lib/daemon.js`'s navigate
  handler and the executor's per-request args, producing merge conflicts and
  duplicated interception logic. Suggestion: promote the link to an explicit
  coordination/ordering statement.
- 🔵 minor (confidence: medium) — *'Blocked by: none' rationale omits the
  executor/daemon integration surface it extends* (Dependencies). The rationale
  names only the three delivered domain pieces, but the surface this work
  extends is `Command::Executor` (no allowance flags today) and the daemon's
  per-request channel — a later 0196 phase. If the executor port is not merged,
  the work is blocked despite the section asserting no blockers. Suggestion:
  confirm the executor and per-request channel are delivered.

### Scope

**Summary**: Well-scoped, coherent story — every requirement serves the single
purpose of applying the existing reachability policy to per-request navigation
and link-following, closing the gap the parent epic deliberately left as a
follow-up. Boundaries are explicit (DNS rebinding, cross-origin auth-header
stripping deferred to 0209, migration-time coupling all out of scope), and the
"story" kind fits the sizing.

**Strengths**:
- All requirements serve one coherent guarantee rather than bundling concerns.
- Boundaries drawn explicitly and defensibly.
- Correctly resists false coupling — "deliberately not a migration-time change".
- Declared kind (story) matches the scope; domain functions already exist.

**Findings**:
- 🔵 suggestion (confidence: low) — *Two enforcement surfaces (navigate and
  links) are technically separable slices within one story* (Requirements).
  `navigate` (error envelope), `links` (`same_origin: false` skip), and redirect
  interception are cohesive under the single guarantee but independently
  deliverable. Impact: none currently — the guarantee is complete only when both
  surfaces are covered. Suggestion: keep as-is; if a split is ever forced, the
  `links` classification is the most naturally separable slice.

### Testability

**Summary**: Acceptance Criteria are unusually strong — most describe concrete,
observable outcomes (error envelope shape with `retryable: false`,
`same_origin: false` for refused destinations, per-request non-leakage, a named
redirect test) that admit a definitive pass/fail. Two criteria lean on
implementation phrasing or leave the pass condition underspecified, and the
concrete refusal input set lives in Context rather than the criterion itself.
None blocking.

**Strengths**:
- The refused-navigate AC pins the exact envelope shape and observable "URL
  never loaded" condition.
- The per-request allowance-scope criterion specifies a complete scenario.
- AC5 reads as a ready-made test spec (public→link-local redirect, internal
  request never issued).
- The `links` criterion is stated as an observable output contract.

**Findings**:
- 🔵 minor (confidence: medium) — *First criterion asserts an implementation path
  rather than a purely observable outcome* (Acceptance Criteria). AC1's "same
  code path `validate-source` uses" is confirmable only by reading source; the
  paired "refuses what that would refuse" clause is verifiable. Suggestion:
  reframe behaviourally over the refused input classes.
- 🔵 minor (confidence: medium) — *Documentation-wording criterion lacks a
  defined pass condition* (Acceptance Criteria). AC6 says wording is "replaced"
  but not what the replacement must say; any edit could be argued to satisfy it.
  Suggestion: state the concrete pass condition (phrases that must no longer
  appear; docs describe the check as an enforced boundary).
- 🔵 suggestion (confidence: low) — *Refusal input set that defines pass/fail is
  only in Context, not in the criterion* (Acceptance Criteria). The encodings
  enumerated in Context (decimal, hex, octal, IPv6-transition, IPv4-mapped) give
  "refuses what that would refuse" its teeth, but the criterion does not restate
  them, so a tester could under-test the refusal surface. Suggestion: add a
  representative refusal-input list to the criterion or AC5's test spec.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-31

**Verdict:** REVISE

Pass 1's findings were all addressed, but one of the edits — asserting in
Requirements that `--allow-insecure-scheme` is "enforced per navigation
alongside reachability, not merely carried" — introduced a scope/clarity/
testability inconsistency that three lenses independently flagged, two as major.
The Summary and every acceptance criterion still frame the work as
reachability-only, so the document now presents two pictures of what
"classified" covers. One decision (is per-navigation scheme enforcement in
scope?) plus a matching Summary/AC edit resolves all three; the remaining new
items are minor polish.

### Previously Identified Issues

- 🟡 **Dependency**: Coordination/ordering coupling with 0209 — Resolved. Now an
  explicit "Coordinate with" mutual-exclusion note.
- 🔵 **Dependency**: 'Blocked by: none' omits executor/daemon surface — Resolved.
  Rationale now names `Command::Executor` and the per-request channel.
- 🔵 **Testability**: AC1 asserts an implementation path — Resolved. Reframed as
  observable equivalence over the refused input classes.
- 🔵 **Testability**: AC6 lacks a defined pass condition — Partially resolved.
  First clause (phrases that must disappear) is now checkable; the added second
  clause is still a subjective reading (see new issues).
- 🔵 **Testability**: Refusal input set only in Context — Resolved. Encodings now
  restated in AC5's test spec.
- 🔵 **Clarity**: `same_origin: false` overloaded — Resolved. Overload now stated
  explicitly in Requirements.
- 🔵 **Clarity**: `--allow-insecure-scheme` scope exceeds framing — Not resolved;
  worsened. The edit asserted enforcement rather than clarifying scope, so the
  Summary/Requirements mismatch is now a major inconsistency (see new issues).
- 🔵 **Clarity**: front-door/boundary distinction — Resolved. AC6 now states
  "boundary" means applied to every navigation.
- 🔵 **Scope**: separable navigate/links slices — Not re-raised.

### New Issues Introduced

- 🟡 **Clarity** (major): Summary's "reachability policy" framing omits the scheme
  enforcement introduced in Requirements — the document presents two different
  pictures of what "classified" covers.
- 🟡 **Testability** (major): Scheme enforcement (`--allow-insecure-scheme` /
  plaintext `http`) is now stated as enforced per navigation but has no
  acceptance criterion, so it could be dropped or shipped broken and still pass.
- 🔵 **Scope** (suggestion): Per-navigation scheme enforcement widens scope beyond
  the reachability-framed Summary and ACs — either fold it in consistently or
  carve it into a follow-up.
- 🔵 **Testability** (minor): Reach-classification payload in the error envelope
  has no defined shape — AC2's "carry the reach classification" names no field or
  values to assert on.
- 🔵 **Testability** (minor): AC6's second clause ("describe the check as an
  enforced boundary") is a subjective reading with no checkable artefact.
- 🔵 **Testability** (suggestion): Positive allowance path (`--allow-internal`
  grants access) is not asserted — only the negative non-leakage case is.
- 🔵 **Dependency** (suggestion): 0209 coordination fixes non-concurrency but not
  which story runs first.
- 🔵 **Clarity** (suggestion): Actor shifts between "the operator" and "the agent
  driving the crawl" without a stated relationship.

### Assessment

Not yet ready. The blocking question is a scope decision the author must make:
is per-navigation scheme enforcement (refusing plaintext `http` without
`--allow-insecure-scheme`) in scope for this story? Fold it into the Summary and
add a matching acceptance criterion to keep it in scope, or walk the Requirements
claim back to "the flag is carried into the `AccessPolicy` verdict but scheme
enforcement is a separate concern" to keep the story purely reachability as
titled. Either resolves both majors and the scope suggestion; the remaining
minors (envelope-payload shape, AC6 second clause, positive allowance path,
sequencing direction, actor relationship) are optional polish.

## Re-Review (Pass 3) — 2026-08-31

**Verdict:** COMMENT

The scheme-scope decision (in scope) was folded in cleanly: the Summary names
the full `AccessPolicy` verdict, a scheme-enforcement AC and a positive-path AC
were added, and the pass-2 actor and sequencing suggestions were resolved. Both
pass-2 majors are gone. The scope lens (having read the parent plan) confirms
the scheme allowance already exists at the front door, so per-request extension
is genuinely the same indivisible change. Findings have converged to fine-grained
polish — one major (the new scheme AC does not mirror AC2's refusal shape) and a
handful of minors — none blocking implementation.

### Previously Identified Issues (Pass 2)

- 🟡 **Clarity**: Summary omits scheme enforcement — Resolved. Summary now names
  reachability and scheme together.
- 🟡 **Testability**: scheme enforcement has no AC — Resolved. New scheme-refusal
  AC added (though its refusal shape needs pinning — see new issues).
- 🔵 **Scope**: scheme enforcement widens scope beyond Summary/ACs — Resolved.
  Now consistent across Summary, Requirements and ACs.
- 🔵 **Testability**: envelope payload has no defined shape — Resolved. AC2 now
  names the classification values (`private`, `link-local`, `reserved`).
- 🔵 **Testability**: AC6 second clause subjective — Resolved. Anchored to an
  explicit per-request statement each doc must contain.
- 🔵 **Testability**: positive allowance path not asserted — Resolved. New AC
  covers both `--allow-internal` and `--allow-insecure-scheme` success.
- 🔵 **Dependency**: sequencing direction unstated — Resolved. Prefers 0206 first.
- 🔵 **Clarity**: operator vs agent actor — Resolved. Assumptions now defines the
  crawl-driving agent as automation acting on the operator's behalf.

### New Issues Introduced

- 🟡 **Testability** (major): The scheme (`http`) refusal AC says "refused" but,
  unlike AC2 for reachability, does not state the observable refusal shape
  (envelope, `retryable`, `details` classification). Mirror AC2, e.g. an
  `insecure-scheme` classification in `details`.
- 🔵 **Clarity** (minor): "reachability and scheme together" implies the front
  door already enforces scheme, which Context does not confirm (the scope lens
  found the plan does confirm it — a one-line Context addition closes the gap).
- 🔵 **Testability** (minor): AC8's "(and equivalent caveats)" is unbounded —
  enumerate the phrases or keep only the positive, checkable half.
- 🔵 **Testability** (minor): "IPv6 transition forms" names a class without
  concrete vectors — reference the exact vector list `validate-source` tests.
- 🔵 **Dependency** (minor): directional precedence over 0209 sits under
  "Coordinate with" but "Blocks: none known" does not reflect it; 0209's own
  Dependencies should record the reciprocal.
- 🔵 **Dependency** (minor): the crawl-driving skill that invokes the executor
  must forward the new allowances, or a legitimate `--allow-internal` crawl is
  refused mid-crawl — this consumer coupling is not captured.
- 🔵 **Clarity** (suggestion): "the design page" referent is not tied to a file.
- 🔵 **Clarity** (suggestion): "the front door" metaphor is used in the Summary
  before Context defines it.
- 🔵 **Scope** (suggestion): the doc-wording cleanup rides along with the code
  change — correctly kept together; no action.
- 🔵 **Testability** (suggestion): positive-path "succeeds" lacks a defined
  observable success outcome.

### Assessment

Ready to implement. The verdict is COMMENT — the work item is acceptable as-is,
and both blocking majors from pass 2 are resolved. Three cheap, high-value fixes
would leave it airtight: mirror AC2's refusal shape for the scheme case (the one
major), add a Context line confirming `validate-source` already enforces scheme
(closes the clarity premise gap the plan already supports), and capture the
crawl-driving skill as a consumer that must forward the allowances. The
remaining minors and suggestions are optional.

## Approval — 2026-08-31

**Verdict:** APPROVE

Approved after the three high-value pass-3 fixes were applied to the work item:
the scheme (`http`) refusal AC now mirrors AC2's shape (`retryable: false`,
`insecure-scheme` classification in `details`, URL never loaded); Context now
confirms `validate-source`'s `AccessPolicy` verdict already enforces scheme, so
this work carries it down rather than inventing it; and Dependencies captures
`inventory-design` as an in-scope consumer that must forward the operator's
allowances. The remaining pass-3 minors and suggestions (unbounded "equivalent
caveats", concrete IPv6-transition vectors, the `Blocks`/0209 reciprocal, the
"design page"/"front door" referents, a defined positive-path success outcome)
are optional polish and were consciously left open. Work item `status` remains
`ready`.
