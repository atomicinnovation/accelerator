---
name: init
description: Prepare a repository with the directories and gitignore entries
  that Accelerator skills expect. Safe to run repeatedly.
argument-hint: "(no arguments — safe to run repeatedly)"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*) 
---

# Initialise Accelerator

You prepare a consumer repository with all directories and gitignore entries
that Accelerator skills expect. This is safe to run repeatedly — it reports
what was created versus what already existed.

## Path Resolution

Resolve each output directory using the plugin's path configuration:

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions`
**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations`
**Review plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans`
**Review PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs`
**Review work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work`
**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes`
**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories`
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps`
**Global directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh global`

## Steps

Execute the following steps in order, tracking what was created versus what
already existed so you can present a summary at the end.

### Steps 1–3: Run the init script

<!-- DIR_COUNT:13 -->

```bash
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/init/scripts/init.sh"
```

The script creates 13 `meta/` directories with `.gitkeep` files, creates the
`.accelerator/` core scaffold (`.gitignore`, `state/`, `tmp/`, `skills/`,
`lenses/`, `templates/`), writes the inner tmp `.gitignore`, and appends the
`.accelerator/config.local.md` rule to the root `.gitignore`. It is idempotent
— safe to run repeatedly.

### Step 4: Report results

Only after all steps above have completed successfully, present a summary:

```
Initialisation complete:

Directories:
  ✓ {plans directory} (created | already exists)
  ✓ {research directory} (created | already exists)
  ✓ {decisions directory} (created | already exists)
  ✓ {prs directory} (created | already exists)
  ✓ {validations directory} (created | already exists)
  ✓ {review plans directory} (created | already exists)
  ✓ {review prs directory} (created | already exists)
  ✓ {review work items directory} (created | already exists)
  ✓ {work items directory} (created | already exists)
  ✓ {notes directory} (created | already exists)
  ✓ {design inventories directory} (created | already exists)
  ✓ {design gaps directory} (created | already exists)
  ✓ {global directory} (created | already exists)

Accelerator scaffold:
  ✓ .accelerator/.gitignore (created | already exists)
  ✓ .accelerator/state/.gitkeep (created | already exists)
  ✓ .accelerator/skills/.gitkeep (created | already exists)
  ✓ .accelerator/lenses/.gitkeep (created | already exists)
  ✓ .accelerator/templates/.gitkeep (created | already exists)
  ✓ .accelerator/tmp/.gitignore (created | already exists)
  ✓ .accelerator/tmp/.gitkeep (created | already exists)

Gitignore entries:
  ✓ .accelerator/config.local.md (added | already present)
```

Use the actual resolved paths in the output (not the variable names).

---

`/accelerator:init` bootstraps a fresh repository. To upgrade an existing repository after a plugin update, use `/accelerator:migrate` instead — it applies ordered, idempotent migrations to bring `meta/`, `.claude/`, and `.accelerator/` in line with the latest schema.
