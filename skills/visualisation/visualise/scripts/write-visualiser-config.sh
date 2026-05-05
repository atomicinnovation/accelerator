#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/config-common.sh"

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

# Pre-flight migration check: reject launches from projects that still carry a
# `paths.tickets` config key without a corresponding `paths.work` key. This
# indicates the project predates the tickets→work-items rename and the
# visualiser would silently produce an empty kanban without this guard.
TICKETS_OVERRIDE="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "paths.tickets" "" 2>/dev/null || true)"
WORK_OVERRIDE="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "paths.work" "" 2>/dev/null || true)"
if [ -n "$TICKETS_OVERRIDE" ] && [ -z "$WORK_OVERRIDE" ]; then
  echo "This project predates the tickets→work-items rename. Run \`/accelerator:migrate\` to apply migration \`0001-rename-tickets-to-work\` before launching the visualiser." >&2
  exit 1
fi

DECISIONS="$(abs_path decisions meta/decisions)"
WORK="$(abs_path work meta/work)"
REVIEW_WORK="$(abs_path review_work meta/reviews/work)"
PLANS="$(abs_path plans meta/plans)"
RESEARCH="$(abs_path research meta/research)"
REVIEW_PLANS="$(abs_path review_plans meta/reviews/plans)"
REVIEW_PRS="$(abs_path review_prs meta/reviews/prs)"
VALIDATIONS="$(abs_path validations meta/validations)"
NOTES="$(abs_path notes meta/notes)"
PRS="$(abs_path prs meta/prs)"

TEMPLATES_USER_ROOT="$(abs_path templates .accelerator/templates)"
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
WI="$(template_tier work-item)"

# Work-item ID pattern config. Read from `work.id_pattern` / `work.default_project_code`;
# compile the scan regex via the work-item-pattern skill's --compile-scan subcommand.
WORK_SCRIPT="$PLUGIN_ROOT/skills/work/scripts/work-item-pattern.sh"
ID_PATTERN="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "work.id_pattern" "{number:04d}" 2>/dev/null || echo "{number:04d}")"
PROJECT_CODE="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "work.default_project_code" "" 2>/dev/null || true)"
SCAN_REGEX="$("$WORK_SCRIPT" --compile-scan "$ID_PATTERN" "$PROJECT_CODE")"

# Build the work_item JSON block. If PROJECT_CODE is empty, omit default_project_code.
if [ -z "$PROJECT_CODE" ]; then
  WORK_ITEM_JSON="$(jq -nc --arg scan_regex "$SCAN_REGEX" --arg id_pattern "$ID_PATTERN" \
    '{scan_regex:$scan_regex, id_pattern:$id_pattern}')"
else
  WORK_ITEM_JSON="$(jq -nc --arg scan_regex "$SCAN_REGEX" --arg id_pattern "$ID_PATTERN" \
    --arg default_project_code "$PROJECT_CODE" \
    '{scan_regex:$scan_regex, id_pattern:$id_pattern, default_project_code:$default_project_code}')"
fi

# Kanban columns config. Read from `visualiser.kanban_columns`; default to the
# seven statuses from templates/work-item.md. Validates inline-array syntax and
# rejects empty lists (a misconfiguration, not a safe default).
KANBAN_DEFAULT="[draft, ready, in-progress, review, done, blocked, abandoned]"
KANBAN_RAW="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "visualiser.kanban_columns" "$KANBAN_DEFAULT" 2>/dev/null || echo "$KANBAN_DEFAULT")"

# Detect unclosed inline-array bracket (starts with [ but doesn't end with ])
case "$KANBAN_RAW" in
  "["*)
    case "$KANBAN_RAW" in
      *"]") : ;; # valid bracket form
      *)
        echo "visualiser.kanban_columns: malformed inline array (missing closing ']')" >&2
        exit 1
        ;;
    esac
    ;;
esac

# Parse array elements → JSON array of strings
KANBAN_COLS_JSON="$(config_parse_array "$KANBAN_RAW" | jq -Rc '.' | jq -s '.')"

# Validate non-empty
if [ "$(printf '%s' "$KANBAN_COLS_JSON" | jq 'length')" -eq 0 ]; then
  echo "visualiser.kanban_columns: configured list must not be empty" >&2
  exit 1
fi

if [ -z "$OWNER_START_TIME" ]; then
  OWNER_START_TIME_JSON="null"
else
  OWNER_START_TIME_JSON="$OWNER_START_TIME"
fi

jq -n \
  --arg plugin_root "$PLUGIN_ROOT" \
  --arg plugin_version "$PLUGIN_VERSION" \
  --arg project_root "$PROJECT_ROOT" \
  --arg tmp_path "$TMP_DIR" \
  --arg host "127.0.0.1" \
  --argjson owner_pid "$OWNER_PID" \
  --argjson owner_start_time "$OWNER_START_TIME_JSON" \
  --arg log_path "$LOG_FILE" \
  --arg decisions "$DECISIONS" --arg work "$WORK" --arg review_work "$REVIEW_WORK" \
  --arg plans "$PLANS" --arg research "$RESEARCH" \
  --arg review_plans "$REVIEW_PLANS" --arg review_prs "$REVIEW_PRS" \
  --arg validations "$VALIDATIONS" --arg notes "$NOTES" --arg prs "$PRS" \
  --argjson adr "$ADR" --argjson plan "$PLAN" --argjson research_t "$RES" \
  --argjson validation "$VAL" --argjson pr_description "$PRD" \
  --argjson work_item_template "$WI" \
  --argjson work_item "$WORK_ITEM_JSON" \
  --argjson kanban_columns "$KANBAN_COLS_JSON" \
  '{
    plugin_root: $plugin_root,
    plugin_version: $plugin_version,
    project_root: $project_root,
    tmp_path: $tmp_path,
    host: $host,
    owner_pid: $owner_pid,
    owner_start_time: $owner_start_time,
    log_path: $log_path,
    doc_paths: {
      decisions: $decisions, work: $work, review_work: $review_work,
      plans: $plans, research: $research, review_plans: $review_plans,
      review_prs: $review_prs, validations: $validations,
      notes: $notes, prs: $prs
    },
    templates: {
      adr: $adr, plan: $plan, research: $research_t,
      validation: $validation, "pr-description": $pr_description,
      "work-item": $work_item_template
    },
    work_item: $work_item,
    kanban_columns: $kanban_columns
  }'
