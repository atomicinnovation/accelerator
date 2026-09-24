---
type: "work-item-review"
id: "0226-unify-the-trust-barrier-for-consent-config-keys-review-1"
title: "Work Item Review: Unify the Trust Barrier for Consent Config Keys"
date: "2026-09-24T12:53:22+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0226"
work_item_id: "0226"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: ["security", "config"]
last_updated: "2026-09-24T13:35:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Unify the Trust Barrier for Consent Config Keys

**Verdict:** REVISE

The item is well-grounded: it defines consent keys and "the repository"
precisely, tabulates the current barriers, draws an explicit out-of-scope
boundary and records its breaking changes as assumptions. It falls short on
the contract it asks implementers to meet — the shared runner's security
properties, the unified refusal codes, per-consumer severity and the
declaration-driven inheritance promise are stated as requirements but not
pinned by verifiable criteria — and the Summary promises a stronger guarantee
than the environment-override assumption allows. Five major findings (no
critical) exceed the two-major REVISE threshold.

### Cross-Cutting Themes

- **Environment-override semantics are stated inconsistently** (flagged by:
  clarity, testability) — the Summary's absolute "never", the ambiguous "it is
  refused" in the tracked-file criteria, the unmapped provenance/value-check
  vocabulary, and the missing negative case (a valid env value accepted despite
  a tracked `config.local.md`) all stem from one unstated rule about what env
  overrides receive.
- **Requirements without criteria** (flagged by: completeness, testability) —
  Requirements 5, 7, 8, 11 and 12 can each be dropped without failing any
  acceptance criterion.
- **Refusal-code contract undefined** (flagged by: testability, clarity) — no
  code names, no organising principle (per reason vs per key), and only a docs
  search as evidence.
- **Unresolved questions hidden in Technical Notes** (flagged by: completeness,
  testability) — whether `ACCELERATOR_ALLOW_INSECURE_LOCAL` is reachable, and
  which `GH_*` variables the runner admits, while Requirement 11 deletes the
  former unconditionally.
- **Breadth and rollback coupling** (flagged by: scope, dependency) — twelve
  requirements across config, three tracker consumers, the design daemon, the
  hook and docs, with two breaking changes sharing one rollback unit.
- **Thin Dependencies section** (flagged by: dependency, completeness) — 0183,
  0080, 0272 and the parent 0136 carry no stated relationship; 0196 and 0227
  lack direction.

### Findings

#### Major

- 🟡 **Clarity**: Summary's "never name or supply a binary" contradicts
  Assumptions and Out of scope
  **Location**: Summary
  A repository can still name a binary via `ACCELERATOR_*` in `mise.toml`
  `[env]` or `.envrc` (value-checked only), and outer-repository binaries are
  out of scope; the absolute guarantee overstates the design.
- 🟡 **Testability**: Shared runner's environment scrub, null stdin, timeout
  and output cap are unverified
  **Location**: Acceptance Criteria
  Only the cwd and `gh auth token` are checked; an implementation dropping the
  timeout, cap or scrub passes every criterion.
- 🟡 **Testability**: Unified refusal-code contract has no verifiable criterion
  beyond a docs search
  **Location**: Acceptance Criteria
  Nothing checks the codes the tooling emits, or that all five keys emit the
  same code for the same reason.
- 🟡 **Testability**: "Every applicable criterion above applies" to a new key
  has no clear pass or fail
  **Location**: Acceptance Criteria
  Which criteria apply to a plain consent vs path-valued vs command-valued key
  is unstated, and the path criteria are written for `design.browser_path`.
- 🟡 **Scope**: Breadth exceeds a single task; natural seams already identified
  **Location**: Frontmatter: kind
  The Drafting Notes' policy → runner → consumers and summary order names
  seams that could be sibling children of 0136, or the item reclassified as a
  story.

#### Minor

- 🔵 **Clarity**: Two barrier vocabularies used without a mapping
  **Location**: Context / Requirement 6
  "Personal-only/repo-inside" vs "provenance/value checks"; whether env
  commands get the full runner or only the cwd move is ambiguous.
- 🔵 **Clarity**: "It is refused" in the tracked-file criteria has an unclear
  referent
  **Location**: Acceptance Criteria
  Read as the whole key, it would refuse env overrides, contradicting
  Requirement 6.
- 🔵 **Clarity**: "Usable value" in the fatal-refusal rule is ambiguous
  **Location**: Requirement 7
  Unclear whether a consumer fallback (bundled browser, default
  `*.atlassian.net`) counts.
- 🔵 **Testability**: Per-consumer refusal severity verified only for
  `design.browser_path`
  **Location**: Acceptance Criteria
  No outcome for a refused `token_cmd` or `allowed_sites` with no usable value.
- 🔵 **Testability / Completeness**: Requirements 11 and 12 have no criteria
  **Location**: Requirements / Acceptance Criteria
  The duplicate `Provenance` types, `ACCELERATOR_ALLOW_INSECURE_LOCAL` and the
  `visualiser.editor` exemption doc could all survive a "done" item.
- 🔵 **Testability**: Missing negative cases
  **Location**: Acceptance Criteria
  No check that a valid env value is accepted despite a tracked file, or that
  an unchanged-path daemon is reused.
- 🔵 **Completeness**: No Open Questions section despite unresolved questions
  in Technical Notes
  **Location**: Open Questions
  `ACCELERATOR_ALLOW_INSECURE_LOCAL` reachability and the `GH_*` admission set.
- 🔵 **Clarity**: "Hands the canonical path to the launcher" has an ambiguous
  referent
  **Location**: Requirement 4
  "Launcher" names the `accelerator` launcher crate elsewhere in the project.
- 🔵 **Clarity**: "Executor" is undefined jargon; "Playwright daemon" and
  "design daemon" used interchangeably
  **Location**: Context
- 🔵 **Scope**: Fail-closed tracking fix is independently shippable
  **Location**: Requirements
  A live fail-open defect waits on the whole unification.
- 🔵 **Scope**: Design daemon restart is a separate subsystem change
  **Location**: Requirement 10
- 🔵 **Scope**: Breaking runner changes share a rollback unit with the
  security barriers
  **Location**: Assumptions
- 🔵 **Dependency**: 0227 recorded as "Relates to" though it is a likely
  downstream consumer
  **Location**: Dependencies
- 🔵 **Dependency**: Whether 0196 must land first is not stated
  **Location**: Dependencies
- 🔵 **Dependency**: Reliance on 0183's `additionalContext` channel is not
  captured
  **Location**: Context

#### Suggestions

- 🔵 **Clarity**: Relationship of executable kinds to consent keys is worded
  inconsistently
  **Location**: Requirement 1 / Acceptance Criteria
- 🔵 **Clarity**: Structure of the unified refusal codes is unspecified
  **Location**: Requirement 8
- 🔵 **Testability**: `gh auth token` criterion depends on verifier machine
  state
  **Location**: Acceptance Criteria
- 🔵 **Scope**: Summary omits the runner, hook warnings and daemon restart
  **Location**: Summary
- 🔵 **Dependency / Completeness**: 0080, 0272 and parent 0136 referenced with
  no stated coupling
  **Location**: References / Dependencies

### Strengths

- ✅ Consent key is defined in the first sentence and used consistently; the
  widening from executable keys is justified in Context and Drafting Notes.
- ✅ Requirement 3's three-root definition of "the repository" removes the
  largest latent ambiguity, and the criteria reuse it verbatim.
- ✅ The Context table makes the current per-key inconsistency concrete.
- ✅ The out-of-scope list is explicit and specific, and the `visualiser.editor`
  premise was dropped on evidence.
- ✅ Criteria cover the hard edge cases: empty personal masking, failed tracking
  query, no VCS, symlink and nonexistent-file escapes, `$HOME` as a repository
  root, and a daemon launched with no path.
- ✅ Breaking changes (the `token_cmd` cwd move, the `github.token_cmd` runner)
  are recorded as accepted assumptions rather than left implicit.
- ✅ Every consumer and bespoke barrier to migrate is named with its location,
  and the Drafting Notes give an internal build order.

### Recommended Changes

1. **State the env-override rule once and align every section to it**
   (addresses: Summary contradiction, two barrier vocabularies, "it is
   refused" referent, missing negative cases)
   Map provenance checks (team-level, tracked, tracking-unknown) and value
   checks (Requirement 4 path rules, the full shared runner) once; qualify the
   Summary to "a repository's config files can never set a consent key";
   reword the tracked-file criteria to "the value from `config.local.md` is
   refused, and a valid `ACCELERATOR_*` override is still used", with a
   matching criterion.
2. **Pin the shared runner's properties with criteria** (addresses: runner
   unverified, `gh auth token` machine state)
   Add criteria for a command exceeding the timeout, exceeding the output cap,
   reading stdin (EOF) and printing its environment (only the admitted set),
   with the thresholds and the admitted `GH_*` variables named.
3. **Define the refusal codes** (addresses: refusal-code contract, code
   structure)
   Name one code per refusal reason carrying the key as a parameter, and add a
   criterion that all five keys emit the same code per reason and no retired
   code appears in CLI output or source.
4. **Make inheritance concrete** (addresses: open-ended new-key criterion,
   executable-kind wording)
   Replace the final criterion with a test-only catalogue entry per kind
   (consent, path-valued, command-valued) and the specific outcomes each must
   exhibit; state that every executable key is a consent key.
5. **Specify per-consumer severity** (addresses: "usable value", severity only
   verified for the browser)
   Define "usable value" and add a small per-key table of the refusal outcome
   when no usable value remains.
6. **Cover Requirements 11 and 12, and surface the open questions** (addresses:
   deletions and exemption unverified, no Open Questions section)
   Add criteria for the single `Provenance`, the inert
   `ACCELERATOR_ALLOW_INSECURE_LOCAL` and the `visualiser.editor` exemption
   doc; move the reachability and `GH_*` questions into an Open Questions
   section, making the deletion conditional on the first.
7. **Decide the split** (addresses: breadth, fail-closed fix, daemon restart,
   breaking-change rollback, Summary omissions)
   Either split along the named seams as children of 0136 — or keep one item
   and record why, naming the phases and their rollback points — and extend
   the Summary accordingly.
8. **Give every linked item a direction** (addresses: 0227, 0196, 0183, 0080,
   0272, 0136)
   Record Blocks/Blocked-by/Relates-to with a one-line reason each, and drop
   any link that proves incidental.
9. **Fix referents** (addresses: "launcher", "executor")
   Name the design executor as the recipient of the canonical path, define it
   on first use, and use one name for the daemon.

## Per-Lens Results

### Clarity

**Summary**: Clear for its breadth: consent key is defined, "the repository"
has a precise three-root definition, and the Context table grounds the
problem. Weaknesses are cross-section consistency — the Summary overstates
the guarantee relative to Assumptions and Out of scope, two barrier
vocabularies go unmapped — and a handful of ambiguous referents.

**Strengths**:
- Consent key defined up front and used consistently.
- Requirement 3's three-root definition, reused by the criteria.
- Context table makes the current state concrete.
- `visualiser.editor` exclusion stated with its reason in three places that
  agree.
- Behavioural changes stated openly in Assumptions.

**Findings**:
- 🟡 major / high — Summary: "never name or supply a binary" contradicts
  Assumptions (env overrides via `mise.toml`/`.envrc` are value-checked only)
  and Out of scope (outer-repository binaries). Qualify the Summary.
- 🔵 minor / high — Context / Requirement 6: personal-only/repo-inside vs
  provenance/value checks are never mapped; Requirement 6 calls Requirement 5
  a check though it is a runner; Assumptions' value-check list omits the
  scrub, timeout and cap. Pick one vocabulary and map it once.
- 🔵 minor / medium — Requirement 4: "the launcher" collides with the
  `accelerator` launcher crate; name the design executor.
- 🔵 minor / medium — Context: "executor" undefined; "Playwright daemon" and
  "design daemon" used interchangeably.
- 🔵 minor / medium — Acceptance Criteria: "it is refused" could mean the key,
  which would refuse env overrides against Requirement 6.
- 🔵 minor / medium — Requirement 7: "usable value" undefined; "only when" is
  necessary, not sufficient.
- 🔵 suggestion / medium — Requirement 1 vs final criterion: "extend" vs "or
  an executable key" leaves open whether executable keys are consent keys.
- 🔵 suggestion / low — Requirement 8: whether codes are per reason with the
  key as a parameter is unstated.

### Completeness

**Summary**: Structurally very complete — Summary, Context with a
current-state table, twelve requirements with an out-of-scope list, seventeen
Given/When/Then criteria, Assumptions, Technical and Drafting Notes, and
valid frontmatter. Gaps: no Open Questions section, several requirements
without criteria, and a sparse Dependencies section.

**Strengths**:
- Summary names all five keys and the declaration-driven goal.
- Context explains the threat model and each current gap.
- Requirements are specific enough to implement, with an explicit boundary.
- Criteria span every behavioural area.
- Accepted breaking changes recorded as assumptions.
- Frontmatter complete and consistent with the body.

**Findings**:
- 🔵 minor / medium — Open Questions: absent, while Technical Notes leave
  `ACCELERATOR_ALLOW_INSECURE_LOCAL` reachability and the `GH_*` set open and
  Requirement 11 deletes the former unconditionally.
- 🔵 minor / medium — Acceptance Criteria: Requirements 12, 11 (deletions), 8
  (bespoke wordings) and 7 have no dedicated criterion.
- 🔵 suggestion / low — Dependencies: parent 0136 and related 0080, 0272
  carry no stated relationship.

### Dependency

**Summary**: Internal couplings are well mapped — every consumer and barrier
is named, with a build order. External couplings are thin: two "Relates to"
entries, 0227 probably blocked but not marked, 0183/0080/0272 unexplained,
and 0196's completion state unstated.

**Strengths**:
- Every migrated consumer named with its code location.
- Build order policy → runner → consumers and summary stated.
- Downstream user impact of the cwd move recorded as an assumption.
- The `gh` environment coupling raised and backed by a criterion.

**Findings**:
- 🔵 minor / medium — Dependencies: 0227 "can reuse the policy" is a likely
  Blocks relationship; record direction.
- 🔵 minor / medium — Dependencies: state whether 0196 has shipped, since this
  item rewrites its barrier and daemon code.
- 🔵 minor / medium — Context: Requirement 9 relies on an `additionalContext`
  warning path; state whether 0183 delivered it or this item builds it.
- 🔵 suggestion / low — References: 0080 and 0272 have no stated coupling.

### Scope

**Summary**: One clear purpose with a well-drawn boundary, but a single task
spanning the catalogue, a policy, a runner, three tracker consumers, the
design daemon, the hook and a docs sweep, with two breaking changes. The
Drafting Notes already name seams; several parts are independently shippable
and revertable.

**Strengths**:
- Every requirement serves the one stated purpose.
- Explicit, specific out-of-scope list.
- Scope revised on evidence (`visualiser.editor` dropped, `allowed_sites`
  added).
- Breadth is a conscious, recorded choice with a phasing order.

**Findings**:
- 🟡 major / medium — Frontmatter: kind: breadth exceeds a task; split along
  the named seams under 0136, or reclassify as a story.
- 🔵 minor / medium — Requirements: the fail-closed tracking fix could ship
  alone, closing a live gap sooner.
- 🔵 minor / medium — Requirement 10: the daemon restart is a separate
  subsystem change with its own rollback needs.
- 🔵 minor / low — Assumptions: breaking runner changes share a rollback unit
  with the barriers.
- 🔵 suggestion / medium — Summary: omits the runner, repository definition,
  fail-closed tracking, hook warnings and daemon restart.

### Testability

**Summary**: Strongly testable overall — explicit Given/When/Then with
concrete preconditions and observable outcomes across a bounded five-key
matrix. Gaps: the runner's properties, emitted refusal codes, per-consumer
severity, the deletions and exemption doc are unverified, and the
inheritance criterion lacks a clear pass/fail.

**Strengths**:
- Preconditions cover the hard edge cases.
- Bounded five-key matrix.
- SessionStart criteria name `additionalContext` as the observable channel.
- Daemon criterion includes the "launched with none" case; browser criteria
  state the fallback outcome.

**Findings**:
- 🟡 major / high — Acceptance Criteria: scrubbed env, null stdin, timeout
  (30s) and cap (64KiB) have no criterion.
- 🟡 major / high — Acceptance Criteria: emitted refusal codes unverified
  beyond a docs search.
- 🟡 major / medium — Acceptance Criteria: "every applicable criterion"
  inheritance check is open-ended; specify a test-only entry per kind.
- 🔵 minor / high — Acceptance Criteria: severity outcome only verified for
  `design.browser_path`.
- 🔵 minor / high — Requirements: 11 and 12 have no criteria, and the
  `ACCELERATOR_ALLOW_INSECURE_LOCAL` reachability test is not part of done.
- 🔵 minor / medium — Acceptance Criteria: no negative cases for env overrides
  under a tracked file or daemon reuse on an unchanged path.
- 🔵 suggestion / medium — Acceptance Criteria: `gh auth token` depends on
  verifier machine state; state preconditions and add a deterministic `GH_*`
  pass-through check.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-09-24T13:17:59+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Summary's "never name or supply a binary" contradicts
  Assumptions — Partially resolved (the env-override contradiction is gone;
  the new wording overclaims for command-valued keys, see below)
- 🟡 **Testability**: Shared runner properties unverified — Resolved
- 🟡 **Testability**: Refusal-code contract unverifiable — Resolved
- 🟡 **Testability**: Open-ended inheritance criterion — Resolved
- 🟡 **Scope**: Breadth exceeds a single task — Still present; the author
  decided to keep one phased item, so this stands as an accepted decision
- 🔵 **Clarity**: Two barrier vocabularies unmapped — Resolved
- 🔵 **Clarity**: "It is refused" referent — Resolved
- 🔵 **Clarity**: "Usable value" ambiguous — Resolved
- 🔵 **Clarity**: "The launcher" referent — Resolved
- 🔵 **Clarity**: "Executor" undefined; two daemon names — Resolved
- 🔵 **Testability**: Severity verified only for `design.browser_path` —
  Partially resolved (fatal side covered; warn-and-continue side still
  untested, see below)
- 🔵 **Testability / Completeness**: Requirements 11 and 12 have no criteria —
  Partially resolved (the exemption doc and single `Provenance` are covered;
  the insecure-local deletion itself is not)
- 🔵 **Testability**: Missing negative cases — Resolved
- 🔵 **Completeness**: No Open Questions section — Resolved
- 🔵 **Scope**: Fail-closed fix, daemon restart and breaking changes coupled —
  Resolved by the phased rollback points
- 🔵 **Dependency**: 0227, 0196, 0183 relationships — Resolved
- 🔵 **Clarity**: Executable kinds vs consent keys — Partially resolved (the
  subtype rule is stated; the attribute's shape is not)
- 🔵 **Clarity**: Refusal-code structure — Resolved
- 🔵 **Testability**: `gh auth token` depends on machine state — Partially
  resolved (precondition stated; still not reproducible in CI)
- 🔵 **Scope**: Summary omits parts of the scope — Resolved
- 🔵 **Dependency / Completeness**: 0080, 0272, 0136 unexplained — Resolved

### New Issues Introduced

- 🟡 **Clarity**: Summary's "no consent key can launch a binary inside the
  repository" holds only for path-valued keys; a `token_cmd` naming an
  absolute in-repository script still runs, since Requirement 5 only moves the
  cwd.
- 🟡 **Clarity**: "The repository" drifts after Requirement 3 defines it —
  `$HOME` "may itself be a repository root" means any VCS root, and the daemon
  is "reused per repository root", a single root.
- 🟡 **Testability**: The environment-scrub criterion has no expected set
  until the `GH_*` open question is settled.
- 🟡 **Testability**: Requirement 7's warn-and-continue side is untested for
  the `token_cmd` keys and `jira.allowed_sites`.
- 🔵 **Completeness / Testability / Clarity**: Timeout and output-cap refusals
  have no codes, and the 64KiB boundary is unstated.
- 🔵 **Completeness**: The runner's cwd is defined only by what it must not
  be.
- 🔵 **Completeness**: The `GH_*` open question has no default or resolution
  approach; clarity also asks whether the admission is scoped to
  `github.token_cmd`.
- 🔵 **Dependency / Completeness**: 0172 is missing from `relates_to` and has
  no stated status.
- 🔵 **Dependency**: Phase 1 does not close the `token_cmd` fail-open gap —
  the duplicate `Provenance` implementations guard those keys until phase 2.
- 🔵 **Dependency**: `gh` is an uncaptured external dependency.
- 🔵 **Scope**: 0227 needs only phase 1 but waits on the whole item; the
  Summary's "along the way" understates first-class scope.
- 🔵 **Clarity**: Requirement 11's "subject to the first open question"
  conflicts with the question's framing that deletion happens either way.
- 🔵 **Testability**: The retirement search names "bespoke wording" without
  listing it; the tracked-file session warning has no required content; the
  canonical-launch criterion needs an input whose canonical form differs; the
  `jira.allowed_sites` criterion's "only if … personal" conflicts with
  Requirement 6's env-override admission.
- 🔵 **Clarity** (suggestions): the trust attribute's shape; glosses for "the
  store", "config root" and "the personal route"; whether a team-level
  `github.token_cmd` warns when a plaintext `github.token` wins.

### Assessment

The first pass's substantive gaps are closed: the runner, the refusal codes,
inheritance and the env-override rule are now pinned by criteria. What
remains is a second tier of precision — the two new clarity majors are
wording fixes, and the two testability majors need one decision (the `GH_*`
set) and two added criteria. The scope major is an accepted decision. One
more short pass should reach COMMENT.

## Re-Review (Pass 3) — 2026-09-24T13:35:09+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Summary overclaims for command-valued keys — Resolved
- 🟡 **Clarity**: "The repository" drifts — Partially resolved (Requirement 3
  and the runner are consistent; one Context paragraph still uses the phrase
  for today's single-root behaviour)
- 🟡 **Testability**: Environment-scrub criterion depends on the `GH_*`
  question — Resolved
- 🟡 **Testability**: Warn-and-continue side untested — Partially resolved
  (covered for the team-level reason only)
- 🟡 **Scope**: Breadth exceeds a single task — Still present; the author's
  keep-one-item decision stands, but the recorded rationale (interim items
  would ship inconsistent codes) no longer holds now phase 1 ships the full
  code set
- 🔵 **Completeness / Testability / Clarity**: Runner refusal codes and the
  cap boundary — Resolved
- 🔵 **Completeness**: Runner cwd defined only negatively — Resolved
- 🔵 **Completeness**: `GH_*` open question — Resolved
- 🔵 **Dependency / Completeness**: 0172 missing — Resolved
- 🔵 **Dependency**: Phase 1 fail-open claim — Resolved
- 🔵 **Dependency**: `gh` uncaptured — Resolved
- 🔵 **Scope**: 0227 waits on the whole item — Partially resolved (stated in
  prose; the `blocks` relation cannot express it)
- 🔵 **Clarity**: Requirement 11 conditional deletion — Resolved
- 🔵 **Testability**: Retirement search, tracked-file warning content,
  canonical-launch input, `jira.allowed_sites` env override — Resolved
- 🔵 **Clarity** (suggestions): attribute shape, term glosses, `github.token`
  warning — Resolved

### New Issues Introduced

- 🟡 **Testability**: "Exactly `PATH`, `HOME` and `TERM`" cannot pass under
  `bash -c`, which adds `PWD`, `SHLVL` and `_`; the jira/linear criterion also
  lacks a parent-environment precondition.
- 🔵 **Clarity**: The config root reads as the marker directory rather than
  its parent, and the search start is unnamed.
- 🔵 **Clarity / Testability**: The daemon restart on a fall-back to the
  bundled browser is neither defined nor tested, nor is how a restart is
  observed.
- 🔵 **Clarity**: "Reported" and "warns" name no runtime channel.
- 🔵 **Clarity / Completeness**: "Phase 1" is referenced from Dependencies
  but defined only in Drafting Notes.
- 🔵 **Clarity**: The Summary omits the tracked-file session warning, and
  Requirement 9 leaves its trigger open.
- 🔵 **Testability**: The path criterion accepts either code for either
  input.
- 🔵 **Testability**: "Killed" and the stream the cap counts are undefined.
- 🔵 **Testability / Completeness**: Per-run freshness and removal of the
  temporary cwd are unchecked.
- 🔵 **Testability**: "Not mode 0600" conflicts with the store's
  group/other-readable rule.
- 🔵 **Testability**: The single-`Provenance` search is undefined.
- 🔵 **Dependency**: Existing `allow-insecure-local` markers have no cleanup
  or advisory; out-of-scope hazards have no follow-up items.
- 🔵 **Scope**: The daemon restart and `SessionStart` warnings are separable;
  per-key barrier deletion is split from its migration phase.
- 🔵 Suggestions: the unification criterion's reason/key matrix, the
  precedence order behind "winning", a clean-config negative case and a
  single-JSON-object check for the hook.

### Assessment

Pass 2's majors are closed or narrowed to one residue each. The remaining
majors are one wording fix (Context), one criterion restatement (`bash -c`
environment) and one criterion table (severity per refusal reason), plus the
scope question now that its rationale has weakened. Findings are converging
on implementation-level precision; after this round's fixes, a further pass
is unlikely to change the item's shape.

### Post-Pass-3 Edits

The pass-3 findings were addressed without a further review pass: the item
was reclassified as a story, its delivery phases moved to Technical Notes,
and the severity matrix, `bash`-boundary environment criteria, runner
kill/cap/working-directory criteria, daemon fall-back restart, config-root
and file-mode boundaries, and `SessionStart` negative cases were added. The
REVISE verdict above reflects pass 3 as reviewed, not the edited item.
