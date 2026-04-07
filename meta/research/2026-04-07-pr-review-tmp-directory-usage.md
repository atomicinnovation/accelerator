---
date: "2026-04-07T22:39:55+01:00"
researcher: Toby Clemson
git_commit: 508ca24b973d8c742e52e829c557f0c62f81076d
branch: main
repository: accelerator
topic: "Why does the PR review skill use /tmp instead of meta/tmp?"
tags: [research, codebase, review-pr, tmp-directory, template-variables]
status: complete
last_updated: "2026-04-07"
last_updated_by: Toby Clemson
---

# Research: Why Does the PR Review Skill Use /tmp Instead of meta/tmp?

**Date**: 2026-04-07T22:39:55+01:00
**Researcher**: Toby Clemson
**Git Commit**: 508ca24b973d8c742e52e829c557f0c62f81076d
**Branch**: main
**Repository**: accelerator

## Research Question

The PR review skill is supposed to use `meta/tmp` to store temporary
PR-related files. However, the skill frequently uses `/tmp` instead.
Why does this happen and what are appropriate solutions?

## Summary

There are two distinct causes. First, the `describe-pr` skill has a
**hardcoded `/tmp` path** at line 123, which is a concrete bug. Second,
the `review-pr` skill itself has no hardcoded `/tmp` references but
relies on a **purely implicit template variable mechanism** where
`{tmp directory}` placeholders are resolved by the LLM reading a bold
label (`**Tmp directory**: meta/tmp`) earlier in the prompt. This
convention-based approach is fragile: the LLM can fail to substitute
correctly and fall back to `/tmp` based on general training knowledge.
The problem compounds when the orchestrator composes sub-agent prompts
— if it fails to resolve `{tmp directory}` before passing the prompt to
the reviewer agent, the agent has no way to recover.

## Detailed Findings

### 1. Hardcoded /tmp in describe-pr (Concrete Bug)

**File**: `skills/github/describe-pr/SKILL.md:123`

```
4. Post with `gh pr edit {number} --body-file /tmp/pr-body-{number}.md`
```

The `describe-pr` skill hardcodes `/tmp` for its temporary body file.
Unlike `review-pr`, it has no `**Tmp directory**` preprocessor line and
no `{tmp directory}` placeholders. This is a straightforward bug — the
skill should use the configured tmp directory.

### 2. Template Variable Resolution is Purely Implicit (Root Cause)

The `review-pr` skill uses this pattern:

1. **Preprocessor output** (line 26): The `!` preprocessor runs
   `config-read-path.sh tmp meta/tmp` and inlines the result, producing:
   ```
   **Tmp directory**: meta/tmp
   ```

2. **Template placeholders** (16 occurrences): The skill body uses
   `{tmp directory}` as a placeholder, e.g.:
   ```
   mkdir -p {tmp directory}/pr-review-{number}
   ```

3. **No programmatic binding**: There is no code that replaces
   `{tmp directory}` with the resolved value. The LLM is expected to
   read the bold label and mentally substitute the value wherever the
   placeholder appears.

This same label-to-placeholder convention is used across all skills:

| Label | Placeholder |
|-------|-------------|
| `**PR reviews directory**:` | `{pr reviews directory}` |
| `**Tmp directory**:` | `{tmp directory}` |
| `**Plans directory**:` | `{plans directory}` |
| `**Tickets directory**:` | `{tickets directory}` |

The mapping rule is implicit: the bold label text (lowercased, without
`**` markers and trailing `:`) equals the curly-brace placeholder name.

### 3. Sub-agent Prompt Composition Amplifies the Risk

At `review-pr/SKILL.md:252-253`, the orchestrator composes a prompt
for reviewer sub-agents:

```
The PR artefacts are in the temp directory at {tmp directory}/pr-review-{number}:
```

This creates a two-hop resolution chain:
1. The orchestrating LLM must first resolve `{tmp directory}` → `meta/tmp`
2. It must embed that resolved path into the sub-agent prompt string
3. The sub-agent then uses the embedded path

If the orchestrator fails at step 1 (uses `/tmp` or passes through the
literal placeholder), the sub-agent has no access to the bold label
context and cannot recover. The reviewer agent definition
(`agents/reviewer.md`) contains zero references to tmp directories — it
relies entirely on what the orchestrator tells it.

### 4. Why /tmp Specifically?

The LLM defaults to `/tmp` because:
- `/tmp` is the conventional Unix temporary directory
- LLM training data overwhelmingly associates "temporary files" with
  `/tmp`
- The word "temp" / "tmp" in the skill instructions primes the
  association
- When the template variable resolution fails (or the model doesn't
  fully attend to the bold label), `/tmp` is the most natural fallback

### 5. The reviewer Agent is Not at Fault

The reviewer agent (`agents/reviewer.md`) is a minimal definition with
only read-only tools (Read, Grep, Glob, LS). It contains no path
references and receives all context via the task prompt. The issue is
entirely in the orchestrator layer.

## Suggested Solutions

### Solution A: Inline the Resolved Path Directly (Recommended)

Replace `{tmp directory}` placeholders in the skill body with
additional preprocessor directives that inline the resolved value
directly. For example, instead of:

```
mkdir -p {tmp directory}/pr-review-{number}
```

Use a preprocessor-resolved variable or repeat the preprocessor call.
However, the current `!` preprocessor only supports single-line
inline expansion — it cannot be used mid-line in a code block.

A more practical variant: define the path once at the top in a way
that creates a stronger binding, such as placing it in a dedicated
"Variables" section with explicit instructions like:

```markdown
## Variable Definitions

The following variables are used throughout this document. Always
substitute the value shown — never use a different path.

| Variable | Value |
|----------|-------|
| `{tmp directory}` | !`config-read-path.sh tmp meta/tmp` |
| `{pr reviews directory}` | !`config-read-path.sh review_prs meta/reviews/prs` |
```

### Solution B: Explicit Substitution Instruction

Add an explicit instruction near the variable definitions telling the
LLM what to do:

```markdown
**IMPORTANT**: Wherever you see `{tmp directory}` in the instructions
below, substitute the actual path shown above (`meta/tmp`). Do NOT
use `/tmp` or any other path.
```

This is low-effort but relies on the model following instructions
(which it usually does when they're prominent enough).

### Solution C: Preprocessor-Level Template Substitution

Extend the `!` preprocessor (or add a post-processing step) to
perform actual string substitution — scan the skill body for
`{variable name}` patterns and replace them with the corresponding
bold-label values before the prompt reaches the LLM. This would
eliminate the implicit resolution entirely but requires changes to the
Claude Code plugin framework or a wrapper script.

### Solution D: Fix describe-pr Immediately

Regardless of which structural solution is chosen for the template
variable mechanism, the hardcoded `/tmp` in `describe-pr/SKILL.md:123`
should be fixed immediately:

1. Add a `**Tmp directory**` preprocessor line to the skill
2. Replace `/tmp/pr-body-{number}.md` with
   `{tmp directory}/pr-body-{number}.md`

## Code References

- `skills/github/review-pr/SKILL.md:26` — Preprocessor line resolving
  tmp directory path
- `skills/github/review-pr/SKILL.md:87-98` — First uses of
  `{tmp directory}` placeholder
- `skills/github/review-pr/SKILL.md:252-253` — Sub-agent prompt
  template using `{tmp directory}`
- `skills/github/describe-pr/SKILL.md:123` — Hardcoded `/tmp` path
  (bug)
- `scripts/config-read-path.sh:22-23` — Path resolution delegation
- `scripts/config-read-value.sh:128` — Default value fallback
- `agents/reviewer.md` — Reviewer agent definition (no tmp references)
- `skills/config/init/SKILL.md:125` — Explicit instruction to use
  resolved paths (precedent)

## Historical Context

- `meta/plans/2026-03-28-initialise-skill-and-review-pr-ephemeral-migration.md`
  — Documents the migration of review-pr ephemeral files from
  `{pr reviews directory}` to `{tmp directory}`, which established the
  current pattern
- `meta/tickets/0003-diff-data-pipeline.md` — Diff data pipeline ticket
  covering shared storage and validation
- `meta/decisions/ADR-0008-shared-temp-directory-for-pr-diff-delivery.md`
  — ADR for the shared temp directory approach (staged, not yet
  committed)

## Open Questions

1. Should the template variable mechanism be made explicit (Solution C)
   or is a stronger convention (Solutions A/B) sufficient?
2. Are there other skills besides `describe-pr` that hardcode paths
   instead of using the configured resolution? (None found in this
   search, but worth auditing.)
3. Would it help to have the `init` skill validate that `meta/tmp`
   exists and is writable, providing an early signal if the directory
   is missing?
