---
type: work-item-review
id: "0178-config-crates-native-yaml-reader-review-1"
title: "Work Item Review: config and config-adapters Crates with Native YAML Reader"
date: "2026-07-07T00:06:05+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
parent: "work-item:0166"
target: "work-item:0178"
work_item_id: "0178"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-07-07T00:34:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: config and config-adapters Crates with Native YAML Reader

**Verdict:** REVISE

0178 is a well-structured, source-anchored task: every section is present and
substantively populated, scope is a coherent domain/adapter crate pair, and its
verification strategy leans on strong tool-enforced boundary checks (cargo-pup,
cargo-deny) and a well-formed Given/When/Then legacy-guard case. The reason for
REVISE is concentrated in two areas — the acceptance criteria under-specify how
the headline new capability (arbitrary YAML nesting) is verified and against
what fixture set, and the downstream/consumer couplings of the config crates are
left implicit in Dependencies. No critical issues; the four major findings are
addressable with targeted edits to Acceptance Criteria and Dependencies.

### Cross-Cutting Themes

- **AC1 is an under-specified verification substrate** (flagged by: testability,
  clarity) — testability shows AC1's "match the bash reader on a shared corpus
  of fixtures" both relies on an undefined fixture corpus and cannot verify
  arbitrary nesting (the bash oracle caps at two levels); clarity independently
  notes the "recognised-key catalogue" is defined as five groups in
  Requirements/AC but silently gains the two doc-type arrays in Technical Notes.
  Together these mean "every recognised key" and "arbitrary nesting" lack a
  concrete, bounded pass/fail procedure.
- **Config-crates → consumer coupling not surfaced** (flagged by: scope,
  dependency) — the Requirements imply wiring "each sub-binary's composition
  root" while AC-3 scopes the deliverable to a single example; dependency notes
  the Dependencies section carries no Blocks despite 0167 and the sub-binaries
  (0169–0173) consuming the config half specifically. The config-half's
  downstream unblocking (esp. 0167) is invisible outside the parent.

### Findings

#### Critical

*(none)*

#### Major

- 🟡 **Testability**: Parity-with-bash oracle cannot verify the headline
  arbitrary-nesting capability
  **Location**: Acceptance Criteria (AC1)
  AC1 verifies via "behaviour matches the bash reader", but Context states the
  bash reader caps nesting at two levels — so for depth ≥3 (the capability this
  task adds) there is no oracle to match, and "arbitrary nesting" is unbounded
  language that cannot be exhaustively tested.

- 🟡 **Testability**: "Shared corpus of fixtures" is the verification substrate
  but is never defined
  **Location**: Acceptance Criteria (AC1)
  AC1's entire parity guarantee rests on a fixture corpus that is never
  enumerated, located, or specified; two verifiers could build very different
  corpora and both claim AC1 passed, so there is no guaranteed coverage floor.

- 🟡 **Testability**: Inline-array typed-sequence parsing has no verifying
  criterion
  **Location**: Requirements / Acceptance Criteria
  Technical Notes and the Size rationale call out inline-array parsing (e.g.
  `review.core_lenses`, serde typed sequences replacing `config_parse_array`) as
  distinct work, but AC1's "resolves every recognised key" does not distinguish
  typed-list from scalar resolution, so sequence handling could ship unverified.

- 🟡 **Dependency**: No Blocks entries despite config crates being the direct
  input to 0167 and sub-binaries 0169–0173
  **Location**: Dependencies
  0178 delivers the config crates that 0167 and 0169–0173 consume, but its
  Dependencies lists only "Blocked by: 0166" and no Blocks. Because 0167 needs
  only the config half (not the corpus siblings), the parent-level tracking hides
  that 0167 becomes unblockable the moment 0178 lands.

#### Minor

- 🔵 **Clarity**: "recognised-key catalogue" scope differs between sections
  **Location**: Requirements
  Requirements/AC define the catalogue as five groups (`paths.*`, `templates.*`,
  `work.*`, `review.*`, `agents.*`), but Technical Notes treat the two doc-type
  parallel arrays as an additional part, leaving the true scope of "every
  recognised key" ambiguous.

- 🔵 **Dependency**: First-mover activation of `cli/deny.toml` and `cli/pup.ron`
  shares artefacts with sibling 0179
  **Location**: Requirements
  0178 is the first-mover activation of the inert deny/pup rules, which 0179
  (corpus) must then extend on the same files. This ordering/coordination
  coupling is not captured in either item's Dependencies.

- 🔵 **Testability**: The "do not port ACCELERATOR_MIGRATION_MODE fallback"
  requirement has no verifying criterion
  **Location**: Requirements (fail-closed legacy guard)
  The negative requirement (fallback must not be ported) has no AC; negative
  requirements silently pass by default, so an accidental re-introduction would
  go undetected.

#### Suggestions

- 🔵 **Scope**: "Wired at each sub-binary's composition root" implies scope over
  the consumer work items
  **Location**: Requirements
  The Requirement wording reads as delivering wiring across every consumer
  binary, but AC-3 scopes it to one composition-root example and the consumers
  are separate items (0167–0173); align the Requirement with the AC.

- 🔵 **Clarity**: "Model 1" used without definition or link
  **Location**: Requirements
  "(Model 1)" is used for the composition-root wiring but never defined here or
  in the parent; define it inline or link to the design note.

- 🔵 **Clarity**: "corpus" reused generically alongside the domain-specific
  corpus crate
  **Location**: Acceptance Criteria
  AC1's "shared corpus of fixtures" overloads "corpus", which is also the sibling
  crate name; a neutral phrase ("a shared fixture suite") avoids the collision.

- 🔵 **Testability**: Legacy-guard AC states "a message" but the exact output
  shape lives only in Technical Notes
  **Location**: Acceptance Criteria (AC2)
  AC2 says "exits non-zero with a run /accelerator:migrate message" while
  Technical Notes pins "two stderr lines then exit 1"; fold exit code, stream,
  and message content into AC2.

### Strengths

- ✅ Structurally and informationally complete: every expected section is present
  and substantively populated, and the frontmatter carries all required fields
  with a recognised `kind` (task).
- ✅ Coherent, indivisible scope: the `config` domain and `config-adapters`
  outbound crates are a hexagonal pair that cannot be delivered or rolled back
  independently; the cargo-deny/cargo-pup activation is the boundary enforcement
  of precisely these two crates.
- ✅ Strong verification anchors where an oracle exists: AC2 is a fully-formed
  Given/When/Then case, and AC3/AC4 delegate to cargo-pup/cargo-deny for binary,
  reproducible pass/fail signals.
- ✅ Exhaustive, line-referenced source anchors in Technical Notes (bash
  catalogue files, the launcher version-hexagon template, the enforcement
  scaffolding), so an implementer can begin without follow-up questions; the
  42-key count reconciles cleanly across the five groups plus the doc-type arrays.
- ✅ Assumptions pre-empt a hidden coupling by recording that the bash 2-level
  reader and the visualiser's JSON-reading `config.rs` are non-reusable.

### Recommended Changes

1. **Rework AC1 into bounded, oracle-appropriate criteria** (addresses:
   Parity-with-bash oracle cannot verify arbitrary nesting; "Shared corpus of
   fixtures" never defined; "recognised-key catalogue" scope differs)
   Split AC1 into (a) parity against the bash reader for keys at nesting depth
   ≤2 over an explicitly enumerated fixture set, and (b) deep-nesting (a bounded
   representative depth, e.g. 3–4 levels) verified against declared expected
   resolved values rather than bash parity. Define the fixture corpus concretely
   — at least one fixture per recognised key across the five groups, a team/local
   precedence-conflict case, and a default-fallback case (`work.id_pattern =
   {number:04d}`) — and state whether the doc-type parallel arrays are part of
   the catalogue AC1 must cover.

2. **Add an inline-array parsing criterion** (addresses: Inline-array
   typed-sequence parsing has no verifying criterion)
   Assert that inline-array keys (e.g. `review.core_lenses`) resolve to a typed
   sequence with the expected element list for a stated fixture, distinct from
   scalar resolution.

3. **Add a negative criterion for the ACCELERATOR_MIGRATION_MODE non-port**
   (addresses: negative requirement has no verifying criterion)
   Given `ACCELERATOR_MIGRATION_MODE` set and a legacy `.claude/accelerator.md`
   layout, the Rust reader still exits non-zero (fails closed) rather than
   honouring the fallback.

4. **Populate Dependencies with Blocks and enforcement-ownership notes**
   (addresses: No Blocks entries; First-mover activation shares artefacts with
   0179)
   Add explicit Blocks entries for the items that consume the config crates
   specifically (at minimum 0167, plus the sub-binaries whose composition roots
   wire config-adapters), and note that 0178 owns the first activation of
   `cli/deny.toml` / `cli/pup.ron` while 0179/0180 extend rather than re-activate
   them.

5. **Align the Requirements wording with AC-3 and tighten AC2 / clarity nits**
   (addresses: "Wired at each sub-binary's composition root" implies consumer
   scope; AC2 output shape; "Model 1" undefined; "corpus" overloaded)
   State that 0178 delivers the crates plus one composition-root example
   demonstrating Model 1 wiring (defined inline), with per-consumer wiring owned
   by 0167–0173; fold exit code 1 + stderr + message content into AC2; and use a
   neutral "fixture suite" phrasing in AC1.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous: scope is consistent across
Summary, Requirements, and Acceptance Criteria, actors and outcomes are concrete
(the reader/guard exits non-zero with a named message, cargo-pup/cargo-deny
enforce the domain boundary), and domain vocabulary is standard for this
project. The main clarity gaps are a parenthetical "Model 1" used without
definition, a slight inconsistency in what "the recognised-key catalogue"
encompasses, and a re-use of the loaded term "corpus" in a generic sense.

**Strengths**:
- Scope is internally consistent — the Summary's two-crate split, the
  Requirements, and the Acceptance Criteria all describe the same boundary with
  no contradictions.
- Outcomes are stated as observable system states (legacy guard exits non-zero
  with the migrate message; boundary rule enforced by cargo-pup/cargo-deny).
- The 42-key count reconciles cleanly across Technical Notes (17 + 6 + 3 + 9 + 7),
  so the numeric claims are internally coherent.

**Findings**:
- 🔵 minor (confidence: medium) — Requirements: "recognised-key catalogue" scope
  differs between sections. Requirements/AC define the catalogue as five groups
  while Technical Notes treat the two doc-type parallel arrays as an additional
  part, leaving "every recognised key" ambiguous. Suggestion: state consistently
  whether the doc-type arrays are part of the catalogue.
- 🔵 suggestion (confidence: medium) — Requirements: "Model 1" used without
  definition or link; define inline or link to the model taxonomy.
- 🔵 suggestion (confidence: low) — Acceptance Criteria: "corpus" reused
  generically ("shared corpus of fixtures") alongside the domain-specific corpus
  crate; use a neutral phrase such as "a shared fixture suite".

### Completeness

**Summary**: This task-kind work item is structurally and informationally
complete: every expected section is present and substantively populated, and the
frontmatter carries all required fields with a recognised `kind`. The
Requirements and Technical Notes give an implementer a fully-scoped,
source-referenced definition of the work. No completeness gaps rise to
actionable severity.

**Strengths**:
- Frontmatter integrity is complete (kind, status, priority, id, parent, and
  housekeeping fields all present and recognised).
- Summary is a single unambiguous statement of what is being built.
- Context explains motivating forces (ADR-0047, the 2-level bash cap,
  non-reusability of the visualiser's JSON reader) rather than restating Summary.
- Requirements map concretely to Acceptance Criteria; Technical Notes give
  exhaustive line-referenced source anchors.
- Dependencies and Assumptions carry genuinely relevant content, not placeholders.

**Findings**: *(none)*

### Dependency

**Summary**: Task 0178 captures its upstream blocker (parent 0166 and the
crate-layer conventions it establishes) but leaves its downstream side almost
entirely implicit: the Dependencies section lists no Blocks despite the config
crates being the direct input to 0167 and being wired into every sub-binary
(0169–0173). The task also owns a first-mover activation of shared enforcement
files that sibling 0179 must extend, an inter-sibling coupling that is not
surfaced. Upstream reliance on the launcher version hexagon and cli workspace
skeleton is captured only transitively through the parent.

**Strengths**:
- Dependencies explicitly names parent 0166 as blocker and captures the
  crate-layer-conventions coupling.
- Assumptions records that the bash 2-level reader and visualiser `config.rs`
  are non-reusable, pre-empting a hidden coupling.
- Technical Notes give precise file/line traceability to source bash and the
  launcher hexagon template.

**Findings**:
- 🟡 major (confidence: medium) — Dependencies: No Blocks entries despite config
  crates being the direct input to 0167 and sub-binaries 0169–0173. Add explicit
  Blocks for the items consuming the config crates specifically.
- 🔵 minor (confidence: medium) — Requirements: First-mover activation of
  `cli/deny.toml` and `cli/pup.ron` shares artefacts with sibling 0179; note the
  ownership/ordering so the two do not collide.
- 🔵 suggestion (confidence: low) — Dependencies: Reliance on the launcher
  version hexagon (0164) and cli workspace skeleton (0163) captured only via the
  parent; optionally note them as satisfied upstream references.

### Scope

**Summary**: 0178 is a coherent, well-scoped task delivering the config half of
parent story 0166: the `config` domain crate and its `config-adapters` outbound
crate form a single indivisible hexagonal unit. The Summary, Requirements, and
Acceptance Criteria all describe the same scope, and the decomposition of 0166
into 0178/0179/0180 is a clean split along crate/capability lines. The one scope
signal is a wording tension between a Requirement implying wiring of every
consumer sub-binary and the AC scoping that down to a single composition-root
example.

**Strengths**:
- The two crates are a domain/adapter pair that cannot be delivered or rolled
  back independently — bundling them is correct, not scope creep.
- Parent 0166 is decomposed along clean crate boundaries; 0178 owns exactly one
  with no overlap into sibling tasks.
- The cargo-deny/cargo-pup activation is the boundary enforcement of precisely
  these two crates, so it belongs with this split.
- Summary, Requirements, and Acceptance Criteria are mutually consistent, with no
  orphan scope in either direction.

**Findings**:
- 🔵 suggestion (confidence: medium) — Requirements: "Wired at each sub-binary's
  composition root" implies scope over the consumer work items (0167–0173); align
  the Requirement with AC-3 so per-consumer wiring stays owned by the consumers.

### Testability

**Summary**: The four Acceptance Criteria lean on strong verification strategies
— differential parity against the bash reader, tool-enforced boundary checks
(cargo-pup/cargo-deny), and a well-formed Given/When/Then legacy-guard case. The
central weakness is AC1: its stated oracle (parity with the bash reader) cannot
verify the headline new capability (arbitrary nesting), because the bash reader
caps at two levels, and the "shared corpus of fixtures" that underpins the whole
parity claim is never defined. Two flagged behaviours (inline-array typed-sequence
parsing and the deliberately-omitted migration-mode fallback) have no verifying
criterion.

**Strengths**:
- AC2 is a fully-formed Given/When/Then case anchored to a named bash parity
  point (`config_assert_no_legacy_layout`).
- AC3 and AC4 delegate verification to cargo-pup and cargo-deny for binary,
  reproducible pass/fail signals; AC4 frames verification as the ban actively
  "biting".
- The recognised-key catalogue is fully enumerated in Technical Notes (42 keys),
  bounding AC1's "every recognised key" to a concrete, countable set.
- Differential testing against the existing bash reader is a strong strategy
  wherever a bash oracle actually exists (nesting depth ≤2).

**Findings**:
- 🟡 major (confidence: high) — Acceptance Criteria (AC1): Parity-with-bash
  oracle cannot verify the headline arbitrary-nesting capability. Split AC1 so
  deep nesting is verified against declared expected values at a bounded depth,
  reserving parity for ≤2-level keys.
- 🟡 major (confidence: medium) — Acceptance Criteria (AC1): "Shared corpus of
  fixtures" is the verification substrate but is never defined. Specify the corpus
  explicitly or reference a committed fixtures path.
- 🟡 major (confidence: medium) — Requirements / Acceptance Criteria:
  Inline-array typed-sequence parsing has no verifying criterion. Add a criterion
  asserting inline-array keys resolve to a typed sequence with the expected
  element list.
- 🔵 minor (confidence: medium) — Requirements (fail-closed legacy guard): The
  "do not port ACCELERATOR_MIGRATION_MODE fallback" requirement has no verifying
  criterion; add a negative case (fallback set + legacy layout still fails closed).
- 🔵 suggestion (confidence: low) — Acceptance Criteria (AC2): AC2 states "a
  message" but the exact output shape lives only in Technical Notes; fold exit
  code, stream, and message content into AC2.

## Re-Review (Pass 2) — 2026-07-07T00:23:35+00:00

**Verdict:** REVISE

Re-ran the four lenses that had findings (clarity, dependency, scope,
testability); completeness was clear in pass 1 and not re-run. **All ten
original findings are resolved.** The verdict remains REVISE only because the
edits introduced two new medium-confidence major findings (an unnamed
"config-reader entry point" artifact now that consumer binaries are explicitly
out of scope, and AC-8's cargo-deny "bites" claim lacking a committed violating
canary). The remaining new findings are narrow consistency nits, several of them
direct artifacts of the pass-1 edits.

### Previously Identified Issues

- 🟡 **Testability** (AC1): parity oracle cannot verify arbitrary nesting —
  **Resolved** (new AC-3 verifies depth ≥3 against declared expected values).
- 🟡 **Testability** (AC1): "shared corpus of fixtures" undefined — **Resolved**
  (new fixture-suite criterion enumerates the substrate).
- 🟡 **Testability**: inline-array parsing has no criterion — **Resolved** (new
  AC-4).
- 🟡 **Dependency**: no Blocks entries — **Resolved** (Blocks 0167 + 0169–0173,
  plus enforcement-ownership and 0163/0164 template notes).
- 🔵 **Clarity**: "recognised-key catalogue" scope differs — **Resolved**
  (Requirements now name the doc-type arrays).
- 🔵 **Dependency**: first-mover deny/pup activation shared with 0179 —
  **Resolved** (enforcement-ownership note added).
- 🔵 **Testability**: `ACCELERATOR_MIGRATION_MODE` non-port unverified —
  **Resolved** (new AC-6 negative case).
- 🔵 **Scope**: "wired at each sub-binary" implies consumer scope — **Resolved**
  (Requirements defer per-consumer wiring to 0167–0173).
- 🔵 **Clarity**: "Model 1" undefined — **Resolved** (defined inline).
- 🔵 **Clarity**: "corpus" overloaded — **Resolved** (now "shared fixture suite").
- 🔵 **Testability** (AC2): output shape only in Technical Notes — **Resolved**
  (AC-5 pins exit 1 + stderr + `/accelerator:migrate` directive).

### New Issues Introduced

- 🟡 **Clarity** (major): AC-5/AC-6 test "a config-reader entry point", but
  Requirements now exclude consumer binaries and AC-7 calls the same thing a
  "composition-root example" — the runnable artifact under test is unnamed and
  referred to by three terms.
- 🟡 **Testability** (major): AC-8's "ban is active and bites" has no committed
  violating canary, so it could be argued met by mere config presence without
  demonstrating cargo-deny actually fails a bad build.
- 🔵 **Clarity** (minor): consumer set given as 0167–0173 in Requirements but
  0169–0173 in Dependencies — the two ranges disagree by two items.
- 🔵 **Clarity** (minor): `WORK_INTEGRATION_VALUES` appears in the catalogue
  enumeration but is excluded from the 42-key total without explanation.
- 🔵 **Clarity** (suggestion): AC-5 says "exit code 1" while AC-6 says
  "non-zero" — unclear whether the migration-mode path must exit exactly 1.
- 🔵 **Testability** (minor): AC-3 depth ≥3 coverage is "representative" with no
  stated minimum key/fixture count.
- 🔵 **Testability** (minor): default-fallback verification pinned to one key
  (`work.id_pattern`) despite multiple defaulted keys (incl. inline-array review
  defaults).
- 🔵 **Dependency** (minor): sibling ordering (0179/0180 extend the deny/pup
  scaffolding) stated only as prose, not surfaced in Blocks.
- 🔵 **Dependency** (minor): parent-listed consumer 0168 not addressed in Blocks
  (presumably consumes corpus, not config — left implicit).
- 🔵 **Dependency** (suggestion): the new YAML crate appears only in Technical
  Notes, not Dependencies, and its choice (the cargo-deny ban target) is undecided.
- 🔵 **Scope** (suggestion): the deny/pup enforcement activation is absent from
  the Summary though it is a distinct deliverable.

### Assessment

The pass-1 edits fully cleared every original finding — the substantive
testability and dependency gaps that drove the REVISE are gone. The residual
REVISE rests on two new majors that are quick, low-risk consistency fixes:
name the composition-root example as the single "config-reader entry point"
under test (unifying the three terms across AC-5/6/7), and attach a violating
canary to AC-8. Fixing those two plus the range/exit-code nits would carry the
item to APPROVE without further structural change.

## Re-Review (Pass 3) — 2026-07-07T00:30:55+00:00

**Verdict:** COMMENT

Re-ran clarity and testability (where the pass-2 majors sat). **Both new majors
are resolved and no major or critical findings remain across the item.** The
verdict moves from REVISE to COMMENT: the work item is acceptable and
implementation-ready, with only optional minor/suggestion polish outstanding.

### Previously Identified Issues (pass 2 → pass 3)

- 🟡 **Clarity** (major): unnamed "config-reader entry point" — **Resolved**
  (Requirements name the composition-root example as the entry point; AC-5/6/7
  now use the term consistently).
- 🟡 **Testability** (major): AC-8 "bites" had no committed canary — **Resolved**
  (AC-8 now requires a deliberately-violating canary confirmed to fail
  cargo-deny, and states presence alone does not satisfy it).
- 🔵 **Clarity** (minor): consumer range 0167–0173 vs 0169–0173 — **Resolved**
  (Requirements now read "0167 and 0169–0173"; Dependencies documents 0168 as a
  corpus consumer).
- 🔵 **Clarity** (minor): `WORK_INTEGRATION_VALUES` excluded from 42-key total —
  **Resolved** (noted as a value-domain constraint, excluded from the count).
- 🔵 **Clarity** (suggestion): AC-5 "code 1" vs AC-6 "non-zero" — **Resolved**
  (AC-6 now says "code 1, same as the previous criterion").
- 🔵 **Testability** (minor): AC-3 "representative" without a count — **Resolved**
  (now ≥1 each of a 3-level scalar, 4-level scalar, nested inline-array).
- 🔵 **Testability** (minor): default-fallback pinned to one key — **Partially
  resolved** (AC-1 fixture suite now spans every defaulted key; a direct
  expected-value assertion is still only transitive — see below).
- 🔵 **Dependency** (minor): 0179/0180 ordering only in prose — **Resolved**
  (Dependencies states they are ordered after this task).
- 🔵 **Dependency** (minor): 0168 not addressed — **Resolved** (documented).
- 🔵 **Dependency** (suggestion): YAML crate only in Technical Notes —
  **Resolved** (surfaced in Dependencies + a new Open Question).
- 🔵 **Scope** (suggestion): enforcement activation absent from Summary —
  **Resolved** (Summary now names the cargo-deny/pup activation).

### New Issues Introduced

- 🔵 **Clarity** (minor): AC-8's "the YAML infra crate" is ambiguous between
  `config-adapters` and the third-party YAML library; name the third-party crate
  (e.g. `serde_yml`) so the canary target is unambiguous without Technical Notes.
- 🔵 **Clarity** (minor): "port" is used as both a hexagonal noun and a verb
  ("port `config_assert_no_legacy_layout`"); use "reimplement/reproduce" for the
  bash-to-Rust sense.
- 🔵 **Testability** (suggestion): AC-7's first clause ("constructs the reader
  from concrete adapters") has no independent observable outcome; tie it to
  running the example against a fixture and resolving a key to its value.
- 🔵 **Testability** (suggestion): default-fallback is verified only transitively
  via bash parity; add a direct expected-default assertion mirroring AC-3.
- 🔵 **Clarity** (suggestion): the 42-key phrasing still reads as if the doc-type
  arrays are inside the 42; disjoin them.
- 🔵 **Clarity** (suggestion): gloss "team (`.accelerator/config.md`) → local
  (`.local`)" on first use.

### Assessment

0178 is now implementation-ready. Every structural and major concern from the
initial review and its first re-review is closed; what remains is optional
terminology and observability polish, none of which blocks planning or
implementation. The one worth doing before implementation starts is naming the
third-party YAML crate in AC-8 so the canary target is unambiguous.

## Approval — 2026-07-07T00:34:39+00:00

**Verdict:** APPROVE (reviewer override of the pass-3 COMMENT)

The two worth-doing polish items were applied after pass 3 — AC-8 now names the
third-party YAML library (`serde_yml`) as the canary target, and the legacy-guard
requirement uses "reimplement/reproduce" instead of the overloaded "port". With
no major/critical findings and the notable clarity nits closed, the reviewer
approves 0178 for implementation. The remaining suggestions (AC-7 observability,
a direct default-value assertion, the 42-key phrasing, the team/local gloss) are
optional and may be folded in during planning.
