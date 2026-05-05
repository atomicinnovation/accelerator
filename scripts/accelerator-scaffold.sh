#!/usr/bin/env bash
# Shared scaffold/gitignore helpers for the .accelerator/ tree.
#
# Sourced by:
#   - skills/config/init/scripts/init.sh (Phase 4)
#   - skills/config/migrate/migrations/0003-relocate-accelerator-state.sh
#
# Idempotency contract: every helper is a no-op when its post-condition
# already holds. Helpers do not depend on any config-resolution layer and
# may be safely sourced from the migration even on legacy-only repos.
#
# Public helpers: accelerator_ensure_*  / accelerator_remove_*
# Implementation helpers: _accelerator_*

# Writes .accelerator/.gitignore containing the unanchored config.local.md
# rule if the file is absent. Defence-in-depth companion to the anchored root
# rule; kept in sync with accelerator_ensure_root_gitignore_rule via this
# shared file so the two surfaces cannot drift.
accelerator_ensure_inner_gitignore() {
  local project_root="$1"
  local dir="$project_root/.accelerator"
  local gi="$dir/.gitignore"
  mkdir -p "$dir"
  if [ ! -f "$gi" ]; then
    printf 'config.local.md\n' > "$gi"
  fi
}

# Ensures the anchored .accelerator/config.local.md rule is present in
# <project_root>/.gitignore exactly once (grep -qFx guard before append).
# Touches .gitignore to create it if absent.
accelerator_ensure_root_gitignore_rule() {
  local project_root="$1"
  local gi="$project_root/.gitignore"
  local rule='.accelerator/config.local.md'
  touch "$gi"
  if ! grep -qFx "$rule" "$gi"; then
    printf '%s\n' "$rule" >> "$gi"
  fi
}

# Removes legacy accelerator-related whole-line rules from <project_root>/.gitignore.
# Refuses with a reconciliation message if any matching line has trailing content
# (e.g. an inline comment). Used only by migration 0003 — not by init.sh.
#
# Legacy rules removed (anchored and unanchored forms):
#   .claude/accelerator.local.md
#   meta/integrations/jira/.lock
#   meta/integrations/jira/.refresh-meta.json
accelerator_remove_legacy_root_gitignore_rules() {
  local project_root="$1"
  local gi="$project_root/.gitignore"
  [ -f "$gi" ] || return 0

  local legacy_patterns=(
    '.claude/accelerator.local.md'
    '/.claude/accelerator.local.md'
    'meta/integrations/jira/.lock'
    '/meta/integrations/jira/.lock'
    'meta/integrations/jira/.refresh-meta.json'
    '/meta/integrations/jira/.refresh-meta.json'
  )

  # Refuse on any line that starts with a legacy pattern but has trailing content.
  while IFS= read -r line; do
    local trimmed="${line%"${line##*[![:space:]]}"}"
    for pattern in "${legacy_patterns[@]}"; do
      if [[ "$trimmed" == "$pattern"* ]] && [ "$trimmed" != "$pattern" ]; then
        printf '%s\n' \
          "accelerator migrate: refusing to rewrite '$line' in $gi —" \
          "line matches a legacy rule but has trailing content." \
          "Reconcile manually and re-run." >&2
        return 1
      fi
    done
  done < "$gi"

  # Remove exact whole-line matches.
  local tmp
  tmp="$(mktemp "$(dirname "$gi")/.gitignore-migrate.XXXXXX")"
  while IFS= read -r line; do
    local trimmed="${line%"${line##*[![:space:]]}"}"
    local skip=0
    for pattern in "${legacy_patterns[@]}"; do
      if [ "$trimmed" = "$pattern" ]; then
        skip=1
        break
      fi
    done
    [ "$skip" -eq 0 ] && printf '%s\n' "$line"
  done < "$gi" > "$tmp"
  mv "$tmp" "$gi"
}

# Creates .accelerator/state/ and .accelerator/state/.gitkeep if absent.
accelerator_ensure_state_dir() {
  local project_root="$1"
  local state_dir="$project_root/.accelerator/state"
  mkdir -p "$state_dir"
  [ -e "$state_dir/.gitkeep" ] || touch "$state_dir/.gitkeep"
}
