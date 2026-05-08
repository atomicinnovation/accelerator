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
