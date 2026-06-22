---
type: plan
id: "2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"
title: "Agent-Decisions Bridge and Documented Invoker Contract Implementation Plan"
date: "2026-06-21T00:48:14+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0117"
parent: "work-item:0117"
derived_from: ["codebase-research:2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"]
relates_to: ["work-item:0115", "work-item:0116", "work-item:0118"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
revision: "8bebd60d03af8132c1efddc31c8c8f9fb0b834d8"
repository: "accelerator"
last_updated: "2026-06-22T15:05:14+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Agent-Decisions Bridge and Documented Invoker Contract Implementation Plan

## Overview

Give an agent a supported way to answer interactive-migration prompts. Three
net-new driver surfaces, on top of the `--decisions-file`/structured-stall
machinery 0116 already landed:

1. A **`--list`** dry-emit mode that enumerates every pending interactive
   transformation (key, proposed value, context) up front — without mutating
   the corpus — so an agent can populate a decisions file it could not write
   blind (proposed values are revealed only by the prompts).
2. **Fail-closed validation** of the decisions file via a **no-mutation
   dry-apply pass** — the real decide loop with mutation, session-log writes, and
   the `APPLY` round-trip suppressed. Because validation is the apply loop minus
   mutation, it runs `migration_validate_edit` on every `edit` value (failing
   *fast*, before any file is touched) and consumes the decisions file
   identically to the live run by construction. It fails closed on a rejected
   edit, an unknown verb, too few, or too many decisions, naming the offending
   position and leaving the corpus unmutated.
3. A documented **invoker contract** in `SKILL.md` (`list → decide → write →
   resume`), promotion of `ACCELERATOR_MIGRATE_DECISIONS_FILE` into `--help`,
   and a recorded judgment that this change is an implementation detail under
   ADR-0037 (not an amendment).

This is fix A of 0115 — the only mitigation that makes interactive migrations
*completable* under agent invocation. All of 0117's definition of done (AC1–AC6)
is verified against a **standalone reference fixture** and does not depend on
0118 having landed; the live-0007 integration (AC7) is explicitly out of scope.

## Current State Analysis

The migration driver is `skills/config/migrate/scripts/run-migrations.sh`. It
forks each interactive migration as a child, talking to it over two FIFOs using
the wire protocol in `scripts/interactive-protocol.sh`; the child-side harness is
`scripts/interactive-harness.sh`; the runner-side prompt/decision loop is
`skills/config/migrate/scripts/interactive-lib.sh`.

**0116 has already landed**, so part of 0117's *described* scope is done:

- `--decisions-file <path>` flag exists (`run-migrations.sh:56-66`) and sets +
  exports `ACCELERATOR_MIGRATE_DECISIONS_FILE`.
- The env var is **no longer** described as a "hidden test-only seam"; the inline
  comment (`run-migrations.sh:14-21`) even names this ticket.
- The structured stall (`emit_no_input_stall` / `read_decision_or_stall`,
  `interactive-lib.sh:306-356`) prints the stable marker `MIGRATION STALLED: no
  decision input available` and resume commands. `read_decision` returns a
  three-valued status (0 ok / 1 read-error / 2 no-input).

What genuinely remains, and the constraints found in the code:

- **No dry-run / `--list` path exists.** Mutation lives only in the child's
  `migration_apply_decision`, gated on the runner sending `APPLY` after
  persisting a session record. Even `skip` writes a session-log record and
  triggers `APPLY`. So a "feed all skips" approach would leave side effects — the
  clean design is a **child-side list mode** that dumps the already-buffered
  `TX_LINES` (`interactive-harness.sh:272-287`) *before* the decide handshake.
- **All four `--list` fields exist at emission time** (`key` / `proposed` /
  `path` / `anchor` — `interactive-protocol.sh:26`), and `render_prompt` already
  joins `path:anchor` with a colon (`interactive-lib.sh:213`) — exactly the
  `--list` join the work item specifies.
- **Flag parsing is a single-leading-flag `if/case` on `$1`**
  (`run-migrations.sh:35-79`) with **no `*)` catch-all** — an unknown first arg
  silently falls through to a normal run. `--list` forces conversion to a
  `while`/`shift` loop.
- **Decisions-file validation is filesystem-only** (`run-migrations.sh:82-98`):
  not-a-directory / exists / readable. There is **no content validation**.
  `read_decision`'s `*)` arm passes unknown verbs through silently
  (`interactive-lib.sh:293-296`).
- **Position maps to the decision-requiring PROMPT subset**, not to all
  transformations: mechanical/resumed transformations consume no decisions-file
  line. `--list` numbering must derive from the same predicate-passing,
  non-resumed filter the child applies (`interactive-harness.sh:311-336`).
- **`SKILL.md`** documents the *author* side of the interactive contract
  extensively (`:89-214`) but says nothing about the *invoker* side; the worked
  example ends at `:214`, `## Executing the migration` starts at `:216`.
- **Test harness**: plain bash + `scripts/test-helpers.sh`; suite is
  `test-migrate-interactive.sh` (1271 lines, ends with `test_summary`). Fixtures
  live under `test-fixtures/interactive/` and are invoked via
  `ACCELERATOR_MIGRATIONS_DIR`. `seed_predicate_sandbox`
  (`:360-369`) and protocol-log assertions (`:401-404`) are the patterns to
  follow. `tasks/test/integration.py` discovers `test-*.sh` migrate suites by
  exec bit with a floor of `_EXPECTED_MIGRATE_SUITES = 4`.

### Key Discoveries:

- The cleanest `--list` lives in the **child** (`interactive-harness.sh`), which
  already buffers the full transformation list and applies the predicate filter
  — single source of truth for ordering and which transformations prompt
  (`interactive-harness.sh:272-336`).
- `INIT` carries two fields today (`resume_state_path`, `decisions_path`);
  `read_frame` tolerates a missing third field (`${FRAME_FIELDS[2]:-}` → empty),
  so an optional `list_mode` third field is backward-compatible — the normal
  `run_interactive_migration` INIT (`interactive-lib.sh:434-436`) needs no change.
- AC6 validation is a **no-mutation dry-apply pass**, not an up-front count
  check. A naive `verbs == N` check is unsound: a rejected `edit` triggers a
  `VALIDATE_ERR` re-prompt that consumes an *extra* decisions-file line
  (`interactive-lib.sh:537-561`, verified), so a file passing `verbs == N` can
  still exhaust mid-run *after* earlier transformations have mutated the corpus.
  Dry-apply reuses the real `read_decision`/predicate/`migration_validate_edit`
  path with mutation suppressed, so what validates is exactly what applies —
  too-few/too-many fall out of the same consumption, and a bad `edit` value is
  caught before the first write.
- `read_decision`'s three-valued return and the 0116 stall region must **not** be
  collapsed (shared-region merge constraint with 0116). Dry-apply *reuses*
  `read_decision` unchanged — it does not fork the verb grammar. The silent `*)`
  pass-through (`interactive-lib.sh:293-296`) becomes unreachable for the
  decisions-file path because dry-apply runs first and rejects an unknown
  `DECIDE_OUTCOME` (any token not `accept`/`skip`/`edit`) before the live run; the
  live path itself is left untouched.
- The decisions file is consumed via fd 9 in `read_decision`, and mutation
  happens only after the runner sends `APPLY` in response to a `RECORDED` frame
  (`interactive-lib.sh:563-573`). A child **dry-apply** mode that emits
  `DRY_OK`/`DRY_REJECT` instead of `RECORDED` therefore never reaches `APPLY` and
  never mutates, while exercising the identical decide/validate path. In
  dry-apply a failed `migration_validate_edit` is a **hard reject** (fail fast),
  not a re-prompt — so the live run, reached only for a fully-valid file, cannot
  partial-mutate.
- Three call sites now fork the interactive child (live run, `--list`
  enumeration, dry-apply), and the per-transformation resume+predicate routing is
  needed by all three — so the FIFO fork/teardown and the TX classify logic are
  **extracted into shared helpers** (`_interactive_fork`/`_interactive_teardown`,
  `_harness_classify_tx`) rather than cloned (see Implementation Approach).
- ADR-0037's recursive-supplement clause is the test for "amendment vs detail":
  the invoker bridge adds no new control verb, display element, or resumability
  guarantee, and the ADR is neutral on invocation mechanics → implementation
  detail.
- bash 3.2 floor, ASCII-only, 80-col apply to all new shell (macOS CI). fd 7/8/9
  are literal because bash 3.2 has no `{var}<` allocator.

## Desired End State

- `bash run-migrations.sh --list` prints the pending interactive transformations
  as `<position>\t<key>\t<proposed>\t<path>:<anchor>` lines (or `no pending
  transformations`), exits 0, and mutates nothing — runnable even on a dirty
  tree.
- A decisions file that does not match the pending transformations (wrong count
  or an unknown verb) is rejected before any mutation, with a message naming the
  offending position and a non-zero exit.
- `ACCELERATOR_MIGRATE_DECISIONS_FILE` appears in `--help` with a one-line format
  description, alongside a `--list` line.
- `SKILL.md` documents the invoker contract end-to-end, confirmable by string
  search.
- A standalone reference fixture (three pinned interactive transformations
  writing real frontmatter) exercises AC1–AC6 with no dependency on the live 0007
  corpus.

Verify: `mise run check` is green; `bash
skills/config/migrate/scripts/test-migrate-interactive.sh` passes including the
new AC1–AC6 cases; `grep` confirms the `--help` and `SKILL.md` strings.

## What We're NOT Doing

- **AC7 (live-0007 integration)** — out of scope; gated on 0118 letting 0007
  reach its interactive stage. Owned by 0118 / the parent epic.
- **No second `--decisions-file` flag** — 0116 owns it; we only validate its
  content and promote the env var into `--help`.
- **No change to `read_decision`'s three-valued return or the 0116 live stall** —
  shared-region constraint. Dry-apply reuses `read_decision` as-is; the unknown-
  verb and edit-validation rejections live in the dry-apply driver, not in the
  live prompt loop.
- **No per-migration decisions-file *input* wiring for multiple interactive
  migrations.** fd 9 is reopened per interactive migration today
  (`interactive-lib.sh:379-383`), so a single decisions file cannot feed two
  migrations. `--list` **segments its display per migration** (id-prefixed
  sections, positions restarting at 1) so the position contract is correct when
  multi-migration eventually lands, but the resume path still consumes one
  decisions file per interactive migration — the multi-file resume protocol is a
  follow-up. The fixture and live 0007 are single-interactive-migration.
- **No all-or-nothing apply across transformations.** The fail-closed guarantee
  is scoped to *validation*: a malformed decisions file is rejected by dry-apply
  before any mutation. Once the live apply loop begins (only for a validated
  file), it mutates per transformation with no rollback, so an apply-time failure
  (e.g. a `migration_apply_decision` error) can leave a partial corpus — VCS
  revert is the recovery path (consistent with the existing stall message and
  0119's guarded resume, now landed). `SKILL.md` states this scope explicitly
  rather than claiming "never a
  partial application" unconditionally. Note too that dry-apply validates only the
  **prompt-route** transformations (those requiring a decision); **mechanical**-route
  applies (predicate rc 1, which mutate without a decisions-file line) are not
  dry-run, so the apply-time partial-mutation caveat applies to them in full. The
  reference fixture is all-prompt, so this gap is unexercised by AC1–AC6.
- **No new dirty-tree-resume machinery — 0119 already provides it.** The
  dirty-tree pre-flight (`run-migrations.sh:252-358`) is bypassed for the
  read-only `--list`. For a `--decisions-file` *resume* on a dirty tree, **0119 has
  landed** (status `done`): its manifest-based **guarded resume**
  (`run-migrations.sh:269-358`) lets a re-run proceed **without
  `ACCELERATOR_MIGRATE_FORCE=1`** when every dirty path is owned by this run — the
  mechanical path manifest *and*, via 0119's Phase 4 interactive reconciliation,
  the current-run interactive session log (`dirty_tree_fully_owned` +
  `is_session_artifact`, base revision unchanged). The canonical agent flow (stall
  at the first prompt → no corpus mutated, only the owned session log dirty)
  therefore resumes cleanly without `FORCE`. `FORCE` remains the escape only for
  genuinely *un-owned* dirt: foreign changes, a moved base revision (the operator
  committed since the partial run), or interactive corpus mutations that are
  deliberately not manifest-tracked — and the `--decisions-file` path never reaches
  the last case, because dry-apply (Phase 2) rejects a too-few file before any
  apply. So 0117 adds **no** dirty-tree-resume code; it only *documents* the
  guarded-resume behaviour 0119 ships. The 0116 stall's copy-pasteable command was
  also reconciled by 0119: it already omits `FORCE` and tells the operator the
  re-run resumes when the base revision is unchanged
  (`interactive-lib.sh:306-339`). The SKILL.md invoker contract must match that —
  it must **not** re-introduce a `FORCE` prefix (Phase 3 §1 step 4).
- **No `migration_verify_applied`/DRIFT handling in list mode** — `--list`
  excludes resumed keys; drift re-prompting is a resume-time concern not modelled
  in the dry emit (documented as such).
- **No ADR-0037 amendment** — recorded as an implementation detail instead.

## Implementation Approach

The child gains two extra modes signalled by an optional third `INIT` field
(`1` = list, `2` = dry-apply; absent/empty = normal run). **List mode** emits
`LIST_ENTRY`/`LIST_DONE` for the predicate-passing, non-resumed subset and exits
before the decide handshake (Phase 1). **Dry-apply mode** runs the real
decide/validate loop but emits `DRY_OK`/`DRY_REJECT`/`DRY_DONE` instead of
`RECORDED`, so it never reaches `APPLY` and never mutates (Phase 2).

Because three call sites now fork the child (live run, list, dry-apply), the
fork/teardown is extracted into a shared `_interactive_fork`/`_interactive_teardown`
(parameterised by the INIT frame and a per-frame handler), and the
per-transformation resume+predicate routing into a shared `_harness_classify_tx`
(setting `key`/`path`/`anchor`/`proposed` and a `route` of resumed|mechanical|
prompt) consumed by the main loop, list mode, and dry-apply alike. The runner
gains `enumerate_interactive_transformations` (read-only fork, Phase 1) and
`dry_apply_interactive_migration` (Phase 2), both built on `_interactive_fork`.
`SKILL.md` and the ADR judgment are documentation-only (Phase 3).

The `_interactive_fork` extraction touches the fork plumbing adjacent to 0116's
`interactive-lib.sh` work, so **merge order must be coordinated with 0116** (the
work item already records this constraint); land this after — or alongside — 0116
and re-run the migrate suite on the merge.

TDD throughout: each behavioural criterion gets its assertion before/with the
code that satisfies it, and each phase leaves `mise run check` + the migrate
suite green so it is independently mergeable.

---

## Phase 1: `--list` dry-emit, standalone fixture, flag parser, `--help`

### Overview

Add the child-side list mode and the runner `--list` surface, the standalone
real-frontmatter reference fixture, the `while/shift` flag parser with strict
unknown-flag rejection, and the `--help` promotions. Satisfies **AC1, AC3, AC4**
and verifies **AC2** against the new fixture's apply path (which already works
via the landed `--decisions-file` machinery).

### Changes Required:

#### 1. Wire protocol — optional list-mode handshake field + list frames

**File**: `scripts/interactive-protocol.sh`
**Changes**: Documentation-only edit to the protocol comment block. Extend `INIT`
to an optional third field and add the two list frames.

```bash
# Runner → migration (migration's stdin):
#   INIT  resume_state_path  decisions_path  [mode]   handshake
#         (mode="1" = dry enumeration / list; mode="2" = dry-apply validation;
#          absent/empty = normal run)
#   ...
# Migration → runner (migration's stdout):
#   ...
#   LIST_ENTRY  key path anchor proposed        one pending decision (list mode)
#   LIST_DONE                                    end of list-mode enumeration
#   DRY_OK       key                            decision validated, not applied
#   DRY_REJECT   key reason                     edit/verb rejected (dry-apply)
#   DRY_DONE                                    end of dry-apply validation
```

No escape/format changes — `LIST_ENTRY`/`DRY_*` fields use the same
`escape_field` encoding as `PROMPT`. The third `INIT` field is optional and
backward-compatible: `read_frame` tolerates its absence (`${FRAME_FIELDS[2]:-}`),
and the normal-run INIT emission (live and protocol-log,
`interactive-lib.sh:434-441`) is left two-field — only the new list and dry-apply
forks set the third field.

#### 2. Child-side list mode

**File**: `scripts/interactive-harness.sh`
**Changes**: Read the optional `mode` from `INIT` in `harness_run`; after emitting
`READY` and buffering `TX_LINES`, branch on `mode="1"` to `_harness_emit_list`
(passing the buffered lines) before the decide loop. Both the list emitter and
the main loop route each TX through a new shared `_harness_classify_tx`.

**Shared classify helper (Decision 2 — eliminates the parse clone).** Extract the
per-TX field-extraction + resume lookup + predicate routing the main loop does
today (`interactive-harness.sh:286-336`, including the load-bearing here-string
predicate eval that avoids SIGPIPE-141 under `pipefail`) into one helper, so the
main loop, list mode, and dry-apply (Phase 2) share a single definition:

```bash
# _harness_classify_tx <tx>: parse one buffered TX line and decide its route.
# Sets the FULL set of per-TX globals (UNESCAPED, exactly as the main loop's
# unescape_field extraction) so NO caller re-parses: key/path/anchor/proposed AND
# predicate_value/extras_tsv/display_b64 (the live PROMPT build needs all seven —
# interactive-harness.sh:298-304), plus ROUTE in {resumed, mechanical, prompt}.
# The ONLY place TX parsing / resume / predicate routing lives — callers act on
# ROUTE and read these globals; they do not re-split the TX.
_harness_classify_tx() {
  local tx="$1"
  # field-split + unescape_field per field (was inline in the main loop)
  ...
  _HARNESS_CURRENT_TSV="$tx"
  _harness_resume_lookup "$key"
  if [ "$RESUME_FOUND" -eq 1 ]; then ROUTE=resumed; return 0; fi
  local predicate_rc=0
  if declare -F migration_evaluate_predicate >/dev/null; then
    # here-string, NOT a pipe: a pipe would mask predicate exit via SIGPIPE-141
    migration_evaluate_predicate <<<"$tx" >/dev/null 2>&1 || predicate_rc=$?
  fi
  case "$predicate_rc" in
    0) ROUTE=prompt ;;
    1) ROUTE=mechanical ;;
    *) ROUTE=fail; PREDICATE_RC="$predicate_rc" ;;
  esac
}
```

```bash
# in harness_run, after reading INIT:
local resume_state_path="${FRAME_FIELDS[0]:-}"
# FRAME_FIELDS[1] = decisions_path (runner-side concern, unused here)
local mode="${FRAME_FIELDS[2]:-}"
...
emit_frame READY "$session_log_path"
# ... buffer TX_LINES (unchanged) ...
if [ "$mode" = "1" ]; then
  _harness_emit_list "${TX_LINES[@]+"${TX_LINES[@]}"}"   # Phase 1
  return 0
elif [ "$mode" = "2" ]; then
  _harness_dry_apply "${TX_LINES[@]+"${TX_LINES[@]}"}"    # Phase 2
  return 0
fi
# ... existing per-transformation decide loop (now via _harness_classify_tx) ...
```

```bash
# _harness_emit_list <tx...>: dry enumeration of decision-requiring
# transformations. Takes the buffered lines as ARGS (TX_LINES is local to
# harness_run), routes each via the shared _harness_classify_tx, and emits
# LIST_ENTRY only for the 'prompt' route. Mutates nothing.
_harness_emit_list() {
  local tx
  for tx in "$@"; do
    _harness_classify_tx "$tx"
    case "$ROUTE" in
      resumed)    : ;;                       # already decided -> no line consumed
      mechanical) : ;;                       # mutates + consumes no line, excluded
      prompt)     emit_frame LIST_ENTRY "$key" "$path" "$anchor" "$proposed" ;;
      fail)       emit_frame FAIL "predicate returned $PREDICATE_RC for key $key"
                  return 1 ;;
    esac
  done
  emit_frame LIST_DONE
}
```

Note: `key`/`path`/`anchor`/`proposed` are set **unescaped** by
`_harness_classify_tx` (the same `unescape_field` extraction the main loop uses);
`emit_frame` re-escapes on the wire and the runner unescapes once on receipt, so
the round-trip is byte-identical to the `PROMPT` path even for values containing
TABs/newlines/backslashes. Passing `TX_LINES` as args keeps `_harness_emit_list`
a separately-testable function rather than an inline block in the already-long
`harness_run`.

#### 3. Runner-side enumeration helper

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: Extract the FIFO setup / fork / READY handling / teardown shared by
`run_interactive_migration` into `_interactive_fork` / `_interactive_teardown`
(Decision 2 — eliminates the fork clone across the live run, list, and
dry-apply), then build `enumerate_interactive_transformations <path> <id>` on top
of it.

```bash
# _interactive_fork <path> <id> <mode> <frame_handler>
# Builds resume state (the pre-fork DEFAULT build_resume_state_file at
# interactive-lib.sh:370, so ALL three modes — not only custom-path ones —
# exclude already-decided keys), sets up the two FIFOs + literal fd 7/8 (bash 3.2
# has no {var}< allocator), forks the child with INIT carrying <mode> as the third
# field, and drives the frame loop dispatching each frame to <frame_handler>.
# Handles READY centrally — including the custom-session-log resume REBUILD
# (interactive-lib.sh:497-504) so list/dry-apply exclude already-decided keys
# identically to the live run. _interactive_teardown closes fd 7/8, removes the
# FIFOs, and reaps the child. Used by run_interactive_migration (mode=""),
# enumerate_interactive_transformations (mode=1), and
# dry_apply_interactive_migration (mode=2).
```

```bash
LIST_ENTRIES=()   # output of enumerate_interactive_transformations
enumerate_interactive_transformations() {
  local f="$1" id="$2"
  LIST_ENTRIES=()                       # reset-on-entry: callers cannot leak state
  _interactive_fork "$f" "$id" 1 _enum_handle_frame || return 1
}
_enum_handle_frame() {
  case "$1" in                          # $1 = frame type, $2.. = unescaped fields
    LIST_ENTRY)
      # Decision 3 guard BEFORE the tab-join: a field carrying an embedded TAB or
      # newline would corrupt the joined row (and the downstream split). Fail
      # closed here, while the fields are still individually intact.
      case "$2$3$4$5" in
        *$'\t'* | *$'\n'*)
          echo "[$id] --list field for key '$2' contains a tab or newline;" \
            "--list output is undefined for such values." >&2
          return 1 ;;
      esac
      LIST_ENTRIES+=("$2"$'\t'"$3"$'\t'"$4"$'\t'"$5") ;;
    LIST_DONE)  return "$_FORK_STOP" ;; # stop the fork loop cleanly (named sentinel)
    FAIL)       echo "[$id] $2" >&2; return 1 ;;
  esac
}
```

`_interactive_fork` opens **no fd 9, writes no session log, and sends no APPLY**
for modes 1 and 2, so no mutation path is reachable in either dry mode. A
non-zero return or a `FAIL` frame propagates out of
`enumerate_interactive_transformations` / `dry_apply_interactive_migration` so the
driver **aborts before the live apply loop** — an enumeration/validation fork
failure must never fall through to a mutating run (Phase 2 §3 wires the
`|| exit 1`).

**Responsibility split (so the extraction removes the clone, not relocates it).**
`run_interactive_migration` is rebuilt on `_interactive_fork` with a
`_live_handle_frame` handler. The split:

- **`_interactive_fork` owns** (shared by all three modes): pre-fork default +
  READY custom-path resume build, FIFO/fd-7/8 setup, child fork + INIT emission,
  the frame-dispatch loop, and `_interactive_teardown`.
- **The per-mode `frame_handler` owns** the per-frame action.
  `_live_handle_frame` is the large one — `PROMPT` -> `read_decision` + send
  `DECIDE`; `VALIDATE_ERR` -> the `LAST_PROMPT_*` re-prompt caching + `PROMPT_INDEX`
  decrement + re-`read_decision`; `RECORDED` -> `write_session_record` + send
  `APPLY` (the mutation); `DRIFT` -> `atomic_jsonl_remove_by_key` + `DRIFT_CLEARED`.
  The `MIGRATION_RESULT: no_op_pending` soft-defer (`:450-459`) is handled **here**
  (the handler returns a "stop, no-op" sentinel), keeping the fork generic.
- **A live-only post-call tail** (in `run_interactive_migration`, NOT the shared
  fork): the 30s watchdog (`:610-627`), the wait-status / `saw_done` reconciliation
  (`:629-640`), and the terminal side effects — STATE_FILE append,
  `INTERACTIVE_APPLIED=1`, resume/stderr cleanup. List/dry never run these.
- **Globals crossing the boundary** (the full handler↔fork contract, documented
  in `_interactive_fork`'s header so handlers can be reasoned about in isolation):
  - **`mig_in` / `mig_out`** — the FIFO write/read fds (literal 7/8) the fork
    sets up; handlers send frames back to the child via `>&"$mig_in"`
    (`_dry_send_decide`, `_live_handle_frame`'s DECIDE/APPLY/DRIFT_CLEARED). These
    **must** be named globals (not `local` to the fork) or passed to the handler,
    or the handler writes to an out-of-scope fd.
  - **`runner_log_path`** — handlers mirror their sent frames to the protocol log
    here; centralise the mirroring in one helper so it is not re-cloned per
    handler.
  - **`SESSION_LOG`** (set by shared READY handling, read by `_live_handle_frame`'s
    `RECORDED`/`DRIFT`), **`saw_done`**, **`saw_no_op_pending`** (set via the
    handler stop-sentinel, read by the tail), and the live re-prompt state
    (`LAST_PROMPT_*`, `PROMPT_INDEX`) the `_live_handle_frame` VALIDATE_ERR arm
    owns.
- **The `no_op_pending` soft-defer is a *pre-dispatch* concern.** It is a raw
  whole-frame string match (`MIGRATION_RESULT: no_op_pending`) that must fire
  **before** field-parsing and is illegal after READY (`:450-459`). It does not
  fit the parsed-type handler model, so `_interactive_fork` owns it as a generic
  pre-dispatch hook (check the raw frame + `saw_ready` guard before unescaping and
  dispatching), not `_live_handle_frame`. Only the live mode acts on it (sets
  `saw_no_op_pending`); list/dry never emit it.

#### 4. Flag parser → `while/shift`, `--list`, strict unknown-flag rejection

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Convert the `if [ $# -gt 0 ]; then case "$1" …` block
(`:35-79`) to a `while [ $# -gt 0 ]; do case "$1" … shift … done` loop. Add a
`--list` arm (`LIST_MODE=1; shift`) and a `*)` arm that rejects unknown flags
(`echo "Unknown argument: $1" >&2; exit 1`) instead of today's silent
fall-through. `--skip`/`--unskip`/`--help` keep exiting; `--decisions-file`
keeps `shift 2`.

```bash
LIST_MODE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --skip) ... exit 0 ;;
    --unskip) ... exit 0 ;;
    --decisions-file) ... ACCELERATOR_MIGRATE_DECISIONS_FILE="$2"; export ...; shift 2 ;;
    --list) LIST_MODE=1; shift ;;
    --help | -h) cat >&2 <<'EOF' ... EOF
      exit 0 ;;
    *) echo "Unknown argument: $1" >&2
       echo "Run with --help for usage." >&2
       exit 1 ;;
  esac
done
```

#### 5. `--list` branch + dirty-tree bypass + source ordering

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**:
- Gate the dirty-tree pre-flight (`:252-358`) on `if [ -z "$LIST_MODE" ]` — a dry,
  read-only list does not require a clean tree. This block is now larger than the
  plan originally assumed: 0119 added the manifest/run-id setup, the `RESUME` state,
  the guarded-resume branch, and the in-flight session-log steer inside it
  (`:269-358`), so `--list` must skip the **whole** block. Because `--list` excludes
  already-decided keys via the resume filter, when an in-flight session log is
  detected in `.accelerator/state/`, emit a one-line notice **on stderr** (not on
  the parseable stdout stream) that a session is in flight and `--list` shows only
  the remaining transformations, pointing at the same resume/discard guidance the
  pre-flight's affordance already prints (`:282-302`). Keeps the read-only path from
  being silent about partial state.
- Move `source "$RUNNER_SCRIPT_DIR/interactive-lib.sh"` (currently `:470`) up to
  immediately after the pending list is computed (`:421-436`), before the preview
  banner (`:438`) — sourcing only defines functions, so it is safe to do earlier.
- Insert the `--list` branch right after sourcing, before the preview banner /
  the "No pending migrations." early exit, so it owns its own output:

```bash
if [ -n "$LIST_MODE" ]; then
  # Identify pending interactive migrations up front (no fork needed) so we know
  # whether to segment before emitting. Then enumerate + print per migration —
  # no flat entry array / manual cursor / seq (avoids the index-arithmetic tangle
  # and the seq dependency; matches the dependency-free idiom in this tree).
  int_files=()
  for f in "${pending_files[@]+"${pending_files[@]}"}"; do
    is_interactive_migration "$f" && int_files+=("$f")
    # mechanical migrations are NOT run in list mode (they would mutate)
  done
  multi=0; [ "${#int_files[@]}" -gt 1 ] && multi=1
  if [ "$multi" -eq 1 ]; then
    # stderr only (stdout stays parseable data); fires only in the multi case,
    # so AC1's single-migration stderr-clean assertion is unaffected.
    echo "Note: ${#int_files[@]} interactive migrations pending; resume one at a" \
      "time with --decisions-file per '# migration <id>' section — a single" \
      "multi-migration decisions file is not yet supported." >&2
  fi
  emitted=0
  for f in "${int_files[@]+"${int_files[@]}"}"; do
    id="$(basename "$f" .sh)"
    enumerate_interactive_transformations "$f" "$id" || exit 1
    [ "${#LIST_ENTRIES[@]}" -eq 0 ] && continue
    # Decision 4: segment ONLY when >1 pending, so the single-migration case (the
    # real case + the AC1 fixture) stays bare canonical lines; positions RESTART
    # at 1 per migration to match the per-migration decisions file.
    [ "$multi" -eq 1 ] && printf '# migration %s\n' "$id"
    pos=0
    for entry in "${LIST_ENTRIES[@]}"; do
      pos=$((pos + 1)); emitted=$((emitted + 1))
      # entry = key<TAB>path<TAB>anchor<TAB>proposed (fields already guarded
      # against embedded TAB/newline at enumerate time in _enum_handle_frame, so
      # the split below is safe).
      IFS=$'\t' read -r k p a v <<<"$entry"
      printf '%s\t%s\t%s\t%s:%s\n' "$pos" "$k" "$v" "$p" "$a"
    done
  done
  [ "$emitted" -eq 0 ] && echo "no pending transformations"
  exit 0
fi
```

List lines (and the `# migration <id>` section headers, only when >1 interactive
migration is pending) go to **stdout** (parseable data); diagnostics stay on
stderr. The single-migration case emits exactly the bare canonical lines — no
header — so AC1's byte-for-byte output is preserved; the column legend lives in
`--help` and `SKILL.md`, not in the stream. Positions restart at 1 per migration
so each section maps 1:1 to that migration's decisions file.

#### 6. `--help` promotion (AC4)

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Extend the `--help` heredoc (`:67-75`) with a `--list` line, the
column legend, and an `ACCELERATOR_MIGRATE_DECISIONS_FILE` line carrying its
one-line format. **Route the explicit `--help`/`-h` path to stdout** (the heredoc
is `>&2` today); usage-on-error messages stay on stderr. This matches the
GNU/POSIX convention so an agent's `--help | grep` (stdout) finds the promoted env
var — AC4's verification can then drop the `2>&1`.

```text
Usage: run-migrations.sh [FLAG]
  --skip <id>             Mark migration <id> skipped; do not run it.
  --unskip <id>           Remove migration <id> from the skip list.
  --list                  Dry-emit pending interactive transformations, one
                          tab-delimited line each, then exit (no mutation):
                            <pos>\t<key>\t<proposed>\t<path>:<field>
                          With >1 pending interactive migration, output is
                          segmented by a `# migration <id>` header and <pos>
                          restarts at 1 per migration.
  --decisions-file <path> Scripted decisions for interactive migrations, one
                          per line: accept | skip | edit <value>. The resume
                          path the no-input stall points at.

Environment:
  ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>
                          Same as --decisions-file: newline-delimited verbs
                          (accept | skip | edit <value>) matched to pending
                          transformations by emission order. Validated up front
                          (dry-apply); a rejected edit, unknown verb, or wrong
                          count fails closed naming the position, corpus
                          unmutated.
```

The legend (`<pos>\t<key>\t<proposed>\t<path>:<field>`) lives here and in
`SKILL.md`, not as an inline header in `--list` stdout — AC1 pins the
single-migration output to exactly the bare data lines.

#### 7. Standalone real-frontmatter reference fixture

**File**: `skills/config/migrate/scripts/test-fixtures/interactive/0006-decisions-bridge/migrations/0006-decisions-bridge.sh`
(new; `# INTERACTIVE: yes`, bash 3.2 / 80-col / ASCII clean)
**Changes**: Hardcodes exactly the three pinned transformations in fixed emission
order and writes **real YAML frontmatter** on apply.

```bash
#!/usr/bin/env bash
# DESCRIPTION: Standalone reference fixture for the 0117 decisions bridge.
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  harness_emit_transformation key=relates_to \
    path=meta/work/0050-example-a.md anchor=body/relates_to \
    proposed=work-item:0042 predicate_value=ambiguous
  harness_emit_transformation key=parent \
    path=meta/work/0051-example-b.md anchor=body/parent \
    proposed=work-item:0031 predicate_value=ambiguous
  harness_emit_transformation key=relates_to \
    path=meta/work/0052-example-c.md anchor=body/relates_to \
    proposed=work-item:0099 predicate_value=ambiguous
}

migration_evaluate_predicate() { return 0; }      # all rows prompt

migration_validate_edit() {
  [ -n "$5" ] || { harness_reject "empty value not allowed"; return 1; }
}

# accept -> proposed; edit -> user value; skip -> not called.
# Insert `<key>: [<value>]` before the closing `---` of the file's frontmatter,
# AND append a decoupled sentinel record so AC2 can assert the decision/value
# independently of the insert mechanics (mirrors 0002-predicate's applied/log).
migration_apply_decision() {
  local key="$1" path="$2" anchor="$3" decision="$4" value="$5"
  local abs="$PROJECT_ROOT/$path"
  # Fail loudly if the target has no second '---' rather than silently no-op or
  # mis-insert (a malformed frontmatter write is a data-integrity bug, and this
  # fixture is a pattern authors may copy). POSIX-awk-safe: count '---' lines,
  # print the new key immediately before the 2nd, gawk/BSD-awk identical.
  # Pass the value via the environment (ENVIRON[]), NOT awk -v: -v assignments
  # undergo C-style escape processing (a backslash in the value would be
  # transformed), whereas ENVIRON[] is value-transparent on both BSD and gawk.
  key_line="$key: [$value]" awk '
    /^---$/ { n++; if (n == 2) { print ENVIRON["key_line"]; seen2 = 1 } }
    { print }
    END { if (!seen2) exit 3 }
  ' "$abs" >"$abs.tmp" || { harness_reject "no closing --- in $path"; return 1; }
  mv "$abs.tmp" "$abs" ||
    { rm -f "$abs.tmp"; harness_reject "rename failed for $path"; return 1; }
  mkdir -p "$PROJECT_ROOT/.fixture/applied"
  printf '%s\t%s\t%s\t%s\t%s\n' "$key" "$path" "$anchor" "$decision" "$value" \
    >>"$PROJECT_ROOT/.fixture/applied/log"
}

harness_run
```

The awk is POSIX-portable (no gawk-only `gsub`/capture-group/dynamic-regex
features), passes the value via `ENVIRON[]` (value-transparent, no `-v` escape
processing), works identically under BSD awk (macOS CI) and gawk (Linux CI), and
**fails closed** (`exit 3` → `harness_reject` → FAIL frame → runner aborts) if the
closing `---` is absent — and the `mv` is guarded so a rename failure surfaces as
a FAIL rather than a false success. AC2 asserts against the `.fixture/applied/log`
sentinel (decision + value per key) as the primary oracle, with a secondary `grep`
on the real frontmatter — so a fixture-insert bug cannot masquerade as a runner
regression.

**File**: corpus seed files created by the test (not committed as fixtures) —
`meta/work/0050-example-a.md`, `0051-example-b.md`, `0052-example-c.md`, each a
three-line frontmatter stub (`---` / `id: "00NN"` / `---`) under the sandbox
`PROJECT_ROOT`. Drive the fixture through the existing `seed_predicate_sandbox` /
`ACCELERATOR_MIGRATIONS_DIR` plumbing (`test-migrate-interactive.sh:360-369`) so
`PROJECT_ROOT` and `CLAUDE_PLUGIN_ROOT` are exported to the child exactly as the
other interactive fixtures expect (`interactive-harness.sh:421-424`) — no new path
assumptions.

#### 8. Shared byte-identity assert helpers

**File**: `scripts/test-helpers.sh`
**Changes**: The existing `assert_eq` / `assert_file_content_eq` compare values
captured via `$(cat …)` command substitution, which **strips trailing newlines** —
so they cannot pin AC1's exact terminal byte nor prove an AC6 corpus file is truly
byte-identical (a trailing-newline change would slip through). Add two **additive**
`cmp`-based asserts (no change to existing ones, so other suites are unaffected):

```bash
# assert_files_identical <label> <expected_file> <actual_file>
assert_files_identical() {
  if cmp -s "$2" "$3"; then pass "$1"; else
    fail "$1: files differ"; diff "$2" "$3" >&2 || true
  fi
}
# assert_stdout_exact <label> <expected_file> <captured_stdout_file>
# (thin alias around assert_files_identical for the AC1/segmentation cases)
```

The AC1 byte-for-byte `--list` check and every AC6 byte-identical corpus
assertion route through these (capture stdout / checksum the seed to a temp file,
then `cmp`), giving the fail-closed and `--list`-output guarantees real teeth.

#### 9. Tests (TDD)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh` (append a
new `=== Phase: --list dry-emit + decisions bridge (0117) ===` section before the
final `test_summary`; do **not** add a new `test-*.sh` file, so the migrate suite
floor of 4 is unchanged).

- **AC1** — seed the three corpus stubs + run the fixture with `--list` (no
  `FORCE`); capture **stdout only** to a temp file and assert via
  `assert_stdout_exact` (the §8 `cmp`-based helper — **not** `assert_eq`, whose
  `$(…)` capture strips the trailing newline) byte-for-byte against an
  expected-output file (pins the exact terminal newline). Assert exit 0, that
  **stderr is diagnostic-free** on the success path (`assert_stderr_empty`), and
  that each corpus stub is byte-identical to its post-seed copy via
  `assert_files_identical`.
- **AC3** — run `0001-empty-interactive` (emits no transformations) with
  `--list`; assert stdout is exactly `no pending transformations` via
  `assert_stdout_exact` (byte-exact, including the sole terminal newline); exit 0;
  no mutation. Add a sub-case with a genuinely empty `pending_files` set to
  confirm the `--list` branch precedes the "No pending migrations." early exit.
- **AC2** — seed the three corpus stubs + a decisions file `accept\nskip\nedit
  work-item:0100\n`; run the fixture with `ACCELERATOR_MIGRATE_FORCE=1` +
  `ACCELERATOR_MIGRATE_DECISIONS_FILE`. **Primary oracle**: the
  `.fixture/applied/log` sentinel shows `relates_to … accept work-item:0042`,
  no `parent` record, and `relates_to … edit work-item:0100` (edit wins, not
  `work-item:0099`). **Secondary**: `assert_contains` `0050-example-a.md` has
  `relates_to: [work-item:0042]`; `assert_not_contains` `parent` in
  `0051-example-b.md`; `assert_contains` `0052-example-c.md` has
  `relates_to: [work-item:0100]`.
- **Resume-aware `--list`** — pre-seed a session log deciding `relates_to`
  (0050); `--list` then emits only positions for 0051/0052 (already-decided key
  excluded). Locks the resume filter.
- **Multi-migration segmentation** — with a second interactive fixture also
  pending, assert `--list` emits two `# migration <id>` sections with positions
  restarting at 1 in each; and assert the single-migration case emits **no**
  header (AC1 byte-exactness).
- **Dirty-tree bypass** — `--list` on a sandbox with uncommitted `meta/` changes
  and no `FORCE` still exits 0 and prints the lines.
- **`--list` FAIL path** — a fixture whose predicate returns an rc other than
  0/1 for one row: `--list` exits non-zero, stderr names the FAIL key, and no
  `LIST_DONE` is emitted (locks the new error path in the enumeration fork).
- **Unknown-flag rejection** — `bash "$DRIVER" --frobnicate` exits non-zero,
  stderr names the unknown argument.
- **`--help` on stdout** — capture `--help` **without** `2>&1`; assert the env
  var + `--list` line appear on **stdout** and stderr is empty on the help path
  (update the existing help test at `test-migrate-interactive.sh:~1178`, which
  uses `2>&1` and so does not pin the stdout routing AC4 relies on).
- Assert `LIST_ENTRY`/`LIST_DONE` frame counts via the migration protocol log
  (`grep -c $'^LIST_ENTRY\t'` == 3) to lock the wire contract.

### Success Criteria:

#### Automated Verification:

- [x] Migrate suite passes: `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [x] AC1 byte-for-byte `--list` output asserted via `assert_stdout_exact` (the §8 cmp helper, not `assert_eq`), against an expected-output file
- [x] AC3 `no pending transformations` asserted; AC2 real-frontmatter outcomes asserted
- [x] `--help` (now on **stdout**) contains the literal `ACCELERATOR_MIGRATE_DECISIONS_FILE` and a `--list` line: `bash skills/config/migrate/scripts/run-migrations.sh --help | grep -e ACCELERATOR_MIGRATE_DECISIONS_FILE -e -- --list`
- [x] Shell lint/format/bashisms clean: `mise run scripts:check`
- [ ] Full read-only gate green: `mise run check` (deferred to end of implementation — shell-only changes)
- [x] Migrate-suite floor still satisfied (no new `test-*.sh`; count stays 4): `mise run test:integration:migrate`

#### Manual Verification:

- [ ] `--list` output is genuinely parseable (tab-delimited, one entry per line) and the `path:anchor` join reads naturally
- [ ] `--list` leaves no session-log / state files behind (inspect `.accelerator/state/` after a dry run)
- [ ] Frame ordering in the protocol log matches emission order 1..3

---

## Phase 2: Fail-closed validation via a no-mutation dry-apply pass (AC6)

### Overview

Validate the decisions file by running the interactive child in a **dry-apply
mode** — the real decide/validate loop with mutation, session-log writes, and the
`APPLY` round-trip suppressed — *before* the live apply run. Because validation is
the apply loop minus mutation, it exercises `migration_validate_edit` on every
`edit` value (failing fast, before any file is touched) and consumes the decisions
file identically to the live run by construction: there is no separate count `N`
or duplicated verb parse to drift. Fail closed on a rejected edit, an unknown
verb, too few, or too many decisions, naming the offending position, leaving the
corpus unmutated. Satisfies **AC6**; re-confirms **AC2**.

### Why dry-apply rather than an up-front count check

An earlier design enumerated to a count `N` and checked `verbs == N` with a
separate verb classifier. That is **unsound**: a rejected `edit` triggers a
`VALIDATE_ERR` re-prompt that consumes an *extra* decisions-file line
(`interactive-lib.sh:537-561`, verified), so a file passing `verbs == N` can still
exhaust mid-run *after* earlier transformations have mutated the corpus —
violating fail-closed — and it forks the verb grammar into a second definition
that can drift from `read_decision`. Dry-apply removes both: it reuses
`read_decision` unchanged (grammar + CRLF/blank parse shared, not cloned) and
reproduces the exact consumption, so what validates is what applies. In dry-apply
a failed `migration_validate_edit` is a **hard reject** (fail fast), not a
re-prompt, so the live run — reached only for a fully-valid file — cannot
partial-mutate on a bad edit.

### As-built notes (decisions taken during implementation)

- **Dry-apply mirrors the live re-prompt rather than hard-rejecting a bad edit.**
  The plan above first described a failed `migration_validate_edit` as a hard
  reject. As built (confirmed with the author), dry-apply instead replicates the
  live `VALIDATE_ERR` re-prompt: a bad edit followed by a recovery line consumes
  both lines and validates exactly as it applies, so "validates == applies"
  holds and the worked example (driven by a recoverable empty-then-valid edit)
  is reproduced unchanged. A **terminal** bad edit (no recovery line) still fails
  closed — the re-prompt read exhausts the file and the runner reports the
  position and the reject reason. The work item's AC6 (too-few / too-many /
  unknown-verb at positions 3/4/2) is satisfied regardless.
- **Resume drift is modelled in dry-apply** (also confirmed with the author): a
  recorded key whose `proposed_value` has drifted (or whose `migration_verify_applied`
  fails) is re-prompted in the dry pass — consuming a decision exactly as the
  live run — but WITHOUT the `DRIFT` round-trip (which would mutate the session
  log). A cleanly-resumed key consumes none. This keeps consumption identical to
  the live run when a session log has drifted.
- **The dry mode rides on the `MIGRATION_HARNESS_MODE` env var, not a third
  `INIT` field.** An empty `decisions_path` middle field collapses under IFS-tab
  word-splitting and would shift a trailing positional field, so the runner
  signals the mode out-of-band; the `INIT` frame stays byte-identical to the
  live run's two-field form.
- **Dry-apply proto frames go to dedicated logs** (`MIGRATION_PROTOCOL_LOG_DRY`
  / `_DRY_RUNNER`, default empty), and `read_decision` is silenced during the
  dry pass (`DECISIONS_QUIET`), so the validation fork never doubles the live
  run's asserted frame counts or its consumption log.
- **Shared fork scope.** `_interactive_fork`/`_interactive_teardown` are shared
  by the two new dry surfaces (`--list`, dry-apply); the battle-tested live
  `run_interactive_migration` loop is left untouched to avoid behavioural risk to
  the landed 0116/0119 machinery. The shared single-classify source the design
  prioritises lives on the harness side (`_harness_classify_tx`), used by all
  three modes. The dry fork closes its inherited fd 7 in the child so an aborted
  validation delivers a clean EOF instead of hanging the teardown's wait.

### Changes Required:

#### 1. Child-side dry-apply mode

**File**: `scripts/interactive-harness.sh`
**Changes**: Add a `mode="2"` dry-apply branch to `harness_run` (`_harness_dry_apply`,
passed the buffered `TX_LINES` like `_harness_emit_list`). It routes each TX via
the shared `_harness_classify_tx` and, for the `prompt` route, runs the **same**
decide handshake the live loop uses — emit `PROMPT`, receive `DECIDE`, run
`migration_validate_edit` on `edit` values — but **suppresses every side effect**:
no `migration_apply_decision`, no session record, no `RECORDED`/`APPLY`/
`APPLIED_CONFIRM`. On a validation failure it emits `DRY_REJECT key reason`
(a hard stop — *not* `VALIDATE_ERR`, so dry-apply fails fast rather than
re-prompting); on success `DRY_OK key`; resumed/mechanical rows consume no
decision (same as live). It emits `DRY_DONE` at the end. The validate callback is
pure (no mutation), so running it here is safe.

#### 2. Runner-side dry-apply driver

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: New `dry_apply_interactive_migration <path> <id>`, built on the
shared `_interactive_fork` (mode=2) with **fd 9 open on the decisions file exactly
as the live run** so `read_decision` consumes it identically. Its frame handler
drives the decide loop and tracks a position counter:

```bash
dry_apply_interactive_migration() {
  local f="$1" id="$2"
  exec 9<"$ACCELERATOR_MIGRATE_DECISIONS_FILE"; DECISIONS_FD=9; DECISIONS_LINE_NUM=0
  DRY_POS=0
  _interactive_fork "$f" "$id" 2 _dry_handle_frame; local rc=$?
  # too-many: decisions remain on fd 9 after DRY_DONE
  if [ "$rc" -eq 0 ] && _decisions_have_more <&9; then
    echo "Error: decisions file has a surplus decision at position" \
      "$((DRY_POS + 1)) (only $DRY_POS transformation(s) require one)." >&2
    rc=1
  fi
  exec 9<&-
  return "$rc"
}
_dry_handle_frame() {                  # $1 = type, $2.. = unescaped fields
  case "$1" in
    PROMPT)
      DRY_POS=$((DRY_POS + 1))
      if ! read_decision; then          # rc 1 = exhausted, too few
        echo "Error: decisions file is missing a decision for position" \
          "$DRY_POS ($2)." >&2; return 1
      fi
      case "$DECIDE_OUTCOME" in          # reuse read_decision's parse; reject unknown
        accept | skip | edit) : ;;
        *) echo "Error: decisions file position $DRY_POS: unknown verb" \
             "'$DECIDE_OUTCOME' (expected accept | skip | edit <value>)." >&2
           return 1 ;;
      esac
      _dry_send_decide ;;               # DECIDE to the child (no APPLY ever follows)
    DRY_REJECT)                          # edit rejected by migration_validate_edit
      echo "Error: decisions file position $DRY_POS: $3 (key $2)." >&2; return 1 ;;
    DRY_OK) : ;;
    DRY_DONE) return "$_FORK_STOP" ;;   # stop the fork loop cleanly (see below)
    FAIL) echo "[$id] $2" >&2; return 1 ;;
  esac
}

# Relay the decision read by read_decision to the child (mirrors the live
# PROMPT arm's DECIDE emission; no APPLY ever follows in dry mode).
_dry_send_decide() {
  printf 'DECIDE\t%s\t%s\n' \
    "$(escape_field "$DECIDE_OUTCOME")" "$(escape_field "$DECIDE_VALUE")" >&"$mig_in"
}

# Surplus look-ahead: reuse read_decision's OWN blank/CRLF-skipping (do NOT
# re-implement it, or a trailing blank line false-positives a surplus). A
# successful read of a further non-blank verb means a genuine surplus.
_decisions_have_more() { read_decision; }
```

`_FORK_STOP` is a named sentinel for the handler/`_interactive_fork` contract: a
handler returns **0** to continue, **1** (or any error) to abort the run, and
`_FORK_STOP` to stop the loop *cleanly* on a `*_DONE` frame. `_interactive_fork`
treats `_FORK_STOP` as success and any other non-zero as failure. Naming it avoids
a bare magic `10` repeated across the enum and dry handlers. Declare it
re-source-safe (`interactive-lib.sh` is sourced by the driver and per-test), e.g.
`[ -n "${_FORK_STOP:-}" ] || readonly _FORK_STOP=10`, so a second source does not
abort on a `readonly` re-assignment. Pick a value clear of common exit codes and
keep handler bodies from leaking an unguarded sub-command `$?` into the return
(the in-band-signalling tradeoff of a return-code sentinel).

This reproduces AC6's exact positions from real consumption: (c) unknown verb at
line 2 → rejected at position 2; (a) two verbs for three prompts → exhausted at
position 3; (b) four verbs → surplus at position 4; a rejected `edit` value →
named at its position **before any file is written**. `_decisions_have_more`
reusing `read_decision` keeps the too-many check on the same blank/CRLF semantics
as consumption, so a valid file with a trailing blank line is **not** mis-flagged
as surplus. The dry handshake is strictly serial per transformation (one
PROMPT → one DECIDE → one DRY_OK/DRY_REJECT before the next PROMPT), so `DRY_POS`
always names the in-flight prompt.

#### 3. Driver wiring

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: After sourcing `interactive-lib.sh` and computing pending (and after
the Phase 1 `--list` branch), when `ACCELERATOR_MIGRATE_DECISIONS_FILE` is set,
dry-apply the pending interactive migration before the live apply loop:

```bash
if [ -n "$ACCELERATOR_MIGRATE_DECISIONS_FILE" ]; then
  for f in "${pending_files[@]+"${pending_files[@]}"}"; do
    id="$(basename "$f" .sh)"
    if is_interactive_migration "$f"; then
      dry_apply_interactive_migration "$f" "$id" || exit 1
    fi
  done
fi
# ... live apply loop unchanged ...
```

A non-zero return or `FAIL` from `dry_apply_interactive_migration` aborts with
`exit 1` **before** the live apply loop, so a validation/fork failure never falls
through to a mutating run. `read_decision`'s silent `*)` pass-through is left
intact but is now unreachable for the decisions-file path (dry-apply rejects an
unknown `DECIDE_OUTCOME` first).

**Composition with 0119's guarded resume.** This dry-apply branch sits *after* the
pre-flight, which 0119 now lets proceed (`RESUME=1`) for a `--decisions-file`
resume over the run's own owned dirt — so the realistic resume path is pre-flight
guarded-resume → dry-apply → live apply, with no `FORCE`. The dry-apply fork
honours the same READY resume-rebuild as the live run (Phase 1 §3), so it validates
only the **remaining** (undecided) transformations against the decisions file —
already-decided keys are excluded identically on both passes. The "Resume +
validate" test below locks this.

#### 4. Tests (TDD)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`

Each malformed case asserts non-zero exit, the named position on stderr, every
seed file **byte-identical** to its post-seed checksum (`cmp`/checksum, not
grep-for-absence), and no session-log / applied entry under `.accelerator/state/`.

- **AC6(a) too few** — `accept\nskip\n` (2 verbs, 3 prompts): names position 3.
- **AC6(b) too many** — `accept\nskip\nedit x\naccept\n` (4 verbs): surplus
  position 4.
- **AC6(c) unknown verb** — `accept\nfrobnicate\nedit x\n`: names position 2.
- **Rejected edit caught up front (the Decision-1 fail-fast case)** — a decisions
  file whose position-3 `edit` value fails the fixture's `migration_validate_edit`
  (empty value): non-zero exit naming position 3 with the reject reason, and
  `0050-example-a.md` (position 1, an accept) **byte-identical** — proving
  dry-apply fails before the first mutation.
- **CRLF + blank-line tolerance** — a valid file with CRLF endings and an
  interspersed blank line passes dry-apply and applies (locks the shared
  `read_decision` parse).
- **Bare `edit` vs `edit <value>`** — both classify as the `edit` verb (not
  "unknown"); assert the full downstream path: a bare `edit` (empty value) reaches
  `DRY_REJECT` from the fixture's empty-value `migration_validate_edit` (named
  position), while `edit <value>` reaches `DRY_OK`.
- **Resume + validate** — pre-seed a session log deciding position 1; a
  correctly-sized decisions file for the remaining positions passes dry-apply and
  applies (the dry pass honours the same resume exclusion as `--list` and live).
- **Fork failure fails closed** — inject a `FAIL`/fork condition; the run exits
  non-zero with the corpus unmutated rather than proceeding to apply.
- **Regression / AC2 through the gate** — the valid `accept\nskip\nedit
  work-item:0100\n` file passes dry-apply and applies (re-assert the Phase 1 AC2
  outcomes via the sentinel log), proving the gate doesn't reject good input.
  Additionally assert the dry-apply protocol log's `DRY_OK`/`DRY_REJECT` sequence
  (three prompt routes) matches the live run's `PROMPT`/`RECORDED` sequence — this
  locks the two enumerations to the same shape, catching any non-determinism
  between the dry and live forks.

### Success Criteria:

#### Automated Verification:

- [x] AC6(a/b/c) + rejected-edit + CRLF/blank + bare-edit + resume + fork-failure cases pass: `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [x] Each malformed case exits non-zero, names the correct position, and leaves every seed file byte-identical (cmp/checksum asserted) with a clean `.accelerator/state/`
- [x] A rejected `edit` value fails before any mutation (position-1 file byte-identical)
- [x] The valid decisions file still applies (AC2 regression green)
- [x] Shell lint/format/bashisms clean: `mise run scripts:check`
- [ ] Full read-only gate green: `mise run check` (deferred to end — shell-only changes)

#### Manual Verification:

- [ ] Error messages are actionable (name the position, the key, and the expected verbs / reject reason)
- [ ] On a failed validation, `.accelerator/state/` shows no partial session log / applied entry for the fixture

---

## Phase 3: `SKILL.md` invoker contract + ADR-0037 judgment (AC5)

### Overview

Document the invoker side of the interactive contract and record the ADR-0037
judgment. Documentation-only — fully independent and mergeable on its own.
Satisfies **AC5** and resolves the work item's Open Question.

### Changes Required:

#### 1. New invoker-contract section

**File**: `skills/config/migrate/SKILL.md`
**Changes**: Insert a new top-level `##` section between the worked example
(`:214`) and `## Executing the migration` (`:216`), keeping author-side and
invoker-side adjacent. Match the doc's existing terminology ("transformations",
"emission order", verbs `accept`/`edit`/`skip`). It must contain, each
confirmable by string search:

- (a) the literal phrase **`list → decide → write → resume`** (and the four steps
  named in order);
- (b) the verb tokens `accept`, `skip`, `edit` and the phrase **"matched by
  emission order"**;
- (c) a link to **0116** for the no-input/structured-stall outcome (reference
  `meta/work/0116-structured-stall-on-no-decision-input.md` and the stable
  marker `MIGRATION STALLED: no decision input available`);
- (d) the literal **`ACCELERATOR_MIGRATE_DECISIONS_FILE`** with a pointer to
  where it is discoverable (**`--help`**);
- (e) the **fail-closed** behaviour: a rejected `edit`, an unknown verb, or a
  count mismatch → non-zero exit, corpus unmutated on validation failure (and the
  apply-time partial-mutation caveat → VCS revert).

Sketch:

```markdown
## Answering prompts as an agent (the invoker contract)

When `/accelerator:migrate` runs without a human at a terminal, an agent answers
the interactive prompts with a **decisions file**, following `list → decide →
write → resume`:

In practice the agent first runs the migration and hits the **structured stall**,
which names the exact decisions-file path — including the migration `<id>` —
(`.accelerator/state/migrations-<id>-decisions.txt`) and a copy-pasteable resume
command. `--list` is then the step that **reveals the proposed values** (which the
stall does not show), so the realistic order is run → stall (learn the `<id>` and
path) → `--list` (see proposed values) → write → resume. The `<id>` comes from the
stall/preview, not from `--list` output.

1. **list** — `bash …/run-migrations.sh --list` dry-emits every pending
   interactive transformation, one tab-delimited line each, without mutating the
   corpus:

   ```
   <position>\t<key>\t<proposed>\t<path>:<field>
   ```

   (Fields are separated by a literal TAB, shown as `\t` here; the same column
   vocabulary — `<path>:<field>` — is used in `--help`.)

   Proposed values are revealed only here, so list before deciding. When parsing,
   **skip lines beginning with `#`**: with more than one pending interactive
   migration the output is segmented by a `# migration <id>` header and
   `<position>` restarts at 1 per migration. Resume each `# migration <id>`
   section separately with its own decisions file (a single multi-migration
   decisions file is not yet supported; `--list` prints a stderr note when more
   than one is pending).
2. **decide** — choose a verb per transformation: `accept`, `skip`, or
   `edit <value>`.
3. **write** — write one verb per line to a decisions file, **matched by
   emission order** (line *i* answers list position *i*; skipped/mechanical
   transformations consume no line). Create the file yourself at a path that
   exists and is readable — the stall message points at
   `.accelerator/state/migrations-<id>-decisions.txt`; do not overwrite existing
   `migrations-<id>-*` state files. For example:

   ```bash
   printf 'accept\nskip\nedit work-item:0100\n' \
     > .accelerator/state/migrations-<id>-decisions.txt
   ```
4. **resume** — re-run with `--decisions-file <path>` (or the equivalent
   `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var, discoverable via `--help`). The
   stall's copy-pasteable command is exactly this bare form — **no
   `ACCELERATOR_MIGRATE_FORCE=1` is needed** in the normal case. A partial
   interactive run dirties the tree only with files this run owns (the interactive
   session log, plus any frontmatter already written), and the **guarded resume**
   shipped in 0119 lets the re-run proceed over that own output without `FORCE`
   when the base revision is unchanged, printing a one-line affordance listing the
   owned paths being resumed over. `FORCE` is required **only** when the pre-flight
   refuses — i.e. the tree carries dirt this run does *not* own (foreign changes,
   or you have committed since the partial run so the base revision moved). In that
   case, re-run once without `FORCE` first to read the refusal / in-flight-session
   guidance, confirm via `jj status`/`git status` that the dirty paths really are
   this migration's own, and only then add `ACCELERATOR_MIGRATE_FORCE=1`. (The
   guarded-resume behaviour is owned by
   `meta/work/0119-resume-safe-partial-migration-failure.md`, now landed.)

The driver **validates the decisions file up front (a no-mutation dry-apply pass)
and fails closed**: a rejected `edit` value, an unknown verb, or a count mismatch
(too few or too many verbs) exits non-zero, names the offending position, and
leaves the corpus **unmutated** — validation never partially applies. Once
validation passes and the live apply begins, transformations are applied in order
without rollback, so an apply-time failure can leave a partial corpus; recover
with VCS revert, then re-run — 0119's guarded resume replays the run's own partial
output without `FORCE` when the base revision is unchanged.

When no decision input is available at all, the run emits the structured stall
(`MIGRATION STALLED: no decision input available`) and stops without further
mutation — see `meta/work/0116-structured-stall-on-no-decision-input.md`.

This contract is scoped to a single pending interactive migration (the realistic
case); decisions files are consumed per migration.
```

#### 2. Author-contract: callbacks may be invoked more than once per run

**File**: `skills/config/migrate/SKILL.md`
**Changes**: Amend the existing author-facing "Optional interactive contract"
section (`:89-214`, alongside the `migration_*` callback table at `:95-108`) to
state that a `--decisions-file` run now invokes the author callbacks **more than
once**: `migration_emit_transformations` and `migration_evaluate_predicate` run
during `--list` enumeration and again during the dry-apply pass before the live
run, and `migration_validate_edit` runs during dry-apply and again at live apply.
They must therefore be **deterministic and side-effect-free**, and in particular
`migration_validate_edit` **must be a pure function of its arguments** — it must
**not** read corpus state that an earlier transformation in the same run could
mutate. (If it did, dry-apply could pass against the unmutated corpus while the
live run fails validation at a later position after earlier files mutated —
re-opening the partial-mutation hole dry-apply closes.) A validator that depends
on corpus state is **unsupported**; this is documented rather than enforced
(policing it is an author-error class, not a framework guarantee). The existing
contract already documents the predicate's side-effects as "none", so this makes
the implied purity explicit and extends it to the validator + the
run-more-than-once reality.

#### 3. Cross-reference

**File**: `skills/config/migrate/SKILL.md`
**Changes**: Add the 0116 work item to the `## Cross-references` list (`:226-232`)
so the stall is discoverable from there too.

#### 4. Record the ADR-0037 judgment

**File**: this plan + the Phase 3 commit message (and resolve the work item's
Open Question on update).
**Changes**: Record explicitly that the invoker bridge is an **implementation
detail under ADR-0037, not an amendment**: it adds no new control verb, display
element, or resumability guarantee, and ADR-0037 is neutral on how the runner is
invoked, so the recursive-supplement clause is not tripped. No edit to the
(immutable, accepted) ADR.

#### 5. Test (string-search assertions)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: Add a small section that `grep`s `SKILL.md` for each AC5 element
(a–e). Resolve `SKILL.md` relative to `PLUGIN_ROOT`. No new `test-*.sh` file (keep
the migrate floor at 4).

```bash
SKILL_MD="$PLUGIN_ROOT/skills/config/migrate/SKILL.md"
assert_contains "AC5(a) list->decide->write->resume" \
  "$(cat "$SKILL_MD")" "list → decide → write → resume"
assert_contains "AC5(b) matched by emission order" \
  "$(cat "$SKILL_MD")" "matched by emission order"
# ... accept/skip/edit, 0116 link, ACCELERATOR_MIGRATE_DECISIONS_FILE + --help,
#     fail-closed wording ...
```

### Success Criteria:

#### Automated Verification:

- [ ] AC5 string-search assertions pass: `bash skills/config/migrate/scripts/test-migrate-interactive.sh`
- [ ] `grep -F 'list → decide → write → resume' skills/config/migrate/SKILL.md` succeeds
- [ ] `grep -F 'matched by emission order' skills/config/migrate/SKILL.md` succeeds
- [ ] `grep -F 'ACCELERATOR_MIGRATE_DECISIONS_FILE' skills/config/migrate/SKILL.md` succeeds and 0116 is linked
- [ ] Markdown/format gate green: `mise run check`

#### Manual Verification:

- [ ] The invoker section reads coherently alongside the author-facing contract and the worked example
- [ ] The ADR-0037 judgment is captured in the commit message / work item resolution and reads correctly against the recursive-supplement clause

---

## Testing Strategy

### Unit / suite tests (all in `test-migrate-interactive.sh`):

- `--list` byte-for-byte output (AC1, stderr diagnostic-free), empty case (AC3),
  resume-aware exclusion, multi-migration segmentation, dirty-tree bypass,
  unknown-flag rejection, `--list` FAIL path, `LIST_ENTRY`/`LIST_DONE` frame
  counts.
- Decisions-file apply outcomes (AC2): asserted against the fixture's
  `.fixture/applied/log` sentinel (primary oracle) plus a secondary frontmatter
  `grep` — accept/skip/edit.
- Fail-closed dry-apply (AC6): too few / too many / unknown verb / rejected edit /
  fork failure, each asserting the named position and a **byte-identical** corpus.
- `SKILL.md` string-search assertions (AC5).

### Key edge cases:

- Resumed keys excluded from `--list` and from dry-apply consumption alike (shared
  `_harness_classify_tx` + the shared READY resume rebuild).
- `edit <value>` vs bare `edit` both classify as valid verbs (not "unknown") —
  via the shared `read_decision`, not a forked parser.
- CRLF line endings and blank lines tolerated in the decisions file — dry-apply
  reuses `read_decision`, so the parse is shared, not re-implemented.
- A rejected `edit` value fails fast in dry-apply, before any mutation (the live
  re-prompt path is never reached for a decisions-file run).
- `--list` produces zero stray bytes on stdout beyond the entry lines / section
  headers / the empty sentinel; fields with a literal tab/newline fail closed.

### Manual testing steps:

1. Run `--list` against the fixture; confirm tab-delimited, ordered output and a
   clean `.accelerator/state/`.
2. Write a decisions file from the list, resume with `--decisions-file`, confirm
   the three frontmatter outcomes.
3. Corrupt the decisions file (drop a line; add a junk verb); confirm the
   fail-closed message names the right position and nothing was written.

## Performance Considerations

Negligible. `--list` (one enumeration fork) and a decisions-file resume (one
dry-apply fork before the live run) each fork the interactive child once more,
bounded by the corpus size; the projected first real consumer is ~140
transformations. The dry-apply pass runs `migration_emit_transformations` and the
predicate a second time, so they must be deterministic and side-effect-free across
two invocations (already implied by the buffer-once design; stated in the harness
author contract). O(N) linear scans throughout, consistent with the existing
harness.

## Migration Notes

No data migration. The wire-protocol change is additive and backward-compatible
(optional third `INIT` field; five new frames — `LIST_ENTRY`/`LIST_DONE`/
`DRY_OK`/`DRY_REJECT`/`DRY_DONE` — that a normal run never emits; the normal-run
INIT stays two-field). Existing mechanical and interactive migrations are
unaffected.

Two behavioural changes to existing flows, both deliberate:

- **Strict unknown-flag rejection** in the driver. Before merging, grep the skill
  bodies, hooks, and `tasks/` for every `run-migrations.sh` invocation and confirm
  each passes only `--skip`/`--unskip`/`--decisions-file`/`--list`/`--help`
  (in-tree the only callers are the skill, which passes no positional args, and
  the test suite); record the audit result here.
- **`--help` now prints to stdout** (was stderr) for the explicit `--help`/`-h`
  path; usage-on-error stays on stderr.

Add a CHANGELOG note — landed under the same `[Unreleased]` migrate-driver bullet
as 0116's `--decisions-file`/stall entries, so the change is framed by the contract
it completes — that an unrecognised driver flag **or positional argument** now
exits non-zero (previously silently ignored) and that `--help` prints to stdout.

## References

- Original work item: `meta/work/0117-agent-decisions-bridge-and-invoker-contract.md`
- Research: `meta/research/codebase/2026-06-21-0117-agent-decisions-bridge-and-invoker-contract.md`
- Parent / siblings: `meta/work/0115-make-interactive-migrations-satisfiable-under-agent-invocation.md`,
  `meta/work/0116-structured-stall-on-no-decision-input.md` (landed),
  `meta/work/0118-reconcile-0007-backfill-sentinel-with-validator.md` (fix C; owns AC7),
  `meta/work/0119-resume-safe-partial-migration-failure.md` (landed; owns the
  manifest-based guarded resume that lets a `--decisions-file` resume proceed over
  the run's own owned dirt without `FORCE`)
- ADR: `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
- Driver flag block + `--help` + decisions-file validation: `skills/config/migrate/scripts/run-migrations.sh:34-98`
- Guarded resume / dirty-tree pre-flight (0119, landed): `skills/config/migrate/scripts/run-migrations.sh:252-358`
- Child TX buffering + predicate filter: `scripts/interactive-harness.sh:272-351`
- PROMPT relay + `path:anchor` join: `skills/config/migrate/scripts/interactive-lib.sh:204,213,509-515`
- Protocol field contract: `scripts/interactive-protocol.sh:14-39`
- Author-side contract / execution guidance: `skills/config/migrate/SKILL.md:89-224`
- Fixture + test patterns: `skills/config/migrate/scripts/test-migrate-interactive.sh:360-404`,
  `skills/config/migrate/scripts/test-fixtures/interactive/0002-predicate/migrations/0002-predicate.sh`
- Suite discovery / floor: `tasks/test/integration.py:138-147`
