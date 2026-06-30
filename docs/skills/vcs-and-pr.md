# VCS & PR Workflow

Alongside the development loop, Accelerator provides skills for version control
and team workflows around pull requests. The commit skill is VCS-agnostic
(git or jujutsu); the PR skills wrap the GitHub CLI.

### <img src="https://api.iconify.design/ph/git-commit-bold.svg?color=%2316a34a" width="18" align="center" alt=""> `/commit [optional message or flags]`

Create VCS commits for session changes. Detects the active VCS (git or jujutsu)
and groups the session's work into well-structured, atomic commits.

### <img src="https://api.iconify.design/ph/git-pull-request-bold.svg?color=%2316a34a" width="18" align="center" alt=""> `/describe-pr [PR number or URL]`

Generate a comprehensive pull request description following the repository's
standard template.

*The output structure is template-driven; eject and edit `pr-description` via
`/configure templates eject pr-description` to match your project's conventions.*

### <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%2316a34a" width="18" align="center" alt=""> `/review-pr [PR number or URL]`

Review a pull request through multiple quality lenses and present a compiled
analysis with inline comments.

*Runs the multi-lens [Review System](review-system.md); see that page for the
lens catalogue and how to enable, disable, or add custom lenses.*

### <img src="https://api.iconify.design/ph/chat-text-bold.svg?color=%2316a34a" width="18" align="center" alt=""> `/respond-to-pr [PR number or URL]`

Respond to pull request review feedback interactively, working through each item
with verification and code changes.

*Pairs with `review-pr`: review surfaces the feedback, respond-to-pr works
through each thread and pushes the fixes.*
