---
type: work-item-review
id: "0164-launcher-and-git-style-dispatch-review-1"
title: "Work Item Review: Launcher and Git-Style Dispatch"
date: "2026-07-03T12:18:42+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0164"
work_item_id: "0164"
reviewer: Toby Clemson
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-07-03T13:40:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Launcher and Git-Style Dispatch

**Verdict:** REVISE

This is a strong, densely-populated story — every section is present with
substantive content, the requirements are implementation-ready, and it maps
cleanly onto a deliberate launcher/distribution decomposition. It falls short
of APPROVE on two reinforcing gaps: several concrete behaviours stated in the
Requirements and Technical Notes (non-UTF-8 arg forwarding, exit-code/signal
propagation, the env-override escape hatch) have no acceptance criterion
verifying them, and the dependency metadata understates two real upstream
couplings — the 0163 blocker is missing from frontmatter and the 0165
producer/consumer dependency is filed only as `relates_to`.

### Cross-Cutting Themes

- **Stated behaviours and couplings not reflected in the verifiable/structured
  sections** (flagged by: testability, dependency) — The prose describes
  behaviours and dependencies that the acceptance criteria and frontmatter do
  not capture. Testability finds Requirements-level behaviours with no gating
  criterion; dependency finds a prose blocker absent from `blocked_by` and a
  hard dependency filed as a peer relation. In both cases the *narrative* is
  complete but the *machine- and verifier-facing* surface is not.
- **The 0165 coupling is stronger than "relates to"** (flagged by: dependency,
  scope) — Both lenses independently observe that the launcher's verification
  and manifest-driven help cannot be exercised end-to-end without 0165's
  manifest, checksums, and minisign pubkey, yet 0165 is listed only as a peer
  relationship rather than a sequencing constraint.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Fifth criterion bundles two under-specified checks with no verification procedure
  **Location**: Acceptance Criteria
  AC5 ("statically linkable (rustls, no OpenSSL); the bash bootstrap is
  bash-3.2-safe") bundles two distinct claims, neither stating the procedure
  that yields pass/fail — no target triple or `ldd`/`otool` check, and no named
  linter/bash version (the repo's `scripts/lint-bashisms.sh` + 3.2 floor go
  unreferenced).

- 🟡 **Testability**: Requirements assert behaviours (non-UTF-8 args, exit-code/signal propagation) not covered by any criterion
  **Location**: Acceptance Criteria
  The Requirements specify that `Vec<OsString>` preserves non-UTF-8 args and
  that Unix `exec` propagates exit codes and signals, but no acceptance
  criterion asserts either. The Summary's core promise — transparent git-style
  dispatch — could regress silently since the contract that makes it
  transparent is never checked.

- 🟡 **Dependency**: 0163 blocker present in prose but absent from frontmatter `blocked_by`
  **Location**: Frontmatter: blocked_by
  Dependencies says "Blocked by: 0163" but the frontmatter has no `blocked_by`
  field, though it populates `blocks`, `relates_to`, and `parent`. Every sibling
  uses the convention (0163 itself declares `blocked_by: ["work-item:0162"]`),
  so the blocker is invisible to structured tooling and Linear sync.

- 🟡 **Dependency**: 0165 is a hard producer/consumer blocker mischaracterised as "Relates to"
  **Location**: Dependencies
  The Requirements/ACs depend directly on 0165's outputs — verify sha256 +
  minisign against 0165's checksums and pubkey, embed that pubkey, and render
  the manifest-driven listing from 0165's manifest `description` field. The
  fetch→verify→cache→exec pipeline cannot be exercised end-to-end without a
  signed, checksummed asset and manifest, yet 0165 is filed only as a peer.

#### Minor

- 🔵 **Testability**: Env-override escape hatch is a preserved behaviour with no verifying criterion
  **Location**: Technical Notes
  Technical Notes requires preserving the `ACCELERATOR_*_BIN` escape hatch for
  air-gapped/offline first use — a concrete, testable behaviour (set the var,
  invoke, assert no fetch) — but no criterion references it, so the offline path
  could ship broken while the story is marked done.

- 🔵 **Testability**: Verification-failure criterion does not specify which failure signal is observed
  **Location**: Acceptance Criteria
  AC3 specifies the negative outcome (no exec) but leaves "reports the failure"
  open — no exit code, no required message content. The security-critical
  refusal path could pass while emitting a diagnostic too vague to distinguish
  a checksum failure from a signature failure or a network error.

- 🔵 **Dependency**: 0167/0169 named as settling the built-in/external split but not in Dependencies
  **Location**: Open Questions
  Open Questions says the built-in-vs-external boundary "is settled alongside
  0167/0169," but neither appears in Dependencies or frontmatter, leaving the
  shared-decision coordination coupling untracked.

- 🔵 **Dependency**: Bootstrap's first-use fetch implies a runtime dependency on the published launcher asset
  **Location**: Requirements
  The bootstrap "fetches the `accelerator` binary itself on first use" — so its
  core behaviour depends on the launcher being a published, verifiable release
  asset (0165's pipeline). Dependencies captures 0165 for manifest/checksums/keys
  but not that the bootstrap's fetch path also depends on the launcher asset
  being published.

#### Suggestions

- 🔵 **Testability**: Manifest-driven help criterion lacks an observable assertion on listing content
  **Location**: Acceptance Criteria
  AC4 requires rendering "the manifest-driven external-subcommands listing" but
  does not state what a verifier should observe (each entry's `description`, a
  known sub-binary present), so an empty/malformed listing could pass.

- 🔵 **Scope**: Fetch/verify/cache pipeline is a large capability co-scoped with the dispatch mechanism
  **Location**: Requirements
  The story bundles the dispatch skeleton (clap `external_subcommand` + `exec` +
  help) with the full fetch→verify→cache→exec pipeline; the dispatch mechanism
  could be delivered/verified against a local binary first. Likely acceptable —
  the research treats "resolve-once-and-cache launcher" as one coherent story —
  but worth a conscious decision if delivery risk matters.

- 🔵 **Scope**: Story depends on 0165-owned artefacts to be fully verifiable
  **Location**: Requirements
  Verification and manifest-driven help consume 0165 artefacts; 0165 is a
  `relates_to`, not `blocked by`, so the end-to-end verification path may only
  be exercisable against stubs until 0165 lands. (Overlaps the dependency-lens
  major finding on the same coupling.)

- 🔵 **Clarity**: "the CLI" introduced as a third term for the launcher/accelerator binary
  **Location**: Summary
  The Summary's "so the CLI grows by on-demand sub-binaries" introduces "the
  CLI" where the rest of the item says "the launcher"/"the `accelerator`
  binary"; a reader must infer whether it means the launcher, the
  launcher-plus-sub-binaries whole, or something broader.

- 🔵 **Clarity**: Bare cross-reference "resolved Q3" is opaque without the research doc
  **Location**: Requirements
  The cache requirement cites "(resolved Q3)" — the resolution is stated inline
  so no meaning is lost, but "Q3" is an unexplained pointer to the source
  research's open question 3.

- 🔵 **Completeness**: Story does not explicitly name the actor whose need is met
  **Location**: Context
  The item describes the mechanism and ADR motivation thoroughly but never names
  the beneficiary (skill authors invoking `accelerator <sub>` at load time, end
  users getting zero-setup sub-binaries) — the "for whom" is left implicit.

- 🔵 **Completeness**: Draft status with self-flagged refinement caveat before promotion
  **Location**: Drafting Notes
  The item is `status: draft` and warns that "Acceptance criteria, dependencies,
  and kind may need refinement before promoting" — the review reflects a draft
  anticipating enrichment rather than a promotion-ready item.

### Strengths

- ✅ All expected sections are present and substantively populated (Summary,
  Context, Requirements, Acceptance Criteria, Open Questions, Dependencies,
  Assumptions, Technical Notes), with strong frontmatter integrity.
- ✅ The first three acceptance criteria are well-formed Given/When behaviours
  with concrete, observable, pass/fail-testable outcomes (fetch-on-miss,
  cache reuse, verification refusal), pinning the exact checks (sha256 AND
  minisign) and cache location (`${CLAUDE_PLUGIN_ROOT}`).
- ✅ Requirements are concrete and implementation-ready (clap signature, Unix
  `exec`, the pipeline, the bootstrap, help synthesis) — an implementer could
  begin without follow-up questions.
- ✅ Scope is coherent and boundaries are drawn: distribution/signing carved out
  to 0165, the config built-in/external split deferred to 0167/0169, the cache
  location already resolved; the launcher/accelerator-binary equivalence is made
  explicit rather than left implicit.
- ✅ Core couplings are named in prose (0163 scaffold, 0168 consumer, 0136
  parent, 0165 distribution) and external crypto/network dependencies
  (reqwest+rustls, minisign-verify, sha256) are captured.

### Recommended Changes

1. **Add acceptance criteria for the exec-contract behaviours** (addresses:
   "Requirements assert behaviours … not covered by any criterion"; "Env-override
   escape hatch … no verifying criterion") — Add criteria asserting: (a) the
   launcher's exit status equals a signalled/non-zero child's; (b) non-UTF-8
   bytes in a forwarded argument reach the sub-binary unmodified; (c) with
   `ACCELERATOR_<SUB>_BIN` set to a local binary, the launcher execs it and
   performs no fetch.

2. **Split and pin AC5's two checks** (addresses: "Fifth criterion bundles two
   under-specified checks") — Make it two criteria naming the procedure, e.g.
   "the release build for all four targets produces a binary with no dynamic
   OpenSSL dependency (verified via `otool -L`/`ldd`)" and "the bash bootstrap
   passes `scripts/lint-bashisms.sh` and executes under bash 3.2".

3. **Add `blocked_by: ["work-item:0163"]` to the frontmatter** (addresses:
   "0163 blocker … absent from frontmatter `blocked_by`") — Make the prose
   blocker machine-readable and consistent with the repo/sibling convention.

4. **Reclassify the 0165 coupling as a blocker (or scope it explicitly)**
   (addresses: "0165 … mischaracterised as 'Relates to'"; "Story depends on
   0165-owned artefacts") — Either move 0165 to `blocked_by`, or state which of
   its artefacts (manifest schema, checksum format, minisign pubkey, published
   launcher asset) must exist before 0164's verification/help/bootstrap ACs are
   testable, and confirm whether 0164 verifies against fixtures so it can close
   independently.

5. **Sharpen AC3 and AC4 observables** (addresses: "Verification-failure
   criterion does not specify which failure signal"; "Manifest-driven help
   criterion lacks an observable assertion") — For AC3, require a non-zero exit
   and a message naming the failed check (sha256 vs minisign) and affected
   sub-binary. For AC4, require the listing to include each manifest entry's
   name + `description` and a known sub-binary (visualiser).

6. **Track the 0167/0169 coordination and bootstrap-asset dependency**
   (addresses: "0167/0169 named … but not in Dependencies"; "Bootstrap's
   first-use fetch implies a runtime dependency on the published launcher
   asset") — Add 0167/0169 to Dependencies (relates_to or a coordination note),
   and note the bootstrap's fetch path depends on 0165 publishing the launcher
   asset.

7. **(Optional polish)** (addresses: clarity + completeness suggestions) —
   Replace/clarify "the CLI" in the Summary, drop or expand the bare "resolved
   Q3", and add a sentence to Context naming the beneficiary.

## Per-Lens Results

### Clarity

**Summary**: The work item communicates a single coherent intent and the
Summary, Requirements, and Acceptance Criteria describe the same scope without
contradiction. Referents are largely unambiguous ("it"/"the launcher"/"the
child" resolve cleanly). The only friction is a small set of terms that shift or
lean on external documents.

**Strengths**:
- Acceptance Criteria use Given/When form with a named actor ("the launcher")
  and unambiguous pronoun resolution.
- Summary, Requirements, and Acceptance Criteria describe the same scope with no
  internal contradictions.
- The launcher/accelerator-binary equivalence is made explicit rather than left
  implicit.

**Findings**:
- 🔵 suggestion (medium) — Summary: "the CLI" introduced as a third term for the
  launcher/accelerator binary. A reader must infer whether it means the launcher,
  the launcher-plus-sub-binaries whole, or something broader.
- 🔵 suggestion (medium) — Requirements: bare cross-reference "resolved Q3" is
  opaque without the research doc; the resolution is inline but "Q3" carries no
  standalone meaning.

### Completeness

**Summary**: Structurally complete and densely populated — every expected
section present with substantive content, frontmatter carries recognised kind
(story), status (draft), and priority. Requirements are implementation-ready and
five Given/When/Then criteria define done. The only gaps are minor: no explicit
actor, and a draft status with a self-flagged refinement caveat.

**Strengths**:
- All expected sections present and substantively populated (no placeholders).
- Strong frontmatter integrity (kind, status, priority, id, title, parent,
  blocks, derived_from all present and recognised).
- Requirements concrete and implementation-ready; five acceptance criteria (well
  above the two-criterion minimum) covering happy path, cache reuse, verification
  failure, help, and static-linkability.
- Context explains the motivation clearly (ADR-0054, luminosity 0008 split, why
  launch-server.sh does not model git-style dispatch).

**Findings**:
- 🔵 suggestion (medium) — Context: story does not explicitly name the actor
  whose need is met; the beneficiary is only implicit.
- 🔵 suggestion (high) — Drafting Notes: draft status with a self-flagged caveat
  that criteria/dependencies/kind may need refinement before promotion.

### Dependency

**Summary**: Core couplings are captured well in prose (0163 blocker, 0168
consumer, 0165 distribution relationship) but two capture gaps stand out: the
0163 blocker is missing from frontmatter `blocked_by` (a convention every sibling
uses), and the 0165 coupling is filed as "Relates to" when the launcher's
verification and manifest-driven help cannot function without 0165's manifest,
checksums, and minisign pubkey.

**Strengths**:
- Upstream blocker (0163), downstream consumer (0168), and parent epic (0136)
  all explicitly named; 0168/0136 also in frontmatter.
- External crypto/network dependencies (reqwest+rustls, minisign-verify
  embedded pubkey, sha256) named, and the 0165 key-lifecycle boundary flagged.
- The air-gapped/offline escape hatch (ACCELERATOR_*_BIN) is captured.

**Findings**:
- 🟡 major (high) — Frontmatter: blocked_by — 0163 blocker present in prose but
  absent from frontmatter `blocked_by`; invisible to structured tooling and
  Linear sync.
- 🟡 major (high) — Dependencies: 0165 is a hard producer/consumer blocker
  mischaracterised as "Relates to"; the fetch→verify→cache→exec pipeline cannot
  be exercised end-to-end without 0165's signed, checksummed asset and manifest.
- 🔵 minor (medium) — Open Questions: 0167/0169 named as settling the
  built-in/external split but not in Dependencies; coordination coupling
  untracked.
- 🔵 minor (medium) — Requirements: bootstrap's first-use fetch implies a runtime
  dependency on the published launcher asset (0165's pipeline), not captured in
  Dependencies.

### Scope

**Summary**: A well-bounded story describing one coherent unit — the launcher's
dispatch mechanism plus its resolve-once-and-cache pipeline behind a thin bash
bootstrap. Requirements all serve a single unified purpose, and boundaries with
0165, 0167/0169 are explicitly drawn. The one tension is that the
fetch→verify→cache→exec pipeline is a substantial capability leaning on
0165-owned artefacts, but the split follows the research's deliberate
launcher/distribution decomposition.

**Strengths**:
- The launcher/distribution split is deliberate and clean; distribution,
  release, cross-compile, signing carved out to 0165.
- All five requirements serve the single theme of git-style dispatch of
  on-demand sub-binaries.
- Cross-work-item boundaries stated rather than left implicit (minisign key
  lifecycle → 0165, config split → 0167/0169, cache location resolved).
- Acceptance Criteria track Requirements one-to-one with no drift.

**Findings**:
- 🔵 suggestion (medium) — Requirements: fetch/verify/cache pipeline is a large
  capability co-scoped with the dispatch mechanism; the dispatch skeleton could
  be delivered/verified against a local binary first. Likely acceptable as-is.
- 🔵 suggestion (low) — Requirements: story depends on 0165-owned artefacts to
  be fully verifiable; 0165 is a `relates_to` not `blocked by`, so the
  end-to-end verification path may only be exercisable against stubs until 0165
  lands.

### Testability

**Summary**: Unusually strong acceptance criteria for a Rust infrastructure
story — four of five are observable Given/When/Then behaviours with concrete
verifiable outcomes. The main gaps are AC5 bundling two loosely-defined checks
without a verification procedure, and a coverage gap where Requirements describe
behaviours (non-UTF-8 arg preservation, exit-code/signal propagation, env-override
escape hatch) that no criterion asserts.

**Strengths**:
- The first three acceptance criteria are well-formed Given/When/Then behaviours
  with concrete, observable, definitive pass/fail outcomes.
- The verification-pipeline criterion pins the specific checks (sha256 AND
  minisign) and the exact cache location (`${CLAUDE_PLUGIN_ROOT}`).
- The help criterion enumerates three distinct triggers with expected rendering
  for each, making each independently testable.

**Findings**:
- 🟡 major (high) — Acceptance Criteria: fifth criterion bundles two
  under-specified checks ("statically linkable", "bash-3.2-safe") with no
  verification procedure.
- 🟡 major (medium) — Acceptance Criteria: Requirements assert behaviours
  (non-UTF-8 args, exit-code/signal propagation) not covered by any criterion.
- 🔵 minor (medium) — Technical Notes: env-override escape hatch is a preserved
  behaviour with no verifying criterion.
- 🔵 minor (medium) — Acceptance Criteria: verification-failure criterion does
  not specify which failure signal (exit code / message) is observed.
- 🔵 suggestion (low) — Acceptance Criteria: manifest-driven help criterion lacks
  an observable assertion on listing content.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-07-03

**Verdict:** REVISE

All four major findings from Pass 1 are resolved. The re-review surfaces three
*new* major findings, all from the testability lens and all a consequence of the
now-more-detailed acceptance criteria — the sharper criteria invite sharper
scrutiny of their observables. These are refinements, not regressions; none
re-opens a Pass-1 issue. The verdict remains REVISE only because the major-count
threshold (2) is met by the three new testability findings.

### Previously Identified Issues

- 🟡 **Testability**: Fifth criterion bundles two under-specified checks — **Resolved.** Split into two procedure-named criteria (no-dynamic-OpenSSL via `otool -L`/`ldd` across four targets; bootstrap passes `lint-bashisms.sh` under bash 3.2).
- 🟡 **Testability**: Requirements behaviours (non-UTF-8 args, exit-code/signal) not covered — **Resolved.** Added exit-status-equals-child and non-UTF-8-forwarded-verbatim criteria.
- 🟡 **Dependency**: 0163 blocker absent from frontmatter `blocked_by` — **Resolved.** `blocked_by: ["work-item:0163"]` added.
- 🟡 **Dependency**: 0165 mischaracterised as "Relates to" — **Resolved.** Reframed as an explicit producer/consumer coupling naming the consumed artefacts and a fixture-based independent-close strategy; both lenses now cite this as a strength.
- 🔵 **Testability**: Env-override escape hatch unverified — **Resolved.** Added the `ACCELERATOR_<SUB>_BIN` no-fetch criterion.
- 🔵 **Testability**: Verification-failure signal unspecified — **Partially resolved.** AC3 now requires non-zero exit + a message naming the failed check and sub-binary; the *cache post-condition* it added is itself now flagged as under-specified (see new findings).
- 🔵 **Dependency**: 0167/0169 coordination untracked — **Resolved.** Added to Dependencies and frontmatter `relates_to`.
- 🔵 **Dependency**: Bootstrap first-use fetch dependency — **Resolved.** Captured in the 0165 coupling note (published launcher asset).
- 🔵 **Suggestion / Scope**: 0165 fixtures for independent close — **Resolved.** Fixture strategy documented in Dependencies.
- 🔵 **Suggestion / Clarity**: "the CLI" ambiguity, bare "resolved Q3" — **Resolved.** Reworded.
- 🔵 **Suggestion / Completeness**: Actor not named — **Resolved.** Beneficiaries named in Summary (completeness now suggests optionally moving this framing into Context).

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria): Cache-reuse criterion names no observable signal for "without re-fetching" — absence of a fetch is not directly observable; add a proxy (unreachable fixture endpoint or asserted zero request count). *Note: this critiques the pre-existing AC2, not a Pass-1 edit.*
- 🟡 **Testability** (Acceptance Criteria): Verification-failure cache post-condition under-specified — "left without the unverified binary" is ambiguous (never-written vs partial-then-removed) and doesn't state what happens to a pre-existing valid entry. *Critiques the AC3 wording added in Pass 1.*
- 🟡 **Testability** (Acceptance Criteria): Manifest-driven help criterion bundles multiple checks with an ambiguous fixture pass condition — the "visualiser present" clause is unclear against a fixture manifest; split into separate criteria and state the fixture expectation. *Critiques the AC4 wording added in Pass 1.*
- 🔵 **Testability** (Requirements): Re-verify-before-every-exec has no dedicated criterion (tamper a cached binary between invocations → reject on next exec).
- 🔵 **Testability** (Acceptance Criteria): OpenSSL-absence check spans four targets but the per-target verifier environment is implicit (`otool -L` for darwin, `ldd` for linux-musl, in CI).
- 🔵 **Testability** (Open Questions): The built-in/external split point this story establishes has no criterion pinning it (`version`/`config` built-in with no fetch; all else external).
- 🔵 **Scope** (Acceptance Criteria): The four-target release-build criterion drifts toward the sibling distribution story (0165); reframe as a host-target launcher-crate build property or move to 0165.
- 🔵 **Dependency** (Dependencies): Cross-story contract-agreement (manifest/checksum/minisign schemas) described but not tracked as an actionable coupling; fixture-vs-production minisign pubkey handoff to 0165 only partially traced.
- 🔵 **Clarity** (Requirements/Technical Notes): Unexplained shorthand leaning on the research doc — "the four targets" (unenumerated), "subdomains" (undefined), "uv-style" (assumes uv knowledge), and `ACCELERATOR_<SUB>_BIN` vs `ACCELERATOR_*_BIN` notation.

### Assessment

The work item is materially stronger than at Pass 1: every blocking issue is
resolved, dependency capture is now cited as exemplary, and the acceptance
criteria grew from 5 to 9 well-formed behaviours. The remaining REVISE verdict
is driven entirely by fine-grained testability observables on the (now much more
specific) criteria — the kind of diminishing-returns polish that is reasonable
to address in a quick follow-up edit or to accept as good-enough for a `draft`
story heading into planning. My recommendation: address the three new
testability majors (cache-reuse observable, verification-failure cache
post-condition, and splitting the bundled help criterion) since they are quick
and remove genuine ambiguity, then the item is comfortably APPROVE-ready. The
remaining minors/suggestions are optional polish that planning can absorb.

### Post-Re-Review Edits (not a new pass)

After Pass 2, the three new testability majors were addressed directly in the
work item (no further review pass was run):

- **Cache-reuse observable (AC2)** — the reuse criterion now proves no re-fetch
  by running the second invocation with the fixture fetch endpoint unreachable
  (or request count asserted zero).
- **Verification-failure cache post-condition (AC3)** — now pins that no cache
  entry (complete or partial/temp) exists for the failed binary afterward and
  any pre-existing verified entry is left intact.
- **Manifest-driven help (AC4)** — split into two criteria: a fixture-manifest
  listing assertion (every fixture entry shown by name + `description`) and a
  separate child-delegation criterion; the ambiguous "visualiser present" clause
  was removed.

Acceptance Criteria now number 10. Only minor/suggestion-level findings remain,
all optional polish for planning to absorb. The item is treated as an
APPROVE-ready draft heading into planning; the recorded Pass-2 verdict (REVISE)
predates these edits.
