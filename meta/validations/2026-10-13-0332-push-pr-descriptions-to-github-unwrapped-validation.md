---
type: "plan-validation"
id: "2026-10-13-0332-push-pr-descriptions-to-github-unwrapped-validation"
title: "Validation Report: Push PR Descriptions to GitHub Unwrapped Implementation Plan"
date: "2026-10-14T15:22:02+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-13-0332-push-pr-descriptions-to-github-unwrapped"
tags: ["collaboration", "github", "markdown", "pulldown-cmark", "describe-pr"]
last_updated: "2026-10-14T15:22:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Push PR Descriptions to GitHub Unwrapped Implementation Plan

Validated at `qsqtvqrp` (`92911d08`, "Trim comments to their reasons"), the
cumulative change since the trunk fork point.

### Implementation Status

✓ Phase 1: The `markdown` technical library - Fully implemented
✓ Phase 2: One-off corpus verification - Fully implemented (record complete)
✓ Phase 3: Unwrapping whitespace breaks GitHub renders as soft - Fully
implemented, including the unplanned `rustls` 0.23.46 bump
✓ Phase 4: Publishing bodies unwrapped - Fully implemented

### Automated Verification Results

✓ Full local CI passes: `mise run` exited 0, and its formatters left the
working copy unchanged
✓ Included in that run: `cli:check`, `pup:check`, `public-api:check`,
`deny:check`, `notices:check`, `lint:crate-dependencies:check`,
`docs:check`, the crate tests for `markdown`, `collaboration`, `github` and
`accelerator-collaboration`, `test:unit:tasks` (3549 passed),
`test:integration:pup`, `test:integration:deny` (116 passed)

### Code Review Findings

#### Matches Plan:

- `cli/markdown/src/lib.rs` implements `unwrap_soft_line_breaks` with
  `soft_line_break_spans`, `renders_as_soft_on_github`,
  `trailing_whitespace_start` and `continuation_start` exactly as Phases 1
  and 3 specify; the doc comment carries the two-spaces constraint.
- `cli/markdown/tests/fixtures/public-api.txt` exposes only
  `unwrap_soft_line_breaks`, with no `pulldown_cmark` type.
- `pulldown-cmark = "=0.13.4"` with default features off, the `deny.toml`
  wrapper ban, the `markdown_transforms_text_only` pup rule, the probe pair,
  the crate-role declaration and the `_PINNED_CRATES` entry are all present.
- The command-runner alternation in `cli/pup.ron` now includes `markdown`
  and `graphql_test_support`, and `tasks/README.md` says "seven things" with
  the new checklist bullet.
- `cli/markdown/src/tests.rs` holds both characterisation tests, every
  Phase 1 and Phase 3 fixture, and the six probe-derived fixtures the corpus
  record names.
- `PullRequestBodyPublishing` sits beside `PullRequestBodyUpdate`;
  `update_pull_request_body` takes it after `body_update` and PATCHes the
  prepared body. The `update` / `update_published_by` split and the
  `RecordingBodyUpdate`, `UnchangedPublishing` and `MarkedPublishing` fakes
  match the plan.
- `GitHubPullRequestBodyPublishing` delegates to `markdown`, is re-exported
  from `github`, and is wired at `cli/collaboration-cli/src/main.rs:241`.
- The end-to-end tests cover the PATCHed body, the unchanged body file and
  independence from 40-column conventions; `cli_surface.rs` pins the
  `--help` surface with a golden.
- `describe-pr` SKILL.md, `docs-site/src/content/docs/collaboration.md` and
  `CHANGELOG.md` carry the planned wording, including the exit-code fix.
- The posted PR #164 body is 85 lines (longest 411 columns) against the
  175-line, wrapped `meta/prs/164-description.md`. It contains the corpus
  verification record and the `rustls` 0.23.46 callout.

#### Deviations from Plan:

- The final "Trim comments to their reasons" commit shortened two planned
  comments. The `cli/pup.ron` rule comment keeps only the `denied`-versus-
  `allowed_only` reason. The `_SHARED_CRATE_WORKSPACE` comment in
  `tests/integration/pup/test_import_rule.py` keeps only the
  compliant-control sentence. Both changes follow the repository's comment
  policy.
- `update_body_leaves_the_body_file_unchanged` compares the file with
  `read_to_string` rather than as bytes. For the ASCII fixture this is
  equivalent.
- Phase 3's `rustls` bump was unplanned. The plan records it, and the PR
  description calls it out.

#### Potential Issues:

- `docs-site/src/content/docs/collaboration.md` and the changelog say hard
  breaks are posted byte-for-byte. This holds only for hard breaks as
  GitHub defines them (two spaces or `\`): a `\t\t` break, which is hard to
  `pulldown-cmark`, is now joined. The wording matches GitHub's semantics,
  but readers who know `pulldown-cmark` could misread it.
- Work item 0332 still has `status: "draft"`. Transition it when the PR
  merges.

### Manual Testing Required:

1. On GitHub:
  - [ ] Confirm that PR #164's paragraphs render at full width and that any
        trailer lines that must stand alone still do

### Recommendations:

- Transition work item 0332 when PR #164 merges.
- When 0320 needs a Linear dialect, add a dialect-aware entry point to
  `markdown` rather than reusing `unwrap_soft_line_breaks`, as the plan's
  "What We're NOT Doing" section anticipates.
