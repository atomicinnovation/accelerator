---
type: "work-item"
id: "PP-1006"
external_id: "PP-1006"
aliases: ["draft-r7bqc1"]
title: "Domain-Owned Doc Type Declarations"
date: "2026-11-25T23:19:59+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "medium"
blocked_by: ["work-item:PP-994"]
tags: ["cli", "corpus", "doc-types", "architecture"]
last_updated: "2026-11-25T23:19:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1006: Domain-Owned Doc Type Declarations

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the CLI workspace, I want every doc type declared by the
context that owns it, so that the corpus holds the model of a doc type and
none of the product facts. ADR-0080 introduces a declarations crate per
context and an assembly that builds the complete doc type catalogue; PP-994
uses them for the topic research and design inventory set shapes only. This
epic moves the rest.

## Context

The `corpus` crate declares all 15 doc types today. ADR-0078 makes doc types
declared data held in a catalogue, and ADR-0080 lets each context declare
what it contributes to platform crates in its own declarations crate, with
one assembly crate combining them. After PP-994 only set shapes live with
their owners; every doc type's facts and rules still live in `corpus`.

Several doc types have no owning domain crate: decisions, plans, plan
reviews, validations, PR descriptions, PR reviews and notes. Templates are
listed as a doc type although they have no root directory and are always
the exception.

## Requirements

- Declare the doc types whose owning domain crate exists in that domain's
  declarations crate (child story).
- Decide a home for every doc type without an owning domain, and for
  templates (child spike), then carry out the follow-up items it raises.
- Give ADR commands their own decisions CLI (PP-790).
- Derive configuration's doc type table from the same declarations
  (child story).

## Acceptance Criteria

- [ ] Every doc type that has a root directory is declared in its owning
      context's declarations crate.
- [ ] The `corpus` crate holds the doc type model and the catalogue's
      construction rules, and no doc type declarations.
- [ ] Templates are no longer a corpus doc type.
- [ ] Configuration holds no hand-maintained doc type table; its doc type
      path keys, defaults and linkage names come from the declarations.

## Open Questions

- None beyond those the child spike holds.

## Dependencies

- Blocked by: PP-994 (declarations crates, the assembly and doc types as
  declared data)
- Blocks: none

## Assumptions

- "Separate templates from the corpus" means templates stop being a doc
  type and get a catalogue of their own.

## Drafting Notes

- Standalone epic, not under PP-860, at the author's direction.
- Only configuration's doc type table is in scope; the rest of domain
  configuration is left out for now, at the author's direction.

## References

- `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
- `meta/work/PP-994-generic-set-model-indexing-and-api.md`
- `meta/work/PP-790-move-adr-to-own-subcommand.md`
