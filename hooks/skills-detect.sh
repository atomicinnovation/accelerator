#!/usr/bin/env bash

if ! command -v jq &>/dev/null; then
  echo '{"systemMessage":"WARNING: jq not installed. Accelerator skills-detect hook could not run."}'
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"

# shellcheck source=../scripts/config-common.sh
source "$PLUGIN_ROOT/scripts/config-common.sh"

# Process bang lines in a skill body: replace !`cmd` lines with command output.
# Only executes commands whose canonicalized path starts with $PLUGIN_ROOT/scripts/
# (allowlist). Commands outside this prefix are skipped silently.
_process_bang_lines() {
  local skill_file="$1"
  local safe_prefix
  safe_prefix="$(cd "$PLUGIN_ROOT/scripts" && pwd)"

  config_extract_body "$skill_file" | while IFS= read -r line; do
    if [[ "$line" =~ ^'!`'(.+)'`'$ ]]; then
      local cmd="${BASH_REMATCH[1]}"
      local resolved="${cmd/\$\{CLAUDE_PLUGIN_ROOT\}/$PLUGIN_ROOT}"
      resolved="${resolved/\$\{PLUGIN_ROOT\}/$PLUGIN_ROOT}"
      local cmd_path="${resolved%% *}"
      local rest="${resolved#"$cmd_path"}"
      # Require the script to exist; use cd+pwd to canonicalize (avoids
      # realpath --canonicalize-missing dependency which is GNU-only).
      local canonical_path
      if [ -f "$cmd_path" ]; then
        canonical_path="$(cd "$(dirname "$cmd_path")" && pwd)/$(basename "$cmd_path")"
      else
        continue
      fi
      # Enforce allowlist: only execute scripts under $PLUGIN_ROOT/scripts/.
      if [[ "$canonical_path" == "$safe_prefix/"* ]]; then
        local output
        output=$("$canonical_path"$rest 2>/dev/null) && printf '%s\n' "$output" || true
      fi
    else
      printf '%s\n' "$line"
    fi
  done
}

# Find a skill file by its `name:` frontmatter field.
_find_skill_by_name() {
  local skill_name="$1"
  [[ "$skill_name" =~ ^[a-zA-Z0-9_-]+$ ]] || return 0
  find "$PLUGIN_ROOT/skills" -name "SKILL.md" \
    -not -path "*/node_modules/*" 2>/dev/null | while IFS= read -r f; do
    if head -10 "$f" 2>/dev/null | grep -q "^name: ${skill_name}$"; then
      echo "$f"
    fi
  done | head -1 || true
}

COMBINED=""
for agent_file in "$PLUGIN_ROOT/agents/"*.md; do
  [ -f "$agent_file" ] || continue

  skills_raw=$(config_extract_frontmatter "$agent_file" 2>/dev/null \
    | awk '/^skills:/{$1=""; print; exit}' | sed 's/^[[:space:]]*//' || true)
  [ -z "$skills_raw" ] && continue

  while IFS= read -r skill_name; do
    [ -z "$skill_name" ] && continue

    skill_file=$(_find_skill_by_name "$skill_name")
    [ -z "$skill_file" ] && continue

    if config_extract_frontmatter "$skill_file" 2>/dev/null \
       | grep -q "^disable-model-invocation: true$"; then
      continue
    fi

    processed=$(_process_bang_lines "$skill_file")
    [ -z "$processed" ] && continue

    COMBINED="${COMBINED}${processed}"$'\n'
  done < <(config_parse_array "$skills_raw")
done

[ -z "$COMBINED" ] && exit 0

if [ ${#COMBINED} -gt 65536 ]; then
  COMBINED="[skills-detect: combined skill output exceeded 64 KB and was truncated]"$'\n'
fi

jq -n --arg context "$COMBINED" '{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": $context
  }
}'
