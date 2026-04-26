#!/usr/bin/env bash
# DESCRIPTION: Rename tickets/work-item terminology in meta/ and config files
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

CONFIG_TEAM="$PROJECT_ROOT/.claude/accelerator.md"
CONFIG_LOCAL="$PROJECT_ROOT/.claude/accelerator.local.md"

# ── Step 1: Resolve user paths ───────────────────────────────────────────────

# Check for malformed frontmatter BEFORE any mutations
for cfg in "$CONFIG_TEAM" "$CONFIG_LOCAL"; do
  [ -f "$cfg" ] || continue
  if ! config_extract_frontmatter "$cfg" > /dev/null 2>&1; then
    echo "Error: malformed frontmatter in $cfg — cannot proceed." >&2
    echo "Fix the config file and re-run /accelerator:migrate." >&2
    exit 1
  fi
done

# Read old paths.tickets / paths.review_tickets from user config (old key names)
pinned_tickets="$(cd "$PROJECT_ROOT" \
  && bash "$PLUGIN_ROOT/scripts/config-read-value.sh" paths.tickets "" 2>/dev/null || true)"
pinned_review_tickets="$(cd "$PROJECT_ROOT" \
  && bash "$PLUGIN_ROOT/scripts/config-read-value.sh" paths.review_tickets "" 2>/dev/null || true)"

if [ -z "$pinned_tickets" ] || [ "$pinned_tickets" = "meta/tickets" ]; then
  tickets_dir="$PROJECT_ROOT/meta/tickets"
  tickets_is_default=1
else
  tickets_dir="$PROJECT_ROOT/$pinned_tickets"
  tickets_is_default=0
fi

if [ -z "$pinned_review_tickets" ] || [ "$pinned_review_tickets" = "meta/reviews/tickets" ]; then
  review_tickets_dir="$PROJECT_ROOT/meta/reviews/tickets"
  review_tickets_is_default=1
else
  review_tickets_dir="$PROJECT_ROOT/$pinned_review_tickets"
  review_tickets_is_default=0
fi

work_dir="$PROJECT_ROOT/meta/work"
review_work_dir="$PROJECT_ROOT/meta/reviews/work"

# ── Step 2: Frontmatter rewrites (idempotent) ────────────────────────────────

if [ -d "$tickets_dir" ]; then
  while IFS= read -r -d '' file; do
    if grep -q '^ticket_id:' "$file" 2>/dev/null; then
      if grep -q '^work_item_id:' "$file" 2>/dev/null; then
        # Both keys present (partial prior rewrite) — remove ticket_id: line
        grep -v '^ticket_id:' "$file" > "$file.tmp"
        mv "$file.tmp" "$file"
      else
        # Only ticket_id: — rename it
        sed 's/^ticket_id:/work_item_id:/' "$file" > "$file.tmp"
        mv "$file.tmp" "$file"
      fi
    fi
  done < <(find "$tickets_dir" -name '*.md' -print0)
fi

# ── Step 3: Collision check ──────────────────────────────────────────────────

if [ "$tickets_is_default" -eq 1 ] \
    && [ -d "$tickets_dir" ] \
    && [ -d "$work_dir" ]; then
  echo "Error: Both $tickets_dir and $work_dir exist — cannot proceed." >&2
  echo "Manually merge or remove one of them, then re-run /accelerator:migrate." >&2
  exit 1
fi

if [ "$review_tickets_is_default" -eq 1 ] \
    && [ -d "$review_tickets_dir" ] \
    && [ -d "$review_work_dir" ]; then
  echo "Error: Both $review_tickets_dir and $review_work_dir exist — cannot proceed." >&2
  echo "Manually merge or remove one of them, then re-run /accelerator:migrate." >&2
  exit 1
fi

# ── Step 4: Directory renames (default paths only) ───────────────────────────

if [ "$tickets_is_default" -eq 1 ]; then
  if [ -d "$tickets_dir" ]; then
    # source exists: attempt mv.
    # If target is already a directory, the collision check (step 3) already aborted.
    # If target is a regular file, mv fails here → script exits non-zero (retry-safe).
    # If target is absent, mv succeeds normally.
    mv "$tickets_dir" "$work_dir"
  fi
  # source absent: already migrated or fresh repo — no-op
fi

if [ "$review_tickets_is_default" -eq 1 ]; then
  if [ -d "$review_tickets_dir" ]; then
    mkdir -p "$(dirname "$review_work_dir")"
    mv "$review_tickets_dir" "$review_work_dir"
  fi
fi

# ── Step 5: Config-key rewrites ──────────────────────────────────────────────

rewrite_config() {
  local cfg="$1"
  [ -f "$cfg" ] || return 0
  # Line-anchored sed rewrites cover both nested-YAML and flat-dotted forms.
  # Order matters: more-specific patterns (with value suffix) run before the
  # generic key-only pattern so the default value is updated to the new default.
  # Comments containing "tickets" are NOT matched (no leading whitespace + colon).
  sed -E \
    -e 's/^([[:space:]]+)tickets:[[:space:]]*meta\/tickets([[:space:]]*)$/\1work: meta\/work/' \
    -e 's/^([[:space:]]+)tickets:/\1work:/' \
    -e 's/^([[:space:]]+)review_tickets:[[:space:]]*meta\/reviews\/tickets([[:space:]]*)$/\1review_work: meta\/reviews\/work/' \
    -e 's/^([[:space:]]+)review_tickets:/\1review_work:/' \
    -e 's/^paths\.tickets:[[:space:]]*meta\/tickets([[:space:]]*)$/paths.work: meta\/work/' \
    -e 's/^paths\.tickets:/paths.work:/' \
    -e 's/^paths\.review_tickets:[[:space:]]*meta\/reviews\/tickets([[:space:]]*)$/paths.review_work: meta\/reviews\/work/' \
    -e 's/^paths\.review_tickets:/paths.review_work:/' \
    -e 's/^([[:space:]]+)ticket_revise_severity:/\1work_item_revise_severity:/' \
    -e 's/^([[:space:]]+)ticket_revise_major_count:/\1work_item_revise_major_count:/' \
    -e 's/^([[:space:]]+)min_lenses_ticket:/\1min_lenses_work_item:/' \
    -e 's/^review\.ticket_revise_severity:/review.work_item_revise_severity:/' \
    -e 's/^review\.ticket_revise_major_count:/review.work_item_revise_major_count:/' \
    -e 's/^review\.min_lenses_ticket:/review.min_lenses_work_item:/' \
    "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
}

rewrite_config "$CONFIG_TEAM"
rewrite_config "$CONFIG_LOCAL"
