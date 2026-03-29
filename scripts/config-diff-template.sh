#!/usr/bin/env bash
set -euo pipefail

# Shows differences between a user's customised template and the plugin
# default.
#
# Usage: config-diff-template.sh <template_name>
#
# Exit codes:
#   0 - Diff shown successfully
#   1 - Error (unknown template, usage error, diff error)
#   2 - No user override exists (using plugin default)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-diff-template.sh <template_name>" >&2
  exit 1
fi

# Verify it's a known template
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ ! -f "$DEFAULT_PATH" ]; then
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Unknown template '$TEMPLATE_NAME'. Available: $AVAILABLE" >&2
  exit 1
fi

# Resolve the template — if it resolves to plugin default, there's no
# user override to diff against
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  echo "No customised template found for '$TEMPLATE_NAME' — using plugin default." >&2
  exit 2
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"

if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" ]; then
  echo "No customised template found for '$TEMPLATE_NAME' — using plugin default." >&2
  exit 2
fi

DISPLAY_DEFAULT=$(config_display_path "$DEFAULT_PATH" "$PLUGIN_ROOT")
DISPLAY_USER=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

echo "Comparing plugin default vs user override:"
echo "  Default: $DISPLAY_DEFAULT"
echo "  User:    $DISPLAY_USER"
echo ""

# diff exits 0 if identical, 1 if different, 2+ if trouble
RC=0
diff -u "$DEFAULT_PATH" "$RESOLVED_PATH" || RC=$?
if [ "$RC" -gt 1 ]; then
  exit 1
elif [ "$RC" -eq 0 ]; then
  echo "Templates are identical."
fi
