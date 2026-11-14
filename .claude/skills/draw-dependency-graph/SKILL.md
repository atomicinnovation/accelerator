---
name: draw-dependency-graph
description: Draw a dependency graph — modules, packages or crates as nested
  boxes joined by "depends on" arrows, with every cycle marked — as an
  interactive page or SVG. Use when the user wants to see what depends on
  what, asks for a dependency or module graph, wants to find dependency
  cycles or check that dependencies run one way, or when another skill needs
  a dependency graph as a figure.
argument-hint: "[what to draw, e.g. the crates under cli/]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a dependency graph

A dependency graph answers one question: **what needs what?** Modules are
boxes, nested as the code nests them, and an arrow runs from a module to
each thing it depends on, drawn as a UML dependency (a dashed line with an
open arrowhead). Any cycle is drawn as a risk and listed under the figure.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/launcher-retention.json` is a graph at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a graph

### 1. Settle what to draw

From the request, decide the **scope** (which part of the codebase) and the
**grain** (crates, packages, modules, or files). A graph is read at a
glance up to about 25 boxes; beyond that, coarsen the grain or narrow the
scope, and say which you did. Ask only if the request names nothing to
start from.

### 2. Find the dependencies

Read them from the code, never from memory or from names:

- between packages: the manifests (`Cargo.toml`, `package.json`,
  `pyproject.toml`, `go.mod`);
- between modules: the imports (`use`, `import`, `require`, `mod`).

Record a dependency only where you saw the line that makes it, and keep
that `path:line` as its `link`. Tell a test-only or build-only dependency
from a runtime one with `kind`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Nest modules as the code nests
them. Give the graph a title that states its point ("Core reaches I/O only
through ports"), not its topic ("Dependencies"), and a caption that says
what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Then look at
the picture in `shots/`: if arrows are hard to follow, set `"reduce": true`
(notation reference, "Reducing a dense graph") and draw it again. If they
still are, the graph is too fine-grained or shows too much: coarsen it or
split it in two. The check needs Chrome or Chromium; without one, say the page was
not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the graph shows in a sentence, name every cycle it found, and say
what you left out and why.
