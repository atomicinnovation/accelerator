#!/usr/bin/env bash
# Entry-point dispatcher for the meta visualiser. Routes a single
# subcommand argument to the underlying lifecycle scripts:
#
#   <empty> | start   -> launch-server.sh
#   stop              -> stop-server.sh
#   status            -> status-server.sh
#
# Both /accelerator:visualise (slash command) and the
# accelerator-visualiser CLI wrapper invoke this script.
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

subcommand="${1:-start}"
case "$subcommand" in
  ""|start) exec "$SCRIPT_DIR/launch-server.sh" ;;
  stop)     exec "$SCRIPT_DIR/stop-server.sh" ;;
  status)   exec "$SCRIPT_DIR/status-server.sh" ;;
  *)
    echo '{"error":"unknown subcommand","hint":"use start (default), stop, or status"}' >&2
    exit 2
    ;;
esac
