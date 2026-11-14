---
name: draw-sequence-diagram
description: Draw a UML sequence diagram — participants as lifelines, the
  messages between them in time order, activation bars, replies, creation and
  destruction, and alt, opt, loop, par and break fragments — as an interactive
  page or SVG. Use when the user wants to see the order in which things
  happen between components, asks for a sequence diagram, wants a protocol,
  a call sequence, a start-up order or a race drawn, or when another skill
  needs a sequence diagram as a figure.
argument-hint: "[what to draw, e.g. what happens on start-up]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a sequence diagram

A sequence diagram answers one question: **what happens, in what order,
between whom?** Each participant is a lifeline running down the page, and
each message is an arrow from its sender to its receiver, later ones below
earlier ones. A bar on a lifeline shows when that participant is doing
something, and a frame shows what happens only sometimes, or repeatedly.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/start-up-claims.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

A sequence diagram shows **one scenario**: one request, one start-up, one
failure. From the request, decide which scenario, where it starts and where
it ends, and who takes part. Read up to about 8 participants and 20
messages at a glance; beyond that, choose a coarser participant (a module,
not each of its types) or draw two scenarios. Ask only if the request names
no scenario.

### 2. Read the scenario from the code

Follow the calls in the code, never from memory or from names. For each
message, note the line that sends it and keep that `path:line` as its
`link`. Note which calls wait for an answer (synchronous), which do not
(asynchronous), where the order depends on a condition, and what repeats.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. A call holds, as its own
`steps`, everything its receiver does before answering, which is what
draws the activation bars correctly. Give the diagram a title that states
its point ("Start-up claims its version before any lookup"), not its topic
("Start-up sequence"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Then look at
the picture in `shots/`: follow it top to bottom and confirm it tells the
scenario you read in the code. If it is too long to take in, it is two
scenarios. The check needs Chrome or Chromium; without one, say the page
was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, and say what you left out and why.
