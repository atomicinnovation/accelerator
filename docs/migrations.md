# Migrations

When a new plugin version renames directories, config keys, or file formats, a
migration script handles the upgrade. Run `/migrate` after updating
the plugin to apply any pending migrations.

```
/migrate
```

Safety guards: the skill refuses to run on a dirty working tree, prints a
pre-run banner listing each pending migration, and previews each one before
applying. All mutations are tracked in `.accelerator/state/migrations-applied`.
Recovery is via VCS revert. Set `ACCELERATOR_MIGRATE_FORCE=1` to bypass the
clean-tree check if needed.

To opt out of an individual migration, run
`bash skills/config/migrate/scripts/run-migrations.sh --skip <id>` (and
`--unskip <id>` to re-enable it). Skipped IDs are tracked in
`.accelerator/state/migrations-skipped` and surfaced by name in the runner's
summary line so a permanent skip is never invisible. A migration can also
self-defer by emitting `MIGRATION_RESULT: no_op_pending` on stdout — useful
for migrations whose preconditions (e.g. a `{project}` pattern in
`work.id_pattern`) aren't yet configured.

A `SessionStart` hook fires automatically when the bundled migrations have not
all been applied, reminding you to run `/migrate`. (On repos that
haven't run migration `0003` yet, the hook reads the legacy
`meta/.migrations-applied` file as a fallback.)

## Skill reference

### <img src="https://api.iconify.design/ph/wrench-bold.svg?color=%23475569" width="18" align="center" alt=""> `/migrate`

Apply pending Accelerator meta-directory migrations to bring a repo into line
with the latest plugin schema.

*Destructive but guarded: it refuses to run on a dirty working tree and previews
each pending migration before applying. Recovery is via VCS revert.*
