#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

LAUNCH_SERVER="$SCRIPT_DIR/launch-server.sh"
EXPECTED_SENTINEL="placeholder://phase-1-scaffold-not-yet-running"

echo "=== launch-server.sh (Phase 1 stub) ==="
echo ""

echo "Test: script is executable"
assert_file_executable "executable bit set" "$LAUNCH_SERVER"

echo "Test: exits 0"
assert_exit_code "exits 0" 0 bash "$LAUNCH_SERVER"

echo "Test: prints the placeholder sentinel"
OUTPUT=$(bash "$LAUNCH_SERVER")
assert_eq "stdout matches sentinel" "$EXPECTED_SENTINEL" "$OUTPUT"

echo "Test: output is exactly one line"
LINE_COUNT=$(bash "$LAUNCH_SERVER" | wc -l | tr -d ' ')
assert_eq "one line of output" "1" "$LINE_COUNT"

echo "Test: stderr is empty on happy path"
assert_stderr_empty "no stderr output" bash "$LAUNCH_SERVER"

echo "Test: ignores extra arguments (forward-compatible for Phase 2 flags)"
assert_exit_code "exits 0 with --foo bar" 0 bash "$LAUNCH_SERVER" --foo bar

test_summary
