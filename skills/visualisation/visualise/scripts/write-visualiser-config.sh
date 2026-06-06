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
    --plugin-version)
      PLUGIN_VERSION="$2"
      shift 2
      ;;
    --project-root)
      PROJECT_ROOT="$2"
      shift 2
      ;;
    --tmp-dir)
      TMP_DIR="$2"
      shift 2
      ;;
    --log-file)
      LOG_FILE="$2"
      shift 2
      ;;
    --owner-pid)
      OWNER_PID="$2"
      shift 2
      ;;
    --owner-start-time)
      OWNER_START_TIME="$2"
      shift 2
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

for required in PLUGIN_VERSION PROJECT_ROOT TMP_DIR LOG_FILE; do
  if [ -z "${!required}" ]; then
    echo "missing required arg: --${required//_/-}" >&2
    exit 2
  fi
done

resolve_path() { "$PLUGIN_ROOT/scripts/config-read-path.sh" "$1"; }
abs_path() {
  echo "$PROJECT_ROOT/$(resolve_path "$1")"
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

DECISIONS="$(abs_path decisions)"
WORK="$(abs_path work)"
REVIEW_WORK="$(abs_path review_work)"
PLANS="$(abs_path plans)"
RESEARCH_CODEBASE="$(abs_path research_codebase)"
RESEARCH_ISSUES="$(abs_path research_issues)"
REVIEW_PLANS="$(abs_path review_plans)"
REVIEW_PRS="$(abs_path review_prs)"
VALIDATIONS="$(abs_path validations)"
NOTES="$(abs_path notes)"
PRS="$(abs_path prs)"
RESEARCH_DESIGN_GAPS="$(abs_path research_design_gaps)"
RESEARCH_DESIGN_INVENTORIES="$(abs_path research_design_inventories)"

TEMPLATES_USER_ROOT="$(abs_path templates)"
TEMPLATES_PLUGIN_ROOT="$PLUGIN_ROOT/templates"

template_tier() {
  local name="$1"
  local override
  override="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "templates.$name" 2>/dev/null || true)"
  local override_json
  local override_source_json="null"
  if [ -z "$override" ]; then
    override_json="null"
  else
    override_json="$(jq -nc --arg p "$override" '$p')"
    # Determine which config file declared the override. config.local.md
    # has higher precedence than config.md, so check the local file first.
    local key_re="^[[:space:]]*${name}:"
    local team_file="$PROJECT_ROOT/.accelerator/config.md"
    local local_file="$PROJECT_ROOT/.accelerator/config.local.md"
    if [ -f "$local_file" ] &&
      awk '/^---[[:space:]]*$/{c++; next} c==1{print}' "$local_file" 2>/dev/null |
      grep -qE "$key_re" 2>/dev/null; then
      override_source_json='".accelerator/config.local.md"'
    elif [ -f "$team_file" ] &&
      awk '/^---[[:space:]]*$/{c++; next} c==1{print}' "$team_file" 2>/dev/null |
      grep -qE "$key_re" 2>/dev/null; then
      override_source_json='".accelerator/config.md"'
    fi
  fi
  jq -nc \
    --argjson config_override "$override_json" \
    --arg user_override "$TEMPLATES_USER_ROOT/$name.md" \
    --arg plugin_default "$TEMPLATES_PLUGIN_ROOT/$name.md" \
    --argjson config_override_source "$override_source_json" \
    '{config_override:$config_override, user_override:$user_override, plugin_default:$plugin_default, config_override_source:$config_override_source}'
}

ADR="$(template_tier adr)"
PLAN="$(template_tier plan)"
RES="$(template_tier codebase-research)"
VAL="$(template_tier validation)"
PRD="$(template_tier pr-description)"
WI="$(template_tier work-item)"
DGAP="$(template_tier design-gap)"
DINV="$(template_tier design-inventory)"

# Work-item ID pattern config. Read from `work.id_pattern` / `work.default_project_code`;
# compile the scan regex via the work-item-pattern skill's --compile-scan subcommand.
WORK_SCRIPT="$PLUGIN_ROOT/skills/work/scripts/work-item-pattern.sh"
ID_PATTERN="$("$PLUGIN_ROOT/scripts/config-read-work.sh" id_pattern)"
PROJECT_CODE="$("$PLUGIN_ROOT/scripts/config-read-work.sh" default_project_code)"
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

# Idle auto-shutdown window. Precedence: env var > visualiser.idle_timeout
# config key > (omit → Rust applies the 8h default).
#
# Note on the empty env var: `:-` treats ACCELERATOR_VISUALISER_IDLE_TIMEOUT=""
# (set-but-empty) identically to unset, so an empty env value falls through to
# the config key rather than overriding it with "".
IDLE_TIMEOUT="${ACCELERATOR_VISUALISER_IDLE_TIMEOUT:-}"
if [ -z "$IDLE_TIMEOUT" ]; then
  IDLE_TIMEOUT="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "visualiser.idle_timeout" "" 2>/dev/null || true)"
fi

if [ -n "$IDLE_TIMEOUT" ]; then
  # Trim surrounding whitespace so the shell's accept-set is a superset of Rust's
  # (resolve_idle_limit_ms trims before parsing); bash-3.2 safe. LANG=C keeps the
  # whitespace class deterministic across locales, matching the launcher's
  # existing locale-hardening. NOTE: this is a *separate* block from the guard
  # below on purpose — a whitespace-only value collapses to empty here and is then
  # treated as unset (falls through to the 8h default), rather than reaching the
  # guard and erroring.
  IDLE_TIMEOUT="$(printf '%s' "$IDLE_TIMEOUT" | LANG=C sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
fi
if [ -n "$IDLE_TIMEOUT" ]; then
  # Coarse typo-guard. A duration-SHAPED value starts with a digit; a disable
  # token is `never`/`0`. Anything else (e.g. "soon", "off", "in a bit") clearly
  # is not a duration and is rejected here, on the user's terminal, where the
  # Rust error is invisible (stderr is /dev/null'd after config load).
  #
  # This is deliberately a *shape* check, NOT the humantime grammar: every valid
  # humantime duration begins with a digit, so this can never reject a value Rust
  # accepts (including compound "1h30m" and spaced "1h 30m"). Rust's
  # resolve_idle_limit_ms is the authoritative parser and fail-fast backstop for
  # anything the guard waves through (e.g. "5 zonks", "0.0").
  # Keep the example list in the message in sync with
  # ConfigError::InvalidIdleTimeout (config.rs).
  case "$IDLE_TIMEOUT" in
    [Nn][Ee][Vv][Ee][Rr] | 0) : ;;  # disable tokens
    # Zero-length durations (0s/0ms) are digit-led, so they pass here as
    # duration-shaped; Rust resolves them to the disable sentinel.
    [0-9]*) : ;;                     # duration-shaped: starts with a digit
    *)
      echo "error: invalid visualiser.idle_timeout '$IDLE_TIMEOUT': expected a duration like \"8h\", \"30m\", \"1h30m\", or \"never\"/0 to disable" >&2
      exit 1
      ;;
  esac
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
  --arg plans "$PLANS" \
  --arg research_codebase "$RESEARCH_CODEBASE" \
  --arg research_issues "$RESEARCH_ISSUES" \
  --arg review_plans "$REVIEW_PLANS" --arg review_prs "$REVIEW_PRS" \
  --arg validations "$VALIDATIONS" --arg notes "$NOTES" --arg prs "$PRS" \
  --arg research_design_gaps "$RESEARCH_DESIGN_GAPS" \
  --arg research_design_inventories "$RESEARCH_DESIGN_INVENTORIES" \
  --argjson adr "$ADR" --argjson plan "$PLAN" --argjson research_t "$RES" \
  --argjson validation "$VAL" --argjson pr_description "$PRD" \
  --argjson work_item_template "$WI" \
  --argjson design_gap "$DGAP" \
  --argjson design_inventory "$DINV" \
  --argjson work_item "$WORK_ITEM_JSON" \
  --argjson kanban_columns "$KANBAN_COLS_JSON" \
  --arg idle_timeout "$IDLE_TIMEOUT" \
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
      plans: $plans,
      research_codebase: $research_codebase,
      research_issues: $research_issues,
      review_plans: $review_plans,
      review_prs: $review_prs, validations: $validations,
      notes: $notes, prs: $prs,
      research_design_gaps: $research_design_gaps,
      research_design_inventories: $research_design_inventories
    },
    templates: {
      adr: $adr, plan: $plan, "codebase-research": $research_t,
      validation: $validation, "pr-description": $pr_description,
      "work-item": $work_item_template,
      "design-gap": $design_gap,
      "design-inventory": $design_inventory
    },
    work_item: $work_item,
    kanban_columns: $kanban_columns
  }
  + (if $idle_timeout == "" then {} else {idle_timeout: $idle_timeout} end)'
