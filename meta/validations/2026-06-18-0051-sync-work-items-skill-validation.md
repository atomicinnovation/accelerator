---
type: plan-validation
id: "2026-06-18-0051-sync-work-items-skill-validation"
title: "Validation Report: Sync Work Items Skill Implementation Plan"
date: "2026-06-19T15:15:08+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-18-0051-sync-work-items-skill"
parent: "plan:2026-06-18-0051-sync-work-items-skill"
tags: [work-management, integrations, sync]
last_updated: "2026-06-19T15:15:08+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Sync Work Items Skill Implementation Plan

### Implementation Status

All eight phases are fully implemented, one commit per phase
(`owtvttqv` → `sszmwzvq`):

✓ Phase 1: Read-side and write-side integration bridges — Fully implemented
✓ Phase 2: Normalisation + content hashing — Fully implemented
✓ Phase 3: `last-sync.json` baseline store — Fully implemented
✓ Phase 4: Change-detection engine + label vocabulary — Fully implemented
✓ Phase 5: `/list-work-items` five-state extension — Fully implemented
✓ Phase 6: `/sync-work-items` core reconciliation — Fully implemented
✓ Phase 7: Conflict resolution UX — Fully implemented
✓ Phase 8: Unsynced batch push + untracked remote pull — Fully implemented

Every planned new file is present:
`work-item-fetch-remote.sh`, `work-item-update-remote.sh`,
`work-item-bridge-codes.sh`, `work-item-normalise.sh`,
`scripts/hash-common.sh`, `work-item-sync-baseline.sh`,
`work-item-sync-classify.sh`, `work-item-sync-decide.sh`,
`work-item-sync-apply.sh`, `work-item-file-dirty.sh`,
`work-item-section-diff.sh`, plus the new `sync-work-items/SKILL.md`,
the extended `work-item-sync-label.sh`, the wired `list-work-items/SKILL.md`,
and all the test files (`test-work-item-fetch-remote.sh`,
`test-work-item-update-remote.sh`, `test-hash-common.sh`,
`test-work-item-sync-apply.sh`).

### Automated Verification Results

All test suites pass:

✓ Fetch bridge: `bash skills/work/scripts/test-work-item-fetch-remote.sh` — all pass
✓ Update bridge: `bash skills/work/scripts/test-work-item-update-remote.sh` — all pass
✓ Work-item scripts: `bash skills/work/scripts/test-work-item-scripts.sh` — all pass
✓ Hash utility: `bash scripts/test-hash-common.sh` — all pass
✓ Sync apply/resumability: `bash skills/work/scripts/test-work-item-sync-apply.sh` — all pass
✓ Hierarchy fence byte-equality: `bash scripts/test-hierarchy-format.sh` — canonical fences match
✓ Config/registration: `bash scripts/test-config.sh` — all pass
✓ Linear search/show (the `updatedAt` add): `test-linear-search.sh` / `test-linear-show.sh` — all pass
✓ bashisms (3.2 floor) on all 11 new scripts — clean (exit 0)
✓ `mise run scripts:check` — green (format + bashisms + shellcheck, exit 0)
✓ `mise run types:build-system:check` (in isolation) — 0 errors

⚠️ `mise run check` (full parallel gate) exited 1 — but **only** on the known,
documented pyrefly/node_modules race: pyrefly globbed
`frontend/node_modules/decimal.js` mid-install and reported a phantom error.
Re-running `types:build-system:check` and `scripts:check` sequentially in
isolation both pass cleanly. This is infrastructure flakiness, not an
implementation defect (see the project memory note "pyrefly node_modules
race").

### Code Review Findings

#### Matches Plan:

- **Three symmetric bridges, one exit taxonomy.** `work-item-fetch-remote.sh`,
  `work-item-update-remote.sh`, `work-item-create-remote.sh`, and
  `work-item-push-decide.sh` all source the single shared
  `work-item-bridge-codes.sh` — the planned retrofit (no per-script copy left
  behind) is done.
- **Five-state label vocabulary** is present and pairwise distinct in both glyph
  and text (`🟢 synced`, `⚪ unsynced`, `🔵 locally modified`,
  `🟣 remotely modified`, `🔴 conflict`); distinctness/no-ANSI is suite-asserted.
- **Destructive-choice mapping is in tested code, not SKILL prose.**
  `work-item-sync-decide.sh resolve-conflict-token` maps
  `remote→accept-remote`, `local→push-local`, and empty/unknown→`skip`.
- **VCS-mode-aware dirty guard** resolves mode via `vcs_mode()` with
  `.jj`-present-wins (jj-colocated ⇒ jj, never git), uses `jj --no-pager diff`,
  and fails safe to *dirty* on indeterminate mode — exactly as specified.
- **Shared sha256 consolidation**: `scripts/hash-common.sh` is the single
  definition; `launcher-helpers.sh` sources it and keeps `sha256_of` as a thin
  one-line wrapper, preserving existing callers/tests.
- **Skill frontmatter** matches the planned `argument-hint`
  (`[--push-only|--pull-only] [--preview] [--all] [filter-flags…]`).

#### Deviations from Plan:

- **Linear `updatedAt` placement (benign).** The plan wording said to add
  `updatedAt` to the issue selection in `linear-graphql.sh`. The selection
  strings actually live inline in `linear-search-flow.sh:159` and
  `linear-show-flow.sh:87`, so the field was added there instead. Functionally
  identical and tested (the suites assert `updatedAt` is both *requested* in the
  captured query body and *returned* in the response). No `linear-graphql.sh`
  edit was needed.
- **No `plugin.json` edit (expected).** Phase 6 §3 named a `plugin.json`
  registration, but the plan's own Phase 6 success criteria already note skills
  are registered by directory (`./skills/work/` covers the new skill), so no
  edit was required — consistent with the corrected criterion.

#### Potential Issues:

- None blocking. The `mise run check` red is purely the documented pyrefly race;
  CI should be expected to occasionally show it and a sequential re-run clears
  it.

### Manual Testing Required:

These require a live tracker (`work.integration: jira` or `linear`) configured
and cannot be exercised by the mock-server unit suites alone:

1. End-to-end sync against a real tracker:
  - [ ] `/sync-work-items` with no `work.integration` prints a clear what/why/how error and exits
  - [ ] `/sync-work-items --preview` reports intended changes and writes nothing (local files + `last-sync.json` unchanged)
  - [ ] Default mode with no conflicts: remote-ahead pulled, local-ahead pushed, baseline updated
  - [ ] `--push-only` makes no local write; `--pull-only` makes no remote write; conflicts reported and skipped in both
  - [ ] Kill a run after one item, re-run — reconciled item is not re-pushed/re-pulled

2. Conflict UX (bidirectional):
  - [ ] A conflict shows a section-grouped LOCAL/REMOTE diff with remote as the recommended choice
  - [ ] Typing `local` pushes local→remote and prints the `OVERRIDE …` log line; `remote` overwrites local

3. `/list-work-items` five-state rendering:
  - [ ] Locally-edited tracked item shows `🔵 locally modified`; remote-edited shows `🟣 remotely modified`; both show `🔴 conflict`
  - [ ] Remote unreachable → still lists every item with synced/unsynced, exits without error or hang
  - [ ] No `last-sync.json` → only synced/unsynced labels

4. Push/pull offers:
  - [ ] Items with no `external_id` trigger the per-item push offer; batch accept-all writes each returned key to `external_id`
  - [ ] An untracked remote issue within `work.default_project_code` is created locally with the remote key as `external_id` and an independent local `id`
  - [ ] `--all` bypasses only the project scope; re-running does not re-create an already-pulled issue

### Recommendations:

- Treat the `mise run check` failure as the known pyrefly/node_modules race;
  CI verification of this work should run the components sequentially (or simply
  re-run) to get a clean signal.
- Run the manual test pass above against a real Jira and a real Linear tenant
  before release, since the divergent adapters (Jira chunked `key in (…)` vs
  Linear team-wide indexed search; ADF vs Markdown body projection) are only
  unit-covered via mock servers.
- No code changes required prior to merge.
