#!/usr/bin/env bash
# DESCRIPTION: Restructure meta/research/ into subject subcategories
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

# ── jj op-id breadcrumb for rollback ─────────────────────────────────────────
if command -v jj >/dev/null 2>&1 && [ -d "$PROJECT_ROOT/.jj" ]; then
  _0004_op_id=$(jj op log -l 1 --no-graph -T 'self.id().short()' \
                  2>/dev/null | head -1 || true)
  if [ -n "$_0004_op_id" ]; then
    echo "0004: pre-migration jj op-id: $_0004_op_id" >&2
    echo "0004: roll back with: jj op restore $_0004_op_id" >&2
  fi
fi

# ── Step 0a: per-file key probe ──────────────────────────────────────────────
# Returns "<present>\t<value>" on stdout.  <present> is 0 or 1; <value> is the
# raw value with inline `# comment` and trailing whitespace stripped.
#
# Detects either nested-YAML form (`paths:` block then indented key) or
# flat-dotted form (`paths.key: value`).
probe_key_in_file() {
  local cfg="$1" prefix="$2" key="$3"
  [ -f "$cfg" ] || { printf '0\t'; return 0; }
  local result
  result=$(awk -v prefix="$prefix" -v key="$key" '
    BEGIN {
      block_re="^" prefix ":[[:space:]]*$"
      nested_re="^[[:space:]]+" key ":"
      flat_re="^" prefix "\\." key ":"
      in_block=0
    }
    $0 ~ block_re { in_block=1; next }
    in_block && /^[^[:space:]]/ { in_block=0 }
    in_block && $0 ~ nested_re {
      sub(nested_re, "")
      sub(/^[[:space:]]+/, "")
      sub(/[[:space:]]*#.*$/, "")
      sub(/[[:space:]]+$/, "")
      printf "P\t%s", $0
      exit
    }
    $0 ~ flat_re {
      sub(flat_re, "")
      sub(/^[[:space:]]+/, "")
      sub(/[[:space:]]*#.*$/, "")
      sub(/[[:space:]]+$/, "")
      printf "P\t%s", $0
      exit
    }
  ' "$cfg")
  if [ -n "$result" ]; then
    printf '1\t%s' "${result#P$'\t'}"
  else
    printf '0\t'
  fi
}

# Cross-file probe: returns first match across config.local.md (preferred)
# then config.md. Used by Step 0a to capture legacy values once.
probe_key() {
  local prefix="$1" key="$2" cfg p
  for cfg in "$PROJECT_ROOT/.accelerator/config.local.md" \
             "$PROJECT_ROOT/.accelerator/config.md"; do
    p=$(probe_key_in_file "$cfg" "$prefix" "$key")
    if [ "${p%%$'\t'*}" = "1" ]; then
      printf '%s' "$p"
      return 0
    fi
  done
  printf '0\t'
}

_probe_research=$(probe_key "paths" "research")
_probe_inv=$(probe_key "paths" "design_inventories")
_probe_gaps=$(probe_key "paths" "design_gaps")

RESEARCH_HAD_OVERRIDE="${_probe_research%%$'\t'*}"
INV_HAD_OVERRIDE="${_probe_inv%%$'\t'*}"
GAPS_HAD_OVERRIDE="${_probe_gaps%%$'\t'*}"

OLD_RESEARCH="${_probe_research#*$'\t'}"
OLD_INV="${_probe_inv#*$'\t'}"
OLD_GAPS="${_probe_gaps#*$'\t'}"

[ "$RESEARCH_HAD_OVERRIDE" = "1" ] || OLD_RESEARCH="meta/research"
[ "$INV_HAD_OVERRIDE" = "1" ]      || OLD_INV="meta/design-inventories"
[ "$GAPS_HAD_OVERRIDE" = "1" ]     || OLD_GAPS="meta/design-gaps"

# Strip trailing slash for consistent path arithmetic.
OLD_RESEARCH="${OLD_RESEARCH%/}"
OLD_INV="${OLD_INV%/}"
OLD_GAPS="${OLD_GAPS%/}"

# D1: research_codebase always nests; design-inv/gaps honor overrides.
NEW_RESEARCH_CODEBASE="${OLD_RESEARCH}/codebase"
NEW_RESEARCH_ISSUES="${OLD_RESEARCH}/issues"
if [ "$INV_HAD_OVERRIDE" = "1" ]; then
  NEW_INV="$OLD_INV"
else
  NEW_INV="${OLD_RESEARCH}/design-inventories"
fi
if [ "$GAPS_HAD_OVERRIDE" = "1" ]; then
  NEW_GAPS="$OLD_GAPS"
else
  NEW_GAPS="${OLD_RESEARCH}/design-gaps"
fi

# ── Step 0b: mixed-state detection ───────────────────────────────────────────
_assert_no_mixed_state() {
  local triple prefix rest old new old_present new_present p cfg
  for triple in \
      "paths:research:research_codebase" \
      "paths:design_inventories:research_design_inventories" \
      "paths:design_gaps:research_design_gaps" \
      "templates:research:codebase-research"; do
    prefix="${triple%%:*}"
    rest="${triple#*:}"
    old="${rest%:*}"; new="${rest#*:}"
    old_present=0; new_present=0
    for cfg in "$PROJECT_ROOT/.accelerator/config.md" \
               "$PROJECT_ROOT/.accelerator/config.local.md"; do
      [ -f "$cfg" ] || continue
      p=$(probe_key_in_file "$cfg" "$prefix" "$old")
      [ "${p%%$'\t'*}" = "1" ] && old_present=1
      p=$(probe_key_in_file "$cfg" "$prefix" "$new")
      [ "${p%%$'\t'*}" = "1" ] && new_present=1
    done
    if [ "$old_present" = "1" ] && [ "$new_present" = "1" ]; then
      log_die "0004: mixed-state config detected — both ${prefix}.${old} and ${prefix}.${new} are set. Resolve manually (remove ${prefix}.${old}) and retry."
    fi
  done
}
_assert_no_mixed_state

# Short-circuit when there is genuinely nothing to migrate: neither the legacy
# research dir nor the legacy design-inv/design-gaps dirs exist. Avoids
# tripping the no-VCS safety net on fresh repos / unrelated test fixtures.
if [ ! -d "$PROJECT_ROOT/$OLD_RESEARCH" ] \
   && [ ! -d "$PROJECT_ROOT/$OLD_INV" ] \
   && [ ! -d "$PROJECT_ROOT/$OLD_GAPS" ]; then
  exit 0
fi

# ── Step 0c: scan-corpus dirty-tree pre-flight ───────────────────────────────
build_scan_corpus() {
  bash "$PLUGIN_ROOT/scripts/config-read-all-paths.sh" 2>/dev/null \
    | awk '
        /^- [^[:space:]]+: / {
          sub(/^- [^[:space:]]+: /, "")
          print
        }
      ' \
    | while IFS= read -r v; do
        [ -d "$PROJECT_ROOT/$v" ] && printf '%s\n' "$PROJECT_ROOT/$v"
      done
}

_preflight_scan_corpus_clean() {
  # No-VCS detection runs unconditionally — even with an empty scan corpus the
  # migration mutates the filesystem (moves, .gitkeep, legacy-dir cleanup),
  # so VCS-based rollback is the safety net.
  local has_vcs=0
  if command -v jj >/dev/null 2>&1 && [ -d "$PROJECT_ROOT/.jj" ]; then
    has_vcs=1
  elif [ -d "$PROJECT_ROOT/.git" ]; then
    has_vcs=1
  fi
  if [ "$has_vcs" = "0" ]; then
    if [ "${ACCELERATOR_MIGRATE_FORCE_NO_VCS:-}" = "1" ]; then
      log_warn "0004: no VCS detected — proceeding because ACCELERATOR_MIGRATE_FORCE_NO_VCS=1. Recovery from a botched migration is not possible without VCS."
    else
      log_die "0004: no VCS detected (.jj or .git absent). Recovery via VCS rollback is unavailable. Set ACCELERATOR_MIGRATE_FORCE_NO_VCS=1 to proceed anyway."
    fi
  fi

  local dirs=()
  while IFS= read -r d; do dirs+=("$d"); done < <(build_scan_corpus)
  [ "${#dirs[@]}" -eq 0 ] && return 0
  [ "$has_vcs" = "0" ] && return 0   # bypass already warned

  local dirty=""
  if [ -d "$PROJECT_ROOT/.jj" ] && command -v jj >/dev/null 2>&1; then
    dirty=$(jj --no-pager diff --name-only -r @ -- "${dirs[@]}" 2>/dev/null || true)
  elif [ -d "$PROJECT_ROOT/.git" ]; then
    dirty=$(git -C "$PROJECT_ROOT" status --porcelain -- "${dirs[@]}" 2>/dev/null \
              | grep -v '^??' || true)
  fi
  if [ -n "$dirty" ]; then
    echo "0004: scan corpus has uncommitted changes — commit or stash first:" >&2
    printf '%s\n' "$dirty" | sed 's/^/  /' >&2
    exit 1
  fi
}
_preflight_scan_corpus_clean

# ── Step 1: plan moves, check collisions, then execute ───────────────────────
PLANNED_MOVES=()  # entries are "src<TAB>dst"
_plan_move() { PLANNED_MOVES+=("$1"$'\t'"$2"); }

_plan_research_moves() {
  [ -d "$PROJECT_ROOT/$OLD_RESEARCH" ] || return 0
  local f base
  while IFS= read -r f; do
    base=$(basename "$f")
    [ "$base" = ".DS_Store" ] && continue
    [ "$base" = ".gitkeep" ] && continue
    _plan_move "$OLD_RESEARCH/$base" "$NEW_RESEARCH_CODEBASE/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_RESEARCH" && \
      ( shopt -s nullglob dotglob; for f in *; do [ -f "$f" ] && printf '%s\n' "$f"; done )
  )
}

_plan_inv_moves() {
  [ "$INV_HAD_OVERRIDE" = "1" ] && return 0
  [ -d "$PROJECT_ROOT/$OLD_INV" ] || return 0
  local d base
  while IFS= read -r d; do
    base=$(basename "$d")
    _plan_move "$OLD_INV/$base" "$NEW_INV/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_INV" && \
      ( shopt -s nullglob; for d in */; do printf '%s\n' "${d%/}"; done )
  )
}

_plan_gaps_moves() {
  [ "$GAPS_HAD_OVERRIDE" = "1" ] && return 0
  [ -d "$PROJECT_ROOT/$OLD_GAPS" ] || return 0
  local f base
  while IFS= read -r f; do
    base=$(basename "$f")
    [ "$base" = ".DS_Store" ] && continue
    [ "$base" = ".gitkeep" ] && continue
    _plan_move "$OLD_GAPS/$base" "$NEW_GAPS/$base"
  done < <(
    cd "$PROJECT_ROOT/$OLD_GAPS" && \
      ( shopt -s nullglob dotglob; for f in *; do [ -f "$f" ] && printf '%s\n' "$f"; done )
  )
}

_plan_research_moves
_plan_inv_moves
_plan_gaps_moves

_check_collisions() {
  local conflicts=() entry dst
  for entry in "${PLANNED_MOVES[@]+"${PLANNED_MOVES[@]}"}"; do
    dst="${entry#*$'\t'}"
    [ -e "$PROJECT_ROOT/$dst" ] && conflicts+=("$dst")
  done
  if [ "${#conflicts[@]}" -gt 0 ]; then
    echo "0004: destination collision(s) detected. Migration aborted with no filesystem changes." >&2
    local c
    for c in "${conflicts[@]}"; do
      echo "  conflict: $c already exists" >&2
    done
    exit 1
  fi
}
_check_collisions

_move_if_pending() {
  local src_rel="$1" dst_rel="$2"
  local src="$PROJECT_ROOT/$src_rel" dst="$PROJECT_ROOT/$dst_rel"
  [ -e "$src" ] || return 0
  mkdir -p "$(dirname "$dst")"
  mv "$src" "$dst"
  echo "0004: moved $src_rel → $dst_rel"
}

for entry in "${PLANNED_MOVES[@]+"${PLANNED_MOVES[@]}"}"; do
  src="${entry%$'\t'*}"; dst="${entry#*$'\t'}"
  _move_if_pending "$src" "$dst"
done

# ── Cleanup legacy parents ───────────────────────────────────────────────────
_cleanup_legacy_parent() {
  local d="$1"
  local full="$PROJECT_ROOT/$d"
  [ -d "$full" ] || return 0
  [ -f "$full/.DS_Store" ] && rm -f "$full/.DS_Store"
  [ -f "$full/.gitkeep" ] && rm -f "$full/.gitkeep"
  if rmdir "$full" 2>/dev/null; then
    echo "0004: removed empty legacy directory $d"
  else
    log_warn "0004: legacy directory $d not empty — preserved as-is."
    local r
    while IFS= read -r r; do
      log_warn "  contains: $r (manual cleanup may be needed)"
    done < <(
      cd "$full" && \
        ( shopt -s nullglob dotglob; for x in *; do printf '%s\n' "$x"; done )
    )
  fi
}

[ "$INV_HAD_OVERRIDE" = "1" ]  || _cleanup_legacy_parent "$OLD_INV"
[ "$GAPS_HAD_OVERRIDE" = "1" ] || _cleanup_legacy_parent "$OLD_GAPS"

# ── Ensure .gitkeep in every destination directory ───────────────────────────
_ensure_gitkeep() {
  local d="$1"
  local full="$PROJECT_ROOT/$d"
  [ -d "$full" ] || mkdir -p "$full"
  [ -e "$full/.gitkeep" ] || { : > "$full/.gitkeep"; echo "0004: created $d/.gitkeep"; }
}

_ensure_gitkeep "$NEW_RESEARCH_CODEBASE"
_ensure_gitkeep "$NEW_RESEARCH_ISSUES"
[ "$INV_HAD_OVERRIDE" = "1" ]  || _ensure_gitkeep "$NEW_INV"
[ "$GAPS_HAD_OVERRIDE" = "1" ] || _ensure_gitkeep "$NEW_GAPS"
