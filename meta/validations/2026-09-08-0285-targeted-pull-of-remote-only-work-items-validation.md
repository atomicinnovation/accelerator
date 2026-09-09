---
type: "plan-validation"
id: "2026-09-08-0285-targeted-pull-of-remote-only-work-items-validation"
title: "Validation Report: Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
date: "2026-09-09T08:06:15+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-09-08-0285-targeted-pull-of-remote-only-work-items"
target: "plan:2026-09-08-0285-targeted-pull-of-remote-only-work-items"
tags: ["work", "sync", "targeting", "pull"]
last_updated: "2026-09-09T08:06:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Targeted Pull of Remote-Only Work Items and Resolution Normalisation

### Implementation Status

✓ Phase 1: Local/local collision becomes a usage error — fully implemented
✓ Phase 2: Engine carries and imports targeted pull ids — fully implemented
✓ Phase 3: Resolve remote-only targets and gate them with `fetch_all` — fully implemented (one accepted deviation, marked `[~]` in the plan)

The three phases ship across four commits — `rpymnquy` (Phase 1), `rzmpkyku`
(Phase 2), `uvvknrqs` (Phase 3), and a follow-up `stmsvvsz` fixing a discovered
never-synced-integration gap. The change set touches exactly the files the plan
names: `cli/work-cli/src/sync.rs`, `cli/work-adapters/src/sync/run.rs`, the two
adapter test files, `cli/work-cli/tests/cli_sync_targets.rs`, and
`skills/work/sync-work-items/SKILL.md`.

### Automated Verification Results

✓ Read-only aggregate clean: `mise run check` (exit 0)
✓ `work-cli` suite green: `cargo test -p accelerator-work` (all binaries, 0 failed)
✓ `work-adapters` suite green: `cargo test -p work-adapters` (0 failed)

Every automated success-criterion command resolves to a test that exists and
passes. The plan's Phase 1–2 criteria name `cargo test -p work-cli`; the crate
publishes as `accelerator-work`, so those invocations run under that `-p` name.
Phase 3 criteria already use `-p accelerator-work`.

### Code Review Findings

#### Matches Plan:

- **Phase 1 collision → exit 2.** `a_local_local_collision_is_a_usage_error_naming_both_files` (`sync.rs:1023`) asserts a single `LocalCollision` failure, `exit_code() == USAGE`, and both `0001.md` and `0002.md` named. `an_own_external_id_token_reconciles_with_no_collision` covers the silent-reconcile arm.
- **`Suppressed` machinery deleted.** No `struct Suppressed`, no `suppression_line`, no `suppressed` parameter survive; grep across `sync.rs` and the target tests is clean. The suppressed-line test is gone.
- **Phase 2 `ItemSelection::Targeted { items, pull_ids }`.** The struct variant, the corpus filter at the injection branch (`run.rs:777`), and `DiscoveryStatus::TargetedPull { attempted }` emitted only for a non-empty confirmed set are all present. `discovery_search_suppressed` renamed from `discovery_suppressed`.
- **Phase 2 engine tests.** All eleven named behaviours have a test: create-and-count, over-`--max-pulls` refusal, preview zero-write, `show`-failure-reports-`Failed`, corpus-filter dedup, AC7 mixed union, one-of-two-fails tally, baseline-write recovery, discovery/pull authoring parity, re-run reconcile + watermark, empty-set `SkippedTargeted`.
- **Phase 3 resolution + gate.** `partition_candidates` (`sync.rs:649`), `push_only_remote_only` (exit 2, `sync.rs:636`), two-spellings dedup, `absent`→3 / `indeterminate`→70 with mixed-batch precedence, and the `run_sync`-driven remote-only pull are all covered.
- **Skill prose reconciled.** The suppressed-note sentences are gone; the discovery-suppression sentence now describes id-reachable targeted pull; precedence reads `2 > 6 > 3 > 70` (`SKILL.md:115`); exit 70 and the push-only remote-only exit-2 case are documented; `targeted-pull` appears in the preview change-class list and the discovery-line enumeration.

#### Deviations from Plan:

- **`[~]` whole-call `fetch_all` `Err` maps to exit 70, not a three-way split.** The plan's Phase 3 §2 proposed routing an unembeddable-id fault to exit 2 and a credential fault to 74. `TrackerError` carries no discriminant to separate these, and a credential fault is already caught at `registry.resolve` (74). Implemented as a single fold to `Indeterminate` (70) per an explicit user decision recorded in the plan; covered by `a_whole_call_fetch_all_error_folds_to_indeterminate_exit_seventy`. Accepted, not a gap.
- **`discovery_line` lives in `work-cli`, not `work-adapters`.** The plan's Phase 2 §2 placed the `targeted-pull` TSV arm in the engine and its test under `cargo test -p work-adapters`. The rendering (`sync.rs:181`) and the assertion (`render_report_emits_each_discovery_status_line`, `sync.rs:1490`, checking `#\tdiscovery\ttargeted-pull\t2`) sit in `accelerator-work`. Behaviour and coverage are identical; only the crate boundary differs.

#### Potential Issues:

- **Follow-up fix `stmsvvsz` addresses a gap the plan did not anticipate.** On a never-synced integration the state directory does not exist, and the atomic-write containment check canonicalises it as the trusted root, so the first baseline write from a targeted pull would fail. The fix `create_dir_all`s the directory up-front and `baseline_written` guards the create-through-baseline path. The single explanatory comment records the containment-canonicalisation invariant — the allowed comment category, not a what-comment.
- **No live-tracker coverage in the automated suite.** The post-credential gate is exercised only through `run_sync` with a stub `TrackerRegistry`, by design (the subprocess harness cannot inject a stub). The Phase 3 manual steps remain the only check against real Linear/Jira.

### Manual Testing Required:

1. Live remote-only pull (Phase 3 manual):
  - [ ] `work sync --target <a real remote-only key>` against Linear creates and reconciles the local file; a second run does not re-pull it.
  - [ ] `work sync --target <a typo'd key>` reports exit 3 naming the token with zero writes.

2. Skill rendering:
  - [ ] The `sync-work-items` skill renders the pull-create and the narrowed exit codes with human phrasing, no raw TSV leaking.

3. Collision (Phase 1 manual, credential-free):
  - [ ] `work sync --target <a local id> --target <a different file's external_id equal to that id>` exits 2 naming both files.

### Recommendations:

- Run the three live-tracker manual steps before merge; they are the only coverage of the real `fetch_all` round-trip and the `show` re-read inside `create_from_remote`.
- Consider correcting the plan's `-p work-cli` criterion crate name to `-p accelerator-work` if the plan is kept as a living reference; immaterial to the shipped code.
