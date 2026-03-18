#!/usr/bin/env bash

# Shared VCS utility functions sourced by hooks and wrapper scripts.
# This is the single source of truth for repo-root detection logic.

# Find the repository root by walking up the directory tree.
# Outputs the root path and returns 0 if found, returns 1 if not.
find_repo_root() {
  local dir="$PWD"
  while [ "$dir" != "/" ]; do
    if [ -d "$dir/.jj" ] || [ -d "$dir/.git" ]; then
      echo "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}
