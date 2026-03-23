# Persist Review Artifacts Implementation Plan

## Overview

The `review-pr` and `review-plan` skills produce rich, structured review
outputs (per-lens results, aggregated summaries, verdicts) but discard them
after the conversation ends. This means teammates who didn't run the review
have no visibility into the findings, and re-reviews in new sessions can't
reference prior results. This plan adds persistent review artifacts to
`meta/reviews/`, extending the existing filesystem-as-shared-memory pattern
that `research-codebase`, `create-plan`, and `create-adr` already follow.

## Current State Analysis

### What exists now

- **review-plan** (`skills/planning/review-plan/SKILL.md`): Spawns parallel
  lens agents, aggregates their JSON outputs into a structured review summary,
  presents it in the conversation, then iterates on plan edits. Per-lens
  results and the aggregated summary exist only in the conversation context.
  Line 457 explicitly says: "Don't write review findings to a separate file."
  Re-reviews (Step 7) compare previous and new findings by `title` + `lens`
  but only within the same conversation.

- **review-pr** (`skills/github/review-pr/SKILL.md`): Similar fan-out/fan-in
  pattern. Writes ephemeral working data to `meta/tmp/pr-review-{number}/`
  (diff, changed-files, PR description, commits, head SHA, repo info, review
  payload JSON). The review summary and per-lens results are consumed in-memory
  during aggregation (Step 4) and then posted to GitHub. Line 527 explicitly
  says: "Don't write review findings to a separate file."

- **README.md** (lines 76-83): Documents the meta/ directory table. No
  `reviews/` entry exists.

- **documents-locator agent** (`agents/documents-locator.md`): Lists expected
  meta/ subdirectories (lines 15-18, 44-53). No `reviews/` entry exists.

### Key Discoveries

- The review summary template in review-plan (lines 283-335) and review-pr
  (lines 345-375) already produces well-structured markdown — we can write
  this directly to disk with minimal changes.
- The re-review template in review-plan (lines 396-409) is a compact
  delta-focused format that fits naturally as an appended section.
- review-pr's `meta/tmp/` directory is already gitignored (`.gitignore:8`),
  so the ephemeral working data stays ephemeral.
- Both skills already have a clear boundary between "compose the review" and
  "present/post the review" — the write step slots in between.

## Desired End State

After this plan is complete:

1. `review-plan` writes a review document to
   `meta/reviews/plans/{plan-stem}-review-{N}.md` after aggregation (Step 4),
   before presenting (Step 5). Re-reviews (Step 7) append a dated section to
   the same file. A new review cycle in a new session creates a new file with
   an incremented review number — no files are ever replaced or deleted.
2. `review-pr` writes a review document to
   `meta/reviews/prs/{number}-review-{N}.md` after aggregation (Step 4),
   before presenting (Step 5). Subsequent reviews of the same PR create new
   numbered files.
3. Both review documents contain YAML frontmatter, the full review summary,
   and per-lens results in readable markdown.
4. `review-plan` checks for existing review documents when starting a fresh
   review of a previously-reviewed plan, enabling cross-session continuity.
5. The README and documents-locator agent reference `meta/reviews/`.
6. `meta/tmp/` remains purely ephemeral — review artifacts live in
   `meta/reviews/` instead.

### Review Numbering Scheme

Two orthogonal counters track review history:

- **Review number** (in filename): The cross-cycle counter. Each independent
  review of a plan or PR gets its own numbered file (`-review-1.md`,
  `-review-2.md`, etc.). Files are never replaced — the full history is
  preserved on disk.
- **`review_pass`** (in frontmatter): The within-cycle counter for re-reviews
  (Step 7 in review-plan). Starts at 1 for the initial review, increments
  each time a re-review is appended to the same file.

Example lifecycle:

```
Session 1: /review-plan @meta/plans/foo.md
  → writes meta/reviews/plans/foo-review-1.md (review_pass: 1)
  → user edits plan, re-reviews
  → appends re-review section to foo-review-1.md (review_pass: 2)

Session 2: /review-plan @meta/plans/foo.md (plan has changed)
  → finds foo-review-1.md, reads it for context
  → writes meta/reviews/plans/foo-review-2.md (review_pass: 1)
```

### Filename Derivation

Review filenames are derived from the target as follows:

- **Plan reviews**: Extract the basename of the plan path without the `.md`
  extension (the "plan stem"). For example, if the plan is at
  `meta/plans/2026-03-22-improve-error-handling.md`, the stem is
  `2026-03-22-improve-error-handling`. The review filename is
  `{stem}-review-{N}.md` where N is the next available review number.
- **PR reviews**: The review filename is `{pr-number}-review-{N}.md`.

To determine the next review number, glob for existing files matching
`{stem}-review-*.md` (or `{number}-review-*.md` for PRs), extract the
highest existing number, and increment by 1. If no files exist, use 1.

### Verification

- Running `/review-plan @meta/plans/some-plan.md` produces a file at
  `meta/reviews/plans/some-plan-review-1.md` with valid frontmatter and
  structured content.
- Running `/review-pr 123` produces a file at
  `meta/reviews/prs/123-review-1.md` with valid frontmatter and structured
  content.
- Re-reviewing a plan appends a re-review section to the existing review file
  and updates the frontmatter `review_pass`.
- Starting a new review cycle of a previously-reviewed plan creates a new
  numbered file (e.g., `-review-2.md`) and references the prior review.
- The documents-locator agent lists `meta/reviews/` when searching for review
  documents.

## What We're NOT Doing

- Not adding `meta/validations/` (separate plan)
- Not updating `respond-to-pr` to cross-reference reviews (separate plan)
- Not standardising frontmatter on existing skills like `create-plan` or
  `describe-pr` (separate plan)
- Not changing how review-pr posts to GitHub — the review still gets posted
  exactly as before; we're adding persistence alongside it
- Not changing any lens skills or the reviewer agent
- Not changing the review output format schemas

## Implementation Approach

The changes are additive: we insert a "write to disk" step between the
existing "compose" and "present" steps in each skill. The review content
itself doesn't change — we're persisting what's already being composed.

For review-plan, we also modify Step 1 to check for prior reviews and Step 7
to append re-review sections rather than composing them only in conversation.

**Directory convention note**: `meta/reviews/` uses a nested structure
(`reviews/plans/`, `reviews/prs/`) unlike the flat convention elsewhere in
`meta/`. This is an intentional departure because reviews are a cross-cutting
concept that applies to two different artifact types. The nesting keeps plan
reviews and PR reviews organised without filename-prefix collisions.

---

## Phase 1: Update `review-plan` to Persist Review Artifacts

### Overview

This is the highest-value change. Plan reviews currently leave no trace on
disk, making them invisible to teammates and lost across sessions. We add a
write step after aggregation, teach re-reviews to append to the same file,
and teach new reviews to discover prior review documents.

### Changes Required

#### 1. Add a write step between Step 4 and Step 5

**File**: `skills/planning/review-plan/SKILL.md`

After Step 4.7 ("Compose the review summary"), add a new sub-step 4.8:

```markdown
8. **Write the review artifact** to `meta/reviews/plans/`:

   Derive the review filename using the plan stem and the next available
   review number (see "Filename Derivation" in the Desired End State
   section). For example, if the plan is
   `meta/plans/2026-03-22-improve-error-handling.md` and no prior reviews
   exist, the review filename is
   `meta/reviews/plans/2026-03-22-improve-error-handling-review-1.md`.

   To determine the next review number:
   ```bash
   mkdir -p meta/reviews/plans
   # Glob for existing reviews of this plan
   ls meta/reviews/plans/{plan-stem}-review-*.md 2>/dev/null
   # Extract the highest number, increment by 1. If none exist, use 1.
   ```

Write the review document with YAML frontmatter followed by the review
summary composed in Step 4.7. Include the per-lens results as a final
section:

   ```markdown
   ---
date: "{ISO timestamp}"
type: plan-review
skill: review-plan
target: "meta/plans/{plan-stem}.md"
review_number: {N}
verdict: {APPROVE | REVISE | COMMENT}
lenses: [{list of lenses used}]
review_pass: 1
status: complete
   ---

{The full review summary from Step 4.7}

## Per-Lens Results

### {Lens 1 Name}

**Summary**: {agent summary}

**Strengths**:
{agent strengths}

**Findings**:
{agent findings — each with severity, confidence, location, and body}

### {Lens 2 Name}

...
   ```

The per-lens results section contains the full content from each agent's
JSON output, converted to readable markdown. This preserves the complete
analysis for future reference while keeping it human-readable.

```

#### 2. Update Step 1 to check for existing reviews

**File**: `skills/planning/review-plan/SKILL.md`

After reading the plan (Step 1.2), add a new sub-step:

```markdown
4. **Check for existing reviews**: Glob for review documents matching
   `meta/reviews/plans/{plan-stem}-review-*.md`. If any are found:
   - Read the most recent review document (highest review number) to
     understand what was previously reviewed
   - Note the previous verdict, review pass count, and key findings
   - Inform the user: "I found {N} previous review(s) of this plan. The
     most recent (review {N}, verdict: {verdict}) will be used as context."
   - The agents do NOT receive the previous review — they review the plan
     fresh. But the aggregation step (Step 4) should reference the previous
     review when composing cross-cutting themes and the assessment:
     specifically, note which findings from the previous review recur in
     the new review and which appear to have been addressed by plan changes.
   - If the prior review file exists but cannot be parsed (e.g., malformed
     frontmatter from a partial write), warn the user and proceed as if no
     prior review exists.

   The new review creates a **new file** with the next review number (e.g.,
   `-review-2.md`). Previous review files are never modified or deleted —
   the full review history is preserved on disk.
```

#### 3. Update Step 7 (re-review) to append to the review file

**File**: `skills/planning/review-plan/SKILL.md`

After the existing re-review presentation (Step 7), add instructions to
update the review document:

```markdown
After composing the re-review summary, **update the review artifact**
as a single write operation:

1. Read the full content of the existing review document at
   `meta/reviews/plans/{plan-stem}-review-{N}.md`
2. In memory, update exactly three frontmatter fields — `verdict`,
   `review_pass`, and `date` — preserving all other fields and body
   content verbatim
3. Append the re-review section at the end of the content (after the
   Per-Lens Results section)
4. Write the complete modified content back to the same file in one
   operation

The frontmatter's `verdict` and `review_pass` fields reflect the
latest re-review state (not the initial review state), so readers can
check the current status without scrolling:

   ```markdown

   ## Re-Review (Pass {N}) — {date}

   **Verdict:** {verdict}

   ### Previously Identified Issues
   - {emoji} **{Lens}**: {title} — {Resolved | Partially resolved | Still present}
   - ...

   ### New Issues Introduced
   - {emoji} **{Lens}**: {title} — {brief description}

   ### Assessment
   {Whether the plan is now in good shape or needs further iteration}
   ```

The document reads chronologically: initial review, per-lens results,
then re-review sections in order. The frontmatter always reflects the
latest verdict and pass count.

```

#### 4. Update the "What NOT to Do" section

**File**: `skills/planning/review-plan/SKILL.md`

Replace line 457:
```

- Don't write review findings to a separate file — all output goes to the
  conversation and then into plan edits

```

With:
```

- Don't skip writing the review artifact — always persist to
  meta/reviews/plans/ so the review is visible to the team

```

### Success Criteria

#### Automated Verification

- [x] The file `skills/planning/review-plan/SKILL.md` contains instructions
      to write to `meta/reviews/plans/`
- [x] The file no longer contains "Don't write review findings to a separate
      file"
- [x] The file contains frontmatter schema with `type: plan-review`, `target`,
      `review_number`, `verdict`, `lenses`, `review_pass`, and `status` fields
- [x] The file contains instructions for re-reviews to append to the existing
      review document
- [x] The file contains instructions to glob for existing reviews and
      determine the next review number

#### Manual Verification

- [ ] Running `/review-plan` on a plan produces a review file at the expected
      path (e.g., `{stem}-review-1.md`) with valid frontmatter and content
- [ ] Re-reviewing the same plan appends a re-review section and updates
      `review_pass` in frontmatter
- [ ] Starting a new review cycle of a previously-reviewed plan creates a new
      numbered file (e.g., `-review-2.md`) and references the prior review
- [ ] Previous review files are preserved on disk (never replaced or deleted)
- [ ] The review document is readable and useful to a teammate who didn't
      run the review

---

## Phase 2: Update `review-pr` to Persist Review Artifacts

### Overview

PR reviews write ephemeral data to `meta/tmp/` and post to GitHub, but the
structured analysis (per-lens results, severity ratings, cross-cutting themes)
is lost. We add a persistent review artifact to `meta/reviews/prs/` while
keeping the existing GitHub posting flow unchanged.

PR reviews use the same numbered-file scheme as plan reviews: each review
creates a new file (`{number}-review-1.md`, `{number}-review-2.md`, etc.)
and no files are ever replaced. However, PR reviews do not have within-file
re-reviews (no `review_pass` incrementing) because the review-pr skill has
no re-review step — PR reviews are one-shot, posted to GitHub, and iterated
via `respond-to-pr` rather than re-review. If a PR is reviewed again (e.g.,
after a force-push), it creates a new numbered review file.

### Changes Required

#### 1. Add a write step between Step 4 and Step 5

**File**: `skills/github/review-pr/SKILL.md`

After Step 4.8 ("Compose the review summary body") and Step 4.9 ("Compose
each inline comment body"), add a new sub-step 4.10:

```markdown
10. **Write the review artifact** to `meta/reviews/prs/`:

    Determine the next review number:
    ```bash
    mkdir -p meta/reviews/prs
    # Glob for existing reviews of this PR
    ls meta/reviews/prs/{number}-review-*.md 2>/dev/null
    # Extract the highest number, increment by 1. If none exist, use 1.
    ```

    Write the review document to `meta/reviews/prs/{number}-review-{N}.md`:

    ```markdown
    ---
    date: "{ISO timestamp}"
    type: pr-review
    skill: review-pr
    target: "PR #{number}"
    pr_number: {number}
    pr_title: "{title}"
    review_number: {N}
    verdict: {APPROVE | REQUEST_CHANGES | COMMENT}
    lenses: [{list of lenses used}]
    status: complete
    ---

    {The full review summary from Step 4.8}

    ## Inline Comments

    ### `{path}:{line}` — {title}
    **Severity**: {severity} | **Confidence**: {confidence} | **Lens**: {lens}

    {comment body}

    ---

    ### `{path}:{line}` — {title}
    ...

    ## Per-Lens Results

    ### {Lens 1 Name}

    **Summary**: {agent summary}

    **Strengths**:
    {agent strengths}

    **Comments**:
    {agent comments — each with path, line, severity, confidence, and body}

    **General Findings**:
    {agent general findings}

    ### {Lens 2 Name}

    ...
    ```

    This review artifact captures the complete analysis. The GitHub review
    (posted in Step 6) may be a curated subset (capped at ~10 inline
    comments), but the persistent artifact retains everything.
```

#### 2. Update the "What NOT to Do" section

**File**: `skills/github/review-pr/SKILL.md`

Replace line 527:

```
- Don't write review findings to a separate file — all output goes to the
  conversation and then to GitHub via the API
```

With:

```
- Don't skip writing the review artifact — always persist to
  meta/reviews/prs/ so the full analysis is available to the team
```

#### 3. Add a note about tmp/ vs reviews/ distinction

**File**: `skills/github/review-pr/SKILL.md`

In the guideline about temp directory cleanup (guideline 7, around line 509),
add a clarifying note:

```markdown
   The `meta/tmp/pr-review-{number}/` directory contains ephemeral working
data (diff, changed-files, PR description, commits, head SHA, repo info,
review payload JSON) used during the review session. The review itself
(summary, inline comments, per-lens results) is persisted separately to
`meta/reviews/prs/{number}-review-{N}.md` and is NOT stored in tmp/.
```

### Success Criteria

#### Automated Verification

- [x] The file `skills/github/review-pr/SKILL.md` contains instructions
  to write to `meta/reviews/prs/`
- [x] The file no longer contains "Don't write review findings to a separate
  file"
- [x] The file contains frontmatter schema with `type: pr-review`,
  `pr_number`, `review_number`, `verdict`, `lenses`, and `status` fields
- [x] The file contains a note clarifying the tmp/ vs reviews/ distinction
- [x] The file contains instructions to glob for existing reviews and
  determine the next review number

#### Manual Verification

- [ ] Running `/review-pr` on a PR produces a review file at the expected
  path (e.g., `123-review-1.md`) with valid frontmatter and content
- [ ] Reviewing the same PR again creates a new numbered file (e.g.,
  `123-review-2.md`) — the previous file is preserved
- [ ] The review file contains the full inline comments (not just the ~10
  posted to GitHub)
- [ ] The review file contains per-lens results with full agent output
- [ ] The existing GitHub posting flow still works correctly
- [ ] The review document is readable and useful to a teammate who didn't
  run the review

---

## Phase 3: Update Supporting Files

### Overview

Update the README meta/ directory table and the documents-locator agent to
reflect the new `meta/reviews/` directory.

### Changes Required

#### 1. Update README meta/ directory table

**File**: `README.md`

Add a `reviews/` row to the table at lines 76-83. The updated table:

```markdown
| Directory    | Purpose                                         | Written by                                 |
|--------------|-------------------------------------------------|--------------------------------------------|
| `research/`  | Research findings with YAML frontmatter         | `research-codebase`                        |
| `plans/`     | Implementation plans with phased changes        | `create-plan`                              |
| `decisions/` | Architecture decision records (ADRs)            | `create-adr`, `extract-adrs`, `review-adr` |
| `reviews/`   | Review summaries and per-lens results           | `review-pr`, `review-plan`                 |
| `prs/`       | PR descriptions                                 | `describe-pr`                              |
| `templates/` | Reusable templates (e.g., PR descriptions)      | manual                                     |
| `tmp/`       | Ephemeral working data (e.g., diffs, PR metadata) | `review-pr`                              |
```

#### 2. Update documents-locator agent directory list

**File**: `agents/documents-locator.md`

Add `meta/reviews/` to the core responsibilities list (around line 15):

```markdown
- Check meta/reviews/ for review artifacts (plan reviews and PR reviews)
```

Add to the directory structure diagram (around line 44):

```
meta/
├── research/  # Research documents
├── plans/     # Implementation plans
├── reviews/   # Review artifacts (plan and PR reviews)
├── decisions/ # Technical and architectural decisions
├── tickets/   # Ticket documentation
├── prs/       # PR descriptions
├── notes/     # General notes
└── global/    # Cross-repository thoughts
```

Add "Reviews" to the categorisation list (around line 27):

```markdown
- Review artifacts (in reviews/)
```

Add a reviews section to the output format example (around line 80):

```markdown
### Reviews

- `meta/reviews/plans/2026-03-22-improve-error-handling-review-1.md` - Review of
  error handling plan (review 1, verdict: REVISE)
- `meta/reviews/prs/456-review-1.md` - Review of PR #456 (review 1, verdict:
  COMMENT)
```

### Success Criteria

#### Automated Verification

- [x] `README.md` contains a `reviews/` row in the meta/ directory table
- [x] `agents/documents-locator.md` references `meta/reviews/` in its
  directory list, structure diagram, and output format example

#### Manual Verification

- [ ] The README table is correctly formatted with aligned columns
- [ ] The documents-locator agent, when asked to find reviews, would know
  to look in `meta/reviews/`

---

## Testing Strategy

### Integration Testing

Since these are prompt-only skill files (no executable code), testing is
manual:

1. Run `/review-plan` on an existing plan and verify the review artifact is
   written as `{stem}-review-1.md`
2. Re-review the same plan and verify the re-review section is appended to
   the same file with `review_pass` incremented
3. Start a new session and review the same plan again — verify a new file
   `{stem}-review-2.md` is created and the prior review is referenced
4. Verify the previous review file (`-review-1.md`) is preserved unchanged
5. Run `/review-pr` on a PR and verify the review artifact is written as
   `{number}-review-1.md`
6. Review the same PR again and verify a new file `{number}-review-2.md` is
   created
7. Verify the GitHub posting still works as before

### Edge Cases

- Plan with no findings (APPROVE verdict): review file should still be
  written with empty findings sections
- Re-review where all findings are resolved: re-review section should show
  all items as "Resolved"
- Multiple re-reviews in one session: each should append and increment
  `review_pass` within the same numbered file
- New review of a plan with multiple prior reviews: should find the highest
  review number and create the next one
- PR reviewed multiple times: each review creates a new numbered file
- Filename derivation: uses basename without `.md` extension, regardless of
  how the plan path was specified (relative, absolute, with `./` prefix)

## References

- Research document: `meta/research/2026-03-18-meta-management-strategy.md`
- review-plan skill: `skills/planning/review-plan/SKILL.md`
- review-pr skill: `skills/github/review-pr/SKILL.md`
- Plan review output format:
  `skills/review/output-formats/plan-review-output-format/SKILL.md`
- PR review output format:
  `skills/review/output-formats/pr-review-output-format/SKILL.md`
- Reviewer agent: `agents/reviewer.md`
- Documents-locator agent: `agents/documents-locator.md`
- README: `README.md`
