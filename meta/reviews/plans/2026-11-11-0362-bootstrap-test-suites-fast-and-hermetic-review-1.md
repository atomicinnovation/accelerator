---
type: "plan-review"
id: "2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic-review-1"
title: "Plan Review: Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan"
date: "2026-11-11T21:43:50+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-11-0362-bootstrap-test-suites-fast-and-hermetic"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "performance", "standards", "portability"]
review_number: 1
review_pass: 2
tags: ["build-system", "performance", "testing", "bootstrap", "minisign", "hooks", "cargo-profile"]
last_updated: "2026-11-12T10:15:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Make the Bootstrap Test Suites Fast and Hermetic Implementation Plan

**Verdict:** COMMENT

Plan is acceptable but could be improved — see the major finding below.

The plan is sound across all eight lenses. Its factual claims about the
bootstrap, launcher, `vcs-cli` and the installation helpers hold almost
everywhere they were checked, the profile override covers the whole hot
path, and no production trust-chain control is weakened. There are no
critical findings and one major: the mutation checks that stand in for the
red step only ever turn the first assertion red, so most assertions in the
two new tests are never seen failing. The rest are minor: failure
diagnosability, three inaccurate statements in the plan's prose, an
overstated claim about release-time coverage, measurement details, and
naming and duplication in the new test module.

### Cross-Cutting Themes

- **The mutation checks prove less than the plan claims** (flagged by:
  test coverage, correctness, portability) — each of the four mutations
  trips only the first failing assertion, and mutation check 2
  (`cwd=REPO_ROOT`) fails only inside a jj workspace or git worktree. In a
  plain git clone, including CI, the mutated test still passes.
- **Assertion order hides the bootstrap's stderr** (flagged by: code
  quality, test coverage, correctness) — the smoke test compares bytes
  before it checks the exit status, and the refusal test's control
  discards its result, so the likely failures surface as a bare
  `FileNotFoundError`.
- **What replaces the published-launcher check is overstated** (flagged
  by: security, test coverage) — `test_launcher_reverify_failure_preserves_draft`
  mocks the re-verification it is said to cover. After PR 2 no
  pull-request test executes the committed vendored shims, and release
  time runs only the darwin-arm64 one, after the tag is pushed.
- **A third copy of the installation fixtures, with a third vocabulary**
  (flagged by: architecture, code quality, standards) — `alias`, `shim`
  and `install` against the entrypoint suite's `host_platform`, `shim_bin`
  and `make_harness`. `install` also fails on a second call and hides its
  parameters behind `**served`.
- **`_local_vcs_subbinary` and the task layer** (flagged by: architecture,
  code quality, standards) — the helper is named for a binary but returns
  an environment, reads `os.environ` in a hermetic test, and repeats a
  default path that `tasks/test/helpers.py` owns. The `hooks` task and
  `accelerator_env` docstrings are left describing the old test.
- **The offline run is manual** (flagged by: test coverage, standards,
  portability) — it sits under Automated Verification, needs a physically
  disconnected host, can fail for unrelated reasons (cargo, uv), and the
  work item says no separate offline run is needed.
- **The refusal test's home** (flagged by: architecture, code quality) —
  a bootstrap trust-chain test in a module named for vcs-detect
  registration, with a docstring that no longer parses or stays true.
- **The CI measurement** (flagged by: performance, portability,
  correctness) — the grep reads pytest's summary where the baseline used
  mise's `Finished in`, the pattern misses `N passed, 1 warning in`, and
  the total cannot separate the fixed fixture build from per-case cost.

### Tradeoff Analysis

- **Hermeticity vs real trust-anchor coverage**: removing the GitHub
  fetch makes the suite fast and offline, but it was the only
  pull-request execution of the committed shim and key against a genuine
  signature, on both CI lanes. The work item accepts this as a check that
  moves. Security suggests a cheap hermetic check of the host's vendored
  shim against a per-run key, which keeps most of the value without the
  network. Recommendation: either add that check, or state the narrower
  gap (Linux shims never executed; detection after the tag) as an
  accepted risk.
- **Scope vs duplication**: architecture and code quality want the shared
  fixtures and bootstrap names lifted into the support module; the plan
  lists that tidying as out of scope ahead of 0366. Recommendation: keep
  it out of scope, adopt the entrypoint suite's names so a later
  consolidation is mechanical, and record the deferral.
- **One module vs cohesion**: a sibling module for the refusal test reads
  better by name but needs the fixtures shared. Recommendation: keep one
  module, reword the docstring to what the module guards today, and say
  the placement is deliberate.

### Findings

#### Critical

None.

#### Major

- 🟡 **Test Coverage**: Mutation checks only turn the first assertion red; most assertions are never observed failing
  **Location**: Phase 2 and Phase 3: Red Step (mutation checks); Implementation Approach
  The Implementation Approach says the mutation checks show each assertion
  can fail for the reason it exists, but pytest stops at the first failing
  assertion and each of the four mutations trips only one. The
  `hookEventName`, download-log, empty-stderr, unverified-log,
  non-execution and cache-glob assertions are never seen red.

#### Minor

- 🔵 **Code Quality / Test Coverage / Correctness**: Assertion order and unguarded reads hide the bootstrap's stderr on failure
  **Location**: Phase 2 smoke test body; Phase 3 refusal test body
  The download-log and `filecmp.cmp` assertions run before
  `assert result.returncode == 0, result.stderr`, and the control's
  `detect()` result is discarded. A refusal or failed fetch surfaces as a
  bare `FileNotFoundError` without the bootstrap's diagnostic.
- 🔵 **Correctness / Standards**: Change bullet removes the `os` import that the code block still uses
  **Location**: Phase 2, Section 1: Module rewrite
  The bullet says to remove the `os`/`subprocess` imports, but
  `_local_vcs_subbinary` calls `os.environ.get`. Only `subprocess` becomes
  unused.
- 🔵 **Correctness**: The `report.rs` profile slice does change: it gains the new comment
  **Location**: Key Discoveries: "`report.rs` is unaffected"
  The slice runs from `[profile.release]` to the next `\n[`, so the new
  table's two-line comment falls inside it. The test passes only because
  the comment contains neither `panic` nor `abort`.
- 🔵 **Correctness**: The lock directory does not remain after a refusal
  **Location**: Phase 3: refusal test notes
  `ensure_launcher` calls `release_lock` before returning on a failed
  `fetch_and_verify`, so `.accelerator-lock-<alias>` is gone. The note
  misstates post-refusal state.
- 🔵 **Correctness / Test Coverage / Portability**: The `cwd=REPO_ROOT` mutation only fails in a secondary workspace
  **Location**: Phase 2: Red Step, mutation check 2
  In a plain main git checkout, `vcs detect` reports git mode with no
  boundary block, so the mutated test passes. The check is not
  reproducible by another contributor.
- 🔵 **Security**: Committed vendored shims are no longer executed on Linux anywhere
  **Location**: What We're NOT Doing; Phase 2
  The current smoke test is the only pull-request test that runs the
  committed `bin/accelerator-verify-<platform>` against the committed key,
  on both CI lanes. Release-time re-verification runs only the
  darwin-arm64 shim.
- 🔵 **Security / Test Coverage**: Cited test does not cover the re-verification the plan relies on
  **Location**: Phase 3: Manual Verification (PR 2 description note)
  `test_launcher_reverify_failure_preserves_draft` mocks
  `_reverify_via_shim`; it shows only that a failure preserves the draft.
  The accept/reject pair at `test_github.py:694-728` patch both the shim
  path and the key.
- 🔵 **Security**: Trust-anchor breakage now surfaces after the tag is pushed, not before merge
  **Location**: What We're NOT Doing
  Release-time re-verification runs after `git.push` and tag creation, so
  a bad committed key or shim is found once the marketplace ref has moved.
  It still fails closed.
- 🔵 **Test Coverage**: Pin test cannot detect the override going inert
  **Location**: Phase 1: Pin test
  Cargo only warns when a profile package spec matches no package, so a
  renamed or replaced verification crate leaves the table and the test
  green while the shim hashes unoptimised.
- 🔵 **Architecture / Code Quality / Standards**: Third hand-copied set of bootstrap fixtures, with a third vocabulary
  **Location**: Phase 2: module-local fixtures
  `alias`, `keys`, `shim`, `downloader` and `install` repeat the entrypoint
  suite's `host_platform`, `keys`, `shim_bin`, `downloader` and
  `make_harness`, and each copy feeds `make_installation`'s unread
  `bootstrap` argument.
- 🔵 **Code Quality**: `install` is a factory that fails on its second call and hides its parameters behind `**served`
  **Location**: Phase 2: `install` fixture
  It always builds at `tmp_path / "root"`, and `make_installation` calls
  `mkdir` without `exist_ok`. The untyped `**served` accepts any
  `make_installation` keyword.
- 🔵 **Architecture**: Bootstrap naming contract re-encoded as literals in the hooks module
  **Location**: Phase 2 (`_served_assets`, `_cached_launcher`); Phase 3 (`_refusal`, cache globs)
  Asset names, the cache path, `.tmp-launcher-*`, the log name and the
  refusal message are spelt out again. A missed glob fails open, since an
  empty glob still passes `assert not list(...)`.
- 🔵 **Architecture / Code Quality / Standards**: `_local_vcs_subbinary` has a misleading name, an ambient input and two owners for its default path
  **Location**: Phase 2: `_local_vcs_subbinary`; Notes
  It returns an environment overlay, reads `os.environ` in a hermetic
  test, and repeats the default that `tasks/test/helpers.py` computes. A
  contributor's exported `ACCELERATOR_VCS_BIN` changes which binary runs.
- 🔵 **Architecture / Code Quality**: Task and helper docstrings describing the old smoke test are left stale
  **Location**: Phase 2: Changes Required (omission)
  `tasks/test/integration.py:124-128`, `tasks/test/helpers.py:29-32` and
  `tasks/README.md:323` still describe the real `bin/accelerator` wrapper.
- 🔵 **Architecture**: Hooks suite ends up with two build-provisioning mechanisms
  **Location**: Key Discoveries; Performance Considerations
  The launcher and `accelerator-vcs` come from mise edges while the shim
  is built in pytest. An unbuilt tree now fails after five seconds with
  `not built: <path>` where the old test named the mise task.
- 🔵 **Code Quality / Architecture / Standards**: Module docstring is ungrammatical and becomes untrue; module name no longer fits
  **Location**: Phase 2 module docstring; Phase 3 docstring extension
  "Guards the retired vcs-detect shell harness carried that live nowhere
  else." does not parse, and Phase 3 adds a guard the retired harness
  never carried without giving the final text.
- 🔵 **Standards / Code Quality**: Cargo profile pin placed in the module that tests the `DEV_BUILDS` registry
  **Location**: Phase 1: Pin test
  Every other test in `test_dev_builds.py` exercises
  `tasks.shared.dev_builds`. The precedent for pinning a configuration
  file is a dedicated module, `test_nextest_filter.py`.
- 🔵 **Standards / Portability / Test Coverage**: Manual offline run listed under Automated Verification
  **Location**: Phase 2: Success Criteria
  It needs a person to disconnect the host, can fail offline for
  unrelated reasons on a cold cache, and the work item says no separate
  offline run is needed.
- 🔵 **Portability**: Default cwd is only outside a checkout if the temp root is
  **Location**: Phase 2: Notes (cwd)
  The cwd comes from `tempfile.TemporaryDirectory()`, which honours
  `TMPDIR`. A temp root under a jj checkout fails the smoke test with what
  looks like a detection bug.
- 🔵 **Portability**: Hard-coded `cli/target/debug` ignores `CARGO_TARGET_DIR`
  **Location**: Phase 2: `BUILT_LAUNCHER` / `BUILT_VCS`; Phase 1 measurements
  The repository assumes this path throughout, so no new class of failure,
  but the measurement `cp` can copy a stale binary, and the launcher
  ignores `ACCELERATOR_BIN` while the sub-binary honours its override.
- 🔵 **Performance / Portability**: CI duration is read with a different instrument, by a brittle grep
  **Location**: Phase 1, Section 4: CI suite speed
  The 1,266 s baseline came from mise's `Finished in` line; the plan reads
  pytest's `passed in`, which misses `N passed, 1 warning in`. The job
  placeholder hides the matrix name `Run integration tests (ubuntu-latest)`.
- 🔵 **Performance**: A single CI total cannot separate the fixed fixture build from per-case cost
  **Location**: Phase 1, Section 4: CI suite speed
  The first CI case took 154 s building the shim and launcher, about 36%
  of the 422 s budget. A miss would leave the follow-up with no evidence
  of the cause.
- 🔵 **Performance**: The override's rebuild cost is understated
  **Location**: Performance Considerations
  A changed dependency profile recompiles every dependent once, in each
  dev-derived configuration (test, llvm-cov, clippy and so on), not only
  the plain debug build.
- 🔵 **Performance**: The hooks suite's new cargo-lock wait is asserted, not measured
  **Location**: Performance Considerations; Phase 2 Manual Verification
  `_cargo_build` runs quietly, so a lock wait is invisible. The only
  check is "no longer takes tens of seconds", against a 2.7 s baseline.

#### Suggestions

- 🔵 **Security**: Smoke test verifies the launcher only; sub-binary verification stays bypassed
  **Location**: Phase 2: `_local_vcs_subbinary`
  The docstring calls it end to end, but `ACCELERATOR_VCS_BIN` skips the
  launcher's own verification of the sub-binary. Say so and name where
  that is covered.
- 🔵 **Architecture**: Hermeticity guarantee is module-scoped while the suite keeps an off-funnel bootstrap path
  **Location**: What We're NOT Doing
  `test_research_guard_registration.py` still runs a copied bootstrap
  through raw `subprocess.run` with an inherited environment.
- 🔵 **Code Quality**: Refusal test carries two acts and a nested closure in one body
  **Location**: Phase 3, Section 1
  Name the results `refused` and `accepted`, extract the repeated asset
  list, and share a module-level `_detect` helper with the smoke test.
- 🔵 **Correctness**: The 422 s threshold treats an exact 3× result as a miss
  **Location**: Phase 1, Section 4
  1,266 / 3 is exactly 422, and the plan triggers the follow-up at
  `≥ 422 s`.
- 🔵 **Correctness**: The claimed launcher shares a path with `build_launcher()`'s claim in one pytest process
  **Location**: Phase 2: `claim_artefact(BUILT_LAUNCHER)`
  Harmless while both are the `test-loopback` variant; record that the
  equivalence is also what makes the shared claim path safe.
- 🔵 **Performance**: The shim timing script has no warm-up and always runs the slow shim first
  **Location**: Phase 1, Section 4: Shim speed
  Run each shim once untimed and alternate them in the loop; print min
  and max beside each median.
- 🔵 **Portability**: Measurement script leaves `$SCRATCH` undefined and tool presence implicit
  **Location**: Phase 1, Section 4: Shim speed
  `minisign -W` needs the mise-pinned version. Locally,
  `require("minisign")` skips, so the hooks suite can pass with both new
  tests skipped.
- 🔵 **Performance**: The smoke test writes the launcher three times and leaves two copies in pytest's retained temp
  **Location**: Phase 2: smoke test
  Negligible time; about 450 MB retained across three runs. No change
  needed for speed.
- 🔵 **Standards**: Module keeps a local `REPO_ROOT` beside imports from a module that exports it
  **Location**: Phase 2: module constants
  Import it from `tests.integration.support.installation`, as the
  skill-invocation suite does.

### Strengths

- ✅ The override covers the whole hot path: the shim does one
  `std::fs::read` and one `PublicKey::verify`, and `minisign-verify` has
  no dependencies of its own.
- ✅ Both new tests go through `run_bootstrap` and `assert_hermetic`, with
  an explicit minimal environment, in place of a raw `subprocess.run`
  with an inherited one.
- ✅ No production control is weakened: no trust-chain code changes, no
  new override or seam, and shipped binaries are built with `--release`.
- ✅ The plan's claims about the refusal path, download-log order, the
  control's fresh fetch, the stub's argv record, the pin test's red
  failure and the drift marker all hold against the code.
- ✅ Every assertion bullet in the work item's hermetic-smoke and refusal
  criteria has a matching assertion in the plan's code.
- ✅ The refusal test carries its own positive control in the same
  installation and asserts the forwarded argv.
- ✅ `_DETECT_COMMAND` derived from `_DETECT_ARGS` gives the registration
  and smoke tests one definition of the hook command.
- ✅ A conditional assertion that could not fail is replaced by a strict
  envelope check.
- ✅ The speed bars sit well below the measured gains, and the method
  controls the main confounders (same-host pairs, medians, discarded
  warm-up, one shared signed launcher).
- ✅ Scope is bounded explicitly, the two PRs are independently
  shippable, and the coupling to 0366 is named in both directions.
- ✅ Every proposed line fits 80 columns, and the retained comments
  record a reason the code cannot express.

### Recommended Changes

1. **Give every assertion a red observation, or narrow the claim**
   (addresses: Mutation checks only turn the first assertion red; The
   `cwd=REPO_ROOT` mutation only fails in a secondary workspace)
   Add targeted mutations per assertion group (for example, drop
   `ACCELERATOR_LOG=error` for empty stderr; drop `ACCELERATOR_VCS_BIN` to
   reach the envelope check), or for each mutation comment out the earlier
   assertions and record every later one that fails. Replace mutation
   check 2 with a `cwd` holding a freshly initialised jj repository.

2. **Reorder assertions so failures show the bootstrap's stderr**
   (addresses: Assertion order and unguarded reads hide the bootstrap's
   stderr)
   Assert exit status and stderr first in the smoke test. Guard the
   unverified-log read with `assert log.exists(), result.stderr`. Keep the
   control's result and assert its exit status before reading the record.

3. **Correct the plan's inaccurate statements** (addresses: Change bullet
   removes the `os` import; `report.rs` slice does change; lock directory
   does not remain; 422 s threshold; swallowed adapter failure wording)
   Remove only `subprocess` and `LAUNCHER`. Say the slice gains the
   comment and passes because it has neither `panic` nor `abort`. List
   what really remains in the cache after a refusal.

4. **State the trust-anchor trade accurately** (addresses: Committed
   vendored shims no longer executed on Linux; Cited test does not cover
   the re-verification; Trust-anchor breakage surfaces after the tag)
   Reword the PR 2 note to cite what each test proves. Then either add a
   cheap hermetic check that the host's vendored shim accepts a per-run
   release signature and rejects an attacker one, or record the gap as an
   accepted risk.

5. **Tidy the new module's helpers and names** (addresses: third copy of
   the fixtures; `install` factory; `_local_vcs_subbinary`; module
   docstring; refusal test's two acts; local `REPO_ROOT`)
   Reuse `host_platform` and `shim_bin`. Give the factory explicit
   keywords. Rename the environment helper for what it returns and pick
   one owner for the sub-binary path. Rewrite the docstring in the present
   tense and give its final Phase 3 text.

6. **Bring the task layer into line** (addresses: stale task and helper
   docstrings; two build-provisioning mechanisms)
   Add a Phase 2 change item for the `hooks` task docstring,
   `accelerator_env` and `tasks/README.md`. Keep a build-task hint for an
   unbuilt tree.

7. **Move or drop the offline run, and guard against skips** (addresses:
   manual offline run under Automated Verification; measurement script
   tool presence)
   Move it to Manual Verification with its warm-run precondition, or drop
   it as the work item allows. Add "neither new test is skipped" to the
   Phase 2 and 3 criteria.

8. **Tighten the measurements** (addresses: CI duration instrument and
   grep; fixed fixture build; rebuild cost; cargo-lock wait; shim timing
   script)
   Record mise's `Finished in` as the figure compared with 1,266 s, plus
   the first-case and median per-case durations and the Linux launcher's
   size. Show how to resolve the ubuntu job id. Record hooks-suite timings
   against the 2.7 s baseline. Define `$SCRATCH`.

9. **Strengthen the pin test** (addresses: pin test cannot detect the
   override going inert; pin placement)
   Also assert `minisign-verify` is a workspace dependency, and either
   move the pin to a module named for Cargo profiles or say why
   `test_dev_builds.py` is its home.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound: it moves the hooks smoke test
onto the existing `make_installation` / `run_bootstrap` funnel rather than
inventing a second harness, keeps per-call re-verification untouched, and
splits the profile change from the test rewrite into independently
shippable PRs. Its structural claims check out. The findings are all
minor.

**Strengths**:
- Reuses the single `run_bootstrap` funnel and its `assert_hermetic`
  preconditions.
- Dependency direction stays clean; no production, task or bootstrap
  changes are needed for Phases 2–3.
- The profile override is the narrowest possible change, and the plan
  checks the two other readers of the manifest.
- The pin test gives 0366 an executable contract for the table it will
  reshape.
- Tradeoffs are stated: the moved key check and the kept
  `ACCELERATOR_VCS_BIN`.
- Respects the `build_launcher()` contract by claiming the mise-built
  output.
- `_DETECT_COMMAND` derived from `_DETECT_ARGS` removes a divergence
  point.

**Findings**:
- **Minor (high)** — *Third hand-wired copy of the installation fixtures,
  including a dead `bootstrap` argument* (Phase 2: fixtures). The
  entrypoint and skill-invocation modules already wire the same five
  things. Each copy calls `copy_bootstrap(...)` only to satisfy a keyword
  `make_installation` never reads. Suggestion: accept explicitly and
  record that the next consumer extracts a shared set, or make
  `bootstrap` optional in PR 2.
- **Minor (high)** — *Bootstrap naming contract re-encoded as literals*
  (Phase 2 and 3 helpers). Asset names, cache path, `.tmp-launcher-*`,
  the log name and the refusal text are repeated; a missed glob fails
  open. Suggestion: lift accessors such as `cached_launcher`,
  `unverified_log` and `served_asset_names` into the support module.
- **Minor (high)** — *Task-layer overlay and descriptions left describing
  the old test shape* (Phase 2: Notes). The overlay reaches the test only
  through an `os.environ` read, the default path has two owners, and the
  `hooks` task docstring is stale. Suggestion: choose one owner in PR 2
  and update both docstrings.
- **Minor (medium)** — *Hooks suite ends up with two build-provisioning
  mechanisms* (Key Discoveries; Performance Considerations). Mise edges
  for the launcher and `accelerator-vcs`, an in-pytest build for the
  shim. Suggestion: state it as an accepted, temporary divergence and
  keep a build-task hint for the unbuilt case.
- **Suggestion (medium)** — *Refusal test widens a module named for
  vcs-detect registration* (Phase 3). It never reaches `vcs detect`.
  Suggestion: a sibling module, or say the placement is deliberate.
- **Suggestion (medium)** — *Hermeticity guarantee is module-scoped*
  (What We're NOT Doing). The sibling research-guard module still runs
  off the funnel. Suggestion: scope the Desired End State wording to the
  module and note the residue.

### Code Quality

**Summary**: The plan is proportionate and well reasoned. The
maintainability concerns are all in the Phase 2/3 test code; none is
blocking and several are cheap to fix before implementation.

**Strengths**:
- Reuses the installation apparatus rather than a parallel harness.
- `_DETECT_COMMAND` derived from `_DETECT_ARGS` removes a duplicated
  literal.
- A conditional assertion becomes unconditional structural assertions.
- The `Cargo.toml` comment records a real "why"; the pin test's name
  expresses intent.
- Extracted helpers keep the test bodies reading as domain statements.
- Scope is bounded, and the mutation-check red step is a sensible
  substitute where no production code changes.

**Findings**:
- **Minor (high)** — *Assertion order and unguarded reads hide the
  bootstrap's stderr on failure* (Phase 2 and 3 test bodies). Also
  `[entry] = …` raises an anonymous `ValueError`. Suggestion: assert the
  exit status first, guard the log read, and check the control's result.
- **Minor (high)** — *Third hand-copied set of bootstrap fixtures, with a
  third vocabulary* (Phase 2: fixtures). Suggestion: hoist into a pytest
  plugin module, or adopt the entrypoint names and record the deferral.
- **Minor (high)** — *`install` is a factory that fails on its second
  call and hides its parameters behind `**served`* (Phase 2). Suggestion:
  explicit keyword-only parameters, or a plain helper function.
- **Minor (high)** — *Module docstring is ungrammatical and becomes
  untrue; module name no longer fits* (Phase 2 and 3). Suggestion:
  rewrite in the present tense with no reference to the retired harness;
  rename the module or say why the refusal case lives there.
- **Minor (high)** — *Task and helper docstrings describing the old smoke
  test are left stale* (Phase 2: omission). Suggestion: add a change item
  for `tasks/test/integration.py`, `tasks/test/helpers.py` and
  `tasks/README.md`.
- **Minor (medium)** — *`_local_vcs_subbinary` has a misleading name, an
  ambient input, and a docstring covering two concerns* (Phase 2). It
  also drops the old test's actionable build hint. Suggestion: rename for
  what it returns, pick one source for the path, and keep only the
  non-obvious sentence.
- **Suggestion (medium)** — *Refusal test carries two acts and a nested
  closure* (Phase 3). Suggestion: `refused` / `accepted`, a shared asset
  helper, `_refusal_diagnostic`, a module-level `_detect`.
- **Suggestion (medium)** — *Pin test sits in a module about the
  dev-build registry* (Phase 1). Acceptable as planned.

### Test Coverage

**Summary**: The plan's test code maps onto every bullet of the work
item's criteria, and the facts it relies on hold. Mutation checks are
sound in principle, but the four listed only ever turn the first failing
assertion red, so the plan's claim does not hold as written.

**Strengths**:
- Every acceptance-criterion bullet has a corresponding assertion.
- The refusal test carries its own positive control and asserts the
  forwarded argv.
- The smoke and refusal tests are tied to the registered hook command.
- An assertion that could not fail is replaced by a strict envelope
  check.
- `assert_hermetic` is enforced structurally, and binaries are claimed.
- The cache assertions use name globs, which tolerate the staged shim.
- Phase 1 is honest that speed cannot be unit-tested.

**Findings**:
- **Major (high)** — *Mutation checks only turn the first assertion red*
  (Phase 2 and 3 Red Steps; Implementation Approach). Serving the stub
  fails at the byte comparison, so `hookEventName`, the download log and
  empty stderr are never seen red. Signing with the release key fails at
  stderr, so the unverified log, non-execution and both cache globs are
  never seen red. A mistyped glob would pass both the mutation checks and
  the green run. Suggestion: targeted mutations per assertion group, or
  comment out earlier assertions per mutation, or narrow the claim.
- **Minor (high)** — *Assertion order and an unchecked control result
  hide the bootstrap's stderr* (Phase 2 and 3). Suggestion: attach
  `result.stderr` or assert the exit status first; check the control.
- **Minor (medium)** — *Pin test cannot detect the override going inert*
  (Phase 1). Suggestion: also assert `minisign-verify` is in
  `[workspace.dependencies]`; optionally a generous wall-clock ceiling.
- **Minor (medium)** — *The cited release-time test mocks the
  re-verification it is said to cover* (Phase 3; What We're NOT Doing).
  Suggestion: reword the note and confirm or record the gap.
- **Suggestion (high)** — *One mutation depends on the checkout type, and
  the offline run is not automated* (Phase 2). Suggestion: a temporary
  jj repository for the mutation; move or drop the offline run.

### Correctness

**Summary**: The plan's factual claims hold almost everywhere, and the
proposed test code is logically sound against the real bootstrap,
launcher, `vcs-cli` and installation helpers. No critical or major
defects. Nothing was executed; verification was by reading the code.

**Strengths**:
- The refusal path matches `bin/accelerator` in exit status, stderr, log
  entry, clean-up and cache state.
- Download-log ordering holds, and the first assertion runs before the
  second call appends.
- The control fetches afresh; the stub writes argv one per line; both
  Phase 3 mutations fail where stated.
- All installation helpers have the signatures the plan assumes.
- The pin test's red failure is `KeyError: 'dev'`, and the table is
  valid in the workspace root manifest.
- The drift marker is unaffected.
- Smoke-test expectations match the launcher's override short-circuit
  and `vcs-cli`'s envelope in a non-repository directory.
- Task wiring matches, and `_DETECT_COMMAND` equals the registered
  string.
- The measurement script uses the shim's real argument order and the
  right minisign flags.

**Findings**:
- **Minor (high)** — *The `report.rs` profile slice does change* (Key
  Discoveries). It gains the new comment and passes because the comment
  has neither `panic` nor `abort`. Suggestion: correct the discovery.
- **Minor (high)** — *The lock directory does not remain after a
  refusal* (Phase 3 notes). Suggestion: reword; optionally assert no
  `.accelerator-lock-*` remains.
- **Minor (high)** — *Change list says to remove the `os` import, but the
  new code uses it* (Phase 2). Suggestion: remove only `subprocess`.
- **Minor (high)** — *Assertion order hides the bootstrap's diagnostic on
  a fail-safe refusal* (Phase 2). Also, the bare `systemMessage` envelope
  fails one line earlier, with `KeyError: 'hookSpecificOutput'`.
  Suggestion: reorder, and reword the discovery.
- **Minor (medium)** — *The `cwd=REPO_ROOT` mutation only fails in a
  secondary workspace* (Phase 2 Red Step). The assertions also do not
  distinguish a non-repository directory from a main git checkout.
  Suggestion: a mutation that fails everywhere.
- **Suggestion (medium)** — *The 422 s threshold treats an exact 3×
  result as a miss* (Phase 1). Suggestion: `> 422 s` for the miss.
- **Suggestion (low)** — *The claimed launcher shares a path with
  `build_launcher()`'s claim in one pytest process* (Phase 2). Harmless
  while both are the same variant; record it.

### Security

**Summary**: The plan does not weaken any production control. Negative
trust-chain coverage is strengthened, and the release-time
re-verification it relies on exists and uses the committed key. What is
lost is the only pull-request execution of the committed vendored shims
and key against a genuine signature. All findings are fail-closed.

**Strengths**:
- No test-only affordance leaks into production; `ACCELERATOR_VCS_BIN` is
  an existing documented override.
- The override is confined to the dev profile; shipped binaries are
  built with `--release`.
- The smoke test stops running the repository's bootstrap with an
  inherited environment.
- The refusal test asserts non-execution, an untainted cache, removed
  temporary files and a durable record, with a same-installation control.
- Byte identity plus the download log closes the old test's holes.
- Release-time coverage is real: `tasks/github.py:176-189`, exercised at
  `test_github.py:694-728`.

**Findings**:
- **Minor (high)** — *Committed vendored shims are no longer executed on
  Linux anywhere* (What We're NOT Doing; Phase 2). Release runs on
  `macos-latest` with the host shim only; the other three shims have
  static guards. A corrupt shim would ship and every session on that
  platform would refuse the launcher. Suggestion: a cheap hermetic check
  of the host's vendored shim against per-run keys, or record the gap.
- **Minor (high)** — *Cited test does not cover the re-verification the
  plan relies on* (Phase 3). Suggestion: cite the accept/reject pair too,
  word the note accurately, and consider pinning `RELEASE_PUBLIC_KEY` to
  `keys/accelerator-release.pub`.
- **Minor (medium)** — *Trust-anchor breakage now surfaces after the tag
  is pushed* (What We're NOT Doing). The old test's `stderr == ""` did
  catch a refused launcher. Suggestion: record the trade; a path-filtered
  or scheduled job would restore early detection.
- **Suggestion (medium)** — *Smoke test verifies the launcher only*
  (Phase 2). Suggestion: say in the docstring that sub-binary
  verification is out of scope and where it is covered.

### Performance

**Summary**: The plan targets the right bottleneck with the smallest
fix, and the code supports its central claim. No critical or major
problems. The concerns are about measurement.

**Strengths**:
- The override covers the hot path, and the research's measurement
  agrees.
- The bars sit well below the measured gains, so fixed overheads cannot
  plausibly cause a false miss.
- The method controls the main confounders.
- Fixture scoping is proportionate.
- The refusal test serves the tiny stub, so only one test moves 76 MB.
- Reruns of the CI job are forbidden.

**Findings**:
- **Minor (high)** — *CI duration is read with a different instrument
  from its baseline* (Phase 1, Section 4). Suggestion: record mise's
  `Finished in` as the compared figure and say what to do on an empty
  grep.
- **Minor (medium)** — *A single CI total cannot separate the fixed
  fixture build from per-case cost* (Phase 1, Section 4). The 3× bar on
  the total needs about 4.2× per case. Suggestion: also record the first
  case's duration, the median per case and the Linux launcher's size.
- **Minor (medium)** — *The override's rebuild cost is understated*
  (Performance Considerations). Suggestion: say it forces a one-off
  rebuild of dependents in every dev-derived configuration, and that no
  timing comes from the first run after the change.
- **Minor (medium)** — *The hooks suite's new cargo-lock wait is
  asserted, not measured* (Performance Considerations; Phase 2).
  Suggestion: record the suite alone and inside one warm full run,
  against the 2.7 s baseline.
- **Suggestion (high)** — *The shim timing script has no warm-up and
  always runs the slow shim first* (Phase 1). Suggestion: one untimed run
  each, alternate, print min and max.
- **Suggestion (medium)** — *The smoke test writes the launcher three
  times and leaves two copies in retained temp* (Phase 2). No change
  needed for speed.

### Standards

**Summary**: The plan follows the project's conventions closely. The
findings are all minor: one contradiction between a bullet and its code
block, names that diverge from the sibling suites, and a manual step
listed as automated. No changelog entry is needed.

**Strengths**:
- Imports the support functions directly, as the skill-invocation suite
  does.
- `keys` and `downloader` match the entrypoint suite; the `BUILT_*`
  constants follow the sibling module.
- Every proposed line fits 80 columns; no `__init__.py` or `conftest.py`
  is added.
- The profile comment records why, in the style of the one above it, and
  cites no work item.
- Old inline comments are dropped; only the non-obvious reason stays.
- The TDD departures are stated and justified.
- Omitting `repo_bin_is_untouched` is a reasoned departure.
- Single-test commands use the runner the mise tasks call.

**Findings**:
- **Minor (high)** — *Change bullet removes the `os` import that the code
  block still uses* (Phase 2). Suggestion: remove `_dispatch_env`,
  `LAUNCHER` and `subprocess` only.
- **Minor (medium)** — *`_local_vcs_subbinary` names a binary but returns
  an environment overlay* (Phase 2). Suggestion: for example
  `_local_vcs_dispatch_env`, keeping only the two non-obvious reasons.
- **Minor (medium)** — *Fixture names give the third bootstrap suite a
  third vocabulary* (Phase 2). Suggestion: `host_platform`, `shim_bin`, a
  noun-phrase factory with explicit keywords.
- **Minor (medium)** — *Cargo profile pin placed in the module that tests
  the `DEV_BUILDS` registry* (Phase 1). Suggestion: a module such as
  `test_cargo_profiles.py`, or a sentence saying why not.
- **Minor (high)** — *Manual offline run listed under Automated
  Verification* (Phase 2). Suggestion: move or drop.
- **Suggestion (medium)** — *Module docstring summary line is a
  garden-path sentence* (Phase 2). Suggestion: lead with what the module
  guards now and give the full Phase 3 text.
- **Suggestion (low)** — *Module keeps a local `REPO_ROOT`* (Phase 2).
  Suggestion: import it from the support module.

### Portability

**Summary**: The plan is largely environment-independent because both new
tests go through `run_bootstrap`. No critical or major blockers. The
remaining risks are host assumptions stated as facts or left implicit.

**Strengths**:
- Only `PATH`, `HOME` and the stub-downloader variables are passed, so
  inherited variables cannot change the result.
- `host_platform()` matches the bootstrap's mapping on all four targets.
- `minisign` and `gh` are pinned, and `require("minisign")` fails in CI.
- `hyperfine` is replaced with a stdlib-only script.
- `claim_artefact` and `serve_launcher` make the executable bit
  independent of umask; the comparison is by bytes.
- `/bin/bash` is pinned, so the test runs on the 3.2 floor.

**Findings**:
- **Minor (high)** — *Default cwd is only outside a checkout if the temp
  root is* (Phase 2 notes). A `noexec` temp root fails earlier with a
  misleading message. Suggestion: an explicit `cwd` with a precondition
  that names `TMPDIR`, or state the assumption.
- **Minor (high)** — *Hard-coded `cli/target/debug` ignores
  `CARGO_TARGET_DIR`* (Phase 2 constants; Phase 1 measurements).
  Suggestion: honour `ACCELERATOR_BIN` for the launcher, and note the
  convention as a host assumption.
- **Minor (high)** — *Offline check is manual, host-specific and
  confounded by cargo and uv* (Phase 2). Suggestion: move with its
  precondition, drop it, or name an OS-level mechanism.
- **Minor (high)** — *`cwd=REPO_ROOT` mutation only fails inside a jj
  secondary workspace* (Phase 2 Red Step). Suggestion: a temporary jj
  repository.
- **Minor (medium)** — *CI log grep assumes the mise prefix and the
  pytest summary shape* (Phase 1). Suggestion: loosen to
  `passed.* in [0-9.]+s`, show how to resolve the job id, and check the
  prefix against the baseline run.
- **Suggestion (medium)** — *Measurement script leaves `$SCRATCH`
  undefined and tool presence implicit* (Phase 1). Suggestion: define it,
  use `mise exec --`, and require that neither new test is skipped.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-11T23:08:07+00:00

**Verdict:** COMMENT

The plan was revised after pass 1: three pull requests rather than two, a
new Phase 2 that extracts shared installation fixtures, a mutation table
per test, a single owner for the sub-binary path, and a new Phase 5 that
runs the committed shim for the host. All eight lenses re-ran.

### Previously Identified Issues

- 🟡 **Test Coverage**: Mutation checks only turn the first assertion red — Partially resolved (now minor). 16 of 18 rows fail at the named assertion; five assertions still have no row
- 🔵 **Code Quality / Test Coverage / Correctness**: Assertion order hides the bootstrap's stderr — Partially resolved. The smoke test and the control are fixed; the refusal test's first assertion carries no message, and `[entry] = …` fails as a bare `ValueError`
- 🔵 **Correctness / Standards**: Change bullet removes the `os` import the code uses — Partially resolved. `os` is right to go; the bullet now wrongly removes `subprocess`, which `_detect`'s annotation needs
- 🔵 **Correctness**: The `report.rs` profile slice does change — Resolved
- 🔵 **Correctness**: The lock directory does not remain after a refusal — Resolved
- 🔵 **Correctness / Test Coverage / Portability**: The `cwd=REPO_ROOT` mutation only fails in a secondary workspace — Resolved
- 🔵 **Correctness**: The 422 s threshold treats an exact 3× result as a miss — Resolved
- 🔵 **Correctness**: The claimed launcher shares a path with `build_launcher()`'s claim — Resolved (stated; see the new architecture finding)
- 🔵 **Security**: Committed vendored shims are no longer executed on Linux anywhere — Resolved by Phase 5
- 🔵 **Security / Test Coverage**: Cited test does not cover the re-verification — Resolved
- 🔵 **Security**: Trust-anchor breakage surfaces after the tag is pushed — Partially resolved. Accepted and stated; the committed key is still unchecked before merge, and "one stage later" understates a post-merge discovery
- 🔵 **Security**: Smoke test verifies the launcher only — Resolved (stated in the docstring)
- 🔵 **Test Coverage**: Pin test cannot detect the override going inert — Resolved
- 🔵 **Architecture / Code Quality / Standards**: Third copy of the fixtures, with a third vocabulary — Resolved. `harness` locals remain in the entrypoint suite
- 🔵 **Code Quality**: `install` factory fails on its second call and hides its parameters — Resolved
- 🔵 **Architecture**: Bootstrap naming contract re-encoded as literals — Accepted as a stated non-goal
- 🔵 **Architecture / Code Quality / Standards**: `_local_vcs_subbinary` name, ambient input, two owners — Partially resolved. The ambient read is gone; the new name still hides that it returns an environment and sets two things
- 🔵 **Architecture / Code Quality**: Stale task and helper docstrings — Partially resolved. Rewritten, but the hooks task still passes an overlay no hooks module reads, and the new docstring describes one test's internals
- 🔵 **Architecture**: Two build-provisioning mechanisms — Accepted as a stated, temporary non-goal
- 🔵 **Code Quality / Architecture / Standards**: Module docstring and name — Partially resolved. The docstring is fixed; the module name still does not cover the refusal test
- 🔵 **Standards / Code Quality**: Pin test's home — Resolved
- 🔵 **Standards / Portability / Test Coverage**: Manual offline run under Automated Verification — Resolved
- 🔵 **Portability**: Default cwd only outside a checkout if the temp root is — Resolved
- 🔵 **Portability**: Hard-coded `cli/target/debug` — Accepted as a stated non-goal
- 🔵 **Performance / Portability**: CI duration instrument and grep — Resolved
- 🔵 **Performance**: Single CI total cannot separate the fixed build — Resolved (extraction method still missing; see below)
- 🔵 **Performance**: Rebuild cost understated — Resolved
- 🔵 **Performance**: Hooks cargo-lock wait asserted, not measured — Partially resolved. Measured locally, once; CI is not read
- 🔵 **Architecture**: Hermeticity guarantee is module-scoped — Accepted as a stated non-goal
- 🔵 **Code Quality**: Refusal test carries two acts and a closure — Resolved
- 🔵 **Performance**: Shim timing script has no warm-up — Resolved
- 🔵 **Portability**: `$SCRATCH` undefined and tool presence implicit — Partially resolved. The bare `uv run pytest` commands still need a mise shell, nothing enforces "no skipped test", and the whole-suite form cannot be met in a root container
- 🔵 **Performance**: Launcher written three times; retained temp — Partially resolved. Writes accepted; retention not mentioned
- 🔵 **Standards**: Local `REPO_ROOT` — Resolved

### New Issues Introduced

- 🔵 **Test Coverage / Correctness**: The planted-lock mutation row misbehaves with the real lock name — as `.accelerator-lock-<platform>` it fails at the first assertion, stalls about 30 s, passes, or hangs, depending on what is planted. Name decoys in the table
- 🔵 **Test Coverage**: Three mutation rows need exact set-up — the jj and worktree repositories must be created after `_outside_any_checkout` returns, the worktree needs a commit identity, and the pre-written log line needs a trailing newline
- 🔵 **Standards / Correctness**: `grep -rn "vcs_bin" tasks` can never find nothing — it matches `vcs_binary` in `tasks/measure.py`. Use `grep -rnw`
- 🔵 **Architecture**: Phase 3 removes vendored-shim coverage that only Phase 5 restores — Phase 5 depends only on Phase 2, so it can move ahead of the smoke-test rewrite
- 🔵 **Security / Test Coverage**: Phase 5's refusing test proves little on its own — it accepts any non-zero exit, and an attacker-key signature probably stops at the key-id comparison. Add a tampered-payload case and assert exit 1 with `signature verification failed`
- 🔵 **Security / Correctness**: The residual-gap statements overlook `check-measure-harness` — a non-blocking pull-request lane that runs the real bootstrap against a published release. Say "the only blocking test"
- 🔵 **Architecture / Standards**: `pytest_plugins` in a test module registers the fixtures session-wide — a first use in the repository. A module that forgets the declaration passes in a directory run and fails alone; require each declaring module to pass alone
- 🔵 **Architecture / Code Quality**: The hooks module is a second writer to the claimed-artefact path and another owner of `cli/target/debug` paths — after a plain `cargo build`, a combined run can swap the binary behind the entrypoint suite's `launcher_bin`
- 🔵 **Architecture / Code Quality**: The hooks task keeps an overlay nothing reads, pinned by a test of an absence — drop `env=` and assert the positive contract, or name the consumer
- 🔵 **Code Quality**: `_detect` takes the environment as `**env` — a misspelt option becomes a variable. Take `env: dict[str, str] | None`
- 🔵 **Code Quality**: `_outside_any_checkout` creates the directory it is named for checking; it also tests the filesystem root, which the detector ignores (portability)
- 🔵 **Code Quality / Standards**: `new_installation` is untyped in the hooks tests and `Callable[..., Installation]` elsewhere — a `Protocol` would keep its keywords checked
- 🔵 **Code Quality / Standards**: The rename leaves about 180 `harness` locals in the entrypoint suite
- 🔵 **Code Quality / Standards**: The new `hooks` task and `test_vendored_shim.py` docstrings describe other files' internals and will go stale
- 🔵 **Standards**: `test_integration_tasks.py` is confusable with `test_integration_skills.py` and `tests/integration/tasks/`
- 🔵 **Standards / Test Coverage**: Some automated criteria need a manual observation — the red states, and "same number of cases" in Phase 2, which also does not rule out skips
- 🔵 **Performance**: The CI figure may come from the run the plan says not to measure; the per-case figures have no extraction method; the baseline's 143 cases against today's count is unrecorded
- 🔵 **Portability**: Contributors on darwin-x64 and linux-arm64 hosts will be the first to execute those shims; an executable-bit check would separate a mode problem from a verification failure
- 🔵 **Code Quality**: The pin test's first assertion does not say why it is there

### Assessment

The revision did its job. The one major finding is now minor, 20 of the 34
earlier findings are resolved outright, four are accepted as stated
non-goals, and ten are partly resolved. No lens found a critical or major
issue, and the correctness lens confirmed the new factual claims by
reading the code (pytest plugin loading, the invoke task test, refusal
ordering, `vcs-cli` strings, shim arguments and exit codes).

What remains is small and concentrated in what the revision added. Four
items would trip an implementer who followed the plan literally and
should be fixed before work starts: the `subprocess` import bullet, the
`vcs_bin` grep, the planted-lock row, and the set-up for the jj,
worktree and log-line rows. Two are worth a decision: moving the
vendored-shim phase ahead of the smoke-test rewrite, and adding a
tampered-payload case to it. The rest is polish.

Not verified by any lens: pytest's real behaviour for module-level
`pytest_plugins` (reasoned from its documentation), the executable bit
on the committed shims, and that the real launcher writes nothing to
stderr when its cache root is the fixture's.

**Final verdict:** APPROVE. Every pass-2 finding was applied to the plan:
the four literal errors, the five decision items (the vendored-shim check
now precedes the smoke-test rewrite and gains a tampered-payload case),
and the polish. The plan is marked `ready`. Those last edits were not
re-reviewed.
