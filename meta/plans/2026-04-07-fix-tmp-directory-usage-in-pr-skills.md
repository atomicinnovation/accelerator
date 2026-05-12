---
date: "2026-04-07T23:15:00+01:00"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Fix /tmp Directory Usage in PR Skills — Implementation Plan

## Overview

The `describe-pr` skill hardcodes `/tmp` for its temporary body file, and the
`review-pr` skill relies on a fragile implicit convention for resolving
`{tmp directory}` placeholders that can silently fall back to `/tmp`. This plan
fixes the concrete bug in `describe-pr` and hardens `review-pr` with explicit
substitution instructions, with particular attention to the sub-agent prompt
composition boundary.

## Current State Analysis

### `describe-pr/SKILL.md`

- Line 16 declares `**PRs directory**:` via preprocessor — so it already uses
  the config system for one path
- Line 123 hardcodes `/tmp/pr-body-{number}.md` for the temporary PR body file
- Has no `**Tmp directory**:` preprocessor line
- Has no `{tmp directory}` placeholders anywhere

### `review-pr/SKILL.md`

- Line 26 declares `**Tmp directory**:` via preprocessor, resolving to
  `meta/tmp` by default
- Uses `{tmp directory}` in 16+ locations throughout the skill body
- Line 253 composes a sub-agent prompt containing `{tmp directory}` — the
  orchestrating LLM must resolve this before passing to the reviewer agent,
  creating a two-hop resolution chain that amplifies failure risk
- The reviewer agent definition (`agents/reviewer.md`) contains zero tmp
  directory references and relies entirely on the orchestrator's prompt

### Key Discoveries

- `skills/config/init/SKILL.md:18-30` — precedent for a dedicated "Path
  Resolution" section with all paths resolved via preprocessor
- `skills/config/init/SKILL.md:125` — precedent for explicit instruction:
  "Use the actual resolved paths in the output (not the variable names)"
- The `!` preprocessor only supports single-line inline expansion; it cannot
  perform mid-line substitution in code blocks
- The implicit mapping convention (bold label text lowercased → curly-brace
  placeholder) is used across all skills but is undocumented in any skill body

## Desired End State

After this plan is complete:

1. `describe-pr` uses the configured tmp directory (via `{tmp directory}`)
   instead of hardcoded `/tmp`
2. `review-pr` has an explicit, prominent instruction telling the LLM to
   substitute `{tmp directory}` with the resolved path value, and a specific
   instruction to resolve all placeholders before composing sub-agent prompts
3. Both skills produce correct paths when invoked, using `meta/tmp` (or
   whatever the user has configured via `paths.tmp`)

### How to Verify

- Read `describe-pr/SKILL.md` and confirm no literal `/tmp` paths remain
  (other than inside explanatory text if any)
- Read `review-pr/SKILL.md` and confirm the explicit substitution instruction
  is present near the bold-label definitions and that a sub-agent-specific
  instruction exists
- Invoke `/describe-pr` on a test PR and verify the temporary body file is
  written to `{tmp directory}/pr-body-{number}.md`, not `/tmp/pr-body-{number}.md`
- Invoke `/review-pr` on a test PR and verify all artefacts land in
  `{tmp directory}/pr-review-{number}/`, not `/tmp/pr-review-{number}/`

## What We're NOT Doing

- Not extending the `!` preprocessor to do programmatic template substitution
  (Solution C from the research)
- Not auditing or changing other skills beyond `describe-pr` and `review-pr`
- Not adding validation that `meta/tmp` exists — `init` already handles this
- Not changing the bold-label-to-placeholder convention itself

## Implementation Approach

Two independent phases, each modifying a single skill file. Phase 1 is a
straightforward bug fix. Phase 2 adds instructional guardrails to an existing
mechanism.

## Phase 1: Fix `describe-pr` Hardcoded `/tmp`

### Overview

Add tmp directory resolution to `describe-pr` and replace the hardcoded `/tmp`
path with the `{tmp directory}` placeholder.

### Changes Required

#### 1. Add `**Tmp directory**` preprocessor line

**File**: `skills/github/describe-pr/SKILL.md`

After the existing `**PRs directory**:` line (line 16), add:

```markdown
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`
```

This follows the same pattern already used for `**PRs directory**:` in this
file and for `**Tmp directory**:` in `review-pr` and `init`.

After the new `**Tmp directory**:` line, add a brief substitution reminder:

```markdown
**IMPORTANT**: Wherever `{prs directory}` or `{tmp directory}` appears in
the instructions below, substitute the actual resolved path shown above.
```

This follows the precedent set by `init/SKILL.md` line 125 ("Use the actual
resolved paths in the output (not the variable names)") and ensures
`describe-pr` receives the same hardening that Phase 2 adds to `review-pr`.

#### 2. Add directory creation step

**File**: `skills/github/describe-pr/SKILL.md`

Before the existing step that writes the temporary body file (line 122),
add a step to ensure the tmp directory exists:

```
  3. Ensure the tmp directory exists: `mkdir -p {tmp directory}`
```

This mirrors the `mkdir -p` in `review-pr` Step 1.2 and is necessary
because `meta/tmp` (unlike `/tmp`) is not guaranteed to exist — the user
may not have run `/init`.

Note: adding this step shifts the subsequent step numbers by 1.

#### 3. Replace hardcoded `/tmp` path

**File**: `skills/github/describe-pr/SKILL.md`

Change the `gh pr edit` step (originally line 123, now step 5) from:

```
  4. Post with `gh pr edit {number} --body-file /tmp/pr-body-{number}.md`
```

To:

```
  5. Post with `gh pr edit {number} --body-file {tmp directory}/pr-body-{number}.md`
```

#### 4. Update the preceding instruction for consistency

**File**: `skills/github/describe-pr/SKILL.md`

The step that currently says "Write everything after the closing `---` line
to a temporary file" (originally line 122) is vague about where the
temporary file goes. Update to step 4:

```
  4. Write everything after the closing `---` line to
     `{tmp directory}/pr-body-{number}.md`
```

And update the cleanup step (originally line 124) to step 6:

```
  6. Clean up `{tmp directory}/pr-body-{number}.md`
```

Note: line numbers in changes 2, 3, and 4 refer to the original file
before Change 1 is applied. Change 1 adds several lines after line 16
(the preprocessor line and the IMPORTANT substitution instruction), so
all subsequent line numbers shift accordingly.

### Success Criteria

#### Automated Verification

- [x] No literal `/tmp/` paths remain in `describe-pr/SKILL.md`:
      `grep -c '/tmp/' skills/github/describe-pr/SKILL.md` returns 0
- [x] The `**Tmp directory**:` preprocessor line is present:
      `grep -c 'Tmp directory' skills/github/describe-pr/SKILL.md` returns at
      least 1
- [x] The `{tmp directory}` placeholder is used:
      `grep -c '{tmp directory}' skills/github/describe-pr/SKILL.md` returns at
      least 1
- [x] A `mkdir -p` step for the tmp directory is present:
      `grep -c 'mkdir -p {tmp directory}' skills/github/describe-pr/SKILL.md`
      returns at least 1
- [x] An explicit substitution instruction is present:
      `grep -c 'IMPORTANT' skills/github/describe-pr/SKILL.md` returns at
      least 1

#### Manual Verification

- [ ] Invoke `/describe-pr` on a test PR and confirm the temporary body file
      is written to `meta/tmp/pr-body-{number}.md`
- [ ] Confirm the PR description is posted successfully to GitHub
- [ ] Confirm the temporary file is cleaned up after posting

---

## Phase 2: Harden `review-pr` Template Variable Resolution

### Overview

Add an explicit substitution instruction to `review-pr` near the bold-label
definitions, and a specific instruction about resolving placeholders before
composing sub-agent prompts. This uses the `init` skill's approach (line 125)
as precedent.

### Changes Required

#### 1. Add explicit substitution instruction after the bold-label definitions

**File**: `skills/github/review-pr/SKILL.md`

After line 26 (`**Tmp directory**: !`...``), add:

```markdown

**IMPORTANT**: Wherever `{tmp directory}` or `{pr reviews directory}` appears
in the instructions below, substitute the actual resolved path shown above.
Never use `/tmp` or any other path not shown above.

**IMPORTANT**: When composing prompts for sub-agents, resolve all `{...}`
path placeholders to their actual values before passing the prompt —
sub-agents cannot see the bold-label definitions above and have no way to
resolve the placeholders themselves.
```

Splitting into two separate callouts (general substitution and sub-agent
composition) follows the existing `**IMPORTANT**:` convention and improves
LLM compliance by keeping each instruction focused on a single concern.

This instruction is placed immediately after the bold-label definitions so
it's the first thing the LLM reads after seeing the resolved values. It
explicitly addresses the sub-agent composition risk identified in the research.

#### 2. Add a reminder at the sub-agent prompt composition site

**File**: `skills/github/review-pr/SKILL.md`

Before the sub-agent prompt template at line 248 (the `Compose each agent's
prompt following this template:` line), add a reminder:

```markdown
**Reminder**: In the template below, replace `{tmp directory}` with the
actual path resolved at the top of this skill before passing the prompt to
the agent.
```

This provides a second, contextually-placed reminder at the exact point where
the two-hop resolution failure occurs.

### Success Criteria

#### Automated Verification

- [x] The explicit substitution instruction is present after line 26:
      `grep -c 'substitute the actual resolved path' skills/github/review-pr/SKILL.md`
      returns at least 1
- [x] The sub-agent composition reminder is present:
      `grep -c 'replace.*{tmp directory}.*actual path' skills/github/review-pr/SKILL.md`
      returns at least 1

#### Manual Verification

- [ ] Invoke `/review-pr` on a test PR and verify all artefacts are written to
      `meta/tmp/pr-review-{number}/` (not `/tmp/pr-review-{number}/`)
- [ ] Verify the sub-agent prompt received by reviewer agents contains the
      resolved path `meta/tmp/pr-review-{number}` (not the literal placeholder
      `{tmp directory}/pr-review-{number}`)
- [ ] Verify the review payload JSON is written to
      `meta/tmp/pr-review-{number}/review-payload.json`

---

## Testing Strategy

### Manual Testing Steps

1. Run `/init` to ensure `meta/tmp` exists
2. Create or identify a test PR
3. Run `/describe-pr {number}` — verify temporary body file lands in
   `meta/tmp/`, not `/tmp/`
4. Run `/review-pr {number}` — verify:
   - `meta/tmp/pr-review-{number}/` is created with diff, changed-files, etc.
   - Reviewer agents receive the resolved `meta/tmp` path in their prompts
   - Review payload JSON is written to `meta/tmp/pr-review-{number}/`
   - The review posts successfully to GitHub

### Edge Cases

- Custom `paths.tmp` configuration (e.g., `.tmp` instead of `meta/tmp`) —
  both skills should respect the configured value
- Missing `meta/tmp` directory — `/init` creates it; if not initialised, the
  `mkdir -p` in both `describe-pr` and `review-pr` creates it on the fly
- Placeholder resolution failure — if the LLM fails to substitute
  `{tmp directory}` and passes the literal string, commands will fail with
  a path error containing curly braces. Verify after invocation that no
  literal `{tmp directory}` strings appear in created paths or sub-agent
  prompts

## References

- Research: `meta/research/codebase/2026-04-07-pr-review-tmp-directory-usage.md`
- Precedent: `skills/config/init/SKILL.md:125` — "Use the actual resolved
  paths in the output (not the variable names)"
- Precedent: `skills/config/init/SKILL.md:18-30` — Path Resolution section
- ADR: `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md`
