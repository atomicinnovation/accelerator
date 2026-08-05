---
title: 'VCS & PR Workflow'
---

Alongside the development loop, Accelerator provides skills for version
control and team workflows around pull requests. The commit skill is
VCS-agnostic (git or jujutsu); the PR skills wrap the GitHub CLI.

A change typically flows through them in order:

1. [`commit`](../reference/skills/vcs/commit.md) — throughout
   implementation. Detects the active VCS (git or jujutsu) and groups
   the session's work into well-structured, atomic commits.
2. [`describe-pr`](../reference/skills/github/describe-pr.md) —
   generates a comprehensive PR description from the repository's
   standard template (eject and edit it via
   `/configure templates eject pr-description` to match your project's
   conventions).
3. [`review-pr`](../reference/skills/github/review-pr.md) — reviews the
   PR through the multi-lens [Review System](review-system.md) and
   presents a compiled analysis with inline comments.
4. [`respond-to-pr`](../reference/skills/github/respond-to-pr.md) —
   works through review feedback thread by thread, verifying and
   pushing fixes. Pairs with `review-pr`: one surfaces the feedback,
   the other resolves it.
