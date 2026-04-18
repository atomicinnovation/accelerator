#!/usr/bin/env bash
set -euo pipefail

# Reads a named field from a ticket file's YAML frontmatter.
# Usage: ticket-read-field.sh <field-name> <path-to-ticket-file>
# Outputs the raw field value (surrounding quotes are stripped).
# Exits with code 1 if the file is missing, frontmatter is missing or
# unclosed, or the field is not present.
#
# Duplicate keys: first-match-wins (consistent with config-read-value.sh;
# diverges from adr-read-status.sh which currently returns last-match).
# Array values (e.g., `tags: [a, b]`) are returned verbatim — callers are
# responsible for parsing them (see config_parse_array in config-common.sh).

TICKET_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$TICKET_SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/config-common.sh"

if [ $# -lt 2 ]; then
  echo "Usage: ticket-read-field.sh <field-name> <ticket-file-path>" >&2
  exit 1
fi

FIELD_NAME="$1"
TICKET_FILE="$2"

if [ ! -f "$TICKET_FILE" ]; then
  echo "Error: File not found: $TICKET_FILE" >&2
  exit 1
fi

# Distinguish no-frontmatter from unclosed-frontmatter before calling the
# helper, so the error message can point at the specific problem.
FIRST_LINE=$(head -n 1 "$TICKET_FILE")
if ! [[ "$FIRST_LINE" =~ ^---[[:space:]]*$ ]]; then
  echo "Error: No YAML frontmatter in $(basename "$TICKET_FILE"). Add a '---' line as the first line of the file." >&2
  exit 1
fi

FRONTMATTER=$(config_extract_frontmatter "$TICKET_FILE") || {
  echo "Error: YAML frontmatter opened but not closed in $(basename "$TICKET_FILE"). Add a '---' line after the last frontmatter key." >&2
  exit 1
}

PREFIX="${FIELD_NAME}:"
FIELD_VALUE=""
FOUND_FIELD=false
while IFS= read -r line; do
  if [[ "$line" == "${PREFIX}"* ]]; then
    FIELD_VALUE="${line#"$PREFIX"}"
    # Order matters: strip both ends of whitespace BEFORE stripping quotes,
    # so trailing whitespace after a closing quote does not leave the quote
    # orphaned. Each command gets its own -e for readability.
    FIELD_VALUE=$(echo "$FIELD_VALUE" \
      | sed -e 's/^[[:space:]]*//' \
            -e 's/[[:space:]]*$//' \
            -e 's/^["'"'"']//' \
            -e 's/["'"'"']$//')
    FOUND_FIELD=true
    break
  fi
done <<< "$FRONTMATTER"

if [ "$FOUND_FIELD" = true ]; then
  echo "$FIELD_VALUE"
  exit 0
fi

echo "Error: No '$FIELD_NAME' field found in frontmatter of $(basename "$TICKET_FILE")." >&2
exit 1
