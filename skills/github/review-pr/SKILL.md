---
name: review-pr
description: Review a pull request through multiple quality lenses and present a
  compiled analysis with inline comments. Use when the user wants a thorough PR
  review.
argument-hint: "[PR number or URL]"
disable-model-invocation: true
---

# Review PR

You are tasked with reviewing a pull request through multiple quality lenses
and then presenting a compiled analysis of the code changes.

## Initial Response

When this command is invoked:

1. **Check if a PR number or URL was provided**:

- If a PR number or URL was provided as an argument, identify the PR
  immediately
- If optional focus arguments were provided (e.g., "focus on security and
  architecture"), note them for lens selection
- Begin the review process

2. **If no argument provided**, respond with:

```
I'll help you review a pull request. Please provide:
1. The PR number or URL (or I'll check the current branch)
2. (Optional) Focus areas to emphasise (e.g., "focus on security and
   architecture")

Tip: You can invoke this command with arguments:
  `/review-pr 123`
  `/review-pr 123 focus on security and test coverage`
```

Then check if the current branch has a PR:
`gh pr view --json number,url,title,state 2>/dev/null`

If a PR is found on the current branch, offer to review it. If not, wait for
the user's input.

## Available Review Lenses

| Lens               | Lens Skill                    | Focus                                                                  |
|--------------------|-------------------------------|------------------------------------------------------------------------|
| **Architecture**   | `architecture-lens`           | Modularity, coupling, dependency direction, structural drift           |
| **Security**       | `security-lens`               | OWASP Top 10, input validation, auth/authz, secrets, data flows        |
| **Test Coverage**  | `test-coverage-lens`          | Coverage adequacy, assertion quality, test pyramid, anti-patterns       |
| **Code Quality**   | `code-quality-lens`           | Complexity, design principles, error handling, code smells             |
| **Standards**      | `standards-lens`              | Project conventions, API standards, naming, accessibility              |
| **Usability**      | `usability-lens`              | Developer experience, API ergonomics, configuration, onboarding        |
| **Performance**    | `performance-lens`            | Algorithmic efficiency, resource usage, concurrency, caching           |
| **Documentation**  | `documentation-lens`          | Documentation completeness, accuracy, audience fit                     |
| **Database**       | `database-lens`               | Migration safety, schema design, query correctness, integrity          |
| **Correctness**    | `correctness-lens`            | Logical validity, boundary conditions, state management, concurrency   |
| **Compatibility**  | `compatibility-lens`          | API contracts, cross-platform, protocol compliance, deps               |
| **Portability**    | `portability-lens`            | Environment independence, deployment flexibility, vendor lock          |
| **Safety**         | `safety-lens`                 | Data loss prevention, operational safety, protective mechanisms        |

## Process Steps

### Step 1: Identify and Fetch the PR

1. **Get PR metadata**:
   `gh pr view {number} --json number,url,title,state,baseRefName,headRefName`

2. **Create temp directory** at `meta/tmp/pr-review-{number}` (substituting
   the actual PR number):
   ```bash
   mkdir -p meta/tmp/pr-review-{number}
   ```

3. **Fetch diff, changed files, PR description, and commit context**:
   ```bash
   gh pr diff {number} > meta/tmp/pr-review-{number}/diff.patch
   gh pr diff {number} --name-only > meta/tmp/pr-review-{number}/changed-files.txt
   gh pr view {number} --json body --jq '.body' > meta/tmp/pr-review-{number}/pr-description.md
   gh pr view {number} --json commits --jq '.commits[].messageHeadline' > meta/tmp/pr-review-{number}/commits.txt
   ```

4. **Read the diff, changed files list, PR description, and commits** to
   understand scope and intent.

5. **Fetch additional metadata for the Reviews API**:
   ```bash
   gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha' > meta/tmp/pr-review-{number}/head-sha.txt
   gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"' > meta/tmp/pr-review-{number}/repo-info.txt
   ```

   Where `{owner}` and `{repo}` are extracted from the PR metadata already
   fetched in step 1.

**Error handling**: If any `gh` command fails, handle these cases:

- **`gh` not installed or not authenticated**: Inform the user that the `gh`
  CLI is required and suggest running `gh auth login` to authenticate.
- **No default remote repository**: Instruct the user to run
  `gh repo set-default` and select the appropriate repository (mirrors the
  pattern in `/describe-pr`).
- **Cannot determine repo owner/name**: If `gh repo view` fails, instruct the
  user to run `gh repo set-default` and select the appropriate repository.
- **Invalid PR number or PR not found**: Inform the user that the PR could not
  be found and suggest checking the number. If on a branch with no PR, list
  open PRs with `gh pr list --limit 10` and ask the user to select one.
- **Empty diff**: If `diff.patch` is empty (e.g., a draft PR with no changes),
  inform the user and ask whether to proceed with a review of the PR
  description and commits only.

### Step 2: Select Review Lenses

Determine which lenses are relevant based on the PR's scope and any
user-provided focus arguments.

**If the user provided focus arguments:**

- Map the focus areas to the corresponding lenses
- Include any additional lenses that are clearly relevant to the PR's scope
- Briefly explain which lenses you're running and why

**If no focus arguments were provided, auto-detect relevance:**

Take time to think carefully about which lenses apply based on:

- **Architecture** — relevant for most PRs; skip only for trivial single-file
  changes
- **Security** — relevant when changes involve: user input handling, auth/authz,
  data storage, external integrations, API endpoints, secrets/config
- **Test Coverage** — relevant for most PRs; skip only for documentation-only
  or configuration-only changes
- **Code Quality** — relevant for most PRs; skip only for documentation-only
  changes
- **Standards** — relevant when changes involve: API changes, new files/modules,
  public interfaces, naming-heavy changes
- **Usability** — relevant when changes involve: public APIs, CLI interfaces,
  configuration surfaces, breaking changes, developer-facing libraries
- **Performance** — relevant when changes involve: data processing, API
  endpoints handling load, algorithm-heavy code, concurrency resource
  efficiency, caching logic, or hot code paths. Skip for documentation-only,
  configuration-only, or simple UI changes.
- **Documentation** — relevant when changes involve: public APIs, README
  files, configuration surfaces, new features that need documentation,
  breaking changes requiring migration guides. Skip for internal refactoring
  with no interface changes.
- **Database** — relevant when changes involve: database migrations, schema
  changes, new queries, ORM model changes, transaction logic, connection
  pool configuration. Skip for changes with no database interaction.
- **Correctness** — relevant for most PRs; skip only for documentation-only,
  configuration-only, or simple renaming changes.
- **Compatibility** — relevant when changes involve: public API
  modifications, dependency updates, serialisation format changes,
  cross-platform code, protocol implementations. Skip for internal-only
  changes with no external consumers.
- **Portability** — relevant when changes involve: infrastructure
  configuration, deployment scripts, containerisation, cloud provider
  integrations, environment-specific code paths. Skip for application logic
  with no environment dependencies.
- **Safety** — relevant when changes involve: data deletion or modification
  operations, deployment configuration, automated batch processes,
  infrastructure changes, feature flags, or critical system components.
  Skip for read-only features, documentation, or UI-only changes.

**Lens selection cap:** With 13 available lenses, running all of them for
every review would be wasteful. Select the **6 to 8 most relevant lenses**
for the change under review. Apply these prioritisation rules:

1. **Always consider the core four**: Architecture, Code Quality, Test
   Coverage, and Correctness are relevant for most non-trivial changes.
   Include them unless the change is clearly outside their scope (e.g.,
   documentation-only).
2. **Add domain-specific lenses based on the change**: Use the auto-detect
   criteria above to identify which of the remaining lenses are relevant.
3. **If more than 8 lenses pass auto-detect**, rank by relevance to the
   specific change and drop the least relevant until you reach 6-8. Prefer
   lenses whose core responsibilities directly overlap with the change's
   primary concerns.
4. **If the user provided focus arguments**, prioritise the requested lenses
   and fill remaining slots (up to 8) with the most relevant auto-detected
   lenses.
5. **Never run fewer than 4 lenses** unless the change is trivially scoped
   (e.g., a typo fix).

When presenting the lens selection, clearly indicate which lenses are
selected and which are skipped, with a brief reason for each skip.

Present lens selection to the user before proceeding:

```
Based on the PR's scope, I'll review through these lenses:
- Architecture: [reason]
- Security: [reason — or "Skipping: no security-sensitive changes identified"]
- Test Coverage: [reason]
- Code Quality: [reason]
- Standards: [reason — or "Skipping: ..."]
- Usability: [reason — or "Skipping: ..."]
- Performance: [reason — or "Skipping: no performance-sensitive changes identified"]
- Documentation: [reason — or "Skipping: ..."]
- Database: [reason — or "Skipping: no database changes identified"]
- Correctness: [reason]
- Compatibility: [reason — or "Skipping: ..."]
- Portability: [reason — or "Skipping: ..."]
- Safety: [reason — or "Skipping: ..."]

Shall I proceed, or would you like to adjust the selection?
```

Wait for confirmation before spawning reviewers.

### Step 3: Spawn Review Agents

For each selected lens, spawn the generic `reviewer` agent with a prompt
that includes paths to the lens skill and output format files. Do NOT read
these files yourself — the agent reads them in its own context.

Compose each agent's prompt following this template:

```
You are reviewing pull request changes through the [lens name] lens.

## Context

The PR artefacts are in the temp directory at meta/tmp/pr-review-{number}:
- `diff.patch` — the full diff
- `changed-files.txt` — list of changed file paths
- `pr-description.md` — PR description
- `commits.txt` — commit messages

PR number: [number]

## Analysis Strategy

1. Read your lens skill and output format files (see paths below)
2. Read `diff.patch` and `changed-files.txt` from the temp directory
3. Read `pr-description.md` and `commits.txt` for intent context
4. Explore the codebase to understand the architectural landscape around
   the changes
5. Evaluate the changes through your lens, applying each key question
6. Identify beyond-the-diff impact — trace how changes affect consumers
7. Anchor findings to precise diff line numbers (lines must be within
   diff hunks)

## Lens

Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md

## Output Format

Read the output format at: ${CLAUDE_PLUGIN_ROOT}/skills/review/output-formats/pr-review-output-format/SKILL.md

IMPORTANT: Return your analysis as a single JSON code block. Do not include
prose outside the JSON block.
```

Spawn all selected agents **in parallel** using the Task tool with
`subagent_type: "reviewer"`.

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

### Step 5: Present the Review

Present a two-part preview showing exactly what will be posted to the PR:

**Part 1: Review summary** (will become the review's body):

Show the composed summary from Step 4.8 in a markdown code block so the user
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

1. Read the HEAD SHA and repo info from the temp directory at
   `meta/tmp/pr-review-{number}/head-sha.txt` and
   `meta/tmp/pr-review-{number}/repo-info.txt` using the Read tool.

2. Construct the review payload as a JSON object containing:
   - `commit_id`: the HEAD SHA
   - `body`: the review summary composed in Step 4.8
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

3. Write the review payload JSON to
   `meta/tmp/pr-review-{number}/review-payload.json`, then post the review:
   ```bash
   gh api repos/{owner}/{repo}/pulls/{number}/reviews \
     --method POST --input meta/tmp/pr-review-{number}/review-payload.json
   ```

   Where `{owner}/{repo}` are the values read from `repo-info.txt`.

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

## Important Guidelines

1. **Read the diff before doing anything else** — you need complete context to
   select lenses and brief the agents properly

2. **Spawn agents in parallel** — the review lenses are independent and should
   run concurrently for efficiency

3. **Synthesise, don't concatenate** — your value is in compiling a balanced
   view across lenses, identifying themes and tradeoffs, and prioritising
   actionable recommendations. Don't just paste seven reports together.

4. **Be balanced** — highlight strengths alongside concerns. A PR that makes
   good architectural decisions but has security gaps should get credit for
   both.

5. **Prioritise by impact** — structural issues that are hard to fix later
   matter more than surface-level concerns. A critical finding from one lens
   outweighs minor findings from all seven.

6. **Respect tradeoffs** — when lenses conflict, present both sides and let the
   user decide. Don't privilege one quality attribute over another without
   justification.

7. **Clean up temp directory only at session end** — agents may need to
   re-reference the PR context during follow-up discussion.

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

## What NOT to Do

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

## Relationship to Other Commands

The PR review sits in the development lifecycle alongside other commands:

1. `/create-plan` — Create the implementation plan
2. `/review-plan` — Review and iterate the plan quality
3. `/implement-plan` — Execute the approved plan
4. `/validate-plan` — Verify implementation matches the plan
5. `/describe-pr` — Generate PR description
6. `/review-pr` — Review the PR through quality lenses (this command)
