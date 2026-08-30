---
type: "plan-validation"
id: "2026-08-19-0212-work-item-script-cutover-validation"
title: "Validation Report: Work-Item Script Cutover Implementation Plan"
date: "2026-08-21T16:08:23+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "plan:2026-08-19-0212-work-item-script-cutover"
target: "plan:2026-08-19-0212-work-item-script-cutover"
tags: ["rust", "cutover", "work-items", "fixtures", "cli", "tracker"]
last_updated: "2026-08-21T16:08:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Work-Item Script Cutover Implementation Plan

**Result: partial** — Phases 1–6 are fully implemented and verified green; Phase 7's offline seed harness is complete and its live credentialed run is deferred by design (infrastructure-gated, 0171-owned, explicitly non-blocking). No defect found. Plan status left at `ready` — one phase remains open.

### Implementation Status

- ✅ Phase 1: Relocate fixtures + convert parity tests to pure Rust — fully implemented
- ✅ Phase 2: Add `search`/`preview_create`/`validate_update` port operations — fully implemented
- ✅ Phase 3: Sync-engine orchestration seams (non-interactive) — fully implemented
- ✅ Phase 4: `work list` command — fully implemented
- ✅ Phase 5: Repoint skills + fold `EXIT_CODES.md` — fully implemented
- ✅ Phase 6: The irreversible deletions — fully implemented
- ⚠️ Phase 7: Credentialed corpus classification — offline harness done; live run deferred (needs scratch Jira/Linear tenants + tokens)

### Automated Verification Results

- ✅ `skills/work/scripts/` is gone; `skills/work/scripts/*.sh` matches nothing.
- ✅ No `cli/` file references `skills/work/scripts` (`git grep -l` empty).
- ✅ No work `SKILL.md` names a `work-item-*.sh` basename or `skills/work/scripts` (excl. `meta/`) — empty, cleaner than the plan's Progress predicted (no CHANGELOG/benchmark survivors in the working tree).
- ✅ No work `SKILL.md` declares `jq`, `curl`, or a `scripts/*` glob in `allowed-tools`; the surviving `jq` mention (`sync-work-items:57`) is prose stating the skill no longer pre-checks it.
- ✅ `cargo nextest run -p accelerator-work -p tracker -p tracker-test-support -p jira-client -p linear-client -p work-adapters --all-features` — **555 passed, 0 skipped, exit 0** (536s). Every named success-criterion test present and green.
- ✅ `cli/tracker/tests/fixtures/dispatch-codes.txt`, `skills/work/scripts/EXIT_CODES.md`, and `cli/tracker-support/tests/mapper_differential.rs` deleted.
- ✅ Build-system floor removed: `_EXPECTED_WORK_SUITES` and `work-item-bridge-codes.sh` gone from `tasks/test/integration.py` and `tasks/lint/scripts.py`.

Not independently re-run (deferred per the plan's mise-nesting note; validated by the passing crate suite + CI on the committed tree): the full `mise run` default, frontend/server/scripts/docs lanes. These were untouched by the change.

### Code Review Findings

#### Matches Plan

- **Port surface** — `SearchScope`, `Discovery`, `FieldResolution`, `CreatePreview`, `ValidationOutcome` and the three trait methods present in `cli/tracker/src/lib.rs`; `public-api.txt` carries exactly the additive surface.
- **Six impl sites** — `search`/`preview_create`/`validate_update` implemented on both real clients, `RecordingTracker`, `FixedTracker` (`tracker/tests/port.rs`), and the `work-adapters` test fakes (`sync_apply.rs`, `sync_create.rs`). No default bodies, no `unimplemented!` stub.
- **Action variants** — `CreateFromRemote`/`CreateFromLocal` in `decide.rs`, wired through `from_keyword`/`Display`/`action_keyword`, never returned by `decide()` (out-of-band), with the exhaustive apply-loop `unreachable!` arm. `work/tests/fixtures/public-api.txt` pins both.
- **`work list`** — `List(Box<ListArgs>)` in `cli.rs` with the enumerated filter surface and `--hierarchy`; `cli_surface.golden` includes `list` and the `--dry-run` flag.
- **Exit-code fold** — `exit_codes.rs` module doc carries the 0–5 / 70–71 / 72–74 taxonomy with the retryable/terminal safety semantics; nominated single authoritative source.
- **Skills repointed** — `sync-work-items` (5), `create-work-item` (19), `list-work-items` (8) invoke `accelerator work …`; stale `scripts/*` glob dropped from `review-work-item`/`extract-work-items` too.

#### Deviations from Plan

All deviations are documented in the plan's per-phase Implementation Progress section and each is deliberate. The load-bearing ones, verified against the tree:

- Both create paths are out-of-band; `decide()` untouched to preserve the frozen `sync-decide.golden` (Phase 3).
- File authoring sits behind a new `LocalAuthor` port (`work-adapters::sync::create`) implemented in the binary layer (Phase 3).
- Discovery gated on `work.default_project_code` scope — Jira runs untracked discovery, Linear (this repo) does not (Phase 3), matching the known multi-team untracked-flood mitigation.
- Jira issue-type resolution is local two-state (no catalogue endpoint wired); the project field is the full three-state remote check the AC targets (Phase 2).
- Create-preview is a `--dry-run` flag on `work create`, not a separate command; splits the bash exit-70 into transport-70 vs unresolvable-exit-0 (Phase 5).
- No committed report/render goldens (`render_report` is bin-private); covered by inline/engine assertions instead (Phases 3–5).

#### Potential Issues

- ❓ **Differential-oracle removal is now guarded only by the sha256 manifest.** The converted shellout tests no longer run bash; `bash_parity_baseline.rs` carries a content-hash manifest over every relocated golden plus a coverage check (broader than the plan's §2/§4 named). This is the intended tripwire and it passes, but a future code+golden co-regeneration would only be caught if the manifest is regenerated separately — the invariant now rests entirely on reviewers honouring the "never regenerate from Rust output" note. No action needed; flagged for awareness.

### Manual Testing Required

Phase 7 live credentialed run — deferred, needs scratch Jira/Linear tenants + tokens (0171-owned). The offline harness and guards are built and unit-tested (`tracker-test-support` seed guards, 24/24 + 28/28 with the live-gated tests as no-ops).

1. Seeded live classification:
  - [ ] `accelerator work sync` classifies every seeded item `synced`, issues no push/pull, exercises ≥1 absent-description item per provider.
  - [ ] `mise run test:integration:tracker-contract` green with `ACCELERATOR_TRACKER_CONTRACT=1` + resolved credentials.
2. Live `--preview` diagnostics:
  - [ ] `/sync-work-items --preview` surfaces an unresolvable Jira project key and a payload missing a required field before any mutation.
  - [ ] `/sync-work-items` lists an untracked remote issue (discovery).

Already done live: Jira `preview_create` three-state distinction proven against a real scratch tenant (`verify-jira-preview.sh`), closing Phase 2's carried-over manual item.

### Recommendations

- Merge Phases 1–6 on the green tree; they are self-contained and each merged with a passing build.
- Track the Phase 7 live run as the remaining developer-run gate under 0171 when scratch tenants are provisioned; it does not block the cutover.
- Consider a lightweight CI guard (or a documented pre-release checklist entry) that re-hashes the parity manifest independently, to harden the differential-oracle invariant against silent co-regeneration.
