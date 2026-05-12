---
date: "2026-04-07T12:00:00+01:00"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Add `accelerator:` Prefix to Default Agent Names

## Overview

Default agent names emitted by the configuration scripts are bare names
(e.g., `reviewer`) instead of fully-qualified plugin-prefixed names
(e.g., `accelerator:reviewer`). This causes skills and spawn points to
reference agents without the required plugin prefix, which means the
Agent tool cannot resolve them to the correct plugin-provided agent
definitions.

## Current State Analysis

Agent names flow through two resolution paths:

1. **Bulk resolution** (`config-read-agents.sh`) — generates a markdown
   "Agent Names" section injected into 10 skill prompts. When no
   override is configured, the default is the bare key name
   (`scripts/config-read-agents.sh:108`: `val="$key"`).

2. **Inline resolution** (`config-read-agent-name.sh`) — used at 2
   spawn points (`review-pr`, `review-plan`) for deterministic
   `subagent_type` values. Falls back to the bare name
   (`scripts/config-read-agent-name.sh:25`: second arg is `"$DEFAULT"`).

Additionally, `config-dump.sh` displays effective configuration and has
its own `AGENT_DEFAULTS` array with bare names
(`scripts/config-dump.sh:143-151`).

All 10 skills that use agents also have a hardcoded fallback line with
bare names, used when the preprocessor output is absent.

### Key Discoveries:

- Agent definition `name:` fields in `agents/*.md` are correctly bare —
  the plugin framework adds the prefix automatically. No changes needed
  there.
- User-provided overrides must be passed through unchanged — users
  specify the exact `subagent_type` value they want.
- The prefix `accelerator:` will be defined as an `AGENT_PREFIX`
  constant in `config-common.sh` (no `CLAUDE_PLUGIN_NAME` env var
  exists; only `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA`).
- Template-style `{agent name}` references in skill bodies (e.g.,
  `{codebase locator agent}`) resolve against the "Agent Names" section
  output, so they will automatically pick up the prefixed values once
  the scripts are fixed. No changes needed to those references.

## Desired End State

All default agent name outputs include the `accelerator:` prefix:

- `config-read-agents.sh` outputs
  `- **reviewer agent**: accelerator:reviewer` (etc.) when no override
  is configured
- `config-read-agent-name.sh reviewer` outputs `accelerator:reviewer`
  when no override is configured
- `config-dump.sh` shows `accelerator:reviewer` as the default value
- All 10 skill fallback lines list prefixed names
- All tests pass with the new defaults

### Verification:

```bash
bash scripts/test-config.sh
```

All tests should pass with zero failures.

## What We're NOT Doing

- Changing agent definition `name:` fields in `agents/*.md` — the
  plugin framework handles prefixing
- Changing user-provided override values — overrides are passed through
  as-is
- Parameterising the prefix via env var — it's defined as an
  `AGENT_PREFIX` constant in `config-common.sh` since no plugin name env
  var exists; skill fallback lines hardcode the literal since they serve
  as a safety net when preprocessing fails
- Changing template-style `{agent name}` references in skill bodies —
  these resolve against the Agent Names section output automatically
- Changing the `respond-to-pr` skill's `{reviewer}` references — those
  are output format references, not agent spawn instructions

## Implementation Approach

Mechanical updates to defaults in 4 scripts, 10 skill files, and 1 test
file. Each change is a targeted string replacement. The phases follow
TDD ordering: tests first (to establish failing expectations), then
scripts, then skills.

## Phase 1: Test Assertions

### Overview

Update `scripts/test-config.sh` to expect `accelerator:`-prefixed
default agent names in all assertions. These tests will fail until the
scripts are updated in Phase 2.

### Changes Required:

#### 1. `config-read-agents.sh` default output test

**File**: `scripts/test-config.sh`
**Lines**: 750-757
**Change**: Update the grep patterns for default agent names.

Current:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: codebase-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase analyser agent\*\*: codebase-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase pattern finder agent\*\*: codebase-pattern-finder' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents locator agent\*\*: documents-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents analyser agent\*\*: documents-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*web search researcher agent\*\*: web-search-researcher'; then
```

New:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: accelerator:reviewer' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: accelerator:codebase-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase analyser agent\*\*: accelerator:codebase-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*codebase pattern finder agent\*\*: accelerator:codebase-pattern-finder' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents locator agent\*\*: accelerator:documents-locator' && \
   echo "$OUTPUT" | grep -q '\- \*\*documents analyser agent\*\*: accelerator:documents-analyser' && \
   echo "$OUTPUT" | grep -q '\- \*\*web search researcher agent\*\*: accelerator:web-search-researcher'; then
```

#### 2. `config-read-agents.sh` partial-overrides test

**File**: `scripts/test-config.sh`
**Line**: 799
**Change**: The partial-overrides test checks that non-overridden agents
show defaults. Update the default assertion for codebase-locator.

Current:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: codebase-locator'; then
```

New:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*codebase locator agent\*\*: accelerator:codebase-locator'; then
```

#### 3. `config-read-agents.sh` no-agents-section test

**File**: `scripts/test-config.sh`
**Line**: 941
**Change**: The no-agents-section test checks that all defaults are output
when config has frontmatter but no `agents` section. Update the default
assertion.

Current:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: reviewer'; then
```

New:
```bash
   echo "$OUTPUT" | grep -q '\- \*\*reviewer agent\*\*: accelerator:reviewer'; then
```

#### 4. `config-read-value.sh` default test

**File**: `scripts/test-config.sh`
**Lines**: 273-274
**Change**: This test calls `config-read-value.sh` directly with
`"reviewer"` as the fallback — this is testing the generic value reader,
not the agent-specific script. The fallback is caller-provided. **No
change needed** — the test correctly validates that
`config-read-value.sh` returns the fallback it was given.

#### 5. `config-read-agent-name.sh` default test (no config)

**File**: `scripts/test-config.sh`
**Lines**: 966-967
**Change**: Update to expect the prefixed default. This test has no
config files at all — it verifies the bare default path.

Current:
```bash
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "reviewer" "$OUTPUT"
```

New:
```bash
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "accelerator:reviewer" "$OUTPUT"
```

#### 6. `config-read-agent-name.sh` default test (override for different agent)

**File**: `scripts/test-config.sh`
**Lines**: 990-991
**Change**: Update to expect the prefixed default. This test has a
config override for `codebase-locator` but requests `reviewer`, so
reviewer still falls back to the default.

Current:
```bash
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "reviewer" "$OUTPUT"
```

New:
```bash
OUTPUT=$(cd "$REPO" && bash "$READ_AGENT_NAME" "reviewer")
assert_eq "outputs default" "accelerator:reviewer" "$OUTPUT"
```

#### 7. `config-dump.sh` agent default values test (new test)

**File**: `scripts/test-config.sh`
**Location**: At the end of the `=== config-dump.sh ===` test section (around line 1810+)
**Change**: Add a new test that verifies config-dump.sh outputs
`accelerator:`-prefixed default values for agent keys.

New test:
```bash
echo "Test: No config overrides -> config-dump shows prefixed agent defaults"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep -q 'agents\.reviewer.*accelerator:reviewer.*default' && \
   echo "$OUTPUT" | grep -q 'agents\.codebase-locator.*accelerator:codebase-locator.*default'; then
  echo "  PASS: config-dump shows prefixed agent defaults"
  PASS=$((PASS + 1))
else
  echo "  FAIL: config-dump shows prefixed agent defaults"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-config.sh` shows failures for the updated
  assertions (tests are red before Phase 2)

---

## Phase 2: Script Defaults

### Overview

Define an `AGENT_PREFIX` constant in `config-common.sh` and update the
3 scripts that produce default agent names to emit
`accelerator:`-prefixed values using that constant.

### Changes Required:

#### 1. `scripts/config-common.sh`

**File**: `scripts/config-common.sh`
**Change**: Add the `AGENT_PREFIX` constant near the top of the file
(after the existing variable declarations).

New:
```bash
AGENT_PREFIX="accelerator:"
```

#### 2. `scripts/config-read-agents.sh`

**File**: `scripts/config-read-agents.sh`
**Line**: 108
**Change**: When no override is found, prefix the bare key with
`accelerator:`.

Current:
```bash
    val="$key"
```

New:
```bash
    val="${AGENT_PREFIX}$key"
```

#### 3. `scripts/config-read-agent-name.sh`

**File**: `scripts/config-read-agent-name.sh`
**Lines**: 17, 25
**Change**: Source `config-common.sh` to access `AGENT_PREFIX`, then
update the fallback default argument to include the prefix.

Add after line 17 (`SCRIPT_DIR=...`):
```bash
source "$SCRIPT_DIR/config-common.sh"
```

Current (line 25):
```bash
"$SCRIPT_DIR/config-read-value.sh" "agents.$DEFAULT" "$DEFAULT"
```

New:
```bash
"$SCRIPT_DIR/config-read-value.sh" "agents.$DEFAULT" "${AGENT_PREFIX}$DEFAULT"
```

#### 4. `scripts/config-dump.sh`

**File**: `scripts/config-dump.sh`
**Lines**: 143-151
**Change**: Update the `AGENT_DEFAULTS` array to use prefixed names via
the `AGENT_PREFIX` constant (already available — this script sources
`config-common.sh`).

Current:
```bash
AGENT_DEFAULTS=(
  "reviewer"
  "codebase-locator"
  "codebase-analyser"
  "codebase-pattern-finder"
  "documents-locator"
  "documents-analyser"
  "web-search-researcher"
)
```

New:
```bash
AGENT_DEFAULTS=(
  "${AGENT_PREFIX}reviewer"
  "${AGENT_PREFIX}codebase-locator"
  "${AGENT_PREFIX}codebase-analyser"
  "${AGENT_PREFIX}codebase-pattern-finder"
  "${AGENT_PREFIX}documents-locator"
  "${AGENT_PREFIX}documents-analyser"
  "${AGENT_PREFIX}web-search-researcher"
)
```

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-config.sh` passes with zero failures (tests
  from Phase 1 are now green)
- [x] `bash scripts/config-read-agents.sh` (run from a repo with no
  config) outputs `accelerator:reviewer`, `accelerator:codebase-locator`,
  etc.
- [x] `bash scripts/config-read-agent-name.sh reviewer` (run from a
  repo with no config) outputs `accelerator:reviewer`

#### Manual Verification:

- [ ] When a user has an override configured (e.g.,
  `agents.reviewer: my-reviewer`), the override value is emitted
  unchanged (no double-prefixing)

---

## Phase 3: Skill Fallback Lines

### Overview

Update the hardcoded fallback defaults in all 10 SKILL.md files that
reference agents. These lines are used when the preprocessor output is
absent.

### Changes Required:

#### 1. All 10 skill files

**Files** (each has the same 3-line block):

| Skill | File | Line |
|---|---|---|
| research-codebase | `skills/research/research-codebase/SKILL.md` | 17 |
| create-plan | `skills/planning/create-plan/SKILL.md` | 16 |
| implement-plan | `skills/planning/implement-plan/SKILL.md` | 17 |
| review-plan | `skills/planning/review-plan/SKILL.md` | 17 |
| stress-test-plan | `skills/planning/stress-test-plan/SKILL.md` | 17 |
| validate-plan | `skills/planning/validate-plan/SKILL.md` | 17 |
| review-pr | `skills/github/review-pr/SKILL.md` | 17 |
| create-adr | `skills/decisions/create-adr/SKILL.md` | 18 |
| extract-adrs | `skills/decisions/extract-adrs/SKILL.md` | 18 |
| review-adr | `skills/decisions/review-adr/SKILL.md` | 19 |

**Change**: Replace the bare-name fallback block with prefixed names.

Current (identical in all 10 files):
```
If no "Agent Names" section appears above, use these defaults: reviewer,
codebase-locator, codebase-analyser, codebase-pattern-finder,
documents-locator, documents-analyser, web-search-researcher.
```

New:
```
If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.
```

### Success Criteria:

#### Automated Verification:

- [x] `grep -r 'use these defaults: reviewer,' skills/` returns no
  matches (no bare-name fallbacks remain)
- [x] `grep -r 'use these defaults:' skills/ | wc -l` returns 10
  (all fallback lines still present)

---

## Testing Strategy

### Automated Tests:

- Run `bash scripts/test-config.sh` — this is the comprehensive test
  suite covering all 3 scripts, preprocessor placement, and agent name
  resolution

### Manual Testing:

1. Run `/accelerator:configure` in a project and verify the agent names
   table shows `accelerator:`-prefixed defaults
2. Run a skill (e.g., `/accelerator:create-plan`) and verify the "Agent
   Names" section in the expanded prompt shows prefixed names
3. Configure an override (`agents.reviewer: my-reviewer`) and verify the
   override value appears as-is (no double-prefixing)

## References

- Research: `meta/research/codebase/2026-04-07-bare-agent-name-references.md`
