#!/usr/bin/env bash
set -euo pipefail

# Outputs a brief summary of active Accelerator configuration.
# Used by the SessionStart hook to inject config awareness into the session.
# Outputs nothing if no config files exist.
# Emits warnings to stderr for malformed config files.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

FILES=()
while IFS= read -r f; do
  FILES+=("$f")
done < <(config_find_files)

[ ${#FILES[@]} -eq 0 ] && exit 0

ROOT=$(config_project_root)
SUMMARY="Accelerator plugin configuration detected:"

for f in "${FILES[@]}"; do
  REL_PATH="${f#"$ROOT"/}"
  if [[ "$f" == *".local.md" ]]; then
    SUMMARY="$SUMMARY
- Personal config: $REL_PATH"
  else
    SUMMARY="$SUMMARY
- Team config: $REL_PATH"
  fi
done

# List configured sections (non-empty top-level YAML keys).
# Pattern matches valid YAML keys: letters, digits, hyphens, underscores.
# Uses a space-delimited string for dedup instead of associative arrays
# to remain compatible with bash 3.2 (macOS default).
SECTIONS=""
for f in "${FILES[@]}"; do
  fm=$(config_extract_frontmatter "$f") || {
    if head -1 "$f" | grep -q '^---'; then
      echo "Warning: $f has unclosed YAML frontmatter — ignoring" >&2
    fi
    continue
  }
  if [ -n "$fm" ]; then
    keys=$(echo "$fm" | grep -E '^[a-zA-Z_][a-zA-Z0-9_-]*:' | sed 's/:.*//' | sort -u)
    for k in $keys; do
      case " $SECTIONS " in
        *" $k "*) ;;  # already seen
        *) SECTIONS="$SECTIONS $k" ;;
      esac
    done
  fi
done

if [ -n "$SECTIONS" ]; then
  SUMMARY="$SUMMARY
- Configured sections:$SECTIONS"
fi

# Check for context (markdown body)
HAS_CONTEXT=false
for f in "${FILES[@]}"; do
  body=$(config_extract_body "$f")
  trimmed=$(printf '%s\n' "$body" | config_trim_body)
  if [ -n "$trimmed" ]; then
    HAS_CONTEXT=true
    break
  fi
done

if [ "$HAS_CONTEXT" = true ]; then
  SUMMARY="$SUMMARY
- Project context: provided (will be injected into skills)"
fi

SUMMARY="$SUMMARY

Skills will read this configuration at invocation time. To view or edit configuration, use /accelerator:configure."

echo "$SUMMARY"
