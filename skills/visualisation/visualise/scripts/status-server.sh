#!/usr/bin/env bash
# INFO/PID_FILE are the caller-set globals that stop_server_status (from
# launcher-helpers.sh) reads; ShellCheck can't see the cross-file consumption.
# shellcheck disable=SC2034
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
source "$SCRIPT_DIR/launcher-helpers.sh"

PROJECT_ROOT="$(find_repo_root)"
cd "$PROJECT_ROOT"

TMP_REL="$("${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config path tmp)"
TMP_DIR="$PROJECT_ROOT/$TMP_REL/visualiser"
INFO="$TMP_DIR/server-info.json"
PID_FILE="$TMP_DIR/server.pid"

stop_server_status
