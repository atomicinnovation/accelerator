---
date: 2026-02-22T15:04:46+00:00
researcher: Toby Clemson
git_commit: N/A
branch: N/A
repository: ~/.claude (Claude Code configuration)
topic: "Modifying review-pr command to suggest inline PR comments instead of monolithic review"
tags: [research, pr-review, inline-comments, github-api, agents, commands]
status: complete
last_updated: 2026-02-22
last_updated_by: Toby Clemson
last_updated_note: "Incorporated formatting patterns from temp-review-gh.md reference command"
---

# Research: Modifying review-pr to Suggest Inline PR Comments

**Date**: 2026-02-22T15:04:46+00:00
**Researcher**: Toby Clemson
**Repository**: ~/.claude (Claude Code configuration)

## Research Question

The `review-pr` command currently presents a compiled review in the
conversation and offers to paste it as a single monolithic comment on the PR.
How should the command and its agents be changed so that it instead suggests
specific inline comments on specific lines in the diff, similar to how a human
reviewer would leave comments?

## Summary

This is achievable but requires changes at three levels:

1. **Agent output format**: Each PR review agent must change its output from
   prose-based findings to structured data that includes precise file paths,
   line numbers, and diff sides
2. **Command orchestration (review-pr.md)**: The synthesis step must change from
   compiling a narrative review to collecting, deduplicating, and presenting a
   set of proposed inline comments, then posting them via the GitHub API
3. **GitHub API integration**: The `gh pr review` command does not support
   inline comments — `gh api` must be used to call the Pull Request Reviews
   endpoint directly

## Detailed Findings

### Current Architecture

The review system has three layers:

**Command** (`commands/review-pr.md`):
- Fetches PR metadata and diff into a temp directory
- Selects which review lenses to run
- Spawns 1-6 specialist agents in parallel via the Task tool
- Waits for all agents, then synthesises findings into a narrative
- Presents the review and offers to post it as a single `gh pr comment`

**Agents** (6 specialist agents in `agents/pr-*.md`):
- `pr-architecture-reviewer`
- `pr-security-reviewer`
- `pr-code-quality-reviewer`
- `pr-test-coverage-reviewer`
- `pr-standards-reviewer`
- `pr-usability-reviewer`

Each agent:
- Reads diff.patch, changed-files.txt, pr-description.md, commits.txt
- Explores the codebase for context
- Returns findings in a structured markdown format with severity levels
  (Critical/Major/Minor/Suggestions)
- Each finding includes: title, confidence, location (`file:line`), issue
  description, impact, and suggestion

**Output**: Currently a compiled narrative review with sections for
cross-cutting themes, findings by severity, tradeoff analysis, strengths, and
recommended changes.

### GitHub API for Inline Review Comments

**Key constraint**: The `gh pr review` CLI command does NOT support inline
comments. Only `gh api` can create them.

**Endpoint**: `POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews`

**Payload structure**:
```json
{
  "commit_id": "<HEAD SHA of PR>",
  "body": "Overall review summary.",
  "event": "COMMENT",
  "comments": [
    {
      "path": "src/utils.ts",
      "line": 42,
      "side": "RIGHT",
      "body": "Comment text for this line."
    },
    {
      "path": "src/auth.ts",
      "start_line": 60,
      "start_side": "RIGHT",
      "line": 68,
      "side": "RIGHT",
      "body": "Multi-line comment spanning lines 60-68."
    }
  ]
}
```

**Key parameters**:
- `line`: The line number in the file (not diff position). Must be within the
  diff.
- `side`: `"RIGHT"` for new/added lines (most common), `"LEFT"` for deleted
  lines
- `start_line` + `start_side`: For multi-line comments spanning a range
- `event`: `"COMMENT"`, `"APPROVE"`, or `"REQUEST_CHANGES"`
- `commit_id`: HEAD SHA of the PR branch

**Critical constraint**: The `line` number must be a line that actually appears
in the diff (added, removed, or context line). Lines outside any hunk will
cause a 422 validation error.

**Practical submission pattern**:
```bash
COMMIT_SHA=$(gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha')

jq -n \
  --arg sha "$COMMIT_SHA" \
  --argjson comments "$COMMENTS_JSON" \
  '{
    commit_id: $sha,
    body: "Automated review summary.",
    event: "COMMENT",
    comments: $comments
  }' | gh api repos/{owner}/{repo}/pulls/{number}/reviews \
    --method POST --input -
```

### Reference: temp-review-gh.md Patterns

An existing reference command (`temp-review-gh.md`) provides several useful
patterns that should be adopted:

**Severity emoji system**:
- 🔴 Critical
- 🟡 Warning
- 🔵 Suggestion
- ✅ Good (strengths — summary only, not inline)

**Two-part posting pattern**: Post a summary comment first (via `gh pr comment`)
containing the verdict, overview, strengths, and a priority issues index, then
post inline comments for specific findings. This gives the PR a navigable
overview alongside the detailed inline comments.

**Inline comment format template**:
```
[severity_emoji] **[Category]**

[Message]

[Details if short enough]

**References:**
- [reference links]
```

**Comment volume limit**: Cap inline comments at ~10 most important to avoid
spamming the PR. Prioritise critical and warning severity over suggestions.

**Positive feedback routing**: Strengths and positive observations go in the
summary comment only — never as inline comments. This keeps inline comments
focused on actionable items.

**Verdict mapping**: The summary should include a clear verdict
(APPROVE / REQUEST_CHANGES / COMMENT) that maps to the `event` field in the
GitHub API.

**Note on `position` vs `line`**: The reference command uses the legacy
`position` parameter (diff-relative offset). The modern API uses `line` (file
line number) + `side` instead. The `position` parameter is being deprecated by
GitHub — the implementation should use `line`/`side`.

### Required Changes

#### 1. Agent Output Format Changes

All six PR review agents need their output format modified. Currently they
produce findings like:

```markdown
#### Critical
- **Missing auth check** (confidence: high)
  **Location**: `src/auth.ts:42` (changed in PR)
  **Issue**: No authorization check before data access
  **Impact**: Horizontal privilege escalation
  **Suggestion**: Add role-based access check before the query
```

They need to produce structured data that maps directly to inline comments.
The recommended approach is to have agents output a JSON block (or structured
markdown that can be parsed) containing:

```json
{
  "summary": "2-3 sentence assessment of this lens",
  "strengths": [
    "Good separation of concerns in the auth module",
    "Comprehensive error types with clear categorisation"
  ],
  "comments": [
    {
      "path": "src/auth.ts",
      "line": 42,
      "end_line": null,
      "side": "RIGHT",
      "severity": "critical",
      "confidence": "high",
      "lens": "security",
      "title": "Missing auth check",
      "body": "🔴 **Security**\n\nNo authorization check before data access. Any authenticated user can access any other user's data (horizontal privilege escalation).\n\n**Suggestion**: Add role-based access check before the query:\n```ts\nawait requirePermission(user, 'read', resourceId);\n```"
    }
  ],
  "general_findings": [
    {
      "severity": "minor",
      "lens": "security",
      "title": "No rate limiting on auth endpoints",
      "body": "The new auth endpoints don't have rate limiting configured. Consider adding rate limiting to prevent brute-force attacks."
    }
  ]
}
```

The `strengths` array feeds into the summary comment (positive feedback stays
out of inline comments). The `general_findings` array captures cross-cutting
observations that can't be anchored to specific lines — these also go in the
summary comment body.

**Key considerations for agent changes**:
- Agents must identify **exact line numbers** in the new file version, not
  just approximate `file:line` references
- Agents must ensure referenced lines are within the diff hunks (context,
  added, or removed lines)
- For findings about deleted code, `side` should be `"LEFT"`
- For findings spanning multiple lines, both `line` and `end_line` should be
  provided
- The `body` field should be self-contained — it will appear as a standalone
  comment on the PR, so it needs enough context to be understood without the
  surrounding narrative

**Specific changes per agent file** (all follow the same pattern):

Each of the 6 agent files in `agents/pr-*.md` needs:

a. **Updated instructions** explaining that findings must map to specific diff
   lines rather than general observations

b. **New output format section** replacing the current markdown template with
   the structured JSON format above

c. **Guidance on line number precision**: Instruct agents to read the diff
   carefully and identify exact line numbers, verify they fall within diff
   hunks, and use the new file's line numbers (RIGHT side) for most comments

d. **Self-contained comment body guidance**: Each comment body must stand alone
   as a PR inline comment — include the lens name, severity, the issue, its
   impact, and the suggestion all in one body

e. **Handling findings that don't map to specific lines**: Some findings are
   architectural or cross-cutting and don't map to a single line. Agents
   should still try to anchor these to the most relevant line in the diff, or
   flag them as "general" findings that belong in the review summary body
   rather than as inline comments

#### 2. Command Orchestration Changes (review-pr.md)

**Step 1 (Fetch PR)**: Add fetching the HEAD commit SHA:
```bash
gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha' > "$REVIEW_DIR/head-sha.txt"
```

Also fetch owner/repo info for the API call:
```bash
gh repo view --json owner,name --jq '.owner.login + "/" + .name' > "$REVIEW_DIR/repo-info.txt"
```

**Step 3 (Spawn agents)**: Agent prompts need to instruct agents to return the
new structured JSON format. Each agent prompt should include:
- A reminder that findings must include exact line numbers from the diff
- The expected JSON output schema
- Instruction to anchor findings to specific diff lines where possible, and
  flag general findings separately

**Step 4 (Compile and Synthesise)**: This is the biggest change. Instead of
compiling a narrative, the command must:

a. Parse the structured output from each agent (extracting the JSON blocks)

b. Collect all inline comment proposals across all agents

c. Deduplicate: Where multiple agents flag the same line/file, merge their
   comments into a single inline comment that references all relevant lenses

d. Separate comments into:
   - **Inline comments**: Have a specific file:line and can be posted as review
     comments
   - **General findings**: Cross-cutting or architectural observations that
     belong in the review summary body

e. Compose the review summary body from:
   - Overall assessment across all lenses
   - General findings that couldn't be anchored to specific lines
   - Cross-cutting themes

f. Compose each inline comment body to be self-contained and useful

**Step 5 (Present the Review)**: Change from showing a narrative to showing a
two-part preview — the summary comment and the proposed inline comments:

a. Preview of the summary comment that will be posted via `gh pr comment`:
```markdown
## Code Review

**Verdict:** COMMENT | REQUEST_CHANGES | APPROVE

[Overall assessment across lenses — 2-3 sentences]

### Strengths
- ✅ [Positive observations aggregated from all agents]
- ✅ [Only in summary, never as inline comments]

### General Findings
- [Cross-cutting findings that couldn't be anchored to specific lines]

### Priority Issues
1. 🔴 [Critical] - `file:line` — brief description
2. 🟡 [Warning] - `file:line` — brief description
3. 🔵 [Suggestion] - `file:line` — brief description

---
*Review generated by /review-pr*
```

b. Preview of proposed inline comments grouped by file (capped at ~10 most
   important, prioritising 🔴 critical and 🟡 warning over 🔵 suggestions):
```
## Proposed Inline Comments (8 of 12 findings)

### src/auth.ts
- Line 42: 🔴 **Security** — Missing auth check (also flagged by: architecture)
- Lines 60-68: 🟡 **Code Quality** — Silent exception swallowing

### src/config.ts
- Line 12: 🔵 **Standards** — Hardcoded value should use env variable

### Deferred to summary (4 findings)
- 🟡 Architecture: No rate limiting on auth endpoints
- 🔵 Test Coverage: Missing integration tests for auth flow
```

c. The user can review, adjust, or remove individual comments before posting

**Step 6 (Offer Follow-Up Options)**: Replace the current options with:

```
The review is ready. Would you like to:
1. Post the review? (summary comment + inline comments)
2. Change the verdict? (currently: COMMENT)
3. Edit or remove specific comments before posting?
4. Discuss any findings in more detail?
```

When the user chooses to post, execute in two steps:

a. Post the summary comment: `gh pr comment {number} --body "..."`
b. Post the inline comments as a review via `gh api`:
   ```bash
   jq -n \
     --arg sha "$COMMIT_SHA" \
     --argjson comments "$COMMENTS_JSON" \
     '{
       commit_id: $sha,
       body: "",
       event: $event,
       comments: $comments
     }' | gh api repos/{owner}/{repo}/pulls/{number}/reviews \
       --method POST --input -
   ```

**Comment volume guideline**: Limit inline comments to ~10 most important.
If agents produce more findings, include only critical and major findings as
inline comments, and roll minor/suggestion findings into the summary or drop
them. This prevents PR comment spam.

#### 3. Diff Line Number Validation

A critical challenge: agents must reference line numbers that actually appear
in the diff. If an agent references a line outside any diff hunk, the GitHub
API will reject it with a 422 error.

**Approach options**:

a. **Agent-side validation**: Include the diff in agent context and instruct
   agents to only reference lines visible in the diff. This is the current
   approach (agents already read the diff) but may not be 100% reliable.

b. **Command-side validation**: After collecting agent outputs, the command
   parses the diff to extract valid line ranges per file, then validates each
   proposed comment's line number against those ranges. Invalid comments get
   moved to the general review body. This is the more robust approach.

c. **Hybrid**: Agents try their best, and the command validates and adjusts.
   This is recommended.

**Diff parsing for validation**: The unified diff format contains hunks like:
```
@@ -10,6 +15,8 @@ function example() {
```
Where `+15,8` means "starting at line 15 in the new file, spanning 8 lines".
The command can parse these hunk headers to build a set of valid line ranges
per file, then check each proposed comment against them.

### Architecture Insights

**The agents are the key change point**. The current agent output format
(structured markdown with `file:line` locations) is already close to what's
needed — the main gap is:
1. Precision: agents sometimes give approximate line references
2. Structure: output needs to be machine-parseable (JSON), not just
   human-readable
3. Self-containment: each finding's body must work as a standalone inline
   comment

**The command's synthesis role changes fundamentally**. Currently it's a
narrative compiler. It needs to become a comment curator — deduplicating,
validating line numbers, merging cross-lens findings, and presenting a list of
discrete comments for user approval before posting.

**The presentation UX changes significantly**. Instead of reading a long
review document, the user reviews a list of proposed inline comments grouped by
file. This is more actionable but potentially less cohesive — the review
summary body should compensate by capturing cross-cutting themes and the
overall narrative.

## Considerations and Trade-offs

### JSON vs Structured Markdown for Agent Output

**JSON approach** (recommended):
- Easier to parse programmatically in the command
- Explicit field names prevent ambiguity
- Agents (LLMs) are reliable at generating JSON
- Can include metadata (severity, confidence, lens) as structured fields

**Structured markdown approach**:
- More natural for LLM output
- Requires regex/heuristic parsing in the command
- More error-prone to extract fields

### Comment Granularity

**One comment per finding** (recommended):
- Matches human reviewer behavior
- Each comment is self-contained and actionable
- Easy to resolve individually on GitHub

**Merged comments per file location**:
- If multiple lenses flag the same line, combine into one comment
- Reduces noise but may create very long comments
- Harder to resolve partially

**Recommendation**: Merge only when multiple lenses flag the exact same line,
otherwise keep comments separate. Include lens attribution in each comment.

### Findings That Don't Map to Lines

Some findings are inherently cross-cutting (e.g., "the PR lacks integration
tests" or "the overall coupling pattern is concerning"). These should go in the
review summary body, not as inline comments. Agents should be instructed to
separate line-specific findings from general observations.

### User Review Before Posting

The current command shows the review and offers to post it. The new version
should similarly show proposed comments and let the user curate them before
posting. This is important because:
- LLM-generated line numbers may be imprecise
- Users may want to adjust tone or remove noise
- Some findings may not be worth commenting on

## Implementation Roadmap

1. **Modify agent output format** (all 6 agents): Add JSON output block with
   structured comment proposals alongside the existing narrative
2. **Update command Step 1**: Fetch HEAD SHA and repo info
3. **Update command Step 3**: Add structured output instructions to agent
   prompts
4. **Rewrite command Step 4**: Parse agent JSON, deduplicate, validate line
   numbers against diff, separate inline vs general findings
5. **Rewrite command Step 5**: Present proposed comments grouped by file
6. **Rewrite command Step 6**: Add posting via `gh api` with review event
   selection
7. **Test with a real PR**: Validate line numbers work, comments render well,
   and the overall UX is good

## Resolved Questions

1. **Maximum number of inline comments?** Yes — cap at ~10 most important,
   prioritising critical and warning. Overflow goes to the summary comment or
   gets dropped. (Adopted from temp-review-gh.md.)
2. **Where do positive observations go?** Summary comment only, never as
   inline comments. Inline comments are exclusively for actionable findings.
   (Adopted from temp-review-gh.md.)
3. **How should severity map to review event?** The verdict (APPROVE /
   REQUEST_CHANGES / COMMENT) should be suggested based on whether any critical
   findings exist, but the user can override before posting.

## Open Questions

1. **Should the command support a "dry run" mode** where it shows proposed
   comments without offering to post? (Useful for reviewing the review itself)
2. **Should the command support replying to existing review threads?** For
   re-reviews, it might be useful to reply to existing comment threads rather
   than creating new ones.
3. **How to handle agent output parsing failures?** If an agent returns
   malformed JSON, should the command fall back to the narrative format or ask
   the user to re-run that lens?
4. **Summary comment vs review body**: Should the overview be posted as a
   separate `gh pr comment` (always visible in the timeline) or as the `body`
   of the review itself (collapsed under the review)? The temp-review-gh.md
   reference uses a separate PR comment, which is more visible. The batched
   review API also supports a `body` field. Potentially both could be used —
   the review body for a brief note, and a separate comment for the full
   summary.
