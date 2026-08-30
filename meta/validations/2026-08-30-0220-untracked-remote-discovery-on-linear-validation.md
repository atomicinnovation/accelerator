---
type: plan-validation
id: "2026-08-30-0220-untracked-remote-discovery-on-linear-validation"
title: "Validation Report: Untracked-Remote Discovery on Linear"
date: "2026-08-30T23:24:29+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
target: "plan:2026-08-30-0220-untracked-remote-discovery-on-linear"
tags: [sync, linear, tracker, discovery]
last_updated: "2026-08-30T23:24:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Untracked-Remote Discovery on Linear

Both phases are fully implemented across three commits and every automated gate
is green. The implementation follows the plan's change list faithfully; three
benign deviations, all documented below, do not affect correctness.

### Implementation Status

✓ Phase 1: Discovery validates its scope pre-flight and refuses an invalid one — fully implemented (`zltwztst`)
✓ Phase 2: Discovery outcome is visible in the report — fully implemented (`skrntktv`)
✓ Cleanup: stale planning references removed from code comments (`vrtxpxrs`)

### Automated Verification Results

✅ `mise run test:unit:cli` — pass, 0 failures (211.93s, exit 0)
✅ `mise run cli:check` — pass, workspace rustfmt + clippy clean (31.85s, exit 0)
✅ `mise run public-api:check` — pass, `tracker` snapshot matches regenerated API (163.97s, exit 0)

The `tracker` public-api fixture carries the three new surfaces the plan
mandated: `ScopeError`, `RemoteTracker::resolve_scope`, and
`TrackerError::into_detail`.

### Code Review Findings

#### Matches Plan:

- `TeamResolver` port + `FixedTeam` double added beside `StateResolver` (`cli/linear-client/src/filter.rs`).
- `CatalogueTeam` catalogue-backed resolver reading `/team/key` + `/team/id` (`cli/linear-client/src/catalogue.rs`), injected into `LinearClient` and built in both `from_config` and `build_with_override` (`cli/linear-cli/src/context.rs:182`).
- `resolve_scope` + `ScopeError` added to the `RemoteTracker` port and all six impls; snapshot regenerated (`cli/tracker/tests/fixtures/public-api.txt`).
- Linear `resolve_scope` resolves key→UUID and refuses with `E_SEARCH_NO_TEAM` / `E_SEARCH_UNKNOWN_TEAM`; `search` consumes the resolved scope and keeps the defensive `None` guard (`E_SEARCH_UNRESOLVED_SCOPE`).
- Jira `resolve_scope` refuses an unscoped run with `E_JQL_NO_PROJECT` (`cli/jira-client/src/client.rs:592`).
- Pre-flight `resolve_scope` call in `prepare_run` maps a fault to `RunError::DiscoveryUnconfigured`, rendered to exit `74` (`cli/work-cli/src/sync.rs:546`).
- Phase 2 `DiscoveryStatus` enum (`Ran`/`SkippedPushOnly`/`Failed`), the `#\tdiscovery` render line, `single_line` sanitiser, `Failed`→`RETRYABLE` (70) exit drive, and the `RecordingTracker.failing_search` seam.
- Doc reconciliation: `exit_codes.rs` 74 band + line, `skills/work/sync-work-items/SKILL.md` Step 0/Step 2, and the `sync-report.golden` `discovery ran found=0` line.

#### Deviations from Plan:

- `ScopeError` gained `Display` + `std::error::Error` impls beyond the plan's `Debug`/`Clone`/`PartialEq`/`Eq` sketch (`cli/tracker/src/lib.rs:301-311`). Improvement — a proper error type; reflected in the snapshot.
- The Phase 2 discovery render is extracted into a named `discovery_line(&DiscoveryStatus)` helper (`cli/work-cli/src/sync.rs:179`) rather than the inline `match` the plan drafted. Stylistic improvement.
- The `resolve_scope` call sits at the existing gate (after `fetch::gather`), not hoisted above it. The plan explicitly left this to author judgement ("Hoist it unless the gate-locality is worth the wasted gather"); gate-locality was kept.

#### Potential Issues:

- ⚠️ Phase 2's optional step-6 refactor was **not** done: the two `RunReport { .. }` constructions (`cli/work-adapters/src/sync/run.rs:847,911`) were not collapsed into a shared `from_prepared` builder. Both thread the new `discovery` field correctly, so this is duplication debt, not a defect. The plan flagged it as a refactor-when-convenient step.
- A non-`PushOnly` run with an invalid scope still pays a full remote `fetch::gather` before the pre-flight refusal fires. Consistent with the kept gate-locality above; correctness ("nothing was sent") holds because the abort precedes the apply/push phase.

### Manual Testing Required:

1. Real-workspace discovery (Phase 1, AC-1/AC-2):
  - [ ] With `work.default_project_code` set to the team key and a seeded untracked issue, `accelerator work sync --preview` lists it as a `create-from-remote` pull.
  - [ ] The emitted GraphQL body carries the team **UUID** in `{team:{id:{eq:…}}}`, not the raw key.

2. Pre-flight refusal (Phase 1, AC-3/AC-5):
  - [ ] A bidirectional run with the key unset refuses with a `discovery unconfigured` message, exits `74`, and sends no push.
  - [ ] A key absent from the catalogue produces the same refusal, naming the key.

3. Report visibility (Phase 2, AC-7):
  - [ ] A normal keyed run prints `discovery ran found=N` and exits `0`.
  - [ ] A transiently failing discovery search prints a `discovery failed` line and exits `70`.

### Recommendations:

- Land the deferred `RunReport::from_prepared` builder in a follow-up to retire the two-site construction duplication; low-risk, no behaviour change.
- Release-note the exit-code contract change per the plan's Migration Notes: an unconfigured discovery scope now shifts `0 → 74` on **both** trackers (an unscoped Jira run included). Cross-reference epic 0146, whose `work.key` rename touches the same `work.default_project_code` field.
- Run the manual checks against a live Linear workspace before merge; they cover the one thing the `MockServer` cannot — that the resolved UUID actually filters the remote search.
