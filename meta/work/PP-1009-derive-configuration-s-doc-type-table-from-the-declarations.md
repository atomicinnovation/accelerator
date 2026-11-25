---
type: "work-item"
id: "PP-1009"
external_id: "PP-1009"
aliases: ["draft-rt7e2v"]
title: "Derive Configuration's Doc Type Table From the Declarations"
date: "2026-11-25T23:28:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-1006"
blocked_by: ["work-item:PP-994"]
tags: ["cli", "config", "doc-types"]
last_updated: "2026-11-25T23:28:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1009: Derive Configuration's Doc Type Table From the Declarations

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the CLI workspace, I want configuration's doc type path
keys, defaults and linkage names derived from the same doc type
declarations the corpus catalogue is built from, so that each doc type is
declared once and the hand-maintained copy in `config` retires.

## Context

`config` keeps its own table of doc type path keys, defaults and linkage
names (`cli/config/src/catalogue.rs`), because ADR-0069 forbids `config`
depending on `corpus`. A single-source test keeps the two aligned.
ADR-0080 makes the declarations assembly derive every platform
registration from one declaration per doc type, configuration's table
included. Declarations crates may depend on both `config` and `corpus`,
so no forbidden edge is needed.

## Requirements

- A doc type declaration carries what configuration needs: its path key,
  default directory and linkage name.
- The declarations assembly builds configuration's doc type table from
  the declarations, and every composition root and the launcher give it to
  configuration.
- `config` holds no doc type entries of its own, and no dependency on
  `corpus`.
- The single-source test retires.

## Acceptance Criteria

- [ ] Given configuration's doc type table before and after, every path
      key, default and linkage name is identical, pinned by a
      characterisation test written first.
- [ ] Given `config`'s sources, no doc type path key, default or linkage
      name is hard-coded.
- [ ] Given the crate-rule lint, `config` does not depend on `corpus` or
      on any declarations crate.
- [ ] Given every command that reads doc roots, including the launcher's
      `config path --doc-types`, they resolve the same paths as before.
- [ ] `mise run` exits 0.

## Open Questions

- How configuration receives the table: injected at each composition root,
  or registered once at start-up.

## Dependencies

- Blocked by: PP-994
- Blocks: none

## Assumptions

- Configuration's other keys, those not about doc types, stay where they
  are; this story covers only the doc type table.

## Drafting Notes

- Domains declaring the rest of their configuration is left out, at the
  author's direction.

## References

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
- `meta/work/PP-1007-declare-doc-types-in-their-domains-declarations-crates.md`
