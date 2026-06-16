#!/usr/bin/env bash
set -euo pipefail

# Thin shim: route to the `a9r config-read-skill-instructions` subcommand when
# a trusted binary resolves, else run the verbatim bash implementation. Both
# paths are proven byte-for-byte equivalent by the parity gate
# (scripts/test-config.sh). Resolution precedence and trust gates live in
# a9r-resolve.sh; A9R_FORCE_BASH forces the bash path.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=a9r-resolve.sh
source "$SCRIPT_DIR/a9r-resolve.sh"

if [ -z "${A9R_FORCE_BASH:-}" ] && bin="$(a9r_bin 2>/dev/null)" && [ -n "$bin" ]; then
  exec "$bin" config-read-skill-instructions "$@"
fi
exec "$SCRIPT_DIR/config-read-skill-instructions-impl.sh" "$@"
