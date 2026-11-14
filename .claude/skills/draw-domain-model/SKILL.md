---
name: draw-domain-model
description: Draw a domain model in the terms of domain-driven design —
  aggregates as boundaries around their root, entities and value objects,
  with domain services, repositories, factories and domain events beside
  them, and the ownership, references by identity and dependencies between
  them — as an interactive page or SVG. Use when the user wants to see the
  domain a piece of code models, its aggregates and their boundaries, asks
  for a domain model, or wants to check that code respects aggregate
  boundaries, or when another skill needs a domain model as a figure.
argument-hint: "[what to draw, e.g. the retention domain]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a domain model

A domain model answers one question: **what does this code say the domain
is made of, and where are its boundaries?** Each aggregate is a dashed
boundary around the things that change together, entered only through its
root. Around the aggregates stand the services that act on them, the
repositories that load them and the events they publish.

It differs from a class diagram in what it is about: the domain's building
blocks and the rules between them, not every type and field. It remarks on
anything that breaks those rules.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/retention-domain.json` is a model at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a model

### 1. Settle what to draw

A domain model covers **one bounded context**, or one part of one. From
the request, decide which, and name it in the domain's own words. A model
reads at a glance up to about 4 aggregates and 14 elements; beyond that,
show fewer members of each aggregate or draw one aggregate at a time. Ask
only if the request names no part of the domain.

### 2. Read the domain from the code

Read it from the code, never from memory or from names:

- **aggregates**: what must stay consistent together, and so is loaded,
  changed and saved as one. The root is the type everything else in it is
  reached through;
- **entities** have an identity that outlasts their values; **value
  objects** are their values and nothing more;
- **services** hold behaviour that belongs to no one entity;
  **repositories** load and save whole aggregates; **factories** build
  them; **events** say that something happened;
- **relationships**, from the line that makes each: a field that owns
  another type, a field holding another aggregate's id, a parameter or a
  call.

Record what the code does, not what the design intends. If the code holds
another aggregate directly, write that: the model will remark on it.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Use the domain's words for
every name. Give the model a title that states its point ("A claim is part
of the cache root, and names its install only by identity"), not its topic
("Retention domain"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: each names a rule of aggregate design the model
breaks. Go back to the code and settle whether the spec is wrong or the
code really does it. Then look at the picture in `shots/`. The check needs
Chrome or Chromium; without one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the model shows in a sentence, report every remark that turned
out to be true of the code as a finding about its design, and say what you
left out and why.
