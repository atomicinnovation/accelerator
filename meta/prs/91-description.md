---
type: "pr-description"
id: "91"
title: "Extract further-ideas backlog into work items 0231-0274"
date: "2026-08-31T12:51:17+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/91"
pr_number: 91
tags: ["backlog", "work-items"]
revision: "e6341c463bb9a754114e2a914a48ac98ec906f70"
repository: "accelerator"
last_updated: "2026-08-31T12:51:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Extract further-ideas backlog into work items 0231-0274

## Summary

Captures a running backlog of 44 feature, bug, and infrastructure ideas as a short-form note, then extracts each one into a structured work item so they enter the normal refinement and planning pipeline.

## Changes

- Adds `meta/notes/2026-06-23-further-ideas-backlog.md`, a running backlog of 44 ideas each tagged with an intended kind (`[Bug]`, `[Story]`, `[Task]`).
- Adds 44 work items, `0231`–`0274`, one per backlog line — each a source-faithful thin draft with Summary, Context, Requirements, seed acceptance criteria, and (where the source leaves a decision open) Open Questions.
- Marks every extracted item `status: draft`, `priority: medium`, and carries the verbatim non-enrichment Drafting Note so `/refine-work-item` and reviewers can identify them as needing individual expansion before promotion to `ready`.

## Context

Extracted from `meta/notes/2026-06-23-further-ideas-backlog.md` via `/extract-work-items`. No owning work item — these files are themselves the extracted backlog. Items span the visualiser, the CLI and its config/domain crates, the design skills, work-item sync, help output, and build-system cleanup.

## Testing

- [x] Slug-collision check against the existing 230 work items before allocation — no collisions.
- [x] IDs `0231`–`0274` allocated in one `accelerator work next-number --count 44` call, preserving presentation order.
- [ ] Frontmatter schema validation runs in CI's docs lane (not run locally in this session).

## Notes for Reviewers

- These are deliberately thin drafts, not fully-formed specs — intent captured, detail deferred to per-item refinement. Acceptance criteria and kinds may shift during refinement; several kinds are inherited from the source tag and flagged in Drafting Notes where the fit is loose (e.g. `0259` tagged bug but reads as a task).
- Open Questions carry real unmade decisions worth resolving during refinement: replacement names for `0263` (`ForeignDirt`) and `0268` (`remote-projection`), ban-vs-resolve for `0234`, config-vs-templates authority for `0238`, and precedence ordering for `0251`.
- A few tasks may grow into epics once refined — `0246`, `0249`, `0253`, `0271`.
