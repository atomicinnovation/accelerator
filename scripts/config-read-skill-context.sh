#!/usr/bin/env bash
set -euo pipefail

# Reads skill-specific context from the per-skill customisation directory.
# Outputs the content wrapped in a section header, or nothing if no file
# exists.
#
# Usage: config-read-skill-context.sh <skill-name>
#
# Looks for: <project-root>/.claude/accelerator/skills/<skill-name>/context.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

SKILL_NAME="${1:-}"
if [ -z "$SKILL_NAME" ]; then
  echo "Usage: config-read-skill-context.sh <skill-name>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)
CONTEXT_FILE="$PROJECT_ROOT/.claude/accelerator/skills/$SKILL_NAME/context.md"

[ -f "$CONTEXT_FILE" ] || exit 0

CONTENT=$(config_trim_body < "$CONTEXT_FILE")
[ -z "$CONTENT" ] && exit 0

echo "## Skill-Specific Context"
echo ""
echo "The following context is specific to the $SKILL_NAME skill. Apply this"
echo "context in addition to any project-wide context above."
echo ""
printf '%s\n' "$CONTENT"
