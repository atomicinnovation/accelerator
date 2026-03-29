---
date: "2026-03-29T00:00:00Z"
type: plan
skill: create-plan
status: draft
---

# Rename `initialise` Skill to `init` Implementation Plan

## Overview

Rename the `initialise` skill to `init` for parity with Claude Code's own
`/init` command. The skill was only just released and hasn't been adopted yet,
so there are no backwards-compatibility concerns.

## Current State Analysis

- Skill directory: `skills/config/initialise/SKILL.md`
- Skill name in frontmatter: `initialise`
- The `config-summary.sh` script references `/accelerator:initialise` in its
  init hint message (line 23) and a comment (line 6)
- `README.md` references `/accelerator:initialise` on line 73
- `CHANGELOG.md` references the old name but these are historical records and
  should not be changed
- Meta research/plan documents reference the old name but are historical

### Key Discoveries:

- The skill directory name must match the `name:` frontmatter field
  (`skills/config/initialise/SKILL.md:2`)
- The `config-summary.sh` script dynamically discovers skill names from
  directories (`scripts/config-summary.sh:98-103`) so the rename will
  automatically update the known skills list
- The SKILL.md body text ("Initialise Accelerator", "Initialisation complete:")
  is fine as-is — no need to shorten

## What We're NOT Doing

- Updating CHANGELOG.md entries (historical record)
- Updating meta research/plan documents (historical record)
- Changing body text within the SKILL.md (title, output messages)
- Adding any aliases or redirects from the old name

## Implementation Approach

This is a single-phase rename with four file-level changes. The directory
rename is done via `jj` (the repository's VCS).

## Phase 1: Rename Skill

### Overview

Rename the directory, update the frontmatter, and update all active references.

### Changes Required:

#### 1. Rename the skill directory

Rename `skills/config/initialise/` to `skills/config/init/`.

```bash
jj file rename skills/config/initialise/SKILL.md skills/config/init/SKILL.md
```

#### 2. Update skill frontmatter

**File**: `skills/config/init/SKILL.md`

Change the `name:` field from `initialise` to `init`:

```yaml
# Before
name: initialise

# After
name: init
```

#### 3. Update `config-summary.sh`

**File**: `scripts/config-summary.sh`

Update the comment on line 6:

```bash
# Before
# Outputs nothing if no config files exist and the repo is already initialised.

# After
# Outputs nothing if no config files exist and the repo is already init'd.
```

Update the `INIT_HINT` on line 23:

```bash
# Before
INIT_HINT="Accelerator has not been initialised in this repository. Type /accelerator:initialise at the prompt to set up the expected directory structure and gitignore entries."

# After
INIT_HINT="Accelerator has not been initialised in this repository. Type /accelerator:init at the prompt to set up the expected directory structure and gitignore entries."
```

#### 4. Update `README.md`

**File**: `README.md`

Update line 73:

```markdown
# Before

and writes to predictable paths within it. Run `/accelerator:initialise` to

# After

and writes to predictable paths within it. Run `/accelerator:init` to
```

### Success Criteria:

#### Automated Verification:

- [x] Skill directory exists at new path: `[ -d skills/config/init ]`
- [x] Old directory no longer exists: `[ ! -d skills/config/initialise ]`
- [x] Skill name in frontmatter is `init`: `head -5 skills/config/init/SKILL.md`
- [x] No remaining references to `/accelerator:initialise` in active files
  (excluding CHANGELOG and meta history):
  `grep -r "accelerator:initialise" --include="*.md" --include="*.sh" skills/ scripts/ README.md`
- [x] `config-summary.sh` hint references `/accelerator:init`:
  `grep "accelerator:init" scripts/config-summary.sh`
- [ ] Skill is discovered correctly by config-summary's dynamic scan:
  run `bash scripts/config-summary.sh` in an uninitialised repo context

#### Manual Verification:

- [ ] `/accelerator:init` invokes the skill correctly
- [ ] SessionStart hook shows the updated hint message for uninitialised repos

## References

- Skill definition: `skills/config/initialise/SKILL.md`
- Config summary script: `scripts/config-summary.sh`
- README: `README.md:73`
