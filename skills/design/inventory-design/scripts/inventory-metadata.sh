#!/usr/bin/env bash
set -euo pipefail

# Generates metadata for inventory-design artifacts.
#
# Usage: inventory-metadata.sh
#
# Outputs key-value lines used to populate inventory frontmatter.

DATETIME_UTC=$(date -u +%Y-%m-%dT%H:%M:%S+00:00)
FILENAME_TS=$(date '+%Y-%m-%d-%H%M%S')

if command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
  REPO_ROOT=$(jj root)
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
echo "Timestamp For Filename: $FILENAME_TS"
[ -n "$REVISION" ] && echo "Current Revision: $REVISION"
[ -n "$REPO_NAME" ] && echo "Repository Name: $REPO_NAME"
