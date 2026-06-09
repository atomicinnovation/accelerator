---
type: work-item
id: "0103"
title: "Audit Skill Frontmatter Emission Against the Unified Schema"
date: "2026-06-09T14:13:02+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: task
priority: medium
parent: "work-item:0057"
relates_to: ["work-item:0070"]
tags: [frontmatter, schema, skills, validation, audit]
last_updated: "2026-06-09T18:09:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0103: Audit Skill Frontmatter Emission Against the Unified Schema

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Audit every Accelerator skill that writes artifact frontmatter and confirm the
frontmatter it emits conforms to the unified schema on every axis the corpus
validator enforces — not just `status`. Fix any divergence at the producing
skill.

## Context

The 0070 corpus migration unified frontmatter and shipped a corpus validator
(`scripts/validate-corpus-frontmatter.sh`) plus a shared emission-rules helper
(`scripts/frontmatter-emission-rules.sh`), with per-type facts in
`scripts/templates-schema.tsv`. But only the *corpus* was migrated and
validated — the *producers* (skills) were never audited against that same
contract. They were already conforming "by inspection", which is exactly how
drift hides.

This surfaced when `/validate-plan` set a passing plan's `status` to `complete`
(`SKILL.md:187`) — outside the plan vocab `draft | ready | in-progress | done`.
Of the two artifacts validate-plan writes, `complete` is valid for its own
`plan-validation` report but not for the `plan` (ADR-0042 maps plan
`complete → done`). Note `plan-validation` — the report validate-plan emits — is
distinct from the `plan-review` type ADR-0042 reconciles; several types carry
`complete` in their vocab, so the constraint is per-type, not "`complete` is
plan-validation-only". The validator caught it only after the file landed in the
corpus, not at emission. Status is one axis; the validator enforces many
(required base fields, quoted `id`, `schema_version: 1` as a bare integer,
provenance bundle iff `code_state_anchored`, no `git_commit`/`branch`, forbidden
own-id keys, per-type extras, omit-when-empty, typed-linkage `"doc-type:id"`
shape, ISO timestamps). Any producer could drift on any of them. This audit
closes the producer-side gap the migration left open.

## Requirements

- Enumerate every skill that writes/substitutes artifact frontmatter, using a
  determinate membership rule: any `SKILL.md` whose instructions write or
  substitute YAML frontmatter into a produced artifact. Derive the set
  mechanically (e.g. `grep -rl` for frontmatter-substitution instructions across
  `skills/`) and record the discovery command so the enumeration is re-runnable
  rather than approximate; as of writing this is ~18 files across `work/`,
  `planning/`, `decisions/`, `research/`, `design/`, `github/`, `notes/`.
- For each producing skill, cross-check its emission instructions against the
  full validator contract for the type(s) it writes:
  - required base fields present; `id` quoted; `schema_version: 1` bare integer;
    `date`/`last_updated` in ISO-timestamp form;
  - `status` (when emitted) in the type's `status_vocab`;
  - provenance bundle (`revision`/`repository`) present iff
    `code_state_anchored=yes`; `git_commit`/`branch` never emitted;
  - the type's `forbidden_own_id_key` never emitted; per-type `extras` present;
  - omit-when-empty (no empty `parent: ""`/`blocks: []` placeholders written);
  - typed-linkage values in `"doc-type:id"` form, never bare/path-shape.
- Fix each divergence in the producing skill's text (e.g. validate-plan
  plan-status `complete → done`), keeping per-type facts sourced from /
  consistent with `templates-schema.tsv` rather than hard-coded divergently.
  Fixes are confined to producing-skill text; any divergence whose root is the
  schema source (`templates-schema.tsv` or `frontmatter-emission-rules.sh`) is
  recorded and raised as a child work item under epic 0057 rather than fixed
  under this work item.
- Be precise about *which* type a skill is writing: e.g. validate-plan
  legitimately emits `status: complete` for its own `plan-validation` report
  (`SKILL.md:161`) while wrongly emitting it for the `plan` (`:187`).
- Add an automated producer-conformance guard that asserts each
  frontmatter-writing skill's emission against the contract and confirms it
  passes `scripts/validate-corpus-frontmatter.sh`, modelled on the existing
  `scripts/test-skill-frontmatter-population.sh`. The guard must verify the value
  the skill actually emits — drive the skill over a fixture where practical; the
  documented-emission mode is acceptable only where the emitted value is a
  verbatim literal in `SKILL.md`, so that corrupting the literal still trips the
  negative test. Wire it into the appropriate `test:integration:*` task so
  future skills cannot drift undetected.

## Acceptance Criteria

- [ ] The frontmatter-writing skill set is enumerated by the re-runnable
      discovery procedure defined in Requirements, and each skill is listed with
      the type(s) it produces.
- [ ] For each skill, a per-attribute conformance table maps every emitted
      frontmatter attribute to the validator rule it satisfies (or the fix
      applied); each mismatch is recorded and fixed. The table is complete when
      its attribute set for each type equals the attribute set the validator
      enforces for that type (derived from `templates-schema.tsv` /
      `frontmatter-emission-rules.sh`), so completeness is mechanically
      checkable rather than judged by inspection.
- [ ] `validate-plan` sets a passing plan's status to `done`, and still sets its
      validation report's own status to `complete`.
- [ ] For each audited producer, the emission(s) exercising every conditional
      axis it can emit (e.g. anchored vs non-anchored provenance, with vs without
      typed-linkage, omit-when-empty branches) pass
      `scripts/validate-corpus-frontmatter.sh` with zero diagnostics.
- [ ] An automated producer-conformance test exists, is discovered by a
      `test:integration:*` task, and fails when a skill is made to emit an
      out-of-contract attribute (proving the guard is wired, not green-path
      only).
- [ ] `mise run test:integration:config` stays green.

## Open Questions

- None outstanding.

## Dependencies

- Blocked by: none (validator, emission-rules helper, and schema shipped via
  0070).
- Internal ordering: the automated conformance guard depends on the producer
  enumeration (first requirement) being settled — build the guard against the
  audited set, not in parallel with discovery.
- Integrates with: the existing `test:integration:*` task wiring and the
  `scripts/test-skill-frontmatter-population.sh` harness pattern — the shared
  test-infrastructure seam where this work is most likely to collide with other
  test-suite changes.
- Downstream (conditional): schema-source divergences surfaced by the audit are
  raised as child work items under epic 0057; if producer-side divergences prove
  large enough to warrant independent delivery, the guard and/or per-skill fixes
  split into sibling work items (see Drafting Notes). The conformance guard's
  final scope is contingent on the settled audited set.
- Relates to: 0070 (migration that introduced the validator), ADR-0042 (status
  reconciliation map).

## Assumptions

- The corpus validator (`validate-corpus-frontmatter.sh`) is the authoritative
  oracle for what conforming frontmatter is; the audit measures producers
  against it rather than re-deriving the contract.

## Technical Notes

- Per-type facts (extras, status vocab, code-state-anchoring, forbidden own-id
  key, typed-linkage keys) live in `scripts/templates-schema.tsv`; cross-cutting
  emission rules live in `scripts/frontmatter-emission-rules.sh`. Prefer
  asserting producer output against these rather than encoding a parallel copy
  of the contract in the new test.
- `scripts/test-skill-frontmatter-population.sh` already validates SKILL.md
  guidance and is the natural precedent for the conformance guard.

## Drafting Notes

- Broadened from a status-only audit (the originally-requested framing) to all
  frontmatter attributes, since `status` is just the one axis that leaked and
  the validator enforces the full contract — a status-only fix would leave the
  same class of bug latent on every other axis.
- Promoted the automated drift-guard from an open question to a requirement and
  acceptance criterion at the author's direction.
- `task`, not `spike`: the deliverable is a concrete fix plus a guard, not an
  investigation.
- "The epic" interpreted as 0057 (the unified-frontmatter epic that 0070
  closes).
- Audit-and-fix and the conformance guard are deliberately kept as one task:
  the guard asserts exactly what the audit establishes, and only validate-plan's
  plan-status is a confirmed divergence today. If the audit surfaces divergences
  large enough to warrant independent delivery, split the guard (or per-skill
  fixes) into sibling work items at that point.

## References

- Source: corpus-validator sanity-check failure during `/validate-plan` of work
  item 0070
- Related: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`,
  `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md`,
  `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md`
- Contract surfaces: `scripts/validate-corpus-frontmatter.sh`,
  `scripts/frontmatter-emission-rules.sh`, `scripts/templates-schema.tsv`,
  `scripts/test-skill-frontmatter-population.sh`
- Offending instruction: `skills/planning/validate-plan/SKILL.md:187`
