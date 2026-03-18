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
