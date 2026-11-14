---
name: draw-flowchart
description: Draw a flowchart — a start and an end, process steps, decisions
  with their question inside a diamond, inputs and outputs, and
  subprocesses, joined by labelled flow lines, optionally in lanes — as an
  interactive page or SVG. Use when the user wants to see an algorithm, a
  procedure or a piece of control flow step by step, including its loops and
  where paths rejoin, asks for a flowchart, or when another skill needs one
  as a figure.
argument-hint: "[what to draw, e.g. how prune handles one tree]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a flowchart

A flowchart answers one question: **what are the steps, and which way does
it go at each decision?** Steps are boxes, each decision is a diamond with
its question written inside and an answer on every line out of it, and the
lines can loop back or rejoin. Its symbols are the common ones from ISO
5807, which most readers know without a legend.

Use a decision tree instead when no path rejoins and the point is every
case; an activity diagram when things happen in parallel; and a sequence
diagram when the point is who calls whom.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/prune-one-tree.json` is a flowchart at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a flowchart

### 1. Settle what to draw

A flowchart shows **one procedure from start to end**. From the request,
decide which, and at what grain: a step is something a reader can
understand without opening it. A flowchart reads at a glance up to about 12
steps; beyond that, make a subprocess of several steps or draw part of the
procedure. Ask only if the request names no procedure.

### 2. Read the procedure from the code

Follow the code, never memory or names. Note each step, each condition with
every way out of it, each loop and what ends it, and every place the
procedure can stop, including early returns and errors. Keep the
`path:line` of each step as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Word each decision as a
question and put the answer on each flow out of it. Give the flowchart a
title that states its point ("A tree is removed only when it is old and no
claim covers it"), not its topic ("Prune flow"), and a caption that says
what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it against the code. Then look at the
picture in `shots/`. The check needs Chrome or Chromium; without one, say
the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the flowchart shows in a sentence, report every remark that turned
out to be true of the code, and say what you left out and why.
