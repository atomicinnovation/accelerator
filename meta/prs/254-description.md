---
type: "pr-description"
id: "254"
title: "Add a file navigator to the explore-pr review deck"
date: "2026-11-22T10:20:26+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["plan:2026-11-22-explore-pr-file-navigator"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/254"
pr_number: 254
tags: ["explore-pr", "skills", "review", "visualisation"]
revision: "c67848c1474523cf533e47a3a650edececbe03ae"
repository: "accelerator"
last_updated: "2026-11-22T10:20:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add a file navigator to the explore-pr review deck

## Summary

Reviewers asked for four things in the two code-reading views of the pages
`explore-pr` builds:

- All files should be browsable as a tree, sliced in more ways than by role.
- Files should take a verdict in bulk.
- Reviewing should work from the keyboard.
- By commit should show its files one at a time, not as one long page.

This PR builds one file navigator and uses it in both views, then adds a
Readiness link that opens it already filtered.

## Changes

- **A navigator in All files** (`assets/file-navigator.js`):
  - The files show as a collapsible directory tree. A chain of folders that
    each hold only one folder shows as one row, and every folder totals what
    is read beneath it.
  - Group by Tree, None (one flat list), Role, Chapter, Risk or Change.
    Sort by path, lines still to read, size, risk, reading order or first
    commit.
  - Filter by progress, verdict, role, risk, change, chapter, what mentions
    the file, and commit. Each filter opens a pop-over menu that counts what
    each value would show, so the list never moves while you choose.
  - A path search takes `*` wildcards and `!` exclusions.
  - The controls stay pinned while the list scrolls. The list can be widened
    by dragging its edge.
  - A "Show risk" toggle marks each row with a bar coloured by the highest
    risk in it.
- **Bulk verdicts** (`assets/bulk-verdicts.js`):
  - Select files one at a time, by shift-click range or by folder. Then mark
    them Reviewed or Skimmed, or clear their verdicts, together.
  - A file marked Issue keeps it, and the notice says how many were left
    alone.
  - Undo reverses the last 20 changes made in All files and By commit: bulk
    and single verdicts, and line marks. `Store.putMany` saves a bulk change
    with one redraw.
- **Keyboard review** (`assets/review-keys.js`):
  - One table of shortcuts drives both views, and `?` lists them.
  - `j`/`k` move between files and `J`/`K` between commits. `]`/`[` move a
    cursor between chunks of changed lines, and `n`/`N` between unread lines.
  - `m`/`M` mark a chunk or the rest of the file read. `r`/`s` give
    Reviewed or Skimmed, then move on to the next file still to read.
  - `i` raises an issue and starts its note with the chunk's lines, for
    example `head 120–134: `. `Backspace` clears a verdict and `z` undoes.
  - `x`/`X`/`Esc` select and deselect. `/`, `u` and `g` search, show only
    files still to read, and change the grouping. `f`/`c` move the keyboard
    between the list and the code, and the arrow keys walk the tree.
  - A chunk is a run of changed lines that annotations do not break
    (`ReviewCoverage.chunksOf`).
- **By commit on the navigator**:
  - Each commit is a row holding the tree of the files it changes. Only the
    selected commit is open.
  - The pane shows one file of the commit, or the whole commit, through a
    switch remembered per viewer.
  - A file's verdict within a commit is a new `commitfile:<sha>:<path>`
    record. It reads only that commit's lines of the file.
    `ReviewCoverage.commitFileClaims` defines it, and the commit verdict is
    now built from it.
  - Your notes lists these verdicts under "Commit files".
- **Readiness**: the high-risk gate's link opens All files filtered to the
  high-risk files not yet read in full.
- **Speed**: a line over 1,000 characters is no longer highlighted, and one
  over 2,000 is cut short with a count of what is left. A single
  282,187-character JSON line had made opening By commit draw about 65,000
  nodes. It now draws about 1,200, and style and layout fall from about
  135 ms to about 11 ms.
- **Look**: names in the list are smaller and folder names are not bold. The
  selection checkboxes sit in one column. Keyboard focus is drawn inside
  each row. Action links use the body font. The rail, the file list and the
  page hide their scroll bars.

## Context

- Plan: `meta/plans/2026-11-22-explore-pr-file-navigator.md`. It records the
  decisions agreed before building and the refinements agreed while
  reviewing each phase.
- This follows PR #252, which restructured the review deck.

## Testing

- [x] Every new module was written test first: `file-navigator`,
  `bulk-verdicts`, `review-keys`, and the additions to `review-coverage`,
  `review-notes` and `readiness`.
- [x] Skill suites pass: 345 pytest and 208 node tests
  (`uv run --with pytest python -m pytest -q .claude/skills/explore-pr/tests`,
  `node --test .claude/skills/explore-pr/tests/*.test.js`).
- [x] `uvx ruff format --check` is clean. `uvx ruff check` reports the same
  40 findings as on `main`, none in touched lines.
- [x] The full `mise run` exits 0.
- [x] `scripts/smoke.js` passes on a page built from PR #250 (47 files,
  37 commits), with no errors and no sideways scroll at 400 px. Its new
  probes check:
  - the tree, filters, search and folder totals, and that nothing moves while
    filtering;
  - bulk verdicts and Undo;
  - every shortcut, sent as real key events;
  - By commit's per-file verdicts and its pane switch;
  - cut-short long lines, the resizable list, the risk toggle and checkbox
    alignment;
  - the Readiness link.
- [ ] A PR with 100+ files and 50+ commits, by hand.
- [ ] Phone width and dark mode, by eye.
- [ ] A screen reader on the tree.
- [ ] A bulk write of 50+ files in a hosted artifact, where each file is its
  own database write.

## Notes for Reviewers

- Most of the change is in `assets/page.html`. The navigator keeps its
  filters, search, selection and folds per view. A file is known by its path
  in All files and by `<sha>:<path>` in By commit, so selection and movement
  are the same code in both views. The pure rules live in the new modules,
  where the tests are.
- ⚠️ `j`/`k` now move between files in By commit. They used to move between
  commits, which is now `J`/`K`.
- ⚠️ With the page's scroll bar hidden, there is no visible cue to how long
  a page is. This was asked for; thin bars shown on hover would be a small
  change.
- A line cut short hides its tail. The file's GitHub link still shows it
  whole.
- Undo is held in memory, so it does not survive a reload.
- Filters are not saved across reloads, as in the other catalogues.
  Grouping, sort, folds, list width, the risk toggle, the commit-pane switch
  and moving on after a verdict are saved per viewer in `localStorage`.
- In a hosted page a bulk verdict sends one database write per file. The
  page updates at once and the writes finish in the background.
