#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

REAL_DISPATCH="$SCRIPT_DIR/visualiser.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

# Build a relocated tree where the lifecycle scripts are replaced
# with sentinel stubs that echo their argv. Lets us assert the
# dispatcher routes to the right script with the right arguments
# without booting the real Rust server.
TEMP_SKILL="$TMPDIR_BASE/skill-copy"
mkdir -p "$TEMP_SKILL/scripts"
cp "$REAL_DISPATCH" "$TEMP_SKILL/scripts/visualiser.sh"
chmod +x "$TEMP_SKILL/scripts/visualiser.sh"
TEMP_DISPATCH="$TEMP_SKILL/scripts/visualiser.sh"

for stub in launch-server stop-server status-server; do
  cat > "$TEMP_SKILL/scripts/$stub.sh" << EOF
#!/usr/bin/env bash
echo "$stub-stub: \$#:\$*"
EOF
  chmod +x "$TEMP_SKILL/scripts/$stub.sh"
done

echo "=== visualiser.sh dispatcher ==="
echo ""

echo "Test: visualiser.sh is executable"
assert_file_executable "executable bit set" "$REAL_DISPATCH"

echo "Test: no arguments → launch-server.sh with no args"
OUT=$(bash "$TEMP_DISPATCH")
assert_eq "no-arg routing" "launch-server-stub: 0:" "$OUT"

echo "Test: empty-string argument → launch-server.sh with no args"
OUT=$(bash "$TEMP_DISPATCH" "")
assert_eq "empty-string routing" "launch-server-stub: 0:" "$OUT"

echo "Test: 'start' argument → launch-server.sh with no args"
OUT=$(bash "$TEMP_DISPATCH" start)
assert_eq "start routing" "launch-server-stub: 0:" "$OUT"

echo "Test: 'stop' argument → stop-server.sh with no args"
OUT=$(bash "$TEMP_DISPATCH" stop)
assert_eq "stop routing" "stop-server-stub: 0:" "$OUT"

echo "Test: 'status' argument → status-server.sh with no args"
OUT=$(bash "$TEMP_DISPATCH" status)
assert_eq "status routing" "status-server-stub: 0:" "$OUT"

echo "Test: unknown subcommand → exit 2 with error JSON on stderr"
ERR_FILE="$TMPDIR_BASE/unknown.err"
RC=0
bash "$TEMP_DISPATCH" bogus >/dev/null 2>"$ERR_FILE" || RC=$?
assert_eq "unknown: exit code" "2" "$RC"
ERR=$(cat "$ERR_FILE")
if echo "$ERR" | jq -e '.error == "unknown subcommand"' >/dev/null 2>&1; then
  echo "  PASS: unknown: error JSON shape"
  PASS=$((PASS + 1))
else
  echo "  FAIL: unknown: expected JSON with error 'unknown subcommand', got: $ERR"
  FAIL=$((FAIL + 1))
fi

test_summary
