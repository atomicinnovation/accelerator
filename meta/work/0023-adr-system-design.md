---
title: "ADR system design"
type: adr-creation-task
status: ready
---

# ADR Ticket: ADR system design

## Summary

In the context of adding architecture decision record management, we decided
for sequential `ADR-NNNN-description.md` numbering in `meta/decisions/`, a
hybrid Nygard+MADR template with YAML frontmatter, an append-only lifecycle
with skill-level enforcement (only `proposed` allows content edits), and three
distinct skills (create, extract, review) delegating to existing sub-agents, to
achieve unambiguous ordering, structured machine-parseable records, transparent
enforcement, and separation of concerns aligned with user intents, accepting
divergence from date-prefixed naming, that direct edits bypass protection, and
template authority residing solely in `create-adr`.

## Context and Forces

- The plugin lacked formal decision documentation despite making numerous
  architectural decisions
- Industry-standard ADR practices (Nygard, MADR, adr-tools) provide proven
  templates and conventions
- The `meta/` directory uses date-prefixed naming for research and plans, but
  ADRs need stable identifiers for supersession chains
- ADR immutability after acceptance is an industry best practice
- Enforcement could happen at the hook level (PreToolUse intercepting file
  writes) or the skill level
- Three distinct user intents exist: creating new ADRs, extracting decisions
  from existing documents, and reviewing/accepting ADRs
- The plugin already has sub-agents (documents-locator, documents-analyser,
  codebase-locator) that ADR skills can delegate to

## Decision Drivers

- Unambiguous ordering and stable cross-references (supersession chains)
- Machine-parseable records (YAML frontmatter) consistent with other artifacts
- Immutability after acceptance as an ADR best practice
- Simple, transparent enforcement over infrastructure-level interception
- Alignment with existing plugin patterns (sub-agent delegation, filesystem
  artifacts)
- Separation of concerns: create vs extract vs review are distinct user intents

## Considered Options

For numbering:
1. **Date-prefixed** — Consistent with research/plans. But unstable for
   cross-references (dates don't convey order unambiguously).
2. **Sequential ADR-NNNN** — Industry standard. Stable cross-references. Date
   captured in frontmatter instead.

For template:
1. **Nygard original** — Minimal: Context, Decision, Status, Consequences.
   Concise but lacks structure.
2. **MADR full** — Comprehensive with multiple variants. Too complex.
3. **Hybrid Nygard + MADR** — Nygard's conciseness plus MADR's Considered
   Options and Decision Drivers, with YAML frontmatter for machine parseability.

For enforcement:
1. **Hook-based** — PreToolUse intercepts file writes to accepted ADRs. Fragile
   file-write interception.
2. **Skill-level** — Skills check status before editing. Only `proposed` allows
   content edits; other statuses permit only status-field transitions. Simpler
   and more transparent.

For skill decomposition:
1. **Single ADR skill** — One skill handles creation, extraction, and review.
   Too broad.
2. **Three skills** — `create-adr` (interactive generation), `extract-adrs`
   (mining existing documents), `review-adr` (quality review and status
   transitions). Each delegates context gathering to existing sub-agents.

## Decision

We will store ADRs as `meta/decisions/ADR-NNNN-description.md` with sequential
numbering. The template is a hybrid of Nygard and MADR: Context, Decision
Drivers, Considered Options, Decision, Consequences (Positive/Negative/Neutral),
References, with YAML frontmatter (`adr_id`, `status`, `supersedes`,
`superseded_by`, `tags`). Enforcement is skill-level: only `proposed` ADRs
allow content edits. Three skills handle distinct intents: `create-adr`,
`extract-adrs`, and `review-adr`, all delegating to existing sub-agents.

## Consequences

### Positive
- Sequential numbering gives unambiguous ordering and stable supersession chains
- Hybrid template balances conciseness with structure
- Skill-level enforcement is simpler and more transparent than hook interception
- Three skills align with distinct user intents
- Reuses existing sub-agent ecosystem (no new agents needed)
- Template authority in `create-adr` prevents duplication drift

### Negative
- Diverges from date-prefixed naming convention used by research/plans
- Direct file edits outside ADR skills bypass enforcement
- Template changes must originate in `create-adr` and be referenced by
  `extract-adrs` by convention
- Numbers are never reused — gaps may appear from skipped decisions

### Neutral
- New ADRs always start with status `proposed` — extraction is discovery, not
  acceptance
- ADR lifecycle extends the plugin's decisions category as a peer to
  research-plan-implement

## Source References

- `meta/research/codebase/2026-03-18-adr-support-strategy.md` — ADR strategy research
  covering numbering, templates, lifecycle, and enforcement
- `meta/plans/2026-03-18-adr-skills.md` — Implementation plan for three ADR
  skills with companion scripts
