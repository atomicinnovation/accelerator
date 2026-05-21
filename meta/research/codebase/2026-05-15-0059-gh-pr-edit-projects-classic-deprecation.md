---
date: 2026-05-15T15:29:17+01:00
author: Toby Clemson
git_commit: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
branch: HEAD
repository: accelerator
topic: "Fix scope and reference patterns for `gh pr edit` → `gh api PATCH` migration in `describe-pr` (work item 0059)"
tags: [research, codebase, describe-pr, review-pr, respond-to-pr, github-rest-api, projects-classic-deprecation]
status: complete
last_updated: 2026-05-15
last_updated_by: Toby Clemson
---

# Research: `gh pr edit` Fails Due to GitHub Projects Classic Deprecation — Fix Scope and Reference Patterns

**Date**: 2026-05-15T15:29:17+01:00
**Author**: Toby Clemson
**Git Commit**: 08a7f5e3cdca3fb84bae5b5ce3a98c909ad2cbb7
**Branch**: HEAD
**Repository**: accelerator

## Research Question

For the bug at `meta/work/0059-gh-pr-edit-fails-due-to-projects-classic-deprecation.md`: what is the exact shape of the edit site in `describe-pr`, what reference patterns already exist in sibling skills (`review-pr`, `respond-to-pr`) for owner/repo resolution and REST body posting, and what conventions does the fix need to introduce versus reuse?

## Summary

The fix is **correctly scoped** to a single edit site — `skills/github/describe-pr/SKILL.md:130` is the only `gh pr edit` invocation in the entire `skills/` tree. No shared helper exists; `review-pr` and `respond-to-pr` reference `describe-pr` only via their "Relationship to Other Commands" workflow narratives, not via runtime shell-out, so the "delegation" the work item describes is documentation-level rather than code-level. The fix therefore propagates transitively because users invoke `/describe-pr` themselves as a documented step in those other workflows, not because the other skills call into it.

Two of the four reference patterns the work item invokes are **already established** in the codebase: the `gh api repos/{owner}/{repo}/pulls/{number}/...` REST endpoint shape, and the `--method <VERB>` convention for explicit HTTP verbs. The other two are **introduced for the first time** by this fix: `gh pr view --json baseRepository` for cross-fork-safe owner/repo resolution (existing skills use `gh repo view --json owner,name`, which is not cross-fork-safe and is explicitly flagged for replacement at this call site), and `jq -Rs '{body: .}' | gh api … --input -` for stdin-piped JSON body encoding (existing skills either use `-f body=…` for short strings or hand-roll JSON with `echo '{…}'` for object payloads).

The frontmatter-stripping acceptance criterion is **already satisfied** by the existing skill (lines 119-129); the fix needs to preserve this step. The cleanup acceptance criterion is **already satisfied unconditionally** at line 131; the fix needs to preserve and potentially extend this if a JSON wrapper file is used. ADR-0010 endorses the *spirit* of the fix (drop to `gh api` when the porcelain CLI is broken) but does not pin down body encoding or owner/repo conventions, so the work item is introducing fresh conventions on top of that precedent.

## Detailed Findings

### Edit Site: `skills/github/describe-pr/SKILL.md`

The full step containing the broken invocation is step 9 ("Update the PR"), lines 117-134. Quoted verbatim:

```
117	9. **Update the PR:**
118	
119	- The `{prs directory}/{number}-description.md` file contains YAML frontmatter
120	  that should not appear on GitHub. Before posting, strip the frontmatter
121	  block from the start of the file:
122	  1. Read the file content
123	  2. The frontmatter block starts with `---` on line 1 and ends at the
124	     next `---` line (which closes the YAML block). Only match the
125	     opening frontmatter block — do not match `---` lines that appear
126	     later in the body (e.g., markdown horizontal rules).
127	  3. Ensure the tmp directory exists: `mkdir -p {tmp directory}`
128	  4. Write everything after the closing `---` line to
129	     `{tmp directory}/pr-body-{number}.md`
130	  5. Post with `gh pr edit {number} --body-file {tmp directory}/pr-body-{number}.md`
131	  6. Clean up `{tmp directory}/pr-body-{number}.md`
132	- Confirm the update was successful
133	- If any verification steps remain unchecked, remind the user to complete
134	  them before merging
```

Implications for the fix:

- **The frontmatter strip is already specified** as a procedural recipe (lines 119-129) and is bound to the body-file write at line 128-129, not to any particular `awk`/`sed`/`jq` command. Acceptance criterion #4 ("the YAML frontmatter is stripped … before the body is posted") is already satisfied by the existing skill — the fix must preserve this step, not introduce it.
- **The body file is `{tmp directory}/pr-body-{number}.md`** where `{tmp directory}` is a placeholder resolved at skill-load time via `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp` (line 18 of the skill). The substitution rule is at lines 20-22.
- **Cleanup at line 131 is unconditional** — the next sequential sub-step after the post call, with no `if successful` qualifier. The fix should mirror this and, if it introduces a JSON wrapper file, extend the cleanup to remove it too.
- **No owner/repo resolution exists** anywhere in the skill; all `gh` invocations rely on `gh`'s implicit default-remote behaviour. The only acknowledgement is the "no default remote repository" remediation at lines 54-55, scoped to `gh pr diff` in step 4, not to the body-posting step. The fix needs to add an owner/repo resolution step and parallel error handling.
- **No `gh api`, `--method PATCH`, `jq -Rs`, or `--input -` usage exists** in the skill today — these patterns must be introduced from scratch.

Skill outline for context (so the fix lands in the right place):

- Lines 1-10: YAML frontmatter
- Lines 12-25: preamble and template injection
- Step 1 (32-35): Use the PR description template
- Step 2 (37-44): Identify the PR
- Step 3 (45-49): Check for existing description
- Step 4 (51-58): Gather PR information — contains the only "no default remote repository" hint (54-55)
- Step 5 (60-67): Analyse the changes
- Step 6 (69-80): Handle verification requirements
- Step 7 (82-90): Generate the description
- Step 8 (92-115): Save and show the description — writes `{prs directory}/{number}-description.md` with frontmatter (97-108)
- **Step 9 (117-134): Update the PR — the edit site (line 130 specifically)**
- Lines 136-147: Important notes and skill-instruction injection

### Reference Pattern A: `review-pr/SKILL.md` (JSON wrapper file + `--input`)

The canonical REST-posting pattern in `review-pr` is:

1. **Owner/repo resolution** at line 116:
   ```
   gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"' > {tmp directory}/pr-review-{number}/repo-info.txt
   ```
   This is **not cross-fork-safe** — `gh repo view` resolves the current checkout's default remote, which on a fork checkout returns the fork, not the upstream base repo where the PR lives. The work item correctly flags this and prescribes `gh pr view --json baseRepository` as the cross-fork-safe replacement.

2. **Per-PR tmp subdirectory convention** (lines 96-100): `{tmp directory}/pr-review-{number}/` keyed by PR number; `mkdir -p` upfront. All intermediate artefacts (diff, payload JSON, head SHA, repo-info, etc.) live in this directory.

3. **Error-handling block** (lines 122-130) — the model to mirror for the new owner/repo resolution step:
   ```
   122	**Error handling**: If any `gh` command fails, handle these cases:
   123	
   124	- **`gh` not installed or not authenticated**: Inform the user that the `gh`
   125	  CLI is required and suggest running `gh auth login` to authenticate.
   126	- **No default remote repository**: Instruct the user to run
   127	  `gh repo set-default` and select the appropriate repository (mirrors the
   128	  pattern in `/describe-pr`).
   129	- **Cannot determine repo owner/name**: If `gh repo view` fails, instruct the
   130	  user to run `gh repo set-default` and select the appropriate repository.
   ```
   (Note line 127-128 references `/describe-pr` as the canonical source — but the actual remediation in `describe-pr` only covers `gh pr diff`, so this cross-reference is mildly aspirational.)

4. **REST POST with file-based JSON wrapper** (lines 571-574):
   ```
   gh api repos/{owner}/{repo}/pulls/{number}/reviews \
     --method POST --input {tmp directory}/pr-review-{number}/review-payload.json
   ```
   Wrapper file is materialised on disk by the prior step and consumed via `--input <file>` (not stdin).

5. **No per-call cleanup** — the JSON wrapper file is left in `{tmp directory}/pr-review-{number}/` and the skill defers cleanup to "session end" (lines 627-634) without specifying a concrete `rm` command.

### Reference Pattern B: `respond-to-pr/SKILL.md` (stdin-piped JSON + `--input -`)

`respond-to-pr` has three distinct posting shapes:

1. **Inline `-f body=…` form** (lines 373-377, 381-384): short text bodies passed as form fields. Not viable for `describe-pr` because PR descriptions are multi-line and contain shell metacharacters, backticks, etc.

2. **Hand-rolled stdin JSON with `--input -`** (lines 468-472):
   ```
   echo '{"reviewers":["reviewer1","reviewer2"]}' | \
     gh api repos/{owner}/{repo}/pulls/{number}/requested_reviewers \
       --method POST --input -
   ```
   This is the closest precedent to the work item's recommended pattern. **Crucially, it uses `echo '{…}'`, not `jq -Rs '{body: .}'`**, because the payload is a fixed object with array values — no arbitrary-content string-escaping problem. For an arbitrary multi-line PR body, the safer construction is `jq -Rs '{body: .}' < <body-file> | gh api … --input -`, which the work item proposes for the first time.

3. **Owner/repo resolution** at line 67 (`gh repo view --json owner,name --jq '"\(.owner.login)/\(.name)"'`) — **inline, no persistence, not cross-fork-safe**. Same caveat as `review-pr`.

4. **No body files or temp files**: `respond-to-pr` never writes to disk for posting purposes. All bodies are constructed inline.

5. **`describe-pr` reference at line 544** is purely a workflow-position note in the "Relationship to Other Commands" section — `respond-to-pr` never calls `describe-pr` at runtime and never touches the PR body itself.

### Reference Pattern C: ADR-0010

`meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md` decided that the review orchestrator would post via `gh api POST /repos/{owner}/{repo}/pulls/{number}/reviews` rather than `gh pr review` because the CLI subcommand cannot post inline comments. Relevant for this fix as **precedent for dropping to `gh api` when a `gh` porcelain command is inadequate**, but the ADR explicitly does *not* pin down:

- Body encoding (no `jq`, `--input`, `-f`, `-F` guidance)
- Owner/repo resolution (uses `{owner}/{repo}` placeholders throughout)
- Cross-fork handling

So the fix conforms to ADR-0010's spirit but introduces new conventions on top of it. A follow-up ADR documenting the chosen body-encoding and owner/repo conventions could be valuable but is not strictly required for the bug fix to land.

### Convention Inventory: Established vs. Introduced

| Pattern | Status in codebase | Action for fix |
|---|---|---|
| `gh api repos/{owner}/{repo}/pulls/{number}/…` endpoint shape | **Established** — `review-pr:115,572`, `respond-to-pr:138,141,375,382,471` | Reuse |
| `--method <VERB>` for explicit HTTP verbs | **Established** — `review-pr:573` (POST), `respond-to-pr:376,383,472` (POST); no PATCH precedent | Introduce `--method PATCH` (no `-X PATCH` precedent in `skills/`) |
| Per-PR tmp subdirectory `{tmp directory}/pr-<verb>-{number}/` | **Established** — `review-pr:96-100` | Optional — could mirror as `pr-describe-{number}/`, but current describe-pr uses `{tmp directory}/` directly without a subdir |
| `gh repo view --json owner,name` for owner/repo | **Established but explicitly flagged for replacement** at this call site | **Do not reuse**; use `gh pr view --json baseRepository` |
| `gh pr view --json baseRepository` (cross-fork-safe resolution) | **Not present** — introduced by this fix | Introduce |
| `gh api --method PATCH` | **Not present** in `skills/` | Introduce |
| `jq -Rs '{body: .}'` for safe arbitrary-string JSON encoding | **Not present** — `respond-to-pr` only hand-rolls fixed-shape JSON via `echo` | Introduce |
| `--input -` (stdin JSON) | **Established** — `respond-to-pr:472` | Reuse |
| `--input <file>` (file-based JSON wrapper) | **Established** — `review-pr:573` | Alternative to stdin pattern |
| "No default remote repository" remediation | **Established** — `review-pr:122-130`, partially in `describe-pr:54-55` | Mirror at the new owner/repo step |
| Frontmatter strip before posting | **Established in `describe-pr:119-129`** | Preserve |
| Unconditional cleanup of body file | **Established in `describe-pr:131`** | Preserve; extend to JSON wrapper if file variant chosen |

### Scope Verification: No Other Edit Sites

Exhaustive grep across `skills/`, `meta/`, and the rest of the repository confirms:

- **`gh pr edit`**: exactly one live invocation in `skills/`, at `describe-pr/SKILL.md:130`. All other occurrences are inside `meta/work/0059-…`, `meta/reviews/work/0057-…`, or older plans/research documenting the historical command — none are executable code.
- **No shared `gh` wrapper script** exists. There are no helper scripts under `scripts/` or `skills/github/` that wrap `gh pr edit`.
- **`review-pr` and `respond-to-pr` do not shell out to `describe-pr`** — the only references are in the "Relationship to Other Commands" sections (`review-pr:683`, `respond-to-pr:544`), which document the user-facing workflow order. The bug surfaces in those other skills' workflows only because their documented workflow includes "user invokes `/describe-pr`" as a step.

This confirms the work item's "Implementation Boundary" claim: the fix is confined to `describe-pr/SKILL.md` and propagates transitively through user-driven workflow narratives, not runtime delegation.

### Framing Nit (Optional)

The work item's Context and Drafting Notes describe `review-pr` and `respond-to-pr` as "delegating PR body posting to `describe-pr`". The mechanism is actually documentation-level (the user follows a multi-step workflow that includes invoking `/describe-pr`), not code-level (no shell-out, no shared helper). This does not change the fix scope or acceptance criteria, but the language could be tightened during planning if precision matters — e.g. "transitively affects workflows that direct the user to invoke `/describe-pr`" rather than "delegate body posting".

### Historical Precedent for Editing This Step

`meta/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills.md` (lines 144, 147, 153) is the most recent prior edit to this exact code path — it migrated the body file from a hardcoded `/tmp/pr-body-{number}.md` to the `{tmp directory}` placeholder. The corresponding review (`meta/reviews/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills-review-1.md`) flags the asymmetry between `describe-pr` and `review-pr` body-handling, which is worth keeping in mind when introducing the JSON wrapper or stdin-pipe pattern: matching `review-pr`'s file-based pattern adds symmetry; choosing the stdin pattern keeps `describe-pr` lighter-weight (no extra cleanup) at the cost of divergence.

## Code References

- `skills/github/describe-pr/SKILL.md:130` — the broken `gh pr edit` invocation (the single edit site)
- `skills/github/describe-pr/SKILL.md:117-134` — full surrounding step 9 ("Update the PR")
- `skills/github/describe-pr/SKILL.md:119-129` — existing frontmatter-strip recipe (preserve)
- `skills/github/describe-pr/SKILL.md:131` — existing unconditional cleanup (preserve/extend)
- `skills/github/describe-pr/SKILL.md:54-55` — existing "no default remote repository" remediation pattern (scoped to `gh pr diff`)
- `skills/github/describe-pr/SKILL.md:18, 20-22` — `{tmp directory}` placeholder resolution and substitution rule
- `skills/github/review-pr/SKILL.md:116` — current `gh repo view --json owner,name` owner/repo resolution (not cross-fork-safe; flagged for non-reuse at this call site)
- `skills/github/review-pr/SKILL.md:122-130` — error-handling block to mirror for the new resolution step
- `skills/github/review-pr/SKILL.md:571-574` — JSON-wrapper-file `--method POST --input <file>` precedent
- `skills/github/respond-to-pr/SKILL.md:67` — inline non-cross-fork-safe owner/repo resolution
- `skills/github/respond-to-pr/SKILL.md:468-472` — stdin-piped `echo '{…}' | gh api … --method POST --input -` precedent (no `jq -Rs` though)
- `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md` — precedent for dropping to `gh api` when porcelain CLI is inadequate

## Architecture Insights

- **PR-touching skills share no runtime helpers.** Each skill embeds its own `gh` invocations directly. The "delegation" between `describe-pr`, `review-pr`, and `respond-to-pr` is purely a documented workflow ordering visible to the user, not a code-level dependency. This means convention drift is a real risk — three skills, three slightly different owner/repo resolution patterns, three different body-posting shapes. The work item is correctly addressing only the broken site; a separate refactor could later extract a helper if drift becomes painful.
- **The codebase's existing owner/repo resolution convention is broken for cross-fork PRs.** Both `review-pr:117` and `respond-to-pr:67` use `gh repo view --json owner,name`, which returns the local checkout's repo, not the PR's base repo. The fix for 0059 will be the first call site to use the cross-fork-safe form `gh pr view --json baseRepository`. If/when this becomes important for inline-comment posting (review-pr) or thread-reply posting (respond-to-pr), those sites will need parallel fixes. They are out of scope for 0059 but worth noting as latent bugs.
- **Body-encoding conventions diverge by payload shape.** Short fixed strings use `-f body="…"` (respond-to-pr); fixed-shape objects use hand-rolled `echo '{…}' | --input -` (respond-to-pr) or file-based `--input <file>` (review-pr). Arbitrary multi-line markdown content has no precedent in the codebase — the work item's `jq -Rs '{body: .}'` proposal is novel but is the correct shell-safe approach.
- **The frontmatter convention for skill-authored description files is consistent.** `describe-pr` writes a YAML frontmatter block (lines 97-108) that must be stripped before posting. The strip logic is already in place; the fix needs to preserve it and not regress acceptance criterion #4.

## Historical Context

- `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md` — established the "drop to `gh api` when porcelain CLI is inadequate" pattern for review posting; precedent for the analogous move in `describe-pr`.
- `meta/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills.md` and its review — most recent prior edit to this exact code path; introduced the `{tmp directory}` placeholder and is the closest editing precedent.
- `meta/plans/2026-02-22-pr-review-inline-comments.md` — original plan that codified `review-pr`'s REST-API posting pattern (the JSON-wrapper-file shape).
- `meta/plans/2026-02-23-respond-to-pr-skill.md` — original plan that codified `respond-to-pr`'s mix of `-f body=` and stdin-piped JSON posting.
- `meta/research/codebase/2026-03-18-meta-management-strategy.md` — documents the `meta/prs/{number}-description.md` → `gh pr edit` flow (the exact call path of the bug).
- `meta/research/codebase/2026-04-07-pr-review-tmp-directory-usage.md` — research backing the 2026-04-07 plan; details how `describe-pr` and `review-pr` differ in body-file mechanics.
- `meta/reviews/work/0057-gh-pr-edit-fails-due-to-projects-classic-deprecation-review-1.md` — review of an earlier version of the work item (note the filename's `0057` reflects an earlier numbering; the work item is now 0059).

## Related Research

- `meta/research/codebase/2026-04-07-pr-review-tmp-directory-usage.md` — directly relevant; covers how the tmp body file is assembled in `describe-pr`.
- `meta/research/codebase/2026-02-22-pr-review-inline-comments.md` — covers the canonical REST-posting pattern for inline comments (the pattern this fix is generalising).
- `meta/research/codebase/2026-02-23-respond-to-pr-feedback-skill.md` — covers the `respond-to-pr` posting patterns including the stdin-JSON form.

## Open Questions

- **Stdin pipe vs. file wrapper for the JSON payload — which to standardise on?** The work item lists both as acceptable, with a preference for the stdin form (`jq -Rs '{body: .}' < {body-file} | gh api … --input -`) on cleanliness grounds. The codebase precedents diverge: `review-pr` is file-based, `respond-to-pr` is stdin-based. Symmetry with `review-pr` would suggest a file wrapper; lighter cleanup footprint and absence of a per-PR tmp subdirectory in `describe-pr` would suggest stdin. This is a planning-level choice, not a research question — flagging here so planning can make the call explicitly.
- **Should the existing line 54-55 "no default remote repository" remediation (scoped to `gh pr diff`) be extended to cover the new `gh pr view --json baseRepository` step, or should a separate error-handling block be added (mirroring `review-pr:122-130`)?** Both are valid; the latter is more discoverable but adds more text to the skill.
- **Does ADR-0010 need an extension or a companion ADR to capture the new conventions (`jq -Rs '{body: .}'`, `gh pr view --json baseRepository`, `--method PATCH`)?** The bug fix can land without one, but a follow-up ADR would prevent drift if the patterns get reused.
- **Out-of-scope follow-ups**: `review-pr:116` and `respond-to-pr:67` both use the non-cross-fork-safe owner/repo resolution. Should follow-up work items be filed to migrate them to `gh pr view --json baseRepository` for consistency? Not blocking 0059, but worth tracking.
