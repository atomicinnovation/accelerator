#!/usr/bin/env bash
set -euo pipefail

# Generates metadata for analyse-design-gaps artifacts.
#
# Usage: gap-metadata.sh
#
# Outputs key-value lines used to populate gap artifact frontmatter.

DATETIME_UTC=$(date -u +%Y-%m-%dT%H:%M:%S+00:00)
FILENAME_DATE=$(date '+%Y-%m-%d')

if command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
  REPO_ROOT=$(jj root)
  # A jj secondary workspace's .jj/repo is a file pointing at the shared store;
  # resolve it so the name is the repository's, not the ephemeral workspace's.
  if [ -f "$REPO_ROOT/.jj/repo" ]; then
    resolved=$(cd "$REPO_ROOT/.jj" 2>/dev/null &&
      cd "$(dirname "$(cat repo)")/.." 2>/dev/null && pwd -P) || true
    [ -n "${resolved:-}" ] && REPO_ROOT="$resolved"
  fi
  REPO_NAME=$(basename "$REPO_ROOT")
  REVISION=$(jj log -r @ --no-graph --template 'commit_id')
elif command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel)
  REPO_NAME=$(basename "$REPO_ROOT")
  REVISION=$(git rev-parse HEAD)
else
  REPO_ROOT=""
  REPO_NAME=""
  REVISION=""
fi

echo "Current Date/Time (UTC): $DATETIME_UTC"
echo "Date For Filename: $FILENAME_DATE"
[ -n "$REVISION" ] && echo "Current Revision: $REVISION"
[ -n "$REPO_NAME" ] && echo "Repository Name: $REPO_NAME"
