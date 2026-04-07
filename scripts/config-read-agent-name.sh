#!/usr/bin/env bash
set -euo pipefail

# Reads a single agent name override from accelerator config files.
# Outputs the configured override or the default agent name.
#
# This script is for inline use at critical spawn points where truly
# deterministic preprocessor-time resolution is required (e.g., the
# subagent_type parameter in review-pr and review-plan). For bulk
# resolution of all agent names, use config-read-agents.sh instead.
#
# Usage: config-read-agent-name.sh <default-agent-name>
#
# Example: config-read-agent-name.sh reviewer
#   -> outputs "my-custom-reviewer" if configured, otherwise "reviewer"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

DEFAULT="${1:-}"
if [ -z "$DEFAULT" ]; then
  echo "Usage: config-read-agent-name.sh <default-agent-name>" >&2
  exit 1
fi

"$SCRIPT_DIR/config-read-value.sh" "agents.$DEFAULT" "${AGENT_PREFIX}$DEFAULT"
