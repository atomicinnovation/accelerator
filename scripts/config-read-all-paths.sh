#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Source config-common.sh to load PATH_KEYS and PATH_DEFAULTS from config-defaults.sh.
# Note: each config-read-value.sh subprocess call re-triggers VCS detection in its own
# process — the parent sourcing does not eliminate that cost (11 VCS detections total).
# shellcheck source=config-common.sh
source "$SCRIPT_DIR/config-common.sh"

# Non-document keys excluded from the document-discovery output.
# All PATH_KEYS not in this exclusion list are emitted automatically, so new document
# path keys added to config-defaults.sh appear here without editing this script.
EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)

_is_excluded() {
  local key="$1"
  for excl in "${EXCLUDED_KEYS[@]}"; do
    [ "$key" = "$excl" ] && return 0
  done
  return 1
}

echo "## Configured Paths"
echo ""
for i in "${!PATH_KEYS[@]}"; do
  full_key="${PATH_KEYS[$i]}"   # e.g. paths.global
  key="${full_key#paths.}"      # strip prefix → global
  _is_excluded "$key" && continue
  default="${PATH_DEFAULTS[$i]}"
  value=$("$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}")
  echo "- ${key}: ${value}"
done
