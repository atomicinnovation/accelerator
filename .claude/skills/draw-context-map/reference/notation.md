# Context map: the notation

```jsonc
{
  "id": "tracker-contexts",                   // unique on the page it is shown on
  "kind": "context-map",
  "title": "Work items keep their own model, with a layer in front of each tracker",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "contexts": [{
    "id": "work", "name": "Work items",       // an id unique on the map, and the name shown
    "subdomain": "core",                      // optional: "core", "supporting" or "generic"
    "team": "Accelerator",                    // optional: who owns it
    "description": "What is to be done, its state and how items relate.",   // optional
    "external": false,                        // optional: true for a context that is not ours
    "status": "changed", "tone": "risk",      // optional
    "link": "cli/work/src/lib.rs:1"           // optional
  }],
  "relationships": [
    { "upstream": "jira", "downstream": "work",          // one sets the terms and the other follows
      "upstreamPatterns": ["open host service", "published language"],   // optional
      "downstreamPatterns": ["anticorruption layer"],                    // optional: or ["conformist"]
      "kind": "customer-supplier",            // optional: the downstream's needs shape the upstream's plans
      "label": "issues",                      // optional: what passes between them
      "status": "new", "tone": "risk", "link": "cli/jira-client/src/lib.rs:1" },
    { "between": ["work", "kernel"],          // equals: neither sets the terms
      "kind": "shared kernel",                // "shared kernel" or "partnership"
      "label": "identifiers" }
  ]
}
```

## Relationships

| Written as | Drawn as | Means |
|---|---|---|
| `upstream` and `downstream` | A line with `U` at the upstream end and `D` at the downstream end, the upstream drawn above | A change to the upstream's model reaches the downstream, never the reverse |
| `upstreamPatterns: ["open host service"]` | `OHS` beside the `U` | The upstream offers a defined protocol for anyone to use |
| `upstreamPatterns: ["published language"]` | `PL` beside the `U` | The upstream documents the model it exchanges |
| `downstreamPatterns: ["anticorruption layer"]` | `ACL` beside the `D` | The downstream translates the upstream's model into its own |
| `downstreamPatterns: ["conformist"]` | `CF` beside the `D` | The downstream uses the upstream's model as it is |
| `kind: "customer-supplier"` | *customer/supplier* on the line | The upstream plans with the downstream's needs in mind |
| `between`, `kind: "shared kernel"` | A line labelled *shared kernel* | The two share part of a model, and change it only together |
| `between`, `kind: "partnership"` | A line labelled *partnership* | The two succeed or fail together, and co-ordinate their plans |

Every abbreviation a map uses is spelled out under the figure.

## How a context is drawn

A box with its name; in brackets its subdomain (`Core`, `Supporting`,
`Generic`) or `External`, and its team; and what it is for. An external
context is greyed.

## Conventions

- Name a context for the model that holds in it, in the domain's words.
- Mark as `external` a context whose model you cannot change.
- Say how every downstream of an external context takes its model. That
  choice is the one most worth a reviewer's attention.
- Mark what the change under review did with `status`.
- Keep to about 8 contexts.

## What it checks

`problemsIn` reports a map with no contexts; a context with no id or
name, or an id used twice; a `subdomain` outside its list; an `external`
that is not true or false; a `team` or `description` that is not text; a
relationship that has neither an upstream and a downstream nor two
contexts it is between, or has both; a relationship naming a context the
map does not have, or relating a context to itself; a `kind` or pattern
outside the list for its side, or given to the wrong kind of relationship;
a downstream that both conforms and keeps an anticorruption layer; a
conformist that is also a customer; a relationship written twice; and a
`status`, `tone` or `direction` outside its list.

It remarks on a context related to nothing; contexts each upstream of the
other, directly or round a ring; a context downstream of an external one
that says neither that it conforms nor that it keeps an anticorruption
layer; and a core context that conforms to another's model.

## What it leaves out

- **Separate ways**: two contexts with no relationship are simply not
  joined.
- **Big ball of mud** as a kind of context.
- **Team topologies** beyond naming a context's team.
- **What crosses each relationship in detail**: use a sequence diagram or
  a domain model.
