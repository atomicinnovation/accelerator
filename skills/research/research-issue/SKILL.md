---
name: research-issue
description: Investigate production issues and bugs through hypothesis-driven
  debugging. Accepts stacktraces, logs, error messages, or vague behavioral
  descriptions and produces a root cause analysis.
argument-hint: "[issue description, stacktrace, or error message]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/*)
---

# Research Issue

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh research-issue`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser.

**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research`

You are tasked with investigating production issues and bugs through
hypothesis-driven debugging. You accept structured input (stacktraces, logs,
error messages) or vague behavioral descriptions and produce a root cause
analysis document.

## Initial Setup:

When this command is invoked, respond with:

```
I'm ready to investigate the issue. Please provide the stacktrace, error message, logs, or description of the behavior you're seeing, and I'll conduct a hypothesis-driven root cause analysis.
```

Then wait for the user's issue description.

## Steps to follow after receiving the issue description:

1. **Extract and classify input:**

- Determine input type: Structured (stacktrace/logs), Vague (behavioral
  description), or Mixed
- For structured input: extract error messages, file paths, line numbers,
  timestamps, request IDs, affected services
- For vague input: identify keywords, affected features, conditions under which
  the issue occurs, frequency patterns
- For intermittent/vague issues: specifically look for race conditions, state
  variance, non-deterministic code paths

2. **Map to code:**

- For structured input: resolve stacktrace frames to actual source files, check
  if referenced lines still match (code may have changed since the error)
- For vague input: identify code paths for the affected functionality
- Read the relevant source files FULLY (no limit/offset) to understand context
- Note any error handling, state management, or concurrency patterns

3. **Check recent changes:**

- Run `git log --oneline -20 -- <affected-files>` on each affected file
- Look for recent modifications that correlate with when the issue started
- Check if any recent refactoring touched the affected code paths
- Use `git diff` on suspicious commits if needed

4. **Form hypotheses (2-3 theories):**

- Based on the evidence gathered, formulate 2-3 plausible root causes
- Each hypothesis should be testable through code inspection
- Rank hypotheses by likelihood based on available evidence
- For vague/intermittent issues: always consider timing, ordering, and state
  as hypothesis categories

5. **Investigate in parallel:**

- Spawn sub-agent tasks to investigate each hypothesis concurrently
- Use the **{codebase analyser agent}** to trace specific code paths
- Use the **{codebase pattern finder agent}** to find similar patterns that
  might reveal the issue
- Use the **{codebase locator agent}** to find related components
- Each agent should look for evidence FOR and AGAINST its assigned hypothesis
- Collect specific file paths and line numbers as evidence

6. **Synthesise into RCA document:**

- Wait for ALL sub-agents to complete
- Evaluate each hypothesis: Confirmed, Eliminated, or Inconclusive
- Identify the root cause with specific code references
- Construct the causal chain from trigger to failure
- Propose fix options with risk/effort assessment
- Gather metadata using
  `${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/research-metadata.sh`
- Write the RCA document to the configured research directory using this
  template:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh rca`

- Filename format: `YYYY-MM-DD-description.md` where description is a brief
  kebab-case summary of the issue (e.g., `2025-01-08-auth-timeout-on-refresh.md`)

7. **Present findings (ONLY after the file has been written):**

- Confirm the file path where the RCA document was saved
- Summarise the root cause concisely
- Highlight the recommended fix with rationale
- Include key file references for easy navigation
- Ask if they want deeper investigation on any aspect

## Important notes:

- **MANDATORY FILE OUTPUT**: You MUST write the RCA document to a file in the
  configured research directory. NEVER present findings only as conversation
  output. The file write in step 6 is NOT optional — it is the primary
  deliverable of this skill. If you reach step 7 without having written a file,
  STOP and go back to step 6.
- Always use parallel Task agents to maximise efficiency
- Hypothesis-driven: generate theories THEN test them — don't just explore
  breadth-first
- For vague/intermittent issues: look for race conditions, state variance,
  non-deterministic paths, timing dependencies
- Evidence-based: every conclusion must reference specific code
- The RCA document should be self-contained and actionable
- Include the causal chain — not just "what" but "why" and "how"
- **File reading**: Always read affected files FULLY (no limit/offset)
- **Critical ordering**: Follow the numbered steps exactly
  - ALWAYS classify input before investigating (step 1)
  - ALWAYS check git history on affected files (step 3)
  - ALWAYS form hypotheses before spawning agents (step 4)
  - ALWAYS wait for all sub-agents before synthesising (step 6)
  - ALWAYS write the RCA document to a file before presenting findings (step 6
    before step 7)
  - NEVER write the RCA document with placeholder values
  - NEVER skip the file write — the document IS the output of this skill

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh research-issue`
