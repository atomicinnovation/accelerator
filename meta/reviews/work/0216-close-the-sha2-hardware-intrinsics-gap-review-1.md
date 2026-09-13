---
type: "work-item-review"
id: "0216-close-the-sha2-hardware-intrinsics-gap-review-1"
title: "Work Item Review: Close the sha2 hardware-intrinsics gap"
date: "2026-09-10T17:59:53+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0216"
work_item_id: "0216"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-10T18:34:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Close the sha2 hardware-intrinsics gap

**Verdict:** REVISE

The work item is structurally excellent — every section is present and
substantively populated, the deliverable is a single coherent action, and the
Context motivates it with measured evidence carried cleanly from 0205/0189. It
tips into REVISE on two grounds: three of the five Acceptance Criteria carry
thresholds or properties that cannot yield a definitive pass/fail, and the
relationship with sibling task 0215 is left as an open question rather than
resolved into a sequencing decision. Both are fixable by editing the
Acceptance Criteria and Dependencies sections; none of the findings question
the item's premise or diagnosis.

### Cross-Cutting Themes

- **The 0216↔0215 relationship is under-specified** (flagged by: dependency,
  scope, clarity) — three lenses independently landed on 0215. Dependency wants
  the mutual-obviation promoted from "relates to" to an explicit gating edge;
  scope wants the units-of-delivery boundary resolved so the two tasks are not
  redundant or conflicting; clarity found the Open Questions *description* of
  the fallback mischaracterises what 0215 actually specifies (it removes the
  cache-hit sha256 and requires a cheaper name/version binding — it does not
  "move the corruption check to BLAKE2b").
- **The Acceptance Criteria cannot fully define "done"** (flagged by:
  testability, completeness) — testability found three criteria that no
  verifier could decide (a parity threshold contradicting the item's own
  residual-gap note, a waivable "does not regress where measurable" clause, and
  no test of the musl runtime-safety property the item hinges on); completeness
  found the definition of done omits the fallback exit path the item itself
  documents.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: 'Does not regress where measurable' has no baseline and can always be waived
  **Location**: Acceptance Criteria (AC 3)
  AC 3 defines no baseline figure to compare against on the aarch64 musl
  target, and the qualifier "where measurable" lets the check be declared
  inapplicable — so the criterion can be argued satisfied regardless of result.

- 🟡 **Testability**: Throughput pass threshold conflicts with the item's own residual-gap note
  **Location**: Acceptance Criteria (AC 1)
  AC 1 requires "at least openssl parity (~1,700 MB/s)", but the Technical Notes
  acknowledge the hardware path may legitimately trail openssl by 10–30%
  (~1,200–1,540 MB/s). A result in that band cannot be classified pass or fail.

- 🟡 **Testability**: No criterion verifies the musl runtime-safety property the item is built around
  **Location**: Acceptance Criteria (AC 3)
  The item makes musl safety central — the path must not depend on forcing `-C
  target-feature=+sha2`, which would fault on CPUs without the extension — but
  AC 3 checks only that the build does not force `+sha2`, not that the binary
  runs without a SIGILL on an aarch64 CPU lacking the extension. A clean build
  does not prove runtime detection works.

- 🟡 **Dependency / Scope**: The 0216↔0215 relationship is left as an open question, not a sequencing edge
  **Location**: Dependencies / Open Questions
  Both tasks operate on the same warm-dispatch sha256 — 0216 makes the digest
  cheap, 0215 removes the call site — and 0216 says it "may make 0215
  unnecessary" while also naming 0215 as its own fallback. The relationship is
  recorded only as "relates to" plus open questions; neither item captures the
  go/no-go ordering. Scheduling both in parallel risks redundant or conflicting
  changes to the same `reverify`/`verifier::sha256_hex` call site.

#### Minor

- 🔵 **Clarity**: Fallback description conflicts with what 0215 actually specifies
  **Location**: Open Questions
  The fallback is described as "move the corruption check to BLAKE2b … via
  0215", but 0215 removes the cache-hit sha256 and explicitly warns that
  minisign's BLAKE2b signs bytes only — it does *not* bind the asset's
  name/version — so a separate cheaper replacement is required. Elsewhere in
  0216 (Dependencies, References) 0215 is described accurately.

- 🔵 **Dependency**: Assumed-available committed measurement harness not captured as a prerequisite
  **Location**: Requirements
  Requirements treat `cli/launcher/tests/warm_terms.rs` as an already-committed
  harness. The 0205 spike ran instrumentation from a differently-named
  throwaway (`spike_0205_warm_terms.rs`) recorded as removed, and lists "the
  committed-harness decision" among 0189's still-open hand-offs. If the harness
  is not in fact present, the measurement criteria cannot be met.

- 🔵 **Dependency**: Linux/musl measurement hand-off to 0217 not captured as ordering
  **Location**: Dependencies
  0216's throughput measurement is scoped to darwin-arm64, yet AC 3 asserts a
  musl throughput property and the Dependencies section names 0217 as the item
  that "measures warm dispatch on linux". The linux/musl evidence appears to
  belong to 0217, but the two are coupled only as "relates to".

- 🔵 **Testability**: 'All four shipped targets' are never enumerated and 'builds cleanly' is undefined
  **Location**: Acceptance Criteria (AC 2)
  AC 2 requires the remedy to "build cleanly across all four shipped targets",
  but the four target triples are never listed (only three are named in
  passing) and "cleanly" is undefined (compiles vs warning-free).

- 🔵 **Testability**: 'Release artefacts remain reproducible' names no verification procedure
  **Location**: Acceptance Criteria (AC 4)
  AC 4 requires artefacts to "remain reproducible" but references no procedure
  for confirming it (e.g. a byte-identical rebuild-and-compare).

#### Suggestions

- 🔵 **Completeness**: Definition of done omits the documented fallback exit path
  **Location**: Acceptance Criteria
  All five criteria assume the hardware backend is enabled successfully; none
  states what "done" looks like if the documented musl-blocker fallback (accept
  the gap, route via 0215) fires. An implementer hitting the blocker has no
  completion definition to satisfy.

- 🔵 **Clarity**: 'the same input' has no antecedent in the work item
  **Location**: Acceptance Criteria (AC 1)
  AC 1's "runs over the same input" has no antecedent within 0216 — the
  ~2.49 MB sub-binary that set the 555 MB/s baseline is named only in the
  referenced 0205/0189 material. A verifier must leave the item to find which
  input makes the comparison valid.

- 🔵 **Clarity**: Specialist acronyms/terms used without definition (MSRV, HWCAP, getauxval)
  **Location**: Technical Notes
  The musl-safety argument leans on "MSRV 1.85 is under the pinned 1.90" and
  "getauxval/HWCAP works under `crt-static`" without glossing these terms, so
  the argument is not self-contained for a developer new to the build concerns.

- 🔵 **Clarity**: 'minisign's own digest' equated with BLAKE2b only implicitly
  **Location**: Context
  Context says "minisign's own digest is already the faster of the two"; the
  identity minisign-digest = BLAKE2b is only confirmed later in Open Questions.
  Making it explicit on first use ("minisign's own digest (BLAKE2b)") would
  speed comprehension of the inversion.

### Strengths

- ✅ Every standard section is present and substantively populated; frontmatter
  integrity is strong — a recognised `kind` (task), `status` (draft) matching
  the stage, and all relationship fields filled.
- ✅ The deliverable is a single coherent unit of work (enable, verify, measure
  the hardware backend), consistent across Summary, Requirements, and
  Acceptance Criteria with no scope drift.
- ✅ Context motivates the work with measured evidence (555 MB/s vs 1,708 MB/s,
  the BLAKE2b inversion) rather than restating the summary.
- ✅ Acceptance Criterion 1 is a well-formed Given/When/Then with a numeric
  threshold, a stated baseline, and a named harness; the Technical Notes add an
  unambiguous fail discriminator (a result still in the ~500 MB/s band means the
  soft backend is still selected).
- ✅ The Drafting Notes record a deliberate spike-to-task conversion that
  narrowed scope, with the accept-the-gap path demoted to a conditional
  fallback rather than a co-equal option.
- ✅ The external `sha2` crate coupling and its downstream effects (the
  0.10.9/0.11.0 lockfile duplication, cargo-deny, MSRV headroom, reproducibility)
  are thoroughly captured across Requirements, Open Questions, and Technical
  Notes.

### Recommended Changes

1. **Resolve the 0216↔0215 relationship into an explicit sequencing/decision
   edge** (addresses: the 0216↔0215 major; Fallback description conflicts with
   0215) — Promote the coupling from "relates to" to a stated go/no-go
   ordering (e.g. 0216 ships and is measured first; 0215 is then re-evaluated
   on architectural grounds), and rewrite the Open Questions fallback so it
   matches 0215's actual scope: remove the cache-hit sha256, rely on minisign's
   BLAKE2b for corruption detection, and preserve name/version binding by the
   cheaper means 0215 defines — not "move the corruption check to BLAKE2b".

2. **Fix AC 1's throughput threshold** (addresses: Throughput pass threshold
   conflicts) — Replace "at least openssl parity (~1,700 MB/s)" with a firm
   floor reflecting the achievable hardware rate (e.g. ≥1,500 MB/s, or a ≥2.5×
   improvement over the 555 MB/s soft baseline) and state how a
   hardware-enabled-but-sub-parity result is classified.

3. **Give AC 3's musl regression clause a baseline** (addresses: 'Does not
   regress where measurable') — Name the figure the musl result must not fall
   below (the pre-change musl throughput, or the 555 MB/s soft baseline) and
   state the condition under which the measurement is required rather than
   optional.

4. **Add a criterion for the musl runtime-safety property** (addresses: No
   criterion verifies the musl runtime-safety property) — Verify that execution
   on (or emulation of) an aarch64 CPU without the sha2 extension produces
   correct output rather than a SIGILL, or explicitly record that runtime
   verification on non-sha2 hardware is out of scope and why.

5. **Cover the fallback exit path in the definition of done** (addresses:
   Definition of done omits the fallback) — Add a criterion (or a note on the
   existing set) defining the accepted-fallback completion state: the fallback
   is recorded, 0215 is referenced as the chosen route, and the gap decision is
   captured.

6. **Tighten the remaining Acceptance Criteria** (addresses: 'All four shipped
   targets' never enumerated; 'Release artefacts remain reproducible' names no
   procedure) — List the four target triples explicitly and define "cleanly"
   (e.g. `cargo build --release` exits 0 with no new warnings per target);
   point AC 4 at a concrete reproducibility check (e.g. two clean rebuilds
   produce byte-identical outputs).

7. **Capture the two implied dependencies** (addresses: Assumed-available
   committed harness; Linux/musl hand-off to 0217) — Confirm
   `cli/launcher/tests/warm_terms.rs` exists and name the work that committed
   it as a prerequisite; record the 0216↔0217 measurement hand-off as ordering
   (whether 0217's linux measurement runs after and evidences this change, or
   whether 0216 must ship first).

8. **Minor clarity polish** (addresses: 'the same input' has no antecedent;
   specialist terms undefined; 'minisign's own digest' implicit) — Name the
   measurement input in AC 1; gloss MSRV/HWCAP/getauxval on first use; write
   "minisign's own digest (BLAKE2b)" in Context.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its core intent unambiguously: the
scope (enable the ARMv8 SHA-2 hardware backend, verify across four targets,
measure the gain) is consistent across Summary, Requirements, and Acceptance
Criteria, and outcomes are stated as concrete, observable states (throughput
≥ ~1,700 MB/s, clean builds, cargo-deny passing). The main clarity concerns are
peripheral: one cross-reference to work item 0215 that mischaracterises its
scope relative to what 0215 actually specifies, a dangling "the same input"
referent, and a few specialist terms used without definition. None obscures the
central deliverable.

**Strengths**:
- Scope is coherent across sections — the Summary's "switch the backend on,
  verify across all four shipped targets, measure the gain" is matched precisely
  by the Requirements and Acceptance Criteria, with no contradiction between a
  narrow stated aim and a broader implied one.
- Outcomes are expressed as observable system states rather than vague
  properties (throughput at openssl parity ~1,700 MB/s up from ~555 MB/s, clean
  builds on all targets, cargo-deny passing, before/after figures recorded).
- Domain terms that recur (soft vs hardware backend, `+sha2`, musl statics, the
  two lockfile sha2 versions 0.10.9/0.11.0) are used consistently in every
  section, so no term shifts meaning between uses.

**Findings**:
- 🔵 minor (confidence: medium) — **Fallback description conflicts with what
  0215 actually specifies** (Open Questions). The fallback is described as
  "accept the gap and move the corruption check to BLAKE2b (which minisign
  already computes) via 0215", but 0215 does not frame itself as moving the
  corruption check to BLAKE2b — it removes the cache-hit sha256 and explicitly
  warns that minisign's BLAKE2b signs bytes only and does *not* bind the asset's
  name/version, so a separate cheaper replacement (a post-exec version check) is
  required. Elsewhere in 0216 (Dependencies, References) 0215 is described
  accurately as "removes the cache-hit sha256". Impact: a reader taking the
  fallback branch could conclude BLAKE2b fully replaces the sha256's role and
  miss the name/version-binding gap that 0215 treats as the criterion the whole
  item turns on.
- 🔵 suggestion (confidence: medium) — **'the same input' has no antecedent in
  the work item** (Acceptance Criteria). AC 1's "runs over the same input" has
  no antecedent within 0216 — the file used to establish the ~555 MB/s baseline
  (the ~2.49 MB sub-binary, named only in the referenced 0205/0189 material) is
  never identified here. Impact: a reader running the before/after measurement
  has to leave the work item to discover which input makes the comparison valid,
  risking a non-comparable measurement.
- 🔵 suggestion (confidence: medium) — **Specialist acronyms/terms used without
  definition (MSRV, HWCAP, getauxval)** (Technical Notes). "MSRV 1.85 is under
  the pinned 1.90" and "getauxval/HWCAP works under `crt-static`" are used
  without definition. Impact: a competent developer without deep
  Rust-toolchain/Linux-libc background may have to look these up to judge whether
  the 0.11 remedy is safe on the musl statics.
- 🔵 suggestion (confidence: low) — **'minisign's own digest' equated with
  BLAKE2b only implicitly** (Context). Context states "minisign's own digest is
  already the faster of the two"; the identity minisign-digest = BLAKE2b is
  never stated at this point and is only confirmed later in Open Questions.
  Impact: a reader unfamiliar with minisign's internals may not immediately
  connect the phrase to the BLAKE2b figure just quoted.

### Completeness

**Summary**: Work item 0216 is an exceptionally complete task: every expected
section (Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, References) is
present and substantively populated, and the frontmatter carries a recognised
kind and status. The Summary states an unambiguous deliverable, the Context
motivates it with concrete measurements carried over from 0205/0189, and the
five acceptance criteria far exceed the minimum bar. The only completeness
observation is that the definition of done does not cover the fallback exit path
the item itself documents.

**Strengths**:
- All standard sections are present and none are empty, placeholder-only, or
  sparse — the item reads as fully specified for a task.
- Frontmatter integrity is strong: kind is a recognised value ("task"), status
  ("draft") matches the item's stage, and priority plus all relationship fields
  are populated.
- The Summary is a single, unambiguous action statement, with the diagnosis and
  the remaining work clearly separated.
- Acceptance Criteria contains five specific criteria (well above the
  two-criterion floor), each tied to a concrete outcome.
- Context explains why the work is needed with measured evidence rather than
  restating the summary, and Requirements give an implementer concrete starting
  actions.
- Optional sections that are genuinely relevant — Dependencies, Assumptions,
  Open Questions, Technical Notes — are all populated, and Drafting Notes usefully
  records the spike-to-task conversion and scope narrowing.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Definition of done omits the documented
  fallback exit path** (Acceptance Criteria). The item documents a fallback exit
  condition in Open Questions and Drafting Notes — if the hardware path cannot be
  made clean across the musl targets, accept the gap and move the corruption
  check to BLAKE2b via 0215 — but all five Acceptance Criteria assume the
  hardware backend is enabled successfully. No criterion states what "done" looks
  like if the fallback fires. Impact: an implementer who hits the musl blocker
  has no completion definition to satisfy and must return to the author.
  Suggestion: add a criterion (or a note on the existing ones) defining the
  accepted-fallback completion state. (Whether the fallback should be co-equal is
  a scope concern; this is only about the done-definition covering a stated path.)

### Dependency

**Summary**: Work item 0216 captures its measurement-provenance couplings (0189,
0205), the parent epic (0136), the sibling warm-path lever (0191), and — most
importantly — the bidirectional relationship with 0215 (which this task may
obviate, and which is also 0216's own fallback). The external crate coupling
(sha2 0.10 vs 0.11) and its ripple effects (lockfile duplication, cargo-deny,
MSRV, reproducibility) are captured in the body rather than the Dependencies
section, which is acceptable. The gaps are all sequencing/decision couplings
left as "relates to" or open questions rather than resolved into ordering: the
mutual-exclusion with 0215, the linux/musl measurement hand-off with 0217, and
the assumed-available committed measurement harness.

**Strengths**:
- The external dependency on the sha2 crate and its version choice is thoroughly
  captured across Requirements, Open Questions, and Technical Notes, including
  the downstream consequences — the 0.10.9/0.11.0 lockfile duplication,
  cargo-deny cleanliness, MSRV headroom, and release reproducibility — so the
  crate coupling is not hidden even though it lives outside the Dependencies
  section.
- The bidirectional relationship with 0215 is surfaced in both the Dependencies
  section and Open Questions: this task may make 0215 unnecessary, and 0215 is
  also named as 0216's fallback exit condition.
- Measurement provenance (0189 measured the rate, 0205 first measured it and
  recorded the BLAKE2b inversion) and the sibling warm-path lever (0191) are each
  named in Dependencies, and the parent epic 0136 and originating plan are
  captured via frontmatter and Dependencies.

**Findings**:
- 🟡 major (confidence: medium) — **0216/0215 mutual-obviation coupling left as
  an open question, not a sequencing edge** (Dependencies). Both operate on the
  same warm-dispatch sha256 — 0215 removes the cache-hit digest recomputation,
  0216 makes that same digest cheap via hardware intrinsics. 0216 states it "may
  make 0215 unnecessary" and also names 0215 as its own fallback, but both items
  record the relationship only as "relates to" plus open questions; neither
  captures the go/no-go ordering. Impact: both are draft tasks touching the same
  `reverify`/`verifier::sha256_hex` call site, so scheduling them in parallel
  risks redundant or directly conflicting changes and rework. Suggestion: promote
  the relationship to an explicit ordering/decision dependency (e.g. mark 0216 as
  gating the go/no-go on 0215).
- 🔵 minor (confidence: medium) — **Assumed-available committed measurement
  harness not captured as a prerequisite** (Requirements). Requirements and
  References treat `cli/launcher/tests/warm_terms.rs` as an already-committed
  harness that "already reports verifier::sha256_hex separately". The 0205 spike
  ran its instrumentation from a differently-named throwaway
  (`spike_0205_warm_terms.rs`) recorded as removed, and lists "the
  committed-harness decision" among still-open 0189 hand-offs. Impact: if the
  committed harness is not in fact present, the task cannot satisfy its
  measurement acceptance criteria, and that blocker is not visible in the
  dependency record.
- 🔵 minor (confidence: medium) — **Linux/musl measurement hand-off to 0217 not
  captured as ordering** (Dependencies). 0216 requires the change to build across
  all four targets and asserts throughput "does not regress where measurable" on
  an aarch64 musl target, but its own throughput measurement is scoped only to
  darwin-arm64. The Dependencies section names 0217 as the item that "measures
  warm dispatch on linux", implying the linux/musl evidence lands in 0217 — yet
  the two are coupled only as "relates to". Impact: the musl-intrinsics criterion
  depends on a capability that appears to belong to 0217, so without a captured
  hand-off the linux verification could fall between the two items.

### Scope

**Summary**: Work item 0216 is a well-scoped, coherent task: every requirement
and acceptance criterion serves the single purpose of enabling and verifying the
ARMv8 SHA-2 hardware backend for the launcher's sha256 path. The Summary,
Requirements, and Acceptance Criteria describe the same scope, and the recorded
spike-to-task conversion cleanly narrowed an open-ended investigation into a
concrete, bounded deliverable with the accept-the-gap/BLAKE2b path explicitly
demoted to a fallback rather than a co-equal option. The only scope-relevant
tension is the acknowledged overlap with sibling task 0215, which 0216 states it
"may make unnecessary".

**Strengths**:
- Single unified purpose — enable, verify, and measure the hardware SHA-2
  backend — with no "and also" bundling of independent concerns across the
  Requirements.
- Summary, Requirements, and Acceptance Criteria are mutually consistent and
  describe the same deliverable, so there is no cross-section scope drift.
- The Drafting Notes record a deliberate spike-to-task conversion that narrowed
  the scope from "investigate and recommend" to "enable the backend".
- The accept-the-gap / move-to-BLAKE2b path is scoped as a conditional fallback
  exit condition (delegated to 0215) rather than folded into this item as a
  second deliverable.
- Kind (task) is appropriate for the size: a focused workspace
  dependency/feature change verified across the four shipped targets.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Overlap with sibling task 0215 leaves
  the units-of-delivery boundary unresolved** (Open Questions). The item states
  that making the digest cheap "may make 0215 unnecessary" and asks whether it
  settles 0215's cache-hit-sha256 removal. Because 0216 (make the sha256 cheap)
  and 0215 (remove the sha256 call site) both target the same warm-dispatch cost,
  there is an unresolved question of whether both are genuinely distinct units of
  delivery. Impact: if unresolved before both are committed, a team could deliver
  overlapping optimisations, or 0216 could complete only to render an in-flight
  0215 moot — though 0216 retains independent value since the backend change
  improves every sha256 the Rust binaries compute. Suggestion: resolve the
  sequencing (e.g. 0216 delivered first, 0215 re-evaluated afterward).

_(Merged into the major "The 0216↔0215 relationship is left as an open question,
not a sequencing edge" in the aggregated findings above, alongside the dependency
lens's finding on the same coupling.)_

### Testability

**Summary**: This task carries mostly outcome-focused criteria, and Criterion 1
in particular is a clean Given/When/Then with a numeric threshold, a stated
baseline (~555 MB/s) and a named measurement harness — genuinely testable. The
weaknesses are around thresholds and coverage: the ~1,700 MB/s parity bar
conflicts with the item's own note that the hardware path may legitimately trail
openssl by 10–30%, Criterion 3's "does not regress where measurable" has no
baseline and can always be waived, and the central musl runtime-safety property
(no fault on CPUs lacking the sha2 extension) is not captured by any verifiable
criterion.

**Strengths**:
- Acceptance Criterion 1 is a well-formed Given/When/Then with a concrete numeric
  threshold, a stated starting baseline (~555 MB/s), and a named measurement
  harness (`cli/launcher/tests/warm_terms.rs`), giving a clear verification
  procedure.
- The Technical Notes supply an unambiguous fail discriminator — a result still
  in the ~500 MB/s band means the soft backend is still selected — which sharpens
  pass/fail interpretation of the throughput criterion.
- Criteria 4 and 5 include concrete present/absent artefact checks (the
  0.10.9/0.11.0 duplication effect recorded, before/after figures recorded) that
  are trivially verifiable.

**Findings**:
- 🟡 major (confidence: medium) — **Throughput pass threshold conflicts with the
  item's own residual-gap note** (Acceptance Criteria). AC 1 requires
  `verifier::sha256_hex` to reach "at least openssl parity (~1,700 MB/s)", but
  the Technical Notes acknowledge the hardware path may legitimately trail
  openssl by 10–30% (~1,200–1,540 MB/s), and the Drafting Notes themselves say
  "adjust if a different threshold is meaningful". A result in that band cannot
  be classified pass or fail. Suggestion: set a firm numeric floor reflecting the
  achievable hardware rate (e.g. ≥1,500 MB/s or ≥2.5× the 555 MB/s baseline) and
  state how a between-band result is classified.
- 🟡 major (confidence: high) — **'Does not regress where measurable' has no
  baseline and can always be waived** (Acceptance Criteria). AC 3's "throughput
  does not regress where measurable" defines no baseline figure for the aarch64
  musl target — Requirements only mandate before/after measurement on
  darwin-arm64 — and the qualifier "where measurable" lets the check be declared
  inapplicable, so the criterion can be argued satisfied regardless of result.
  Suggestion: state the musl baseline the result must not fall below and specify
  when the measurement is required rather than optional.
- 🟡 major (confidence: medium) — **No criterion verifies the musl runtime-safety
  property the item is built around** (Acceptance Criteria). The Summary and
  Requirements make musl safety central — the chosen path must not depend on
  forcing `-C target-feature=+sha2`, which "would fault on CPUs without the
  extension" — but the only related criterion (AC 3) checks that the build does
  not force `+sha2`, not that the resulting binary runs without an
  illegal-instruction fault. A clean build does not prove runtime detection
  works. Suggestion: add a criterion verifying execution on (or emulation of) an
  aarch64 CPU without the sha2 extension produces correct output rather than a
  SIGILL, or record that runtime verification is out of scope and why.
- 🔵 minor (confidence: medium) — **'All four shipped targets' are never
  enumerated and 'builds cleanly' is undefined** (Acceptance Criteria). AC 2
  requires the remedy to "build cleanly across all four shipped targets", but the
  four target triples are never enumerated (only darwin-arm64,
  aarch64-apple-darwin and aarch64-unknown-linux-musl are named in passing) and
  "cleanly" is not defined. Suggestion: list the four target triples explicitly
  and define "cleanly".
- 🔵 minor (confidence: low) — **'Release artefacts remain reproducible' names no
  verification procedure** (Acceptance Criteria). AC 4 requires artefacts to
  "remain reproducible" but references no procedure for confirming it. Suggestion:
  point to the reproducibility verification step or state the expectation
  concretely (e.g. two clean rebuilds produce byte-identical outputs).

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** COMMENT

The item was edited to address every pass-1 finding, then re-reviewed through
all five lenses. The four pass-1 majors are resolved. The re-review surfaced a
threshold contradiction — a regression from the pass-1 AC 1 edit, flagged
independently by clarity, testability, and completeness — which has been
reconciled in place during this pass, along with three smaller newly-surfaced
items. One recommended follow-up remains, and it requires editing a second work
item (0215) plus a scheduling decision; it does not block implementation.

### Previously Identified Issues

- 🟡 **Testability**: "Does not regress where measurable" has no baseline — Resolved (AC 3 reframed; the waivable clause is gone).
- 🟡 **Testability**: Throughput threshold conflicts with the residual-gap note — Resolved (floor moved to ≥2.5× the baseline; the parity-vs-band conflict is gone).
- 🟡 **Testability**: No criterion verifies the musl runtime-safety property — Resolved (AC 3 asserts the runtime-detected backend is selected, verified by config inspection; non-sha2-CPU execution recorded as an accepted limitation).
- 🟡 **Dependency / Scope**: 0216↔0215 coupling left as an open question — Resolved on 0216's side (now a gating edge with a no-parallel constraint; the units-of-delivery boundary is stated). See the new one-sided-edge issue below.
- 🔵 **Clarity**: Fallback description conflicts with 0215 — Resolved.
- 🔵 **Dependency**: Committed harness not captured as a prerequisite — Partially resolved (now a prerequisite with a confirm-exists guard; not yet attributed to a producing work item — see below).
- 🔵 **Dependency**: Linux/musl hand-off to 0217 not captured as ordering — Resolved.
- 🔵 **Testability**: Four targets not enumerated; "cleanly" undefined — Resolved (targets listed; "cleanly" defined, with the warnings baseline tightened this pass).
- 🔵 **Testability**: "Reproducible" names no procedure — Resolved (byte-identical rebuilds).
- 🔵 **Completeness**: Definition of done omits the fallback exit path — Resolved (AC 6).
- 🔵 **Clarity**: "the same input" has no antecedent — Resolved.
- 🔵 **Clarity**: MSRV / HWCAP / getauxval undefined — Resolved.
- 🔵 **Clarity**: "minisign's own digest" equated with BLAKE2b only implicitly — Resolved.

### New Issues Introduced

- 🟡 **Clarity + Testability + Completeness**: Success throughput threshold stated inconsistently across Summary, AC 1, Technical Notes (~2,000+ MB/s), and Drafting Notes (~1,700 MB/s) — introduced by the pass-1 AC 1 edit. **Addressed this pass**: AC 1 names the ~1,390 MB/s floor as the single binding gate; the contradictory "achievable hardware band" clause is removed; Drafting Notes and Technical Notes relabel ~1,700 / ~2,000+ as expectation/context, not the gate.
- 🟡 **Dependency**: The gating edge to 0215 is one-sided — 0215's own record encodes only a symmetric "relates to", omits the no-parallel-scheduling constraint, and still calls 0216 a spike. **Addressed by follow-up edit** — 0215 now records a reciprocal "gated by 0216" edge with the no-parallel constraint, adds `work-item:0216` to its `relates_to`, and drops the "spike" reference.
- 🔵 **Testability**: The musl runtime-detection mechanism had no defined verification procedure. **Addressed this pass**: AC 3 now names the config inspection (no `+sha2` forced, `sha2` 0.11 `aarch64-sha2` backend) as the check.
- 🔵 **Scope**: The accepted-fallback fork (AC 6) leaves a mild task/spike hybrid character — whether hardware enablement is a hard requirement or a fallback-permitted outcome is not pinned. **Addressed by follow-up edit** — hardware enablement pinned as the committed outcome; the fallback fires only on a genuine musl block (Open Questions + AC 6).
- 🔵 **Dependency**: The term-harness prerequisite is not attributed to the work item that lands `warm_terms.rs`. **Residual**.
- 🔵 **Clarity + Testability**: "No new warnings" (AC 2) lacked a defined baseline. **Addressed this pass**: baseline defined as an unchanged build of the same target.
- 🔵 **Clarity**: "0189's closing session" referent was not linked. **Addressed this pass**: `meta/measurements/warm-dispatch-3.json` added to References.

### Assessment

The work item is ready for planning. All four pass-1 majors are resolved, and
the threshold contradiction the re-review exposed — a regression from the pass-1
edit — was reconciled in place, so AC 1 now carries a single binding gate with
the higher figures labelled as expectation. Following the re-review, the two
decision-dependent residuals were also closed: 0215 was edited to reciprocate
the gating / no-parallel edge (and no longer calls this item a spike), and
hardware enablement was pinned as the committed outcome with the fallback
conditional on a genuine musl block. One minor residual remains — the
measurement harness is not attributed to the work item that lands it — closable
at scheduling. Verdict COMMENT rather than APPROVE on that single minor.

## Approval — 2026-09-10

**Verdict:** APPROVE

Approved by the reviewer. The single remaining minor — the measurement harness
not being attributed to the work item that lands it — is accepted as a
scheduling-time confirmation and does not block implementation. The frontmatter
verdict is set to APPROVE to reflect this decision.
