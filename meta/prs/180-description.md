---
type: "pr-description"
id: "180"
title: "Add an explore-pr skill that builds a review page for a pull request"
date: "2026-11-03T21:26:52+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/180"
pr_number: 180
tags: []
revision: "812a282e570aa9da26d9b099d9e0a6b7b35c2f0d"
repository: "accelerator"
last_updated: "2026-11-05T13:00:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add an explore-pr skill that builds a review page for a pull request

## Summary

Adds `explore-pr`, a project-local skill that turns a large pull request into
an interactive review page: the change's design and decisions as diagrams, a
guided tour by concept with code as the evidence, an attention map of where
the risk sits, a ledger of its riskiest operations, prior `review-pr` findings
re-checked against the code, and views built for the PR alone. The reviewer
records a verdict on each decision, chapter, finding and risky operation, and
those notes can be read back into a session.

Large PRs are hard to review from a diff. The page leads with design rather
than lines, and draws before it writes, so a reviewer can approve the shape
of a change before auditing its detail.

## Changes

The skill lives under `.claude/skills/explore-pr/` beside `raise-pr` and
`clean-comments`. It is not part of the shipped plugin, and nothing outside
that directory changes.

| Area | Lines | What it is |
|---|---|---|
| `SKILL.md` and `reference/` | 1,073 | The process, content-model contract, pass briefs, custom-view guide |
| `assets/page.html` | 1,803 | The page shell: every core view, figure renderers, review store, extension API |
| `scripts/` | 1,303 | Data extraction, the build, the headless-Chrome smoke check |
| `tests/` and `evals/` | 1,232 | 61 pytest tests over the five Python scripts and the page shell; 6 skill evals |
| `assets/vendor/` | 1,306 | marked 12.0.2 and highlight.js 11.9.0, unmodified, with licences |
| `examples/` | 1,937 | A chapter and the prune simulator from PR #170, as the expected standard |

### The process (`SKILL.md`)

1. Reconnoitre the PR: scale by layer, CI, prior reviews, linked work, and
   the architecture it lands in.
2. Agree the goal, custom views and delivery with the user in one question
   set.
3. Export the base and head trees and extract the diff, findings and
   criteria with the bundled scripts.
4. Partition the change into 5–9 chapters by design concept, then run
   parallel reading passes of 2,000–3,000 production lines each, plus one
   design pass that writes `design.json`. Changes under ~800 lines are read
   inline instead.
5. Build custom views, write the overview, build, smoke-check, and deliver as
   a standalone HTML file, a hosted claude.ai artifact, or both.

### The page shell (`assets/page.html`)

- **Views**: Overview, Design, In pictures, Attention map, Guided tour,
  Ledger, Findings, Criteria → tests, All files, Domain terms, Your notes.
- **Reading order**: a chapter shows its figures, then the terms it
  introduces, then what it adds to the design, so every term is defined
  before the prose uses it. Each stop opens with a `fit` ("In the bigger
  picture") placing its code in the change before the `note` explains it;
  read alone, a chapter's `fit` lines tell its story.
- **Figures**: flow, sequence, tree, matrix, timeline, compare and raw SVG,
  each laid out and themed from JSON so drawing costs no more than writing.
- **Review store**: verdicts go to the artifact's `db` collection `reviews`
  when hosted, otherwise to `localStorage` keyed by `repo#number`.
- **Extension API**: `window.ExplorePR.registerView` and `registerKind` let a
  per-PR script add a view that reuses the page's figures, code slices and
  review controls.

### The scripts

- **`export_trees.py`** fetches each changed file at the merge base and head
  through `gh api`, so no local checkout is needed and the skill stays
  VCS-neutral. A renamed file's base body lands under its new name, so its
  diff shows the edit rather than a whole-file addition.
- **`extract_diff.py`** keeps a source file's whole body as context, so the
  page can show any line, and cuts bulk files (lockfiles, minified output,
  files over 4,000 lines) to five lines either side of a change.
- **`parse_review.py`** turns a stored `review-pr` review into findings. It
  reads all three lens-comment layouts the repository's reviews use
  (`(RIGHT), minor/high:`, `(minor, high) —` and `— minor, high —`) and
  treats "See inline comment", "Inline above" and "Merged into the inline"
  as merge markers, so a general finding folded into an inline comment is
  not counted twice.
- **`criteria_map.py`** turns a plan's acceptance-criteria table into
  criteria rows. It finds the table under any heading level titled as a map
  or a traceability section, locates the criterion, tests and phase columns
  by header name in any order, and resolves "as above" to the previous row's
  tests. Before this, no plan in the repository matched its fixed heading.
- **`build.py`** merges the workspace and warns about every reference it
  cannot honour: stops or ledger lines outside their file, stops with no
  `fit`, unknown findings, malformed figures, chapters without a figure or
  design content, and duplicate ids. It writes either one self-contained
  file with everything inlined or the hosted split of `index.html`,
  `content.json` and `diff.json`.
- **`smoke.js`** drives headless Chrome over CDP with no dependencies,
  visiting every registered route at desktop and phone width.

## Context

No work item. The worked examples come from building a page for #170, the
cache-root pruning change; the example chapter now carries a `fit` on each
stop.

This branch is stacked on #179 only because both were raised together; it
does not depend on that change.

## Testing

- [x] `uv run pytest .claude/skills/explore-pr/tests`: 61 passed
- [x] `mise run build-system:check` passes. Repo-wide `ruff` picks up the
  skill's 14 Python files through its nested `ruff.toml`, and `pyrefly`
  reports 0 errors.
- [x] Built a minimal synthetic workspace, including a stop with a `fit`,
  with `build.py` and ran `smoke.js` against it: all 7 core routes rendered,
  `"ok": true`, with no errors and no phone-width overflow
- [x] The vendored libraries' SHA-256 sums match `assets/vendor/README.md`
- [ ] The six evals in `evals/evals.json` were not run for this description
- [ ] The full flow against a real PR (GitHub export, reading passes, hosted
  publish) was not re-run since the parser, criteria-map and `fit` changes
- [ ] `mise run`, the full default task, was not run

## Notes for Reviewers

- **No CI lane runs the tests.** Lint and type checks cover the scripts, but
  no `mise run test:*` task runs the 61 tests. Decide whether to add a lane
  or accept them as a local-only suite.
- **Model-written HTML renders unsanitised.** `md()` passes raw HTML through
  `marked.parse`, and the new `fit` field renders through it too. The `svg`
  figure's regex scrub catches quoted `on*=` handlers but not unquoted ones
  (`<svg onload=alert(1)>`) or `javascript:` URLs. Content is written by
  subagents reading untrusted PR text, and the hosted page can write to the
  owner's `reviews` store. DOMPurify or escape-first rendering would close
  this.
- **A failed file fetch fails silently.** `export_trees.py` skips any file
  whose `gh api` call fails, and `extract_diff.py` then classifies a modified
  file with no base body as `added`. A transient API error therefore renders
  as a whole-file addition with no warning.
- **An unknown `risk` value breaks the attention map.** The build accepts any
  `risk` in `files*.json`, but the page throws a `TypeError` in the Attention
  map for anything outside `high|medium|low|skim`. The smoke check catches
  it, but the build gives no warning.
- **The review parser is regex-shaped to today's reviews.** `parse_review.py`
  now matches three comment layouts; a fourth would drop findings silently
  rather than warn.
- **Skip the vendored and example files.** 3,243 of the 8,659 added lines are
  vendored libraries or worked examples. The review effort belongs on
  `page.html`, `scripts/` and `SKILL.md`.
