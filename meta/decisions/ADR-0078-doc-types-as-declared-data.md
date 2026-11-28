---
type: "adr"
id: "ADR-0078"
title: "Doc Types as Declared Data"
date: "2026-11-24T22:50:54+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:PP-994"
relates_to: ["adr:ADR-0067", "adr:ADR-0069", "adr:ADR-0077", "adr:ADR-0080"]
tags: ["corpus", "doc-types", "visualiser", "server", "set-model"]
last_updated: "2026-11-25T22:38:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0078: Doc Types as Declared Data

**Date**: 2026-11-24
**Status**: Accepted
**Author**: Toby Clemson

## Context

`DocTypeKey` in the `corpus` crate (`cli/corpus/src/doc_type.rs`) is a
closed enum of 15 variants. Each fact about a doc type is a `const fn`
with a `match` arm per variant: its config path key, linkage name, label,
lifecycle flags, kanban membership, whether it is virtual, its nested
manifest filename and its wire token. Rules elsewhere branch on named
variants too: slug derivation, the cluster-key walk, lifecycle rank and
completeness marks, linkage id resolution, the visualiser's library
phases, facets and the work-item drafts listing.

The workspace has about 640 references to `DocTypeKey` across about 45
files, about 90 of them production `match` arms that carry real rules.

ADR-0077 gives each set doc type a shape, declared by the domain that
writes the set and registered with the corpus. Proving the set model
generic needs a doc type declared only in test code, through the same
mechanism topic research uses, that gets the whole set model. A closed enum
cannot hold a type that exists only in tests, and a `match` arm per
variant puts each doc type's facts in a dozen places.

The `config` crate keeps its own table of linkage names and path keys
(`cli/config/src/catalogue.rs`). ADR-0069 forbids `config` depending on
`corpus`, and a single-source test keeps the two tables aligned.

## Decision Drivers

- A doc type declared only in tests goes through the same mechanism as a
  built-in one.
- One declaration, by its owner, holds each doc type's facts, rules and
  shape.
- Code that serves doc types branches on what a doc type declares, never
  on which doc type it is.
- A bad declaration fails when it is assembled, never at first use.
- Referring to a doc type stays as cheap as it is today, so the hundreds
  of places that do so change mechanically.
- Every composition root gets the same complete catalogue, set shapes
  included.
- The crate dependency rules of ADR-0069, as extended by ADR-0080, hold.

## Considered Options

1. **Keep the closed set of doc types, one function per fact,** and add
   set shapes the same way. A doc type cannot exist only in tests.
2. **Keep the closed set, and give the visualiser its own table of set
   shapes.** Only the visualiser would know set shapes, so the corpus
   resolver keeps its own rules, and every other rule stays per doc type.
3. **Add a test-only member to the closed set behind a build feature.**
   Feature unification leaks it into every crate that depends on
   `corpus`.
4. **Doc types as declared values in a catalogue** (chosen).

## Decision

We will make each doc type a declared value, held in an ordered catalogue,
and express every per-doc-type rule as a field of that declaration.

### Declarations

A doc type declaration holds:

- its identity and facts: wire token, configuration path key, linkage
  name, label and capability flags;
- its rules: how a slug is derived, how its items cluster, how a linkage
  id resolves, its lifecycle rank and completeness mark, its library
  phase and its facets;
- how it is listed: as flat files, with an optional drafts directory and
  filename filter, or as sets. A set listing carries the set's
  shape and presentation.

Each rule is one of a closed, named set of choices, the rules the code
encodes per doc type today, so a declaration is typed data, not strings.

A **presentation** says how the visualiser shows a set: as a **set page**
that lists its members, or as an **entry point**, a root document that
shows its members within its own page. It is declared data, and it
changes nothing until the set page and screenshot rendering read it.

### Set shapes

A shape is a root filename plus member slots, each slot admitting
documents or assets. A slot is a `/`-separated pattern relative to the set
directory. Each segment is a literal name or `*`, with an optional file
extension. `*` matches one name that does not start with a dot. There is
no recursive wildcard.

A slot's file extension, in its final segment only, matches regardless of
case, so `screenshots/a.PNG` fits `screenshots/*.png`. Directory segments
and the rest of the name match case-sensitively. Extensions ignore case
because:

- macOS's default APFS volume is case-insensitive but case-preserving,
  and a listing returns the stored name, so a case-sensitive match would
  skip `a.PNG` there too;
- cameras, phone exports and Windows tools often write uppercase
  extensions;
- the rule gives the same result on every platform. On a case-sensitive
  volume `a.png` and `a.PNG` become two members, which is harmless.

The corpus defines what a shape is. The domain that writes a set declares
its shape in those terms, from its own layout:

| Set type | Owner | Root | Document slots | Asset slots | Presentation |
|---|---|---|---|---|---|
| Topic research | research | `manifest.md` | `brief.md`, `outline.md`, `synthesis.md`, `findings/*.md`, `findings/*.levels/*.md`, `reports/*.md` | none | set page |
| Design inventories | design | `inventory.md` | none | `screenshots/*.png` | entry point |

The topic research slots follow the rules by which the research domain
reads its findings and reports, not the stricter rules by which it names
new ones.

### Assembling the catalogue

The corpus defines what a doc type declaration and a shape are. Each doc
type is declared once: by its owner, in the owner's declarations crate
(ADR-0080), or by the corpus while it has no owner. A set type's shape is
part of its declaration. The declarations assembly builds the complete
catalogue from every declaration. Building it refuses a duplicate
declaration, a set type with no shape, a shape for a type that is not a
set, and a slot pattern that does not parse, so a bad declaration fails at
startup. Tests build a catalogue the same way.

### Identity

A doc type is referred to by a cheap, copyable key that points at its
declaration and compares by wire token. Existing call sites keep their
shape. Tests declare a type the way built-in types are declared.

### The catalogue

The catalogue is an ordered list of declarations, looked up by wire
token, linkage name or configuration path key. The assembled catalogue is
the single source of today's doc types. Consumers that need a runtime
lookup are given a catalogue, and tests substitute their own. Building a
catalogue also refuses a duplicate wire token, linkage name or
configuration path key.

### Built-in declarations name every field

Test declarations may start from flat defaults. Built-in declarations may
not, so adding a built-in doc type forces a choice for every rule.

### No production code names a built-in doc type

Outside the declarations themselves, in the corpus and the declarations
crates, and the assembly that combines them, no production code names a
built-in doc type. The exception is code that owns one doc type's domain,
such as the research domain's topic set commands and the corpus CLI's ADR
commands. A
boundary test enforces this.

### Typed linkage references stay as they are

The typed reference forms of ADR-0034 (`work-item:`, `plan:`, `adr:`,
`pr:`) encode the linkage grammar, not per-type facts, and stay as they
are. A test-only type takes part in linkage through a plain path
reference.

### Root resolution stays with configuration

Built-in types resolve their roots through configuration's shared
doc-root resolver. A test-only type's root is supplied directly to the
code that serves it.

### Boundaries

- Doc types are declared in code only. Users cannot declare doc types in
  configuration.
- The visualiser frontend keeps its own static list of doc types.
- Configuration's table of doc type path keys and linkage names is
  derived from the same declarations under ADR-0080, not kept by hand.
- Frozen migrations keep working against built-in doc types, with
  unchanged behaviour.

## Consequences

### Positive

- A doc type's facts and rules live in one declaration, and adding a doc
  type means adding one value.
- Tests can declare a doc type, a set doc type included, and exercise the
  visualiser through the mechanism the built-in types use.
- The visualiser holds no knowledge of any named doc type, which the
  boundary test keeps true.
- A duplicate declaration or a missing or malformed shape fails when the
  catalogue is built.
- A set's shape lives beside the code that writes the set.

### Negative

- The change touches every crate that depends on `corpus`.
- Rules that read as one branch per doc type are now spread across
  declaration fields; reading one type's behaviour means reading its
  declaration.
- Consumers that need runtime lookup must be handed a catalogue.

### Neutral

- Code still passes the same kind of key around; only what it refers to
  changes.
- The frontend and the wire tokens are unchanged.
- Doc types move to their owners incrementally. Until one moves, its
  facts stay in the corpus beside a shape its owner declares, and
  configuration keeps its entry in a table of its own.

## References

- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md`
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
