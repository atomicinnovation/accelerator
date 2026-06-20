---
type: plan-validation
id: "2026-06-20-0116-structured-stall-on-no-decision-input-validation"
title: "Validation Report: Structured Stall on No Decision Input Implementation Plan"
date: "2026-06-20T18:49:30+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-20-0116-structured-stall-on-no-decision-input"
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T18:49:30+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Structured Stall on No Decision Input Implementation Plan

### Implementation Status

✓ Phase 1: `--decisions-file` switch on the driver — Fully implemented
✓ Phase 2: Structured stall at both emit sites — Fully implemented

Both phases were implemented in this session and landed across three
atomic commits:

- `cd64524c` — Add `--decisions-file` switch to the migration runner
- `c9a942a7` — Replace opaque interactive-migration aborts with a structured stall
- `82300a4a` — Mark 0116 stall plan phases complete; add its planning artifacts

Diff stat across the two implementation commits: 3 files changed,
277 insertions(+), 28 deletions(-) — `interactive-lib.sh`,
`run-migrations.sh`, `test-migrate-interactive.sh`. No source file
outside the plan's stated scope was touched.

### Automated Verification Results

**Phase 1**
- ✓ `--decisions-file` / env-var parity (exit + JSONL count)
- ✓ `--decisions-file` no-arg → usage error
- ✓ `--decisions-file <missing>` → relocated validation fires (`does not exist`)
- ✓ `--skip` / `--unskip` non-regression after the reorder
- ✓ `--help` lists `--decisions-file`

**Phase 2**
- ✓ PROMPT-site no-input stall (marker, key, switch + env-var forms, id, no opaque string, no `unbound variable`)
- ✓ VALIDATE_ERR-site no-input stall (exactly one re-prompt, then stall)
- ✓ Exhausted-decisions-file regression — legacy abort fires, stall does **not**
- ✓ Unterminated final decision line is parsed and applied, not stalled

**Suite / lint / CI**
- ✓ `bash skills/config/migrate/scripts/test-migrate-interactive.sh` — 160 passed, 0 failed (exit 0)
- ✓ `bash skills/config/migrate/scripts/test-migrate-0007.sh` — exit 0
- ✓ `mise run scripts:check` — shfmt + ShellCheck + bashisms clean
- ✓ `mise run check` — full read-only CI mirror green (exit 0)

Every "Automated Verification" checkbox in both phases of the plan is
checked and independently re-confirmed during validation.

### Code Review Findings

#### Matches Plan:

- **`read_decision` three-valued contract** (`interactive-lib.sh:236-239`):
  header documents `0` / `1` / `2`; the bare-fd-0 branch returns `2` only
  when the read fails **and** `line` is empty, exactly as specified. The
  decisions-file-exhausted and `/dev/tty` paths keep `return 1`.
- **`emit_no_input_stall`** (`:306`): prints the `MIGRATION STALLED: no
  decision input available` marker, names the pending key, emits the
  `--decisions-file` switch form (primary) and the
  `ACCELERATOR_MIGRATE_DECISIONS_FILE=` env-var form, and embeds the
  `migrations-<id>-decisions.txt` path. Resume lines are flush-left
  (copy-pasteable); diagnostic lines carry the `[$id]` prefix.
- **`read_decision_or_stall`** (`:344`): centralises the read/branch/emit
  logic, `set -e`-safe via `|| rc=$?`, returns `read_decision`'s status so
  the caller keeps the `exec 7>&-; return 1` teardown.
- **Both emit sites converted** (`:516`, `:550`): PROMPT keyed on `p_key`
  with verb `decision`; VALIDATE_ERR keyed on `LAST_PROMPT_KEY` with verb
  `re-decision`. The legacy `failed to obtain …` string now exists only
  inside the wrapper — no drift between the two sites.
- **`run-migrations.sh`**: `--decisions-file` case sets+exports the env
  var and `shift 2`s through to a normal run; `--help`/`-h` lists all
  three flags; validation block relocated to `1b`, after flag parsing,
  covering both env- and flag-supplied paths.
- **Constraints honoured**: stall text is ASCII-only (bash-3.2-clean, no
  associative arrays / `${var,,}`); the pre-flight session-log resume hint
  (`run-migrations.sh:89-134`) is byte-unchanged; no `--list` or
  fail-closed count/verb validation was added (correctly deferred to 0117).

#### Deviations from Plan:

- None of substance. One trivial formatting normalisation: the parity
  test's `DEC=$(mktemp); printf …` one-liner from the plan snippet was
  split across two lines to satisfy shfmt (no behavioural change).

#### Potential Issues:

- **Intentional coverage gap (documented in the plan)**: the `/dev/tty`
  interactive-human arm is not exercised by the suite (no fake TTY). A
  future edit to `read_decision` that touched that branch would be
  unguarded. Flagged in the plan's "Key edge cases"; acceptable.
- **Accepted semantic change (documented)**: `--skip`/`--unskip` now
  `exit 0` before the relocated validation, so they succeed even with an
  invalid `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var. Harmless
  (skip/unskip never read the decisions file) and pinned by Phase 1 test 5.
- **Single-leading-flag constraint (documented)**: `--decisions-file`
  works only as the first argument and cannot be combined with another
  flag in one invocation. 0117's `--list` will convert the `if`/`case`
  into a `while`/`shift` loop, retiring this limitation.

### Manual Testing Required:

The two manual-verification checkboxes in Phase 2 require an interactive
terminal and a live scratch repo, so they were not executed during this
automated validation:

1. Stall block readability:
  - [ ] In a scratch repo with a pending interactive migration, run the
        migrate skill under an agent (no TTY); confirm the stall block
        prints with the correct key and a resume command naming the id.
  - [ ] Confirm the switch-form resume command is copy-pasteable as-is
        (paths resolve, no escaping artifacts).
2. Resume round-trip:
  - [ ] Author the named decisions file (`accept`/`skip`/`edit <value>`
        lines) and run the printed `--decisions-file` command; confirm
        the migration resumes and completes.

### Recommendations:

- The implementation is complete and merge-ready from an automated-checks
  standpoint. Run the two manual checks above once in a real
  agent-invoked scratch repo to close the loop on copy-paste ergonomics.
- When 0117 lands, ensure the `--decisions-file` flag (0116's surface) and
  the promoted env var + `--list` (0117's surface) stay mutually
  consistent and the `migrations-<id>-decisions.txt` resume path remains
  stable across both, as noted in the plan's Migration Notes.
