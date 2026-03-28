---
name: respond-to-pr
description: Respond to pull request review feedback interactively, working
  through each item with verification and code changes. Use when the user wants
  to address PR review comments.
argument-hint: "[PR number or URL]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Respond to PR

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh respond-to-pr`

**PR reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`

You are tasked with working through outstanding pull request review feedback
in a systematic, interactive fashion. For each piece of feedback, you verify
it against the codebase, confirm the approach with the user, make changes,
and respond on GitHub.

## Initial Response

When this command is invoked:

1. **Check if a PR number or URL was provided**:

- If a PR number or URL was provided as an argument, identify the PR
  immediately
- Begin the process

2. **If no argument provided**, respond with:

```
I'll help you respond to pull request review feedback. Please provide:
1. The PR number or URL (or I'll check the current branch)

Tip: You can invoke this command with an argument:
  `/respond-to-pr 123`
```

Then check if the current branch has a PR:
`gh pr view --json number,url,title,state 2>/dev/null`

If a PR is found on the current branch, offer to use it. If not, wait for
the user's input.

## Process Steps

### Step 1: Fetch PR Context and Outstanding Feedback

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

4. **Ensure on the correct branch**: Check the current branch or bookmark
   using the appropriate VCS command for this repository (refer to the
   session's VCS context). Compare with the PR's `headRefName`. If different,
   inform the user and ask if they want to switch.

5. **Check for a structured review artifact**:

   Look for `{pr reviews directory}/{number}-review-*.md` (e.g.,
   `123-review-1.md`, `123-review-2.md`). If one or more files match:

- Load the highest-numbered file (the most recent review cycle)
- Read the review document
- Extract the verdict, lenses used, and per-lens findings
- Note the severity and confidence of each finding
- This will inform the triage in Step 2

If no matching files are found, proceed without review context — the
skill works the same as before, just without the additional context.

6. **Fetch review threads via GraphQL** (primary source for inline feedback):

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

7. **Fetch top-level reviews and issue comments via REST**:

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

8. **Error handling** (standard pattern):
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

### Step 2: Filter and Triage Feedback

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

3. **Group by review thread**: Keep inline comments that belong to the same
   review thread together — they share context and should be addressed as a
   unit.

4. **Order by priority**: Blocking > Simple > Complex > Question >
   Disagreement. Within the same category, order by reviewer (address one
   reviewer's feedback before moving to the next for coherence).

5. **Handle empty result**: If no outstanding feedback remains after
   filtering (all threads resolved, no actionable comments), inform the
   user:
   ```
   No outstanding feedback on PR #{number}. All review threads are resolved.
   ```
   If there are reviewers who previously requested changes, offer to
   re-request review from them. Then exit the workflow.

### Step 3: Present Feedback Summary and Gather Preferences

If a review artifact was loaded, add a note after the header:

```
> Review context loaded from `{pr reviews directory}/{number}-review-{N}.md`
> (verdict: {verdict}, {N} lenses, {date})
```

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

### Step 4: Work Through Each Item

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

When committing, follow the `commit` skill pattern using the appropriate
VCS commands for this repository (refer to the session's VCS context).
Keep commits focused and atomic.

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

### Step 5: Wrap Up

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

## Important Guidelines

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
   Follow the `commit` skill pattern for this repository's VCS (refer to
   session VCS context). Keep commits focused and atomic.

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

## What NOT to Do

- Don't make code changes without user confirmation
- Don't post GitHub responses without showing the user first
- Don't implement all changes at once — work through them one at a time
- Don't skip the verification step — always check the codebase first
- Don't use performative language in GitHub responses
- Don't commit without following the user's chosen commit strategy
- Don't resolve threads without having addressed the feedback
- Don't automatically re-request review — always offer first
- Don't add co-author information or Claude attribution to commits
- When staging files, always add specific files by name — never bulk-add

## Relationship to Other Commands

The respond-to-pr skill fills a gap in the development lifecycle:

1. `/create-plan` — Create the implementation plan
2. `/review-plan` — Review and iterate the plan quality
3. `/implement-plan` — Execute the approved plan
4. `/validate-plan` — Verify implementation matches the plan
5. `/describe-pr` — Generate PR description
6. `/review-pr` — Review the PR through quality lenses
7. **`/respond-to-pr`** — Address review feedback (this command)
8. `/commit` — Commit changes (used within this workflow)

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh respond-to-pr`
