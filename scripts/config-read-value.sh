#!/usr/bin/env bash
set -euo pipefail

# Reads a single configuration value from accelerator config files.
# Usage: config-read-value.sh <key> [default]
#
# Supports dot notation for 2-level nesting:
#   config-read-value.sh agents.reviewer reviewer
#   config-read-value.sh review.max_inline_comments 10
#
# For top-level keys:
#   config-read-value.sh enabled true
#
# Precedence: .claude/accelerator.local.md overrides .claude/accelerator.md
# If the key is not found in either file, outputs the default value.
# If no default is provided and key not found, outputs nothing.
#
# Emits warnings to stderr when config files exist but have parse issues.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

KEY="${1:-}"
DEFAULT="${2:-}"

if [ -z "$KEY" ]; then
  echo "Usage: config-read-value.sh <key> [default]" >&2
  exit 1
fi

# Split key into section and subkey
if [[ "$KEY" == *.* ]]; then
  SECTION="${KEY%%.*}"
  SUBKEY="${KEY#*.}"
else
  SECTION=""
  SUBKEY="$KEY"
fi

# Read a value from a single file's frontmatter.
# Uses string comparison (substr/index) instead of regex to avoid
# metacharacter injection when keys contain dots, brackets, etc.
_read_from_file() {
  local file="$1"
  local fm
  fm=$(config_extract_frontmatter "$file") || {
    # Frontmatter exists but is malformed (unclosed)
    if head -1 "$file" | grep -q '^---'; then
      echo "Warning: $file has unclosed YAML frontmatter — ignoring" >&2
    fi
    return 1
  }
  [ -z "$fm" ] && return 1

  if [ -n "$SECTION" ]; then
    # 2-level key: find section, then find subkey within indented block.
    # Section exit: only on non-empty lines that start with a non-space
    # character (blank lines within a section are allowed in YAML).
    echo "$fm" | awk -v section="$SECTION" -v subkey="$SUBKEY" '
      {
        # Section start: line is exactly "section:" with optional trailing content
        prefix = section ":"
        if (substr($0, 1, length(prefix)) == prefix && \
            (length($0) == length(prefix) || \
             substr($0, length(prefix)+1, 1) ~ /[ \t]/)) {
          in_section = 1
          next
        }
      }
      # Exit section on non-empty, non-indented lines (new top-level key)
      in_section && /^[^ \t]/ && /[^ \t]/ { in_section = 0 }
      in_section {
        stripped = $0
        sub(/^[ \t]+/, "", stripped)
        kprefix = subkey ":"
        if (substr(stripped, 1, length(kprefix)) == kprefix) {
          val = substr(stripped, length(kprefix) + 1)
          sub(/^[ \t]*/, "", val)
          sub(/[ \t]+$/, "", val)
          # Strip optional surrounding quotes
          if (val ~ /^".*"$/ || val ~ /^'"'"'.*'"'"'$/) {
            val = substr(val, 2, length(val) - 2)
          }
          print val
          found = 1
          exit
        }
      }
      END { exit (found ? 0 : 1) }
    '
  else
    # Top-level key: match non-indented lines using string comparison
    echo "$fm" | awk -v key="$SUBKEY" '
      /^[^ \t]/ {
        prefix = key ":"
        if (substr($0, 1, length(prefix)) == prefix) {
          val = substr($0, length(prefix) + 1)
          sub(/^[ \t]*/, "", val)
          sub(/[ \t]+$/, "", val)
          if (val ~ /^".*"$/ || val ~ /^'"'"'.*'"'"'$/) {
            val = substr(val, 2, length(val) - 2)
          }
          print val
          found = 1
          exit
        }
      }
      END { exit (found ? 0 : 1) }
    '
  fi
}

# Process files in precedence order: team first, local second.
# Don't break on first match — later files (local) override earlier (team).
# This ordering is guaranteed by config_find_files().
RESULT=""
FOUND=false
while IFS= read -r config_file; do
  if val=$(_read_from_file "$config_file"); then
    RESULT="$val"
    FOUND=true
  fi
done < <(config_find_files)

if [ "$FOUND" = true ]; then
  echo "$RESULT"
else
  echo "$DEFAULT"
fi
