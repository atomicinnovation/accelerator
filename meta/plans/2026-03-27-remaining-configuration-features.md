---
date: "2026-03-27T23:30:00+0000"
type: plan
skill: create-plan
ticket: ""
status: final
---

# Remaining Configuration Features Implementation Plan

## Overview

The skill customisation system (Strategy D Hybrid) has been substantially
implemented across four prior plans, delivering 30 discrete customisation points
plus free-form context injection. However, several gaps and inconsistencies
remain between what the original research envisioned and what was implemented.
This plan addresses all remaining issues: non-deterministic agent name
resolution, hardcoded operational paths, hardcoded numeric defaults in prose,
misaligned template handling in `describe-pr`, and hardcoded example paths.

## Current State Analysis

The configuration infrastructure is solid: two-tier config files, shell-based
YAML parsing, SessionStart hook integration, `/accelerator:configure` skill, and
per-skill preprocessor integration. The gaps are:

1. **Agent name resolution is non-deterministic for 6 of 7 agents**: Only the
   `reviewer` agent is resolved at preprocessor time (in `review-pr` and
   `review-plan`). All other agents rely on an override table that the LLM must
   interpret — a fragile pattern the original research warned against.

2. **5 hardcoded operational paths** will break if users override `paths.*`
   config: `respond-to-pr` has no dynamic path variable at all;
   `review-pr`/`review-plan` have hardcoded `mkdir` commands; `implement-plan`
   hardcodes `meta/plans/` in instructions.

3. **12+ hardcoded numeric defaults** in review prose ("10", "6 to 8", "3")
   would become stale if defaults change or users override them. The "6 to 8"
   lens range is actually wrong — the real default minimum is 4, not 6.

4. **`describe-pr` uses manual template resolution** instead of
   `config-read-template.sh`, missing both the config key override and plugin
   default fallback.

5. **~14 hardcoded example paths** across 7 skills reference `meta/` paths that
   would be misleading if paths are overridden.

### Key Discoveries:

- `config-read-agents.sh` already reads all overrides in a single pass but
  outputs a table for LLM interpretation rather than labeled variable
  definitions (`scripts/config-read-agents.sh:101-129`)
- `config-read-review.sh` only outputs values that differ from defaults
  (`scripts/config-read-review.sh:275-277`) — it needs to always output labeled
  definitions for prose references
- `respond-to-pr/SKILL.md` is the only review-related skill that does NOT
  declare a dynamic path variable via `config-read-path.sh`
- No plugin default `pr-description.md` template exists in `templates/`
- `implement-plan` and `validate-plan` are missing `config-read-agents.sh`
  entirely despite spawning sub-tasks

## Desired End State

After this plan is complete:

1. **All agent names are resolved reliably** via labeled variable definitions
   output by a single `config-read-agents.sh` call per skill. No skill prose
   contains hardcoded agent names — all reference `{agent name}` variables.
   For the critical `subagent_type` parameter in `review-pr` and
   `review-plan`, the existing `config-read-agent-name.sh` inline calls are
   retained for truly deterministic resolution at preprocessor time.

2. **All operational paths use dynamic variables** from `config-read-path.sh`.
   No `mkdir`, `ls`, or glob commands contain literal `meta/` paths.

3. **All numeric defaults in review prose reference configured variables**
   output by `config-read-review.sh`. No skill prose contains hardcoded numbers
   like "10", "6 to 8", or "3" for configurable values.

4. **`describe-pr` uses `config-read-template.sh`** with a plugin default
   fallback, consistent with all other template-using skills.

5. **All example paths use dynamic directory variables** where those variables
   are defined in the skill.

### Verification:

- `grep -rn 'meta/reviews/prs' skills/` returns zero results
- `grep -rn 'meta/reviews/plans' skills/` returns zero results
- `grep -rn 'meta/plans/' skills/` returns zero results (except in
  preprocessor default arguments)
- `grep -rn 'meta/decisions/' skills/` returns zero results (except in
  preprocessor default arguments)
- `grep -rn 'meta/research/' skills/` returns zero results (except in
  preprocessor default arguments)
- `grep -rn 'meta/tickets/' skills/` returns zero results (except in
  preprocessor default arguments)
- `config-read-agent-name.sh` is only referenced in `review-pr` and
  `review-plan` (for the `subagent_type` parameter)
- All skills that spawn agents include `config-read-agents.sh` and reference
  agent names via `{...}` variables
- `scripts/test-config.sh` passes
- Manual test: configure `agents.codebase-locator: my-locator` and verify
  `create-plan` skill shows `my-locator` in its agent references

## What We're NOT Doing

- **Per-skill configuration** — config remains global-only
- **Response style/tone customisation** — emoji prefixes, attribution rules
  remain hardcoded (rated low value in original research)
- **Output format schema overrides** — review JSON schemas remain fixed
- **File naming convention overrides** — date-prefix patterns remain hardcoded
- **Config unset sentinel** — no mechanism to unset a team config key in local
  config
- **Context injection into sub-agents** — known Claude Code platform limitation

## Implementation Approach

The plan is structured as five phases, each independently testable. The approach
minimises latency impact by using bulk preprocessor output (one script call
emitting multiple labeled variable definitions) rather than per-value script
calls. This follows the established patterns for `config-read-review.sh` and
`config-read-path.sh`.

Key design decisions:

- **Agent names**: Modify `config-read-agents.sh` to always output all agent
  names (resolved or default) as labeled variable definitions. Skills reference
  these via `{codebase locator agent}` etc. Single preprocessor call per skill.
  For the `subagent_type` parameter in `review-pr` and `review-plan`, retain
  the existing `config-read-agent-name.sh` inline calls for truly deterministic
  resolution at the critical spawn point.
- **Numeric defaults**: Modify `config-read-review.sh` to always output labeled
  variable definitions for all numeric values, even when they match defaults.
  Prose references these variables.
- **Example paths**: Replace hardcoded `meta/` paths in examples with
  `{directory variable}` references where the variable is already defined in the
  skill.

**Note on line numbers**: Line references throughout this plan are relative to
the pre-change file state. Earlier phases (especially Phase 1 adding fallback
instructions and Phase 2 adding path declarations) will shift line numbers in
later phases. Implementers should match on surrounding text context rather than
relying on exact line numbers.

---

## Phase 1: Reliable Agent Name Resolution

### Overview

Replace the non-deterministic override table pattern with labeled variable
definitions that the skill prose references directly. This is the single
highest-value improvement for configuration reliability. For the critical
`subagent_type` parameter in review skills, retain the existing inline
`config-read-agent-name.sh` calls for truly deterministic preprocessor-time
resolution.

### Changes Required:

#### 1. Modify `config-read-agents.sh` to output labeled variable definitions

**File**: `scripts/config-read-agents.sh`
**Changes**: Replace the override table output (lines 101-129) with labeled
variable definitions that always emit all agent names (resolved or default).

Replace lines 101-129 with:

```bash
# Build resolved name for each agent (override or default).
# Convert hyphenated keys to space-separated display names for consistency
# with path variable labels (e.g., "codebase locator" not "codebase-locator").
AGENT_LINES=""
for key in "${AGENT_KEYS[@]}"; do
  val=$(printf '%s\n' "$OVERRIDES" | grep "^${key}=" | tail -1 | sed 's/^[^=]*=//' || true)
  if [ -z "$val" ]; then
    val="$key"
  fi
  display_name="${key//-/ }"
  AGENT_LINES="${AGENT_LINES}- **${display_name} agent**: ${val}
"
done

# Always output agent names block (skills reference these variables).
echo "## Agent Names"
echo ""
echo "The following agent names are configured for this project. Always use"
echo "the name shown for each role as the \`subagent_type\` parameter when"
echo "spawning agents via the Agent/Task tool."
echo ""
printf '%s' "$AGENT_LINES"
```

This means:

- Output is always emitted (no early exit when no overrides configured)
- Each agent gets a labeled line with spaces (matching path variable convention):
  `- **codebase locator agent**: codebase-locator`
- Skills reference these via `{codebase locator agent}` variable syntax
- Single preprocessor call per skill (no latency change)

**Failure mode mitigation**: Because skills now depend on the agent names block
for variable resolution, each skill that uses `{...agent}` variables should
include a fallback instruction after the preprocessor call:

```markdown
If no "Agent Names" section appears above, use these defaults: reviewer,
codebase-locator, codebase-analyser, codebase-pattern-finder,
documents-locator, documents-analyser, web-search-researcher.
```

This ensures skills degrade gracefully if `config-read-agents.sh` fails to
produce output (e.g., config parse error, shell failure).

#### 2. Update all 8 skills using `config-read-agents.sh` to reference variables

For each of the 8 skills that already include `config-read-agents.sh`, replace
hardcoded agent name references in prose with `{agent name agent}` variable
references. Variable names use spaces (not hyphens) to match the path variable
convention (e.g., `{codebase locator agent}` not `{codebase-locator agent}`).

**File**: `skills/research/research-codebase/SKILL.md`

- Line 62: `**codebase-locator**` → `**{codebase locator agent}**`
- Line 63: `**codebase-analyser**` → `**{codebase analyser agent}**`
- Line 64: `**codebase-pattern-finder**` → `**{codebase pattern finder agent}**`
- Line 69: `**documents-locator**` → `**{documents locator agent}**`
- Line 72: `**documents-analyser**` → `**{documents analyser agent}**`
- Line 77: `**web-search-researcher**` → `**{web search researcher agent}**`

**File**: `skills/planning/create-plan/SKILL.md`

- Line 71: `**codebase-locator**` → `**{codebase locator agent}**`
- Line 73: `**codebase-analyser**` → `**{codebase analyser agent}**`
- Line 75: `**documents-locator**` → `**{documents locator agent}**`
- Line 136: `**codebase-locator**` → `**{codebase locator agent}**`
- Line 138: `**codebase-analyser**` → `**{codebase analyser agent}**`
- Line 140: `**codebase-pattern-finder**` →
  `**{codebase pattern finder agent}**`
- Line 144: `**documents-locator**` → `**{documents locator agent}**`
- Line 146: `**documents-analyser**` → `**{documents analyser agent}**`

**File**: `skills/planning/review-plan/SKILL.md`

- Line 209: `` generic `reviewer` agent `` → `{reviewer agent}`
- Line 248: **Retain** existing `config-read-agent-name.sh` inline call
  (deterministic resolution for the critical `subagent_type` parameter)

**File**: `skills/planning/stress-test-plan/SKILL.md`

- Line 35: `**codebase-locator**` → `**{codebase locator agent}**`
- Line 36: `**codebase-analyser**` → `**{codebase analyser agent}**`

**File**: `skills/github/review-pr/SKILL.md`

- Line 233: `` generic `reviewer` agent `` → `{reviewer agent}`
- Line 279: **Retain** existing `config-read-agent-name.sh` inline call
  (deterministic resolution for the critical `subagent_type` parameter)

**File**: `skills/decisions/review-adr/SKILL.md`

- Line 105: `**documents-locator**` → `**{documents locator agent}**`
- Line 106: `**codebase-locator**` → `**{codebase locator agent}**`

**File**: `skills/decisions/extract-adrs/SKILL.md`

- Line 51: `**documents-locator**` → `**{documents locator agent}**`
- Line 72: `**documents-analyser**` → `**{documents analyser agent}**`

**File**: `skills/decisions/create-adr/SKILL.md`

- Line 71: `**documents-locator**` → `**{documents locator agent}**`
- Line 73: `**codebase-locator**` → `**{codebase locator agent}**`
- Line 199: backtick `documents-locator` and `codebase-locator` →
  `{documents locator agent}` and `{codebase locator agent}`

#### 3. Add `config-read-agents.sh` to skills that spawn sub-tasks

**File**: `skills/planning/implement-plan/SKILL.md`

- Add after line 12 (after `config-read-context.sh`):
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh``

**File**: `skills/planning/validate-plan/SKILL.md`

- Add after line 13 (after `config-read-context.sh`):
  `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh``

Then update any agent name references in these skills to use
`{agent name agent}` variable syntax (spaces, not hyphens).

#### 4. Update `config-read-agent-name.sh` documentation

**File**: `scripts/config-read-agent-name.sh`
**Action**: Retain this script. It continues to be used for the `subagent_type`
parameter in `review-pr` and `review-plan` where truly deterministic
preprocessor-time resolution is required. Update the script's header comment to
note that bulk resolution is handled by `config-read-agents.sh` and this script
is for inline use at critical spawn points only.

#### 5. Update `test-config.sh` for new agent output format

**File**: `scripts/test-config.sh`
**Changes**:

- Update `config-read-agents.sh` test cases (lines ~661-760) to expect the new
  labeled format (`- **codebase locator agent**: codebase-locator`) instead of
  the old table format (`| \`reviewer\` | \`my-custom-reviewer\` |`)
- Add new test case: no config produces all 7 agents with default names under
  `## Agent Names` heading (previously produced empty output)
- Update the expected heading from `## Agent Overrides` to `## Agent Names`
- Update the agent skill count assertion (line ~922-924) from 8 to 10
- Move `implement-plan` and `validate-plan` from `NON_AGENT_SKILLS` list
  (lines ~989-1001) to `AGENT_SKILLS` list
- Verify `config-read-agent-name.sh` placement tests (lines ~1012-1020) still
  pass (these are retained since the inline calls remain in `review-pr` and
  `review-plan`)
- Add verification grep: no skill references agent names without the ` agent`
  suffix in variable braces (e.g., `{codebase locator}` without `agent` should
  not appear)

### Success Criteria:

#### Automated Verification:

- [ ] `grep -rn 'Agent Overrides' scripts/` returns zero results (old table
  heading removed)
- [ ] `scripts/test-config.sh` passes (including updated assertions)
- [ ] All skills that include `config-read-agents.sh` reference agent names
  only via `{..agent}` variables:
  `grep -rn 'codebase-locator\|codebase-analyser\|documents-locator\|documents-analyser\|codebase-pattern-finder\|web-search-researcher' skills/ | grep -v 'config-read\|agent}\|subagent_type'`
  should return zero matches in skill prose (matches in preprocessor arguments
  and `subagent_type` values are expected)
- [ ] `grep -rn '{codebase locator}\|{documents locator}\|{reviewer}' skills/`
  returns zero results (all variable references must include ` agent` suffix)

#### Manual Verification:

- [ ] Configure `agents.codebase-locator: my-locator` in
  `.claude/accelerator.local.md` and invoke `/accelerator:create-plan` — verify
  the skill text shows `my-locator` where `codebase-locator` would normally
  appear
- [ ] Configure `agents.reviewer: my-reviewer` and invoke
  `/accelerator:review-pr` — verify the `subagent_type` parameter shows
  `my-reviewer`
- [ ] With no agent config, verify all skills show default agent names

#### Accepted Risk:

Agent name resolution for all agents except `reviewer` in review skills remains
LLM-interpreted (the LLM reads labeled definitions and applies them to `{...}`
variable references in prose). This is a significant improvement over the
override table pattern but is not truly deterministic. The fallback instruction
mitigates the failure case. Fully deterministic resolution for all agents would
require per-reference inline `config-read-agent-name.sh` calls, which would add
unacceptable preprocessor latency.

---

## Phase 2: Fix Hardcoded Operational Paths

### Overview

Fix all operational path references that would break or mislead if users
override `paths.*` configuration. This includes adding missing
`config-read-path.sh` declarations, fixing hardcoded `mkdir` commands, and
updating artifact template `target` fields.

### Changes Required:

#### 1. Add dynamic path variable to `respond-to-pr`

**File**: `skills/github/respond-to-pr/SKILL.md`
**Changes**: Add path declaration after `config-read-context.sh` and update
all hardcoded `meta/reviews/prs/` references.

Add after line 13:

```markdown
**PR reviews directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
```

Then update:

- Line 74: `meta/reviews/prs/{number}-review-*.md` →
  `{pr reviews directory}/{number}-review-*.md`
- Line 217: `` `meta/reviews/prs/{number}-review-{N}.md` `` →
  `` `{pr reviews directory}/{number}-review-{N}.md` ``

#### 2. Add dynamic path variable to `implement-plan`

**File**: `skills/planning/implement-plan/SKILL.md`
**Changes**: Add path declaration and update hardcoded references.

Add after the `config-read-agents.sh` line (added in Phase 1):

```markdown
**Plans directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
```

Then update:

- Line 3 (description): `meta/plans/` → `the configured plans directory`
- Line 15: `` `meta/plans/` `` → `the configured plans directory (shown above)`

#### 3. Fix hardcoded `mkdir` in `review-pr`

**File**: `skills/github/review-pr/SKILL.md`

- Line 413: `mkdir -p meta/reviews/prs` → `mkdir -p {pr reviews directory}`

#### 4. Fix hardcoded `mkdir` in `review-plan`

**File**: `skills/planning/review-plan/SKILL.md`

- Line 397: `mkdir -p meta/reviews/plans` → `mkdir -p {plan reviews directory}`

#### 5. Fix artifact template `target` fields

**File**: `skills/planning/validate-plan/SKILL.md`

Add path declaration after line 13 (after `config-read-agents.sh` added in
Phase 1):

```markdown
**Plans directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
```

- Line 129: `target: "meta/plans/{plan-filename}.md"` →
  `target: "{plans directory}/{plan-filename}.md"`

**File**: `skills/planning/review-plan/SKILL.md`

Add path declaration after line 17:

```markdown
**Plans directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
```

- Line 412: `target: "meta/plans/{plan-stem}.md"` →
  `target: "{plans directory}/{plan-stem}.md"`

### Success Criteria:

#### Automated Verification:

- [ ] `grep -rn 'mkdir -p meta/' skills/` returns zero results
- [ ] `grep -n 'meta/reviews/prs' skills/github/respond-to-pr/SKILL.md`
  returns zero results
- [ ] `grep -n 'meta/plans' skills/planning/implement-plan/SKILL.md` returns
  zero results (except preprocessor default argument)
- [ ] `grep -q 'config-read-path.sh review_prs' skills/github/respond-to-pr/SKILL.md`
  succeeds (structural test for new path injection)
- [ ] `scripts/test-config.sh` passes

#### Manual Verification:

- [ ] Configure `paths.review_prs: reviews/pull-requests` and invoke
  `/accelerator:review-pr` — verify the mkdir and glob commands use the
  configured path
- [ ] Configure `paths.plans: docs/plans` and invoke
  `/accelerator:implement-plan` — verify instructions reference the configured
  path
- [ ] Invoke `/accelerator:respond-to-pr` with `paths.review_prs` overridden —
  verify the review artifact lookup uses the configured path

---

## Phase 3: Inject Review Numeric Defaults via Preprocessor

### Overview

Modify `config-read-review.sh` to always output labeled variable definitions
for all numeric and threshold values, then update review prose to reference
these variables instead of hardcoding numbers.

### Changes Required:

#### 1. Modify `config-read-review.sh` to always output a consolidated section

**File**: `scripts/config-read-review.sh`
**Changes**: Replace the conditional "Review Configuration" section (which only
emits when values differ from defaults) with a single always-emitted "Review
Configuration" section that includes both labeled variable definitions and
override annotations. This eliminates the dual-section redundancy identified in
the review.

Restructure the output (after validation, line 121) to always emit:

```bash
# --- Always output review configuration ---
echo "## Review Configuration"
echo ""

if [ "$MODE" = "pr" ]; then
  _emit_value "max inline comments" "$max_inline_comments" "$DEFAULT_MAX_INLINE_COMMENTS"
  _emit_value "dedup proximity" "$dedup_proximity" "$DEFAULT_DEDUP_PROXIMITY"
  _emit_value "pr request changes severity" "$pr_request_changes_severity" "$DEFAULT_PR_REQUEST_CHANGES_SEVERITY"
fi

if [ "$MODE" = "plan" ]; then
  _emit_value "plan revise severity" "$plan_revise_severity" "$DEFAULT_PLAN_REVISE_SEVERITY"
  _emit_value "plan revise major count" "$plan_revise_major_count" "$DEFAULT_PLAN_REVISE_MAJOR_COUNT"
fi

_emit_value "min lenses" "$min_lenses" "$DEFAULT_MIN_LENSES"
_emit_value "max lenses" "$max_lenses" "$DEFAULT_MAX_LENSES"
echo ""
```

Where `_emit_value` is a helper function:

```bash
_emit_value() {
  local label="$1" value="$2" default="$3"
  if [ "$value" != "$default" ]; then
    echo "- **${label}**: ${value} (default: ${default})"
  else
    echo "- **${label}**: ${value}"
  fi
}
```

This produces labeled variable definitions that skill prose references (e.g.,
`{max inline comments}`) while also annotating overridden values with their
defaults for informational purposes — all in a single section.

**Preserve existing conditional output blocks** after the `_emit_value` calls,
within the same "Review Configuration" section. The current script (lines
300-340) has three conditional blocks that must be retained:

1. **Core lenses** (lines 300-305): If `core_lenses` is configured, output
   `- **Core lenses**: arch, code-quality, ... (default: ...)`. Keep as-is.
2. **Disabled lenses** (lines 307-311): If `disabled_lenses` is configured,
   output `- **Disabled lenses**: ... (these lenses should be skipped ...)`.
   Keep as-is.
3. **Verdict overrides** (lines 313-340): If PR or plan verdict thresholds
   differ from defaults, output the verdict display logic. Keep as-is.

These blocks remain conditional (only shown when overridden) since they are
informational — skill prose does not reference them as `{...}` variables.

The output is always emitted (remove the `has_config` early exit at lines
275-277). The lens catalogue section continues unchanged after this block.

Place the `_emit_value` calls at ~line 275 (replacing the `has_config` early
exit), after all validation and custom lens discovery are complete. This ensures:
(a) emitted values have been validated, (b) lens name validation against the
combined built-in + custom set has run, and (c) the entire "Review
Configuration" section — numeric values, core_lenses, disabled_lenses, verdict
— is emitted as one contiguous block rather than split across the script.

**Update `test-config.sh`**: Update existing review config tests to expect:
- Always-emitted "Review Configuration" section (no longer empty when defaults)
- New labeled format with `- **label**: value` syntax
- Test: no config with `pr` mode emits all PR-relevant defaults including
  `min lenses: 4` and `max lenses: 8`
- Test: no config with `plan` mode emits plan-relevant defaults
- Test: overridden values show `(default: X)` annotation

#### 2. Update `review-pr/SKILL.md` prose to reference variables

**File**: `skills/github/review-pr/SKILL.md`

Replace all hardcoded numeric defaults with variable references:

- Line 175: `Otherwise, use the defaults: **6 to 8**` →
  `Otherwise, use the defaults: **{min lenses} to {max lenses}**`
- Line 329: `the configured dedup_proximity — default: 3 — of each other` →
  `the configured dedup proximity ({dedup proximity}) of each other`
- Line 346: `even if that exceeds 10` →
  `even if that exceeds {max inline comments}`
- Line 347: `the configured max_inline_comments (default: 10) comments` →
  `the configured max inline comments ({max inline comments}) comments`
- Line 397: `Only if more than 10 inline comments were produced` →
  `Only if more than {max inline comments} inline comments were produced`
- Line 470: `capped at ~10 inline` →
  `capped at ~{max inline comments} inline`
- Line 498: `due to the ~10 cap` →
  `due to the ~{max inline comments} cap`
- Line 614-615: `Use the configured max (default: 10)` →
  `Use the configured max ({max inline comments})`
- Line 635:
  `` the configured `max_inline_comments` (default: ~10) `` →
  `the configured max inline comments ({max inline comments})`

#### 3. Update `review-plan/SKILL.md` prose to reference variables

**File**: `skills/planning/review-plan/SKILL.md`

- Line 151: `Otherwise, use the defaults: **6 to 8**` →
  `Otherwise, use the defaults: **{min lenses} to {max lenses}**`
- Line 308-309:
  `` at or above `plan_revise_severity` (default: `critical`) `` →
  `at or above the plan revise severity ({plan revise severity})`
- Line 310-311:
  `` `plan_revise_major_count` or more `"major"` findings exist (default: 3) `` →
  `{plan revise major count} or more "major" findings exist`

### Success Criteria:

#### Automated Verification:

- [ ] 
  `grep -n 'default: 10\|default: ~10\|default: 3' skills/github/review-pr/SKILL.md`
  returns zero results
- [ ] `grep -n '6 to 8' skills/` returns zero results
- [ ] `scripts/test-config.sh` passes (including updated review config tests)
- [ ] Run `scripts/config-read-review.sh pr` with no config — verify it
  outputs a "Review Configuration" section with all labeled values including
  `min lenses: 4` and `max lenses: 8`

#### Manual Verification:

- [ ] Configure `review.max_inline_comments: 5` and invoke
  `/accelerator:review-pr` — verify all references to the cap show "5"
- [ ] Configure `review.min_lenses: 3` and invoke `/accelerator:review-plan` —
  verify lens range shows "3 to 8"
- [ ] With no review config, verify the skill text shows correct defaults
  (4 to 8, not 6 to 8)

---

## Phase 4: Align `describe-pr` Template Handling

### Overview

Switch `describe-pr` from manual filesystem check to `config-read-template.sh`,
adding a plugin default template for consistent behaviour with other
template-using skills.

### Changes Required:

#### 1. Create plugin default `pr-description.md` template

**File**: `templates/pr-description.md` (new file)
**Changes**: Create a sensible default PR description template. This is the
fallback when no user template exists.

```markdown
---
date: "{ISO timestamp}"
type: pr-description
skill: describe-pr
pr_number: {number}
pr_title: "{title}"
status: complete
---

# {PR Title}

## Summary

[1-3 sentence overview of what this PR does and why]

## Changes

- [Key change 1]
- [Key change 2]
- [Key change 3]

## Context

[Link to relevant ticket, plan, or research document if applicable]

## Testing

- [ ] [How the changes were tested]
- [ ] [Edge cases considered]

## Notes for Reviewers

[Any specific areas to focus on, known limitations, or follow-up work planned]
```

The template includes YAML frontmatter consistent with all other plugin default
templates. The skill's existing step 9 strips frontmatter before posting to
GitHub.

#### 2. Update `describe-pr` to use `config-read-template.sh`

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**: Replace the manual template resolution (lines 23-29) with
`config-read-template.sh` preprocessor injection.

Add template injection after the path declarations (after line 16):

```markdown
**PR description template**:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh pr-description`
```

Update the instructions (lines 23-29) to reference the injected template
instead of checking the filesystem:

```markdown
1. **Use the PR description template:**

- The template is shown above under "PR description template"
- Read the template carefully to understand all sections and requirements
```

Remove the instructions that tell the user to create a template file if one
doesn't exist — the plugin default now serves as the fallback.

**Also remove** the now-unused `config-read-path.sh templates meta/templates`
declaration (line 16). After switching to `config-read-template.sh`, nothing in
the skill references `{templates directory}`, so it becomes dead context.

#### 3. Update `config-read-template.sh` error message and header

**File**: `scripts/config-read-template.sh`
**Changes**: The script's resolution logic already handles any key name
(it checks config, then templates dir, then plugin default). However, two
updates are needed:

1. **Line 7**: Update the header comment from
   `Template names: plan, research, adr, validation` to include `pr-description`
2. **Line 85**: Update the error message from
   `Available templates: plan, research, adr, validation` to dynamically list
   available templates by scanning the plugin's `templates/` directory:
   ```bash
   available=""
   for f in "$PLUGIN_ROOT/templates/"*.md; do
     [ -f "$f" ] || continue
     name="$(basename "$f" .md)"
     if [ -z "$available" ]; then
       available="$name"
     else
       available="$available, $name"
     fi
   done
   if [ -z "$available" ]; then
     available="(none found)"
   fi
   echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: $available" >&2
   ```
   This avoids fragile `ls` output parsing, handles filenames with special
   characters safely, and produces a meaningful message if the templates
   directory is empty.

**Add test to `test-config.sh`**: Verify `config-read-template.sh pr-description`
succeeds and outputs the content of `templates/pr-description.md`.

#### 4. Update configure skill documentation

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add `templates.pr-description` to the list of template config
keys, documenting that it supports the same three-tier resolution as other
templates.

### Success Criteria:

#### Automated Verification:

- [ ] `templates/pr-description.md` exists
- [ ] `scripts/config-read-template.sh pr-description` succeeds and outputs
  template content (three-tier resolution works for new key)
- [ ] `grep -n 'check if.*templates directory.*pr-description' skills/github/describe-pr/SKILL.md`
  returns zero results (manual check removed)
- [ ] `scripts/test-config.sh` passes (including new pr-description test)

#### Manual Verification:

- [ ] Invoke `/accelerator:describe-pr` with no user template — verify the
  plugin default template is used
- [ ] Create `.claude/accelerator/templates/pr-description.md` with custom
  content — verify it takes precedence over the plugin default
- [ ] Configure `templates.pr-description: path/to/custom.md` in config —
  verify the explicit path takes precedence

---

## Phase 5: Dynamic Example and Metadata Paths

### Overview

Replace hardcoded `meta/` paths in examples, argument hints, and illustrative
text with dynamic `{directory variable}` references. This is cosmetic but
prevents user confusion when paths are overridden.

### Changes Required:

#### 1. Update example paths in `review-plan/SKILL.md`

**File**: `skills/planning/review-plan/SKILL.md`

- Line 37: `` `meta/plans/2025-01-08-ENG-1478-feature.md` `` →
  `` `{plans directory}/2025-01-08-ENG-1478-feature.md` ``
- Lines 41-42: Example invocations `meta/plans/...` →
  `{plans directory}/...`
- Line 391: `` `meta/plans/2026-03-22-improve-error-handling.md` `` →
  `` `{plans directory}/2026-03-22-improve-error-handling.md` ``

Note: This requires adding a plans directory path declaration to `review-plan`
(already added in Phase 2).

#### 2. Update example paths in `validate-plan/SKILL.md`

**File**: `skills/planning/validate-plan/SKILL.md`

- Line 116: `` `meta/plans/2026-03-22-improve-error-handling.md` `` →
  `` `{plans directory}/2026-03-22-improve-error-handling.md` ``

Note: This requires adding a plans directory path declaration (already added
in Phase 2).

#### 3. Update example paths in `stress-test-plan/SKILL.md`

**File**: `skills/planning/stress-test-plan/SKILL.md`

Add path declaration after `config-read-agents.sh`:

```markdown
**Plans directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
```

- Line 46: `` `/stress-test-plan meta/plans/2025-01-08-ENG-1478-feature.md` ``
  → `` `/stress-test-plan {plans directory}/2025-01-08-ENG-1478-feature.md` ``

#### 4. Update example paths in `create-plan/SKILL.md`

**File**: `skills/planning/create-plan/SKILL.md`

- Line 393: `See meta/tickets/eng-1478.md` →
  `See {tickets directory}/eng-1478.md`

(Plans directory and tickets directory are already declared in this skill.)

#### 5. Update `argument-hint` fields in frontmatter

**File**: `skills/decisions/review-adr/SKILL.md`

- Line 8: `argument-hint: "[@meta/decisions/ADR-NNNN.md] [--deprecate reason]"`

Add path declaration after `config-read-agents.sh`:

```markdown
**Decisions directory**: !
`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
```

Since `argument-hint` is YAML frontmatter processed before the preprocessor
runs, it cannot use `{decisions directory}` variables. Use a generic description
consistent with other skills' argument-hint style:

- `argument-hint: "[path to ADR] [--deprecate reason]"`

**File**: `skills/decisions/extract-adrs/SKILL.md`

- Line 7:
  `argument-hint: "[@meta/research/doc.md ...] or leave empty to scan all"`

Same limitation — use a generic description:

- `argument-hint: "[research doc paths...] or leave empty to scan all"`

#### 6. Update `research-codebase/SKILL.md` generic guidance

**File**: `skills/research/research-codebase/SKILL.md`

- Line 98: `Verify all meta/ paths are correct` →
  `Verify all output paths are correct`

#### 7. Update `adr-next-number.sh` comment

**File**: `skills/decisions/scripts/adr-next-number.sh`

- Line 5: `# Scans meta/decisions/ for the highest existing ADR-NNNN number` →
  `# Scans the configured decisions directory for the highest existing ADR-NNNN number`

### Success Criteria:

#### Automated Verification:

- [ ] `grep -rn 'meta/plans/' skills/ | grep -v 'config-read-path\|test-'`
  returns zero results
- [ ] `grep -rn 'meta/tickets/' skills/ | grep -v 'config-read-path\|test-'`
  returns zero results
- [ ] `grep -rn 'meta/decisions/' skills/ | grep -v 'config-read-path\|test-\|configure/'`
  returns zero results
- [ ] `grep -rn 'meta/research/' skills/ | grep -v 'config-read-path\|test-\|configure/'`
  returns zero results
- [ ] `scripts/test-config.sh` passes

#### Manual Verification:

- [ ] Configure `paths.plans: docs/plans` and invoke
  `/accelerator:review-plan` — verify example paths show `docs/plans/...`
- [ ] Invoke `/accelerator:stress-test-plan` — verify example invocation
  shows configured plans directory

---

## Testing Strategy

### Unit Tests:

- Run `scripts/test-config.sh` after each phase
- Verify `config-read-agents.sh` outputs correct labeled format with and
  without overrides (new test cases added in Phase 1)
- Verify `config-read-review.sh` always outputs "Review Configuration" section
  with labeled variables (new test cases added in Phase 3)
- Verify `config-read-template.sh` handles `pr-description` key (new test
  case added in Phase 4)
- Verify `config-read-template.sh` error message dynamically lists available
  templates

### Integration Tests:

- Configure overrides for agents, paths, and review settings simultaneously
- Invoke each modified skill and verify all variables resolve correctly
- Test with both team config and local config to verify precedence

### Manual Testing Steps:

1. With no config: verify all skills show correct defaults
2. With team config only: verify overrides apply
3. With team + local config: verify local overrides take precedence
4. With paths overridden: verify all mkdir, glob, and example paths use
   configured values
5. With review settings overridden: verify all numeric references show
   configured values

## Performance Considerations

- **Phase 1**: No latency change — `config-read-agents.sh` is already called
  once per skill; the output format changes but the number of shell processes
  is identical. The `config-read-agent-name.sh` inline calls in `review-pr`
  and `review-plan` are retained (no change to those skills' latency).
- **Phase 2**: Adding `config-read-path.sh` to `respond-to-pr` and
  `implement-plan` adds ~20-30ms per skill (one shell process each). Adding
  it to `validate-plan` and `review-plan` (for plans directory) adds the same.
- **Phase 3**: No additional latency — `config-read-review.sh` is already
  called; it just outputs more.
- **Phase 4**: No additional latency — replaces manual filesystem check with
  `config-read-template.sh` (one shell process, same as the filesystem check
  the LLM was doing).
- **Phase 5**: Adding `config-read-path.sh` to `stress-test-plan` and
  `review-adr` adds ~20-30ms each.

Net impact: +3-4 shell processes across all skills (one per new
`config-read-path.sh` call). Total: ~+60-120ms additional latency distributed
across the affected skills. Context token overhead increases slightly from
always-emitted agent names and review settings blocks (~200-400 tokens per
skill invocation), which is an acceptable trade-off for configuration
reliability.

## References

- Original research:
  `meta/research/2026-03-22-skill-customisation-and-override-patterns.md`
- Implementation status review:
  `meta/research/2026-03-27-skill-customisation-implementation-status.md`
- Plan 1 (config infrastructure):
  `meta/plans/2026-03-23-config-infrastructure.md`
- Plan 2 (context and agent customisation):
  `meta/plans/2026-03-23-context-and-agent-customisation.md`
- Plan 3 (review system customisation):
  `meta/plans/2026-03-23-review-system-customisation.md`
- Plan 4 (template and path customisation):
  `meta/plans/2026-03-23-template-and-path-customisation.md`
