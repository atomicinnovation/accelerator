---
date: "2026-03-28T22:30:00+0000"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Initialise Skill & review-pr Ephemeral File Migration — Implementation Plan

## Overview

Introduce an `/accelerator:initialise` skill that prepares a consumer
repository with all directories and gitignore entries the Accelerator plugin
expects, and migrate `review-pr`'s ephemeral working files from
`{pr reviews directory}/pr-review-{number}/` to
`{tmp directory}/pr-review-{number}/`
so that transient data is cleanly separated from persistent review artifacts.

## Current State Analysis

- **No initialisation mechanism exists.** Skills create directories on demand
  via inline `mkdir -p` or prose instructions. Some skills
  (`research-codebase`, `create-plan`) don't mention directory creation at all
  and assume the directory exists.
- **`meta/tmp/` is documented but never created.** The README
  (`README.md:86`) lists `tmp/` as written by `review-pr`, but no skill
  writes to it — `review-pr` writes ephemeral files inside
  `{pr reviews directory}/pr-review-{number}/` instead.
- **Ephemeral and persistent data are co-located.** The `review-pr` skill
  writes both its transient working files (`diff.patch`, `changed-files.txt`,
  etc.) and its persistent review artifact (`{number}-review-{N}.md`) under
  the same base directory (`meta/reviews/prs/`), making accidental commits
  easy and gitignoring difficult.
- **`tmp` is not a configurable path.** All other output directories are
  resolvable via `config-read-path.sh`, but `tmp` has no entry.
- **`.claude/accelerator.local.md` is not reliably gitignored.** The
  `configure` skill offers to add it during `create`, but only if the user
  runs that flow.
- **SessionStart hooks have no first-run detection.** `config-detect.sh`
  delegates to `config-summary.sh`, which exits silently when no config files
  exist. Neither detects missing directories.

### Key Discoveries:

- `skills/config/configure/SKILL.md:1-7` — `disable-model-invocation: true`
  pattern suitable for `initialise`
- `.claude-plugin/plugin.json:17` — `./skills/config/` already registered, no
  manifest change needed
- `scripts/config-read-path.sh:7-17` — 10 path keys documented; `tmp` will
  be the 11th
-
`skills/github/review-pr/SKILL.md:84-87,92-95,103-104,250,261-262,528-530,553-556,610-617`
— all locations referencing the ephemeral working directory

## Desired End State

After this plan is complete:

1. Running `/accelerator:initialise` in any consumer repository creates all
   configured output directories with `.gitkeep` files, sets up the tmp
   directory with a self-contained `.gitignore` (the sole ignore mechanism
   for tmp contents), and ensures `.claude/accelerator.local.md` is in the
   consumer's `.gitignore`.
2. The `review-pr` skill writes all ephemeral working files to
   `{tmp directory}/pr-review-{number}/` instead of
   `{pr reviews directory}/pr-review-{number}/`.
3. The `tmp` path is configurable via `config-read-path.sh` like all other
   paths.
4. The SessionStart hook detects missing meta directories and suggests
   running `/accelerator:initialise`.
5. Documentation (`configure` skill paths table, README) reflects the new
   `tmp` path key.

### Verification:

- Run `/accelerator:initialise` in a fresh consumer repo — all directories
  are created, `.gitkeep` files are present, `meta/tmp/.gitignore` is
  created, `.claude/accelerator.local.md` is added to root `.gitignore`.
- Run it again — output reports everything already in place, no duplicates.
- Run `/accelerator:review-pr` — ephemeral files land in
  `{tmp directory}/pr-review-{number}/`, not under `meta/reviews/prs/`.
- Start a new session in an uninitialised repo — SessionStart hook suggests
  running `/accelerator:initialise`.
- Configure `paths.tmp: custom/tmp` in `.claude/accelerator.md` and re-run
  initialise — the custom path is used.

## What We're NOT Doing

- **Template seeding**: Not copying plugin default templates to the consumer's
  `meta/templates/`. The existing fallback-to-plugin-defaults behaviour is
  sufficient.
- **Removing inline `mkdir -p` from existing skills**: Skills should continue
  to create their directories on demand so they work even if `initialise` has
  not been run.
- **Migrating `describe-pr`'s `/tmp/` usage**: It uses system `/tmp/` which
  is automatically cleaned up. Not worth changing.
- **Adding a `.claude/.gitignore` file**: We'll add entries to the consumer's
  root `.gitignore` for simplicity.

## Implementation Approach

The changes are ordered so each phase builds on the previous one: first the
infrastructure (`tmp` path key), then the skill that uses it, then the
consumer of the ephemeral directory, then the detection hook, and finally
documentation.

---

## Phase 1: Add `tmp` Path Key to `config-read-path.sh`

### Overview

Add `tmp` as the 11th configurable path key so the initialise skill and
review-pr can resolve it consistently.

### Changes Required:

#### 1. `scripts/config-read-path.sh`

**File**: `scripts/config-read-path.sh`
**Changes**: Add `tmp` to the documented path keys comment block.

Current (lines 7-17):

```bash
# Path keys:
#   plans         → where plans are written (default: meta/plans)
#   research      → where research docs are written (default: meta/research)
#   decisions     → where ADRs are written (default: meta/decisions)
#   prs           → where PR descriptions are written (default: meta/prs)
#   validations   → where validation reports are written (default: meta/validations)
#   review_plans  → where plan reviews are written (default: meta/reviews/plans)
#   review_prs    → where PR review working dirs go (default: meta/reviews/prs)
#   templates     → where user templates are found (default: meta/templates)
#   tickets       → where ticket files are stored (default: meta/tickets)
#   notes         → where notes are stored (default: meta/notes)
```

Updated:

```bash
# Path keys:
#   plans         → where plans are written (default: meta/plans)
#   research      → where research docs are written (default: meta/research)
#   decisions     → where ADRs are written (default: meta/decisions)
#   prs           → where PR descriptions are written (default: meta/prs)
#   validations   → where validation reports are written (default: meta/validations)
#   review_plans  → where plan reviews are written (default: meta/reviews/plans)
#   review_prs    → where PR review working dirs go (default: meta/reviews/prs)
#   templates     → where user templates are found (default: meta/templates)
#   tickets       → where ticket files are stored (default: meta/tickets)
#   notes         → where notes are stored (default: meta/notes)
#   tmp           → ephemeral working data, gitignored (default: meta/tmp)
```

No functional change needed — `config-read-path.sh` already delegates any key
to `config-read-value.sh` generically. The comment is documentation only.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/config-read-path.sh tmp meta/tmp` outputs `meta/tmp`
- [ ] All existing path keys still resolve correctly:
  `bash scripts/config-read-path.sh plans meta/plans` outputs `meta/plans`

#### Manual Verification:

- [ ] Comment block in `config-read-path.sh` includes `tmp` entry

---

## Phase 2: Create the Initialise Skill

### Overview

Create `skills/config/initialise/SKILL.md` — a prompt-only skill that
prepares a consumer repository with all expected directories and gitignore
entries.

### Changes Required:

#### 1. `skills/config/initialise/SKILL.md`

**File**: `skills/config/initialise/SKILL.md` (new file)
**Changes**: Create the skill definition.

The skill should:

1. **Resolve all 11 path keys** by running `config-read-path.sh` for each:
  - `plans` (default: `meta/plans`)
  - `research` (default: `meta/research`)
  - `decisions` (default: `meta/decisions`)
  - `prs` (default: `meta/prs`)
  - `validations` (default: `meta/validations`)
  - `review_plans` (default: `meta/reviews/plans`)
  - `review_prs` (default: `meta/reviews/prs`)
  - `templates` (default: `meta/templates`)
  - `tickets` (default: `meta/tickets`)
  - `notes` (default: `meta/notes`)
  - `tmp` (default: `meta/tmp`)

2. **Create each directory** with `mkdir -p` and place a `.gitkeep` file
   (empty, created via `touch`) if one doesn't already exist.

3. **Set up the tmp directory specially**:
  - Create `.gitkeep` (as above)
  - Write a `.gitignore` file containing:
    ```
    # Ignore everything in this directory except the directory itself
    *
    !.gitkeep
    !.gitignore
    ```
  - This inner `.gitignore` is the sole mechanism for ignoring tmp contents.
    Do **not** add the tmp path to the consumer's root `.gitignore` — a
    root-level ignore entry (e.g., `meta/tmp/`) would prevent git from
    descending into the directory at all, which means the inner `.gitignore`'s
    `!.gitkeep` and `!.gitignore` exclusions would never take effect and the
    directory would not survive fresh clones.

4. **Add `.claude/accelerator.local.md` to the consumer's root `.gitignore`**
   if not already present. Check before appending.

5. **Report results**: For each action, indicate whether it was created or
   already existed. Only output the summary after all steps have completed
   successfully — if the session is interrupted mid-execution, no
   "Initialisation complete" message should appear. Use a summary format
   like:
   ```
   Initialisation complete:

   Directories:
     ✓ meta/plans (created)
     ✓ meta/research (already exists)
     ...

   Gitignore entries:
     ✓ .claude/accelerator.local.md (added)

   Tmp directory:
     ✓ meta/tmp/.gitignore (created)
     ✓ meta/tmp/.gitkeep (created)
   ```

**Skill frontmatter**:

```yaml
---
name: initialise
description: Prepare a repository with the directories and gitignore entries
  that Accelerator skills expect. Safe to run repeatedly.
argument-hint: "(no arguments — safe to run repeatedly)"
disable-model-invocation: true
---
```

**Skill body structure** (prompt instructions, not executable code):

The skill should be structured as a sequence of numbered steps with bash
commands that the LLM executes. It should use the `!` backtick syntax to
resolve each path at skill load time, similar to how `review-pr` resolves
`{pr reviews directory}`:

```
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
... (all 11 paths)
```

Then provide step-by-step instructions referencing these resolved values.

Each step should use bash commands via the Bash tool. The skill should track
what was created vs. what already existed, and present the summary at the end.

The `.gitignore` operations should:

- If no `.gitignore` exists at the project root, create one containing the
  required entries
- Read the current `.gitignore` content
- For each entry to add, check if it is already present by trimming
  whitespace from both the existing lines and the candidate entry before
  comparison. Also check for the trailing-slash variant (e.g., check for
  both `.claude/accelerator.local.md` as-is). This prevents duplicate
  entries when the existing file has minor formatting differences.
- Append missing entries
- Not modify existing entries

### Success Criteria:

#### Automated Verification:

- [ ] File exists at `skills/config/initialise/SKILL.md`
- [ ] Frontmatter has `name: initialise` and `disable-model-invocation: true`
- [ ] No changes needed to `plugin.json` (verify `./skills/config/` is
  already registered)

#### Manual Verification:

- [ ] Run `/accelerator:initialise` in a fresh consumer repo — all 11
  directories are created with `.gitkeep` files
- [ ] `meta/tmp/.gitignore` contains the expected content (`*`, `!.gitkeep`,
  `!.gitignore`)
- [ ] Consumer's root `.gitignore` does NOT contain `meta/tmp/` (tmp is
  managed solely by its inner `.gitignore`)
- [ ] Consumer's `.gitignore` contains `.claude/accelerator.local.md`
- [ ] Run again — all items reported as "already exists"/"already present"
- [ ] Configure `paths.tmp: custom/tmp` and run again — `custom/tmp/` is
  created with its own `.gitignore` (inner ignore mechanism)

---

## Phase 3: Migrate `review-pr` Ephemeral Files

### Overview

Update the `review-pr` skill to write ephemeral working files to
`{tmp directory}/pr-review-{number}/` instead of
`{pr reviews directory}/pr-review-{number}/`.

### Changes Required:

#### 1. `skills/github/review-pr/SKILL.md`

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Add a tmp directory path resolution and replace all ephemeral
directory references.

**a) Add tmp directory resolution (after line 23)**

Current line 23:

```
**PR reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
```

Add after it:

```
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`
```

**b) Update temp directory creation (lines 84-88)**

Current:

```
2. **Create temp directory** at `{pr reviews directory}/pr-review-{number}` (substituting
   the actual PR number):
   ```bash
   mkdir -p {pr reviews directory}/pr-review-{number}
   ```

```

Replace with:
```

2. **Create temp directory** at `{tmp directory}/pr-review-{number}` (
   substituting
   the actual PR number):
   ```bash
   mkdir -p {tmp directory}/pr-review-{number}
   ```

```

**c) Update diff/changed-files/description/commits writes (lines 92-95)**

Replace all four occurrences of `{pr reviews directory}/pr-review-{number}/`
with `{tmp directory}/pr-review-{number}/`:

```bash
gh pr diff {number} > {tmp directory}/pr-review-{number}/diff.patch
gh pr diff {number} --name-only > {tmp directory}/pr-review-{number}/changed-files.txt
gh pr view {number} --json body --jq '.body' > {tmp directory}/pr-review-{number}/pr-description.md
gh pr view {number} --json commits --jq '.commits[].messageHeadline' > {tmp directory}/pr-review-{number}/commits.txt
```

**d) Update head-sha/repo-info writes (lines 103-104)**

Replace both occurrences:

```bash
gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha' > {tmp directory}/pr-review-{number}/head-sha.txt
gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"' > {tmp directory}/pr-review-{number}/repo-info.txt
```

**e) Update agent prompt (line 250)**

Current:

```
The PR artefacts are in the temp directory at {pr reviews directory}/pr-review-{number}:
```

Replace with:

```
The PR artefacts are in the temp directory at {tmp directory}/pr-review-{number}:
```

**f) Update agent instructions to read from temp directory (lines 261-262)**

Current references to reading from `the temp directory` — these reference the
path set in (e). Verify these lines reference the directory by name rather
than by full path. If they use the full path, update to
`{tmp directory}/pr-review-{number}`.

**g) Update head-sha/repo-info reads (lines 528-530)**

Current:

```
1. Read the HEAD SHA and repo info from the temp directory at
   `{pr reviews directory}/pr-review-{number}/head-sha.txt` and
   `{pr reviews directory}/pr-review-{number}/repo-info.txt` using the Read tool.
```

Replace with:

```
1. Read the HEAD SHA and repo info from the temp directory at
   `{tmp directory}/pr-review-{number}/head-sha.txt` and
   `{tmp directory}/pr-review-{number}/repo-info.txt` using the Read tool.
```

**h) Update review-payload.json write and read (lines 553-556)**

Current:

```
3. Write the review payload JSON to
   `{pr reviews directory}/pr-review-{number}/review-payload.json`, then post the review:
   ```bash
   gh api repos/{owner}/{repo}/pulls/{number}/reviews \
     --method POST --input {pr reviews directory}/pr-review-{number}/review-payload.json
   ```

```

Replace with:
```

3. Write the review payload JSON to
   `{tmp directory}/pr-review-{number}/review-payload.json`, then post the
   review:
   ```bash
   gh api repos/{owner}/{repo}/pulls/{number}/reviews \
     --method POST --input {tmp directory}/pr-review-{number}/review-payload.json
   ```

```

**i) Update cleanup guidelines (lines 610-617)**

Current:
```

7. **Clean up temp directory only at session end** — agents may need to
   re-reference the PR context during follow-up discussion.

   The `{pr reviews directory}/pr-review-{number}/` directory contains ephemeral
   working
   data (diff, changed-files, PR description, commits, head SHA, repo info,
   review payload JSON) used during the review session. The review itself
   (summary, inline comments, per-lens results) is persisted separately to
   `{pr reviews directory}/{number}-review-{N}.md` and is NOT stored in tmp/.

```

Replace with:
```

7. **Clean up temp directory only at session end** — agents may need to
   re-reference the PR context during follow-up discussion.

   The `{tmp directory}/pr-review-{number}/` directory contains ephemeral
   working
   data (diff, changed-files, PR description, commits, head SHA, repo info,
   review payload JSON) used during the review session. The review itself
   (summary, inline comments, per-lens results) is persisted separately to
   `{pr reviews directory}/{number}-review-{N}.md`.

```

Note: The final sentence about "is NOT stored in tmp/" is removed since the
ephemeral data now *is* stored in the tmp directory.

**j) Migration note for stale ephemeral directories**

After this migration, existing `{pr reviews directory}/pr-review-*/`
directories from previous review sessions will remain at their old location.
These are inert but un-gitignored. Add a note to the CHANGELOG (see Phase 5)
documenting that these can be safely deleted:

```
After upgrading, you can safely remove any existing pr-review-*/ directories
under your PR reviews path (default: meta/reviews/prs/):
  rm -rf meta/reviews/prs/pr-review-*/
```

### Success Criteria:

#### Automated Verification:

- [ ] No remaining references to `{pr reviews directory}/pr-review-` in
      `skills/github/review-pr/SKILL.md` (search for the pattern)
- [ ] `{tmp directory}` is referenced correctly throughout
- [ ] The `{pr reviews directory}` variable is still used for the persistent
      review artifact path (`{pr reviews directory}/{number}-review-{N}.md`)

#### Manual Verification:

- [ ] Run `/accelerator:review-pr` on a test PR — ephemeral files appear in
      `meta/tmp/pr-review-{number}/`, not in `meta/reviews/prs/pr-review-{number}/`
- [ ] Persistent review artifact still written to `meta/reviews/prs/`
- [ ] The tmp directory contents are gitignored (not shown in `jj status`)

---

## Phase 4: Add Initialisation Detection to SessionStart Hook

### Overview

Enhance the SessionStart hook to detect when a consumer repository has not
been initialised and suggest running `/accelerator:initialise`.

### Changes Required:

#### 1. `scripts/config-summary.sh`

**File**: `scripts/config-summary.sh`
**Changes**: Add an initialisation check before the existing config detection
logic. This should run regardless of whether config files exist (a repo can
be uninitialised even with config).

The check should:

1. Resolve the `tmp` path via `config-read-path.sh tmp meta/tmp`
2. Check whether the `.gitignore` file inside the resolved tmp directory
   exists (i.e., `{tmp path}/.gitignore`). This file is the initialisation
   sentinel — it is only created by the `initialise` skill, not by `mkdir -p`
   when other skills (e.g., `review-pr`) create the tmp directory on demand.
   A code comment should explain this choice:
   `# Check for tmp/.gitignore (not just tmp/) as the initialisation sentinel,`
   `# because review-pr creates tmp/ organically via mkdir -p.`
3. If the sentinel does not exist, append an initialisation hint to the
   summary output

The hint should be added at the end of the summary (or as the entire output
if no config summary would otherwise be produced). The message should be:

```
Accelerator has not been initialised in this repository. Type /accelerator:initialise at the prompt to set up the expected directory structure and gitignore entries.
```

Note: Use "Type" rather than "Run" since the hint appears in hook context
output and the user needs to type it at the prompt, not execute it as a shell
command.

**Implementation detail**: Currently `config-summary.sh` exits at line 17
when no config files exist (`[ ${#FILES[@]} -eq 0 ] && exit 0`). This early
exit needs to be adjusted so the initialisation check can still run. The
restructured flow should be:

1. Discover config files (existing logic)
2. Check initialisation state (new logic — runs regardless of config files)
3. If no config files AND already initialised, exit silently
4. If no config files AND NOT initialised, output only the initialisation
   hint (do NOT append the trailing "Skills will read this configuration..."
   instruction since there is no configuration to read)
5. If config files exist, output the config summary (including the trailing
   instruction) AND append the initialisation hint if not initialised

The initialisation check is lightweight — one `config-read-path.sh` call
and one file existence check.

#### 2. `hooks/config-detect.sh`

**File**: `hooks/config-detect.sh`
**Changes**: No changes needed. The hook already passes through whatever
`config-summary.sh` outputs, so the new initialisation hint will
automatically appear in the session context.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/config-summary.sh` in an uninitialised repo (no
      `meta/tmp/.gitignore`) outputs the initialisation hint
- [ ] `bash scripts/config-summary.sh` in an initialised repo does not
      output the hint
- [ ] `bash scripts/config-summary.sh` in a repo with config but no
      initialisation outputs both the config summary and the hint
- [ ] `bash scripts/config-summary.sh` in a repo where `meta/tmp/` exists
      (created by `review-pr`) but `meta/tmp/.gitignore` does not — still
      outputs the hint (sentinel is the file, not the directory)
- [ ] The no-config-but-uninitialised path does NOT include the trailing
      "Skills will read this configuration..." instruction

#### Manual Verification:

- [ ] Start a new Claude session in an uninitialised consumer repo — the
      session context includes the initialisation suggestion
- [ ] Start a new session after running `/accelerator:initialise` — no
      initialisation suggestion appears

---

## Phase 5: Update Documentation

### Overview

Update the `configure` skill's paths reference table and README to include
the new `tmp` path key.

### Changes Required:

#### 1. `skills/config/configure/SKILL.md`

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add `tmp` to the paths table (around line 341) and the example
configuration block (around line 355).

Add to the table after the `notes` row:
```

| `tmp` | `meta/tmp` | Ephemeral working data (gitignored) |

```

Add to the example configuration:
```yaml
  tmp: docs/tmp
```

#### 2. `CHANGELOG.md` and `plugin.json` version bump

**Files**: `CHANGELOG.md`, `.claude-plugin/plugin.json`
**Changes**: Add a CHANGELOG entry under a new version (e.g., 1.8.0)
documenting:
- New `/accelerator:initialise` skill for repository setup
- New `tmp` configurable path key
- `review-pr` ephemeral files now written to `{tmp directory}/pr-review-{number}/`
  instead of `{pr reviews directory}/pr-review-{number}/`
- Migration note: existing `pr-review-*/` directories under the PR reviews
  path can be safely deleted
- SessionStart hook now detects uninitialised repositories

Bump the `version` field in `plugin.json` to match.

#### 3. `README.md`

**File**: `README.md`
**Changes**: Update the `meta/` directory table (lines 77-87):
- Verify `tmp/` is listed as written by `review-pr` (correct after Phase 3)
- Add missing `tickets/` row (manual, used by `create-plan`)
- Add missing `notes/` row (manual)

Also update the sentence at lines 73-74 that says "directories are created on
first use by their respective skills" to mention the initialise skill:

Current:

```
serves as persistent state for the development workflow. Each skill reads from
and writes to predictable paths within it (directories are created on first use
by their respective skills).
```

Updated:

```
serves as persistent state for the development workflow. Each skill reads from
and writes to predictable paths within it. Run `/accelerator:initialise` to
create all directories up front, or let skills create them on first use.
```

### Success Criteria:

#### Automated Verification:

- [ ] `tmp` appears in the paths table in `skills/config/configure/SKILL.md`
- [ ] `tmp` appears in the example configuration block
- [ ] CHANGELOG has an entry for the new version documenting all changes
- [ ] `plugin.json` version is bumped
- [ ] README directory table includes `tickets/` and `notes/` rows

#### Manual Verification:

- [ ] Run `/accelerator:configure help` — the `tmp` path key is listed in
  the paths section
- [ ] README accurately reflects the current behaviour

---

## Testing Strategy

### Integration Testing:

Test the full flow in a clean consumer repository:

1. Start a session — verify the initialisation hint appears
2. Run `/accelerator:initialise` — verify all directories created
3. Run `/accelerator:initialise` again — verify idempotent (no duplicates)
4. Run `/accelerator:review-pr` on a test PR — verify ephemeral files in
   `meta/tmp/pr-review-{number}/`
5. Start a new session — verify the initialisation hint no longer appears

### Edge Cases:

- Consumer repo has no `.gitignore` — initialise should create one
- Consumer repo has custom paths configured — initialise honours overrides
- Consumer repo already has some directories — initialise creates missing
  ones and reports existing ones
- `.claude/accelerator.local.md` already in `.gitignore` with trailing
  whitespace — should not duplicate (dedup trims whitespace)

### Manual Testing Steps:

1. Clone a fresh test repo with no meta/ directory
2. Install the Accelerator plugin
3. Start a Claude session and verify the initialisation hint
4. Run `/accelerator:initialise` and verify all output
5. Check `jj status` / `git status` — `.gitkeep` files should be tracked,
   tmp contents should not be
6. Run `/accelerator:review-pr` on a real PR
7. Verify ephemeral files in `meta/tmp/` and review artifact in
   `meta/reviews/prs/`

## Performance Considerations

- The initialise skill runs `config-read-path.sh` 11 times (once per path
  key). Each invocation sources `config-common.sh` and `vcs-common.sh` and
  reads up to 2 config files. This is negligible overhead for a one-time
  setup operation.
- The SessionStart hook adds one `config-read-path.sh` call and one directory
  existence check. This adds ~50ms to session start, which is acceptable.

## References

- Research: `meta/research/2026-03-28-initialise-skill-requirements.md`
- Configure skill (pattern to follow): `skills/config/configure/SKILL.md`
- Path resolution: `scripts/config-read-path.sh`
- Review-pr skill (migration target): `skills/github/review-pr/SKILL.md`
- SessionStart hook: `hooks/config-detect.sh` → `scripts/config-summary.sh`
- Plugin manifest: `.claude-plugin/plugin.json`
