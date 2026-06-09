---
date: "2026-05-16T11:00:00+01:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-15-0059-gh-pr-edit-projects-classic-deprecation"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility]
review_pass: 5
status: complete
id: "2026-05-15-0059-gh-pr-edit-projects-classic-deprecation-review-1"
title: "2026-05-15-0059-gh-pr-edit-projects-classic-deprecation-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-16T11:00:00+01:00"
last_updated_by: Toby Clemson
---

## Plan Review: gh pr edit → REST PATCH Migration in describe-pr

**Verdict:** REVISE

The plan is structurally strong — strict TDD ordering, faithful adherence to
the per-skill `scripts/` + `test-*-scripts.sh` pattern, good preservation of
the existing frontmatter-strip and cleanup responsibilities, and correct
identification of `gh pr view --json baseRepository` as the cross-fork-safe
resolver. However, multiple lenses converged on the same error-handling
deficiencies: the Phase 2 resolver swallows `gh`'s actual stderr behind a
canned remediation hint that may be wrong, and the Phase 3 pipeline cannot
distinguish jq-encoding failures from PATCH failures. The PATH-stub design
also has a likely-blocking bug (unconditional `cat` on every gh invocation),
and the task wiring sits under the wrong mise family (`test:unit:*` rather
than `test:integration:*`). These are addressable with focused edits to the
plan; the overall approach is sound.

### Cross-Cutting Themes

- **Resolver swallows stderr** (flagged by: code-quality, correctness,
  usability, compatibility — 4 lenses) — Phase 2's `2>/dev/null` plus
  unconditional `gh repo set-default` hint means auth, network, 404, and
  rate-limit failures all surface as misleading remediation while the real
  `gh` error is discarded.
- **Pipeline error attribution** (flagged by: code-quality, correctness,
  usability, compatibility — 4 lenses) — `jq | gh api` with one generic
  "PATCH failed" message misblames jq failures as REST failures and risks
  losing `gh api`'s actual stderr (HTTP 422 details, rate-limit text).
- **Stub dispatch fragility** (flagged by: code-quality, correctness,
  compatibility — 3 lenses) — `case "$1 $2" in "api"*|"api ")` has a
  redundant alternative, no default arm, and unconditional stdin read that
  will block `gh pr view` calls.
- **Test 5 assertion contradiction** (flagged by: code-quality,
  test-coverage, correctness — 3 lenses) — "byte-for-byte (modulo
  JSON-acceptable whitespace)" is self-contradictory; should use the same
  round-trip pattern as test 6.
- **jq null interpolation** (flagged by: code-quality, correctness,
  compatibility — 3 lenses) — `"\(.owner.login)/\(.name)"` produces literal
  `null/null` instead of erroring when fields are absent.

### Tradeoff Analysis

- **Resolver extraction vs scope discipline** (architecture vs scope) —
  Architecture suggests extracting the cross-fork-safe resolver so
  `review-pr` and `respond-to-pr` can adopt it; the plan deliberately
  defers this as out-of-scope. Defensible, but Architecture's point that
  burying the resolver inside `pr-update-body.sh` makes future extraction
  *more* costly is well taken. At minimum, capture a follow-up work item.
- **Long absolute invocation path vs grep-friendliness** (usability vs
  maintainability) — Usability flags
  `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh`
  as the longest command in the skill, but it's also the most discoverable
  and matches the explicit-path convention. Probably leave as-is.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Code Quality / Correctness / Usability / Compatibility**: Resolver swallows gh's stderr behind a canned remediation hint
  **Location**: Phase 2, resolver code block (the `2>/dev/null` redirect)
  `gh pr view ... 2>/dev/null` discards the actual error; the script
  unconditionally prints the `gh repo set-default` hint regardless of
  whether the failure was auth, 404, rate limit, network, or schema. Users
  debugging non-default-remote failures see misleading remediation and no
  real diagnostic. Test 9 will pass mechanically but the script's behaviour
  is wrong for every other failure mode.

- 🟡 **Code Quality / Correctness / Usability / Compatibility**: Pipeline error attribution conflates jq and gh failures
  **Location**: Phase 3, `jq -Rs '{body: .}' <"$body_file" | gh api ...`
  Under `pipefail`, jq failures and gh failures both yield the same
  "PATCH failed for repos/..." message. jq encoder failures get
  misattributed as REST failures, and `gh api`'s real stderr (HTTP 422
  validation detail, rate-limit text) risks being buried.

- 🟡 **Test Coverage**: fake-gh.sh unconditionally reads stdin; resolver tests will block
  **Location**: Phase 1 §2, fake-gh.sh dispatcher
  The stub does `cat >> "$GH_STDIN_LOG"` for every invocation. The
  production `gh pr view` calls don't pipe stdin, so the stub inherits the
  harness's stdin and blocks (or reads unintended bytes) waiting on EOF.
  Tests 2, 3, 9 will hang or behave non-deterministically. Gate stdin
  capture on `api` subcommand only.

- 🟡 **Architecture**: Cross-fork-safe resolver duplicated, perpetuating latent bug
  **Location**: What We're NOT Doing (cross-fork resolver scoping)
  `review-pr` and `respond-to-pr` keep the cross-fork-unsafe `gh repo
  view` path; the correct resolver is buried inside `pr-update-body.sh`,
  making future extraction *more* costly. At minimum a follow-up work
  item should be tracked explicitly rather than deferred parenthetically.

- 🟡 **Test Coverage**: Empty body edge case listed in strategy but not in enumerated tests
  **Location**: Phase 1 §2 / Testing Strategy section
  Strategy mentions "empty body, single-line, multi-line" but only
  multi-line (test 5), metacharacters (6), unicode (7) are enumerated.
  `jq -Rs '{body: .}'` on empty input should emit `{"body":""}`; a
  regression that swaps for `null` would not be caught.

- 🟡 **Correctness / Code Quality / Compatibility**: jq interpolation produces `null/null` on missing fields
  **Location**: Phase 2, `--jq '"\(.baseRepository.owner.login)/\(.baseRepository.name)"'`
  If gh returns absent/null fields (deleted upstream, schema shift),
  `base_repo="null/null"`, the PATCH targets `repos/null/null/pulls/N`,
  and the failure surfaces as a confusing 404. Stubs always return
  well-formed JSON so this is not caught by the harness.

- 🟡 **Standards**: test:unit:github misclassifies shell harness
  **Location**: Phase 1 §3
  Every skill shell harness in the codebase is wired under
  `test:integration:*` (decisions, config, visualiser, binary-acquisition)
  via `run_shell_suites` in `tasks/test/integration.py`. `test:unit:*`
  is reserved for cargo/Vitest/pytest. New task should be
  `test:integration:github` and added to `[tasks."test:integration"].depends`.

- 🟡 **Standards**: allowed-tools frontmatter not updated for new helper
  **Location**: Phase 4
  `describe-pr/SKILL.md` declares `allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`. The plan adds an
  invocation of `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh`
  but does not extend allowed-tools. Other skills (create-adr, visualise)
  enumerate their script dirs. Without this, the helper invocation will
  prompt for permission or be blocked.

- 🟡 **Compatibility**: jq is an undeclared external dependency
  **Location**: Phase 1 §1 / Phase 3 §1
  `jq` is not in `mise.toml` `[tools]`. Existing scripts that depend on
  jq (`hooks/vcs-detect.sh`, `launch-server.sh:91`, `jira-common.sh:225`)
  defensively `command -v jq` with a clear remediation. Without that,
  jq-less users get a cryptic SIGPIPE-coloured failure.

#### Minor

- 🔵 **Architecture**: No retry/idempotency strategy or differentiated failure modes
  **Location**: What We're NOT Doing (item 6) / Phase 3
  All gh failures collapse to one exit code; no seam for a future retry
  wrapper. Either explicitly document the trade-off or introduce
  distinct exit codes per failure class.

- 🔵 **Architecture**: First github-scripts helper sets new conventions without ADR
  **Location**: What We're NOT Doing (item 3)
  PATH-stubbed gh, `jq -Rs '{body: .}'`, and `gh pr view --json
  baseRepository` are new conventions. ADR-0010 doesn't cover them. At
  minimum, capture in a script header comment or README.

- 🔵 **Architecture**: Frontmatter-strip responsibility split between SKILL.md and helper
  **Location**: Phase 4
  Helper's `<body-file>` signature implicitly assumes
  frontmatter-stripped content; not visible at call site. Document the
  precondition in the helper's header comment.

- 🔵 **Code Quality / Correctness**: Suspicious fake-gh case-statement glob
  **Location**: Phase 1 §2
  `case "$1 $2" in "api"*|"api ")` — `"api"*` already covers `"api "`;
  no default arm means unknown verbs silently exit 0. Use exact-match
  arms and add a `*) echo "...unexpected: $*" >&2; exit 99 ;;` default.

- 🔵 **Code Quality / Correctness**: Test 5 byte-for-byte vs whitespace-modulo contradiction
  **Location**: Phase 1 §2 test 5
  "byte-for-byte (modulo JSON-acceptable whitespace)" is
  self-contradictory. Use the same round-trip pattern as test 6
  (`jq -r .body` of recorded stdin equals input file).

- 🔵 **Code Quality**: 7+ env vars per test is repetitive
  **Location**: Phase 1 §2 stub contract
  Define a `setup_gh_stub <tmpdir>` helper mirroring
  `make_fake_visualiser`/`reap_visualiser_fakes` in
  `test-launch-server.sh`.

- 🔵 **Test Coverage**: Test 11 doesn't verify PATCH was called
  **Location**: Phase 1 §2 test 11
  Only asserts exit 0; a regression that resolves and exits without
  calling PATCH passes. Also assert argv log contains exactly one
  `api` entry.

- 🔵 **Test Coverage**: Substring argv assertions risk mock-shaped passes
  **Location**: Phase 1 §2 test 4 and others
  Substring-based assertions on `gh api` argv pass for invocation orders
  that real gh might reject. Assert full argv line with `assert_eq` for
  the canonical PATCH call shape.

- 🔵 **Test Coverage**: No automated regression test prevents reintroduction of `gh pr edit`
  **Location**: Phase 4 success criteria
  Add a single guard-test in the harness:
  `assert empty (grep -r 'gh pr edit' skills/)`.

- 🔵 **Test Coverage**: Wrong-arg-count test only covers zero args
  **Location**: Phase 1 §2 test 13
  Add 1-arg and 3-arg boundary cases (most likely off-by-one mutations).

- 🔵 **Correctness**: Test 3 unsatisfiable in Phase 2; plan defers split
  **Location**: Phase 2 success criteria
  "test 3 will still FAIL if it depends on PATCH being called — split
  if necessary" — actually split it in Phase 1 (3a resolver, 3b PATCH).

- 🔵 **Standards**: mise depends formatting; lowercase `usage:`; exit-code split
  **Location**: Phase 1 §3 / Phase 2 §1
  - If repositioning under `test:integration`, follow multi-line
    `depends = [...]` block at `mise.toml:114-122`.
  - Capitalised `Usage:` is the dominant convention; test 13's
    assertion substring needs to match.
  - exit 1 for validation matches `adr-*`/`work-item-*`; if keeping
    exit 2, document an `Exit codes:` block in the script header.

- 🔵 **Standards**: Inline fake-gh stub deviates from sourced-helper pattern
  **Location**: Phase 1 §2
  `test-launch-server.sh` and jira tests factor fake binaries via
  `test-helpers.sh`. Consider co-locating a small
  `skills/github/describe-pr/scripts/test-helpers.sh`.

- 🔵 **Usability / Standards**: Remediation hint in SKILL.md keyed to swallowed stderr
  **Location**: Phase 4 sub-step 5 prose
  Model is told to look for "no default remote repository" in stderr,
  but the script substitutes its own message. Coordinate: either
  propagate gh's stderr or rewrite the SKILL.md prose to be more
  general.

#### Suggestions

- 🔵 **Code Quality**: Shellcheck not wired into test:unit:github task
  Either prepend a shellcheck invocation in the harness or extend the
  mise task to lint before running.

- 🔵 **Test Coverage**: No test for pipefail propagation on jq failure
  Add a test with an unreadable body file (mode 000) and assert
  non-zero exit.

- 🔵 **Test Coverage**: Very long body not tested
  Smoke-test ~100 KiB body to validate the stdin pipeline scales.

- 🔵 **Usability**: Long absolute invocation path in SKILL.md
  Tradeoff with grep-friendliness; probably leave as-is.

- 🔵 **Usability**: Phase 5 cross-fork fallback bounds residual risk
  Add a note articulating what the unit test doesn't cover
  (real `gh api` against a remote upstream from a forked checkout).

- 🔵 **Usability**: Phase 1 stub message could reference the plan path
  `pr-update-body.sh: incomplete (see meta/plans/2026-05-15-0059-...)`.

- 🔵 **Standards**: Script naming `pr-update-body.sh`
  Form is consistent with `work-item-update-tags.sh`; document the
  intended `pr-<verb>-<field>.sh` pattern for future github helpers.

### Strengths

- ✅ Strict TDD with explicit RED-state verification in Phase 1 locks the
  contract before implementation pressure can erode it.
- ✅ Test 6's round-trip approach (`jq -r .body` of recorded stdin vs
  input file) is the right shape for verifying JSON encoding without
  coupling to jq's formatting choices.
- ✅ Helper's minimal positional-arg interface (`<pr-number> <body-file>`)
  is intuitive and matches how SKILL.md invokes it.
- ✅ Phased TDD progression (RED → resolver GREEN → PATCH GREEN → SKILL.md
  swap → manual verification) gives clean correctness checkpoints.
- ✅ Each test case maps to an acceptance criterion or technical-note
  convention, making coverage traceability easy to audit.
- ✅ Script directory layout, harness naming, and shared `test-helpers.sh`
  sourcing all follow established codebase conventions.
- ✅ Uses `gh pr view --json baseRepository` (cross-fork-safe) rather than
  the cross-fork-unsafe `gh repo view` used in sibling skills, with
  explicit justification.
- ✅ Targets stable REST contract `PATCH /repos/{owner}/{repo}/pulls/{N}`
  with documented `body` field; pinned `gh` 2.89.0 comfortably supports
  `--jq`, `--method PATCH`, `--input -`.
- ✅ Preserves existing frontmatter-strip (lines 119-129) and unconditional
  cleanup (line 131) byte-identically, respecting AC4 and AC5.
- ✅ Reuses existing remediation phrasing at lines 54-55 for the new
  resolver-failure path, preserving in-skill wording consistency.
- ✅ Uses `set -euo pipefail`, quoted variables, exit-code differentiation
  — matches codebase bash hygiene conventions.

### Recommended Changes

1. **Stop swallowing the resolver's stderr** (addresses: "Resolver
   swallows gh's stderr" — 4 lenses)
   In Phase 2, capture `gh pr view`'s stderr to a tempfile or variable
   and replay it on failure. Make the `gh repo set-default` hint
   conditional on grep-matching the captured stderr ("no default remote
   repository"). Coordinate with Phase 4's SKILL.md prose so the model's
   error-pattern instructions still apply.

2. **Disambiguate the jq | gh api pipeline failure path** (addresses:
   "Pipeline error attribution" — 4 lenses)
   In Phase 3, either (a) encode body to a tempfile first
   (`jq -Rs '{body: .}' <"$body_file" >"$tmp_payload"`, check exit,
   then `gh api ... --input "$tmp_payload"`), or (b) inspect
   `${PIPESTATUS[@]}` and emit a stage-specific message. Add the new
   tempfile to the cleanup trap.

3. **Fix fake-gh.sh stdin handling and dispatch** (addresses: "fake-gh
   blocks on pr view", "stub dispatch fragility")
   Gate `cat >> "$GH_STDIN_LOG"` on the `api` subcommand only. Replace
   `case "$1 $2" in "api"*|"api ")` with exact-match arms and add a
   default arm that fails loudly (`*) echo "fake-gh: unexpected: $*"
   >&2; exit 99 ;;`).

4. **Guard the jq null-interpolation case** (addresses: "jq null/null")
   Either use a jq expression that errors on missing fields
   (`.baseRepository | "\(.owner.login // error("missing owner"))/\(.name
   // error("missing name"))"`) or post-validate `$base_repo` against
   `^[^/]+/[^/]+$` before the PATCH.

5. **Reposition mise task under `test:integration:*`** (addresses:
   "test:unit:github misclassifies")
   Rename to `test:integration:github`, back it with a Python invoke
   task calling `run_shell_suites(context, 'skills/github')` to match
   the decisions/config/visualiser pattern, and add to
   `[tasks."test:integration"].depends`.

6. **Extend SKILL.md allowed-tools frontmatter** (addresses:
   "allowed-tools not updated")
   In Phase 4, add
   `- Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/*)`
   to the allowed-tools list.

7. **Add jq dependency preflight** (addresses: "jq undeclared")
   In Phase 2, add a `command -v jq >/dev/null || { echo
   'pr-update-body.sh: jq is required' >&2; exit 2; }` preflight,
   matching `jira-common.sh:225` and `launch-server.sh:91`. Optionally
   pin jq in `mise.toml` `[tools]`.

8. **Add an empty-body test case** (addresses: "empty body missing")
   Insert a test asserting an empty body file produces
   `{"body":""}` on stdin and exit 0. Also consider a
   single-line-no-trailing-newline case.

9. **Split test 3 into resolver-argv and PATCH-URL halves** (addresses:
   "test 3 unsatisfiable in Phase 2")
   Make 3a (resolver argv) testable in Phase 2 and 3b (PATCH URL) in
   Phase 3; update both phases' success criteria.

10. **Rewrite test 5 as a round-trip assertion** (addresses: "test 5
    contradictory")
    Use the same `jq -r .body` round-trip pattern as test 6 for
    tests 5, 7, and the new empty-body case.

11. **Add a regression-guard for `gh pr edit`** (addresses: "no
    regression test for reintroduction")
    Append a harness assertion:
    `assert empty output (grep -r 'gh pr edit' skills/)`.

12. **Track the cross-fork resolver follow-up explicitly** (addresses:
    "resolver duplicated")
    Either create the follow-up work item now, or add a one-line
    pointer in `describe-pr/SKILL.md` indicating that
    `pr-update-body.sh` encapsulates the cross-fork-safe resolver and
    that `review-pr`/`respond-to-pr` have a known latent bug tracked
    in `<id>`.

13. **Minor tightenings** (addresses: minor findings)
    - Capitalised `Usage:` in stub and matching test 13 assertion
    - Decide and document the exit-code convention (1 vs 2 for usage)
    - Add boundary tests for 1 and 3 args in test 13
    - Strengthen test 11 to assert PATCH was called (argv log
      contains one `api` entry)
    - Use `assert_eq` on full argv lines for the canonical PATCH
      invocation rather than substring matches
    - Document jq preconditions and frontmatter precondition in the
      helper's header comment

---

*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Plan is architecturally sound: isolates failure in a single
testable helper behind a stable positional-arg contract, follows
established per-skill scripts/test harness pattern, explicitly preserves
existing responsibilities. Main concerns are evolutionary fitness:
deferred resolver extraction, no retry/idempotency seam, no ADR capturing
new conventions.

**Strengths**: clear module boundary; pattern consistency; functional
core / imperative shell separation; precedent acknowledgement; phased
TDD locks interface shape.

**Findings**: 1 major (resolver duplicated), 4 minor (resilience seam,
ADR omission, mock dispatch fragility, frontmatter responsibility split).

### Code Quality

**Summary**: Well-structured plan, follows TDD discipline and codebase
patterns. Main concerns are error-message specificity, pipe-failure
attribution, and several maintainability rough edges.

**Strengths**: set -euo pipefail; quoted expansions; usage/runtime exit
distinction; canonical harness shape; narrow scope; idiomatic jq -Rs.

**Findings**: 2 major (resolver swallows stderr, pipe failure), 5 minor
(case-pattern, null guard, env-var boilerplate, test 5 wording,
allowed-tools), 1 suggestion (shellcheck wiring).

### Test Coverage

**Summary**: Disciplined TDD with 13 enumerated cases mapped to ACs;
round-trip assertion in test 6 is strong. Missing edge cases (empty
body, very long body), likely-blocking stdin bug in PATH stub, and
inconsistent assertion specificity.

**Strengths**: explicit RED verification; round-trip approach; AC
traceability; resolver/PATCH split; cross-fork URL covered.

**Findings**: 2 major (stub stdin blocks, empty body missing), 5 minor
(test 11 doesn't verify PATCH, test 5 contradictory, substring assertions,
no regression guard, arg-count boundary), 2 suggestions (pipefail test,
long body).

### Correctness

**Summary**: Generally correct tool usage. Concrete logic concerns:
resolver's blanket 2>/dev/null swallows the underlying error and emits
remediation that may be wrong; jq expression produces literal null/null
for absent fields; stub dispatch and several assertions have ambiguity.

**Strengths**: preserves frontmatter strip and cleanup; cross-fork-safe
resolver; idiomatic jq -Rs; phased checkpoints; correct REST endpoint
shape.

**Findings**: 2 major (resolver stderr swallow, jq null/null), 4 minor
(stub dispatch redundancy + no default, pipeline error attribution,
test 3 unsatisfiable in Phase 2, test 5 over-specified).

### Standards

**Summary**: Generally follows conventions, but task family choice is
wrong, allowed-tools not extended, and a few stylistic departures.

**Strengths**: scripts dir layout; harness file naming; shared
test-helpers sourcing; canonical script style; preserves SKILL.md
frontmatter strip byte-identically; reuses remediation phrasing;
shellcheck enforcement.

**Findings**: 2 major (test:unit vs test:integration; allowed-tools),
3 minor (mise formatting, lowercase usage, exit-code split, inline stub
vs sourced helper), 1 suggestion (script naming pattern).

### Usability

**Summary**: Well-scoped helper interface, good remediation-phrase
consistency, but error-experience design is hurt by stderr-swallowing
resolver and ambiguous pipeline failure. Phase 5 cross-fork fallback is
acceptable as a documented limitation.

**Strengths**: minimal positional argv; exit-code differentiation;
remediation reuse; remediation tested explicitly; named failure modes;
stub fails closed.

**Findings**: 1 major (resolver swallows stderr), 2 minor (pipeline
ambiguity, SKILL.md prose coupled to swallowed stderr), 3 suggestions
(long invocation path, AC3 residual risk, stub message).

### Compatibility

**Summary**: Stable REST contract usage; gh version safely supports
required flags; bash conventions match codebase. Main gap is undeclared
jq dependency with no preflight. A couple of smaller stability concerns.

**Strengths**: stable PATCH endpoint; gh 2.89.0 well above minimum; bash
shebang + pipefail; remediation phrasing reuse; jq -Rs portable encoding.

**Findings**: 1 major (jq undeclared), 4 minor (null/null guard, stub
case pattern, resolver stderr swallow, pipeline error attribution).

---

## Re-Review (Pass 2) — 2026-05-15

**Verdict:** REVISE

The revision substantially improves the plan: 6 of 7 lenses had **all**
prior findings either resolved or knowingly deferred, the cross-cutting
themes around stderr swallowing and pipeline attribution are now
addressed, and the scope expansion to migrate `review-pr` and
`respond-to-pr` to the shared resolver eliminates the latent
cross-fork bug across all three skills. However, the re-review surfaces
three new majors — all of them test-design bugs introduced by the
revision rather than production-code issues — that would prevent the
GREEN phases from actually verifying what the tests claim. The most
load-bearing is a structural mismatch between the fake-gh stdin capture
and the new `gh api --input <file>` posting pattern: the round-trip
body assertions cannot inspect what they think they are inspecting. Two
adjacent issues compound it (a resolver argv assertion that references
a `--jq` flag the implementation no longer uses; tempfile-cleanup
assertions that can't find the tempfile without controlling `TMPDIR`).
Fixing these three is straightforward but mandatory before the plan
can drive a meaningful TDD cycle.

### Previously Identified Issues

**Architecture (5 findings)**:
- 🟡 Cross-fork-safe resolver duplicated — **Resolved** (shared helper at `skills/github/scripts/pr-base-repo.sh`; Phases 5 and 6 migrate the siblings)
- 🔵 No retry/idempotency strategy — **Partially resolved** (stage-specific exit codes added; retry remains explicitly out of scope as documented)
- 🔵 First helper sets new conventions without ADR — **Still present** (conventions captured as header comments; deliberate trade-off)
- 🔵 Fake-gh dispatch coupling — **Resolved** (exact-match arms + failing default)
- 🔵 Frontmatter-strip responsibility split — **Resolved** (precondition documented in helper header)

**Code Quality (8 findings)**:
- 🟡 Resolver swallows stderr — **Resolved** (capture-to-tempfile + replay + conditional hint)
- 🟡 Pipeline error attribution — **Resolved** (encode-to-tempfile with distinct exit codes)
- 🔵 Case-statement glob — **Resolved** (exact-match + `${2:-}` guard)
- 🔵 Null owner/name guard — **Resolved** (`// ""` + `-z` check)
- 🔵 Stub env-var boilerplate — **Resolved** (`setup_gh_stub` helper)
- 🔵 Test 5 contradiction — **Resolved** (uniform round-trip)
- 🔵 allowed-tools not updated — **Resolved** (extended in Phases 4, 5, 6)
- 🔵 Shellcheck not wired — **Still present** (acknowledged suggestion)

**Test Coverage (9 findings)**:
- 🟡 fake-gh stdin blocks — **Resolved** (gated on `api`)
- 🟡 Empty body missing — **Resolved** (test 9 + no-trailing-newline test 13)
- 🔵 Test 11 doesn't verify PATCH called — **Resolved** (test 20 asserts argv-line counts)
- 🔵 Test 5 contradiction — **Resolved**
- 🔵 Substring argv assertions — **Resolved** (full-line match on test 7)
- 🔵 No regression guard for `gh pr edit` — **Resolved** (test 22)
- 🔵 Arg-count boundary — **Resolved** (tests 2/3/4 cover 0/1/3)
- 🔵 No pipefail test — **Resolved** (test 18, with caveats — see new finding)
- 🔵 Long body — **Still present** (acknowledged low-priority suggestion)

**Correctness (6 findings)**: **All Resolved**

**Standards (7 findings)**: **All Resolved**

**Usability (6 findings)**: **All Resolved** (long invocation path kept as tradeoff; cross-fork residual-risk note added; stub message includes plan reference)

**Compatibility (5 findings)**: **All Resolved**

### New Issues Introduced

#### Major

- 🟡 **Test Coverage**: Round-trip body assertions cannot work — fake-gh captures stdin but production uses `--input <file>`
  **Location**: Phase 1 §5 tests 9-13 + §3 fake-gh
  Tests 9-13 rely on `GH_STDIN_LOG` being populated. But Phase 3's
  implementation passes `--input "$payload_file"` to `gh api`, which
  causes real `gh` to read from the file, not stdin. The fake-gh
  still reads its own (empty) stdin into the log. Every round-trip
  assertion silently passes against an empty string (test 9 by
  coincidence) or fails (tests 10-13).
  **Fix**: Have fake-gh parse `--input <path>` and copy the file's
  contents into `$GH_STDIN_LOG`, OR record the `--input` path
  separately and have tests read that file directly.

- 🟡 **Correctness / Test Coverage**: Test 5 (resolver argv) asserts a shape the implementation does not produce
  **Location**: Phase 1 §4 test 5 vs Phase 2 implementation
  Test 5 specifies full-line `pr view 119 --json baseRepository --jq ...`,
  but Phase 2's `gh pr view "$pr_number" --json baseRepository` has no
  `--jq` flag (jq runs locally on the captured payload). The full-line
  match fails against the actual invocation.
  **Fix**: Update test 5 to assert exactly
  `pr view 119 --json baseRepository`.

- 🟡 **Correctness**: mktemp cleanup tests cannot find the encoder tempfile without controlling `TMPDIR`
  **Location**: Phase 1 §5 tests 15-16; Phase 3 implementation
  Bare `mktemp` creates files under `$TMPDIR` (or `/tmp`), not the
  test's tempdir. The harness can't reliably enumerate the encoder
  tempfile without scoping `TMPDIR` to a fresh empty dir before
  invoking the helper.
  **Fix**: Have `setup_gh_stub` export `TMPDIR="$tmpdir/mktemp"` and
  mkdir it before each test; cleanup assertion becomes
  `[ -z "$(ls -A "$TMPDIR")" ]` after the helper exits.

#### Minor

- 🔵 **Test Coverage**: No permanent regression guard against re-introducing `gh repo view --json owner,name`
  Test 22 guards `gh pr edit` but not the cross-fork-unsafe resolver.
  **Fix**: Add test 23 — `grep -rn 'gh repo view --json owner,name' skills/github/` returns no matches.

- 🔵 **Test Coverage**: Test 10 (jq preflight) — prepending an empty bin-dir doesn't hide jq
  `command -v jq` walks the full PATH and finds the system jq later.
  **Fix**: Set `PATH="$tmpdir/bin"` (full replacement, not prepend) for this test.

- 🔵 **Test Coverage**: Mode-000 encode-failure test silently no-ops when run as root
  Test 18 will pass for the wrong reason in root-based CI.
  **Fix**: Use a directory as the body file path, or inject a fake `jq` that exits 1.

- 🔵 **Test Coverage**: Null guards don't cover missing fields or non-JSON responses
  Tests 8/9 cover `{"login":null}`, not missing-field or HTML-error-page cases.
  **Fix**: Add cases for missing `baseRepository` entirely and non-JSON stdout from `gh pr view`.

- 🔵 **Test Coverage**: Tempfile cleanup assertions don't specify mechanism
  Tests 15/16 say "assert tempfile removed" without specifying how the harness identifies it.
  **Fix**: Tie to the TMPDIR fix above.

- 🔵 **Test Coverage**: Sibling migrations rely solely on grep
  Phases 5/6 success criteria don't assert allowed-tools globs match helper paths.
  **Fix**: Add allowed-tools grep assertions in the harness, OR explicitly acknowledge the gap in Testing Strategy.

- 🔵 **Correctness**: Regression-guard grep uses path relative to harness CWD
  Test 22's `grep -r 'gh pr edit' skills/` may not resolve depending on `run_shell_suites` invocation directory.
  **Fix**: Use the `_REPO_ROOT` computed in `test-helpers.sh` and grep `"$_REPO_ROOT/skills/"`.

- 🔵 **Correctness**: Encode-failure error attribution drifts when shell redirect fails before jq runs
  Mode-000 file fails at `<"$body_file"`, not at jq.
  **Fix**: Either add `[ -r "$body_file" ]` check, or rephrase the error message to cover both subcases.

- 🔵 **Architecture**: Resolver exit codes collapsed when propagated through `pr-update-body.sh`
  All non-zero resolver exits become `pr-update-body.sh` exit 3.
  **Fix**: Either preserve the resolver's exit code verbatim or document why collapsing is intentional.

- 🔵 **Architecture**: Aggregate `test:integration` red between Phases 1 and 3 if landed separately
  Adding to `depends` before Phase 3 turns the helper GREEN breaks the aggregate.
  **Fix**: Gate the `depends` addition to Phase 3, OR land Phases 1-3 as a single squashed commit.

- 🔵 **Architecture**: Base-repo resolved twice on chained `/review-pr` → describe-pr flow
  No way to feed an already-resolved owner/name into `pr-update-body.sh`.
  **Fix**: Out of scope — capture as a follow-up note.

- 🔵 **Standards**: New mise task uses `uv run --project tasks invoke ...` form
  Every existing `test:integration:*` entry uses bare `invoke test.integration.<name>`.
  **Fix**: Change the run line to `run = "invoke test.integration.github"`.

- 🔵 **Standards**: `_REPO_ROOT` underscore-prefixed variable in new test-helpers.sh
  Diverges from codebase convention (`SCRIPT_DIR` / `PLUGIN_ROOT`) and from the user's stated preference against underscore-prefixed names.
  **Fix**: Rename to `PLUGIN_ROOT` (no underscore); consider not chaining the source at all — match the visualiser pattern where consumers source both helpers independently.

- 🔵 **Usability**: Exit-code taxonomy not exposed to the model in SKILL.md prose
  Model sees stderr verbatim but not the 1/2/3/4 stage semantics.
  **Fix**: Add a one-line aside in sub-step 5 — "The helper uses distinct exit codes per failure stage; see the script header for the taxonomy."

#### Suggestions (carried)

- Stale `test-fake-gh.sh` reference in Implementation Approach (file was folded into `test-helpers.sh`)
- Pin `jq` to an explicit version in `mise.toml [tools]` for parity with other tools
- Resolver trap needs header note (must invoke as subprocess, not source)
- PATCH-stage stderr should be captured-and-replayed like the resolver's
- `pwd -P` in `script_dir` to follow symlinks
- Shellcheck should be wired into the mise task, not just spot-checked in success criteria
- Exit code 1 (encode) collides with generic shell failures — consider 10/11/12 range
- Suggest a low-cost cross-fork live test in Phase 7 rather than treating it as unreachable

### Assessment

The plan is close. The structural concept (shared resolver + describe-pr
helper + sibling migrations + TDD harness) is sound and the major
production-code issues from review 1 have all been addressed
substantively. What remains is a cluster of test-design bugs introduced
by the scope expansion — chiefly the fake-gh / `--input <file>` mismatch
that would invalidate the encoding round-trip assertions, the argv shape
mismatch on resolver test 5, and the mktemp cleanup tests' inability to
locate the tempfile. Each fix is small and local; together they would
let the GREEN phases actually verify what the plan claims.

Recommended next step: apply the three major fixes, the high-value
minors (especially regression-guard for `gh repo view`, the `uv run`
divergence, and the `_REPO_ROOT` rename), and a third review pass can
plausibly verify approval. The remaining suggestions are nice-to-haves
that can be triaged at edit time.

---

## Re-Review (Pass 3) — 2026-05-15

**Verdict:** REVISE

The pass-2 majors are all resolved: round-trip body assertions work
against the new fake-gh `--input` parser, test 5 matches the
implementation's actual argv, tempfile cleanup tests are valid with
the new TMPDIR scoping. The pass-2 minors are largely resolved
(mise task form, `_REPO_ROOT` rename, regression-guard CWD,
sub-process invocation contract, exit-code preservation, `pwd -P`,
PATCH stderr capture-replay). What remains is small, mostly
documentation drift and two test-design loose ends introduced by
the pass-3 edits themselves.

### Previously Identified Issues (Pass 2)

**Architecture (4)**: 3 Resolved, 1 Partially resolved
(area-level-directory documentation still narrative-only)

**Code Quality (3 suggestions)**: 2 Resolved (subprocess contract,
PATCH stderr replay); 1 Still present (shellcheck wiring, deliberate)

**Test Coverage (3 major + 7 minor/suggestion)**: All 3 majors
Resolved; minors Resolved with caveats (see new findings below).

**Correctness (2 major + 5 minor)**: All Resolved.

**Standards (2 minor + 1 suggestion)**: All Resolved.

**Usability (1 minor + 1 suggestion)**: 1 Resolved (taxonomy aside
in SKILL.md), 1 carried (long invocation path / recognisable shape).

**Compatibility (2 suggestions)**: 1 Resolved (jq pin), 1 deliberate
skip (mktemp template).

### New Issues Introduced (Pass 3)

#### Major

- 🟡 **Test Coverage**: Phase 3 GREEN criterion is unsatisfiable because tests 22 and 23 (regression guards) can't pass until Phases 4-6
  **Location**: Phase 3 success criteria vs. Phase 1 tests 22/23
  Tests 22/23 assert `gh pr edit` and `gh repo view --json owner,name`
  have no matches under `skills/`, but those edits only happen in
  Phase 4 (describe-pr) and Phases 5-6 (sibling skills). Phase 3's
  "all tests pass" and "`mise run test:integration:github` exits 0"
  criteria are mutually unsatisfiable with these tests in the harness.
  **Fix**: Either move tests 22/23 into a separate harness enabled
  only after Phase 6, or explicitly mark them expected-RED in Phase 3
  success criteria and adjust the aggregate check.

- 🟡 **Test Coverage / Correctness**: Fake-jq mechanism for test 18 would short-circuit the resolver
  **Location**: Phase 1 §5 test 18
  Test 18 injects a fake `jq` to force the encode-failure branch,
  but `pr-update-body.sh` runs the resolver first — which also calls
  `jq -r` for owner/name extraction. A naive fake-jq fails the
  resolver before the encoder ever runs, so test 18 silently
  exercises resolver propagation instead of encode failure.
  **Fix**: Specify the fake-jq behaviour explicitly: succeed on
  `jq -r .../...` (resolver pattern), fail on `jq -Rs '{body: .}'`
  (encoder pattern). Add `setup_fake_jq` as a named helper in
  `skills/github/scripts/test-helpers.sh`.

#### Cross-cutting Minor (flagged by 3 lenses: test-coverage, usability, architecture)

- 🔵 **Testing Strategy still references "resolver failure (exit 3)"**
  **Location**: Testing Strategy → Unit / Integration Tests bullet
  Phase 3's verbatim-propagation rule (resolver-1 → 1, resolver-2 →
  2) contradicts this stale bullet. Reviewers/implementers writing
  tests against the Testing Strategy will encode the wrong assertion.
  **Fix**: Replace `resolver failure (exit 3)` with `resolver failure
  (preserved verbatim: 1 = resolution, 2 = usage/missing jq)`.

#### Other Minors

- 🔵 **Correctness / Code Quality**: jq encode stderr silenced with `2>/dev/null`
  Pass-3 introduced `jq -Rs ... 2>/dev/null` in Phase 3, regressing
  the diagnostic-visibility principle the resolver and PATCH stages
  carefully implement. **Fix**: capture jq stderr to `encode_err`
  tempfile and replay on failure (mirror the resolver/PATCH pattern).

- 🔵 **Correctness**: `patch_err=$(mktemp)` sits outside the trap window
  If the second `mktemp` fails, `payload_file` leaks (no trap yet).
  **Fix**: install trap after the first mktemp and extend it, OR use
  a single `mktemp -d` with files inside.

- 🔵 **Code Quality**: Inline comment about `if ! cmd` semantics is inaccurate
  The comment claims `if ! cmd` "inverts $? in some bash semantics" —
  not true. The rewrite is correct; the justification is wrong.
  **Fix**: reword the comment to focus on explicit exit-code capture
  rather than alleged bash semantics.

- 🔵 **Test Coverage**: Test 14 readability assertion mechanism is implicit
  Test claims "file is readable at the time gh api is invoked" but
  the EXIT trap unlinks the file before assertions run. Mechanism is
  indirect (non-empty `$GH_STDIN_LOG`).
  **Fix**: spell out the indirect mechanism in the test description.

- 🔵 **Test Coverage**: Fake-gh `--input` fallback to stdin can hang/silently pass
  When `--input` path doesn't exist, fake falls back to `cat` on
  stdin. In CI with closed stdin, returns immediately, masking bugs.
  **Fix**: fail loudly (`exit 98 "input path not readable"`) when
  `--input <path>` is in argv but path is unreadable.

- 🔵 **Usability / Standards**: Exit-1 overload in SKILL.md aside isn't flagged
  `1=encode/resolver-resolution` conflates two distinct stages under
  one code. Stderr disambiguates, but the aside doesn't say so.
  **Fix**: extend aside to `1=encode (pr-update-body.sh) or
  resolver-resolution (pr-base-repo.sh); stderr prefix identifies
  which stage`.

- 🔵 **Standards**: SKILL.md taxonomy aside diverges from existing styles
  Other SKILL.md files use bullet lists with arrow notation. The
  inline parenthetical is terser but less scannable. Acceptable
  tradeoff; flagged for consistency.

- 🔵 **Architecture**: Area-level `skills/github/scripts/` convention
  still undocumented (no README); narrative-only.

#### Suggestions

- Fake-gh should fail loudly on edge cases (`--input` trailing with
  no value)
- `setup_fake_jq` should be added to `skills/github/scripts/test-helpers.sh`

### Assessment

Two real test-design issues (Phase 3 GREEN unsatisfiability and the
fake-jq short-circuit) plus the cross-cutting stale exit-3 reference
in Testing Strategy are the load-bearing items. Each fix is small.
The other minors are documentation tightening or hardening — none
block the TDD cycle if applied, but they would tip the balance toward
APPROVE on a fourth pass.

Recommended: apply the two majors + the cross-cutting stale-exit-3
fix, then a fourth pass should be APPROVE-track. The remaining
minors can be applied at the same time or deferred to edit-time
during implementation.

---

## Re-Review (Pass 4) — 2026-05-16

**Verdict:** REVISE

Both pass-3 majors are resolved (PHASE-gated regression guards;
`install_fake_jq` / `setup_fake_jq` with `-r` delegation). All other
pass-3 findings resolved cleanly. The pass-4 edits introduce two new
test-infrastructure majors — both straightforward plumbing gaps in
the test machinery the pass-4 edits depend on — plus a sprinkle of
minor robustness, precedent, and discoverability notes.

### Previously Identified Issues (Pass 3)

- **Architecture**: 2/2 Resolved (Testing Strategy exit-3 fix;
  area-level helper directory README added)
- **Code Quality**: 3/3 Resolved (comment accuracy, single
  `mktemp -d` + trap, fake-gh fail-loud on unreadable `--input`)
- **Test Coverage**: 5/5 Resolved (PHASE-gated guards, fake-jq
  spec, exit-3 wording, test 14 mechanism, stdin-fallback fail-loud)
- **Correctness**: 3/3 Resolved (encode stderr capture-replay,
  single-trap window, fake-jq delegation)
- **Standards**: 1/1 Resolved (SKILL.md arrow-bullet style)
- **Usability**: 1/1 Resolved + 1 carried (exit-3 wording fixed;
  exit-1 overload now explicitly disambiguated)
- **Compatibility**: All clear

### New Issues Introduced (Pass 4)

#### Major (2 — both test-infra plumbing)

- 🟡 **Test Coverage**: `skip_test` and `assert_grep_empty` helpers invoked but never defined
  **Location**: Phase 1 §5 tests 22/23 implementation sketch
  The phase-gating sketch calls `skip_test "test 22: deferred ..."`
  and `assert_grep_empty "$PLUGIN_ROOT/skills/" "gh pr edit"`, but
  neither exists in `scripts/test-helpers.sh` (which has `assert_eq`,
  `assert_contains`, `assert_exit_code`, `test_summary` but no skip
  mechanism), nor in the new `skills/github/scripts/test-helpers.sh`
  (which adds only the fake-gh/fake-jq factories). Phase 3's GREEN
  gate (`PHASE=3 mise run test:integration:github` exits 0) depends
  entirely on `skip_test` being a real helper that counts as PASS.
  **Fix**: Add concrete definitions to `scripts/test-helpers.sh`:
  - `skip_test <message>` — increments PASS counter, prints
    `  SKIP: <message>`
  - `assert_grep_empty <path> <pattern>` — runs `grep -r <pattern> <path>`,
    fails with offending lines if any match
  Spell out the bodies in Phase 1's Changes Required list.

- 🟡 **Test Coverage**: PHASE env-var threading from CLI through mise → invoke → shell harness is asserted but never wired
  **Location**: Phase 1 §7 + `tasks/test/integration.py` + `mise.toml`
  Every phase success criterion uses `PHASE=N mise run
  test:integration:github` but the wiring is implicit. mise.toml
  declares only `run = "invoke test.integration.github"`; the invoke
  task uses `run_shell_suites(context, 'skills/github')` whose
  `context.run` inherits env by default — but never asserted. A
  future tightening (`replace_env=True`, `pty=True`) silently breaks
  the entire phase-gating model.
  **Fix**: Either (a) explicitly declare PHASE in mise.toml's task
  `env` block with a `default(value="final")` filter, (b) explicitly
  pass `env={"PHASE": os.environ.get("PHASE", "final")}` in the
  invoke task, OR (c) add a meta-test that asserts `printenv PHASE`
  reaches the harness via the mise→invoke chain.

#### Minor

- 🔵 **Test Coverage**: fake-jq fallback assumes a real jq exists on `ORIG_PATH`
  If a CI container has no system jq (and mise hasn't yet installed
  the pinned 1.7.1), the resolver stage exits 127 misattributed.
  **Fix**: Preflight check in `setup_fake_jq` asserting jq is
  locatable on `ORIG_PATH` before proceeding.

- 🔵 **Correctness**: PHASE case-statement default arm enforces guards on unrecognised values
  Typos (`PAHSE=3`, `PHASE=1.5`) silently fall through to enforce,
  producing confusing REDs.
  **Fix**: Explicit case arms with an `*) echo "unknown PHASE";
  exit 2 ;;` catchall.

- 🔵 **Correctness**: Local jq parse errors in resolver leak raw stderr
  Phase 2's resolver doesn't capture jq stderr; non-JSON `gh pr view`
  output produces opaque parse errors. Test 12 anticipates this.
  **Fix**: Pre-validate `$payload` with `jq -e .` before extracting
  fields.

- 🔵 **Standards**: `skills/github/scripts/README.md` has no precedent in other skill areas
  No other `skills/<area>/scripts/` directory has a README. Either
  acknowledge as a deliberate new convention (with follow-up to
  backfill) or fold the placement rule into a header comment on
  `skills/github/scripts/test-helpers.sh`.

- 🔵 **Standards**: PHASE env-var convention has no codebase precedent
  No existing harness/mise task/invoke task reads any phase-style env
  var. Together with the new `skip_test` helper, two new conventions
  land at once without standards anchoring.
  **Fix**: Document the new pattern in Implementation Approach;
  optionally drop tests 22/23 from the harness and enforce them via
  a one-line `grep` precondition in the invoke task's `final` mode.

- 🔵 **Standards**: `ORIG_PATH` env var collides with shell convention
  `ORIG_PATH` is a generic name some users export in shell init to
  track pre-mise/pre-asdf PATH.
  **Fix**: Rename to `FAKE_JQ_REAL_PATH` or similar scoped name.

- 🔵 **Compatibility**: fake-jq self-skip via `BASH_SOURCE[0]` string-equality is fragile under macOS symlinks
  `/var/folders/...` is a symlink target of `/private/var/folders/...`.
  Non-canonical PATH entries vs realpath BASH_SOURCE could cause
  fake-jq to exec-loop.
  **Fix**: Use `[ "$candidate" -ef "${BASH_SOURCE[0]}" ]` (inode
  comparison) instead of string equality. Or capture the fake's
  install path at install time and inject into the heredoc.

#### Suggestions

- 🔵 **Code Quality**: `install_fake_jq`'s `BASH_SOURCE[0]` self-check is dead-defensive given the `setup_fake_jq` ORIG_PATH-snapshot invariant; either drop it (and document the invariant) or strengthen it (canonical comparison).

- 🔵 **Code Quality**: `setup_fake_jq` could capture a stale fake-jq dir into `ORIG_PATH` if called twice in the same shell. Document single-use contract or strip prior fake-jq dirs before snapshot.

- 🔵 **Code Quality**: `IFS=":"` PATH parsing yields empty entries for `::`/leading/trailing colons. Add `[ -n "$dir" ] || continue` or document the limitation.

- 🔵 **Test Coverage**: fake-gh exit codes 98 (unreadable `--input`) and 99 (unknown verb) are inline sentinels without a documented contract. Add a header-comment block listing reserved exit codes.

- 🔵 **Usability**: PHASE env-var threading is non-obvious; add a one-sentence note to Implementation Approach.

### Assessment

The plan is on the threshold of APPROVE. Two pass-4 majors are both
test-infrastructure plumbing gaps — small fixes (define `skip_test`
and `assert_grep_empty` in `scripts/test-helpers.sh`, and explicitly
wire PHASE through mise.toml's task env block). The remaining minors
are robustness, precedent, and discoverability concerns that
implementers can absorb at edit time without blocking the GREEN
gates.

After 4 passes the plan has gone from 11 majors (pass 1) to 3
(pass 2) to 2 (pass 3) to 2 (pass 4), each round resolving the
previous load-bearing issues completely. A fifth pass focusing on
the two test-infra fixes would plausibly clear to APPROVE.

---

## Re-Review (Pass 5) — 2026-05-16

**Verdict:** REVISE (asymptotic convergence — see assessment)

All pass-4 issues resolved. However, the pass-5 edits introduced a
new cluster of majors localised to the freshly-added test
infrastructure (`skip_test`, `assert_grep_empty`, `setup_fake_jq`,
PHASE wiring). The pattern is consistent across passes 2-5: each
round resolves the prior load-bearing issues but introduces a
smaller, more localised wave of issues in the code-newly-introduced-
by-the-fix.

### Previously Identified Issues (Pass 4)

- **Test Coverage**: 2/2 Resolved (skip_test/assert_grep_empty
  defined; PHASE wired explicitly)
- **Code Quality**: 3/3 Resolved (BASH_SOURCE → `-ef`; ORIG_PATH →
  FAKE_JQ_REAL_PATH; IFS empty-entry guard)
- **Correctness**: 4/4 Resolved (PHASE default; -ef; payload
  pre-validation; IFS guard)
- **Standards**: 3/3 Resolved (PHASE convention sanctioned by
  helper-library additions; ORIG_PATH rename; SKILL.md style)
- **Usability**: 1/1 Resolved (PHASE Implementation Approach note)
- **Compatibility**: 1/1 Resolved (BASH_SOURCE → `-ef`)
- **Architecture**: All clear

### New Issues Introduced (Pass 5)

#### Cross-cutting Major (flagged by 4 lenses)

- 🟡 **setup_fake_jq `return 1` under `set -e` aborts whole harness**
  (test-coverage, code-quality, correctness, usability)
  Test harnesses run under `set -euo pipefail`. Calling
  `setup_fake_jq` directly (per test 18 spec) when jq is missing
  will hard-abort the entire harness mid-suite, not just fail test
  18 — and `test_summary` never runs.
  **Fix**: Either gate the test with
  `if ! setup_fake_jq ...; then skip_test 'no jq'; return; fi`,
  OR have `setup_fake_jq` call `skip_test` internally and return 0.
  Document the caller idiom in the helper's docstring.

#### Cross-cutting Major (flagged by 3 lenses)

- 🟡 **`PASS_COUNT`/`FAIL_COUNT` counter mismatch with library
  `PASS`/`FAIL`** (test-coverage, code-quality, standards)
  The new helpers reference `PASS_COUNT`/`FAIL_COUNT` but the
  library uses `PASS`/`FAIL`. Plan has a hedge note but ships
  broken code as canonical.
  **Fix**: Update Section 0 to use `PASS=$((PASS + 1))` and
  `FAIL=$((FAIL + 1))` directly. Drop the hedge.

#### Other Majors

- 🟡 **`skip_test` inflates PASS — no SKIP counter** (test-coverage,
  standards)
  A green run with many SKIPs is indistinguishable from a green run
  with all PASSes. Phase-deferred guards become invisible.
  **Fix**: Add `SKIP=0` and `Skipped: N` line in `test_summary`;
  `skip_test` increments SKIP, not PASS.

- 🟡 **`assert_grep_empty` silently PASSES when path doesn't exist**
  (test-coverage)
  `grep -rn <pattern> <missing-path>` returns exit 2 (grep error);
  `2>/dev/null` swallows it; `matches` is empty → function reports
  PASS. A typo in `$PLUGIN_ROOT` permanently passes the regression
  guard.
  **Fix**: Preflight `[ -e "$path" ]` before the grep. Distinguish
  grep exit 1 (no match → PASS) from exit 2 (error → FAIL) by
  capturing `$?` explicitly.

- 🟡 **Test 23 (and test 22) self-match in harness file**
  (correctness)
  The pattern `gh repo view --json owner,name` appears as a literal
  string argument to `assert_grep_empty` inside the harness file at
  `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`.
  The recursive grep matches this very file, making the guard
  permanently RED in Phase 6/final. Same hazard for test 22's
  `gh pr edit` pattern.
  **Fix**: Scope the grep to SKILL.md files only:
  `assert_grep_empty <name> "$PLUGIN_ROOT/skills/" "gh pr edit" --include='*.md'`,
  or use `--exclude-dir=scripts`. The forbidden patterns only live
  in SKILL.md prose, so this is both safe and tighter.

#### Minor

- 🔵 **`assert_grep_empty` breaks the assert_* family signature**
  (standards)
  Every existing `assert_*` helper takes `test_name` as the first
  arg. New helper omits it, fragmenting output formatting.
  **Fix**: Reshape to `assert_grep_empty <test_name> <path> <pattern>`.

- 🔵 **mise Tera template syntax unverified**
  (compatibility, standards, usability)
  `env = { PHASE = "{{ env.PHASE | default(value=\"final\") }}" }`
  has no precedent in `mise.toml`. Canonical mise idiom is
  `get_env(name="PHASE", default="final")`.
  **Fix**: Switch to
  `env = { PHASE = "{{ get_env(name=\"PHASE\", default=\"final\") }}" }`
  OR add a Phase 1 verification step that asserts the template
  resolves correctly.

- 🔵 **`jq -e .` conflates valid-JSON-null with non-JSON** (correctness)
  `jq -e .` exits 1 on `null`/`false` (valid JSON, just degenerate),
  which the resolver misattributes as "non-JSON output".
  **Fix**: Use `jq empty <<<"$payload" 2>/dev/null` instead — exits
  0 on any valid JSON, non-zero only on parse failure.

- 🔵 **PHASE `*)` arm exits 2 mid-harness** (correctness)
  Default arm `exit 2` from inside the harness terminates remaining
  tests and skips `test_summary`.
  **Fix**: Move PHASE validation to a single guard at the top of
  the harness (validate once), OR use a `fail_test` helper that
  increments FAIL and continues.

- 🔵 **PHASE-conditional scaffolding has no decommissioning path**
  (architecture)
  Once Phase 6 lands and trunk is permanently `final`, the entire
  PHASE infrastructure becomes dead scaffolding with no removal step.
  **Fix**: Add a Phase 7 cleanup (or follow-up work-item) that
  removes the PHASE env block, case-statement scaffolding, and
  unconditionally enforces tests 22/23.

#### Suggestions

- Reserved exit codes 97/98/99 form an ad-hoc convention; consider
  consolidating into a single comment block (standards)
- `assert_grep_empty` could accept `--include`/`--exclude` for
  scope control (code-quality)
- mise task description should mention PHASE override (usability)
- Optional bash-version check at top of fake-jq script
  (compatibility)

### Assessment

**The plan has converged.** The structural design is sound; each
review pass since pass-2 has resolved its prior load-bearing issues
completely, but each set of fixes introduces a smaller wave of
issues localised to the newly-added code. Pass-5's majors are all
in the test infrastructure I just added — none touch the production
helpers (resolver and helper script implementations) or the SKILL.md
migrations, which are stable across the last three passes.

Trajectory: 11 → 3 → 2 → 2 → 5 majors. The pass-5 uptick is
specifically because I introduced ~30 lines of new helper code in
pass-5, and that code has bugs. Continuing this loop would likely
follow the same pattern: fix the 5 new helper bugs → next pass
finds 1-2 new issues in the fixes → fix those → and so on.

**Recommended path forward:**

1. Apply the 5 majors from pass 5 (all small, well-scoped fixes
   targeting `scripts/test-helpers.sh`, `setup_fake_jq`, and the
   regression-guard grep scoping). This brings the plan to a
   genuinely landing-ready state for the implementer.
2. Mark this review REVISE-done with the pass-5 fixes applied.
3. The remaining minors and suggestions can be triaged at
   implementation time — they're all small, observable, and the
   implementer is the right person to make the trade-off calls
   when they hit the actual code.

Calling another full pass-6 review would likely produce diminishing
returns. The plan's design is settled; what remains is fine
craftsmanship of the test infrastructure, which is better refined
during implementation than in plan review.

---

## Pass 5 Fixes Applied (no further review pass)

The user elected to apply the 5 pass-5 majors and call REVISE-done
rather than continue iterating.

Fixes applied:

1. **Counter names corrected** — `scripts/test-helpers.sh` Section 0
   now uses `PASS`/`FAIL`/`SKIP` (bare, no `_COUNT`) matching the
   existing library convention. Hedge note dropped.

2. **SKIP counter added** — third outcome counter introduced.
   `test_summary` updated to report `Skipped: N` alongside passes
   and failures. `skip_test` now increments SKIP rather than PASS.
   Green gate remains `FAIL=0`.

3. **`assert_grep_empty` hardened** — added `<test_name>` as first
   positional arg (matching family signature), path-existence
   preflight (returns FAIL on missing path), and explicit handling
   of grep exit codes 0 (FAIL with matches), 1 (PASS), and 2+
   (FAIL with grep-error diagnostic). Accepts trailing extra args
   passed through to grep (`--include`, `--exclude-dir`, etc.).

4. **Tests 22/23 self-match fixed** — both regression-guard greps
   now use `--include='*.md'` to scope to SKILL.md files only,
   eliminating the harness-file self-match. PHASE validation moved
   to a single early gate at the top of the harness so subsequent
   case statements compose cleanly with `test_summary`.

5. **`setup_fake_jq` caller idiom documented** — header comment
   now spells out the REQUIRED caller pattern
   (`if ! setup_fake_jq ...; then skip_test ...; return; fi`) so
   the missing-jq case becomes a skip rather than a hard harness
   abort under `set -e`. Test 18's spec updated to use this idiom.

Remaining minors and suggestions deferred to implementation time.

Final verdict: **REVISE-done**. The plan is ready for the
implementer.






