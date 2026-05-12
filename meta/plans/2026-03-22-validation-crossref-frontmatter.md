# Validation Artifacts, Cross-Referencing, and Frontmatter Implementation Plan

## Overview

This plan covers the remaining meta/ enhancements identified in the research
at `meta/research/codebase/2026-03-18-meta-management-strategy.md`. It depends on Plan
1 (`meta/plans/2026-03-22-persist-review-artifacts.md`) being complete, since
the cross-referencing phases require review artifacts to exist at
`meta/reviews/`.

The three concerns addressed here are:

1. **Validation persistence**: `validate-plan` produces a structured validation
   report but only renders it in the conversation. Persisting it to
   `meta/validations/` completes the lifecycle audit trail.
2. **Cross-referencing**: `review-plan` and `respond-to-pr` can leverage
   persisted review artifacts to make better decisions.
3. **Frontmatter standardisation**: `create-plan` and `describe-pr` currently
   produce artifacts without YAML frontmatter, unlike `research-codebase` and
   the new review/validation artifacts.

## Current State Analysis

### validate-plan

- **File**: `skills/planning/validate-plan/SKILL.md` (196 lines)
- Prompt-only skill (`disable-model-invocation: true`)
- Step 3 (lines 98-150) generates a markdown validation report rendered in
  the conversation only
- The report contains: Implementation Status, Automated Verification Results,
  Code Review Findings (matches, deviations, issues), Manual Testing Required,
  and Recommendations
- No instructions to write to `meta/` or any persistent location
- No frontmatter template

### review-plan (cross-referencing gap)

- After Plan 1, `review-plan` writes review artifacts to
  `meta/reviews/plans/`. However, the skill doesn't yet know how to read a
  previous review when starting a re-review in a **new session** — Plan 1
  handles detection of prior reviews at the start of a fresh review cycle, but
  doesn't cover the case where `review-plan` is invoked for re-review and
  wants to load the prior review's findings for delta comparison.

  Actually, Plan 1 already covers this in Phase 1, change 2 (check for
  existing reviews in Step 1) and change 3 (re-review appends to the same
  file). The cross-referencing value here is about `review-plan` using prior
  reviews from **different review cycles** to inform its assessment — e.g.,
  noting that a finding from a previous cycle is still present.

### respond-to-pr (cross-referencing gap)

- **File**: `skills/github/respond-to-pr/SKILL.md` (506 lines)
- Step 1 fetches PR context (metadata, review threads, top-level reviews,
  issue comments) but does not check for a structured review artifact at
  `meta/reviews/prs/{number}-review-*.md`
- Step 2 triages feedback by category (Blocking, Simple, Complex, Question,
  Disagreement) without access to the original review's severity ratings,
  confidence levels, or cross-cutting analysis
- The skill would benefit from loading the review artifact to inform triage
  priorities and provide richer context for each feedback item

### create-plan (no frontmatter)

- **File**: `skills/planning/create-plan/SKILL.md`
- The plan template (lines 209-300) starts directly with `# [Feature/Task
  Name] Implementation Plan` — no YAML frontmatter
- Plans are stored at `meta/plans/YYYY-MM-DD-description.md`

### describe-pr (no frontmatter)

- **File**: `skills/github/describe-pr/SKILL.md`
- PR descriptions are written to `meta/prs/{number}-description.md` (line 81)
- No YAML frontmatter in the output

### Key Discoveries

- The validate-plan report template (lines 102-150) is already well-structured
  markdown — adding frontmatter and a file write step is straightforward
- respond-to-pr's Step 1 already fetches extensive context; adding one more
  file read is minimal overhead
- create-plan's template is a 4-backtick fenced block (line 209: `````markdown`)
  so adding frontmatter requires inserting it inside the template
- describe-pr's output goes both to `meta/prs/` and to GitHub via `gh pr edit`
  — frontmatter must not appear in the GitHub-posted version

## Desired End State

After this plan is complete:

1. `validate-plan` writes a validation report to
   `meta/validations/{plan-filename}-validation.md` with YAML frontmatter
   after generating the report (Step 3)
2. `respond-to-pr` checks for `meta/reviews/prs/{number}-review-*.md` during
   Step 1, loads the latest review cycle, and uses the structured review data
   to inform triage in Step 2
3. `create-plan` includes YAML frontmatter in its plan template
4. `describe-pr` includes YAML frontmatter in `meta/prs/` output (stripped
   before posting to GitHub)
5. The README meta/ directory table includes `validations/`
6. The documents-locator agent references `meta/validations/`

### Verification

- Running `/validate-plan @meta/plans/some-plan.md` produces a file at
  `meta/validations/some-plan-validation.md` with valid frontmatter
- Running `/respond-to-pr 123` where `meta/reviews/prs/123-review-1.md` exists
  shows the triage informed by the review's severity ratings
- Running `/create-plan` produces a plan with YAML frontmatter
- Running `/describe-pr 123` produces a description at `meta/prs/` with
  frontmatter, but the GitHub-posted version has frontmatter stripped

## What We're NOT Doing

- Not changing any review lens skills or the reviewer agent
- Not changing review output format schemas
- Not adding a new artifact type for respond-to-pr (GitHub threads are the
  persistent record; we're just reading the review artifact for context)
- Not retroactively adding frontmatter to existing plans or PR descriptions
- Not changing how implement-plan reads plans (frontmatter is ignored by
  markdown parsers and won't affect plan execution)

## Implementation Approach

Four independent phases, ordered by value:

1. Validation persistence (completes the lifecycle audit trail)
2. respond-to-pr cross-referencing (leverages Plan 1's PR review artifacts)
3. create-plan frontmatter (low-risk template change)
4. describe-pr frontmatter (slightly more complex due to GitHub posting)

### Frontmatter Convention

All meta/ artifacts that include YAML frontmatter share a common base set of
fields. Each artifact type may add type-specific fields beyond the base.

**Common base fields** (required on all artifact types):

| Field    | Description                        | Example                   |
|----------|------------------------------------|---------------------------|
| `date`   | ISO timestamp of artifact creation | `"2026-03-22T14:30:00Z"`  |
| `type`   | Artifact type identifier           | `plan`, `plan-validation` |
| `skill`  | Skill that produced the artifact   | `create-plan`             |
| `status` | Lifecycle status of the artifact   | `draft`, `complete`       |

**Type-specific extensions**: Each artifact type adds fields relevant to its
purpose (e.g., `target` and `result` for validations, `ticket` for plans,
`pr_number` for PR descriptions). The existing research-codebase frontmatter
predates this convention and uses a different field set (`researcher`,
`git_commit`, `branch`, `repository`, `topic`, `tags`); retrofitting it is
out of scope for this plan but should be considered in future work.

---

## Phase 1: Persist Validation Reports

### Overview

Add a file write step to `validate-plan` so the validation report is
persisted to `meta/validations/` alongside the conversation output. This
lets teammates see validation results without needing the original session.

### Changes Required

#### 1. Add a write step after Step 3

**File**: `skills/planning/validate-plan/SKILL.md`

After Step 3 ("Generate Validation Report", line 98), add a new step that
writes the report to disk. Insert after line 150 (end of the report template)
and before line 152 ("Working with Existing Context"):

```markdown
### Step 4: Persist the Validation Report

Write the validation report to `meta/validations/`:

1. Derive the filename from the plan filename: extract the filename stem
   (without directory path or `.md` extension) regardless of how the path
   was provided. For example, if the plan is
   `meta/plans/2026-03-22-improve-error-handling.md`, the validation is
   `meta/validations/2026-03-22-improve-error-handling-validation.md`.

2. Create the directory if it doesn't exist:
   ```bash
   mkdir -p meta/validations
   ```

3. Write the validation document with YAML frontmatter followed by the
   report from Step 3:

   ```markdown
   ---
   date: "{ISO timestamp}"
   type: plan-validation
   skill: validate-plan
   target: "meta/plans/{plan-filename}.md"
   result: {pass | partial | fail}
   status: complete
   ---

   {The full validation report from Step 3}
   ```

   Determine the `result` field from the report:

- `pass`: all phases fully implemented, all automated checks pass
- `partial`: some phases implemented or some checks failing
- `fail`: major deviations or critical failures

4. If the validation result is `pass`, update the plan's frontmatter
   `status` field to `complete` (if the plan has YAML frontmatter with a
   `status` field). This closes the plan lifecycle.

5. Inform the user where the report was saved:
   ```
   Validation report saved to meta/validations/{filename}.md
   ```

```

#### 2. Update the "Relationship to Other Commands" section

**File**: `skills/planning/validate-plan/SKILL.md`

Update the workflow list (lines 186-189) to note the validation artifact:

```markdown
1. `/implement-plan` - Execute the implementation
2. `/commit` - Create atomic commits for changes
3. `/validate-plan` - Verify implementation correctness (saves report to
   `meta/validations/`)
4. `/describe-pr` - Generate PR description
```

#### 3. Update README meta/ directory table

**File**: `README.md`

Add a `validations/` row to the table (after the `reviews/` row added by
Plan 1):

```markdown
| `validations/` | Plan validation reports |
`validate-plan`                            |
```

The full table becomes:

```markdown
| Directory      | Purpose                                         | Written by                                 |
|----------------|-------------------------------------------------|--------------------------------------------|
| `research/`    | Research findings with YAML frontmatter         | `research-codebase`                        |
| `plans/`       | Implementation plans with phased changes        | `create-plan`                              |
| `decisions/`   | Architecture decision records (ADRs)            | `create-adr`, `extract-adrs`, `review-adr` |
| `reviews/`     | Review summaries and per-lens results           | `review-pr`, `review-plan`                 |
| `validations/` | Plan validation reports                         | `validate-plan`                            |
| `prs/`         | PR descriptions                                 | `describe-pr`                              |
| `templates/`   | Reusable templates (e.g., PR descriptions)      | manual                                     |
| `tmp/`         | Ephemeral working data (e.g., review artifacts) | `review-pr`                                |
```

#### 4. Update documents-locator agent

**File**: `agents/documents-locator.md`

**Note**: These changes assume Plan 1's documents-locator updates (adding
`meta/reviews/`) are already in place. The `reviews/` line in the directory
diagram below should already be present from Plan 1. Apply validations/
entries after the reviews/ entries for logical consistency.

Add `meta/validations/` to the core responsibilities list (around line 18):

```markdown
- Check meta/validations/ for plan validation reports
```

Add to the directory structure diagram (around line 44):

```
meta/
├── research/     # Research documents
├── plans/        # Implementation plans
├── reviews/      # Review artifacts (plan and PR reviews)
├── validations/  # Plan validation reports
├── decisions/    # Technical and architectural decisions
├── tickets/      # Ticket documentation
├── prs/          # PR descriptions
├── notes/        # General notes
└── global/       # Cross-repository thoughts
```

Add "Validations" to the categorisation list (around line 27):

```markdown
- Validations (in validations/ — plan validation reports)
```

Add a validations section to the output format example (around line 80):

```markdown
### Validations

- `meta/validations/2026-03-22-improve-error-handling-validation.md` -
  Validation of error handling plan (result: partial)
```

### Success Criteria

#### Automated Verification

- [x] `skills/planning/validate-plan/SKILL.md` contains instructions to write
  to `meta/validations/`
- [x] `skills/planning/validate-plan/SKILL.md` contains frontmatter schema
  with `type: plan-validation`, `target`, `result`, and `status` fields
- [x] `README.md` contains a `validations/` row in the meta/ directory table
- [x] `agents/documents-locator.md` references `meta/validations/`

#### Manual Verification

- [ ] Running `/validate-plan` on a plan produces a validation file at the
  expected path with valid frontmatter and content
- [ ] The `result` field correctly reflects pass/partial/fail
- [ ] The validation document is readable and useful to a teammate

---

## Phase 2: Cross-Reference Reviews in `respond-to-pr`

### Overview

When `respond-to-pr` addresses review feedback, it benefits from knowing the
structured analysis behind the review — severity ratings, confidence levels,
cross-cutting themes, and the lens that produced each finding. This phase
teaches `respond-to-pr` to load the review artifact if one exists.

### Changes Required

#### 1. Add review artifact loading to Step 1

**File**: `skills/github/respond-to-pr/SKILL.md`

After Step 1.4 ("Ensure on the correct branch", lines 64-67), add a new
sub-step:

```markdown
5. **Check for a structured review artifact**:

   Look for `meta/reviews/prs/{number}-review-*.md` (e.g.,
   `123-review-1.md`, `123-review-2.md`). Plan 1 produces review files
   with an incrementing review cycle suffix. If one or more files match:

- Load the highest-numbered file (the most recent review cycle)
- Read the review document
- Extract the verdict, lenses used, and per-lens findings
- Note the severity and confidence of each finding
- This will inform the triage in Step 2

If no matching files are found, proceed without review context — the
skill works the same as before, just without the additional context.
```

Renumber the subsequent sub-steps (current 5 becomes 6, 6 becomes 7,
7 becomes 8).

#### 2. Enhance Step 2 triage with review context

**File**: `skills/github/respond-to-pr/SKILL.md`

In Step 2.2 ("Categorise each piece of feedback", lines 154-162), add
guidance for using the review artifact:

```markdown
   If a structured review artifact was loaded in Step 1, cross-reference
GitHub review comments against the review's findings. For each comment
that matches a finding from the review:

- Use the finding's `severity` to inform the category (critical severity
  → Blocking; major affecting multiple files or requiring design decisions
  → Complex; major affecting a single file with a clear fix → Simple)
- Note the `confidence` level — low-confidence findings may warrant
  more careful verification in Step 4a
- Note the `lens` — this provides context about what quality dimension
  the finding addresses
- If the review's cross-cutting themes are relevant, mention them when
  presenting the item to the user in Step 3
```

#### 3. Add a note to the feedback summary

**File**: `skills/github/respond-to-pr/SKILL.md`

In Step 3 ("Present Feedback Summary", lines 181-234), add a note when
review context is available:

```markdown
If a review artifact was loaded, add a note after the header:

```

> Review context loaded from `meta/reviews/prs/{number}-review-{N}.md`
> (verdict: {verdict}, {N} lenses, {date})

```
```

### Success Criteria

#### Automated Verification

- [x] `skills/github/respond-to-pr/SKILL.md` contains instructions to check
  for `meta/reviews/prs/{number}-review-*.md` in Step 1
- [x] The file contains guidance for using review severity/confidence in
  triage categorisation

#### Manual Verification

- [ ] Running `/respond-to-pr` on a PR with a review artifact shows the
  review context note in the feedback summary
- [ ] Triage categories are informed by the review's severity ratings
- [ ] Running `/respond-to-pr` on a PR without a review artifact works
  exactly as before (no errors, no missing context messages)

---

## Phase 3: Add Frontmatter to `create-plan`

### Overview

Plans are the most-consumed artifact type (read by `implement-plan`,
`review-plan`, `validate-plan`) but currently lack YAML frontmatter. Adding
it makes plans machine-parseable and consistent with research documents and
the new review/validation artifacts.

### Changes Required

#### 1. Add frontmatter to the plan template

**File**: `skills/planning/create-plan/SKILL.md`

In Step 4.2, update the template structure (line 209 onwards) to include
YAML frontmatter at the top:

````markdown
```markdown
---
date: "{ISO timestamp}"
type: plan
skill: create-plan
ticket: "{ticket reference, if any}"
status: draft
---

# [Feature/Task Name] Implementation Plan

## Overview
...
```
````

The `status` field starts as `draft`. When `validate-plan` produces a
passing validation (result: `pass`), it should update the plan's `status`
field to `complete` (see Phase 1, Step 4 above). Other transitions
(`draft` → `ready`, `draft` → `in-progress`) are available for manual use
but are not automated by any skill in this plan.

The `ticket` field is empty string if no ticket was provided.

#### 2. Update the plan filename documentation

**File**: `skills/planning/create-plan/SKILL.md`

No change needed to the filename pattern — it remains
`YYYY-MM-DD-ENG-XXXX-description.md`. The frontmatter is additional
metadata inside the file.

### Success Criteria

#### Automated Verification

- [x] `skills/planning/create-plan/SKILL.md` contains a YAML frontmatter
  block in the plan template with `date`, `type`, `skill`, `ticket`,
  and `status` fields

#### Manual Verification

- [ ] Running `/create-plan` produces a plan with valid YAML frontmatter
- [ ] `implement-plan` still reads and executes plans correctly (frontmatter
  doesn't interfere with markdown rendering)
- [ ] `review-plan` still reads plans correctly

---

## Phase 4: Add Frontmatter to `describe-pr`

### Overview

PR descriptions are written to `meta/prs/{number}-description.md` and also
posted to GitHub via `gh pr edit --body-file`. YAML frontmatter must be
included in the local file but stripped before posting to GitHub, since
GitHub would render it as visible text.

### Changes Required

#### 1. Add frontmatter to the description output

**File**: `skills/github/describe-pr/SKILL.md`

In step 8 ("Save and show the description", line 79), update the
instructions to include frontmatter:

```markdown
8. **Save and show the description:**

- Write the completed description to `meta/prs/{number}-description.md`
  with YAML frontmatter:

  ```markdown
  ---
  date: "{ISO timestamp}"
  type: pr-description
  skill: describe-pr
  pr_number: {number}
  pr_title: "{title}"
  status: complete
  ---

  {The generated PR description}
  ```

- Show the user the generated description (without frontmatter — they'll
  see what gets posted to GitHub)
- On re-run (when `meta/prs/{number}-description.md` already exists),
  regenerate the frontmatter with an updated `date` timestamp. The
  existing step 3 already handles reading the prior description for
  context; the frontmatter is simply regenerated fresh.

```

#### 2. Update the GitHub posting step to strip frontmatter

**File**: `skills/github/describe-pr/SKILL.md`

In step 9 ("Update the PR", line 84), update the instructions:

```markdown
9. **Update the PR:**

- The `meta/prs/{number}-description.md` file contains YAML frontmatter
  that should not appear on GitHub. Before posting, strip the frontmatter
  block from the start of the file:
  1. Read the file content
  2. The frontmatter block starts with `---` on line 1 and ends at the
     next `---` line (which closes the YAML block). Only match the
     opening frontmatter block — do not match `---` lines that appear
     later in the body (e.g., markdown horizontal rules).
  3. Write everything after the closing `---` line to a temporary file
  4. Post with `gh pr edit {number} --body-file /tmp/pr-body-{number}.md`
  5. Clean up the temporary file
- Confirm the update was successful
- If any verification steps remain unchecked, remind the user to complete
  them before merging
```

### Success Criteria

#### Automated Verification

- [x] `skills/github/describe-pr/SKILL.md` contains instructions to write
  YAML frontmatter to `meta/prs/{number}-description.md`
- [x] The file contains instructions to strip frontmatter before posting to
  GitHub

#### Manual Verification

- [ ] Running `/describe-pr` produces a description file with valid YAML
  frontmatter
- [ ] The GitHub PR description does NOT contain frontmatter
- [ ] Re-running `/describe-pr` on the same PR correctly reads the existing
  description (including its frontmatter)

---

## Testing Strategy

### Integration Testing

Since these are prompt-only skill files (no executable code), testing is
manual:

1. Run `/validate-plan` on a previously-implemented plan and verify the
   validation report is written correctly
2. Run `/respond-to-pr` on a PR that has a review artifact — verify triage
   is informed by the review
3. Run `/respond-to-pr` on a PR without a review artifact — verify it works
   as before
4. Run `/create-plan` and verify the plan includes frontmatter
5. Run `/describe-pr` and verify frontmatter in local file, absence in GitHub

### Edge Cases

- validate-plan in an existing conversation (where implementation just
  happened): should still write the report to disk
- respond-to-pr where the review artifact is from a different review cycle
  (older review): should still be useful context
- create-plan with and without a ticket reference: `ticket` field should
  handle both cases
- describe-pr re-run: should read existing file including frontmatter and
  update correctly

## Performance Considerations

- Phase 2 adds one file read to respond-to-pr's Step 1. This is negligible
  compared to the multiple GitHub API calls already made.
- Frontmatter adds a few lines to each artifact. No meaningful size impact.

## References

- Research document: `meta/research/codebase/2026-03-18-meta-management-strategy.md`
- Plan 1 (prerequisite): `meta/plans/2026-03-22-persist-review-artifacts.md`
- validate-plan skill: `skills/planning/validate-plan/SKILL.md`
- respond-to-pr skill: `skills/github/respond-to-pr/SKILL.md`
- create-plan skill: `skills/planning/create-plan/SKILL.md`
- describe-pr skill: `skills/github/describe-pr/SKILL.md`
- Documents-locator agent: `agents/documents-locator.md`
- README: `README.md`
