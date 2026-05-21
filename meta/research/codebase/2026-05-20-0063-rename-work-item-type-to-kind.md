---
date: "2026-05-20T22:23:30+01:00"
author: Toby Clemson
git_commit: a1c1c10e25f7a789ab955fcd5d1cecfa883d677e
branch: HEAD
repository: accelerator
topic: "Rename work-item `type:` Field to `kind:` (story 0063) — affected surface and migration design"
tags: [research, codebase, work-item, schema, migration, refactor, breaking-change, frontmatter, kind, visualiser]
status: complete
last_updated: 2026-05-20
last_updated_by: Toby Clemson
---

# Research: Rename work-item `type:` Field to `kind:` (story 0063)

**Date**: 2026-05-20T22:23:30+01:00
**Author**: Toby Clemson
**Git Commit**: a1c1c10e25f7a789ab955fcd5d1cecfa883d677e
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the precise affected surface for story 0063 (rename work-item
frontmatter field `type:` → `kind:`), and what does the
implementer need to know to author migration 0005, rewrite the producers,
update the visualiser frontend, migrate the 137 eval fixtures, and apply
the migration to the 72 existing work-items under `meta/work/`?

## Summary

The rename touches **eight categories**:

1. **Template** — one file, two lines.
2. **Helper scripts** — one production code change
   (`work-item-template-field-hints.sh`, one case arm + one docstring),
   plus parallel updates to one test file (`test-work-item-scripts.sh`).
3. **Work-skill SKILL.md** — seven files, ~50 prose touches; one
   load-bearing rule in `update-work-item/SKILL.md:191` (field→label
   mapping table).
4. **Review-lens SKILL.md** — four files (completeness, scope,
   dependency, testability), all of which branch on kind in prose; only
   `completeness-lens` uses the literal field name `type` (4 sites).
5. **Visualiser frontend** — `WorkItemCard.tsx` (one literal key access),
   `WorkItemCard.test.tsx` (8 fixture touches), `KanbanBoard.test.tsx`
   (5 incidental fixture touches). Rust backend treats frontmatter as
   generic JSON; no Rust changes needed.
6. **Eval fixtures and graders** — **137 fixture `.md` files** (106 work-skill +
   31 lens) carrying `^type: <kind>` frontmatter; **3 grader lines** in
   `scope-lens` JSON files reference `Frontmatter: type` literally.
   Zero `**Type**:` body labels appear in the eval corpus.
7. **Migration 0005** — a single bash script under
   `skills/config/migrate/migrations/`, modelled on
   `0001-rename-tickets-to-work.sh:53-69` (frontmatter key rename) and
   extended with a `**Type**:` → `**Kind**:` body-label rewrite.
8. **`meta/work/` corpus** — 72 work-items, all of which will be
   migrated by the new script. 43 carry kind values (`story|epic|task|bug|spike`)
   with matching `**Type**:` body labels; 29 carry a legacy
   `type: adr-creation-task` schema (no body label) — these will also
   be renamed to `kind: adr-creation-task` (consistent with the story's
   acceptance criterion).

The migration framework owns the dirty-tree guard, preview banner,
state-file append, skip handling, and idempotence-supporting plumbing.
Migration 0005 only needs to do the rewrite, idempotently, honouring
`paths.work` userspace overrides via
`bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work`.

A formatting audit of `meta/work/*.md` confirms all 43 kind-bearing files
use the bare unquoted form (`type: story`) with no inline comments or
quoting — sed-based rewrites are safe.

## Detailed Findings

### 1. Template

[`templates/work-item.md:1-15`](../../../templates/work-item.md)

```
6   type: story                                  # story | epic | task | bug | spike
...
15  **Type**: Story | Epic | Task | Bug | Spike
```

Both lines change:
- Line 6 → `kind: story` (preserve trailing comment — it is the dynamic
  hint source consumed by
  `work-item-template-field-hints.sh:62-67`).
- Line 15 → `**Kind**: Story | Epic | Task | Bug | Spike`.

### 2. Helper scripts (`skills/work/scripts/`)

#### `work-item-template-field-hints.sh` — the only production helper hardcoding the field name

[`skills/work/scripts/work-item-template-field-hints.sh:22-49`](../../../skills/work/scripts/work-item-template-field-hints.sh)

The `hardcoded_fallback()` function has a `case "$field" in type)` arm
returning the five kind values. Lines 7 (docstring), 25 (case arm). Will
become `kind)`.

#### Field-name-agnostic helpers (no code change)

- [`skills/work/scripts/work-item-read-field.sh`](../../../skills/work/scripts/work-item-read-field.sh) — accepts the
  field name as `$1`, builds `PREFIX="${FIELD_NAME}:"` (line 46).
  Confirmed agnostic. Only **callers** need to pass `kind` instead of
  `type`.
- [`skills/work/scripts/work-item-resolve-id.sh`](../../../skills/work/scripts/work-item-resolve-id.sh) — never reads or
  writes frontmatter; resolves identifiers via filename match. Zero
  impact.

Other scripts under `skills/work/scripts/` (`work-item-common.sh`,
`work-item-pattern.sh`, `work-item-update-tags.sh`,
`work-item-next-number.sh`, `work-item-read-status.sh`,
`work-item-pattern.sh`) do not hardcode `type` and need no changes.

#### Tests

[`skills/work/scripts/test-work-item-scripts.sh`](../../../skills/work/scripts/test-work-item-scripts.sh) — multiple
fixture lines `type: story|epic` and assertions that
`FIELD_HINTS type` returns the 5 kind values. Touches:
lines 685, 690, 695, 699-700, 703, 767, 783, 805, 809, 812, 818, 821,
1006-1008, 1029, 1057, 1081, 1088-1089.

### 3. Work-skill `SKILL.md` files (seven)

For each, key touch sites with file:line. The load-bearing one is
`update-work-item/SKILL.md:191` (field→label mapping).

- [`skills/work/create-work-item/SKILL.md`](../../../skills/work/create-work-item/SKILL.md) — 11 sites:
  lines 57, 115, 238, 254-256, 275, 322, 344, 521-523, 527, 562.
- [`skills/work/refine-work-item/SKILL.md`](../../../skills/work/refine-work-item/SKILL.md) — 8 sites:
  lines 31, 83, 130, 189-191 (child kind derivation rule), 363-366
  (canonical tree fence template), 370 (example), 402.
- [`skills/work/list-work-items/SKILL.md`](../../../skills/work/list-work-items/SKILL.md) — 13 sites:
  lines 5, 48, 52 (invokes `work-item-template-field-hints.sh type` —
  must change argument), 58, 74, 80, 87, 184, 195, 225 (table header
  `| ID | Title | Type | ...`), 246-249 (tree fence template), 272,
  298, 302.
- [`skills/work/update-work-item/SKILL.md`](../../../skills/work/update-work-item/SKILL.md) — load-bearing rule
  at **line 191**: `` `type` ↔ `**Type**: ` `` mapping in the
  Body label sync table. Also line 273 (enumerated label list:
  `**Status**:, **Type**:, **Priority**:, **Author**:`), and lines
  279-280 (legacy-item examples). The migration script must mirror this
  body-label rewrite contract.
- [`skills/work/review-work-item/SKILL.md`](../../../skills/work/review-work-item/SKILL.md) — 4 sites:
  lines 77 (`type-appropriate content` — semantically about kind),
  91-92, 173, 354. **Line 354 is out of scope**: `type: work-item-review`
  is the review-artifact's own frontmatter (a distinct schema, not the
  work-item kind). Do not touch.
- [`skills/work/extract-work-items/SKILL.md`](../../../skills/work/extract-work-items/SKILL.md) — 13 sites:
  lines 34, 70-71, 181-182, 188, 191, 210, 224 (generic English, do not
  touch), 257, 309, 403 (generic English, do not touch), 502-504, 507.
- [`skills/work/stress-test-work-item/SKILL.md`](../../../skills/work/stress-test-work-item/SKILL.md) — 2 sites:
  lines 166-167 (frontmatter immutability rule listing `type` and
  `**Type**:` body label).

### 4. Review-lens `SKILL.md` files

Only four lenses branch on kind. Three of them (`scope`, `dependency`,
`testability`) reason about kinds in prose only — they describe per-kind
review rules but never reference the literal field name. **Only
`completeness-lens` references the literal `type:` field.**

- [`skills/review/lenses/completeness-lens/SKILL.md`](../../../skills/review/lenses/completeness-lens/SKILL.md) —
  literal `type` field references at lines 28, 36, 54, 96; section
  headings at 26, 76, 94, 96; kind-conditional rules at lines 29-35,
  78-92, 107-108, 133-136.
- [`skills/review/lenses/scope-lens/SKILL.md`](../../../skills/review/lenses/scope-lens/SKILL.md) — kind-conditional
  prose at lines 34, 43-66, 87-96, 113-118, 131. No literal field
  reference.
- [`skills/review/lenses/dependency-lens/SKILL.md`](../../../skills/review/lenses/dependency-lens/SKILL.md) —
  kind-conditional prose at lines 55-56, 89-102, 109-111. No literal
  field reference.
- [`skills/review/lenses/testability-lens/SKILL.md`](../../../skills/review/lenses/testability-lens/SKILL.md) —
  kind-conditional prose at lines 31-40, 69-78, 103-105. No literal
  field reference.

Verified that no other lens (`correctness`, `compatibility`,
`portability`, `safety`, `architecture`, `security`, `code-quality`,
`test-coverage`, `standards`, `performance`, `database`, `usability`,
`clarity`, `documentation`) references the work-item kind field.

No scripts exist under any lens directory.

### 5. Visualiser frontend

#### `WorkItemCard.tsx` — the only production consumer

[`skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:27-55`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx)

```ts
27  const fmType = entry.frontmatter['type']
28  const typeLabel = typeof fmType === 'string' && fmType.length > 0 ? fmType : null
...
55  {typeLabel !== null && <p className={styles.cardType}>{typeLabel}</p>}
```

Changes:
- Line 27: `'type'` → `'kind'`.
- Rename variables: `fmType` → `fmKind`, `typeLabel` → `kindLabel` for
  consistency.
- Optional: CSS class `.cardType` (line 55 and
  `WorkItemCard.module.css:27`) may be renamed `.cardKind`.

**Falsefriend at line 39**: `params={{ type: 'work-items', fileSlug }}`
is the TanStack Router path-param name for `/library/$type/$fileSlug` —
do NOT touch.

#### Tests

[`skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.test.tsx`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.test.tsx) — frontmatter `type:` literal at
lines 27, 42, 54, 65 (test title `frontmatter.type is missing`), 70
(absent-type case), 83, 96, 108. Only line 33 asserts on the rendered
text (`'adr-creation-task'`).

[`skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx) — incidental fixture `type:`
at lines 149, 153, 157, 170, 181 (no assertions).

#### TypeScript types

[`skills/visualisation/visualise/frontend/src/api/types.ts:73`](../../../skills/visualisation/visualise/frontend/src/api/types.ts):
`frontmatter: Record<string, unknown>` — frontmatter is an open
string-keyed bag. No interface declares `type?:`. The rename requires
zero TS type updates.

#### Rust backend

[`skills/visualisation/visualise/server/src/indexer.rs:172`](../../../skills/visualisation/visualise/server/src/indexer.rs):
`pub frontmatter: serde_json::Value`. Frontmatter is a generic JSON
value; no field-name hardcoding. The `"type"` literals in
[`sse_hub.rs:16`](../../../skills/visualisation/visualise/server/src/sse_hub.rs) (`#[serde(tag = "type")]`
for SSE payloads) and [`api/docs.rs:21`](../../../skills/visualisation/visualise/server/src/api/docs.rs)
(`#[serde(rename = "type")]` for the `?type=` query parameter) are
unrelated to frontmatter. The story's claim that the backend needs no
change is confirmed.

### 6. Eval fixtures and graders

#### Fixtures (`*.md` files with `^type: (story|epic|task|bug|spike)`)

**Total: 137 files** (story's "roughly 100" undercounted the corpus).
Breakdown:

| Surface | Count |
|---|---|
| `skills/work/create-work-item/evals/files/` | 7 |
| `skills/work/refine-work-item/evals/files/` | 89 (19 single-scenario + 35 scenario-11b stubs + 35 scenario-11c stubs + targets) |
| `skills/work/review-work-item/evals/files/` | 1 |
| `skills/work/stress-test-work-item/evals/files/` | 6 |
| `skills/work/extract-work-items/evals/` | 0 (input prose, no structured FM) |
| `skills/work/update-work-item/evals/` | 0 (JSON drivers only) |
| `skills/review/lenses/clarity-lens/evals/files/` | 5 |
| `skills/review/lenses/completeness-lens/evals/files/` | 5 |
| `skills/review/lenses/dependency-lens/evals/files/` | 8 |
| `skills/review/lenses/scope-lens/evals/files/` | 9 |
| `skills/review/lenses/testability-lens/evals/files/` | 5 |
| **Total** | **137** |

**Zero** of the 137 contain a `^\*\*Type\*\*:` body label — the bold
label convention isn't used in the eval corpus.

The work-skill `refine-work-item` is the heaviest single surface (89/137
files), largely due to scenario-11b/c stub fleets (35 files each).

#### Graders

The story-recommended regex
`"type"\s*:\s*"(story|epic|task|bug|spike)"` against
`*evals.json`/`*benchmark.json` returns **zero matches** — graders do
not embed kind values as JSON literals.

However, three grader lines reference `Frontmatter: type` semantically
(scope-lens evals); these must also be rewritten:

- [`skills/review/lenses/scope-lens/evals/evals.json:19`](../../../skills/review/lenses/scope-lens/evals/evals.json) — `"located in Summary or Frontmatter: type"`.
- [`skills/review/lenses/scope-lens/evals/benchmark.json:62`](../../../skills/review/lenses/scope-lens/evals/benchmark.json) — `"Finding is located in Summary or Frontmatter: type"` and `"Location: 'Frontmatter: type'."`.
- [`skills/review/lenses/scope-lens/evals/benchmark.json:92`](../../../skills/review/lenses/scope-lens/evals/benchmark.json) — `"Finding is in Frontmatter: type, Requirements, or Stories"` / `"Major finding in Frontmatter: type"`.

The remaining 219 grader `type` occurrences (across `refine-work-item`,
`create-work-item`, `extract-work-items` benchmark/evals JSON) are
false positives — they describe grader infrastructure types
(`"type": "llm"`, `"type": "contains"`), filesystem/output types,
review-artefact types — and must be left alone.

Other work-skill SKILL.md-side touches that affect eval grading:
benchmark.json files mention `type` as a grader assertion location in
`create-work-item/evals/benchmark.json:518, 552, 1377-1411, 1634, 1658,
1663`, `extract-work-items/evals/benchmark.json:277, 340`, and
`stress-test-work-item/evals/benchmark.json:294` — check these in
context; many are grader-infrastructure false positives.

### 7. Migration framework and migration 0005 design

#### Driver

[`skills/config/migrate/scripts/run-migrations.sh`](../../../skills/config/migrate/scripts/run-migrations.sh):

- **Discovery** (lines 87-93): `find -maxdepth 1` for
  `[0-9][0-9][0-9][0-9]-*.sh` in `MIGRATIONS_DIR`
  (`ACCELERATOR_MIGRATIONS_DIR` overridable). Sorted lexicographically.
- **Dirty-tree guard** (lines 41-70): scans `jj diff` or
  `git status` against `meta/`, `.claude/accelerator`, `.accelerator/`.
  Bypass: `ACCELERATOR_MIGRATE_FORCE=1`.
- **Preview banner** (lines 162-175): always prints
  `"About to apply N migration(s):"` with `# DESCRIPTION:` lines.
- **State-file append** (lines 204-205): driver calls
  `atomic_append_unique "$STATE_FILE" "$id"` on success. **Migrations
  MUST NOT touch the state file directly.**
- **Exports to migration** (line 184): `PROJECT_ROOT`,
  `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1`.
- **`# DESCRIPTION:` parsing** (lines 165-166): second line of script,
  format `# DESCRIPTION: <text>`.

#### Canonical model — migration 0001

[`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:53-69`](../../../skills/config/migrate/migrations/0001-rename-tickets-to-work.sh):

```bash
if [ -d "$tickets_dir" ]; then
  while IFS= read -r -d '' file; do
    if grep -q '^ticket_id:' "$file" 2>/dev/null; then
      if grep -q '^work_item_id:' "$file" 2>/dev/null; then
        # Both keys present (partial prior rewrite) — remove old key line
        grep -v '^ticket_id:' "$file" > "$file.tmp"
        mv "$file.tmp" "$file"
      else
        # Only old key — rename it
        sed 's/^ticket_id:/work_item_id:/' "$file" > "$file.tmp"
        mv "$file.tmp" "$file"
      fi
    fi
  done < <(find "$tickets_dir" -name '*.md' -print0)
fi
```

This is the **idempotence convention**: outer guard
`grep -q '^old_key:'`, then dual-presence sub-check on new key.

0001 uses `> .tmp; mv` for the rewrite; **migrations 0002-0004 use
`atomic_write`** from
[`scripts/atomic-common.sh`](../../../scripts/atomic-common.sh) (preferred —
mkdir -p, mktemp same-FS, EXIT-trap cleanup, atomic mv). Migration 0005
should use `atomic_write`.

#### Body-content rewrite precedent

[`migrations/0002-rename-work-items-with-project-prefix.sh`](../../../skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh) has the closest model for
rewriting both frontmatter and body prose (functions
`rewrite_frontmatter_in_file`, `rewrite_markdown_links_in_file`,
`rewrite_prose_in_file`). For a simpler line-anchored body label
rewrite (`^\*\*Type\*\*:` → `^\*\*Kind\*\*:`), a sed pass mirroring
0001's frontmatter loop is sufficient.

#### Userspace path resolution

[`scripts/config-read-path.sh:21-42`](../../../scripts/config-read-path.sh):
`config-read-path.sh <key> [default]`. Returns a path relative to the
project root (e.g. `meta/work`). Honours `paths.work` from
`.accelerator/config.md` overlaid by `.accelerator/config.local.md`,
defaulting from `config-defaults.sh:36`. Callers must prefix with
`$PROJECT_ROOT/`.

#### Recommended migration 0005 skeleton

```bash
#!/usr/bin/env bash
# DESCRIPTION: Rename work-item `type:` frontmatter field to `kind:` and `**Type**:` body label to `**Kind**:`.

set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

work_dir_rel="$(bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work)"
work_dir="$PROJECT_ROOT/$work_dir_rel"

[ -d "$work_dir" ] || exit 0

while IFS= read -r -d '' file; do
  needs_fm_rewrite=0
  needs_body_rewrite=0
  grep -q '^type:' "$file" 2>/dev/null && needs_fm_rewrite=1
  grep -q '^\*\*Type\*\*:' "$file" 2>/dev/null && needs_body_rewrite=1

  [ "$needs_fm_rewrite" = 0 ] && [ "$needs_body_rewrite" = 0 ] && continue

  if [ "$needs_fm_rewrite" = 1 ] && grep -q '^kind:' "$file" 2>/dev/null; then
    # Both present (partial prior run) — drop stale `type:` line
    grep -v '^type:' "$file" | atomic_write "$file"
  elif [ "$needs_fm_rewrite" = 1 ]; then
    sed 's/^type:/kind:/' "$file" | atomic_write "$file"
  fi

  if [ "$needs_body_rewrite" = 1 ]; then
    sed 's/^\*\*Type\*\*:/**Kind**:/' "$file" | atomic_write "$file"
  fi
done < <(find "$work_dir" -name '*.md' -print0)
```

Idempotence properties:
- Re-running on a fully-migrated file (`kind:` only, `**Kind**:` only) is
  a no-op (both grep guards return false).
- Partial prior run (both `type:` and `kind:` present) — drops the stale
  `type:` line.
- Files with no `**Type**:` (the 29 legacy `adr-creation-task` files)
  skip the body rewrite cleanly.

### 8. `meta/work/` corpus state

- **72 `.md` files**, every one carries a `^type:` line.
- **43 carry kind values** (`story|epic|task|bug|spike`) and have
  matching `^\*\*Type\*\*: <Title-Case>` body labels.
- **29 carry legacy `type: adr-creation-task`** (with no body label).
- All 43 use bare unquoted YAML — no inline comments, no quoting
  anomalies. Sed-based rewrites are safe.

Sample shapes:
- Modern: [`meta/work/0072-playwright-daemon-cjs-import-bug.md:1-15`](../../meta/work/0072-playwright-daemon-cjs-import-bug.md), [`meta/work/0036-sidebar-redesign.md:1-15`](../../meta/work/0036-sidebar-redesign.md).
- Legacy ADR-creation-task: [`meta/work/0001-three-layer-review-system-architecture.md:1-7`](../../meta/work/0001-three-layer-review-system-architecture.md).

### 9. `agents/*.md` — confirmed out of scope

The story disputes the parent epic 0057's claim that `agents/*.md` files
reference the work-item kind field. All `type` mentions across
`browser-locator.md`, `browser-analyser.md`, `documents-locator.md`,
`documents-analyser.md`, `codebase-locator.md`,
`codebase-pattern-finder.md`, `reviewer.md`, `web-search-researcher.md`
are unrelated (TypeScript `type` keywords, `subagent_type` parameters,
HTML input types, browser command `type`). **No `agents/*.md` change is
needed.** Parent epic 0057's technical-notes claim should be amended.

## Code References

### Template
- [`templates/work-item.md:6`](../../../templates/work-item.md) — `type: story` frontmatter line
- [`templates/work-item.md:15`](../../../templates/work-item.md) — `**Type**:` body label

### Helper scripts
- [`skills/work/scripts/work-item-template-field-hints.sh:7,25`](../../../skills/work/scripts/work-item-template-field-hints.sh) — hardcoded `type)` case arm
- [`skills/work/scripts/work-item-read-field.sh:46`](../../../skills/work/scripts/work-item-read-field.sh) — field-name-agnostic
- [`skills/work/scripts/test-work-item-scripts.sh`](../../../skills/work/scripts/test-work-item-scripts.sh) — multiple fixture/assertion sites

### Work skills
- [`skills/work/update-work-item/SKILL.md:191`](../../../skills/work/update-work-item/SKILL.md) — load-bearing field↔body-label mapping
- See body of section 3 above for the full per-skill list.

### Review lenses
- [`skills/review/lenses/completeness-lens/SKILL.md:28,36,54,96`](../../../skills/review/lenses/completeness-lens/SKILL.md) — literal `type` references
- [`skills/review/lenses/scope-lens/SKILL.md`](../../../skills/review/lenses/scope-lens/SKILL.md), [`dependency-lens/SKILL.md`](../../../skills/review/lenses/dependency-lens/SKILL.md), [`testability-lens/SKILL.md`](../../../skills/review/lenses/testability-lens/SKILL.md) — kind-conditional prose only

### Visualiser
- [`skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:27-55`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx)
- [`skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.test.tsx:27,33,42,54,65,70,83,96,108`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.test.tsx)
- [`skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx:149,153,157,170,181`](../../../skills/visualisation/visualise/frontend/src/routes/kanban/KanbanBoard.test.tsx)
- [`skills/visualisation/visualise/frontend/src/api/types.ts:73`](../../../skills/visualisation/visualise/frontend/src/api/types.ts) — generic frontmatter map

### Migration framework
- [`skills/config/migrate/scripts/run-migrations.sh`](../../../skills/config/migrate/scripts/run-migrations.sh) — driver
- [`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:53-69`](../../../skills/config/migrate/migrations/0001-rename-tickets-to-work.sh) — canonical model
- [`skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh`](../../../skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh) — body-rewrite precedent
- [`scripts/atomic-common.sh:16`](../../../scripts/atomic-common.sh) — `atomic_write`
- [`scripts/config-read-path.sh:21-42`](../../../scripts/config-read-path.sh) — userspace path resolver

### Eval graders requiring rewrites
- [`skills/review/lenses/scope-lens/evals/evals.json:19`](../../../skills/review/lenses/scope-lens/evals/evals.json)
- [`skills/review/lenses/scope-lens/evals/benchmark.json:62,92`](../../../skills/review/lenses/scope-lens/evals/benchmark.json)

## Architecture Insights

- **Frontmatter access pattern**: Across the entire codebase, only two
  consumers hardcode the literal field name `type` for work-item
  semantic kind: `work-item-template-field-hints.sh` (hint fallback)
  and `WorkItemCard.tsx` (display). Everything else is either field-name
  parametric (Rust backend, helper scripts, generic readers) or
  documents the field name in prose only. This is excellent factoring —
  the rename is a one-character production-code change.

- **Idempotence convention** for frontmatter-key-rename migrations:
  outer `grep -q '^old:'` guard, dual-presence sub-check for the new
  key. Established by migration 0001, mirrored by 0002-0004 with
  variations (value equality, `_move_if_pending`,
  `MIGRATION_RESULT: no_op_pending`).

- **Body content vs. frontmatter rewrites**: The migration framework
  doesn't distinguish — migrations may rewrite body content (see 0002).
  The convention is sed/awk piped into `atomic_write` for crash safety.

- **Three discriminator-like uses of `type:`** coexist in this repo:
  1. Work-item semantic kind (`story | epic | task | bug | spike`) —
     the field being renamed.
  2. Review-artifact own type (`type: work-item-review`) — distinct
     schema, untouched.
  3. Forthcoming unified artifact-type discriminator (`work-item`,
     `plan`, `adr`, etc.) per ADR-0033 — the slot this rename frees.

- **Userspace path resolution**: Every migration that operates on
  `meta/work/` (or any other path-configured directory) must use
  `config-read-path.sh <key>` to honour `paths.work` overrides; never
  hardcode `meta/work`.

- **Visualiser frontmatter typing**: Frontmatter is a
  `Record<string, unknown>` (TS) and `serde_json::Value` (Rust) —
  intentionally open. This means renaming frontmatter fields requires
  zero type-system changes. New fields and renames are absorbed
  generically; only literal-key accesses in render code matter.

## Historical Context

### Key ADRs

- [`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`](../../meta/decisions/ADR-0033-unified-base-frontmatter-schema.md) (accepted 2026-05-19) — defines the unified base
  frontmatter schema across all twelve artifact types. Lines 34-37
  explicitly call out the work-item `type:` overload as the conflict
  this rename resolves. **This is the ADR that motivates story 0063.**
- [`meta/decisions/ADR-0034-typed-linkage-vocabulary.md`](../../meta/decisions/ADR-0034-typed-linkage-vocabulary.md) (accepted 2026-05-20) — defines typed linkage vocabulary
  (`parent`, `supersedes`, `blocks`, `target`, `derived_from`,
  `relates_to`, `source`).
- [`meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md`](../../meta/decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md) (accepted 2026-03-22) — the original minimal common
  schema that ADR-0033 supplements.
- [`meta/decisions/ADR-0023-meta-directory-migration-framework.md`](../../meta/decisions/ADR-0023-meta-directory-migration-framework.md) — discovery contract (lines 99-101),
  migration script API (lines 102-106), dirty-tree guard (113-115),
  preview banner (117), state-file contract (108-110, 118-119).
- [`meta/decisions/ADR-0022-work-item-terminology.md`](../../meta/decisions/ADR-0022-work-item-terminology.md) (accepted 2026-04-25) — the `tickets` → `work` rename
  that originally necessitated the migration framework. Precedent for
  this rename's structure.

### Parent epic and siblings

- [`meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`](../../meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md) — parent epic. Line 147 contains
  the "Coordinate this rename in a single story" instruction that
  underpins 0063's single-PR scope decision. Line 72 and 147 include
  the speculative `agents/*.md` claim that 0063 disputes (lines
  233-235 of 0063); the epic's technical-notes should be amended.
- [`meta/work/0065-update-artifact-templates-to-unified-schema.md`](../../meta/work/0065-update-artifact-templates-to-unified-schema.md) — child story, blocked by 0063 because it
  touches `templates/work-item.md`. Confirmed by 0065:50.
- [`meta/work/0070-ship-meta-corpus-unified-schema-migration.md`](../../meta/work/0070-ship-meta-corpus-unified-schema-migration.md) — child story, blocked by 0063. **0063
  takes the `type:` → `kind:` rewrite step out of 0070's migration;**
  0070's migration only covers the broader unified-schema changes
  (provenance bundle, `schema_version`, typed linkage population). See
  0063:188-191 and 0063:252-256.

### Other relevant research / plans
- [`meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md`](2026-04-25-rename-tickets-to-work-items.md) — precedent research for the
  earlier `tickets` → `work` rename, which produced migration 0001
  (the canonical model).

## Related Research

- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` —
  precedent rename pattern.
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
  — patterns for verifying full rename completion via deterministic
  greps.
- `meta/research/codebase/2026-05-03-update-visualiser-for-work-item-terminology.md`
  — visualiser frontmatter consumption patterns.

## Open Questions

1. **`review-work-item/SKILL.md:77`** uses the phrase
   "type-appropriate content" — generic English about the work-item
   kind. Should this be reworded to "kind-appropriate" for consistency
   with the rename? Likely yes, but flag with maintainer.

2. **`scope-lens` grader strings** at evals.json:19 and benchmark.json:62,92
   reference `Frontmatter: type` as an evidence-location string. These
   are user-facing assertion text that compares LLM output. Rewriting
   them to `Frontmatter: kind` requires regenerating any baseline
   benchmark scores — confirm whether the eval baseline is regenerated
   as part of the story's acceptance criterion 9 ("eval-suite pass rate
   is unchanged from the pre-rename baseline").

3. **CSS class `.cardType`** at
   `WorkItemCard.module.css:27` and the `styles.cardType` access at
   `WorkItemCard.tsx:55` — rename to `.cardKind` or leave?
   Style class names are internal and have no protocol impact.
   Recommendation: rename for consistency.

4. **Story line count for "eval fixtures"** — story 0063 says "roughly
   100"; actual count is **137** (or 106 if scoped to work-skills
   only). The story may want amending, though "roughly 100" is not
   numerically wrong.

5. **Body label sequencing in migration 0005** — the skeleton above
   does two `atomic_write` passes per file when both rewrites are
   needed (one for `^type:`, one for `^\*\*Type\*\*:`). This is
   acceptable but slightly inefficient. An awk-based single-pass
   rewrite would be cleaner; defer to implementer preference.
