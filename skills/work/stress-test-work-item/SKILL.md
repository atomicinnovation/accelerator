---
name: stress-test-work-item
description: Interactively stress-test a work item by grilling the user
  on scope, assumptions, acceptance criteria, edge cases, and
  dependencies to surface issues, gaps, and flawed assumptions before
  implementation is planned.
argument-hint: "[work item number or path]"
disable-model-invocation: false
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Stress-Test Work Item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh stress-test-work-item`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`

You are tasked with stress-testing a work item by interviewing the user
relentlessly about every aspect of it. Your goal is to find issues,
inconsistencies, missing edge cases, flawed assumptions, and vague acceptance
criteria before implementation is planned.

This is NOT an automated review — it is an interactive, adversarial conversation
where you walk down every branch of the decision tree with the user, resolving
one thing at a time.

## Initial Response

When this command is invoked:

1. **If a work item path or number was provided**:

   - Accepted forms: a path (e.g. `meta/work/0042-user-auth.md`) or a bare
     work item number (e.g. `0042` or `42`, resolved against `{work_dir}`)
   - If the resolved path does not exist: report "No work item file at <path>"
     and exit without reading any other file or starting questioning
   - Read the work item file FULLY
   - If the work item has a non-empty `parent` field, read the parent work item too
   - Optionally spawn codebase agents ({codebase locator agent},
     {codebase analyser agent}) if the work item makes specific technical claims
     that the agents can verify — do NOT spawn them reflexively for every work item
   - Wait for any spawned agents to complete before asking the first question
   - Begin stress-testing (see process below)

2. **If no work item path or number provided**, respond with:

   ```
   I'll stress-test your work item. Please provide the path or work item number.

   Example: `/stress-test-work-item {work_dir}/0042-user-auth.md`
   Or by number: `/stress-test-work-item 42`

   Run `/list-work-items` to see available work items.
   ```

   Then wait for the user's input.

## The Stress-Testing Process

### How to Conduct the Interrogation

1. **One question at a time** (or a small, tightly related cluster)

   - Do NOT dump a bulleted list of all issues — that defeats the purpose
   - Each question should build on the previous answer
   - Follow the thread to its conclusion before switching branches

2. **Walk the decision tree depth-first**

   - When you find a vague term or missing detail, follow it to its implications
   - "The work item says X — does that mean Y? What happens when Z?"
   - Resolve one ambiguity completely before moving to the next

3. **Self-answer from the codebase when possible**

   - If a question about the work item could be answered by reading the actual code,
     spawn a codebase agent instead of asking the user
   - Only ask the user questions that require human judgment: intent, priorities,
     trade-offs, scope decisions

4. **Be adversarial but constructive**

   - Challenge vague acceptance criteria: "'Handle errors gracefully' — what
     does that mean concretely? What would a failing test assert?"
   - Probe edge cases: "What happens when the input is empty? Malformed? Too large?"
   - Question scope: "These five acceptance criteria cover unrelated concerns —
     should this work item be decomposed?"
   - Surface contradictions: "The Summary says X, but the Requirements say Y"
   - Test completeness: "The Dependencies section is empty but Requirements imply
     a database schema change — is that intentional?"

### What to Stress-Test

Work through these areas as the conversation naturally leads to them. Do not
treat this as a checklist to run through mechanically.

- **Assumptions**: What is the work item assuming about the system that might
  be wrong? Verify against the actual codebase where applicable.
- **Acceptance criteria**: Are they testable, specific, and measurable? Do
  they cover failure paths, not just the happy path? Would a passing test
  be unambiguous?
- **Scope**: Too big? Too small? Does it try to do too much in one work item?
  Are there items that belong in a separate work item?
- **Edge cases**: What happens with empty data, concurrent access, partial
  failures, malformed input, timeouts, large datasets — whichever are
  relevant to this work item's domain.
- **Dependencies**: What must exist before this work? What does this work
  block? Are there schema or data changes that must precede this? Is the
  Dependencies section accurate?
- **Non-functional concerns**: Performance, security, accessibility,
  observability — do the requirements address these where applicable?
- **Definition of done**: Are the completion criteria clear and verifiable?
  Is it obvious when this work item is truly done?
- **Consistency**: Do the sections agree with each other? Does the Summary
  match the Requirements? Do the Requirements match the Acceptance Criteria?

### When to Stop

Stop stress-testing when:

- All major branches of the decision tree have been explored
- You cannot think of a realistic scenario that the work item fails to address
- The user has confirmed their position on all identified ambiguities
- Edge cases have been identified and decisions made about how to handle them

Do NOT stop just because the user seems tired of questions. If there are genuine
issues remaining, flag them explicitly before wrapping up.

## Capturing Changes

As you identify issues during the conversation, track them. Once the
stress-testing is complete:

1. **Summarise all findings**:

```
Here's what we found during the stress test:

**Issues to fix:**
- [Issue]: [What needs to change in the work item]

**Decisions confirmed:**
- [Decision]: [User confirmed this is intentional]

**Risks accepted:**
- [Risk]: [User acknowledges this and accepts it]

Would you like me to update the work item with these changes?
```

2. **If the user agrees, edit the work item**:

   - Use the Edit tool to apply targeted modifications to the body sections
     **Acceptance Criteria**, **Dependencies**, **Assumptions**, or
     **Technical Notes** ONLY
   - Never modify any frontmatter field (`work_item_id`, `title`, `date`,
     `author`, `type`, `status`, `priority`, `parent`, `tags`) nor the body
     `**Type**:`, `**Status**:`, `**Priority**:`, or `**Author**:` labels —
     those transitions are `/update-work-item`'s concern
   - Do NOT rewrite sections beyond what was agreed in the conversation
   - If an Edit target string cannot be matched (the section content differs
     from what was read), abort that specific edit with a clear diagnostic and
     continue with the remaining agreed edits

3. **After editing, summarise changes made**

## Important Guidelines

1. **This is a conversation, not a report**: The value is in the back-and-forth.
   Don't just list problems — dig into each one with the user until it's resolved.

2. **Don't redesign the work item**: Your job is to find problems, not to propose
   a different architecture. If you think the approach is fundamentally wrong,
   raise it as a concern and let the user decide. Do not rewrite Requirements
   to reflect an alternative approach.

3. **Verify against reality**: Use codebase agents to check whether the work item's
   technical assumptions are correct. The most valuable findings come from
   discovering that the code doesn't work the way the work item assumes.

4. **Depth over breadth**: It's better to thoroughly stress-test the riskiest
   parts of the work item than to superficially cover everything.

5. **Respect confirmed decisions**: If the user has explained their reasoning
   and confirmed a decision, don't circle back to it. Move on.

6. **Edit conservatively**: When updating the work item, make the minimum changes
   needed to address what was agreed.

## Relationship to Other Commands

This skill sits in the work item lifecycle between review and planning:

1. `/create-work-item` or `/extract-work-items` — create the work item
2. `/refine-work-item` — decompose and enrich
3. `/review-work-item` — automated multi-lens quality review
4. `/stress-test-work-item` — interactive adversarial examination (this command)
5. `/create-plan` — plan implementation from an approved work item

`/review-work-item` and `/stress-test-work-item` are complementary:

- `/review-work-item` gives broad, automated coverage through multiple quality lenses
  — good for catching structural issues and standards violations
- `/stress-test-work-item` goes deep through interactive conversation — good for
  finding logical inconsistencies, missing edge cases, flawed assumptions, and
  gaps that only surface when you trace through scenarios step by step

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh stress-test-work-item`
