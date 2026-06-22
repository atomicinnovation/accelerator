---
type: work-item
id: "0136"
title: "Migrate Shell Scripts into a Rust CLI"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0136: Migrate Shell Scripts into a Rust CLI

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the shell-script library that backs the skills into a Rust CLI,
consolidating logic into a typed, testable, cross-platform binary.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). A large bash library (config
reading, VCS detection, frontmatter parsing, migrations) currently backs the
skills and is constrained by a bash 3.2 floor.

## Requirements

- Identify the shell scripts and shared library functions in scope for
  migration.
- Define the Rust CLI surface that replaces them, preserving the
  `${CLAUDE_PLUGIN_ROOT}` invocation contract used by skills.
- Plan an incremental migration that keeps skills working throughout.

## Acceptance Criteria

- [ ] Shell-script responsibilities are migrated into a Rust CLI without
      regressing skill behaviour, with the migration sequenced so the plugin
      stays functional at each step.

## Open Questions

- Which scripts migrate first, and which (if any) remain as shell?
- How is the Rust CLI distributed alongside the existing visualiser binary?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- The Rust CLI would be distributed similarly to the existing visualiser binary
  (release artefact + checksum verification).

## Technical Notes

- Removes the bash 3.2 floor constraint for migrated functionality.
- Must preserve bare-path invocation semantics expected by skill `allowed-tools`.

## Drafting Notes

- Treated as an epic per the user's instruction; migration sequencing and the
  per-script breakdown are decomposition work for refinement.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
