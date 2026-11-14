---
name: draw-c4-diagram
description: Draw a C4 model diagram at the system context or container
  level — the people who use a software system, the systems it works with,
  and, opened up, the containers it is made of and how they talk — as an
  interactive page or SVG. Use when the user wants the big picture of a
  system, who uses it and what it depends on, its applications, services and
  data stores, asks for a C4, context or container diagram, or when another
  skill needs one as a figure.
argument-hint: "[what to draw, e.g. Accelerator's containers]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a C4 diagram

A C4 diagram answers one question: **what is this system, who uses it, and
what is it made of?** At the **context** level a system is one box among
the people who use it and the systems it talks to. At the **container**
level that box is opened into the separately running or deployable things
inside it: applications, services, data stores. Every box says what it is
in words, and every arrow says what it is for.

C4 sets out what each level shows and what each element must say, not a
notation: it has no standard shapes. This skill draws the usual
conventions.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/accelerator-containers.json` is a diagram at the
standard expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

Decide the **system in scope** and the **level**. Context: the system as
one box, with its users and neighbours. Container: the system opened up,
its neighbours still closed. A diagram reads at a glance up to about 10
elements; beyond that, leave out neighbours that do not bear on the point.
Ask only if the request names no system.

### 2. Read the system from the code and its documentation

Read it from what is there, never from memory: the README and
architecture notes for who uses the system and why, and the build,
deployment and source layout for what runs separately. A container is
something that runs or is stored on its own: an application, a service, a
database, a file store. A library linked into an application is not one.
For each arrow, find the code that makes the call and keep its `path:line`
as the `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give every element a short
description of what it is for, and every relationship a label that reads
as a sentence from its source to its target ("Fetches versions from").
Give the diagram a title that states its point, not its topic, and a
caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it. Then look at the picture in
`shots/`. The check needs Chrome or Chromium; without one, say the page was
not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, and say what you left out and why.
