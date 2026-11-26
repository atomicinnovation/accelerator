---
type: "pr-review"
id: "270-review-1"
title: "[PP-943] Build the slowest CLI unit tests' repository states once and copy them"
date: "2026-11-26T10:42:17+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:270"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "performance", "standards", "portability", "documentation"]
review_number: 1
pr_number: 270
tags: []
last_updated: "2026-11-26T10:42:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #270 - [PP-943] Build the slowest CLI unit tests' repository states once and copy them

**Verdict:** COMMENT

The PR goes after the costs that dominated: about 182 of roughly 248 jj spawns in `guarded_resume`, per-pass repository rebuilds in `rekey_run`, and `F_FULLFSYNC` serialisation. No scenario or assertion is lost, and each reused state is checked against a fresh build. The structure is mostly sound. Reading and projecting repository state are split cleanly, the third family is registered on every surface the existing two use, and the Rust/Python contract files stop the two halves drifting. Nothing blocks merging. The main concerns are blind spots in the `RepositoryState` projection that the copy-equals-scratch checks rely on, migrate-specific knowledge settling in `vcs-test-support`, a test store that duplicates production write logic, and figures measured on macOS only.

### Cross-Cutting Themes

- **`RepositoryState` cannot see every difference it should** (flagged by: correctness, test-coverage, documentation) — The projection is now the oracle for "a copy equals a fresh build", but some genuinely different states project equal. It never records which jj commit is `@`, only `@`'s parents. It stores `HEAD` as a commit ID, so an attached and a detached `HEAD` at the same commit look alike. It drops author, committer and the jj operation log, and decodes contents with `from_utf8_lossy`. Its git reader's annotated-tag peeling, conflict-stage keys and gitlink skipping have no test. Record `@` and `HEAD`'s symbolic target, and list in the module doc what the projection deliberately leaves out.
- **Migrate's knowledge now lives in `vcs-test-support`** (flagged by: architecture, standards) — The VCS fixture crate now holds the ordered `MIGRATIONS_BEFORE_0007` list, the ledger path, `migrate_command` and its scrubbed variables, and a `REPRODUCTION` string naming migrate's test. The ledger is now split across two crates, and a module named `corpus` shadows the workspace's `corpus` crate inside `guarded_resume.rs`. Keep all migrate knowledge behind the `stalled_migration` boundary under a name that does not clash, and keep the ledger in one place.
- **`UnsyncedStore` duplicates the store crate's write protocol** (flagged by: architecture, code-quality, test-coverage) — `create_new` re-implements `store::atomic_create` without the fsyncs, `TEMP_RANDOM_LEN` copies a private constant, and `refusal` copies `corpus_adapters::store::to_store_error`. The eight contract tests help but skip a symlink as the final path component. A `store::create_without_sync` beside `replace_without_sync` would keep durability a policy of one crate.
- **`each_of` guards that cannot fail** (flagged by: code-quality, test-coverage) — In `rekey_run`, `expected` and `items` both come from `mutations_of_a_whole_run*()`. The guard therefore catches only zero iterations, and the `checked + skipped == halts` assertion is a tautology. If production mutations dropped from 14 to 3, both tail tests would check far fewer halt points and still pass. Fix the counts to 13 and 14 once.
- **A family with inputs is a new extension point with no how-to** (flagged by: architecture, code-quality, documentation, standards) — `SharedFamily::inputs_from_environment` is required of every family but called only from the concrete `StalledMigration` arm. Adding the third family took edits in about eight places across two languages, and the README has no checklist for it. The README's guard bullet also omits the newly guarded `copy_private_tree`.
- **Speed-ups measured only on macOS** (flagged by: performance, portability) — Both reasons for the speed-ups, `fclonefileat` and avoiding `F_FULLFSYNC`, are macOS-specific, and both records come from one Mac. The ubuntu CI leg copies entry by entry, so the gain there is unmeasured. State the platform in the figures, and sample the Linux CI leg's nextest output for `guarded_resume` and `rekey_run`.

### Tradeoff Analysis

- **Fidelity vs cost of the equivalence checks**: The checks that copies match fresh builds cost about 19% of `guarded_resume`'s test-seconds and 7.7 s in the rekey kill test. Correctness and test coverage want the projection to see more. Performance points out that `assert_copy_matches_a_fresh_build` builds each template twice. Keep the checks and strengthen the projection; its extra reads are cheap at 23–49 ms per projection. Comparing a rekey copy against the very template it came from halves that check without losing what it guards.
- **Fail-fast build step vs blast radius**: A stall that does not stall aborts the build step before any test runs. `tasks/README.md` documents this, and only runs that select `guarded_resume` build the family. Architecture prefers a non-fatal family failure with a private-build fallback, so a TDD red step on migrate shows as targeted test failures rather than one build error. Fail-fast is simpler and louder. If it stays, add a sentence on why it was chosen.

### Strengths

- ✅ No scenario lost: all 13 `guarded_resume` tests keep their assertions, and the stall checks that `Stalled::on` used to make moved into `Stall::build`, which names the key, migrate's exit code and stderr.
- ✅ The edit-refusal test now halts at all 14 mutations, and `a_whole_run_makes_exactly_the_counted_mutations` cross-checks the scripted count against a real run.
- ✅ `RepositoryState` keeps I/O in `reading.rs`, which spawns a fixed 2–4 processes through one `git cat-file --batch` session. `order.rs` and `masking.rs` are pure and unit-tested in either input order. `ordered_positions` refuses to order commits that nothing tells apart rather than picking one arbitrarily.
- ✅ The `fclonefileat` path falls back only on `XDEV`/`NOTSUP` and propagates every other errno. Restoration runs top-down, so restored directory times survive their children's changes. Both copy strategies share one contract test suite that runs on both CI platforms.
- ✅ The third family is registered on every surface: the Rust traits, Python `Family`/`_Traits`, `CONSUMERS`, the contract JSON, the fixture-build guard, the README tables and `mise.toml`. The new `executables` key ties Rust's `MIGRATE_VARIABLE` to Python's.
- ✅ `UnsyncedStore` is held to `FileCorpusStore` by running the same eight contract cases against both stores.
- ✅ `measure:test-durations` keeps parsing in the pure `test_durations.py`, anchors its tests to three real baseline logs, and comes with a regression test for the `ERROR task failed` naming fix.
- ✅ The figures are honest: medians of 3, checks charged to the test they verify, and misses reported against their thresholds.

### General Findings

- 🔵 **Documentation / Standards**: The `tasks/README.md` "Shared VCS fixtures" bullet (around lines 417–422) still lists only `Workbench::with`, `copy_part` and private-set builders as guarded. Add `copy_private_tree` and how to extend its allow-list.
- 🔵 **Architecture / Documentation**: Adding a shared family touches about eight places across Rust, Python and the JSON contract. A family with inputs also needs `Inputs`, `Family.executables` and `writable_with`. Add a "Registering a shared fixture family" checklist beside the sub-binary and library-crate checklists.
- 🔵 **Performance / Portability**: The before and after figures are macOS-only, and the Linux CI leg takes the recursive copy for every stall and template. Note the platform, and sample the ubuntu leg.
- 🔵 **Performance**: The kill test and `matrix_private_build` finish last partly because they start late. nextest 0.9.138 supports per-test `priority` overrides, and `cli/.config/nextest.toml` sets none.
- 🔵 **Performance**: The copy-equals-scratch checks are a lasting share of the suite's test-seconds; treat their cost as part of the budget.

### Additional Findings

- 🔵 `cli/vcs-test-support/src/stalled_migration.rs:136` — Shared build step now runs production migrate; a stall that does not stall aborts the whole step (architecture)
- 🔵 `cli/migrate-cli/tests/guarded_resume.rs:21` — `stalled_migration::corpus` shadows the workspace `corpus` crate (standards)
- 🔵 `cli/migrate-cli/tests/common/mod.rs:25` — Migration ledger constants split across two crates (standards)
- 🔵 `cli/vcs-test-support/src/stalled_migration.rs:72` — Colocation's display name defined on `StallKey`, not `JjColocation` (standards, code-quality)
- 🔵 `cli/vcs-test-support/src/copies.rs:171` — `restore_what_a_clone_drops` also adds owner-write; its name and doc do not say so (code-quality, documentation)
- 🔵 `cli/migrate-cli/tests/guarded_resume.rs:139` — Stale doc comment on the restructured `Stalled` (documentation)
- 🔵 `tasks/README.md:325` — Two rewrapped prose lines exceed 80 columns (standards)
- 🔵 `tasks/shared/build_measurement/test_durations.py:225` — Validation by calling a query and discarding its result (code-quality)
- 🔵 `cli/vcs-test-support/src/repository_state/order.rs:19` — Tie-breaks compare unmasked file contents, so build-specific IDs can decide order (correctness)
- 🔵 `cli/vcs-test-support/src/repository_state.rs:166` — Projection drops author, committer and the jj operation log (test-coverage)
- 🔵 `cli/vcs-test-support/src/stalled_migration.rs:183` — The shared build's success path has no test of its own (test-coverage)
- 🔵 `cli/vcs-test-support/src/stalled_migration_manifest.rs:28` — Manifest stores absolute stall roots, so the shared tree cannot be moved (portability)
- 🔵 `cli/vcs-test-support/src/stalled_migration.rs:133` — A missing migrate binary fails without naming the path or its variable (portability)
- 🔵 `cli/vcs-test-support/src/copies.rs:207` — Each directory-copy strategy runs on only one platform (portability)
- 🔵 `cli/work-adapters/tests/rekey_run.rs:1011` — Copy check builds every template twice (performance)
- 🔵 `cli/work-adapters/tests/rekey_run.rs:983` — New exact-count test rebuilds repositories instead of copying a template (performance)
- 🔵 `cli/work-adapters/tests/rekey_run.rs:972` — Two parallel ways to build a committed repository (code-quality)
- 🔵 `tasks/shared/build_measurement/test_durations.py:27` — Family build times parsed from a console format defined in `session.py` (architecture)
- 🔵 `tasks/README.md:688` — `measure:test-durations` paragraph omits the family-build output and option rules (documentation)
- 🔵 `cli/vcs-test-support/src/copies.rs:98` — `copy_private_tree` rustdoc refers to "a guard" without naming it (documentation)
- 🔵 `cli/vcs-test-support/src/copies.rs:174` — A cloned copy is walked twice (performance)
- 🔵 `cli/vcs-test-support/src/repository_state/reading.rs:443` — `TreeEntries` yields `Option<Option<_>>` via an immediately invoked closure (code-quality)
- 🔵 `tasks/shared/build_measurement/test_durations.py:22` — Log-line classification duplicated from `timeline.py` (code-quality)
- 🔵 `tasks/measure.py:3866` — Log discovery helpers added to an already 3,800-line task module (code-quality)
- 🔵 `cli/migrate-cli/tests/common/mod.rs:56` — Doc comment on `run` now restates its body (documentation)
- 🔵 `cli/vcs-test-support/src/repository_state.rs:174` — `CommitPosition`'s meaning documented only on a private function (documentation)
- 🔵 `cli/vcs-test-support/Cargo.toml:27` — `rustix` declared for every platform but used only on macOS (portability)
- 🔵 `tasks/shared/build_measurement/test_durations.py:23` — Duration parsing depends on nextest's human-readable lines (portability)
- 🔵 `cli/work-adapters/tests/rekey_run.rs:1275` — `UnsyncedStore` contract omits a symlinked final path component (test-coverage)
- 🔵 `tasks/shared/shared_fixtures/session.py:43` — One- and two-family "building …" messages lost their test (test-coverage)
- 🔵 `cli/vcs-test-support/src/repository_state.rs:258` — Lossy UTF-8 decoding can make different binary contents compare equal (correctness)

## Inline Comments

### `cli/migrate-cli/tests/guarded_resume.rs:167-169` — Every access to the stall repeats a fallible map lookup
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`Stalled` stores a `StallKey` and looks the `Stall` up again on every access. `self.stall()?` appears about 40 times in this file, often several times in one statement (`stalled.stall()?.root.join(...)` next to `fs::read_to_string(stalled.stall()?.root.join(...))`). Every test body now carries `?`s for a lookup that cannot fail once `copied` has succeeded.

**Impact**: The scenarios are much harder to read than before, and a reader has to ask why a field access could fail.

**Suggestion**: Resolve the stall once in `Stalled::copied`. Clone the `Stall` into the struct (it is `Clone`: root, base, sibling) and keep `Held` only to own the copy, or add infallible `root()`, `base()` and `sibling()` accessors. Call sites then become `stalled.root()`.

---

### `cli/work-adapters/tests/support/unsynced_store.rs:62-90` — UnsyncedStore re-implements the store crate's atomic create and error mapping
**Severity**: major | **Confidence**: high | **Lens**: code-quality, architecture

🟡 **Code Quality / Architecture**

`UnsyncedStore::create_new` copies `store::atomic_create` by hand: containment check, `kernel::TEMP_PREFIX` staging, its own `TEMP_RANDOM_LEN = 6` (a copy of the store crate's private constant), mode, then a no-clobber persist. `refusal` is a near-copy of `corpus_adapters::store::to_store_error`, and `failure` maps io errors a third way. The `store` crate already treats durability as a policy for replacement (`replace_without_sync` beside `atomic_write`) but not for exclusive creation, so the test crate filled the gap by hand.

**Impact**: About 130 lines of test code shadow production write semantics. A change to staging, containment or error mapping that the eight contract tests do not cover will let this double drift unnoticed. The contract also skips one case: a target that is itself a symlink pointing outside the root.

**Suggestion**: Add `store::create_without_sync` beside `replace_without_sync`, and expose the `WriteError`→`StoreError` mapping from `corpus-adapters`. `UnsyncedStore` then becomes a thin composition, like its `remove` delegation.

---

### `cli/work-adapters/tests/rekey_run.rs:1072-1082` — Kill test, now the phase's last finisher, runs both VCSs one after another in one test
**Severity**: major | **Confidence**: medium | **Lens**: performance

🟡 **Performance**

`a_run_killed_at_every_write_resumes_over_its_own_writes` builds the git template, then the jj template (init, commit and repack each), then works through all 2 × 13 halt passes serially in one test. The after record shows it as the last finisher in every run (21.3 s charged; 13.7 s of its own under the suite), so its wall-clock time sets the end of the nextest phase.

**Impact**: nextest cannot spread this work across cores, so one serial loop sets the phase end the PR is trying to shorten. Each VCS's half is independent, with its own template and its own copies.

**Suggestion**: Split it into one test per VCS (`..._in_git`, `..._in_jj`) calling a shared helper that takes the `Vcs`. Each test builds only its own template, so the halves run in parallel and the tail roughly halves. `each_of` can still prove each test's loop count.

---

### `cli/vcs-test-support/src/repository_state/reading.rs:201-203` — The jj projection never records which commit is the working copy (@)
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

The jj listing reads `current_working_copy`, but only uses it to copy `@`'s parents into `working_copy_parents`. `@` itself is never marked in `RepositoryState`, and `TieBreak` only has `is_working_copy_parent`. Two states that differ only in which of two sibling commits is `@` project equal when both siblings have the same parents and tree. Example: `jj new P -m a; jj new P -m b` (`@` = b), versus the same history followed by `jj edit a`.

**Impact**: The projection exists to prove a copied state equals a fresh build. A copy whose working copy is editing a different change still compares equal, though that decides where later edits land (as in `guarded_resume`'s `jj edit` moves).

**Suggestion**: Record `@` explicitly, e.g. `working_copy: Option<CommitPosition>` on `RawRepository` and `RepositoryState` plus an `is_working_copy` tie-break, and add a "differs when `@` differs" case.

---

### `cli/vcs-test-support/src/repository_state/reading.rs:115-118` — An attached HEAD and a detached HEAD at the same commit project equal
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

`git_side` resolves `HEAD` through `cat-file` to a commit ID and stores it like any other reference. Whether `HEAD` is a symbolic ref to `refs/heads/main` or detached at the same commit is lost: both yield `{HEAD: C, refs/heads/main: C}`.

**Impact**: The two states behave differently, since the next commit advances the branch in one and not the other. The equality checks, such as the `rekey_run` template-copy tests, cannot tell them apart.

**Suggestion**: Also record `HEAD`'s symbolic target as a separate field of `GitSide`. Reading `.git/HEAD` directly (`ref: refs/heads/main` vs a bare ID) avoids an extra process.

---

### `tasks/shared/build_measurement/test_durations.py:76` — A figure that is None in every run crashes with StatisticsError instead of a refusal
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

`RunFigure.over` calls `median` on the non-`None` values, and `median([])` raises `statistics.StatisticsError`. `DurationFigures.__post_init__` only checks that each charged test passes in *some* run. A target and its checker that never pass in the same run make `_charged` return `None` for every run, which is plausible with `--log` alone-run logs filtered to single tests. The same happens in `_binary_seconds_in` when a `--served-by` family's build line never shares a run with that binary's passes. The task catches only `MeasurementRefusedError`, so the user gets a traceback.

**Suggestion**: Route per-test and binary figures through `_figure_over` (which raises `MeasurementRefusedError`), e.g. refusing with "`<target>` and its checker `<checker>` never pass in the same run", and add a test for it.

---

### `cli/work-adapters/tests/rekey_run.rs:1106` — each_of's expected count comes from its own items, so the guard is tautological
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage, code-quality

🔵 **Test Coverage / Code Quality**

`each_of`'s doc says the caller states `expected` independently of `items`. Here (and in the kill test, `Vcs::ALL.len() * halts`) both come from `mutations_of_a_whole_run*()`, so `each_of` guards only against zero iterations. The `assert_eq!(checked + skipped, halts)` that follows is always true, because every iteration either skips or increments `checked`. Nothing fixes the count at the 14 (and 13) the PR description relies on.

**Impact**: If a production change cut a run's mutations from 14 to 3, both tail tests would quietly check far fewer halt points and still pass.

**Suggestion**: Fix the counts once in `a_whole_run_makes_exactly_the_counted_mutations`, e.g. `assert_eq!(mutations_of_a_whole_run()?, 13)` and `assert_eq!(mutations_of_a_whole_run_over(seed_synced_only)?, 14)`. Replace the tautology with an assertion on the expected number of `checked` halts. `each_of(1, [Vcs::Git], ...)` at line 1035 can become a direct call.

---

### `cli/vcs-test-support/src/family.rs:119-127` — inputs_from_environment is on the SharedFamily trait but never called generically
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`SharedFamily::inputs_from_environment` is a required trait method, but its only caller is the concrete `Family::StalledMigration` arm in `shared_build.rs`, which calls `StalledMigrations::inputs_from_environment()` directly. `Matrix`, `States` and the test doubles in `tests/workbench.rs` implement no-ops that nothing reads. The trait now mixes the build step's handover policy, reading ambient environment, with what a family is.

**Impact**: Every future family pays for a hook that nothing generic uses, and the trait suggests a uniform build path that does not exist.

**Suggestion**: Keep `type Inputs`, which the private-build signatures need. Either make the dispatch generic (`F::build_for_sharing(tree, &F::inputs_from_environment()?)`) so the method earns its place, or move the lookup to an inherent `StalledMigrations` function next to the other handover reading.

---

### `cli/vcs-test-support/src/stalled_migration/corpus.rs:12-43` — vcs-test-support now owns migrate's migration list and invocation contract
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

`vcs-test-support`, which about 13 workspace crates depend on, now holds migrate-specific knowledge: the ordered `MIGRATIONS_BEFORE_0007` IDs, the ledger path, the work-item frontmatter shape, and `migrate_command`, including which `ACCELERATOR_MIGRATE_*` variables to scrub. `migrate-cli/tests/common::run` now gets its own binary's invocation contract from a shared crate. The ledger is also split: `MIGRATIONS_FROM_0007` and its helpers stay in `migrate-cli/tests/common`. Inside `guarded_resume.rs`, this module's name shadows the workspace's `corpus` crate.

**Impact**: Adding a migration, renaming a migrate variable or renaming the test binary now means editing a VCS fixture crate, in two places.

**Suggestion**: Keep all migrate knowledge behind the `stalled_migration` boundary (move `stalled_migration_manifest` beneath it too), under a name that does not clash with `corpus`, e.g. `stalling_corpus`. Keep the whole ordered ledger in one place, and record the tradeoff in `tasks/README.md`.

---

### `cli/vcs-test-support/src/repository_state/reading.rs:112` — Three git-reader branches have no test: annotated-tag peeling, conflict stages and gitlinks
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The git reader peels annotated tags through `%(*objectname)`, keys conflicted index entries as `path (stage n)`, and skips gitlinks (mode 160000) in both the index and trees. No test reaches these branches: `git_history` makes only a lightweight tag, and no fixture has a conflicted index or a submodule entry. Dropping the peel, or merging the stages into one key, would leave every test green.

**Impact**: `RepositoryState` is now the oracle for copy fidelity, so a mis-read here, such as two conflict stages collapsing into one entry, could let unequal states compare equal unnoticed.

**Suggestion**: Add cases to `tests/repository_state.rs`: an annotated tag projecting to the tagged commit's position, an unmerged index that differs from a resolved one, and a submodule entry being ignored.

---

## Per-Lens Results

### Architecture

**Summary**: The PR fits a third family into the existing shared-fixture build, splits repository-state reading into an I/O part and a pure part, and extends the Rust/Python contract files instead of copying values between the two languages. The most significant structural change is that the shared build step, which used to depend only on git and jj, now runs the production `accelerator-migrate` binary, so a change in migrate's stall behaviour stops the build step before any test starts. `vcs-test-support` also now holds migrate-specific knowledge, and `SharedFamily` has an environment-reading hook that no generic code calls.

**Strengths**:
- `RepositoryState` keeps reading (`reading.rs`, the only place that spawns git/jj) separate from pure projecting (`order.rs`, `masking.rs`).
- The `executables` key in `contract/variables.json` is checked against both Rust's `MIGRATE_VARIABLE` and Python's `Family.executables`.
- The macOS `fclonefileat` path falls back on `XDEV`/`NOTSUP` and compiles to `false` elsewhere, hidden behind `copy_part`.
- `guarded_resume` builds its reference stalls with its own recipe, so drift in the family's recipe is caught.
- `measure:test-durations` keeps parsing pure and I/O in the `invoke` task.
- Contract tests hold `UnsyncedStore` to `FileCorpusStore`.

**Comments**:
- `cli/vcs-test-support/src/stalled_migration.rs:136-147` — major (orchestrator: minor after checking), medium — Shared build step now depends on production migrate behaviour; a stall that does not stall aborts the step. *Orchestrator note: `tasks/README.md:406` documents this, and only runs selecting `guarded_resume` build the family; downgraded to minor.* Suggests a non-fatal family failure with a private-build fallback, or stating the rationale.
- `cli/vcs-test-support/src/stalled_migration/corpus.rs:12-43` — minor, high — `vcs-test-support` now owns migrate's migration list and invocation contract.
- `cli/vcs-test-support/src/family.rs:119-127` — minor, high — `inputs_from_environment` on `SharedFamily` is never called generically.
- `cli/work-adapters/tests/support/unsynced_store.rs:62-89` — minor, medium — Test store re-implements the store crate's staged-create protocol and error mapping.
- `tasks/shared/build_measurement/test_durations.py:27-29` — minor, medium — Family build times are parsed from a console format defined in `session.py`, not from the fixture log's `seconds`.

**General Findings**:
- minor — Adding a shared fixture family touches about eight places across two languages with no checklist.

### Code Quality

**Summary**: Mostly well structured: `RepositoryState` is split into focused, unit-tested modules, duplicated logic is pulled out (`require_clean`, `corpus`, `migrate_command`), and error messages name the stall key, migrate's exit code and a reproduction command. The main maintenance costs are `guarded_resume`'s `Stalled` handle repeating a fallible lookup on almost every access and `UnsyncedStore` re-implementing production store logic, plus several smaller design points.

**Strengths**:
- `RepositoryState` split into reading, order (with `TieBreak`'s derived order as the tie-break order) and masking, each unit-tested.
- `TreeCopy::require_clean` replaces a hand-built message in `Workbench` and is reused by `copy_private_tree`.
- The 0007 stall corpus, `work_item` and `migrate_command` live in one place.
- Failure messages name the key, exit code, stderr and a reproduction command.
- `each_of` makes loop counts a stated expectation.
- `ParentShape`, `StallKey`, `JjColocation` and `MigrateBinary` replace string flags and the `Base` enum.

**Comments**:
- `cli/migrate-cli/tests/guarded_resume.rs:167-169` — major, high — Every access to the stall repeats a fallible map lookup.
- `cli/work-adapters/tests/support/unsynced_store.rs:62-90` — major, high — `UnsyncedStore` re-implements the store crate's atomic create and error mapping.
- `cli/vcs-test-support/src/family.rs:127` — minor, medium — `inputs_from_environment` is on the generic trait but only one family uses it.
- `cli/work-adapters/tests/rekey_run.rs:1111` — minor, high — `each_of`'s expected count is derived from its own items, so the guard is tautological.
- `cli/work-adapters/tests/rekey_run.rs:972-977` — minor, medium — Two parallel ways to build a committed repository (`Committed` vs `committed`/`five_files_committed`).
- `tasks/shared/build_measurement/test_durations.py:225` — minor, high — Validation done by calling a query and discarding its result (also line 199).
- `cli/vcs-test-support/src/copies.rs:171-190` — minor, medium — `restore_what_a_clone_drops` also adds owner-write, which its name and doc do not say.
- `cli/vcs-test-support/src/stalled_migration.rs:72-77` — suggestion, medium — Colocation's display name lives on `StallKey` rather than `JjColocation`.
- `cli/vcs-test-support/src/repository_state/reading.rs:443-467` — suggestion, medium — `TreeEntries` yields `Option<Option<TreeEntry>>` via an immediately invoked closure; `"40000"` is an unnamed literal.
- `tasks/shared/build_measurement/test_durations.py:22` — suggestion, medium — Log-line classification duplicated from `timeline.py`.
- `tasks/measure.py:3866-3897` — suggestion, medium — Log discovery helpers added to an already 3,800-line task module.

**General Findings**: none.

### Test Coverage

**Summary**: The rewritten tests keep every earlier scenario and assertion, and are stronger in places: the edit-refusal test now halts at every mutation, `each_of` makes loops prove their count, and each reused state has a copy-equals-fresh-build test. The new support code is well tested. Gaps: parts of the projection and its readers no test covers, a shared-build success path with no test of its own, and call sites that weaken `each_of`'s independent-count guarantee.

**Strengths**:
- No scenario lost; `Stalled::on`'s stall checks moved into `Stall::build`.
- Edit-refusal test halts at all 14 mutations; a new test cross-checks the scripted count against a real run.
- `each_of` has its own unit tests for exact, fewer, more, zero and failing-body cases.
- Copy-equals-fresh-build tests for both colocations and parent shapes, and both VCSes for the rekey templates.
- `RepositoryState` tested from both directions: equal builds compare equal, and each covered field changing makes them differ.
- `UnsyncedStore` runs the same eight contract cases as `FileCorpusStore`.
- Python tests anchored to three real baseline logs; regression test for the `ERROR task failed` fix.
- Stalled-migration failure paths tested with stub migrate scripts.

**Comments**:
- `cli/vcs-test-support/src/repository_state.rs:166-171` — minor, medium — The oracle cannot see author identity, jj operation history or working-copy state.
- `cli/vcs-test-support/src/repository_state/reading.rs:112` — minor, high — Annotated-tag peeling, conflict stages and gitlinks are untested.
- `cli/vcs-test-support/src/stalled_migration.rs:183` — minor, medium — The shared build's success path (write manifest, adopt, copy) has no test of its own.
- `cli/work-adapters/tests/rekey_run.rs:1106` — minor, medium — `each_of`'s expected count comes from its items; a shrinking mutation count would go unnoticed.
- `cli/work-adapters/tests/rekey_run.rs:1275` — suggestion, low — `UnsyncedStore` contract omits a symlinked final path component.
- `tasks/shared/shared_fixtures/session.py:43` — suggestion, high — The one- and two-family "building …" messages lost their test.

**General Findings**: none.

### Correctness

**Summary**: The core mechanics hold up: the `fclonefileat` clone and its pre-order restoration are sound, the stalled-migration family, manifest and rebasing are consistent with the existing families, and the rewritten loops are guarded by iteration counts. The main gaps are in the `RepositoryState` projection: it never records the jj working-copy commit, and it treats attached and detached `HEAD` at the same commit as equal. Separately, `measure:test-durations` can crash with an unhandled error when a charged test never runs in the same log as its checker.

**Strengths**:
- `restore_what_a_clone_drops` walks top-down, so restored directory times survive changes to children.
- The clone falls back only on `EXDEV`/`ENOTSUP` and propagates every other errno.
- `ordered_positions` refuses indistinguishable candidates, unknown parents and cycles.
- `each_of` with expected 0 always fails; the edit-refusal halt count now comes from the same corpus it halts.
- The manifest parser rejects malformed lines and an empty manifest.
- The `measure:build` fix makes `tasks_mise_reported_failing` agree with `_is_counted`.

**Comments**:
- `cli/vcs-test-support/src/repository_state/reading.rs:201-203` — minor, high — The jj projection never records which commit is `@`. *Orchestrator verified.*
- `cli/vcs-test-support/src/repository_state/reading.rs:115-118` — minor, high — An attached and a detached `HEAD` at the same commit project equal.
- `cli/vcs-test-support/src/repository_state/order.rs:19-20` — minor, medium — Tie-breaks compare unmasked file contents, so build-specific IDs can decide position and cause flaky inequality.
- `tasks/shared/build_measurement/test_durations.py:76` — minor, medium (orchestrator: high after checking) — A figure that is `None` in every run crashes in `median([])` instead of a refusal. *Orchestrator verified.*
- `cli/vcs-test-support/src/repository_state.rs:258` — suggestion, low — Lossy UTF-8 decoding can make different binary contents compare equal.

**General Findings**: none.

### Performance

**Summary**: The PR goes after the costs that dominated: about 182 of roughly 248 jj spawns in `guarded_resume`, per-pass rebuilds in `rekey_run`, and `F_FULLFSYNC` serialisation. New costs are measured and mostly small: the stalled-migration family builds in 1.1–1.8 s alongside a 5.1 s matrix build, and each projection takes 23–49 ms. The biggest remaining lever is the kill test, now the last finisher, running both VCSs serially in one test.

**Strengths**:
- The family builds its 4 stalls on parallel threads, concurrently with the matrix and status-log builds, adding nothing to the wait.
- `RepositoryState::of` spawns a fixed 2–4 processes with one `git cat-file --batch` session and a blob cache.
- `UnsyncedStore` removes per-write `F_FULLFSYNC`, held to `FileCorpusStore` by contract tests.
- Templates are made cheap to copy: `git init --template=`, `repack -a -d`, and APFS whole-directory clones.
- `matrix_private_build` halved its alone figure (6.6 s to 3.2 s).
- Careful measurement: medians of 3, checks charged to their targets, a noise verdict, a projected budget gate.

**Comments**:
- `cli/work-adapters/tests/rekey_run.rs:1072-1082` — major, medium — Kill test, the phase's last finisher, runs both VCSs serially in one test.
- `cli/work-adapters/tests/rekey_run.rs:1011-1017` — minor, medium — Copy check builds every template twice.
- `cli/work-adapters/tests/rekey_run.rs:983-985` — minor, medium — New exact-count test rebuilds repositories instead of copying a template.
- `cli/vcs-test-support/src/copies.rs:174-205` — minor, low — A cloned copy is walked twice: once to restore, once to scan contents.

**General Findings**:
- minor — Late-starting long tests still decide where the phase ends; nextest `priority` overrides are unused.
- minor — Copy-based speed-ups measured only on macOS; the Linux CI leg uses the entry-by-entry fallback.
- suggestion — The copy-equals-scratch checks are a lasting share of the suite's test-seconds.

### Standards

**Summary**: The PR mostly follows the repo's conventions: the new family is registered on every surface the existing families use, and Rust names follow existing patterns. The issues are naming and placement — a `corpus` module shadows the workspace's `corpus` crate, the migration ledger is split across two crates, and a jj colocation's display name is defined twice — plus two README lines over 80 columns and a README guard description not updated for `copy_private_tree`.

**Strengths**:
- The third family is registered on every surface; nothing half-registered.
- `stalled_migration_manifest.rs` mirrors the existing manifest modules; `writable_stalled_migrations` follows `writable_status_log_states`.
- `copy_private_tree` got a guard entry with an explicit allow-list.
- `measure:test-durations` follows the measure module's conventions, with help per argument and a `test_mise.py` registration.
- No comment references a work item, ADR or plan phase.

**Comments**:
- `cli/migrate-cli/tests/guarded_resume.rs:21` — minor, high — `stalled_migration::corpus` shadows the workspace `corpus` crate.
- `cli/migrate-cli/tests/common/mod.rs:25` — minor, medium — Migration ledger constants now split across two crates.
- `cli/vcs-test-support/src/stalled_migration.rs:72-77` — minor, medium — Colocation's display name is defined on `StallKey`, not `JjColocation`; assertion messages use `flag()` instead.
- `tasks/README.md:325-331` — minor, high — Two rewrapped prose lines exceed 80 columns (84 and 92).

**General Findings**:
- minor — README's list of what the fixture-build guard refuses omits `copy_private_tree`.
- suggestion — `Prefix` in `guarded_resume` needs its doc comment to say what it is; a domain name such as `CorpusBesideSibling` would let it go.

### Portability

**Summary**: The one platform-specific mechanism is contained: the macOS `fclonefileat` path sits behind `cfg(target_os = "macos")` and falls back to the recursive copy, and the copy-contract tests run on both CI legs. Remaining points: `rustix` is declared for every platform, the stalled-migration manifest records absolute paths, the figures come from macOS only, and the duration report depends on nextest's human-readable output.

**Strengths**:
- `cloned_whole` returns `false` rather than failing; no platform loses the ability to copy.
- `tests/copies.rs` holds both strategies to one contract, run on `ubuntu-latest` and `macos-latest`.
- The copy guard checks both given and canonical source paths, catching `/var` vs `/private/var`.
- The migrate path is an explicit input with a named variable; builder tests use stub executables.
- `UnsyncedStore` uses the production staging and publishing primitives, tested on both platforms.

**Comments**:
- `cli/vcs-test-support/Cargo.toml:27` — suggestion, high — `rustix` declared for every platform but used only on macOS; follow `cli/launcher/Cargo.toml`'s target table.
- `cli/vcs-test-support/src/copies.rs:207-237` — minor, medium — Each directory-copy strategy runs on only one platform; the macOS block is never linted in CI.
- `cli/vcs-test-support/src/stalled_migration_manifest.rs:28-34` — minor, medium — Manifest stores absolute stall roots, so the shared tree cannot be moved and a path mismatch drops a stall silently.
- `cli/vcs-test-support/src/stalled_migration.rs:133-134` — minor, medium — A missing or stale migrate path fails without naming the path or its variable.
- `tasks/shared/build_measurement/test_durations.py:23-26` — suggestion, low — Duration parsing depends on nextest's human-readable status lines.

**General Findings**:
- minor — The speed-up rationale and figures are macOS-only; the Linux CI path is unmeasured.

### Documentation

**Summary**: The documentation is mostly accurate and in proportion: `tasks/README.md` covers the family, its handover variable, the builder input, per-family timings and `measure:test-durations`, and spot-checks against the code and plan found no contradiction. Gaps: one stale doc comment, one partial rustdoc, a rustdoc pointing at an unnamed guard, and two README omissions.

**Strengths**:
- `tasks/README.md` updated everywhere the new family touches, agreeing with `families.py`, `consumers.py` and the contract JSON.
- The `measure:test-durations` paragraph, mise description, help text and docstring agree with each other and the code.
- Doc comments record rationale (`IdMasking`, `TieBreak`, `UnsyncedStore`, "the median of sums, never the sum of medians").
- `RepositoryState::of`'s process-count claim matches `reading.rs`.
- No comment references a work item, plan phase or ADR.
- The PR description and plan agree with the code as built.

**Comments**:
- `cli/migrate-cli/tests/guarded_resume.rs:139-140` — minor, high — Stale doc comment on the restructured `Stalled` (mentions a `base` field it no longer has).
- `cli/vcs-test-support/src/copies.rs:171-173` — minor, high — `restore_what_a_clone_drops` doc omits the owner-write step and implies the clone loses it.
- `cli/vcs-test-support/src/copies.rs:98-99` — minor, medium — `copy_private_tree` rustdoc refers to "a guard" without naming `tests/unit/tasks/test_fixture_builds.py`.
- `tasks/README.md:688-699` — minor, medium — `measure:test-durations` paragraph omits the family-build output, that `--binary` is required, the `--record`/`--log` exclusivity and the file-name-only glob.
- `cli/migrate-cli/tests/common/mod.rs:56` — suggestion, medium — Doc comment on `run` now restates its body.
- `cli/vcs-test-support/src/repository_state.rs:174` — suggestion, low — `CommitPosition`'s meaning is documented only on a private function.

**General Findings**:
- minor — README guard bullet does not list `copy_private_tree`, nor the template-copy pattern with its `RepositoryState` check.
- suggestion — A family that needs a bin target is a new extension point with no how-to.

---
*Review generated by /accelerator:review-pr*
