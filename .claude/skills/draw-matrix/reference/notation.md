# Matrix: the notation

```jsonc
{
  "id": "prune-by-root",                      // unique on the page it is shown on
  "kind": "matrix",
  "title": "Only a relocated root claims before it prunes",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",         // optional
  "rowLabel": "Root", "colLabel": "Trigger",  // optional: what the rows and the columns are
  "rows": ["Default", { "text": "`Relocated`", "link": "src/root.rs:12" }],
  "cols": ["Start-up", "Cold path"],
  "cells": [                                  // one list per row, one cell per column
    ["no claim", { "text": "prune, `Try`", "tone": "ok" }],
    [{ "text": "claim", "tone": "warn", "link": "src/start_up.rs:78" }, "prune"]
  ]
}
```

A row, a column or a cell is text, or an object with `text` and an
optional `link`. A cell's object may also carry a `tone`. Text in
backticks is shown as code.

## Tones

| `tone` | Means | Shown as |
|---|---|---|
| `ok` | The outcome is the safe or intended one | A green tint |
| `warn` | It works, with a caution | An amber tint |
| `bad` | It is wrong, dangerous or refused when it should not be | A red tint |
| `info` | Worth noticing, neither good nor bad | A blue tint |
| `none` | Nothing happens | Greyed text |

## How it is drawn

As an HTML table, not a drawing: it takes the width it is given, wraps
text within cells, and scrolls sideways inside its own frame when it is
too wide for the page.

## Conventions

- Put the dimension a reader scans first in the rows.
- Keep a cell to a few words. A cell that needs a sentence needs a note in
  the caption, or its own figure.
- Fill every cell: say "nothing" or "refused" where that is what happens.
- Use tone for the outcomes a reviewer should look at, not for decoration.

## What it checks

`problemsIn` reports a matrix with no rows or no columns; a row, column or
cell with no text; cells that do not fit the rows and the columns; a
`tone` outside its list; and a `link`, `rowLabel` or `colLabel` that is
not text.

## What it leaves out

- **A third dimension**: draw one matrix per value.
- **Merged cells** and **row groups**.
