---
type: "adr"
id: "ADR-0080"
title: "Domain Declarations Crates"
date: "2026-11-25T22:38:04+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:PP-994"
relates_to: ["adr:ADR-0069", "adr:ADR-0077", "adr:ADR-0078"]
tags: ["cli", "architecture", "crates", "corpus", "config"]
last_updated: "2026-11-25T22:38:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0080: Domain Declarations Crates

**Date**: 2026-11-25
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADR-0077 makes the domain that writes a document set declare the set's
shape and register it with the corpus. ADR-0069 gives each workspace crate
one role and restricts the edges between them. Two of its rules bear on
where such a declaration can live:

- No context depends on a product context, and only composition roots may
  depend on any crate. The owners of today's sets, `research` and
  `design`, are product contexts.
- The launcher may depend only on `kernel`, platform crates and technical
  libraries.

If an owner's domain crate holds its declaration, every composition root
that needs the complete set of doc types must depend on that domain crate,
with all its model and ports. The launcher cannot depend on it at all. The
domain crate also gains a dependency on the platform crate it declares
against, which its own model may not otherwise need.

The same need will recur. Configuration keeps its own hand-maintained
table of doc type path keys and linkage names, which a test keeps aligned
with the corpus. Domains could declare their doc types' configuration the
way they declare set shapes, so that one declaration serves both.

## Decision Drivers

- A domain's declarations reach every composition root, the launcher
  included.
- A domain crate depends only on what its own model needs.
- Every composition root gets the same complete set of declarations, and
  none can forget one.
- Each doc type is declared once, and every platform registration that
  concerns it, the corpus catalogue and configuration's doc type table
  alike, is derived from that one declaration.
- The rest of ADR-0069 holds.

## Considered Options

1. **Owners' domain crates hold their declarations,** and composition
   roots register them. Every root that needs the complete catalogue
   depends on every owning domain, the launcher cannot, and each domain
   crate depends on the platform crates it declares against.
2. **Platform crates hold every declaration,** with a test against each
   owner's layout. The platform crates hold copies of product facts, kept
   aligned only by tests.
3. **A declarations crate per context, with no assembly.** Every root
   registers every domain itself and must not miss one.
4. **A declarations crate per context, plus one assembly crate** (chosen).

## Decision

We will add a **declarations** crate role. A context's declarations crate
holds what that context declares to cross-cutting platform crates: its
doc types, with their layouts, set shapes and configuration path keys.
One **declarations assembly** crate combines every context's declarations
into the complete registrations each platform crate needs.

- **One declaration per doc type.** A doc type is declared once, by its
  owner. The assembly derives both the complete corpus catalogue and
  configuration's doc type table, its path keys, defaults and linkage
  names, from those declarations. Configuration receives its table rather
  than keeping its own, so it gains no dependency on the corpus.
- **A declarations crate** holds declared data and no behaviour. It
  depends only on `kernel` and platform domain crates. Any crate in its
  own context, any composition root and the launcher may depend on it.
- **The declarations assembly** depends on platform domain crates and on
  declarations crates. Only composition roots, the launcher and tests
  depend on it. It is the single place a new declarations crate is added.
- **A domain crate** depends on its own context's declarations crate for
  the facts it shares with platform crates, such as its layout, so the
  domain and its declarations cannot drift.
- **Naming.** A context's declarations crate is `<context>-declarations`.
  The assembly is `declarations`.
- **Enforcement.** The crate-rule lint learns both roles.

This extends ADR-0069's role table and its inward rule, since a domain
crate may also depend on its own context's declarations crate. It
partially supersedes ADR-0069's launcher rule: the launcher may also
depend on declarations crates and the assembly. Following the precedent
ADR-0069 set for ADR-0054, ADR-0069 stays `accepted` and unedited, and the
change is recorded here.

The first declarations crates are `research-declarations` and
`design-declarations`, holding the topic research and design inventory
layouts and set shapes. Doc types move to their owners' declarations
crates incrementally; until a doc type moves, its declaration stays in the
corpus and configuration keeps its entry in its own table.

## Consequences

### Positive

- Every composition root, the launcher included, can build the complete
  doc type catalogue through one dependency.
- `research` and `design` stay free of a `corpus` dependency their models
  do not need.
- A root cannot assemble a partial catalogue by forgetting a domain.
- Configuration's hand-maintained doc type table, and the test that keeps
  it aligned with the corpus, retire once every doc type is declared by
  its owner.

### Negative

- Two new roles in the crate model, and two new crates now, with one more
  per domain that declares anything.
- The assembly depends on every declarations crate, so adding a domain's
  declarations changes it.
- Every new declarations crate goes through the crate registration
  checklist, and the crate-rule lint and intra-crate rules carry two more
  roles.

### Neutral

- Doc types with no owning domain crate stay declared in the corpus.
- Until a doc type moves to its owner, its facts stay in the corpus beside
  a shape the owner declares, and configuration keeps its entry in a table
  of its own.
- The launcher's limit to platform crates otherwise stands.

## References

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
- `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
