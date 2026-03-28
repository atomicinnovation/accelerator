---
title: "Artifact metadata, cross-referencing, and lifecycle transitions"
type: adr-creation-task
status: todo
---

# ADR Ticket: Artifact metadata, cross-referencing, and lifecycle transitions

## Summary

In the context of multiple artifacts lacking machine-parseable metadata and no
automated lifecycle management, we decided for a common base YAML frontmatter
schema (`date`, `type`, `skill`, `status`) extended per artifact type, with
downstream skills loading prior artifacts (e.g., `respond-to-pr` reading review
artifacts for triage enrichment) and passing validation automatically closing
plan status to `complete`, to achieve uniform machine-parseability, inter-skill
communication via filesystem artifacts, and automated lifecycle closure.

## Context and Forces

- Research documents had frontmatter; plans and PR descriptions did not
- Without consistent metadata, automated tooling (status queries, filtering,
  lifecycle tracking) is impossible
- Skills produce artifacts that are consumed by other skills, but there was no
  standardised way to discover and read these artifacts
- Plan lifecycle had no automated transitions — plans stayed in draft/active
  status even after successful validation
- `respond-to-pr` could benefit from review severity ratings and lens context
  but had no access to review artifacts

## Decision Drivers

- Uniform machine-parseability across all artifact types
- Enable inter-skill communication via filesystem artifacts
- Automated lifecycle transitions to reduce manual status tracking
- Consistent data contract that downstream tools can depend on

## Considered Options

1. **No frontmatter convention** — Each skill defines its own metadata. No
   consistency.
2. **Common base schema** — Shared base fields (`date`, `type`, `skill`,
   `status`) extended per artifact type. Downstream skills can reliably parse
   any artifact.
3. **External metadata store** — Database or JSON index file. Over-engineered;
   duplicates what YAML frontmatter provides naturally.

## Decision

We will adopt a common base YAML frontmatter schema with `date`, `type`,
`skill`, and `status` fields, extended per artifact type with type-specific
fields. Downstream skills will load prior artifacts to enrich their work (e.g.,
`respond-to-pr` loads the latest review artifact for triage). When
`validate-plan` produces `result: pass`, it updates the plan's frontmatter
`status` to `complete`, creating automated lifecycle closure.

## Consequences

### Positive
- Uniform machine-parseability across all artifact types
- Inter-skill communication pattern established (filesystem artifacts as shared
  state)
- Automated lifecycle closure reduces manual status tracking
- Consistent data contract for future tooling

### Negative
- Coupling between validation and plan artifacts
- Retrofitting frontmatter to existing artifacts requires migration
- Downstream skills must handle missing artifacts gracefully (graceful
  degradation when no review artifact exists)

### Neutral
- The existing `research-codebase` frontmatter predates this convention and
  retrofitting is deferred
- Automated lifecycle transitions are one-directional (validation → complete)

## Source References

- `meta/plans/2026-03-22-validation-crossref-frontmatter.md` — Frontmatter
  convention, cross-referencing design, and lifecycle transition logic
