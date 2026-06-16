#!/usr/bin/env bash
set -euo pipefail

# Thin shim: route to the `a9r artifact-derive-metadata` subcommand when a
# trusted binary resolves, else run the verbatim bash implementation. Output is
# live (timestamps, VCS revision), so the two backends are not byte-identical;
# the contract is the output *shape*, gated by test-metadata-helpers.sh (which
# runs in a9r mode via this shim). Resolution lives in a9r-resolve.sh;
# A9R_FORCE_BASH forces the bash path.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=a9r-resolve.sh
source "$SCRIPT_DIR/a9r-resolve.sh"

if [ -z "${A9R_FORCE_BASH:-}" ] && bin="$(a9r_bin 2>/dev/null)" && [ -n "$bin" ]; then
  exec "$bin" artifact-derive-metadata "$@"
fi
exec "$SCRIPT_DIR/artifact-derive-metadata-impl.sh" "$@"
