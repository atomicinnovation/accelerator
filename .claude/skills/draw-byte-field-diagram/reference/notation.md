# Byte-field diagram: the notation

```jsonc
{
  "id": "websocket-frame",                    // unique on the page it is shown on
  "kind": "byte-field-diagram",
  "title": "A WebSocket frame says how long it is in 7 bits, and in 16 more when 7 are not enough",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "unit": "bit",                              // optional: "bit" (default) or "byte"
  "perRow": 32,                               // optional: units to a row; 32 for bits, 8 for bytes
  "fields": [{                                // in the order they appear
    "name": "length",                         // the name shown
    "size": 7,                                // in units, or "variable"
    "id": "length",                           // optional: otherwise f1, f2 and so on
    "reserved": true,                         // optional: greyed, for bits that carry nothing
    "status": "new", "tone": "risk",          // optional
    "link": "src/frame.rs:12"                 // optional
  }]
}
```

## How it is drawn

| What | Drawn as |
|---|---|
| The scale | The unit, then a number over every bit or byte of a row, counted from 0 at the left |
| A row | The offset it starts at, to its left |
| A field | A cell as wide as its size, with its name in the middle |
| A field longer than what is left of its row | Carried on in the next row or rows; its name is in every part wide enough for it |
| A field more than two whole rows long | One row with a break in each side, saying its size; the offsets carry on past it |
| A field of `"variable"` size | The rest of its row, with a break in each side, saying *variable*; the next field starts a new row, and rows after it show no offset |
| A `reserved` field | Greyed |

Cells widen, up to a limit, until names fit. A name that still does not
fit its field is replaced by a letter, and the letters are spelled out
under the figure.

## Conventions

- Number bits as the format's own specification does. The scale counts
  from 0 at the left, as network specifications do.
- Use bits where any field is not a whole number of bytes, and bytes
  otherwise.
- Name a field as the code or the specification names it.
- Draw one variant of a format per diagram, and say which.
- Mark what the change under review did with `status`.

## What it checks

`problemsIn` reports a layout with no fields; a `unit` outside its list; a
`perRow` that is not a whole number from 1 to 64; a field with no name,
with a size that is neither a whole number above zero nor `"variable"`, or
with an id used twice; a `reserved` that is not true or false; and a
`status` or `tone` outside its list.

It remarks on fields counted in bits that do not total a whole number of
bytes.

## What it leaves out

- **Numbering from the right**, as hardware registers are often drawn.
- **Values**: what a field holds in a particular frame. Use an object
  diagram.
- **Nested structure**: a group of fields under one name. Draw the group
  as its own diagram.
- **Alternatives** in one diagram: draw each variant.
