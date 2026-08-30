---
type: "pr-description"
id: "71"
title: "[0213] Conversational conflict-resolution flow for sync"
date: "2026-08-19T11:57:53+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0213"
parent: "work-item:0213"
relates_to: ["work-item:0171", "work-item:0212", "work-item:0194"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/71"
pr_number: 71
tags: ["skills", "sync", "work-items", "conflicts", "cli", "rust"]
revision: "5d4f6fdce5095499aca68f9568288c5d7fcc839e"
repository: "accelerator"
last_updated: "2026-08-19T11:57:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0213] Conversational conflict-resolution flow for sync

## Summary

Closes the report → prompt → resolve loop for `/sync-work-items`. `accelerator work sync` now materialises a renderable **conflict dossier** for every item it leaves as a `Prompt`, and the skill's conflict-resolution section reads those dossiers, prompts once per work item with the pinned typed token, and re-invokes with matching `--resolve <id>=<remote|local|skip>` orders. No new port operation, no second fetch — the dossier is built from the run's own gathered facts.

This lands Phases 1–4 of the plan. Phase 5 (the eval suite, its committed evidence, and the 0171 Decisions reconcile) is deferred to a follow-up focused on eval improvements; Phases 1–4 stand on their own.

## Changes

- **Freeze the report format (Phase 1).** A characterisation test over the private `render_report` against a new committed golden (`cli/work-cli/tests/fixtures/sync-report.golden`), covering every line shape plus the empty-corpus `synced` summary row, so any drift in the four-column report or a `SyncState` `Display` rename reddens rather than passing silently.
- **Dossier extraction and rendering (Phase 2).** A `ConflictDossier` type and a `dossiers` field on `RunReport`, populated for every `Prompt` item in a mode-independent pass (preview runs get dossiers too), plus an injectable `render_dossier` that downgrades to `Unrenderable` when `diff -u` is missing or the local file is unreadable — never fabricating a local side.
- **Persist, preview, argv (Phase 3).** At the CLI seam, dossiers are rendered through the real `diff_shellout` and written one file per item to a gitignored `conflicts/` directory, on preview and apply alike. The write is fail-closed (verifies the `*` ignore before clearing), token-safe (reuses the corpus canonical-id check), and clears only canonical-id dossiers so a user's own files under `conflicts/` survive. `--preview`'s doc is narrowed to "no work-item write".
- **Skill flow (Phase 4).** The conflict-resolution section of `sync-work-items/SKILL.md` is rewritten to read dossiers and drive `accelerator work sync --resolve`, preserving the pinned `[remote/local/skip]` token, framing both `remote` and `local` as destructive OVERWRITEs, treating the dossier body as untrusted data, and partitioning the full exit-code taxonomy. Guarding this prose against regression is deferred to the eval suite rather than a substring lint.

## Context

- Work item: `meta/work/0213-conversational-conflict-resolution-flow.md`
- Plan: `meta/plans/2026-08-19-0213-conversational-conflict-resolution-flow.md` (with its research, plan review, and validation report in `meta/`)
- Parent epic: `meta/work/0171-jira-and-linear-integrations.md`
- Coupled child (repoints the same skill file): `meta/work/0212-work-item-script-cutover.md`

## Testing

- [x] CLI format + clippy: `mise run cli:check`
- [x] CLI unit + integration tests: `mise run test:unit:cli` (the 17 plan-named tests — report golden, value-bound dossiers over `RecordingTracker` and the real client, `local_unreadable`/unrenderable downgrades, `id_is_token_safe`, the config-independent ignore, `persist_dossiers`, the canonical-id stale clear, and the four argv cases — all pass)
- [x] No public-API drift: `mise run public-api:check`
- [x] Architecture rules: `mise run pup:check`
- [x] Build-system unit tests: `mise run test:unit:tasks`
- [x] Build-system, skill-permissions, templates: `mise run build-system:check`, `mise run lint:skill-permissions:check`, `mise run test:unit:templates`
- [x] `git check-ignore` confirms the dossier path is ignored
- [ ] Manual: preview a real conflict (dossier written, gitignored, no work item mutated); the `diff`-removed unrenderable path; driving `/sync-work-items` end-to-end

## Notes for Reviewers

- **Phase 5 is deferred by design**, recorded in the plan and in commit `Defer the 0213 eval suite to a follow-up`. Until it lands, the behavioural expectations (six render fields, one prompt per conflict, one `--resolve` per choice) are covered by the static lint, not by an eval suite — `mise run` does not gate the eval flow.
- **`--preview` gains a local side effect**: it now writes and clears gitignored conflict dossiers. It still issues no push or pull and mutates no work item, and the clear is scoped to canonical-id `<id>.md` files, never a recursive wipe.
- **Coordination with 0212**: both PRs edit the same conflict-resolution section of `sync-work-items/SKILL.md`. 0213 owns the final dossier-driven form; the static lint guards it against either landing order.
- Focus areas: the fail-closed persist guarantee in `cli/work-cli/src/sync.rs`, and the untrusted-body / header-region-only handling in the skill rewrite.
