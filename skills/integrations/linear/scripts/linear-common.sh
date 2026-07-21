#!/usr/bin/env bash
# Linear-domain helpers. Source this from Linear integration scripts:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/linear-common.sh"
#
# Calling convention: result on stdout, errors on stderr with stable
# E_* prefix, exit code 0 on success / non-zero on error.
#
# Stable error-code prefixes (testable contract):
#   E_NO_REPO         — repo root not locatable
#   E_BAD_JSON        — input does not parse as JSON
#   E_MISSING_DEP     — required dependency (jq >=1.6, curl, awk) absent
#   E_REFRESH_LOCKED  — linear_with_lock timed out (exit 53)
#
# State-directory resolution:
#   linear_state_dir          -> reads paths.integrations,
#                                returns <root>/.../linear/ (creates it)
#
# JSON manipulation:
#   linear_jq_field <json> <p>  -> jq -r extract; empty if missing
#   linear_atomic_write_json    -> validate JSON on stdin + atomic_write
#
# Concurrency:
#   linear_with_lock <fn>     -> mkdir-based atomic exclusive lock on
#                                linear_state_dir/.lock; stale holders
#                                detected via PID + start-time stamp;
#                                timeout exits E_REFRESH_LOCKED (53).
#                                Test seams (require ACCELERATOR_TEST_MODE=1):
#                                  LINEAR_LOCK_TIMEOUT_SECS  (default: 60)
#                                  LINEAR_LOCK_SLEEP_SECS    (default: 0.1)
#
# Dependency checks:
#   linear_require_dependencies -> assert jq (>=1.6), curl, awk on PATH
#
# Unlike Jira, Linear is Markdown-native: there is no ADF subsystem and no
# UUID generator here. WorkflowState UUIDs come from the server.

_LINEAR_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LINEAR_PLUGIN_ROOT="$(cd "$_LINEAR_SCRIPT_DIR/../../../.." && pwd)"

source "$_LINEAR_PLUGIN_ROOT/scripts/atomic-common.sh"

# Files inside .accelerator/state/integrations/linear/ that must not be
# committed: per-developer viewer identity, refresh timestamp sidecar (not
# byte-idempotent), transient lock dir. catalogue.json (team + states, which
# are team-scoped, not user-scoped) IS committed and is deliberately absent.
# Unlike Jira's array, this one is not pinned to a migration-script copy
# because no migration writes it (the linear state path is net-new);
# test-linear-paths.sh asserts these rules directly.
# shellcheck disable=SC2034 # consumed by sourcing scripts and pinned by test-linear-paths.sh
LINEAR_INNER_GITIGNORE_RULES=(
  viewer.json
  .refresh-meta.json
  .lock/
)
source "$_LINEAR_PLUGIN_ROOT/scripts/vcs-common.sh"
source "$_LINEAR_PLUGIN_ROOT/scripts/log-common.sh"
# shellcheck source=../../../../scripts/work-common.sh
source "$_LINEAR_PLUGIN_ROOT/scripts/work-common.sh"

linear_die() { log_die "$1"; }
linear_warn() { log_warn "$1"; }

# ---------------------------------------------------------------------------
# State-directory resolution

linear_state_dir() {
  local root
  root=$(find_repo_root) || {
    log_die "E_NO_REPO: cannot locate repository root"
    # shellcheck disable=SC2317 # defensive return after log_die, which exits the process; unreachable by design
    return 1
  }
  local integrations_path
  integrations_path=$(cd "$root" && "${ACCELERATOR_BIN:-$_LINEAR_PLUGIN_ROOT/bin/accelerator}" config path \
    integrations)
  local state_dir
  if [[ "$integrations_path" == /* ]]; then
    state_dir="$integrations_path/linear"
  else
    state_dir="$root/$integrations_path/linear"
  fi
  mkdir -p "$state_dir"
  printf '%s\n' "$state_dir"
}

# ---------------------------------------------------------------------------
# JSON manipulation

linear_jq_field() {
  local json="$1"
  local path="$2"
  printf '%s\n' "$json" | jq -r "$path // empty" 2>/dev/null || true
}

linear_atomic_write_json() {
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

_linear_proc_starttime() {
  local pid="$1"
  if [[ -r "/proc/$pid/stat" ]]; then
    awk '{print $22}' "/proc/$pid/stat" 2>/dev/null || echo ""
  else
    ps -o lstart= -p "$pid" 2>/dev/null | tr -d ' ' || echo ""
  fi
}

_linear_lockdir_mtime_age() {
  local lockdir="$1"
  local now
  now=$(date +%s)
  local mtime
  mtime=$(stat -f '%m' "$lockdir" 2>/dev/null) ||
    mtime=$(stat -c '%Y' "$lockdir" 2>/dev/null) || {
    echo 0
    return
  }
  echo $((now - mtime))
}

linear_with_lock() {
  local fn="$1"
  local state_dir
  state_dir=$(linear_state_dir) || return 1
  local lockdir="$state_dir/.lock"

  local timeout_secs=60
  local sleep_secs=0.1
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" ]]; then
    timeout_secs="${LINEAR_LOCK_TIMEOUT_SECS:-$timeout_secs}"
    sleep_secs="${LINEAR_LOCK_SLEEP_SECS:-$sleep_secs}"
  fi

  local deadline
  deadline=$(($(date +%s) + timeout_secs))

  while true; do
    if mkdir "$lockdir" 2>/dev/null; then
      # Lock acquired — record holder identity.
      # $BASHPID gives the current subshell's PID directly (no command substitution).
      # Using $() here would create a new subshell whose PID would immediately die.
      local my_pid="${BASHPID:-$$}"
      local my_start
      my_start=$(_linear_proc_starttime "$my_pid")
      printf '%s\n' "$my_pid" >"$lockdir/holder.pid"
      printf '%s\n' "$my_start" >"$lockdir/holder.start"
      printf '%s\n' "$(basename "${0:--}")" >"$lockdir/holder.cmd"

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
    [[ -f "$lockdir/holder.pid" ]] && holder_pid=$(cat "$lockdir/holder.pid" 2>/dev/null || true)
    [[ -f "$lockdir/holder.start" ]] && holder_start=$(cat "$lockdir/holder.start" 2>/dev/null || true)

    local holder_alive=0
    if [[ -n "$holder_pid" ]] && kill -0 "$holder_pid" 2>/dev/null; then
      if [[ -n "$holder_start" ]]; then
        local current_start
        current_start=$(_linear_proc_starttime "$holder_pid")
        if [[ "$current_start" == "$holder_start" ]]; then
          holder_alive=1
        fi
        # start-time mismatch → PID recycled → stale
      else
        # No start time recorded; fall back to lockdir age
        local age
        age=$(_linear_lockdir_mtime_age "$lockdir")
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
      [[ -f "$lockdir/holder.cmd" ]] &&
        holder_cmd=$(cat "$lockdir/holder.cmd" 2>/dev/null || echo "unknown")
      echo "E_REFRESH_LOCKED: lock held by ${holder_cmd} (pid ${holder_pid:-?}) for >${timeout_secs}s" >&2
      return 53
    fi

    sleep "$sleep_secs"
  done
}

# ---------------------------------------------------------------------------
# Dependency checks

linear_require_dependencies() {
  local missing=()
  command -v jq >/dev/null 2>&1 || missing+=("jq")
  command -v curl >/dev/null 2>&1 || missing+=("curl")
  command -v awk >/dev/null 2>&1 || missing+=("awk")

  if [[ ${#missing[@]} -gt 0 ]]; then
    log_die "E_MISSING_DEP: required dependencies not found: ${missing[*]}"
    # shellcheck disable=SC2317 # defensive return after log_die, which exits the process; unreachable by design
    return 1
  fi

  local jq_ver
  jq_ver=$(jq --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+' | head -1 || echo "")
  if [[ -n "$jq_ver" ]]; then
    local major="${jq_ver%%.*}"
    local minor="${jq_ver#*.}"
    if [[ "$major" -lt 1 ]] || { [[ "$major" -eq 1 ]] && [[ "$minor" -lt 6 ]]; }; then
      log_die "E_MISSING_DEP: jq >= 1.6 required, found $jq_ver"
      # shellcheck disable=SC2317 # defensive return after log_die, which exits the process; unreachable by design
      return 1
    fi
  fi
}

# ---------------------------------------------------------------------------
# Generic request-error hint emitter

# Emit a generic Hint: line for a propagated linear-graphql.sh exit code.
# Returns 0 if a generic hint was emitted, 1 if the code is flow-specific
# (caller should emit its own message).
_linear_emit_generic_hint() {
  local code="$1"
  case "$code" in
    11 | 22) printf 'Hint: check credentials with /init-linear.\n' >&2 ;;
    20) printf 'Hint: Linear returned a server error; check the Linear status page.\n' >&2 ;;
    21) printf 'Hint: connection failed; check network connectivity to api.linear.app.\n' >&2 ;;
    34) printf 'Hint: the query or mutation was rejected; check the error above.\n' >&2 ;;
    35) printf 'Hint: rate-limited by Linear; wait briefly and retry.\n' >&2 ;;
    36) printf 'Hint: query exceeded the 10,000-point complexity cap; request fewer fields or a smaller page.\n' >&2 ;;
    *) return 1 ;;
  esac
  return 0
}
