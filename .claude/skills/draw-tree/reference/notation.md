# Tree: the notation

```jsonc
{
  "id": "claims-directory",                   // unique on the page it is shown on
  "kind": "tree",
  "title": "What a prune reads in claims/",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",   // optional
  "root": "<root>/claims/",                   // optional: shown at the top
  "items": [{
    "path": "locks/prune.lock",               // from the root; a path ending in / is a directory
    "state": "added",                         // optional: added, removed, changed or kept
    "note": "held for the whole prune",       // optional; text in backticks is shown as code
    "link": "src/prune.rs:40"                 // optional
  }]
}
```

## How it is drawn

| Written as | Drawn as |
|---|---|
| An item | A row: guide lines, its name, and its note beside it |
| A directory on the way to an item | A row of its own, in bold, with no state |
| `state: "added"` | `+` before the name, in green |
| `state: "removed"` | `−` before the name, in red and struck through |
| `state: "changed"` | `~` before the name, in amber |
| `state: "kept"` or none | The name as it is |

Entries are drawn in the order written, each directory's entries together.
It is drawn as text, not a drawing, so it scrolls sideways inside its own
frame when a path is too long for the page.

## Conventions

- Use real names, or names in the form the real ones take.
- Show the entries that make the point; leave the rest out.
- Say in the note why an entry is there or why it goes.
- Mark what the change or the operation did with `state`.

## What it checks

`problemsIn` reports a tree with no items; an item with no path or a path
written twice; a `state` outside its list; a `note`, `link` or `root` that
is not text; and a `where`, which is now written `link`.

## What it leaves out

- **Sizes, permissions and dates** as columns: put what matters in the
  note.
- **Collapsing**: leave out what does not make the point.
