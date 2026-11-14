# Deployment diagram: the notation

```jsonc
{
  "id": "launcher-deployment",                // unique on the page it is shown on
  "kind": "deployment-diagram",
  "title": "Claims are files in the user's cache, written by the launcher on every start",
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "nodes": [{
    "id": "machine",                          // unique among nodes and artefacts
    "name": "developer machine",              // optional: the name shown; defaults to the id
    "stereotype": "device",                   // optional: "device", "execution environment" or any other word
    "status": "changed", "tone": "risk",      // optional
    "link": "docs/install.md:10",             // optional
    "nodes": [ /* optional: the nodes inside it, to any depth */ ],
    "artifacts": [{                           // optional: what is deployed on it
      "id": "launcher", "name": "accelerator",
      "stereotype": "executable",             // optional: shown in place of «artifact»
      "status": "new", "tone": "risk", "link": "cli/launcher/Cargo.toml:1"
    }]
  }],
  "paths": [{                                 // optional: how two nodes communicate
    "from": "machine", "to": "release", "label": "HTTPS",
    "status": "new", "tone": "risk", "link": "src/fetch.rs:10"
  }],
  "dependencies": [{                          // optional: one artefact needing another
    "from": "launcher", "to": "claims", "label": "refreshes",
    "status": "new", "tone": "risk", "link": "src/start_up.rs:78"
  }]
}
```

## How it is drawn

| What | Drawn as |
|---|---|
| A node | A box with depth, with its stereotype and name. A node that holds nodes or artefacts is drawn around them |
| A device | A node marked `«device»`: a machine, physical or virtual |
| An execution environment | A node marked `«execution environment»`, inside the device it runs on: a process, a container, a runtime, a shell |
| An artefact | A box marked `«artifact»`, or with its own stereotype, inside the node it is deployed on |
| A communication path | A plain line between two nodes, labelled with the protocol or means |
| A dependency | A dashed arrow from one artefact to another it needs |

Deployment is shown by nesting: an artefact is drawn inside its node, not
joined to it by a `«deploy»` arrow. A path has no direction, so it is
written once whichever way round.

## Conventions

- Name a node for what it is, and an artefact by its file or package name.
- A directory or a database that matters can be a node, with a stereotype
  that says so, when things are stored in it.
- Mark what the change under review did with `status`, and what crosses a
  boundary it should not with `tone: "risk"`.
- Keep to about 8 nodes and 12 artefacts.

## What it checks

`problemsIn` reports a diagram with no nodes; a node or artefact with no
id, or an id used twice among them; a `stereotype` that is not text; a
path with no `from` or `to`, that does not join two nodes the diagram has,
or written twice in either direction; a dependency with no `from` or `to`,
that does not join two artefacts the diagram has, or written twice; and a
`status`, `tone` or `direction` outside its list.

It remarks on an artefact that depends on one deployed on a different node
when no communication path joins the two nodes or any node that holds
them; and on a node with nothing on it that is joined to nothing.

## What it leaves out

- **Deployment specifications** and the `«deploy»` and `«manifest»`
  dependencies: nesting shows deployment, and a component diagram shows
  what an artefact implements.
- **Node instances** as distinct from node types, and multiplicities on
  paths.
- **Components drawn inside nodes**: show the artefact that carries them.
