---
name: draw-byte-field-diagram
description: Draw a byte-field diagram — the fields of a binary format laid
  side by side on a scale of bits or bytes, wrapped into rows, with each
  field as wide as its size — as an interactive page or SVG. Use when the
  user wants to see how a packet, a frame, a header, a file format, a
  record on disk or a hardware register is laid out, asks for a packet,
  bit-field or byte-field diagram, or when another skill needs one as a
  figure.
argument-hint: "[what to draw, e.g. the header of a cache entry]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a byte-field diagram

A byte-field diagram answers one question: **what is where in these
bytes?** A scale along the top numbers the bits or bytes of a row, each
field is a cell as wide as its size, and each row says the offset it
starts at. A field of no fixed size is a row with a break in each side.

Use an entity-relationship or class diagram for the structure of data that
has no fixed layout, and a sequence diagram for the order frames are sent
in.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/websocket-frame.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which layout: one header, one frame, one record.
Where a format has variants (a length that may take 7 bits or 23), draw
one variant and say which in the caption. Ask only if the request names no
format.

### 2. Read the layout from the code

Read it from the code that writes or parses the bytes, or from the
specification it implements, never from memory. For each field, in the
order it appears on the wire or on disk, take its name as the code or the
specification has it and its size. Decide the unit: bits where any field
is not a whole number of bytes, bytes otherwise. Keep the `path:line` that
reads or writes each field as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give the diagram a title that
states its point ("The length comes before anything that depends on it"),
not its topic ("Frame layout"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read any
remark under the figure: it is either a mistake in the spec or a finding.
Settle which against the code. Then look at the picture in `shots/`. The
check needs Chrome or Chromium; without one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the layout shows in a sentence, and say what you left out and
why.
