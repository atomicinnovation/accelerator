#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential work item ID under the configured pattern.
# Scans the configured work directory for the highest existing matching
# number and increments by one. Output is one ID per line, formatted via
# the pattern's compile-format string.
#
# Usage: work-item-next-number.sh [--project CODE] [--count N]
#   --project CODE  Project code substituted into {project} when the
#                   pattern contains it. Falls back to
#                   work.default_project_code config value.
#   --count N       Output N sequential IDs, one per line (default: 1)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/vcs-common.sh"
# shellcheck source=work-item-common.sh
source "$SCRIPT_DIR/work-item-common.sh"

COUNT=1
PROJECT=""
PROJECT_FLAG_GIVEN=0

while [ $# -gt 0 ]; do
  case "$1" in
    --count)
      if [ $# -lt 2 ]; then
        echo "Error: --count requires a value" >&2; exit 1
      fi
      COUNT="$2"; shift 2 ;;
    --project)
      if [ $# -lt 2 ]; then
        echo "Error: --project requires a value" >&2; exit 1
      fi
      PROJECT="$2"
      PROJECT_FLAG_GIVEN=1
      shift 2 ;;
    *) echo "Usage: work-item-next-number.sh [--project CODE] [--count N]" >&2; exit 1 ;;
  esac
done

if ! [[ "$COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: --count requires a positive integer, got '$COUNT'" >&2
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

# Validate pattern
if ! wip_validate_pattern "$PATTERN"; then
  exit 1
fi

# Resolve project value: --project > config default
if [ "$PROJECT_FLAG_GIVEN" -eq 0 ]; then
  PROJECT="$DEFAULT_PROJECT"
fi

PATTERN_HAS_PROJECT=0
if [[ "$PATTERN" == *"{project}"* ]]; then
  PATTERN_HAS_PROJECT=1
fi

if [ "$PATTERN_HAS_PROJECT" -eq 1 ] && [ -z "$PROJECT" ]; then
  echo "E_PATTERN_MISSING_PROJECT: pattern '$PATTERN' contains {project} but no value supplied — pass --project or set work.default_project_code" >&2
  exit 1
fi

if [ "$PATTERN_HAS_PROJECT" -eq 0 ] && [ "$PROJECT_FLAG_GIVEN" -eq 1 ] && [ -n "$PROJECT" ]; then
  echo "E_PATTERN_PROJECT_UNUSED: --project is meaningless for pattern '$PATTERN' (no {project} token)" >&2
  exit 1
fi

# Compile scan regex and format string for use-time
SCAN_RE=$(wip_compile_scan "$PATTERN" "$PROJECT") || exit 1
FORMAT=$(wip_compile_format "$PATTERN" "$PROJECT") || exit 1
CAP=$(wip_pattern_max_number "$PATTERN") || exit 1

HIGHEST=0
HIGHEST_FILE=""
if [ ! -d "$WORK_DIR" ]; then
  echo "Warning: work directory '$WORK_DIR' does not exist — defaulting to next number. Run /accelerator:init or create the directory to persist work items." >&2
else
  shopt -s nullglob
  for f in "$WORK_DIR"/*.md; do
    [ -e "$f" ] || continue
    BASE=$(basename "$f")
    if [[ "$BASE" =~ $SCAN_RE ]]; then
      NUM="${BASH_REMATCH[1]}"
      NUMVAL=$((10#$NUM))
      if [ "$NUMVAL" -gt "$HIGHEST" ]; then
        HIGHEST="$NUMVAL"
        HIGHEST_FILE="$BASE"
      fi
    fi
  done
  shopt -u nullglob
fi

# Overflow guard: HIGHEST + COUNT must not exceed CAP (10^N - 1).
if [ "$((HIGHEST + COUNT))" -gt "$CAP" ]; then
  if [ "$HIGHEST" -gt "$CAP" ]; then
    echo "E_PATTERN_OVERFLOW: out-of-width file '$HIGHEST_FILE' has number $HIGHEST exceeding the pattern '$PATTERN' cap of $CAP. Rename the stray file or widen the pattern." >&2
  else
    echo "E_PATTERN_OVERFLOW: pattern '$PATTERN' number space exhausted (highest=$HIGHEST, cap=$CAP). Archive completed work items or widen the pattern." >&2
  fi
  # Still emit any numbers that fit before the boundary
  for ((i = 1; i <= COUNT; i++)); do
    NEXT=$((HIGHEST + i))
    [ "$NEXT" -gt "$CAP" ] && break
    # shellcheck disable=SC2059
    printf "$FORMAT\n" "$NEXT"
  done
  exit 1
fi

for ((i = 1; i <= COUNT; i++)); do
  # shellcheck disable=SC2059
  printf "$FORMAT\n" "$((HIGHEST + i))"
done
