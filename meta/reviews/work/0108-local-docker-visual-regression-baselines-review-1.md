---
type: work-item-review
id: "0108-local-docker-visual-regression-baselines-review-1"
title: "Work Item Review: Local Docker-Based Visual Regression Baseline Generation"
date: "2026-06-12T09:52:06+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0108"
work_item_id: "0108"
reviewer: Toby Clemson
verdict: APPROVE
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: ["visual-regression", "testing", "ci", "docker"]
last_updated: "2026-06-12T11:10:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Local Docker-Based Visual Regression Baseline Generation

**Verdict:** REVISE

This is a strong, thoroughly-researched story: every expected section is present
and substantively populated, the user-story framing is consistent, the L sizing
is justified by genuine breadth across config/tasks/CI/docs, and the Codebase
anchors give an implementer exact file/line starting points. The REVISE verdict
is driven not by structural weakness but by four major findings clustered around
two seams — an internal path contradiction the document declares against itself,
and acceptance criteria that assert precise outcomes (byte-identical baselines,
"CI passes") without a self-contained verification procedure. Addressing these
plus the documentation-AC gap would make the item implementation-ready.

### Cross-Cutting Themes

- **Baseline path contradicts itself within the document** (flagged by: clarity,
  completeness) — Summary, Context, and AC3 reference
  `tests/visual-regression/__screenshots__/`, but the Codebase anchors section
  explicitly corrects this to the frontend-package path
  `skills/visualisation/visualise/frontend/tests/visual-regression/` and states
  the earlier references "should be read as this path." A reader acting on the
  normative leading sections uses a directory the work item itself declares wrong;
  AC3's inspection target is unsatisfiable as literally written.

- **Documentation requirement has no acceptance gate** (flagged by: completeness,
  scope, testability) — Requirement 8 mandates contributor-workflow docs (Docker
  prerequisite, when/how to re-baseline, local debugging) but none of the six ACs
  gate it. The technical change can ship "done" with all coded ACs green while the
  contributor-facing half — the value for the secondary Linux/public stakeholder —
  is silently dropped.

- **Local↔CI anti-drift is the design's riskiest property but is untested**
  (flagged by: dependency, testability) — Requirements mandate a single shared
  source of truth for the image tag and consistent channel/locale pinning "so they
  cannot drift," and AC2 depends on byte-parity that this guarantee underpins. No
  AC verifies the no-drift property, so a regression where local and CI diverge
  could pass review.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Baseline path stated in Summary/Context/AC3 contradicts the
  Codebase anchors
  **Location**: Summary / Context vs Codebase Anchors
  The normative sections refer to `tests/visual-regression/__screenshots__/`, but
  the anchors explicitly state the real path is
  `skills/visualisation/visualise/frontend/tests/visual-regression/`. The same
  referent has two values in one document, and AC3 is unsatisfiable as written.

- 🟡 **Dependency**: External Docker image registry (MCR) dependency not captured
  **Location**: Dependencies
  The whole approach depends on pulling `mcr.microsoft.com/playwright:vX.Y.Z` and
  on Microsoft continuing to publish a tag matching the resolved Playwright
  version, yet Dependencies says "Blocked by: none identified." If MCR is
  unreachable, rate-limits, or stops publishing a matching tag, both local
  re-baselining and the entire CI visual leg fail with no recorded fallback.

- 🟡 **Testability**: AC2 "byte-identical" outcome asserted without a verification
  procedure
  **Location**: Acceptance Criteria
  "Byte-identical to CI" is precise, but the criterion never says how a verifier
  obtains the CI-produced bytes to diff against, nor at what point (the contributor
  has not run CI; the committed set is the only comparable artefact). As written it
  can be argued passed or failed depending on which "CI bytes" are chosen.

- 🟡 **Testability**: AC1 depends on a "subsequent CI run" — a non-deterministic,
  externally-triggered check
  **Location**: Acceptance Criteria
  Verifying "a subsequent CI run passes" requires pushing a branch and waiting for
  the full pipeline, and a pass can be confounded by unrelated CI flake. The
  genuinely testable part ("without any baseline push-back") overlaps AC4; the
  "CI passes" part conflates this change's correctness with overall pipeline health.

#### Minor

- 🔵 **Clarity**: "task" refers to two distinct artefacts whose relationship is
  left optional
  **Location**: Requirements / Acceptance Criteria
  Requirement 2 leaves compare mode as "the same task in compare mode, or a
  companion task," then AC1/AC2/AC5 variously say "the single re-baseline task,"
  "the same task," and "the local Docker compare task" — leaving unclear whether
  one task in two modes or two tasks exist, and what AC2's "the same task" denotes.

- 🔵 **Dependency**: Docker-as-prerequisite stated as an assumption, not a
  dependency edge
  **Location**: Assumptions
  Docker is a hard runtime prerequisite for all visual-test work, recorded only as
  a soft assumption. There is no note on whether the CI runners (`test-e2e` on
  `ubuntu-latest`/`macos-latest`) already provide a usable Docker daemon, so a
  missing daemon would surface at implementation time.

- 🔵 **Dependency**: Shared-artefact consumers beyond 0082 not surveyed
  **Location**: Dependencies
  Collapsing the per-platform baseline set + `snapshotPathTemplate` couples every
  other in-flight baseline-capturing item the way 0082 is coupled. Only 0082 is
  enumerated; no statement confirms it is the sole outstanding (non-`done`)
  consumer, so the absence reads as omission rather than deliberate conclusion.

- 🔵 **Testability**: No AC verifies the version/image-tag single-source-of-truth
  anti-drift guarantee
  **Location**: Requirements
  The most failure-prone part of the design (local↔CI drift) has stated intent but
  no acceptance gate; AC2 mentions the conditions parenthetically but does not test
  that local and CI derive tag/locale from the same source.

- 🔵 **Testability**: AC6 "standard (non-visual) test tasks" input set not
  enumerated
  **Location**: Acceptance Criteria
  The set of "standard test tasks" is undefined, so a verifier does not know which
  invocations (e.g. `mise run test`, `test:e2e`) must be exercised to confirm the
  visual specs are skipped; the criterion can be claimed met after checking one
  path while another native task still runs the visual specs.

- 🔵 **Dependency**: Version-pin implies an unstated ordering between local task
  and CI cutover
  **Location**: Requirements
  If CI adopts the Docker image before the shared pin + local task exist (or vice
  versa), AC2's byte-identical contract is briefly violated. This internal ordering
  constraint between the constituent changes is not stated.

#### Suggestions

- 🔵 **Completeness**: Documentation requirement has no corresponding acceptance
  criterion
  **Location**: Requirements
  Requirement 8 (contributor-workflow docs) has no matching done-definition, so it
  can be silently dropped at completion without failing any stated criterion.

- 🔵 **Scope**: Build (new Docker tooling) and teardown (remove push-back workflow,
  collapse `-darwin`) are bundled
  **Location**: Requirements
  The bundling is correct — splitting would create a transient broken state — but
  worth confirming the team is comfortable landing the cutover atomically rather
  than behind a brief co-existence window.

- 🔵 **Scope**: Docs requirement could stand alone but completes the increment's
  value
  **Location**: Requirements
  Leave docs in scope, but make the corresponding AC a closure gate so the
  contributor-facing half of the story is not silently dropped.

- 🔵 **Testability**: Documentation requirement has no verifiable completion check
  **Location**: Requirements
  The three enumerated doc topics (re-baseline procedure, Docker prerequisite,
  local debugging) could be turned into a checkable artefact-existence test.

- 🔵 **Clarity**: "GHA" and "OOM" used without expansion
  **Location**: Context / Technical Notes
  Expand on first use (GitHub Actions (GHA); out-of-memory (OOM)).

- 🔵 **Clarity**: "documented industry-standard replacement" is unsourced
  **Location**: Context
  Either drop the framing or point it at one of the listed References (e.g. the
  Playwright Docker docs) so the claim resolves to a concrete source.

### Strengths

- ✅ Exceptionally complete: every expected section (Summary, Context,
  Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
  Technical Notes, References) is present and substantively populated, with
  complete, correct frontmatter (kind=story, status=draft).
- ✅ Well-formed story statement — identifies the user, the desired capability,
  and the motivation — with a consistently-named actor ("a developer" / "a
  contributor on Linux") recurring identically across Summary and ACs.
- ✅ Rich Context that pre-empts the apparent contradiction (one baseline cannot
  span two hosts, yet a single Linux baseline is the goal) by introducing the
  singular-Docker-environment mechanism.
- ✅ Single coherent goal: all eight requirements trace to "single Docker-generated
  Linux baseline + remove push-back," with no unrelated bundling and no
  cross-section scope drift; the L size is justified by breadth, not padding, and
  the work is genuinely indivisible.
- ✅ The 0082 "blocks" edge is correctly framed as soft coordination with a precise
  sequencing rationale, and the v1.49 Chromium Open Question is consciously triaged
  as "a technical note, not a blocker."
- ✅ Five of six ACs are crisp, observable, and procedurally verifiable (file
  inspection, workflow absence, pass/no-diff), framed as behaviours rather than
  implementation steps; Codebase anchors supply exact paths and line numbers for
  every referenced artefact.

### Recommended Changes

1. **Resolve the baseline-path contradiction** (addresses: clarity path
   contradiction, completeness path inconsistency) — Use the fully-qualified
   `skills/visualisation/visualise/frontend/tests/visual-regression/__screenshots__/`
   everywhere, or define it once as a named shorthand up front and reference that
   shorthand in Summary, Context, and AC3. Remove the "should be read as this path"
   reconciliation note once the path is consistent.

2. **Reframe AC1 and AC2 as self-contained checks** (addresses: AC1 non-determinism,
   AC2 undefined procedure) — For AC2, assert a checksum match (e.g. sha256 of each
   contributor-produced baseline equals the committed baseline's), naming the
   committed set as the reference artefact. For AC1, split into the locally
   verifiable core ("the visual-regression CI job passes against the freshly
   regenerated committed baselines") and fold the push-back assertion into AC4.

3. **Add an acceptance criterion for the documentation requirement** (addresses:
   completeness docs-AC gap, scope docs closure gate, testability docs check) — Gate
   that a named contributor/testing doc exists and covers the three enumerated
   topics: re-baseline procedure, Docker prerequisite, and local debugging.

4. **Add an anti-drift acceptance criterion** (addresses: testability no anti-drift
   AC, dependency version-pin ordering) — e.g. "Given the Playwright version is
   bumped in package.json, when both the local task and CI run, then both resolve to
   the same image tag/locale from the shared source without manual edits to either."

5. **Capture the external and tooling dependencies** (addresses: MCR dependency,
   Docker prerequisite edge, shared-artefact consumers) — Add to Dependencies: the
   MCR registry coupling (availability + tag-publishing assumption); Docker as an
   environment/tooling prerequisite, noting whether CI runners already provide a
   daemon; and a one-line statement confirming 0082 is the only outstanding
   non-`done` baseline-capturing consumer (or listing any others).

6. **Disambiguate the "task" referent** (addresses: clarity task ambiguity) — Commit
   to either one task with two modes or two named tasks (re-baseline / compare), and
   make every AC reference resolve to a single artefact.

7. **Enumerate AC6's task set and expand acronyms** (addresses: AC6 input set,
   GHA/OOM, industry-standard sourcing) — Minor polish: name the specific native
   invocations the skip must hold for, expand GHA/OOM on first use, and source or
   soften the "documented industry-standard" claim.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear and internally consistent: the Summary,
Context, Requirements, and Acceptance Criteria all tell the same story, and the
actor is consistently named. A few clarity issues remain: a path referent that the
Summary/Context state and the Codebase Anchors explicitly contradict, two different
uses of "task" that may refer to the same or different artefacts, and one acronym
used without definition. None are severe, but the path contradiction is worth
resolving so a reader does not act on the wrong directory.

**Strengths**:
- The user-story actor is consistently named throughout — "a developer" / "a
  contributor on Linux" appear in the Summary and recur identically in the ACs.
- The Context explicitly reconciles "one baseline cannot be shared across two host
  environments" with "a single Linux baseline is the goal" via the
  singular-environment (Docker) mechanism.
- ACs use Given/When/Then with concrete, named subjects, each resolving to an
  explicitly introduced referent.

**Findings**:
- 🟡 **major** (confidence: high) — _Summary / Context vs Codebase Anchors_: The
  Summary, Context, and AC3 refer to `tests/visual-regression/__screenshots__/`, but
  the Codebase anchors subsection explicitly contradicts this with
  `skills/visualisation/visualise/frontend/tests/visual-regression/` and states the
  earlier reference "should be read as this path." A reader stopping at the
  normative sections acts on a path the work item declares wrong, and AC3's
  inspection target is unsatisfiable as written. Fix by using the fully-qualified
  path everywhere or defining a named shorthand once.
- 🔵 **minor** (confidence: medium) — _Requirements / Acceptance Criteria_: "task"
  refers to two distinct artefacts whose relationship is optional. Requirement 2
  allows "the same task in compare mode, or a companion task," while AC1/AC2/AC5 say
  "the single re-baseline task," "the same task," and "the local Docker compare
  task," leaving unclear whether one or two tasks exist and what "the same task"
  denotes. Commit to one shape.
- 🔵 **suggestion** (confidence: medium) — _Context / Requirements / Technical
  Notes_: "GHA" and "OOM" are used without expansion though "GitHub Actions" is
  spelled out elsewhere. Expand on first use.
- 🔵 **suggestion** (confidence: low) — _Context_: "the documented industry-standard
  replacement for a CI-push-back workflow" asserts authority without naming where it
  is documented. Drop the framing or point it at a listed Reference.

### Completeness

**Summary**: This story is exceptionally complete: every expected section is present
and substantively populated, and the frontmatter carries all required fields with a
recognised kind. The Summary states the user, want, and why in classic story form;
the Context explains the motivating problem in depth; and the six ACs define done
across the major dimensions. No critical or major completeness gaps — only a couple
of minor observations about reader-facing self-containment.

**Strengths**:
- Summary is a well-formed story statement (user, capability, motivation) plus a
  clear problem paragraph.
- Context is rich — per-platform convention, why Linux lags, the
  rasterizer/antialiasing reason, and why a single Docker environment is the fix.
- Six ACs in Given/When/Then form, far exceeding the two-criterion minimum.
- Kind-appropriate content fully satisfied; Requirements are specific and
  actionable; optional sections all populated; frontmatter complete and correct.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Summary_: Summary/Context refer to
  `tests/visual-regression/__screenshots__/` but the Codebase anchors later correct
  this to the frontend-package path. A reader working top-to-bottom gets the wrong
  path until the anchors block. Correct inline or add a forward pointer.
- 🔵 **suggestion** (confidence: low) — _Requirements_: The documentation
  deliverable has no corresponding acceptance criterion, so it can be silently
  dropped at completion without failing any stated criterion. Add an AC asserting
  the contributor-workflow documentation is added/updated.

### Dependency

**Summary**: The work item captures its most important coupling well: the soft
Blocks edge to 0082 is named with a clear sequencing rationale, and the v1.49
Chromium Open Question is correctly flagged as a technical note rather than a
blocker. The principal gap is external: the work depends on MCR
(`mcr.microsoft.com/playwright`) being reachable and on Docker being installed for
every contributor and CI runner, yet neither appears in Dependencies. A secondary
gap is that 0108 collapses a shared artefact that any other in-flight
baseline-capturing item would be coupled to, and only 0082 is enumerated.

**Strengths**:
- The Blocks edge to 0082 is explicitly captured with a precise soft-coordination
  rationale.
- The v1.49 straddle is surfaced as an Open Question and explicitly classified as
  "not a blocker."
- The macOS CI-matrix-skip coupling (AC6 ↔ main.yml ↔ visual-regression project
  isolation) is traced through Requirements, ACs, and Technical Notes.

**Findings**:
- 🟡 **major** (confidence: high) — _Dependencies_: The approach is built entirely
  on pulling the official MCR image, so every re-baseline, local compare, and CI
  visual job depends on MCR being reachable and Microsoft continuing to publish a
  matching tag, yet Dependencies says "Blocked by: none identified." If MCR is
  unreachable, rate-limits, or drops a tag, both local re-baselining and the CI
  visual leg fail with no recorded fallback. Add an external-dependency note naming
  MCR and the tag-publishing assumption.
- 🟡 **minor** (confidence: medium) — _Assumptions_: Docker is a hard runtime
  prerequisite recorded only as a soft assumption, with no note on whether CI
  runners already provide a Docker daemon. Promote to a Dependencies tooling
  coupling and state CI-runner Docker availability.
- 🔵 **minor** (confidence: medium) — _Dependencies_: Collapsing the shared baseline
  set + `snapshotPathTemplate` couples any other in-flight baseline-capturing item
  the way 0082 is coupled; only 0082 is enumerated. Add a line confirming 0082 is
  the sole outstanding non-`done` consumer, or list others.
- 🔵 **minor** (confidence: low) — _Requirements_: The shared version-pin source +
  AC2 byte-parity imply the pin, local task, and CI change must land together; this
  ordering constraint is unstated. Note it in Dependencies or Technical Notes.

### Scope

**Summary**: Work item 0108 describes a single coherent goal — replace the
CI-push-back baseline workflow with a local Docker-based single-Linux regime — and
every requirement traces back to it. The L sizing is justified by breadth across
config/tasks/CI/docs rather than padding, and the story kind is appropriate for one
team-owned increment. The only mild tension is that the eight requirements form a
tightly coupled bundle that must land atomically, which is correct here rather than
a decomposition opportunity; the 0082 Blocks edge is correctly characterised as
soft coordination.

**Strengths**:
- All eight requirements serve one unified purpose with no "and also" bundling.
- Summary/Context/Requirements/ACs describe the same scope — no drift.
- L size justified by described breadth; the work is genuinely indivisible.
- The 0082 dependency is framed as soft coordination, not a hard block.
- The v1.49 Open Question is explicitly bounded as "a technical note, not a blocker."

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Requirements_: The requirements bundle
  build-out (new Docker tooling) with teardown (remove `update-visual-baselines.yml`,
  collapse `-darwin`). They are tightly coupled — splitting would create a transient
  broken state — so the bundling is correct, but worth confirming the team accepts an
  atomic cutover. Optionally note that the old workflow stays until the new task is
  verified passing in CI.
- 🔵 **suggestion** (confidence: low) — _Requirements_: The documentation requirement
  could stand alone as a follow-up without breaking the technical change, but it
  completes the increment's value for public Linux contributors. Keep it in scope but
  ensure the corresponding AC treats docs as a closure gate.

### Testability

**Summary**: The ACs are unusually strong for a story: five of six are Given/When/Then
with observable, procedurally verifiable outcomes. The main gaps are AC2's
"byte-identical" claim resting on conditions a verifier cannot fully observe in one
place, and AC1's dependence on a "subsequent CI run" that introduces an environmental
variable into what should be a deterministic check. A couple of requirements
(locale/channel pinning, version single-source-of-truth) carry verifiable intent but
have no corresponding acceptance criterion.

**Strengths**:
- AC3 and AC4 are crisply verifiable by direct inspection (single baseline set;
  absence of the named GHA workflow).
- AC5 and AC6 define concrete, observable pass/fail outcomes with clear preconditions.
- Criteria are framed as behaviours, appropriate for a story.
- Codebase anchors supply exact paths/line numbers so a verifier can locate every
  referenced artefact.

**Findings**:
- 🟡 **major** (confidence: high) — _Acceptance Criteria_: AC2's "byte-identical to
  CI" is precise but the criterion never states how a verifier obtains the
  CI-produced bytes to diff against, nor when (the contributor has not run CI; the
  committed set is the only comparable artefact). Reframe as a checksum match against
  the named committed set.
- 🟡 **major** (confidence: medium) — _Acceptance Criteria_: AC1 requires "a
  subsequent CI run passes," which needs a branch push + full pipeline wait and can be
  confounded by unrelated flake. The testable part ("without any baseline push-back")
  overlaps AC4. Split into the locally verifiable core and fold the push-back
  assertion into AC4.
- 🔵 **minor** (confidence: medium) — _Requirements_: The version/image-tag
  single-source-of-truth and channel/locale pinning have stated intent but no AC
  testing the no-drift property. Add an AC checking local and CI resolve the same tag/
  locale from the shared source after a version bump.
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_: AC6's "standard
  (non-visual) test tasks" input set is not enumerated, so a verifier does not know
  which invocations to exercise. Enumerate the specific task invocations.
- 🔵 **minor** (confidence: low) — _Requirements_: The documentation requirement has
  no verifiable completion check though it lists three concrete topics. Add an AC that
  a named doc exists and covers the three topics.

## Re-Review (Pass 2) — 2026-06-12

**Verdict:** REVISE

The pass-1 edits cleanly resolved every original finding: the path contradiction is
gone, the MCR and Docker couplings are captured (now cited as strengths), AC2 is a
byte-exact sha256 check, and the docs + anti-drift criteria exist. **Completeness now
returns zero findings.** However, the re-review surfaces three major findings —
partly *introduced* by the pass-1 edits (the macOS-runner Docker phrasing and the
anti-drift AC's tag/locale conflation) and partly residual tightening on AC1. The
verdict stays REVISE under the 2-major threshold, but the remaining work is narrow
and mostly self-inflicted by the previous round's wording.

### Previously Identified Issues
- 🟡 **Clarity** (path contradiction in Summary/Context/AC3) — **Resolved.** Now uses
  the fully-qualified frontend path throughout; reconciliation note removed.
- 🟡 **Dependency** (MCR registry coupling not captured) — **Resolved.** Now an
  explicit external dependency and cited as a strength.
- 🟡 **Testability** (AC2 byte-identical procedure undefined) — **Resolved.** Now a
  sha256 checksum match against the named committed reference set; cited as a strength.
- 🟡 **Testability** (AC1 depends on a non-deterministic CI run) — **Partially
  resolved.** Reframed to "CI job passes against the regenerated committed baselines,"
  but *which* CI run/commit is the subject is still ambiguous (re-flagged major).
- 🔵 **Clarity** ("task" referent ambiguity) — **Partially resolved.** ACs are now
  consistent ("re-baseline task" / "compare task"), but Requirement 2 still offers two
  shapes without committing, vs AC5's definite "the local Docker compare task"
  (re-flagged minor, low).
- 🔵 **Dependency** (Docker prerequisite as soft assumption) — **Resolved.** Promoted
  to a Dependencies tooling coupling.
- 🔵 **Dependency** (consumers beyond 0082 not surveyed) — **Resolved.** 0082 confirmed
  sole outstanding consumer; 0037/0073/0095 noted as done.
- 🔵 **Dependency** (version-pin land-together ordering) — **Resolved.** Cutover-ordering
  note added to Technical Notes; cited as a strength.
- 🔵 **Testability** (no anti-drift AC) — **Resolved**, but see new AC6 locale issue.
- 🔵 **Testability** (AC6 task set not enumerated) — **Resolved.** Now names
  `mise run test` / `mise run test:e2e`.
- 🔵 **Completeness** (path inconsistency; docs requirement no AC) — **Resolved.** Docs
  AC added; completeness returns zero findings this pass.
- 🔵 **Clarity / Scope / Testability** (docs verifiability, build/teardown, acronyms,
  industry-standard sourcing) — **Resolved/Addressed.**

### New Issues Introduced
- 🔴 **Dependency** (major, high) — _Dependencies_: The pass-1 wording
  "confirming/enabling Docker on the macOS runner is part of this work" overstates the
  coupling. GitHub-hosted macOS runners ship no Docker daemon by default — but per AC6
  the macOS leg *skips* the visual project, so it needs no daemon. The phrasing now
  reads as in-scope work that may not exist. Reframe: the macOS leg skips Docker; the
  daemon is required only on the Linux CI leg and locally.
- 🔴 **Testability** (major, medium) — _AC1_: Reframing left it ambiguous *which* CI
  run is the subject of "passes against the regenerated committed baselines." Pin it,
  e.g. "when the developer commits the regenerated baselines and pushes the branch, the
  visual-regression CI job on that same commit passes with zero pixel diffs."
- 🟡 **Testability** (major, medium) — _AC5_: Defines the outcome (compare passes, no
  diffs) but not the precondition that makes it meaningful — a clean checkout whose
  baselines are the CI-generated reference set. Without it, a developer trivially
  passes against their own freshly-generated files. State the clean-checkout precondition.
- 🔵 **Testability** (minor, medium) — _AC6_: The anti-drift AC conflates the
  version-derived image tag with the fixed locale pin — a `package.json` version bump
  does not change LANG/LC_ALL, so the locale half has no defined trigger. Split locale
  into its own same-source assertion; enumerate every value sharing the source (tag,
  channel, locale).
- 🔵 **Dependency** (minor, high) — _Dependencies_: The Blocks edge to 0082 is
  asymmetric — 0082's own record does not reference 0108, so a scheduler reading from
  0082 won't see the "land 0108 first" recommendation. Add a reciprocal note to 0082.
- 🔵 **Clarity** (minor, medium) — _Dependencies_: "that risk is accepted, not
  mitigated" has an ambiguous antecedent (MCR unreachability vs future tag
  unavailability). Make explicit which risk(s) are accepted.

### Assessment
The work item is materially stronger than pass 1 — all twelve original findings are
resolved or substantially addressed, and completeness is now clean. It is **not yet
APPROVE** only because three majors remain, two of which are wording regressions
introduced by the pass-1 edits (macOS-runner Docker scope, AC6 tag/locale conflation)
and one a residual tightening (AC1's subject CI run). These are small, well-localised
fixes — a focused third pass should clear them and reach APPROVE.

## Re-Review (Pass 3) — 2026-06-12

**Verdict:** COMMENT

The pass-2 edits cleared every pass-2 major: the macOS-runner Docker overstatement
is gone (the visual job now runs as a dedicated Linux-only CI job; macOS leg needs no
daemon), AC1 pins the subject commit, AC5 has its clean-checkout precondition, the
no-drift AC is split into tag/channel and locale criteria, the 0082 edge is reciprocal,
and the "that risk" antecedent is explicit. **Dependency now has zero majors; clarity
and scope raise only minors/suggestions.** The verdict is COMMENT (not APPROVE) on the
strength of a single new major from testability — and it is a genuine design issue, not
a wording nit: the pass-1 sha256 fix may have over-specified.

### Previously Identified Issues (pass 2)
- 🔴 **Dependency** (macOS-runner Docker overstated) — **Resolved.** Reworked into a
  dedicated Linux-only CI job; the daemon coupling is now correctly scoped (Linux CI +
  local macOS only). Cited as a strength this pass.
- 🔴 **Testability** (AC1 subject CI run ambiguous) — **Resolved.** Now pinned to "commit
  the regenerated baselines and push the branch → the job on that same commit passes with
  zero pixel diffs."
- 🟡 **Testability** (AC5 precondition missing) — **Resolved.** Now "a clean checkout of
  `main` whose baselines were generated by CI … run the compare task without re-baselining."
- 🔵 **Testability** (AC6 tag/locale conflation) — **Resolved.** Split into a tag+channel
  version-bump criterion and a separate locale same-source criterion.
- 🔵 **Dependency** (asymmetric 0082 edge) — **Resolved.** 0082 now records the reciprocal
  coordination edge; verified by the dependency reviewer.
- 🔵 **Clarity** ("that risk" antecedent) — **Resolved.** Now "Both risks — MCR
  unreachability and a missing tag — are accepted, not mitigated."

### New Issues Introduced
- 🔴 **Testability** (major, high) — _AC2_: The sha256-checksum-equality criterion (the
  pass-1 fix for the old "byte-identical undefined" major) may not be a definitive
  pass/fail. PNG byte-reproducibility is not guaranteed across host machines, Docker
  storage drivers, and especially **CPU architecture** — Apple Silicon pulls the `arm64`
  image variant by default while CI (`ubuntu-latest`) is `amd64`, and Chromium/Skia
  rendering + PNG encoding can differ at the byte level. A contributor on an M-series Mac
  could see a checksum mismatch unrelated to correctness. This is a real design gap, not
  just an AC-wording problem: the byte-identical promise only holds if the image platform
  is pinned (e.g. `--platform=linux/amd64`). Recommended: add a requirement pinning the
  image platform to match CI, and reframe AC2 around the pixel-diff comparator (what AC5
  already verifies) rather than raw cross-machine sha256.
- 🔵 **Clarity** (minor, high) — _Requirements_: Requirement 2 still offers "the same task
  in compare mode, or a companion task" without committing; state it in outcome terms or
  pick one shape (the compare-task AC already assumes a runnable compare task).
- 🔵 **Testability** (minor) — _AC10 (docs)_: "covers" three topics is a subjective
  coverage judgement; make it enumerable (a named section per topic, a worked debug
  example).
- 🔵 **Testability** (minor) — _AC4_: "no mechanism pushes regenerated baselines back to
  `main`" is a negative-universal; bound it to "no workflow under `.github/workflows/`
  commits/pushes files under the `__screenshots__` directory."
- 🔵 **Dependency** (minor) — CI Linux runner `--ipc=host` support and future
  `@playwright/test` version bumps are coupled actions worth a one-line note.
- 🔵 **Clarity / Scope** (suggestions) — name the "single shared source of truth" artefact
  concretely; docs remain the natural fast-follow seam; story is L-by-design.

### Assessment
The work item is in good shape and **acceptable as-is** — every pass-1 and pass-2 finding
is resolved, and the verdict is COMMENT rather than APPROVE only because one substantive
new concern surfaced: the **cross-architecture byte-identical / sha256 assumption (AC2)**.
That one is worth resolving before implementation because it is a design decision (pin the
image platform to `linux/amd64`) not just an acceptance-criterion tweak — left unaddressed,
Apple Silicon contributors could hit spurious mismatches. The remaining minors are polish
that can be folded in opportunistically. Recommend one more targeted edit on AC2 + a
platform-pin requirement, after which this reaches APPROVE.

## Re-Review (Pass 4 — confirming, testability + dependency) — 2026-06-12

**Verdict:** COMMENT

Confirming pass on the two lenses that touched the cross-architecture concern. The
pass-3 major is resolved: a new requirement pins the Docker image platform to
`linux/amd64` on both local and CI, AC2 is reframed around the zero-pixel-diff
comparator (architecture-independent, consistent with AC5), and a Technical Note
documents the emulation trade-off on Apple Silicon. **Both lenses now return zero
majors** — only minors and suggestions remain, none of which gate implementation.

### Previously Identified Issues (pass 3)
- 🔴 **Testability** (AC2 sha256 architecture-fragile) — **Resolved.** Replaced with a
  comparator-based zero-pixel-diff criterion explicitly covering Apple Silicon, backed
  by a `linux/amd64` platform-pin requirement and Technical Note. No longer flagged.

### New Issues Introduced
- 🔵 **Testability** (minor) — _AC2_: "fonts" is enumerated among the pinned inputs but
  no requirement establishes font pinning — fonts come bundled with the official image,
  not separately configured. Drop "fonts" from AC2's list, or add a requirement that
  fonts derive solely from the pinned image with no host leakage.
- 🔵 **Testability** (minor) — _Docs AC_: verifies topic coverage but not executability;
  optionally strengthen to "a contributor following the documented steps from a clean
  checkout produces a zero-diff compare run" (reuses the existing compare outcome).
- 🔵 **Testability** (minor) — _Technical Notes_: no criterion bounds the
  `maxDiffPixelRatio`/`threshold` tolerance, so a loose value could force green passes
  silently; consider an AC capping it at the current per-spec values (0.05/0.01, no
  global raise).
- 🔵 **Testability** (suggestion) — `--ipc=host` OOM mitigation has no observable check;
  optionally assert the full 23-spec run completes without crash on CI Linux and an
  emulated amd64 host.
- 🔵 **Dependency** (minor) — _Open Questions_: resolve the v1.49 straddle question
  before the sprint; if it applies, the one-time full regeneration becomes an in-work
  ordering step that belongs in the cutover note, not an open conditional.
- 🔵 **Dependency** (minor) — _Dependencies_: 0083 (downstream of 0082) also captures
  visual baselines, so the land-0108-first preference extends transitively; add a brief
  note to the 0082 Blocks entry.

### Assessment
The architecture concern that held the work item at COMMENT is fully resolved, and both
confirming lenses are clear of majors. The work item is **implementation-ready** — the
verdict is COMMENT rather than APPROVE only because a handful of minor polish items
remain (the "fonts" enumeration nit, tolerance-bound AC, docs executability, the v1.49
straddle and 0083 transitive notes), none of which block planning or implementation.
These can be folded in opportunistically or addressed during planning. No further review
pass is required before this work item proceeds.

## Approval (Pass 5) — 2026-06-12

**Verdict:** APPROVE

Two of the pass-4 minors were folded in before approval: AC2 now reads "the pinned image
(which bundles its own fonts)" rather than listing `fonts` as a separately-pinned input,
and a new acceptance criterion bounds the comparison tolerance (`maxDiffPixelRatio` at or
below the current 0.05 / 0.01 per-spec values, no global `threshold` raise) so a loose
tolerance cannot silently mask cross-environment deltas. The work item now carries 11
acceptance criteria.

Across five passes every critical and major finding has been resolved (4 majors → 3 → 1 →
0), and the remaining open items are non-gating polish:
- Docs AC could be strengthened to an executable check (contributor follows docs → zero-diff
  compare run).
- `--ipc=host` run-stability has no observable criterion (suggestion only).
- Resolve the v1.49 Chromium-headless straddle Open Question during planning; if it applies,
  the one-time full regeneration becomes an in-work ordering step.
- Add a transitive-consumer note for 0083 (downstream of 0082) to the 0082 Blocks entry.

Approved by Toby Clemson; the work item is ready for planning. Status transition handled
separately via /update-work-item.
