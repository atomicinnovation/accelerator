# Entity-relationship diagram: the notation

```jsonc
{
  "id": "work-items-schema",                  // unique on the page it is shown on
  "kind": "entity-relationship-diagram",
  "title": "A sync run records what it did to each work item, once per tracker",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "entities": [{
    "id": "work_item",                        // unique among the entities
    "name": "work_item",                      // optional: the name shown; defaults to the id
    "status": "changed", "tone": "risk",      // optional
    "link": "migrations/0007_sync.sql:12",    // optional: where it is defined
    "attributes": [
      "title",                                // a name alone, or:
      { "name": "remote_key",
        "type": "text",                       // optional
        "key": "unique",                      // optional: "primary", "foreign", "unique", or a list of them
        "nullable": true,                     // optional: it may be empty
        "status": "new" }                     // optional: what the change did to this attribute
    ]
  }],
  "relationships": [{
    "from": "tracker", "to": "work_item",
    "label": "tracks",                        // optional: read from "from" to "to"
    "fromCardinality": "one",                 // how many of "from" for each of "to"
    "toCardinality": "zero-or-many",          // how many of "to" for each of "from"
    "identifying": false,                     // optional: false when the child's key does not include the parent's
    "status": "new", "tone": "risk",          // optional
    "link": "migrations/0007_sync.sql:20"     // optional: the foreign key that makes it
  }]
}
```

## How many

Each end of a relationship carries one of four symbols, read from the
other end: "each `tracker` tracks **zero or many** `work_item`s; each
`work_item` is tracked by **one** `tracker`".

| Cardinality | Symbol at that end | Means |
|---|---|---|
| `one` | Two bars | Exactly one |
| `zero-or-one` | A ring and a bar | At most one |
| `one-or-many` | A bar and a crow's foot | At least one |
| `zero-or-many` | A ring and a crow's foot | Any number |

The symbol nearest the entity says the most there can be (a bar for one, a
crow's foot for many), and the one beyond it says the least (a bar for
one, a ring for none).

## How it is drawn

| What | Drawn as |
|---|---|
| An entity | A box with its name over a rule, and a row for each attribute |
| An attribute | Its keys (`PK`, `FK`, `UK`), its name, and its type at the right. A type followed by `?` may be empty |
| An attribute the change touched | Its row in the colour of its `status` |
| A relationship | A line with a symbol at each end and its label beside it. A solid line is identifying: the child's primary key includes the parent's. A dashed line is not |

## Conventions

- Name entities and attributes as the schema does.
- Write a relationship from the referenced entity to the one holding the
  foreign key, so a parent is drawn above its children.
- List the attributes the point needs. For an entity at the edge of the
  diagram, its keys are enough.
- Mark what a migration does with `status`, on entities and on single
  attributes, and what it puts at risk (a column dropped, a constraint
  loosened) with `tone: "risk"`.
- Keep to about 10 entities.

## What it checks

`problemsIn` reports a diagram with no entities; an entity with no id or
an id used twice; `attributes` that are not a list; an attribute that is
neither a name nor an object with a name, a name used twice in one entity,
a key outside the list, a `type` that is not text, or a `nullable` that is
not true or false; a relationship with no `from` or `to`, naming an entity
the diagram does not have, missing either cardinality or giving one
outside the list, or written twice; an `identifying` that is not true or
false; and a `status`, `tone` or `direction` outside its list.

It remarks on an entity with no primary key; an entity related to nothing
when the diagram has relationships; and two entities related many to many,
which a database can only hold with a table between them.

## What it leaves out

- **Chen notation**: diamonds for relationships and ovals for attributes.
- **Checking a foreign key against its relationship**: nothing verifies
  that an attribute marked `FK` has a relationship to match, or that the
  cardinalities agree with `nullable` and `unique`.
- **Composite and derived attributes**, **weak entities** as a symbol of
  their own, **subtypes** and **n-ary relationships**.
- **Indexes**, **check constraints**, **defaults** and **triggers**.
