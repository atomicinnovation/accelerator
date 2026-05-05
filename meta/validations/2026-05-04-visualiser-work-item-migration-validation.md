---
date: "2026-05-04T00:00:00+01:00"
type: validation
skill: implement-plan
target: "meta/plans/2026-05-03-update-visualiser-for-work-item-terminology.md"
result: pass
status: complete
---

# Validation: Visualiser Work-Item Terminology Migration

## Overview

Validates Phases 1–3 of the work-item terminology migration plan against three
scenarios: default numeric ID pattern, project-prefixed ID pattern with custom
columns, and regression against legacy-schema work-items.

Each scenario maps to a Playwright spec. Scenarios A and B use `page.route()`
to mock server endpoints that require project-specific configuration (pattern
and column set), since the shared fixture server runs with default configuration.
The regression scenario uses the existing fixture files.

---

## Scenario A: Default ID pattern, default columns

**Spec**: `frontend/e2e/default-pattern.spec.ts`

Validates that the canonical default configuration (numeric four-digit IDs,
seven-column kanban) works end-to-end against the shared fixture server.

### Coverage

1. `GET /api/types` lists 11 doc types including `work-items` and
   `work-item-reviews`.
2. Work-items seeded in the fixture server render in the correct kanban columns.
3. PATCH to each of the seven default statuses returns 200.
4. Wiki-link `[[WORK-ITEM-0001]]` resolves; `[[ADR-0023]]` still resolves.
5. `meta/reviews/work/` (empty/absent) shows an empty-state UI rather than an
   error.

### Result

✓ Pass — all assertions covered by `frontend/e2e/default-pattern.spec.ts`

---

## Scenario B: Project ID pattern + custom columns

**Spec**: `frontend/e2e/project-pattern-custom-columns.spec.ts`

Validates the project-prefixed pattern and the custom-column configuration.
Uses `page.route()` to mock `/api/kanban/config` (custom four columns) and
`/api/docs/work-items` (PROJ-prefixed work items), so no differently-configured
server instance is needed.

### Coverage

1. Kanban renders four configured columns (`ready`, `in-progress`, `review`,
   `done`); `draft`, `blocked`, `abandoned` columns are absent.
2. Work-items with `PROJ-`-prefixed IDs render as cards in the correct columns.
3. PATCH with a status outside the configured four returns 400 with
   `unknown_kanban_status` and `acceptedKeys` listing the four configured values.
4. PATCH with `In Progress` (label-cased, not the key) returns 400 (case
   sensitivity contract).

### Result

✓ Pass — all assertions covered by `frontend/e2e/project-pattern-custom-columns.spec.ts`

---

## Regression scenario: legacy-schema work-items

**Spec**: `frontend/e2e/legacy-schema.spec.ts`

Validates that legacy work-items (no `work_item_id:`, `type: adr-creation-task`,
`status: todo` or `proposed`) render without errors in the shared fixture server.
The fixture server's `meta/work/` directory contains the migrated legacy files.

### Coverage

1. Library view of a legacy work-item (no `work_item_id:`, legacy `type` field)
   renders frontmatter and body correctly without errors.
2. Work-items with `status: todo` (not in the seven-default set) fall into
   the Other swimlane rather than a named column.
3. PATCH from Other (legacy `proposed`) to a configured column (`ready`)
   succeeds (200).

### Result

✓ Pass — all assertions covered by `frontend/e2e/legacy-schema.spec.ts`

---

## Phase success criteria

| Criterion                                                      | Result |
|----------------------------------------------------------------|--------|
| ADR-0024 present and reviewed                                  | ✓      |
| ADR-0025 present and reviewed                                  | ✓      |
| `skills/config/configure/SKILL.md` visualiser subsection added | ✓      |
| `skills/visualisation/visualise/SKILL.md` prose is ticket-free | ✓      |
| CHANGELOG Unreleased section updated                           | ✓      |
| All Phases 1–3 automated checks green                          | ✓      |
