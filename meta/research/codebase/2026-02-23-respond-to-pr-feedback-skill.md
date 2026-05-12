---
date: "2026-02-23T01:30:00+00:00"
researcher: Toby Clemson
git_commit: N/A
branch: N/A
repository: .claude
topic: "New skill for responding to pull request feedback interactively"
tags: [ research, codebase, skills, pull-requests, code-review, github ]
status: complete
last_updated: "2026-02-23"
last_updated_by: Toby Clemson
---

# Research: New Skill for Responding to Pull Request Feedback

**Date**: 2026-02-23T01:30:00+00:00
**Researcher**: Toby Clemson
**Repository**: .claude (Claude Code configuration)

## Research Question

How should we design a new skill for responding to pull request feedback in a
systematic fashion? The skill should:

- Be triggered for a given PR number or URL
- Read outstanding comments and requests on the pull request
- Interactively work through each piece of feedback
- Make changes to the codebase as needed
- Respond to comments on GitHub when addressed
- Be consistent with the existing skills in this repository

An example skill at
[obra/superpowers/receiving-code-review](https://github.com/obra/superpowers/blob/main/skills/receiving-code-review/SKILL.md)
was provided as inspiration.

## Summary

The codebase has 18 user-level skills following a consistent pattern: each is a
directory under `~/.claude/skills/` containing a `SKILL.md` with YAML
frontmatter and detailed step-by-step instructions. The most relevant existing
skills are `review-pr` (which reads PR data and orchestrates parallel review
agents), `describe-pr` (which writes PR descriptions using `gh`), and
`implement-plan` (which executes changes phase by phase with verification). The
new skill would combine patterns from all three: fetching PR data like
`review-pr`, making code changes like `implement-plan`, and interacting with
GitHub like `describe-pr`. The example skill from `obra/superpowers` provides
good principles for *how to evaluate* feedback (verify before implementing, push
back when wrong) but is purely behavioural guidance rather than a procedural
workflow -- our skill needs to be a concrete, step-by-step workflow consistent
with the conventions in this repository.

## Detailed Findings

### 1. Existing Skills: Conventions and Patterns

All user-level skills follow a consistent structure:

**YAML Frontmatter Pattern:**

```yaml
---
name: <kebab-case-name>
description: <what it does and when to use it, 1-2 sentences>
argument-hint: "[description of expected arguments]"
disable-model-invocation: true  # All workflow skills use this
---
```

**Common Structural Elements:**

- **Title**: `# Skill Name` (imperative or noun phrase)
- **Role Statement**: "You are tasked with..." (1-2 sentences)
- **Initial Response / Getting Started**: How to handle invocation with and
  without arguments; what to prompt if no arguments provided; a "Tip" showing
  argument usage
- **Process Steps**: Numbered phases (`### Step N: Name`) with detailed
  instructions
- **Important Guidelines**: Numbered list of behavioural rules
- **What NOT to Do**: Explicit anti-patterns
- **Relationship to Other Commands**: Where this skill fits in the workflow
  lifecycle

**Key Conventions Observed:**

- All workflow skills set `disable-model-invocation: true` (user-triggered only)
- All PR-related skills accept a PR number or URL as argument
- `gh` CLI is the universal interface to GitHub (never direct API calls outside
  of `gh api`)
- Error handling for `gh` failures is always explicit (not authenticated, no
  default remote, PR not found)
- Interactive confirmation before destructive or visible actions
- Sub-agents are spawned via `Task` tool for parallel work
- Skills reference each other by name in a "Relationship to Other Commands"
  section

**File paths:**

- `~/.claude/skills/review-pr/SKILL.md` -- most complex example (500 lines)
- `~/.claude/skills/describe-pr/SKILL.md` -- GitHub write operations
- `~/.claude/skills/commit/SKILL.md` -- simple, focused workflow
- `~/.claude/skills/create-plan/SKILL.md` -- interactive, iterative pattern
- `~/.claude/skills/implement-plan/SKILL.md` -- code change execution pattern

### 2. GitHub API Operations for PR Feedback

The `review-pr` skill demonstrates the `gh` CLI patterns used in this codebase:

**Fetching PR metadata:**

```bash
gh pr view {number} --json number,url,title,state,baseRefName,headRefName
gh pr diff {number}
gh pr diff {number} --name-only
gh pr view {number} --json body --jq '.body'
gh pr view {number} --json commits --jq '.commits[].messageHeadline'
```

**GitHub API calls (via `gh api`):**

```bash
# Get repo info
gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'

# Get HEAD SHA
gh api repos/{owner}/{repo}/pulls/{number} --jq '.head.sha'

# Post a review
gh api repos/{owner}/{repo}/pulls/{number}/reviews --method POST --input -
```

**For the new skill, we need these additional API operations:**

```bash
# List review comments (inline comments from reviews)
gh api repos/{owner}/{repo}/pulls/{number}/comments

# List reviews (top-level review bodies)
gh api repos/{owner}/{repo}/pulls/{number}/reviews

# List issue comments (conversation-level comments)
gh api repos/{owner}/{repo}/issues/{number}/comments

# Reply to a review comment thread
gh api repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies \
  --method POST -f body="..."

# Post a top-level PR comment
gh pr comment {number} --body "..."

# Resolve a review thread (mark conversation as resolved)
# Note: GitHub GraphQL API is needed for this - REST API doesn't support it
```

### 3. The Example Skill (obra/superpowers)

The `receiving-code-review` skill from obra/superpowers is a **behavioural
guidance skill**, not a procedural workflow. Key principles it establishes:

1. **READ -> UNDERSTAND -> VERIFY -> EVALUATE -> RESPOND -> IMPLEMENT** pattern
2. **Verify before implementing** -- check suggestions against codebase reality
3. **Push back when wrong** -- technical correctness over social comfort
4. **No performative agreement** -- "Fixed. [description]" not "Great point!"
5. **Clarify unclear items before implementing any** -- items may be related
6. **YAGNI check** -- grep codebase before adding suggested features
7. **One at a time, test each** -- implement in priority order
8. **Reply in comment threads** -- use `gh api .../comments/{id}/replies`

**What we should adopt:**

- The verification-before-implementation philosophy
- The priority ordering (blocking > simple > complex)
- The thread-level reply pattern
- The factual acknowledgement style ("Fixed. [description]")

**What we should NOT adopt:**

- The personality-specific elements ("your human partner", "Circle K" signal)
- The purely behavioural format (we need a concrete procedural workflow)
- The lack of structured GitHub API interaction steps

### 4. Related Plugin: pr-review-toolkit

The `pr-review-toolkit` plugin in the marketplace has relevant agents:

- **comment-analyzer** -- analyses code comments for accuracy (advisory only)
- **code-reviewer** -- general-purpose code review against project guidelines

However, these agents are designed for *giving* reviews, not *responding* to
them. The new skill has a fundamentally different workflow: read feedback,
triage,
implement changes, respond on GitHub.

### 5. Design Considerations for the New Skill

**Workflow Phases (proposed):**

1. **Fetch & Parse** -- Get all outstanding review feedback from the PR
2. **Triage & Prioritise** -- Categorise feedback, identify dependencies
3. **Present to User** -- Show organised feedback, get confirmation on approach
4. **Work Through Items** -- For each item: verify, implement/push back, respond
5. **Wrap Up** -- Summary of what was addressed, push changes

**Key Design Decisions:**

- **Interactive vs Autonomous**: The skill should be interactive, presenting
  each item and confirming the approach before making changes (consistent with
  `create-plan` and `review-pr` patterns which always confirm before acting)
- **Granularity of GitHub responses**: Reply in the specific comment thread
  (consistent with the example skill's guidance), not as top-level comments
- **Handling disagreements**: When the feedback seems wrong, present analysis
  to the user and let them decide whether to implement or push back
- **Commit strategy**: Bundle related changes or commit per feedback item?
  Should be configurable/confirmable per session
- **Re-requesting review**: After addressing all feedback, offer to
  re-request review from the original reviewers

**Types of PR Feedback to Handle:**

| Type                     | Source                | API Endpoint                                  |
|--------------------------|-----------------------|-----------------------------------------------|
| Review comments (inline) | Code review threads   | `pulls/{n}/comments`                          |
| Review body              | Top-level review text | `pulls/{n}/reviews`                           |
| Conversation comments    | PR discussion         | `issues/{n}/comments`                         |
| Requested changes        | Review verdict        | `pulls/{n}/reviews` (event=CHANGES_REQUESTED) |

**Filtering Logic:**

- Only show unresolved review threads
- Only show comments not authored by the current user
- Group by review/reviewer for context
- Flag "changes requested" reviews prominently

## Architecture Insights

### Skill Composition Pattern

The codebase uses a clear composition pattern:

- **Orchestrator skills** (`review-pr`, `review-plan`) coordinate sub-agents
- **Lens skills** (`architecture-lens`, etc.) provide domain expertise
- **Output format skills** (`pr-review-output-format`) define structured schemas
- **Workflow skills** (`commit`, `describe-pr`) execute specific tasks

The new skill is primarily a **workflow skill** (like `implement-plan`) but with
significant GitHub API interaction (like `review-pr`). It doesn't need
sub-agents since the work is inherently sequential (address one item, move to
the next).

### Naming Convention

Following the existing pattern:

- `review-pr` -- reviews a PR (gives feedback)
- `describe-pr` -- describes a PR (writes description)
- Logical name: `respond-to-pr` or `address-pr-feedback`

Given the existing naming convention of `{verb}-{noun}`, `respond-to-pr` fits
best as a concise, action-oriented name. Alternative: `address-pr-feedback`.

### Lifecycle Position

The skill fills a gap in the existing development lifecycle:

1. `/create-plan` -- Plan implementation
2. `/review-plan` -- Review the plan
3. `/implement-plan` -- Execute the plan
4. `/validate-plan` -- Verify implementation
5. `/describe-pr` -- Write PR description
6. `/review-pr` -- Review the PR
7. **`/respond-to-pr`** -- Address review feedback *(NEW)*
8. `/commit` -- Commit changes (used throughout)

## Open Questions

1. **Thread resolution**: GitHub's REST API doesn't support resolving review
   threads (marking conversations as resolved). The GraphQL API does via
   `resolveReviewThread` mutation. Should the skill attempt to resolve threads
   after addressing them, or leave that to the reviewer?

2. **Re-requesting review**: After all feedback is addressed, should the skill
   automatically re-request review from the original reviewers, or just offer
   to do so?

3. **Batching commits**: Should changes be committed per feedback item, or
   bundled? The `commit` skill handles this well -- perhaps the new skill
   should just suggest using `/commit` at natural stopping points.

4. **Scope of code changes**: Should the skill make changes directly (like
   `implement-plan`) or should it focus on analysis and response, leaving
   implementation to the user? The request clearly states it should make
   changes, but the verification-before-implementation principle from the
   example skill suggests confirming each change with the user.

## Code References

- `~/.claude/skills/review-pr/SKILL.md` -- Primary pattern for PR API
  interaction
- `~/.claude/skills/describe-pr/SKILL.md` -- Pattern for GitHub write operations
- `~/.claude/skills/implement-plan/SKILL.md` -- Pattern for making code changes
- `~/.claude/skills/create-plan/SKILL.md` -- Pattern for interactive workflows
- `~/.claude/skills/commit/SKILL.md` -- Pattern for focused workflow skills
- `~/.claude/settings.json` -- Permission allowlist (gh:* already allowed)
- `~/.claude/agents/reviewer.md` -- Agent definition pattern

## External References

- [obra/superpowers receiving-code-review SKILL.md](https://github.com/obra/superpowers/blob/main/skills/receiving-code-review/SKILL.md) --
  Example skill for inspiration
- [GitHub REST API: Pull Request Comments](https://docs.github.com/en/rest/pulls/comments) --
  API for review comments
- [GitHub REST API: Pull Request Reviews](https://docs.github.com/en/rest/pulls/reviews) --
  API for reviews
- [GitHub REST API: Issue Comments](https://docs.github.com/en/rest/issues/comments) --
  API for conversation comments
- [GitHub GraphQL: resolveReviewThread](https://docs.github.com/en/graphql/reference/mutations#resolvereviewthread) --
  Thread resolution
