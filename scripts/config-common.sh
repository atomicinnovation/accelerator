#!/usr/bin/env bash

# Shared configuration utilities sourced by config reader scripts.
# Intentionally omits set -euo pipefail — inherits caller's shell options,
# matching the vcs-common.sh convention.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/vcs-common.sh"

# Locate the project root. Reuses find_repo_root() from vcs-common.sh
# with a fallback to $PWD if no VCS root is found.
config_project_root() {
  find_repo_root || echo "$PWD"
}

# Find config files. Outputs paths that exist, one per line.
# Order matters: team config first, local config second. This ordering
# is relied upon by read-value.sh for override precedence (last-writer-wins).
config_find_files() {
  local root
  root=$(config_project_root)
  local team="$root/.claude/accelerator.md"
  local local_="$root/.claude/accelerator.local.md"
  [ -f "$team" ] && echo "$team"
  [ -f "$local_" ] && echo "$local_"
}

# Extract YAML frontmatter from a file as raw text (between --- delimiters).
# Outputs the frontmatter lines (excluding the --- delimiters themselves).
# Returns nothing if:
#   - The file has no frontmatter (no --- on line 1)
#   - The frontmatter is unclosed (opening --- but no closing ---)
config_extract_frontmatter() {
  local file="$1"
  awk '
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    NR == 1 && !/^---[[:space:]]*$/ { exit }
    in_fm && /^---[[:space:]]*$/ { closed = 1; exit }
    in_fm { lines[++n] = $0 }
    END {
      if (!closed) exit 1
      for (i = 1; i <= n; i++) print lines[i]
    }
  ' "$file"
}

# Extract markdown body from a file (everything after the closing ---).
# If no frontmatter exists (no --- on line 1), outputs the entire file.
# If frontmatter is unclosed, outputs nothing (treats file as malformed).
config_extract_body() {
  local file="$1"
  awk '
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    NR == 1 && !/^---[[:space:]]*$/ { no_fm = 1; print; next }
    no_fm { print; next }
    in_fm && /^---[[:space:]]*$/ { in_fm = 0; past_fm = 1; next }
    in_fm { next }
    past_fm { print }
  ' "$file"
}

# Trim leading and trailing blank lines from stdin.
# Centralised to avoid duplicating fragile sed idioms.
# Parse a YAML-style inline array string into one element per line.
# Input: "[a, b, c]" (as returned by config-read-value.sh)
# Output: one element per line, whitespace-trimmed
# Empty input or "[]" produces no output.
config_parse_array() {
  local raw="$1"
  # Strip brackets
  raw="${raw#\[}"
  raw="${raw%\]}"
  # Empty after stripping → nothing to output
  [ -z "$raw" ] && return 0
  # Split on commas and trim whitespace
  echo "$raw" | tr ',' '\n' | while IFS= read -r item; do
    # Trim leading/trailing whitespace
    item=$(echo "$item" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
    [ -n "$item" ] && echo "$item"
  done
}

config_trim_body() {
  awk '
    NF { found = 1 }
    found { lines[++n] = $0 }
    END {
      # Trim trailing blank lines
      while (n > 0 && lines[n] ~ /^[[:space:]]*$/) n--
      for (i = 1; i <= n; i++) print lines[i]
    }
  '
}
