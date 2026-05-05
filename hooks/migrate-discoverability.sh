#!/usr/bin/env bash

# SessionStart hook: warn when meta/.migrations-applied lags the bundled
# migrations. Exits 0 in all cases — informational only.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../scripts/config-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT=$(config_project_root 2>/dev/null || true)
fi
if [ -z "$PROJECT_ROOT" ]; then
  exit 0
fi

# Only fire for repos that appear to use Accelerator
if [ ! -d "$PROJECT_ROOT/.accelerator" ] \
    && [ ! -f "$PROJECT_ROOT/.claude/accelerator.md" ] \
    && [ ! -d "$PROJECT_ROOT/meta" ]; then
  exit 0
fi

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
MIGRATIONS_DIR="$PLUGIN_ROOT/skills/config/migrate/migrations"

# Exist-aware fallback: prefer the post-migration path when its file exists,
# fall back to the legacy path for un-migrated or partial-recovery repos.
# Per-file existence check (not per-directory) so a partial-recovery repo
# where .accelerator/ exists but its state file does not falls through to
# the legacy path rather than treating the new branch as authoritative-but-empty.
# This fallback chain is a deprecation-track shim — the only place runtime code
# reads legacy paths. A follow-up work item tracks sunset criteria.
NEW_STATE_FILE="$PROJECT_ROOT/.accelerator/state/migrations-applied"
LEGACY_STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"
if [ -f "$NEW_STATE_FILE" ]; then
  STATE_FILE="$NEW_STATE_FILE"
else
  STATE_FILE="$LEGACY_STATE_FILE"
fi

# Determine highest available migration ID (lex-max of 0001-* prefix)
highest_available=""
while IFS= read -r -d '' f; do
  id="$(basename "$f" .sh)"
  if [ -z "$highest_available" ] || [ "$id" \> "$highest_available" ]; then
    highest_available="$id"
  fi
done < <(find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 \
  2>/dev/null | sort -z) || true

# No bundled migrations at all → nothing to warn about
[ -z "$highest_available" ] && exit 0

# Determine highest applied migration ID
highest_applied=""
if [ -f "$STATE_FILE" ]; then
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    if [ -z "$highest_applied" ] || [ "$line" \> "$highest_applied" ]; then
      highest_applied="$line"
    fi
  done < "$STATE_FILE"
fi

# Warn if applied < available (or state file absent while migrations exist)
if [ -z "$highest_applied" ] || [ "$highest_applied" \< "$highest_available" ]; then
  applied_label="${highest_applied:-(none)}"
  echo "[accelerator] $STATE_FILE is behind the plugin" \
       "(highest applied: $applied_label; highest available: $highest_available)." \
       "Run /accelerator:migrate to bring it up to date." >&2
fi

exit 0
