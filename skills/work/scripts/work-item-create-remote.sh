#!/usr/bin/env bash
set -euo pipefail

# work-item-create-remote.sh — the single sanctioned work → integrations bridge.
#
# Routes a create request to the configured remote tracker's create-and-return
# primitive and returns a UNIFORM contract so /create-work-item never branches on
# tracker-specific output or codes.
#
# Usage:
#   work-item-create-remote.sh --integration <sys> --title TEXT --body-file PATH [--kind KIND]
#   work-item-create-remote.sh --integration <sys> [--kind KIND] --dry-run
#
# --dry-run resolves and previews the tracker's user-visible target fields
# WITHOUT creating anything, so /create-work-item can show an informed push
# offer without reaching past the work → integrations boundary itself (it only
# ever calls this dispatcher, never an integration script directly). It prints a
# tab-separated preview line and, for Jira, surfaces an unresolvable project as a
# pre-create failure (70) BEFORE the confirm gate. Output, by tracker:
#   jira\t<issue_type>\t<issue_type_source>\t<project>\t<project_source>
#   linear\t(no user-resolvable type/project — team fixed by /init-linear)
#
# <sys> is the active tracker. The caller (/create-work-item) MUST source it from
# the same `config work integration` read used to gate the push, so the
# gate and the route cannot diverge; this script does not re-derive it.
#
# Contract:
#   - stdout (on success): EXACTLY the bare validated identifier for every
#     tracker. No JSON, no per-tracker response parsing here — Linear emits the
#     bare identifier already, and the Jira `.key` extraction is pushed down to
#     jira-emit-key.sh. Identifier FORMAT validation is per-tracker (each
#     integration validates its own native shape); this dispatcher applies only a
#     tracker-agnostic safety check before passing the value through.
#   - exit-code taxonomy (this script's namespace; see EXIT_CODES.md):
#       0   success — identifier on stdout
#       70  E_DISPATCH_RETRYABLE      failure provably BEFORE the remote mutation
#                                     (arg/validation/auth/connect-refused) — safe to retry
#       71  E_DISPATCH_TERMINAL       failure AT/AFTER the mutation (request sent;
#                                     response/identifier lost or invalid) — a remote
#                                     issue MAY already exist — NOT safe to retry
#       72  E_DISPATCH_NOT_AVAILABLE  trello/github-issues: no create path yet
#       73  E_DISPATCH_UNRECOGNISED   <sys> not in {linear,jira,trello,github-issues}
#                                     or empty — fail closed
#   - the integration's own error text is surfaced on stderr alongside the mapped code.

_WICR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WICR_INTEGRATIONS="$(cd "$_WICR_DIR/../../integrations" && pwd)"

# Exit-code taxonomy (E_DISPATCH_*) is owned by one sourced definition shared by
# every bridge and the decision scripts — see work-item-bridge-codes.sh.
# shellcheck source=skills/work/scripts/work-item-bridge-codes.sh
source "$_WICR_DIR/work-item-bridge-codes.sh"

_wicr_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-create-remote.sh --integration <sys> --title TEXT --body-file PATH [--kind KIND]
  <sys> ∈ {linear, jira, trello, github-issues}
USAGE
}

# Tracker-agnostic safety check on a returned identifier. Scoped narrowly to what
# actually breaks an unquoted YAML scalar writeback — reject control characters,
# newlines, a leading `---`, and a leading (optionally space-indented) `#` comment
# trigger. `/`, `#`, and `@` are explicitly permitted MID-token, since GitHub
# (owner/repo#42) and Trello identifiers legitimately contain them.
_wicr_identifier_safe() {
  local id="$1"
  [ -n "$id" ] || return 1
  # Newline / CR / tab: grep splits on newlines, so check these explicitly.
  case "$id" in
    *$'\n'* | *$'\r'* | *$'\t'*) return 1 ;;
  esac
  # Any other control character.
  if printf '%s' "$id" | LC_ALL=C grep -q '[[:cntrl:]]'; then
    return 1
  fi
  case "$id" in
    ---*) return 1 ;; # YAML document separator
  esac
  # Leading `#` (after optional leading whitespace) → YAML comment trigger.
  local lead_trimmed="${id#"${id%%[![:space:]]*}"}"
  case "$lead_trimmed" in
    '#'*) return 1 ;;
  esac
  return 0
}

# Map a Linear no-file create exit code to the dispatcher taxonomy.
#   108 E_CREATE_PRE_SEND  → retryable; 109 E_CREATE_POST_SEND → terminal.
# Any other non-zero is treated conservatively as terminal (may have created).
_wicr_map_linear() {
  case "$1" in
    108) return "$E_DISPATCH_RETRYABLE" ;;
    *) return "$E_DISPATCH_TERMINAL" ;;
  esac
}

# Map a Jira (resolver / jira-emit-key / jira-create-flow / jira-request) exit
# code to the dispatcher taxonomy. Retryable = provably no issue created (arg /
# validation / auth / 4xx-reject / rate-limit / unresolvable-config). Everything
# else — bad-response (16), 5xx (20), connect/DNS/timeout (21), and any
# unrecognised code — is conservatively terminal: the request may have been
# transmitted, so a remote issue may exist.
_wicr_map_jira() {
  case "$1" in
    100 | 101 | 102 | 103 | 104 | 105 | 106 | 107 | 108) return "$E_DISPATCH_RETRYABLE" ;;
    11 | 12 | 13 | 14 | 15 | 17 | 19 | 22 | 34) return "$E_DISPATCH_RETRYABLE" ;;
    *) return "$E_DISPATCH_TERMINAL" ;;
  esac
}

# Resolve + preview a tracker's user-visible target fields without creating.
_wicr_dry_run() {
  local integration="$1" kind="$2"
  case "$integration" in
    linear)
      printf 'linear\t(no user-resolvable type/project — team fixed by /init-linear)\n'
      ;;
    jira)
      local resolved resolve_rc=0
      resolved=$("$_WICR_INTEGRATIONS/jira/scripts/jira-resolve-fields.sh" \
        --kind "$kind") || resolve_rc=$?
      if ((resolve_rc != 0)); then
        _wicr_map_jira "$resolve_rc"
        return $?
      fi
      printf 'jira\t%s\n' "$resolved"
      ;;
    trello | github-issues)
      printf 'E_DISPATCH_NOT_AVAILABLE: create support for %s is not built yet (see work items 0049/0050)\n' \
        "$integration" >&2
      return "$E_DISPATCH_NOT_AVAILABLE"
      ;;
    *)
      printf 'E_DISPATCH_UNRECOGNISED: unknown or empty work.integration value: %q\n' \
        "$integration" >&2
      return "$E_DISPATCH_UNRECOGNISED"
      ;;
  esac
}

_wicr_main() {
  local integration="" title="" body_file="" kind=""
  local title_set=0 body_set=0 dry_run=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --integration)
        integration="$2"
        shift 2
        ;;
      --title)
        title="$2"
        title_set=1
        shift 2
        ;;
      --body-file)
        body_file="$2"
        body_set=1
        shift 2
        ;;
      --kind)
        kind="$2"
        shift 2
        ;;
      --dry-run)
        dry_run=1
        shift
        ;;
      --help | -h)
        _wicr_usage
        exit 0
        ;;
      *)
        _wicr_usage
        return "$E_DISPATCH_UNRECOGNISED"
        ;;
    esac
  done

  if ((dry_run)); then
    _wicr_dry_run "$integration" "$kind"
    return $?
  fi

  if ((!title_set)) || [[ -z "$title" ]]; then
    printf 'work-item-create-remote.sh: --title is required\n' >&2
    return "$E_DISPATCH_RETRYABLE"
  fi
  if ((!body_set)); then
    printf 'work-item-create-remote.sh: --body-file is required\n' >&2
    return "$E_DISPATCH_RETRYABLE"
  fi

  local identifier rc=0
  case "$integration" in
    linear)
      identifier=$("$_WICR_INTEGRATIONS/linear/scripts/linear-create-flow.sh" \
        --title "$title" --body-file "$body_file" --quiet) || rc=$?
      if ((rc != 0)); then
        _wicr_map_linear "$rc"
        return $?
      fi
      ;;
    jira)
      # Resolve project + issue type (single source of truth) before creating.
      local resolved resolve_rc=0
      resolved=$("$_WICR_INTEGRATIONS/jira/scripts/jira-resolve-fields.sh" \
        --kind "$kind") || resolve_rc=$?
      if ((resolve_rc != 0)); then
        _wicr_map_jira "$resolve_rc"
        return $?
      fi
      local jtype jproject
      jtype=$(printf '%s' "$resolved" | cut -f1)
      jproject=$(printf '%s' "$resolved" | cut -f3)
      identifier=$("$_WICR_INTEGRATIONS/jira/scripts/jira-emit-key.sh" \
        --project "$jproject" --type "$jtype" --summary "$title" \
        --body-file "$body_file" --quiet) || rc=$?
      if ((rc != 0)); then
        _wicr_map_jira "$rc"
        return $?
      fi
      ;;
    trello | github-issues)
      printf 'E_DISPATCH_NOT_AVAILABLE: create support for %s is not built yet (see work items 0049/0050)\n' \
        "$integration" >&2
      return "$E_DISPATCH_NOT_AVAILABLE"
      ;;
    *)
      printf 'E_DISPATCH_UNRECOGNISED: unknown or empty work.integration value: %q\n' \
        "$integration" >&2
      return "$E_DISPATCH_UNRECOGNISED"
      ;;
  esac

  # Success: the integration created the issue and returned a bare identifier.
  # Apply the tracker-agnostic safety check before passing it through. An unsafe
  # identifier means the issue exists remotely but cannot be safely written back
  # — terminal (do NOT retry).
  if ! _wicr_identifier_safe "$identifier"; then
    printf 'E_DISPATCH_TERMINAL: %s returned an unsafe identifier %q; an issue may exist — do NOT retry\n' \
      "$integration" "$identifier" >&2
    return "$E_DISPATCH_TERMINAL"
  fi

  printf '%s\n' "$identifier"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _wicr_main "$@"
fi
