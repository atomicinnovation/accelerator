# Component diagram: the notation

```jsonc
{
  "id": "launcher-components",                // unique on the page it is shown on
  "kind": "component-diagram",
  "title": "Retention needs a disk and a clock, and does not care which",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",                       // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "components": [{
    "id": "retention",                        // unique among components and interfaces
    "name": "Retention",                      // optional: the name shown; defaults to the id
    "stereotype": "subsystem",                // optional: shown as «subsystem»; defaults to «component»
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/mod.rs:1",         // optional
    "components": [ /* optional: the components it holds, to any depth */ ]
  }],
  "interfaces": [{
    "id": "Disk", "name": "Disk",             // unique among components and interfaces
    "status": "new", "tone": "risk", "link": "src/ports.rs:12"
  }],
  "relationships": [{
    "from": "retention", "to": "Disk",
    "kind": "requires",                       // "provides", "requires" or "dependency"
    "label": "to remove trees",               // optional
    "status": "new", "tone": "risk", "link": "src/retention/prune.rs:20"
  }]
}
```

## Relationships

| Kind | Write it `from` → `to` | Drawn as | Means |
|---|---|---|---|
| `provides` | component → interface | A plain line from the component to the interface's ball | The component implements the interface |
| `requires` | component → interface | A dashed arrow from the component to the ball | The component uses the interface, whoever provides it |
| `dependency` | component → component | A dashed arrow | The component names the other directly |

A component that requires an interface is drawn above it, and the
component that provides it below, so use reads down the page.

## How it is drawn

| What | Drawn as |
|---|---|
| A component | A box marked `«component»`, or with its own stereotype, over its name |
| A component that holds others | A frame, marked and named the same way, around them |
| An interface | A small ball with its name beside it |

A required interface is drawn as a dependency on the interface's ball. UML
also allows a socket (a half circle) on the component; the two say the
same, and the arrow keeps a diagram with many users of one interface
legible.

## Conventions

- Name components and interfaces as the code does.
- Show an interface only where it separates a user from a provider that
  could be replaced. Leave out interfaces with one user and one provider
  that will never change, and draw a dependency.
- Mark what the change under review did with `status`, and a coupling that
  should not be there with `tone: "risk"`.
- Keep to about 12 components and 8 interfaces.

## What it checks

`problemsIn` reports a diagram with no components; a component or
interface with no id, or an id used twice among them; a `stereotype` that
is not text; a relationship with no `from`, `to` or `kind`, naming nothing
the diagram has, of an unknown kind, or written twice; a `provides` or
`requires` that does not run from a component to an interface; a
`dependency` that does not run between components; and a `status`, `tone`
or `direction` outside its list.

It remarks on an interface that is required but that nothing in the
diagram provides; an interface neither provided nor required; a component
joined to nothing; and components that need each other in a ring, whether
directly or through an interface one requires and another provides.

## What it leaves out

- **Ports**, and **delegation connectors** from a component's port to a
  part inside it.
- **The socket** for a required interface, and the ball-and-socket
  assembly connector: see above.
- **Realisation of a component by classes**, and **artefacts** that
  manifest a component: use a deployment diagram for those.
- **Interfaces drawn as boxes with their operations**: use a class diagram.
