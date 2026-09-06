---
type: "pr-description"
id: "100"
title: "Reparent Rust CLI work items under 0136 and add successor epic 0276"
date: "2026-09-05T22:38:10+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0136", "work-item:0276"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/100"
pr_number: 100
tags: ["work-items", "rust", "cli", "linear-sync"]
revision: "e48e7ec5653f0ccc5f05741df48a8e41c13c3ca7"
repository: "accelerator"
last_updated: "2026-09-05T22:38:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Reparent Rust CLI work items under 0136 and add successor epic 0276

## Summary

Reorganises a parentless backlog of Rust-CLI work items into the right epics and
records the Linear sync that followed. An audit of every work item numbered above
0136 found 37 that belonged to the shell-to-Rust migration or the workspace it
produced, none of them linked to an owning epic.

## Changes

- **Fold 13 migration remnants under epic 0136.** Bugs in the shipped
  `config`/`corpus`/`migrate` crates, bash-cleanup tasks, the foundational
  architecture spike, and the runtime-cache cluster are now children of 0136 and
  appear in a dated "Post-migration remnants" subsection of its decomposition.
- **Add successor epic 0276, Rust CLI Consolidation and Hardening.** Gathers the
  24 post-migration evolution items — crate architecture, naming hygiene, CLI
  ergonomics, and design-daemon lifecycle — that extend the workspace rather than
  complete the migration. Renumbered from 0275 to avoid a cross-branch id clash.
- **Reparent 24 items under 0276** with `parent` links and a dated provenance
  note on each.
- **Record the Linear sync.** 55 issues created on team PP (33 reorganised items
  plus 22 pre-existing drafts), with their `external_id`s written back and the
  sync baseline advanced.
- **Capture the source note** `meta/notes/2026-08-31-third-ideas-backlog.md` from
  which 0276's children were drawn.

## Context

Spans two epics rather than one work item: 0136 (Migrate Shell Scripts into a
Rust CLI) and the new 0276. The Tier A/B split — remnants that belong to 0136
versus evolution that belongs to 0276 — is documented in 0276's Drafting Notes
and Open Questions. 0274 (isolate `gh` calls into a Python module) is deliberately
left out and raised as an open question, as it runs counter to the Rust direction.

## Testing

- [x] Frontmatter validated across all 62 changed meta files (`accelerator corpus
      frontmatter validate`) — zero violations.
- [x] Push-only Linear sync completed with zero per-item failures; all 55 creates
      wrote back an `external_id`.
- [ ] No code changed, so no build or test suite applies to this PR.

## Notes for Reviewers

- **Two commits, deliberately split:** `9b4c99c` carries the reorganisation and
  the source note; `f0a8f65` carries the Linear write-backs and baseline. The 33
  items that were both reparented and created on Linear carry their new
  `external_id` in the first commit — the parent edit and the id share a file and
  cannot be separated at file granularity.
- **28 Linear conflicts remain unresolved**, including 0182, 0201, and 0208 whose
  parent updates did not reach Linear (remotely modified since the last sync).
  Resolving them needs a bidirectional pass with `work.default_project_code: PP`
  set — a deliberate follow-up, out of scope here.
- **Open questions on 0276** (whether the crate-architecture cluster should be its
  own sub-epic; whether 0274 belongs) are recorded on the epic for triage, not
  settled by this PR.
