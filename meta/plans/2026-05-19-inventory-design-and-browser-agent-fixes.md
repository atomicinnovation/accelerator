---
date: "2026-05-19T08:36:10Z"
type: plan
skill: create-plan
work-item: ""
status: approved
---

# Inventory-Design and Browser-Agent Fixes Implementation Plan

## Overview

Fix the four independent defects surfaced by the 2026-05-19 `/accelerator:inventory-design`
run. The fixes are intentionally focused: each phase addresses one defect,
follows test-driven development, and leaves the broader inventory-design
workflow unchanged.

## Current State Analysis

The four defects, each from a separate note under `meta/notes/2026-05-19-*.md`:

1. **Browser agents self-discover the Playwright executor.** Neither
   `agents/browser-locator.md` nor `agents/browser-analyser.md` is told where
   `run.sh` lives, even though the parent `inventory-design` skill resolves
   the path explicitly in its own bash blocks. Agents end up running
   `which run.sh || find / -name run.sh`.

2. **`browser-locator` cannot enumerate routes.** Its allowed commands
   (`navigate`, `snapshot`) cannot surface URLs on a client-side SPA — the
   accessibility-tree snapshot exposes only ARIA `role` and `name`. The two
   commands that could (`evaluate`, `click`) are explicitly forbidden. On the
   visualiser specifically every URL returns the same 1073-byte SPA shell
   with HTTP 200, so navigation to a fabricated path is indistinguishable
   from a real one.

3. **`config-read-path.sh` prints `warning: unknown key`.** Five SKILL.md
   preambles still call the bare `design_inventories` / `design_gaps` keys,
   but migration 0004 renamed them to `research_design_inventories` /
   `research_design_gaps`. The warning bleeds into the rendered skill
   preamble where a path should be.

4. **Playwright daemon dies between Claude Code turns.** `run.sh` passes the
   launcher shell's own `$$` as `--owner-pid`. Under the Claude Code Bash
   tool the launcher shell is ephemeral and exits as soon as `run.sh`
   returns, so by the time the daemon's 60s owner-watcher tick fires, the
   PID is already gone and the daemon shuts down with reason `owner-exited`.
   Every agent turn starts a fresh daemon and loses any browser context the
   previous turn established. The owner-PID watcher is the **only**
   shutdown mechanism that depends on the launcher's lifetime; the daemon
   also has an idle timer (30 min), a per-op wall-clock budget (5 min),
   explicit `daemon-stop`, and SIGTERM/SIGINT handling. The watcher's only
   marginal value is "kill the daemon faster than the idle timer when an
   interactive user exits their terminal without `daemon-stop`" — which
   does not justify breaking the primary (Claude Code) use case.

### Key Discoveries:

- `agents/documents-locator.md:10-12` already uses `skills: - accelerator:paths`
  to inject a "Configured Paths" block before the agent acts. The same
  pattern works for the executor path.
- `skills/config/paths/SKILL.md:1-22` is the preloaded-skill shape to mirror,
  with `user-invocable: false` (NOT `disable-model-invocation: true`, which
  blocks preloading per a maintainer comment at lines 11-16).
- `init.sh:18-31` already uses the canonical `research_design_*` keys,
  so the bare-key problem is isolated to the SKILL.md preambles.
- Migration 0004
  (`skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:414-415`)
  is a **hard rename**, not a soft alias. The user has confirmed we should
  trust the migration and rename call sites rather than add aliases. We
  will also add a migration-aware warning to `config-read-path.sh` so users
  who upgrade the plugin before running `/accelerator:migrate` get a clear
  hint instead of a silent fallback to defaults.
- The five affected SKILL.md call sites are:
  - `skills/design/inventory-design/SKILL.md:30`
  - `skills/design/analyse-design-gaps/SKILL.md:27, 28`
  - `skills/config/init/SKILL.md:31, 32`
  - `skills/visualisation/visualise/SKILL.md:27, 28`
- The Playwright daemon command-dispatch surface is at
  `scripts/playwright/lib/daemon.js:113-224`. Adding a new command means a
  new `case` in the switch, mirroring the existing `evaluate` shape at
  `:178-182`.
- The daemon has **one daemon per project state dir**
  (`$PROJECT_ROOT/.accelerator/tmp/inventory-design-playwright/`) enforced
  by `run.sh`'s reuse short-circuit + launcher-lock. Concurrent browser
  agents within the same project share that one daemon's `browser` and
  `page`; this is a pre-existing limitation that the plan does not change.
- `daemon.test.js:77, 99, 120, 141, 164` invokes the daemon with
  `--owner-pid 0` in every test, so the watcher's production code path is
  untested. After Phase 2, the watcher no longer exists and these
  `--owner-pid 0` arguments are removed.
- `PROTOCOL.md` (in the playwright/ directory) is the canonical wire-protocol
  reference for the executor. Every existing command has a subsection.

## Desired End State

- A run of `/accelerator:inventory-design` against the visualiser produces
  no `unknown key` warnings, no `find /` discovery commands, no daemon
  death between turns, and no fabricated routes.
- Both `browser-locator` and `browser-analyser` receive a resolved executor
  path via a preloaded `skills:` block; their agent bodies reference that
  resolved value rather than the bare `run.sh` token.
- `browser-locator` has a `links` command available so it can enumerate
  routes on an SPA without needing JavaScript execution. The command
  returns enriched, server-normalised entries (resolved URL, pathname,
  same-origin flag, scheme) so the agent does not need to guess.
- The five remaining bare-key call sites (`design_inventories`,
  `design_gaps`) use the canonical `research_design_*` keys. Users who have
  not yet run migration 0004 see a clear hint from `config-read-path.sh`.
- The Playwright daemon's owner-PID watcher is removed; the daemon is
  bounded by an idle timer (lowered from 30 min to 10 min), the per-op
  wall-clock budget (unchanged at 5 min), explicit `daemon-stop`, and
  process signals. The daemon naturally survives across Claude Code Bash
  invocations, with no env-var contract or caller discipline required.
- `PROTOCOL.md` documents the new `links` command and the daemon
  environment-variable surface.
- `CHANGELOG.md` records each fix in `[Unreleased]`.

### Verification

- `bash scripts/test-design.sh` passes
- `bash scripts/test-config.sh` passes
- `node --test skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
  passes (with watcher tests removed and an explicit idle-default test added)
- `bash skills/design/inventory-design/scripts/playwright/test-run.sh` passes,
  including the new `links` test
- Manual run of `/accelerator:inventory-design current-app http://localhost:5173`
  on the visualiser produces an inventory with **real** routes (not
  fabricated ones) and no path-resolution warnings

## What We're NOT Doing

- **No new alias keys.** `paths.design_inventories` / `paths.design_gaps`
  remain absent from `config-defaults.sh`. Users keep using
  `paths.research_design_*` after migration 0004.
- **No restructuring of `inventory-design` Step 8.** The locator-agent pre-
  extraction reshape (open question 3 in the research) is out of scope.
- **No expansion of `evaluate` policy.** `click`, `type`, and arbitrary
  `evaluate` remain forbidden for `browser-locator`.
- **No changes to other agents** beyond `browser-locator` and
  `browser-analyser`. `browser-analyser` does **not** gain `links` (the
  earlier "for symmetry" rationale was insufficient; add only when a
  concrete use case appears).
- **No new concurrency support.** One-daemon-per-project remains; concurrent
  browser agents in the same project keep their existing limitations.
- **No folding of `browser-executor` into the `paths` skill.** The `paths`
  skill is project-configurable document directories; the executor path is
  a plugin-internal absolute path. Keep them conceptually separate; address
  drift with `test -x` validation in the resolver script.

## Implementation Approach

Five phases, ordered from lowest-risk to highest-impact. Phases 1–4 each
address one defect. Phase 5 captures cross-cutting documentation
(PROTOCOL.md, CHANGELOG.md). Each phase follows TDD: failing test or
assertion first, then the production fix, then green-pass verification.

---

## Phase 1: Rename design-key call sites to `research_design_*`

### Overview

Eliminate the `config-read-path.sh: warning: unknown key` noise by updating
the five remaining SKILL.md preambles to call the canonical key names that
migration 0004 introduced, and by giving `config-read-path.sh` a
migration-aware warning for users who upgrade before running the migration.

### Changes Required:

#### 1. Strengthen and broaden `test-design.sh` assertions

**File**: `scripts/test-design.sh`
**Changes**: Strengthen the existing assertions to require the
`research_design_*` form, and replace the per-skill enumeration in the
draft with a single repo-wide grep so any future skill that adopts the
bare-key form is caught automatically.

```bash
# Replace lines 13-16
assert_contains "init lists research_design_inventories path key" \
  "$(cat "$INIT")" "research_design_inventories"
assert_contains "init lists research_design_gaps path key" \
  "$(cat "$INIT")" "research_design_gaps"

# Replace lines 32-35
assert_contains "configure paths table includes research_design_inventories" \
  "$(cat "$CONFIGURE")" "research_design_inventories"
assert_contains "configure paths table includes research_design_gaps" \
  "$(cat "$CONFIGURE")" "research_design_gaps"

# Add: repo-wide check that no SKILL.md (or agent) calls the bare keys.
# Word-bounded grep so `research_design_inventories` does not match the
# bare `design_inventories` form.
echo "=== Design key call sites use canonical research_design_* form ==="
assert_exit_code "no SKILL.md or agent uses bare design_(inventories|gaps)" 1 \
  bash -c "grep -rE 'config-read-path\\.sh[[:space:]]+design_(inventories|gaps)\\b' \"$PLUGIN_ROOT/skills\" \"$PLUGIN_ROOT/agents\""
```

Run the test suite to confirm it now fails (red).

#### 2. Update the five SKILL.md call sites

**File**: `skills/design/inventory-design/SKILL.md`
**Changes**: Line 30 — `design_inventories` → `research_design_inventories`.

**File**: `skills/design/analyse-design-gaps/SKILL.md`
**Changes**: Lines 27-28 — `design_inventories` → `research_design_inventories`
and `design_gaps` → `research_design_gaps`.

**File**: `skills/config/init/SKILL.md`
**Changes**: Lines 31-32 — same two substitutions.

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: Lines 27-28 — same two substitutions.

After all four files are updated, the test suite (red → green).

#### 3. Add migration-aware warning to `config-read-path.sh`

**File**: `scripts/config-read-path.sh`
**Changes**: Emit a stderr warning in TWO scenarios:

1. **Bare-key call site** (defensive): if a skill author or external
   caller still invokes `config-read-path.sh design_inventories`, name
   the migration in the warning instead of emitting the generic
   "unknown key" message.

2. **Pre-migration user with legacy override** (the actually-load-bearing
   case): when the canonical `research_design_*` key is looked up — which
   is what every plugin call site does after Phase 1 Step 2 — probe the
   user's config for the legacy `design_*` alias. If present, the user's
   override is being silently ignored, so emit a warning naming the
   ignored key and pointing at `/accelerator:migrate`.

This second case is the one that actually fires for users in production
(plugin code uses canonical keys; only their config is stale). The first
case is a defence against future skill-author drift.

**Also add a `source` line near the top of the script** (next to the
existing `source "$SCRIPT_DIR/config-defaults.sh"` at line 19), to pull
in `find_repo_root` for the cheap-gate. `find_repo_root` is pure-bash
directory-walk with no VCS-detection cost (the cost comment in
config-defaults.sh refers to config-common.sh's
`config_assert_no_legacy_layout`, not vcs-common.sh):

```bash
# shellcheck source=vcs-common.sh
# Sourced for find_repo_root, used by the migration-aware warning's
# cheap-gate below.
source "$SCRIPT_DIR/vcs-common.sh"
```

```bash
key="${1:-}"
if [ -z "$key" ]; then
  echo "Usage: config-read-path.sh <path_key> [default]" >&2
  exit 1
fi

if [ -n "${2:-}" ]; then
  default="${2}"
else
  default=""
  for i in "${!PATH_KEYS[@]}"; do
    if [ "${PATH_KEYS[$i]}" = "paths.${key}" ]; then
      default="${PATH_DEFAULTS[$i]}"
      break
    fi
  done
  if [ -z "$default" ]; then
    case "$key" in
      design_inventories|design_gaps)
        # Defensive: a caller (skill author, external script) is still
        # invoking the legacy bare key. The plan removed all in-tree call
        # sites in Phase 1 Step 2, so this only fires for out-of-tree
        # callers.
        echo "config-read-path.sh: warning: key '${key}' was renamed by migration 0004 to 'research_${key}'; run /accelerator:migrate" >&2
        ;;
      *)
        echo "config-read-path.sh: warning: unknown key '${key}' — no centralized default" >&2
        ;;
    esac
  fi
fi

# Pre-migration user check: when the canonical key is requested, probe
# the user's config for the legacy alias. If present, their override is
# silently being ignored — emit a warning naming the ignored key.
#
# Cheap-gate: most users have already run /accelerator:migrate or never
# had the legacy key, so a fast grep over the config files at the repo
# root avoids the config-read-value.sh subprocess fork on every
# canonical-key resolution.
#
# Path resolution: use find_repo_root (sourced from vcs-common.sh) to
# match the resolution semantics of config-read-value.sh's downstream
# config_find_files. Earlier drafts used `$PWD/.accelerator/...` here,
# but that misses the config when the caller's CWD is a subdirectory of
# the project root — a common invocation case for skill preambles. Fall
# through silently if no repo root is found (the warning is informational
# only; missing it is acceptable when the script runs outside a repo).
case "$key" in
  research_design_inventories|research_design_gaps)
    legacy="${key#research_}"
    project_root="$(find_repo_root 2>/dev/null || true)"
    if [ -n "$project_root" ] && \
       grep -qF "$legacy" \
         "$project_root/.accelerator/config.md" \
         "$project_root/.accelerator/config.local.md" 2>/dev/null; then
      legacy_value=$(bash "$SCRIPT_DIR/config-read-value.sh" "paths.${legacy}" "" 2>/dev/null || true)
      if [ -n "$legacy_value" ]; then
        echo "config-read-path.sh: warning: your config sets 'paths.${legacy}' (renamed by migration 0004 to 'paths.${key}'); the legacy override is being ignored. Run /accelerator:migrate" >&2
      fi
    fi
    ;;
esac

exec "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}"
```

Add `test-config.sh` assertions for BOTH scenarios:

```bash
echo "Test: config-read-path.sh emits migration-aware warning for bare design_inventories (defensive)"
REPO=$(setup_repo)
ERR=$(cd "$REPO" && bash "$CONFIG_READ_PATH" design_inventories 2>&1 >/dev/null || true)
assert_contains "bare-key warning names migration 0004" "$ERR" "migration 0004"
assert_contains "bare-key warning names the canonical key" "$ERR" "research_design_inventories"

echo "Test: config-read-path.sh warns when canonical key called but legacy alias is in user config"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
printf -- '---\npaths:\n  design_inventories: my-custom-path\n  design_gaps: my-other-path\n---\n' > "$REPO/.accelerator/config.md"
ERR=$(cd "$REPO" && bash "$CONFIG_READ_PATH" research_design_inventories 2>&1 >/dev/null || true)
assert_contains "legacy-in-config warning names migration 0004" "$ERR" "migration 0004"
assert_contains "legacy-in-config warning names the ignored key" "$ERR" "paths.design_inventories"
assert_contains "legacy-in-config warning says override is ignored" "$ERR" "ignored"

ERR=$(cd "$REPO" && bash "$CONFIG_READ_PATH" research_design_gaps 2>&1 >/dev/null || true)
assert_contains "legacy-in-config warning fires for design_gaps too" "$ERR" "paths.design_gaps"

echo "Test: config-read-path.sh does NOT warn when no legacy alias is set"
REPO=$(setup_repo)
ERR=$(cd "$REPO" && bash "$CONFIG_READ_PATH" research_design_inventories 2>&1 >/dev/null || true)
assert_not_contains "no legacy-warning noise when user config is clean" "$ERR" "ignored"
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-design.sh` passes, including the new
      repo-wide assertion
- [x] `bash scripts/test-config.sh` passes, including the new
      migration-aware warning test
- [x] `grep -rE 'config-read-path\.sh[[:space:]]+design_(inventories|gaps)\b' skills/ agents/`
      returns no matches

#### Manual Verification:

- [ ] Render the `inventory-design` SKILL.md preamble (e.g. via
      `/accelerator:inventory-design` invocation that fails at validation,
      so the rendered preamble is visible). The "Design inventories
      directory" line shows a resolved path, no stderr warning.
- [ ] In a test repo where `.accelerator/config.md` still has
      `paths.design_inventories` (pre-migration state), invoke a skill
      that resolves `research_design_inventories`. The user sees the
      "your config sets 'paths.design_inventories' … override is being
      ignored. Run /accelerator:migrate" warning, naming the ignored
      legacy key — not silence (which is what they would have seen if
      the warning fired only on bare-key calls).

---

## Phase 2: Remove Playwright daemon owner-PID watcher and tighten idle timer

### Overview

The owner-PID watcher exists to shut down the daemon faster than the
30-minute idle timer when the launching shell exits. Under Claude Code's
Bash tool every launcher shell is ephemeral, so the watcher fires on every
invocation and destroys the daemon before subsequent agent turns can reuse
it. The watcher protects no correctness or safety invariant — the daemon
is already bounded by an idle timer, a per-op wall-clock budget, explicit
`daemon-stop`, and process signals.

The fix is to remove the watcher entirely (its production code path is
already untested; every existing test opts out via `--owner-pid 0`) and to
lower the idle-timer default from 30 minutes to 10 minutes so the daemon
still self-cleans on its own schedule.

### Changes Required:

#### 1. Update test fixtures to remove `--owner-pid` arguments and add an idle-default test (red)

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
**Changes**:

- Remove `'--owner-pid', '0'` from every `fork` call in the five existing
  tests (lines 77, 99, 120, 141, 164). The CLI argument is going away.

**File**: `skills/design/inventory-design/scripts/playwright/test-run.js`
**Changes**:

- Remove `'--owner-pid', '0'` from the `fork` call at line 68.

These two files exhaust the in-tree callers of the flag (verified by
`grep -rE 'owner-pid|ownerPid|OWNER_POLL' skills/design/inventory-design/scripts/playwright/`).

Then add a small new test in `daemon.test.js` that the daemon's default
`IDLE_MS` is 10 min, documented by reading the source. (We don't want a
10-minute wait in tests; the existing idle-timer test at line 159
overrides `ACCELERATOR_PLAYWRIGHT_IDLE_MS=300` and still works after the
change.)

```javascript
// -- IDLE_MS default ---------------------------------------------------

test('daemon module declares IDLE_MS default of 10 minutes', async () => {
  const src = readFileSync(
    new URL('./daemon.js', import.meta.url).pathname, 'utf8');
  // Pin the default value at the source level (avoid runtime probe
  // requiring a 10-minute wait).
  assert.match(src, /IDLE_MS\s*=\s*parseInt\(process\.env\.ACCELERATOR_PLAYWRIGHT_IDLE_MS\s*\|\|\s*'600000'/);
});
```

Run `node --test daemon.test.js` to confirm the IDLE_MS test fails (the
source still has `'1800000'`).

#### 2. Remove the watcher and lower IDLE_MS in `daemon.js`

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
**Changes**:

- Update the file-header comment at line 2 to drop the `--owner-pid <pid>`
  fragment from the launch-example. The new header reads:
  `// Launched via: node run.js daemon --state-dir <dir>`.
- Lower `IDLE_MS` default from `'1800000'` (30 min) to `'600000'` (10 min)
  at line 17.
- Delete `OWNER_POLL_MS` constant at line 21.
- Delete the `ownerWatcher` `setInterval` block at lines 88-97 (and the
  surrounding header comment).
- Remove `clearInterval(ownerWatcher)` from the `shutdown` function at
  line 43.
- Remove `ownerPid` from `startDaemon`'s parameter destructuring at line 25.

```javascript
// Header comment for the lowered IDLE_MS:
// 10-min default balances cross-turn daemon reuse (Claude Code sessions
// typically span minutes) against bounding the lifetime of an
// auth-bearing browser context held in memory. Override via
// ACCELERATOR_PLAYWRIGHT_IDLE_MS.
const IDLE_MS = parseInt(process.env.ACCELERATOR_PLAYWRIGHT_IDLE_MS || '600000', 10);
```

Run `node --test daemon.test.js` to confirm the IDLE_MS test now passes
and all existing tests still pass.

#### 3. Drop `--owner-pid` handling from `run.js`

**File**: `skills/design/inventory-design/scripts/playwright/run.js`
**Changes**:

- Update the file-header comment at line 4 to drop the `--owner-pid <pid>`
  fragment. New header: `// run.sh daemon launch: node run.js daemon --state-dir <dir>`.
- Remove the `--owner-pid` arg parsing at lines 21-25 and the
  `ownerPid` argument passed to `startDaemon` at line 31. The `--state-dir`
  arg remains. Unknown flags fall through silently — acceptable for an
  internal CLI.

```javascript
if (command === 'daemon') {
  let stateDirArg = stateDir;
  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--state-dir') stateDirArg = args[++i];
  }
  if (!stateDirArg) {
    process.stderr.write('run.js daemon: --state-dir is required\n');
    process.exit(2);
  }
  const { startDaemon } = await import('./lib/daemon.js');
  await startDaemon({ stateDir: stateDirArg });
}
```

#### 4. Drop `--owner-pid` from the launcher

**File**: `skills/design/inventory-design/scripts/playwright/run.sh`
**Changes**: Remove the `--owner-pid "$$"` argument from the daemon spawn at
lines 108-111.

```bash
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  >> "$BOOTSTRAP_LOG" 2>&1 &
```

#### 5. Add a smoke test in `test-run.sh` that the daemon survives launcher exit

**File**: `skills/design/inventory-design/scripts/playwright/test-run.sh`
**Changes**: Add a smoke test that:

1. Invokes `run.sh ping` in a sub-shell (so its `$$` exits).
2. Reads `server.pid` from the state dir.
3. Verifies the daemon process is still alive (`kill -0 $pid`) after
   a short wait.
4. Cleanly stops the daemon and asserts `server-stopped.json` reason is
   `daemon-stop` (NOT `owner-exited`).

```bash
echo "=== run.sh: daemon survives launcher shell exit (smoke test) ==="
# Smoke test: confirms the end-to-end happy path (daemon comes up, the
# sub-shell launcher exits cleanly, daemon survives, daemon-stop produces
# the expected reason). The actual regression guard against re-introducing
# an owner-PID watcher lives in test-design.sh as source-level grep
# assertions over the playwright/ tree — those are stronger because they
# fire regardless of timing.

PROJECT_TMP="$(mktemp -d)"
trap 'rm -rf "$PROJECT_TMP"' EXIT
export ACCELERATOR_PLAYWRIGHT_CACHE="${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}"

# Launch the daemon via run.sh ping; the sub-shell exits after ping returns.
(cd "$PROJECT_TMP" && bash "$RUN_SH" ping >/dev/null)

STATE_DIR="$PROJECT_TMP/.accelerator/tmp/inventory-design-playwright"
SERVER_PID="$(tr -cd '0-9' < "$STATE_DIR/server.pid")"
assert_not_empty "daemon wrote server.pid" "$SERVER_PID"

sleep 2
assert_exit_code "daemon process is still alive after launcher shell exited" 0 \
  kill -0 "$SERVER_PID"

# Clean stop and check the reason. `daemon-stop` is the expected reason;
# `owner-exited` would indicate the watcher had been silently restored.
(cd "$PROJECT_TMP" && bash "$RUN_SH" daemon-stop >/dev/null)
sleep 1
STOPPED_REASON="$(jq -r '.reason' "$STATE_DIR/server-stopped.json" 2>/dev/null || echo "")"
assert_eq "daemon stopped with reason daemon-stop (not owner-exited)" \
  "daemon-stop" "$STOPPED_REASON"
```

**Why this is a smoke test, not a regression test for the watcher
removal**: with the watcher gone, this assertion (`kill -0` after 2s)
would pass trivially. Even WITH the old watcher, the default
`OWNER_POLL_MS=60000` means the first watcher tick wouldn't have fired
within a 2-second window — so the test wouldn't have caught the old
bug at default settings either. The Phase 2 Step 6 source-level
`grep -rnE '\bownerPid\b|--owner-pid|\bOWNER_POLL_MS\b'` assertion over
the playwright/ tree is the genuine regression guard against
re-introducing any watcher via its original identifier forms; this
smoke test additionally pins the post-shutdown reason
(`daemon-stop`, not `owner-exited`), catching any re-introduction that
manages to evade the source-level grep by using renamed symbols.

#### 6. Update `test-design.sh` to assert the watcher is gone

**File**: `scripts/test-design.sh`
**Changes**: Add assertions pinning the watcher's removal so it cannot be
silently re-introduced:

```bash
echo "=== daemon: owner-PID watcher removed ==="
PLAYWRIGHT_DIR="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright"
# Repo-wide sweep: no source file under the playwright/ tree references
# any part of the watcher mechanism. Catches future regressions
# regardless of which file (or new test) reintroduces the symbol.
#
# Pattern notes:
#   - `ownerPid` / `OWNER_POLL_MS`: identifier-only forms used in JS.
#   - `--owner-pid`: the CLI flag form (matched literally with --).
#   - We deliberately do NOT match the bare string `owner-exited` here:
#     it is sufficiently generic that legitimate references in test
#     assertion messages or PROTOCOL.md cross-references would create
#     false positives. The three identifier forms above are sufficient
#     drift coverage because they are what would have to be present in
#     source code to re-introduce the watcher.
assert_exit_code "no watcher identifier references under playwright/ tree" 1 \
  grep -rnE '\bownerPid\b|--owner-pid|\bOWNER_POLL_MS\b' "$PLAYWRIGHT_DIR"
```

### Success Criteria:

#### Automated Verification:

- [ ] `node --test skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
      passes, including the new `IDLE_MS default of 10 minutes` test
- [ ] `bash skills/design/inventory-design/scripts/playwright/test-run.sh`
      passes, including the new launcher-shell-exit survival test
- [ ] `bash scripts/test-design.sh` passes, including the new
      watcher-removed assertion (repo-wide grep over the playwright/ tree)
- [ ] `grep -rnE '\bownerPid\b|--owner-pid|\bOWNER_POLL_MS\b' skills/design/inventory-design/scripts/playwright/`
      returns no matches (covers daemon.js, run.js, run.sh, daemon.test.js,
      test-run.js, and any future files). Note: the literal string
      `owner-exited` is deliberately NOT in the pattern — see the
      Phase 2 Step 6 comment for why.

#### Manual Verification:

- [ ] Run `/accelerator:inventory-design current-app http://localhost:5173`
      against the visualiser. After Step 4 completes, inspect
      `${PATH_TMP}/inventory-design-playwright/server-info.json` and
      `server.pid`. Wait 90 seconds (longer than the former
      `OWNER_POLL_MS`). The PID is still alive (`kill -0 <pid>` succeeds)
      and no `server-stopped.json` has appeared.
- [ ] After a successful inventory-design run, the daemon either has been
      stopped explicitly by Step 12 or will be reaped by the 10-min idle
      timer; in either case no orphan daemon survives indefinitely.

---

## Phase 3: Add `browser-executor` preloaded skill

### Overview

Adopt the `documents-locator` precedent (work item 0052) for the executor
path. A new preloaded skill resolves the absolute path of `run.sh` and
injects it into both browser agents' context at spawn time. The agents
stop self-discovering the executor and reference the resolved value
instead.

### Changes Required:

#### 1. Add a test for the new path-resolver script (red)

**File**: `scripts/test-design.sh` (new section at the end of the file,
before `test_summary`)
**Changes**: Assert that the new resolver script exists, is executable,
emits the expected block, and refuses to emit a stale path if `run.sh` is
missing.

```bash
echo "=== browser-executor preloaded skill ==="

EXEC_SCRIPT="$PLUGIN_ROOT/scripts/config-read-browser-executor.sh"
assert_file_exists "config-read-browser-executor.sh exists" "$EXEC_SCRIPT"
assert_file_executable "config-read-browser-executor.sh is executable" "$EXEC_SCRIPT"

EXEC_OUT="$("$EXEC_SCRIPT" 2>&1)"
EXPECTED_PATH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/run.sh"
assert_contains "browser-executor output begins with ## Browser Executor header" \
  "$EXEC_OUT" "## Browser Executor"
assert_contains "browser-executor output names browser-executor-script key" \
  "$EXEC_OUT" "- browser-executor-script:"
assert_contains "browser-executor output contains absolute run.sh path" \
  "$EXEC_OUT" "$EXPECTED_PATH"

# The resolver must fail loudly rather than emit a stale path if the
# target moves. Simulate by pointing the resolver at a non-existent path
# (override via env var).
NONEXISTENT_OUT="$(ACCELERATOR_BROWSER_EXECUTOR_OVERRIDE=/tmp/does-not-exist/run.sh "$EXEC_SCRIPT" 2>&1 || true)"
assert_contains "resolver refuses missing run.sh" \
  "$NONEXISTENT_OUT" "run.sh not found"

EXEC_SKILL="$PLUGIN_ROOT/skills/config/browser-executor/SKILL.md"
assert_file_exists "browser-executor SKILL.md exists" "$EXEC_SKILL"
assert_contains "browser-executor SKILL.md sets user-invocable: false" \
  "$(cat "$EXEC_SKILL")" "user-invocable: false"
assert_not_contains "browser-executor SKILL.md does not set disable-model-invocation: true" \
  "$(cat "$EXEC_SKILL")" "disable-model-invocation: true"
assert_contains "browser-executor SKILL.md invokes config-read-browser-executor.sh" \
  "$(cat "$EXEC_SKILL")" "config-read-browser-executor.sh"

for agent in agents/browser-locator.md agents/browser-analyser.md; do
  body="$(cat "$PLUGIN_ROOT/$agent")"
  assert_contains "$agent declares accelerator:browser-executor skill" \
    "$body" "accelerator:browser-executor"
  assert_contains "$agent has preload guard checking for Browser Executor block" \
    "$body" "Preload guard"
  assert_contains "$agent guard names the expected key" \
    "$body" "browser-executor-script:"
done
```

Run the test suite to confirm it fails (red).

#### 2. Create `scripts/config-read-browser-executor.sh`

**File**: `scripts/config-read-browser-executor.sh`
**Changes**: New script. Emits a `## Browser Executor` markdown block with
the absolute path of `run.sh`. Validates that the file exists and is
executable before emitting; exits non-zero with a clear error otherwise.
This addresses the drift concern by failing loudly rather than silently
emitting a stale path.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Resolves the absolute path of the Playwright executor (run.sh) for
# preloading into browser-agent contexts. Mirrors the shape of
# scripts/config-read-all-paths.sh.
#
# Note: the executor path is also referenced in the inventory-design
# SKILL.md `allowed-tools` glob. If you move run.sh, both this script and
# that glob need to update in lockstep. The `test -x` check below ensures
# this script fails loudly if the file moves without a coordinated edit.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Override hook for tests.
RUN_SH="${ACCELERATOR_BROWSER_EXECUTOR_OVERRIDE:-$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/run.sh}"

if [ ! -x "$RUN_SH" ]; then
  echo "config-read-browser-executor.sh: run.sh not found or not executable at $RUN_SH" >&2
  exit 1
fi

echo "## Browser Executor"
echo ""
echo "- browser-executor-script: $RUN_SH"
```

Make it executable: `chmod +x scripts/config-read-browser-executor.sh`.

#### 3. Create the preloaded skill

**File**: `skills/config/browser-executor/SKILL.md`
**Changes**: New file. Mirrors `skills/config/paths/SKILL.md` exactly in
shape (`user-invocable: false`, `allowed-tools` restricted to the
`config-*` glob), with a single bang call to the new script.

```markdown
---
name: browser-executor
description: Resolves the absolute path of the Playwright executor (run.sh)
  for browser agents. Preloaded by agent definitions that need to invoke
  the executor without self-discovery; not intended for direct user
  invocation.
user-invocable: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

<!--
Maintainer note: this skill uses `user-invocable: false` (hide from the /
menu) rather than `disable-model-invocation: true`, because the latter
blocks preload via subagent `skills:` frontmatter (per Claude Code docs).
Do not change to disable-model-invocation without re-reading the
subagents docs. The same constraint applies to the sibling `paths` skill.
-->

The path below is authoritative for invoking the Playwright executor.
Reference this resolved value from agent bodies — do not run `which run.sh`
or `find` to discover it.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-browser-executor.sh`
```

#### 4. Add `skills:` frontmatter to both browser agents

**File**: `agents/browser-locator.md`
**Changes**: Insert a `skills:` block after the `tools: Bash` line at line 7:

```yaml
tools: Bash
skills:
  - accelerator:browser-executor
```

Rewrite ONLY the Tools section header text (lines 36-47) to reference the
resolved path. Preserve all surrounding sections (Search Strategy,
Important Guidelines, What NOT to Do, Cleanup) intact.

```markdown
## Tools

Use the Playwright executor as the primary browser interface. The
absolute path of `run.sh` is provided in the **Browser Executor** block
injected into your context by the preloaded `browser-executor` skill.

**Preload guard (best-effort)**: Before taking any action, check that
your context contains a `## Browser Executor` block with a
`browser-executor-script:` key. If it does NOT, immediately stop and
surface this message to the user verbatim:

> The `accelerator:browser-executor` preloaded skill did not inject
> its Browser Executor block into this agent's context. The Playwright
> executor location cannot be resolved. Please report this to the
> plugin maintainer along with your Claude Code version; the verified
> baseline is recorded in the plugin README.

Then stop. Do not attempt to discover `run.sh` via `which`, `find`, or
any other fallback — the failure mode must remain visible.

This guard is best-effort defence-in-depth, not a hard guarantee:
self-introspection of preloaded context by an LLM is not always
reliable, and the version baseline (next paragraph) is the mechanical
companion. Maintainer note for future debugging: when this fires,
verify the `skills:` frontmatter on this agent and the Claude Code
subagent skills-preload mechanism against the baseline.

In the examples below, `{browser-executor-script}` is the placeholder
for the value of the `browser-executor-script` key in the **Browser
Executor** block. Substitute it literally with the resolved path. (The
curly-brace convention mirrors the `documents-locator` agent's
references to preloaded `paths` values like `{work}` and `{plans}`.)

```
{browser-executor-script} navigate '{"url":"<url>"}'
{browser-executor-script} snapshot
```
```

**File**: `agents/browser-analyser.md`
**Changes**: Same frontmatter insertion. Rewrite ONLY the Tools-section
header prose (immediately above the command list at lines 16-27) to add
the same `{browser-executor-script}` substitution instruction AND the
same preload guard (verbatim, with "Browser Executor block" /
"browser-executor-script" wording matched). **Preserve verbatim** the
full command list (`navigate`, `snapshot`, `screenshot`, `evaluate`,
`click`, `type`, `wait_for`), the `run.sh evaluate Payload Allowlist`
section (lines 62-95) with all forbidden-payload rationales, and the
error-handling guidance. Update each command-line example to use
`{browser-executor-script}` instead of bare `run.sh`.

#### 5. Have `inventory-design` rely on the preloaded contract

**File**: `skills/design/inventory-design/SKILL.md`
**Changes**: At Step 8 (lines 162-202), add a two-sentence note:

```
The `browser-locator` and `browser-analyser` agents receive the executor
path via their preloaded `accelerator:browser-executor` skill — they do
not need it in their spawn prompt. Previously the agents fell back to
`which run.sh` / `find / -name run.sh` because no contract told them
where the executor lived; this mirrors the work-item-0052 fix that gave
`documents-locator` a preloaded `accelerator:paths` skill.
```

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/config-read-browser-executor.sh` prints a
      `## Browser Executor` block with an absolute path to `run.sh`
- [ ] `ACCELERATOR_BROWSER_EXECUTOR_OVERRIDE=/tmp/missing bash scripts/config-read-browser-executor.sh`
      exits non-zero with a clear error
- [ ] `bash scripts/test-design.sh` passes the new browser-executor section
- [ ] `bash scripts/test-config.sh` passes
- [ ] `grep -A2 "^skills:" agents/browser-locator.md agents/browser-analyser.md`
      shows `accelerator:browser-executor` in both
- [ ] `grep "## run.sh evaluate Payload Allowlist" agents/browser-analyser.md`
      still finds the section (regression guard for the analyser rewrite)

#### Manual Verification:

- [ ] Run `/accelerator:inventory-design` and inspect the spawn prompt for
      `browser-locator` (visible in tool-call traces). The agent's
      preloaded context contains the `## Browser Executor` block with the
      resolved path.
- [ ] The agent's first bash invocation uses the resolved path, not
      `which run.sh` or `find / -name run.sh`.

---

## Phase 4: Add `links` command to the executor and update browser-locator

### Overview

The `browser-locator` cannot discover routes on a client-side SPA because
its allowed commands (`navigate`, `snapshot`) cannot surface `<a href>`
URLs. Add a `links` command to the executor that returns enriched,
server-normalised entries for every anchor on the current page. The
daemon does the URL-resolution, same-origin determination, and
whitespace-normalisation work so the locator agent doesn't have to.
Only `browser-locator` gains the command in its agent body;
`browser-analyser` does not (no concrete use case justifies it).

### Changes Required:

#### 1. Add a fixture-page test for `links` (red)

**File**: `skills/design/inventory-design/scripts/playwright/__fixtures__/links.html`
**Changes**: New fixture file with anchors covering: same-origin relative,
same-origin absolute, cross-origin, explicit role, `mailto:`, fragment-only,
and embedded whitespace in text.

```html
<!doctype html>
<html><body>
<a href="/work-items">Work items</a>
<a href="/library/work-items">  Library
  Items  </a>
<a href="https://example.com/external" role="button">External</a>
<a href="mailto:user@example.com">Contact</a>
<a href="#top"></a>
<a href="?q=foo">Search</a>
</body></html>
```

**File**: `skills/design/inventory-design/scripts/playwright/test-run.sh`
**Changes**: Add a behavioural test that navigates to the fixture and
asserts the enriched response shape.

```bash
echo "=== run.sh links command ==="
FIXTURE_PATH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/__fixtures__/links.html"
FIXTURE_URL="file://$FIXTURE_PATH"

bash "$RUN_SH" navigate "{\"url\":\"$FIXTURE_URL\"}" >/dev/null
LINKS_OUT="$(bash "$RUN_SH" links)"

# Envelope: includes the current page URL so callers can verify context.
assert_contains "links output names the current page URL" "$LINKS_OUT" '"url":"file://'
assert_contains "links output is JSON with links field" "$LINKS_OUT" '"links"'

# Same-origin relative paths are resolved into pathnames.
assert_contains "links output includes /work-items pathname" "$LINKS_OUT" '"pathname":"/work-items"'
assert_contains "links output includes /library/work-items pathname" "$LINKS_OUT" '"/library/work-items"'

# Whitespace normalised in text.
assert_contains "links output collapses internal whitespace in text" "$LINKS_OUT" '"text":"Library Items"'

# Role preserved verbatim (null when unset).
assert_contains "links output preserves explicit role" "$LINKS_OUT" '"role":"button"'
assert_contains "links output uses null role for anchors without role attribute" "$LINKS_OUT" '"role":null'

# Same-origin flag.
#
# Note: this fixture is served from file://, whose page origin is opaque
# ("null" per the HTML spec). With the opaque-origin guard
# (`u.origin === pageOrigin && u.origin !== 'null'`), EVERY anchor on a
# file:// page reports `same_origin: false` — including the relative
# /work-items anchor. That is correct: opaque-origin pages have no
# same-origin peers, so the locator's "filter same_origin: true" rule
# yields zero candidates and prevents accidental navigation. To exercise
# the `same_origin: true` branch we would need an HTTP origin fixture;
# that is out of scope for this plan.
assert_contains "links output marks cross-origin anchors as not same-origin" \
  "$LINKS_OUT" '"same_origin":false'
# The security-critical assertion: mailto: must NOT be marked
# same-origin on a file:// page (without the opaque-origin guard, both
# share the "null" origin string and would falsely match).
assert_contains "links output marks mailto: as cross-origin (opaque-origin guard)" \
  "$LINKS_OUT" '"scheme":"mailto"'
assert_not_contains "no anchor reports same_origin: true on a file:// page (opaque-origin guard)" \
  "$LINKS_OUT" '"same_origin":true'

# Scheme.
assert_contains "links output includes file scheme for relative same-origin" "$LINKS_OUT" '"scheme":"file"'
assert_contains "links output includes https scheme for absolute cross-origin" "$LINKS_OUT" '"scheme":"https"'
assert_contains "links output includes mailto scheme" "$LINKS_OUT" '"scheme":"mailto"'

# Critical: response MUST NOT include raw href or fully-resolved URL —
# these would leak query strings / fragments / tokens into the locator's
# context. The locator works exclusively off pathname + same_origin.
assert_not_contains "links response does not include raw 'href' field" "$LINKS_OUT" '"href"'
assert_not_contains "links response does not include fully-resolved 'resolved' field" "$LINKS_OUT" '"resolved"'
assert_not_contains "links response does not echo query string from ?q=foo anchor" "$LINKS_OUT" 'q=foo'
assert_not_contains "links response does not echo fragment from #top anchor" "$LINKS_OUT" '#top'

# Pre-navigate (about:blank) case: links returns an empty list with the
# blank URL envelope, not an error.
bash "$RUN_SH" daemon-stop >/dev/null
bash "$RUN_SH" navigate '{"url":"about:blank"}' >/dev/null
BLANK_OUT="$(bash "$RUN_SH" links)"
assert_contains "links on about:blank names the URL" "$BLANK_OUT" '"url":"about:blank"'
assert_contains "links on about:blank returns empty array" "$BLANK_OUT" '"links":[]'

bash "$RUN_SH" daemon-stop >/dev/null
```

Run the test suite to confirm it fails (red — `unknown-command: links`).

#### 2. Add the enriched `links` command to the daemon

**File**: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
**Changes**: Add `'links'` to `BLOCKING_OPS` at line 23 with a comment, and
add a new `case 'links':` to the switch after `case 'snapshot':` (around
line 161). The daemon does URL resolution, origin comparison, and
whitespace normalisation server-side so the locator agent does not have to.

```javascript
// BLOCKING_OPS membership: operations that perform browser I/O and must
// be wall-clock bounded. `links` joins the set because page.evaluate()
// can hang on a hostile page, mirroring the existing `evaluate` and
// `snapshot` entries.
const BLOCKING_OPS = new Set(['navigate', 'snapshot', 'links', 'screenshot', 'evaluate', 'click', 'type', 'wait_for']);
```

```javascript
case 'links': {
  const pageUrl = page.url();
  const links = await page.evaluate(() => {
    const pageOrigin = location.origin;
    return Array.from(document.querySelectorAll('a[href]')).map(a => {
      let pathname = null;
      let sameOrigin = false;
      let scheme = null;
      try {
        const u = new URL(a.href);
        pathname = u.pathname;
        scheme = u.protocol.replace(':', '');
        // Same-origin check: both origins must match AND must not be
        // opaque ('null'). Opaque-origin schemes (file:, data:,
        // javascript:, blob:) all share the literal "null" origin
        // string in browsers, so a naive `u.origin === pageOrigin`
        // would mark a `mailto:` or `javascript:void(0)` anchor on a
        // file:// page as same-origin, which is wrong both
        // semantically and as a security signal (the locator's
        // route-following rule trusts same_origin).
        sameOrigin = u.origin === pageOrigin && u.origin !== 'null';
      } catch {
        // href could not be resolved against the current document
        // (e.g. malformed). Leave the derived fields null.
      }
      return {
        text: (a.textContent || '').replace(/\s+/g, ' ').trim(),
        pathname,
        same_origin: sameOrigin,
        scheme,
        role: a.getAttribute('role'),  // verbatim; null when unset
      };
    });
  });
  return { protocol: PROTOCOL, url: pageUrl, links };
}
```

**Why no `href` or `resolved` in the response?** The locator only needs
`pathname` (the route identifier) and `same_origin` (the filter) to do
its job. Returning raw `href` or fully-resolved URLs would leak query
strings and fragments — which can contain auth tokens, OAuth codes,
session IDs, signed-URL signatures, and other secrets — into the
agent's transcript, tool-call traces, and any intermediate captures,
even though the final `inventory.md` is route-scrubbed elsewhere. By
constructing the response from scrubbed fields only, the daemon
maintains the URL-scrubbing trust boundary without depending on the
locator's prose discipline. If a future caller needs the raw or full
URL, add an explicit opt-in flag at that point — not as a default.

Run the fixture test to confirm it passes (green).

#### 3. Pin `links` into `BLOCKING_OPS` via source-level assertion

**File**: `scripts/test-design.sh`
**Changes**: Add an assertion that `'links'` appears in the `BLOCKING_OPS`
initialiser in `daemon.js`. This is sufficient regression coverage: the
wall-clock arming flow (`armWallClock` / `disarmWallClock` in
`handleRequest`) is unchanged by this plan and already covers every
member of the set identically, so set membership alone determines whether
the new command is bounded.

```bash
echo "=== daemon: links is in BLOCKING_OPS ==="
DAEMON_SRC="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/lib/daemon.js"
assert_contains "BLOCKING_OPS includes 'links'" \
  "$(grep -E '^const BLOCKING_OPS' "$DAEMON_SRC")" "'links'"
```

Why not a runtime test? An obvious approach — set
`ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=1`, navigate, then call `links` and
expect `wall-clock-exceeded` — does not work: `navigate` is also in
`BLOCKING_OPS`, so the 1ms timer fires on the navigate call first and
shuts the daemon down before `links` is dispatched. Constructing a test
that exercises `links`-specific wall-clock requires a hostile fixture
whose `page.evaluate(...)` hangs long enough to fire the timer; this is
out of proportion to the value (the arming flow is already covered
implicitly by the existing tests for other BLOCKING_OPS members). The
source-level assertion is the right level for this contract.

#### 4. Update `browser-locator.md` to allow and document `links`

**File**: `agents/browser-locator.md`
**Changes**:

Add `{browser-executor-script} links` to the Tools section (the section
header rewritten in Phase 3):

```
{browser-executor-script} navigate '{"url":"<url>"}'
{browser-executor-script} snapshot
{browser-executor-script} links
```

Update the Search Strategy to make `links` the primary route-discovery
mechanism, and explicitly require the agent to use `pathname` (server-
normalised) and `same_origin` (server-computed):

```markdown
## Search Strategy

1. Navigate to the application root using `{browser-executor-script} navigate '{"url":"<url>"}'`
2. Invoke `{browser-executor-script} links` to enumerate anchors on the
   current screen. Each entry has
   `{text, pathname, same_origin, scheme, role}` — note that raw `href`
   and full resolved URL are deliberately omitted so query strings and
   fragments (which may contain auth tokens) never reach you. Use
   `pathname` as the route identifier and filter to `same_origin: true`.
3. Take an accessibility snapshot using `{browser-executor-script} snapshot`
   to record the component structure of the current screen
4. For each newly-discovered same-origin pathname, navigate to it and
   repeat steps 2–3 (depth-first, deduplicated by pathname)
5. Stop when no new pathnames are discovered, or the page cap is reached
```

Update the "Important Guidelines" to make the fabrication-prevention
rule explicit:

```
- **Routes come from `links`** — never invent a URL that did not appear in
  a `links` response with `same_origin: true`. If the SPA renders the
  same shell for every URL, trust the anchor list, not navigation
  success.
- **`pathname` is the route identifier** — the executor returns it
  already resolved against the current URL and stripped of query strings
  and fragments. The raw `href` is intentionally NOT in the response
  (to avoid leaking secrets in URL params).
```

The "What NOT to Do" section keeps its existing prohibitions on
`evaluate`, `click`, and `type` — they remain forbidden for the locator.

#### 5. Do NOT add `links` to `browser-analyser.md`

**File**: `agents/browser-analyser.md`
**Changes**: None for the `links` command. The analyser's role is
screen-focused, not route-focused; adding an unused verb dilutes the
agent's "only use these" constraint. If a concrete need emerges (e.g.
analyser needs to follow a CTA link), open a separate work item.

#### 6. Update `test-design.sh` to assert the new contract

**File**: `scripts/test-design.sh`
**Changes**: Add assertions for the locator's body content. Explicitly
assert that the analyser does NOT have `links` in its Tools section,
making the scope decision testable.

```bash
echo "=== browser-locator links contract ==="
assert_contains "browser-locator body documents the links command" \
  "$(cat "$LOC")" "{browser-executor-script} links"
assert_contains "browser-locator body uses pathname as route identifier" \
  "$(cat "$LOC")" "Use \`pathname\`"
assert_contains "browser-locator body restricts route names to links output" \
  "$(cat "$LOC")" "Routes come from"
assert_contains "browser-locator body requires same_origin filter" \
  "$(cat "$LOC")" "same_origin: true"
assert_not_contains "browser-analyser body does NOT advertise the links command" \
  "$(cat "$ANA")" "{browser-executor-script} links"
```

### Success Criteria:

#### Automated Verification:

- [ ] `bash skills/design/inventory-design/scripts/playwright/test-run.sh`
      passes the new `links` test (all enriched-shape assertions)
- [ ] `node --test skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
      passes (no regressions to existing tests)
- [ ] `bash scripts/test-design.sh` passes the new `links` contract
      assertions (including the negative assertion on browser-analyser)
- [ ] `grep -n "case 'links'" skills/design/inventory-design/scripts/playwright/lib/daemon.js`
      shows the new switch case
- [ ] `grep -E "^const BLOCKING_OPS" skills/design/inventory-design/scripts/playwright/lib/daemon.js`
      shows `'links'` in the set

#### Manual Verification:

- [ ] Run `/accelerator:inventory-design current-app http://localhost:5173`
      against the visualiser. The resulting `inventory.md` lists the real
      routes (e.g. `/library/work-items` — note the `/library/` prefix)
      and does NOT fabricate paths like `/work-items` that don't exist.
- [ ] If the visualiser is running, manually invoke
      `{browser-executor-script} navigate '{"url":"http://localhost:5173/"}'`
      followed by `{browser-executor-script} links` (substituting the
      resolved run.sh path for the placeholder). The output JSON contains
      the SPA's real nav hrefs with `pathname` and `same_origin: true`.

---

## Phase 5: Cross-cutting documentation (PROTOCOL.md and CHANGELOG.md)

### Overview

Capture the protocol-surface changes and the user-visible behaviour
changes in the two canonical docs. Phases 1–4 each leave a discrete
artifact that should appear in PROTOCOL.md (the wire-protocol reference)
or CHANGELOG.md (the user-visible release log) but not in any code change.

### Changes Required:

#### 1. Add `links` section to PROTOCOL.md

**File**: `skills/design/inventory-design/PROTOCOL.md`
**Changes**: Insert this exact block after `### snapshot` (around
line 179, before `### screenshot`):

```markdown
### `links`

Enumerate same-origin and cross-origin anchors on the current page,
with URLs server-resolved and scrubbed (query strings and fragments
removed). Used by `browser-locator` for SPA route discovery without
requiring JavaScript execution at the agent layer.

**Request**

```json
{
  "protocol": 1,
  "command": "links"
}
```

**Success response**

```json
{
  "protocol": 1,
  "url": "https://example.com/page",
  "links": [
    {
      "text": "Work items",
      "pathname": "/work-items",
      "same_origin": true,
      "scheme": "https",
      "role": null
    },
    {
      "text": "External",
      "pathname": "/external",
      "same_origin": false,
      "scheme": "https",
      "role": "button"
    }
  ]
}
```

The `url` envelope field is the current `page.url()` at the moment
the command was dispatched; callers can use it to verify context.

Per-entry fields:

| Field         | Type            | Notes                                                        |
|---------------|-----------------|--------------------------------------------------------------|
| `text`        | string          | Anchor `textContent`, whitespace collapsed with `/\s+/g → ' '` then trimmed |
| `pathname`    | string \| null  | URL pathname only; query strings and fragments stripped. `null` if href is unparseable |
| `same_origin` | boolean         | True iff resolved origin matches the page origin             |
| `scheme`      | string \| null  | URL scheme without the trailing colon (`https`, `mailto`, `file`, etc.). `null` if unparseable |
| `role`        | string \| null  | `getAttribute('role')` verbatim; `null` if the attribute is absent |

**Why no `href` or `resolved` URL?** Raw and fully-resolved URLs are
deliberately omitted so query strings and fragments — which may carry
auth tokens, OAuth codes, session IDs, or signed-URL signatures — never
reach agent context. Callers that need route identity work off
`pathname` + `same_origin`. If a future caller requires the raw URL,
add an explicit opt-in flag rather than relaxing the default.

**Blocking**: yes (member of `BLOCKING_OPS`); the per-op wall-clock
timer arms during execution.

**Error codes**

| Code                  | Category  | Retryable | Condition                            |
|-----------------------|-----------|-----------|--------------------------------------|
| `wall-clock-exceeded` | `browser` | false     | Op exceeded per-op wall-clock budget |
| `internal-error`      | `browser` | false     | Unexpected Playwright exception      |

**Notes**

- Invoking `links` before any `navigate` returns
  `{ url: "about:blank", links: [] }` rather than an error. Callers
  should ensure they have navigated first.
- `mailto:`, `javascript:`, `data:`, and other non-HTTP schemes appear
  in the response with their `scheme` field set; callers are
  responsible for filtering.
- **Same-origin semantics (security-relevant)**: `same_origin` is
  computed as `URL(href).origin === location.origin && origin !== 'null'`.
  The trailing `!== 'null'` guard prevents opaque-origin matches:
  `file:`, `data:`, `javascript:`, and `blob:` URLs all share the
  literal `"null"` origin string per HTML spec, so without the guard a
  `mailto:` or `javascript:void(0)` anchor on a `file://` page would
  report `same_origin: true`. With the guard, every anchor on an
  opaque-origin page reports `same_origin: false`, which correctly
  yields zero same-origin candidates for the locator's route-following
  rule.

---
```

Then insert this exact `## Environment Variables` section before the
"Stability commitment" section (currently around line 498):

```markdown
## Environment Variables

The Playwright daemon reads the following environment variables. All
are optional; defaults apply when unset. Variables are read once at
daemon startup and cannot be changed at runtime without restarting
the daemon.

| Variable                                | Default       | Set by    | Meaning |
|-----------------------------------------|---------------|-----------|---------|
| `ACCELERATOR_PLAYWRIGHT_IDLE_MS`        | `600000`      | caller    | Idle shutdown timeout (ms). Bounds the in-memory lifetime of an auth-bearing browser context; do not raise without considering auth-context exposure. Lowered from `1800000` in this release. |
| `ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS`  | `300000`      | caller    | Per-op wall-clock budget (ms) for any `BLOCKING_OPS` command. Hard-capped at 1800000 (30 min) regardless of override. |
| `ACCELERATOR_PLAYWRIGHT_CACHE`          | `${HOME}/.cache/accelerator/playwright` | environment | Root directory for the Playwright browser cache (versioned by package-lock hash). |
| `ACCELERATOR_PLAYWRIGHT_NS_ROOT`        | derived       | run.sh    | Namespace root for the active Playwright install (cache root + lockhash). Set by `run.sh` when invoking the daemon or client; callers should not set it directly. |
| `ACCELERATOR_PLAYWRIGHT_STATE_DIR`      | derived       | run.sh    | Per-project state directory. Set by `run.sh`; callers should not set it directly. |
| `ACCELERATOR_PLAYWRIGHT_KEEP_STDIO`     | unset         | debug     | When non-empty, retains the daemon's stdout/stderr instead of redirecting to `/dev/null`. Useful for daemon debugging. |

**Removed in this release**: `ACCELERATOR_PLAYWRIGHT_OWNER_POLL_MS` is
no longer read — the owner-PID watcher was removed (see the Breaking
section in CHANGELOG.md `[Unreleased]`).

---
```

#### 2. Add `test-design.sh` assertions that PROTOCOL.md is in sync

**File**: `scripts/test-design.sh`
**Changes**: Assert that every command in `daemon.js`'s dispatch switch
has a matching section in PROTOCOL.md, AND that the Environment Variables
section enumerates every `ACCELERATOR_PLAYWRIGHT_*` identifier that
appears in `daemon.js` (sans `OWNER_POLL_MS`, removed by Phase 2).

```bash
echo "=== PROTOCOL.md is in sync with daemon dispatch ==="
PROTOCOL_MD="$PLUGIN_ROOT/skills/design/inventory-design/PROTOCOL.md"
DAEMON_SRC="$PLUGIN_ROOT/skills/design/inventory-design/scripts/playwright/lib/daemon.js"
assert_file_exists "PROTOCOL.md exists" "$PROTOCOL_MD"

# Every dispatched command must appear as a `### cmd` heading.
for cmd in ping daemon-status daemon-stop navigate snapshot links screenshot evaluate click type wait_for; do
  assert_contains "PROTOCOL.md documents the $cmd command" \
    "$(cat "$PROTOCOL_MD")" "### \`$cmd\`"
done

# Every ACCELERATOR_PLAYWRIGHT_* env var read by daemon.js must appear in
# the Environment Variables section. Catches drift when a new env var is
# added without doc.
assert_contains "PROTOCOL.md has Environment Variables section" \
  "$(cat "$PROTOCOL_MD")" "## Environment Variables"
DAEMON_ENV_VARS=$(grep -oE 'ACCELERATOR_PLAYWRIGHT_[A-Z_]+' "$DAEMON_SRC" | sort -u)
for var in $DAEMON_ENV_VARS; do
  assert_contains "PROTOCOL.md Environment Variables section names $var" \
    "$(cat "$PROTOCOL_MD")" "$var"
done
```

#### 3. Update CHANGELOG.md

**File**: `CHANGELOG.md`
**Changes**: Add entries to the `[Unreleased]` section. Use the existing
heading style (`Added`, `Changed`, `Fixed`, etc.) — match whatever the
existing file uses.

Use the existing heading style. The current `[Unreleased]` block uses
`### Breaking`, `### Added`, `### Changed`, and `### Fixed` as peer
top-level subsections (see CHANGELOG.md lines 5, 59, 81 for the
established convention). Append the new entries under those existing
peer headings — do NOT introduce a new `### Removed` section or a
nested `#### Breaking` subsection. The CHANGELOG already has an
`### Breaking` entry under `[Unreleased]`; add the watcher-removal
entry to that same section.

```markdown
### Breaking

- Playwright daemon owner-PID watcher removed: the `--owner-pid` CLI
  argument on `node run.js daemon`, the
  `ACCELERATOR_PLAYWRIGHT_OWNER_POLL_MS` environment variable, and the
  watcher's polling logic are all removed. The daemon is now bounded
  only by its idle timer, per-op wall-clock budget, explicit
  `daemon-stop`, and process signals. External tooling that invoked
  `node run.js daemon --owner-pid <pid>` directly must drop the flag
  (unknown flags are silently ignored). If you wrap `node run.js
  daemon` directly and relied on `--owner-pid` for cleanup, replace
  with explicit `daemon-stop` or rely on the idle timer.

### Added

- `links` command for the Playwright executor: enumerates anchors on the
  current page with server-resolved `pathname`, `same_origin`, `scheme`,
  and normalised `text`. Raw `href` and fully-resolved URLs are
  deliberately omitted so query strings and fragments (which may carry
  auth tokens) never reach agent context. Used by `browser-locator` to
  discover routes on client-side SPAs. See PROTOCOL.md `### links` for
  the wire schema and rationale.
- `accelerator:browser-executor` preloaded skill that injects the
  resolved absolute path of `run.sh` into browser-agent context, mirroring
  the `accelerator:paths` precedent from work item 0052. Browser agents
  now include an explicit preload guard that fails loudly if the
  injected block is missing.
- Migration-aware warning in `config-read-path.sh`: fires when the
  canonical `research_design_*` key is requested but the user's config
  still has the legacy `paths.design_*` alias (silently being ignored),
  pointing the user at `/accelerator:migrate`.

### Changed

- Playwright daemon idle timeout default lowered from 30 min to 10 min
  (override via `ACCELERATOR_PLAYWRIGHT_IDLE_MS`). Bounds the in-memory
  lifetime of an auth-bearing browser context held across Claude Code
  turns; better fit for the inventory-design lifecycle. Override path
  unchanged: callers needing the previous 30-min behaviour can set
  `ACCELERATOR_PLAYWRIGHT_IDLE_MS=1800000`.
- Internal: five SKILL.md preambles that called the bare `design_*` path
  keys now call the canonical `research_design_*` keys introduced by
  migration 0004.

### Fixed

- `inventory-design` browser agents no longer self-discover the
  Playwright executor with `which run.sh` / `find / -name run.sh`. The
  executor's absolute path is now injected via the
  `accelerator:browser-executor` preloaded skill.
- `browser-locator` no longer fabricates SPA routes by treating
  navigation success as evidence a route exists. Route truth now comes
  from the new `links` command's anchor enumeration filtered by
  `same_origin: true`.
- Skills calling `design_inventories` / `design_gaps` no longer emit
  `config-read-path.sh: warning: unknown key` noise in their rendered
  preambles (call sites renamed to canonical `research_design_*` form).
- Playwright daemon no longer dies between Claude Code Bash-tool
  invocations. The owner-PID watcher (whose first tick fired at most
  ~60 s after the ephemeral launcher shell exited, destroying the
  daemon) has been removed; see Breaking above for the consumer impact.
```

#### 4. Record verified Claude Code version baseline in README

**File**: `README.md`
**Changes**: Add a short paragraph (under the existing Requirements /
installation section, or as a new "Compatibility" subsection if no
Requirements section exists) documenting the Claude Code version
verified to support the subagent `skills:` preload mechanism that
Phase 3 depends on. This is the mechanical companion to the
best-effort Preload guard in the agent bodies: when the guard fires,
the user has a concrete version to compare against.

```markdown
### Claude Code compatibility

This plugin relies on Claude Code's subagent `skills:` preload mechanism
(see `agents/documents-locator.md` and `agents/browser-locator.md`
frontmatter). The **browser-executor verified baseline** is Claude Code
**vX.Y.Z** (record the version at implementation time — the version
at which the preloaded `accelerator:browser-executor` skill was
observed to inject its `## Browser Executor` block into the
browser-locator and browser-analyser agent contexts). Earlier Claude
Code releases may not support the mechanism; later releases that
change subagent skill-preloading semantics will surface the failure
via the agents' Preload guard.
```

The exact version is filled in during implementation by running
`/accelerator:inventory-design` end-to-end on the implementer's local
Claude Code and recording the reported version (or via `claude --version`
if that command exists; otherwise the version shown in the Claude
Code UI). The literal string `browser-executor verified baseline` is
asserted by Phase 5 Success Criteria.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-design.sh` passes the new PROTOCOL.md
      in-sync assertion
- [ ] `grep -F '### links' skills/design/inventory-design/PROTOCOL.md` finds
      the new section
- [ ] `grep -F '## Environment Variables' skills/design/inventory-design/PROTOCOL.md`
      finds the new section
- [ ] `grep -F 'browser-executor' CHANGELOG.md` finds the new entry
- [ ] `grep -F 'owner-PID watcher removed' CHANGELOG.md` finds the
      Breaking-section entry
- [ ] `grep -F 'browser-executor verified baseline' README.md` finds the
      Claude Code version baseline note (see Step 4 below)
- [ ] `grep -F 'vX.Y.Z' README.md` returns no match — guards against
      shipping the unfilled placeholder. The implementer MUST substitute
      a real Claude Code version observed during their end-to-end
      verification run.
- [ ] Both the `### Breaking` (line 5) and `### Fixed` headings exist
      under `[Unreleased]` in CHANGELOG.md (verify by inspection or
      `awk '/^## \[Unreleased\]/{p=1} p && /^## \[/{if(seen) exit; seen=1} p' CHANGELOG.md | grep -E '^### (Breaking|Fixed)'`)

#### Manual Verification:

- [ ] PROTOCOL.md reads as a complete reference: every command listed in
      the Subcommand Reference table has a body subsection with request,
      response, and error-code coverage.
- [ ] CHANGELOG.md entries are grouped under existing headings
      (Breaking / Added / Changed / Fixed) and use sentence-case prose
      consistent with prior entries. The watcher removal sits under the
      existing `### Breaking` section in `[Unreleased]`, not a separate
      `### Removed` section.

---

## Testing Strategy

### Unit Tests:

- `daemon.test.js`: existing tests have `--owner-pid 0` arguments removed
  (Phase 2); a new `IDLE_MS default of 10 minutes` test pins the source-
  level default (Phase 2); a new wall-clock test asserts `links` is
  bounded (Phase 4).
- `test-run.sh`: new launcher-shell-exit survival test (Phase 2); new
  `links` behavioural test against the fixture (Phase 4).
- `test-design.sh`: new repo-wide assertion for canonical key names
  (Phase 1); new assertions that the owner-PID watcher is gone (Phase 2);
  new browser-executor preloaded-skill section (Phase 3); new locator
  `links` contract + negative assertion on analyser (Phase 4); new
  PROTOCOL.md in-sync assertion (Phase 5).
- `test-config.sh`: new migration-aware warning test (Phase 1).

### Integration Tests:

- End-to-end Playwright lifecycle is exercised by Phase 2's
  launcher-shell-exit smoke test (`run.sh ping` in a sub-shell +
  post-exit daemon liveness check + clean `daemon-stop`).
- Cross-phase: the `links` test (Phase 4) implicitly exercises Phase 2
  (daemon survives between `navigate` and `links` calls) and Phase 3
  (the test uses the resolved executor path, not a bare command — though
  in `test-run.sh` the path is already known via `$RUN_SH`).

### Known Coverage Gaps:

- **Locator agent's runtime consumption of `links` is verified only by
  prose-presence assertions** in `test-design.sh` (e.g. the body
  contains the literal phrase "Routes come from", references
  `pathname`, mentions `same_origin: true`). These pin the
  *documentation* of the contract but cannot prove the locator agent
  actually behaves accordingly at inference time. Manual verification
  step 6 (visualiser run produces real routes, no fabrications) is the
  end-to-end check for this contract. This is a deliberate gap given
  the limits of testing markdown agent bodies.
- **Locator's same-origin filtering and pathname use** is similarly only
  pinned in prose. A future agent-body rewrite that retains the words
  but reorders/weakens the guidance could pass all assertions while
  regressing.
- **The `same_origin: true` positive branch of the daemon's opaque-origin
  guard is not exercised by automated tests.** The Phase 4 fixture is
  served from `file://`, whose page origin is opaque (`"null"`), so the
  guard `u.origin === pageOrigin && u.origin !== 'null'` correctly
  forces every anchor on the fixture to report `same_origin: false`.
  Adding an HTTP-origin fixture (via `http.createServer` in
  `test-run.sh`) would exercise the positive branch but introduces a
  new test-suite dependency on a tiny ad-hoc server with port-binding
  semantics, which is disproportionate for this plan's scope. The
  positive branch is exercised end-to-end at every Manual Verification
  step 6 run (the visualiser is HTTP-origin, so its anchors must
  report `same_origin: true` for routes to be discovered). A future
  follow-up could add the HTTP fixture and a targeted assertion if
  this gap proves load-bearing in practice.

### Manual Testing Steps:

1. Start the visualiser locally.
2. Run `/accelerator:inventory-design current-app http://localhost:5173`.
3. Verify the rendered preamble shows resolved paths (no warnings).
4. Verify the spawn prompt for `browser-locator` includes a
   `## Browser Executor` preloaded block (Phase 3).
5. Verify the locator's first action uses the resolved path, not `find /`
   (Phases 3 and 4).
6. Verify the locator's reported routes match the visualiser's actual
   navigation structure (Phase 4 — no fabricated paths; each route
   reported has `same_origin: true` in the `links` output).
7. Verify the daemon stays alive between agent turns (inspect
   `${PATH_TMP}/inventory-design-playwright/server.pid` across turns —
   Phase 2).
8. Verify no `config-read-path.sh: warning: unknown key` lines appear in
   the rendered preamble (Phase 1).
9. In a pre-migration test repo (with `paths.design_inventories` still in
   `.accelerator/config.md`), verify the migration-aware warning fires
   and names `/accelerator:migrate` (Phase 1).

## Performance Considerations

- The new `links` command runs a single `page.evaluate` over
  `document.querySelectorAll('a[href]')`. For pages with thousands of
  anchors this is O(n) but well-bounded; the wall-clock timer caps it at
  5 min by default. No streaming or chunking needed.
- The idle-timer change (30 min → 10 min) means the daemon self-cleans
  3x faster after the last invocation. For typical multi-turn skill
  sessions (seconds to minutes), this is invisible. Power users running
  back-to-back inventories with > 10-minute gaps incur one extra
  cold-start.
- Removing the owner-PID watcher eliminates one `setInterval` per
  daemon and one `process.kill(pid, 0)` syscall per 60s — negligible
  but strictly positive.

## Migration Notes

- **Existing users** with `paths.design_inventories` /
  `paths.design_gaps` overrides in their team config: migration 0004
  rewrites these to `paths.research_design_*`. Users who upgrade the
  plugin before running `/accelerator:migrate` now see a clear
  migration-aware warning (Phase 1, change 3) instead of a generic
  "unknown key" message.
- **Daemon idle timeout default lowered to 10 min.** Users who relied on
  the previous 30-min default for cross-session daemon reuse can override
  via `ACCELERATOR_PLAYWRIGHT_IDLE_MS`. Documented in PROTOCOL.md and
  CHANGELOG.md.
- **`--owner-pid` CLI arg removed.** Any external tooling that invoked
  `node run.js daemon --owner-pid <pid>` directly must drop the flag.
  This is internal to the executor; no documented external use exists.
- **No state-dir or daemon-protocol version bump.** The `links` command
  is additive to the existing protocol (`PROTOCOL = 1`).

## References

- Research document:
  `meta/research/codebase/2026-05-19-inventory-design-and-browser-agent-fixes.md`
- Plan review:
  `meta/reviews/plans/2026-05-19-inventory-design-and-browser-agent-fixes-review-1.md`
- Originating notes:
  - `meta/notes/2026-05-19-browser-agents-self-discover-playwright-executor.md`
  - `meta/notes/2026-05-19-browser-locator-cannot-enumerate-routes.md`
  - `meta/notes/2026-05-19-config-read-path-missing-design-keys.md`
  - `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`
- Precedent work item (preloaded-skill mechanism):
  `meta/work/0052-make-documents-locator-paths-config-driven.md`
- Migration that renamed the design path keys:
  `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:414-415`
- Existing `paths` preloaded skill (shape to mirror):
  `skills/config/paths/SKILL.md:1-22`
- Existing path-resolver script (shape to mirror):
  `scripts/config-read-all-paths.sh:1-34`
- Daemon command-dispatch site:
  `skills/design/inventory-design/scripts/playwright/lib/daemon.js:113-224`
- Daemon owner-watcher (removed in Phase 2):
  `skills/design/inventory-design/scripts/playwright/lib/daemon.js:88-97`
- Launcher script:
  `skills/design/inventory-design/scripts/playwright/run.sh:100-131`
- Wire-protocol reference (updated in Phase 5):
  `skills/design/inventory-design/PROTOCOL.md`
