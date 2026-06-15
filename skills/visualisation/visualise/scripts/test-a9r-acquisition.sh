#!/usr/bin/env bash
# Black-box suite for the a9r provisioning path: the SessionStart hook
# (hooks/a9r-provision.sh) and the acquire_binary helper it shares with
# launch-server.sh. Mirrors test-launch-server.sh's HTTP-fixture pattern.
#
# The hook's contract is fail-open and never-stall: every degrade path
# (sentinel, version drift, offline, 404, SHA mismatch, timeout, no
# downloader) must exit 0 and leave no partial file at the cache path. Only a
# genuinely valid release populates the cache. These tests pin all of that.
#
# bash 3.2 floor (macOS /bin/bash, ADR-0016); scanned by lint-bashisms.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/test-helpers.sh"
source "$SCRIPT_DIR/launcher-helpers.sh"

HOOK="$PLUGIN_ROOT/hooks/a9r-provision.sh"
PLUGIN_VERSION="$(jq -r .version "$PLUGIN_ROOT/.claude-plugin/plugin.json")"

OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"
case "$OS_RAW" in darwin) OS="darwin" ;; linux) OS="linux" ;; *) OS="$OS_RAW" ;; esac
case "$ARCH_RAW" in arm64 | aarch64) ARCH="arm64" ;; x86_64) ARCH="x64" ;; *) ARCH="$ARCH_RAW" ;; esac
PLATFORM="$OS-$ARCH"
ASSET="a9r-$PLATFORM"
ZERO_SHA="0000000000000000000000000000000000000000000000000000000000000000"

TMPDIR_BASE="$(mktemp -d)"
HTTP_PIDS=""
PORT=""
cleanup() {
  local p
  for p in $HTTP_PIDS; do kill "$p" 2>/dev/null || true; done
  rm -rf "$TMPDIR_BASE"
}
trap cleanup EXIT

echo "=== a9r-provision.sh + acquire_binary (Phase 5) ==="
echo ""

# A fake skill root: $dir/bin holds checksums.json + the cache file.
make_skill() {
  mkdir -p "$1/bin"
}

# write_manifest <skill> <version> <a9r-sha>  — nested binaries[platform][asset]
write_manifest() {
  local skill="$1" version="$2" a9r_sha="$3"
  cat >"$skill/bin/checksums.json" <<JSON
{"version":"$version","binaries":{"$PLATFORM":{"accelerator-visualiser-$PLATFORM":"sha256:$ZERO_SHA","$ASSET":"sha256:$a9r_sha"}}}
JSON
}

# start_http <docroot> <mode> <delay>  — backgrounds a fixture server in THIS
# shell (so the EXIT trap can reap it) and sets the global PORT. mode: ok | 404.
# delay: seconds to stall before the body (0 = none).
start_http() {
  local docroot="$1" mode="$2" delay="$3"
  local port_file="$TMPDIR_BASE/port.$RANDOM.$RANDOM"
  DOCROOT="$docroot" MODE="$mode" DELAY="$delay" PORTFILE="$port_file" \
    python3 - <<'PYEOF' &
import http.server, os, time
DOCROOT = os.environ["DOCROOT"]
MODE = os.environ.get("MODE", "ok")
DELAY = float(os.environ.get("DELAY", "0"))
PORTFILE = os.environ["PORTFILE"]


class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        if MODE == "404":
            self.send_response(404)
            self.end_headers()
            return
        path = os.path.join(DOCROOT, self.path.lstrip("/"))
        if not os.path.isfile(path):
            self.send_response(404)
            self.end_headers()
            return
        data = open(path, "rb").read()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if DELAY > 0:
            time.sleep(DELAY)
        try:
            self.wfile.write(data)
        except Exception:
            pass


srv = http.server.HTTPServer(("127.0.0.1", 0), H)
with open(PORTFILE, "w") as f:
    f.write(str(srv.server_address[1]) + "\n")
srv.serve_forever()
PYEOF
  HTTP_PIDS="$HTTP_PIDS $!"
  local _
  for _ in $(seq 1 30); do
    [ -f "$port_file" ] && break
    sleep 0.1
  done
  PORT="$(tr -d '[:space:]' <"$port_file")"
}

# run_hook <skill> <url-base> [EXTRA_ENV=VAL ...]  — runs the hook fail-open,
# echoes its exit code.
run_hook() {
  local skill="$1" url="$2"
  shift 2
  local rc=0
  env ACCELERATOR_VISUALISER_SKILL_ROOT="$skill" \
    ACCELERATOR_VISUALISER_RELEASES_URL="$url" \
    ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1 \
    "$@" bash "$HOOK" >/dev/null 2>&1 || rc=$?
  printf '%s\n' "$rc"
}

# ─── 1. all-zeros sentinel → no network, exit 0, no cache ────────
echo "Test: all-zeros a9r sentinel → degrade (no network, no cache)"
S="$TMPDIR_BASE/sentinel"
make_skill "$S"
write_manifest "$S" "$PLUGIN_VERSION" "$ZERO_SHA"
RC="$(run_hook "$S" "http://127.0.0.1:1")"
assert_eq "sentinel: exit 0" "0" "$RC"
assert_file_not_exists "sentinel: no cache written" "$S/bin/$ASSET"

# ─── 2. version drift → exit 0, no cache ─────────────────────────
echo "Test: manifest version != plugin version → degrade"
S="$TMPDIR_BASE/drift"
make_skill "$S"
write_manifest "$S" "0.0.0-not-this-plugin" "$(printf 'a%.0s' $(seq 1 64))"
RC="$(run_hook "$S" "http://127.0.0.1:1")"
assert_eq "drift: exit 0" "0" "$RC"
assert_file_not_exists "drift: no cache written" "$S/bin/$ASSET"

# ─── 3. successful download → cache populated, SHA-correct ───────
echo "Test: valid release asset → downloaded, verified, cached"
S="$TMPDIR_BASE/ok"
make_skill "$S"
DOC="$TMPDIR_BASE/doc-ok"
mkdir -p "$DOC/v$PLUGIN_VERSION"
printf 'genuine a9r binary bytes\n' >"$DOC/v$PLUGIN_VERSION/$ASSET"
GOOD_SHA="$(sha256_of "$DOC/v$PLUGIN_VERSION/$ASSET")"
write_manifest "$S" "$PLUGIN_VERSION" "$GOOD_SHA"
start_http "$DOC" ok 0
RC="$(run_hook "$S" "http://127.0.0.1:$PORT")"
assert_eq "ok: exit 0" "0" "$RC"
assert_file_exists "ok: cache populated" "$S/bin/$ASSET"
assert_eq "ok: cached SHA matches manifest" "$GOOD_SHA" "$(sha256_of "$S/bin/$ASSET")"
assert_file_executable "ok: cache is executable" "$S/bin/$ASSET"

# ─── 4. fast-path: valid cache → no network ──────────────────────
echo "Test: already-valid cache → fast-path, no network hit"
S="$TMPDIR_BASE/fast"
make_skill "$S"
printf 'genuine a9r binary bytes\n' >"$S/bin/$ASSET"
chmod 0755 "$S/bin/$ASSET"
FAST_SHA="$(sha256_of "$S/bin/$ASSET")"
write_manifest "$S" "$PLUGIN_VERSION" "$FAST_SHA"
# Point at a server serving DIFFERENT bytes; a fast-path must not fetch them.
DOC="$TMPDIR_BASE/doc-fast"
mkdir -p "$DOC/v$PLUGIN_VERSION"
printf 'different bytes that would fail the SHA\n' >"$DOC/v$PLUGIN_VERSION/$ASSET"
start_http "$DOC" ok 0
RC="$(run_hook "$S" "http://127.0.0.1:$PORT")"
assert_eq "fast: exit 0" "0" "$RC"
assert_eq "fast: cache untouched (SHA unchanged)" "$FAST_SHA" "$(sha256_of "$S/bin/$ASSET")"

# ─── 5. SHA mismatch → exit 0, no partial published ──────────────
echo "Test: served bytes do not match manifest SHA → degrade, no partial"
S="$TMPDIR_BASE/shamm"
make_skill "$S"
DOC="$TMPDIR_BASE/doc-shamm"
mkdir -p "$DOC/v$PLUGIN_VERSION"
printf 'wrong bytes that will not match\n' >"$DOC/v$PLUGIN_VERSION/$ASSET"
write_manifest "$S" "$PLUGIN_VERSION" "$(printf 'b%.0s' $(seq 1 64))"
start_http "$DOC" ok 0
RC="$(run_hook "$S" "http://127.0.0.1:$PORT")"
assert_eq "shamm: exit 0" "0" "$RC"
assert_file_not_exists "shamm: no partial at cache path" "$S/bin/$ASSET"

# ─── 6. 404 → exit 0, no cache ───────────────────────────────────
echo "Test: mirror 404 → degrade, no cache"
S="$TMPDIR_BASE/e404"
make_skill "$S"
write_manifest "$S" "$PLUGIN_VERSION" "$(printf 'c%.0s' $(seq 1 64))"
start_http "$TMPDIR_BASE/doc-404" 404 0
RC="$(run_hook "$S" "http://127.0.0.1:$PORT")"
assert_eq "404: exit 0" "0" "$RC"
assert_file_not_exists "404: no cache written" "$S/bin/$ASSET"

# ─── 7. timeout (slow server, short max-time) → exit 0 in budget ─
echo "Test: hung server + short max-time → degrade within budget"
S="$TMPDIR_BASE/timeout"
make_skill "$S"
DOC="$TMPDIR_BASE/doc-timeout"
mkdir -p "$DOC/v$PLUGIN_VERSION"
printf 'slow body\n' >"$DOC/v$PLUGIN_VERSION/$ASSET"
write_manifest "$S" "$PLUGIN_VERSION" "$(sha256_of "$DOC/v$PLUGIN_VERSION/$ASSET")"
start_http "$DOC" ok 8
START="$(date +%s)"
RC="$(run_hook "$S" "http://127.0.0.1:$PORT" \
  ACCELERATOR_DOWNLOAD_MAX_TIME=1 ACCELERATOR_DOWNLOAD_CONNECT_TIMEOUT=1)"
ELAPSED=$(($(date +%s) - START))
assert_eq "timeout: exit 0" "0" "$RC"
assert_file_not_exists "timeout: no cache written" "$S/bin/$ASSET"
if [ "$ELAPSED" -le 5 ]; then
  echo "  PASS: timeout: returned within budget (${ELAPSED}s ≤ 5s, server stalled 8s)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: timeout: did not return within budget (${ELAPSED}s)"
  FAIL=$((FAIL + 1))
fi

# ─── 8. no downloader (curated PATH, neither curl nor wget) → 0 ──
echo "Test: neither curl nor wget on PATH → degrade (127), no cache"
S="$TMPDIR_BASE/nodl"
make_skill "$S"
write_manifest "$S" "$PLUGIN_VERSION" "$(printf 'd%.0s' $(seq 1 64))"
SANDBOX="$TMPDIR_BASE/sandbox-bin"
mkdir -p "$SANDBOX"
for cmd in bash sh env jq uname tr awk sha256sum shasum mktemp dirname mv rm \
  chmod sed grep cat sleep seq date head printf; do
  src="$(command -v "$cmd" 2>/dev/null || true)"
  [ -n "$src" ] && ln -sf "$src" "$SANDBOX/$cmd"
done
RC=0
env -i ACCELERATOR_VISUALISER_SKILL_ROOT="$S" \
  ACCELERATOR_VISUALISER_RELEASES_URL="http://127.0.0.1:1" \
  ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1 \
  PATH="$SANDBOX" \
  bash "$HOOK" >/dev/null 2>&1 || RC=$?
assert_eq "nodl: exit 0" "0" "$RC"
assert_file_not_exists "nodl: no cache written" "$S/bin/$ASSET"

# ─── 9. atomic publish: concurrent reader never sees a partial ───
echo "Test: slow download — cache path never appears partial (atomic publish)"
S="$TMPDIR_BASE/atomic"
make_skill "$S"
DOC="$TMPDIR_BASE/doc-atomic"
mkdir -p "$DOC/v$PLUGIN_VERSION"
printf 'atomically published a9r bytes\n' >"$DOC/v$PLUGIN_VERSION/$ASSET"
ATOMIC_SHA="$(sha256_of "$DOC/v$PLUGIN_VERSION/$ASSET")"
write_manifest "$S" "$PLUGIN_VERSION" "$ATOMIC_SHA"
start_http "$DOC" ok 2
env ACCELERATOR_VISUALISER_SKILL_ROOT="$S" \
  ACCELERATOR_VISUALISER_RELEASES_URL="http://127.0.0.1:$PORT" \
  ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1 \
  ACCELERATOR_DOWNLOAD_MAX_TIME=30 \
  bash "$HOOK" >/dev/null 2>&1 &
HOOK_PID=$!
PARTIAL_SEEN=0
for _ in $(seq 1 30); do
  if [ -e "$S/bin/$ASSET" ]; then
    # If it exists at all it must be the complete, SHA-correct file — never a
    # half-written download (those go to .acquire.* siblings, renamed in last).
    if [ "$(sha256_of "$S/bin/$ASSET" 2>/dev/null || true)" != "$ATOMIC_SHA" ]; then
      PARTIAL_SEEN=1
      break
    fi
  fi
  sleep 0.1
done
wait "$HOOK_PID" 2>/dev/null || true
assert_eq "atomic: never observed a partial cache file" "0" "$PARTIAL_SEEN"
assert_eq "atomic: final cache SHA correct" "$ATOMIC_SHA" "$(sha256_of "$S/bin/$ASSET")"
LEFTOVER="$(find "$S/bin" -name '.acquire.*' 2>/dev/null || true)"
assert_eq "atomic: no leftover .acquire temp" "" "$LEFTOVER"

test_summary
