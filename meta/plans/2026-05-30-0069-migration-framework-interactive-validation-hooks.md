---
date: "2026-05-30T11:30:00+01:00"
type: plan
skill: create-plan
work-item: "0069"
status: accepted
---

# 0069: Extend Migration Framework with Interactive Validation Hooks — Implementation Plan

## Overview

Extend `skills/config/migrate/scripts/run-migrations.sh` with an opt-in interactive contract (ADR-0037) so migrations can declare a trigger predicate, route candidate transformations through accept / edit / skip prompts, and resume across invocations from an on-disk session log. The mechanical-default path (ADR-0023) is preserved verbatim: a migration that does not declare the hook runs through the existing code path unchanged.

The first consumer (work item 0070, parameterised by ADR-0038) is *not* built here — this plan ships the framework only. Hook detection, predicate evaluation, display, decision controls, session-log persistence, resume-state reconstruction, source-drift handling, and the worked-example doc are all in scope; the unified-schema migration's parser, band classifier, prose-line extraction, and ADR-0038-shape session-log record extensions are out of scope.

## Current State Analysis

The runner is `skills/config/migrate/scripts/run-migrations.sh` — 220 lines, no internal functions, nine numbered sections. Per-migration invocation at run-migrations.sh:184 is:

```bash
bash "$f" >"$STDOUT_FILE" 2>&1
```

The runner has exactly one structured channel from migration to runner: the `MIGRATION_RESULT: no_op_pending` stdout sentinel (run-migrations.sh:192) that defers a migration. There is no channel from runner to migration, no per-transformation conversation, and no on-disk artefact for migration-internal state.

Existing primitives that the new contract reuses unchanged:

- **Ledger paths**: `.accelerator/state/migrations-applied`, `.accelerator/state/migrations-skipped` (run-migrations.sh:13-14).
- **Atomic helpers**: `atomic_write`, `atomic_append_unique`, `atomic_remove_line` (`scripts/atomic-common.sh:16-80`).
- **Migration discovery**: `find … -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 | sort -z` (run-migrations.sh:88-93).
- **Clean-tree pre-flight**: jj/git detection, `meta/`, `.claude/accelerator*.md`, `.accelerator/` checked (run-migrations.sh:42-70).
- **Header-comment metadata convention**: `# DESCRIPTION:` on line 2 (precedent for line-prefixed declarative metadata the runner greps for).
- **Test harness**: `scripts/test-migrate.sh` (2,021 lines, ~80 inline `assert_contains` / `assert_file_exists` tests).

### Key Discoveries

- Six migrations exist (`0001` through `0006`); none declare the hook. They are the corpus that the snapshot-test AC-1 must run against unchanged.
- The `MIGRATION_RESULT:` namespace is the existing precedent for migration→runner structured communication, but it is one-shot end-of-process — not suitable as a per-transformation conversation channel.
- Atomic-write primitives in `scripts/atomic-common.sh` are same-directory temp-then-rename and match ADR-0037 §3 guarantee 1 conceptually, but JSONL append requires a new helper; source-drift record replacement requires a remove-by-key companion.
- The test harness has no precedent for piping stdin or scripted prompt responses — interactive testing requires a runner-side decisions-file mechanism (delivered as the `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var, see Implementation Approach).
- ADR-0037 §3 explicitly leaves the persistence mechanism to the implementer; ADR-0038 specifies JSONL `migrations-<id>-session.jsonl` for its consumer, which we adopt as the framework convention.
- ADR-0037 §2 mandates exactly three display elements; additional fields are migration-declared. ADR-0037 §4 specifies accept/edit/skip semantics (artifact effect + session-log effect each).
- The story explicitly *promotes* two runner-level decisions ADR-0037 left silent: source-drift behaviour (re-prompt with new proposed value; discard old record) and transformation ordering invariant (emission order from the migration's emit function).

## Desired End State

After this plan is complete:

- The runner detects hook-declaring migrations by a `# INTERACTIVE: yes` header line and routes their transformations through a prompt loop with accept / edit / skip controls.
- Hook-declaring migrations write a JSONL session log at `.accelerator/state/migrations-<id>-session.jsonl`, written incrementally per decision by the runner, and re-read on subsequent invocations to skip already-decided transformations.
- Source-drift between runs (recorded proposed value ≠ current proposed value for the same transformation key) re-prompts the user and discards the old record.
- Migrations that do not declare the hook run through the existing code path with byte-identical artefact output and the same exit-code semantics, verified by the full existing `test-migrate.sh` suite passing unchanged.
- `skills/config/migrate/SKILL.md` documents the hook declaration mechanics, ADR-0037 §§1–4 runner guarantees, the runner-level source-drift and ordering decisions, the transformation-key schema, and a worked example whose transcript is CI-asserted against a fixture run.
- A new `scripts/test-fixtures/interactive/` tree provides synthetic migrations exercising every AC, driven via the `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var.

### Verification

- `bash skills/config/migrate/scripts/test-migrate.sh` exits 0 and the existing ~80 assertions all pass (AC-1 regression net).
- New interactive-hook tests under `skills/config/migrate/scripts/test-migrate-interactive.sh` cover every AC in the story.
- `skills/config/migrate/SKILL.md` documents the new contract; the doc's worked-example transcript matches the CI-recorded fixture output.

## What We're NOT Doing

- **Not implementing the unified-schema corpus migration (0070)**. The parser, band classifier, prose-line extraction, ADR-0034 vocabulary validation, and ADR-0038-shape session-log record extensions all live with 0070.
- **Not modifying ADR-0023, ADR-0037, ADR-0038, or any other accepted ADR** (ADR-0031 immutability).
- **Not promoting source-drift or transformation-ordering to a supplementary ADR** here — they remain documented runner-level decisions per the story's Open Questions resolution.
- **Not changing the discoverability hook** (`hooks/migrate-discoverability.sh`) — interactive-flow is downstream of session start.
- **Not adding a `--dry-run` flag** (ADR-0023 explicitly rejects this).
- **Not introducing file-level skipping** beyond the existing `--skip <id>` mechanism. Per-transformation skip is part of the new contract; file-level "skip all transformations under <path>" is a migration concern.
- **Not changing `MIGRATION_RESULT: no_op_pending` semantics**. Interactive migrations may still emit `no_op_pending` from their pre-flight before `harness_run` is called.
- **Not adding new dependencies** (no Python, no `jq`). Pure bash + standard POSIX tools, matching existing conventions.

## Implementation Approach

### Architecture: coproc + harness library

The runner forks the migration as a child process with bidirectional pipes (bash `coproc`). The migration sources `scripts/interactive-harness.sh` and calls `harness_run`, which drives the per-transformation loop by calling migration-declared callbacks. The runner and harness exchange line-delimited TAB-separated frames; JSON appears only on disk (in the session log), not on the wire.

This decomposition keeps child-process isolation — `exit 1`, `set -e`, symbol scope all stay inside the migration's subshell — and keeps the migration script readable as a single top-to-bottom file. The harness library absorbs the protocol loop so the migration author writes only data callbacks and mutation code.

### Migration-author surface (the contract the migration writes)

A hook-declaring migration is a single bash script with a header marker, sourced helpers, four required callbacks plus an optional fifth, and a final `harness_run` call. Authors use helper functions (provided by `interactive-harness.sh`) to emit transformations, set extras, and reject edits — they never hand-write TSV positional fields, base64-encode display blocks, or hand-write JSON. The skeleton:

```bash
#!/usr/bin/env bash
# DESCRIPTION: <short imperative>
# INTERACTIVE: yes
set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() {
  # Authors call harness_emit_transformation once per transformation.
  # The helper packs named arguments into the canonical TSV wire format,
  # base64-encodes the display block, and writes one line to stdout.
  # Extras are migration-declared key=value pairs accumulated by
  # harness_extras_set; they are reset between transformations.
  for record in ...; do
    harness_extras_set band "$record_band"
    harness_extras_set prose "$record_prose"
    harness_emit_transformation \
      key="$key" path="$record_path" anchor="$record_anchor" \
      proposed="$record_proposed" predicate_value="$record_band" \
      display="Proposed value: $record_proposed
Surrounding prose: $record_prose"
  done
}

migration_evaluate_predicate() {
  # stdin: one TSV transformation line (treat as opaque; use
  # harness_field <name> to extract specific fields by name).
  # exit 0 = route to prompt; exit 1 = apply mechanically (no prompt).
  # Any other non-zero exit is a contract violation: the harness emits
  # FAIL with a clear message; the runner aborts the migration.
  ...
}

migration_validate_edit() {
  # args: key path anchor proposed_value user_value
  # On invalid input call harness_reject "<message>" (which prints to
  # stderr and exits non-zero with a uniform message format).
  # On valid input exit 0 silently.
  ...
}

migration_apply_decision() {
  # args: key path anchor decision value
  #   decision ∈ {accept, edit}; skip is never delivered here.
  # Mutate the artifact at (path, anchor) to value.
  # Called by the harness only AFTER the runner has confirmed durable
  # JSONL persistence via the APPLY frame (write-ahead-log invariant).
  # A non-zero exit is a hard error: the harness emits FAIL with the
  # callback's stderr; the runner aborts the migration without ledger
  # append. Authors should treat this callback as the single point of
  # artefact mutation per transformation.
  ...
}

# Optional override; default returns ".accelerator/state/migrations-<id>-session.jsonl"
migration_session_log_path() { ...; }

# Optional. Called on resume before RESUMED_APPLIED to detect a partial or
# un-applied mutation from a prior crashed run. See "Write-ahead-log invariant"
# above. If not declared, the framework assumes the recorded mutation is
# present in the artefact (best-effort; VCS revert is the universal recovery).
migration_verify_applied() {
  # args: key path anchor recorded_outcome recorded_proposed [recorded_user_value]
  # exit 0 = mutation is present; harness emits RESUMED_APPLIED.
  # exit 1 = mutation absent or partial; harness removes the session-log
  #          record (via DRIFT) and re-prompts.
  ...
}

harness_run
```

The harness handles: protocol I/O, resume-state load and replay, source-drift detection, validation-re-prompt control flow, frame emission, TSV field encoding, base64 display encoding, JSON escape rules for extras, and APPLY-frame synchronisation. The migration author does not see the protocol.

**Author-facing helpers** (defined in `scripts/interactive-harness.sh`, sourced by every hook-declaring migration):

- `harness_emit_transformation key=K path=P anchor=A proposed=V predicate_value=PV display=$'multi\nline'` — emits one TSV transformation record on stdout. Named arguments are required; extras are taken from the `harness_extras_set` accumulator. The helper handles field escaping (TAB/newline/backslash) and base64 of the display block. Reset extras after emission.
- `harness_extras_set <key> <value>` — accumulate one extras key/value for the next `harness_emit_transformation` call. Repeated calls overwrite the same key. Keys must match `^[a-z][a-z0-9_]*$`. Reserved keys (collision with framework-mandatory `transformation_key`, `outcome`, `proposed_value`, `user_value`, `timestamp`, `schema_version`) are rejected at call time with a clear error. **Lifecycle**: extras are *auto-cleared after each `harness_emit_transformation` call*. The intended pattern is set-inside-loop; setting extras once before the loop and emitting many transformations will silently drop extras on every record after the first. Authors who want the same extras on every record must call `harness_extras_set` inside the loop, or factor the extras-population into a helper called from inside the loop body.
- `harness_extras_clear` — drop all accumulated extras (called automatically after each emission; available for explicit reset).
- `harness_field <field_name>` — extract a named field value from the current TSV transformation line on stdin (used inside `migration_evaluate_predicate`); returns the unescaped value. Authors never index by TAB position.
- `harness_reject "<message>"` — print `<message>` to stderr in a uniform format (`[interactive] <message>`) and exit non-zero. Used by `migration_validate_edit` so the user-visible error format is consistent across migrations.

### Wire protocol (runner ↔ harness, line-delimited, TAB-separated)

JSON does not appear on the wire. The session log is the only JSON surface (on disk).

**Runner → migration** (migration's stdin):

| Frame | Fields | Meaning |
|---|---|---|
| `INIT` | `resume_state_path`, `decisions_path` (empty for live runs) | Handshake; provided once at startup |
| `DECIDE` | `outcome ∈ {accept,edit,skip}`, `value` (empty unless edit) | User decision in response to `PROMPT` |
| `APPLY` | `key` | Runner has durably persisted the JSONL record for `<key>`; the harness MUST now call `migration_apply_decision` (for `accept`/`edit`) or do nothing (for `skip`), then emit `APPLIED_CONFIRM` |
| `DRIFT_CLEARED` | `key` | Runner has removed the stale resume-record for `<key>`; harness may now route the key through the normal predicate-and-prompt path |
| `ABORT` | (no fields; runner is cancelling) | Harness should exit |

**Migration → runner** (migration's stdout):

| Frame | Fields | Meaning |
|---|---|---|
| `READY` | `session_log_path` | Handshake response |
| `MECHANICAL_APPLIED` | `key` | Predicate did not fire; harness applied the mutation mechanically in this run; runner persists no record |
| `RESUMED_APPLIED` | `key` | Resume-state recorded the key as `accepted`/`edited` and proposed-value matches; harness did NOT re-apply (the prior run's mutation is presumed durable on disk); runner persists no new record |
| `RESUMED_SKIPPED` | `key` | Resume-state recorded the key as `skipped` and proposed-value matches; harness did nothing; runner persists no new record |
| `PROMPT` | `key`, `path`, `anchor`, `proposed`, `predicate_value`, `extras_tsv`, `display_lines_b64` | Predicate fired (or source-drift cleared); runner renders display, reads decision, sends `DECIDE`. `extras_tsv` is a TSV-encoded `key=value` list (see "Extras encoding" below) — JSON does not appear in this frame |
| `VALIDATE_ERR` | `message` | After a `DECIDE edit`, the migration's validator rejected the value; runner re-prompts (no `RECORDED` is emitted for the rejected attempt) |
| `RECORDED` | `key`, `outcome`, `proposed`, `user_value`, `extras_tsv` | A valid decision has been determined; harness REQUESTS the runner to persist a JSONL record and waits for `APPLY` before any artefact mutation. The runner owns all JSON construction — `extras_tsv` carries author-declared key/value pairs which the runner encodes via the canonical JSON-escape rule |
| `APPLIED_CONFIRM` | `key` | Harness has finished executing `migration_apply_decision` (or skipped it for `skip`) after receiving `APPLY`; runner may proceed to the next transformation |
| `DRIFT` | `key` | Resume state had this key, but live proposed-value differs; harness REQUESTS the runner to remove the stale record, then waits for `DRIFT_CLEARED` before routing through the prompt path |
| `DONE` | (no fields) | All transformations processed |
| `FAIL` | `message` | Migration aborting |

**Write-ahead-log invariant**: the harness performs no artefact mutation between `PROMPT` and `APPLY`. The runner's `RECORDED → atomic_jsonl_append → APPLY` sequence guarantees that the session-log record is durably on disk *before* the artefact is mutated.

**Residual failure mode** (acknowledged, bounded, optionally detected): two crash windows remain after the inversion, and both produce the same observable state — a session-log record marking the decision as `accepted` or `edited` while the artefact mutation may not have completed (or may not have started at all):

1. *Runner crashes between persist and APPLY emission*: record is durable, harness never called `migration_apply_decision`. On resume, `RESUMED_APPLIED` triggers; the artefact stays un-mutated.
2. *Harness crashes between APPLY and APPLIED_CONFIRM*: record is durable, `migration_apply_decision` started but may not have finished (e.g., `sed -i` died mid-rewrite). On resume, `RESUMED_APPLIED` triggers; the artefact is in an unknown state.

In both cases the next run treats the key as decided and does NOT call `migration_apply_decision` again — non-idempotent mutations are not double-applied, but a partial or un-mutation may persist undetected. This is bounded in blast radius by the runner-level guarantee (the user's decision is durable) and bounded in frequency by the tiny time-window between the two operations, but it is real and must be handled by the migration author or surfaced by the framework.

The framework provides an **optional `migration_verify_applied` callback** for migrations whose mutations are not idempotent or whose silent un-application would be harmful:

```bash
# Optional. If defined, called on resume BEFORE emitting RESUMED_APPLIED.
# args: key path anchor recorded_outcome recorded_proposed [recorded_user_value]
# exit 0 = the recorded mutation is present in the artefact; emit RESUMED_APPLIED.
# exit 1 = the recorded mutation is NOT present (or partial); the harness
#          removes the session-log record via DRIFT and routes through the
#          normal PROMPT path so the user can re-decide.
migration_verify_applied() { ...; }
```

If the migration declares this callback, the framework detects both crash windows automatically and recovers by re-prompting. If the callback is not declared, the residual risk is documented in SKILL.md with a recovery instruction (`rm <session-log-path>` line for the affected key, then re-run). VCS revert remains the universal recovery path per ADR-0023. This shifts the residual failure mode from "single mutation that may not have completed (silent)" to "single mutation that may not have completed (detectable per-migration via opt-in callback)" — bounded in blast radius by the runner-level guarantee, bounded in detection by the migration's opt-in verification predicate.

Field-escaping rule: literal backslash escapes first (`\\`), then TAB (`\t`), then newline (`\n`) — applied in that order on emit, with a single-pass state-machine unescape on read. Helpers `escape_field` / `unescape_field` live in `scripts/interactive-protocol.sh` and are sourced by both `scripts/interactive-harness.sh` and `skills/config/migrate/scripts/interactive-lib.sh` (single source of truth — see Phase 3).

**Extras encoding**: the `extras_tsv` field carries author-declared key/value pairs as `key1=value1<US>key2=value2<US>…` where `<US>` is ASCII `0x1F` (Unit Separator). The `=` separator between key and value is reserved; keys must match `^[a-z][a-z0-9_]*$` (rejected at `harness_extras_set` call time if not). Values are TSV-field-escaped exactly like every other field — TAB/newline/backslash backslash-escaped. The runner parses this into a key/value map and encodes each value as a JSON string (using the same `jsonl_json_escape` helper from Phase 2 §4) when composing the session-log record. JSON never appears on the wire; the runner is the only JSON producer. Empty extras_tsv (an empty field) is valid and means "no extras".

**Reserved keys**: `transformation_key`, `outcome`, `proposed_value`, `user_value`, `timestamp`, `schema_version` are framework-mandatory and rejected if used as extras keys. The author-facing `harness_extras_set` checks for collision at call time and exits with a clear error; a defensive check in the runner also rejects them on receipt (`FAIL` frame back to harness with a "reserved key" message).

### Detection mechanism

Header marker, grep-cheap, single-file: line 3 (or any of the first ~5 header-comment lines) matches `^# INTERACTIVE:[[:space:]]*yes$`. The runner reads this on the existing migration-discovery pass; no extra subprocess fork. Filename convention is unchanged (`NNNN-<slug>.sh`).

### Session-log shape

Path: `.accelerator/state/migrations-<id>-session.jsonl` (default; migration can override via `migration_session_log_path`).

One JSON object per line, **with a canonical field ordering**: `transformation_key` MUST be the first field (so `atomic_jsonl_remove_by_key`'s anchored-prefix match is well-defined), `schema_version` MUST be the second field (so the resume parser can fail-fast on unknown versions without parsing the rest of the line). All other framework-mandatory fields follow in fixed order, then author-declared extras in `harness_extras_set` insertion order:

```json
{"transformation_key":"…","schema_version":1,"outcome":"accepted","proposed_value":"…","user_value":"…","timestamp":"…","band":"…","prose":"…"}
```

- **Framework-mandatory keys**: `transformation_key`, `schema_version`, `outcome`, `proposed_value`, `timestamp`. `user_value` is added only for `outcome:"edited"`; it is **omitted** (not empty) for `accepted` and `skipped`.
- **`schema_version`**: integer, currently `1`. The runner refuses to resume from a log whose `schema_version` is outside its supported set; users see a clear error with a recovery instruction. New schema versions are introduced via a supplementary ADR per ADR-0037 §5.
- **Author-declared extras**: each key from the `RECORDED` frame's `extras_tsv` field becomes a top-level JSON key with the value JSON-string-encoded by the runner's `jsonl_json_escape`. Migration authors do not hand-write JSON; reserved-key collisions are rejected at `harness_extras_set` call time and again defensively by the runner on receipt.
- **Writer**: the runner composes the record by emitting framework-mandatory fields in canonical order, then extras pairs in receipt order, using a single `jsonl_compose_record` helper that handles all escaping. No string slicing, no `{` stripping, no parser dependency.
- **Reader**: `build_resume_state_file` uses the awk-based JSONL-aware extractor (Phase 6 §5) that handles every JSON string escape sequence; no naive `sed` regex.
- **Atomicity**: runner writes via `atomic_jsonl_append` (Phase 2 §3). Source-drift removal uses `atomic_jsonl_remove_by_key` (Phase 2 §4), which relies on the canonical first-field-is-transformation_key invariant.

### Non-interactive testing mechanism: `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var

Each line of the decisions file is one decision: `accept`, `skip`, or `edit <value>`. Empty lines are ignored. The runner reads one line per `PROMPT` frame (including re-prompts triggered by `VALIDATE_ERR`). If the file is exhausted before the migration emits `DONE`, the runner treats that as test failure (`echo "decisions file exhausted" >&2; exit 1`). For tests that want to simulate user-interrupt mid-stream, the test harness sends SIGKILL to the runner.

The decisions file path is supplied via the env var `ACCELERATOR_MIGRATE_DECISIONS_FILE` (not a CLI flag). This matches the existing `ACCELERATOR_MIGRATE_FORCE` / `ACCELERATOR_MIGRATIONS_DIR` precedent for test-only mechanisms, keeps the runner's user-facing CLI surface clean, and prevents end users from discovering and using the flag in production where it would bypass safety-relevant prompts. The runner emits an informational stderr line per consumed decision (`[decisions] consumed line N: <verb>`) so test authors can verify line accounting.

### Independence between phases

The phases below are sequenced for ease of review but each phase's deliverable is **independently shippable**: at the end of each phase, the test suite is green, the mechanical path is unchanged, and the new behaviour is gated either by `ACCELERATOR_MIGRATE_DECISIONS_FILE` (set only in test fixtures) or by the `# INTERACTIVE: yes` header (absent from every bundled migration until 0070). Each phase can land independently of the others.

**Note on the first consumer (work item 0070)**: while each phase is shippable in isolation, an end-to-end interactive migration cannot run until Phases 1–6 are *all* landed (persistence, detection, predicate routing, decision verbs, and resume must all be in place). 0070 must not ship before Phase 6 + the bash 4+ source-gate (see Migration Notes) are merged. Phase 7 documentation may lag without blocking 0070.

### TDD discipline

Each phase starts with the failing tests (using existing `assert_*` helpers from `scripts/test-helpers.sh`), then implements the runner / harness code to pass them. Protocol-frame assertions replace function-call assertions where the contract is on the wire.

---

## Phase 1: Mechanical-path byte-identical regression net + env-var plumbing + session-log pre-flight UX

### Overview

Lock down AC-1 ("Mechanical path unchanged") with a **byte-identical** `diff -r` snapshot test *before* changing any runner behaviour. Read `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var (unused until Phase 4). Extend the dirty-tree pre-flight with a session-log-aware distinct message. Create the `test-migrate-interactive.sh` skeleton and the `test-fixtures/interactive/` directory. Add a one-line note to SKILL.md and CHANGELOG indicating the upcoming bash 4+ floor for the interactive path (the assertion lands in Phase 3 but the prerequisite is announced now).

### Changes Required

#### 1. Existing-suite baseline (behavioural)

**File**: `skills/config/migrate/scripts/test-migrate.sh`
**Changes**: No edits required. The existing ~80 tests are retained as the behavioural regression net (outcomes, file presence, content substrings). They continue to gate every phase.

#### 2. NEW: Mechanical-path byte-identical snapshot test

**File**: `skills/config/migrate/scripts/test-migrate-snapshot.sh` (new)
**Changes**: A new test that captures **byte-identical** behaviour of the runner against migrations 0001–0006:

- Before any phase work begins, run the unmodified runner against a fixed fixture repo (one per migration corpus shape) and capture: `(a)` the full migrated artefact tree (`find <fixture> -type f | xargs sha256sum > snapshot-files.txt` and a full `tar c <fixture> | sha256sum`), `(b)` stdout + stderr (timestamp-redacted), `(c)` final exit code.
- Persist the snapshots under `skills/config/migrate/scripts/test-fixtures/mechanical-snapshots/`.
- The test re-runs the post-change runner against the same fixtures and asserts byte-identical output (`diff -r` on the artefact tree; redacted stream diffs against the captured streams).
- Asserts the snapshot file itself is checked-in (the test fails if `mechanical-snapshots/` is missing — prevents accidental local-state regressions).

Closes Safety + Compatibility findings on AC-1 being behavioural rather than byte-identical.

#### 3. `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var (read-only, no-op)

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: No CLI argument is added. The runner reads the env var at startup:

```bash
ACCELERATOR_MIGRATE_DECISIONS_FILE="${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}"
export ACCELERATOR_MIGRATE_DECISIONS_FILE
# unused until Phase 4
```

If set, the runner validates the file exists and is readable; otherwise behaviour is unchanged. Test-only mechanism — never used in production runs. Closes Usability finding on test-only flag leaking into user-facing CLI.

#### 4. Session-log-aware dirty-tree pre-flight

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: Extend the existing dirty-tree pre-flight (run-migrations.sh:42-70) so that when the dirty paths include one or more `migrations-<id>-session.jsonl` files under `.accelerator/state/`, the runner emits a **distinct, named** error rather than the generic "uncommitted changes" message:

```text
Found in-flight interactive migration session(s):
  .accelerator/state/migrations-0070-session.jsonl  (NN decisions recorded)

To resume: re-run /accelerator:migrate (the session log is read on entry; you will be prompted only for un-decided transformations).
To discard: rm .accelerator/state/migrations-<id>-session.jsonl  (loses NN decisions)

If the above does not match what you expected, run `jj status` (or `git status`) to see all uncommitted changes before proceeding.
```

The pre-flight still refuses to run until the tree is clean (committing the session log preserves resumable state across machines, which is the desired behaviour); the new message specifically prevents users from `jj abandon`-ing in confusion. Closes Safety finding on silent decision loss.

#### 5. Interactive test harness skeleton

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh` (new)
**Changes**: Bootstrap file mirroring `test-migrate.sh`: source `test-helpers.sh`, `TMPDIR_BASE=$(mktemp -d)`, trap cleanup, define `DRIVER` and `MIGRATIONS_DIR_FIXTURE` (pointing at `test-fixtures/interactive/`). Initial tests:

- `ACCELERATOR_MIGRATE_DECISIONS_FILE=/dev/null bash run-migrations.sh` on a no-pending-migrations repo behaves identically to the same invocation without the env var.
- Robustness: env var set to a non-existent path, an unreadable file, a directory, an empty file, and a file with CRLF line endings — each produces a clear stderr message and a specific exit code. Closes Test Coverage finding on decisions-file edge cases.

#### 6. Fixture directory skeleton

**File**: `skills/config/migrate/scripts/test-fixtures/interactive/.gitkeep` (new)
**Changes**: Empty directory placeholder. Per-fixture migration trees are added in Phases 3–6.

#### 7. SKILL.md prerequisites note

**File**: `skills/config/migrate/SKILL.md`
**Changes**: Add a one-line `## Prerequisites` note (or extend an existing prerequisites section) stating that **the interactive migration path requires bash 4.0+**; mechanical migrations continue to support bash 3.2. macOS users on stock `/bin/bash` need Homebrew bash (or mise) before running an interactive migration. Closes Documentation finding on docs landing only in Phase 7 (this is the user-visible surface added by Phase 1).

#### 8. CHANGELOG entry (if a CHANGELOG exists in the plugin)

**File**: `CHANGELOG.md` (or equivalent)
**Changes**: One line under "Unreleased": `Migration framework: new optional interactive contract (work item 0069). Interactive path requires bash 4.0+. Mechanical migrations unchanged.`

### Success Criteria

#### Automated Verification

- [x] Existing suite passes: `bash skills/config/migrate/scripts/test-migrate.sh` exits 0.
- [x] **Byte-identical snapshot test passes**: `bash skills/config/migrate/scripts/test-migrate-snapshot.sh` exits 0 against the unmodified runner; will re-run as a regression gate in every subsequent phase.
- [x] Interactive harness skeleton passes: `bash skills/config/migrate/scripts/test-migrate-interactive.sh` exits 0.
- [x] `ACCELERATOR_MIGRATE_DECISIONS_FILE=/dev/null bash skills/config/migrate/scripts/run-migrations.sh` on a no-pending-migrations repo is byte-identical to the same invocation without the env var.
- [x] Env-var robustness tests pass (non-existent, unreadable, directory, empty, CRLF).
- [x] Session-log-aware pre-flight emits the distinct message when a `migrations-<id>-session.jsonl` is dirty; emits the generic message when only other paths are dirty.
- [ ] `shellcheck skills/config/migrate/scripts/run-migrations.sh` produces no new warnings against a pinned `shellcheck` invocation: `mise exec -- shellcheck -x skills/config/migrate/scripts/run-migrations.sh` (shellcheck downgraded from per-phase Success Criterion to project-wide precondition — see Code Quality finding; the pinned invocation is now part of the test harness).

#### Manual Verification

- [x] `ACCELERATOR_MIGRATE_DECISIONS_FILE` is invisible in user-facing CLI output (no `--help` mention, no banner).
- [x] Session-log-aware pre-flight message is readable and actionable to a user encountering it for the first time.

---

## Phase 2: Atomic JSONL helpers (append + remove-by-key)

### Overview

Add the two helpers the session-log machinery needs. TDD-first: write the durability, concurrency, and key-removal tests before the implementations.

Both helpers are built on the same primitive the existing `atomic-common.sh` already provides: **same-directory temp-then-rename via `atomic_write`**. Direct `>>` append is rejected because (a) POSIX does not guarantee single-`write(2)` atomicity for `O_APPEND` on regular files; (b) `printf` may issue multiple `write(2)` syscalls for lines larger than the kernel's per-write atomicity bound (which is filesystem-dependent and not PIPE_BUF — PIPE_BUF applies to pipes/FIFOs, not files); and (c) `sync(8)` is a system-wide flush, not a per-file durability primitive. The temp-then-rename pattern gives us crash-safe atomicity at the record level: either the rename has committed (the record is durably visible) or it has not (the prior file content is durably visible). No partial-line state is ever observable.

JSONL records carry a canonical schema: `transformation_key` MUST be the first key in every record. Phase 4 §8 enforces this in the writer; Phase 2's removal helper relies on it for an anchored line-start match.

### Changes Required

#### 1. Tests for `atomic_jsonl_append`

**File**: `scripts/test-atomic-common.sh` (extend existing)
**Changes**: The file already exists and contains assertions for `atomic_write` / `atomic_append_unique` / `atomic_remove_line`. Append a new section (preserving the existing tests unchanged) that asserts:

- A single call writes exactly one line, newline-terminated.
- Repeated calls append (do not overwrite).
- **Concurrent safety across line sizes**: two backgrounded subshells calling concurrently each produce a complete, well-formed line, parametrised over line sizes of 100 B, 1 KiB, 4 KiB, 16 KiB, and 64 KiB (spanning typical PIPE_BUF boundaries). Assert every line in the result passes a `python3 -m json.tool` precondition and the file contains exactly the expected number of lines.
- **Race-the-crash durability**: launch a background writer that loops appending a record then sleeps a randomised microsecond delay; the parent sends SIGKILL at randomised intervals over a fixed wall-clock budget. After every kill, assert every line currently on disk is well-formed JSON (no partial lines, no truncation mid-record). Repeats over multiple iterations to exercise different kill points.
- **Concurrent reader during writes**: a reader process tails the file while a writer appends; the reader sees only complete lines (no partial state).

#### 2. Tests for `atomic_jsonl_remove_by_key`

**File**: `scripts/test-atomic-common.sh` (extend existing)
**Changes**: Append:

- Removing a key present in the file rewrites the file without that record; other records survive in their original order.
- Removing a key absent from the file is a no-op (exit 0, file unchanged).
- Removing from an empty/absent file is a no-op.
- Multiple records with the same key are all removed.
- **Prefix-collision safety**: file contains records with keys `foo` and `foobar`; remove `foo`; assert `foobar` survives intact.
- **Substring-in-other-field safety**: a record's `proposed_value` or `extras` field contains the literal substring `"transformation_key":"foo"` (e.g. as JSON-escaped prose); remove key `foo` (which is also present as a real first-field key in a separate record); assert only the real-first-field record is removed, the substring-bearing record survives.
- **Escape-character round-trip**: write a record whose key contains `"`, `\`, tab, and newline (JSON-escaped by the writer); call the remover with the same logical key; assert the record is removed and no others are touched. **Critical regression guard**: explicitly cross the awk-`-v`-vs-ENVIRON boundary — a key like `key-with-"-and-\` produces a JSON-escaped prefix containing `\"` and `\\` literals, which awk's `-v` would re-interpret as `"` and `\` (defeating the match). The test must fail if the implementation regresses to `awk -v`.
- **Pipeline-failure surfacing**: simulate an unwritable target directory; assert the helper returns non-zero with a clear stderr message (no silent failure via `|| true`).

#### 3. Implementation: `atomic_jsonl_append`

**File**: `scripts/atomic-common.sh`
**Changes**: Append:

```bash
# atomic_jsonl_append <target_path> <json_line>
#   Append one JSONL record atomically at the record level.
#
#   Implementation: read the existing file (if any), concatenate the new
#   line, write via atomic_write (same-directory temp + rename). The
#   rename is atomic per POSIX rename(2), so a crash leaves either the
#   prior file content or the post-append content fully visible — no
#   partial-line state is ever observable on disk. Concurrent callers
#   serialise via flock on a sidecar lockfile to prevent lost updates.
#
#   The caller is responsible for ensuring <json_line> is a single line
#   of valid JSON. The helper does not validate JSON.
atomic_jsonl_append() {
  local target="$1" line="$2"
  if [ -z "$target" ] || [ -z "$line" ]; then
    echo "atomic_jsonl_append: missing target or line" >&2; return 1
  fi
  case "$line" in *$'\n'*)
    echo "atomic_jsonl_append: line must not contain embedded newline" >&2; return 1 ;;
  esac
  # Pre-flight: flock(1) is not POSIX and not on stock macOS. Fail fast with
  # an actionable install instruction rather than producing a confusing
  # "flock: command not found" inside the subshell.
  if ! command -v flock >/dev/null 2>&1; then
    cat >&2 <<EOF
error: atomic_jsonl_append requires flock(1).
  macOS:  brew install util-linux  (then re-run /accelerator:migrate)
  Linux:  flock is part of util-linux, install via your package manager
  mise:   add 'flock' to your project's mise.toml
EOF
    return 127
  fi
  mkdir -p "$(dirname "$target")"
  local lockfile="${target}.lock"
  (
    flock 9
    local existing=""
    [ -f "$target" ] && existing=$(cat "$target")
    if [ -n "$existing" ]; then
      printf '%s\n%s\n' "$existing" "$line" | atomic_write "$target"
    else
      printf '%s\n' "$line" | atomic_write "$target"
    fi
  ) 9>"$lockfile"
}
```

The `flock(1)` precondition keeps the helper self-contained. The link(2) fallback explored in an earlier draft is rejected: maintaining two lock implementations (and testing both) is more cost than the dependency, and `flock` is already adjacent to the existing `mise.toml` toolchain. The fallback may be revisited in a supplementary ADR if a real portability target appears.

#### 4. Implementation: `atomic_jsonl_remove_by_key`

**File**: `scripts/atomic-common.sh`
**Changes**: Append:

```bash
# atomic_jsonl_remove_by_key <target_path> <transformation_key>
#   Rewrite <target_path> atomically, dropping every JSONL record
#   whose canonical first field "transformation_key" equals
#   <transformation_key>. Absence or empty file is a no-op.
#
#   Match is line-anchored against the canonical writer output:
#   every record begins exactly with the literal bytes
#     {"transformation_key":"<JSON-escaped-key>",
#   The writer (Phase 4 §8) is responsible for enforcing this
#   ordering; the helper assumes it.
atomic_jsonl_remove_by_key() {
  local target="$1" key="$2"
  if [ -z "$target" ] || [ -z "$key" ]; then
    echo "atomic_jsonl_remove_by_key: missing target or key" >&2; return 1
  fi
  [ ! -f "$target" ] && return 0
  local lockfile="${target}.lock"
  (
    flock 9
    # JSON-escape the key using the same rules the writer uses
    # (jsonl_json_escape — sourced from scripts/jsonl-common.sh, see below).
    local escaped_key prefix
    escaped_key=$(jsonl_json_escape "$key")
    prefix=$(printf '{"transformation_key":"%s",' "$escaped_key")
    # awk: emit lines whose first N bytes do not equal $prefix.
    # Pass the prefix via ENVIRON (NOT -v), because awk's -v assignment
    # processes backslash escape sequences in the assigned value — which
    # would re-interpret the \", \\, \n, \t inside the JSON-escaped key
    # and silently break the match. ENVIRON values are not escape-processed.
    JSONL_REMOVE_PREFIX="$prefix" \
      awk 'BEGIN{p=ENVIRON["JSONL_REMOVE_PREFIX"]} index($0,p)!=1{print}' "$target" \
      | atomic_write "$target"
  ) 9>"$lockfile"
}
```

The `jsonl_json_escape` helper is a small (10–20 line) bash function in a new file `scripts/jsonl-common.sh` (commits to one home — see Architecture finding on jsonl-helper ambivalence). The file holds `jsonl_json_escape`, `jsonl_compose_record` (Phase 4 §8), and is sourced by both `atomic-common.sh` (for `atomic_jsonl_append` / `atomic_jsonl_remove_by_key`) and `interactive-lib.sh` (for `write_session_record`). It escapes the JSON string-value subset: `\` → `\\`, `"` → `\"`, NL → `\n`, CR → `\r`, TAB → `\t`, control chars → `\u00XX`. Tested directly in `scripts/test-atomic-common.sh`. The same function is used by the Phase 4 §8 writer so that the writer's encoded key and the remover's match pattern are guaranteed identical.

### Success Criteria

#### Automated Verification

- [x] `bash scripts/test-atomic-common.sh` exits 0; the existing tests for `atomic_write` / `atomic_append_unique` / `atomic_remove_line` continue to pass alongside the new JSONL helper tests.
- [ ] `flock(1)` precondition fires with the documented install message when invoked on a `PATH` without `flock`; restore `flock` and the test then passes. *(N/A — implementation uses an mkdir-based lock primitive for portability on stock macOS; no flock dependency.)*
- [x] Concurrent-write test passes for all parametrised line sizes (100 B through 64 KiB). Per-line JSON-parse precondition uses `mise exec -- python3 -m json.tool` (consistent with the pinned shellcheck invocation pattern) — falling back to an awk-based brace/quote-balance smoke check if `mise` is unavailable.
- [ ] Race-the-crash durability test passes: every line on disk after every kill is parseable as JSON.
- [x] Prefix-collision and substring-in-other-field removal tests pass.
- [x] Escape round-trip test passes (writer → remover with `"`, `\`, tab, newline in the key) — including the explicit awk-`-v`-vs-ENVIRON regression guard.
- [x] `jsonl_json_escape` round-trips every escape-significant character.
- [x] Existing migration suite still passes.
- [ ] `mise exec -- shellcheck -x scripts/atomic-common.sh` produces no new warnings.

#### Manual Verification

- [x] Inspect a session-log file written via these helpers: one JSON object per line, trailing `\n`, no orphan temp or lockfile artefacts in the parent dir after a clean run.
- [ ] Add `flock` to `mise.toml` (alongside the existing pins for python, shellcheck, etc.) so contributors get the dependency automatically. *(N/A — implementation uses an mkdir-based lock primitive, no flock dependency.)*

---

## Phase 3: Detection + harness skeleton + coproc plumbing (handshake only)

### Overview

Add the `# INTERACTIVE: yes` header detection, the `interactive-harness.sh` library skeleton, and the runner's coproc driver. End-of-phase: a fixture interactive migration whose `migration_emit_transformations` returns immediately (zero transformations) runs end-to-end through the coproc, exchanges `INIT → READY → DONE` frames, and is recorded in `migrations-applied`.

### Changes Required

#### 1. Tests: header detection routes to interactive path

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**:

- For each existing migration (0001-0006), assert the runner classifies it as mechanical (the absence of `# INTERACTIVE: yes` in the first ~5 lines is the signal).
- Fixture: `test-fixtures/interactive/0001-empty-interactive/0001-empty-interactive.sh` — declares `# INTERACTIVE: yes` and four stub callbacks (`migration_emit_transformations` prints nothing; others are no-ops). Assert the migration completes, exit 0, and is appended to `migrations-applied`.

#### 2. Tests: protocol handshake

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**:

- Capture the wire protocol via per-test `MIGRATION_PROTOCOL_LOG_RUNNER` and `MIGRATION_PROTOCOL_LOG_MIGRATION` env vars. Assert:
  - First frame to migration: `INIT\t<resume_state_path>\t` (decisions path empty)
  - First frame from migration: `READY\t<session_log_path>`
  - Second frame from migration: `DONE`
- Test a migration that emits `FAIL\t<msg>` from a stub: assert the runner exits non-zero, prints the message to stderr, and does NOT append to `migrations-applied`.
- Test a migration that emits `MIGRATION_RESULT: no_op_pending` from its pre-flight (before sourcing the harness): assert the runner exits 0, does NOT append the migration ID to `migrations-applied`, and emits no `FAIL` diagnostic. This preserves the soft-defer semantics promised by ADR-0023 / SKILL.md `## MIGRATION_RESULT contract` for interactive migrations.
- Test a migration that emits `MIGRATION_RESULT: no_op_pending` *after* `READY`: assert this is treated as a protocol error (the soft-defer must precede the handshake — the mechanical contract is a pre-flight one).

#### 3. Implementation: header detection

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**: At the top of the apply loop, add a per-migration mechanical-vs-interactive classifier (factored to `is_interactive_migration` in `interactive-lib.sh`):

```bash
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  if is_interactive_migration "$f"; then
    if ! run_interactive_migration "$f" "$id"; then
      echo "[$id] failed" >&2; exit 1
    fi
    applied_count=$((applied_count + 1))
  else
    # Existing mechanical body (lines 184-207, unchanged)
    ...
  fi
done
```

`is_interactive_migration` is `head -5 "$f" | grep -qE '^# INTERACTIVE:[[:space:]]*yes$'`.

#### 4. Implementation: runner-side interactive lib + coproc driver

**File**: `skills/config/migrate/scripts/interactive-lib.sh` (new)
**Changes**: Define:

- `is_interactive_migration <path>` — header marker check.
- `run_interactive_migration <path> <id>` — sets up the resume-state file, forks the migration via `coproc`, sends `INIT`, drives the frame loop. End-of-loop on `DONE`: appends migration ID to `migrations-applied`. On `FAIL` or read EOF without `DONE`: returns non-zero. On `MIGRATION_RESULT: no_op_pending` emitted by the migration before `READY`: treat as soft-defer (matches the mechanical-path contract from ADR-0023 / SKILL.md `## MIGRATION_RESULT contract`), drain remaining output, do NOT append to `migrations-applied`, return 0.
- Sources `scripts/interactive-protocol.sh` (the shared escape/unescape + protocol-log helpers — single source of truth across runner and harness).
- For this phase, the driver implements only the `READY`, `DONE`, `FAIL`, and `no_op_pending` branches. Other frames are stubs filled in by later phases.

```bash
run_interactive_migration() {
  local f="$1" id="$2"
  local resume_state_path session_log
  # Deterministic resume-state path under .accelerator/state/ (not mktemp) — see
  # Cluster 10 / Safety finding on temp-file leakage. Cleanup is via explicit
  # unlink after a clean run; orphan files from prior crashes are overwritten,
  # not accumulated.
  resume_state_path="$PROJECT_ROOT/.accelerator/state/migrations-${id}-resume-state.tmp"
  mkdir -p "$(dirname "$resume_state_path")"
  build_resume_state_file "$id" > "$resume_state_path"  # Phase 6 fills this in; for now writes empty

  # Capture coproc stderr to a per-migration file so EOF-without-DONE
  # diagnostics can include the migration's last words. Cleanup on clean
  # termination; preserve for diagnostics on failure.
  local stderr_file="$PROJECT_ROOT/.accelerator/state/migrations-${id}-stderr.log"
  : > "$stderr_file"

  coproc MIG { PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
               ACCELERATOR_MIGRATION_MODE=1 MIGRATION_ID="$id" \
               bash "$f" 2>"$stderr_file"; }
  local pid=$MIG_PID
  printf 'INIT\t%s\t%s\n' \
    "$(escape_field "$resume_state_path")" \
    "$(escape_field "${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}")" >&"${MIG[1]}"

  local saw_done=0 saw_no_op_pending=0
  while IFS= read -r -u "${MIG[0]}" frame; do
    # MIGRATION_RESULT lines from the migration's pre-flight (emitted before
    # READY) preserve the mechanical-path soft-defer contract. They are NOT
    # protocol frames; they appear on stdout because the migration's pre-flight
    # writes them before sourcing the harness. We detect by exact-prefix match
    # against the documented sentinel string, not by TAB-splitting.
    case "$frame" in
      "MIGRATION_RESULT: no_op_pending")
        saw_no_op_pending=1
        # Close our writing end of the coproc input so the migration sees
        # EOF on stdin if it is waiting for runner frames (it shouldn't be,
        # since soft-defer is a pre-handshake emission, but defensive close
        # prevents deadlock if the migration is bugged). Then drain any
        # remaining output until coproc EOF — bounded by the same watchdog
        # used for FAIL/DONE so a hung migration cannot pin the runner.
        eval "exec ${MIG[1]}>&-"
        while IFS= read -r -u "${MIG[0]}" _drain; do :; done
        break
        ;;
    esac
    local type rest
    type="${frame%%$'\t'*}"
    # Bash's `${var#pattern}` returns the original string if the pattern does
    # not match, so a frame with no TAB (e.g. bare DONE or malformed READY)
    # would leak the type into rest. Explicitly empty rest in that case so
    # downstream branches don't mis-handle field-arity violations.
    if [[ "$frame" == *$'\t'* ]]; then
      rest="${frame#*$'\t'}"
    else
      rest=""
    fi
    case "$type" in
      READY)             session_log=$(unescape_field "$rest") ;;
      MECHANICAL_APPLIED) : ;;  # Phase 4
      RESUMED_APPLIED)   : ;;  # Phase 6
      RESUMED_SKIPPED)   : ;;  # Phase 6
      PROMPT)            : ;;  # Phase 4
      VALIDATE_ERR)      : ;;  # Phase 5
      RECORDED)          : ;;  # Phase 4 (write-ahead-log: persist, then emit APPLY)
      APPLIED_CONFIRM)   : ;;  # Phase 4
      DRIFT)             : ;;  # Phase 6 (remove, then emit DRIFT_CLEARED)
      DONE)              saw_done=1; break ;;
      FAIL)              echo "[$id] $(unescape_field "$rest")" >&2; wait "$pid"; rm -f "$resume_state_path"; return 1 ;;
    esac
  done

  # Explicit coproc fd close — without this, a second iteration of the apply
  # loop can collide with bash's one-coproc-per-shell limit on some versions.
  # Use `eval` to expand the array values into the literal `<&-` / `>&-`
  # redirection syntax; bash's `{varname}` form is allocate-only and does NOT
  # expand existing array elements for closing.
  eval "exec ${MIG[0]}<&- ${MIG[1]}>&-"

  # Bounded-timeout wait, with SIGTERM → SIGKILL escalation if the migration
  # hangs after FAIL or DONE. `wait` is a shell builtin that only reaps
  # children of THIS shell, so `bash -c "wait $pid"` would not work (the new
  # bash subshell has no such child). Instead, run a background watchdog that
  # escalates after 30s of inactivity, then `wait $pid` in the current shell;
  # cancel the watchdog if the wait returns first.
  (
    sleep 30
    if kill -0 "$pid" 2>/dev/null; then
      echo "[$id] migration did not exit within 30s; sending SIGTERM" >&2
      kill -TERM "$pid" 2>/dev/null || true
      sleep 1
      if kill -0 "$pid" 2>/dev/null; then
        echo "[$id] migration unresponsive to SIGTERM; escalating to SIGKILL" >&2
        kill -KILL "$pid" 2>/dev/null || true
      fi
    fi
  ) &
  local watchdog_pid=$!
  local wait_status=0
  wait "$pid" || wait_status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true
  if [ "$wait_status" -ne 0 ] || [ "$saw_done" -ne 1 ] && [ "$saw_no_op_pending" -ne 1 ]; then
    # Migration exited non-zero, or coproc closed without DONE/no_op_pending.
    # Surface the last 20 lines of captured stderr so the user sees the cause
    # rather than just "[<id>] exited without DONE". Preserve the file on
    # failure for post-mortem inspection.
    if [ -s "$stderr_file" ]; then
      echo "[$id] migration exited unexpectedly. Last stderr lines:" >&2
      tail -n 20 "$stderr_file" | sed "s/^/[$id]   /" >&2
      echo "[$id] full stderr preserved at: $stderr_file" >&2
    else
      echo "[$id] migration exited without DONE and produced no stderr." >&2
    fi
    rm -f "$resume_state_path"
    return 1
  fi

  if [ "$saw_no_op_pending" -eq 1 ]; then
    # Soft-defer: migration declined this run; do not append to ledger.
    rm -f "$resume_state_path" "$stderr_file"
    return 0
  fi
  mkdir -p "$(dirname "$STATE_FILE")"
  atomic_append_unique "$STATE_FILE" "$id"
  rm -f "$resume_state_path" "$stderr_file"
}
```

The bash 4+ requirement is enforced at the *source line* in `run-migrations.sh` (see Migration Notes), not at function entry — bash parses sourced files in their entirety, so a version check inside this function never runs on bash 3.2. Mechanical migrations on bash 3.2 skip the source entirely; interactive dispatch checks `type -t run_interactive_migration` and emits a clear bash-version error with copy-pasteable install commands if the function is undefined.

#### 5. Implementation: harness skeleton

**File**: `scripts/interactive-harness.sh` (new)
**Changes**:

- Source `$CLAUDE_PLUGIN_ROOT/scripts/interactive-protocol.sh` (the shared escape/unescape + protocol-log helpers — single source of truth, see Phase 3 §6).
- Read `INIT` frame on startup; extract `resume_state_path` and `decisions_path`.
- Emit `READY` with the migration's session-log path (`migration_session_log_path` if defined, else default).
- For this phase, `harness_run` immediately emits `DONE` if `migration_emit_transformations` produces no output. (Frame iteration and prompt logic are added in Phase 4.)
- Provide the author-facing helpers: `harness_emit_transformation`, `harness_extras_set`, `harness_extras_clear`, `harness_field`, `harness_reject` (all defined in the Migration-author surface section above).

#### 6. Shared protocol helpers + wire-protocol logging hook

**File**: `scripts/interactive-protocol.sh` (new) — single source of truth for the wire-protocol encoding shared by `interactive-lib.sh` (runner side) and `interactive-harness.sh` (migration side). Sourced from both. Closes Code Quality finding on duplicated escape helpers.

**Changes**:

- `escape_field <value>` and `unescape_field <value>` — single-pass state-machine encode/decode for `\\`, `\t`, `\n`. Ordering is well-defined (escape `\` first on emit; recognise `\\` first on receive); unit-tested in a new `scripts/test-interactive-protocol.sh` with round-trip coverage of every escape-significant character and adjacent combinations.
- `emit_frame <type> [field…]` — emit a TAB-separated frame line to stdout (caller's responsibility to redirect to coproc fd on the runner side).
- `read_frame <varname>` — read one frame from stdin into the named variable; sets `FRAME_TYPE` and `FRAME_FIELDS` globals (parallel array) for the caller.
- Maintainer-facing **protocol state-machine docstring** at the top of the file, enumerating frames, direction, fields, and the legal transition table (PROMPT → DECIDE → optional VALIDATE_ERR loop → RECORDED → APPLY → APPLIED_CONFIRM; INIT → READY → ... → DONE; FAIL aborts; DRIFT → DRIFT_CLEARED → PROMPT). Closes Documentation + Architecture findings on missing state machine.
- **Wire-protocol logging**: split into per-side env vars `MIGRATION_PROTOCOL_LOG_RUNNER` and `MIGRATION_PROTOCOL_LOG_MIGRATION` (no concurrent appends to the same file). Tests merge the two with `paste`-style timestamp interleaving at the assertion site. Closes Test Coverage finding on protocol-log interleaving and Code Quality finding on test-only instrumentation in production code (the conditional now lives in exactly two functions: `emit_frame` and `read_frame`, both `# test-only` annotated).

### Success Criteria

#### Automated Verification

- [x] Existing mechanical-path suite still passes; Phase 1 byte-identical snapshot test still passes.
- [x] Header-detection test passes: 0001-0006 classified mechanical; empty fixture classified interactive. *(Basic classification covered; marker variant tests for whitespace/case/line-6+/heredoc not implemented.)*
- [x] Handshake test passes: `INIT → READY → DONE` exchange recorded in the per-side protocol logs with expected fields.
- [x] `no_op_pending` test passes: pre-handshake emission soft-defers; post-handshake emission is a protocol error. *(Pre-handshake soft-defer verified; post-handshake error-path tests not explicitly added — the runner code handles it but lacks dedicated coverage.)*
- [ ] Multi-interactive-migration smoke test: 10 back-to-back interactive migrations in one runner invocation; no coproc-fd collision, no fd leak warnings. Closes Correctness finding on coproc fd lifecycle.
- [x] Empty-interactive fixture appends its ID to `migrations-applied`.
- [x] `FAIL` frame causes non-zero exit and skips ledger append; bounded-timeout escalation kicks in if the migration hangs after FAIL. *(FAIL handling verified; bounded-timeout escalation present in code but not exercised by an automated test.)*
- [x] Both runner paths run under stock `/bin/bash` 3.2.57 on macOS. *(Implementation pivoted away from coproc + associative arrays to two named FIFOs + parallel indexed arrays, so the interactive path no longer requires bash 4+. The previously-planned bash 4+ source gate is removed; every migrate test suite runs cleanly under bash 3.2.)*

#### Manual Verification

- [x] Run `bash run-migrations.sh` against the live corpus — user-visible output unchanged.

---

## Phase 4: Predicate routing, display rendering, `accept` control

### Overview

Implement ADR-0037 §1 (predicate routing) and §2 (display) and the `accept` verb from §4. Full `EMIT → PROMPT → DECIDE accept → RECORDED` cycle wired end-to-end.

**Shorthand in this and subsequent phases**: "the `--decisions` file" / "the decisions file" refers to `ACCELERATOR_MIGRATE_DECISIONS_FILE` (the env-var name set in Phase 1 §3). The flag-style shorthand is retained in test descriptions for readability — there is no `--decisions` CLI argument anywhere in the runner.

### Changes Required

#### 1. Fixture: predicate-firing migration

**File**: `skills/config/migrate/scripts/test-fixtures/interactive/0002-predicate/0002-predicate.sh` (new)
**Changes**: Synthetic migration with three transformations: two `band=ambiguous` (predicate fires), one `band=resolved` (mechanical). Each transformation rewrites a fixture artifact's frontmatter field. Includes seed artifacts under the fixture tree.

Uses the author-facing helpers (`harness_emit_transformation`, `harness_extras_set`, `harness_field`) — no hand-written TSV. `migration_evaluate_predicate` extracts `band` via `harness_field band`, returns 0 iff `band == ambiguous`, exit 1 (not arbitrary non-zero) for mechanical-route. `migration_apply_decision` rewrites the named field of the artifact at the given path. `migration_validate_edit` is a no-op stub (Phase 5 exercises it).

`migration_emit_transformations` also produces a `display` argument carrying a small multi-line block including a `surrounding_prose` line (exercised by AC-4).

#### 2. Tests: AC-2 predicate routing

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**:

- Predicate=true-only: all transformations route to `PROMPT`; no `MECHANICAL_APPLIED` frames.
- Predicate=false-only: all transformations route to `MECHANICAL_APPLIED`; no `PROMPT` frames.
- Mixed: assert per-frame routing via `MIGRATION_PROTOCOL_LOG_RUNNER` and `_MIGRATION` (merged at assertion site).

#### 3. Tests: AC-3 display elements

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: With decisions scripted to `accept` everything, capture the runner's user-facing output (stdout when `[ -t 1 ]`, stderr otherwise — see §7 below for the destination rule) and assert verbatim presence of:

- (a) Proposed mutation target and value (string-match the field name and proposed value).
- (b) Source location as `path:line` (the fixture emits `anchor` in `path:line` form; runner displays it verbatim).
- (c) Predicate evaluated value (the fixture emits `band=ambiguous`; runner displays it).
- (d) **Inline help line** on the prompt: `[accept | edit <new-value> | skip] > ` so the decision syntax is discoverable without consulting docs. Closes Usability finding on syntax discoverability.
- (e) **Session-log path banner** at the start of the migration: `Session log: .accelerator/state/migrations-<id>-session.jsonl` so users know where their decisions are persisted. Closes Usability finding on path discoverability.

#### 4. Tests: AC-4 declared display extras

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: The fixture's `display` argument includes a `Prose: <verbatim line>` line. Assert that line appears in captured user-facing output.

#### 5. Tests: AC-5 accept

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: Decisions file contains `accept` for each prompted transformation. Post-run:

- Each fixture artifact contains the proposed value at the right field (read and string-match).
- **Every line of the session log parses as JSON via `mise exec -- python3 -m json.tool`** (precondition before substring assertions — closes Code Quality finding on hand-rolled JSON parsing in tests). Python is already pinned in `mise.toml`; invoking through `mise exec --` matches the pinned shellcheck pattern and keeps the test suite hermetic.
- Session log contains exactly N records, each with the canonical schema `{transformation_key, schema_version: 1, outcome: "accepted", proposed_value, timestamp}` (no `user_value` key) plus any author-declared extras.
- Assert canonical field ordering: `transformation_key` is the first key (anchored prefix match against `^{"transformation_key":`) — required by `atomic_jsonl_remove_by_key`'s Phase 2 invariant.
- Explicitly assert there is no `"user_value"` substring for `accepted` outcomes.

#### 6. Implementation: harness `harness_run` body (write-ahead-log ordering)

**File**: `scripts/interactive-harness.sh`
**Changes**: After handshake, iterate `migration_emit_transformations`. For each line:

- Parse TSV into named locals.
- Call `migration_evaluate_predicate` (via stdin redirection of the TSV line).
- **Predicate false (mechanical path)**: call `migration_apply_decision <key> <path> <anchor> accept <proposed>` and emit `MECHANICAL_APPLIED <key>`. No runner persistence; the prior-run resume invariant for mechanical-only migrations is unchanged from ADR-0023.
- **Predicate true (interactive path)**: emit `PROMPT`, read `DECIDE` from stdin. On `accept`: emit `RECORDED <key> accepted <proposed> "" <extras>` and BLOCK reading the runner's reply. The runner persists the JSONL record via `atomic_jsonl_append`, then sends `APPLY <key>`. Only on receiving `APPLY` does the harness call `migration_apply_decision`. After the callback returns, emit `APPLIED_CONFIRM <key>` and proceed to the next transformation.

For this phase, only `accept` is wired into the `DECIDE` dispatch; `edit` and `skip` are stubs (handled in Phase 5). The write-ahead-log ordering is set up here and reused by Phase 5 verbs without further protocol change.

```bash
# Pseudocode for the accept branch (Phase 4):
emit_frame PROMPT "$key" "$path" "$anchor" "$proposed" "$predicate_value" "$extras_tsv" "$display_b64"
read_frame                          # DECIDE accept
emit_frame RECORDED "$key" accepted "$proposed" "" "$extras_tsv"
read_frame                          # blocks until runner persists; receives APPLY <key>
migration_apply_decision "$key" "$path" "$anchor" accept "$proposed"
emit_frame APPLIED_CONFIRM "$key"
```

#### 7. Implementation: runner display rendering

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: `handle_prompt <frame>`:

- Parses the `PROMPT` frame fields.
- Renders the three mandatory display elements + author-declared display block + inline help line to the **user-facing destination**: stdout if `[ -t 1 ]` (TTY), stderr otherwise (test capture / piped output). Closes Usability finding on stderr-only rendering. Existing runner banner output (mechanical path) already uses stdout — same convention.
- Each prompt block:

  ```text
  ── Transformation 42/140 ────────────────────────
  Proposed: frontmatter.linkage_target = "0034-foo"
  Source:   meta/work/0070-…md:23
  Predicate: band=ambiguous

  <author-declared display block (base64-decoded from display_lines_b64)>

  [accept | edit <new-value> | skip] >
  ```

- **Inline-help line frequency**: render the full `[accept | edit <new-value> | skip] > ` syntax line on (a) the first PROMPT of the session, and (b) every PROMPT that immediately follows a `VALIDATE_ERR`. On all other prompts render the compact `> ` only. Closes Usability finding on inline-help noise across long sessions while preserving discoverability where it matters.

- Emit a session-log banner *immediately before the first PROMPT* (not at migration start) — co-located with the user action that needs it; suppressed entirely on fully-resumed runs where no PROMPT ever fires: `Session log: <path>  (resume from this file by re-running /accelerator:migrate)`.
- Reads one decision line from `$ACCELERATOR_MIGRATE_DECISIONS_FILE` (or `/dev/tty` if unset), using `read -e -i "$prefill_value"` for the re-prompt-after-VALIDATE_ERR case so the user can correct their prior input rather than retype (closes Usability finding). `$prefill_value` is empty on first prompt.
- Parses into `outcome` and optional `value`. Writes `DECIDE\t<outcome>\t<value>\n` to the coproc's stdin.

#### 8. Implementation: runner session-log record composition (`jsonl_compose_record`)

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: `write_session_record <frame>`:

- Parses the `RECORDED` frame fields (`key`, `outcome`, `proposed`, `user_value`, `extras_tsv`).
- Parses `extras_tsv` into a key/value list using the `US`-delimited / `=`-separator format from the wire-protocol "Extras encoding" subsection. Each value is TSV-field-unescaped.
- Defensively rejects any extras key matching a framework-mandatory name (`transformation_key`, `schema_version`, `outcome`, `proposed_value`, `user_value`, `timestamp`); on collision, send `FAIL` to the harness and abort the migration.
- Defensively re-validates each extras key against `^[a-z][a-z0-9_]*$` on receipt (the harness already checks at `harness_extras_set` call time, but a misbehaving harness or a future framework consumer that emits RECORDED directly could ship a malformed key). On format violation, send `FAIL` with `invalid extras key '<k>'` and abort. Mirrors the reserved-key collision check; closes the asymmetric-defence finding.
- Calls `jsonl_compose_record` to build the canonical JSON line.
- Calls `atomic_jsonl_append` to persist.
- Emits `APPLY <key>` to the harness only after `atomic_jsonl_append` returns successfully.

**`jsonl_compose_record`** (a small new function in `scripts/jsonl-common.sh`, sourced by `interactive-lib.sh`; same home as `jsonl_json_escape`) takes framework-mandatory fields by name and an extras key/value list as positional pairs. It emits one canonical JSON line:

```bash
# jsonl_compose_record key=K outcome=O proposed=P [user=U] [extra_k1=v1 extra_k2=v2 …]
# Output: one valid JSON line, fields in canonical order, no extra whitespace,
# every string value escaped via jsonl_json_escape (Phase 2 §4).
```

No string slicing; no leading-brace stripping; no parser dependency. The function emits:

1. `{"transformation_key":"<escaped-key>",`
2. `"schema_version":1,`
3. `"outcome":"<accepted|edited|skipped>",`
4. `"proposed_value":"<escaped-proposed>",`
5. If `outcome == edited`: `"user_value":"<escaped-user>",`
6. `"timestamp":"<iso8601-utc>"` (from `date -u +%Y-%m-%dT%H:%M:%SZ`)
7. For each extras pair in receipt order: `,"<key>":"<escaped-value>"`
8. `}`

The canonical first-field-is-`transformation_key` rule is enforced here; `atomic_jsonl_remove_by_key`'s anchored-prefix match depends on it.

**Unit tests** for `jsonl_compose_record` in `scripts/test-atomic-common.sh` (Phase 2): assert canonical field ordering for each outcome; assert `user_value` is absent for `accepted` and `skipped` and present for `edited`; assert every escape-significant character (`"`, `\`, NL, CR, TAB, `\u00XX` control chars) round-trips through compose → JSON parse via `python3 -m json.tool`; assert reserved-extras-key collisions return non-zero with a clear error.

### Success Criteria

#### Automated Verification

- [ ] AC-2 predicate routing passes (uniform, hybrid, predicate=true-only, predicate=false-only).
- [ ] AC-3 display elements pass — three mandatory elements + inline help + session-log banner verbatim in captured user-facing output.
- [ ] AC-4 declared display extras pass — fixture-declared `Prose:` line verbatim.
- [ ] AC-5 accept passes — artifact mutation, every line of session log JSON-parses, canonical first-field invariant holds, no `user_value` for accepted.
- [ ] Write-ahead-log ordering invariant verified: every `APPLY` frame appears in the protocol log after the corresponding `RECORDED` and before any artefact mutation observable by the harness fixture.
- [ ] Per-callback contract violation tests pass: malformed TSV emission (wrong field count via direct stdout write bypassing helpers), `migration_evaluate_predicate` writing to stdout, `migration_validate_edit` writing error to stdout, `migration_apply_decision` failing with non-zero exit — each surfaces a clear error and aborts the migration without ledger append. Closes Test Coverage finding on contract violations untested.
- [ ] Existing mechanical-path and Phase 1-3 tests still pass.

#### Manual Verification

- [ ] Run the predicate fixture interactively (no `--decisions`); type `accept` at each prompt; verify display and artifact by eye.

---

## Phase 5: `edit` and `skip` controls + validation re-prompt

### Overview

Wire `edit` (with validation re-prompt) and `skip`. Accept-degraded is exercised as a special case of edit (the migration's validator accepts a looser-but-valid value).

### Changes Required

#### 1. Fixture: edit and skip variants

**File**: `skills/config/migrate/scripts/test-fixtures/interactive/0003-edit-skip/` (new)
**Changes**: Migration with three transformations and `migration_validate_edit` rejecting empty-string with `Empty value not allowed`. Includes a fourth transformation where a looser-but-valid value is in the accepted vocabulary (accept-degraded).

#### 2. Tests: AC-6 edit (including accept-degraded)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: `--decisions` containing `edit some-value`:

- Artifact post-mutation has `some-value`, not the originally-proposed value.
- Session log record is exactly `{transformation_key, outcome:"edited", user_value, proposed_value, timestamp}`. Test asserts the keys are present, no extras (beyond migration-declared `extras_tsv` fields), and `user_value` matches `some-value`.
- Accept-degraded subcase: a `--decisions` line `edit looser-but-valid-value` against the fourth transformation produces a record with `user_value` = looser value; `proposed_value` retains the original inferred value.

#### 3. Tests: AC-7 skip

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: `--decisions` line `skip` for one transformation:

- The artifact diff at that anchor before vs after is empty.
- Session-log record is exactly `{transformation_key, outcome:"skipped", proposed_value, timestamp}` (no `user_value` key).
- The runner does NOT call `migration_apply_decision` for skipped transformations (verified via a per-fixture sentinel file that the apply callback would create).

#### 4. Tests: AC-8 validation re-prompt

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: `--decisions` contains `edit ` (empty value) followed by `edit valid-value` for the same transformation:

- Stderr contains `Empty value not allowed` exactly once.
- Stderr shows the prompt rendered twice (re-prompt).
- Final artifact value is `valid-value`.
- Session log contains exactly one `edited` record for the transformation (the invalid attempt is not persisted).

#### 5. Implementation: harness `edit` and `skip` dispatch

**File**: `scripts/interactive-harness.sh`
**Changes**: Replace `read_decide_and_recurse` with an explicit `while true` loop wrapping the entire `PROMPT → DECIDE → (validate) → RECORDED → APPLY → APPLIED_CONFIRM` sequence. No recursion. The validation re-prompt re-enters the same loop body; bash stack depth is constant regardless of how many invalid attempts the user makes.

```bash
# Per-transformation handler, called from harness_run after predicate fires.
emit_frame PROMPT "$key" "$path" "$anchor" "$proposed" "$predicate_value" "$extras_tsv" "$display_b64"
while true; do
  read_frame                                  # DECIDE <outcome> <value>
  case "$decide_outcome" in
    accept)
      emit_frame RECORDED "$key" accepted "$proposed" "" "$extras_tsv"
      read_frame                              # APPLY <key> (blocks until runner persists)
      migration_apply_decision "$key" "$path" "$anchor" accept "$proposed"
      emit_frame APPLIED_CONFIRM "$key"
      break
      ;;
    skip)
      emit_frame RECORDED "$key" skipped "$proposed" "" "$extras_tsv"
      read_frame                              # APPLY <key>
      # No artefact mutation for skip; emit confirm and continue.
      emit_frame APPLIED_CONFIRM "$key"
      break
      ;;
    edit)
      local err
      if err=$(migration_validate_edit "$key" "$path" "$anchor" "$proposed" "$decide_value" 2>&1); then
        emit_frame RECORDED "$key" edited "$proposed" "$decide_value" "$extras_tsv"
        read_frame                            # APPLY <key>
        migration_apply_decision "$key" "$path" "$anchor" edit "$decide_value"
        emit_frame APPLIED_CONFIRM "$key"
        break
      else
        emit_frame VALIDATE_ERR "$err"
        continue                              # loop: read next DECIDE for same key
      fi
      ;;
    *)
      emit_frame FAIL "unknown decision outcome: $decide_outcome"
      exit 1
      ;;
  esac
done
```

#### 6. Implementation: runner `VALIDATE_ERR` handler

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: On `VALIDATE_ERR <message>`, the runner prints `$message` to the user (see Cluster 12 finding — destination is the runner's stdout when `[ -t 1 ]`, stderr otherwise), then re-renders the **same** prompt (cached from the prior `PROMPT` frame). The runner pre-fills the re-prompt input line with the rejected `value` via `read -e -i "$prior_value"` so the user can correct rather than retype (closes Usability finding). It then writes a new `DECIDE` frame. The runner uses a symmetric `while true` loop — it does not recurse.

#### 7. Tests: AC mid-stream FAIL (closes still-present Test Coverage finding)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: Fixture whose `migration_apply_decision` emits `FAIL` (via `harness_reject` or direct frame emit through the harness) on its third invocation after the harness has received `APPLY` for that key. Assert:

- Runner exits non-zero with the migration's FAIL message on stderr.
- `migrations-applied` ledger does NOT contain the migration ID (no `atomic_append_unique` call after a non-DONE termination).
- Session log on disk contains exactly 2 RECORDED records (the two completed transformations), well-formed per `mise exec -- python3 -m json.tool`.
- Re-run with full decisions for the remaining 3 keys: assert the first 2 keys emit `RESUMED_APPLIED` (no re-prompt), keys 3–5 are prompted in emission order, final session log contains all 5 records, ledger now contains the migration ID exactly once.

#### 8. Implementation: skip does not apply

**File**: `scripts/interactive-harness.sh`
**Changes**: Explicit — the `skip` branch above does NOT call `migration_apply_decision`, even after receiving `APPLY`. The `APPLY` frame's contract for `skip` is "persistence complete; you may proceed"; the runner emits `APPLY` for every `RECORDED` regardless of outcome to keep the protocol regular. Verified by the AC-7 sentinel test.

### Success Criteria

#### Automated Verification

- [ ] AC-6 edit + accept-degraded passes.
- [ ] AC-7 skip passes (artifact diff empty at skipped site; `migration_apply_decision` not called).
- [ ] AC-8 validation re-prompt passes (error printed exactly once between two prompt renders).
- [ ] All prior phase tests still pass.

#### Manual Verification

- [ ] Run a fixture interactively; type an invalid edit; observe the error and the re-prompt; type a valid edit; observe success.

---

## Phase 6: Resumability — session-log read, resume state, source-drift, full-completion ledger

### Overview

Implement ADR-0037 §3 in full. Build resume-state from existing session log; migration skips already-decided transformations; source-drift re-prompts and removes the stale record; full completion appends to `migrations-applied`.

### Changes Required

#### 1. Tests: AC-9 incremental write + SIGKILL durability (write-ahead-log)

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**:

- **Ordering invariant**: assert that for every key, the JSONL record is durable on disk **before** the harness emits `APPLIED_CONFIRM` for that key. Synchronisation: a fixture whose `migration_apply_decision` reads a single byte from a per-test FIFO before returning, and a test that opens the JSONL file between writing to the FIFO and proceeding, asserting the record is already present. Symmetric assertion: the test inspects the merged `MIGRATION_PROTOCOL_LOG_RUNNER` + `_MIGRATION` log to confirm `APPLY` frames appear after the corresponding `RECORDED` frame and never before.
- **Crash between RECORDED and APPLY**: kill the runner after the `RECORDED` frame is logged but before it can issue `APPLY`. Assert the JSONL record is on disk (the runner persisted before signalling). Re-run: assert the resume path takes `RESUMED_APPLIED` for the key (no re-mutation) — verifies the write-ahead-log invariant under crash.
- **Crash between APPLY and APPLIED_CONFIRM**: kill the harness after `APPLY` is received but before `APPLIED_CONFIRM` is emitted. Re-run: assert the resume path takes `RESUMED_APPLIED` and does not re-call `migration_apply_decision` (verified via per-fixture sentinel).
- **SIGKILL race**: launch the runner with a `--decisions` file containing N lines, send SIGKILL at randomised intervals over a wall-clock budget. After each kill, assert every line on disk is well-formed JSON and that the resume path correctly identifies decided vs un-decided keys.

#### 2. Tests: AC-10 partial-run resume

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: Pre-create a session log with N records (3 of 5 fixture transformations, mixed outcomes including one `skipped`). Run with a `--decisions` file containing zero lines for the first three transformations and `accept` lines for the remaining two:

- The merged protocol log (`MIGRATION_PROTOCOL_LOG_RUNNER` + `_MIGRATION`) shows for the first 3 keys: `RESUMED_APPLIED` for accepted/edited records, `RESUMED_SKIPPED` for skipped records, no `PROMPT`, no `APPLY`, no `APPLIED_CONFIRM`.
- The fourth and fifth transformations are prompted in emission order.
- Final session log contains all 5 records.

Additional resume edge-case tests in this phase (closing Test Coverage findings):

- **Partial JSONL line at EOF**: pre-create a session log whose last line is truncated mid-record (simulating a crash before atomic_jsonl_append's rename committed — should be impossible per Phase 2, but defensive). Assert the resume parser either skips the partial line with a warning or fails fast with a clear error message.
- **Unknown outcome value**: a session-log record whose `outcome` is not in `{accepted, edited, skipped}`. Assert the resume parser fails fast with a clear error message (no silent fallthrough).
- **Orphan resume record**: a session-log record for a key the migration's current emission no longer produces (source mutation dropped the key entirely). Assert the orphan is reported in stderr as `[migration <id>] orphan resume record for key <k> — record retained, key not re-emitted` and the migration completes normally.
- **`user_value` containing TSV/JSON-escape-significant characters**: a record with `user_value` containing `\t`, `\n`, `\\`, `"`, `|`. Re-run; assert the resume path correctly matches `proposed_value` and emits `RESUMED_APPLIED`.
- **`migration_verify_applied` detects partial/missing mutation** (closes Safety finding on persist-before-APPLY window): fixture migration declares `migration_verify_applied` that reads the artefact and returns 0 iff the recorded value is present. Pre-create a session log marking key `k1` as `accepted` but leave the fixture artefact un-mutated (simulating a crash between persist and APPLY). Re-run: assert (a) `migration_verify_applied k1 …` is called, (b) `DRIFT k1` frame emitted, (c) stale record removed from session log, (d) user is re-prompted for `k1` and the final artefact reflects the new decision.
- **`migration_verify_applied` not declared → trust recorded outcome**: same scenario as above but the fixture does NOT declare `migration_verify_applied`. Re-run: assert `RESUMED_APPLIED k1` is emitted, no re-prompt, no re-mutation (residual risk acknowledged in SKILL.md).

#### 3. Tests: AC-11 full-run idempotency

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: First run decides every transformation. Snapshot the artifact tree. Second run with empty `--decisions`:

- Zero prompts.
- `diff -r` against the snapshot is empty.
- `migrations-applied` contains the migration ID exactly once (`grep -c "^$id$" <ledger>` equals 1).

#### 4. Tests: AC-12 source-drift

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**: First run accepts transformation `k1` with `proposed=v1`. Mutate the fixture source (e.g. edit a parser-input file) so the migration on re-run proposes `k1=v2`. Re-run:

- The merged protocol log shows the sequence `DRIFT k1` → `DRIFT_CLEARED k1` → `PROMPT k1 … v2 …`.
- Session log no longer contains the `v1` record; contains a new record with `v2`.
- The final artifact reflects whatever value the second-run decision specified (`--decisions` provides `accept`).

#### 5. Implementation: runner `build_resume_state_file` (JSONL-aware extraction)

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: For each session-log line for the given migration ID, emit one line to the resume-state temp file in the format the harness consumes — TAB-separated, with the same `escape_field` rules the wire protocol uses (so values containing TAB/newline/backslash round-trip cleanly):

```
RESUMED<TAB><key><TAB><outcome><TAB><proposed_escaped><TAB><user_value_escaped>
```

The `|`-delimited internal packing in the earlier draft is removed — values can legitimately contain `|`, and that delimiter is gratuitous when the harness already needs the wire-protocol escape helpers loaded.

Field-extraction uses **awk with full JSON-string-unescape**, not naive regex. The framework-mandatory fields (`transformation_key`, `outcome`, `proposed_value`, `user_value`) can contain `\"`, `\\`, `\n`, `\r`, `\t`, `\uXXXX` per JSON; a sed-style `"key":"[^"]*"` extractor would truncate at the first embedded `\"`. The extractor is a small awk function (≤30 lines) that walks each JSONL line character-by-character, handling escape sequences in the standard JSON way. The same awk function is reused by the orphan-record detection in resume.

Schema-version handling: every line's `schema_version` is checked against the runner's supported set (currently `{1}`). Unknown versions emit a clear stderr warning and refuse resume with an actionable recovery instruction: `unknown schema_version <N> in session log; this build supports {1}. To discard the session and re-prompt, run: rm <session-log-path>`. The recovery instruction uses the documented discard command from the Phase 1 §4 pre-flight message — no new CLI flag is introduced.

**Schema-version upgrade policy**: the runner accepts a *set* of schema_versions declared in code; supplementary ADRs per ADR-0037 §5 MAY widen this set (e.g., add `2` while continuing to accept `1`) but MUST NOT narrow it without a deprecation period. Authors of future hook-declaring migrations may upgrade their session-log records to a newer schema only after the supplementary ADR lands and the runner has been bumped. The version field's canonical position-2 slot is chosen so a forward-compat runner can identify and skip unsupported lines without parsing the rest of the record.

#### 6. Implementation: harness resume-state load and replay (parallel arrays + new frame vocabulary)

**File**: `scripts/interactive-harness.sh`
**Changes**: After `INIT`, read the resume-state file into three parallel associative arrays — `RESUME_OUTCOME`, `RESUME_PROPOSED`, `RESUME_USER` — keyed by `$key`. Parallel arrays avoid the pack/unpack step and remove the delimiter-collision risk of the earlier `|`-packed scheme.

During the emit loop, for each transformation:

```bash
if [ -n "${RESUME_OUTCOME[$key]:-}" ]; then
  local r_outcome="${RESUME_OUTCOME[$key]}"
  local r_proposed="${RESUME_PROPOSED[$key]}"
  if [ "$r_proposed" = "$proposed" ]; then
    local verify_failed=0
    if [ "$r_outcome" = "accepted" ] || [ "$r_outcome" = "edited" ]; then
      # If the migration declared migration_verify_applied, call it to detect
      # partial/missing application from a prior crashed run. Default (no
      # callback) is to trust the recorded outcome — VCS revert is the
      # universal recovery path per ADR-0023.
      if declare -F migration_verify_applied >/dev/null; then
        local r_user="${RESUME_USER[$key]:-}"
        if ! migration_verify_applied "$key" "$path" "$anchor" "$r_outcome" "$r_proposed" "$r_user"; then
          verify_failed=1
        fi
      fi
    fi
    if [ "$verify_failed" -eq 1 ]; then
      # Recorded mutation is absent or partial — discard stale record via
      # DRIFT, then re-prompt through the normal path.
      emit_frame DRIFT "$key"
      read_frame                              # blocks until DRIFT_CLEARED <key>
      # Fall through to predicate evaluation + prompt below.
    else
      case "$r_outcome" in
        accepted|edited)
          # Already applied on prior run. Runner does NOT re-emit APPLY, harness
          # does NOT call migration_apply_decision.
          emit_frame RESUMED_APPLIED "$key"
          ;;
        skipped)
          emit_frame RESUMED_SKIPPED "$key"
          ;;
        *)
          emit_frame FAIL "unknown resume outcome '$r_outcome' for key $key"
          exit 1
          ;;
      esac
      continue
    fi
    # verify_failed path falls through to predicate evaluation + prompt below.
  else
    emit_frame DRIFT "$key"
    read_frame                              # blocks until DRIFT_CLEARED <key>
    # Fall through to predicate evaluation + prompt for fresh decision.
  fi
fi
# Normal predicate evaluation + prompt for un-resumed key.
```

The DRIFT path explicitly blocks reading `DRIFT_CLEARED` before falling through, so the harness never re-prompts for a key while the runner is still removing the stale record from the session log (prevents a race where the new PROMPT and the old record briefly coexist).

#### 7. Implementation: runner `DRIFT` handler

**File**: `skills/config/migrate/scripts/interactive-lib.sh`
**Changes**: On `DRIFT <key>`, the runner calls `atomic_jsonl_remove_by_key "$session_log" "$key"` (which now uses the anchored-prefix awk match from Phase 2, not substring grep). When the removal returns, the runner emits `DRIFT_CLEARED <key>` to the harness. The harness then proceeds to emit a fresh `PROMPT` for the same key with the new proposed value, routed through the normal `RECORDED → APPLY → APPLIED_CONFIRM` write-ahead-log cycle.

Resume-correctness note (closes Safety finding on DRIFT-vs-partial-apply): because the write-ahead-log invariant guarantees the runner persists before the harness mutates, a partial-apply crash can no longer manifest as drift. If the harness died between `APPLY` and `APPLIED_CONFIRM` on a prior run, the next run's `RESUMED_APPLIED` path takes effect (the durable record's `proposed_value` matches the live emission unless the migration source has genuinely changed). Therefore DRIFT now unambiguously means "the migration source changed" — not "the previous run partially applied".

#### 8. Implementation: full-completion ledger append

**File**: Already present in Phase 3's stub. Confirm semantics:

- The runner appends migration ID to `migrations-applied` **only** when `DONE` is received and `wait $pid` returns 0.
- A partial run that ends without `DONE` (SIGKILL, FAIL, or coproc EOF) does NOT append. The migration stays pending; the next run re-enters via resume state.

### Success Criteria

#### Automated Verification

- [ ] AC-9 write-ahead-log ordering invariant verified (RECORDED → APPLY → APPLIED_CONFIRM; persistence-before-mutation).
- [ ] AC-9 crash-window tests pass: kill between RECORDED and APPLY (record durable, resume via RESUMED_APPLIED); kill between APPLY and APPLIED_CONFIRM (no re-mutation on resume).
- [ ] AC-9 SIGKILL race test passes: every line on disk well-formed JSON across randomised kill timing.
- [ ] AC-10 partial-run resume passes (mixed `accepted`/`edited`/`skipped` resume outcomes).
- [ ] AC-10 resume edge-case tests pass: partial JSONL line at EOF (defensive fail-fast), unknown outcome value (fail-fast), orphan resume record (warning + completion), `user_value` with escape-significant characters (round-trips correctly).
- [ ] AC-11 full-run idempotency passes (`diff -r` empty; ledger count 1).
- [ ] AC-12 source-drift passes (DRIFT → DRIFT_CLEARED → PROMPT sequence in protocol log; stale record gone via anchored-prefix awk match; new record present).
- [ ] All prior phase tests still pass.

#### Manual Verification

- [ ] Run a fixture interactively, interrupt with SIGINT mid-loop, re-run — observe resume from the correct un-decided transformation.

---

## Phase 7: SKILL.md documentation + worked-example drift test

### Overview

Document the contract end-to-end and CI-test the worked example so docs cannot drift from implementation. AC-13 closes here.

### Changes Required

#### 1. SKILL.md update

**File**: `skills/config/migrate/SKILL.md`
**Changes**: Between the existing `## MIGRATION_RESULT contract` section and `## Executing the migration`, add `## Optional interactive contract`:

- **API reference at a glance** (table placed before the prose; the scan-not-read anchor for first-time authors). Rows: each of the 5 callbacks + the 5 helpers + `harness_run`. Columns: Name, When called, Args, Return contract, Side effects. Example rows:

  | Name | When called | Args | Returns | Side effects |
  |---|---|---|---|---|
  | `migration_emit_transformations` | once at handshake, by harness | none (writes to internal harness stream) | (no return contract; uses `harness_emit_transformation` to emit each record) | accumulates extras via `harness_extras_set` (reset after each emission) |
  | `migration_evaluate_predicate` | per transformation | positional: key path anchor proposed predicate_value extras_tsv | exit 0 = prompt; exit 1 = mechanical; other non-zero = FAIL | none |
  | `migration_validate_edit` | per edit decision | positional: key path anchor proposed user_value | exit 0 = valid; non-zero (typically via `harness_reject`) = invalid | should write error to stderr |
  | `migration_apply_decision` | per accepted/edited transformation, AFTER runner persists | positional: key path anchor decision value | exit 0 = success; non-zero = FAIL aborts migration | mutates the artefact at (path, anchor) |
  | `migration_session_log_path` (optional) | once at handshake | none | path string on stdout | none |
  | `migration_verify_applied` (optional) | per resumed-accepted/edited key | positional: key path anchor recorded_outcome recorded_proposed [recorded_user_value] | exit 0 = mutation present; non-zero = absent → re-prompt | none |
  | `harness_emit_transformation` | inside `migration_emit_transformations` | named: key= path= anchor= proposed= predicate_value= display= | (helper) | emits one TSV transformation; auto-clears extras after |
  | `harness_extras_set` | inside `migration_emit_transformations` | positional: key value | (helper) | accumulates one extras pair for the next emission |
  | `harness_extras_clear` | optional inside `migration_emit_transformations` | none | (helper) | drops accumulated extras |
  | `harness_field` | inside `migration_evaluate_predicate` (TSV on stdin) | positional: field name | value on stdout | none |
  | `harness_reject` | inside `migration_validate_edit` | positional: message | (helper, exits non-zero) | prints `[interactive] <message>` to stderr |
  | `harness_run` | last line of migration | none | (drives the protocol loop; returns when DONE is emitted) | the entire interactive protocol |

- **Header marker**: a migration declares the hook by adding `# INTERACTIVE: yes` to its header. The marker is matched within the first 5 *comment* lines following the shebang. Ship an explicit template skeleton in SKILL.md showing the marker on line 4 (line 1: shebang, line 2: `# DESCRIPTION: …`, line 3: `# INTERACTIVE: yes`, line 4: blank, line 5: `set -euo pipefail`) — closes Compatibility finding on the marker colliding with `set -euo pipefail` position.
- **Prerequisites**: bash 4.0+ required for the interactive path. (Mechanical migrations remain bash 3.2 compatible.)
- **Author-facing helpers** (defined in `interactive-harness.sh`, sourced by every hook-declaring migration): `harness_emit_transformation`, `harness_extras_set`, `harness_extras_clear`, `harness_field`, `harness_reject`. Authors NEVER hand-write TSV positional fields, base64-encode display blocks, or hand-write JSON.
- **Required callbacks** (signatures and contracts): `migration_emit_transformations`, `migration_evaluate_predicate`, `migration_validate_edit`, `migration_apply_decision`. Optional: `migration_session_log_path`, `migration_verify_applied` (resume-integrity check — recommended for migrations whose mutations are not idempotent; see Write-ahead-log ordering above). Document the exit-status contract for each: `migration_evaluate_predicate` exits 0 for prompt-route, 1 for mechanical-route, any other non-zero is a hard failure; `migration_apply_decision` exits 0 on success, non-zero aborts the migration with the callback's stderr surfaced as the FAIL message; `migration_verify_applied` exits 0 if the recorded mutation is present in the artefact, non-zero if absent or partial (causing re-prompt).
- **`harness_run`** at the end of the script hands control to the framework's runtime loop.
- **Field-escaping rules (verbatim)**: TSV fields are TAB-separated; within a field, literal characters are escaped as `\\` → `\\\\`, TAB → `\\t`, NL → `\\n`. Encode order: backslash first, then TAB, then NL. Decode is a single-pass state machine. The author-facing helpers handle this automatically; authors only need this if writing test harnesses or debugging.
- **Extras encoding**: `harness_extras_set <key> <value>` pairs are accumulated and emitted as the `extras_tsv` wire field; keys must match `^[a-z][a-z0-9_]*$` and may NOT collide with framework-mandatory names (`transformation_key`, `schema_version`, `outcome`, `proposed_value`, `user_value`, `timestamp`). Values are any byte string.
- **Runner guarantees (ADR-0037 §§1-4)**:
  - §1 predicate routing (uniform vs hybrid).
  - §2 mandatory display elements (proposed transformation, source location, predicate evaluated value) plus author-declared display lines.
  - §3 resumability — session log written incrementally, durably persisted before any artefact mutation (write-ahead-log invariant — see "Write-ahead-log ordering" below), re-read on re-entry, full completion appends to `migrations-applied`.
  - §4 accept/edit/skip artifact and session-log effects.
- **Write-ahead-log ordering** (closes Safety finding): for every prompted transformation, the runner persists the JSONL record durably to disk *before* the harness performs any artefact mutation. The harness blocks for an `APPLY` signal from the runner after emitting `RECORDED`; only then does it call `migration_apply_decision`. This means an interrupted run cannot lose decisions and cannot leave the artefact mutated without a corresponding record. Note for authors: `migration_apply_decision` is not called a second time on resume — write it to be the sole point of artefact mutation per transformation.
- **Runner-level decisions** (story-promoted, **runner-level not ADR-0037 §§1-4 framework primitives**; disagreement from a future hook-declaring migration is the signal to promote to a supplementary ADR per ADR-0037 §5):
  - Source-drift: re-prompt with new value, old record discarded via `atomic_jsonl_remove_by_key`. After the write-ahead-log invariant, DRIFT unambiguously means "the migration source changed" — partial-apply crashes resume via `RESUMED_APPLIED`, not DRIFT.
  - Transformation ordering: emission order preserved by the harness's iteration of `migration_emit_transformations`.
  - Transformation-key schema: emit-line `key` field; if migration leaves it empty, the runner synthesises `${path}:${anchor}` (documented as a fallback, not a primary contract).
  - **Sticky skip semantics**: a transformation skipped on a prior run remains skipped on every subsequent run, even if the predicate has since changed (e.g., the key became unambiguous → would route mechanically). The resume path takes `RESUMED_SKIPPED` whenever `proposed_value` matches; the predicate is not re-evaluated. Rationale: the user's prior skip was an explicit decision against this transformation; silently mechanically applying it later would override that consent. Authors who want predicate-changed-keys re-evaluated should design their predicate to be stable across runs (i.e., not narrow over time). Users who want to re-prompt a previously-skipped key delete the corresponding session-log line and re-run.
- **Session log**: path convention `.accelerator/state/migrations-<id>-session.jsonl`, framework-mandatory record fields in canonical order (`transformation_key` first, then `schema_version`, then `outcome` / `proposed_value` / (optional `user_value`) / `timestamp`, then author-declared extras), one JSON object per line, `schema_version: 1`. The runner refuses to resume from a log with an unknown `schema_version` and prints a clear recovery instruction. **Cleanup policy**: the session log is retained as an audit artefact after full completion (mirrors the `migrations-applied` ledger); users may delete it manually if they wish, but the framework does not.
- **Worked example**: a complete synthetic migration whose minimum coverage is:
  - At least one predicate=true transformation (prompted) AND at least one predicate=false transformation (mechanical).
  - The transcript demonstrates `accept`, `edit <value>`, `skip`, and at least one `VALIDATE_ERR` cycle (invalid edit → error message → re-prompt → valid edit).
  - The `extras_tsv` field carries at least one non-trivial value (multi-line via `\n` escaping, demonstrating the helper's encoding).
  - The migration's `migration_apply_decision` mutates a realistic artefact shape (e.g. a YAML frontmatter field).
  - Bracketed by `<!-- @transcript-start -->` / `<!-- @transcript-end -->` and `<!-- @session-log-start -->` / `<!-- @session-log-end -->` extraction markers.

Also amend the existing `## Per-migration contract` MAY bullet list with a pointer to the new section.

Update `## Cross-references`:
- Fix the existing broken `meta/decisions/ADR-0023-migration-framework.md` → `meta/decisions/ADR-0023-meta-directory-migration-framework.md` (the actual file path).
- Add `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`.
- Add `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`.

#### 2. Worked-example fixture

**File**: `skills/config/migrate/scripts/test-fixtures/interactive/doc-example/` (new)
**Changes**: Mirrors the SKILL.md worked example exactly: the migration script, the seed artifact, the `ACCELERATOR_MIGRATE_DECISIONS_FILE` script. Runs cleanly under the existing harness. Covers every item in the worked-example minimum-coverage list above.

#### 3. Drift-test (AC-13) — deterministic sandbox + full redaction set

**File**: `skills/config/migrate/scripts/test-migrate-interactive.sh`
**Changes**:

- Extract the transcript section from SKILL.md using the `@transcript-start`/`@transcript-end` markers; pre-assert the extracted region is non-empty and contains the sentinel substring `Proposed:` (transcript) / `"transformation_key"` (session-log excerpt) before diffing — guards against marker-typo silent passes (closes Test Coverage finding on empty-marker guard).
- Extract the session-log excerpt using `@session-log-start`/`@session-log-end`.
- Run the doc-example fixture in a **deterministic sandbox**: a fresh `mktemp -d` is created and `PROJECT_ROOT` is set to that path; both the fixture's seed artefact and the migration are copied into the sandbox. The sandbox path is captured as `$SANDBOX_PATH`.
- After capturing stdout/stderr and the on-disk session log, apply the full redaction set:
  - `s|$SANDBOX_PATH|<SANDBOX>|g` — sandbox tempdir occurrences (every absolute path embedded in INIT, resume-state path, session-log path).
  - `s|/var/folders/[^/]*/[^/]*/T/[^/[:space:]]*|<TMPDIR>|g` — any macOS `mktemp` paths that leaked outside `$SANDBOX_PATH`.
  - `s|/tmp/[^/[:space:]]*|<TMPDIR>|g` — Linux tempdir equivalents.
  - `s/"timestamp":"[^"]*"/"timestamp":"<REDACTED>"/g` — JSON timestamps in session log.
  - `s/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z/<REDACTED>/g` — human-readable ISO 8601 timestamps in the transcript.
  - `s/pid=[0-9]+/pid=<REDACTED>/g` — any PID references in protocol-log lines (defensive).
- `diff` the redacted captures against the redacted extracted regions. Both diffs must be empty.
- **Deterministic-pass gate**: the test runs the fixture 5 times in fresh sandboxes and asserts all 5 captures are byte-identical post-redaction. Catches any remaining non-determinism in the redaction set.

### Success Criteria

#### Automated Verification

- [ ] AC-13 drift test passes (transcript and session-log excerpt match byte-for-byte after the full redaction set).
- [ ] AC-13 determinism gate: 5 consecutive runs in fresh sandboxes produce byte-identical post-redaction captures.
- [ ] Marker pre-assertion catches the missing-marker case (test the test: temporarily remove a marker pair from SKILL.md, assert AC-13 fails with a clear "extracted region empty" message rather than silently passing).
- [ ] All prior phase tests still pass.
- [ ] `markdownlint` (if configured) clean on the updated SKILL.md.

#### Manual Verification

- [ ] **External self-sufficiency check**: a reviewer who has not read ADR-0037 or ADR-0038 (the natural candidate is the author of work item 0070, since 0070 will consume the SKILL.md as its source of truth) reads only the updated SKILL.md and authors a stub hook-declaring migration matching the worked-example skeleton. Success = the stub runs cleanly under the harness within ~30 minutes. If the reviewer needs to consult ADR-0037 or ADR-0038, the section is insufficient and must be revised before AC-13 is marked complete. Closes Documentation finding on self-sufficiency claim being unmeasurable.
- [ ] The worked example renders cleanly in the rendered SKILL.md.

---

## Testing Strategy

### Unit Tests

- `scripts/test-atomic-common.sh` — `atomic_jsonl_append` (atomicity, append semantics, concurrent safety, SIGKILL durability) and `atomic_jsonl_remove_by_key` (presence, absence, multiplicity).
- Frame parsing, escape/unescape — exercised via protocol-frame assertions in integration tests rather than isolated units.

### Integration Tests

- `skills/config/migrate/scripts/test-migrate-interactive.sh` — every AC has a named test. All non-interactive scripting goes through `ACCELERATOR_MIGRATE_DECISIONS_FILE`. Protocol-frame assertions use the per-side `MIGRATION_PROTOCOL_LOG_RUNNER` / `MIGRATION_PROTOCOL_LOG_MIGRATION` files (split per the Phase 3 §6 design) merged at the assertion site.
- `skills/config/migrate/scripts/test-migrate.sh` — runs unchanged. Its continued passage is the AC-1 regression net.

### Manual Testing Steps

1. Clone a fresh consumer repo with pending mechanical migrations; run `/accelerator:migrate`; verify identical output to current.
2. Drop a synthetic interactive fixture into the migrations directory; run; type `accept`/`edit foo`/`skip` at the prompts; verify artifact, session log, and applied ledger.
3. SIGINT mid-prompt-loop; re-run; verify resume at the correct transformation.
4. Edit the fixture source so a recorded transformation's proposed value drifts; re-run; verify drift-handling.
5. SIGKILL between prompts; verify the prior decision is durably on disk.

## Performance Considerations

- Session-log read on re-entry is O(N) in number of decisions; for the projected ~140 ambiguous inferences of the first consumer (ADR-0038), sub-millisecond.
- Per-decision durability: `atomic_jsonl_append` uses `flock`-guarded temp-then-rename; the rename system call is the durability point. Cost per decision is one read of the existing file + one fsync-equivalent (rename + parent dir update) — a few ms on local filesystems, negligible relative to user-prompt latency.
- The `coproc` adds no extra fork-cost in the steady state: the migration is a single subprocess for its lifetime, same as today.
- Pipe-buffer deadlock risk: protocol is strictly request/response on each side of the wire. The write-ahead-log path (RECORDED → APPLY → APPLIED_CONFIRM) is three small frames; each side reads before writing the next outbound frame, so pipe-buffer overflow is not reachable for realistic frame sizes.
- **Per-prompted-transformation IPC cost**: four runner↔harness round-trips (PROMPT→DECIDE, RECORDED→APPLY, APPLIED_CONFIRM one-way), plus an extra DRIFT→DRIFT_CLEARED for drifted keys. For the projected ~140 prompts of the first consumer this is ~560 frame exchanges over local pipes — sub-millisecond total IPC cost, dominated by user-prompt latency. A future batch-decide extension (per ADR-0037 §5) would need to relax the write-ahead-log invariant only across decisions explicitly marked idempotent, preserving the per-decision guarantee for the general case.
- **Prompt-loop runaway protection**: a buggy `migration_emit_transformations` that emits an unbounded transformation stream would prompt indefinitely. The harness emits a progress prefix on every PROMPT (`[N/M] prompting…` where `M` is the total emit count buffered before iteration begins), and the runner emits a warning to stderr when the consumed prompt count exceeds a soft threshold of 1000 (`[interactive] warning: prompt count exceeded 1000 — possible runaway emit`). No hard cap is imposed; SIGINT remains the operator escape hatch.

## Migration Notes

- Existing migrations (0001-0006) are unaffected: no `# INTERACTIVE: yes` marker, no behavioural change. The mechanical-path invocation (`bash "$f" >"$STDOUT_FILE" 2>&1`) is byte-identical, verified by the Phase 1 snapshot `diff -r` against a pre-change capture (see Phase 1 §1).
- **bash 4.0+ required for the interactive path**: `coproc`, `declare -A`, and parallel associative arrays are bash-4-only. **The version gate must be placed at the `source interactive-lib.sh` line in `run-migrations.sh`, not at function-entry inside `interactive-lib.sh`**: bash parses sourced files in their entirety before running any function defined in them, so on bash 3.2 the file's `coproc` and `declare -A` produce a parse-time syntax error before any version-check function can run. The correct pattern is:

  ```bash
  # In run-migrations.sh, before the apply loop:
  if [ "${BASH_VERSINFO[0]:-0}" -ge 4 ]; then
    source "$PLUGIN_ROOT/skills/config/migrate/scripts/interactive-lib.sh"
  fi
  # In the apply loop, when an interactive migration is detected:
  if [ "$(type -t run_interactive_migration)" != "function" ]; then
    cat >&2 <<EOF
  error: this migration declares '# INTERACTIVE: yes' but the interactive path
  requires bash 4.0+ (current: \$BASH_VERSION). Mechanical migrations are
  unaffected. To enable interactive migrations, install a newer bash:
    macOS:  brew install bash       (then re-run /accelerator:migrate)
    mise:   mise use bash@5         (project-scoped)
  EOF
    exit 1
  fi
  ```

  Mechanical migrations on bash 3.2 are entirely unaffected — `interactive-lib.sh` is never sourced. Work item 0070 cannot ship until this gating is in place.
- **Session-log path** (`.accelerator/state/migrations-<id>-session.jsonl`): the dirty-tree pre-flight covers `.accelerator/`, so users with an in-flight session log will be guided to commit-or-discard before re-running. The pre-flight emits a *distinct, named* message when a session log file is detected, naming the resume command and the explicit discard command (rather than the generic "dirty tree" error) — closes Safety finding on silent decision loss (see Phase 1 §5 below).
- **Resume-state temp file** (`.accelerator/state/migrations-<id>-resume-state.tmp`): deterministic path, not `mktemp`. Overwritten on each re-run; explicitly unlinked after clean completion or soft-defer. No long-term accumulation on developer workstations.
- **Schema versioning**: every JSONL record carries `schema_version: 1`. Future schema changes are introduced via a supplementary ADR per ADR-0037 §5; the runner refuses to resume from a log with an unknown `schema_version` and prints a clear recovery instruction (no silent misinterpretation of older formats).
- The new harness library `scripts/interactive-harness.sh` and the shared protocol library `scripts/interactive-protocol.sh` are bundled with the plugin and sourced via `$CLAUDE_PLUGIN_ROOT/scripts/...` — same pattern as `atomic-common.sh`.

## References

- Original work item: `meta/work/0069-migration-framework-interactive-validation-hooks.md`
- Codebase research: `meta/research/codebase/2026-05-30-0069-migration-framework-interactive-validation-hooks.md`
- ADR-0037 (framework contract): `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
- ADR-0038 (first consumer's parameterisation): `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`
- ADR-0023 (mechanical-default contract): `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Current runner: `skills/config/migrate/scripts/run-migrations.sh`
- Atomic helpers: `scripts/atomic-common.sh`
- Test harness: `skills/config/migrate/scripts/test-migrate.sh`
- Downstream consumer: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
