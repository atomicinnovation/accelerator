#!/usr/bin/env bash
set -euo pipefail

# Reads a template file, checking user overrides before plugin defaults.
# Usage: config-read-template.sh <template_name>
#
# Template names: plan, research, adr, validation, pr-description
# (Invalid names produce an error listing available templates.)
#
# Resolution order:
# 1. Path specified in config: templates.<name> (if set and file exists)
# 2. Configured templates directory: <paths.templates>/<name>.md
#    (defaults to meta/templates/<name>.md)
# 3. Plugin default: <plugin_root>/templates/<name>.md
#
# All user-facing templates live in one place (meta/templates/ or whatever
# paths.templates is set to). The .claude/accelerator/ directory is only
# used for custom lenses (Plan 3), not templates.
#
# Outputs the template content to stdout, wrapped in markdown code fences
# (```markdown ... ```) so the LLM interprets the content as a template to
# follow rather than instructions to execute. If the template file already
# starts with a code fence, it is output as-is (no double-wrapping).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-read-template.sh <template_name>" >&2
  exit 1
fi

# Output template content, wrapping in code fences if not already fenced.
_output_template() {
  local file="$1"
  local first_line
  first_line=$(head -1 "$file")
  if [[ "$first_line" == '```'* ]]; then
    # Already fenced — output as-is
    cat "$file"
  else
    # Wrap in code fences
    echo '```markdown'
    cat "$file"
    echo '```'
  fi
}

# Resolve template through three-tier fallback
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: $AVAILABLE" >&2
  exit 1
}

IFS=$'\t' read -r _SOURCE RESOLVED_PATH <<< "$RESOLUTION"
_output_template "$RESOLVED_PATH"
