#!/usr/bin/env bash
# Thin CLI wrapper around jira-auth.sh. Resolves Jira credentials and
# prints them to stdout as three key=value lines:
#
#   site=<JIRA_SITE>
#   email=<JIRA_EMAIL>
#   token=<JIRA_TOKEN>
#
# Usage: jira-auth-cli.sh [--debug]
#
# --debug: write resolution-path metadata to stderr. The token value is
#   NEVER written to stderr; the literal "***" is substituted instead.
#   --debug does NOT propagate to downstream curl as -v/--verbose/--trace.
#
# Exit codes:
#   0   — credentials resolved
#   24  — E_NO_TOKEN
#   25  — E_TOKEN_CMD_FAILED
#   27  — E_AUTH_NO_SITE
#   28  — E_AUTH_NO_EMAIL
#   29  — E_LOCAL_PERMS_INSECURE
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/jira-auth.sh"

_DEBUG=0
for _arg in "$@"; do
  case "$_arg" in
    --debug) _DEBUG=1 ;;
    *) echo "Usage: jira-auth-cli.sh [--debug]" >&2; exit 2 ;;
  esac
done

jira_resolve_credentials || exit $?

if [ "$_DEBUG" -eq 1 ]; then
  echo "token resolved from: ${JIRA_RESOLUTION_SOURCE_TOKEN} (value: ***)" >&2
  echo "site resolved from: ${JIRA_RESOLUTION_SOURCE_SITE}" >&2
  echo "email resolved from: ${JIRA_RESOLUTION_SOURCE_EMAIL}" >&2
fi

printf 'site=%s\nemail=%s\ntoken=%s\n' "$JIRA_SITE" "$JIRA_EMAIL" "$JIRA_TOKEN"
