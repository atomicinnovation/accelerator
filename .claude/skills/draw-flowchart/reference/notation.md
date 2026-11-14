# Flowchart: the notation

```jsonc
{
  "id": "prune-one-tree",                     // unique on the page it is shown on
  "kind": "flowchart",
  "title": "A tree is removed only when it is old and no claim covers it",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",                        // optional
  "direction": "down",                        // optional, without lanes: "down" (default) or "right"
  "lanes": [{ "id": "disk", "label": "Disk" }],   // optional: one lane each, left to right
  "steps": [{
    "id": "claimed",                          // unique among the steps
    "kind": "decision",                       // optional: "process" (default), or one of the kinds below
    "label": "Any claim inside the claim window?",
    "lane": "disk",                           // needed for every step when there are lanes
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/prune.rs:60"       // optional: where the step is
  }],
  "flows": [{
    "from": "claimed", "to": "remove",
    "label": "no",                            // the answer, on a flow out of a decision
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/prune.rs:64"
  }]
}
```

## Steps

| Kind | Symbol | Use it for |
|---|---|---|
| `start`, `end` | A box with rounded ends (terminator) | Where the procedure begins and where it stops. A flowchart may have several ends |
| `process` | A rectangle | Something done |
| `decision` | A diamond with the question inside | A condition. Two or more flows leave it, each labelled with its answer |
| `input`, `output` | A parallelogram | Data read or written |
| `subprocess` | A rectangle with a second line inside each side (predefined process) | A step that is a procedure of its own, defined elsewhere |

A flow is a solid line with a filled arrowhead. A flow may go back to an
earlier step to make a loop, and several flows may arrive at one step.

With `lanes`, the page is divided into lanes down its length, each headed
with its label, and every step stands in its lane. Lanes are always drawn
down the page.

## Conventions

- Word a process as a verb phrase and a decision as a question.
- Put an answer on every flow out of a decision, and make the answers
  different.
- Flow down the page; let only loops go back up.
- Mark what the change under review did with `status`, and the step that
  cannot be undone with `tone: "risk"`.
- Keep to about 12 steps.

## What it checks

`problemsIn` reports a flowchart with no steps; a step with no id, an id
used twice, an unknown kind or no label; a lane with no id or an id used
twice; a step in a lane the flowchart does not have, or in none when it
has lanes; a `direction` given with lanes; a flow with no `from` or `to`,
naming a step the flowchart does not have, out of an end or into a start;
a label that is not text; a start with no flow out; a decision with fewer
than two flows out; and a `status`, `tone` or `direction` outside its list.

It remarks on a flowchart with no start; a step that is not an end and has
no flow out; a step nothing flows to; and a decision with a flow out that
has no answer on it, or with two flows out carrying the same answer.

## What it leaves out

- **The rest of ISO 5807**: document, manual input, manual operation,
  preparation, stored data, display, and the on-page and off-page
  connectors.
- **Parallel mode** (the double line): use an activity diagram's fork and
  join.
- **Annotations** attached to a step.
