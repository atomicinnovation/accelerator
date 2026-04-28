#!/usr/bin/env bash
set -euo pipefail

# Resolve a user-supplied work-item identifier to a canonical file path.
#
# Usage: work-item-resolve-id.sh <input>
#
# Classifies the input as one of: path | full_id | bare_number | invalid
# and probes the corpus for a matching work-item file.
#
# Exit codes:
#   0  Single match resolved; absolute path on stdout.
#   1  Invalid input shape; stderr names the reason.
#   2  Multiple matches; stderr lists candidates with source category.
#   3  No match; stderr describes the miss.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/vcs-common.sh"
# shellcheck source=work-item-common.sh
source "$SCRIPT_DIR/work-item-common.sh"

if [ $# -ne 1 ]; then
  echo "Usage: work-item-resolve-id.sh <input>" >&2
  exit 1
fi

INPUT="$1"

if [ -z "$INPUT" ]; then
  echo "E_RESOLVE_INVALID: empty input" >&2
  exit 1
fi

REPO_ROOT=$(find_repo_root) || REPO_ROOT="$PWD"

WORK_PATH=$("$PLUGIN_ROOT/scripts/config-read-path.sh" work meta/work)
if [[ "$WORK_PATH" == /* ]]; then
  WORK_DIR="$WORK_PATH"
else
  WORK_DIR="$REPO_ROOT/$WORK_PATH"
fi

PATTERN=$("$PLUGIN_ROOT/scripts/config-read-value.sh" work.id_pattern "{number:04d}")
DEFAULT_PROJECT=$("$PLUGIN_ROOT/scripts/config-read-value.sh" work.default_project_code "")

if ! wip_validate_pattern "$PATTERN"; then
  exit 1
fi

PATTERN_HAS_PROJECT=0
if [[ "$PATTERN" == *"{project}"* ]]; then
  PATTERN_HAS_PROJECT=1
fi

# Determine width
WIDTH=4
if [[ "$PATTERN" =~ \{number:0([1-9][0-9]*)d\} ]]; then
  WIDTH="${BASH_REMATCH[1]}"
fi

# -- classification ---------------------------------------------------

classify_input() {
  local s="$1"
  if [[ "$s" == ./* ]] || [[ "$s" == /* ]] || [[ "$s" == */* ]]; then
    echo "path"
    return
  fi
  if parsed=$(wip_parse_full_id "$s" "$PATTERN" 2>/dev/null); then
    if [ "$PATTERN_HAS_PROJECT" -eq 1 ] && [[ "$parsed" == *"	"* ]]; then
      local proj="${parsed%	*}"
      if [ -n "$proj" ]; then
        echo "full_id"
        return
      fi
    fi
    # No-project pattern parses bare numbers as full IDs too; prefer
    # the bare_number path so the corpus search uses both legacy and
    # pattern-shape candidate sets.
    if [[ "$s" =~ ^[0-9]+$ ]]; then
      echo "bare_number"
      return
    fi
    echo "full_id"
    return
  fi
  if [[ "$s" =~ ^[0-9]+$ ]]; then
    echo "bare_number"
    return
  fi
  echo "invalid"
}

CLASS=$(classify_input "$INPUT")

case "$CLASS" in
  path)
    # Path-shaped inputs are always resolved as paths
    if [ -f "$INPUT" ]; then
      cd "$(dirname "$INPUT")"
      printf '%s\n' "$(pwd)/$(basename "$INPUT")"
      exit 0
    fi
    if [[ "$INPUT" == /* ]] && [ -f "$INPUT" ]; then
      printf '%s\n' "$INPUT"
      exit 0
    fi
    echo "E_RESOLVE_NOT_FOUND: no work item at path '$INPUT'" >&2
    exit 3
    ;;
  full_id)
    shopt -s nullglob
    matches=()
    for f in "$WORK_DIR"/"$INPUT"-*.md; do
      [ -e "$f" ] && matches+=("$f")
    done
    shopt -u nullglob
    case "${#matches[@]}" in
      0)
        echo "E_RESOLVE_NOT_FOUND: no work item with ID '$INPUT' in $WORK_DIR" >&2
        exit 3
        ;;
      1)
        printf '%s\n' "${matches[0]}"
        exit 0
        ;;
      *)
        echo "E_RESOLVE_AMBIGUOUS: multiple work items match ID '$INPUT':" >&2
        for m in "${matches[@]}"; do
          echo "  $m" >&2
        done
        exit 2
        ;;
    esac
    ;;
  bare_number)
    NUM=$((10#$INPUT))
    # Build candidate set with source-category tags. Each candidate is
    # encoded as "path<tab>tag" so deduplication keeps the highest-
    # priority tag for the same disk file.
    declare -a cand_paths=()
    declare -a cand_tags=()
    add_candidate() {
      local p="$1" tag="$2"
      local i
      for i in "${!cand_paths[@]}"; do
        if [ "${cand_paths[$i]}" = "$p" ]; then
          # Already present; do not overwrite (priority order: caller
          # adds project-prepended first so it wins over cross-project)
          return
        fi
      done
      cand_paths+=("$p")
      cand_tags+=("$tag")
    }

    # (a) Project-prepended candidate
    if [ "$PATTERN_HAS_PROJECT" -eq 1 ] && [ -n "$DEFAULT_PROJECT" ]; then
      PADDED=$(printf "%0${WIDTH}d" "$NUM")
      FORMATTED=$(wip_compile_format "$PATTERN" "$DEFAULT_PROJECT") || exit 1
      # shellcheck disable=SC2059
      FULL_ID=$(printf "$FORMATTED" "$NUM")
      shopt -s nullglob
      for f in "$WORK_DIR"/"$FULL_ID"-*.md; do
        [ -e "$f" ] && add_candidate "$f" "project-prepended"
      done
      shopt -u nullglob
      _=$PADDED
    fi

    # (b) Legacy candidate (only if input has ≤4 digits)
    if [ "${#INPUT}" -le 4 ]; then
      LEGACY_PADDED=$(printf "%04d" "$NUM")
      shopt -s nullglob
      for f in "$WORK_DIR"/"$LEGACY_PADDED"-*.md; do
        [ -e "$f" ] && add_candidate "$f" "legacy"
      done
      shopt -u nullglob
    fi

    # (c) Pattern-shape candidate (only if pattern lacks {project})
    if [ "$PATTERN_HAS_PROJECT" -eq 0 ]; then
      FORMATTED=$(wip_compile_format "$PATTERN" "") || exit 1
      # shellcheck disable=SC2059
      FULL_ID=$(printf "$FORMATTED" "$NUM")
      shopt -s nullglob
      for f in "$WORK_DIR"/"$FULL_ID"-*.md; do
        [ -e "$f" ] && add_candidate "$f" "pattern-shape"
      done
      shopt -u nullglob
    fi

    # (d) Cross-project scan candidate (only if pattern has {project})
    if [ "$PATTERN_HAS_PROJECT" -eq 1 ]; then
      PADDED=$(printf "%0${WIDTH}d" "$NUM")
      shopt -s nullglob
      for f in "$WORK_DIR"/*-"$PADDED"-*.md; do
        [ -e "$f" ] || continue
        BASE=$(basename "$f")
        # Parse the project code from the filename via wip_parse_full_id
        # against the pattern. The "ID portion" is everything before
        # the slug, so split on the last '-<slug>.md'.
        # Better: extract the prefix portion before '-<padded>-'.
        PREFIX="${BASE%-${PADDED}-*}"
        if [ "$PREFIX" = "$BASE" ]; then
          # No match — basename did not contain -<padded>-
          continue
        fi
        # PREFIX is the project code (no hyphens permitted in project code per rule 5)
        if [[ "$PREFIX" =~ ^[A-Za-z][A-Za-z0-9]*$ ]]; then
          add_candidate "$f" "$PREFIX"
        fi
      done
      shopt -u nullglob
    fi

    case "${#cand_paths[@]}" in
      0)
        echo "E_RESOLVE_NOT_FOUND: no work item matching bare number '$INPUT' in $WORK_DIR" >&2
        exit 3
        ;;
      1)
        printf '%s\n' "${cand_paths[0]}"
        exit 0
        ;;
      *)
        echo "E_RESOLVE_AMBIGUOUS: multiple work items match bare number '$INPUT':" >&2
        for i in "${!cand_paths[@]}"; do
          echo "  ${cand_paths[$i]} [${cand_tags[$i]}]" >&2
        done
        exit 2
        ;;
    esac
    ;;
  invalid)
    echo "E_RESOLVE_INVALID: input '$INPUT' is not a recognised path, full ID, or bare number" >&2
    exit 1
    ;;
esac
