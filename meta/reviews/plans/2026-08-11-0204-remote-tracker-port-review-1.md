---
type: plan-review
id: "2026-08-11-0204-remote-tracker-port-review-1"
title: "Plan Review: RemoteTracker Port Implementation Plan"
date: "2026-08-11T16:22:44+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0204"
target: "plan:2026-08-11-0204-remote-tracker-port"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, test-coverage, code-quality, compatibility, usability, documentation, standards]
review_number: 1
review_pass: 6
tags: [rust, tracker, sync, port, cargo-pup]
last_updated: "2026-08-11T22:57:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: RemoteTracker Port Implementation Plan

**Verdict:** REVISE

The plan is strong where it matters most for a contract crate: the boundary is
drawn correctly and enforced mechanically, the doc comments teach the
non-obvious obligations rather than restating signatures, and the `FetchOutcome`
deviation is a real catch made before either consumer started. But the same
reasoning stopped one step short — `FetchOutcome.found` freezes a mandatory
`body` into a bulk read that neither provider can populate and that the bash
contract deliberately omits, so the frozen surface is not satisfiable as
written. Compounding that, the three mechanisms meant to hold the freeze are all
weaker than claimed: the surface golden cannot see trait methods or derives, the
flagship port test does not compile, and the parity test never touches
`TrackerError`. Four errors of fact about the codebase (pup rule shapes,
`must_use_candidate`, the `pup:check` lane, the already-applied 0204 edits) would
be committed as comments or plan prose.

### Cross-Cutting Themes

- **The bulk read demands a body that cannot exist** (flagged by: architecture,
  compatibility, usability, correctness) — four lenses independently reached
  the same conclusion by different routes. `work-item-fetch-remote.sh:20-35`
  normalises the bulk contract to `{ "found": { "<key>": { "updated": … } } }`
  with no body, and reserves `show` for "the per-item full-fidelity read
  returning the issue's body … (the genuinely-changed minority)". Both adapters
  build `{updated: …}` only. This is the same class of defect Deviation 1 was
  raised to fix, and it can equally make the port produce a wrong answer.

- **The freeze mechanisms do not deliver the freeze** (flagged by: correctness,
  test-coverage, code-quality, compatibility, usability) — `surface.rs` filters
  on `line.starts_with("pub ")`, but trait methods carry no `pub` keyword and
  `#[derive(...)]` lines do not either. So the golden pins neither the four
  operations nor any derive. A fifth method *with a default body* passes both
  the golden and the fake. AC 2 and the plan's Desired End State both claim the
  opposite.

- **Sample code that has not been compiled** (flagged by: correctness,
  test-coverage, code-quality, compatibility, standards) — `port.rs`'s
  `assert_eq!` over `Result<String, TrackerError>` requires
  `TrackerError: PartialEq`, which the freeze omits; the `expect_used` allow is
  deferred when the lint set makes it mandatory; several lines exceed 80
  columns.

- **Tests that assert less than their names promise** (flagged by:
  test-coverage, code-quality, correctness) — the dispatch-parity test never
  references `TrackerError`, the timestamp round-trip passes vacuously on an
  empty fixture, the pup probe's compliant control has no imports so the permit
  list is never exercised, and no test ever receives an `Err` through the port.

- **Errors of fact that would be committed** (flagged by: architecture,
  correctness, test-coverage, compatibility, standards, documentation) — the
  four domain pup rules are *not* shape-identical (`work` permits `^corpus`,
  `migrate` permits `^corpus` and `^document`); `must_use_candidate` is
  `allow`, not enforcing; the twelve probe invocations land on
  `test:integration:pup`, not `pup:check`; and the 0204 edits the Deviations
  section demands are already applied.

### Tradeoff Analysis

- **Crate emptiness (AC 9) vs seam safety**: architecture and correctness both
  observe that the partition-totality invariant — the one the plan calls
  load-bearing — is unenforceable precisely because AC 9 forbids the smart
  constructor that would make the wrong answer unrepresentable. The plan should
  either relax AC 9 for one narrow choke point
  (`FetchOutcome::partition(requested, found, retrieval_was_complete)`), or pull
  0194's parameterised contract test forward, or state the trade-off explicitly.
  Recommendation: state it explicitly and add totality to the 0194 handoff; the
  smart constructor is the better engineering answer but reopens a criterion the
  work item argued for deliberately.

- **Freeze fidelity vs consumer ergonomics on derives**: usability and
  code-quality want `Hash`/`Display` on `ExternalId` and `PartialEq`/`Clone` on
  `TrackerError` (every sibling error type has them; 0194 must join
  `FetchOutcome.found` against local items every run). Compatibility notes the
  freeze protocol makes adding them later a new work item. Recommendation: decide
  now, while the block is already open — the cost of deciding wrong is one
  additive derive; the cost of deferring is a new work item.

- **Phase 3's scope vs the plan's "cheap-to-reach milestone" framing**:
  architecture argues Phase 3 is unrelated infrastructure work; test-coverage and
  standards want it *expanded* to cover the widened rules. Both are right about
  their own concern. Recommendation: split Phase 3 into its own work item, where
  it can be scoped to cover the widenings and the two unprobed adapter rules
  properly, and let 0204 land without it.

### Findings

#### Critical

- 🔴 **Architecture / Compatibility / Usability / Correctness**: `FetchOutcome.found` mandates a body no bulk path can supply
  **Location**: Phase 2, Section 1: The partition type and the trait
  `found: Vec<(ExternalId, RemoteIssue)>` requires a non-optional `body: String`
  for every resolved id, but `work-item-fetch-remote.sh` normalises the bulk
  contract to timestamps only, and Linear's bulk selection set requests no
  `description` at all. A conforming client must either fabricate `String::new()`
  — which by `RemoteIssue.body`'s own doc comment "reclassifies every synced item
  as remotely modified" — or issue ~180 per-item `show` calls, destroying the
  bulk design and colliding with 0194's own acceptance criterion of "zero `show`
  calls" in bulk mode. Fix: `found: Vec<(ExternalId, RemoteTimestamp)>`, or
  `body: Option<String>` documented as never populated by `fetch_all`.

#### Major

- 🟡 **Correctness / Test Coverage / Code Quality / Compatibility / Usability**: the surface golden is blind to trait methods and derives
  **Location**: Phase 2, Section 3: `cli/tracker/tests/surface.rs`
  `declarations()` keeps only lines starting with `pub `. Trait method
  declarations carry no `pub`, and `#[derive(...)]` lines do not either, so the
  golden pins neither `RemoteTracker`'s four operations nor any derive set. AC 2
  ("a fifth operation cannot be added additively without failing the surface
  golden") and AC 7 (`RemoteTimestamp` derives no `PartialOrd`/`Ord`) are both
  unenforced.

- 🟡 **Test Coverage**: the golden is generated from the implementation, so it can never be red
  **Location**: Phase 2, Section 2: The surface golden
  Producing the golden by "running the test once and pasting its actual value"
  makes it a characterisation snapshot of whatever was built, not a pin against
  the block 0204 declares frozen. Hand-write it from the Requirements block
  first, so the test starts red.

- 🟡 **Correctness / Test Coverage / Code Quality / Compatibility**: `port.rs` does not compile
  **Location**: Phase 2, Section 3: `cli/tracker/tests/port.rs`
  `assert_eq!(tracker.show(&id).map(|issue| issue.body), Ok(…))` compares two
  `Result<String, TrackerError>` values, which requires `TrackerError: PartialEq`
  — the freeze specifies `#[derive(Debug)]` alone. The obvious local fix widens
  the frozen surface, and no test would notice (see the golden finding above).
  Every sibling error type in the workspace derives `Clone, PartialEq, Eq`.

- 🟡 **Architecture**: read operations are given a `Terminal` class the bash taxonomy forbids
  **Location**: Phase 2, Section 1: `show` / `fetch_all` doc comments
  `work-item-fetch-remote.sh:44-48` states: "A read mutates nothing, so 71
  (terminal-may-have-mutated) does not apply here — any underlying read failure
  collapses to 70". The plan's `show` doc says the opposite. The parity fixture
  compares only code names and numbers, so this divergence is invisible to the
  artefact meant to hold the port 1:1 against the taxonomy.

- 🟡 **Test Coverage / Code Quality / Correctness**: the parity test never references `TrackerError`
  **Location**: Phase 1, Section 7: `exactly_two_dispatch_codes_map_onto_the_two_error_classes`
  It compares fixture strings against the hardcoded literals
  `vec!["Retryable", "Terminal"]`. Renaming `TrackerError::Retryable` leaves it
  green. AC 6 asks for a 1:1 map onto the *enum's* classes.

- 🟡 **Correctness / Test Coverage**: the pup probe never exercises the permit list
  **Location**: Phase 1, Section 4: The pup probe pair
  `_TRACKER_PORT_COMPLIANT` has no `use` statement, and the real `lib.rs` has
  none either (everything is fully qualified). A mistyped anchor — `^crate($|::)`
  for `^crate(::|$)` — passes the violation probe, the control, and
  `pup:check`. The rule is proven to deny, never proven to allow.

- 🟡 **Architecture / Correctness / Test Coverage**: partition totality is unenforceable and untested against any real implementer
  **Location**: Deviations 1; Phase 2, Section 3
  The totality tests exercise the plan's own fake, which honours the invariant by
  construction. `FetchOutcome` ships as three bare `Vec`s with no constructor,
  and AC 9 forbids one. 0171's clients — the code most likely to violate it —
  are unblocked by 0204 alone, while the parameterised contract test is deferred
  to 0194.

- 🟡 **Test Coverage**: no test exercises an `Err` return through the port
  **Location**: Phase 2, Section 3: `cli/tracker/tests/port.rs`
  Both error branches of `FixedTracker` are dead code; the trait-object test uses
  only happy paths and `is_ok()`. The work item asks for "a fake `RemoteTracker`
  and a consumer exercising both error classes"; `errors.rs` constructs the
  values directly rather than receiving them from a port call.

- 🟡 **Test Coverage**: test-first is asserted but every phase sequences production code before its tests
  **Location**: Implementation Approach; Phase 1 and Phase 2 Changes Required
  Phase 1 lists `src/lib.rs` (§6) before the tests (§7); Phase 2 lists the trait
  (§1) before the port tests (§3). No expected red is recorded for any step. For
  a crate whose entire deliverable is its tests, the ordering *is* the guarantee,
  and code-first sequencing is how a freeze ends up pinning an implementation
  rather than a contract.

- 🟡 **Architecture / Correctness / Test Coverage / Compatibility / Standards / Documentation**: the four domain pup rules are not shape-identical
  **Location**: Phase 3: Pup Probe Backfill
  `work_domain_imports_only_permitted` additionally permits `^corpus(::|$)`
  (`cli/pup.ron:100`); `migrate_domain_imports_only_permitted` additionally
  permits `^corpus(::|$)` and `^document(::|$)` (`cli/pup.ron:181-183`). The
  parameterised writer never imports either, so the two rules carrying
  deliberately-argued widenings get the least coverage — and the claim would be
  committed as a comment.

- 🟡 **Usability**: `body` means two different things on the read and write sides
  **Location**: Phase 2, Section 1: `RemoteIssue.body` vs `create`/`update`'s `body`
  The write-side `body` is the local work-item body; the read-side `body` is the
  *projected* body, which carries the title line and canonicalises Jira's ADF. So
  `create(title, body)` followed by `show` does not return the `body` you sent.
  This is the single most consequential mistake available to a 0171 implementer,
  and the doc delegates the definition to a script 0171 will delete.

- 🟡 **Usability**: the `Terminal` Display message inverts the recovery advice
  **Location**: Phase 1, Section 6: `impl Display for TrackerError`
  `Terminal` renders as "tracker call failed unrecoverably", but `Terminal` means
  *the remote mutation state is unknown* — the bash bridge tells the user "an
  issue may exist — do NOT retry". "Unrecoverably" invites assuming nothing
  happened. This string reaches a user through 0194's report.

- 🟡 **Usability**: `detail: String` is the only diagnostic channel and has no content contract
  **Location**: Phase 1, Section 6: `TrackerError`
  Unlike `RemoteIssue.body`, `detail` gets no doc comment and no stated
  obligation. Two independently written clients will produce
  `"connection refused"` and `"linear-graphql exited 34"`, and 0194 can render
  neither consistently nor say which provider or issue failed.

- 🟡 **Correctness / Usability**: `show` cannot express absence while `fetch_all` can
  **Location**: Phase 2, Section 1: the trait
  A deleted remote issue can only be reported as `Retryable` (retrying never
  succeeds) or `Terminal` (nothing was mutated, and 0194 gives `Terminal` a
  destructive local consequence). Any read path using `show` alone can never
  produce `remote-absent`.

- 🟡 **Compatibility**: the empty `RemoteTimestamp` collapses two unknowns the bash classifier keeps apart
  **Location**: Phase 1, Section 6: `RemoteTimestamp`
  `work-item-sync-classify.sh:177` short-circuits to unchanged only when
  `[ -n "$base_remote_updated" ] && [ "$a" = "$b" ]`, so an empty baseline never
  matches an empty remote stamp. Derived `PartialEq` loses that guard:
  `RemoteTimestamp::new(String::new()) == RemoteTimestamp::new(String::new())`.
  The bulk contract's `"updated": null` is a second, undocumented source of the
  empty value.

- 🟡 **Test Coverage / Code Quality**: the timestamp round-trip test passes vacuously if the fixture shrinks
  **Location**: Phase 1, Section 7: `cli/tracker/tests/vocabulary.rs`
  The test asserts inside a loop over the fixture's lines. An emptied or
  truncated fixture reports green, and nothing asserts both provider shapes are
  present — the exact property AC 7 exists to guarantee.

- 🟡 **Documentation**: the contract's doc-comment obligations are verified in the wrong phase, or not at all
  **Location**: Phase 1 and Phase 2 Manual Verification
  All three doc obligations ship in Phase 1's `lib.rs`, but only
  `RemoteIssue.body` is checked, and it is checked in Phase 2 — while the phases
  are declared independently mergeable. The retryable/terminal asymmetry (which
  the plan calls "the part a client implementer will get wrong") and the
  opaque-`kind` doc are checked nowhere, and no automated test can see doc text.

- 🟡 **Documentation / Test Coverage**: the probe cost is charged to the wrong task
  **Location**: Performance Considerations; Phase 3; Phase 1 Success Criteria
  The twelve invocations are pytest cases under `test:integration:pup`
  (`mise.toml:293`), which is in neither `check` nor `default`. `pup:check`
  (`mise.toml:525`) runs `cargo +nightly pup` over the real workspace and gains
  nothing from Phase 3. The stated mitigation therefore protects a cost that does
  not land where the plan says. Phase 1's "the pup rule is exercised:
  `mise run pup:check`" is misleading for the same reason — a compliant
  `tracker` never trips it.

- 🟡 **Architecture / Compatibility / Documentation**: Deviations and Handoffs describe edits already applied to 0204
  **Location**: Deviations From The Work Item; Handoffs
  0204's Requirements already declare `FetchOutcome` and
  `fetch_all(&self, ids: &[ExternalId])`, AC 1 already says six items and that
  `cargo public-api` is deliberately not used, and the Drafting Notes record the
  reopening. Meanwhile the *downstream* items were not updated: 0194 and 0171
  still describe a five-item port.

- 🟡 **Documentation**: the parity fixture cannot record the reasoning it is required to record
  **Location**: Phase 1, Section 5: `dispatch-codes.txt`
  0204 requires the fixture to record why 72 and 73 resolve above the port. The
  flat four-line format carries an unexplained `above-the-port` token, and the
  proposed reader would error on a `#` comment line.

- 🟡 **Documentation**: the plan's central discovery is written down nowhere durable
  **Location**: Current State Analysis; Phase 3 Overview
  "A new crate ships with zero architectural enforcement until its `pup.ron` rule
  is written by hand, and nothing notices the omission" — `tasks/README.md:458`
  calls this "the generic add-a-Rust-crate surface" and documents it nowhere. The
  next library crate re-derives it, and the hazard reopens.

- 🟡 **Usability**: the create bridge's `--dry-run` preview has no port surface and no handoff
  **Location**: What We're NOT Doing; Handoffs
  `work-item-create-remote.sh:12-20` previews the resolved issue type and project
  without creating anything, surfacing an unresolvable Jira project *before* the
  confirm gate. The plan asserts "the four operations mirror the three bash
  bridge scripts" but neither ports this nor records it as a non-goal.

#### Minor

- 🔵 **Code Quality / Compatibility / Standards**: the `must_use_candidate` rationale is wrong
  **Location**: Key Discoveries (the `warnings = "deny"` bullet)
  `cli/Cargo.toml:147` sets `must_use_candidate = "allow"`. The attributes are
  correct house style (~260 sites) — the stated reason is not.

- 🔵 **Compatibility / Standards**: `const fn` and `#[must_use]` change the frozen block but are not listed as deviations
  **Location**: Phase 1, Section 6; Deviations
  0204's block says `pub fn new(value: String) -> Self;`. The golden will capture
  `pub const fn new(…)`, so the two artefacts disagree on day one.

- 🔵 **Correctness**: the parity parser keys on an exact literal prefix
  **Location**: Phase 1, Section 7: `codes_declared_by_the_bash_taxonomy`
  `strip_prefix("readonly E_DISPATCH_")` misses `export E_DISPATCH_X=74`, a
  double space, or a bare assignment — and two empty maps compare equal. Add a
  floor assertion.

- 🔵 **Correctness**: the suggested lockfile command targets a package not yet in the lockfile
  **Location**: Phase 1, Section 1: Workspace registration
  `cargo update -p tracker --workspace` cannot resolve `tracker`, and `-p` with
  `--workspace` is a contradictory pair. Use `cargo generate-lockfile` or plain
  `cargo check`.

- 🔵 **Standards**: the `expect_used` allow is deferred but is mandatory
  **Location**: Phase 2, Section 3 (note after `port.rs`)
  `tasks/lint/cli.py` runs clippy `--all-targets … -D warnings`, so test targets
  are linted. Six existing test files carry the file-level allow. The fake's
  `if let Some(…) else if … else` chain will also trip nursery's
  `option_if_let_else`.

- 🔵 **Standards**: the `Display` impl form departs from a unanimous house recipe and is left open
  **Location**: Phase 1, Section 6 (note after `lib.rs`)
  All seven hand-written `Display` impls use `use std::fmt::Display;` +
  `use std::fmt::Formatter;`, which also makes the signature fit on one line.
  Leaving the choice open means the committed golden depends on which form the
  implementer picks.

- 🔵 **Standards**: several samples exceed 80 columns
  **Location**: Phase 1 §6/§7; Phase 2 §1/§3
  The `assert_eq!` in `port.rs`, the `format!` in `surface.rs`, and the
  `Retryable` `write!` arm. Since the golden is captured from formatted source,
  the samples and the golden must agree.

- 🔵 **Architecture**: 0194 cannot call the port from `work` without widening `work`'s pup rule
  **Location**: Handoffs
  `work_domain_imports_only_permitted` does not permit `^tracker`. 0194 will hit
  this mid-implementation, where the cheapest fix also erodes the boundary.

- 🔵 **Architecture**: the surface pin's single-file assumption is unenforced
  **Location**: Phase 2, Section 3; Phase 1, Section 6
  A later `src/port.rs` would be invisible to the golden. Walk `src/`, or assert
  it contains only `lib.rs`.

- 🔵 **Architecture**: one four-method read+write trait diverges from the segregated-port precedent
  **Location**: Phase 2, Section 1: the trait
  `collaboration` — the nearest structural analogue — declares three narrow
  single-purpose traits. Defensible as a role interface mirroring the bridges,
  but unacknowledged.

- 🔵 **Architecture**: no deadline or cancellation seam, and the freeze forecloses adding one
  **Location**: Phase 2, Section 1: trait doc comment
  Defensible (timeouts live behind the port today) but unstated, so it reads as
  an oversight rather than a delegation.

- 🔵 **Code Quality**: `the_error_taxonomy_has_exactly_two_classes` is fully subsumed by `each_class_routes_to_a_distinct_outcome`
  **Location**: Phase 1, Section 7: `errors.rs`
  Both are wildcard-free matches; the second also asserts distinct outcomes. The
  plan's prose refers to "the two `assert_exhaustive` closures" when only one is
  so named and only one carries the comment.

- 🔵 **Code Quality**: stringly-typed fixture rows and a positional `(String, String)` tuple
  **Location**: Phase 1, Sections 5 and 7
  Nothing says which `String` is the code and which the classification, and the
  second column mixes variant names with a routing note.

- 🔵 **Code Quality**: doc comments pin bash filenames the plan says will be deleted
  **Location**: Phase 1, Section 6; Handoffs
  The reference earns its place today, but updating it should be an explicit 0171
  handoff.

- 🔵 **Test Coverage**: AC 9 is left to manual verification despite being trivially automatable
  **Location**: Phase 1 and Phase 2 Manual Verification
  `surface.rs` already reads `src/lib.rs` into a string; asserting the absence of
  `#[cfg(test)]` and `mod ` is one line — and it is the same argument the plan
  makes for automating AC 8.

- 🔵 **Usability**: three adjacent `&str` parameters on `create`, and the reference fake models the wrong call
  **Location**: Phase 2, Section 3
  `tracker.create("ENG-2", "body", "story")` passes an identifier as the title,
  and the fake returns `ExternalId::new(title)`. This is the file 0171 will copy.

- 🔵 **Usability**: `kind`'s value domain and empty case are unstated
  **Location**: Phase 2, Section 1: `create`
  `work-item-create-remote.sh:163-166` makes `--kind` optional, so `""` is a real
  value. Two clients will handle it differently.

- 🔵 **Usability**: `ExternalId` has no `Display`, `Hash` or `&str` construction
  **Location**: Phase 1, Section 6
  0194 must join `FetchOutcome.found` against local items every run; without
  `Hash` it must linear-scan or re-key by `String`, discarding the newtype where
  it is most useful.

- 🔵 **Documentation**: `FetchOutcome`'s public fields are undocumented and the pairing rationale is dropped
  **Location**: Phase 2, Section 1
  `RemoteIssue`'s fields each carry a doc; `FetchOutcome`'s three do not, and the
  work item's reason for `Vec<(ExternalId, RemoteIssue)>` never reaches the crate.

- 🔵 **Documentation**: `work-item-bridge-codes.sh` gains a cross-language consumer its header does not mention
  **Location**: Phase 1, Section 7
  A shell author reformatting a declaration reddens a Rust test whose message
  says the taxonomy "disagrees" — misleading.

- 🔵 **Standards**: reusing `_CONFIG_SERVICE_*` and `_CORE_KERNEL_ERROR` for corpus/vcs/work/migrate misnames them
  **Location**: Phase 3
  A failing `migrate` probe built from `_CONFIG_SERVICE_VIOLATION` reads as a
  copy-paste error.

- 🔵 **Standards**: whole-file pytest runs should go through the mise task
  **Location**: Phase 1 and Phase 3 Success Criteria
  Use `mise run test:integration:pup`; keep `uv run pytest … -k tracker` for the
  filtered inner loop.

#### Suggestions

- 🔵 **Architecture**: carry Phase 3 as its own work item — it is unrelated
  infrastructure taxing a plan whose value is being cheap to reach, and it would
  then have room to cover the widened rules and the two unprobed adapter rules.
- 🔵 **Usability**: add a `//! # Implementing this port` doc example to `lib.rs`
  — the only worked example today is a test file rustdoc does not render.
- 🔵 **Code Quality**: show Phase 3's parameterised writer in the plan as it does
  for the Phase 1 probe; it is the phase's main maintainability risk.
- 🔵 **Documentation**: trim `RemoteTimestamp`'s doc, which duplicates the
  fixture's literal stamps in a fourteen-line comment.
- 🔵 **Standards**: reconsider the single-file `lib.rs`, whose justification is a
  test's parsing convenience rather than the domain.

### Strengths

- ✅ The crate boundary is the right one and is enforced mechanically — empty
  manifest, whole-crate `RestrictImports`, and a probe pair driving the *shipped*
  `cli/pup.ron` — rather than asserted by review.
- ✅ Dependency direction is clean and acyclic, matching the ports-and-adapters
  shape ADR-0053 establishes and the `collaboration`/`github` precedent.
- ✅ The `FetchOutcome` deviation is a genuine catch made before either consumer
  started, correctly identifying that a flat `Vec` forces the caller into the
  unsound `requested − returned` inference the bash path exists to prevent.
- ✅ `RemoteTimestamp` as an opaque byte-preserving newtype with no ordering and
  no validating constructor is faithful to the evidence: raw `=` comparison in
  the classifier, and `""` as a real stored value.
- ✅ Doc comments teach the contract's hardest obligations rather than restating
  signatures — the conservative-default rule, cache-key-not-clock, and
  absence-carries-the-weight are all things a client author would otherwise learn
  from an incident.
- ✅ Failure messages are aimed at the developer and name the next action and the
  affected consumers.
- ✅ Replacing `cargo public-api` with a self-reading golden is the right call for
  this repository — it avoids a third Rust toolchain `tasks/README.md:257-291`
  records as unpinnable.
- ✅ Registration coverage is complete for a plain library crate, with the
  dispatch-only checklist steps excluded with justification; `version.workspace`
  and the `--locked` clippy trap are both handled.
- ✅ Phasing is sound: three independently mergeable phases, with the pup rule and
  the probe that proves it landing together.
- ✅ Deviations, What We're NOT Doing and Handoffs together leave very few
  unowned obligations.

### Recommended Changes

1. **Give the bulk read a body-less record** (addresses: the critical finding)
   Change `FetchOutcome.found` to `Vec<(ExternalId, RemoteTimestamp)>` — or make
   `RemoteIssue.body` an `Option<String>` documented as never populated by
   `fetch_all` — mirroring the bash two-stage bulk-then-`show` design. Propagate
   into 0204's Requirements and into 0194's bulk-vs-`show` criterion. This also
   removes the Linear-`description` constraint currently handed to 0171.

2. **Make the surface golden see what it claims to** (addresses: the golden is
   blind to trait methods and derives; the golden can never be red)
   Widen `declarations()` to capture `fn ` lines inside the trait and every
   `#[derive(...)]` line, and hand-write the golden from 0204's frozen block
   *before* writing `src/lib.rs` so it starts red. Add the AC 9 assertions (no
   `#[cfg(test)]`, no `mod `) to the same test, and either walk `src/` or assert
   it contains only `lib.rs`.

3. **Settle the derive sets now, while the block is open** (addresses: `port.rs`
   does not compile; `ExternalId` ergonomics)
   Decide `PartialEq`/`Clone` on `TrackerError` (every sibling error type has
   them; both consumers' tests will want them) and `Hash`/`Display` on
   `ExternalId` (0194 joins on it every run). Fold `const fn` and `#[must_use]`
   into 0204's block at the same time, and fix the `assert_eq!` regardless.

4. **Make the parity artefacts assert what their names promise** (addresses: the
   parity test never references `TrackerError`; the parser keys on a literal
   prefix; the fixture cannot record its reasoning)
   Derive the expected class names from constructed `TrackerError` values via a
   wildcard-free match, match on the constant name rather than the `readonly`
   keyword, add a floor assertion on the parsed count, and let the fixture carry
   `#` comment lines so the 72/73 rationale lives where 0204 requires it.

5. **Correct the read-failure classification** (addresses: reads given a
   `Terminal` class the taxonomy forbids)
   State in `show` and `fetch_all`'s `# Errors` sections that a read failure is
   always `Retryable` because a read mutates nothing, and record in the fixture
   that the mapping is operation-scoped — the plan already knows this from the
   Linear code 34 trap.

6. **Give the probe pair real imports** (addresses: the permit list is never
   exercised)
   Add `use std::path::Path;` and a `use crate::…;` to the compliant control, and
   adopt the house `use std::fmt::Display;` form in `lib.rs` so the shipped crate
   exercises its own rule too.

7. **Reorder every phase test-first and record the expected red** (addresses:
   test-first is asserted but not sequenced)
   Move each test file ahead of the source it drives, and name the failure each
   red state produces. Add the missing `Err`-through-the-port assertions and the
   fixture-shape assertions to `port.rs` and `vocabulary.rs`.

8. **Fix the four errors of fact** (addresses: pup rule shapes;
   `must_use_candidate`; the `pup:check` lane; the already-applied 0204 edits)
   Reword Phase 3's premise (two rules share `config`'s shape, two are widened)
   and either parameterise the allowance list or record the widenings as unprobed
   follow-ups; correct the `must_use_candidate` bullet; attribute the twelve
   invocations to `test:integration:pup` and restate where the cost lands; and
   rewrite Deviations in the past tense, replacing the first Handoff with an
   obligation to update 0194's and 0171's port descriptions.

9. **Close the documentation gaps that the freeze depends on** (addresses:
   obligations verified in the wrong phase; `body` means two things; `detail` has
   no contract; `Terminal`'s message inverts the advice)
   Move all three doc checks into Phase 1's Manual Verification and add the
   asymmetry and opaque-`kind` checks; state the projection recipe inline rather
   than by script reference and name the read/write `body` asymmetry; give
   `detail` a content contract; and reword both `Display` arms to carry the
   consequence rather than a severity adjective.

10. **Record the obligations the port cannot resolve** (addresses: partition
    totality; `show` cannot express absence; `--dry-run`; `work`'s pup rule; the
    post-push `show`)
    Add each to Handoffs with an owner: totality to 0194's parameterised contract
    test, absence-detection semantics to `show`'s doc comment, the create preview
    to 0171 or to a follow-up item, and the `work`-rule widening decision to
    0194. Then write the "adding a plain library crate" subsection into
    `tasks/README.md` with `tracker` as the worked example.

11. **Split Phase 3 into its own work item** (addresses: Phase 3 scope; the
    unprobed widenings)
    It touches nothing the other phases touch, it is not required by 0204, and as
    a separate item it can be scoped to cover `work`'s and `migrate`'s widenings
    plus the `work_adapters::filesystem` and `migrate_adapters` rules properly.

## Per-Lens Results

### Architecture

**Summary**: The plan draws a genuinely good boundary: a zero-dependency
`tracker` domain crate with an acyclic dependency direction, a
mechanically-enforced import rule that ships alongside the probe proving it
works, and an opaque `RemoteTimestamp` that correctly models a cache key rather
than a clock. The single most important architectural catch — replacing the flat
`fetch_all` return with a three-way `FetchOutcome` partition — is right and well
argued. However, the same reasoning was not carried far enough: `FetchOutcome.found`
freezes a full `RemoteIssue` (body included) into a bulk-read contract whose bash
original deliberately returns timestamps only, the read-side error semantics
contradict the taxonomy the parity fixture claims 1:1 agreement with, and the
partition-totality invariant that carries all the safety weight has no
enforcement mechanism reaching the implementers who are unblocked by this item.

**Strengths**:
- The crate boundary is the right one and is justified against the alternatives;
  the exclusion is enforced mechanically rather than by review.
- Dependency direction is clean and acyclic — the ports-and-adapters shape
  ADR-0053 establishes, matching the `collaboration`/`github` precedent.
- Deviation 1 is a real architectural catch, raised before the freeze rather
  than after.
- Enforcement is proven rather than asserted, and lands in the same phase as the
  rule it proves.
- The two-class error taxonomy correctly resolves codes 72/73 above the port at
  the composition root.
- `RemoteTimestamp` with no ordering, no parsing and `""` legal is faithful to
  the raw-equality comparison in the classifier.
- Returning `Err(TrackerError)` matches the nearest sibling port
  (`collaboration::RepositoryLookup`).
- Phasing is sound — governance lands before the contract it governs.

**Findings**:

- **critical** / high confidence — *`FetchOutcome.found` freezes a body into the bulk read, which the contract it mirrors deliberately does not carry* (Phase 2 §1)
  The bash bulk contract returns `{ "found": { "<key>": { "updated": … } } }` —
  timestamps only — and reserves `show` for "the genuinely-changed minority".
  The plan notices half of this (Linear has no `description`) but treats it as a
  client-side fix; it is not, because the *type* demands a value the bulk
  protocol is designed not to retrieve. Every client is forced to issue N `show`
  calls or fabricate an empty body, which the doc comment says reclassifies every
  synced item. Suggestion: `found: Vec<(ExternalId, RemoteTimestamp)>` or a
  distinct `RemoteSummary`, leaving `RemoteIssue` as `show`'s return type only.

- **major** / high — *Read operations are given a `Terminal` class the bash taxonomy explicitly forbids, and the parity fixture cannot detect the drift* (Phase 2 §1; Phase 1 §5)
  `work-item-fetch-remote.sh:42-48` states "A read mutates nothing, so 71 …
  does not apply here — any underlying read failure collapses to 70". The
  classification is operation-scoped, yet both parity tests compare only code
  names and numbers.

- **major** / high — *The partition-totality invariant is unenforceable and untested against any real implementer* (Deviations §1; Phase 2 §3)
  `FetchOutcome` ships as three public `Vec`s with no constructor; AC 9 forbids a
  smart one; the totality tests exercise the plan's own fake. The parameterised
  contract test is deferred to 0194 while 0171 is unblocked by 0204 alone.
  Suggestion: relax AC 9 for one choke point
  (`FetchOutcome::partition(requested, found, retrieval_was_complete)`), or pull
  the contract test forward, or note the tension explicitly.

- **major** / medium — *The surface pin's single-file assumption is unenforced* (Phase 2 §3; Phase 1 §6)
  Nothing prevents a later `src/port.rs` whose `pub` items the golden never sees.
  Letting the enforcement mechanism dictate source layout also leaves the layout
  constraint unwritten.

- **minor** / high — *Phase 3's parameterisation is premised on a uniformity the shipped rules do not have* (Phase 3)
  `cli/pup.ron:92-107` gives `work` an extra `^corpus(::|$)`; `:173-189` gives
  `migrate` `^corpus` and `^document`. The rules carrying bespoke widenings get
  the least specific coverage.

- **minor** / high — *0194 cannot call the port from `work` without widening `work`'s own pup rule — unrecorded* (Handoffs)
  0194 will hit this as a surprise enforcement failure where the cheapest fix
  erodes the boundary.

- **minor** / medium — *One four-method read+write trait diverges from the segregated-port precedent without acknowledgement* (Phase 2 §1)
  `collaboration` declares three narrow traits. A read-only consumer must be
  handed the full mutating surface.

- **minor** / medium — *No deadline or cancellation seam, and the freeze forecloses adding one* (Phase 2 §1)
  Defensible as a delegation, but unstated. `/list-work-items` explicitly relies
  on the read path not hanging.

- **minor** / high — *Deviations 1 and 2 and the first Handoff describe edits already applied to the work item* (Deviations; Handoffs)
  0204 already carries `FetchOutcome`, "exactly these six items", and the
  `cargo public-api` clause, with the change recorded in its Drafting Notes.

- **suggestion** / high — *Phase 3 is unrelated infrastructure work taxing a plan whose value is being cheap to reach* (Implementation Approach; Phase 3; Performance Considerations)
  Carry it as its own work item.

### Correctness

**Summary**: The plan's domain reasoning is unusually strong — it correctly
identifies that a flat `Vec` return from `fetch_all` forces the caller into the
unsound `absent = requested − returned` inference, and correctly models
`RemoteTimestamp` as a byte-preserving opaque newtype. However, several pieces of
the shown code will not compile or do not do what the plan claims: the `port.rs`
trait-object test compares a `Result<_, TrackerError>` with `assert_eq!` although
`TrackerError` derives only `Debug`; the self-reading surface extractor is blind
to trait method signatures and to derive attributes; and the new cargo-pup rule's
permit list is never exercised by anything. Separately, `FetchOutcome.found`
mandates a `RemoteIssue.body` that neither provider's bulk query can supply.

**Strengths**:
- Deviation 1 is the correct call and correctly reasoned; Linear's
  `_WIFR_LINEAR_LIMIT=250` against ~180 synced items makes it live.
- `RemoteTimestamp`'s shape is correct against the evidence, and the referenced
  Jira stamp really is at `apply-push-204-show.json:17`.
- The asymmetric retryable/terminal rule is carried into the doc comment rather
  than paraphrased away.
- The closed-set guard in `errors.rs` genuinely works.
- The bash-taxonomy parser correctly excludes the `#`-prefixed comment block and
  the `_WORK_ITEM_BRIDGE_CODES_SOURCED=1` guard line.
- Fixture addressing via `CARGO_MANIFEST_DIR` plus `../..` resolves correctly.

**Findings**:

- **major** / high — *Trait-object test will not compile — `assert_eq!` on a `Result` whose error type has no `PartialEq`* (Phase 2 §3)
  `Result<String, TrackerError>: PartialEq` requires `TrackerError: PartialEq`,
  which the frozen `#[derive(Debug)]` does not provide. The instinctive fix
  silently widens the contract.

- **major** / high — *The surface extractor cannot see trait method signatures, so an additively-added fifth operation is invisible* (Phase 2 §3)
  Trait methods carry no `pub`. A fifth method with a default body is caught by
  neither the golden nor the fake.

- **major** / high — *`FetchOutcome.found` mandates a body neither provider's bulk query can produce* (Phase 2 §1; What We're NOT Doing)
  The bash key-scoped path indexes only `{updated: <iso|null>}` for both
  providers (`work-item-fetch-remote.sh:126-128,169-171`). Suggestion:
  `found: Vec<(ExternalId, RemoteTimestamp)>` or `body: Option<String>`.

- **major** / high — *Nothing ever exercises the tracker rule's `allowed_only` list — a mistyped anchor is undetectable* (Phase 1 §3, §4)
  The real `lib.rs` has zero `use` statements and so does the compliant control.
  A typo passes the violation probe, the control and `pup:check`.

- **major** / high — *The four domain rules are not "identical in shape", so the parametrised probes leave two allowances unproven* (Phase 3)
  Deleting `^corpus` from the `work` rule would break `work`'s legitimate imports
  and leave all twelve new cases green.

- **minor** / high — *The forbidden `PartialOrd`/`Ord` derive on `RemoteTimestamp` is guarded by nothing* (Phase 2 §3; Deviations §2)
  Adding them compiles cleanly, changes no signature and breaks no test — while
  AC 7 requires their absence for a load-bearing domain reason.

- **minor** / medium — *The parity parser keys on an exact literal prefix, so a fifth code declared differently slips through* (Phase 1 §7)
  Suggestion: match on the constant name, and add
  `assert!(declared.len() >= 4)` so an empty-vs-empty comparison fails loudly.

- **minor** / high — *The "1:1 onto TrackerError's classes" test never references TrackerError* (Phase 1 §7)
  It compares a fixture the same commit authored against a literal the same
  commit authored.

- **minor** / medium — *`show` cannot express absence while `fetch_all` can* (Phase 2 §1)
  A caller observing a stamp change and then calling `show` will read a
  concurrent deletion as a retryable failure and retry indefinitely.

- **minor** / medium — *The bulk contract's `updated: null` has no stated representation* (Phase 1 §6)
  Two semantically distinct conditions collapse onto `""` at the seam, left to
  each client to rediscover.

- **minor** / medium — *Partition totality is asserted in prose but only demonstrated against a fake that satisfies it by construction* (Phase 2 §1; Handoffs)
  Also silently false when `ids` contains duplicates.

- **minor** / medium — *The suggested lockfile-regeneration command targets a package that is not yet in the lockfile* (Phase 1 §1)
  `cargo update -p tracker --workspace` errors; `-p` with `--workspace` is
  contradictory.

### Test Coverage

**Summary**: For a crate that ships no runtime behaviour the plan is right to
treat the tests as the deliverable, and it reaches for compile-time guards rather
than runtime assertions where the property is type-level. But several of the
guarantees the plan and the acceptance criteria claim are not delivered: the
self-reading surface golden is blind to `#[derive]` lines and to trait method
declarations; the golden is generated from the implementation rather than from
the frozen block; the dispatch-code parity test never references `TrackerError`;
and no test ever exercises an `Err` return through the port. One listing
(`port.rs`) does not compile. Red-green ordering is asserted but contradicted by
every phase's Changes Required.

**Strengths**:
- Compile-time guards are chosen over runtime assertions wherever the property is
  type-level; the closed-set idiom reproduces `cli/vcs/src/classify.rs:584-599`
  exactly.
- The parity test reads the live script rather than a hand-copied snapshot, and
  its failure message tells the reader what to do.
- The boundary conditions that matter are identified and committed as fixtures.
- AC 8's enforcement is automated as a re-runnable probe pair driving the shipped
  `cli/pup.ron`.
- The plan is honest about the limits of its own instruments.
- Three independently mergeable phases, with the pup rule landing together with
  its proof.

**Findings**:

- **major** / high — *`port.rs` does not compile: `assert_eq!` on a `Result` requires `PartialEq` on `TrackerError`, which the freeze omits* (Phase 2 §3)
  The obvious local fix breaks the freeze, and no test would notice.

- **major** / high — *The surface golden is blind to derives and to trait methods, so two clauses of the freeze have no test* (Phase 2 §3)
  `FetchOutcome`'s derives are never exercised at all — deleting its entire
  `#[derive]` line leaves every test green.

- **major** / high — *The golden is generated from the implementation, so it can never be red and pins mistakes as intended* (Phase 2 §2)
  Hand-write it from the Requirements block first; keep paste-the-actual-value
  as the procedure for *deliberate* later changes only.

- **major** / high — *The dispatch-code mapping test never references `TrackerError`* (Phase 1 §7)
  Also: `codes_declared_by_the_bash_taxonomy` only recognises the literal prefix
  `readonly E_DISPATCH_`; assert the parsed count is non-zero.

- **major** / high — *No test exercises an `Err` return through the port* (Phase 2 §3)
  Both error branches of the fake are dead code, and `is_ok()` assertions would
  pass on a fake returning the wrong `ExternalId`.

- **major** / medium — *The timestamp round-trip test passes vacuously if the fixture shrinks* (Phase 1 §7)
  Assert at least two stamps, one `Z`-suffixed and one containing `+0000`, none
  blank.

- **major** / medium — *The probe's positive control has no imports, so a mistyped permit list is undetectable* (Phase 1 §4)
  Follow `_LIBRARY_COMPLIANT` at `test_import_rule.py:549-552`.

- **major** / medium — *Test-first is asserted but every phase sequences production code before its tests* (Implementation Approach; Phase 1/2 Changes Required)
  For a crate whose deliverable is its tests, the ordering *is* the guarantee.

- **minor** / high — *The four domain rules are not identical in shape, and the parameterisation under-probes the differences* (Phase 3)
  Add a per-crate negative control (e.g. `vcs` importing `document`) and positive
  cases for `work`→`corpus` and `migrate`→`document`.

- **minor** / high — *AC 9 is left to manual verification despite being trivially automatable* (Phase 1/2 Manual Verification)
  `surface.rs` already reads `src/lib.rs`; two assertions close it.

- **minor** / medium — *The partition tests verify the fake, not a property, so AC 3's claim overstates what they prove* (Phase 2 §3)
  Express the invariant as a reusable multiset/disjointness check.

- **minor** / medium — *It is unclear which CI lane gates the pup probe pair, and the plan contradicts itself* (Phase 1 Success Criteria; Phase 3; Performance Considerations)
  If the probes only run nightly, AC 8's enforcement does not gate a PR — say so
  as a recorded trade-off.

### Code Quality

**Summary**: The plan proposes a genuinely well-shaped artefact: a
zero-dependency domain crate with six items, no behaviour, and doc comments that
carry non-obvious invariants rather than restating code. The design is trivially
testable and the deviations are enumerated and argued rather than absorbed
silently. The weaknesses are all in the verification code shown verbatim: one
test will not compile, the surface golden cannot see trait methods, and the
dispatch-code parity test never touches `TrackerError` — three guards whose names
promise more than their bodies do.

**Strengths**:
- The simplest design that meets the requirement: no dependencies, no behaviour,
  one file, six items.
- Doc comments earn their place under the repo's very low comment tolerance —
  none restate what the code says.
- Testability is excellent by construction: `&self`-taking, dyn-compatible, no
  I/O, no test-support crate, no feature flag.
- Named precedents are followed faithfully rather than invented.
- Naming reads as domain language throughout.
- The `FetchOutcome` deviation is a real design improvement, not churn.
- Phasing is sound — enforcement is never asserted without evidence.

**Findings**:

- **major** / high — *`port.rs` will not compile — `TrackerError` derives only `Debug` but is compared with `assert_eq!`* (Phase 2 §3)
  Every other domain error in the workspace derives `Clone, PartialEq, Eq`;
  `TrackerError` is the outlier, and both consumers will hit the same wall.

- **major** / high — *The surface golden cannot see trait methods, so it does not deliver the freeze the plan claims for it* (Phase 2 §3; Deviations §2)
  The plan's prose and AC 2 are both false under this implementation.

- **major** / medium — *The parity test asserting a 1:1 map onto `TrackerError` never mentions `TrackerError`* (Phase 1 §7)
  Renaming `Retryable` to `Retriable` leaves it green.

- **minor** / medium — *Clippy runs `--all-targets -D warnings`, so the test-file lint allow is required, not conditional* (Phase 2 §3 note)
  Eight existing test files carry it. The fake's `if let … else if … else` chain
  will also trip nursery's `option_if_let_else`.

- **minor** / high — *`must_use_candidate` is `allow` in this workspace — the discovery mis-states which lint forces what* (Key Discoveries)
  The attributes are correct house style; the stated reason is not.

- **minor** / medium — *Stringly-typed fixture rows and a positional `(String, String)` tuple obscure the dispatch taxonomy* (Phase 1 §5, §7)
  Primitive obsession in the one place the crate's taxonomy is written down.

- **minor** / high — *`the_error_taxonomy_has_exactly_two_classes` is fully subsumed by `each_class_routes_to_a_distinct_outcome`* (Phase 1 §7)
  The plan's prose about "the two `assert_exhaustive` closures" is also
  inaccurate.

- **minor** / medium — *Doc comments pin bash script filenames the plan says will be deleted* (Phase 1 §6; Handoffs)
  Keep the reference but add updating it to the 0171 handoff.

- **minor** / medium — *The golden's fidelity is coupled to rustfmt line-breaking, and the assembling expression is over-nested* (Phase 2 §3)
  Capturing only the first line means a future wrapped signature degrades to a
  name-only pin.

- **suggestion** / medium — *The parameterised probe writer is unspecified and reuses `config`/`core`-named constants for four other crates* (Phase 3)

- **suggestion** / high — *Leaving the `Display` import style open diverges from a uniform house recipe for no benefit* (Phase 1 §6 note)

- **suggestion** / high — *The stamp round-trip test passes vacuously if the fixture is empty* (Phase 1 §7)

### Compatibility

**Summary**: The plan is unusually disciplined about contract stability — the
freeze mechanism, the opaque byte-preserving `RemoteTimestamp`, and the
zero-dependency pup rule are all well chosen, and the one deviation that mattered
was correctly reopened and propagated back into 0204. However, the frozen
`fetch_all` still returns `RemoteIssue` with a mandatory `body`, which neither
provider's bulk path can populate — so the contract as frozen is not satisfiable
by 0171's clients in the mode 0194's own acceptance criteria require. Two
secondary freeze mechanisms are weaker than claimed, and `TrackerError`'s derive
set blocks the equality assertions both consumers' tests need.

**Strengths**:
- Replacing the absent `cargo public-api` with a self-reading golden plus a
  compile-time consumer test is right for this repository.
- `RemoteTimestamp`'s shape protects baselines already on users' disks.
- The `FetchOutcome` deviation was propagated back into 0204's Requirements and
  ACs rather than left to the implementer.
- The dependency contract is enforced mechanically, closing the acknowledged
  absence of any pup coverage guard.
- `version.workspace = true` plus the `--locked` clippy note correctly handle the
  version-coherence machinery.
- Reading the four dispatch codes from the live script keeps the taxonomy pinned.

**Findings**:

- **critical** / high — *`fetch_all`'s frozen return type cannot be populated by either provider's bulk path* (Phase 2 §1)
  0194's own criterion — "the fake tracker records exactly one `fetch_all` call
  and **zero `show` calls**" — makes the collision unavoidable. Fix now and
  propagate into 0204 and 0194.

- **major** / high — *The surface golden cannot see trait items, variants or derives* (Phase 2 §3)
  Adding `PartialOrd`/`Ord` to `RemoteTimestamp` — which 0204 explicitly forbids
  — passes every test in the plan.

- **major** / high — *`TrackerError`'s derive set blocks the assertions both consumers need, and the plan's own test hits it* (Phase 2 §3)
  Under the freeze protocol, adding `PartialEq` after acceptance is a new work
  item.

- **major** / medium — *`RemoteTimestamp` collapses two distinct "unknown" states that the bash classifier keeps apart* (Phase 1 §6)
  `work-item-sync-classify.sh:177` requires `[ -n "$base_remote_updated" ]`;
  derived `PartialEq` loses that guard.

- **minor** / high — *The plan says 0204 must be edited, but 0204 already carries the edits — while 0194 and 0171 do not* (Deviations; Handoffs)
  Both consumers still describe a five-item port.

- **minor** / high — *`const fn` and `#[must_use]` change the frozen block without being recorded as deviations, and the `must_use_candidate` rationale is wrong* (Key Discoveries; Phase 1 §6)

- **minor** / medium — *`create` and `update` return nothing the sync baseline needs, so 0194 owes an extra `show` per push* (Phase 2 §1)
  Matches the bash path, but is unrecorded and unrevisitable after the freeze.

- **minor** / high — *Phase 3's "identical in shape" claim is wrong for `work` and `migrate`* (Phase 3)

### Usability

**Summary**: The plan is unusually strong on prose: the doc comments capture the
conservative retryable/terminal rule, the cache-key-not-clock framing and the
absence-carries-the-weight invariant better than most API docs. But the port's
read surface does not match the shape of the data its two mandated providers can
return, and `RemoteIssue.body` silently means something different from the `body`
parameter on `create`/`update`. Those two, plus an unspecified `detail: String`
and a `Display` string that misdescribes what `Terminal` means, are where a 0171
client author will go wrong at 11pm.

**Strengths**:
- The doc comments teach the contract's hardest obligations rather than restating
  the signature.
- The `fetch_all` deviation is exactly right from a consumer's point of view.
- Failure messages aimed at the developer name the next action and the affected
  consumers.
- The exhaustive-consumer test is a good time-to-first-success artefact as well
  as a guard.
- Deviations, What We're NOT Doing and Handoffs leave very few unowned
  obligations.

**Findings**:

- **critical** / high — *`fetch_all` demands a full `RemoteIssue` from a bulk path that structurally cannot supply a body* (Phase 2 §1)
  The plan's own fake returns fully-populated issues from `fetch_all`, actively
  teaching the mental model that cannot be implemented. Add a body-less fake
  variant so the supported shape is demonstrated rather than inferred.

- **major** / high — *`body` means two different things on the read and write sides, and the doc points at a script 0171 will delete* (Phase 2 §1)
  The projected body carries the title line and canonicalises Jira's ADF, so
  `create(title, body)` then `show` does not return the `body` you sent.
  Suggestion: state the recipe inline; consider `projected_body`.

- **major** / high — *The `Terminal` Display message tells the user the opposite of what `Terminal` means* (Phase 1 §6)
  The bash bridge says "an issue may exist — do NOT retry". Reword both arms to
  carry the consequence.

- **major** / high — *`detail: String` is the port's only diagnostic channel and has no content contract* (Phase 1 §6)
  Doc-only fix: name the provider, the operation, the external id, and the
  underlying status.

- **major** / medium — *`show` cannot report that a remote issue is gone, while `fetch_all` can* (Phase 2 §1)
  Two operations answering the same question with different vocabularies.

- **major** / medium — *The bash create bridge's `--dry-run` preview capability has no port surface and no recorded handoff* (What We're NOT Doing; Handoffs)
  0194's `--preview` is a different thing and does not discharge it.

- **minor** / high — *Three adjacent `&str` parameters on `create`, and the reference fake models the wrong call* (Phase 2 §3; Phase 2 §1)

- **minor** / medium — *`kind` is documented as opaque but its value domain and empty case are unstated* (Phase 2 §1)

- **minor** / medium — *`ExternalId` has no `Display`, `Hash` or `&str` construction, so every consumer call site is noisy* (Phase 1 §6)

- **minor** / medium — *No worked example in the crate's rendered docs — the only reference implementation is a test file* (Phase 1 §6; Testing Strategy)

- **minor** / high — *The freeze the plan promises consumers has a hole exactly where an additive change lands* (Phase 2 §3; Deviations §2)

### Documentation

**Summary**: The plan is unusually strong on inline contract documentation: the
proposed doc comments carry the obligations the freeze depends on as "why" rather
than "what", and they match the house recipe. The weaknesses are elsewhere:
several of the plan's own explanations are already or will become untrue — the
Deviations and Handoffs sections describe edits the work item has absorbed,
Phase 3 and Performance Considerations conflate `pup:check` with
`test:integration:pup`, and Phase 3 asserts a rule-shape equivalence that `work`
and `migrate` do not have. Separately, the doc-comment obligations all land in
Phase 1 but are verified (partially) in Phase 2, the parity fixture's format
cannot carry the reasoning it is required to record, and nothing tells the next
crate author what a new Rust crate owes.

**Strengths**:
- The proposed doc comments carry contract obligations rather than restating
  code.
- The `# Errors` sections state the classification policy per operation rather
  than boilerplate.
- The one inline comment in the crate's tests is justified against an exact
  precedent that checks out verbatim.
- Each test binary gets a `//!` module doc, and the surface-golden assertion
  carries both the regeneration instruction and the tell-0171-and-0194 reminder.
- The `pup.ron` rule carries a rationale comment for the omitted allowance.
- Correctly scoping out user-facing documentation is right — `tracker` is a
  library crate with no binary, no token and no user surface.

**Findings**:

- **major** / high — *The "Deviations" and "Handoffs" sections describe work-item edits that have already been made* (Deviations; Handoffs)
  Rewrite in the past tense, or delete and point at 0204's Drafting Notes.

- **major** / high — *The plan attributes the probe cost to `pup:check`, but the probes run under `test:integration:pup`* (Performance Considerations; Phase 3; Phase 1 Success Criteria)
  The proposed mitigation is aimed at a cost that does not land there, and
  `pup:check` over a compliant `tracker` cannot exercise the rule's
  discriminating power.

- **major** / high — *Phase 3's stated rationale misdescribes the `work` and `migrate` pup rules* (Phase 3)
  The Manual Verification step asks a reader to confirm something untrue.

- **major** / medium — *The contract's doc-comment obligations land in Phase 1 but are verified only in Phase 2, and two are never verified* (Phase 1/2 Manual Verification)
  If Phase 1 merges alone, the conservative-default rule ships with no acceptance
  check at all.

- **major** / medium — *The parity fixture cannot record the reasoning the work item requires it to record* (Phase 1 §5)
  A `#` rationale line would fail `split_once('=')` and error the test.

- **major** / medium — *The plan's central discovery — a new crate silently ships with no pup rule — is written down nowhere durable* (Current State Analysis; Phase 3)
  Add an "Adding a plain library crate" subsection to `tasks/README.md` with
  `tracker` as the worked example.

- **minor** / medium — *`work-item-bridge-codes.sh` gains a cross-language consumer its header does not mention* (Phase 1 §7)

- **minor** / medium — *`FetchOutcome`'s public fields are undocumented and the pairing rationale is dropped* (Phase 2 §1)

- **suggestion** / low — *The `RemoteTimestamp` doc duplicates the fixture's literal stamps* (Phase 1 §6)

### Standards

**Summary**: The plan is unusually well-grounded in this repo's Rust conventions:
it names the crate for its directory, copies the pup rule's two distinct anchor
forms verbatim, inherits every workspace manifest field, keeps one item per `use`
line, and reproduces the house test layout. Registration coverage is complete for
a plain library crate, with the dispatch-only checklist steps correctly excluded.
The gaps are narrower: a factually wrong claim that the four domain pup rules are
shape-identical (which would be committed as a comment), a deferred
`#![allow(clippy::expect_used)]` that the lint set makes mandatory, a `Display`
impl form that departs from a unanimous house recipe and is left ambiguous, and
`const`/`#[must_use]` additions that change the frozen block without being
recorded as deviations.

**Strengths**:
- The bare package name is correctly grounded in `tasks/README.md:454-459`.
- The pup rule copies both anchor forms in the right positions, with the
  alternation-order difference called out.
- The manifest inherits every workspace field and uses `[lints] workspace = true`.
- Registration coverage is complete, and dispatch steps are excluded with
  justification.
- One-item-per-`use`, fully `crate::`-qualified, with the grouped-import hazard
  pinned to a test.
- Test conventions are followed closely — names, `//!` docs, `TestError`,
  `CARGO_MANIFEST_DIR` + `..`, golden-with-instructions.
- The one inline comment is justified against a real precedent.
- Deviations are enumerated and routed back into 0204 rather than absorbed.

**Findings**:

- **major** / high — *The four domain pup rules are not shape-identical, and the plan proposes committing a comment saying they are* (Phase 3)
  A committed comment that misdescribes the rules it probes is exactly the stale
  comment the repo's policy exists to prevent.

- **minor** / high — *The `expect_used` allow attribute is deferred, but the workspace lint set makes it mandatory* (Phase 2 §3 note)
  Six existing precedents; drop the hedge.

- **minor** / high — *The `Display` impl departs from a unanimous house recipe and the plan leaves the choice open* (Phase 1 §6)
  With the `use` form the signature fits on one line, so the divergence buys
  nothing — and the golden depends on which form is picked.

- **minor** / high — *`const fn` and `#[must_use]` change the frozen block but are not recorded as deviations, and the `must_use_candidate` rationale is wrong* (Key Discoveries; Deviations)

- **minor** / high — *Reusing `_CONFIG_SERVICE_*` and `_CORE_KERNEL_ERROR` constants to write corpus/vcs/work/migrate probe sources misnames them* (Phase 3)

- **suggestion** / high — *Several code samples exceed the 80-column limit and are not rustfmt-clean* (Phase 1 §6/§7; Phase 2 §1/§3)

- **suggestion** / medium — *The single-file `lib.rs` departs from the `error.rs` convention, justified by test-parsing convenience* (Phase 1 §6)

- **suggestion** / medium — *Whole-file pytest runs should go through the mise task* (Phase 1 and Phase 3 Success Criteria)

## Re-Review (Pass 2) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran against the revised plan. The critical finding is
resolved and most majors are closed, but the revision introduced one new
critical and a cluster of majors — several of them in the edits made to fix the
first pass. The pattern is worth naming: three of the new findings are cases
where a correction went one step too far or restated a mechanism the code does
not actually implement.

### Previously Identified Issues

**Critical**

- 🔴 **Architecture / Compatibility / Usability / Correctness**: `FetchOutcome.found`
  mandates a body no bulk path can supply — **Resolved.** Compatibility verified
  deviation 5 against the oracle independently: the sync SKILL bulk-fetches
  `{found:{<key>:{updated}}}` and only then calls `show` for keys whose stamp
  moved. All four lenses agree the stamps-only arm makes the wrong answer
  unrepresentable.

**Major — resolved**

- 🟡 `port.rs` does not compile — **Resolved.** Correctness traced ownership,
  borrows, trait bounds and match exhaustiveness through every sample and found
  no compile errors.
- 🟡 Phase 3's "four rules identical in shape" — **Resolved.** Premise corrected,
  allowance list parameterised.
- 🟡 No test exercises an `Err` return through the port — **Resolved.**
- 🟡 `detail: String` has no content contract — **Resolved.**
- 🟡 The timestamp round-trip test passes vacuously — **Resolved.** Test-coverage
  singled out `the_fixture_covers_both_incompatible_provider_formats` as
  covering the failure mode data-driven tests almost never cover.
- 🟡 Deviations and Handoffs describe already-applied edits — **Resolved.**
- 🟡 The parity fixture cannot record its required reasoning — **Resolved.**
- 🟡 The create bridge's `--dry-run` has no handoff — **Resolved.**
- 🟡 `body` means two things on read and write — **Resolved** as a stated
  asymmetry. But see the new critical: the body *shape* the doc now states is
  wrong.
- 🟡 `Terminal`'s Display message inverts the recovery advice — **Resolved** for
  `Terminal`. See new issues for the `Retryable` counterpart.
- 🟡 `show` cannot express absence — **Resolved** as a documented limitation.
- 🟡 Doc-comment obligations verified in the wrong phase — **Resolved**, except
  the `#[non_exhaustive]` check, still stranded in Phase 2 for a Phase 1
  artefact.

**Major — partially resolved**

- 🟡 The surface golden is blind to trait methods and derives — **Partially
  resolved.** The extractor now captures `fn `, `#[derive(` and
  `#[non_exhaustive]`, but five lenses independently found what it still
  misses: it sorts an unattributed multiset (so moving `Hash` between types, or
  `as_str` between the two newtypes, is invisible), captures wrapped signatures
  by their head alone (so swapping `create`'s same-typed `title`/`body`
  parameters passes both the golden *and* the fake), omits enum variant lines
  entirely, never sees `impl std::error::Error`, and walks `src/`
  non-recursively so a directory-style module escapes both it and the AC 9
  guard.
- 🟡 The golden is generated from the implementation — **Partially resolved.**
  It is now hand-written first, but correctness makes a strong case that it
  *cannot* be hand-written from 0204's block: the extractor emits impl
  boilerplate (`fn fmt(...)` twice), raw `#[derive(...)]` text, and bare
  rustfmt-wrapped heads, none of which is derivable from the Requirements block.
  The practical outcome is pasting the actual output — the procedure the plan
  forbids for the first commit.
- 🟡 The parity test never references `TrackerError` — **Partially resolved.**
  It now calls `class_of`, but `class_of` returns hand-written string literals,
  so renaming a variant changes only the match pattern and leaves the test
  green. The plan's claim that it "derives the names from the enum", the Phase 1
  success criterion resting on it, and manual step 2 are all invalid. Separately,
  the test compares an unordered multiset, so **swapping the two fixture rows**
  (70→Terminal, 71→Retryable) still passes — it never checks which code maps to
  which class.
- 🟡 The pup probe never exercises the permit list — **Partially resolved** for
  `tracker`. Phase 3's four backfilled controls remain import-free.
- 🟡 Reads given a `Terminal` class — **Partially resolved.** Both read
  operations now say "Always `Retryable`", but `TrackerError::Terminal`'s
  variant doc still states the unconditional rule ("everything else, including a
  lost or unparseable response, belongs here") that Phase 1 manual verification
  requires to ship verbatim. The two texts ship in the same file and contradict
  each other.
- 🟡 Test-first asserted but not sequenced — **Partially resolved.** Stated in
  bold and Phase 1 reordered, but Phase 2 still numbers the trait §2 ahead of
  `port.rs` §4 and `surface.rs` §5 — and since `surface.rs` is what reads the
  golden, the golden is never red at any point in the prescribed order.
- 🟡 `pup:check` vs `test:integration:pup` — **Partially resolved, then
  over-corrected.** The task attribution is now right, but the new claim that
  the probes "do not gate a PR" is wrong: `.github/workflows/main.yml` runs on
  `pull_request` and `check-architecture` runs `mise run test:integration:pup`
  unconditionally. "Nightly" in this repo names the Rust *toolchain*, not a
  schedule. Both the stated coverage weakness and the cost analysis are
  therefore inverted.
- 🟡 The pup-coverage gap written down nowhere durable — **Partially resolved.**
  Phase 1 §8 exists but is the only deliverable with no drafted text and no
  success criterion, and standards found it would break a committed guard (see
  new issues).
- 🟡 Totality unenforceable — **Resolved as a recorded decision**, but
  compatibility notes the designated catcher does not currently exist: 0194's
  contract-test criterion specifies only `create`→`show` and `update`→`show`
  round-trips, with no `fetch_all` partition case.
- 🟡 The empty stamp collapses two unknowns — **Partially resolved.** Documented
  on the type, but no test pins it, it is absent from Handoffs (unlike the
  structurally identical totality gap), and derived `PartialEq` still reports
  two unknowns as equal — so the doc states a rule the type's own `==`
  contradicts.

### New Issues Introduced

**Critical**

- 🔴 **Documentation / Usability**: the projected-body doc comment states a body
  shape the recipe does not produce.
  `work-item-project-remote.sh:73,84` is `printf '%s\n%s\n' "$summary" "$desc"`
  — title line, then description, **no blank line between them**. The revised
  `RemoteIssue.body` doc says "the issue's title line, then a blank line, then
  its description", and `port.rs`'s example data encodes the same wrong shape.
  Verified directly against the script. By the doc comment's own warning, an
  extra newline "reclassifies every synced item as remotely modified", and
  `work-item-normalise.sh` strips only *trailing* blanks, so an interior one
  survives into the hash. This is in the frozen contract 0171 reproduces
  byte-exactly, and Phase 1's manual check only verifies the comment *states* a
  contract, so it would pass while the contract is wrong.

**Major**

- 🟡 **Correctness / Compatibility / Standards**: `cargo generate-lockfile` is the
  one lockfile command this repo bans. `tasks/CLAUDE.md:9` says "the minimal
  update, never `generate-lockfile`", `tasks/version.py:62-65` explains it would
  "re-resolve the whole ~360-package closure and float every caret-bounded
  dependency", and `tests/unit/tasks/test_version.py:88` asserts no command
  contains it. Verified. The house command is
  `cargo metadata --manifest-path cli/Cargo.toml`.
- 🟡 **Standards**: Phase 1 §8 would break a committed guard.
  `tests/unit/tasks/test_registration_docs.py` slices `tasks/README.md` from
  `## Registering a dispatched sub-binary` to the next `## ` heading and asserts
  exactly 13 numbered items with `**[PR]**`/`**[release]**`/`**[author]**` tags.
  A `###` subsection with four numbered steps inside that region breaks it — and
  Phase 1's verification set runs `build-system:check` (format/lint/types), not
  the unit suite, so nothing in the phase would catch it.
- 🟡 **Correctness / Test Coverage**: the "mistyped anchor" verification cannot
  fail. `^crate($|::)` and `^crate(::|$)` are the same regex — alternation order
  affects which branch a backtracking engine tries first, never whether a match
  exists. The Phase 1 success criterion and manual step 10 both instruct making
  exactly that edit and confirming the control goes red; it will stay green. The
  Key Discovery claiming the alternation order is semantically load-bearing is
  also unsupported by `cli/pup.ron:1-8`.
- 🟡 **Test Coverage**: `impl std::error::Error for TrackerError {}` is pinned by
  nothing. The extractor does not capture `impl ` lines, and no test coerces a
  `TrackerError` into `Box<dyn std::error::Error>`. Deleting the impl compiles
  and passes every test.
- 🟡 **Usability**: the reworded `Retryable` message contradicts `show`'s own
  doc. It asserts "the request failed before it was sent, so it is safe to
  retry", but both reads document *every* failure as `Retryable` — including a
  deleted issue — while `show` says "do not build a retry loop around a `show`
  that may be reading a deleted issue".
- 🟡 **Code Quality / Correctness / Documentation / Standards**: Phase 3's
  controls remain import-free. `_DOMAIN_SERVICE_COMPLIANT` is the renamed
  `_CONFIG_SERVICE_COMPLIANT` — `pub fn make() -> u8 { 0 }`, no imports — and
  Phase 3 describes it as "a positive control (std only)". Phase 1 argues at
  length that exactly this proves nothing, so the same standard is applied
  unevenly within one plan.
- 🟡 **Architecture / Compatibility**: the read bridge's third mode has no port
  surface and no non-goal. `work-item-fetch-remote.sh` exposes plain `search`
  (unkeyed discovery) alongside `search --keys` and `show`; the sync SKILL uses
  it for untracked remote pull, and `fetch_all(ids)` cannot express it. Given
  the same treatment as `--dry-run`, this should be a recorded non-goal with an
  owner.
- 🟡 **Compatibility**: the 0204 edit list is incomplete. Three further places
  in the work item are falsified by deviations 5-6: the Requirements sentence
  "`Display` and `Error` on `TrackerError` are the sole permitted impls with
  bodies" (inside the frozen block), the Technical Notes paragraph handing 0171
  the Linear-`description` constraint that deviation 5 retires, and the Drafting
  Notes' "Four changes" count, now seven.
- 🟡 **Compatibility**: the 0194 handoff understates the change. Dropping the
  body makes 0194's criterion "exactly one `fetch_all` call and zero `show`
  calls" unsatisfiable except on an all-unchanged corpus — not a one-line
  correction. The deviation-5 rationale also cites that criterion backwards.

**Minor**

- 🔵 Phases 1 and 3 both edit `test_import_rule.py` (Phase 3 renames constants in
  it), so the "touches nothing either of the others touch" claim is false and
  the two conflict on merge. Flagged by four lenses.
- 🔵 Phase 1's shipped doc comments forward-reference `FetchOutcome` and the
  trait, which Phase 2 adds — so Phase 1 merged alone documents items a reader
  cannot find.
- 🔵 `FetchOutcome`'s doc calls duplicate ids "unspecified", but
  `partitions_totally` rejects *every* possible behaviour for them; the bash
  adapters `unique` the key set, so the real contract is stateable.
- 🔵 `FixedTracker::show`'s two-arm `match` on an `Option` trips nursery
  `option_if_let_else`, which the file-level allow does not cover — the same
  lint flagged against the first draft's `if let` chain.
- 🔵 Work-item numbers are embedded in shipped doc comments and an assertion
  message, against CLAUDE.md's explicit rule.
- 🔵 `sources()` uses non-recursive `read_dir`, so a directory-style module
  escapes both the golden and the AC 9 guard.
- 🔵 Deviation 6's heading says "three derives"; it adds four.
- 🔵 A found issue whose provider reports no timestamp has no stated home, and
  the natural `filter_map` idiom would drop it from `found` into `absent`.
- 🔵 The bash parser's digits-only filter silently drops a code written as
  `E_DISPATCH_X=74 # note`, and the `< 4` floor cannot catch the drop.
- 🔵 Several one-shot mutation checks are filed under "Automated Verification",
  including one unverifiable process claim ("the golden was red before…").
- 🔵 `the_crate_carries_no_test_module_and_no_behaviour` asserts only the first
  half of its name.
- 🔵 The golden fixture name departs from the `<test_file>.golden` convention the
  cited precedent uses.
- 🔵 Deviation 7's premise (nursery `missing_const_for_fn` forces `const fn`) is
  worth confirming empirically before reopening the frozen block for it — the
  lint has historically declined to fire on parameters carrying a `Drop` impl,
  which `String` does.
- 🔵 The Key Discovery "every existing `as_str` is `const fn ... -> &'static str`
  on a fieldless enum" has a counterexample at
  `cli/visualiser/server/src/frontmatter.rs:12`; the conclusion survives, the
  universal does not.
- 🔵 Code quality notes the plan has grown to ~2000 lines for ~140 lines of
  source, with several facts now stated four or five times over — a maintenance
  liability for a document that still needs edits.

### Assessment

The revision fixed the thing that mattered most: the port can no longer demand a
body no provider can supply, and four lenses independently confirmed the new
bulk arm against the bash oracle. Most of the first pass's majors are genuinely
closed rather than papered over.

But the plan is not ready. One new critical sits inside the frozen contract —
a wrong body shape that would mass-reclassify every synced item, which is
precisely the failure the projection doc exists to prevent. Two more new majors
are self-inflicted: the lockfile command is the one this repo has a committed
test banning, and the new README section would break a different committed
guard. Three of the first pass's fixes are weaker than the plan now claims:
`class_of` transcribes rather than derives, the anchor-mistype verification
cannot fail, and the CI-lane correction over-shot into a claim that is the
opposite of true.

The recurring lesson across both passes is the same one the plan itself keeps
rediscovering: a verification mechanism is worth only what it can actually
detect. Several of this revision's new guards assert less than their prose
promises, and in three cases the plan's own success criteria instruct an
implementer to confirm a property that cannot fail. Those criteria are worse
than absent, because they will be ticked.

A third pass should be cheap — the fixes are mostly small and local, and the
plan's structure and reasoning are sound. Re-verify each claimed mechanism
against the file it names before restating what it guarantees.

## Re-Review (Pass 3) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran. Most of pass 2's findings are closed, and several
closed well. But four lenses independently hand-traced the rewritten surface
extractor against the plan's own `src/lib.rs` and converged on the same four
defects, and a fifth lens found that the retryable/terminal rule has been stated
more narrowly than the taxonomy it claims 1:1 parity with. Both were verified
directly against the source.

The meta-observation matters more than any individual finding: this is the third
consecutive pass in which the surface pin — the crate's whole reason for
existing — has been found not to pin what its prose claims, each time after being
patched rather than reconsidered.

### Previously Identified Issues

**Resolved**

- 🔴 The projected-body blank line — **Resolved.** The documentation lens
  re-verified the doc comment against `work-item-project-remote.sh:73,84` and the
  committed fixtures; title line then description, no blank line, trailing
  newline, Jira-only `jq -cS` canonicalisation, all now correct.
- 🟡 `cargo generate-lockfile` — **Resolved.** The command now matches
  `tasks/version.py:67-71` character for character, and the ban is correctly
  attributed to all three sources.
- 🟡 §8 breaking `test_registration_docs.py` — **Resolved.** Appending a sibling
  `##` after the existing section genuinely leaves the slice untouched; standards
  and documentation both confirmed the guard's parsing independently.
- 🟡 The CI-lane claim — **Resolved.** `check-architecture` carries no `if:` gate
  and runs on `pull_request`; "nightly" names the toolchain.
- 🟡 The anchor-mistype verification — **Resolved** for the swapped-alternation
  case, which is now correctly described as undetectable. But see new issues: one
  of the two replacement corruptions does not work either.
- 🟡 The `Terminal` Display message — **Resolved.** Mutation-safety wording is
  right, and usability confirms it no longer contradicts the read path.
- 🟡 Phase independence, the 0204 edit list, the `--dry-run` handoff, the
  `#[non_exhaustive]` placement, `option_if_let_else`, the fixture rationale
  trim, work-item numbers in the golden message — all **resolved**.

**Partially resolved**

- 🟡 The surface pin — **Not resolved; the rewrite introduced different
  defects.** See below.
- 🟡 `class_of` — **Resolved as designed** (reading the name from `Debug` does
  propagate a rename), but `errors.rs` no longer compiles for an unrelated reason.
- 🟡 The parity mapping — **Resolved.** Per-code assertion closes the row-swap
  hole. One manual step describes the mutation ambiguously.
- 🟡 Phase 3's control — **Resolved in intent, broken in execution.** See below.

### New Issues Introduced

**Critical**

- 🔴 **Correctness / Test Coverage / Code Quality / Compatibility**: the surface
  extractor is defective in four distinct ways, each verified by hand-tracing it
  against the plan's own `src/lib.rs`.
  1. `declaration_from`'s terminator set omits `}`, so
     `impl std::error::Error for TrackerError {}` never terminates. Harmless in
     Phase 1 (last line of file); in Phase 2, where `FetchOutcome` is *appended*
     after it, the reader swallows ~15 lines of doc comment into one golden entry.
  2. `TrackerError`'s variants are brace-struct variants — `Retryable {` ends in
     `{`, not `,` — so `names_a_variant` never matches and **neither variant name
     reaches the golden**. AC 1's rename guarantee is unmet.
  3. The same clause instead captures `detail: String,` twice and `formatter,`
     twice — the latter lifted out of the `write!` arms inside a private function
     body.
  4. `owner_named_by` returns `None` for `#[derive(...)]` lines, which *precede*
     their item, so every derive is attributed to the previous item and the first
     gets an empty owner.
  Consequence: the hand-written-golden procedure Phase 2 §1 mandates is not
  achievable, and the fallback is exactly the characterisation snapshot §1 argues
  against. **Recommended fix (code quality, endorsed): abandon the parser.** For
  each `src/*.rs`, strip `///`/`//!` and blank lines and assert the remainder
  equals the golden — ~8 lines, no parsing, pins derives, variant names, field
  order, full signatures, impls and `#[must_use]`, and the golden becomes
  literally 0204's frozen block, genuinely hand-writable.

- 🔴 **Usability / Correctness**: the retryable rule is stated as "the request
  provably never reached the tracker", which is stricter than the taxonomy it
  claims parity with. Verified: `work-item-create-remote.sh:100-104` says
  "Retryable = provably no issue created (arg / validation / auth / **4xx-reject**
  / rate-limit / unresolvable-config)" and maps codes 11-15, 17, 19, 22, 34 there
  — all of which *reach* the tracker. `work-item-bridge-codes.sh:9` defines 70 as
  "failure provably BEFORE any remote **mutation**". The axis is mutation, not
  transmission. The variant's own summary line ("No remote change occurred") is
  right; the sentence beneath it and both mutating `# Errors` sections are wrong.
  A client following them classifies every 400/401/429 as `Terminal`, so the sync
  refuses to retry calls that provably changed nothing. The parity fixture cannot
  catch this — it pins code membership, not condition-to-class mapping.

**Major**

- 🟡 **Correctness**: `errors.rs` does not compile. `a_tracker_error_is_usable_as_a_std_error`
  calls `boxed.source()`, but the file's only reference to the trait is the type
  alias `Box<dyn std::error::Error>`; an alias does not bring a trait into
  method-resolution scope (E0599). Needs `use std::error::Error;`.
- 🟡 **Correctness / Test Coverage / Code Quality**: Phase 3's rewritten shared
  control breaks both call sites it is said to upgrade. `use crate::Marker;`
  fails at the `config` site (`_CONFIG_LIB` declares no `Marker`) and is a
  *violation* at the `version::core` site, whose rule permits only
  `^crate::version::core(::|$)`. The phase's own success criterion is
  unsatisfiable as written.
- 🟡 **Documentation** (verified): every AC cross-reference from the fifth onward
  is off by one. 0204 has **eleven** criteria; the parity fixture is AC 6 (plan
  says 5), the no-dependency/probe criterion AC 9 (says 8), the no-behaviour
  criterion AC 10 (says 9), the nextest criterion AC 11 (says 10). Concretely:
  Handoffs instructs editing "AC 9's list of permitted function bodies" — AC 9
  has no such list; AC 10 does.
- 🟡 **Documentation / Compatibility**: two Key Discoveries state opposite facts
  about `String::as_str` const-stability, and deviation 7 cites the wrong one.
- 🟡 **Documentation / Code Quality / Architecture**: Phase 2 §5 still describes
  the head-only extractor §2 replaced, asserting the golden is format-coupled
  when §2 says the opposite. Stale text from pass 2.
- 🟡 **Compatibility / Architecture / Usability**: the create bridge's
  identifier-safety check (`_wicr_identifier_safe` — rejects control characters,
  newlines, leading `---` or `#`, classified `Terminal`) has no port surface, no
  non-goal and no handoff. It exists because the value is written unquoted into
  YAML frontmatter. The update bridge's `--dry-run` is likewise unrecorded.
- 🟡 **Test Coverage**: Phase 3's fifteen cases probe permissions but never a
  boundary — broadening `^kernel::Error` to `^kernel` in any of the four rules
  still passes everything. The file already contains the precedent showing why
  (`test_core_importing_kernel_infra_is_rejected`).
- 🟡 **Test Coverage**: one of the two replacement anchor corruptions does not
  work either. `allowed_only` entries are prefix patterns, so **dropping the `^`
  widens** rather than narrows — neither probe reddens.
- 🟡 **Compatibility**: the Handoffs list is headed "Six places" and has five
  bullets, and misses AC 8's claim that the probe pair runs on `mise run
  pup:check` plus the Requirements prose "`fetch_all` pairs each issue with its
  `ExternalId`", which deviation 5 falsifies.
- 🟡 **Usability**: `RemoteIssue.body` omits the case where the two providers
  diverge — a Jira issue with no description projects the literal `null`
  (`jq -cS '… // null'`), Linear projects an empty line.
- 🟡 **Code Quality**: the plan now carries ~150 lines narrating superseded
  drafts ("went through two weaker versions", "took two attempts to get right"),
  and duplicates its mutation checklist across two sections and its Phase 3 cost
  analysis across two more. That residue is how the §5 contradiction survived.

**Minor** — the fake violates its own documented duplicate rule; `FetchOutcome`
ordering and the empty-request case are unspecified; the null-stamped-entry rule
has no test; `#[must_use]` is invisible to the golden; AC 8's manifest half is
unguarded; `two_unknown_stamps_compare_equal…` is a tautology that would block
the fix it warns about; `partitions_totally` lacks `#[track_caller]`;
`manual_pattern_char_comparison` will fire on two new sites; the `bash-parity`
gate question for `errors.rs` is unaddressed (the cited precedent is gated, the
correct precedent is not); Phase 3's stated coupling to Phase 1 is
mis-described; `fetch_all` is now a misnomer.

### Assessment

The contract's *content* is close to right. The two-tier read matches what
`/list-work-items` already does in bash line-for-line; the partition, the opaque
timestamp, the operation-scoped classification and the projection recipe are all
now grounded in verified source. Pass 3 fixed real things and fixed them
properly.

What is not right is the freeze mechanism, and the reason is structural rather
than incidental. A hand-rolled Rust parser has now been patched twice and found
defective three times — not because the patches were careless, but because
line-shape heuristics cannot be verified by reading them. Each pass someone
traced it further and found another shape it mishandles: first trait methods and
derives, then attribution and wrapped signatures, now brace-struct variants,
empty impls, attribute ordering and function-body fragments. The whole-file
stripped-golden approach removes the entire class: no parsing, no heuristics, and
the golden becomes the frozen block itself. That is the change to make before a
fourth pass, and it is smaller than what it replaces.

The second critical is a different lesson. "Provably never reached the tracker"
has survived three passes because it *sounds* like the conservative reading, and
nobody checked it against the mapping tables — where 4xx rejects sit squarely in
the retryable set. Prose that sounds cautious is not the same as prose that is
correct, and this crate ships prose as its deliverable.

Neither critical is expensive to fix. But the plan should not be implemented
until they are, because both are in the frozen surface: the extractor determines
what the golden can pin, and the retryable rule is what two independently
written clients will each encode in a mapping table.

## Re-Review (Pass 4) — 2026-08-11

**Verdict:** REVISE

**Incomplete pass.** A session limit terminated five of the eight lenses
mid-run: correctness, architecture, documentation, compatibility and code
quality returned no findings. Only **standards, test-coverage and usability**
completed. Everything below comes from those three, and the coverage gap matters
— correctness and code quality are the two lenses that hand-traced code in
passes 2 and 3 and found what other lenses missed, and neither ran against the
new build-system code.

Treat this as a partial result. The three lenses that did run found enough to
warrant a revision, but a clean pass still requires the other five.

### What the switch to `cargo-public-api` resolved

The tool change did what you predicted. None of the three completed lenses found
a defect in what the pin *catches* — the four extractor defects and the
hand-writability problem are gone as a class, and test-coverage confirms the
mutation checks now largely hold. The findings below are about the **lane** the
tool arrives on, not the tool.

### New Issues — the build-system lane

**Major**

- 🟡 **Standards / Test Coverage**: the lane would ship dead. Verified:
  `tasks/__init__.py` assembles the invoke namespace by hand — every module is
  both imported and registered via `ns.add_collection(Collection.from_module(…))`
  (`pup` at lines 14 and 63). §9's file list omits it, so `mise run api:check`
  → `invoke api.check` fails with "No idea what 'api.check' is!". Because the
  task also joins the aggregate `check` and the bare `default`, **every local
  run breaks** until it is fixed.
- 🟡 **Standards / Test Coverage**: two committed guards go unextended. Verified:
  `tests/unit/tasks/test_mise.py:18` holds
  `_CHECK_GATES = ["cli:check", "deny:check", "pup:check"]`, whose stated purpose
  is that "a gate cannot be silently unwired from the read-only CI-mirror" — the
  new gate must join it. And `test_workflows.py:386-390` asserts *positively*
  that `check-architecture` runs both pup tasks; §9 only extends the negative
  leak-detection half (`_NIGHTLY_MARKERS`), so nothing asserts the new CI step
  exists at all.
- 🟡 **Standards**: three committed statements in `tasks/README.md` declare the
  nightly lane cargo-pup-only and are left stale — the standalone-gate
  enumeration (43-47), the `### Rust nightly lane (cargo-pup)` section
  ("Only `pup:check` and `test:integration:pup` consume it", 257-291), and the
  CI-job table (476-486) whose rule is "each CI check job mirrors a single
  `mise run` task". `tasks/shared/rust.py:4-7` carries the same claim.
- 🟡 **Standards**: from-source `cargo install` replicates cargo-pup's documented
  *workaround* without cargo-pup's documented *reason*. Every other third-party
  binary is an exact `[tools]` pin hash-locked by `mise.lock`; cargo-pup is the
  sole exception, recorded as "an accepted unverified surface", singular — and
  §9's own reasoning establishes neither constraint applies here.
- 🟡 **Test Coverage**: `tasks/api.py` and `deps:install:public-api` ship with no
  test provision, though `test_deps.py` and `test_rust.py` cover the cargo-pup
  equivalents thoroughly — including the exact hazard §9 names (whole-token
  version equality, so `0.4.1` does not false-match `0.4.10`). Also unspecified:
  what `api:check` does when the snapshot is missing or empty, the classic
  vacuity mode for snapshot checks.
- 🟡 **Usability**: there is no regeneration task. §9 says the check fails "with
  a message naming … the regeneration command", but defines only a check task
  and never states the command. The one workflow the lane exists to support
  after the first commit — a deliberate, reviewed surface change — has no
  documented path, and hand-reconstructing the invocation risks drift from the
  check's own.
- 🟡 **Usability**: the new library-crate checklist omits the snapshot. §8 lists
  four obligations for a new crate and §9's invocation is hard-scoped
  (`-p tracker`), so the next library crate is silently exempt from the surface
  pin — the exact hazard §8 exists to close, now with two exceptions recorded
  as one.

### New Issues — coverage and contract

**Major**

- 🟡 **Test Coverage**: moving the pin out of the test suite concentrates it on
  the least reliable lane. Verified: CI runs per-component tasks and never
  `mise run check`, so the only CI execution of `api:check` is the
  `check-architecture` step — the job whose fragility is the stated reason
  `test_workflows.py` isolates it. A nightly break, rustdoc-JSON skew or a failed
  `cargo install` disables the freeze while `check-cli` and `test-unit` stay
  green, and the prescribed inner loop (`cargo nextest run -p tracker`) no
  longer exercises the contract. Suggested mitigation: exhaustive destructuring
  in `port.rs` (`let RemoteIssue { updated, body } = issue;`) so a changed public
  field reddens the stable lane at compile time.
- 🟡 **Test Coverage**: default output is not hand-writable, which undermines the
  red-first property deviation 8 rests on. With blanket and auto-trait impls
  included, six items become dozens of `impl Send/Sync/Unpin/RefUnwindSafe…` and
  `impl<T, U> Into<U> for T` lines. §10 concedes this and prescribes
  generate-then-reconcile — which *is* the characterisation the deviation says
  it avoids. **Fix: `--omit blanket-impls,auto-trait-impls`** — not
  `--simplified`, which additionally omits `auto-derived-impls` and would
  discard the derive coverage we switched for. This makes the whole snapshot
  transcribable and restores the property.
- 🟡 **Test Coverage**: three things the deleted `surface.rs` covered now have no
  automated successor — AC 9's absent `[dependencies]`/`[dev-dependencies]`
  tables (rustdoc JSON cannot see a manifest; pup runs without `--tests`), AC
  10's absent `tracker-adapters` member (no check at all, manual or otherwise),
  and `#[must_use]`. All three are cheap to add to `no_test_module.rs`, which
  already reads files under `CARGO_MANIFEST_DIR`.
- 🟡 **Test Coverage**: AC 2's "none with a default body" is guarded only for the
  *additive* case. Giving one of the four *existing* methods a default body is
  the more dangerous change — a client silently need not implement it — and
  neither the fake (which overrides all four) nor the snapshot (which renders
  signatures without marking defaults) would notice.
- 🟡 **Test Coverage**: Phase 3's fix is half-applied. I remedied `version::core`
  but left `_write_config_probe`, whose `_CONFIG_LIB` is `pub mod service;` with
  no `Marker` — so `test_real_config_rule_passes_a_compliant_service` fails on a
  compile error, and the phase's own success criterion fails on first run.
- 🟡 **Test Coverage**: Phase 3's extras are probed for permission, never
  prohibition. Copy-pasting `^document(::|$)` into `vcs_domain_imports_only_permitted`
  passes all fifteen cases. The backfill detects a widening being *lost*, never
  one being *gained* — which is the erosion 0194's own handoff warns about.
- 🟡 **Usability**: the reworded Retryable enumeration reads as a status
  allowlist and disagrees with the oracle. Listing "4xx rejects, rate limits" as
  categories invites a status-to-class table — but `work-item-update-remote.sh:66-72`
  deliberately omits Linear code 34 from the retryable set on `update` while
  `create` admits it, because `linear-graphql.sh` has no pre/post-send
  distinction. Same status, same provider, different class. The Handoffs bullet
  also names `_wicr_map_jira`/`_wiur_map_jira` as "the Jira and Linear tables" —
  both are Jira.
- 🟡 **Usability**: the identifier-safety obligation lives only in this plan, and
  `ExternalId`'s doc says the opposite ("taken as opaque: the port does not
  parse, validate or interpret"). Once the plan is archived, the only surviving
  statement of a check protecting frontmatter from corruption is a script 0171
  is chartered to delete.
- 🟡 **Usability**: `RemoteIssue.body` still omits the absent-description case —
  `jq -cS '.fields.description // null'` means Jira with no description projects
  the literal `null`; Linear projects an empty line. Unguessable from the prose,
  and it mass-reclassifies.
- 🟡 **Usability**: the empty-stamp trap now has a warning in four places and an
  affordance in none. When an API needs the same caveat repeated four times, the
  type is missing something — a three-line `is_known()` would let the safe
  comparison be written without reaching for `as_str()`.
- 🟡 **Usability**: `port.rs` omits the three shapes a client will get wrong — a
  *failing* `create` (the decision between a duplicated and a lost issue), a
  partial multi-chunk `fetch_all`, and `detail` strings matching the documented
  shape (the fake's violate it).

**Minor** — nine AC cross-references still wrong after last pass's partial
rename (verified: L86, L232, L271, L311, L415, L419, L544, L2010, L2154 —
the bulk string-replace only caught exact matches); `api:check` naming breaks
the tool-entity convention its two siblings establish and collides with the
visualiser's HTTP-API namespace; the crate-root snapshot departs from the
`tests/fixtures/` convention; `no_test_module.rs` lands in Phase 2 though
`lib.rs` ships in Phase 1, contradicting the plan's own phase rule; the
swap-the-rows mutation check is a no-op as worded (a `BTreeMap` keyed by name
ignores line order); the fake still contradicts the documented duplicate rule;
install-task failure contract and both mise descriptions unspecified; `fetch_all`'s
all-failed case undecided (the two adapters disagree); `show`/`fetch_all` no
longer a singular/plural pair; `Display` strings stack a generic prefix onto a
detail that already names the failure; `kind`'s empty-string sentinel among
three same-typed `&str`; empty-request and ordering contracts unstated; style
nits (misindented brace in `no_test_module.rs`, inconsistent `TestError` alias).

### Assessment

The tool switch was the right call and it worked. The freeze mechanism is no
longer something we maintain, and for the first time in four passes no lens
found a defect in what the pin detects. Two of the three completed lenses spent
their findings on the lane rather than the contract, which is the shape you would
want.

But the lane has the same character as the parser did: it is new infrastructure
introduced by a work item whose purpose is being cheap to reach, and it is
under-specified in the places this repo has committed guards. Two of those guards
would fail on the first run, and one omission (`tasks/__init__.py`) would break
every local `mise run check` until noticed. These are all small, concrete fixes —
but there are enough of them that the lane is a work item's worth of care, not a
phase's.

The single most valuable finding is `--omit blanket-impls,auto-trait-impls`. It
restores the hand-writable, starts-red property that deviation 8 is built on, and
without it §10's reconcile step quietly reintroduces the characterisation
snapshot the switch was meant to eliminate.

Two recommendations beyond the fixes. First, consider splitting the
`cargo-public-api` lane into its own work item — the same argument three passes
have made about Phase 3, and now with more force, since the lane is workspace
infrastructure that will pin other crates. Second, re-run the five lenses that
did not complete before treating any of this as settled; correctness and code
quality in particular have found what others missed in every prior pass, and
neither has seen the new build-system code.

## Re-Review (Pass 5) — 2026-08-11

**Verdict:** REVISE

The five lenses cut short in pass 4 have now run (code quality and architecture
first, then correctness, documentation and compatibility on retry), so together
with pass 4's three this is a **complete eight-lens pass** on the current
revision.

The tool switch has held: no lens found a defect in what `cargo-public-api`
*detects*. Every finding below is about the lane, the plan document, or contract
prose — and a striking proportion are defects introduced by pass 4's own fixes.

### Confirmed against source

I verified the load-bearing claims rather than relaying them:

- `every_public_field_is_accounted_for` appears exactly once in the plan — its
  definition. Never called.
- Jira codes 15, 17 and 22 are `E_BAD_SITE`, `E_REQ_BAD_PATH` and
  `E_REQ_NO_CREDS` (`jira-request.sh:10-26`) — local validation failures that
  never reach the tracker.
- `_wiur_map_linear` retryable set is `{11, 18, 22, 23, 25, 27, 29, 35, 36}`;
  the create-side set is `{11, 22, 34, 35, 36}` (via `E_CREATE_PRE_SEND`).
  Update is **wider** on five codes and narrower only on 34.
- Phase 1's manual check still demands "absence of transmission"; §7's doc
  comment says "absence of a remote change". They contradict.
- The `tasks/README.md` block says "owes four things" and lists five bullets.

### New Issues

**Critical**

- 🔴 **Correctness / Code Quality**: `port.rs` cannot compile.
  `every_public_field_is_accounted_for` — the stable-lane destructuring guard
  added in pass 4 — is never called, and `cli/Cargo.toml:133-134` sets
  `warnings = "deny"`. `dead_code` becomes a hard error; clippy `--all-targets`
  denies it again. Phase 2's own success criterion cannot be met, and the guard
  never runs.
- 🔴 **Compatibility / Correctness / Architecture**: `fetch_all`'s `# Errors`
  says `Err` occurs "only when the request could not be issued at all" and then
  that "a total outage is an `Ok` with every id indeterminate". Those classify
  the same condition oppositely. The Linear adapter sides with `Err`
  (`_wifr_linear_keys` returns 70 on any search failure); the Jira adapter sides
  with `Ok`. Two clients will diverge on the same wire condition, in a frozen
  contract.

**Major — defects pass 4 introduced**

- 🟡 `update`'s "Provability is narrower here than on `create`" is **backwards**
  for five of the six diverging Linear codes. A client author trusting the
  direction treats `create` as more permissive — the one misclassification that
  duplicates remote issues on a non-idempotent operation.
- 🟡 The Key Discovery "Every one of those reached the tracker and was rejected"
  is false for 15, 17 and 22. The conclusion (mutation, not transmission) still
  holds on the six that did; the overstatement could seed a wrong mapping table.
- 🟡 Phase 1's manual check demands the transmission wording the same revision
  spent a Key Discovery refuting — and the 0204 Requirements sentence carrying
  it is **not** on the Handoffs edit list. The likely resolution during
  implementation is to "fix" the correct doc comment back to the wrong rule.
- 🟡 The `tasks/README.md` block ships "owes four things" over five bullets —
  into committed repo documentation, beside a checklist whose count is
  machine-asserted.
- 🟡 A **fourth** stale statement missed by §10's list: the repo-root
  `CLAUDE.md:35-36` enumerates Rust enforcement as "(cargo-deny, cargo-pup)".
  That is the always-loaded orientation file.

**Major — pre-existing**

- 🟡 **Correctness / Compatibility**: the snapshot is not hand-writable as
  claimed. With only blanket and auto-trait impls omitted, output retains
  derive-generated *methods* — `pub fn ExternalId::hash<__H>(&self, state: &mut
  __H)`, `impl StructuralPartialEq`, `core::error::Error` rather than `std::` —
  whose names the compiler chooses and 0204 never states. Roughly forty such
  lines. The realistic outcome is regeneration, losing the red-first property
  deviation 8 rests on. Fix: run the tool once against an existing crate and
  record the rendering conventions before authoring, or split the file into a
  hand-written declarations section and a captured impls section.
- 🟡 **Correctness**: Phase 1's step order makes its own success criteria
  unachievable — the snapshot is §11 but `lib.rs` is §7, and §1 syncs the
  lockfile before §2 creates the manifest (`cargo metadata` fails on a missing
  member).
- 🟡 **Correctness**: the fake classifies "no such issue" on `update` as
  `Terminal`, which under this crate's own contract is a provable non-mutation
  — `Retryable`. The plan calls `port.rs` "the only worked example a 0171 client
  author has".
- 🟡 **Correctness**: deviation 7 makes `src/lib.rs`'s constructors `const fn`
  on an unresolved lint expectation, while `FixedTracker`'s identically-shaped
  constructors are left non-`const`. Both cannot be right.
- 🟡 **Compatibility**: the propagation list is materially incomplete — five
  further statements in 0204 (Assumptions, the "four operations mirror three
  scripts" prose, AC 2's "surface golden", two Technical Notes), two in 0194
  (the unwidened-import-rule assertions, the `project_remote.rs` wiring), and
  **six capabilities** the plan discovered but never recorded in 0171
  (unkeyed `search`, both `--dry-run` modes, identifier safety, timeouts, page
  cap). The plan argues these must not be rediscovered at cutover, then leaves
  them in a document 0171's implementer has no reason to open.
- 🟡 **Compatibility / Architecture**: the `work` → `tracker` import question is
  deferred to 0194, whose Requirements:99 and AC:399-400 already assert
  classify/decide compile "under its existing unwidened import rule" while
  requiring a pre-fetched `tracker` record. A real contradiction, resolved
  under implementation pressure by whoever hits the pup failure.
- 🟡 **Architecture**: the lane and Phase 3 are both workspace infrastructure
  inside a milestone whose purpose is being cheap to reach. Phase 1 now carries
  eleven steps including the entire tool lane; Phase 2 adds one struct, one
  trait and a test file. Three unresolved empirical questions gate Phase 1.
- 🟡 **Compatibility**: `PUP_NIGHTLY` reuse couples the freeze to a constant
  bumped for cargo-pup's reasons, twice over (rustdoc-JSON format *and* output
  rendering). Because `public-api:check` joins `check` and `default`, a break
  reddens every local run — not the isolated lane. The escape hatch is written
  as a setup contingency, not a bump-time procedure.
- 🟡 **Code Quality**: the mutation checklist exists in three copies which have
  already drifted (one still says `api:check`), and revision-history narration
  now spans six places.

**Minor** — the bash-taxonomy parser hard-errors on any non-declaration mention
of `E_DISPATCH_`; `structure.rs`'s predicates false-positive on
`#[cfg(feature = "latest")]` and miss `[build-dependencies]`; two Phase 1 manual
checks reference Phase 2 artefacts; the empty-request test cannot verify its own
"makes no call" claim (and the Linear adapter violates it); `public-api:check`
reintroduces the per-crate opt-in gap §9 exists to close; five count statements
disagree with their lists; the `[dev-dependencies]` bullet resolves an ambiguity
AC 9 does not have; "only inline comment" is contradicted by `vocabulary.rs`;
`--profile minimal`'s remedy names a rustup component that does not exist
(rustdoc ships in `rustc`); the doc-staleness handoff under-enumerates by three;
`structure.rs` asserts workspace-manifest invariants from a leaf crate's tests;
two `issue` functions in one file; ordering pinned by tests the contract
disclaims.

### Assessment

Two things are now clear, and they point in opposite directions.

**The contract is close.** Across five passes the port's design has converged:
the two-tier read, the partition, the operation-scoped taxonomy and the
projection recipe are all traceable to the bash oracles, and this pass verified
that lineage rather than assuming it. The instrument question is settled — the
tool detects what we need and no lens disputed it.

**The plan document has become the problem.** This pass found ~14 majors, and
five of them were introduced by pass 4's fixes: a dead function that breaks the
build, a self-contradicting `# Errors` section, a backwards provability claim, a
false universal about Jira codes, and a manual check that contradicts the doc it
checks. That is not carelessness compounding — it is what a 2670-line document
covering a 140-line crate, a workspace tool lane and an unrelated probe backfill
does to anyone editing it. Each fix cycle now has a meaningful chance of
introducing a defect elsewhere, and the review cost to find it is a full
eight-lens pass.

The recommendation is therefore structural rather than another fix list:

1. **Split the document.** The `cargo-public-api` lane and Phase 3 are separate
   work items — three lenses have now said so independently, and the lane has
   three unresolved empirical questions that would otherwise gate the port.
   0204 then covers the crate, its vocabulary, the port, `structure.rs` and the
   pup probe pair: roughly 800 lines, reviewable in one sitting.
2. **Fix the two criticals and the five self-inflicted majors in place** — they
   are small and localised, and the contract text should not carry known errors
   into a split.
3. **Resolve the empirical questions before the lane is planned**: does
   `--profile minimal` carry rustdoc (probably yes — it ships in `rustc`), does
   `missing_const_for_fn` fire through a `Drop` parameter, does an aqua/ubi
   backend publish cargo-public-api, and what does the tool's output actually
   look like. Four commands, and they decide three deviations.

The plan is not far from implementable. But it should be three documents when it
gets there, not one.

## Approval (Pass 6) — 2026-08-11

**Verdict:** APPROVE

Approved by Toby Clemson after five review passes, with the open items below
accepted deliberately rather than resolved. This section records what was
accepted, so a later reader can tell an accepted trade-off from an oversight.

### Closed since pass 5

Both criticals and eleven of the majors are fixed in the plan:

- `port.rs` compiles — the destructuring guard is a live `#[test]` rather than
  dead code under `warnings = "deny"`.
- `fetch_all`'s `# Errors` has one `Err` trigger (pre-flight only), with the
  deliberate divergence from the Linear bridge recorded.
- `update`'s provability claim is non-directional; the two operations' sets are
  not nested either way.
- The Jira retryable set is described accurately — 15, 17 and 22 never leave
  the machine; the mixture is *why* the test is mutation, not transmission.
- The Phase 1 doc check matches the doc comment it checks, and 0204's
  "absence of transmission" sentence is on the edit list.
- The snapshot is explicitly two halves: a hand-written contract half that must
  start red, and a captured derive-method half whose names come from the
  expansion.
- The fake models the classification axis correctly — a rejected write is
  `Retryable`, a lost response `Terminal`.
- The bash tables are recorded as authoritative where they are more
  conservative than the provability rule.
- The `const fn` question applies uniformly to the fake's constructors.
- AC 2's default-body clause has an explicit check and an instruction to record
  it as unguarded if nothing catches it.
- All six stale cross-references are corrected; the README count matches its
  bullets; the stale-docs list is complete at five entries.

### Accepted as open

Six majors remain, accepted on the understanding that they are either
cross-item work, resolved by running four commands, or readability:

1. **Propagation is incomplete** — five further statements in 0204, two in
   0194, and six capabilities never recorded in 0171 (unkeyed `search`, both
   `--dry-run` modes, identifier safety, per-call timeouts, the page cap).
   Natural to handle when 0194 and 0171 are next touched.
2. **The `work` → `tracker` import decision** is deferred to 0194, which
   asserts the opposite in Requirements:99 and AC:399-400. Expect a `pup:check`
   failure mid-implementation; the cheap fix there erodes the boundary.
3. **`tasks/public_api.py` is prose-only**, with an unresolved provisioning
   fork (aqua/ubi pin versus from-source install) that changes which files and
   guards are touched.
4. **`PUP_NIGHTLY` couples the snapshot's content**, not only its format. A
   nightly bump can produce a rendering diff indistinguishable from a surface
   change, and the task sits in `check` and `default`.
5. **The mutation checklist exists in three copies.** This is how the
   `api:check` drift reached pass 5; it will make the next edit riskier.
6. **Revision-history narration in six places.** A reader implementing from the
   document must filter superseded material.

Items 3 and 4 are largely settled by the four empirical commands the plan
already calls for: whether `--profile minimal` carries `rustdoc` (it ships in
`rustc`, so probably yes — and the plan's "add a component" remedy names one
that does not exist), whether `missing_const_for_fn` fires through a `Drop`
parameter, whether an aqua/ubi backend publishes `cargo-public-api`, and what
the tool's output actually looks like. Run them before Phase 1.

The recommendation to split the plan into three work items — the crate, the
`cargo-public-api` lane, and the Phase 3 probe backfill — was raised across
three passes and declined. The plan proceeds as one document at ~2800 lines.

---
*Review generated by /accelerator:review-plan*
