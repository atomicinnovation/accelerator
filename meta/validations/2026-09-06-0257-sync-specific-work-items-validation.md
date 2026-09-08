---
type: "plan-validation"
id: "2026-09-06-0257-sync-specific-work-items-validation"
title: "Validation Report: Sync Specific Work Items Implementation Plan"
date: "2026-09-08T12:01:02+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-06-0257-sync-specific-work-items"
tags: ["sync", "cli", "work-sync"]
last_updated: "2026-09-08T12:01:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Sync Specific Work Items Implementation Plan

Result: **pass**. All five phases are implemented as one commit each
(`turluzmu` → `kpxwpzlo`), every automated success criterion is green, and the
code matches the plan's seams. Three deviations exist, all benign refinements
that the plan itself anticipated. Manual verification against a live tracker
remains outstanding — it cannot run in this environment.

### Implementation Status

- ✅ Phase 1: Work-directory containment for the Path resolve class — fully
  implemented (`turluzmu`).
- ✅ Phase 2: Per-item baseline watermark — fully implemented (`zzrupwys`).
- ✅ Phase 3: Engine `ItemSelection` and discovery suppression — fully
  implemented (`rkypqsuk`).
- ✅ Phase 4: `--target` flag and target resolution — fully implemented
  (`urwuwrkv`).
- ✅ Phase 5: `sync-work-items` skill surface — fully implemented (`kpxwpzlo`).

### Automated Verification Results

| Check | Command | Status |
| --- | --- | --- |
| CLI fmt + clippy | `mise run cli:check` | 🟢 exit 0 |
| Full read-only aggregate | `mise run check` | 🟢 exit 0 |
| Work crate tests | `cargo test -p accelerator-work -p work-adapters -p work` | 🟢 exit 0 |

Per-suite test results, all `0 failed`:

| Suite | Passed | Covers |
| --- | --- | --- |
| `cli_resolve` | 8 | Phase 1 containment (exit 6, traversal) |
| `cli_sync_targets` | 6 | Phase 4 credential-independent aborts (3/6/2, collect-all) |
| `cli_sync` | 11 | `sync --help` names exit 3 and 6 |
| `sync_run` | 23 | Phase 3 narrowing, discovery suppression, watermark regression |
| `sync_create` | 17 | Phase 3 double-binding guard under `Targeted` |
| `sync_fetch` | 8 | per-item watermark sourcing |
| `work-adapters` lib | 49 | Phase 2 `Entry` round-trip, backfill, corrupt-value |
| `accelerator-work` lib | 45 | resolution closure, suppression-line formatter |
| `work` lib | 78 | per-item gating in `classify` |

⚠️ `mise run check` emits one pre-existing `rustdoc::invalid_html_tags` warning
in the `corpus` lib doc (`cli/corpus/.../lib.rs:611`). It is unrelated to the
0257 change set — `corpus` is not touched by this plan — and the aggregate still
exits 0. Not attributable to this implementation.

### Code Review Findings

#### Matches Plan

- **Phase 1** — `RunOutcome::OutsideWorkDir` added; `resolve_path_class` takes a
  canonical `root`, rejects a candidate not under it before the `is_file` check,
  and returns the canonicalised path as the `Resolved` payload
  (`resolve.rs:33-49`). A work-dir canonicalisation fault surfaces through `Err`
  (exit 1), not `NotFound`, exactly as specified (`resolve.rs:73-85`).
- **Phase 1** — `RESOLVE_OUTSIDE_WORKDIR = 6` added; the module doc band header
  widened to `0`–`6` with the `6` bullet inside the band (`exit_codes.rs:8,18`).
  `main.rs` maps the variant. All three skill callers gained an Exit 6 branch;
  `create-work-item` stops rather than treating the path as a topic string
  (`create-work-item/SKILL.md:103-106`).
- **Phase 2** — `local_synced_at: u64` added to `Entry`; a missing *or
  non-integer* value backfills from the document `timestamp`, never a raw value
  (`baseline.rs:71-93`). Hand-built `render` keeps `timestamp`-before-`items`
  ordering that `project_remote` depends on (`baseline.rs:101-190`).
- **Phase 3** — `SyncRequest.items` split into `corpus` + payload-free
  `ItemSelection::All`, with `reconciled()` / `discovery_suppressed()`
  accessors. Reconciliation reads (`fetch::gather`, `plan_inputs`,
  `ItemIndex::build`, `validate_pushes`, apply loops, `unsynced_creates`) use
  `reconciled()`; whole-corpus reads (`corpus_carries`, `discover_untracked`)
  use `corpus` (`run.rs:98-139,752-916`). `SkippedTargeted` beats `PushOnly`
  (`run.rs:786-798`).
- **Phase 3** — watermark advance is scoped to `definitively_reconciled` items,
  closing the pre-existing read-failure hazard (`Indeterminate`/`Failed`/
  awaiting-human items keep their watermark) (`run.rs:218-253`).
- **Phase 4** — `resolve_targets` collect-all with `TargetResolutionFailure`
  and `exit_code()` as the sole code mapping; empty/blank guarded before
  `classify_input`; local-id-wins with a `Suppressed` note; ambiguous/outside
  do not cascade; de-duplication by resolved item id
  (`sync.rs:407-550`). `highest_precedence_code` orders `2 > 6 > 3`
  (`sync.rs:566-577`). `suppression_line` sanitises `token`/`local_id` through
  `single_line` (`sync.rs:609-616`).
- **Phase 4** — directory resolution relocated ahead of `registry.resolve`, so
  target validation precedes the credential check; `sync --help` names 3 and 6
  (`cli.rs:103-105`; `cli_surface.golden`).
- **Phase 5** — `argument-hint` gains `[--target …]…`; the targeted discovery
  line, exit codes 3/6/2, and the human-phrased summary rows are documented
  (`sync-work-items/SKILL.md`).

#### Deviations from Plan

- **Phase 1 resolver split (improvement).** `run` was not left inline; it was
  split into `canonical_work_dir` + the infallible `resolve_with`
  (`resolve.rs:54-132`). This realises the plan's own Phase 4 requirement to
  "resolve the scheme and work directory once up front", so the closure's
  `Fn(&str) -> RunOutcome` contract is honest.
- **Phase 3 `finalise_run` signature.** Implemented as `finalise_run(blank,
  advance_ids, run_start_epoch, advance_document: bool)` rather than the plan's
  suggested `Option<u64>` document watermark (`baseline_store.rs:100-123`).
  Semantically identical — `advance_document` is the `All`/`Targeted` signal —
  and still selection-agnostic at the persistence layer.
- **`Baseline::advance_watermark` helper added** (`baseline.rs:216-220`). A
  clean, single-purpose accessor the engine drives per id; not named in the plan
  but consistent with its per-item design.

#### Potential Issues

- None affecting correctness. The `corpus` rustdoc warning above is pre-existing
  and out of scope.

### Manual Testing Required

These require a live Linear/Jira tracker and were not run in this environment.

1. Targeted reconciliation:
   - [ ] `accelerator work sync --target 0257` reconciles only 0257; discovery
     line reads `skipped\ttargeted`.
   - [ ] `--target PP-787` (a remote id) reconciles the item whose
     `external_id` is `PP-787`.
   - [ ] `--target PP-999` (real remote issue, no local counterpart) aborts with
     the "untracked locally — run a full sync" message.
   - [ ] A dual-shape token prints a `#\ttarget\tsuppressed` line and reconciles
     the local-id item.
2. Aborts and parity:
   - [ ] `accelerator work resolve ../README.md` from `meta/work` exits 6.
   - [ ] Full sync → edit a non-targeted item → targeted sync of another → full
     sync: the non-targeted edit is still detected (watermark regression, by
     hand).
   - [ ] A full-sync run produces the same report and re-hash behaviour as
     before Phase 2.

### Recommendations

- Run the manual tracker checklist against the configured Linear tracker before
  merge, particularly the by-hand watermark regression (step 2), since it is the
  one behaviour the fake-tracker suite proves only indirectly.
- No code changes required; the implementation is faithful to the plan.
