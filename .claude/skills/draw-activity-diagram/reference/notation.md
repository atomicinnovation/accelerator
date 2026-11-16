# Activity diagram: the notation

```jsonc
{
  "id": "cold-start",                         // unique on the page it is shown on
  "kind": "activity-diagram",
  "title": "A cold start fetches a version, then prunes while it records the fetch",
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional, without partitions: "down" (default) or "right"
  "partitions": [{                            // optional: one lane each, left to right
    "id": "store", "label": "Store",
    "status": "changed", "tone": "risk"       // optional
  }],
  "nodes": [{
    "id": "prune",                            // unique among the nodes
    "kind": "action",                         // optional: "action" (default), or one of the control nodes below
    "label": "Prune stale trees",             // needed for an action
    "partition": "store",                     // needed for every node when there are partitions
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/prune.rs:40"       // optional: where the step is
  }],
  "flows": [{
    "from": "cached", "to": "fetch",
    "guard": "else",                          // optional: drawn in square brackets
    "label": "the manifest",                  // optional: what is passed along
    "status": "new", "tone": "risk",          // optional
    "link": "src/resolve.rs:88"
  }]
}
```

Nodes are laid out in the order the flows give them, start at the top.

## Nodes

| Kind | Drawn as | Flows |
|---|---|---|
| `action` | A rounded box with what is done | Any number in and out |
| `initial` | A filled disc | At least one out, none in |
| `final` | A ringed disc: the whole activity ends | None out |
| `flow final` | A ring with a cross: this path ends, others carry on | None out |
| `decision` | A diamond | One or more in, two or more out, each with a guard |
| `merge` | A diamond | Two or more in, one out: alternative paths come back together |
| `fork` | A bar | One in, two or more out: all of them proceed at once |
| `join` | A bar | Two or more in, one out: it waits for all of them |

A decision chooses **one** flow out; a fork takes **all** of them. A merge
passes each arrival straight on; a join waits for every arrival. Using a
merge where a join is meant, or a decision where a fork is meant, changes
what the diagram says.

## Flows and partitions

- A flow is a solid arrow with an open head. Its guard is written in
  square brackets, and `else` is the guard for "otherwise".
- With `partitions`, the page is divided into lanes down its length, each
  headed with its label, and every node stands in its own partition's
  lane. A flow that crosses a lane boundary is a hand-over. Partitions are
  always drawn down the page.

## Conventions

- Name an action as a verb phrase, in the code's own terms: `Prune stale
  trees`, not `Pruning` or `prune.rs`.
- Give every flow out of a decision a guard; make them cover every case,
  using `else` for the last.
- Bring alternative paths back together with a merge before going on, not
  by pointing several flows at one action.
- Mark what the change under review did with `status`, and where the
  activity can go wrong with `tone: "risk"`.
- Keep to about 15 actions and 4 partitions.

## What it checks

`problemsIn` reports an activity with no nodes; a node with no id, an id
used twice or an unknown kind; an action with no label; a partition with
no id or an id used twice; a node in a partition the activity does not
have, or in none when it has partitions; a `direction` given with
partitions; a flow with no `from` or `to`, naming a node the activity does
not have, out of a final node or into the initial node; a guard or label
that is not text; an initial node with no flow out; a decision or fork
with fewer than two flows out; a merge or join with fewer than two flows
in; a merge or join without exactly one flow out, and a fork without
exactly one in; and a `status`, `tone` or `direction` outside its list.

It remarks on an activity with no initial node; an action with no flow
out; a node nothing flows to; and a decision with more than one flow out
that has no guard.

## What it leaves out

- **Object nodes and object flows**: what is passed along can be named on
  a flow, but not drawn as a node or a pin.
- **Signals**: accepting an event, sending a signal, and time events.
- **Interruptible regions**, **expansion regions**, **exception
  handlers** and **structured activity nodes**.
- **Partitions in two dimensions**, nested partitions, and partitions
  drawn across the page.
- **Calling another activity** (the rake symbol).
