#!/usr/bin/env bash
set -euo pipefail

# jira-emit-key.sh — thin post-create wrapper around jira-create-flow.sh.
#
# Runs jira-create-flow.sh with the given flags, then extracts and validates the
# `.key` from the {id, key, self} response and prints ONLY the bare key on
# stdout. This keeps response parsing on the Jira side so the /create-work-item
# dispatcher gets a bare validated identifier from every integration and carries
# no per-tracker JSON handling. Field resolution (project / issue type) is a
# separate concern — see jira-resolve-fields.sh.
#
# All flags are forwarded verbatim to jira-create-flow.sh (--project, --type,
# --summary, --body-file, …). On a jira-create-flow.sh failure its exit code is
# propagated unchanged so the dispatcher can map transport/4xx/5xx codes. A
# success whose response carries no usable key is reported as E_REQ_BAD_RESPONSE
# (16) — the create may have produced an issue, so this is a post-create
# condition the dispatcher must treat as non-retryable.

_JEK_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Jira issue key shape, e.g. ENG-456 / PROJ-123.
readonly JIRA_KEY_RE='^[A-Z][A-Z0-9]+-[0-9]+$'

_jek_main() {
  local response rc=0
  response=$(bash "$_JEK_SCRIPT_DIR/jira-create-flow.sh" "$@") || rc=$?
  if ((rc != 0)); then
    return "$rc"
  fi

  # A dry-run (--print-payload) prints {method,path,…}, not {id,key,self};
  # pass it through unchanged so callers can preview.
  local arg
  for arg in "$@"; do
    if [[ "$arg" == "--print-payload" ]]; then
      printf '%s\n' "$response"
      return 0
    fi
  done

  local key
  key=$(printf '%s' "$response" | jq -r '.key // empty' 2>/dev/null || true)
  if [[ -z "$key" ]] || ! [[ "$key" =~ $JIRA_KEY_RE ]]; then
    printf 'E_REQ_BAD_RESPONSE: Jira create returned no usable key (got: %q); an issue may have been created — do NOT blindly retry\n' \
      "$key" >&2
    return 16
  fi

  printf '%s\n' "$key"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _jek_main "$@"
fi
