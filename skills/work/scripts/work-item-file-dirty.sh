#!/usr/bin/env bash
set -euo pipefail

# work-item-file-dirty.sh — a small VCS-mode-aware predicate guarding the
# pull-overwrite path. The recovery model for a clobbered local file is VCS
# revert, which CANNOT recover working-copy changes not yet captured in a commit
# — under BOTH supported VCSs. So before any local-overwrite write, /sync-work-
# items asks this whether the file has uncommitted working-copy changes.
#
# Usage:
#   work-item-file-dirty.sh <path>
#
# Exit: 0 if the file is DIRTY (uncommitted working-copy changes), 1 if clean.
#
# Mode resolution is `.jj`-present-WINS via vcs_mode() (jj-colocated ⇒ jj, never
# git, whose index lags the jj working copy). Dispatch:
#   jj   → the path appears in `jj --no-pager diff --name-only` for @ (the
#          --no-pager flag matches run-migrations and stops a configured pager
#          hanging or injecting control codes into captured output).
#   git  → `git status --porcelain -- <path>` is non-empty. An untracked path
#          (^??) counts as DIRTY (a deliberate deviation from run-migrations'
#          `grep -v '^??'`: an untracked file is not VCS-recoverable either, so
#          overwriting it must not be silent).
# Indeterminate VCS mode (no .jj and no .git, or detection fails) → FAIL SAFE to
# DIRTY, so the overwrite is routed to prompt/skip rather than proceeding.
#
# Test seam (gated on ACCELERATOR_TEST_MODE=1): WORK_DIRTY_MODE_OVERRIDE forces
# the resolved mode (jj|git|none) and WORK_DIRTY_STATUS_OVERRIDE injects the
# per-VCS status output, so the guard is exercised under git / jj / jj-colocated
# / indeterminate without a real working copy.

_WIFD_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/vcs-common.sh
source "$(cd "$_WIFD_DIR/../../.." && pwd)/scripts/vcs-common.sh"

_wifd_test_mode() { [ "${ACCELERATOR_TEST_MODE:-}" = "1" ]; }

# Resolve the VCS command-set mode, honouring the test override only in test mode.
_wifd_mode() {
  local root="$1"
  if _wifd_test_mode && [ -n "${WORK_DIRTY_MODE_OVERRIDE:-}" ]; then
    printf '%s\n' "$WORK_DIRTY_MODE_OVERRIDE"
    return 0
  fi
  vcs_mode "$root"
}

# jj changed-path list for @ (or the injected stub in test mode).
_wifd_jj_changed() {
  local root="$1"
  if _wifd_test_mode && [ -n "${WORK_DIRTY_STATUS_OVERRIDE:-}" ]; then
    printf '%s\n' "$WORK_DIRTY_STATUS_OVERRIDE"
    return 0
  fi
  (cd "$root" && jj --no-pager diff --name-only 2>/dev/null) || true
}

# git porcelain for one path (or the injected stub in test mode).
_wifd_git_porcelain() {
  local root="$1" path="$2"
  if _wifd_test_mode && [ -n "${WORK_DIRTY_STATUS_OVERRIDE:-}" ]; then
    printf '%s' "$WORK_DIRTY_STATUS_OVERRIDE"
    return 0
  fi
  (cd "$root" && git status --porcelain -- "$path" 2>/dev/null) || true
}

_wifd_main() {
  local path="${1-}"
  if [ -z "$path" ]; then
    echo "Usage: work-item-file-dirty.sh <path>" >&2
    return 2
  fi

  local root mode
  root=$(find_repo_root 2>/dev/null) || root=""
  if [ -z "$root" ]; then
    # Indeterminate: cannot locate a repo → fail safe to dirty.
    return 0
  fi
  mode=$(_wifd_mode "$root")

  case "$mode" in
    jj)
      local relpath changed
      relpath="${path#"$root"/}"
      changed=$(_wifd_jj_changed "$root")
      if printf '%s\n' "$changed" | grep -qxF "$relpath"; then
        return 0 # dirty
      fi
      return 1 # clean
      ;;
    git)
      local porcelain
      porcelain=$(_wifd_git_porcelain "$root" "$path")
      if [ -n "$porcelain" ]; then
        return 0 # dirty (incl. untracked ^??)
      fi
      return 1 # clean
      ;;
    *)
      # Indeterminate VCS mode → fail safe to dirty.
      return 0
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wifd_main "$@"
fi
