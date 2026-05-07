#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

NOTIFY="$SCRIPT_DIR/notify-downgrade.sh"
MESSAGES_JSON="$SCRIPT_DIR/notify-downgrade-messages.json"
FIXTURES_DIR="$SCRIPT_DIR/../evals/fixtures/notify-downgrade"

echo "=== notify-downgrade.sh: structural ==="

assert_file_exists "notify-downgrade.sh exists" "$NOTIFY"
assert_file_executable "notify-downgrade.sh is executable" "$NOTIFY"
assert_file_exists "notify-downgrade-messages.json exists" "$MESSAGES_JSON"
assert_exit_code "notify-downgrade-messages.json is valid JSON" 0 jq empty "$MESSAGES_JSON"
assert_exit_code "fixtures directory exists" 0 test -d "$FIXTURES_DIR"

echo ""
echo "=== notify-downgrade.sh: per-reason output matches fixtures ==="

while IFS= read -r key; do
  FIXTURE="$FIXTURES_DIR/${key}.expected.txt"
  assert_file_exists "fixture exists for $key" "$FIXTURE"
  ACTUAL="$(bash "$NOTIFY" --reason "$key")"
  EXPECTED="$(cat "$FIXTURE")"
  assert_eq "output matches fixture for $key" "$EXPECTED" "$ACTUAL"
done < <(jq -r 'keys[]' "$MESSAGES_JSON")

echo ""
echo "=== notify-downgrade.sh: set-equality between JSON keys and fixtures ==="

JSON_KEYS="$(jq -r 'keys[]' "$MESSAGES_JSON" | sort)"
FIXTURE_KEYS="$(find "$FIXTURES_DIR" -maxdepth 1 -name '*.expected.txt' | sed 's|.*/||;s|\.expected\.txt$||' | sort)"
assert_eq "JSON keys equal fixture set" "$JSON_KEYS" "$FIXTURE_KEYS"

echo ""
echo "=== notify-downgrade.sh: error handling ==="

assert_exit_code "unknown reason rejects with exit 2" 2 \
  bash "$NOTIFY" --reason "not-a-real-reason"

assert_exit_code "missing --reason rejects with exit 2" 2 \
  bash "$NOTIFY"

echo ""
echo "=== notify-downgrade.sh: control character sanitisation ==="

TMPDIR_TEST="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_TEST"' EXIT

PATCHED_JSON="$TMPDIR_TEST/messages.json"

# ANSI CSI escape — should be stripped
PATCHED_NOTIFY="$TMPDIR_TEST/notify-patched.sh"
sed "s|$MESSAGES_JSON|$PATCHED_JSON|g" "$NOTIFY" > "$PATCHED_NOTIFY"
chmod +x "$PATCHED_NOTIFY"
printf '{"node-missing":"hello\033[31mworld\033[0m"}' > "$PATCHED_JSON"
ACTUAL_ESC="$(bash "$PATCHED_NOTIFY" --reason node-missing 2>/dev/null || true)"
assert_not_contains "ANSI escape stripped from output" "$ACTUAL_ESC" $'\033'

# CR (carriage return) — should be stripped
printf '{"node-missing":"hello\rworld"}' > "$PATCHED_JSON"
ACTUAL_CR="$(bash "$PATCHED_NOTIFY" --reason node-missing 2>/dev/null || true)"
assert_not_contains "CR stripped from output" "$ACTUAL_CR" $'\r'

echo ""
echo "=== notify-downgrade.sh: --from and --to accepted without error ==="

assert_exit_code "--from and --to accepted" 0 \
  bash "$NOTIFY" --from hybrid --to code --reason bootstrap-failed

test_summary
