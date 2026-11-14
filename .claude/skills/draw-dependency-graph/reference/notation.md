# Dependency graph: the notation

```jsonc
{
  "id": "launcher-retention",                 // unique on the page it is shown on
  "kind": "dependency-graph",
  "title": "Retention reaches disk and time only through ports",  // the point, not the topic
  "caption": "What to notice, in a sentence or two.",             // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "modules": [{
    "id": "retention",                        // unique among all modules, at every depth
    "label": "retention",                     // optional: the name shown; defaults to the id
    "detail": "what it is for, in a few words",   // optional, on a module that holds no others
    "status": "new",                          // optional: new, changed, removed or existing
    "tone": "risk",                           // optional: risk or ok
    "link": "src/retention/mod.rs:16",        // optional: where it is defined
    "modules": [ /* optional: the modules it holds, to any depth */ ]
  }],
  "dependencies": [{
    "from": "prune",                          // the module that depends
    "to": "disk",                             // the module it depends on
    "kind": "runtime",                        // optional: tells two dependencies between the same modules apart
    "label": "unlink",                        // optional: shown on the arrow in place of the kind
    "status": "new", "tone": "risk",          // optional, as for a module
    "link": "src/retention/prune.rs:540"      // optional: the line that makes the dependency
  }]
}
```

## How it is drawn

- A module that holds no others is a box; one that holds others is a titled
  frame around them.
- A dependency is a UML dependency: a dashed line with an open arrowhead,
  pointing **from the dependent to what it depends on**. So an arrow from
  `prune` to `disk` reads "`prune` needs `disk`".
- An arrow may start or end on a module that holds others, when the whole of
  it depends or is depended on.
- Dependencies that form a cycle are drawn as a risk, unless they are given
  a tone of their own, and each cycle is listed under the figure by the
  modules it runs through.

## Conventions

- Use the code's own names for modules.
- Order modules as a reader would meet them: what calls first, what is
  called after. The layout keeps dependents above what they depend on
  (or to the left, with `"direction": "right"`).
- Mark what the change under review did with `status`. Leave it off what
  the change did not touch, so that what is marked stands out.
- Keep to about 25 boxes. Group, coarsen or split anything larger.

## What it checks

`problemsIn` reports a graph with no modules; a module with no id or an id
used twice; a dependency with no `from` or `to`, or naming a module the
graph does not have; the same dependency written twice; and a `status`,
`tone` or `direction` outside its list.

## What it leaves out

- **Weights and counts** on arrows (how many imports make a dependency).
- **Layers as a constraint**: it cannot yet be told "nothing in `core` may
  depend on `adapters`" and report violations. Cycles are the only rule it
  checks.
- **Other UML relationships**. Generalisation, realisation and association
  belong to a class or component diagram, not here.
