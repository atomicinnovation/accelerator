#!/usr/bin/env bash
# Initialise accelerator scaffold in the current project.
# Idempotent: safe to run repeatedly.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
CONFIG_READ_PATH="$PLUGIN_ROOT/scripts/config-read-path.sh"

PROJECT_ROOT="${PROJECT_ROOT:-$PWD}"
cd "$PROJECT_ROOT"

# Step 1: project-content directories under meta/
DIR_KEYS=(
  plans research decisions prs validations
  review_plans review_prs review_work
  templates work notes
  design_inventories design_gaps
  tmp
)
DIR_DEFAULTS=(
  meta/plans meta/research meta/decisions meta/prs meta/validations
  meta/reviews/plans meta/reviews/prs meta/reviews/work
  meta/templates meta/work meta/notes
  meta/design-inventories meta/design-gaps
  meta/tmp
)

for i in "${!DIR_KEYS[@]}"; do
  key="${DIR_KEYS[$i]}"
  default="${DIR_DEFAULTS[$i]}"
  dir=$(bash "$CONFIG_READ_PATH" "$key" "$default")
  mkdir -p "$dir"
  [ -e "$dir/.gitkeep" ] || touch "$dir/.gitkeep"
done

# Step 2: inner gitignore for tmp (ADR-0019 pattern)
TMP_DIR=$(bash "$CONFIG_READ_PATH" tmp meta/tmp)
TMP_GITIGNORE="$TMP_DIR/.gitignore"
if [ ! -f "$TMP_GITIGNORE" ]; then
  cat > "$TMP_GITIGNORE" <<'EOF'
*
!.gitkeep
!.gitignore
EOF
fi

# Step 3: root .gitignore append for .claude/accelerator.local.md
ROOT_GI="$PROJECT_ROOT/.gitignore"
RULE='.claude/accelerator.local.md'
touch "$ROOT_GI"
if ! grep -qFx "$RULE" "$ROOT_GI"; then
  printf '%s\n' "$RULE" >> "$ROOT_GI"
fi
