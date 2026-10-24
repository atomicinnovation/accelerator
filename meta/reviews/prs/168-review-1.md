---
type: "pr-review"
id: "168-review-1"
title: "[0302] Add a re-key command to renumber work items onto a changed ID pattern"
date: "2026-10-18T08:41:23+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:168"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "usability", "documentation"]
review_number: 1
pr_number: 168
tags: []
last_updated: "2026-10-18T08:41:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #168 - [0302] Add a re-key command to renumber work items onto a changed ID pattern

**Verdict:** COMMENT

The re-key engine is carefully built. It has a pure, well-tested domain model in `work::rekey`, a run plan that is persisted before every write and resumes cleanly (with kill-at-every-write tests to prove it), whole-run refusals on collisions, and an `IdentityRunGuard` proof token. No lens found a critical defect. The majors fall into four groups. Two are correctness and safety gaps: relative links break when an item moves between `work/` and `drafts/`, and a resumed or forced apply can overwrite uncommitted edits without naming them. A further group is an error taxonomy whose codes and remedies sometimes point at the wrong next step. The last is a set of compatibility and documentation gaps around older binaries, schema bumps and `work.key`.

### Cross-Cutting Themes

- **Error codes and remedies that send the user the wrong way** (flagged by: usability, code-quality, compatibility, test-coverage) — `E_REKEY_IN_PROGRESS` covers both an interrupted run (finish or discard it) and a live run in another session (wait). `StateUnwritable` is the catch-all for lock, corpus and work-dir failures, but its remedy names `.accelerator/state/rekey/`. An I/O error reading the plan is reported as `Malformed`. `E_IDENTITY_RUN_BUSY` exits `1`, the internal-error code. Skills branch on these codes, so each conflation becomes wrong routing.
- **Repository resolution below the VCS root is copied, not shared** (flagged by: architecture, code-quality, test-coverage) — `facts_at`, canonicalise, `strip_prefix`, then `seen_from` is written out in both `migrate-adapters` and `work-adapters`, and migrate's copy has no adapter-level tests. This is the code where the PR's own bug lived.
- **`ACCELERATOR_FORCE` reach and visibility** (flagged by: safety, compatibility, usability, documentation) — one generic variable now overrides migrate's pre-flight, rekey's apply guard and rekey's discard revert check. A forced rekey never lists what it overrides. The deprecated `ACCELERATOR_MIGRATE_FORCE=0` still forces, while `--help` says it is the "same as `ACCELERATOR_FORCE=1`".
- **CHANGELOG gaps for user-visible behaviour** (flagged by: documentation, compatibility) — the CHANGELOG does not mention that `migrate` now refuses when `.accelerator/` sits below the VCS root, the whole-token link fix, exit 4 now possibly meaning "refused, wrote nothing" for existing commands, or `next-number` now writing lock state.
- **Repeated plumbing** (flagged by: architecture, code-quality) — repo-relative path encoding is defined twice (`RekeyCorpus::relative`, `FileRunPlanStore::relative`). The begin-identity-run-or-refuse block, its `Refused` variant and its `main.rs` arm are repeated across 6 commands, and each passes the command name as a string literal.

### Tradeoff Analysis

- **Safety vs usability (`ACCELERATOR_FORCE`)**: a single generic override is easy to learn and script, but a stale `export` now also reaches irreversible rekey paths. Recommendation: keep the variable for the dirty-tree check, require an explicit `--force` on `--discard` to set a plan aside, and list the overridden paths on every forced run.
- **Compatibility vs safety (run-plan schema)**: refusing a newer-schema plan protects resume correctness, but it also turns a leftover newer-schema *proposed* preview into a lock-out of older sessions. Recommendation: bump the schema only for changes that alter a field's meaning, and let the gate read `state` from a newer plan so `proposed` stays `Clear`.
- **Architecture vs delivery (lock hierarchy)**: threading `&IdentityRunGuard` into each use case works today. Making `acquire_retirement_lock` require the guard would let the compiler enforce the documented lock order. That is a modest follow-up, not a blocker.

### Strengths

- ✅ The pure re-key model (classify, mapping, sequence, collision, fingerprint, write set, run plan) is I/O-free and sits behind narrow ports (`RunPlanStore`, `IdentityRunPort`, `KeyRefresh`, `RekeyProgress`), so it is unit-testable without a filesystem or tracker.
- ✅ The run plan is saved in progress before the first write. `before_apply` records each path before it is written, and `WriteSet::extended_with` never overwrites a first-recorded state, so discard compares against true start states.
- ✅ `a_run_killed_at_every_write_converges_like_an_uninterrupted_run` stops the run at every write, resumes it, and checks the result matches an uninterrupted run, which tests the resumability claim directly.
- ✅ `IdentityRunGuard` has a private constructor pinned by a `compile_fail` doctest. Lock acquisition follows the documented order everywhere it was traced.
- ✅ `refuse_collisions` ignores case and covers `id`, aliases, `external_id` and in-run targets, so chained or swapped renames are refused rather than half-applied.
- ✅ Retirement is extended through `finish_retirement_with`'s typed `before_apply` hook, with the old API kept via `Infallible`, rather than forked.
- ✅ Path spellings are now rewritten as whole tokens only (`Case::{Exact, Ignored}`), which fixes `0041-first.md` matching inside `ACC-0041-first.md`.
- ✅ The `fetch_all` "found only under its current key" obligation is part of the port contract and is enforced by a reusable property run against the Jira and Linear offline mocks.
- ✅ Every refusal renders as `Error: <CODE>: <what>. <remedy>.` with root-relative paths. Every variant is pinned to a frozen exit code by an exhaustive test.
- ✅ The deprecation of `ACCELERATOR_MIGRATE_FORCE` is gentle and tracked (0346), and `ACCELERATOR_FORCE` deliberately rejects `0`/`false`.

### General Findings

- 🟡 **Documentation**: the CHANGELOG has no entry for two user-visible fixes: `migrate` now refusing when `.accelerator/` sits below the VCS root, and the whole-token path rewrite that stops links inside longer file names being corrupted. Add `### Fixed` entries.
- 🔵 **Compatibility**: `run_sync` takes the identity-run lock before any branch, so read-only `work sync --preview` now waits behind a re-key and refuses while one is in progress, unlike `create --dry-run`. This was not verified at the PR head.
- 🔵 **Architecture**: accelerator-specific guard policy (scopes, the `ACCELERATOR_FORCE` name and parsing) now lives in the generic `vcs` crate, with a hand-synced `migrate::preflight::SCOPES` copy. Consider moving it to a small, dependency-free home.
- 🔵 **Architecture**: identity-run acquisition is hand-rolled at every guarded CLI entry point, and the holder command is stringly typed (`timed_out` compares against `"rekey"`). Suggest an `IdentityCommand` enum and a `with_identity_run` helper.
- 🔵 **Documentation**: `skills/config/configure/SKILL.md` (around lines 709-710) still says items created before `{tracker}` keep their IDs, with no pointer to `/rekey-work-items`.
- 🔵 **Documentation**: the existing Unreleased CHANGELOG entry about jj resume still says to re-run "without `ACCELERATOR_MIGRATE_FORCE`".
- 🔵 **Test Coverage**: the rekey test harnesses (`Repo`, `ScriptedRepository`, counting and halting stores) are re-implemented in 4–5 files, and several tests halt at bare write indices.
- 🔵 **Safety**: on resume, the exemption for an untracked directory the VCS collapses (`meta/work/drafts/`) also hides unrelated untracked files in it.
- 🔵 **Usability**: preview prints `<old> → <new>` while apply prints `<old> → <new>\t<path>`, and both use a multi-byte arrow separator.
- 🔵 **Usability**: the `E_REKEY_UNCLASSIFIABLE_ID` remedy gives no example of an accepted shape.
- 🔵 **Code Quality**: `RekeyStatus` is an intermediate enum used only to derive `RunPlanGate`, and `ProposedPlan`/`InProgressPlan` duplicate their accessors.

### Additional Findings

- 🔵 `cli/work-adapters/src/identity_run.rs:110` — The identity-run concept depends on rekey's types and error vocabulary (architecture)
- 🔵 `cli/work-adapters/src/rekey/working_copy.rs:15` — `work-adapters` depends on `store`'s file-naming internals (architecture)
- 🔵 `cli/work-adapters/src/rekey/run.rs:292` — Tracker network reads run while the identity-run, retirement and create locks are held (architecture)
- 🔵 `cli/work-adapters/src/rekey/execute.rs:48` — Repo-relative path encoding has two definitions (architecture, code-quality)
- 🔵 `cli/work-adapters/src/rekey/run.rs:156` — `StateUnwritable` used as a catch-all with a misleading remedy (code-quality)
- 🔵 `cli/work-adapters/src/rekey/execute.rs:180` — `Debug` formatting of `FinishFailure` leaks into a refusal (code-quality)
- 🔵 `cli/work-cli/src/rekey.rs:114` — Tracker-selection failure travels through a `RefCell` side channel (code-quality)
- 🔵 `cli/work/src/rekey/outcome.rs:159` — Wildcard arms in `relative_to` and `listing` defeat exhaustiveness (code-quality)
- 🔵 `cli/work-cli/src/sync.rs:1390` — Begin-identity-run-or-refuse block repeated across 6 commands (code-quality)
- 🔵 `cli/work-adapters/src/rekey/run_plan_store.rs:382` — An I/O error reading the plan is reported as malformed (code-quality)
- 🔵 `cli/work/src/next_number.rs:115` — `allocate_above` returns a wider error than it can produce (code-quality)
- 🔵 `cli/work-adapters/src/rekey/refresh.rs:57` — Helper named `unreachable` reads like the panic macro (code-quality)
- 🔵 `cli/work-adapters/src/rekey/run.rs:211` — `--preview` skips retirement planning, so some refusals appear only at apply (correctness)
- 🔵 `cli/work-cli/src/rekey.rs:155` — One generic variable disables both the apply guard and the discard revert check (safety)
- 🔵 `cli/work-adapters/src/rekey/run.rs:440` — Discard after a VCS revert leaves lockdirs and atomic temporaries behind (safety)
- 🔵 `cli/work-cli/src/rekey.rs:234` — Corpus roots outside the config root are silently unguarded (safety)
- 🔵 `cli/work/src/rekey/run_plan.rs:11` — Schema bump rule is stricter than the decoder needs (compatibility)
- 🔵 `cli/work-cli/src/exit_codes.rs:16` — Exit 4 now means both "ran, items await a human" and "refused, did nothing" (compatibility)
- 🔵 `cli/work-cli/src/exit_codes.rs:149` — `E_IDENTITY_RUN_BUSY` shares exit 1 with internal errors (compatibility, usability)
- 🔵 `cli/migrate-adapters/src/working_copy.rs:40` — `migrate`'s new below-root refusal is missing from the CHANGELOG (compatibility)
- 🔵 `cli/vcs/src/guarded_working_copy.rs:14` — Generic `ACCELERATOR_FORCE` widens a migrate-era override (compatibility)
- 🔵 `cli/work-cli/src/next_number.rs:127` — `work next-number` is no longer read-only (compatibility)
- 🔵 `skills/work/rekey-work-items/SKILL.md:80` — The "re-key is running (pid n)" branch is unreachable from `work rekey` and has no eval (usability, test-coverage)
- 🔵 `cli/work-cli/src/identity_run.rs:48` — Guarded commands report a lock failure with no code or remedy (usability)
- 🔵 `cli/work/src/rekey/outcome.rs:479` — "Restore to its content when the run first touched it" gives nothing to restore from (usability)
- 🔵 `cli/work/src/rekey/outcome.rs:334` — `E_REKEY_PARTLY_APPLIED` remedy is one dense sentence (usability)
- 🔵 `cli/work-cli/src/rekey.rs:405` — A successful apply ends with no summary and no prompt to commit (usability)
- 🔵 `cli/work-cli/src/rekey.rs:87` — Headers and their listings are split across stdout and stderr (usability)
- 🔵 `cli/work-adapters/src/identity_run.rs:130` — Waiting line omits the 60 s ceiling and drops the pid when the command is unknown (usability)
- 🔵 `cli/migrate-cli/src/main.rs:95` — Old and new force variables interpret values differently (usability)
- 🔵 `cli/migrate-cli/src/cli.rs:16` — `--help` calls the deprecated variable "same as `ACCELERATOR_FORCE=1`" (documentation)
- 🔵 `cli/work-cli/src/cli.rs:157` — `work rekey --help` omits `ACCELERATOR_FORCE` for apply and `E_DRAFT_ID_EXHAUSTED` from exit 1 (documentation)
- 🔵 `docs-site/src/content/docs/guides/sync-work-items.mdx:186` — Sync guide lacks the upgrade-every-session warning (documentation)
- 🔵 `cli/corpus/src/lock.rs:43` — `clear_abandoned` trait doc is less precise than its adapter (documentation)
- 🔵 `cli/work/src/rekey/creation_date.rs:72` — Date-format boundaries that silently degrade to undated are untested (test-coverage)
- 🔵 `cli/work-adapters/src/rekey/execute.rs:149` — Two of the three pre-write refusal mappings are never exercised (test-coverage)
- 🔵 `cli/migrate-adapters/src/working_copy.rs:40` — migrate's new repository-discovery branches lack adapter tests (test-coverage)
- 🔵 `cli/work-adapters/tests/identity_run.rs:194` — Test named for a proposed plan only exercises the `Clear` gate (test-coverage)
- 🔵 `cli/work-adapters/tests/identity_run_lock.rs:145` — Wall-clock upper bounds may flake on a loaded runner (test-coverage)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/work/src/retirement.rs:403-429` — Relative links break in an item moved between work/ and drafts/
**Severity**: major | **Confidence**: high | **Lens**: correctness

🟡 **Correctness**

`path_spellings` spells every new path as a link *from the work directory* would (`drafts/<new>.md` or `<new>.md`). The same rewrite is then applied to the moved item's own content and to files inside `drafts/`. Retirement can now move an item across directories: into `drafts/` for every unsynced item under `{tracker}`, and out of `drafts/` when a draft is renumbered under a numeric pattern. After such a move, relative links stop resolving:

- A self-link `[self](0042-add-search.md)` becomes `drafts/draft-k3x9mq-add-search.md` inside a file that now lives in `drafts/`, so it resolves to `drafts/drafts/…`.
- Sibling and parent links in the moved item (`[parent](0041-parent.md)`, `[plan](../plans/p.md)`) are left untouched, so they now resolve one directory off.
- A draft that links a sibling draft by bare name gets the bare new name, which still resolves inside `drafts/` after the target has moved to `work/`.

`retired_content_matches_the_plan_for_a_draft_destination` only checks that the planner and the applier agree, not that the links resolve.

**Impact**: a `{tracker}` re-key breaks relative links in every unsynced item it moves into `drafts/`, a numeric re-key does the same for every draft it moves out, and the run still reports success.

**Suggestion**: spell each path relative to the directory of the file being rewritten, and re-base relative links in the moved item for its new directory. Add a test that resolves each rewritten link from the moved file's new location.

---

### `cli/work-adapters/src/rekey/working_copy.rs:48-70` — Repository resolution below the VCS root is duplicated rather than shared
**Severity**: major | **Confidence**: high | **Lens**: architecture, code-quality

🟡 **Architecture / Code Quality**

The PR describes the dirty-tree check as moved into `vcs::guarded_working_copy`, but only the scope filter (`observe_guarded`) moved. The part that fixes the bug where `.accelerator/` below the VCS root saw no dirty paths is copied almost line for line here and in `VcsWorkingCopy::probed` (`cli/migrate-adapters/src/working_copy.rs:33-54`). Both copies run `facts_at`, canonicalise with `unwrap_or_else`, then `strip_prefix` with the same "lies outside the repository at" error, then `working_copy_state(...).seen_from(within)`. The copies already differ in error type and in how they treat a missing repository, and migrate's copy has no adapter-level tests.

**Impact**: the next fix here (symlinked roots, jj workspaces, a canonicalisation failure) has to be made twice, and a third guarded command will add a third copy. Duplication of this kind is how the original gap arose.

**Suggestion**: move the resolution into `vcs`, e.g. `fn working_copy_seen_from(probe: &dyn RepositoryProbe, root: &Path) -> Result<Option<WorkingCopyState>, kernel::Error>`, with `None` meaning no repository. Each caller then maps only `None`/`Err` to its own probe-failure policy: migrate treats it as clean, rekey refuses.

---

### `cli/work-adapters/src/identity_run.rs:32-43` — The global lock order is not type-enforced
**Severity**: major | **Confidence**: high | **Lens**: architecture

🟡 **Architecture**

The PR says "the compiler stops a caller skipping the lock", but `acquire_retirement_lock(&dyn ExclusiveLock)` still produces a `RetirementLockGuard` without an `IdentityRunGuard`. `finish_retirement`/`apply_retirement` need only that guard, and `work::next_number::allocate` needs neither. Two of the new parameters are unused tokens (`_identity_run`), so whether the lock is held depends on where the tokens happen to be threaded, not on the lock hierarchy.

**Impact**: a future caller, or a refactor of `promote`, can retire or allocate IDs during a re-key without a compile error. That breaks the identity run → batch journal → retirement → create order documented in `cli/corpus/src/lock.rs`.

**Suggestion**: make `acquire_retirement_lock` (and the create-lock acquisition) take `&IdentityRunGuard`, so a `RetirementLockGuard` can only exist inside an identity run. The per-use-case `_identity_run` parameters then go away.

---

### `cli/work/src/rekey/outcome.rs:207-210` — `E_REKEY_IN_PROGRESS` conflates an interrupted run with a running one
**Severity**: major | **Confidence**: high | **Lens**: usability (with test-coverage)

🟡 **Usability**

`InProgress(None)` is an interrupted run, which needs finishing or discarding. `InProgress(Some(holder))` is a re-key still running in another session, which needs waiting for. Both carry one code, and skills are told to branch on codes. The new routing blocks in `sync-work-items`, `create-work-item`, `extract-work-items` and `refine-work-item` all say "run `/rekey-work-items` to finish or undo it". A user who hits this behind a live re-key is therefore sent to start a second, competing flow, overriding the CLI's correct "Re-run once it finishes" remedy. No eval covers the running case.

**Suggestion**: give the running case its own code (e.g. `E_REKEY_RUNNING`), or report it as `E_IDENTITY_RUN_BUSY` with a rekey holder. Route it in the guarded-command skills as "another session is re-keying; retry once it finishes", and add an eval asserting no Finish/Undo question is asked.

---

### `cli/work-adapters/src/identity_run.rs:225-226` — Identity-run lock I/O failure is reported as an unsaveable run plan
**Severity**: major | **Confidence**: high | **Lens**: usability (with code-quality)

🟡 **Usability**

A failure to take the identity-run lock (an I/O error under `.accelerator/state/locks/`) maps to `NotRekeyed::StateUnwritable`, here and in `From<IdentityRunRefusal>` for `Lock`. The user sees `E_REKEY_STATE_UNWRITABLE: the run plan cannot be saved or removed (…). Check permissions on .accelerator/state/rekey/`, which names the wrong file and the wrong directory. The same variant is also the catch-all for work-dir creation, corpus listing and corpus-walk failures (`run.rs:156-163`, `execute.rs:60`, `execute.rs:181`).

**Suggestion**: add distinct variants, e.g. `IdentityRunLockFailed` (remedy naming `.accelerator/state/locks/identity-run.lockdir`) and `CorpusUnreadable { path, detail }`. Keep `StateUnwritable` for run-plan writes only.

---

### `docs-site/src/content/docs/skills/work-items.md:70-74` — Docs promise a re-key after a `work.key` change that the engine does not perform
**Severity**: major | **Confidence**: high | **Lens**: documentation

🟡 **Documentation**

This page, and the `description` frontmatter of `skills/work/rekey-work-items/SKILL.md` (lines 4-6), say that after changing `work.id_pattern` *or* `work.key`, every existing item takes an ID in the new shape. The engine does not do this for a key change. `is_in_target_shape` renders an ID under its own project key (`project_key_of`), so an ID that is in target shape under any project key counts as on target. If `{key}-{number:04d}` stays and `work.key` goes from `ACC` to `PRJ`, every `ACC-0041` is already on target and the run reports nothing to re-key.

**Impact**: a user renaming their project key gets "every work item already matches `work.id_pattern`" and may decide the tool is broken. The skill description also steers model invocation towards that case.

**Suggestion**: narrow both texts to an `id_pattern` change, or to a `work.key` change affecting bare-number, tracker-keyed or draft items. Add one sentence saying that items already prefixed with a project key keep it.

---

### `cli/work-adapters/src/rekey/run.rs:354` — A forced rekey never lists the uncommitted paths it overrides
**Severity**: major | **Confidence**: high | **Lens**: safety

🟡 **Safety**

Under `Forcing::Forced`, `working_copy` returns `Ok(None)` without probing. A forced `--preview` therefore reports no dirty paths, and a forced apply prints only the generic `skipped_by_force` warning. The PR and CHANGELOG say a forced run lists what it skipped, and `migrate` does (`forced_past(&skipped)`); rekey does not. Under the override, `/rekey-work-items` falls into its no-warning branch and offers **Yes, re-key** with nothing to show.

**Impact**: with `ACCELERATOR_FORCE=1` left exported, a user can approve a bulk rename and rewrite over uncommitted edits without seeing which files are affected. A later VCS revert then destroys those edits.

**Suggestion**: when forced, still probe best-effort, return the dirty list with `base: None`, and print it on stderr for both preview and apply, as migrate's `PreflightOutcome::Forced { skipped }` does. Fall back to the generic warning only when the probe fails.

---

### `cli/work-adapters/src/rekey/run.rs:180` — A resumed run exempts its whole planned write set from the dirty-tree check
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

On resume, `working_copy(request.forcing, ports, plan.write_set())` exempts every write-set path. That set is computed up front for *all* entries, so referrers belonging only to entries the run never reached are exempt too. If the user edits such a file after the interruption (the gate stops accelerator commands, not editors) and then chooses "Finish it", the resume rewrites it over the uncommitted edit without refusing.

**Impact**: the stated guarantee is that an apply refuses over uncommitted changes because VCS revert is the undo. After a resume that guarantee no longer holds, and reverting the run silently loses the intervening edits.

**Suggestion**: exempt a dirty path only when its current digest equals its recorded start state, or when it belongs to an entry already retired. Add a test that halts the run, edits a referrer it has not touched yet, resumes, and expects `E_REKEY_DIRTY_WORKING_COPY`.

---

### `cli/work-adapters/src/rekey/run_plan_store.rs:417-424` — A newer-schema proposed plan would block older sessions' identity commands
**Severity**: major | **Confidence**: medium | **Lens**: compatibility

🟡 **Compatibility**

`gate()` maps any `Unreadable` plan, `NewerSchema` included, to a refusal. `run_plan.rs` says the schema is bumped "whenever the run plan gains a field", and the plan expects 0343 to bump it. Once that ships, a `--preview` from a newer binary leaves a proposed plan that makes every older session's `create`, `create-batch`, `next-number`, `sync`, `promote` and `update --push` exit 4 with `E_REKEY_PLAN_UNREADABLE`. Under the current schema, the same proposed plan blocks nothing (`Proposed => Clear`), and the decoder already ignores unknown fields.

**Suggestion**: bump `RUN_PLAN_SCHEMA` only when a field changes meaning, and say so in its doc. Alternatively, have the gate read the top-level `state` from a newer-schema plan and treat `"proposed"` as `Clear`, refusing only for in-progress plans.

---

### `CHANGELOG.md:302-304` — Older binaries are not prevented from acting during a re-key
**Severity**: major | **Confidence**: medium | **Lens**: compatibility

🟡 **Compatibility**

Older binaries know neither the identity-run lock nor the run plan. An older `work sync` can pull and recreate a retired file mid-run, and an older `next-number` can hand out a number the re-key is about to use. The only protection is this "upgrade every session" line. Claude Code sessions on different plugin versions coexist easily (a per-version plugin cache, long-lived sessions), so this is a realistic risk.

**Suggestion**: check which corpus-level locks the older `work sync` pull and `next-number` already take, and hold those for the whole run, as rekey already does for the create and retirement locks. Name any commands still unprotected here, and put the restart-other-sessions prompt in `/rekey-work-items`'s pre-apply confirmation, where it is acted on.

---

## Per-Lens Results

### Architecture

**Summary**: The PR keeps a clean split between a pure core and an I/O shell. The pure model is in `work::rekey`, the state machine in `work-adapters` behind narrow ports, and the wiring in `work-cli`, and retirement is extended through a `before_apply` hook. Weak spots: the lock is only partly enforced by the compiler, the identity-run concept borrows rekey's vocabulary, repository resolution below the root is duplicated, and `work-adapters` depends on `store`'s file-naming internals.

**Strengths**: Pure domain behind ports; open-closed retirement extension; `IdentityRunGuard` proof token; `CorpusWorkspace` split with `baseline: Option`; shared `prepare_ignored_dir`; `fetch_all` contract made explicit; versioned run plan with repo-relative paths.

**Comments**:
- `cli/work-adapters/src/rekey/working_copy.rs:48-70` (major, high) — Config-root-within-repository resolution duplicated rather than shared.
- `cli/work-adapters/src/identity_run.rs:32-43` (major, high) — Global lock order not type-enforced; retirement lock obtainable without `IdentityRunGuard`.
- `cli/work-adapters/src/identity_run.rs:110-126` (minor, medium) — `Holder` lives in `work::rekey::outcome`; every guarded command reports through `NotRekeyed`. Move to `work::identity_run`.
- `cli/work-adapters/src/rekey/working_copy.rs:15-23` (minor, medium) — `is_machinery` hard-codes `.lockdir` and calls `store` predicates; `store` became a production dependency. Expose one predicate from the owning adapter.
- `cli/work-adapters/src/rekey/run.rs:292-311` (minor, medium) — `{tracker}` refresh (no call ceiling) runs while the identity-run, retirement and create locks are held, so a slow tracker blocks all local allocation. Refresh before taking locks and validate with the fingerprint.
- `cli/work-adapters/src/rekey/execute.rs:48-58` (minor, medium) — Repo-relative encoding defined twice; introduce `RepoRelativePath`.
- `cli/work-adapters/src/rekey/run.rs:156-157` (suggestion, medium) — Direct `std::fs` I/O beside ports (`create_dir_all`, `read_dir` in `unsettled_recoveries`).

**General Findings**:
- (minor) Product guard policy in the generic `vcs` crate, with a hand-synced `migrate::preflight::SCOPES`.
- (minor) Identity-run acquisition hand-rolled per command; stringly typed holder command.

### Code Quality

**Summary**: The pure model is small, named in domain terms and mostly pure; `run.rs` reads clearly. Risks sit in the adapter/CLI layers: duplicated repository location and path rendering, repeated begin-or-refuse blocks, coarse `StateUnwritable` categorisation, and a `RefCell` side channel for tracker-selection failure.

**Strengths**: `Forcing`/`DirtyPaths`/`RekeyAction` enums rather than booleans; proof-token guard; pure, isolated modules; typed `before_apply` hook via `Infallible`; shared atomic `prepare_ignored_dir`; explicit `Case::{Exact, Ignored}`.

**Comments**:
- `cli/work-adapters/src/rekey/working_copy.rs:48-70` (major, high) — Sub-root repository locating and scoping duplicated.
- `cli/work-adapters/src/rekey/run.rs:156-163` (minor, high) — `StateUnwritable` catch-all with misleading remedy.
- `cli/work-adapters/src/rekey/execute.rs:180-182` (minor, high) — `format!("{failure:?}")` leaks `Debug` into a user-facing refusal.
- `cli/work-cli/src/rekey.rs:114-142` (minor, high) — `SelectionError` passed via `RefCell` with `TrackerUnreachable` stand-in; make it a first-class outcome.
- `cli/work/src/rekey/outcome.rs:159` (minor, medium) — `other => other` / `_ => Vec::new()` wildcards on a 21-variant enum.
- `cli/work-cli/src/sync.rs:1390-1398` (minor, high) — Begin-identity-run-or-refuse block repeated across 6 commands.
- `cli/work-adapters/src/rekey/run_plan_store.rs:116-126` (minor, high) — `relative` duplicated with `RekeyCorpus::relative`.
- `cli/work-adapters/src/rekey/run_plan_store.rs:382-384` (minor, medium) — Non-NotFound read errors become `Unreadability::Malformed`; suggested recovery cannot succeed.
- `cli/work/src/next_number.rs:115-130` (minor, medium) — `allocate_above` returns full `AllocationError`, forcing an `unreachable!` in `mapping.rs`.
- `cli/work-adapters/src/rekey/refresh.rs:57` (suggestion, medium) — Rename `unreachable` helper.

**General Findings**:
- (suggestion) `RekeyStatus` exists only to derive `RunPlanGate`.
- (suggestion) `ProposedPlan`/`InProgressPlan` duplicate accessors and doc comments.

### Test Coverage

**Summary**: Coverage is very strong and proportioned to risk. The domain has thorough unit tests, a kill-at-every-write convergence test, real git/jj working-copy tests, frozen exit-code pinning and 11 skill evals. Gaps are narrow: unreached defensive branches, untested date-format boundaries, unexercised skill branches and duplicated harnesses.

**Strengths**: Kill-at-every-write convergence; exhaustive outcome/code guard; guards tested at unit, use-case and CLI level; regression tests for every listed fix; race windows tested with seams rather than sleeps; `fetch_all` property run against mocks; specific assertions throughout.

**Comments**:
- `skills/work/rekey-work-items/SKILL.md:80-81` (minor, medium) — No evals for the running-pid branch, the empty preview, exits 70/72-74, or `--per-item-reads`.
- `cli/work/src/rekey/creation_date.rs:72-82` (minor, medium) — `.123Z`, `+0000` and space/lowercase separators silently become undated; pin the behaviour.
- `cli/work-adapters/src/rekey/execute.rs:149-166` (minor, medium) — `IdTaken`/`KeyLinked` → `Collision` and `ItemNotFound` → `PreviewStale` arms untested.
- `cli/migrate-adapters/src/working_copy.rs:40-57` (minor, medium) — No adapter tests for no-repository / outside-repository / nested-root branches.
- `cli/work-adapters/tests/identity_run.rs:194-207` (suggestion, high) — `a_non_rekey_command_begins_with_a_proposed_plan` only uses `RunPlanGate::Clear`.
- `cli/work-adapters/tests/identity_run_lock.rs:145-150` (minor, low) — Wall-clock upper bounds may flake.

**General Findings**:
- (minor) Rekey test harnesses re-implemented per file; bare halt indices.

### Correctness

**Summary**: Core logic is sound for the cases traced: allocation (overflow-guarded), collision refusal, the run-plan state machine, discard checks, lock ordering and the `fetch_all`/`locate` refresh. One real link-integrity bug for items moved between `work/` and `drafts/`, plus a preview/apply refusal gap.

**Strengths**: `saturating_add`/`checked_add` allocation; every write recorded before it happens; `extended_with` keeps first-recorded state; lock order followed everywhere; case-folded collision checks; whole-token rewriting with aligned offsets; indeterminate keys sent to `locate`; `seen_from` trailing-slash prefix.

**Comments**:
- `cli/work/src/retirement.rs:403-429` (major, high) — Relative links in an item moved between `work/` and `drafts/` break.
- `cli/work-adapters/src/rekey/run.rs:211-218` (minor, high) — `--preview` skips `plan_retirement`, so `E_REKEY_TARGET_EXISTS` and similar appear only at apply.

**General Findings**: none.

### Safety

**Summary**: The engine is conservative: the plan is persisted first, digests are recorded before each write, whole-run refusals apply, and unreadable plans and working copies that cannot be probed fail closed. Gaps are in the override and resume paths: the resume exemption is too broad, forced runs are silent about what they override, discard leaves lock debris, and corpus roots outside the config root are left unguarded.

**Strengths**: Plan saved before any write; digest-based discard and timestamped set-aside; broad collision refusal; whole-token rewrites; `clear_abandoned` reuses single-owner reclaim; strict `forces` parsing; nested-root gap closed; unprobeable working copy treated as dirty; skill never sets the override.

**Comments**:
- `cli/work-adapters/src/rekey/run.rs:180` (major, medium) — Resume exempts the whole planned write set from the dirty-tree check.
- `cli/work-adapters/src/rekey/run.rs:354` (major, high) — Forced rekey never lists the uncommitted paths it overrides.
- `cli/work-cli/src/rekey.rs:155-159` (minor, medium) — One generic variable disables both the apply guard and the discard revert check.
- `cli/work-adapters/src/rekey/run.rs:440-444` (minor, medium) — Discard after a VCS revert leaves `*.lockdir` and `.tmp-*` debris.
- `cli/work-cli/src/rekey.rs:234-245` (minor, low) — Corpus roots outside the config root silently dropped from guarded scopes.

**General Findings**:
- (suggestion) Untracked-directory exemption on resume can hide unrelated untracked files.

### Compatibility

**Summary**: Contract changes are mostly handled with care: `ACCELERATOR_MIGRATE_FORCE` keeps its old rule with a notice and tracked removal, the `fetch_all` obligation is documented and pinned, public-api fixtures are updated, and consuming skills learn the new codes. Weak spots: the schema-bump rule versus the gate, no protection against older binaries, exit 4 now covering two meanings, and the missing CHANGELOG note on the `migrate` refusal.

**Strengths**: Deprecation path preserved; strict new-variable parsing; contract obligation documented and enforced; all in-repo trackers already compliant; public-api fixtures updated; repo-relative run plan ignoring unknown fields; skills taught new codes; `create --dry-run` and local `update` remain unguarded.

**Comments**:
- `cli/work-adapters/src/rekey/run_plan_store.rs:417-424` (major, medium) — Newer-schema proposed plan blocks older sessions.
- `cli/work/src/rekey/run_plan.rs:11-13` (minor, medium) — Schema bump rule stricter than the decoder needs.
- `cli/work-cli/src/exit_codes.rs:16-18` (minor, high) — Exit 4 now also means a no-write refusal.
- `cli/work-cli/src/exit_codes.rs:149-157` (minor, medium) — `E_IDENTITY_RUN_BUSY` shares exit 1 with internal errors; `create` now serialises behind `sync` for up to 60 s.
- `cli/migrate-adapters/src/working_copy.rs:40-57` (minor, high) — New `migrate` refusal below the root not in the CHANGELOG.
- `cli/vcs/src/guarded_working_copy.rs:14` (minor, medium) — Generic `ACCELERATOR_FORCE` widens a migrate-era override.
- `cli/work-cli/src/next_number.rs:127-133` (minor, medium) — `next-number` now writes lock state and can create `.accelerator/`.
- `CHANGELOG.md:302-304` (major, medium) — Older binaries not prevented from acting during a re-key.

**General Findings**:
- (minor) The identity-run guard also applies to read-only `work sync --preview` (not verified at the PR head).

### Usability

**Summary**: The `work rekey` surface is carefully designed: one refusal shape, relative paths, actionable listings and forced-run warnings. Weak points: one code covers two opposite actions, a lock failure points at the wrong directory, and a transient busy state exits with the internal-error code, plus smaller gaps in success output and restore guidance.

**Strengths**: Consistent, test-enforced refusal shape; balanced `ACCELERATOR_FORCE` design; preview warns on dirty paths before confirmation; mutually exclusive flags and a `note:` for an ignored `--per-item-reads`; waiters name the holder within the skill shell timeout; gentle migrate deprecation.

**Comments**:
- `cli/work/src/rekey/outcome.rs:207-210` (major, high) — `E_REKEY_IN_PROGRESS` conflates interrupted and running.
- `skills/work/rekey-work-items/SKILL.md:80-81` (minor, medium) — Running-pid branch unreachable from `work rekey` (it exits `E_IDENTITY_RUN_BUSY`).
- `cli/work-adapters/src/identity_run.rs:225-226` (major, high) — Lock I/O failure reported as `StateUnwritable`.
- `cli/work-cli/src/identity_run.rs:48-51` (minor, high) — Guarded commands print a lock failure with no code or remedy.
- `cli/work/src/rekey/outcome.rs:195-199` (minor, medium) — Busy refusal exits 1.
- `cli/work/src/rekey/outcome.rs:479` (minor, medium) — The "restore to its content when the run first touched it" heading gives no source to restore from.
- `cli/work/src/rekey/outcome.rs:334-342` (minor, medium) — Dense `E_REKEY_PARTLY_APPLIED` remedy.
- `cli/work-cli/src/rekey.rs:405` (suggestion, medium) — No summary or commit prompt after a successful apply.
- `cli/work-cli/src/rekey.rs:87-92` (suggestion, medium) — Headers and listings on different streams.
- `cli/work-adapters/src/identity_run.rs:130-142` (suggestion, medium) — Waiting line omits the ceiling and drops the pid.
- `cli/migrate-cli/src/main.rs:95-100` (minor, medium) — Old and new force variables interpret values differently; unrecognised values ignored silently.
- `skills/work/create-work-item/SKILL.md:432-433` (minor, medium) — Routing assumes the run is always interrupted.

**General Findings**:
- (suggestion) Preview and apply print different line shapes with a multi-byte separator.
- (suggestion) Unclassifiable-ID remedy lacks examples.

### Documentation

**Summary**: User-facing docs are largely accurate; the skill's branches match the strings the CLI actually prints, and the exit-code docs agree across 4 sources. Main issues: the `work.key` claim is wrong, the CHANGELOG omits two behaviour fixes, and there are smaller gaps in `--help` and the guides.

**Strengths**: Skill branch conditions tied to real CLI strings; exit-code docs consistent; evals quote real messages; lock-order doc matches code; CHANGELOG states `ACCELERATOR_FORCE` value rules; docs-site cross-links correct.

**Comments**:
- `docs-site/src/content/docs/skills/work-items.md:70-74` (major, high) — `work.key` change alone does not re-key.
- `skills/work/rekey-work-items/SKILL.md:4-6` (major, high) — Skill description makes the same promise.
- `cli/migrate-cli/src/cli.rs:16` (minor, high) — `--help` says "same as `ACCELERATOR_FORCE=1`" though the value rules differ.
- `cli/work-cli/src/cli.rs:157-172` (minor, high) — `work rekey --help` omits `ACCELERATOR_FORCE` for apply and `E_DRAFT_ID_EXHAUSTED`.
- `docs-site/src/content/docs/guides/sync-work-items.mdx:186-190` (minor, medium) — Missing the upgrade-every-session warning.
- `cli/corpus/src/lock.rs:43-45` (suggestion, medium) — `clear_abandoned` trait doc less precise than the adapter.

**General Findings**:
- (major) CHANGELOG omits the `migrate` below-root refusal and the whole-token rewrite fix.
- (minor) `configure` SKILL.md still says pre-`{tracker}` items keep their IDs.
- (minor) Existing Unreleased CHANGELOG entry still names `ACCELERATOR_MIGRATE_FORCE`.
