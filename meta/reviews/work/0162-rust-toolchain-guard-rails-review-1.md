---
type: work-item-review
id: "0162-rust-toolchain-guard-rails-review-1"
title: "Work Item Review: Rust Toolchain Guard Rails in mise + CI"
date: "2026-06-28T23:41:15+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0162"
work_item_id: "0162"
reviewer: Toby Clemson
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [rust, tooling, ci, guard-rails]
last_updated: "2026-06-28T23:54:50+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Rust Toolchain Guard Rails in mise + CI

**Verdict:** REVISE

This is a strong, well-structured story — every section is present and densely
populated, the Summary/Requirements/Acceptance Criteria stay tightly aligned
around a single coherent "guard rails" scope, and domain jargon is consistently
anchored to linked ADRs. The blocking concern is testability: the
architecture- and supply-chain-enforcement criteria (cargo-deny bans, cargo-pup
module-import rules) assert that *violations are caught* without specifying any
concrete triggering input, and the coverage half of the test requirement has no
verifying criterion — so a verifier could mark the item "done" with the guard
rails configured but inert. A cluster of underspecified-verification gaps
(paths.py revisit, the grep tripwire) and a few vague referents reinforce the
same theme.

### Cross-Cutting Themes

- **Requirements that no acceptance criterion verifies** (flagged by:
  testability, scope) — the `tasks/shared/paths.py` multi-crate revisit and the
  grep-based dependency tripwire both appear in Requirements/Assumptions but are
  pinned by no criterion; the coverage requirement is the same shape. Verifier
  has nothing concrete to check.
- **The unratified quality-tool stack rests on an unnamed authority** (flagged
  by: clarity, dependency) — "taken from the established direction; no dedicated
  ADR ratifies it" leaves both *what* the established direction is and *which
  decision artefact* backs the stack unstated, which matters precisely because
  the sentence flags the absence of an ADR.
- **The pinned-nightly cargo-pup lane is a distinct concern** (flagged by:
  scope, dependency) — standing up a second toolchain lane carries a different
  risk profile from the stable-lane gates, and its nightly-toolchain
  compatibility is an external coupling recorded only as an open question.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Negative-path criteria (deny bans, pup violation) specify
  no triggering input
  **Location**: Acceptance Criteria
  AC3, AC4, and AC5 all assert that a *violation* is caught, but none specifies
  the concrete input that constitutes a violation (e.g. a crate adding
  `openssl-sys`, a domain module importing an adapter). Without a defined
  triggering case, a verifier cannot demonstrate the guard rail bites versus
  being configured-but-inert — especially given the Technical Note that
  cross-crate bans are "largely inert until the workspace splits."

- 🟡 **Testability**: Coverage is required but no criterion verifies it is
  computed or enforced
  **Location**: Requirements
  Requirements call for coverage folded into the test run and AC2 lists
  "tests-with-coverage", but no criterion states what a verifier checks —
  whether a report is produced, whether a threshold gates the build, or whether
  coverage is informational only. As written it is unverifiable.

#### Minor

- 🔵 **Clarity**: "the Rust component" (singular) conflicts with the per-crate
  "components" described elsewhere
  **Location**: Acceptance Criteria
  AC2 says the default "exits 0 end-to-end with the Rust component included"
  (singular), but Requirements introduce per-crate `<crate>:check` *components*
  (plural) and AC1 refers to "every workspace crate." The referent of "the Rust
  component" is ambiguous.

- 🔵 **Dependency**: This Phase 0 gate blocks every downstream migration phase
  but lists no Blocks entries
  **Location**: Dependencies
  The source research positions this as Phase 0 enforcement wired before any
  other migration phase, yet Dependencies records no "Blocks" entries, leaving
  the downstream gating relationship implicit.

- 🔵 **Dependency**: Pinned-nightly toolchain is an external compatibility
  coupling not captured in Dependencies
  **Location**: Requirements
  cargo-pup's lint engine is tied to a specific nightly compiler ABI, so the
  nightly choice is an external-tooling availability/compatibility dependency,
  not merely an implementation detail — but it is recorded only as an open
  question with no fallback.

- 🔵 **Dependency**: Quality-tool stack adopted without a ratifying ADR is an
  undated decision coupling
  **Location**: Assumptions
  Part of the tooling choice rests on an undocumented decision rather than a
  captured upstream decision artefact, which can resurface as a re-litigation
  blocker.

- 🔵 **Completeness**: Context explains the extension but not the motivating
  problem
  **Location**: Context
  The Context establishes *what* the story is (an extension, mirrors luminosity
  0006) but not the underlying motivation — that without these guard rails the
  new workspace has no mechanical enforcement of the ADR-0053/0046 constraints.

- 🔵 **Testability**: "every workspace crate" is unbounded and currently
  near-tautological
  **Location**: Acceptance Criteria
  AC1 requires per-crate checks "for every workspace crate", but the workspace
  is single-crate until 0166 and the crate set is not enumerated, so the
  criterion has no checkable source of truth.

- 🔵 **Testability**: The paths.py multi-crate revisit has no verifying
  criterion
  **Location**: Requirements
  The "revisit `tasks/shared/paths.py`" requirement is verb-only with no
  measurable result, so a verifier cannot determine whether it was done
  correctly or merely looked at.

- 🔵 **Testability**: The grep-based dependency tripwire is introduced but has
  no acceptance criterion
  **Location**: Assumptions
  A net-new check the item commits to adding appears in no Requirement or
  criterion, so there is no procedure to verify it exists or fires.

#### Suggestions

- 🔵 **Clarity**: "per the source" / "the source research" rely on an implicit
  referent
  **Location**: Open Questions
  The deferral source is never named inline; the reader must infer it points to
  the `Source:` References entry.

- 🔵 **Clarity**: "the established direction" is an undefined referent for the
  tool-stack choice
  **Location**: Assumptions
  Unclear whether this means the source research, the 0158 spike, the mirrored
  luminosity item, or team convention.

- 🔵 **Clarity**: Same concept named three ways — "light/domain crates",
  "infra-out-of-domain rule", "cross-crate ban-lists"
  **Location**: Requirements
  Shifting vocabulary forces the reader to confirm these describe one rule;
  "light" vs "domain" crates are used interchangeably without saying whether
  they are the same set.

- 🔵 **Completeness**: No explicit beneficiary identified for the story
  **Location**: Summary
  The beneficiary (contributors and CI protected from merging violating Rust) is
  implicit rather than named.

- 🔵 **Scope**: Pinned-nightly cargo-pup lane is a distinct concern from the
  stable-lane quality tooling
  **Location**: Requirements
  Standing up a second toolchain lane is a self-contained sub-effort with a
  different risk profile; confirm it is intended to land atomically with the
  stable gates.

- 🔵 **Scope**: paths.py revisit is a tail-end concern bundled with new-tooling
  setup
  **Location**: Requirements
  A pre-existing-code Python adjustment riding along under an otherwise
  Rust-gate-focused story; no split needed, but consider an explicit criterion.

### Strengths

- ✅ All expected story sections are present and substantively populated — no
  empty placeholders — and the five Acceptance Criteria are well above the
  two-criterion floor.
- ✅ Summary, Requirements, and Acceptance Criteria describe one coherent scope
  with no contradictory requirements; the check-vs-test split (coverage folds
  into test, not check; check stays read-only) is stated precisely and
  consistently.
- ✅ Architecture-enforcement jargon is consistently tied to specific linked
  ADRs (0053, 0046), so each term resolves to a definition; actors and triggers
  are explicit about which tool performs each check.
- ✅ Requirements are concrete and actionable — they name the specific tools,
  the files to create (rustfmt.toml, clippy.toml, deny.toml), and the paths.py
  revisit — so an implementer could start without follow-up.
- ✅ The upstream pairing with 0163 is explicitly named with rationale, the
  downstream ban-list activation point (0166) is captured, and the cross-
  component touch on `tasks/shared/paths.py` is flagged.
- ✅ Story sizing is appropriate for the declared kind; Open Questions correctly
  externalise the genuinely undecided inputs (restriction lints, nightly pin) as
  implementation-time decisions rather than ambiguous acceptance gates.

### Recommended Changes

1. **Add concrete triggering fixtures to the negative-path criteria** (addresses:
   "Negative-path criteria specify no triggering input", "grep tripwire has no
   AC"). For each enforcement criterion, state the input that constitutes a
   violation and the expected failure — e.g. "introducing a dependency on a
   native-tls crate makes `cargo deny check` exit non-zero", "a domain module
   importing an adapter module makes the cargo-pup lane fail", and an equivalent
   for the grep tripwire. A deliberately-failing case proves each guard is live,
   not merely configured.

2. **Pin the coverage outcome in an acceptance criterion** (addresses: "Coverage
   is required but no criterion verifies it"). Either fix the observable outcome
   ("the test run emits an llvm-cov report at <path>"; and if gating is intended,
   "coverage below X% fails the run") or state explicitly that coverage is
   collected but not gated at this stage so the verifier knows there is no
   threshold to test.

3. **Add a verifying criterion for the paths.py multi-crate revisit** (addresses:
   "paths.py revisit has no verifying criterion", scope's paths.py suggestion).
   E.g. "version-coherence/path resolution enumerates all workspace member
   `Cargo.toml` files and a version mismatch in any member is detected."

4. **Bind "every workspace crate" to a checkable source of truth** (addresses:
   "'every workspace crate' is unbounded"). E.g. "every member listed in
   `cli/Cargo.toml [workspace].members` has a corresponding `<crate>:check`
   task", so the verification procedure has a concrete enumeration.

5. **Record the downstream gating and the pinned-nightly coupling in
   Dependencies** (addresses: "Phase 0 gate lists no Blocks entries",
   "pinned-nightly is an external coupling"). Add a Blocks note that this story
   establishes the enforcement floor later phases depend on, and capture the
   pinned-nightly + cargo-pup pairing as an external-tooling maintenance
   dependency.

6. **Name the authority behind the quality-tool stack** (addresses: "'the
   established direction' is undefined", "unratified quality-tool stack").
   Reference the specific decision artefact (source research / 0158 spike), or
   note that ADR ratification of the stack is a tracked follow-up.

7. **Resolve "the Rust component" referent and unify the architecture-rule
   vocabulary** (addresses: "'the Rust component' conflicts with per-crate
   components", "same concept named three ways"). Replace "the Rust component"
   with the intended plural referent; pick one canonical phrase for the
   infra-out-of-domain rule and clarify whether "light" and "domain" crates are
   the same set.

8. **Add the motivating problem and beneficiary to Context/Summary** (addresses:
   "Context explains the extension but not the problem", "no explicit
   beneficiary"). One sentence stating that without these guard rails the new
   workspace has no mechanical enforcement of the ADR-0053/0046 constraints, and
   naming contributors/CI as the beneficiary.

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent unambiguously overall: the
Summary, Requirements, and Acceptance Criteria describe the same scope, actors
are clear, and domain jargon is anchored to linked ADRs. The main weaknesses are
a singular/plural mismatch around "the Rust component" versus the multiple
per-crate components, and a few underspecified referents ("the source", "the
established direction").

**Strengths**:
- Summary, Requirements, and Acceptance Criteria describe one coherent scope with
  no contradictory requirements.
- Architecture-enforcement jargon is consistently tied to specific linked ADRs
  (0053, 0046).
- Actors and triggers are explicit: requirements name what mise tasks add, what
  CI gates, and which tool performs each check.
- The check-vs-test split is stated precisely and consistently.

**Findings**:
- 🔵 minor (confidence: medium) — **Acceptance Criteria**: "the Rust component"
  (singular) conflicts with the per-crate "components" described elsewhere. AC2
  says the default "exits 0 end-to-end with the Rust component included", but
  Requirements introduce per-crate `<crate>:check` components (plural) and AC1
  refers to "every workspace crate." A reader cannot tell whether "the Rust
  component" means the whole `cli/` workspace, a single aggregate task, or the
  collection of per-crate components. Suggestion: replace with the intended
  referent (e.g. "all per-crate Rust checks plus the workspace-scope deny/pup
  checks").
- 🔵 suggestion (confidence: medium) — **Open Questions**: "per the source" /
  "the source research" rely on an implicit referent never named inline; the
  reader must infer it points to the `Source:` References entry. Suggestion: name
  the referent on first use (e.g. "per the 0136 migration-scope research").
- 🔵 suggestion (confidence: low) — **Assumptions**: "the established direction"
  is an undefined referent for the tool-stack choice — unclear whether it means
  the source research, the 0158 spike, the mirrored luminosity item, or team
  convention, which matters because the sentence flags the absence of an ADR.
- 🔵 suggestion (confidence: low) — **Requirements**: same concept named three
  ways ("light/domain crates", "infra-out-of-domain rule", "cross-crate
  ban-lists"); "light" and "domain" crates are used as if interchangeable without
  stating whether they are the same set. Suggestion: pick one canonical phrase
  and clarify the crate-set relationship.

### Completeness

**Summary**: Structurally thorough and substantively populated: every section a
story needs is present and dense, the Summary states a clear scope, and the five
Acceptance Criteria are specific and tied to concrete tooling outcomes. The main
gap is the absence of an explicit beneficiary and the why-now motivation — the
Context explains what the work extends but not the underlying problem.
Frontmatter integrity is clean.

**Strengths**:
- All expected story sections are present and substantively populated — none are
  empty placeholders.
- Five specific Acceptance Criteria, well above the two-criterion floor.
- Frontmatter is complete and coherent (kind, status, priority, parent,
  relates_to, external_id, derived_from).
- Open Questions, Dependencies, and Assumptions carry genuine content.
- Requirements are concrete and actionable — name specific tools, files, and the
  paths.py revisit.

**Findings**:
- 🔵 minor (confidence: medium) — **Context**: Context explains the extension but
  not the motivating problem. It establishes what the story is and its lineage
  but not the cost of not doing it (the new workspace would have no mechanical
  enforcement of the ADR-0053 inward-dependency rule or the ADR-0046 musl-static
  constraint). Suggestion: add one sentence stating the problem being solved.
- 🔵 suggestion (confidence: low) — **Summary**: no explicit beneficiary
  identified — the contributor/CI being protected is implicit. Suggestion:
  optionally note the beneficiary in the Summary or Context.

### Dependency

**Summary**: The work item captures its core couplings well: the pairing with
0163, the parent epic 0136, the downstream ban-list activation at 0166, and the
cross-component touch on `tasks/shared/paths.py`. The main gaps are downstream:
the Dependencies section never states that this Phase 0 gate sits beneath every
later migration phase, and the external pinned-nightly toolchain coupling is
mentioned only as an open question.

**Strengths**:
- The upstream pairing with 0163 is explicitly named with rationale, and both are
  correctly noted as landing before the green-build criteria can pass.
- The downstream activation point of the cargo-deny cross-crate ban-lists is
  captured in Technical Notes (first bite at 0166).
- The cross-component coupling to the Python build system is captured
  (paths.py revisit).

**Findings**:
- 🔵 minor (confidence: medium) — **Dependencies**: this Phase 0 gate blocks
  every downstream migration phase but lists no Blocks entries, leaving the
  gating relationship implicit. Suggestion: add a Blocks note that this story
  establishes the enforcement floor subsequent phases depend on.
- 🔵 minor (confidence: medium) — **Requirements**: the pinned-nightly toolchain
  is an external compatibility coupling (cargo-pup's lint engine is tied to a
  specific nightly ABI) not captured in Dependencies; recorded only as an open
  question with no fallback. Suggestion: capture it as an external-tooling
  coupling.
- 🔵 minor (confidence: low) — **Assumptions**: the quality-tool stack adopted
  without a ratifying ADR is an undated decision coupling that can resurface as a
  re-litigation blocker. Suggestion: reference the source research's decision log
  or note ADR ratification as a follow-up.

### Scope

**Summary**: Work item 0162 describes a single, coherent unit of work —
establishing the automated quality and architecture-enforcement bar for the new
Rust `cli/` workspace, mirroring luminosity 0006 as the Phase 0 foundational
child of epic 0136. The Summary, Requirements, and Acceptance Criteria are
tightly aligned, and story sizing is appropriate. The only modest tension is the
spread of distinct tooling lanes, but the research frames these as one cohesive
bar.

**Strengths**:
- All requirements serve one unified purpose; no scope drift across Summary,
  Requirements, and Acceptance Criteria.
- Clear boundaries: framed as an extension of existing wiring, mapped 1:1 onto
  luminosity 0006.
- The 0163 pairing is captured, with a clean boundary between "wire the gates"
  (0162) and "create the code" (0163).
- Story kind is appropriate for the scope.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Requirements**: the pinned-nightly
  cargo-pup lane is a distinct concern from the stable-lane quality tooling —
  standing up a second toolchain lane is a self-contained sub-effort with a
  different risk profile. Suggestion: confirm during planning it is intended to
  land atomically with the stable gates; consider a sibling story if it carries
  materially higher risk.
- 🔵 suggestion (confidence: low) — **Requirements**: the paths.py revisit is a
  tail-end Python-build-system concern bundled with new-tooling setup. No split
  needed — a legitimately coupled prerequisite — but optionally add an acceptance
  criterion covering the paths.py multi-crate behaviour.

### Testability

**Summary**: The Acceptance Criteria are mostly framed as observable pass/fail
outcomes (`mise run check` exits 0, CI fails and is non-mergeable, a violation
fails the build), giving a verifier concrete commands. The main gaps are
negative-path criteria asserted without a specified triggering input, an
unbounded "every workspace crate" scope, and two requirements (coverage and the
paths.py revisit) not pinned by any criterion.

**Strengths**:
- AC1 and AC2 give exact runnable commands with a definitive expected outcome
  (exit 0), and AC1 asserts the read-only/test-free property of `check`.
- AC3 frames enforcement as a concrete pass/fail behaviour rather than an
  implementation instruction.
- AC5 ties the cargo-pup nightly lane to a verifiable behaviour and pins the
  contrasting stable-product-build expectation.
- Open Questions correctly externalise genuinely undecided inputs as
  implementation-time decisions.

**Findings**:
- 🟡 major (confidence: high) — **Acceptance Criteria**: negative-path criteria
  (deny bans, pup violation) specify no triggering input. AC3, AC4, and AC5
  assert a violation is caught but none specifies the concrete input — a verifier
  cannot demonstrate the guard rail bites versus being configured-but-inert,
  especially given the Technical Note that cross-crate bans are "largely inert
  until the workspace splits." Suggestion: add an explicit fixture/scenario per
  negative-path criterion (a deliberately-failing test case).
- 🟡 major (confidence: high) — **Requirements**: coverage is required but no
  criterion verifies it is computed or enforced. AC2 lists "tests-with-coverage"
  but no criterion states whether a report is produced, a threshold gates the
  build, or coverage is informational. Suggestion: add a criterion fixing the
  observable outcome, or explicitly state coverage is collected but not gated.
- 🔵 minor (confidence: medium) — **Acceptance Criteria**: "every workspace
  crate" is unbounded and currently near-tautological — the workspace is
  single-crate until 0166 and the crate set is not enumerated. Suggestion: bind
  to a checkable source of truth (e.g. `cli/Cargo.toml [workspace].members`).
- 🔵 minor (confidence: medium) — **Requirements**: the paths.py multi-crate
  revisit has no verifying criterion ("revisit" is verb-only with no measurable
  result). Suggestion: add a criterion such as version-coherence enumerating all
  workspace member `Cargo.toml` files.
- 🔵 minor (confidence: medium) — **Assumptions**: the grep-based dependency
  tripwire is introduced but has no acceptance criterion — a net-new check the
  item commits to adding is covered by no criterion. Suggestion: promote it into
  Requirements/Acceptance Criteria with a concrete check.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-28

**Verdict:** COMMENT

Both original major findings are resolved, so the work item drops below the
REVISE threshold (2 majors). It is now acceptable as-is. The edits that added
deliberately-failing fixtures and bound the criteria to concrete artefacts
sharpened the acceptance set — but in doing so they surfaced one genuine,
previously-latent issue: several criteria (the infra-out-of-domain fixture, the
per-crate "every member" enumeration, and the green-build) are not independently
verifiable within 0162 because they depend on the paired scaffold 0163 and the
downstream split at 0166. This is the dominant remaining theme; it is a
COMMENT-level observation, not a blocker.

### Previously Identified Issues

- 🟡 **Testability**: Negative-path criteria specify no triggering input —
  **Resolved**. AC4/AC5 now name deliberately-failing fixtures (native-tls/OpenSSL
  crate → `deny check` non-zero; domain→adapter import → build fails), and a grep
  tripwire AC was added.
- 🟡 **Testability**: Coverage required but no criterion verifies it — **Resolved**.
  A dedicated AC now states the `test` roll-up emits an llvm-cov report, collected
  but not gated at Phase 0. (A minor refinement remains — see New Issues.)
- 🔵 **Clarity**: "the Rust component" singular/plural mismatch — **Resolved**.
  AC2 now reads "all per-crate Rust checks plus the workspace-scope deny/pup checks
  and the tests-with-coverage run."
- 🔵 **Clarity**: "the source" / "the source research" implicit referent —
  **Resolved**. Open Questions now says "per the source research — see References."
- 🔵 **Clarity**: "the established direction" undefined — **Resolved**. Assumptions
  now names the 0158 architecture spike, added to References.
- 🔵 **Clarity**: same concept named three ways — **Partially resolved**.
  Requirements now use the canonical "infra-out-of-domain dependency rule", but a
  new "domain (light) crates" conflation was introduced (see New Issues).
- 🔵 **Completeness**: Context lacks motivating problem — **Resolved**. Context now
  states the cost of not doing the work (no mechanical enforcement of ADR-0053/0046).
- 🔵 **Completeness**: no explicit beneficiary — **Resolved**. Summary now names
  contributors and CI; downgraded to a soft suggestion.
- 🔵 **Dependency**: Phase 0 gate lists no Blocks entries — **Resolved**.
  Dependencies now has a Blocks entry (0163–0174 inherit the floor; 0166 first bite).
- 🔵 **Dependency**: pinned-nightly external coupling uncaptured — **Resolved**.
  Dependencies now records the cargo-pup nightly ABI coupling and its isolation.
- 🔵 **Dependency**: unratified quality-tool stack — **Resolved**. Assumptions names
  the 0158 spike as the direction-setter and flags ADR ratification as a follow-up.
- 🔵 **Scope**: pinned-nightly lane as a distinct concern — **Resolved/accepted**.
  Not re-raised; kept bundled by deliberate decision (research treats it as one bar).
- 🔵 **Scope** / **Testability**: paths.py revisit / "every workspace crate"
  unverified — **Resolved**. A paths.py version-coherence AC was added and the
  crate set is now bound to `cli/Cargo.toml [workspace].members`. (A residual
  contingency on 0166 remains — see New Issues.)
- 🔵 **Testability**: grep tripwire had no AC — **Resolved**. Promoted into its own
  AC. (A refinement on the matched import pattern remains — see New Issues.)

### New Issues Introduced

- 🟡 **Dependency** (major): The infra-out-of-domain cargo-deny fixture (and,
  read closely, the per-crate "every member" checks and green-build) cannot be
  fully demonstrated until the workspace splits at 0166 / scaffold 0163 lands —
  yet 0166 appears only as something this story *blocks*, creating a
  circular-looking ordering. The native-tls/OpenSSL ban is verifiable now; the
  cross-crate ban fixture is not.
- 🔵 **Scope** (minor): Several acceptance criteria are not independently
  verifiable within 0162 — the "done" state is observable only jointly with 0163.
- 🔵 **Clarity** (minor): "domain (light) crates" conflates two terms the source
  research distinguishes (light crates like `kernel`/`config` vs domain subdomain
  crates); the protected set should be defined once.
- 🔵 **Clarity** (minor): "the foundation ADRs" in Context is an unpinned set.
- 🔵 **Dependency** (minor): the per-crate AC depends on the `cli/Cargo.toml`
  workspace-manifest contract owned by 0163; not called out as a contract coupling.
- 🔵 **Dependency** (minor): cargo-deny/nextest/llvm-cov are new pinned external
  tools whose CI availability coupling is not captured the way the nightly lane's is.
- 🔵 **Testability** (minor): the coverage report's wellformedness (non-empty,
  covers each member) is not checkable — only "a report is emitted".
- 🔵 **Testability** (minor): the grep tripwire names no concrete import pattern
  (the source research gives `use crate::{adapters,inbound,outbound}`).
- 🔵 **Suggestions**: annotate bare `0166`; disambiguate the cargo-pup vs grep
  overlapping criteria; add an observable check for nightly-lane isolation.

### Assessment

The work item is **ready for implementation as-is** — it is materially stronger
than at pass 1, with measurable, fixture-backed criteria and well-mapped
dependencies. The remaining issues are all variations on a single theme worth a
brief deliberate decision before planning: **how to handle acceptance criteria
that 0162 cannot demonstrate alone.** The cleanest resolution is to either (a)
frame 0162 + 0163 as a jointly-verified pair and move the cross-crate
domain→infra fixture into 0166's acceptance criteria where the split makes it
bite, or (b) explicitly scope 0162's criteria to what is checkable against the
minimal scaffold. Neither is blocking; both would tighten the story's "done"
boundary. The clarity refinements (define the domain/light crate set; pin "the
foundation ADRs"; concrete grep pattern) are low-effort polish.
