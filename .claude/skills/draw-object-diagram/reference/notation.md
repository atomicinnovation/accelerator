# Object diagram: the notation

```jsonc
{
  "id": "claims-snapshot",                    // unique on the page it is shown on
  "kind": "object-diagram",
  "title": "A stale claim still protects its version until the next prune",   // what the case shows
  "caption": "Which values to look at, and why.",                              // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "objects": [{
    "id": "stale",                            // unique among the objects
    "name": "stale",                          // optional: the instance's name
    "class": "VersionClaim",                  // optional: its type
    "slots": ["version = \"1.24.0\"", "mtime = 21 d ago"],   // optional: one value per line
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:24"   // optional: where its type is defined
  }],
  "links": [{
    "from": "claims", "to": "stale",
    "label": "holds",                         // optional: the association this link is an instance of
    "fromRole": "owner", "toRole": "claims",  // optional: what each end is called at the other
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:20"
  }]
}
```

## How it is drawn

| What | Drawn as |
|---|---|
| An object | A box whose heading is underlined: `name: Class`, `: Class` for an instance with no name, or `name` alone |
| Its slots | Under a rule, one `attribute = value` per line. An object with no slots is the heading alone |
| A link | A plain solid line between two objects, with its name beside it and a role at either end. A link has no arrowhead and no multiplicity: it joins exactly these two instances |

## Conventions

- Show one case at one moment. Two moments are two diagrams, side by side
  in a comparison.
- Write values as the code or the data would show them: quoted strings,
  units on durations and sizes (`14 d`, `41 MiB`).
- Show the slots the case turns on. An instance with every field listed
  hides the two that matter.
- Mark what the case is about with `tone: "risk"`, and what the change
  under review added with `status`.
- Keep to about 10 objects.

## What it checks

`problemsIn` reports a diagram with no objects; an object with no id or an
id used twice; a `name` or `class` that is not text; `slots` that are not a
list, or hold something that is neither text nor an object with text; a
link with no `from` or `to`, naming an object the diagram does not have,
or written twice; and a `status`, `tone` or `direction` outside its list.

It remarks on an object linked to nothing when the diagram has links.

## What it leaves out

- **Checking against a class diagram**: nothing verifies that a slot is an
  attribute of the object's class or that a link is allowed between them.
- **Nested objects** drawn inside the object that owns them: show the
  owning as a link.
- **Object states** in square brackets after the name, and **multi-objects**.
