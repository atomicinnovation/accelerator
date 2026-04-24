#!/usr/bin/env bash
set -euo pipefail

# Lint every ticket review lens SKILL.md for structural conformance.
# When a single lens directory name is given (e.g. "scope-lens") only that
# lens is checked; otherwise all *-lens directories under the lenses base are
# checked.
#
# Structural checks apply to every lens.  The peer-ticket-lens reference check
# applies only to lenses whose identifier appears in TICKET_LENSES (the five
# built-in ticket lenses) because code-review lenses are not expected to
# reference ticket-specific peers.
#
# Usage:
#   scripts/test-lens-structure.sh [lens-dir-name]
#
# Exit code: 0 if all assertions pass, 1 if any fail.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

LENSES_BASE="$SCRIPT_DIR/../skills/review/lenses"

# Built-in ticket lens identifiers — peer-reference check applies only to these.
TICKET_LENSES=(clarity completeness dependency scope testability)

_is_ticket_lens() {
  local id="$1"
  for tl in "${TICKET_LENSES[@]}"; do
    [ "$id" = "$tl" ] && return 0
  done
  return 1
}

# Determine which lenses to lint.
if [ "${1:-}" != "" ]; then
  LENS_DIRS=("$LENSES_BASE/$1")
else
  LENS_DIRS=()
  while IFS= read -r -d '' dir; do
    LENS_DIRS+=("$dir")
  done < <(find "$LENSES_BASE" -maxdepth 1 -name '*-lens' -type d -print0 | sort -z)
fi

for LENS_DIR in "${LENS_DIRS[@]}"; do
  SKILL_FILE="$LENS_DIR/SKILL.md"
  LENS_NAME="$(basename "$LENS_DIR")"  # e.g. "scope-lens"
  LENS_ID="${LENS_NAME%-lens}"         # e.g. "scope"

  echo "=== Linting $LENS_NAME ==="

  if [ ! -f "$SKILL_FILE" ]; then
    echo "  FAIL: $SKILL_FILE does not exist"
    FAIL=$((FAIL + 1))
    continue
  fi

  # Extract frontmatter (between first and second ---)
  FRONTMATTER="$(awk '/^---$/{n++; if(n==1){next} if(n==2){exit}} n==1{print}' "$SKILL_FILE")"

  # --- Frontmatter checks ---

  if echo "$FRONTMATTER" | grep -q "^user-invocable: false$"; then
    echo "  PASS: $LENS_NAME has user-invocable: false"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing 'user-invocable: false' in frontmatter"
    FAIL=$((FAIL + 1))
  fi

  if echo "$FRONTMATTER" | grep -q "^disable-model-invocation: true$"; then
    echo "  PASS: $LENS_NAME has disable-model-invocation: true"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing 'disable-model-invocation: true' in frontmatter"
    FAIL=$((FAIL + 1))
  fi

  NAME_VAL="$(echo "$FRONTMATTER" | grep "^name:" | head -1 | sed 's/^name:[[:space:]]*//')"
  if [ -n "$NAME_VAL" ]; then
    echo "  PASS: $LENS_NAME has non-empty 'name' in frontmatter"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing or empty 'name' in frontmatter"
    FAIL=$((FAIL + 1))
  fi

  if echo "$FRONTMATTER" | grep -q "^description:"; then
    echo "  PASS: $LENS_NAME has 'description' in frontmatter"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing 'description' in frontmatter"
    FAIL=$((FAIL + 1))
  fi

  # --- Section heading checks ---

  for HEADING in \
    "## Core Responsibilities" \
    "## Key Evaluation Questions" \
    "## Important Guidelines" \
    "## What NOT to Do"
  do
    if grep -qF "$HEADING" "$SKILL_FILE"; then
      echo "  PASS: $LENS_NAME has '$HEADING'"
      PASS=$((PASS + 1))
    else
      echo "  FAIL: $LENS_NAME missing '$HEADING'"
      FAIL=$((FAIL + 1))
    fi
  done

  # --- H1 heading check ---
  if grep -q "^# " "$SKILL_FILE"; then
    echo "  PASS: $LENS_NAME has H1 heading"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing H1 heading"
    FAIL=$((FAIL + 1))
  fi

  # --- Persona sentence check ---
  # A single persona sentence must exist between the H1 and the first ## heading.
  # For ticket lenses this should follow the "Review as a[n] ... specialist ..."
  # shape; for code-review lenses any non-empty line is accepted.
  PERSONA_LINE="$(awk '/^# /{found_h1=1; next} found_h1 && /^## /{exit} found_h1 && /[[:alnum:]]/{print; exit}' "$SKILL_FILE")"
  if [ -n "$PERSONA_LINE" ]; then
    echo "  PASS: $LENS_NAME has persona sentence between H1 and first ##"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing persona sentence between H1 and first ##"
    FAIL=$((FAIL + 1))
  fi

  # --- What NOT to Do: peer ticket-lens references (ticket lenses only) ---
  if _is_ticket_lens "$LENS_ID"; then
    WHAT_NOT_BODY="$(awk '/^## What NOT to Do/{found=1; next} found && /^## /{exit} found{print}' "$SKILL_FILE")"
    PEER_COUNT=0
    for PEER in "${TICKET_LENSES[@]}"; do
      [ "$PEER" = "$LENS_ID" ] && continue
      if echo "$WHAT_NOT_BODY" | grep -qE "\b$PEER\b"; then
        PEER_COUNT=$((PEER_COUNT + 1))
      fi
    done
    if [ "$PEER_COUNT" -ge 3 ]; then
      echo "  PASS: $LENS_NAME 'What NOT to Do' names at least 3 peer ticket lenses ($PEER_COUNT found)"
      PASS=$((PASS + 1))
    else
      echo "  FAIL: $LENS_NAME 'What NOT to Do' names only $PEER_COUNT peer ticket lenses (need >= 3)"
      FAIL=$((FAIL + 1))
    fi
  fi

  # --- Closing Remember: paragraph check ---
  if grep -q "^Remember:" "$SKILL_FILE"; then
    echo "  PASS: $LENS_NAME has closing 'Remember:' paragraph"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $LENS_NAME missing closing 'Remember:' paragraph"
    FAIL=$((FAIL + 1))
  fi

done

echo ""
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
if [ "$FAIL" -eq 0 ]; then
  echo "All lens structure checks passed!"
else
  echo "Some lens structure checks failed."
  exit 1
fi
