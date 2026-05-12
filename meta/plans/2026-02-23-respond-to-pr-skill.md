# Respond to PR Skill Implementation Plan

## Overview

Create a new skill `/respond-to-pr` that allows users to interactively work
through outstanding pull request review feedback. The skill fetches all
unresolved comments and review requests, triages them by priority, then walks
through each item one at a time — verifying the feedback against the codebase,
confirming the approach with the user, making changes, responding on GitHub, and
resolving the thread. After all items are addressed, it offers to re-request
review from the original reviewers.

## Current State Analysis

The skills directory at `~/.claude/skills/` contains 17 skills following a
consistent pattern. The development lifecycle currently covers planning,
implementation, validation, PR description, and PR review — but has no skill
for **responding** to review feedback. This is a gap: after `/review-pr`
generates feedback (or a human reviewer leaves comments), there is no structured
workflow for addressing that feedback.

### Key Discoveries:

- All workflow skills use `disable-model-invocation: true` (user-triggered only)
  — `~/.claude/skills/review-pr/SKILL.md:7`
- PR-related skills accept a PR number or URL as argument —
  `~/.claude/skills/review-pr/SKILL.md:6`
- `gh` CLI is the universal GitHub interface; `gh api` for REST/GraphQL calls —
  `~/.claude/skills/review-pr/SKILL.md:63-85`
- Error handling for `gh` failures follows a standard pattern (not
  authenticated,
  no default remote, PR not found) —
  `~/.claude/skills/review-pr/SKILL.md:87-101`
- Interactive confirmation before visible actions is standard —
  `~/.claude/skills/review-pr/SKILL.md:148-150`
- The `implement-plan` skill shows the pattern for making code changes with
  user checkpoints — `~/.claude/skills/implement-plan/SKILL.md:56-65`
- The `commit` skill handles atomic commit creation with user confirmation —
  `~/.claude/skills/commit/SKILL.md:36-42`

### Patterns to Follow:

- YAML frontmatter with `name`, `description`, `argument-hint`,
  `disable-model-invocation`
- Title as `# Skill Name`
- Role statement: "You are tasked with..."
- Steps as `### Step N: Name`
- "Important Guidelines" as numbered list
- "What NOT to Do" as explicit anti-patterns
- "Relationship to Other Commands" section

### GitHub API Operations Required:

| Operation                                    | Command                                                                                                          |
|----------------------------------------------|------------------------------------------------------------------------------------------------------------------|
| Get PR metadata                              | `gh pr view {n} --json number,url,title,state,baseRefName,headRefName`                                           |
| Get repo owner/name                          | `gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'`                                               |
| Get current user                             | `gh api user --jq '.login'`                                                                                      |
| Fetch review threads (GraphQL, primary)      | `gh api graphql` with `pullRequest.reviewThreads` query (paginated via cursor)                                   |
| List reviews (top-level, REST)               | `gh api repos/{owner}/{repo}/pulls/{n}/reviews --paginate`                                                       |
| List issue comments (conversation, REST)     | `gh api repos/{owner}/{repo}/issues/{n}/comments --paginate`                                                     |
| Reply to review comment thread               | `gh api repos/{owner}/{repo}/pulls/{n}/comments/{id}/replies --method POST -f body="..."`                        |
| Post top-level PR comment                    | `gh pr comment {n} --body "..."`                                                                                 |
| Resolve review thread (GraphQL)              | `gh api graphql -f query='mutation { resolveReviewThread(input: {threadId: "..."}) { thread { isResolved } } }'` |
| Re-request review                            | `echo '{"reviewers":[...]}' \| gh api repos/{owner}/{repo}/pulls/{n}/requested_reviewers --method POST --input -`|

## Desired End State

A new skill at `~/.claude/skills/respond-to-pr/SKILL.md` that:

1. Is invocable via `/respond-to-pr [PR number or URL]`
2. Fetches all outstanding review feedback from a PR
3. Filters to only unresolved feedback
4. Triages feedback by priority (blocking > simple > complex)
5. Presents an organised summary to the user for confirmation
6. Works through each item interactively:
  - Verifies the feedback against the codebase
  - Presents analysis and proposed approach to the user
  - Makes code changes upon confirmation
  - Offers the user a chance to commit after each item
  - Replies in the GitHub comment thread
  - Resolves the review thread via GraphQL
7. After all items: offers to re-request review
8. Follows all established skill conventions

### Verification:

- File exists at `~/.claude/skills/respond-to-pr/SKILL.md`
- YAML frontmatter is valid and follows conventions
- Skill is listed when running `/help` or checking available skills
- Skill can be invoked with `/respond-to-pr 123` and follows the documented
  workflow

## What We're NOT Doing

- **No sub-agents**: The work is inherently sequential (one item at a time), so
  no reviewer or analyser agents are needed
- **No new agent definitions**: No files in `~/.claude/agents/`
- **No output format skills**: Responses are conversational, not structured JSON
- **No automated testing**: This is a skill definition (markdown), not code
- **No changes to existing skills**: Only creating a new skill
- **No automatic re-requesting of review**: Only offering to do so

## Implementation Approach

Create a single file `~/.claude/skills/respond-to-pr/SKILL.md` containing the
complete skill definition. The skill follows the established workflow pattern
most similar to `implement-plan` (sequential, interactive, makes code changes)
combined with the GitHub API patterns from `review-pr` and `describe-pr`.

The skill adopts key principles from the obra/superpowers example:

- Verify feedback against the codebase before implementing
- Priority ordering: blocking > simple fixes > complex changes
- Factual acknowledgement style ("Fixed. [description]"), no performative
  agreement
- Push back when feedback is technically incorrect, with reasoning

## Phase 1: Create the respond-to-pr Skill

### Overview

Create the skill directory and SKILL.md file with the complete workflow
definition.

### Changes Required:

#### 1. Create skill directory and file

**File**: `~/.claude/skills/respond-to-pr/SKILL.md` (new file)

**Heading levels in the generated skill** (must match existing conventions):
- `#` — Skill title only
- `##` — Major sections: Initial Response, Process Steps, Important Guidelines,
  What NOT to Do, Relationship to Other Commands
- `###` — Steps within Process Steps: Step 1, Step 2, etc.

**Structure** (sections in order):

```markdown
---
name: respond-to-pr
description: ...
argument-hint: "[PR number or URL]"
disable-model-invocation: true
---

# Respond to PR

You are tasked with ...

## Initial Response
...

## Process Steps

### Step 1: Fetch PR Context and Outstanding Feedback
...

### Step 2: Filter and Triage Feedback
...

(etc.)

## Important Guidelines
...

## What NOT to Do
...

## Relationship to Other Commands
...
```

**Detailed content for each section:**

##### YAML Frontmatter

```yaml
---
name: respond-to-pr
description: Respond to pull request review feedback interactively, working
  through each item with verification and code changes. Use when the user wants
  to address PR review comments.
argument-hint: "[PR number or URL]"
disable-model-invocation: true
---
```

##### Title and Role Statement

```markdown
# Respond to PR

You are tasked with working through outstanding pull request review feedback
in a systematic, interactive fashion. For each piece of feedback, you verify
it against the codebase, confirm the approach with the user, make changes,
and respond on GitHub.
```

##### Initial Response

Follow the established pattern from `review-pr`:

- If PR number/URL provided: identify the PR immediately, begin the process
- If no argument: prompt the user, also check the current branch for a PR
  using `gh pr view --json number,url,title,state 2>/dev/null`
- If current branch has a PR, offer to use it
- Tip showing argument usage: `/respond-to-pr 123`

##### Step 1: Fetch PR Context and Outstanding Feedback

1. **Get PR metadata**:
   ```bash
   gh pr view {number} --json number,url,title,state,baseRefName,headRefName
   ```

2. **Validate PR state**: Check that `state` is `OPEN`. If the PR is closed
   or merged, inform the user and ask whether they still want to proceed
   (they may want to make changes for a follow-up, but posting responses
   and resolving threads on a closed PR is typically not useful). If the user
   declines, exit the workflow.

3. **Get repo info and current user**:
   ```bash
   gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'
   gh api user --jq '.login'
   ```

4. **Ensure on the correct branch**: Check if the user is on the PR's head
   branch. If not, inform them and ask if they want to switch:
   ```bash
   git branch --show-current
   ```
   Compare with the PR's `headRefName`. If different, ask the user whether to
   switch to it.

5. **Fetch review threads via GraphQL** (primary source for inline feedback):

   Use the GraphQL API as the single source of truth for review thread data.
   This avoids the need to correlate REST and GraphQL IDs — each thread node
   includes the thread ID (for resolution), resolution status, and all comment
   data in one query.

   ```bash
   gh api graphql -f query='
     query($owner: String!, $repo: String!, $number: Int!, $cursor: String) {
       repository(owner: $owner, name: $repo) {
         pullRequest(number: $number) {
           reviewThreads(first: 100, after: $cursor) {
             pageInfo { hasNextPage endCursor }
             nodes {
               id
               isResolved
               isOutdated
               path
               line
               comments(first: 100) {
                 nodes {
                   id
                   databaseId
                   body
                   author { login }
                   createdAt
                 }
               }
             }
           }
         }
       }
     }
   ' -f owner="{owner}" -f repo="{repo}" -F number={number}
   ```

   **Pagination**: Check `pageInfo.hasNextPage`. If true, re-run the query
   with `-f cursor="{endCursor}"` to fetch the next page. Repeat until all
   threads are fetched. If the total exceeds 100 threads, inform the user of
   the count so they know the full scope.

6. **Fetch top-level reviews and issue comments via REST**:

   These are not part of the review thread model and require REST:

   ```bash
   # Reviews (top-level review bodies and verdicts)
   gh api repos/{owner}/{repo}/pulls/{number}/reviews --paginate

   # Issue comments (conversation-level discussion)
   gh api repos/{owner}/{repo}/issues/{number}/comments --paginate
   ```

   **Note**: Issue comments and top-level review bodies cannot be "resolved"
   via the `resolveReviewThread` mutation — they are not part of the review
   thread model. These items follow a "respond only" path in Step 4 (skip
   the resolve step).

7. **Error handling** (standard pattern):
  - `gh` not installed or not authenticated: suggest `gh auth login`
  - No default remote: suggest `gh repo set-default`
  - PR not found: inform user, list open PRs with `gh pr list --limit 10`
  - Rate limiting (HTTP 403 with rate limit headers): inform the user,
    suggest waiting and retrying
  - GraphQL errors: check for an `errors` key in the response before
    processing `data`; report specific error messages to the user
  - Network/connectivity failures: suggest checking `gh auth status`

   **Mid-workflow error recovery** (for Steps 4e and 4f):
  - Thread reply fails (404 — thread deleted or outdated after force-push):
    offer to post as a top-level PR comment instead
  - Thread resolution fails (permission or stale thread ID): inform user
    the thread was not resolved; suggest resolving manually on GitHub
  - Partial failure (code changed but response not posted): clearly state
    what succeeded and what failed so the user knows the current state

##### Step 2: Filter and Triage Feedback

1. **Filter out noise**:
  - Exclude resolved review threads (using GraphQL `isResolved` data)
  - Exclude reviews with verdict `APPROVED` that have no body text
  - Exclude bot comments (e.g., CI bots) unless they contain actionable
    feedback

2. **Categorise each piece of feedback**:

   | Category | Description | Examples |
      |----------|-------------|---------|
   | **Blocking** | Must fix before merge | Changes requested, security issues, bugs |
   | **Simple** | Quick fixes, low risk | Typos, naming, import ordering, small refactors |
   | **Complex** | Requires thought/discussion | Architecture changes, design decisions, large refactors |
   | **Question** | Needs a response, not code changes | Clarification requests, "why did you..." |
   | **Disagreement** | Feedback appears incorrect or inappropriate | Suggestions that would break things, misunderstandings |

3. **Group by review thread**: Keep inline comments that belong to the same
   review thread together — they share context and should be addressed as a
   unit.

4. **Order by priority**: Blocking > Simple > Complex > Question >
   Disagreement. Within the same category, order by review (address one
   reviewer's feedback before moving to the next for coherence).

5. **Handle empty result**: If no outstanding feedback remains after
   filtering (all threads resolved, no actionable comments), inform the
   user:
   ```
   No outstanding feedback on PR #{number}. All review threads are resolved.
   ```
   If there are reviewers who previously requested changes, offer to
   re-request review from them. Then exit the workflow.

##### Step 3: Present Feedback Summary and Gather Preferences

Present the triaged feedback to the user with a total count upfront:

```
## Outstanding Feedback for PR #{number}: {title}

Found **{total} items** of outstanding feedback ({blocking_count} blocking,
{simple_count} simple, {complex_count} complex, {question_count} questions,
{disagreement_count} potential disagreements).

### Blocking ({count})
1. **{reviewer}** on `{file}:{line}` — {summary of comment}
2. ...

### Simple Fixes ({count})
3. **{reviewer}** on `{file}:{line}` — {summary of comment}
4. ...

### Complex Changes ({count})
5. **{reviewer}** — {summary of review body or comment}
6. ...

### Questions to Answer ({count})
7. **{reviewer}** on `{file}:{line}` — {summary of question}
8. ...

### Potential Disagreements ({count})
9. **{reviewer}** on `{file}:{line}` — {summary + why it may be wrong}
10. ...
```

After the summary, gather workflow preferences:

```
Before we start, a few preferences:

1. **Working mode**:
   a. **Guided** — confirm each item individually (recommended for complex/
      disagreement items)
   b. **Express** — for Simple items, present change + response together and
      apply without per-step confirmation; Guided for all other categories

2. **Commit strategy**:
   a. Commit after each item
   b. Commit after each category
   c. Commit at the end (or when you say so)

3. **Thread resolution**:
   a. Resolve threads after responding (default)
   b. Leave threads for reviewers to resolve

4. **Order**: Proceed in the order above, or re-prioritise / skip items?
```

Wait for user confirmation and preferences before proceeding.

##### Step 4: Work Through Each Item

At any point during this step, the user can:
- **Skip**: "Skip this item" — mark as deferred, move to the next item
- **Stop**: "Stop here" — present a progress summary of what was addressed
  vs. remaining, and exit the workflow

For each feedback item, follow this cycle (adapting to guided vs express
mode as selected in Step 3):

**4a. Verify the feedback:**

- Read the relevant code in the codebase (not just the diff)
- Understand the reviewer's concern in full context
- Check if the feedback is technically correct
- Check if the suggested change would break anything

**4b. Present analysis to the user:**

For items where the feedback is correct:

```
### Item {N}: {summary}
**Reviewer**: {name} on `{file}:{line}`
**Feedback**: {quote or summary}

**Analysis**: {explanation of the issue and what the reviewer is asking for}

**Proposed change**: {description of what you'll do}
**Draft response**: "Fixed. {concise description of what will be changed}"

Shall I proceed with this change?
```

For items where the feedback seems incorrect (Disagreement category):

```
### Item {N}: {summary}
**Reviewer**: {name} on `{file}:{line}`
**Feedback**: {quote or summary}

**Analysis**: I've verified this against the codebase and believe the current
implementation is correct because:
- {reason 1}
- {reason 2}

**Draft response** (if pushing back): "{technical reasoning}"

**Options**:
1. Push back with this reasoning
2. Implement the suggestion anyway
3. Propose an alternative approach

What would you prefer?
```

For questions (no code changes needed):

```
### Item {N}: {summary}
**Reviewer**: {name} on `{file}:{line}`
**Question**: {quote}

**Draft response**: {response explaining the reasoning}

Shall I post this response, or would you like to adjust it?
```

In all cases, show the user the exact text that will be posted to GitHub
before posting it. The user can edit the draft response before it is sent.

**4c. Implement the change** (upon user confirmation):

- Make the code changes using Edit/Write tools
- Read surrounding code for context before editing
- Keep changes focused on what the feedback asked for

**4d. Commit** (according to the user's preference from Step 3):

- **Per-item**: Offer to commit after this item. Show files modified and a
  suggested commit message.
- **Per-category**: Track changes. When the current category is complete,
  offer to commit all changes in that category.
- **At-end**: Track changes but don't offer to commit until Step 5.

When committing, follow the `commit` skill pattern: `git add` specific
files, create the commit with the suggested message (or user's amended
message). Never use `git add -A` or `git add .`.

**4e. Respond on GitHub:**

Reply in the specific comment thread (not as a top-level comment):

```bash
# For inline review comments — reply in the thread
gh api repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies \
  --method POST -f body="Fixed. {concise description of what was changed}"
```

For questions or disagreements:

```bash
gh api repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies \
  --method POST -f body="{response text}"
```

For top-level review body comments:

```bash
gh pr comment {number} --body "{response text}"
```

Response style:

- Factual and concise: "Fixed. Renamed the variable to match the convention."
- No performative agreement: NOT "Great point! You're absolutely right!"
- For pushback: Technical reasoning, not defensiveness
- For questions: Clear explanation of the reasoning

**4f. Resolve the review thread** (review thread items only):

For items that belong to a review thread (inline code comments from the
GraphQL query), resolve the thread via GraphQL if the user chose to
auto-resolve in their preferences:

```bash
gh api graphql -f query='
  mutation($threadId: ID!) {
    resolveReviewThread(input: {threadId: $threadId}) {
      thread { isResolved }
    }
  }
' -f threadId="{thread_node_id}"
```

The `threadId` is the GraphQL node `id` from the review threads query in
Step 1.

**For issue comments and top-level review body comments**: Skip this step.
These are not part of the review thread model and cannot be resolved via the
GitHub API. Note this in the item's completion message.

**4g. Move to next item:**

```
Item {N} addressed and thread resolved. Moving to item {N+1}...
```

Repeat the cycle for each remaining item.

##### Step 5: Wrap Up

After all items have been addressed (or the user says "stop here"):

1. **Present summary**:
   ```
   ## Summary

   {addressed_count} of {total_count} feedback items addressed:
   - {change_count} changes implemented
   - {question_count} questions answered
   - {pushback_count} items pushed back on
   - {skip_count} items skipped/deferred
   - {commit_count} commits created

   Changes are on branch `{branch_name}`.
   ```

   If items were skipped or deferred, note:
   ```
   {skip_count} items were skipped. You can re-run `/respond-to-pr {number}`
   to pick up remaining unresolved items.
   ```

2. **Offer to push** (if there are unpushed commits):
   ```
   Would you like me to push these changes to the remote?
   ```

3. **Offer to re-request review**:
   ```
   The following reviewers left feedback:
   - {reviewer1} (requested changes)
   - {reviewer2} (commented)

   Would you like me to re-request review from them?
   ```

   If yes:
   ```bash
   echo '{"reviewers":["reviewer1","reviewer2"]}' | \
     gh api repos/{owner}/{repo}/pulls/{number}/requested_reviewers \
       --method POST --input -
   ```

##### Important Guidelines

1. **Verify before implementing** — Always check the reviewer's suggestion
   against the actual codebase before making changes. The reviewer may not
   have full context.

2. **One item at a time** — Work through feedback sequentially. In guided
   mode, each item gets its own verify-implement-respond cycle. In express
   mode, simple items can be presented and applied more efficiently.

3. **Confirm before acting** — In guided mode, always present your analysis
   and proposed approach before making code changes or posting GitHub
   responses. In express mode, present the change and response together
   but still show what will be done before doing it.

4. **Factual responses only** — Reply with what was done ("Fixed. Extracted
   the helper into a shared utility."), not with performative agreement
   ("Great catch! You're absolutely right!").

5. **Technical pushback when warranted** — If feedback is incorrect, present
   the technical reasoning to the user. Don't implement changes you believe
   are wrong without flagging them.

6. **Respect commit preference** — Follow the user's chosen commit strategy
   (per-item, per-category, or at-end). Keep commits atomic and focused.
   Follow the `commit` skill pattern (specific file adds, no `-A`).

7. **Reply in threads** — Always reply in the specific comment thread, not
   as a new top-level comment. This keeps the conversation organised.

8. **Respect the user's judgment** — For disagreements, present your analysis
   but let the user decide. They may have context you don't.

9. **Handle errors gracefully** — Follow the error recovery guidance in
   Step 1.7. For mid-workflow failures, clearly state what succeeded and
   what failed, and offer specific alternatives (e.g., post as top-level
   comment, resolve manually, retry).

10. **Don't over-change** — When implementing feedback, change only what was
    asked for. Don't refactor surrounding code, add comments to unchanged
    lines, or "improve" things that weren't mentioned.

11. **Re-invocation is resumption** — If the user stops mid-way and later
    re-runs `/respond-to-pr {number}`, the resolved-thread filter in Step 2
    naturally excludes already-addressed items. Inform the user: "I see N
    threads were already resolved, picking up from the remaining M items."

##### What NOT to Do

- Don't make code changes without user confirmation
- Don't post GitHub responses without showing the user first
- Don't implement all changes at once — work through them one at a time
- Don't skip the verification step — always check the codebase first
- Don't use performative language in GitHub responses
- Don't commit without following the user's chosen commit strategy
- Don't resolve threads without having addressed the feedback
- Don't automatically re-request review — always offer first
- Don't add co-author information or Claude attribution to commits
- Don't use `git add -A` or `git add .` — add specific files only

##### Relationship to Other Commands

The respond-to-pr skill fills a gap in the development lifecycle:

1. `/create-plan` — Create the implementation plan
2. `/review-plan` — Review and iterate the plan quality
3. `/implement-plan` — Execute the approved plan
4. `/validate-plan` — Verify implementation matches the plan
5. `/describe-pr` — Generate PR description
6. `/review-pr` — Review the PR through quality lenses
7. **`/respond-to-pr`** — Address review feedback (this command)
8. `/commit` — Commit changes (used within this workflow)

### Success Criteria:

#### Automated Verification:

- [x] File exists: `test -f ~/.claude/skills/respond-to-pr/SKILL.md`
- [x] YAML frontmatter is parseable:
  `head -6 ~/.claude/skills/respond-to-pr/SKILL.md | grep -q 'name: respond-to-pr'`
- [x] Contains all required sections: title, role statement, initial response,
  steps 1-5, important guidelines, what not to do, relationship to other
  commands

#### Manual Verification:

- [ ] `/respond-to-pr` appears in available skills
- [ ] `/respond-to-pr 123` can be invoked and follows the documented workflow
  against a real PR with review comments
- [ ] GitHub API commands in the skill work correctly (fetch threads via
  GraphQL, reply to threads, resolve threads)
- [ ] The interactive flow feels natural — user has clear control at each step
- [ ] Guided and express modes work as documented
- [ ] All three commit strategies work (per-item, per-category, at-end)
- [ ] Skip and stop-here work correctly, with accurate progress summaries
- [ ] Re-invocation correctly picks up only unresolved items
- [ ] Thread resolution preference (auto-resolve vs leave-for-reviewer) works
- [ ] Re-request review workflow works correctly
- [ ] Zero-feedback case produces a clear message and exits gracefully
- [ ] Closed/merged PR produces a warning and asks before proceeding

---

## Testing Strategy

### Manual Testing Steps:

1. Create a test PR with review comments of various types (inline, top-level,
   questions, change requests)
2. Invoke `/respond-to-pr {number}` and verify:
  - All feedback is fetched and correctly categorised
  - Resolved threads are filtered out
  - Priority ordering is sensible
3. Work through 2-3 items and verify:
  - Verification step catches incorrect feedback
  - Code changes are focused and correct
  - Commits are atomic (one per item)
  - GitHub thread replies appear in the correct threads
  - Threads are resolved after addressing
4. Test the wrap-up flow:
  - Summary is accurate
  - Push offer works
  - Re-request review offer works

## References

- Research document:
  `~/.claude/meta/research/codebase/2026-02-23-respond-to-pr-feedback-skill.md`
- Primary pattern (PR API interaction):
  `~/.claude/skills/review-pr/SKILL.md`
- Pattern (GitHub write operations):
  `~/.claude/skills/describe-pr/SKILL.md`
- Pattern (code changes with user checkpoints):
  `~/.claude/skills/implement-plan/SKILL.md`
- Pattern (commit workflow):
  `~/.claude/skills/commit/SKILL.md`
- Example skill (behavioural principles):
  [obra/superpowers receiving-code-review](https://github.com/obra/superpowers/blob/main/skills/receiving-code-review/SKILL.md)
- GitHub REST
  API: [Pull Request Comments](https://docs.github.com/en/rest/pulls/comments)
- GitHub REST
  API: [Pull Request Reviews](https://docs.github.com/en/rest/pulls/reviews)
- GitHub
  GraphQL: [resolveReviewThread](https://docs.github.com/en/graphql/reference/mutations#resolvereviewthread)
