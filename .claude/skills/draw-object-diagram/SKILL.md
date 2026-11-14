---
name: draw-object-diagram
description: Draw a UML object diagram — particular instances with their
  values at one moment, and the links between them — as an interactive page
  or SVG. Use when the user wants to see a concrete example of a structure, a
  snapshot of state, what is on disk or in memory in a given case, a test
  fixture or a before-and-after of data, asks for an object or instance
  diagram, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. the claims directory after two installs]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw an object diagram

An object diagram answers one question: **what does this look like in one
particular case?** Where a class diagram says what can exist, an object
diagram shows what does exist at one moment: named instances, the values
they hold, and which is linked to which. It is the picture for a worked
example, an edge case, a bug's starting state or a before and after.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/claims-snapshot.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

An object diagram shows **one moment in one case**. From the request,
decide the case (which scenario, which inputs) and the moment (before the
call, after the prune, when the bug shows). A diagram reads at a glance up
to about 10 objects; beyond that, leave out the instances the case does
not turn on. Ask only if the request names no case.

### 2. Work out the instances

Take the types and their fields from the code, and the values from the
case: a test fixture, a reproduction, a log, or the values the scenario
implies. Never invent a value the case does not fix; leave that slot out.
Show the slots the case turns on, not every field.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give the diagram a title that
states what the case shows ("A stale claim still protects its version
until the next prune"), not its topic ("Claims snapshot"), and a caption
that says which values to look at and why. Mark the instance or link the
case turns on with `tone: "risk"`.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Then look at
the picture in `shots/`. The check needs Chrome or Chromium; without one,
say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the case shows in a sentence, where each value came from, and what
you left out and why.
