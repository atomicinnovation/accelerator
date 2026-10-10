---
type: "work-item"
id: "0320"
title: "Linear Push Corrupts Markdown Content"
date: "2026-10-10T16:36:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0146"
blocked_by: ["work-item:0296"]
relates_to: ["work-item:0296"]
tags: ["sync", "linear", "bug", "markdown"]
last_updated: "2026-10-10T16:36:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0320: Linear Push Corrupts Markdown Content

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Linear's parsing of a pushed `description` changes the meaning of some
markdown: it renumbers wrapped lines that start with digits, invents links
around dotted filenames and domains, and keeps list indentation inside wrapped
code spans.
Local work items are hard-wrapped at 80 columns, so every push risks these
rewrites, and 0296's equivalence reports them as real differences.

## Context

A 2026-10-10 inspection of the 32 pulled or conflicted items found thirteen
kinds of Linear re-serialisation, tabulated in 0296's Equivalence section.
0296 absorbs the ten cosmetic ones; the three below change meaning. Linear's parser departs from CommonMark: CommonMark
lets an ordered list interrupt a paragraph only when it starts at `1.`, and
strips continuation-line indentation before parsing inline code.

## Requirements

**Defect 1 — wrapped digit lines become ordered lists**

- Reproduction: 0161 (PP-180) has the list item `- The tier infrastructure …
  owned by` wrapped onto a continuation line `  0160.`; push it.
- Expected: the text `0160.` stays part of the list item.
- Actual: Linear reads `160.` as an ordered list item.

**Defect 2 — dotted tokens become links**

- Reproduction: 0276 (PP-860) contains bare `server.pid`; 0293 (PP-870)
  contains bare `jql.rs:167`; 0203 contains bare `crates.io`; push any.
- Expected: plain text.
- Actual: `[server.pid](<http://server.pid>)`,
  `[jql.rs:167](<http://jql.rs:167>)` and `[crates.io](<http://crates.io>)`.
- Bare URLs are also linked, as `[u](<u>)`; 0296 treats that as cosmetic
  because the link text equals its destination.

**Defect 3 — wrapped code spans keep list indentation**

- Reproduction: 0293 has a code span `` `labels NOT IN ('b')` `` wrapped
  across a line inside an indented list item; push it.
- Expected: the span's text is `labels NOT IN ('b')`.
- Actual: `labels NOT IN     ('b')`.

**Required behaviour**

- A pushed body read back from Linear is equivalent, under 0296's definition,
  to the local body: no renumbered lists, no invented links, no changed
  code-span text.
- Remote descriptions already corrupted are repaired by the next push of their
  item; there is no bulk rewrite.

**Out of scope**

- The cosmetic re-serialisations 0296 absorbs.
- Jira.
- Any change to local files.

## Acceptance Criteria

- [ ] Given a local body with a wrapped continuation line starting `0160.`,
      when it is pushed and read back, then the text `0160.` survives and no
      ordered list exists.
- [ ] Given a local body with bare `server.pid`, `jql.rs:167` or `crates.io`,
      when it is pushed and read back, then no link wraps it.
- [ ] Given a code span wrapped inside an indented list item, when it is pushed
      and read back, then its text equals the CommonMark rendering of the
      local span.
- [ ] Given each of the three reproductions, when 0296's equivalence compares
      the pushed and read-back bodies, then they are equivalent.
- [ ] Given each conflicted item whose remote body carries one of the three
      rewrites (at least 0161, 0203, 0276 and 0293), when it is resolved with
      `--resolve <id>=local`, then its next sync reports `synced`.

## Open Questions

- Fix on the push side (unwrap soft breaks, escape a leading `<digits>.`,
  escape linkifiable tokens), or report upstream to Linear? The first is
  within our control; the second is not.

## Dependencies

- Blocked by: 0296 — its criteria compare by 0296's equivalence.
- Blocks: none.

## Assumptions

- The rewrites happen when Linear parses a pushed `description`, not in its
  editor; a captured read-back fixture would confirm or refute this.

## Technical Notes

- The corrupted bodies were found by diffing local bodies against `accelerator
  linear show` descriptions after 0296's normalisation.

## Drafting Notes

- Framed as a push-time problem because all three rewrites trace to hard
  wrapping or bare tokens in local markdown.

## References

- Parent: 0146 — Work Item Synchronisation Enhancements
- Related: 0296 — Sync Round-Trip Defects Between Local Work Items and the Tracker
