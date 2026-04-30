#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./jira-jql.sh
source "$SCRIPT_DIR/jira-jql.sh"

if [[ $# -eq 0 ]]; then
  echo "Usage: jira-jql-cli.sh compose [flags...]" >&2
  exit 2
fi

subcmd="$1"
shift

case "$subcmd" in
  compose)
    jql_compose "$@"
    printf '\n'
    ;;
  *)
    echo "E_JQL_BAD_FLAG: unknown subcommand: $subcmd" >&2
    exit 32
    ;;
esac
