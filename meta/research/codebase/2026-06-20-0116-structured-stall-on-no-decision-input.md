---
type: codebase-research
id: "2026-06-20-0116-structured-stall-on-no-decision-input"
title: "Research: Structured Stall on No Decision Input (work item 0116)"
date: "2026-06-20T15:40:00+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0116"
parent: "work-item:0116"
topic: "Structured Stall on No Decision Input"
tags: [research, codebase, migrate, interactive-migration, agent-invocation, interactive-lib, run-migrations]
revision: "1fd8ceeccfa2af3146dc8034973152919b3ad0c1"
repository: "accelerator"
last_updated: "2026-06-20T15:40:00+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Added 'Interaction Model — how 0116 and 0117 couple' section clarifying the batch list→decide→write→resume flow vs live back-and-forth, and the decisions-file bulk-response semantics"
schema_version: 1
---

# Research: Structured Stall on No Decision Input (work item 0116)

**Date**: 2026-06-20 15:40 UTC
**Author**: Toby Clemson
**Git Commit**: 1fd8ceeccfa2af3146dc8034973152919b3ad0c1
**Branch**: (detached HEAD)
**Repository**: accelerator

## Research Question

For work item `meta/work/0116-structured-stall-on-no-decision-input.md`: replace
the two opaque `read_decision()` failure aborts (`failed to obtain decision` /
`failed to obtain re-decision`) with a structured, actionable stall that names
the pending decision key(s) and prints an exact resume command. What is the
shape of the code that must change, where exactly are the emit sites, what
variables are in scope at each, what does the "resume command" actually look
like in the codebase today, and how is this path tested?

## Summary

The change is genuinely small and localised to **one file**:
`skills/config/migrate/scripts/interactive-lib.sh`. There are exactly two
two-line `echo … >&2; … return 1` abort blocks to convert (PROMPT handler at
`:449-453`, VALIDATE_ERR re-prompt at `:484-488`), plus the supporting
`read_decision()` input-source chain at `:238-288` that decides when no input
channel exists. The migration id, the current decision key, and the
decisions-file env-var path are all in scope at both emit sites, so the
firm acceptance criteria (name the current key + print a resume command
carrying the id and a non-empty `ACCELERATOR_MIGRATE_DECISIONS_FILE` path) are
straightforwardly achievable.

**Three findings materially shape the implementation:**

1. **The work item's resume-command requirement contradicts the code it cites.**
   The work item says the stall's resume command must "match the in-flight
   session-log resume hint produced by the pre-flight check
   (`run-migrations.sh:90-132`)". But that hint does **not** set
   `ACCELERATOR_MIGRATE_DECISIONS_FILE` — it prints `To resume: re-run
   /accelerator:migrate …` and a `To discard: rm <path>` line. The
   acceptance criteria, meanwhile, require the resume command to *assign*
   `ACCELERATOR_MIGRATE_DECISIONS_FILE` a non-empty path. **No code emits that
   shape today** — 0116 introduces it. The implementer must treat the "match
   the pre-flight hint" sentence as aspirational/shape-borrowing, not literal:
   the AC is the binding contract. (The 0116 review already landed on the AC
   form as the verifiable contract; see Historical Context.)

2. **There is no accumulator of all pending decision keys.** Only the single
   most-recent prompt is cached (`LAST_PROMPT_KEY` and siblings). Listing "all
   accumulated keys" would require new state. This is why the work item demotes
   all-keys to a best-effort note and makes only the *current* key a firm
   criterion — the code confirms that was the right call.

3. **`ACCELERATOR_MIGRATE_DECISIONS_FILE` is deliberately a test-only seam.**
   `run-migrations.sh:14-18` documents it as never user-facing. The stall's
   resume command will therefore be the *first* user-facing reference to it.
   This is a known wrinkle: fix A (0117) is what formally promotes the env var
   to a documented interface, and an agent cannot populate the file correctly
   without first seeing the prompts. So 0116's resume command is an actionable
   breadcrumb for relay, not a self-sufficient resume mechanism — exactly the
   "mitigation, not fix" framing in the source research.

The non-regression criterion (AC3) is well-supported: the existing interactive
suite drives every happy path through `ACCELERATOR_MIGRATE_DECISIONS_FILE` and
asserts exit status + JSONL line counts, and **no existing test asserts on
either baseline string**, so converting them breaks nothing and the new
assertions are purely additive.

## Detailed Findings

### `read_decision()` — input-source selection (the no-input detector)

`interactive-lib.sh:238-288` (header comment at `:236-237`). The function reads
one decision line, choosing its source, then parses it into the output globals
`DECIDE_OUTCOME` / `DECIDE_VALUE`.

Source-selection chain:

- **Decisions-file branch** (`:240-257`): taken when env var
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` is set. Reads from fd `DECISIONS_FD`
  (literal `9`, opened via `exec 9<…` at `:313`, set at `:314`). Normalises
  CRLF (`line="${line%$'\r'}"`, `:247`), skips blank lines, increments
  `DECISIONS_LINE_NUM`. On exhaustion writes `[interactive] decisions file
  exhausted` to **stderr** and `return 1` (`:242-243`, `:251-252`).
- **Interactive/TTY branch** (`:258-264`): the `else`. `if [ -t 0 ]` →
  `IFS= read -r line </dev/tty || return 1` (`:260`); otherwise bare fd 0:
  `IFS= read -r line || return 1` (`:262`).

```sh
  else
    if [ -t 0 ]; then
      IFS= read -r line </dev/tty || return 1
    else
      IFS= read -r line || return 1
    fi
  fi
```

Return contract: `return 1` on any read failure (no `DECIDE_*` reset); implicit
`0` on success after the parse `case` (`:266-287`). Writes **nothing to
stdout** — all diagnostics go to stderr.

**The no-input case the work item targets** is the bare-fd-0 sub-branch
(`:262`) reaching EOF when `ACCELERATOR_MIGRATE_DECISIONS_FILE` is unset and
stdin is not a TTY. `read_decision` returns 1 here exactly as it does on a
genuine read error — there is no distinguishing signal inside the function, so
the no-input detection (no decisions file ∧ not a TTY ∧ EOF) is most cleanly
asserted **at the call sites** where the abort messages live, or hoisted into
`read_decision` as an explicit pre-read check.

### Emit site 1 — PROMPT frame handler (`failed to obtain decision`)

PROMPT case spans `:442-470`; the abort block is `:449-453`:

```sh
        if ! read_decision; then
          echo "[$id] failed to obtain decision for $p_key" >&2
          exec 7>&-
          return 1
        fi
```

In scope:
- **Current key**: `p_key` — `local`, set `:443` from `${fields[0]:-}` (the
  PROMPT frame's first field). Full PROMPT locals (`:443-446`): `p_key`,
  `p_path`, `p_anchor`, `p_proposed`, `p_predicate`, `p_extras`, `p_display`.
- **Migration id**: `id` — function-local parameter of
  `run_interactive_migration` (`local f="$1" id="$2"` at `:292`).
- **Decisions-file path**: only as the env var
  `${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}` (no local copy). `DECISIONS_LINE_NUM`
  also in scope.
- The `LAST_PROMPT_*` cache (`:455-461`) is set **after** this block, so it
  holds the *previous* prompt here (unset on first prompt).

Abort mechanism: `exec 7>&-` closes the runner→migration FIFO write end
(delivers EOF to the migration's stdin), then `return 1` out of
`run_interactive_migration`. **No `exit`** — control returns to the sourcing
`run-migrations.sh`. There is no named abort helper; this is the pattern to
preserve so the stall remains a non-zero halt with the same teardown.

### Emit site 2 — VALIDATE_ERR re-prompt (`failed to obtain re-decision`)

VALIDATE_ERR case spans `:471-497`; the abort block is `:484-488`:

```sh
        if ! read_decision; then
          echo "[$id] failed to obtain re-decision for $LAST_PROMPT_KEY" >&2
          exec 7>&-
          return 1
        fi
```

In scope:
- **Current key**: `LAST_PROMPT_KEY` — a **global** (no `local`), set `:455`
  from `p_key`. The PROMPT locals (`p_key` etc.) are **not** in scope here (they
  were `local` to a prior loop iteration). The VALIDATE_ERR path reuses the
  cached `LAST_PROMPT_*` set (`:455-461`).
- **Migration id**: `id` (same local as above).
- Also: `LAST_PROMPT_HAD_VALIDATE_ERR` (`:475`), `PROMPT_INDEX` (`:479`), env
  decisions-file path.

Abort mechanism identical: `exec 7>&-` (`:486`) then `return 1` (`:487`).

**Implication for the implementer**: the two sites name the current key through
*different variables* (`p_key` vs `LAST_PROMPT_KEY`). A shared stall-emitting
helper should take the key (and id) as arguments rather than reading a single
global.

### No all-keys accumulator exists

The only retained prompt state is the single-most-recent cache `LAST_PROMPT_KEY`
+ siblings (`:455-461`). Keys are *persisted* one-at-a-time to the JSONL session
log via `write_session_record` on each RECORDED frame (`:498-512`), but there is
no in-memory array of pending keys. Listing all accumulated keys would require
either a new array appended in the PROMPT case or re-reading `$SESSION_LOG` —
hence "best-effort, out of scope for the firm AC" in the work item is correct.

### No existing structured-error helper

There is no general multi-line/structured stderr emitter in `interactive-lib.sh`
or `interactive-protocol.sh`. Error reporting is ad-hoc `echo "[$id] …" >&2`.
Two existing style precedents if a framed block is wanted:
- `render_prompt()` (`:201-234`) — a grouped `{ … } >&"$USER_OUT_FD"` box with
  box-drawing rules (`:210-232`); writes to the user-output fd, not stderr.
- The unexpected-exit reporter (`:566-572`) — multiple stderr lines each
  prefixed `[$id]` (`tail -n 20 … | sed "s/^/[$id]   /" >&2`). This is the
  natural model for a stderr stall block.

`USER_OUT_FD` is fd 1 when stdout is a TTY, else fd 2 (chosen `:318-323`).

### The pre-flight resume hint — what it actually prints (`run-migrations.sh`)

This is the finding that corrects the work item's premise. The pre-flight check
(`run-migrations.sh:89-134`, inside the clean-tree guard `:67-141`, skipped when
`ACCELERATOR_MIGRATE_FORCE` is set) detects dirty paths matching the session-log
pattern `.accelerator/state/migrations-<id>-session.jsonl` and prints:

```
Found in-flight interactive migration session(s):
  <path>  (<N> decisions recorded)

To resume: re-run /accelerator:migrate (the session log is read on entry;
you will be prompted only for un-decided transformations).
To discard: rm <path>  (loses <N> decisions)

If the above does not match what you expected, run `jj status`
to see all uncommitted changes before proceeding.
```

It does **not** assign `ACCELERATOR_MIGRATE_DECISIONS_FILE`, and the migration
id appears only embedded in `<path>`. So:

- The work item's Requirements/Technical Notes say "match the in-flight
  session-log resume hint (`run-migrations.sh:90-132`) — i.e. an invocation that
  re-runs the driver with `ACCELERATOR_MIGRATE_DECISIONS_FILE` set". That
  conflates two different things; the cited hint re-runs `/accelerator:migrate`,
  not the driver with the env var.
- The **acceptance criteria** (binding) require the resume command to contain a
  non-empty `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>` assignment, the failing
  migration's id as a literal substring, and `run-migrations.sh` as the invoked
  driver. **This shape exists nowhere in the codebase yet** — 0116 defines it.

Recommendation captured in Code References below: build the stall's resume line
fresh to satisfy the AC, and do not expect to reuse the pre-flight `printf`
lines verbatim.

### `ACCELERATOR_MIGRATE_DECISIONS_FILE` semantics

- **Declared/validated**: `run-migrations.sh:14-37` — defaulted empty and
  exported (`:19-20`); if non-empty must exist, be a non-directory, and be
  readable, else exit 1 (`:21-37`). Comment (`:14-18`) explicitly: "test-only
  mechanism … Never documented in --help or any user-facing banner."
- **Consumed**: opened on fd 9 (`interactive-lib.sh:312-316`), read in
  `read_decision` (`:240-257`), forwarded to the child migration as the INIT
  frame's second field (`:367-369`).
- **File format**: newline-delimited decision lines, one per prompt:
  `accept` / `skip` / `edit <value>` / `edit` (anything else → verbatim outcome
  with empty value). CRLF tolerated, blank lines skipped. This is **distinct**
  from the JSONL session log (`write_session_record`, `:153-189`), which is the
  resumable artifact the pre-flight hint detects.

### How id and globals flow runner → interactive layer

- `run-migrations.sh` sources `interactive-lib.sh` at `:250`; it in turn sources
  `interactive-protocol.sh`.
- Dependencies flow via **globals not args**: `interactive-lib.sh` header
  (`:11-16`) requires `$PROJECT_ROOT`, `$PLUGIN_ROOT`, `$STATE_FILE` to be set;
  `PROJECT_ROOT` is exported before dispatch (`run-migrations.sh:257`).
- id derivation: `id="$(basename "$f" .sh)"` (`run-migrations.sh:207/235/255`),
  passed positionally as `$2` to `run_interactive_migration` → local `id`
  (`interactive-lib.sh:292`). Used to build deterministic state paths under
  `.accelerator/state/` (resume state `:295`, stderr log `:296`, session log
  `:302`, FIFOs `:341-342`) and exported to the child as `MIGRATION_ID` (`:355`).

### Testing — `test-migrate-interactive.sh`

`skills/config/migrate/scripts/test-migrate-interactive.sh` (1115 lines),
standalone `set -euo pipefail` bash, sources `scripts/test-helpers.sh` (`:9`)
for `PASS`/`FAIL`/`SKIP` counters and assertions (`assert_eq`, `assert_neq`,
`assert_contains`, `assert_not_contains`, `assert_file_exists`, `test_summary`).
Tests are inline, grouped by `=== Phase N … ===` banners. Driver under test:
`DRIVER="$SCRIPT_DIR/run-migrations.sh"` (`:11`).

Key facts for 0116:

- **Decisions are supplied only via `ACCELERATOR_MIGRATE_DECISIONS_FILE`** —
  never via stdin pipe or fake TTY. The `else` (TTY/bare-fd-0) branch of
  `read_decision` is therefore **never exercised by any test**. The genuine
  no-input path (var unset, non-TTY, EOF) is **uncovered** — this is the gap
  0116 + 0120 fill.
- **No test asserts on `failed to obtain decision` / `failed to obtain
  re-decision`** (a grep matches only the two `interactive-lib.sh` source
  lines). Converting them breaks no assertion; the stall assertions are
  additive.
- **AC3 non-regression anchors** already exist: the `0002-predicate`
  accept/edit/skip cases (`:378-666`) and the doc-example determinism gate
  (`:1069-1111`) assert exit 0 + exact JSONL line counts via
  `COUNT=$(wc -l <"$LOG" | tr -d ' ')`. These are the tests AC3 says must pass
  unchanged.
- **Primary PROMPT fixture to mirror**: `test-fixtures/interactive/0002-predicate/
  migrations/0002-predicate.sh` — reads `$PROJECT_ROOT/.fixture/transformations`,
  routes a row to a PROMPT when `predicate_value=ambiguous`, ends with
  `harness_run`. The seeder is `seed_predicate_sandbox` (`:360-369`).
- **Suggested new test for the stall**: a PROMPT-emitting fixture (mirror
  `0002-predicate`) run with **no `ACCELERATOR_MIGRATE_DECISIONS_FILE` and a
  non-TTY stdin at EOF** (the default `$(…)` invocation already gives non-TTY;
  just omit the env var), asserting non-zero exit, the stall marker, the current
  key, the `ACCELERATOR_MIGRATE_DECISIONS_FILE=` + id + `run-migrations.sh`
  substrings, and `assert_not_contains` for the old baseline string. A
  validation-failure variant (short decisions file whose decision fails the
  fixture's `harness_reject "empty value not allowed"`, then EOF) exercises the
  VALIDATE_ERR site.
- No dedicated `test-run-migrations.sh`; the runner is covered by the four
  `test-migrate-*.sh` suites. `test-migrate-0007.sh` delegates its interactive
  body-path to `test-migrate-interactive.sh`.

## Interaction Model — how 0116 and 0117 couple

A question that the file:line view alone does not answer: when there is no input
channel, does the migration print *all* findings at once, or print the next
finding and work back-and-forth with the invoker? The answer is that **the
designed model (across 0116 + 0117) is a two-phase *batch* exchange, not a live
turn-by-turn protocol** — and 0116 alone can only ever surface the *current*
finding. The two work items split the problem deliberately.

### Why the live migration is one-prompt-at-a-time

The forked migration emits `PROMPT` frames **one at a time, blocking on each
decision before computing and emitting the next** (the runner loop reads a
PROMPT, calls `read_decision`, writes the decision back through FIFO fd 7, the
migration validates → possibly `VALIDATE_ERR` → `RECORDED`, then proceeds to the
next transformation). The proposed value for each prompt is revealed only by that
prompt. There is **no accumulator** of prior keys — only the single-prompt
`LAST_PROMPT_KEY` cache (`interactive-lib.sh:455`).

Consequence for **0116**: at the instant the stall fires (first prompt, no input
channel), the migration has emitted exactly one frame and is blocked; the
remaining findings *do not exist yet*. So 0116 is structurally limited to
**"print the current/next finding + a resume breadcrumb, then halt non-zero"**.
It cannot dump all findings, and it does not loop back-and-forth (no channel — it
stops). This is precisely why the work item makes only the *current* key a firm
criterion and demotes all-keys to best-effort.

### What 0117 adds: the batch `list → decide → write → resume` flow

0117 (`meta/work/0117-agent-decisions-bridge-and-invoker-contract.md`) is the
net-new capability that makes all-findings-at-once possible. Its required model
is the literal phrase `list → decide → write → resume` (AC5, `0117:142`):

1. **`--list` (dry-emit)** surfaces *every* pending interactive transformation up
   front, "before any prompt blocks, without mutating the corpus", one
   tab-delimited line each: `<position>\t<key>\t<proposed-value>\t<path>:<band/field>`
   (`0117:58-60`, `:99-107`). This is a **separate code path** from the live
   `harness_run` apply loop — it enumerates without blocking and without
   mutating, which is why it is a Medium task and cannot fold into 0116's
   two-line message change.
2. The agent **decides all** of them.
3. The agent **writes one complete decisions file** (newline-delimited,
   positional `accept` / `skip` / `edit <value>`, matched by emission order).
4. The agent **resumes** by re-running the driver with
   `ACCELERATOR_MIGRATE_DECISIONS_FILE` pointing at that file; the driver
   consumes it in bulk and **fails closed** on a count mismatch or unknown verb
   (`0117:61-68`, AC6).

This is a deliberately *batch* exchange. It is **not** a live protocol where the
agent answers one frame at a time through the running migration — the agent
communicates by issuing tool calls, not by writing bytes into a blocking `read`
on a FIFO, so there is no way to pump the live protocol turn-by-turn from the
Bash tool. The design sidesteps live interaction entirely.

### The decisions file is exactly "bulk-respond to N queries"

Confirmed: `ACCELERATOR_MIGRATE_DECISIONS_FILE` is a newline-delimited file of
positional verbs, **one line per prompt, consumed in emission order**
(`read_decision` reads one line per PROMPT off fd 9; parsing at
`interactive-lib.sh:266-287`). It is provisioned in bulk but consumed
sequentially in lockstep with the protocol. 0117 promotes it from the hidden
test-only seam it is today (`run-migrations.sh:14-18`) to a documented `--help`
interface and adds the count/verb fail-closed validation it currently lacks.

### How the two items couple (and a wording caveat for 0116)

0116 is the **breadcrumb**; 0117 is the **mechanism**. When an agent hits a
migration with no decisions file, 0116's stall names the pending key and points
at `ACCELERATOR_MIGRATE_DECISIONS_FILE`; 0117's `SKILL.md` invoker contract
documents the full `list → decide → write → resume` procedure and **links back to
0116 as the no-input outcome** (`0117:71`, AC5c). Both items record the soft
ordering: 0116 should land before/alongside 0117 so the documented stall matches
reality.

A nuance this exposes, relevant to **how 0116 words its resume command**: without
`--list`, the only way to grind through is an iterative resume loop — answer the
current prompt → re-run → the session log replays answered ones so you are
"prompted only for un-decided transformations" (the pre-flight hint's own words)
→ stall on the next → repeat, with a full re-run per decision. That iterative
back-and-forth *is* technically possible today via the session log, but is N
round-trips; `--list` collapses it to one. So **0116's resume command, used
before 0117 lands, is effectively a one-prompt-at-a-time breadcrumb; 0117 turns
it into "run `--list`, decide everything once".** This is adjacent to Open
Question 1 (resume-command reconciliation): whatever path/command 0116 prints
should be forward-compatible with 0117's `--list`-then-resume flow so the wording
need not change twice.

## Code References

- `skills/config/migrate/scripts/interactive-lib.sh:238-288` — `read_decision()`
  input-source chain; the no-input branch is `:262` (bare fd 0).
- `skills/config/migrate/scripts/interactive-lib.sh:449-453` — **emit site 1**
  (PROMPT), `failed to obtain decision for $p_key`; key var `p_key`, id `$id`.
- `skills/config/migrate/scripts/interactive-lib.sh:484-488` — **emit site 2**
  (VALIDATE_ERR), `failed to obtain re-decision for $LAST_PROMPT_KEY`; key var
  `LAST_PROMPT_KEY`, id `$id`.
- `skills/config/migrate/scripts/interactive-lib.sh:455-461` — `LAST_PROMPT_*`
  single-prompt cache (the only retained prompt state; no all-keys array).
- `skills/config/migrate/scripts/interactive-lib.sh:292` — `id` local; `:355`
  exported as `MIGRATION_ID`.
- `skills/config/migrate/scripts/interactive-lib.sh:312-316` — decisions file
  opened on fd 9; `:153-189` `write_session_record` (JSONL session log).
- `skills/config/migrate/scripts/interactive-lib.sh:566-572` — existing
  multi-line prefixed-stderr precedent for a stall block.
- `skills/config/migrate/scripts/run-migrations.sh:89-134` — pre-flight resume
  hint; prints `/accelerator:migrate` re-run + `rm` discard, **not** an env-var
  assignment (premise correction).
- `skills/config/migrate/scripts/run-migrations.sh:14-37` —
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` declared test-only + validated.
- `skills/config/migrate/scripts/test-migrate-interactive.sh:378-666,1069-1111` —
  AC3 non-regression anchors (exit 0 + JSONL counts).
- `skills/config/migrate/scripts/test-fixtures/interactive/0002-predicate/migrations/0002-predicate.sh`
  — canonical PROMPT-emitting fixture to mirror for a stall test.

## Architecture Insights

- **The stall is a re-messaging of an existing halt, not new control flow.** Both
  emit sites already do `exec 7>&-; return 1`. The change is the `echo` line.
  Keeping `exec 7>&-` + `return 1` preserves the FIFO teardown and the non-zero
  propagation the AC requires — the stall must *replace the message, not the exit
  semantics*, which matches the existing structure exactly.
- **Globals-as-interface convention.** The interactive layer relies on caller-set
  globals (`PROJECT_ROOT`, `id`, `LAST_PROMPT_KEY`). A stall helper should take
  the key + id as parameters because the current key lives in different variables
  at the two sites (`p_key` vs `LAST_PROMPT_KEY`).
- **Test-only seam becoming user-facing.** 0116 makes `ACCELERATOR_MIGRATE_
  DECISIONS_FILE` appear in user-facing output for the first time, ahead of 0117
  formally documenting it. Coordinate wording so 0117 doesn't have to re-message.
- **bash 3.2 floor** applies (macOS): the literal fd 9 for the decisions file
  (`:313-314`) exists because of it. Any new stall code must stay 3.2-clean (no
  associative arrays, no `${var,,}`).

## Historical Context

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  — the RCA that spawned 0115-0120. Confirms (H1, `:99-113`) the
  `read_decision` chain failing under agent invocation and the opaque
  `failed to obtain decision` abort *after corpus mutation*. Fix Options table
  (`:212-218`): **B = "make read_decision detect the no-input case and emit an
  actionable, structured stall (list the pending keys + the exact resume
  command)"**, Risk/Effort Low/Low → this is 0116. Recommends "C + D as durable
  fix, with B as immediate low-risk mitigation"; B "worth shipping regardless".
  Notes the decisions-file seam is deliberately hidden (`:193-198`) and that an
  agent "could not populate it correctly anyway without first seeing the prompts"
  — so B is a relay/debuggability mitigation, not a self-sufficient fix.
- `meta/reviews/work/0116-structured-stall-on-no-decision-input-review-1.md` —
  two-pass review, final **APPROVE**. Pass 1 (REVISE) flagged exactly the
  ambiguities the code confirms: the resume command "lacks a verifiable shape"
  and key cardinality was inconsistent. Resolution that shipped: AC now requires
  a non-empty `ACCELERATOR_MIGRATE_DECISIONS_FILE` path + id substring +
  `run-migrations.sh`; cardinality reconciled to "at least the current key"
  firm, all-keys best-effort; "structured stall" defined inline (non-zero halt +
  parseable block). This review is why the AC, not the "match the pre-flight
  hint" prose, is the binding contract.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — foundational
  migration framework (the runner 0116 edits).
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
  — the interactive contract the stall must honour.
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
  — interactive validation parameters for the 0007 linkage migration.
- Prior codebase research:
  `meta/research/codebase/2026-05-30-0069-migration-framework-interactive-validation-hooks.md`,
  `…/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md`,
  `…/2026-06-04-migration-upgrade-failures.md`.

## Related Research

- Sibling work items in the same fix set: `meta/work/0115-…` (parent),
  `0117-agent-decisions-bridge-and-invoker-contract.md` (fix A — promotes the
  env var; same code region, intended to land *after* 0116),
  `0118-reconcile-0007-backfill-sentinel-with-validator.md` (fix C — functional
  precondition for 0007 reaching the interactive phase),
  `0119-resume-safe-partial-migration-failure.md` (fix E — owns the partial-run
  resume the stall's command is a breadcrumb for),
  `0120-prevention-tests-for-agent-invocation-path.md` (asserts this stall;
  must land after 0116).
- No implementation plan exists yet for 0116 or any sibling (0115-0120).

## Open Questions

1. **Resume-command reconciliation (must resolve before implementing).** The
   work item prose says match `run-migrations.sh:90-132`, but that hint prints a
   `/accelerator:migrate` re-run, not an `ACCELERATOR_MIGRATE_DECISIONS_FILE`
   assignment. The binding AC requires the env-var form. Confirm the intended
   resume line is, e.g., `ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>
   bash <run-migrations.sh> # migration <id>` and that pointing the user at the
   *test-only* env var (ahead of 0117 documenting it) is acceptable for the
   mitigation.
2. **Where to put no-input detection** — hoist an explicit
   "no decisions file ∧ `! [ -t 0 ]`" check into `read_decision` (returning a
   distinct status), or detect at the two call sites? The call-site approach
   keeps `read_decision` unchanged but duplicates the condition; a distinct
   return status centralises it but touches the success/failure contract.
3. **`<path>` value when no decisions file was ever set.** The stall needs a
   concrete path to print. Likely the deterministic
   `.accelerator/state/migrations-<id>-session.jsonl` or a suggested
   decisions-file path under `.accelerator/state/` — but the session log is
   JSONL while the env var expects the `accept|edit|skip` line format, so they
   are not interchangeable. Decide which path the resume command should name.
