# Data flow diagram: the notation

```jsonc
{
  "id": "launcher-bootstrap",                 // unique on the page it is shown on
  "kind": "data-flow-diagram",
  "title": "Nothing fetched from the release host runs until its signature is checked",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "right",                       // optional: "right" (default) or "down"
  "externals": [{ "id": "releases", "name": "GitHub release host" }],
  "processes": [{
    "id": "verify", "name": "Verify the signature",   // an id unique in the diagram, and the name shown
    "status": "changed", "tone": "ok",        // optional
    "link": "src/verify.rs:40"               // optional
  }],
  "stores": [{ "id": "cache", "name": "Launcher cache" }],
  "boundaries": [{
    "id": "machine", "name": "The user's machine",
    "holds": ["verify", "cache"],             // elements and other boundaries inside it
    "tone": "risk"                            // optional, as are status and link
  }],
  "flows": [
    { "from": "releases", "to": "fetch",
      "data": "launcher and signature",       // what flows, as it is at that point
      "status": "new", "tone": "risk", "link": "src/fetch.rs:12" }
  ]
}
```

Externals, processes, stores and boundaries share one set of ids.

## How each is drawn

| Written in | Drawn as | Means |
|---|---|---|
| `processes` | A circle with its name inside | Something that reads, changes, checks or forwards data |
| `stores` | Its name between two parallel lines | Somewhere data rests |
| `externals` | A rectangle | A person or system outside what is under review, where data comes from or goes to |
| `flows` | A solid arrow with a filled head, labelled with its `data` | Data moving from one element to another |
| `boundaries` | A dashed frame around what it holds, named at its top left | A trust boundary: the level of trust differs between its inside and its outside |

This is the Yourdon and DeMarco notation, with the trust boundary added as
threat modelling draws it.

## Order

The diagram reads in the order the flows are written. A flow that returns
data to somewhere earlier flows have already left (a response to a request,
a result handed back to the caller) is drawn running back against the rest,
so a request and its response sit side by side.

## Conventions

- Name a process for what it does to the data: a verb and an object.
- Name a flow for the data as it is at that point: "unverified launcher"
  and "verified launcher" are different flows.
- Draw a boundary wherever trust changes, and nowhere else.
- Mark a crossing `tone: "ok"` when something checks or protects the data
  there, and `tone: "risk"` when nothing does.
- Mark what the change under review did with `status`.
- Keep to about 10 elements; follow one kind of data per diagram.

## What it checks

`problemsIn` reports a diagram with no processes; an element or boundary
with no id or name, or an id used twice; a boundary that holds nothing,
holds something the diagram does not have, holds something another
boundary already holds, or is inside itself; a flow with no `from` or
`to`, to or from something the diagram does not have or a boundary, back
to where it began, with no `data`, or written twice; and a `status`, `tone`
or `direction` outside its list.

It remarks on a flow with a process at neither end, since data moves only
through a process; a process that receives no data, or sends none on; an
element joined to nothing; and, in one remark, every flow that crosses a
trust boundary.

## What it leaves out

- **Levelling**: process numbers and a process opened into a diagram of its
  own. Draw a second diagram.
- **Two-headed flows**: write the two flows, each named for its data.
- **Gane and Sarson shapes**.
- **Control flows** and timing: use an activity or sequence diagram.
