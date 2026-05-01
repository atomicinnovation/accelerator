#!/usr/bin/env bash
# Jira credential resolver. Source from scripts that need Jira credentials:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/jira-auth.sh"
#
# After sourcing, call jira_resolve_credentials. On success, sets in the
# caller's scope:
#   JIRA_SITE, JIRA_EMAIL, JIRA_TOKEN
#   JIRA_RESOLUTION_SOURCE_TOKEN  ∈ {env, env_cmd, local, local_cmd, shared}
#   JIRA_RESOLUTION_SOURCE_SITE   ∈ {shared, local}
#   JIRA_RESOLUTION_SOURCE_EMAIL  ∈ {shared, local}
#
# On failure, returns non-zero with E_* prefix on stderr:
#   E_NO_TOKEN (24)                  — no token found in any source
#   E_TOKEN_CMD_FAILED (25)          — token_cmd exited non-zero
#   E_TOKEN_CMD_FROM_SHARED_CONFIG   — token_cmd in accelerator.md ignored
#   E_AUTH_NO_SITE (27)              — jira.site not configured
#   E_AUTH_NO_EMAIL (28)             — jira.email not configured
#   E_LOCAL_PERMS_INSECURE (29)      — accelerator.local.md mode > 0600
#
# Token security:
#   token_cmd from accelerator.md (shared/team config) is NEVER executed;
#   only accelerator.local.md may supply token_cmd. This prevents shared
#   config from injecting credential-access commands.
#
#   accelerator.local.md must have mode ≤ 0600. Looser modes cause a
#   fail-closed exit (29) unless ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND
#   a VCS-tracked .claude/insecure-local-ok marker is present.
#
# --debug is NOT forwarded to downstream curl; the CLI wrapper handles this.

_JIRA_AUTH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_JIRA_AUTH_PLUGIN_ROOT="$(cd "$_JIRA_AUTH_SCRIPT_DIR/../../../.." && pwd)"

source "$_JIRA_AUTH_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_AUTH_PLUGIN_ROOT/scripts/config-common.sh"

# ---------------------------------------------------------------------------
# Internal helpers

# Read a jira.<subkey> value from a single config file's frontmatter.
# Returns 0 and prints the value if found; returns 1 if absent.
_jira_read_field_from_file() {
  local file="$1"
  local subkey="$2"
  [ -f "$file" ] || return 1
  local fm
  fm=$(config_extract_frontmatter "$file") || return 1
  [ -z "$fm" ] && return 1
  printf '%s\n' "$fm" | awk -v subkey="$subkey" '
    /^jira:/ { in_section = 1; next }
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
_jira_file_mode() {
  local file="$1"
  stat -f '%Lp' "$file" 2>/dev/null || stat -c '%a' "$file" 2>/dev/null || echo ""
}

# Return 0 if mode has no group/other bits (last two digits are "00").
_jira_mode_is_secure() {
  local mode="$1"
  [ -z "$mode" ] && return 1
  local last_two="${mode: -2}"
  [ "$last_two" = "00" ]
}

# Return 0 if the given absolute path is tracked by the repo's VCS.
_jira_is_vcs_tracked() {
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
_jira_trim_trailing() {
  awk '{sub(/[[:space:]]+$/,""); print}'
}

# Run a token_cmd string via bash -c; capture stdout, trim, and print.
# token_cmd stderr is suppressed (avoids leaking secret names or vault paths).
# Returns 0 on success; exits with E_TOKEN_CMD_FAILED (25) on failure.
_jira_run_token_cmd() {
  local cmd="$1"
  local raw
  local rc=0
  raw=$(bash -c "$cmd" 2>/dev/null) || rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "E_TOKEN_CMD_FAILED: command exited $rc" >&2
    return 25
  fi
  printf '%s' "$raw" | _jira_trim_trailing
}

# ---------------------------------------------------------------------------
# Public API

# Resolve Jira credentials and export them to the caller's scope.
# Sets JIRA_SITE, JIRA_EMAIL, JIRA_TOKEN and JIRA_RESOLUTION_SOURCE_*.
# Returns non-zero and writes E_* to stderr on failure.
jira_resolve_credentials() {
  local root
  root=$(find_repo_root) || {
    echo "E_NO_REPO: cannot locate repository root" >&2
    return 1
  }
  local team_cfg="$root/.claude/accelerator.md"
  local local_cfg="$root/.claude/accelerator.local.md"

  # --- site and email (config files only; no env-var override) ---
  JIRA_SITE=""
  JIRA_EMAIL=""
  JIRA_RESOLUTION_SOURCE_SITE="shared"
  JIRA_RESOLUTION_SOURCE_EMAIL="shared"

  local _v
  if _v=$(_jira_read_field_from_file "$team_cfg" "site") && [ -n "$_v" ]; then
    JIRA_SITE="$_v"
    JIRA_RESOLUTION_SOURCE_SITE="shared"
  fi
  if _v=$(_jira_read_field_from_file "$local_cfg" "site") && [ -n "$_v" ]; then
    JIRA_SITE="$_v"
    JIRA_RESOLUTION_SOURCE_SITE="local"
  fi
  if _v=$(_jira_read_field_from_file "$team_cfg" "email") && [ -n "$_v" ]; then
    JIRA_EMAIL="$_v"
    JIRA_RESOLUTION_SOURCE_EMAIL="shared"
  fi
  if _v=$(_jira_read_field_from_file "$local_cfg" "email") && [ -n "$_v" ]; then
    JIRA_EMAIL="$_v"
    JIRA_RESOLUTION_SOURCE_EMAIL="local"
  fi

  # --- token resolution (in precedence order) ---
  JIRA_TOKEN=""
  JIRA_RESOLUTION_SOURCE_TOKEN=""

  # 1. ACCELERATOR_JIRA_TOKEN env var
  if [ -n "${ACCELERATOR_JIRA_TOKEN:-}" ]; then
    JIRA_TOKEN="$ACCELERATOR_JIRA_TOKEN"
    JIRA_RESOLUTION_SOURCE_TOKEN="env"
  fi

  # 2. ACCELERATOR_JIRA_TOKEN_CMD env var
  if [ -z "$JIRA_TOKEN" ] && [ -n "${ACCELERATOR_JIRA_TOKEN_CMD:-}" ]; then
    local _tok
    _tok=$(_jira_run_token_cmd "$ACCELERATOR_JIRA_TOKEN_CMD") || return $?
    JIRA_TOKEN="$_tok"
    JIRA_RESOLUTION_SOURCE_TOKEN="env_cmd"
  fi

  # 3. accelerator.local.md — permissions check, then token / token_cmd
  if [ -z "$JIRA_TOKEN" ] && [ -f "$local_cfg" ]; then
    local _mode
    _mode=$(_jira_file_mode "$local_cfg")

    # Symlink → reject outright (can't verify the target's permissions)
    if [ -L "$local_cfg" ]; then
      echo "E_LOCAL_PERMS_INSECURE: accelerator.local.md is a symlink; chmod 600 to allow credential read, or set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 to override (set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit .claude/insecure-local-ok to override)" >&2
      return 29
    fi

    if ! _jira_mode_is_secure "$_mode"; then
      local _insecure_ok=0
      if [ "${ACCELERATOR_ALLOW_INSECURE_LOCAL:-}" = "1" ]; then
        local _marker="$root/.claude/insecure-local-ok"
        # Marker must be a regular non-symlink file that is VCS-tracked
        if [ ! -L "$_marker" ] && [ -f "$_marker" ] && _jira_is_vcs_tracked "$_marker"; then
          _insecure_ok=1
          echo "Warning: accelerator.local.md is mode ${_mode}; honouring ACCELERATOR_ALLOW_INSECURE_LOCAL because .claude/insecure-local-ok is present" >&2
        fi
      fi
      if [ "$_insecure_ok" -eq 0 ]; then
        echo "E_LOCAL_PERMS_INSECURE: accelerator.local.md is mode ${_mode}; chmod 600 to allow credential read, or set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 to override (set ACCELERATOR_ALLOW_INSECURE_LOCAL=1 AND commit .claude/insecure-local-ok to override)" >&2
        return 29
      fi
    fi

    if _v=$(_jira_read_field_from_file "$local_cfg" "token") && [ -n "$_v" ]; then
      JIRA_TOKEN="$_v"
      JIRA_RESOLUTION_SOURCE_TOKEN="local"
    elif _v=$(_jira_read_field_from_file "$local_cfg" "token_cmd") && [ -n "$_v" ]; then
      local _tok
      _tok=$(_jira_run_token_cmd "$_v") || return $?
      JIRA_TOKEN="$_tok"
      JIRA_RESOLUTION_SOURCE_TOKEN="local_cmd"
    fi
  fi

  # 4. accelerator.md — token only, and only when accelerator.local.md is absent.
  # (token_cmd is never honoured from shared config.)
  if [ -z "$JIRA_TOKEN" ] && [ ! -f "$local_cfg" ]; then
    if _v=$(_jira_read_field_from_file "$team_cfg" "token_cmd") && [ -n "$_v" ]; then
      echo "E_TOKEN_CMD_FROM_SHARED_CONFIG: jira.token_cmd in accelerator.md ignored — move to accelerator.local.md" >&2
    fi
    local _shared_token
    _shared_token=$("$_JIRA_AUTH_PLUGIN_ROOT/scripts/config-read-value.sh" jira.token "")
    if [ -n "$_shared_token" ]; then
      JIRA_TOKEN="$_shared_token"
      JIRA_RESOLUTION_SOURCE_TOKEN="shared"
    fi
  fi

  # Validation
  if [ -z "$JIRA_TOKEN" ]; then
    echo "E_NO_TOKEN: no Jira token found; configure jira.token or jira.token_cmd in .claude/accelerator.local.md" >&2
    return 24
  fi

  if [ -z "$JIRA_SITE" ]; then
    echo "E_AUTH_NO_SITE: jira.site not configured in .claude/accelerator.md" >&2
    return 27
  fi

  if [ -z "$JIRA_EMAIL" ]; then
    echo "E_AUTH_NO_EMAIL: jira.email not configured in .claude/accelerator.md" >&2
    return 28
  fi
}
