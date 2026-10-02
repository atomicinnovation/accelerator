---
type: "pr-description"
id: "139"
title: "Let /review-pr fix its findings locally"
date: "2026-10-02T14:03:40+00:00"
author: "John Cowie Del Corral"
producer: "describe-pr"
status: "complete"
relates_to: ["plan:2026-10-02-review-pr-local-fix-loop"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/139"
pr_number: 139
tags: ["skills", "review-pr", "github", "evals"]
revision: "104ac383094d8ac17295d9724e37c500f87aaf38"
repository: "accelerator"
last_updated: "2026-10-02T14:03:40+00:00"
last_updated_by: "John Cowie Del Corral"
schema_version: 1
---

# Let /review-pr fix its findings locally

## Summary

`/review-pr` gains a sixth post-review action that works through every finding
one at a time on the PR's branch — verify, fix, commit — **without posting the
review or any comment to GitHub**. It serves authors who run `/review-pr` on
their own PR to polish it before asking colleagues for review. When the pass
ends, its outcome is appended to the persisted review file under a
**Local Resolution** section, so review history records what was acted on.

## Changes

- **Sixth menu option.** Step 6 now offers "Work through the findings and fix
  them on this branch? (nothing is posted to GitHub)". The menu stays plain
  text: six options exceed `AskUserQuestion`'s four-option cap.
- **Local fix loop.** Choosing it:
  - checks the working copy is on the PR's `headRefName`, offering to switch if
    not, and warns about uncommitted changes;
  - asks the commit strategy up front (after each finding / at the end / don't
    commit);
  - lists *every* finding — inline comments, those deferred by the inline cap,
    and general findings — by severity then confidence, as "Finding N of M";
  - verifies each against the current code before proposing a change; a
    `side: "LEFT"` finding whose code the PR already removed is offered as
    "Skip as already resolved" rather than invented into an edit;
  - asks Apply / Skip / Discuss, honours "skip" and "stop here" on the turn
    they arrive, and ends with a summary and a two-option push offer.
- **Local Resolution record.** The wrap-up appends
  `## Local Resolution — {date}` to `{pr reviews directory}/{number}-review-{N}.md`
  with one row per finding: `applied (<commit>)`, `applied (uncommitted)`,
  `skipped: <reason>`, or `unaddressed`. Only `last_updated` and
  `last_updated_by` change in the frontmatter — `verdict` and `status` keep
  describing the review itself.
- **Guardrails.** A new guideline says the local fix loop never calls the
  reviews or comments APIs. "Don't modify any code" now carves out the fix
  loop (one confirmed finding at a time), and the skill inherits
  respond-to-pr's rules: stage files by name, no attribution in commits.
- **Evals.** `/review-pr` gets its first colocated `skill-creator` evals: nine
  scenarios over a fictional PR #4242 (fixtures under
  `skills/github/review-pr/evals/files/local-fix/`), with `benchmark.json`.
- **Docs and changelog.** The review guide's step 5 becomes "Post, fix locally,
  or keep it local"; `CHANGELOG.md` gains an Added entry.
- **Unrelated:** `docs-site/package-lock.json` lifts `devalue` and `dompurify`
  past their advisories.

## Context

- Plan: `meta/plans/2026-10-02-review-pr-local-fix-loop.md` (both phases).
- Loop modelled on `skills/github/respond-to-pr/SKILL.md` Step 4; counter and
  interrupt pattern from `skills/work/extract-work-items/SKILL.md`.
- No work item.

## Testing

- [x] Evals 7–9 (Local Resolution) ran red against the Phase 1 skill — the
  review file came back unchanged in all three. Evals 1–6 ran red against the
  pre-change skill in Phase 1.
- [x] All nine evals pass against the final skill: `benchmark.json`
  `with_skill` mean 1.0.
- [x] `mise run check` exits 0.
- [x] `mise run docs:check` exits 0.
- [x] `mise run test:unit:tasks` — 2860 passed (includes the eval structure
  guard and the CHANGELOG hyphenation guard).
- [x] `mise run test:integration:conformance` — 28 passed;
  `test:integration:skill-invocation` — 139 passed.
- [ ] Full `mise run` is not green locally, for two reasons that are not part of
  this change: 8 `accelerator-migrate` `dirty_tree_preflight` tests need git ≥
  2.45 (the local Homebrew git is 2.41), and
  `test_warm_path_on_the_shasum_fallback_batches_and_trusts` fails identically
  on `main`'s tip.
- [ ] Manual: run `/accelerator:review-pr` on your own PR, choose option 6, fix
  at least one finding and commit it; confirm the PR gains no new review or
  comment, and the review file's Local Resolution table is correct and passes
  `accelerator corpus frontmatter validate --file <review>`.
- [ ] Manual: options 1–5 behave exactly as before, and the visualiser still
  renders an updated review file.

## Notes for Reviewers

- **Evals are scripted simulations.** Each eval prompt has the model read
  `SKILL.md`, simulate `gh`/VCS commands and user replies, and save outputs that
  are graded against the expectations. They pin the skill's prose behaviour,
  not real tool execution — hence the manual checks above.
- **Option 6 is offered on any PR**, not only the user's own; the branch check
  is the only gate.
- **No re-review after fixes** — re-running `/review-pr` produces a fresh
  `-review-{N+1}.md`. The review template is unchanged; the Local Resolution
  section is appended only when the loop runs.
- **No shared loop module** with `respond-to-pr`; skills stay self-contained
  prose.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
