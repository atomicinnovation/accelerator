# The page and what is bundled

`scripts/build.py` builds the page from a workspace of JSON content
(`reference/content-model.md` is the contract). A view whose data is absent
is not shown.

## Views

| View | Gives the reviewer |
|---|---|
| Overview | The change at a glance as a figure, the design in brief, a verdict control, what stands in the way of merging, what changes, breaking changes before and after, accepted risks, CI |
| Design | The design summary and architecture figures; key abstractions with an automatic diagram of how they collaborate; design decisions (chosen over what, why, at what cost) with agree, question or disagree controls; fit with the existing architecture's principles; where the next change goes |
| Attention map | A treemap of every changed line, shaded by risk, and a review-time budget |
| Guided tour | Chapters by design concept in reading order, each opening with its figures, then the terms it uses, then what it adds to the design; code at every stop as the evidence, each placed in the change before it is explained; invariants with the tests that pin them; a sign-off question per chapter |
| Ledger | Every call site of the PR's riskiest class of operation: what guards it, what is held, what races it, what pins it |
| By commit | Each commit in order with its message and hunks, stepped through with Previous/Next or `j`/`k`; lines marked one at a time, by block or by a commit verdict, shared with All files and the tour; lines a later commit superseded shown but not counted |
| Custom views | Whatever this PR needs: a simulator, a stepper, a state machine, an explorer |
| Findings | Each prior finding with an independent verdict against the code, its level (design or implementation), and a triage control |
| Criteria → tests | Acceptance criteria resolved to the tests that pin them |
| All files, Domain terms, Your notes | Annotated diffs showing which changed lines are read and which are still to read, marked one at a time, by block or from the tour, the ledger, a commit or a file verdict, with the commits that changed each file; the change's vocabulary; and everything recorded, copyable as Markdown |

## Bundled files

These ship with the skill, so the page's quality does not depend on
rebuilding them. Paths are relative to the skill directory.

| Path | Purpose |
|---|---|
| `assets/page.html` | The page shell: every core view including Design, the two figures the page draws itself (a comparison of two others, and raw SVG); every other figure is drawn by the diagram kit, the abstraction-collaboration diagram, review controls, highlighting, the treemap, the extension API |
| `assets/review-coverage.js` | Which changed lines the reviewer's verdicts have read, and which of them a verdict from an earlier head still covers, inlined into every build |
| `assets/prose-references.js` | Turns a stop id or a chapter named in prose into a link to it, named by the stop's number and title or the chapter's title, inlined into every build |
| `../diagram-kit/` | The diagram kit, a skill beside this one: it checks and lays out every diagram of a kind a `draw-*` skill draws when the page is built, and its renderer, stylesheet and tracing are inlined into every build. Tracing follows a graph's arrows, outbound and inbound, from every box the reviewer has pressed; pressing a box again drops it and Esc clears them all |
| `assets/vendor/` | marked 12.0.2 and highlight.js 11.9.0, inlined by the standalone build |
| `scripts/export_trees.py` | Base and head copies of every changed file, through the GitHub API |
| `scripts/extract_diff.py` | `data/diff.json` from those trees |
| `scripts/export_commits.py` | Every file each commit leaves behind, through the GitHub API |
| `scripts/extract_commits.py` | `data/commits.json`: each commit's hunks, every changed line traced onto the whole diff |
| `scripts/parse_review.py` | `content/findings.json` from a review-pr review |
| `scripts/criteria_map.py` | `content/criteria.json` from a plan's acceptance-criterion map |
| `scripts/build.py` | The standalone page, or the hosted layout, with every reference checked and every stop and ledger row compared with the code it quotes; `--reanchor` moves those whose code shifted |
| `scripts/smoke.js` | Headless-Chrome check of every route, custom views included: errors, unresolved code, phone-width overflow, lines read flowing between the tour and All files, every kit diagram put through the kit's own checks (text inside its box, nothing overlapping, tracing, a box's ↗ control following its link), a stop named in prose linking to its chapter at that stop, a verdict from an earlier head leaving rewritten lines unread, screenshots |
| `reference/` | The content model, the reading-pass and design-pass briefs, and the custom-view guide |
| `examples/` | A real chapter at the expected standard, with its figures, abstractions and decisions, and the PR 170 prune simulator with its specification |

The shell is a starting point. Copy `assets/page.html` to the workspace and
change it when the PR calls for something the shell cannot do; the build uses
the workspace copy.
