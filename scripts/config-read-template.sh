#!/usr/bin/env bash
set -euo pipefail

# Thin shim: route to the `a9r config-read-template` subcommand when a trusted
# binary resolves, else run the verbatim bash implementation. Both paths are
# proven byte-for-byte equivalent by the parity gate (scripts/test-config.sh +
# test-config-parity.sh). Resolution precedence and trust gates live in
# a9r-resolve.sh; A9R_FORCE_BASH forces the bash path.
#
# The a9r subcommand cannot derive the plugin root from its own path, so the
# shim exports ACCELERATOR_PLUGIN_ROOT (the parent of scripts/) — the same
# value the bash impl computes from its SCRIPT_DIR/.. — before exec'ing it.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=a9r-resolve.sh
source "$SCRIPT_DIR/a9r-resolve.sh"

if [ -z "${A9R_FORCE_BASH:-}" ] && bin="$(a9r_bin 2>/dev/null)" && [ -n "$bin" ]; then
  ACCELERATOR_PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
  export ACCELERATOR_PLUGIN_ROOT
  exec "$bin" config-read-template "$@"
fi
exec "$SCRIPT_DIR/config-read-template-impl.sh" "$@"
