#!/usr/bin/env bash
# Shared helpers for work-management consumers (integration skills).
# Sourced by jira-common.sh (and by future linear-common.sh / etc.).

WORK_COMMON_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=log-common.sh
source "$WORK_COMMON_SCRIPT_DIR/log-common.sh"

# Resolve the default project code, warning when work.integration is set
# but work.default_project_code is empty. Echoes the project code
# (possibly empty) for the caller to use as its default.
#
# Both reads intentionally omit `2>/dev/null` and `||` fallback so that
# Phase 2's enum validation hard-fail (log_die) propagates via set -euo
# pipefail, and real script errors on the default_project_code read are
# surfaced rather than masked into a silent empty value.
work_resolve_default_project() {
  local read_work integration project
  read_work="$WORK_COMMON_SCRIPT_DIR/config-read-work.sh"
  integration=$("$read_work" integration) || return $?
  project=$("$read_work" default_project_code) || return $?
  if [ -n "$integration" ] && [ -z "$project" ]; then
    log_warn "work.default_project_code is empty but work.integration is set ($integration) — pass --project explicitly or set default_project_code in .accelerator/config.md"
  fi
  echo "$project"
}
