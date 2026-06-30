# VCS & PR Workflow

Alongside the development loop, Accelerator provides skills for version control
and team workflows around pull requests. The commit skill is VCS-agnostic
(git or jujutsu); the PR skills wrap the GitHub CLI.

### `/commit`

**What it does** — Create VCS commits for session changes. Detects the active
VCS (git or jujutsu) and groups the session's work into well-structured, atomic
commits.

**How to use it** — `/commit [optional message or flags]`

### `/describe-pr`

**What it does** — Generate a comprehensive pull request description following
the repository's standard template.

**How to use it** — `/describe-pr [PR number or URL]`

**Advice & guidelines** — The output structure is template-driven; eject and
edit `pr-description` via `/configure templates eject pr-description`
to match your project's conventions.

### `/review-pr`

**What it does** — Review a pull request through multiple quality lenses and
present a compiled analysis with inline comments.

**How to use it** — `/review-pr [PR number or URL]`

**Advice & guidelines** — Runs the multi-lens [Review System](review-system.md);
see that page for the lens catalogue and how to enable, disable, or add custom
lenses.

### `/respond-to-pr`

**What it does** — Respond to pull request review feedback interactively,
working through each item with verification and code changes.

**How to use it** — `/respond-to-pr [PR number or URL]`

**Advice & guidelines** — Pairs with `review-pr`: review surfaces the feedback,
respond-to-pr works through each thread and pushes the fixes.
