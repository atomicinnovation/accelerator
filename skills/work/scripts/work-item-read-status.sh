#!/usr/bin/env bash
set -euo pipefail

# Reads the status field from a work item file's YAML frontmatter.
# Usage: work-item-read-status.sh <path-to-work-item-file>
# Outputs the status value (e.g., "draft", "ready", "in-progress").
# Exits with code 1 if file not found or no valid frontmatter.
#
# Convenience wrapper around work-item-read-field.sh.

if [ $# -lt 1 ]; then
  echo "Usage: work-item-read-status.sh <work-item-file-path>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/work-item-read-field.sh" status "$1"
