#!/usr/bin/env bash
set -euo pipefail

# Reads review configuration and outputs a markdown block with effective
# review settings and a unified lens catalogue.
#
# Usage: config-read-review.sh <pr|plan|ticket>
#
# Outputs nothing if no review config exists AND no custom lenses are found.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

MODE="${1:-}"
if [ -z "$MODE" ] || { [ "$MODE" != "pr" ] && [ "$MODE" != "plan" ] && [ "$MODE" != "ticket" ]; }; then
  echo "Usage: config-read-review.sh <pr|plan|ticket>" >&2
  exit 1
fi

READ_VALUE="$SCRIPT_DIR/config-read-value.sh"

# --- Defaults ---
DEFAULT_MAX_INLINE_COMMENTS=10
DEFAULT_DEDUP_PROXIMITY=3
DEFAULT_PR_REQUEST_CHANGES_SEVERITY="critical"
DEFAULT_PLAN_REVISE_SEVERITY="critical"
DEFAULT_PLAN_REVISE_MAJOR_COUNT=3
DEFAULT_TICKET_REVISE_SEVERITY="critical"
DEFAULT_TICKET_REVISE_MAJOR_COUNT=2
DEFAULT_MIN_LENSES=4
DEFAULT_MIN_LENSES_TICKET=3
DEFAULT_MAX_LENSES=8
DEFAULT_CORE_LENSES="architecture code-quality test-coverage correctness"
DEFAULT_DISABLED_LENSES=""

# Built-in lens names for code reviews (PR and plan modes)
BUILTIN_CODE_LENSES=(
  architecture
  code-quality
  compatibility
  correctness
  database
  documentation
  performance
  portability
  safety
  security
  standards
  test-coverage
  usability
)

# Built-in lens names for ticket reviews (ticket mode)
BUILTIN_TICKET_LENSES=(
  clarity
  completeness
  dependency
  scope
  testability
)

# Select the appropriate built-in lenses for the active mode.
# Returns them as newline-separated names via echo (Bash 3.2 compatible).
_select_builtin_lenses_for_mode() {
  if [ "$MODE" = "ticket" ]; then
    printf '%s\n' "${BUILTIN_TICKET_LENSES[@]+"${BUILTIN_TICKET_LENSES[@]}"}"
  else
    printf '%s\n' "${BUILTIN_CODE_LENSES[@]}"
  fi
}

# Effective min_lenses default depends on mode
if [ "$MODE" = "ticket" ]; then
  DEFAULT_MIN_LENSES_EFFECTIVE=$DEFAULT_MIN_LENSES_TICKET
else
  DEFAULT_MIN_LENSES_EFFECTIVE=$DEFAULT_MIN_LENSES
fi

# --- Step 1: Read all config values ---
max_inline_comments=$("$READ_VALUE" "review.max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS")
dedup_proximity=$("$READ_VALUE" "review.dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY")
pr_request_changes_severity=$("$READ_VALUE" "review.pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY")
plan_revise_severity=$("$READ_VALUE" "review.plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY")
plan_revise_major_count=$("$READ_VALUE" "review.plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT")
ticket_revise_severity=$("$READ_VALUE" "review.ticket_revise_severity" "$DEFAULT_TICKET_REVISE_SEVERITY")
ticket_revise_major_count=$("$READ_VALUE" "review.ticket_revise_major_count" "$DEFAULT_TICKET_REVISE_MAJOR_COUNT")
min_lenses=$("$READ_VALUE" "review.min_lenses" "$DEFAULT_MIN_LENSES_EFFECTIVE")
max_lenses=$("$READ_VALUE" "review.max_lenses" "$DEFAULT_MAX_LENSES")
core_lenses_raw=$("$READ_VALUE" "review.core_lenses" "")
disabled_lenses_raw=$("$READ_VALUE" "review.disabled_lenses" "")

# Parse array values
core_lenses=()
if [ -n "$core_lenses_raw" ]; then
  while IFS= read -r lens; do
    [ -n "$lens" ] && core_lenses+=("$lens")
  done < <(config_parse_array "$core_lenses_raw")
fi

disabled_lenses=()
if [ -n "$disabled_lenses_raw" ]; then
  while IFS= read -r lens; do
    [ -n "$lens" ] && disabled_lenses+=("$lens")
  done < <(config_parse_array "$disabled_lenses_raw")
fi

# --- Step 2: Validate numeric values ---
validate_non_negative_int() {
  local name="$1" value="$2" default="$3"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "Warning: review.$name must be a non-negative integer, got '$value' — using default ($default)" >&2
    echo "$default"
    return
  fi
  echo "$value"
}

validate_positive_int() {
  local name="$1" value="$2" default="$3"
  if ! [[ "$value" =~ ^[0-9]+$ ]] || [ "$value" -eq 0 ]; then
    echo "Warning: review.$name must be a positive integer, got '$value' — using default ($default)" >&2
    echo "$default"
    return
  fi
  echo "$value"
}

validate_severity() {
  local name="$1" value="$2" default="$3"
  case "$value" in
    critical|major|none) echo "$value" ;;
    *)
      echo "Warning: review.$name must be 'critical', 'major', or 'none', got '$value' — using default ($default)" >&2
      echo "$default"
      ;;
  esac
}

# Validate applies_to value (YAML flow-array or bare scalar).
# Prints the list of valid modes, one per line. Emits warnings for bad entries.
# arg1: lens name (for warning messages), arg2: raw applies_to value
validate_applies_to() {
  local lens_name="$1" raw="$2"
  local valid_modes="pr plan ticket"

  # Empty string means field was absent — caller handles this
  if [ -z "$raw" ]; then
    return 0
  fi

  # Parse: strip surrounding brackets if present (flow array)
  local stripped="$raw"
  if [[ "$stripped" == \[*\] ]]; then
    stripped="${stripped#[}"
    stripped="${stripped%]}"
  fi

  # Empty after stripping brackets → empty applies_to
  local trimmed
  trimmed=$(echo "$stripped" | tr -d ' ')
  if [ -z "$trimmed" ]; then
    echo "Warning: Custom lens '$lens_name' has empty applies_to — lens will not appear in any mode" >&2
    return 0
  fi

  # Split comma-separated entries
  local seen_modes=""
  local has_valid=false
  local all_invalid=true
  IFS=',' read -ra entries <<< "$stripped"
  for entry in "${entries[@]}"; do
    local mode
    mode=$(echo "$entry" | tr -d ' ')
    [ -z "$mode" ] && continue

    # Check if recognised
    local is_known=false
    for known in $valid_modes; do
      if [ "$mode" = "$known" ]; then
        is_known=true
        break
      fi
    done

    if [ "$is_known" = false ]; then
      echo "Warning: Custom lens '$lens_name' declares applies_to containing unrecognised mode '$mode' — ignoring that entry" >&2
      continue
    fi

    # Deduplicate
    if echo "$seen_modes" | grep -qw "$mode"; then
      continue
    fi
    seen_modes="$seen_modes $mode"
    echo "$mode"
    has_valid=true
    all_invalid=false
  done
}

max_inline_comments=$(validate_non_negative_int "max_inline_comments" "$max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS")
dedup_proximity=$(validate_non_negative_int "dedup_proximity" "$dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY")
min_lenses=$(validate_positive_int "min_lenses" "$min_lenses" "$DEFAULT_MIN_LENSES_EFFECTIVE")
max_lenses=$(validate_positive_int "max_lenses" "$max_lenses" "$DEFAULT_MAX_LENSES")
plan_revise_major_count=$(validate_positive_int "plan_revise_major_count" "$plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT")
ticket_revise_major_count=$(validate_positive_int "ticket_revise_major_count" "$ticket_revise_major_count" "$DEFAULT_TICKET_REVISE_MAJOR_COUNT")
pr_request_changes_severity=$(validate_severity "pr_request_changes_severity" "$pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY")
plan_revise_severity=$(validate_severity "plan_revise_severity" "$plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY")
ticket_revise_severity=$(validate_severity "ticket_revise_severity" "$ticket_revise_severity" "$DEFAULT_TICKET_REVISE_SEVERITY")

# Validate min_lenses <= max_lenses
if [ "$min_lenses" -gt "$max_lenses" ]; then
  echo "Warning: review.min_lenses ($min_lenses) > review.max_lenses ($max_lenses) — using defaults ($DEFAULT_MIN_LENSES_EFFECTIVE, $DEFAULT_MAX_LENSES)" >&2
  min_lenses=$DEFAULT_MIN_LENSES_EFFECTIVE
  max_lenses=$DEFAULT_MAX_LENSES
fi

# --- Step 3: Discover custom lenses ---
PROJECT_ROOT=$(config_project_root)
CUSTOM_LENSES_DIR="$PROJECT_ROOT/.claude/accelerator/lenses"

custom_lens_names=()
custom_lens_paths=()
custom_lens_auto_detect=()
custom_lens_applies_to=()  # parallel array; empty string means "all modes"

# Helper: extract a scalar field from frontmatter text
_read_frontmatter_scalar() {
  local fm="$1" key="$2"
  echo "$fm" | awk -v key="$key" '
    /^[^ \t]/ {
      prefix = key ":"
      if (substr($0, 1, length(prefix)) == prefix) {
        val = substr($0, length(prefix) + 1)
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

# Helper: extract a flow-array (or scalar) field from frontmatter text
_read_frontmatter_array() {
  local fm="$1" key="$2"
  echo "$fm" | awk -v key="$key" '
    /^[^ \t]/ {
      prefix = key ":"
      if (substr($0, 1, length(prefix)) == prefix) {
        val = substr($0, length(prefix) + 1)
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

if [ -d "$CUSTOM_LENSES_DIR" ]; then
  for lens_dir in "$CUSTOM_LENSES_DIR"/*/; do
    [ -d "$lens_dir" ] || continue
    skill_file="$lens_dir/SKILL.md"
    if [ ! -f "$skill_file" ]; then
      continue
    fi

    # Extract frontmatter
    fm=$(config_extract_frontmatter "$skill_file" 2>/dev/null) || {
      echo "Warning: Custom lens at $lens_dir has invalid frontmatter — skipping" >&2
      continue
    }

    lens_name=$(_read_frontmatter_scalar "$fm" "name") || {
      echo "Warning: Custom lens at $lens_dir missing 'name' in frontmatter — skipping" >&2
      continue
    }

    if [ -z "$lens_name" ]; then
      echo "Warning: Custom lens at $lens_dir has empty 'name' in frontmatter — skipping" >&2
      continue
    fi

    # Check for name collision with any built-in lens (union of all modes)
    for builtin in "${BUILTIN_CODE_LENSES[@]}" "${BUILTIN_TICKET_LENSES[@]+"${BUILTIN_TICKET_LENSES[@]}"}"; do
      if [ "$lens_name" = "$builtin" ]; then
        echo "Warning: Custom lens '$lens_name' conflicts with built-in lens name — skipping" >&2
        continue 2
      fi
    done

    # Extract optional auto_detect field
    auto_detect=$(_read_frontmatter_array "$fm" "auto_detect" 2>/dev/null) || auto_detect=""

    # Extract optional applies_to field
    applies_to_raw=$(_read_frontmatter_array "$fm" "applies_to" 2>/dev/null) || applies_to_raw=""

    custom_lens_names+=("$lens_name")
    custom_lens_paths+=("$skill_file")
    custom_lens_auto_detect+=("$auto_detect")
    custom_lens_applies_to+=("$applies_to_raw")
  done
fi

# Determine which custom lenses apply to the active mode
# and build the filtered lists
active_custom_lens_names=()
active_custom_lens_paths=()
active_custom_lens_auto_detect=()

if [ ${#custom_lens_names[@]} -gt 0 ]; then
  for i in "${!custom_lens_names[@]}"; do
    lens_name="${custom_lens_names[$i]}"
    applies_to_raw="${custom_lens_applies_to[$i]}"

    if [ -z "$applies_to_raw" ]; then
      # No applies_to field — applies to all modes
      active_custom_lens_names+=("$lens_name")
      active_custom_lens_paths+=("${custom_lens_paths[$i]}")
      active_custom_lens_auto_detect+=("${custom_lens_auto_detect[$i]}")
      continue
    fi

    # Validate applies_to and collect valid modes for this lens
    valid_modes_for_lens=()
    while IFS= read -r m; do
      [ -n "$m" ] && valid_modes_for_lens+=("$m")
    done < <(validate_applies_to "$lens_name" "$applies_to_raw")

    # Empty after validation means no valid modes (all unrecognised or empty array)
    if [ ${#valid_modes_for_lens[@]} -eq 0 ]; then
      continue
    fi

    # Check if active mode is in the lens's valid modes
    for m in "${valid_modes_for_lens[@]}"; do
      if [ "$m" = "$MODE" ]; then
        active_custom_lens_names+=("$lens_name")
        active_custom_lens_paths+=("${custom_lens_paths[$i]}")
        active_custom_lens_auto_detect+=("${custom_lens_auto_detect[$i]}")
        break
      fi
    done
  done
fi

# --- Step 4: Validate lens names in disabled_lenses and core_lenses ---
# Build combined set of all valid lens names (union across all modes)
all_lens_names=("${BUILTIN_CODE_LENSES[@]}" "${BUILTIN_TICKET_LENSES[@]+"${BUILTIN_TICKET_LENSES[@]}"}")
if [ ${#custom_lens_names[@]} -gt 0 ]; then
  for name in "${custom_lens_names[@]}"; do
    all_lens_names+=("$name")
  done
fi

is_valid_lens() {
  local name="$1"
  for valid in "${all_lens_names[@]}"; do
    [ "$name" = "$valid" ] && return 0
  done
  return 1
}

# Build the active mode's lens set (built-ins + active custom) for cross-mode filter
active_mode_lens_names=()
while IFS= read -r l; do
  [ -n "$l" ] && active_mode_lens_names+=("$l")
done < <(_select_builtin_lenses_for_mode)
for n in "${active_custom_lens_names[@]+"${active_custom_lens_names[@]}"}"; do
  active_mode_lens_names+=("$n")
done

is_active_mode_lens() {
  local name="$1"
  for active in "${active_mode_lens_names[@]+"${active_mode_lens_names[@]}"}"; do
    [ "$name" = "$active" ] && return 0
  done
  return 1
}

if [ ${#disabled_lenses[@]} -gt 0 ]; then
  for lens in "${disabled_lenses[@]}"; do
    if ! is_valid_lens "$lens"; then
      echo "Warning: review.disabled_lenses contains unrecognised lens '$lens'" >&2
    fi
  done
fi

# For core_lenses: warn on unknown entries; collect filtered (not-in-active-mode) entries
filtered_core_lenses=()  # valid cross-mode entries dropped for this mode
effective_core_lenses=() # entries valid for the active mode

if [ ${#core_lenses[@]} -gt 0 ]; then
  for lens in "${core_lenses[@]}"; do
    if ! is_valid_lens "$lens"; then
      echo "Warning: review.core_lenses contains unrecognised lens '$lens'" >&2
    elif ! is_active_mode_lens "$lens"; then
      filtered_core_lenses+=("$lens")
    else
      effective_core_lenses+=("$lens")
    fi
  done
elif [ "$MODE" = "ticket" ]; then
  # Default core lenses for ticket mode = all built-in ticket lenses
  while IFS= read -r l; do
    [ -n "$l" ] && effective_core_lenses+=("$l")
  done < <(_select_builtin_lenses_for_mode)
fi

# Check for lenses in both core and disabled
if [ ${#effective_core_lenses[@]} -gt 0 ] && [ ${#disabled_lenses[@]} -gt 0 ]; then
  for lens in "${effective_core_lenses[@]}"; do
    for disabled in "${disabled_lenses[@]}"; do
      if [ "$lens" = "$disabled" ]; then
        echo "Warning: Lens '$lens' appears in both core_lenses and disabled_lenses — disabled_lenses takes precedence" >&2
      fi
    done
  done
fi

# Check available lens count vs min_lenses (count only active-mode lenses)
builtin_count=${#active_mode_lens_names[@]}
available_count=$builtin_count
if [ ${#disabled_lenses[@]} -gt 0 ]; then
  for active_lens in "${active_mode_lens_names[@]+"${active_mode_lens_names[@]}"}"; do
    for disabled in "${disabled_lenses[@]}"; do
      if [ "$active_lens" = "$disabled" ]; then
        available_count=$((available_count - 1))
        break
      fi
    done
  done
fi

if [ "$available_count" -lt "$min_lenses" ]; then
  echo "Warning: Only $available_count lenses available after disabling, but min_lenses is $min_lenses" >&2
fi

# In ticket mode, when the user has explicitly set core_lenses to a subset of
# the built-in ticket lenses, emit a one-time informational note so they know
# the remaining non-disabled built-ins will be added up to max_lenses.
if [ "$MODE" = "ticket" ] && [ ${#core_lenses[@]} -gt 0 ]; then
  _missing_from_core=""
  for _blens in "${BUILTIN_TICKET_LENSES[@]}"; do
    _in_disabled=false
    for _dlens in "${disabled_lenses[@]+"${disabled_lenses[@]}"}"; do
      [ "$_blens" = "$_dlens" ] && _in_disabled=true && break
    done
    _in_core=false
    for _clens in "${core_lenses[@]}"; do
      [ "$_blens" = "$_clens" ] && _in_core=true && break
    done
    if ! $_in_disabled && ! $_in_core; then
      _missing_from_core="$_missing_from_core $_blens"
    fi
  done
  if [ -n "$_missing_from_core" ]; then
    _missing_from_core="${_missing_from_core# }"
    printf >&2 'Note: built-in ticket lens(es) not in your core_lenses but will be added up to max_lenses: %s\n' "$_missing_from_core"
    printf >&2 '      Add them to disabled_lenses to opt out, or raise core_lenses to include them explicitly.\n'
  fi
fi

# --- Helper: emit a labeled value, annotating overrides ---
_emit_value() {
  local label="$1" value="$2" default="$3"
  if [ "$value" != "$default" ]; then
    echo "- **${label}**: ${value} (default: ${default})"
  else
    echo "- **${label}**: ${value}"
  fi
}

# --- Generate output ---
echo "## Review Configuration"
echo ""

# Always output labeled variable definitions for all numeric/threshold values
if [ "$MODE" = "pr" ]; then
  _emit_value "max inline comments" "$max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS"
  _emit_value "dedup proximity" "$dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY"
  _emit_value "pr request changes severity" "$pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY"
fi

if [ "$MODE" = "plan" ]; then
  _emit_value "plan revise severity" "$plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY"
  _emit_value "plan revise major count" "$plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT"
fi

if [ "$MODE" = "ticket" ]; then
  _emit_value "ticket revise severity" "$ticket_revise_severity" "$DEFAULT_TICKET_REVISE_SEVERITY"
  _emit_value "ticket revise major count" "$ticket_revise_major_count" "$DEFAULT_TICKET_REVISE_MAJOR_COUNT"
fi

_emit_value "min lenses" "$min_lenses" "$DEFAULT_MIN_LENSES_EFFECTIVE"
_emit_value "max lenses" "$max_lenses" "$DEFAULT_MAX_LENSES"

# Conditional blocks: only shown when overridden (informational, not referenced as variables)
if [ ${#core_lenses[@]} -gt 0 ]; then
  core_str=$(printf '%s, ' "${core_lenses[@]}" | sed 's/, $//')
  default_core_str=$(echo "$DEFAULT_CORE_LENSES" | tr ' ' ', ' | sed 's/,/, /g')
  echo "- **Core lenses**: $core_str"
  echo "  (default: $default_core_str)"
fi

# Show filtered cross-mode core lenses info
if [ ${#filtered_core_lenses[@]} -gt 0 ]; then
  filtered_str=$(printf '%s, ' "${filtered_core_lenses[@]}" | sed 's/, $//')
  echo "- **Filtered core lenses (not applicable to $MODE mode)**: $filtered_str"
fi

if [ ${#disabled_lenses[@]} -gt 0 ]; then
  disabled_str=$(printf '%s, ' "${disabled_lenses[@]}" | sed 's/, $//')
  echo "- **Disabled lenses**: $disabled_str"
  echo "  (these lenses should be skipped regardless of auto-detect)"
fi

# Verdict overrides (conditional — only shown when changed)
if [ "$MODE" = "pr" ]; then
  if [ "$pr_request_changes_severity" != "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY" ]; then
    if [ "$pr_request_changes_severity" = "none" ]; then
      echo "- **Verdict**: REQUEST_CHANGES disabled (severity-based escalation turned off)"
      echo "  (default: any \`$DEFAULT_PR_REQUEST_CHANGES_SEVERITY\`)"
    else
      echo "- **Verdict**: REQUEST_CHANGES when any \`$pr_request_changes_severity\` or higher"
      echo "  (default: any \`$DEFAULT_PR_REQUEST_CHANGES_SEVERITY\`)"
    fi
  fi
elif [ "$MODE" = "plan" ]; then
  plan_verdict_changed=false
  if [ "$plan_revise_severity" != "$DEFAULT_PLAN_REVISE_SEVERITY" ] || \
     [ "$plan_revise_major_count" != "$DEFAULT_PLAN_REVISE_MAJOR_COUNT" ]; then
    plan_verdict_changed=true
  fi

  if [ "$plan_verdict_changed" = true ]; then
    if [ "$plan_revise_severity" = "none" ]; then
      sev_part="severity-based REVISE disabled"
    else
      sev_part="any \`$plan_revise_severity\`"
    fi
    echo "- **Verdict**: REVISE when $sev_part or ${plan_revise_major_count}+ \`major\`"
    echo "  (default: any \`$DEFAULT_PLAN_REVISE_SEVERITY\` or ${DEFAULT_PLAN_REVISE_MAJOR_COUNT}+ \`major\`)"
  fi
elif [ "$MODE" = "ticket" ]; then
  ticket_verdict_changed=false
  if [ "$ticket_revise_severity" != "$DEFAULT_TICKET_REVISE_SEVERITY" ] || \
     [ "$ticket_revise_major_count" != "$DEFAULT_TICKET_REVISE_MAJOR_COUNT" ]; then
    ticket_verdict_changed=true
  fi

  if [ "$ticket_verdict_changed" = true ]; then
    if [ "$ticket_revise_severity" = "none" ]; then
      sev_part="severity-based REVISE disabled"
    else
      sev_part="any \`$ticket_revise_severity\`"
    fi
    echo "- **Verdict**: REVISE when $sev_part or ${ticket_revise_major_count}+ \`major\`"
    echo "  (default: any \`$DEFAULT_TICKET_REVISE_SEVERITY\` or ${DEFAULT_TICKET_REVISE_MAJOR_COUNT}+ \`major\`)"
  fi
fi

# --- Lens Catalogue ---
echo ""
echo "### Lens Catalogue"
echo ""
echo "Use the paths below when constructing agent prompts. Always use the path"
echo "from this table rather than constructing paths from the lens name."
echo ""
echo "| Lens | Path | Source |"
echo "|------|------|--------|"

LENSES_BASE="$SCRIPT_DIR/../skills/review/lenses"
_LENSES_BASE_REL="skills/review/lenses"

while IFS= read -r lens; do
  [ -n "$lens" ] || continue
  lens_path="$_LENSES_BASE_REL/${lens}-lens/SKILL.md"
  echo "| $lens | $lens_path | built-in |"
done < <(_select_builtin_lenses_for_mode)

if [ ${#active_custom_lens_names[@]} -gt 0 ]; then
  for i in "${!active_custom_lens_names[@]}"; do
    name="${active_custom_lens_names[$i]}"
    path="${active_custom_lens_paths[$i]}"
    auto="${active_custom_lens_auto_detect[$i]}"
    if [ -n "$auto" ]; then
      echo "| $name | $path | custom |"
    else
      echo "| $name | $path | custom (always include) |"
    fi
  done
fi
