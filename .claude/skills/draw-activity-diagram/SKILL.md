---
name: draw-activity-diagram
description: Draw a UML activity diagram — actions joined by control flows,
  with decisions and merges, forks and joins for what happens in parallel,
  and partitions (swimlanes) for who does what — as an interactive page or
  SVG. Use when the user wants to see a workflow, a process, an algorithm's
  steps, what runs in parallel, or which component is responsible for each
  step, asks for an activity diagram or a swimlane diagram, or when another
  skill needs one as a figure.
argument-hint: "[what to draw, e.g. what a cold start does]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw an activity diagram

An activity diagram answers one question: **what is done, in what order,
and by whom?** Each action is a rounded box, and control flows from one to
the next: through a decision where the path depends on a condition, through
a fork where several things proceed at once and a join where they must all
finish. Partitions divide the page into lanes, one per responsible party,
so every action sits in the lane of whoever does it.

Use a sequence diagram instead when the question is who calls whom, and a
state machine when it is what state one thing is in.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/cold-start.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

An activity diagram shows **one activity from start to finish**. From the
request, decide which, where it starts and ends, and whether who does each
step matters: if it does, use partitions. A diagram reads at a glance up to
about 15 actions and 4 partitions; beyond that, make an action of several
steps or draw part of the activity. Ask only if the request names no
activity.

### 2. Read the activity from the code

Follow the code, never memory or names. Note each step, each condition that
changes the path (and both of its outcomes), what is started to run
alongside something else and where the code waits for it, and where the
activity can end. Keep the `path:line` of each step as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Name each action for what it
does, as a verb phrase. Give every flow out of a decision a guard. Give the
diagram a title that states its point ("A cold start fetches a version,
then prunes while it records the fetch"), not its topic ("Cold start"), and
a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: an action that cannot be reached or has no flow
out, or a decision with nothing to decide by, is either a mistake in the
spec or a finding about the code. Settle which against the code. Then look
at the picture in `shots/`. The check needs Chrome or Chromium; without
one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the activity shows in a sentence, report every remark that turned
out to be true of the code, and say what you left out and why.
