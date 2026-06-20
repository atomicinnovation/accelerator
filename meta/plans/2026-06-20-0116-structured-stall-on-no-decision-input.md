---
type: plan
id: "2026-06-20-0116-structured-stall-on-no-decision-input"
title: "Structured Stall on No Decision Input Implementation Plan"
date: "2026-06-20T16:08:59+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0116"
parent: "work-item:0116"
derived_from: ["codebase-research:2026-06-20-0116-structured-stall-on-no-decision-input"]
relates_to: ["work-item:0117", "work-item:0119", "work-item:0120"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
revision: "1466496a6b385c4fffe419c8f63ff2858b985032"
repository: "accelerator"
last_updated: "2026-06-20T18:19:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Structured Stall on No Decision Input Implementation Plan

## Overview

Replace the two opaque `read_decision()` failure aborts in the interactive
migration runner — `failed to obtain decision` (PROMPT frame handler) and
`failed to obtain re-decision` (VALIDATE_ERR re-prompt) — with a single
structured, machine-parseable **stall** that names the pending decision key and
prints an exact, copy-pasteable resume command. This is **fix B** for 0115: the
low-risk mitigation that makes every interactive migration debuggable under
agent invocation, even before the full agent↔decisions bridge (0117) lands.

The stall is a *re-messaging of an existing halt*, not new control flow. Both
emit sites already tear down identically (`exec 7>&-; return 1`); we change
which message they print and add explicit detection of the no-input case so the
stall fires **only** when there is genuinely no input channel.

As a deliberate part of this work (decided during planning) we also add a
`--decisions-file <path>` switch to `run-migrations.sh` so the resume command
the stall prints is a first-class, ergonomic invocation rather than a reference
to a hidden test-only env var. The stall prints the switch form as primary and
the `ACCELERATOR_MIGRATE_DECISIONS_FILE=` form as the documented equivalent.

## Current State Analysis

All relevant code lives in `skills/config/migrate/scripts/`:

- **`interactive-lib.sh:238-288`** — `read_decision()`. Source-selection chain:
  decisions file (fd 9) → `/dev/tty` (gated on `[ -t 0 ]` at `:259`) → bare fd 0
  (`:262`). On any read failure it `return 1`. Under agent invocation the var is
  unset, fd 0 is not a TTY and is at EOF, so the bare-fd-0 read at `:262` fails —
  indistinguishable, today, from a genuine read error.
- **`interactive-lib.sh:449-453`** — emit site 1 (PROMPT). Current key is the
  local `p_key` (`:443`); migration id is the function-local `id` (`:292`).
- **`interactive-lib.sh:484-488`** — emit site 2 (VALIDATE_ERR re-prompt).
  Current key is the **global** `LAST_PROMPT_KEY` (`:455`); the `p_*` locals are
  out of scope here. Same `id`.
- **`run-migrations.sh:14-37`** — `ACCELERATOR_MIGRATE_DECISIONS_FILE` defaulted,
  exported, and validated (dir / not-exist / not-readable → exit 1). Documented
  as "test-only … never user-facing".
- **`run-migrations.sh:42-65`** — existing `--skip` / `--unskip` flag block
  (uses `SKIP_FILE`/`STATE_FILE` defined at `:39-40`).
- **`run-migrations.sh:5`** — `RUNNER_SCRIPT_DIR="$SCRIPT_DIR"`, set before
  `interactive-lib.sh` is sourced (`:250`), so it is in scope inside the lib.
- **`run-migrations.sh:89-134`** — pre-flight session-log resume hint. **Premise
  correction:** it prints a `/accelerator:migrate` re-run and a `rm` discard line;
  it does **not** assign `ACCELERATOR_MIGRATE_DECISIONS_FILE`. The AC's env-var
  assignment shape exists nowhere in the codebase today — 0116 introduces it.

### Key Discoveries

- The no-input case is detectable precisely at one point: the bare-fd-0 read
  failure (`interactive-lib.sh:262`) is reachable **only** when the env var is
  unset and stdin is not a TTY. So returning a distinct status there pinpoints
  the no-input case without a separate pre-check, and without disturbing the
  decisions-file-exhausted (`:242-243`, `:251-252`) or TTY-EOF (`:260`) paths,
  which keep `return 1`.
- The two emit sites name the current key through **different variables**
  (`p_key` vs `LAST_PROMPT_KEY`), so the stall helper must take the key (and id)
  as parameters — confirmed by research.
- There is **no accumulator** of all pending keys (only the single-prompt
  `LAST_PROMPT_*` cache at `:455-461`). Naming the *current* key is the firm AC;
  all-keys is explicitly out of scope.
- **No existing test asserts on either baseline string** (`failed to obtain
  decision` / `failed to obtain re-decision`) — grep matches only the two source
  lines. Converting them breaks no assertion; new stall assertions are additive.
- Happy paths drive decisions exclusively through
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` and assert exit-0 + exact JSONL line
  counts (`test-migrate-interactive.sh:378-666`, `:1069-1111`). These are the
  AC3 non-regression anchors.
- bash 3.2 floor applies (macOS): the literal fd 9 exists because of it. New
  code must stay 3.2-clean (no associative arrays, no `${var,,}`) and ASCII-only
  in the stall text (no em-dashes).

## Desired End State

When an interactive migration emits a PROMPT (or a VALIDATE_ERR re-prompt) and
no input channel exists, the runner exits non-zero having printed a structured
stall block to stderr that:

1. carries a stable machine-detectable marker (`MIGRATION STALLED: no decision
   input available`);
2. names the current pending decision key;
3. prints a resume command in two equivalent forms — the new
   `bash <run-migrations.sh> --decisions-file <path>` switch form (primary), and
   the `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path> bash <run-migrations.sh>` env
   form — where `<path>` is `$PROJECT_ROOT/.accelerator/state/migrations-<id>-decisions.txt`
   (embeds the id as a literal substring; non-empty; in the accept/skip/edit
   line format the consumer expects);
4. does **not** contain the old baseline strings.

When a decisions file or TTY **is** present, behaviour is unchanged.

`run-migrations.sh --decisions-file <path>` is accepted, validated, and behaves
identically to setting the env var.

**Verification:** `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
exits 0 with the new stall tests passing and all prior tests unchanged; `mise run
check` is green.

## What We're NOT Doing

- **Not** listing all accumulated pending keys (no accumulator exists; current
  key only — per the work item note).
- **Not** changing exit semantics: the stall is a better-described halt, still
  non-zero, still `exec 7>&-; return 1`.
- **Not** altering the pre-flight session-log resume hint (`run-migrations.sh:89-134`).
  The stall builds its own resume line; 0119 owns partial-run resume.
- **Not** building the `--list` dry-emit flow or bulk count/verb fail-closed
  validation — that is 0117 (fix A).
- **Not** documenting the *full* canonical agent contract — the decisions-file
  format details, the fail-closed count/verb validation, the SKILL.md invoker
  procedure, and the env var's own `--help` promotion all remain 0117's job. We
  add a *minimal* `--help` entry for `--decisions-file` (so the flag the stall
  advertises is confirmable) but not the exhaustive interface documentation.
- **Not** editing the 0116 work item's acceptance criteria. Printing the env-var
  form alongside the switch keeps AC1/AC2 literally satisfied.

## Implementation Approach

Two phases, each a self-contained, green, independently mergeable change:

- **Phase 1** adds the `--decisions-file` switch to `run-migrations.sh` (plus a
  test). It is standalone-valuable and touches only the runner. The stall in
  Phase 2 references this switch, so Phase 1 lands first.
- **Phase 2** adds the no-input detection and the structured stall in
  `interactive-lib.sh` (plus two emit-site tests and the non-regression gate).

Both follow TDD: write the failing test first, then the implementation.

---

## Phase 1: `--decisions-file` switch on the driver

### Overview

Add `--decisions-file <path>` to `run-migrations.sh` so a decisions file can be
supplied via a flag, not just the env var. Reorder so the existing env-var
validation runs **after** flag parsing, giving a single validation site that
covers both the env-supplied and flag-supplied path.

### Changes Required

#### 1. Reorder validation below flag parsing and add the flag

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**:

- Keep the var default + export at the top (`:19-20`) but **move the validation
  block** (`:21-37`, the dir / not-exist / not-readable checks) to immediately
  **after** the `--skip`/`--unskip`/`--decisions-file` flag block. Nothing
  between them consumes the validated path, so the move is safe.
  - **Stated semantic change**: `--skip`/`--unskip` `exit 0` *before* the
    relocated validation, so an invocation like `--skip <id>` with an invalid
    `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var (which exits 1 today) will now
    succeed. This is harmless (skip/unskip never read the decisions file) and is
    accepted intentionally; test 5 pins that `--skip`/`--unskip` still work.
- Add a `--decisions-file` case to the existing flag block. It sets and exports
  the env var, then `shift`s and falls through to normal execution (unlike
  `--skip`/`--unskip`, which exit early).
  - **Single-leading-flag constraint**: the flag block is a one-shot
    `if [ $# -gt 0 ]; then case "$1"` (not a parsing loop), so `--decisions-file`
    works only as the *first* argument and cannot be combined with another flag
    in one invocation (e.g. `--decisions-file X --skip Y` silently ignores the
    `--skip`). Acceptable for this single-flag mitigation; **note that 0117's
    `--list` will force converting this `if`/`case` into a `while`/`shift`
    loop**, at which point ordering becomes irrelevant.
- Update the env-var's header comment (`:14-18`). It currently states the var is
  "test-only … Never documented in --help or any user-facing banner" — Phase 2
  falsifies that by printing it in the stall and Phase 1 by wiring the public
  `--decisions-file` flag onto it and documenting that flag in `--help` (below).
  Reword to note the var is now reachable via the documented `--decisions-file`
  flag and surfaced in stall output. The env var's *own* `--help` promotion and
  the fail-closed count/verb validation remain 0117's (0117 promotes the env var
  per its AC4 and adds `--list`).
- Add a minimal `--help` / `-h` case to the flag block so the
  `--decisions-file` flag the stall advertises is confirmable via the
  conventional channel. List all three flags with one-line descriptions; the
  *fuller* canonical contract (decisions-file format details, fail-closed
  semantics, the SKILL.md invoker procedure, and the env var's promotion) stays
  with 0117. Keep every line within 80 columns.

```sh
    --help | -h)
      cat >&2 <<'EOF'
Usage: run-migrations.sh [FLAG]
  --skip <id>             Mark migration <id> skipped; do not run it.
  --unskip <id>           Remove migration <id> from the skip list.
  --decisions-file <path> Scripted decisions for interactive migrations, one
                          per line: accept | skip | edit <value>. The resume
                          path the no-input stall points at.
EOF
      exit 0
      ;;
```

```sh
    --decisions-file)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --decisions-file <path>" >&2
        exit 1
      fi
      # Unlike --skip/--unskip (which exit 0), this flag sets the env var and
      # falls through to a normal migration run.
      ACCELERATOR_MIGRATE_DECISIONS_FILE="$2"
      export ACCELERATOR_MIGRATE_DECISIONS_FILE
      shift 2
      ;;
```

The relocated validation block then validates whatever the var holds (env- or
flag-supplied), so a flag-supplied missing/unreadable/dir path still exits 1
with the existing clear message.

### Changes Required (tests)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`

#### 2. `--decisions-file` / env-var parity (JSONL count, not just exit)

The load-bearing test: prove the flag plumbs the **same** path as the env var by
asserting an identical JSONL record count, not merely exit 0 (an exit-only check
would pass even if the flag set the var but the run never consumed it).

```sh
DEC=$(mktemp); printf 'accept\n' >"$DEC"
# Env-var run.
SBX_ENV=$(setup_sandbox "decfile-parity-env")
seed_predicate_sandbox "$SBX_ENV" "k1|f1|a1|v1|ambiguous|prose1"
RC_ENV=0
ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX_ENV" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 ACCELERATOR_MIGRATE_DECISIONS_FILE="$DEC" \
  bash "$DRIVER" >/dev/null 2>&1 || RC_ENV=$?
LOG_ENV="$SBX_ENV/.accelerator/state/migrations-0002-predicate-session.jsonl"
COUNT_ENV=$(wc -l <"$LOG_ENV" | tr -d ' ')
# Flag run (no env var).
SBX_FLAG=$(setup_sandbox "decfile-parity-flag")
seed_predicate_sandbox "$SBX_FLAG" "k1|f1|a1|v1|ambiguous|prose1"
RC_FLAG=0
ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX_FLAG" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" --decisions-file "$DEC" >/dev/null 2>&1 || RC_FLAG=$?
LOG_FLAG="$SBX_FLAG/.accelerator/state/migrations-0002-predicate-session.jsonl"
COUNT_FLAG=$(wc -l <"$LOG_FLAG" | tr -d ' ')
assert_eq "--decisions-file exit parity" "$RC_ENV" "$RC_FLAG"
assert_eq "--decisions-file JSONL count parity" "$COUNT_ENV" "$COUNT_FLAG"
```

#### 3. `--decisions-file` with no argument → usage error

```sh
RC=0
OUTPUT=$(PROJECT_ROOT="$(setup_sandbox "decfile-noarg")" \
  bash "$DRIVER" --decisions-file 2>&1) || RC=$?
assert_neq "no-arg --decisions-file exits non-zero" "0" "$RC"
assert_contains "usage message shown" "$OUTPUT" "Usage: run-migrations.sh --decisions-file"
```

#### 4. `--decisions-file <missing-path>` → relocated validation fires

```sh
RC=0
OUTPUT=$(PROJECT_ROOT="$(setup_sandbox "decfile-missing")" \
  bash "$DRIVER" --decisions-file /nonexistent/decisions.txt 2>&1) || RC=$?
assert_neq "missing --decisions-file path exits non-zero" "0" "$RC"
assert_contains "validation message shown" "$OUTPUT" "does not exist"
```

#### 5. `--skip` / `--unskip` non-regression after the reorder

The validation-block relocation moves validation below the flag block; assert (in
CI, not by hand) that the early-exiting recovery flags still work. These are
operator recovery levers, so a silent regression must be caught by CI.

```sh
SBX=$(setup_sandbox "skip-unskip-after-reorder")
SKIPF="$SBX/.accelerator/state/migrations-skipped"
RC=0
PROJECT_ROOT="$SBX" bash "$DRIVER" --skip 0002-predicate >/dev/null 2>&1 || RC=$?
assert_eq "--skip exits 0 after reorder" "0" "$RC"
assert_contains "skip entry added" "$(cat "$SKIPF" 2>/dev/null)" "0002-predicate"
RC=0
PROJECT_ROOT="$SBX" bash "$DRIVER" --unskip 0002-predicate >/dev/null 2>&1 || RC=$?
assert_eq "--unskip exits 0 after reorder" "0" "$RC"
assert_not_contains "skip entry removed" "$(cat "$SKIPF" 2>/dev/null)" "0002-predicate"
```

#### 6. `--help` documents `--decisions-file`

```sh
RC=0
OUTPUT=$(PROJECT_ROOT="$(setup_sandbox "help-lists-decisions-file")" \
  bash "$DRIVER" --help 2>&1) || RC=$?
assert_eq "--help exits 0" "0" "$RC"
assert_contains "--help lists --decisions-file" "$OUTPUT" "--decisions-file"
```

(Use whichever skip-file / sandbox helpers the suite already provides; the point
is that all five checks above are automated, not manual checkboxes.)

### Success Criteria

#### Automated Verification

- [x] New test passes: a `0002-predicate` run invoked with `--decisions-file
      <file>` (no env var) produces the **same** exit status and JSONL record
      count as the equivalent env-var run.
- [x] New test passes: `--decisions-file` with no argument exits non-zero with
      the usage message.
- [x] New test passes: `--decisions-file <missing-path>` exits non-zero and the
      output contains `does not exist` (relocated validation still fires).
- [x] New test passes: `--skip <id>` then `--unskip <id>` each exit 0 and the
      skip-file entry is added then removed after the reorder (automated, not a
      manual checkbox) — see Phase 1 test 5.
- [x] New test passes: `--help` exits 0 and its output lists `--decisions-file`
      (see Phase 1 test 6), so the flag the stall advertises is confirmable.
- [x] Full interactive suite passes:
      `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [x] Shell lint/format green: `mise run scripts:check`

---

## Phase 2: Structured stall at both emit sites

### Overview

Make `read_decision()` return a distinct status (`2`) on the no-input branch,
add a shared `emit_no_input_stall` helper plus a `read_decision_or_stall`
wrapper that centralises the read-status / branch / emit logic, and reduce both
emit sites to a single call to the wrapper (structured stall on `2`, legacy
message otherwise). Exit semantics (`exec 7>&-; return 1`) are preserved and
stay at the call sites.

### Changes Required

#### 1. Distinct no-input return status

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: in the bare-fd-0 sub-branch (`:262`), return `2` instead of `1`.
The decisions-file-exhausted and `/dev/tty` paths keep `return 1`. Also extend
`read_decision`'s header comment (`:236-237`) to document the now-three-valued
return contract, so the load-bearing `2` is discoverable at the definition site
(not only at the two call branches) — important because 0117 touches this same
region and must not collapse any non-zero to a generic failure.

```sh
# read_decision: read one line of input from the decisions file (if set)
# or /dev/tty (interactive). Sets DECIDE_OUTCOME, DECIDE_VALUE.
# Returns: 0 = decision read; 1 = read error / decisions file exhausted /
# TTY EOF; 2 = no input channel available (caller emits the structured stall).
```

```sh
  else
    if [ -t 0 ]; then
      IFS= read -r line </dev/tty || return 1
    else
      # No decisions file and stdin is not a TTY. Treat this as the no-input
      # case ONLY when the read fails AND nothing was read (EOF, no channel);
      # signal it distinctly (2) so callers emit the structured stall rather
      # than the opaque abort. A populated-but-unterminated final line (read
      # fails but `line` is non-empty) still carries a usable decision, so fall
      # through and parse it instead of discarding it.
      if ! IFS= read -r line && [ -z "$line" ]; then
        return 2
      fi
    fi
  fi
```

This is exact: the bare-fd-0 branch is reached only when the env var is unset
and `! [ -t 0 ]`, so `2` cannot be raised when a decisions file or TTY exists.
Piped stdin with data still succeeds and proceeds normally; `2` is raised only
when the read fails **and** `line` is empty (EOF with nothing buffered) — a
final decision line without a trailing newline still leaves `line` populated, so
it is parsed rather than misread as no-input.

#### 2. Shared stall helper

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: add a helper (near the other emit-time helpers) that prints the
structured block to stderr. Takes id + key as parameters (the key lives in
different variables at the two sites). ASCII-only; bash-3.2-clean.

```sh
# emit_no_input_stall <id> <key>
# Structured, parseable stall printed to stderr when no decision input channel
# exists. Replaces the opaque "failed to obtain {decision,re-decision}" abort.
# Diagnostic lines carry the [$id] log prefix; the resume command lines are
# emitted flush-left so they can be copied and run verbatim. Caller still
# performs `exec 7>&-; return 1`.
emit_no_input_stall() {
  local id="$1" key="$2"
  # PROJECT_ROOT and RUNNER_SCRIPT_DIR are both hard preconditions of the
  # interactive layer (set by run-migrations.sh before sourcing); the :-
  # default on the driver path is belt-and-braces, not a real fallback.
  local state_dir="$PROJECT_ROOT/.accelerator/state"
  local decisions_path="$state_dir/migrations-${id}-decisions.txt"
  local driver="${RUNNER_SCRIPT_DIR:-.}/run-migrations.sh"
  {
    echo "[$id] MIGRATION STALLED: no decision input available"
    echo "[$id]   pending decision: $key"
    echo "[$id]   No decisions file, terminal, or piped input was available to"
    echo "[$id]   answer this prompt, so the migration cannot proceed."
    echo "[$id]"
    echo "[$id]   This migration may have already partially modified the"
    echo "[$id]   working tree. Inspect or revert it before resuming;"
    echo "[$id]   resume-safety for partial runs is tracked separately (0119)."
    echo "[$id]"
    echo "[$id]   To resume: each run answers the current prompt only (you"
    echo "[$id]   may be stalled again for the next undecided transformation):"
    echo "[$id]     1. write the decision (accept | skip | edit <value>),"
    echo "[$id]        one per line, to: $decisions_path"
    echo "[$id]        (create this file yourself; do not overwrite existing"
    echo "[$id]        migrations-${id}-* state files)"
    echo "[$id]     2. then run (copy-pasteable):"
    echo ""
    echo "bash $driver --decisions-file $decisions_path"
    echo ""
    echo "[$id]   equivalent env-var form:"
    echo ""
    echo "ACCELERATOR_MIGRATE_DECISIONS_FILE=$decisions_path bash $driver"
  } >&2
}
```

#### 2b. Shared read-and-branch wrapper

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: rather than copy the read-status / branch / emit logic at both emit
sites (which would be the exact drift hazard 0116 is fixing), centralise it in
one wrapper. The wrapper reads the next decision, emits the stall on status `2`
or the legacy `failed to obtain <verb>` message otherwise, and returns
`read_decision`'s status so the **caller** still owns the `exec 7>&-; return 1`
teardown. It is `set -e`-safe via `|| rc=$?` and is always invoked in an `if !`
condition, which fully suppresses `set -e` inside it.

```sh
# read_decision_or_stall <id> <key> <verb>
# Reads the next decision. On no-input (status 2) emits the structured stall; on
# any other failure emits the legacy "failed to obtain <verb> for <key>" abort.
# Returns read_decision's status (0 on success); the caller tears down on nonzero.
read_decision_or_stall() {
  local id="$1" key="$2" verb="$3" rc=0
  read_decision || rc=$?
  if [ "$rc" -ne 0 ]; then
    if [ "$rc" -eq 2 ]; then
      emit_no_input_stall "$id" "$key"
    else
      echo "[$id] failed to obtain $verb for $key" >&2
    fi
  fi
  return "$rc"
}
```

#### 3. Convert emit site 1 (PROMPT)

**File**: `skills/config/migrate/scripts/interactive-lib.sh` (`:449-453`)
**Changes**: replace the abort block with a single call to the wrapper, keyed on
the PROMPT-local `p_key` with verb `decision`.

```sh
        if ! read_decision_or_stall "$id" "$p_key" decision; then
          exec 7>&-
          return 1
        fi
```

#### 4. Convert emit site 2 (VALIDATE_ERR re-prompt)

**File**: `skills/config/migrate/scripts/interactive-lib.sh` (`:484-488`)
**Changes**: same single call, keyed on the global `LAST_PROMPT_KEY` with verb
`re-decision`.

```sh
        if ! read_decision_or_stall "$id" "$LAST_PROMPT_KEY" re-decision; then
          exec 7>&-
          return 1
        fi
```

### Changes Required (tests)

#### 5. PROMPT no-input stall test

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: mirror `seed_predicate_sandbox` with one `ambiguous` row, run the
`0002-predicate` fixture with **no `ACCELERATOR_MIGRATE_DECISIONS_FILE`** and
**stdin redirected from `/dev/null`** (critical: guarantees `! [ -t 0 ]` and
immediate EOF — do not rely on the ambient stdin of `$(...)`, which is a TTY
when the suite is run interactively and would hang on `/dev/tty`).

```sh
RC=0
SBX=$(setup_sandbox "stall-no-input")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f1|a1|v1|ambiguous|prose1"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" </dev/null 2>&1) || RC=$?
assert_neq "non-zero exit on no-input stall" "0" "$RC"
assert_contains "stall marker present" "$OUTPUT" "MIGRATION STALLED"
assert_contains "names the current key" "$OUTPUT" "k1"
assert_contains "resume switch form" "$OUTPUT" "--decisions-file"
assert_contains "resume names the driver" "$OUTPUT" "run-migrations.sh"
assert_contains "resume env-var form" "$OUTPUT" "ACCELERATOR_MIGRATE_DECISIONS_FILE="
assert_contains "migration id in resume path" "$OUTPUT" "0002-predicate"
assert_not_contains "old opaque message gone" "$OUTPUT" "failed to obtain decision"
# Guard the new set -u plumbing: the stall path must complete cleanly, not exit
# non-zero via a shell error (assert_neq alone would pass on such a crash).
assert_not_contains "no shell errors on stall path" "$OUTPUT" "unbound variable"
```

#### 6. VALIDATE_ERR no-input stall test

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: to reach the VALIDATE_ERR site **without** a decisions file (which
would disable the stall by design), supply exactly **one** decision via a stdin
pipe so the first `read_decision` succeeds, fails validation (empty `edit `),
and the re-prompt's `read_decision` then hits EOF → `return 2`.

```sh
RC=0
SBX=$(setup_sandbox "stall-revalidate-no-input")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|artifact|a|original|ambiguous|p"
OUTPUT=$(printf 'edit \n' | \
  ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" 2>&1) || RC=$?
assert_neq "non-zero exit on re-decision no-input stall" "0" "$RC"
assert_contains "validator message surfaced" "$OUTPUT" "empty value not allowed"
# Ordering + multiplicity: exactly one validation re-prompt occurred before the
# stall (assert_contains alone is presence-only and order-agnostic). Use the
# validator message as the per-occurrence marker, matching the combined 2>&1
# grep style; the implementer may instead count VALIDATE_ERR frames against
# $runner_log_path, as the existing AC-8 test does.
VE_COUNT=$(printf '%s\n' "$OUTPUT" | grep -c "empty value not allowed" || true)
assert_eq "exactly one validation re-prompt before stall" "1" "$VE_COUNT"
assert_contains "stall marker present" "$OUTPUT" "MIGRATION STALLED"
assert_contains "names the current key" "$OUTPUT" "k1"
assert_contains "resume switch form" "$OUTPUT" "--decisions-file"
assert_not_contains "old opaque message gone" "$OUTPUT" "failed to obtain re-decision"
assert_not_contains "no shell errors on stall path" "$OUTPUT" "unbound variable"
```

#### 7. Decisions-file-exhausted regression test (legacy `rc!=2` arm)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: the two stall tests above exercise only the `rc==2` arm. This test
pins the legacy `rc==1` arm so the stall cannot be mis-fired on the
decisions-file path and the `failed to obtain` message is not silently lost.
Supply a decisions file with **fewer** decisions than the run needs (here: one
decision for a two-prompt fixture), so the second `read_decision` hits
`decisions file exhausted` → `return 1` → the legacy abort, **not** the stall.

```sh
RC=0
SBX=$(setup_sandbox "stall-exhausted-not-stalled")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
# Two ambiguous rows => two PROMPTs; the decisions file answers only the first.
seed_predicate_sandbox "$SBX" \
  "k1|f1|a1|v1|ambiguous|prose1" \
  "k2|f2|a2|v2|ambiguous|prose2"
DEC=$(mktemp)
printf 'accept\n' >"$DEC"
OUTPUT=$(ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" --decisions-file "$DEC" 2>&1) || RC=$?
assert_neq "non-zero exit on exhausted decisions file" "0" "$RC"
assert_contains "exhaustion message surfaced" "$OUTPUT" "decisions file exhausted"
assert_contains "legacy abort fired" "$OUTPUT" "failed to obtain"
assert_not_contains "stall must NOT fire on exhausted file" "$OUTPUT" "MIGRATION STALLED"
```

(Mirror the seeder's actual multi-row signature if it differs from the above;
the load-bearing assertions are the presence of `decisions file exhausted` /
`failed to obtain` and the **absence** of `MIGRATION STALLED`.)

#### 8. Unterminated final decision line is parsed, not stalled

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: pins the emptiness-guard fall-through added in change 1 — a piped
final decision **without a trailing newline** still carries a usable decision,
so it must be parsed and applied, not misread as no-input. `printf 'accept'`
(no `\n`) makes `read` return non-zero with `line` populated.

```sh
RC=0
SBX=$(setup_sandbox "stall-unterminated-line")
echo "$SBX" >"$INTERACTIVE_FIXTURE_SANDBOX_FILE"
seed_predicate_sandbox "$SBX" "k1|f1|a1|v1|ambiguous|prose1"
OUTPUT=$(printf 'accept' | \
  ACCELERATOR_MIGRATIONS_DIR="$MIGRATIONS_DIR_FIXTURE/0002-predicate/migrations" \
  PROJECT_ROOT="$SBX" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
  ACCELERATOR_MIGRATE_FORCE=1 \
  bash "$DRIVER" 2>&1) || RC=$?
assert_eq "unterminated decision still applied (exit 0)" "0" "$RC"
assert_not_contains "no stall on a usable unterminated line" "$OUTPUT" "MIGRATION STALLED"
LOG="$SBX/.accelerator/state/migrations-0002-predicate-session.jsonl"
assert_eq "the decision was recorded" "1" "$(wc -l <"$LOG" | tr -d ' ')"
```

### Success Criteria

#### Automated Verification

- [x] Phase 2 tests (5, 6, 7, 8) pass — the two stall tests, the
      decisions-file-exhausted regression that pins the legacy `rc!=2` arm, and
      the unterminated-final-line test that pins the emptiness-guard fall-through.
- [x] AC3 non-regression: all prior interactive tests pass unchanged —
      `bash skills/config/migrate/scripts/test-migrate-interactive.sh` exits 0.
- [x] 0007 suite green (delegates its interactive body to the interactive
      suite): `bash skills/config/migrate/scripts/test-migrate-0007.sh`
- [x] Shell lint/format + bashisms green: `mise run scripts:check`
- [x] Full read-only CI mirror green: `mise run check`

#### Manual Verification

- [ ] The printed stall block is readable and the switch-form resume command is
      copy-pasteable as-is (paths resolve, no escaping artifacts).
- [ ] Running the printed resume command after authoring the named decisions
      file resumes the migration to completion.

---

## Testing Strategy

### Unit / suite Tests

- **Phase 1**: `--decisions-file` parity with env var (exit + JSONL count);
  no-arg usage error; missing-path validation error.
- **Phase 2**: PROMPT-site stall (no env var, stdin `</dev/null`) and
  VALIDATE_ERR-site stall (one piped `edit ` then EOF). Both assert the marker,
  the current key, the switch + env-var + id + driver substrings, and the
  absence of the corresponding old baseline string.
- **Non-regression (AC3)**: the existing accept/edit/skip and doc-example
  determinism gates (`:378-666`, `:1069-1111`) must pass unchanged.

### Key edge cases

- Decisions file **present but exhausted** → keeps `return 1` and the existing
  `decisions file exhausted` + legacy abort (no stall). Explicitly gated by test
  7 (a short decisions file for a two-prompt fixture asserts the legacy abort
  fires and `MIGRATION STALLED` does **not**); the happy-path tests do not cover
  this arm because they supply exactly enough decisions and exit 0.
- TTY present (interactive human) → unchanged. Not exercised by the suite (no
  fake TTY) and intentionally untouched. **Known, intentional coverage gap**:
  the `/dev/tty` arm is the one branch distinguishing "has a channel" from "no
  channel", so a future change to `read_decision` that touched it would be
  unguarded — flagged here so a maintainer editing this function knows.
- Piped stdin **with** data → consumed normally; only EOF on the bare-fd-0
  branch raises `2`.

### Manual Testing Steps

1. In a scratch repo with a pending interactive migration, run the migrate skill
   under an agent (no TTY); confirm the stall block prints with the correct key
   and a resume command naming the migration id.
2. Author the named decisions file (`accept`/`skip`/`edit <value>` lines) and
   run the printed `--decisions-file` command; confirm the migration resumes and
   completes.

## Performance Considerations

None — the change is on a failure path that previously aborted anyway.

## Migration Notes

No data/state format changes. The `.accelerator/state/migrations-<id>-decisions.txt`
path named in the resume command is a *suggested* path the invoker authors; it
is not created or read by this change unless the invoker supplied it. The stall
text explicitly tells the invoker to create the file and not to overwrite
existing `migrations-<id>-*` state files, since it sits alongside the live
session log and resume-state. **Contract ownership**: 0116 owns the
`--decisions-file` flag as the primary agent-facing resume contract and
documents it in `--help` itself (it does not depend on a 0117 amendment —
verified that 0117's AC4 promotes the *env var* `ACCELERATOR_MIGRATE_DECISIONS_FILE`
into `--help` and adds `--list`, not a `--decisions-file` flag). The env-var form
the stall prints remains the documented equivalent. The two documented surfaces
(0116's flag in `--help`; 0117's env var in `--help` plus `--list` and the
canonical SKILL.md invoker contract) must stay mutually consistent, and the
`migrations-<id>-decisions.txt` path the stall prints should remain stable across
both — but there is no re-messaging dependency: each work item documents its own
surface.

## References

- Original work item: `meta/work/0116-structured-stall-on-no-decision-input.md`
- Research: `meta/research/codebase/2026-06-20-0116-structured-stall-on-no-decision-input.md`
- Emit sites: `skills/config/migrate/scripts/interactive-lib.sh:449-453`, `:484-488`
- No-input branch: `skills/config/migrate/scripts/interactive-lib.sh:262`
- Driver flags + env-var validation: `skills/config/migrate/scripts/run-migrations.sh:14-37`, `:42-65`
- Pre-flight resume hint (shape reference only): `skills/config/migrate/scripts/run-migrations.sh:89-134`
- AC3 non-regression anchors + VALIDATE_ERR pattern to mirror:
  `skills/config/migrate/scripts/test-migrate-interactive.sh:378-666`, `:632-666`, `:1069-1111`
- Related: 0117 (fix A — formal `--decisions-file`/`--list` promotion), 0119
  (partial-run resume), 0120 (asserts this stall)
