---
title: Review a Pull Request
description: How to run a multi-lens PR review, post it to GitHub, and
  work through the resulting feedback.
---

This guide runs a pull request through Accelerator's multi-lens review
and, optionally, works through the feedback afterwards. It assumes the
plugin is installed.

## Prerequisites

- The [`gh` CLI](https://cli.github.com/) installed and authenticated
  (`gh auth login`).
- A default remote set for the repo (`gh repo set-default`) so PR
  numbers resolve.

## Steps

1. **Start the review.** Run
   [`review-pr`](../reference/skills/github/review-pr.md) with a PR
   number or URL — or bare, and it offers the current branch's PR:

   ```
   /accelerator:review-pr 123
   ```

   You can add focus areas, e.g.
   `/accelerator:review-pr 123 focus on security`.

2. **Confirm the lens selection.** The skill proposes a set of review
   lenses — the core four are architecture, code quality, test
   coverage, and correctness, drawn from a catalogue of thirteen — and
   asks you to proceed or adjust. Lens selection is
   [configurable](configuration-cookbook.md#tune-review-behaviour),
   and you can add your own lenses under `.accelerator/lenses/`.

3. **Wait for the parallel review.** Each lens runs in its own
   subagent against the PR diff. Findings are anchored to diff lines,
   deduplicated, capped, and aggregated into a verdict: APPROVE,
   REQUEST_CHANGES, or COMMENT.

4. **Inspect the preview.** You are shown the summary body and every
   inline comment before anything is posted. From here you can post the
   review, change the verdict, edit or remove comments, discuss
   findings, or re-run lenses.

5. **Post (or keep it local).** Posting submits a real GitHub review
   via `gh`. Either way, the full review — summary, inline comments,
   and per-lens results — is persisted to
   `meta/reviews/prs/<number>-review-<N>.md`, so review history
   survives the conversation.

6. **Work through the feedback.** On the receiving end of a review
   (yours or a colleague's), run
   [`respond-to-pr`](../reference/skills/github/respond-to-pr.md):

   ```
   /accelerator:respond-to-pr 123
   ```

   It fetches unresolved review threads, triages them, and works
   through each item with you: verify the claim against the code,
   change the code, commit, reply in-thread, and resolve. Re-running it
   later resumes where you left off.

## See also

- [Review System](../skills/review-system.md) — the full lens catalogue
  and how verdicts are computed.
- [`describe-pr`](../reference/skills/github/describe-pr.md) — generate
  the PR description before requesting review.
- [Which skill do I need?](which-skill.md)
