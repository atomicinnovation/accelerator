#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_ROOT="${ACCELERATOR_VISUALISER_SKILL_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
source "$SCRIPT_DIR/launcher-helpers.sh"

# ─── top-level pipeline ──────────────────────────────────────

PROJECT_ROOT="$(find_repo_root)"
cd "$PROJECT_ROOT"

TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp meta/tmp)"
TMP_DIR="$PROJECT_ROOT/$TMP_REL/visualiser"

INFO="$TMP_DIR/server-info.json"
PID_FILE="$TMP_DIR/server.pid"
LOG_FILE="$TMP_DIR/server.log"
CFG="$TMP_DIR/config.json"
STOPPED="$TMP_DIR/server-stopped.json"
LOCK="$TMP_DIR/launcher.lock"

# Reuse short-circuit with (pid, start_time) identity cross-check.
# Runs before the sentinel check so an already-running server is not
# killed by a transient sentinel deletion.
if [ -d "$TMP_DIR" ] && [ -f "$INFO" ] && [ -f "$PID_FILE" ]; then
  EXISTING_PID="$(tr -cd '0-9' < "$PID_FILE")"
  EXPECTED_START="$(jq -r '.start_time // empty' "$INFO" 2>/dev/null || true)"
  if [ -n "$EXISTING_PID" ] && kill -0 "$EXISTING_PID" 2>/dev/null; then
    if [ -z "$EXPECTED_START" ] || [ "$(start_time_of "$EXISTING_PID" 2>/dev/null || echo '')" = "$EXPECTED_START" ]; then
      URL="$(jq -r '.url // empty' "$INFO" 2>/dev/null || true)"
      if [[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/?$ ]]; then
        echo "**Visualiser URL**: $URL"
        exit 0
      fi
    fi
  fi
  rm -f "$INFO" "$PID_FILE"
fi

# Init sentinel: reject launches in projects that haven't run /accelerator:init.
SENTINEL="$PROJECT_ROOT/$TMP_REL/.gitignore"
if [ ! -f "$SENTINEL" ]; then
  die_json "$(jq -nc \
    --arg error 'accelerator not initialised' \
    --arg hint "run /accelerator:init in $PROJECT_ROOT before launching the visualiser" \
    --arg root "$PROJECT_ROOT" \
    '{error:$error,hint:$hint,project_root:$root}')"
fi

mkdir -p "$TMP_DIR"
chmod 0700 "$TMP_DIR" 2>/dev/null || true

# Serialise concurrent invocations.
if command -v flock >/dev/null 2>&1; then
  exec 9>"$LOCK"
  if ! flock -n 9; then
    die_json "$(jq -nc --arg error 'another launcher is running' \
      --arg hint "wait for it to finish, or check $TMP_DIR for a stale lock" \
      '{error:$error,hint:$hint}')"
  fi
else
  if ! mkdir "$LOCK.d" 2>/dev/null; then
    die_json "$(jq -nc --arg error 'another launcher is running' \
      --arg hint "rm -rf $LOCK.d if it's stale" \
      '{error:$error,hint:$hint}')"
  fi
  trap 'rmdir "$LOCK.d" 2>/dev/null || true' EXIT
fi

rm -f "$STOPPED"

# Platform detection.
OS_RAW="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_RAW="$(uname -m)"
case "$OS_RAW" in
  darwin|linux) OS="$OS_RAW" ;;
  *) die_json "$(jq -nc --arg error 'unsupported platform' --arg os "$OS_RAW" \
       '{error:$error,os:$os}')" ;;
esac
case "$ARCH_RAW" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64) ARCH="x64" ;;
  *) die_json "$(jq -nc --arg error 'unsupported architecture' --arg arch "$ARCH_RAW" \
       '{error:$error,arch:$arch}')" ;;
esac

if ! command -v jq >/dev/null 2>&1; then
  die_json '{"error":"jq is required but not found","hint":"brew install jq / apt install jq / apk add jq"}'
fi

PLUGIN_VERSION="$(jq -r .version "$PLUGIN_ROOT/.claude-plugin/plugin.json")"
MANIFEST="$SKILL_ROOT/bin/checksums.json"
BIN_CACHE="$SKILL_ROOT/bin/accelerator-visualiser-${OS}-${ARCH}"
RELEASES_URL_BASE="${ACCELERATOR_VISUALISER_RELEASES_URL:-https://github.com/atomicinnovation/accelerator/releases/download}"

# ─── tri-precedence binary resolution ────────────────────────

BIN=""
if [ -n "${ACCELERATOR_VISUALISER_BIN:-}" ]; then
  BIN="$ACCELERATOR_VISUALISER_BIN"
else
  CONFIG_BIN="$("$PLUGIN_ROOT/scripts/config-read-value.sh" visualiser.binary 2>/dev/null || true)"
  if [ -n "$CONFIG_BIN" ]; then
    case "$CONFIG_BIN" in
      /*) ;;
      *) CONFIG_BIN="$PROJECT_ROOT/$CONFIG_BIN" ;;
    esac
    if [ ! -x "$CONFIG_BIN" ]; then
      die_json "$(jq -nc --arg error 'configured visualiser.binary is not executable' \
        --arg path "$CONFIG_BIN" '{error:$error,path:$path}')"
    fi
    BIN="$CONFIG_BIN"
  fi
fi

if [ -z "$BIN" ]; then
  EXPECTED_SHA_RAW="$(jq -r ".binaries[\"${OS}-${ARCH}\"] // empty" "$MANIFEST")"
  EXPECTED_SHA="${EXPECTED_SHA_RAW#sha256:}"
  if [ "$EXPECTED_SHA" = "0000000000000000000000000000000000000000000000000000000000000000" ]; then
    die_json "$(jq -nc \
      --arg error 'no released binary for this plugin version' \
      --arg version "$PLUGIN_VERSION" \
      --arg hint 'set ACCELERATOR_VISUALISER_BIN=<path> (one-shot) or add `visualiser:\n  binary: <path>` to .claude/accelerator.local.md (persistent)' \
      '{error:$error,plugin_version:$version,hint:$hint}')"
  fi
  MANIFEST_VERSION="$(jq -r '.version // empty' "$MANIFEST")"
  if [ -n "$MANIFEST_VERSION" ] && [ "$MANIFEST_VERSION" != "$PLUGIN_VERSION" ]; then
    die_json "$(jq -nc \
      --arg error 'checksum manifest version drift' \
      --arg plugin "$PLUGIN_VERSION" --arg manifest "$MANIFEST_VERSION" \
      '{error:$error,plugin_version:$plugin,manifest_version:$manifest}')"
  fi
  if [ -x "$BIN_CACHE" ] && [ ! -L "$BIN_CACHE" ]; then
    ACTUAL_SHA="$(sha256_of "$BIN_CACHE")"
  else
    [ -L "$BIN_CACHE" ] && rm -f "$BIN_CACHE"
    ACTUAL_SHA=""
  fi
  if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
    echo "Downloading visualiser server (first run, ~8 MB)…"
    ASSET_URL="${RELEASES_URL_BASE}/v${PLUGIN_VERSION}/accelerator-visualiser-${OS}-${ARCH}"
    TMP_PART="$(mktemp "$SKILL_ROOT/bin/accelerator-visualiser.XXXXXX")"
    if ! download_to "$ASSET_URL" "$TMP_PART"; then
      rm -f "$TMP_PART"
      die_json "$(jq -nc --arg error 'download failed' --arg url "$ASSET_URL" \
        --arg hint 'set ACCELERATOR_VISUALISER_BIN=<path> or ACCELERATOR_VISUALISER_RELEASES_URL for a mirror' \
        '{error:$error,url:$url,hint:$hint}')"
    fi
    DOWNLOADED_SHA="$(sha256_of "$TMP_PART")"
    if [ "$DOWNLOADED_SHA" != "$EXPECTED_SHA" ]; then
      rm -f "$TMP_PART"
      die_json "$(jq -nc --arg error 'checksum mismatch' \
        --arg expected "$EXPECTED_SHA" --arg actual "$DOWNLOADED_SHA" \
        '{error:$error,expected:$expected,actual:$actual}')"
    fi
    install -m 0755 "$TMP_PART" "$BIN_CACHE"
    rm -f "$TMP_PART"
  fi
  BIN="$BIN_CACHE"
fi

# ─── owner PID + start_time for the lifecycle handshake ──────

OWNER_PID="$(ppid_of "$PPID" 2>/dev/null || echo '')"
if [ -z "$OWNER_PID" ] || [ "$OWNER_PID" = "1" ]; then OWNER_PID="$PPID"; fi
if [ "$OWNER_PID" = "1" ]; then OWNER_PID=0; fi

OWNER_START_TIME=""
if [ "$OWNER_PID" -gt 0 ]; then
  OWNER_START_TIME="$(start_time_of "$OWNER_PID" 2>/dev/null || echo '')"
fi

# ─── write config.json ───────────────────────────────────────

CONFIG_ARGS=(
  --plugin-version "$PLUGIN_VERSION"
  --project-root "$PROJECT_ROOT"
  --tmp-dir "$TMP_DIR"
  --log-file "$LOG_FILE"
  --owner-pid "$OWNER_PID"
)
if [ -n "$OWNER_START_TIME" ]; then
  CONFIG_ARGS+=(--owner-start-time "$OWNER_START_TIME")
fi
"$SCRIPT_DIR/write-visualiser-config.sh" "${CONFIG_ARGS[@]}" > "$CFG"

# ─── background launch; server writes its own pid file ───────

BOOTSTRAP_LOG="$TMP_DIR/server.bootstrap.log"
: > "$BOOTSTRAP_LOG"
chmod 0600 "$BOOTSTRAP_LOG"
nohup "$BIN" --config "$CFG" >> "$BOOTSTRAP_LOG" 2>&1 &
SERVER_PID=$!
disown "$SERVER_PID" 2>/dev/null || true

for _ in $(seq 1 50); do
  [ -f "$INFO" ] && [ -f "$PID_FILE" ] && break
  sleep 0.1
done
if [ ! -f "$INFO" ]; then
  die_json "$(jq -nc --arg error 'server-info.json did not appear within 5s' \
    --arg log "$LOG_FILE" '{error:$error,log:$log}')"
fi

URL="$(jq -r '.url // empty' "$INFO")"
if ! [[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/?$ ]]; then
  die_json "$(jq -nc --arg error 'server-info.json contained an invalid url' \
    --arg url "$URL" '{error:$error,url:$url}')"
fi
echo "**Visualiser URL**: $URL"
