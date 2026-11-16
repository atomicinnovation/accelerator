---
name: draw-railroad-diagram
description: Draw a railroad (syntax) diagram — one track per grammar rule,
  with terminals and references to other rules joined by rail, branches for
  choices, bypasses for optional parts and loops for repeats — as an
  interactive page or SVG. Use when the user wants to see what a grammar, a
  parser, a command line, a configuration syntax, a version or identifier
  format or a regular expression accepts, asks for a railroad or syntax
  diagram, or when another skill needs one as a figure.
argument-hint: "[what to draw, e.g. the syntax of a version requirement]"
allowed-tools:
  - Bash(accelerator config *)
---

# Draw a railroad diagram

A railroad diagram answers one question: **what does this syntax
accept?** Each rule is a track read from left to right. Any route along
the rails from the bars at one end to the bars at the other is something
the rule accepts. A pill is text that appears as written; a rectangle is
another rule.

Use a state machine for what a parser does as it reads, a byte-field
diagram for a binary layout, and a decision tree for how a value is
classified once parsed.

**Skill directory**: `${CLAUDE_SKILL_DIR}`
**Kit directory**: `${CLAUDE_SKILL_DIR}/../diagram-kit`
**Tmp directory**: !`accelerator config path tmp --fail-safe`

Wherever `{skill dir}`, `{kit}` or `{tmp directory}` appears below, use the
path shown above.

The spec is described in `{skill dir}/reference/notation.md`, and
`{skill dir}/examples/version-requirement.json` is a diagram at the
standard expected. Read both before writing one.

## When another skill wants the figure

Write the spec as that skill asks, and check it:

```bash
node {kit}/bin/diagram.js check {spec file}
```

Fix every warning at its source. The other skill lays it out and draws it.

## When the user asks for a diagram

### 1. Settle what to draw

From the request, decide which syntax, and how far down to go: stop at
rules a reader already knows (`digit`, `letter`, `identifier`) and leave
them as references. A diagram reads well up to about 6 rules. Ask only if
the request names no syntax.

### 2. Read the grammar from the code

Read it from the parser, the grammar file or the regular expression, never
from memory or from the documentation alone: the point is what the code
accepts. For each rule, find what must appear, what may be left out, what
may repeat and what separates the repeats, and where there is a choice.
Keep the `path:line` that parses each part as its `link`.

### 3. Write the spec

Write `{tmp directory}/diagrams/{id}.json`. Put the rule the reader starts
from first, and each rule it uses after it. Give the diagram a title that
states its point ("A comparator may leave out its operator and the end of
its version"), not its topic ("Version grammar"), and a caption with an
example of what is accepted.

### 4. Draw and check it

```bash
node {kit}/bin/diagram.js render {tmp directory}/diagrams/{id}.json --out {tmp directory}/diagrams/{id}.html
node {kit}/tools/check-page.js {tmp directory}/diagrams/{id}.html --shots {tmp directory}/diagrams/shots
```

Add `--legend` to the render to put under the figure, closed until
opened, what each symbol of this notation means.

The render must exit 0 and the check must report `"ok": true`. Read any
remark under the figure: it is either a mistake in the spec or a finding.
Then look at the picture in `shots/`, and follow the rails for two or
three inputs the code accepts and one it rejects. The check needs Chrome
or Chromium; without one, say the page was not checked.

### 5. Deliver

Give the page's absolute path. It opens by double-click and works offline.
Say what the syntax accepts in a sentence, and say which rules you left as
references and why.
