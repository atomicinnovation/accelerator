#!/usr/bin/env bash
set -euo pipefail

# Reads a named field from a work item file's YAML frontmatter.
# Usage: work-item-read-field.sh <field-name> <path-to-work-item-file>
# Outputs the raw field value (surrounding quotes are stripped).
# Exits with code 1 if the file is missing, frontmatter is missing or
# unclosed, or the field is not present.
#
# Own-identity alias: when the caller asks for `work_item_id` against a
# unified-shape file that only carries `id:`, the value of `id:` is
# returned. Symmetrically, when the caller asks for `id` against a
# legacy file that only carries `work_item_id:`, the legacy value is
# returned. This bridges the unified schema with work items written
# under the older shape until a corpus migration normalises them.
#
# Duplicate keys: first-match-wins (consistent with config-read-value.sh;
# diverges from adr-read-status.sh which currently returns last-match).
# Array values (e.g., `tags: [a, b]`) are returned verbatim — callers are
# responsible for parsing them (see config_parse_array in config-common.sh).

WORK_ITEM_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$WORK_ITEM_SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/config-common.sh"

if [ $# -lt 2 ]; then
  echo "Usage: work-item-read-field.sh <field-name> <work-item-file-path>" >&2
  exit 1
fi

FIELD_NAME="$1"
WORK_ITEM_FILE="$2"

if [ ! -f "$WORK_ITEM_FILE" ]; then
  echo "Error: File not found: $WORK_ITEM_FILE" >&2
  exit 1
fi

# Distinguish no-frontmatter from unclosed-frontmatter before calling the
# helper, so the error message can point at the specific problem.
FIRST_LINE=$(head -n 1 "$WORK_ITEM_FILE")
if ! [[ "$FIRST_LINE" =~ ^---[[:space:]]*$ ]]; then
  echo "Error: No YAML frontmatter in $(basename "$WORK_ITEM_FILE"). Add a '---' line as the first line of the file." >&2
  exit 1
fi

FRONTMATTER=$(config_extract_frontmatter "$WORK_ITEM_FILE") || {
  echo "Error: YAML frontmatter opened but not closed in $(basename "$WORK_ITEM_FILE"). Add a '---' line after the last frontmatter key." >&2
  exit 1
}

read_field() {
  local name="$1"
  local prefix="${name}:"
  local value=""
  local found=false
  while IFS= read -r line; do
    if [[ "$line" == "${prefix}"* ]]; then
      value="${line#"$prefix"}"
      # Order matters: strip both ends of whitespace BEFORE stripping
      # quotes, so trailing whitespace after a closing quote does not
      # leave the quote orphaned. Each command gets its own -e for
      # readability.
      value=$(echo "$value" |
        sed -e 's/^[[:space:]]*//' \
          -e 's/[[:space:]]*$//' \
          -e 's/^["'"'"']//' \
          -e 's/["'"'"']$//')
      found=true
      break
    fi
  done <<<"$FRONTMATTER"
  if [ "$found" = true ]; then
    printf '%s' "$value"
    return 0
  fi
  return 1
}

if value=$(read_field "$FIELD_NAME"); then
  echo "$value"
  exit 0
fi

# Own-identity fallback: if the caller asked for `id` against a legacy
# file, try `work_item_id`. If the caller asked for `work_item_id`
# against a unified file, try `id`. See the header comment for the full
# rationale.
if [ "$FIELD_NAME" = "id" ]; then
  if value=$(read_field "work_item_id"); then
    echo "$value"
    exit 0
  fi
elif [ "$FIELD_NAME" = "work_item_id" ]; then
  if value=$(read_field "id"); then
    echo "$value"
    exit 0
  fi
fi

echo "Error: No '$FIELD_NAME' field found in frontmatter of $(basename "$WORK_ITEM_FILE")." >&2
exit 1
