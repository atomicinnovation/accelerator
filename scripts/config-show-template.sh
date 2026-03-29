#!/usr/bin/env bash
set -euo pipefail

# Shows a template's content with source metadata.
# Usage: config-show-template.sh <template_name>
#
# Outputs the template source information followed by the raw content.
# Unlike config-read-template.sh, does NOT wrap in code fences.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-show-template.sh <template_name>" >&2
  exit 1
fi

RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: $AVAILABLE" >&2
  exit 1
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"
DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

echo "Source: $RESOLVED_SOURCE ($DISPLAY_PATH)"
echo "---"
cat "$RESOLVED_PATH"
