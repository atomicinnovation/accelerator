---
type: pr-description
id: "88"
title: "Bound Linear untracked discovery to the configured team"
date: "2026-08-30T23:33:36+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "work-item:0220"
parent: "work-item:0220"
relates_to: ["work-item:0146"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/88"
pr_number: 88
tags: [sync, linear, jira, tracker, discovery]
revision: "65916547e336fffe5244219d1385427c0e00a14a"
repository: "accelerator"
last_updated: "2026-08-30T23:33:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bound Linear untracked discovery to the configured team

## Summary

Untracked-remote discovery returned zero pulls on Linear for two independent reasons: with the scope key unset the discovery gate skipped the search silently, and with the key set it was handed to Linear as a team **key** where the filter wanted the team **UUID**, matching no team. This PR lifts key→UUID resolution into a new `resolve_scope` port method that validates the discovery scope before any push — a missing or unresolvable key refuses the run pre-flight (exit `74`, nothing sent), a resolved key drives a UUID-bounded search — and makes every discovery outcome (ran, skipped, transiently failed) visible in the sync report so a skip is never again mistaken for an empty result.

## Changes

- Added `resolve_scope` and a dedicated `ScopeError` to the `RemoteTracker` port — a deterministic, local resolution-and-validation step distinct from the network `search`, so a config fault can never be routed through the transient-failure exit path. Public-api snapshot regenerated.
- Linear: introduced a `TeamResolver` port plus a catalogue-backed `CatalogueTeam` resolver injected into `LinearClient`; `resolve_scope` maps the configured team key to its UUID and refuses an unknown or absent key (`E_SEARCH_NO_TEAM` / `E_SEARCH_UNKNOWN_TEAM`). `search` now consumes the resolved UUID and drops the credential-team fallback, keeping a defensive guard so an unresolved scope can never reach Linear's empty, workspace-wide filter.
- Jira: `resolve_scope` moves the `E_JQL_NO_PROJECT` refusal ahead of `search`, so an unscoped bidirectional Jira run is refused pre-flight rather than issuing an unbounded search.
- Pre-flight refusal in `prepare_run`: an invalid scope aborts as `RunError::DiscoveryUnconfigured` before the apply/push phase, rendered by the CLI as exit `74` with nothing sent.
- Discovery observability: a `DiscoveryStatus` (ran / skipped-push-only / transiently-failed) threaded through the report and rendered as a `#\tdiscovery` line; a transient search failure now drives exit `70` distinctly instead of folding silently into an empty result.
- Docs: the exit-code taxonomy (`exit_codes.rs`) and the `sync-work-items` skill reconciled for `74`'s second source and the new report line.
- Meta: the full 0220 lifecycle (research, plan, two reviews, validation) plus a decomposition of epic 0146 into children 0228/0229/0230 (layered configuration-key model, per-tracker pull scope, tracker-owned work-item id generation).

## Context

- Work item: `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md`
- Plan: `meta/plans/2026-08-30-0220-untracked-remote-discovery-on-linear.md`
- Research: `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md`
- Validation: `meta/validations/2026-08-30-0220-untracked-remote-discovery-on-linear-validation.md`
- Parent epic: `meta/work/0146-work-item-sync-enhancements.md`

## Testing

- [x] `mise run test:unit:cli` — full CLI unit suite green, 0 failures
- [x] `mise run cli:check` — workspace rustfmt + clippy clean
- [x] `mise run public-api:check` — regenerated `tracker` snapshot matches
- [ ] Manual: against a real Linear workspace with the team key set and a seeded untracked issue, `work sync --preview` lists it as a `create-from-remote` pull and the emitted GraphQL body carries the team UUID in `{team:{id:{eq:…}}}`
- [ ] Manual: a key-unset (or unknown-key) run refuses pre-flight with a `discovery unconfigured` message, exits `74`, and sends no push

## Notes for Reviewers

- Behavioural contract change to release-note: an unconfigured discovery scope now refuses a bidirectional or pull-only run pre-flight (exit `74`) where it completed cleanly (exit `0`) before — on **both** trackers, so an unscoped Jira run shifts `0 → 74`. The escape hatch is setting the key or running `--push-only`.
- The port surface grew deliberately (`resolve_scope` + `ScopeError`); the public-api snapshot regeneration is the intended diff, not drift.
- Follow-up debt: the two `RunReport` constructions were left un-collapsed (the optional `from_prepared` refactor was deferred); both thread the new `discovery` field correctly.
- The 0146 decomposition (0228/0229/0230) rides along because the plan's scope-authority decisions map onto that epic's config redesign; reconciliation of the `work.default_project_code` → `work.key` rename is recorded in the plan's Assumptions.
