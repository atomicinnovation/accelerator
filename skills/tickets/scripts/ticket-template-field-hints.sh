#!/usr/bin/env bash
set -euo pipefail

# Extracts hint values for a frontmatter field from the ticket template.
# Usage: ticket-template-field-hints.sh <field>
# Outputs one value per line, parsed from the template's trailing comment.
# Falls back to hardcoded defaults for type/status/priority if the template
# comment is absent or the template cannot be read.
# Exit code: always 0.

TICKET_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$TICKET_SCRIPT_DIR/../../.." && pwd)"

if [ $# -lt 1 ]; then
  echo "Usage: ticket-template-field-hints.sh <field>" >&2
  exit 1
fi

FIELD="$1"

# Hardcoded fallback values matching the shipping template's trailing comments
hardcoded_fallback() {
  local field="$1"
  case "$field" in
    type)
      echo "story"
      echo "epic"
      echo "task"
      echo "bug"
      echo "spike"
      ;;
    status)
      echo "draft"
      echo "ready"
      echo "in-progress"
      echo "review"
      echo "done"
      echo "blocked"
      echo "abandoned"
      ;;
    priority)
      echo "high"
      echo "medium"
      echo "low"
      ;;
    *)
      ;;
  esac
}

# Try to read the template
TEMPLATE_OUTPUT=""
TEMPLATE_OUTPUT=$("$PLUGIN_ROOT/scripts/config-read-template.sh" ticket 2>/dev/null) || {
  hardcoded_fallback "$FIELD"
  exit 0
}

# Find the field line in the template frontmatter.
# The template output is wrapped in code fences by config-read-template.sh,
# so we need to look inside the fenced content.
FIELD_LINE=""
while IFS= read -r line; do
  if [[ "$line" =~ ^${FIELD}: ]]; then
    FIELD_LINE="$line"
    break
  fi
done <<< "$TEMPLATE_OUTPUT"

if [ -z "$FIELD_LINE" ]; then
  hardcoded_fallback "$FIELD"
  exit 0
fi

# Check for trailing comment (everything after the first #)
if [[ "$FIELD_LINE" != *"#"* ]]; then
  hardcoded_fallback "$FIELD"
  exit 0
fi

COMMENT="${FIELD_LINE#*#}"

# Split on | and trim each token
echo "$COMMENT" | tr '|' '\n' | while IFS= read -r token; do
  token=$(echo "$token" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  [ -n "$token" ] && echo "$token"
done

exit 0
