#!/usr/bin/env bash
set -euo pipefail

# Reads a work-management configuration value with centralised defaults.
# Usage: config-read-work.sh <work_key>
#
# Recognised keys:
#   integration            → active remote tracker
#                            (allowed: jira, linear, trello, github-issues; default empty)
#   id_pattern             → DSL controlling work-item ID shape
#                            (default {number:04d})
#   default_project_code   → project code substituted into {project}
#                            (default empty)
#
# When called for an unknown work.* key, prints a warning to stderr and
# delegates to config-read-value.sh with empty default (mirrors config-read-path.sh).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=config-defaults.sh
source "$SCRIPT_DIR/config-defaults.sh"
# shellcheck source=log-common.sh
source "$SCRIPT_DIR/log-common.sh"

key="${1:-}"
if [ -z "$key" ]; then
  echo "Usage: config-read-work.sh <work_key>" >&2
  exit 1
fi

default=""
found=false
for i in "${!WORK_KEYS[@]}"; do
  if [ "${WORK_KEYS[$i]}" = "work.${key}" ]; then
    default="${WORK_DEFAULTS[$i]}"
    found=true
    break
  fi
done

if [ "$found" = false ]; then
  echo "config-read-work.sh: warning: unknown key 'work.${key}' — no centralized default" >&2
fi

value=$("$SCRIPT_DIR/config-read-value.sh" "work.${key}" "${default}")

if [ "$key" = "integration" ] && [ -n "$value" ]; then
  valid=false
  for allowed in "${WORK_INTEGRATION_VALUES[@]}"; do
    if [ "$value" = "$allowed" ]; then valid=true; break; fi
  done
  if [ "$valid" = false ]; then
    allowed_list="${WORK_INTEGRATION_VALUES[*]}"
    log_die "Error: work.integration must be one of: ${allowed_list// /, } (got '${value}'). Update work.integration in .accelerator/config.md or run '/accelerator:configure view' to inspect the current value."
  fi
fi

echo "$value"
