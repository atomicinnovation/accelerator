# PR Review Inline Comments Implementation Plan

## Overview

Update the 6 PR review agents and the `review-pr` command so that reviews
produce inline diff comments posted via the GitHub Reviews API, rather than a
monolithic narrative pasted as a single PR comment. The agents will output
structured JSON with precise line references, and the command will curate,
deduplicate, preview, and post these as a proper GitHub code review.

## Current State Analysis

**Command** (`commands/review-pr.md`):
- Fetches PR metadata and diff into a temp directory
- Selects which review lenses to run
- Spawns 1-6 specialist agents in parallel via the Task tool
- Synthesises agent output into a narrative review document
- Presents the review in-conversation
- Offers to post the entire review as a single `gh pr comment`

**Agents** (6 files in `agents/pr-*.md`):
- `pr-architecture-reviewer`, `pr-security-reviewer`,
  `pr-code-quality-reviewer`, `pr-test-coverage-reviewer`,
  `pr-standards-reviewer`, `pr-usability-reviewer`
- Each returns structured markdown with findings at severity levels
  (Critical/Major/Minor/Suggestions), each including a `file:line` location
- Locations are approximate — sufficient for human reading but not precise
  enough for the GitHub API

**Gap**: No mechanism to post inline comments on specific diff lines. The
`gh pr review` CLI does not support inline comments. The GitHub REST API
endpoint `POST /repos/{owner}/{repo}/pulls/{number}/reviews` is required,
using `line`/`side` parameters (not the deprecated `position`).

### Key Discoveries:

- All 6 agents share the same structural pattern: frontmatter, intro, Core
  Responsibilities, Analysis Strategy (3-5 steps), Output Format, Important
  Guidelines, What NOT to Do, closing Remember line
- Agent findings already include `file:line` references but lack the precision
  and structure needed for the API (exact line within diff hunks, LEFT/RIGHT
  side, self-contained body text)
- The `line` parameter in the API must reference a line visible in the diff
  (added, removed, or context line). Lines outside diff hunks cause 422 errors.
- The review API's `body` field is the natural place for the summary — it
  appears at the top of the review in GitHub's UI, keeping summary + inline
  comments as a single cohesive review

## Desired End State

After this plan is complete:

1. Each PR review agent outputs a JSON block containing:
   - `summary`: lens-specific assessment text
   - `strengths`: positive observations (for the summary, never inline)
   - `comments`: line-anchored findings with exact path, line, side, severity,
     and self-contained body text formatted for GitHub inline comments
   - `general_findings`: cross-cutting observations that cannot be anchored to
     specific lines

2. The `review-pr` command:
   - Fetches additional metadata needed for the API (HEAD SHA, owner/repo)
   - Instructs agents to return the JSON format
   - Validates agent line numbers against diff hunks, moving invalid references
     to general findings automatically
   - Aggregates, deduplicates, and prioritises findings across all agents
   - Presents a two-part preview: summary + inline comments grouped by file
   - Posts a single GitHub review via `gh api` containing both the summary body
     and up to 10 inline comments (all critical findings always included)
   - Offers the user control over verdict and comment selection before posting

**Verification**: Run `/review-pr` against a real PR and confirm:
- Agents return valid JSON with line numbers within diff hunks
- The command successfully posts a review visible in GitHub's "Files changed"
  tab with inline comments on the correct lines
- The review summary appears at the top of the review
- No 422 errors from invalid line references

## What We're NOT Doing

- Replying to existing review comment threads (re-review support)
- A separate "dry run" mode (the preview-before-posting flow serves this need)
- A separate `gh pr comment` for the summary — we post the summary as the
  review `body` field in a single API call. This is simpler and atomic (no
  orphaned summary comments if the review fails), though it means the summary
  is collapsed under the review in GitHub's UI rather than appearing as a
  standalone timeline comment. We accept this tradeoff for simplicity.
- Changes to the plan review agents (`plan-*-reviewer`) — only `pr-*-reviewer`
  agents are in scope
- Changes to the plugin-provided `pr-review-toolkit` review command

## Implementation Approach

Two phases, where Phase 2 depends on Phase 1:

1. **Phase 1**: Update all 6 agent output formats to produce structured JSON
   with precise line references
2. **Phase 2**: Rewrite the command's synthesis, presentation, and posting steps
   to consume the new agent output and post via the GitHub Reviews API

---

## Phase 1: Update Agent Output Format

### Overview

All 6 PR review agents get the same structural changes: a new analysis step
for anchoring findings to precise diff locations, a replaced Output Format
section producing JSON, and updated guidelines for self-contained comment
bodies and emoji formatting.

### Changes Required

The following changes apply identically to all 6 agent files:

- `agents/pr-architecture-reviewer.md`
- `agents/pr-security-reviewer.md`
- `agents/pr-code-quality-reviewer.md`
- `agents/pr-test-coverage-reviewer.md`
- `agents/pr-standards-reviewer.md`
- `agents/pr-usability-reviewer.md`

#### 1. Add New Analysis Step: "Anchor Findings to Diff Locations"

**Location**: Insert as the final step in the "Analysis Strategy" section,
after all existing evaluation steps and before the "Output Format" section.

This becomes the last numbered step (Step 5 or Step 6 depending on the agent).

**Content to add**:

```markdown
### Step N: Anchor Findings to Diff Locations

For each finding identified in the previous steps:

1. **Identify the exact line number** in the diff where the finding is most
   relevant:
   - For findings about added or modified code, use the line number in the new
     file version and set side to `"RIGHT"`
   - For findings about deleted code, use the line number in the old file
     version and set side to `"LEFT"`
   - The line MUST appear within a diff hunk (an added, removed, or context
     line) — lines outside diff hunks will cause API errors

2. **For findings spanning multiple lines**, identify both the start line
   (`line`) and end line (`end_line`) within the same diff hunk

3. **For findings that cannot be anchored** to a specific diff line
   (cross-cutting concerns, missing functionality, architectural observations),
   classify them as general findings — these will go in the review summary
   rather than as inline comments

4. **Compose self-contained comment bodies** for each line-anchored finding:
   - Start with the severity emoji and lens category
   - Include enough context for the comment to be understood on its own,
     without surrounding narrative
   - Include the impact and a concrete suggestion
   - Keep to 2-5 sentences plus an optional short code snippet
```

The step number depends on the agent:
- `pr-architecture-reviewer`: Insert as **Step 5** (after Step 4: Identify
  Beyond-the-Diff Impact)
- `pr-security-reviewer`: Insert as **Step 6** (after Step 5: Check for
  Secrets and Information Disclosure)
- `pr-code-quality-reviewer`: Insert as **Step 5** (after Step 4: Assess
  Maintainability)
- `pr-test-coverage-reviewer`: Insert as **Step 6** (after Step 5: Review
  Test Architecture and Reliability)
- `pr-standards-reviewer`: Insert as **Step 5** (after Step 4: Distinguish
  Convention from Preference)
- `pr-usability-reviewer`: Insert as **Step 6** (after Step 5: Evaluate
  Migration and Compatibility)

#### 2. Replace Output Format Section

**Location**: Replace the entire "## Output Format" section (from the heading
through the closing ``` of the markdown code block).

**Replace with** (substituting `[Lens Name]` with the agent's lens — e.g.,
"Architecture", "Security", "Code Quality", "Test Coverage", "Standards",
"Usability"):

````markdown
## Output Format

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "[lens-name]",
  "summary": "2-3 sentence [Lens Name] assessment of the PR.",
  "strengths": [
    "Positive observation about what the PR gets right from a [lens] perspective"
  ],
  "comments": [
    {
      "path": "src/example.ts",
      "line": 42,
      "end_line": null,
      "side": "RIGHT",
      "severity": "critical",
      "confidence": "high",
      "lens": "[lens-name]",
      "title": "Brief finding title",
      "body": "🔴 **[Lens Name]**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences, optionally with a code snippet]."
    }
  ],
  "general_findings": [
    {
      "severity": "minor",
      "lens": "[lens-name]",
      "title": "Cross-cutting finding title",
      "body": "Description of the finding that cannot be anchored to a specific diff line."
    }
  ]
}
```

### Field Reference

- **lens**: Agent lens identifier — one of `"architecture"`, `"security"`,
  `"code-quality"`, `"test-coverage"`, `"standards"`, `"usability"`
- **summary**: 2-3 sentence assessment from this lens's perspective
- **strengths**: Positive observations (fed into the review summary — never
  posted as inline comments)
- **comments**: Line-anchored findings for inline PR comments
  - **path**: File path relative to repository root (as shown in the diff
    header, e.g., `src/auth/handler.ts`)
  - **line**: Line number in the file where the comment should appear. For
    `"RIGHT"` side, this is the line number in the new version of the file. For
    `"LEFT"` side, this is the line number in the old version. The line MUST be
    visible in the diff (within a hunk).
  - **end_line**: Last line number for multi-line comments, or `null` for
    single-line. Must be in the same diff hunk as `line`.
  - **side**: `"RIGHT"` for commenting on added, modified, or context lines in
    the new file (the vast majority of comments). `"LEFT"` only for commenting
    on deleted lines.
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each comment so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained comment body formatted for a GitHub PR inline
    comment. See "Comment Body Format" below.
- **general_findings**: Findings that cannot be anchored to specific diff lines
  (cross-cutting concerns, missing functionality, architectural observations)
  - **severity**, **lens**, **title**, **body**: Same semantics as in `comments`

### Multi-Line Comment API Mapping

The agent schema uses `line` (start of range) and `end_line` (end of range).
The GitHub API inverts this: `start_line` is the beginning and `line` is the
end. The orchestrator handles this mapping when constructing the API payload.

Example — agent output:
```json
{ "line": 10, "end_line": 15, "side": "RIGHT" }
```
Becomes API payload:
```json
{ "start_line": 10, "start_side": "RIGHT", "line": 15, "side": "RIGHT" }
```

For single-line comments (`end_line` is `null`), the API payload uses only
`line` and `side` — `start_line` and `start_side` are omitted entirely (not
set to `null`).

### Severity Emoji Prefixes

Use these at the start of each comment `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

### Comment Body Format

Each comment `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix, optionally with a short code snippet].
```

Example:

```
🔴 **Security**

No authorization check before data access at this endpoint. Any authenticated
user can read any other user's records.

**Impact**: Horizontal privilege escalation — user data is exposed.

**Suggestion**: Add a permission check before the query:
\`\`\`ts
await requirePermission(user, 'read', resource.ownerId);
\`\`\`
```
````

Each agent's `[Lens Name]` and `[lens-name]` values:

| Agent File | `[Lens Name]` | `[lens-name]` |
|---|---|---|
| `pr-architecture-reviewer.md` | Architecture | architecture |
| `pr-security-reviewer.md` | Security | security |
| `pr-code-quality-reviewer.md` | Code Quality | code-quality |
| `pr-test-coverage-reviewer.md` | Test Coverage | test-coverage |
| `pr-standards-reviewer.md` | Standards | standards |
| `pr-usability-reviewer.md` | Usability | usability |

#### 3. Update Important Guidelines Section

**Location**: In the "## Important Guidelines" section, add these bullet points
after the existing guidelines:

```markdown
- **Anchor findings to precise diff lines** — every finding in `comments` must
  reference a line number that is visible in the diff (within a hunk). If you
  cannot identify a precise line, put the finding in `general_findings` instead.
  Prefer anchoring to a specific line wherever possible.
- **Make each comment body self-contained** — it will appear as a standalone
  inline comment on the PR, without surrounding narrative or other findings for
  context. Include the lens name, severity, issue, impact, and suggestion all
  in the one body.
- **Use severity emoji prefixes** — start each comment body with 🔴 (critical),
  🟡 (major), or 🔵 (minor/suggestion) followed by the lens name in bold
- **Output only the JSON block** — do not include additional prose, narrative
  analysis, or markdown outside the JSON code fence. The orchestrator parses
  your output as JSON.
```

#### 4. Update What NOT to Do Section

**Location**: In the "## What NOT to Do" section, add this bullet:

```markdown
- Don't include findings in `comments` that reference lines outside the diff —
  the GitHub API will reject them. When in doubt, use `general_findings`.
```

### Agent-Specific Adjustments

Beyond the shared template above, each agent needs its lens-specific values
substituted. No other agent-specific structural changes are needed — the Core
Responsibilities, Analysis Strategy steps (1 through N-1), What NOT to Do
section, and the closing "Remember:" paragraph all remain unchanged for each
agent.

### Success Criteria

#### Automated Verification:

- [x] All 6 agent files parse as valid markdown
- [x] Each agent file contains a JSON code block in its Output Format section
- [x] Each agent file contains the "Anchor Findings to Diff Locations" step

#### Manual Verification:

- [ ] Spot-check 2-3 agent files to confirm the lens name is correctly
  substituted in the Output Format section
- [ ] Confirm the new analysis step number is correct for each agent (matches
  the number of existing steps + 1)
- [ ] Review the Output Format section for clarity — could an LLM follow these
  instructions to produce valid JSON with correct line references?

---

## Phase 2: Update Command Orchestration

### Overview

Rewrite the `review-pr` command (`commands/review-pr.md`) to consume the new
agent JSON output, aggregate and deduplicate findings, present a structured
preview, and post via the GitHub Reviews API.

### Changes Required

**File**: `commands/review-pr.md`

#### 1. Step 1: Add HEAD SHA and Repo Info Fetching

**Location**: In "### Step 1: Identify and Fetch the PR", add to the fetch
commands in item 3 (after the existing `gh pr diff` and `gh pr view` commands):

```markdown
5. **Fetch additional metadata for the Reviews API**:
   ```bash
   gh api repos/$(gh repo view --json owner --jq '.owner.login')/$(gh repo view --json name --jq '.name')/pulls/{number} --jq '.head.sha' > "$REVIEW_DIR/head-sha.txt"
   gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"' > "$REVIEW_DIR/repo-info.txt"
   ```
```

Add to the error handling section:

```markdown
- **Cannot determine repo owner/name**: If `gh repo view` fails, instruct the
  user to run `gh repo set-default` and select the appropriate repository.
```

#### 2. Step 3: Update Agent Spawn Prompts

**Location**: Replace the entire "### Step 3: Spawn Review Agents" section.

**Replace with**:

````markdown
### Step 3: Spawn Review Agents

Spawn all selected review agents **in parallel** using the Task tool. Each
agent receives:

- The temp directory path containing:
  - `diff.patch` — full PR diff
  - `changed-files.txt` — list of changed file paths
  - `pr-description.md` — the PR description/body
  - `commits.txt` — commit message headlines
- The PR number and metadata for context
- Instructions to return structured JSON output (not prose)

Example spawn pattern:

```
Task 1 (pr-architecture-reviewer):
  "Review PR #{number} for architectural concerns. The PR context is at
  {temp_dir}/ — read diff.patch, changed-files.txt, pr-description.md, and
  commits.txt. Explore the codebase for context.

  IMPORTANT: Return your analysis as a single JSON code block following your
  Output Format specification. Ensure all comment line numbers reference lines
  visible in the diff. Do not include prose outside the JSON block."

Task 2 (pr-security-reviewer):
  "Review PR #{number} for security vulnerabilities. The PR context is at
  {temp_dir}/ — read diff.patch, changed-files.txt, pr-description.md, and
  commits.txt. Explore the codebase for context.

  IMPORTANT: Return your analysis as a single JSON code block following your
  Output Format specification. Ensure all comment line numbers reference lines
  visible in the diff. Do not include prose outside the JSON block."

[Same pattern for all selected agents]
```

**IMPORTANT**: Wait for ALL review agents to complete before proceeding.

**Handling malformed agent output**:

If an agent's response is not a clean JSON block, apply this extraction
strategy:

1. Look for a JSON code block fenced with triple backticks (optionally with
   a `json` language tag)
2. If found, extract and parse the content within the fences
3. If the extracted JSON is valid, use it normally
4. If no JSON code block is found, or the JSON within it is invalid, apply
   the fallback: treat the agent's entire output as a single general finding
   with the agent's lens name and `"major"` severity, and include it in the
   review summary body

When falling back, warn the user that the agent's output could not be parsed
and present the raw agent output in a collapsed form so the user can see what
the agent actually found.
````

#### 3. Step 4: Rewrite Compile and Synthesise

**Location**: Replace the entire "### Step 4: Compile and Synthesise Findings"
section.

**Replace with**:

````markdown
### Step 4: Aggregate and Curate Findings

Once all reviews are complete:

1. **Parse agent outputs**: Extract the JSON block from each agent's response
   (see the extraction strategy in Step 3). Collect the `summary`, `strengths`,
   `comments`, and `general_findings` arrays from each.

2. **Aggregate across agents**:
   - Combine all `comments` arrays into a single list
   - Combine all `general_findings` arrays into a single list
   - Combine all `strengths` arrays into a single list
   - Collect all `summary` strings

3. **Validate line numbers against the diff**: Parse the hunk headers in
   `diff.patch` to build valid line ranges per file. For each `@@` header:
   - Extract the new-file range from `@@ -a,b +c,d @@` — lines `c` through
     `c+d-1` are valid RIGHT-side lines
   - Extract the old-file range — lines `a` through `a+b-1` are valid
     LEFT-side lines
   - For each comment in the aggregated `comments` list, check that its
     `path`/`line`/`side` falls within a valid range for that file
   - Move any comments with out-of-range lines to `general_findings`
     automatically, preserving all their metadata (severity, lens, title, body)
   - If a comment was moved, note it in the preview so the user knows

4. **Deduplicate inline comments**: Where multiple agents flag the same file,
   same side, and overlapping or adjacent line range (same path, lines within
   3 of each other), consider merging — but only when the findings address the
   same underlying concern from different lens perspectives. Spatial proximity
   alone is not sufficient; the findings must be semantically related.

   When merging:
   - Combine the bodies, attributing each part to its lens
   - Use the highest severity among the merged findings
   - Use the highest confidence among the merged findings
   - Note all contributing lenses in the title

   When in doubt, keep comments separate — distinct inline comments are easier
   to resolve individually on GitHub than a merged comment covering multiple
   concerns.

5. **Prioritise and cap inline comments**:
   - Sort by severity: critical > major > minor > suggestion
   - Within the same severity, sort by confidence: high > medium > low
   - Always include all critical findings, even if that exceeds 10
   - Select up to 10 comments total for inline posting (more if all critical
     findings push beyond 10)
   - Move any remaining comments to the summary body as an "Additional
     Findings" list (title + file:line only)

6. **Determine suggested verdict**:
   - If any `"critical"` severity findings exist → suggest `REQUEST_CHANGES`
   - If only `"major"` or lower → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`

7. **Identify cross-cutting themes**: Look for findings that appear across
   multiple lenses — issues flagged by 2+ agents reinforce each other and
   should be highlighted in the summary. Also identify tradeoffs where
   different lenses conflict (e.g., security wants more validation, usability
   wants less friction).

8. **Compose the review summary body** (this becomes the `body` field of the
   GitHub review):

   ```markdown
   ## Code Review: #{number} - {title}

   **Verdict:** [APPROVE | REQUEST_CHANGES | COMMENT]

   [Combined assessment: take each agent's summary and synthesise into 2-3
   sentences covering the overall quality of the PR across all lenses]

   ### Cross-Cutting Themes
   [Issues that multiple lenses identified — these deserve the most attention]
   - **[Theme]** (flagged by: [lenses]) — [description]

   ### Tradeoff Analysis
   [Where different lenses disagree, present both perspectives]
   - **[Quality A] vs [Quality B]**: [description and recommendation]

   [Omit either section if there are no cross-cutting themes or tradeoffs]

   ### Strengths
   - ✅ [Aggregated and deduplicated strengths from all agents]

   ### General Findings
   - [emoji] **[Lens]**: [General findings from all agents, sorted by severity]

   ### Additional Findings
   [Only if more than 10 inline comments were produced and some were deferred]
   - [emoji] `file:line` — [title] ([lens])

   ---
   *Review generated by /review-pr*
   ```

9. **Compose each inline comment body**: Each comment's `body` field should
   already be self-contained from the agent output. For merged comments,
   combine the bodies with a blank line separator and attribute each section
   to its lens.
````

#### 4. Step 5: Rewrite Present the Review

**Location**: Replace the entire "### Step 5: Present the Review" section.

**Replace with**:

````markdown
### Step 5: Present the Review

Present a two-part preview showing exactly what will be posted to the PR:

**Part 1: Review summary** (will become the review's body):

Show the composed summary from Step 4.6 in a markdown code block so the user
can see exactly what will be posted.

**Part 2: Inline comments** (will be attached to specific diff lines):

```
## Proposed Inline Comments ([count] comments)

### [file path 1]
- Line [N]: [emoji] **[Lens]** — [title]
  > [First 1-2 sentences of body as preview]

- Lines [N-M]: [emoji] **[Lens]** — [title]
  > [First 1-2 sentences of body as preview]

### [file path 2]
- Line [N]: [emoji] **[Lens]** — [title]
  > [First 1-2 sentences of body as preview]

[If comments were deferred due to the ~10 cap:]
### Deferred to summary ([count] findings)
- [emoji] [Lens]: [title] — `file:line`
```
````

#### 5. Step 6: Rewrite Follow-Up Options

**Location**: Replace the entire "### Step 6: Offer Follow-Up Options" section.

**Replace with**:

````markdown
### Step 6: Offer Actions

After presenting the preview:

```
The review is ready. Would you like to:
1. Post the review? (summary + [count] inline comments, verdict: [suggested verdict])
2. Change the verdict? (currently: [suggested verdict])
3. Edit or remove specific inline comments before posting?
4. Discuss any findings in more detail?
5. Re-run specific lenses with adjusted focus?
```

**When the user chooses to post** (option 1):

1. Read the HEAD SHA and repo info from the temp directory:
   ```bash
   COMMIT_SHA=$(cat "$REVIEW_DIR/head-sha.txt")
   REPO_INFO=$(cat "$REVIEW_DIR/repo-info.txt")
   ```

2. Construct the review payload as a JSON object containing:
   - `commit_id`: the HEAD SHA
   - `body`: the review summary composed in Step 4.6
   - `event`: the verdict (`"COMMENT"`, `"REQUEST_CHANGES"`, or `"APPROVE"`)
   - `comments`: array of inline comment objects, each with:
     - `path`: file path from the agent's comment
     - `line`: line number from the agent's comment
     - `side`: side from the agent's comment
     - `body`: the self-contained comment body
     - `start_line` and `start_side`: included only if `end_line` is not null.
       For multi-line comments, the agent's fields map to the API's fields
       with an inversion (see "Multi-Line Comment API Mapping" in Phase 1):
       - API `start_line` ← agent's `line` (the beginning of the range)
       - API `start_side` ← agent's `side`
       - API `line` ← agent's `end_line` (the end of the range)
       - API `side` ← agent's `side`

       Example: agent `{line: 10, end_line: 15, side: "RIGHT"}` becomes
       API `{start_line: 10, start_side: "RIGHT", line: 15, side: "RIGHT"}`

3. Post the review:
   ```bash
   jq -n \
     --arg sha "$COMMIT_SHA" \
     --arg body "$REVIEW_BODY" \
     --arg event "$VERDICT" \
     --argjson comments "$COMMENTS_JSON" \
     '{
       commit_id: $sha,
       body: $body,
       event: $event,
       comments: $comments
     }' | gh api "repos/$REPO_INFO/pulls/{number}/reviews" \
       --method POST --input -
   ```

4. Confirm success and show the PR URL:
   ```bash
   gh pr view {number} --json url --jq '.url'
   ```

**If the API returns a 422 error** (typically an invalid line reference or
stale commit):
- Report the error to the user
- If the error indicates an invalid line reference, identify which comment(s)
  caused the failure and offer to retry without them (move them to the summary)
- If the error indicates a stale `commit_id` (the PR's HEAD has changed since
  the review started), re-fetch the HEAD SHA and warn the user that new commits
  were pushed. Offer to retry with the updated SHA, noting that line numbers
  may have shifted and some comments may now be invalid

**When the user chooses to edit comments** (option 3):
- Present each comment with a number
- Allow the user to remove specific comments by number
- Allow the user to edit a comment's body text
- After edits, re-present the preview and offer the same action options

**When the user changes the verdict** (option 2):
- Ask which verdict they prefer (APPROVE, COMMENT, REQUEST_CHANGES)
- Update the summary body and re-present the preview
````

#### 6. Update Important Guidelines

**Location**: In the "## Important Guidelines" section, add:

```markdown
8. **Handle API errors gracefully** — if the review post fails due to invalid
   line references, identify the problematic comments and offer to retry
   without them rather than failing entirely

9. **Cap inline comments at 10** — if agents produce more findings, prioritise
   critical and major severity. Always include all critical findings even if
   that exceeds 10. Move overflow to the summary body. This prevents PR comment
   spam.

10. **Keep positive feedback in the summary** — strengths and good observations
    go in the review body, never as inline comments. Inline comments are
    exclusively for actionable findings.
```

#### 7. Update What NOT to Do

**Location**: In the "## What NOT to Do" section, keep the existing items that
are still relevant and add the new items. The merged list should be:

```markdown
- Don't write review findings to a separate file — all output goes to the
  conversation and then to GitHub via the API
- Don't post inline comments for positive feedback — strengths go in the
  summary only
- Don't post more than ~10 inline comments — prioritise by severity (always
  include all critical findings even if that exceeds 10)
- Don't post generic or vague inline comments — each must be specific and
  actionable
- Don't skip the preview step — always show the user what will be posted
  before posting
- Don't skip the lens selection step — always confirm with the user which
  lenses will run
- Don't present raw agent output — always aggregate and curate into the
  structured format
- Don't run lenses that clearly aren't relevant
- Don't modify any code — this is a read-only review
```

### Success Criteria

#### Automated Verification:

- [x] `commands/review-pr.md` parses as valid markdown
- [x] The file contains references to `gh api` for posting reviews
- [x] The file contains references to `head-sha.txt` and `repo-info.txt`
- [x] The file contains the JSON payload structure for the Reviews API

#### Manual Verification:

- [ ] Run `/review-pr` against a real PR with at least 2 changed files
- [ ] Confirm agents return valid JSON with `comments`, `strengths`,
  `general_findings`, and `summary` fields
- [ ] Confirm the command presents a two-part preview (summary + inline
  comments grouped by file)
- [ ] Confirm diff line validation moves out-of-range comments to general
  findings with a warning in the preview
- [ ] Confirm inline comments are capped at 10 (with all critical included)
- [ ] Confirm the user is offered a choice of verdict before posting
- [ ] Post the review and verify in GitHub's UI:
  - [ ] The review summary appears as the review body
  - [ ] Inline comments appear on the correct files and lines in the
    "Files changed" tab
  - [ ] Severity emojis render correctly
  - [ ] Comment bodies are self-contained and readable
- [ ] Test the 422 error handling: intentionally include a bad line reference
  and confirm the command reports the error and offers to retry

---

## Testing Strategy

### Manual Testing Steps

1. **Agent output validation**: Run a single agent (e.g., `pr-security-reviewer`)
   against a known PR diff and verify the JSON output:
   - Contains valid JSON
   - All `line` values fall within diff hunks
   - `side` values are appropriate (RIGHT for most, LEFT for deleted lines)
   - Comment bodies follow the emoji + lens + issue + impact + suggestion format
   - Strengths and general_findings are populated where appropriate

2. **End-to-end review flow**: Run `/review-pr` against a real PR:
   - Verify lens selection works as before
   - Verify agents run in parallel and return JSON
   - Verify the preview shows summary + inline comments grouped by file
   - Verify posting creates a visible review with inline comments in GitHub
   - Verify the verdict (COMMENT/REQUEST_CHANGES/APPROVE) is applied correctly

3. **Diff line validation**: Verify that the command correctly validates agent
   line numbers against diff hunks:
   - Agent provides a line number outside any hunk → moved to general findings
   - Agent provides a line number within a hunk → kept as inline comment
   - LEFT side comment on a deleted line → validated against old-file ranges

4. **Edge cases**:
   - PR with only one changed file (single-file review)
   - PR with only documentation changes (should skip security, test coverage)
   - PR with deleted files (LEFT side comments)
   - Agent returning malformed JSON (graceful fallback with raw output shown)
   - More than 10 findings across all agents (cap and defer, all critical kept)
   - No findings at all (APPROVE verdict, strengths-only summary)
   - Two agents flagging adjacent lines with unrelated issues (should NOT merge)
   - Two agents flagging the same line with the same concern (should merge)
   - PR HEAD changes between review start and post (stale SHA handling)

## References

- Research document: `meta/research/codebase/2026-02-22-pr-review-inline-comments.md`
- Reference command patterns: `~/.claude/temp-review-gh.md` (ephemeral
  reference that informed the design; not required for implementation)
- GitHub Reviews API: `POST /repos/{owner}/{repo}/pulls/{number}/reviews`
- Current command: `commands/review-pr.md`
- Current agents: `agents/pr-architecture-reviewer.md`,
  `agents/pr-security-reviewer.md`, `agents/pr-code-quality-reviewer.md`,
  `agents/pr-test-coverage-reviewer.md`, `agents/pr-standards-reviewer.md`,
  `agents/pr-usability-reviewer.md`
