# Block diagram: the notation

```jsonc
{
  "id": "claim-name",                         // unique on the page it is shown on
  "kind": "block-diagram",
  "title": "One claim per install, not per version",   // the point, not the topic
  "caption": "What the arrows mean and what to notice.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "blocks": [{
    "id": "sha",                              // unique among all blocks, at every depth
    "name": "SHA-256",                        // optional: the name shown; defaults to the id
    "detail": "first 8 bytes",                // optional, on a block that holds no others
    "status": "new", "tone": "risk",          // optional
    "link": "src/identity.rs:20",             // optional
    "blocks": [ /* optional: the blocks it holds, to any depth */ ]
  }],
  "arrows": [{
    "from": "sha", "to": "id",
    "label": "hex",                           // optional: what the arrow means or carries
    "dashed": true,                           // optional: a dashed line, for a weaker or indirect link
    "status": "new", "tone": "risk",          // optional
    "link": "src/identity.rs:24"              // optional
  }]
}
```

## How it is drawn

| Written as | Drawn as |
|---|---|
| A block | A box with its name and, under it, its detail |
| A block that holds others | A frame around them with its name as a heading |
| An arrow | A solid line with a filled head, carrying its label |
| An arrow with `dashed` | The same, dashed |

The notation gives none of these a meaning. The title, the caption and the
labels do.

## Conventions

- Use another kind of diagram wherever one fits.
- Give arrows one meaning per diagram, say it in the caption, and label
  any arrow that means something else.
- Write the blocks in the order they should be read.
- Mark what the change under review did with `status`.
- Keep to about 10 blocks.

## What it checks

`problemsIn` reports a diagram with no blocks; a block with no id or an id
used twice; a `name` or `detail` that is not text; an arrow with no `from`
or `to`, naming a block the diagram does not have, returning to where it
began, or written twice; a `label` that is not text; a `dashed` that is
not true or false; and a `status`, `tone` or `direction` outside its list.

It remarks on a block joined to nothing when the diagram has arrows.

## What it leaves out

- **Shapes other than boxes**, **arrowheads other than one**, and
  **two-headed arrows**: these carry meaning, and a diagram that needs
  them is one of the other kinds.
- **Rows or layers**: group the blocks, or let the direction order them.
