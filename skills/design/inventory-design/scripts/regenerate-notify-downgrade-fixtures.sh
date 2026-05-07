#!/usr/bin/env bash
# Regenerates evals/fixtures/notify-downgrade/<key>.expected.txt from
# notify-downgrade-messages.json. Run after editing the messages JSON.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MESSAGES_JSON="$SCRIPT_DIR/notify-downgrade-messages.json"
FIXTURES_DIR="$SCRIPT_DIR/../evals/fixtures/notify-downgrade"

mkdir -p "$FIXTURES_DIR"

jq -r 'keys[]' "$MESSAGES_JSON" | while read -r key; do
  bash "$SCRIPT_DIR/notify-downgrade.sh" --reason "$key" \
    > "$FIXTURES_DIR/${key}.expected.txt"
  echo "Written: ${key}.expected.txt"
done

echo "Done."
