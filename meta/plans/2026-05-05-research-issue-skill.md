---
date: "2026-05-05 16:54:05 CEST"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# `research-issue` Skill Implementation Plan

## Overview

Create a dedicated `research-issue` skill that provides hypothesis-driven debugging workflows for production issues. It lives alongside `research-codebase` in `skills/research/` but uses a distinct RCA output template and a causal-chain investigation flow rather than exploratory breadth-first research.

## Current State Analysis

- `skills/research/` contains only `research-codebase`
- The template system (`config-read-template.sh`) resolves templates from `templates/<key>.md` — adding a new template just requires adding the file
- The metadata script (`research-metadata.sh`) is generic and reusable
- Test registration requires adding the skill to 3 arrays in `test-config.sh` plus adding specific path/template assertions
- All research skills output to the configured `research` path (default: `meta/research/codebase/`)

### Key Discoveries:

- `config-read-template.sh` auto-discovers templates from `templates/*.md` — no code changes needed to add a new template key
- `test-config.sh` has a hardcoded count for `config-read-skill-instructions.sh` usage (currently 31) that must be bumped
- The three test arrays (`CONTEXT_SKILLS`, `AGENT_SKILLS`, `ALL_SKILLS`) all need the new skill added

## Desired End State

A working `/research-issue` skill that:
1. Accepts structured (stacktrace/logs) or vague ("sometimes X causes Y") input
2. Follows a hypothesis-driven investigation flow
3. Outputs an RCA document to `meta/research/codebase/` using a dedicated `rca` template
4. Passes all existing `test-config.sh` assertions

### Verification:

```bash
bash scripts/test-config.sh  # All tests pass (no regressions, new skill included)
```

## What We're NOT Doing

- Cross-repository investigation via `gh` CLI (future enhancement)
- "Reproduce" step or observability tool integration
- Changes to `config-read-template.sh` or `config-common.sh` logic
- A separate metadata script (reusing `research-metadata.sh`)

## Implementation Approach

Minimal addition: one new template file, one new SKILL.md, and test registration. The skill reuses all existing infrastructure (config scripts, agent types, metadata script, output path).

## Phase 1: RCA Output Template

### Overview

Create the dedicated RCA template that `research-issue` will use for its output documents.

### Changes Required:

#### 1. New template file

**File**: `templates/rca.md`
**Changes**: Create new file

```markdown
---
date: [Current date and time with timezone in ISO format]
researcher: [Git author]
git_commit: [Current commit hash]
branch: [Current branch name]
repository: [Repository name]
topic: "[Brief description of the issue]"
tags: [research, debugging, affected-component-names]
status: complete
last_updated: [Current date in YYYY-MM-DD format]
last_updated_by: [Researcher name]
---

# Investigation: [Brief Issue Description]

**Date**: [Current date and time with timezone]
**Researcher**: [Researcher name]
**Git Commit**: [Current commit hash]
**Branch**: [Current branch name]
**Repository**: [Repository name]

## Issue Description

[What was reported — error message, stacktrace, behavioral description, or user report]

## Input Classification

[Structured (stacktrace/logs) | Vague (behavioral description) | Mixed]

## Affected Components

- `path/to/file.ext:line` - [Role in the issue]

## Timeline / Reproduction

[For structured input: sequence of events leading to the failure]
[For vague input: conditions under which the issue occurs]

## Hypotheses

### Hypothesis 1: [Name]
- **Evidence for**: [What supports this theory]
- **Evidence against**: [What contradicts it]
- **Verdict**: [Confirmed / Eliminated / Inconclusive]

### Hypothesis 2: [Name]
- **Evidence for**: [What supports this theory]
- **Evidence against**: [What contradicts it]
- **Verdict**: [Confirmed / Eliminated / Inconclusive]

## Root Cause

[The confirmed root cause with specific code references]

## Causal Chain

1. [Trigger event]
2. [Intermediate step]
3. [Failure point]

## Contributing Factors

- [Factor that made the issue possible or harder to detect]

## Fix Options

| Option | Description | Risk | Effort |
|--------|-------------|------|--------|
| A | [Description] | [Low/Med/High] | [Low/Med/High] |
| B | [Description] | [Low/Med/High] | [Low/Med/High] |

## Recommended Fix

[Which option and why]

## Prevention

- [What would prevent this class of issue in the future]

## Recent Changes

[Relevant git history on affected files, if applicable]

## Open Questions

[Any remaining uncertainties — omit section if none]
```

### Success Criteria:

#### Automated Verification:

- [x] Template resolves: `bash scripts/config-read-template.sh rca` outputs the template content

---

## Phase 2: Skill Definition

### Overview

Create the `research-issue` SKILL.md with the hypothesis-driven investigation workflow.

### Changes Required:

#### 1. Skill file

**File**: `skills/research/research-issue/SKILL.md`
**Changes**: Create new file with the hypothesis-driven workflow prompt

The SKILL.md should:
- Use the same frontmatter pattern as `research-codebase`
- Include `config-read-context.sh`, `config-read-skill-context.sh research-issue`, `config-read-agents.sh`
- Reference `config-read-path.sh research` for output directory
- Reference `config-read-template.sh rca` for output format
- Reference `research-metadata.sh` from the sibling skill for metadata
- Implement the 6-step workflow from the research document:
  1. Extract and classify input (structured vs vague)
  2. Map to code (stacktrace frames or action code paths)
  3. Check recent changes (`git log` on affected files)
  4. Form hypotheses (2-3 theories)
  5. Investigate in parallel (spawn agents per hypothesis)
  6. Synthesize into RCA document

**Key differentiation from `research-codebase`**:
- Input parsing: extracts errors, timestamps, request IDs, affected services
- Approach: hypothesis-driven (generate → test → eliminate) not breadth-first
- For vague/intermittent issues: specifically looks for race conditions, state variance, non-deterministic paths
- Output: RCA document (not research document)

#### Frontmatter:

```yaml
---
name: research-issue
description: Investigate production issues and bugs through hypothesis-driven
  debugging. Accepts stacktraces, logs, error messages, or vague behavioral
  descriptions and produces a root cause analysis.
argument-hint: "[issue description, stacktrace, or error message]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/*)
---
```

### Success Criteria:

#### Automated Verification:

- [x] File exists at `skills/research/research-issue/SKILL.md`
- [x] Contains `config-read-context.sh` within 5 lines of first `#` heading
- [x] Contains `config-read-skill-context.sh research-issue` on the line after context injection
- [x] Contains `config-read-agents.sh` after skill-context
- [x] Contains `config-read-path.sh research`
- [x] Contains `config-read-template.sh rca`
- [x] Contains `config-read-skill-instructions.sh research-issue`

---

## Phase 3: Test Registration

### Overview

Register `research-issue` in `test-config.sh` so it's validated by the test suite.

### Changes Required:

#### 1. Add to test arrays

**File**: `scripts/test-config.sh`
**Changes**:
- Add `"research/research-issue"` to `CONTEXT_SKILLS` array (line ~1047)
- Add `"research/research-issue"` to `AGENT_SKILLS` array (line ~1082)
- Add `"research/research-issue"` to `ALL_SKILLS` array (line ~3447)
- Bump `config-read-skill-instructions.sh` count from 31 to 32 (line ~3437)

#### 2. Add path and template assertions

**File**: `scripts/test-config.sh`
**Changes**: Add after the existing `research-codebase` assertions (~line 2998):

```bash
echo "Test: research-issue uses config-read-path.sh"
if grep -q 'config-read-path.sh research' "$SKILLS_DIR/research/research-issue/SKILL.md"; then
  echo "  PASS: research-issue has research path injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: research-issue has research path injection"
  FAIL=$((FAIL + 1))
fi

echo "Test: research-issue uses config-read-template.sh rca"
if grep -q 'config-read-template.sh rca' "$SKILLS_DIR/research/research-issue/SKILL.md"; then
  echo "  PASS: research-issue has rca template injection"
  PASS=$((PASS + 1))
else
  echo "  FAIL: research-issue has rca template injection"
  FAIL=$((FAIL + 1))
fi
```

### Success Criteria:

#### Automated Verification:

- [x] All tests pass: `bash scripts/test-config.sh`

---

## Testing Strategy

### Automated:

- `bash scripts/test-config.sh` — validates structural correctness (context placement, agent ordering, path/template injection, skill-instructions count)
- `bash scripts/config-read-template.sh rca` — validates template resolves

### Manual:

- Invoke `/research-issue` with a stacktrace and verify it follows the hypothesis-driven flow
- Invoke `/research-issue` with a vague description ("sometimes X causes Y") and verify it adapts its approach
- Verify output document matches the RCA template structure

## References

- Research document: `meta/research/codebase/2026-05-05-debug-issue-skill-design.md`
- Model skill: `skills/research/research-codebase/SKILL.md`
- Template infrastructure: `scripts/config-read-template.sh`, `scripts/config-common.sh`
- Test suite: `scripts/test-config.sh`
