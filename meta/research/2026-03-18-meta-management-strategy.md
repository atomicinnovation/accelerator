---
date: "2026-03-18T02:46:16+00:00"
researcher: Toby Clemson (via Claude)
git_commit: 2bfac71efe3c5cd83ea1fa3b48b69fa805c4919f
branch: improve-meta
repository: accelerator
topic: "Meta directory management strategy: preserving review artifacts and improving consistency"
tags: [ research, meta, review-pr, review-plan, skills, architecture ]
status: complete
last_updated: "2026-03-18"
last_updated_by: Toby Clemson (via Claude)
---

# Research: Meta Directory Management Strategy

**Date**: 2026-03-18T02:46:16+00:00
**Researcher**: Toby Clemson (via Claude)
**Git Commit**: 2bfac71efe3c5cd83ea1fa3b48b69fa805c4919f
**Branch**: improve-meta
**Repository**: accelerator

## Research Question

Many of the skills follow the pattern of using markdown files as input and/or
output, stored in the meta/ directory. However, not all of them do and some
context is lost. In particular, review-plan and review-pr throw away all of the
review results from each of the lenses they use, as well as the overall review
summary. How can we improve meta management across all skills, including
preserving review artifacts?

## Summary

The plugin has a clear philosophical commitment to filesystem-as-shared-memory
(stated in README lines 17-29), but the implementation is inconsistent. Three
skills (`research-codebase`, `create-plan`, `describe-pr`) follow the pattern
well — they write structured, reusable artifacts to predictable paths in
`meta/`. Two skills (`review-pr`, `review-plan`) explicitly discard their most
valuable outputs: the per-lens review results and the synthesised review
summary.
Three skills (`validate-plan`, `respond-to-pr`, `commit`) produce no meta
artifacts at all. This creates gaps in the development lifecycle's audit trail
and prevents downstream skills from leveraging upstream outputs.

Crucially, these skills are designed for **collaborative, cross-team use**.
Any team member should be able to pick up where another left off — reviewing a
plan someone else created, responding to PR feedback on a colleague's review,
or validating a plan they didn't write. The `meta/` directory is the mechanism
that makes this possible: it's checked into version control and shared across
the team. When a skill discards its output to the conversation instead of
writing to `meta/`, that output is locked to a single person's session and
invisible to the rest of the team.

## Detailed Findings

### Current State: How Each Skill Uses meta/

#### Skills That Write Persistent Artifacts

| Skill               | Writes To                                 | Format                      | Reusable By                                      |
|---------------------|-------------------------------------------|-----------------------------|--------------------------------------------------|
| `research-codebase` | `meta/research/YYYY-MM-DD-description.md` | Markdown + YAML frontmatter | `create-plan`, `documents-locator`               |
| `create-plan`       | `meta/plans/YYYY-MM-DD-description.md`    | Markdown + YAML frontmatter | `implement-plan`, `review-plan`, `validate-plan` |
| `describe-pr`       | `meta/prs/{number}-description.md`        | Markdown                    | `gh pr edit` (posted to GitHub)                  |
| `create-adr`        | `meta/decisions/ADR-NNNN-title.md`        | Markdown + YAML frontmatter | `review-adr`, `extract-adrs`, `documents-locator` |

These four skills exemplify the intended pattern: structured output to a
predictable path, with clear consumers.

Note: The ADR skills (`create-adr`, `review-adr`, `extract-adrs`) were added
recently and follow the meta/ pattern well. `create-adr` writes formal ADRs to
`meta/decisions/`, `review-adr` reads and updates ADR status in-place (similar
to how `implement-plan` checks off items in a plan), and `extract-adrs` scans
existing meta documents (research, plans) to identify implicit decisions and
convert them into formal ADRs. These skills are a good example of the pattern
working as intended across the team — one person can create an ADR and another
can review it, with the `meta/decisions/` directory as the handoff point.

#### Skills That Write Only Ephemeral Artifacts

| Skill       | Writes To                      | Format                   | Problem                                                                                                             |
|-------------|--------------------------------|--------------------------|---------------------------------------------------------------------------------------------------------------------|
| `review-pr` | `meta/tmp/pr-review-{number}/` | Mixed (patch, txt, json) | Only raw PR data is saved; per-lens JSON results and the final review summary are discarded after posting to GitHub |

`review-pr` creates a temp directory with the diff, changed files, PR
description, commits, head SHA, repo info, and the final review payload JSON.
However:

- The **individual lens review results** (the JSON blocks returned by each
  reviewer agent) are consumed in-memory during aggregation (Step 4) and never
  written to disk.
- The **aggregated review summary** (the markdown composed in Step 4.8) is
  written only to the `review-payload.json` as the `body` field — a JSON string
  inside a JSON file, not a standalone readable document.
- The SKILL.md explicitly says (line 527): "Don't write review findings to a
  separate file — all output goes to the conversation and then to GitHub via
  the API"

#### Skills That Write No Artifacts

| Skill           | Current Output       | What's Lost                                                                    |
|-----------------|----------------------|--------------------------------------------------------------------------------|
| `review-plan`   | Conversation only    | Per-lens JSON results, aggregated review summary, verdict, recommended changes |
| `validate-plan` | Conversation only    | Validation report, pass/fail results, deviation analysis                       |
| `respond-to-pr` | GitHub comments only | Triage summary, per-item analysis, response decisions, progress tracking       |
| `commit`        | VCS commits only     | N/A (appropriate — commits are the artifact)                                   |

`review-plan` is the most significant gap. Its SKILL.md (line 457) says: "Don't
write review findings to a separate file — all output goes to the conversation
and then into plan edits." This means:

- If you review a plan, iterate on it, then come back in a new session, there's
  no record of what was reviewed or what changed.
- The review-then-edit cycle has no audit trail.
- Re-reviews can't compare against previous review results because they don't
  exist on disk.
- **A different team member cannot see the review**: if one person reviews a
  plan and another person later runs `/implement-plan`, the implementer has no
  visibility into the review findings, the verdict, or the recommended changes.
  They only see the plan as-edited, with no context about why it was changed or
  what tradeoffs were accepted.

#### Skills That Read from meta/

| Skill            | Reads From                                | Purpose                         |
|------------------|-------------------------------------------|---------------------------------|
| `create-plan`    | `meta/tickets/`, `meta/research/`         | Context gathering               |
| `implement-plan` | `meta/plans/`                             | Execution instructions          |
| `review-plan`    | `meta/plans/`                             | Review target                   |
| `describe-pr`    | `meta/templates/`, `meta/prs/`            | Template + existing description |
| `review-pr`      | `meta/tmp/pr-review-{number}/`            | Own ephemeral artifacts         |
| `review-adr`     | `meta/decisions/`                         | ADR review + status transitions |
| `extract-adrs`   | `meta/research/`, `meta/plans/`, `meta/decisions/` | Source documents + existing ADRs |

### Gap Analysis: What's Being Lost

#### 1. Review Lens Results (review-pr and review-plan)

Each reviewer agent returns a structured JSON block containing:

- `lens`: identifier
- `summary`: 2-3 sentence assessment
- `strengths`: positive observations
- `comments` / `findings`: detailed issues with severity, confidence, location

For a typical 7-lens review, this is 7 structured JSON documents — a rich
dataset that's consumed once and discarded. This data would be valuable for:

- **Historical analysis**: tracking which lenses consistently find issues
- **Review iteration**: comparing re-review results against originals
- **Downstream skills**: `respond-to-pr` could reference the original review
  findings
- **Audit trail**: understanding why specific review comments were made
- **Team handoff**: a teammate who didn't run the review can see the full
  analysis, understand the severity and confidence of each finding, and make
  informed decisions about what to address

#### 2. Aggregated Review Summaries

The synthesised review summary (Step 4.8 in review-pr, Step 4.7 in
review-plan) includes cross-cutting themes, tradeoff analysis, verdict, and
prioritised findings. For review-pr, this gets embedded in a JSON payload; for
review-plan, it exists only in the conversation.

#### 3. Validation Reports (validate-plan)

`validate-plan` generates a structured validation report (Step 3) with
implementation status, automated verification results, deviations, and
recommendations. This report exists only in conversation output.

#### 4. Respond-to-PR Triage and Decisions

`respond-to-pr` performs detailed triage (categorisation, prioritisation) and
records decisions (implement, push back, skip) for each feedback item. None of
this is persisted.

### The documents-locator Agent's Expectations

The `documents-locator` agent (which helps research-codebase find relevant
documents) already expects a richer meta/ structure than currently exists. It
enumerates these expected subdirectories:

- `meta/research/` — exists, populated
- `meta/plans/` — exists, populated
- `meta/prs/` — exists conceptually (created by describe-pr)
- `meta/decisions/` — now created by `create-adr`, read by `review-adr` and
  `extract-adrs`
- `meta/global/` — referenced but never created
- `meta/tickets/` — referenced by create-plan as input, but not created by
  any skill
- `meta/notes/` — referenced as example only

This suggests the original vision was broader than what's currently implemented.

## Recommendations

### Strategy: Every Skill Phase Gets a Persistent, Team-Visible Artifact

The core principle should be: **if a skill produces structured output that
would be valuable to a different team member or in a future session, it writes
to meta/**. The existing three-skill pattern (research, plan, PR description)
should be extended to all skills that produce meaningful artifacts.

This principle follows directly from how these skills are used in practice:
skills are designed for collaborative, cross-team workflows where different
people may run different phases. One person researches and plans; another
reviews the plan; a third implements it; a fourth reviews the PR. The `meta/`
directory — checked into version control and shared across the team — is the
only mechanism that enables this handoff. Any output that stays in a single
person's conversation is effectively lost to the team.

**The test for whether a skill should write to meta/**: Could a different team
member benefit from seeing this output? If yes, it belongs in `meta/`, not just
in the conversation.

### Recommendation 1: Add `meta/reviews/` for Review Artifacts

Create a new `meta/reviews/` directory to store review outputs from both
`review-plan` and `review-pr`.

**Directory structure:**

```
meta/reviews/
├── plans/
│   └── YYYY-MM-DD-description-review.md     # Plan review summary
└── prs/
    └── {number}-review.md                    # PR review summary
```

**Each review document should contain:**

```markdown
---
date: "ISO timestamp"
type: plan-review | pr-review
target: path/to/plan.md | PR #number
verdict: APPROVE | REVISE | COMMENT | REQUEST_CHANGES
lenses: [architecture, security, ...]
status: complete
---

# Review: [Target Name]

## Verdict: [VERDICT]

[Synthesised summary]

## Cross-Cutting Themes

...

## Tradeoff Analysis

...

## Findings

...

## Strengths

...

## Per-Lens Results

### Architecture

[Raw lens summary and findings]

### Security

[Raw lens summary and findings]

...
```

**Key design decisions:**

- The per-lens results are included in the review document itself, under a
  "Per-Lens Results" section, rather than as separate files. This keeps each
  review self-contained and avoids a proliferation of small files.
- The frontmatter includes the verdict and lenses used, making it
  machine-parseable for future analysis.
- The target field links to the plan path or PR number, connecting the review
  to what was reviewed.

**Changes required:**

- **review-pr SKILL.md**: Remove the "Don't write review findings to a separate
  file" guideline. After Step 4 (aggregation), write the review summary +
  per-lens results to `meta/reviews/prs/{number}-review.md`. Keep the
  `meta/tmp/` directory for ephemeral working data (diff, changed-files, etc.)
  but the review itself is now a persistent artifact.
- **review-plan SKILL.md**: Remove the equivalent guideline. After Step 4,
  write the review summary + per-lens results to
  `meta/reviews/plans/YYYY-MM-DD-description-review.md` (matching the plan's
  filename pattern).
- **README.md**: Add `reviews/` to the meta/ directory table.
- **documents-locator agent**: Add `meta/reviews/` to its enumerated
  directories.

### Recommendation 2: Add `meta/validations/` for Validation Reports

`validate-plan` already generates a structured validation report. Persist it.

**Path**: `meta/validations/YYYY-MM-DD-description-validation.md`

**Changes required:**

- **validate-plan SKILL.md**: After generating the validation report (Step 3),
  write it to `meta/validations/` with YAML frontmatter containing the plan
  path, pass/fail status, and date.
- **README.md**: Add `validations/` to the meta/ directory table.

### Recommendation 3: Let review-plan Reference Previous Reviews

When `review-plan` runs a re-review (Step 7), it currently compares "previous
findings against new findings" but only within the same conversation. With
reviews persisted to `meta/reviews/plans/`, re-reviews (and fresh reviews of
previously-reviewed plans) can:

1. Check for an existing review document for the same plan.
2. Load the previous review's findings.
3. Generate a delta-focused re-review that explicitly references the prior
   review.

**Changes required:**

- **review-plan SKILL.md**: In Step 1, after reading the plan, check
  `meta/reviews/plans/` for an existing review of the same plan. If found,
  load it and use it for comparison in the re-review flow.

### Recommendation 4: Let respond-to-pr Reference the Review

When `respond-to-pr` addresses review feedback, it would benefit from knowing
which items came from the structured `/review-pr` analysis vs. organic reviewer
comments. With `meta/reviews/prs/{number}-review.md` available:

1. `respond-to-pr` can cross-reference GitHub review comments against the
   structured review findings.
2. It can leverage the original analysis (severity, confidence, lens) to
   prioritise its triage.

**Changes required:**

- **respond-to-pr SKILL.md**: In Step 1, after fetching PR context, check
  `meta/reviews/prs/{number}-review.md`. If found, load it to inform triage
  priorities and provide richer context for each feedback item.

### Recommendation 5: Update the meta/ Directory Table in README

The README's meta/ table (line 73-79) should be updated to reflect the full
set of directories:

| Directory      | Purpose                                      | Written by                          |
|----------------|----------------------------------------------|-------------------------------------|
| `research/`    | Research findings with YAML frontmatter      | `research-codebase`                 |
| `plans/`       | Implementation plans with phased changes     | `create-plan`                       |
| `decisions/`   | Architecture decision records (ADRs)         | `create-adr`, `review-adr`          |
| `reviews/`     | Review summaries and per-lens results        | `review-pr`, `review-plan`          |
| `validations/` | Plan validation reports                      | `validate-plan`                     |
| `prs/`         | PR descriptions                              | `describe-pr`                       |
| `templates/`   | Reusable templates (e.g., PR descriptions)   | manual                              |
| `tickets/`     | Input tickets/specs (user-provided)          | manual                              |
| `tmp/`         | Ephemeral working data (e.g., diff, patches) | `review-pr`                         |

### Recommendation 6: Consistent Frontmatter Across All Artifacts

All persistent meta/ artifacts should use YAML frontmatter for
machine-parseability. Currently:

- `research-codebase` uses detailed frontmatter (date, researcher, git_commit,
  branch, repository, topic, tags, status)
- `create-plan` uses no frontmatter (just markdown)
- `describe-pr` uses no frontmatter

**Proposed minimum frontmatter for all artifacts:**

```yaml
---
date: "ISO timestamp"
type: research | plan | review | validation | pr-description
skill: skill-name-that-created-this
status: complete | draft | superseded
---
```

This is a lower-priority improvement but would make `documents-locator` and
future tooling more effective.

### Recommendation 7: Clarify tmp/ vs Persistent Artifacts

The current `meta/tmp/` usage by `review-pr` mixes ephemeral working data
(diff patches, changed-files lists) with what could be persistent artifacts
(the review itself). With reviews moving to `meta/reviews/`, `meta/tmp/` can
be purely ephemeral:

- **Keep in tmp/**: `diff.patch`, `changed-files.txt`, `pr-description.md`,
  `commits.txt`, `head-sha.txt`, `repo-info.txt`, `review-payload.json`
- **Move to reviews/**: The synthesised review summary and per-lens results

This aligns with `meta/tmp/` being gitignored (as it already is in
`.gitignore` line 8).

### Skills Not Requiring Changes

- **implement-plan**: Reads from `meta/plans/` and modifies the plan file's
  checkboxes in-place. This is appropriate — the plan file IS the artifact.
- **commit**: Produces VCS commits, which are their own persistent artifact.
  No meta/ output needed.
- **respond-to-pr**: Could optionally write a response log to meta/, but the
  GitHub thread history already serves as the persistent record. The main
  benefit is cross-referencing with reviews (Recommendation 4), not creating
  a new artifact type.

## Architecture Insights

The plugin's architecture follows a "filesystem as message bus" pattern where
`meta/` serves the role that a database or message queue would in a
traditional system. The current gaps are analogous to having some microservices
that publish events and others that fire-and-forget.

The key architectural principle from the README is: "No skill assumes access to
another skill's conversation history." This means ANY valuable output that
might be needed across sessions or by other skills MUST be written to `meta/`.
The review skills violate this principle by keeping their most valuable output
(the structured review) only in conversation.

This principle is strengthened by the collaborative, cross-team nature of the
workflow. The `meta/` directory is checked into version control, making it the
shared workspace for the entire team. Consider the following scenarios:

- **Alice researches, Bob plans, Carol implements**: This works today because
  research and plans are written to `meta/`. Each person picks up the previous
  person's output from disk.
- **Alice reviews Bob's plan, Bob iterates**: This partially works — Alice's
  review findings exist only in her conversation. Bob sees the plan edits but
  not the rationale behind them, the accepted tradeoffs, or the findings that
  were intentionally left unaddressed.
- **Alice reviews a PR, Bob responds to the review**: Bob sees the GitHub
  comments but not the structured analysis that produced them — the severity
  ratings, confidence levels, cross-cutting themes, and tradeoff analysis are
  all lost.
- **Alice validates a plan implementation, Bob picks up the remaining work**:
  Bob has no visibility into what was validated, what passed, what deviated, or
  what still needs attention.

The recommended changes close these gaps by ensuring every phase of the
lifecycle produces a team-visible artifact. The changes are conservative — they
extend the existing pattern rather than introducing new mechanisms. The
`meta/reviews/` directory mirrors the existing `meta/research/` and
`meta/plans/` patterns.

## Priority Order for Implementation

1. **High**: Add `meta/reviews/` and update `review-pr` + `review-plan` to
   write review artifacts (Recommendations 1, 5, 7) — this addresses the
   primary pain point
2. **Medium**: Add `meta/validations/` and update `validate-plan`
   (Recommendation 2) — completes the lifecycle audit trail
3. **Medium**: Enable cross-referencing between reviews and downstream skills
   (Recommendations 3, 4) — leverages the new artifacts
4. **Low**: Standardise frontmatter across all artifacts (Recommendation 6) —
   nice-to-have for consistency and tooling

## Code References

- `skills/github/review-pr/SKILL.md:527` — "Don't write review findings to a
  separate file"
- `skills/planning/review-plan/SKILL.md:457` — "Don't write review findings to a
  separate file"
- `skills/github/review-pr/SKILL.md:71-91` — tmp directory creation and artifact
  writes
- `skills/github/review-pr/SKILL.md:280-375` — Step 4 aggregation (where results
  are consumed and discarded)
- `skills/planning/review-plan/SKILL.md:229-335` — Step 4 aggregation
- `skills/review/output-formats/pr-review-output-format/SKILL.md` — JSON schema
  for PR review agent output
- `skills/review/output-formats/plan-review-output-format/SKILL.md` — JSON
  schema for plan review agent output
- `agents/reviewer.md` — Generic reviewer agent definition
- `agents/documents-locator.md` — References expected meta/ subdirectories
- `README.md:66-88` — meta/ directory documentation
- `.gitignore:8` — meta/tmp/ is gitignored

## Open Questions

1. **Review versioning**: When a plan is reviewed multiple times, should each
   review be a separate file (e.g., `-review-1.md`, `-review-2.md`) or should
   re-reviews append to / update the original review document? Separate files
   preserve history but create clutter; updating in-place is cleaner but loses
   the audit trail.

2. **Per-lens results granularity**: Should the per-lens JSON results be stored
   verbatim (the full JSON block) or converted to readable markdown? JSON
   preserves machine-parseability; markdown is more readable. The recommendation
   above suggests markdown within the review document, but storing the raw JSON
   alongside could enable future tooling.

3. **Lifecycle of tmp/ artifacts**: Should `review-pr` clean up `meta/tmp/`
   automatically at session end (as currently documented in guideline 7), or
   should cleanup be manual? With the review itself persisted elsewhere, the
   tmp/ data is less critical to preserve.
