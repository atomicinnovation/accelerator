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
