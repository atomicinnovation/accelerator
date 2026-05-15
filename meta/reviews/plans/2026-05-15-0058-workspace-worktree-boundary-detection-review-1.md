---
date: "2026-05-15T15:35:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-15-0058-workspace-worktree-boundary-detection.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, portability, compatibility]
review_pass: 3
status: accepted
---

## Plan Review: Workspace and Worktree Boundary Detection Implementation Plan

**Verdict:** REVISE

The plan is structurally strong: five well-scoped phases, strict TDD with red-then-green per phase, AC5 byte-identity captured up-front as a regression guard, helper decomposition that follows the established thin-orchestrator/sourced-library pattern, and explicit alignment with project conventions (unprefixed snake_case, no `set` flags in `*-common.sh`, `realpath` as a global rule). However, multiple lenses converge on two material behavioural gaps — **the AC4 cross-VCS nesting path silently drops the outer-VCS parent**, and **the colocated fixture builder likely fails because `git worktree add` refuses a non-empty target** — plus a cluster of robustness gaps (no graceful degradation when `jj`/`git` are missing, brittle golden snapshots, fragile AC9 substring assertions) that the plan should resolve before implementation.

### Cross-Cutting Themes

These were flagged by multiple lenses and deserve the most attention:

- **AC4 cross-VCS nesting silently drops the outer parent** (flagged by: architecture, correctness) — `classify_checkout` returns `jj-secondary` for a jj-in-git nest, so only the jj parent is emitted; the git parent (which AC4 requires) is never named in the boundary block. The plan's own AC4 test does not catch this because it only asserts the jj parent appears in CTX.
- **Colocated fixture builder will likely fail at build time** (flagged by: correctness, compatibility) — `make_colocated_secondary` runs `jj workspace add` first (populating the target with `.jj/`), then `git worktree add` (which refuses a non-empty path). Under `set -euo pipefail` the entire suite aborts before any AC3 assertion runs.
- **No graceful degradation when `jj` or `git` binaries are missing** (flagged by: architecture, code-quality, test-coverage) — the existing hook gracefully degrades on missing `jq`, but new helpers shell out to `jj`/`git` unconditionally. A user without `jj` installed inside a jj workspace receives no boundary block and no diagnostic.
- **AC9 placement-comment test is too weak / too brittle simultaneously** (flagged by: code-quality, test-coverage, portability, compatibility, standards) — substring assertions (`workspace-detect.sh`, `REPO_ROOT`) are trivially satisfied by unrelated content, while the `head -n 6` line bound makes any future header reformatting break the test. The plan also acknowledges that the implementation produces a four-line comment vs the AC9-mandated two-line form.
- **Coupling to jj's officially-internal `.jj/repo` marker** (flagged by: architecture, correctness, compatibility, portability) — no version-pinning safety net, no fallback path, no canary test if jj upstream changes the marker format. The work item flagged this as a known external coupling but the plan offers no mitigation beyond `mise.toml` version pinning.
- **AC5 golden-snapshot capture procedure is fragile** (flagged by: test-coverage, correctness, portability) — no TMPDIR/realpath determinism, no environment isolation, no recorded source commit, no structural prevention against later phases re-capturing from changed code. The regression guard the entire plan relies on may not survive its first regeneration.
- **`classify_checkout` redundantly re-runs probes** (flagged by: architecture, code-quality) — `classify_checkout`, `find_jj_main_workspace_root`, `find_git_main_worktree_root`, and the case arm each independently shell out to `jj workspace root` / `git rev-parse`. A colocated session invokes jj or git three times. Detection logic is duplicated across `classify_checkout` and the `find_*` helpers.
- **Path-normalisation divergence between new helpers and `find_repo_root`** (flagged by: architecture, code-quality, compatibility) — new helpers always `realpath`-normalise; `find_repo_root` does not. Within `vcs-detect.sh` itself the existing VCS-mode block uses un-normalised `REPO_ROOT` while the new boundary block uses normalised paths — same logical path, two representations.
- **Trailing-newline stripping by `$(build_boundary_block ...)`** (flagged by: code-quality, correctness, compatibility) — command substitution strips terminating newlines from the helper, so the boundary block ends without a final newline. Cosmetic today, but brittle for any future appended content.
- **AC8 byte-identity hard-codes a JSON entry string** (flagged by: test-coverage, compatibility) — fragile to harmless reformatting or future optional-field additions; structural assertions would express AC8's intent more faithfully.

### Tradeoff Analysis

- **Architecture (`find_repo_root` left untouched) vs Code Quality (path-normalisation consistency)**: The plan deliberately preserves `find_repo_root`'s contract to avoid a 45-caller ripple. This is the right call for scope, but it creates a real consistency cost inside `vcs-detect.sh` where two normalisation regimes coexist. Recommendation: keep `find_repo_root` untouched (scope discipline wins) but locally `realpath`-normalise `REPO_ROOT` inside `vcs-detect.sh` only.
- **Standards (`test-fixtures/` sibling convention) vs Plan (new `tests/fixtures/` tree)**: The plan introduces a new top-level fixture tree, diverging from the established sibling convention used everywhere else in the codebase. Recommendation: follow the existing convention (`hooks/test-fixtures/vcs-detect/`) — the plan is the only consumer and there is no reason to fragment.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Architecture / Correctness**: AC4 cross-VCS nesting silently drops the outer-VCS parent
  **Location**: Phase 2 (`classify_checkout`); Phase 4 (hook extension and AC4 test, lines 796–814)
  For a jj secondary workspace nested inside a pure-git parent, `classify_checkout` returns `jj-secondary` because from the inner directory `git_dir == git_common_dir` (no linked worktree was created); the hook then enters only the `jj-secondary` case arm and emits the jj parent alone. The work item's AC4 explicitly requires that the outer git-common-dir parent also be named with the three prohibitions. The AC4 test only asserts the jj parent appears in CTX, so the gap is not caught. Either extend the classifier with `nested-jj-in-git` / `nested-git-in-jj` enum values, or decouple classification from emission and emit a parent block for every non-empty resolved parent.

- 🟡 **Correctness / Compatibility**: Colocated fixture builder likely fails because target is non-empty
  **Location**: Phase 1 §3 (`make_colocated_secondary`, lines 286–298)
  The fixture runs `jj workspace add --quiet "$target"` first, populating the directory with `.jj/`, then `git worktree add -q "$target"` which refuses a non-empty target. Under `set -euo pipefail` the subshell fails and the entire suite aborts before any colocated assertion runs. Reverse the order (`git worktree add` first into a non-existent path, then `jj workspace add` into the existing directory), and add a fixture-build smoke check asserting both `.jj/repo` (file) and `.git` (file with `gitdir:` pointer) exist at the target.

- 🟡 **Correctness**: `find_git_main_worktree_root` breaks on bare repos and submodules
  **Location**: Phase 2 §2 (`find_git_main_worktree_root`, lines 531–542)
  `dirname "$common_dir"` is correct only for non-bare main checkouts. For bare repos, `--git-common-dir` returns the bare repo path and `dirname` returns its enclosing directory (not a checkout). For submodules, `--git-common-dir` points at `<superproject>/.git/modules/<name>` and `dirname` returns `<superproject>/.git/modules`. Also honours an externally-set `GIT_DIR`. Add `git rev-parse --is-bare-repository` and `--show-superproject-working-tree` handling; document or scrub `GIT_DIR`; add a bare-repo fixture.

- 🟡 **Architecture / Test Coverage**: No graceful degradation when `jj` or `git` binaries are missing
  **Location**: Phase 2 (helpers); Phase 3 (hook extension)
  The existing hook gracefully degrades on missing `jq` (exits 0 with a `systemMessage`). New helpers unconditionally shell out to `jj workspace root` and `git rev-parse`. On a machine without `jj`, `classify_checkout` returns `none` or `main` silently and the boundary block is suppressed inside a real jj secondary workspace — the exact failure mode the work item targets. Add `command -v jj` / `command -v git` guards mirroring the `jq` pattern, and a regression test that runs the hook with `PATH` munged to hide `jq`/`jj`/`git`.

- 🟡 **Code Quality**: `classify_checkout` duplicates probe logic; hook re-runs the same probes
  **Location**: Phase 2 (`classify_checkout`) + Phase 3/4 case arms
  Probe logic exists in three places: `classify_checkout`, `find_jj_main_workspace_root`/`find_git_main_worktree_root`, and the case arms (which call `jj workspace root` and `git rev-parse --show-toplevel` again). A colocated session invokes jj/git ~3 times. Have `classify_checkout` return a structured record (kind + boundary + parents) the hook consumes once, or have the helpers and classifier share a `_resolve_jj_workspace_state` / `_resolve_git_worktree_state` primitive.

- 🟡 **Test Coverage**: AC6 "does not error" threshold under-tested
  **Location**: Phase 1 §5 (lines 359–367)
  AC6 currently asserts only exit 0 and the absence of `do not edit files in`. The work-item review explicitly flagged the "does not error" threshold as undefined. Add (1) stderr-empty assertion via a separate redirect, (2) stdout-is-valid-JSON via `jq -e . >/dev/null`, (3) absence of all three prohibition phrases (not just the edit phrase).

- 🟡 **Test Coverage / Correctness / Portability**: AC5 golden snapshot capture is not isolated or deterministic
  **Location**: Phase 1 §4 (lines 314–330)
  Capture procedure does not pin `TMPDIR` (macOS `/var/folders/...` symlink), does not set `GIT_CEILING_DIRECTORIES` (could walk into the accelerator's own `.jj`), does not match the fixture builder's commit step (capture creates no commit; `make_main_git_checkout` does), and runs only on the author's host (CI is Ubuntu-only). Convert the procedure to a committed `regenerate.sh` script, capture on Linux as the canonical form, record the source commit hash alongside the fixture, and add a guard test that fails loudly if a snapshot contains a host-path artefact (`/private/var`, `/var/folders`).

- 🟡 **Test Coverage**: AC9 substring assertions trivially satisfiable
  **Location**: Phase 5 §1 (lines 916–919)
  `assert_contains` for the literal substrings `workspace-detect.sh` and `REPO_ROOT` could be satisfied by any unrelated comment. Tighten to canonical contiguous phrases (`do not split into workspace-detect.sh`, `shared REPO_ROOT and VCS_MODE computation`), or replace with a structural check that the file begins with a multi-line comment block of N or more lines.

- 🟡 **Test Coverage**: Helper failure-mode contracts not asserted
  **Location**: Phase 2 §1 (lines 440–444)
  The helper tests explicitly "do not assert exit code" for the failure path. The helper documents `exits 1 with empty stdout on failure`; not asserting that allows a refactor to silently change the failure contract. Use `assert_exit_code` (already in `scripts/test-helpers.sh`) for both helpers' failure paths, and add a `find_git_main_worktree_root` failure-path test that does not currently exist.

- 🟡 **Test Coverage**: Graceful "jq missing" path not regression-tested
  **Location**: Key Discoveries (line 121); Phase 1 / Phase 5
  The plan acknowledges the existing graceful-degradation contract but adds no test exercising it. A future change moving boundary emission ahead of the `jq` guard would silently break the contract. Add a test that runs the hook with `PATH` munged to hide `jq` and asserts exit 0 with a `systemMessage` envelope.

- 🟡 **Standards**: Fixture directory diverges from established `test-fixtures/` sibling convention
  **Location**: Phase 1 §3 (`FIXTURE_ROOT=...tests/fixtures/...`) and §4
  Every existing shell-test fixture lives in a sibling `test-fixtures/` next to the consuming test (`scripts/test-fixtures/`, `skills/work/scripts/test-fixtures/`, `skills/integrations/jira/scripts/test-fixtures/`). The plan creates a brand-new top-level `tests/fixtures/` tree. Use `hooks/test-fixtures/vcs-detect/` instead.

- 🟡 **Portability**: `jj 0.36.0` mise plugin availability not verified; CI test job Linux-only
  **Location**: Phase 1 §1 (mise.toml change); `.github/workflows/main.yml`
  Pin is deferred to implementation with a "confirm with `mise plugins list`" parenthetical. Macos-specific defects (BSD `mktemp`/`realpath`/`head`) are not caught because CI runs only `ubuntu-latest`. Verify `mise ls-remote jj` before merging Phase 1 and either add a macOS test matrix or explicitly accept the gap in "What We're NOT Doing".

- 🟡 **Portability**: `jj git init --quiet` flag support on 0.36.0 not verified
  **Location**: Phase 1 §3 fixture builders
  An unsupported flag fails every fixture builder at construction time; with `set -euo pipefail` this masks the real test results. Run `jj git init --help` and `jj workspace add --help` against the pinned version and adjust if needed.

- 🟡 **Portability / Correctness**: BSD vs GNU `mktemp` and TMPDIR symlinks
  **Location**: Phase 1 §3 (`new_workdir`) and §4 (snapshot capture)
  `new_workdir` wraps `mktemp` output in `realpath` — good — but the AC5 capture procedure uses bare `mktemp -d` and produces host-specific paths. If the existing hook ever emits a path (Phase 3+ certainly does), the snapshot is non-portable. Verify the current hook's output is host-path-agnostic and treat snapshots as Linux-canonical.

- 🟡 **Portability / Compatibility / Correctness**: `.jj/repo` internal format coupling unmitigated
  **Location**: Phase 2 §2 (`find_jj_main_workspace_root`); Desired End State
  The work item flagged jj-vcs/jj#8758 as known coupling. The plan accepts the risk but adds no version-pin, no fallback, no canary, and no defensive validation of the file contents (no length cap, no rejection of absolute paths, no post-resolve invariant check). Add a defensive invariant: after computing the candidate main root, assert `<candidate>/.jj/repo` is a directory; otherwise return 1. Wrap the marker test in a single internal function so an upstream `jj workspace repo-root` flip-over is a one-line change.

- 🟡 **Compatibility**: Plan does not gate boundary emission on Claude Code 2.1.0+
  **Location**: Implementation Approach / Desired End State
  The boundary block is significantly longer than today's VCS-mode block. On Claude Code <2.1.0 it surfaces as a visible chat message on every session start — a meaningful UX regression. The plan does not document this trade-off in the Migration Notes section nor offer a version-probe.

#### Minor

- 🔵 **Architecture**: `find_repo_root` left buggy in production for ~45 callers
  **Location**: "What We're NOT Doing"
  Explicitly out of scope, but the asymmetry leaves the broader system (e.g., `vcs-guard.sh` PreToolUse) mis-identifying linked worktrees. Add a deprecation comment on `find_repo_root` or a follow-up work item to migrate callers.

- 🔵 **Architecture**: Duplicated boundary-block formatters violate DRY
  **Location**: Phase 3 (`build_boundary_block`), Phase 4 (`build_colocated_boundary_block`)
  Prohibition prose lives in three places — drift risk. Factor `_emit_parent_block <label> <parent>` used by both.

- 🔵 **Code Quality / Correctness / Compatibility**: Trailing newlines stripped by `$(build_boundary_block ...)`
  **Location**: Phase 3/4 hook extension
  `$()` strips trailing newlines; the boundary block ends mid-character with no terminating newline. Append `$'\n'` after the substitution or use `+=` with explicit newline, and document at the call site.

- 🔵 **Architecture / Code Quality / Compatibility**: Path-normalisation divergence within `vcs-detect.sh`
  **Location**: Phase 2/3 emission
  `REPO_ROOT` (un-normalised) and `BOUNDARY`/`PARENT` (normalised) coexist in the same file. Locally `realpath` `REPO_ROOT` inside the hook (not in `find_repo_root`).

- 🔵 **Code Quality**: Pipe-delimited multi-value fixture returns are an ad-hoc convention
  **Location**: Phase 1 §3 fixture builders
  Use named globals (the conventional bash idiom) or newline-separated output read via `mapfile -t`.

- 🔵 **Code Quality**: `echo "$1" | jq` should be `jq <<< "$1"` or `printf '%s\n'`
  **Location**: Phase 3 (`extract_context`); fixture builders
  Matches `test-helpers.sh` convention and avoids `echo` portability quirks.

- 🔵 **Code Quality / Correctness**: Redundant `realpath` wrapping in `classify_checkout`
  **Location**: Phase 2 §2
  Post-`pwd` strings are already canonical; the inner `realpath` calls are redundant and obscure intent.

- 🔵 **Correctness**: No invariant check after resolving `.jj/repo` relative path
  **Location**: Phase 2 §2 (`find_jj_main_workspace_root` secondary branch)
  Verify `<candidate>/.jj/repo` is a directory before returning.

- 🔵 **Test Coverage**: AC4 only covers jj-in-git, not git-in-jj
  **Location**: Phase 4 (line 766 note; line 798 implementation)
  Plan flags symmetry but only implements one direction. Add `make_git_worktree_in_jj_parent` and mirrored assertions.

- 🔵 **Test Coverage**: `find_repo_root` unchanged-behaviour is only manually verified
  **Location**: Phase 2 Success Criteria — Manual Verification
  Six fixtures already exist; add automated assertions across them.

- 🔵 **Test Coverage / Compatibility**: AC8 hard-coded JSON entry string is brittle
  **Location**: Phase 5 §1 (lines 909–913)
  Replace with three structural jq assertions (matcher empty, one hook entry, command equals expected literal).

- 🔵 **Test Coverage / Portability**: No skip-if-missing guard for `jj`/`git` in fixture builders
  **Location**: Phase 1 §3
  Local devs running outside `mise run` see opaque failures. Add a preflight `command -v jj && command -v git` at the top of the test file.

- 🔵 **Correctness**: Snapshot capture-order discipline is procedural with no structural guard
  **Location**: Phase 1 §4
  Record the source commit hash in a sibling `CAPTURE-SOURCE.txt` and add a verification step that the fixture commit did not also touch production code.

- 🔵 **Compatibility / Portability**: AC9 `head -n 6` couples to header line count
  **Location**: Phase 5 §1 (line 917)
  Future header reformatting silently breaks AC9. Scan the whole top-of-file comment block instead, or grep the full file for canonical phrases.

- 🔵 **Standards**: AC9 four-line comment vs Desired End State "two-line" contradicts itself
  **Location**: Desired End State (line 90); Phase 5 §2
  Either tighten the comment to two lines or update Desired End State.

- 🔵 **Standards**: Invoke task docstring mentions PreToolUse but plan doesn't touch it
  **Location**: Phase 1 §2 (`@task hooks`)
  Tighten docstring to `"""Integration tests for SessionStart hooks (vcs-detect, etc.)."""` or `"""Integration tests for the hooks/ subtree."""`.

- 🔵 **Portability**: Test file uses bash process substitution; minimum bash version undocumented
  **Location**: Phase 1 §3 test scaffold
  Add a header guard checking `$BASH_VERSION` or a one-line bash-required note.

- 🔵 **Portability**: `git rev-parse --git-common-dir` is git 2.5+; min version undocumented
  **Location**: Phase 2 §2
  Document the minimum git version in a comment next to `find_git_main_worktree_root`.

- 🔵 **Portability**: macOS BSD `realpath` availability not verified
  **Location**: Throughout
  Pre-macOS-12.3 hosts need `brew install coreutils`. Add a preflight check or replace with `cd "$dir" && pwd -P`.

- 🔵 **Compatibility / Architecture**: jj-internal-marker dependency lacks a single isolation point
  **Location**: Phase 2 (`find_jj_main_workspace_root`, `classify_checkout`)
  Wrap the marker test in a single `_jj_workspace_is_secondary <workspace_root>` function so the eventual `jj workspace repo-root` migration is a one-line change.

- 🔵 **Code Quality**: Four parallel boolean-flag ints in `classify_checkout` is procedural
  **Location**: Phase 2 §2
  Replace cascade with `case "$in_jj:$jj_secondary:$in_git:$git_worktree" in ...` to make the truth table explicit.

- 🔵 **Code Quality**: Mixed responsibility — `find_git_main_worktree_root` always realpaths
  **Location**: Phase 2 §2
  Asymmetric with `find_repo_root`. Document the asymmetry at the top of `vcs-common.sh`.

- 🔵 **Code Quality**: AC9 comment grows to four lines despite plan claiming two
  **Location**: Phase 5 §2 (lines 928–939)
  Compress to two lines (the AC9 wording fits) or formally widen the AC.

- 🔵 **Test Coverage**: No direct assertion that `main`/`none` produce no boundary block
  **Location**: Phase 3
  Add explicit `assert_not_contains` for `WORKSPACE BOUNDARY DETECTED`, `Boundary (active workspace):`, `Parent repository` in the AC5/AC6 tests.

- 🔵 **Correctness**: Embedded spaces in `.jj/repo` contents tolerated but undocumented
  **Location**: Phase 2 §2
  Assert post-`cd` directory basename is `repo` and parent basename is `.jj` before walking up.

- 🔵 **Correctness**: Existing CONTEXT heredoc has no trailing newline; boundary block compensates implicitly
  **Location**: Phase 3 hook extension
  Add a comment near the case statement documenting the leading-`\n\n` invariant.

#### Suggestions

- 🔵 **Architecture**: Adding `jj` to `mise.toml` broadens the toolchain dependency surface
  **Location**: Phase 1 §1
  Acknowledge in Phase 1 that only `test:integration:hooks` consumes `jj`, or scope the declaration to a mise environment.

- 🔵 **Standards**: Fixture suffix `-pre-0058` mixes work-item numbering into a long-lived artefact name
  **Location**: Phase 1 §4
  Name `vcs-detect-golden` (or `vcs-detect`); rely on git/jj history for provenance.

- 🔵 **Standards**: Test script uses `set -euo pipefail` while sourced hooks/libraries do not
  **Location**: Phase 1 §3 test scaffold
  Add a one-line note distinguishing test-harness strict-mode from sourced-library inherit-options.

### Strengths

- ✅ Five-phase TDD structure with red-then-green discipline; every AC has a named test, mapped explicitly in the Testing Strategy section.
- ✅ AC5 golden-snapshot capture is correctly ordered before any production-code change — Phase 1 is read-only and the manual-verification checklist enforces commit order.
- ✅ Placement decision (extend `vcs-detect.sh`, not split into a sibling hook) is well justified — shared `REPO_ROOT`/`VCS_MODE` computation and one coherent SessionStart message preserve cohesion.
- ✅ Detection helpers pushed into `vcs-common.sh` as pure sourcable functions that can be exercised independently of the hook (AC7) — strong functional-core / imperative-shell separation.
- ✅ `find_repo_root` left untouched, avoiding a 45-caller ripple — a deliberate, documented scope decision.
- ✅ Boundary content folded into the existing `additionalContext` string rather than introducing a new sibling field — preserves AC5 byte-identity and avoids a schema change downstream.
- ✅ Classifier returns a small closed enum consumed via a single case statement — an extensible seam for future checkout kinds.
- ✅ Function naming (`find_jj_main_workspace_root`, `classify_checkout`) follows the unprefixed snake_case convention; no `set` flags in `vcs-common.sh` per the established library pattern.
- ✅ Mise task naming (`test:integration:hooks`), invoke task style (`@task hooks(context)`), and test-file naming (`hooks/test-vcs-detect.sh`) all match existing patterns.
- ✅ `realpath` normalisation treated as a global rule, explicitly motivated by macOS `/var` → `/private/var` symlink behaviour, and applied to both helper outputs and substituted parent paths in the prohibition phrases.
- ✅ AC1/AC2 substring assertions pin the full canonical phrase (`do not edit files in $PARENT_REAL`) so mutating any operative word would fail the test (mutation-testing-sound, resolves a pass-2 work-item-review concern).
- ✅ AC3 colocated test asserts the boundary line appears exactly once via a count assertion — directly verifies the "single coherent message" requirement.
- ✅ Helper-function names referenced (`assert_eq`, `assert_contains`, `assert_not_contains`) match `scripts/test-helpers.sh` exactly with the correct argument order.
- ✅ AC8 preserved by design: `hooks/hooks.json` is explicitly listed under "Files to leave untouched" and Phase 5 adds an explicit byte-identity guard.
- ✅ Authoritative VCS probes (`jj workspace root`, `git rev-parse --git-dir`/`--git-common-dir`) preferred over path-walking — the right protocol-compliance choice.

### Recommended Changes

Ordered by impact:

1. **Fix the AC4 emission gap** (addresses: AC4 cross-VCS nesting silently drops outer parent)
   Extend `classify_checkout` with `nested-jj-in-git` and `nested-git-in-jj` enum values (returned when `in_jj==1 && in_git==1` but jj and git roots differ and neither is `colocated`). Add corresponding case arms in the hook that emit a boundary block naming **both** parents (reusing the colocated formatter). Update the AC4 test to assert `GIT_PARENT_REAL` appears in CTX with full prohibition phrasing, plus add a symmetric `make_git_worktree_in_jj_parent` fixture for the mirrored direction.

2. **Repair the colocated fixture builder** (addresses: colocated fixture likely fails at build time)
   Reverse operation order: run `git worktree add` first into a non-existent path, then `jj workspace add` into the existing directory. Add a smoke check at the bottom of the fixture builder asserting both `.jj/repo` (file) and `.git` (file with `gitdir:` pointer) exist before returning.

3. **Harden `find_git_main_worktree_root`** (addresses: breaks on bare repos and submodules)
   Add `git rev-parse --is-bare-repository` (return 1 for bare) and `git rev-parse --show-superproject-working-tree` (resolve submodule parents). Document or scrub `GIT_DIR`. Add a bare-repo fixture in Phase 1.

4. **Add graceful degradation for missing `jj`/`git`** (addresses: silent under-detection on missing binaries)
   Mirror the existing `jq` pattern in `hooks/vcs-detect.sh:4-7`: emit a `systemMessage` warning and exit 0 if `command -v jj` or `command -v git` fails inside a directory where the other VCS would have classified it as secondary. Add a regression test that runs the hook with `PATH` munged.

5. **Restructure `classify_checkout` to share probe state** (addresses: probe duplication, redundant invocations)
   Have `classify_checkout` print a structured record on stdout (`KIND|BOUNDARY|JJ_PARENT|GIT_PARENT`) that the hook parses once. The case statement becomes pure formatting with no re-probing, and the `find_*` helpers either become callees of the classifier or share a `_resolve_jj_workspace_state` primitive with it.

6. **Tighten AC6 thresholds** (addresses: AC6 "does not error" under-tested)
   Add stderr-empty assertion (separate `2>` redirect), stdout-is-valid-JSON via `jq -e .`, and absence of all three prohibition phrases.

7. **Make the AC5 snapshot capture deterministic and verifiable** (addresses: AC5 fragility)
   Replace the inline procedure with a committed `regenerate.sh` script that pins `TMPDIR=/tmp`, exports `GIT_CEILING_DIRECTORIES`, and reuses the same fixture-builder logic as the test. Capture on Linux as the canonical form. Record the source commit hash in a sibling `CAPTURE-SOURCE.txt`. Add a guard test that fails if any snapshot contains `/private/var` or `/var/folders` substrings.

8. **Strengthen AC9** (addresses: AC9 trivially satisfiable + brittle + length mismatch)
   Replace the substring assertions with canonical contiguous phrases (`do not split into workspace-detect.sh`, `shared REPO_ROOT and VCS_MODE computation`). Replace `head -n 6` with a leading-comment-block scan. Decide whether the comment is two lines or four and align Desired End State with the Phase 5 implementation.

9. **Move fixtures to `hooks/test-fixtures/vcs-detect/`** (addresses: convention divergence)
   Drop the `tests/fixtures/` top-level tree; use the sibling pattern that every other shell-test fixture in the codebase uses. Drop the `-pre-0058` suffix.

10. **Assert helper failure contracts** (addresses: failure-mode contracts not asserted)
    Use `assert_exit_code` for both helpers' failure paths. Add `find_git_main_worktree_root` failure-path coverage.

11. **Replace AC8 hard-coded JSON entry with structural assertions** (addresses: AC8 brittleness)
    Three jq assertions: entry exists, `matcher == ""`, `hooks[0].command == "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh"`.

12. **Document Claude Code 2.1.0+ trade-off in Migration Notes** (addresses: silent UX degradation on older clients)
    Acknowledge that the boundary block will appear as a visible message on Claude Code <2.1.0; no version-probe is feasible today but the trade-off is conscious.

13. **Verify jj toolchain assumptions before Phase 1 merges** (addresses: jj plugin availability, `--quiet` flag)
    Run `mise ls-remote jj` to confirm 0.36.0 is offered; run `jj git init --help` and `jj workspace add --help` against the resolved version to confirm `--quiet` exists.

14. **Add a `_jj_workspace_is_secondary` isolation function** (addresses: jj internal marker coupling)
    Wrap the file-vs-directory test in one place so the eventual `jj workspace repo-root` migration is a single-line change. Add a defensive invariant check that the resolved main root actually has `.jj/repo` as a directory.

15. **Fix trailing-newline stripping** (addresses: `$(build_boundary_block ...)` strips final newline)
    Append `$'\n'` after the substitution or use `CONTEXT+=...` with explicit newline; document at the call site.

16. **Locally normalise `REPO_ROOT` inside `vcs-detect.sh`** (addresses: path-normalisation divergence)
    One-line `REPO_ROOT=$(realpath "$REPO_ROOT")` inside the hook closes the in-file divergence without touching `find_repo_root` callers.

17. **Add a bash-required preflight to the test file** (addresses: portability minor)
    Guard with `[ -z "$BASH_VERSION" ] && { echo 'bash required' >&2; exit 1; }` and add a `command -v jj && command -v git` skip check.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound: it extends the existing thin-orchestrator hook plus sourced common library, preserves the JSON envelope, and pushes detection into independently-testable helpers. Coupling and cohesion are appropriate; the open-closed seam (a single `case` on `classify_checkout`) makes future cases additive. Main risks: no graceful degradation if `jj` or `git` binaries are missing (unlike the existing `jq`-missing path); an inconsistent classifier output model (cross-VCS nesting collapses to single-VCS labels and drops the outer parent); a latent fragility from depending on jj's officially-internal `.jj/repo` marker.

**Strengths**:
- Placement decision well justified — shared `REPO_ROOT`/`VCS_MODE` and one coherent SessionStart message keeps cohesion high.
- Detection pushed into `vcs-common.sh` as pure sourcable helpers — preserves thin-orchestrator-over-library architecture.
- `find_repo_root` left untouched — avoids ripple changes through ~45 call sites.
- Boundary content folded into existing `additionalContext` string — preserves AC5 byte-identity.
- Classifier returns a small closed enum consumed via a single case statement — extensible seam.
- Phase 1 captures golden snapshots before any production-code edit — AC5 as regression-guarding architectural invariant.
- Convention adherence explicit: unprefixed snake_case helpers, no `set` flags in `*-common.sh`.

**Findings**:
- 🟡 **major**: `classify_checkout` enum is lossy for cross-VCS nesting (AC4) — the `jj-secondary` arm only calls `find_jj_main_workspace_root` and never emits the outer git parent; AC4's stated behaviour is not realised.
- 🟡 **major**: No graceful degradation when `jj` or `git` binaries are missing — asymmetric with the existing `jq`-missing branch; silent under-detection.
- 🔵 **minor**: Coexistence of buggy `find_repo_root` with new helpers leaves a known-broken detector in production for the ~45 other callers.
- 🔵 **minor**: Dependency on jj-internal `.jj/repo` marker is a fragile coupling — wrap in a single isolation function.
- 🔵 **minor**: Duplicated boundary-block formatters violate DRY for prohibition prose.
- 🔵 **minor**: `classify_checkout` result is re-derived rather than passed to emission — the hook re-runs the same probes.
- 🔵 **suggestion**: Adding `jj` to `mise.toml` broadens the toolchain dependency surface.

### Code Quality

**Summary**: Well-organised plan that follows existing conventions (function-naming, no `set` flags in sourced libraries, test-helpers usage), decomposes work into reviewable phases with red-then-green discipline. Several maintainability issues are baked into the shell code: redundant invocations of `jj`/`git` probes across `classify_checkout` and the hook's case arms, a DRY violation between `classify_checkout` and the `find_*` helpers, two helper functions that repeat substring extraction and emission logic, and small shell-quoting / `printf`-vs-`echo` / trailing-newline subtleties.

**Strengths**:
- Test-driven structure with red-then-green per phase.
- Anticipates the no-`set`-flags convention for `*-common.sh` libraries.
- Function-naming follows the unprefixed snake_case convention.
- `assert_contains` argument order matches `scripts/test-helpers.sh`.
- Fixture builders use real jj/git state rather than mocking.

**Findings**:
- 🟡 **major**: `classify_checkout` duplicates probe logic and the hook re-runs the same probes a third time — share probe semantics behind a single primitive.
- 🔵 **minor**: Pipe-delimited multi-value fixture returns are an ad-hoc convention — use named globals or `mapfile`.
- 🔵 **minor**: `echo "$1" | jq` used where `printf '%s\n' "$1" | jq` or `jq -r ... <<< "$1"` is the project convention.
- 🔵 **minor**: Trailing newlines stripped by `$(...)` — document or compensate.
- 🔵 **minor**: AC9 comment grows to four lines despite the plan asserting two.
- 🔵 **minor**: Mixed responsibility — `find_git_main_worktree_root` always `realpath`s while `find_repo_root` does not.
- 🔵 **minor**: Error paths swallow underlying failure context — add `command -v` guards and `systemMessage` warnings.
- 🔵 **minor**: Four parallel int-flag variables in `classify_checkout` are procedural — replace with a composed-key `case`.

### Test Coverage

**Summary**: Strong test-first structure with exhaustive AC-to-test mapping and correct helper-name assumptions. Principal gaps: under-defined AC6 "does not error" threshold, no regression tests for degraded environments (missing `jq`/`jj`/`git`), fragile AC9 substring assertions, unasserted error-path contract for the new helpers, AC5 golden-capture procedure lacking determinism/isolation.

**Strengths**:
- AC-to-test mapping exhaustive across all nine criteria.
- AC5 golden-snapshot capture correctly ordered before any production-code edit.
- Helper-function names referenced match `scripts/test-helpers.sh` exactly.
- AC1/AC2 substring assertions pin canonical phrases (mutation-testing-sound).
- Phase 2 helper tests cover every `classify_checkout` return value across all six fixtures.
- AC3 colocated test asserts the boundary line appears exactly once.
- Tests use real jj/git fixtures rather than stubs.

**Findings**:
- 🟡 **major**: AC6 "does not error" threshold under-tested — add stderr/JSON validity assertions.
- 🟡 **major**: AC5 golden snapshots not isolated from environmental drift — TMPDIR symlinks, `find_repo_root` walk-up, fixture-vs-capture step mismatch.
- 🟡 **major**: AC9 substring assertions trivially satisfiable — `workspace-detect.sh` and `REPO_ROOT` can appear in any unrelated comment.
- 🟡 **major**: Helper failure-mode contracts not asserted — explicit `# do not assert exit code` leaves the documented return-1 path unverified.
- 🟡 **major**: Graceful "jq missing" path not regression-tested — future change could silently break the contract.
- 🔵 **minor**: AC4 covers only one of two cross-VCS nesting directions.
- 🔵 **minor**: `find_repo_root` unchanged-behaviour is manual-only.
- 🔵 **minor**: No skip-if-missing guard for `jj`/`git` binaries in fixture builders.
- 🔵 **minor**: AC8 hooks.json equality test is brittle to whitespace and field order.
- 🔵 **minor**: No direct assertion that `main` and `none` classifications produce no boundary block.

### Correctness

**Summary**: The classification algorithm and emission are largely correct for the simple cases, and AC5 byte-identity works because the main path produces no case-arm match. The cross-VCS nesting case (AC4) has a real correctness gap: `classify_checkout` returns `jj-secondary` (or `git-worktree`) and the hook emits only the inner-VCS parent — silently dropping the outer parent the work item requires. Edge cases around `find_git_main_worktree_root` (bare repos, submodules) and a fixture-construction issue in the colocated builder also need addressing.

**Strengths**:
- Early-return for `in_jj==0 && in_git==0` correctly emits `none`, not `main`.
- AC5 byte-identity preserved by design (no `main` arm in the case statement).
- `find_jj_main_workspace_root`'s algorithm correctly maps jj's internal layout.
- Command substitution strips the trailing newline from `.jj/repo` contents (correct in this case).
- `jq -r '.hookSpecificOutput.additionalContext'` un-escapes `\n` correctly for substring assertions.

**Findings**:
- 🟡 **major**: Cross-VCS nested checkout silently drops the outer-VCS parent (AC4).
- 🟡 **major**: `find_git_main_worktree_root` breaks on bare repos and submodules; honours `GIT_DIR`.
- 🟡 **major**: Colocated fixture builder likely fails because target is non-empty after `jj workspace add`.
- 🔵 **minor**: Redundant `realpath` calls in `classify_checkout` (no functional bug, mask intent).
- 🔵 **minor**: No invariant check after resolving `.jj/repo` relative path.
- 🔵 **minor**: Trailing-newline stripping by `$()` removes the boundary block's final newline.
- 🔵 **minor**: Snapshot capture-order discipline is procedural with no structural guard.
- 🔵 **suggestion**: Existing CONTEXT heredoc has no trailing newline; boundary block compensates implicitly.

### Standards

**Summary**: The plan aligns with most established conventions (unprefixed snake_case helpers, no `set` flags in `*-common.sh`, `test:integration:<area>` mise task naming, `test-*.sh` test file naming, `invoke test.integration.<area>` task wrapper). The most material divergence is the fixture directory location: the plan introduces a brand-new top-level `tests/fixtures/` tree, whereas every existing shell-test fixture in the codebase lives in a sibling `test-fixtures/` directory next to the consuming test.

**Strengths**:
- Function naming follows the unprefixed snake_case convention.
- Correctly identifies and preserves the `*-common.sh` no-`set`-flags convention.
- Mise task naming `test:integration:hooks` follows the existing pattern.
- Invoke task style matches `tasks/test/integration.py` exactly.
- Test file name `hooks/test-vcs-detect.sh` matches `test-*.sh` convention.

**Findings**:
- 🟡 **major**: Fixture directory diverges from established `test-fixtures/` sibling convention — use `hooks/test-fixtures/vcs-detect/`.
- 🔵 **minor**: Test script uses `set -euo pipefail` while the hook it tests does not — acknowledge convention boundary.
- 🔵 **minor**: AC9 two-line vs four-line comment contradiction between Desired End State and Phase 5.
- 🔵 **minor**: Task docstring mentions PreToolUse despite plan only exercising SessionStart.
- 🔵 **suggestion**: Fixture suffix `-pre-0058` mixes work-item numbering into a long-lived artefact name.

### Portability

**Summary**: Reasonably aware of macOS/Linux divergence (`/private/var` symlinks, `realpath` normalisation) but several environment assumptions remain fragile: `jj 0.36.0` pinning flagged as uncertain yet unresolved, CI runs only `ubuntu-latest` so macOS-only divergences won't be caught, and the fixture builder uses bash features whose semantics differ between GNU and BSD. No vendor lock-in concerns and production-code dependencies sensibly delegated to mise, but a developer on stock macOS (no coreutils, bash 3.2) may fail before reaching CI.

**Strengths**:
- Toolchain pinning via mise is used consistently.
- `realpath` normalisation treated as a global rule motivated by macOS `/var` → `/private/var`.
- No cloud-provider or vendor coupling introduced.
- Plan acknowledges `jj` is the gating prerequisite for CI.

**Findings**:
- 🟡 **major**: `jj 0.36.0` mise plugin availability not verified; CI test job Linux-only.
- 🟡 **major**: `jj git init --quiet` flag support on 0.36.0 not verified.
- 🟡 **major**: BSD vs GNU `mktemp` template handling and TMPDIR symlink divergence on macOS — snapshot capture is host-specific.
- 🟡 **major**: `.jj/repo` internal file-format coupling unmitigated.
- 🔵 **minor**: Bash process substitution used; minimum bash version undocumented.
- 🔵 **minor**: `git rev-parse --git-common-dir` is git 2.5+; minimum git version undocumented.
- 🔵 **minor**: AC9 `head -n 6` couples to comment line count.
- 🔵 **minor**: macOS BSD `realpath` availability not verified.

### Compatibility

**Summary**: Preserves the SessionStart hook's JSON envelope shape (single `hookSpecificOutput.additionalContext` string), keeps `hooks.json` byte-identical, and keeps `find_repo_root` untouched — three strong compatibility wins. Main risks: coupling to jj's officially-internal `.jj/repo` marker (acknowledged but unmitigated), an unverified assumption that `git worktree add` will accept a target path already populated by `jj workspace add`, and silent UX degradation on Claude Code <2.1.0 that the plan does not call out.

**Strengths**:
- AC8 preserved by design; JSON envelope shape unchanged.
- AC5 golden-snapshot regression guard captured before any production-code change.
- `find_repo_root` left untouched, preserving 45+ existing callers.
- Authoritative VCS probes preferred over path-walking.
- Existing `jq`-missing graceful-degradation contract preserved.

**Findings**:
- 🟡 **major**: Colocated fixture builder may fail because `git worktree add` refuses a non-empty target.
- 🟡 **major**: jj `.jj/repo` marker is officially internal; no fallback or version pin.
- 🟡 **major**: Plan does not gate on Claude Code 2.1.0+; older versions show large visible message.
- 🔵 **minor**: AC8 hard-coded JSON entry string is brittle to harmless reformatting.
- 🔵 **minor**: AC9 `head -n 6` couples to comment line count.
- 🔵 **minor**: Path-normalisation divergence between new helpers and `find_repo_root` callers.
- 🔵 **minor**: `jj 0.36.0` mise plugin availability and CI parity not verified.
- 🔵 **suggestion**: Command substitution strips trailing newlines from `build_boundary_block`.

## Re-Review (Pass 2) — 2026-05-15

**Verdict:** REVISE (3 new major findings — all clustered around the missing-binary degradation mechanism introduced in response to pass-1 findings, plus the colocated fixture order which is still broken in a different direction)

### Previously Identified Issues

**Architecture (pass 1)**
- 🟡 AC4 cross-VCS nesting drops outer parent — Resolved (`nested-jj-in-git`/`nested-git-in-jj` KIND values; both parents flow through `build_boundary_block`)
- 🟡 No graceful degradation for missing jj/git — Partially resolved (`command -v` guards added in helpers; `systemMessage` mechanism in hook; but see new major #1)
- 🔵 `find_repo_root` left buggy for ~45 callers — Acknowledged with regression guard locking in unchanged behaviour; scope discipline preserved
- 🔵 jj-internal marker coupling — Resolved (`_jj_workspace_is_secondary` single migration point)
- 🔵 Duplicated boundary-block formatters — Resolved (`_emit_parent_block` factored)
- 🔵 `classify_checkout` re-derived rather than passed to emission — Partially resolved (case arms no longer re-probe; duplication moved inside the classifier instead)
- 🔵 Adding jj to mise.toml broadens toolchain — Acknowledged; jj remains a global toolchain entry

**Code Quality (pass 1)**
- 🟡 `classify_checkout` probe duplication — Partially resolved (hook-level eliminated; classifier still calls find_* helpers that re-probe)
- 🔵 Pipe-delimited multi-value returns — Resolved (named globals)
- 🔵 `echo "$1" | jq` — Resolved (`jq -r <<< "$1"`)
- 🔵 Trailing newlines stripped by `$()` — Resolved (`$'\n'` append at every concatenation site)
- 🔵 AC9 four-line comment vs two-line claim — Resolved (Desired End State rewritten; AC9 redefined as contiguous block)
- 🔵 `find_git_main_worktree_root` realpath asymmetry — Resolved (documented in Migration Notes)
- 🔵 Error paths swallow failure context — Resolved (`command -v` guards plus `systemMessage`)
- 🔵 Four parallel int-flag variables — Not addressed (cascade unchanged, now 7 arms)

**Test Coverage (pass 1)**
- 🟡 AC6 "does not error" threshold under-tested — Resolved (exit 0 + empty stderr + JSON validity + absence of all three prohibition phrases)
- 🟡 AC5 fixture isolation — Resolved (committed `regenerate.sh` + `CAPTURE-SOURCE.txt` + host-path-artefact guard + Linux-canonical policy)
- 🟡 AC9 substring assertions trivially satisfiable — Resolved (canonical contiguous phrases via awk-scanned leading block)
- 🟡 Helper failure-mode contracts not asserted — Resolved (exit-code + empty-stdout assertions for both helpers including bare-repo path)
- 🟡 `jq missing` path not regression-tested — Resolved (Phase 5 PATH-munged test — but see new major #3 about the test's own correctness)
- 🔵 AC4 only one direction — Resolved (`make_git_worktree_in_jj_parent` + symmetric AC4 test)
- 🔵 `find_repo_root` unchanged-behaviour manual-only — Resolved (Phase 2 regression block across three fixtures)
- 🔵 No skip-if-missing guard for jj/git — Resolved (preflight in test scaffold)
- 🔵 AC8 brittle — Resolved (three structural jq assertions)
- 🔵 No assertion main/none produce no boundary block — Resolved (`assert_not_contains` for boundary markers in AC5 and AC6)

**Correctness (pass 1)**
- 🟡 Cross-VCS drops outer parent — Resolved (architecture echo)
- 🟡 `find_git_main_worktree_root` bare/submodules — Resolved (`--is-bare-repository`, `--show-superproject-working-tree`, `GIT_DIR` scrub)
- 🟡 Colocated fixture builder fails (non-empty target) — Still present in different direction (new major #2): reversing the order moved the failure from `git worktree add` to `jj workspace add`, which also refuses an existing target by default
- 🔵 Redundant realpath in `classify_checkout` — Not addressed (still present but post-pwd is canonical, not a runtime bug)
- 🔵 No invariant check after `.jj/repo` resolve — Resolved (`[ -d "$candidate/.jj/repo" ]`)
- 🔵 Trailing newline strip — Resolved
- 🔵 Capture-order procedural with no structural guard — Resolved (CAPTURE-SOURCE.txt)

**Standards (pass 1)**
- 🟡 Fixture directory diverges from `test-fixtures/` — Resolved (`hooks/test-fixtures/vcs-detect/`)
- 🔵 Test script `set -euo pipefail` boundary — Resolved (explicit comment in scaffold)
- 🔵 AC9 two-line vs four-line — Resolved (AC9 redefined)
- 🔵 Task docstring mentions PreToolUse — Resolved
- 🔵 Fixture suffix `-pre-0058` — Resolved (dropped)

**Portability (pass 1)**
- 🟡 jj 0.36.0 plugin availability + Linux-only CI — Resolved (binding pre-merge `mise ls-remote jj`; macOS gap accepted in writing)
- 🟡 `jj git init --quiet` flag — Resolved (binding pre-merge `--help | grep --quiet` step)
- 🟡 BSD vs GNU mktemp / TMPDIR — Resolved (`regenerate.sh` pins TMPDIR; host-path-artefact guard)
- 🟡 `.jj/repo` internal format coupling — Resolved (isolation + invariant)
- 🔵 Bash min version undocumented — Resolved (preflight in test scaffold)
- 🔵 git 2.5 min version undocumented — Resolved (References + inline comment) — but see new minor about submodule deference requiring 2.13+
- 🔵 AC9 `head -n 6` brittleness — Resolved (awk leading-block scan)
- 🔵 macOS BSD realpath — Resolved (preflight)

**Compatibility (pass 1)**
- 🟡 Colocated fixture builder may fail — Still present in different direction (echoed by correctness)
- 🟡 jj `.jj/repo` marker is internal — Partially resolved (isolated + invariant, but no runtime canary or version pin at runtime)
- 🟡 Plan does not gate on Claude Code 2.1.0+ — Partially resolved (documented as conscious trade-off, but not version-probed or feature-flagged)
- 🔵 AC8 brittle — Resolved
- 🔵 AC9 `head -n 6` — Resolved
- 🔵 Path-normalisation divergence — Resolved (local realpath of `REPO_ROOT` in hook)
- 🔵 jj mise plugin verification — Resolved
- 🔵 Trailing newlines — Resolved

### New Issues Introduced

#### Major (new)

- 🟡 **Correctness / Architecture / Test Coverage (Phase 3 lines 1228-1237; Phase 5 lines 1554-1574)**: **Missing-binary diagnostic cannot fire in the very scenario it targets.** The `SYSTEM_MESSAGE` is gated on `case "$C_KIND" in jj-secondary|git-worktree|colocated|nested-jj-in-git|nested-git-in-jj`. When `jj` is absent from PATH inside a lone jj secondary workspace (no surrounding git), `classify_checkout`'s jj probe is silently skipped, `KIND` collapses to `none`, and the case never matches. The Phase 5 test `assert_contains "systemMessage names jj"` against `make_jj_secondary_workspace` will fail. **Fix**: detect missing-binary inside `classify_checkout` and surface via a `JJ_MISSING=1` / `GIT_MISSING=1` record field, or use a pre-classify path-walk fallback (if `.jj/` exists anywhere in ancestors but jj is missing, emit the diagnostic regardless of KIND).

- 🟡 **Correctness / Compatibility (Phase 1 §3, `make_colocated_secondary` lines 419-441)**: **Colocated fixture order still broken.** The plan reversed the order to `git worktree add` first, then `jj workspace add` — but `jj workspace add` also refuses an existing target by default (it expects to create the directory itself). The fixture aborts in Phase 1 fixture setup; the new smoke-checks at the end can't produce a useful diagnostic because the abort happens before them. **Fix**: verify `jj workspace add --help` for an `--existing-dir` / `--colocate` / `--here` flag on jj 0.36.0; if no such flag exists, construct the colocated fixture by `jj git init --colocate` against a fresh dir (single dir that is both jj main and git main), then layer secondaries on top, OR assemble the layout manually via `mkdir + cp .jj/.git markers`.

- 🟡 **Code Quality / Test Coverage (Phase 5 lines 1558-1585)**: **PATH-munging in the missing-binary regression test has bash syntax/scoping bugs.** Three defects: (1) `PATH=$NEW_PATH OUTPUT=$(...)` is parsed as two assignments to the current shell — PATH leaks for the rest of the script; (2) `RC=$?` after `PATH=$ORIG_PATH` captures the assignment's exit status (always 0), not the hook's — the graceful-exit contract is unverified; (3) `PATH=$NEW_PATH (cd ...)` is invalid syntax because env-prefix only applies to simple commands, not subshells — bash either errors or silently parses it as an assignment-then-subshell with PATH permanently set. **Fix**: use `RC=0; OUTPUT=$(PATH="$NEW_PATH" run_hook "$FIXTURE_SECONDARY") || RC=$?` (assignment within the command substitution applies only to the substitution's subshell), and `assert_eq "PATH restored" "$ORIG_PATH" "$PATH"` after each PATH-munged case.

#### Minor (new)

- 🔵 **Correctness (Phase 2 lines 1032-1036)**: Colocated arm in `classify_checkout` doesn't gate on `jj_main_root`/`git_main_root` non-emptiness. A defensive-invariant failure in `find_jj_main_workspace_root` would produce a misleading `KIND=colocated, JJ_PARENT=` record. The nested arms do gate; the colocated arm should mirror them.
- 🔵 **Test Coverage (Phase 2 lines 930-936)**: Submodule branch of `find_git_main_worktree_root` has no fixture or test. Newly-added code path is unexercised.
- 🔵 **Test Coverage (Phase 2 lines 920-925)**: `GIT_DIR` scrub branch in `find_git_main_worktree_root` has no test. The contract is unverifiable.
- 🔵 **Test Coverage (Phase 3 lines 1233-1235)**: Git-missing diagnostic branch is not regression-tested (only the jj-missing branch is).
- 🔵 **Test Coverage (Phase 1 §4)**: `regenerate.sh` byte-reproducibility is asserted as a success criterion but no test exercises it.
- 🔵 **Test Coverage (Phase 2 line 903)**: Defensive post-resolve invariant in `find_jj_main_workspace_root` has no negative-path test — the safety net could be silently removed in a refactor.
- 🔵 **Code Quality (Phase 2 lines 761-771; Phase 3 lines 1213-1221)**: Structured-record parser is duplicated between `parse_classification` in the test and the inline parser in the hook. Add `_parse_classification` to `vcs-common.sh` as a single owner.
- 🔵 **Code Quality / Architecture (Phase 2 classify_checkout)**: Probe duplication reduced but not eliminated — `classify_checkout` calls `find_jj_main_workspace_root` and `find_git_main_worktree_root`, each of which re-runs the same VCS probes. Factor `_resolve_jj_workspace_state` / `_resolve_git_worktree_state` primitives shared between classifier and helpers.
- 🔵 **Correctness (Phase 2 classify_checkout arm ordering)**: Arm ordering is load-bearing (`colocated` must precede `nested-*` because colocated also satisfies `jj_main_root != git_main_root`); not documented in code. Add a comment.
- 🔵 **Correctness (Phase 2 docs)**: `--show-superproject-working-tree` requires git 2.13+, not 2.5. Documented minimum understates the submodule deference dependency.
- 🔵 **Compatibility (Phase 2/3 structured-record contract)**: `KEY=VALUE` record format has under-specified escaping for paths containing `=` or newlines (POSIX-legal but unusual). Document the contract or switch to `${line%%=*}` / `${line#*=}` parsing that preserves `=` in values.
- 🔵 **Compatibility (Phase 3 envelope shape)**: Hook envelope now has two shapes depending on whether `systemMessage` is emitted alongside `hookSpecificOutput`. AC5 only covers the single-shape case; downstream tooling must handle both. Add explicit envelope-shape coverage in tests.
- 🔵 **Code Quality (Phase 3 §2)**: The systemMessage's jq integration is described in prose ("follow the existing `jq -n --arg ... --arg ...` pattern") but the actual extended invocation is not shown. An implementer could ship an unconditional `systemMessage:""` field that breaks AC5 byte-identity. Show the conditional-omission jq expression explicitly.
- 🔵 **Code Quality (Phase 3 lines 1186-1192)**: `_emit_parent_block` lives in `hooks/vcs-detect.sh` while `_jj_workspace_is_secondary` lives in `scripts/vcs-common.sh`. Document the placement rule, or move `_emit_parent_block` to `vcs-common.sh` for future hook reuse.
- 🔵 **Architecture (Phase 3 line 1182)**: Local `realpath` of `REPO_ROOT` overwrites the variable in place, creating two `REPO_ROOT` representations across the hook's lifetime (pre-block: un-normalised used by the existing `case "$VCS_MODE"` CONTEXT; post-block: normalised). Either name the normalised form differently or document the invariant explicitly inline.
- 🔵 **Architecture (Phase 2 lines 1037-1046)**: Nested-VCS classification is gated on `jj_secondary==1` or `git_worktree==1`. A jj *main* workspace nested inside a git checkout (e.g., `jj git init` inside an unrelated git working tree) is not classified as nested. Either document the scope in `classify_checkout`'s comment or extend the condition to fire whenever `jj_main_root != git_main_root`.
- 🔵 **Compatibility / Test Coverage (Phase 5 PATH-munging)**: Stripping the entire `dirname` of the binary from PATH on systems where jj/jq share a directory with bash/cat/jq (e.g., `/usr/bin`, `/opt/homebrew/bin`) also strips essential tools, producing misleading test results. Use a shadowing tmpdir or curated PATH instead.
- 🔵 **Portability (Phase 1 §5 host-artefact guard)**: AC5 guard omits `/private/tmp` — the macOS-resolved form of `/tmp`, which is exactly what `regenerate.sh` hardcodes. Add `/private/tmp` to the needle list (and arguably `/tmp/` itself).
- 🔵 **Portability (Phase 1 §4 regenerate.sh)**: `TMPDIR=/tmp` is a plain assignment, not exported. The script's own `mktemp` uses the value via parameter expansion (OK), but subprocesses (jj, git) see the inherited TMPDIR. Tighten to `export TMPDIR=/tmp` or scope the determinism claim narrower.
- 🔵 **Code Quality (Phase 5 AC9 awk script)**: The leading-comment-block scan conflates the placement-rationale comment with any other leading comments (e.g., a future `# shellcheck disable=...`). Either delimit with sentinel markers (`# AC9-RATIONALE-BEGIN/END`) or document the broader scope in the test comment.
- 🔵 **Compatibility / Code Quality (Phase 3/4 trailing-newline)**: `$'\n'` restoration is duplicated at every concatenation site (four call sites). A future contributor adding a new arm could silently regress. Move restoration into a single `append_boundary_block` helper or into `build_boundary_block` itself.
- 🔵 **Compatibility (jj internal-marker coupling)**: Isolated correctly but no runtime canary — a user with a non-pinned jj version (system or brew install bypassing mise) could hit a marker-format change without any diagnostic. Add a `jj --version` compatibility-band probe inside `_jj_workspace_is_secondary` or a nightly CI canary against the latest jj release.

#### Suggestions

- 🔵 **Standards**: `CAPTURE-SOURCE.txt` is a new artefact pattern with no codebase precedent. Accept as deliberate or fold the provenance lines into a JSON-snapshot header.
- 🔵 **Standards**: `regenerate.sh` location and naming diverge slightly from the existing `scripts/regenerate-notify-downgrade-fixtures.sh` example. The co-located placement is arguably clearer; document the choice briefly in the regenerate.sh header.

### Assessment

The pass-1 review's findings are largely resolved — every MAJOR from pass 1 is either resolved or has its resolution mechanism in place (with one straggler, the colocated fixture). The substantial restructure (structured `KEY=VALUE` record, seven `KIND` values, `_jj_workspace_is_secondary` isolation function, `_emit_parent_block` factor, missing-binary `systemMessage` machinery, host-path-artefact guard, `regenerate.sh` + `CAPTURE-SOURCE.txt` provenance, awk-scanned AC9, structural AC8 assertions) addresses the architectural and test-coverage problems that drove the pass-1 REVISE verdict.

The new majors are mostly second-order: the missing-binary mechanism added in response to pass-1's graceful-degradation finding has a gating bug (it can't fire in its primary target scenario); the colocated fixture's reverse-the-order fix moved the failure from `git worktree add` to `jj workspace add` (both VCSes refuse an existing target by default); and the missing-binary regression test introduced in Phase 5 has bash syntax/scoping bugs that prevent it from exercising what it claims. These are real but localised — none are structural problems with the plan; all three are fixable with focused edits.

**Recommendation**: Address the three new majors and the minor about the colocated arm not gating on parent presence, then mark the plan ready for implementation. The 20-ish remaining new minors are largely consequences of the restructure being substantial; treat them as a punch list for a third edit pass or as backlog notes the implementer can knock out during Phase 2–5 execution. Do **not** spawn another full review pass — this pass shows diminishing returns and the remaining minors are the kind of detail best caught during implementation rather than further plan editing.

## Pass-3 Resolution Note — 2026-05-15

**Verdict:** APPROVE

The four pass-2 issues flagged for resolution are all addressed in the plan:

- 🟡 Missing-binary diagnostic gating — Resolved. `classify_checkout` now surfaces `JJ_MISSING` / `GIT_MISSING` fields in the structured record via a new `_ancestor_has_marker` helper (scoped to diagnostic, explicitly NOT detection). The hook's diagnostic gates on these record fields instead of `KIND`, so a lone jj secondary with jj absent correctly produces the systemMessage even when `KIND=none`.
- 🟡 Colocated fixture builder — Resolved. Manual-graft approach (git worktree first into fresh path, then jj workspace to tmp + move `.jj/` into target with absolute `.jj/repo`) is portable across BSD/GNU realpath and decoupled from jj's flag set. Pure-filesystem smoke checks replace the helper-call dependency.
- 🟡 Phase 5 PATH-munging — Resolved. The bash syntax / scoping bugs are fixed: env-prefix on the function call inside `$(...)` for the jj case, subshell wrapper `( PATH=...; ... )` for the jq case. Explicit `assert_eq "PATH not leaked"` and exit-code assertions added.
- 🔵 Colocated arm parent gating — Resolved. `[ -n "$jj_main_root" ] && [ -n "$git_main_root" ]` added to the colocated condition with an explanatory arm-ordering comment.

**Bonus**: Concretised the jq envelope extension with the conditional `+ (if $sys == "" then {} else {systemMessage: $sys} end)` so AC5 byte-identity is provably preserved when no systemMessage is emitted.

The plan is accepted for implementation. The ~20 remaining minor findings from pass 2 (submodule fixture, GIT_DIR scrub test, git-missing diagnostic mirror, regenerate.sh reproducibility test, `/private/tmp` host-artefact guard, structured-record `=`-in-path edge case, etc.) are deferred to the implementer's punch list — they are most efficient to resolve during the actual Phase 1–5 execution rather than through further plan editing.
