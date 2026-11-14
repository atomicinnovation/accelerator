---
name: draw-diagram
description: Choose the kind of diagram that answers a question and draw it
  — class, object, component, package, deployment, sequence, state, activity,
  entity-relationship, data flow, C4, domain model, context map, dependency
  graph, flowchart, decision tree, byte-field, railroad or plain block
  diagram — as an interactive
  page or SVG. Use when the user asks for a diagram, a picture or a
  visualisation of some code or design without naming its kind, is unsure
  which diagram fits, wants several views of one subject, or when another
  skill needs to decide which figures to draw.
argument-hint: "[what to draw, e.g. how sign-in works]"
---

# Draw a diagram

A diagram is worth drawing when it answers a question the reader has
faster than prose would. This skill settles what that question is, chooses
the kind of diagram that answers it, and hands over to the skill that
draws that kind. It draws nothing itself.

**Skill directory**: `${CLAUDE_SKILL_DIR}`

The guide to choosing is `${CLAUDE_SKILL_DIR}/reference/choosing.md`. Read
it before choosing. Each kind is drawn by the skill beside this one that
the guide names, at `${CLAUDE_SKILL_DIR}/../draw-<kind>`.

## When another skill wants to know which figures to draw

Read the guide and choose by it. That skill says how the figures are
written and where they go.

## When the user asks for a diagram

### 1. Settle the question

Say, in one sentence, what the reader wants to know: "which module may
depend on which", "what happens to a claim between start-up and prune",
"where the token goes". The request usually gives it. Where it gives only
a subject ("diagram the launcher"), look at the code first and pick the
question most worth answering about it: what is new, what is hard to hold
in the head, or what would be dangerous to get wrong. Ask only if the
request names no part of the system.

### 2. Choose the kind

Find the question in the guide. Where two kinds seem to fit, the guide's
"Telling neighbours apart" says which. Where the subject needs more than
one view, choose one kind per question, and no more than three diagrams
unless the user asks for more.

Tell the user the choice in a line, with the reason: "A sequence diagram:
you asked about the order of calls in one scenario."

### 3. Hand over

Invoke the skill for the chosen kind and follow it to the end: it reads
the code, writes the spec, draws the page, checks it and delivers it. For
several diagrams, do each in turn.

### 4. Deliver

Give each page's absolute path, say in a sentence what each shows, and say
which question you left undrawn and why.
