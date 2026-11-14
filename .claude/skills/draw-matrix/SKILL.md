---
name: draw-matrix
description: Draw a matrix — a table of rows by columns in which each cell
  is a short outcome with a tone — as a page. Use when the user wants to see
  which case gets which outcome across two dimensions, such as a decision
  table, a permission or configuration matrix, what each combination of
  inputs does, or a compatibility grid; asks for a matrix or decision
  table; or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. what each root does at each trigger]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a matrix

A matrix answers one question: **which case gets which outcome?** Rows
are one dimension, columns the other, and each cell says in a few words
what happens there, tinted by whether that is fine, a caution or a
problem. It is drawn as a table, so it reflows and scrolls like one.

Use a decision tree when the conditions are asked one after another, and a
state machine when the outcome depends on what happened before.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/prune-by-root.json` is one at the standard expected. Read
both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill draws it.

## When the user asks for one

### 1. Settle what to draw

Name the two dimensions. If there are three, draw one matrix per value of
the third. A matrix reads at a glance up to about 6 columns and 10 rows.
Ask only if the request names no decision.

### 2. Read it from the code

Read every cell from the code, never from memory: find the branch that
handles that combination and say what it does. A cell you could not find
is a finding, not a blank. Keep the `path:line` of the branch as the
cell's `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give it a title that
states its point ("Only a relocated root claims before it prunes"), not
its topic, and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Then look
at the picture in `shots/`. The check needs Chrome or Chromium; without
one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the matrix shows in a sentence, and name any combination the code
does not handle.
