# Domain model: the notation

```jsonc
{
  "id": "retention-domain",                   // unique on the page it is shown on
  "kind": "domain-model",
  "title": "A claim is part of the cache root, and names its install only by identity",
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "aggregates": [{
    "id": "cache-root",                       // unique among everything in the model
    "name": "CacheRoot",                      // optional: the name shown; defaults to the id
    "status": "changed", "tone": "risk",      // optional
    "link": "src/root.rs:1",                  // optional
    "members": [{
      "id": "VersionClaim",                   // unique among everything in the model
      "name": "VersionClaim",                 // optional
      "kind": "entity",                       // "aggregate root", "entity" or "value object"
      "attributes": ["+ version: Version"],   // optional: one line each
      "operations": ["+ refresh(now)"],       // optional
      "status": "new", "tone": "risk", "link": "src/claims.rs:24"   // optional
    }]
  }],
  "others": [{                                // what stands outside every aggregate
    "id": "Retention",
    "kind": "domain service",                 // "domain service", "repository", "factory", "domain event",
                                              // or an "entity" or "value object" no aggregate holds
    "attributes": [], "operations": ["+ keep(tree, claims, now): Verdict"],
    "status": "new", "tone": "risk", "link": "src/retention.rs:10"
  }],
  "relationships": [{
    "from": "CacheRoot", "to": "VersionClaim",   // ids of elements, or of an aggregate as a whole
    "kind": "composition",                    // see the table below
    "label": "holds",                         // optional
    "fromMultiplicity": "1", "toMultiplicity": "0..*",   // optional, on a composition or association
    "fromRole": "owner", "toRole": "claims",  // optional, likewise
    "navigable": true,                        // optional, likewise
    "status": "new", "tone": "risk", "link": "src/root.rs:30"
  }]
}
```

Every aggregate has exactly one member of kind `aggregate root`.

## Relationships

| Kind | Write it `from` → `to` | Drawn as | Use it when |
|---|---|---|---|
| `composition` | owner → owned | A solid line with a filled diamond at the owner | A root or entity owns an entity or value object inside the same aggregate |
| `reference` | holder → aggregate | A dashed arrow labelled *by identity* | One aggregate names another by its id, not by holding it |
| `association` | holder → held | A solid line; an open arrowhead if `navigable` | Something outside an aggregate holds a reference to its root, as a repository does |
| `dependency` | user → used | A dashed arrow | A service acts on an aggregate; a root publishes an event |
| `generalisation` | specific → general | A solid line to a hollow triangle | One element is a kind of another |

## How it is drawn

| What | Drawn as |
|---|---|
| An aggregate | A dashed boundary, marked `«aggregate»` and named, around its members |
| A member or other element | A class box whose stereotype is its building block: `«aggregate root»`, `«entity»`, `«value object»`, `«domain service»`, `«repository»`, `«factory»`, `«domain event»` |
| Attributes and operations | Compartments, as in a class diagram |

## What it remarks on

Each remark is a rule of aggregate design that the model, as written, breaks:

- **Reaching past a root**: a relationship from outside an aggregate to a
  member that is not its root. Everything outside an aggregate should go
  through the root, or the root cannot keep the aggregate consistent.
- **Holding another aggregate**: a composition or association from inside
  one aggregate to another. Aggregates should refer to each other by
  identity, so each can be loaded, changed and saved alone.
- **A value object that owns an entity**: a value object is its values, so
  it cannot own something with an identity of its own.

A remark is a finding about the code if the code really does it, and a
mistake in the spec if it does not.

## Conventions

- Use the domain's own words, as the code spells them.
- Show the members and operations that carry the domain's rules. Leave
  out plumbing: getters, serialisation, constructors.
- Mark what the change under review did with `status`, and the boundary it
  strains with `tone: "risk"`.
- Keep to about 4 aggregates and 14 elements.

## What it checks

`problemsIn` reports a model with nothing in it; an aggregate with no id,
no members, or not exactly one root; any id used twice across aggregates,
members and other elements; an element with no kind, or a kind that does
not belong where it is; `attributes` or `operations` that are not lists of
text; a relationship with no `from`, `to` or `kind`, naming nothing the
model has, of an unknown kind, or written twice; a multiplicity, role or
`navigable` on a relationship that is not a composition or association; a
multiplicity not written as a number, a range or `*`; and a `status`,
`tone` or `direction` outside its list.

## What it leaves out

- **Bounded contexts and the relationships between them**: that is a
  context map.
- **Aggregates nested in aggregates**, and **modules** around them.
- **Invariants** written on an aggregate, and **specifications** and
  **policies** as building blocks of their own.
- **Realisation**: an element implementing an interface belongs to a class
  diagram.
