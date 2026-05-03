#!/usr/bin/env bash
set -euo pipefail

# Audits a design-gap document for cue-phrase compliance.
#
# Usage: audit-cue-phrases.sh <file-path>
#
# For each non-empty H2 section in the file, asserts that at least one paragraph
# matches the canonical cue-phrase list from
# scripts/extract-work-items-cue-phrases.txt.
#
# The first three patterns (we need to, users? need, the system must) are applied
# case-insensitively. The fourth (implement [A-Z]) is applied case-sensitively so
# that "implement Foo" (proper noun feature name) matches but "implement foo" does not.
#
# Exits 0 if all non-empty H2 sections have at least one cue-phrase paragraph.
# Exits 1 and prints the offending section name(s) to stderr if any fail.

FILE="${1:-}"

if [ -z "$FILE" ]; then
  echo "error: audit-cue-phrases.sh requires a file path argument" >&2
  exit 1
fi

if [ ! -f "$FILE" ]; then
  echo "error: file not found: $FILE" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CUE_FILE="$(cd "$SCRIPT_DIR/../../../.." && pwd)/scripts/extract-work-items-cue-phrases.txt"

if [ ! -f "$CUE_FILE" ]; then
  echo "error: cue-phrases file not found: $CUE_FILE" >&2
  exit 1
fi

# Build the case-insensitive pattern (all non-comment, non-implement lines).
CASE_INSENSITIVE_PATTERN=$(grep -vE '^\s*#|\[Ii\]mplement' "$CUE_FILE" | grep -v '^$' | paste -sd '|' -)

# Parse H2 sections and check each for cue-phrase compliance.
FAILED_SECTIONS=()

# Use awk to split document into H2 sections.
# Each section is output as: SECTION_NAME\x00\nSECTION_CONTENT\x00\n
# We process line-by-line to collect sections.

CURRENT_SECTION=""
CURRENT_CONTENT=""
FAILED=0

check_section() {
  local section_name="$1"
  local section_content="$2"

  # Skip empty sections (only whitespace)
  if [ -z "$(echo "$section_content" | tr -d '[:space:]')" ]; then
    return 0
  fi

  # Check case-insensitive patterns first
  if [ -n "$CASE_INSENSITIVE_PATTERN" ] && echo "$section_content" | grep -qiE "$CASE_INSENSITIVE_PATTERN"; then
    return 0
  fi

  # Check [Ii]mplement [A-Z] case-sensitively for the uppercase word requirement
  if echo "$section_content" | grep -qE "[Ii]mplement [A-Z]"; then
    return 0
  fi

  # No cue phrase found
  echo "error: H2 section '${section_name}' has no cue-phrase paragraph. Add prose matching one of: we need to / users need / the system must / implement <ProperNoun>." >&2
  return 1
}

while IFS= read -r line || [ -n "$line" ]; do
  if [[ "$line" =~ ^##[[:space:]] ]]; then
    # Process the previous section if it exists
    if [ -n "$CURRENT_SECTION" ]; then
      check_section "$CURRENT_SECTION" "$CURRENT_CONTENT" || FAILED=$((FAILED + 1))
    fi
    CURRENT_SECTION="${line#\#\# }"
    CURRENT_CONTENT=""
  elif [[ "$line" =~ ^#[[:space:]] ]]; then
    # H1 heading — process and reset current section
    if [ -n "$CURRENT_SECTION" ]; then
      check_section "$CURRENT_SECTION" "$CURRENT_CONTENT" || FAILED=$((FAILED + 1))
    fi
    CURRENT_SECTION=""
    CURRENT_CONTENT=""
  else
    CURRENT_CONTENT="${CURRENT_CONTENT}"$'\n'"$line"
  fi
done < "$FILE"

# Process the last section
if [ -n "$CURRENT_SECTION" ]; then
  check_section "$CURRENT_SECTION" "$CURRENT_CONTENT" || FAILED=$((FAILED + 1))
fi

if [ "$FAILED" -gt 0 ]; then
  exit 1
fi

exit 0
