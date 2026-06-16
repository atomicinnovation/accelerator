#!/usr/bin/env bash

# Shared configuration utilities sourced by config reader scripts.
# Intentionally omits set -euo pipefail — inherits caller's shell options,
# matching the vcs-common.sh convention.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/vcs-common.sh"
source "$SCRIPT_DIR/config-defaults.sh"
source "$SCRIPT_DIR/atomic-common.sh"

# shellcheck disable=SC2034
AGENT_PREFIX="accelerator:"

# Locate the project root. Reuses find_repo_root() from vcs-common.sh
# with a fallback to $PWD if no VCS root is found.
config_project_root() {
  find_repo_root || echo "$PWD"
}

# Find config files. Outputs paths that exist, one per line.
# Order matters: team config first, local config second. This ordering
# is relied upon by read-value.sh for override precedence (last-writer-wins).
# In migration mode (ACCELERATOR_MIGRATION_MODE=1) also falls back to the
# legacy .claude/accelerator.md location so migrations 0001/0002 can read
# config before migration 0003 has moved it.
config_find_files() {
  local root
  root=$(config_project_root)
  local team="$root/.accelerator/config.md"
  local local_="$root/.accelerator/config.local.md"
  local found=0
  if [ -f "$team" ]; then
    echo "$team"
    found=1
  fi
  if [ -f "$local_" ]; then
    echo "$local_"
    found=1
  fi
  if [ "$found" -eq 0 ] && [ "${ACCELERATOR_MIGRATION_MODE:-}" = "1" ]; then
    # Legacy fallback: config not yet moved by migration 0003
    local legacy_team="$root/.claude/accelerator.md"
    local legacy_local="$root/.claude/accelerator.local.md"
    [ -f "$legacy_team" ] && echo "$legacy_team"
    [ -f "$legacy_local" ] && echo "$legacy_local"
  fi
  return 0
}

# Assert that the project is not using the legacy .claude/accelerator.md layout.
# Exits 1 with a human-readable message if the legacy file exists and the new
# .accelerator/config.md does not. Call from every config reader entry point.
# Skipped when ACCELERATOR_MIGRATION_MODE=1 (migration scripts run on old repos).
config_assert_no_legacy_layout() {
  [ "${ACCELERATOR_MIGRATION_MODE:-}" = "1" ] && return 0
  local root
  root=$(config_project_root)
  local team="$root/.accelerator/config.md"
  local legacy_team="$root/.claude/accelerator.md"
  if [ ! -f "$team" ] && [ -f "$legacy_team" ]; then
    printf '%s\n' \
      "Accelerator: legacy config detected at .claude/accelerator.md." \
      "Run /accelerator:migrate to update the layout, then retry." >&2
    exit 1
  fi
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

# Set (overwrite) a single top-level frontmatter field in a Markdown file.
# Usage: config_set_frontmatter_field <file> <key> <value>
#
# This is the cross-cutting writeback primitive (the Linear/Jira create-issue
# writeback and 0047's sync flow all want the identical operation), so it lives
# in shared scripts/ rather than an integration-specific helper.
#
# Contract:
#   - Operates ONLY inside the frontmatter block (between the first two `---`
#     delimiters), anchored to that range — never the Markdown body.
#   - Replaces exactly the named field's line. Fails closed (non-zero, file
#     untouched) if the field is absent or matched more than once, if the file
#     has no frontmatter, or if the frontmatter is unclosed.
#   - <value> is treated as literal data (no awk/regex metacharacter
#     interpretation), so a value containing &, /, or \ cannot corrupt the
#     substitution. It is passed via the environment, not awk -v.
#   - Before the atomic rename, re-extracts the frontmatter from the candidate
#     output and verifies it still parses and now carries the expected value —
#     the integrity check atomic_write itself does not perform.
# Shared integrity re-check for the frontmatter writeback primitives. Given a
# candidate file body (the full file after a replace or insert), verify the
# frontmatter still parses (is a closed block) AND <key> now reads back as
# <value>. Returns 0 on success, non-zero (with a stderr message) otherwise.
# Both config_set_frontmatter_field (replace) and config_upsert_frontmatter_field
# (insert) run this before the atomic rename, so the two share one definition.
_config_fm_integrity_check() {
  local candidate="$1" key="$2" value="$3"
  local check
  check=$(printf '%s\n' "$candidate" | awk '
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    NR == 1 { exit 1 }
    in_fm && /^---[[:space:]]*$/ { closed = 1; exit }
    in_fm { print }
    END { if (!closed) exit 1 }
  ') || {
    echo "config: candidate frontmatter no longer parses" >&2
    return 1
  }

  local got
  got=$(CSF_KEY="$key" awk '
    BEGIN { key = ENVIRON["CSF_KEY"]; kpat = "^" key ":" }
    $0 ~ kpat {
      v = substr($0, length(key) + 2)
      sub(/^[ \t]+/, "", v)
      sub(/[ \t]+$/, "", v)
      print v
      exit
    }
  ' <<<"$check")
  if [ "$got" != "$value" ]; then
    echo "config: integrity check failed — value not set as expected" >&2
    return 1
  fi
  return 0
}

config_set_frontmatter_field() {
  local file="$1" key="$2" value="$3"
  if [ ! -f "$file" ]; then
    echo "config_set_frontmatter_field: file not found: $file" >&2
    return 1
  fi

  local candidate awk_rc=0
  candidate=$(CSF_KEY="$key" CSF_VALUE="$value" awk '
    BEGIN {
      key = ENVIRON["CSF_KEY"]
      value = ENVIRON["CSF_VALUE"]
      kpat = "^" key ":"
      count = 0
    }
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; print; next }
    NR == 1 { bad_start = 1; print; next }
    in_fm && /^---[[:space:]]*$/ { in_fm = 0; closed = 1; print; next }
    in_fm && $0 ~ kpat {
      count++
      print key ": " value
      next
    }
    { print }
    END {
      if (bad_start) exit 3
      if (!closed) exit 4
      if (count == 0) exit 5
      if (count > 1) exit 6
    }
  ' "$file") || awk_rc=$?

  case "$awk_rc" in
    0) ;;
    3)
      echo "config_set_frontmatter_field: file has no frontmatter (must start with ---): $file" >&2
      return 1
      ;;
    4)
      echo "config_set_frontmatter_field: unclosed frontmatter: $file" >&2
      return 1
      ;;
    5)
      echo "config_set_frontmatter_field: field '$key' not found in frontmatter: $file" >&2
      return 1
      ;;
    6)
      echo "config_set_frontmatter_field: field '$key' matched more than once: $file" >&2
      return 1
      ;;
    *)
      echo "config_set_frontmatter_field: internal error ($awk_rc)" >&2
      return 1
      ;;
  esac

  # Integrity check: candidate frontmatter must still parse (be a closed block)
  # and the field must now read back as <value>. Shared with the upsert helper.
  if ! _config_fm_integrity_check "$candidate" "$key" "$value"; then
    return 1
  fi

  printf '%s\n' "$candidate" | atomic_write "$file"
}

# Upsert a single top-level frontmatter field: replace it if present, else
# insert a new "key: value" line immediately before the closing `---`.
# Usage: config_upsert_frontmatter_field <file> <key> <value>
#
# This is the writeback primitive the create-issue flows use when the target
# field (external_id) may not yet exist on the work-item file — the replace-only
# config_set_frontmatter_field cannot insert.
#
# Contract (fails closed — file left byte-unchanged on any non-insert condition):
#   - If the field is present, replace it (delegates to
#     config_set_frontmatter_field; same integrity + injection guarantees).
#   - Insert ONLY when the field is genuinely absent from a well-formed,
#     closed frontmatter block. No frontmatter, unclosed frontmatter, or a
#     duplicate/present key all propagate as failure without writing.
#   - <value> is passed via the environment (no awk/regex interpretation) and
#     written UNQUOTED, matching the identifier writeback shape so the shared
#     read-back equality check holds.
#   - Runs the same _config_fm_integrity_check as the replace path before the
#     atomic rename.
config_upsert_frontmatter_field() {
  local file="$1" key="$2" value="$3"
  if [ ! -f "$file" ]; then
    echo "config_upsert_frontmatter_field: file not found: $file" >&2
    return 1
  fi

  # Replace path: succeeds iff the field is present exactly once.
  if config_set_frontmatter_field "$file" "$key" "$value"; then
    return 0
  fi

  # config_set_frontmatter_field collapses no-frontmatter / unclosed /
  # field-absent / duplicate to a single non-zero return. Re-detect the ONLY
  # insertable condition — genuinely absent in a parseable block — and fail
  # closed on everything else (unclosed frontmatter propagates here; a present
  # or duplicate key is caught by the grep below; no-frontmatter is caught by
  # the insert awk's bad_start exit).
  local fm
  if ! fm=$(config_extract_frontmatter "$file" 2>/dev/null); then
    return 1 # unclosed frontmatter → not well-formed → do NOT insert
  fi
  # Same `^key:` anchoring config_set_frontmatter_field uses, so a present or
  # duplicate key reads as present and propagates (fail closed).
  if printf '%s\n' "$fm" | grep -q "^${key}:"; then
    return 1
  fi

  # Genuinely absent in a well-formed block → insert "key: value" before the
  # closing `---`. The awk also revalidates structure: a file with no
  # frontmatter (bad_start) or an unclosed block exits non-zero and we bail.
  local candidate awk_rc=0
  candidate=$(CSF_KEY="$key" CSF_VALUE="$value" awk '
    BEGIN {
      key = ENVIRON["CSF_KEY"]
      value = ENVIRON["CSF_VALUE"]
      inserted = 0
    }
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; print; next }
    NR == 1 { bad_start = 1; print; next }
    in_fm && /^---[[:space:]]*$/ {
      print key ": " value
      inserted = 1
      in_fm = 0
      closed = 1
      print
      next
    }
    { print }
    END {
      if (bad_start) exit 3
      if (!closed) exit 4
      if (!inserted) exit 5
    }
  ' "$file") || awk_rc=$?

  if [ "$awk_rc" -ne 0 ]; then
    echo "config_upsert_frontmatter_field: cannot insert '$key' — frontmatter is missing or unclosed: $file" >&2
    return 1
  fi

  if ! _config_fm_integrity_check "$candidate" "$key" "$value"; then
    return 1
  fi

  printf '%s\n' "$candidate" | atomic_write "$file"
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

# Enumerate available template keys from the plugin's templates directory.
# Outputs one template key per line (basename without .md extension).
# Arguments:
#   $1 - plugin root directory path
config_enumerate_templates() {
  local plugin_root="$1"
  local templates_dir="$plugin_root/templates"
  if [ ! -d "$templates_dir" ]; then
    return 0
  fi
  for f in "$templates_dir"/*.md; do
    [ -f "$f" ] || continue
    basename "$f" .md
  done
}

# Format the list of available template keys as a comma-separated string.
# Returns "(none found)" if no templates exist.
# Arguments:
#   $1 - plugin root directory path
config_format_available_templates() {
  local plugin_root="$1"
  local available
  available=$(config_enumerate_templates "$plugin_root" | tr '\n' ', ' |
    sed 's/,$//' | sed 's/,/, /g')
  if [ -z "$available" ]; then
    echo "(none found)"
  else
    echo "$available"
  fi
}

# Source labels used by config_resolve_template. Defined as constants so
# both the resolver and consumers reference the same values.
CONFIG_TEMPLATE_SOURCE_CONFIG_PATH="config path"
CONFIG_TEMPLATE_SOURCE_USER_OVERRIDE="user override"
CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT="plugin default"

# Resolve a template key through the three-tier resolution order.
# Outputs a single tab-delimited line: <source>\t<path>
# If the template is not found, outputs nothing and returns 1.
#
# Resolution order:
#   1. Config-specified path (templates.<key>)
#   2. Templates directory (<paths.templates>/<key>.md)
#   3. Plugin default (<plugin_root>/templates/<key>.md)
#
# Requires: $SCRIPT_DIR must be set to the scripts/ directory by the
#   sourcing script before calling this function (used to locate
#   config-read-value.sh and config-read-path.sh).
#
# Arguments:
#   $1 - template key name
#   $2 - plugin root directory path
config_resolve_template() {
  local key="$1"
  local plugin_root="$2"
  local project_root
  project_root=$(config_project_root)

  # Tier 1: Config-specified path
  local config_path
  config_path=$("$SCRIPT_DIR/config-read-value.sh" "templates.${key}" "")
  if [ -n "$config_path" ]; then
    if [[ "$config_path" != /* ]]; then
      config_path="$project_root/$config_path"
    fi
    if [ -f "$config_path" ]; then
      printf '%s\t%s\n' "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" "$config_path"
      return 0
    else
      echo "Warning: configured template path '$config_path' not found, falling back to defaults" >&2
    fi
  fi

  # Tier 2: Templates directory
  local templates_dir
  templates_dir=$("$SCRIPT_DIR/config-read-path.sh" templates)
  if [[ "$templates_dir" != /* ]]; then
    templates_dir="$project_root/$templates_dir"
  fi
  if [ -f "$templates_dir/${key}.md" ]; then
    printf '%s\t%s\n' "$CONFIG_TEMPLATE_SOURCE_USER_OVERRIDE" "$templates_dir/${key}.md"
    return 0
  fi

  # Tier 3: Plugin default
  local default_path="$plugin_root/templates/${key}.md"
  if [ -f "$default_path" ]; then
    printf '%s\t%s\n' "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" "$default_path"
    return 0
  fi

  return 1
}

# Shorten an absolute path for display purposes.
#   - Paths under project root are shown relative to it
#   - Paths under plugin root are shown as <plugin>/...
#   - Other paths are shown as-is
#
# Arguments:
#   $1 - absolute path to shorten
#   $2 - plugin root directory path
config_display_path() {
  local path="$1"
  local plugin_root="$2"
  local project_root
  project_root=$(config_project_root)

  if [[ "$path" == "$project_root"/* ]]; then
    echo "${path#"$project_root"/}"
  elif [[ "$path" == "$plugin_root"/* ]]; then
    echo "<plugin>/${path#"$plugin_root"/}"
  else
    echo "$path"
  fi
}
