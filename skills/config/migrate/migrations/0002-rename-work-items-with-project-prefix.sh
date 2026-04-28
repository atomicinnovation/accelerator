#!/usr/bin/env bash
# DESCRIPTION: Rename legacy NNNN-*.md work items to the configured project-prefix pattern
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"
source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/skills/work/scripts/work-item-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

WORK_DIR="$PROJECT_ROOT/meta/work"

# ── Step 1: validate_preconditions ──────────────────────────────────────────

PATTERN=$(cd "$PROJECT_ROOT" && bash "$PLUGIN_ROOT/scripts/config-read-value.sh" \
  work.id_pattern "{number:04d}")
DEFAULT_PROJECT=$(cd "$PROJECT_ROOT" && bash "$PLUGIN_ROOT/scripts/config-read-value.sh" \
  work.default_project_code "")

if [[ "$PATTERN" != *"{project}"* ]]; then
  echo "MIGRATION_RESULT: no_op_pending"
  exit 0
fi

if [ -z "$DEFAULT_PROJECT" ]; then
  echo "error: migration 0002 requires a value for work.default_project_code" \
       "(your pattern '$PATTERN' contains {project}). Set work.default_project_code" \
       "in your config to apply, or run 'bash run-migrations.sh --skip" \
       "0002-rename-work-items-with-project-prefix' to opt out. See" \
       "skills/config/configure/SKILL.md > Work Items for details on choosing." >&2
  exit 1
fi

# ── Step 2: build_rename_map ────────────────────────────────────────────────

FORMAT=$(wip_compile_format "$PATTERN" "$DEFAULT_PROJECT")

declare -a OLD_PATHS=()
declare -a NEW_PATHS=()
declare -a OLD_IDS=()
declare -a NEW_IDS=()

if [ -d "$WORK_DIR" ]; then
  while IFS= read -r -d '' f; do
    base="$(basename "$f" .md)"
    if [[ "$base" =~ ^([0-9]{4})-(.+)$ ]]; then
      old_num="${BASH_REMATCH[1]}"
      slug="${BASH_REMATCH[2]}"
      old_id="$old_num"
      # shellcheck disable=SC2059
      new_id=$(printf "$FORMAT" "$((10#$old_num))")
      new_path="$WORK_DIR/${new_id}-${slug}.md"
      OLD_PATHS+=("$f")
      NEW_PATHS+=("$new_path")
      OLD_IDS+=("$old_id")
      NEW_IDS+=("$new_id")
    fi
  done < <(find "$WORK_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.md' -print0 \
    2>/dev/null | sort -z)
fi

if [ ${#OLD_PATHS[@]} -eq 0 ]; then
  exit 0
fi

# ── Step 3: check_collisions ────────────────────────────────────────────────

collisions=()
for i in "${!NEW_PATHS[@]}"; do
  if [ -f "${NEW_PATHS[$i]}" ] && [ "${NEW_PATHS[$i]}" != "${OLD_PATHS[$i]}" ]; then
    collisions+=("${OLD_PATHS[$i]} → ${NEW_PATHS[$i]}")
  fi
done

if [ ${#collisions[@]} -gt 0 ]; then
  echo "error: rename collision detected — target files already exist:" >&2
  for c in "${collisions[@]}"; do
    echo "  $c" >&2
  done
  exit 1
fi

# ── Step 4: rename_with_frontmatter ─────────────────────────────────────────

for i in "${!OLD_PATHS[@]}"; do
  old_path="${OLD_PATHS[$i]}"
  new_path="${NEW_PATHS[$i]}"
  new_id="${NEW_IDS[$i]}"

  awk -v new_id="$new_id" '
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; print; next }
    in_fm && /^---[[:space:]]*$/ { in_fm = 0; print; next }
    in_fm && /^work_item_id:/ {
      print "work_item_id: \"" new_id "\""
      next
    }
    { print }
  ' "$old_path" | atomic_write "$old_path"

  if [ "$old_path" != "$new_path" ]; then
    mv "$old_path" "$new_path"
  fi
done

# ── Step 5: rewrite_frontmatter_refs ────────────────────────────────────────

FIELDS_PATTERN="^[[:space:]]*(work_item_id|parent|related|blocks|blocked_by|supersedes|superseded_by):"

rewrite_frontmatter_in_file() {
  local file="$1"
  local content
  content=$(cat "$file")
  local original="$content"

  for i in "${!OLD_IDS[@]}"; do
    local old_id="${OLD_IDS[$i]}"
    local new_id="${NEW_IDS[$i]}"

    local new_content=""
    local in_fm=0
    local in_list=0

    while IFS= read -r line; do
      if [ "$in_fm" -eq 0 ] && [[ "$line" =~ ^---[[:space:]]*$ ]] && [ -z "$new_content" ]; then
        in_fm=1
        new_content+="$line"$'\n'
        continue
      fi
      if [ "$in_fm" -eq 1 ] && [[ "$line" =~ ^---[[:space:]]*$ ]]; then
        in_fm=0
        in_list=0
        new_content+="$line"$'\n'
        continue
      fi
      if [ "$in_fm" -eq 0 ]; then
        new_content+="$line"$'\n'
        continue
      fi

      # Inside frontmatter
      if [[ "$line" =~ $FIELDS_PATTERN ]]; then
        in_list=0
        if [[ "$line" == *"["* ]]; then
          # Inline list — rewrite element by element
          local rewritten
          rewritten=$(printf '%s' "$line" | sed -E \
            -e "s/\"${old_id}\"/\"${new_id}\"/g" \
            -e "s/'${old_id}'/\"${new_id}\"/g" \
            -e "s/\[([[:space:]]*)${old_id}([[:space:]]*[],])/[\1\"${new_id}\"\2/g" \
            -e "s/,([[:space:]]*)${old_id}([[:space:]]*[],])/,\1\"${new_id}\"\2/g")
          new_content+="$rewritten"$'\n'
          # Check if list continues on next lines
          if [[ "$line" != *"]"* ]]; then
            in_list=1
          fi
        elif [[ "$line" =~ ^([[:space:]]*[a-z_]+:[[:space:]]*) ]]; then
          # Scalar field
          local prefix="${BASH_REMATCH[1]}"
          local val="${line#"$prefix"}"
          # Strip trailing whitespace
          val="${val%"${val##*[![:space:]]}"}"
          if [ "$val" = "\"${old_id}\"" ] || [ "$val" = "'${old_id}'" ] || [ "$val" = "${old_id}" ]; then
            new_content+="${prefix}\"${new_id}\""$'\n'
          else
            new_content+="$line"$'\n'
          fi
        else
          # Field with list on following lines
          in_list=1
          new_content+="$line"$'\n'
        fi
      elif [ "$in_list" -eq 1 ] && [[ "$line" =~ ^[[:space:]]*-[[:space:]] ]]; then
        # List item continuation
        if [[ "$line" =~ ^([[:space:]]*-[[:space:]]*)(.*)$ ]]; then
          local prefix="${BASH_REMATCH[1]}"
          local val="${BASH_REMATCH[2]}"
          val="${val%"${val##*[![:space:]]}"}"
          if [ "$val" = "\"${old_id}\"" ] || [ "$val" = "'${old_id}'" ] || [ "$val" = "${old_id}" ]; then
            new_content+="${prefix}\"${new_id}\""$'\n'
          else
            new_content+="$line"$'\n'
          fi
        else
          new_content+="$line"$'\n'
        fi
      else
        in_list=0
        new_content+="$line"$'\n'
      fi
    done <<< "$content"

    # Remove trailing newline added by heredoc
    content="${new_content%$'\n'}"
  done

  if [ "$content" != "$original" ]; then
    printf '%s\n' "$content" | atomic_write "$file"
  fi
}

# ── Step 6: rewrite_markdown_links ──────────────────────────────────────────

rewrite_markdown_links_in_file() {
  local file="$1"
  local content
  content=$(cat "$file")
  local original="$content"

  for i in "${!OLD_IDS[@]}"; do
    local old_id="${OLD_IDS[$i]}"
    local new_id="${NEW_IDS[$i]}"

    content=$(printf '%s\n' "$content" | sed -E \
      "s|(\[[^]]*\]\([^)]*/)${old_id}-([^)]+\.md)(#[^)]*)?\)|\1${new_id}-\2\3)|g")
  done

  if [ "$content" != "$original" ]; then
    printf '%s\n' "$content" | atomic_write "$file"
  fi
}

# ── Step 7: rewrite_prose_refs ──────────────────────────────────────────────

rewrite_prose_in_file() {
  local file="$1"
  local content
  content=$(cat "$file")
  local original="$content"

  # Heading-line #NNNN references
  for i in "${!OLD_IDS[@]}"; do
    local old_id="${OLD_IDS[$i]}"
    local new_id="${NEW_IDS[$i]}"

    local new_content=""
    while IFS= read -r line; do
      if [[ "$line" =~ ^#+[[:space:]] ]]; then
        # Heading line — rewrite bounded #NNNN refs
        local out=""
        local rest="$line"
        while [[ "$rest" == *"#${old_id}"* ]]; do
          local before="${rest%%"#${old_id}"*}"
          local after="${rest#*"#${old_id}"}"
          # Check prefix boundary
          local pre_ok=1
          if [ -n "$before" ]; then
            local last_char="${before: -1}"
            if [[ "$last_char" =~ [A-Za-z0-9_] ]]; then
              pre_ok=0
            fi
          fi
          # Check suffix boundary
          local post_ok=1
          if [ -n "$after" ]; then
            local first_char="${after:0:1}"
            if [[ "$first_char" =~ [A-Za-z0-9_-] ]]; then
              post_ok=0
            fi
          fi
          if [ "$pre_ok" -eq 1 ] && [ "$post_ok" -eq 1 ]; then
            out+="${before}#${new_id}"
            rest="$after"
          else
            out+="${before}#${old_id}"
            rest="$after"
          fi
        done
        out+="$rest"
        new_content+="$out"$'\n'
      else
        new_content+="$line"$'\n'
      fi
    done <<< "$content"

    content="${new_content%$'\n'}"
  done

  # Fenced-code-block path references (tagged blocks only)
  for i in "${!OLD_IDS[@]}"; do
    local old_id="${OLD_IDS[$i]}"
    local new_id="${NEW_IDS[$i]}"

    local new_content=""
    local in_tagged=0
    while IFS= read -r line; do
      if [[ "$line" =~ ^\`\`\`(bash|sh|yaml|json|text) ]]; then
        in_tagged=1
        new_content+="$line"$'\n'
      elif [[ "$line" =~ ^\`\`\` ]]; then
        if [ "$in_tagged" -eq 1 ]; then
          in_tagged=0
        fi
        new_content+="$line"$'\n'
      elif [ "$in_tagged" -eq 1 ]; then
        # Only rewrite path-shaped references
        local rewritten="${line//meta\/work\/${old_id}-/meta\/work\/${new_id}-}"
        new_content+="$rewritten"$'\n'
      else
        new_content+="$line"$'\n'
      fi
    done <<< "$content"

    content="${new_content%$'\n'}"
  done

  if [ "$content" != "$original" ]; then
    printf '%s\n' "$content" | atomic_write "$file"
  fi
}

# ── Apply cross-reference rewrites to all meta/**/*.md files ────────────────

while IFS= read -r -d '' file; do
  rewrite_frontmatter_in_file "$file"
done < <(find "$PROJECT_ROOT/meta" -name '*.md' -print0 2>/dev/null)

while IFS= read -r -d '' file; do
  rewrite_markdown_links_in_file "$file"
done < <(find "$PROJECT_ROOT/meta" -name '*.md' -print0 2>/dev/null)

while IFS= read -r -d '' file; do
  rewrite_prose_in_file "$file"
done < <(find "$PROJECT_ROOT/meta" -name '*.md' -print0 2>/dev/null)
