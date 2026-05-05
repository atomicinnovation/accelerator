#!/usr/bin/env bash
set -euo pipefail

# Reads skill-specific instructions from the per-skill customisation
# directory. Outputs the content wrapped in a section header, or nothing
# if no file exists.
#
# Usage: config-read-skill-instructions.sh <skill-name>
#
# Looks for:
#   <project-root>/.accelerator/skills/<skill-name>/instructions.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
config_assert_no_legacy_layout

SKILL_NAME="${1:-}"
if [ -z "$SKILL_NAME" ]; then
  echo "Usage: config-read-skill-instructions.sh <skill-name>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)
INSTRUCTIONS_FILE="$PROJECT_ROOT/.accelerator/skills/$SKILL_NAME/instructions.md"

[ -f "$INSTRUCTIONS_FILE" ] || exit 0

CONTENT=$(config_trim_body < "$INSTRUCTIONS_FILE")
[ -z "$CONTENT" ] && exit 0

echo "## Additional Instructions"
echo ""
echo "The following additional instructions have been provided for the"
echo "$SKILL_NAME skill. Follow these instructions in addition to all"
echo "instructions above."
echo ""
printf '%s\n' "$CONTENT"
