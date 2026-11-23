# The page and what is bundled

`scripts/build.py` builds the page from a workspace of JSON content
(`reference/content-model.md` is the contract). A view whose data is absent
is not shown.

## Views

The rail groups the views in five sections. A section with nothing in it is
not shown.

| Section | View | Gives the reviewer |
|---|---|---|
| Start | Overview | The change at a glance as a figure, the design summary with a link to the article, a readiness strip (the recommendation and the gates by state, linking to Readiness), where you are, what changes, breaking changes before and after, accepted risks, CI |
| Start | Design | The design as an article: headed sections of prose, figures, short anchored code excerpts that claim no lines as read, and asides that open the decisions and abstractions in place; then the sign-off on the design as a whole. Without an article, the summary, the figures and links to the catalogues |
| Start | Attention map | A treemap of every changed line, shaded by risk, and a review-time budget |
| Read | Guided tour | Chapters by design concept in reading order, each opening with its figures, then a strip naming the decisions, abstractions and domain terms it introduces; code at every stop as the evidence, each topped by a strip naming what it shows (the decisions, abstractions, invariants and risks it carries and the domain terms it uses), each opening in place and badged "new here" where the tour shows it first, then its role in the design, why it is shaped this way, what breaks if it is wrong and what to check, with the lines that matter marked; its invariants, risks, findings and tests one line each, each opening in place; a sign-off question per chapter |
| Read | By commit | The All files navigator with each commit, in order, as a row holding the tree of the files it changes; the selected commit open and the rest folded. Beside it the commit's message and one of its files, or the whole commit, stepped through with `j`/`k` across commits and `J`/`K` between them; a verdict on a file within a commit reads only that commit's lines of it; lines marked one at a time, by block, by a file or a commit verdict, shared with All files and the tour; lines a later commit superseded shown but not counted |
| Read | All files | A navigator of every changed file: a directory tree by default, or grouped by role, chapter, risk or change type; sorted by path, lines still to read, size, risk, reading order or first commit; narrowed by progress, verdict, role, risk, change, chapter, what mentions it, commit and a path search, each choice counted; each folder totalling what is read beneath it; each row edged, when asked for, in the colour of the highest risk in it; the list widened by dragging its edge. Files can be selected one at a time, by shift-click range or by folder, and given Reviewed, Skimmed or no verdict together, leaving any file with an Issue as it is, with Undo. Keyboard shortcuts (`?` lists them) move between files, chunks of changed lines and unread lines, mark a chunk or the rest of a file read, give verdicts, raise an issue noting the chunk, select, filter and move through the tree. Beside it, annotated diffs showing which changed lines are read and which are still to read, marked one at a time, by block or from the tour, the ledger, a commit or a file verdict, with the commits that changed each file |
| Interact | Custom views | Whatever this PR needs: a simulator, a stepper, a state machine, an explorer |
| Decide | Readiness | The skill's recommended verdict, its assessment and the conditions that would change it, with each cited item linked; the gates worked out from the content and your records, each with its state, what is outstanding, each item linked and marked with your verdict on it, and a link that opens the matching catalogue already filtered; your own verdict and what stands in the way |
| Decide | Your notes | Everything recorded, copyable as Markdown |
| Reference | Findings | One set of findings from the prior review, the reading passes and the design pass, each with its source, filterable by it: each prior finding with an independent verdict against the code, each observation with what it builds on and where else it was raised, every one with its level (design or implementation) and a triage control. Each chapter lists the findings in it compactly |
| Reference | Decisions | Every design decision with its control, filterable by assessment, chapter and your verdict; fit with the existing architecture; where the next change goes |
| Reference | Abstractions | The diagram of how the key abstractions collaborate, and every abstraction, filterable by status and chapter |
| Reference | Invariants & risks | Every invariant with its control, filterable by the reading pass's view, by whether a test pins it, by chapter and by your verdict; then every risk, highest first |
| Reference | Tests | Acceptance criteria resolved to the tests that pin them, and every test named anywhere with what names it |
| Reference | Domain terms | The change's vocabulary |
| Reference | The ledger | Every call site of the PR's riskiest class of operation: what guards it, what is held, what races it, what pins it |

## Links and where they land

Every item has an address, `#<view>/<item id>`: `#decisions/dec-cap-depth`,
`#tour-claims/stop-claims-s1`. Opening one lands the item just below the top
bar and any filters, outlined for a moment. Back returns a view to where it was
scrolled; a new view opens at its top.

A link to a decision, abstraction, invariant, risk, finding, test or domain
term opens it in place rather than leaving the page: a popover beside the link
on a wide screen, the card unfolded below its paragraph on a phone. The card
carries its review control, where it takes one, and a link to open it in its
catalogue. Esc, Close, the link
again or, for a popover, a click elsewhere closes it. A link to a stop or a
chapter navigates.

Every link to an item that takes a verdict (a stop, chapter, decision,
invariant, finding or ledger row) starts with a mark: an empty ring until the
reviewer records a verdict, then a filled dot in that verdict's tone carrying
✓, ?, ✕, ↻ or –, so it reads without colour. A stop whose lines were all read
elsewhere is marked read. Marks follow every verdict as it is given, wherever
it is given.

A link into a catalogue can open it already filtered: an element with
`data-jump="<route>"` and `data-filter="<json>"` sets that catalogue's filters
before it opens. On a phone a catalogue's filters fold into one row naming how
many are set and how many items are shown. An old link to `#criteria` opens
Tests.

## Bundled files

These ship with the skill, so the page's quality does not depend on
rebuilding them. Paths are relative to the skill directory.

| Path | Purpose |
|---|---|
| `assets/page.html` | The page shell: every core view including Design, the two figures the page draws itself (a comparison of two others, and raw SVG); every other figure is drawn by the diagram kit, the abstraction-collaboration diagram, review controls, highlighting, the treemap, the extension API |
| `assets/review-coverage.js` | Which changed lines the reviewer's verdicts have read, and which of them a verdict from an earlier head still covers, inlined into every build |
| `assets/prose-references.js` | Turns a stop id or a chapter named in prose into a link to it, named by the stop's number and title or the chapter's title, inlined into every build |
| `assets/item-addresses.js` | The address of a view and an item within it, `#<view>/<item id>`, inlined into every build |
| `assets/verdict-marks.js` | Every review control's verdicts, the target each linked item's verdict is recorded under, and the mark a link carries for it, inlined into every build |
| `assets/stop-terms.js` | Which domain terms each stop shows, read from its code and its text, the first stop that shows each, and which terms an abstraction already names, inlined into every build |
| `assets/stop-evidence.js` | Which decisions, abstractions, invariants and risks each stop carries, and which stops show each of them, inlined into every build |
| `assets/review-sections.js` | Which views the rail shows, in which of its five sections, and which route a retired hash now opens, inlined into every build |
| `assets/readiness.js` | The readiness gates, each met, needing the reviewer or not met, worked out from the content and the reviewer's records, inlined into every build |
| `assets/target-renames.js` | The verdicts on old observation targets, copied once to the findings they became, inlined into every build |
| `assets/file-navigator.js` | The All files navigator: the directory tree with its folder totals, the groupings and sorts, the facet filters with their counts and the path search, inlined into every build |
| `assets/bulk-verdicts.js` | What a bulk verdict writes, which files it leaves alone because they carry an Issue, and what undoing any verdict or line mark puts back, inlined into every build |
| `assets/review-keys.js` | The keyboard shortcuts of All files and By commit, what each key does on each view, and the list `?` shows, inlined into every build |
| `assets/catalogue.js` | The Reference catalogues: every decision, abstraction, invariant, risk and named test, each with where it was declared or what names it, inlined into every build |
| `../diagram-kit/` | The diagram kit, a skill beside this one: it checks and lays out every diagram of a kind a `draw-*` skill draws when the page is built, and its renderer, stylesheet and tracing are inlined into every build. Tracing follows a graph's arrows, outbound and inbound, from every box the reviewer has pressed; pressing a box again drops it and Esc clears them all |
| `assets/vendor/` | marked 12.0.2 and highlight.js 11.9.0, inlined by the standalone build |
| `scripts/export_trees.py` | Base and head copies of every changed file, through the GitHub API |
| `scripts/extract_diff.py` | `data/diff.json` from those trees |
| `scripts/export_commits.py` | Every file each commit leaves behind, through the GitHub API |
| `scripts/extract_commits.py` | `data/commits.json`: each commit's hunks, every changed line traced onto the whole diff |
| `scripts/parse_review.py` | `content/findings.json` from a review-pr review |
| `scripts/criteria_map.py` | `content/criteria.json` from a plan's acceptance-criterion map |
| `scripts/build.py` | The standalone page, or the hosted layout, with every reference checked and every stop and ledger row compared with the code it quotes; `--reanchor` moves those whose code shifted |
| `scripts/smoke.js` | Headless-Chrome check of every route, custom views included: errors, unresolved code, phone-width overflow, lines read flowing between the tour and All files, every kit diagram put through the kit's own checks (text inside its box, nothing overlapping, tracing, a box's ↗ control following its link), a stop named in prose linking to its chapter at that stop, a stop naming everything it carries and each name opening what it names in place, a decision named in the design opening in a popover whose verdict marks every link to it, Back returning a view to where it was scrolled, every link to an item that takes a verdict carrying its mark, every line a stop focuses on marked in its code, a verdict from an earlier head leaving rewritten lines unread, each catalogue drawing one card per item and a verdict given in one showing at the stop that first carries it, the design article drawn whole with its excerpts resolved, every observation listed in Findings and narrowed by source, the readiness gates matching their model and following a verdict, the verdict control driving the top bar, each decision met once in the tour, at phone width the top bar on one line, a catalogue's filters folded and stuck below it, an item opened by its address landing below both and a peek unfolding below its paragraph, screenshots (a tall view in 5,000 px tiles); fails rather than hangs when Chrome stops answering |
| `scripts/capture.js` | The smoke check's limits on Chrome: the tiles a tall screenshot is taken in, and the deadline on every DevTools call |
| `reference/` | The content model, the reading-pass and design-pass briefs, and the custom-view guide |
| `examples/` | A real chapter at the expected standard, with its figures, abstractions and decisions; a real design article, written by the design pass for PR 250 (`examples/design-article.json`); and the PR 170 prune simulator with its specification |

The shell is a starting point. Copy `assets/page.html` to the workspace and
change it when the PR calls for something the shell cannot do; the build uses
the workspace copy.
