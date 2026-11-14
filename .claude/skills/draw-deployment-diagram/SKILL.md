---
name: draw-deployment-diagram
description: Draw a UML deployment diagram — devices and execution
  environments as nested nodes, the artefacts deployed on each, the
  communication paths between nodes and the dependencies between artefacts —
  as an interactive page or SVG. Use when the user wants to see where things
  run or are stored, what is installed on which machine, process or
  directory, what crosses a network or process boundary, or asks for a
  deployment diagram, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. where the launcher and its cache live]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a deployment diagram

A deployment diagram answers one question: **what runs or lives where?**
Each node is a box drawn with depth: a device, or an execution environment
inside one. The artefacts deployed on a node are drawn inside it, a plain
line between two nodes is a way they can communicate, and a dashed arrow
between artefacts is one depending on the other.

It is the picture for a change that moves something across a boundary: to
another process, machine, directory or service.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/launcher-deployment.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which system, and which boundaries matter:
machines, processes, containers, directories, services. A diagram reads at
a glance up to about 8 nodes and 12 artefacts; beyond that, draw one
environment or one side of a boundary. Ask only if the request names no
system.

### 2. Read the deployment from the code and its configuration

Read it from what is actually there, never from memory or from names:
build and packaging scripts, container and service definitions,
installers, configuration, and the code that opens files, sockets and
processes. For each artefact, find where it ends up; for each pair of
nodes that talk, find how. Keep the `path:line` that shows each as its
`link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Nest a node inside the node it
runs on. Give the diagram a title that states its point ("Claims are files
in the user's cache, written by the launcher on every start"), not its
topic ("Deployment"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: an artefact that depends on one on another node
with no path between the nodes means either a path is missing from the
spec or the dependency cannot work as drawn. Settle which. Then look at
the picture in `shots/`. The check needs Chrome or Chromium; without one,
say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, report every remark that turned
out to be true, and say what you left out and why.
