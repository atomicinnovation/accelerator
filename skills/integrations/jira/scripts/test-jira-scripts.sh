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
bash "$SCRIPT_DIR/test-jira-jql.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-adf-to-md.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-md-to-adf.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-adf-roundtrip.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-request.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-fields.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-init-flow.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-render-adf-fields.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-search.sh" || EXIT_CODE=$?
bash "$SCRIPT_DIR/test-jira-show.sh" || EXIT_CODE=$?

exit "$EXIT_CODE"
