---
type: "plan"
id: "2026-11-22-explore-pr-file-navigator"
title: "explore-pr File Navigator Implementation Plan"
date: "2026-11-22T00:12:56+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "draft"
relates_to: ["plan:2026-11-19-explore-pr-review-deck-restructure"]
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "bb0c42be9b90b46eae811ec2804a0baf293daa06"
repository: "accelerator"
last_updated: "2026-11-22T00:12:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# explore-pr File Navigator Implementation Plan

## Overview

Reviewers using the pages that `explore-pr` builds asked for four changes to
the two code-reading views, All files and By commit:

- **All files** should show a collapsible directory tree by default. Today's
  role grouping should become one filter among several, alongside other ways
  to group, sort and filter a large pull request.
- **Bulk verdicts**: select several files and mark them Reviewed or Skimmed in
  one action.
- **Keyboard shortcuts** for moving between files, giving verdicts, raising an
  issue, and marking chunks of changed lines read.
- **By commit** should use the same navigator, so a commit's files can be
  browsed one at a time instead of as one long page.

The plan builds one **file navigator** and uses it in both views. It has five
phases, and each can be merged on its own.

These decisions were agreed with the user before planning:

| Decision | Agreed |
|---|---|
| By commit layout | Commits are top-level rows in the navigator, each expanding to its own file tree. The pane shows one file of the selected commit, with a One file / Whole commit switch |
| Verdict on a file within a commit | A new `commitfile` kind, target `commitfile:<sha>:<path>`. It claims only that commit's counted lines in that file. The file's head verdict is untouched |
| `j` / `k` | Next / previous file on both views. `J` / `K` move between commits |
| `i` on a chunk | Sets the file's Issue verdict, opens its note and starts it with the chunk's line range. No new data type |
| Moving on after a verdict | After `r` or `s`, jump to the next file still to read. A setting turns this off. No jump after `i` |
| Bulk scope | Bulk offers Reviewed, Skimmed and Clear. Issue is per file. A bulk Reviewed or Skimmed skips files whose verdict is Issue |
| Skimmed | Keeps counting a file's lines as read, as every verdict does today |

## Current State Analysis

All paths are relative to `.claude/skills/explore-pr/`. `P` means
`assets/page.html`.

- **All files.** `vFiles` (P:1823-1837) draws a flat list grouped by
  `layersList()` (P:1515), which is the manifest's `layers`.
  - A text box filters by substring and a checkbox shows only files still to
    read. Both are module-level state (`fileQuery`, `unreadOnly`, P:1820-1821)
    and both re-render the whole view.
  - Selecting a file swaps `#file-pane` in place (P:1888).
  - `filePane` (P:1846-1883) draws the head, the file verdict control
    (`ctl('file:' + path, 'file', …)`), "Changed in" commit links and the
    annotated diff.
- **By commit.** `vCommits` (P:1937-1943) lists commits on the left.
  - `commitPane` (P:1968-1985) stacks every file of the selected commit in one
    `#commit-code` block, each drawn by `commitRowsHtml` (P:1944-1967).
  - `j` and `k` step between commits (P:2010-2014).
- **Keys.** Two separate `keydown` listeners exist, `n` (P:1909-1913) and
  `j`/`k`. Each checks the route and skips form fields itself. There is no
  help panel.
- **Line marks.** A changed row is `tr[data-lk][data-lp]`.
  - Clicking a line number marks one row, and shift-clicking marks a range
    (P:943-958).
  - `unreadRunFrom` and `startsUnreadRun` (P:899-908) find runs of unread rows
    across `tr.anno` rows. `refreshLineMarks` puts a "Mark N lines below read"
    row (`tr.blk`) above each run.
  - `markRows` (P:921-936) writes the `lines:<path>` records.
- **Verdicts.** `KINDS` (P:482-493) holds `file` and `commit`, each with the
  options `ok`/`skip`/`issue`.
  - `Store.put` (P:540-549) writes one record and calls `onStateChange` each
    time. In db mode it flushes each target separately.
  - `recordedClaims` (P:829-841) turns verdicts into reading claims. Any
    status counts, so Skimmed and Issue mark lines read.
- **Commit claims.** `ReviewCoverage.commitClaims`
  (`assets/review-coverage.js:196-201`) claims, for each file, the lines it
  changes in the whole diff (`fate` entries containing `:`) and its own
  pseudo-path.
  - That pseudo-path is `commit:<sha>:<path>` (`commitPath`, :168). It is the
    reason the new verdict target is `commitfile:`.
- **Catalogue filters.** `BLANK_FILTERS` and `filters` (P:1347-1354) hold
  each catalogue's filter state. `[data-jump][data-filter]` links
  (P:1433-1442) set a route's filters before opening it. `files` has no entry.
- **Readiness.** The high-risk gate (`assets/readiness.js:54-56`) links to
  `files` with no filter.
- **Your notes.** `review-notes.js:6-15` lists the record kinds by section.
  An unknown kind falls into "Other".
- **Statuses.** Whole-diff files (`D[path].status`) are `added`, `deleted` or
  `modified`. Commit files keep GitHub's statuses (`added`, `removed`,
  `modified`, `renamed`) with `previous` and `binary`.
- **Build.** Each page model is a path constant (`scripts/build.py:51-58`)
  listed in `with_page_models` (:2222-2233). `tests/test_build.py:234-240`
  lists each model's node suite in `PAGE_MODEL_SUITES`.
- **Smoke.** `scripts/smoke.js` runs fixed probes (`*_PROBE`, :110-460)
  against a built page and folds each into `report.ok` (:543). It is run by
  hand against a real workspace.
- **Baseline.** 342 pytest tests and 139 node tests pass.
  `uvx ruff check .claude/skills/explore-pr` reports 40 existing findings.
  `SKILL.md` is at its 360-line cap (`tests/test_documents.py:44`).

## Desired End State

**All files**
- The left column is a navigator: a tree of changed files with single-child
  folder chains merged into one row.
- Each folder row totals its subtree: the read share, the number of files, the
  highest risk, and the number of verdicts of each kind.
- Above the tree:
  - a Group by control (Tree, Role, Chapter, Risk, Change);
  - a Sort control (Path, Still to read, Size, Risk, Reading order, First
    commit);
  - facet chips (Progress, Your verdict, Role, Risk, Change, Chapter,
    Mentioned in, Commit), each value showing its count;
  - a path search;
  - a "Showing N of M files" line.
- Files can be selected one at a time, by range, or by folder, and the
  selection given a verdict in bulk, with Undo.

**By commit**
- The same navigator with commits as top-level rows. The selected commit
  expands to its file tree.
- The pane shows the commit's head and one file of the commit, or the whole
  commit when the switch is set to Whole commit.
- A verdict on a file there is a `commitfile` record.

**Both views**
- They answer the keys in the table below, listed in a `?` panel generated
  from the same bindings.
- A chunk cursor outlines the current run of changed lines.

**Readiness**
- The high-risk gate opens All files filtered to high-risk files still to
  read.

| Key | Command |
|---|---|
| `j` / `k` | Next / previous file in navigator order |
| `J` / `K` | Next / previous commit (By commit) |
| `]` / `[` | Next / previous chunk |
| `n` / `N` | Next / previous unread line |
| `m` / `M` | Mark the chunk read (again to unmark) / mark the file's remaining lines read |
| `r` / `s` / `i` | Reviewed / Skimmed / Issue on the selection, else the current file |
| `Backspace` | Clear the verdict on the selection, else the current file |
| `z` | Undo the last change |
| `x` / `X` / `Esc` | Toggle the current file in the selection / select every file shown / clear the selection, or close the `?` panel |
| `/` | Focus the path search |
| `u` | Toggle the Still to read filter |
| `g` | Cycle Group by |
| `?` | Show every shortcut |
| Arrow keys | Move, collapse and expand while the tree has focus |

Verify with the automated criteria in each phase and the manual steps at the
end.

### Key Discoveries

- **Coverage needs no new rules.** A `commitfile` record becomes a reading
  claim exactly as a `commit` record does (P:837-838). The new verdict changes
  how the reading bars, the meters and Readiness count lines, without new
  counting code.
- **The commit verdict can be built from the new per-file claim.**
  `commitClaims` is already a per-file `flatMap` (`review-coverage.js:197`).
  Extracting `commitFileClaims(commit, file, by)` gives both verdicts one
  definition.
- **Chunks already exist on the page.** The run that `unreadRunFrom` walks
  (P:899) is a chunk restricted to unread rows. A chunk is the same run of
  `tr[data-lk]` rows across `tr.anno` rows, whatever their state.
- **Refreshing in place is established.** `refreshCoverage`,
  `refreshControls` and `refreshCommitCoverage` patch `data-*` elements
  without a re-render. Navigator totals follow the same pattern, so a verdict
  never rebuilds the tree or moves focus.
- **Commits have no renames in the whole diff.** `extract_diff.py:72-77`
  reports only `added`, `deleted` and `modified`. The Change facet offers
  `renamed` only in By commit, where GitHub's status supplies it.

## What We're NOT Doing

- Issues anchored to lines, shown inline and carried across heads. `i` writes
  the file's Issue note with the range as text.
- Bulk Issue, and bulk "mark lines read".
- A separate progress colour for Skimmed.
- Saving filters across reloads. Filters stay in memory like every other
  catalogue's. Group by, Sort, collapsed folders, the move-on setting and the
  One file / Whole commit switch are saved per viewer in `localStorage`.
- Undo that survives a reload, or that covers verdicts given outside All
  files and By commit.
- Changing what an existing `file`, `commit` or `lines:` record means.
- Virtualised list rendering. See Performance Considerations.
- Wiring the skill's suites into `mise run`, or automating the smoke check.

## Implementation Approach

The rules in `CLAUDE.md` hold throughout:
- every change starts from a failing test;
- names follow the reviewer's domain (navigator, facet, chunk, selection,
  bulk verdict);
- comments only where nothing else can carry the meaning;
- 80-column lines.

The logic lives in **pure modules** with `node:test` suites, in the same UMD
form as `review-coverage.js`:
- `assets/file-navigator.js`
- `assets/bulk-verdicts.js`
- `assets/review-keys.js`
- additions to `assets/review-coverage.js`

The page builds plain **file entries** from `C.files`, `D`, `COMMITS` and the
`Store`. It passes them to the modules and draws what comes back. The modules
never touch the DOM or the `Store`.

Each new module also needs:
- a path constant and a `with_page_models` entry in `scripts/build.py`;
- an entry in `PAGE_MODEL_SUITES`;
- a row in `reference/page.md`'s bundled-files table.

Each phase adds a smoke probe and updates `reference/page.md` for the views
it changes. `SKILL.md` stays net-zero in lines.

## Shared verification commands

Run these from the repository root.

```bash
uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests
node --test .claude/skills/explore-pr/tests/*.test.js
uvx ruff format --check .claude/skills/explore-pr
uvx ruff check .claude/skills/explore-pr --statistics
mise run check
```

`ruff check` must report no more than its 40 findings today, and none in
lines a phase touches.

The smoke check runs against a workspace built from a real PR with commits
and more than 40 files:

```bash
node .claude/skills/explore-pr/scripts/smoke.js --page <workspace>/out/index.html
```

---

## Phase 1: The navigator in All files

### Overview

Replace the flat role list with the navigator. Phase 1 adds grouping,
sorting, facet filters, path search and folder totals. Selection and keys
come later.

### Changes Required

#### 1. File navigator model

**File**: `assets/file-navigator.js` (new), `tests/file-navigator.test.js`
(new)

A **file entry** is what the page hands the model:

```js
{
  path, layer, risk, chapter, readingOrder,
  changes: ['modified', 'binary'],
  add, del,
  coverage: { changed, read, state },
  verdict: 'ok' | 'skip' | 'issue' | '',
  mentions: ['stop', 'ledger', 'finding'],
  commits: ['<sha>', …],
}
```

Here `readingOrder` is the chapter's tour index, followed by the line of the
first stop in the file.

```js
FileNavigator.parseQuery(text)
FileNavigator.matches(entry, filters, query)
FileNavigator.facetCounts(entries, filters, query)
FileNavigator.treeOf(entries, { groupBy, sortBy, groups })
FileNavigator.rowsOf(tree, collapsed)
FileNavigator.fileOrder(tree)
FileNavigator.ancestorsOf(tree, path)
FileNavigator.filesUnder(node)
```

**Filters** are `{ progress, verdict, layer, risk, change, chapter, mentions,
commit }`, each a list of chosen values, empty meaning any. `BLANK` is
exported.

**Tests first.**

*Query*
- Terms separated by spaces must all match.
- Matching is a case-insensitive substring of the path.
- `*` matches any run of characters within the path.
- A term starting with `!` excludes paths it matches.
- An empty query matches everything.

*Filters*
- Values within one facet widen the match. Different facets narrow it.
- `progress` values are `unread`, `partial` and `read`, from
  `coverage.state`. A file with no line-level changes counts as `read` only
  when it has a verdict.
- `verdict` value `none` matches an empty verdict.
- `change` matches any value in `entry.changes`.
- `mentions` and `commit` match any value in the entry's list.

*Facet counts*
- Each facet value counts the entries that would match if that value alone
  were chosen in its facet, with every other facet's filters and the query
  still applied.
- A value with a zero count is still returned, so its chip can show 0.

*Tree*
- `groupBy: 'tree'` nests files by directory.
  - A chain of folders that each hold one folder and no files merges into one
    folder named `a/b/c/`.
  - A folder holding a single file is not merged into the file.
- Sibling order: folders before files, then by `sortBy`, then by path.
  - `path` sorts alphabetically.
  - `unread` sorts by unread lines, most first.
  - `size` sorts by `add + del`, largest first.
  - `risk` sorts high, medium, low, skim.
  - `reading` sorts by `readingOrder`, files outside the tour last.
  - `commit` sorts by the index of the first commit that touched the file.
  - A folder sorts by the same measure taken over its subtree: the sum for
    counts, the highest for risk, the earliest for order.
- Each folder carries totals:
  `{ files, changed, read, highestRisk, verdicts: { ok, skip, issue, none } }`.
- Any other `groupBy` gives one top-level group per value, each holding a
  directory tree of its files.
  - Groups follow `groups[groupBy]` order: manifest layers, tour chapters,
    risk tiers, or `added`, `modified`, `deleted`, `renamed`.
  - Values outside that order follow, labelled by their id.
  - Files with no chapter go in a final group "Not in the tour".
  - A group with no matching files is left out.
- `rowsOf` returns the visible rows depth-first, each with `depth`. Rows under
  a collapsed folder or group are left out.
- `fileOrder` returns every file path in display order, including those under
  collapsed folders.
- `ancestorsOf` returns the folder and group ids that must be open to show a
  file.

#### 2. Page: entries, navigator and preferences

**File**: `assets/page.html`

- `fileEntries(recorded)` builds the entries:
  - `changes` from `D[path].status` and `binary`;
  - `mentions` from `annotationsFor`'s sources (stops, the ledger, findings),
    read once per render;
  - `commits` from `COMMITS`.
- `NavigatorPrefs` reads and writes `explore-pr-nav:<repo>#<number>`.
  - It holds `groupBy`, `sortBy`, `collapsed` (per route), `advance` (true by
    default) and `commitPane`.
  - Every access is wrapped in `try`/`catch`, and the page uses the defaults
    when storage is unavailable.
- `BLANK_FILTERS.files` takes `FileNavigator.BLANK`. `fileQuery` and
  `unreadOnly` are removed. The Still to read checkbox becomes the Progress
  chip with `unread` and `partial` chosen.
- `vFiles` draws `#navigator`, which contains:
  - the controls row (Group by, Sort, path search);
  - facet chips as `<details>` menus of checkboxes with counts;
  - "Showing N of M files";
  - the tree as `role="tree"` with `role="treeitem"` rows.
- Rows:
  - A folder row has a chevron, its name, a reading bar, `read/changed`, its
    file count and a risk dot. It carries `aria-expanded` and `aria-level`.
  - A file row keeps today's `.fl-item` content.
  - The current file carries `aria-current="true"`.
- A "Collapse read" button collapses every folder whose totals are fully
  read.
- Opening a file expands its ancestors, so a `data-open` link from elsewhere
  always lands on a visible row.
- `refreshNavigator(recorded)` runs from `onStateChange` on the `files`
  route. It patches folder totals in place through `data-folder-of`
  attributes.
- The filtered set is fixed when the filters change. A file that stops
  matching after a verdict stays visible, dimmed with `.stale`, until the next
  filter change, so the list does not move under the reviewer.
- At phone width, `#navigator` sits above the pane inside a
  `<details open>` drawer and closes when a file is opened.

**File**: `scripts/build.py`, `tests/test_build.py`
**Changes**: Add `FILE_NAVIGATOR_MODEL` to `with_page_models`, and
`"file-navigator"` to `PAGE_MODEL_SUITES`. Test first: both layouts inline
it.

#### 3. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: `NAVIGATOR_PROBE` checks that:
- the default Tree grouping draws a folder row for the deepest shared
  directory;
- collapsing a folder hides its files, and opening a file under it expands
  it;
- choosing a Role chip narrows the rows to that layer and the chip counts
  match the rows;
- `!test` in the search excludes test paths;
- a verdict on a file updates its folder's total without a re-render (the
  folder element is the same node before and after).

The probe is added to `report.ok`. The 400 px overflow check covers the
drawer.

**File**: `reference/page.md`
**Changes**: Rewrite the All files row. Add `assets/file-navigator.js` to the
bundled files.

### Success Criteria

#### Automated Verification

- [ ] The `file-navigator` node suite passes:
  `node --test .claude/skills/explore-pr/tests/file-navigator.test.js`
- [ ] Both layouts inline the model:
  `uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests/test_build.py`
- [ ] Every command under Shared verification commands passes
- [ ] The smoke report has `navigator` holding and `ok: true`

#### Manual Verification

- [ ] On a PR with 100+ files, the tree opens readable without scrolling past
  folders that hold one file each.
- [ ] Every Group by and Sort choice gives an order that makes sense, and
  switching between them keeps the open file.
- [ ] The facet counts read correctly when two facets are combined.
- [ ] At phone width, the drawer opens, picks a file and closes.

---

## Phase 2: Multi-select, bulk verdicts and Undo

### Overview

Add selection to the navigator, bulk Reviewed, Skimmed and Clear, and an
in-memory undo stack.

### Changes Required

#### 1. Bulk verdict model

**File**: `assets/bulk-verdicts.js` (new), `tests/bulk-verdicts.test.js`
(new)

```js
BulkVerdicts.plan(records, targets, status)
BulkVerdicts.undoOf(records, targets)
```

`records` maps a target to its stored record. `plan` returns
`{ writes: [{ target, patch }], skipped: [{ target, reason }], undo }`.

**Tests first.**
- Giving `ok` or `skip` writes `{ status }` to every target whose status
  differs.
- A target that already has the status is neither written nor skipped.
- A target whose status is `issue` is skipped with reason `issue`, unless the
  status being given is `''`.
- Giving `''` clears every target with a status and keeps its note.
- `issue` is refused: `plan` throws, because Issue is per file.
- `undo` restores each written target's `status`, `note`, `head` and
  `claimed` exactly. A target with no record before restores to
  `{ status: '', note: '' }`.
- Repeated targets are written once.

#### 2. Store and undo stack

**File**: `assets/page.html`

- `Store.putMany(entries, opts)` writes every record, bumps `version` once
  and calls `onStateChange` once.
  - In local mode it writes `localStorage` once.
  - In db mode it queues every target through the existing `flush`.
  - `Store.put` becomes `putMany` with one entry.
- `UndoStack` holds up to 20 `{ label, undo }` entries in memory.
  - Bulk writes, line marks and file or commit-file verdicts given in All
    files or By commit push onto it. This includes clicks on their controls.
  - Undoing writes the inverse with `preserveHead: true`.
- A toast in `#undo-toast` (`role="status"`) reads, for example, "Marked 12
  files Reviewed · 2 with an Issue left as they were · Undo". It stays up for
  8 seconds.

#### 3. Selection and the action bar

**File**: `assets/page.html`

- Every file row gets a checkbox. It shows on hover and focus, and on every
  row while the selection is not empty.
  - Clicking it toggles the file.
  - Shift-clicking selects the visible range from the last file toggled.
- A folder or group row gets a tri-state checkbox. Checking it selects every
  visible file under it, and unchecking clears them.
- The selection is a `Set` of paths per route. It survives re-renders. It is
  narrowed to visible files when the filters change, and the toast says how
  many files were dropped.
- `#bulk-bar` (sticky at the top of the navigator) shows while the selection
  is not empty: "N selected", Reviewed, Skimmed, Clear verdict, Deselect.
- The verdict goes to `file:<path>`, with the same `meta` that `filePane`
  passes to `ctl`.

**File**: `scripts/build.py`, `tests/test_build.py`
**Changes**: Register `BULK_VERDICTS_MODEL` and `"bulk-verdicts"`, test
first.

#### 4. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: `BULK_PROBE`
1. Sets a test file to Issue.
2. Selects a folder containing it.
3. Marks the selection Reviewed.
4. Checks every other file in the folder is `ok`, the Issue file is still
   `issue`, and the folder total reads in full except for that file.
5. Presses Undo and checks every record is back as it was.

**File**: `reference/page.md`
**Changes**: Add selection and bulk verdicts to the All files row. Add
`assets/bulk-verdicts.js` to the bundled files.

### Success Criteria

#### Automated Verification

- [ ] The `bulk-verdicts` node suite passes
- [ ] Both layouts inline the model
- [ ] Every command under Shared verification commands passes
- [ ] The smoke report has `bulk` holding and `ok: true`

#### Manual Verification

- [ ] Selecting 200 lockfiles by folder and marking them Skimmed redraws once
  and feels immediate.
- [ ] In a hosted artifact, a bulk write of 50+ files reaches the database.
  Reloading shows every verdict.
- [ ] Undo after a bulk Clear brings back the notes as well as the statuses.

---

## Phase 3: Keyboard shortcuts and the chunk cursor

### Overview

Put every key in one binding table, add the chunk cursor, and give both
views the full keymap. By commit still stacks its files in this phase. There,
`j` and `k` move between the stacked files, and `J` and `K` take over
stepping between commits.

### Changes Required

#### 1. Chunks

**File**: `assets/review-coverage.js`, `tests/review-coverage.test.js`

```js
ReviewCoverage.chunksOf(kinds)
```

`kinds` is a list of row kinds in order: `changed`, `context`, `gap` or
`annotation`. `chunksOf` returns `[{ first, last }]` index pairs.

**Tests first.**
- A chunk is a maximal run of `changed` rows.
- `annotation` rows inside a run do not split it, and are not chunk ends.
- `context` and `gap` rows end a run.
- A list with no changed rows has no chunks.

#### 2. Key bindings

**File**: `assets/review-keys.js` (new), `tests/review-keys.test.js` (new)

```js
ReviewKeys.BINDINGS
ReviewKeys.commandFor(press, context)
```

- `press` is `{ key, shiftKey, metaKey, ctrlKey, altKey, inField }`.
- `context` is `{ route, treeFocused, helpOpen }`.
- `BINDINGS` is a list of `{ keys, command, routes, label }`, in the order
  the `?` panel shows them.

**Tests first.**
- Each key in the Desired End State table resolves to its command on `files`
  and `commits`, and to nothing on any other route.
  - `J` and `K` resolve only on `commits`.
  - `?` resolves on both views.
- A press with Meta, Control or Alt resolves to nothing.
- A press in a form field resolves to nothing, except `Esc`, which resolves
  to `blur-field`.
- `j` and `J` are distinct (`key` is case-sensitive).
- Arrow keys resolve to tree commands only when `treeFocused`.
- While `helpOpen`, only `Esc` and `?` resolve, both to `close-help`.
- Every binding has a non-empty label, and no key is bound twice for the same
  route.

#### 3. Page: commands, cursor and panel

**File**: `assets/page.html`

- One `keydown` listener calls `ReviewKeys.commandFor` and dispatches the
  result through a `COMMANDS` map. It replaces the `n` and `j`/`k` listeners.
- **Chunk cursor**
  - The rows of the code in view map to kinds, and `chunksOf` finds the
    chunks.
  - `]` and `[` move `.chunk-cursor` (an outline on the chunk's rows) and
    scroll it to centre.
  - `n` and `N` place the cursor on the chunk of the line they reach.
  - The cursor resets when the file changes.
- **Chunk marking**
  - `m` with every row of the chunk read by the reviewer's own marks unmarks
    them.
  - Otherwise `m` marks the chunk's unread rows through `markRows`.
  - Rows read by other means flash, as a click does today.
  - `M` marks every unread row of the current file.
- **Verdicts**
  - `r`, `s` and `Backspace` go through `BulkVerdicts.plan`, on the selection
    when it is not empty, otherwise on the current file.
  - After `r` or `s` on a single file with `advance` on, the next file still
    to read in `fileOrder` after the current one opens. If there is none, the
    toast reads "Every file shown is read".
- **`i`**
  - Sets the current file's Issue verdict, opens its note and focuses it.
  - With the cursor on a chunk, it starts the note with the chunk's ranges in
    `rangeText` form followed by ": ", for example `head 120–134: `.
  - An existing note keeps its text and gains the range on a new line.
- **Selection and search keys**
  - `x` toggles the current file, `X` selects every visible file, and `Esc`
    clears the selection.
  - `/` focuses the path search, `u` toggles the Still to read progress
    values, and `g` cycles Group by.
  - `z` undoes.
- **Tree keys**
  - The tree uses a roving `tabindex`.
  - Up and down move between rows.
  - Left collapses a folder or moves to its parent; right expands a folder.
  - `Enter` opens a file.
- **Help panel**
  - `?` opens `#keys-help`, a `role="dialog"` listing `BINDINGS` grouped by
    route with each key in `<kbd>`.
  - A "Move on after a verdict" checkbox there sets `advance`.
- **By commit in this phase**
  - `j` and `k` scroll to the next or previous file block in the stacked
    pane. At either end they cross into the neighbouring commit.
  - `J` and `K` step between commits.
- **Copy**
  - The ledes of both views drop their key lists and say "Press `?` for
    shortcuts".
  - `vCommits`'s pager hint reads `J` and `K`.

**File**: `scripts/build.py`, `tests/test_build.py`
**Changes**: Register `REVIEW_KEYS_MODEL` and `"review-keys"`, test first.

#### 4. Smoke probe

**File**: `scripts/smoke.js`

- `KEYS_PROBE` dispatches `KeyboardEvent`s on `files` and checks that:
  - `j` changes the current file to the next in `fileOrder`;
  - `]` puts `.chunk-cursor` on the first chunk;
  - `m` makes that chunk's rows `.read`;
  - `i` sets `issue` and leaves the note starting with the range;
  - `z` undoes the Issue;
  - `?` opens `#keys-help`, and `Esc` closes it.
- `COMMITS_PROBE`'s `nextUnreadKey` check keeps working through the new
  listener. Its `j` usage moves to `J`.

**File**: `reference/page.md`
**Changes**: Update the By commit and All files rows for keys. Add
`assets/review-keys.js` to the bundled files.

### Success Criteria

#### Automated Verification

- [ ] The `review-keys` and `review-coverage` node suites pass
- [ ] Both layouts inline the model
- [ ] Every command under Shared verification commands passes
- [ ] The smoke report has `keys` and `commits` holding and `ok: true`

#### Manual Verification

- [ ] A file can be read and given a verdict from the keyboard alone, from
  the first chunk to the next file.
- [ ] Typing in a note or the search never triggers a shortcut, and `Esc`
  leaves the field.
- [ ] A screen reader announces the tree's rows, levels and expanded states.
- [ ] Turning off "Move on after a verdict" survives a reload.

---

## Phase 4: By commit on the navigator

### Overview

Rebuild By commit on the navigator. Each commit holds its own file tree. The
pane shows one file of the commit by default, and a file's verdict within a
commit is a `commitfile` record.

### Changes Required

#### 1. Commit-file claims

**File**: `assets/review-coverage.js`, `tests/review-coverage.test.js`

```js
ReviewCoverage.commitFileClaims(commit, file, by)
```

**Tests first.**
- It claims, at `file.headPath`, exactly the file's `fate` entries that reach
  the whole diff, plus the whole `commit:<sha>:<path>` pseudo-path.
- `commitClaims(commit, by)` equals the concatenation of `commitFileClaims`
  over the commit's files. The existing `commitClaims` tests keep passing.
- A claim for one file in a commit leaves another file's counted lines in the
  same commit unread under `commitCoverage`.

#### 2. The commit-file kind

**File**: `assets/page.html`
- `KINDS.commitfile` is `{ label: 'File in commit', opts: [ok, skip, issue] }`
  with the same labels and tones as `file`.
- `recordedClaims` adds a claim for each `commitfile:<sha>:<path>` record
  with a status, through `commitFileClaims`, labelled
  `Commit <short sha>`.

**File**: `assets/review-notes.js`, `tests/review-notes.test.js`
- **Tests first**: a `commitfile` record falls in a "Commit files" section
  after "Commits", ordered `issue`, `ok`, `skip`.
- **Change**: add the section.

#### 3. Commit entries in the navigator

**File**: `assets/file-navigator.js`, `tests/file-navigator.test.js`

```js
FileNavigator.commitTreeOf(commits, entriesByCommit, options)
```

**Tests first.**
- One top-level `commit` node per commit, in order, carrying `sha`, index,
  subject and the merge flag.
- Each commit node's children are `treeOf` of its entries under the same
  `options`.
- A commit with no matching entries is kept, with no children, so the
  commit list never has gaps.
- `fileOrder` over a commit tree yields `<sha>:<path>` keys in commit order.
  `j` and `k` therefore cross commit boundaries.
- `rowsOf` shows a commit's files only when the commit is not collapsed.

#### 4. Page: By commit

**File**: `assets/page.html`

- **Entries**
  - `commitFileEntries(commit, recorded)` builds entries from the commit's
    files.
  - `changes` comes from GitHub's status, with `removed` mapped to `deleted`
    and `renamed` kept, plus `binary`.
  - `coverage` is `commitCoverage` over that file alone.
  - `verdict` is the file's `commitfile` status.
  - `path` is `headPath`, falling back to `path`.
- **Left column**
  - `vCommits` draws `#navigator` from `commitTreeOf`.
  - A commit row has its number, subject, reading bar and
    `files with a verdict / files`.
  - Only the selected commit is expanded unless the reviewer expands others.
    Collapsed commits are saved per viewer.
  - The Commit facet is hidden. The Change facet offers `renamed`.
- **Pane**
  - `commitPane(sha, path)` keeps today's head: message, pager, summary and
    commit verdict.
  - With `commitPane` set to `one` (the default), it draws only the selected
    file's `commitRowsHtml`.
  - Above the code is a head with the file path, `+add −del`, the
    `ctl('commitfile:<sha>:<path>', 'commitfile', …)` control and the "in All
    files" link.
  - With `whole` it draws today's stack, each file with its own control.
- **Verdicts**
  - A file in a commit without its own verdict, in a commit that has one,
    shows the commit's verdict as "via commit" through the existing
    `[data-via-note]` hint.
  - Bulk actions, `r`, `s`, `i` and `Backspace` target
    `commitfile:<sha>:<path>`.
- **Keys**
  - `j` and `k` follow `fileOrder` across commits. `J` and `K` move to the
    first file of the next or previous commit.
  - The Phase 3 stacked-file stepping is removed.
- `[data-goto-commit]` links keep working, and open the commit's first file.

#### 5. Smoke probe

**File**: `scripts/smoke.js`
**Changes**: `COMMIT_NAVIGATOR_PROBE` checks that:
- the selected commit is expanded and others are collapsed;
- `j` past the last file of a commit opens the next commit's first file;
- `r` on a file writes `commitfile:<sha>:<path>` and marks only that file's
  counted lines read in All files;
- the Whole commit switch draws every file of the commit.

`COMMITS_PROBE` is updated for the one-file pane.

**File**: `reference/page.md`, `reference/content-model.md`
**Changes**:
- Rewrite the By commit row.
- In `content-model.md`, add the `commitfile` kind to the list of stored
  record kinds. It is reviewer state, not content.

### Success Criteria

#### Automated Verification

- [ ] The `review-coverage`, `file-navigator` and `review-notes` node suites
  pass
- [ ] Every command under Shared verification commands passes
- [ ] The smoke report has `commitNavigator` and `commits` holding and
  `ok: true`

#### Manual Verification

- [ ] On a PR with 50+ commits, the commit list reads as a list. Opening a
  commit shows its files and nothing else expands.
- [ ] Marking a file Reviewed in a commit leaves its lines from other commits
  unread in All files.
- [ ] A file with only superseded lines in a commit can still be given a
  verdict, and that verdict doesn't count those lines.
- [ ] Your notes lists commit-file notes under "Commit files".

---

## Phase 5: Filtered links from Readiness

### Overview

The high-risk gate opens All files filtered to what it is waiting on.

### Changes Required

**File**: `assets/readiness.js`, `tests/readiness.test.js`
- **Tests first**: the high-risk gate's `needsYou` outcome carries the filter
  `{ risk: ['high'], progress: ['unread', 'partial'] }`.
- **Change**: pass that filter to `outcome`.

**File**: `assets/page.html`
**Changes**: The `[data-jump]` handler merges a `files` filter into
`filters.files`. It clears the path search and the selection, and fixes the
filtered set afresh.

**File**: `scripts/smoke.js`
**Changes**: `READINESS_PROBE` follows the high-risk gate's link and checks
that every file row shown is high-risk and not read in full.

### Success Criteria

#### Automated Verification

- [ ] The `readiness` node suite passes
- [ ] Every command under Shared verification commands passes
- [ ] The smoke report has `readiness` holding and `ok: true`

#### Manual Verification

- [ ] From Readiness, "Open the outstanding items" on the high-risk gate
  lands on exactly the files the gate counts.

---

## Testing Strategy

### Unit tests

Every rule lives in a module with a `node:test` suite. The main cases each
suite must cover:

- **file-navigator**
  - folder chains merging at the root, inside a group and down to a single
    file;
  - sorting folders by subtree totals;
  - facet counts ignoring their own facet;
  - `!` and `*` together in one query;
  - commit trees whose commits have no files left after filtering.
- **bulk-verdicts**
  - skipping Issue files;
  - Clear keeping notes;
  - undoing a target that had no record before.
- **review-keys**
  - shifted keys;
  - modifier keys;
  - presses in form fields;
  - presses while the help panel is open;
  - route scoping;
  - no key bound twice.
- **review-coverage**
  - chunks across annotation rows;
  - commit-file claims composing into commit claims.

### Integration tests

`tests/test_build.py` asserts that both layouts inline each new model and
that each model's node suite passes. The smoke probes exercise the page
end-to-end in headless Chrome against a real build.

### Manual testing steps

1. Build a page for a real PR with 100+ files and 20+ commits.
2. In All files, group by Role, filter to Still to read and `!test`, select
   a folder, and mark it Skimmed. Undo, then redo it from the keyboard
   (`X`, `s`).
3. Read one file chunk by chunk with `]` and `m`. Raise an issue on a chunk
   with `i`, then move on with `j`.
4. In By commit, walk two commits with `j` alone. Mark a file Reviewed, then
   check in All files that only that commit's lines are read.
5. Turn on Whole commit, reload, and check the setting stuck.
6. From Readiness, follow the high-risk gate.
7. Repeat steps 2–4 at phone width.

## Performance Considerations

- ⏱️ The tree is drawn as plain rows from `rowsOf`.
  - For 1,000 files with every folder open, that is about 1,300 rows. Drawing
    a string of that size into `innerHTML` takes a few tens of milliseconds,
    which is acceptable. This has not been measured.
  - The Phase 1 smoke probe times a render on its PR. The manual step on a
    100+ file PR is the check.
  - Virtualisation is deferred until a real PR shows a problem.
- Verdicts, marks and bulk writes never rebuild the tree. `refreshNavigator`
  patches totals in place. Only filter, grouping and sort changes re-render.
- ⏱️ `fileEntries` computes `coverageOf` for every file.
  - `recordedClaims` is memoised on `Store.version`.
  - `fileEntries` is memoised on the same key, so one state change computes
    coverage once for both the tree and the facet counts.
- ⏱️ In db mode, a bulk write queues one write per target through `flush`.
  - Each is a separate document set, so a 200-file bulk write is 200
    requests.
  - The UI updates at once. The writes drain in the background, and the
    banner already reports an unreachable database.
  - Batching is deferred until the artifact runtime offers a batch write.

## Migration Notes

- Existing `file:`, `commit:` and `lines:` records keep their meaning. The new
  `commitfile:` records sit beside them.
- An older page ignores `commitfile:` records except in Your notes' "Other"
  section.
- Per-viewer preferences use a new `localStorage` key. Clearing it restores
  the defaults.
- Reviewers who used `j`/`k` for commits now use `J`/`K`. The lede and the
  `?` panel say so.

## References

- Prior plan for this skill:
  `meta/plans/2026-11-19-explore-pr-review-deck-restructure.md`
- Line-coverage model: `.claude/skills/explore-pr/assets/review-coverage.js`
- Current All files view: `.claude/skills/explore-pr/assets/page.html:1819-1927`
- Current By commit view: `.claude/skills/explore-pr/assets/page.html:1930-2026`
- Readiness gates: `.claude/skills/explore-pr/assets/readiness.js:54-56`
