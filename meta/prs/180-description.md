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
revision: "6cafaac15986dc41feaf9259b44d6ede078da7fd"
repository: "accelerator"
last_updated: "2026-11-09T00:43:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add an explore-pr skill that builds a review page for a pull request

## Summary

Adds `explore-pr`, a project-local skill that turns a large pull request into
an interactive review page: the change's design and decisions as diagrams, a
guided tour by concept with code as the evidence, an attention map of where
the risk sits, a ledger of its riskiest operations, prior `review-pr` findings
re-checked against the code, a commit-by-commit view, and views built for the
PR alone. The reviewer records a verdict on each decision, chapter, finding
and risky operation, and those notes can be read back into a session.

Large PRs are hard to review from a diff. The page leads with design rather
than lines, and draws before it writes, so a reviewer can approve the shape
of a change before auditing its detail. It also keeps one record of which
changed lines have been read, so every view shows what is still to review,
whichever view the reading was done in.

## Changes

The skill lives under `.claude/skills/explore-pr/` beside `raise-pr` and
`clean-comments`. It is not part of the shipped plugin, and nothing outside
that directory changes.

| Area | Lines | What it is |
|---|---|---|
| `SKILL.md` and `reference/` | 1,110 | The process, content-model contract, pass briefs, custom-view guide |
| `assets/page.html` | 2,156 | The page shell: every core view, figure renderers, review store, extension API |
| `assets/review-coverage.js` | 192 | The model of which changed lines have been read, shared by every view |
| `scripts/` | 1,849 | Data extraction for the diff and each commit, the build, the headless-Chrome smoke check |
| `tests/` and `evals/` | 1,985 | 87 pytest tests over the seven Python scripts and the page shell, one of which runs the coverage model's 29 node tests; 6 skill evals |
| `assets/vendor/` | 1,306 | marked 12.0.2 and highlight.js 11.9.0, unmodified, with licences |
| `examples/` | 1,937 | A chapter and the prune simulator from PR #170, as the expected standard |

### The process (`SKILL.md`)

1. Reconnoitre the PR: scale by layer, CI, prior reviews, linked work, and
   the architecture it lands in.
2. Agree the goal, custom views and delivery with the user in one question
   set.
3. Export the base and head trees and each commit's files, and extract the
   diff, the commits traced onto it, the findings and the criteria with the
   bundled scripts.
4. Partition the change into 5–9 chapters by design concept, then run
   parallel reading passes of 2,000–3,000 production lines each, plus one
   design pass that writes `design.json`. Changes under ~800 lines are read
   inline instead.
5. Build custom views, write the overview, build, smoke-check, and deliver as
   a standalone HTML file, a hosted claude.ai artifact, or both.

### The page shell (`assets/page.html`)

- **Views**: Overview, Design, In pictures, Attention map, Guided tour,
  Ledger, Findings, Criteria → tests, All files, By commit, Domain terms,
  Your notes.
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

### Tracking what has been read

Which changed lines are read is worked out from the review records every
time, never stored, so it survives re-runs that move line numbers and costs
the hosted store no extra writes. A changed line is an added head line or a
removed base line; context never needs reading.

- **What counts as reading a line**: a verdict of any kind on a tour stop
  (its range), a ledger row (its new optional `start`/`end`, or its call line
  alone), a file (all of it) or a commit (every line it changes), and the
  lines a reviewer marks one at a time.
- **All files** shows a progress bar and count per file, a "Still to read"
  filter, and a summary naming what read the file. In the code, a line still
  to read carries an amber edge; a read line carries a green edge, a tick and
  faded text, with a tooltip naming what read it and the commit that last
  changed it.
- **Marking lines one at a time**, in All files and By commit: click a line
  number, shift-click for a range, or use the link above each run of unread
  lines. `n` jumps to the next unread line. A line read through a verdict
  cannot be unticked on its own; its tooltip names the verdict to untick.
- **In the other direction**, a stop or file whose lines are already read
  elsewhere shows "Read via …" without a verdict of its own. The ledger meter
  still counts only Safe/Unsure/Unsafe judgements.
- **By commit** (under Reference) steps through each commit's message and
  hunks with three lines of context, with Previous/Next or `j`/`k`, and takes
  a Reviewed/Skimmed/Issue verdict per commit. A line a later commit rewrote,
  or that only an earlier commit added, is **superseded**: shown greyed and
  markable, but not counted, because what merges is the code at head. All
  files lists the commits that changed each file.
- **Your notes** ends with the lines still to read, file by file, and lists
  commit verdicts. Line marks are stored as one `lines:<path>` record per file
  holding range strings (`new:12-15`), and commit verdicts as
  `commit:<sha>`.

### The scripts

- **`export_trees.py`** fetches each changed file at the merge base and head
  through `gh api`, so no local checkout is needed and the skill stays
  VCS-neutral. A renamed file's base body lands under its new name, so its
  diff shows the edit rather than a whole-file addition.
- **`extract_diff.py`** keeps a source file's whole body as context, so the
  page can show any line, and cuts bulk files (lockfiles, minified output,
  files over 4,000 lines) to five lines either side of a change.
- **`export_commits.py`** lists the PR's commits and fetches every file each
  one leaves behind, plus each file's body at the merge base as the first
  commit to touch it found it. Merge commits are listed but not exported.
- **`extract_commits.py`** diffs each commit and follows every line it
  changes through the later commits, giving it a fate: its key in the whole
  diff (`new:<head line>` or `old:<base line>`), or superseded by a named
  commit. Where identical lines sit side by side and the whole diff chose a
  different one of them as the changed line, the commit's line takes the
  nearest unclaimed line of the same text.
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
  cannot honour: stops or ledger lines outside their file, ledger ranges that
  miss their call, traced commit lines the whole diff does not change, stops
  with no `fit`, unknown findings, malformed figures, chapters without a
  figure or design content, and duplicate ids. It writes either one
  self-contained file with everything inlined, the coverage model and the
  commits included, or the hosted split of `index.html`, `content.json`,
  `diff.json` and, when there are commits, `commits.json`.
- **`smoke.js`** drives headless Chrome over CDP with no dependencies,
  visiting every registered route at desktop and phone width. It then checks
  that reading flows between views: a ticked stop reads its lines in All
  files, a reviewed file shows the stop as read via it, a line marked in By
  commit reads in All files and the reverse, single lines can be marked, each
  unread run has one mark link, and `n` reaches an unread line.

## Context

No work item. The worked examples come from building a page for #170, the
cache-root pruning change; the example chapter now carries a `fit` on each
stop. The read tracking, line marking and By commit view came from reviewing
#170 through its page, and the #170 page was updated with the same changes.

This branch is stacked on #179 only because both were raised together; it
does not depend on that change.

## Testing

- [x] `uv run pytest .claude/skills/explore-pr/tests`: 87 passed, including
  the coverage model's 29 node tests
- [x] `mise run build-system:check` passes. Repo-wide `ruff` picks up the
  skill's Python files through its nested `ruff.toml`, and `pyrefly` reports
  0 errors.
- [x] Built a synthetic workspace with two commits, one rewriting a line the
  other added, through `export`-shaped data, `extract_commits.py` and
  `build.py`, and ran `smoke.js` against it: every route rendered, including
  By commit, `"ok": true`, every cross-view check passed, with no errors and
  no phone-width overflow. Each of the four commits since the last
  description was checked at its own state.
- [x] Exported and traced #170's 81 commits: no traced line is outside the
  whole diff's changed lines, none is claimed twice, and 18 of the whole
  diff's 24,119 changed lines belong to no commit (see Notes). The same page
  changes, applied to the #170 page, passed the cross-view checks against
  its real data.
- [x] The vendored libraries' SHA-256 sums match `assets/vendor/README.md`
- [ ] The six evals in `evals/evals.json` were not run for this description
- [ ] The full flow against a real PR (reading passes, `build.py` on a real
  workspace with commits, hosted publish) was not re-run since these changes
- [ ] `mise run`, the full default task, was not run

## Notes for Reviewers

- **No CI lane runs the tests.** Lint and type checks cover the scripts, but
  no `mise run test:*` task runs the 87 tests, and the coverage model's node
  suite is skipped where `node` is missing. Decide whether to add a lane or
  accept them as a local-only suite.
- **Line tracing is a heuristic at its edges.** Commit-by-commit diffs and
  the whole diff can pick different copies of identical neighbouring lines
  as the changed one; the nearest-same-text match settles most, but on #170
  18 lines, all braces and blank lines, belong to no commit and can be read
  only from All files or the tour.
- **Commit export is slow and heavy for long PRs.** #170's 81 commits took
  about 80 s of `gh api` calls and produce a 2.1 MB `commits.json`, which the
  page loads after the diff so the first view is not delayed.
- **Merge commits are not reviewed.** They appear in By commit as a stub
  rather than a diff against their first parent.
- **Model-written HTML renders unsanitised.** `md()` passes raw HTML through
  `marked.parse`, and the `fit` field renders through it too. The `svg`
  figure's regex scrub catches quoted `on*=` handlers but not unquoted ones
  (`<svg onload=alert(1)>`) or `javascript:` URLs. Content is written by
  subagents reading untrusted PR text, and the hosted page can write to the
  owner's `reviews` store. DOMPurify or escape-first rendering would close
  this.
- **A failed file fetch fails silently.** `export_trees.py` and
  `export_commits.py` skip any file whose `gh api` call fails, and
  `extract_diff.py` then classifies a modified file with no base body as
  `added`. A transient API error therefore renders as a whole-file addition
  with no warning.
- **An unknown `risk` value breaks the attention map.** The build accepts any
  `risk` in `files*.json`, but the page throws a `TypeError` in the Attention
  map for anything outside `high|medium|low|skim`. The smoke check catches
  it, but the build gives no warning.
- **The review parser is regex-shaped to today's reviews.** `parse_review.py`
  matches three comment layouts; a fourth would drop findings silently
  rather than warn.
- **Skip the vendored and example files.** 3,243 of the 10,700 added lines
  are vendored libraries or worked examples. The review effort belongs on
  `page.html`, `review-coverage.js`, `scripts/` and `SKILL.md`.
