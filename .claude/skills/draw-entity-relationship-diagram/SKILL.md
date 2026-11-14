---
name: draw-entity-relationship-diagram
description: Draw an entity-relationship diagram in crow's foot notation —
  tables or entities with their columns, types and keys, and the
  relationships between them with how many at each end — as an interactive
  page or SVG. Use when the user wants to see a database schema, a data
  model, how tables relate, what a migration changes, or asks for an ER or
  entity-relationship diagram, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. the tables the sync migration touches]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw an entity-relationship diagram

An entity-relationship diagram answers one question: **what is stored, and
how do the things stored relate?** Each entity is a box listing its
attributes with their types and keys, and each relationship is a line whose
ends say how many: one, at most one, at least one, or any number. It is the
picture for a schema and for what a migration does to it.

Use a class diagram for types in code, and a domain model for the domain's
aggregates and their boundaries.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/work-items.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which part of the schema: the tables a feature or
a migration touches, and the ones they refer to. A diagram reads at a
glance up to about 10 entities; beyond that, draw the tables around one
subject, and show only keys for the tables at its edge. Ask only if the
request names no schema.

### 2. Read the schema

Read it from what defines it, never from memory or from names: migrations,
schema files, ORM models, or the structs that are serialised. For each
table take its columns with their types, which may be empty, the primary
key, unique columns, and each foreign key with the table it refers to. For
each foreign key, work out both ends: a non-null foreign key means exactly
one at the referenced end, a nullable one means at most one; the other end
is any number unless the key is unique. Keep the `path:line` of each
definition as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Write each relationship from
the referenced table to the one holding the foreign key, so parents sit
above children. Give the diagram a title that states its point ("A sync
run records what it did to each work item, once per tracker"), not its
topic ("Sync schema"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure and settle it against the schema. Then look at the
picture in `shots/`. The check needs Chrome or Chromium; without one, say
the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, report every remark that turned
out to be true of the schema, and say what you left out and why.
