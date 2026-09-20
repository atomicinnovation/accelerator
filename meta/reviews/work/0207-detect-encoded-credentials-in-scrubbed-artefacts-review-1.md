---
type: "work-item-review"
id: "0207-detect-encoded-credentials-in-scrubbed-artefacts-review-1"
title: "Work Item Review: Detect Encoded Credentials In Scrubbed Artefacts"
date: "2026-09-10T21:34:12+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0207"
work_item_id: "0207"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-10T22:20:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Detect Encoded Credentials In Scrubbed Artefacts

**Verdict:** REVISE

Work item 0207 is structurally complete, densely written, and tightly scoped: every requirement extends one scanner function toward a single security goal, thresholds are named, and the trade-offs are recorded. It fails review on precision rather than substance — the whitespace-normalisation mechanism as specified cannot catch the leakage shape it targets, the 16-character length gate is stated against two different quantities ("value" vs derived "form"), and the encoding derivations are named but not pinned to an alphabet or character set. These ambiguities are load-bearing in a security scrubber where a false negative leaks a credential and an over-refusal is unrecoverable, so they should be closed before implementation.

### Cross-Cutting Themes

- **Whitespace-normalisation is self-defeating as specified** (flagged by: testability, clarity) — `whitespace_normalise(V)` equals `V` for a credential with no interior whitespace, so the derived needle is identical to the literal value and cannot match an artefact where a line wrap *inserted* whitespace into the token. Normalisation must apply to the artefact (haystack) side, not the needle. This is the single highest-signal finding.
- **Truncation scope contradicts itself** (flagged by: clarity, testability) — Context motivates "an ellipsis in the middle of a long token", but Drafting Notes exclude mid-token truncation from detection and the prefix rule catches only the head. The in/out boundary is never reconciled, and the deliberately-excluded case has no pinning negative test.
- **Encoding and length definitions are imprecise** (flagged by: testability, clarity) — base64 alphabet/padding and the percent-encoding character set are unspecified, and the 16-char gate reads on "value" in some sections and "form" in others. A short value with a long base64 form falls on different sides of the rule depending on which reading holds.
- **The upstream scanner dependency is provenance, not a blocker** (flagged by: dependency, completeness) — the entire scanner surface this work extends is produced by plan `2026-08-11-0196-design-cli-migration`, which is still status `ready`, yet the coupling appears only as "Derived from" with no explicit "Blocked by".

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: Whitespace-normalisation criterion is not verifiable as intended (tautological)
  **Location**: Acceptance Criteria / Requirements
  The needle is derived as `whitespace_normalise(V)` and matched against the raw artefact, but a credential value has no interior whitespace, so the derived needle equals `V` — the existing literal match. No input both follows the mechanism and demonstrates reflow detection.

- 🟡 **Clarity**: 16-character threshold stated against "value" in some sections and "form" in others
  **Location**: Acceptance Criteria
  Requirements bullet 2 and the pseudocode gate on derived *form* length (`if len(f) >= 16`); AC4 and Drafting Notes gate on the *value*. A short raw value can produce a 16+ char base64 form, so the two readings behave differently.

- 🟡 **Clarity**: Context motivates mid-token-ellipsis truncation that Drafting Notes exclude
  **Location**: Context
  Context lists "truncated with an ellipsis in the middle of a long token" as a target shape; Drafting Notes state mid-token truncation is "deliberately not given their own detection". Whether such a token is caught depends on 12 characters surviving before the ellipsis — a condition stated nowhere.

- 🟡 **Testability**: Encoding transformations undefined, so inputs cannot be constructed to exercise them
  **Location**: Requirements / Technical Notes
  base64 (standard/URL-safe, padded/unpadded) and the percent-encoding byte classes are unspecified. For an alphanumeric value, `percent_encode(V)` yields `V` unchanged, so the "percent-encoding is detected" criterion can pass without testing percent-encoding.

- 🟡 **Testability**: False-positive criterion is near-tautological and does not pin the P/M boundary
  **Location**: Acceptance Criteria (false-positive bullet)
  "A realistic false-positive candidate … is not flagged" names no input; a tester can always pick a string sharing fewer than 12 leading characters, passing by construction. The prefix-collision risk that P=12/M=16 exists to guard is left unpinned.

- 🟡 **Dependency**: Upstream scanner is named as provenance, not as a blocking prerequisite
  **Location**: References
  The scanner (`leaked_credentials.rs`, `NamedSecret::needles()`, `scan()`, the `AUTH_HEADER` value-half split, `CREDENTIAL_VARIABLES`) is created by Phase 2 of plan `2026-08-11-0196-design-cli-migration`, still status `ready`. A scheduler reading "Derived from" as idea-provenance would not see the ordering constraint before the sprint.

#### Minor

- 🔵 **Clarity**: "whitespace-reflowed" (Summary) versus "whitespace-normalisation" (everywhere else)
  **Location**: Summary
  A reader cannot be certain "reflowed" (what the model does) and "normalisation" (what the scanner derives) name the same phenomenon for a core transformation.

- 🔵 **Testability**: "Are detected" does not state the observable outcome per encoding
  **Location**: Acceptance Criteria (first bullet)
  The first criterion says the encodings "are detected" without stating the observable (exit 1, artefact not written, variable named); those outcomes are pinned only in later, separate criteria, so a per-encoding test could assert a weaker observable.

- 🔵 **Dependency**: `AUTH_HEADER` value-half extraction consumed as a fixed contract, not named as a coupling
  **Location**: Requirements
  This work derives new needles from the migration's first-colon value-half split. If that extraction shape changes, the auth-header input is silently lost, and nothing in the record links the two.

#### Suggestions

- 🔵 **Testability**: Deliberately-unhandled truncation cases lack a pinning negative test
  **Location**: Acceptance Criteria / Drafting Notes
  The short-value trade-off is pinned by a criterion, but the long-value mid/tail-truncation non-behaviour is asserted in prose only. A future widening into mid-token truncation would fail no criterion.

- 🔵 **Scope**: `kind: story` may fit `task` better given the single-function footprint
  **Location**: Frontmatter: kind
  The work extends one function in one file plus tests, which reads closer to a task; leave as `story` if team norms treat security-hardening increments as stories.

- 🔵 **Scope**: Story fuses two independently-deliverable mechanisms (encoding derivation vs prefix matching)
  **Location**: Requirements
  Either could ship without the other; Requirement 3 is the only coupling. Noted so the fusion is a conscious choice — no split needed unless the prefix rule proves separable.

- 🔵 **Completeness**: No explicit Dependencies section though the work extends existing scanner code
  **Location**: Dependencies (absent)
  A reader must infer from Technical Notes and References whether the prerequisite code has landed, rather than seeing it stated. (Overlaps the dependency major above.)

- 🔵 **Clarity**: "needle" used without definition on first use
  **Location**: Context
  The search-string metaphor is standard but unglossed; a reader outside the team must infer it before "needle form" / "prefix needle" read cleanly.

### Strengths

- ✅ The Summary is a canonical story statement — actor (a developer running `accelerator design scrub-secrets`), capability, and motivating "so that" — graspable without follow-up.
- ✅ Thresholds are named once (P = 12, P/M = 16) and reused consistently, and the hard-refusal contract (exit 1, artefact not written) and name-only-reporting invariant are stated identically across Context, Requirements, and Acceptance Criteria.
- ✅ Acceptance Criteria are eight and mostly scenario-shaped, including explicit false-positive-avoidance and too-short-value criteria that close deliberate trade-offs with tests.
- ✅ Drafting Notes record the rationale for every accepted trade-off (why P/M were chosen, why hard refusal is retained, why mid/tail truncation is excluded), matching a `ready` status with no open decisions.
- ✅ Scope is atomic and cleanly bounded — one crate, one file, standalone value, deliverable and rollback-able on its own — and the upstream code artefacts it builds on are named precisely in Technical Notes.
- ✅ The exit-1 hard-refusal contract is held constant, introducing no new downstream coupling to the invoking skills' exit-code branching.

### Recommended Changes

1. **Redefine whitespace handling to normalise the artefact, not the needle** (addresses: Whitespace-normalisation tautological; whitespace-reflowed vs normalisation)
   Restate the requirement and criterion so normalisation applies to the haystack: "Given a configured value rendered with whitespace inserted mid-token across a line wrap, when scrubbed, then the variable is named." Give the exact reflow input so the test demonstrates a match the literal scan misses. Align the Summary term with the mechanism.

2. **Pick one quantity — "value" or derived "form" — for the 16-character gate** (addresses: 16-char threshold value-vs-form)
   Restate AC4 and Drafting Notes in terms of "form" length to match the pseudocode, or explicitly note the threshold applies per derived form (so a short value may gain a prefix needle via its base64 form). The two readings diverge for short values with long encodings.

3. **Reconcile the truncation scope and pin the excluded case** (addresses: Context/Drafting-Notes contradiction; missing negative test)
   State that the prefix rule catches a truncated token only when its leading 12 characters survive intact, and clarify whether the Context's "ellipsis in the middle" shape is in or out of scope. Add a negative criterion: "a long value truncated in its middle or tail is not flagged", with a concrete input.

4. **Specify the exact base64 alphabet/padding and percent-encoding character set** (addresses: Encodings undefined)
   Pin these in Requirements or Technical Notes, and require each encoding's test to use a value containing characters the encoding actually transforms, so the derived form differs from the raw value.

5. **Pin the P/M boundary with concrete cases** (addresses: False-positive tautological)
   Assert that an artefact sharing an 11-character prefix (P−1) with a configured value is not flagged while a 12-character prefix is, using stated example strings, so the test exercises the exact threshold.

6. **Make the upstream dependency an explicit blocker** (addresses: Provenance-not-blocker; AUTH_HEADER contract; absent Dependencies section)
   Add a Dependencies section naming plan `2026-08-11-0196-design-cli-migration` Phase 2 (`scrub-secrets` / `leaked_credentials.rs`) as "Blocked by", with its current landed state, and note that this work consumes the `AUTH_HEADER` value-half extraction as a fixed contract.

7. **State the observable outcome in the first acceptance criterion** (addresses: "Are detected" outcome unstated)
   Extend it: "…is detected: the command exits 1, no artefact is written, and only the variable name is reported", or cross-reference the outcome criteria so each encoding case inherits the full contract.

## Per-Lens Results

### Clarity

**Summary**: Generally precise and internally disciplined — P/M thresholds defined explicitly, consistent exit-code language, name-only reporting stated identically everywhere. The most significant problem is a contradiction between the truncation shape the Context motivates ("an ellipsis in the middle of a long token") and the Drafting Notes' explicit exclusion of mid-token truncation. A secondary issue is the 16-character threshold stated against a "value" in some sections and a "form" in others — quantities that differ once encodings are derived.

**Strengths**:
- Thresholds named and reused consistently (P = 12, M = 16 defined once, matched in Requirements and Acceptance Criteria).
- Hard-refusal contract and name-only guarantee stated identically across Context, Requirements, and Acceptance Criteria.
- The actor (the scrub-secrets scanner) is unambiguous throughout, so passive-voice phrasing does not obscure responsibility.

**Findings**:
- 🟡 major, medium confidence — **Context motivates mid-token-ellipsis truncation that Drafting Notes exclude** (Context). Summary/Context promise detection of a token "truncated with an ellipsis in the middle"; Drafting Notes exclude mid-token truncation and the prefix rule catches only the head. Whether such a token is caught depends on 12 characters preceding the ellipsis — stated nowhere. Suggest stating the reconciliation explicitly.
- 🟡 major, high confidence — **16-character threshold stated against "value" vs "form"** (Acceptance Criteria). Requirements bullet 2 and pseudocode gate on form length (`len(f) >= 16`); AC4 and Drafting Notes gate on the value. A short value can produce a 16+ char base64 form, giving different behaviour. Pick one quantity and use it uniformly.
- 🔵 minor, medium confidence — **"whitespace-reflowed" (Summary) vs "whitespace-normalisation" (elsewhere)** (Summary). Terminology drift for a core transformation; use one term or distinguish leakage shape from derived needle.
- 🔵 suggestion, low confidence — **"needle" used without definition on first use** (Context). Gloss the search-string metaphor once so "needle form" / "prefix needle" read unambiguously.

### Completeness

**Summary**: A structurally complete, densely populated story. All expected sections are present and substantively filled — canonical Summary, forces-explaining Context, five specific Requirements, eight testable Acceptance Criteria, and Technical/Drafting Notes recording file locations, thresholds, and trade-off rationale. Frontmatter is complete and valid. The only observation is the absence of an explicit Dependencies section given the work extends scanner code from another work item.

**Strengths**:
- Summary is an unambiguous story statement — user, capability, motivating "so that".
- Context explains the forces (literal-substring matching, prior AUTH_HEADER widening, remaining encoding shapes) rather than restating the Summary.
- Eight scenario-shaped Acceptance Criteria including explicit false-positive and too-short-value cases.
- Requirements name encodings, P/M thresholds, the hard-refusal contract, and name-only reporting.
- Drafting Notes capture deliberate trade-offs, matching a "ready" status with no open decisions.
- Frontmatter complete and internally consistent.

**Findings**:
- 🔵 suggestion, low confidence — **No explicit Dependencies section though the work extends existing scanner code** (Dependencies, absent). Requirements and Technical Notes build on `NamedSecret::needles()`, `scan()`, and the value-half extraction from prior migration work. Parent is marked done and References capture the derivation, so the coupling may be satisfied — hence low confidence. Consider a brief Dependencies note confirming the prerequisite landed. (Identifying the specific coupling is the dependency lens's domain.)

### Dependency

**Summary**: A well-scoped, largely self-contained hardening of an existing credential scanner. Its one material coupling — the entire scanner surface it extends, produced by the 0196 CLI-migration plan — is described only as provenance in Context and References, not as a scheduling blocker, and the referenced plan is still status `ready`. No external systems, cross-team actions, or downstream consumers are implied, so their absence is appropriate.

**Strengths**:
- Preserves the exit-1 hard-refusal contract, introducing no downstream coupling to the invoking skills' exit-code branching — a real dependency held constant.
- Technical Notes name the precise upstream artefacts (`leaked_credentials.rs`, `NamedSecret::needles()`, `scan()`, `design-adapters/src/environment.rs`), so the surface is traceable.
- Env-config-only and in-process, correctly implying no third-party API or cross-team provisioning coupling.

**Findings**:
- 🟡 major, medium confidence — **Upstream scanner is provenance, not a blocking prerequisite** (References). The scanner is created by Phase 2 of plan `2026-08-11-0196-design-cli-migration`, still status `ready` (not done). "Derived from" reads as idea-provenance, hiding the ordering constraint from a scheduler. Add a "Blocked by" Dependencies entry with the migration's current landed state.
- 🔵 minor, medium confidence — **`AUTH_HEADER` value-half extraction consumed as a fixed contract** (Requirements). This work derives needles from the migration's first-colon split; if that shape changes, the auth-header input is silently lost. Note the coupling in Dependencies or Technical Notes.

### Scope

**Summary**: A tightly-bounded, coherent story — every requirement extends a single scanner function toward one security goal: catching a configured browser credential a model transcribed in encoded or truncated form. Summary, Requirements, and Acceptance Criteria describe the same scope with no drift; the work is atomic with standalone value as a follow-up to the 0196 migration. Mild observations: it fuses two independently-deliverable mechanisms under one story, and its single-function footprint sits at the small end of "story".

**Strengths**:
- All five requirements serve one purpose within a single crate/file, no cross-service or cross-team boundaries.
- Summary, Requirements, and Acceptance Criteria are mutually consistent — the four leakage shapes each map to a Requirement and a Criterion.
- Clear standalone value and clean boundaries; in/out scope stated explicitly (mid-token and tail truncation deliberately excluded).

**Findings**:
- 🔵 suggestion, low confidence — **Fuses two independently-deliverable mechanisms** (Requirements). Encoding derivation (Requirement 1) and truncation-prefix matching (Requirements 2-3) could each ship alone; Requirement 3 is the only coupling. Confirm they are one intended indivisible increment (they appear to be).
- 🔵 suggestion, medium confidence — **`kind: story` may fit `task` better** (Frontmatter: kind). The footprint is a targeted enhancement to one function plus tests, reading closer to a task; leave as `story` if team norms treat security-hardening increments as stories.

### Testability

**Summary**: Unusually well-quantified for a security scrubber — fixed P=12/M=16 thresholds, a needle-derivation algorithm, and several Given/When/Then criteria with explicit exit-code and artefact-not-written outcomes. The main weaknesses: three of the four derived transformations (base64 variant, percent-encoding scope, and especially whitespace-normalisation) are never defined precisely enough to construct a definitive input, and the whitespace and false-positive criteria are effectively tautological — satisfiable without exercising the behaviour they guard.

**Strengths**:
- Thresholds concrete and named (P = 12, M = 16), so the prefix/whole-form boundary is a definite pass/fail.
- Several criteria framed as precondition/action/outcome triples a tester can turn directly into cases.
- The hard-refusal criterion pins observable outcomes (exit 1, artefact not written); the report criterion pins the name-only invariant across every form.
- The too-short case is pinned as an explicit assertion rather than left implicit.
- The needle-derivation pseudocode makes the intended derived-form set enumerable.

**Findings**:
- 🟡 major, high confidence — **Whitespace-normalisation criterion is not verifiable as intended (tautological)** (Acceptance Criteria / Requirements). `whitespace_normalise(V)` equals `V` for a value with no interior whitespace, so the derived needle is the literal value and cannot match an artefact where whitespace was inserted by a line wrap. Restate so normalisation applies to the artefact side, with an exact reflow input.
- 🟡 major, medium confidence — **Encoding transformations undefined** (Requirements / Technical Notes). base64 variants and percent-encoding byte classes are unspecified; for an alphanumeric value, `percent_encode(V)` yields `V`, so the criterion can pass without testing percent-encoding. Specify the alphabet/padding and character set, and require inputs the encoding actually transforms.
- 🟡 major, medium confidence — **False-positive criterion is near-tautological and does not pin the P/M boundary** (Acceptance Criteria, false-positive bullet). "Realistic" and "high-entropy-looking" are subjective; a tester can always pick a non-colliding string, passing by construction. Pin the boundary: an 11-char prefix (P−1) is not flagged while a 12-char prefix is, with stated strings.
- 🔵 minor, medium confidence — **"Are detected" does not state the observable outcome per encoding** (Acceptance Criteria, first bullet). The observable (exit 1, artefact not written, variable named) is pinned only in later criteria; a per-encoding test could assert a weaker observable. Make the outcome explicit or cross-reference.
- 🔵 suggestion, medium confidence — **Deliberately-unhandled truncation cases lack a pinning negative test** (Acceptance Criteria / Drafting Notes). The long-value mid/tail-truncation non-behaviour is asserted in prose only; a future widening would fail no criterion. Add a negative criterion mirroring the short-value one.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** COMMENT

Re-ran the four lenses that carried findings (clarity, completeness, dependency, testability). All six majors and three minors from Pass 1 are resolved. One new major surfaced — the percent-encoding truncated-head path has no acceptance criterion, though the base64 path does — which alone keeps the item below the 2-major REVISE threshold. Verdict improves from REVISE to COMMENT: acceptable to implement, with a few low-cost polish opportunities.

### Previously Identified Issues

- 🟡 **Testability**: Whitespace-normalisation tautological — **Resolved**. Whitespace now stripped on the artefact (haystack) side; AC reframed as a reflow scenario.
- 🟡 **Clarity**: 16-char threshold value-vs-form — **Resolved**. AC and Drafting Notes now read on derived form length, matching the pseudocode.
- 🟡 **Clarity**: Context motivates mid-token truncation Drafting Notes exclude — **Resolved**. Context now describes head-truncation; a negative AC pins mid/tail truncation as out of scope.
- 🟡 **Testability**: Encodings undefined — **Resolved**. RFC 4648 base64 with padding and the percent-encoding unreserved set are specified; AC1 requires transforming inputs.
- 🟡 **Testability**: False-positive criterion tautological / P/M boundary unpinned — **Resolved**. New AC pins the 11-char (P−1) vs 12-char (P) boundary at both edges.
- 🟡 **Dependency**: Upstream scanner provenance not blocker — **Resolved**. Dependencies section records the 0196 Phase 2 scanner as landed, plus the `AUTH_HEADER` value-half contract.
- 🔵 **Clarity**: "whitespace-reflowed" vs "normalisation" — **Resolved** (with a residual, below).
- 🔵 **Testability**: "Are detected" outcome unstated — **Resolved**. AC1 now states exit 1, no artefact, name-only.
- 🔵 **Dependency**: `AUTH_HEADER` contract coupling — **Resolved**. Captured as a fixed-contract Dependencies bullet.
- 🔵 **Completeness**: No Dependencies section — **Resolved**. Section added.
- 🔵 (suggestions) truncation negative test, needle glossary — **Resolved**.

### New Issues Introduced

- 🟡 **Testability** (major, medium): Percent-encoding truncated-head has no acceptance criterion — only the base64 path is pinned, so half of "apply the prefix rule to every derived encoding" is unverified.
- 🔵 **Testability** (minor, medium): The P-boundary AC omits the precondition that the form under test is ≥16 characters, so a short-form test setup can appear to contradict it.
- 🔵 **Clarity** (minor, medium): Whitespace operation still named two ways — Requirements heading said "Normalise" while body/Technical Notes said "strip". (Fixed after this pass: standardised on "strip".)
- 🔵 **Clarity/Testability** (suggestions): "form" / "derived form" / "derived encoding" / "raw value" vocabulary shifts subtly; P and M defined only in Technical Notes after first use; "inventory" vs "artefact" used interchangeably in Context; no criterion pins the compound encoded-plus-reflowed shape; false-positive fixture has no defined strength floor.

### Assessment

The work item is ready to implement. After this pass, three fixes were applied: the whitespace terminology defect was corrected (standardised on "strip"), a percent-encoding truncated-head acceptance criterion was added (closing the sole remaining major), and the P-boundary AC gained its M≥16 precondition (closing the companion minor). No major or minor findings remain; only the optional readability suggestions (vocabulary uniformity, threshold placement, "inventory"/"artefact" wording, a compound encoded-plus-reflowed criterion, and a false-positive fixture floor) are left, at the author's discretion.
