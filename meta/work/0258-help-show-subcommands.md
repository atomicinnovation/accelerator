---
type: "work-item"
id: "0258"
title: "Help Should Show Subcommands"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "bug"
priority: "medium"
parent: "work-item:0276"
relates_to: ["work-item:0164", "work-item:0187", "work-item:0259", "work-item:0260"]
tags: ["cli", "help", "discoverability"]
last_updated: "2026-09-05T11:07:48+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-788"
---

# 0258: Help Should Show Subcommands

**Kind**: Bug
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator help` and bare `accelerator` list only the four built-in
commands (`version`, `config`, `cache`, `help`), omitting the nine
dispatched sub-binaries (`work`, `vcs`, `jira`, `linear`, `corpus`,
`collaboration`, `design`, `migrate`, `visualiser`) that `accelerator
--help` already lists. Anyone reaching for `help` or the bare command
gets an incomplete, misleading command index. `--help` is the closest to
correct but not the target state: it too is restructured here — its
"External subcommands" heading removed and its descriptions rewritten — so
all three entry points converge on one merged, user-facing listing.

## Context

The launcher (`cli/launcher/`) exposes its surface through clap: built-in
commands are defined on the clap `Command` enum, dispatched sub-binaries
via clap's `external_subcommand` catch-all, enumerated from a manifest at
help time. Three entry points diverge today — `accelerator --help` shows
built-ins plus a separate "External subcommands:" section listing the
sub-binaries; `accelerator help` and bare `accelerator` show built-ins
only. The divergence is deliberate in the current code and traces to work
item 0164, which scoped the discoverable listing to `--help` alone.

## Requirements

**Reproduction**: run `accelerator help`, then `accelerator`, then
`accelerator --help`, and compare the command lists.

**Expected**: all three show the same complete command set.
**Actual**: only `--help` includes the sub-binaries, and under a separate
"External subcommands" heading.

- `accelerator help`, bare `accelerator`, and `accelerator --help` all
  list the complete command set — built-ins and dispatched sub-binaries —
  identically.
- Dispatched sub-binaries appear in the same "Commands" list as built-ins,
  with no "External subcommands" heading and no origin distinction.
- Each command carries a concise, user-facing one-line description,
  replacing the current launcher-internal phrasing (e.g. `The work
  create|show|resolve|diff|update sub-binary.`). The intended descriptions
  for the dispatched sub-binaries are:
  - `work` — Manage work items
  - `vcs` — Detect and inspect version-control state
  - `jira` — Manage Jira issues
  - `linear` — Manage Linear issues
  - `corpus` — Manage the meta-document corpus (ADRs, metadata, frontmatter)
  - `collaboration` — Manage pull-request collaboration
  - `design` — Validate and process design sources
  - `migrate` — Apply pending meta-directory schema migrations
  - `visualiser` — Serve the meta-directory visualiser
- The listing is driven by the same source of truth as dispatch, so a
  newly registered sub-binary appears automatically.

## Acceptance Criteria

- [ ] Running `accelerator help` lists every dispatched sub-binary
      alongside the built-ins.
- [ ] Running `accelerator` with no arguments lists the same complete
      command set as `accelerator help`.
- [ ] Bare `accelerator` prints the full command list but retains clap's
      existing non-zero missing-subcommand exit status.
- [ ] `accelerator help`, `accelerator`, and `accelerator --help` produce
      identical command listings — same commands, same descriptions, no
      heading separating built-ins from sub-binaries.
- [ ] No listed command carries launcher-internal phrasing such as
      "… sub-binary."; each dispatched sub-binary's description matches the
      intended text enumerated in Requirements.
- [ ] A test that registers a fixture sub-binary confirms it appears in
      all three help outputs with no change to help code.

## Open Questions

- None outstanding. Resolved: bare `accelerator` retains clap's non-zero
  missing-subcommand exit status while printing the full command list (see
  Acceptance Criteria).

## Dependencies

- Relates to 0259 (Unify Help Style) — the user-facing description wording
  and the command-list layout (merging built-ins and sub-binaries into one
  unheadinged list) are owned here; 0259 governs help *styling* on top.
  Sequencing: 0258 lands the merged-list restructure first, then 0259
  restyles it — 0259's scope note must be adjusted to cede command-list
  layout ownership to this item.
- Relates to 0260 (Move ADR Into Its Own Subcommand) — 0260 moves the ADR
  (Architecture Decision Record) surface into its own subcommand, changing
  the command surface the listing renders; the output must reflect whatever
  the surface becomes.
- Consumers of the sub-binary descriptions — the one-line descriptions are
  sourced from each sub-binary crate's `Cargo.toml` `description`, surfaced
  via the manifest (`BinaryEntry.description`). That field is dual-use: it is
  also the crate's published package metadata read by cargo tooling, so
  rewriting it for help display is a deliberate overload rather than a
  help-only change. The launcher's own `--help` external-subcommands block is
  the known help renderer, which this fix subsumes; confirm no other surface
  (e.g. the visualiser) and no cargo/packaging consumer depends on the current
  wording before rewriting.
- Built on 0164 (Launcher and Git-Style Dispatch, done) and 0187
  (Generalise the Sub-Binary Registration Surface, done) — the mechanism
  being fixed.

## Assumptions

- "Complete" means whatever dispatch currently knows about — nine
  sub-binaries today, but the set is dynamic (0260 will change it). The fix
  surfaces exactly what dispatch already knows about, adding none and hiding
  none, so the listing tracks the source of truth rather than a hardcoded
  nine.
- "Merged list" means users need not know which commands are in-process
  built-ins and which are dispatched external binaries — that distinction
  is an implementation detail.

## Technical Notes

- Divergence locus: `cli/launcher/src/main.rs` — `is_root_help()` returns
  false when the first argument is `help`, and `handle_parse_error()`
  routes only clap's `DisplayHelp` (the `--help` flag) to
  `render_augmented_help()`. Bare invocation yields
  `DisplayHelpOnMissingArgumentOrSubcommand`, which never reaches the
  augmented path.
- Augmentation appends the external block via clap `after_help`, sourced
  from `external_subcommands_section()` in
  `cli/launcher/src/launch/help.rs`. Merging into one list likely means
  abandoning `after_help` and composing the full command list, since
  built-ins come from the clap `Command` enum
  (`cli/launcher/src/launch/inbound/cli.rs`) and sub-binaries from the
  manifest.
- Descriptions live in manifest data (`BinaryEntry.description`,
  `cli/launcher/src/launch/outbound/resolve/manifest.rs`), not launcher
  source — the user-facing rewrite edits manifest/registration data,
  broadening the change beyond help code.
- Tests to extend: `cli/launcher/tests/dispatch.rs`, and the inline tests
  in `help.rs` and `main.rs`.

## Drafting Notes

- Reframed the Summary and Context: the original stub said help shows no
  subcommands, but `accelerator --help` already lists them, so the true
  defect is `help` and the bare command omitting what `--help` shows.
  Correct this if the author meant `--help` is broken too.
- Read "merge into one list" as hiding the built-in/external distinction
  entirely; a lighter reading is one heading with the sub-binaries visually
  grouped beneath it.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
- Related: 0259, 0260, 0164, 0187
