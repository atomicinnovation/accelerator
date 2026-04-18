---
date: "2026-04-18T12:30:00+01:00"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Meta Visualiser — Phase 1: Skill Scaffolding and No-Op Preprocessor

## Overview

Scaffold the `accelerator:visualise` slash command and matching
`accelerator-visualiser` CLI wrapper so that both entry points are
invokable, emit a clearly non-functional placeholder sentinel, and
are registered in the plugin manifest. No server binary, no frontend,
no file watching — this phase validates the slash-command plumbing,
the CLI wrapper pattern, and the test-harness discipline that all
future phases will build on.

Phase 1 lands as **six sub-phases**. The first extracts the bash
test-harness helpers into a single sourced file and migrates existing
harnesses so no test code is duplicated after this phase; later
sub-phases add the stub, wrapper, SKILL.md (plus the `test-config.sh`
invariant fix the new skill requires), plugin-manifest entry, and a
glob-discovered integration-test runner that does not need manual
enrolment for future suites.

Development is TDD-first: every shell artefact gets a failing test
before any implementation. Tests and implementations land in separate
`jj` commits where practical so `jj log` preserves the red-then-green
transition as an audit trail; where commits are bundled, a mutation
smoke test (temporarily remove the artefact, observe failure) is
called out in the manual-verification checklist.

## Current State Analysis

- **No visualisation skill exists**. `skills/visualisation/` is absent
  entirely; nothing is registered in the plugin manifest under that
  path.
- **Plugin manifest** (`.claude-plugin/plugin.json`) currently lists 8
  skill categories: `vcs`, `github`, `planning`, `research`,
  `decisions`, `review/lenses`, `review/output-formats`, `config`.
- **Canonical SKILL.md pattern** is established:
  `skills/github/review-pr/SKILL.md:1-36` and
  `skills/config/init/SKILL.md:1-31` are the closest references —
  frontmatter with `disable-model-invocation: true`, `!`-prefixed
  bash preamble using `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh`,
  bold-labelled placeholders substituted in the skill body.
- **Skill-local script bootstrap pattern** is established:
  `skills/decisions/scripts/adr-next-number.sh:11-15` derives
  `PLUGIN_ROOT` by walking up from `$SCRIPT_DIR`. No Phase 1 script
  derives `PLUGIN_ROOT` for runtime use (the stub is a one-liner and
  the CLI wrapper only needs `SKILL_ROOT`), but the Phase 1 test
  harnesses do derive `PLUGIN_ROOT` to source shared helpers — four
  levels up from `skills/visualisation/visualise/scripts/`.
- **Bash test harness duplication today**: `scripts/test-config.sh`
  and `skills/decisions/scripts/test-adr-scripts.sh` each inline the
  same `assert_eq` / `assert_exit_code` / PASS-FAIL bookkeeping. Phase
  1.1 extracts these into `scripts/test-helpers.sh` and migrates both
  existing harnesses so only one copy survives.
- **Integration test task** lives at `tasks/test.py:4-11` and today
  hard-codes two `context.run` calls. Phase 1.6 replaces this with a
  glob-based discovery loop (`**/test-*.sh` filtered to executable
  files) so future suites auto-enrol.
- **Config path resolution** is a solved problem:
  `scripts/config-read-path.sh` accepts `<key> [default]` and writes
  the resolved relative path to stdout. Every one of the 11 path keys
  (`plans`, `research`, `decisions`, `prs`, `validations`,
  `review_plans`, `review_prs`, `templates`, `tickets`, `notes`,
  `tmp`) is consumed identically.
- **`test-config.sh` skill-count invariants**: the existing suite
  hard-codes `"13"` in three assertions (lines 1025, 2859, 2863) and
  references an `ALL_SKILLS` array (line 2866) and a `CONTEXT_SKILLS`
  array (line 1032) listing every SKILL.md that invokes the context
  preprocessors. Adding `visualisation/visualise` requires bumping the
  counts and appending the new entry to both arrays; this happens in
  the same sub-phase as the SKILL.md creation so CI stays green
  throughout.

### Key Discoveries

- **`launch-server.sh` stdout shape is deliberately plain text** for
  Phase 1. Phase 2 will enrich it (URL + status + PID hint); the
  SKILL.md `**Visualiser URL (not yet running)**` line is a known
  churn point at that phase boundary — see Migration Notes.
- **Placeholder is non-routable by design**. Phase 1 emits
  `placeholder://phase-1-scaffold-not-yet-running` rather than an
  `http://localhost:N` string, so a user who copies the value into a
  browser hits an immediate "invalid URL" signal instead of a
  `ERR_CONNECTION_REFUSED` that suggests a real server failed.
- **Per D1 of the research**, the CLI wrapper is a thin shell script
  committed in the plugin tree at
  `skills/visualisation/visualise/cli/accelerator-visualiser`; users
  symlink it onto `$PATH` themselves. It execs the same
  `launch-server.sh` the slash command uses. The wrapper's symlink
  walk uses only POSIX tools (`readlink` without `-f`, `cd`, `pwd -P`,
  `dirname`, `basename`) — the same pattern as
  `scripts/config-*.sh` — removing any dependency on GNU `coreutils`
  or `perl` and guaranteeing identical behaviour on macOS and Linux.
- **Per D4 of the research**, all work stays inside the
  `visualisation-system` jj workspace; the workspace root is the sole
  repository root. The plugin manifest lives at
  `./.claude-plugin/plugin.json` inside this workspace.
- **The 11-path preamble in the SKILL.md is intentional forward-compat
  scaffolding** (see recommendation 5 of the Phase 1 plan review:
  explicitly retained). Phase 1 does not consume the resolved paths;
  Phase 2 will wire them into the server's `config.json`. The in-body
  HTML comment in the SKILL.md documents this for maintainers so the
  choice is not mistaken for noise.

## Desired End State

After this phase ships:

1. A user running `/accelerator:visualise` in Claude Code sees the
   preamble-resolved paths, a single `**Visualiser URL (not yet
   running)**: placeholder://phase-1-scaffold-not-yet-running` line,
   a user-facing Availability block (phase jargon confined to a
   Claude-only HTML comment), and a pre-resolved `**Install
   command**` line they can copy verbatim.
2. A user running
   `${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser`
   from a terminal gets the placeholder sentinel on stdout.
3. `mise run test:integration` auto-discovers every `test-*.sh` in
   the tree, runs four suites (`test-config.sh`, `test-adr-scripts.sh`,
   `test-launch-server.sh`, `test-cli-wrapper.sh`), and all pass.
   Adding a new `test-*.sh` in a later phase requires no edit to
   `tasks/test.py`.
4. The plugin manifest lists `./skills/visualisation/` alongside the
   other 8 skill category paths.
5. `scripts/test-helpers.sh` is the sole copy of the
   `assert_eq` / `assert_exit_code` / `assert_file_executable` /
   `assert_stderr_empty` / `test_summary` helpers; the two existing
   bash harnesses source it.
6. Both the SKILL.md's frontmatter and the plugin manifest are valid
   according to their respective parsers (`yq` / `jq`).

### Verification

- `mise run test:integration` exits 0.
- `jq -e . .claude-plugin/plugin.json` exits 0.
- `jq -r '.skills[]' .claude-plugin/plugin.json | grep -Fx './skills/visualisation/'` matches.
- `yq '.name' skills/visualisation/visualise/SKILL.md` prints
  `visualise`.
- `yq '."disable-model-invocation"' skills/visualisation/visualise/SKILL.md`
  prints `true`.
- Neither `scripts/test-config.sh` nor
  `skills/decisions/scripts/test-adr-scripts.sh` contains a local
  `assert_eq()` definition — both source `scripts/test-helpers.sh`.
- Manually invoking `/accelerator:visualise` and the CLI wrapper both
  emit the placeholder sentinel line.

## What We're NOT Doing

Explicitly out of scope for Phase 1, each deferred to the phase
indicated.

- **No Rust server** — Phase 2. No `server/` directory, no
  `Cargo.toml`, no `main.rs`.
- **No frontend scaffold** — Phase 5. No `frontend/` directory, no
  `package.json`, no Vite config.
- **No `bin/` or `checksums.json`** — Phase 2 (binary acquisition)
  and Phase 12 (release).
- **No `config.json` schema definition** — Phase 2. The stub does
  not produce one and the SKILL.md does not yet resolve paths into
  one.
- **No `server-info.json` / `server.pid` / `server.log` /
  `server-stopped.json`** — Phase 2.
- **No `stop-server.sh`** — Phase 2.
- **No PID-file management, owner-PID watch, or idle-timeout
  logic** — Phase 2.
- **No network behaviour** — the stub prints a sentinel string;
  nothing listens on any port.
- **No Rust edition / MSRV pinning** — Phase 2.
- **No first-run download UX** — Phase 2.
- **No release flow / `release-visualiser-binaries.sh`** — Phase 12.
- **No pre-release binary policy decision** — Phase 12.
- **No CHANGELOG entry and no plugin version bump** — version bumps
  happen at release time; Phase 1 ships as scaffolding only.
- **No `ACCELERATOR_VISUALISER_BIN` dev override** — Phase 2 (there
  is no binary for it to override yet).
- **No user-facing README update** — Phase 12.
- **No generalisation of `scripts/test-config.sh` skill-count
  assertions** to derive counts from the manifest — the Phase 1
  change bumps literals and extends the arrays. Dynamic derivation
  is noted as a potential follow-up but out of scope here.
- **No Windows support** — consistent with the plugin's existing
  macOS/Linux scope.

## Implementation Approach

Six sub-phases, ordered so each depends only on earlier ones:

1. **Shared test helpers** (TDD infrastructure). Extract
   `assert_*` / PASS-FAIL bookkeeping into `scripts/test-helpers.sh`
   and migrate `test-config.sh` + `test-adr-scripts.sh` to source it.
   Unlocks the new harnesses without duplicating boilerplate.
2. **`launch-server.sh` stub and tests** (TDD). Placeholder-sentinel
   stub with a test harness that sources the shared helpers.
3. **CLI wrapper and tests** (TDD). Pure-bash symlink-walk wrapper
   that execs the stub; tests source the shared helpers and include
   a sentinel-based delegation proof plus argument-forwarding test.
4. **SKILL.md + `test-config.sh` invariant fix**. Create the
   slash-command entry point with the canonical preamble (and the
   explicit forward-compat annotation on the 11-path block) and
   update the hard-coded skill-count assertions and arrays in
   `test-config.sh` so CI stays green in the same commit.
5. **Plugin manifest registration**. Register
   `./skills/visualisation/` in `.claude-plugin/plugin.json`.
6. **Glob-discovered test suites**. Replace the hand-curated list in
   `tasks/test.py` with a discovery loop that auto-enrols every
   executable `test-*.sh`.

TDD discipline per sub-phase:

- **Red**: add a test that fails because the artefact doesn't exist
  or is incorrect. Where practical, commit the test in a distinct
  `jj` commit before the implementation commit so the red-then-green
  transition is preserved in `jj log`.
- **Green**: implement the minimum to make tests pass.
- **Refactor**: only clean up if the implementation is awkward.

If tests and implementation land in the same commit (common for
small scaffolding changes), the sub-phase's Manual Verification
includes a mutation smoke test: temporarily remove the artefact
and confirm the suite fails, proving the tests actually depend on
what they claim to.

---

## Phase 1.1: Shared test harness helpers

### Overview

Extract the bash test-harness helpers into a single sourced file and
migrate the two existing harnesses to use it. After this phase there
is exactly one copy of `assert_eq` / `assert_exit_code` / PASS-FAIL
bookkeeping in the tree.

### Changes Required

#### 1. Shared helpers file

**File**: `scripts/test-helpers.sh`
**Changes**: New file, **not executable** (sourced only — leaving the
exec bit off is what lets Phase 1.6's glob discovery exclude it from
the test runner without a hand-curated exception).

```bash
#!/usr/bin/env bash
# Shared bash test-harness helpers. Source from test-*.sh scripts:
#
#   source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/..(etc)../scripts/test-helpers.sh"
#
# Exposes: PASS/FAIL counters and assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary.

PASS=0
FAIL=0

assert_eq() {
  local test_name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected: $(printf '%q' "$expected")"
    echo "    Actual:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit_code() {
  local test_name="$1" expected_code="$2"
  shift 2
  local stderr_file
  stderr_file=$(mktemp)
  local actual_code=0
  "$@" >/dev/null 2>"$stderr_file" || actual_code=$?
  if [ "$expected_code" -eq "$actual_code" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected exit code: $expected_code"
    echo "    Actual exit code:   $actual_code"
    if [ -s "$stderr_file" ]; then
      echo "    stderr:"
      sed 's/^/      /' "$stderr_file"
    fi
    FAIL=$((FAIL + 1))
  fi
  rm -f "$stderr_file"
}

assert_file_executable() {
  local test_name="$1" path="$2"
  if [ -x "$path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name ($path not executable)"
    FAIL=$((FAIL + 1))
  fi
}

assert_stderr_empty() {
  local test_name="$1"
  shift
  local stderr
  stderr=$("$@" 2>&1 >/dev/null)
  if [ -z "$stderr" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Unexpected stderr: $stderr"
    FAIL=$((FAIL + 1))
  fi
}

test_summary() {
  echo ""
  echo "=== Results ==="
  echo "Passed: $PASS"
  echo "Failed: $FAIL"
  if [ "$FAIL" -gt 0 ]; then
    return 1
  fi
  echo "All tests passed!"
}
```

#### 2. Migrate existing harnesses

**Files**:
- `scripts/test-config.sh`
- `skills/decisions/scripts/test-adr-scripts.sh`

**Changes**:
- Remove the inlined `PASS=0`, `FAIL=0`, `assert_eq`, `assert_exit_code`
  definitions from both files.
- Near the top of each (before any test), add the concrete bootstrap
  for that file's nesting depth:

  For `scripts/test-config.sh` (one level up from `scripts/`):
  ```bash
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
  source "$PLUGIN_ROOT/scripts/test-helpers.sh"
  ```

  For `skills/decisions/scripts/test-adr-scripts.sh` (three levels up):
  ```bash
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
  source "$PLUGIN_ROOT/scripts/test-helpers.sh"
  ```
- Replace the final `=== Results ===` / echo / exit trailer with a
  single call to `test_summary`.

### Success Criteria

#### Automated Verification

- [x] `bash -c 'source scripts/test-helpers.sh; declare -F | grep -E "assert_eq|assert_exit_code|assert_file_executable|assert_stderr_empty|test_summary"'` lists all five helpers.
- [x] `bash scripts/test-config.sh` exits 0 after migration.
- [x] `bash skills/decisions/scripts/test-adr-scripts.sh` exits 0 after migration.
- [x] `grep -R '^assert_eq()' scripts/ skills/` returns exactly one
      file: `scripts/test-helpers.sh`.

#### Manual Verification

- [x] `scripts/test-helpers.sh` is **not** marked executable
      (`[ ! -x scripts/test-helpers.sh ]`). This is intentional — the
      file is sourced, not invoked, and the non-executable bit is
      what keeps Phase 1.6's glob runner from trying to run it as a
      test.

---

## Phase 1.2: `launch-server.sh` stub and tests (TDD)

### Overview

Create the no-op stub that prints the placeholder sentinel, and a
matching test harness that sources the shared helpers.

### Changes Required

#### 1. Test harness

**File**: `skills/visualisation/visualise/scripts/test-launch-server.sh`
**Changes**: New file, executable, sources `scripts/test-helpers.sh`.

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

LAUNCH_SERVER="$SCRIPT_DIR/launch-server.sh"
EXPECTED_SENTINEL="placeholder://phase-1-scaffold-not-yet-running"

echo "=== launch-server.sh (Phase 1 stub) ==="
echo ""

echo "Test: script is executable"
assert_file_executable "executable bit set" "$LAUNCH_SERVER"

echo "Test: exits 0"
assert_exit_code "exits 0" 0 bash "$LAUNCH_SERVER"

echo "Test: prints the placeholder sentinel"
OUTPUT=$(bash "$LAUNCH_SERVER")
assert_eq "stdout matches sentinel" "$EXPECTED_SENTINEL" "$OUTPUT"

echo "Test: output is exactly one line"
LINE_COUNT=$(bash "$LAUNCH_SERVER" | wc -l | tr -d ' ')
assert_eq "one line of output" "1" "$LINE_COUNT"

echo "Test: stderr is empty on happy path"
assert_stderr_empty "no stderr output" bash "$LAUNCH_SERVER"

echo "Test: ignores extra arguments (forward-compatible for Phase 2 flags)"
assert_exit_code "exits 0 with --foo bar" 0 bash "$LAUNCH_SERVER" --foo bar

test_summary
```

`chmod +x` the file.

#### 2. Stub implementation

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`
**Changes**: New file, executable.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Phase 1 stub for the meta visualiser launcher.
# Prints a deliberately-invalid placeholder sentinel so the
# /accelerator:visualise skill has something to show. The scheme is
# intentionally not http:// so a user who pastes this into a browser
# sees an immediate "invalid URL" signal rather than a
# connection-refused error that suggests a real server failed to
# start. The real Rust server bootstrap lands in Phase 2 and replaces
# this file wholesale.

echo "placeholder://phase-1-scaffold-not-yet-running"
```

`chmod +x` the file.

### Success Criteria

#### Automated Verification

- [x] `[ -x skills/visualisation/visualise/scripts/test-launch-server.sh ]`
- [x] `[ -x skills/visualisation/visualise/scripts/launch-server.sh ]`
- [x] `bash skills/visualisation/visualise/scripts/test-launch-server.sh` exits 0.
- [x] `bash skills/visualisation/visualise/scripts/launch-server.sh` → `placeholder://phase-1-scaffold-not-yet-running`.

#### Manual Verification

- [ ] Tests were written first. Either (a) the test-harness change was
      committed in a distinct `jj` commit before the stub (so
      `jj log -r @-` on the earlier change shows the harness alone
      and `bash …/test-launch-server.sh` on that revision fails), or
      (b) after committing together, run a mutation smoke test:
      `mv launch-server.sh launch-server.sh.bak`,
      `bash test-launch-server.sh` must fail,
      `mv launch-server.sh.bak launch-server.sh` to restore.

---

## Phase 1.3: CLI wrapper and tests (TDD)

### Overview

Add the CLI wrapper that execs `launch-server.sh`. Wrapper uses a
pure-bash POSIX symlink walk (no `readlink -f`, no capability probe,
no `perl` fallback). Tests source the shared helpers and prove the
wrapper actually delegates (sentinel test) and forwards arguments
(Phase 2 forward-compat).

### Changes Required

#### 1. CLI wrapper test harness

**File**: `skills/visualisation/visualise/scripts/test-cli-wrapper.sh`
**Changes**: New file, executable, sources `scripts/test-helpers.sh`.

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

REAL_CLI="$SCRIPT_DIR/../cli/accelerator-visualiser"
REAL_STUB="$SCRIPT_DIR/launch-server.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Make a relocated copy of the cli/scripts sibling pair. Mutation tests
# operate on the copy only — the real files are never touched, so a
# mid-test failure can never corrupt the working tree. This also
# exercises the wrapper's plugin-root resolution from an arbitrary
# directory, proving the cli/↔scripts/ sibling contract beyond the
# in-tree happy path.
TEMP_SKILL="$TMPDIR_BASE/skill-copy"
mkdir -p "$TEMP_SKILL/cli" "$TEMP_SKILL/scripts"
cp "$REAL_CLI" "$TEMP_SKILL/cli/accelerator-visualiser"
cp "$REAL_STUB" "$TEMP_SKILL/scripts/launch-server.sh"
chmod +x "$TEMP_SKILL/cli/accelerator-visualiser" "$TEMP_SKILL/scripts/launch-server.sh"
TEMP_CLI="$TEMP_SKILL/cli/accelerator-visualiser"
TEMP_STUB="$TEMP_SKILL/scripts/launch-server.sh"

echo "=== accelerator-visualiser CLI wrapper (Phase 1) ==="
echo ""

echo "Test: wrapper is executable"
assert_file_executable "executable bit set" "$REAL_CLI"

echo "Test: wrapper exits 0"
assert_exit_code "exits 0" 0 bash "$REAL_CLI"

echo "Test: wrapper output matches stub output"
REAL_OUTPUT=$(bash "$REAL_CLI")
STUB_OUTPUT=$(bash "$REAL_STUB")
assert_eq "wrapper output equals stub output" "$STUB_OUTPUT" "$REAL_OUTPUT"

echo "Test: wrapper works from a relocated tree"
RELOCATED_OUTPUT=$(bash "$TEMP_CLI")
assert_eq "relocated wrapper output equals stub output" "$STUB_OUTPUT" "$RELOCATED_OUTPUT"

echo "Test: wrapper actually delegates (proven by sentinel)"
# Replace the TEMP_STUB with a unique-UUID echo. If the wrapper ever
# inlined its own echo instead of exec'ing the stub, the sentinel
# would not appear. Mutation is on the copy only.
SENTINEL="delegation-sentinel-$$-$RANDOM"
cat > "$TEMP_STUB" <<EOF
#!/usr/bin/env bash
echo "$SENTINEL"
EOF
chmod +x "$TEMP_STUB"
DELEGATION_OUTPUT=$(bash "$TEMP_CLI")
assert_eq "wrapper delegated to stub" "$SENTINEL" "$DELEGATION_OUTPUT"

echo "Test: wrapper forwards arguments verbatim (incl. spaces and empty)"
# Replace TEMP_STUB with a script that echoes each argv on its own line
# so quoting regressions (exec "..." $@ vs exec "..." "$@") surface.
cat > "$TEMP_STUB" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
chmod +x "$TEMP_STUB"
ARG_OUTPUT=$(bash "$TEMP_CLI" --foo bar "hello world" "" "a b c")
EXPECTED_ARGS=$(printf -- "--foo\nbar\nhello world\n\na b c")
assert_eq "wrapper preserves \$@ quoting" "$EXPECTED_ARGS" "$ARG_OUTPUT"

echo "Test: wrapper works via symlink (skip if filesystem forbids symlinks)"
LINK="$TMPDIR_BASE/accelerator-visualiser-link"
if ln -s "$REAL_CLI" "$LINK" 2>/dev/null; then
  LINK_OUTPUT=$("$LINK")
  assert_eq "symlink output equals stub output" "$STUB_OUTPUT" "$LINK_OUTPUT"
  assert_exit_code "symlink exits 0" 0 "$LINK"
else
  echo "  SKIP: filesystem does not permit symlinks"
fi

echo "Test: wrapper source contains symlink-cycle guard"
# The 40-hop counter is defence-in-depth against a pathological
# symlink install. It cannot be exercised behaviourally: any cyclic
# symlink chain fails at the kernel's execve() with ELOOP before
# bash starts, so the wrapper's walk never runs via invocation.
# Instead we assert the guard is present in the source so an
# accidental removal surfaces as a test failure.
if grep -q 'HOPS.*-gt.*40' "$REAL_CLI" \
  && grep -q 'symlink loop detected' "$REAL_CLI"; then
  echo "  PASS: hop counter and loop-detected diagnostic present"
  PASS=$((PASS + 1))
else
  echo "  FAIL: hop counter or loop diagnostic missing"
  FAIL=$((FAIL + 1))
fi

echo "Test: stderr is empty on happy path"
assert_stderr_empty "no stderr output" bash "$REAL_CLI"

test_summary
```

`chmod +x` the file.

#### 2. CLI wrapper implementation

**File**: `skills/visualisation/visualise/cli/accelerator-visualiser`
**Changes**: New file, executable. No `.sh` extension by design — the
file is meant to be symlinked onto `$PATH` as `accelerator-visualiser`
and typed directly.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Standalone CLI entry point for the meta visualiser. No .sh extension
# because users symlink this onto $PATH and type the command directly.
# Delegates to the same launch-server.sh that the
# /accelerator:visualise slash command uses, so both entry points have
# identical behaviour.
#
# Symlink resolution uses only POSIX tools — walks the symlink chain
# with readlink (no -f), then canonicalises via `cd ... && pwd -P`.
# This mirrors the pattern in scripts/config-*.sh and works identically
# on macOS (BSD readlink) and Linux (GNU readlink), with no dependency
# on coreutils or perl. The hop counter bounds the walk at 40 (matching
# SYMLOOP_MAX on both BSD and GNU) so a cyclic symlink install fails
# with a clear diagnostic instead of hanging.
SELF="${BASH_SOURCE[0]}"
HOPS=0
while [ -L "$SELF" ]; do
  HOPS=$((HOPS + 1))
  if [ "$HOPS" -gt 40 ]; then
    echo "accelerator-visualiser: symlink loop detected (exceeded 40 hops)" >&2
    exit 1
  fi
  LINK_TARGET=$(readlink "$SELF")
  case "$LINK_TARGET" in
    /*) SELF="$LINK_TARGET" ;;
    *)  SELF="$(cd "$(dirname "$SELF")" && pwd -P)/$LINK_TARGET" ;;
  esac
done
SCRIPT_DIR="$(cd "$(dirname "$SELF")" && pwd -P)"
SKILL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"

exec "$SKILL_ROOT/scripts/launch-server.sh" "$@"
```

`chmod +x` the file.

### Success Criteria

#### Automated Verification

- [x] `[ -x skills/visualisation/visualise/scripts/test-cli-wrapper.sh ]`
- [x] `[ -x skills/visualisation/visualise/cli/accelerator-visualiser ]`
- [x] `bash skills/visualisation/visualise/scripts/test-cli-wrapper.sh` exits 0.
- [x] Parity, tree-relocation, delegation, argument-forwarding
      (including spaces and empty strings), symlink resolution,
      structural-guard-present, and stderr-silence assertions all
      pass (covered by the harness).
- [x] After any test run — including a deliberately failing one — the
      real `launch-server.sh` is byte-identical to its pre-run state
      (mutation tests operate on the tempdir copy only).

#### Manual Verification

- [ ] Tests landed before implementation via separate `jj` commits,
      **or** after bundled commit a mutation smoke test ran
      (`mv accelerator-visualiser accelerator-visualiser.bak`;
      `bash test-cli-wrapper.sh` must fail; restore).
- [ ] Symlink the CLI onto `$PATH`
      (`ln -s "$(pwd)/skills/visualisation/visualise/cli/accelerator-visualiser" ~/.local/bin/accelerator-visualiser`),
      then from a fresh shell `accelerator-visualiser` prints the
      placeholder sentinel. Remove the symlink afterwards.

---

## Phase 1.4: `SKILL.md` + `test-config.sh` invariant fix

### Overview

Create the slash-command entry point and update
`scripts/test-config.sh` in the same sub-phase so CI never sees a
broken state. The SKILL.md resolves all 11 path keys as **deliberate
forward-compat scaffolding** for Phase 2 (intentional per recommendation
5 of the plan review — annotated here and in the SKILL.md body).

### Changes Required

#### 1. SKILL.md

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: New file.

```markdown
---
name: visualise
description: Open the accelerator meta visualiser. Currently prints a placeholder sentinel; the companion-window server is not yet available.
argument-hint: "(no arguments — server not yet available)"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)
---

# Visualise Meta Directory

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh visualise`

<!--
The 11 path-key resolutions below are intentional forward-compat
scaffolding for Phase 2, where the launcher will take these paths as
inputs to the server's config.json. They are not consumed in Phase 1;
the only functional payload in Phase 1 is the
`Visualiser URL (not yet running)` line further down. Resolving them
now locks in the preamble shape so Phase 2 can extend it without
re-editing this block.
-->

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs meta/prs`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations meta/validations`
**Review plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans meta/reviews/plans`
**Review PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
**Templates directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates meta/templates`
**Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes meta/notes`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`

**Visualiser URL (not yet running)**: !`bash ${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/launch-server.sh`

## Availability

<!--
Context for Claude only — do not relay to the user:
This is the Phase 1 scaffold. The Rust server that will eventually
back the URL line is added in Phase 2. No server is running and no
port is listening.
-->

Tell the user, without referring to phases, sub-phases, or release
numbers: the visualiser UI isn't ready yet — this is a scaffold
release. There's no server to connect to; the `placeholder://` line
above will be replaced by a real URL in a future release. Do not
attempt to open the placeholder in a browser.

To use the same entry point from a terminal (also a placeholder
today), symlink the wrapper onto `$PATH`. Copy the full command
below — the path is pre-resolved for you:

**Install command**: !`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`

If `accelerator-visualiser` is not found after running that command,
make sure `$HOME/.local/bin` is on your `$PATH` (on macOS you may
need to add `export PATH="$HOME/.local/bin:$PATH"` to your shell rc).

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh visualise`
```

Notes on the preamble:

- All 11 path keys are resolved in the same order and style as
  `skills/config/init/SKILL.md:16-30`. This is **deliberate
  forward-compat scaffolding** (intentional per recommendation 5 of
  the plan review): Phase 2 will pass these paths to the binary as
  inputs to `config.json`; keeping the preamble shape stable now lets
  Phase 2 extend it without touching this block. The in-body HTML
  comment documents the choice for future maintainers.
- `config-read-context.sh`, `config-read-skill-context.sh visualise`,
  and `config-read-skill-instructions.sh visualise` are included so
  users can override instructions for this skill through the standard
  userspace config mechanism.
- `config-read-agents.sh` is **not** called — this skill does not
  spawn sub-agents.
- `allowed-tools` permits `scripts/config-*` (for the shared config
  scripts) and `skills/visualisation/visualise/scripts/*` (for
  `launch-server.sh`).

#### 2. Update `scripts/test-config.sh` invariants

**File**: `scripts/test-config.sh`
**Changes**: The existing suite hard-codes `"13"` as the expected
count of skills that invoke each of three context-injection
preprocessors, and maintains two arrays enumerating those skills.
Adding `visualisation/visualise` as a 14th skill that invokes all
three preprocessors requires updating each of:

- **Line ~1025**: `assert_eq "13 skills have context injection" "13" "$CONTEXT_COUNT"`
  → `assert_eq "14 skills have context injection" "14" "$CONTEXT_COUNT"`.
- **Line ~2859**: `assert_eq "13 skills have skill-context injection" "13" "$SKILL_CONTEXT_COUNT"`
  → `assert_eq "14 skills have skill-context injection" "14" "$SKILL_CONTEXT_COUNT"`.
- **Line ~2863**: `assert_eq "13 skills have skill-instructions injection" "13" "$SKILL_INSTRUCTIONS_COUNT"`
  → `assert_eq "14 skills have skill-instructions injection" "14" "$SKILL_INSTRUCTIONS_COUNT"`.
- **Line ~1023 and ~2857 and ~2861** (the `echo "Test: … appears in exactly 13 skills"`
  banners): update the printed count from 13 → 14 for consistency.
- **`CONTEXT_SKILLS` array (around line 1032)**: append
  `"visualisation/visualise"`.
- **`ALL_SKILLS` array (around line 2866)**: append
  `"visualisation/visualise"`.

Both the count bump and the array additions must land in the same
commit as the SKILL.md creation so `mise run test:integration` never
observes a failing state between sub-phases.

### Success Criteria

#### Automated Verification

- [x] `[ -f skills/visualisation/visualise/SKILL.md ]`.
- [x] `yq --front-matter=extract '.' skills/visualisation/visualise/SKILL.md >/dev/null` (plain `yq` fails on all SKILL.md files; `--front-matter=extract` is the correct flag).
- [x] `[ "$(yq --front-matter=extract '.name' skills/visualisation/visualise/SKILL.md)" = "visualise" ]`.
- [x] `[ "$(yq --front-matter=extract '."disable-model-invocation"' skills/visualisation/visualise/SKILL.md)" = "true" ]`.
- [x] `allowed-tools` frontmatter contains both the shared config glob
      and the skill-local script glob (grep against the frontmatter).
- [x] `bash scripts/test-config.sh` exits 0 after the invariant updates.

#### Manual Verification

- [ ] Invoking `/accelerator:visualise` in a Claude Code session shows:
      - The 11 resolved path values under their bold labels.
      - The `**Visualiser URL (not yet running)**:` line showing the
        `placeholder://` sentinel (invoked via `bash` so a missing
        exec bit on the stub does not surface as `Permission denied`).
      - The user-facing Availability block, free of phase jargon.
      - The pre-resolved **Install command** line with the absolute
        path already substituted for `${CLAUDE_PLUGIN_ROOT}`.
- [ ] Claude's post-expansion response paraphrases the Availability
      block for the user without mentioning phases, sub-phases, or
      release numbers, and without attempting to open the placeholder
      in a browser.

---

## Phase 1.5: Plugin manifest registration

### Overview

Add `./skills/visualisation/` to the `skills` array in
`.claude-plugin/plugin.json` so Claude Code discovers the new skill.

### Changes Required

#### 1. Plugin manifest

**File**: `.claude-plugin/plugin.json`
**Changes**: Insert `"./skills/visualisation/"` into the `skills`
array. The existing order is workflow-grouped (not alphabetical);
place the new entry immediately before `./skills/config/` so
tooling-adjacent skills sit together. All other fields unchanged;
version is **not** bumped.

```json
{
  "name": "accelerator",
  "version": "1.19.0-pre.2",
  "description": "A Claude Code plugin for structured, context-efficient software development.",
  "author": {
    "name": "Toby Clemson",
    "email": "toby@go-atomic.io"
  },
  "license": "MIT",
  "skills": [
    "./skills/vcs/",
    "./skills/github/",
    "./skills/planning/",
    "./skills/research/",
    "./skills/decisions/",
    "./skills/review/lenses/",
    "./skills/review/output-formats/",
    "./skills/visualisation/",
    "./skills/config/"
  ]
}
```

### Success Criteria

#### Automated Verification

- [ ] `jq -e . .claude-plugin/plugin.json`.
- [ ] `jq -e '.skills | index("./skills/visualisation/")' .claude-plugin/plugin.json`.
- [ ] `jq -e '.skills | contains(["./skills/vcs/","./skills/github/","./skills/planning/","./skills/research/","./skills/decisions/","./skills/review/lenses/","./skills/review/output-formats/","./skills/config/"])' .claude-plugin/plugin.json`.
- [ ] `jq -r '.version' .claude-plugin/plugin.json` unchanged.

#### Manual Verification

- [ ] In a fresh Claude Code session, `/accelerator:visualise`
      appears in the slash-command palette and is invokable.

---

## Phase 1.6: Glob-discovered test suites in `tasks/test.py`

### Overview

Replace the hand-curated list of `context.run` calls with a
glob-discovery loop that walks the repository for every executable
`test-*.sh`. Future phases that add test suites auto-enrol without
editing this task.

### Changes Required

#### 1. Integration test task

**File**: `tasks/test.py`
**Changes**: Full replacement of the `integration` task.

```python
import os
from pathlib import Path

from invoke import Context, task

# Helper files that happen to match the `test-*.sh` pattern but are
# sourced, not run. Belt-and-braces alongside the executable-bit
# filter: the name check catches cases where exec bits are synthesised
# uniformly (e.g. WSL-mounted NTFS) and the exec-bit check catches
# contributors who forget to `chmod +x` a new suite.
EXCLUDED_HELPER_NAMES = {"test-helpers.sh"}


@task
def integration(context: Context):
    """Run integration tests — auto-discovers every executable test-*.sh."""
    repo = Path(__file__).resolve().parent.parent
    suites = sorted(
        p.relative_to(repo).as_posix()
        for p in repo.glob("**/test-*.sh")
        if p.is_file()
        and p.name not in EXCLUDED_HELPER_NAMES
        and os.access(p, os.X_OK)
    )
    if not suites:
        raise RuntimeError("No executable test-*.sh suites discovered")
    for suite in suites:
        print(f"Running {suite}...")
        context.run(suite)
        print()
```

Two independent filters exclude `scripts/test-helpers.sh` from
discovery: (1) the file is intentionally non-executable so
`os.access(..., os.X_OK)` rejects it on POSIX filesystems, and (2) a
name-level exclusion backs up the exec-bit check on filesystems where
exec bits are synthesised uniformly (WSL-mounted NTFS, some container
bind-mounts). `invoke.Context.run` raises non-zero on failing
sub-commands, so the runner fails fast on the first broken suite.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration` exits 0 and runs four suites:
      `scripts/test-config.sh`,
      `skills/decisions/scripts/test-adr-scripts.sh`,
      `skills/visualisation/visualise/scripts/test-cli-wrapper.sh`,
      `skills/visualisation/visualise/scripts/test-launch-server.sh`.
- [ ] `scripts/test-helpers.sh` is **not** in the discovered set
      (its non-executable bit keeps it out).
- [ ] Spot-check: `touch scripts/test-dummy.sh && chmod +x scripts/test-dummy.sh && printf '#!/usr/bin/env bash\nexit 0\n' > scripts/test-dummy.sh && mise run test:integration`
      picks up the new suite automatically; remove the dummy
      afterwards.

#### Manual Verification

- [ ] Running `mise run test:integration` prints one `Running …`
      banner per discovered suite, each followed by the suite's own
      `All tests passed!` summary.

---

## Testing Strategy

### Unit Tests

Phase 1 introduces two bash test suites colocated with the code they
test. Both source `scripts/test-helpers.sh` so there is no duplicated
harness code after this phase:

- **`test-launch-server.sh`** — exit code, single-line stdout, exact
  sentinel match, stderr-empty check, extra-argument tolerance,
  executable bit set.
- **`test-cli-wrapper.sh`** — executable bit, exit code, output
  parity with stub, tree-relocation (the wrapper resolves its
  plugin root from an arbitrary directory), sentinel-based
  delegation proof, argument forwarding with quoting edge cases
  (spaces and empty strings), symlink resolution (capability-gated),
  structural guard for the 40-hop counter (behavioural test is
  impossible because the kernel's ELOOP fires before bash starts
  on any cyclic chain), stderr-empty. All mutation tests operate
  on a tempdir copy of the skill tree so a failed assertion can
  never corrupt the working-copy files.

Existing suites (`scripts/test-config.sh`,
`skills/decisions/scripts/test-adr-scripts.sh`) are migrated to
source the same shared helpers in Phase 1.1, so the refactor lands
in one place rather than deferring the DRY fix.

### Integration Tests

Phase 1.6 converts the `integration` invoke task to a glob-discovery
loop, so every executable `test-*.sh` in the tree runs automatically.
No future suite needs a manual registration step.

### Manual Testing Steps

1. Run `mise run test:integration` from the workspace root; confirm
   all four discovered suites pass.
2. Run
   `bash skills/visualisation/visualise/scripts/launch-server.sh`
   directly; confirm it prints
   `placeholder://phase-1-scaffold-not-yet-running` and nothing else.
3. Run
   `bash skills/visualisation/visualise/cli/accelerator-visualiser`
   directly; confirm identical output.
4. Symlink the CLI into `$PATH`
   (`ln -s "$(pwd)/skills/visualisation/visualise/cli/accelerator-visualiser" ~/.local/bin/accelerator-visualiser`),
   open a fresh shell, run `accelerator-visualiser`. Output should
   still match. Remove the symlink afterwards.
5. Open a Claude Code session rooted at the workspace; type
   `/accelerator:visualise`. Confirm the slash command appears in
   the palette, invokes cleanly, and its expanded skill prompt
   contains:
   - All 11 bold-label resolved paths (with the forward-compat HTML
     comment above them).
   - The `**Visualiser URL (not yet running)**: placeholder://…`
     line (invoked via `bash` in the preamble so it works even if
     the stub lost its exec bit in transit).
   - The user-facing Availability block and the pre-resolved
     `**Install command**` line with the absolute plugin path
     substituted in.
6. Confirm Claude's response relays the Availability copy in
   user-facing language (no phase or sub-phase references) and does
   not attempt to open the placeholder in a browser.

## Performance Considerations

None. Everything Phase 1 ships is a shell stub or a small Python
glob; execution is sub-millisecond and the total added disk
footprint is well under 20 KB.

## Migration Notes

Phase 1 adds new files (`scripts/test-helpers.sh`, the two new
visualisation scripts, two new test scripts, the new SKILL.md) and
makes targeted edits to three existing files (`scripts/test-config.sh`
invariants, `.claude-plugin/plugin.json` skills array, `tasks/test.py`
replacement task). The two pre-existing bash harnesses
(`test-config.sh`, `test-adr-scripts.sh`) are refactored to source
the shared helpers but their externally-observable behaviour is
unchanged.

Known Phase 2 churn points introduced by this phase:

- **`launch-server.sh` stdout shape.** The stub prints a single line.
  Phase 2's real launcher will emit richer output (URL + status
  signal + PID hint, likely JSON). The SKILL.md line
  `**Visualiser URL (not yet running)**: !`.../launch-server.sh`` is
  the consumer — its shape will change when the stub is replaced.
- **SKILL.md Availability block.** The scaffold-release wording (and
  the Claude-only HTML comment above it) is transitional. Phase 2
  replaces it with real operational guidance (reuse semantics, how
  to stop the server, log locations).
- **`test-config.sh` skill counts.** Future skills that use the
  context-injection preprocessors will require bumping `"14"` →
  `"15"` etc. The counts remain hand-curated in Phase 1; generalising
  them to derive from the manifest is noted as a follow-up.

## References

- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
- Design spec: `meta/specs/2026-04-17-meta-visualisation-design.md`
- Plan review: `meta/reviews/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding-review-1.md`
- Canonical SKILL.md preamble: `skills/github/review-pr/SKILL.md:1-36`
- Canonical path-resolution preamble (all 11 keys): `skills/config/init/SKILL.md:1-31`
- Canonical bash test harness (before extraction): `skills/decisions/scripts/test-adr-scripts.sh`
- Integration test task (before replacement): `tasks/test.py:4-11`
- Plugin manifest: `.claude-plugin/plugin.json`
- `config-read-path.sh` wrapper for path keys: `scripts/config-read-path.sh:1-24`
- Pure-bash symlink-walk pattern precedent: `scripts/config-read-path.sh:20` (`SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"`) and similar in every `scripts/config-*.sh`.
