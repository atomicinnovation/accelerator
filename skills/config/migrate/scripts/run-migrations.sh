#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"
# shellcheck source=../../../../scripts/atomic-common.sh
source "$PLUGIN_ROOT/scripts/atomic-common.sh"

# ── 1. Resolve PROJECT_ROOT ──────────────────────────────────────────────────
PROJECT_ROOT="${PROJECT_ROOT:-$(config_project_root)}"

STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"
SKIP_FILE="$PROJECT_ROOT/meta/.migrations-skipped"

# ── --skip / --unskip flags ──────────────────────────────────────────────────
if [ $# -gt 0 ]; then
  case "$1" in
    --skip)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --skip <migration-id>" >&2
        exit 1
      fi
      mkdir -p "$PROJECT_ROOT/meta"
      atomic_append_unique "$SKIP_FILE" "$2"
      echo "Skipped migration: $2"
      exit 0
      ;;
    --unskip)
      if [ $# -lt 2 ]; then
        echo "Usage: run-migrations.sh --unskip <migration-id>" >&2
        exit 1
      fi
      atomic_remove_line "$SKIP_FILE" "$2"
      echo "Unskipped migration: $2"
      exit 0
      ;;
  esac
fi

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

# ── 3. Read state files ──────────────────────────────────────────────────────
applied_ids=()
if [ -f "$STATE_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] && applied_ids+=("$line")
  done < "$STATE_FILE"
fi

skipped_ids=()
if [ -f "$SKIP_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] && skipped_ids+=("$line")
  done < "$SKIP_FILE"
fi

# ── 4. Glob bundled migrations ───────────────────────────────────────────────
MIGRATIONS_DIR="${ACCELERATOR_MIGRATIONS_DIR:-$PLUGIN_ROOT/skills/config/migrate/migrations}"
migration_files=()
while IFS= read -r -d '' f; do
  migration_files+=("$f")
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 \
  2>/dev/null | sort -z) || true

# ── 5. Warn about unknown applied or skipped IDs ─────────────────────────────
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

for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
  found=0
  for known_id in "${known_ids[@]+"${known_ids[@]}"}"; do
    [ "$skipped_id" = "$known_id" ] && found=1 && break
  done
  if [ "$found" -eq 0 ]; then
    echo "[warning] meta/.migrations-skipped references unknown migration" \
         "$skipped_id — preserved on rewrite" >&2
  fi
done

# Cross-state inconsistency: ID in both applied and skipped
for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
  for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
    if [ "$applied_id" = "$skipped_id" ]; then
      echo "[warning] migration $applied_id appears in BOTH .migrations-applied" \
           "and .migrations-skipped — applied takes precedence" >&2
    fi
  done
done

# ── 6. Compute pending list ──────────────────────────────────────────────────
pending_files=()
for f in "${migration_files[@]+"${migration_files[@]}"}"; do
  id="$(basename "$f" .sh)"
  is_applied=0
  for applied_id in "${applied_ids[@]+"${applied_ids[@]}"}"; do
    [ "$applied_id" = "$id" ] && is_applied=1 && break
  done
  is_skipped=0
  for skipped_id in "${skipped_ids[@]+"${skipped_ids[@]}"}"; do
    [ "$skipped_id" = "$id" ] && is_skipped=1 && break
  done
  if [ "$is_applied" -eq 0 ] && [ "$is_skipped" -eq 0 ]; then
    pending_files+=("$f")
  fi
done

# ── 7. Print preview ─────────────────────────────────────────────────────────
if [ ${#pending_files[@]} -eq 0 ]; then
  echo "No pending migrations."
  if [ "${#skipped_ids[@]}" -gt 0 ]; then
    echo "Skipped: $(printf '%s ' "${skipped_ids[@]}")"
  fi
  exit 0
fi

# Pre-run banner: surface the destructive-write expectation before any
# migration runs, so users facing future destructive migrations see the
# warning consistently. Pre-flight already enforces the clean-tree rule.
echo "About to apply ${#pending_files[@]} migration(s):"
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  description=$(grep '^# DESCRIPTION:' "$f" | head -1 \
    | sed 's/^# DESCRIPTION:[[:space:]]*//')
  echo "  $id — $description"
  echo "    To skip: bash $0 --skip $id"
done
echo ""
echo "Migrations rewrite files and may make repo-wide changes; commit"
echo "your working tree before running so VCS revert is available as"
echo "rollback. The pre-flight will refuse to run on a dirty tree"
echo "unless ACCELERATOR_MIGRATE_FORCE=1 is set."
echo ""

# ── 8. Apply each pending migration ─────────────────────────────────────────
applied_count=0
for f in "${pending_files[@]}"; do
  id="$(basename "$f" .sh)"
  echo "[${id}] running" >&2
  export PROJECT_ROOT
  STDOUT_FILE=$(mktemp)
  if ! PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" bash "$f" >"$STDOUT_FILE" 2>&1; then
    cat "$STDOUT_FILE" >&2
    rm -f "$STDOUT_FILE"
    echo "[${id}] failed" >&2
    exit 1
  fi
  # Inspect stdout for the no_op_pending sentinel
  NO_OP_PENDING=0
  if grep -qx 'MIGRATION_RESULT: no_op_pending' "$STDOUT_FILE"; then
    NO_OP_PENDING=1
  fi
  # Strip the sentinel from the user-visible output
  grep -v -x 'MIGRATION_RESULT: no_op_pending' "$STDOUT_FILE" >&2 || true
  rm -f "$STDOUT_FILE"

  if [ "$NO_OP_PENDING" -eq 1 ]; then
    echo "[${id}] no-op (stays pending)" >&2
    continue
  fi

  mkdir -p "$PROJECT_ROOT/meta"
  atomic_append_unique "$STATE_FILE" "$id"
  echo "[${id}] applied" >&2
  applied_count=$((applied_count + 1))
done

# ── 9. Summary ───────────────────────────────────────────────────────────────
echo ""
SUMMARY="applied: $applied_count"
if [ "${#skipped_ids[@]}" -gt 0 ]; then
  SUMMARY="$SUMMARY; skipped: $(printf '%s ' "${skipped_ids[@]}" | sed 's/ $//')"
fi
PENDING_REMAINING=$(( ${#pending_files[@]} - applied_count ))
if [ "$PENDING_REMAINING" -gt 0 ]; then
  SUMMARY="$SUMMARY; pending (no-op): $PENDING_REMAINING"
fi
echo "Migration complete. $SUMMARY."
