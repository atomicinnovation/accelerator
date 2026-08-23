#!/usr/bin/env bash
# Captures the Linear bash flows' exit-code contract into bash-exit-codes.txt,
# the authoritative name->integer oracle exit_codes_parity.rs pins against.
#
# Two sources, mirroring how the bash declares them:
#   - The per-flow argument/outcome codes are `readonly E_*=NN` constants, so
#     they are grepped straight out of the flow scripts (E_ prefix stripped to
#     match the Rust const names).
#   - The shared transport/auth/common codes (11-53) are bare-literal
#     return/exit integers, not `readonly` constants (verified against
#     EXIT_CODES.md), so they are declared here under the Rust const names the
#     binary maps them to.
#
# Kept for provenance; re-run against the flows at the recorded revision. The
# search codes stay at their bash values (70-73) here — exit_codes.rs remaps
# them off the reserved 70-74 band, and the parity allowlist records that.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
FLOWS="$PLUGIN_ROOT/skills/integrations/linear/scripts"
OUT="$SCRIPT_DIR/bash-exit-codes.txt"

{
  echo "# Linear bash exit-code contract, captured by capture-bash-exit-codes.sh."
  echo "# <rust-const-name>=<bash-integer>. exit_codes_parity.rs pins these."
  echo ""
  echo "# Shared transport/auth/common codes (bare-literal in linear-graphql.sh"
  echo "# and linear-auth.sh; the Rust binary reads them structurally)."
  echo "UNAUTHORIZED=11"
  echo "BAD_RESPONSE=16"
  echo "SERVER_ERROR=20"
  echo "CONNECT=21"
  echo "NO_CREDS=22"
  echo "NO_TOKEN=24"
  echo "TOKEN_CMD_FAILED=25"
  echo "TOKEN_MALFORMED=27"
  echo "LOCAL_PERMS_INSECURE=29"
  echo "BAD_REQUEST=34"
  echo "RATELIMITED=35"
  echo "COMPLEXITY=36"
  echo "REFRESH_LOCKED=53"
  echo ""
  echo "# Per-flow codes, grepped from each flow's readonly E_*=NN block."
  for flow in "$FLOWS"/linear-*-flow.sh; do
    grep -oE 'readonly E_[A-Z_]+=[0-9]+' "$flow" |
      sed -E 's/^readonly E_//'
  done | sort -t= -k2 -n
} >"$OUT"

echo "captured $(grep -cvE '^#|^$' "$OUT") codes into $OUT"
