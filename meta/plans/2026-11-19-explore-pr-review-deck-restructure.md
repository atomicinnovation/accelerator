---
type: "plan"
id: "2026-11-19-explore-pr-review-deck-restructure"
title: "Restructure the explore-pr Review Deck Implementation Plan"
date: "2026-11-19T21:58:01+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "draft"
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "41a36fef0fd9de2e2bfe6f2c1b8e2c8a08d348cb"
repository: "accelerator"
last_updated: "2026-11-19T21:58:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Restructure the explore-pr Review Deck Implementation Plan

## Overview

Reviewers using the pages that `explore-pr` builds asked for six changes:

- The Design view should read as an **article**, with figures and short code
  excerpts set between broad descriptions of the design.
- Abstractions and decisions should be **met in the guided tour**, where the
  code that embodies them is shown. They should not be laid out as a wall of
  cards.
- Interactive tools should get their own **Interact** section in the rail.
- Catalogues (decisions, abstractions, invariants, risks, findings, tests, the
  ledger) should live in **Reference**.
- **Decide** should become a readiness check.
- Observations from the reading passes should join the **findings**, without
  repeating findings already raised. **Your notes** should take freeform text.

This plan delivers those changes in six phases. Each phase can be merged on its
own.

The proposal was agreed with the user before planning. These are the decisions
it settled:

- The ledger goes in **Reference**. **Interact** holds custom views only.
- A chapter shows a strip of chips naming its decisions and abstractions. Each
  decision is shown in full, with its control, at the **first stop that
  carries it**.
- Duplicate findings are prevented in two ways. The pass briefs hand every pass
  the whole prior-findings list, and the build **warns** when two findings cite
  overlapping lines without being linked.
- **Your notes** gets a freeform box, plus a card for every recorded item whose
  note can be edited in place.
- The design pass runs **after** the chapter passes, with their output as
  input.
- **Readiness** carries a recommended verdict and the conditions that would
  change it, as well as the gates the page works out for itself.

## Current State Analysis

All paths below are relative to `.claude/skills/explore-pr/`. `P` means
`assets/page.html`.

- **Rail.** `routes()` (P:1032-1047) returns `[id, title, group]` triples,
  sorted into `Start | Read | Decide | Reference`. Custom views and the ledger
  default to Read. Findings and Criteria → tests sit in Decide. All files, By
  commit, Domain terms and Your notes sit in Reference.
- **Design view.** `vDesign` (P:1234-1247) draws the summary and figures, then a
  grid of abstractions, decision cards, the principles table, the extension
  table and the module map. Each chapter repeats the abstraction and decision
  cards through `designSection` under "What it adds to the design" (P:1475).
- **Evidence.** `assets/stop-evidence.js` builds `declared`, `carriedBy` and
  `shownAt` from `stop.carries` alone.
  - The build's `unevidenced` (`scripts/build.py:1272-1295`) warns about an
    undeclared decision or high risk that no stop carries, but only within the
    chapter that declares it.
  - Decisions in `design.json` are never checked for being carried.
  - Abstractions are never checked at all.
- **Observations.** Chapter `ownNotes` are drawn only inside their chapter, as
  "New observations" (P:1467).
  - Their review targets are **positional** (`own:<chapter>-<k>`). Inserting a
    note re-points every stored verdict after it.
  - They never reach the Findings view, and no brief asks a pass to check them
    against prior findings or against other passes.
  - In Python they are touched only by `OBJECT_LISTS` (:165) and `VOCABULARY`
    (:1452).
- **Notes.** `vNotes` (P:1857-1863) is a read-only textarea filled by
  `notesMarkdown()` (P:1824-1846).
  - `notesMarkdown` prints only records whose kind is on a fixed list.
  - `vNotes` counts every record that is not `lines`, so the two can disagree.
  - `onStateChange` re-renders the whole notes view on every change (P:649).
    That would take focus away from anything being typed there.
- **Verdict.** The overall verdict control and "What stands in the way" sit in
  Overview (P:1253-1290). The top-bar chip, the meters and the banner read the
  `decision` record wherever it is drawn (P:996-1028, P:652-666).
- **Store.** `Store.put` (P:517-526) keeps only `status`, `note`, `head` and
  `claimed` from the existing record. A note-only write keeps the old head
  (P:549-554). `ctl()` throws for a kind that is not in `KINDS` (P:572-586).
  `refreshControls()` keeps every control for a target in sync without a
  re-render (P:621-641).
- **Build.**
  - `assemble` (`scripts/build.py:409-470`) builds the model.
  - `sources()` (:1333-1343) decides which content gets reference, type and
    vocabulary checks.
  - `VOCABULARY` (:1437-1457) is keyed by `(collection, field)`.
  - Anchors are checked for stops and ledger rows only: `anchored_items` (:746)
    feeds `anchor_problem` (:765) and `reanchor` (:844).
  - Every page-side model must be listed in `with_page_models` (:1759-1765)
    for either layout to inline it.
- **Tests.**
  - `tests/test_page_shell.py` matches strings in `page.html`; it does not run
    the page.
  - The page-side modules (`stop-evidence.js`, `prose-references.js`,
    `review-coverage.js`) have real `node:test` suites.
  - `scripts/smoke.js` takes its routes from `ExplorePR.routes()`. It runs
    general checks on every route plus fixed probes (:336-374). No test runs
    it; it is run by hand against a real workspace.
- **Limits and mismatches.**
  - `SKILL.md` is at its **360-line cap**, set by
    `tests/test_documents.py:43`.
  - The reading pass recommends a finding triage of `drop`, but the reviewer's
    control offers `reject`. Nothing here depends on that mismatch, and it is
    left alone.
- **Baseline.**
  - The skill's suites are not part of `mise run`. Today 273 pytest tests and
    74 node tests pass.
  - `uvx ruff check .claude/skills/explore-pr` reports 40 existing findings.

## Desired End State

A page built by the skill has this rail:

| Group | Views |
|---|---|
| Start | Overview, Design, Attention map |
| Read | Guided tour, By commit, All files |
| Interact | Custom views (the group is hidden when there are none) |
| Decide | Readiness, Your notes |
| Reference | Findings, Decisions, Abstractions, Invariants & risks, Tests, Domain terms, the ledger |

- **Design** is an article: headed sections of prose, figures, anchored code
  excerpts and asides that link decisions and abstractions. It ends with the
  design sign-off.
- **Each chapter** opens with its figures and a strip of the decisions and
  abstractions it introduces. A decision appears in full, with its control, at
  the first stop that carries it.
- **Findings** is one set drawn from three sources: the prior review, the
  reading passes and the design pass. Findings are linked where they relate,
  and the build warns about overlapping findings that are not linked.
- **Readiness** shows:
  - the skill's recommended verdict and the conditions that would change it;
  - gates worked out from the content and the reviewer's records;
  - the reviewer's own verdict.
- **Your notes** opens with an autosaving freeform box, then lists every
  recorded item with an editable note, then the Markdown export.
- **Process.** The design pass reads the chapter passes' output and writes the
  article.

Verify the end state with the automated criteria below and one end-to-end run
of the skill on a real pull request (see Manual Testing Steps).

### Key Discoveries

- **Same target, same state.** A second `ctl()` for a target already drawn
  elsewhere stays in sync through `refreshControls()` (P:621-641). So a
  decision control can appear at a stop and in Reference with no new state
  code.
- **Moving the verdict is safe.** The `decision` record feeds the chip, the
  meters and the banner wherever its control is drawn.
- **Excerpts don't count as reading.** A code slice claims lines read only
  through `ReviewCoverage.readingClaimsIn` (`assets/review-coverage.js:19-33`).
  Article excerpts drawn with `codeSlice` therefore claim nothing, which is
  what we want.
- **Prose links extend naturally.** `ProseReferences.link`
  (`assets/prose-references.js:36-65`) already links bare and backticked
  hyphenated ids. Decision and abstraction ids (`d-…`, `a-…`) fit the same
  tokeniser.
- **Freeform records are safe.** A record with only a note leaves out the
  banner (P:659), the coverage and the totals. It needs its own textarea,
  because `ctl()` requires a registered kind.
- **Vocabulary keys on the collection name.** `vocabulary_of` matches on the
  last collection name in the trail (`build.py:1471-1480`). New enums must sit
  under collection names that don't collide, which rules out a second
  `risks.level`.
- **Clean-baseline tests are brittle.** Tests that assert `warnings == []`
  (for example `tests/test_stops.py:103`) break as soon as a new check fires on
  their fixture. Each phase updates those fixtures as part of its red step.

## What We're NOT Doing

- Detecting duplicate findings by meaning. The build compares where findings
  sit, not what they say. The briefs and the orchestrator handle the rest.
- Deleting stored review records. Renamed targets are copied forward, and the
  old record stays but is ignored.
- Freeform notes per chapter, or notes shared by several reviewers. The
  artifact's write rule stays owner-only.
- Changing the diagram kit, `review-pr`, `parse_review.py`'s output shape or
  the triage vocabulary mismatch (`drop`/`reject`).
- Wiring the skill's test suites into `mise run`.
- Making the smoke check part of an automated test run.

## Implementation Approach

The rules in `CLAUDE.md` hold throughout:
- every change starts from a failing test;
- names follow the reviewer's domain (gate, observation, catalogue, article);
- comments only where nothing else can carry the meaning;
- 80-column lines.

The page's logic moves into **pure modules** beside the existing ones, under
`assets/*.js` with `node:test` suites:
- `review-sections.js`
- `catalogue.js`
- `review-notes.js`
- `target-renames.js`
- `readiness.js`

This gives the page real behavioural tests rather than more string matching.
`test_page_shell.py` keeps guarding only the layout order of templates. Each
new module is added to `with_page_models` and to `page.md`'s list of bundled
files.

Each phase also does the following:
- updates the content model, the briefs and `page.md` for what it changes;
- keeps `SKILL.md` net-zero in lines, moving detail into `reference/` when
  something has to grow;
- adds a smoke probe for each behaviour that applies to a single route.

## Shared verification commands

Run these from the repository root. Every phase's automated criteria refer to
them.

```bash
uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests
node --test .claude/skills/explore-pr/tests/*.test.js
uvx ruff format --check .claude/skills/explore-pr
uvx ruff check .claude/skills/explore-pr --statistics
mise run check
```

`ruff check` must report no more than the 40 findings it reports today, and
none in lines the phase touches.

---

## Phase 1: Rail sections and the Reference catalogues

### Overview

Regroup the rail into five sections, and build the Reference catalogues the
later phases link into.

The Design view and the chapters keep their current cards until phases 3 and
4. For one phase, abstractions and decisions are drawn both there and in
Reference.

### Changes Required

#### 1. Review sections model

**File**: `assets/review-sections.js` (new), `tests/review-sections.test.js`
(new), `tests/test_review_sections.py` (new; runs the node suite, as
`test_stop_evidence.py` does)

**Tests first**:
- The groups come in the order Start, Read, Interact, Decide, Reference.
- A group with no routes is left out, which hides Interact when there are no
  custom views.
- A custom view with no group, or a group that isn't recognised, goes in
  Interact.
- Read holds the tour, By commit and All files, in that order.
- Reference holds Findings, Decisions, Abstractions, Invariants & risks,
  Tests, Domain terms and the ledger, each only when its content exists.
  The ledger is titled by `ledgerMeta.title`.
- Decide holds Your notes.
- The old hash `criteria` resolves to `tests`.

```js
ReviewSections.of(content, customViews)
ReviewSections.routeFor(hash)
```

`of` returns `[{ group, routes: [{ id, title }] }]`. `routeFor` resolves a
retired hash to the route that replaced it.

**File**: `assets/page.html`
**Changes**: `routes()` and `renderRail()` take their content from
`ReviewSections.of`, and `parseHash()` uses `ReviewSections.routeFor`.

**File**: `scripts/build.py`
**Changes**: add `REVIEW_SECTIONS_MODEL` to `with_page_models`. Test first in
`tests/test_build.py`: the standalone page and the hosted page both inline the
module.

#### 2. Catalogue model

**File**: `assets/catalogue.js` (new), `tests/catalogue.test.js` (new),
`tests/test_catalogue.py` (new)

**Tests first** (`Catalogue.of(content)`):
- `decisions` merges `design.decisions` with every chapter's decisions. Each
  entry carries its declaring chapter, or `null` for design.
- `abstractions` is `content.keyAbstractions`, with its chapters.
- `invariants` and `risks` come from every chapter, in tour order, each with
  its chapter.
- `tests` gathers every test named by a chapter's `tests`, an invariant's
  `pinnedBy`, a ledger row's `tests` or a criterion's resolved hits.
  - Tests are de-duplicated by `path::name`.
  - Each test lists what names it (for example `inv:claims-i1`, `ledger:L-3`
    or `criterion:4`).
- `principles` and `extension` come from `design.json`.

#### 3. Catalogue views

**File**: `assets/page.html`
**Changes**:
- New views:
  - `vDecisions`: decision cards, filterable by assessment, chapter and your
    verdict, then the "Fit with the existing architecture" table and the
    "Where the next change goes" table.
  - `vAbstractions`: the generated collaboration figure (`design.relations`),
    then abstraction cards filterable by status and chapter.
  - `vInvariants`: invariants with their controls, filterable by the reading
    pass's `holds`, by whether a test pins them, by chapter and by your
    verdict; then risks by level.
  - `vTests`: the criteria table that was `vCriteria`, then every pinning test
    from `Catalogue.of`, with what names it.
- The cards keep their element ids (`dec-`, `abs-`, `inv-`, `risk-`), so
  `data-scroll-to` keeps working.
- Each catalogue's filter state lives with its view, as `findingFilter` does.
- Generalise `data-jump-finding` into `data-jump="<route>"` with
  `data-filter="<json>"`, so the readiness gates in phase 6 can open any
  catalogue already filtered. Existing `data-jump-finding` links become
  `data-jump="findings"` with `{"q": id}`.

**File**: `tests/test_page_shell.py`
**Changes**: replace
`test_by_commit_sits_with_the_reference_views_after_all_files` with tests that
assert:
- `routes()` takes its content from `ReviewSections.of`;
- each catalogue view's template keeps its card ids.

#### 4. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: add `CATALOGUE_PROBE`.
- For each catalogue route, it asserts that the number of cards matches the
  number of items in the content.
- It sets the verdict on one decision in `#decisions`, then asserts that the
  same control in `#design` reflects it, and cleans up afterwards.

#### 5. Documents

- `reference/page.md`: rewrite the views table by rail group, and list the two
  new modules.
- `reference/custom-views.md:40`: `group` is now `Interact` by default, and the
  allowed values become Start, Read, Interact, Decide and Reference.

### Success Criteria

#### Automated Verification

- [x] The new node suites fail before the modules exist, and pass after.
- [x] Every shared verification command passes.

#### Manual Verification

- [x] On a rebuilt workspace, the rail shows the five groups. Interact is
      absent when there are no custom views.
- [ ] Each catalogue's filters narrow the list, and a verdict given in a
      catalogue shows wherever else that item's control is drawn.
- [x] The smoke check reports `"ok": true`.

---

## Phase 2: Editable notes

### Overview

**Your notes** opens with an autosaving freeform box, lists every recorded item
as a card with an editable note, and keeps the Markdown export below.

### Changes Required

#### 1. Review notes model

**File**: `assets/review-notes.js` (new), `tests/review-notes.test.js` (new),
`tests/test_review_notes.py` (new)

**Tests first**:
- The Markdown opens with a "General notes" section containing the
  `notes:general` note. The section is absent when there is no such note.
- Records whose kind isn't in the known sections go under "Other". None is
  dropped.
- The number of entries matches the number of items in the Markdown.
- Sections are ordered as `notesMarkdown` orders them today.
- A record recorded before the head moved is flagged.
- Records whose target is a renamed-from target (phase 5) are left out.

```js
ReviewNotes.entries(records, renames)
ReviewNotes.markdown(entries, context)
```

**File**: `assets/page.html`
**Changes**:
- `notesMarkdown` and `vNotes` move onto the model.
- `vNotes` renders, in order:
  1. a labelled `textarea#general-notes`. It writes
     `Store.put('notes:general', 'notes', { note }, { label: 'General notes' })`
     after 700 ms of no typing, and again on `focusout`, mirroring the note
     handling at P:603-619.
  2. one card per entry, drawn with `ctl(target, kind, meta, { noteOpen: true })`,
     where `meta` comes from the stored `label` and `where`. Entries of an
     unregistered kind (`notes`, `lines`) are left out of the cards.
  3. the export textarea and the copy button.
- `onStateChange` stops re-rendering the notes view wholesale:
  - it refreshes the export text in place;
  - it re-renders only when the set of targets with entries has changed and no
    textarea in the view has focus.
- The freeform box is read-only when `!Store.canWrite`.

#### 2. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: add `NOTES_PROBE`.
1. Open `#notes` and type into `#general-notes`.
2. Wait 900 ms.
3. Assert that `Store.get('notes:general').note` holds the text, and that the
   export starts with it.
4. Edit a card's note and assert that the record changed.
5. Restore both records.

#### 3. Documents

- `SKILL.md` "Reading notes back": name the `notes:general` record as the
  reviewer's freeform notes, read first. Keep the line count unchanged.
- `reference/content-model.md` "Review state": add the `notes:general` target.

### Success Criteria

#### Automated Verification

- [x] The review-notes suite fails before the model exists, and passes after.
- [x] Every shared verification command passes.
- [x] `tests/test_documents.py` still holds `SKILL.md` to 360 lines or fewer.

#### Manual Verification

- [ ] Typing in the freeform box survives a reload, both from a file and when
      hosted.
- [ ] `ArtifactData` lists `notes:general` in `reviews`.
- [ ] Editing a card's note updates the same note in the tour.
- [x] Focus is not lost while typing.

---

## Phase 3: The tour carries the design

### Overview

- The per-chapter "What it adds to the design" wall goes.
- A strip of chips under the chapter title names what the chapter introduces
  and decides.
- Each decision is shown in full, with its control, at the first stop that
  carries it. Each abstraction is shown at the first stop that carries it.
- Decisions and abstractions may be linked from their own side with
  `carriedBy`, and the build requires every one to have a carrying stop.
- Prose links decision and abstraction ids to their entries in Reference.

### Changes Required

#### 1. Evidence model

**File**: `assets/stop-evidence.js`, `tests/stop-evidence.test.js`

**Tests first**:
- An item's `carriedBy: [stopIds]` counts as if each stop carried it.
- `shownAt(id)` merges both directions, de-duplicated and in tour order.
- `carriedBy(stop)` includes the items that name the stop.
- `introducedAt(stop)` returns the declared decisions and abstractions whose
  first showing, in tour order, is this stop.
- A `carriedBy` that names a missing stop is ignored.

#### 2. Build checks

**File**: `scripts/build.py`, `tests/test_design.py`,
`tests/test_stop_evidence.py`

**Tests first**:
- `carriedBy` is a text-list field, and each id must name a stop
  (`decision d-x: carriedBy names unknown stop claims-s9`).
- The set of carried ids includes reverse links.
- `abstraction a-x: no stop carries it` is now a warning, for chapters and for
  `design.json`.
- `decision d-x: no stop carries it` now also applies to `design.json`
  decisions.
- Prose that names a backticked `d-…` or `a-…` id that nothing declares is a
  warning, like a stop that does not exist.

Fixtures that assert `warnings == []` gain carrying stops.

#### 3. Prose references

**File**: `assets/prose-references.js`, `tests/prose-references.test.js`

**Tests first**:
- `link(html, chapters, catalogue)` links a bare or backticked declared
  decision, abstraction or invariant id to
  `#decisions` / `#abstractions` / `#invariants`, with
  `data-scroll-to="<prefix><id>"`. The link is named by the item's label.
- An id on its own card is not linked.
- `md()` passes the declared items.

#### 4. Chapter layout

**File**: `assets/page.html`, `tests/test_page_shell.py`

**Tests first** (layout order, in `chapter_layout()` and `stop_layout()`):
- The chapter has no `designSection`.
- The chips strip follows the figures and comes before the terms.
- At a stop, the introduced decision cards follow "Evidence for" and come
  before "Role in the design".

**Changes**:
- `carriedLink` points at the item's entry in Reference.
- `decisionCard` gets a `compact` form for stops, with no "Shown at".
- Abstractions are introduced at a stop as a compact card with no control.

#### 5. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: `EVIDENCE_PROBE` additionally asserts these things:
- each declared decision's control `design:<id>` appears exactly once in the
  tour, inside its first carrying stop;
- each chapter has its strip;
- no chapter has `.abs-grid`.

#### 6. Documents

- `reference/content-model.md`:
  - "Design before lines": decisions and abstractions are met at the stop that
    first carries them.
  - Chapter and design schemas: add `carriedBy`.
  - "What the build requires": the new warnings.
- `reference/reading-pass-brief.md`: order stops so that a decision's first
  carrying stop is where it is best explained, and make sure every abstraction
  has a carrying stop.

### Success Criteria

#### Automated Verification

- [x] The new evidence, prose and build tests fail first, then pass.
- [x] Every shared verification command passes.

#### Manual Verification

- [ ] In a rebuilt chapter, each decision reads naturally at the stop where it
      first appears.
- [ ] Its verdict matches Reference.
- [ ] The chip strip links to the right stop.

---

## Phase 4: The Design article, and the design pass after the chapters

### Overview

- `design.json` gains an `article`, and the Design view renders it.
- The catalogue content leaves the Design view; phase 1 already gave it a
  home.
- The design pass now runs after the chapter passes and writes the article.

### Changes Required

#### 1. Article content model

**File**: `reference/content-model.md`, section "`design.json`"

```jsonc
"article": [{
  "id": "shape",
  "heading": "≤ 8 words: the point of the section",
  "blocks": [
    { "kind": "prose", "md": "md, ≤ 150 words" },
    { "kind": "figure", "figure": { } },
    { "kind": "code", "file": "path", "side": "new",
      "start": 40, "end": 52, "anchor": "pub trait Disk",
      "anchorLine": 41, "focus": [{ "line": 44, "why": "…" }],
      "caption": "md, 1 sentence: what to notice" },
    { "kind": "aside", "refs": ["d-claim-heartbeat", "a-design-store"] }
  ]
}]
```

The arc is recommended, not enforced:
1. the problem and the system before;
2. the shape after;
3. one section per central idea;
4. the fit with the existing architecture;
5. where the next change goes.

`summary` stays, at 3–5 sentences. It feeds the Overview teaser.

#### 2. Build checks

**File**: `scripts/build.py`, `tests/test_design.py`, `tests/test_anchors.py`,
`tests/test_shape.py`

**Tests first**:
- **Shape.**
  - A section needs an `id`, a `heading` and a list of `blocks`.
  - A block's `kind` is `prose|figure|code|aside`.
  - A code block needs `file`, `start`, `end` and `anchor`.
  - Malformed parts are left out with a warning, through `well_formed`.
- **Existence and length.**
  - `design: no article` is a warning when `design.json` exists.
  - `design article: no figure` is a warning.
  - A code block longer than 15 lines is a warning that points to a stop.
  - A prose block of more than 150 words is a note.
- **Anchors.**
  - Code blocks are checked like stops, through an `excerpt_anchorage`
    descriptor yielded by `anchored_items`.
  - `--reanchor` moves them in `design.json`.
  - A range outside the file is a warning.
- **References.** An aside's `refs` must be declared ids.
- **Figures.** Figure blocks are checked and laid out by the kit like any other
  figure. The collectors behind `figure_warnings` (:1013) and `kit_warnings`
  (:638) take in article figures.
- **Prose.** Prose blocks get the prose checks, so the article is reachable
  through `sources()`.

#### 3. Design view

**File**: `assets/page.html`, `tests/test_page_shell.py`

**Tests first** (layout):
- `vDesign` renders the sections in order, then the design sign-off.
- It has no `.abs-grid`, no `decisionCard`, no `principlesTable` and no
  `extensionTable`.
- Without an `article`, it falls back to the summary, the figures and links to
  Decisions and Abstractions.

**Changes**:
- Code blocks render with `codeSlice(file, start, end, side, { pad: 0, focus })`
  and their caption, so they claim no lines.
- A new `after.design` fills pending slices, as `after.tour` does.
- Asides render a compact line for each ref (its label and its assessment
  chip), linking to Reference.
- `figureViews()` counts article figures.
- The Overview's "The design" section becomes the summary plus a "Read how the
  change is designed" link. The list of decisions goes.

#### 4. Design pass after the chapters

**File**: `reference/design-pass-brief.md`
**Changes**:
- **Inputs.** The brief is spawned once the chapter passes finish. It takes
  every `content/chapter-*.json` and each pass's final message.
- **Ids.**
  - Reuse a decision or abstraction a chapter already declared, by id.
  - Declare a new one only when it is cross-cutting.
  - Link the new ones to existing stops with `carriedBy`.
  - Never edit chapter files.
- **The article.** Write it to the contract and the recommended arc. Use
  excerpts only where a few lines show the shape better than a figure.
- **Final message.** Add "decisions you reused and any you disagree with a
  chapter about".

**File**: `SKILL.md`, steps 5, 6 and 8 (net zero lines)
- Step 6: spawn the chapter passes. When they finish, spawn the design pass
  with their output. At quick depth, write the design after your own chapters.
- Step 8: reconciling the design pass with the chapters stays.
- The "Two principles" bullet "The Design view comes second" now describes the
  article.

**File**: `reference/re-running.md`
**Changes**: when an older workspace has no `article`, the design pass writes
one on re-run.

#### 5. Example

**File**: `examples/design-article.json` (new)
**Changes**: take this from the end-to-end run below once the build passes on
it. Name it in `content-model.md` and the design-pass brief, so that
`test_documents.py` checks it exists.

#### 6. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: add `DESIGN_PROBE`. It asserts that:
- the design route renders as many sections as the content has;
- every code block resolved;
- the sign-off control is present;
- there is no `.abs-grid`.

### Success Criteria

#### Automated Verification

- [x] The new build and layout tests fail first, then pass.
- [x] Every shared verification command passes.
- [x] `tests/test_documents.py` passes: `SKILL.md` stays at 360 lines or
      fewer, and the example exists.

#### Manual Verification

- [ ] On a real pull request, the Design view reads as an article.
- [ ] Someone who reads only that view can explain the change's shape, its key
      decisions and where the next change goes.
- [ ] The design pass reused the chapters' ids and added `carriedBy` links.
- [ ] The build passes without reworking any chapter.

---

## Phase 5: One deduplicated findings set

### Overview

- Observations from the reading passes and the design pass become findings
  with ids, severities and levels, and are linked to the findings they repeat
  or extend.
- The build lifts them into one findings set, warns about overlapping
  findings that are not linked, and keeps old `ownNotes` workable.
- Stored verdicts on positional `own:` targets are renamed to `finding:`
  targets.

### Changes Required

#### 1. Observation content model

**File**: `reference/content-model.md`, chapter schema and `design.json`

`ownNotes` is replaced by `observations`. A chapter writes ids
`P-<chapter>-NN`; the design pass writes `D-NN`.

```jsonc
"observations": [{
  "id": "P-claims-01",
  "severity": "critical|major|minor|suggestion|question",
  "level": "design|implementation",
  "title": "≤ 12 words",
  "summary": "md, ≤ 3 sentences",
  "where": "path:line or path:start-end",
  "extends": "R1-06",
  "sameAs": ["P-store-02"],
  "distinctFrom": ["R1-07"]
}]
```

- A repeat of a prior finding is never an observation. It goes in that
  finding's `findingChecks` note.
- `sameAs` joins observations from different passes that report the same
  thing.
- `extends` marks an observation that adds to a finding without repeating it.
- `distinctFrom` records that two findings at overlapping lines are
  different.

#### 2. Findings register

**File**: `scripts/build.py`, `tests/test_findings_register.py` (new)

**Tests first** (`findings_register(findings, chapters, design)`, called from
`assemble`):
- **Sources.** Prior findings get `source: "review"`. Observations become
  findings with `source` `reading-pass` or `design-pass`, along with their
  `chapter`, `file`, `start` and `end` (taken from `where`) and
  `kind: "inline"`, or `"general"` when they have no `where`.
- **Merging.** A `sameAs` group becomes one finding: the first in tour order,
  with `alsoRaisedIn` listing the chapters of the others. Their ids still
  resolve to it, through `C.findingAliases`.
- **Link errors.**
  - `sameAs` naming a review finding is a warning that says to use
    `findingChecks`.
  - An unknown id in `sameAs`, `extends` or `distinctFrom` is a warning.
- **Overlap.** Two findings, at least one of them an observation, that cite
  overlapping lines of the same file and are not linked either way produce a
  warning: `R1-06 and P-claims-01 cite overlapping lines of store.rs
  (210-214); link them with sameAs, extends or distinctFrom`. Two review
  findings never produce this warning.
- **Legacy notes.**
  - `ownNotes` are still accepted. Each becomes a finding with id
    `N-<chapter>-<k>`, severity taken from `level`, and its text as the
    summary.
  - The build adds the rename `own:<chapter>-<k>` → `finding:N-<chapter>-<k>`
    to `C.targetRenames`.
  - The build writes a note: `ownNotes is the old name for observations`.
- **Vocabulary.** New `VOCABULARY` entries for
  `("observations", "severity")` and `("observations", "level")`. The
  existing `("ownNotes", "level")` stays.
- **References.** `finding_references` accepts observation ids anywhere a
  finding id is allowed: stop `findings`, risk `refs`, ledger `findings` and
  readiness `refs`.

#### 3. Target renames

**File**: `assets/target-renames.js` (new), `tests/target-renames.test.js`
(new), `tests/test_target_renames.py` (new)

**Tests first** (`TargetRenames.pending(docs, renames)`):
- Returns one copy for each renamed-from record that has a status or a note,
  when the renamed-to target has no record.
- The copy carries the status, note, head and claimed lines, and takes kind
  `finding`.
- Returns nothing once the target exists, which makes the rename idempotent.
- Ignores renames whose source has no record.

**File**: `assets/page.html`
**Changes**:
- After `Store.init` loads, apply `TargetRenames.pending` through `Store.put`.
  `put` stamps the current head, so the copy must keep the copied `head` and
  `claimed`, and `put` needs a `{ preserveHead: true }` option to allow it.
  Test that option at the module boundary.
- `ReviewNotes.entries` skips renamed-from targets.

#### 4. Findings in the page

**File**: `assets/page.html`, `tests/test_page_shell.py`
**Changes**:
- `findingRow` shows:
  - a source chip;
  - for an observation, its chapter;
  - `extends` as a link;
  - `alsoRaisedIn`.
- The Findings view gains a "Source" filter, and its lede counts findings by
  source.
- `data-jump` resolves aliases.
- The chapter's "New observations" and "Prior findings here" sections become
  one compact "Findings here" list (id, severity, title, verdict chip) that
  links into Findings.

#### 5. Briefs and orchestration

- `reference/reading-pass-brief.md`:
  - Every pass receives **every** prior finding: id, title and location.
  - Before raising an observation, check it against that list and against the
    finding checks. A repeat goes in `findingChecks`; an addition is an
    observation with `extends`.
  - Observations replace `ownNotes`, including unknown whys raised as
    `question`.
- `reference/design-pass-brief.md`:
  - The design pass gets the chapters' observations as well.
  - It writes `observations` with `D-` ids, links repeats with `sameAs`, and
    never restates a chapter's observation.
- `SKILL.md` step 8 (net zero lines): reconcile observations across passes with
  `sameAs` before building. The build's overlap warnings list any pair that is
  still unresolved.
- `reference/re-running.md`: in a workspace that already has `ownNotes`, leave
  them as they are, so stored verdicts keep their targets. Write new findings
  as `observations`.

#### 6. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: add `FINDINGS_PROBE`. It asserts that the Findings view lists
every observation, that the source filter narrows the list, and that a
chapter's "Findings here" links open the matching finding.

### Success Criteria

#### Automated Verification

- [ ] The new register and rename tests fail first, then pass.
- [ ] Every shared verification command passes.

#### Manual Verification

- [ ] A workspace built before this phase, with `ownNotes` and verdicts on
      them, rebuilds.
- [ ] Its verdicts appear on `N-…` findings, and nothing is listed twice in
      Your notes.
- [ ] On a new run, no issue appears twice in Findings.

---

## Phase 6: Readiness

### Overview

**Decide** opens with **Readiness**. It shows:
- the skill's recommended verdict and the conditions that would change it;
- gates worked out from the content and the reviewer's records;
- the principles of the existing architecture;
- the reviewer's own verdict, with what stands in the way.

Overview keeps a compact readiness strip.

### Changes Required

#### 1. Readiness gates model

**File**: `assets/readiness.js` (new), `tests/readiness.test.js` (new),
`tests/test_readiness.py` (new)

`Readiness.gates(content, recordOf, coverageOf)` returns
`[{ id, title, state, detail, outstanding, jump }]`.
- `state` is `met`, `needs-you` or `not-met`.
- `outstanding` lists `{ target, label }` pairs.
- `jump` is `{ route, filter }`.

A gate whose content is absent is left out. **Tests first**, one per state for
each gate:

| Gate | Not met | Needs you | Met |
|---|---|---|---|
| CI | any check `fail` | — | all `pass`/`skipped` |
| Design decisions | any `disagree` | any without a verdict | all `agree`/`question` |
| Fit with the architecture | any principle `broken` | any `strained` | all `holds` |
| Findings | any triaged `block` | any `critical`/`major` not triaged | otherwise |
| High-risk files | — | any `high` file not read in full | all read |
| Invariants | any marked `issue` | any `partly`/`unsure` or unpinned without a reviewer `ok` | otherwise |
| Criteria → tests | — | any criterion test unresolved | all resolved |
| Breaking changes | any without `changelog` | — | all documented |
| Accepted risks | — | any re-raising finding not triaged | otherwise |
| Ledger | any row `issue` | any row without a verdict | all `ok` |
| Chapters | any `blocking` | any without a sign-off | all signed off |

Statuses are read through the `recordOf` passed in, so the model holds no
store code. `KINDS` is unchanged.

#### 2. Readiness content

**File**: `scripts/build.py`, `tests/test_readiness_content.py` (new),
`reference/content-model.md`

```jsonc
{
  "recommendation": "approve|request-changes|comment",
  "summary": "md, ≤ 3 sentences: the assessment behind it",
  "conditions": [{
    "when": "before-merge|would-improve",
    "text": "md: what has to happen",
    "refs": ["R1-06", "claims-i1", "d-claim-heartbeat"]
  }]
}
```

**Tests first**:
- Read with `held(content, "readiness", …)`, with
  `OBJECT_LISTS["readiness"] = ("conditions",)` and added to `sources()`.
- Vocabulary for `("readiness", "recommendation")` and
  `("conditions", "when")`. Check that `vocabulary_of` resolves the top-level
  trail, and add the collection name if it does not.
- Every id in `refs` must name a finding, an invariant, a decision, a stop or a
  chapter.
- `no readiness assessment` is a warning when the file is absent.

#### 3. Readiness view

**File**: `assets/page.html`, `tests/test_page_shell.py`
**Changes**:
- `vReadiness`, as the first route in Decide (update `ReviewSections`, test
  first), renders:
  1. the skill's assessment: the recommendation chip, the summary and the
     conditions grouped by `when`, with refs linked;
  2. the gates, each with its state chip and detail, and an
     "Open the outstanding items" link through `data-jump`. The architecture
     gate expands to the principles table;
  3. the verdict panel: `ctl('decision', …)` and "What stands in the way",
     moved from Overview.
- Overview: the verdict panel is replaced by a readiness strip. The strip
  counts gates by state, shows the recommendation, and links to `#readiness`.
  "Where you are" stays.
- Gates are recomputed on `onStateChange`, in place, when the route is
  `readiness`.

#### 4. Orchestration

- `SKILL.md` step 8 (net zero lines): write `content/readiness.json` last, from
  the reconciled findings, the design assessment and CI.
- The "Report" section names the recommendation.
- Overview `paths` may end at Readiness.
- `reference/page.md`: the Decide views.

#### 5. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: add `READINESS_PROBE`.
1. Assert that the gates drawn match `Readiness.gates` for the content.
2. Set one decision to `disagree` and assert that the design gate turns
   `not-met`.
3. Clear the decision, then assert that the verdict control in `#readiness`
   drives the top-bar chip.

### Success Criteria

#### Automated Verification

- [ ] The readiness model and content tests fail first, then pass.
- [ ] Every shared verification command passes.
- [ ] `mise run` exits 0.

#### Manual Verification

- [ ] On the end-to-end run, every gate's state is right against the records.
- [ ] Each "outstanding" link opens a filtered catalogue holding exactly those
      items.
- [ ] The recommendation and conditions match the reconciled findings.

---

## Testing Strategy

### Unit Tests

- **Pure page models** (`node:test`, each run from a pytest wrapper):
  - `review-sections`: grouping, ordering, hidden groups, hash aliases.
  - `catalogue`: merging across chapters and design, test de-duplication.
  - `review-notes`: section order, no record dropped, counts agree, renamed
    targets skipped.
  - `stop-evidence`: reverse links and first showing.
  - `prose-references`: decision and abstraction links, no self-links.
  - `target-renames`: copy once, preserve head, idempotent.
  - `readiness`: every gate in every state.
- **Build** (pytest):
  - article shape, anchors, `--reanchor` and figure collection;
  - `carriedBy` and the new carried-evidence warnings;
  - the findings register, legacy `ownNotes`, overlap warnings;
  - the readiness content;
  - inlining of every new model.
- **Layout** (`test_page_shell.py`): template order only.

### Integration Tests

- Each phase's smoke probe runs against a real workspace. It checks the
  behaviour of one route in Chrome, and the general checks cover every new
  route: a heading, resolved slices, no overflow at phone width, no errors.

### Manual Testing Steps

1. After phase 4, and again after phase 6, run `/explore-pr` at standard depth
   on a recent pull request in this repository. Deliver it both standalone and
   hosted.
2. Read Design without opening anything else, and judge it as the skill's own
   step 9 asks.
3. Walk one chapter. Check that each decision appears once, at the right stop,
   and that its verdict shows in Decisions.
4. In Findings, filter by source. Confirm that no issue appears twice and that
   `extends` links make sense.
5. Write freeform notes and edit two card notes. Reload, then read them back
   with `ArtifactData`.
6. Work through Readiness until every gate is met or deliberately not met.
   Check each "outstanding" link.
7. Rebuild a workspace made before phase 5 and confirm that its observation
   verdicts survived.

## Performance Considerations

- Moving the design pass after the chapter passes adds one pass to the wall
  clock at standard and deep depth. The user accepted that in exchange for a
  design pass that builds on the chapters.
- The new page models are linear in the content. Readiness recomputes on each
  state change only while its route is open.

## Migration Notes

- **Old workspaces.**
  - Workspaces that still have `ownNotes` keep working. Their findings take
    `N-…` ids, and stored verdicts are copied to the new targets once, when
    the page loads.
  - A workspace whose `design.json` has no `article` still renders, through
    the fallback, but the build warns until the design pass writes one
    (`reference/re-running.md`).
- **Old links.** A bookmark to `#criteria` opens `#tests`.
- **Old records.** Nothing is deleted from the `reviews` collection.

## References

- Skill: `.claude/skills/explore-pr/SKILL.md`,
  `reference/content-model.md`, `reference/page.md`
- Page runtime: `assets/page.html`:
  - Store (P:484-554)
  - `ctl` (P:572-641)
  - routes (P:1032-1086)
  - Design (P:1173-1247)
  - Overview (P:1249-1330)
  - Chapter (P:1441-1484)
  - Findings (P:1554-1594)
  - Notes (P:1824-1870)
- Build: `scripts/build.py`:
  - `assemble` :409
  - anchors :726-866
  - evidence :1184-1298
  - references :1319-1690
  - vocabulary :1437-1507
  - page models :1759
- Smoke check: `scripts/smoke.js` (:322-374)
