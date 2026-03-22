# Configuration Infrastructure Implementation Plan

## Overview

Establish the foundation for userspace configurability in the Accelerator plugin.
This plan creates the config file format, reader scripts, SessionStart hook
enhancements, and a setup skill. All subsequent configuration plans (agent
overrides, review customisation, template/path overrides) build on this
infrastructure.

## Current State Analysis

The plugin has **zero userspace configuration mechanisms**. All behaviour is
hardcoded in SKILL.md files and shell scripts. The only user-override pattern is
`describe-pr` reading `meta/templates/pr-description.md` from the user's
repository — a content template, not plugin configuration.

The existing infrastructure provides:
- `${CLAUDE_PLUGIN_ROOT}` for plugin-internal path references
- `!`command`` preprocessor in skills (used by `commit/SKILL.md:11-12`)
- SessionStart hooks injecting `additionalContext` (used by `vcs-detect.sh`)
- Shell scripts following a `SCRIPT_DIR` + `source` pattern for shared utilities

### Key Discoveries:

- `scripts/vcs-common.sh` provides `find_repo_root()` — walks up from `$PWD`
  looking for `.jj`/`.git`. Useful for locating project-level config files.
- `skills/decisions/scripts/adr-read-status.sh:27-43` already parses YAML
  frontmatter using a line-by-line `while IFS= read -r` loop — same pattern
  needed for config parsing.
- The `!`command`` preprocessor runs at skill invocation time, replacing the
  entire line with stdout. If a script outputs nothing, the line becomes blank —
  clean fallback for "no config found."
- SessionStart hooks output JSON with `additionalContext` field via `jq`
  (`hooks/vcs-detect.sh:77-82`).

## Desired End State

After this plan:
1. Users can create `.claude/accelerator.md` (team-shared) and/or
   `.claude/accelerator.local.md` (personal, gitignored) with YAML frontmatter
   for structured settings and a markdown body for free-form project context.
2. Shell scripts can read individual config values with dot-notation keys and
   defaults.
3. Skills can inject project context via the `!`command`` preprocessor.
4. The SessionStart hook detects config files and injects a summary of active
   configuration into the session context.
5. Users can run `/accelerator:configure` to create or view their configuration.
6. A test script validates the config reader logic.

## What We're NOT Doing

- Adding any actual configuration keys yet (agents, lenses, paths, templates) —
  that's Plans 2-4
- Supporting directory-based overrides (`.claude/accelerator/`) — deferred until
  config options grow
- Per-skill configuration — global config only
- Config validation or schema enforcement — keep it simple for now
- Deeply nested YAML (3+ levels) — max 2 levels for bash parsability
- Documenting future configuration keys in the configure skill — each plan
  should update the skill's help text when it adds keys (deferred to Plans 2-4)

### Known Parser Constraints

The awk-based YAML parser supports simple scalar values only. The following are
**not** supported and should be documented in the configure skill's help text:

- Values containing colons (e.g., URLs like `https://example.com`) — the parser
  splits on the first colon only, so these work correctly
- Multi-line YAML values (block scalars `|` / `>`)
- YAML comments (the `#` character in values will be included as-is)
- Deeply nested structures (3+ levels)
- Empty quoted values (`""` or `''`) are passed through as empty strings, not
  treated as "key not found." This means `key: ""` is semantically different
  from the key being absent — consumers receive an empty string rather than the
  default. Consumers that need to distinguish "explicitly empty" from "not set"
  should check for empty values. This behaviour may serve as a future sentinel
  for unsetting team config values (see deferred item 4).

If these constraints become limiting as Plans 2-4 add real keys, upgrade to a
more robust parser at that point.

## Implementation Approach

Use the same patterns already established in the codebase: shell scripts with
`set -euo pipefail`, `SCRIPT_DIR` for relative path resolution,
`source` for shared utilities, `jq` for JSON output in hooks, and the
`!`command`` preprocessor for injecting dynamic content into skills.

Config scripts live in `scripts/` with a `config-` prefix (matching the `vcs-`
prefix convention), not in a subdirectory. The sourced utility file
(`config-common.sh`) intentionally omits `set -euo pipefail` because it
inherits the caller's shell options — matching the `vcs-common.sh` convention.

Config file precedence: `.claude/accelerator.local.md` overrides
`.claude/accelerator.md` for YAML keys (last-writer-wins). Markdown bodies
from both files are concatenated (team context + personal context).

## Phase 1: Config Reader Scripts

### Overview

Create the core shell scripts that locate, parse, and read configuration files.
These scripts are used by all subsequent phases and plans.

### Changes Required:

#### 1. Shared Config Utilities

**File**: `scripts/config-common.sh`
**Changes**: New file. Shared functions for locating config files and parsing
YAML frontmatter. Sources `vcs-common.sh` for repo-root detection rather than
reimplementing it.

```bash
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
    in_fm { print }
    END { if (!closed) exit 1 }
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
    in_fm && /^---[[:space:]]*$/ { past_fm = 1; next }
    in_fm { next }
    past_fm { print }
  ' "$file"
}

# Trim leading and trailing blank lines from stdin.
# Centralised to avoid duplicating fragile sed idioms.
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
```

#### 2. Value Reader Script

**File**: `scripts/config-read-value.sh`
**Changes**: New file. Reads a single config value by dot-notation key with a
default fallback. Uses string comparison (not regex) for key matching to avoid
metacharacter issues.

```bash
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
```

#### 3. Context Reader Script

**File**: `scripts/config-read-context.sh`
**Changes**: New file. Reads and concatenates markdown bodies from config files.
Outputs nothing if no config files exist or if bodies are empty. Uses
`config_trim_body` from config-common.sh for whitespace trimming.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads the markdown body (project context) from accelerator config files.
# Outputs the team context first, then local context, separated by a blank line.
# If no config files exist or bodies are empty, outputs nothing.
#
# Usage: config-read-context.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

OUTPUT=""
while IFS= read -r config_file; do
  body=$(config_extract_body "$config_file")
  trimmed=$(printf '%s\n' "$body" | config_trim_body)
  if [ -n "$trimmed" ]; then
    if [ -n "$OUTPUT" ]; then
      OUTPUT="$OUTPUT"$'\n\n'"$trimmed"
    else
      OUTPUT="$trimmed"
    fi
  fi
done < <(config_find_files)

if [ -n "$OUTPUT" ]; then
  echo "## Project Context"
  echo ""
  echo "The following project-specific context has been provided. Take this into"
  echo "account when making decisions, selecting approaches, and generating output."
  echo ""
  printf '%s\n' "$OUTPUT"
fi
```

#### 4. Config Summary Script (for SessionStart hook)

**File**: `scripts/config-summary.sh`
**Changes**: New file. Outputs a brief summary of active configuration for the
SessionStart hook to inject via `additionalContext`. Emits warnings for
malformed config files.

```bash
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
```

### Success Criteria:

#### Automated Verification:

- [ ] `scripts/config-common.sh` exists and is executable
- [ ] `scripts/config-read-value.sh` exists and is executable
- [ ] `scripts/config-read-context.sh` exists and is executable
- [ ] `scripts/config-summary.sh` exists and is executable
- [ ] `bash scripts/test-config.sh` passes all tests

#### Manual Verification:

- [ ] `config-read-value.sh agents.reviewer reviewer` outputs `reviewer` (default)
- [ ] With a test config file, `config-read-value.sh agents.reviewer reviewer`
  outputs the configured value
- [ ] `config-read-context.sh` outputs nothing when no config files exist
- [ ] `config-read-context.sh` outputs formatted context when config files exist

---

## Phase 2: Test Script

### Overview

Create a comprehensive test script for the config reader, following the pattern
established by `skills/decisions/scripts/test-adr-scripts.sh`.

### Changes Required:

#### 1. Config Test Script

**File**: `scripts/test-config.sh`
**Changes**: New file. Tests for all config scripts including the shared
utility functions.

The test script should:

1. Create temporary directories simulating project roots with `.git/` dirs
2. Create `.claude/accelerator.md` and `.claude/accelerator.local.md` with
   various configurations
3. **Working directory management**: Since `config_project_root()` calls
   `find_repo_root()` which walks up from `$PWD`, each test case that uses
   a different config setup must `cd` into its temp directory. Use subshells
   `(cd "$tmpdir" && run_test)` to isolate each test's working directory,
   preventing cross-contamination between tests. Follow the same `setup_repo`
   pattern from `test-adr-scripts.sh` but wrap script invocations in subshells.
4. Test cases:

**config_extract_frontmatter tests (direct function tests):**
- File with valid frontmatter → outputs frontmatter content
- File with no frontmatter (no `---` on line 1) → outputs nothing
- File with unclosed frontmatter (only opening `---`) → outputs nothing, exit 1
- File where `---` appears on line 1 as frontmatter AND later in body → only
  extracts content between first and second `---`
- File with trailing spaces on `---` delimiter → still recognised as delimiter
- File with only `---` on line 1 and `---` on line 2 → outputs nothing (empty
  frontmatter)

**config_extract_body tests (direct function tests):**
- File with valid frontmatter and body → outputs only body after closing `---`
- File with no frontmatter → outputs entire file
- File with unclosed frontmatter → outputs nothing (malformed)
- File with `---` horizontal rule in body after frontmatter → includes it in
  body output
- File with empty body after frontmatter → outputs nothing

**config-read-value.sh tests:**
- No config files → outputs default
- Top-level key present → outputs value
- Nested key (section.key) present → outputs value
- Key not found → outputs default
- Key not found, no default → outputs nothing
- Local overrides team for same key
- Values with quotes (single and double) are stripped
- Values with trailing whitespace are trimmed
- Empty frontmatter → outputs default
- No frontmatter (plain markdown file) → outputs default
- Array values (e.g., `[a, b, c]`) are output as-is
- Values containing colons (e.g., `https://example.com`) → output correctly
- Blank line within a YAML section → does not terminate section scanning
- Key with underscore (e.g., `max_count`) → matches exactly, not as regex
- Unclosed frontmatter → outputs default, warning to stderr

**config-read-context.sh tests:**
- No config files → outputs nothing
- Team config with body → outputs body under "Project Context" header
- Local config with body → outputs body under "Project Context" header
- Both configs with bodies → outputs both, team first
- Config with frontmatter but no body → outputs nothing
- Config with empty body → outputs nothing
- Config with unclosed frontmatter → outputs nothing (not entire file)

**config-summary.sh tests:**
- No config files → outputs nothing
- Team config present → lists it
- Both configs present → lists both
- Config with frontmatter sections → lists section names
- Duplicate section keys across team and local → deduplicated in output
- Config with whitespace-only body → does not report "project context: provided"
- Config with frontmatter but no top-level keys → no "Configured sections" line
- Section keys with hyphens and digits → included in output
- Unclosed frontmatter → warns to stderr, does not crash

**config-detect.sh tests (hook output):**
- No config files → outputs nothing (empty stdout)
- Config present → outputs valid JSON with hookSpecificOutput.additionalContext
- JSON structure matches SessionStart hook contract

Follow the `assert_eq` / `assert_exit_code` pattern from
`skills/decisions/scripts/test-adr-scripts.sh`.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes all tests
- [ ] Test script is executable: `chmod +x scripts/test-config.sh`

---

## Phase 3: SessionStart Hook Enhancement

### Overview

Enhance the SessionStart hook to detect config files and inject a summary into
the session context alongside the existing VCS detection.

### Changes Required:

#### 1. Hook Configuration Update

**File**: `hooks/hooks.json`
**Changes**: Add config detection to the SessionStart hooks.

**Note**: Verify during implementation whether the plugin system supports
multiple matcher entries with the same (empty) matcher. If both entries are
processed independently, the separate-entry approach below is correct and keeps
concerns isolated. If only the first match is used, add config-detect as a
second hook within the existing entry's `hooks` array instead.

Preferred approach (separate entries for isolation):
```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh"
          }
        ]
      },
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/config-detect.sh"
          }
        ]
      }
    ],
```

Fallback approach (if single-matcher-entry required):
```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/vcs-detect.sh"
          },
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/config-detect.sh"
          }
        ]
      }
    ],
```

#### 2. Config Detection Hook Script

**File**: `hooks/config-detect.sh`
**Changes**: New file. Detects config files and injects a summary via
`additionalContext`.

```bash
#!/usr/bin/env bash

# Check for jq dependency (matching vcs-detect.sh pattern)
if ! command -v jq &>/dev/null; then
  echo '{"systemMessage":"WARNING: jq is not installed. Accelerator config detection could not run. Install jq for config support."}'
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Run config-summary.sh. Let stderr pass through naturally (matching
# vcs-detect.sh pattern) so warnings reach the terminal without polluting
# the JSON output. If the script fails, discard stdout and continue.
SUMMARY=$("$SCRIPT_DIR/../scripts/config-summary.sh") || SUMMARY=""

# Only output if there's something to report
if [ -n "$SUMMARY" ]; then
  jq -n --arg context "$SUMMARY" '{
    "hookSpecificOutput": {
      "hookEventName": "SessionStart",
      "additionalContext": $context
    }
  }'
fi
```

### Success Criteria:

#### Automated Verification:

- [ ] `hooks/config-detect.sh` exists and is executable
- [ ] `hooks/hooks.json` is valid JSON: `jq . hooks/hooks.json`
- [ ] Hook JSON includes config-detect.sh in SessionStart configuration
- [ ] `bash scripts/test-config.sh` config-detect.sh tests pass

#### Manual Verification:

- [ ] Starting a Claude Code session in a project with
  `.claude/accelerator.md` shows config detection in session context
- [ ] Starting a session without config files produces no extra context
- [ ] VCS detection continues to work alongside config detection

---

## Phase 4: Configure Skill

### Overview

Create the `/accelerator:configure` skill that helps users create, view, and
understand their configuration options.

### Changes Required:

#### 1. Skill Directory and File

**File**: `skills/config/configure/SKILL.md`
**Changes**: New skill.

```markdown
---
name: configure
description: View, create, or edit Accelerator plugin configuration. Use when the
  user wants to customise how Accelerator skills behave in their project.
argument-hint: "[view | create | help]"
disable-model-invocation: true
---

# Configure Accelerator

You help users manage their Accelerator plugin configuration.

## Configuration Files

Accelerator reads configuration from two files in the project's `.claude/`
directory:

| File | Scope | Git | Purpose |
|------|-------|-----|---------|
| `.claude/accelerator.md` | Team-shared | Committed | Shared project context and settings |
| `.claude/accelerator.local.md` | Personal | Gitignored | Personal overrides and preferences |

Both files use YAML frontmatter for structured settings and a markdown body for
free-form project context. Local settings override team settings for the same
key.

## Available Actions

When invoked:

1. **Check current configuration state**:
   - Check if `.claude/accelerator.md` exists
   - Check if `.claude/accelerator.local.md` exists
   - If either exists, read and display current settings
   - **If a config file already exists, always show its current contents and ask
     the user to confirm before overwriting. Never silently replace an existing
     config file.**

2. **Based on the argument or user intent**:

### `view` (or no argument with existing config)

Display the current configuration:
```
## Current Accelerator Configuration

### Team Config (.claude/accelerator.md)
[Display frontmatter settings as a formatted table]
[Display markdown body if present]

### Personal Config (.claude/accelerator.local.md)
[Display frontmatter settings as a formatted table]
[Display markdown body if present]

### Effective Settings
[Show merged settings with source attribution]
```

### `create` (or no argument with no existing config)

Help the user create a configuration file. Focus on gathering project context
for the markdown body — this is the primary value of config files in the
current version. Structured settings will be added in future versions.

1. Ask whether they want to create a team config (shared) or personal config
   (local), or both
2. If creating a personal config, check whether `.claude/accelerator.local.md`
   is in `.gitignore` (or `.claude/.gitignore`). If not, offer to add it to
   the repo root `.gitignore`.
3. Ask about their project context — frame questions around "What should
   Accelerator skills know about your project?":
   - What tech stack do they use? (languages, frameworks, build system)
   - Any specific conventions or standards?
   - Any domain-specific context that should inform skills?
   - Build and test commands?
4. Write the config file with a markdown body containing the gathered context.
   Include a minimal YAML frontmatter section with a comment noting that
   structured settings will be available in future versions.

### `help`

Display the configuration reference:
```
## Accelerator Configuration Reference

### File Format

Both config files use YAML frontmatter with a markdown body:

\```yaml
---
# Structured settings (YAML) — settings will be added in future versions.
# For now, the frontmatter section can be left empty or omitted.
---

# Free-form project context (markdown)
Additional context that skills will consider when making decisions.
\```

### Structured Settings

Structured configuration settings (for customising agents, review behaviour,
output paths, etc.) will be added in future versions of the plugin. When
available, they will use YAML frontmatter with max 2-level nesting:

\```yaml
---
section:
  key: value
---
\```

### Project Context

The markdown body of your config file is injected into skills that benefit
from project awareness. This is the primary configuration mechanism in the
current version. Use it for:
- Tech stack description
- Coding conventions
- Domain-specific terminology
- Build and test commands
- Architecture notes

### Parser Constraints

The configuration parser supports simple scalar YAML values. The following
are not currently supported in frontmatter values:
- Multi-line YAML values (block scalars `|` / `>`)
- YAML comments (the `#` character in values is included as-is)
- Nesting deeper than 2 levels
```

## Important Notes

- Config changes take effect on the next skill invocation (no restart needed
  for skills using the `!` preprocessor)
- The SessionStart hook summary requires a session restart to update
- `.local.md` files should be gitignored — the `create` action will help
  with this
- Team config should contain only project-relevant context, not personal
  preferences
```

#### 2. Plugin Manifest Update

**File**: `.claude-plugin/plugin.json`
**Changes**: Add the config skill directory to the skills list.

Add `"./skills/config/"` to the `skills` array.

### Success Criteria:

#### Automated Verification:

- [ ] `skills/config/configure/SKILL.md` exists
- [ ] `.claude-plugin/plugin.json` includes `"./skills/config/"` in skills array
- [ ] Plugin JSON is valid: `jq . .claude-plugin/plugin.json`

#### Manual Verification:

- [ ] `/accelerator:configure` shows help when no config exists
- [ ] `/accelerator:configure create` walks through config creation
- [ ] `/accelerator:configure view` shows current settings
- [ ] Created config file has correct YAML frontmatter and markdown body

---

## Phase 5: Documentation

### Overview

Update the README to document the configuration system.

### Changes Required:

#### 1. README Update

**File**: `README.md`
**Changes**: Add a "Configuration" section after "VCS Detection" (between
lines 111 and 113).

The new section should cover:
- Config file locations and precedence
- File format (YAML frontmatter + markdown body)
- Team vs personal config
- How to get started (`/accelerator:configure`)
- Brief list of configurable areas (with "coming in future versions" notes for
  Plans 2-4)
- Example config file

### Success Criteria:

#### Manual Verification:

- [ ] README has a "Configuration" section
- [ ] Example config file in README is valid YAML
- [ ] Documentation accurately describes the implemented behaviour

---

## Testing Strategy

### Unit Tests:

- `scripts/test-config.sh` covers all config reader logic
- Direct tests for `config_extract_frontmatter` and `config_extract_body`
  utility functions (sourcing `config-common.sh` and calling directly)
- Black-box tests for `config-read-value.sh`, `config-read-context.sh`, and
  `config-summary.sh`
- Hook output tests for `config-detect.sh` (valid JSON structure)
- Test edge cases: missing files, empty files, malformed/unclosed frontmatter,
  unicode, special characters in values, values containing colons, blank lines
  within YAML sections

### Integration Tests:

- Manual: create config files in a test project, invoke skills, verify context
  injection
- Manual: verify SessionStart hook detects config

### Manual Testing Steps:

1. Start Claude Code with `--plugin-dir` pointing to this plugin
2. Verify no errors when no config exists
3. Create `.claude/accelerator.md` with test settings
4. Restart session, verify config detection in SessionStart
5. Run `/accelerator:configure view` to see settings
6. Create `.claude/accelerator.local.md` with overrides
7. Verify local overrides team settings
8. Create a config with unclosed frontmatter, verify warning in SessionStart

## Deferred to Future Plans

The following items were originally in scope for this plan but have been
deferred. Each future plan that adds configuration keys **must** include these
updates as part of its own scope:

### Plans 2-4 (Agent Overrides, Review Customisation, Template/Path Overrides)

Each plan that adds configuration keys must:

1. **Update the configure skill's `help` action** to document the new keys
   under the "Structured Settings" section. The current help text has a
   placeholder noting that settings will be added in future versions — replace
   it with actual key documentation as keys are added.

2. **Update the configure skill's `create` action** to prompt for the new
   structured settings during config file creation, in addition to the
   project context gathering.

3. **Add a `scripts/config-dump.sh` script** (or equivalent) when the number
   of config keys warrants it — a tool that outputs all configured keys with
   their effective values and source attribution (team vs local). Consider
   this when 5+ keys exist.

4. **Consider a sentinel value for unsetting team config** (e.g., `~` or
   `null`) if the override semantics prove limiting — i.e., when a team config
   sets a value that individual developers need to disable locally.

## References

- Research: `meta/research/2026-03-22-skill-customisation-and-override-patterns.md`
- Plugin extraction research: `meta/research/2026-03-14-plugin-extraction.md`
- Existing YAML parsing pattern: `skills/decisions/scripts/adr-read-status.sh`
- Existing test pattern: `skills/decisions/scripts/test-adr-scripts.sh`
- Existing preprocessor usage: `skills/vcs/commit/SKILL.md:11-12`
- Existing hook pattern: `hooks/vcs-detect.sh`
- Anthropic plugin-dev settings pattern:
  [Plugin Settings SKILL.md](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/plugin-settings/SKILL.md)
