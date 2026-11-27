---
type: "work-item"
id: "PP-1007"
external_id: "PP-1007"
aliases: ["draft-p72rv4"]
title: "Declare Doc Types in Their Domains' Declarations Crates"
date: "2026-11-25T23:20:27+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:PP-1006"
blocked_by: ["work-item:PP-994"]
tags: ["cli", "corpus", "doc-types"]
last_updated: "2026-11-27T16:32:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-1007: Declare Doc Types in Their Domains' Declarations Crates

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the CLI workspace, I want the doc types owned by the
work, research and design contexts declared in those contexts'
declarations crates, so that each domain owns its doc types' facts and
rules and the `corpus` crate holds fewer product facts.

## Context

After PP-994, `research-declarations` and `design-declarations` hold the
topic research and design inventory set shapes, and the `declarations`
assembly builds the catalogue from the corpus's built-in declarations plus
those shapes. The doc types' facts and rules (wire token, path key, linkage
name, label, flags, slug, cluster, linkage-id and lifecycle rules, library
phase, facets) all remain in `corpus`.

## Requirements

- Move these doc type declarations, whole, out of the corpus's built-in
  declarations:

  | Doc types | Declared in |
  |---|---|
  | Work items, work item reviews | `work-declarations` (new) |
  | Topic research, codebase research, root cause analyses | `research-declarations` |
  | Design inventories, design gaps | `design-declarations` |

- Create `work-declarations` under ADR-0080's declarations role.
- The `declarations` assembly registers each crate's doc types; the
  corpus's built-in declarations keep only the doc types the spike
  decides.
- Production code names a moved doc type by none of its identities
  written as a string: wire token, configuration path key or linkage
  name. PP-994 left these places naming topic research or design
  inventories as strings:

  | Place | What names them |
  |---|---|
  | `config/src/catalogue.rs` | `DOC_TYPES` linkage-name to path-key table |
  | `corpus/src/frontmatter_validation/schema.rs` | schema rows and `LINKAGE_SOURCE_TYPES` |
  | `corpus/src/frontmatter_validation/template_shape.rs` | `SOURCE_TYPES` |
  | `corpus/src/linkage.rs` | `TYPE_PAIRS` |
  | `corpus/src/cluster.rs` | `STAGE_PUSH_ORDER` label |
  | `launcher/src/config_command/core/init.rs` | scaffold directory keys |
  | `launcher/src/config_command/core/paths.rs` | legacy path-key aliases |
  | `research/src/conduct/template.rs` | `ConductTemplate::SET` |

  `config` depends only on `kernel`, so it cannot read the catalogue
  itself; a composition root must hand it the declared identities. The
  tables above list every doc type, so each moves whole.

## Acceptance Criteria

- [ ] Given the catalogue before and after the move, every doc type's wire
      token, configuration path key, linkage name, label, flags and rules
      are identical, pinned by a characterisation test written first.
- [ ] Given a search of production code outside the declarations crates
      and the assembly, no moved doc type is named, either as a
      `DocTypeKey` constant or by an identity written as a string. The
      boundary test's string scan, which PP-994 limited to the
      visualiser server, covers the whole `cli/` workspace.
- [ ] Given the `config` single-source test, it passes without change.
- [ ] Given the crate-rule lint, the new crate and edges pass it.
- [ ] `mise run` exits 0.

## Open Questions

- None.

## Dependencies

- Blocked by: PP-994
- Blocks: none

## Assumptions

- Codebase research and root cause analyses belong to the research
  context. If not, they move to the spike.

## Drafting Notes

- PR descriptions and PR reviews are left to the spike, because whether
  they belong to `collaboration` or a new `github` context is undecided.
- Moving the whole declaration for topic research and design inventories
  goes beyond PP-994's shapes-and-layout scope by design.

## References

- `meta/decisions/ADR-0078-doc-types-as-declared-data.md`
- `meta/decisions/ADR-0080-domain-declarations-crates.md`
- `meta/plans/2026-11-24-PP-994-generic-set-model-indexing-and-api.md`
