#!/usr/bin/env bash
set -euo pipefail

# Resets a user's customised template to the plugin default.
#
# Usage: config-reset-template.sh [--confirm] <template_name>
#
# Without --confirm: reports the override location (dry-run).
# With --confirm: deletes the override file.
#
# Exit codes:
#   0 - Override found (or successfully deleted with --confirm)
#   1 - Error (unknown template, usage error)
#   2 - No override exists (already using plugin default)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CONFIRM=false
TEMPLATE_NAME=""

while [ $# -gt 0 ]; do
  case "$1" in
    --confirm)
      CONFIRM=true
      shift
      ;;
    -*)
      echo "Error: unknown option '$1'" >&2
      exit 1
      ;;
    *)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: unexpected argument '$1'" >&2
        exit 1
      fi
      TEMPLATE_NAME="$1"
      shift
      ;;
  esac
done

if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-reset-template.sh [--confirm] <template_name>" >&2
  exit 1
fi

# Verify it's a known template
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ ! -f "$DEFAULT_PATH" ]; then
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Unknown template '$TEMPLATE_NAME'. Available: $AVAILABLE" >&2
  exit 1
fi

# Resolve the template
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  echo "No customised template found for '$TEMPLATE_NAME' — already using plugin default." >&2
  exit 2
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"

if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" ]; then
  echo "No customised template found for '$TEMPLATE_NAME' — already using plugin default." >&2
  exit 2
fi

# Check if the override file is outside the project root
PROJECT_ROOT=$(config_project_root)
OUTSIDE_PROJECT=false
if [[ "$RESOLVED_PATH" != "$PROJECT_ROOT"/* ]]; then
  OUTSIDE_PROJECT=true
fi

DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

if [ "$CONFIRM" = false ]; then
  echo "Found override: $RESOLVED_SOURCE"
  echo "Path: $DISPLAY_PATH"
  if [ "$OUTSIDE_PROJECT" = true ]; then
    echo "Warning: This file is outside the project directory ($RESOLVED_PATH)."
  fi
  if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" ]; then
    echo "Note: After deletion, also remove the 'templates.$TEMPLATE_NAME' entry from your config."
  fi
  exit 0
fi

# Delete the override
rm "$RESOLVED_PATH"
echo "Reset: $TEMPLATE_NAME"
if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" ]; then
  echo "Note: Also remove the 'templates.$TEMPLATE_NAME' entry from your config."
fi
