#!/usr/bin/env bash
# DESCRIPTION: Rename work-item type field to kind in frontmatter and body labels.
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

work_dir_rel="$(cd "$PROJECT_ROOT" \
  && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work)"

if [ -z "$work_dir_rel" ]; then
  log_die "0005: config-read-path.sh returned empty for 'work'"
fi

case "$work_dir_rel" in
  .|..|/|/*|*/..|../*|*/../*)
    log_die "0005: refusing dangerous paths.work value: $work_dir_rel"
    ;;
esac

work_dir="$PROJECT_ROOT/$work_dir_rel"

if [ ! -d "$work_dir" ]; then
  log_warn "0005: work directory does not exist: $work_dir_rel"
  echo "0005: rewrote 0 file(s) under $work_dir_rel"
  exit 0
fi

rewrote=0
while IFS= read -r -d '' file; do
  touched=0

  # Pass 1: frontmatter key
  if grep -q '^type:' "$file" 2>/dev/null; then
    if grep -q '^kind:' "$file" 2>/dev/null; then
      old_type=$(grep -m 1 '^type:' "$file" | sed 's/^type:[[:space:]]*//')
      old_kind=$(grep -m 1 '^kind:' "$file" | sed 's/^kind:[[:space:]]*//')
      if [ "$old_type" != "$old_kind" ]; then
        log_warn "0005: divergent type/kind in $file — kept kind=$old_kind, dropped type=$old_type"
      fi
      grep -v '^type:' "$file" | atomic_write "$file"
    else
      sed 's/^type:/kind:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  # Pass 2: body label
  if grep -q '^\*\*Type\*\*:' "$file" 2>/dev/null; then
    if grep -q '^\*\*Kind\*\*:' "$file" 2>/dev/null; then
      old_type_body=$(grep -m 1 '^\*\*Type\*\*:' "$file" | sed 's/^\*\*Type\*\*:[[:space:]]*//')
      old_kind_body=$(grep -m 1 '^\*\*Kind\*\*:' "$file" | sed 's/^\*\*Kind\*\*:[[:space:]]*//')
      if [ "$old_type_body" != "$old_kind_body" ]; then
        log_warn "0005: divergent **Type**/**Kind** body label in $file — kept Kind=$old_kind_body, dropped Type=$old_type_body"
      fi
      grep -v '^\*\*Type\*\*:' "$file" | atomic_write "$file"
    else
      sed 's/^\*\*Type\*\*:/**Kind**:/' "$file" | atomic_write "$file"
    fi
    touched=1
  fi

  rewrote=$((rewrote + touched))
done < <(find "$work_dir" -name '*.md' -print0)

echo "0005: rewrote $rewrote file(s) under $work_dir_rel"
