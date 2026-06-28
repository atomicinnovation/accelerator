---
type: work-item
id: "0157"
title: "Port Luminosity ADRs and Feeding Spikes into Accelerator"
date: "2026-06-27T11:43:29+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: task
priority: medium
external_id: "PP-178"
tags: [adr, decisions, porting, luminosity, architecture, rust-cli, spikes]
last_updated: "2026-06-27T11:43:29+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0157: Port Luminosity ADRs and Feeding Spikes into Accelerator

**Kind**: Task
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Port all eleven ADRs captured in the luminosity project
([meta/decisions](https://github.com/atomicinnovation/luminosity/tree/main/meta/decisions))
into this repository's `meta/decisions/`, making only the minimal edits needed
for each to apply to Accelerator in its current form. Several of those ADRs were
the result of spikes whose research and outcomes are recorded against the spike
work items; those feeding spikes are ported locally as well so the ADRs' decision
provenance lives in this repo.

## Context

Luminosity recently recorded a coherent set of foundational architecture
decisions that apply equally well to Accelerator — particularly those describing
a Rust-CLI direction (hexagonal core, on-demand static binaries, three-/multi-
toolchain split) that aligns with the migration surface already being researched
here (`meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`,
staged in this branch). Importing these decisions gives Accelerator a ready-made,
already-reasoned decision record rather than re-deriving each one.

The eleven luminosity ADRs are all `accepted` upstream:

| Luminosity ADR | Title | Nature in Accelerator |
|---|---|---|
| ADR-0001 | Skills-vs-CLI Division of Labour | net-new |
| ADR-0002 | Zero-Setup Static-Binary Distribution | net-new (spike-fed) |
| ADR-0003 | Multi-Level Userspace Configuration Model | **overlaps** local ADR-0016/0017 |
| ADR-0004 | Three-Toolchain Split (Python/Shell/Rust) | overlaps reality (Accelerator has 4 toolchains incl. TS frontend) |
| ADR-0005 | Bash 3.2 Compatibility Floor | documents an existing Accelerator convention (no local ADR) |
| ADR-0006 | mise + invoke Task Runner | documents an existing Accelerator convention (no local ADR) |
| ADR-0007 | Skills as the Product | likely net-new (verify vs README philosophy) |
| ADR-0008 | Filesystem as Message Bus and Knowledge Corpus | **overlaps** local ADR-0001/0027 |
| ADR-0009 | Thin CLI over a Hexagonal Ports-and-Adapters Core | net-new (spike-fed) |
| ADR-0010 | Git-Style Modular CLI of On-Demand Static Binaries | net-new (spike-fed) |
| ADR-0011 | Inspect as the Skill-Evaluation Harness | net-new (spike-fed) |

Feeding spikes (both `done`, results captured against the work item):
- [work item 0002 — Modular Rust CLI Architecture & Hexagonal Workspace Layout](https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md)
  (feeds ADR-0002, ADR-0009, ADR-0010)
- [work item 0003 — Skill Evaluation Framework Selection](https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0003-skill-evaluation-framework-selection.md)
  (feeds ADR-0011)

This repo already holds ADR-0001…0044 and a populated `meta/work/`, so
luminosity's identifiers collide and cannot be preserved verbatim — "current
form" applies to each ADR's *content and reasoning*, not its identifier.

## Requirements

- Port all eleven luminosity ADRs into `meta/decisions/`, each renumbered to the
  next free local sequential ID (0045 onward) per ADR-0029, and imported with
  status `proposed`.
- Make only the minimal content edits required for each ADR to apply to
  Accelerator as-is: project/repository name references, repo-specific facts
  (e.g. ADR-0004 says *three* toolchains — Accelerator has *four*, including the
  TypeScript/React frontend), and paths. Preserve each ADR's structure,
  reasoning, options, and consequences otherwise.
- Rewrite every internal cross-reference between the ported ADRs (frontmatter
  `relates_to`/`supersedes` and in-prose references) to the new local ADR
  numbers, so the ported set is internally coherent.
- Where a ported ADR overlaps an existing **accepted** local ADR, determine the
  **superset** of the two decisions, author the ported ADR as that superset, and
  mark the overlapped local ADR(s) as **superseded** by it. Because immutability
  (ADR-0031) only allows accepted→superseded transitions, the supersede edge is
  applied (via `/review-adr`) once the superset ADR is itself accepted. Candidate
  overlaps to evaluate during porting:
  - luminosity ADR-0003 (config model) ↔ local ADR-0016 + ADR-0017
  - luminosity ADR-0008 (filesystem as bus/corpus) ↔ local ADR-0001 + ADR-0027
  - verify ADR-0004/0005/0006/0007 against existing conventions before deciding
    net-new vs superset.
- Port the two feeding spikes (luminosity work items 0002 and 0003) into
  `meta/work/`, renumbered to the next free local IDs, preserving their research
  content and captured outcomes.
- Re-point each ported ADR's provenance references to the locally-ported spikes,
  retaining the luminosity originals as full GitHub URLs (secondary reference).
- Ensure all ported artifacts conform to this repo's schema: unified base
  frontmatter (ADR-0033), typed-linkage vocabulary (ADR-0034), omit-when-empty
  emission (ADR-0040), and the local ADR template (ADR-0030). Set
  `author`/`date`/`last_updated`/`last_updated_by` to the porting context.
- Every reference to luminosity throughout the ported artifacts uses full
  `https://github.com/atomicinnovation/luminosity/…` URLs; local relative paths
  may appear only as a secondary aid.

## Out of Scope

- Implementing the architecture the ported ADRs describe (Rust CLI, hexagonal
  core, static-binary distribution, inspect harness). These ADRs record
  decisions; building against them is separate downstream work.
- Re-running the spikes — their captured results are imported, not regenerated.
- Recording Accelerator conventions as ADRs beyond the supersets that porting
  forces.

## Acceptance Criteria

- [ ] All 11 luminosity ADRs exist in `meta/decisions/` under new sequential IDs
      (0045+), status `proposed`, conforming to ADR-0030/0033/0034/0040.
- [ ] Each ported ADR's content matches its luminosity original except for
      documented minimal edits (names, repo-specific facts, paths) and rewritten
      cross-references.
- [ ] No internal cross-reference in a ported ADR points at a luminosity ADR
      number; all point at the assigned local numbers.
- [ ] For every identified overlap, a single superset ADR exists and the
      overlapped local ADR(s) carry a `superseded` status with a `superseded_by`
      edge to it (applied once the superset is accepted).
- [ ] Luminosity work items 0002 and 0003 are present in `meta/work/` under new
      local IDs with their research and outcomes intact.
- [ ] Each spike-fed ADR references the locally-ported spike as primary
      provenance and the luminosity original as a full-URL secondary reference.
- [ ] Every luminosity reference in the ported artifacts is a full GitHub URL,
      with any local path present only as a secondary aid.
- [ ] `mise run check` passes (frontmatter/schema and any meta-directory
      validations remain green).

## Open Questions

- Should the ported feeding spikes carry status `done` (a faithful historical
  record of work conducted in luminosity) or be reset to `draft`/`abandoned`
  since the spike effort did not occur in this repo? Draft assumes `done`.
- For ADR-0007 (Skills as the Product) and ADR-0008, is the existing Accelerator
  material substantial enough to constitute an ADR-level overlap requiring a
  superset, or should they be ported net-new? Resolved during the overlap pass.

## Dependencies

- Read access to luminosity at
  https://github.com/atomicinnovation/luminosity (local checkout at
  `../luminosity` may be used as a secondary convenience during implementation).

## Assumptions

- Accelerator is committed to (or seriously pursuing) the Rust-CLI migration
  direction these ADRs describe — backed by the staged migration-surface
  research. If that direction is not settled, the four spike-fed/CLI ADRs
  (0002, 0009, 0010, 0011) describe an architecture not yet adopted here, which
  would change whether they should be imported as `proposed` decisions or held.

## Technical Notes

- Local ADR allocation uses sequential identifiers (ADR-0029); spikes/work items
  are allocated via `work-item-next-number.sh`. Do not hand-pick numbers —
  allocate through the existing scripts so the sequence stays coherent.
- `/review-adr` is the sanctioned path for the accepted→superseded transition and
  enforces immutability (ADR-0031); the supersede edges cannot be applied while
  the superset ADR is still `proposed`.
- Luminosity ADR/spike frontmatter is already accelerator-derived, so schema
  drift should be minor — chiefly identity fields, dates, authorship, and
  linkage refs.

## Drafting Notes

- Kind set to `task` per the explicit request; the work is bounded porting with
  judgement, not feature delivery or open-ended research.
- The net-new vs overlap classification in the Context table is a *candidate*
  assessment from titles and known local ADRs; the implementer must verify each
  before deciding superset-vs-port-as-is.
- Numbering decision (renumber to 0045+ and rewrite refs), overlap decision
  (merge into superset + supersede prior local ADRs), spike decision (port the
  feeding spikes locally), and status decision (import as `proposed`) were all
  confirmed by the requester.
- Ported-spike status assumed `done` pending the Open Question above.

## References

- Source: luminosity ADRs — https://github.com/atomicinnovation/luminosity/tree/main/meta/decisions
- Source: luminosity feeding spikes — https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0002-modular-rust-cli-architecture-and-hexagonal-workspace-layout.md ,
  https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0003-skill-evaluation-framework-selection.md
- Related: this repo's `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- Related local ADRs touched by overlaps: ADR-0001, ADR-0016, ADR-0017, ADR-0027;
  conventions referenced: ADR-0029, ADR-0030, ADR-0031, ADR-0033, ADR-0034, ADR-0040
