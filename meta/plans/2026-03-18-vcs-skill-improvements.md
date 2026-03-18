# VCS Skill Improvements Implementation Plan

## Overview

Restructure the git skill category into separate VCS and GitHub concerns, add
jujutsu (jj) support via auto-detection hooks, and make VCS-touching skills
backend-agnostic. This enables the accelerator plugin to work transparently
in git, jj colocated, and pure jj repositories.

## Current State Analysis

The `skills/git/` directory bundles two distinct concerns:

| Skill           | Primary CLI                                                  | Actual Concern  |
|-----------------|--------------------------------------------------------------|-----------------|
| `commit`        | `git add`, `git commit`, `git diff`, `git status`, `git log` | VCS operations  |
| `describe-pr`   | `gh` exclusively                                             | GitHub platform |
| `review-pr`     | `gh` exclusively                                             | GitHub platform |
| `respond-to-pr` | `gh` + git refs at lines 67, 326-328, 494                    | GitHub platform |

The `commit` skill has two `!` backtick expressions that hardcode git commands:

- Line 12: `` !`git diff --cached --stat` ``
- Line 13: `` !`git log --oneline -5` ``

The plugin manifest at `.claude-plugin/plugin.json` registers `./skills/git/`
as a single skill directory. No hooks directory or hooks configuration exists.

### Key Discoveries:

- `describe-pr` and `review-pr` have zero git CLI references — pure `gh`
  (`skills/git/describe-pr/SKILL.md`, `skills/git/review-pr/SKILL.md`)
- `respond-to-pr` has three git references:
  - `git branch --show-current` at line 67
  - `git add` specific files / `commit` skill pattern at lines 326-328
  - `git add -A` / `git add .` prohibition at line 494
  (`skills/git/respond-to-pr/SKILL.md`)
- The `commit` skill references git commands in both dynamic context
  expressions (lines 12-13) and process steps (lines 20-21, 39, 41)
  (`skills/git/commit/SKILL.md`)
- Plugin manifest has no hooks section (`.claude-plugin/plugin.json`)
- No `hooks/` directory exists anywhere in the project

## Desired End State

After implementation:

1. **Directory structure** reflects concern boundaries:
  - `skills/vcs/` contains VCS-backend-agnostic operations (commit)
  - `skills/github/` contains GitHub platform operations (describe-pr,
    review-pr, respond-to-pr)
  - `skills/git/` no longer exists

2. **SessionStart hook** auto-detects VCS backend (git, jj colocated, pure jj)
   and injects persistent context telling Claude which commands to use

3. **Commit skill** uses generic VCS language, with no hardcoded git commands
   or `!` backtick expressions

4. **respond-to-pr skill** uses VCS-agnostic language for all VCS operations
   (branch checking, committing), relying on SessionStart context for
   specific commands

5. **PreToolUse guard hook** blocks raw git VCS commands in pure jj repos and
   warns in colocated repos

### Verification:

- All skills invoke correctly via `/accelerator:commit`,
  `/accelerator:describe-pr`, `/accelerator:review-pr`,
  `/accelerator:respond-to-pr`
- In a git repo: SessionStart context says "git", commit skill uses git
  commands, guard hook is dormant
- In a jj colocated repo: SessionStart context says "jj-colocated", commit
  skill uses jj commands, guard hook warns on raw git VCS commands
- In a pure jj repo: SessionStart context says "jj", commit skill uses jj
  commands, guard hook blocks raw git VCS commands

## What We're NOT Doing

- Phase 5 from the research (jj-specific skills like `jj-split`, `jj-evolve`)
  — future work
- Modifying the RTK hook or registry to understand jj commands
- Creating separate `jj-commit` skills — using a single VCS-agnostic skill
- Changing `describe-pr` or `review-pr` skill content (they use `gh` only)

## Implementation Approach

Four sequential phases, each independently deployable:

1. **Restructure directories** — mechanical move, no content changes
2. **VCS detection hook** — new SessionStart hook with `additionalContext`
3. **VCS-agnostic commit skill** — rewrite skill content, remove backtick
   expressions
4. **Guard hook + respond-to-pr update** — safety net for git commands in jj
   repos, plus minor skill update

---

## Phase 1: Restructure Skill Directories

### Overview

Move skills from `skills/git/` into `skills/vcs/` and `skills/github/` to
reflect actual concern boundaries. Update the plugin manifest.

### Changes Required:

#### 1. Create new directory structure

Create:

- `skills/vcs/commit/`
- `skills/github/describe-pr/`
- `skills/github/review-pr/`
- `skills/github/respond-to-pr/`

#### 2. Move skill files

| Source                              | Destination                            |
|-------------------------------------|----------------------------------------|
| `skills/git/commit/SKILL.md`        | `skills/vcs/commit/SKILL.md`           |
| `skills/git/describe-pr/SKILL.md`   | `skills/github/describe-pr/SKILL.md`   |
| `skills/git/review-pr/SKILL.md`     | `skills/github/review-pr/SKILL.md`     |
| `skills/git/respond-to-pr/SKILL.md` | `skills/github/respond-to-pr/SKILL.md` |

#### 3. Update plugin manifest

**File**: `.claude-plugin/plugin.json`

Replace the skills array entry `"./skills/git/"` with `"./skills/vcs/"` and
`"./skills/github/"`:

```json
{
  "skills": [
    "./skills/vcs/",
    "./skills/github/",
    "./skills/planning/",
    "./skills/research/",
    "./skills/review/lenses/",
    "./skills/review/output-formats/"
  ]
}
```

#### 4. Remove old directory

Delete the now-empty `skills/git/` directory.

#### 5. Commit atomically

All file moves, manifest updates, and directory removal **must be committed
in a single atomic commit**. A partially completed restructure (e.g., files
moved but manifest not updated) would break all skill invocations.

#### 6. Verify skill resolution mechanism

Before implementing, verify whether Claude Code resolves skills by the `name`
frontmatter field or by directory path. If by directory path, this is a
breaking change that warrants a major version bump (2.0.0). If by `name`
field only, the directory restructure is non-breaking. Document the finding
in this section.

### Success Criteria:

#### Automated Verification:

- [x] `skills/git/` directory does not exist
- [x] `skills/vcs/commit/SKILL.md` exists and matches original content
- [x] `skills/github/describe-pr/SKILL.md` exists and matches original content
- [x] `skills/github/review-pr/SKILL.md` exists and matches original content
- [x] `skills/github/respond-to-pr/SKILL.md` exists and matches original content
- [x] `.claude-plugin/plugin.json` contains `"./skills/vcs/"` and
  `"./skills/github/"` but not `"./skills/git/"`

#### Manual Verification:

- [ ] `/accelerator:commit` invokes correctly in a Claude Code session
- [ ] `/accelerator:describe-pr` invokes correctly
- [ ] `/accelerator:review-pr` invokes correctly
- [ ] `/accelerator:respond-to-pr` invokes correctly

---

## Phase 2: Implement VCS Detection Hook

### Overview

Create a `SessionStart` hook that detects the VCS backend and injects
persistent context for the entire session. This is the foundation that all
subsequent VCS-agnostic behaviour depends on.

### Changes Required:

#### 1. Create hooks and scripts directories

Create `hooks/` and `scripts/` at the project root.

#### 2. Create shared VCS library

**File**: `scripts/vcs-common.sh`

```bash
#!/usr/bin/env bash

# Shared VCS utility functions sourced by hooks and wrapper scripts.
# This is the single source of truth for repo-root detection logic.

# Find the repository root by walking up the directory tree.
# Outputs the root path and returns 0 if found, returns 1 if not.
find_repo_root() {
  local dir="$PWD"
  while [ "$dir" != "/" ]; do
    if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
      echo "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}
```

#### 3. Create detection script

**File**: `hooks/vcs-detect.sh`

```bash
#!/usr/bin/env bash

# Check for jq dependency
if ! command -v jq &>/dev/null; then
  echo '{"hookSpecificOutput":{"additionalContext":"WARNING: jq is not installed. VCS detection could not run. Install jq for full VCS support. Defaulting to git commands."}}'
  exit 0
fi

# Source shared VCS utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../scripts/vcs-common.sh"

REPO_ROOT=$(find_repo_root)
if [ -z "$REPO_ROOT" ]; then
  # Not in a VCS repository at all — default to git mode
  VCS_MODE="git"
else
  # Determine VCS backend by checking for .jj and .git directories at repo root
  if [ -d "$REPO_ROOT/.jj" ]; then
    if [ -d "$REPO_ROOT/.git" ]; then
      VCS_MODE="jj-colocated"
    else
      VCS_MODE="jj"
    fi
  else
    VCS_MODE="git"
  fi
fi

# Build context based on VCS mode
case "$VCS_MODE" in
  jj|jj-colocated)
    CONTEXT="This repository uses jujutsu (jj) as its version control system (mode: ${VCS_MODE}).

VCS Command Reference:
- Use \`jj status\` instead of \`git status\`
- Use \`jj diff\` instead of \`git diff\`
- Use \`jj log\` instead of \`git log\`
- Use \`jj commit -m \"message\"\` instead of \`git add + git commit\` (there is no staging area; all tracked changes are included)
- Use \`jj squash\` instead of \`git commit --amend\`
- Use \`jj describe -m \"message\"\` to edit a commit message
- Use \`jj git push\` instead of \`git push\`
- Use \`jj bookmark list\` instead of \`git branch\`
- Use \`jj new\` to start a new change after the current one
- Use \`jj bookmark list\` or \`jj status\` instead of \`git branch --show-current\`

Key conceptual differences from git:
- No staging area: all tracked changes are automatically part of the working-copy commit
- The working copy is always a commit — there is no \"uncommitted\" state
- \`jj new\` creates a new empty change (like finishing current work and starting fresh)
- \`jj describe\` edits any commit's message without \`--amend\` semantics

IMPORTANT: Do NOT use raw git commands for VCS operations. Always use jj.
The \`gh\` CLI for GitHub operations remains unchanged."
    ;;
  git)
    CONTEXT="This repository uses git as its version control system.

VCS Command Reference:
- Use \`git status\` to see current changes
- Use \`git diff\` to see modifications (use \`--cached\` for staged changes)
- Use \`git log --oneline\` to see recent commit history
- Use \`git add <files>\` to stage changes — NEVER use \`-A\` or \`.\` (always add specific files by name)
- Use \`git commit -m \"message\"\` to commit staged changes
- Use \`git branch --show-current\` to check the current branch
- Use \`git push\` to push to remote

Key conventions:
- Always stage specific files by name, never bulk-add
- Use \`--cached\` with \`git diff\` to see what is staged
- Prefer atomic, focused commits over large multi-concern commits"
    ;;
esac

# Output as SessionStart hook response
# Use jq to safely encode the context string as JSON
jq -n --arg context "$CONTEXT" '{
  "hookSpecificOutput": {
    "additionalContext": $context
  }
}'
```

#### 3. Create hooks registration

**File**: `hooks/hooks.json`

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh"
          }
        ]
      }
    ]
  }
}
```

Note: The `${CLAUDE_PLUGIN_ROOT}` variable resolves to the plugin's installed
path at runtime. The empty `matcher` string matches all session starts.

**Prerequisites**: `jq` must be installed on the system. The detection script
checks for its availability and provides a degraded-but-functional fallback
if absent.

**Hooks discovery**: Verify before implementation how Claude Code discovers
plugin hooks. If hooks require a `hooks` field in `.claude-plugin/plugin.json`,
add that to the Phase 1 manifest update. If hooks are auto-discovered from a
conventional `hooks/` directory, document this assumption. The `hooks.json`
file within the hooks directory is the registration format for hook
definitions.

### Success Criteria:

#### Automated Verification:

- [x] `hooks/vcs-detect.sh` exists and is executable
- [x] `hooks/hooks.json` exists and is valid JSON
- [x] Running `hooks/vcs-detect.sh` in a git repo outputs valid JSON with
  `hookSpecificOutput.additionalContext` containing "git"
- [x] Running `hooks/vcs-detect.sh` in a directory with `.jj/` and `.git/`
  outputs JSON mentioning "jj-colocated"
- [x] Running `hooks/vcs-detect.sh` in a directory with only `.jj/` outputs
  JSON mentioning "jj" mode

#### Manual Verification:

- [ ] Starting a new Claude Code session in a git repo shows git VCS context
- [ ] Starting a new Claude Code session in a jj repo shows jj VCS context
- [ ] The injected context persists throughout the session (visible in
  subsequent interactions)

---

## Phase 3: Make Commit Skill VCS-Agnostic

### Overview

Rewrite the commit skill to use generic VCS language instead of hardcoded git
commands. The SessionStart context (Phase 2) tells Claude which specific
commands to use. Replace the git-specific `!` backtick expressions with
VCS-aware wrapper scripts that detect the backend and run the appropriate
command.

### Changes Required:

#### 1. Create VCS-aware wrapper scripts

These scripts detect the VCS backend and run the appropriate command,
preserving the pre-populated context that backtick expressions provide.

**File**: `scripts/vcs-status.sh`

```bash
#!/usr/bin/env bash

# VCS-aware status script for backtick expressions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/vcs-common.sh"

REPO_ROOT=$(find_repo_root)
if [ -n "$REPO_ROOT" ] && [ -d "$REPO_ROOT/.jj" ]; then
  jj status 2>/dev/null || echo "(jj status unavailable)"
else
  git diff --cached --stat 2>/dev/null || echo "(git status unavailable)"
fi
```

**File**: `scripts/vcs-log.sh`

```bash
#!/usr/bin/env bash

# VCS-aware log script for backtick expressions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/vcs-common.sh"

REPO_ROOT=$(find_repo_root)
if [ -n "$REPO_ROOT" ] && [ -d "$REPO_ROOT/.jj" ]; then
  jj log --limit 5 2>/dev/null || echo "(jj log unavailable)"
else
  git log --oneline -5 2>/dev/null || echo "(git log unavailable)"
fi
```

#### 2. Rewrite commit skill

**File**: `skills/vcs/commit/SKILL.md`

Replace the entire skill content with a VCS-agnostic version:

```markdown
---
name: commit
description: Create VCS commits for session changes. Use when the user wants to
  commit their work with well-structured, atomic commits.
argument-hint: "[optional message or flags]"
disable-model-invocation: true
---

# Commit Changes

!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-status.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-log.sh`

## Process:

1. **Think about what changed:**

- Review the conversation history and understand what was accomplished
- Review the VCS status and diff above to see what files changed
- Consider whether changes should be one commit or multiple logical commits

2. **Plan your commit(s):**

- Identify which files belong together
- Draft clear, descriptive commit messages
- Use imperative mood in commit messages
- Focus on why the changes were made, not just what

3. **Present your plan to the user:**

- List the files you plan to include in each commit
- Show the commit message(s) you'll use
- Ask: "I plan to create [N] commit(s) with these changes. Shall I proceed?"

4. **Execute upon confirmation:**

- Stage and commit the planned files with your planned messages
- Use the VCS commands appropriate for this repository (refer to the
  session's VCS context for the correct commands)
- Show recent commit history to confirm the result

## Important:

- **NEVER add co-author information or Claude attribution**
- Commits should be authored solely by the user
- Do not include any "Generated with Claude" messages
- Do not add "Co-Authored-By" lines
- Write commit messages as if the user wrote them
- Use the VCS commands appropriate for this repository's version control
  system (refer to the session's VCS context for the correct commands)
- When staging files, always add specific files by name — never bulk-add
  all changes at once

## Remember:

- You have the full context of what was done in this session
- Group related changes together
- Keep commits focused and atomic when possible
- The user trusts your judgment - they asked you to commit
```

### Prerequisite Verification:

Before implementing, verify that `${CLAUDE_PLUGIN_ROOT}` is expanded within
`!` backtick expressions in SKILL.md files. This variable is documented for
hook command fields but its availability in backtick expression context is
unverified. If it is not expanded, the wrapper scripts will not be found and
the commit skill's pre-populated context will break — a regression from the
current working state. If unavailable, determine an alternative path mechanism
(e.g., relative paths from the skill file, or a PATH-based discovery).

### Key Changes from Original:

1. **Replaced git-specific `!` backtick expressions** with VCS-aware wrapper
   scripts (`vcs-status.sh`, `vcs-log.sh`) that detect the backend and run
   the appropriate command — preserving pre-populated context at invocation
2. **Replaced git-specific commands** with generic VCS language that defers
   entirely to session VCS context — no inline git or jj command references
3. **Updated description** from "Create git commits" to "Create VCS commits"
4. **Added instruction** to refer to session VCS context for correct commands

### Success Criteria:

#### Automated Verification:

- [x] `scripts/vcs-common.sh` exists and is executable
- [x] `scripts/vcs-status.sh` exists and is executable
- [x] `scripts/vcs-log.sh` exists and is executable
- [x] Running `scripts/vcs-status.sh` in a git repo runs `git diff --cached --stat`
- [x] Running `scripts/vcs-status.sh` in a jj repo runs `jj status`
- [x] Running `scripts/vcs-log.sh` in a git repo runs `git log --oneline -5`
- [x] Running `scripts/vcs-log.sh` in a jj repo runs `jj log --limit 5`
- [x] `skills/vcs/commit/SKILL.md` uses `!` backtick expressions referencing
  the VCS-aware wrapper scripts (not hardcoded git commands)
- [x] `skills/vcs/commit/SKILL.md` does not contain hardcoded `git status`,
  `git diff`, `git add`, or `git log` as instructions in the process steps
- [x] YAML frontmatter is valid with `name: commit`

#### Manual Verification:

- [ ] In a git repo: `/accelerator:commit` uses git commands correctly
- [ ] In a jj repo: `/accelerator:commit` uses jj commands correctly
  (no staging, uses `jj commit`)
- [ ] Commit messages follow the same quality as before (imperative mood,
  focused, no attribution)

---

## Phase 4: VCS Guard Hook and respond-to-pr Update

### Overview

Add a `PreToolUse` guard hook that blocks raw git VCS commands in pure jj repos
and warns in colocated repos. Also update `respond-to-pr` to use generic
branch-checking language.

### Changes Required:

#### 1. Create guard script

**File**: `hooks/vcs-guard.sh`

```bash
#!/usr/bin/env bash

# VCS Guard: PreToolUse hook for Bash tool calls
# Blocks git VCS commands in pure jj repos, warns in colocated repos
# Allows git-specific commands (e.g., git push) and all gh commands
#
# Requirements: bash 4+, jq, GNU-compatible grep

# Check for jq dependency
if ! command -v jq &>/dev/null; then
  # Can't parse input without jq — allow through silently
  exit 0
fi

# Source shared VCS utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../scripts/vcs-common.sh"

REPO_ROOT=$(find_repo_root)

# Only act if we're in a jj repo
if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT/.jj" ]; then
  exit 0
fi

# Read the command from stdin (PreToolUse hook receives tool input as JSON)
INPUT=$(timeout 5 cat 2>/dev/null || cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if [ -z "$COMMAND" ]; then
  exit 0
fi

# Allow all gh commands unconditionally
if echo "$COMMAND" | grep -qE '^\s*gh\s'; then
  exit 0
fi

# Allow rtk-wrapped commands through (rtk handles its own rewriting)
if echo "$COMMAND" | grep -qE '^\s*rtk\s'; then
  exit 0
fi

# Split compound commands and check each subcommand independently
# Splits on &&, ||, ;, and | (pipe)
check_git_vcs_command() {
  local cmd="$1"

  # List of git VCS commands that have jj equivalents.
  # Commands NOT in this list (e.g., git push, git pull, git fetch, git remote,
  # git clone, git config, git tag) are implicitly allowed through since they
  # have no jj equivalent or jj delegates to them.
  local vcs_pattern='^\s*git\s+(status|diff|add|commit|log|branch|checkout|switch|merge|rebase|reset|stash|show)(\s|$)'

  if echo "$cmd" | grep -qE "$vcs_pattern"; then
    return 0  # Blocked/warned
  fi
  return 1  # Not a git VCS command (including allowed git commands like push)
}

# Extract first matching git VCS subcommand from compound command
FOUND_SUBCMD=""
while IFS= read -r subcmd; do
  # Trim leading/trailing whitespace
  subcmd=$(echo "$subcmd" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  if check_git_vcs_command "$subcmd"; then
    FOUND_SUBCMD=$(echo "$subcmd" | grep -oE 'git\s+(status|diff|add|commit|log|branch|checkout|switch|merge|rebase|reset|stash|show)' | head -1 | awk '{print $2}')
    break
  fi
done <<< "$(echo "$COMMAND" | sed 's/&&/\n/g; s/||/\n/g; s/;/\n/g; s/|/\n/g')"

if [ -z "$FOUND_SUBCMD" ]; then
  exit 0
fi

# Determine mode
if [ -d "$REPO_ROOT/.git" ]; then
  MODE="colocated"
else
  MODE="pure-jj"
fi

# Build jj equivalent suggestion
case "$FOUND_SUBCMD" in
  status)  JJ_ALT="jj status" ;;
  diff)    JJ_ALT="jj diff" ;;
  add)     JJ_ALT="(not needed — jj has no staging area; use jj commit directly)" ;;
  commit)  JJ_ALT="jj commit -m \"message\"" ;;
  log)     JJ_ALT="jj log" ;;
  branch)  JJ_ALT="jj bookmark list" ;;
  show)    JJ_ALT="jj show" ;;
  *)       JJ_ALT="check jj documentation for equivalent" ;;
esac

if [ "$MODE" = "pure-jj" ]; then
  # Block in pure jj repos
  jq -n --arg subcmd "$FOUND_SUBCMD" --arg alt "$JJ_ALT" '{
    "decision": "block",
    "reason": ("This is a pure jujutsu repository. Use jj instead of git " + $subcmd + ". Equivalent: " + $alt)
  }'
else
  # Warn in colocated repos
  jq -n --arg subcmd "$FOUND_SUBCMD" --arg alt "$JJ_ALT" '{
    "decision": "allow",
    "hookSpecificOutput": {
      "systemMessage": ("This is a jj-colocated repository. Prefer jj over git " + $subcmd + ". Suggested equivalent: " + $alt)
    }
  }'
fi
```

**Known limitations**: The command splitting on `&&`, `||`, `;`, `|` is
heuristic-based and may not handle all edge cases (e.g., these characters
inside quoted strings). This is acceptable for the guard hook's purpose as a
best-effort safety net — the SessionStart context is the primary mechanism
for guiding correct VCS usage.

#### 2. Update hooks registration

**File**: `hooks/hooks.json`

Add the `PreToolUse` section alongside the existing `SessionStart` section:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-guard.sh"
          }
        ]
      }
    ]
  }
}
```

#### 3. Update respond-to-pr skill

**File**: `skills/github/respond-to-pr/SKILL.md` (after Phase 1 move)

Update all three git-specific references to use VCS-agnostic language:

**Change A** — Branch check (lines 64-70):

**Current**:
```markdown
4. **Ensure on the correct branch**: Check if the user is on the PR's head
   branch. If not, inform them and ask if they want to switch:
   ```bash
   git branch --show-current
   ```

Compare with the PR's `headRefName`. If different, ask the user whether to
switch to it.
```

**Replace with**:
```markdown
4. **Ensure on the correct branch**: Check the current branch or bookmark
   using the appropriate VCS command for this repository (refer to the
   session's VCS context). Compare with the PR's `headRefName`. If different,
   inform the user and ask if they want to switch.
```

**Change B** — Commit instructions (lines 326-328):

**Current**:
```markdown
When committing, follow the `commit` skill pattern: `git add` specific
files, create the commit with the suggested message (or user's amended
message). Never use `git add -A` or `git add .`.
```

**Replace with**:
```markdown
When committing, follow the `commit` skill pattern using the appropriate
VCS commands for this repository (refer to the session's VCS context).
Keep commits focused and atomic.
```

**Change C** — git add prohibition (line 494):

**Current**:
```markdown
- Don't use `git add -A` or `git add .` — add specific files only
```

**Replace with**:
```markdown
- When staging files, always add specific files by name — never bulk-add
```

**Change D** — Commit skill pattern reference (line 461):

**Current**:
```markdown
   Follow the `commit` skill pattern (specific file adds, no `-A`).
```

**Replace with**:
```markdown
   Follow the `commit` skill pattern for this repository's VCS (refer to
   session VCS context). Keep commits focused and atomic.
```

### Success Criteria:

#### Automated Verification:

- [x] `hooks/vcs-guard.sh` exists and is executable
- [x] `hooks/hooks.json` contains both `SessionStart` and `PreToolUse` entries
- [x] `hooks/hooks.json` is valid JSON
- [x] Running `hooks/vcs-guard.sh` with a `git status` command in a directory
  with only `.jj/` outputs JSON with `"decision": "block"`
- [x] Running `hooks/vcs-guard.sh` with a `git status` command in a directory
  with both `.jj/` and `.git/` outputs JSON with `"decision": "allow"` and a
  `systemMessage` warning
- [x] Running `hooks/vcs-guard.sh` with a `gh pr view` command outputs nothing
  (exit 0)
- [x] Running `hooks/vcs-guard.sh` with a `git push` command in a jj repo
  outputs nothing (exit 0 — push is allowed)
- [x] `skills/github/respond-to-pr/SKILL.md` does not contain
  `git branch --show-current`
- [x] `skills/github/respond-to-pr/SKILL.md` does not contain
  `git add` as an instruction (may reference git in explanatory context)
- [x] `skills/github/respond-to-pr/SKILL.md` does not contain
  `git add -A` or `git add .`

#### Manual Verification:

- [ ] In a pure jj repo: running `git status` via Claude is blocked with a
  helpful message
- [ ] In a colocated repo: running `git status` via Claude shows a warning
  but proceeds
- [ ] In a git repo: git commands work without interference
- [ ] `gh` commands work in all repo types without interference
- [ ] `/accelerator:respond-to-pr` correctly identifies the current branch
  in both git and jj repos

---

## Testing Strategy

### Test Framework and Infrastructure

Use [bats-core](https://github.com/bats-core/bats-core) (Bash Automated
Testing System) for automated testing of shell scripts. Test files live in
`tests/hooks/` and `tests/scripts/`.

**Directory structure**:
```
tests/
  hooks/
    vcs-detect.bats
    vcs-guard.bats
  scripts/
    vcs-common.bats
    vcs-status.bats
    vcs-log.bats
  test_helper.bash    # Shared setup/teardown for temp directories
```

**Deliverables**: Test file creation is part of Phase 2 (for detection hook
tests and wrapper script tests) and Phase 4 (for guard hook tests). Tests
should be runnable via `bats tests/`.

### Unit Tests — vcs-detect.sh

Test in three directory configurations using temporary directories:

- [ ] Git repo (`.git/` only) → outputs JSON with `"git"` in context
- [ ] jj colocated (`.jj/` + `.git/`) → outputs JSON with `"jj-colocated"`
- [ ] Pure jj (`.jj/` only) → outputs JSON with `"jj"` in context
- [ ] No VCS directory → defaults to git mode
- [ ] Subdirectory of git repo → still detects git (walks up to find root)
- [ ] Subdirectory of jj repo → still detects jj (walks up to find root)
- [ ] JSON structure: verify `jq -e '.hookSpecificOutput.additionalContext'`
  succeeds and returns non-empty string
- [ ] Missing jq: outputs fallback JSON with warning message

### Unit Tests — vcs-guard.sh

Test with various command inputs and directory configurations:

**Core behavior**:
- [ ] `git status` in pure jj → `"decision": "block"`
- [ ] `git status` in colocated → `"decision": "allow"` with `systemMessage`
- [ ] `git status` in git repo → exit 0, no output
- [ ] `git push` in pure jj → exit 0 (allowed)
- [ ] `gh pr view` → exit 0 (allowed unconditionally)
- [ ] `rtk git status` → exit 0 (rtk-wrapped, allowed)

**Compound command edge cases**:
- [ ] `git status && git push` in pure jj → blocks (detects `git status`)
- [ ] `git push && git commit -m "msg"` in pure jj → blocks (detects
  `git commit`)
- [ ] `echo "git status" | grep test` → exit 0 (git not in command position)
- [ ] `git push` alone in pure jj → exit 0 (allowed)

**Error cases**:
- [ ] Empty command → exit 0
- [ ] Malformed JSON on stdin → exit 0 (graceful failure)
- [ ] Missing jq → exit 0 (allow through)

### Unit Tests — wrapper scripts

- [ ] `vcs-status.sh` in git repo → runs `git diff --cached --stat`
- [ ] `vcs-status.sh` in jj repo → runs `jj status`
- [ ] `vcs-log.sh` in git repo → runs `git log --oneline -5`
- [ ] `vcs-log.sh` in jj repo → runs `jj log --limit 5`
- [ ] Both scripts in subdirectory → detect repo root correctly

### Structural Validation

- [ ] Parse `plugin.json` and verify each skills path entry corresponds to a
  directory containing at least one `SKILL.md` file
- [ ] Verify `hooks.json` is valid JSON with expected structure

### Manual Testing Steps

1. Install the plugin in a git repo, start session, run `/accelerator:commit`
   — should use git commands, status/log pre-populated
2. Install the plugin in a jj colocated repo, start session, run
   `/accelerator:commit` — should use jj commands, status/log pre-populated
3. In a jj colocated repo, attempt a raw `git status` — should see warning
4. In a pure jj repo, attempt a raw `git status` — should be blocked
5. In any repo type, run `/accelerator:describe-pr` — should work unchanged
6. In any repo type, run `/accelerator:respond-to-pr` — should detect branch
   correctly using appropriate VCS command
7. Start a session from a subdirectory — VCS detection should still work

## Performance Considerations

- `SessionStart` hook runs once per session — negligible overhead
- `PreToolUse` guard hook runs on every Bash tool call — the script is
  lightweight (directory existence checks and regex matching), so impact is
  minimal
- The guard hook exits early (exit 0) for non-jj repos, so there is zero
  overhead in plain git repos

## Dependencies

- **jq**: Required for JSON construction/parsing in hook scripts. Both
  `vcs-detect.sh` and `vcs-guard.sh` check for availability and degrade
  gracefully if absent.
- **bash 4+**: Hook scripts use `#!/usr/bin/env bash` and bash-compatible
  constructs.
- **bats-core**: Required for running automated tests (`tests/` directory).

## References

- Research:
  `meta/research/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`
- Plugin manifest: `.claude-plugin/plugin.json`
- Commit skill: `skills/git/commit/SKILL.md` (pre-move)
- respond-to-pr skill: `skills/git/respond-to-pr/SKILL.md` (pre-move) —
  git references at lines 67, 326-328, 494
- Claude Code hooks documentation: SessionStart with `additionalContext`,
  PreToolUse with `decision` and `systemMessage`
- Community reference: `kawaz/claude-plugin-jj` (PreToolUse guard pattern)
