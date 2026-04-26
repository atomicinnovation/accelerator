---
name: migrate
description: Apply pending Accelerator meta-directory migrations to bring a repo into line with the latest plugin schema. Destructive by default but guarded — refuses to run on a dirty working tree and prints a one-line preview per pending migration before applying.
allowed-tools: [Read, Write, Edit, Bash]
---

> **Warning: this skill rewrites files in `meta/` and `.claude/accelerator*.md`.** Recovery is via VCS revert. Before running, ensure your repo is committed and you understand what each pending migration does. The safety guards (clean-tree check, preview) exist to give you a moment to stop — they are not a substitute for understanding the changes.

## When to invoke

Run `/accelerator:migrate` after upgrading the Accelerator plugin to a version that bundles new migrations. The SessionStart hook will tell you when this is needed:

```
[accelerator] meta/.migrations-applied is behind the plugin
(highest applied: 0001-rename-tickets-to-work; highest available: 0002-...).
Run /accelerator:migrate to bring it up to date.
```

You can also run it proactively — if no migrations are pending, it prints `No pending migrations.` and exits cleanly.

## How it works

The driver script `skills/config/migrate/scripts/run-migrations.sh` orchestrates the migration lifecycle:

1. **Clean-tree pre-flight.** Checks `meta/` and `.claude/accelerator*.md` for uncommitted changes. Aborts if any are found. Set `ACCELERATOR_MIGRATE_FORCE=1` to bypass (advanced users only).

2. **Read state.** Loads `meta/.migrations-applied` — a newline-delimited list of migration IDs already run. If the file is absent, the applied set is empty. Unknown IDs (from a newer plugin version) are preserved verbatim and warned about.

3. **Discover migrations.** Globs `skills/config/migrate/migrations/[0-9][0-9][0-9][0-9]-*.sh` in sorted order. The directory can be overridden via `ACCELERATOR_MIGRATIONS_DIR` (used in tests).

4. **Preview.** Prints one line per pending migration — `[<ID>] <description>` — extracted from each script's `# DESCRIPTION:` comment. If nothing is pending, exits 0 immediately.

5. **Apply in order.** For each pending migration: runs it with `PROJECT_ROOT` exported; on success atomically appends its ID to `meta/.migrations-applied`; on failure prints the migration's output to stderr, exits 1, and leaves the state file at the last successful migration.

6. **Summary.** Prints a count of applied migrations.

## Per-migration contract

Each migration script under `migrations/` must:

- Start with `#!/usr/bin/env bash` then `# DESCRIPTION: <short imperative description>` on line 2
- Receive `PROJECT_ROOT` as an exported env var pointing to the consumer repo
- Self-detect and exit 0 on already-applied state (idempotent)
- Use atomic write patterns (`temp-file-then-rename`) for any file rewrites
- NOT honour any `DRY_RUN` env var — this framework has no dry-run mode

## State file format

`meta/.migrations-applied` contains one migration ID per line, in the order migrations were applied:

```
0001-rename-tickets-to-work
0002-some-future-migration
```

The file is human-readable and constitutes the audit trail. Do not edit it manually unless you are deliberately marking a migration as applied or unapplied.

## Executing the migration

Invoke via Bash:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/scripts/run-migrations.sh"
```

The driver script resolves `PROJECT_ROOT` automatically from the current working directory. Run it from within the consumer repository.

## Cross-references

- `meta/decisions/ADR-0023-migration-framework.md` — framework design rationale
- `skills/config/init/SKILL.md` — `init` bootstraps fresh repos; `migrate` upgrades existing ones
- `skills/config/configure/SKILL.md` — configuration reference
