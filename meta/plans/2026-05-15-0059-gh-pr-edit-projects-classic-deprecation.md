---
date: "2026-05-15T16:50:00+01:00"
type: plan
producer: create-plan
work_item_id: "0059"
status: done
id: "2026-05-15-0059-gh-pr-edit-projects-classic-deprecation"
title: "gh pr edit → REST PATCH Migration in describe-pr Implementation Plan"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-15T16:50:00+01:00"
last_updated_by: Toby Clemson
revision: "4a4febd1f1ac"
repository: "ticket-management"
relates_to: ["work-item:0059", "codebase-research:2026-05-15-0059-gh-pr-edit-projects-classic-deprecation", "adr:ADR-0010"]
---

# gh pr edit → REST PATCH Migration in describe-pr Implementation Plan

## Overview

Replace the failing `gh pr edit` invocation in
`skills/github/describe-pr/SKILL.md:130` with a call to a new, tested shell
helper that PATCHes the PR body via the GitHub REST API. The resolution of
the base (upstream) repository is itself factored into a separately tested
shared helper, `skills/github/scripts/pr-base-repo.sh`, which is reused by
`describe-pr`, `review-pr`, and `respond-to-pr` so all three skills become
cross-fork-safe in a single change. The describe-pr helper encodes the body
to a tempfile via `jq -Rs '{body: .}'` and PATCHes through
`gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input <file>`,
bypassing the Projects (classic) GraphQL path entirely.

The work follows strict test-driven development: stub scripts and complete
test harnesses for both helpers land in Phase 1 against a `gh`-stub
installed on `PATH`, and tests are confirmed **red** before any production
logic is written. Phase 2 implements the shared resolver. Phase 3
implements the describe-pr helper (encoder + PATCH) on top of the shared
resolver. Phase 4 wires `describe-pr/SKILL.md` to the helper. Phases 5 and
6 migrate `review-pr` and `respond-to-pr` to the shared resolver. Phase 7
covers the live-PR manual verification that no automation can substitute.

## Current State Analysis

- **Single broken edit site**: `skills/github/describe-pr/SKILL.md:130` is
  the only live `gh pr edit` invocation in `skills/` (verified by research
  at `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`).
  `review-pr` and `respond-to-pr` reference `describe-pr` only via
  user-facing workflow narratives — they inherit the failure transitively,
  not via runtime shell-out.
- **Cross-fork-unsafe resolver duplicated across sibling skills**:
  `review-pr/SKILL.md:117` and `respond-to-pr/SKILL.md:67` both call
  `gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'`. This
  returns the local checkout's repo (the fork, for a contributor working
  in a fork), not the base repository the PR targets. The Reviews API and
  the body PATCH must address the **base** repo. This is a latent bug in
  both skills; the fix in scope replaces both call sites with a shared
  helper that uses `gh pr view --json baseRepository` (cross-fork-safe).
- **No github-skills helper scripts exist today**: `skills/github/`
  currently contains only three `SKILL.md` files. There is no
  `skills/github/scripts/` or `skills/github/*/scripts/` directory and no
  precedent for testable shell helpers in this area. This plan introduces
  the first ones.
- **Strong codebase pattern for testable shell helpers**:
  `skills/work/scripts/`, `skills/decisions/scripts/`, and
  `skills/visualisation/visualise/scripts/` all follow the same shape: a
  per-script production file plus a companion `test-*-scripts.sh` harness
  that sources `scripts/test-helpers.sh` for
  `assert_eq`/`assert_contains`/`assert_exit_code` and friends.
- **mise integration-test wiring is per-area**: shell harnesses for skills
  are wired under `test:integration:*` (decisions, config, visualiser,
  binary-acquisition) via `run_shell_suites(...)` in
  `tasks/test/integration.py`. `test:unit:*` is reserved for cargo,
  Vitest, and pytest runners. The new github harnesses go under
  `test:integration:github`, registered in
  `[tasks."test:integration"].depends`.
- **No PATH-stubbed `gh` precedent**: tests in
  `skills/visualisation/visualise/scripts/` mock external binaries via
  per-test tempdirs and env-var injection (`ACCELERATOR_VISUALISER_BIN`),
  but no existing test PATH-stubs `gh`. A PATH-stub for `gh` is the
  cleanest mock here because the production script shells out to `gh`
  with no env hook.
- **jq is an undeclared external dependency**: `mise.toml [tools]` pins
  `uv`, `python`, `gh`, `rust`, `node`, `shellcheck`, and `jj` but not
  `jq`. Existing scripts that depend on jq
  (`hooks/vcs-detect.sh`, `hooks/config-detect.sh`,
  `skills/visualisation/visualise/scripts/launch-server.sh:91`,
  `skills/integrations/jira/scripts/jira-common.sh:225`) defensively
  preflight with `command -v jq` and emit a clear remediation hint. The
  new helpers follow the same pattern.
- **Frontmatter strip and tmp cleanup are already in place** at
  `describe-pr/SKILL.md:119-129` and `:131` respectively. AC4 ("frontmatter
  is stripped before the body is posted") and the related cleanup
  criterion are already satisfied by the existing skill — the fix must
  preserve, not re-introduce, these steps.
- **Error-handling precedent at line 54-55** of the same skill already
  documents the "no default remote repository" remediation for
  `gh pr diff`; the new helpers surface the same remediation when
  resolution fails, with the underlying `gh` stderr preserved alongside.
- **ADR-0010**
  (`meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`)
  is the precedent for dropping to `gh api` when porcelain CLI is
  inadequate. It does not pin down body encoding, owner/repo resolution,
  or cross-fork handling, so this plan introduces those conventions on
  top of the ADR's spirit and records them as header documentation on
  the helpers.
- **describe-pr's `allowed-tools` frontmatter is currently scoped to
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`** (line 8-9). The new
  helper invocations require extending this. `review-pr` and
  `respond-to-pr` need analogous extensions for the shared resolver.

### Key Discoveries:

- `skills/github/describe-pr/SKILL.md:130` — sole edit site (the broken
  `gh pr edit` call).
- `skills/github/describe-pr/SKILL.md:119-129` — existing frontmatter
  strip; preserve.
- `skills/github/describe-pr/SKILL.md:131` — existing unconditional
  cleanup; preserve.
- `skills/github/describe-pr/SKILL.md:54-55` — existing "no default remote
  repository" remediation; mirror for the new resolution step.
- `skills/github/review-pr/SKILL.md:117` — cross-fork-unsafe resolver
  call site; migrate in Phase 5.
- `skills/github/review-pr/SKILL.md:130-131` — error-handling prose for
  the resolver; update remediation pointer in Phase 5.
- `skills/github/respond-to-pr/SKILL.md:67` — cross-fork-unsafe resolver
  call site; migrate in Phase 6.
- `scripts/test-helpers.sh` — shared assertion library
  (`assert_eq`/`assert_contains`/`assert_exit_code`/`assert_file_executable`/
  `test_summary`).
- `skills/decisions/scripts/test-adr-scripts.sh:1-40` — canonical shape
  for a per-skill test harness (sourcing test-helpers.sh, `mktemp -d` +
  trap cleanup, `setup_repo` helper).
- `tasks/test/integration.py` — invoke task definitions for
  `test:integration:*`; the new github task follows the same
  `run_shell_suites(context, 'skills/github')` pattern.
- `mise.toml:114-122` — `test:integration` aggregate task with
  multi-line `depends = [...]` block formatting that the new
  `test:integration:github` task should be added to.
- `skills/visualisation/visualise/scripts/test-launch-server.sh:1-50` —
  pattern for env-driven fake binaries; this plan adopts a similar but
  simpler PATH-stub variant for `gh`.
- `skills/integrations/jira/scripts/jira-common.sh:225` — canonical
  `command -v jq` preflight pattern.

## Desired End State

After this plan is complete:

- `skills/github/scripts/pr-base-repo.sh` exists, is executable, takes
  `<pr-number>` as its single positional arg, resolves the base
  (upstream) repository via `gh pr view --json baseRepository`, prints
  `owner/name` to stdout on success, exits non-zero with the captured
  `gh` stderr followed by a conditional `gh repo set-default`
  remediation hint on failure. It validates that owner and name are
  non-empty (no `null/null` smuggled through) and preflights `jq`.
- `skills/github/scripts/test-pr-base-repo-scripts.sh` exists and
  exercises all resolver scenarios against a PATH-stubbed `gh`.
- `skills/github/describe-pr/scripts/pr-update-body.sh` exists, is
  executable, takes `<pr-number> <body-file>` as positional args,
  resolves the base repo via the shared helper, encodes the body to a
  tempfile via `jq -Rs '{body: .}'`, then PATCHes via
  `gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input <tempfile>`.
  It cleans up the tempfile on both success and failure (trap), exits 0
  on success, and exits non-zero with a stage-specific stderr message
  ("encode failed" vs "PATCH failed") on failure.
- `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
  exists and exercises all acceptance scenarios (encoding edge cases,
  PATCH URL targeting, failure-path attribution) against a PATH-stubbed
  `gh`.
- `mise.toml` has a new `test:integration:github` task backed by an
  invoke task in `tasks/test/integration.py` that calls
  `run_shell_suites(context, 'skills/github')`; the task is added to
  `[tasks."test:integration"].depends`. `jq` is added to `[tools]`
  alongside `gh`.
- `skills/github/describe-pr/SKILL.md` step 9 no longer references
  `gh pr edit`; it invokes
  `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh`
  with the PR number and the stripped body file. Frontmatter strip
  (lines 119-129) and tmp cleanup (line 131) are preserved verbatim.
  Its `allowed-tools` is extended for the new helper.
- `skills/github/review-pr/SKILL.md` step 1.5 (line 117) no longer
  invokes `gh repo view`; it invokes
  `${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh
  {number} > {tmp directory}/pr-review-{number}/repo-info.txt`. Its
  `allowed-tools` is extended; the error-handling prose at lines
  130-131 points at the new helper's remediation.
- `skills/github/respond-to-pr/SKILL.md` step 3 (line 67) no longer
  invokes `gh repo view`; it invokes the shared helper. Its
  `allowed-tools` is extended.
- `mise run test:integration:github` passes with all assertions green.
- `mise run test:integration` (the aggregate) continues to pass.
- A live `/describe-pr` run against any open PR posts the body cleanly
  with no `gh pr edit` invocation in the command trace and no
  "deprecation-fallback" note emitted (verified manually in Phase 7).

### How to verify it:

- `mise run test:integration:github` — all assertions green.
- `mise run test:integration` — full integration aggregate green.
- `grep -rn "gh pr edit" skills/` — returns no matches (also a guard
  test in the harness).
- `grep -rn "gh repo view --json owner,name" skills/github/` — returns
  no matches (the two sibling-skill call sites have been migrated).
- Live `/describe-pr` against a same-repo PR — body posts, no fallback
  note.
- Live `/describe-pr` against a cross-fork PR (or a `gh api` simulation)
  — PATCH targets `{upstream-owner}/{upstream-repo}/pulls/{number}`.
- Live `/review-pr` and `/respond-to-pr` against any open PR — the
  resolver line writes the correct base owner/name and the workflows
  proceed without error.

## What We're NOT Doing

- Not extracting a general-purpose `gh` wrapper. The shared resolver is
  scoped to base-repository resolution; future helpers (e.g.
  `pr-update-title.sh`) will live alongside `pr-base-repo.sh` under
  `skills/github/scripts/` only. Phase 1 lands a brief
  `skills/github/scripts/README.md` documenting this area-level vs
  per-skill helper convention so the rule is durable rather than
  implicit.
- Not writing an ADR. ADR-0010's spirit covers the "drop to `gh api`"
  pattern; this plan captures the new
  `jq -Rs '{body: .}'`, `gh pr view --json baseRepository`,
  `--method PATCH`, and tempfile-encode conventions in
  header-comment documentation on the helpers. If a follow-up ADR is
  desired to canonicalise them, that is a separate decision.
- Not removing or rewording the existing frontmatter-strip recipe in
  step 9. AC4 is already satisfied; the strip remains a procedural
  recipe in SKILL.md prose, not delegated to the script. The helper's
  header comment documents the precondition that its `<body-file>` is
  already frontmatter-stripped.
- Not changing how step 9 sources or writes the body file
  (`{tmp directory}/pr-body-{number}.md`). The helper consumes that
  file as-is; the path remains under SKILL.md's control.
- Not adding retry logic, exponential backoff, or other resilience
  features beyond exit-code parity with a single PATCH call. PATCH by
  body-replacement is naturally idempotent; manual re-run is the
  documented recovery path. Distinct exit codes per stage (encode vs
  PATCH) are introduced so a future wrapper could differentiate
  without modifying the helper.
- Not optimising chained `/review-pr` → `describe-pr` flows that
  re-resolve the base repo twice (once in `review-pr` step 1.5,
  once inside `pr-update-body.sh`). The duplicate call is idempotent
  and sub-second; if it becomes load-bearing, add an optional
  `--base-repo <owner/name>` flag to `pr-update-body.sh` in a
  follow-up work item.

## Implementation Approach

Strict TDD red-green: Phase 1 lands two executable stub scripts (each
just prints a "not yet implemented" message and exits 1) **and** complete
test harnesses for both, plus the `test:integration:github` mise task and
its invoke backing. The phase confirms harnesses fail for the expected
reasons. Phase 2 implements the shared resolver; its tests turn green.
Phase 3 implements the describe-pr helper on top of the resolver; its
tests turn green. Phase 4 swaps describe-pr step 9 and extends its
`allowed-tools`. Phases 5 and 6 migrate `review-pr` and `respond-to-pr`
to the shared resolver (independent of each other; sequenced for review
clarity). Phase 7 covers live-PR manual verification.

The PATH-stubbed `gh` mock is a single script that dispatches on `$1
$2` (exact-match arms: `pr view`, `api`) and emits canned stdout/stderr
from per-test env-var-pointed files. Stdin capture is gated on the
`api` subcommand only so `gh pr view` calls don't block on inherited
stdin; further, the fake parses `--input <path>` from its argv and
copies the file's contents into `$GH_STDIN_LOG` (production code uses
`--input <file>`, not stdin), so round-trip body assertions can inspect
the encoded JSON uniformly. An unknown-verb default arm fails loudly.
A sibling `skills/github/scripts/test-helpers.sh` exposes
`install_fake_gh` and `setup_gh_stub` factories; each harness sources
both the repo-root `scripts/test-helpers.sh` (for assertions) and the
github-local `test-helpers.sh` (for the fake-gh factory)
independently, matching the visualiser-scripts source pattern.

Tests 22 and 23 are tree-state regression guards (asserting that
certain `gh` invocations no longer appear under `skills/`). They are
phase-conditional: the harness reads a `PHASE` env var (default
`final`) and short-circuits guards in earlier phases via a
`skip_test` helper. Phase-by-phase local runs override via
`PHASE=<n> mise run test:integration:github`; CI defaults to
`PHASE=final` so all guards are enforced at merge time. The PHASE
contract is wired explicitly through the mise task's `env` block so
it is not dependent on implicit env-inheritance.

---

## Phase 1: Skeletons, RED Tests, and mise Wiring

### Overview

Land both executable-but-incomplete scripts, both test harnesses, the
shared fake-gh factory, the mise task, and confirm tests are red. No
production logic is written in this phase — every assertion should fail
because the stubs are no-ops, not because the harnesses are wrong.

### Changes Required:

#### 0. Extend the shared assertion library

**File**: `scripts/test-helpers.sh`
**Changes**: Add a third outcome counter (`SKIP`) and two helpers that
the new github harnesses (and any future harnesses with skip semantics
or grep-based regression guards) will use. These do not exist in the
library today.

First, extend the counter declarations at the top of the file
(currently `PASS=0; FAIL=0`) to add a SKIP counter:

```bash
PASS=0
FAIL=0
SKIP=0
```

Update `test_summary` to report skips:

```bash
test_summary() {
  printf 'Passed: %d\n' "$PASS"
  printf 'Skipped: %d\n' "$SKIP"
  printf 'Failed: %d\n' "$FAIL"
  [ "$FAIL" -eq 0 ]
}
```

Add the new helpers (using the existing `assert_*` signature
convention `<test_name>` first):

```bash
# skip_test <test_name> <reason>
# Reports a test as skipped (increments SKIP, not PASS, so deferred
# guards remain visible in the summary). The green gate is FAIL=0
# regardless of SKIP count.
skip_test() {
  local test_name="$1"
  local reason="$2"
  printf '  SKIP: %s (%s)\n' "$test_name" "$reason"
  SKIP=$((SKIP + 1))
}

# assert_grep_empty <test_name> <path> <pattern> [<grep_extra_args>...]
# Asserts that `grep -rn <pattern> <path>` returns no matches.
# Distinguishes "no matches" (grep exit 1 → PASS) from "search error"
# (grep exit 2, missing path, etc. → FAIL with a clear diagnostic).
# Extra args (e.g. --include='*.md') are passed through to grep.
assert_grep_empty() {
  local test_name="$1"
  local path="$2"
  local pattern="$3"
  shift 3
  if [ ! -e "$path" ]; then
    printf '  FAIL: %s — path does not exist: %q\n' "$test_name" "$path"
    FAIL=$((FAIL + 1))
    return 1
  fi
  local matches rc
  matches=$(grep -rn "$@" "$pattern" "$path" 2>&1)
  rc=$?
  case "$rc" in
    0)
      printf '  FAIL: %s — found unexpected matches for %q under %q\n' \
        "$test_name" "$pattern" "$path"
      printf '%s\n' "$matches" | sed 's/^/    /'
      FAIL=$((FAIL + 1))
      return 1
      ;;
    1)
      printf '  PASS: %s — no matches for %q under %q\n' \
        "$test_name" "$pattern" "$path"
      PASS=$((PASS + 1))
      ;;
    *)
      printf '  FAIL: %s — grep error (exit %d) searching %q under %q:\n' \
        "$test_name" "$rc" "$pattern" "$path"
      printf '%s\n' "$matches" | sed 's/^/    /'
      FAIL=$((FAIL + 1))
      return 1
      ;;
  esac
}
```

Note that the counter names are `PASS`/`FAIL`/`SKIP` (bare, no
`_COUNT` suffix) matching the existing library convention, and
`assert_grep_empty` takes `<test_name>` as its first positional
argument matching every other `assert_*` helper. The green gate
remains `FAIL=0` so skipped tests don't block CI.

#### 1. Shared resolver stub

**File**: `skills/github/scripts/pr-base-repo.sh` (new)
**Changes**: Create executable stub.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Usage: pr-base-repo.sh <pr-number>
# Prints "<owner>/<name>" of the base (upstream) repository for the given
# pull request to stdout. Cross-fork-safe: resolves via
# `gh pr view --json baseRepository`, not `gh repo view`. Used by
# describe-pr (for PATCHing the body), review-pr, and respond-to-pr.
#
# Exit codes:
#   0  success
#   1  resolution failed (auth, network, 404, malformed JSON, ...)
#   2  usage error (wrong arg count, missing jq, ...)
#
# Implementation arrives in Phase 2.

echo "pr-base-repo.sh: not yet implemented (see meta/plans/2026-05-15-0059-...)" >&2
exit 1
```

#### 2. Describe-pr helper stub

**File**: `skills/github/describe-pr/scripts/pr-update-body.sh` (new)
**Changes**: Create executable stub.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Usage: pr-update-body.sh <pr-number> <body-file>
# Posts the contents of <body-file> as the body of pull request
# <pr-number> on the base (upstream) repository, using the GitHub REST
# API (PATCH /repos/{owner}/{repo}/pulls/{number}).
#
# Precondition: <body-file> MUST already have YAML frontmatter stripped.
# This helper does not strip frontmatter — that is SKILL.md's
# responsibility per the existing recipe in describe-pr/SKILL.md
# sub-steps 1-4 of step 9.
#
# Exit codes:
#   0   success
#   1   encode failed (jq could not read or encode the body file)
#   2   usage error (wrong arg count, missing body file, missing jq)
#   4   PATCH failed (gh api error against the GitHub REST endpoint)
#   other  bubbled up from pr-base-repo.sh
#          (1 = resolution failed, 2 = resolver usage / missing jq)
#
# Implementation arrives in Phase 3.

echo "pr-update-body.sh: not yet implemented (see meta/plans/2026-05-15-0059-...)" >&2
exit 1
```

#### 3. Shared test helpers and fake-gh factory

**File**: `skills/github/scripts/test-helpers.sh` (new)
**Changes**: Add the fake-gh installer and stub-setup helper used by
both github harnesses. Does NOT source the repo-root
`scripts/test-helpers.sh` — each consumer harness sources both files
independently, matching the visualiser-scripts pattern.

```bash
#!/usr/bin/env bash
# shellcheck shell=bash

# install_fake_gh <bin-dir>
# Writes a fake `gh` binary into <bin-dir>. The fake records argv to
# $GH_ARGV_LOG and, for `gh api` calls that pass `--input <file>`,
# copies the file's contents into $GH_STDIN_LOG so tests can inspect
# the JSON payload uniformly (production code uses --input, not stdin).
#
# Reserved exit codes (the script-under-test MUST NOT return these):
#   98 --input <path> argv supplied but path is not a readable file
#      (loud trap against silent stdin-fallback hangs in CI)
#   99 unknown verb / unexpected invocation (loud trap against typos
#      and accidental subcommand additions)
# Tests that assert on script-under-test exit codes must not collide
# with 98 or 99.
install_fake_gh() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/gh" <<'FAKE_GH'
#!/usr/bin/env bash
echo "$@" >> "${GH_ARGV_LOG:?GH_ARGV_LOG must be set}"
case "$1 ${2:-}" in
  "pr view")
    if [ -n "${GH_PR_VIEW_OUT:-}" ] && [ -f "$GH_PR_VIEW_OUT" ]; then
      cat "$GH_PR_VIEW_OUT"
    fi
    if [ -n "${GH_PR_VIEW_ERR:-}" ] && [ -f "$GH_PR_VIEW_ERR" ]; then
      cat "$GH_PR_VIEW_ERR" >&2
    fi
    exit "${GH_PR_VIEW_RC:-0}"
    ;;
  "api "*|"api")
    # Real gh reads --input <file> when present, otherwise from stdin.
    # Mirror that so round-trip body assertions work regardless of the
    # caller's posting style. Fail loudly if --input <path> is in
    # argv but the path is not a readable file (a CI/hang trap).
    if [ -n "${GH_STDIN_LOG:-}" ]; then
      input_path=""
      input_explicit=0
      prev=""
      for arg in "$@"; do
        if [ "$prev" = "--input" ]; then
          if [ "$arg" = "-" ]; then
            : # explicit stdin sentinel — fall through to cat below
          else
            input_path="$arg"
            input_explicit=1
          fi
          break
        fi
        prev="$arg"
      done
      if [ "$input_explicit" -eq 1 ]; then
        if [ -f "$input_path" ] && [ -r "$input_path" ]; then
          cat "$input_path" >> "$GH_STDIN_LOG"
        else
          echo "fake-gh: --input path $input_path is not a readable file" >&2
          exit 98
        fi
      else
        cat >> "$GH_STDIN_LOG"
      fi
    fi
    if [ -n "${GH_API_OUT:-}" ] && [ -f "$GH_API_OUT" ]; then
      cat "$GH_API_OUT"
    fi
    if [ -n "${GH_API_ERR:-}" ] && [ -f "$GH_API_ERR" ]; then
      cat "$GH_API_ERR" >&2
    fi
    exit "${GH_API_RC:-0}"
    ;;
  *)
    echo "fake-gh: unexpected invocation: $*" >&2
    exit 99
    ;;
esac
FAKE_GH
  chmod +x "$bin_dir/gh"
}

# setup_gh_stub <tmpdir>
# Creates a bin directory under tmpdir, installs the fake gh, scopes
# TMPDIR to a fresh empty subdir so tempfile-cleanup assertions can
# detect leaks, and exports defaults for all env vars the fake reads.
setup_gh_stub() {
  local tmpdir="$1"
  local bin_dir="$tmpdir/bin"
  install_fake_gh "$bin_dir"
  export PATH="$bin_dir:$PATH"
  export TMPDIR="$tmpdir/mktemp"
  mkdir -p "$TMPDIR"
  export GH_ARGV_LOG="$tmpdir/gh-argv.log"
  export GH_STDIN_LOG="$tmpdir/gh-stdin.log"
  : >"$GH_ARGV_LOG"
  : >"$GH_STDIN_LOG"
  unset GH_PR_VIEW_OUT GH_PR_VIEW_ERR GH_PR_VIEW_RC
  unset GH_API_OUT GH_API_ERR GH_API_RC
}

# install_fake_jq <bin-dir>
# Writes a fake `jq` into <bin-dir> that mimics the real binary closely
# enough for the resolver (`jq -r '<filter>'`) to keep working while
# forcing the encoder (`jq -Rs '<filter>'`) to fail. Used by test 18 to
# force the encode-failure branch without short-circuiting the resolver
# stage that runs earlier in pr-update-body.sh.
#
# Reserved exit codes (must not collide with the script-under-test):
#   5  encode-mode simulated failure (the documented test signal)
#   97 no real jq locatable on FAKE_JQ_REAL_PATH
install_fake_jq() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/jq" <<'FAKE_JQ'
#!/usr/bin/env bash
# Mode of operation is decided by the first flag. -Rs forces the
# encoder branch to exit non-zero; any other invocation delegates to
# the real jq located via FAKE_JQ_REAL_PATH (snapshotted before the
# fake bin was prepended). Self-detection uses `-ef` (inode equality)
# so macOS symlink-vs-realpath PATH mismatches can't cause exec-loops.
case "${1:-}" in
  -Rs)
    echo "fake-jq: simulated encode failure" >&2
    exit 5
    ;;
  *)
    real_jq=""
    IFS=":" read -ra parts <<<"${FAKE_JQ_REAL_PATH:-$PATH}"
    for dir in "${parts[@]}"; do
      [ -n "$dir" ] || continue
      candidate="$dir/jq"
      if [ -x "$candidate" ] && ! [ "$candidate" -ef "${BASH_SOURCE[0]}" ]; then
        real_jq="$candidate"
        break
      fi
    done
    if [ -z "$real_jq" ]; then
      echo "fake-jq: no real jq found on FAKE_JQ_REAL_PATH" >&2
      exit 97
    fi
    exec "$real_jq" "$@"
    ;;
esac
FAKE_JQ
  chmod +x "$bin_dir/jq"
}

# setup_fake_jq <bin-dir>
# Installs install_fake_jq into <bin-dir>. Snapshots the current PATH
# into FAKE_JQ_REAL_PATH BEFORE prepending the fake's bin so the
# delegation can locate the real jq. Call after setup_gh_stub so both
# fakes co-exist on PATH.
#
# Preflight: asserts a real jq is locatable on the snapshotted PATH
# before installing the fake. Returns 1 (without `exit`) if jq is
# absent, so the caller can convert the missing-dependency case into a
# skip rather than a hard harness abort under `set -e`.
#
# Caller idiom (REQUIRED — direct invocation under `set -e` would
# terminate the entire harness if jq is missing):
#
#   if ! setup_fake_jq "$tmpdir/jqbin"; then
#     skip_test "test 18" "real jq required for fake-jq delegation"
#     return  # or `continue` / early-return from the test function
#   fi
#
# Single-use contract: must be called at most once per shell process
# with a clean PATH (no prior fake-jq prepended). The harness's
# per-test tempdir + subshell pattern enforces this naturally.
setup_fake_jq() {
  local bin_dir="$1"
  if ! command -v jq >/dev/null 2>&1; then
    echo "setup_fake_jq: real jq required on PATH for delegation fallback" >&2
    return 1
  fi
  export FAKE_JQ_REAL_PATH="$PATH"
  install_fake_jq "$bin_dir"
  export PATH="$bin_dir:$PATH"
}
```

Each harness sources both helpers from its own script directory.
Example header for `skills/github/scripts/test-pr-base-repo-scripts.sh`:

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=/dev/null
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/test-helpers.sh"
```

For `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`,
adjust the second `source` to `"$SCRIPT_DIR/../../scripts/test-helpers.sh"`
and the `PLUGIN_ROOT` computation to climb four levels.

#### 4. Resolver test harness

**File**: `skills/github/scripts/test-pr-base-repo-scripts.sh` (new)
**Changes**: Source `test-helpers.sh`; install the gh stub per-test;
exercise every resolver scenario. Each test uses a fresh tempdir.

Test cases (each maps to a behaviour the resolver must guarantee):

1. **Script is executable** — tripwire against missing chmod.
2. **Usage: zero args** — exits 2; stderr contains `Usage:`.
3. **Same-repo PR resolves** — stub returns
   `{"baseRepository":{"owner":{"login":"acme"},"name":"app"}}`; assert
   stdout is `acme/app\n` and exit 0.
4. **Cross-fork PR resolves to upstream** — stub returns
   `{"baseRepository":{"owner":{"login":"upstream-org"},"name":"upstream-repo"}}`;
   assert stdout is `upstream-org/upstream-repo\n` (and not the fork's
   coordinates). Covers AC3.
5. **Argv shape** — assert recorded argv is exactly
   `pr view 119 --json baseRepository` (full-line match; not substring).
   The resolver runs jq locally on the captured payload rather than
   delegating to `gh --jq`, so the recorded argv has no `--jq` flag.
6. **Resolver failure — stderr preserved, conditional hint** — stub's
   `gh pr view` exits 1 with stderr `no default remote repository`;
   assert script exits 1, stderr contains the literal phrase
   `no default remote repository` (preserved from `gh`), and stderr
   also contains the `gh repo set-default` remediation.
7. **Resolver failure — non-matching stderr, no false hint** — stub
   exits 1 with stderr `HTTP 403: SSO required`; assert script exits 1,
   stderr contains `HTTP 403: SSO required`, and stderr does **not**
   contain `gh repo set-default` (the hint must be conditional, not
   unconditional).
8. **Null owner guard** — stub returns
   `{"baseRepository":{"owner":{"login":null},"name":"app"}}`; assert
   script exits 1 with stderr explaining the field is missing (and
   does NOT print `null/app` to stdout).
9. **Null name guard** — symmetric to test 8.
10. **Missing jq preflight** — set `PATH="$tmpdir/bin"` (full
    replacement, not prepend) so the only thing on PATH is the
    fake-gh bin-dir with no `jq`; assert exit 2 with stderr
    `jq is required`. Prepending alone wouldn't hide system jq.
11. **Missing baseRepository field** — stub returns `{}` (no
    `baseRepository` key); assert exit 1 with stderr explaining the
    field is missing.
12. **Non-JSON stdout** — stub `pr view` exits 0 but prints an HTML
    error page or plain-text auth nag to stdout; assert exit 1 with
    a clear error rather than an opaque jq parse error (the resolver
    may need to wrap the jq calls with `2>/dev/null || err_msg` to
    surface a useful message — adjust the implementation if needed).

#### 5. Describe-pr helper test harness

**File**: `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` (new)
**Changes**: Source both the repo-root `scripts/test-helpers.sh`
(assertions) and `../../scripts/test-helpers.sh` (the github-skills
fake-gh factory) independently, using the `SCRIPT_DIR`/`PLUGIN_ROOT`
idiom shown in section 3 above. Each test uses a fresh tempdir and
`setup_gh_stub`.

Test cases:

1. **Script is executable** — tripwire.
2. **Usage at 0 args** — exit 2, stderr `Usage:`.
3. **Usage at 1 arg** — exit 2, stderr `Usage:` (boundary).
4. **Usage at 3 args** — exit 2, stderr `Usage:` (boundary).
5. **Missing body file** — exit 2, stderr names the missing file.
6. **Same-repo PR — resolver-argv** — stub `pr view` returns acme/app;
   assert `pr view` argv recorded with correct flags.
7. **Cross-fork PR — PATCH URL targets upstream** — stub returns
   upstream coordinates; assert recorded `gh api` argv is exactly
   `api --method PATCH repos/upstream-org/upstream-repo/pulls/119 --input <path>`
   (full-line match; URL targets upstream, NOT the fork). Covers AC3.
8. **PATCH method explicit** — assert recorded `gh api` argv contains
   `--method PATCH` (not relying on the default).
9. **JSON body encoding — empty body** — body file is empty; assert
   recorded stdin to `gh api`, when read back through `jq -r .body`,
   equals the empty input file byte-for-byte (i.e. `{"body":""}` was
   sent).
10. **JSON body encoding — multi-line** — body contains `Hello\n\nWorld\n`;
    assert round-trip equality via `jq -r .body` of recorded stdin.
11. **JSON body encoding — shell metacharacters** — body contains
    backticks, `$()`, `"`, `'`, `\`; assert round-trip equality.
    Covers AC6.
12. **JSON body encoding — unicode** — body contains an emoji and an
    accented character; assert round-trip equality.
13. **JSON body encoding — no trailing newline** — body has no final
    `\n`; assert round-trip equality (jq -Rs preserves this).
14. **Stdin pipe via `--input <file>`** — assert the recorded `gh api`
    argv (from `$GH_ARGV_LOG`) contains `--input <path>` where
    `<path>` is not `-`, and assert `$GH_STDIN_LOG` is non-empty.
    Readability-at-invocation-time is verified indirectly: the
    fake-gh reads the `--input` path's contents into `$GH_STDIN_LOG`
    synchronously while `gh api` is running, so a non-empty log
    means the file was readable when `gh api` opened it. (The
    helper's EXIT trap unlinks the tempfile before assertions run,
    so direct filesystem inspection is not possible.)
15. **Tempfile cleanup on success** — after a successful run, assert
    the controlled `$TMPDIR` (set per-test by `setup_gh_stub`) is
    empty (`[ -z "$(ls -A "$TMPDIR")" ]`). This verifies the trap
    removed the encoder-tempfile.
16. **Tempfile cleanup on PATCH failure** — same `$TMPDIR` empty
    check after `gh api` exits non-zero.
17. **Resolver failure propagated** — stub `pr view` exits 1 with
    stderr `no default remote repository`; assert script exits 1
    (the resolver's own resolution-failure exit code, preserved
    verbatim by `pr-update-body.sh`), stderr contains the preserved
    gh stderr, and stderr contains `gh repo set-default`.
18. **Encode failure exit code** — invoke `setup_fake_jq` via the
    documented caller idiom so a missing jq becomes a skip rather
    than a harness abort:

    ```bash
    if ! setup_fake_jq "$tmpdir/jqbin"; then
      skip_test "test 18" "real jq required for fake-jq delegation"
      return
    fi
    ```

    `setup_fake_jq` (defined in `skills/github/scripts/test-helpers.sh`)
    installs a fake `jq` that delegates to the real `jq` for `-r`
    filters (so the resolver stage still succeeds) but exits 5 with
    stderr `fake-jq: simulated encode failure` when invoked with
    `-Rs` (the encoder's flag). Configure `GH_PR_VIEW_OUT` so the
    resolver can complete; then assert the helper exits 1 with
    stderr containing `encode failed` and the captured `fake-jq`
    stderr. Sanity check: `$GH_ARGV_LOG` must contain a `pr view`
    line (confirming the resolver reached gh) but no `api` line
    (confirming PATCH was never attempted).
19. **PATCH failure — stage-specific stderr** — stub `gh api` exits 1
    with stderr `HTTP 422: Validation Failed`; assert script exits 4
    with stderr containing `HTTP 422` and a stage-specific message
    naming `PATCH` (not just generic "failed").
20. **PATCH success — exit 0 AND PATCH was actually called** — both
    stubs return 0; assert script exits 0, and assert `$GH_ARGV_LOG`
    contains exactly one line starting with `pr view` and exactly one
    line starting with `api`.
21. **jq preflight** — see test 10 in the resolver harness; similar
    coverage here.
**Tree-state regression guards (Phase-conditional expectations)**:

22. **Regression guard — `gh pr edit` not present in skills/** — assert
    `grep -r 'gh pr edit' "$PLUGIN_ROOT/skills/"` returns no matches.
    Use the `PLUGIN_ROOT` computed in the harness header so the test
    is CWD-independent.
    **Expected state**: RED in Phases 1-3 (the existing SKILL.md still
    contains `gh pr edit`); GREEN from Phase 4 onwards.
23. **Regression guard — cross-fork-unsafe resolver not present in
    skills/github/** — assert
    `grep -rn 'gh repo view --json owner,name' "$PLUGIN_ROOT/skills/github/"`
    returns no matches. Prevents future re-introduction of the
    cross-fork-unsafe pattern.
    **Expected state**: RED in Phases 1-5 (review-pr and respond-to-pr
    still contain the call); GREEN from Phase 6 onwards.

Tests 22 and 23 are tree-state guards: they assert facts about the
repository file contents, not about the helper script's behaviour, so
their pass/fail state depends on which phase the tree is in. The
harness reports them with their expected-state for the current phase
(`PHASE` env var, defaulting to `final`); other phases short-circuit
them to PASS with a `(deferred — guards Phase N)` note so the harness
overall can be GREEN at every phase boundary. Implementation sketch:

Both regression-guard tests scope their grep to `*.md` files only.
The forbidden patterns (`gh pr edit`, `gh repo view --json owner,name`)
should only ever appear in SKILL.md prose — scoping by extension
(a) prevents the harness file itself from matching its own
`assert_grep_empty` arguments, and (b) excludes shell/python source
where the strings might legitimately appear in comments or test
fixtures.

```bash
# Validate PHASE once at harness start so subsequent tests can rely
# on the value being well-formed and skips/enforce flows compose
# cleanly with test_summary. fail_loud_phase is a one-line wrapper
# around emitting a clear stderr and exit 2 if PHASE is malformed.
phase="${PHASE:-final}"
case "$phase" in
  1|2|3|4|5|6|final) : ;;
  *) echo "unknown PHASE: $phase (expected 1-6 or final)" >&2; exit 2 ;;
esac

# Test 22 — regression guard against gh pr edit.
case "$phase" in
  1|2|3) skip_test "test 22" "deferred — guards Phase 4" ;;
  4|5|6|final)
    assert_grep_empty "test 22" \
      "$PLUGIN_ROOT/skills/" "gh pr edit" \
      --include='*.md'
    ;;
esac

# Test 23 — regression guard against cross-fork-unsafe resolver.
case "$phase" in
  1|2|3|4|5) skip_test "test 23" "deferred — guards Phase 6" ;;
  6|final)
    assert_grep_empty "test 23" \
      "$PLUGIN_ROOT/skills/github/" "gh repo view --json owner,name" \
      --include='*.md'
    ;;
esac
```

PHASE validation is moved to a single early gate (top of the
harness) rather than duplicated in each test's case statement. This
means malformed PHASE values still fail loudly at exit 2 but do so
before any tests run, so `test_summary` is not skipped and the
harness's earlier passes are not orphaned.

The `mise run test:integration:github` invocation passes `PHASE`
through; CI runs with `PHASE=final` so both guards are enforced at
merge time. Local phase-by-phase development can override via
`PHASE=3 mise run test:integration:github`.

#### 6. Area-level helper-directory README

**File**: `skills/github/scripts/README.md` (new)
**Changes**: Two-sentence explainer so future contributors don't
have to reverse-engineer the placement convention:

```markdown
# skills/github/scripts

Helpers shared by two or more github skills (currently
`pr-base-repo.sh` is shared by describe-pr, review-pr, and
respond-to-pr; `test-helpers.sh` is shared by both harnesses in
this area). Helpers specific to one skill live under that skill's
own `scripts/` directory (e.g.
`skills/github/describe-pr/scripts/pr-update-body.sh`).
```

#### 7. mise task, invoke backing, and aggregate sequencing note

**File**: `tasks/test/integration.py`
**Changes**: Add a `github` invoke task that calls
`run_shell_suites(context, 'skills/github')`. Pattern matches the
existing `decisions`, `config`, `visualiser`, `binary_acquisition`
tasks.

**File**: `mise.toml`
**Changes**: Add the task entry under the existing `[tasks."test:integration:*"]`
block, and append to `[tasks."test:integration"].depends` using the
established multi-line array formatting at lines 114-122. Also pin `jq`
in `[tools]` for parity with the other version-pinned entries.

```toml
[tasks."test:integration:github"]
description = "Run github skills integration tests (shell harnesses)"
run = "invoke test.integration.github"
env = { PHASE = "{{ env.PHASE | default(value=\"final\") }}" }

# In [tools], add:
jq = "1.7.1"

# Add "test:integration:github" to the existing
# [tasks."test:integration"].depends array.
```

The `run = "invoke ..."` form (without `uv run --project tasks` prefix)
matches every existing `test:integration:*` entry at mise.toml lines
101/105/109/113/117 — mise's environment activation handles uv/venv,
so the prefix is unnecessary.

The explicit `env.PHASE` template ensures the phase-conditional
guards work whether or not the caller supplies `PHASE`: it defaults
to `final` (so CI enforces all guards) but honours an override (so
`PHASE=3 mise run test:integration:github` deferred-pass-throughs
tests 22 and 23). This avoids depending on implicit env-inheritance
through the mise→invoke→bash chain.

**Aggregate stability between phases**: adding the new task to
`[tasks."test:integration"].depends` in Phase 1 means the aggregate
`mise run test:integration` will be RED between Phase 1 and Phase 3
completion. Either land Phases 1–3 as a single squashed commit, or
gate the `depends` addition until Phase 3 — pick one explicitly when
sequencing commits.

#### 8. Phase 1 verification: tests are RED

Run `PHASE=1 mise run test:integration:github`; confirm every
behavioural test fails (the stubs are no-ops) and tests 22/23
report as `(deferred — guards Phase 4 / Phase 6)`. The harness
output should show every behavioural assertion as FAIL, none as
unexpected PASS — sanity-check that the stubs really are no-ops.

### Success Criteria:

#### Automated Verification:

- [ ] Stub scripts exist and are executable:
  - `test -x skills/github/scripts/pr-base-repo.sh`
  - `test -x skills/github/describe-pr/scripts/pr-update-body.sh`
- [ ] Test harnesses exist and are executable.
- [ ] `scripts/test-helpers.sh` exposes `skip_test` and
  `assert_grep_empty` (added in section 0 of this phase).
- [ ] `skills/github/scripts/test-helpers.sh` exists with
  `install_fake_gh`, `setup_gh_stub`, `install_fake_jq`, and
  `setup_fake_jq`.
- [ ] Mise task entry includes the explicit
  `env = { PHASE = "..." }` block so PHASE wiring is independent
  of mise→invoke env inheritance.
- [ ] `skills/github/scripts/README.md` documents the area-level vs
  per-skill helper convention.
- [ ] mise task is registered: `mise tasks | grep test:integration:github`.
- [ ] `jq` is pinned in `mise.toml [tools]` (e.g. `jq = "1.7.1"`).
- [ ] `PHASE=1 mise run test:integration:github` exits non-zero
  (RED — behavioural tests fail because the scripts are stubs;
  tests 22 and 23 report as deferred and pass-through).
- [ ] The harness output shows **every** behavioural assertion as
  FAIL, none as unexpected PASS, and tests 22/23 explicitly marked
  as `deferred (guards Phase N)`.
- [ ] `shellcheck skills/github/scripts/pr-base-repo.sh
  skills/github/describe-pr/scripts/pr-update-body.sh
  skills/github/scripts/test-helpers.sh
  skills/github/scripts/test-pr-base-repo-scripts.sh
  skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
  reports no issues.

#### Manual Verification:

- [ ] Reviewer can confirm by inspection that each test case in both
  harnesses maps to an acceptance criterion or a technical-note
  convention from the work item.

---

## Phase 2: GREEN — Shared Base-Repo Resolver

### Overview

Implement `skills/github/scripts/pr-base-repo.sh`. The resolver test
harness turns fully green. The describe-pr harness remains red (its
posting path isn't wired yet) except for any tests that only exercise
the resolver-passthrough.

### Changes Required:

#### 1. Resolver implementation

**File**: `skills/github/scripts/pr-base-repo.sh`
**Changes**: Replace the stub body with full resolution logic.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Usage: pr-base-repo.sh <pr-number>
# (Header comment as documented in Phase 1, plus the conventions
# captured below — kept in the file so future maintainers can find them
# without chasing ADR-0010 + this plan.)
#
# Conventions:
# - Cross-fork-safe: resolves via `gh pr view --json baseRepository`.
#   `gh repo view` returns the local checkout's repo (the fork, for
#   contributors), which is wrong for cross-fork PR operations.
# - Preserves the underlying gh stderr on failure so callers see the
#   real cause; emits a conditional `gh repo set-default` remediation
#   only when the captured stderr matches the known phrase.
# - Validates that owner.login and name are non-empty so a degenerate
#   gh response can't smuggle "null/null" downstream.
#
# Invocation: must be run as a subprocess (e.g. via command
# substitution or direct execution). The EXIT trap on the internal
# err_file would clobber a caller's own EXIT trap if this script were
# `source`d. All current callers spawn a subshell, which is safe.

if [ $# -ne 1 ]; then
  echo "Usage: pr-base-repo.sh <pr-number>" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "pr-base-repo.sh: jq is required (install via Homebrew, apt, or mise)" >&2
  exit 2
fi

pr_number="$1"

# Resolve the base (upstream) repo. Capture stderr to a tempfile so we
# can replay it on failure rather than silently substituting our own
# remediation hint.
err_file=$(mktemp)
trap 'rm -f "$err_file"' EXIT

if ! payload=$(gh pr view "$pr_number" --json baseRepository 2>"$err_file"); then
  if [ -s "$err_file" ]; then
    cat "$err_file" >&2
  fi
  echo "pr-base-repo.sh: could not resolve base repo for PR #$pr_number." >&2
  if grep -q "no default remote repository" "$err_file"; then
    echo "  Run 'gh repo set-default' and select the appropriate repository." >&2
  fi
  exit 1
fi

# Pre-validate that gh returned parseable JSON. Without this, a
# malformed response (HTML auth-nag, plain-text proxy error) reaches
# the jq calls below and produces opaque parse errors instead of the
# helper's own remediation.
if ! jq -e . >/dev/null 2>&1 <<<"$payload"; then
  echo "pr-base-repo.sh: gh returned non-JSON output for PR #$pr_number." >&2
  echo "  Raw payload: $payload" >&2
  exit 1
fi

# Extract and validate owner/name without smuggling "null/null" downstream.
owner=$(jq -r '.baseRepository.owner.login // ""' <<<"$payload")
name=$(jq -r '.baseRepository.name // ""' <<<"$payload")

if [ -z "$owner" ] || [ -z "$name" ]; then
  echo "pr-base-repo.sh: baseRepository.owner.login or .name was empty/null in gh response." >&2
  echo "  Raw payload: $payload" >&2
  exit 1
fi

printf '%s/%s\n' "$owner" "$name"
```

### Success Criteria:

#### Automated Verification:

- [ ] All tests in
  `skills/github/scripts/test-pr-base-repo-scripts.sh` pass.
- [ ] `mise run test:integration:github` exits non-zero overall (the
  describe-pr harness is still red) but the resolver subset is fully
  green.
- [ ] `shellcheck skills/github/scripts/pr-base-repo.sh` reports no
  issues.

#### Manual Verification:

- [ ] None for this phase — purely automated test progression.

---

## Phase 3: GREEN — Describe-pr Helper

### Overview

Implement `pr-update-body.sh` on top of `pr-base-repo.sh`. Encode the
body to a tempfile via `jq -Rs '{body: .}'`, then PATCH via
`gh api --method PATCH ... --input <tempfile>`. All describe-pr tests
turn green.

### Changes Required:

#### 1. Helper implementation

**File**: `skills/github/describe-pr/scripts/pr-update-body.sh`
**Changes**: Replace the stub body with full posting logic.

```bash
#!/usr/bin/env bash
set -euo pipefail

# (Header comment as documented in Phase 1.)
#
# Conventions:
# - Body encoded as JSON via `jq -Rs '{body: .}'` to a tempfile so
#   encode and PATCH failures can be distinguished cleanly.
# - PATCH targets the base (upstream) repo per the shared resolver.
# - Explicit `--method PATCH` (gh api's default is GET, or POST when a
#   body is present — relying on the default is brittle).

if [ $# -ne 2 ]; then
  echo "Usage: pr-update-body.sh <pr-number> <body-file>" >&2
  exit 2
fi

pr_number="$1"
body_file="$2"

if [ ! -f "$body_file" ]; then
  echo "pr-update-body.sh: body file not found: $body_file" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "pr-update-body.sh: jq is required (install via Homebrew, apt, or mise)" >&2
  exit 2
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
resolver="$script_dir/../../scripts/pr-base-repo.sh"

# Capture the resolver's exit code explicitly so callers can
# distinguish resolver-usage (2) from resolver-resolution (1) failures.
if base_repo=$("$resolver" "$pr_number"); then
  :
else
  resolver_rc=$?
  # Resolver already emitted its own stderr (preserved gh stderr +
  # conditional hint). Re-emit a contextual line and propagate the
  # resolver's exit code verbatim.
  echo "pr-update-body.sh: base-repo resolution failed for PR #$pr_number." >&2
  exit "$resolver_rc"
fi

# Allocate one tempdir for all stage artefacts so a single trap
# covers cleanup unconditionally, including any mktemp-failure path.
stage_dir=$(mktemp -d)
trap 'rm -rf "$stage_dir"' EXIT
payload_file="$stage_dir/payload"
encode_err="$stage_dir/encode.err"
patch_err="$stage_dir/patch.err"

# Encode the body. Capture jq's stderr so real failures (not just
# the test-injected fake-jq) surface verbatim — mirrors the
# resolver/PATCH diagnostic-preservation pattern.
if ! jq -Rs '{body: .}' <"$body_file" >"$payload_file" 2>"$encode_err"; then
  if [ -s "$encode_err" ]; then
    cat "$encode_err" >&2
  fi
  echo "pr-update-body.sh: encode failed for $body_file (could not read or JSON-encode the file)." >&2
  exit 1
fi

# Capture gh api stderr similarly so HTTP 422 / rate-limit text
# reaches the user alongside the contextual line.
if ! gh api --method PATCH "repos/$base_repo/pulls/$pr_number" --input "$payload_file" 2>"$patch_err"; then
  if [ -s "$patch_err" ]; then
    cat "$patch_err" >&2
  fi
  echo "pr-update-body.sh: PATCH failed for repos/$base_repo/pulls/$pr_number." >&2
  exit 4
fi
```

Resolver-failure exit-code propagation: `pr-update-body.sh` preserves
the resolver's exit code verbatim. The Phase 1 `Exit codes` block in
the header comment documents this: codes 1 (encode) and 4 (PATCH) are
owned by `pr-update-body.sh`; any non-zero exit other than 4 may be
bubbled up from `pr-base-repo.sh` (resolver 1 = resolution failed,
2 = usage / missing jq). Exit code 1 is thus shared between the
helper's own encode-failure and the resolver's resolution-failure;
the stderr prefix disambiguates.

All stage tempfiles live inside a single `mktemp -d` directory so a
single EXIT trap covers cleanup unconditionally — no mktemp-failure
race window leaves tempfiles behind. Each stage that shells out
(jq encode, gh api PATCH) captures stderr to its own file inside
that directory and replays it on failure before emitting the
helper's contextual line, matching the resolver's
preserve-and-replay pattern for diagnostic visibility.

### Success Criteria:

#### Automated Verification:

- [ ] All behavioural tests in
  `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
  pass. Tree-state regression guards (tests 22 and 23) remain
  deferred per their phase-conditional skip logic — `PHASE=3 mise
  run test:integration:github` exits 0, with tests 22/23 reported
  as deferred (guards Phase 4 / Phase 6).
- [ ] `PHASE=3 mise run test:integration` exits 0.
- [ ] `shellcheck skills/github/describe-pr/scripts/pr-update-body.sh`
  reports no issues.

#### Manual Verification:

- [ ] None for this phase — purely automated test progression.

---

## Phase 4: Wire describe-pr SKILL.md to the Helper

### Overview

Swap the broken `gh pr edit` line in `describe-pr/SKILL.md` step 9 for
an invocation of the new helper. Preserve the frontmatter-strip recipe
and tmp cleanup. Extend `allowed-tools`.

### Changes Required:

#### 1. SKILL.md step 9

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**: Replace line 130's `gh pr edit ...` invocation with a
`pr-update-body.sh` invocation. The surrounding sub-steps
(frontmatter strip 1–4 and cleanup 6) are unchanged.

Replacement sub-step 5:

```markdown
  5. Post the body via the helper script, which resolves the base
     (upstream) repository for cross-fork safety and PATCHes via the
     GitHub REST API:
     `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh {number} {tmp directory}/pr-body-{number}.md`
     If the helper exits non-zero, surface its stderr verbatim to the
     user — it includes preserved `gh` error text and, where applicable,
     a `gh repo set-default` remediation hint. Exit codes:
     - **Exit 1** → encode failed (`pr-update-body.sh:` stderr prefix)
       OR resolver-resolution failed (`pr-base-repo.sh:` prefix)
     - **Exit 2** → usage error / missing jq
     - **Exit 4** → PATCH failed
     The stderr prefix identifies which stage failed when exit code
     alone is ambiguous.
```

Sub-step 6 (cleanup) remains exactly:

```markdown
  6. Clean up `{tmp directory}/pr-body-{number}.md`
```

#### 2. SKILL.md allowed-tools frontmatter

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**: Extend `allowed-tools`.

```yaml
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)
```

### Success Criteria:

#### Automated Verification:

- [ ] `grep -n "gh pr edit" skills/github/describe-pr/SKILL.md` returns
  no matches.
- [ ] `grep -n "pr-update-body.sh" skills/github/describe-pr/SKILL.md`
  returns exactly one match in step 9.
- [ ] `diff` confirms the frontmatter-strip recipe at lines 119-129 is
  byte-identical.
- [ ] `PHASE=4 mise run test:integration:github` exits 0; test 22
  (gh pr edit regression guard) is now ENFORCED (not deferred) and
  passes. Test 23 remains deferred (guards Phase 6).
- [ ] `PHASE=4 mise run test:integration` continues to pass.

#### Manual Verification:

- [ ] Inspection confirms AC4 (frontmatter stripped before body posted)
  is still satisfied by sub-steps 1–4 preceding the new sub-step 5.
- [ ] Inspection confirms the SKILL.md prose tells the model to surface
  the helper's stderr verbatim (so preserved-gh-error text and the
  conditional `gh repo set-default` hint reach the user).

---

## Phase 5: Migrate review-pr to the Shared Resolver

### Overview

Replace the cross-fork-unsafe `gh repo view --json owner,name` call at
`review-pr/SKILL.md:117` with `pr-base-repo.sh`. Update the
error-handling prose at lines 130-131 to reflect the new remediation
path. Extend `allowed-tools`.

### Changes Required:

#### 1. SKILL.md step 1.5

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Line 117 currently reads:

```
gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"' > {tmp directory}/pr-review-{number}/repo-info.txt
```

Replace with:

```
${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh {number} > {tmp directory}/pr-review-{number}/repo-info.txt
```

#### 2. SKILL.md error-handling prose

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Lines 130-131 currently read:

```
- **Cannot determine repo owner/name**: If `gh repo view` fails, instruct the
  user to run `gh repo set-default` and select the appropriate repository.
```

Replace with:

```
- **Cannot determine base repo owner/name**: If `pr-base-repo.sh` exits
  non-zero, surface its stderr verbatim — it preserves the underlying
  `gh` error and includes a `gh repo set-default` remediation hint when
  applicable.
```

#### 3. SKILL.md allowed-tools

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Extend `allowed-tools` with
`- Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)`.

### Success Criteria:

#### Automated Verification:

- [ ] `grep -n "gh repo view --json owner,name" skills/github/review-pr/SKILL.md`
  returns no matches.
- [ ] `grep -n "pr-base-repo.sh" skills/github/review-pr/SKILL.md`
  returns at least one match.
- [ ] `PHASE=5 mise run test:integration:github` exits 0. Test 23
  remains deferred (still requires Phase 6 to eliminate the call
  from `respond-to-pr/SKILL.md`).
- [ ] `PHASE=5 mise run test:integration` passes.

#### Manual Verification:

- [ ] Inspection confirms the surrounding step-1.5 flow (writing to
  `repo-info.txt` and downstream consumers reading from it) is
  preserved.

---

## Phase 6: Migrate respond-to-pr to the Shared Resolver

### Overview

Replace the cross-fork-unsafe `gh repo view --json owner,name` call at
`respond-to-pr/SKILL.md:67`. Extend `allowed-tools`.

### Changes Required:

#### 1. SKILL.md step 3

**File**: `skills/github/respond-to-pr/SKILL.md`
**Changes**: Line 67 currently reads:

```
gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'
```

Replace with:

```
${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh {number}
```

(Where `{number}` is the PR number captured in step 1.)

#### 2. SKILL.md allowed-tools

**File**: `skills/github/respond-to-pr/SKILL.md`
**Changes**: Extend `allowed-tools` with
`- Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)`.

### Success Criteria:

#### Automated Verification:

- [ ] `grep -n "gh repo view --json owner,name" skills/github/respond-to-pr/SKILL.md`
  returns no matches.
- [ ] `grep -n "pr-base-repo.sh" skills/github/respond-to-pr/SKILL.md`
  returns at least one match.
- [ ] `grep -rn "gh repo view --json owner,name" skills/github/`
  returns no matches across the entire `skills/github/` tree.
- [ ] `mise run test:integration:github` exits 0 with no PHASE
  override (defaults to `final`); test 23 (cross-fork-unsafe resolver
  regression guard) is now ENFORCED and passes alongside test 22.
- [ ] `mise run test:integration` passes (final mode, all guards
  enforced).

#### Manual Verification:

- [ ] Inspection confirms the surrounding step-3 flow (using the
  resolved owner/name in subsequent gh-api calls) is preserved.

---

## Phase 7: Live-PR Manual Verification

### Overview

The work item's acceptance criteria include observable behaviour during
real skill invocations that cannot be asserted from shell tests: no
`gh pr edit` in the command trace, no deprecation-fallback note, and
the migrated sibling skills proceed without error.

### Changes Required:

#### 1. No file changes

All edits land in Phases 1-6. This phase is verification only.

### Success Criteria:

#### Automated Verification:

- [ ] None for this phase.

#### Manual Verification:

- [ ] **AC1 / AC7 — primary REST path on a same-repo PR**: pick any
  open PR on this repository; run `/describe-pr <number>`; observe the
  command trace; confirm (a) no `gh pr edit` invocation appears,
  (b) no deprecation-fallback note is emitted, (c) the PR body on
  GitHub updates to match the stripped content of
  `{prs directory}/{number}-description.md` byte-for-byte.
- [ ] **AC2 — transitive coverage via review-pr / respond-to-pr**:
  run `/review-pr <number>` and `/respond-to-pr <number>` against an
  open PR; confirm both proceed without error and that the resolved
  base-repo coordinates are correct.
- [ ] **AC3 — cross-fork PR**: if a fork-based PR against this repo is
  unavailable, simulate by invoking the resolver directly:
  `${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/pr-base-repo.sh <fork-pr-number>`
  against a known fork-based PR on a public repo where you have access.
  Confirm the resolver prints the **upstream** coordinates, not the
  fork's. Then invoke `pr-update-body.sh <fork-pr-number> <body-file>`
  and confirm via `gh api repos/{upstream-owner}/{upstream-repo}/pulls/{number} --jq .body`
  that the body landed on the upstream. **Residual risk if not live-
  verifiable**: the unit tests cover argv shape against a mocked `gh`
  but do not validate that real `gh api` correctly addresses the
  upstream from a forked checkout, nor auth-context behaviour against
  the upstream. Document this in the PR description.
- [ ] **AC5 — cleanup**: after a run, confirm
  `{tmp directory}/pr-body-{number}.md` is removed (SKILL.md cleanup
  sub-step 6 still runs) AND the helper's encoder-tempfile is removed
  (test 15/16 cover this in unit, but spot-check `/tmp` after a run).

---

## Testing Strategy

### Unit / Integration Tests:

All Phase 1 test cases (resolver suite + describe-pr suite). The
harnesses use a PATH-stubbed `gh` to simulate every external
interaction:

- **Owner/repo resolution paths**: same-repo, cross-fork, the
  "no default remote" failure mode, the non-default-remote failure
  mode (where the conditional hint must NOT fire), and null-field
  guards.
- **Body encoding edge cases**: empty body, single-line, multi-line,
  shell metacharacters (backticks, `$()`, quotes, backslashes),
  unicode (emoji, accented chars), and no-trailing-newline. All use a
  round-trip assertion (`jq -r .body` of recorded stdin equals input
  file byte-for-byte) rather than literal JSON equality.
- **Exit-code propagation**: resolver failure (preserved verbatim
  from `pr-base-repo.sh`: 1 = resolution failed, 2 = usage / missing
  jq); encode failure (exit 1, owned by `pr-update-body.sh`); PATCH
  failure (exit 4); success (exit 0). Exit code 1 is shared by encode
  and resolver-resolution; stage attribution flows through the
  stderr prefix (`pr-update-body.sh:` vs `pr-base-repo.sh:`).
- **Argument validation**: missing args at 0/1/3 boundaries, missing
  body file, missing jq.
- **Regression guard**: a single assertion that
  `grep -r 'gh pr edit' skills/` returns no matches.

### Integration with real GitHub:

None automated. The script's only collaborator (`gh`) is mocked.
Integration is covered by manual verification on a live PR in Phase 7.

### Manual Testing Steps:

1. Run `/describe-pr <number>` against an open PR. Confirm no
   `gh pr edit` in the trace; confirm no fallback note; confirm the PR
   body on GitHub matches.
2. Run `/review-pr <number>` and `/respond-to-pr <number>` against an
   open PR; confirm the resolver step produces correct coordinates.
3. If a cross-fork PR is reachable, repeat (1) against it.
4. Trigger the resolver-failure path by detaching the default remote
   (`gh repo set-default --unset` in a clean clone) and re-running
   `pr-base-repo.sh` directly; confirm both the preserved gh stderr
   and the conditional `set-default` hint appear in stderr.
5. Trigger the non-default-remote failure (e.g. with an expired auth
   token in a sandbox env) and confirm the `set-default` hint does
   NOT fire spuriously.

## Performance Considerations

Not applicable. The fix replaces one `gh` invocation with up to two
per skill (`gh pr view` + `gh api` in describe-pr; `gh pr view` only
in review-pr and respond-to-pr), adding at most one extra API round-
trip per skill run. Both calls are sub-second in practice and the
skills are interactive.

## Migration Notes

Not applicable to end users. The change is internal to three skills'
instruction files and their companion helper scripts; there is no
on-disk data or persistent configuration to migrate. Users running
pre-fix versions continue to receive the deprecation-fallback note
until they pick up the fix; no rollout coordination is needed beyond
shipping the updated plugin.

For repository maintainers: `jq` becomes a declared dependency in
`mise.toml`. Anyone using mise to manage the toolchain picks it up
automatically; anyone without mise needs jq on `PATH` (the helpers
preflight and emit a clear remediation hint).

## References

- Work item: `meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md`
- Research: `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`
- Plan review: `meta/reviews/plans/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation-review-1.md`
- Edit site: `skills/github/describe-pr/SKILL.md:130`
- Sibling resolver sites: `skills/github/review-pr/SKILL.md:117`,
  `skills/github/respond-to-pr/SKILL.md:67`
- Existing frontmatter strip (preserve):
  `skills/github/describe-pr/SKILL.md:119-129`
- Existing cleanup (preserve): `skills/github/describe-pr/SKILL.md:131`
- Error-remediation precedent: `skills/github/describe-pr/SKILL.md:54-55`
- Test harness shape: `skills/decisions/scripts/test-adr-scripts.sh`
- Test helpers: `scripts/test-helpers.sh`
- mise integration-test wiring: `tasks/test/integration.py`,
  `mise.toml:114-122`
- jq preflight pattern: `skills/integrations/jira/scripts/jira-common.sh:225`
- ADR precedent: `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`
- Stdin-piped JSON posting precedent (no `jq -Rs` though):
  `skills/github/respond-to-pr/SKILL.md:468-472`
- JSON-wrapper-file `--input` posting precedent:
  `skills/github/review-pr/SKILL.md:571-574`
- Sunset notice (root cause):
  https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/
