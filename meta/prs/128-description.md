---
type: "pr-description"
id: "128"
title: "[0229] Per-tracker pull scope configuration"
date: "2026-09-20T20:30:28+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0229"
parent: "work-item:0229"
relates_to: ["work-item:0146", "work-item:0228", "work-item:0292", "work-item:0293"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/128"
pr_number: 128
tags: ["sync", "scoping", "tracker", "configuration", "discovery"]
revision: "1bb5bb5f2e6ceeb48aeea19a663d95cc85fd60ea"
repository: "accelerator"
last_updated: "2026-09-22T21:45:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0229] Per-tracker pull scope configuration

## Summary

Adds a per-tracker `pull` config block that governs how broadly a sync
discovers remote issues, replacing the hard-coded single-entity assumption
with an explicit, validated scope. Discovery can now broaden across additional
teams/projects or the whole accessible workspace, apply normalised filters, and
is bounded by configurable `max_items` / `max_pages` ceilings that fail loud
rather than truncating silently. Push gains the matching `max_items` bound, so
both write directions share one knob surface. This realises 0146's
long-dormant "restrict sync by label/project" requirement and promotes the
wired-but-unused `SearchScope.filters` seam to the primary discovery mechanism.

## Changes

- **Structured `pull` config surface.** Per-tracker block with
  `additional_teams` / `additional_projects`, an `all_teams` / `all_projects`
  flag, a flat `filters` bag, and `max_items` / `max_pages` ceilings. The config
  catalogue now carries structured map/array values; a personal block wholly
  replaces the team block with no field-level merge.
- **Configure-time structural validation.** Eight distinct rejection branches
  (unsupported filter key, `all` / `any` reserved keys, `all_*` vs
  `additional_*` mutual exclusion, `max_pages: 0`, wrong-tracker nouns, …), each
  naming the offending value, the accepted set, and the resolving config
  level/file. Wired into both the config command and the sync read path.
- **`unlimited` sentinel and config-sourced ceilings.** A
  `Ceiling { Bounded(usize), Unlimited }` type; `max_pages` defaults raised
  20 → 50 with independent `discovery` / `keyed_read` overrides. One
  `tracker-support::ceiling::from_token` conversion backs config validation,
  both block readers, and the CLI parser, so a ceiling string accepted in one
  place is accepted everywhere.
- **Symmetric pull/push write bounds.** `--max-pulls` / `--max-pushes` take the
  full config grammar (a non-negative integer, `0` refuses all, or
  `unlimited`). Push gains a `<tracker>.push.max_items` key with the same
  built-in default of 25, validated at sync time and surfaced in `config dump`.
  The bulk-overwrite refusal names both keys and the file each resolved from.
- **Fail-loud on silent truncation.** A tri-state `Completeness` on fetch and
  discovery outcomes promotes the two surviving silent truncations — standalone
  `search` and the keyed reconcile read — to hard, zero-write errors with
  dedicated exit codes (`SEARCH_CAP_HIT=79`, `KEYED_READ_CAPPED=7`). Search also
  reports a distinct `cap-hit` outcome keyword, keeping `truncated` for a
  transient cutoff.
- **Filters end-to-end (OR-within-a-key).** Keys AND'd, values within a key
  OR'd into an `IN` clause. Jira lowers via an explicit config-key → JQL-field
  map with a safe-identifier assertion and grammar-correct escaping; Linear
  lowers multi-value keys to `in`, single values to `eq`.
- **Broadened discovery + whole-workspace scope.** An exclusive
  `EntityScope { Keyed { base, additional }, WholeWorkspace }` sum type; a
  `enumerate_visible_entities` port method resolves `all_*` to an explicit
  enumerated `IN` list (never an unbounded query), preserving 0220's
  flood-guards. An unbounded-write gate (`unlimited` + broadened scope) refuses
  fail-safe with `REFUSED_UNBOUNDED=8` and drives the `--allow-unbounded`
  re-run. Push has no analogue, so it keeps no such gate.
- **Deterministic reconciliation.** Dedup by `canonical_external_key` precedes
  the local subtraction; discovered issues are totally ordered by
  `(prefix, sequence, raw id)`; `max_items` counts the post-dedup,
  post-subtraction set.
- **Skills and follow-ups.** `search-jira-issues` and `search-linear-issues`
  branch on the `cap-hit` / `truncated` outcome keywords; `sync-work-items`
  documents the unbounded refusal and the new bounds. New work items 0292
  (Linear project pull filter) and 0293 (negated pull filters) are filed under
  epic 0146.

## Context

- Implements **work item 0229** (`meta/work/0229-per-tracker-pull-scope-configuration.md`).
- Plan: `meta/plans/2026-09-11-0229-per-tracker-pull-scope-configuration.md`
  (all 8 phases, one commit each).
- Validation: `meta/validations/2026-09-11-0229-per-tracker-pull-scope-configuration-validation.md`
  (result: **partial** — see Notes for Reviewers).
- Layers on 0228's canonical-key base-scope model; delegates remote-existence
  checks of named entities to 0227's `accelerator config validate`.

## Testing

Re-run in full after rebasing onto `main` (`b72703bb`) at this PR's head tree.

- [x] `mise run` (full local CI mirror) — every task green except one, below
- [x] `mise run public-api:check` — `tracker` and `tracker-support` snapshots
  re-pinned for the new `UNLIMITED_TOKEN`, `block`, `ceiling`, and `push`
  surfaces
- [x] `lint:integration-skills:check` and
  `tests/unit/tasks/test_integration_skills.py` — the search skills cite no exit
  integers and every outcome keyword they branch on is declared
- [x] `linear-cli` / `jira-cli` suites — the cap-hit flow asserts
  `outcome: cap-hit` alongside exit 79; the keyword-surface goldens pin
  `cap-hit`
- [ ] `test:integration:tasks` — one local failure,
  `TestPrereleaseSign::test_signs_and_emits_manifest_under_secret_context`,
  caused by stale gitignored `dist/release/` tarballs being signed for real; all
  39 release tests pass with them moved aside, and CI's integration lane passes
- [ ] Live-tracker manual verification — Linear `IssueFilter` accepts `in` on
  `labels.name` / `state.id` / `team.id`; a hostile Jira filter value stays
  contained; `all_*` enumeration against a real workspace. Not yet run.

## Notes for Reviewers

⚠️ **Search output contract change.** A cap-hit now reports
`outcome: cap-hit` rather than `truncated`; the exit code (79) and the
`truncated: true` envelope field are unchanged. The two in-repo consumers — the
search skills — are updated. `main`'s integration-skill guard forbids skills
branching on exit integers, and the old shared `truncated` keyword left no
other way to tell a cap-hit from a transient cutoff.

⚠️ **Push bound now config-sourced.** An unconfigured push still defaults to 25,
but a `<tracker>.push.max_items` block can now raise, lower, or lift it, and a
malformed block fails the sync before any write.

The validation result is **partial**, not pass, for two reasons: the
`config dump` override annotation (Phase 1.4) was not built, and all
live-tracker manual verification is outstanding. The work item is left `ready`,
not `done`.

Three deviations from the plan text, two of them improvements:

- **Filter schema consolidated into `work`** rather than per-client
  `pull.rs` instances — a single shared `FILTER_SCHEMA` with
  `validate(&PullConfig, Tracker)` selecting internally (YAGNI; the accepted
  sets do not diverge per tracker today).
- **Unbounded-write gate lives in `work-cli`** (pre-flight) rather than
  `work-adapters`, with only the `is_broadened` predicate in `work-adapters`.
  Behaviour is identical.
- **`config dump` override shown per-row** via each row's `local (...)` source
  rather than a dedicated callout — the one gap above, matching the manual box
  the plan itself left unchecked.

Two unrelated flake fixes rode along; neither touches 0229 code:

- **`design-adapters::spawn_properties`** — three child-spawn tests starved
  under full-suite fork contention (a `setsid`-detached child past a 10s
  bounded wait). The bound was widened to 60s.
- **Launcher cache-root probe** — on Linux, a sibling thread forking between
  the probe's write and exec makes exec fail with `ETXTBSY`, which the probe
  misread as a non-executable cache root. This flaked
  `two_concurrent_cold_resolutions_issue_exactly_one_archive_fetch` on ubuntu
  CI (4 of the last 40 failed runs, across branches). The probe now retries a
  text-file-busy exec up to 10 times at 5ms intervals.
