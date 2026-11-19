---
type: "pr-review"
id: "247-review-1"
title: "[0365] Build the VCS fixture matrix once per test:unit:cli run"
date: "2026-11-19T13:40:02+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:247"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "security", "portability", "usability"]
review_number: 1
pr_number: 247
tags: ["build-system", "testing", "rust", "vcs", "nextest", "mise"]
last_updated: "2026-11-19T13:40:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #247 - [0365] Build the VCS fixture matrix once per test:unit:cli run

**Verdict:** COMMENT

Every lens rates this a careful, well-tested change. The design is sound: collaborators are injected, a crate-private token forces private builds through the workbench, and contract files pin the Rust/Python boundary. Deletion is tightly bounded, the run lock and sweep are free of races, and messages are actionable. No lens found a blocking defect. The findings cluster in four places: a few edges of the session's failure and interrupt handling; two parts of the cross-language protocol that are not pinned (the builder's flags, and the "same build" check); readiness for the status-log family; and the matrix being read-only only by convention.

### Cross-Cutting Themes

- **The builder's flags are outside the cross-language contract** (flagged by: code-quality, architecture, test-coverage). Python builds `--{family.value}`, and Rust hand-writes `Family::flag()`. Each side's tests assert `--matrix` on their own, so drift shows up only when the build step runs. Fix: derive the flag from the name, or add the flags to `contract/variables.json`.
- **The "nextest compiled nothing" check scrapes cargo's progress text** (flagged by: architecture, portability). With `CARGO_TERM_COLOR=always`, ANSI escapes come before `Compiling`, so the check passes without noticing a recompile; quiet settings would suppress the text entirely. Fix: strip ANSI escapes, or better, check a structural signal such as `UnitListing.rebuilt`.
- **Readiness for the status-log family** (flagged by: architecture, code-quality). The consumer map is keyed by binary, so one binary can consume only one family. `Manifest` has a generic name but fits only the matrix. `SharedFamily` lives in `workbench.rs`, which creates a module cycle. Adding a family touches about ten hand-maintained places, and the Python ones fail only at run time.
- **The shared matrix is read-only by convention only** (flagged by: usability, safety). Nothing in the `matrix()` API says so. A write is invisible under a plain `cargo nextest run`, and under `mise run` it is caught only by the digest after every test has finished. Fix: document the contract on `matrix()`, and consider removing write permission from the tree once it is built.
- **Edges of the session's failure and interrupt handling** (flagged by: correctness, code-quality). A body exception whose message is empty is swallowed. An interrupt that invoke absorbs while nextest still exits 0 is dropped. Test failures are re-typed as `SharedFixturesError`. Any `BaseException` is recast as `KeyboardInterrupt`.

### Tradeoff Analysis

- **Fail-fast lane vs partial runs**: Safety notes that a broken builder now stops all ~7,770 cli tests, not just 19. Usability credits the `_RUN_DIRECTLY` hint that keeps diagnosis possible. Keep fail-fast for now, as the PR argues; revisit if more families arrive.
- **Read-only shared tree vs simplicity**: Removing write permission would make a writing test fail at the point of the write rather than after the run. It costs a permission walk after the build, and the cleanup already has the matching make-writable walk. Recommended.
- **Pinning mise vs the repo's tool-provisioning model**: Portability wants a `min_version` (and a `version:` on `jdx/mise-action`), because `--filter` depends on how mise behaves. mise is the one tool the repo cannot provision for itself, so a `min_version` floor is the lightest fix.

### Strengths

- ✅ The crate-private `PrivateBuild` token makes the compiler stop any private build of a shared family that bypasses the logging workbench. The one gap it cannot close (`Workbench::with`) is stated and guarded by `test_fixture_builds.py`.
- ✅ The contract files (`contract/fixture-log.jsonl`, `contract/variables.json`) are asserted by both the Rust and Python suites, so neither side can rename an event field or a variable alone.
- ✅ The run lock is flocked before it is hard-linked into place, the directory is made after the lock and removed before it, and liveness checks fail safe (a lock in an UNKNOWN state is kept). No race sweeps a live run.
- ✅ Every deletion is bounded to names matching the run-id pattern, inside a root that is refused if it is a symlink, foreign-owned, writable by others, or inside a checkout. The make-writable walk never follows a symlink and never changes a file's mode.
- ✅ `--filter` is quoted correctly from mise through the shell to invoke, and the forwarding was checked under dash. `--filter ''` is refused rather than meaning "everything".
- ✅ Adoption fails closed, naming the variable and the path. `replace_env=True` stops a stale hand-over in the developer's shell from reaching a run.
- ✅ `tree_digest` uses lstat and never opens a FIFO, socket or device, so cleanup cannot hang. Mode counts towards the digest and timestamps do not.
- ✅ Tests run real SIGKILLed child processes and real flock contention, with injected seams for every race. The listing capture fails when the nextest pin moves.
- ✅ Moving a test onto the shared matrix is two lines, and failure messages say exactly what to rerun or edit (`consumers.py`, `--filter 'binary_id(...)'`, `TMPDIR`).

### General Findings

- 🔵 **Architecture**: Adding the status-log family touches about ten hand-maintained places in two languages. The Rust `match` arms are checked by the compiler, but the Python per-property dicts fail only at run time with a `KeyError`. Consider one attribute table per family in each language.
- 🔵 **Test Coverage**: No `vcs-test-support` test builds the real matrix with `build_for_sharing` and then adopts it. A manifest mismatch would surface as 19 consumer failures rather than one focused one.
- 🔵 **Usability**: The README's "Shared VCS fixtures" section needs a short how-to for test authors: the two-line snippet, "write scratch files under `workbench.path()`", and "add a new binary to `CONSUMERS`".
- 🔵 **Portability**: The new advice to set `TMPDIR` conflicts with the `fixture_tempdir_path` mask, which matches only `/tmp` and `/var/folders`. That matters once the status-log goldens move under the shared root.
- 🔵 **Portability**: Hermetic paths are now about 150 characters on macOS. That is within `PATH_MAX`, but close to the 104-byte Unix-socket limit if a fixture ever creates a socket.
- 🔵 **Safety**: A broken builder stopping the whole lane is deliberate and documented. No change is needed now.

### Additional Findings

- 🔵 `tasks/shared/shared_fixtures/session.py:98` — An interrupt that invoke swallows is lost if nextest exits 0; any `BaseException` is recast as `KeyboardInterrupt` (correctness, code-quality)
- 🔵 `cli/vcs-test-support/src/manifest.rs:9` — `Manifest` has a generic name but fits only the matrix, and forms a cycle with `fixtures.rs` (architecture)
- 🔵 `cli/vcs-test-support/src/workbench.rs:84` — Three unrelated public types are named `Fixture`/`Fixtures` (code-quality)
- 🔵 `tasks/test/cli.py:63` — The `_shared_root` alias exists only as a monkeypatch seam (code-quality)
- 🔵 `tasks/shared/shared_fixtures/consumers.py:13` — The consumer map is keyed by binary, so one binary can consume only one family (code-quality)
- 🔵 `tasks/shared/shared_fixtures/session.py:146` — The shared tree stays writable; a write is detected only after every test finishes (safety)
- 🔵 `mise.toml:337` — `--filter` forwarding depends on a mise version that is not pinned (portability)
- 🔵 `cli/vcs-test-support/src/fixtures.rs:157` — The trait `adopt` delegates to an inherent `adopt` of the same name (code-quality)
- 🔵 `tasks/shared/shared_fixtures/log.py:51` — `None` stands for "the private sets" on the Python side (code-quality)
- 🔵 `tasks/shared/selection.py:31` — The "see --help for its syntax" message points at help that has a single example (usability)
- 🔵 `cli/vcs-test-support/src/shared_build.rs:120` — No test covers a build that succeeds but cannot log its shared-build event (test-coverage)
- 🔵 `cli/vcs-test-support/src/fixture_log.rs:76` — Concurrent log appends rely on `O_APPEND` behaving as it does on a local filesystem (portability)
- 🔵 `tasks/shared/shared_fixtures/runs.py:227` — Removing an orphaned run assumes every process sharing the root uses the same lock protocol (safety)
- 🔵 `tasks/shared/shared_fixtures/runs.py:205` — The run lock's flock result is ignored, and nothing checks that flock actually excludes other processes (correctness, safety)
- 🔵 `tasks/shared/shared_fixtures/log.py:120` — The summary prints a hard-coded `0 copies` and an unexplained "private sets" line (usability, code-quality)
- 🔵 `tasks/test/cli.py:334` — The builder failure message is empty when the builder dies without writing to stderr (usability)
- 🔵 `cli/vcs-test-support/src/workbench.rs:109` — `Workbench::with` is a discoverable bypass next to `for_this_test()` (usability)
- 🔵 `cli/vcs-test-support/src/fixture_log.rs:124` — `Default` for `TestName` reads the process environment (code-quality)
- 🔵 `tasks/shared/shared_fixtures/session.py:176` — `_clean_up` mixes timing, four broad `except`s and printing (code-quality)
- 🔵 `tasks/test/cli.py:65` — The set of hand-over variables has two sources of truth (architecture)
- 🔵 `tasks/shared/shared_fixtures/log.py:393` — The binary is parsed back out of a combined test-name string (architecture)
- 🔵 `cli/vcs-test-support/tests/workbench.rs:342` — A test changes the process environment and never restores it (test-coverage)
- 🔵 `tasks/shared/shared_fixtures/log.py:159` — `splitlines()` splits on Unicode line boundaries, not the `\n` the writer uses (correctness)
- 🔵 `tasks/shared/shared_fixtures/runs.py:307` — The hold point writes into a directory named by the environment without the root's ownership and symlink checks (security)

---
*Review generated by /review-pr*

## Inline Comments

### `tasks/README.md:102-107` — Roll-ups silently ignore --filter and run the whole suite
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

`mise run test:unit --filter x` (and likewise `test`, `test:integration` and `test:e2e`) accepts the flag and silently runs the whole roll-up unfiltered. The only warning is this paragraph, in a section written for task authors rather than for the developer typing the command.

**Impact**: A developer who reaches for the familiar roll-up gets a full run lasting several minutes instead of the few tests they asked for, with nothing telling them the flag was dropped. Elsewhere this PR carefully refuses `--filter ''` for exactly this reason.

**Suggestion**: Until Phase 9 settles the design, make the roll-ups refuse words they don't declare. A task with both `usage` and `run` rejects them, so a trivial `run` plus an empty `usage` on each roll-up should make this exit 1. Failing that, say in each roll-up's `description` that it takes no options.

---

### `tasks/shared/shared_fixtures/session.py:169-174` — The body's failure can be swallowed, or re-typed
**Severity**: minor | **Confidence**: high | **Lens**: correctness, code-quality

🔵 **Correctness**

`_finish` decides whether to raise only from `outcome.failure()`, which drops `body_failure` when `str(failure)` is empty (`[self.body_failure] if self.body_failure`). If the body raises an exception whose `str()` is `''` (a bare `AssertionError()`, `Exception()` or `Exit(code=1)`) and judging finds no problems, `_finish` returns normally. The `@contextmanager` then treats the exception as handled, and the task carries on as though the tests had passed, through the coverage report and floor. Raise whenever `failure is not None`, falling back to `repr(failure)` when the message is empty.

🔵 **Code Quality**

Even when the failure is raised, an ordinary test failure (`Exit("nextest: cli tests failed")`) comes out as `SharedFixturesError`, so its type no longer says which category it belongs to, and the original `Exit` code is lost. When there are no cleanup problems, consider re-raising the original failure unchanged.

---

### `tasks/test/cli.py:312-320` — The builder's flags are duplicated by convention, outside the contract
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality, test-coverage

🔵 **Architecture / Code Quality / Test Coverage**

The task builds the `shared-fixtures` command line as `f"--{family.value}"`, while Rust's `Family::flag()` (`cli/vcs-test-support/src/family.rs:35-41`) is a separately hand-written `"--matrix"` that isn't derived from `name()`. `contract/variables.json` and `contract/fixture-log.jsonl` pin the variable names and the log format, but nothing pins the builder's arguments, and each side's tests assert `--matrix` on their own.

**Impact**: A family whose flag doesn't match its name, or a renamed flag, would pass both unit suites. It would only fail when the build step ran and the builder rejected an unknown argument, stopping the whole lane.

**Suggestion**: Derive `flag()` from `name()` in Rust, and/or add each family's flag to `contract/variables.json` and assert it from both suites, as is already done for the hand-over variables.

---

### `tasks/test/cli.py:384-396` — The "nextest compiled nothing" check scrapes coloured cargo output
**Severity**: minor | **Confidence**: high | **Lens**: architecture, portability

🔵 **Portability**

`_COMPILING = re.compile(r"^\s*Compiling ", re.MULTILINE)` never matches with `CARGO_TERM_COLOR=always`, a common developer setting, because cargo then writes `\x1b[1m\x1b[32m   Compiling\x1b[0m foo`. Escape codes come both before and after the word.

🔵 **Architecture**

More generally, a correctness guarantee (the builder and the tests come from one build) rests on cargo's human-readable progress output, which a quiet setting or a change of wording would suppress. The check fails open.

**Suggestion**: At least strip ANSI escapes before matching, and add a test with a coloured line. Better: check a structural signal, such as re-running the unit listing and asserting `UnitListing.rebuilt == 0`, which the coverage sweep already does.

---

### `cli/vcs-test-support/src/fixtures.rs:161-166` — The real matrix's private-build fallback no longer runs in CI
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

Every matrix consumer now adopts the shared tree, and the log check fails a run on any private build, so `Matrix::build_privately` never runs in CI. The workbench test covers the fallback only with the `Marked` stand-in. This is the path `_RUN_DIRECTLY` sends developers down while they diagnose a failure, and it was checked by hand only.

**Suggestion**: Add one `vcs-test-support` test that opens a workbench with a test-local `FixtureLog`, calls `matrix()`, and asserts the expected keys. The `Workbench::with` guard would need to allow a second file for it.

---

### `cli/vcs-test-support/tests/shared_build.rs:93-117` — The "refused before anything is written" test uses a tree that already exists
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The test points the build at `CARGO_MANIFEST_DIR/src`, which already exists. `build_for_sharing` runs `create_dir_all(base)` before `build_in`'s ancestor check, because that check canonicalises `base`. So for a new tree inside a checkout, directories are created before the refusal. The test cannot catch what its name promises.

**Suggestion**: Use a tree that does not exist yet inside a scratch checkout (`<tempdir>/.git` plus `<tempdir>/run/matrix`) and assert that `<tempdir>/run` is absent afterwards. Otherwise, rename the test to match what it actually checks.

---

### `cli/vcs-test-support/src/workbench.rs:193-198` — Nothing at the API says the shared matrix is read-only
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

`Workbench::matrix()` is the entry point every test author uses, but neither it nor `Matrix` (whose `base` is public) says the tree is shared and must not be written to, or that `workbench.path()` is where scratch files belong. This is the trap `scrub.rs` had to be changed to avoid. A write succeeds under a plain `cargo nextest run` and only fails the whole run later, under `mise run`.

**Suggestion**: Add a one-line summary to `matrix()` and `shared()`, e.g. `/// The run's shared matrix: treat it as read-only; write scratch files under [`Self::path`].`

---

### `cli/vcs-test-support/src/workbench.rs:34-54` — SharedFamily lives in its concrete consumer, creating a fixtures <-> workbench cycle
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

`SharedFamily` and `PrivateBuild` are defined in `workbench.rs`, which imports `crate::fixtures::Matrix`. Meanwhile `fixtures.rs` imports them back from `crate::workbench` to implement the trait. The status-log set will be the next implementor and will join the cycle.

**Suggestion**: Move `SharedFamily` and `PrivateBuild` into `family.rs`, so each family module depends only on `family`, and `workbench` depends on `family`.

---

### `tasks/shared/shared_fixtures/runs.py:198` — No test covers a run whose liveness is unknown being kept
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`live_run_ids` counts a run as live when its lock is `LIVE` or `UNKNOWN`, but `TestLiveRunIds` covers only a held lock and an abandoned one. If `UNKNOWN` were dropped from the set, every test would still pass, and log pruning could delete the log of a live run whose lock merely can't be read.

**Suggestion**: Add a case with an injected `flock` failing with `ENOLCK` over an abandoned run, asserting its id is in `live_run_ids()`, and a `chmod 000` variant marked `@no_permission_stops_root`.

---

### `tasks/shared/shared_fixtures/runs.py:354-360` — The inside-a-checkout refusal asks for a "writable" TMPDIR, and lacks the common prefix
**Severity**: minor | **Confidence**: high | **Lens**: usability, code-quality

🔵 **Usability**

"…lies inside a checkout; set TMPDIR to a writable temp directory" names the wrong constraint. The developer's TMPDIR is almost certainly writable already; what it must be is outside any git or jj checkout. Name the real constraint, and the ancestor that matched.

🔵 **Code Quality**

Some `SharedFixturesError` messages start with `shared VCS fixtures:` and others, including this one, don't. When several are joined, the output is uneven. Add the prefix once, where errors are presented.

---

## Per-Lens Results

### Architecture

**Summary**: The overall shape is sound: a Python session owns the run lifecycle with its collaborators injected, a thin Rust bin delegates to a testable library, tests acquire shared fixtures through one Workbench seam, and committed contract files pin the Rust/Python boundary. The concerns are about how easily the design will take the status-log family: the shared-family abstraction lives in its concrete consumer, which creates module cycles; `Manifest` is generically named but fits only the matrix; and the builder flags, the test-name structure and the hand-over variable set are duplicated by hand.

**Strengths**: the `PrivateBuild` token; the injected session collaborators; the contract files; fail-closed adoption; the builder as a `[[bin]]` under `tests/fixtures/` needing no extra build configuration; the consumer table checked against the log in both directions; deliberate staging of the zero-spawn retirement.

**Comments**:
- `cli/vcs-test-support/src/workbench.rs:34-54` (minor, high): SharedFamily lives in its concrete consumer, creating a cycle.
- `cli/vcs-test-support/src/manifest.rs:9-16` (minor, high): generically named Manifest is matrix-specific and forms a cycle with fixtures.rs.
- `tasks/test/cli.py:312-319` (minor, high): the builder's argument protocol is duplicated by convention.
- `tasks/test/cli.py:65-71` (suggestion, medium): the set of hand-over variables has two sources of truth.
- `tasks/test/cli.py:384-396` (minor, medium): the same-build invariant is checked by scraping cargo's progress text.
- `tasks/shared/shared_fixtures/log.py:393-394` (suggestion, medium): the binary is parsed back out of a combined test-name string.

**General Findings**:
- (minor) Adding the status-log family touches about ten hand-maintained places in two languages.

### Code Quality

**Summary**: The code is well built, with injected collaborators, errors that can be acted on, and two-line consumer changes. The remaining issues are about maintainability: three things named Fixture/Fixtures, inconsistent error types and prefixes, and a few places that duplicate logic or stand in for domain concepts with primitive values.

**Strengths**: the `PrivateBuild` token; the injection seams (`Handover::from_variables`, `build_with`, the session's collaborators); the contract files; actionable error messages; extracting `try_exclusive_flock` and `Manifest`; `scrub` writing into its own workbench.

**Comments**:
- `cli/vcs-test-support/src/workbench.rs:84-89` (minor, high): three unrelated public types named Fixture/Fixtures.
- `cli/vcs-test-support/src/fixtures.rs:157-159` (minor, medium): the trait's `adopt` delegates to an inherent `adopt` of the same name.
- `cli/vcs-test-support/src/fixture_log.rs:124-128` (suggestion, medium): `Default` for `TestName` reads the process environment.
- `tasks/test/cli.py:63` (minor, high): a module alias that exists only as a monkeypatch seam.
- `tasks/test/cli.py:315-320` (minor, medium): the builder flag's spelling is duplicated outside the contract.
- `tasks/shared/selection.py:38-39` (minor, medium): `_quoted` re-implements `shlex.quote`. *Not carried into the summary: the always-quoted form is deliberate, because `shlex.quote` leaves a safe word like `x` bare and the messages are meant to always show the filter in quotes.*
- `tasks/shared/shared_fixtures/session.py:98-106` (minor, medium): every non-Exception `BaseException` is recast as `KeyboardInterrupt`.
- `tasks/shared/shared_fixtures/session.py:169-174` (minor, medium): failures from the test phase are re-raised as `SharedFixturesError`.
- `tasks/shared/shared_fixtures/session.py:176-217` (suggestion, medium): `_clean_up` mixes timing, four broad `except`s and printing.
- `tasks/shared/shared_fixtures/log.py:51-57` (minor, medium): `None` stands for "the private sets".
- `tasks/shared/shared_fixtures/log.py:137-141` (suggestion, low): a hard-coded zero `copies` count.
- `tasks/shared/shared_fixtures/consumers.py:13-24` (minor, medium): the consumer map is keyed by binary.
- `tasks/shared/shared_fixtures/runs.py:354-360` (minor, medium): the `shared VCS fixtures:` prefix appears on some messages and not others.

**General Findings**: none.

### Test Coverage

**Summary**: Coverage is strong and in proportion to the risk, with focused behavioural tests for every new piece, exact-message assertions, real SIGKILL and flock checks, and contract files tested from both languages. The main gap is that the real matrix's private-build fallback no longer runs in CI. The smaller gaps are a test that cannot catch what its name promises, and a few untested branches.

**Strengths**: contract files asserted from both languages; the `Marked` stand-in family; liveness checked with real processes; every session exit path covered; edge cases for `tree_digest`; the listing capture tied to the nextest pin; the `Workbench::with` guard; end-to-end tests of the bin.

**Comments**:
- `cli/vcs-test-support/src/fixtures.rs:161-166` (minor, high): the real matrix's private-build fallback no longer runs in CI.
- `cli/vcs-test-support/tests/shared_build.rs:93-117` (minor, high): the "refused before anything is written" test uses a tree that already exists.
- `cli/vcs-test-support/src/shared_build.rs:120-128` (minor, medium): no test covers a build that succeeds but cannot log its event.
- `tasks/shared/shared_fixtures/runs.py:198` (minor, high): no test covers a run of unknown liveness being treated as live.
- `cli/vcs-test-support/src/family.rs:35-41` (suggestion, medium): the builder's command-line flags are outside the contract.
- `cli/vcs-test-support/tests/workbench.rs:342-353` (suggestion, low): a test changes the process environment and never restores it.

**General Findings**:
- (minor) No test covers the real round trip of `build_for_sharing` followed by `adopt` outside a full run.

### Correctness

**Summary**: The run lock and sweep are carefully ordered, and no race was found that could sweep a live run; the digest, the log judging and the Rust side are sound. The remaining issues are at the edges of the session lifecycle: a body exception with an empty message can be swallowed, an interrupt that invoke absorbs can be dropped, the flock result is ignored, and the reader splits lines differently from the writer.

**Strengths**: the lock is flocked before it is linked; teardown happens in the right order; leftover temporary locks are checked with flock as well as by age; interrupts are recognised through `interruption.arrived`; `tree_digest` uses lstat; scoped threads are joined explicitly; the manifest is written last.

**Comments**:
- `tasks/shared/shared_fixtures/session.py:169-171` (minor, high): a body exception with an empty message is silently swallowed. *Confirmed against `RunOutcome.failure()`.*
- `tasks/shared/shared_fixtures/session.py:98-106` (minor, medium): an interrupt that invoke swallows is lost when nextest still exits 0.
- `tasks/shared/shared_fixtures/runs.py:208-210` (suggestion, low): the run lock's flock result is ignored.
- `tasks/shared/shared_fixtures/log.py:159` (suggestion, low): `splitlines()` splits on Unicode line boundaries.

**General Findings**: none.

### Safety

**Summary**: Deletion is well contained, liveness checks fail safe, cleanup runs with signals deferred, and adoption fails closed. What remains is mostly about the shared tree being writable in place, plus two unchecked assumptions: that flock really excludes other processes, and that every process sharing the root uses the same lock protocol.

**Strengths**: bounded deletion; liveness checks that fail safe; create and close ordering that keeps the orphan rule sound; a symlink-safe make-writable walk; deferred signals during cleanup; fail-closed adoption and `replace_env`; fault switches that fail safe.

**Comments**:
- `tasks/shared/shared_fixtures/session.py:146-151` (minor, medium): the shared tree stays writable, and a mutation is detected only after the run.
- `tasks/shared/shared_fixtures/runs.py:227-236` (minor, low): removing an orphaned run depends on every process sharing the root using the same lock protocol.
- `tasks/shared/shared_fixtures/runs.py:205-210` (suggestion, low): nothing checks that flock excludes other processes.

**General Findings**:
- (suggestion) A broken builder now blocks the whole cli lane. This is deliberate and documented; no change is needed.

### Security

**Summary**: Careful for build tooling. The private root is checked against symlinks, foreign owners and loose modes; the lock is created with `O_EXCL` and mode 0600 inside it; the sweep never follows symlinks; and `--filter` is quoted correctly end to end. The one gap is the developer-only hold directory.

**Strengths**: the lstat checks on the private root, including after a lost `mkdir` race; the lock created with `O_EXCL`, flocked and then linked; symlink-safe removal; the FIFO-safe digest; safe quoting of the filter; `replace_env` stripping inherited hand-over variables; fault switches that fail closed.

**Comments**:
- `tasks/shared/shared_fixtures/runs.py:307-317` (suggestion, low): the hold point writes into a directory named by the environment, without the root's ownership and symlink checks.

**General Findings**: none.

### Portability

**Summary**: Careful handling of macOS's `/private/var`, a digest that never opens special files, clear errors that name TMPDIR, and POSIX forwarding checked under dash. The remaining risks depend on each developer's environment: coloured cargo output, an unpinned mise version, and the fixture log assuming appends never interleave, which only holds on local filesystems.

**Strengths**: the temp directory is resolved in Python and canonicalised in Rust; `tree_digest` skips special files; clear TMPDIR remedies; lock descriptors are not inherited by child processes; run ids use UTC; the forwarding is POSIX; permission tests skip under root.

**Comments**:
- `tasks/test/cli.py:391` (minor, medium): the recompile check misses coloured cargo output.
- `mise.toml:337-340` (minor, medium): `--filter` forwarding depends on an unpinned mise version.
- `cli/vcs-test-support/src/fixture_log.rs:76-93` (minor, low): concurrent log appends rely on how `O_APPEND` behaves on a local filesystem.

**General Findings**:
- (suggestion) The advice to set TMPDIR conflicts with the temp-path mask.
- (suggestion) The shared-root layout makes every hermetic path longer.

### Usability

**Summary**: The developer-facing surface is well designed overall: two-line consumer changes, messages that can be acted on, and a `--filter` that checks its input up front. The friction points are roll-ups silently ignoring `--filter`, the matrix being read-only only by convention, filter-syntax help with a single example, and a few misleading messages and summary lines.

**Strengths**: the two-line migration for a consumer; the bypass that no longer compiles; changed-tree failures that name the reruns to try; messages about the consumer list that say exactly what to edit; the `_RUN_DIRECTLY` hint; `--filter ''` refused; help that includes an example; root refusals that give the remedy.

**Comments**:
- `tasks/README.md:102-107` (major, high): roll-ups silently ignore `--filter` and run the whole suite.
- `cli/vcs-test-support/src/workbench.rs:193-198` (minor, high): nothing at the API says the shared matrix is read-only.
- `tasks/shared/selection.py:31-35` (minor, medium): "see --help for its syntax" points at help with a single example.
- `tasks/shared/shared_fixtures/runs.py:357-360` (minor, high): the inside-a-checkout refusal asks for a "writable" TMPDIR.
- `tasks/shared/shared_fixtures/log.py:120-141` (suggestion, medium): the summary shows an always-zero `copies` count and an unexplained "private sets" line.
- `tasks/test/cli.py:334-337` (suggestion, low): the builder failure message can be empty.
- `cli/vcs-test-support/src/workbench.rs:109-114` (suggestion, low): `Workbench::with` is a discoverable bypass.

**General Findings**:
- (suggestion) The README's "Shared VCS fixtures" section lacks a minimal how-to for test authors.
