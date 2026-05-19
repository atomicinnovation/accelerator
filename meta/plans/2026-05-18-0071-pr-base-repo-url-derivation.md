---
date: "2026-05-18T20:55:37+00:00"
type: plan
skill: create-plan
work-item: "0071"
status: accepted
---

# pr-base-repo.sh URL-Derivation Migration Implementation Plan

## Overview

Swap the base-repo resolver at `skills/github/scripts/pr-base-repo.sh:48`
from `gh pr view --json baseRepository` to `gh pr view --json url`,
deriving `owner/repo` by parsing the PR URL path. The current call
requests a `--json` field that is not in `gh 2.65.0`'s `pr view`
allowlist, which blocks the `describe-pr`, `review-pr`, and
`respond-to-pr` skills from completing their "post to GitHub" steps.
URL derivation preserves cross-fork safety because the PR `url` field
reflects the upstream `owner/repo` even on fork PRs, and the resolver's
JSON-parse defence stays intact. The Phase 4 smoke check is the
source of truth for which `--json` fields the installed `gh` accepts
— the plan deliberately makes no static claim about a minimum `gh`
version.

The work follows strict test-driven development across four phases. Phases
1 and 2 land **atomically** as a single commit/PR: Phase 1 reshapes
harness assertions to the new argv/payload shape (**red** against the
unchanged script), Phase 2 swaps the resolver's data source and turns
the harness **green**. Phase 3 rewrites the resolver's header comment
to explain URL-based cross-fork safety and adds an unconditional
tree-state regression guard (test 24) asserting that
`--json baseRepository` does not reappear under `skills/github/`.
Phase 4 lands a new real-`gh` smoke check at
`skills/github/scripts/test-pr-base-repo-real-gh.sh` that probes
the installed `gh`'s allowlist semantically — by invoking
`gh pr view --json INVALID` and parsing the error message gh emits,
which is the same runtime surface the resolver itself hits. The
harness skips cleanly when `gh` is not on `PATH`. No SKILL.md edits
are required — all three consumer skills read the resolver's
`<owner>/<name>\n` stdout contract, which is preserved unchanged.

## Current State Analysis

- **Single broken line**: `skills/github/scripts/pr-base-repo.sh:48`
  calls `gh pr view "$pr_number" --json baseRepository`. On `gh 2.65.0`
  this exits 1 with `Unknown JSON field: "baseRepository"` plus the
  allowlist on stderr. Downstream, the resolver replays that stderr
  (lines 49-51) and exits 1 with `could not resolve base repo for PR
  #<n>.` (line 52).
- **Resolver's reason for existing is preserved by the fix**: header
  comment at `pr-base-repo.sh:15-18` declares cross-fork safety —
  `gh repo view` would return the local checkout's repo (the fork for
  contributors), which is wrong for cross-fork PR operations. The PR
  `url` field reflects the **upstream** repo
  (`https://github.com/<base-owner>/<base-repo>/pull/<n>`) even on
  fork-originated PRs, so URL derivation preserves the property.
- **All resolver defences downstream of line 48 stay reusable**: the
  JSON-parse pre-validation at lines 63-67 still applies because
  `--json url` returns `{"url":"..."}` (still JSON). The emptiness
  check at lines 72-76 still applies because the URL-derived
  `(owner, name)` pair can independently be empty if the URL doesn't
  match the expected pattern. The conditional `no default remote
  repository` hint at lines 53-55 stays correct.
- **The resolver's stdout contract is `"<owner>/<name>\n"`** (line 78).
  All three consumer skills depend on the literal text output, not the
  internal JSON shape: `pr-update-body.sh` captures it via command
  substitution and substitutes into the `gh api PATCH` URL;
  `review-pr/SKILL.md:118` writes it to `repo-info.txt`;
  `respond-to-pr/SKILL.md:68` consumes it inline. Preserving the
  stdout contract means zero SKILL.md edits.
- **Existing harness pins the broken argv shape**:
  `test-pr-base-repo-scripts.sh:119-120` asserts exact equality
  `"pr view 119 --json baseRepository"`. The sibling harness at
  `describe-pr/scripts/test-pr-update-body-scripts.sh:143-144` makes
  the same assertion. Both must flip in Phase 1.
- **Existing tests use stubbed JSON payloads**: tests 3, 4, 5, 8, 9,
  11 in `test-pr-base-repo-scripts.sh` write payloads of the form
  `{"baseRepository":{"owner":{"login":"..."},"name":"..."}}`. These
  must reshape to `{"url":"https://github.com/<owner>/<repo>/pull/<n>"}`
  with the appropriate negative-case variants for the null-guard
  tests.
- **Existing PHASE env-var pattern is the precedent for tree-state
  guards**: `test-pr-base-repo-scripts.sh:25-29, 264-282` show how the
  0059 plan gated tests 22 and 23 by phase. Test 24 follows the same
  shape — skipped in phases 1-2, enforced from phase 3 onward and at
  `final`.
- **Existing test runner picks up new shell harnesses automatically**:
  `tasks/test/helpers.py:13-34` (`run_shell_suites`) globs
  `**/test-*.sh` under each registered area, requires the executable
  bit, and skips `test-helpers.sh`. A new
  `test-pr-base-repo-real-gh.sh` at `skills/github/scripts/` is picked
  up by `mise run test:integration:github` (wired via
  `mise.toml:120-123` → `tasks/test/integration.py:46-49`) without
  registration plumbing.
- **The workspace's pinned `gh` is 2.89.0** (`mise.toml:7`) — the
  workspace's own automated runs cannot reproduce the bug, since
  2.89.0 accepts `baseRepository`. The real-`gh` smoke check (Phase 4)
  probes the installed `gh`'s field allowlist via the
  `Unknown JSON field` error path (the same runtime surface the
  resolver itself hits), catching any future regression where a
  requested field disappears from the allowlist on the installed
  `gh`.

## Desired End State

After this plan is complete:

- `gh pr view <n> --json url` is the only `gh pr view --json` call in
  `pr-base-repo.sh`. The resolver parses `.url` via `jq` and extracts
  `<owner>/<name>` via a bash regex (`BASH_REMATCH`) whose capture
  charsets match GitHub's actual rules: owner
  `[A-Za-z0-9][A-Za-z0-9-]*` (no dots or underscores), repo
  `[A-Za-z0-9._-]+` (permissive, accepts e.g. `.github`). The stdout
  contract (`"<owner>/<name>\n"`) is byte-identical to the current
  behaviour on any valid input. The regex's structural non-empty
  guarantee replaces the previous post-extraction null-guard; no
  separate empty-check follows the regex.
- All three consumer skills (`describe-pr`, `review-pr`,
  `respond-to-pr`) complete their "post to GitHub" steps on
  `gh 2.65.0` without operator intervention. (Verified manually
  against a real PR; see Manual Verification under Phase 2.)
- `test-pr-base-repo-scripts.sh` and
  `describe-pr/scripts/test-pr-update-body-scripts.sh` are green at
  `PHASE=final` with assertions reshaped to the new argv/payload and
  the post-fix-specific stderr messages.
- A new unconditional tree-state regression guard at
  `test-pr-base-repo-scripts.sh` test 24 asserts the absence of
  `--json baseRepository` under `skills/github/`.
- A new real-`gh` smoke check at
  `skills/github/scripts/test-pr-base-repo-real-gh.sh` asserts every
  `--json` field the resolver requests is in the installed `gh`'s
  allowlist, probed via `gh pr view --json INVALID 2>&1` (the same
  error-message surface the resolver itself hits at runtime). Skips
  cleanly if `gh` is absent. `mise run test:integration:github` picks
  it up via autodiscovery.
- The replaced header comment (the lines-4-28 block in
  `pr-base-repo.sh`) explains why URL derivation is cross-fork-safe
  (the PR URL reflects the upstream repo, not the fork's checkout)
  and points operators at the Phase 4 smoke check as the source of
  truth for `gh`-side allowlist behaviour, replacing the
  now-misleading reference to `gh pr view --json baseRepository`.

### Key Discoveries:

- The defect is genuinely localised to one line:
  `pr-base-repo.sh:48`. Three skills consume the resolver via its
  stdout contract; none parse the internal JSON shape. Research at
  `meta/research/codebase/2026-05-18-0071-describe-pr-base-repo-resolver-unsupported-gh-field.md`.
- The PR `url` field shape is the canonical GitHub HTML URL
  (`https://<host>/<owner>/<repo>/pull/<n>`) and is in the `--json`
  allowlist on the workspace's pinned `gh 2.89.0`. The plan
  deliberately makes no static claim about which other `gh` releases
  accept the `url` field — the Phase 4 smoke harness probes the
  installed `gh`'s allowlist at integration-test time and is the
  source of truth at runtime.
- Existing test stubs cannot catch this class of regression by
  construction: `install_fake_gh` in
  `skills/github/scripts/test-helpers.sh:17-99` dispatches solely on
  `$1 $2` and never validates `--json` field names. The smoke check
  at Phase 4 is the required corrective layer.
- The `gh pr edit --body-file` GraphQL deprecation flagged in the
  work item's Context is **already fixed** by the 0059 plan — no
  skill in `skills/` reaches that path. Out of scope here.

## What We're NOT Doing

- **Not addressing the `gh pr edit` Projects-classic GraphQL
  deprecation.** That call site was removed by the 0059 plan; no
  skill in `skills/` reaches it. The work item's Open Questions item
  on this is resolved as "no follow-up needed."
- **Not pinning a minimum `gh` version in `SKILL.md` or asserting it
  at resolver startup** (work item's Open Questions fix-path
  candidate (c)). Users running an older `gh` that lacks `url` from
  the `--json` allowlist will receive the same shape of
  `Unknown JSON field` error this plan eliminates for `baseRepository`
  on `gh 2.65.0` — just for a different field. This is accepted: the
  Phase 4 smoke harness gives operators a clear diagnostic against the
  actually-installed `gh`, and operators on out-of-support `gh`
  releases have other reasons to upgrade. The plan deliberately makes
  no static claim about a minimum `gh` version in the resolver or
  consumer SKILL.md files; the smoke harness is the source of truth
  at integration-test time.
- **Not introducing dual code paths in the resolver** (fix-path
  candidate (b)). The structured `baseRepository` path is dropped
  entirely; if `baseRepository` lands in a future `gh` allowlist, a
  future plan can reintroduce it. Carrying two paths now buys nothing.
  Recovery from a future gh-side surprise (e.g. `url` being removed
  from the allowlist) requires landing a new plan — the rollback path
  is `git revert` of this change. The Phase 4 smoke harness gives
  operators an early-warning signal on `gh` upgrades, so the recovery
  window is bounded by integration-test cadence rather than the next
  end-to-end failure in production.
- **Not provisioning sandbox PR fixtures** for the AC #4 regression
  matrix. AC #4 is explicitly out of scope by user decision: the
  one-line fix plus comprehensive stub coverage plus the new
  real-`gh` smoke layer (Phase 4) provide proportionate protection
  without the operational overhead of provisioning sandbox repos.
- **Not adding an automated self-test sibling for Phase 4's
  diagnostic branches** (Marker-sanity SKIP, Control-field FAIL).
  A `test-pr-base-repo-real-gh-self-test.sh` sibling that stubs `gh`
  via PATH and feeds canned stderr through the parser would lock
  these branches against silent regression between manual runs. This
  is explicitly accepted as a trade-off: the four manual walkthroughs
  in Phase 4's Manual Verification block exercise each branch at
  implementation time, and the diagnostic-branch code is small and
  read-only against gh's response — the regression surface is
  small enough that the additional ~30 lines of self-test plumbing
  are not justified by the marginal coverage gain. If a future
  regression in these branches is discovered, a follow-up work item
  should add the sibling at that point.
- **Not changing any SKILL.md file.** The resolver's stdout contract
  is preserved; no call site needs adjustment.
- **Not changing `pr-update-body.sh` production code.** Its PATCH
  path is independently confirmed working on `gh 2.65.0`. Only its
  sibling test's argv assertion (line 144) flips.
- **Not extending coverage to AC #3 with real-`gh` failure-mode
  tests** (auth missing, network failure, malformed JSON, deleted
  PR). The resolver's behaviour under these conditions is fully
  determined by the `gh`-stub interface, which the existing tests 6,
  7, 8, 9, 11, 12 already exercise. Re-running the same conditions
  against real `gh` would duplicate coverage without surfacing new
  defects, since the only `gh`-version-specific surface is the
  `--json` field allowlist, which Phase 4 already covers.

## Implementation Approach

Strict test-driven development across four phases. **Phases 1 and 2
land atomically as a single commit/PR** — Phase 1 alone leaves
production unchanged with red CI, which is an unsafe partial state
(a contributor reading the failure could revert Phase 1 instead of
completing Phase 2). Phase 3 and Phase 4 may land in the same PR or
in follow-ups; both are additive over the green state Phase 2
establishes.

The existing `PHASE` env var (1-6 or `final`, gated in both harnesses
at lines 25-29) is reused for the existing red→green→refactor sequence
on the reshaped assertions. Test 24 is enforced **unconditionally**
(no PHASE gate), since Phases 1+2 land together and the staged-landing
rationale that motivated PHASE gating for tests 22/23 does not apply
to this work item. The new real-`gh` smoke harness is similarly
unconditional: it lands after Phase 2, so the resolver's `--json url`
shape is in tree before the smoke harness runs.

The TDD sequence is:

1. **Red (Phase 1)**: Reshape harness assertions to the post-fix
   shape and post-fix-specific error-message text. Production script
   unchanged. Confirm tests 3, 4, 5, 8, 9, 11 in
   `test-pr-base-repo-scripts.sh` and test 6 in
   `test-pr-update-body-scripts.sh` are red at `PHASE=1` for the
   intended reason (argv/payload/stderr text mismatch).
2. **Green (Phase 2)**: Swap the data source in `pr-base-repo.sh`.
   The regex's structural non-empty guarantee replaces the previous
   null-guard. Confirm all tests pass at `PHASE=2`.
3. **Refactor (Phase 3)**: Rewrite header comment; add test 24
   (tree-state regression guard against `--json baseRepository`).
   Confirm all tests pass at `PHASE=3` and `PHASE=final`.
4. **Smoke (Phase 4)**: Add real-`gh` smoke check that probes the
   installed `gh`'s allowlist via the `gh pr view --json INVALID`
   error-message surface. Confirm
   `mise run test:integration:github` picks it up via autodiscovery
   and exercises it on the workspace's pinned `gh 2.89.0`.

---

## Phase 1: Red — Reshape Harness Assertions

### Overview

Update every test in
`skills/github/scripts/test-pr-base-repo-scripts.sh` and
`skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
that pins the current argv shape or payload schema to expect the
post-fix shape. The production resolver is unchanged; the failing
tests prove the harness is exercising the right surface before the
fix lands.

### Changes Required:

#### 1. `skills/github/scripts/test-pr-base-repo-scripts.sh`

**Reshape Test 3 (same-repo) payload and assertion** (lines 86-94)

```bash
# --- test 3: same-repo resolves ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 3: stdout is acme/app" "acme/app" "$out"
```

**Reshape Test 4 (upstream URL parses to upstream coords)** (lines
96-107). Renamed from "cross-fork resolves to upstream" — the
stubbed harness cannot exercise real cross-fork behaviour (the stub
dispatches only on `$1 $2`), so this test only verifies the URL-parsing
branch with an upstream-shaped payload. The real cross-fork-safety
property is covered by an enumerated manual-verification step in
Phase 2.

```bash
# --- test 4: upstream URL parses to upstream coords ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" \
  '{"url":"https://github.com/upstream-org/upstream-repo/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4: stdout matches the URL's upstream coords" \
  "upstream-org/upstream-repo" "$out"
```

**Add Test 4b (GHE host)** — locks the host-agnostic property the
regex permits but no existing test exercises:

```bash
# --- test 4b: GHE host parses correctly ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" \
  '{"url":"https://github.acme.corp/team-a/repo/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4b: GHE host extracts owner/repo correctly" \
  "team-a/repo" "$out"
```

**Add Test 4c (rejects unsafe URL characters)** — locks the
tightened repo-name charset against percent-encoded smuggling:

```bash
# --- test 4c: percent-encoded chars in owner rejected ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
# Percent-encoded slash in owner segment — must fail regex extraction.
write_file "$payload" '{"url":"https://github.com/ac%2fme/app/pull/119"}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 4c: percent-encoded owner rejected with exit 1" 1 "$rc"
assert_contains "test 4c: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"
```

**Add Test 4d (leading-dot repo name)** — locks the relaxed repo
charset for `.github`-style repos (every GitHub org has one for
workflow config):

```bash
# --- test 4d: leading-dot repo (.github) accepted ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/.github/pull/119"}'
out=$(GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$T/stderr") || true
assert_eq "test 4d: leading-dot repo accepted" \
  "acme/.github" "$out"
```

**Add Test 4e (percent-encoded in repo segment)** — locks the
charset on the repo side too (test 4c only covered the owner side):

```bash
# --- test 4e: percent-encoded chars in repo rejected ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app%2fevil/pull/119"}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 4e: percent-encoded repo rejected with exit 1" 1 "$rc"
assert_contains "test 4e: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"
```

**Reshape Test 5 (argv shape)** (lines 110-120)

```bash
# --- test 5: argv shape ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{"url":"https://github.com/acme/app/pull/119"}'
GH_PR_VIEW_OUT="$payload" "$SCRIPT" 119 >/dev/null 2>"$T/stderr" || true
argv=$(cat "$GH_ARGV_LOG")
assert_eq "test 5: argv is exactly 'pr view 119 --json url'" \
  "pr view 119 --json url" "$argv"
```

**Reshape Test 8 (malformed URL) and Test 9 (truncated URL)** (lines
165-210). The original tests covered null-owner and null-name in the
structured `baseRepository` payload. Reshape to two equivalent
negative cases against the URL shape. **The stderr assertions must
match the post-fix-specific message text** (`could not extract
owner/repo from url`) — the original null-guard error echoes the raw
payload (which contains the literal JSON key `"url"`), so a bare
`assert_contains ... "url"` would pass against the unchanged production
script and break the red claim.

```bash
# --- test 8: malformed URL guard ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
# Missing the owner segment — extraction must yield empty owner and exit 1.
write_file "$payload" '{"url":"https://github.com//app/pull/119"}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 8: malformed-URL exits 1" 1 "$rc"
if grep -qE "^/" "$stdout_capture"; then
  echo "  FAIL: test 8: must NOT print '/app' to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 8: stdout does not smuggle malformed coords"
  PASS=$((PASS + 1))
fi
assert_contains "test 8: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"
```

```bash
# --- test 9: truncated URL guard ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
# Missing the repo segment — extraction must yield empty name and exit 1.
write_file "$payload" '{"url":"https://github.com/acme/pull/119"}'
stderr_capture="$T/stderr"
stdout_capture="$T/stdout"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 >"$stdout_capture" 2>"$stderr_capture" || rc=$?
assert_eq "test 9: truncated-URL exits 1" 1 "$rc"
if grep -qE "/$" "$stdout_capture"; then
  echo "  FAIL: test 9: must NOT print 'acme/' to stdout"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: test 9: stdout does not smuggle truncated coords"
  PASS=$((PASS + 1))
fi
assert_contains "test 9: stderr names URL-extraction failure" \
  "$(cat "$stderr_capture")" "could not extract owner/repo from url"
```

**Reshape Test 11 (missing field)** (lines 230-241). Originally asserted
exit 1 on a payload of `{}` (missing `baseRepository`). Reshape to a
payload missing the `url` key, asserting both exit 1 and the
post-fix-specific stderr text. Without the stderr assertion the test
would pass against the unchanged production script (which also exits 1
on `{}` via the structured null-guard).

```bash
# --- test 11: missing url field ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
payload="$T/pr-view.json"
write_file "$payload" '{}'
stderr_capture="$T/stderr"
rc=0
GH_PR_VIEW_OUT="$payload" \
  "$SCRIPT" 119 2>"$stderr_capture" >/dev/null || rc=$?
assert_eq "test 11: missing url exits 1" 1 "$rc"
assert_contains "test 11: stderr names empty/null url" \
  "$(cat "$stderr_capture")" "url was empty/null"
```

**No change required**: tests 1, 2, 6, 7, 10, 12 do not pin the
JSON-payload schema; they cover orthogonal concerns (executable bit,
usage, stderr replay, jq preflight, non-JSON guard) and remain
correct against either implementation. Tests 22 and 23 (tree-state
guards) are untouched in Phase 1.

#### 2. `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`

**Reshape Test 6 (resolver argv)** (lines 130-145)

```bash
# --- test 6: resolver argv shape ---
new_case; T=$CASE_DIR
setup_gh_stub "$T"
GH_PR_VIEW_OUT="$T/pr-view.json"
write_file "$GH_PR_VIEW_OUT" "$(default_payload)"
export GH_PR_VIEW_OUT
body_file="$T/body.md"
write_file "$body_file" "hello world"
"$SCRIPT" 119 "$body_file" >/dev/null 2>"$T/stderr" || true
pr_view_line=$(grep "^pr view" "$GH_ARGV_LOG" || true)
assert_eq "test 6: pr view argv shape" \
  "pr view 119 --json url" "$pr_view_line"
unset GH_PR_VIEW_OUT
```

**Reshape `default_payload` and `upstream_payload` helpers** (lines
57-65)

```bash
# Standard same-repo base-repo payload reused across many tests.
default_payload() {
  echo '{"url":"https://github.com/acme/app/pull/119"}'
}

# Standard upstream payload reused by the cross-fork tests.
upstream_payload() {
  echo '{"url":"https://github.com/upstream-org/upstream-repo/pull/119"}'
}
```

These helpers are reused by tests 6-20 (most of the harness), so
reshaping them once flips the entire downstream chain to the URL
shape.

### Success Criteria:

#### Automated Verification:

- [x] At `PHASE=1`, tests 3, 4, 4b, 4c, 4d, 4e, 5, 8, 9, 11 in
  `test-pr-base-repo-scripts.sh` fail: `PHASE=1 mise run test:integration:github`.
  Tests 8, 9, and 11 fail on the **stderr text assertion** (the post-fix-
  specific message `could not extract owner/repo from url` / `url was
  empty/null` is not present in the unchanged production's null-guard
  output) — not on the exit-code assertion, which is satisfied by the
  legacy null-guard branch.
- [x] At `PHASE=1`, test 6 in `test-pr-update-body-scripts.sh` fails
  (the resolver's argv assertion).
- [x] At `PHASE=1`, tests 1, 2, 6, 7, 10, 12 in
  `test-pr-base-repo-scripts.sh` still pass (orthogonal concerns).
- [x] At `PHASE=1`, `shellcheck skills/github/scripts/test-pr-base-repo-scripts.sh
  skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` exits 0.

#### Manual Verification:

- [x] Run `PHASE=1 bash skills/github/scripts/test-pr-base-repo-scripts.sh`
  and visually confirm the failing tests fail for the *intended*
  reason (argv mismatch, payload-shape mismatch, or stderr-text
  mismatch), not an incidental harness issue.

---

## Phase 2: Green — Swap pr-base-repo.sh to URL Derivation

### Overview

Replace the `--json baseRepository` call in `pr-base-repo.sh:48` with
`--json url`, parse `.url` via `jq`, and derive `(owner, name)` by
splitting the URL path. The emptiness guard at lines 72-76 stays
semantically — only the source of `owner` and `name` changes. The
JSON-parse pre-validation at lines 63-67 stays unchanged.

### Changes Required:

#### 1. `skills/github/scripts/pr-base-repo.sh`

**Replace line 48**

```bash
if ! payload=$(gh pr view "$pr_number" --json url 2>"$err_file"); then
```

**Replace lines 69-76** (extraction)

```bash
url=$(jq -r '.url // ""' <<<"$payload")

if [ -z "$url" ]; then
  echo "pr-base-repo.sh: url was empty/null in gh response." >&2
  echo "  Raw payload: $payload" >&2
  exit 1
fi

# Derive owner/name from the upstream PR URL. The PR url field always
# points at the base (upstream) repo, even when the PR was opened from
# a fork — this is the cross-fork-safety property the resolver
# guarantees. URL shape: https://<host>/<owner>/<repo>/pull/<n>.
# Charsets match GitHub's actual rules:
#   - Owner (user/org): must start with [A-Za-z0-9], may contain
#     hyphens. No dots, no underscores.
#   - Repo: any combination of [A-Za-z0-9._-], may start with `.` or
#     `_` (e.g. the widely-used `.github` repo). Cannot be empty.
# Characters outside these sets (notably `%` for percent-encoded
# smuggling) are rejected at parse time rather than passed through to
# `gh api` interpolation downstream.
if ! [[ "$url" =~ ^https://[^/]+/([A-Za-z0-9][A-Za-z0-9-]*)/([A-Za-z0-9._-]+)/pull/[0-9]+$ ]]; then
  echo "pr-base-repo.sh: could not extract owner/repo from url: $url" >&2
  exit 1
fi
owner="${BASH_REMATCH[1]}"
name="${BASH_REMATCH[2]}"
```

**Rationale for the regex over `sed`**: Bash's `=~` operator with
`BASH_REMATCH` captures is already used elsewhere in the codebase
(e.g. `skills/visualisation/visualise/scripts/launch-server.sh`),
needs no external process, and gives the resolver a single
extraction-failure exit path. The charset matches GitHub's actual
rules, so the regex doubles as a smuggling guard: a URL containing
`%2f`, a percent-encoded path segment, or any character outside the
permitted classes is rejected at parse time rather than passed
through to `gh api` interpolation. Owner is stricter than repo
(GitHub disallows `.`/`_` in user and org names, but allows them in
repo names — `.github` being the canonical example). The regex's `+`
and non-empty leading classes make the captures structurally
non-empty, so no separate empty-check follows — the regex match is
the sole invariant.

**Line 78 unchanged**

```bash
printf '%s/%s\n' "$owner" "$name"
```

The stdout contract is preserved byte-for-byte.

#### 2. (Optional) Manually probe the installed `gh`'s field allowlist

The Phase 4 smoke harness automates this once it lands; before Phase 4
the implementer can run the same probe manually to confirm the
workspace's pinned `gh` accepts `url`:

```bash
gh pr view 1 --repo cli/cli --json url 2>&1 | head -n 3
```

Expected: the call may fail with auth or PR-not-found errors, but
must NOT emit `Unknown JSON field: "url"`. If it does, the smoke
harness will FAIL when added in Phase 4 and an alternative resolver
strategy is required.

### Success Criteria:

#### Automated Verification:

- [x] At `PHASE=2`, all tests in `test-pr-base-repo-scripts.sh` pass
  (including reshaped tests 3, 4, 4b, 4c, 4d, 4e, 5, 8, 9, 11):
  `PHASE=2 mise run test:integration:github`
- [x] At `PHASE=2`, all tests in `test-pr-update-body-scripts.sh`
  pass (including the reshaped test 6).
- [x] `shellcheck skills/github/scripts/pr-base-repo.sh` exits 0.
- [x] `mise run test:integration:github` (default `PHASE=final`) is
  fully green. Test 24 has not yet been added (Phase 3) and the
  real-`gh` smoke harness has not yet been added (Phase 4), so the
  test count is the existing total plus four new tests (4b GHE host,
  4c percent-encoded owner rejection, 4d `.github` repo, 4e
  percent-encoded repo rejection). Tests 8, 9, and 11 are reshaped in
  place rather than added.

#### Manual Verification:

The path `<plugin-root>` below refers to the installed plugin
directory. Resolve via:

```bash
PLUGIN_ROOT=$(ls -d ~/.claude/plugins/cache/atomic-innovation-prerelease/accelerator/*/ | sort -V | tail -n 1)
```

- [ ] Run `gh pr view 1 --repo cli/cli --json url 2>&1` on the
  workspace's pinned `gh` and confirm the output does NOT contain
  `Unknown JSON field: "url"` (the call itself may fail with auth or
  PR-not-found errors — only the field-validation outcome matters).
- [ ] **Cross-fork-safety automated coverage is structurally absent**
  (the stubbed harness can't model fork-vs-upstream). To verify the
  resolver's central invariant, run `gh pr view <pr-number> --json url
  --jq .url` from inside an actual **fork checkout** (not the
  upstream) of a real open cross-fork PR. Confirm the returned URL
  has the **upstream** host/owner/repo segments, not the fork's.
  Capture the output in the PR description as the verification
  record. This is the canonical evidence that the URL derivation
  preserves cross-fork safety.
- [ ] On a machine with `gh 2.65.0` (the failure-reproducing
  version): run
  `$PLUGIN_ROOT/skills/github/describe-pr/scripts/pr-update-body.sh <pr> /tmp/body.md`
  against a real same-repo open PR; confirm exit 0 and that the PR
  body on GitHub matches the supplied file byte-for-byte (covers
  work item AC #1).
- [ ] On the same `gh 2.65.0`: run against a real cross-fork open
  PR; confirm the PATCH targets the upstream `owner/repo`, not the
  fork (covers work item AC #2).
- [ ] On the same `gh 2.65.0`: run the three skills end-to-end
  (`/accelerator:describe-pr`, `/accelerator:review-pr`,
  `/accelerator:respond-to-pr`) against an open PR; confirm each
  reaches its "post to GitHub" step without manual intervention
  (covers work item AC #4 per-skill outcome, sampled at gh 2.65.0
  only since the full matrix is out of scope).

If `gh 2.65.0` is unobtainable: fall back to verifying the Phase 4
smoke check passes on the installed `gh`, and document the
unavailability in the PR description. The smoke + reshaped unit
tests give adequate confidence; the gh 2.65.0 runs are
defence-in-depth.

---

## Phase 3: Refactor — Header Rewrite and Tree-State Guard

### Overview

Update the resolver's header comment (the comment block on lines 4-28
of the current file, between `set -euo pipefail` and the
`if [ $# -ne 1 ]` block) to explain URL-based cross-fork safety,
replacing the now-misleading reference to
`gh pr view --json baseRepository`. Add an unconditional tree-state
regression guard (test 24) that asserts `--json baseRepository` does
not reappear anywhere under `skills/github/`, mirroring the
established pattern from tests 22 and 23 in the 0059 plan but without
the PHASE gating (Phases 1+2 land atomically; the staged-landing
rationale that motivated gating tests 22/23 does not apply here).

### Changes Required:

#### 1. `skills/github/scripts/pr-base-repo.sh` (replace the header comment block — lines 4-28 of the current file)

```bash
# Usage: pr-base-repo.sh <pr-number>
# Prints "<owner>/<name>" of the base (upstream) repository for the given
# pull request to stdout. Cross-fork-safe: resolves via
# `gh pr view --json url`, parsing the upstream owner/repo out of the
# PR URL path. Used by describe-pr (for PATCHing the body), review-pr,
# and respond-to-pr.
#
# Exit codes:
#   0  success
#   1  resolution failed (auth, network, 404, malformed JSON, ...)
#   2  usage error (wrong arg count, missing jq, ...)
#
# Conventions:
# - Cross-fork-safe: the PR URL field reflects the base (upstream) repo
#   even when the PR was opened from a fork — extracting owner/repo
#   from that URL guarantees PATCHes target the right resource. The
#   alternative `gh repo view` would return the local checkout's repo
#   (the fork, for contributors), which is wrong for cross-fork PR
#   operations.
# - The Phase 4 smoke check at scripts/test-pr-base-repo-real-gh.sh is
#   the source of truth for which `--json` fields the installed `gh`
#   accepts. The resolver makes no static claim about a minimum `gh`
#   version; the smoke check fails loudly on an installed `gh` whose
#   allowlist omits `url`, with an actionable error pointing at work
#   item 0071.
# - Preserves the underlying gh stderr on failure so callers see the
#   real cause; emits a conditional `gh repo set-default` remediation
#   only when the captured stderr matches the known phrase.
# - The URL extraction regex restricts owner/repo to GitHub's actual
#   repo-name charset ([A-Za-z0-9._-]); URLs containing percent-encoded
#   or otherwise unusual characters are rejected at parse time, so
#   nothing smuggles into the downstream `gh api` URL interpolation.
#
# Invocation: must be run as a subprocess (e.g. via command
# substitution or direct execution). The EXIT trap on the internal
# err_file would clobber a caller's own EXIT trap if this script were
# `source`d. All current callers spawn a subshell, which is safe.
```

#### 2. `skills/github/scripts/test-pr-base-repo-scripts.sh` (add test 24)

Insert after the existing test-23 block (after line 282), before
`test_summary`:

```bash
# Test 24 — regression guard against the broken --json baseRepository
# request that work item 0071 fixed.
#
# Background: gh 2.65.0 does not allowlist `baseRepository` in
# `gh pr view --json`, so any reappearance of that flag combination
# under skills/github/ would re-break the describe-pr / review-pr /
# respond-to-pr post-step on gh 2.65.0. URL derivation via
# `gh pr view --json url` is the replacement (see
# pr-base-repo.sh's header and meta/work/0071-*.md).
#
# Unconditional (no PHASE gate): Phases 1+2 land atomically, so the
# staged-landing rationale that motivated PHASE gating for tests 22
# and 23 does not apply here.
assert_grep_empty "test 24 (regression guard for 0071 — see pr-base-repo.sh header)" \
  "$PLUGIN_ROOT/skills/github/" "--json baseRepository" \
  -F --
```

`assert_grep_empty`'s signature is
`assert_grep_empty <name> <path> <pattern> [extras...]` and the helper
expands to `grep -rn "$@" "$pattern" "$path"` (see
`scripts/test-helpers.sh:322`). The `-F --` extras force fixed-string
matching and terminate option processing **before** the pattern, so
grep treats `--json baseRepository` as a literal string rather than
parsing `--json` as an unknown long option (which both GNU and BSD
grep do regardless of argv position, unless `--` precedes the
pattern). Tests 22 and 23 don't need this because their patterns
(`gh pr edit`, `gh repo view --json owner,name`) don't start with
`--`. No `--include` flag is needed because the pattern should not
reappear in SKILL.md, in scripts, or anywhere else under
`skills/github/`. The test name includes a breadcrumb to the resolver
header so a future operator hitting this FAIL has a recovery path.

### Success Criteria:

#### Automated Verification:

- [x] At every `PHASE` value, test 24 fires (it has no PHASE gate).
  Confirm at `PHASE=1`, `PHASE=2`, `PHASE=3`, and `PHASE=final` that
  test 24 reports PASS once Phase 2's `--json url` swap is in tree.
- [x] At `PHASE=final`, all of `test-pr-base-repo-scripts.sh` is
  green, including tests 22, 23, and 24.
- [x] `grep -rF -- "--json baseRepository" skills/github/` returns
  no matches.
- [x] `shellcheck skills/github/scripts/test-pr-base-repo-scripts.sh`
  and `shellcheck skills/github/scripts/pr-base-repo.sh` both exit 0.

#### Manual Verification:

- [x] Visually read the updated header comment in
  `pr-base-repo.sh` (the replaced lines 4-28 block) and confirm it
  accurately describes the new data-source behaviour and the
  cross-fork-safety property.

#### Implementation Notes:

- Test 24 self-matches if the literal string `--json baseRepository`
  appears anywhere in its own source (the file lives under the search
  root `skills/github/`). The pattern is built at runtime by
  concatenating a `LEGACY_FIELD="baseRepository"` variable with the
  `--json` flag prefix, and the surrounding comments avoid the literal
  pair. The plan's `assert_grep_empty ... "--json baseRepository"`
  literal would have self-matched.

---

## Phase 4: Smoke — Real-`gh` Field Allowlist Check

### Overview

Add a new harness at
`skills/github/scripts/test-pr-base-repo-real-gh.sh` that runs against
the real `gh` on `PATH` (skipping cleanly if `gh` is absent) and
asserts every `--json` field the resolver requests is accepted by the
installed `gh pr view`. This is the corrective layer for the
structural blind spot in the existing PATH-stubbed harness, which
dispatches on `$1 $2` alone and cannot validate `gh`-side field
allowlists.

**Probe via error message, not help text.** The harness invokes
`gh pr view --json INVALID_FIELD 2>&1`, captures the
`Unknown JSON field: "INVALID_FIELD"` error gh emits along with the
allowlist, and parses the allowlist from that error. This is the
same runtime surface the resolver itself hits when a field is
missing — by construction, what passes the smoke check is accepted at
runtime, and a field-removal regression on a future `gh` release is
guaranteed to be caught. Help-text scraping was considered and
rejected: the help format is loosely structured and a short token
like `url` can appear in flag descriptions outside the JSON-fields
allowlist, producing false PASSes.

### Changes Required:

#### 1. New file: `skills/github/scripts/test-pr-base-repo-real-gh.sh`

```bash
#!/usr/bin/env bash
# set -e intentionally omitted so a failing assertion does not abort
# the harness — assertion failures tally into FAIL and the suite runs
# to completion. Mirrors the convention from sibling test scripts.
set -uo pipefail

# Real-gh smoke harness for skills/github/scripts/pr-base-repo.sh.
# Asserts every --json field the resolver requests is in the allowlist
# printed by `gh pr view --json INVALID` on the installed gh.
#
# Skipped (does not fail) if `gh` is not on PATH.
#
# Picked up automatically by run_shell_suites in
# tasks/test/helpers.py (globs **/test-*.sh under skills/github/).
#
# Rationale: the sibling PATH-stubbed harness in
# `test-pr-base-repo-scripts.sh` dispatches on `$1 $2` alone and cannot
# detect cases where the resolver requests a --json field that is not
# in the installed gh's allowlist. Work item 0071 documents the
# specific defect this catches.
#
# Why probe via `gh pr view --json INVALID` rather than scrape
# `gh pr view --help`: gh's error path emits a structured allowlist
# (`Unknown JSON field: "INVALID"\nAvailable fields: ...`) that is the
# same surface the resolver itself hits at runtime, so what passes
# this check is guaranteed to be accepted at runtime. The help text,
# by contrast, mentions short field tokens like `url` in flag
# descriptions outside the allowlist, producing false PASSes on
# real regressions.
#
# github.com-only: the probe targets the gh-cli project's own repo
# (cli/cli) to force gh past argv/repo-discovery into field validation.
# Operators with `GH_HOST` set to a GitHub Enterprise host will see
# the harness SKIP (via the marker-sanity guard below) rather than
# falsely FAIL — see the probe-invocation comment block for details.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=/dev/null
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
# Intentionally do NOT source skills/github/scripts/test-helpers.sh —
# this harness exercises the real gh, not the PATH stub.

SCRIPT="$SCRIPT_DIR/pr-base-repo.sh"

echo "=== pr-base-repo.sh real-gh smoke ==="

if ! command -v gh >/dev/null 2>&1; then
  skip_test "real-gh smoke" "gh not on PATH"
  test_summary
  exit 0
fi

# Extract every --json field the resolver requests from its source.
# Greps for `--json <token>` and emits the token. If the resolver
# evolves to request multiple fields in one call (e.g.
# `--json url,state`), the comma-split handles it. The field-name
# character class [A-Za-z][A-Za-z0-9_,]* is safe to interpolate into
# the ERE allowlist regex below without escaping.
fields=$(
  grep -oE -- "--json [A-Za-z][A-Za-z0-9_,]*" "$SCRIPT" \
    | awk '{print $2}' \
    | tr ',' '\n' \
    | sort -u
)

if [ -z "$fields" ]; then
  echo "  FAIL: real-gh smoke: no --json fields extracted from $SCRIPT"
  echo "    (resolver source may have refactored to a variable; see"
  echo "     meta/work/0071-*.md and pr-base-repo.sh header for context)"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi

# Probe gh's allowlist by deliberately requesting an invalid field.
# Capture stderr only — stdout is empty on this error path. Use a
# field name so unusual it cannot collide with a real gh field.
#
# Why pass `1 --repo cli/cli`: `gh pr view` validates positional args
# and resolves the repo context before validating the --json field set.
# Without a PR number we'd hit `accepts 1 arg(s)`; without --repo we'd
# hit `no git remotes found` in CI / outside git checkouts. Passing a
# known-stable public repo + an arbitrary PR number forces gh past
# the argv/repo-discovery checks and into field validation, which is
# the surface we actually want to probe. The PR may or may not exist
# on cli/cli — irrelevant, because field validation is parse-time and
# fires before network fetch.
#
# github.com-only by design: cli/cli is the gh-cli project's own repo.
# Operators with `GH_HOST` set to a GitHub Enterprise host won't have
# access to cli/cli; the probe will then hit a repo-resolution error
# *before* reaching field validation, the marker-sanity check below
# will fail to find `Unknown JSON field`, and the harness SKIPs with
# diagnostic stderr. This is the documented degradation path for GHE
# — see the GHE manual-verification step in Phase 4 success criteria.
# If GHE coverage becomes a hard requirement, replace cli/cli with
# dynamic discovery via `gh repo list --limit 1 --json nameWithOwner`.
PROBE_FIELD='__ACCEL_PROBE__'
probe_stderr=$(gh pr view 1 --repo cli/cli --json "$PROBE_FIELD" 2>&1 1>/dev/null || true)

# Format-sanity check: the probe MUST emit gh's canonical
# `Unknown JSON field` marker. If it doesn't, gh either reached a
# different error path (auth missing, network down) or has changed
# its error format. Either way, we cannot reliably parse an allowlist,
# so SKIP rather than emit field FAILs with misleading attribution.
if ! grep -q "Unknown JSON field" <<<"$probe_stderr"; then
  skip_test "real-gh smoke" \
    "gh did not emit expected 'Unknown JSON field' marker — auth, network, or error-format issue. Captured stderr:
$probe_stderr
(see meta/work/0071-*.md and pr-base-repo.sh header for context)"
  test_summary
  exit 0
fi

# gh's "Unknown JSON field" error includes a list of valid fields,
# typically rendered as a comma-separated allowlist on subsequent
# lines. Strip the marker line, then collapse the rest into a
# single newline-delimited token stream we can grep against.
# `awk '{print $1}'` takes the first whitespace-delimited token of
# each line — gh may render the allowlist as `Specify one of: a, b, c`
# (commas converted to newlines, awk strips trailing prose) or as a
# bullet-per-line list (awk strips any non-token prefix). The first
# token per line after comma-splitting is the field identifier.
allowlist_tokens=$(
  printf '%s\n' "$probe_stderr" \
    | grep -v "Unknown JSON field" \
    | tr ',' '\n' \
    | awk '{print $1}' \
    | grep -E '^[A-Za-z][A-Za-z0-9_]*$' \
    | sort -u
)

# Control-field check: assert a known-stable gh field is in the parsed
# allowlist. If our parser is broken (gh changed its rendering, the
# stripping logic dropped real tokens), this assertion catches the
# parser bug before it manifests as misleading per-field FAILs.
CONTROL_FIELD='number'
if ! printf '%s\n' "$allowlist_tokens" | grep -qx -- "$CONTROL_FIELD"; then
  echo "  FAIL: real-gh smoke: parser sanity check — control field '$CONTROL_FIELD' not in parsed allowlist"
  echo "    (parser may be broken; gh's error format may have changed)"
  echo "    Captured stderr:"
  printf '%s\n' "$probe_stderr" | sed 's/^/      /'
  echo "    Parsed tokens:"
  printf '%s\n' "$allowlist_tokens" | sed 's/^/      /'
  echo "    (see meta/work/0071-*.md and pr-base-repo.sh header)"
  FAIL=$((FAIL + 1))
  test_summary
  exit 1
fi

for field in $fields; do
  if printf '%s\n' "$allowlist_tokens" | grep -qx -- "$field"; then
    echo "  PASS: real-gh smoke: '$field' in gh's allowlist"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: real-gh smoke: '$field' NOT in installed gh's allowlist"
    echo "    (installed gh: $(gh --version 2>/dev/null | head -n 1))"
    echo "    (resolver: $SCRIPT)"
    echo "    (see meta/work/0071-*.md and pr-base-repo.sh header;"
    echo "     a gh upgrade may have dropped this field from --json)"
    FAIL=$((FAIL + 1))
  fi
done

test_summary
```

Make the file executable (`chmod +x`) so `run_shell_suites` picks it
up.

### Success Criteria:

#### Automated Verification:

- [ ] `bash skills/github/scripts/test-pr-base-repo-real-gh.sh` exits
  0 on the workspace's pinned `gh 2.89.0` and reports a PASS for the
  `url` field.
- [ ] `mise run test:integration:github` includes the new harness in
  its output (autodiscovery).
- [ ] When `gh` is not on `PATH`, the harness reports "skipped" and
  still exits 0. Reproduce by temporarily masking `gh` with
  `PATH=/usr/bin bash skills/github/scripts/test-pr-base-repo-real-gh.sh`
  (assumes `gh` is not in `/usr/bin`; otherwise use a tmpdir-only
  PATH).
- [ ] `shellcheck skills/github/scripts/test-pr-base-repo-real-gh.sh`
  exits 0.
#### Manual Verification:

**Precondition for the walkthroughs below**: each walkthrough must
start from a clean working tree. After running each walkthrough,
verify with `jj diff skills/github/scripts/` (or `git diff` if
adapted) that no edits remain before starting the next. The
walkthroughs that mutate `pr-base-repo.sh` are caught by test 24 (the
tree-state guard) if forgotten, but the walkthroughs that mutate the
smoke harness itself (`PROBE_FIELD`, `CONTROL_FIELD`) have no
automated guard — a forgotten revert would silently disable the
diagnostic branches.

- [ ] **Counterfactual check** (proves the smoke check catches the
  "field-not-in-allowlist" defect class — the failure shape that
  motivated work item 0071): temporarily edit `pr-base-repo.sh` to
  request a guaranteed-absent synthetic field — for example
  `--json __nonexistent_test_field__` (do NOT commit). Confirm the
  smoke check reports FAIL on the installed `gh` (version-independent
  because the synthetic field is never in any gh's allowlist), with
  the FAIL message naming the field, installed gh version, and
  pointing at work item 0071. Revert the edit. NOTE: using
  `--json baseRepository` would NOT work here — gh 2.89.0 (the
  workspace's pinned version) accepts `baseRepository`, so the smoke
  check would PASS, defeating the demo. The synthetic field makes
  the outcome version-independent.
- [ ] **Probe-format check** (anchors the parser to gh's actual
  output): run
  `gh pr view 1 --repo cli/cli --json __ACCEL_PROBE__ 2>&1` and
  confirm the output starts with `Unknown JSON field` followed by a
  parseable allowlist (comma-separated bare tokens, or one bare token
  per line).
- [ ] **Marker-sanity SKIP path** (proves the harness SKIPs cleanly
  on a non-canonical error path): temporarily edit the harness to set
  `PROBE_FIELD='url'` so the probe succeeds without emitting any
  `Unknown JSON field` line. Run the harness and confirm it reports
  SKIP with diagnostic stderr — not per-field FAILs. Revert.
- [ ] **Control-field FAIL path** (proves the parser-sanity guard
  fires): temporarily edit the harness to set
  `CONTROL_FIELD='__nonexistent_field__'`. Run and confirm the
  harness reports `parser sanity check — control field '...' not in
  parsed allowlist` and exits 1. Revert.
- [ ] **Auth-absent check** (proves the harness doesn't false-FAIL
  on unauthenticated machines): run the harness in a subshell with
  auth env vars cleared:
  `( unset GH_TOKEN GITHUB_TOKEN; bash skills/github/scripts/test-pr-base-repo-real-gh.sh )`.
  The subshell scopes the unset, so no manual revert is needed. NOTE:
  on macOS machines with `gh auth login`-stored keychain credentials,
  unsetting the env vars may not actually unauthenticate gh (gh
  consults the keychain independently). For a definitive
  unauthenticated test, run in a fresh-HOME subshell:
  `( unset GH_TOKEN GITHUB_TOKEN; HOME=$(mktemp -d) bash skills/github/scripts/test-pr-base-repo-real-gh.sh )`.
  Confirm the harness either passes (if gh reaches field validation
  before auth) or emits the SKIP path with captured stderr — not a
  stream of per-field FAILs. Expected SKIP-path stderr contains gh's
  auth error (e.g. `authentication required`), not
  `Unknown JSON field`.
- [ ] **GHE host check** (proves the github.com-only assumption is
  honest): discover your configured host via `gh auth status` — the
  `Logged in to <host>` line names it. If `<host>` is not
  `github.com`, run the harness and confirm it emits SKIP with
  diagnostic stderr (the enterprise host cannot resolve `cli/cli`)
  rather than a stream of per-field FAILs. If you are on github.com,
  exercise the same path synthetically by running the harness with
  `GH_HOST` overridden to a known-invalid host:
  `( GH_HOST=github.acme.invalid bash skills/github/scripts/test-pr-base-repo-real-gh.sh )`
  — the subshell scopes the override, no manual revert is needed.
  Confirm SKIP fires with diagnostic stderr.
- [ ] On `gh 2.65.0`: confirm the smoke check passes (the `url`
  field is in 2.65.0's allowlist). This is the critical version for
  the bug.
- [ ] Inspect the harness output and confirm the failure message on
  a synthetic FAIL is actionable (names the field, the installed gh
  version, the script, and points at work item 0071 / the resolver
  header for context).

---

## Testing Strategy

### Unit Tests:

- The two existing PATH-stubbed harnesses
  (`test-pr-base-repo-scripts.sh`,
  `test-pr-update-body-scripts.sh`) are the unit-test surface and
  remain green at `PHASE=final` after this plan.
- The reshaped tests cover: same-repo, upstream-URL parsing, GHE
  host, percent-encoded-charset rejection, argv shape, malformed-URL
  guard, truncated-URL guard, missing-field, JSON-parse guard,
  non-JSON guard, usage, stderr replay, conditional hint, missing-jq
  preflight.
- **Cross-fork-safety is not covered by automated tests** by
  construction: `install_fake_gh` dispatches on `$1 $2` alone and
  cannot model gh's fork-vs-upstream behaviour. The renamed test 4
  ("upstream URL parses to upstream coords") verifies only the
  parsing branch; the real cross-fork invariant is verified by a
  dedicated enumerated manual step in Phase 2.
- Behaviour under auth / network / not-found failure modes (work
  item AC #3) is fully determined by the `gh`-stub interface; tests
  6, 7, 11 and 12 already cover these surfaces. No additional unit
  coverage is required.

### Integration Tests:

- The new real-`gh` smoke check
  (`test-pr-base-repo-real-gh.sh`) runs as part of
  `mise run test:integration:github` and exercises the *installed*
  `gh`'s field allowlist via the same error-message surface the
  resolver hits at runtime — the only gh-version-specific dimension
  that matters for this defect class.

### Manual Testing Steps:

1. **Cross-fork-safety**: from inside a fork checkout of a real open
   cross-fork PR, run `gh pr view <pr> --json url --jq .url` and
   confirm the returned URL has upstream host/owner/repo segments.
   Record the captured output in the PR description.
2. On `gh 2.65.0`: confirm `pr-update-body.sh` succeeds against a
   real same-repo open PR; check PR body on GitHub matches input.
3. On `gh 2.65.0`: confirm `pr-update-body.sh` succeeds against a
   real cross-fork open PR; check PATCH targeted upstream coords.
4. On `gh 2.65.0`: run `/accelerator:describe-pr`,
   `/accelerator:review-pr`, and `/accelerator:respond-to-pr`
   end-to-end against an open PR; verify each completes its post step
   without manual intervention.
5. On `gh 2.65.0`: run the real-`gh` smoke check; confirm it passes.
6. Confirm `grep -rF -- "--json baseRepository" skills/github/`
   returns nothing.

## Performance Considerations

None. The resolver's single shell-out and `jq` invocation are
unchanged in shape; URL regex extraction is in-process bash and
constant-time relative to the existing implementation.

## Migration Notes

No data migration. No SKILL.md edits. No `allowed-tools` frontmatter
changes — the resolver path and exit-code contract are byte-identical.
Downstream operators on `gh 2.65.0` who were previously unable to
complete the post step are unblocked the moment the plugin ships a
release with this fix.

**Rollback**: the structured `baseRepository` code path is removed
entirely (no dual-path fallback). Recovery from a future gh-side
surprise — for example, `url` being removed from the `--json`
allowlist on a future `gh` release — requires landing a new plan;
the immediate rollback path is `git revert` of this change. The
Phase 4 smoke harness gives operators an early-warning signal on `gh`
upgrades: a failed run on a previously-green CI is the cue to file
a follow-up work item. The recovery window is therefore bounded by
integration-test cadence rather than end-user reports.

**Out-of-range `gh`**: users running a `gh` release whose `--json`
allowlist omits `url` will see the same shape of `Unknown JSON field`
error this plan eliminates for `baseRepository` on `gh 2.65.0` —
just for a different field. The resolver makes no static `gh`
version assertion; the smoke harness is the source of truth and will
fail with an actionable diagnostic pointing at work item 0071.

## References

- Work item: `meta/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field.md`
- Research: `meta/research/codebase/2026-05-18-0071-describe-pr-base-repo-resolver-unsupported-gh-field.md`
- Work item review: `meta/reviews/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field-review-1.md`
- Precedent plan (introduced the broken resolver): `meta/plans/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`
- Precedent research: `meta/research/codebase/2026-05-15-0059-gh-pr-edit-projects-classic-deprecation.md`
- ADR-0010 (gh api drop-down precedent): `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`
- Resolver source: `skills/github/scripts/pr-base-repo.sh`
- Body-updater source: `skills/github/describe-pr/scripts/pr-update-body.sh`
- Existing harness: `skills/github/scripts/test-pr-base-repo-scripts.sh`
- Sibling harness: `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
- Test runner wiring: `tasks/test/integration.py:46-49`,
  `tasks/test/helpers.py:13-34` (`run_shell_suites` autodiscovery)
