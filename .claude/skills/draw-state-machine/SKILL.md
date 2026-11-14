---
name: draw-state-machine
description: Draw a UML state machine diagram — states with their entry, do
  and exit activities, the transitions between them labelled event [guard] /
  action, initial and final states, choices, and composite states holding
  their own — as an interactive page or SVG. Use when the user wants to see a
  lifecycle, the statuses something moves through, what can happen in each
  state, or asks for a state diagram or state machine, or when another skill
  needs one as a figure.
argument-hint: "[what to draw, e.g. the lifecycle of a version claim]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a state machine

A state machine answers one question: **what states can this be in, and
what moves it from one to another?** Each state is a rounded box, and each
transition is an arrow labelled with the event that causes it, the guard
that must hold, and the action taken. It suits one thing with a lifecycle:
a record, a connection, a job, a lock, a file on disk.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/claim-lifecycle.json` is a diagram at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

A state machine describes **one thing**. From the request, decide which
thing, and whether its states are explicit in the code (an enum, a status
field, a typestate) or implicit (a file that exists or not, a lock held or
free, a timestamp older or younger than a window). A diagram reads at a
glance up to about 10 states; beyond that, group related states into a
composite state or draw two machines. Ask only if the request names
nothing with a lifecycle.

### 2. Read the machine from the code

Read it from the code, never from memory or from names:

- **states**: every value the status can take, or every condition the code
  distinguishes;
- **transitions**: every place a state is assigned or a condition is made
  true or false, with what triggers it (a call, a message, a timeout), the
  condition checked first, and what else is done there;
- **where it starts and ends**: how the thing is created, and how it stops
  existing.

Keep the `path:line` of each transition as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. List the states in the order
the thing passes through them, initial state first: the diagram is laid
out in that order. Give it a title that states its point ("A claim
protects its version until it has gone unrefreshed for the whole window"),
not its topic ("Claim states"), and a caption that says what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: a state that cannot be reached or has no way out,
a choice with nothing to choose by, or two transitions on one event with
no guard between them is either a mistake in the spec or a finding about
the code. Go back to the code and settle which. Then look at the picture
in `shots/`. The check needs Chrome or Chromium; without one, say the page
was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the machine shows in a sentence, report every remark that turned
out to be true of the code, and say what you left out and why.
