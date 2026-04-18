#!/usr/bin/env bash
set -euo pipefail

# Reads the status field from a ticket file's YAML frontmatter.
# Usage: ticket-read-status.sh <path-to-ticket-file>
# Outputs the status value (e.g., "draft", "ready", "in-progress").
# Exits with code 1 if file not found or no valid frontmatter.
#
# Convenience wrapper around ticket-read-field.sh.

if [ $# -lt 1 ]; then
  echo "Usage: ticket-read-status.sh <ticket-file-path>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ticket-read-field.sh" status "$1"
