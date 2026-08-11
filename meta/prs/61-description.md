---
type: pr-description
id: "61"
title: "[0185] Converge corpus-adapters on the Library-Backed VCS Adapter"
date: "2026-08-11T09:46:24+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0185"
parent: "work-item:0185"
relates_to: ["work-item:0188", "work-item:0198", "work-item:0203"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/61"
pr_number: 61
tags: [rust, vcs, cleanup, tech-debt]
revision: "47987ff4ddf94a1cda3617c0ad163709d0277570"
repository: "accelerator"
last_updated: "2026-08-11T09:46:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# [0185] Converge corpus-adapters on the Library-Backed VCS Adapter

## Summary

`vcs_adapters::facts` resolved repository facts by spawning `jj log -r @ -T commit_id` and `git rev-parse HEAD`. 0188 delivered a fully-implemented in-process alternative — `InProcessProbe`, backed by `gix` and `jj-lib` — but deliberately left it unwired, so the crate carried two complete implementations of the same ports. This PR repoints the composition root onto the library-backed probe, deletes the subprocess pair, extends the crate's zero-spawn guarantee to cover the corpus metadata-read path, and records the three policy decisions the switch depends on in the code itself. It also corrects a licensing finding that turned out to be wrong for five of the six shipped sub-binaries.

## Changes

- **Composition root repointed**: `vcs_adapters::facts` now composes `InProcessProbe` for both the `RepoRoot` and `VcsProbe` positions. No `Command::new` for `jj` or `git` remains anywhere on the `facts` path.
- **Subprocess probes deleted**: `CommandProbe`, `MarkerWalkRoot`, their private `jj_repository_root` helper, and the `OriginRemote`-only `run_checked`/`wait_capped_checked` pair are gone, along with the transitional dual-adapter comparisons in `tests/detection.rs` (`assert_implementations_agree`, `facts_via`) and `tests/library.rs` (`assert_parity`). `scrub_environment`/`run_capped` survive untouched — 0198's `status`/`log` subprocess path shares them, and `subprocess.rs` is now scoped to that alone.
- **Zero-spawn proof extended across the composition**: a new `corpus-adapters-fixture` reference binary drives the real `VcsBackedRepoFactsProbe` → `vcs_adapters::facts` → `InProcessProbe` chain as a black box, and `tests/zero_spawn.rs` proves it resolves both a plain-git and a pure-jj repository with `git`/`jj` stubbed off `PATH`. Previously only the individual taxonomy queries were proven; the metadata-read path the corpus actually uses was unproven. The fixture lives under `tests/fixtures/` rather than `src/bin/`, mirroring `vcs-adapters-fixture`, so it stays off the crate's normal build surface and out of release staging.
- **Three policy decisions recorded in code, not left open**: (1) a sha256-format repository is unsupported — `gix` 0.85 cannot read one at all, so `revision` folds to `None` through its existing fallible-to-`Option` contract rather than misreading; (2) no timeout, memory cap, or crash-isolation bound is added around `InProcessProbe`, with the decision naming what it *removes* (the old 10-second cap with kill-on-timeout) and the sharpest blast radius it exposes (`work create` holds a creation lock for the duration of the call, and its reclaim mechanism only reclaims a dead holder); (3) no `corpus-adapters` write path depends on jj's snapshot-on-read side effect, scoped explicitly so it does not imply the same for the authoring skills that copy `Current Revision:` into committed frontmatter.
- **New tests backing those decisions rather than asserting them**: a sha256 `revision()` case (the existing sha256 tests covered `is_bare`/`worktree`/`dual_roots`/`classify` but never `revision`), and a malformed-git-`HEAD` case giving the git dispatch path the graceful-failure proof the jj path already had.
- **Git-side revision oracle added**: deleting `assert_parity` removed the only place that cross-checked `InProcessProbe`'s git revision against the real `git` binary, leaving a 40-hex format check as the sole guard. A `git_revision_oracle` helper mirroring the existing `jj_revision_oracle` closes that, so the simplification stays symmetric.
- **MPL-2.0 finding corrected**: `cli/deny.toml`'s `uluru` exception was recorded on the premise that dead-code elimination stripped the whole `gix`/`jj-lib` closure from every shipped binary. Measured across all six `DISPATCHED_SUBBINARIES` on unstripped `--release` builds, the premise holds only for the visualiser. `accelerator-vcs`, `accelerator-work`, `accelerator-collaboration` and `accelerator-migrate` already linked it *before* this change (each constructs `InProcessProbe` directly through call sites unrelated to `facts`); `accelerator-corpus` is the one this switch causes. Follow-up `work-item:0203` filed for the attribution artefact the release upload set now owes.
- **Verification method corrected too**: the two string literals the original procedure relied on (`extensions.objectFormat`, `There is no Jujutsu repo`) are unsound as an absence test — both are missing from binaries that demonstrably link the closure. The recorded finding is now a `gix_`/`jj_lib`/`uluru` symbol count via `nm -a`, with the literals' unreliability recorded alongside it so the next person does not repeat the mistake.

## Context

Implements `work-item:0185` (Converge corpus-adapters on the Library-Backed VCS Adapter), researched in `meta/research/codebase/2026-08-10-0185-converge-corpus-adapters-library-backed-vcs.md`, planned in `meta/plans/2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs.md`, reviewed at both the work-item and plan level (`meta/reviews/work/0185-...-review-1.md` and `meta/reviews/plans/2026-08-10-0185-...-review-1.md`, both APPROVE), and validated in `meta/validations/2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs-validation.md` (result: pass).

Completes the convergence 0169 deliberately deferred and `work-item:0188` set up: 0188 built and proved `InProcessProbe` without wiring it, so that the composition switch could be reviewed on its own merits. Bounded against `work-item:0198`, which owns the separate `status`/`log` subprocess path and is the reason `scrub_environment`/`run_capped` survive this deletion.

Surfaces `work-item:0203` as its own follow-up: MPL-2.0 §3.2's notice obligation is live today for five shipped binaries, four of them independently of this change.

## Testing

- [x] `mise run check` — the read-only CI-equivalent gate across frontend, server, cli, build-system and scripts — exits 0.
- [x] `mise run test:unit:cli` — 1486/1486 pass across the whole `cli/` workspace, including the reworked `detection.rs`/`library.rs` and both new decision-backing tests.
- [x] `mise run test:integration:zero-spawn` — passes. The metadata-read test is verified non-vacuous: both shapes resolve real facts (`name=PG revision=1cbe1df0…`, `name=c revision=32922826…`) rather than matching as "absent" twice.
- [x] The strong-form zero-spawn lane was simulated locally — the compiled `zero_spawn` binary run under `env -i` with a `PATH` carrying no `git`/`jj` and the fixture matrix handed over via `ACCELERATOR_ZERO_SPAWN_MATRIX`. This caught a real break in the first cut of the new tests (they built fixtures inside the test body, which the shadow window cannot support) and verified the fix against a proven red.
- [x] No `CommandProbe` or `MarkerWalkRoot` reference remains anywhere under `cli/`; the only surviving `std::process::Command` uses in `vcs-adapters` non-test code serve 0198's `status`/`log` path.
- [x] The MPL-2.0 finding reproduces independently by symbol count on the existing release artefacts, matching what `cli/deny.toml` now records.
- [ ] Not verified this session: the Linux CI lane, and the real `test:integration:zero-spawn:strong` job — it `sudo mv`s system `git`/`jj` and belongs on an ephemeral runner, so it was covered by the simulation above rather than run locally.

## Notes for Reviewers

- **The one user-visible behavioural change** is the loss of jj's snapshot-on-read side effect on `corpus metadata derive`. For `corpus-adapters`' own Rust write paths this is stdout-only (`work create` reads the derived timestamp, never the revision). It is *not* stdout-only for the authoring skills — `create-plan`, `research-issue`, `create-note` and others copy the printed `Current Revision:` into committed `meta/` frontmatter, so an artifact authored with unsnapshotted edits present now records the last recorded commit. Accepted as a best-effort provenance degradation rather than a correctness regression, and documented as such on `VcsBackedRepoFactsProbe`.
- **The containment decision deserves a second opinion.** It removes a 10-second cap with kill-on-timeout from the `facts()` call site and adds nothing in its place, on the grounds that the same unbounded exposure already runs on the hook path and no crash-isolation precedent exists in the workspace. The doc comment names the `work create` lock blast radius explicitly and states a structural revisit condition. If you disagree with the trade, this is the place to say so.
- **`work-item:0203` is a licensing obligation, not a nice-to-have.** It is filed but `ready`, not done. Four of the five affected binaries were already shipping the MPL-2.0 closure before this PR, so the obligation predates it — flagging so the finding is not read as introduced here, nor assumed closed by it.
- **One narrow behaviour is documented but untested**: `InProcessProbe::repository_root` canonicalises where `MarkerWalkRoot` did not, so if the repository directory itself is a symlink the canonicalised final path component can change the persisted `Repository Name:`. Judged too narrow to warrant new test infrastructure; say if you would rather it were pinned.
- **One commit exceeds the plan's stated scope**: "Strip comments that restate the code or name deleted implementations" also removes stale bash-port references from `corpus-adapters`' `work_item_pattern.rs` and `tests/parity.rs`, files the plan never named. Consistent with the repo's comment policy and with prior precedent, but called out rather than folded in silently.
- **History reviews commit by commit**: policy decisions → composition switch → deletion → licence finding → comment cleanup → zero-spawn lane fix → validation. The deletion commit is large but purely subtractive.
