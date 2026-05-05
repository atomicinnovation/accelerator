#!/usr/bin/env bash
# Initialise accelerator scaffold in the current project.
# Idempotent: safe to run repeatedly.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
CONFIG_READ_PATH="$PLUGIN_ROOT/scripts/config-read-path.sh"

# shellcheck source=../../../../scripts/accelerator-scaffold.sh
source "$PLUGIN_ROOT/scripts/accelerator-scaffold.sh"

PROJECT_ROOT="${PROJECT_ROOT:-$PWD}"
cd "$PROJECT_ROOT"

# Step 1: project-content directories under meta/ (12 items)
DIR_KEYS=(
  plans research decisions prs validations
  review_plans review_prs review_work
  work notes
  design_inventories design_gaps
)
DIR_DEFAULTS=(
  meta/plans meta/research meta/decisions meta/prs meta/validations
  meta/reviews/plans meta/reviews/prs meta/reviews/work
  meta/work meta/notes
  meta/design-inventories meta/design-gaps
)

for i in "${!DIR_KEYS[@]}"; do
  key="${DIR_KEYS[$i]}"
  default="${DIR_DEFAULTS[$i]}"
  dir=$(bash "$CONFIG_READ_PATH" "$key" "$default")
  mkdir -p "$dir"
  [ -e "$dir/.gitkeep" ] || touch "$dir/.gitkeep"
done

# Step 2: .accelerator/ core scaffold via shared helpers
accelerator_ensure_inner_gitignore "$PROJECT_ROOT"
accelerator_ensure_state_dir "$PROJECT_ROOT"

# Step 2b: extension-point .gitkeep files (migration 0003 does not pre-create
# these because they receive moves and a pre-existing dir would break mv)
ACC_ROOT="$PROJECT_ROOT/.accelerator"
mkdir -p "$ACC_ROOT/skills" "$ACC_ROOT/lenses" "$ACC_ROOT/templates"
for d in skills lenses templates; do
  [ -e "$ACC_ROOT/$d/.gitkeep" ] || touch "$ACC_ROOT/$d/.gitkeep"
done

# Step 3: tmp directory and inner .gitignore (path may be overridden)
TMP_DIR=$(bash "$CONFIG_READ_PATH" tmp .accelerator/tmp)
mkdir -p "$TMP_DIR"
TMP_GITIGNORE="$TMP_DIR/.gitignore"
if [ ! -f "$TMP_GITIGNORE" ]; then
  cat > "$TMP_GITIGNORE" <<'EOF'
*
!.gitkeep
!.gitignore
EOF
fi
[ -e "$TMP_DIR/.gitkeep" ] || touch "$TMP_DIR/.gitkeep"

# Step 4: anchored root .gitignore rule
accelerator_ensure_root_gitignore_rule "$PROJECT_ROOT"
