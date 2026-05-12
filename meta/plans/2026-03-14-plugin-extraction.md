# Accelerator Plugin Extraction Plan

## Overview

Extract the 7 agents, 18 skills (+ 1 script), and `meta/` directory from
`~/.claude/` into a standalone Claude Code plugin at
`~/Code/organisations/atomic/company/accelerator`. The initial extraction
preserves the flat skill layout for a clean, verifiable first commit.
Skill restructuring into logical groups is a subsequent phase, allowing each
change to be committed and verified independently.

## Current State Analysis

### Source Inventory

- **7 agents** in `~/.claude/agents/` (all `.md` files)
- **18 SKILL.md files** across 18 skill directories in `~/.claude/skills/`
- **1 script**: `research-codebase/scripts/research-metadata.sh`
- **13 meta documents**: 6 plans + 7 research documents in `~/.claude/meta/`

### Cross-References Requiring Update (3 files, 5 references)

| Source                       | Line | Reference                                                         |
|------------------------------|------|-------------------------------------------------------------------|
| `review-pr/SKILL.md`         | 191  | `~/.claude/skills/[lens]-lens/SKILL.md`                           |
| `review-pr/SKILL.md`         | 195  | `~/.claude/skills/pr-review-output-format/SKILL.md`               |
| `review-plan/SKILL.md`       | 144  | `~/.claude/skills/[lens]-lens/SKILL.md`                           |
| `review-plan/SKILL.md`       | 148  | `~/.claude/skills/plan-review-output-format/SKILL.md`             |
| `research-codebase/SKILL.md` | 95   | `~/.claude/skills/research-codebase/scripts/research-metadata.sh` |

### Key Discoveries

- All path references use `~/.claude/` prefix (no `$HOME` or absolute paths)
- No files exist at the target directory yet — clean slate
- Skills use `disable-model-invocation: true` in frontmatter consistently
- The `meta/` directory contains paired research/plan documents that provide
  context for how the skills and agents evolved

## Desired End State

A fully functional Claude Code plugin at
`~/Code/organisations/atomic/company/accelerator` with:

1. All agents and skills working identically to their `~/.claude/` counterparts
2. Skills restructured into logical groups for better discoverability
3. Marketplace-ready metadata, documentation, and licensing
4. The `meta/` directory preserved for historical context
5. A git repository with commits for extraction (flat) and restructuring
6. Original files removed from `~/.claude/` to avoid duplication

### Verification

- `claude --plugin-dir ~/Code/organisations/atomic/company/accelerator` loads
  all agents and skills
- All 9 user-invocable skills appear with `/accelerator:` prefix
- `/accelerator:review-pr` successfully spawns reviewer agents with lens skills
- `/accelerator:research-codebase` runs the metadata script correctly
- All 7 agents appear in the agent selection list

## What We're NOT Doing

- Modifying skill behavior or logic (content stays the same, only paths change)
- Migrating `settings.json`, `keybindings.json`, or memory system
- Creating new skills or agents
- Setting up CI/CD or automated publishing
- Registering on any marketplace (just preparing the metadata)

## Implementation Approach

The extraction happens in 8 phases: create the plugin scaffold, copy skills
(flat), copy agents and meta, update path references, add documentation and
create the initial commit, restructure skills into groups, update path
references for the new layout, and clean up originals. Phases 1–5 produce a
working plugin with a flat skill layout and a clean commit. Phases 6–7
restructure into logical groups as a separate commit. Phase 8 cleans up
originals.

### Approach Rationale

Following the research recommendation (Option A), the initial extraction
preserves the flat `~/.claude/skills/` layout. This isolates the extraction
from the restructuring, so each change can be verified and committed
independently. If an issue surfaces, it's immediately clear whether it's an
extraction bug or a restructuring bug.

Restructuring into logical groups (Option B from research) happens as a
subsequent phase with its own commit, once the flat extraction is proven
working.

---

## Phase 1: Create Plugin Scaffold

### Overview

Create the target directory, initialize git, and set up the plugin manifest and
marketplace metadata.

### Changes Required

#### 1. Directory Structure

Create the base directory tree:

```bash
mkdir -p ~/Code/organisations/atomic/company/accelerator/.claude-plugin
mkdir -p ~/Code/organisations/atomic/company/accelerator/agents
mkdir -p ~/Code/organisations/atomic/company/accelerator/skills
mkdir -p ~/Code/organisations/atomic/company/accelerator/meta
```

#### 2. Plugin Manifest

**File**: `.claude-plugin/plugin.json`

```json
{
  "name": "accelerator",
  "version": "1.0.0",
  "description": "Development acceleration toolkit with multi-lens code review, implementation planning, codebase research, and git workflow automation",
  "author": {
    "name": "Toby Clemson",
    "email": "tobyclemson@gmail.com"
  }
}
```

#### 3. Initialize Git Repository

```bash
cd ~/Code/organisations/atomic/company/accelerator
git init
```

### Success Criteria

#### Automated Verification

- [x] Directory exists:
  `test -d ~/Code/organisations/atomic/company/accelerator/.claude-plugin`
- [x] Manifest is valid JSON:
  `python3 -m json.tool ~/Code/organisations/atomic/company/accelerator/.claude-plugin/plugin.json`
- [x] Git repo initialized:
  `git -C ~/Code/organisations/atomic/company/accelerator rev-parse --git-dir`

#### Manual Verification

- [x] Directory structure matches the proposed layout

---

## Phase 2: Copy Skills (Flat)

### Overview

Copy all 18 SKILL.md files and the research-metadata.sh script into a flat
skill directory layout, mirroring the original `~/.claude/skills/` structure.

### Changes Required

#### 1. Copy All Skills

```bash
SRC=~/.claude/skills
DST=~/Code/organisations/atomic/company/accelerator/skills

for skill_dir in $SRC/*/; do
  cp -r "$skill_dir" "$DST/"
done
```

This copies all 18 skill directories with their contents (SKILL.md files and
scripts) directly into `skills/`, preserving the flat layout.

#### 2. Ensure Script is Executable

```bash
chmod +x $DST/research-codebase/scripts/research-metadata.sh
```

### Success Criteria

#### Automated Verification

- [x] All 18 SKILL.md files copied:
  `find ~/Code/organisations/atomic/company/accelerator/skills -name "SKILL.md" | wc -l`
  returns 18
- [x] Script exists and is executable:
  `test -x ~/Code/organisations/atomic/company/accelerator/skills/research-codebase/scripts/research-metadata.sh`
- [x] Each skill directory from `~/.claude/skills/` has a corresponding
  directory in `skills/`:
  `diff <(ls ~/.claude/skills/) <(ls ~/Code/organisations/atomic/company/accelerator/skills/)`
  returns empty

---

## Phase 3: Copy Agents and Meta

### Overview

Copy all 7 agent definitions and the entire `meta/` directory.

### Changes Required

#### 1. Agents

```bash
cp ~/.claude/agents/*.md ~/Code/organisations/atomic/company/accelerator/agents/
```

#### 2. Meta Directory

```bash
cp -r ~/.claude/meta/* ~/Code/organisations/atomic/company/accelerator/meta/
```

### Success Criteria

#### Automated Verification

- [ ] All 7 agents copied:
  `ls ~/Code/organisations/atomic/company/accelerator/agents/*.md | wc -l`
  returns 7
- [ ] Meta plans copied:
  `ls ~/Code/organisations/atomic/company/accelerator/meta/plans/*.md | wc -l`
  returns 6
- [ ] Meta research copied:
  `ls ~/Code/organisations/atomic/company/accelerator/meta/research/codebase/*.md | wc -l`
  returns 7

---

## Phase 4: Update Path References

### Overview

Replace all `~/.claude/skills/` and `~/.claude/` path references in skill files
with `${CLAUDE_PLUGIN_ROOT}/skills/` paths. Since the flat layout is preserved,
this is a straightforward prefix substitution.

### Changes Required

#### 1. `skills/review-pr/SKILL.md`

**Line ~191**: Update lens skill path

```
# Before:
Read the lens skill at: ~/.claude/skills/[lens]-lens/SKILL.md

# After:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/[lens]-lens/SKILL.md
```

**Line ~195**: Update output format path

```
# Before:
Read the output format at: ~/.claude/skills/pr-review-output-format/SKILL.md

# After:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/pr-review-output-format/SKILL.md
```

#### 2. `skills/review-plan/SKILL.md`

**Line ~144**: Update lens skill path

```
# Before:
Read the lens skill at: ~/.claude/skills/[lens]-lens/SKILL.md

# After:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/[lens]-lens/SKILL.md
```

**Line ~148**: Update output format path

```
# Before:
Read the output format at: ~/.claude/skills/plan-review-output-format/SKILL.md

# After:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/plan-review-output-format/SKILL.md
```

#### 3. `skills/research-codebase/SKILL.md`

**Line ~95**: Update script path

```
# Before:
- Run the `~/.claude/skills/research-codebase/scripts/research-metadata.sh`

# After:
- Run the `${CLAUDE_PLUGIN_ROOT}/skills/research-codebase/scripts/research-metadata.sh`
```

### Success Criteria

#### Automated Verification

- [ ] No remaining `~/.claude/` references in plugin (excluding meta/):
  `grep -r "~/.claude/" ~/Code/organisations/atomic/company/accelerator/ --exclude-dir=meta`
  returns nothing
- [ ] `${CLAUDE_PLUGIN_ROOT}` appears in the 3 updated files:
  `grep -rl "CLAUDE_PLUGIN_ROOT" ~/Code/organisations/atomic/company/accelerator/skills/ | wc -l`
  returns 3

#### Path Resolution Verification

For each `${CLAUDE_PLUGIN_ROOT}` reference, substitute the actual plugin root
and confirm the target file exists:

```bash
ROOT=~/Code/organisations/atomic/company/accelerator

# Verify all referenced paths resolve to real files
test -f $ROOT/skills/architecture-lens/SKILL.md
test -f $ROOT/skills/code-quality-lens/SKILL.md
test -f $ROOT/skills/performance-lens/SKILL.md
test -f $ROOT/skills/security-lens/SKILL.md
test -f $ROOT/skills/standards-lens/SKILL.md
test -f $ROOT/skills/test-coverage-lens/SKILL.md
test -f $ROOT/skills/usability-lens/SKILL.md
test -f $ROOT/skills/pr-review-output-format/SKILL.md
test -f $ROOT/skills/plan-review-output-format/SKILL.md
test -x $ROOT/skills/research-codebase/scripts/research-metadata.sh
```

#### `${CLAUDE_PLUGIN_ROOT}` Sub-Agent Resolution Test

The review-pr and review-plan skills pass `${CLAUDE_PLUGIN_ROOT}` paths in
agent prompts to spawned reviewer sub-agents. Verify the variable resolves in
that context:

- [ ] Spawn a single reviewer agent with a prompt containing a
  `${CLAUDE_PLUGIN_ROOT}` path and confirm the agent can Read the referenced
  file. For example, run `/accelerator:review-pr` with a single lens (e.g.,
  `focus on architecture`) on a test PR and verify the agent output includes
  lens-specific content that proves the SKILL.md was loaded.
- [ ] If `${CLAUDE_PLUGIN_ROOT}` does not resolve in agent prompts, update the
  orchestrator skills (review-pr, review-plan, research-codebase) to resolve
  the path to an absolute path before injecting it into the agent prompt.

#### Manual Verification

- [ ] Read each of the 3 modified files and confirm paths look correct

---

## Phase 5: Documentation and Distribution Preparation

### Overview

Add marketplace metadata, LICENSE, README, and a .gitignore. Create the initial
commit.

### Changes Required

#### 1. LICENSE

**File**: `LICENSE`

Use MIT License with copyright holder "Toby Clemson" and year 2026.

#### 2. README.md

**File**: `README.md`

Content should include:

- Plugin name and description
- Table of user-invocable skills (9) with descriptions
- Table of agents (7) with descriptions
- Installation instructions (`claude plugin install` and `--plugin-dir`)
- Usage examples showing `/accelerator:skill-name` invocation
- Brief description of the review lens system
- Brief description of the planning workflow
- License section

#### 3. CHANGELOG.md

**File**: `CHANGELOG.md`

```markdown
# Changelog

## 1.0.0 — 2026-03-14

Initial extraction from `~/.claude/` into a standalone Claude Code plugin.

- 7 agents: codebase-analyser, codebase-locator, codebase-pattern-finder,
  documents-analyser, documents-locator, reviewer, web-search-researcher
- 9 user-invocable skills: commit, create-plan, describe-pr, implement-plan,
  research-codebase, respond-to-pr, review-plan, review-pr, validate-plan
- 9 supporting skills: 7 review lenses + 2 output formats
- Skills organized into logical groups: git/, planning/, review/, research/
```

Note: The CHANGELOG is created during Phase 5 (initial commit) with the flat
layout entry, then updated during Phase 7 to reflect the restructured layout.
Alternatively, since both phases happen before release, write the final
CHANGELOG content in Phase 7 after restructuring is complete.

#### 4. .gitignore

**File**: `.gitignore`

```
.DS_Store
*.swp
*~
```

#### 5. Initial Commit

```bash
cd ~/Code/organisations/atomic/company/accelerator
git add -A
git commit -m "Initial extraction of accelerator plugin from ~/.claude/

Extracts 7 agents, 18 skills, and meta directory into a standalone
Claude Code plugin with flat skill layout mirroring the original
~/.claude/skills/ structure.

All ~/.claude/ path references updated to use \${CLAUDE_PLUGIN_ROOT}."
```

### Success Criteria

#### Automated Verification

- [ ] LICENSE file exists:
  `test -f ~/Code/organisations/atomic/company/accelerator/LICENSE`
- [ ] README.md exists:
  `test -f ~/Code/organisations/atomic/company/accelerator/README.md`
- [ ] CHANGELOG.md exists:
  `test -f ~/Code/organisations/atomic/company/accelerator/CHANGELOG.md`
- [ ] .gitignore exists:
  `test -f ~/Code/organisations/atomic/company/accelerator/.gitignore`
- [ ] Clean git status after commit:
  `git -C ~/Code/organisations/atomic/company/accelerator status --porcelain`
  returns empty
- [ ] Plugin loads:
  `claude --plugin-dir ~/Code/organisations/atomic/company/accelerator --print-plugins 2>&1`
  shows accelerator

#### Manual Verification

- [ ] README is clear and comprehensive
- [ ] `/accelerator:commit` works in a test project
- [ ] `/accelerator:review-pr` spawns reviewer agents correctly
- [ ] `/accelerator:research-codebase` runs the metadata script
- [ ] All 7 agents appear in agent list when plugin is loaded

---

## Phase 6: Restructure Skills into Groups

### Overview

Reorganize the flat skill layout into logical groups using subdirectories. The
plugin system discovers skills by scanning for `SKILL.md` files recursively, so
nesting within `skills/` is supported. The skill `name` in frontmatter
determines the invocation name, not the directory path.

### Target Layout

```
skills/
├── git/
│   ├── commit/SKILL.md
│   ├── describe-pr/SKILL.md
│   └── respond-to-pr/SKILL.md
├── planning/
│   ├── create-plan/SKILL.md
│   ├── implement-plan/SKILL.md
│   └── validate-plan/SKILL.md
├── review/
│   ├── review-pr/SKILL.md
│   ├── review-plan/SKILL.md
│   ├── lenses/
│   │   ├── architecture-lens/SKILL.md
│   │   ├── code-quality-lens/SKILL.md
│   │   ├── performance-lens/SKILL.md
│   │   ├── security-lens/SKILL.md
│   │   ├── standards-lens/SKILL.md
│   │   ├── test-coverage-lens/SKILL.md
│   │   └── usability-lens/SKILL.md
│   └── output-formats/
│       ├── pr-review-output-format/SKILL.md
│       └── plan-review-output-format/SKILL.md
└── research/
    └── research-codebase/
        ├── SKILL.md
        └── scripts/research-metadata.sh
```

### Changes Required

#### 1. Create Group Directories

```bash
DST=~/Code/organisations/atomic/company/accelerator/skills

mkdir -p $DST/git
mkdir -p $DST/planning
mkdir -p $DST/review/lenses
mkdir -p $DST/review/output-formats
mkdir -p $DST/research
```

#### 2. Move Skills into Groups

```bash
# Git
mv $DST/commit $DST/git/
mv $DST/describe-pr $DST/git/
mv $DST/respond-to-pr $DST/git/

# Planning
mv $DST/create-plan $DST/planning/
mv $DST/implement-plan $DST/planning/
mv $DST/validate-plan $DST/planning/

# Review
mv $DST/review-pr $DST/review/
mv $DST/review-plan $DST/review/

# Lenses
for lens in architecture code-quality performance security standards test-coverage usability; do
  mv $DST/${lens}-lens $DST/review/lenses/
done

# Output formats
mv $DST/pr-review-output-format $DST/review/output-formats/
mv $DST/plan-review-output-format $DST/review/output-formats/

# Research
mv $DST/research-codebase $DST/research/
```

### Success Criteria

#### Automated Verification

- [ ] All 18 SKILL.md files still present:
  `find ~/Code/organisations/atomic/company/accelerator/skills -name "SKILL.md" | wc -l`
  returns 18
- [ ] Git skills: 3 SKILL.md files in `skills/git/`
- [ ] Planning skills: 3 SKILL.md files in `skills/planning/`
- [ ] Review skills: 2 SKILL.md files in `skills/review/` (direct), 7 in
  `skills/review/lenses/`, 2 in `skills/review/output-formats/`
- [ ] Research skills: 1 SKILL.md file in `skills/research/`
- [ ] Script still executable:
  `test -x ~/Code/organisations/atomic/company/accelerator/skills/research/research-codebase/scripts/research-metadata.sh`
- [ ] No skill directories remain at the top level (all moved into groups):
  `ls -d ~/Code/organisations/atomic/company/accelerator/skills/*/SKILL.md 2>/dev/null | wc -l`
  returns 0

#### Plugin Verification

- [ ] All 9 user-invocable skills still appear with `accelerator:` prefix after
  restructuring (frontmatter `name` not affected by directory move)

---

## Phase 7: Update Path References for Restructured Layout

### Overview

Update the `${CLAUDE_PLUGIN_ROOT}` paths in the 3 cross-referencing skills to
reflect the new nested directory structure.

### Changes Required

#### 1. `skills/review/review-pr/SKILL.md`

**Line ~191**: Update lens skill path

```
# Before:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/[lens]-lens/SKILL.md

# After:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

**Line ~195**: Update output format path

```
# Before:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/pr-review-output-format/SKILL.md

# After:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/pr-review-output-format/SKILL.md
```

#### 2. `skills/review/review-plan/SKILL.md`

**Line ~144**: Update lens skill path

```
# Before:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/[lens]-lens/SKILL.md

# After:
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

**Line ~148**: Update output format path

```
# Before:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/plan-review-output-format/SKILL.md

# After:
Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/plan-review-output-format/SKILL.md
```

#### 3. `skills/research/research-codebase/SKILL.md`

**Line ~95**: Update script path

```
# Before:
- Run the `${CLAUDE_PLUGIN_ROOT}/skills/research-codebase/scripts/research-metadata.sh`

# After:
- Run the `${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/research-metadata.sh`
```

#### 4. Commit

```bash
cd ~/Code/organisations/atomic/company/accelerator
git add -A
git commit -m "Restructure skills into logical groups

Organize 18 skills into 4 groups for better discoverability:
- git/: commit, describe-pr, respond-to-pr
- planning/: create-plan, implement-plan, validate-plan
- review/: review-pr, review-plan, 7 lenses, 2 output formats
- research/: research-codebase

Update cross-reference paths to reflect new nested layout."
```

### Success Criteria

#### Automated Verification

- [ ] No flat `${CLAUDE_PLUGIN_ROOT}/skills/[lens]` paths remain (all should
  include group prefix):
  `grep "CLAUDE_PLUGIN_ROOT}/skills/[a-z]" ~/Code/organisations/atomic/company/accelerator/skills/review/review-pr/SKILL.md`
  returns nothing (all paths now include `/review/lenses/` or
  `/review/output-formats/`)
- [ ] Lens path pattern is correct:
  `grep "CLAUDE_PLUGIN_ROOT.*review/lenses" ~/Code/organisations/atomic/company/accelerator/skills/review/review-pr/SKILL.md`
  matches expected pattern

#### Path Resolution Verification

```bash
ROOT=~/Code/organisations/atomic/company/accelerator

test -f $ROOT/skills/review/lenses/architecture-lens/SKILL.md
test -f $ROOT/skills/review/lenses/code-quality-lens/SKILL.md
test -f $ROOT/skills/review/lenses/performance-lens/SKILL.md
test -f $ROOT/skills/review/lenses/security-lens/SKILL.md
test -f $ROOT/skills/review/lenses/standards-lens/SKILL.md
test -f $ROOT/skills/review/lenses/test-coverage-lens/SKILL.md
test -f $ROOT/skills/review/lenses/usability-lens/SKILL.md
test -f $ROOT/skills/review/output-formats/pr-review-output-format/SKILL.md
test -f $ROOT/skills/review/output-formats/plan-review-output-format/SKILL.md
test -x $ROOT/skills/research/research-codebase/scripts/research-metadata.sh
```

#### Manual Verification

- [ ] Read each of the 3 modified files and confirm paths look correct
- [ ] Clean git status after commit

---

## Phase 8: Cleanup

### Overview

Remove the original agents and skills from `~/.claude/` now that the plugin is
verified. The `meta/` directory stays in `~/.claude/` as it's also used by the
memory system and may be referenced by project-specific configurations.

### Changes Required

#### 0. Create Backup

Before any deletions, archive the originals for rollback:

```bash
tar czf ~/.claude/pre-extraction-backup.tar.gz -C ~/.claude agents skills
```

To restore if needed:

```bash
tar xzf ~/.claude/pre-extraction-backup.tar.gz -C ~/.claude
```

#### 1. Remove Original Agents

```bash
rm ~/.claude/agents/codebase-analyser.md
rm ~/.claude/agents/codebase-locator.md
rm ~/.claude/agents/codebase-pattern-finder.md
rm ~/.claude/agents/documents-analyser.md
rm ~/.claude/agents/documents-locator.md
rm ~/.claude/agents/reviewer.md
rm ~/.claude/agents/web-search-researcher.md
```

#### 2. Remove Original Skills

```bash
rm -rf ~/.claude/skills/architecture-lens
rm -rf ~/.claude/skills/code-quality-lens
rm -rf ~/.claude/skills/commit
rm -rf ~/.claude/skills/create-plan
rm -rf ~/.claude/skills/describe-pr
rm -rf ~/.claude/skills/implement-plan
rm -rf ~/.claude/skills/performance-lens
rm -rf ~/.claude/skills/plan-review-output-format
rm -rf ~/.claude/skills/pr-review-output-format
rm -rf ~/.claude/skills/research-codebase
rm -rf ~/.claude/skills/respond-to-pr
rm -rf ~/.claude/skills/review-plan
rm -rf ~/.claude/skills/review-pr
rm -rf ~/.claude/skills/security-lens
rm -rf ~/.claude/skills/standards-lens
rm -rf ~/.claude/skills/test-coverage-lens
rm -rf ~/.claude/skills/usability-lens
rm -rf ~/.claude/skills/validate-plan
```

### Success Criteria

#### Automated Verification

- [ ] No agent files remain: `ls ~/.claude/agents/*.md 2>/dev/null | wc -l`
  returns 0
- [ ] No skill directories remain:
  `ls -d ~/.claude/skills/*/ 2>/dev/null | wc -l` returns 0
- [ ] Meta directory still intact: `ls ~/.claude/meta/plans/*.md | wc -l`
  returns 6

#### Manual Verification

- [ ] Claude Code session without `--plugin-dir` no longer shows the old skills
- [ ] Claude Code session with plugin installed shows all skills with
  `/accelerator:` prefix

---

## Testing Strategy

### Smoke Tests — Flat Extraction (after Phase 5, before Phase 6)

1. Load plugin:
   `claude --plugin-dir ~/Code/organisations/atomic/company/accelerator`
2. Verify skill list shows all 9 user-invocable skills with `accelerator:`
   prefix
3. Verify agent list shows all 7 agents
4. Run `/accelerator:commit` in a test repo with staged changes
5. **Lens resolution test**: Run `/accelerator:review-pr` with
   `focus on architecture` on an existing PR. Verify the reviewer agent output
   includes architecture-lens-specific vocabulary (e.g., "coupling",
   "modularity", "scalability") that proves the lens SKILL.md was loaded via
   `${CLAUDE_PLUGIN_ROOT}`.
6. Run `/accelerator:create-plan` to test planning workflow
7. **Script execution test**: Run `/accelerator:research-codebase` with a
   simple question on a test repository. Verify the generated research document
   contains valid metadata (date, commit hash, branch) proving the
   `research-metadata.sh` script executed successfully via the plugin path.

### Smoke Tests — Restructured (after Phase 7, before Phase 8)

1. Re-run smoke tests 1–3, 5, 7 to confirm restructuring did not break
   anything
2. Verify cross-reference paths resolve correctly after restructuring (the
   lens resolution test in #5 covers this)
3. Confirm no skills were lost during the move operations

### Regression Tests (after Phase 8)

1. Confirm no duplicate skills/agents appear
2. Confirm skills that cross-reference (review-pr, review-plan) correctly
   resolve `${CLAUDE_PLUGIN_ROOT}` paths to find lens skills and output formats
3. Confirm `research-metadata.sh` is found and executable via
   `${CLAUDE_PLUGIN_ROOT}` path
4. Run tests in a **fresh Claude Code session** (not the one used for smoke
   testing) to confirm plugin loading is not dependent on session state

## Performance Considerations

None — this is a file-copy and path-update operation with no runtime performance
implications.

## Migration Notes

- The `meta/` directory is copied (not moved) — it remains in `~/.claude/meta/`
  as well as existing in the plugin. The plugin's `meta/` is a **historical
  snapshot** from extraction and should not receive new documents. New plugin
  development documentation goes in the plugin's `meta/`; project-specific
  research stays in `~/.claude/meta/`. The README should document this
  convention.
- Skill invocation changes from `/commit` to `/accelerator:commit`. Users will
  need to update muscle memory and any documented workflows.
- **Plugin name decision**: The name `accelerator` is kept for clarity and
  discoverability, despite the verbosity. The directory name and plugin manifest
  both use `accelerator`. If this proves too verbose in practice, renaming is a
  single-field change in `plugin.json` and can be done later. The README should
  include concrete invocation examples for the most common workflows
  (`/accelerator:commit`, `/accelerator:review-pr`, `/accelerator:create-plan`,
  `/accelerator:research-codebase`) to help users learn the new prefix.
- A backup archive is created before cleanup (Phase 8, step 0). To restore:
  `tar xzf ~/.claude/pre-extraction-backup.tar.gz -C ~/.claude`

## References

- Research: `~/.claude/meta/research/codebase/2026-03-14-plugin-extraction.md`
- Plugin spec: https://code.claude.com/docs/en/plugins-reference
- Plugin tutorial: https://code.claude.com/docs/en/plugins
