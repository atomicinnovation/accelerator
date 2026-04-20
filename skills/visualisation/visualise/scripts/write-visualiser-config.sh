#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

PLUGIN_VERSION=""
PROJECT_ROOT=""
TMP_DIR=""
LOG_FILE=""
OWNER_PID=0
OWNER_START_TIME=""

while [ $# -gt 0 ]; do
  case "$1" in
    --plugin-version)    PLUGIN_VERSION="$2";    shift 2 ;;
    --project-root)      PROJECT_ROOT="$2";       shift 2 ;;
    --tmp-dir)           TMP_DIR="$2";            shift 2 ;;
    --log-file)          LOG_FILE="$2";           shift 2 ;;
    --owner-pid)         OWNER_PID="$2";          shift 2 ;;
    --owner-start-time)  OWNER_START_TIME="$2";   shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

for required in PLUGIN_VERSION PROJECT_ROOT TMP_DIR LOG_FILE; do
  if [ -z "${!required}" ]; then
    echo "missing required arg: --${required//_/-}" >&2
    exit 2
  fi
done

resolve_path() { "$PLUGIN_ROOT/scripts/config-read-path.sh" "$1" "$2"; }
abs_path() {
  echo "$PROJECT_ROOT/$(resolve_path "$1" "$2")"
}

DECISIONS="$(abs_path decisions meta/decisions)"
TICKETS="$(abs_path tickets meta/tickets)"
PLANS="$(abs_path plans meta/plans)"
RESEARCH="$(abs_path research meta/research)"
REVIEW_PLANS="$(abs_path review_plans meta/reviews/plans)"
REVIEW_PRS="$(abs_path review_prs meta/reviews/prs)"
VALIDATIONS="$(abs_path validations meta/validations)"
NOTES="$(abs_path notes meta/notes)"
PRS="$(abs_path prs meta/prs)"

TEMPLATES_USER_ROOT="$(abs_path templates meta/templates)"
TEMPLATES_PLUGIN_ROOT="$PLUGIN_ROOT/templates"

template_tier() {
  local name="$1"
  local override
  override="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "templates.$name" 2>/dev/null || true)"
  local override_json
  if [ -z "$override" ]; then
    override_json="null"
  else
    override_json="$(jq -nc --arg p "$override" '$p')"
  fi
  jq -nc \
    --argjson config_override "$override_json" \
    --arg user_override "$TEMPLATES_USER_ROOT/$name.md" \
    --arg plugin_default "$TEMPLATES_PLUGIN_ROOT/$name.md" \
    '{config_override:$config_override, user_override:$user_override, plugin_default:$plugin_default}'
}

ADR="$(template_tier adr)"
PLAN="$(template_tier plan)"
RES="$(template_tier research)"
VAL="$(template_tier validation)"
PRD="$(template_tier pr-description)"

if [ -z "$OWNER_START_TIME" ]; then
  OWNER_START_TIME_JSON="null"
else
  OWNER_START_TIME_JSON="$OWNER_START_TIME"
fi

jq -n \
  --arg plugin_root "$PLUGIN_ROOT" \
  --arg plugin_version "$PLUGIN_VERSION" \
  --arg tmp_path "$TMP_DIR" \
  --arg host "127.0.0.1" \
  --argjson owner_pid "$OWNER_PID" \
  --argjson owner_start_time "$OWNER_START_TIME_JSON" \
  --arg log_path "$LOG_FILE" \
  --arg decisions "$DECISIONS" --arg tickets "$TICKETS" \
  --arg plans "$PLANS" --arg research "$RESEARCH" \
  --arg review_plans "$REVIEW_PLANS" --arg review_prs "$REVIEW_PRS" \
  --arg validations "$VALIDATIONS" --arg notes "$NOTES" --arg prs "$PRS" \
  --argjson adr "$ADR" --argjson plan "$PLAN" --argjson research_t "$RES" \
  --argjson validation "$VAL" --argjson pr_description "$PRD" \
  '{
    plugin_root: $plugin_root,
    plugin_version: $plugin_version,
    tmp_path: $tmp_path,
    host: $host,
    owner_pid: $owner_pid,
    owner_start_time: $owner_start_time,
    log_path: $log_path,
    doc_paths: {
      decisions: $decisions, tickets: $tickets, plans: $plans,
      research: $research, review_plans: $review_plans,
      review_prs: $review_prs, validations: $validations,
      notes: $notes, prs: $prs
    },
    templates: {
      adr: $adr, plan: $plan, research: $research_t,
      validation: $validation, "pr-description": $pr_description
    }
  }'
