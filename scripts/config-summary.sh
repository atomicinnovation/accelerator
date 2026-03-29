#!/usr/bin/env bash
set -euo pipefail

# Outputs a brief summary of active Accelerator configuration.
# Used by the SessionStart hook to inject config awareness into the session.
# Outputs nothing if no config files exist and the repo is already initialised.
# Emits warnings to stderr for malformed config files.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

FILES=()
while IFS= read -r f; do
  FILES+=("$f")
done < <(config_find_files)

# Check for tmp/.gitignore (not just tmp/) as the initialisation sentinel,
# because review-pr creates tmp/ organically via mkdir -p.
TMP_PATH=$("$SCRIPT_DIR/config-read-path.sh" tmp meta/tmp)
INITIALISED=true
[ ! -f "$TMP_PATH/.gitignore" ] && INITIALISED=false

INIT_HINT="Accelerator has not been initialised in this repository. Type /accelerator:initialise at the prompt to set up the expected directory structure and gitignore entries."

if [ ${#FILES[@]} -eq 0 ]; then
  if [ "$INITIALISED" = false ]; then
    echo "$INIT_HINT"
  fi
  exit 0
fi

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

# Check for per-skill customisations
SKILL_CUSTOM_DIR="$ROOT/.claude/accelerator/skills"
SKILL_CUSTOMISATIONS=""

# Derive known skill names dynamically from plugin skill directories
# (excludes configure, which is not customisable via this mechanism)
PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KNOWN_SKILLS=""
for skill_md in "$PLUGIN_ROOT"/skills/*/SKILL.md "$PLUGIN_ROOT"/skills/*/*/SKILL.md; do
  [ -f "$skill_md" ] || continue
  sname=$(awk '/^name:/{print $2; exit}' "$skill_md")
  [ "$sname" = "configure" ] && continue
  [ -n "$sname" ] && KNOWN_SKILLS="$KNOWN_SKILLS $sname"
done
KNOWN_SKILLS="${KNOWN_SKILLS# }"

if [ -d "$SKILL_CUSTOM_DIR" ]; then
  for skill_dir in "$SKILL_CUSTOM_DIR"/*/; do
    [ -d "$skill_dir" ] || continue
    skill_name=$(basename "$skill_dir")

    # Warn about unrecognised skill names
    case " $KNOWN_SKILLS " in
      *" $skill_name "*) ;;
      *) echo "Warning: .claude/accelerator/skills/$skill_name/ does not match any known skill name. Valid names: $KNOWN_SKILLS" >&2 ;;
    esac

    # Check for non-empty content (matching reader script behaviour)
    has_context=false
    has_instructions=false
    if [ -f "$skill_dir/context.md" ]; then
      trimmed=$(config_trim_body < "$skill_dir/context.md")
      [ -n "$trimmed" ] && has_context=true
    fi
    if [ -f "$skill_dir/instructions.md" ]; then
      trimmed=$(config_trim_body < "$skill_dir/instructions.md")
      [ -n "$trimmed" ] && has_instructions=true
    fi

    if [ "$has_context" = true ] || [ "$has_instructions" = true ]; then
      types=""
      [ "$has_context" = true ] && types="context"
      if [ "$has_instructions" = true ]; then
        [ -n "$types" ] && types="$types + "
        types="${types}instructions"
      fi
      SKILL_CUSTOMISATIONS="$SKILL_CUSTOMISATIONS
    - $skill_name ($types)"
    fi
  done
fi

if [ -n "$SKILL_CUSTOMISATIONS" ]; then
  SUMMARY="$SUMMARY
- Per-skill customisations:$SKILL_CUSTOMISATIONS"
fi

SUMMARY="$SUMMARY

Skills will read this configuration at invocation time. To view or edit configuration, use /accelerator:configure."

if [ "$INITIALISED" = false ]; then
  SUMMARY="$SUMMARY

$INIT_HINT"
fi

echo "$SUMMARY"
