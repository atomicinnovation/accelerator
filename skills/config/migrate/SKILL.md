---
name: migrate
description: Apply pending Accelerator meta-directory migrations to bring a repo into line with the latest plugin schema. Destructive by default but guarded — refuses to run on a dirty working tree and prints a one-line preview per pending migration before applying.
allowed-tools: 
  - Read
  - Write
  - Edit
  - Bash
---

> **Warning: this skill rewrites files in `meta/` and `.claude/accelerator*.md`.** Recovery is via VCS revert. Before running, ensure your repo is committed and you understand what each pending migration does. The safety guards (clean-tree check, preview) exist to give you a moment to stop — they are not a substitute for understanding the changes.

## When to invoke

Run `/accelerator:migrate` after upgrading the Accelerator plugin to a version that bundles new migrations. The SessionStart hook will tell you when this is needed:

```
[accelerator] .accelerator/state/migrations-applied is behind the plugin
(highest applied: 0001-rename-tickets-to-work; highest available: 0002-...).
Run /accelerator:migrate to bring it up to date.
```

You can also run it proactively — if no migrations are pending, it prints `No pending migrations.` and exits cleanly.

**Upgrade sequence.** After pulling a new plugin version, run `/accelerator:migrate` before invoking any skill that reads or writes paths affected by pending migrations. Skills do not gate themselves on pending migrations; the SessionStart hook only warns when migrations are pending. Running write-side skills (e.g., `/accelerator:research-codebase`) between the plugin upgrade and the migration may produce results written to or read from new-default paths that do not yet exist on disk.

## How it works

The driver script `skills/config/migrate/scripts/run-migrations.sh` orchestrates the migration lifecycle:

1. **Clean-tree pre-flight.** Checks `meta/`, `.claude/accelerator*.md`, and `.accelerator/` for uncommitted changes. Aborts if any are found. Set `ACCELERATOR_MIGRATE_FORCE=1` to bypass (advanced users only).
2. **Read state.** Loads `.accelerator/state/migrations-applied` and `.accelerator/state/migrations-skipped` — newline-delimited lists of migration IDs. If either file is absent, its set is empty. Unknown IDs (from a newer plugin version) are preserved verbatim and warned about. An ID appearing in both files triggers a warning; applied takes precedence.
3. **Discover migrations.** Globs `skills/config/migrate/migrations/[0-9][0-9][0-9][0-9]-*.sh` in sorted order. The directory can be overridden via `ACCELERATOR_MIGRATIONS_DIR` (used in tests).
4. **Compute pending.** A migration is pending if its ID is in neither the applied nor the skipped set.
5. **Preview banner.** Prints one line per pending migration — `<ID> — <description>` — with a per-migration skip hint (`--skip <id>`). If nothing is pending, prints `No pending migrations.` (plus any skipped names) and exits 0 immediately.
6. **Apply in order.** For each pending migration: runs it with `PROJECT_ROOT` exported; on success atomically appends its ID to `.accelerator/state/migrations-applied`; on failure prints the migration's output to stderr, exits 1, and leaves the state file at the last successful migration. If the migration emits `MIGRATION_RESULT: no_op_pending` on stdout, it is treated as a soft skip — the migration stays pending and will be retried on future runs.
7. **Summary.** Prints counts of applied, skipped, and pending (no-op) migrations.

## Per-migration contract

Each migration script under `migrations/` must:

- Start with `#!/usr/bin/env bash` then `# DESCRIPTION: <short imperative description>` on line 2
- Receive `PROJECT_ROOT` as an exported env var pointing to the consumer repo
- Self-detect and exit 0 on already-applied state (idempotent)
- Use atomic write patterns (source `scripts/atomic-common.sh` for `atomic_write`, `atomic_append_unique`, `atomic_remove_line`) for any file rewrites
- NOT honour any `DRY_RUN` env var — this framework has no dry-run mode
- Optionally emit `MIGRATION_RESULT: no_op_pending` on stdout to signal that preconditions are not met and the migration should remain pending (retried on future runs)

## State file format

`.accelerator/state/migrations-applied` contains one migration ID per line, in the order migrations were applied:

```
0001-rename-tickets-to-work
0002-some-future-migration
```

`.accelerator/state/migrations-skipped` contains one migration ID per line for migrations the user has chosen to defer:

```
0002-some-future-migration
```

Both files are human-readable and constitute the audit trail. Do not edit them manually unless you are deliberately marking a migration as applied, unapplied, or skipped.

## Skip-tracking

Skip a migration to defer it indefinitely:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/scripts/run-migrations.sh" --skip <migration-id>
```

Unskip a previously skipped migration so it becomes pending again:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/scripts/run-migrations.sh" --unskip <migration-id>
```

Skipped migrations never run and do not block other pending migrations. The pre-run banner includes a `--skip` hint for each pending migration. `ACCELERATOR_MIGRATE_FORCE=1` bypasses the dirty-tree pre-flight only; skipped migrations remain skipped even with FORCE.

## MIGRATION_RESULT contract

A migration that exits 0 and emits `MIGRATION_RESULT: no_op_pending` on stdout is treated as a soft deferral — it stays pending and will be retried on future runs. The sentinel line is stripped from user-visible output.

Migrations emitting this sentinel MUST guarantee they performed no destructive operations before the line was emitted. Migrations doing destructive work must either succeed (recorded as applied) or fail non-zero.

## Optional interactive contract

Mechanical migrations are the default — they run start-to-finish with no user interaction. A migration that needs to **ask the user about ambiguous transformations** can opt in to the interactive contract by adding `# INTERACTIVE: yes` to its header. The framework then drives a per-transformation accept / edit / skip prompt loop, persists each decision durably before any artefact mutation, and resumes from the on-disk session log on subsequent runs.

### API reference at a glance

| Name | When called | Args | Returns | Side effects |
|---|---|---|---|---|
| `migration_emit_transformations` | once at handshake | none (uses `harness_emit_transformation` to emit each record) | (no return contract) | accumulates extras via `harness_extras_set` (reset after each emission) |
| `migration_evaluate_predicate` | per transformation | TSV line on stdin (use `harness_field`) | exit 0 = prompt; exit 1 = mechanical; other non-zero = FAIL | none |
| `migration_validate_edit` | per edit decision | positional: `key path anchor proposed user_value` | exit 0 = valid; non-zero (via `harness_reject`) = invalid | should write error to stderr |
| `migration_apply_decision` | per accept/edit, AFTER runner persists; never for skip | positional: `key path anchor decision value` | exit 0 = success; non-zero = FAIL aborts migration | mutates the artefact at (path, anchor) |
| `migration_session_log_path` *(optional)* | once at handshake | none | path string on stdout | none |
| `migration_verify_applied` *(optional)* | per resumed-accepted/edited key | positional: `key path anchor recorded_outcome recorded_proposed [recorded_user_value]` | exit 0 = mutation present; non-zero = absent → re-prompt | none |
| `harness_emit_transformation` | inside `migration_emit_transformations` | named: `key= path= anchor= proposed= predicate_value= display=` | (helper) | emits one TSV transformation; auto-clears extras after |
| `harness_extras_set` | inside `migration_emit_transformations` | positional: `key value` | (helper) | accumulates one extras pair for the next emission |
| `harness_extras_clear` | optional inside `migration_emit_transformations` | none | (helper) | drops accumulated extras |
| `harness_field` | inside `migration_evaluate_predicate` (TSV on stdin) | positional: `field name` | value on stdout | none |
| `harness_reject` | inside `migration_validate_edit` | positional: `message` | (helper, exits non-zero) | prints `[interactive] <message>` to stderr |
| `harness_run` | last line of the migration | none | (drives the protocol loop) | the entire interactive protocol |

### Header marker

A migration declares the hook by adding `# INTERACTIVE: yes` somewhere in its first five header-comment lines. Template skeleton:

```bash
#!/usr/bin/env bash
# DESCRIPTION: <short imperative>
# INTERACTIVE: yes

set -euo pipefail
source "$CLAUDE_PLUGIN_ROOT/scripts/atomic-common.sh"
source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"

migration_emit_transformations() { ... }
migration_evaluate_predicate()   { ... }
migration_validate_edit()        { ... }
migration_apply_decision()       { ... }

harness_run
```

### Author-facing helpers

Authors NEVER hand-write TSV positional fields, base64-encode display blocks, or hand-write JSON. The helpers exposed by `scripts/interactive-harness.sh` cover every wire-protocol concern:

- `harness_emit_transformation key=K path=P anchor=A proposed=V predicate_value=PV display=$'multi\nline'` — emits one transformation. Extras from `harness_extras_set` are attached then cleared.
- `harness_extras_set <key> <value>` — accumulate one extras pair. Keys must match `^[a-z][a-z0-9_]*$` and cannot collide with framework-mandatory names (`transformation_key`, `schema_version`, `outcome`, `proposed_value`, `user_value`, `timestamp`). Extras are **auto-cleared after each `harness_emit_transformation`** — set them inside the emit loop, not once before it.
- `harness_extras_clear` — drop all accumulated extras.
- `harness_field <name>` — inside `migration_evaluate_predicate`, extract a field by name from the TSV transformation line on stdin.
- `harness_reject "<message>"` — inside `migration_validate_edit`, reject the user's input in a uniform format (`[interactive] <message>`). Always return non-zero from the validator after calling this.

### Callback contracts

- `migration_evaluate_predicate` exits 0 to route the transformation through the prompt loop, exit 1 to apply it mechanically (no prompt), or any other non-zero status to abort the migration with `FAIL`.
- `migration_apply_decision` is called once per `accept` or `edit` decision, **after** the runner has durably persisted the JSONL session-log record (write-ahead-log invariant). It is **not** called for `skip`. A non-zero exit aborts the migration without ledger append.
- `migration_verify_applied` (optional) is called on resume *before* emitting `RESUMED_APPLIED`. Returning non-zero tells the framework the recorded mutation is absent (e.g. from a partial-apply crash); the framework removes the stale record via DRIFT and re-prompts.

### Runner guarantees (ADR-0037 §§1–4)

- **§1 Predicate routing**: each transformation is routed to either the prompt loop or the mechanical path based on the predicate's exit code.
- **§2 Display elements**: every prompt shows the proposed mutation target and value, the source location as `path:anchor`, and the predicate's evaluated value, plus any author-declared `display=` content.
- **§3 Resumability**: every prompted decision is durably persisted to `.accelerator/state/migrations-<id>-session.jsonl` **before** the artefact is mutated (write-ahead-log invariant). On re-entry, already-decided keys emit `RESUMED_APPLIED` / `RESUMED_SKIPPED` and skip the prompt. A migration completes (and the ID is appended to `migrations-applied`) only when the harness emits `DONE`.
- **§4 Decision verbs**: `accept` applies the proposed value; `edit <value>` substitutes a user-provided value (validated by `migration_validate_edit`); `skip` records the user's decision but does not mutate the artefact.

### Runner-level decisions

- **Source-drift**: if a recorded record's `proposed_value` differs from the live emission's, the runner re-prompts and the old record is discarded via `atomic_jsonl_remove_by_key`. Because the write-ahead-log invariant guarantees persistence before mutation, DRIFT unambiguously means "the migration source changed" — partial-apply crashes resume via `RESUMED_APPLIED` (or via `migration_verify_applied` if declared).
- **Transformation ordering**: emission order from `migration_emit_transformations` is the canonical iteration order.
- **Sticky skip semantics**: a transformation skipped on a prior run remains skipped on every subsequent run, even if its predicate would have changed. Users who want to re-prompt a previously-skipped key delete the corresponding session-log line and re-run.

### Session log

- Path: `.accelerator/state/migrations-<id>-session.jsonl` (override via `migration_session_log_path`). Relative paths are resolved against `PROJECT_ROOT`.
- One JSON object per line, canonical field order: `transformation_key`, `schema_version: 1`, `outcome` ∈ `{accepted, edited, skipped}`, `proposed_value`, optional `user_value` (only for `edited`), `timestamp`, followed by any author-declared extras in receipt order.
- The session log is retained as an audit artefact after full completion; users may delete it manually. The runner refuses to resume from a log with an unknown `schema_version` and prints a clear recovery instruction.

### Worked example

The fixture in `scripts/test-fixtures/interactive/doc-example/` ships with the plugin and is exercised by `test-migrate-interactive.sh` (AC-13). Three transformations are emitted — one ambiguous (prompted), one resolved (mechanical), one ambiguous with an empty-value validator. With the scripted decisions `edit ` (empty), `edit 0123-renamed`, `skip`, the user-facing transcript is:

<!-- @transcript-start -->
```
Session log: <SANDBOX>/.accelerator/state/migrations-0099-doc-example-session.jsonl  (resume from this file by re-running /accelerator:migrate)

── Transformation 1 ────────────────────────
Proposed:  0034-foo
Source:    meta/work/example-A.md:14
Predicate: ambiguous

Proposed value: 0034-foo
Surrounding prose: the linkage paragraph

[accept | edit <new-value> | skip] >
[interactive] empty value not allowed
── Transformation 1 ────────────────────────
Proposed:  0034-foo
Source:    meta/work/example-A.md:14
Predicate: ambiguous

Proposed value: 0034-foo
Surrounding prose: the linkage paragraph

[accept | edit <new-value> | skip] >
── Transformation 2 ────────────────────────
Proposed:  0007-baz
Source:    meta/work/example-C.md:21
Predicate: ambiguous

Proposed value: 0007-baz
Surrounding prose: the paragraph the author wants to revise

>
```
<!-- @transcript-end -->

And the on-disk session log (with timestamps redacted) is:

<!-- @session-log-start -->
```
{"transformation_key":"link-A","schema_version":1,"outcome":"edited","proposed_value":"0034-foo","user_value":"0123-renamed","timestamp":"<REDACTED>","band":"ambiguous","prose":"the linkage paragraph"}
{"transformation_key":"link-C","schema_version":1,"outcome":"skipped","proposed_value":"0007-baz","timestamp":"<REDACTED>"}
```
<!-- @session-log-end -->

The middle transformation (`link-B`, band `resolved`) routed through the mechanical path — predicate exited 1, the harness emitted `MECHANICAL_APPLIED`, no record was persisted, the artefact was mutated unconditionally.

## Executing the migration

Invoke via Bash:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/scripts/run-migrations.sh"
```

The driver script resolves `PROJECT_ROOT` automatically from the current working directory. Run it from within the consumer repository. Run `/accelerator:migrate` from a single shell at a time; it does not acquire a lock.

## Cross-references

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — framework design rationale
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` — optional interactive contract
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md` — first consumer's parameterisation
- `skills/config/init/SKILL.md` — `init` bootstraps fresh repos; `migrate` upgrades existing ones
- `skills/config/configure/SKILL.md` — configuration reference
