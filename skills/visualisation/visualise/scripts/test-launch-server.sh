#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
source "$SCRIPT_DIR/test-helpers.sh"

LAUNCH_SERVER="$SCRIPT_DIR/launch-server.sh"

TMPDIR_BASE="$(mktemp -d)"
ORIG_DIR="$PWD"
trap '
  reap_visualiser_fakes "$TMPDIR_BASE"
  cd "$ORIG_DIR"
  rm -rf "$TMPDIR_BASE"
' EXIT

PLUGIN_VERSION="$(jq -r .version "$PLUGIN_ROOT/.claude-plugin/plugin.json")"
OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"
case "$OS_RAW" in darwin) OS="darwin" ;; linux) OS="linux" ;; *) OS="$OS_RAW" ;; esac
case "$ARCH_RAW" in arm64|aarch64) ARCH="arm64" ;; x86_64) ARCH="x64" ;; *) ARCH="$ARCH_RAW" ;; esac

# All test projects get .jj so find_repo_root succeeds, plus the init sentinel.
make_project() { local d="$1"; mkdir -p "$d/.jj" "$d/.claude" "$d/meta/tmp"; : > "$d/meta/tmp/.gitignore"; }

echo "=== launch-server.sh (Phase 2) ==="
echo ""

# ─── 1. executable ───────────────────────────────────────────────
echo "Test: script is executable"
assert_file_executable "executable bit set" "$LAUNCH_SERVER"

# ─── 2. placeholder sentinel refusal ─────────────────────────────
echo "Test: placeholder checksums → sentinel refusal"
PROJ="$TMPDIR_BASE/t-sentinel"; make_project "$PROJ"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
RC=0; ERR="$TMPDIR_BASE/t-sentinel.err"
bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
assert_eq "sentinel: exit code" "1" "$RC"
assert_json_eq "sentinel: error field" ".error" "no released binary for this plugin version" "$ERR"
cd "$ORIG_DIR"

# ─── 3. ACCELERATOR_VISUALISER_BIN happy path ────────────────────
echo "Test: ACCELERATOR_VISUALISER_BIN → server starts and URL is printed"
PROJ="$TMPDIR_BASE/t-binenv"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-binenv"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
OUT="$TMPDIR_BASE/t-binenv.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "binenv: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
URLMATCH="$(echo "$URL" | grep -cE '^http://127\.0\.0\.1:[0-9]+/?$')" || true
assert_eq "binenv: URL format" "1" "$URLMATCH"
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "binenv: curl 200" "0" "$CURLRC"
CFG_FILE="$PROJ/meta/tmp/visualiser/config.json"
assert_json_eq "config: decisions"    ".doc_paths.decisions"    "$PROJ/meta/decisions"     "$CFG_FILE"
assert_json_eq "config: tickets"      ".doc_paths.tickets"      "$PROJ/meta/tickets"       "$CFG_FILE"
assert_json_eq "config: plans"        ".doc_paths.plans"        "$PROJ/meta/plans"         "$CFG_FILE"
assert_json_eq "config: research"     ".doc_paths.research"     "$PROJ/meta/research"      "$CFG_FILE"
assert_json_eq "config: review_plans" ".doc_paths.review_plans" "$PROJ/meta/reviews/plans" "$CFG_FILE"
assert_json_eq "config: review_prs"   ".doc_paths.review_prs"   "$PROJ/meta/reviews/prs"   "$CFG_FILE"
assert_json_eq "config: validations"  ".doc_paths.validations"  "$PROJ/meta/validations"   "$CFG_FILE"
assert_json_eq "config: notes"        ".doc_paths.notes"        "$PROJ/meta/notes"         "$CFG_FILE"
assert_json_eq "config: prs"          ".doc_paths.prs"          "$PROJ/meta/prs"           "$CFG_FILE"
assert_json_eq "config: adr user_override"            ".templates.adr.user_override"                "$PROJ/meta/templates/adr.md"            "$CFG_FILE"
assert_json_eq "config: plan user_override"           ".templates.plan.user_override"               "$PROJ/meta/templates/plan.md"           "$CFG_FILE"
assert_json_eq "config: research user_override"       ".templates.research.user_override"           "$PROJ/meta/templates/research.md"       "$CFG_FILE"
assert_json_eq "config: validation user_override"     ".templates.validation.user_override"         "$PROJ/meta/templates/validation.md"     "$CFG_FILE"
assert_json_eq "config: pr-description user_override" '.templates."pr-description".user_override'  "$PROJ/meta/templates/pr-description.md" "$CFG_FILE"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 4. reuse short-circuit ──────────────────────────────────────
echo "Test: reuse short-circuit — second launch returns same URL"
PROJ="$TMPDIR_BASE/t-reuse"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-reuse"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
URL1="$(bash "$LAUNCH_SERVER" 2>/dev/null | grep '^\*\*Visualiser URL\*\*:' | sed 's/\*\*Visualiser URL\*\*: //')" || true
URL2="$(bash "$LAUNCH_SERVER" 2>/dev/null | grep '^\*\*Visualiser URL\*\*:' | sed 's/\*\*Visualiser URL\*\*: //')" || true
assert_eq "reuse: same URL both times" "$URL1" "$URL2"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 5. visualiser.binary config key (absolute path) ─────────────
echo "Test: visualiser.binary config key (absolute path)"
PROJ="$TMPDIR_BASE/t-cfgabs"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-cfgabs"; make_fake_visualiser "$FAKE"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
printf -- '---\nvisualiser:\n  binary: %s\n---\n' "$FAKE" > "$PROJ/.claude/accelerator.local.md"
OUT="$TMPDIR_BASE/t-cfgabs.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "cfgabs: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "cfgabs: curl 200" "0" "$CURLRC"
cd "$ORIG_DIR"

# ─── 6. visualiser.binary config key (relative path) ─────────────
echo "Test: visualiser.binary config key (relative path)"
PROJ="$TMPDIR_BASE/t-cfgrel"; make_project "$PROJ"; mkdir -p "$PROJ/bin"
FAKE="$PROJ/bin/fake-server"; make_fake_visualiser "$FAKE"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
printf -- '---\nvisualiser:\n  binary: bin/fake-server\n---\n' > "$PROJ/.claude/accelerator.local.md"
OUT="$TMPDIR_BASE/t-cfgrel.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "cfgrel: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "cfgrel: curl 200" "0" "$CURLRC"
cd "$ORIG_DIR"

# ─── 7. env var beats config key ─────────────────────────────────
echo "Test: ACCELERATOR_VISUALISER_BIN env var takes precedence over config key"
PROJ="$TMPDIR_BASE/t-prec"; make_project "$PROJ"
FAKE_ENV="$TMPDIR_BASE/fake-env-prec"; make_fake_visualiser "$FAKE_ENV"
FAKE_CFG="$TMPDIR_BASE/fake-cfg-prec"; make_fake_visualiser "$FAKE_CFG"
cd "$PROJ"
printf -- '---\nvisualiser:\n  binary: %s\n---\n' "$FAKE_CFG" > "$PROJ/.claude/accelerator.local.md"
export ACCELERATOR_VISUALISER_BIN="$FAKE_ENV"
OUT="$TMPDIR_BASE/t-prec.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "prec: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "prec: curl 200" "0" "$CURLRC"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 8. non-executable binary → error ────────────────────────────
echo "Test: non-executable visualiser.binary → error"
PROJ="$TMPDIR_BASE/t-nonexec"; make_project "$PROJ"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
echo "not-a-binary" > "$TMPDIR_BASE/nonexec-file"
printf -- '---\nvisualiser:\n  binary: %s\n---\n' "$TMPDIR_BASE/nonexec-file" > "$PROJ/.claude/accelerator.local.md"
RC=0; ERR="$TMPDIR_BASE/t-nonexec.err"
bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
assert_eq "nonexec: exit code" "1" "$RC"
assert_json_eq "nonexec: error field" ".error" "configured visualiser.binary is not executable" "$ERR"
cd "$ORIG_DIR"

# ─── 9. concurrent launch → flock refusal ────────────────────────
echo "Test: concurrent launch is serialised (flock refusal)"
PROJ="$TMPDIR_BASE/t-conc"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-conc"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
LOCK_FILE="$PROJ/meta/tmp/visualiser/launcher.lock"
mkdir -p "$(dirname "$LOCK_FILE")"
if command -v flock >/dev/null 2>&1; then
  exec 8>"$LOCK_FILE"; flock 8
  RC=0; ERR="$TMPDIR_BASE/t-conc.err"
  bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
  assert_eq "concurrent: exit code" "1" "$RC"
  assert_json_eq "concurrent: error field" ".error" "another launcher is running" "$ERR"
  exec 8>&-
else
  mkdir "$LOCK_FILE.d"
  RC=0; ERR="$TMPDIR_BASE/t-conc.err"
  bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
  assert_eq "concurrent (mkdir): exit code" "1" "$RC"
  assert_json_eq "concurrent (mkdir): error field" ".error" "another launcher is running" "$ERR"
  rmdir "$LOCK_FILE.d"
fi
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 10. PID identity mismatch → fresh launch ────────────────────
echo "Test: stale server-info.json with wrong start_time → fresh launch"
PROJ="$TMPDIR_BASE/t-pidmm"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-pidmm"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
INFO_DIR="$PROJ/meta/tmp/visualiser"; mkdir -p "$INFO_DIR"
OWN_PID=$$
cat > "$INFO_DIR/server-info.json" << INFOJSON
{"version":"0.0.0-stale","pid":$OWN_PID,"start_time":1,"host":"127.0.0.1","port":9999,"url":"http://127.0.0.1:9999","log_path":"$INFO_DIR/server.log","tmp_path":"$INFO_DIR"}
INFOJSON
echo "$OWN_PID" > "$INFO_DIR/server.pid"
OUT="$TMPDIR_BASE/t-pidmm.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "pidmismatch: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
STALE="$(echo "$URL" | grep -c ':9999')" || true
assert_eq "pidmismatch: not the stale URL" "0" "$STALE"
CURLRC=0; curl -fsS "$URL" >/dev/null 2>/dev/null || CURLRC=$?
assert_eq "pidmismatch: fresh server reachable" "0" "$CURLRC"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 11. checksum mismatch (HTTP fixture) ────────────────────────
echo "Test: checksum mismatch → error (HTTP fixture serving wrong bytes)"
PROJ="$TMPDIR_BASE/t-shamm"; make_project "$PROJ"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
FAKE_SKILL="$TMPDIR_BASE/fake-skill"; mkdir -p "$FAKE_SKILL/bin"
EXPECTED_SHA="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
cat > "$FAKE_SKILL/bin/checksums.json" << CHECKJSON
{"version":"$PLUGIN_VERSION","binaries":{"darwin-arm64":"sha256:$EXPECTED_SHA","darwin-x64":"sha256:$EXPECTED_SHA","linux-arm64":"sha256:$EXPECTED_SHA","linux-x64":"sha256:$EXPECTED_SHA"}}
CHECKJSON
FIXTURE_DIR="$TMPDIR_BASE/fixture-srv/v${PLUGIN_VERSION}"
mkdir -p "$FIXTURE_DIR"
echo "wrong-content-will-not-match-sha" > "$FIXTURE_DIR/accelerator-visualiser-${OS}-${ARCH}"
PORT_FILE="$TMPDIR_BASE/fixture-port"
python3 - << PYEOF &
import http.server, os
class H(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
srv = http.server.HTTPServer(('127.0.0.1', 0), H)
os.chdir("$TMPDIR_BASE/fixture-srv")
open("$PORT_FILE", 'w').write(str(srv.server_address[1]) + '\n')
srv.serve_forever()
PYEOF
HTTP_PID=$!
for _ in $(seq 1 30); do [ -f "$PORT_FILE" ] && break; sleep 0.1; done
if [ -f "$PORT_FILE" ]; then
  SRV_PORT="$(tr -d '[:space:]' < "$PORT_FILE")"
  RC=0; ERR="$TMPDIR_BASE/t-shamm.err"
  ACCELERATOR_VISUALISER_SKILL_ROOT="$FAKE_SKILL" \
    ACCELERATOR_VISUALISER_RELEASES_URL="http://127.0.0.1:${SRV_PORT}" \
    ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1 \
    bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
  kill "$HTTP_PID" 2>/dev/null || true
  assert_eq "shamm: exit code" "1" "$RC"
  assert_json_eq "shamm: error field" ".error" "checksum mismatch" "$ERR"
else
  kill "$HTTP_PID" 2>/dev/null || true
  echo "  SKIP: could not start HTTP fixture server"
fi
cd "$ORIG_DIR"

# ─── 12. unsupported platform → error ────────────────────────────
echo "Test: unsupported platform → error"
PROJ="$TMPDIR_BASE/t-bados"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-bados"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
FAKE_BINS="$TMPDIR_BASE/fake-bins-os"; mkdir -p "$FAKE_BINS"
printf '#!/usr/bin/env bash\ncase "$1" in -s) echo "Windows_NT";; -m) echo "x86_64";; *) echo "Windows_NT";; esac\n' \
  > "$FAKE_BINS/uname"
chmod +x "$FAKE_BINS/uname"
RC=0; ERR="$TMPDIR_BASE/t-bados.err"
PATH="$FAKE_BINS:$PATH" bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
assert_eq "badplatform: exit code" "1" "$RC"
assert_json_eq "badplatform: error field" ".error" "unsupported platform" "$ERR"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 13. uninitialised project is rejected ───────────────────────
echo "Test: uninitialised project (no sentinel) → rejected with JSON error"
PROJ="$TMPDIR_BASE/t-uninit"
mkdir -p "$PROJ/.jj" "$PROJ/.claude" "$PROJ/meta/tmp"
cd "$PROJ"
unset ACCELERATOR_VISUALISER_BIN 2>/dev/null || true
RC=0; ERR="$TMPDIR_BASE/t-uninit.err"
bash "$LAUNCH_SERVER" >/dev/null 2>"$ERR" || RC=$?
assert_eq "uninit: exit code" "1" "$RC"
UNINIT_ERR="$(jq -r '.error // empty' "$ERR" 2>/dev/null)"
UNINIT_HINT="$(jq -r '.hint // empty' "$ERR" 2>/dev/null)"
assert_eq "uninit: error field" "accelerator not initialised" "$UNINIT_ERR"
assert_contains "uninit: hint mentions /accelerator:init" "$UNINIT_HINT" "/accelerator:init"
assert_contains "uninit: hint mentions project root" "$UNINIT_HINT" "$PROJ"
assert_dir_absent "uninit: no visualiser tmp dir created" "$PROJ/meta/tmp/visualiser"
cd "$ORIG_DIR"

# ─── 14. initialised project proceeds past sentinel ──────────────
echo "Test: initialised project proceeds past sentinel check"
PROJ="$TMPDIR_BASE/t-initok"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-initok"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
OUT="$TMPDIR_BASE/t-initok.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "initok: exit code" "0" "$RC"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

# ─── 15. sentinel deletion does not kill already-running server ──
echo "Test: sentinel deletion mid-session → reuse short-circuit still works"
PROJ="$TMPDIR_BASE/t-sentdel"; make_project "$PROJ"
FAKE="$TMPDIR_BASE/fake-sentdel"; make_fake_visualiser "$FAKE"
cd "$PROJ"
export ACCELERATOR_VISUALISER_BIN="$FAKE"
bash "$LAUNCH_SERVER" >/dev/null 2>/dev/null || true
# Delete the sentinel after the server is running.
rm -f "$PROJ/meta/tmp/.gitignore"
OUT="$TMPDIR_BASE/t-sentdel.out"
RC=0; bash "$LAUNCH_SERVER" >"$OUT" 2>/dev/null || RC=$?
assert_eq "sentdel: exit code" "0" "$RC"
URL="$(grep '^\*\*Visualiser URL\*\*:' "$OUT" 2>/dev/null | sed 's/\*\*Visualiser URL\*\*: //')" || true
URLMATCH="$(echo "$URL" | grep -cE '^http://127\.0\.0\.1:[0-9]+/?$')" || true
assert_eq "sentdel: URL still returned" "1" "$URLMATCH"
unset ACCELERATOR_VISUALISER_BIN
cd "$ORIG_DIR"

test_summary
