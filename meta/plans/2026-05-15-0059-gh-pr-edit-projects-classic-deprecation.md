---
date: "2026-05-15T16:50:00+01:00"
type: plan
skill: create-plan
work-item: "0059"
status: draft
---

# gh pr edit → REST PATCH Migration in describe-pr Implementation Plan

## Overview

Replace the failing `gh pr edit` invocation in `skills/github/describe-pr/SKILL.md:130`
with a call to a new, tested shell helper that resolves the PR's **base**
(upstream) repository via `gh pr view --json baseRepository`, JSON-encodes the
body via `jq -Rs '{body: .}'`, and PATCHes the body through
`gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input -`. This
bypasses the Projects (classic) GraphQL path entirely (the root cause of the
bug) and is cross-fork-safe by construction.

The work follows strict test-driven development: a stub script and a complete
test harness with all acceptance scenarios land in Phase 1 against a
`gh`-stub installed on `PATH`, and tests are confirmed **red** before any
production logic is written. Phases 2 and 3 turn the tests green one resolver
and one posting path at a time. Phase 4 wires the SKILL.md to the helper.
Phase 5 covers the only acceptance criterion that cannot be automated
(observable absence of `gh pr edit` and the deprecation fallback note in a
live `/describe-pr` run).

## Current State Analysis

- **Single broken edit site**: `skills/github/describe-pr/SKILL.md:130` is the
  only live `gh pr edit` invocation in `skills/` (verified by research at
  `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`).
  `review-pr` and `respond-to-pr` reference `describe-pr` only via user-facing
  workflow narratives — they inherit the failure transitively, not via
  runtime shell-out.
- **No github-skills helper scripts exist**: `skills/github/` currently
  contains only three `SKILL.md` files. There is no `skills/github/*/scripts/`
  directory and no precedent for testable shell helpers in this area. The fix
  introduces the first one.
- **Strong codebase pattern for testable shell helpers**: `skills/work/scripts/`,
  `skills/decisions/scripts/`, and `skills/visualisation/visualise/scripts/`
  all follow the same shape: a per-script production file plus a companion
  `test-*-scripts.sh` harness that sources `scripts/test-helpers.sh` for
  `assert_eq`/`assert_contains`/`assert_exit_code` and friends.
- **mise unit-test wiring is per-area**: `mise.toml:73-95` defines
  `test:unit:visualiser`, `test:unit:frontend`, `test:unit:tasks`. There is no
  catch-all that auto-discovers `test-*.sh` files, so a new
  `test:unit:github` task is needed to enrol the new harness, with `test:unit`
  extended to depend on it.
- **No PATH-stubbed `gh` precedent**: tests in `skills/visualisation/visualise/scripts/`
  mock external binaries via per-test tempdirs and env-var injection
  (`ACCELERATOR_VISUALISER_BIN`), but no existing test PATH-stubs `gh`. A
  PATH-stub for `gh` is the cleanest mock here because the production script
  shells out to `gh` with no env hook.
- **Frontmatter strip and tmp cleanup are already in place** at
  `describe-pr/SKILL.md:119-129` and `:131` respectively. AC4 ("frontmatter is
  stripped before the body is posted") and the related cleanup criterion are
  already satisfied by the existing skill — the fix must preserve, not
  re-introduce, these steps.
- **Error-handling precedent at line 54-55** of the same skill already
  documents the "no default remote repository" remediation for `gh pr diff`;
  the new resolution step in the helper script needs to surface the same
  remediation when `gh pr view` fails.
- **ADR-0010** (`meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`)
  is the precedent for dropping to `gh api` when porcelain CLI is inadequate.
  It does not pin down body encoding, owner/repo resolution, or cross-fork
  handling, so this plan introduces those conventions on top of the ADR's
  spirit.

### Key Discoveries:

- `skills/github/describe-pr/SKILL.md:130` — sole edit site (the broken
  `gh pr edit` call).
- `skills/github/describe-pr/SKILL.md:119-129` — existing frontmatter strip;
  preserve.
- `skills/github/describe-pr/SKILL.md:131` — existing unconditional cleanup;
  preserve.
- `skills/github/describe-pr/SKILL.md:54-55` — existing "no default remote
  repository" remediation; mirror for the new resolution step.
- `scripts/test-helpers.sh` — shared assertion library
  (`assert_eq`/`assert_contains`/`assert_exit_code`/`assert_file_executable`/
  `test_summary`).
- `skills/decisions/scripts/test-adr-scripts.sh:1-40` — canonical shape for a
  per-skill test harness (sourcing test-helpers.sh, `mktemp -d` + trap
  cleanup, `setup_repo` helper).
- `mise.toml:73-95` — `test:unit:*` task pattern that the new
  `test:unit:github` task should follow.
- `skills/visualisation/visualise/scripts/test-launch-server.sh:1-50` —
  pattern for env-driven fake binaries; this plan adopts a similar but
  simpler PATH-stub variant for `gh`.

## Desired End State

After this plan is complete:

- `skills/github/describe-pr/scripts/pr-update-body.sh` exists, is
  executable, takes `<pr-number> <body-file>` as positional args, resolves
  the base repo via `gh pr view --json baseRepository`, and PATCHes the body
  via `gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input -`
  using a `jq -Rs '{body: .}'` stdin pipe. It exits 0 on success and
  non-zero with a clear stderr remediation hint on failure.
- `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` exists
  and exercises all acceptance scenarios (same-repo, cross-fork,
  multi-line/backtick/unicode body content, gh-failure paths) against a
  PATH-stubbed `gh`.
- `mise.toml` has a new `test:unit:github` task that runs the new harness;
  `test:unit` depends on it.
- `skills/github/describe-pr/SKILL.md` step 9 no longer references
  `gh pr edit`; line 130 (or its successor) invokes
  `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh`
  with the PR number and the stripped body file. Frontmatter strip
  (lines 119-129) and tmp cleanup (line 131) are preserved verbatim.
- `mise run test:unit:github` passes with all assertions green.
- `mise run test:unit` (the aggregate) continues to pass.
- A live `/describe-pr` run against any open PR posts the body cleanly with
  no `gh pr edit` invocation in the command trace and no
  "deprecation-fallback" note emitted (verified manually in Phase 5).

### How to verify it:

- `mise run test:unit:github` — all assertions green.
- `mise run test:unit` — full unit aggregate green.
- `grep -rn "gh pr edit" skills/` — returns no matches.
- Live `/describe-pr` against a same-repo PR — body posts, no fallback note.
- Live `/describe-pr` against a cross-fork PR (or a `gh api` simulation
  thereof) — PATCH targets `{upstream-owner}/{upstream-repo}/pulls/{number}`.

## What We're NOT Doing

- Not migrating `review-pr` or `respond-to-pr` to use the cross-fork-safe
  `gh pr view --json baseRepository` resolver. Their existing
  `gh repo view --json owner,name` is a latent bug for cross-fork posting
  but is out of scope for 0059 (see research §Open Questions; should be
  tracked as a follow-up work item).
- Not extracting a shared cross-skill `gh` wrapper. The new script lives
  under `skills/github/describe-pr/scripts/` only. If a future refactor
  generalises it, that is a separate work item.
- Not writing an ADR. ADR-0010's spirit covers the "drop to `gh api`"
  pattern; if a follow-up ADR is desired to capture the new
  `jq -Rs '{body: .}'`, `gh pr view --json baseRepository`, and
  `--method PATCH` conventions, that is a separate decision.
- Not removing or rewording the existing frontmatter-strip recipe in
  step 9. AC4 is already satisfied; the strip remains a procedural recipe
  in SKILL.md prose, not delegated to the script.
- Not changing how step 9 sources or writes the body file
  (`{tmp directory}/pr-body-{number}.md`). The helper script consumes that
  file as-is; the path remains under SKILL.md's control.
- Not adding `--method PATCH` retry logic, exponential backoff, or other
  resilience features beyond exit-code parity with a single PATCH call.

## Implementation Approach

Strict TDD red-green: Phase 1 lands an executable stub script (just `exit 1`)
**and** the full test harness covering every acceptance scenario, plus the
`test:unit:github` mise task that runs it. The phase confirms the harness
fails for the expected reasons (so we know the assertions are wired right).
Phase 2 implements the `gh pr view --json baseRepository` resolution and
its error remediation; the subset of tests covering resolution and the
cross-fork criterion turns green. Phase 3 implements the `jq -Rs` body
encoding and the `gh api --method PATCH ... --input -` call; the remaining
tests turn green. Phase 4 swaps the SKILL.md step 9 line and validates the
file-level acceptance criteria (frontmatter strip preserved, cleanup
preserved, no `gh pr edit` left). Phase 5 covers the live-PR manual
verification that no automation can substitute for.

The PATH-stubbed `gh` mock is a single script that dispatches on `$1 $2`
(e.g. `pr view`, `api`) and emits canned stdout/stderr from per-test
env-var-pointed files, recording its argv and stdin to log files so tests
can assert on both the request shape (URL, method, JSON payload) and the
exit-code behaviour.

---

## Phase 1: Skeleton, RED Tests, and mise Wiring

### Overview

Land the executable-but-incomplete script, the full test harness, the
mise task, and confirm tests are red. No production logic is written in
this phase — every assertion should fail because the stub does nothing,
not because the harness is wrong.

### Changes Required:

#### 1. Stub script

**File**: `skills/github/describe-pr/scripts/pr-update-body.sh` (new)
**Changes**: Create executable stub that prints a usage line to stderr and
exits 1. Marks intent without implementing logic.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Posts the contents of <body-file> as the body of pull request <pr-number>
# on the base (upstream) repository, using the GitHub REST API.
# Implementation arrives in Phases 2 and 3.

echo "pr-update-body.sh: not yet implemented" >&2
exit 1
```

#### 2. Test harness

**File**: `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` (new)
**Changes**: Source `scripts/test-helpers.sh`; install a `gh` stub on PATH;
exercise every acceptance scenario. Each test uses a fresh tempdir for
stub-state isolation. The structure mirrors
`skills/decisions/scripts/test-adr-scripts.sh:1-40`.

The `gh` stub dispatches on the first two argv elements:

```bash
# fake-gh.sh (written into TEMP_BIN_DIR by the harness)
#!/usr/bin/env bash
echo "$@" >> "$GH_ARGV_LOG"
if [ -n "${GH_STDIN_LOG:-}" ]; then
  cat >> "$GH_STDIN_LOG"
fi
case "$1 $2" in
  "pr view")
    if [ -n "${GH_PR_VIEW_OUT:-}" ] && [ -f "$GH_PR_VIEW_OUT" ]; then
      cat "$GH_PR_VIEW_OUT"
    fi
    exit "${GH_PR_VIEW_RC:-0}"
    ;;
  "api"*|"api ")
    if [ -n "${GH_API_OUT:-}" ] && [ -f "$GH_API_OUT" ]; then
      cat "$GH_API_OUT"
    fi
    if [ -n "${GH_API_ERR:-}" ] && [ -f "$GH_API_ERR" ]; then
      cat "$GH_API_ERR" >&2
    fi
    exit "${GH_API_RC:-0}"
    ;;
esac
```

Test cases (each maps to a specific acceptance criterion or research
finding):

1. **Script is executable** (file marker; tripwire against missing chmod).
2. **Same-repo PR — resolver output**: `gh pr view --json baseRepository`
   stub returns `{"baseRepository":{"owner":{"login":"acme"},"name":"app"}}`;
   assert script's argv recording includes
   `pr view 119 --json baseRepository`.
3. **Cross-fork PR — PATCH URL targets upstream base**: stub returns
   `{"baseRepository":{"owner":{"login":"upstream-org"},"name":"upstream-repo"}}`;
   assert the recorded `gh api` argv contains
   `repos/upstream-org/upstream-repo/pulls/119` (and NOT the head fork's
   repo path). Covers AC3.
4. **PATCH method**: assert the recorded `gh api` argv contains
   `--method PATCH` (not `POST`, not absent). Covers technical-note
   convention.
5. **JSON body encoding — basic**: body file contains
   `Hello\n\nWorld\n`; assert the recorded stdin payload equals
   `{"body":"Hello\n\nWorld\n"}` byte-for-byte (modulo JSON-acceptable
   whitespace).
6. **JSON body encoding — shell metacharacters**: body contains
   backticks, `$()`, `"`, `'`, `\`; assert these survive intact in the
   JSON body field (round-trip via `jq -r .body` of the recorded stdin
   equals the input file byte-for-byte). Covers AC6.
7. **JSON body encoding — unicode**: body contains an emoji and an
   accented character; assert round-trip equality.
8. **Stdin pipe via `--input -`**: assert the recorded `gh api` argv
   ends with `--input -`.
9. **Resolver failure — non-zero exit and remediation hint**: stub's
   `gh pr view` exits 1 with stderr "no default remote repository";
   assert script exits non-zero and stderr contains the
   `gh repo set-default` remediation phrase.
10. **PATCH failure — non-zero exit and stderr propagation**: stub's
    `gh api` exits 1 with stderr "HTTP 422"; assert script exits
    non-zero and `HTTP 422` appears in script stderr.
11. **PATCH success — script exits 0**: stub returns 0 from both calls;
    assert `assert_exit_code 0`.
12. **Body file missing — clear error**: invoke with a non-existent
    body file path; assert non-zero exit and stderr names the missing
    file.
13. **Wrong arg count — usage message**: invoke with zero args; assert
    non-zero exit and stderr contains `usage:`.

Each test must set `GH_ARGV_LOG`, `GH_STDIN_LOG`, and stub-output env
vars in a per-test tempdir; the harness's `EXIT` trap reaps everything.

#### 3. mise task

**File**: `mise.toml`
**Changes**: Add a new task and extend `test:unit`'s `depends` list.

```toml
[tasks."test:unit:github"]
description = "Run github skills unit tests (shell harnesses)"
run = "bash skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh"

[tasks."test:unit"]
description = "Run all unit tests in parallel"
depends = ["test:unit:visualiser", "test:unit:frontend", "test:unit:tasks", "test:unit:github"]
```

(The exact insertion point is `mise.toml:93-95`; the only edit to existing
text is appending `"test:unit:github"` to the `depends` array.)

### Success Criteria:

#### Automated Verification:

- [ ] Stub script exists and is executable:
  `test -x skills/github/describe-pr/scripts/pr-update-body.sh`
- [ ] Test harness exists and is executable:
  `test -x skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
- [ ] mise task is registered: `mise tasks | grep test:unit:github`
- [ ] `mise run test:unit:github` exits non-zero (RED — tests fail as
  expected because the script is a stub).
- [ ] The harness output shows **every** assertion as FAIL, none as
  unexpected PASS (sanity check that the stub really is a no-op).
- [ ] `shellcheck skills/github/describe-pr/scripts/pr-update-body.sh
  skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
  reports no issues (the codebase already requires shellcheck on
  shell scripts via `mise.toml:[tools]`).

#### Manual Verification:

- [ ] Reviewer can confirm by inspection that each test case in the
  harness maps to an acceptance criterion or a technical-note
  convention from the work item.

---

## Phase 2: GREEN — Base-Repo Resolver

### Overview

Implement the `gh pr view --json baseRepository` resolver and its error
remediation. The subset of tests covering owner/repo resolution, the
cross-fork PATCH URL, and the "no default remote" error path turns green.
PATCH-call tests stay red (no PATCH wiring yet).

### Changes Required:

#### 1. Resolver in script

**File**: `skills/github/describe-pr/scripts/pr-update-body.sh`
**Changes**: Replace the stub body with argument validation and base-repo
resolution. Leave the PATCH call as a `TODO` exit-1 so Phase 3 tests
remain red.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Posts the contents of <body-file> as the body of pull request <pr-number>
# on the base (upstream) repository, using the GitHub REST API.

if [ $# -ne 2 ]; then
  echo "usage: pr-update-body.sh <pr-number> <body-file>" >&2
  exit 2
fi

pr_number="$1"
body_file="$2"

if [ ! -f "$body_file" ]; then
  echo "pr-update-body.sh: body file not found: $body_file" >&2
  exit 2
fi

# Resolve the base (upstream) repo. gh pr view --json baseRepository works
# correctly for both same-repo and cross-fork PRs — unlike gh repo view,
# which returns the local checkout's repo (the fork, for contributors).
if ! base_repo=$(gh pr view "$pr_number" --json baseRepository \
    --jq '"\(.baseRepository.owner.login)/\(.baseRepository.name)"' \
    2>/dev/null); then
  echo "pr-update-body.sh: could not resolve base repo for PR #$pr_number." >&2
  echo "  If this is the 'no default remote repository' error, run" >&2
  echo "  'gh repo set-default' and select the appropriate repository." >&2
  exit 1
fi

# TODO(phase-3): JSON-encode body file and PATCH.
echo "pr-update-body.sh: PATCH not yet implemented (resolved base_repo=$base_repo)" >&2
exit 1
```

### Success Criteria:

#### Automated Verification:

- [ ] Tests 2 (resolver argv), 3 (cross-fork PATCH URL), 9 (resolver
  failure remediation), 12 (body file missing), 13 (wrong arg count) all
  PASS. Note that test 3 will still FAIL the PATCH-URL assertion if it
  depends on PATCH being called — split that test's assertions if
  necessary so the resolver-argv part of it passes here.
- [ ] Tests 4 (PATCH method), 5-8 (body encoding), 10 (PATCH failure),
  11 (PATCH success) still FAIL (no PATCH wiring yet).
- [ ] `mise run test:unit:github` exits non-zero, with the FAIL count
  reduced to exactly the Phase-3 tests.
- [ ] `shellcheck skills/github/describe-pr/scripts/pr-update-body.sh`
  reports no issues.

#### Manual Verification:

- [ ] None for this phase — it is purely automated test progression.

---

## Phase 3: GREEN — Body Encoding and PATCH Call

### Overview

Implement `jq -Rs '{body: .}'` body encoding and the
`gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input -`
call. All remaining tests turn green.

### Changes Required:

#### 1. Posting logic in script

**File**: `skills/github/describe-pr/scripts/pr-update-body.sh`
**Changes**: Replace the Phase-2 `TODO(phase-3)` stub with the encoder +
PATCH call.

```bash
# Encode body as a JSON object { "body": "<file content>" } and PATCH.
# jq -Rs reads stdin as a raw string and emits a JSON-safe string scalar;
# wrapping in {body: .} produces the required GitHub REST payload shape.
if ! jq -Rs '{body: .}' <"$body_file" \
    | gh api --method PATCH "repos/$base_repo/pulls/$pr_number" --input -; then
  echo "pr-update-body.sh: PATCH failed for repos/$base_repo/pulls/$pr_number." >&2
  exit 1
fi
```

(The earlier `echo … PATCH not yet implemented` stub is removed.)

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:github` exits 0; all assertions green.
- [ ] `mise run test:unit` exits 0 (full unit aggregate green).
- [ ] `shellcheck skills/github/describe-pr/scripts/pr-update-body.sh`
  reports no issues.
- [ ] Spot-check of the recorded stdin payload in test 6 confirms
  shell metacharacters survive intact byte-for-byte.

#### Manual Verification:

- [ ] None for this phase — it is purely automated test progression.

---

## Phase 4: SKILL.md Wires to the Helper

### Overview

Swap the broken `gh pr edit` line in `describe-pr/SKILL.md` step 9 for
an invocation of the new helper. Preserve the frontmatter-strip recipe
and tmp cleanup. Leave the rest of step 9 (and the entire rest of the
skill) byte-identical.

### Changes Required:

#### 1. SKILL.md step 9

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**: Replace line 130's `gh pr edit ...` invocation with a
`pr-update-body.sh` invocation. The surrounding sub-steps (frontmatter
strip 1–4 and cleanup 6) are unchanged. Adjust the prose of step 5 to
name the helper and explain that it resolves the base repo and posts
via the REST API; reuse the lines 54-55 remediation phrasing for
consistency.

The replacement for the current sub-step 5 (line 130):

```markdown
  5. Post the body via the helper script, which resolves the base
     (upstream) repository for cross-fork safety and PATCHes via the
     GitHub REST API:
     `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh {number} {tmp directory}/pr-body-{number}.md`
     If you get an error about no default remote repository, instruct
     the user to run `gh repo set-default` and select the appropriate
     repository (mirrors the pattern in step 4).
```

Sub-step 6 (cleanup) remains exactly:

```markdown
  6. Clean up `{tmp directory}/pr-body-{number}.md`
```

### Success Criteria:

#### Automated Verification:

- [ ] `grep -n "gh pr edit" skills/github/describe-pr/SKILL.md` returns
  no matches.
- [ ] `grep -n "pr-update-body.sh" skills/github/describe-pr/SKILL.md`
  returns exactly one match in step 9.
- [ ] `diff` between pre-edit and post-edit SKILL.md confirms the only
  changes are in lines 130-131 (i.e., the replacement sub-step 5 and
  any wording-parity tweaks); the frontmatter-strip recipe at
  119-129 is byte-identical.
- [ ] `mise run test:unit` continues to pass.

#### Manual Verification:

- [ ] Inspection confirms the existing remediation pattern at lines
  54-55 is consistent with the new step's wording.
- [ ] Inspection confirms acceptance criterion AC4 (frontmatter
  stripped before body posted) is still satisfied by sub-steps 1–4
  preceding the new sub-step 5.

---

## Phase 5: Live-PR Manual Verification

### Overview

The work item's acceptance criteria include observable behaviour during a
real `/describe-pr` invocation that cannot be asserted from shell tests:
no `gh pr edit` appears in the command trace, and no deprecation-fallback
note is emitted. This phase runs `/describe-pr` end-to-end against a real
open PR and confirms.

### Changes Required:

#### 1. No file changes

All edits land in Phases 1-4. This phase is verification only.

### Success Criteria:

#### Automated Verification:

- [ ] None for this phase.

#### Manual Verification:

- [ ] **AC1 / AC7 — primary REST path on a same-repo PR**: pick any
  open PR on this repository (`atomic-innovation/accelerator`); run
  `/describe-pr <number>`; observe the command trace; confirm
  (a) no `gh pr edit` invocation appears, (b) no deprecation-fallback
  note is emitted, (c) the PR body on GitHub updates to match the
  stripped content of `{prs directory}/{number}-description.md` byte-
  for-byte.
- [ ] **AC2 — transitive coverage via `review-pr` / `respond-to-pr`**:
  in a session where `/review-pr` or `/respond-to-pr` directs the user
  to invoke `/describe-pr`, confirm the body posts cleanly without
  emitting the deprecation note.
- [ ] **AC3 — cross-fork PR**: if no fork-based PR is available
  against this repo, simulate by running
  `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh
  <fork-pr-number> <body-file>` against a known fork-based PR on
  GitHub (e.g. an open PR from a contributor's fork on a public
  repository where you have write access via REST); confirm via
  `gh api repos/{upstream-owner}/{upstream-repo}/pulls/{number} --jq
  .body` that the body landed on the upstream PR, not the head fork.
  If no suitable cross-fork PR is reachable for live verification,
  document this limitation in the PR description and rely on test 3
  from Phase 1.
- [ ] **AC5 — cleanup**: after the run, confirm
  `{tmp directory}/pr-body-{number}.md` is removed (the SKILL.md
  cleanup sub-step 6 still runs).

---

## Testing Strategy

### Unit Tests:

All Phase-1 test cases (1-13). The harness uses a PATH-stubbed `gh` to
simulate every external interaction:

- **Owner/repo resolution paths**: same-repo, cross-fork, and the
  "no default remote" failure mode.
- **Body encoding edge cases**: empty body, single-line, multi-line,
  shell metacharacters (backticks, `$()`, quotes, backslashes), unicode
  (emoji, accented chars).
- **Exit-code propagation**: resolver failure, PATCH failure, success.
- **Argument validation**: missing args, missing body file, wrong arg
  count.

### Integration Tests:

None. The script's only collaborator (`gh`) is mocked; integration is
covered by manual verification on a live PR in Phase 5.

### Manual Testing Steps:

1. Run `/describe-pr <number>` against an open PR on this repo. Confirm
   no `gh pr edit` in the trace; confirm no fallback note; confirm the
   PR body on GitHub matches.
2. If a cross-fork PR is reachable, repeat against it; confirm the
   PATCH targets `{upstream-owner}/{upstream-repo}/pulls/{number}`.
3. Trigger the resolver-failure path by detaching the default remote
   (`gh repo set-default --unset` in a clean clone) and re-running the
   helper directly; confirm the remediation hint appears in stderr.

## Performance Considerations

Not applicable. The fix replaces one `gh` invocation with two
(`gh pr view` + `gh api`), adding a single API round-trip per
`/describe-pr` run. Both calls are sub-second in practice and the skill
is interactive.

## Migration Notes

Not applicable. The change is internal to a skill's instruction file and
its companion helper script; there is no on-disk data or persistent
configuration to migrate. Users running pre-fix versions of the skill
will continue to receive the deprecation-fallback note until they pick
up the fix; no rollout coordination is needed beyond shipping the
updated plugin.

## References

- Work item: `meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md`
- Research: `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`
- Edit site: `skills/github/describe-pr/SKILL.md:130`
- Existing frontmatter strip (preserve): `skills/github/describe-pr/SKILL.md:119-129`
- Existing cleanup (preserve): `skills/github/describe-pr/SKILL.md:131`
- Error-remediation precedent: `skills/github/describe-pr/SKILL.md:54-55`
- Test harness shape: `skills/decisions/scripts/test-adr-scripts.sh`
- Test helpers: `scripts/test-helpers.sh`
- mise unit-test wiring: `mise.toml:73-95`
- ADR precedent: `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`
- Stdin-piped JSON posting precedent (no `jq -Rs` though):
  `skills/github/respond-to-pr/SKILL.md:468-472`
- JSON-wrapper-file `--input` posting precedent:
  `skills/github/review-pr/SKILL.md:571-574`
- Sunset notice (root cause):
  https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/
