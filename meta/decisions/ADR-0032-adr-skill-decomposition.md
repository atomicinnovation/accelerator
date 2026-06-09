---
id: "ADR-0032"
date: "2026-03-18T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [adr, decisions, skills, decomposition]
type: adr
title: "ADR-0032: ADR skill decomposition"
schema_version: 1
last_updated: "2026-03-18T00:00:00+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0029", "adr:ADR-0030", "adr:ADR-0031", "codebase-research:2026-03-18-adr-support-strategy", "plan:2026-03-18-adr-skills", "work-item:0023"]
---

# ADR-0032: ADR skill decomposition

**Date**: 2026-03-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

Users interact with ADRs in three distinct ways: authoring a new decision
interactively, mining decisions out of existing documents (research
notes, design docs, retrospectives) in batch, and reviewing a proposed
ADR to accept, reject, or deprecate it. Each interaction has a different
flow, audience, and output. The plugin needs to decide whether to surface
this as a single skill with sub-commands or as several focused skills.

## Decision Drivers

- Match skill boundaries to distinct user intents
- Reuse the existing sub-agent ecosystem (`documents-locator`,
  `documents-analyser`, `codebase-locator`) for context gathering rather
  than introducing ADR-specific agents
- Keep each skill's prompt focused and small so it stays maintainable
- Align skill boundaries with the lifecycle boundaries defined by
  ADR-0031 (creation vs transitions)

## Considered Options

1. **Single ADR skill with mode flags** (e.g. `--create`, `--extract`,
   `--review`) — one entry point and one body of code, so template
   parsing, frontmatter handling, and status checks live in one place
   and template drift is impossible by construction. The cost is that
   a single prompt has to cover three different flows, audiences, and
   outputs, which conflicts with the focused-prompt driver.
2. **Two skills, lifecycle-aligned** — an author skill covering
   creation, extraction, and supersession (everything that produces a
   `proposed` ADR) and a review skill covering transitions out of
   `proposed`. Matches ADR-0031's lifecycle boundary cleanly. The
   trade-off is that the author skill still has to cover two very
   different *flows*: interactive single-decision authoring versus
   batch mining from existing documents, with different inputs,
   pacing, and outputs.
3. **Three skills, intent-aligned** — `create-adr`, `extract-adrs`,
   `review-adr`. Each handles one intent end-to-end and delegates
   context gathering to the existing sub-agents. Each prompt stays
   focused on a single flow.

## Decision

Three skills, one per intent:

- **`create-adr`** — interactive authoring of a single new ADR. Sole
  authority for the template defined by ADR-0030. Handles supersession
  via `--supersedes`.
- **`extract-adrs`** — batch mining of decisions from existing
  documents into draft ADRs. All extracted ADRs are created at status
  `proposed`; acceptance is a separate, explicit step.
- **`review-adr`** — quality review of `proposed` ADRs and lifecycle
  transitions for non-`proposed` ADRs (`--deprecate`).

All three delegate context gathering to the existing sub-agent
ecosystem rather than introducing ADR-specific agents.

The intent-aligned split is preferred over the lifecycle-aligned
two-skill alternative because creation and extraction differ more in
flow (interactive single vs batch mining) than they share in output
state. Co-locating them under one prompt would force one of the two
flows to dominate the prompt's shape.

## Consequences

### Positive

- Each skill's prompt covers a single flow and audience, keeping
  prompts focused and easier to evolve
- Reuse of existing sub-agents — no new agents required
- Skill boundaries match the lifecycle boundaries enforced by
  ADR-0031: `create-adr` produces `proposed`, `review-adr` transitions
  out of `proposed`

### Negative

- Template authority concentrated in `create-adr` means `extract-adrs`
  must track the template by convention rather than by shared code;
  drift is possible
- Three skill prompts to maintain instead of one

### Neutral

- Supersession lives in `create-adr` (via `--supersedes`), not
  `review-adr` — review handles acceptance, rejection, and
  deprecation; supersession is authoring a new decision that points
  back at the old one
- Extraction always yields `proposed` ADRs; acceptance is always a
  separate, explicit step via `review-adr`
- Template-drift mitigation between `create-adr` and `extract-adrs`
  is deferred; the convention-tracking risk is accepted as residual
  for now rather than addressed by a shared template script or lint

## References

- `meta/decisions/ADR-0029-sequential-adr-identifiers.md` — identifier
  scheme allocated by `create-adr`
- `meta/decisions/ADR-0030-adr-template.md` — template owned by
  `create-adr`
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` —
  lifecycle rules enforced by these skills
- `meta/research/codebase/2026-03-18-adr-support-strategy.md` — skill
  decomposition research
- `meta/plans/2026-03-18-adr-skills.md` — implementation plan
- `meta/work/0023-adr-system-design.md` — source work item
