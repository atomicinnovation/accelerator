#!/usr/bin/env bash
set -euo pipefail

# Work-item ID pattern compiler CLI.
#
# Thin wrapper around work-item-common.sh's wip_validate_pattern,
# wip_compile_scan, and wip_compile_format functions for callers that
# prefer subprocess invocation.
#
# Usage:
#   work-item-pattern.sh --validate <pattern>
#     Exit 0 if the pattern is valid; 2 if invalid (stderr starts with
#     E_PATTERN_*); 1 on usage error.
#
#   work-item-pattern.sh --compile-scan <pattern> <project_value>
#     Emit the ERE scan regex on stdout (capture group 1 = number).
#     project_value may be empty when the pattern has no {project}.
#
#   work-item-pattern.sh --compile-format <pattern> <project_value>
#     Emit the printf format string on stdout.
#
# Validation rules and the token grammar live in work-item-common.sh
# and the Pattern DSL Reference section of
# meta/plans/2026-04-28-configurable-work-item-id-pattern.md.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=work-item-common.sh
source "$SCRIPT_DIR/work-item-common.sh"

usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-pattern.sh --validate <pattern>
  work-item-pattern.sh --compile-scan <pattern> <project_value>
  work-item-pattern.sh --compile-format <pattern> <project_value>
USAGE
}

if [ $# -lt 1 ]; then
  usage
  exit 1
fi

MODE="$1"
shift

case "$MODE" in
  --validate)
    if [ $# -ne 1 ]; then
      usage
      exit 1
    fi
    if wip_validate_pattern "$1"; then
      exit 0
    else
      exit 2
    fi
    ;;
  --compile-scan)
    if [ $# -ne 2 ]; then
      usage
      exit 1
    fi
    if wip_compile_scan "$1" "$2"; then
      exit 0
    else
      exit 2
    fi
    ;;
  --compile-format)
    if [ $# -ne 2 ]; then
      usage
      exit 1
    fi
    if wip_compile_format "$1" "$2"; then
      exit 0
    else
      exit 2
    fi
    ;;
  *)
    usage
    exit 1
    ;;
esac
