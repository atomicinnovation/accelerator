---
type: pr-description
id: "18"
title: "Update work items"
date: "2026-07-10T21:35:56+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
pr_url: "https://github.com/atomicinnovation/accelerator/pull/18"
pr_number: 18
tags: []
revision: "b77b5f456ba97ce1692e074de00ac81000177574"
repository: "accelerator"
last_updated: "2026-07-10T21:35:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Update work items

## Summary

Work-item housekeeping in `meta/work/`: restructures the work-management epic by splitting its unfinished integrations into a new epic, marks several completed items as done, records the corresponding Linear sync state, and relocates the design skill's test fixture into its eval directory. No product code (skills, hooks, server, CLI) behaviour changes.

## Changes

- **Split additional integrations out of the work-management epic.** Carves the unfinished Trello and GitHub Issues integrations into a new epic, *Work Management Additional Integrations* (`0181`, Linear `PP-701`), and reparents stories `0049` (Trello) and `0050` (GitHub Issues) under it. Trims the original epic `0045` to its shipped Jira + Linear scope and marks it **done** (Linear `PP-67`). The new epic was initially numbered `0178` and later renumbered to `0181`, freeing `0178` for the config-crates work.
- **Marked completed work items done.** `0164` (launcher + git-style dispatch, `PP-185`), `0165` (multi-binary distribution + release pipeline, `PP-186`), and `0178` (config/config-adapters native YAML reader, `PP-702`) transitioned to **done**.
- **Linked config-crates work items to Linear.** Records `external_id`s for `0178`/`0179`/`0180` → `PP-702`/`PP-703`/`PP-704`.
- **Relocated the design test fixture.** Moves `examples/design-test-app/` to `skills/design/inventory-design/evals/fixtures/design-test-app/` — it is used only by the inventory-design evals and the design skill's path-validation test — and rewires both consumers (`evals.json`, `scripts/test-design.sh`) to the new location.
- **Refreshed the Linear sync baseline** (`.accelerator/state/integrations/linear/last-sync.json`) to reflect the pushes above plus the earlier transitions of `0175`/`0176` to done.

## Context

Continues the Linear integration (0048) and work-management epic (0045) tracks, and aligns the Rust CLI migration epic (0136) work items (`0164`, `0165`) and the config-crates work (`0166` children `0178`–`0180`) with their remote Linear issues. This is a bookkeeping PR that reconciles local `meta/work/` state with Linear rather than shipping a feature.

## Testing

- [x] Verified end-state frontmatter for every changed work item (status, parent, `external_id`, tags) is internally consistent — epic `0045` done, new epic `0181` (`PP-701`) parenting `0049`/`0050`, and `0164`/`0165`/`0178` done with their Linear IDs.
- [x] Confirmed the design fixture relocation: old `examples/design-test-app/` is gone, the fixture exists at the new evals path, and both active consumers (`evals.json`, `scripts/test-design.sh`) reference the new location — the only remaining old-path references are in historical `meta/plans/` documents, which are point-in-time records left untouched.
- [ ] Full `mise run check` / design eval suite — left to CI (no product code changed; content is `meta/` markdown, sync-baseline JSON, and a fixture move).

## Notes for Reviewers

- The bulk of the diff by size is the single-line `.accelerator/state/integrations/linear/last-sync.json` baseline; the meaningful review surface is the `meta/work/*.md` frontmatter/body edits and the fixture relocation.
- Note the epic renumber: the additional-integrations epic moved `0178 → 0181`, and `0178` was subsequently reused for the config-crates YAML reader task — both files appear in the diff and are the intended final numbering.
