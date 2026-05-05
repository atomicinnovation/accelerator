#!/usr/bin/env bash
# DESCRIPTION: Relocate Accelerator-owned files from .claude/ and meta/ to .accelerator/
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"
source "$PLUGIN_ROOT/scripts/accelerator-scaffold.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

# ── Shared state ──────────────────────────────────────────────────────────────
MOVED_THIS_RUN=()

# Files inside .accelerator/state/integrations/jira/ that must not be committed.
# Must stay byte-equal to JIRA_INNER_GITIGNORE_RULES in jira-common.sh (added
# in Phase 5). Both copies are pinned to equality by a test in test-jira-paths.sh.
JIRA_INNER_GITIGNORE_RULES=(
  site.json
  .refresh-meta.json
  .lock/
)

# ── _move_if_pending <rel-src> <rel-dst> ─────────────────────────────────────
# Moves $PROJECT_ROOT/<rel-src> to $PROJECT_ROOT/<rel-dst> when the source
# exists and the destination does not. Idempotent for all other states.
# Exits non-zero with a reconciliation message when both paths exist and differ.
_move_if_pending() {
  local rel_src="$1" rel_dst="$2"
  local src="$PROJECT_ROOT/$rel_src"
  local dst="$PROJECT_ROOT/$rel_dst"

  # Both absent — nothing to do
  [ ! -e "$src" ] && [ ! -e "$dst" ] && return 0

  # Source absent, dest present — already moved on a prior run
  [ ! -e "$src" ] && [ -e "$dst" ] && return 0

  # Both present — conflict
  if [ -e "$src" ] && [ -e "$dst" ]; then
    printf '%s\n' \
      "accelerator migrate: conflict — both '$rel_src' and '$rel_dst' exist." \
      "Prior moves in this run: ${MOVED_THIS_RUN[*]:-(none)}" >&2
    if [ -d "$src" ] && [ -d "$dst" ]; then
      diff -r "$src" "$dst" >&2 || true
    else
      diff "$src" "$dst" >&2 || true
    fi
    printf '%s\n' \
      "Recover with: jj op restore / git reset, reconcile manually, then re-run." >&2
    exit 1
  fi

  # Source present, dest absent — move it
  local dst_parent
  dst_parent="$(dirname "$dst")"
  mkdir -p "$dst_parent"
  mv "$src" "$dst"
  MOVED_THIS_RUN+=("$rel_src → $rel_dst")
}

# ── _awk_probe_paths_key <key> (reads stdin) ──────────────────────────────────
# Extracts the value of paths.<key> from a config file on stdin.
# Anchors on a column-0 `paths:` block to avoid false positives from nested keys.
_awk_probe_paths_key() {
  local key="$1"
  awk -v key="$key" '
    /^paths:[[:space:]]*$/  { in_paths = 1; next }
    /^[^[:space:]]/         { in_paths = 0 }
    in_paths {
      regex = "^[[:space:]]+" key ":[[:space:]]*"
      if ($0 ~ regex) {
        sub(regex, "")
        print
        exit
      }
    }
  '
}

# ── probe_paths_key <key> ─────────────────────────────────────────────────────
# Returns the effective value of paths.<key> from the legacy config files
# (.claude/accelerator.md overridden by .claude/accelerator.local.md).
# Does NOT use config-read-path.sh — the migration is itself rewiring that script.
probe_paths_key() {
  local key="$1"
  local team_file="$PROJECT_ROOT/.claude/accelerator.md"
  local local_file="$PROJECT_ROOT/.claude/accelerator.local.md"
  local raw=""
  [ -f "$team_file" ] && raw=$(_awk_probe_paths_key "$key" < "$team_file")
  if [ -f "$local_file" ]; then
    local local_raw
    local_raw=$(_awk_probe_paths_key "$key" < "$local_file")
    [ -n "$local_raw" ] && raw="$local_raw"
  fi
  # Strip trailing comment and whitespace before returning.
  printf '%s\n' "$raw" | sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//'
}

# ── _step_preflight ───────────────────────────────────────────────────────────
# Returns 0 (no-op, emit sentinel) when no legacy sources exist and .accelerator/
# contains nothing beyond the minimal scaffold written by this migration.
# Returns 1 to proceed with migration steps.
_step_preflight() {
  local has_source=0

  for src in \
    "$PROJECT_ROOT/.claude/accelerator.md" \
    "$PROJECT_ROOT/.claude/accelerator.local.md" \
    "$PROJECT_ROOT/.claude/accelerator/skills" \
    "$PROJECT_ROOT/.claude/accelerator/lenses" \
    "$PROJECT_ROOT/meta/templates" \
    "$PROJECT_ROOT/meta/integrations" \
    "$PROJECT_ROOT/meta/.migrations-applied" \
    "$PROJECT_ROOT/meta/.migrations-skipped"; do
    if [ -e "$src" ]; then
      has_source=1
      break
    fi
  done

  # Also check meta/tmp if paths.tmp is unset (it would be a move source)
  if [ "$has_source" -eq 0 ]; then
    local tmp_val
    tmp_val=$(probe_paths_key tmp)
    if [ -z "$tmp_val" ] && [ -e "$PROJECT_ROOT/meta/tmp" ]; then
      has_source=1
    fi
  fi

  if [ "$has_source" -eq 1 ]; then
    return 1  # proceed
  fi

  # No sources — check whether .accelerator/ has content beyond the minimal scaffold.
  # Minimal scaffold files: .accelerator/.gitignore, .accelerator/state/.gitkeep
  if [ -d "$PROJECT_ROOT/.accelerator" ]; then
    while IFS= read -r -d '' path; do
      local rel="${path#"$PROJECT_ROOT/.accelerator/"}"
      case "$rel" in
        .gitignore|state|state/.gitkeep)
          ;;
        *)
          return 1  # extra content beyond scaffold — proceed
          ;;
      esac
    done < <(find "$PROJECT_ROOT/.accelerator" -mindepth 1 \( -type f -o -type d \) -print0 2>/dev/null)
  fi

  echo "MIGRATION_RESULT: no_op_pending"
  return 0
}

# ── _step_init_scaffold ───────────────────────────────────────────────────────
_step_init_scaffold() {
  accelerator_ensure_inner_gitignore "$PROJECT_ROOT"
  accelerator_ensure_state_dir "$PROJECT_ROOT"
}

# ── _step_rewrite_root_gitignore ─────────────────────────────────────────────
_step_rewrite_root_gitignore() {
  accelerator_remove_legacy_root_gitignore_rules "$PROJECT_ROOT"
  accelerator_ensure_root_gitignore_rule "$PROJECT_ROOT"
}

# ── _step_warn_pinned_overrides ───────────────────────────────────────────────
_step_warn_pinned_overrides() {
  local templates_val integrations_val
  templates_val=$(probe_paths_key templates)
  integrations_val=$(probe_paths_key integrations)

  if [ -n "$templates_val" ]; then
    log_warn "paths.templates is explicitly set to '$templates_val'. The migration" \
      "moves meta/templates/ to .accelerator/templates/ unconditionally." \
      "Update paths.templates to .accelerator/templates (or remove the override)" \
      "after migration to restore correct resolution."
  fi
  if [ -n "$integrations_val" ]; then
    log_warn "paths.integrations is explicitly set to '$integrations_val'. The migration" \
      "moves meta/integrations/jira/ to .accelerator/state/integrations/jira/ unconditionally." \
      "Update paths.integrations to .accelerator/state/integrations (or remove the override)" \
      "after migration to restore correct resolution."
  fi
}

# ── _step_move_sources ────────────────────────────────────────────────────────
_step_move_sources() {
  # Probe paths.tmp BEFORE moving legacy config files; the probe reads from
  # .claude/accelerator.md which is about to be moved in the first _move_if_pending.
  local tmp_val
  tmp_val=$(probe_paths_key tmp)

  _move_if_pending .claude/accelerator.md          .accelerator/config.md
  _move_if_pending .claude/accelerator.local.md    .accelerator/config.local.md
  _move_if_pending .claude/accelerator/skills      .accelerator/skills
  _move_if_pending .claude/accelerator/lenses      .accelerator/lenses
  _move_if_pending meta/templates                  .accelerator/templates
  _move_if_pending meta/integrations/jira          .accelerator/state/integrations/jira

  if [ -z "$tmp_val" ]; then
    _move_if_pending meta/tmp .accelerator/tmp
  fi
}

# ── _step_relocate_state_files ────────────────────────────────────────────────
# Merges legacy meta/.migrations-* into .accelerator/state/migrations-* and
# removes the source files. Deduplicates union when destination already exists.
_merge_state_file() {
  local src_rel="$1" dst_name="$2"
  local src="$PROJECT_ROOT/$src_rel"
  local dst="$PROJECT_ROOT/.accelerator/state/$dst_name"

  [ -f "$src" ] || return 0

  # Collect lines from destination (if exists) then source — first-seen wins.
  local all_lines=()
  if [ -f "$dst" ]; then
    while IFS= read -r line; do
      [ -n "$line" ] && all_lines+=("$line")
    done < "$dst"
  fi
  while IFS= read -r line; do
    [ -n "$line" ] && all_lines+=("$line")
  done < "$src"

  # Deduplicate preserving order.
  local seen=() unique=()
  for line in "${all_lines[@]+"${all_lines[@]}"}"; do
    local found=0
    for s in "${seen[@]+"${seen[@]}"}"; do
      [ "$s" = "$line" ] && found=1 && break
    done
    if [ "$found" -eq 0 ]; then
      unique+=("$line")
      seen+=("$line")
    fi
  done

  mkdir -p "$(dirname "$dst")"
  {
    for line in "${unique[@]+"${unique[@]}"}"; do
      printf '%s\n' "$line"
    done
  } | atomic_write "$dst"

  rm -f "$src"
}

_step_relocate_state_files() {
  _merge_state_file meta/.migrations-applied  migrations-applied
  _merge_state_file meta/.migrations-skipped  migrations-skipped
}

# ── _step_inner_jira_gitignore ────────────────────────────────────────────────
_step_inner_jira_gitignore() {
  local jira_dir="$PROJECT_ROOT/.accelerator/state/integrations/jira"
  [ -d "$jira_dir" ] || return 0

  local gi="$jira_dir/.gitignore"
  touch "$gi"
  for rule in "${JIRA_INNER_GITIGNORE_RULES[@]}"; do
    grep -qFx "$rule" "$gi" 2>/dev/null || printf '%s\n' "$rule" >> "$gi"
  done
  [ -e "$jira_dir/.gitkeep" ] || touch "$jira_dir/.gitkeep"
}

# ── _step_done ────────────────────────────────────────────────────────────────
_step_done() {
  echo "MIGRATION_RESULT: applied"
}

# ── main ──────────────────────────────────────────────────────────────────────
main() {
  _step_preflight && return 0
  _step_init_scaffold
  _step_rewrite_root_gitignore
  _step_warn_pinned_overrides
  _step_move_sources
  _step_relocate_state_files
  _step_inner_jira_gitignore
  _step_done
}

main "$@"
