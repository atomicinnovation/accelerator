---
date: "2026-05-08T23:00:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0052-make-documents-locator-paths-config-driven.md"
status: complete
---

# 0052: Documents-Locator Config-Driven Paths — Implementation Plan

## Overview

Implement work item 0052 in five TDD phases: (1) add the `global` path key to
config infrastructure and init, (2) create `config-read-all-paths.sh`, (3) create
`skills/config/paths/SKILL.md`, (4) extend the harness to process `skills:`
frontmatter in agent definitions (new `hooks/skills-detect.sh`), and (5) update
`agents/documents-locator.md` to use the preloaded path block. Every phase writes
failing tests before implementation code.

## Current State Analysis

- `scripts/config-defaults.sh` has 15 PATH_KEYS/PATH_DEFAULTS entries; `global`
  is absent (lines 27–61).
- `config-read-path.sh` is fully generic — adding `global` to config-defaults.sh
  is sufficient; no logic changes needed (lines 27–42).
- `skills/config/init/scripts/init.sh` maintains its own parallel `DIR_KEYS`/
  `DIR_DEFAULTS` arrays at lines 18–29 (12 entries); `global` must be added here
  too (known divergence tracked in config-defaults.sh:14–17).
- `skills/config/init/SKILL.md` has 12 bang lines (lines 20–31), `<!-- DIR_COUNT:12 -->`
  (line 40), "12 meta/ directories" prose (line 47), and a 12-entry summary block
  (lines 59–72). All four must update to 13 for `global`.
- `agents/documents-locator.md` embeds hardcoded `meta/` paths at lines 15–21
  (core responsibilities), 49–59 (ASCII tree), and 75–100 (example output).
  Frontmatter has no `skills:` key (lines 1–5).
- No hook in `hooks/` currently reads the agent definition name at session start;
  the SessionStart hooks (`vcs-detect.sh`, `config-detect.sh`) do not read stdin.
  Whether Claude Code sends agent identity in the SessionStart event payload is
  an open question requiring a discovery spike.
- `scripts/config-read-all-paths.sh` does not exist.
- `skills/config/paths/SKILL.md` does not exist.
- `hooks/skills-detect.sh` does not exist.

## Desired End State

After this plan, invoking `documents-locator` in a project with path overrides in
`.accelerator/config.md` causes the agent to search the configured paths rather
than hardcoded `meta/` defaults. The harness extension is agent-agnostic: any
agent definition may gain `skills: [paths]` in its frontmatter and the skill
content will be injected without editing the agent body or the path skill.

### Key Discoveries

- `config-defaults.sh:27–43` — PATH_KEYS array; append `"paths.global"` before
  the closing `)` on line 43.
- `config-defaults.sh:45–61` — PATH_DEFAULTS array; append `"meta/global"` before
  the closing `)` on line 61.
- `skills/config/init/SKILL.md:40` — `<!-- DIR_COUNT:12 -->` must become 13.
  The DIR_COUNT invariant test at `test-config.sh:4510–4514` validates this.
- `hooks/hooks.json` uses `${CLAUDE_PLUGIN_ROOT}` path variable and
  `"matcher": ""` with `"type": "command"` for SessionStart hooks — the new
  `skills-detect.sh` follows the same pattern.
- `config_parse_array()` in `config-common.sh:108–121` parses YAML inline arrays
  (`[a, b, c]`) into one element per line — reuse this in `skills-detect.sh` to
  parse `skills:` frontmatter.
- `config_extract_frontmatter()` in `config-common.sh:73–85` and
  `config_extract_body()` in `config-common.sh:90–100` are the correct primitives
  for processing SKILL.md files in the hook.
- `test-config.sh:2441–2453` — PATH_KEYS and PATH_DEFAULTS snapshot tests hardcode
  both the array length (`15`) and the exact space-joined content string. Both must
  be updated in Phase 1 to reflect the 16-entry arrays that include
  `"paths.global"`/`"meta/global"` — without this, Phase 1's "full suite green"
  criterion is unachievable.
- `test-config.sh:3737–3745` — the configure-skill exclusion test checks that
  `config/configure/SKILL.md` does NOT contain `config-read-skill-context.sh` or
  `config-read-skill-instructions.sh`. `config/paths/SKILL.md` needs a parallel
  exclusion test.
- The preprocessor count tests at `test-config.sh:1085–1091` expect EXACTLY 31
  skills to have `config-read-skill-context.sh`. The new `paths` skill is a
  preload-only skill (not user-invokable), so it is exempt — the count stays 31.

## What We're NOT Doing

- Not modifying `config-read-path.sh` logic — the loop is already generic.
- Not changing `config-read-value.sh` — called by the new script as a subprocess.
- Not adding `design_inventories` or `design_gaps` to `config-read-all-paths.sh`
  output — those 2 keys are excluded from the document-discovery subset.
- Not adding `tmp`, `templates`, or `integrations` to the document-discovery
  output — not document paths.
- Not removing the directory tree diagram from `documents-locator.md` entirely —
  replacing it with prose that defers to the preloaded block.
- Not surveying agents beyond `agents/documents-locator.md` — survey is complete
  (work item §Technical Notes; no other agent has `meta/` paths).
- Not using the "self-resolving via Grep" approach described in the old notes doc.
- Not targeting skill injection by agent identity (unless the discovery spike
  confirms Claude Code sends agent name in SessionStart event data) — unconditional
  injection of all agents' skills is the fallback for this scope. This is treated as
  temporary architectural debt: a follow-on work item should scope injection to the
  active agent once the hook event payload is better understood.

## Implementation Approach

Strict TDD throughout: each phase adds failing tests to `scripts/test-config.sh`
(or a new hook test file) before adding implementation code, then runs
`bash scripts/test-config.sh` to confirm green. Phases 1–3 are sequential
(each builds on the previous). Phase 4 (harness) is independent of Phases 1–3
and can be worked in parallel, but Phase 5 (agent update) depends on Phases 3
and 4 being complete.

---

## Phase 1: Add `global` Path Key

### Overview

Adds `paths.global → meta/global` to the central config vocabulary, init script,
and init skill. After this phase, `config-read-path.sh global` resolves correctly
and the init process creates `meta/global/`.

### Changes Required

#### 1. `scripts/test-config.sh` — add failing tests first

**File**: `scripts/test-config.sh`  
**Where**: After line 2854 (after the `design_gaps` no-default test), before the
`templates` test block.

```bash
echo "Test: global key → meta/global with no \$2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" global)
assert_eq "global default" "meta/global" "$OUTPUT"

echo "Test: global key returns config override when set"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  global: custom/global
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" global)
assert_eq "global config override" "custom/global" "$OUTPUT"

echo "Test: global key returns config.local.md override (last-writer-wins)"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  global: custom/global
---
FIXTURE
cat > "$REPO/.accelerator/config.local.md" << 'FIXTURE'
---
paths:
  global: local/override
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" global)
assert_eq "global local override" "local/override" "$OUTPUT"
```

Run `bash scripts/test-config.sh 2>&1 | grep -E "(PASS|FAIL|global)"` — expect 3
FAIL (not yet implemented).

Also update the PATH_KEYS and PATH_DEFAULTS snapshot tests at `test-config.sh:2441–2453`
to expect 16-entry arrays ending with `paths.global` / `meta/global`. These tests will
fail as soon as `config-defaults.sh` is modified; update them **before** running the suite
or as part of the same edit pass. The expected value strings to update are
`EXPECTED_PATH_KEYS` (space-joined, append `paths.global`) and `EXPECTED_PATH_DEFAULTS`
(space-joined, append `meta/global`), and both `assert_eq "... length" "15"` calls
should become `"16"`.

#### 2. `scripts/config-defaults.sh` — add `global` to arrays

**File**: `scripts/config-defaults.sh`

Append `"paths.global"` to `PATH_KEYS` before line 43's `)`:
```bash
  "paths.global"
)
```

Append `"meta/global"` to `PATH_DEFAULTS` before line 61's `)`:
```bash
  "meta/global"
)
```

#### 3. `skills/config/init/scripts/init.sh` — add `global` to DIR_KEYS/DIR_DEFAULTS

**File**: `skills/config/init/scripts/init.sh`

Add `global` to `DIR_KEYS` (line 18–23 array, after `design_gaps`):
```bash
DIR_KEYS=(
  plans research decisions prs validations
  review_plans review_prs review_work
  work notes
  design_inventories design_gaps
  global
)
```

Add `meta/global` to `DIR_DEFAULTS` (lines 24–29 array, after `meta/design-gaps`):
```bash
DIR_DEFAULTS=(
  meta/plans meta/research meta/decisions meta/prs meta/validations
  meta/reviews/plans meta/reviews/prs meta/reviews/work
  meta/work meta/notes
  meta/design-inventories meta/design-gaps
  meta/global
)
```

Also update the inline count comment on line 17 from `(12 items)` to `(13 items)`:
```bash
# Step 1: project-content directories under meta/ (13 items)
```

#### 4. `skills/config/init/SKILL.md` — add bang line, update DIR_COUNT and prose

**File**: `skills/config/init/SKILL.md`

After line 31 (the `design_gaps` bang line), insert:
```
**Global directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh global`
```

Update line 40:
```
<!-- DIR_COUNT:13 -->
```

Update line 47 prose:
```
The script creates 13 `meta/` directories with `.gitkeep` files, …
```

Add a `global` directory entry in the summary block (after `design_gaps` entry,
before the blank line before `Accelerator scaffold:`):
```
  ✓ {global directory} (created | already exists)
```

### Success Criteria

#### Automated Verification

- [x] New `global` tests pass: `bash scripts/test-config.sh 2>&1 | grep -E "global"`
  → all three "global default", "global config override", "global local override" show PASS
- [x] PATH_KEYS/PATH_DEFAULTS snapshot tests pass: `bash scripts/test-config.sh 2>&1 | grep -E "PATH_KEYS|PATH_DEFAULTS"`
  → PASS (length=16, content includes `paths.global` / `meta/global`)
- [x] DIR_COUNT invariant still passes: `bash scripts/test-config.sh 2>&1 | grep "directory count"`
  → PASS (expected=13, actual=13)
- [x] Full test suite green: `bash scripts/test-config.sh` exits 0

#### Manual Verification

- [x] `bash scripts/config-read-path.sh global` → `meta/global`
- [x] `bash scripts/config-read-path.sh global` in a project with
  `paths: {global: custom/global}` → `custom/global`

---

## Phase 2: Create `scripts/config-read-all-paths.sh`

### Overview

New script that emits all 11 document-discovery path keys as a labelled Markdown
block. Depends on Phase 1 (needs `global` in config-defaults.sh).

### Changes Required

#### 1. `scripts/test-config.sh` — add failing tests first

**File**: `scripts/test-config.sh`  
**Where**: After line 2913 (end of the `config-read-path.sh` section), before the
`config-read-template.sh` section.

```bash
# ============================================================
echo "=== config-read-all-paths.sh ==="
echo ""

READ_ALL_PATHS="$SCRIPT_DIR/config-read-all-paths.sh"

echo "Test: outputs ## Configured Paths header"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_ALL_PATHS")
assert_contains "has Configured Paths header" "$OUTPUT" "## Configured Paths"

echo "Test: all 11 document-discovery keys present with defaults"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_ALL_PATHS")
for key_default in \
  "plans: meta/plans" \
  "research: meta/research" \
  "decisions: meta/decisions" \
  "prs: meta/prs" \
  "validations: meta/validations" \
  "review_plans: meta/reviews/plans" \
  "review_prs: meta/reviews/prs" \
  "review_work: meta/reviews/work" \
  "work: meta/work" \
  "notes: meta/notes" \
  "global: meta/global"; do
  assert_contains "default for ${key_default%:*}" "$OUTPUT" "- ${key_default}"
done

echo "Test: excluded keys not in output"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_ALL_PATHS")
for excl in tmp templates integrations design_inventories design_gaps; do
  assert_not_contains "excluded key ${excl} absent" "$OUTPUT" "- ${excl}:"
done

echo "Test: config override reflected in output"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  work: docs/work-items
  global: shared/global
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_ALL_PATHS")
assert_contains "work override reflected" "$OUTPUT" "- work: docs/work-items"
assert_contains "global override reflected" "$OUTPUT" "- global: shared/global"
assert_contains "unset key still defaults" "$OUTPUT" "- plans: meta/plans"

echo ""
```

Run — expect all FAIL (script doesn't exist yet).

#### 2. `scripts/config-read-all-paths.sh` — new script

**File**: `scripts/config-read-all-paths.sh` (new)

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Source config-common.sh to load PATH_KEYS and PATH_DEFAULTS from config-defaults.sh.
# Note: each config-read-value.sh subprocess call re-triggers VCS detection in its own
# process — the parent sourcing does not eliminate that cost (11 VCS detections total).
# shellcheck source=config-common.sh
source "$SCRIPT_DIR/config-common.sh"

# Non-document keys excluded from the document-discovery output.
# All PATH_KEYS not in this exclusion list are emitted automatically, so new document
# path keys added to config-defaults.sh appear here without editing this script.
EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)

_is_excluded() {
  local key="$1"
  for excl in "${EXCLUDED_KEYS[@]}"; do
    [ "$key" = "$excl" ] && return 0
  done
  return 1
}

echo "## Configured Paths"
echo ""
for i in "${!PATH_KEYS[@]}"; do
  full_key="${PATH_KEYS[$i]}"   # e.g. paths.global
  key="${full_key#paths.}"      # strip prefix → global
  _is_excluded "$key" && continue
  default="${PATH_DEFAULTS[$i]}"
  value=$("$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}")
  echo "- ${key}: ${value}"
done
```

Make executable: `chmod +x scripts/config-read-all-paths.sh`

### Success Criteria

#### Automated Verification

- [x] New test section fully passes: `bash scripts/test-config.sh 2>&1 | grep -A100 "config-read-all-paths.sh"`
  → all PASS
- [x] Full test suite green: `bash scripts/test-config.sh` exits 0

#### Manual Verification

- [x] `bash scripts/config-read-all-paths.sh` in a default project outputs 11 lines
  + header
- [x] `bash scripts/config-read-all-paths.sh` in a project with one override
  reflects that override; all others show defaults

---

## Phase 3: Create `skills/config/paths/SKILL.md`

### Overview

New preload-only skill that calls `config-read-all-paths.sh` via a bang command.
Exempt from the `config-read-skill-context.sh` / `config-read-skill-instructions.sh`
preprocessor requirement (it is not user-invokable). The preprocessor count test
expecting exactly 31 skills must continue to pass.

### Changes Required

#### 1. `scripts/test-config.sh` — add failing structural tests first

**File**: `scripts/test-config.sh`  
**Where**: After the `init SKILL.md directory count invariant` section (after line
4514), before `design templates: auto-discovery`.

```bash
# ============================================================
echo "=== skills/config/paths/SKILL.md structural tests ==="
echo ""

PATHS_SKILL="$PLUGIN_ROOT/skills/config/paths/SKILL.md"

echo "Test: skills/config/paths/SKILL.md exists"
assert_file_exists "paths skill exists" "$PATHS_SKILL"

echo "Test: paths skill contains bang call to config-read-all-paths.sh"
if grep -q 'config-read-all-paths\.sh' "$PATHS_SKILL"; then
  echo "  PASS: bang call to config-read-all-paths.sh present"
  PASS=$((PASS + 1))
else
  echo "  FAIL: bang call to config-read-all-paths.sh missing"
  FAIL=$((FAIL + 1))
fi

echo "Test: paths skill name frontmatter is 'paths'"
FM_NAME=$(config_extract_frontmatter "$PATHS_SKILL" | awk '/^name:/{print $2; exit}')
assert_eq "paths skill name" "paths" "$FM_NAME"

echo "Test: paths skill does NOT contain config-read-skill-context.sh"
if ! grep -q 'config-read-skill-context\.sh' "$PATHS_SKILL"; then
  echo "  PASS: skill-context preprocessor absent (preload-only skill)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: skill-context preprocessor present — paths skill must be exempt"
  FAIL=$((FAIL + 1))
fi

echo "Test: paths skill does NOT contain config-read-skill-instructions.sh"
if ! grep -q 'config-read-skill-instructions\.sh' "$PATHS_SKILL"; then
  echo "  PASS: skill-instructions preprocessor absent (preload-only skill)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: skill-instructions preprocessor present — paths skill must be exempt"
  FAIL=$((FAIL + 1))
fi

echo "Test: paths skill does NOT contain disable-model-invocation: true"
if ! grep -q 'disable-model-invocation' "$PATHS_SKILL"; then
  echo "  PASS: disable-model-invocation absent"
  PASS=$((PASS + 1))
else
  echo "  FAIL: disable-model-invocation present — harness preload pipeline skips such skills"
  FAIL=$((FAIL + 1))
fi

echo "Test: paths skill has user-invocable: false"
if grep -q 'user-invocable: false' "$PATHS_SKILL"; then
  echo "  PASS: user-invocable: false present"
  PASS=$((PASS + 1))
else
  echo "  FAIL: user-invocable: false missing — preload-only skills must signal non-invocable"
  FAIL=$((FAIL + 1))
fi

echo ""
```

Run — expect FAIL on "exists" (rest cascade once file is created).

#### 2. `skills/config/paths/SKILL.md` — new skill

**Directory**: `skills/config/paths/` (new)  
**File**: `skills/config/paths/SKILL.md` (new)

```markdown
---
name: paths
description: Resolves all configured document-discovery paths for the current
  project. Preloaded by agent definitions that need config-driven directory
  locations; not intended for direct user invocation.
user-invocable: false
---

# Configured Paths

The following paths are resolved from the project's Accelerator configuration.
When this skill is preloaded into an agent context, the agent should treat these
values as the authoritative directory locations for all document searches,
overriding any hardcoded defaults.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-all-paths.sh`

If a path key is not listed above, use the plugin default for that key.
```

### Success Criteria

#### Automated Verification

- [x] New structural tests pass: `bash scripts/test-config.sh 2>&1 | grep -A30 "skills/config/paths"`
  → all PASS
- [x] Preprocessor count tests unchanged — still 31: `bash scripts/test-config.sh 2>&1 | grep "31 skills"`
  → PASS
- [x] Full test suite green: `bash scripts/test-config.sh` exits 0

---

## Phase 4: Extend Harness for `skills:` Frontmatter in Agent Definitions

### Overview

New `hooks/skills-detect.sh` fires at SessionStart. It scans all agent definitions
in `${CLAUDE_PLUGIN_ROOT}/agents/*.md`, collects `skills:` frontmatter entries,
locates each named skill, processes its bang lines (executing shell commands and
substituting their output), and outputs the combined result as `additionalContext`.

This phase has a **discovery step** before implementation: confirm whether Claude
Code sends any event data (specifically agent identity) on stdin to SessionStart
hooks. If it does, targeted injection (only injecting skills for the active agent)
becomes possible. If it doesn't, unconditional injection of all agents' skills is
the correct fallback.

### Discovery Step — Verify SessionStart Hook Stdin

Before writing the production hook, add a temporary diagnostic script to verify
what data (if any) Claude Code sends to SessionStart hooks:

```bash
#!/usr/bin/env bash
# hooks/tmp-stdin-spy.sh — TEMPORARY, delete after spike
# IMPORTANT: remove this file and its hooks.json entry before committing Phase 4.
SPY_LOG=$(mktemp /tmp/claude-sessionstart-spy.XXXXXX)
STDIN_DATA=$(timeout 2 cat 2>/dev/null || true)
if [ -n "$STDIN_DATA" ]; then
  # Log to a private temp file (hook stdout must be valid JSON or empty)
  echo "$STDIN_DATA" >> "$SPY_LOG"
  echo "stdin spy wrote to: $SPY_LOG" >&2
fi
# Exit cleanly — output nothing so Claude Code ignores this hook
exit 0
```

Register it temporarily in `hooks/hooks.json` as a SessionStart hook, trigger a
documents-locator agent session, then examine the log file path printed to stderr.

**Also verify during this spike** that Claude Code does not reject agent definitions
with unknown frontmatter keys. Add `skills: []` to a minimal test agent definition
and confirm it loads and invokes without error. Gate Phase 5 on this verification —
if Claude Code's agent parser is strict, the `skills:` key mechanism must use a
supported frontmatter field or a different injection approach.

**If agent identity is present in stdin**: implement targeted injection (only
inject skills for the agent named in the event payload). Update the plan's
implementation in this phase accordingly.

**If stdin is empty** (expected based on existing hooks): implement unconditional
injection as described below. Remove the spy script and its hooks.json entry.

### Changes Required (Unconditional Injection Path)

#### 1. `hooks/test-skills-detect.sh` — new test file with failing tests first

**File**: `hooks/test-skills-detect.sh` (new)

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK="$SCRIPT_DIR/skills-detect.sh"

source "$PLUGIN_ROOT/scripts/test-helpers.sh"

TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_fake_plugin() {
  # Creates a minimal plugin root with agents/ and skills/ for testing
  local dir
  dir=$(mktemp -d -p "$TMPDIR_BASE")
  mkdir -p "$dir/agents" "$dir/scripts" "$dir/hooks"
  # Copy real scripts the hook relies on
  cp "$PLUGIN_ROOT/scripts/config-common.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-defaults.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/vcs-common.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-read-value.sh" "$dir/scripts/"
  cp "$PLUGIN_ROOT/scripts/config-read-path.sh" "$dir/scripts/"
  echo "$dir"
}

echo "=== hooks/skills-detect.sh ==="
echo ""

echo "Test: no agents with skills: frontmatter → no output"
FAKE=$(setup_fake_plugin)
cat > "$FAKE/agents/example.md" << 'EOF'
---
name: example
description: An example agent with no skills.
tools: Grep
---
Example agent body.
EOF
OUTPUT=$(CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_eq "no skills frontmatter → empty output" "" "$OUTPUT"

setup_fake_skill() {
  # Copies the real paths skill and all-paths script into a fake plugin
  local fake_root="$1"
  mkdir -p "$fake_root/skills/config/paths"
  cp "$PLUGIN_ROOT/skills/config/paths/SKILL.md" "$fake_root/skills/config/paths/"
  cp "$PLUGIN_ROOT/scripts/config-read-all-paths.sh" "$fake_root/scripts/"
}

echo "Test: agent with skills: [paths] → output contains ## Configured Paths"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_contains "paths skill injected" "$OUTPUT" "Configured Paths"

echo "Test: output is valid JSON with additionalContext key"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "valid JSON with additionalContext" "$CONTEXT" "Configured Paths"

echo "Test: config override flows through to additionalContext"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/doc-locator.md" << 'EOF'
---
name: doc-locator
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git" "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  work: custom/work-items
---
FIXTURE
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "config override in context" "$CONTEXT" "work: custom/work-items"
assert_contains "default still present" "$CONTEXT" "plans: meta/plans"

echo "Test: two agents sharing the same skill → skill content accumulated once per occurrence"
FAKE=$(setup_fake_plugin)
setup_fake_skill "$FAKE"
cat > "$FAKE/agents/agent-a.md" << 'EOF'
---
name: agent-a
skills: [paths]
tools: Grep
---
Body.
EOF
cat > "$FAKE/agents/agent-b.md" << 'EOF'
---
name: agent-b
skills: [paths]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || true)
assert_contains "multi-agent: context present" "$CONTEXT" "Configured Paths"

echo "Test: unknown skill name → silently skipped (no crash, no output)"
FAKE=$(setup_fake_plugin)
cat > "$FAKE/agents/has-missing-skill.md" << 'EOF'
---
name: has-missing-skill
skills: [nonexistent]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
EXIT_CODE=0
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null) || EXIT_CODE=$?
assert_eq "missing skill → exit 0" "0" "$EXIT_CODE"
assert_eq "missing skill → no output" "" "$OUTPUT"

echo "Test: bang line outside \$PLUGIN_ROOT/scripts/ → silently skipped (allowlist rejection)"
FAKE=$(setup_fake_plugin)
mkdir -p "$FAKE/skills/config/malicious"
cat > "$FAKE/skills/config/malicious/SKILL.md" << 'EOF'
---
name: malicious
user-invocable: false
---
## Section
!`/bin/sh -c 'echo PWNED'`
EOF
cat > "$FAKE/agents/has-malicious-skill.md" << 'EOF'
---
name: has-malicious-skill
skills: [malicious]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
CONTEXT=$(echo "$OUTPUT" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null || echo "")
if echo "$CONTEXT" | grep -q "PWNED"; then
  echo "  FAIL: allowlist rejected — bang output outside scripts/ was executed"
  FAIL=$((FAIL + 1))
else
  echo "  PASS: bang line outside scripts/ was silently skipped"
  PASS=$((PASS + 1))
fi

echo "Test: skill with disable-model-invocation: true → skipped (not injected)"
FAKE=$(setup_fake_plugin)
mkdir -p "$FAKE/skills/config/disabled"
cat > "$FAKE/skills/config/disabled/SKILL.md" << 'EOF'
---
name: disabled
disable-model-invocation: true
---
## Should Not Appear
This content must not appear in additionalContext.
EOF
cat > "$FAKE/agents/has-disabled-skill.md" << 'EOF'
---
name: has-disabled-skill
skills: [disabled]
tools: Grep
---
Body.
EOF
REPO=$(mktemp -d -p "$TMPDIR_BASE")
mkdir -p "$REPO/.git"
OUTPUT=$(cd "$REPO" && CLAUDE_PLUGIN_ROOT="$FAKE" bash "$HOOK" 2>/dev/null)
assert_eq "disabled skill → no output" "" "$OUTPUT"

echo ""
test_summary
```

Run — expect FAIL on the first test (hook doesn't exist yet).

#### 2. `hooks/skills-detect.sh` — new hook

**File**: `hooks/skills-detect.sh` (new)

```bash
#!/usr/bin/env bash

# Check for jq dependency (matching config-detect.sh pattern)
if ! command -v jq &>/dev/null; then
  echo '{"systemMessage":"WARNING: jq not installed. Accelerator skills-detect hook could not run."}'
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source config-common.sh for config_extract_frontmatter, config_extract_body,
# and config_parse_array.
# shellcheck source=../scripts/config-common.sh
source "$PLUGIN_ROOT/scripts/config-common.sh"

# Process bang lines in a skill body: replace !`cmd` lines with command output.
# Only executes commands whose canonicalized path starts with $PLUGIN_ROOT/scripts/
# (allowlist). Commands outside this prefix are skipped silently.
# Direct execution (not bash -c) avoids shell metacharacter injection and
# handles spaces in PLUGIN_ROOT correctly.
_process_bang_lines() {
  local skill_file="$1"
  local safe_prefix
  safe_prefix="$(cd "$PLUGIN_ROOT/scripts" && pwd)"  # canonical form

  config_extract_body "$skill_file" | while IFS= read -r line; do
    if [[ "$line" =~ ^'!`'(.+)'`'$ ]]; then
      local cmd="${BASH_REMATCH[1]}"
      # Expand ${CLAUDE_PLUGIN_ROOT} and ${PLUGIN_ROOT} to the trustworthy PLUGIN_ROOT
      # derived from BASH_SOURCE, not the ambient environment variable.
      local resolved="${cmd/\$\{CLAUDE_PLUGIN_ROOT\}/$PLUGIN_ROOT}"
      resolved="${resolved/\$\{PLUGIN_ROOT\}/$PLUGIN_ROOT}"
      # Extract the command path (first whitespace-delimited token) and
      # canonicalize it so that ../traversal cannot bypass the prefix check.
      local cmd_path="${resolved%% *}"
      local canonical_path
      canonical_path=$(realpath --canonicalize-missing "$cmd_path" 2>/dev/null) || continue
      # Enforce allowlist: only execute scripts whose canonical path is
      # under $PLUGIN_ROOT/scripts/. Direct execution avoids shell parsing
      # entirely — no metacharacter injection risk.
      if [[ "$canonical_path" == "$safe_prefix/"* ]]; then
        local output
        output=$("$resolved" 2>/dev/null) && printf '%s\n' "$output" || true
      fi
    else
      printf '%s\n' "$line"
    fi
  done
}

# Find a skill file by its `name:` frontmatter field.
# Excludes node_modules and matches only within the first 10 lines (frontmatter).
# Validates skill_name to [a-zA-Z0-9_-]+ before use in patterns.
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

# Collect all skill content from agents with `skills:` frontmatter
COMBINED=""
for agent_file in "$PLUGIN_ROOT/agents/"*.md; do
  [ -f "$agent_file" ] || continue

  # Extract skills: value from frontmatter (inline-array syntax only: skills: [a, b])
  skills_raw=$(config_extract_frontmatter "$agent_file" 2>/dev/null \
    | awk '/^skills:/{$1=""; print; exit}' | sed 's/^[[:space:]]*//' || true)
  [ -z "$skills_raw" ] && continue

  # Parse inline array [a, b, c] into one name per line
  while IFS= read -r skill_name; do
    [ -z "$skill_name" ] && continue

    skill_file=$(_find_skill_by_name "$skill_name")
    [ -z "$skill_file" ] && continue

    # Skip skills that set disable-model-invocation: true
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

# Cap injected content to avoid filling the session context window.
if [ ${#COMBINED} -gt 65536 ]; then
  COMBINED="[skills-detect: combined skill output exceeded 64 KB and was truncated]"$'\n'
fi

jq -n --arg context "$COMBINED" '{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": $context
  }
}'
```

Make executable: `chmod +x hooks/skills-detect.sh`

#### 3. `hooks/hooks.json` — register new hook

**File**: `hooks/hooks.json`

Append the new `skills-detect.sh` entry as the **fourth (final)** entry in the
`SessionStart` array, after the existing `migrate-discoverability.sh` entry
(which currently follows `config-detect.sh`):

```json
{
  "matcher": "",
  "hooks": [
    {
      "type": "command",
      "command": "${CLAUDE_PLUGIN_ROOT}/hooks/skills-detect.sh"
    }
  ]
}
```

### Success Criteria

#### Automated Verification

- [x] Spy script removed: `hooks/tmp-stdin-spy.sh` does not exist and
  `hooks/hooks.json` contains no reference to it — confirm before registering the
  production hook
- [x] Agent frontmatter tolerance confirmed: adding `skills: []` to a minimal test
  agent did not break its invocation — confirm before Phase 5 proceeds
- [x] Hook test suite passes: `bash hooks/test-skills-detect.sh` exits 0
- [x] Full test suite unaffected: `bash scripts/test-config.sh` still exits 0
- [x] `bash hooks/skills-detect.sh` in a project where no agent has `skills:`
  exits 0 with no output
- [x] `bash hooks/skills-detect.sh` in the accelerator repo (after Phase 5 adds
  `skills: [paths]` to documents-locator) outputs valid JSON with `additionalContext`
  containing `## Configured Paths`

#### Manual Verification

- [ ] Invoke a `documents-locator` subagent session and confirm the `## Configured
  Paths` block appears in the session context (visible in verbose mode or hook
  output)
- [ ] Invoke a second agent (e.g. `codebase-locator`) and confirm it does NOT get
  the paths block (since it has no `skills:` frontmatter)
- [x] Confirm the discovery spike result: stdin is empty for SessionStart hooks;
  unconditional injection implemented as fallback (architectural debt noted)

---

## Phase 5: Update `agents/documents-locator.md`

### Overview

Adds `skills: [paths]` to the agent's frontmatter and replaces all hardcoded
`meta/` path references with instructions to use the preloaded path block.
Depends on Phases 3 and 4 being complete.

### Changes Required

#### 1. `agents/documents-locator.md` — frontmatter and body

**File**: `agents/documents-locator.md`

**Frontmatter** (lines 1–5): add `skills: [paths]`:
```yaml
---
name: documents-locator
description: Discovers relevant documents in meta/ directory …
tools: Grep, Glob, LS
skills: [paths]
---
```

**Core Responsibilities** (lines 11–22): replace the hardcoded directory list
with a reference to the preloaded path block. Replace:
```markdown
1. **Search meta/ directory structure**

- Check meta/research/ for research on specific work items
- Check meta/plans/ for implementation plans for specific work items
- Check meta/decisions/ for documents about architectural decision for the
  codebase
- Check meta/reviews/ for review artifacts (plan reviews and PR reviews)
- Check meta/validations/ for plan validation reports
- Check meta/global/ for cross-repo information
```
With:
```markdown
1. **Search the configured directory structure**

Use the paths from the **Configured Paths** block injected into your context
(provided by the preloaded `paths` skill). If a path key is not present in
the block, fall back to the plugin default for that key:
- `research` → `meta/research/`
- `plans` → `meta/plans/`
- `decisions` → `meta/decisions/`
- `reviews` (review_plans, review_prs, review_work) → `meta/reviews/`
- `validations` → `meta/validations/`
- `global` → `meta/global/`
- `work` → `meta/work/`
- `notes` → `meta/notes/`
- `prs` → `meta/prs/`
```

**Directory Structure diagram** (lines 46–59): replace the hardcoded ASCII tree
with a prose instruction:
```markdown
### Directory Structure

The directory layout follows the configured paths from the preloaded
**Configured Paths** block. Each key maps to a directory:
`research`, `plans`, `reviews`, `validations`, `decisions`, `work`,
`prs`, `notes`, `global`. Use the resolved values from the block —
do not assume default `meta/` prefixes if overrides are configured.
```

**Example output block** (lines 67–103): replace the hardcoded example paths with
placeholder-based versions. Replace the entire example block:
```markdown
## Output Format

Structure your findings like this:

```
## Documents about [Topic]

### Work Items
- `{work}/0001-implement-rate-limiting.md` - Implement rate limiting for API

### Research Documents
- `{research}/2024-01-15_rate_limiting_approaches.md` - Research on different rate limiting strategies

### Implementation Plans
- `{plans}/api-rate-limiting.md` - Detailed implementation plan for rate limits

### Related Discussions
- `{notes}/meeting-2024-01-10.md` - Team discussion about rate limiting
- `{decisions}/rate-limit-values.md` - Decision on rate limit thresholds

### Reviews
- `{review_plans}/2026-03-22-plan-review.md` - Review (verdict: REVISE)

### Validations
- `{validations}/2026-03-22-validation.md` - Validation result: partial

### PR Descriptions
- `{prs}/pr-456-rate-limiting.md` - PR that implemented basic rate limiting

Total: 7 relevant documents found
```

Where `{research}`, `{plans}`, etc. are the resolved paths from the Configured
Paths block.
```

**Closing reminder** (line 140): update `meta/` reference to be generic:
```markdown
Remember: You're a document finder for the configured document directories.
Help users quickly discover what historical context and documentation exists.
```

### Success Criteria

#### Automated Verification

- [x] Grep confirms no hardcoded `meta/` paths remain in instructions:
  `grep -n 'meta/' agents/documents-locator.md` should return only comments or
  the fallback-default list in Core Responsibilities (which is intentional)
- [x] Grep confirms `skills: [paths]` is in frontmatter:
  `grep 'skills:' agents/documents-locator.md` → `skills: [paths]`
- [x] Grep confirms no reference to `documents-locator` in `skills/config/paths/SKILL.md`:
  `grep -i 'documents-locator' skills/config/paths/SKILL.md` → no output
- [x] Full test suite green: `bash scripts/test-config.sh` exits 0

#### Manual Verification

- [ ] Invoke `documents-locator` agent with a project where `paths.work: custom-work`
  is configured → agent searches `custom-work/` for work items
- [ ] Invoke `documents-locator` agent with no config → agent searches `meta/work/`
  (identical behaviour to before)
- [ ] Invoke `documents-locator` agent with `paths.global: custom-global` →
  agent searches `custom-global/` rather than `meta/global/`
- [ ] Add `skills: [paths]` to a second agent definition and confirm it also
  receives the path block (agent-agnostic harness)

---

## Testing Strategy

### Automated Tests

All automated tests live in `scripts/test-config.sh` (phases 1–3) and
`hooks/test-skills-detect.sh` (phase 4). Run the full suite after each phase:

```bash
bash scripts/test-config.sh
bash hooks/test-skills-detect.sh
```

Both must exit 0 before moving to the next phase.

### Key Invariants to Preserve

- `test-config.sh:1085` — `31 skills have skill-context injection` must stay 31
  (paths skill is exempt, not added to ALL_SKILLS array)
- `test-config.sh:2441–2453` — PATH_KEYS/PATH_DEFAULTS snapshot tests must be
  updated to 16 entries in Phase 1 alongside the array changes; these tests fail
  if the arrays are extended without updating the expected values
- `test-config.sh:4510–4514` — DIR_COUNT in init SKILL.md must equal the count
  of `**X directory**:` bang lines (both updated to 13 in Phase 1)
- `test-config.sh:2883–2911` — no inline hardcoded defaults to `config-read-path.sh`
  (new `config-read-all-paths.sh` calls `config-read-value.sh` directly; no violation)

### Manual Testing Steps

1. Create a test project with `paths.work: custom/work-items` in
   `.accelerator/config.md`
2. Invoke `documents-locator` on a topic
3. Confirm it searches `custom/work-items/` not `meta/work/`
4. Remove the config override; re-invoke → must fall back to `meta/work/`
5. Add `paths.global: shared/global`; re-invoke → must search `shared/global/`

---

## Migration Notes

No migrations needed — new files only, no schema changes to existing documents.

## References

- Work item: `meta/work/0052-make-documents-locator-paths-config-driven.md`
- Research: `meta/research/2026-05-08-0052-documents-locator-config-driven-paths.md`
- Related work item: `meta/work/0030-centralise-path-defaults.md` (landed as
  commit `da6c42901`)
- Historical note: `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
- `scripts/config-defaults.sh:27–61` — PATH_KEYS/PATH_DEFAULTS arrays to modify
- `scripts/config-read-path.sh:27–42` — generic loop (no changes needed)
- `scripts/config-read-value.sh` — called by the new all-paths script
- `skills/config/init/SKILL.md:20–31,40,47` — bang lines, DIR_COUNT, prose
- `skills/config/init/scripts/init.sh:18–29` — DIR_KEYS/DIR_DEFAULTS arrays
- `hooks/config-detect.sh:17–23` — additionalContext pattern to follow
- `scripts/config-common.sh:73–100,108–121` — `config_extract_frontmatter`,
  `config_extract_body`, `config_parse_array` — reused by skills-detect.sh
- `scripts/test-config.sh:2828–2913` — no-default lookup section (Phase 1 tests)
- `scripts/test-config.sh:4505–4514` — DIR_COUNT invariant test
- `scripts/test-config.sh:3737–3745` — configure-skill exclusion test (pattern
  for Phase 3 paths-skill exclusion test)
