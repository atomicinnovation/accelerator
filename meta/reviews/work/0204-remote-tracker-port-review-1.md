---
type: "work-item-review"
id: "0204-remote-tracker-port-review-1"
title: "Work Item Review: RemoteTracker Port"
date: "2026-08-10T17:04:12+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0204"
relates_to: ["work-item-review:0194-tracker-crate-and-remote-sync-engine-review-2"]
work_item_id: "0204"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: ["rust", "tracker", "sync", "port"]
last_updated: "2026-08-10T17:52:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: RemoteTracker Port

**Verdict:** REVISE

0204 is a well-motivated, genuinely atomic extraction: all five lenses agree
the split from 0194 was structurally right, the unit is coherent, every
requirement carries its rationale, the negative scope is explicit, and the
`work`-dependency invariant is mechanically enforced rather than asserted.
The problem is that the one thing the item exists to deliver — a frozen port
signature two stories build against in parallel — is not actually written
down: `fetch_all()` has no return shape, `update` has no return type,
`RemoteIssue.updated` has no type, and the criterion guarding all of this
cannot fail. Compounding it, the item disagrees with itself about whether
the port's error type is part of the public API, and its Assumptions license
the frozen surface to reopen after acceptance while Open Questions records
"None outstanding".

### Cross-Cutting Themes

- **The frozen contract is not stated** (flagged by: clarity, completeness,
  testability, scope) — three of the four operations' signatures are
  incomplete or absent, and the criterion meant to gate them ("all four
  operations carry complete signatures") is compiler-guaranteed for any crate
  that compiles. The item instructs the implementer to state the shape the
  item itself exists to freeze. This is the single defect that undermines the
  split's purpose.

- **"Exposes exactly three types" contradicts the port error type** (flagged
  by: clarity, completeness, scope, testability) — AC1 enumerates
  `RemoteTracker`, `ExternalId`, `RemoteIssue` as the whole public API, while
  AC3 and AC4 require a publicly-matchable port error type. A consumer cannot
  match on a type the crate does not export. The error type is never named,
  and it is unresolved whether it is crate-local or `kernel::Error`.

- **The contract is frozen and not-frozen at once** (flagged by: clarity,
  completeness, scope, dependency) — Assumptions concedes that if 0194's
  `create --push` retry idempotency needs a lookup, "that surface lands here";
  Open Questions says "None outstanding"; and the item unblocks 0171 and 0194
  in parallel, so 0194's discovery necessarily lands *after* 0171 has begun.
  That is exactly the churn scenario the split was made to prevent.

- **Verification artefacts are unscoped deliverables** (flagged by:
  completeness, scope, testability) — the probe crate, the fake and the
  consumer appear only inside acceptance criteria. None has a name, a home in
  the `cli/` workspace, a lifecycle, or a task that builds and runs it. A
  compile-time guard nothing builds is not a guard.

- **`RemoteIssue.updated` is required but ungated** (flagged by:
  completeness, testability, dependency) — one of three defects the Drafting
  Notes claim are "fixed here rather than carried over" has no acceptance
  criterion and no stated type, and its own rationale says a mismatch fails
  *silently*.

- **The dependency record disagrees with its neighbours** (flagged by:
  dependency, scope) — 0171 records `blocked_by: [0187, 0204, 0194]` and says
  its cutover half still waits on 0194, contradicting this item's "0171 no
  longer needs anything from 0194"; and parent epic 0136 has no entry for
  0204 while still recording 0194 as preceding 0171's clients.

### Findings

#### Critical

- 🔴 **Testability**: The four-operations criterion cannot fail
  **Location**: Acceptance Criteria (four port operations)
  Rust requires fully-typed trait method signatures, so "carry complete
  signatures — parameters and return types" is satisfied automatically by any
  crate that compiles. The second half asks that `fetch_all()`'s shape be
  "stated", but the work item never states it — whichever shape the
  implementer picks becomes the frozen contract by default rather than by
  decision.

#### Major

- 🟡 **Clarity**: The frozen signature is left unstated in three places,
  contradicting the Summary and Drafting Notes
  **Location**: Requirements
  `fetch_all()` has no return shape, `update(external_id, title, body)` has no
  return type while `create` and `show` both carry one, and
  `RemoteIssue.updated` is only required to be "stated". Summary calls the
  crate "a finished contract rather than a partial one" and Drafting Notes
  claims two of these three gaps are already fixed here.

- 🟡 **Completeness**: Two of the four operations have no stated return type
  **Location**: Requirements
  The Requirements bullet demands "all four operations with complete
  signatures" while itself giving return types for only two. The signature
  0171 waits on is not knowable from reading the work item.

- 🟡 **Clarity**: "Exposes exactly three types" collides with the port's own
  error type
  **Location**: Acceptance Criteria
  A type named in every public signature is part of the public API, so either
  AC1's list is wrong or the error is not the crate's own. The
  `RestrictImports` rule permitting `kernel::Error` leaves it genuinely
  unclear which.

- 🟡 **Completeness**: The port's error type is never named and is excluded
  from the "exactly" enumeration
  **Location**: Acceptance Criteria
  A verifier cannot tell whether an exported `TrackerError` passes or fails
  AC1, and an implementer must guess where the error lives — a decision that
  changes every method signature in the contract this item exists to freeze.

- 🟡 **Scope**: Summary and AC1 define a three-item surface; Requirements and
  AC3/AC4 require a fourth
  **Location**: Summary / Acceptance Criteria / Requirements
  An implementer satisfying AC1 literally either omits the error type or
  reuses `kernel::Error` and loses the retryable/terminal distinction both
  consumers depend on.

- 🟡 **Testability**: The public-API criterion's pass condition is
  contradictory
  **Location**: Acceptance Criteria (public API)
  Two verifiers can reach opposite verdicts on the same crate, and the
  ambiguity sits on the item's most basic criterion.

- 🟡 **Clarity**: Assumptions contemplates the port growing after this item
  lands
  **Location**: Assumptions
  0171 is unblocked the moment this lands and 0194 runs in parallel, so 0194's
  discovery will almost certainly come after 0171 has begun. A reader cannot
  tell whether the signature is frozen at acceptance or provisionally frozen.

- 🟡 **Completeness**: Open Questions declares nothing outstanding while
  Assumptions carries an unresolved contract question
  **Location**: Open Questions
  The item's status is `ready`, and this question is the one thing that could
  invalidate the milestone's stated purpose.

- 🟡 **Scope**: The item advertises a frozen contract while its own
  Assumptions leave the surface open
  **Location**: Assumptions / Open Questions
  0194's pass-3 re-review records the create-retry criterion as still having
  no constructible precondition, noting explicitly that fixing it "may need
  port surface, which would break the signature promised to 0171".

- 🟡 **Dependency**: The post-acceptance widening has no ordering constraint
  against 0171 starting
  **Location**: Assumptions
  0194 names itself "the port's first consumer and its design driver", yet
  nothing states what happens to 0171's in-flight adapters or this item's
  probe crate if the trait widens after acceptance.

- 🟡 **Dependency**: Dependencies claims this is the whole of 0171's wait,
  contradicting 0171's own record
  **Location**: Dependencies
  0171 carries `blocked_by: [0187, 0204, 0194]` and states it is blocked by
  0194 "for the **cutover half only** — the script removal, skill repointing,
  conversational conflict flow and contract-suite run". Only 0171's client
  crates are freed by 0204.

- 🟡 **Dependency**: Parent epic 0136 has no entry for 0204 and still records
  0194 as preceding 0171's clients
  **Location**: Frontmatter: parent / Dependencies
  The epic can satisfy its own completion criterion with 0204 unbuilt, and its
  ordering record still points 0171 at 0194 for a milestone 0204 now owns.

- 🟡 **Completeness**: The `RemoteIssue.updated` requirement has no acceptance
  criterion
  **Location**: Acceptance Criteria
  The other two named inherited defects each received a dedicated criterion;
  this one received none, and the item never states the type. Its own
  rationale says a mismatch at that boundary "silently breaks classification".

- 🟡 **Testability**: `RemoteIssue.updated` is ungated and untyped
  **Location**: Acceptance Criteria / Requirements
  A verifier working from the criteria alone would accept an implementation
  that types `updated` however it likes and never records the baseline
  correspondence.

- 🟡 **Completeness**: The probe crate is a workspace deliverable that appears
  only in an acceptance criterion
  **Location**: Acceptance Criteria
  It has no name, no home in `cli/`, and no statement of whether it ships, is
  test-only, or needs the registration checklist — while sitting beside this
  item's own no-sibling-crate rule.

- 🟡 **Testability**: The probe crate and fake have no stated home or runner
  **Location**: Acceptance Criteria (probe crate; error classes)
  Only the final criterion names an invocation, and it is scoped to "the new
  crate" (singular). A guard no task builds can rot or be silently excluded.

- 🟡 **Testability**: The signature-narrowness criterion produces both false
  failures and false passes
  **Location**: Acceptance Criteria (signature narrowness)
  The permitted list omits `Result` and containers, so a conforming
  `fetch_all() -> Result<Vec<RemoteIssue>, Error>` reads as a violation; while
  the probe-crate mechanism accepts any `std` type, so a widening to
  `PathBuf` or `Duration` would compile and pass.

- 🟡 **Testability**: Sync-vs-async, receiver form and dyn-compatibility are
  pinned by no criterion
  **Location**: Acceptance Criteria / Requirements
  0194 selects the active client at the composition root per `work.integration`
  — typically `Box<dyn RemoteTracker>` — and 0171's clients perform HTTP,
  forcing the sync-versus-async question. A flip after 0171 starts breaks the
  freeze.

- 🟡 **Testability**: "Carries no behavioural logic" is unmeasurable
  **Location**: Acceptance Criteria (no behavioural logic)
  A `Display` impl, a `FromStr` for `ExternalId`, or an `is_retryable()`
  helper could each be argued in or out. Only the `-adapters`-absence half is
  checkable.

- 🟡 **Dependency**: The retryable/terminal taxonomy's dual ownership with the
  live bash bridge codes is uncaptured
  **Location**: Requirements
  `work-item-bridge-codes.sh` stays authoritative in the interim with the Rust
  definition "asserted against it by fixture", and 0171 carries a criterion to
  delete "its parity fixture" — an artefact no item is obliged to produce.

- 🟡 **Dependency**: The projected-body obligation on `RemoteIssue.body` is
  absent from the port
  **Location**: Requirements
  0194 requires adapters to return an already-projected body rather than raw
  tracker JSON, and 0171 must reproduce the projection recipes exactly because
  a whitespace difference reclassifies every synced item as
  `remotely-modified`. Neither obligation is visible in the artefact 0171
  builds to.

#### Minor

- 🔵 **Clarity**: The permitted-type list cannot express `fetch_all()`'s return
  **Location**: Acceptance Criteria
  Read literally it forbids the collection type and the `Result` wrapper the
  port must have.

- 🔵 **Clarity**: References to 0194's "Phase A" and "four phases" no longer
  resolve
  **Location**: Context
  Post-split, 0194 lists three phases and its Phase A is the sync state
  machine, not the port.

- 🔵 **Clarity**: `create()`'s `kind` parameter is undefined and carries
  work-domain meaning across a boundary required to be free of `work`
  **Location**: Requirements
  Without a stated value set, each of 0171's adapter authors invents their own
  Jira/Linear issue-type mapping.

- 🔵 **Clarity**: The retryable/terminal distinction is named in two
  vocabularies without a stated mapping
  **Location**: Requirements
  0194 expresses it as `E_DISPATCH_RETRYABLE`/`E_DISPATCH_TERMINAL`; 0204
  never says whether the port's classes must correspond.

- 🔵 **Completeness**: 0194 is listed under both `relates_to` and `blocks`
  **Location**: Frontmatter: relates_to / blocks
  Duplicate edges of differing strength make the graph ambiguous for
  traversing consumers. 0194 itself lists 0204 under `blocked_by` only.

- 🔵 **Scope**: The probe crate is a second workspace crate no Requirement
  scopes in
  **Location**: Acceptance Criteria / Requirements
  The delivered footprint is larger than the footprint the Summary uses to
  justify the split as "cheap to reach".

- 🔵 **Scope**: Ownership of the fake `RemoteTracker` is unassigned between
  0204 and 0194
  **Location**: Acceptance Criteria / Dependencies
  0194 separately requires a fake plus a shared parameterised contract test;
  neither item says whether these are one artefact or two.

- 🔵 **Dependency**: Jira and Linear appear nowhere in Dependencies
  **Location**: Dependencies
  A contract frozen ahead of its implementers is being fixed without the two
  external APIs that must satisfy it — bulk-fetch pagination, rate limits,
  timestamp granularity.

- 🔵 **Dependency**: 0166 is not the source of the `kernel` crate this item
  depends on
  **Location**: Dependencies
  0166 is "Shared config, corpus, and store Crates"; the workspace and kernel
  came from the earlier foundation items.

- 🔵 **Dependency**: The `updated` ↔ `remote_updated_at` coupling names no
  owner for the baselines bash already wrote
  **Location**: Requirements
  A type that cannot round-trip values already on users' disks would be
  discovered only in 0194's stability check, after the freeze.

- 🔵 **Testability**: The "without reference to any provider type" clause is
  vacuously satisfied
  **Location**: Acceptance Criteria (error classes)
  The per-provider crates are 0171's deliverable and do not yet exist.

#### Suggestions

- 🔵 **Clarity**: "No behaviour to test" understates the three test artefacts
  the criteria mandate
  **Location**: Context
  The item's stated cheapness is the argument for the split.

- 🔵 **Clarity**: The registration checklist reference is qualified in a way
  that leaves its applicability unresolved
  **Location**: Technical Notes

- 🔵 **Completeness**: Context explains why this is a separate item but not
  what the port ultimately serves
  **Location**: Context

- 🔵 **Scope**: `kind: story` is a stretch for a trait, two value types and a
  lint rule
  **Location**: Frontmatter: kind

- 🔵 **Scope**: Parent epic 0136's decomposition does not list 0204
  **Location**: Dependencies

- 🔵 **Testability**: No criterion exercises the sufficiency Assumptions names
  as the sharpest risk
  **Location**: Assumptions

### Strengths

- ✅ The extraction rationale is structural rather than size-driven, and is
  documented on both sides — 0194 records `blocked_by: 0204` and explains the
  split, so the new unit is agreed by both items rather than asserted by one.
- ✅ Exceptional coherence: all seven Requirements and seven Acceptance
  Criteria serve a single deliverable, with nothing that could be delivered or
  rolled back independently of the rest.
- ✅ The boundary is stateable in both directions — the item declares what is
  deliberately not built (no logic, no persistence, no HTTP, no
  `tracker-adapters` sibling) and records the two weighed-and-rejected
  alternatives.
- ✅ The no-`work`-dependency invariant is mechanically enforced, not asserted:
  a `Cargo.toml` absence check plus a named cargo-pup `RestrictImports` rule
  whose permitted set is spelled out, failing `cli:check` rather than review.
- ✅ Every Requirements bullet carries its rationale inline, so no constraint
  reads as arbitrary.
- ✅ "Port" is used consistently in its hexagonal sense throughout, fixing the
  term overloading a prior review found in 0194.
- ✅ The four operations are each anchored to a named bash script, and the
  three-scripts-for-four-operations mismatch is explained rather than left as
  unexplained arithmetic.
- ✅ Every expected section is present and substantively populated; frontmatter
  is complete and kind-appropriate, and the `blocks` edges are reciprocated by
  `blocked_by` on both 0171 and 0194.
- ✅ The public-API criterion is mechanised with a probe crate rather than left
  to inspection — a deliberate, documented improvement over 0194's version.
- ✅ The retryable/terminal criterion demands a demonstration rather than
  asserting the property as a fact about the type.
- ✅ Drafting Notes carry a dated trail naming the three defects inherited from
  0194 and the review pass that surfaced them.

### Recommended Changes

1. **Write the four signatures verbatim into Requirements** (addresses: the
   four-operations criterion cannot fail; frozen signature left unstated; two
   operations have no return type; `updated` ungated and untyped;
   sync/async/dyn unpinned)
   Give the literal trait definition — receiver form, sync or async,
   `fetch_all()`'s return collection and its keying, `update`'s return type,
   `RemoteIssue.updated`'s concrete type — and restate the criterion as a
   comparison against it: "the four trait methods match the signatures given
   in Requirements exactly". Add a criterion that a `Box<dyn RemoteTracker>`
   is constructible from the fake and all four operations invoked through it,
   so object-safety is gated too.

2. **Name the port error type and reconcile the "exactly" list** (addresses:
   three vs four public items; error type never named; public-API criterion
   contradictory; narrowness criterion false pass/fail)
   Decide whether the error is a crate-local `TrackerError` or a reuse of
   `kernel::Error`, state it in Requirements alongside the value types, and
   rewrite AC1 to enumerate the full intended export set. Reword the
   narrowness criterion to permit `Result` and the chosen container
   explicitly, so it forbids provider and work-domain types rather than
   everything outside the four named items.

3. **Close the retry-idempotency question inside 0204** (addresses: the
   contract is frozen and not-frozen at once; Open Questions vs Assumptions;
   post-acceptance widening has no ordering constraint)
   Either decide the port carries a lookup operation (making it five) or
   record the decision that retry idempotency is resolved locally in `work`
   via a pending-push marker and never touches the port. Move that decision
   from Assumptions into Requirements with a matching criterion, and add a
   Dependencies bullet naming the reverse coupling on 0194 with the condition
   under which the signature is considered frozen for 0171.

4. **Scope the verification artefacts as first-class deliverables**
   (addresses: probe crate appears only in an AC; no stated home or runner;
   second workspace crate unscoped; fake ownership unassigned)
   Add a Requirements bullet naming the probe crate (or, cheaper and truer to
   the one-crate framing, a `trybuild` compile-fail test inside `tracker`),
   siting it in the workspace, stating its lifecycle, and confirming it is
   built and run by `mise run cli:check`. Say whether this item's fake is a
   throwaway fixture and the shared parameterised contract test is 0194's.

5. **Correct the dependency record against its neighbours** (addresses: "the
   whole of 0171's wait"; epic 0136 missing 0204; 0194 double-linked; 0166 not
   the `kernel` source)
   Narrow the Blocks claim to 0171's client adapter crates, noting the cutover
   half remains blocked by 0194. Add 0204 to epic 0136's Decomposition and
   completion criterion, and rewrite 0194's decomposition entry so it no
   longer claims to precede 0171's client adapters. Drop `work-item:0194` from
   `relates_to`, and name the correct upstream for `kernel`.

6. **Capture the couplings that pass through the port** (addresses:
   retryable/terminal dual ownership; projected-body obligation;
   `updated` ↔ `remote_updated_at` ownership; Jira and Linear absent)
   State that `body` is the already-projected domain body per
   `work-item-project-remote.sh`'s recipe; record that
   `work-item-bridge-codes.sh` remains the authoritative taxonomy until 0171
   retires it and name which item owns the parity fixture; require that
   `updated`'s type round-trips the `remote_updated_at` values existing
   bash-written baselines contain; and add an external-systems bullet naming
   Jira REST and Linear GraphQL with the constraints they place on
   `fetch_all()`.

7. **Fix the smaller clarity gaps** (addresses: `kind` undefined; Phase A
   references; error vocabularies; Context lacks the served capability;
   registration checklist applicability)
   State `kind`'s type and permitted values (or say it is opaque); mark the
   0194 phase quotes as describing the pre-split structure; state whether the
   port's error classes correspond to `E_DISPATCH_RETRYABLE`/`_TERMINAL`; open
   Context with the capability the port serves; and name which registration
   obligations apply to a plain library crate.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0204 is short, consistently voiced and mostly unambiguous:
pronouns resolve cleanly, every requirement names the implementer as actor,
each port operation is anchored to a named bash script, and "port" is used
only in its hexagonal sense (fixing the overloading a prior review found in
0194). The clarity weaknesses are internal contradictions rather than vague
prose: the frozen signature the item exists to deliver is itself left unstated
in three places while Summary and Drafting Notes assert it is complete and
that those exact gaps are "fixed here"; the "exposes exactly three types"
criterion collides with the port's own error type appearing in every public
signature; and the Assumptions bullet contemplates the signature changing
after the item lands, undercutting the stability premise the whole split was
made for.

**Strengths**:
- The crate's vocabulary is enumerated explicitly rather than described in the
  abstract.
- "Port" is used consistently in its hexagonal sense throughout.
- The four operations are each anchored to a named bash script, and the
  three-scripts-for-four-operations mapping is explained.
- Every constraint carries its rationale inline.
- Requirements are imperative with an unambiguous actor; criteria state
  observable states.
- Specialised terms are anchored to existing precedent (the cargo-pup rule
  "matching the shape used for `config`, `corpus`, `vcs` and `work`").

**Findings**:

- **major / high** — *The frozen signature the item exists to deliver is
  itself left unstated in three places, contradicting the Summary and Drafting
  Notes* — **Location**: Requirements
  Three elements of the signature are deferred to the implementer:
  `fetch_all()` has no return shape, `update(external_id, title, body)` has no
  return type while `create` and `show` both carry one, and
  `RemoteIssue.updated`'s type is only required to be "stated". This
  contradicts the surrounding prose — Summary calls the crate "a finished
  contract rather than a partial one", the Acceptance Criteria demand
  `fetch_all()`'s return shape be "stated explicitly rather than left to the
  implementer" (an instruction that cannot be satisfied by the implementer who
  is being told it), and Drafting Notes asserts all three gaps are "fixed here
  rather than carried over" when only the error type actually is.
  **Impact**: Two implementers reading this item would freeze two different
  signatures, defeating the item's sole purpose.
  **Suggestion**: Write the four concrete signatures into Requirements, and
  correct the Drafting Notes claim.

- **major / high** — *"Exposes exactly RemoteTracker, ExternalId and
  RemoteIssue" contradicts "the port's own error type" in every public
  signature* — **Location**: Acceptance Criteria
  A type named in every public signature is part of the public API, so either
  the list of three is wrong or the error is not the crate's own — and the item
  never says which. The `RestrictImports` rule permits `kernel::Error`, leaving
  it unclear whether the implementer defines a new `tracker` error enum or
  reuses the shared one.
  **Impact**: The error type is one of the three things this item freezes and
  both consumers match on it; the "exactly three" criterion would pass or fail
  depending on which reading a verifier picks.
  **Suggestion**: Name the error type explicitly, say where it is defined, and
  reconcile the "exactly" list.

- **major / medium** — *Assumptions contemplates the port surface growing after
  this item lands, contradicting the stability premise the split was made for*
  — **Location**: Assumptions
  Summary and Context justify the split on 0171 not starting "against an
  unaccepted branch whose signature can still churn underneath it", yet
  Assumptions says that if 0194 needs a lookup the port does not offer, "that
  surface lands here rather than being bolted on after 0171 has begun
  implementing" — but 0171 is unblocked the moment this lands and 0194 runs in
  parallel, so 0194's discovery will come after. Open Questions declares "None
  outstanding" while this would change the frozen contract.
  **Impact**: A reader cannot tell whether the signature is frozen at
  acceptance or provisionally frozen pending 0194's findings.
  **Suggestion**: Settle the retry-idempotency surface now, or state plainly
  that the port may gain one operation and record the reopening condition.

- **minor / medium** — *The permitted-type list in the public-API criterion
  cannot express `fetch_all()`'s return* — **Location**: Acceptance Criteria
  "References only `ExternalId`, `RemoteIssue`, `String`/`&str` and the port's
  own error type" read literally forbids the collection type `fetch_all()` must
  return and the `Result` wrapper the error implies.
  **Impact**: A verifier applying the criterion as written would reject a
  correct implementation.
  **Suggestion**: Permit `std` container and `Result` types explicitly.

- **minor / medium** — *References to 0194's "Phase A" and "four phases" no
  longer resolve against 0194 as it now stands* — **Location**: Context
  Post-split, 0194 lists three phases and its Phase A is the sync state
  machine, not the port.
  **Impact**: The passages explaining why the split happened send the reader to
  a phase label whose meaning has changed.
  **Suggestion**: Mark the quotes as describing 0194's pre-split structure, or
  drop the ordinals.

- **minor / medium** — *The `kind` parameter of `create()` is undefined, and it
  is a work-domain concept crossing a port required to be free of `work`* —
  **Location**: Requirements
  No type, no value set, and no statement of whether it is the work item's own
  `kind` frontmatter value or a tracker-side issue type.
  **Impact**: 0171's adapters must map `kind` onto a Jira issue type and a
  Linear equivalent; without a stated value set each author invents their own.
  **Suggestion**: State `kind`'s type and permitted values, or say it is an
  opaque caller-supplied string the port does not interpret.

- **minor / medium** — *The retryable/terminal distinction is named in two
  vocabularies across 0204 and its consumer without a stated mapping* —
  **Location**: Requirements
  0194 expresses the same distinction as `E_DISPATCH_RETRYABLE` and
  `E_DISPATCH_TERMINAL`, owned by `work-item-bridge-codes.sh`; 0204 never names
  those codes or says whether its two classes correspond.
  **Impact**: A reader cannot tell whether the port must reproduce the existing
  taxonomy's semantics; 0194's criteria are written against that reading.
  **Suggestion**: State the correspondence and give one-line semantics for each
  class.

- **suggestion / low** — *"No behaviour to test" understates the three test
  artefacts the criteria mandate* — **Location**: Context
  The criteria require a fake, a consumer, and a probe crate. The criteria are
  compile-time rather than behavioural, so the statements are reconcilable, but
  a reader sizing the work from Context alone would not expect three artefacts.
  **Suggestion**: Reword to "no runtime behaviour to test — verification is
  compile-time" and say where the probe crate lives.

- **suggestion / medium** — *The registration checklist reference is qualified
  in a way that leaves its applicability unresolved* — **Location**: Technical
  Notes
  The qualification ("though this crate is a library with no dispatch token of
  its own") withdraws most of what the pointer offers without saying which
  parts survive.
  **Suggestion**: Name the obligations that apply to a plain library crate, or
  state that only workspace membership is required.

### Completeness

**Summary**: 0204 is structurally complete — every expected section is present
and substantively populated, the frontmatter is intact and kind-appropriate for
a story, six of its seven Requirements have a matching Acceptance Criterion,
and the Drafting Notes carry a dated trail naming the three defects it
inherited from 0194 and set out to fix. The gaps are content gaps inside
otherwise healthy sections, and they cluster on the one thing the item exists
to deliver: the frozen port contract.

**Strengths**:
- Every expected section is present and substantively populated, with no
  placeholder sections.
- Frontmatter is complete and kind-appropriate, and the `blocks: work-item:0194`
  edge is bidirectionally consistent with 0194's `blocked_by: work-item:0204`.
- Requirements→Acceptance Criteria coverage is near-complete (six of seven),
  and the criteria are mechanised rather than left to inspection.
- Each Requirements bullet carries its rationale rather than just an
  instruction.
- Context names a genuinely structural motivation for the split.
- Technical Notes record the weighed-and-rejected alternatives and the
  live-workspace context; Drafting Notes enumerate the three inherited defects.
- Open Questions is explicitly closed out rather than left blank, and
  Dependencies enumerates blockers, both Blocks edges with reasons, and the
  parent epic.

**Findings**:

- **major / high** — *Two of the four port operations still have no stated
  return type, in the item whose sole purpose is to state them* —
  **Location**: Requirements
  The bullet demands "all four operations with complete signatures" yet gives
  return types for only two. `update` has no return arrow; `fetch_all()` has
  neither parameters nor a return shape, only a meta-instruction. The
  Acceptance Criteria repeat the instruction rather than closing it.
  **Impact**: The signature 0171 waits on is not knowable from reading the work
  item.
  **Suggestion**: Write the two missing return types into Requirements —
  including whether `update` returns anything on success, and a `fetch_all()`
  collection shape carrying enough identity to associate each record with a
  work item.

- **major / high** — *The port's error type is never named and is excluded from
  the criterion that enumerates the public API "exactly"* — **Location**:
  Acceptance Criteria
  Two criteria depend on the error type, but it is never named, and the first
  criterion's three-item list omits it. The cargo-pup rule permits
  `kernel::Error`, leaving it unresolved whether the error is crate-defined
  (violating the enumeration) or a reuse (which cannot obviously carry a
  retryable/terminal split without further specification).
  **Impact**: A verifier cannot tell whether an exported `TrackerError` passes;
  an implementer must guess where the error lives.
  **Suggestion**: Name it in Requirements, state where it is defined, and
  reconcile the "exactly three types" criterion.

- **major / medium** — *The `RemoteIssue.updated` requirement — one of three
  named inherited defects — has no acceptance criterion* — **Location**:
  Acceptance Criteria
  The other two inherited defects each received a dedicated criterion; this one
  received none, and the item never states the type. The criteria mention
  `RemoteIssue` only as a type appearing in the public API.
  **Impact**: The one requirement of seven with no verification is the one
  whose failure mode the item describes as silent.
  **Suggestion**: State `updated`'s concrete type and add a criterion asserting
  both the type and the documented `remote_updated_at` correspondence.

- **major / medium** — *The probe crate is a workspace deliverable that appears
  only in an acceptance criterion* — **Location**: Acceptance Criteria
  No Requirements bullet describes it; it has no name, no home in `cli/`, and
  no statement of whether it ships, is test-only, or needs the registration
  checklist — sitting awkwardly beside the no-sibling-crate rule.
  **Impact**: A deliverable that adds a crate to a workspace governed by a
  thirteen-point registration checklist is specified only as a verification
  aside.
  **Suggestion**: Add a Requirements bullet naming it, siting it, and stating
  its lifecycle, and clarify it does not violate the no-sibling rule.

- **major / medium** — *Open Questions declares nothing outstanding while
  Assumptions carries an unresolved contract question* — **Location**: Open
  Questions
  The Assumptions bullet records a live, undecided design question and concedes
  that if it resolves the other way, the signature this item exists to freeze
  would widen after the fact. The item's status is `ready`.
  **Impact**: A reader taking Open Questions at face value concludes the
  contract is settled.
  **Suggestion**: Move the question into Open Questions, or resolve it now.

- **minor / medium** — *0194 is listed under both `relates_to` and `blocks`* —
  **Location**: Frontmatter: relates_to / blocks
  The same target expressed twice under two linkage keys of differing strength.
  0194 itself lists 0204 under `blocked_by` only, reserving `relates_to` for
  items with no blocking edge.
  **Impact**: Ambiguous graph for traversing consumers; invites drift.
  **Suggestion**: Drop `work-item:0194` from `relates_to`.

- **suggestion / medium** — *Context explains why this is a separate item but
  not what the port ultimately serves* — **Location**: Context
  Context is entirely split provenance; it never states the capability the port
  serves (remote work-item sync from the Rust CLI, replacing the bash bridge
  scripts) or who benefits.
  **Suggestion**: Add one or two sentences naming the capability and its
  beneficiary, then keep the split rationale as the second paragraph.

### Dependency

**Summary**: 0204 is dependency-aware by construction — it exists solely to
turn a fragment-level blocker into a schedulable edge, both downstream
consumers are named in Dependencies and the frontmatter `blocks` list is
reciprocated by `blocked_by` on both 0171 and 0194. The gaps are in the
couplings that survive the split rather than in the split itself: the
Dependencies section asserts 0171 waits on nothing but this item, which 0171's
own record contradicts; the parent epic has no entry for 0204 and still records
the superseded ordering; and three obligations that pass through this port are
visible in the consumers but not in the artefact both build against.

**Strengths**:
- Both downstream consumers are named as explicit Blocks entries with reasons,
  and the frontmatter edges are bidirectionally consistent.
- The item is itself the fix for a dependency defect, converting an
  inexpressible fragment-level coupling into a first-class item.
- The "no blockers" claim is justified rather than merely asserted.
- The `tracker`-must-not-depend-on-`work` constraint is captured as a
  mechanically enforced coupling limit.
- Assumptions names the risk that the frozen contract may still need to move,
  and states which consumer would drive it.

**Findings**:

- **major / high** — *Dependencies claims this item is the whole of 0171's
  wait, contradicting 0171's own `blocked_by` record* — **Location**:
  Dependencies
  0171 carries `blocked_by: [0187, 0204, 0194]` and states it is blocked by
  0194 "for the **cutover half only** — the script removal, skill repointing,
  conversational conflict flow and contract-suite run all need the binary that
  story delivers". Only 0171's client crates are freed by 0204.
  **Impact**: Anyone scheduling from this record will believe 0171 can complete
  once 0204 lands; the two items disagree about the same edge.
  **Suggestion**: Narrow the claim to 0171's client adapter crates and thin
  binaries, and qualify the Summary's "neither should wait on the other".

- **major / high** — *Parent epic 0136 has no entry for 0204 and still records
  0194 as the item preceding 0171's clients* — **Location**: Frontmatter:
  parent / Dependencies
  0136's Decomposition runs 0162–0174 plus 0185–0188 and 0194–0197 with no
  0204, and its completion criterion enumerates children by number without it.
  Its 0194 entry still reads "…and precedes 0171's client adapters". The epic
  was updated on 2026-08-05 to record the 0194 split, so the precedent exists.
  **Impact**: The epic can satisfy its own acceptance criterion with 0204
  unbuilt, and still points 0171 at 0194 for a milestone 0204 now owns.
  **Suggestion**: Add 0204 to the Decomposition and the completion criterion,
  and rewrite 0194's entry.

- **major / medium** — *The post-acceptance port widening 0194 is licensed to
  drive has no ordering constraint against 0171 starting* — **Location**:
  Assumptions
  0194's own Requirements name it "the port's first consumer and its design
  driver", while this item encourages 0171 and 0194 to run in parallel the
  moment 0204 lands. No requirement that 0194's design-driver pass completes
  (or is waived) before 0171 begins, and no protocol for in-flight adapters or
  the probe crate if the trait widens.
  **Impact**: The whole value proposition is undermined if 0194 widens the
  trait mid-flight.
  **Suggestion**: Add a Dependencies bullet recording the reverse coupling and
  pin the freeze condition.

- **major / medium** — *The retryable/terminal error taxonomy's dual ownership
  with the live bash bridge codes is uncaptured* — **Location**: Requirements
  The semantics are today owned by `work-item-bridge-codes.sh`, which 0194 says
  "stays authoritative in the interim" with the Rust definition "asserted
  against it by fixture", and which 0171 carries a criterion to delete along
  with "its parity fixture". 0204 names none of this.
  **Impact**: Two implementations, no named owner or gate holding them in step,
  and 0171 is scheduled to delete a fixture that may never exist.
  **Suggestion**: Record the interim authority, name the item that creates the
  parity fixture, and cross-check 0171's deletion criterion.

- **major / medium** — *The already-projected obligation on `RemoteIssue.body`
  — the anti-drift contract 0171 must satisfy — is absent from the port* —
  **Location**: Requirements
  0204 specifies `RemoteIssue { updated, body }` only negatively, which a raw
  provider JSON string would satisfy. 0194 requires adapters to "return an
  already-projected `RemoteIssue { updated, body }` in domain terms rather than
  raw tracker JSON", and 0171 must "reproduce the existing per-provider
  projection recipes exactly" because a body differing by whitespace
  reclassifies every synced item as `remotely-modified`.
  **Impact**: 0171 implements the trait against a contract missing the
  single highest-consequence constraint on the value type it must return.
  **Suggestion**: State that `body` is the already-projected domain body per
  `work-item-project-remote.sh`, and record projection as a coupling.

- **minor / medium** — *Jira and Linear — the two external APIs the frozen
  contract must satisfy — appear nowhere in Dependencies* — **Location**:
  Dependencies
  Neither provider is named (they appear only inside 0171's title), and nothing
  records the constraints they place on the frozen shape. 0194's Dependencies,
  by contrast, notes that "`fetch_all()` exposes it to per-tenant rate limits on
  large corpora".
  **Impact**: A provider constraint surfacing during 0171 (pagination, rate
  limits, timestamp granularity) would break the promised-stable signature.
  **Suggestion**: Add an external-systems bullet naming both APIs and the known
  `fetch_all()` constraints.

- **minor / medium** — *The sole named upstream, 0166, is not the source of the
  `kernel` crate this item depends on* — **Location**: Dependencies
  Per 0136's decomposition, 0166 is "Shared config, corpus, and store Crates",
  while the workspace and its kernel came from the earlier foundation items
  (0163/0164).
  **Impact**: Nothing is actually gated, but the upstream record misattributes
  the dependency.
  **Suggestion**: Name the crate sources accurately.

- **minor / medium** — *The `RemoteIssue.updated` ↔ `remote_updated_at`
  coupling names no owner for the baselines bash already wrote* —
  **Location**: Requirements
  `last-sync.json` and its `remote_updated_at` values are written today by the
  live bash sync path; 0194 owns preserving that storage contract and requires
  items whose baselines bash wrote to still classify as `synced`.
  **Impact**: A type that cannot round-trip existing on-disk values would be
  discovered only in 0194's stability check, after the freeze.
  **Suggestion**: Record the round-trip requirement and name 0194 as the owner
  of the baseline storage contract.

### Scope

**Summary**: 0204 is a genuinely coherent, atomic unit: every requirement and
criterion serves one deliverable — the `RemoteTracker` port crate, its two
value types and the lint rule that keeps it narrow — with no bundled second
concern, no cross-service span, and explicit negative scope. The extraction
from 0194 is well-justified structurally. The scope weaknesses are
boundary-definition rather than bundling: the item advertises a frozen contract
while its own Assumptions concede the surface may reopen after acceptance, and
the deliverable set disagrees between Summary/AC1 and Requirements/AC3–AC4.

**Strengths**:
- Exceptional coherence — no "and also" bundling, nothing independently
  deliverable or rollback-able.
- The boundary is stateable in both directions, with the rejected alternatives
  recorded.
- The extraction rationale is structural and documented on both sides.
- The item does not depend on a parallel thread; it can be planned, delivered
  and verified as a single increment.
- The no-`work` invariant gives the crate a crisp, mechanically-enforced scope
  boundary rather than a stylistic one.

**Findings**:

- **major / high** — *The item's stated purpose is a frozen contract, but its
  own Assumptions leave the surface open to reopening after acceptance* —
  **Location**: Assumptions / Open Questions
  Not hypothetical: 0194's pass-3 re-review records the create-retry criterion
  as still having no constructible precondition and notes explicitly that
  "fixing it may need port surface, which would break the signature promised to
  0171".
  **Impact**: 0204 can be accepted and then reopened after 0171 has begun —
  precisely the failure mode the split was made to prevent.
  **Suggestion**: Decide now whether the port carries a lookup/idempotency
  operation, or record that retry idempotency is resolved locally in `work` via
  a pending-push marker; move the decision into Requirements with a criterion.

- **major / high** — *Summary and AC1 define a three-item public surface;
  Requirements and AC3/AC4 require a fourth* — **Location**: Summary /
  Acceptance Criteria / Requirements
  The sections disagree on what the crate contains.
  **Impact**: For an item whose sole deliverable is an exactly-pinned public
  API, this is a real delivery risk — an implementer satisfying AC1 literally
  either omits the error type or reuses `kernel::Error` and loses the
  distinction both consumers depend on.
  **Suggestion**: Restate the deliverable set consistently in all three places
  as four public items, and word AC1's "exactly" against that list.

- **minor / high** — *The probe crate mandated by the ACs is a second workspace
  crate that no Requirement scopes in* — **Location**: Acceptance Criteria /
  Requirements
  A probe crate is a second, permanent workspace member with its own manifest,
  registration, and cargo-deny/cargo-pup exposure — none of which appears in
  Requirements, which enumerate one crate that "holds no logic".
  **Impact**: The delivered footprint is larger than the one the Summary uses
  to justify the split as "cheap to reach"; the implementer decides
  unilaterally whether it ships.
  **Suggestion**: Name the verification artefacts in Requirements and state
  whether the probe is a workspace member or a compile-fail test inside
  `tracker`.

- **minor / medium** — *Ownership of the fake `RemoteTracker` is unassigned
  between 0204 and 0194* — **Location**: Acceptance Criteria / Dependencies
  0194 separately requires a fake for its whole unit-test suite plus "a shared
  `RemoteTracker` contract test parameterised over implementations". Neither
  item says whether these are one artefact or two.
  **Impact**: Two parallel stories each build a fake, risking duplicated
  divergent doubles or an unplanned test-support surface in a crate promised to
  keep to three or four public items.
  **Suggestion**: State that 0204's fake is a throwaway fixture and the shared
  contract test is 0194's (or the reverse).

- **suggestion / medium** — *`kind: story` is a stretch for a trait, two value
  types and a lint rule with no behaviour* — **Location**: Frontmatter: kind
  **Impact**: Low — housekeeping, though it can distort planning signals if
  story counts are used for sizing across 0136's larger children.
  **Suggestion**: Consider `chore`/`task` if the taxonomy offers one; keep
  `story` if 0136's convention is that all children are stories.

- **suggestion / medium** — *The parent epic's decomposition does not yet list
  0204 as a child* — **Location**: Dependencies
  **Impact**: The epic can be judged complete without 0204, and a reader
  tracing from the parent cannot see the unit both 0171 and 0194 wait on.
  **Suggestion**: Add 0204 under the subdomain-migration group in the same form
  used for the 0194 and 0195–0197 splits, and extend the completion criterion.

### Testability

**Summary**: 0204 is a small item with seven criteria, and its two strongest —
the no-`work`-dependency rule and the `mise run cli:check` gate — are genuinely
mechanical and falsifiable. The weakness is that the item's headline
deliverable, a frozen port signature, is guarded by criteria that cannot fail:
"all four operations carry complete signatures" is compiler-guaranteed for any
crate that compiles, the one shape it singles out (`fetch_all()`) is never
stated anywhere in the item, and the dimensions that would actually cause churn
(sync-vs-async, receiver form, dyn-compatibility, the `updated` type) are pinned
by no criterion at all.

**Strengths**:
- The no-`work`-dependency criterion is mechanically falsifiable and names both
  checks and the failing gate.
- The public-API narrowness criterion is mechanised with a probe crate — a
  deliberate, documented improvement over 0194's version.
- The retryable/terminal criterion demands a demonstration rather than
  asserting the property.
- The final criterion names an exact, runnable command as the gate.
- The no-`tracker-adapters` half is checkable by directory absence, and the
  criteria set is proportionate to a crate with no behaviour.

**Findings**:

- **critical / high** — *The four-operations criterion cannot fail* —
  **Location**: Acceptance Criteria (four port operations)
  Rust trait methods are required by the compiler to have fully-typed
  parameters and return types, so the first half is satisfied automatically by
  any crate that compiles (already asserted by the preceding criterion). The
  second half asks that `fetch_all()`'s shape be "stated" — but the work item
  never states it. Requirements give sketch signatures for `create`, `update`
  and `show` and pointedly leave `fetch_all()` with none, while asserting
  "leaving it open is how a frozen signature stops being frozen".
  **Impact**: The criterion guarding the contract has zero discriminating
  power — a verifier cannot fail any implementation, and whichever shape the
  implementer picks (`Vec<RemoteIssue>`, a map keyed by `ExternalId`, an
  iterator, a paginated cursor) becomes the frozen contract by default rather
  than by decision. This is the exact defect the Drafting Notes claim is "fixed
  here rather than carried over" from 0194.
  **Suggestion**: Write the four signatures into Requirements verbatim,
  including `fetch_all()`'s return type and its keying (e.g.
  `fn fetch_all(&self) -> Result<Vec<(ExternalId, RemoteIssue)>, Error>`), and
  restate the criterion as a comparison against them.

- **major / medium** — *Sync-vs-async, receiver form and dyn-compatibility are
  pinned by no criterion* — **Location**: Acceptance Criteria / Requirements
  0194 requires the active client to be selected at the work binary's
  composition root per the `work.integration` config key and faked in tests — a
  runtime selection that typically needs `Box<dyn RemoteTracker>` — and 0171's
  clients perform HTTP, which forces the sync-versus-async question.
  **Impact**: A signature that compiles but is unusable by a runtime-selected
  composition root, or that must flip between blocking and async once a real
  HTTP client lands, breaks the freeze after 0171 has begun.
  **Suggestion**: State the chosen form in Requirements and add a criterion
  that constructs `Box<dyn RemoteTracker>` from the fake and invokes all four
  operations through it.

- **major / high** — *`RemoteIssue.updated` is ungated and untyped* —
  **Location**: Acceptance Criteria / Requirements
  No criterion mentions `updated`, `remote_updated_at`, or their
  correspondence; the seven criteria cover crate existence, signature
  completeness, error classes, narrowness, the `work` dependency, absence of
  logic, and `cli:check`.
  **Impact**: A high-consequence, silent-failure property with nothing
  verifying it.
  **Suggestion**: State the type (the cargo-pup rule permitting only
  `std`/`core`/`alloc` already excludes a third-party datetime type) and add a
  criterion asserting the type plus the documented baseline correspondence.

- **major / high** — *The public-API criterion's pass condition is
  contradictory* — **Location**: Acceptance Criteria (public API)
  A consumer outside the crate cannot match on a type the crate does not
  export, yet AC1 enumerates three types and AC3/AC4 require a fourth.
  **Impact**: Two verifiers can reach opposite verdicts on the same crate.
  **Suggestion**: Decide whether the error is `tracker`-owned or a
  `kernel::Error` reuse, and rewrite the criterion to enumerate the full
  intended export set.

- **major / medium** — *"Carries no behavioural logic" is unmeasurable* —
  **Location**: Acceptance Criteria (no behavioural logic)
  Three examples but no procedure and no boundary; a `Display` impl, a
  `FromStr` for `ExternalId`, an `is_retryable()` helper, or a validating
  constructor could each be argued in or out.
  **Impact**: No verification value for the property the Summary leans on
  hardest.
  **Suggestion**: Replace with mechanical proxies — no dependency other than
  `kernel`; `src/` contains only the trait and the two value types; no unit
  tests asserting behaviour beyond the fake-and-consumer demonstration.

- **major / medium** — *The probe crate and fake have no stated home or runner*
  — **Location**: Acceptance Criteria (probe crate; error classes)
  Only the final criterion names an invocation, scoped to "the new crate"
  (singular), while the sibling no-`work` criterion explicitly ties its
  enforcement to `cli:check`.
  **Impact**: A compile-time guard no task builds can rot or be silently
  excluded, letting a widening reach 0171 and 0194 unflagged.
  **Suggestion**: Name the artefacts' homes (a workspace member
  `cli/tracker-probe`, or a `trybuild` test inside `tracker`) and state that
  they are built and run by `mise run cli:check`.

- **major / medium** — *The signature-narrowness criterion produces both false
  failures and false passes* — **Location**: Acceptance Criteria (signature
  narrowness)
  The permitted list omits `Result` and containers, so a conforming
  `fetch_all() -> Result<Vec<RemoteIssue>, Error>` reads as a violation; the
  probe-crate mechanism accepts any `std` type, so a widening to `PathBuf`,
  `Duration` or a re-exported `serde_json::Value` would compile and pass.
  **Impact**: The check meant to keep the port narrow cannot settle a dispute
  about whether a signature conforms.
  **Suggestion**: Restate the permitted set to include the intended wrapper and
  container forms, and either say the probe verifies reachability only or add
  the exhaustive signature comparison.

- **minor / medium** — *The "without reference to any provider type" clause is
  vacuously satisfied* — **Location**: Acceptance Criteria (error classes)
  The per-provider client crates are 0171's deliverable and do not yet exist,
  so no consumer could reference a provider type even if the design permitted
  it.
  **Impact**: A reader may take the clause as evidence the provider-leakage
  risk is gated here when it is gated only by the pup rule and the probe crate.
  **Suggestion**: Drop the clause, or convert it into something checkable now —
  e.g. exhaustive match arms over a closed error-class enum.

- **suggestion / medium** — *No criterion exercises the sufficiency Assumptions
  names as the sharpest risk* — **Location**: Assumptions
  The criteria verify that the port compiles, is narrow, and carries error
  classes; none walks a consumer through the operations the way 0194's sync and
  `--push` flows will.
  **Impact**: Insufficiency would surface after 0171's clients are already
  being implemented against the frozen signature.
  **Suggestion**: Add a criterion that a fake plus a consumer stub exercises
  all four operations in the shapes 0194 needs — create-then-`show` round trip,
  whole-content `update`, and a bulk `fetch_all` fed to a caller-side lookup.

## Re-Review (Pass 2) — 2026-08-10

**Verdict:** REVISE

All five lenses re-run against the revised item. Every finding from pass 1
is resolved. Two new criticals were introduced by the revision itself and
have since been fixed in the same pass; what remains are three couplings
that can only be closed by editing 0194 and 0136.

### Previously Identified Issues

- 🔴 **Testability**: The four-operations criterion cannot fail — **Resolved**.
  The verbatim trait block gives the criterion a real oracle, and it now
  also pins the method count and forbids default bodies.
- 🟡 **Clarity / Completeness**: Frozen signature left unstated —
  **Resolved**. All five public items are given verbatim.
- 🟡 **Clarity / Completeness / Scope / Testability**: "Exactly three
  types" vs the port error type — **Resolved**. Five items, and AC1 now
  names a `cargo public-api` snapshot as the check.
- 🟡 **Clarity / Completeness / Scope / Dependency**: Frozen and
  not-frozen at once — **Resolved in 0204**. Retry idempotency is decided
  against the port. See "New Issues" for the 0194 half.
- 🟡 **Completeness / Scope / Testability**: Verification artefacts
  unscoped — **Resolved**. Fake, consumer, signature probe and both
  fixtures are Requirements bullets with criteria and a stated home.
- 🟡 **Completeness / Testability / Dependency**: `RemoteIssue.updated`
  ungated — **Resolved**. Typed as `RemoteTimestamp`, with a round-trip
  criterion plus assertions that no ordering or parsing surface exists.
- 🟡 **Dependency**: "Whole of 0171's wait" — **Resolved**. Narrowed to
  0171's client half; the cutover half stays blocked by 0194.
- 🟡 **Dependency**: Bridge-codes taxonomy dual ownership — **Resolved**.
  The parity fixture is now a requirement and a criterion here.
- 🟡 **Dependency**: Projected-body obligation absent — **Resolved**.
  Stated in Requirements and gated by a doc-comment criterion.
- 🟡 **Testability**: "No behavioural logic" unmeasurable — **Resolved**.
  Restated as inspectable facts about `src/` contents.
- 🔵 All pass-1 minors and suggestions — **Resolved**, except the epic
  0136 decomposition (see below).

### New Issues Introduced (and fixed within this pass)

- 🔴 **Clarity / Testability**: `TrackerError` was specified as both
  "closed" and `#[non_exhaustive]` — opposites. `#[non_exhaustive]` exists
  to let variants be added *without* breaking consumers, and it makes the
  wildcard-free match the criterion demanded impossible for a
  `tracker/tests/` consumer. **Fixed**: attribute dropped; the criterion
  now requires a match with no wildcard arm.
- 🔴 **Completeness**: `ExternalId` was named as a frozen public item but
  never defined, so the freeze did not cover the type carrying remote
  identity. **Fixed**: all five items given verbatim with derives,
  constructors and accessors.
- 🟡 **Testability**: Enforcement was attributed to `cli:check`, which
  runs workspace rustfmt and clippy only — cargo-pup is the separate
  nightly `pup:check` lane. **Fixed**: criteria name `pup:check` and
  `deny:check`, and require the rule be demonstrated rather than asserted.
- 🟡 **Clarity / Scope**: A Requirements bullet assigned the pending-push
  marker into 0194's crate. **Fixed**: recast as the negative scope
  statement this item can honour, with the handoff recorded in
  Dependencies.
- 🔵 **Clarity / Completeness**: `kernel`'s use was never stated. **Fixed**:
  `tracker` takes no dependency at all; the pup allowance is headroom.
- 🔵 **Dependency**: The `last-sync.json` fixture had no provenance and
  appeared to depend on 0194. **Fixed**: captured and committed here as a
  single opaque string needing no credentials.

### Remaining — outside this file

- 🟡 **Dependency / Scope**: 0194's first Requirements bullet still
  carries the pre-split protocol ("that surface lands in 0204"),
  contradicting the freeze recorded here. The two halves of the split
  license opposite responses to the same event. 0204 now names the
  collision; closing it needs the 0194 edit.
- 🟡 **Dependency / Scope**: Epic 0136 lists 0204 in neither its
  Decomposition, its Children list nor its completion criterion, and still
  annotates 0194 as preceding 0171's client adapters.
- 🔵 **Clarity**: 0194 still describes 0204 as "a trait, two value types
  and a lint rule", omitting the error type.

### Assessment

0204 itself is now implementation-ready: the contract is stated verbatim,
every requirement has a falsifiable criterion naming a real gate, and the
scope boundary is drawn mechanically. The verdict stays REVISE only
because three cross-item edits remain — 0194's freeze protocol, 0194's
description of this crate, and 0136's decomposition. None of them requires
further change to 0204, and none blocks starting work on it; they are
graph-consistency repairs in neighbouring documents.

## Re-Review (Pass 3) — 2026-08-10

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Dependency / Scope**: 0194 carried the pre-split protocol ("that
  surface lands in 0204"), contradicting the freeze — **Resolved**. 0194's
  Requirements now treat the signature as frozen at 0204's acceptance and
  route unmet surface needs into a new additive port item. The pending-push
  marker was added there as an explicit requirement, so the mechanism is
  recorded where it will be built.
- 🟡 **Dependency / Scope**: Epic 0136 listed 0204 nowhere and still
  annotated 0194 as preceding 0171's client adapters — **Resolved**. 0204
  is in the Decomposition, the completion criterion and the Children list;
  the 0194 annotation now reads "precedes 0171's cutover half". The
  Children list was also missing 0194–0197, corrected at the same time.
- 🔵 **Clarity**: 0194 described 0204 as "a trait, two value types and a
  lint rule" — **Resolved** in both places. 0171 carried the same stale
  description and was corrected too.

### New Issues Introduced

None. A grep for the stale phrases across `meta/work/` returns nothing.

### Assessment

0204 is approved and ready for planning. The contract is stated verbatim
— five public items with their derives, fields, variants and inherent
methods — and every requirement has a falsifiable criterion naming a real
gate (`cargo public-api` snapshot, `pup:check`, `deny:check`, a
wildcard-free match, a byte-identical round trip). The scope boundary is
mechanical rather than argued, and the four items sharing the port's
dependency graph (0136, 0171, 0194, 0204) now agree on the edges between
them and on the protocol for changing the frozen signature.

Note for whoever plans this: the item is approved as a *specification*.
Nothing has been implemented — the `tracker` crate does not exist yet.
