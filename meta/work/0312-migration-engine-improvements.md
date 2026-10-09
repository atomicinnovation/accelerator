---
type: "work-item"
id: "0312"
title: "Migration Engine Improvements"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "epic"
priority: "medium"
relates_to: ["work-item:0136", "work-item:0172", "work-item:0276"]
tags: ["migration-engine", "migrate", "rust"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0312: Migration Engine Improvements

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a maintainer of the migration capability, I want its post-port bugs and
contract gaps tracked together, so that the engine, the bundled migrations,
and their documentation converge on the contract the successor ADRs record.

## Context

work-item:0172 ported the migration framework to native Rust. Reconciling
the migration-engine ADRs against that port (work-item:0202) surfaced
behaviour that diverges from the recorded contract, user-visible bugs, and
guarantees nothing enforces. 0136 closes the port itself; this epic owns
what comes after it.

## Requirements

Scope covers the engine crates (`cli/migrate`, `cli/migrate-adapters`,
`cli/migrate-cli`), the bundled migrations m0001–m0010, and
`skills/config/migrate/SKILL.md`.

Initial children:

- 0202 — Supersede the migration-engine ADRs with Rust-native successors
- 0313 — Make migration 0007's transformation emission side-effect-free
- 0314 — Resolve migration 0007's unshipped ADR-0038 interactive affordances
- 0315 — Unknown applied migration IDs silence the lag advisory
- 0316 — Stall lists already-decided transformations as pending
- 0317 — Guard migration registry order and ID uniqueness
- 0318 — Give `verify_applied` read access to the corpus

## Acceptance Criteria

- [ ] Given the children listed in Requirements and any added later, when
      this epic closes, then every child has a terminal status.
- [ ] Given the accepted ADR-0037 successor, when this epic closes, then
      no bundled migration is recorded as non-compliant with it.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Standalone rather than under 0276 (Rust CLI Consolidation and
  Hardening): migration-contract work is a coherent domain of its own.

## Technical Notes

None.

## Drafting Notes

- 0202 moves here from 0136. It reconciles the port's documentation and is
  the source of every other child, so it belongs with them.
- Later findings from 0202's drafting are added as further children rather
  than folded into existing ones.

## References

- Codebase research:
  `meta/research/codebase/2026-10-09-0202-reconcile-migration-engine-adrs-against-the-rust-port.md`
- Related: 0136, 0172, 0202, 0276
