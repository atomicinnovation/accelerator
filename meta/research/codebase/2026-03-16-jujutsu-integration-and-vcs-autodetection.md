---
date: "2026-03-16T12:24:27+00:00"
researcher: Toby Clemson
git_commit: 4a96a19d86bd224b45d7e97f39bbd2079141eb80
branch: main
repository: accelerator
topic: "Jujutsu integration, VCS auto-detection, and git/github skill restructuring"
tags: [ research, jujutsu, git, github, hooks, vcs, auto-detection, skills ]
status: complete
last_updated: "2026-03-16"
last_updated_by: Toby Clemson
---

# Research: Jujutsu Integration, VCS Auto-Detection, and Git/GitHub Skill Restructuring

**Date**: 2026-03-16T12:24:27+00:00
**Researcher**: Toby Clemson
**Git Commit**: 4a96a19d86bd224b45d7e97f39bbd2079141eb80
**Branch**: main
**Repository**: accelerator

## Research Question

The git skill category currently bundles two concerns: git workflow skills
(commit) and GitHub platform skills (describe-pr, respond-to-pr, review-pr).
These should be split along those lines. Additionally, how can jujutsu (jj)
skills be introduced such that all skills — including existing ones —
auto-detect
whether to use jj or git for their CLI interactions? Could a hook-based approach
set context that affects all subsequent interactions?

## Summary

The research identifies a clean architectural approach with three layers:

1. **Skill restructuring**: Split `skills/git/` into `skills/vcs/` (for
   commit-like VCS operations) and `skills/github/` (for PR/review platform
   operations). This reflects the actual concern boundaries.

2. **VCS auto-detection via SessionStart hook**: A `SessionStart` hook can
   detect the VCS backend (jj colocated, jj non-colocated, or plain git) by
   checking for a `.jj/` directory, then inject `additionalContext` that
   persists for the entire session. This context tells all skills which VCS
   commands to use without modifying any skill individually.

3. **Jujutsu-specific skills**: New VCS skills (e.g., `jj-commit`, `jj-log`)
   can coexist alongside git equivalents, or the existing `commit` skill can
   become VCS-agnostic by reading the injected context.

The hook-based approach is strongly preferred because it requires zero changes
to existing skills that don't directly use git commands (the GitHub skills use
`gh` which works regardless of VCS backend), and it provides a single
configuration point for the detection heuristic.

## Detailed Findings

### Current Skill Structure

The `skills/git/` directory contains four skills with two distinct concerns:

| Skill           | Primary Tool                                                 | Actual Concern                       |
|-----------------|--------------------------------------------------------------|--------------------------------------|
| `commit`        | `git add`, `git commit`, `git diff`, `git status`, `git log` | **VCS operations**                   |
| `describe-pr`   | `gh pr view`, `gh pr edit`, `gh pr diff`, `gh pr list`       | **GitHub platform**                  |
| `review-pr`     | `gh pr view`, `gh pr diff`, `gh api`                         | **GitHub platform**                  |
| `respond-to-pr` | `gh api`, `gh pr view`, `gh pr comment`, `git branch`        | **GitHub platform** (with minor VCS) |

The `commit` skill is the only one that primarily uses git CLI commands. The
three PR skills use `gh` almost exclusively — the only git command in PR skills
is `git branch --show-current` in `respond-to-pr` (Step 1.4).

### Proposed Skill Directory Structure

```
skills/
  vcs/                          # VCS-backend-agnostic operations
    commit/SKILL.md             # Currently skills/git/commit
    # Future: status, log, diff, etc.
  github/                       # GitHub platform operations
    describe-pr/SKILL.md        # Currently skills/git/describe-pr
    review-pr/SKILL.md          # Currently skills/git/review-pr
    respond-to-pr/SKILL.md      # Currently skills/git/respond-to-pr
  planning/                     # (unchanged)
  research/                     # (unchanged)
  review/                       # (unchanged)
```

Plugin registration in `plugin.json` would change from:

```json
"skills": ["./skills/git/", ...]
```

to:

```json
"skills": ["./skills/vcs/", "./skills/github/", ...]
```

### VCS Auto-Detection Heuristics

The canonical detection method is checking for the `.jj/` directory:

```bash
if [ -d .jj ]; then
  if [ -d .git ]; then
    VCS_BACKEND="jj-colocated"    # Both .jj and .git exist
  else
    VCS_BACKEND="jj"              # Pure jj, git data inside .jj/
  fi
else
  VCS_BACKEND="git"               # Plain git repo
fi
```

This is the same heuristic used by all existing jj integrations (kawaz plugin,
MCP servers, community hooks). No environment variables or config files are
needed — the directory structure is definitive.

### Git-to-Jujutsu Command Mapping (Key Operations)

| Operation       | Git                                      | Jujutsu                | Notes                                               |
|-----------------|------------------------------------------|------------------------|-----------------------------------------------------|
| Status          | `git status`                             | `jj status` / `jj st`  | Working copy is always a commit in jj               |
| Stage + commit  | `git add <files> && git commit -m "msg"` | `jj commit -m "msg"`   | No staging area in jj; all tracked changes included |
| Amend           | `git commit --amend`                     | `jj squash`            | Moves working-copy diff into parent                 |
| Diff (working)  | `git diff`                               | `jj diff`              | Shows current working-copy change                   |
| Diff (staged)   | `git diff --cached`                      | *(N/A)*                | No staging area                                     |
| Log             | `git log --oneline -5`                   | `jj log -n 5`          | Different default output format                     |
| Current branch  | `git branch --show-current`              | `jj bookmark list`     | jj uses "bookmarks" not "branches"                  |
| Push            | `git push`                               | `jj git push`          | Must specify bookmark or `--all`                    |
| Describe commit | *(edit during commit)*                   | `jj describe -m "msg"` | Can edit message of any change                      |
| Undo            | `git reflog` + `git reset`               | `jj undo`              | First-class undo in jj                              |

**Key conceptual differences:**

- No staging area — `git add` has no equivalent; all changes are automatically
  part of the working-copy commit
- The working copy is always a commit — there's no "uncommitted" state
- `jj new` creates a new empty change (like finishing current work and starting
  fresh)
- `jj describe` edits any commit's message without `--amend` semantics

### Hook-Based Auto-Detection Architecture

#### Mechanism: SessionStart Hook with additionalContext

Claude Code's `SessionStart` hook supports returning `additionalContext` in
its stdout, which persists as context for the entire session. This is the
ideal injection point for VCS detection.

**How it works:**

1. A `SessionStart` hook runs a detection script
2. The script checks for `.jj/` and `.git/` directories
3. It outputs JSON with `hookSpecificOutput.additionalContext` containing
   VCS-specific instructions
4. Claude receives this context at session start and uses it for all
   subsequent interactions

**Example hook configuration** (in `.claude/settings.json` or project-level):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/vcs-detect.sh"
          }
        ]
      }
    ]
  }
}
```

**Example detection script** (`hooks/vcs-detect.sh`):

```bash
#!/usr/bin/env bash

# Determine VCS backend
if [ -d .jj ]; then
  if [ -d .git ]; then
    VCS_MODE="jj-colocated"
  else
    VCS_MODE="jj"
  fi
else
  VCS_MODE="git"
fi

# Build context based on VCS mode
case "$VCS_MODE" in
  jj|jj-colocated)
    CONTEXT="This repository uses jujutsu (jj) as its version control system.

VCS Command Reference:
- Use \`jj status\` instead of \`git status\`
- Use \`jj diff\` instead of \`git diff\`
- Use \`jj log\` instead of \`git log\`
- Use \`jj commit -m \"message\"\` instead of \`git add + git commit\`
  (there is no staging area; all tracked changes are included)
- Use \`jj squash\` instead of \`git commit --amend\`
- Use \`jj describe -m \"message\"\` to edit a commit message
- Use \`jj git push\` instead of \`git push\`
- Use \`jj bookmark list\` instead of \`git branch\`
- Use \`jj new\` to start a new change after the current one

IMPORTANT: Do NOT use raw git commands for VCS operations. Always use jj.
The \`gh\` CLI for GitHub operations remains unchanged."
    ;;
  git)
    CONTEXT="This repository uses git as its version control system."
    ;;
esac

# Output as SessionStart hook response
cat <<EOF
{
  "hookSpecificOutput": {
    "additionalContext": "$CONTEXT"
  }
}
EOF
```

#### Why SessionStart + additionalContext is the Right Approach

| Approach                            | Pros                                                                                     | Cons                                                                                                                 |
|-------------------------------------|------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| **SessionStart hook** (recommended) | Runs once, persists all session; zero skill modifications needed; single source of truth | Must restart session if repo VCS changes                                                                             |
| **PreToolUse hook on Bash**         | Can rewrite commands dynamically (like RTK)                                              | Brittle — must parse arbitrary commands; high maintenance; doesn't help with skills that compose commands in prompts |
| **Skill-level conditionals**        | Explicit per-skill control                                                               | Every skill must be modified; duplication; easy to miss                                                              |
| **UserPromptSubmit hook**           | Runs before each prompt                                                                  | Redundant — VCS doesn't change mid-session                                                                           |
| **CLAUDE.md instructions**          | Simple text                                                                              | Must be manually maintained per-project; no auto-detection                                                           |

The `SessionStart` approach is superior because:

1. **Skills don't need modification** — the context injection tells Claude
   which commands to use, and Claude adapts its tool calls accordingly
2. **Works for all skills** — not just the ones we author, but any skill or
   ad-hoc interaction
3. **Single detection point** — the heuristic lives in one script
4. **Matches the RTK pattern** — users already have a PreToolUse hook for
   RTK command rewriting; this complements it at a different lifecycle point

#### Complementary PreToolUse Hook (Optional Safety Net)

For extra safety, a `PreToolUse` hook on `Bash` could intercept and warn/block
raw git commands in jj repos:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/vcs-guard.sh"
          }
        ]
      }
    ]
  }
}
```

This mirrors the approach of the `kawaz/claude-plugin-jj` community plugin,
which uses a `PreToolUse` hook to intercept `git` commands in `.jj` repos and
either block them or suggest the jj equivalent. The guard script could:

- Detect `git commit`, `git add`, `git status`, etc. in the command
- If in a jj repo, output a `systemMessage` suggesting the jj equivalent
- Allow `git push` and other git-specific commands that jj delegates to git
- Allow all `gh` commands unconditionally

### Impact on Existing Skills

#### commit skill (HIGH impact — needs VCS-aware rewrite)

The `commit` skill at `skills/git/commit/SKILL.md` directly embeds git
commands:

- Dynamic context: `!`git diff --cached --stat`` and `!`git log --oneline -5``
  (lines 12-13)
- Process steps reference `git status`, `git diff`, `git add`, `git log`
  (lines 21-41)

**Options:**

1. **Single VCS-agnostic skill**: Rewrite the commit skill to reference
   "VCS commands" generically and rely on the SessionStart context to tell
   Claude which specific commands to use. The `!` backtick expressions would
   need to be conditional or replaced with a script that outputs the right
   command.

2. **Parallel skills**: Create `skills/vcs/jj-commit/SKILL.md` alongside
   the existing commit skill, and let the SessionStart context guide which
   one to invoke. This has the downside of duplication.

3. **Conditional skill content**: Use a script in the `!` backtick
   expressions that detects VCS and runs the appropriate command. This is
   the most surgical change.

**Recommendation**: Option 1 (VCS-agnostic skill) is cleanest long-term.
The `!` backtick expressions could invoke a small wrapper script that
detects VCS and runs the appropriate status/log command. The process steps
can use generic language ("check status", "review diff") and rely on the
SessionStart context for specific commands.

#### describe-pr skill (LOW impact)

Uses `gh` exclusively. The only change needed is moving it from
`skills/git/` to `skills/github/`. No command modifications required.

#### review-pr skill (LOW impact)

Uses `gh` exclusively. Same as describe-pr — just a directory move.

#### respond-to-pr skill (LOW impact)

Uses `gh` almost exclusively. The one `git branch --show-current` call
(line 68) would need a jj equivalent (`jj bookmark list` or check jj
status), but this is a single line and the SessionStart context should
handle it.

### Existing Community Integrations

Several projects already solve parts of this problem:

1. **kawaz/claude-plugin-jj** — A Claude Code plugin with three layers:
  - `PreToolUse` hook that intercepts git commands in jj repos
  - Skill providing git-to-jj mapping reference
  - Agent with comprehensive jj knowledge
  - GitHub: https://github.com/kawaz/claude-plugin-jj

2. **Matthew Sanabria's Stop hook** — Auto-snapshots via `jj show --summary`
   after every Claude stop event
  -
  Blog: https://matthewsanabria.dev/posts/running-jujutsu-with-claude-code-hooks/

3. **Anthony Panozzo's safety hooks** — SessionStart + PreCompact hooks that
   run `jj status` for state snapshots before context loss
  -
  Blog: https://www.panozzaj.com/blog/2025/11/22/avoid-losing-work-with-jujutsu-jj-for-ai-coding-agents/

4. **@olupdev/jujutsu-mcp-server** — MCP server providing `jj_status`,
   `jj_log`, `jj_show` tools

### Proposed Implementation Plan

#### Phase 1: Restructure skill directories

1. Create `skills/vcs/` and `skills/github/`
2. Move `commit` to `skills/vcs/commit/`
3. Move `describe-pr`, `review-pr`, `respond-to-pr` to `skills/github/`
4. Update `plugin.json` skills array
5. Remove empty `skills/git/` directory

#### Phase 2: Implement VCS detection hook

1. Create `hooks/vcs-detect.sh` — the SessionStart detection script
2. Register hook in plugin's `hooks/hooks.json`
3. Test with plain git repo, jj colocated repo, and jj non-colocated repo

#### Phase 3: Make commit skill VCS-agnostic

1. Create helper script `skills/vcs/commit/vcs-status.sh` for `!` backtick
   expressions
2. Rewrite commit skill to use generic VCS language
3. Add jj-specific process notes (no staging area, `jj commit` vs
   `git add + git commit`)

#### Phase 4: Optional safety guard hook

1. Create `hooks/vcs-guard.sh` — PreToolUse hook that warns on raw git
   commands in jj repos
2. Register alongside the existing RTK hook (both match on `Bash`)

#### Phase 5: Jujutsu-specific skills (future)

1. Consider jj-specific skills that have no git equivalent:
  - `jj-split` — interactive change splitting
  - `jj-evolve` — resolve divergent changes
  - `jj-op-log` — operation history and undo
2. These would live under `skills/vcs/` with `jj-` prefix

### RTK Hook Interaction

The existing RTK `PreToolUse` hook at `~/.claude/hooks/rtk-rewrite.sh`
rewrites bash commands to use `rtk` for token savings. This hook:

- Matches on `Bash` tool calls
- Delegates to `rtk rewrite` for command transformation
- Returns `updatedInput` with the rewritten command

The VCS guard hook would also match on `Bash` and run in the same
`PreToolUse` array. Claude Code runs all hooks in the array sequentially, so
both can coexist. The VCS guard should run **before** RTK (array ordering) so
that it intercepts the original command before RTK rewrites it. Alternatively,
the RTK rewrite registry could be extended to understand jj commands.

## Code References

- `skills/git/commit/SKILL.md` — Git-specific commit skill with `!` backtick
  expressions (lines 12-13) and git CLI references (lines 21-41)
- `skills/git/describe-pr/SKILL.md` — GitHub-only skill using `gh` CLI
- `skills/git/review-pr/SKILL.md` — GitHub-only skill using `gh` CLI
- `skills/git/respond-to-pr/SKILL.md` — GitHub-only skill with one
  `git branch` call (line 68)
- `.claude-plugin/plugin.json` — Plugin manifest with skills array (lines 9-14)
- `~/.claude/settings.json` — User hooks configuration (lines 73-85)
- `~/.claude/hooks/rtk-rewrite.sh` — RTK PreToolUse hook pattern (reference
  implementation for command rewriting)

## Architecture Insights

### Hook Lifecycle and Context Injection

Claude Code's hook system supports multiple injection points, but
`SessionStart` with `additionalContext` is the only one that provides
**persistent, session-wide context** without repeated execution. This makes it
the natural fit for VCS detection, which is a per-session constant.

The hook output protocol uses structured JSON on stdout:

```json
{
  "hookSpecificOutput": {
    "additionalContext": "string injected into Claude's context"
  }
}
```

Exit code 0 means the hook succeeded and its output should be processed.
Non-zero exit codes are treated as errors.

### Plugin Hook Registration

Plugins can register hooks via `hooks/hooks.json` in the plugin directory.
The accelerator plugin does not currently have a `hooks/` directory, so this
would be a new addition. The `${CLAUDE_PLUGIN_ROOT}` variable resolves to
the plugin's installed path at runtime, allowing portable script references.

### Skill Dynamic Context (`!` Backtick Expressions)

The `commit` skill uses `!`command`` syntax to inject live data at skill
invocation time. For VCS-agnostic behaviour, these expressions could invoke
wrapper scripts that detect VCS and run the appropriate command. For example:

```markdown
- Current status: !`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-status.sh`
- Recent history: !`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-log.sh`
```

## Related Research

- `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md` — Documents
  the skill/agent/command distinction and skill-scoped hooks capability
- `meta/research/codebase/2026-03-14-plugin-extraction.md` — Documents plugin directory
  structure including `hooks/` as a standard location
- `meta/research/codebase/2026-03-15-context-management-approaches.md` — Documents
  hook-based context injection patterns

## Open Questions

1. **Should the commit skill become a single VCS-agnostic skill or should there
   be separate `commit` and `jj-commit` skills?** A single skill is DRYer but
   requires more sophisticated prompt engineering to handle the conceptual
   differences (staging area vs. no staging area).

2. **Should the VCS detection hook live in the accelerator plugin or in the
   user's personal hooks?** If in the plugin, all users of the plugin get
   auto-detection. If personal, the user has full control but must configure it
   themselves.

3. **How should the `!` backtick expressions in the commit skill be handled?**
   These are evaluated at invocation time and currently hardcode git commands.
   Options: wrapper scripts, conditional expressions, or removing them in
   favour of in-skill command execution.

4. **Should the PreToolUse guard hook block git commands in jj repos or just
   warn?** Blocking is safer but may frustrate users who intentionally use git
   commands in colocated repos. Warning (via `systemMessage`) is gentler.

5. **How does this interact with RTK?** If RTK rewrites `jj status` to
   `rtk jj status`, the RTK registry would need to understand jj commands. If
   RTK doesn't know about jj, the rewrite would be skipped (which is fine — rtk
   exits cleanly for unknown commands).
