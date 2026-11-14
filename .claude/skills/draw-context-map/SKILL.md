---
name: draw-context-map
description: Draw a domain-driven design context map — bounded contexts, which
  is upstream and which downstream of each relationship, and the pattern each
  side follows (open host service, published language, anticorruption layer,
  conformist, customer/supplier, shared kernel, partnership) — as an
  interactive page or SVG. Use when the user wants to see how bounded
  contexts, teams, services or models relate and who sets the terms, asks
  for a context map, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. how work items relate to the trackers]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a context map

A context map answers one question: **whose model sets the terms?** Each
bounded context is a box. A relationship between two says which is
upstream (its model changes, and the other must follow) and which is
downstream, and how each side handles that: whether the upstream publishes
a stable language, and whether the downstream translates it or simply
conforms. Where neither sets the terms, the relationship is a partnership
or a shared kernel.

Use a domain model for what is inside one context, and a C4 or component
diagram for how the software is put together.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/tracker-contexts.json` is a map at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a map

### 1. Settle what to draw

From the request, decide which contexts: the ones a change touches, and
the ones they exchange models with. A map reads at a glance up to about 8
contexts. Ask only if the request names no part of the system.

### 2. Read the relationships from the code

Read them from the code, never from memory or from names. A bounded
context is where one model holds: usually a module, crate or service with
its own types for the domain. For each pair that exchanges data:

- **who is upstream**: the one whose types or API the other adapts to;
- **how the upstream offers its model**: a documented API or protocol many
  can use is an open host service, and a documented schema or format is a
  published language;
- **how the downstream takes it**: code that translates the upstream's
  types into its own is an anticorruption layer; using the upstream's
  types directly is conforming;
- **whether they are equals**: two contexts that share code both must
  agree to change have a shared kernel; two that must release together are
  a partnership.

Record what the code does. Keep the `path:line` that shows each as its
`link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give the map a title that
states its point ("Work items keep their own model, with a layer in front
of each tracker"), not its topic ("Context map"), and a caption that says
what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: each is a strategic question about the design,
and it is either a mistake in the spec or a finding. Settle which against
the code. Then look at the picture in `shots/`. The check needs Chrome or
Chromium; without one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the map shows in a sentence, report every remark that turned out
to be true of the code, and say what you left out and why.
