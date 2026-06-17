---
type: plan-review
id: "2026-06-20-0122-executable-bits-skill-entrypoint-scripts-review-1"
title: "Plan Review: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts"
date: "2026-06-20T18:19:51+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-20-0122-executable-bits-skill-entrypoint-scripts"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, usability, documentation, standards, portability]
review_number: 1
review_pass: 3
tags: [scripts, permissions, ci, lint, plugin, executable-bit]
last_updated: "2026-06-20T18:46:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts

**Verdict:** REVISE

This is a well-engineered plan with a sound core design: a single clearly-stated
invariant (*a tracked `.sh` is executable iff it is not on the library-list*),
enforced bidirectionally by reusing the existing fail-closed `shell_sources()`
shell-lint idiom, decomposed into three independently-mergeable, dependency-ordered
phases, with an unusually strong TDD strategy (anti-vacuous-pass sentinel,
synthetic-tree unit layers, real-tree integration closing the loop). The reason for
REVISE is not any structural flaw but a cluster of issues converging on the plan's
single source of truth — the `SHELL_LIBRARIES` constant: its membership is
internally contradictory (23/26/30 entries), its load-bearing classification rule
("sourced AND zero path-invocations") is neither self-verified by the guard nor
covered by any automated test, and two adjacent mechanisms (the `test-fixtures/`
exemption and the working-copy-vs-committed-mode read) are under-documented and
broader than their stated rationale. Each is straightforward to address; together
they cross enough lenses to warrant a revision pass before implementation.

### Cross-Cutting Themes

- **The `SHELL_LIBRARIES` count is internally contradictory (23 / 26 / 30)**
  (flagged by: Architecture, Code Quality, Correctness, Documentation, Standards,
  Usability, Test Coverage — 7 of 8 lenses). The Overview says "23 mode changes",
  the §1 header and embedded code comment say "26 sourced-only paths", the literal
  lists 30 entries, and a trailing note says "trust the audit, not the count". The
  manifest is the linchpin of the entire invariant; shipping it self-contradictory
  invites either a false-positive guard failure (a real library omitted → wrongly
  required `+x`) or a silently-weakened invariant (an entrypoint wrongly listed →
  its missing `+x` never caught, the exact 0106 failure this work exists to
  prevent).

- **The library-list's correctness is unverified and untestable** (flagged by:
  Architecture, Code Quality, Test Coverage, Correctness). The guard verifies only
  the *cheap half* (the listed path exists); the *load-bearing half* — "sourced AND
  never invoked by path", which the research itself says is the only thing making
  the bidirectional invariant sound — stays a manual-only audit deferred to
  "implementation time". The classification is genuinely non-obvious (three
  documented dual-use scripts), so a wrong future addition would pass CI green while
  breaking a production entrypoint at runtime.

- **The `test-fixtures/` exemption is under-documented and broader than its
  rationale** (flagged by: Documentation, Correctness, Architecture, Code Quality).
  The rationale ("fixtures are bash-run, never need `+x`") justifies only one
  direction, but `if "test-fixtures" in rel.split("/"): continue` skips *both*
  directions for *any* path containing that segment anywhere in the tree; and the
  exemption appears in no AC6 documentation a contributor would encounter.

- **The guard reads the working-copy mode; the ACs specify the committed VCS mode**
  (flagged by: Portability, Test Coverage, Correctness, Code Quality, Usability).
  The plan acknowledges this but treats it as only a "local uncommitted `chmod`"
  concern. CI runs on Linux/git while local dev runs on macOS/jj, and the sibling
  `tasks/test/helpers.py` precedent explicitly defends against exec-bit-lossy /
  bit-synthesising filesystems; the new guard has no equivalent safeguard and no
  test asserting modes are actually *committed*.

### Tradeoff Analysis

- **Consistency vs single-source-of-truth (Architecture vs the existing design).**
  The library-list deliberately duplicates classification knowledge the test runner
  already derives differently (`tasks/test/helpers.py` discovers suites by exec bit
  + a name-exclusion list). A second representation of "sourced-only" can drift from
  the first undetected. Recommendation: accept the duplication (the list is small
  and slow-changing) but add a cross-consistency assertion so the two notions cannot
  silently diverge.

- **Uniform "consistency 755" vs strictly-required `+x` (Architecture/Correctness).**
  Only the Python suite runner *strictly* needs the bit; bare-path SKILL.md
  invocation is the other real consumer, while sibling/migration calls use `bash X`
  and never stat it. Enforcing uniform 755 on every entrypoint is the right
  consistency call, but it is a *convention*, not a per-script functional
  requirement — and the Desired End State doesn't frame it that way. Recommendation:
  keep the uniform rule; label it explicitly as a convention so future maintainers
  understand its basis if 0106's mechanics ever change.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Documentation / Standards / Usability / Test Coverage**: `SHELL_LIBRARIES` membership is internally contradictory (23/26/30) and not yet reconciled
  **Location**: Overview; Current State Analysis; Phase 2 §1 (the library-list constant) and its trailing note
  The plan describes the manifest with three conflicting cardinalities and defers the authoritative membership to "implementation time". Because every executable-bit decision derives from this set, an off-by-one, dropped, or duplicated entry from the manual re-derivation ships silently — the guard's other tests run on synthetic trees and cannot detect a wrong real-world list.

- 🟡 **Architecture / Code Quality / Test Coverage / Correctness**: The library-list's load-bearing classification is neither self-verified nor tested
  **Location**: Phase 2 §1 (the constant) and §3 (library-list integrity test)
  The integrity test asserts only that each listed path *exists*. The "sourced AND zero path-invocations" half — the sole soundness condition per the research — and list *completeness* stay manual-only. A sourced-only library accidentally omitted is invisible: Phase 1 already `chmod`'d it 755, so the real-tree guard passes green forever while it ships executable contrary to the invariant.

- 🟡 **Test Coverage**: The three dual-use scripts have no regression test pinning their classification
  **Location**: Phase 2/3 Testing Strategy
  `linkage-parser.sh`, `validate-source.sh`, `jira-fields.sh` are the central correctness trap (sourced *and* path-invoked → must stay off the list at 755). The single most error-prone classification decision has zero automated guard: a wrong list addition would pass CI (file becomes 644) while breaking a production entrypoint at runtime.

- 🟡 **Documentation / Correctness / Architecture / Code Quality**: The `test-fixtures/` exemption is undocumented and bidirectionally over-broad
  **Location**: Phase 2 §2 (guard `@task`); Decisions item 1; Phase 2 §4 (docs)
  The exemption skips *both* directions for *any* path with a `test-fixtures` segment anywhere in the tree, though its rationale justifies only the "never needs `+x`" direction; and it appears in none of the AC6 documentation. A contributor adding a fixture sees the guard neither demand `+x` nor require a list entry, with no documented "why".

- 🟡 **Portability / Test Coverage / Correctness / Code Quality / Usability**: Guard reads working-copy mode, but the ACs (and a fresh clone / CI) see the committed VCS mode
  **Location**: Phase 2 §2; Current State Analysis ("working-copy vs committed mode"); Phase 3 §2 (real-tree assertion)
  `os.access(..., os.X_OK)` reads the working copy; the invariant is about the committed `100755`/`100644`. CI is git-on-Linux, local is jj-on-macOS, and the sibling `tasks/test/helpers.py:20-27` precedent explicitly guards against exec-bit-lossy / bit-synthesising filesystems — the new guard does not. Nothing verifies the corrected modes were actually *committed*.

#### Minor

- 🔵 **Correctness / Code Quality**: Stale-entry check keys on `is_file()`, not `shell_sources()` membership
  **Location**: Phase 2 §2 (stale-entry loop)
  A library that exists on disk but drops out of `shell_sources()` (gitignored, relocated under `workspaces/`, or loses its `.sh` extension) passes the existence check yet is never mode-checked — one direction of the invariant is silently dropped for that file. Make the stale check assert membership in `sources`, not mere existence.

- 🔵 **Correctness**: Phase 1 assumes the seed libraries are already `644` without acting on them
  **Location**: Phase 1 §3 (re-run the audit before freezing)
  `accelerator-scaffold.sh` and `doc-type-inference.sh` are asserted to land at `644` but appear only in `SHELL_LIBRARIES`, not the 23-file chmod set — their correctness is contingent on an unverified premise. If either is `755` today, Phase 1 finishes without correcting it and Phase 3 fails. Have the audit stat *every* list member and add any `755` library to the `chmod -x` set.

- 🔵 **Test Coverage**: The extensionless `accelerator-visualiser` entrypoint has no unit test
  **Location**: Phase 2 §2/§3
  The plan flags it must remain 755 and "not be miscounted", but no test exercises the no-`.sh`-extension path. A refactor of `shell_sources()` or path handling could drop/misclassify it without a red test.

- 🔵 **Test Coverage**: AC5's real-tree assertion is keyed to a concrete Playwright path and is brittle
  **Location**: Phase 3 §2 (AC5 spot-check)
  Asserting `reinstall_chrome_stable_linux.sh` is absent couples the test to a third-party file that may move on a dependency bump. Prefer the stable invariant "no enumerated path contains a `node_modules` segment", or keep the exclusion at the `shell_sources()` unit level (synthetic tree).

- 🔵 **Architecture**: The invariant is broader than the single mechanism that strictly requires `+x`, and isn't framed as a convention
  **Location**: Desired End State; Architecture Insights
  Readers may assume every `+x` is functionally required. State explicitly (plan + README) that the rule codifies a uniform-755 *convention* for entrypoints, so it can be re-evaluated if 0106's mechanics change.

- 🔵 **Architecture**: Path-string manifest couples the guard to the tree layout; rename guidance is missing
  **Location**: Phase 2 §1; Decisions item 2
  A renamed/moved library manifests as a stale entry plus a newly-unlisted entrypoint at an unrelated time. The docs cover "add a new library" but not "rename/move/remove a library" — add that case.

- 🔵 **Documentation**: The stale-list-entry failure mode is enforced but not documented for contributors
  **Location**: Phase 2 §4; Desired End State
  A contributor removing/renaming a library hits a check failure whose remedy (prune `SHELL_LIBRARIES`) isn't anticipated by the documented mechanism. Extend the §4 "must be added" sentence to also cover removal/rename.

- 🔵 **Documentation**: `SHELL_LIBRARIES` source comment points at the whole README, not the anchor
  **Location**: Phase 2 §1 (the comment ends "See tasks/README.md.")
  This codebase cites specific sections/line ranges. Reference the named subsection so the pointer resolves directly.

- 🔵 **Standards**: `import os` must land in alphabetical stdlib order or ruff `I001` fails
  **Location**: Phase 2 §2 ("Add `import os` at the top of the module")
  Inserting `os` after `shlex` trips ruff (`select = ["ALL"]`), which Phase 2's own AC requires clean. Specify `os` before `shlex`.

- 🔵 **Standards**: The new constant, comment, and offender f-strings must respect the hand-duplicated 80-column floor
  **Location**: Phase 2 §1/§2
  The longest `SHELL_LIBRARIES` paths and the multi-line `Exit(...)` message are E501 risks; call out the width constraint for the new block.

- 🔵 **Standards / Portability**: Relationship to `tasks/test/helpers.py`'s exec-bit-lossy-filesystem `EXCLUDED_HELPER_NAMES` safeguard is unstated
  **Location**: Phase 2 §2
  The sibling discovery code pairs the `os.access` read with a name-level belt-and-braces for filesystems that synthesise exec bits. Confirm the membership-based approach is the intended, consistent stance and note the assumption.

- 🔵 **Portability**: The `rel.split("/")` fixture check depends on `shell_sources()` emitting POSIX separators
  **Location**: Phase 2 §2
  Sound for the macOS/Linux targets, but an undocumented coupling. Add a one-line comment (or test) noting the dependency, mirroring `_keep()`'s existing POSIX-path contract.

#### Suggestions

- 🔵 **Usability**: Emit a copy-pasteable `chmod` command per offender
  **Location**: Phase 2 §2 (guard message); Phase 3 manual-verification AC
  The message states the target mode ("should be 0755") but not the literal command. Phase 3's AC claims it "states the required `chmod`" — only partially true. Emit `chmod +x <path>` / `chmod -x <path>` (and "remove from `SHELL_LIBRARIES` or restore the file" for stale entries).

- 🔵 **Usability / Documentation**: Teach the *two-part* classification rule and the dual-use counter-example in the docs
  **Location**: Phase 2 §4
  The §4 content teaches "sourced → library", the naive heuristic the research flags as unsafe. State the rule as "on the list iff sourced AND never invoked by path" with `jira-fields.sh` as the worked counter-example.

- 🔵 **Usability**: Surface the fail-safe default as the contributor's mental model
  **Location**: Desired End State; Phase 2 §4
  Lead with "new `.sh` files are entrypoints (`chmod +x` and commit) unless they are sourced-only libraries", so newcomers know which side they get for free.

- 🔵 **Usability**: Point the CI failure back to the discriminator docs
  **Location**: Phase 3 Success Criteria; Phase 2 §2
  Append a one-line pointer to the `Exit` message ("If you believe a flagged file is mis-classified, see the library-list rules in tasks/README.md").

- 🔵 **Documentation**: Pin the CLAUDE.md pointer to the "Conventions and gotchas" shell bullet
  **Location**: Phase 2 §4 (line 398)
  Place it beside "Shell has no autofixer" rather than in the descriptive Architecture paragraph, where contributors scan for actionable rules.

- 🔵 **Code Quality**: Hoist the `test-fixtures` magic string to a named constant
  **Location**: Phase 2 §2
  `_FIXTURE_SEGMENT = "test-fixtures"` referenced by both guard and test gives the exemption one source of truth.

- 🔵 **Code Quality**: Disambiguate the three `test-helpers.sh` entries by full path everywhere
  **Location**: Phase 2 §1; tests/docs
  Three files share the basename; always refer to them by repo-relative path to avoid wrong-file copy/paste errors.

- 🔵 **Correctness**: Optionally skip/label non-existent enumerated paths for message clarity
  **Location**: Phase 2 §2
  A file deleted between the walk and `os.access` is reported as "entrypoint missing +x" — a benign single-threaded TOCTOU, message clarity only.

### Strengths

- ✅ **Single enumeration authority**: the guard reuses `shell_sources()` rather than introducing a second walk, inheriting `node_modules/`/`workspaces/`/`target/`/gitignore exclusions for free and guaranteeing format/lint/guard never disagree about scope.
- ✅ **Mirrors the established idiom**: fail-closed `_EMPTY_SCOPE`, single `invoke.Exit(message, code=1)` listing offenders, auto-registration via `Collection.from_module` with no `__init__.py` edit, no `lint:fix` entry (preserving "shell has no autofixer"), and a mise.toml task-block + depends-list wiring that matches `shellcheck`/`bashisms` exactly.
- ✅ **Clean, dependency-ordered phasing**: Phases 1 and 2 are order-independent and each leaves the tree green; Phase 3 is the single integration point. Every intermediate state is mergeable.
- ✅ **Unusually strong test strategy**: TDD red-then-green, an anti-vacuous-pass sentinel plus a non-empty-`shell_sources()` assertion, the fail-closed-on-empty-scope test, and a real-tree integration assertion that makes the wired guard the regression net for Phase 1's untestable mechanical `chmod`s.
- ✅ **Good fixture-handling decision**: a guard-local exemption rather than mutating `shell_sources()` or polluting the library-list, keeping `shell_sources()`'s single responsibility intact.
- ✅ **The invariant fails safe toward executable**: an unclassified new script defaults to "must be `+x`", the correct bias given entrypoints are the majority and a missing-`+x` failure is the louder one.
- ✅ **Correct documentation home**: `tasks/README.md` "Conventions (learn once)" sits beside the existing shell-lint norms, with CLAUDE.md as a one-line secondary pointer.
- ✅ **Anticipated subtleties**: the dual-use trap, the working-copy-vs-committed-mode note, and the escalation trigger are all surfaced rather than discovered late.

### Recommended Changes

1. **Reconcile the count and pin the membership as a Phase 2 gate** (addresses: the
   23/26/30 contradiction across 7 lenses). Drop hard counts from prose and the code
   comment; describe membership *by rule*, not cardinality. Elevate the AC1
   re-derivation from a deferred note to a blocking Phase 2 success criterion that
   records the final reconciled set, and have the integrity test assert exact
   sorted-set equality (which also catches duplicates), not mere existence.

2. **Make the library-list's load-bearing classification an automated regression
   net** (addresses: "unverified/untestable classification", "dual-use scripts have
   no test"). Add a test that cross-checks list membership against an independent
   signal — at minimum assert the three named dual-use scripts (`linkage-parser.sh`,
   `validate-source.sh`, `jira-fields.sh`) are absent from `SHELL_LIBRARIES` and
   executable on the real tree; ideally grep the SKILL.md/agents/hooks corpus so any
   list member that *is* path-invoked, or any off-list script only ever sourced,
   turns the build red.

3. **Tighten and document the `test-fixtures/` exemption** (addresses: "undocumented
   and over-broad"). Scope it to the known fixture root (prefix match on
   `skills/config/migrate/scripts/test-fixtures/`) or assert in a test that a fixture
   carrying `+x` is *deliberately* not flagged; and add it as an explicit third
   category (library / entrypoint / fixture) in the AC6 README content and the guard
   comment.

4. **Make the working-copy-vs-committed-mode stance explicit and tested** (addresses:
   the mode-divergence theme). Either read the committed VCS mode the ACs describe,
   or state in `tasks/README.md` and a test that the guard intentionally enforces
   working-copy mode, document the exec-bit-lossy-filesystem / `core.fileMode`
   assumptions, and add a manual-verification step confirming the corrected modes are
   *committed* (`jj diff` shows `100755`/`100644`).

5. **Fix the stale-entry / enumeration-domain mismatch** (addresses: the minor
   correctness/code-quality gap). Flag a `SHELL_LIBRARIES` entry as stale when it is
   not in `sources` (not merely when it is not a file on disk), so a library that
   leaves `shell_sources()` scope cannot silently escape mode enforcement.

6. **Harden Phase 1's audit to act on all list members** (addresses: "assumes seed
   libraries are already 644"). Stat every `SHELL_LIBRARIES` member during the
   pre-freeze audit and add any `755` library it finds to the `chmod -x` set, rather
   than assuming the seed libraries are already correct.

7. **Polish the guard's developer experience and standards compliance** (addresses
   the Usability suggestions and Standards minors). Emit copy-pasteable `chmod`
   commands per offender; teach the two-part classification rule (with the
   `jira-fields.sh` counter-example) and the fail-safe default in the docs; point the
   CI failure message at the discriminator docs; and note the `import os` ordering and
   80-column constraints so Phase 2's `build-system:check` AC stays green.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a single, clearly-stated invariant and enforces it
bidirectionally through the existing, well-factored shell-lint infrastructure.
Structurally sound: it reuses `shell_sources()` as the single enumeration authority,
mirrors the established fail-closed `@task` idiom, auto-registers via
`Collection.from_module`, and phases the work into three independently-mergeable,
dependency-ordered PRs. The main architectural tensions are around where the
source-of-truth for classification lives (a hand-maintained Python constant
duplicating knowledge the test runner derives differently), the dual-representation
of the invariant between the guard and `tasks/test/helpers.py`, and an internal
inconsistency in the plan's own library-list enumeration.

**Strengths**:
- Single enumeration authority: reuses `shell_sources()` rather than a second walk, inheriting exclusions for free.
- Mirrors the existing fail-closed `@task` idiom and auto-registers with no `__init__.py` edit.
- Clean phase decomposition with explicit dependency ordering; every intermediate state is mergeable.
- Fixture handling localised as a guard-local exemption, preserving `shell_sources()`'s single responsibility.
- Invariant fails safe toward executable (the correct bias).

**Findings**:
- 🟡 **major** (high) — *Phase 2 §1*: Library-list is a hand-maintained frozenset duplicating classification the test runner derives by exec bit + name-exclusion; the two representations of "sourced-only" can silently drift. Add a cross-consistency assertion.
- 🟡 **major** (high) — *Phase 2 §1*: The plan's own enumeration is internally inconsistent (prose 26, literal 30, research 26, note "trust the audit"). Make the AC1 re-derivation a blocking criterion with the final count pinned.
- 🔵 **minor** (high) — *Phase 2 §2 / Decisions 1*: The `test-fixtures` path-segment exemption embeds a second, divergent classification mechanism beside `SHELL_LIBRARIES`. Document it as a named third category and bound it with a test.
- 🔵 **minor** (medium) — *Current State / Architecture Insights*: The "consistency 755" invariant is broader than the single mechanism that strictly requires `+x`; frame it explicitly as a convention.
- 🔵 **minor** (medium) — *Phase 2 §1 / Decisions 2*: Path-string manifest couples the guard to tree layout; document the rename/move case (only "add" is covered).

### Code Quality

**Summary**: Well-structured for maintainability: the guard mirrors the established
fail-closed `shell_sources()` + single-`Exit` idiom, the phasing is clean and
independently mergeable, and the code sample is short, readable, and uses guard
clauses well. The main concerns are a latent maintenance hazard in the
dual-source-of-truth design (`SHELL_LIBRARIES` duplicates a hand-audited
classification the guard cannot self-verify), self-acknowledged inconsistencies
between prose and code (23 vs 26 vs 30; working-copy vs committed mode), and a couple
of small readability/robustness smells.

**Strengths**:
- The guard deliberately mirrors the `shellcheck`/`bashisms` idiom — one consistent pattern across the family.
- Short, flat guard body with early-`continue` guard clauses and a single offender list.
- Genuinely decoupled phasing; small, reviewable PR blast radius.
- Testability designed in (patchable `shell_sources`/`repo_root` seams, anti-vacuous-pass sentinel).
- Offender messages designed to be actionable (file + direction).

**Findings**:
- 🟡 **major** (high) — *Phase 2 §1*: `SHELL_LIBRARIES` is a hand-maintained classification the guard can only half-verify; the load-bearing "sourced AND never path-invoked" half stays manual, so the invariant silently degrades as new scripts are added. Automate the cross-check.
- 🟡 **major** (high) — *Phase 2 §1*: Internal count inconsistencies (23/26/30) will confuse the implementer and future readers. Pick one authoritative number derived from the code block, or drop hard counts.
- 🔵 **minor** (medium) — *Phase 2 §2*: Stale-entry check (`is_file()`) and the main loop (`for rel in sources`) compute membership independently; a library outside `shell_sources()` scope passes the existence check yet is never mode-checked. Assert membership in `sources`.
- 🔵 **minor** (medium) — *Phase 2 §2*: Magic-string exemption `"test-fixtures" in rel.split("/")` is primitive-obsession; hoist to a named constant shared by guard and test.
- 🔵 **suggestion** (medium) — *Phase 2 §1/§3*: Three files literally named `test-helpers.sh`; always refer by full path to avoid wrong-file mistakes.
- 🔵 **suggestion** (low) — *Phase 2 §4 / Current State*: Working-copy-vs-committed gap is documented but not guarded; phrase the offender message/README to remind that the bit must be *committed*.

### Test Coverage

**Summary**: An unusually strong testing strategy — mandated TDD, the fail-closed
idiom, an anti-vacuous-pass sentinel, and a real-tree integration assertion that
turns the guard into the regression net for Phase 1's mechanical chmods. The gaps are
concentrated in the library-list itself: the integrity test verifies only that listed
paths exist, leaving the load-bearing "sourced AND never path-invoked" classification
and list completeness as manual-only audits — exactly the dimension the research flags
as the guard's only soundness condition. A few specific risk paths (dual-use scripts,
the extensionless extra, working-copy-vs-committed mode) lack automated tests.

**Strengths**:
- TDD explicitly sequenced; the seven named unit tests map cleanly onto the guard's branches.
- Anti-vacuous-pass sentinel + non-empty `shell_sources()` assertion address the silent-pass risk.
- Fail-closed-on-empty-scope test preserves the existing `_EMPTY_SCOPE` invariant.
- Phase 3's real-tree assertion makes the wired guard the regression test for the untestable chmods.
- Reuses proven, low-flake seams (`mocker.patch.object`, `monkeypatch` of `repo_root`, `tmp_path` + `p.chmod`).

**Findings**:
- 🔴 **major** (high) — *Phase 2 §3*: The only library-list coverage is an existence check; the load-bearing classification half and list completeness are untested, so an omitted library ships executable with a green guard. Cross-check membership against an independent signal.
- 🟡 **major** (medium) — *Phase 2/3 Testing Strategy*: The three dual-use scripts — the central correctness trap — have no test pinning them off-list/executable. A wrong list addition passes CI while breaking a production entrypoint.
- 🔵 **minor** (medium) — *Phase 2 §1*: No test asserts list size/membership; a copy/paste slip in the manual re-derivation ships silently. Resolve 26-vs-30 and assert `len(SHELL_LIBRARIES)`.
- 🔵 **minor** (medium) — *Key Discoveries / Phase 3 §2*: The real-tree assertion checks the working tree, not that modes were committed; a staged-but-uncommitted chmod passes locally, fails a fresh CI checkout.
- 🔵 **minor** (low) — *Phase 2 §2/§3*: The extensionless `accelerator-visualiser` off-list entrypoint has no unit test. Add a synthetic test mirroring `test_sources.py`'s extensionless-CLI assertion.
- 🔵 **suggestion** (low) — *Phase 3 §1*: AC5's real-tree assertion keyed to a Playwright path is brittle across dependency bumps; assert "no enumerated path contains a `node_modules` segment" or keep it at the `shell_sources()` unit level.

### Correctness

**Summary**: The core invariant is logically sound and the guard's three-direction
offender detection is well-formed, fail-closed on empty scope, and reuses the verified
`shell_sources()` enumeration. The main risks are not in the control flow but in the
correctness of its data: `SHELL_LIBRARIES` is internally inconsistent and
acknowledged as not-yet-verified against the live tree (the jira/linear directories
contain many helper-looking scripts whose classification was not individually
checked). Two subtle logic gaps: the stale-entry check keys on filesystem existence
rather than `shell_sources()` membership, and the `test-fixtures` exemption is
bidirectional (broader than its one-directional rationale).

**Strengths**:
- Fail-closed on empty `shell_sources()`; anti-vacuous-pass sentinel + non-empty real-root assertion close the silent-no-op hole.
- The three violation branches partition the non-fixture space exhaustively.
- Correctly identifies and accepts the working-copy-vs-committed-mode gap rather than making a false claim.
- The dual-use analysis correctly validates that "sourced AND zero path-invocations", not "sourced", keeps the three entrypoints off the list.

**Findings**:
- 🔴 **major** (high) — *Phase 2 §1*: The literal has 30 entries, the comment says 26, the note says 30 citing research's 26; the set is the guard's sole source of truth and is unverified. A missing real library yields false offenders; a wrongly-listed entrypoint yields a silent false pass. Make AC1 re-derivation a hard precondition and assert exact membership.
- 🔵 **minor** (high) — *Phase 2 §2 (stale-entry loop)*: Stale check keys on `is_file()` while enforcement iterates `sources`; a library still on disk but out of `shell_sources()` scope is never mode-checked. Flag staleness on `not in sources`.
- 🔵 **minor** (medium) — *Phase 2 §2 (exemption)*: `"test-fixtures" in rel.split("/")` skips both directions for any path with that segment anywhere; rationale justifies only one direction. Scope to the known fixture root or document/test the bidirectional intent.
- 🔵 **minor** (medium) — *Phase 1 §3*: `accelerator-scaffold.sh` / `doc-type-inference.sh` are asserted to land at 644 but nothing in Phase 1 acts on them; correctness is contingent on an unverified premise. Stat every list member and chmod any 755 library.
- 🔵 **suggestion** (high) — *Phase 2 §2*: `os.access` on a path deleted between walk and access returns False → reported as "entrypoint missing +x" (benign single-threaded TOCTOU); optionally skip/label missing paths.

### Usability

**Summary**: From a developer-experience standpoint this plan is unusually strong: it
carves out documentation (runner-vs-helper example, the add-a-new-library rule), makes
the guard fail-closed, and asserts the offender message must be actionable. The gaps:
the offender message stops one step short of copy-pasteable, the fail-safe default ("a
new unclassified script must be executable") is never stated as the contributor's
mental model, and the two-part classification rule a contributor needs lives mostly in
plan/research prose rather than the doc the guard error points them to.

**Strengths**:
- Guard fails closed and names every offender; an AC requires the message to be actionable — DX as a first-class criterion.
- §4 mandates a concrete runner-vs-helper discriminating example.
- The message pre-empts the "why didn't `mise run fix` correct this" confusion ("no autofixer").
- Anticipates the working-copy-vs-committed foot-gun ("commit the chmod").
- The `SHELL_LIBRARIES` comment states the invariant and maintenance obligation at the point of edit — good progressive disclosure.

**Findings**:
- 🔵 **minor** (high) — *Phase 2 §2 / Phase 3*: The message states the target mode but not the literal command; Phase 3's AC over-claims it "states the required `chmod`". Emit copy-pasteable `chmod +x/-x <path>` per offender.
- 🔵 **minor** (medium) — *Phase 2 §4 / Current State*: The docs teach "sourced → library", the naive heuristic the research flags as unsafe; a dual-use script would be wrongly listed. Add the two-part rule and the `jira-fields.sh` counter-example.
- 🔵 **minor** (medium) — *Desired End State / Phase 2*: The fail-safe default is never surfaced; lead with "new `.sh` files are entrypoints unless sourced-only".
- 🔵 **minor** (high) — *Phase 3 / Migration Notes*: No guidance ties a CI guard failure back to the discriminator docs; append a one-line pointer to the `Exit` message.
- 🔵 **suggestion** (high) — *Phase 2 §1*: The 30-vs-"26" mismatch is handled in prose but invites contributor doubt; drop hard counts from the docs.

### Documentation

**Summary**: The AC6 requirements (a `tasks/README.md` subsection plus a CLAUDE.md
pointer) are well-targeted at the right discoverability home and the prescribed
content covers the real contributor questions. However, the plan carries an internal
count inconsistency (26 vs 30) that would propagate into the shipped source comment,
and the non-obvious `test-fixtures/` exemption is documented nowhere a contributor
would encounter it — leaving a real gap for anyone who adds a fixture and is surprised
by the guard's silence.

**Strengths**:
- AC6 targets the correct home (`tasks/README.md` "Conventions (learn once)").
- Prescribed README content answers the real questions (purpose, add-a-new-library, runner-vs-helper, commit-the-chmod).
- CLAUDE.md correctly designated as a one-line secondary pointer, not a duplicate.
- The `SHELL_LIBRARIES` source comment explains the "why" and cross-references the README.

**Findings**:
- 🔴 **major** (high) — *Phase 2 §1*: `SHELL_LIBRARIES` count is described three inconsistent ways (26 header, 30 literal, 30-citing-26 note); the research says 26 yet enumerates differently. A reader cannot trust any cardinality. Pick one authoritative count or drop counts from prose.
- 🟡 **major** (high) — *Phase 2 §4*: The `test-fixtures/` exemption is documented nowhere the contributor meets it (not the README content, not the source comment). Add a one-line item explaining fixtures are exempt in both directions because they are bash-run.
- 🔵 **minor** (medium) — *Phase 2 §1*: The source comment ends "See tasks/README.md." — point to the named subsection, not a 45-line file.
- 🔵 **minor** (medium) — *Phase 2 §4 / Desired End State*: The stale-list-entry failure mode is enforced but undocumented; extend the "must be added" sentence to cover removal/rename.
- 🔵 **suggestion** (low) — *Phase 2 §4*: The CLAUDE.md pointer location is unspecified; attach it to the "Conventions and gotchas" shell bullet near "Shell has no autofixer".

### Standards

**Summary**: The plan adheres closely to the project's conventions for the Python
invoke toolchain: it mirrors the fail-closed `_EMPTY_SCOPE` idiom, the single `Exit`
raise, `shell_sources()`/`repo_root()` reuse, auto-registration with no `__init__.py`
edit, the no-shell-autofixer rule, and the mise.toml task-block + depends-list wiring.
A few convention-level inconsistencies remain in the proposed code: a stale "26" count
contradicting the 30-entry literal, an unflagged `import os` ordering risk, the
80-column floor for the new block, and an unstated relationship to the existing
exec-bit-lossy-filesystem safeguard.

**Strengths**:
- Reuses the mandated `shell_sources()` rather than per-task globbing.
- Mirrors the fail-closed `_EMPTY_SCOPE` idiom exactly.
- Single `invoke.Exit(message, code=1)` naming every offender.
- Correctly relies on `Collection.from_module` auto-registration (no `__init__.py` edit).
- Preserves "shell has no autofixer" (nothing added to `lint:fix`).
- mise.toml wiring precisely matches the existing `shellcheck`/`bashisms` pattern.
- Honours importlib test mode and relaxed-ruff-in-tests (`SLF001`).

**Findings**:
- 🔵 **minor** (high) — *Phase 2 §1*: Stale "26" count contradicts the 30-entry literal in both prose and the to-be-shipped code comment. Describe membership by rule, not cardinality.
- 🔵 **minor** (high) — *Phase 2 §2*: Guard uses `Exit`/`os` but only `import os` is mentioned; inserting `os` after `shlex` trips ruff `I001`. Specify alphabetical stdlib order.
- 🔵 **minor** (medium) — *Phase 2 §2*: The guard relies solely on `os.access` with no equivalent of `tasks/test/helpers.py`'s `EXCLUDED_HELPER_NAMES` exec-bit-lossy-filesystem safeguard; confirm the membership-based stance is intended.
- 🔵 **minor** (medium) — *Phase 2/3*: `exec_bits` (Python) vs `exec-bits` (mise) is the first multi-word shell-lint task; confirm invoke's default underscore→hyphen normalisation is in effect.
- 🔵 **minor** (medium) — *Phase 2 §1*: The frozenset literal, comment, and offender f-strings must respect the hand-duplicated 80-column floor or ruff E501 fails the AC.

### Portability

**Summary**: The plan reuses the VCS-agnostic `shell_sources()` enumeration and
follows the established `os.access(p, os.X_OK)` precedent, inheriting correct
git-vs-jj and Linux-vs-macOS behaviour for free. The two genuine concerns are
environment-dependent: the guard reads working-copy modes on filesystems where the
exec bit may not survive (checkout settings, exec-bit-lossy/synthesised filesystems,
the local-jj-on-macOS vs committed-VCS-on-Linux split), turning what the existing
`run_shell_suites` precedent treats as a silent shrink into a hard build failure; and
`rel.split("/")` assumes POSIX separators, which holds only because `shell_sources()`
guarantees POSIX-relative output. Both are appropriate for the macOS+Linux-only
target matrix but deserve sharper treatment than a one-line note.

**Strengths**:
- Reuses `shell_sources()`, built to behave identically under git checkouts (CI) and jj workspaces (local) — no `git ls-files` blind spot.
- Follows the existing `os.access(p, os.X_OK)` idiom — one consistent cross-platform mechanism.
- The `rel.split("/")` check is consistent with `_keep()`'s own and relies on the same POSIX-relative contract.
- AC5 confirms vendored `node_modules/` trees are excluded portably via the shared gitignore-honouring walk.

**Findings**:
- 🔴 **major** (high) — *Phase 2 §2 / Current State*: The guard reads working-copy exec bits but the ACs specify committed VCS mode; CI is git-on-Linux, local is jj-on-macOS, and the sibling precedent explicitly defends against exec-bit-lossy/bit-synthesising filesystems while the new guard does not. On a synthesising filesystem it passes vacuously; on a lossy one it hard-fails on files correct in VCS. Make the enforced invariant explicit (read committed mode, or document the working-copy stance + assumptions).
- 🔵 **minor** (medium) — *Phase 2 §2*: The `test-fixtures` exemption depends on `shell_sources()` emitting POSIX separators — sound for the targets but an undocumented coupling. Add a one-line comment/test, mirroring `_keep()`'s POSIX-path contract.

## Re-Review (Pass 2) — 2026-06-20T18:32:39+00:00

**Verdict:** COMMENT

The revision is a clear success: **all five original major findings are
resolved**, and the plan now carries an exact-set integrity test, a dual-use
regression net, copy-pasteable offender messages, a documented working-copy-mode
stance, an expanded six-point §4 documentation spec, and a hardened Phase 1 audit.
The re-review (all 8 lenses, fresh) found **one new major** (an ergonomic gap in
how the commit reminder renders) plus a cluster of worthwhile minors — none
blocking. With 1 major (below the REVISE threshold of 3) and no critical findings,
the verdict drops from REVISE to COMMENT: the plan is acceptable as-is, and the
remaining items are polish.

### Previously Identified Issues

- 🟡 **Architecture / Code Quality / Correctness / Documentation / Standards / Usability / Test Coverage**: `SHELL_LIBRARIES` count contradiction (23/26/30) — **Resolved**. §1 now defines membership "by rule, not by count" and forbids anchoring prose/comments to a cardinality; the exact set-equality test pins the literal. Documentation lens: "fully neutralised"; Standards lens: "resolved".
- 🟡 **Architecture / Code Quality / Test Coverage / Correctness**: Library-list correctness unverified/untestable — **Resolved (residual minor)**. The exact-set, each-member-enumerated, and dual-use real-tree tests convert the highest-risk slices into automated checks. Correctness notes a residual minor: the tests prove the literal matches the *audit*, not that the audit's manual judgement is itself correct — inherent to a manual classification, now downgraded.
- 🟡 **Test Coverage**: Dual-use scripts had no regression test — **Resolved**. The three-script absent-from-list-and-executable assertion is now present and called out as a strength.
- 🟡 **Documentation / Correctness / Architecture / Code Quality**: `test-fixtures/` exemption undocumented and over-broad — **Resolved**. Now a named `_FIXTURE_SEGMENT` constant, documented as a third category in §4(e), and bounded by `test_fixture_exemption_scope`. Architecture suggests (minor) anchoring to the fixture-root prefix rather than a bare segment match — a refinement, not the original concern.
- 🟡 **Portability / Test Coverage / Correctness / Code Quality / Usability**: Working-copy vs committed mode — **Resolved**. The deliberate working-copy-mode stance is documented in §2 (matrix, assumptions, no name-level fallback), §4(f), and the strengthened Phase 1 committed-mode verification. Portability lens confirms the prior finding "resolved".

### New Issues Introduced

- 🟡 **major** (Usability) — *Phase 2 §2 / §4(f)*: The copy-paste fix can appear to fail under the working-copy gap. The per-offender line `chmod +x <path>  # entrypoint must be 0755` does not itself carry the "and commit it" nudge — that lives only in the shared preamble, which a developer scanning a long offender list may skip, producing the trust-eroding "I ran the exact command and CI still fails". Fix: fold the commit reminder into the per-offender comment, or ensure the preamble renders above the list (it does — but make the reminder unmissable).
- 🔵 **suggestion** (Code Quality) — *Phase 2 §2*: `os.access(p, os.X_OK)` returns True for any bit when the process runs as root, conflating the exec-bit check with ownership/ACL semantics. A library at `0644` could read as executable on a root CI runner. GitHub Actions runs as a non-root `runner` user so this is latent, but worth a precise `stat().st_mode & 0o111` check or a comment confirming the guard step never runs as root.
- 🔵 **minor** (Test Coverage) — *Phase 2 §3*: `test_flags_stale_library_entry` can pass vacuously — with a synthetic `shell_sources` list, most real `SHELL_LIBRARIES` entries are un-enumerated, so the stale branch fires incidentally. The test must patch `SHELL_LIBRARIES` (e.g. `frozenset({"scripts/gone.sh"})`) and assert the offender names that specific path.
- 🔵 **minor** (Test Coverage) — *Phase 2 §3*: The synthetic-tree tests must write each mocked source path to disk under the patched `repo_root` at the intended `0644`/`0755` mode — otherwise `os.access` on a non-existent path returns False and every off-list path falsely reads as "missing +x", passing for the wrong reason.
- 🔵 **minor** (Test Coverage / Correctness) — *Phase 2 §3*: The fixture-scope test should include a near-miss (`scripts/test-fixtures-x.sh`, a `test-fixturesX/` dir) asserting those are NOT exempted — the real failure mode of segment matching.
- 🔵 **minor** (Correctness) — *Phase 2 §2*: The stale-entry loop does not apply the `_FIXTURE_SEGMENT` exemption (benign — no library is a fixture). Add a cheap assertion that `SHELL_LIBRARIES` contains no `test-fixtures/` segment, or a comment noting the deliberate omission.
- 🔵 **minor/suggestion** (Usability / Documentation) — *§4*: Document the guard's offender message in §4 (close the failure→docs loop); give §4(b) the concrete grep recipe for "never invoked by path"; note that adding/removing a library also requires updating the integrity test's expected set; make the error string name the README *subsection*, not the bare file; land §4 as a single named subsection (e.g. "### Executable-bit invariant").
- 🔵 **suggestion** (Architecture) — *Phase 2*: As 0107 lands on the same module, consider moving `SHELL_LIBRARIES` + `exec_bits` to a dedicated `tasks/lint/exec_bits.py` sibling to preserve cohesion.
- 🔵 **suggestion** (Standards) — *Phase 2 §1 / Phase 3 §1*: Keep the `SHELL_LIBRARIES` literal in sorted order (matches the sorted integrity assertion); the four-segment `lint:scripts:exec-bits:check` task name is a slight shape departure from the three-segment siblings (acceptable).
- 🔵 **suggestion** (Portability) — *Phase 2 §2*: A mass "library must be 0644" report can indicate an exec-bit-synthesising filesystem rather than real drift; consider noting this in the message/README, mirroring `run_shell_suites`. Optionally normalise `shell_sources()` to POSIX explicitly (`.as_posix()`) so the `split("/")` rests on an enforced contract.

### Assessment

The plan is in good shape and ready for implementation. The one new major is a
message-ergonomics refinement, not a design or correctness defect, and is a
two-line change. The most valuable remaining minors are the two **test-construction
guards** (patch `SHELL_LIBRARIES` in the stale-entry test; write synthetic files to
disk at the intended mode) — without them, two of the guard's own tests could pass
vacuously, which matters for a change whose entire value is a regression net. I'd
recommend folding in the new major and those two test-construction notes before
implementation; the rest are optional polish that can be addressed during
implementation or deferred.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-06-20T18:46:15+00:00

**Verdict:** APPROVE

A targeted confirmation pass (Usability, Test Coverage, Documentation) verified
the Pass-2 edits and the subsequent fixes. The Pass-2 major (offender-message
commit reminder) is resolved, both test-construction vacuity guards are adequately
specified, and the documentation cross-reference loop is now closed (the
`SHELL_LIBRARIES` source comment, the guard's offender message, and the
`### Executable-bit invariant` README heading all cite the same anchor). The
remaining items are optional polish recorded above (root/`os.access(X_OK)` caveat,
sorted literal ordering, four-segment task name, the 0107 module-split). The plan
is sound, fully specified, and approved for implementation.

### Previously Identified Issues

- 🟡 **Usability**: Copy-paste fix could appear to fail under the working-copy gap — **Resolved**. Each offender line now carries an inline `# … then commit` reminder; the `chmod` stays paste-safe; `test_offender_message_is_copy_pasteable` pins the reminder against regression.
- 🔵 **Test Coverage**: `test_flags_stale_library_entry` vacuity — **Resolved**. Now patches `SHELL_LIBRARIES` to a one-element synthetic set, keeps other files compliant, and asserts the specific offender name.
- 🔵 **Test Coverage**: synthetic files must exist on disk at the intended mode — **Resolved**. Mandated as a construction guard, with the extensionless and fixture-scope tests pinned to it.
- 🔵 **Documentation**: source comment pointed at the bare README file — **Resolved**. Now names the `"Executable-bit invariant"` subsection.

### New Issues Introduced

_None._

### Assessment

Approved. All actionable findings across the three review passes are resolved or
explicitly accepted as optional polish. The plan is ready for `/implement-plan`.

---
*Verdict set to APPROVE by reviewer (Toby Clemson).*
