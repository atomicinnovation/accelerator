---
name: draw-block-diagram
description: Draw a block diagram — named boxes, optionally grouped, joined
  by labelled arrows, with no notation's rules behind it — as an interactive
  page or SVG. Use when the user wants a simple boxes-and-arrows picture of
  how a few things connect or what leads to what, asks for a block diagram,
  or when what is to be shown fits none of the other diagram skills; also
  when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. how a claim's name is derived]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a block diagram

A block diagram is boxes and arrows with no rules of their own: a box is
whatever its name says, and an arrow means whatever its label says. That
makes it the right picture when a handful of things connect in a way no
notation was made for, and the wrong one whenever a notation fits, because
a notation says more with the same ink and the kit can check it.

**Try the other kinds first.** The guide at
`${CLAUDE_SKILL_DIR}/../draw-diagram/reference/choosing.md` lists the
question each answers. Dependencies, calls in order, states, steps of a
procedure, data crossing a boundary, types and tables each have their own.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/claim-name.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

Say in a sentence the point the picture makes. Because nothing else gives
a block diagram meaning, decide what an arrow means in this one (leads to,
is derived from, feeds, calls) and keep to it: one diagram, one meaning.
Ask only if the request names no part of the system.

### 2. Read it from the code

Read what the boxes stand for from the code, never from memory or from
names. Name each box as the code names the thing, and keep the `path:line`
that shows each as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Label every arrow whose
meaning differs from the one you settled on. Give the diagram a title that
states its point ("One claim per install, not per version"), not its
topic, and a caption that says what the arrows mean and what to notice.

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
Say what the diagram shows in a sentence, and say what you left out and
why.
