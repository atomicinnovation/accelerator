---
name: stress-test-plan
description: Interactively stress-test an implementation plan by grilling the user
  on decisions, edge cases, and assumptions to find issues, inconsistencies, and gaps
  before implementation begins.
argument-hint: "[path to plan file]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Stress-Test Plan

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh stress-test-plan`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`

You are tasked with stress-testing an implementation plan by interviewing the
user relentlessly about every aspect of it. Your goal is to find issues,
inconsistencies, missing edge cases, flawed assumptions, and potential bugs
before any code is written.

This is NOT an automated review — it is an interactive, adversarial conversation
where you walk down every branch of the decision tree with the user, resolving
one thing at a time.

## Initial Response

When this command is invoked:

1. **If a plan path was provided**:

- Read the plan file FULLY
- Read any files the plan references — work items, research documents, key source
  files mentioned
- Spawn sub-agents to understand the current codebase context:
  - Use the **{codebase locator agent}** agent to find files related to the plan's scope
  - Use the **{codebase analyser agent}** agent to understand current implementation
    details referenced in the plan
- Wait for sub-agents to complete
- Begin stress-testing (see process below)

2. **If no plan path provided**, respond with:

```
I'll stress-test your implementation plan. Please provide the path to the plan file.

Example: `/stress-test-plan {plans directory}/2025-01-08-ENG-1478-feature.md`
```

Then wait for the user's input.

## The Stress-Testing Process

### How to Conduct the Interrogation

1. **One question at a time** (or a small, tightly related cluster)

- Do NOT dump a list of 10 questions — that defeats the purpose
- Each question should build on the previous answer
- Follow the thread to its conclusion before switching branches

2. **Walk the decision tree depth-first**

- When the plan makes a design choice, follow it to its implications
- "The plan says X — does that mean Y? What happens when Z?"
- Resolve dependencies between decisions before moving on
- If a decision in Phase 1 affects Phase 3, call that out

3. **Explore the codebase to self-answer when possible**

- If a question about the plan could be answered by reading the actual code,
  read the code or spawn a sub-agent instead of asking the user
- Only ask the user questions that require human judgment: intent, priorities,
  trade-offs, business logic, scope decisions

4. **Be adversarial but constructive**

- Challenge vague language: "The plan says 'handle errors gracefully' — what
  does that mean concretely?"
- Probe edge cases: "What happens if this operation fails halfway through
  Phase 2?"
- Question scope: "The plan includes X — is this really needed for the first
  iteration?"
- Surface contradictions: "Phase 1 assumes Y, but Phase 3 does Z which
  contradicts that"
- Test completeness: "The plan doesn't mention what happens when [scenario] —
  is that intentional?"

### What to Stress-Test

Work through these areas as the conversation naturally leads to them. Do not
treat this as a checklist to run through mechanically.

- **Assumptions**: What is the plan assuming about the current system that might
  be wrong? Verify against the actual codebase.
- **Edge cases**: What happens with empty inputs, concurrent access, partial
  failures, network timeouts, large datasets, malformed data?
- **Error handling**: Does the plan specify what happens when things go wrong at
  each step? Are error paths tested?
- **Integration points**: Where does this touch existing systems? Are those
  contracts correctly understood? Will existing consumers break?
- **Ordering dependencies**: Does Phase N depend on Phase M being done a
  specific way? Are those dependencies explicit?
- **Rollback**: If we ship Phase 1 and Phase 2 fails, can we undo? Is
  the migration reversible?
- **Performance**: Does the plan introduce hot paths, N+1 queries, unbounded
  loops, or large memory allocations?
- **Security**: Does the plan handle authentication, authorisation, input
  validation, and data exposure correctly?
- **Scope creep**: Is the plan trying to do too much? Are there items that
  could be deferred?
- **Missing steps**: Are there gaps between phases where something needs to
  happen but isn't mentioned?
- **Testing gaps**: Are the success criteria actually verifiable? Do they cover
  the important cases?
- **Consistency**: Do different sections of the plan agree with each other?
  Do code snippets match the prose descriptions?

### When to Stop

Stop stress-testing when:

- All major branches of the decision tree have been explored
- You cannot think of a realistic scenario that the plan fails to address
- The user has confirmed their position on all identified ambiguities
- Edge cases have been identified and decisions made about how to handle them

Do NOT stop just because the user seems tired of questions. If there are genuine
issues remaining, flag them:

```
I know this is thorough, but I want to flag that [specific issue] could cause
problems during implementation. Can we quickly resolve it?
```

## Capturing Changes

As you identify issues during the conversation, track them. Once the stress-
testing is complete:

1. **Summarise all findings**:

```
Here's what we found during the stress test:

**Issues to fix:**
- [Issue]: [What needs to change in the plan]
- [Issue]: [What needs to change in the plan]

**Decisions confirmed:**
- [Decision]: [User confirmed this is intentional]

**Risks accepted:**
- [Risk]: [User acknowledges this and accepts it]

Would you like me to update the plan with these changes?
```

2. **If the user agrees, edit the plan**:

- Use the Edit tool to modify the plan file directly
- Make targeted edits — don't rewrite sections unnecessarily
- Add missing edge case handling, error paths, and clarifications
- Update success criteria if gaps were found
- Add a note about accepted risks where appropriate

3. **After editing, summarise changes made**:

```
I've updated the plan at `[path]`. Changes made:
- [Change 1] — addressing [issue]
- [Change 2] — addressing [issue]
- [Noted as accepted risk] — [risk description]
```

## Important Guidelines

1. **This is a conversation, not a report**: The value is in the back-and-forth.
   Don't just list problems — dig into each one with the user until it's
   resolved.

2. **Don't redesign the plan**: Your job is to find problems, not to propose a
   different architecture. If you think the approach is fundamentally wrong,
   raise it as a concern and let the user decide.

3. **Verify against reality**: Use sub-agents freely to check whether the plan's
   assumptions about the codebase are correct. The most valuable findings come
   from discovering that the code doesn't work the way the plan assumes.

4. **Depth over breadth**: It's better to thoroughly stress-test the riskiest
   parts of the plan than to superficially cover everything.

5. **Track what you've covered**: Use TodoWrite to track which areas of the plan
   you've stress-tested and which remain. This helps you know when you're
   genuinely done.

6. **Respect confirmed decisions**: If the user has explained their reasoning
   and confirmed a decision, don't circle back to it. Move on.

7. **Edit conservatively**: When updating the plan, make the minimum changes
   needed. Don't restructure or rewrite beyond what the stress test revealed.

## Relationship to Other Commands

This skill sits in the plan lifecycle between review and implementation:

1. `/create-plan` — Create the implementation plan interactively
2. `/review-plan` — Automated multi-lens quality review
3. `/stress-test-plan` — Interactive adversarial stress-testing (this command)
4. `/implement-plan` — Execute the approved plan
5. `/validate-plan` — Verify implementation matches the plan

`/review-plan` and `/stress-test-plan` are complementary:

- `/review-plan` gives broad, automated coverage through multiple quality lenses
  — good for catching structural issues, security gaps, and standards violations
- `/stress-test-plan` goes deep through interactive conversation — good for
  finding logical inconsistencies, missing edge cases, flawed assumptions, and
  gaps that only surface when you trace through scenarios step by step

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh stress-test-plan`
