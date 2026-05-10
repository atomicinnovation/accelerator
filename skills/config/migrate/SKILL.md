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

## Executing the migration

Invoke via Bash:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/scripts/run-migrations.sh"
```

The driver script resolves `PROJECT_ROOT` automatically from the current working directory. Run it from within the consumer repository. Run `/accelerator:migrate` from a single shell at a time; it does not acquire a lock.

## Cross-references

- `meta/decisions/ADR-0023-migration-framework.md` — framework design rationale
- `skills/config/init/SKILL.md` — `init` bootstraps fresh repos; `migrate` upgrades existing ones
- `skills/config/configure/SKILL.md` — configuration reference
