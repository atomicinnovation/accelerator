---
type: "adr"
id: "ADR-0077"
title: "Generic Set Model for Directory Doc Types"
date: "2026-11-24T22:50:54+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:PP-994"
relates_to: ["adr:ADR-0067", "adr:ADR-0068", "adr:ADR-0069", "adr:ADR-0078", "adr:ADR-0079", "adr:ADR-0080"]
tags: ["corpus", "set-model", "topic-research", "design-inventories"]
last_updated: "2026-11-25T00:36:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0077: Generic Set Model for Directory Doc Types

**Date**: 2026-11-24
**Status**: Accepted
**Author**: Toby Clemson

## Context

Some doc types store each entry as a directory written by one domain, the
set's **owner**. A topic research entry is a `manifest.md` beside its
brief, outline, findings, level notes, synthesis and reports. A design inventory is an
`inventory.md` beside a `screenshots/` directory. More will follow, work
items with attachments among them.

Nothing declares what such a directory contains, so each reader decides
for itself, and they disagree:

- The visualiser indexes one entry per directory, and only when the root
  file exists. It sees nothing else in the directory.
- The corpus CLI's resolver (ADR-0068) treats every subdirectory of a set
  doc root as a set, root file or not. It resolves a path to any file at
  any depth inside a set directory, dot-prefixed and unslotted files
  included.
- The research crate's `research::topic::layout` describes the topic
  files for its own commands, in constants no other reader shares.

The only shared fact is the root filename,
`DocTypeKey::nested_manifest_filename()` in the `corpus` crate.

Set directories also hold files their owner never wrote. Some design
inventories carry hand-placed `prototype-full/` and `assets/` trees. Owners
also keep their own working files with a leading dot: the research skill's
quarantined files and run ledger, and the temporary directories in which
both the research and inventory-design skills build a set before renaming
it into place.

## Decision Drivers

- One definition of what belongs to a set, shared by its owner and every
  reader.
- The definition cannot drift from the code that writes the set.
- A new directory doc type needs a declaration, and no new reader code.
- Files the owner did not write stay out of the set.

## Considered Options

1. **Each reader walks the directory its own way,** as today. Readers keep
   disagreeing, and each new reader repeats the walk.
2. **A deny-list:** every file in the directory is part of the set, except
   dot-prefixed paths and declared exclusions such as `prototype-full/`.
   Each new hand-placed tree is in the set until someone excludes it.
3. **A declared allow-list shape, declared in the corpus** for every set
   type, with a contract test against each owner's layout. The corpus holds
   a copy of each owner's layout, and only the test keeps them aligned.
4. **A declared allow-list shape, declared by the owner** from its own
   layout and registered with the corpus (chosen).

## Decision

We will model a directory doc type's entry as a **document set**: a root
document plus **members**, where a member is a file that fits a **member
slot** of the doc type's declared **shape**. A slot admits either
documents (markdown, whatever the state of their frontmatter) or assets.

- **No root, no set.** A directory without its root file is not a set and
  has no members. Removing the root removes the set.
- **Outside the shape is outside the set.** A file that fits no slot is
  not part of any set.
- **Identity.** A set's `id` is its root's frontmatter `id`, or else its
  directory's name.
- **The owner declares the shape** from its own layout, and registers it
  with the corpus, so the shape and the owner's layout cannot drift
  apart.
- **The shape is the contract.** Every reader other than the owner
  decides membership through the shape, and none walks a set directory by
  its own rules.
- **The dot prefix is the owner's boundary.** Anything dot-prefixed at a
  set doc root or inside a set directory, such as temporary set
  directories, quarantined files and run ledgers, belongs to the owner and is invisible to every other
  reader, whether or not it fits a slot. Owners do not declare these
  working files.

Shapes are registered under ADR-0078. Doc roots may not overlap
(ADR-0079), so every member belongs to exactly one set doc type.

## Consequences

### Positive

- Owners and readers agree on what a set holds, and the shape is built
  from the owner's own layout. A new directory doc type is a new
  declaration and a registered shape.
- Hand-placed trees and owners' working files stay out of every reader.
- The visualiser can index members, and features that build on them get
  one definition: member routes, the set page, Sources rendering,
  screenshot rendering and the derived set response.

### Negative

- A file an owner writes outside its declared shape without a dot prefix
  is silently ignored by every other reader.
- A stray file in a set directory cannot be told from a working file the
  owner forgot to dot-prefix; nothing reports either.
- Readers that currently treat a set as a whole now have to tell a root
  from a member. In the visualiser, listings, counts and lifecycle
  clusters keep to roots, and a consumer that forgets this counts members
  as items.
- The corpus CLI's resolver changes to conform. It stops treating a
  directory without a root file, or a dot-prefixed directory, as a set,
  and resolves a path to a set only when the path is the set directory,
  its root, a member, or a directory that can hold members. A slug or
  path that resolves to a set today can stop resolving.
- The owners of today's sets, the research and design contexts, gain a
  dependency on the corpus to express their shapes in its terms, through
  the declarations crates of ADR-0080.

### Neutral

- Assets are part of the model before anything serves them; screenshot
  rendering is their first consumer.
- The visualiser's member API is specified in
  `meta/specs/2026-11-24-set-member-api.md`.

## References

- `meta/specs/2026-11-24-set-member-api.md`
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
- `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md`
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
- `meta/decisions/ADR-0079-doc-roots-may-not-overlap.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
- `skills/research/research-topic/SKILL.md`
- `skills/design/inventory-design/SKILL.md`
