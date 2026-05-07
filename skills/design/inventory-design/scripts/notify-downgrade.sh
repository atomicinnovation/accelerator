#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MESSAGES_JSON="$SCRIPT_DIR/notify-downgrade-messages.json"

usage() {
  echo "usage: notify-downgrade.sh --reason <enum> [--from <mode>] [--to <mode>]" >&2
  exit 2
}

REASON=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --reason) REASON="${2:-}"; shift 2 ;;
    --from)   shift 2 ;;  # accepted for forward-compat; not used in message selection
    --to)     shift 2 ;;  # accepted for forward-compat; not used in message selection
    *) usage ;;
  esac
done

[[ -z "$REASON" ]] && usage

# Validate reason is a known key
if ! jq -e --arg r "$REASON" 'has($r)' "$MESSAGES_JSON" >/dev/null 2>&1; then
  echo "error: notify-downgrade.sh: unknown --reason '$REASON'" >&2
  echo "       Valid values: $(jq -r 'keys | join(", ")' "$MESSAGES_JSON")" >&2
  exit 2
fi

# Read message from JSON
MESSAGE="$(jq -r --arg r "$REASON" '.[$r]' "$MESSAGES_JSON")"

# Reject bidi-override codepoints (U+202A-U+202E, U+2066-U+2069)
if printf '%s' "$MESSAGE" | grep -qP '[\x{202a}-\x{202e}\x{2066}-\x{2069}]' 2>/dev/null; then
  echo "error: notify-downgrade.sh: message contains bidi-override codepoints" >&2
  exit 1
fi

# Strip bytes outside printable ASCII (keep 0x20-0x7E and newline 0x0A)
# shellcheck disable=SC2020
printf '%s\n' "$MESSAGE" | LC_ALL=C tr -cd '\n\040-\176'
