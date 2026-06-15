#!/usr/bin/env bash
# shellcheck disable=SC2034 # LINEAR_*/LINEAR_RESOLUTION_SOURCE_* are set in the caller's scope (see below) and consumed by linear-auth-cli.sh
# Linear credential resolver. Source from scripts that need a Linear token:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/linear-auth.sh"
#
# After sourcing, call linear_resolve_credentials. On success, sets in the
# caller's scope:
#   LINEAR_TOKEN
#   LINEAR_RESOLUTION_SOURCE_TOKEN  ∈ {env, env_cmd, local, local_cmd, shared}
#
# Unlike Jira, Linear resolves a token ONLY — no site, no email. A Linear
# personal API key (lin_api_...) is user-scoped and grants access to every
# team the user belongs to.
#
# On failure, returns non-zero with E_* prefix on stderr:
#   E_NO_TOKEN (24)                  — no token found in any source
#   E_TOKEN_CMD_FAILED (25)          — token_cmd exited non-zero
#   E_TOKEN_CMD_FROM_SHARED_CONFIG   — token_cmd in config.md ignored (warning)
#   E_TOKEN_MALFORMED (27)           — token contains control chars / quotes /
#                                      backslash / newline (would corrupt the
#                                      curl --config - directive)
#   E_LOCAL_PERMS_INSECURE (29)      — config.local.md mode > 0600
#
# Token security:
#   token_cmd from config.md (shared/team config) is NEVER executed; only
#   config.local.md may supply token_cmd. This prevents shared config from
#   injecting credential-access commands.
#
#   config.local.md must have mode ≤ 0600. Looser modes cause a fail-closed
#   exit (29) unless ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND a VCS-tracked
#   .claude/insecure-local-ok marker is present.
#
#   The token is embedded inside a *quoted* curl --config - directive
#   (header = "Authorization: <token>"), so a token containing a double-quote,
#   backslash, newline, or control character could terminate or inject the
#   directive. linear_resolve_credentials rejects such tokens before use.

_LINEAR_AUTH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LINEAR_AUTH_PLUGIN_ROOT="$(cd "$_LINEAR_AUTH_SCRIPT_DIR/../../../.." && pwd)"

source "$_LINEAR_AUTH_SCRIPT_DIR/linear-common.sh"
source "$_LINEAR_AUTH_PLUGIN_ROOT/scripts/config-common.sh"

# ---------------------------------------------------------------------------
# Internal helpers

# Read a linear.<subkey> value from a single config file's frontmatter.
# Returns 0 and prints the value if found; returns 1 if absent.
_linear_read_field_from_file() {
  local file="$1"
  local subkey="$2"
  [ -f "$file" ] || return 1
  local fm
  fm=$(config_extract_frontmatter "$file") || return 1
  [ -z "$fm" ] && return 1
  printf '%s\n' "$fm" | awk -v subkey="$subkey" '
    /^linear:/ { in_section = 1; next }
    in_section && /^[^ \t]/ && /[^ \t]/ { in_section = 0 }
    in_section {
      stripped = $0
      sub(/^[ \t]+/, "", stripped)
      kprefix = subkey ":"
      if (substr(stripped, 1, length(kprefix)) == kprefix) {
        val = substr(stripped, length(kprefix) + 1)
        sub(/^[ \t]*/, "", val)
        sub(/[ \t]+$/, "", val)
        if (val ~ /^".*"$/ || val ~ /^'"'"'.*'"'"'$/) {
          val = substr(val, 2, length(val) - 2)
        }
        print val
        found = 1
        exit
      }
    }
    END { exit (found ? 0 : 1) }
  '
}

# Return the octal permission bits of a file (e.g. "600", "644").
_linear_file_mode() {
  local file="$1"
  stat -f '%Lp' "$file" 2>/dev/null || stat -c '%a' "$file" 2>/dev/null || echo ""
}

# Return 0 if mode has no group/other bits (last two digits are "00").
_linear_mode_is_secure() {
  local mode="$1"
  [ -z "$mode" ] && return 1
  local last_two="${mode: -2}"
  [ "$last_two" = "00" ]
}

# Return 0 if the given absolute path is tracked by the repo's VCS.
_linear_is_vcs_tracked() {
  local path="$1"
  local root
  root=$(find_repo_root) || return 1
  local relpath="${path#"$root"/}"
  if [ -d "$root/.jj" ]; then
    local result
    result=$(cd "$root" && jj file list "$relpath" 2>/dev/null) || return 1
    [ -n "$result" ]
  elif [ -d "$root/.git" ]; then
    git -C "$root" ls-files --error-unmatch "$relpath" >/dev/null 2>&1
  else
    return 1
  fi
}

# Trim trailing whitespace (including \r) from every line on stdin.
_linear_trim_trailing() {
  awk '{sub(/[[:space:]]+$/,""); print}'
}

# Run a token_cmd string via bash -c; capture stdout, trim, and print.
# token_cmd stderr is suppressed (avoids leaking secret names or vault paths).
# Returns 0 on success; exits with E_TOKEN_CMD_FAILED (25) on failure.
_linear_run_token_cmd() {
  local cmd="$1"
  local raw
  local rc=0
  raw=$(bash -c "$cmd" 2>/dev/null) || rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "E_TOKEN_CMD_FAILED: command exited $rc" >&2
    return 25
  fi
  printf '%s' "$raw" | _linear_trim_trailing
}

# Validate a resolved token before it can reach the curl --config - directive.
# The token sits inside a quoted config string
# (header = "Authorization: <token>"), so a value containing a double-quote,
# backslash, newline, or any control character could terminate or inject the
# directive. Reject such values fail-closed. Returns 27 on rejection.
_linear_validate_token() {
  local token="$1"
  # Control characters (incl. CR/LF/tab/NUL). Count bytes rather than grep —
  # grep splits on newlines, so an EMBEDDED newline would slip past a line-wise
  # match; `tr -dc '[:cntrl:]' | wc -c` counts every control byte directly.
  local ctrl_count
  ctrl_count=$(printf '%s' "$token" | LC_ALL=C tr -dc '[:cntrl:]' | wc -c | tr -d ' ')
  if [ "${ctrl_count:-0}" -gt 0 ]; then
    echo "E_TOKEN_MALFORMED: token contains a control character or newline" >&2
    return 27
  fi
  case "$token" in
    *'"'*)
      echo "E_TOKEN_MALFORMED: token contains a double-quote" >&2
      return 27
      ;;
    *\\*)
      echo "E_TOKEN_MALFORMED: token contains a backslash" >&2
      return 27
      ;;
  esac
  return 0
}

# ---------------------------------------------------------------------------
# Public API

# Resolve a Linear token and export it to the caller's scope.
# Sets LINEAR_TOKEN and LINEAR_RESOLUTION_SOURCE_TOKEN.
# Returns non-zero and writes E_* to stderr on failure.
linear_resolve_credentials() {
  local root
  root=$(find_repo_root) || {
    echo "E_NO_REPO: cannot locate repository root" >&2
    return 1
  }
  local team_cfg="$root/.accelerator/config.md"
  local local_cfg="$root/.accelerator/config.local.md"

  # --- token resolution (in precedence order) ---
  LINEAR_TOKEN=""
  LINEAR_RESOLUTION_SOURCE_TOKEN=""

  local _v

  # 1. ACCELERATOR_LINEAR_TOKEN env var
  if [ -n "${ACCELERATOR_LINEAR_TOKEN:-}" ]; then
    LINEAR_TOKEN="$ACCELERATOR_LINEAR_TOKEN"
    LINEAR_RESOLUTION_SOURCE_TOKEN="env"
  fi

  # 2. ACCELERATOR_LINEAR_TOKEN_CMD env var
  if [ -z "$LINEAR_TOKEN" ] && [ -n "${ACCELERATOR_LINEAR_TOKEN_CMD:-}" ]; then
    local _tok
    _tok=$(_linear_run_token_cmd "$ACCELERATOR_LINEAR_TOKEN_CMD") || return $?
    LINEAR_TOKEN="$_tok"
    LINEAR_RESOLUTION_SOURCE_TOKEN="env_cmd"
  fi

  # 3. config.local.md — permissions check, then token / token_cmd
  if [ -z "$LINEAR_TOKEN" ] && [ -f "$local_cfg" ]; then
    local _mode
    _mode=$(_linear_file_mode "$local_cfg")

    # Symlink → reject outright (can't verify the target's permissions)
    if [ -L "$local_cfg" ]; then
      echo "E_LOCAL_PERMS_INSECURE: config.local.md is a symlink; chmod 600 to allow credential read, or set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 to override (set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit .claude/insecure-local-ok to override)" >&2
      return 29
    fi

    if ! _linear_mode_is_secure "$_mode"; then
      local _insecure_ok=0
      if [ "${ACCELERATOR_ALLOW_INSECURE_LOCAL:-}" = "1" ]; then
        local _marker="$root/.claude/insecure-local-ok"
        # Marker must be a regular non-symlink file that is VCS-tracked
        if [ ! -L "$_marker" ] && [ -f "$_marker" ] && _linear_is_vcs_tracked "$_marker"; then
          _insecure_ok=1
          echo "Warning: config.local.md is mode ${_mode}; honouring ACCELERATOR_ALLOW_INSECURE_LOCAL because .claude/insecure-local-ok is present" >&2
        fi
      fi
      if [ "$_insecure_ok" -eq 0 ]; then
        echo "E_LOCAL_PERMS_INSECURE: config.local.md is mode ${_mode}; chmod 600 to allow credential read, or set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 to override (set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit .claude/insecure-local-ok to override)" >&2
        return 29
      fi
    fi

    if _v=$(_linear_read_field_from_file "$local_cfg" "token") && [ -n "$_v" ]; then
      LINEAR_TOKEN="$_v"
      LINEAR_RESOLUTION_SOURCE_TOKEN="local"
    elif _v=$(_linear_read_field_from_file "$local_cfg" "token_cmd") && [ -n "$_v" ]; then
      local _tok
      _tok=$(_linear_run_token_cmd "$_v") || return $?
      LINEAR_TOKEN="$_tok"
      LINEAR_RESOLUTION_SOURCE_TOKEN="local_cmd"
    fi
  fi

  # 4. config.md — token only, and only when config.local.md is absent.
  # (token_cmd is never honoured from shared config.)
  if [ -z "$LINEAR_TOKEN" ] && [ ! -f "$local_cfg" ]; then
    if _v=$(_linear_read_field_from_file "$team_cfg" "token_cmd") && [ -n "$_v" ]; then
      echo "E_TOKEN_CMD_FROM_SHARED_CONFIG: linear.token_cmd in config.md ignored — move to config.local.md" >&2
    fi
    local _shared_token
    _shared_token=$("$_LINEAR_AUTH_PLUGIN_ROOT/scripts/config-read-value.sh" linear.token "")
    if [ -n "$_shared_token" ]; then
      LINEAR_TOKEN="$_shared_token"
      LINEAR_RESOLUTION_SOURCE_TOKEN="shared"
    fi
  fi

  # Validation
  if [ -z "$LINEAR_TOKEN" ]; then
    echo "E_NO_TOKEN: no Linear token found; configure linear.token or linear.token_cmd in .accelerator/config.local.md" >&2
    return 24
  fi

  # Malformed-token guard — runs on the final resolved token for EVERY tier
  # (env, env_cmd, local, local_cmd, shared), after the token_cmd trim, before
  # the value can reach the curl --config - directive.
  _linear_validate_token "$LINEAR_TOKEN" || return $?
}
