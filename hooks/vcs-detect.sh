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
