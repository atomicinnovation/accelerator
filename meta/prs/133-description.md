---
type: "pr-description"
id: "133"
title: "[0292] Resolve Linear pull filters to catalogue ids and add the project filter"
date: "2026-09-24T18:52:11+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0292"
parent: "work-item:0292"
relates_to: ["work-item:0294"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/133"
pr_number: 133
tags: ["linear", "pull-filters", "catalogue", "sync"]
revision: "832fc3a25a46224f764c932b2760268a5cdd6583"
repository: "accelerator"
last_updated: "2026-09-24T18:52:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0292] Resolve Linear pull filters to catalogue ids and add the project filter

## Summary

Every Linear pull filter now lowers to ids. `project` is new; `label`,
`assignee` and `state` used to lower to names or to one init-team state id.
Each value resolves per team in scope against a catalogue that records every
synced team's states, labels, members and projects. A value that resolves to
nothing refuses the sync with exit 74 rather than being dropped silently or
reported as "cut short… retry". It also fixes multi-team `state` pulls, which
used to match only the init team's state.

## Changes

- **`catalogue.json` has one shape and one writer** (`linear-client`).
  - A `baseTeam` pointer and a `teams` array of per-team entries, each
    carrying `states`, `labels`, `members` and `projects`, plus a top-level
    workspace `labels` array.
  - Only **synced teams** are catalogued: the base team, and each team that
    owns a tracked work item.
  - The legacy `team` and `workflowStates` keys are written as projections
    of the base entry for one minor release; 0294 removes them.
  - `init-linear` and sync share one locked, lossless, section-by-section
    write. It keeps unknown keys at every level, writes keys alphabetically
    and entries in id order, and refuses a catalogue it cannot parse, naming
    the recovery.
- **Batched team-section fetch** (`discovery.rs`). A spike against the live
  tenant settled one paginated query per section, attributed to teams. Each
  section has a page ceiling, one deadline spans the whole fetch, and a
  nested connection past one page fails loud rather than truncating.
- **Resolution domain** (`resolution.rs`, `catalogue/`).
  - `FilterFamily`, `NonEmpty`/`ResolvedIds`, `TeamScopedResolver` and a
    `ResolverSet` that answers identity, coverage and per-family resolution
    from one set of entries.
  - Active records win over archived or disabled ones of the same name.
    Assignees match on email, then full name, then display name.
- **Two-stage resolution** (`filter.rs`, `client.rs`).
  - *Pre-flight* in `resolve_scope` validates every filter's structure. When
    the scope is base-only and the base entry covers the configured
    families, it also resolves every value and refuses before any request.
  - *Completion* in `search` fetches only the sections that uncovered scoped
    teams need, completes each value over every scoped team, and hands
    `compose` a `LowerableSearch` of ids only.
- **A new port class, `TrackerError::Unconfigured`** (`tracker`). It is the
  read-side configuration refusal. Every exhaustive match gains an arm, sync
  maps it to `DiscoveryUnconfigured` (exit 74), and `exit_code_for_report`
  ranks 71 > 4 > 74 > 70. The public-API snapshot changes deliberately.
- **Self-healing catalogue** (`healing.rs`, `work-cli/src/finaliser.rs`).
  - An apply-mode sync hands what `search` fetched to `CatalogueHealing`,
    through a provider-neutral `RunFinaliser`, which replaces
    `grow_linear_catalogue`.
  - Healing confirms each identifier prefix against Linear with
    `issue(id:)`, at most three lookups per prefix. It then records complete
    entries for synced teams only, and names the teams it recorded in a
    `note:`.
  - Preview and push-only runs never write.
- **`project` accepted under Linear only** (`tracker-support/src/pull.rs`).
  The accepted keys are now split per tracker, and Jira still refuses
  `project` at validation.
- **Standalone `accelerator linear search`** scopes `--label` and
  `--assignee` to the init team, as `--state` already was. An unknown or
  ambiguous value exits 89, and a missing catalogue team exits 77.
- **Test harness** (`http-test-support`, `cli-test-support`). GraphQL routes
  are keyed on the operation name, unmatched operations panic on drop, and
  `Route::Delayed` serves slow responses.
- **Docs and changelog.** The `init-linear`, `search-linear-issues`,
  `sync-work-items` and `configure` skills are updated, and `CHANGELOG.md`
  gains `Added`, `Changed`, `Migrations` and `Security` entries.
- **Repository state.**
  - The committed `catalogue.json` is refreshed into the per-team shape.
  - `.accelerator/config.md` now filters Linear pulls to the `Accelerator`
    project.
  - The branch also carries the sync write-back for drafts 0285–0291 and a
    one-line fix pinning `_tree_artifacts_staged` in the prerelease-sign
    test.

## Context

- Work item: `meta/work/0292-linear-project-pull-filter.md`
- Plan: `meta/plans/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids.md`
  (status `done`), with the spike outcome recorded inline.
- Research: `meta/research/codebase/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids.md`
- Plan reviews: `meta/reviews/plans/…-review-1.md` and `…-review-2.md`
- Validation: `meta/validations/2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-validation.md`
  (result `pass`)
- Follow-up: `meta/work/0294-remove-the-legacy-linear-catalogue-projections.md`

## Testing

- [x] Full local CI mirror: `mise run` exits 0. That covers format, lint,
  types, the docs lane, and every test suite.
- [x] Real-client sync tests against `MockServer`. Ids are on the wire for
  every scope shape, including whole-workspace. A legacy catalogue fetches
  and succeeds, and a rebuilt client makes no section fetch.
- [x] Binary-level tests. An unknown Linear `label` exits 74, and a Linear
  `project` filter reaches resolution. A Jira `project` filter fails before
  discovery.
- [x] Live-tenant checks.
  - A filtered preview on a legacy catalogue leaves it byte-identical.
  - An apply pull heals the base entry, and a third pull writes nothing.
  - A whole-workspace `state` pull covers other teams but catalogues only
    synced ones, and a deleted team entry is re-derived.
  - `search --state` stays on `PP`.
  - Jira refuses `project`, and a live `project` pull returns only that
    project's issues.
- [ ] Ambiguous `assignee` against the live tenant. It can't be set up
  there, because no two members share a name. Unit tests in
  `tests/catalogue.rs` cover it.

## Notes for Reviewers

- ⚠️ **Mixed plugin versions.**
  - A binary from before this change fails a sync whose config carries
    `project`, and this PR commits such a filter. Merge once everyone
    sharing the repo can upgrade.
  - A pre-0292 `init discover` overwrites the catalogue with the legacy
    shape. The next apply sync re-derives the other synced teams.
- ⚠️ **Assignee is a breaking change.** An assignee must now be a member of
  a team in scope. There is no fallback to other workspace users.
- 🔒 **The committed catalogue now holds member names and emails** for every
  synced team. A foreign `external_id` that happens to match a Linear
  identifier can get a team catalogued, so review the `note:` before
  committing.
- **Where to focus.**
  - `complete_scope` in `client.rs`: partition, single fetch, one hold after
    success.
  - `CatalogueHealing::heal`: prefix confirmation, and the no-work path that
    takes no lock and writes nothing.
  - The `record_team_entries` merge rules in `catalogue/document.rs`.
- **Known limitations** (recorded in the validation report, not fixed here):
  - A renamed or foreign identifier prefix costs up to three lookups on
    every apply sync.
  - A heal that writes only workspace labels prints no `note:`.
  - A create refused as `Unconfigured` falls back to a local save, which no
    path can produce today.
