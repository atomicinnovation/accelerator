#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"

# ── 1. Resolve PROJECT_ROOT ──────────────────────────────────────────────────
PROJECT_ROOT="${PROJECT_ROOT:-$(config_project_root)}"

# ── 2. Pre-flight: clean working tree check ──────────────────────────────────
if [ -z "${ACCELERATOR_MIGRATE_FORCE:-}" ]; then
  vcs=""
  if [ -d "$PROJECT_ROOT/.jj" ]; then
    vcs="jj"
  elif [ -d "$PROJECT_ROOT/.git" ]; then
    vcs="git"
  fi

  dirty=""
  if [ "$vcs" = "jj" ]; then
    if command -v jj &>/dev/null; then
      dirty=$(jj --no-pager diff --name-only 2>/dev/null \
        | grep -E '^(meta/|\.claude/accelerator)' || true)
    fi
  elif [ "$vcs" = "git" ]; then
    dirty=$(git -C "$PROJECT_ROOT" status --porcelain \
        "meta/" ".claude/accelerator.md" ".claude/accelerator.local.md" \
        2>/dev/null | grep -v '^??' || true)
  fi

  if [ -n "$dirty" ]; then
    echo "Error: dirty working tree — uncommitted changes detected in meta/ or" \
         ".claude/accelerator*.md." >&2
    echo "Commit or discard those changes first, or set" \
         "ACCELERATOR_MIGRATE_FORCE=1 to skip this check." >&2
    exit 1
  fi
fi

# ── 3. Read state file ───────────────────────────────────────────────────────
STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"
applied_ids=()
if [ -f "$STATE_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] && applied_ids+=("$line")
  done < "$STATE_FILE"
fi

# ── 4. Glob bundled migrations ───────────────────────────────────────────────
MIGRATIONS_DIR="${ACCELERATOR_MIGRATIONS_DIR:-$PLUGIN_ROOT/skills/config/migrate/migrations}"
migration_files=()
while IFS= read -r -d '' f; do
  migration_files+=("$f")
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 \
  2>/dev/null | sort -z) || true

# ── 5. Warn about unknown applied IDs ────────────────────────────────────────
known_ids=()
for f in "${migration_files[@]+"${migration_files[@]}"}"; do
  known_ids+=("$(basename "$f" .sh)")
done

for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
  found=0
  for known_id in "${known_ids[@]+"${known_ids[@]}"}"; do
    [ "$applied_id" = "$known_id" ] && found=1 && break
  done
  if [ "$found" -eq 0 ]; then
    echo "[warning] meta/.migrations-applied references unknown migration" \
         "$applied_id — preserved on rewrite" >&2
  fi
done

# ── 6. Compute pending list ──────────────────────────────────────────────────
pending_files=()
for f in "${migration_files[@]+"${migration_files[@]}"}"; do
  id="$(basename "$f" .sh)"
  is_applied=0
  for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
    [ "$applied_id" = "$id" ] && is_applied=1 && break
  done
  [ "$is_applied" -eq 0 ] && pending_files+=("$f")
done

# ── 7. Print preview ─────────────────────────────────────────────────────────
if [ ${#pending_files[@]} -eq 0 ]; then
  echo "No pending migrations."
  exit 0
fi

echo "Pending migrations:"
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  description=$(grep '^# DESCRIPTION:' "$f" | head -1 \
    | sed 's/^# DESCRIPTION:[[:space:]]*//')
  echo "  [$id] $description"
done
echo ""

# ── 8. Apply each pending migration ─────────────────────────────────────────
applied_count=0
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  echo "[${id}] running" >&2
  export PROJECT_ROOT
  if ! PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$f" >&2 2>&1; then
    echo "[${id}] failed" >&2
    exit 1
  fi
  # Atomic append to state file
  mkdir -p "$PROJECT_ROOT/meta"
  {
    [ -f "$STATE_FILE" ] && cat "$STATE_FILE"
    echo "$id"
  } > "$STATE_FILE.tmp"
  mv "$STATE_FILE.tmp" "$STATE_FILE"
  echo "[${id}] applied" >&2
  applied_count=$((applied_count + 1))
done

# ── 9. Summary ───────────────────────────────────────────────────────────────
echo ""
echo "Migration complete. $applied_count migration(s) applied."
