---
type: "plan-validation"
id: "2026-08-19-0213-conversational-conflict-resolution-flow-validation"
title: "Validation Report: Conversational Conflict Resolution Flow for Sync"
date: "2026-08-19T11:50:19+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "plan:2026-08-19-0213-conversational-conflict-resolution-flow"
target: "plan:2026-08-19-0213-conversational-conflict-resolution-flow"
tags: ["skills", "sync", "work-items", "conflicts", "cli"]
last_updated: "2026-08-19T11:50:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Conversational Conflict Resolution Flow for Sync

**Result: partial** — Phases 1–4 fully implemented and green end-to-end; Phase 5 (eval suite + committed evidence + 0171 reconcile) deliberately deferred to a follow-up, recorded in the plan itself and in commit `888a90b9` "Defer the 0213 eval suite to a follow-up". The `partial` verdict reflects only that intentional deferral, not any defect in the implemented scope.

### Implementation Status

- ✅ Phase 1: Freeze the report format — fully implemented (`nvyxkrmv`).
- ✅ Phase 2: Dossier extraction + rendering — fully implemented (`xlwkvypw`).
- ✅ Phase 3: Persist + preview + argv — fully implemented (`xqrwlnul`).
- ✅ Phase 4: Skill flow + static lint — fully implemented (`ovuwpwuz`, docstring tidy `tyvkoxmy`).
- ⚪ Phase 5: Evals + evidence — deferred by design; not implemented.

### Automated Verification Results

Every automated success criterion across Phases 1–4 was executed and passed.

| Check | Command | Status |
| --- | --- | --- |
| CLI format + clippy | `mise run cli:check` | ✅ exit 0 |
| CLI unit + integration tests | `mise run test:unit:cli` | ✅ exit 0 |
| Public API (no drift) | `mise run public-api:check` | ✅ exit 0 |
| Architecture (pup) | `mise run pup:check` | ✅ exit 0 |
| Conflict-flow lint | `mise run lint:sync-conflict-flow:check` | ✅ exit 0 |
| Build-system tasks | `mise run test:unit:tasks` | ✅ 2550 passed |
| Build-system format + lint | `mise run build-system:check` | ✅ exit 0 |
| Skill permissions | `mise run lint:skill-permissions:check` | ✅ exit 0 |
| Template checks | `mise run test:unit:templates` | ✅ 170 passed |

The 17 tests named in the plan all ran and passed (not skipped): `render_report_sorts_fixed_width_ids_numerically`, `render_report_emits_the_summary_row_for_an_empty_corpus`, `a_two_conflict_corpus_builds_a_dossier_per_item_with_bound_values`, `jira_builds_a_dossier_per_conflict_with_values_bound_to_each_side`, `a_prompt_item_with_an_unreadable_local_file_is_marked_local_unreadable`, `both_absent_stamp_variants_and_absent_mtime_render_as_unavailable`, `a_failing_renderer_downgrades_to_unrenderable_with_raw_values`, `a_local_unreadable_dossier_is_unrenderable_without_rendering`, `id_is_token_safe_admits_only_canonical_ids`, `prepare_conflicts_dir_writes_a_config_independent_ignore`, `persist_dossiers_writes_safe_ids_and_skips_unsafe_ones`, `the_stale_clear_removes_only_canonical_dossiers_and_artefacts`, and the four `sync_resolves_argv` cases.

### Code Review Findings

#### Matches Plan:

- **Report golden** (`cli/work-cli/tests/fixtures/sync-report.golden`) carries the ten crafted line shapes plus the `#\tsummary\tsynced\t1` row, matching the documented column vocabulary. The `0008` synced item is line-suppressed but counted (`synced 1`), and the `remote-absent`/`indeterminate` `noop` state lines are frozen — exactly the `SyncState` `Display` strings the skill's Phase 4 branch string-matches.
- **Dossier engine** (`cli/work-adapters/src/sync/run.rs`): `ConflictDossier` with the six fields + `local_unreadable`, `DossierRender::{Renderable,Unrenderable}`, an injectable `render_dossier`, and a `build_dossiers` pass reusing the `Pull` arm's gathering. Absent timestamps render `(unavailable)`; a failed local read yields `local_unreadable` (no fabricated local side) and the item stays `Action::Prompt`.
- **Persist path** (`cli/work-cli/src/sync.rs`): fail-closed `prepare_conflicts_dir` (verifies the `*` ignore before clearing), `id_is_token_safe` reusing the corpus canonical-id check, canonical-id-only stale clearing (a `notes.md` survives), atomic writes, IO failures surfaced to stderr. Runs on both preview and apply.
- **Gitignore**: repo-root `.accelerator/state/integrations/*/conflicts/` entry present with the pending-push-style comment; `git check-ignore` confirms IGNORED. Directory-local `*` ignore verified config-independent by test.
- **Preview doc** (`cli/work-cli/src/cli.rs:199-202`) narrowed to "no work-item write; gitignored conflict dossiers are still written".
- **Skill rewrite** (`skills/work/sync-work-items/SKILL.md`): dossier-driven flow, pinned typed `[remote/local/skip]` token with re-ask-once-then-skip normalisation, both `remote` and `local` framed as OVERWRITE, per-work-item choice with the multi-section scope line, discrete-argv `--resolve` emission, missing/unrenderable dossier handled fail-safe, dossier body treated as untrusted data, renderability read from the header region only. Full exit-code taxonomy partitioned (report-read set `0/4/70/71`; surfaced `1/2/5/72/73/74` + catch-all).
- **Static lint** (`tasks/lint/sync_conflict_flow.py`) fully registered: `tasks/lint/__init__.py`, `tasks/__init__.py`, `mise.toml` task block, wired into `lint:check`'s `depends`, and the `test_mise.py` placement assertion.
- **Stale config reference** removed: no `config-read-work.sh` mention remains; `SKILL.md:35` points at the `!`-preprocessor read.

#### Deviations from Plan:

- **`render_dossier`/`persist_dossiers` signatures thread a `scheme` argument** beyond the plan's illustrative literal, to reuse the corpus canonical-id check for both filename safety and the `--resolve` token. Sound and in the plan's spirit; the literals were explicitly illustrative.
- **Docstring reword** (`tyvkoxmy`, plus one further uncommitted edit) drops `0213`/`0212` work-item references from the lint module docstring, aligning with the repo's comment policy. Improvement, not a regression.

#### Potential Issues:

- ⚠️ **One uncommitted change is in the working tree** — `M tasks/lint/sync_conflict_flow.py` (the docstring reword). It is not yet committed; `jj` auto-snapshots it into the working-copy change, but it should be squashed into the Phase 4 commit or committed before the branch is finalised.
- **Phase 5 is unimplemented by design.** No `evals/` suite, no committed evidence, no evidence-hygiene/existence guard, and 0171's Decisions entry remains *pending (0213)*. `mise run` does not gate the eval flow, so this leaves the behavioural expectations (six fields, one prompt per conflict, one `--resolve` per choice) covered only by the static lint until the follow-up lands.

### Manual Testing Required:

The plan's automated ACs are all satisfied; the following manual checks from the plan remain unverified in this session (they require a live conflict and PATH manipulation):

1. Preview a real conflict:
  - [ ] `accelerator work sync --preview` writes `.../conflicts/<id>.md`, mutates no work item, file is gitignored.
  - [ ] A second preview with the conflict resolved leaves no stale dossier.
  - [ ] With `diff` removed from `PATH`, the dossier is written `status: unrenderable`, still lists section names + raw values, item stays `unresolved`.
2. Drive the skill:
  - [ ] `/sync-work-items` through a conflict — one prompt per item, typed-token shape, a single `--resolve` re-invocation.
  - [ ] A clean sync reports no conflicts and issues no `--resolve`.
3. Golden vocabulary spot-check (Phase 1 manual AC) — confirm the golden lines against the documented column vocabulary.

### Recommendations:

- Commit the uncommitted `tasks/lint/sync_conflict_flow.py` docstring edit before finalising the branch.
- Run the three manual checks above once against a seeded conflict — the unrenderable-with-`diff`-removed path especially, as it is the one degradation the static lint cannot exercise.
- Track Phase 5 as a named follow-up so the eval suite, its hygiene/existence guards, and the 0171 Decisions reconcile are not lost. The "evidence committed" AC clause stays vacuous until the existence check lands.
- Do not mark the plan `done` — Phase 5 remains outstanding. Plan status left at `ready`.
