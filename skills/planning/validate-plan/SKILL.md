---
name: validate-plan
description: Validate that an implementation plan was correctly executed by
  verifying success criteria and identifying deviations. Use after implementing
  a plan to verify correctness.
argument-hint: "[path to plan file]"
allowed-tools:
   - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Validate Plan

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh validate-plan`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations`

## Plan Validation Template

The template below defines the frontmatter and body structure that
every plan validation report must carry. Read it now — use it to guide
what information you record in the validation process and what shape
you persist in Step 4.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh validation`

You are tasked with validating that an implementation plan was correctly
executed, verifying all success criteria and identifying any deviations or
issues.

## Initial Setup

When invoked:

1. **Determine context** - Are you in an existing conversation or starting
   fresh?

- If existing: Review what was implemented in this session
- If fresh: Need to discover what was done through git and codebase analysis

2. **Locate the plan**:

- If plan path provided, use it
- Otherwise, search recent commits for plan references or ask user

3. **Gather implementation evidence**:
   ```bash
   # Check recent commits
   git log --oneline -n 20
   git diff HEAD~N..HEAD  # Where N covers implementation commits

   # Run comprehensive checks
   cd $(git rev-parse --show-toplevel) && make check test
   ```

## Validation Process

### Step 1: Context Discovery

If starting fresh or need more context:

1. **Read the implementation plan** completely
2. **Identify what should have changed**:

- List all files that should be modified
- Note all success criteria (automated and manual)
- Identify key functionality to verify

3. **Spawn parallel research tasks** to discover implementation:
   ```
   Task 1 - Verify database changes:
   Research if migration [N] was added and schema changes match plan.
   Check: migration files, schema version, table structure
   Return: What was implemented vs what plan specified

   Task 2 - Verify code changes:
   Find all modified files related to [feature].
   Compare actual changes to plan specifications.
   Return: File-by-file comparison of planned vs actual

   Task 3 - Verify test coverage:
   Check if tests were added/modified as specified.
   Run test commands and capture results.
   Return: Test status and any missing coverage
   ```

### Step 2: Systematic Validation

For each phase in the plan:

1. **Check completion status**:

- Look for checkmarks in the plan (- [x])
- Verify the actual code matches claimed completion

2. **Run automated verification**:

- Execute each command from "Automated Verification"
- Document pass/fail status
- If failures, investigate root cause

3. **Assess manual criteria**:

- List what needs manual testing
- Provide clear steps for user verification

4. **Think deeply about edge cases**:

- Were error conditions handled?
- Are there missing validations?
- Could the implementation break existing functionality?

### Step 3: Generate Validation Report

Compose the validation report body following the structure defined in
the Plan Validation Template at the top of this skill (Implementation
Status, Automated Verification Results, Code Review Findings, Manual
Testing Required, Recommendations).

### Step 4: Persist the Validation Report

Write the validation report to the configured validations directory:

1. Derive the filename from the plan filename: extract the filename stem
   (without directory path or `.md` extension) regardless of how the path
   was provided. For example, if the plan is
   `{plans directory}/2026-03-22-improve-error-handling.md`, the validation is
   `{validations directory}/2026-03-22-improve-error-handling-validation.md`.

2. Create the configured validations directory if it doesn't exist.

#### Populate frontmatter

The `target:` field is filled automatically from the plan reference under
validation — this is what makes the report traceable back to the plan it
covers. Per ADR-0034, the typed-linkage form is `"plan:<plan-id>"`.

Before writing the validation file, capture metadata and substitute the
unified base fields and per-type extras into the template's frontmatter
block:

1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
   to obtain `Current Date/Time (UTC):`.
2. **Substitute** every field below with the indicated value:
   - `type:` ← `plan-validation`
   - `id:` ← the validation filename stem (e.g.
     `2026-05-30-0065-update-artifact-templates-to-unified-schema-validation`),
     always quoted as a YAML string
   - `title:` ← `Validation Report: {plan title}`
   - `date:` ← the `Current Date/Time (UTC):` value
   - `author:` ← the author value resolved per `create-work-item/SKILL.md:578-580`
   - `producer:` ← `validate-plan`
   - `status:` ← `complete`
   - `last_updated:` ← the same `Current Date/Time (UTC):` value
   - `last_updated_by:` ← the same value resolved for `author`
   - `schema_version:` ← `1` (bare integer, not quoted)
   - `parent:` ← typed-linkage ref to the parent plan (`"plan:NNNN"`).
     Fill when the validation names a parent plan; otherwise omit the
     key.
   - `target:` ← `"plan:<plan-id>"` (e.g.
     `"plan:2026-05-30-0065-update-artifact-templates-to-unified-schema"`);
     the typed-linkage ref to the plan being validated, per ADR-0034.
     Always fill — every validation has a target.
   - `relates_to:` ← list of typed-linkage refs to related artifacts
     (`["plan-validation:NNNN", ...]`). Fill when related validations
     are explicit; otherwise omit the key.
   - `result:` ← `pass | partial | fail` per the Implementation
     Status of the report (see derivation rule below)
3. Write the file with the substituted frontmatter block followed by
   the validation report body from Step 3.

Determine the `result` field from the report:

- `pass`: all phases fully implemented, all automated checks pass
- `partial`: some phases implemented or some checks failing
- `fail`: major deviations or critical failures

4. If the validation result is `pass`, update the plan's frontmatter
`status` field to `done` (if the plan has YAML frontmatter with a
`status` field). This closes the plan lifecycle.

5. Inform the user where the report was saved:
```
Validation report saved to {validations directory}/{filename}.md
```

## Working with Existing Context

If you were part of the implementation:

- Review the conversation history
- Check your todo list for what was completed
- Focus validation on work done in this session
- Be honest about any shortcuts or incomplete items

## Important Guidelines

1. **Be thorough but practical** - Focus on what matters
2. **Run all automated checks** - Don't skip verification commands
3. **Document everything** - Both successes and issues
4. **Think critically** - Question if the implementation truly solves the
   problem
5. **Consider maintenance** - Will this be maintainable long-term?

## Validation Checklist

Always verify:

- [ ] All phases marked complete are actually done
- [ ] Automated tests pass
- [ ] Code follows existing patterns
- [ ] No regressions introduced
- [ ] Error handling is robust
- [ ] Documentation updated if needed
- [ ] Manual test steps are clear

## Relationship to Other Commands

Recommended workflow:

1. `/implement-plan` - Execute the implementation
2. `/commit` - Create atomic commits for changes
3. `/validate-plan` - Verify implementation correctness (saves report to
   the configured validations directory)
4. `/describe-pr` - Generate PR description

The validation works best after commits are made, as it can analyze the git
history to understand what was implemented.

Remember: Good validation catches issues before they reach production. Be
constructive but thorough in identifying gaps or improvements.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh validate-plan`
