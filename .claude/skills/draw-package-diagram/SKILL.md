---
name: draw-package-diagram
description: Draw a UML package diagram — packages nested as the source is
  organised, and the dependencies, imports, accesses and merges between
  them — as an interactive page or SVG. Use when the user wants to see how
  code is organised into packages, modules or namespaces and which may use
  which, asks for a package diagram or a layering picture, or when another
  skill needs one as a figure.
argument-hint: "[what to draw, e.g. the packages under src/launch]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a package diagram

A package diagram answers one question: **how is the code organised, and
which part may use which?** Each package is a folder, nested as the source
nests it, and a dashed arrow runs from a package to each one it depends
on. It is the picture for layering rules: what is allowed to know about
what.

Use a dependency graph for the same question without UML's package
notation or when the things are not packages, and a component diagram when
the parts meet through interfaces.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/launcher-packages.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which part of the source, and how deep to show its
nesting. A diagram reads at a glance up to about 15 packages; beyond that,
show fewer levels or one subtree. Ask only if the request names no part of
the source.

### 2. Read the packages from the code

Read them from the source tree and its imports, never from memory or from
names. A package depends on another when any file in it imports from the
other. Tell an import that makes names available (`use`, `import`) from a
plain dependency only when the difference matters to the point. Keep the
`path:line` of an import that makes each dependency as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Nest packages as the source
does. Give the diagram a title that states its point ("Only outbound
touches the system, and nothing in it knows the launcher"), not its topic
("Launcher packages"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it against the code. Then look at the
picture in `shots/`. The check needs Chrome or Chromium; without one, say
the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, name every cycle it found, and
say what you left out and why.
