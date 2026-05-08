#!/usr/bin/env bash
set -euo pipefail

# Outputs all configured review and agent keys with their effective values
# and source attribution (team, local, or default).
#
# Usage: config-dump.sh
#
# Outputs nothing if no config files exist.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
config_assert_no_legacy_layout

READ_VALUE="$SCRIPT_DIR/config-read-value.sh"

# Check if any config files exist
config_files=$(config_find_files || true)
if [ -z "$config_files" ]; then
  exit 0
fi

PROJECT_ROOT=$(config_project_root)
TEAM_FILE="$PROJECT_ROOT/.accelerator/config.md"
LOCAL_FILE="$PROJECT_ROOT/.accelerator/config.local.md"

# Read a value from a specific file only (returns empty if not found)
_read_from_file() {
  local file="$1" key="$2"
  local section="" subkey=""

  if [[ "$key" == *.* ]]; then
    section="${key%%.*}"
    subkey="${key#*.}"
  else
    subkey="$key"
  fi

  local fm
  fm=$(config_extract_frontmatter "$file" 2>/dev/null) || return 1
  [ -z "$fm" ] && return 1

  if [ -n "$section" ]; then
    echo "$fm" | awk -v section="$section" -v subkey="$subkey" '
      {
        prefix = section ":"
        if (substr($0, 1, length(prefix)) == prefix && \
            (length($0) == length(prefix) || \
             substr($0, length(prefix)+1, 1) ~ /[ \t]/)) {
          in_section = 1
          next
        }
      }
      in_section && /^[^ \t]/ && /[^ \t]/ { in_section = 0 }
      in_section {
        stripped = $0
        sub(/^[ \t]+/, "", stripped)
        kprefix = subkey ":"
        if (substr(stripped, 1, length(kprefix)) == kprefix) {
          val = substr(stripped, length(kprefix) + 1)
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
  else
    echo "$fm" | awk -v key="$subkey" '
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
  fi
}

# Determine source of a key's value
get_source() {
  local key="$1"
  if [ -f "$LOCAL_FILE" ] && _read_from_file "$LOCAL_FILE" "$key" >/dev/null 2>&1; then
    echo "local (.accelerator/config.local.md)"
  elif [ -f "$TEAM_FILE" ] && _read_from_file "$TEAM_FILE" "$key" >/dev/null 2>&1; then
    echo "team (.accelerator/config.md)"
  else
    echo "default"
  fi
}

# All known config keys with their defaults
declare -A DEFAULTS
DEFAULTS=(
  ["review.max_inline_comments"]="10"
  ["review.min_lenses"]="4"
  ["review.max_lenses"]="8"
  ["review.dedup_proximity"]="3"
  ["review.core_lenses"]="[architecture, code-quality, test-coverage, correctness]"
  ["review.disabled_lenses"]="[]"
  ["review.pr_request_changes_severity"]="critical"
  ["review.plan_revise_severity"]="critical"
  ["review.plan_revise_major_count"]="3"
)

# Ordered key list for consistent output
REVIEW_KEYS=(
  "review.max_inline_comments"
  "review.min_lenses"
  "review.max_lenses"
  "review.dedup_proximity"
  "review.core_lenses"
  "review.disabled_lenses"
  "review.pr_request_changes_severity"
  "review.plan_revise_severity"
  "review.plan_revise_major_count"
)

# Agent keys
AGENT_KEYS=(
  "agents.reviewer"
  "agents.codebase-locator"
  "agents.codebase-analyser"
  "agents.codebase-pattern-finder"
  "agents.documents-locator"
  "agents.documents-analyser"
  "agents.web-search-researcher"
)

AGENT_DEFAULTS=(
  "${AGENT_PREFIX}reviewer"
  "${AGENT_PREFIX}codebase-locator"
  "${AGENT_PREFIX}codebase-analyser"
  "${AGENT_PREFIX}codebase-pattern-finder"
  "${AGENT_PREFIX}documents-locator"
  "${AGENT_PREFIX}documents-analyser"
  "${AGENT_PREFIX}web-search-researcher"
)

echo "## Effective Configuration"
echo ""
echo "| Key | Value | Source |"
echo "|-----|-------|--------|"

for key in "${REVIEW_KEYS[@]}"; do
  default="${DEFAULTS[$key]}"
  value=$("$READ_VALUE" "$key" "$default")
  source=$(get_source "$key")
  echo "| \`$key\` | \`$value\` | $source |"
done

for i in "${!AGENT_KEYS[@]}"; do
  key="${AGENT_KEYS[$i]}"
  default="${AGENT_DEFAULTS[$i]}"
  value=$("$READ_VALUE" "$key" "$default")
  source=$(get_source "$key")
  echo "| \`$key\` | \`$value\` | $source |"
done

# Path keys (defined in config-defaults.sh)
for i in "${!PATH_KEYS[@]}"; do
  key="${PATH_KEYS[$i]}"
  default="${PATH_DEFAULTS[$i]}"
  value=$("$READ_VALUE" "$key" "$default")
  source=$(get_source "$key")
  echo "| \`$key\` | \`$value\` | $source |"
done

# Template keys (defined in config-defaults.sh)
for key in "${TEMPLATE_KEYS[@]}"; do
  value=$("$READ_VALUE" "$key" "")
  source=$(get_source "$key")
  if [ -n "$value" ]; then
    echo "| \`$key\` | \`$value\` | $source |"
  else
    echo "| \`$key\` | *(not set)* | default |"
  fi
done
