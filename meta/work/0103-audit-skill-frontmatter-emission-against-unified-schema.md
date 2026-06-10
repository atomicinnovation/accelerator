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
last_updated: "2026-06-10T13:37:48+00:00"
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

## Discovery Pass Record

This section is the auditable artifact AC1/AC2 require: the re-runnable
discovery procedure, the producer set with emitted type(s), and the
per-(skill, type) conformance table. It is a **point-in-time snapshot** (audited
2026-06-10). The live authority on attribute completeness is the Phase 3 guard
(`scripts/test-skill-frontmatter-conformance.sh`), which derives the enforced
attribute set per type from the same contract files and asserts a synthesized
fixture exercises every attribute; a stale table here is expected as the schema
evolves, not a defect.

### 1. Discovery procedure (re-runnable)

Producing-skill discovery (run from repo root). The markers indicate a SKILL.md
that writes/substitutes a schema-governed frontmatter block:

```bash
grep -rlE 'schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh' \
  skills --include='SKILL.md' | sort -u
```

This returns **17 files** (verified). Reconciliation mirrors the population
test's `comm -23` discovery assertion: the discovered set, minus a recorded
`EXCLUDED` allowlist, must equal the `EMITTERS` allowlist; the status-axis-only
mutators are tracked separately because no full-block marker reaches them.

- **`EMITTERS` (16 full-block emitters)** — cross-checked against
  `skills-schema.tsv` rows 2–17:
  - **work/**: `create-work-item`, `extract-work-items`, `refine-work-item`,
    `review-work-item`
  - **planning/**: `create-plan`, `review-plan`, `validate-plan`
  - **decisions/**: `create-adr`, `extract-adrs`
  - **research/**: `research-codebase`, `research-issue`
  - **design/**: `inventory-design`, `analyse-design-gaps`
  - **github/**: `describe-pr`, `review-pr`
  - **notes/**: `create-note`
- **`EXCLUDED` (1, grep-surfaced but out of scope)**:
  `skills/config/migrate/SKILL.md` — the corpus transformer, governed
  differently (subtracted via the allowlist so `comm -23` is empty).
- **`STATUS_AXIS_ONLY` (not surfaced by the discovery grep; tracked by hand)**:
  `validate-plan`→`plan` (its *second* type; already an `EMITTERS` member for
  `plan-validation`) and `review-adr`→`adr`.

`update-work-item` (arbitrary field mutator) and `list-work-items` (consumer)
are out-of-scope by construction — neither carries a discovery marker, so
neither is surfaced.

Reconciliation result (verified 2026-06-10): discovery returns 17 files;
`comm -23 <(discovered) <(EMITTERS ∪ EXCLUDED)` is empty; `${#EMITTERS[@]} == 16`;
both status-axis mutators present.

### 2. Producer → emitted type(s)

| Skill | Emitted type(s) | Mode |
|-------|-----------------|------|
| `work/create-work-item` | `work-item` | full block |
| `work/extract-work-items` | `work-item` | full block |
| `work/refine-work-item` | `work-item` | full block |
| `work/review-work-item` | `work-item-review` | full block |
| `planning/create-plan` | `plan` | full block |
| `planning/review-plan` | `plan-review` | full block |
| `planning/validate-plan` | `plan-validation`; `plan` (status axis) | full block; status-axis mutation |
| `decisions/create-adr` | `adr` | full block |
| `decisions/extract-adrs` | `adr` | full block |
| `decisions/review-adr` | `adr` (status axis) | status-axis mutation |
| `research/research-codebase` | `codebase-research` | full block |
| `research/research-issue` | `issue-research` | full block |
| `design/inventory-design` | `design-inventory` | full block |
| `design/analyse-design-gaps` | `design-gap` | full block |
| `github/describe-pr` | `pr-description` | full block |
| `github/review-pr` | `pr-review` | full block |
| `notes/create-note` | `note` | full block |

### 3. Conformance table — universal base fields (all (skill, type))

Every full-block emitter composes its emission from three sources: **literal**
(verbatim in SKILL.md), **template** (the `config-read-template.sh <name>` slot
the skill loads), and **helper** (`artifact-derive-metadata.sh` /
author-resolution). The base-field block is sourced identically for every
producer, so it is factored out here and referenced by every per-type table in
§4:

| attribute | source | validator rule (diagnostic) |
|-----------|--------|-----------------------------|
| `type` | literal (fixed per type, also in template) | known schema type (`INVALID-TYPE`) |
| `id` | helper/skill (filename stem or `work-item-next-number.sh`), quoted | required base field + quoted (`MISSING-BASE-FIELD`, `UNQUOTED-ID`) |
| `title` | skill (composed) | required base field (`MISSING-BASE-FIELD`) |
| `date` | helper (`artifact-derive-metadata.sh`) | required base field + ISO timestamp (`MISSING-BASE-FIELD`, `BAD-TIMESTAMP`) |
| `author` | helper (VCS author resolution) | required base field (`MISSING-BASE-FIELD`) |
| `producer` | literal (skill name) | not validator-required; emitted by every producer |
| `tags` | template (slot) | required base field; `tags` exempt from omit-when-empty (`MISSING-BASE-FIELD`) |
| `last_updated` | helper | required base field + ISO timestamp (`MISSING-BASE-FIELD`, `BAD-TIMESTAMP`) |
| `last_updated_by` | helper (author resolution) | required base field (`MISSING-BASE-FIELD`) |
| `schema_version` | literal (`1`, bare integer) | required base field + bare `1` (`MISSING-BASE-FIELD`, `BAD-SCHEMA-VERSION`) |
| `status` | literal (per-type value) | in the type's `status_vocab` (`BAD-STATUS`) — see §4 per-type values |

### 4. Conformance table — per-type axes (status, extras, provenance, linkage)

`req extras` = type extras minus `FM_OPTIONAL_EXTRAS`
(`external_id reviewer pr_url merge_commit decision_makers work_item_id`);
`opt extras` are omit-when-empty. Anchored types additionally carry
`revision` + `repository` (source: **helper**). Forbidden own-id keys must be
absent (`FORBIDDEN-OWN-ID`). Typed-linkage values are quoted `"doc-type:id"`
(source: **template** slot, skill-filled; `BAD-LINKAGE-SHAPE`, +`DANGLING-REF`
in corpus mode). `git_commit`/`branch` are never emitted by any producer
(`FORBIDDEN-PROVENANCE`).

| type (skills) | anchored | status literal → vocab | req extras (source) | opt extras | linkage keys | forbidden own-id |
|---------------|----------|------------------------|---------------------|------------|--------------|------------------|
| `work-item` (create-work-item, extract-work-items, refine-work-item) | no | `draft` → ✓ | `kind` `priority` (literal/template) | `external_id` | `parent blocks blocked_by derived_from relates_to source` | `work_item_id` (absent ✓) |
| `plan` (create-plan) | yes | `draft` → ✓ | — | `reviewer` | `parent blocks blocked_by derived_from relates_to` | — |
| `plan` (validate-plan, status-axis mutation) | yes | **`complete` → ✗ BAD-STATUS** (fix → `done`) | — | — | — | — |
| `plan-validation` (validate-plan) | no | `complete` → ✓ | `result` (template slot, skill-filled) | — | `parent target relates_to` | — |
| `pr-description` (describe-pr) | yes | `complete` → ✓ | `pr_number` (template, bare int) | `pr_url` `merge_commit` | `parent relates_to` | `pr_title` (absent ✓) |
| `adr` (create-adr, extract-adrs) | no | `proposed` → ✓ | — | `decision_makers` | `parent supersedes relates_to` | `adr_id` (absent ✓) |
| `adr` (review-adr, status-axis mutation) | no | `accepted`/`superseded`/`deprecated` → ✓; **`rejected` → ✗ BAD-STATUS** (schema-source, → 0104) | — | — | — | — |
| `codebase-research` (research-codebase) | yes | `complete` → ✓ | `topic` (template/skill) | — | `parent relates_to` | — |
| `issue-research` (research-issue) | yes | `complete` → ✓ | `topic` (template/skill) | — | `parent relates_to` | — |
| `design-inventory` (inventory-design) | yes | `draft` → ✓ | `source source_kind source_location crawler sequence screenshots_incomplete` (template/skill) | — | `parent relates_to` | — |
| `design-gap` (analyse-design-gaps) | no | `draft` → ✓ | `current_inventory target_inventory` (template/skill) | — | `parent relates_to` | — |
| `plan-review` (review-plan) | no | `complete` → ✓ | `verdict lenses review_number review_pass` (template, skill-filled) | `reviewer` | `parent target relates_to` | — |
| `work-item-review` (review-work-item) | no | `complete` → ✓ | `verdict lenses review_number review_pass` (template, skill-filled) | `reviewer` `work_item_id` | `parent target relates_to` | — |
| `pr-review` (review-pr) | no | `complete` → ✓ | `verdict lenses review_number pr_number` (template, skill-filled) | `reviewer` | `parent target relates_to` | `pr_title review_pass` (absent ✓) |
| `note` (create-note) | yes | `captured` → ✓ | `topic` (template/skill) | — | `parent relates_to` | — |

### 5. Blind-spot axes (by inspection — not validator-checkable)

The validator is a partial oracle on two axes (see Context). These are audited
by inspection of the composed emission (skill literals + loaded template) and
will be folded into the validator under work item 0105, at which point this
by-inspection coverage collapses back to the single oracle.

- **Provenance over-emission on non-anchored types** — checked for every
  non-anchored type (`work-item`, `plan-validation`, `adr`, `design-gap`,
  `plan-review`, `work-item-review`, `pr-review`). **Result: clean.** None of
  their templates carry `revision`/`repository`, and `skills-schema.tsv` lists
  `revision repository` in `fields_to_assert` only for the six anchored
  producers (`create-plan`, `describe-pr`, `research-codebase`,
  `research-issue`, `inventory-design`, `create-note`). No producer over-emits
  provenance.
- **Bare/unquoted typed-linkage values** — checked for every type carrying
  linkage keys. **Result: clean.** Every template slot uses the quoted
  `"doc-type:id"` form (scalars) or quoted-element lists (`["plan:NNNN", ...]`);
  no producer documents a bare `parent: 0042` or path-shaped value. The scalar
  `work_item_id:` alias on plan / pr-description / research / work-item-review
  is a foreign-ref (in `FM_OPTIONAL_EXTRAS`), not a typed-linkage key, and is
  emitted quoted-or-omitted — not in scope for the linkage-shape axis.

### 6. Divergence triage

| # | Divergence | Source location | Classification | Disposition |
|---|------------|-----------------|----------------|-------------|
| 1 | `validate-plan` sets a passing **plan**'s `status` to `complete`, outside the plan vocab `draft\|ready\|in-progress\|done` | `validate-plan/SKILL.md:186-188` | **Producer-text** | Fixed in Phase 2 (`complete` → `done`). Its sibling `:161` correctly sets the **plan-validation** report to `complete` (that vocab is `complete`) — left unchanged. |
| 2 | `review-adr` persists `status: rejected` (+`rejected_reason`) per the accepted ADR-0031 lifecycle, but the `adr` `status_vocab` omits `rejected` | `review-adr/SKILL.md:85,192-201`; `templates-schema.tsv:6` | **Schema-source** | NOT fixed here. The producer correctly implements ADR-0031 (accepted); the TSV vocab is incomplete. Raised as child work item **0104** under epic 0057. Phase 3 guard represents this axis as a `skip_test` keyed to 0104. |

Rationale for #2 (schema-source, not producer): ADR-0031 (accepted)
explicitly adopts the lifecycle vocabulary `proposed, accepted, rejected,
superseded, deprecated` (`ADR-0031:28-29`) with `proposed → rejected` in its
transition table (`:75`) and a `rejected_reason` field (`:56,67`). `review-adr`
faithfully implements that lifecycle. The unified-schema TSV dropping `rejected`
contradicts an accepted ADR, so the correction belongs to the schema source, not
the producer.

A second schema-source follow-on (not a divergence, but a contract-coverage
gap) is raised as child work item **0105** under 0057: fold the two validator
blind spots (§5) into `validate-corpus-frontmatter.sh` so the Phase 3 guard's
bespoke checks collapse back to the single oracle.
