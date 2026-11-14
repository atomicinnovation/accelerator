---
name: draw-data-flow-diagram
description: Draw a data flow diagram — the processes that handle data, the
  stores that keep it, the external entities it comes from and goes to, the
  named flows between them, and the trust boundaries those flows cross — as
  an interactive page or SVG. Use when the user wants to see where data
  goes, what handles it and where it is kept, wants a security or threat
  model review of a change, asks for a data flow diagram or DFD, or when
  another skill needs one as a figure.
argument-hint: "[what to draw, e.g. where a credential goes during sign-in]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a data flow diagram

A data flow diagram answers one question: **where does this data go, and
what does it cross on the way?** Processes are circles, stores are open
boxes, and the people and systems outside are rectangles. Every arrow is
named for the data it carries. A dashed frame is a trust boundary: what is
inside it is trusted in a way that what is outside is not, so every flow
across one is where data must be checked, authorised or protected.

Use a sequence diagram for the order of calls in one scenario, an activity
diagram or flowchart for the steps of a procedure, and a deployment diagram
for what runs where.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/launcher-bootstrap.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which data to follow: a credential, a request, a
file, a record. One diagram follows one kind of data well. A diagram reads
at a glance up to about 10 elements. Ask only if the request names no data
and no part of the system.

### 2. Read the flows from the code

Read them from the code, never from memory or from names. Follow the data
from where it enters:

- **external entities**: who or what hands the data over or receives it,
  and is outside what is under review;
- **processes**: each piece of code that reads, changes, checks or forwards
  the data, named for what it does to it, as a verb and an object;
- **stores**: every place the data rests: a file, a table, a cache, a
  queue, an environment variable;
- **flows**: each hand-over, named for the data as it is at that point
  ("unverified launcher", not "data");
- **trust boundaries**: where the level of trust changes: the edge of the
  machine, the process, the network, the privileged part.

Keep the `path:line` that shows each as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Write the flows in the order
the data moves: that order is the order the diagram reads in. Give the
diagram a title that states its point ("Nothing fetched from the release
host runs until its signature is checked"), not its topic ("Data flow"),
and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure. One lists the flows that cross a trust boundary:
for each, find in the code what checks or protects the data there, and mark
the flow `tone: "ok"` where something does and `tone: "risk"` where nothing
does. Any other remark is a rule of the notation broken: either a mistake
in the spec or a finding. Settle which against the code. Then look at the
picture in `shots/`. The check needs Chrome or Chromium; without one, say
the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the diagram shows in a sentence, name each boundary crossing and
what guards it, and say what you left out and why.
