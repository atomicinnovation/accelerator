#!/usr/bin/env bash
set -euo pipefail

# Reads a single agent name override from accelerator config files.
# Outputs the configured override or the default agent name.
#
# Usage: config-read-agent-name.sh <default-agent-name>
#
# Example: config-read-agent-name.sh reviewer
#   -> outputs "my-custom-reviewer" if configured, otherwise "reviewer"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DEFAULT="${1:-}"
if [ -z "$DEFAULT" ]; then
  echo "Usage: config-read-agent-name.sh <default-agent-name>" >&2
  exit 1
fi

"$SCRIPT_DIR/config-read-value.sh" "agents.$DEFAULT" "$DEFAULT"
