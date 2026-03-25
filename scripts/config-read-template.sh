#!/usr/bin/env bash
set -euo pipefail

# Reads a template file, checking user overrides before plugin defaults.
# Usage: config-read-template.sh <template_name>
#
# Template names: plan, research, adr, validation
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

PROJECT_ROOT=$(config_project_root)

# 1. Check config-specified path
CONFIG_PATH=$("$SCRIPT_DIR/config-read-value.sh" "templates.${TEMPLATE_NAME}" "")
if [ -n "$CONFIG_PATH" ]; then
  # Resolve relative to project root
  if [[ "$CONFIG_PATH" != /* ]]; then
    CONFIG_PATH="$PROJECT_ROOT/$CONFIG_PATH"
  fi
  if [ -f "$CONFIG_PATH" ]; then
    _output_template "$CONFIG_PATH"
    exit 0
  else
    echo "Warning: configured template path '$CONFIG_PATH' not found, falling back to defaults" >&2
  fi
fi

# 2. Check configured templates directory (paths.templates, default: meta/templates)
TEMPLATES_DIR=$("$SCRIPT_DIR/config-read-path.sh" templates meta/templates)
if [[ "$TEMPLATES_DIR" != /* ]]; then
  TEMPLATES_DIR="$PROJECT_ROOT/$TEMPLATES_DIR"
fi
if [ -f "$TEMPLATES_DIR/${TEMPLATE_NAME}.md" ]; then
  _output_template "$TEMPLATES_DIR/${TEMPLATE_NAME}.md"
  exit 0
fi

# 3. Fall back to plugin default
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ -f "$DEFAULT_PATH" ]; then
  _output_template "$DEFAULT_PATH"
  exit 0
fi

echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: plan, research, adr, validation" >&2
exit 1
