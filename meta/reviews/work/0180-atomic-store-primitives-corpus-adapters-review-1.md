---
type: work-item-review
id: "0180-atomic-store-primitives-corpus-adapters-review-1"
title: "Work Item Review: Atomic-Store Primitives in corpus-adapters"
date: "2026-07-18T20:00:45+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0180"
work_item_id: "0180"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-07-18T23:08:55+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Atomic-Store Primitives in corpus-adapters

**Verdict:** REVISE

This is a strong, densely-specified task: every section is present and
substantive, the three primitives are named identically across Summary,
Requirements, and Acceptance Criteria, and the subtle concurrency semantics are
explained in context rather than assumed. It falls to REVISE on four major
findings that cluster around two themes — the unresolved bash↔Rust escaper
byte-parity question leaves both the deliverable's boundary and its definition
of done unfixed (four lenses converge here), and two load-bearing behaviours
(the 300 s lock ceiling / back-off shape, and cross-implementation JSONL parity)
have no acceptance criterion. A self-contradiction over `tempfile` vs `std::fs`
in the Technical Notes rounds out the majors.

### Cross-Cutting Themes

- **Escaper byte-parity / on-disk compatibility is unresolved and under-covered**
  (flagged by: completeness, dependency, scope, testability) — Open Question 1
  admits two materially different deliverables (a self-consistent Rust escaper
  vs. one byte-compatible with records already written by `jsonl-common.sh`).
  The work item itself calls the latter "a stronger constraint than mere
  self-consistency" and "the single most important parity requirement," yet AC-4
  only tests a Rust→Rust round-trip. This one open question drives the
  completeness, scope, and testability majors/findings simultaneously: it leaves
  the M sizing uncertain, the definition of done incomplete, and the
  highest-risk failure mode (silent remove-by-key mismatch on a bash-written
  record the visualiser reads) unverified.

- **`atomic_write` temp-handling has contradictory guidance** (flagged by:
  clarity, dependency) — one Technical Note instructs `NamedTempFile::new_in`
  (the `tempfile` crate) while the Open Question and two other notes recommend
  avoiding `tempfile` in favour of `std::fs`. The document does not carry a
  single coherent intent on a decision it also flags as open.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: No criterion verifies bash-to-Rust on-disk escaper parity
  **Location**: Acceptance Criteria
  The Technical Notes call byte-identical escaper behaviour "the single most
  important parity requirement," and Open Question 1 confirms bash- and
  Rust-written JSONL may coexist on disk and be read by the visualiser, yet AC-4
  only verifies a Rust→Rust round-trip. The highest-risk failure mode has no
  acceptance test.

- 🟡 **Testability**: 300 s ceiling and back-off shape are required but not tested
  **Location**: Acceptance Criteria
  The Requirements name the 300 s acquisition ceiling and jittered back-off and
  the Technical Notes specify a shape to "preserve exactly," but no AC verifies
  that acquisition against a permanently-held lock terminates with a timeout
  error, nor the writability pre-check early-error path. AC-3 covers dead-holder
  reclaim but not the give-up-after-ceiling path.

- 🟡 **Dependency**: JSONL canonical-order contract has named consumers but no
  Blocks entry
  **Location**: Dependencies
  The task introduces a shared JSONL contract and `atomic_write`/lock port whose
  downstream consumers are named in Technical Notes (migrate session log / 0172,
  work-item-sync / 0170, the visualiser reading canonical field order), but the
  Dependencies section lists no Blocks entry and frontmatter has no `blocks`
  field — the coupling is captured only transitively via parent 0166.

- 🟡 **Clarity**: Contradictory `tempfile` vs `std::fs` guidance for `atomic_write`
  **Location**: Technical Notes / Open Questions
  The "Same-directory temp invariant" note instructs "use `NamedTempFile::new_in`"
  (the `tempfile` crate), while the in-workspace-precedents note and Open
  Question 2 say to prefer the `std::fs` (no `tempfile`) path. A reader taking
  the first note at face value would adopt the very dependency the others say to
  avoid.

#### Minor

- 🔵 **Dependency**: New `libc` direct dependency couples to cargo-deny ban-list
  activation
  **Location**: Technical Notes
  The dependency decision prefers `libc::kill` as a new direct dep and notes deps
  are "subject to the cargo-deny ban-lists 0162/0166 activate" — an ordering
  coupling (the `cli/deny.toml` allow-list must admit `libc`) not surfaced in
  Dependencies.

- 🔵 **Clarity**: "Model 1" referenced but never defined
  **Location**: Technical Notes: Port-trait design
  The Port-trait design note references "Model 1" without a gloss, and neither
  referenced document defines it (0166 also uses the bare label). A reader can't
  tell what composition-root constraint it imposes without an external source.

- 🔵 **Clarity**: "Port to spec intent" is ambiguous
  **Location**: Technical Notes: Bash-isms that fall away
  The validation-nuance note says `proposed_value` "is documented required ...
  but omitted from the emptiness check ... — port to spec intent," without
  stating which behaviour to port (enforce the documented requirement vs.
  preserve the shipped unchecked behaviour).

- 🔵 **Testability**: AC-1 is an existence checklist redundant with AC-2/3/4
  **Location**: Acceptance Criteria
  AC-1 restates the Summary/Requirements as a feature inventory; each clause is
  verified in more specific form by AC-2/3/4. On its own it can be argued met
  merely because the functions exist.

- 🔵 **Testability**: AC-2 interruption invariant lacks a stated verification seam
  **Location**: Acceptance Criteria
  AC-2's outcome is well-defined, but the criterion gives no deterministic way to
  induce the mid-write interruption (no fault-injection seam between temp-write
  and rename), risking treatment as aspirational.

#### Suggestions

- 🔵 **Completeness**: Definition of done may not cover the escaper-compatibility
  case the item flags
  **Location**: Acceptance Criteria
  Once Open Question 1 resolves, fold its outcome into the acceptance criteria —
  either an explicit bash-written-record match criterion or a note that a clean
  cutover puts it out of scope.

- 🔵 **Scope**: Escaper-compatibility question leaves the deliverable's boundary
  unfixed
  **Location**: Open Questions
  Cross-implementation byte-parity plus a differential corpus is a meaningfully
  larger effort than a fresh self-consistent port, so the M sizing may not hold
  depending on the answer. Resolve before scheduling.

- 🔵 **Testability**: AC-4 "single shared escaper" states implementation, not
  outcome
  **Location**: Acceptance Criteria
  The verifiable intent (adversarial round-trip) is already captured; whether one
  or two escapers are used is only observable through that round-trip. Demote the
  "single shared escaper" clause to Requirements/Technical Notes.

### Strengths

- ✅ Subtle domain terms (mkdir-lock, PID-owner reclaim, jittered back-off,
  anchored-prefix match, escape parity) are each explained in context, so a
  reader who knows Rust but not the bash internals can follow them.
- ✅ Scope is consistent across Summary, Requirements, and Acceptance Criteria —
  the same three primitives are named identically with no drift, and the three
  form one genuinely coupled atomic-store capability (the JSONL remover depends
  on the writer's canonical field order).
- ✅ Every section is present and densely populated (Context explains *why* the
  work is subtle rather than restating the Summary; Open Questions, Dependencies,
  and Assumptions carry substantive content, not placeholders).
- ✅ The one hard upstream blocker (0179) is captured bidirectionally — in
  `blocked_by` frontmatter, the Dependencies section, and confirmed reciprocal on
  0179 per the Drafting Notes.
- ✅ AC-3 (dead-holder reclaim) and AC-4 (adversarial escape round-trip) name
  observable outcomes and enumerable inputs, giving a tester a clear pass/fail
  procedure; the clean carve-out from parent 0166 with no overlap against sibling
  0179 makes it a well-bounded unit.
- ✅ Out-of-scope boundaries are stated explicitly (the server's async
  `tokio::Mutex`/etag layer and the vcs probe are named as excluded).

### Recommended Changes

1. **Resolve Open Question 1 (on-disk escaper compatibility) before scheduling**
   (addresses: escaper parity theme — completeness, scope, testability, and the
   testability major on bash-to-Rust parity)
   Decide whether the Rust escaper must be byte-compatible with records already
   written by `jsonl-common.sh` (coexistence) or whether a clean cutover applies.
   This single decision fixes the M sizing, the definition of done, and whether
   a differential parity test is required.

2. **Add acceptance criteria for the resolved parity outcome** (addresses:
   Testability "No criterion verifies bash-to-Rust on-disk escaper parity";
   Completeness suggestion)
   Either require a differential test (remove-by-key on a bash-produced record
   matches the Rust-computed prefix, gated by the existing `bash-parity` feature)
   or explicitly scope byte-parity out with a stated clean-cutover rationale.

3. **Add acceptance criteria for the lock ceiling and back-off** (addresses:
   Testability "300 s ceiling and back-off shape are required but not tested")
   Add an AC that acquisition against a live-held lock returns a lock-timeout
   error bounded by the ceiling — with the ceiling injectable so the test does
   not run for 300 s — plus one covering the not-writable early-error path.

4. **Reconcile the `tempfile` vs `std::fs` guidance** (addresses: Clarity
   "Contradictory `tempfile` vs `std::fs` guidance")
   Reword the "Same-directory temp invariant" note so the `NamedTempFile::new_in`
   mention is explicitly the contested option (or illustrates the same-dir-temp
   *shape*), and align the Open Question 2 framing with it.

5. **Surface the downstream JSONL/atomic-store couplings** (addresses: Dependency
   "JSONL canonical-order contract has named consumers but no Blocks entry"; the
   `libc`/cargo-deny minor)
   Add explicit Blocks entries (or a Dependencies note) naming the downstream
   consumers — at minimum 0172 (migrate session log) and the visualiser/0168
   refactor — and note that landing the `libc` direct dep requires the
   `cli/deny.toml` allow-list to admit it.

6. **Tidy the acceptance criteria and clarify the two minor clarity points**
   (addresses: Testability AC-1 redundancy, AC-2 seam, AC-4 "single shared
   escaper"; Clarity "Model 1" and "spec intent")
   Reframe or drop AC-1 as an umbrella; note the fault-injection seam for AC-2;
   demote AC-4's "single shared escaper" clause to design guidance. Gloss
   "Model 1" on first use and replace "port to spec intent" with the concrete
   target behaviour for `proposed_value`.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually precise for a concurrency port: it
defines each subtle primitive progressively, keeps scope coherent across Summary,
Requirements, and Acceptance Criteria, and poses its two Open Questions
unambiguously. The main clarity defect is an internal contradiction in the
Technical Notes over whether `atomic_write` should use the `tempfile` crate or
plain `std::fs`. A couple of undefined-jargon and ambiguous-instruction points
are minor.

**Strengths**:
- Subtle domain terms are each explained in context rather than assumed.
- Scope is consistent across Summary, Requirements, and Acceptance Criteria.
- The two Open Questions are precisely framed, each stating competing options and
  the ruling needed.

**Findings**:
- 🟡 **major** (high) — Technical Notes / Open Questions — The Technical Notes
  contradict themselves on `atomic_write`'s temp handling: the "Same-directory
  temp invariant" note instructs `NamedTempFile::new_in` (the `tempfile` crate)
  while the "In-workspace sync atomic-write precedents" note and Open Question 2
  recommend the `std::fs` (no `tempfile`) path. A reader taking the first note at
  face value adopts the very dependency the others say to avoid.
- 🔵 **minor** (medium) — Technical Notes: Port-trait design — "Model 1" is
  referenced as a named design without definition; neither 0166 nor ADR-0053
  defines it.
- 🔵 **minor** (medium) — Technical Notes: Bash-isms that fall away — "port to
  spec intent" for `proposed_value` is ambiguous: the documented spec says
  required while the code doesn't check it, and the phrase doesn't state which to
  port.

### Completeness

**Summary**: This task-kind work item is exceptionally complete: all sections are
present and densely populated. The Summary states a clear action, the Context
explains the motivating forces, and four specific acceptance criteria define
done. Frontmatter is intact with a recognised kind and appropriate draft status.

**Strengths**:
- Summary is an unambiguous action statement naming exactly what is ported.
- Context genuinely explains why the work exists and why it is subtle.
- Four specific, concrete acceptance criteria — well beyond the two-criterion
  floor.
- Open Questions, Dependencies, and Assumptions are all substantively populated.
- Frontmatter is complete and coherent.
- Requirements map one-to-one onto the three primitives, detailed enough to begin
  without follow-up.

**Findings**:
- 🔵 **suggestion** (low) — Acceptance Criteria — The first Open Question flags a
  potentially stronger requirement (byte-compatibility with bash-written records)
  yet the acceptance criteria only assert a self-consistent Rust round-trip. Once
  the open question resolves, fold its outcome into the acceptance criteria.

### Dependency

**Summary**: Upstream blocking is cleanly captured (0179 in both frontmatter and
Dependencies, reciprocal edge confirmed). The main gap is downstream: the task
delivers a shared JSONL contract and `atomic_write`/lock port with named Rust
consumers, yet carries no Blocks entry, so that coupling is captured only
transitively via parent 0166. The escaper-coexistence coupling is well surfaced
as an Open Question.

**Strengths**:
- The one hard upstream blocker (0179) is captured bidirectionally.
- The cross-component coexistence coupling (bash/Rust JSONL sharing one on-disk
  format read by the visualiser) is explicitly an Open Question.
- Technical Notes enumerate the concrete consuming surfaces of `atomic_write` and
  the JSONL primitives.

**Findings**:
- 🟡 **major** (medium) — Dependencies — The task introduces a shared JSONL
  contract and `atomic_write`/lock port with named downstream consumers (0172
  migrate session log, 0170 work, the visualiser), but lists no Blocks entry and
  has no `blocks` frontmatter — captured only transitively via 0166. Add explicit
  Blocks entries mirroring the reciprocal edges used with 0179.
- 🔵 **minor** (low) — Technical Notes — The dependency decision prefers a new
  `libc` direct dep and notes deps are subject to the cargo-deny ban-lists
  0162/0166 activate; the ordering coupling (the `cli/deny.toml` allow-list must
  admit `libc`) is not called out in Dependencies.

### Scope

**Summary**: 0180 is a tightly-bounded, coherent unit: it ports the three
atomic-store primitives into a single crate behind one driven-port trait. Summary,
Requirements, and Acceptance Criteria describe the same three-primitive scope; it
is a clean carve-out of the atomic-store portion of 0166 with no overlap against
0179. Sizing (M) is appropriate. The only soft edge is the unresolved
bash-byte-parity question that materially changes the deliverable size.

**Strengths**:
- The three primitives form one cohesive atomic-store capability; the JSONL
  remover depends on the writer's canonical field order, so splitting would
  fracture a coupled unit.
- Section alignment is strong with no scope drift.
- Clean decomposition boundary against parent 0166 and sibling 0179.
- Single crate, single team, single increment, with in/out-of-scope stated
  explicitly.

**Findings**:
- 🔵 **suggestion** (medium) — Open Questions — The first Open Question admits two
  materially different deliverables (self-consistent Rust escaper vs.
  byte-compatible with bash-written records). Cross-implementation byte-parity
  plus a differential corpus is a larger effort than a fresh self-consistent
  port, so M sizing may not hold. Resolve before scheduling.

### Testability

**Summary**: The Acceptance Criteria are mostly concrete and verifiable — AC-2,
AC-3, and AC-4 each name observable outcomes and (for AC-4) enumerable adversarial
inputs. The main gaps are omissions: the most heavily-emphasised risk (bash↔Rust
escaper byte-parity) and two named behaviours (the 300 s ceiling and exact
back-off shape) have no acceptance criterion.

**Strengths**:
- AC-3 specifies the dead-holder reclaim test with both a positive and negative
  condition.
- AC-4 enumerates the exact adversarial input classes the round-trip must survive.
- AC-2 states a clear atomicity invariant with observable outcomes.

**Findings**:
- 🟡 **major** (high) — Acceptance Criteria — The Technical Notes call
  byte-identical escaper behaviour "the single most important parity requirement"
  and Open Question 1 confirms coexistence, yet AC-4 only verifies a Rust→Rust
  round-trip. The highest-risk failure mode (silent remove-by-key mismatch on a
  bash-written record) has no acceptance test. Add a differential test or
  explicitly scope byte-parity out.
- 🟡 **major** (high) — Acceptance Criteria — The 300 s ceiling and back-off shape
  are named in Requirements/Technical Notes but no AC verifies that acquisition
  against a permanently-held lock terminates with a timeout, nor the
  writability-precheck early error. Add an AC (with an injectable ceiling) for the
  give-up path and the not-writable path.
- 🔵 **minor** (medium) — Acceptance Criteria — AC-1 is an existence checklist
  redundant with AC-2/3/4; on its own it can be argued met merely because the
  functions exist. Drop or reframe as an umbrella.
- 🔵 **minor** (medium) — Acceptance Criteria — AC-2's interruption invariant
  lacks a stated verification seam; a real interruption can't be reliably
  reproduced without a fault-injection seam between temp-write and rename.
- 🔵 **suggestion** (low) — Acceptance Criteria — AC-4's "single shared escaper"
  clause states an implementation choice, not an observable outcome; the
  round-trip already captures the verifiable intent. Demote to
  Requirements/Technical Notes.

## Re-Review (Pass 2) — 2026-07-18

**Verdict:** REVISE

The four majors from Pass 1 are all resolved or downgraded, and every Pass-1
minor was addressed. The item improved materially. It stays at REVISE because
the re-review surfaced three finer-grained majors — two are direct consequences
of this pass's edits (scoping out bash-parity sharpened an unverified
field-order gap; the clean-cutover ruling introduced an uncaptured constraint on
the migrate consumer), and one is a load-bearing lock invariant that was always
under-tested. All three are targeted and cheap to close.

### Previously Identified Issues

- 🟡 **Testability**: No criterion verifies bash-to-Rust escaper parity —
  **Resolved** (clean-cutover ruling; AC scopes byte-parity out with rationale).
- 🟡 **Testability**: 300 s ceiling and back-off shape not tested — **Partially
  resolved** (ceiling + not-writable now have an AC; the exact back-off *shape* —
  doubling/cap/jitter — remains unverified, now a minor).
- 🟡 **Dependency**: JSONL contract has named consumers but no Blocks entry —
  **Resolved** (consumers enumerated in Dependencies prose; residual question of
  frontmatter `blocks` edges downgraded to a suggestion).
- 🟡 **Clarity**: Contradictory `tempfile` vs `std::fs` guidance — **Resolved**
  (ruling recorded; all three Technical Notes now consistent on `tempfile`).
- 🔵 **Dependency**: `libc`/cargo-deny coupling — **Resolved** (captured in
  Dependencies + Technical Notes).
- 🔵 **Clarity**: "Model 1" undefined — **Resolved** (glossed on first use).
- 🔵 **Clarity**: "port to spec intent" ambiguous — **Resolved** (made concrete:
  `proposed_value` required, empty rejected; testability now wants an AC for it —
  see New Issues).
- 🔵 **Testability**: AC-1 existence checklist — **Resolved** (reframed as
  umbrella).
- 🔵 **Testability**: AC-2 lacks a verification seam — **Resolved**
  (fault-injection seam added).
- 🔵 **Testability**: AC-4 "single shared escaper" states implementation —
  **Resolved** (round-trip kept; guidance demoted).

### New Issues Introduced

- 🟡 **Testability** (high): Canonical field order has no verifying criterion —
  the round-trip is self-referential and passes even if the emitted field order
  diverges from the contract the visualiser reads. Add a golden-record byte
  assertion (`transformation_key` first, `schema_version` second, remaining
  fields in defined order). Sharpened by scoping out bash-parity this pass.
- 🟡 **Dependency** (medium): The clean-cutover ruling assumes no file is written
  by both bash and Rust concurrently, but that precondition rests on the migrate
  consumer (0172) performing an atomic cutover — a constraint stated as a given,
  not captured. Record it as an explicit constraint/assumption on 0172.
- 🟡 **Testability** (medium): The mid-acquisition "empty/absent `owner` file =
  live" reclaim window (flagged load-bearing in Technical Notes) is not covered —
  AC-3 tests only dead-holder-gone and live-owner-held. Add an AC that an
  empty/absent `owner` sentinel is treated as held, not reclaimed.
- 🔵 **Clarity + Completeness** (minor): Inserting the umbrella criterion left the
  "AC-4" references (Open Questions, Drafting Notes) stale — the round-trip is now
  the fifth bullet. Number the ACs explicitly (AC-1…AC-5) or reference by name.
- 🔵 **Testability** (minor): The exact back-off shape ("preserve exactly") has no
  verifying criterion beyond the ceiling bound — either gate it over a
  seam-observable delay sequence or state explicitly that timing is intentionally
  not gated.
- 🔵 **Testability** (minor): The deliberate `proposed_value`-required behavioural
  change has no AC — add one (empty `proposed_value` → validation error).
- 🔵 **Suggestions**: direct `blocks` edges vs. parent-level capture (dependency);
  the three primitives could split if JSONL parity dominates effort (scope, "no
  change likely warranted"); AC-2's `tempfile::NamedTempFile::new_in` mention is
  an implementation instruction inside an outcome (testability).

### Assessment

The work item is close. The Pass-1 blockers are genuinely cleared; what remains
is a tighter band of testability gaps around verifying the *contract* (field
order) and the *subtle lock states* (empty-owner window, back-off shape), plus
one dependency constraint the clean-cutover ruling created and a cheap AC-numbering
cleanup. None requires a design decision — they are additive acceptance criteria
and one Dependencies note. Addressing them would bring the item to APPROVE.

## Re-Review (Pass 3) — 2026-07-18

**Verdict:** COMMENT

The work item is acceptable — it dropped below the REVISE threshold (one major,
threshold is two). Both Pass-2 testability majors are resolved (AC-7 golden
field-order assertion, AC-4 empty-owner window), the AC-numbering regression is
fixed (AC-1…AC-8), and the back-off-shape and `proposed_value` minors are
addressed. One dependency major persists in a sharper form: recording the
clean-cutover obligation in 0180's prose does not propagate it onto 0172, so a
forward edge is still wanted. The rest are polish-grade minors and suggestions.

### Previously Identified Issues

- 🟡 **Testability**: Canonical field order not verified (self-referential
  round-trip) — **Resolved** (AC-7 golden-record assertion added; residual minor:
  AC-7 doesn't enumerate the remaining-field order — see New Issues).
- 🟡 **Testability**: Empty/absent-`owner` reclaim window uncovered — **Resolved**
  (AC-4 added).
- 🟡 **Dependency**: Clean-cutover constraint on 0172 uncaptured — **Partially
  resolved** (now stated in 0180's Dependencies, but recorded only on this side;
  no edge propagates it to 0172 — re-flagged major below).
- 🔵 **Clarity/Completeness**: Stale "AC-4" references / unnumbered ACs —
  **Resolved** (ACs numbered AC-1…AC-8; references corrected).
- 🔵 **Testability**: Back-off shape unverified — **Partially resolved** (noted as
  ungated design intent in AC-5; testability observes it *is* deterministically
  checkable via a fake sleep port — now a suggestion).
- 🔵 **Testability**: `proposed_value` required had no AC — **Resolved** (AC-8
  added; residual minor: AC-8 covers empty but not absent — see New Issues).

### New Issues Introduced

- 🟡 **Dependency** (high): The clean-cutover obligation delegated to 0172 is
  recorded only in 0180. With no `relates_to`/`blocks` edge or action item on
  0172, nothing ensures 0172's own record captures the constraint — if 0172 is
  planned without it, the byte-parity scope-out becomes silently unsafe. Add a
  forward edge to 0172.
- 🔵 **Clarity + Completeness + Scope** (minor/suggestion): AC-8's
  `proposed_value` validation is not named in Summary/Requirements and reads as a
  behaviour beyond a strict port. Add a one-line Requirements bullet and note the
  deliberate port-to-documented-spec divergence.
- 🔵 **Testability** (minor): AC-7 references a canonical field order it never
  enumerates — a verifier must reverse-engineer it from `jsonl_compose_record`.
  Enumerate the order (or cite the exact `jsonl-common.sh` lines) in AC-7.
- 🔵 **Testability** (minor): AC-8 verifies only the *empty* case of a
  "required and non-empty" rule; an *absent* `proposed_value` key is uncovered.
  Extend AC-8 to reject a missing key too.
- 🔵 **Testability** (suggestion): The back-off shape is deterministically
  testable via a fake sleep port (recording requested durations) without
  wall-clock flakiness — consider an optional criterion asserting the
  doubling/cap/jitter-range.
- 🔵 **Clarity** (minor): `AtomicStore`/`RecordStore` slash naming leaves the port
  count ambiguous (one trait or two?); the "StoreError location is a live
  decision" note contradicts Open Questions reporting all resolved; the
  `atomic-common.sh:105-247` header range excludes the `atomic_write` (`:16-32`)
  it heads; "twin extraction" is used without a gloss.
- 🔵 **Dependency** (minor): Downstream consumers (0172/0170/0168) remain
  prose-only, not `blocks` edges — recurring across passes; either add the edges
  or record that story-level capture is the deliberate choice.

### Assessment

The item is ready to plan against as-is; none of the remaining items block
implementation. The single highest-value follow-up is propagating the
clean-cutover obligation onto 0172 (a `relates_to` edge or an action item), since
that constraint is load-bearing for the byte-parity scope-out and currently lives
only on the producer side. The remaining minors are low-cost polish — enumerating
AC-7's field order, extending AC-8 to the absent-key case, naming the
`proposed_value` validation in Requirements, and resolving the small clarity slips
— that would carry the item to a clean APPROVE but are not prerequisites.

## Approval (Pass 4) — 2026-07-18

**Verdict:** APPROVE

The one remaining major from Pass 3 is resolved: the clean-cutover obligation is
now propagated onto 0172 via a reciprocal `relates_to` edge and a note in 0172's
Dependencies, so the byte-parity scope-out no longer rests on a producer-side-only
assertion. The two key Pass-3 minors were also closed — AC-7 now enumerates the
full canonical field order against `jsonl-common.sh:129-148`, AC-8 covers both the
empty and absent `proposed_value` cases, and the validation is named as a
Requirements bullet (port-to-documented-spec). No findings at or above the major
threshold remain. The review closes APPROVE across all five lenses; the work item
is ready for implementation planning.

Remaining open items are optional polish only, deferred by the author and not
blocking: the `AtomicStore`/`RecordStore` port-count naming, the "StoreError
location is a live decision" note vs. Open Questions, the `atomic-common.sh`
header line-range, a "twin" gloss, an optional fake-sleep back-off-shape
criterion, and whether downstream consumers should be direct `blocks` edges vs.
story-level capture.
