#!/usr/bin/env bash
set -euo pipefail

# Mutates the tags field on a ticket: adds or removes a single tag.
# Usage: ticket-update-tags.sh <ticket-path> add <tag>
#        ticket-update-tags.sh <ticket-path> remove <tag>
# Outputs:
#   - The new canonical array string on success (e.g. [api, search, backend])
#   - "no-change" if the mutation is a no-op (duplicate add or absent remove)
# Exit codes:
#   0 — success or no-change
#   1 — validation error (missing file, bad frontmatter, block-style tags)

TICKET_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$TICKET_SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/config-common.sh"

if [ $# -lt 3 ]; then
  echo "Usage: ticket-update-tags.sh <ticket-path> add|remove <tag>" >&2
  exit 1
fi

TICKET_FILE="$1"
ACTION="$2"
TAG="$3"

if [ ! -f "$TICKET_FILE" ]; then
  echo "Error: file not found: $TICKET_FILE" >&2
  exit 1
fi

FIRST_LINE=$(head -n 1 "$TICKET_FILE")
if ! [[ "$FIRST_LINE" =~ ^---[[:space:]]*$ ]]; then
  echo "Error: No YAML frontmatter in $(basename "$TICKET_FILE"). Add a '---' line as the first line of the file." >&2
  exit 1
fi

FRONTMATTER=$(config_extract_frontmatter "$TICKET_FILE") || {
  echo "Error: YAML frontmatter opened but not closed in $(basename "$TICKET_FILE"). Add a '---' line after the last frontmatter key." >&2
  exit 1
}

# Block-style detection: check the raw frontmatter for a tags line
# followed by an indented list item.
TAGS_LINE=""
TAGS_LINE_NUM=0
LINE_NUM=0
NEXT_LINE=""
while IFS= read -r line; do
  LINE_NUM=$((LINE_NUM + 1))
  if [ "$TAGS_LINE_NUM" -gt 0 ] && [ -z "$NEXT_LINE" ]; then
    NEXT_LINE="$line"
    break
  fi
  if [[ "$line" =~ ^tags: ]]; then
    TAGS_LINE="$line"
    TAGS_LINE_NUM=$LINE_NUM
  fi
done <<< "$FRONTMATTER"

if [ -n "$TAGS_LINE" ]; then
  TAGS_VALUE_PART="${TAGS_LINE#tags:}"
  TAGS_VALUE_TRIMMED=$(echo "$TAGS_VALUE_PART" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  if [ -z "$TAGS_VALUE_TRIMMED" ] && [[ "$NEXT_LINE" =~ ^[[:space:]]+- ]]; then
    echo "Error: tags field is in block format — convert to tags: [...] first. Example: tags: [api, search]" >&2
    exit 1
  fi
fi

# Read the current tags value via ticket-read-field.sh
FIELD_ABSENT=false
RAW_TAGS=$("$TICKET_SCRIPT_DIR/ticket-read-field.sh" tags "$TICKET_FILE" 2>/dev/null) || FIELD_ABSENT=true

# Parse current tags into an array
declare -a CURRENT_TAGS=()
if [ "$FIELD_ABSENT" = false ] && [ -n "$RAW_TAGS" ]; then
  STRIPPED="${RAW_TAGS#\[}"
  STRIPPED="${STRIPPED%\]}"
  if [ -n "$STRIPPED" ]; then
    while IFS= read -r item; do
      item=$(echo "$item" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
      # Strip surrounding quotes from parsed items
      item=$(echo "$item" | sed "s/^[\"']//;s/[\"']$//")
      [ -n "$item" ] && CURRENT_TAGS+=("$item")
    done < <(echo "$STRIPPED" | tr ',' '\n')
  fi
fi

# Check if a tag needs quoting (contains comma, colon, or hash)
needs_quoting() {
  local val="$1"
  if [[ "$val" == *,* ]] || [[ "$val" == *:* ]] || [[ "$val" == *#* ]]; then
    return 0
  fi
  return 1
}

# Format a tag value, quoting if necessary
format_tag() {
  local val="$1"
  if needs_quoting "$val"; then
    echo "\"$val\""
  else
    echo "$val"
  fi
}

# Build the canonical array string from an array of tags
build_canonical() {
  local result="["
  local first=true
  for t in "$@"; do
    if [ "$first" = true ]; then
      first=false
    else
      result+=", "
    fi
    result+="$(format_tag "$t")"
  done
  result+="]"
  echo "$result"
}

case "$ACTION" in
  add)
    # Check for duplicate
    for existing in "${CURRENT_TAGS[@]+"${CURRENT_TAGS[@]}"}"; do
      if [ "$existing" = "$TAG" ]; then
        echo "no-change"
        exit 0
      fi
    done
    # Append
    CURRENT_TAGS+=("$TAG")
    build_canonical "${CURRENT_TAGS[@]}"
    ;;
  remove)
    if [ "$FIELD_ABSENT" = true ]; then
      echo "no-change"
      exit 0
    fi
    # Check if tag exists
    FOUND=false
    declare -a NEW_TAGS=()
    for existing in "${CURRENT_TAGS[@]+"${CURRENT_TAGS[@]}"}"; do
      if [ "$existing" = "$TAG" ]; then
        FOUND=true
      else
        NEW_TAGS+=("$existing")
      fi
    done
    if [ "$FOUND" = false ]; then
      echo "no-change"
      exit 0
    fi
    build_canonical "${NEW_TAGS[@]+"${NEW_TAGS[@]}"}"
    ;;
  *)
    echo "Usage: ticket-update-tags.sh <ticket-path> add|remove <tag>" >&2
    exit 1
    ;;
esac
