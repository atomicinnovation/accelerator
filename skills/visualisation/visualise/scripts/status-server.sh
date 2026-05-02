#!/usr/bin/env bash
set -euo pipefail
umask 077

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$PLUGIN_ROOT/scripts/vcs-common.sh"
source "$SCRIPT_DIR/launcher-helpers.sh"

PROJECT_ROOT="$(find_repo_root)"
cd "$PROJECT_ROOT"

TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp meta/tmp)"
TMP_DIR="$PROJECT_ROOT/$TMP_REL/visualiser"
INFO="$TMP_DIR/server-info.json"
PID_FILE="$TMP_DIR/server.pid"

stop_server_status
