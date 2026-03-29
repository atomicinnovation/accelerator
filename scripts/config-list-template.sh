#!/usr/bin/env bash
set -euo pipefail

# Lists all available templates with their resolution source and path.
# Usage: config-list-template.sh
#
# For each template key, shows:
#   - The key name
#   - The resolution source (config path / user override / plugin default)
#   - The resolved file path
#
# Outputs a markdown table to stdout.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "| Template | Source | Path |"
echo "|----------|--------|------|"

for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do
  RESOLUTION=$(config_resolve_template "$KEY" "$PLUGIN_ROOT" 2>/dev/null) || true

  if [ -n "$RESOLUTION" ]; then
    IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"
    DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")
  else
    RESOLVED_SOURCE="not found"
    DISPLAY_PATH="—"
  fi

  echo "| \`$KEY\` | $RESOLVED_SOURCE | \`$DISPLAY_PATH\` |"
done
