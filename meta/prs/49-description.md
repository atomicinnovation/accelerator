---
type: "pr-description"
id: "49"
title: "Update VCS CLI rewrite and documentation stories"
date: "2026-08-05T19:08:29+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0187", "work-item:0188", "work-item:0192", "work-item:0193", "work-item:0136"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/49"
pr_number: 49
tags: ["meta", "work-items", "housekeeping"]
revision: "6e3f9ed5f3d767c7bb0be4072599d948636d5cc3"
repository: "accelerator"
last_updated: "2026-08-05T19:14:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Update VCS CLI rewrite and documentation stories

## Summary

Two independent pieces of `meta/` corpus housekeeping. First, it resolves a genuine work-item ID collision: `0178` and `0179` were each claimed by two different work items — the Rust CLI crate stories and the documentation epic/story — so the documentation pair is renumbered to the next free IDs, `0192` and `0193`. Second, it marks `0187` and `0188` done now that both have landed, which clears the last two blockers gating the VCS subdomain migration.

No product code changes — every file is Markdown under `meta/`.

## Changes

**Renumber the documentation work items (`0178` → `0192`, `0179` → `0193`)**

- Renamed `meta/work/0178-documentation.md` → `0192-documentation.md` and `meta/work/0179-make-the-docs-amazing.md` → `0193-make-the-docs-amazing.md`, updating each file's `id:` and body H1 to match.
- Renamed the derived artifacts to keep the filename/ID convention intact: `meta/plans/2026-07-10-0179-make-the-docs-amazing.md` and `meta/research/codebase/2026-07-10-0179-make-the-docs-amazing.md` both move to the `-0193-` form, with their `id:`, `work_item_id:`, `parent:`, and `derived_from:` slots repointed.
- Repointed the inbound `parent:` links on the two children of the docs epic — `0145-documentation-improvements.md` and `0177-documentation-site-for-docs-tree.md` — from `work-item:0178` to `work-item:0192`.
- Updated the prose references throughout the plan and both research documents (`2026-07-10-…-make-the-docs-amazing`, `2026-07-13-docs-site-visualiser-design-alignment`) so narrative mentions of "epic 0178" / "work item 0179" now read 0192 / 0193.

**Mark `0187` and `0188` done**

- `0187` (Generalise the Sub-Binary Registration Surface) and `0188` (Library-Backed VCS Adapter over gix and jj-lib) both move `status: ready` → `done`, with the body `**Status**:` label synced and `last_updated` bumped.

## Context

- Epic `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` — owns `0187` and `0188`. With both done, the dependency graph opens up: `0169` (VCS Subdomain and Hooks Migration) has all seven of its blockers satisfied and is the only newly-unblocked item already at `ready`. `0170`, `0171`, `0173`, and `0185` also clear, though all four are still `draft` and need refinement before they are workable. `0172` and `0174` remain blocked behind `0169`.
- Epic `meta/work/0192-documentation.md` — the renumbered documentation umbrella, with children `0145`, `0177`, and `0193`.
- Plan `meta/plans/2026-07-10-0193-make-the-docs-amazing.md` and research `meta/research/codebase/2026-07-10-0193-make-the-docs-amazing.md` — the active docs-site work that the renumber keeps addressable.

## Testing

- [x] Confirmed the collision was real: on `main`, `meta/work/` contained both `0178-config-crates-native-yaml-reader.md` and `0178-documentation.md`, and both `0179-corpus-crates-parsing-conventions.md` and `0179-make-the-docs-amazing.md`.
- [x] Confirmed `0192`/`0193` were the next free IDs — the highest previously allocated work item is `0191`.
- [x] Grepped the repo for stale references to the old docs IDs (`0178-documentation`, `0179-make-the-docs-amazing`). The only surviving hits are the two `docs/0179-docs-polish` branch-name mentions in `2026-07-13-docs-site-visualiser-design-alignment.md`, which are a historical VCS branch name and correctly left alone.
- [x] Verified every remaining `"work-item:0178"` / `"work-item:0179"` typed reference in `meta/` now unambiguously resolves to the crate work items (plans, research, reviews, and PR descriptions for `0178`/`0179`/`0180`, plus `blocked_by` slots on `0167`, `0180`, `0185`, and `0188`).
- [x] `mise run check` passes (exit 0, 78.8s) — format, lint, and type-checks across frontend, server, cli, build-system, and scripts. It exercises none of the changed files, since every one of them is Markdown under `meta/`, but it confirms the branch is clean.

## Notes for Reviewers

- **Two unrelated changes in one PR.** The documentation renumber and the `0187`/`0188` status flips share no files and no rationale; they are simply co-resident on this branch. Each commit is self-contained, so they can be split if you would rather review them separately.
- **The renumber is the part worth reading carefully.** It is a rename-plus-rewrite across seven files, and the failure mode is a dangling reference rather than a broken build — nothing in CI validates work-item linkage, and `mise run check` never reads `meta/`, so the grep evidence above is the only guard.
- **`last_updated` was deliberately not bumped on the renumbered files.** `0192` and `0193` keep their original 2026-07-10 timestamps on the grounds that renumbering fixes identity rather than changing content. Worth a second opinion if you read that field as "last touched".
- **The `docs/0179-docs-polish` branch name is intentionally stale.** It names a real branch and would be wrong to rewrite.
- **Follow-up.** With `0169` now genuinely unblocked, it is the natural next item off this epic.
