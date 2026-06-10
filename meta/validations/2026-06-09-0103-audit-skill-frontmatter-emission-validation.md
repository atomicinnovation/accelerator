---
type: plan-validation
id: "2026-06-09-0103-audit-skill-frontmatter-emission-validation"
title: "Validation Report: Audit Skill Frontmatter Emission Against the Unified Schema"
date: "2026-06-10T14:22:48+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-06-09-0103-audit-skill-frontmatter-emission"
target: "plan:2026-06-09-0103-audit-skill-frontmatter-emission"
relates_to: ["work-item:0103", "work-item:0104", "work-item:0105"]
tags: [frontmatter, schema, skills, validation, audit, test-harness]
last_updated: "2026-06-10T14:22:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Audit Skill Frontmatter Emission Against the Unified Schema

### Implementation Status

✓ Phase 1: Producer Enumeration and Per-Attribute Conformance Audit — Fully implemented
✓ Phase 2: Fix Confirmed Producer-Text Divergences — Fully implemented
✓ Phase 3: Automated Producer-Conformance Guard — Fully implemented

All three phases land as their own commits in the intended dependency order:

- `usypyktz` — Audit skill frontmatter emission and record the conformance findings (Phase 1)
- `qtkmmkrk` — Set a passing plan's status to done, not the out-of-vocab complete (Phase 2)
- `kxuymlpt` — Add an automated producer frontmatter-conformance guard (Phase 3)

### Automated Verification Results

✓ Discovery command re-runnable, returns 17 files:
  `grep -rlE 'schema_version:|Populate frontmatter|Substitute .*frontmatter|frontmatter-emission|artifact-derive-metadata\.sh' skills --include='SKILL.md' | sort -u | wc -l` → 17
✓ Conformance guard passes standalone: `bash scripts/test-skill-frontmatter-conformance.sh` → **129 passed, 1 skip, 0 failed** (matches plan's claim)
✓ Integration config suite green and floor tracks the new gate: `mise run test:integration:config` → **25 passed, 0 failed**; `_EXPECTED_CONFIG_SUITES = 16`
✓ Templates unit suite stays green: `mise run test:unit:templates` → **36 passed, 0 failed**
✓ Corpus validator clean on the 0103 work item, the 0104/0105 child items, and the full `meta/` corpus (all rc=0)
✓ By-name CI wiring enforced: `_REQUIRED_CONFIG_SUITES = ("scripts/test-skill-frontmatter-conformance.sh",)` with a fail-closed `missing` check (`tasks/test/integration.py:57-64`)
✓ Negative/liveness test proves wiring: reverting `validate-plan:187` `done → complete` produced **2 failures** on the `validate-plan -> plan` status axis (`status 'complete' ∈ plan vocab (rc=1, expected 0)`), restored cleanly

### Code Review Findings

#### Matches Plan:

- **Phase 2 producer fix** — `skills/planning/validate-plan/SKILL.md:187` sets a passing plan's status to `done`; sibling `:161` retains `complete` for the plan-validation report. Both sites match the plan exactly.
- **Phase 3 guard** — `scripts/test-skill-frontmatter-conformance.sh` exists, is executable, sources `frontmatter-emission-rules.sh` and reads `templates-schema.tsv` (no re-encoded contract, asserted in-suite). Covers composed-emission acceptance, both blind-spot axes (provenance over-emission, bare/unquoted linkage) with liveness + control cases, and the four-axis mutation self-test (type, status, extra, schema_version) each asserting its specific diagnostic.
- **Fixture-helper refactor** — `emit_valid`/`assert_*` factored into `scripts/frontmatter-fixtures.sh`, sourced by **both** `test-validate-corpus-frontmatter.sh` and `test-skill-frontmatter-conformance.sh` (the two suites are peers sharing one fixture authority, as designed). The existing validator suite stays green.
- **Phase 1 audit artifact** — the work item carries a complete "Discovery Pass Record" with the re-runnable discovery command, the `EMITTERS`(16)/`EXCLUDED`(1)/`STATUS_AXIS_ONLY`(2) reconciliation, and the per-(skill, type) conformance table.
- **Divergence triage** — both schema-source items raised as child work items under epic 0057: `0104` (add `rejected` to ADR status vocab) and `0105` (close validator provenance/linkage blind spots). Both `parent: "work-item:0057"`, `relates_to: ["work-item:0103"]`, `status: draft` — recorded, not fixed here, per scope.
- **Deferred-divergence handling** — the guard represents the deferred ADR `rejected` axis as an explicit `skip_test` keyed to `0104` (`scripts/test-skill-frontmatter-conformance.sh:336`), which accounts for the single skip — visible in test output, flips to a live assertion when 0104 lands, exactly as the plan specified.

#### Deviations from Plan:

- The plan's success criteria cited `15 → 16` config suites and an integration run of "15 suites"; the actual `test:integration:config` now reports **25 passed**. This reflects suites added/counted since the plan was authored — the floor (`_EXPECTED_CONFIG_SUITES = 16`) and the by-name gate both hold, so the intent (the guard is discovered and fail-closed) is satisfied. Not a defect; a stale headline number in the plan text.

#### Potential Issues:

- None blocking. The guard's two blind-spot checks duplicate authority the validator does not yet hold; this is intentional and tracked back to a single oracle via the `0105` child item, with in-script comments naming it (`:23-24`, `:327`).

### Manual Testing Required:

1. Guard liveness (already exercised during validation):
  - [x] Reverting the `validate-plan` plan-status literal to `complete` fails the guard with a plan `BAD-STATUS`-class diagnostic
  - [x] Restoring the literal returns the guard to green

2. bash 3.2 safety:
  - [x] Plan records both the guard and refactored validator suite passing under `/bin/bash` 3.2.57 on macOS; extraction anchors on ASCII tokens under `LC_ALL=C`. (Accepted from plan evidence; not re-run in this session.)

### Recommendations:

- Update the plan's "15 suites" headline numbers (Phase 2/3 ACs and Testing Strategy) to reflect the current count if the plan text is retained for reference — purely cosmetic.
- Prioritise child work item `0105` so the guard's bespoke blind-spot checks can collapse back into `validate-corpus-frontmatter.sh`, retiring the temporary three-authority state the plan flagged.
