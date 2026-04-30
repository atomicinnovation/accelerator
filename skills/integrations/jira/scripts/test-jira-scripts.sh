#!/usr/bin/env bash
set -euo pipefail

# Umbrella test runner for Jira integration scripts.
# Run: bash skills/integrations/jira/scripts/test-jira-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

EXIT_CODE=0
bash "$SCRIPT_DIR/test-jira-common.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-auth.sh" || EXIT_CODE=$?

exit "$EXIT_CODE"
