---
type: work-item-review
id: "0165-multi-binary-distribution-and-release-pipeline-review-1"
title: "Work Item Review: Multi-Binary Static Distribution and Release Pipeline with minisign"
date: "2026-07-05T21:20:27+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0165"
work_item_id: "0165"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-07-05T22:55:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Multi-Binary Static Distribution and Release Pipeline with minisign

**Verdict:** REVISE

This is a structurally complete, densely-populated, and unusually well-framed
story: the producer/consumer split is stated up front, most acceptance criteria
define mechanically-checkable outcomes (readelf assertions, Mach-O magic,
exact-equality version coherence, end-to-end fetch+verify), and the scope is one
genuinely indivisible increment. The REVISE verdict is driven not by structural
gaps but by two clusters of concrete issues: an ambiguous **checksums.json /
0168 ownership boundary** that recurs across four lenses, and **uncaptured
operational prerequisites** for signing (the secret key and its GHA secret slot)
combined with **two Requirements that have no verifying acceptance criterion**.

### Cross-Cutting Themes

- **checksums.json retirement vs 0168 ownership** (flagged by: clarity, scope,
  dependency, testability) — Requirements say retiring the visualiser's
  standalone `bin/checksums.json` "couples to 0168, **which removes**" it, while
  Dependencies says of 0168 that its checksums.json "is retired **here**". The
  ownership reads as sitting in both stories at once; there is no directed
  ordering; and no acceptance criterion confirms the removal. This single
  ambiguity produces a clarity contradiction, a scope-boundary blur, a
  dependency sequencing gap, and a testability coverage hole.

- **Signing depends on operational prerequisites that are stated only as
  actions, not as blockers** (flagged by: dependency) — the pipeline can only
  sign if the secret half of the committed public key exists *and* a repo admin
  has provisioned it as a GHA encrypted secret. Neither is captured in
  Dependencies, and the "generate a new `-W` key" instruction sits in unresolved
  tension with the "must be the counterpart of the committed public key"
  assumption.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity / Scope**: checksums.json ownership contradicts between Requirements and Dependencies
  **Location**: Requirements / Dependencies
  Requirements attribute removal of the visualiser's standalone
  `bin/checksums.json` to 0168 ("which removes…"), but Dependencies says it "is
  retired **here**" (0165). An implementer cannot tell whether that removal is
  in-scope for 0165 or deferred to 0168, risking duplicated work or a gap where
  neither story owns it.

- 🟡 **Dependency**: minisign secret key matching the committed public key is an uncaptured prerequisite
  **Location**: Requirements / Assumptions
  Assumptions state the launcher "rejects every release" unless the signing key
  is the exact counterpart of the committed `keys/accelerator-release.pub`, yet
  Requirements say to "generate a passwordless (`-W`) key" — a freshly generated
  key would not match. Whether the matching secret half already exists and is
  retrievable is a hard prerequisite recorded nowhere as a blocker.

- 🟡 **Dependency**: GitHub Actions secret provisioning is an uncaptured cross-actor action
  **Location**: Requirements
  The signing step depends on the key being "stored as a GitHub Actions
  encrypted secret," which requires a repo admin to provision it before CI can
  sign. This operational coupling — which gates the whole pipeline and the
  end-to-end AC — is not recorded in Dependencies.

- 🟡 **Testability**: no criterion verifies checksums.json is actually retired
  **Location**: Acceptance Criteria
  Requirements mandate retiring the flat `checksums.json` "entirely," but no AC
  confirms the removal. A build that emits `manifest.json` while *still*
  producing `checksums.json` would pass every listed criterion, letting a stated
  deliverable be silently skipped.

- 🟡 **Testability**: draft-preserve-on-verification-failure behaviour is unverified
  **Location**: Acceptance Criteria
  Requirements require the `gh` upload flow to preserve the draft release on
  verification failure, but every AC describes only the happy path. The highest-
  risk safety behaviour — not publishing a broken/unverifiable release — has no
  criterion forcing a verifier to confirm it.

#### Minor

- 🔵 **Dependency**: 0168 coupling is an ordering constraint recorded only as an undirected "relates to"
  **Location**: Dependencies
  0165 and 0168 share a mutable artifact (the visualiser's checksums.json) and
  imply an ordering constraint, but this is captured only as undirected "Relates
  to: 0168" with no indication of which lands first or whether the removal must
  be a single coordinated change.

- 🔵 **Testability**: AC2 verifies description presence but not its required source
  **Location**: Acceptance Criteria
  Requirements specify each binary's `description` is sourced from that crate's
  `Cargo.toml` `package.description`, but AC2 only checks a description is
  present — a hardcoded or placeholder string would pass, so the source-of-truth
  constraint is untested.

#### Suggestions

- 🔵 **Clarity**: undefined terms SLSA and fnox
  **Location**: Requirements / Drafting Notes
  "SLSA" and "fnox" are used without expansion or a link, so a reader cannot
  confirm what provenance guarantee or key-storage tool is meant.

- 🔵 **Clarity**: dangling "Q6" reference
  **Location**: Requirements
  "(resolved Q6)" points at a numbered question that is defined nowhere in the
  work item, so the rationale for dropping the runtime provenance hook cannot be
  verified from the document.

- 🔵 **Testability**: AC6 "is documented" lacks a named artifact
  **Location**: Acceptance Criteria
  The key-lifecycle documentation criterion names no target artifact or required
  contents, so "documented" could be argued as met by a single sentence.

- 🔵 **Dependency**: potential third-party `minisign-action` dependency not surfaced
  **Location**: Technical Notes
  If CI signing adopts `thomasdesr/minisign-action` (raised only in notes), that
  is a supply-chain coupling in the release path that would be invisible to
  anyone scanning Dependencies.

- 🔵 **Scope**: provenance-hook / RELEASING.md cleanup is thematically adjacent
  **Location**: Requirements
  The RELEASING.md/provenance-hook cleanup is the one requirement whose removal
  would not break the launcher contract; fine to keep if small, but a candidate
  to split out if it grows.

- 🔵 **Scope**: minisign key lifecycle is an operational deliverable on a code story
  **Location**: Requirements
  The key generation/storage/rotation procedure is a distinct operational
  deliverable; no split needed given the tight coupling, but track it separately
  if it needs its own access-control sign-off (e.g. the future fnox-in-repo
  path).

### Strengths

- ✅ Structurally complete: every expected section (Summary, Context,
  Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
  Technical Notes, References) is present and substantively populated;
  frontmatter carries all required fields with a recognised kind (story) and
  appropriate draft status.
- ✅ The producer/consumer framing is established up front and the two core
  subjects ("the launcher" and "the pipeline") stay stable and unambiguous
  throughout.
- ✅ Acceptance criteria express outcomes as concrete, observable system states
  (no PT_INTERP/DT_NEEDED entries, `manifest.version` equals release version,
  build fails on coherence mismatch) rather than vague desired properties; AC1
  even rules out the wrong check (do not assert ELF type EXEC).
- ✅ Genuinely indivisible scope: the launcher only accepts a release once every
  binary and the manifest are signed and versions cohere, so this is one atomic
  increment, and it is deliberately fenced off from the frozen manifest schema
  and launcher-side verify.
- ✅ Opaque flags and choices are defined inline where introduced (passwordless
  `-W`, whole-file vs `-H` prehashed signing, why `ldd` is unsuitable), and
  prior enrichment decisions are captured in Drafting Notes so an implementer
  inherits the resolved context.

### Recommended Changes

1. **Disambiguate checksums.json ownership between 0165 and 0168** (addresses:
   the clarity/scope major, the dependency minor, and the testability major on
   retirement). State explicitly which artifact each story owns — e.g. 0165
   retires the release pipeline's flat `checksums.json`; 0168 owns removal of
   the visualiser's standalone `bin/checksums.json` — align the Requirements and
   Dependencies wording, and record the relationship as a directed
   Blocks/Blocked-by (or "must land together") rather than an undirected
   "relates to."

2. **Add an acceptance criterion for checksums.json retirement** (addresses:
   testability major). E.g. "A pipeline-produced release contains no
   `checksums.json` asset; `manifest.json` is the only integrity artifact
   published."

3. **Add an acceptance criterion for the draft-preserve-on-failure path**
   (addresses: testability major). E.g. "Given re-verification of an uploaded
   asset fails, when the release step runs, the release remains in draft and no
   assets are published."

4. **Capture the signing prerequisites in Dependencies** (addresses: both
   dependency majors). Record that (a) the secret counterpart of
   `keys/accelerator-release.pub` must be available — and reconcile that with the
   "generate a new `-W` key" instruction — and (b) the release flow is gated on a
   repo admin provisioning the signing key as a GHA encrypted secret.

5. **Tighten the softer acceptance criteria** (addresses: testability minor +
   suggestion). Make AC2 assert each `description` *equals* its crate's
   `Cargo.toml` `package.description`, and name the artifact/required contents
   for the AC6 key-lifecycle documentation (e.g. a RELEASING.md section covering
   `-W` generation, GHA-secret storage, compromise-only rotation).

6. **Resolve the small clarity nits** (addresses: clarity suggestions). Expand
   SLSA and fnox on first use, and inline or link the "Q6" resolution.

## Per-Lens Results

### Clarity

**Summary**: A dense but generally well-disciplined work item — the two lead
subjects (launcher/consumer, pipeline/producer) are named early and used
consistently, pronouns resolve cleanly, and outcomes are stated as observable
states. Main weaknesses: a genuine internal inconsistency about which work item
retires the visualiser's standalone checksums.json, plus undefined terms and a
dangling Q6 cross-reference.

**Strengths**:
- Producer/consumer framing established up front; core subjects stay stable and
  unambiguous throughout.
- Outcomes expressed as concrete observable system states rather than vague
  desired properties.
- Opaque flags/choices defined inline where introduced (`-W`, whole-file vs
  `-H`, why `ldd` is unsuitable).

**Findings**:
- 🟡 major (confidence: medium) — Dependencies — Requirements assign removal of
  the visualiser's `bin/checksums.json` to 0168 ("which removes…"), but
  Dependencies says of 0168 that its checksums.json "is retired here," directly
  contradicting the assignment. Impact: implementer cannot tell whether the
  removal is in-scope for 0165 or deferred. Suggestion: make the referent of
  "here" explicit and align the two sections.
- 🔵 suggestion (confidence: medium) — Requirements — "SLSA" and "fnox" are used
  without expansion or link. Impact: a developer cannot confirm the provenance
  guarantee or key-storage tool. Suggestion: expand/link both on first use.
- 🔵 suggestion (confidence: medium) — Requirements — "(resolved Q6)" references
  a question defined nowhere in the item. Impact: rationale for dropping the hook
  is unverifiable. Suggestion: inline the Q6 substance or link the enrichment doc.

### Completeness

**Summary**: Structurally complete and densely populated. Every expected section
is present and substantively filled, frontmatter carries all required fields with
a recognised kind (story) and draft status, and content matches what a story
demands. No material completeness gaps found.

**Strengths**:
- All expected story sections present and substantively populated; Context
  explains motivation rather than restating the summary.
- Six specific, self-contained acceptance criteria covering builds, manifest
  conformance, end-to-end verification, version coherence, RELEASING.md, and key
  lifecycle.
- Frontmatter complete and internally consistent (kind, status, priority,
  parent, relates_to, external_id).
- Dependencies, Assumptions, and Open Questions all populated with applicable
  content; beneficiary system explicitly identified.
- Technical/Drafting Notes capture prior enrichment decisions so an implementer
  inherits resolved context.

**Findings**: _None._

### Dependency

**Summary**: Captures its primary structural couplings well (0163 blocker, 0164
frozen contract, 0168 checksums retirement). The gaps are operational: the
signing secret key and the GHA secret slot must exist and be provisioned before
CI can sign, yet neither is a captured prerequisite; and the 0168 relationship is
a real ordering constraint recorded only as an undirected "relates to."

**Strengths**:
- Upstream build prerequisite captured correctly ("Blocked by: 0163").
- 0164 coupling well-explained and directionally unambiguous across Context,
  Requirements, and Dependencies.
- 0168 checksums.json coupling surfaced in both Requirements and Dependencies.

**Findings**:
- 🟡 major (confidence: medium) — Requirements/Assumptions — the secret
  counterpart of the committed public key is an uncaptured prerequisite, and
  "generate a new `-W` key" conflicts with "must be the counterpart of the
  committed public key." Suggestion: add a Dependencies entry for the secret
  key's availability and reconcile the two statements.
- 🟡 major (confidence: medium) — Requirements — GHA encrypted-secret
  provisioning requires a privileged actor before the signing step can run; not
  recorded in Dependencies. Suggestion: add a Dependencies note that the release
  flow is gated on an admin provisioning the secret.
- 🔵 minor (confidence: medium) — Dependencies — 0168 ordering/coordination
  constraint recorded only as undirected "relates to." Suggestion: make it a
  directed Blocks/Blocked-by or "must land together."
- 🔵 suggestion (confidence: low) — Technical Notes — a potential third-party
  `thomasdesr/minisign-action` dependency is named only in notes, not
  Dependencies. Suggestion: once chosen, record any third-party action/binary
  (pinned) in Dependencies.

### Scope

**Summary**: A coherent, well-bounded story — every requirement converges on a
single deliverable (a pipeline producing launcher-consumable, minisign-signed,
manifest-described artifacts). On the larger end for a story but genuinely
indivisible, since the launcher rejects any non-conforming release. Main
observations are minor: a documentation cleanup that rides along and a slightly
blurred checksums-retirement boundary with 0168.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe the same scope with a
  clear in/out boundary (consumer contract frozen in 0164, explicitly out of
  scope).
- Broad but genuinely indivisible: one atomic increment rather than a bundle of
  separable concerns.
- Deliberately fenced off from the frozen manifest schema and launcher-side
  verify; parent/sibling relationships made explicit.

**Findings**:
- 🔵 minor (confidence: medium) — Requirements — checksums.json retirement
  ownership reads as sitting in both 0165 and 0168 at once. Suggestion: state
  which file each work item owns.
- 🔵 suggestion (confidence: medium) — Requirements — the provenance-hook /
  RELEASING.md cleanup is separable from the core deliverable. Suggestion: keep
  if small, split if it grows.
- 🔵 suggestion (confidence: low) — Requirements — the minisign key lifecycle is
  an operational deliverable coupled onto a code story. Suggestion: no split
  needed given tight coupling; track separately if it needs its own operational
  sign-off.

### Testability

**Summary**: Unusually strong acceptance criteria for a distribution/pipeline
story — most define concrete, mechanically-checkable outcomes and AC2/AC3 are
well-framed input-output pairs. Main weaknesses are coverage gaps: two
Requirements (retiring checksums.json; preserving the draft on verification
failure) have no corresponding AC, and one AC verifies a value's presence but not
its required source.

**Strengths**:
- AC1 gives a precise, tool-based pass/fail procedure and rules out the wrong
  check (do not assert ELF type EXEC).
- AC2/AC3 framed as observable end-to-end behaviours against a real consumer.
- AC4 states an exact-equality contract testable by introducing a deliberate
  mismatch.
- AC5 directly checkable against RELEASING.md content and CI output.

**Findings**:
- 🟡 major (confidence: high) — Acceptance Criteria — no criterion verifies
  `checksums.json` is retired; a build emitting both would pass every AC.
  Suggestion: add "release contains no `checksums.json` asset."
- 🟡 major (confidence: high) — Acceptance Criteria — the
  draft-preserve-on-verification-failure path is unverified; all ACs are happy
  path. Suggestion: add a failure-path criterion.
- 🔵 minor (confidence: medium) — Acceptance Criteria — AC2 checks description
  presence but not its required `Cargo.toml` source. Suggestion: require equality
  with `package.description`.
- 🔵 suggestion (confidence: medium) — Acceptance Criteria — AC6 "is documented"
  names no artifact/contents. Suggestion: name RELEASING.md and required section
  coverage.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-07-05

**Verdict:** COMMENT

Re-ran the four lenses that had findings (clarity, dependency, scope,
testability); completeness had none and was skipped. **All five original major
findings are resolved** — none recurred, and several are now cited by the
re-review as strengths. No major or critical findings this pass; the remaining
items are minor/suggestion polish, mostly next-layer nits the reviewers reached
now that the structural issues are gone. Verdict moves REVISE → COMMENT: the work
item is acceptable for implementation as-is.

### Previously Identified Issues

- 🟡 **Clarity/Scope**: checksums.json ownership contradiction — **Resolved**.
  Both lenses now cite the explicit per-file ownership boundary (0165 owns the
  release pipeline's flat `checksums.json`; 0168 owns the visualiser's
  `bin/checksums.json`) as a strength.
- 🟡 **Dependency**: minisign secret key uncaptured prerequisite — **Resolved**.
  The placeholder-key replacement and HEAD-rebuild ordering are now captured; the
  generate-vs-reuse tension is reconciled in Assumptions and Requirements.
- 🟡 **Dependency**: GHA secret provisioning uncaptured — **Resolved**. The
  dependency lens now lists the repo-admin provisioning action as an explicitly
  captured cross-actor coupling.
- 🟡 **Testability**: checksums.json retirement untested — **Resolved**. New AC
  ("release contains no `checksums.json` asset") is cited as an observable
  outcome.
- 🟡 **Testability**: draft-preserve-on-failure unverified — **Resolved**. New
  AC present; only a minor remains about how the failure is induced (see below).
- 🔵 **Dependency**: 0168 recorded as undirected "relates to" — **Partially
  resolved**. Now a directed "coordinate with" on file ownership, but the lens
  re-raises it in a new form: the pipeline-existence ordering (0165 enables 0168)
  is still not a Blocks entry.
- 🔵 **Testability**: AC2 description source untested — **Resolved** (now asserts
  equality with crate `package.description`).
- 🔵 **Clarity**: SLSA/fnox undefined, dangling "Q6" — **Resolved** (defined
  inline; Q6 removed).
- 🔵 **Testability**: AC6 documentation artifact unnamed — **Resolved** (now
  `RELEASING.md`; a low suggestion remains to frame it as a per-element
  checklist).

### New Issues Introduced

None are regressions; these are next-layer polish surfaced now the majors are
gone:

- 🔵 **Clarity** (minor): "every workspace binary" is ambiguous about whether the
  visualiser is in scope for 0165 or deferred to 0168.
- 🔵 **Clarity** (suggestion): the "(ADR-0046)" citation in the Summary attaches
  to version coherence, but the ADR governs the hand-rolled-pipeline choice.
- 🔵 **Dependency** (minor): 0168 ordering should be an explicit Blocks/enables
  entry — 0168 presupposes this pipeline exists.
- 🔵 **Dependency** (suggestion): if a specific sub-binary must be present for the
  multi-binary AC, name its producing story; else note the pipeline is generic
  over present crates.
- 🔵 **Testability** (minor): AC1 "with a sha256" has no defined location and
  reads in tension with the "manifest is the only integrity artifact" AC — state
  the sha256 lives inside `manifest.json`.
- 🔵 **Testability** (minor): the draft-preserve AC names no failure-injection
  trigger, so it could be marked passed by happy-path inspection.
- 🔵 **Testability** (suggestion): frame the key-lifecycle doc AC as a per-element
  presence checklist.
- 🔵 **Scope** (suggestions): the RELEASING.md/provenance cleanup isn't reflected
  in the Summary; the story is large but the indivisibility rationale holds
  (both defensible, unchanged from pass 1).

### Assessment

The work item is ready for implementation. Every blocking issue from pass 1 is
resolved and the verdict is now COMMENT. The remaining minor/suggestion items are
optional refinements — the most worthwhile are quick one-liners: clarify whether
the visualiser is in 0165's "every workspace binary" scope, state that per-binary
sha256 lives in `manifest.json` (removing the AC1↔AC3 tension), and add a
failure-injection trigger to the draft-preserve AC. None block starting work.

## Approval — 2026-07-05

**Verdict:** APPROVE

The three worthwhile polish items from pass 2 were applied to the work item
(visualiser scope clarified in Requirement 1, per-binary sha256 located in
`manifest.json` in AC1, failure-injection trigger added to the draft-preserve
AC). Reviewer approved the work item for implementation; remaining
suggestion-level items are optional and non-blocking.
