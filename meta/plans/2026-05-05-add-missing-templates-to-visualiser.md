---
date: "2026-05-05T00:00:00Z"
type: plan
skill: create-plan
work-item: ""
status: accepted
---

# Add Design Gap and Design Inventory to Visualiser

## Overview

The plugin ships 8 templates but the visualiser only exposes 6 of them (`design-gap` and `design-inventory` are never wired into the config script). Additionally, these two document types have no corresponding entries in the server's `DocTypeKey` enum or the frontend's type registry, so their documents cannot be browsed or included in lifecycle clusters. This plan adds them as first-class document types and wires up their templates, updating all associated test infrastructure in lockstep.

A secondary finding: the test infrastructure lags behind the live config script by one template (`work-item` was added to the script but the test common helper and JSON fixtures were never updated). This is fixed in Phase 1.

## Current State Analysis

**Templates on disk** (`templates/` — 8 files):
`adr`, `design-gap`, `design-inventory`, `plan`, `pr-description`, `research`, `validation`, `work-item`

**Templates wired in `write-visualiser-config.sh:81-86`** (6):
`adr`, `plan`, `research`, `validation`, `pr-description`, `work-item`

**Templates in test infrastructure** (5 — `tests/common/mod.rs:34`, `config.valid.json`, `config.optional-override-null.json`):
`adr`, `plan`, `research`, `validation`, `pr-description`

**Doc types registered in `DocTypeKey` enum** (`src/docs.rs:6-18`):
11 variants — no `DesignGaps` or `DesignInventories`

**Doc types in frontend** (`frontend/src/api/types.ts:4-17`):
11 entries — no `'design-gaps'` or `'design-inventories'`

### Key Discoveries

- `write-visualiser-config.sh:50-59,148-170` — two changes needed per new doc type: a variable assignment calling `abs_path`, and a `--arg` + key in the jq `doc_paths` object.
- `src/docs.rs:6-18,21-34,37-51,53-67,69-79` — the `DocTypeKey` enum is the authoritative server registry; `in_lifecycle()` already defaults to `true` for any non-Templates variant via `!matches!(self, Templates)`, so no per-variant override is needed.
- `src/clusters.rs:67-81,97-125` — `canonical_rank()` and `derive_completeness()` are exhaustive matches; both need a new arm per new type. Design gaps and inventories have `canonical_rank` values and tracking fields in `Completeness` (`has_design_inventory`, `has_design_gap`), enabling the lifecycle pipeline dots to illuminate when documents are present. They appear in the **long-tail** section of the pipeline steps via `longTail: true` rather than in the main workflow dots — distinct from `WorkItemReviews`, which has a no-op completeness arm and no pipeline step at all.
- `src/config.rs:226` — inline test asserts `doc_paths.len() == 9` against `config.valid.json`; fixture and assertion both need updating.
- `tests/config_contract.rs:49` — asserts `doc_paths.len() == 10` from the live script output; needs updating to 12.
- `tests/api_types.rs:13,32` — test name and length assertion hardcode `eleven`/`11`; both need updating to 13.
- `src/docs.rs` inline tests — three tests hardcode `11`; all need updating to 13. The `kebab_case_round_trip_covers_every_variant` test needs two new pairs.
- `SKILL.md:14-25` — lists directory paths for Claude context; two new directory lines needed.
- Frontend sidebar and library views are data-driven from `/api/types` — no changes needed beyond `types.ts`.

## Desired End State

- The visualiser sidebar shows "Design gaps" and "Design inventories" as document sections.
- Documents in `meta/design-gaps/` and `meta/design-inventories/` are indexed, browsable, and appear in lifecycle clusters.
- The visualiser templates page lists all 8 templates including `design-gap` and `design-inventory`.
- All tests pass; count assertions reflect 13 doc types and 8 templates.

## What We're NOT Doing

- No `WORKFLOW_PIPELINE_STEPS` promotion — both new types carry `longTail: true`, so they appear in the long-tail section alongside `notes` rather than in the main workflow dots.
- No changes to existing template file contents.
- No new config keys beyond `design_gaps` and `design_inventories` — the `templates.design-gap` / `templates.design-inventory` config-override path is wired automatically by the existing `template_tier` helper.

---

## Phase 1: Wire the Two Missing Templates End-to-End

### Overview

Add `design-gap` and `design-inventory` to the template config, and bring the test infrastructure up to date for all 8 templates (including the previously-lagging `work-item`).

### Changes Required

#### 1. `write-visualiser-config.sh` — register two new templates

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`

After line 86 (after `WI="$(template_tier work-item)"`), add:
```bash
DGAP="$(template_tier design-gap)"
DINV="$(template_tier design-inventory)"
```

Add two `--argjson` arguments to the `jq -n` call (after `--argjson work_item_template "$WI"`):
```bash
  --argjson design_gap "$DGAP" \
  --argjson design_inventory "$DINV" \
```

Extend the `templates` object (after `"work-item": $work_item_template`):
```
"design-gap": $design_gap,
"design-inventory": $design_inventory
```

#### 2. `config_contract.rs` — update template count assertion

**File**: `skills/visualisation/visualise/server/tests/config_contract.rs`

Line 68: change `6` → `8`.
Line 74: add `"design-gap"` and `"design-inventory"` to the checked name list.

#### 3. `tests/common/mod.rs` — seed all 8 templates

**File**: `skills/visualisation/visualise/server/tests/common/mod.rs`

Line 34: extend the name array to include all 8:
```rust
for name in ["adr", "plan", "research", "validation", "pr-description",
             "work-item", "design-gap", "design-inventory"] {
```

#### 4. `tests/fixtures/templates/` — add three missing fixture files

Add minimal placeholder files matching the existing convention (single heading):
- `tests/fixtures/templates/work-item.md` → `# work-item plugin default`
- `tests/fixtures/templates/design-gap.md` → `# design-gap plugin default`
- `tests/fixtures/templates/design-inventory.md` → `# design-inventory plugin default`

#### 5. `config.valid.json` — add three missing template entries

**File**: `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`

After the `"pr-description"` block, add:
```json
"work-item": {
  "config_override": null,
  "user_override": "/abs/path/to/project/.accelerator/templates/work-item.md",
  "plugin_default": "/abs/path/to/plugin/templates/work-item.md"
},
"design-gap": {
  "config_override": null,
  "user_override": "/abs/path/to/project/.accelerator/templates/design-gap.md",
  "plugin_default": "/abs/path/to/plugin/templates/design-gap.md"
},
"design-inventory": {
  "config_override": null,
  "user_override": "/abs/path/to/project/.accelerator/templates/design-inventory.md",
  "plugin_default": "/abs/path/to/plugin/templates/design-inventory.md"
}
```

#### 6. `config.optional-override-null.json` — add three missing template entries

**File**: `skills/visualisation/visualise/server/tests/fixtures/config.optional-override-null.json`

Same three blocks as above.

#### 7. `src/config.rs` — update templates count assertion

**File**: `skills/visualisation/visualise/server/src/config.rs`

Line 230: change `assert_eq!(c.templates.len(), 5)` → `assert_eq!(c.templates.len(), 8)`.

(The adjacent `doc_paths.len()` assertion on line 226 is not touched until Phase 2 Step 8.)

### Success Criteria

#### Automated Verification

- [ ] Contract test passes (asserts 8 templates): `cargo test -p accelerator-visualiser config_contract`
- [ ] All server tests pass: `cargo test -p accelerator-visualiser`
- [ ] Frontend tests pass: `npm test` (run from `skills/visualisation/visualise/frontend/`)

#### Manual Verification

- [ ] Navigating to `/library/templates` shows all 8 templates as links
- [ ] Clicking `design-gap` and `design-inventory` loads their detail pages with plugin-default tier active

---

## Phase 2: Add Design Gaps and Design Inventories as Document Types

### Overview

Register `DesignGaps` and `DesignInventories` as first-class doc types in the server enum, wire their directory paths through the config script, and add them to the frontend type registry. Default directory paths are `meta/design-gaps` and `meta/design-inventories`.

The convention throughout the codebase is that multi-word doc types use plural nouns in their directory names, config keys, and wire-format keys: `plans`, `decisions`, `plan-reviews`, etc. The template frontmatter field `type: design-gap` (singular) is a per-document tag, distinct from the collection key `design-gaps` (plural) used for routing and indexing.

Both types are added to `Completeness` with `longTail: true` in the frontend pipeline steps — they're meaningful milestones when present but are not part of the standard delivery pipeline. Design inventory logically precedes gap analysis, so it appears first in the step order.

### Changes Required

#### 1. `write-visualiser-config.sh` — add two new doc path variables

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`

After line 59 (after `PRS="$(abs_path prs meta/prs)"`), add:
```bash
DESIGN_GAPS="$(abs_path design_gaps meta/design-gaps)"
DESIGN_INVENTORIES="$(abs_path design_inventories meta/design-inventories)"
```

Add two `--arg` flags to the `jq -n` call (after `--arg prs "$PRS"`):
```bash
  --arg design_gaps "$DESIGN_GAPS" \
  --arg design_inventories "$DESIGN_INVENTORIES" \
```

Extend the `doc_paths` object (after `prs: $prs`):
```
design_gaps: $design_gaps,
design_inventories: $design_inventories
```

#### 2. `SKILL.md` — add two directory lines

**File**: `skills/visualisation/visualise/SKILL.md`

After line 24 (`**Notes directory**`), add:
```markdown
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps meta/design-gaps`
**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories meta/design-inventories`
```

#### 3. `src/docs.rs` — register two new enum variants

**File**: `skills/visualisation/visualise/server/src/docs.rs`

**Enum** (lines 6-18): add after `Prs`:
```rust
DesignGaps,
DesignInventories,
```

**`all()` array** (lines 21-35): change `[DocTypeKey; 11]` → `[DocTypeKey; 13]`, add both variants before `Templates`:
```rust
DocTypeKey::DesignGaps,
DocTypeKey::DesignInventories,
```

**`config_path_key()`** (lines 37-51): add before the `Templates => None` arm:
```rust
DocTypeKey::DesignGaps => Some("design_gaps"),
DocTypeKey::DesignInventories => Some("design_inventories"),
```

**`label()`** (lines 53-67): add before the `Templates` arm:
```rust
DocTypeKey::DesignGaps => "Design gaps",
DocTypeKey::DesignInventories => "Design inventories",
```

**`in_lifecycle()`, `in_kanban()`, `is_virtual()`**: no changes needed — the existing negation patterns (`!matches!(self, Templates)` / `matches!(self, WorkItems)` / `matches!(self, Templates)`) automatically give both new variants the correct values (`true`, `false`, `false`).

**Inline tests in `docs.rs`**:

- `kebab_case_round_trip_covers_every_variant` (line 116): add two pairs:
  ```rust
  (DocTypeKey::DesignGaps, "design-gaps"),
  (DocTypeKey::DesignInventories, "design-inventories"),
  ```
- `all_returns_every_variant_exactly_once` (line 143): change `11` → `13`
- `doc_type_key_all_returns_eleven_variants` (line 170): rename to `doc_type_key_all_returns_thirteen_variants` and change `11` → `13`
- `describe_types_populates_dir_paths_from_config` (line 213): change `types.len()` from `11` → `13`

#### 4. `src/clusters.rs` — add rank and completeness arms

**File**: `skills/visualisation/visualise/server/src/clusters.rs`

**`Completeness` struct** (lines 10-20): add two new boolean fields:
```rust
pub has_design_inventory: bool,
pub has_design_gap: bool,
```

**`derive_completeness()` initialiser** (lines 98-108): add to the `Completeness { .. }` literal:
```rust
has_design_inventory: false,
has_design_gap: false,
```

**`derive_completeness()` match arms** (lines 109-123): add two new tracking arms alongside the existing no-op arms — `WorkItemReviews => {}` and `Templates => {}` stay in place:
```rust
DocTypeKey::DesignGaps => c.has_design_gap = true,
DocTypeKey::DesignInventories => c.has_design_inventory = true,
// existing arms remain:
// DocTypeKey::WorkItemReviews => {}
// DocTypeKey::Templates => {}
```

**`canonical_rank()`** (lines 67-81): add before the `Templates => u8::MAX` arm:
```rust
DocTypeKey::DesignInventories => 9,
DocTypeKey::DesignGaps => 10,
```

(Inventory precedes gap in the lifecycle order, matching the `LIFECYCLE_PIPELINE_STEPS` ordering below.)

**`completeness_flags_track_present_types` test** (line 186): extend the existing assertions to cover the two new fields being false when absent:
```rust
assert!(!c.has_design_gap);
assert!(!c.has_design_inventory);
```

Also add three new tests:
```rust
#[test]
fn completeness_camelcase_field_names_match_typescript_interface() {
    let entries = vec![
        entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
        entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
    ];
    let clusters = compute_clusters(&entries);
    let json = serde_json::to_value(&clusters[0].completeness).unwrap();
    assert_eq!(json["hasDesignGap"], true);
    assert_eq!(json["hasDesignInventory"], true);
}

#[test]
fn design_gap_and_inventory_completeness_flags_are_set() {
    let entries = vec![
        entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
        entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
    ];
    let clusters = compute_clusters(&entries);
    assert_eq!(clusters.len(), 1);
    let c = &clusters[0].completeness;
    assert!(c.has_design_gap);
    assert!(c.has_design_inventory);
}

#[test]
fn design_inventory_sorts_before_design_gap_in_cluster() {
    let entries = vec![
        entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
        entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
        entry(DocTypeKey::Notes, "foo", 30, "Note"),
    ];
    let clusters = compute_clusters(&entries);
    assert_eq!(clusters.len(), 1);
    let types: Vec<DocTypeKey> = clusters[0].entries.iter().map(|e| e.doc_type).collect();
    let notes_pos = types.iter().position(|t| *t == DocTypeKey::Notes).unwrap();
    let inv_pos = types.iter().position(|t| *t == DocTypeKey::DesignInventories).unwrap();
    let gap_pos = types.iter().position(|t| *t == DocTypeKey::DesignGaps).unwrap();
    assert!(notes_pos < inv_pos, "Notes should sort before DesignInventories");
    assert!(inv_pos < gap_pos, "DesignInventories should sort before DesignGaps");
}
```

#### 5. `tests/common/mod.rs` — add two doc_paths to seeded config

**File**: `skills/visualisation/visualise/server/tests/common/mod.rs`

In `seeded_cfg`, after the `doc_paths.insert("prs"...)` call, add:
```rust
doc_paths.insert("design_gaps".into(), meta.join("design-gaps"));
doc_paths.insert("design_inventories".into(), meta.join("design-inventories"));
```

#### 6. `config.valid.json` — add two doc_path entries

**File**: `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`

After `"prs": "/abs/path/to/project/meta/prs"`, add:
```json
"design_gaps": "/abs/path/to/project/meta/design-gaps",
"design_inventories": "/abs/path/to/project/meta/design-inventories"
```

> **Note on `review_work`**: The live config script also produces a `review_work` doc path key, but both JSON fixtures already omit it (a pre-existing gap). Adding `review_work` to the fixtures is out of scope for this plan; the static fixture and live script will continue to diverge on that key. The `config.rs` inline test (`parses_valid_config`) counts fixture entries only, so the assertion updating from 9 → 11 is correct for these two fixtures as they stand.

#### 7. `config.optional-override-null.json` — add two doc_path entries

**File**: `skills/visualisation/visualise/server/tests/fixtures/config.optional-override-null.json`

Same two entries as above.

#### 8. `src/config.rs` — update doc_paths count assertion

**File**: `skills/visualisation/visualise/server/src/config.rs`

Line 226: change `assert_eq!(c.doc_paths.len(), 9)` → `assert_eq!(c.doc_paths.len(), 11)`.

#### 9. `tests/config_contract.rs` — update doc_paths count assertion

**File**: `skills/visualisation/visualise/server/tests/config_contract.rs`

Line 45 (`assert_eq!` start, value `10` on line 47): change the literal `10` → `12`.

Also add the two new keys to the checked list at line 51:
```rust
"design_gaps",
"design_inventories",
```

#### 10. `tests/api_types.rs` — update type count assertion and add property checks

**File**: `skills/visualisation/visualise/server/tests/api_types.rs`

Line 13: rename test from `types_returns_eleven_entries_with_virtual_flag_on_templates` → `types_returns_thirteen_entries_with_virtual_flag_on_templates`.
Line 32: change `arr.len() == 11` → `arr.len() == 13`.

After the existing property assertions (e.g. the `decisions` check), add assertions for the two new types:
```rust
let design_gaps = arr.iter().find(|t| t["key"] == "design-gaps").unwrap();
assert_eq!(design_gaps["virtual"], false);
assert_eq!(design_gaps["inLifecycle"], true);
assert!(design_gaps["dirPath"].is_string());

let design_inventories = arr.iter().find(|t| t["key"] == "design-inventories").unwrap();
assert_eq!(design_inventories["virtual"], false);
assert_eq!(design_inventories["inLifecycle"], true);
assert!(design_inventories["dirPath"].is_string());
```

#### 11. `frontend/src/api/types.ts` — register two new type keys

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`

`DocTypeKey` union (lines 4-7): add `'design-gaps'` and `'design-inventories'`.

`DOC_TYPE_KEYS` array (lines 13-17): add both strings.

`Completeness` interface (lines 100-110): add two new boolean fields:
```typescript
hasDesignInventory: boolean
hasDesignGap: boolean
```

`PipelineStepKey` type (lines 130-133): add `'hasDesignInventory'` then `'hasDesignGap'` (inventory before gap, matching the `Completeness` interface field order and the `LIFECYCLE_PIPELINE_STEPS` ordering below).

`LIFECYCLE_PIPELINE_STEPS` array (lines 135-151): append two long-tail entries after the `hasNotes` entry:
```typescript
{ key: 'hasDesignInventory', docType: 'design-inventories', label: 'Design inventory', placeholder: 'no design inventory yet', longTail: true },
{ key: 'hasDesignGap',       docType: 'design-gaps',         label: 'Design gap',       placeholder: 'no design gap yet',       longTail: true },
```

**`LifecycleIndex.test.tsx` — update `empty` Completeness fixture**: The `empty` object literal typed as `Completeness` in this file must include the two new required fields or the TypeScript compiler will reject it. Add to the `empty` object:
```typescript
hasDesignInventory: false,
hasDesignGap: false,
```

**`LifecycleIndex.test.tsx` — add long-tail rendering test**: Add a test that renders a cluster with `hasDesignInventory: true` and `hasDesignGap: true` and asserts the corresponding long-tail list items appear with the correct labels:
```typescript
it('shows design inventory and design gap in the long-tail section when present', () => {
  // render a lifecycle card with both new flags set to true
  // assert list items with text 'Design inventory' and 'Design gap' are present
});
```

The exact rendering implementation depends on how the existing long-tail section tests are structured — follow the same pattern used for the `hasNotes` long-tail case.

### Success Criteria

#### Automated Verification

- [ ] All server tests pass: `cargo test -p accelerator-visualiser`
- [ ] Contract test asserts 12 doc_paths: `cargo test -p accelerator-visualiser config_contract`
- [ ] Frontend tests pass: `npm test` (run from `skills/visualisation/visualise/frontend/`)

#### Manual Verification

- [ ] "Design gaps" and "Design inventories" appear in the visualiser sidebar
- [ ] Creating a `.md` file in `meta/design-gaps/` causes it to appear in the library under "Design gaps"
- [ ] Creating a `.md` file in `meta/design-inventories/` causes it to appear under "Design inventories"
- [ ] Both doc types appear in lifecycle clusters when their filenames share a slug with other docs
- [ ] No regressions on existing doc types, kanban, or lifecycle views

---

## References

- Templates on disk: `templates/` (8 files)
- Config script: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
- SKILL.md: `skills/visualisation/visualise/SKILL.md:14-25`
- Server doc type enum: `skills/visualisation/visualise/server/src/docs.rs:6-80`
- Server cluster logic: `skills/visualisation/visualise/server/src/clusters.rs:67-125`
- Contract test: `skills/visualisation/visualise/server/tests/config_contract.rs`
- API types test: `skills/visualisation/visualise/server/tests/api_types.rs`
- Test common helper: `skills/visualisation/visualise/server/tests/common/mod.rs`
- JSON fixtures: `skills/visualisation/visualise/server/tests/fixtures/config.valid.json`, `config.optional-override-null.json`
- Template fixtures dir: `skills/visualisation/visualise/server/tests/fixtures/templates/`
- Frontend types: `skills/visualisation/visualise/frontend/src/api/types.ts`
- Frontend list view: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`
