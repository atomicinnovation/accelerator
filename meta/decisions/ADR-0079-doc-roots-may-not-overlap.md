---
type: "adr"
id: "ADR-0079"
title: "Doc Roots May Not Overlap"
date: "2026-11-25T00:36:26+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:PP-994"
relates_to: ["adr:ADR-0068", "adr:ADR-0077", "adr:ADR-0078"]
tags: ["config", "corpus", "visualiser", "migrate"]
last_updated: "2026-11-25T00:36:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0079: Doc Roots May Not Overlap

**Date**: 2026-11-25
**Status**: Accepted
**Author**: Toby Clemson

## Context

Each doc type's root directory is configured under a `paths.*` key.
`config::paths::doc_type_dirs` resolves the full set of roots for the
visualiser, `accelerator config path --doc-types`, the corpus and work
commands, and the migrate adapter. It checks each path for safety, but
never compares two roots, so a root may equal or sit inside another.
A second resolver, `config::paths::resolve_with_fallback`, looks up one
`paths.*` key on its own, for the research commands, parts of the corpus
commands and `accelerator config path <key>`.

The end-to-end harness and the API smoke test depend on this. They point
`research_codebase` at `meta/research`, which puts the topic research and
design inventory roots inside it.

A file under two roots has no single doc type, and readers resolve the
ambiguity differently:

- The visualiser picks the first matching root while iterating a
  `HashMap` (`kind_for_canonical_path`), so the type it gives a file on
  refresh is effectively random.
- The corpus crate's `infer` picks the longest match.
- Flat listing does not recurse, so a flat root that contains a set root
  sees only its own top-level files.

Under ADR-0077, every member must belong to exactly one set doc type. A
symlink can also alias one root into another even when the configured
paths do not overlap.

## Decision Drivers

- Every file under the doc roots has at most one doc type.
- One rule, applied wherever the full set of roots is resolved.
- A bad configuration fails before any command that resolves a doc root
  reads or writes.
- A bad configuration can always be corrected.

## Considered Options

1. **Allow overlap, and attribute each file to the longest matching
   root** everywhere. Every reader must use the same rule, and a flat
   root still claims files that also sit inside a set root.
2. **Refuse only a set root nested inside another root.** This covers
   the set model's case, but leaves the visualiser's flat types
   ambiguous.
3. **Refuse any doc root that equals or contains another** (chosen).

## Decision

We will refuse a configuration in which one doc root equals or contains
another. The refusal names both configuration keys.

- **Where.** The refusal lives in configuration's shared resolver for the
  full set of doc roots, so every command that resolves that set
  refuses: the visualiser, the configuration, corpus and work commands,
  and `migrate`. `migrate` refuses before it applies or lists any
  migration, and a migration that writes an overlapping configuration
  stops the run and is not recorded as applied.
- **How roots are compared.** By path segments after normalisation,
  without touching the filesystem. `meta/plans/` and `./meta/plans` equal
  `meta/plans`, and `meta/plansx` does not overlap `meta/plans`.
- **Symlinks.** The visualiser applies the same rule again to the real
  directories it serves from, so a symlink alias is refused too.
- **Single-key lookups check too.** A lookup of one doc type's root
  resolves the full set and refuses the same way, so the research
  commands and a lookup of one configured path refuse as well. Lookups of
  path keys that are not doc roots are unaffected.
- **Correction stays possible.** Commands that edit configuration never
  resolve doc roots, so an overlapping configuration can always be fixed.

## Consequences

### Positive

- A file's doc type is never ambiguous, so the visualiser, the corpus and
  the set model all agree.
- `migrate` stops before applying anything, instead of running over the
  wrong directories.

### Negative

- A configuration that nests one root inside another, accepted until
  now, is refused. This is a breaking change for anyone relying on it.
- Test setups that nest roots, including the visualiser's end-to-end and
  API smoke tests, move to the default paths.
- Looking up one doc root now reads every doc root's key, not one.

### Neutral

- The default paths do not overlap, so an unconfigured project is
  unaffected.
- Frozen migrations that read one root directly need no change, because
  `migrate` refuses before any migration runs.
- Reading the doc roots inside a migration can now fail. Migrations pass
  that failure on and otherwise behave as before.

## References

- `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md`
- `meta/decisions/ADR-0077-generic-set-model-for-directory-doc-types.md`
- `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
