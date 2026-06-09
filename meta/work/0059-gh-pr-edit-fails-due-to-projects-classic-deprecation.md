---
id: "0059"
title: "gh pr edit Fails Due to GitHub Projects Classic Deprecation"
date: "2026-05-15T11:26:39+00:00"
author: Toby Clemson
kind: bug
status: done
priority: medium
tags: [github, skills, describe-pr, review-pr, respond-to-pr]
type: work-item
schema_version: 1
last_updated: "2026-05-15T11:26:39+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0010"]
---

# 0059: gh pr edit Fails Due to GitHub Projects Classic Deprecation

**Kind**: Bug
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The `describe-pr` skill posts PR descriptions via `gh pr edit`, which now
fails because the GitHub CLI's underlying GraphQL call references the
deprecated Projects (classic) API. The failure surfaces through any skill
that delegates PR body posting to `describe-pr` — most notably `review-pr`
and `respond-to-pr` — which currently fall back to the REST API and emit a
note to the user every time.

## Context

GitHub announced the sunset of Projects (classic) in May 2024
(https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/).
The `gh pr edit` command still queries `repository.pullRequest.projectCards`
under the hood; in current practice the deprecation error fires broadly on
recent `gh` versions (the precise repository-state precondition is not
narrowly characterised), and any such invocation returns a GraphQL error
and exits non-zero.

The `describe-pr` skill (`skills/github/describe-pr/SKILL.md:130`) is the
canonical author of PR descriptions in this plugin. `review-pr` and
`respond-to-pr` both delegate body posting to `describe-pr` rather than
calling `gh pr edit` themselves, so they inherit the failure and the
deprecation-note workaround transitively.

## Requirements

**Reproduction**:

1. Run `/describe-pr` (or `/review-pr` / `/respond-to-pr` which delegate to
   it) against an open PR.
2. The skill writes the body to `{tmp directory}/pr-body-{number}.md` and
   then runs `gh pr edit <number> --body-file <path>`.

**Actual behaviour**:

```
Bash(gh pr edit 119 --body-file .accelerator/tmp/pr-body-119.md)
  ⎿  Error: Exit code 1
     GraphQL: Projects (classic) is being deprecated in favor of the new
     Projects experience, see:
     https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/.
     (repository.pullRequest.projectCards)
```

The skill falls back to `gh api -X PATCH /repos/{owner}/{repo}/pulls/{number}`
and emits a note:

```
Note: gh pr edit failed with a GitHub Projects-classic GraphQL deprecation
error, so I posted via the REST API (gh api -X PATCH .../pulls/119) instead.
```

**Expected behaviour**: The PR body is updated cleanly via the REST API as
the primary path, with no deprecation error and no user-facing fallback note.

**Fix approach**: Replace the `gh pr edit` invocation in
`skills/github/describe-pr/SKILL.md:130` with a direct REST API call.
Resolve the base (upstream) repo via
`gh pr view {number} --json baseRepository` so the PATCH targets the
correct repository for both same-repo and cross-fork PRs, then issue
`gh api --method PATCH repos/{owner}/{repo}/pulls/{number}` with the body
delivered as a JSON-encoded `body` field (e.g. via `jq -Rs '{body: .}'`
stdin). This avoids the deprecated GraphQL path entirely and is correct
for cross-fork PRs.

## Acceptance Criteria

- [ ] Given a PR description update via `describe-pr`, when the skill
  posts the body, then the primary REST path posts successfully on the
  first attempt with no GraphQL error encountered.
- [ ] Given the fix is applied, when `review-pr` or `respond-to-pr`
  delegate to `describe-pr` for a body update, then the body posts via
  the primary REST path on the first attempt and no deprecation-fallback
  note is emitted.
- [ ] Given a PR opened from a fork against an upstream repository, when
  the skill posts the body, then the PATCH request URL targets
  `{upstream-owner}/{upstream-repo}/pulls/{number}` (the base repo, not
  the head fork) and the updated body appears on the upstream PR.
- [ ] Given the fix is applied, when `/describe-pr` runs end-to-end, then
  the YAML frontmatter is stripped from
  `{prs directory}/{number}-description.md` before the body is posted.
- [ ] Given a successful PATCH, when the skill completes, then the body
  file at `{tmp directory}/pr-body-{number}.md` (and any intermediate
  JSON wrapper file, if the wrapper-file variant was chosen) is removed.
- [ ] Given a successful PATCH, when the PR is fetched, then the PR body
  on GitHub matches the stripped content of
  `{prs directory}/{number}-description.md` byte-for-byte.
- [ ] Manual verification: run `/describe-pr` against any open PR and
  confirm that no `gh pr edit` invocation appears in the command trace
  and the body updates without error output.

## Dependencies

- Blocked by: none
- Blocks: none
- External systems: GitHub REST API endpoint
  `PATCH /repos/{owner}/{repo}/pulls/{number}` and the
  `gh pr view --json baseRepository` resolver — the fix replaces a
  GraphQL-backed call with this REST endpoint, so the work item's success
  is contingent on the endpoint's availability and contract stability.
- Related decisions: ADR-0010
  (`meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`)
  — the precedent for REST-API-via-`gh api` posting that this fix
  conforms to.

## Assumptions

- The fix is confined to `describe-pr/SKILL.md`; no shared `gh` wrapper
  exists, so `review-pr` and `respond-to-pr` inherit the fix transitively
  with no edits of their own. If a future refactor extracts a helper, it
  should land there.

## Technical Notes

**Size**: S — single SKILL.md file, but adds owner/repo resolution, body JSON encoding, and error-handling parity beyond the one-line `gh` swap.

- Edit site: `skills/github/describe-pr/SKILL.md:130` — the single
  `gh pr edit` invocation in the live skill set. No helper scripts;
  `review-pr` and `respond-to-pr` only reference `describe-pr` textually
  (`review-pr/SKILL.md:684`, `respond-to-pr/SKILL.md:544`), so the fix
  does not propagate to other files.
- The fix is **not a single-line edit**. The current invocation relies on
  `gh`'s default-remote resolution for owner/repo; the REST URL has
  `{owner}/{repo}` baked in, so the skill must resolve it explicitly
  before the PATCH call.
- **Owner/repo resolution** — use
  `gh pr view {number} --json baseRepository --jq '"\(.baseRepository.owner.login)/\(.baseRepository.name)"'`
  so the PATCH targets the upstream base repo (correct for cross-fork
  PRs). Persistence style is the implementer's call; for reference,
  `review-pr/SKILL.md:117` persists owner/repo to `repo-info.txt` in the
  per-PR tmp subdirectory, and `respond-to-pr/SKILL.md:67` uses an inline
  `gh repo view` without persistence. Both reference patterns use
  `gh repo view`, which is **not** cross-fork-safe — adopt the
  `gh pr view --json baseRepository` form here to satisfy the cross-fork
  criterion.
- **Body encoding** — `gh api -f body=@<path>` does **not** work. `-f`
  and `-F` treat the RHS as a literal string; neither loads files. Use
  one of:
  - `jq -Rs '{body: .}' < {tmp directory}/pr-body-{number}.md | gh api --method PATCH repos/{owner}/{repo}/pulls/{number} --input -`
    — cleanest; mirrors the stdin-JSON pattern at
    `respond-to-pr/SKILL.md:470-472`; no extra file to clean up.
  - Or write a JSON wrapper file containing
    `{"body": <JSON-encoded markdown>}` and pass `--input <wrapper>`
    (mirrors `review-pr/SKILL.md:573-575`; requires a second cleanup
    step alongside the existing `pr-body-{number}.md` removal).
- **Method** — pass `--method PATCH` (or `-X PATCH`) explicitly. `gh api`
  defaults to GET, or POST when a body is present; relying on the
  default is brittle. Codebase convention is `--method <VERB>`
  (`review-pr/SKILL.md:574`, `respond-to-pr/SKILL.md:376`, `:471`).
- **Error-handling parity** — `describe-pr/SKILL.md:54-55` already
  documents the "no default remote repository" error for `gh pr diff`.
  The new `gh repo view` step inherits the same failure mode; reference
  the same remediation, mirroring `review-pr/SKILL.md:127-131`.
- **Cross-fork PR support** — in scope for this fix. `gh pr edit` resolves
  the PR's home repo automatically; the REST URL must point at the
  **base** (upstream) repo, not the head fork. `gh repo view` returns the
  current checkout's repo, which for a contributor working in a fork is
  the fork, not upstream. Use
  `gh pr view {number} --json baseRepository` as the canonical resolver
  — it works correctly for both same-repo and cross-fork PRs.
- **Related pattern** — ADR-0010
  (`meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`)
  documents the precedent for REST-API-via-`gh api` posting in this
  plugin.

## Drafting Notes

- Reported by the user as a bug in `review-pr` and `respond-to-pr`, but the
  `gh pr edit` invocation actually lives in `describe-pr/SKILL.md:130`.
  Those two skills delegate PR body posting to `describe-pr`, so they
  surface the failure transitively. Scoped the fix to `describe-pr`
  accordingly.
- Fix approach pinned to "use REST API directly" rather than a
  try-edit-fallback-rest pattern, per user choice. This eliminates the
  deprecation note rather than masking it.
- Priority set to medium: skills still complete the action via the
  fallback, but the note is emitted on every PR update and the primary
  path may break entirely when GitHub completes the sunset.

## References

- Source: `skills/github/describe-pr/SKILL.md:130`
- Surfaces through: `skills/github/review-pr/SKILL.md`,
  `skills/github/respond-to-pr/SKILL.md`
- External: https://github.blog/changelog/2024-05-23-sunset-notice-projects-classic/
