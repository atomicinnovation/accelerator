# C4 diagram: the notation

```jsonc
{
  "id": "accelerator-containers",             // unique on the page it is shown on
  "kind": "c4-diagram",
  "title": "Only the launcher touches the cache, and only it decides what is kept",   // the point
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "people": [{
    "id": "developer", "name": "Developer",   // an id unique in the diagram, and the name shown
    "description": "Works in a repository with the plugin installed.",   // optional, but say it
    "external": false,                        // optional: true for someone outside the organisation
    "status": "new", "tone": "risk", "link": "README.md:12"   // optional
  }],
  "systems": [{
    "id": "accelerator", "name": "Accelerator",
    "description": "A Claude Code plugin for a research, plan, implement workflow.",
    "external": false,                        // optional: true for a system that is not yours
    "status": "changed", "tone": "risk", "link": "README.md:1",
    "containers": [{                          // optional: give these to open the system up
      "id": "launcher", "name": "Launcher",
      "technology": "Rust",                   // optional: what it is built with
      "kind": "container",                    // optional: "container" (default) or "store"
      "description": "Resolves a version, claims it, and runs its sub-binary.",
      "status": "changed", "tone": "risk", "link": "cli/launcher/src/main.rs:1"
    }]
  }],
  "relationships": [{
    "from": "launcher", "to": "cache",        // ids of people, systems or containers
    "label": "Claims, reads and prunes",      // needed: what the relationship is for
    "technology": "file system",              // optional: how
    "status": "new", "tone": "risk", "link": "cli/launcher/src/retention.rs:1"
  }]
}
```

## The two levels

- **System context**: give no system any `containers`. Each system is one
  box.
- **Container**: give `containers` to the system in scope, and to no other.
  It is drawn as a boundary around its containers, and its neighbours stay
  closed boxes. Relationships then run to and from the containers, not the
  system.

## How it is drawn

| What | Drawn as |
|---|---|
| A person | A box with a head above it, saying its name, `[Person]` and what they do |
| A software system | A box saying its name, `[Software System]` and what it is for |
| A system opened up | A dashed boundary, marked `«software system»` and named, around its containers |
| A container | A box saying its name, `[Container: technology]` and what it does |
| A store | The same, saying `[Store: technology]`: a database, a file store, a queue |
| Something external | The same shapes, greyed |
| A relationship | An arrow labelled with what it is for and, in brackets, how |

## Conventions

- Say what every element is for in a sentence. A box with only a name
  leaves the reader to guess.
- Word a relationship so it reads as a sentence from source to target:
  `Launcher` *fetches versions from* `GitHub Releases`.
- Mark as `external` what the team does not own.
- Mark what the change under review did with `status`, and the element it
  puts at risk with `tone: "risk"`.
- Keep to about 10 elements.

## What it checks

`problemsIn` reports a diagram with no people and no systems; a person,
system or container with no id or name, or an id used twice among them; a
`description` or `technology` that is not text; an `external` that is not
true or false; a container `kind` outside its list; a relationship with no
`from` or `to`, naming nothing the diagram has, with no label, or written
twice; and a `status`, `tone` or `direction` outside its list.

It remarks on an element joined to nothing, and on a relationship that
runs to or from a system whose containers are shown.

## What it leaves out

- **The component and code levels** of C4: use a component diagram and a
  class diagram.
- **The supplementary diagrams**: system landscape, dynamic and
  deployment. Use a sequence diagram and a deployment diagram.
- **Shapes for particular kinds of container**: a cylinder for a database,
  a pipe for a queue, a browser window for a web application.
- **A key**: the bracketed type on every element does its job.
