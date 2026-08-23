#!/usr/bin/env bash
# Captures the Jira bash cluster's exit-code contract into bash-exit-codes.txt,
# the authoritative name->integer oracle exit_codes_parity.rs pins against.
#
# Unlike the Linear flows, the Jira scripts emit bare-literal `exit N` rather
# than `readonly E_*=NN` constants, so the declared contract is the namespace
# table in scripts/EXIT_CODES.md — the single document every helper draws from.
# This script parses that table into `<rust-const-name>=<bash-integer>` rows,
# plus the unnamed shared HTTP codes the table lists with a `—` name (the Rust
# binary reads them structurally from the client). Two normalisations:
#   - `E_ADF_UNSUPPORTED_*` (a wildcard family) -> ADF_UNSUPPORTED.
#   - code 26 (E_TOKEN_CMD_FROM_SHARED_CONFIG) is a stderr prefix only, never a
#     fatal exit, so it is omitted (EXIT_CODES.md:28 flags this).
#   - the E_BODY_* helper codes (1-6) are caller-namespaced: every caller remaps
#     them to its own flow code (create->105, comment->94, update->116,
#     transition->125), so they are not part of the binary's taxonomy and are
#     omitted.
#
# Kept for provenance; re-run against EXIT_CODES.md at the recorded revision.
# The search codes stay at their bash values (70-73) here — exit_codes.rs remaps
# them off the reserved 70-74 dispatch band, and the parity allowlist records
# that.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
TABLE="$PLUGIN_ROOT/skills/integrations/jira/scripts/EXIT_CODES.md"
OUT="$SCRIPT_DIR/bash-exit-codes.txt"

{
  echo "# Jira bash exit-code contract, captured by capture-bash-exit-codes.sh"
  echo "# from scripts/EXIT_CODES.md. <rust-const-name>=<bash-integer>."
  echo "# exit_codes_parity.rs pins these."
  echo ""
  echo "# Unnamed shared HTTP codes (a '—' name in the table; the Rust binary"
  echo "# reads them structurally from bash_code(outcome))."
  echo "UNAUTHORIZED=11"
  echo "FORBIDDEN=12"
  echo "NOT_FOUND=13"
  echo "GONE=14"
  echo "RATELIMITED=19"
  echo "SERVER_ERROR=20"
  echo ""
  echo "# Named codes, grepped from the EXIT_CODES.md namespace table."
  grep -oE '^\| [0-9]+ +\| `E_[A-Z_]+\*?`' "$TABLE" |
    sed -E 's/^\| ([0-9]+) +\| `E_([A-Z_]+)\*?`/\2=\1/' |
    sed -E 's/^ADF_UNSUPPORTED_=/ADF_UNSUPPORTED=/' |
    grep -vE '^TOKEN_CMD_FROM_SHARED_CONFIG=|^BODY_' |
    sort -t= -k2 -n
} >"$OUT"

echo "captured $(grep -cvE '^#|^$' "$OUT") codes into $OUT"
