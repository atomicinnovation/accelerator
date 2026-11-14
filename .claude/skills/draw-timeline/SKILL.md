---
name: draw-timeline
description: Draw a timeline — an axis, linear or logarithmic, with
  labelled marks at moments and bars for the spans between them — as a page.
  Use when the user wants to see ages, windows, timeouts, retries and
  backoff, expiry, thresholds or how durations compare; asks for a
  timeline; or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. how long a claim protects a version]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a timeline

A timeline answers one question: **how do these moments and spans sit
against one another?** Values are placed on one axis in one unit. A mark
is a moment; a bar is a span; bars that would overlap sit in lanes.

Use a sequence diagram for the order of calls, and a state machine for
what moves something from one state to the next.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/claim-ages.json` is one at the standard expected. Read
both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill draws it.

## When the user asks for one

### 1. Settle what to draw

Decide the unit and the range. Use a logarithmic scale when the values
span more than about two orders of magnitude (a 60 second timeout beside a
14 day window). Ask only if the request names no duration or moment.

### 2. Read it from the code

Read every value from the code, never from memory: the constant, its
unit and where it is compared. Name each mark and span as the code names
it. Keep the `path:line` of the constant as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Give ticks the
labels a reader thinks in ("14 d", not 1209600). Give it a title that
states its point ("A claim protects its version for 14 days"), not its
topic, and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Then look
at the picture in `shots/`. The check needs Chrome or Chromium; without
one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the timeline shows in a sentence, with the values that matter.
