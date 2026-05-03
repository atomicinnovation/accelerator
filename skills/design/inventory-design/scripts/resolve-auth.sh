#!/usr/bin/env bash
set -euo pipefail

# Resolves authentication mode from ACCELERATOR_BROWSER_* environment variables.
#
# Usage: resolve-auth.sh
#
# Outputs one of: header | form | none
# Exits non-zero with a message to stderr when partial form-login vars are set.
#
# Precedence:
#   1. ACCELERATOR_BROWSER_AUTH_HEADER set → output "header", warn if form vars also set
#   2. All three of USERNAME + PASSWORD + LOGIN_URL set → output "form"
#   3. Any (but not all) of USERNAME/PASSWORD/LOGIN_URL set → exit 1 with clear error
#   4. None set → output "none"

AUTH_HEADER="${ACCELERATOR_BROWSER_AUTH_HEADER:-}"
USERNAME="${ACCELERATOR_BROWSER_USERNAME:-}"
PASSWORD="${ACCELERATOR_BROWSER_PASSWORD:-}"
LOGIN_URL="${ACCELERATOR_BROWSER_LOGIN_URL:-}"

if [ -n "$AUTH_HEADER" ]; then
  # Header wins; warn if any form-login vars are also set
  IGNORED=()
  [ -n "$USERNAME" ] && IGNORED+=("ACCELERATOR_BROWSER_USERNAME")
  [ -n "$PASSWORD" ] && IGNORED+=("ACCELERATOR_BROWSER_PASSWORD")
  [ -n "$LOGIN_URL" ] && IGNORED+=("ACCELERATOR_BROWSER_LOGIN_URL")

  if [ "${#IGNORED[@]}" -gt 0 ]; then
    IGNORED_LIST=$(IFS=", "; echo "${IGNORED[*]}")
    echo "warning: ACCELERATOR_BROWSER_AUTH_HEADER is set; form-login vars ignored: ${IGNORED_LIST}" >&2
  fi

  echo "header"
  exit 0
fi

# Count how many form-login vars are set
SET_COUNT=0
[ -n "$USERNAME" ] && SET_COUNT=$((SET_COUNT + 1))
[ -n "$PASSWORD" ] && SET_COUNT=$((SET_COUNT + 1))
[ -n "$LOGIN_URL" ] && SET_COUNT=$((SET_COUNT + 1))

if [ "$SET_COUNT" -eq 3 ]; then
  echo "form"
  exit 0
fi

if [ "$SET_COUNT" -gt 0 ]; then
  # Partial set — fail fast and name the missing var(s)
  MISSING=()
  [ -z "$USERNAME" ] && MISSING+=("ACCELERATOR_BROWSER_USERNAME")
  [ -z "$PASSWORD" ] && MISSING+=("ACCELERATOR_BROWSER_PASSWORD")
  [ -z "$LOGIN_URL" ] && MISSING+=("ACCELERATOR_BROWSER_LOGIN_URL")

  MISSING_LIST=$(IFS=", "; echo "${MISSING[*]}")
  echo "error: partial form-login configuration — missing: ${MISSING_LIST}. Set all three of ACCELERATOR_BROWSER_USERNAME, ACCELERATOR_BROWSER_PASSWORD, and ACCELERATOR_BROWSER_LOGIN_URL together, or use ACCELERATOR_BROWSER_AUTH_HEADER instead." >&2
  exit 1
fi

echo "none"
exit 0
