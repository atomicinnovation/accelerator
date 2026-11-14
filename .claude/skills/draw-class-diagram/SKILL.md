---
name: draw-class-diagram
description: Draw a UML class diagram — classes, interfaces, traits, structs
  and enumerations with their attributes and operations, and the
  generalisation, realisation, association, aggregation, composition and
  dependency relationships between them — as an interactive page or SVG. Use
  when the user wants to see how types relate, asks for a class diagram or a
  type hierarchy, wants to see what implements or depends on what at the
  level of types, or when another skill needs a class diagram as a figure.
argument-hint: "[what to draw, e.g. the types in the retention module]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a class diagram

A class diagram answers one question: **what types are there, and how do
they relate?** Each type is a box with its name, its attributes and its
operations, and each relationship is a line whose ends say what kind it is:
which type is the more general, which is the whole and which the part,
which merely uses the other, and how many of each there are.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/retention-classes.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

A class diagram makes **one point about a design**: these types reach I/O
only through traits; this hierarchy has one root; this aggregate owns
these parts. From the request, decide the point and which types it needs.
A diagram reads at a glance up to about 12 types; beyond that, show fewer
members, leave out types the point does not need, or draw two diagrams.
Ask only if the request names no types or module to start from.

### 2. Read the types from the code

Read them from the code, never from memory or from names:

- **each type**: what it is (class, struct, trait, interface, enum), and
  the fields and methods the point needs, not all of them;
- **each relationship**, from the line that makes it: an `impl`, `extends`
  or `implements`; a field holding another type (and whether it owns it);
  a parameter, a return type or a call that makes one type use another.

Keep the `path:line` of each type and relationship as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Choose each relationship's
kind by what the code does, using the table in the notation reference:
ownership is composition, a shared reference is association or
aggregation, and a type that only appears in a signature is a dependency.
Give the diagram a title that states its point ("Retention knows the disk
and the clock only as traits"), not its topic ("Retention classes"), and a
caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it against the code. Then look at the
picture in `shots/`: if lines are hard to follow, show fewer types or
fewer relationships. The check needs Chrome or Chromium; without one, say
the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, and say what you left out and why.
