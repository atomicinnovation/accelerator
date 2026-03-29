---
date: "2026-03-29T15:30:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Template Management Subcommands Implementation Plan

## Overview

Add five template management subcommands to the configure skill
(`templates list|show|eject|diff|reset`) so users can inspect, customise,
and manage document templates without manually locating plugin internals.
Uses the hybrid approach: scripts for operations needing reliable resolution
logic (`list`, `show`, `eject`, `diff`) and prompt-only for interactive
operations (`reset`). Also fixes the missing `pr-description` key in
`config-dump.sh`.

## Current State Analysis

The template system is fully functional for **consumption** — skills resolve
templates via `config-read-template.sh`'s three-tier fallback (config path →
templates directory → plugin default). However, there is no tooling for
**management**: users must manually locate plugin templates, copy files to the
right directory, and understand the resolution order. The configure skill's
`help` subcommand documents the override mechanism but provides no commands to
act on it.

### Key Discoveries:

- `config-read-template.sh:36-49` wraps output in code fences for LLM
  consumption — management scripts need raw output
- `config-read-template.sh:85-94` already enumerates available templates from
  `$PLUGIN_ROOT/templates/*.md` for error messages — this logic can be
  extracted
- `config-dump.sh:209-214` is missing `templates.pr-description` from the
  `TEMPLATE_KEYS` array
- The configure skill (`skills/config/configure/SKILL.md`) dispatches via
  prose H3 headings (`view`, `create`, `help`) — template subcommands follow
  the same pattern
- `config-common.sh` provides shared utilities but has no template-specific
  helpers yet
- All config scripts follow a consistent pattern: `set -euo pipefail`,
  source `config-common.sh`, output markdown to stdout, warnings/errors to
  stderr

## Desired End State

After implementation:

1. Users can run `/accelerator:configure templates list` to see all template
   keys, their resolution source (plugin default / user override / config
   path), and resolved file path.
2. Users can run `/accelerator:configure templates show <key>` to view a
   template's content with source metadata.
3. Users can run `/accelerator:configure templates eject <key>` (or
   `eject --all`) to copy plugin defaults to their templates directory for
   customisation.
4. Users can run `/accelerator:configure templates diff <key>` to see
   differences between their customised template and the plugin default.
5. Users can run `/accelerator:configure templates reset <key>` to remove
   their customised template and revert to the plugin default.
6. `config-dump.sh` correctly includes `templates.pr-description`.
7. All new scripts have tests in `test-config.sh`.

### Verification:

- `bash scripts/test-config.sh` passes with all new tests
- Each script can be invoked standalone from a project root
- The configure skill correctly dispatches to template subcommands
- Error cases (unknown key, no override to diff/reset, eject when exists)
  produce helpful messages

## What We're NOT Doing

- Template versioning / staleness detection (user decided not needed)
- A `templates edit` subcommand (eject + manual edit is sufficient)
- A separate `/accelerator:templates` skill (nesting under configure)
- Changes to how skills consume templates (the `!` preprocessor mechanism)
- Changes to the three-tier resolution order itself

## Implementation Approach

Five new scripts follow the existing `config-*` naming convention:
`config-list-template.sh`, `config-show-template.sh`,
`config-eject-template.sh`, `config-diff-template.sh`, and
`config-reset-template.sh`. Three shared helper functions are added to
`config-common.sh`: `config_enumerate_templates()` to list available template
keys, `config_resolve_template()` to perform three-tier resolution returning
the source label and resolved path as a tab-delimited line, and
`config_format_available_templates()` to format the available templates list
for error messages.

The configure skill's SKILL.md is updated with a new `templates` dispatch
section containing sub-sections for each action. For `list`, `show`, `diff`,
`eject`, and `reset`, the skill instructs Claude to run the appropriate
script via Bash and present the output. For `reset`, the script resolves the
override and outputs what it found; the skill layer handles user confirmation
before running with `--confirm`.

---

## Phase 1: Fix & Foundation

### Overview

Fix the missing `pr-description` in `config-dump.sh` and add shared
template helpers to `config-common.sh`: enumeration, resolution, and
available-templates formatting.

### Changes Required:

#### 1. Fix `config-dump.sh` template keys

**File**: `scripts/config-dump.sh`
**Changes**: Add `templates.pr-description` to the `TEMPLATE_KEYS` array.

Replace lines 209-214:

```bash
TEMPLATE_KEYS=(
  "templates.plan"
  "templates.research"
  "templates.adr"
  "templates.validation"
  "templates.pr-description"
)
```

#### 2. Add template helpers to `config-common.sh`

**File**: `scripts/config-common.sh`
**Changes**: Add three helper functions after the `config_trim_body()`
function. These centralise logic currently duplicated across scripts and
will be reused by all management scripts.

**Prerequisite**: `config_resolve_template()` references `$SCRIPT_DIR`
internally (to call `config-read-value.sh` and `config-read-path.sh`).
`config-common.sh` does not set `$SCRIPT_DIR` itself — it relies on the
sourcing script having already set it. Every `config-*.sh` script already
follows this convention (`SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"` before `source config-common.sh`). Callers that source
`config-common.sh` directly (e.g., tests) must also set `$SCRIPT_DIR` to
the `scripts/` directory before sourcing.

Append after the `config_trim_body()` function:

```bash
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
  available=$(config_enumerate_templates "$plugin_root" | tr '\n' ', ' \
    | sed 's/,$//' | sed 's/,/, /g')
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
  templates_dir=$("$SCRIPT_DIR/config-read-path.sh" templates meta/templates)
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
```

#### 3. Refactor `config-read-template.sh` to use shared helpers

**File**: `scripts/config-read-template.sh`
**Changes**: Replace the inline three-tier resolution (lines 53-83) with a
call to `config_resolve_template()`, and replace the inline template
enumeration in the error path (lines 85-99) with
`config_format_available_templates()`. This keeps the existing behaviour
identical but eliminates duplicated logic.

Replace lines 53-99:

```bash
# Resolve template through three-tier fallback
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: $AVAILABLE" >&2
  exit 1
}

IFS=$'\t' read -r _SOURCE RESOLVED_PATH <<< "$RESOLUTION"
_output_template "$RESOLVED_PATH"
```

#### 4. Add Phase 1 tests to `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Add tests for the new shared helpers and the refactored
`config-read-template.sh` immediately, so regressions are caught before
building on this foundation. Add script references and assertion helpers
as described in Phase 8 (direct function test setup, `assert_file_exists`,
`assert_file_not_exists`, `assert_file_content_eq`), then add the
following test sections:

**`config_enumerate_templates` tests** (direct function tests):
- Lists all template keys from plugin templates directory
- Returns nothing if templates directory is empty
- Only returns `.md` files (ignores other extensions)
- Returns nothing if directory exists with only non-`.md` files

**`config_resolve_template` tests** (direct function tests):
- Resolves to plugin default when no config or override exists
- Resolves to user override when present in templates directory
- Resolves to config path when `templates.<key>` is set
- Config path takes precedence over user override
- Returns 1 when template key is unknown
- Emits warning to stderr when config path is missing but falls back

**`config_format_available_templates` tests** (direct function tests):
- Formats template keys as comma-separated list
- Returns "(none found)" when no templates exist

**`config-dump.sh` pr-description test**:
- Output contains `templates.pr-description` row

**`config-read-template.sh` regression tests**:
- All existing `config-read-template.sh` tests still pass (already in
  test file — just verify no regressions)
- `config-read-template.sh nonexistent` error message lists all 5
  template names (including `pr-description`)

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes (all existing tests still pass,
      new Phase 1 tests pass)
- [ ] `config-dump.sh` output includes `templates.pr-description` row
- [ ] `config-read-template.sh nonexistent` error message still lists all 5
      template names
- [ ] `config_resolve_template` returns correct source and path for each tier
- [ ] `config_format_available_templates` returns comma-separated list

#### Manual Verification:

- [ ] `/accelerator:configure view` shows `templates.pr-description` in the
      dump output if config is present

---

## Phase 2: `config-list-template.sh` Script

### Overview

Create a new script that lists all template keys with their resolution
source and resolved file path, displayed as a markdown table.

### Changes Required:

#### 1. Create `scripts/config-list-template.sh`

**File**: `scripts/config-list-template.sh` (new)
**Changes**: New script following the established `config-*` pattern. Uses
the shared `config_resolve_template()` helper for resolution.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Lists all available templates with their resolution source and path.
# Usage: config-list-template.sh
#
# For each template key, shows:
#   - The key name
#   - The resolution source (config path / user override / plugin default)
#   - The resolved file path
#
# Outputs a markdown table to stdout.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "| Template | Source | Path |"
echo "|----------|--------|------|"

for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do
  RESOLUTION=$(config_resolve_template "$KEY" "$PLUGIN_ROOT" 2>/dev/null) || true

  if [ -n "$RESOLUTION" ]; then
    IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"
    DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")
  else
    RESOLVED_SOURCE="not found"
    DISPLAY_PATH="—"
  fi

  echo "| \`$KEY\` | $RESOLVED_SOURCE | \`$DISPLAY_PATH\` |"
done
```

Make executable: `chmod +x scripts/config-list-template.sh`

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/config-list-template.sh` outputs a markdown table with
      5 rows (one per template)
- [ ] All 5 templates resolve to `plugin default` when no config exists
- [ ] When a user override exists in `meta/templates/`, the source shows
      `user override`
- [ ] When a config path is set via `templates.<key>`, the source shows
      `config path`
- [ ] Tests pass in `test-config.sh`

---

## Phase 3: `config-show-template.sh` Script

### Overview

Create a script that outputs a template's raw content (without code fence
wrapping) along with source metadata.

### Changes Required:

#### 1. Create `scripts/config-show-template.sh`

**File**: `scripts/config-show-template.sh` (new)
**Changes**: New script. Uses the shared `config_resolve_template()` helper
for three-tier resolution, and outputs raw content with a source header
instead of code-fenced content.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Shows a template's content with source metadata.
# Usage: config-show-template.sh <template_name>
#
# Outputs the template source information followed by the raw content.
# Unlike config-read-template.sh, does NOT wrap in code fences.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-show-template.sh <template_name>" >&2
  exit 1
fi

RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: $AVAILABLE" >&2
  exit 1
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"
DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

echo "Source: $RESOLVED_SOURCE ($DISPLAY_PATH)"
echo "---"
cat "$RESOLVED_PATH"
```

Make executable: `chmod +x scripts/config-show-template.sh`

### Success Criteria:

#### Automated Verification:

- [ ] `config-show-template.sh plan` outputs source metadata line + raw
      content (no code fences)
- [ ] Output starts with `Source: plugin default (...)` when no override
      exists
- [ ] User override is correctly identified as `user override`
- [ ] Config path override is correctly identified as `config path`
- [ ] Unknown template name produces error listing available templates
- [ ] Missing argument produces usage message to stderr and exit 1
- [ ] Tests pass in `test-config.sh`

---

## Phase 4: `config-eject-template.sh` Script

### Overview

Create a script that copies a plugin default template to the user's
templates directory. Supports `--all` to eject all templates and `--force`
to overwrite existing files.

### Changes Required:

#### 1. Create `scripts/config-eject-template.sh`

**File**: `scripts/config-eject-template.sh` (new)
**Changes**: New script. Copies the plugin default template to the user's
templates directory. Uses shared helpers for error messages. Exit codes
communicate status:
- 0: successfully ejected (or `--dry-run` with no conflicts)
- 1: error (unknown key, missing plugin default, usage error)
- 2: target already exists (without `--force`)

```bash
#!/usr/bin/env bash
set -euo pipefail

# Ejects (copies) a plugin default template to the user's templates
# directory for customisation.
#
# Usage: config-eject-template.sh [--force] [--dry-run] <template_name|--all>
#
# Options:
#   --force    Overwrite existing template files
#   --dry-run  Show what would happen without writing files
#   --all      Eject all templates
#
# Exit codes:
#   0 - Successfully ejected (or dry-run with no conflicts)
#   1 - Error (unknown template, missing default, usage error)
#   2 - Target already exists (use --force to overwrite)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

FORCE=false
DRY_RUN=false
TEMPLATE_NAME=""

# Parse arguments
while [ $# -gt 0 ]; do
  case "$1" in
    --force)
      FORCE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --all)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: cannot combine --all with a template name" >&2
        exit 1
      fi
      TEMPLATE_NAME="--all"
      shift
      ;;
    -*)
      echo "Error: unknown option '$1'" >&2
      exit 1
      ;;
    *)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: unexpected argument '$1' (only one template name allowed)" >&2
        exit 1
      fi
      TEMPLATE_NAME="$1"
      shift
      ;;
  esac
done

if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-eject-template.sh [--force] [--dry-run] <template_name|--all>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)

# Resolve target directory
TEMPLATES_DIR=$("$SCRIPT_DIR/config-read-path.sh" templates meta/templates)
if [[ "$TEMPLATES_DIR" != /* ]]; then
  TEMPLATES_DIR="$PROJECT_ROOT/$TEMPLATES_DIR"
fi

# Eject a single template. Returns 0 on success, 1 on error, 2 if exists.
_eject_one() {
  local key="$1"
  local source_path="$PLUGIN_ROOT/templates/${key}.md"
  local target_path="$TEMPLATES_DIR/${key}.md"
  local display_target
  display_target=$(config_display_path "$target_path" "$PLUGIN_ROOT")

  if [ ! -f "$source_path" ]; then
    AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
    echo "Error: No plugin default template for '$key'. Available: $AVAILABLE" >&2
    return 1
  fi

  if [ -f "$target_path" ] && [ "$FORCE" = false ]; then
    if [ "$DRY_RUN" = true ]; then
      echo "Would skip: $key (exists at $display_target, use --force to overwrite)"
    else
      echo "Exists: $display_target (use --force to overwrite)" >&2
    fi
    return 2
  fi

  if [ "$DRY_RUN" = true ]; then
    if [ -f "$target_path" ]; then
      echo "Would overwrite: $key -> $display_target"
    else
      echo "Would eject: $key -> $display_target"
    fi
    return 0
  fi

  local existed=false
  [ -f "$target_path" ] && existed=true

  mkdir -p "$TEMPLATES_DIR"
  cp "$source_path" "$target_path"
  if [ "$existed" = true ]; then
    echo "Overwritten: $key -> $display_target"
  else
    echo "Ejected: $key -> $display_target"
  fi
}

if [ "$TEMPLATE_NAME" = "--all" ]; then
  HAD_ERROR=false
  HAD_EXISTS=false
  for KEY in $(config_enumerate_templates "$PLUGIN_ROOT"); do
    RC=0
    _eject_one "$KEY" || RC=$?
    if [ "$RC" -eq 1 ]; then
      HAD_ERROR=true
    elif [ "$RC" -eq 2 ]; then
      HAD_EXISTS=true
    fi
  done
  if [ "$HAD_ERROR" = true ]; then
    echo "Some templates were not ejected. Fix the errors above and re-run with --force to complete." >&2
    exit 1
  elif [ "$HAD_EXISTS" = true ]; then
    echo "Some templates already exist. Re-run with --force to overwrite." >&2
    exit 2
  fi
  exit 0
else
  _eject_one "$TEMPLATE_NAME"
fi
```

Make executable: `chmod +x scripts/config-eject-template.sh`

### Success Criteria:

#### Automated Verification:

- [ ] `config-eject-template.sh plan` creates
      `meta/templates/plan.md` with plugin default content
- [ ] Creates the templates directory if it doesn't exist
- [ ] Exit code 2 when target already exists without `--force`
- [ ] `--force` overwrites existing file and exits 0
- [ ] `--all` ejects all 5 templates
- [ ] `--all` with some existing files exits 2 (without `--force`); error
      (exit 1) takes precedence over exists (exit 2)
- [ ] `--all --force` overwrites all existing files
- [ ] `--dry-run` outputs what would happen without writing files
- [ ] Multiple positional arguments produce an error
- [ ] Unknown template name produces error with available templates list
- [ ] Respects `paths.templates` config override for target directory
- [ ] Tests pass in `test-config.sh`

---

## Phase 5: `config-diff-template.sh` Script

### Overview

Create a script that shows differences between a user's customised template
and the plugin default.

### Changes Required:

#### 1. Create `scripts/config-diff-template.sh`

**File**: `scripts/config-diff-template.sh` (new)
**Changes**: New script. Uses `config_resolve_template()` to find the
user's override (Tier 1 or Tier 2 only — skips Tier 3 since we diff
*against* the plugin default). Outputs unified diff format.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Shows differences between a user's customised template and the plugin
# default.
#
# Usage: config-diff-template.sh <template_name>
#
# Exit codes:
#   0 - Diff shown successfully
#   1 - Error (unknown template, usage error, diff error)
#   2 - No user override exists (using plugin default)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-diff-template.sh <template_name>" >&2
  exit 1
fi

# Verify it's a known template
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ ! -f "$DEFAULT_PATH" ]; then
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Unknown template '$TEMPLATE_NAME'. Available: $AVAILABLE" >&2
  exit 1
fi

# Resolve the template — if it resolves to plugin default, there's no
# user override to diff against
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  echo "No customised template found for '$TEMPLATE_NAME' — using plugin default." >&2
  exit 2
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"

if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" ]; then
  echo "No customised template found for '$TEMPLATE_NAME' — using plugin default." >&2
  exit 2
fi

DISPLAY_DEFAULT=$(config_display_path "$DEFAULT_PATH" "$PLUGIN_ROOT")
DISPLAY_USER=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

echo "Comparing plugin default vs user override:"
echo "  Default: $DISPLAY_DEFAULT"
echo "  User:    $DISPLAY_USER"
echo ""

# diff exits 0 if identical, 1 if different, 2+ if trouble
RC=0
diff -u "$DEFAULT_PATH" "$RESOLVED_PATH" || RC=$?
if [ "$RC" -gt 1 ]; then
  exit 1
elif [ "$RC" -eq 0 ]; then
  echo "Templates are identical."
fi
```

Make executable: `chmod +x scripts/config-diff-template.sh`

### Success Criteria:

#### Automated Verification:

- [ ] `config-diff-template.sh plan` with no override outputs "No customised
      template found" message and exits 2
- [ ] With a user override, outputs unified diff
- [ ] Diff output includes the file paths for both default and user template
- [ ] Unknown template name produces error listing available templates
- [ ] Missing argument produces usage message and exit 1
- [ ] Config path overrides (Tier 1) are correctly found and diffed
- [ ] Templates directory overrides (Tier 2) are correctly found and diffed
- [ ] Tests pass in `test-config.sh`

---

## Phase 6: `config-reset-template.sh` Script

### Overview

Create a script that finds a user's customised template and either reports
what it found (default) or deletes it (`--confirm`). The skill layer
handles user confirmation between the two invocations.

### Changes Required:

#### 1. Create `scripts/config-reset-template.sh`

**File**: `scripts/config-reset-template.sh` (new)
**Changes**: New script. Uses `config_resolve_template()` to find the user's
override. Without `--confirm`, reports what it found. With `--confirm`,
deletes the override file.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Resets a user's customised template to the plugin default.
#
# Usage: config-reset-template.sh [--confirm] <template_name>
#
# Without --confirm: reports the override location (dry-run).
# With --confirm: deletes the override file.
#
# Exit codes:
#   0 - Override found (or successfully deleted with --confirm)
#   1 - Error (unknown template, usage error)
#   2 - No override exists (already using plugin default)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CONFIRM=false
TEMPLATE_NAME=""

while [ $# -gt 0 ]; do
  case "$1" in
    --confirm)
      CONFIRM=true
      shift
      ;;
    -*)
      echo "Error: unknown option '$1'" >&2
      exit 1
      ;;
    *)
      if [ -n "$TEMPLATE_NAME" ]; then
        echo "Error: unexpected argument '$1'" >&2
        exit 1
      fi
      TEMPLATE_NAME="$1"
      shift
      ;;
  esac
done

if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-reset-template.sh [--confirm] <template_name>" >&2
  exit 1
fi

# Verify it's a known template
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ ! -f "$DEFAULT_PATH" ]; then
  AVAILABLE=$(config_format_available_templates "$PLUGIN_ROOT")
  echo "Error: Unknown template '$TEMPLATE_NAME'. Available: $AVAILABLE" >&2
  exit 1
fi

# Resolve the template
RESOLUTION=$(config_resolve_template "$TEMPLATE_NAME" "$PLUGIN_ROOT") || {
  echo "No customised template found for '$TEMPLATE_NAME' — already using plugin default." >&2
  exit 2
}

IFS=$'\t' read -r RESOLVED_SOURCE RESOLVED_PATH <<< "$RESOLUTION"

if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_PLUGIN_DEFAULT" ]; then
  echo "No customised template found for '$TEMPLATE_NAME' — already using plugin default." >&2
  exit 2
fi

# Check if the override file is outside the project root
PROJECT_ROOT=$(config_project_root)
OUTSIDE_PROJECT=false
if [[ "$RESOLVED_PATH" != "$PROJECT_ROOT"/* ]]; then
  OUTSIDE_PROJECT=true
fi

DISPLAY_PATH=$(config_display_path "$RESOLVED_PATH" "$PLUGIN_ROOT")

if [ "$CONFIRM" = false ]; then
  echo "Found override: $RESOLVED_SOURCE"
  echo "Path: $DISPLAY_PATH"
  if [ "$OUTSIDE_PROJECT" = true ]; then
    echo "Warning: This file is outside the project directory ($RESOLVED_PATH)."
  fi
  if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" ]; then
    echo "Note: After deletion, also remove the 'templates.$TEMPLATE_NAME' entry from your config."
  fi
  exit 0
fi

# Delete the override
rm "$RESOLVED_PATH"
echo "Reset: $TEMPLATE_NAME"
if [ "$RESOLVED_SOURCE" = "$CONFIG_TEMPLATE_SOURCE_CONFIG_PATH" ]; then
  echo "Note: Also remove the 'templates.$TEMPLATE_NAME' entry from your config."
fi
```

Make executable: `chmod +x scripts/config-reset-template.sh`

### Success Criteria:

#### Automated Verification:

- [ ] `config-reset-template.sh plan` with no override exits 2 with
      "already using plugin default" message
- [ ] With an override, exits 0 and outputs the override path and source
- [ ] `--confirm` with an override deletes the file
- [ ] Config path overrides produce note about removing config entry
- [ ] Unknown template name produces error listing available templates
- [ ] Missing argument produces usage message and exit 1
- [ ] Tests pass in `test-config.sh`

#### Manual Verification:

- [ ] `/accelerator:configure templates reset plan` shows override info
      and asks for confirmation before running with `--confirm`

---

## Phase 7: Configure Skill SKILL.md Updates

### Overview

Update the configure skill to dispatch template subcommands, update the
`argument-hint`, and add template management to the `help` text.

### Changes Required:

#### 1. Update frontmatter `argument-hint`

**File**: `skills/config/configure/SKILL.md`
**Changes**: Update the `argument-hint` and `description` fields in the
frontmatter.

```yaml
argument-hint: "[view | create | help | templates ...]"
description: "View, create, or edit Accelerator plugin configuration. Manage document templates."
```

#### 2. Add `templates` dispatch section

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add a new H3 section after the `help` section (before
`## Important Notes`). This section handles all template subcommands.

Insert before the `## Important Notes` section:

```markdown
### `templates`

When the user's argument starts with `templates`, dispatch based on the
action that follows. The `CLAUDE_PLUGIN_ROOT` environment variable points
to the plugin installation directory where scripts are located.

#### `templates list`

Run the list script and display its output:

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-list-template.sh"
\```

Present the table output to the user.

#### `templates show <key>`

Run the show script with the template key:

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-show-template.sh" <key>
\```

Present the source metadata and template content to the user. If the user
doesn't specify a key, ask which template they'd like to see, or suggest
running `templates list` first.

#### `templates eject <key>` or `templates eject --all`

**Before ejecting**, run with `--dry-run` to preview what will happen:

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --dry-run <key|--all>
\```

Present the dry-run output to the user. If any templates already exist
(exit code 2), ask whether they want to overwrite. If the user confirms
overwriting, run a second dry-run with `--force` to show the full preview
(including which files will be overwritten):

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --dry-run --force <key|--all>
\```

Present this preview, then run the actual eject with `--force`:

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" --force <key|--all>
\```

If no templates already exist (exit code 0 from the initial dry-run),
proceed directly with the eject (no `--force` needed):

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-eject-template.sh" <key|--all>
\```

If the user says `eject --all` or `eject all`, pass `--all` to the script.

After successful ejection, inform the user:
- Which file(s) were created and where
- That they can now edit the template(s) at the ejected path
- That the customised template will be used by the relevant skill on next
  invocation

#### `templates diff <key>`

Run the diff script:

\```bash
bash "$CLAUDE_PLUGIN_ROOT/scripts/config-diff-template.sh" <key>
\```

Present the diff output to the user. If exit code is 2, no customisation
exists — relay the "using plugin default" message.

#### `templates reset <key>`

This action removes a user's customised template to revert to the plugin
default. Reset operates on a **single template at a time** — if the user
requests resetting all templates, process them one-by-one with individual
confirmations.

1. Determine the template key. If not provided, ask the user.
2. Run the reset script without `--confirm` to check for an override:
   \```bash
   bash "$CLAUDE_PLUGIN_ROOT/scripts/config-reset-template.sh" <key>
   \```
3. If exit code 2: tell the user "No customised template found for '<key>'
   — already using plugin default."
4. If exit code 0: present the override information to the user and ask for
   confirmation. Show the file path and note about config entry if present.
   If the output includes "Warning: This file is outside the project
   directory", explicitly highlight this to the user and ask them to
   confirm they want to delete a file outside the project root.
5. On confirmation, run with `--confirm`:
   \```bash
   bash "$CLAUDE_PLUGIN_ROOT/scripts/config-reset-template.sh" --confirm <key>
   \```
6. Inform the user that the template was reset. If the script output
   includes a note about removing a config entry (i.e., the override was
   a config path / Tier 1), also remove the `templates.<key>` entry from
   the config using the Edit tool. Check both `.claude/accelerator.md`
   (team) and `.claude/accelerator.local.md` (local) for the entry:
   - If the entry exists in **local only**: remove it from local.
   - If the entry exists in **team only**: remove it from team.
   - If the entry exists in **both with the same value**: remove from both.
   - If the entry exists in **both with different values**: remove from
     local only (team config may affect other team members). Inform the
     user that the team config still has a `templates.<key>` entry and
     they should coordinate with their team if it should also be removed.
```

#### 3. Update `help` subcommand template section

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add template management commands to the existing `### templates`
subsection within the `help` subcommand output.

Insert after the cross-references note at the end of the `### templates`
subsection (the paragraph mentioning `config-read-template.sh` and the
`!` preprocessor):

```markdown

#### Template Management Commands

Use `/accelerator:configure templates <action>` to manage templates:

| Command | Description |
|---------|-------------|
| `templates list` | List all templates with resolution source and path |
| `templates show <key>` | Display the effective template content |
| `templates eject <key>` | Copy plugin default to your templates directory |
| `templates eject --all` | Eject all templates at once |
| `templates diff <key>` | Show differences between your template and the default |
| `templates reset <key>` | Remove your customisation, revert to plugin default |

Available template keys: `plan`, `research`, `adr`, `validation`,
`pr-description`.
```

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes
- [ ] The skill frontmatter `argument-hint` includes `templates`
- [ ] The skill frontmatter `description` mentions template management

#### Manual Verification:

- [ ] `/accelerator:configure templates list` shows the template table
- [ ] `/accelerator:configure templates show plan` shows template content
- [ ] `/accelerator:configure templates eject plan` copies template to
      templates directory
- [ ] `/accelerator:configure templates diff plan` shows diff after ejection
- [ ] `/accelerator:configure templates reset plan` deletes ejected template
- [ ] `/accelerator:configure help` includes template management commands
      section

---

## Phase 8: Tests

### Overview

Add comprehensive tests for all new and modified scripts to
`test-config.sh`.

### Changes Required:

#### 1. Add tests to `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Add new test sections before the summary output. Follow the
existing patterns: `setup_repo`, inline fixtures via heredocs, process-level
script invocation, `assert_eq`/`assert_contains`/`assert_exit_code`.

Add a script reference at the top of the file (after line 17):

```bash
LIST_TEMPLATE="$SCRIPT_DIR/config-list-template.sh"
SHOW_TEMPLATE="$SCRIPT_DIR/config-show-template.sh"
EJECT_TEMPLATE="$SCRIPT_DIR/config-eject-template.sh"
DIFF_TEMPLATE="$SCRIPT_DIR/config-diff-template.sh"
RESET_TEMPLATE="$SCRIPT_DIR/config-reset-template.sh"
```

**New assertion helpers** (add alongside existing `assert_eq` etc.):

```bash
assert_file_exists() {
  local test_name="$1" file_path="$2"
  if [ -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_not_exists() {
  local test_name="$1" file_path="$2"
  if [ ! -f "$file_path" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected file to not exist: $file_path"
    FAIL=$((FAIL + 1))
  fi
}

assert_file_content_eq() {
  local test_name="$1" file_path="$2" expected="$3"
  local actual
  actual=$(cat "$file_path" 2>/dev/null) || {
    echo "  FAIL: $test_name"
    echo "    File not found: $file_path"
    FAIL=$((FAIL + 1))
    return
  }
  if [ "$expected" = "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected content: $(printf '%q' "$expected")"
    echo "    Actual content:   $(printf '%q' "$actual")"
    FAIL=$((FAIL + 1))
  fi
}
```

**Test sections to add:**

Note: Tests for shared helpers (`config_enumerate_templates`,
`config_resolve_template`, `config_format_available_templates`),
`config-dump.sh` pr-description, and `config-read-template.sh` regression
tests are written in Phase 1 (Step 4). The assertion helpers and direct
function test setup are also introduced in Phase 1. Phase 8 adds tests
for the new management scripts and integration tests only.

##### `config-list-template.sh` tests

- No config → all 5 templates show `plugin default` source
- User override in `meta/templates/` → shows `user override`
- Config path override → shows `config path`
- Custom `paths.templates` → finds override in custom directory
- Output is valid markdown table (starts with header row)
- Mixed sources in single run: config path for one key, user override
  for another, plugin default for rest → all correctly labelled

##### `config-show-template.sh` tests

- No override → shows `Source: plugin default (...)` + raw content
- User override → shows `Source: user override (...)` + user content
- Config path override → shows `Source: config path (...)` + content
- Unknown template name → error to stderr, exit 1
- No argument → usage to stderr, exit 1
- Content is raw (no code fences added)

##### `config-eject-template.sh` tests

- Ejects template to default `meta/templates/` directory
- Creates templates directory if it doesn't exist
- File content matches plugin default
- Exit code 2 when target exists without `--force`
- `--force` overwrites existing file, exit 0
- `--all` ejects all 5 templates
- `--all --force` overwrites all existing
- `--all` with some existing: exit 2, but non-conflicting templates
  still written
- `--all` with one error and one exists: exit 1 (error takes precedence)
- `--dry-run` outputs what would happen without writing files
- `--dry-run` produces exit 2 for existing files
- Multiple positional arguments → error, exit 1
- Respects `paths.templates` config override
- Unknown template name → error, exit 1
- No argument → usage, exit 1

##### `config-diff-template.sh` tests

- No override → outputs "No customised template found" message to stderr,
  exit 2
- User override with differences → outputs unified diff
- User override with known added line → diff output contains `+` prefix
  for the addition (verifies diff argument direction)
- User override identical to default → outputs "Templates are identical."
- Config path override → diffs correctly
- Unknown template name → error, exit 1
- No argument → usage, exit 1

##### `config-reset-template.sh` tests

- No override → exit 2 with "already using plugin default" to stderr
- User override without `--confirm` → exit 0, outputs override path and
  source
- Config path override without `--confirm` → exit 0, includes note about
  config entry removal
- Config path override pointing outside project root without `--confirm` →
  output includes "Warning: This file is outside the project directory"
- `--confirm` with user override → deletes file
- `--confirm` with no override → exit 2
- Unknown template name → error, exit 1
- No argument → usage, exit 1

##### `config-dump.sh` pr-description test

- Output contains `templates.pr-description` row

##### Skill integration check

- Configure skill SKILL.md contains `config-list-template.sh`
- Configure skill SKILL.md contains `config-show-template.sh`
- Configure skill SKILL.md contains `config-eject-template.sh`
- Configure skill SKILL.md contains `config-diff-template.sh`
- Configure skill SKILL.md contains `config-reset-template.sh`

##### Integration tests

- Eject then list: `config-eject-template.sh plan` followed by
  `config-list-template.sh` shows `plan` as `user override`
- Eject then diff (identical): `config-eject-template.sh plan` followed
  by `config-diff-template.sh plan` produces diff with no content lines
- Eject + edit + diff: eject, append a line, then diff shows the addition
  with `+` prefix
- Eject then reset: `config-eject-template.sh plan` followed by
  `config-reset-template.sh --confirm plan` deletes the override

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes with 0 failures
- [ ] All new test sections produce PASS results
- [ ] No regressions in existing tests

---

## Testing Strategy

### Unit Tests:

All in `scripts/test-config.sh`:
- Each new script gets its own `=== ... ===` test section
- `config_enumerate_templates` function tested directly (sourced)
- Edge cases: unknown keys, missing files, empty directories, argument
  validation

### Integration Tests:

(Enumerated in Phase 8 test catalogue above.)

- Eject → list: verifies `user override` source
- Eject → diff: verifies no diff for identical content
- Eject + edit → diff: verifies additions shown with `+` prefix
- Eject → reset: verifies deletion of override

### Manual Testing Steps:

1. Run `/accelerator:configure templates list` — verify table output
2. Run `/accelerator:configure templates show plan` — verify content display
3. Run `/accelerator:configure templates eject plan` — verify file creation
4. Edit the ejected template
5. Run `/accelerator:configure templates diff plan` — verify diff shows edits
6. Run `/accelerator:configure templates reset plan` — verify deletion with
   confirmation
7. Run `/accelerator:configure templates list` — verify `plan` reverted to
   `plugin default`
8. Run `/accelerator:configure templates eject --all` — verify all templates
   ejected
9. Run `/accelerator:configure help` — verify template management section
   appears

## References

- Research: `meta/research/2026-03-29-template-management-subcommands.md`
- Template resolution: `scripts/config-read-template.sh`
- Config dump: `scripts/config-dump.sh:209-224`
- Configure skill: `skills/config/configure/SKILL.md`
- Test harness: `scripts/test-config.sh`
- Shared utilities: `scripts/config-common.sh`
