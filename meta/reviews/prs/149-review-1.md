---
type: "pr-review"
id: "149-review-1"
title: "[0299] Hold the CLI workspace's crates to ADR-0069's injection rules"
date: "2026-10-07T23:42:53+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:149"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility", "performance", "safety"]
review_number: 1
pr_number: 149
tags: []
last_updated: "2026-10-07T23:42:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #149 - [0299] Hold the CLI workspace's crates to ADR-0069's injection rules

**Verdict:** COMMENT

The refactor does what it sets out to do. The dependency graph now satisfies ADR-0069, the new ports sit in the domain crates that own each capability, and the lint encodes the rules as small pure functions over a rich `Role`/`Kind`/`BoundedContext` model. The characterisation suite is hermetic and can genuinely fail, and the moves it guarded (`store::lock`, the validation pipeline, m0001/m0008, the tracker-block validator) are byte-for-byte faithful. The open concerns all come from the launcher's new in-process tracking. The old 2 s / 4 KiB child-process boundary is gone and nothing replaces it. A process-wide panic hook now covers every launcher panic. And the doubled binary is verified in full on every hook invocation, while the only figure that measures that cost is deferred until after merge.

### Cross-Cutting Themes

- **In-process tracking has no deadline** (flagged by: safety, correctness, compatibility, architecture) — `DispatchedTracking` folded a hang to `Unknown` after 2 s. `InProcessTracking` bounds only panics, so a stalled gix index read or a stalled jj `load_at_head` blocks SessionStart until Claude Code's hook timeout, which then drops the whole summary envelope.
- **Launcher size lands on the hottest path** (flagged by: architecture, performance) — `bin/accelerator` runs `accelerator-verify` on every cache hit, and the verifier `fs::read`s the whole launcher before hashing it. Going from 8.07 MB to 16.67 MB doubles that work on each PreToolUse invocation, which runs twice per Bash tool call. The summary-latency task calls the launcher directly, so it never measures this. Warm dispatch is measured only after merge, against a 10× gate.
- **Process-wide panic hook** (flagged by: code-quality, safety, correctness, test-coverage) — `report_panics_through_the_log` replaces the default hook for the whole launcher. It double-logs every recovered fold (ERROR, then WARN) and silences genuine panics when `ACCELERATOR_LOG=off`. The test meant to cover it relies on a debug-only overflow in `gix-index`.
- **Panic fold depends on `panic = "unwind"` with nothing enforcing it** (flagged by: safety, architecture) — `[profile.release]` sets only `strip` and `lto`. Switching to `panic = "abort"` is the obvious way to shrink the doubled launcher, and it would silently turn the fail-closed `Unknown` into an abort.
- **Visualiser registration window** (flagged by: performance, correctness, compatibility, test-coverage) — the server now accepts connections before watches register. The 1024-slot channel is not drained during registration, no rescan runs once it finishes, no readiness signal reaches clients, and `registered()` has no test.
- **Lint findings print ADR rule numbers** (flagged by: code-quality, standards) — `jira-client -> corpus-adapters: rule 4` means nothing without the ADR open, even though the `Rule` members already carry domain names.
- **Lint reads only the first domain crate** (flagged by: code-quality, correctness) — `_resolved_context` takes `domains[0]`, so a second domain crate's `kind` or `downstreams` is never checked.
- **`MigrationContext` keeps widening** (flagged by: code-quality, architecture) — three default methods fail at runtime with "not implemented" and re-wrap a `FrontmatterParser` the context already holds.

### Tradeoff Analysis

- **Rule 6 compliance vs hook-path cost**: answering tracking in the launcher removes a cross-binary protocol, works offline, and makes SessionStart 4–5× faster. The cost is gix/jj-lib in the binary that is verified and executed on every hook. ADR-0069 allows the launcher to link platform adapters but does not require it. Measure warm dispatch through the bootstrap before merge. If the regression is material, a dispatched tracking helper keeps rule 6 and keeps the launcher small.
- **Fault isolation vs latency**: the child process gave a hard time, memory and crash boundary, and the in-process call is faster and simpler. A `Deadline<T: RepositoryTracking>` decorator next to `PanicFold` (spawned thread plus `recv_timeout`) restores the time bound without bringing back the child.
- **Rule 6 vs platform purity**: moving Jira/Linear block vocabulary into `config::catalogue` drops `launcher → tracker-support`, but it pushes product knowledge into a platform crate. Accept that and record it, or keep the schema mechanism in `config` and have `tracker` own the `TRACKERS` instances.

### Strengths

- ✅ The lint models the ADR in domain terms: `Role`, `Kind`, `BoundedContext.admits` and `shares_adapters_with`, with `broken_rules` as a pure, exhaustive match. The shipped workspace passes with no exception list, and `_EXPECTED_DECLARATIONS` pins every crate's self-declared role and kind.
- ✅ The new ports are placed correctly. `vcs::tracking::RepositoryTracking`, `collaboration::RepositoryOrigin`, `corpus::frontmatter::FrontmatterParser` and `migrate::ports::SyncBaselines` keep cross-context wiring at the composition roots. `TrackedConfigFile` translates `FileTracking` into config's own `Tracking` vocabulary rather than sharing the type.
- ✅ The validation pipeline moved into `corpus` with no `std::fs`, env or process use, which strengthens the functional-core / imperative-shell split.
- ✅ `store::lock` is a verbatim move: nonce-bound single-winner reclaim, PID-liveness gate, 300 s ceiling, and the same `owner`/`reclaiming` on-disk names, so different binaries still coordinate. `from_lock_error` keeps every display string.
- ✅ m0008's value-change guard is kept exactly, and the new `MigrationContext` defaults refuse rather than write.
- ✅ The characterisation harness scrubs the environment, uses loopback network URLs, refuses nested fixture roots, fails on a missing golden, refuses `UPDATE_GOLDEN` in CI, and enforces git/jj parity at collection. The consent-read goldens tell tracked from untracked runs apart for every composition root.
- ✅ `repository_tracking.rs` exercises the port against real git, jj, colocated, nested, worktree and secondary-workspace layouts, including a corrupted index and crafted `.jj` markers.
- ✅ SessionStart median latency fell from ~32 ms to 6–9 ms. Tracking is only asked when `config.local.md` exists, and jj reads the recorded tree without a snapshot.
- ✅ `vcs tracking` has no remaining callers in skills, hooks, agents, templates or docs-site. The launcher fetches sub-binaries pinned to its own version tag, so version skew cannot occur.
- ✅ The `BashCommandRunner` fold is strictly stricter than before and ships with a regression test.
- ✅ The "fourteen-point" count agrees across root `CLAUDE.md`, `tasks/CLAUDE.md`, `tasks/public_api.py` and the README, and `workspace_packages()` moved into `tasks/shared/`.

### General Findings

- 🔵 **Compatibility**: the Unreleased CHANGELOG entry "Session start warns about consent keys" still describes the sub-binary check and its "skipped" note, both removed by this PR. It also leaves out that a tracked `config.local.md` now warns in every session, offline included.
- 🔵 **Architecture**: `cli/config/src/catalogue.rs` (546–584, outside the diff hunks) now carries Jira/Linear block vocabulary, and the ceiling semantics are duplicated across `config::tracker_block` and `tracker` with only a test keeping them in step.
- 🔵 **Test Coverage**: no test or pup rule pins the removed `launcher → store` and `collaboration → vcs` edges, and the lint permits both.
- 🔵 **Correctness**: the e2e health port is bind-then-release, and `start-server.mjs` has no `'error'` handler, so losing the race crashes the wrapper opaquely after a 60 s timeout.
- 🔵 **Safety**: if m0008's partial-failure recovery advice ("revert this migration commit") is followed, any baselines outside the repository stay realigned. This behaviour predates the PR.
- 🔵 **Code Quality**: `pull.rs` and `push.rs` duplicate the same per-block error, parse, validate and read scaffolding.
- 🔵 **Architecture**: consent wiring repeats across 7 roots in 2 different shapes.
- 🔵 **Performance**: the before and after summary figures were taken at host loads of 8.2 and 46.9, as point medians with no confidence interval.

### Additional Findings

- 🔵 `cli/vcs-adapters/src/panic_fold.rs:5` — fold silently depends on `panic = "unwind"` (safety, architecture)
- 🔵 `cli/visualiser/server/src/watcher.rs:109` — events dropped or never reconciled during the registration window, with no readiness signal (performance, correctness, compatibility)
- 🔵 `cli/visualiser/server/src/watcher.rs:97` — graceful shutdown waits on the in-flight `spawn_blocking` registration after lifecycle files are removed (correctness)
- 🔵 `tasks/lint/crate_dependencies.py:248` — second domain crate in a context silently ignored; `""` sentinel in `refused_contexts` (code-quality, correctness)
- 🔵 `cli/tracker-support/src/pull.rs:116` — parser matches hard-coded field names rather than catalogue constants (code-quality)
- 🔵 `cli/migrate/src/ports.rs:240` — `MigrationContext` default methods fail at runtime and re-wrap the corpus port (code-quality, architecture)
- 🔵 `tasks/measure.py:3373` — summary harness still builds and pins `accelerator-vcs` and guards a removed note (code-quality)
- 🔵 `cli/work-adapters/src/sync/realign.rs:19` — `RealignError::Listing` loses path context and `source()` (code-quality)
- 🔵 `tasks/lint/crate_dependencies.py:401` — rule 2 cycle check covers only platform contexts (architecture)
- 🔵 `cli/vcs-adapters/src/tracking.rs:56` — git index decoded twice in colocated repos; benchmarked only on a 4.6k-entry index (performance)
- 🔵 `cli/config-adapters/src/command_runner.rs:82` — symlink fold only catches a lexically canonical prefix (correctness)
- 🔵 `cli/visualiser/server/tests/sse_e2e.rs:110` — rewrite-until-event loop with a 300 s budget gives up the latency guarantee (test-coverage)
- 🔵 `tests/integration/support/characterisation.py:393` — no detection of orphaned or colliding goldens (test-coverage)
- 🔵 `cli/vcs/src/lib.rs:149` — `Repository*` ports beside existing `Repo*` names (standards)
- 🔵 `cli/store/src/lock.rs:174` — comments narrate design history (code-quality)
- 🔵 `tests/integration/characterisation/test_summary_tracking.py:40` — with/without-vcs-binary parameter no longer separates behaviour (test-coverage)
- 🔵 `mise.toml:254` — `characterisation` in the `<token>` slot; `build:cli:dev` now launcher-only (standards)
- 🔵 `mise.toml:435` — several single-binary dev builds per lane may rebuild gix/jj-lib under different feature sets (performance)
- 🔵 `tasks/shared/dev_builds.py:10` — two comments better names could replace (standards)
- 🔵 `tasks/lint/crate_dependencies.py:316` — edges attributed by package name only, not path dependency (correctness)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/launcher/src/main.rs:283` — In-process tracking check has no deadline (Safety, Correctness, Compatibility, Architecture)
**Severity**: major | **Confidence**: high | **Lens**: safety, correctness, compatibility, architecture

🟡 **Safety / Correctness / Compatibility / Architecture**

The deleted `DispatchedTracking`/`UnixCapture` pair ran `vcs tracking` as a captured child with a 2 s deadline, a 4 KiB output cap and process-group teardown, and folded any of those to `Unknown`. `TrackedConfigFile(InProcessTracking)` keeps only the panic case. `panic_fold.rs` says it "does not cover a hang", and `InProcessProbe` documents "no time, memory or crash bound". `hooks/hooks.json` sets no `timeout` on the `config summary` entry.

**Impact**: three things can now block session start until Claude Code's default hook timeout: `jj_is_tracked`'s `load_at_head` under `pollster::block_on`, a gix index read on a network filesystem, or a wedged jj op store. The kill then drops the whole summary envelope, consent warnings included. Before, this degraded to a fail-closed answer after 2 s.

**Suggestion**: add a `Deadline<T: RepositoryTracking>` decorator next to `PanicFold`. It runs the question on a spawned thread and folds a `recv_timeout(Duration::from_secs(2))` miss to `FileTracking::Unknown`; the abandoned thread dies with the short-lived process. Test that a slow `RepositoryTracking` answers `Unknown` within the bound. If an unbounded hook is a deliberate choice, record it in ADR-0069's negative consequences.

---

### `cli/launcher/Cargo.toml:36` — Every launcher invocation pays for the gix/jj-lib closure only SessionStart uses (Architecture, Performance)
**Severity**: major | **Confidence**: high | **Lens**: architecture, performance

🟡 **Architecture / Performance**

Linking `vcs-adapters` takes the launcher from 8.07 MB to 16.67 MB. On every cache hit, `bin/accelerator` runs `verify_launcher` (line 435), and `cli/verify/src/main.rs` calls `std::fs::read(target_file)` on the whole launcher before minisign hashes it. PreToolUse invokes the launcher twice per Bash tool call (`vcs guard`, `research guard`), so each tool call now reads and hashes ~33 MB instead of ~16 MB, on a path that never asks the tracking question. `cli/Cargo.toml`'s own profile comment calls binary size "a per-call latency term".

**Impact**: an estimated ~10–20 ms per invocation on Apple Silicon against the 44 ms warm-dispatch baseline. This is inferred from the mechanism and has not been measured. A capability used once per session moves its cost onto the most frequent path in the plugin.

**Suggestion**: before merge, hyperfine `accelerator-verify` over the old and new launcher; this needs no signed prerelease. If the regression is material, consider caching the verification result keyed on (inode, size, mtime, signature digest), or streaming the file through the hasher. Alternatively, answer tracking in a dispatched platform binary. ADR-0069 allows the launcher to link platform adapters but does not require it.

---

### `tasks/measure.py:3456-3466` — Summary-latency figures bypass the bootstrap; warm dispatch is gated only after merge at 10×
**Severity**: major | **Confidence**: high | **Lens**: performance

🟡 **Performance**

`time_summaries` runs `[str(launcher), *SUMMARY_ARGS]` directly, skipping `bin/accelerator`. The 0.18–0.25 ratios therefore leave out the one cost that grows with binary size. "Cold" mode keeps the OS page cache warm. The only figure that covers PreToolUse, warm dispatch, is deferred until after merge with a 10× stop threshold, so 44 ms could become 440 ms and pass, while the harness's own `RATIO_THRESHOLD` is 1.4. The before C1 figure also came from a run marked `invalid-post-run`.

**Impact**: the evidence that the size doubling is harmless covers SessionStart only. A regression on every tool call could ship and be caught only by a revert.

**Suggestion**: add a pre-merge figure that goes through the bootstrap: point the timed command at `bin/accelerator` with `ACCELERATOR_LAUNCHER_BIN` set to the release build. Gate the post-merge warm-dispatch check at 1.4, not 10.

---

### `cli/launcher/src/main.rs:439-450` — Process-wide panic hook installed for one adapter (Code Quality, Safety, Correctness)
**Severity**: major | **Confidence**: medium | **Lens**: code-quality, safety, correctness

🟡 **Code Quality / Safety / Correctness**

`report_panics_through_the_log` replaces the global hook for every launcher command, but the in-process tracking check is its only justification. The hook runs before `catch_unwind` takes over, so every panic that `PanicFold` deliberately recovers logs as `ERROR panicked` and then as `WARN panicked answering …`, and a handled refusal reads like a crash. Panics nothing catches (resolve, fetch/verify, tree leasing) now go only through `tracing`. With `ACCELERATOR_LOG=off`, which the characterisation harness itself sets, they exit 101 with nothing on stderr.

**Impact**: a fail-closed condition handled by design is reported at error level, and a genuine launcher crash becomes undiagnosable under a restrictive filter.

**Suggestion**: scope the quiet hook to the folded call. Use `take_hook`, install the quiet one, and restore the previous hook around `PanicFold`, or have one hook check a thread-local "folding" flag. Leave the default hook in place everywhere else. Failing that, log at the fold's level and fall back to the previous hook when no subscriber would record the event.

---

### `cli/launcher/tests/config_read.rs:2963-2980` — Panic-hook test relies on a debug-only overflow inside gix-index
**Severity**: major | **Confidence**: medium | **Lens**: test-coverage

🟡 **Test Coverage**

`a_tracking_panic_with_logging_off_is_unknown_and_silent` causes its panic by writing `DIRC-not-an-index` over `.git/index`. Per the plan's Phase 7 notes, that only panics because of a subtract-with-overflow in `gix-index` under debug arithmetic checks. A release build, or a gix upgrade that fixes the overflow, turns it into an ordinary `Err` → `Unknown`. The test still passes, but it no longer exercises `report_panics_through_the_log` or `PanicFold`. Nothing tests the hook's effect on panics outside the fold.

**Impact**: the test's name promises coverage that a dependency bump can silently remove.

**Suggestion**: add a seam that panics deliberately. One option is a `test-loopback`-style feature hook in a fixture binary that panics inside the tracking call; another is a unit test of `PanicFold` with the installed hook. Assert `Unknown` and empty stderr. Separately, pin what a panic outside the fold shows the user.

---

### `tasks/lint/crate_dependencies.py:55-66` — Findings report opaque rule numbers instead of the enum's domain names (Code Quality, Standards)
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, standards

🔵 **Code Quality / Standards**

`Rule`'s members have meaningful names (`INWARD`, `UPSTREAM`, `INJECTION`, `WIRING_AT_ROOTS`, `LAUNCHER`), but their values, which `Finding.__str__` prints, are `"rule 1"`, `"rule 4"` and so on. A failure reads `jira-client -> corpus-adapters: rule 4`. The module docstring (line 1), the `check` docstring and `Exit` message (lines 475–479), and the mise description all name ADR-0069, which `CLAUDE.md` asks code not to do.

**Impact**: whoever hits the lint has to open the ADR to learn what broke, and the output goes stale if the ADR is superseded or renumbered.

**Suggestion**: give each value a self-explanatory phrase, e.g. `INJECTION = "an adapter depends on another context's adapter; inject a port at a composition root"`, and drop the ADR references from the docstrings, the `Exit` message and `mise.toml`.

---

### `cli/config-adapters/src/consent.rs:1-5` — Module comment claims the launcher is kept free of gix/jj-lib, which this PR makes false
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

The module doc says injecting the port "keeps `gix` and `jj-lib` out of its dependents, the launcher and the visualiser server among them". In this same PR the launcher depends on `vcs-adapters` directly and links the full gix/jj-lib/uluru closure.

**Suggestion**: state only what this crate guarantees ("depends on the `vcs` domain alone"), or drop the sentence now that the crate-dependency lint enforces it.

---

### `cli/corpus-adapters/src/store.rs:124-135` — `#[non_exhaustive]` on an internal `LockError` forces dead catch-all arms that discard context
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

All of `store::lock::LockError`'s consumers are in this workspace, yet it is `#[non_exhaustive]`. So `from_lock_error` carries an unreachable `other =>` arm that builds `StoreError::Io { path: String::new(), … }`, and `CacheError::Io` does the same in jira-client and linear-client. Each variant holds its cause as `detail: String`, so `source()` is lost.

**Impact**: a new lock failure would be folded into a path-less I/O error instead of the compiler pointing at every call site.

**Suggestion**: drop `#[non_exhaustive]` (the public-api fixture already tracks breaking changes) and delete the wildcard arms.

---

### `tasks/lint/crate_dependencies.py:259-263` — Some declaration-check branches survive mutation
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The only test of misplaced `downstreams` uses a platform domain crate. Three things can be deleted without failing any test: downstreams on an adapter (259–263), downstreams on a role with no context (167–168), and the `refused_contexts` skip (293–294), which exempts every sibling of a malformed crate from judgement.

**Suggestion**: add `test_a_malformed_declaration_is_reported` cases for `_crate("tracker-support", "adapter", context="tracker", downstreams=["work"])` and `_crate("store", "technical-library", downstreams=["work"])`. Add one test that pins what happens to a valid sibling's forbidden edge when another crate in its context is malformed.

---

### `cli/visualiser/server/src/watcher.rs:634` — `Watching::registered` contract and the failed-open path are untested
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`starting_does_not_wait_for_watch_registration` never asserts that `registered()` stays pending while `HeldRegistration` blocks. Moving `registered_tx.send(true)` ahead of `register(...)` would fail no test. The `open` → `Err` path is untested too: `registered()` should still resolve, and `finished()` should return the `JoinError` that `server.rs` logs.

**Impact**: the OS-watcher unit tests depend on `registered()` meaning "watches are live". If that signal fired early, they would become flaky rather than fail clearly.

**Suggestion**: before dropping `release`, assert that `tokio::time::timeout(short, watching.registered())` times out. Add a test with an `open` that returns `notify::Error::generic("x")` and assert `finished().await.is_err()`.

## Per-Lens Results

### Architecture

**Summary**: The dependency graph now does what ADR-0069 asks. Adapters no longer reach into other contexts' adapters, the ports sit in the owning domain crates, and cross-context wiring is visible at the roots. The lint encodes rules 1, 2, 4, 5 and 6 cleanly. The main concern is the launcher's in-process gix/jj-lib closure. Every invocation pays for a once-per-session question, warm dispatch is deferred until after merge, and the 2 s / 4 KiB isolation boundary is gone. Smaller concerns: Jira/Linear vocabulary in `config`, a growing `MigrationContext`, and the lint's platform-only cycle check.

**Strengths**:
- The lint models the ADR in domain terms, and `test_the_shipped_declarations_match_the_expected_table` closes the self-declared-taxonomy gap.
- `RepositoryTracking` is a proper platform port, translated by `TrackedConfigFile` rather than shared, so config-adapters and the visualiser stay free of gix/jj-lib.
- `RepositoryOrigin`, `FrontmatterParser`, `SyncBaselines` and `Capabilities` move cross-context capability to the roots. migrate → work happens only in `migrate-cli::sync_baselines`.
- The validation pipeline in `corpus` does no I/O; everything goes through ports.
- The `store::lock` and `kernel::TEMP_PREFIX` moves use the technical-library and kernel roles as intended.
- The `DirectoryWatcher` port with blocking registration is a sound resilience improvement, and the symbol gate confirms the visualiser links no VCS library.

**Comments**:
- `cli/launcher/Cargo.toml:36` (RIGHT) — major / medium — Every launcher invocation pays for the gix/jj-lib closure only SessionStart uses. Size is a per-call latency term because the bootstrap re-verifies on every warm start, and PreToolUse runs the launcher two or three times per tool use. Measure warm dispatch before merge, or answer tracking in a dispatched binary.
- `cli/launcher/src/main.rs:283` (RIGHT) — minor / high — In-process tracking drops the fault-isolation boundary around repository-controlled parsing. Only the panic case is kept; the fold depends on an unguarded `panic = "unwind"`; a global panic hook is installed. Record this in the ADR's consequences and guard the panic strategy.
- `cli/config/src/catalogue.rs:546-584` (RIGHT; outside the diff hunks, moved to general findings) — minor / medium — Platform `config` now carries Jira/Linear-specific block vocabulary to satisfy rule 6. Ceiling semantics are duplicated across `config::tracker_block` and `tracker`. Accept and record this, or keep the schema in `config` and have `tracker` own the instances.
- `cli/migrate/src/ports.rs:240-284` (RIGHT) — suggestion / medium — `MigrationContext` keeps growing, and its frontmatter methods re-wrap an existing corpus port. Expose `&dyn FrontmatterParser` directly and split narrow ports.
- `tasks/lint/crate_dependencies.py:401-412` (RIGHT) — minor / medium — Rule 2's "acyclic" is only checked among platform contexts. A shared-context cycle passes. Build the cycle graph over every non-product crossing, or reject platform/shared names in `downstreams`.

**General Findings**:
- suggestion — Consent wiring repeats across 7 roots in 2 shapes (`credential_ports` vs hand-assembled `ProvenanceContext`/`ConsentPorts`). A single config-adapters factory taking `impl RepositoryTracking` would keep the roots uniform.

### Code Quality

**Summary**: The refactor is well structured: a rich lint domain model, a small `RepositoryTracking` port with a `PanicFold` decorator, and an injected tracker catalogue and migration capabilities. The main problems are a process-wide panic hook that double-logs recovered panics at ERROR, a now-false module comment, tracker parsers that still match hard-coded field names, and several smaller error-handling and vestigial-code issues.

**Strengths**:
- `crate_dependencies.py` puts behaviour on `Role`, `Kind` and `BoundedContext`, and `broken_rules` is a pure, exhaustive match.
- `workspace_packages` moved to `tasks/shared/cargo_metadata.py` and is reused.
- `PanicFold<T>` keeps panic handling apart from the VCS walk.
- `dump::assemble` takes the `TrackerCatalogue` as a parameter, so it can be tested against a made-up catalogue, and three row functions collapse into one.
- `FileMigrationContext` takes an explicit `Capabilities` struct of borrowed ports.
- `dev_builds.py` describes builds as data.
- `TrackedConfigFile` translates into config's own vocabulary.

**Comments**:
- `cli/launcher/src/main.rs:439-450` — major / medium — Process-wide panic hook installed for one adapter, double-logging recovered panics at ERROR.
- `cli/config-adapters/src/consent.rs:1-5` — minor / high — Module comment claims the launcher is kept free of gix/jj-lib.
- `tasks/lint/crate_dependencies.py:55-66` — minor / high — Findings report opaque rule numbers.
- `tasks/lint/crate_dependencies.py:248-251` — minor / medium — A second domain crate is silently ignored, and the `""` sentinel is added to `refused_contexts`.
- `cli/tracker-support/src/pull.rs:116-132` — minor / medium — Parser matches hard-coded `"filters"`/`"max_items"`/`"max_pages"` and the page-cap keys instead of catalogue constants, and `push.rs` does the same. Export named constants from `config::catalogue` and drive `parse_page_caps` from `FieldKind::PageCaps`.
- `cli/migrate/src/ports.rs:240-284` — minor / medium — Three default methods fail at runtime, and the "Only migrations 0001 and 0008 call this" doc lines will go stale. Move them to a narrower port with no default body.
- `cli/corpus-adapters/src/store.rs:124-135` — minor / high — `#[non_exhaustive]` `LockError` forces dead catch-all arms with an empty path.
- `tasks/measure.py:3373-3406` — minor / medium — The summary harness still builds and pins `accelerator-vcs`, threads `vcs_binary`, and guards the removed `UNCHECKED_NOTE`.
- `cli/store/src/lock.rs:174-180` — suggestion / medium — Comments narrate design history ("Two earlier shapes…", CI frequency). Keep the invariant and drop the history.
- `cli/work-adapters/src/sync/realign.rs:19-32` — minor / medium — `RealignError::Listing(io::Error)` has no path and no `source()`.

**General Findings**:
- suggestion — `pull.rs` and `push.rs` duplicate per-block error, parse, validate and read scaffolding. Share one `BlockError` and a generic read/validate helper.

### Test Coverage

**Summary**: Coverage is strong. The characterisation suite is hermetic, can fail, enforces parity, refuses `UPDATE_GOLDEN` in CI and fails on a missing golden. The tracking port is tested against real repositories, the lint tests follow the acceptance criteria, and `store::lock` keeps its deterministic race tests. Gaps: the panic-hook test relies on a debug-only overflow, the 300 s SSE loop gives up the latency guarantee, `registered()` is untested, some lint branches survive mutation, and orphaned goldens are never detected.

**Strengths**:
- The characterisation harness scrubs the environment, uses a counting loopback server, refuses nested fixture roots, and unit-tests its parity and newline rules.
- `test_consent_reads.py` tells tracked from untracked runs per root (exit 24 vs 25).
- `repository_tracking.rs` covers real git/jj/colocated/nested/worktree/secondary-workspace layouts, crafted `.jj` markers, the tripwire, `core.ignorecase` and a corrupted index.
- The launcher summary is covered across all repository shapes, with empty-stderr assertions.
- `store::lock` keeps its deterministic reclaim-race test, and the `LockError` mapping is tested.
- `test_crate_dependencies.py` uses exact finding strings plus real-tree tests.
- The `BashCommandRunner` symlink regression test.

**Comments**:
- `cli/launcher/tests/config_read.rs:2963-2980` — major / medium — Panic-hook test relies on a debug-only gix-index overflow.
- `cli/visualiser/server/tests/sse_e2e.rs:110-122` — minor / medium — The rewrite-until-event loop with a 300 s budget passes on a dropped first event or a growing debounce, and a genuine failure costs ~15 minutes. Split the readiness wait from a tight single-write assertion.
- `cli/visualiser/server/src/watcher.rs:634` — minor / high — `Watching::registered` and the failed-open path are untested.
- `tasks/lint/crate_dependencies.py:259-263` — minor / high — Declaration-check branches survive mutation.
- `tests/integration/support/characterisation.py:393-410` — minor / medium — No detection of orphaned or colliding goldens. Add a collection-time uniqueness and ownership check.
- `tests/integration/characterisation/test_summary_tracking.py:40-44` — suggestion / medium — The with/without-vcs-binary goldens are byte-identical. Use a stub `ACCELERATOR_VCS_BIN` that answers wrongly, so the test guards "never dispatches".

**General Findings**:
- minor — No test or pup rule pins the removed `launcher → store` and `collaboration → vcs` edges.

### Correctness

**Summary**: Mostly faithful. The lock move, the validation-pipeline move and the m0001/m0008 rewiring are line-for-line equivalent. The derived tracker validator keeps the fault order and message text, and the `LockError` translation keeps the display strings. The risks sit on edges the old design covered by accident: the tracking deadline, a lexical-only symlink fold, and blocking registration outliving graceful shutdown. There are also two small lint gaps.

**Strengths**:
- `store::lock` is a verbatim move, and `holder_pid` now shares the sentinel readers.
- m0008's value-change guard is kept exactly; `FrontmatterValue` mirrors `document::Yaml` one-to-one.
- `TrackerCatalogue::validate` keeps the old stage order and messages.
- `from_lock_error` keeps every display string, and tests pin it.
- `PanicFold` fails closed, and the PATH/locator filtering is equivalent.
- The lint reads package names, treats build and target deps as normal, and excludes only dev deps.

**Comments**:
- `cli/launcher/src/main.rs:283` — major / medium — In-process tracking lost its 2 s deadline.
- `cli/config-adapters/src/command_runner.rs:82-87` — minor / medium — The symlink fold only catches a base whose written path lexically starts with a canonical root. macOS `TMPDIR` (`/var/folders` → `/private/var/folders`) can miss it. Judge each ancestor by its canonical form.
- `cli/visualiser/server/src/watcher.rs:97-108` — minor / medium — Tokio `Runtime` drop waits on the in-flight `spawn_blocking` registration after `server-info.json` and `server.pid` are removed. Use `shutdown_timeout`/`shutdown_background`, or a plain thread.
- `cli/visualiser/server/src/watcher.rs:109-116` — minor / low — Changes between index build and registration are never reconciled, and clients can now see that window. Rescan, or broadcast a resync when `registered` flips.
- `tasks/lint/crate_dependencies.py:251` — minor / medium — Only `domains[0]`'s kind and downstreams are read.
- `tasks/lint/crate_dependencies.py:316` — suggestion / low — Edges are attributed by package name only. Also require a path dependency.

**General Findings**:
- suggestion — E2E health-port allocation is bind-then-release, and `start-server.mjs` has no `'error'` handler.
- minor — The global panic hook replaces the default for every launcher panic.

### Standards

**Summary**: Follows conventions closely. All 42 manifests carry `[package.metadata.accelerator]` in the same place, pinned by `_EXPECTED_DECLARATIONS`. The lint follows `lint:<guard>:check` and is wired into both aggregates, and every "fourteen-point" reference agrees. The issues are minor: ADR coupling in the lint, mixed `Repo`/`Repository` naming, a lane name in the `build:cli:<token>:dev` slot, and two explanatory comments.

**Strengths**:
- The declarations are placed consistently, and `context`, `kind` and `downstreams` appear only where they should.
- The checklist count agrees everywhere and is pinned by `test_the_checklist_has_fourteen_points`.
- The lint task is named, registered and wired consistently, and the README says "six Python guards".
- `workspace_packages()` lives in `tasks/shared/`.
- The `build:cli:<token>:dev` leaves follow `build:<component>:<variant>` and are documented in the README.

**Comments**:
- `tasks/lint/crate_dependencies.py:1` — minor / medium — Docstrings, error text and the mise description tie the lint to ADR-0069 and rule numbers.
- `cli/vcs/src/lib.rs:149` — minor / low — New `Repository*` ports sit beside the crate's existing `Repo*` names. Settle on one spelling within `vcs`.
- `mise.toml:254-256` — suggestion / low — `build:cli:characterisation:dev` puts a lane name in the `<token>` slot, and `build:cli:dev` is now launcher-only.
- `tasks/shared/dev_builds.py:10-19` — suggestion / medium — Two explanatory comments that names such as `_BUILT_BY_SERVER_DEV_TASK` and `_BUILT_WITH_TEST_LOOPBACK` could replace.

**General Findings**: none.

### Compatibility

**Summary**: Removing `accelerator vcs tracking` is safe: it has no callers in shipped surfaces, and sub-binaries are pinned to the launcher's version tag. All crates with changed public APIs are `publish = false`, and their baselines were updated. The main gap is the stale Unreleased CHANGELOG entry. Smaller points: the tracking check has no deadline, and the visualiser answers before its watches register.

**Strengths**:
- No remaining `vcs tracking` callers, and two goldens pin the unknown-subcommand failure.
- Version-pinned sub-binary fetch rules out skew, and the cross-binary wire protocol is gone.
- `kernel::TEMP_PREFIX` keeps `.tmp-`, and `store::lock` keeps the on-disk names.
- cargo-public-api baselines were updated in step, and `consent-adapters` was dropped from `_PINNED_CRATES`.
- The SessionStart envelope shape is unchanged, and docs-site `configuration.md` was updated.
- The MPL-2.0 obligation for the launcher is covered by the existing exception and the notices file.
- The docs-site security bumps change only the lockfile.

**Comments**:
- `cli/launcher/src/main.rs:283` — minor / low — The SessionStart tracking check no longer has a deadline.
- `cli/visualiser/server/src/watcher.rs:95-97` — minor / medium — The visualiser is reachable before its watches register, and clients get no readiness signal. Expose readiness via SSE or `/api/health`.

**General Findings**:
- minor — The Unreleased CHANGELOG entry (around lines 168–174) still describes the removed sub-binary tracking check and its "skipped" note. Edit it in place.

### Performance

**Summary**: In-process tracking is a clear SessionStart win: no child process, lazy, no jj snapshot, and medians down 4–5×. The main risk is the hottest path. The bootstrap minisign-verifies the whole launcher on every hook invocation, the launcher doubled, warm dispatch is not measured before merge, and the only gate is 10×. The summary task runs the launcher directly, so it never times this cost.

**Strengths**:
- Dropping the child removes fork, exec and polling; the median fell from ~32 ms to ~6–9 ms.
- Tracking is only asked when `config.local.md` exists.
- `jj_is_tracked` reads the recorded tree with no snapshot.
- `PanicFold` costs nothing on the success path.
- `spawn_blocking` registration halved the visualiser lib suite (137.6 s → 63.5 s).
- `cargo metadata --no-deps` keeps both lints cheap.
- Grouped dev builds compile in one parallel pass.

**Comments**:
- `cli/launcher/Cargo.toml:36` — major / high — Doubling the launcher doubles the per-invocation signature-verification cost on every PreToolUse hook.
- `tasks/measure.py:3456-3466` — major / high — Summary-latency figures bypass the bootstrap, and warm dispatch is gated only after merge at 10×.
- `cli/vcs-adapters/src/tracking.rs:56-58` — minor / medium — In-process tracking work grows with index size, has no deadline, and decodes the index twice in colocated repos. It was benchmarked on a 4.6k-entry index. Open once per root and re-measure on 100k+ entries.
- `cli/visualiser/server/src/watcher.rs:109-116` — minor / medium — The event channel is not drained until every watch registers, so early events overflow the 1024-slot buffer. Drain immediately and keep the watcher alive another way.
- `mise.toml:435` — suggestion / low — Several single-binary dev builds per lane may queue on the target lock and rebuild gix/jj-lib under different feature sets. Verify with `cargo build -v`.

**General Findings**:
- suggestion — Before and after figures were taken at very different host loads, as point medians with no CI. Reuse the bootstrap-interval machinery.

### Safety

**Summary**: Holds up. The lock move keeps reclaim, liveness and the ceiling; m0008 still refuses a value-changing re-render; the `MigrationContext` defaults refuse; and the runner guard is stricter. The main new risk is that the in-process tracking check has no time bound inside SessionStart. Smaller: the panic fold silently depends on `panic = "unwind"`, and the process-wide panic hook can silence uncaught panics.

**Strengths**:
- `store::lock` moved without semantic change, and every consumer still maps `Timeout` to its own error.
- `PanicFold` fails closed, and credential roots now get the fold too.
- The new `MigrationContext` methods refuse by default, and m0008 still compares before and after.
- The `BashCommandRunner` guard is only stricter.
- The harness refuses `UPDATE_GOLDEN` in CI and fails on a missing golden.
- Watch registration off the serve path, with failures still surfaced.
- Tracked `config.local.md` is now warned about offline.

**Comments**:
- `cli/launcher/src/main.rs:283` — major / medium — The in-process tracking check has no time limit inside the SessionStart hook. Use a `recv_timeout` thread or a `hooks.json` timeout, and test it.
- `cli/vcs-adapters/src/panic_fold.rs:5-7` — minor / medium — The panic fold silently depends on `panic = "unwind"`, and nothing checks it. Pin it in `[profile.release]` and add a check.
- `cli/launcher/src/main.rs:442-446` — minor / medium — The process-wide panic hook can make uncaught launcher panics print nothing.

**General Findings**:
- suggestion — m0008's recovery advice does not cover baselines outside version control (the behaviour predates this PR). Snapshot the baselines, or name their paths in the message.
