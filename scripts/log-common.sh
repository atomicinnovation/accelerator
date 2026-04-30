#!/usr/bin/env bash
# Generic logging helpers. Source from scripts that need log_die / log_warn:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/log-common.sh"
#
# Functions:
#   log_die <msg>  — write msg to stderr and exit non-zero
#   log_warn <msg> — write "Warning: msg" to stderr and return

log_die() {
  echo "$1" >&2
  exit 1
}

log_warn() {
  echo "Warning: $1" >&2
}
