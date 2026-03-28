#!/usr/bin/env bash
set -euo pipefail

# Reads review configuration and outputs a markdown block with effective
# review settings and a unified lens catalogue.
#
# Usage: config-read-review.sh <pr|plan>
#
# Outputs nothing if no review config exists AND no custom lenses are found.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

MODE="${1:-}"
if [ -z "$MODE" ] || { [ "$MODE" != "pr" ] && [ "$MODE" != "plan" ]; }; then
  echo "Usage: config-read-review.sh <pr|plan>" >&2
  exit 1
fi

READ_VALUE="$SCRIPT_DIR/config-read-value.sh"

# --- Defaults ---
DEFAULT_MAX_INLINE_COMMENTS=10
DEFAULT_DEDUP_PROXIMITY=3
DEFAULT_PR_REQUEST_CHANGES_SEVERITY="critical"
DEFAULT_PLAN_REVISE_SEVERITY="critical"
DEFAULT_PLAN_REVISE_MAJOR_COUNT=3
DEFAULT_MIN_LENSES=4
DEFAULT_MAX_LENSES=8
DEFAULT_CORE_LENSES="architecture code-quality test-coverage correctness"
DEFAULT_DISABLED_LENSES=""

# Built-in lens names (13 lenses matching skills/review/lenses/)
BUILTIN_LENSES=(
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

# --- Step 1: Read all config values ---
max_inline_comments=$("$READ_VALUE" "review.max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS")
dedup_proximity=$("$READ_VALUE" "review.dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY")
pr_request_changes_severity=$("$READ_VALUE" "review.pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY")
plan_revise_severity=$("$READ_VALUE" "review.plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY")
plan_revise_major_count=$("$READ_VALUE" "review.plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT")
min_lenses=$("$READ_VALUE" "review.min_lenses" "$DEFAULT_MIN_LENSES")
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

max_inline_comments=$(validate_non_negative_int "max_inline_comments" "$max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS")
dedup_proximity=$(validate_non_negative_int "dedup_proximity" "$dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY")
min_lenses=$(validate_positive_int "min_lenses" "$min_lenses" "$DEFAULT_MIN_LENSES")
max_lenses=$(validate_positive_int "max_lenses" "$max_lenses" "$DEFAULT_MAX_LENSES")
plan_revise_major_count=$(validate_positive_int "plan_revise_major_count" "$plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT")
pr_request_changes_severity=$(validate_severity "pr_request_changes_severity" "$pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY")
plan_revise_severity=$(validate_severity "plan_revise_severity" "$plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY")

# Validate min_lenses <= max_lenses
if [ "$min_lenses" -gt "$max_lenses" ]; then
  echo "Warning: review.min_lenses ($min_lenses) > review.max_lenses ($max_lenses) — using defaults ($DEFAULT_MIN_LENSES, $DEFAULT_MAX_LENSES)" >&2
  min_lenses=$DEFAULT_MIN_LENSES
  max_lenses=$DEFAULT_MAX_LENSES
fi

# --- Step 3: Discover custom lenses ---
PROJECT_ROOT=$(config_project_root)
CUSTOM_LENSES_DIR="$PROJECT_ROOT/.claude/accelerator/lenses"

custom_lens_names=()
custom_lens_paths=()
custom_lens_auto_detect=()

if [ -d "$CUSTOM_LENSES_DIR" ]; then
  for lens_dir in "$CUSTOM_LENSES_DIR"/*/; do
    [ -d "$lens_dir" ] || continue
    skill_file="$lens_dir/SKILL.md"
    if [ ! -f "$skill_file" ]; then
      continue
    fi

    # Extract frontmatter and check for name field
    fm=$(config_extract_frontmatter "$skill_file" 2>/dev/null) || {
      echo "Warning: Custom lens at $lens_dir has invalid frontmatter — skipping" >&2
      continue
    }

    lens_name=$(echo "$fm" | awk -v key="name" '
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
    ') || {
      echo "Warning: Custom lens at $lens_dir missing 'name' in frontmatter — skipping" >&2
      continue
    }

    if [ -z "$lens_name" ]; then
      echo "Warning: Custom lens at $lens_dir has empty 'name' in frontmatter — skipping" >&2
      continue
    fi

    # Check for name collision with built-in lenses
    for builtin in "${BUILTIN_LENSES[@]}"; do
      if [ "$lens_name" = "$builtin" ]; then
        echo "Warning: Custom lens '$lens_name' conflicts with built-in lens name — skipping" >&2
        continue 2
      fi
    done

    # Extract optional auto_detect field
    auto_detect=$(echo "$fm" | awk -v key="auto_detect" '
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
    ' 2>/dev/null) || auto_detect=""

    custom_lens_names+=("$lens_name")
    custom_lens_paths+=("$skill_file")
    custom_lens_auto_detect+=("$auto_detect")
  done
fi

# --- Step 4: Validate lens names in disabled_lenses and core_lenses ---
# Build combined set of all valid lens names
all_lens_names=("${BUILTIN_LENSES[@]}")
for name in "${custom_lens_names[@]}"; do
  all_lens_names+=("$name")
done

is_valid_lens() {
  local name="$1"
  for valid in "${all_lens_names[@]}"; do
    [ "$name" = "$valid" ] && return 0
  done
  return 1
}

for lens in "${disabled_lenses[@]}"; do
  if ! is_valid_lens "$lens"; then
    echo "Warning: review.disabled_lenses contains unrecognised lens '$lens'" >&2
  fi
done

for lens in "${core_lenses[@]}"; do
  if ! is_valid_lens "$lens"; then
    echo "Warning: review.core_lenses contains unrecognised lens '$lens'" >&2
  fi
done

# Check for lenses in both core and disabled
for lens in "${core_lenses[@]}"; do
  for disabled in "${disabled_lenses[@]}"; do
    if [ "$lens" = "$disabled" ]; then
      echo "Warning: Lens '$lens' appears in both core_lenses and disabled_lenses — disabled_lenses takes precedence" >&2
    fi
  done
done

# Check available lens count vs min_lenses
available_count=${#BUILTIN_LENSES[@]}
for builtin in "${BUILTIN_LENSES[@]}"; do
  for disabled in "${disabled_lenses[@]}"; do
    if [ "$builtin" = "$disabled" ]; then
      available_count=$((available_count - 1))
      break
    fi
  done
done
available_count=$((available_count + ${#custom_lens_names[@]}))

if [ "$available_count" -lt "$min_lenses" ]; then
  echo "Warning: Only $available_count lenses available after disabling, but min_lenses is $min_lenses" >&2
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

_emit_value "min lenses" "$min_lenses" "$DEFAULT_MIN_LENSES"
_emit_value "max lenses" "$max_lenses" "$DEFAULT_MAX_LENSES"

# Conditional blocks: only shown when overridden (informational, not referenced as variables)
if [ ${#core_lenses[@]} -gt 0 ]; then
  core_str=$(printf '%s, ' "${core_lenses[@]}" | sed 's/, $//')
  default_core_str=$(echo "$DEFAULT_CORE_LENSES" | tr ' ' ', ' | sed 's/,/, /g')
  echo "- **Core lenses**: $core_str"
  echo "  (default: $default_core_str)"
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

for lens in "${BUILTIN_LENSES[@]}"; do
  lens_path=$(cd "$LENSES_BASE/${lens}-lens" 2>/dev/null && echo "$(pwd)/SKILL.md")
  echo "| $lens | $lens_path | built-in |"
done

for i in "${!custom_lens_names[@]}"; do
  name="${custom_lens_names[$i]}"
  path="${custom_lens_paths[$i]}"
  auto="${custom_lens_auto_detect[$i]}"
  if [ -n "$auto" ]; then
    echo "| $name | $path | custom |"
  else
    echo "| $name | $path | custom (always include) |"
  fi
done
