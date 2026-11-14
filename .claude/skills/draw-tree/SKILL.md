---
name: draw-tree
description: Draw a tree — files, directories, keys or modules as an
  indented listing, each marked added, removed, changed or kept, with a note
  — as a page. Use when the user wants to see a layout on disk, what a run
  creates or leaves behind, what a change adds to or removes from a
  directory, a configuration's shape or a module tree; asks for a file or
  directory tree; or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. what a prune leaves in the cache]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a tree

A tree answers one question: **what is where, and what happened to it?**
Each entry is a path. Its mark and colour say whether the change added,
removed or changed it, and its note says why it matters.

Use a package diagram for which part of the source may use which, and a
deployment diagram for what runs where.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/claims-directory.json` is one at the standard expected. Read
both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill draws it.

## When the user asks for one

### 1. Settle what to draw

Decide which directory or structure, and at what moment: before, after, or
what one operation reads. Show the entries that make the point and leave
the rest out. Ask only if the request names nothing to list.

### 2. Read it from the code

Read the layout from the code that creates, reads or removes it, never
from memory. Use real names, or names in the real form. Keep the
`path:line` that writes or removes each entry as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give it a title that
states its point ("What a prune reads in claims/"), not its topic, and a
caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Then look
at the picture in `shots/`. The check needs Chrome or Chromium; without
one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the tree shows in a sentence, and say what you left out and why.
