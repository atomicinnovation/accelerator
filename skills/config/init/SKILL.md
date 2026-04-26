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

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs meta/prs`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations meta/validations`
**Review plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans meta/reviews/plans`
**Review PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
**Review work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work meta/reviews/work`
**Templates directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates meta/templates`
**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes meta/notes`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`

## Steps

Execute the following steps in order, tracking what was created versus what
already existed so you can present a summary at the end.

### Step 1: Create output directories

For each of the 12 directories resolved above, run:

```bash
mkdir -p {directory}
```

Then check whether `.gitkeep` already exists. If not, create it:

```bash
touch {directory}/.gitkeep
```

Track the status of each directory:
- If the directory did not exist before `mkdir -p`: **created**
- If the directory already existed: **already exists**

To determine this, check whether the directory exists before running `mkdir -p`
(use `[ -d {directory} ]`).

### Step 2: Set up the tmp directory

The tmp directory needs a `.gitignore` that ignores everything except itself.
This inner `.gitignore` is the sole mechanism for ignoring tmp contents — do
NOT add the tmp path to the consumer's root `.gitignore` (a root-level ignore
entry would prevent git from descending into the directory, which means the
inner `.gitignore`'s exclusions would never take effect and the directory would
not survive fresh clones).

Check whether `{tmp directory}/.gitignore` already exists. If not, write it
using the Write tool with this exact content:

```
# Ignore everything in this directory except the directory itself
*
!.gitkeep
!.gitignore
```

Track whether it was **created** or **already exists**.

### Step 3: Add `.claude/accelerator.local.md` to root `.gitignore`

Check whether a `.gitignore` file exists at the project root.

**If no `.gitignore` exists**, create one containing:

```
.claude/accelerator.local.md
```

**If `.gitignore` already exists**, read it and check whether
`.claude/accelerator.local.md` is already present. When checking, trim
whitespace from both the existing lines and the candidate entry before
comparison. If not present, append it to the end of the file (with a leading
newline if the file does not end with one).

Track whether it was **added** or **already present**.

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
  ✓ {templates directory} (created | already exists)
  ✓ {work items directory} (created | already exists)
  ✓ {notes directory} (created | already exists)
  ✓ {tmp directory} (created | already exists)

Tmp directory:
  ✓ {tmp directory}/.gitignore (created | already exists)
  ✓ {tmp directory}/.gitkeep (created | already exists)

Gitignore entries:
  ✓ .claude/accelerator.local.md (added | already present)
```

Use the actual resolved paths in the output (not the variable names).
