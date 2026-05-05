---
date: "2026-05-05T00:00:00Z"
type: plan
skill: create-plan
work-item: ""
status: draft
---

# Visualiser Work Item Label and Template Fixes

## Overview

Two small post-rename issues in the visualiser library:

1. `DocTypeKey::WorkItemReviews` has the label `"Work-item reviews"` (hyphen) but should match the style of `"Work items"` — i.e. `"Work item reviews"` (no hyphen).
2. The `write-visualiser-config.sh` script builds the `templates` JSON object with five entries (`adr`, `plan`, `research`, `validation`, `pr-description`) but omits `work-item`, which has a plugin default at `templates/work-item.md`.

## Current State Analysis

- **Label**: `server/src/docs.rs:61` — `DocTypeKey::WorkItemReviews => "Work-item reviews"`. No test asserts this specific string; the fix is a one-character change.
- **Template omission**: `scripts/write-visualiser-config.sh` — the `template_tier` helper is called for five names; `work-item` is absent. The file `templates/work-item.md` exists. The test in `scripts/test-launch-server.sh` (lines 76–80) asserts `user_override` paths for those five templates but does not cover `work-item`.

## Desired End State

- The library sidebar shows "Work item reviews" (no hyphen) for `work-item-reviews` documents.
- The templates API endpoint includes a `work-item` entry alongside the existing five.
- `test-launch-server.sh` asserts the `work-item` template `user_override` path.

### Key Discoveries

- Label defined: `server/src/docs.rs:61`
- Template wiring: `scripts/write-visualiser-config.sh` lines ~70–140
- Template test: `scripts/test-launch-server.sh` lines 76–80
- Plugin default template: `templates/work-item.md` (exists)

## What We're NOT Doing

- Renaming or reordering any other labels.
- Adding config support for a user-overridable `work-item` template path (the `template_tier` helper already handles that correctly once the entry is added).
- Touching the Rust template resolver logic — it is data-driven from the config map.

## Implementation Approach

Two independent one-liner fixes plus a one-line test addition. No phasing needed — changes are tiny and orthogonal.

---

## Change 1: Fix the label

**File**: `skills/visualisation/visualise/server/src/docs.rs`  
**Line**: 61  
**Change**: Remove the hyphen from `"Work-item reviews"`.

```rust
// before
DocTypeKey::WorkItemReviews => "Work-item reviews",
// after
DocTypeKey::WorkItemReviews => "Work item reviews",
```

### Success Criteria

#### Automated Verification

- [x] Server unit tests pass: `cd skills/visualisation/visualise/server && cargo test`

#### Manual Verification

- [ ] Library sidebar shows "Work item reviews" (no hyphen) for `work-item-reviews` documents.

---

## Change 2: Add work-item to the templates config

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`

Two additions:

1. After the `PRD="$(template_tier pr-description)"` line, add:

```bash
WI="$(template_tier work-item)"
```

2. In the final `jq -n` call, add `--argjson work_item "$WI"` to the arguments and `"work-item": $work_item` to the `templates` object:

```bash
# argument line (add alongside --argjson pr_description "$PRD"):
--argjson work_item "$WI" \

# inside templates object (add alongside "pr-description": $pr_description):
"work-item": $work_item
```

**File**: `skills/visualisation/visualise/scripts/test-launch-server.sh`

Add one assertion alongside lines 76–80:

```bash
assert_json_eq "config: work-item user_override" '.templates."work-item".user_override' "$PROJ/meta/templates/work-item.md" "$CFG_FILE"
```

### Success Criteria

#### Automated Verification

- [x] Config script test passes: `bash skills/visualisation/visualise/scripts/test-launch-server.sh`
- [ ] Templates API returns a `work-item` entry when server runs against a real project.

#### Manual Verification

- [ ] Templates library view in the visualiser includes a `work-item` entry.
- [ ] Plugin default tier is marked active when no user or config override exists.

---

## References

- Label source: `skills/visualisation/visualise/server/src/docs.rs:61`
- Template wiring: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
- Template test: `skills/visualisation/visualise/scripts/test-launch-server.sh:76-80`
- Plugin default template: `templates/work-item.md`
