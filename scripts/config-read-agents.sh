#!/usr/bin/env bash
set -euo pipefail

# Reads agent name overrides from accelerator config files.
# Outputs a markdown instruction block listing overrides, or nothing if
# no overrides are configured.
#
# Usage: config-read-agents.sh
#
# Config format (in .accelerator/config.md or .accelerator/config.local.md):
#   ---
#   agents:
#     reviewer: my-custom-reviewer
#     codebase-locator: my-locator-agent
#   ---
#
# Agent config keys use the same hyphenated names as the agents themselves
# (e.g., codebase-locator, not codebase_locator).
#
# Performance: Extracts frontmatter once per config file and parses all
# agent keys in a single awk pass, rather than shelling out to
# config-read-value.sh per key (~20-30ms vs ~100-200ms).
#
# Note: The list of valid agent keys is also documented in the configure
# skill (skills/config/configure/SKILL.md). Update both when adding agents.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

# Valid agent names in display order. This is the canonical list of agents
# that can be overridden. Order determines table row order in output.
AGENT_KEYS=(
  reviewer
  browser-analyser
  browser-locator
  codebase-locator
  codebase-analyser
  codebase-pattern-finder
  documents-locator
  documents-analyser
  web-search-researcher
)

# Build a space-delimited string of valid keys for awk to reference.
VALID_KEYS_STR="${AGENT_KEYS[*]}"

# Parse all agent overrides from config files in a single pass per file.
# Uses last-writer-wins precedence: team config is read first, local config
# second. If both define the same key, the local value wins.
#
# Stores results in a newline-delimited string of "key=value" pairs to
# avoid bash 4+ associative arrays (macOS ships bash 3.2).
OVERRIDES=""

while IFS= read -r config_file; do
  fm=$(config_extract_frontmatter "$config_file") || continue
  [ -z "$fm" ] && continue

  # Single awk pass: extract all key-value pairs from the agents section,
  # and flag unrecognised keys.
  parsed=$(echo "$fm" | awk -v valid_keys="$VALID_KEYS_STR" '
    BEGIN { split(valid_keys, vk, " "); for (i in vk) valid[vk[i]] = 1 }
    /^agents:/ { in_section = 1; next }
    in_section && /^[^ \t]/ { exit }
    in_section && /^[ \t]+[a-zA-Z]/ {
      stripped = $0
      sub(/^[ \t]+/, "", stripped)
      key = stripped
      sub(/:.*/, "", key)
      val = stripped
      sub(/^[^:]+:[ \t]*/, "", val)
      # Strip optional surrounding quotes
      if (val ~ /^".*"$/ || val ~ /^'"'"'.*'"'"'$/) {
        val = substr(val, 2, length(val) - 2)
      }
      if (key in valid) {
        print "OVERRIDE:" key "=" val
      } else {
        print "WARN:" key
      }
    }
  ')

  # Process parsed output
  while IFS= read -r line; do
    case "$line" in
      OVERRIDE:*)
        pair="${line#OVERRIDE:}"
        key="${pair%%=*}"
        val="${pair#*=}"
        # Remove any previous override for this key (last-writer-wins)
        OVERRIDES=$(printf '%s\n' "$OVERRIDES" | grep -v "^${key}=" || true)
        OVERRIDES="${OVERRIDES}"$'\n'"${key}=${val}"
        ;;
      WARN:*)
        found_key="${line#WARN:}"
        echo "Warning: unknown agent key '$found_key' in $config_file — ignoring" >&2
        ;;
    esac
  done <<< "$parsed"
done < <(config_find_files)

# Build resolved name for each agent (override or default).
# Convert hyphenated keys to space-separated display names for consistency
# with path variable labels (e.g., "codebase locator" not "codebase-locator").
AGENT_LINES=""
for key in "${AGENT_KEYS[@]}"; do
  val=$(printf '%s\n' "$OVERRIDES" | grep "^${key}=" | tail -1 | sed 's/^[^=]*=//' || true)
  if [ -z "$val" ]; then
    val="${AGENT_PREFIX}$key"
  fi
  display_name="${key//-/ }"
  AGENT_LINES="${AGENT_LINES}- **${display_name} agent**: ${val}
"
done

# Always output agent names block (skills reference these variables).
echo "## Agent Names"
echo ""
echo "The following agent names are configured for this project. Always use"
echo "the name shown for each role as the \`subagent_type\` parameter when"
echo "spawning agents via the Agent/Task tool."
echo ""
printf '%s' "$AGENT_LINES"
