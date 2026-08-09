---
title: Migrations
---

When a new plugin version renames directories, config keys, or file formats, a
migration handles the upgrade. Run `/migrate` after updating
the plugin to apply any pending migrations.

```
/migrate
```

Migrations run in-process, compiled into the `accelerator-migrate` sub-binary
— there is no forked script per migration. Safety guards: the runner refuses
to run on a dirty working tree, prints a pre-run banner listing each pending
migration, and previews each one before applying. All mutations are tracked
in `.accelerator/state/migrations-applied`. Recovery is via VCS revert. Set
`ACCELERATOR_MIGRATE_FORCE=1` to bypass the clean-tree check if needed.

To opt out of an individual migration, run
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate --skip <id>`
(and `--unskip <id>` to re-enable it). Skipped IDs are tracked in
`.accelerator/state/migrations-skipped` and surfaced by name in the runner's
summary line so a permanent skip is never invisible. A migration can also
self-defer by returning a typed no-op-pending outcome — useful
for migrations whose preconditions (e.g. a `{project}` pattern in
`work.id_pattern`) aren't yet configured.

A `SessionStart` hook fires automatically when the bundled migrations have not
all been applied, reminding you to run `/migrate`. (On repos that
haven't run migration `0003` yet, the hook reads the legacy
`meta/.migrations-applied` file as a fallback.)

## Skill reference

For invocation and arguments, see the
[`migrate`](reference/skills/config/migrate.md) skill reference.
