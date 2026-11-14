---
name: draw-component-diagram
description: Draw a UML component diagram — components and the subsystems
  that hold them, the interfaces each provides and requires, and the
  dependencies between them — as an interactive page or SVG. Use when the
  user wants to see the replaceable parts of a system and how they plug
  together, which part implements a port or interface and which uses it,
  asks for a component diagram or a ports-and-adapters picture, or when
  another skill needs one as a figure.
argument-hint: "[what to draw, e.g. the launcher's components]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a component diagram

A component diagram answers one question: **what are the replaceable parts,
and through what do they plug together?** Each component is a box, each
interface is a small ball, and the lines say who provides an interface and
who requires it. A component that requires an interface does not depend on
whoever provides it, and the diagram shows exactly that.

Use a class diagram for the types inside a component, a package diagram for
how the source is organised, and a dependency graph when there are no
interfaces to show.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/launcher-components.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which system or subsystem, and at what grain: a
component is a part that could be replaced by another that provides the
same interfaces. A diagram reads at a glance up to about 12 components and
8 interfaces; beyond that, draw one subsystem. Ask only if the request
names no system.

### 2. Read the components from the code

Read them from the code, never from memory or from names:

- **components**: modules, crates, services or classes that stand behind
  an interface, and the subsystems that group them;
- **interfaces**: the traits, interfaces, ports or protocols through which
  one part is used by another;
- **who provides each**, from the `impl` or `implements`, and **who
  requires each**, from the field, parameter or import that names the
  interface and not an implementation;
- **direct dependencies**, where a component names another component and
  not an interface.

Keep the `path:line` of each as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give the diagram a title that
states its point ("Retention needs a disk and a clock, and does not care
which"), not its topic ("Launcher components"), and a caption that says
what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it against the code: an interface
nothing provides may be provided outside the diagram's scope, and a cycle
through an interface is still a cycle. Then look at the picture in
`shots/`. The check needs Chrome or Chromium; without one, say the page was
not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, report every remark that turned
out to be true of the code, and say what you left out and why.
