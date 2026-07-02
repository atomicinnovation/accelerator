---
type: plan-review
id: "2026-07-02-0162-rust-toolchain-guard-rails-review-1"
title: "Plan Review: Rust Toolchain Guard Rails in mise + CI"
date: "2026-07-02T07:19:55+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-02-0162-rust-toolchain-guard-rails"
target: "plan:2026-07-02-0162-rust-toolchain-guard-rails"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, compatibility, usability, security]
review_number: 1
review_pass: 2
tags: [rust, tooling, ci, guard-rails, architecture-enforcement]
last_updated: "2026-07-02T15:05:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Rust Toolchain Guard Rails in mise + CI

**Verdict:** REVISE

The plan is architecturally strong and unusually disciplined: it separates the
two enforcement mechanisms at their correct granularities (cargo-deny between
crates, cargo-pup inside a crate), isolates the nightly lane structurally, phases
the work for independent mergeability, and respects the check/test disjointness
invariant and the task-naming contract throughout. However, two tool-pin
declarations diverge from the luminosity reference in ways that will fail to
install or fail to load (a broken nextest aqua pin and cargo-pup wrongly declared
as a stable-lane `[tools]` entry), and a load-bearing CI workaround the source
research explicitly flagged as required for any new cargo job has been silently
dropped. These, plus a cluster of test-hermeticity and version-coherence-design
gaps, warrant revision before implementation.

### Cross-Cutting Themes

- **cargo-pup provisioned wrongly as a stable `[tools]` entry** (flagged by:
  Compatibility 🔴, Security 🟡) — Phase 4 §2 adds `"cargo:cargo-pup" = "0.1.8"`
  to mise `[tools]`, which luminosity deliberately never does. It builds against
  the stable 1.90.0 toolchain (lacking the nightly `rustc_private` ABI cargo-pup
  requires), breaks lane isolation, and installs from source with no
  checksum/provenance verification. This is the single most important fix.

- **New cargo CI jobs drop the RUSTUP_HOME toolchain-race workaround** (flagged
  by: Architecture 🟡, Compatibility 🟡, Correctness 🟡) — the research
  (main.yml:143-160) calls the `RUSTUP_HOME`-routing + `cache_key_prefix`
  workaround on `check-visualiser-server` load-bearing "for any new cargo-running
  job". The plan substitutes `Swatinem/rust-cache` alone (mirroring luminosity,
  which never carried accelerator's race), leaving `check-cli`,
  `check-supply-chain`, `check-architecture`, and the extended test jobs exposed
  to the "detected conflict: bin/cargo" flake — or the repo carrying two
  divergent, unexplained CI caching strategies.

- **Version-coherence member-enumeration is net-new and under-designed** (flagged
  by: Correctness 🟡, Code Quality 🟡) — luminosity has no member enumeration at
  all, so `_read_workspace_version` / `_member_version_mismatches` are original
  design, not a mirror. Folding a pre-filtered mismatch dict into the `found` dict
  (which is then re-diffed) mixes two representations and risks double-filtering.

- **Fixture regressions' hermeticity and config coupling are unspecified**
  (flagged by: Test Coverage 🟡, Code Quality 🔵) — neither the deny nor the pup
  fixture states whether it resolves a real crate graph (network), synthesises or
  checks-in the fixture, or exercises the *real* `cli/deny.toml`/`cli/pup.ron` vs
  a copy. This affects both determinism and whether the shipped rule is actually
  proven.

- **Escape-hatch discoverability and toolchain onboarding are thin** (flagged by:
  Usability 🟡🔵) — `ACCELERATOR_COVERAGE` has no documentation task at all, the
  GC'd-nightly recovery message is promised "actionable" but never specified, and
  the new env toggles skip the repo's established README env-var-table convention.

- **cd vs --manifest-path split within the cli component** (flagged by:
  Architecture 🔵, Standards 🔵) — format/lint use `--manifest-path` (repo
  convention) while deny/pup `cd cli/` (config-relative tools). Largely forced,
  but should be flagged as deliberate rather than accidental.

- **CI job names diverge from the check-\<component\> pattern** (flagged by:
  Standards 🔵, Usability 🔵) — `check-supply-chain`/`check-architecture` name the
  concern, not the task, so a red job isn't mechanically traceable to its local
  `mise run` command.

### Tradeoff Analysis

- **Nightly-lane isolation vs test reach**: Security and Usability value running
  the heavy nightly work in exactly one isolated job (`check-architecture`), so a
  nightly outage gates only that job. Test Coverage notes the cost: the pup
  regression — the sole proof the lane enforces anything — then never runs on the
  ubuntu+macos matrix or in a local `mise run test`, so it can silently be dropped
  from its one host. Recommendation: keep the isolation, but add a structural
  `test_workflows.py` assertion that `check-architecture` actually invokes both
  `pup:check` and `test:integration:pup`, so the regression can't vanish
  unnoticed.

- **Luminosity parity vs accelerator conventions**: Mirroring luminosity is the
  plan's core strength, but it is the direct source of three divergences —
  dropping the RUSTUP_HOME workaround (luminosity never needed it), the cd-based
  deny/pup invocation (luminosity's workspace root *is* repo root), and a
  comment-density clash (luminosity is heavily commented; accelerator leans
  low-comment). Where accelerator's own research or conventions contradict the
  mirror, the accelerator side should win, with the deviation noted.

### Findings

#### Critical

- 🔴 **Compatibility / Security**: cargo-pup declared as a `cargo:` mise `[tools]`
  entry — wrong toolchain, no provenance, breaks lane isolation
  **Location**: Phase 4, Section 2 (Tool pins + shared constants)
  Phase 4 §2 adds `"cargo:cargo-pup" = "0.1.8"` to `[tools]`, which luminosity
  deliberately omits (it provisions cargo-pup only via `deps:install:pup` with
  `cargo +{PUP_NIGHTLY} install cargo_pup --locked`). A `cargo:` backend builds
  from source on the stable 1.90.0 toolchain — which lacks the `rustc-dev` /
  `rust-src` nightly components cargo-pup's `rustc_private` ABI needs — so it
  either fails to build or yields an ABI-mismatched binary, duplicates/conflicts
  with `deps:install:pup`, pulls the nightly-only tool into the stable path, and
  installs with no checksum verification (unlike the `aqua:` backends).
  **Suggestion**: Remove the `"cargo:cargo-pup"` `[tools]` entry entirely;
  provision cargo-pup solely through `deps:install:pup`, keeping `PUP_VERSION` in
  `tasks/shared/rust.py` as the single source of truth.

- 🔴 **Compatibility**: nextest aqua pin drops the required sub-tool path and will
  not install
  **Location**: Phase 5, Section 1 (Tool pins + coverage toggle)
  The plan pins `"aqua:nextest-rs/nextest" = "0.9.138"`, but the working
  luminosity pin is `"aqua:nextest-rs/nextest/cargo-nextest" = "0.9.138"`. The
  repo publishes its binary as `cargo-nextest`; without the trailing sub-tool path
  the aqua backend cannot resolve the asset, so `mise install` fails on both
  ubuntu and macos, breaking `test:unit:cli` and the bare `mise run`.
  **Suggestion**: Change the pin to
  `"aqua:nextest-rs/nextest/cargo-nextest" = "0.9.138"` to match luminosity
  exactly.

#### Major

- 🟡 **Architecture / Compatibility / Correctness**: new cargo CI jobs adopt
  `Swatinem/rust-cache` but drop the load-bearing RUSTUP_HOME toolchain-race
  workaround
  **Location**: Phase 2 §5, Phase 3 §5, Phase 4 §6, Phase 5 §3 (CI jobs)
  The research explicitly states any new cargo-running job needs the
  `RUSTUP_HOME=$HOME/.local/share/mise/rustup` routing + a dedicated
  `cache_key_prefix` (the server job's workaround for a cache-hit-but-absent
  toolchain race, "detected conflict: bin/cargo"). rust-cache caches
  `~/.cargo` + `target/`, not the rustup toolchain, so it does not close this
  race. `cli:check` fans format+lint into two parallel cargo passes under one job.
  **Suggestion**: Either replicate the RUSTUP_HOME routing + per-job
  `cache_key_prefix` on each new cargo job, or document why
  `deps:install:rust-components` running first fully closes the race and reconcile
  the server job so the repo doesn't carry two contradictory rationales.

- 🟡 **Correctness / Code Quality**: version-coherence member enumeration is
  net-new (not a luminosity mirror) and folds a pre-filtered mismatch dict into
  the re-diffed `found` dict
  **Location**: Phase 1, Section 3 (Version-coherence read)
  luminosity's `build.py` has no coherence logic or member enumeration, so
  `_read_workspace_version` / `_member_version_mismatches` are original design.
  `_member_version_mismatches` already returns only offenders (`pinned != expected`),
  yet the plan folds it into `found`, which `validate_version_coherence` then
  re-diffs against `expected_version` — two filtering mechanisms combined, correct
  only by coincidence and brittle to a future change of the helper's return
  contract.
  **Suggestion**: Pick one shape — have the helper contribute `{crate: pinned}`
  entries into the mismatch set directly (single filtering site), or return all
  pinned member versions and let the existing comprehension filter. State
  explicitly that this path is original design so it gets first-class tests.

- 🟡 **Correctness**: `_read_workspace_version` / member enumeration has no guard
  for a missing/malformed manifest or empty members list
  **Location**: Phase 1, Section 3
  Bare subscripts (`data["workspace"]["package"]["version"]`,
  `data["workspace"]["members"]`) raise opaque KeyError at release time
  (`create_checksums`) if the manifest is restructured; `members = []` silently
  returns `{}` and coherence passes with zero member coverage, defeating the AC.
  **Suggestion**: Raise a clear `VersionCoherenceError` when `[workspace.package].version`
  is absent, and add tests for the empty-members and missing-table boundaries
  (fail loudly, not silently pass).

- 🟡 **Test Coverage**: the cargo-pup regression's core assertion is
  behaviourally unverified until implementation
  **Location**: Phase 4, Step 1 + Step 5
  The regression asserts a domain→adapter fixture makes `cargo +nightly pup` exit
  non-zero, but Step 1 concedes this exit-code contract is unknown ("wrap in an
  `assert_lints` form if the bare CLI does not exit non-zero"). If cargo-pup 0.1.8
  logs-but-exits-zero, both the regression and `pup:check` pass while enforcing
  nothing.
  **Suggestion**: Assert on the concrete observable contract discovered in Step 1
  (specific exit code AND a match on the violation message/rule name), so a
  log-but-exit-zero tool fails the test. Consider resolving cargo-pup's exit
  behaviour as a prerequisite spike before committing to Phase 4's structure.

- 🟡 **Test Coverage**: the cargo-deny native-tls fixture regression may not be
  hermetic or deterministic
  **Location**: Phase 3, Step 4 (tests/integration/deny/)
  cargo-deny operates on a resolved dependency graph; the fixture needs a
  `Cargo.lock` / crate resolution that normally reaches crates.io. This suite is
  wired into `test:integration` (ubuntu+macos matrix), where network flakiness has
  historically flaked shell suites.
  **Suggestion**: Commit a `Cargo.lock` for the fixture (native-tls + transitive
  openssl) so `cargo deny check bans` runs offline against a fixed graph, and
  assert the violation names `native-tls`/`openssl` specifically.

- 🟡 **Test Coverage**: the pup regression runs only in one CI job and never on
  the test matrix or locally
  **Location**: Phase 4, Steps 5-6 (`test:integration:pup` excluded from
  `test:integration`)
  The architecture regression executes only inside `check-architecture`
  (ubuntu-only); a local `mise run test` never runs it, and if that one job is
  skipped/mis-cached the gap is invisible.
  **Suggestion**: Reasonable isolation, but add a `test_workflows.py` assertion
  that `check-architecture` invokes both `pup:check` and `test:integration:pup` so
  the regression can't be dropped from its host job without a red build.

- 🟡 **Test Coverage**: the nightly-lane isolation guard is under-specified and
  hard to make discriminating
  **Location**: Phase 4, Step 6 (test_workflows.py)
  The plan asserts "check-architecture is the only job invoking the nightly lane"
  without defining how a job is textually identified as a nightly consumer, and
  gives no parametrised known-bad mutations (unlike the existing concurrency
  guard). A loose implementation could pass while a future job adds a nightly step
  or a `needs: check-architecture` edge.
  **Suggestion**: Define the detection rule concretely (a job is a nightly
  consumer iff it runs `pup:check`/`deps:install:pup`/`+nightly`) and add
  known-bad mutations (inject a `needs: check-architecture` into a stable job; a
  nightly step into another) asserting each fails the guard.

- 🟡 **Compatibility**: the `test-unit` macos leg compiles/coverage-instruments
  Rust for the first time — unverified darwin tooling
  **Location**: Phase 5, Section 3 (CI build caching)
  Folding `test:unit:cli` into `test:unit` makes `macos-latest` run cargo for the
  first time (previously only ubuntu's server-check compiled Rust). The plan does
  not confirm cargo-llvm-cov 0.8.7 / cargo-nextest 0.9.138 publish darwin
  (aarch64+x86_64) aqua assets, nor that `llvm-cov` instrumentation behaves the
  same on darwin.
  **Suggestion**: Add a success criterion that all three aqua/cargo tools resolve
  on both OSes and `test:unit:cli` coverage runs green on the macos leg before
  relying on the matrix.

- 🟡 **Standards**: the 80-col hand-duplication convention omits `cli/pup.ron`
  **Location**: Implementation Approach; Phase 4 §4
  The plan names rustfmt.toml and deny.toml for the 80-col convention (with a
  Phase 3 width check) but Phase 4's new `cli/pup.ron` is not held to it, contra
  CLAUDE.md/ADR-0048 ("80 everywhere, no automated sync check").
  **Suggestion**: State that `cli/pup.ron` is ≤80 columns and add a
  manual-verification checkbox mirroring Phase 3's deny.toml width check.

- 🟡 **Security**: deny.toml advisory policy relies on version-sensitive defaults;
  no `version = 2` schema pin nor explicit vulnerability policy
  **Location**: Phase 3, Section 2 (deny.toml)
  The `[advisories]` block sets only `unmaintained = "all"` / `yanked = "deny"`,
  with no `version = 2` pin and no explicit vulnerability policy. cargo-deny 0.16+
  changed advisory defaults; effective vuln-gating then depends on the resolved
  tool version — the exact hazard the repo's exact-pin discipline exists to
  prevent.
  **Suggestion**: Pin `version = 2` and state the vulnerability/severity policy
  explicitly rather than leaning on version-sensitive defaults.

- 🟡 **Usability**: the `ACCELERATOR_COVERAGE` escape hatch has no documentation
  task
  **Location**: Phase 5 (no Docs section)
  Only `ACCELERATOR_PUP_MODE=warn` is documented (Phase 4). Phase 5's coverage
  toggle ships visible only in an automated-verification checkbox — an
  undiscoverable escape hatch is functionally absent.
  **Suggestion**: Add a Phase 5 docs change documenting `ACCELERATOR_COVERAGE=off`
  alongside `ACCELERATOR_PUP_MODE=warn` as one "toolchain escape hatches"
  subsection.

- 🟡 **Usability**: the GC'd-nightly recovery is promised "actionable" but the
  actual error message is never specified
  **Location**: Phase 4, Section 3 (Nightly provisioning)
  `install_pup` "fails loudly (recovery: bump the pair)" but the message a
  contributor sees is never defined; a raw `rustup toolchain install` non-zero
  exit is a stack trace, not "nightly-2026-01-22 unavailable; bump
  PUP_NIGHTLY/PUP_VERSION in tasks/shared/rust.py together".
  **Suggestion**: Make it a deliverable that `install_pup` catches the failure and
  prints a message naming the pinned nightly and pointing to the matched-pair
  bump, with a verification that the message appears.

- 🟡 **Code Quality**: the re-derived server manifest literals are left
  un-consolidated while a parallel cli constant is introduced
  **Location**: Phase 1 / Phase 5 (test/unit.py:19, test/integration.py:47)
  The research flags that the two test files hand-copy the server manifest string
  instead of importing `CARGO_TOML`; Phase 1 adds `CLI_WORKSPACE_CARGO_TOML` to
  the same `paths.py` but leaves the literals, entrenching an inconsistent
  convention (some manifests via constant, some inline) that the flagged 0168
  fold-in will have to chase down.
  **Suggestion**: Replace the two inline server literals with the existing
  `CARGO_TOML` import as a small in-scope cleanup so all manifest references route
  through `paths.py`.

- 🟡 **Architecture**: in the single-crate bootstrap the sole inward-dependency
  enforcer can pass vacuously
  **Location**: Desired End State; Phase 4 §4; What We're NOT Doing
  cargo-deny's cross-crate bans are inert until 0166 and cargo-pup runs a
  near-empty rule over an `fn main(){}` launcher, so at 0162's close the only
  active enforcement is the native-tls ban and a rule with nothing real to
  constrain — the load-bearing ADR-0053 rule is scaffolded but cannot fire against
  shipped code, and the grep-tripwire floor was rejected.
  **Suggestion**: Acknowledged co-verification boundary, but state the interim
  exposure explicitly in Desired End State (inward-dependency enforcement against
  real code is deferred to 0163/0166; fixtures are the only proof until then).

#### Minor

- 🔵 **Correctness**: "Phases 2-5 are order-independent" overstates the guarantee
  — every phase edits `mise.toml` and Phases 2/4/5 edit `tasks/shared/rust.py`; an
  out-of-order merge can reference a not-yet-added task or assert against a partial
  workflow. **Location**: Implementation Approach. Reword to "independently
  mergeable on top of the prior merged state; not commutable".

- 🔵 **Correctness**: `deny.toml [graph].targets` described as "four shipped +
  gnu/darwin dev triples" is ambiguous (darwin triples are already shipped); if
  `x86_64-unknown-linux-gnu` (the ubuntu CI graph) is omitted, a gnu-only banned
  edge is built but never deny-evaluated. **Location**: Phase 3 §2. State the exact
  five-triple list.

- 🔵 **Correctness**: pup.ron adapted from luminosity may reference module paths
  (`version::core`, `kernel`) absent from the accelerator scaffold, making the rule
  match nothing (vacuous pass) or fail the clean workspace. **Location**: Phase 4
  §4. Author pup.ron against the actual scaffold module names and have the fixture
  verify both the vacuous-pass and a genuine fail.

- 🔵 **Correctness / Standards**: `cli_member_manifests` calls `tomllib.load` but
  `paths.py` imports no `tomllib` today (ruff `select=ALL` → undefined name), and
  adding TOML parsing shifts the pure path-constants module's role.
  **Location**: Phase 1 §2. Add the import, or site the helper in build.py /
  shared rust helper.

- 🔵 **Correctness**: `_CLI_WORKSPACE_CARGO_TOML_RELATIVE` is used but never shown
  defined; an inconsistent relative-path derivation would make the offending-crate
  key not match the AC's expected `cli/launcher/Cargo.toml`. **Location**: Phase 1
  §3. Define it explicitly mirroring `_CARGO_TOML_RELATIVE`.

- 🔵 **Code Quality**: `cli_member_manifests`'s absolute module-constant default
  is never exercised (all callers pass `root / ...`); a test that forgets the root
  silently reads the real repo manifest. **Location**: Phase 1 §2. Drop the default
  or make root the sole parameter.

- 🔵 **Test Coverage**: format-drift and clippy gates are proven only by one-shot
  commands, not persisted regressions — a later change to the lint table or
  `-D warnings` could silently stop the gate biting. **Location**: Phase 2 Success
  Criteria. Consider a guard on the resolved `[workspace.lints.clippy]` set + the
  `-D warnings` flag.

- 🔵 **Test Coverage**: `pup_mode()` (default deny, off→warn, unrecognised→deny)
  and the nightly-unavailable isolation are left to Manual Verification only,
  though the branching unit-tests well. **Location**: Phase 4 Manual Verification.
  Add unit tests for `pup_mode()` and the leaf's warn/deny branch.

- 🔵 **Test Coverage**: the edition-guard test is introduced in Phase 1 but
  `cli/rustfmt.toml` doesn't exist until Phase 2, so it asserts trivially in the
  interim — a false sense of drift protection if phases land separately.
  **Location**: Phase 1 §5. Introduce the guard in Phase 2, or skip/xfail it in
  Phase 1.

- 🔵 **Test Coverage**: extending the shared `fake_repo_tree` fixture changes the
  baseline every existing coherence test sees; a member-enumeration bug could make
  pre-existing tests pass for the wrong reason. **Location**: Phase 1 §5. Assert an
  existing coherence test still passes, or use a dedicated `fake_repo_tree_with_cli`.

- 🔵 **Code Quality**: the six-plus new cargo leaves each repeat the
  `run(warn=True)` → check exit → `raise Exit` boilerplate; a drift (one leaf
  forgetting `warn=True`) becomes plausible. **Location**: Phase 2/3/5. Consider a
  shared `run_or_exit` helper, or make the keep-copies-for-parity choice explicit.

- 🔵 **Code Quality**: the luminosity sources are heavily commented; a verbatim
  port clashes with accelerator's low-comment norm. **Location**: Implementation
  Approach. Add a drafting directive: keep only non-obvious *why* comments (GC'd
  nightly fatal path, mise-skips-components gotcha, call-time env read), prune the
  rest.

- 🔵 **Code Quality**: the deny/pup fixtures don't state whether they exercise the
  *real* `cli/deny.toml`/`cli/pup.ron` or a copy, nor how they control cwd/config
  path — brittle (breaks on unrelated edits) or vacuous (passes against a stale
  copy). **Location**: Phase 3/4. Exercise the real config against a minimal
  generated fixture and state the cwd/config handling.

- 🔵 **Standards**: `cli/rustfmt.toml`'s `max_width = 80` becomes a fourth
  hand-maintained copy (with .editorconfig, pyproject.toml, server/rustfmt.toml)
  without acknowledging the sync obligation. **Location**: Phase 2 §1. Note it is a
  hand-duplicated copy per the documented no-auto-sync convention.

- 🔵 **Standards / Usability**: new CI jobs `check-supply-chain` /
  `check-architecture` name the concern, not the component/task, breaking the
  mechanical `check-\<component\>` → local-command mapping. **Location**: Phase 3
  §5, Phase 4 §6. Either rename to `check-deny`/`check-pup`, or add a CI-job →
  `mise run` mapping table and justify the concern-named departure.

- 🔵 **Architecture / Standards**: the deny/pup leaves `cd cli/` while every other
  cargo leaf uses `--manifest-path`, splitting the convention within one component.
  **Location**: Phase 3 §3, Phase 4 §4. Largely forced (config is cwd-relative) —
  note it as deliberate in the leaf docstrings.

- 🔵 **Architecture**: version-coherence machinery couples to a manifest contract
  owned by 0163 with no in-plan guard on its shape. **Location**: Phase 1 §2-4.
  Have Phase 1 tests assert the *contract* (virtual workspace, `members` present,
  `[workspace.package].version` present) rather than a fixed member set.

- 🔵 **Architecture**: deny.toml evaluates a graph keyed on release triples the
  cli/ workspace does not yet build, so the native-tls ban guards a hypothetical
  multi-triple build. **Location**: Phase 3 §2. Note the musl-static guarantee the
  ban stands in for is not itself exercised in 0162.

- 🔵 **Standards**: the `tasks/README.md` per-component table's model (each
  `\<component\>:check` folds format+lint) doesn't capture that the cli component
  also owns standalone `deny:check`/`pup:check` wired straight into top-level
  `check`. **Location**: Phase 2 §6 / Phase 4 §7. Explain the Rust component spans
  `cli:check` plus standalone entity tasks.

- 🔵 **Compatibility**: cargo-deny is pinned `0.19.9` vs luminosity's validated
  `0.19.8`, an unremarked patch bump against a version-sensitive config schema.
  **Location**: Phase 3 §1. Align to 0.19.8 or record that 0.19.9 was validated
  against the deny.toml schema.

- 🔵 **Compatibility**: the dated `nightly-2026-01-22` + cargo-pup 0.1.8 pair is a
  hard external dependency with a finite GC window and no stable-lane fallback.
  **Location**: Phase 4 §3; Assumptions. No code change — ensure the bump procedure
  and GC-window liability land in the planned docs.

- 🔵 **Usability**: no single orientation for a contributor new to the nightly
  lane (why a second toolchain, that it's isolated, the multi-minute first-run
  build). **Location**: Phase 2 §6. Add a short tasks/README.md subsection
  mirroring the "Executable-bit invariant" style.

- 🔵 **Usability**: the new `ACCELERATOR_*` toggles skip the repo's README
  env-var-table convention where contributors already look. **Location**: Phase 4/5
  docs. Add them to a README env-var table.

- 🔵 **Usability**: `lint:cli:fix` (cargo clippy --fix) only mechanically fixes a
  subset (e.g. `unwrap_used` can't auto-rewrite); no "run cli:check for the
  remainder" caveat is carried over from the server pattern. **Location**: Phase 2
  §4/§6. Add the one-line caveat mirroring `lint:server:fix`.

- 🔵 **Security**: aqua/cargo tool pins are version-only; no committed `mise.lock`
  records artifact hashes, a weaker posture than the repo's SHA-256/SLSA bar for
  its shipped binary. **Location**: Phase 3/4/5 §1. Consider a committed
  `mise.lock` to hash-pin the aqua artifacts.

- 🔵 **Security**: the pinned nightly + from-source cargo-pup are the
  least-verifiable components, and with the grep floor rejected, an
  `ACCELERATOR_PUP_MODE=warn` CI default or a nightly outage leaves the inward rule
  unenforced. **Location**: Phase 4 §3. Ensure `warn` cannot be the CI default
  (fail-closed to deny in `check-architecture`).

#### Suggestions

- 🔵 **Architecture**: Phase 4 embeds an unresolved behavioural spike ("verify the
  installed binary first") inside an "independently mergeable" phase; the whole
  deny-mode model depends on its outcome. **Location**: Phase 4 §1. Resolve
  cargo-pup's exit-code behaviour as a prerequisite spike, or scope the spike as
  the phase's mandatory first step with the documented `assert_lints` fallback.

- 🔵 **Security**: the infra-out-of-domain ban ships inert until 0166, so the
  cross-crate direction rule is unenforced by any mechanism in the interim.
  **Location**: Phase 3 §2. No change for this plan (co-verified at 0166) — ensure
  0166 is a hard prerequisite before any second infra-bearing crate lands.

### Strengths

- ✅ The two-mechanism enforcement design (cargo-deny between crates via global
  ban + `wrappers`; cargo-pup inside a crate at module granularity) is sound and
  non-redundant, matching the granularities the rules actually operate at.
- ✅ Nightly-lane isolation is a genuine structural boundary: rustup-managed (never
  a mise `[tools]` pin), reachable only via `check-architecture`, kept out of the
  `test:integration` roll-up, and backed by a topology guard.
- ✅ Phase decomposition respects check/test disjointness and independent
  mergeability; each phase adds one enforcement family and leaves `check` and the
  bare `default` green.
- ✅ The version-coherence generalisation is well-shaped for evolution: a single
  write site at `[workspace.package].version` while the read side enumerates
  members, catching a member that opts out of inheritance and drifts.
- ✅ Task naming rigorously follows the `tasks/README.md` contract (entity-first
  roll-ups, family-first leaves, no `\<component\>:fix` roll-up), and module names
  avoid the underscore-prefix ban.
- ✅ Test-first ordering is explicit and file-level throughout; both enforcement
  fixtures cover the positive and negative case.
- ✅ The `pup_mode()` fail-closed default and the `_pup_already_installed`
  token-equality probe are faithfully and correctly mirrored.
- ✅ The supply-chain gate is well-conceived: crates.io-only sources, a
  permissive-only license allow-list, a fresh-fetched RustSec DB, and a native-tls
  ban proven by an automated regression rather than by inspection.
- ✅ The brownfield adaptation is respected: the standalone visualiser server crate
  stays separate, the cli/ workspace is additive, and coherence spans both
  manifests plus plugin.json/checksums.json.
- ✅ New CI jobs are least-privilege by construction (no token, no secrets, no
  release concurrency group).

### Recommended Changes

1. **Fix the two tool-pin defects** (addresses: cargo-pup `[tools]` critical;
   nextest aqua pin critical). Remove `"cargo:cargo-pup" = "0.1.8"` from `[tools]`
   entirely (provision only via `deps:install:pup`); change the nextest pin to
   `"aqua:nextest-rs/nextest/cargo-nextest" = "0.9.138"`. These are install/load
   blockers.

2. **Resolve the CI toolchain-caching strategy explicitly** (addresses: RUSTUP_HOME
   workaround major). Either apply the `RUSTUP_HOME` routing + per-job
   `cache_key_prefix` to `check-cli`, `check-supply-chain`, `check-architecture`,
   and the extended test jobs, or document why `deps:install:rust-components`
   closes the race without it — and reconcile the server job so one rationale
   stands.

3. **Redesign the version-coherence folding and guard its boundaries** (addresses:
   folding major; missing-manifest guard major; `_CLI_WORKSPACE_CARGO_TOML_RELATIVE`
   minor; default-arg minor; tomllib import minor). Pick one filtering
   representation, raise a clear error on a missing `[workspace.package].version`,
   test the empty-members boundary, define the relative-path constant, add the
   `tomllib` import, and label the path as original design (not a luminosity
   mirror).

4. **Harden both fixture regressions** (addresses: pup assertion major; deny
   hermeticity major; pup runs only one job major; isolation guard major; fixture
   config coupling minor). Assert pup on the concrete exit-code + message contract
   from Step 1; commit a `Cargo.lock` for the deny fixture (offline, named-crate
   assertion); add a `test_workflows.py` assertion that `check-architecture`
   invokes both `pup:check` and `test:integration:pup`; define the nightly-consumer
   detection rule with known-bad mutations; exercise the real `cli/deny.toml` /
   `cli/pup.ron`.

5. **Pin the cargo-deny advisory schema** (addresses: advisory defaults major). Set
   `version = 2` and state the vulnerability/severity policy explicitly rather than
   relying on version-sensitive defaults.

6. **Close the documentation and DX gaps** (addresses: ACCELERATOR_COVERAGE
   undocumented major; GC'd-nightly message major; 80-col pup.ron major; nightly
   onboarding minor; README env-var table minor; lint:cli:fix caveat minor; CI-job
   naming minor). Add a Phase 5 docs change for `ACCELERATOR_COVERAGE`; specify the
   GC'd-nightly error text as a deliverable; hold `cli/pup.ron` to 80 cols; add a
   nightly-lane orientation subsection and a CI-job → local-command mapping;
   document the toggles in the README env-var table.

7. **Confirm cross-platform tooling on macos** (addresses: test-unit macos major).
   Add a success criterion that all three aqua/cargo tools resolve on
   ubuntu+macos and `test:unit:cli` coverage runs green on the macos leg.

8. **Tighten the phase-independence and enforcement-window wording** (addresses:
   order-independence minor; single-crate vacuous-enforcer major; deny graph
   targets minor; pup.ron module scope minor). Reword "order-independent" to
   "mergeable on top of prior merged state"; state the interim inward-rule exposure
   in Desired End State; give the exact deny target-triple list; author pup.ron
   against real scaffold module names.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally strong — correct separation of the two enforcement
mechanisms, genuine structural nightly-lane isolation, and phase decomposition
that respects independent mergeability and the check/test invariant. Principal
risks: the CI toolchain-cache divergence (RUSTUP_HOME workaround dropped for
rust-cache), a single-crate window where the sole inward enforcer passes
vacuously, and coupling to the 0163-authored manifest contract.

**Strengths**:
- Two-mechanism design (cargo-deny between crates, cargo-pup inside) is sound and
  non-redundant.
- Nightly-lane isolation is structural (rustup-managed, out of test:integration,
  topology-guarded), not just naming.
- Phase decomposition respects check/test disjointness and leaves check + default
  green each phase.
- Version-coherence generalisation scales to N crates with no per-member wiring.
- Brownfield adaptation respected (standalone server crate stays separate).

**Findings**:
- 🟡 major (high): new cargo CI jobs adopt Swatinem/rust-cache but drop the repo's
  load-bearing RUSTUP_HOME toolchain-race workaround, creating two divergent
  caching strategies (Phase 2 §5 / Phase 4 §6 / Phase 5 §3).
- 🟡 major (medium): in the single-crate bootstrap the sole inward-dependency
  enforcer can pass vacuously, so the architecture guard is structurally inert
  until 0163/0166 supply real modules (Desired End State; Phase 4 §4).
- 🔵 minor (high): version-coherence machinery coupled to a manifest contract owned
  by 0163, with no in-plan guard on the contract shape (Phase 1 §2-4).
- 🔵 minor (medium): cargo invocation convention split across the cli component —
  --manifest-path for format/lint, cd for deny/pup (Phase 2 §3 vs Phase 3/4).
- 🔵 minor (medium): deny.toml evaluates a graph keyed on release triples the cli/
  workspace does not yet build (Phase 3 §2).
- 🔵 suggestion (medium): Phase 4 embeds an unresolved behavioural spike inside an
  "independently mergeable" phase (Phase 4 §1).

### Code Quality

**Summary**: Well-structured, phased for mergeability, faithful to luminosity's
leaf+aggregate pattern where it exists. Main risks: the net-new Python it invents
(member-enumeration / coherence helpers luminosity has no analogue for), a
persistent re-derived-manifest-literal DRY smell left partially unaddressed, and
inconsistent comment-density guidance versus the low-comment norm.

**Strengths**:
- Faithfully copies the leaf-executor + mise-depends-aggregate separation.
- Extracts `tasks/shared/rust.py` for cross-cutting constants.
- `pup_mode()`/`coverage_enabled()` correctly specified as call-time env reads;
  `pup_mode()` fails closed with a visible warning.
- `deps:install:pup` fail-loud design gives a diagnosable error.
- TDD ordering called out per phase.

**Findings**:
- 🔴→🟡 major (high): `_read_workspace_version`/`_member_version_mismatches` are
  net-new (not a luminosity mirror), and folding a pre-filtered mismatch dict into
  the re-diffed `found` dict mixes two representations (Phase 1 §3). [merged with
  Correctness folding finding]
- 🟡 major (high): test/unit.py + test/integration.py manifest literals left
  un-consolidated while a parallel cli constant is added to the same paths.py
  (Phase 1/5).
- 🔵 minor (medium): six-plus new cargo leaves repeat the run/exit boilerplate;
  consider a shared `run_or_exit` (Phase 2/3/5).
- 🔵 minor (medium): ported luminosity comments clash with the low-comment norm;
  add a keep-only-non-obvious-why directive (Implementation Approach).
- 🔵 minor (medium): deny/pup fixtures don't state real-config-vs-copy or
  cwd/config handling — brittle or vacuous (Phase 3/4).
- 🔵 minor (low): `cli_member_manifests` absolute-default arg never exercised;
  a rootless test reads the real repo manifest (Phase 1 §2).

### Test Coverage

**Summary**: Unusually test-conscious for infra work — test-first files named for
every coherence/topology change, and both enforcement gates backed by
positive+negative fixtures. Gaps: hermeticity/determinism of the two fixture
suites, a pup assertion admitted to be behaviourally unverified, and several
load-bearing behaviours left to one-shot commands or Manual Verification.

**Strengths**:
- Test-first ordering explicit and file-level.
- Both fixtures cover positive and negative cases.
- Version-coherence tests target the exact opt-out-and-drift edge with specific
  assertions.
- Extends the strong existing test_workflows.py guard pattern.
- Coverage kept report-only with an explicit not-gated check and an
  ACCELERATOR_COVERAGE=off path.

**Findings**:
- 🟡 major (high): cargo-pup regression's core assertion behaviourally unverified
  until implementation (Phase 4 Step 1 + Step 5).
- 🟡 major (medium): deny native-tls fixture may not be hermetic/deterministic
  (network-resolved graph in the ubuntu+macos matrix) (Phase 3 Step 4).
- 🟡 major (medium): pup regression runs only in check-architecture, never on the
  test matrix or locally (Phase 4 Steps 5-6).
- 🟡 major (medium): nightly-lane isolation guard under-specified, no known-bad
  mutations (Phase 4 Step 6).
- 🔵 minor (high): format-drift/clippy gates verified only by one-shot commands,
  not persisted regressions (Phase 2 Success Criteria).
- 🔵 minor (medium): pup_mode()/nightly-unavailable left to Manual Verification
  though unit-testable (Phase 4 Manual Verification).
- 🔵 minor (medium): edition-guard test asserts trivially in the Phase 1→2 interim
  (Phase 1 §5).
- 🔵 minor (low): mutating the shared fake_repo_tree could regress existing
  coherence tests silently (Phase 1 §5).

### Correctness

**Summary**: Central coherence logic largely sound — the `isinstance(pinned, str)`
guard correctly distinguishes an inheriting member from a drifting one, and
`pup_mode()` fail-closed is faithful. Main risks: under-specified folding of member
mismatches, unguarded KeyError/empty-members boundaries, the toolchain-install race
addressed only partially, and isolation/ordering invariants stated in prose rather
than encoded.

**Strengths**:
- The `isinstance(pinned, str)` type guard correctly handles the tomllib
  dict-vs-string distinction for inheriting vs pinning members.
- Members with no `[package]` are safely skipped.
- `pup_mode()` fail-closed default correctly separates findings-failure from
  toolchain-unavailable.
- Nightly-lane isolation structurally correct (test:integration:pup out of the
  roll-up).
- The `_pup_already_installed` probe uses token equality, avoiding the
  0.1.80-matches-0.1.8 hazard.

**Findings**:
- 🟡 major (medium): folding pre-filtered member mismatches into the coherence
  `found` dict double-filters against expected (Phase 1 §3).
- 🟡 major (medium): `_read_workspace_version`/member enumeration has no guard for a
  missing/malformed manifest or empty members (Phase 1 §3).
- 🟡 major (medium): new cargo CI jobs drop the RUSTUP_HOME routing the research
  identified as required for the parallel toolchain auto-install race (CI wiring).
- 🔵 minor (high): "Phases 2-5 order-independent" is false for shared mutable files
  (mise.toml, tasks/shared/rust.py) (Implementation Approach).
- 🔵 minor (medium): deny.toml [graph].targets "gnu/darwin dev triples" ambiguous;
  risk of omitting the ubuntu-gnu CI triple (Phase 3 §2).
- 🔵 minor (medium): pup.ron adapted from luminosity may reference module paths
  absent from the accelerator scaffold (Phase 4 §4).
- 🔵 minor (low): `_CLI_WORKSPACE_CARGO_TOML_RELATIVE` used but never shown defined
  (Phase 1 §3).

### Standards

**Summary**: Unusually disciplined about the task-naming contract and
file-organisation conventions — leaf modules, mise aggregates, and family
placement all match the documented grammar, and module names avoid the
underscore-prefix ban. Gaps: the 80-col convention omits pup.ron and doesn't note
the rustfmt.toml copy coherence, a missing tomllib import, and CI job names that
diverge from check-\<component\>.

**Strengths**:
- Task naming rigorously follows the tasks/README.md contract; no \<component\>:fix
  roll-up introduced.
- New module names avoid the underscore-prefix ban and mirror the per-component
  layout.
- Correctly identifies all five registration sites a new component touches.
- check/test disjointness respected.
- deps:install:* naming and depends-only aggregate pattern followed; leaves raise
  Exit(code=1).

**Findings**:
- 🟡 major (high): 80-col hand-duplication convention omits cli/pup.ron
  (Implementation Approach; Phase 4 §4).
- 🔵 minor (high): plan doesn't confirm cli/rustfmt.toml keeps the three (now four)
  80-col copies coherent (Phase 2 §1).
- 🔵 minor (high): cli_member_manifests uses tomllib but paths.py imports none
  today (Phase 1 §2).
- 🔵 minor (medium): CI job names (check-supply-chain, check-architecture) diverge
  from the check-\<component\> pattern (Phase 3 §5, Phase 4 §6).
- 🔵 minor (medium): deny/pup leaves run "from the cli/ dir" while every other cargo
  leaf uses --manifest-path (Phase 3 §3, Phase 4 §4).
- 🔵 minor (medium): the cli row in the per-component README table doesn't capture
  the standalone deny:check/pup:check wired into top-level check (Phase 2 §6 /
  Phase 4 §7).

### Compatibility

**Summary**: Extends a proven luminosity wiring into accelerator's brownfield;
most dependency/version-coherence and platform reasoning is sound. But two tool-pin
declarations diverge from the reference in ways that fail to install or fail to
load, and the macOS/CI-provisioning concerns for the new cargo jobs are
under-specified relative to the research's own load-bearing warnings.

**Strengths**:
- Nightly/cargo-pup ABI coupling correctly recognised as a matched pair with a
  bump-together procedure and fail-on-GC path.
- edition = 2021 against stable 1.90.0 is compatible.
- Version-coherence correctly handles inheritance vs a drifting member.
- Nightly lane designed so an outage fails only check-architecture.
- deny.toml scoped to shipped+dev triples, avoiding false-positive native-tls bans.

**Findings**:
- 🔴 critical (high): `aqua:nextest-rs/nextest` pin drops the required
  `/cargo-nextest` sub-tool path and will not install on either OS (Phase 5 §1).
- 🔴 critical (high): `cargo:cargo-pup` `[tools]` entry contradicts the nightly ABI
  coupling — builds against stable 1.90.0, ABI-mismatched, breaks isolation
  (Phase 4 §2).
- 🟡 major (high): RUSTUP_HOME routing + cache_key_prefix workaround (research calls
  load-bearing for any new cargo job) not applied to the new jobs (Phase 2/3/4/5).
- 🟡 major (medium): test-unit macos leg compiles/coverage-runs Rust for the first
  time; darwin aqua assets + llvm-cov behaviour unverified (Phase 5 §3).
- 🔵 minor (high): cargo-deny pinned 0.19.9 vs luminosity's validated 0.19.8, an
  unremarked bump against a version-sensitive schema (Phase 3 §1).
- 🔵 minor (medium): dated nightly + cargo-pup is a hard external dependency with a
  finite GC window and no stable fallback (Phase 4 §3; Assumptions).

### Usability

**Summary**: Preserves the central DX invariant (fast read-only inner loop; slow
coverage/pup work in test/CI) and surfaces first-run friction and lane isolation
well. But the escape-hatch ergonomics and the nightly bump procedure are
under-specified: ACCELERATOR_COVERAGE is undocumented, the GC'd-nightly recovery
message is never specified, the toggles skip the README env-var convention, and
there's a task/CI-job naming inconsistency and an onboarding tax.

**Strengths**:
- check/test disjointness respected precisely — inner loop stays fast.
- First-run pup-build friction anticipated and presence-probed.
- Task naming largely follows the documented conventions.
- Escape hatches fail-safe by default and fail-closed on unrecognised values.
- Lane isolation verified structurally and manually.

**Findings**:
- 🟡 major (high): ACCELERATOR_COVERAGE escape hatch has no documentation task
  (Phase 5).
- 🟡 major (high): GC'd-nightly recovery asserted actionable but the actual error
  message is never specified (Phase 4 §3).
- 🔵 minor (high): escape-hatch env vars skip the repo's README env-var-table
  convention (Phase 4/5 docs).
- 🔵 minor (medium): CI job / mise task naming inconsistent and only partially
  predictable (Phases 2-4).
- 🔵 minor (medium): thin onboarding for a contributor new to the nightly lane
  (Phase 2 §6).
- 🔵 minor (medium): cli fix ergonomics diverge from the shell-parity model — no
  "fix doesn't resolve everything" caveat (Phase 2 §4).

### Security

**Summary**: A supply-chain/CI-provisioning plan whose core instrument (the
cargo-deny gate) is well-conceived — crates.io-only sources, permissive-only
licenses, native-tls ban. Dominant concern: trust/integrity asymmetry in the new
toolchain (a pinned nightly + from-source cargo-pup with no artifact verification,
against the repo's strict SLSA/SHA-256 posture elsewhere), plus reliance on
version-sensitive cargo-deny advisory defaults.

**Strengths**:
- [sources] restricts to crates.io only — closes dependency-confusion/rogue-git.
- native-tls/OpenSSL ban is a genuine control, proven by an automated regression.
- Nightly lane isolated so a compromised/unavailable nightly gates only
  check-architecture.
- New CI jobs least-privilege (no token, no secrets, no release concurrency group).
- RustSec DB fetched fresh, never cached stale.
- `cargo install --locked` pins cargo-pup's transitive build deps.

**Findings**:
- 🟡 major (high): cargo-pup added as a `cargo:` mise tool has no
  checksum/provenance verification and diverges from the reference (Phase 4 §2).
  [merged into the cargo-pup critical]
- 🟡 major (medium): deny.toml advisory policy relies on version-sensitive defaults;
  no `version = 2` nor explicit vulnerability/severity shown (Phase 3 §2).
- 🔵 minor (medium): aqua/cargo tool pins are version-only; no committed mise
  lockfile records artifact hashes (Phase 3/4/5 §1).
- 🔵 minor (medium): pinned nightly is an unverified-provenance dependency with no
  stable floor if the lane is bypassed (Phase 4 §3).
- 🔵 suggestion (low): infra-out-of-domain ban ships inert; the supply-chain
  direction rule is unenforced until 0166 (Phase 3 §2).

## Re-Review (Pass 2) — 2026-07-02T15:05:42+00:00

**Verdict:** APPROVE

All 8 lenses were re-run against the revised plan, each given its prior findings
to verify plus a mandate to hunt for regressions the edits introduced. **Both
prior criticals and all 14 prior majors are resolved** (two were `[PARTIAL]` after
the first edit pass and have since been closed). The re-review surfaced a set of
new, smaller findings — including two outright errors introduced by the edits —
which have now been fixed. No critical or major finding remains outstanding; the
plan is sound and ready for implementation, with only optional polish deliberately
deferred.

### Previously Identified Issues

- 🔴 **Compatibility/Security**: cargo-pup as a `cargo:` `[tools]` entry — **Resolved** (entry removed; provisioned only via `deps:install:pup`).
- 🔴 **Compatibility**: nextest aqua pin missing sub-tool path — **Resolved** (`aqua:nextest-rs/nextest/cargo-nextest`).
- 🟡 **Architecture/Compatibility/Correctness**: RUSTUP_HOME toolchain-race workaround dropped — **Resolved** (applied to all new cargo jobs; see new cache-namespace finding below, now also fixed).
- 🟡 **Correctness/Code Quality**: version-coherence folding mixed representations — **Resolved** (single filtering site; `_pinned_member_versions` feeds `found`).
- 🟡 **Correctness**: no guard for missing/empty manifest — **Resolved** (workspace-version, empty-members, missing-`members`, and absent-member-file all raise `VersionCoherenceError`; the members-read half was `[PARTIAL]` and is now closed).
- 🟡 **Test Coverage**: pup regression assertion behaviourally unverified — **Resolved** (asserts exit code + message; §1 mandatory prerequisite).
- 🟡 **Test Coverage**: deny fixture not hermetic — **Resolved** (committed offline `Cargo.lock`, named-crate assertion, both-OS offline criterion).
- 🟡 **Test Coverage**: pup regression only in one job — **Resolved** (topology guard asserts `check-architecture` invokes both `pup:check` and `test:integration:pup`).
- 🟡 **Test Coverage**: isolation guard under-specified — **Resolved** (detection rule + parametrised known-bad mutations).
- 🟡 **Standards**: 80-col convention omitted pup.ron — **Resolved**.
- 🟡 **Security**: deny.toml advisory version-sensitive defaults — **Resolved** (`version = 2` on `[advisories]`; and now `[licenses]` too — see new findings).
- 🟡 **Compatibility**: macos test-unit compiles Rust first time — **Resolved** (tool-resolution + macos-green criteria; corroborated by luminosity running identical pins on macos-latest).
- 🟡 **Code Quality**: manifest literals un-consolidated — **Resolved** (Phase 1 §6).
- 🟡 **Usability**: ACCELERATOR_COVERAGE undocumented — **Resolved** (Phase 5 §4).
- 🟡 **Usability**: GC'd-nightly recovery message unspecified — **Resolved** (Phase 4 §3 deliverable; now also unit-tested).
- 🟡 **Architecture**: single-crate vacuous enforcer — **Resolved** (interim-exposure paragraph in Desired End State).
- 🔵 (all prior minors across Standards/Usability/Correctness/Code Quality: cd-vs-manifest-path docstrings, README per-component table, rustfmt copy-coherence, tomllib import, deny target list, pup.ron real module names, `_CLI_WORKSPACE_CARGO_TOML_RELATIVE`, default-arg, edition-guard move, shared-fixture backstop, CI-job mapping, nightly onboarding, lint:cli:fix caveat, comment-porting directive, mise.lock, warn-CI-default) — **Resolved**.

### New Issues Introduced (by the edits) — and their disposition

- 🟡 **Correctness/Test Coverage**: `pup_mode()` test spec said `off`→`warn`, contradicting the fail-closed `deny` default and the documented `warn` hatch — **Fixed** (corrected to default `deny`; `warn`→`warn`; `off`/unrecognised→`deny`).
- 🟡 **Security**: `version = 2` applied to `[advisories]` only, leaving `[licenses]` on version-sensitive v1 defaults — **Fixed** (added `version = 2` to `[licenses]`).
- 🟡 **Compatibility**: committed `mise.lock` cross-arch portability unverified (a lock authored on one arch could dirty the tree / force a fetch on another) — **Fixed** (must carry all target-platform entries; added a both-OS clean-`mise.lock` success criterion; scoped the integrity claim to aqua backends).
- 🟡 **Architecture/Compatibility**: new jobs paired RUSTUP_HOME + Swatinem with a shared `cache_key_prefix`, risking cross-job/cross-OS cache collision and diverging from the server job — **Fixed** (each job gets a distinct per-job prefix; noted the two mechanisms cache disjoint artifacts and that the server job is knowingly left as-is pending 0168).
- 🟡 **Usability**: toolchain docs fragmented across five surfaces; and the referenced "README `ACCELERATOR_*` env-var table" does not exist (the existing tables are user-facing) — **Fixed** (designated `tasks/README.md` as the canonical home with CLAUDE.md/README as pointers; established a new contributor env-var table there).
- 🟡 **Test Coverage**: pup config-placement-across-members recorded but never asserted; the single-crate pass could be vacuous — **Fixed** (fixture laid out as a member subdir; added a positive control so "passes" means "evaluated and allowed").
- 🔵 **Code Quality**: `_render_workspace_cargo_toml` write path under-specified vs the validating read side — **Fixed** (noted the asymmetry is intentional; read side validates).
- 🔵 **Standards**: new aqua pins omit the why-pinned inline-comment convention — **Fixed** (Phase 3 and Phase 5 note the inline comments mirroring the actionlint precedent).
- 🔵 **Test Coverage**: `coverage_enabled()` and the install_pup failure path left to one-shot/manual — **Fixed** (added to `test_rust.py`/`test_deps.py`).
- 🔵 **Code Quality**: `_read_workspace_version` re-parses the workspace manifest (double open) — **Accepted** (harmless I/O; a `_load_toml` helper is optional polish).
- 🔵 **Test Coverage/Correctness**: format/clippy gates + `check.depends` wiring still verified only by one-shot commands, not a topology guard — **Accepted** (deliberate; the aggregate CI green-bar is the backstop). A `cli:check`/`deny:check`/`pup:check`-appear-in-`check.depends` guard remains available as optional future rigour.
- 🔵 **Security**: tool-version bumps are trust-on-first-use with no upstream attestation anchor — **Accepted as suggestion** (bump procedure is documented; a checksum-verify-at-bump note is optional).

### Assessment

The plan is in good shape and ready for implementation. The first edit pass cleared
every prior critical and major; this pass fixed the two errors it introduced
(`pup_mode` mapping, `[licenses]` schema pin) and tightened the four scope-decision
areas (mise.lock portability, per-job cache namespaces, canonical docs home, pup
positive control). The only residuals are explicitly-accepted optional polish
(a `_load_toml` helper, a `check.depends` topology guard, a bump-time checksum
note) — none blocks implementation.

---
*Re-review generated by /accelerator:review-plan*
