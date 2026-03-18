#!/usr/bin/env bash
set -euo pipefail

# Reads the status field from an ADR file's YAML frontmatter.
# Usage: adr-read-status.sh <path-to-adr-file>
# Outputs the status value (e.g., "proposed", "accepted").
# Exits with code 1 if file not found or no valid frontmatter.

if [ $# -lt 1 ]; then
  echo "Usage: adr-read-status.sh <adr-file-path>" >&2
  exit 1
fi

ADR_FILE="$1"

if [ ! -f "$ADR_FILE" ]; then
  echo "Error: File not found: $ADR_FILE" >&2
  exit 1
fi

# Extract YAML frontmatter (between first two --- lines)
# and find the status field. Requires properly closed frontmatter.
IN_FRONTMATTER=false
FRONTMATTER_CLOSED=false
FOUND_STATUS=false
STATUS_VALUE=""
while IFS= read -r line; do
  if [ "$line" = "---" ]; then
    if [ "$IN_FRONTMATTER" = true ]; then
      FRONTMATTER_CLOSED=true
      break
    fi
    IN_FRONTMATTER=true
    continue
  fi
  if [ "$IN_FRONTMATTER" = true ]; then
    if echo "$line" | grep -qE '^status:'; then
      # Strip key, whitespace, and optional quotes
      STATUS_VALUE=$(echo "$line" | sed 's/^status:[[:space:]]*//' | sed 's/^["'"'"']//; s/["'"'"']$//' | sed 's/[[:space:]]*$//')
      FOUND_STATUS=true
    fi
  fi
done < "$ADR_FILE"

if [ "$FRONTMATTER_CLOSED" = true ] && [ "$FOUND_STATUS" = true ]; then
  echo "$STATUS_VALUE"
  exit 0
fi

echo "Error: No status field found in frontmatter of $(basename "$ADR_FILE")." >&2
echo "Expected YAML frontmatter with 'status: proposed|accepted|rejected|superseded|deprecated' between --- delimiters." >&2
exit 1
