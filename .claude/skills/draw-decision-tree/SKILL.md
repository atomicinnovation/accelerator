---
name: draw-decision-tree
description: Draw a decision tree — questions, the answers that branch from
  each, and the outcome every path ends in — as an interactive page or SVG.
  Use when the user wants to see how a decision is reached, which conditions
  lead to which result, every case a piece of logic distinguishes, or asks
  for a decision tree, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. how prune decides whether to keep a tree]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a decision tree

A decision tree answers one question: **which conditions lead to which
outcome?** Each question is a box, each answer is an arrow down from it, and
every path ends in an outcome. It suits logic that asks several questions in
turn, where what matters is every case and where each one ends up.

Use a flowchart instead when paths join again or loop, and a table when two
or three independent conditions are simply combined.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/keep-or-remove.json` is a tree at the standard
expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a tree

### 1. Settle what to draw

A decision tree shows **one decision**: one function's verdict, one policy,
one routing choice. From the request, decide which, and what its outcomes
are. A tree reads at a glance up to about 6 questions and 10 outcomes;
beyond that, draw one subtree at a time. Ask only if the request names no
decision.

### 2. Read the decision from the code

Follow the code, never memory or names. Take the conditions in the order
the code tests them, since an early return makes the order part of the
logic. For each, note every answer the code distinguishes and what happens
next. Follow every path to its end, including the error and default paths:
those are the outcomes most often missed. Keep the `path:line` of each
condition as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Word each question so that its
answers are short (`yes`, `no`, a value). Give the outcome that matters
most `tone: "risk"` and the safe ones `tone: "ok"`. Give the tree a title
that states its point ("Every doubt about a tree falls on the side of
keeping it"), not its topic ("Prune decision"), and a caption that says
what to notice.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

The render must exit 0 and the check must report `"ok": true`. Read every
remark under the figure: a question with one answer, or whose answers all
lead to the same outcome, is either a mistake in the spec or a condition
in the code that does nothing. Settle which against the code. Then look at
the picture in `shots/`. The check needs Chrome or Chromium; without one,
say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the tree shows in a sentence, report every remark that turned out
to be true of the code, and say what you left out and why.
