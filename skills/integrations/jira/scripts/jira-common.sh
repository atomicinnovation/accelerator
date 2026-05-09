#!/usr/bin/env bash
# Jira-domain helpers. Source this from Jira integration scripts:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/jira-common.sh"
#
# Calling convention: result on stdout, errors on stderr with stable
# E_* prefix, exit code 0 on success / non-zero on error.
#
# Stable error-code prefixes (testable contract):
#   E_NO_REPO         — repo root not locatable
#   E_BAD_JSON        — input does not parse as JSON
#   E_MISSING_DEP     — required dependency (jq >=1.6, curl, awk) absent
#   E_REFRESH_LOCKED  — jira_with_lock timed out (exit 53)
#
# State-directory resolution:
#   jira_state_dir            -> reads paths.integrations,
#                                returns <root>/.../jira/ (creates it)
#
# JSON manipulation:
#   jira_jq_field <json> <p>  -> jq -r extract; empty if missing
#   jira_atomic_write_json    -> validate JSON on stdin + atomic_write
#
# Concurrency:
#   jira_with_lock <fn>       -> mkdir-based atomic exclusive lock on
#                                jira_state_dir/.lock; stale holders
#                                detected via PID + start-time stamp;
#                                timeout exits E_REFRESH_LOCKED (53).
#                                Test seams (require ACCELERATOR_TEST_MODE=1):
#                                  JIRA_LOCK_TIMEOUT_SECS  (default: 60)
#                                  JIRA_LOCK_SLEEP_SECS    (default: 0.1)
#
# Dependency checks:
#   jira_require_dependencies -> assert jq (>=1.6), curl, awk on PATH
#
# UUID generation:
#   _jira_uuid_v4             -> portable UUID v4 (uuidgen -> od+awk);
#                                honours JIRA_ADF_LOCALID_SEED when
#                                ACCELERATOR_TEST_MODE=1

_JIRA_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_JIRA_PLUGIN_ROOT="$(cd "$_JIRA_SCRIPT_DIR/../../../.." && pwd)"

source "$_JIRA_PLUGIN_ROOT/scripts/atomic-common.sh"

# Files inside .accelerator/state/integrations/jira/ that must not be
# committed: per-developer site identity, refresh timestamp sidecar (not
# byte-idempotent), transient lock dir.
# Must stay byte-equal to JIRA_INNER_GITIGNORE_RULES in
# 0003-relocate-accelerator-state.sh. Both copies are pinned to equality by a
# test in test-jira-paths.sh (Phase 6 / 4b).
JIRA_INNER_GITIGNORE_RULES=(
  site.json
  .refresh-meta.json
  .lock/
)
source "$_JIRA_PLUGIN_ROOT/scripts/vcs-common.sh"
source "$_JIRA_PLUGIN_ROOT/scripts/log-common.sh"
# shellcheck source=../../../../scripts/work-common.sh
source "$_JIRA_PLUGIN_ROOT/scripts/work-common.sh"

jira_die()  { log_die  "$1"; }
jira_warn() { log_warn "$1"; }

# ---------------------------------------------------------------------------
# State-directory resolution

jira_state_dir() {
  local root
  root=$(find_repo_root) || {
    log_die "E_NO_REPO: cannot locate repository root"
    return 1
  }
  local integrations_path
  integrations_path=$(cd "$root" && "$_JIRA_PLUGIN_ROOT/scripts/config-read-path.sh" \
    integrations)
  local state_dir
  if [[ "$integrations_path" == /* ]]; then
    state_dir="$integrations_path/jira"
  else
    state_dir="$root/$integrations_path/jira"
  fi
  mkdir -p "$state_dir"
  printf '%s\n' "$state_dir"
}

# ---------------------------------------------------------------------------
# JSON manipulation

jira_jq_field() {
  local json="$1"
  local path="$2"
  printf '%s\n' "$json" | jq -r "$path // empty" 2>/dev/null || true
}

jira_atomic_write_json() {
  local target="$1"
  local json
  json=$(cat)
  if ! printf '%s\n' "$json" | jq empty 2>/dev/null; then
    echo "E_BAD_JSON: input does not parse as JSON" >&2
    return 1
  fi
  local dir
  dir="$(dirname "$target")"
  if ! { mkdir -p "$dir" 2>/dev/null && [ -w "$dir" ]; }; then
    echo "E_WRITE_FAILED: directory not writable: $dir" >&2
    return 1
  fi
  printf '%s\n' "$json" | atomic_write "$target"
}

# ---------------------------------------------------------------------------
# Concurrency — mkdir-based lock

_jira_proc_starttime() {
  local pid="$1"
  if [[ -r "/proc/$pid/stat" ]]; then
    awk '{print $22}' "/proc/$pid/stat" 2>/dev/null || echo ""
  else
    ps -o lstart= -p "$pid" 2>/dev/null | tr -d ' ' || echo ""
  fi
}

_jira_lockdir_mtime_age() {
  local lockdir="$1"
  local now
  now=$(date +%s)
  local mtime
  mtime=$(stat -f '%m' "$lockdir" 2>/dev/null) || \
  mtime=$(stat -c '%Y' "$lockdir" 2>/dev/null) || {
    echo 0; return
  }
  echo $(( now - mtime ))
}

jira_with_lock() {
  local fn="$1"
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local lockdir="$state_dir/.lock"

  local timeout_secs=60
  local sleep_secs=0.1
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" ]]; then
    timeout_secs="${JIRA_LOCK_TIMEOUT_SECS:-$timeout_secs}"
    sleep_secs="${JIRA_LOCK_SLEEP_SECS:-$sleep_secs}"
  fi

  local deadline
  deadline=$(( $(date +%s) + timeout_secs ))

  while true; do
    if mkdir "$lockdir" 2>/dev/null; then
      # Lock acquired — record holder identity
      # $BASHPID gives the current subshell's PID directly (no command substitution).
      # Using $() here would create a new subshell whose PID would immediately die.
      local my_pid="${BASHPID:-$$}"
      local my_start
      my_start=$(_jira_proc_starttime "$my_pid")
      printf '%s\n' "$my_pid"               > "$lockdir/holder.pid"
      printf '%s\n' "$my_start"             > "$lockdir/holder.start"
      printf '%s\n' "$(basename "${0:--}")" > "$lockdir/holder.cmd"

      # Release on exit (including SIGTERM; not SIGKILL — stale recovery handles that)
      trap 'rm -rf "'"$lockdir"'"' EXIT
      local _rc=0
      "$fn" || _rc=$?
      rm -rf "$lockdir"
      trap - EXIT
      return "$_rc"
    fi

    # Could not acquire — determine if holder is alive
    local holder_pid="" holder_start=""
    [[ -f "$lockdir/holder.pid" ]]   && holder_pid=$(cat   "$lockdir/holder.pid"   2>/dev/null || true)
    [[ -f "$lockdir/holder.start" ]] && holder_start=$(cat "$lockdir/holder.start" 2>/dev/null || true)

    local holder_alive=0
    if [[ -n "$holder_pid" ]] && kill -0 "$holder_pid" 2>/dev/null; then
      if [[ -n "$holder_start" ]]; then
        local current_start
        current_start=$(_jira_proc_starttime "$holder_pid")
        if [[ "$current_start" == "$holder_start" ]]; then
          holder_alive=1
        fi
        # start-time mismatch → PID recycled → stale
      else
        # No start time recorded; fall back to lockdir age
        local age
        age=$(_jira_lockdir_mtime_age "$lockdir")
        if [[ "$age" -lt "$timeout_secs" ]]; then
          holder_alive=1
        fi
      fi
    fi
    # kill -0 failure → PID dead → stale

    if [[ "$holder_alive" -eq 0 ]]; then
      # Atomically reclaim: mv is the linearisation point
      if mv "$lockdir" "${lockdir}.stale.$$" 2>/dev/null; then
        rm -rf "${lockdir}.stale.$$"
      fi
      continue
    fi

    # Holder is alive — check timeout
    if [[ "$(date +%s)" -ge "$deadline" ]]; then
      local holder_cmd="unknown"
      [[ -f "$lockdir/holder.cmd" ]] && \
        holder_cmd=$(cat "$lockdir/holder.cmd" 2>/dev/null || echo "unknown")
      echo "E_REFRESH_LOCKED: lock held by ${holder_cmd} (pid ${holder_pid:-?}) for >${timeout_secs}s" >&2
      return 53
    fi

    sleep "$sleep_secs"
  done
}

# ---------------------------------------------------------------------------
# Dependency checks

jira_require_dependencies() {
  local missing=()
  command -v jq   >/dev/null 2>&1 || missing+=("jq")
  command -v curl >/dev/null 2>&1 || missing+=("curl")
  command -v awk  >/dev/null 2>&1 || missing+=("awk")

  if [[ ${#missing[@]} -gt 0 ]]; then
    log_die "E_MISSING_DEP: required dependencies not found: ${missing[*]}"
    return 1
  fi

  local jq_ver
  jq_ver=$(jq --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+' | head -1 || echo "")
  if [[ -n "$jq_ver" ]]; then
    local major="${jq_ver%%.*}"
    local minor="${jq_ver#*.}"
    if [[ "$major" -lt 1 ]] || { [[ "$major" -eq 1 ]] && [[ "$minor" -lt 6 ]]; }; then
      log_die "E_MISSING_DEP: jq >= 1.6 required, found $jq_ver"
      return 1
    fi
  fi
}

# ---------------------------------------------------------------------------
# UUID generation

_jira_uuid_v4() {
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" ]] && [[ -n "${JIRA_ADF_LOCALID_SEED:-}" ]]; then
    printf '%s' "$JIRA_ADF_LOCALID_SEED" | md5sum 2>/dev/null | awk '{
      h = $1
      printf "%s-%s-4%s-%s-%s\n",
        substr(h,1,8), substr(h,9,4), substr(h,14,3),
        substr(h,17,4), substr(h,21,12)
    }' || printf '00000000-0000-4000-8000-000000000001\n'
    return
  fi
  if command -v uuidgen >/dev/null 2>&1; then
    uuidgen | tr '[:upper:]' '[:lower:]'
  else
    od -An -N16 -tx1 /dev/urandom 2>/dev/null | tr -d ' \n' | \
      awk '{
        h = $0
        v = strtonum("0x" substr(h, 17, 2))
        vb = sprintf("%02x", (v % 64) + 128)
        printf "%s-%s-4%s-%s%s-%s\n",
          substr(h,1,8), substr(h,9,4), substr(h,14,3),
          vb, substr(h,19,2), substr(h,21,12)
      }'
  fi
}

# ---------------------------------------------------------------------------
# Generic request-error hint emitter

# Emit a generic Hint: line for a propagated jira-request.sh exit code.
# Returns 0 if a generic hint was emitted, 1 if the code is flow-specific
# (caller should emit its own message).
_jira_emit_generic_hint() {
  local code="$1"
  case "$code" in
    11|12|22) printf 'Hint: check credentials with /init-jira.\n' >&2 ;;
    19)       printf 'Hint: rate-limited by Jira; wait briefly and retry.\n' >&2 ;;
    20)       printf 'Hint: Jira returned a server error; check the Jira status page.\n' >&2 ;;
    21)       printf 'Hint: connection failed; check network and ACCELERATOR_JIRA_BASE_URL.\n' >&2 ;;
    34)       printf 'Hint: check the field error above; run /init-jira --refresh-fields if a custom field id was rejected.\n' >&2 ;;
    *)        return 1 ;;
  esac
  return 0
}
