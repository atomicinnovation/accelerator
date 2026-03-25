# Context Injection & Agent Customisation Implementation Plan

## Overview

Add two high-value configuration features: (1) project context injection into
skills, so that skills are informed by tech-stack-specific guidance, and
(2) agent name overrides, so that users can swap agent implementations globally.
These are the most useful and most likely-to-change configuration options.

**Depends on**: Plan 1 (Configuration Infrastructure) must be complete first.

## Current State Analysis

Skills reference agents by name in two patterns:
- **Explicit parameter**: `subagent_type: "reviewer"` (used by `review-pr:258`,
  `review-plan:227`)
- **Prose reference**: "Use the **codebase-locator** agent" (used by 10+ skills)

No skill currently reads project-specific context. Every skill operates with
only its embedded instructions and the CLAUDE.md context that Claude Code
provides natively.

### Key Discoveries:

- Seven distinct agent names are referenced across 12 skills
  (`review-pr/SKILL.md:258`, `create-plan/SKILL.md:64-69,129-139`,
  `research-codebase/SKILL.md:54-68`, `stress-test-plan/SKILL.md:31-33`,
  `create-adr/SKILL.md:65-67`, `review-adr/SKILL.md:99-100`,
  `extract-adrs/SKILL.md:43,64`)
- Agent names appear inline in prose text — they can't be preprocessor-
  substituted without restructuring. The cleanest approach is an override
  instructions block at the top of each skill. For skills with explicit
  `subagent_type` parameters (`review-pr`, `review-plan`), inline
  preprocessor substitution provides deterministic override.
- The `!`command`` preprocessor outputs nothing on empty stdout — ideal for
  "inject only when configured" blocks.
- `implement-plan` and `validate-plan` use generic/unnamed sub-tasks, not named
  agents — they don't need agent override blocks.
- `describe-pr` and `respond-to-pr` don't spawn named agents either.

## Desired End State

After this plan:
1. Skills that perform research, planning, or review inject project-specific
   context from the user's config file at the top of their prompt.
2. Users can override any agent name globally in their config file.
3. When agent overrides are configured, each skill that spawns agents includes
   an override instruction block telling Claude which names to substitute.
4. The override mechanism is invisible when no overrides are configured (no
   empty sections, no "no overrides found" messages).

## What We're NOT Doing

- Per-skill agent overrides (global only for now)
- Adding new built-in agents
- Changing the agent markdown files themselves
- Supporting agent definition overrides (only name overrides — the agent's
  behaviour is defined by its markdown file, which must exist in the plugin
  or user's agents directory)
- Modifying unnamed sub-task spawning in `validate-plan` or `implement-plan`
- Adding context injection to review lenses (those are read by reviewer agents,
  not by the orchestrating skill). Note: sub-agents do not inherit the parent
  skill's injected context — this is a known limitation deferred to a future
  plan

## Implementation Approach

Three mechanisms are used to inject configuration into skills:

1. **Context block**: `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh``
   — injects project context or nothing. Added near the top of each affected
   skill.
2. **Agent override block**: `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh``
   — injects agent name mapping table or nothing. Added near the top of each
   skill that spawns named agents via prose references.
3. **Inline agent name substitution**:
   `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh <default>``
   — outputs the configured agent name or the default. Used inline within
   `subagent_type` parameters in `review-pr` and `review-plan` for
   deterministic override (avoids relying on Claude interpreting an override
   table to contradict a literal parameter value).

All scripts output structured markdown (or a single name) when configuration
exists and nothing (or the default) when it doesn't. This keeps skills clean
when no config is present.

## Phase 1: Agent Override Script

### Overview

Create the `config-read-agents.sh` script that reads agent name overrides from
config and outputs a markdown instruction block.

### Changes Required:

#### 1. Agent Override Reader

**File**: `scripts/config-read-agents.sh`
**Changes**: New file. Uses inline frontmatter parsing rather than calling
`config-read-value.sh` per key, to avoid forking 7 subprocesses (each of
which re-parses config files). Extracts frontmatter once per config file
and parses all agent keys in a single awk pass.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads agent name overrides from accelerator config files.
# Outputs a markdown instruction block listing overrides, or nothing if
# no overrides are configured.
#
# Usage: config-read-agents.sh
#
# Config format (in .claude/accelerator.md or .claude/accelerator.local.md):
#   ---
#   agents:
#     reviewer: my-custom-reviewer
#     codebase-locator: my-locator-agent
#   ---
#
# Agent config keys use the same hyphenated names as the agents themselves
# (e.g., codebase-locator, not codebase_locator).
#
# Performance: Extracts frontmatter once per config file and parses all
# agent keys in a single awk pass, rather than shelling out to
# config-read-value.sh per key (~20-30ms vs ~100-200ms).
#
# Note: The list of valid agent keys is also documented in the configure
# skill (skills/config/configure/SKILL.md). Update both when adding agents.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

# Valid agent names in display order. This is the canonical list of agents
# that can be overridden. Order determines table row order in output.
AGENT_KEYS=(
  reviewer
  codebase-locator
  codebase-analyser
  codebase-pattern-finder
  documents-locator
  documents-analyser
  web-search-researcher
)

# Check if a string matches a valid agent key.
_is_valid_key() {
  local needle="$1"
  for k in "${AGENT_KEYS[@]}"; do
    [ "$k" = "$needle" ] && return 0
  done
  return 1
}

# Build a space-delimited string of valid keys for awk to reference.
VALID_KEYS_STR="${AGENT_KEYS[*]}"

# Parse all agent overrides from config files in a single pass per file.
# Uses last-writer-wins precedence: team config is read first, local config
# second. If both define the same key, the local value wins.
#
# Stores results in a newline-delimited string of "key=value" pairs to
# avoid bash 4+ associative arrays (macOS ships bash 3.2).
OVERRIDES=""

while IFS= read -r config_file; do
  fm=$(config_extract_frontmatter "$config_file") || continue
  [ -z "$fm" ] && continue

  # Single awk pass: extract all key-value pairs from the agents section,
  # and flag unrecognised keys.
  parsed=$(echo "$fm" | awk -v valid_keys="$VALID_KEYS_STR" '
    BEGIN { split(valid_keys, vk, " "); for (i in vk) valid[vk[i]] = 1 }
    /^agents:/ { in_section = 1; next }
    in_section && /^[^ \t]/ { exit }
    in_section && /^[ \t]+[a-zA-Z]/ {
      stripped = $0
      sub(/^[ \t]+/, "", stripped)
      key = stripped
      sub(/:.*/, "", key)
      val = stripped
      sub(/^[^:]+:[ \t]*/, "", val)
      # Strip optional surrounding quotes
      if (val ~ /^".*"$/ || val ~ /^'"'"'.*'"'"'$/) {
        val = substr(val, 2, length(val) - 2)
      }
      if (key in valid) {
        print "OVERRIDE:" key "=" val
      } else {
        print "WARN:" key
      }
    }
  ')

  # Process parsed output
  while IFS= read -r line; do
    case "$line" in
      OVERRIDE:*)
        pair="${line#OVERRIDE:}"
        key="${pair%%=*}"
        val="${pair#*=}"
        # Remove any previous override for this key (last-writer-wins)
        OVERRIDES=$(printf '%s\n' "$OVERRIDES" | grep -v "^${key}=" || true)
        OVERRIDES="${OVERRIDES}"$'\n'"${key}=${val}"
        ;;
      WARN:*)
        found_key="${line#WARN:}"
        echo "Warning: unknown agent key '$found_key' in $config_file — ignoring" >&2
        ;;
    esac
  done <<< "$parsed"
done < <(config_find_files)

# Build override rows in AGENT_KEYS order (fixed display order).
OVERRIDE_ROWS=""
for key in "${AGENT_KEYS[@]}"; do
  val=$(printf '%s\n' "$OVERRIDES" | grep "^${key}=" | tail -1 | sed 's/^[^=]*=//' || true)
  if [ -n "$val" ] && [ "$val" != "$key" ]; then
    OVERRIDE_ROWS="${OVERRIDE_ROWS}| \`$key\` | \`$val\` |
"
  fi
done

# Output nothing if no overrides configured
if [ -z "$OVERRIDE_ROWS" ]; then
  exit 0
fi

# Output markdown instruction block with rows in AGENT_KEYS order
echo "## Agent Overrides"
echo ""
echo "The following agent names are overridden by project configuration."
echo "When the instructions below reference an agent by its default name,"
echo "use the configured name instead when spawning sub-agents:"
echo ""
echo "| Default Agent | Use Instead |"
echo "|---|---|"
printf '%s' "$OVERRIDE_ROWS"
echo ""
echo "This applies to all agent references in this skill, including"
echo "the \`subagent_type\` parameter when spawning agents via the"
echo "Agent/Task tool."
```

#### 2. Agent Name Reader (for inline substitution)

**File**: `scripts/config-read-agent-name.sh`
**Changes**: New file. Reads a single agent name override. Used inline in
`subagent_type` parameters for deterministic substitution (see Phase 3).
Reuses `config-read-value.sh` since it's only called once per skill
invocation — no performance concern.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a single agent name override from accelerator config files.
# Outputs the configured override or the default agent name.
#
# Usage: config-read-agent-name.sh <default-agent-name>
#
# Example: config-read-agent-name.sh reviewer
#   → outputs "my-custom-reviewer" if configured, otherwise "reviewer"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DEFAULT="${1:-}"
if [ -z "$DEFAULT" ]; then
  echo "Usage: config-read-agent-name.sh <default-agent-name>" >&2
  exit 1
fi

"$SCRIPT_DIR/config-read-value.sh" "agents.$DEFAULT" "$DEFAULT"
```

#### 3. Test Updates

**File**: `scripts/test-config.sh`
**Changes**: Add test cases for `config-read-agents.sh` and
`config-read-agent-name.sh`:

**`config-read-agents.sh` tests:**
- No config → outputs nothing
- Config with agents section → outputs override table
- Config with partial overrides → only changed agents listed
- Local overrides team for same agent key (verifies last-writer-wins
  precedence in the inline parsing logic, not inherited from
  `config-read-value.sh`)
- Non-overlapping overrides across team and local → both appear in output
- Unknown agent keys → ignored with warning to stderr
- Agent key with same value as default → not listed as override
- Table rows appear in fixed order (matching AGENT_KEYS declaration order)
- Config with frontmatter but no agents section → outputs nothing
- AGENT_KEYS list matches actual agent files (`.md`) in the plugin (use a
  separate mechanism such as grep to extract the array from the script
  source — do not source the script, as that would execute it)

**`config-read-agent-name.sh` tests:**
- No config → outputs the default agent name
- Config with override for requested agent → outputs override value
- Config with override for different agent → outputs the default
- Local overrides team for same agent key
- No argument → exits with error

### Success Criteria:

#### Automated Verification:

- [x] `scripts/config-read-agents.sh` exists and is executable
- [x] `scripts/config-read-agent-name.sh` exists and is executable
- [x] `bash scripts/test-config.sh` passes all tests including new ones

#### Manual Verification:

- [ ] With no config, `config-read-agents.sh` produces no output
- [ ] With agent overrides in config, produces a formatted markdown table
- [ ] `config-read-agent-name.sh reviewer` outputs `reviewer` (default)
- [ ] With override configured, `config-read-agent-name.sh reviewer` outputs
  the configured value

---

## Phase 2: Context Injection into Skills

### Overview

Add `!`command`` preprocessor blocks to all skills that benefit from project
context. These blocks inject the project context from the user's config file
at skill invocation time.

### Changes Required:

For each skill listed below, add a single preprocessor line after the first
heading and before the first instruction section. The exact insertion point
varies per skill.

#### Skills receiving context injection:

**1. `skills/planning/create-plan/SKILL.md`**
Insert after the `# Implementation Plan` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**2. `skills/planning/review-plan/SKILL.md`**
Insert after the `# Review Plan` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**3. `skills/planning/implement-plan/SKILL.md`**
Insert after the `# Implement Plan` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**4. `skills/planning/validate-plan/SKILL.md`**
Insert after the `# Validate Plan` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**5. `skills/planning/stress-test-plan/SKILL.md`**
Insert after the `# Stress Test Plan` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**6. `skills/research/research-codebase/SKILL.md`**
Insert after the `# Research Codebase` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**7. `skills/github/review-pr/SKILL.md`**
Insert after the `# Review PR` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**8. `skills/github/describe-pr/SKILL.md`**
Insert after the `# Generate PR Description` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**9. `skills/github/respond-to-pr/SKILL.md`**
Insert after the `# Respond to PR` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**10. `skills/decisions/create-adr/SKILL.md`**
Insert after the `# Create ADR` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**11. `skills/decisions/extract-adrs/SKILL.md`**
Insert after the `# Extract ADRs` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**12. `skills/decisions/review-adr/SKILL.md`**
Insert after the `# Review ADR` heading:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**13. `skills/vcs/commit/SKILL.md`**
Insert after the existing `!`command`` preprocessor lines:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
```

**Not receiving context injection:**
- Review lens skills (read by reviewer agents, not user-facing)
- Output format skills (structural schemas, not context-sensitive)

### Success Criteria:

#### Automated Verification:

- [x] All 13 skills listed above contain the `config-read-context.sh` preprocessor line
- [x] `grep -r 'config-read-context.sh' skills/` returns 13 matches
- [x] `bash scripts/test-config.sh` preprocessor placement tests pass (each
  skill has the line within a few lines of the first `#` heading, not
  relative to the start of the file — this decouples the test from
  frontmatter length)

#### Manual Verification:

- [ ] Invoking a skill with no config shows no "Project Context" section
- [ ] Invoking a skill with project context in config shows the context block
  at the top of the skill output
- [ ] Context appears in the skill prompt, not as a separate message

---

## Phase 3: Agent Override Injection into Skills

### Overview

Add `!`command`` preprocessor blocks to all skills that spawn named agents.
When agent overrides are configured, the block injects an override instruction
table. When no overrides exist, it outputs nothing.

### Changes Required:

For each skill below, add the `config-read-agents.sh` preprocessor line
immediately after the `config-read-context.sh` line added in Phase 2.

#### Skills receiving agent override blocks:

**1. `skills/planning/create-plan/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `codebase-locator`, `codebase-analyser`,
`codebase-pattern-finder`, `documents-locator`, `documents-analyser`

**2. `skills/planning/review-plan/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `reviewer`

Additionally, replace the literal `subagent_type: "reviewer"` at line 227
with inline preprocessor substitution:
```markdown
subagent_type: "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh reviewer`"
```
This ensures the `subagent_type` parameter is deterministically set to the
configured agent name, rather than relying on Claude interpreting the
override table to contradict a literal value.

**3. `skills/planning/stress-test-plan/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `codebase-locator`, `codebase-analyser`

**4. `skills/research/research-codebase/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `codebase-locator`, `codebase-analyser`,
`codebase-pattern-finder`, `documents-locator`, `documents-analyser`,
`web-search-researcher`

**5. `skills/github/review-pr/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `reviewer`

Additionally, replace the literal `subagent_type: "reviewer"` at line 258
with inline preprocessor substitution:
```markdown
subagent_type: "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh reviewer`"
```
Same rationale as `review-plan` above.

**6. `skills/decisions/create-adr/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `documents-locator`, `codebase-locator`

**7. `skills/decisions/extract-adrs/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `documents-locator`, `documents-analyser`

**8. `skills/decisions/review-adr/SKILL.md`**
Insert after the `config-read-context.sh` line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```
Agents referenced: `documents-locator`, `codebase-locator`

#### Skills NOT receiving agent override blocks:

- `implement-plan` — uses unnamed generic sub-tasks only
- `validate-plan` — uses unnamed generic sub-tasks only
- `describe-pr` — does not spawn agents
- `respond-to-pr` — does not spawn agents
- `commit` — does not spawn agents

### Success Criteria:

#### Automated Verification:

- [x] All 8 skills listed above contain the `config-read-agents.sh` preprocessor line
- [x] `grep -r 'config-read-agents.sh' skills/` returns 8 matches
- [x] The 5 non-agent skills (`implement-plan`, `validate-plan`, `describe-pr`,
  `respond-to-pr`, `commit`) do NOT contain `config-read-agents.sh`
- [x] `bash scripts/test-config.sh` preprocessor placement tests pass
  (`config-read-agents.sh` appears on the line after `config-read-context.sh`
  in each of the 8 skills)
- [x] `review-pr` and `review-plan` contain
  `config-read-agent-name.sh reviewer` inline in their `subagent_type` line

#### Manual Verification:

- [ ] Invoking a skill with no agent overrides shows no "Agent Overrides" section
- [ ] Invoking a skill with agent overrides shows the override table
- [ ] When overrides are present, Claude uses the overridden agent name when
  spawning agents (verify by observing the `subagent_type` in task spawning)

---

## Phase 4: Configure Skill Updates

### Overview

Update the `/accelerator:configure` skill to document the new `agents` config
section and project context.

### Changes Required:

#### 1. Configure Skill Content

**File**: `skills/config/configure/SKILL.md`
**Changes**: This fulfils the deferred obligation from Plan 1 (Configuration
Infrastructure), which required each plan that introduces config keys to update
the configure skill. Two changes are needed:

**a) Update the `help` action**: Replace the "Structured Settings" placeholder
text (which reads "settings will be added in future versions") with actual
documentation of the `agents` section keys.

Add to the help section:

```markdown
### agents

Override which agents are used when skills spawn sub-agents. Config keys
use the same hyphenated names as the agents themselves:

Available agents and their roles:

| Config Key | Default Role |
|---|---|
| `reviewer` | Reviews plans and PRs using configured lenses |
| `codebase-locator` | Finds relevant source files for a given task |
| `codebase-analyser` | Analyses implementation details of components |
| `codebase-pattern-finder` | Finds similar implementations and usage examples |
| `documents-locator` | Discovers relevant documents in meta/ directory |
| `documents-analyser` | Deep-dives on research topics in documents |
| `web-search-researcher` | Researches topics via web search |

\```yaml
---
agents:
  reviewer: my-custom-reviewer
  codebase-locator: my-locator
  codebase-analyser: my-analyser
  codebase-pattern-finder: my-pattern-finder
  documents-locator: my-doc-locator
  documents-analyser: my-doc-analyser
  web-search-researcher: my-web-researcher
---
\```

Only list agents you want to override. Unlisted agents use their defaults.
Unrecognised keys produce a warning to stderr and are ignored. Override
values can be any agent name — the plugin does not validate values since
the override may reference a user-defined agent outside the plugin.

### Project Context

The markdown body is injected into skills as project-specific guidance:

\```markdown
---
agents:
  reviewer: my-custom-reviewer
---

# Project Context

## Tech Stack
- Language: TypeScript with strict mode
- Framework: Next.js 14 with App Router
- Database: PostgreSQL via Prisma ORM
- Testing: Vitest for unit tests, Playwright for E2E

## Conventions
- All API routes use GraphQL (no REST endpoints)
- Database migrations must be backward-compatible
- Feature flags managed via LaunchDarkly

## Build & Test
- Build: `npm run build`
- Test: `npm run test`
- Lint: `npm run lint`
- Full check: `npm run check`
\```
```

**b) Update the `create` action**: When walking users through config creation,
add an optional advanced section for agent overrides. Keep the primary focus on
project context gathering (the highest-value feature). Agent overrides should be
presented as optional: "Would you also like to configure custom agent overrides?
(This is an advanced feature — most users can skip this.)"

### Success Criteria:

#### Manual Verification:

- [ ] `/accelerator:configure help` shows agents documentation with role descriptions
- [ ] `/accelerator:configure help` shows project context examples
- [ ] `/accelerator:configure create` gathers project context and optionally
  prompts for agent overrides

---

## Testing Strategy

### Unit Tests:

- `config-read-agents.sh` tests in `test-config.sh` (see Phase 1)
- `config-read-agent-name.sh` tests in `test-config.sh` (see Phase 1)
- Edge cases: empty agents section, agents section with no overrides,
  unrecognised keys warn to stderr
- AGENT_KEYS consistency: assert that every entry in the AGENT_KEYS array
  corresponds to a `.md` file in `agents/` and vice versa (extract the
  array via grep, not by sourcing the script)

### Preprocessor Placement Tests:

Automated tests in `test-config.sh` verifying correct skill injection:
- Each of the 13 context-injection skills contains `config-read-context.sh`
  within a few lines of the first `#` heading (not relative to start of
  file — decouples test from frontmatter length)
- Each of the 8 agent-override skills contains `config-read-agents.sh`
  on the line immediately following `config-read-context.sh`
- The 5 non-agent skills (`implement-plan`, `validate-plan`, `describe-pr`,
  `respond-to-pr`, `commit`) do NOT contain `config-read-agents.sh`
- `grep -r 'config-read-context.sh' skills/` returns exactly 13 matches
- `grep -r 'config-read-agents.sh' skills/` returns exactly 8 matches
- `review-pr` and `review-plan` contain `config-read-agent-name.sh` inline

### Automated Integration Tests:

In `test-config.sh`, add end-to-end tests that exercise the full config → script
→ output path:
- Create a temp project directory with `.git/` and
  `.claude/accelerator.md` containing agent overrides and a markdown body
- Run `config-read-agents.sh` from that directory and verify the output contains
  a markdown table with the correct override rows
- Run `config-read-context.sh` from that directory and verify the output contains
  the "Project Context" header and the markdown body content
- Run both scripts with no config files and verify empty output

### Manual Integration Tests:

- Create `.claude/accelerator.md` with agents overrides and project context
- Invoke `/accelerator:create-plan` and verify:
  - Project context appears at top of skill output
  - Agent override table appears
  - Claude uses overridden agent names when spawning

### Manual Testing Steps:

1. Create test config:
   ```yaml
   ---
   agents:
     codebase-locator: my-test-locator
   ---

   We use Go with GRPC and PostgreSQL.
   ```
2. Invoke `/accelerator:research-codebase "test query"`
3. Verify project context is visible in the skill prompt
4. Verify "Agent Overrides" table shows codebase-locator → my-test-locator
5. Observe that Claude attempts to use `my-test-locator` as the subagent_type

## Accepted Risks

- **Prose-reference override relies on Claude interpretation**: For the 6
  skills that reference agents in prose (not `subagent_type`), the override
  table instructs Claude to substitute names. This is non-deterministic but
  consistent with how those skills already work. The two explicit
  `subagent_type` cases (`review-pr`, `review-plan`) use deterministic
  inline preprocessor substitution instead.
- **Sub-agents don't inherit project context**: Spawned sub-agents (e.g.,
  the reviewer agent) do not see the project context injected into the
  parent skill's prompt. This is a known Claude Code limitation. Addressing
  this is deferred to a future plan that handles context injection into
  agent definitions.
- **Script errors fail fast**: If config infrastructure scripts (Plan 1)
  are broken or missing, the preprocessor commands will fail. This is
  intentional — a broken config setup should surface immediately rather
  than silently degrading skill behaviour.

## References

- Plan 1: `meta/plans/2026-03-23-config-infrastructure.md`
- Research: `meta/research/2026-03-22-skill-customisation-and-override-patterns.md`
- Agent spawning patterns: `review-pr/SKILL.md:257-258`,
  `create-plan/SKILL.md:64-69`, `research-codebase/SKILL.md:54-68`
- Preprocessor usage: `commit/SKILL.md:11-12`
