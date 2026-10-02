---
type: "plan"
id: "2026-10-02-review-pr-local-fix-loop"
title: "Review PR Local Fix Loop Implementation Plan"
date: "2026-10-02T12:49:37+00:00"
author: "John Cowie Del Corral"
producer: "create-plan"
status: "draft"
relates_to: ["plan:2026-06-27-ask-user-question-skill-upgrades", "plan:2026-02-23-respond-to-pr-skill"]
tags: ["skills", "review-pr", "github", "evals"]
revision: "e0029929cb3c01a0180d9074545b05023dac2bbc"
repository: "accelerator"
last_updated: "2026-10-02T12:49:37+00:00"
last_updated_by: "John Cowie Del Corral"
schema_version: 1
---

# Review PR Local Fix Loop Implementation Plan

## Overview

Give `/review-pr` a sixth post-review action: work through every finding one
at a time, fixing each on the PR's branch, **without posting the review or any
comment to GitHub**. This serves authors who run `/review-pr` on their own PR
purely to improve it before asking colleagues for review. The outcome of the
pass is then recorded in the persisted review artifact.

## Current State Analysis

- `skills/github/review-pr/SKILL.md:596-607` ends the review with a plain-text
  five-option menu (post, change verdict, edit comments, discuss, re-run
  lenses). Every route either posts to GitHub or loops back to the menu;
  nothing applies findings.
- `skills/github/review-pr/SKILL.md:743` states "Don't modify any code — this
  is a read-only review", which the new mode contradicts.
- `skills/github/review-pr/SKILL.md:393-402` caps inline comments at
  `{max inline comments}` and moves overflow to an "Additional Findings" list.
  The cap exists to limit PR noise; it is irrelevant when nothing is posted.
- The only genuine per-item fix loop in the repo is
  `skills/github/respond-to-pr/SKILL.md` Step 4 (lines 282-441): verify →
  present analysis + proposed change → `AskUserQuestion` apply/skip → edit →
  commit per the up-front strategy → (GitHub reply, thread resolve) → next.
  Up-front preferences live at lines 257-280; skip/stop interrupts at
  lines 284-287; wrap-up summary and push offer at lines 443-470.
- `skills/work/extract-work-items/SKILL.md:206-296` contributes the
  "Candidate #N of M" counter and the rule that skip/stop interrupts are
  honoured on the turn they arrive.
- The post-review menu is deliberately plain text: `AskUserQuestion` caps at
  four options (`meta/plans/2026-06-27-ask-user-question-skill-upgrades.md:98-101`).
- `/review-pr` has no evals. Colocated `skill-creator` evals
  (`skills/<group>/<skill>/evals/{evals.json,benchmark.json}`) are the only
  behavioural test mechanism for skills today, guarded structurally by
  `tests/unit/tasks/test_evals_structure.py` (every eval id needs a
  `with_skill` run; `run_summary.with_skill.pass_rate.mean >= 0.9`).
- Evals are scripted scenarios: the prompt instructs the model to read the
  SKILL.md, simulate sub-agents and user replies, and save outputs
  (`response.md`, `behavior.md`, …) that expectations grade — see
  `skills/work/extract-work-items/evals/evals.json` id 7.

## Desired End State

After the preview, the `/review-pr` menu offers a sixth option to fix the
findings locally. Choosing it:

1. Confirms the working copy is on the PR's `headRefName` (offering to switch
   if not) and that the working tree is clean enough to commit against.
2. Asks the commit strategy up front: commit after each finding, commit at the
   end, or don't commit.
3. Presents every finding — inline comments (including those deferred by the
   cap) and general findings — in severity then confidence order, as
   "Finding N of M", each verified against the current code before a fix is
   proposed.
4. For each, asks Apply / Skip / Discuss; honours "skip" and "stop" at any
   point.
5. Never calls the GitHub reviews or comments APIs, and never writes
   `review-payload.json`.
6. Ends with a summary and an offer to push unpushed commits.
7. Appends a `## Local Resolution` section to the review artifact recording
   each finding's outcome, and bumps `last_updated`/`last_updated_by`.

Verified by `skills/github/review-pr/evals/` scenarios passing at
`pass_rate.mean >= 0.9` and by `mise run` exiting 0.

### Key Discoveries:

- Finding schema (path, line, end_line, side, severity, confidence, lens,
  title, body; general findings lack path/line):
  `skills/review/output-formats/pr-review-output-format/SKILL.md:19-42`.
- `side: "LEFT"` findings reference the old file version — the code may no
  longer exist on the branch, so verification must handle "already gone".
- Artifact body shape: `templates/pr-review.md:60-87`.
- respond-to-pr wrap-up/push pattern: `skills/github/respond-to-pr/SKILL.md:443-470`.
- respond-to-pr guardrails worth inheriting: stage files by name, no
  attribution in commits, keep changes to what the finding asked
  (`skills/github/respond-to-pr/SKILL.md:523-543`).

## What We're NOT Doing

- Not converting the post-review menu to `AskUserQuestion` (six options
  exceed the four-option cap; prior plan kept it plain text deliberately).
- Not restricting the option to PR authors — it is offered on any PR; the
  branch check is the only gate.
- Not posting a "self-review" summary, PR comment, or review to GitHub in any
  form, nor a hybrid "post then fix" mode.
- Not adding a re-review pass after fixes (users can re-run `/review-pr`,
  producing `-review-{N+1}.md`).
- Not extracting a shared loop module between `review-pr` and
  `respond-to-pr`; skills stay self-contained prose.
- Not changing `templates/pr-review.md`; the Local Resolution section is
  appended only when the loop runs.
- Not porting to the Inspect eval harness (ADR-0055); we follow the current
  colocated `skill-creator` convention.

## Implementation Approach

Skill prose is the product, so evals are the tests. Each phase writes its
eval scenarios first and runs them against the unchanged skill to observe
failure (red), then edits `SKILL.md` until they pass (green), then tidies the
prose (refactor) and records `benchmark.json`. Because the structural guard
rejects an `evals.json` without a passing benchmark, evals and skill change
land in the same phase, keeping every phase independently mergeable.

Evals are authored and run interactively with the `skill-creator` skill,
matching how existing benchmarks were produced.

---

## Phase 1: Local Fix Loop

### Overview

Add the sixth menu option and the full per-finding fix loop, with its evals,
docs, and changelog entry.

### Changes Required:

#### 1. Eval fixtures (written first)

**Directory**: `skills/github/review-pr/evals/files/local-fix/`

- `review-findings.json` — aggregated, already-curated review state as Step 4
  would hold it for a fictional PR #4242 titled "Add order total
  calculation", head branch `feature/order-totals`:
  - one 🔴 critical inline comment (correctness): `src/order_total.py`
    sums `price` without multiplying by `quantity`.
  - one 🟡 major inline comment (code-quality) on `src/order_total.py`
    with a magic discount constant.
  - one 🔵 minor inline comment deferred by the cap (standards): naming in
    `src/order_total.py`.
  - one 🟡 major general finding (test-coverage): no test for empty orders.
  - one 🔵 suggestion with `side: "LEFT"` referencing code removed by the PR.
- `src/order_total.py` — a small module exhibiting the flagged defects.
- `pr-metadata.json` — `number`, `title`, `headRefName`, `baseRefName`,
  `state`.

#### 2. Eval scenarios (written first, observed failing)

**File**: `skills/github/review-pr/evals/evals.json`

Each prompt opens with "Read skills/github/review-pr/SKILL.md fully. You are
running the review-pr skill." and states: the working directory is the plugin
root; Steps 1–4 are already complete with the state in
`skills/github/review-pr/evals/files/local-fix/`; simulate every `gh` and
VCS command by stating what you would run instead of running it; do not spawn
agents; do not write outside outputs/.

| id | name | Scenario | Key expectations |
|----|------|----------|------------------|
| 1 | `menu-offers-local-fix` | Present Step 5 preview and Step 6 menu; stop. Save `menu.md`. | Menu lists six options; one offers fixing findings on the branch; that option states nothing will be posted to GitHub; existing five options still present. |
| 2 | `local-fix-setup` | User chooses the local-fix option. Simulated current branch is `main`. Stop at the first question after setup. Save `response.md`, `behavior.md`. | Detects branch differs from `feature/order-totals` and offers to switch; asks commit strategy with per-finding / at-end / no-commit choices before presenting any finding; states total of 5 findings (includes deferred and general). |
| 3 | `local-fix-ordering-and-presentation` | User is on the branch, picks "commit after each finding". Present the first two findings, user applies the first. Save `findings.md`, `behavior.md`. | First presented is the critical correctness finding as "Finding 1 of 5"; each shows lens, severity, location, analysis from reading the actual file, proposed change; asks Apply / Skip / Discuss; after apply, edit is limited to the finding and a commit naming only `src/order_total.py` is proposed. |
| 4 | `local-fix-skip-stop-summary` | User skips finding 2 with reason "intentional", then says "stop here". Save `summary.md`, `behavior.md`. | Skip recorded with reason and loop advances; stop ends the loop immediately; summary reports applied/skipped/unaddressed counts and commits created; offers to push via a two-option choice. |
| 5 | `local-fix-never-posts` | Run the whole loop applying everything (simulated). Save `commands.md` listing every command you would run. | No `gh api …/reviews`, `…/comments`, `gh pr comment`, or `gh pr review`; no `review-payload.json` written. |
| 6 | `local-fix-left-side-finding` | Reach the LEFT-side suggestion. Save `finding.md`. | Recognises the referenced code no longer exists on the branch and offers to skip it as already resolved rather than inventing an edit. |

Run all six against the current `SKILL.md` with `skill-creator` and confirm
they fail (the menu has five options and there is no loop).

#### 3. Skill change

**File**: `skills/github/review-pr/SKILL.md`

- **Step 6 menu** (`:600-607`): add
  `6. Work through the findings and fix them on this branch? (nothing is posted to GitHub)`.
- **New "When the user chooses to fix findings locally (option 6)"
  subsection** under Step 6, after the verdict-change handling, containing:
  1. **Branch check** — compare the current branch/bookmark (per session VCS
     context) with `headRefName`; if different, `AskUserQuestion`:
     **Yes, switch to the PR branch** / **No, stay on the current branch**.
     If the working copy has uncommitted changes, say so before continuing.
  2. **Commit strategy** — `AskUserQuestion` with three options:
     **Commit after each finding** / **Commit at the end** /
     **Don't commit**.
  3. **Finding list** — every inline comment (including those deferred by the
     inline cap) plus every general finding, ordered by severity then
     confidence, announced with the total.
  4. **Per-finding cycle**, headed `### Finding {N} of {M}: {title}` with
     lens, severity, confidence and `path:line` (or "general"):
     - **Verify** by reading the current code, not only the diff; for
       `LEFT`-side findings or code that no longer matches, say so and offer
       to skip as already resolved.
     - **Present** analysis and proposed change.
     - `AskUserQuestion`: **Apply this fix** / **Skip this finding** /
       **Discuss first**. Discussion does not advance N; on skip, ask for an
       optional reason in plain text.
     - **Apply** with Edit/Write, limited to what the finding asked.
     - **Commit** per the chosen strategy, following the `commit` skill
       pattern, staging files by name.
     - Announce the move to the next finding.
  5. **Interrupts** — "skip" and "stop here" are honoured on the turn they
     arrive.
  6. **Wrap-up** — summary of applied / skipped / unaddressed counts and
     commits created, on branch `{headRefName}`; if there are unpushed
     commits, `AskUserQuestion`: **Yes, push now** / **No, skip**. State
     that nothing was posted to the PR.
- **Important Guidelines**: add "**Fixing locally never touches GitHub** —
  in the local fix loop, never call the reviews or comments APIs; the review
  stays private to the author."
- **What NOT to Do** (`:743`): replace "Don't modify any code — this is a
  read-only review" with "Don't modify any code unless the user chooses to
  fix findings locally, and then only one confirmed finding at a time".
- **What NOT to Do**: add "Don't add co-author information or Claude
  attribution to commits" and "When staging files, always add specific files
  by name — never bulk-add", mirroring respond-to-pr.
- **Step 5 heading text** (`:568`): change "showing exactly what will be
  posted to the PR" to "showing exactly what would be posted to the PR" so
  it reads correctly when nothing is posted.

Re-run the evals; iterate on prose until all pass. Then record
`skills/github/review-pr/evals/benchmark.json` with `with_skill` runs for ids
1–6 and `run_summary.with_skill.pass_rate.mean >= 0.9`.

#### 4. Docs

**File**: `docs-site/src/content/docs/guides/review-a-pr.mdx`

- Step 4: add "or fix the findings yourself" to the listed actions.
- Step 5 retitled "**Post, fix locally, or keep it local.**" with a sentence:
  choosing to fix locally walks you through each finding — verify, fix,
  commit — on the PR's branch without posting anything, useful for
  polishing your own PR before requesting review.

#### 5. Changelog

**File**: `CHANGELOG.md`, under `## [Unreleased]` → `### Added`:

```markdown
- **`/review-pr` can fix its findings locally instead of posting them.** A
  sixth post-review action works through every finding one at a time on the
  PR's branch — verify, fix, commit per your chosen strategy — without posting
  the review or any comment to GitHub. Useful for polishing your own PR before
  asking for review.
```

### Success Criteria:

#### Automated Verification:

- [x] Eval structure guard passes: `mise run test:unit` (covers
      `tests/unit/tasks/test_evals_structure.py`)
- [x] Skill permission/CLI-ref/bare-invocation lints pass: `mise run check`
- [x] Hyphenation guard over CHANGELOG passes (part of `mise run test:unit`)
- [x] Docs build and anchors pass: `mise run docs:check`
- [ ] Full local CI mirror is green: `mise run`

#### Manual Verification:

- [x] All six evals ran red against the pre-change skill (recorded in the PR
      description)
- [x] `benchmark.json` `with_skill` mean ≥ 0.9 for ids 1–6
- [ ] Running `/accelerator:review-pr` on a real own PR, choosing option 6,
      fixes at least one finding, creates a commit, and leaves the PR with no
      new review or comment on GitHub
- [ ] Options 1–5 behave exactly as before

---

## Phase 2: Record Local Resolution in the Review Artifact

### Overview

Persist the outcome of a local fix pass into
`{pr reviews directory}/{number}-review-{N}.md`, so the history shows which
findings were acted on.

### Changes Required:

#### 1. Eval fixture (written first)

**File**: `skills/github/review-pr/evals/files/local-fix/4242-review-1.md` — a
complete review artifact for the Phase 1 findings, with valid frontmatter
per `templates/pr-review.md`.

#### 2. Eval scenarios (written first, observed failing)

**File**: `skills/github/review-pr/evals/evals.json` — add:

| id | name | Scenario | Key expectations |
|----|------|----------|------------------|
| 7 | `local-resolution-appended` | Copy the fixture artifact to outputs/, treat that copy as the review artifact. Apply finding 1 (simulated commit `abc1234`), skip finding 2 with reason "intentional", stop. Save the updated artifact. | Artifact ends with `## Local Resolution — {date}`; lists finding 1 as applied with `abc1234`, finding 2 as skipped with "intentional", findings 3–5 as unaddressed; original sections unchanged. |
| 8 | `local-resolution-frontmatter` | Same as 7. Save `frontmatter-diff.md`. | Only `last_updated` and `last_updated_by` change; `verdict`, `status`, `review_number`, `target` unchanged. |
| 9 | `local-resolution-no-commit-strategy` | Strategy "Don't commit"; apply finding 1, stop. | Applied finding recorded as "applied (uncommitted)". |

Run 7–9 against the Phase 1 skill and confirm they fail.

#### 3. Skill change

**File**: `skills/github/review-pr/SKILL.md`

- Add a step 7 to the local-fix subsection's wrap-up: **Record the outcome**
  — append to the review artifact written in Step 4.10:

  ```markdown
  ## Local Resolution — {date}

  | # | Finding | Location | Lens | Severity | Outcome |
  |---|---------|----------|------|----------|---------|
  | 1 | {title} | `{path}:{line}` or general | {lens} | {severity} | applied ({short commit id}) / applied (uncommitted) / skipped: {reason} / unaddressed |
  ```

  Then update only `last_updated` (from `accelerator corpus metadata derive`)
  and `last_updated_by`; leave every other frontmatter field unchanged. If
  the user stopped early, the remaining findings are recorded as
  `unaddressed`.
- Extend guideline 7's description of the persisted artifact to mention the
  Local Resolution section.

Re-run evals 1–9; iterate until green; update `benchmark.json` with runs for
ids 7–9 keeping `pass_rate.mean >= 0.9`.

#### 4. Docs and changelog

- `docs-site/src/content/docs/guides/review-a-pr.mdx` step 5: add that the
  outcome of a local fix pass is appended to the review file.
- `CHANGELOG.md`: extend the Phase 1 entry with "The outcome of each finding
  is appended to the review file under **Local Resolution**."

### Success Criteria:

#### Automated Verification:

- [x] Eval structure guard passes with ids 1–9: `mise run test:unit`
- [x] Frontmatter conformance passes for review-pr: `mise run test:integration`
- [x] Read-only CI set passes: `mise run check`
- [ ] Full local CI mirror is green: `mise run`

#### Manual Verification:

- [x] Evals 7–9 ran red against the Phase 1 skill
- [ ] After a real local fix pass, the review file shows a correct Local
      Resolution table and `accelerator corpus frontmatter validate --file
      <review>` reports no errors
- [ ] The visualiser still renders the updated review file

---

## Testing Strategy

### Evals (behavioural tests):

- Menu presence and wording; setup (branch check, commit strategy before any
  finding); ordering including deferred and general findings; apply / skip /
  discuss / stop; push offer; the absence of any GitHub write; LEFT-side
  finding handling; artifact append and frontmatter discipline.

### Structural tests (existing, must stay green):

- `tests/unit/tasks/test_evals_structure.py`
- `tests/unit/tasks/test_skill_permissions.py`,
  `tests/unit/tasks/test_skill_cli_refs.py`,
  `tests/unit/tasks/test_bare_invocation.py`,
  `tests/unit/tasks/test_git_tokens.py`
- `tests/integration/conformance/test_conformance.py`

### Manual Testing Steps:

1. Open a throwaway PR on your fork with a deliberate bug; run
   `/accelerator:review-pr <n>`.
2. Choose option 6 from `main` — confirm the switch prompt.
3. Choose "Commit after each finding"; apply one, skip one with a reason,
   then say "stop here".
4. Confirm one commit exists, the push offer appears, the PR on GitHub has no
   new review/comments, and the review file has the Local Resolution table.

## Performance Considerations

None — the loop reuses findings already in context; no extra agent runs.

## Migration Notes

None. Existing review artifacts are untouched; the section is only appended
when the loop runs.

## References

- Skill under change: `skills/github/review-pr/SKILL.md`
- Loop modelled on: `skills/github/respond-to-pr/SKILL.md:257-470`
- Counter/interrupt pattern: `skills/work/extract-work-items/SKILL.md:206-296`
- Menu kept plain text: `meta/plans/2026-06-27-ask-user-question-skill-upgrades.md:98-101`
- Eval convention: `skills/work/extract-work-items/evals/evals.json`
- Posting decision: `meta/decisions/ADR-0010-atomic-review-posting-via-github-rest-api.md`
