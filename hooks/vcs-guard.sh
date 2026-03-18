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
