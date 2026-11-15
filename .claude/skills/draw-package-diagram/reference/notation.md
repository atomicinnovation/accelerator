# Package diagram: the notation

```jsonc
{
  "id": "launcher-packages",                  // unique on the page it is shown on
  "kind": "package-diagram",
  "title": "Only outbound touches the system, and nothing in it knows the launcher",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "packages": [{
    "id": "retention",                        // unique among all packages, at every depth
    "name": "launch::retention",              // optional: the name shown; defaults to the id
    "status": "new", "tone": "risk",          // optional
    "link": "src/launch/retention/mod.rs:1",  // optional
    "packages": [ /* optional: the packages it holds, to any depth */ ]
  }],
  "relationships": [{
    "from": "retention", "to": "ports",
    "kind": "import",                         // optional: "dependency" (default), "import", "access" or "merge"
    "label": "the Disk trait",                // optional: shown in place of the kind
    "status": "new", "tone": "risk",          // optional
    "link": "src/launch/retention/mod.rs:3"   // optional: an import that makes the dependency
  }]
}
```

## How it is drawn

| What | Drawn as |
|---|---|
| A package that holds others | A folder: a tab with its name, over a frame around what it holds |
| A package that holds nothing | A small closed folder with its name in the body |
| A `dependency` | A dashed arrow from the package that depends to the one it depends on |
| An `import` | The same, labelled `«import»`: the names become available, and are passed on to whoever imports this package |
| An `access` | The same, labelled `«access»`: the names become available here only |
| A `merge` | The same, labelled `«merge»`: this package extends the other's contents |

A package is drawn above the packages it depends on.

## Conventions

- Name a package as the code does, qualified when a short name would be
  ambiguous.
- Draw a dependency at the level it matters. If every package in `launch`
  uses `kernel`, one arrow from `launch` says so.
- Use plain `dependency` unless the kind matters to the point.
- Mark what the change under review did with `status`, and a dependency
  that breaks a layering rule with `tone: "risk"`.
- Keep to about 15 packages.

## What it checks

`problemsIn` reports a diagram with no packages; a package with no id or
an id used twice; a relationship with no `from` or `to`, naming a package
the diagram does not have, of an unknown kind, or written twice; and a
`status`, `tone` or `direction` outside its list.

It remarks on packages that depend on each other in a ring, and on a
package that depends on one it holds or on the one that holds it.

## What it leaves out

- **The classes and other elements inside a package**: use a class diagram.
- **Layering as a rule to check**: it cannot yet be told which package may
  not depend on which. Cycles are the only rule it checks.
- **Package templates**, **profiles** and **models** as kinds of package.
- **Containment drawn with the circled plus**, as an alternative to nesting.
