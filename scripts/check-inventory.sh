#!/usr/bin/env bash
set -euo pipefail

# Verifies the 0167 inventory records against a pinned pre-deletion revision, so
# the check stays meaningful after Phase 7 deletes the removal set. A working-
# tree extraction would yield nothing once the scripts are gone and pass
# trivially at exactly the moment it matters, so the removal-set floor reads
# from the pinned revision via `jj file show`, never the working tree.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INV="$REPO_ROOT/meta/inventories"

# A commit where the removal set is whole. change_ids are stable across rebases.
PINNED_REV="vnorwskwqlrv"

# The canonical removal set (21 paths). Kept here as the single list the gate
# reconciles the inventory against.
REMOVAL_SET="
scripts/config-read-value.sh
scripts/config-read-path.sh
scripts/config-read-all-paths.sh
scripts/config-read-doc-type-paths.sh
scripts/config-read-work.sh
scripts/config-read-agents.sh
scripts/config-read-agent-name.sh
scripts/config-read-context.sh
scripts/config-read-review.sh
scripts/config-read-skill-context.sh
scripts/config-read-skill-instructions.sh
scripts/config-read-template.sh
scripts/config-list-template.sh
scripts/config-show-template.sh
scripts/config-eject-template.sh
scripts/config-diff-template.sh
scripts/config-reset-template.sh
scripts/config-dump.sh
scripts/config-summary.sh
skills/config/init/scripts/init.sh
"

REMOVAL_SET_FLOOR=20

# Config suites that (directly or via a sourced helper) exercise a removal-set
# script — the set that keeps member 4 empty.
COVERING_SUITES="
scripts/test-config.sh
scripts/test-config-read-doc-type-paths.sh
skills/config/init/scripts/test-init.sh
"

fail() {
  echo "check-inventory: $1" >&2
  exit 1
}

require_jj() {
  command -v jj >/dev/null 2>&1 || fail "jj not on PATH — the pinned-revision \
extraction cannot run; the removal-set floor requires it"
}

# 1. Removal-set floor at the pinned revision (anti-tautology).
check_removal_set_floor() {
  require_jj
  local count=0 path
  for path in $REMOVAL_SET; do
    if jj file show -r "$PINNED_REV" "$path" >/dev/null 2>&1; then
      count=$((count + 1))
    else
      fail "removal-set path absent at pinned rev $PINNED_REV: $path"
    fi
  done
  if [ "$count" -lt "$REMOVAL_SET_FLOOR" ]; then
    fail "removal-set floor: found $count, expected >= $REMOVAL_SET_FLOOR"
  fi
  echo "check-inventory: removal-set floor ok ($count files at $PINNED_REV)"
}

# 2. Every test named in the divergences record resolves to a real symbol.
check_divergence_tests() {
  local div="$INV/0167-divergences.md" ref file name src
  [ -f "$div" ] || fail "missing $div"
  local found=0
  while IFS= read -r ref; do
    file="${ref%%::*}"
    name="${ref##*::}"
    case "$file" in
      read.rs) src="cli/launcher/tests/config_read.rs" ;;
      parity.rs) src="cli/config-adapters/tests/parity.rs" ;;
      store.rs) src="cli/config-adapters/src/store.rs" ;;
      compose.rs) src="cli/config-adapters/src/compose.rs" ;;
      *) fail "divergences names an unknown test file: $file" ;;
    esac
    if ! grep -qE "fn ${name}\b" "$REPO_ROOT/$src"; then
      fail "divergences names a test that does not resolve: $ref (in $src)"
    fi
    found=$((found + 1))
  done < <(grep -oE '(read|parity|store|compose)\.rs::[a-z_]+' "$div" | sort -u)
  [ "$found" -gt 0 ] || fail "divergences record names no tests"
  echo "check-inventory: divergence tests ok ($found resolve)"
}

# 3. The deletion ledger has a row for every removal-set path.
check_deletion_ledger() {
  local ledger="$INV/0167-deletion-ledger.md" path
  [ -f "$ledger" ] || fail "missing $ledger"
  for path in $REMOVAL_SET; do
    grep -qF "$path" "$ledger" || fail "deletion ledger omits: $path"
  done
  echo "check-inventory: deletion ledger ok (covers the removal set)"
}

# 4. Member 4 is empty: every removal-set script is named by a covering suite,
#    read from the pinned revision so a Phase-7-deleted suite still counts.
check_member_four_empty() {
  require_jj
  local path base suite content covered uncovered=""
  for path in $REMOVAL_SET; do
    base="$(basename "$path" .sh)"
    covered=0
    for suite in $COVERING_SUITES; do
      # Capture then substring-match: a piped `grep -q` closing the pipe early
      # would SIGPIPE `jj file show`, which `pipefail` then reports as failure.
      content="$(jj file show -r "$PINNED_REV" "$suite" 2>/dev/null || true)"
      case "$content" in
        *"$base"*)
          covered=1
          break
          ;;
      esac
    done
    [ "$covered" -eq 1 ] || uncovered="$uncovered $base"
  done
  if [ -n "$uncovered" ]; then
    fail "member 4 is not empty — uncovered removal-set scripts:$uncovered. \
List them in the behaviour inventory or add a covering suite."
  fi
  echo "check-inventory: member 4 empty (every script has a covering suite)"
}

check_removal_set_floor
check_divergence_tests
check_deletion_ledger
check_member_four_empty
echo "check-inventory: OK"
