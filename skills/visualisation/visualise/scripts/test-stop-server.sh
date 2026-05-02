#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
source "$SCRIPT_DIR/test-helpers.sh"

LAUNCH_SERVER="$SCRIPT_DIR/launch-server.sh"
STOP_SERVER="$SCRIPT_DIR/stop-server.sh"

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

echo "=== stop-server.sh ==="
echo ""

# ─── 1. executable ───────────────────────────────────────────────
echo "Test: stop-server.sh is executable"
assert_file_executable "executable bit set" "$STOP_SERVER"

# ─── 2. launcher-helpers.sh is NOT executable ────────────────────
echo "Test: launcher-helpers.sh is not executable (sourced only)"
if [ ! -x "$SCRIPT_DIR/launcher-helpers.sh" ]; then
  echo "  PASS: not executable"
  PASS=$((PASS + 1))
else
  echo "  FAIL: launcher-helpers.sh should not be executable"
  FAIL=$((FAIL + 1))
fi

# ─── 3. not_running when no lifecycle files ───────────────────────
echo "Test: stop with no server → not_running"
PROJ="$TMPDIR_BASE/t-notrun"; make_project "$PROJ"
cd "$PROJ"
OUT="$TMPDIR_BASE/t-notrun.out"; RC=0
bash "$STOP_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "notrun: exit code" "0" "$RC"
assert_json_eq "notrun: status" ".status" "not_running" "$OUT"
cd "$ORIG_DIR"

# ─── 4. stop running server ──────────────────────────────────────
echo "Test: stop running server → stopped, URL unreachable, server-stopped.json exists"
PROJ="$TMPDIR_BASE/t-stop"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-stop"; make_fake_visualiser "$FAKE"
launch_fake "$PROJ" "$FAKE"

INFO_FILE="$PROJ/meta/tmp/visualiser/server-info.json"
URL="$(jq -r '.url' "$INFO_FILE")"

cd "$PROJ"
OUT="$TMPDIR_BASE/t-stop.out"; RC=0
bash "$STOP_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "stop: exit code" "0" "$RC"
assert_json_eq "stop: status" ".status" "stopped" "$OUT"

STOPPED_FILE="$PROJ/meta/tmp/visualiser/server-stopped.json"
if [ -f "$STOPPED_FILE" ]; then
  echo "  PASS: server-stopped.json exists"
  PASS=$((PASS + 1))
else
  echo "  FAIL: server-stopped.json missing"
  FAIL=$((FAIL + 1))
fi

CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
if [ "$CURLRC" -ne 0 ]; then
  echo "  PASS: URL unreachable after stop"
  PASS=$((PASS + 1))
else
  echo "  FAIL: URL still reachable after stop"
  FAIL=$((FAIL + 1))
fi

PID_FILE_PATH="$PROJ/meta/tmp/visualiser/server.pid"
if [ ! -f "$PID_FILE_PATH" ]; then
  echo "  PASS: server.pid removed"
  PASS=$((PASS + 1))
else
  echo "  FAIL: server.pid still present"
  FAIL=$((FAIL + 1))
fi
cd "$ORIG_DIR"

# ─── 5. double launch → reuse, same URL ──────────────────────────
echo "Test: double launch reuses existing server (same URL)"
PROJ="$TMPDIR_BASE/t-reuse"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-reuse"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
URL1="$(bash "$LAUNCH_SERVER" 2>/dev/null | grep '^\*\*Visualiser URL\*\*:' | sed 's/\*\*Visualiser URL\*\*: //')" || true
URL2="$(bash "$LAUNCH_SERVER" 2>/dev/null | grep '^\*\*Visualiser URL\*\*:' | sed 's/\*\*Visualiser URL\*\*: //')" || true
assert_eq "reuse: same URL both times" "$URL1" "$URL2"
PID1="$(tr -cd '0-9' < "$PROJ/meta/tmp/visualiser/server.pid")"
CURLRC=0; curl -fsS "$URL2" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "reuse: URL reachable on second launch" "0" "$CURLRC"
unset ACCELERATOR_VISUALISER_BIN
bash "$STOP_SERVER" >/dev/null 2>/dev/null || true
cd "$ORIG_DIR"

# ─── 6. stale PID cleanup ────────────────────────────────────────
echo "Test: stale server.pid → launcher starts fresh server"
PROJ="$TMPDIR_BASE/t-stale"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-stale"; make_fake_visualiser "$FAKE"
STALE_PID="$(spawn_and_reap_pid)"

INFO_DIR="$PROJ/meta/tmp/visualiser"; mkdir -p "$INFO_DIR"
cat > "$INFO_DIR/server-info.json" << INFOJSON
{"version":"0.0.0-stale","pid":$STALE_PID,"start_time":null,"host":"127.0.0.1","port":9998,"url":"http://127.0.0.1:9998","log_path":"$INFO_DIR/server.log","tmp_path":"$INFO_DIR"}
INFOJSON
echo "$STALE_PID" > "$INFO_DIR/server.pid"

cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
OUT="$TMPDIR_BASE/t-stale.out"
bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
NEW_PID="$(tr -cd '0-9' < "$INFO_DIR/server.pid" 2>/dev/null || echo '')"
if [ "$NEW_PID" != "$STALE_PID" ] && [ -n "$NEW_PID" ]; then
  echo "  PASS: fresh PID (not the stale one)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: expected fresh PID, got '$NEW_PID' (stale was '$STALE_PID')"
  FAIL=$((FAIL + 1))
fi
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "stale: fresh server reachable" "0" "$CURLRC"
unset ACCELERATOR_VISUALISER_BIN
bash "$STOP_SERVER" >/dev/null 2>/dev/null || true
cd "$ORIG_DIR"

# ─── 7. identity-mismatch refusal ────────────────────────────────
echo "Test: stop refuses when PID start_time doesn't match"
PROJ="$TMPDIR_BASE/t-idmm"; make_project "$PROJ"
INFO_DIR="$PROJ/meta/tmp/visualiser"; mkdir -p "$INFO_DIR"
OWN_PID=$$
cat > "$INFO_DIR/server-info.json" << INFOJSON
{"version":"0.0.0-test","pid":$OWN_PID,"start_time":1,"host":"127.0.0.1","port":9997,"url":"http://127.0.0.1:9997","log_path":"$INFO_DIR/server.log","tmp_path":"$INFO_DIR"}
INFOJSON
echo "$OWN_PID" > "$INFO_DIR/server.pid"

cd "$PROJ"
OUT="$TMPDIR_BASE/t-idmm.out"; RC=0
bash "$STOP_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "idmm: exit code (refused)" "1" "$RC"
assert_json_eq "idmm: status" ".status" "refused" "$OUT"
# Verify the reason field contains the expected substring from the captured output
REASON="$(jq -r '.reason // empty' "$OUT" 2>/dev/null || true)"
if echo "$REASON" | grep -qF "pid identity mismatch"; then
  echo "  PASS: idmm: reason contains 'pid identity mismatch'"
  PASS=$((PASS + 1))
else
  echo "  FAIL: idmm: expected reason to contain 'pid identity mismatch', got: $REASON"
  FAIL=$((FAIL + 1))
fi

# Confirm our own process is still alive
if kill -0 "$OWN_PID" 2>/dev/null; then
  echo "  PASS: harness process still alive after refused stop"
  PASS=$((PASS + 1))
else
  echo "  FAIL: harness process was killed!"
  FAIL=$((FAIL + 1))
fi

# Stale lifecycle files should be removed
if [ ! -f "$INFO_DIR/server.pid" ] && [ ! -f "$INFO_DIR/server-info.json" ]; then
  echo "  PASS: stale lifecycle files removed after refusal"
  PASS=$((PASS + 1))
else
  echo "  FAIL: stale lifecycle files not cleaned up"
  FAIL=$((FAIL + 1))
fi
cd "$ORIG_DIR"

# ─── 8. forced SIGKILL path ──────────────────────────────────────
echo "Test: unkillable server escalates to SIGKILL, server-stopped.json synthesised"
PROJ="$TMPDIR_BASE/t-sigkill"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-sigkill"; make_unkillable_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
bash "$LAUNCH_SERVER" >/dev/null 2>/dev/null
OUT="$TMPDIR_BASE/t-sigkill.out"; RC=0
bash "$STOP_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "sigkill: exit code" "0" "$RC"
assert_json_eq "sigkill: status" ".status" "stopped" "$OUT"
assert_json_eq "sigkill: forced flag" ".forced" "true" "$OUT"
STOPPED_FILE="$PROJ/meta/tmp/visualiser/server-stopped.json"
if [ -f "$STOPPED_FILE" ]; then
  echo "  PASS: server-stopped.json synthesised after SIGKILL"
  PASS=$((PASS + 1))
else
  echo "  FAIL: server-stopped.json missing after SIGKILL"
  FAIL=$((FAIL + 1))
fi
assert_json_eq "sigkill: stopped reason" ".reason" "forced-sigkill" "$STOPPED_FILE"
assert_json_eq "sigkill: stopped written_by" ".written_by" "stop-server.sh" "$STOPPED_FILE"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

test_summary
