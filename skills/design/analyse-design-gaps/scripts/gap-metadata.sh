#!/usr/bin/env bash
set -euo pipefail

# Generates metadata for analyse-design-gaps artifacts.
#
# Usage: gap-metadata.sh
#
# Outputs key-value lines used to populate gap artifact frontmatter.

DATETIME_TZ=$(date '+%Y-%m-%d %H:%M:%S %Z')
FILENAME_DATE=$(date '+%Y-%m-%d')

if command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
  REPO_ROOT=$(jj root)
  REPO_NAME=$(basename "$REPO_ROOT")
  GIT_COMMIT=$(jj log -r @ --no-graph --template 'commit_id' 2>/dev/null || echo "")
  GIT_BRANCH=$(jj bookmark list --all 2>/dev/null | head -1 | awk '{print $1}' || echo "")
elif command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel)
  REPO_NAME=$(basename "$REPO_ROOT")
  GIT_BRANCH=$(git branch --show-current 2>/dev/null || git rev-parse --abbrev-ref HEAD)
  GIT_COMMIT=$(git rev-parse HEAD)
else
  REPO_ROOT=""
  REPO_NAME=""
  GIT_BRANCH=""
  GIT_COMMIT=""
fi

echo "Current Date/Time (TZ): $DATETIME_TZ"
echo "Date For Filename: $FILENAME_DATE"
[ -n "$GIT_COMMIT" ] && echo "Current Git Commit Hash: $GIT_COMMIT"
[ -n "$GIT_BRANCH" ] && echo "Current Branch Name: $GIT_BRANCH"
[ -n "$REPO_NAME" ] && echo "Repository Name: $REPO_NAME"
