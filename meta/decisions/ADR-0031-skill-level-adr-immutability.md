---
adr_id: ADR-0031
date: "2026-03-18T00:00:00+00:00"
author: Toby Clemson
status: accepted
tags: [adr, decisions, lifecycle, immutability, enforcement]
---

# ADR-0031: Skill-level ADR immutability

**Date**: 2026-03-18
**Status**: Accepted
**Author**: Toby Clemson

## Context

ADRs are conventionally treated as immutable after acceptance. Once a
decision has been ratified, the record of that decision should not
change, even if the project later moves in a different direction —
superseding ADRs document the new direction without overwriting the
historical record. Standard ADR tooling (Nygard, MADR, adr-tools) uses
a small lifecycle vocabulary: `proposed`, `accepted`, `rejected`,
`superseded`, `deprecated`. The plugin adopts that vocabulary and
needs to decide where to enforce the immutability rule that follows
from it: at the filesystem-write level (e.g. a `PreToolUse` hook
intercepting writes to accepted ADRs) or inside the skills that touch
ADRs.

## Decision Drivers

- Prevent accidental edits to historical decision records
- Avoid coupling enforcement to a specific tool runtime or harness
- Keep the enforcement mechanism transparent enough that authors
  understand why an edit was refused
- Accommodate the small set of edits that must still happen on
  non-`proposed` ADRs (status transitions, supersession metadata)

## Considered Options

1. **Hook-based filesystem enforcement** — A `PreToolUse` hook inspects
   target paths and frontmatter status and refuses writes to accepted
   ADRs. Strong guarantee: no skill can bypass it. Weak: couples
   enforcement to a specific tool/runtime, does not apply to edits made
   by other clients (IDE, shell, other agents), and rejects writes with
   a runtime error far from the skill flow that prompted them.
2. **Skill-level enforcement** — `review-adr` and other ADR skills
   inspect frontmatter `status` before editing and refuse to modify
   content when status is not `proposed`. Each transition writes a
   small set of associated metadata fields (`superseded_by` on
   supersession, `deprecated_reason` on deprecation, `rejected_reason`
   on rejection) atomically with the status change.
   Bypassable by direct edits outside the skills, but transparent in
   the skill prompt and decoupled from any runtime.

## Decision

Immutability is enforced at the skill level. Only `proposed` permits
content edits. Transitions out of `proposed` and out of `accepted` are
performed atomically by the ADR skills: the skill writes the new
`status` together with the reason field associated with that
transition (`rejected_reason` for rejection, `superseded_by` for
supersession, `deprecated_reason` for deprecation). Once written, no
further edits are permitted on non-`proposed` ADRs.

Permitted transitions:

| From       | To                  | Via                          |
|------------|---------------------|------------------------------|
| proposed   | accepted, rejected  | `review-adr`                 |
| accepted   | superseded          | `create-adr --supersedes`    |
| accepted   | deprecated          | `review-adr --deprecate`     |
| rejected   | (terminal)          | —                            |
| superseded | (terminal)          | —                            |
| deprecated | (terminal)          | —                            |

## Consequences

### Positive

- Enforcement is transparent: refusal happens inside the skill prompt,
  with a message explaining the rule and the recommended path (create
  a superseding ADR)
- No coupling to any specific tool runtime — the rule travels with the
  skill definition and is portable across harnesses
- The small set of metadata edits required for status transitions and
  supersession is accommodated without special-casing the enforcement
  mechanism

### Negative

- Direct file edits outside the ADR skills bypass enforcement entirely
- An accidental edit in an editor leaves no trace beyond VCS history

### Neutral

- New ADRs always start at `proposed`; extraction is discovery, not
  acceptance
- No write-side status mutation script exists; skills edit frontmatter
  directly. Adding such a script is deferred until the convention
  proves insufficient.
- The enforcement boundary is a stated trade-off: the population of
  editors is small and self-selecting (people working through the ADR
  skills), so a bypassable rule is sufficient — anyone editing an
  accepted ADR by hand is doing so knowingly, not by accident the
  rule was meant to catch

## References

- `meta/decisions/ADR-0029-sequential-adr-identifiers.md` — identifier
  scheme this lifecycle operates on
- `meta/decisions/ADR-0030-adr-template.md` — `status` field this
  lifecycle reads and writes
- `meta/decisions/ADR-0032-adr-skill-decomposition.md` — skills that
  enforce these rules
- `meta/research/codebase/2026-03-18-adr-support-strategy.md` —
  lifecycle and enforcement research
- `meta/plans/2026-03-18-adr-skills.md` — implementation plan
- `meta/work/0023-adr-system-design.md` — source work item
