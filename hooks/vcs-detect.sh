#!/usr/bin/env bash

# Check for jq dependency
if ! command -v jq &>/dev/null; then
  echo '{"systemMessage":"WARNING: jq is not installed. VCS detection could not run. Install jq for full VCS support. Defaulting to git commands."}'
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

# Locally realpath-normalise REPO_ROOT inside this hook only (we do NOT
# touch find_repo_root, which has many external callers). After this line,
# every path in the emitted additionalContext message shares one
# normalisation regime — REPO_ROOT and the new boundary paths.
if [ -n "$REPO_ROOT" ]; then
  REPO_ROOT=$(realpath "$REPO_ROOT" 2>/dev/null || printf '%s' "$REPO_ROOT")
fi

# Single source of truth for AC1 prohibition wording. Used by every kind
# that emits one or more parent blocks.
_emit_parent_block() {
  local label="$1" parent="$2"
  printf 'Parent repository (%s): %s\n' "$label" "$parent"
  printf 'do not edit files in %s\n' "$parent"
  printf 'do not run VCS commands against %s\n' "$parent"
  printf 'do not grep, find, or research files in %s\n' "$parent"
}

# Build the boundary block. The kind_suffix is "" for single-VCS kinds
# and " (colocated)" / " (nested)" for dual-parent kinds.
build_boundary_block() {
  local kind_suffix="$1" boundary="$2" jj_parent="$3" git_parent="$4"
  printf '\n\nWORKSPACE BOUNDARY DETECTED%s\n' "$kind_suffix"
  printf 'You are inside a checkout that is NOT the main repository.\n'
  printf 'Boundary (active workspace): %s\n' "$boundary"
  printf '\n'
  if [ -n "$jj_parent" ]; then
    _emit_parent_block "jj" "$jj_parent"
    [ -n "$git_parent" ] && printf '\n'
  fi
  if [ -n "$git_parent" ]; then
    _emit_parent_block "git" "$git_parent"
  fi
}

# Probe once, parse once. C_JJ_MISSING / C_GIT_MISSING default to "0"
# so the diagnostic branch never fires for older classify_checkout
# implementations that did not emit those fields (forward-compat).
CHECKOUT_RECORD=$(classify_checkout .)
C_KIND=""; C_BOUNDARY=""; C_JJ_PARENT=""; C_GIT_PARENT=""
C_JJ_MISSING="0"; C_GIT_MISSING="0"
while IFS='=' read -r k v; do
  case "$k" in
    KIND) C_KIND=$v ;;
    BOUNDARY) C_BOUNDARY=$v ;;
    JJ_PARENT) C_JJ_PARENT=$v ;;
    GIT_PARENT) C_GIT_PARENT=$v ;;
    JJ_MISSING) C_JJ_MISSING=$v ;;
    GIT_MISSING) C_GIT_MISSING=$v ;;
  esac
done <<< "$CHECKOUT_RECORD"

# Missing-binary diagnostic. Fires based on the classifier's
# JJ_MISSING / GIT_MISSING fields rather than KIND, because a missing
# binary collapses KIND toward `none` or single-VCS — gating on KIND
# would silently fail to diagnose the most common scenario (jj missing
# inside a lone jj secondary workspace, where KIND=none). Mirrors the
# existing jq-missing pattern in this hook.
SYSTEM_MESSAGE=""
if [ "$C_JJ_MISSING" = "1" ]; then
  SYSTEM_MESSAGE="vcs-detect.sh: jj binary not on PATH; jj-side boundary detection was skipped (ancestor .jj marker present)."
elif [ "$C_GIT_MISSING" = "1" ]; then
  SYSTEM_MESSAGE="vcs-detect.sh: git binary not on PATH; git-side boundary detection was skipped (ancestor .git marker present)."
fi

case "$C_KIND" in
  jj-secondary|git-worktree)
    BOUNDARY_OUT=$(build_boundary_block "" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    # $() strips trailing newlines; explicitly restore one so future
    # appended content is not run-on with the prohibition lines.
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
  colocated)
    BOUNDARY_OUT=$(build_boundary_block " (colocated)" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
  nested-jj-in-git|nested-git-in-jj)
    BOUNDARY_OUT=$(build_boundary_block " (nested)" \
      "$C_BOUNDARY" "$C_JJ_PARENT" "$C_GIT_PARENT")
    CONTEXT="${CONTEXT}${BOUNDARY_OUT}"$'\n'
    ;;
esac

# Output as SessionStart hook response.
# Use jq to safely encode the context string as JSON. The systemMessage
# field is added only when SYSTEM_MESSAGE is non-empty, preserving the
# AC5 byte-identity contract for main-checkout sessions.
jq -n \
  --arg context "$CONTEXT" \
  --arg sys "$SYSTEM_MESSAGE" \
  '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $context}}
   + (if $sys == "" then {} else {systemMessage: $sys} end)'
