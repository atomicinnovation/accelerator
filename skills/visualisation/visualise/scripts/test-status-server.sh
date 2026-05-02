#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
source "$SCRIPT_DIR/test-helpers.sh"

LAUNCH_SERVER="$SCRIPT_DIR/launch-server.sh"
STOP_SERVER="$SCRIPT_DIR/stop-server.sh"
STATUS_SERVER="$SCRIPT_DIR/status-server.sh"

TMPDIR_BASE="$(mktemp -d)"
ORIG_DIR="$PWD"
trap '
  reap_visualiser_fakes "$TMPDIR_BASE"
  cd "$ORIG_DIR"
  rm -rf "$TMPDIR_BASE"
' EXIT

make_project() { local d="$1"; mkdir -p "$d/.jj" "$d/.claude" "$d/meta/tmp"; : > "$d/meta/tmp/.gitignore"; }

launch_fake() {
  local proj="$1" fake="$2"
  cd "$proj"
  export ACCELERATOR_VISUALISER_BIN="$fake"
  bash "$LAUNCH_SERVER" >/dev/null 2>/dev/null
  cd "$ORIG_DIR"
  unset ACCELERATOR_VISUALISER_BIN
}

echo "=== status-server.sh ==="
echo ""

# ─── 1. executable ───────────────────────────────────────────────
echo "Test: status-server.sh is executable"
assert_file_executable "executable bit set" "$STATUS_SERVER"

# ─── 2. not_running when no lifecycle files ──────────────────────
echo "Test: status with no server → not_running"
PROJ="$TMPDIR_BASE/t-statusnr"; make_project "$PROJ"
cd "$PROJ"
OUT="$TMPDIR_BASE/t-statusnr.out"; RC=0
bash "$STATUS_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "statusnr: exit code" "0" "$RC"
assert_json_eq "statusnr: status" ".status" "not_running" "$OUT"
cd "$ORIG_DIR"

# ─── 3. running server → running + url + pid ─────────────────────
echo "Test: status with running server → running + url + pid"
PROJ="$TMPDIR_BASE/t-statusrun"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-statusrun"; make_fake_visualiser "$FAKE"
launch_fake "$PROJ" "$FAKE"

INFO_FILE="$PROJ/meta/tmp/visualiser/server-info.json"
EXPECTED_URL="$(jq -r '.url' "$INFO_FILE")"

cd "$PROJ"
OUT="$TMPDIR_BASE/t-statusrun.out"; RC=0
bash "$STATUS_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "statusrun: exit code" "0" "$RC"
assert_json_eq "statusrun: status" ".status" "running" "$OUT"
assert_json_eq "statusrun: url" ".url" "$EXPECTED_URL" "$OUT"
# Clean up
bash "$STOP_SERVER" >/dev/null 2>/dev/null || true
cd "$ORIG_DIR"

# ─── 4. dead PID in info file → stale ────────────────────────────
echo "Test: status with dead PID in info file → stale"
PROJ="$TMPDIR_BASE/t-statusstale"; make_project "$PROJ"
DEAD_PID="$(spawn_and_reap_pid)"
INFO_DIR="$PROJ/meta/tmp/visualiser"; mkdir -p "$INFO_DIR"
cat > "$INFO_DIR/server-info.json" << INFOJSON
{"version":"0.0.0-test","pid":$DEAD_PID,"start_time":null,"host":"127.0.0.1","port":9996,"url":"http://127.0.0.1:9996","log_path":"$INFO_DIR/server.log","tmp_path":"$INFO_DIR"}
INFOJSON
echo "$DEAD_PID" > "$INFO_DIR/server.pid"

cd "$PROJ"
OUT="$TMPDIR_BASE/t-statusstale.out"; RC=0
bash "$STATUS_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "statusstale: exit code" "0" "$RC"
assert_json_eq "statusstale: status" ".status" "stale" "$OUT"
cd "$ORIG_DIR"

test_summary
