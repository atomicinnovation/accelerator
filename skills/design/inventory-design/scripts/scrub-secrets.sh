#!/usr/bin/env bash
set -euo pipefail

# Pre-write secret scrubber for inventory-design artifacts.
#
# Usage: scrub-secrets.sh <file-path>
#
# Reads the literal values of all set ACCELERATOR_BROWSER_* environment
# variables and checks whether any appear verbatim in the given file.
# Exits non-zero and names the offending env var (NOT its value) if a
# literal match is found. Exits 0 if the file is clean.

FILE="${1:-}"

if [ -z "$FILE" ]; then
  echo "error: scrub-secrets.sh requires a file path argument" >&2
  exit 1
fi

if [ ! -f "$FILE" ]; then
  echo "error: file not found: $FILE" >&2
  exit 1
fi

FOUND=0

check_var() {
  local var_name="$1"
  local var_value="${!var_name:-}"
  # Skip empty values — nothing to scrub
  [ -z "$var_value" ] && return 0
  if grep -qF "$var_value" "$FILE"; then
    echo "error: the literal value of ${var_name} appears in the generated inventory body. The artifact was not written. Check your content for accidental secret leakage." >&2
    FOUND=1
  fi
}

check_var "ACCELERATOR_BROWSER_AUTH_HEADER"
check_var "ACCELERATOR_BROWSER_USERNAME"
check_var "ACCELERATOR_BROWSER_PASSWORD"
check_var "ACCELERATOR_BROWSER_LOGIN_URL"

if [ "$FOUND" -ne 0 ]; then
  exit 1
fi

exit 0
