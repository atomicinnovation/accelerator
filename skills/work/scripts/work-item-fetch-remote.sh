#!/usr/bin/env bash
set -euo pipefail

# work-item-fetch-remote.sh — the READ counterpart to work-item-create-remote.sh.
#
# Routes a remote read to the configured tracker and returns a UNIFORM contract
# so the consuming skills (/list-work-items, /sync-work-items) never branch on
# tracker-specific output.
#
# Usage:
#   work-item-fetch-remote.sh --integration <sys> search [filter-flags…]
#   work-item-fetch-remote.sh --integration <sys> search --keys k1,k2,…
#   work-item-fetch-remote.sh --integration <sys> show --external-id <key>
#
# search (plain) → forwards user filter flags to the tracker's search adapter and
#   prints its raw JSON (the untracked-discovery path). jira ALWAYS injects
#   --fields updated,summary,description so .issues[].fields.updated (the remote
#   pre-filter source) is present. linear emits its own shape and auto-paginates.
#
# search --keys k1,k2,… → the key-scoped pre-filter read. One tracker-agnostic
#   CONTRACT — given the tracked external_ids, return their remote state — with
#   two adapters behind it. Output is a normalised JSON object:
#       { "found":         { "<key>": { "updated": "<iso|null>" }, … },
#         "absent":        [ "<key>", … ],   # gone from a COMPLETE fetch
#         "indeterminate": [ "<key>", … ] }  # fetch incomplete — never absent
#   - jira adapter: chunked `key in (…)` JQL (≤ 50 keys/request) paired with
#     --all-projects (so the key set is the sole filter — no injected
#     `project = <default>` clause that would drop cross-project keys), --limit
#     100, paginated via nextPageToken. A chunk that fails or hits the page cap
#     marks its keys indeterminate, NEVER absent.
#   - linear adapter: ONE team-wide search (--limit 250; it auto-paginates
#     internally) indexed by identifier; the bridge selects the tracked subset. A
#     truncated:true result marks every un-confirmed key indeterminate, never
#     absent — "absent ⇒ remote-absent" is only ever drawn from a provably
#     complete (truncated:false) fetch.
#
# show --external-id <key> → per-item full-fidelity read returning the issue's
#   body + updated timestamp (the genuinely-changed minority).
#     jira:   jira-show-flow.sh <key> --no-render-adf  (raw ADF for hashing)
#     linear: linear-show-flow.sh <identifier>         (Markdown-native, no ADF)
#
# Exit taxonomy (shared with the create/update bridges — work-item-bridge-codes.sh):
#   0 success; 70 retryable read failure / degrade; 72 not-available
#   (trello/github-issues read not built); 73 unrecognised <sys>.
# A read mutates nothing, so 71 (terminal-may-have-mutated) does not apply here —
# any underlying read failure collapses to 70 (the caller degrades to
# presence-only). Per-key partial failures inside --keys are reported as
# indeterminate markers with exit 0, not as a bridge-level failure.

_WIFR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=skills/work/scripts/work-item-bridge-codes.sh
source "$_WIFR_DIR/work-item-bridge-codes.sh"
_WIFR_INTEGRATIONS="$(cd "$_WIFR_DIR/../../integrations" && pwd)"

# Key-scoped read tuning. A ≤50-key chunk fits one 100-result page, so no
# speculative page-2 probe; the page loop still follows nextPageToken to
# exhaustion and caps at _WIFR_PAGE_CAP pages as a runaway backstop.
readonly _WIFR_JIRA_CHUNK=50
readonly _WIFR_JIRA_LIMIT=100
readonly _WIFR_PAGE_CAP=20
readonly _WIFR_LINEAR_LIMIT=250

_wifr_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-fetch-remote.sh --integration <sys> search [filter-flags…]
  work-item-fetch-remote.sh --integration <sys> search --keys k1,k2,…
  work-item-fetch-remote.sh --integration <sys> show --external-id <key>
  <sys> ∈ {linear, jira, trello, github-issues}
USAGE
}

# A jira key safe to embed unquoted in a `key in (…)` clause. Jira keys are
# `ABC-123`-shaped; reject anything carrying a character outside the safe set so
# a malformed/hostile external_id can never inject JQL.
_wifr_jira_key_safe() {
  case "$1" in
    '' | *[!A-Za-z0-9_-]*) return 1 ;;
  esac
  return 0
}

# jira --keys adapter. Emits the normalised {found,absent,indeterminate} object.
_wifr_jira_keys() {
  local keys_csv="$1"
  local -a requested=()
  local -a raw=()
  IFS=',' read -ra raw <<<"$keys_csv"
  local k
  for k in "${raw[@]+"${raw[@]}"}"; do
    [ -z "$k" ] && continue
    if ! _wifr_jira_key_safe "$k"; then
      printf 'work-item-fetch-remote.sh: unsafe jira key: %q\n' "$k" >&2
      return "$E_DISPATCH_RETRYABLE"
    fi
    requested+=("$k")
  done

  local found='{}' indeterminate='[]'
  local n=${#requested[@]} i=0
  while [ "$i" -lt "$n" ]; do
    local -a chunk=("${requested[@]:i:_WIFR_JIRA_CHUNK}")
    local clause list
    list=$(
      IFS=','
      printf '%s' "${chunk[*]}"
    )
    clause="key in ($list)"

    local page_token="" pages=0 chunk_ok=1
    while :; do
      pages=$((pages + 1))
      # SC2054: updated,summary,description is a single CSV field token, not
      # comma-separated array elements.
      # shellcheck disable=SC2054
      local -a args=(--all-projects --jql "$clause"
        --fields updated,summary,description --limit "$_WIFR_JIRA_LIMIT" --quiet)
      [ -n "$page_token" ] && args+=(--page-token "$page_token")
      local resp rc=0
      resp=$(bash "$_WIFR_INTEGRATIONS/jira/scripts/jira-search-flow.sh" \
        "${args[@]}") || rc=$?
      if [ "$rc" -ne 0 ]; then
        chunk_ok=0
        break
      fi
      found=$(printf '%s' "$resp" | jq -c --argjson acc "$found" '
        $acc + (reduce (.issues[]?) as $i ({};
          . + {($i.key): {updated: ($i.fields.updated // null)}}))')
      page_token=$(printf '%s' "$resp" | jq -r '.nextPageToken // empty')
      [ -z "$page_token" ] && break
      if [ "$pages" -ge "$_WIFR_PAGE_CAP" ]; then
        chunk_ok=0
        break
      fi
    done

    if [ "$chunk_ok" -ne 1 ]; then
      indeterminate=$(jq -cn --argjson acc "$indeterminate" --args \
        '$acc + $ARGS.positional' "${chunk[@]}")
    fi
    i=$((i + _WIFR_JIRA_CHUNK))
  done

  jq -cn --argjson found "$found" --argjson ind "$indeterminate" --args '
    ($ARGS.positional) as $req
    | ($found | keys) as $fk
    | { found: $found,
        indeterminate: ($ind | unique),
        absent: ($req - $fk - $ind | unique) }
  ' "${requested[@]+"${requested[@]}"}"
}

# linear --keys adapter. One team-wide auto-paginated search, indexed by
# identifier; truncated ⇒ un-confirmed keys are indeterminate, never absent.
_wifr_linear_keys() {
  local keys_csv="$1"
  local -a keys=()
  IFS=',' read -ra keys <<<"$keys_csv"

  local resp rc=0
  resp=$(bash "$_WIFR_INTEGRATIONS/linear/scripts/linear-search-flow.sh" \
    --limit "$_WIFR_LINEAR_LIMIT" --quiet) || rc=$?
  if [ "$rc" -ne 0 ]; then
    return "$E_DISPATCH_RETRYABLE"
  fi

  local truncated index
  truncated=$(printf '%s' "$resp" | jq -r '.data.issues.truncated // false')
  index=$(printf '%s' "$resp" | jq -c '
    reduce (.data.issues.nodes[]?) as $n ({};
      . + {($n.identifier): {updated: ($n.updatedAt // null)}})')

  jq -cn --argjson idx "$index" --arg trunc "$truncated" --args '
    ($ARGS.positional | map(select(. != "")) | unique) as $req
    | ($req | map(select($idx[.] != null))) as $present
    | ($req - $present) as $missing
    | { found: ($present | reduce .[] as $k ({}; . + {($k): $idx[$k]})),
        indeterminate: (if $trunc == "true" then $missing else [] end),
        absent: (if $trunc == "true" then [] else $missing end) }
  ' "${keys[@]+"${keys[@]}"}"
}

_wifr_main() {
  local integration="" op=""

  while [ $# -gt 0 ]; do
    case "$1" in
      --integration)
        integration="$2"
        shift 2
        ;;
      --help | -h)
        _wifr_usage
        exit 0
        ;;
      search | show)
        op="$1"
        shift
        break
        ;;
      *)
        _wifr_usage
        return "$E_DISPATCH_UNRECOGNISED"
        ;;
    esac
  done

  if [ -z "$op" ]; then
    _wifr_usage
    return "$E_DISPATCH_UNRECOGNISED"
  fi

  # Operation-scoped argument collection. (keys_arg is a plain CSV string; the
  # name avoids colliding with the `keys` array inside the adapter functions.)
  local keys_arg="" external_id=""
  local -a forward=()
  if [ "$op" = "search" ]; then
    while [ $# -gt 0 ]; do
      case "$1" in
        --keys)
          keys_arg="$2"
          shift 2
          ;;
        *)
          forward+=("$1")
          shift
          ;;
      esac
    done
  else # show
    while [ $# -gt 0 ]; do
      case "$1" in
        --external-id)
          external_id="$2"
          shift 2
          ;;
        *)
          _wifr_usage
          return "$E_DISPATCH_UNRECOGNISED"
          ;;
      esac
    done
    if [ -z "$external_id" ]; then
      printf 'work-item-fetch-remote.sh: show requires --external-id\n' >&2
      return "$E_DISPATCH_RETRYABLE"
    fi
  fi

  case "$integration" in
    jira)
      case "$op" in
        search)
          if [ -n "$keys_arg" ]; then
            _wifr_jira_keys "$keys_arg"
            return $?
          fi
          local rc=0
          bash "$_WIFR_INTEGRATIONS/jira/scripts/jira-search-flow.sh" \
            --fields updated,summary,description \
            "${forward[@]+"${forward[@]}"}" || rc=$?
          if [ "$rc" -ne 0 ]; then return "$E_DISPATCH_RETRYABLE"; fi
          ;;
        show)
          local rc=0
          bash "$_WIFR_INTEGRATIONS/jira/scripts/jira-show-flow.sh" \
            "$external_id" --no-render-adf || rc=$?
          if [ "$rc" -ne 0 ]; then return "$E_DISPATCH_RETRYABLE"; fi
          ;;
      esac
      ;;
    linear)
      case "$op" in
        search)
          if [ -n "$keys_arg" ]; then
            _wifr_linear_keys "$keys_arg"
            return $?
          fi
          local rc=0
          bash "$_WIFR_INTEGRATIONS/linear/scripts/linear-search-flow.sh" \
            --quiet "${forward[@]+"${forward[@]}"}" || rc=$?
          if [ "$rc" -ne 0 ]; then return "$E_DISPATCH_RETRYABLE"; fi
          ;;
        show)
          local rc=0
          bash "$_WIFR_INTEGRATIONS/linear/scripts/linear-show-flow.sh" \
            "$external_id" || rc=$?
          if [ "$rc" -ne 0 ]; then return "$E_DISPATCH_RETRYABLE"; fi
          ;;
      esac
      ;;
    trello | github-issues)
      printf 'E_DISPATCH_NOT_AVAILABLE: read support for %s is not built yet (see work items 0049/0050)\n' \
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

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wifr_main "$@"
fi
