#!/usr/bin/env bash
# Thin CLI wrapper around linear-auth.sh. Resolves a Linear token and prints
# it to stdout as a single key=value line:
#
#   token=<LINEAR_TOKEN>
#
# Usage: linear-auth-cli.sh [--debug]
#
# --debug: write resolution-path metadata to stderr. The token value is
#   NEVER written to stderr; the literal "***" is substituted instead.
#   --debug does NOT propagate to downstream curl as -v/--verbose/--trace.
#
# Exit codes:
#   0   — token resolved
#   24  — E_NO_TOKEN
#   25  — E_TOKEN_CMD_FAILED
#   27  — E_TOKEN_MALFORMED
#   29  — E_LOCAL_PERMS_INSECURE
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/linear-auth.sh"

_DEBUG=0
for _arg in "$@"; do
  case "$_arg" in
    --debug) _DEBUG=1 ;;
    *)
      echo "Usage: linear-auth-cli.sh [--debug]" >&2
      exit 2
      ;;
  esac
done

linear_resolve_credentials || exit $?

if [ "$_DEBUG" -eq 1 ]; then
  echo "token resolved from: ${LINEAR_RESOLUTION_SOURCE_TOKEN} (value: ***)" >&2
fi

printf 'token=%s\n' "$LINEAR_TOKEN"
