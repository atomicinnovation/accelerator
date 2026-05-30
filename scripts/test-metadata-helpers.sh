#!/usr/bin/env bash
set -euo pipefail

# Metadata-helper output contract test. Runs each of the three helpers
# inside a hermetically isolated temp repository (both git and, when
# available, jj) and asserts the unified output shape:
#   - ISO `+00:00` UTC timestamp on a label-anchored line
#   - `Current Revision:` label (replaces `Current Git Commit Hash:`)
#   - `Repository Name:` label
#   - no `Current Branch Name:` / `GIT_BRANCH` line
#   - no non-ISO `%Z`-shaped timestamp

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=test-helpers.sh
source "$SCRIPT_DIR/test-helpers.sh"
cd "$ROOT"

echo "=== Metadata-helper output ==="

HELPERS=(
  "$ROOT/scripts/artifact-derive-metadata.sh"
  "$ROOT/skills/design/inventory-design/scripts/inventory-metadata.sh"
  "$ROOT/skills/design/analyse-design-gaps/scripts/gap-metadata.sh"
)

# Run a helper script inside a clean VCS-isolated subshell.
run_helper_in_clean_repo() {
  local vcs="$1" helper="$2"
  local tmpdir
  tmpdir=$(mktemp -d)
  (
    trap "rm -rf '$tmpdir'" EXIT
    unset GIT_DIR GIT_WORK_TREE JJ_CONFIG
    export HOME="$tmpdir"
    export XDG_CONFIG_HOME="$tmpdir/.config"

    mkdir -p "$tmpdir/repo"
    cd "$tmpdir/repo"

    case "$vcs" in
      git)
        git -c init.defaultBranch=main -c commit.gpgsign=false init -q .
        git -c user.email=test@example.com -c user.name=Test \
            -c commit.gpgsign=false \
            commit --allow-empty -q -m init
        ;;
      jj)
        jj git init --colocate . >/dev/null 2>&1
        ;;
      *)
        echo "run_helper_in_clean_repo: unknown vcs '$vcs'" >&2
        return 2
        ;;
    esac

    bash "$helper"
  )
}

assert_helper_output() {
  local label="$1" output="$2"

  assert_matches_regex \
    "$label: Current Revision label present with non-empty value" \
    '^Current Revision:[[:space:]]+[^[:space:]]+$' \
    "$output"

  assert_matches_regex \
    "$label: Current Date/Time (UTC) is ISO +00:00" \
    '^Current Date/Time \(UTC\):[[:space:]]+[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\+00:00$' \
    "$output"

  assert_not_matches_regex \
    "$label: no Current Branch Name line" \
    '^Current Branch Name:' \
    "$output"

  assert_not_matches_regex \
    "$label: no GIT_BRANCH= line" \
    '^GIT_BRANCH=' \
    "$output"

  assert_not_matches_regex \
    "$label: no Current Git Commit Hash line" \
    '^Current Git Commit Hash:' \
    "$output"

  assert_not_matches_regex \
    "$label: no non-ISO %Z-style timestamp" \
    '[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2} [A-Z]+' \
    "$output"
}

JJ_AVAILABLE=0
if command -v jj >/dev/null 2>&1; then
  JJ_AVAILABLE=1
fi

for helper in "${HELPERS[@]}"; do
  helper_basename=$(basename "$helper")
  echo "--- $helper_basename ---"

  # git branch
  output=$(run_helper_in_clean_repo git "$helper" || true)
  assert_helper_output "$helper_basename (git)" "$output"

  # jj branch
  if [ "$JJ_AVAILABLE" = 1 ]; then
    output=$(run_helper_in_clean_repo jj "$helper" || true)
    assert_helper_output "$helper_basename (jj)" "$output"
  else
    skip_test "$helper_basename (jj)" "jj not installed"
  fi
done

test_summary
