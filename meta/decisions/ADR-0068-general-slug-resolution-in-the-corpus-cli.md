---
type: "adr"
id: "ADR-0068"
title: "General Slug Resolution in the Corpus CLI"
date: "2026-09-09T12:36:29+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:0277"
relates_to: ["adr:ADR-0067"]
tags: ["corpus", "cli", "slug", "resolution", "topic-research"]
last_updated: "2026-09-09T12:36:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0068: General Slug Resolution in the Corpus CLI

**Date**: 2026-09-09
**Status**: Accepted
**Author**: Toby Clemson

## Context

Skills and tools repeatedly need to turn a human-supplied reference into a
document's canonical root on the filesystem — for a flat document, the file
itself; for a set of related documents, the set's root directory. The reference
arrives in tolerant forms: a bare identifier, the containing directory, or a
path to a document within a set. This resolution is a cross-cutting concern
shared by any command that operates on documents by identifier.

Two existing facts shape where resolution belongs. The CLI already establishes a
resolution precedent in `work resolve`, which turns paths, IDs, and bare numbers
into a work item's root. The corpus domain already models a **slug** as a
cross-document-type identifier, so resolution has an established vocabulary to
reuse rather than a new term to coin.

Skills themselves have no resolution precedent — existing multi-verb skills pass
positionals opaquely to a CLI. Resolution expressed in skill prose would be
untestable and reimplemented in every skill that needed it.

## Decision Drivers

- Make resolution a reusable primitive usable by any skill and any doc type,
  not a per-skill or per-domain reimplementation.
- Reuse the CLI's existing `work resolve` resolution pattern rather than
  inventing a new one.
- Reuse the established "slug" vocabulary for the resolved identifier.
- Keep resolution in a tested CLI rather than in brittle skill prose.
- Avoid the cost of a dedicated sub-binary and crate triad when the required
  surface is a single resolution command.

## Considered Options

1. **Resolve inline in skill prose** — no precedent exists, the logic is
   untestable, and it would be duplicated across every skill that needs it.
2. **A dedicated per-domain sub-binary** (domain + adapters + cli triad, with
   full registration) — the purest mirror of `work`, but it incurs three crates
   and a full registration to host a single resolution command, a cost repeated
   for every domain that needs resolution.
3. **A general `corpus resolve` subcommand on the existing `corpus` binary**,
   parameterised by doc type — reuses the corpus config plumbing and the `work
   resolve` pattern; one command serves every doc type.

## Decision

We will add a general `corpus resolve --type <type> <slug>` subcommand to the
`corpus` binary that resolves a slug to a document's root for any registered doc
type — a file for flat types, the set directory for nested-manifest types. The
doc type is a parameter, so a single command serves every type.

It mirrors the `work resolve` precedent: input classification and ambiguity
candidate handling from the domain crate, and the canonicalise-then-under-root
containment check from the adapter. The corpus config plumbing supplies the
configured directory for the given type; the resolver joins, canonicalises, and
enforces containment under that root, accepting the tolerant forms — a bare
slug, the containing directory, or a path to a document within the set — that
normalise to the root.

## Consequences

### Positive

- One general resolution primitive that any skill or doc type can use.
- Reuses the corpus binary's config plumbing; no new crate, binary, or
  registration.
- Resolution is exercised by CLI tests rather than embedded untestably in skill
  prose.
- Keeps the identifier vocabulary consistent by reusing "slug".

### Negative

- Type-specific set-root semantics live in a generic tool rather than a
  domain-owned crate — a deliberate layering compromise traded for reuse.
- A single generic resolver cannot encode domain-specific resolution rules as
  richly as a dedicated per-domain resolver could.
- A doc type is resolvable only once it is registered in the corpus doc-type
  registry; resolution is coupled to registration.

### Neutral

- A document's "root" means different things by type — a file for flat types,
  the set directory for nested-manifest types — so the resolver branches on type.
- Flat dated types (`YYYY-MM-DD-…-<slug>.md`) carry ambiguous slugs, so
  resolution relies on the same candidate handling as `work resolve`.

## References

- `cli/work/src/resolve.rs` — the `work resolve` domain algorithm
  (classification, candidate handling) mirrored
- `cli/work-cli/src/resolve.rs` — the `work resolve` adapter
  (canonicalise-under-root) mirrored
- `meta/decisions/ADR-0067-kind-discriminated-corpus-schema-and-templates.md` —
  sibling corpus decision
