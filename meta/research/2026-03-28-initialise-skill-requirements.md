---
date: "2026-03-28T21:49:56+0000"
researcher: Toby Clemson
git_commit: 508ca24b973d8c742e52e829c557f0c62f81076d
branch: main
repository: accelerator
topic: "Initialise skill: codebase analysis and requirements"
tags: [research, codebase, initialise, skill, directory-structure, configuration]
status: complete
last_updated: "2026-03-28"
last_updated_by: Toby Clemson
last_updated_note: "Added analysis of review-pr ephemeral file placement problem and proposed migration to meta/tmp/"
---

# Research: Initialise Skill — Codebase Analysis and Requirements

**Date**: 2026-03-28T21:49:56+0000
**Researcher**: Toby Clemson
**Git Commit**: 508ca24b973d8c742e52e829c557f0c62f81076d
**Branch**: main
**Repository**: accelerator

## Research Question

How should an initialise skill be introduced to the Accelerator plugin to
prepare a codebase with everything the plugin expects? What directories,
files, and git configuration need to be created? How should it interact with
existing configuration?

## Summary

The plugin currently has no initialisation mechanism — directories are created
on demand by individual skills via inline `mkdir -p` commands or prose
instructions. This is fragile: some directories (notably `meta/tmp/`) are
documented and gitignored but never created by any skill. An initialise skill
should create all configurable output directories (honouring path overrides),
set up `meta/tmp/` with a `.gitkeep` and `.gitignore`, and ensure
`.claude/accelerator.local.md` is gitignored. The skill should live at
`skills/config/initialise/SKILL.md` alongside the existing `configure` skill,
and should be fully idempotent.

Additionally, the `review-pr` skill currently writes ephemeral working files
(diff, changed-files, commits, head-sha, repo-info, review-payload) to
`{pr reviews directory}/pr-review-{number}/` — the same parent directory that
holds persistent review artifacts like `{number}-review-{N}.md`. This makes
it easy to accidentally commit temporary files and hard to gitignore them
without also ignoring the persistent reviews. As part of the initialise skill
work, `review-pr` should be updated to write ephemeral files to
`meta/tmp/pr-review-{number}/` instead, which is already gitignored.

## Detailed Findings

### 1. Current Directory Creation Patterns

Skills currently create directories in three inconsistent ways:

| Pattern | Skills | Example |
|---------|--------|---------|
| Inline `mkdir -p` in bash block | `review-plan`, `review-pr` | `mkdir -p {plan reviews directory}` |
| Prose instruction ("create if it doesn't exist") | `create-adr`, `extract-adrs`, `validate-plan` | "Create the configured decisions directory if it doesn't exist" |
| No mention at all (assumes directory exists) | `research-codebase`, `create-plan`, `describe-pr` | Writes directly to path |

No shell scripts handle directory creation — it is left entirely to the LLM at
skill invocation time.

### 2. Configurable Paths (via `config-read-path.sh`)

The `config-read-path.sh` script resolves paths using `config-read-value.sh`
with a `paths.` prefix. Each path has a default and is used by specific skills:

| Path Key | Default | Written By |
|----------|---------|------------|
| `plans` | `meta/plans` | `create-plan` |
| `research` | `meta/research` | `research-codebase` |
| `decisions` | `meta/decisions` | `create-adr`, `extract-adrs`, `review-adr` |
| `prs` | `meta/prs` | `describe-pr` |
| `validations` | `meta/validations` | `validate-plan` |
| `review_plans` | `meta/reviews/plans` | `review-plan` |
| `review_prs` | `meta/reviews/prs` | `review-pr` |
| `templates` | `meta/templates` | manual (user-placed templates) |
| `tickets` | `meta/tickets` | manual (user-placed ticket files) |
| `notes` | `meta/notes` | manual |

The initialise skill would need to resolve each of these via `config-read-path.sh`
to honour any user overrides before creating directories.

### 3. The `meta/tmp/` Problem and `review-pr` Ephemeral Files

`meta/tmp/` is documented in the README as "Ephemeral working data (e.g.,
review artifacts)" written by `review-pr`. The `.gitignore` includes
`meta/tmp/` (line 8), confirming the intent for it to exist as a gitignored
temp directory. However, no skill currently writes to `meta/tmp/`.

The `review-pr` skill writes ephemeral working files to
`{pr reviews directory}/pr-review-{number}/` — a subdirectory of the same
`meta/reviews/prs/` directory that holds persistent review artifacts like
`{number}-review-{N}.md`. This creates two problems:

1. **Accidental commits**: The ephemeral `pr-review-{number}/` directories
   contain files like `diff.patch`, `changed-files.txt`, `pr-description.md`,
   `commits.txt`, `head-sha.txt`, `repo-info.txt`, and
   `review-payload.json`. These sit alongside committable review documents
   and are easily staged by accident (e.g., `git add meta/`).
2. **Difficult to gitignore**: You cannot add `pr-review-*/` to `.gitignore`
   without also carefully ensuring the persistent `{number}-review-{N}.md`
   files are not affected. The naming conventions are different, so a pattern
   could be crafted, but it's fragile.

**Proposed fix**: Move `review-pr`'s ephemeral working directory from
`{pr reviews directory}/pr-review-{number}/` to
`meta/tmp/pr-review-{number}/`. This cleanly separates ephemeral data
(gitignored via the existing `meta/tmp/` entry) from persistent review
artifacts. The `review-pr` SKILL.md would need updates at:
- Line 84-88: temp directory creation (`mkdir -p`)
- Lines 92-95: diff/changed-files/description/commits writes
- Lines 103-104: head-sha/repo-info writes
- Line 250: agent prompt referencing temp directory path
- Line 261: agent instruction to read from temp directory
- Lines 528-530: reading head-sha and repo-info
- Lines 553-556: writing and posting review-payload.json
- Lines 610-617: cleanup guideline text

The `describe-pr` skill uses system `/tmp/` for its transient file
(`/tmp/pr-body-{number}.md`), which is a different pattern. This could
optionally also move to `meta/tmp/` for consistency, though system `/tmp/`
is automatically cleaned up by the OS.

The initialise skill should:
1. Create `meta/tmp/` (or the configured equivalent)
2. Place a `.gitkeep` in it so the directory is committed
3. Place a `.gitignore` in it containing `*` and `!.gitkeep` and `!.gitignore`
   so contents are ignored but the directory structure is preserved

**Note**: `meta/tmp/` is not currently a configurable path via
`config-read-path.sh`. The initialise skill could either:
- Hardcode it relative to the project root (simplest)
- Derive it from a common prefix of the other paths (fragile)
- Add a new `tmp` path key to `config-read-path.sh` (most consistent)

### 4. `.gitkeep` and `.gitignore` State

**No `.gitkeep` files exist anywhere** in the repository. All `meta/`
subdirectories are created on demand by skills.

**Root `.gitignore` contains:**
```
.DS_Store
*.swp
*~
.idea
*.iml
*.ipr
*.iws
meta/tmp/
workspaces/
```

Key observations:
- `meta/tmp/` is already gitignored at the root level
- `.claude/accelerator.local.md` is **not** in `.gitignore` — the `configure`
  skill offers to add it during `create`, but this only happens if the user
  runs that skill
- No `.claude/.gitignore` file exists

### 5. Skill Placement and Structure

The plugin organises skills into category directories registered in
`.claude-plugin/plugin.json`. The `configure` skill already lives at
`skills/config/configure/SKILL.md`. The new `initialise` skill should sit
alongside it at `skills/config/initialise/SKILL.md` since:

- It is a configuration/setup concern, not a development workflow concern
- The `skills/config/` path is already registered in `plugin.json`
- It follows the pattern of `configure` (no subagent spawning, direct
  filesystem interaction)

**Frontmatter pattern for non-agent skills:**
```yaml
---
name: initialise
description: ...
argument-hint: "[optional flags]"
disable-model-invocation: true
---
```

The `configure` skill omits `allowed-tools` (giving full tool access), which
would also be appropriate for `initialise` since it needs to run arbitrary
bash commands (`mkdir`, write files, modify `.gitignore`).

### 6. Hook Integration — "Not Yet Initialised" Detection

Currently, the SessionStart hooks have no first-run detection:
- `config-detect.sh` is completely silent when no config files exist
- `config-summary.sh` exits immediately with no output when no config files
  are found

The initialise skill could optionally enhance the SessionStart hook to detect
when initialisation hasn't been run and suggest it. However, this is a separate
concern from the skill itself and may be better addressed later.

### 7. Additional Initialisation Candidates

Beyond the explicitly requested items, the following should be considered:

**a) `.claude/accelerator.local.md` in `.gitignore`**
The `configure` skill's `create` action already offers to gitignore the local
config file, but only if the user runs that flow. The initialise skill should
ensure this entry is present regardless.

**b) Eager creation of all output directories**
Currently, skills create directories on first use. This leads to errors or
confusing behaviour when a skill runs before any directory exists (e.g.,
`research-codebase` and `create-plan` don't mention creating their output
directories at all). Pre-creating all configurable directories makes the
first skill invocation smoother.

**c) `.gitkeep` files for empty directories**
Git does not track empty directories. If the team wants to commit the `meta/`
structure so all developers have it, `.gitkeep` files are needed. This is
particularly important for:
- `meta/tmp/` (must exist for any skill that might use it)
- `meta/templates/` (users need to know where to place custom templates)
- Directories like `meta/tickets/` and `meta/notes/` that are only manually
  populated

For directories populated by skills (`meta/plans/`, `meta/research/`, etc.),
`.gitkeep` ensures the directory exists in fresh clones but becomes unnecessary
once the first artifact is written.

**d) Template directory seeding**
The `templates/` directory in the plugin root contains default templates. The
initialise skill could optionally copy these to the project's `meta/templates/`
directory as a starting point for customisation. However, this may be better
left as a separate action since the template resolution system already falls
back to plugin defaults.

### 8. Idempotency Requirements

The skill must be safe to run repeatedly:
- `mkdir -p` is inherently idempotent
- `.gitkeep` writes should check for existence (or just overwrite — the
  content is always empty)
- `.gitignore` entries should be checked before appending to avoid duplicates
- The `meta/tmp/.gitignore` should be written with fixed content (overwrite is
  safe since it's a plugin-managed file)
- The skill should report what it did and what was already in place

### 9. Path Resolution Mechanics

The `config-read-path.sh` script takes a path key and default, delegates to
`config-read-value.sh` with a `paths.` prefix. The resolution chain:

1. Check `.claude/accelerator.md` frontmatter for `paths.<key>`
2. Check `.claude/accelerator.local.md` frontmatter for `paths.<key>`
   (last-writer-wins)
3. If neither has it, use the caller-provided default

The initialise skill can call `config-read-path.sh` for each directory to
honour overrides. For example:
```bash
plans_dir=$(bash "$PLUGIN_ROOT/scripts/config-read-path.sh" plans meta/plans)
```

## Code References

- `scripts/config-read-path.sh:1-22` — Path resolution entry point
- `scripts/config-read-value.sh:1-129` — Generic value reader with
  last-writer-wins override
- `scripts/config-common.sh:12-14` — `config_project_root()` for project root
  discovery
- `scripts/config-common.sh:19-26` — `config_find_files()` for config file
  discovery
- `scripts/config-summary.sh:1-138` — SessionStart summary builder (per-skill
  customisation detection at lines 78-132)
- `skills/config/configure/SKILL.md` — Configure skill structure and
  `.gitignore` handling (lines 80-82)
- `skills/github/review-pr/SKILL.md:84-88` — Inline `mkdir -p` pattern for
  ephemeral working directory creation
- `skills/github/review-pr/SKILL.md:92-95` — Ephemeral file writes (diff,
  changed-files, description, commits)
- `skills/github/review-pr/SKILL.md:103-104` — Ephemeral file writes
  (head-sha, repo-info)
- `skills/github/review-pr/SKILL.md:250-261` — Agent prompt referencing
  ephemeral temp directory for PR artifacts
- `skills/github/review-pr/SKILL.md:528-556` — Reading ephemeral files and
  writing review-payload.json
- `skills/github/review-pr/SKILL.md:610-617` — Cleanup guideline for
  ephemeral working directory
- `skills/decisions/create-adr/SKILL.md:143` — Prose directory creation pattern
- `.claude-plugin/plugin.json:9-18` — Skill path registration (includes
  `./skills/config/`)
- `.gitignore:8` — `meta/tmp/` already gitignored
- `README.md:77-87` — `meta/` directory table (including stale `tmp/` entry)

## Architecture Insights

1. **No centralised directory management**: Every skill is responsible for its
   own output directory. This is the core gap the initialise skill fills.

2. **Configuration before initialisation**: The path resolution system is
   designed to work with zero configuration (defaults are baked into every
   call). This means `initialise` can run before or after `configure` — if
   config exists, overrides are honoured; if not, defaults are used.

3. **The `config/` skill category**: This is the natural home for
   infrastructure skills. The plugin.json already includes `./skills/config/`
   as a registered skill path, so adding `initialise/SKILL.md` requires no
   manifest changes.

4. **Git-awareness via `vcs-common.sh`**: The shared VCS utilities handle both
   git and jujutsu. The initialise skill's `.gitignore` operations should
   account for jujutsu-managed repositories (where `.gitignore` still works
   since jj uses git's ignore mechanisms).

5. **`review-pr` ephemeral file migration**: The README documents `meta/tmp/`
   as written by `review-pr`, but the skill currently writes to
   `meta/reviews/prs/pr-review-{number}/`. Migrating the ephemeral files to
   `meta/tmp/pr-review-{number}/` would make the README accurate, cleanly
   separate ephemeral from persistent data, and eliminate the accidental-commit
   risk. This is a companion change to the initialise skill — `meta/tmp/`
   needs to exist (via initialisation) before `review-pr` can write there.

## Open Questions

1. **Should all `meta/` directories get `.gitkeep` files?** Arguments for:
   consistent directory structure across clones, clearer onboarding. Arguments
   against: file noise, unnecessary for directories that will be populated
   quickly. A middle ground: `.gitkeep` for `tmp/`, `templates/`, `tickets/`,
   and `notes/` (manually populated); no `.gitkeep` for skill-output
   directories (they get artifacts on first use).

2. **Should `meta/tmp/` become a configurable path?** Adding a `tmp` key to
   `config-read-path.sh` would be consistent with all other paths, but
   currently no skill references it. The simpler approach is to derive it
   from the project root directly.

3. **Should the SessionStart hook hint at initialisation?** The hook could
   detect missing `meta/` directories and suggest running `/accelerator:initialise`.
   This adds complexity but improves the first-run experience. It could be
   deferred to a follow-up.

4. **Should template files be copied to the project?** The initialise skill
   could seed `meta/templates/` with copies of the plugin's default templates,
   making them visible and editable. Alternatively, the current fallback-to-
   plugin-defaults behaviour may be sufficient.

5. **Should `initialise` handle `.claude/accelerator.local.md` gitignoring?**
   The `configure` skill already offers this during `create`. Having
   `initialise` also do it is more robust (catches the case where `configure`
   was never run) but creates overlap. The initialise skill could add the
   gitignore entry unconditionally (it's harmless if the file doesn't exist
   yet) or only if the file exists.
