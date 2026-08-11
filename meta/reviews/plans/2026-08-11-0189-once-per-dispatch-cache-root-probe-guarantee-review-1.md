---
type: plan-review
id: "2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-review-1"
title: "Plan Review: At-Most-Once Cache-Root Probe Guarantee Implementation Plan"
date: "2026-08-11T16:21:18+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, test-coverage, code-quality, architecture, standards, documentation, performance, portability]
review_number: 1
review_pass: 6
tags: [cli, launcher, performance, bootstrap]
last_updated: "2026-08-12T00:16:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: At-Most-Once Cache-Root Probe Guarantee Implementation Plan

**Verdict:** REVISE

Phases 1–3 are the strongest part of the plan and are close to ready: the
at-most-once proof was traced correctly against `mod.rs:180-233`, the decision to
introduce a new `PROBES` counter rather than expose `SEQUENCE` is verifiably
right (`SEQUENCE.fetch_add` at `cache_root.rs:114` sits after the
`create_dir_all` early return at `:111-113`), and the proposed test code
type-checks against the real `cache::store` signature with a seeded entry that is
genuinely findable and re-verifiable. Every mise task the plan names exists, and
the gitignore, exec-bit and bashisms claims for Phase 5's scratch script all
check out. The plan repeatedly reads the code rather than trusting its own work
item, and corrects the item where it is wrong.

Phase 5 is where the plan does not yet hold. Its composition budget leaves
exactly one term unmeasured — the cache-hit `reverify` — and that is the only
O(binary-size) term in the budget: `verify_binary` runs a full `sha256_hex` pass
and then a full minisign/blake2b pass over the entire cached binary, which for an
~8 MB artefact plausibly exceeds the ~2.6 ms of headroom the plan computes. The
budget also double-counts the launcher exec, borrows platform-sensitive point
estimates as if neutral, specifies no repository fixture for the shell baseline
whose cost is dominated by fixture-dependent subprocess spawns, and adds no
per-sample validity assertion despite `--fail-safe` making the failure path the
fast path. The gate is arithmetically likely to fail before a sample is taken,
and the pre-decided overrun policy terminates every branch in recording rather
than action.

Three defects also cut across the test work: a false rationale that hides a real
isolation dependency, one uncovered production branch into the probe, and a
non-hermetic discriminating test that corrupts a neighbouring test when its
unstated precondition breaks.

### Cross-Cutting Themes

- **The delta rationale is factually wrong, and it hides a real dependency**
  (flagged by: Correctness, Test Coverage, Architecture) — the plan asserts three
  times that the delta form "is the only form that survives a bare `cargo test`
  in one shared process". It does not: under `cargo test`, libtest runs test
  functions as parallel threads in one process, so a delta around
  `harness.resolve()` absorbs concurrent increments from
  `two_concurrent_first_use_resolves_both_succeed` and every other resolving
  test. What actually makes the assertions sound is nextest's process-per-test
  model — an unrecorded environmental dependency. `tasks/test/cli.py:27`
  explicitly documents that features are gated off by default "so `cargo test`
  stays runnable", so the unsafe invocation is a supported one. The work item's
  own precondition offered two ways to discharge this ("in its own test process,
  **or is serialised** against the concurrent-first-use tests") and the plan
  silently drops the second.

- **The discriminating unit test is non-hermetic and damages a neighbour when it
  fails** (flagged by: Portability, Code Quality, Test Coverage, Correctness) —
  `a_probe_against_an_uncreatable_directory_still_counts` hardcodes
  `/nonexistent-acc-parent-dir/cache` and depends on the runner not being root.
  Run as root on Linux, `create_dir_all` succeeds, the assertion fails, and the
  test *creates and leaves behind* `/nonexistent-acc-parent-dir` — which
  permanently breaks `candidate_performs_no_filesystem_write_or_process_spawn`
  (`cache_root.rs:210-228`), asserting `!unwritable_parent.exists()` on the same
  literal path. Behaviour also diverges by platform: macOS denies the create even
  for root, so this stays green on `macos-latest` and goes red on
  `ubuntu-latest`. This is the single test the whole Phase 1 design rests on.

- **One production branch into the probe is left uncounted** (flagged by:
  Correctness, Test Coverage) — the plan claims the five tests cover "every path
  through `FetchVerifyCacheResolver::resolve` that can reach the probe", but both
  refetch tests poison the cached *bytes*, which always routes through the
  `ChecksumMismatch` arm at `mod.rs:203-217`. The sibling plain-cache-I/O arm at
  `mod.rs:222-228` gets no delta test. `each_of_two_cold_misses_…` is a
  repetition of the cold-miss path, not a fourth path, so the coverage claim
  overstates by one.

- **The module doc describes the function being deleted** (flagged by: Code
  Quality, Documentation, Standards) — Phase 3 asserts that removing
  `cache_root::resolve` touches "no documentation", but `cache_root.rs:1-6`
  describes exactly that composition ("`${ACCELERATOR_PLUGIN_ROOT}/bin` when
  writable and exec-capable, else the `ACCELERATOR_CACHE_DIR` override … Read-only
  /noexec roots are probed"). No grep catches it because the header never names
  the qualified path. Its stated precedence is already inverted relative to
  `candidate`, which checks the override first. The plan ships a docs fix for
  identical staleness one directory away.

- **`probes_performed` names the opposite of what it counts** (flagged by: Code
  Quality, Architecture) — the counter increments as the first statement of
  `verify_writable`, so it counts *invocations*, including the case where
  `create_dir_all` fails and no probe is written, chmodded or executed. That case
  is precisely what the plan's discriminating test exists to pin. Two `AtomicU64`s
  with adjacent-sounding names and different meanings now sit in one module — the
  conflation Phase 1 set out to prevent, recreated at the naming layer.

- **The Phase 5 harness is claimed to be embedded but is not** (flagged by: Code
  Quality, Performance, Test Coverage) — "What We're NOT Doing" states "0186's
  harness lived as a heredoc inside its plan and this one does the same", but
  Phase 5 contains only six bullet properties and a cross-reference. 0186's
  script is not adaptable by inspection: it passes no stdin, checks stdout with
  `startswith("accelerator ")`, and swaps variants by `jj file show` of a
  bootstrap revision. Phase 5 needs piped stdin, an envelope validity check and a
  different baseline recovery.

### Tradeoff Analysis

- **Architectural purity vs scope discipline**: Architecture argues the
  process-global counter puts a test-only concept into an adapter's public
  contract, and that the dependency-injection alternative is dismissed by citing
  scope rather than by argument — in a module that already injects for
  testability (`CacheRootConfig`, `with_fetcher`). Code Quality reaches the
  opposite conclusion: the seam is defensible under genuinely tight constraints
  (a cargo feature is refused because CI passes `--all-features`; a port is the
  restructure the work item excludes). Both are right about their own concern.
  The plan's decision is sound, but its *justification* is an appeal to scope.
  Recommendation: keep the seam, replace the scope appeal with an argument, and
  note that the global counter is what forces the new test coupling to
  `cache::store` — a per-resolver counter would not need it.

- **Coverage completeness vs diff width**: Test Coverage and Correctness both
  want a sixth delta test on the benign-I/O arm; the plan's stated instinct is to
  keep the diff narrow and leave existing poisoning tests untouched. The
  arrangement already exists in
  `a_signature_read_io_error_propagates_the_refetch_error_verbatim`, so the cost
  is low. Recommendation: add it, or drop the "every path" claim.

- **Measurement rigour vs the cost of one session**: Performance wants dispersion
  statistics, paired differences, a bootstrap CI, in-session re-measurement of
  every borrowed term, and quiescence capture. That is a materially bigger job
  than the plan scopes. But the decision being made is whether one median exceeds
  another by 10%, and 0186's own numbers on this host show a near-2× p90 tail.
  Recommendation: since the harness already interleaves, adding variants and
  percentiles to the same run is cheap; adding a bootstrap CI is the part worth
  debating.

### Findings

#### Critical

- 🔴 **Performance**: The only unmeasured budget term is the one that scales with
  binary size, and it likely exceeds all stated headroom
  **Location**: Phase 5, Section 4: Composition Budget
  The budget leaves the cache-hit `reverify` as "to be measured — ", yet that is
  the only O(binary-size) work in it: `reverify` does `std::fs::read` of the whole
  cached binary, a full sha256 pass (`verifier.rs:36`) and a full minisign/blake2b
  pass over the same bytes (`keys.rs:68`). The plan then asserts the known terms
  "already sum to ~36 ms … against a gate of `1.1 × B ≈ 38.6 ms`" — ~2.6 ms of
  headroom. 0186 measured "staged shim exec + minisign verify of the 7.6 MB
  launcher" at ~6.8 ms; the launcher artefacts in `bin/` are ~8 MB, so a
  comparable `accelerator-vcs` gives a `reverify` term two to four times the
  entire headroom as the *optimistic* case. The budget therefore cannot
  discriminate "composed as expected" from "unbudgeted multi-millisecond
  verification cost", and the gate is arithmetically likely to fail before a
  sample is taken.

#### Major

- 🟡 **Correctness / Test Coverage / Architecture**: The delta form does not
  survive a shared-process test run, as the plan claims it does
  **Location**: Implementation Approach; Key Discoveries; Testing Strategy
  See the cross-cutting theme above. The five `assert_eq!` assertions are correct
  only because nextest gives one OS process per test, and nothing in the repo
  pins that — the plan itself notes there is no `nextest.toml`. Under `cargo
  test` these tests are non-deterministic and fail with a bare `left: 4, right:
  1` that reads as a real at-most-once regression.

- 🟡 **Portability / Code Quality / Test Coverage / Correctness**:
  `a_probe_against_an_uncreatable_directory_still_counts` is privilege-dependent
  and leaves residue that breaks a neighbouring test
  **Location**: Phase 1, Section 1: The discriminating unit tests
  See the cross-cutting theme above. Additionally, the plan mis-attributes the
  precondition: it says the test "shares the non-root assumption already carried
  by `candidate_performs_no_filesystem_write_or_process_spawn`", but that test
  performs no I/O and passes for any uid. The genuine carriers are the `0o555`
  read-only tests — one of which Phase 3 deletes.

- 🟡 **Correctness / Test Coverage**: The benign-cache-I/O refetch arm is a third
  branch to the probe and gets no delta test
  **Location**: Phase 2, Section 2: The five delta tests; Testing Strategy
  See the cross-cutting theme above. A future change adding a probe on that
  branch specifically — plausible, since it reads and rewrites the cache — would
  pass all five new tests.

- 🟡 **Code Quality / Documentation / Standards**: Deleting `cache_root::resolve`
  leaves the module doc describing it
  **Location**: Phase 3, Section 1: Delete the wrapper
  See the cross-cutting theme above.

- 🟡 **Code Quality / Architecture**: `probes_performed` names the opposite of
  what its own discriminating test pins
  **Location**: Phase 1, Section 2: The counter and its accessor
  See the cross-cutting theme above. Suggested names: `probe_attempts_made()`,
  `verify_writable_calls()`, or `verifications_attempted()`.

- 🟡 **Code Quality / Performance**: Phase 5 claims to embed a harness like 0186
  but supplies only a prose description
  **Location**: Phase 5, Section 3: Measurement method
  See the cross-cutting theme above. 0189's requirements exceed what 0186's
  heredoc does — byte-identical stdin, precondition assertions, host/OS/chip
  capture — none of which that script performs.

- 🟡 **Correctness / Performance**: The composition budget double-counts the
  launcher exec
  **Location**: Phase 5, Section 4: Composition Budget
  Row 1 is labelled "Warm bootstrap (`bin/accelerator` → verify shim → launcher
  exec) ~29.92 ms" — by its own description already including the exec — and row
  2 then adds "Launcher process exec ~2.4 ms". 0186's 29.92 ms is a whole-run
  median whose own decomposition includes "launcher exec ~2.4". The mixed units
  (one whole-run figure among marginal costs) is what let it slip in, and the same
  confusion buries the harness floor (1.60 ms) and bash startup (6.10 ms) inside
  one row. The arithmetic also quietly uses the low end of the 3.6–4.7 ms range,
  so "~36 ms" is really a 35.9–37.0 ms band.

- 🟡 **Standards**: The 0169 backfill targets a file with no Validation Results
  section
  **Location**: Phase 5, Item 5: Where the figures land; References
  Phase 5 instructs backfilling "0169's five `_pending_` slots … in
  `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`". Verified:
  that file has no `## Validation Results` heading and zero `_pending_`
  occurrences. The slots live in `meta/work/0169-vcs-subdomain-and-hooks-migration.md:700-701`.
  The step that exists to stop the obligation being orphaned a second time points
  at a document where it cannot be found.

- 🟡 **Documentation**: The same falsified read-only claim survives in
  `CHANGELOG.md`
  **Location**: Phase 3, Section 3: Correct the read-only install documentation
  `CHANGELOG.md:26-28` still reads "a cache directory populated once may
  afterwards be read-only for warm invocations (dispatching a subcommand to a
  separate binary still needs it writable)". Verified. That parenthetical is false
  for the same reason `internals.md:277-280` is. The changelog is what a user
  reads at upgrade time and it ships with 1.24.0; 0186 documented this behaviour
  in both files as a pair, so fixing one leaves them contradicting each other.

- 🟡 **Test Coverage**: The five integration tests have no red step; the mutation
  is used as post-hoc evidence
  **Location**: Phase 2 Success Criteria; Phase 1 Success Criteria
  Phase 1 merges independently, so when Phase 2 writes the tests they compile and
  pass on first run. The criterion "All five tests fail before Phase 1's accessor
  exists" is unachievable as written. The only state in which these tests are ever
  red is the mutation, applied *after* they are green — the characterisation-test
  failure mode where a mis-wired assertion is invisible because nothing forced it
  to fail. Suggested fix: apply the mutation *first*, observe each test red, then
  revert to green. That is a genuine red-green loop and yields the same recorded
  evidence.

- 🟡 **Test Coverage**: The warm-hit delta test — the property that matters — is
  never mutation-demonstrated
  **Location**: Phase 2, Section 3: The mutation exercise
  The plan applies one mutation and explicitly expects
  `a_warm_hit_never_probes_the_cache_root` to stay green. But that delta of 0
  protects the property 0169 actually delivered, and it is the only one of the
  five whose ability to fail is left undemonstrated. Add a second one-line
  mutation — `verify_writable` as the first statement of `ResolveBinary::resolve`
  — and record that the warm-hit test goes red under it.

- 🟡 **Test Coverage**: Deleting `a_writable_plugin_root_is_used` loses the only
  assertion that `verify_writable` creates a missing directory
  **Location**: Phase 3, Section 2: Re-home two tests, delete two
  The plan's mapping misses a third assertion the test carried: it passed a bare
  temp dir as `plugin_root`, so the probed `temp/bin` did not exist, and the test
  passed only because `probe_writable_and_executable` calls `create_dir_all`.
  `verify_writable_accepts_a_writable_directory` passes an already-existing
  tempdir. After Phase 3 nothing exercises `verify_writable` against a
  non-existent directory, so the documented create-if-needed behaviour becomes
  untested — and work item AC 8 requires *each* deleted assertion to be
  discharged.

- 🟡 **Test Coverage**: `skip_if_no_minisign!` turns all five guard tests into
  silent passes, including during the mutation exercise
  **Location**: Phase 2, Section 2: The five delta tests
  The macro prints to stderr and `return Ok(())`, which nextest reports as PASS.
  Run the mutation exercise in a shell without the mise-provisioned `minisign` and
  all five tests pass under the mutation — the recorded evidence would show the
  exact opposite of what the plan intends, and "warm hit stayed green" becomes
  indistinguishable from a skip.

- 🟡 **Architecture**: Process-global counter puts a test-only concept in an
  adapter's public contract, and the DI alternative is dismissed by scope
  **Location**: Implementation Approach; Phase 1; What We're NOT Doing
  See Tradeoff Analysis. The concrete cost: because the count is process-global
  rather than per-resolver, Phase 2 must additionally retain `asset_sig` on
  `Harness` and add `seed_cache` reaching into `cache::store` — test-only coupling
  to the cache's on-disk layout that an injected counter would not require.

- 🟡 **Architecture**: "Structural" is overstated — deleting a dead wrapper
  constrains nothing
  **Location**: Overview; Desired End State; Phase 3
  `cache_root::resolve` has no production caller, so deleting it is dead-code
  removal. `verify_writable` remains `pub` and callable from any launcher module;
  the invariant continues to live in `resolve`'s control flow, guarded only by
  tests — as Phase 2's own one-line mutation demonstrates. Either reword to what
  is achieved, or take the genuinely structural lever: narrow `verify_writable` to
  `pub(in crate::launch::outbound::resolve)`, making an outside probe a compile
  error.

- 🟡 **Architecture**: The likeliest regression site is the composition root,
  which none of the five tests observe
  **Location**: Phase 2; Phase 3
  All five tests wrap `FetchVerifyCacheResolver::resolve`. The deleted
  `cache_root::resolve` was an eager fail-fast composition, and the natural place
  to reintroduce it is `main.rs:65`, which already calls `cache_root::candidate`
  and needs one more line to probe. A probe there is invisible to all five tests
  yet breaks the per-dispatch invariant the work item states.

- 🟡 **Documentation**: The work item is left defining "probe count" as the
  counter the plan abandons
  **Location**: Phase 1; Phase 5, Section 2
  The item's definitional sentence (`:37-38`), its Open Question default
  (`:178-182`) and its Technical Notes (`:241-245`) all name `SEQUENCE`. The plan
  correctly finds that false but schedules no correction — even though Phase 5
  already opens the item to edit its Dependencies bullet. Every count acceptance
  criterion is phrased as "the probe count delta", a term the item defines as a
  counter the implementation deliberately does not use.

- 🟡 **Documentation**: The replacement paragraph drops the operational verdict
  its section exists to give
  **Location**: Phase 3, Section 3
  The rewrite enumerates which dispatches probe but never says what *happens* on a
  cold dispatch against a read-only cache (`CacheRootUnavailable`) or what the
  operator should do (pre-warm, or point `ACCELERATOR_CACHE_DIR` somewhere
  writable). It also defines "warm dispatch" only by exclusion, so a reader may
  take it as "cache hit" and conclude a read-only cache is always safe on a hit —
  untrue when re-verification fails.

- 🟡 **Standards**: Absolute production URL in SKILL.md deviates from the
  docs-site cross-link convention
  **Location**: Phase 4, Section 1
  SKILL.md bodies are rendered verbatim into docs-site pages
  (`tasks/shared/skill_pages.py:174`), so the link lands on
  `docs-site/…/reference/skills/visualisation/visualise.md` — a site page linking
  absolutely off-site to itself. Five existing pages use
  `[Terminal Invocation](internals.md#terminal-invocation)`. It also duplicates
  the hosting decision `docs-site/astro.config.mjs:9-12` documents as living
  there alone. The hook message can keep the absolute URL; it never reaches the
  site.

- 🟡 **Portability**: "The `cache_root` test module is already unix-only" is
  inaccurate, and Phase 3 erodes its evidence
  **Location**: What We're NOT Doing
  Verified: the test module at `cache_root.rs:142` carries only `#[cfg(test)]`.
  The `#[cfg(unix)]`/`#[cfg(not(unix))]` pair at `:130`/`:137` gates production
  `make_executable`. The module is unix-only only in that it *fails to compile*
  elsewhere, via two unconditional `use std::os::unix::fs::PermissionsExt`
  statements inside test bodies — one of which Phase 3 deletes, leaving the
  property resting on a single incidental import.

- 🟡 **Portability**: Darwin-only measurement with a platform-specific budget
  presented as neutral
  **Location**: Phase 5, Sections 3-4
  The budget's inherited terms are darwin-arm64 figures, but the bootstrap selects
  `sha256sum` when present and falls back to `shasum -a 256` — a difference this
  repo has measured as ~3.5 ms versus ~11.7 ms. The signed manifest publishes
  `linux-x64` and `linux-arm64`, and CI runs the cli suite on `ubuntu-latest` too,
  so the gate closes on one of four shipped platforms with no linux handoff.

- 🟡 **Performance**: No repository fixture or cwd is specified, and `B` is
  dominated by fixture-dependent subprocess spawns
  **Location**: Phase 5, Section 3
  The recovered baseline sources `scripts/vcs-common.sh`, whose
  `classify_checkout` (`:177`) spawns `jj workspace root`, two `git rev-parse`
  forms, `realpath` and repeated `command -v` probes — *which* of these run
  depends on the checkout kind. 0188 measured a single `jj` subprocess at 7.05 ms
  and `git rev-parse` at 4.40 ms on this host, so fixture choice can shift `B` by
  more than the whole 10% margin, in the direction that makes the gate easier to
  pass. The plan lists "payload + fixture" as a slot to backfill while never
  specifying the fixture.

- 🟡 **Performance**: No per-sample validity assertion, and `--fail-safe` makes
  the failure path the fast path
  **Location**: Phase 5, Section 3
  `swallow_under_fail_safe` (`main.rs:212-220`) exits 0 on an availability failure
  *without ever exec'ing the sub-binary*, so any degraded sample skips the whole
  `reverify` + exec cost and records a spuriously **low** latency. 0186's harness
  guarded exactly this by aborting on any sample whose stdout did not start with
  `accelerator `; Phase 5 inherits the shape but not the guard. The gate could
  record a PASS produced by 50 fast failures.

- 🟡 **Performance**: Borrowed 0186/0188 terms are not transferable without
  re-measurement
  **Location**: Phase 5, Section 4
  The 29.92 ms term was measured at `v1.24.0-pre.21`; the tree is at `pre.36`.
  0186 explicitly recorded its `sha256_file` cost as a range and stated "0169 is
  handed the range and the backend, not a point estimate" — the plan borrows the
  point estimate. The 3.6–4.7 ms term comes from 0188's *prototype fixture*
  binary, which 0188 said "Phase 4 must re-measure against the delivered
  two-binary shape". A single unrecorded backend difference moves the budget by
  ~17 ms, four times the gate's margin.

- 🟡 **Performance**: Medians alone cannot support a 10% gate given this
  harness's known dispersion
  **Location**: Phase 5, Success Criteria
  No sample count, min/p90/IQR or confidence interval is required, despite
  claiming to inherit 0186's harness, which printed `min / median / p90 /
  median-minus-floor / n`. 0186's own numbers: `before` ran min 119.02, median
  125.35, **p90 234.15**; even the quiet `after` spanned 27.18–32.44 around a
  29.92 median (±8%). The decision is whether one median exceeds another by 10%.
  Because samples are interleaved *pairs*, the plan discards the one statistic
  that would resolve this cheaply.

- 🟡 **Performance**: The pre-decided overrun policy leaves no outcome that
  triggers action
  **Location**: Phase 5, Section 4
  Every branch terminates in recording. The only sub-check with teeth is the ≤25%
  residual, which is a self-consistency check on the budget's own bookkeeping, not
  a check on `G`. Combined with the critical finding, the gate cannot fail in any
  consequential sense — which is how an obligation already deferred once gets
  orphaned a third time.

#### Minor

- 🔵 **Correctness / Standards**: The Phase 3 deletion criterion is vacuous
  **Location**: Phase 3, Success Criteria
  Verified: `rg -n 'cache_root::resolve' cli/ docs-site/ skills/ hooks/ scripts/`
  already returns nothing, because the function is only ever referenced
  unqualified — `resolve(&config())` inside its own `mod tests` and via `use
  super::{…}` at `:149`. A check that cannot fail certifies nothing, and the same
  pattern fails to substantiate the load-bearing "only four call sites" premise.

- 🔵 **Correctness**: The Phase 1 discrimination step contradicts the "SEQUENCE
  unchanged" guarantee
  **Location**: Phase 1, Success Criteria and Manual Verification
  `SEQUENCE` is declared *inside* `probe_writable_and_executable`'s body
  (`:109`), so no accessor can read it without hoisting the static to file scope —
  contradicting the phase's own manual check that `SEQUENCE` keeps the "same
  declaration, same position". The same criterion also describes the new tests as
  "failing before the accessor exists", but absent `probes_performed` the crate
  does not compile, so the observation is a build error, not a red assertion.

- 🔵 **Documentation**: The `internals.md` replacement range starts mid-sentence
  **Location**: Phase 3, Section 3
  Verified: line 277 begins `invocations. That exemption stops at the bootstrap:
  running any subcommand that`. The word `invocations.` completes the *previous*
  sentence, and the replacement block does not carry it. Applying `:277-280`
  literally ships broken prose.

- 🔵 **Correctness**: `empty_cache` mutates the directory while iterating a live
  `read_dir` stream
  **Location**: Phase 2, Section 1
  POSIX leaves it unspecified whether entries removed after the stream was opened
  are still returned. It also calls `remove_file` unconditionally, failing on a
  directory entry. If the binary entry were skipped, the second `resolve()` in
  `each_of_two_cold_misses_…` would be a cache *hit*, delta 1, failing with a
  message pointing at the probe invariant rather than the helper.

- 🔵 **Test Coverage**: `a_successful_refetch_…` cannot distinguish a refetch from
  a cold miss
  **Location**: Phase 2, Section 2
  A cold miss also yields delta 1 and also returns `Ok`, so if `seed_cache` ever
  stopped producing a findable entry the test would silently degrade into a
  duplicate of the cold-miss test and still pass. Pin the arrangement (`assert!
  (cache::find(…).is_some())` before poisoning) and add an outcome assertion.

- 🔵 **Code Quality / Standards**: Two sibling harness helpers with two error
  types, and one reads as a predicate
  **Location**: Phase 2, Section 1
  `seed_cache` returns `Result<_, ResolutionError>` while `empty_cache`, added in
  the same block, returns `Result<_, Box<dyn Error>>`. `seed_cache` also leaks a
  production domain error from a fixture that performs no resolution. `empty_cache`
  reads as a query rather than the command it is — `clear_cache` states the intent.

- 🔵 **Code Quality**: The accessor's doc comment restates its signature
  **Location**: Phase 1, Section 2
  "The number of probes `verify_writable` has performed in this process" above
  `probes_performed() -> u64` is a restatement — the comment class this repo bans —
  and it is not compiler-forced (`missing_docs` is off). The genuinely non-obvious
  fact (a test-only seam, `pub` solely because the integration tests are a separate
  crate, with no production reader) sits on the private static where tests never see
  it. `cli/vcs-adapters/src/subprocess.rs:53` and
  `cli/work-adapters/src/diff_shellout.rs:38` already document test seams as such.

- 🔵 **Code Quality**: A fourth verbatim copy of the offline-resolver construction
  **Location**: Phase 2, Section 2
  That block already appears at `resolution.rs:465-473`, `:501-510` and `:531-539`.
  `Harness::resolve()` exists precisely to hide the `ExternalCommand` literal, so
  the omission is inconsistent rather than deliberate. A `Harness::resolve_offline`
  beside it would do.

- 🔵 **Standards**: Importing `cache` while leaving the fully-qualified call site
  **Location**: Phase 2, Section 1
  `resolution.rs:17-25` establishes a grouped-import convention with `:521` as the
  single exception — an exception that exists only because nothing had imported
  `cache` yet. After the change the module is reached two ways in one file, and the
  deviation loses its reason. Collapsing `:521` is a two-token edit.

- 🔵 **Standards**: No named command runs the hook tests, and `docs:check` does not
  validate the external anchor
  **Location**: Phase 4, Success Criteria
  `scripts:check` is shfmt + ShellCheck + bashisms + exec-bits only, and `mise run
  check` contains no test tasks (`mise.toml:575-577`); the repo's task is `mise run
  test:integration:hooks`. And `starlightLinksValidator` validates internal links
  with `errorOnRelativeLinks: false` (`astro.config.mjs:74`), so `docs:check` passes
  whether or not `#terminal-invocation` resolves. Phases 1-2 also use
  `ACCELERATOR_COVERAGE=off` while Phase 3 invokes the same task bare, unexplained.

- 🔵 **Standards**: `rtk proxy` — a personal CLI wrapper — leaks into a committed
  criterion
  **Location**: Phase 4, Success Criteria
  `rtk proxy` appears nowhere else in the repository; every other command in the
  plan is `mise run`, `cargo`, `jj` or bare `rg`. A committed plan should not carry
  a criterion only the author's machine can run verbatim.

- 🔵 **Standards**: In-place amendment of closed documents departs from the
  recorded dated-note practice
  **Location**: Phase 5, Items 2 and 5
  The 0189 item records the practice: retractions in 0169 and 0186 were "dated notes
  appended beside the original text rather than edits to it". 0169 is `status:
  done`. Neither sub-step says which form it takes, and neither mentions refreshing
  `last_updated`/`last_updated_by`.

- 🔵 **Documentation**: The work item's own References still points at
  `docs/internals.md`
  **Location**: Phase 4
  `meta/work/0189-…md:296` carries the exact never-existent path this phase exists
  to eliminate, and Phase 4's grep excludes `meta/**`. Relatedly, the item has no
  acceptance criterion for the internals.md correction and Validation Results has
  no slot for the documentation work, so a validator can close 0189 without checking
  it.

- 🔵 **Portability**: Precondition set covers only the two `*_BIN` overrides
  **Location**: Phase 5, Section 3
  `ACCELERATOR_CACHE_DIR`, `ACCELERATOR_PLUGIN_ROOT`,
  `ACCELERATOR_RELEASE_BASE_URL`, the `ACCELERATOR_UNAME_S`/`_M` seams and the
  `/.accelerator-dev-launcher` marker all change what `G` measures. Also `env | grep
  -E …` returns exit 1 on no match, so it cannot be used verbatim under `set -e`,
  and it cannot distinguish unset from empty.

- 🔵 **Portability**: Baseline recovery assumes a jj working copy and POSIX chmod
  **Location**: Phase 5, Section 1
  A contributor on a plain git clone needs `git show cf42441e2aad^:hooks/vcs-guard.sh`.
  The gitignore reasoning itself is verified sound.

- 🔵 **Portability**: `/usr/bin/true` instrument floor is a hardcoded absolute path
  **Location**: Phase 5, Section 3
  Resolves on darwin and merged-`/usr` glibc, not on busybox images. Harmless for the
  planned darwin session, but the harness is what would be re-run for a linux lane.

- 🔵 **Portability**: Counting tests inherit an undocumented exec-capable-`TMPDIR`
  requirement
  **Location**: Phases 1 and 2
  `verify_writable` proves exec-capability by writing and running a `#!/bin/sh`
  script, so every counting test requires an exec-capable `TMPDIR` — failing on hosts
  that mount `/tmp` `noexec`. The coupling is pre-existing but the plan adds several
  instances without naming it, so the failure reads as a counting defect.

- 🔵 **Performance**: Machine quiescence, power and thermal state are unasserted
  **Location**: Phase 5, Section 3; Manual Testing Steps
  On Apple silicon, battery or Low Power Mode schedules short-lived subprocesses onto
  efficiency cores, and a sustained run on a hot M4 Max throttles. Interleaving
  cancels drift between `B` and `G` but cannot rescue the cross-session budget terms
  being compared against this session's `G`.

- 🔵 **Standards**: The scratch script borrows a gitignore glob reserved for
  launcher runtime artefacts
  **Location**: Phase 5, Section 1
  `bin/.tmp-*` sits in a block whose documented scope is what "`bin/accelerator` and
  the launcher write", with an explicit note that patterns were deliberately
  narrowed. A future tightening would silently start snapshotting the benchmark
  script. Architecture separately notes `.tmp-` is the `store::TEMP_PREFIX`
  namespace for in-flight atomic-write temps.

#### Suggestions

- 🔵 **Architecture**: Phases are a stack, not independent — Phase 2 will not
  compile without Phase 1
  **Location**: Implementation Approach
  Reword to "stacked in this order; each is individually shippable and leaves the
  tree green", and state the one hard dependency (2 → 1) plus which phases are
  genuinely order-free (3, 4, 5).

- 🔵 **Architecture**: Phase 4 is unrelated scope with no work-item mandate
  **Location**: Phase 4
  Neither pointer touches the probe, the launcher crate, or any 0189 acceptance
  criterion. Phase 3's `internals.md` fix is different and defensible — that
  paragraph is directly falsified by the behaviour this plan pins. Either lift Phase
  4 out or say why the coupling is worth it.

- 🔵 **Architecture**: Phase 5 binds a CI-verifiable refactor's closure to a
  one-shot single-host measurement
  **Location**: Phase 5
  Phases 1-3 are machine-verifiable; Phase 5 needs a quiet darwin host, a warm real
  cache, a resurrected deleted script and an uncommitted harness. Splitting it lets
  the invariant work close on CI evidence alone.

- 🔵 **Architecture**: The memoisation ban rests on an assumption the plan does not
  carry forward
  **Location**: What We're NOT Doing
  The ban is correct because a launcher process performs one resolution — an
  assumption the work item records and the plan does not. Name the condition under
  which it should be revisited (more than one resolution per process), so the
  criterion pinning it can be retired deliberately.

- 🔵 **Correctness**: The concurrent two-thread path gets no probe-count assertion
  **Location**: What We're NOT Doing; Phase 2, Section 2
  The plan cites `two_concurrent_first_use_resolves_both_succeed` twice as
  justification but adds no observation there. Assert a bounded delta —
  `(1..=2).contains(&delta)` — since the count legitimately depends on whether the
  second thread's `cache::find` observes the first thread's `store`.

- 🔵 **Documentation**: Have the `PROBES` doc comment name the `SEQUENCE`
  distinction
  **Location**: Phase 1, Section 2
  Two `AtomicU64`s in one small module with no stated distinction invites a tidy-up
  merge, which silently breaks the invariant on the create-failure path. One clause —
  invocations, including those that fail before writing — is exactly the
  subtle-invariant "why" the comment policy permits.

- 🔵 **Test Coverage**: The latency baseline becomes unreproducible after the phase
  **Location**: Phase 5
  The baseline subject is recovered into a gitignored path and removed again, so
  nothing in the tree lets `B` be re-derived. Record the harness verbatim plus the
  sha256 of the recovered script, and state explicitly that no automated latency
  ratchet exists.

- 🔵 **Performance**: Name the per-dispatch full-file verification cost as the
  lever, not only 0191's ~2.5 ms
  **Location**: Phase 5, Section 4
  The verify shim hashes the ~7.6 MB launcher twice and minisign-verifies it, then
  the launcher reads and hashes the cached `accelerator-vcs` twice more — on the
  order of 15 MB hashed per hook invocation, and the guard fires on every Bash tool
  call. The sha256 in `verify_binary` is a corruption check strictly subsumed by the
  minisign signature on the cache-hit path; a streamed/mmap digest avoids allocating
  the whole binary. Both are larger wins than 0191 if `reverify` is what closes the
  gap.

- 🔵 **Portability**: Literal `/some-override-dir` is safe only while `candidate`
  stays pure
  **Location**: Phase 3, Section 2
  Safe today — arguably *more* portable than the tempdir version — but contingent on
  `candidate` gaining no canonicalisation or existence check. Carry the plan's prose
  rationale into the test name as intent.

- 🔵 **Portability**: The offline simulation relies on `127.0.0.1:1` not being
  proxied
  **Location**: Phase 2, Section 2
  `Fetcher` builds its client without disabling proxy discovery
  (`resolve/fetcher.rs:88-90`), so on a proxied host the refused connection becomes a
  proxied response and the matched error variant may change. Pre-existing and shared
  with three tests, but this is the fourth copy and its assertion is variant-specific.

### Strengths

- ✅ The at-most-once proof is traced correctly and precisely: all three arms of
  `match self.reverify(&cached)` are `return` expressions, so control cannot fall
  through to the tail call at `mod.rs:231`, and the only retry loop lives in
  `Fetcher::get` and never re-enters `fetch_verify_store`.
- ✅ The plan reads the production code rather than trusting the work item, and
  falsifies the item's own definitional claim that `SEQUENCE` increments once per
  `verify_writable` call — then drives the correction with a discriminating test
  written first.
- ✅ `Harness::seed_cache` type-checks against the real `cache::store` signature,
  and the entry it writes is genuinely findable (`find` strips
  `visualiser-{VERSION}-`, leaving 64 hex chars) and genuinely re-verifiable (the
  sidecar carries the raw `.minisig` text), so the warm-hit delta of 0 means what
  it claims.
- ✅ The failed-refetch test's routing is correct, and the distinction from the
  existing sidecar-corruption test is real: poisoning the binary bytes fails the
  sha256 check first, hitting the `ChecksumMismatch | SignatureMismatch` arm and
  therefore `CorruptCacheAndRefetchFailed`.
- ✅ Correctly refuses a verifier port on evidence — byte poisoning already
  produces both refetch outcomes because the expected sha256 is parsed out of the
  cache filename — avoiding a speculative abstraction.
- ✅ The test seam is chosen under stated constraints rather than by default: a
  cargo feature is rejected with a cited workspace precedent because CI passes
  `--all-features`, and the permitted public-surface growth is held to exactly one
  function.
- ✅ The counting seam is confined to the outbound adapter; `launch::core` and the
  `launch_core_imports_only_permitted` cargo-pup rule are untouched, and no side
  effect is added to any pure computation.
- ✅ The mutation exercise predicts per-case outcomes in advance (2 / 0 / 2 / 2 /
  4) rather than a blanket "tests go red", and names the case expected to stay
  green as a preservation guard to be recorded.
- ✅ Phase 3 records an honest two re-homed / two already-covered mapping rather
  than padding to four redundant tests, and leaves the existing poisoning tests'
  behavioural assertions untouched.
- ✅ Every mise task the plan names exists, `docs:check` is correctly treated as
  outside the aggregate `check`, `ACCELERATOR_COVERAGE=off` is the documented
  fast-loop form, and `test:unit:cli` genuinely does cover `resolution.rs`.
- ✅ The exec-bit-invariant claim for the gitignored baseline is accurate and
  non-obvious: `.gitignore:56` really is `bin/.tmp-*`, and `walk_files()` filters
  on the root gitignore spec, so the shell linters never enumerate it. The hook
  edit trips nothing on the bash-4 denylist and the bash 3.2 floor is untouched.
- ✅ Phase 4's factual survey holds up under verification: exactly two stale
  pointers, the two full-path hits are genuine repo-internal tooling, and
  `## Terminal Invocation` really is at `internals.md:145` so `#terminal-invocation`
  is the right Starlight anchor.
- ✅ The `internals.md` replacement is technically accurate against the resolver:
  `reverify` only reads, and both self-heal arms route to `fetch_verify_store`, so
  classifying failed re-verification as *cold* makes "without writing or probing"
  true rather than glib.
- ✅ Phase 5 inherits the right benchmark mechanics from 0186 rather than 0169's
  bare "median of 20": interleaved sampling, alternating within-pair order, one
  Python process holding the clock, and instrument floors measured in the same run.
- ✅ The Performance Considerations claim is correct and verifiable —
  `verify_writable` is unreachable on a warm hit, so the counter adds nothing to
  the hot path — and refusing memoisation is right on performance grounds too,
  since the path it would optimise is already zero-cost warm.
- ✅ Pre-registering the response to an overrun before seeing the number is the
  right discipline, and naming 0191 as a co-requisite is honest about where the
  remaining milliseconds are.
- ✅ Honest accounting throughout: the false definitional sentence is corrected,
  the stale release-blocked dependency is challenged with named evidence, and the
  `Ordering::Relaxed` justification is sound.

### Recommended Changes

1. **Measure `reverify` before committing to Phase 5's arithmetic** (addresses:
   the critical finding, plus the double-count and borrowed-figures findings)
   A direct in-process timing of `read + sha256_hex + TrustedKeys::verifies` over
   the cached asset, plus the asset's byte size, takes minutes and converts the
   residual into a budgeted term. Record ms-per-MB so it transfers as the binary
   grows. Then re-derive the headroom and decide whether `G ≤ 1.1 × B` is
   reachable with 0191 alone — before the plan commits to the gate.

2. **Fix the composition budget's bookkeeping** (addresses: double-counts the
   launcher exec; borrowed terms not transferable)
   Drop the separate launcher-exec row, label each row as whole-run median or
   marginal cost, carry the `accelerator-vcs` term as a range, and name the
   launcher-internal work currently attributed to nothing. Since the harness
   already interleaves, add `bin/accelerator version` and a direct cached-binary
   invocation as extra variants so every term comes from one session on one host
   and nothing is borrowed.

3. **Correct the delta rationale and pin the isolation it depends on**
   (addresses: the delta form does not survive `cargo test`)
   Replace the false claim with the true one — the counter is process-global and
   these tests depend on nextest's process-per-test model — state that dependency
   on `probes_performed`'s doc comment, and either add a `nextest.toml`
   serial/test-group covering the counting tests plus
   `two_concurrent_first_use_resolves_both_succeed`, or give the assertions failure
   messages naming cross-test interference as the first thing to rule out.

4. **Make the discriminating test hermetic** (addresses: privilege-dependent test
   leaving residue that breaks a neighbour)
   Drive the `create_dir_all` failure from a privilege-independent cause — create
   a regular *file* in a `tempdir()` and target a path beneath it, so it fails with
   `ENOTDIR` for every user including root. Nothing is written outside the temp
   dir, nothing collides with
   `candidate_performs_no_filesystem_write_or_process_spawn`, and the intent is
   stated more directly. Also correct the mis-cited precedent.

5. **Invert the Phase 2 sequence so the tests have a real red step** (addresses:
   no red step; warm-hit never mutation-demonstrated)
   Apply the duplicate-probe mutation *first*, write each delta test and observe it
   red, then revert to green. Add a second mutation — `verify_writable` as the
   first statement of `ResolveBinary::resolve` — and record the warm-hit test going
   red under it. Two mutations demonstrate the full envelope rather than only its
   upper half. Require the recorded output to show no `skipping: minisign not on
   PATH` line.

6. **Add the sixth delta test on the benign-cache-I/O arm** (addresses: uncovered
   third branch)
   Arrange as `a_signature_read_io_error_propagates_the_refetch_error_verbatim`
   does but seeded via `seed_cache`: write invalid UTF-8 over the signature path,
   resolve, assert delta 1. Alternatively, correct the "every path" wording to say
   which branch is unpinned and why.

7. **Keep a replacement for the lost create-if-missing assertion** (addresses:
   deleting `a_writable_plugin_root_is_used`)
   Add `verify_writable_creates_a_missing_directory` — call
   `verify_writable(&temp.path().join("bin"))` and assert both `Ok` and that the
   directory now exists — and record it as the third row of the old-test →
   discharging-test mapping rather than reporting the deleted test as fully covered.

8. **Rewrite the `cache_root` module doc as part of Phase 3** (addresses: stale
   module doc)
   State the two operations the module now offers — selection (override first, else
   `${ACCELERATOR_PLUGIN_ROOT}/bin`) and a probe reached from one production call
   site — keep the no-XDG rationale, record the at-most-once invariant there, and
   add a manual criterion that the header matches the surviving surface.

9. **Rename the counter for what it measures** (addresses: `probes_performed`
   names the opposite)
   `probe_attempts_made()` / `verify_writable_calls()` / `verifications_attempted()`.
   Rename `PROBES` to match, move the test-seam intent onto the accessor, delete the
   signature restatement, and let the doc comment carry the `SEQUENCE` distinction.

10. **Fix Phase 5's backfill target and extend Phase 3 to `CHANGELOG.md`**
    (addresses: wrong backfill file; CHANGELOG falsehood)
    Tick the Phase 10 checkbox in `meta/plans/2026-08-05-0169-…md:1507`, and fill
    the five `_pending_` slots in `meta/work/0169-…md:700-701`; correct the
    References attribution. Separately, amend the `CHANGELOG.md` `[Unreleased]`
    → Changed bullet in the same edit as `internals.md`, and quote the
    `internals.md` replacement range unambiguously (line 277 begins mid-sentence).

11. **Add per-sample validity assertions and dispersion reporting to Phase 5**
    (addresses: no validity assertion; medians alone; fixture unspecified;
    quiescence)
    Assert per sample that exit code and hook envelope match the expected decision
    and that `B` and `G` agree on the same stdin, aborting on first mismatch.
    Specify the fixture (a fresh pure-jj scratch repo) and set both subprocess cwd
    and the envelope's `cwd` to it. Record `n`, min/median/p90, IQR, the median of
    the 50 paired differences, and the resolved digest backend, bash and jj/git
    versions. Inline the harness verbatim rather than cross-referencing 0186.

12. **Give the overrun policy a consequence** (addresses: no outcome triggers
    action)
    Name the figure at which 0191 must land and `G` be re-measured before the
    criterion closes, and a threshold on the measured `reverify` term above which a
    follow-up work item for warm-dispatch verification cost is raised.

13. **Correct the plan's platform and scope claims** (addresses: "already
    unix-only"; darwin-only gate; phase independence; `rtk proxy`)
    Replace the "already unix-only" justification with a deliberate platform-scope
    decision (either `#[cfg(unix)]` on the module, or delete the dead
    `#[cfg(not(unix))]` arm). State that the gate is verified on darwin-arm64 only
    and record a named linux handoff. Reword "each merges independently" to
    "stacked; each leaves the tree green", naming the 2 → 1 dependency. Drop the
    `rtk proxy` prefix. Add `mise run test:integration:hooks` to Phase 4 and
    requalify its `docs:check` criterion.

14. **Decide Phase 4's and Phase 5's placement** (addresses: unrelated scope;
    closure coupling)
    Phase 4 has no work-item mandate and touches two different components; Phase 5
    binds a CI-verifiable refactor's closure to a one-shot single-host measurement.
    Either lift them into their own items, or record explicitly why the coupling is
    worth it.

15. **Amend the work item alongside the plan** (addresses: stale definitional
    sentence; stale References; missing criteria)
    Restate "probe count" as the `PROBES` invocation counter and say why `SEQUENCE`
    cannot serve; resolve the Open Question in the item, not only in the plan; fix
    the `docs/internals.md` References line; add a documentation-corrections row to
    Validation Results; and refresh `last_updated`/`last_updated_by` on every
    touched meta document, using dated notes for retractions in closed documents per
    the practice the item itself records.

## Per-Lens Results

### Correctness

**Summary**: The plan's central structural claim — that
`FetchVerifyCacheResolver::resolve` cannot reach `fetch_verify_store` twice — is
correct as traced against the code, and its key seam decision (a new `PROBES`
counter rather than exposing `SEQUENCE`, because `SEQUENCE.fetch_add` at
`cache_root.rs:114` sits after the `create_dir_all` early return at `:111-113`) is
verifiably right. The proposed test code type-checks against the real
`cache::store` signature, the seeded entry is findable by `cache::find`, and the
failed-refetch test does reach `CorruptCacheAndRefetchFailed`. Three logical
defects remain: one of the three probe-reaching branches is left unasserted despite
the plan claiming full path coverage, the stated justification for the delta form is
false because libtest parallelises within one process, and the Phase 5 composition
budget double-counts the launcher exec term against an ambiguous base.

**Strengths**:
- The at-most-once proof is traced correctly and precisely: all three arms of
  `match self.reverify(&cached)` are `return` expressions, so control cannot fall
  through the `if let Some(cached)` block to the tail call at `mod.rs:231`, and the
  only retry loop lives in `Fetcher::get`.
- Correctly refutes the work item's own definitional sentence: `SEQUENCE`'s
  `fetch_add` runs after the `create_dir_all` early return, so it counts writes
  rather than invocations.
- `Harness::seed_cache` type-checks against the real
  `cache::store(root, name, version, sha256, bytes, signature)` signature, and the
  entry is genuinely findable and genuinely re-verifiable.
- The failed-refetch test's routing is correct and the distinction from the existing
  sidecar-corruption test is real.
- The mutation-exercise arithmetic is right, and the plan records the warm hit
  staying green as a preservation guard rather than a defect.
- The `Ordering::Relaxed` justification is sound — same-atomic read-after-write
  within one thread is coherent regardless of ordering.
- `ResolutionError` implements `std::error::Error` (`core.rs:165`), so `seed_cache`'s
  return type converts via `?` without extra plumbing.

**Findings**:
- 🟡 major (high) — *One of the three probe-reaching branches has no at-most-once
  assertion* — Phase 2, Section 2. `resolve` has three syntactic
  `fetch_verify_store` call sites and only two are exercised; both refetch tests
  poison binary bytes and route through `mod.rs:203-217`, leaving the plain-cache-
  I/O arm at `mod.rs:222-228` unasserted. The plan's own Current State Analysis
  enumerates three call sites, so the exhaustive-coverage claim is false.
- 🟡 major (high) — *The delta form does not survive a shared-process test run* —
  Implementation Approach; Key Discoveries; Testing Strategy. Under `cargo test`
  libtest runs test functions in parallel threads of one process, so a delta absorbs
  concurrent increments. A delta only neutralises probes that happened *before* the
  window on the same thread.
- 🟡 major (medium) — *The composition budget double-counts the launcher exec and
  computes its residual from an ambiguous base* — Phase 5, Section 4. Row 1's own
  description includes the exec; row 2 adds it again. The arithmetic also uses the
  low end of a range, so "~36 ms" is a 35.9–37.0 ms band.
- 🔵 minor (high) — *The deletion criterion is vacuous* — Phase 3, Success
  Criteria. `rg -n 'cache_root::resolve'` already returns nothing; the four call
  sites use the bare form via `use super::{…}`.
- 🔵 minor (high) — *The discriminating-failure verification step contradicts the
  'SEQUENCE unchanged' guarantee and is not a red test* — Phase 1, Success Criteria.
  `SEQUENCE` is declared inside the function body, so no accessor can read it
  without hoisting; and absent `probes_performed` the crate does not compile, so the
  observation is a build error.
- 🔵 minor (medium) — *`empty_cache` mutates the directory while iterating its live
  read_dir stream* — Phase 2, Section 1. POSIX leaves post-open removals
  unspecified; `remove_file` also fails on a directory entry. Failure would
  misattribute to the probe invariant.
- 🔵 minor (medium) — *The uncreatable-directory test's non-root precondition is
  mis-attributed* — Phase 1, Section 1. `candidate_performs_no_filesystem_write_or_process_spawn`
  carries no such assumption; the genuine carriers are the `0o555` tests.
- 🔵 suggestion (medium) — *The concurrent two-thread resolution path gets no
  probe-count assertion* — What We're NOT Doing; Phase 2. Assert a bounded delta
  since the count is legitimately 1 or 2.

### Test Coverage

**Summary**: Unusually thoughtful about test design for a guard-the-invariant task:
it correctly falsifies the work item's premise that `SEQUENCE` counts invocations,
drives that with a discriminating unit test, introduces a non-resolving cache seed so
the warm-hit delta of 0 is unambiguous, and commits to per-case mutation recording.
Three substantive gaps remain: the five integration tests are written after the seam
they exercise and therefore never go red, the single most valuable assertion — the
warm-hit delta of 0 — is deliberately expected to stay green under the only mutation
applied, and one behavioural assertion is lost when `a_writable_plugin_root_is_used`
is deleted, against an acceptance criterion requiring every deleted assertion to be
discharged.

**Strengths**:
- Reads the code rather than the work item and finds that `SEQUENCE.fetch_add` sits
  after the early return — then writes a specific test whose whole purpose is to
  discriminate the two. Genuine test-design reasoning, not ceremony.
- `seed_cache` built on `cache::store` is the right arrangement primitive: real
  bytes, real sha256, real trusted signature, zero prior probes.
- The old-test → discharging-test mapping is reported honestly as two re-homed and
  two already covered, explicitly refusing to pad it.
- The mutation exercise is specified per-case with an expected value for each test
  rather than a blanket "tests go red".
- Test-runner behaviour is verified rather than assumed — the absence of a
  `nextest.toml` is checked.
- Memoisation is ruled out and the one criterion that discriminates it is retained
  with an explicit note.
- Phase 5 decides the overrun policy in advance, explicitly to avoid choosing a
  threshold after seeing the number.

**Findings**:
- 🟡 major (high) — *The five integration tests have no red step; the mutation is
  used as post-hoc evidence* — Phase 2 / Phase 1 Success Criteria. Phase 1 merges
  independently, so the tests pass on first run; the criterion "fail before Phase 1's
  accessor exists" is unachievable. Phase 1's `SEQUENCE`-based comparison is
  described only in success criteria, not in Changes Required, with no Validation
  Results slot.
- 🟡 major (high) — *The warm-hit delta test is never mutation-demonstrated* — Phase
  2, Section 3. The delta of 0 protects what 0169 actually delivered and is the only
  one of five whose ability to fail is undemonstrated. Add a mutation to
  `ResolveBinary::resolve`.
- 🟡 major (high) — *Deleting `a_writable_plugin_root_is_used` loses the only
  assertion that `verify_writable` creates a missing directory* — Phase 3, Section 2.
  It passed a bare temp dir so the probed `temp/bin` did not exist; after Phase 3
  nothing exercises the documented create-if-needed behaviour.
- 🟡 major (high) — *The benign-cache-I/O refetch arm is a fourth path and gets no
  delta test* — Phase 2, Section 2. `each_of_two_cold_misses_…` is a repetition of
  the cold-miss path, so the "one per path" framing overstates by one.
- 🟡 major (high) — *`skip_if_no_minisign!` turns all five guard tests into silent
  passes* — Phase 2, Section 2. Bites hardest during the mutation exercise: all five
  would pass under the mutation and the recorded evidence would show the opposite of
  what is intended.
- 🟡 major (high) — *The stated justification for the delta form is wrong* —
  Implementation Approach; Testing Strategy. `tasks/test/cli.py` explicitly treats
  plain `cargo test` as supported, where these tests are non-deterministic.
- 🔵 minor (medium) — *`a_probe_against_an_uncreatable_directory_still_counts` is
  privilege-dependent and writes to `/` when run as root* — Phase 1, Section 1.
  Suggests targeting a path beneath a regular file so `create_dir_all` fails with
  `ENOTDIR` regardless of privilege.
- 🔵 minor (high) — *`a_successful_refetch_…` cannot distinguish a refetch from a
  cold miss* — Phase 2, Section 2. Both yield delta 1 and `Ok`, so the test could
  silently degrade into a duplicate and still pass.
- 🔵 suggestion (medium) — *The latency gate is a one-shot manual measurement whose
  baseline becomes unreproducible afterwards* — Phase 5. Nothing ratchets, and the
  comparison cannot be repeated once the recovered baseline is deleted — which is how
  0169's Phase 10 came to be orphaned.

### Code Quality

**Summary**: A carefully-reasoned plan that reads the code rather than trusting the
work item, and picks a defensible seam shape under genuinely tight constraints. The
design weaknesses are concentrated in naming and in the code-level documentation it
proposes: the accessor's name asserts the opposite of what its own discriminating
test pins, and Phase 3 corrects a stale statement in `internals.md` while leaving the
equally stale module doc at the top of the file it is editing. Secondary concerns are
test-harness hygiene — a non-hermetic discriminating test, inconsistent error types
across two sibling helpers, a fourth verbatim copy of the offline-resolver
construction — plus a Phase 5 harness described in prose but, contrary to the plan's
own claim, not provided.

**Strengths**:
- Falsifies the item's definitional claim and drives the correction with a
  discriminating test written first — exactly the red-green discipline the repo
  mandates.
- The test seam is chosen under stated constraints rather than by default, with a
  cited workspace precedent for refusing a cargo feature, and public-surface growth
  held to exactly one function.
- The delta form is justified permanently rather than as a nextest artefact.
- The mutation exercise predicts per-case outcomes in advance and names the
  warm-hit-stays-green case as a preservation guard.
- Phase 3 records an honest mapping rather than padding to four, and leaves the
  existing poisoning tests untouched.
- Phase 5 decides the overrun policy before the number is seen.

**Findings**:
- 🟡 major (high) — *`probes_performed` names the opposite of what its own
  discriminating test pins* — Phase 1, Section 2. The counter increments on a call
  where no probe is performed at all. Suggests `probe_attempts_made()` or
  `verify_writable_calls()`.
- 🟡 major (high) — *Deleting `cache_root::resolve` leaves the module doc describing
  it* — Phase 3, Section 1. `cache_root.rs:1-6` references it by behaviour rather
  than symbol, so no grep catches it; its stated precedence is already backwards.
- 🟡 major (medium) — *Phase 5 claims to embed a harness like 0186 but supplies only
  a prose description* — Phase 5, Section 3. 0189's requirements exceed 0186's
  heredoc, which passes no stdin and asserts no preconditions.
- 🔵 minor (high) — *The accessor's doc comment restates its signature instead of
  declaring test-seam intent* — Phase 1, Section 2. Cites
  `cli/vcs-adapters/src/subprocess.rs:53` and
  `cli/work-adapters/src/diff_shellout.rs:38` as the existing convention; suggests
  `#[doc(hidden)]`.
- 🔵 minor (high) — *The discriminating test is non-hermetic: a hard-coded path under
  `/` plus a non-root assumption* — Phase 1, Section 1. It is the least hermetic test
  in a module where every other test uses `tempfile`.
- 🔵 minor (high) — *Two sibling harness helpers with two different error types;
  `empty_cache`'s failure mode is opaque* — Phase 2, Section 1.
- 🔵 minor (high) — *`a_failed_refetch_…` adds a fourth verbatim copy of the
  offline-resolver construction* — Phase 2, Section 2. Already at `:465-473`,
  `:501-510`, `:531-539`.
- 🔵 suggestion (medium) — *`empty_cache` reads as a predicate, and the module import
  is left half-applied* — Phase 2, Section 1. Suggests `clear_cache` and collapsing
  `:521`.

### Architecture

**Summary**: Well-grounded — it re-verifies its premises against the tree, correctly
refuses a speculative verifier port, and keeps the new seam inside the outbound
adapter so neither `launch::core` nor its cargo-pup import rule is touched. Its
architectural weakness is the seam itself: a process-global `static PROBES` plus a
`pub` accessor makes the observation non-compositional, pushes a test-only concept
into an adapter's public contract, and forces extra test coupling to cache internals
— and the DI alternative is dismissed by citing scope rather than by argument, in a
module that already injects for testability. Separately, the claim that deleting
`cache_root::resolve` makes the property "structural" is overstated.

**Strengths**:
- The counting seam is confined to the outbound adapter; `launch::core` and the
  `launch_core_imports_only_permitted` cargo-pup rule (`cli/pup.ron:25-39`) are
  untouched, and the functional-core/imperative-shell split is preserved.
- Correctly refuses to reuse `SEQUENCE` and writes a discriminating test first.
- Correctly identifies that no verifier port is needed, argued from evidence.
- Refuses memoisation on behavioural grounds and names the single discriminating
  criterion.
- The Phase 5 overrun policy is decided before the number is seen, and the
  composition-budget discipline is inherited rather than reinvented.
- Honest accounting: the mapping is not padded, the false definitional sentence is
  corrected, and the stale release-blocked dependency is challenged with named
  evidence.

**Findings**:
- 🟡 major (high) — *Process-global counter puts a test-only concept in an adapter's
  public contract, and the DI alternative is dismissed by scope rather than argument*
  — Implementation Approach; Phase 1; What We're NOT Doing. Notes the concrete cost
  (retaining `asset_sig`, adding `seed_cache` coupled to `cache::store`) and offers a
  lighter middle path: count `tracing` events under a per-test subscriber, as
  `cli/visualiser/server/src/log.rs:233` does.
- 🟡 major (high) — *The delta convention does not make a global counter safe; the
  guard rests on nextest's process-per-test model, which nothing records or asserts*
  — Implementation Approach; Testing Strategy; Current State Analysis. Notes the plan
  drops the second half of the work item's own precondition ("**or is serialised**").
- 🟡 major (high) — *"Structural" is overstated — deleting a dead wrapper does not
  constrain future callers* — Overview; Desired End State; Phase 3. Offers the cheap
  genuinely structural lever: narrow `verify_writable` to
  `pub(in crate::launch::outbound::resolve)`.
- 🟡 major (medium) — *The five tests guard the resolver, but the likeliest
  regression site is the composition root* — Phase 2; Phase 3. `main.rs:65` already
  calls `cache_root::candidate` and needs one line to probe; invisible to all five
  tests.
- 🔵 minor (high) — *`PROBES` / `probes_performed` names something other than what it
  counts* — Phase 1, Section 2. Two atomics of identical type in one module with
  adjacent-sounding names and different meanings.
- 🔵 minor (high) — *Phase 4 is unrelated scope with no work-item mandate* — Phase 4.
  Phase 3's `internals.md` correction is different and defensible; Phase 4 is
  plan-invented scope in two different components.
- 🔵 minor (medium) — *Phase 5 binds a CI-verifiable refactor's closure to a one-shot
  single-host measurement with a non-binding gate and no durable guard* — Phase 5.
  Also notes `bin/.tmp-vcs-guard-baseline.sh` squats on the `store::TEMP_PREFIX`
  namespace inside the launcher's real cache root.
- 🔵 suggestion (high) — *Phases are a stack, not independent* — Implementation
  Approach. Phase 2 will not compile without Phase 1.
- 🔵 suggestion (medium) — *The memoisation ban rests on an assumption the plan does
  not carry forward* — What We're NOT Doing. Names the plausible future case (a
  prefetch/warm-cache subcommand resolving several sub-binaries in one process).

### Standards

**Summary**: Unusually disciplined about repo conventions: every mise task exists,
`docs:check` is correctly identified as outside the aggregate `check`, the
`ACCELERATOR_COVERAGE=off` escape matches `tasks/README.md:320`, test names follow
the file-local convention, the 80-column rewrite is correct, and the
gitignore/exec-bit/bashisms claims all check out. The material gaps are in the
documentation-pointer and cross-document-backfill work: Phase 4 introduces an
absolute production URL into a SKILL.md body rendered verbatim into the docs site,
and Phase 5 points its backfill at a file that has no Validation Results section.
Two verification criteria are also weaker than they read.

**Strengths**:
- Every mise task the plan invokes exists, and it correctly treats `docs:check` as
  manual and non-aggregate.
- `ACCELERATOR_COVERAGE=off mise run test:unit:cli` is the documented fast-loop form,
  and `test:unit:cli` genuinely does cover `resolution.rs` since `tasks/test/cli.py`
  runs `cargo nextest run --workspace --all-features` over every target.
- New test names follow the established full-sentence snake_case convention.
- The exec-bit-invariant claim is accurate and non-obvious: `.gitignore:56` really is
  `bin/.tmp-*`, and `walk_files()`/`shell_sources()` (`tasks/shared/sources.py:88-135`)
  filter on the root gitignore spec.
- The shell edit is a string-literal-only change to an existing `case` arm and trips
  nothing on the bash-4 denylist; even the awk trailing-comment strip does not fire.
- The hook-test claim is verified: `:328` and `:342` assert only on
  `"CLAUDE_PLUGIN_DATA unavailable"`.
- The `#[must_use]` / `must_use_candidate = "allow"` reasoning against
  `cli/Cargo.toml:136-147` is correct, and the cargo-feature refusal cites the real
  precedent.
- Phase 4's factual survey holds up, and the 80-column convention is honoured where
  it can be, with the over-80 SKILL.md link line justified against README precedent
  rather than silently broken.

**Findings**:
- 🟡 major (high) — *Backfill targets the wrong 0169 file — that plan has no
  Validation Results section* — Phase 5, Item 5; References. The five `_pending_`
  slots live in `meta/work/0169-…md:700-701` under that document's own `## Validation
  Results` at `:688`; the plan's Phase 10 checkbox is at `:1507`.
- 🟡 major (medium) — *Absolute production URL in SKILL.md deviates from the
  docs-site cross-link convention and duplicates the hosting decision* — Phase 4.
  Cites the five existing relative cross-links and
  `docs-site/astro.config.mjs:9-12`; notes no SKILL.md currently contains an
  `atomicinnovation.github.io` URL.
- 🔵 minor (high) — *The `cache_root::resolve` absence grep is vacuous* — Phase 3,
  Success Criteria.
- 🔵 minor (high) — *Module doc comment describes the deleted function and is left
  stale* — Phase 3, Section 1.
- 🔵 minor (high) — *No named command runs the hook tests, and `docs:check` does not
  validate the external anchor* — Phase 4, Success Criteria. `mise run check`
  contains no test tasks (`mise.toml:575-577`); the repo's task is
  `test:integration:hooks`. `starlightLinksValidator` is configured
  `errorOnRelativeLinks: false` (`astro.config.mjs:74`) and validates internal links
  only. Also notes the inconsistent `ACCELERATOR_COVERAGE` usage across Phases 1-3.
- 🔵 minor (medium) — *Importing `cache` while leaving the fully-qualified call site
  leaves one module addressed two ways* — Phase 2, Section 1.
- 🔵 minor (medium) — *The scratch script borrows a gitignore glob reserved for
  launcher runtime artefacts* — Phase 5, Section 1.
- 🔵 minor (high) — *`rtk proxy` — a personal CLI wrapper — leaks into a committed
  plan's verification criterion* — Phase 4, Success Criteria.
- 🔵 minor (medium) — *In-place amendment of a closed document departs from the
  recorded dated-note practice* — Phase 5, Items 2 and 5. Also notes no mention of
  refreshing `last_updated`/`last_updated_by`.
- 🔵 suggestion (medium) — *Two new harness helpers with mismatched error types, a
  noun-phrase name, and a duplicated magic path* — Phase 2, Section 1; Phase 1,
  Section 1. Suggests extracting the uncreatable path into a named const so the
  shared non-root assumption is expressed once, in code.

### Documentation

**Summary**: Unusually strong on documentation for a performance/guard task: it
correctly identifies a shipped falsehood in the published Internals page, verifies
the replacement text against the resolver's actual warm/cold behaviour, and argues
convincingly for published-URL pointers over source paths. Every factual claim was
verified and the load-bearing ones hold. The gaps are in coverage rather than
accuracy: the same falsified claim survives in `CHANGELOG.md` and in `cache_root.rs`'s
module doc, the work item is left defining "probe count" as a counter the plan
abandons, and the replacement paragraph drops the operational verdict its section
exists to give.

**Strengths**:
- Correctly identifies that `internals.md:277-280` is a shipped falsehood and fixes
  it in the same change as the structural work.
- The replacement paragraph is technically accurate against the resolver: `reverify`
  only reads, and both self-heal arms route to `fetch_verify_store`, so classifying
  failed re-verification as *cold* makes "without writing or probing" true.
- Phase 4's reasoning for the published URL is sound and matches README's convention
  (including `README.md:54` for this exact page), and the anchor is genuinely valid.
- The "exactly two stale references" claim is verifiably correct for tracked non-meta
  files, and the mirrored copy at
  `docs-site/…/reference/skills/visualisation/visualise.md:167` is gitignored and
  regenerated, so it self-heals.
- The proposed `PROBES` doc comments explain a "why" rather than restating the code.
- Records an honest old-test → discharging-test mapping instead of padding it.

**Findings**:
- 🟡 major (high) — *The same falsified claim survives in `CHANGELOG.md:23-29`* —
  Phase 3, Section 3. 0186 documented this behaviour in internals.md and CHANGELOG.md
  as a pair, so fixing one leaves them contradicting each other.
- 🟡 major (high) — *Deleting the wrapper leaves the module doc at
  `cache_root.rs:1-6` describing exactly the composition being deleted* — Phase 3,
  Section 1.
- 🟡 major (high) — *The work item is left defining "probe count" as the counter the
  plan abandons* — Phase 1; Phase 5, Section 2. The item's `:37-38`, `:178-182` and
  `:241-245` all name `SEQUENCE`; Phase 5 already opens the item to edit.
- 🟡 major (medium) — *The replacement paragraph drops the operational verdict its
  section exists to give* — Phase 3, Section 3. Never says a cold dispatch fails with
  `CacheRootUnavailable`, nor what the operator should do; defines "warm dispatch"
  only by exclusion.
- 🔵 minor (high) — *The replacement range starts mid-sentence* — Phase 3, Section 3.
  Line 277 begins `invocations. That exemption stops at the bootstrap: …`.
- 🔵 minor (high) — *The work item's own References still points at
  `docs/internals.md`* — Phase 4. `meta/work/0189-…md:296`. Also notes the item has
  no acceptance criterion for the internals.md correction and Validation Results has
  no documentation slot.
- 🔵 minor (high) — *`docs:check` cannot see the two repointed links* — Phase 4,
  Success Criteria. Suggests a cheap guard in `scripts/test-design.sh`, which already
  holds `internals.md` and uses `assert_contains`.
- 🔵 suggestion (medium) — *Have the `PROBES` doc comment name the `SEQUENCE`
  distinction* — Phase 1, Section 2. Two `AtomicU64`s with no stated distinction
  invites a tidy-up merge that silently breaks the create-failure path.

### Performance

**Summary**: Phases 1–4 are performance-neutral and the plan is right that the added
relaxed `fetch_add` is immeasurable and that nothing lands on the warm dispatch path
— verified against `mod.rs:191-199`. Phase 5, however, is where the performance
content lives, and its measurement design is not yet strong enough to support a 10%
ratio decision: the one unmeasured budget term is the term that scales with binary
size and plausibly exceeds all stated headroom; the borrowed 0186 figures are
double-counted in one row and host/backend-sensitive by ±17 ms in another; and the
harness is specified only by cross-reference, without a repo fixture, a per-sample
validity assertion, or any dispersion reporting. The pre-decided overrun policy is
honest as pre-registration but converts the gate into a report.

**Strengths**:
- The Performance Considerations claim is correct and verifiable: `verify_writable`
  is unreachable on a warm hit (`mod.rs:191-199` returns from the `reverify` Ok arm
  before `fetch_verify_store`).
- Refusing to memoise is right on performance grounds too — the path it would
  optimise is already zero-cost warm.
- Refusing to reuse `SEQUENCE` keeps two meanings separate at a cost below
  measurement.
- Phase 5 inherits the right mechanics from 0186 rather than 0169's bare "median of
  20". This is the one place in the repo that got benchmark methodology right, and
  reusing it is correct.
- Preconditions are asserted by the harness rather than assumed.
- Pre-registering the response to an overrun avoids choosing a threshold after the
  fact.
- Naming 0191 as a co-requisite is honest about where the remaining milliseconds are.

**Findings**:
- 🔴 critical (high) — *The only unmeasured budget term is the one that scales with
  binary size, and it likely exceeds all stated headroom* — Phase 5, Section 4.
  `reverify` does a full read, a full sha256 pass (`verifier.rs:36`) and a full
  minisign/blake2b pass (`keys.rs:68`); 0186 measured a comparable 7.6 MB verify at
  ~6.8 ms against ~2.6 ms of headroom. Suggests measuring it directly and recording
  ms-per-MB.
- 🟡 major (high) — *The budget double-counts the launcher exec already inside the
  borrowed 29.92 ms figure* — Phase 5, Section 4. 0186's own composition table
  decomposes that median into terms including "launcher exec ~2.4". The mixed units
  also bury the harness floor (1.60 ms) and bash startup (6.10 ms).
- 🟡 major (high) — *No repo fixture or cwd is specified, and `B` is dominated by
  fixture-dependent subprocess spawns* — Phase 5, Section 3. `classify_checkout`
  (`:177`) spawns `jj workspace root`, two `git rev-parse` forms, `realpath` and
  repeated `command -v`; 0188 measured `jj` at 7.05 ms and `git rev-parse` at 4.40 ms
  on this host.
- 🟡 major (high) — *No per-sample validity assertion, and `--fail-safe` makes the
  failure path the fast path* — Phase 5, Section 3. `swallow_under_fail_safe`
  (`main.rs:212-220`) exits 0 without exec'ing the sub-binary. The gate could record
  a PASS produced by 50 fast failures.
- 🟡 major (high) — *Borrowed 0186/0188 terms are not transferable without
  re-measurement* — Phase 5, Section 4. The 29.92 ms term is from `pre.21` (tree is
  `pre.36`); 0186 explicitly handed on "the range and the backend, not a point
  estimate"; the 3.6–4.7 ms term is from 0188's prototype fixture, which 0188 said
  must be re-measured.
- 🟡 major (high) — *Medians alone cannot support a 10% gate given the dispersion
  this harness has shown* — Phase 5, Success Criteria. 0186's `before` ran min 119.02
  / median 125.35 / **p90 234.15**; the quiet `after` spanned ±8%. Suggests n,
  min/median/p90, IQR, paired differences and a bootstrap 95% CI.
- 🟡 major (high) — *The pre-decided overrun policy leaves no outcome that triggers
  action* — Phase 5, Section 4. The only sub-check with teeth is a self-consistency
  check on the budget's own bookkeeping.
- 🔵 minor (medium) — *Machine quiescence, power and thermal state are unasserted* —
  Phase 5, Section 3; Manual Testing Steps. Suggests capturing AC/battery, Low Power
  Mode and load average, and running the harness twice.
- 🔵 minor (high) — *The harness is specified only by cross-reference to a script
  that cannot be reused as-is* — Phase 5, Section 3.
- 🔵 suggestion (medium) — *Name the per-dispatch full-file verification cost as the
  lever, not only 0191's ~2.5 ms* — Phase 5, Section 4. ~15 MB hashed per hook
  invocation, and the guard fires on every Bash tool call; the sha256 in
  `verify_binary` is subsumed by the minisign signature on the cache-hit path.

### Portability

**Summary**: The core mechanism — a process-local `AtomicU64` read as a delta — is
platform-neutral and adds no environment coupling, and the plan is unusually explicit
about which environment assumptions it is choosing to preserve. The risk is
concentrated in two places: the new unit test, which hardcodes an absolute path at
the real filesystem root and depends on the test user not being root — an assumption
that breaks (and leaves persistent residue that breaks a neighbouring test) on
Linux-as-root containers while silently holding on macOS; and Phase 5's darwin-only
gate, whose composition budget is built from demonstrably platform-specific terms
without saying so. CI runs the cli suite on both `ubuntu-latest` and `macos-latest`.

**Strengths**:
- The counting seam is fully portable: a file-level `AtomicU64` with
  `Ordering::Relaxed` and a `pub fn` reader introduces no OS-specific facility, path
  assumption or new system dependency.
- Phase 5's preconditions forbid the dev overrides so `G` measures the real path, and
  require host, OS version, chip and plugin version to be captured rather than
  assumed.
- Phase 4's choice of the published URL is reasoned from deployment reality.
- The gitignored parking spot checks out: `.gitignore:56` carries `bin/.tmp-*` and
  `tasks/shared/sources.py` honours the root `.gitignore`, so the exec-bit invariant
  genuinely cannot see the file.
- The plan enumerates the environment assumptions it inherits rather than leaving
  them implicit, making them reviewable instead of accidental.

**Findings**:
- 🟡 major (high) — *New test depends on not running as root and writes to the real
  filesystem root, breaking a neighbouring test when it succeeds* — Phase 1, Section
  1. Run as root, `create_dir_all` succeeds and the test leaves
  `/nonexistent-acc-parent-dir` behind, permanently breaking
  `candidate_performs_no_filesystem_write_or_process_spawn` (`:210-228`) which asserts
  `!unwritable_parent.exists()` on the same literal path. macOS denies the create even
  for root, so it stays green on `macos-latest` and reddens on `ubuntu-latest`.
- 🟡 major (medium) — *"The `cache_root` test module is already unix-only" is true
  only by accident of an unconditional import, and Phase 3 erodes the evidence* — What
  We're NOT Doing. No `#[cfg(unix)]` on the module or any test; the cited lines are
  unconditional `use std::os::unix::fs::PermissionsExt` statements. Phase 3 deletes one
  of the two. Meanwhile production keeps a `#[cfg(not(unix))]` arm no test can reach.
- 🟡 major (medium) — *Darwin-only measurement and a platform-specific composition
  budget presented without a platform caveat or a linux handoff* — Phase 5, Sections
  3-4. The bootstrap selects `sha256sum` or falls back to `shasum -a 256`
  (`bin/accelerator:272-276`), a difference measured as ~3.5 ms versus ~11.7 ms; the
  signed manifest publishes both linux platforms.
- 🔵 minor (high) — *Precondition set covers only the two `*_BIN` overrides* — Phase
  5, Section 3. Names `ACCELERATOR_CACHE_DIR`, `ACCELERATOR_PLUGIN_ROOT`,
  `ACCELERATOR_RELEASE_BASE_URL`, the `ACCELERATOR_UNAME_S`/`_M` seams and the
  dev-launcher marker; also notes `env | grep` returns exit 1 on no match.
- 🔵 minor (medium) — *Baseline recovery assumes a jj working copy and POSIX chmod
  with no stated alternative* — Phase 5, Section 1.
- 🔵 minor (medium) — *`/usr/bin/true` instrument floor is a hardcoded absolute path*
  — Phase 5, Section 3.
- 🔵 minor (medium) — *Counting tests inherit an undocumented exec-capable-`TMPDIR`
  requirement* — Phases 1 and 2.
- 🔵 suggestion (medium) — *Published docs URL hardcoded into two shipped runtime
  artifacts while the docs origin is otherwise externalised* — Phase 4.
  `docs-site/astro.config.mjs:11-12` reads them from `DOCS_SITE`/`DOCS_BASE`.
- 🔵 suggestion (low) — *Literal `/some-override-dir` is safe only while `candidate`
  stays free of filesystem access* — Phase 3, Section 2.
- 🔵 suggestion (low) — *The offline resolver simulation relies on `127.0.0.1:1` not
  being routed through a system proxy* — Phase 2, Section 2.
  `resolve/fetcher.rs:88-90` does not disable proxy discovery.

## Re-Review (Pass 2) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran against the revised plan. The launcher-side work is now
in good shape and was independently verified rather than merely re-asserted:
Correctness traced every proposed snippet against the real signatures and
confirmed they type-check and borrow-check, that the new sixth test genuinely
reaches the plain-cache-I/O arm at `mod.rs:222-228` and yields `Ok` with delta 1,
that narrowing `verify_writable` breaks no caller, that the ENOTDIR fixture is
privilege-independent on both platforms, and that the delta subtraction cannot
underflow. Test Coverage confirmed the mutation-first ordering produces a genuine
*behavioural* red for all six assertions rather than a compile error, and that
the old-test → discharging-test mapping is now complete.

The revision also introduced new defects, several of them in text written during
the edit pass. One is critical: the Phase 4 documentation link fix is broken, and
its stated CI verification is structurally incapable of detecting it. Phase 5's
arithmetic acquired three fresh errors while fixing its original one.

### Previously Identified Issues

**Resolved (17)**

- 🔴 **Performance**: Unmeasured `reverify` term — Resolved. Phase 5 item 1 now
  measures it first and refuses to fill the budget until it exists; Performance
  calls the sequencing "the right call".
- 🟡 **Correctness / Test Coverage / Architecture**: Delta form does not survive
  `cargo test` — Resolved. Rationale corrected, dependency documented.
- 🟡 **Portability +3**: Privilege-dependent test leaving residue — Resolved.
  Correctness independently verified `ENOTDIR` propagates for root and non-root
  alike on macOS and Linux; Portability calls it "a genuine portability win".
- 🟡 **Code Quality / Documentation / Standards**: Stale module doc — Resolved
  for `cache_root.rs`.
- 🟡 **Code Quality / Architecture**: `probes_performed` misnaming — Resolved.
- 🟡 **Standards**: 0169 backfill targeted the wrong file — Resolved and
  re-verified (`meta/work/0169…:700-701` under `## Validation Results` at `:688`;
  plan criterion at `:1507`).
- 🟡 **Documentation**: `CHANGELOG.md` carried the same falsehood — Resolved.
- 🟡 **Test Coverage**: Five tests had no red step — Resolved. Mutation-first
  ordering achieves the strong form of red.
- 🟡 **Test Coverage**: Warm-hit never mutation-demonstrated — Resolved by
  Mutation B.
- 🟡 **Test Coverage**: Deleting `a_writable_plugin_root_is_used` lost the
  create-if-missing assertion — Resolved by a replacement test.
- 🟡 **Architecture**: Composition root unguarded — Resolved. `main.rs` is a
  separate crate, so the narrowing structurally blocks it.
- 🟡 **Performance**: No fixture specified — Resolved (pure-jj scratch repo).
- 🟡 **Performance**: No per-sample validity assertion — Resolved. Performance
  calls it "the single most valuable guard in the harness".
- 🟡 **Performance**: Medians alone cannot carry a 10% gate — Resolved.
- 🟡 **Performance**: Overrun policy had no teeth — Resolved.
- 🔵 **Correctness / Standards**: Vacuous `cache_root::resolve` grep — Resolved.
- 🔵 Various minors — Resolved: Phase 1 discrimination contradiction (throwaway
  hoist), `internals.md` mid-sentence range, `empty_cache` live `read_dir`
  (now `remove_dir_all`), mismatched helper error types, `rtk proxy`,
  `test:integration:hooks`, `#[cfg(unix)]` claim, extended overrides.

**Partially resolved (8)**

- 🟡 **Correctness / Test Coverage**: Benign-cache-I/O arm — the test exists and
  reaches the arm, but nothing *pins* it there: a cold miss and the checksum arm
  both also produce delta 1 + `Ok`. The failing half of that arm is still
  uncovered.
- 🟡 **Architecture**: "Structural" overstated — **still present**. The narrowing
  blocks the composition root but does not constrain the invariant; both of the
  plan's own mutations compile unchanged under it.
- 🟡 **Code Quality / Performance**: Harness not embedded — the plan now promises
  it verbatim in Validation Results, but still does not contain it; the module
  doc text and the six assertion messages are likewise specified but unwritten.
- 🟡 **Correctness / Performance**: Budget double-count — the duplicate row is
  gone, but two *new* arithmetic errors replaced it (see below).
- 🟡 **Test Coverage**: `skip_if_no_minisign!` — the mutation recording must now
  prove minisign ran, but no permanent guard exists.
- 🟡 **Architecture**: Global counter / DI dismissed by scope — now weighed
  honestly, but a thread-local counter was never considered (see below).
- 🟡 **Documentation**: Work-item definitional sentence — amendments are now
  specified, but scheduled in Phase 5 (the most deferrable phase) rather than
  Phase 1, and criteria 3/4 plus the Requirements ordering rationale are omitted.
- 🟡 **Portability**: Darwin-only gate — now stated with a hand-off, but the
  hand-off has no work item and the new machine-state gating is darwin-only
  tooling, so the hand-off is a rewrite rather than a re-run.

**Regressed (1)**

- 🟡 **Standards**: The SKILL.md absolute-URL finding was addressed by switching
  to a site-relative link — which is broken. See the critical below.

### New Issues Introduced

#### Critical

- 🔴 **Documentation** (also found by Correctness and Standards): **The Phase 4
  SKILL.md link does not resolve.** `[Terminal Invocation](internals.md#…)` was
  justified by existing cross-links in `corpus.md`, `collaboration.md`,
  `releases-and-compatibility.md` and `visualiser.md` — but all of those sit at
  `docs-site/src/content/docs/`, siblings of `internals.md`. The generated skill
  page lands at `docs-site/src/content/docs/reference/skills/visualisation/visualise.md`
  (`tasks/shared/paths.py:13` + `skill_pages.py:125-128`), three directories
  deeper, so the link resolves to `reference/skills/visualisation/internals.md`,
  which does not exist. The phase's own automated criterion cannot catch it:
  `starlightLinksValidator({ errorOnRelativeLinks: false })`
  (`astro.config.mjs:74`) skips relative links — demonstrable today, since
  `skills/integrations/jira/init-jira/SKILL.md:26` already carries an
  unresolvable `](../../config/configure/SKILL.md#work)` and `docs:check` passes.
  Phase 4 exists to remove a pointer that never resolved and would replace it
  with a second one that also does not. **Verified directly during this review.**
  Fix: either the absolute published URL (works in the prompt *and* on the site)
  or `../../../internals.md`, matching the `../`-prefixed form
  `reference/agents.md:10` and `reference/meta-directory.mdx:13` already use.

#### Major

- 🟡 **Performance**: `bin/accelerator version` is not a valid proxy for the
  bootstrap term. `main.rs:50-53` states the laziness exists precisely so a
  `version` built-in never constructs the resolver, so the guard dispatch
  additionally pays `install_crypto_provider`, `TrustedKeys::embedded` and
  `Fetcher::new` (reqwest client + tokio runtime + rustls config) before the
  cache is consulted — none of which the residual row enumerates.
- 🟡 **Performance / Correctness**: The budget's reconciliation rule is wrong in
  two ways. It subtracts the harness floor from `G` only, while both new variants
  are whole-run subprocess medians carrying their own floors; and taken literally
  the sum omits the bootstrap row entirely, which would drive the residual to
  ~100% of `G` on a healthy run and misfire the 25% threshold.
- 🟡 **Performance / Correctness**: The follow-up rationale's volume arithmetic
  is wrong. The two warm-path `sha256_file` calls hash the ~475 KB verify shim
  (`bin/accelerator:291`, `:295`), not the 8 MB launcher, which is hashed once by
  BLAKE2b at `:353`. 0191's saving is process-spawn overhead, not hashing
  throughput. The stated "~15 MB per hook invocation" does not even sum to its
  own (incorrect) enumeration.
- 🟡 **Documentation**: The new closing sentence of the `internals.md` paragraph
  is inaccurate three separate ways — "the same named error" has no antecedent
  and is a *different* error (`no usable cache directory` from the launcher vs
  `no writable, exec-capable cache directory` from the bootstrap); under
  `--fail-safe` (which the git guard uses) a cold dispatch does not fail at all
  but exits 0 with a warning; and "warm it once" contradicts the version-bump
  clause two lines above, ignores that warming is per sub-binary, and cannot
  coexist with "permanently read-only".
- 🟡 **Code Quality**: A thread-local counter was never weighed. It would satisfy
  every assertion the plan writes (`verify_writable` is called synchronously on
  the calling thread; no planned assertion counts across threads) and would
  delete the nextest dependency, the `nextest.toml` non-goal, the third
  doc-comment paragraph and all six cross-test-interference messages.
- 🟡 **Architecture**: Phase 5's overrun branch makes 0189's closure contingent
  on 0191, a dependency edge absent from the work item's `blocked_by`.
- 🟡 **Architecture**: The work-item definitional correction is scheduled in
  Phase 5 though it is a consequence of Phase 1, so if Phase 5 stalls the item's
  normative definition contradicts the implementation indefinitely.
- 🟡 **Test Coverage**: The healed-bytes rationale is wrong. `cache::store`
  derives its path deterministically from `{name}-{version}-{sha256}`, so a
  degraded seed would make `resolve` a cold miss writing the *same* path with the
  *same* bytes. The real discriminator is `seed_cache`'s findability
  postcondition — which a maintainer could now trim as redundant.
- 🟡 **Portability**: Phase 5's machine-state gating (Low Power Mode, battery) is
  reachable only via `pmset`, so the harness the linux hand-off is meant to
  re-run cannot run on linux; the load threshold is also absolute rather than
  per-core.

#### Minor

- 🔵 **Architecture / Correctness / Documentation**: The revision introduced a
  *new* intra-doc link to a now-private item — "Counts calls to
  [`verify_writable`]" in the accessor doc — and the pre-existing hedge at
  `cache_root.rs:41` is conditioned on a rustdoc lint that can never fire, since
  nothing in the repo runs `cargo doc`.
- 🔵 **Code Quality**: `resolve_offline` is added citing three duplicate sites
  (`:465-473`, `:501-510`, `:531-539`) but the plan never instructs updating
  them, leaving four copies instead of one.
- 🔵 **Standards**: 179 lines exceed 80 columns in 1590, against 4/1547 in the
  0186 plan and 14/1598 in the 0169 plan. Four proposed `///` lines sit at 81
  columns and would land in `cache_root.rs` where rustfmt will not rewrap them
  (`wrap_comments` is off by default); the Mutation B snippet signature is 82
  where the real one at `mod.rs:181-184` is split across four lines.
  **Verified during this review.**
- 🔵 **Code Quality / Architecture / Standards**: `pub(in crate::launch::outbound::resolve)`
  is exactly equivalent to `pub(super)` here, and `pub(super)` is the workspace's
  only restricted-visibility idiom (`cli/vcs-adapters/src/library/dirty_paths.rs:30`).
- 🔵 **Code Quality / Test Coverage**: `an_override_is_used_verbatim_without_touching_the_filesystem`
  promises more than it asserts; the literal `/some-override-dir` reintroduces
  the same root-residue hazard the Phase 1 fixture was rearranged to avoid.
- 🔵 **Portability**: The `sudo -E cargo nextest` verification step compiles as
  root into the shared `cli/target/` and is likely to fail on Debian/Ubuntu
  `secure_path` regardless.
- 🔵 **Performance**: The 0188 fixture citation uses superseded prototype figures
  (7.05 ms / 4.40 ms); the delivered figures in 0188's work item are 23.84 ms and
  5.34 ms, understating the fixture argument roughly threefold.
- 🔵 **Standards**: "the five existing cross-links" — four are listed.
- 🔵 **Architecture / Standards**: Phase 4 breached its own stated tripwire ("if
  this phase grows beyond the two pointers it should be lifted") by acquiring a
  third change; and the `scripts/test-design.sh` anchor guard is design-domain,
  discoverable only by accident, with no criterion naming
  `mise run test:integration:config` as the task that runs it.
- 🔵 **Documentation**: The sibling module doc at `resolve/mod.rs:1-2` still says
  "resolved cache root", stale for exactly the reason `cache_root.rs:1-6` was.
- 🔵 **Correctness**: Placing `fetch_add` as the first statement weakens the
  memoisation canary — a memo placed just below it inside `verify_writable` would
  still record delta 2 and keep `each_of_two_cold_misses_…` green.
- 🔵 **Test Coverage**: No resolver-level delta on a *failing* probe, which is
  the shape of the "repair the cache root before retrying" regression the plan
  itself names; and Mutation B's expected collateral failure of
  `resolve_succeeds_from_a_read_only_cache_root_on_a_hit` is unrecorded.
- 🔵 **Portability**: The uncreatable-directory test asserts only `is_err()`, so
  on a `noexec` `TMPDIR` it would pass vacuously and stop discriminating.
- 🔵 **Standards**: `relates_to` is omitted from frontmatter though the plan
  mutates two 0169 documents and is gated by 0191.

### Assessment

The invariant work (Phases 1–3) is close to done and has now been independently
verified at the code level rather than argued. What remains there is small and
mostly mechanical: pin the sixth test to its arm, correct the healed-bytes
rationale, decide thread-local versus the documented nextest dependency, and
apply `pub(super)`.

Phase 4 must not ship as written — its one substantive change is broken, and it
would be caught only by the manual step, late. Phase 5 remains the weakest part:
each revision has fixed the previous arithmetic error and introduced another,
which is itself a signal that its budget should be derived from directly
instrumented in-process measurements rather than from proxy variants reconciled
by hand.

The plan is also now ~11% over the 80-column convention its siblings hold to, and
carries substantial defence of superseded drafts that belongs in this review
rather than in an implementer's instructions.

## Re-Review (Pass 3) — 2026-08-11

**Verdict:** REVISE

All eight lenses re-ran. Phases 1–3 have converged and are close to implementable:
the thread-local counter was independently verified sound by two lenses from
different angles, branch coverage is now genuinely complete, and the mutation-first
ordering is confirmed as a real red-green loop. Every file:line citation Standards
spot-checked — roughly forty of them — resolves and says what the plan claims.

Phase 5 has not converged. Its methodology has now been fixed and replaced with a
different flaw in three consecutive passes: pass 1 left the dominant term
unmeasured and double-counted the launcher exec; pass 2's proxy variants were
structurally invalid; pass 3's replacement instrument cannot be reached at all, its
clock domains do not reconcile, and its budget identity closes by construction so
the residual check can no longer fail. Two lenses independently recommend lifting
the phase out of this plan.

A recurring pattern in this pass: most new findings are in text written during the
pass-2 edit, not in the original plan.

### Previously Identified Issues

**Resolved (14)**

- 🔴 **Documentation**: SKILL.md site-relative link broken — Resolved. Reverted to
  the absolute URL with the depth arithmetic recorded.
- 🟡 **Performance**: `bin/accelerator version` invalid proxy — Resolved; withdrawn
  with the `main.rs:50-53` reasoning, and Performance confirms both withdrawals.
- 🟡 **Performance / Correctness**: budget double-count — Resolved (replaced by a
  different defect, below).
- 🟡 **Performance / Correctness**: shell-side volume arithmetic — Resolved.
  Performance confirms the shim ×2 / launcher ×1 accounting and the 0191
  spawn-overhead reading are now correct.
- 🟡 **Performance**: superseded 0188 figures — Resolved; delivered figures now
  cited.
- 🟡 **Code Quality**: thread-local never weighed — Resolved and adopted.
  Correctness verified no thread hop on any probe path and that `const { Cell::new(0) }`
  is valid at the pinned 1.90/edition-2021 floor; Portability verified `Cell<u64>`
  has no `Drop`, so no TLS destructor — which matters for the musl-static linux
  artefacts.
- 🟡 **Test Coverage / Correctness**: benign-cache-I/O arm uncovered — Resolved.
  Both lenses confirm branch coverage is now complete.
- 🟡 **Test Coverage**: healed-bytes rationale wrong — Resolved.
- 🟡 **Architecture / Code Quality**: "structural" overstated — Resolved in Phase 3
  item 1's body (though not in its Overview, below).
- 🟡 **Portability**: `sudo -E cargo` step — Resolved in form (replaced by a
  binary-under-sudo instruction), but the replacement has its own defect, below.
- 🔵 **Standards**: 80-column compliance — Resolved for the plan prose: 179 → 9
  over-length lines, all unwrappable. (The proposed *replacement blocks* still
  break it once spliced — new finding below.)
- 🔵 Various minors — Resolved: `rtk proxy`, `test:integration:hooks`, override
  test's non-existence assertion, `mod.rs:1-2` sibling doc, verbatim CHANGELOG
  wording, `relates_to` frontmatter (Standards confirms it validates against
  `frontmatter_validation/schema.rs:48-54`), the ENOTDIR fixture's vacuous-pass
  hazard.

**Still present (5)**

- 🟡 **Architecture / Code Quality**: Phase 3's *Overview* still says the narrowing
  makes the property "enforced by the compiler rather than only by test", which the
  phase body then contradicts in bold. The top-level Overview likewise still says
  "make it structural".
- 🟡 **Architecture / Code Quality / Documentation / Standards**: Migration Notes
  still names `pub(in crate::launch::outbound::resolve)` and claims
  `#[doc(hidden)]`, neither of which matches the phase snippets. Four lenses.
- 🟡 **Architecture**: the 0191 dependency remains prose-only — `blocked_by` still
  names only 0169 while Phase 5 pre-registers "re-measure after 0191 lands before
  this criterion closes".
- 🔵 **Correctness / Code Quality / Architecture**: Performance Considerations still
  describes "one relaxed `fetch_add`"; there is no atomic.
- 🔵 **Portability**: the POSIX-only scope is documented but not structural — the
  `#[cfg(not(unix))]` stub still returns `true`, so a non-unix build compiles and
  then fails every resolution rather than failing honestly.

### New Issues Introduced

#### Critical

- 🔴 **Performance** (Architecture and Portability concur, independently): **The
  throwaway instrumented launcher cannot be reached.** `bin/accelerator:378`
  minisign-verifies the cached launcher before `exec`, so a locally built binary is
  reachable only via the dev-launcher override at `:239-251` — which Phase 5's own
  preconditions forbid (`! env | grep -qE 'ACCELERATOR_'`, marker absent) and which
  `exec`s at `:250`, *above* the two `sha256_file` calls at `:291`/`:295` and the
  launcher verification at `:353`/`:378`. Those are exactly the bootstrap costs the
  process-entry timestamp is meant to bound. **Verified directly during this
  review.** Either the instrumented step is unrunnable as specified, or it is run
  via the override and silently records a bootstrap missing ~7 ms of shim hashing
  and ~6.8 ms of launcher verification.

#### Major

- 🟡 **Performance**: **Clock domains never reconcile.** The harness brackets with
  `time.perf_counter()`; the launcher emits "monotonic timestamps". Rust's `Instant`
  exposes no absolute value at all, and on darwin CPython's `perf_counter` is
  `CLOCK_UPTIME_RAW` (excludes sleep) where `CLOCK_MONOTONIC` does not. "`G` median
  minus the instrumented process-entry timestamp" is not a computable quantity as
  specified.
- 🟡 **Performance**: **The budget identity closes by construction.** Every term is
  now a contiguous interval of the same `[spawn, exit]` window, with both end terms
  anchored on `G` itself, so the residual is structurally zero (or exactly minus the
  floor) regardless of whether any timestamp is placed correctly. The ≤25%
  unexplained-residual check — 0186's actual evidence mechanism, and the thing that
  makes a ratio pass mean anything — can no longer fail.
- 🟡 **Performance**: **Medians are not additive.** The budget subtracts and sums
  medians of separate, strongly right-skewed distributions (0186: median 125.35 vs
  p90 234.15), so the terms need not reconcile with `median(G)` even when every
  timestamp is correct — making real mis-attribution indistinguishable from skew.
- 🟡 **Correctness / Code Quality / Test Coverage / Standards**: **`seed_cache` is
  specified two or three incompatible ways.** The code block returns
  `Result<PathBuf, …>` and `Ok(cached.path)`; the prose four paragraphs later says it
  returns `CachedBinary` so the sidecar test can use `seeded.signature_path`; and the
  sidecar test snippet still does the `cache::find(...).ok_or(...)` re-query the
  prose forbids. Four lenses. This is the shared fixture three tests depend on.
- 🟡 **Correctness**: **`probes_during` inverts a deliberate ordering.**
  `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` restores `0o755`
  *before* asserting so a failure cannot leave an unremovable temp dir. Wrapping the
  call moves the assertion inside the read-only window: a delta mismatch panics at
  `0o555`, the chmod-back never runs, and `TempDir::drop` cannot unlink children of a
  `r-x` directory — leaking a permanent fixture into `TMPDIR`.
- 🟡 **Test Coverage**: **The failing-probe delta is never observed red.** Under
  Mutation A the first probe returns `Err` and `?` propagates before the second runs;
  under Mutation B the injected probe fails against `0o555` and returns early. Delta
  stays 1 both times — so the assertion added specifically to guard the "repair the
  cache root before retrying" regression is demonstrated by nothing. Needs a third
  mutation (retry-on-`Err`).
- 🟡 **Test Coverage / Correctness**: **Mutation tables cover six of eight
  delta-bearing tests.** Mutation A *does* redden the sidecar-I/O test at delta 2,
  falsifying the plan's "Mutation A perturbs no existing test"; Mutation B reddens
  all five non-zero new deltas, not just the warm hit. The recorded evidence would
  contain unpredicted reds, which is the ambiguity the exercise exists to remove.
- 🟡 **Correctness / Architecture / Test Coverage**: **Two more work-item passages
  are falsified and unamended** — `0189:96-101` ("the counter is process-wide, so an
  absolute read is never the right observation") and `0189:125-129` (the isolation
  precondition requiring per-process or serialised execution). The plan explicitly
  refuses to provide that serialisation. Three lenses.
- 🟡 **Code Quality / Documentation**: **The verbatim module doc reintroduces the
  staleness class it replaces.** It restates the `pub(super)` modifier in prose —
  which the plan's own justification says not to do — and asserts "at most once per
  resolution", an invariant owned by `resolve/mod.rs` and unverifiable from that
  file. Documentation adds that "module-scoped so no caller outside this package can
  reach it" is wrong twice over: `pub(super)` covers the module *and its
  descendants*, and `main.rs` is in the same Cargo *package*.
- 🟡 **Code Quality**: **The instrumented launcher has no revert gate.** Phase 2
  gates its mutations with a `jj diff` residue check; Phase 5's instrumentation
  touches production code on the hottest dispatch path and nothing asserts it is
  gone. Left behind it writes timing noise to stderr on the `--format=hook` path.
- 🟡 **Documentation**: **The `--fail-safe` claim is false for one of the three
  cold-dispatch cases the same sentence enumerates.** A re-verification failure
  routes to `CorruptCacheAndRefetchFailed` → `kernel::Error::Refusal`
  (`core.rs:174-177`), and `swallow_under_fail_safe` (`core.rs:219-224`) swallows
  only `Failed` — so that path exits 2, which for a `PreToolUse` hook is a **block**,
  not a silent pass. **Verified directly.** The paragraph tells operators the guard
  degrades silently precisely where it does the opposite.
- 🟡 **Standards**: **The new shell suite would be vacuous.** Every
  `scripts/test-*.sh` ends in `test_summary`, which is what converts the `FAIL`
  counter into a non-zero exit (`test-helpers.sh:371-381`) — the `assert_*` helpers
  only increment a counter. The plan specifies the suite's purpose but not its
  content, so a suite written without `test_summary` exits 0 with failing
  assertions. **Verified directly.**
- 🟡 **Architecture**: **The thread-local observes a narrower scope than the
  invariant.** The work item defines dispatch as one launcher *process*; the counter
  is per-thread. The warm-hit delta of 0 means "no probe on this thread" — so a
  regression probing from a spawned thread passes green, while the five non-zero
  deltas would fail loudly. The asymmetry is inverted from what the guard needs.
- 🟡 **Portability**: **The root check runs the whole test binary**, and two existing
  tests cannot pass as root (`verify_writable_rejects_a_read_only_directory`,
  `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss`) because the
  superuser bypasses `0o555`. On darwin a "root container" is a linux VM and cannot
  run the darwin binary at all. Needs an exact-filter invocation.
- 🟡 **Portability**: **The linux hand-off's build target is wrong.** Shipped linux
  is `*-unknown-linux-musl` via `cargo zigbuild`; a native build is `gnu`, whose
  allocator differs markedly for the 8 MB `std::fs::read` dominating `reverify`. The
  profile/target discipline was applied to Phase 5 item 1 but not item 4.

#### Minor

- 🔵 **Code Quality**: "six delta assertions" is stated in Desired End State, Testing
  Strategy and Phase 2's Manual Verification, but eight are mandated — and the two
  retrofits are, by the plan's own argument, the most load-bearing.
- 🔵 **Standards**: Phase 1's snippet already shows `pub(super)`, which is Phase 3's
  change — breaking the phase independence the plan claims and making Phase 3's
  criterion certify work already done.
- 🔵 **Standards**: the `pub(super)` convention claim is factually wrong.
  `pub(crate)` — also restricted visibility — appears **56 times across 26 files**.
  **Verified directly.** The choice is still right; the justification is not.
- 🔵 **Standards**: both verbatim replacement blocks break 80 columns once spliced
  onto their retained leading fragments (`invocations. ` prepended), and the
  CHANGELOG parenthetical drops its closing full stop.
- 🔵 **Standards**: the precondition writes `/.accelerator-dev-launcher`
  root-absolute; the bootstrap consults `${plugin_root}/.accelerator-dev-launcher`
  (`bin/accelerator:225`). **Verified directly.** As written the check tests a
  filesystem-root path that is trivially absent, so it always passes.
- 🔵 **Test Coverage / Standards**: `_EXPECTED_CONFIG_SUITES` is already behind —
  16 discoverable suites against a floor of 15 (**verified**), so bumping to 16
  leaves the new guard able to vanish if its exec bit drops. `_REQUIRED_CONFIG_SUITES`
  guards by name and is the right mechanism. The bump also misses the file's
  annotate-every-change comment convention.
- 🔵 **Documentation / Standards**: `swallow_under_fail_safe` is cited as
  `main.rs:212-220` in Phase 5 and correctly as `core.rs:219-224` in Phase 3.
- 🔵 **Test Coverage**: `each_of_two_cold_misses_probes_the_cache_root_once` asserts
  the *sum*, so 0-then-2 passes; its name claims a per-resolution property it does
  not pin. Two `probes_during(1, …)` calls would make the name true.
- 🔵 **Test Coverage**: `probes_during` needs `#[track_caller]`, or all eight
  failures report the same helper line.
- 🔵 **Test Coverage**: Phase 1's item order inverts its own dependency — item 1
  instructs running a test defined in item 2.
- 🔵 **Test Coverage**: `verify_writable_creates_a_missing_directory` is the one new
  assertion never observed failing, though it is the sole guard on the
  `create_dir_all` the plan says would otherwise break first-run resolution silently.
- 🔵 **Documentation**: the CHANGELOG replacement omits the third cold trigger
  (re-verification failure), so the two files still will not agree — and the manual
  criterion "now agree" would be ticked anyway.
- 🔵 **Performance**: five instrumentation points but only four boundaries in the
  table — the pre-`exec` timestamp appears in no row; and the construction interval
  also contains `Cli::try_parse`, `logging::init`, `override_path` and
  `cache_root::candidate`, overstating the recoverable saving.
- 🔵 **Performance**: two floors are measured but the identity says "one harness
  floor" without naming which. 0186's bash floor (6.10 ms) contains ~4.5 ms of real
  bash startup — a genuine bootstrap cost that subtracting it would delete.
- 🔵 **Performance**: item 4 has no build-profile requirement, though item 1 warns a
  debug build would be several-fold wrong.
- 🔵 **Performance**: Performance Considerations omits the ~10 added real probes
  (write+chmod+fork+exec+unlink) at ~108 ms each on darwin ≈ 1 s of added suite time.
- 🔵 **Portability**: load-per-core is not cgroup-aware — `getloadavg` is host-wide
  and `os.cpu_count()` ignores quota, so the threshold means different things on a
  container runner.
- 🔵 **Architecture / Documentation / Portability / Standards**: the anchor guard
  pins the hardcoded published origin, converting an acknowledged soft duplication
  into a load-bearing constant — a hosting move becomes a five-file change in which
  one file is the test meant to catch a missed site.

### Assessment

Phases 1–3 are converging well and are close to implementable. What remains there is
mechanical: settle `seed_cache`'s return type, fix the `probes_during` ordering
inversion for the two chmod-bracketed tests, extend the mutation tables to all eight
assertions and add the retry mutation, sweep the pre-thread-local residue, and
correct the module doc and the `pub(super)` justification.

Phase 5 should be lifted into its own work item. Two lenses recommend it
independently, and the evidence is now three passes deep: each revision has fixed
the previous methodology error and introduced another, because the phase is trying
to reconstruct an in-process decomposition from outside a trust boundary that
deliberately prevents it. It shares no code, no file and no test with Phases 1–3,
discharges 0169's deferred gate rather than 0189's probe invariant, and gates this
plan's closure on 0191 through prose alone — reproducing the orphaned-obligation
shape it exists to repair.

Phase 4 should also not ship as specified: its guard would exit 0 with failing
assertions, which is worse than no guard.

### Action taken after Pass 3

The plan was split, as this pass recommended. Phase 5 now lives in
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` — a second plan
against the same work item (`work_item_id`/`parent`: `work-item:0189`), following
the precedent 0167 set with two plans. The measurement plan opens with three
Open Decisions (OD-1 decomposition route, OD-2 clock domain, OD-3 residual
definition) recording the findings above as unresolved rather than re-answering
them a fourth time; it is not implementable until they are closed.

The probe-guarantee plan is now four phases and closes on CI evidence alone. The
majors in this pass that belong to it — the `seed_cache` contradiction, the
`probes_during` ordering inversion, the incomplete mutation tables and third
mutation, the unamended work-item passages, the module-doc restatements, the
Migration Notes drift and the vacuous shell suite — remain outstanding against
that plan.

## Re-Review (Pass 4) — 2026-08-11

**Verdict:** REVISE

First pass in which the plan's core is verified rather than corrected.
Correctness traced the mutation table cell by cell and found all twenty-four
cells correct against the real control flow, confirmed branch coverage of the
probe is genuinely complete, and confirmed the thread-local seam is monotonic,
underflow-free and read on the driving thread. Test Coverage independently
reached the same conclusion on the table and called Mutation C "a genuinely
surgical mutant". Standards verified the `pub(super)` precedent, the suite-count
arithmetic and that the `internals.md`/`CHANGELOG.md` blocks are 80-column
compliant once spliced. Portability verified the shell suite is bash-3.2 clean.

One critical defect was found — and it was in the block written during the pass-3
edit, not in the original plan. Three lenses caught it independently.

### Previously Identified Issues

**Resolved (14 of the 15 carried forward)**

- 🟡 `seed_cache` specified inconsistently — Resolved (returns `CachedBinary`;
  though the import instruction was wrong, below).
- 🟡 `probes_during` ordering inversion — Resolved; Test Coverage calls the
  carve-out "correct test hygiene".
- 🟡 Mutation tables incomplete — Resolved and extended to a full A/B/C table
  with both collateral reds pre-declared.
- 🟡 Failing-probe delta never reddened — Resolved by Mutation C.
- 🟡 Work-item passages unamended — Resolved for `:96-101` and `:125-129`.
- 🟡 Module doc restatements — Resolved (cut from ten lines to four).
- 🟡 Migration Notes drift — Resolved (`pub(super)`, `#[doc(hidden)]` claim
  dropped, arithmetic corrected to −1).
- 🟡 Shell suite vacuous — Resolved; `test_summary` present and the suite
  registered by name in `_REQUIRED_CONFIG_SUITES`, which Test Coverage and
  Standards both endorse over the count floor.
- 🟡 Phase 3 Overview claimed compiler enforcement — Resolved.
- 🟡 Six-vs-eight counting — Resolved throughout.
- 🟡 Root check ran the whole binary — Resolved (scoped by name).
- 🔵 `pub(super)` convention claim — Resolved; Standards verified `pub(super)`
  appears exactly twice and `pub(in crate::…)` zero times.
- 🔵 Splice widths, CHANGELOG full stop, dev-launcher marker path — Resolved.
- 🔵 `_EXPECTED_CONFIG_SUITES` floor — Resolved; three lenses confirm 16
  discoverable against a floor of 15, so 17 is right.

**Still present (1)**

- 🔵 The POSIX-only scope remains documented rather than structural. Portability
  adds that the sentence understates it: `HOST_PLATFORM`
  (`resolve/mod.rs:20-28`) has no non-unix arm, so the crate does not build off
  unix at all and the `#[cfg(not(unix))]` stub is unreachable dead code.

### New Issues Introduced

#### Critical

- 🔴 **Standards** (Test Coverage and Portability concur, independently): **both
  `assert_contains` calls in the verbatim shell suite used the wrong argument
  order.** The helper is `assert_contains <test_name> <haystack> <needle>`
  (`scripts/test-helpers.sh:33-44`); the block passed haystack, needle, label.
  As written it grepped the prose message out of a 12-character haystack, so the
  suite failed unconditionally — and because it is simultaneously registered in
  `_REQUIRED_CONFIG_SUITES`, it would have hard-failed the CI lane on landing,
  with the fail-closed criterion passing vacuously. **Verified and fixed during
  this pass**, before the lenses that reported it had returned.

#### Major

- 🟡 **Documentation / Correctness**: the rewritten `--fail-safe` claim
  over-corrected. Pass 3 found "the guard degrades silently" false for the
  integrity case; the fix asserted a failed re-verification is *never* swallowed,
  which is false for the plain cache-I/O arm at `mod.rs:222-228` — that arm
  returns the refetch error unwrapped, so a read-only root yields
  `CacheRootUnavailable` → `Failed` → swallowed → exit 0. Both over-broad
  readings are wrong; the shipped sentence now names the integrity check
  specifically. **Verified against the two arms.**
- 🟡 **Documentation**: the shipped paragraph omitted the exit-2/PreToolUse-block
  consequence its own rationale rested on.
- 🟡 **Documentation / Standards**: adding `ACCELERATOR_<TOKEN>_BIN` as a remedy
  contradicted the section's "Two environment variables" lead and two-row table,
  bent the placement rule at `tasks/README.md:429-435`, cited two
  `## Local development` tables as evidence it is an offline remedy, and omitted
  that the override bypasses signature verification entirely — in a section
  framed around trust-root inputs. The remedy has been withdrawn.
- 🟡 **Test Coverage**: `each_of_two_cold_misses_…` was never reddened by a
  *memoisation* mutation — under A and B it reddens for the same reason as six
  other rows, so nothing demonstrated it catches anything the plain cold-miss
  test does not. Mutation D (a `OnceLock` guard) has been added; it reddens that
  row and nothing else.
- 🟡 **Standards**: `mise run cli:check` was described as running cargo-pup. It
  does not — `pup:check` is a sibling under `check`, and the plan contradicted
  itself on this in Phase 3.
- 🟡 **Standards**: the nextest filter `test(probe)` selected only one of the two
  new unit tests; the other is the one the increment-deleted criterion exercises.
- 🟡 **Standards**: the `rg 'docs/internals\.md'` sweep would return three hits
  after Phase 4, not the asserted two — the new suite's own `DOCS_INTERNALS`
  assignment contains the substring.

#### Minor

- 🔵 **Code Quality / Correctness**: the `CachedBinary` import instruction would
  not compile — `resolve/mod.rs:15` imports it privately, so it is reachable only
  as `resolve::cache::CachedBinary`. **Verified and fixed.**
- 🔵 **Performance**: two probe costs quoted for one operation (~130 ms vs
  ~108 ms); ~130 is the *addressable total* including shim hashing.
- 🔵 **Performance**: "roughly ten new real probes" overstated — the count is
  derivable exactly as net +7, and `a_probe_against_an_uncreatable_directory_still_counts`
  costs microseconds, not 108 ms, because it fails before the write.
- 🔵 **Performance**: `seed_cache` recomputes a digest `happy_harness` already
  computes at `resolution.rs:165` and drops — the same defect the plan fixes for
  `asset_sig`.
- 🔵 **Correctness**: the `TempDir`-leak justification was wrong for the test it
  was attached to (that cache is empty at the point of failure); the hazard is
  real for `resolve_succeeds_from_a_read_only_cache_root_on_a_hit`.
- 🔵 **Correctness**: `Fetch` is not uniquely reachable through `:222-228` — a
  cold miss with a failing manifest fetch also yields it
  (`resolution.rs:413-424`), so the findability assertion is load-bearing rather
  than belt-and-braces.
- 🔵 **Correctness**: the thread-local bound is asymmetric — a probe *moved*
  off-thread reddens the cold-miss deltas, but a probe *added* off-thread is
  invisible to all eight assertions.
- 🔵 **Architecture / Code Quality / Performance / Standards**: residue from the
  split — "three items" over four, two references to a Phase 5 this plan no
  longer has, and a Validation Results slot naming `scripts/test-design.sh`,
  which Phase 4 explicitly rejects.
- 🔵 **Test Coverage**: `verify_writable_creates_a_missing_directory` and the new
  assertion in the re-homed override test are written against passing code with
  no red step.
- 🔵 **Standards**: `**File**`/`**Changes**` collapsed onto one line against
  `templates/plan.md`; `### Key Discoveries:` under Current State Analysis rather
  than Desired End State; Phase 4 missing its `---` separator.

### Assessment

The invariant work is converging. Every structural claim the plan makes was
verified this pass rather than merely re-argued, and the majors are now
concentrated in text written during the previous edit rather than in the design.

The pattern worth naming: for the third consecutive pass, the `internals.md`
paragraph was corrected and the correction was wrong in the opposite direction.
Pass 2 said it claimed silence too broadly; pass 3's fix claimed refusal too
broadly. The underlying cause is that `reverify` has two failure arms with
opposite `--fail-safe` behaviour, and any sentence that does not name which arm
it means will be wrong for the other. The current wording names the arm.

Remaining work is small and mechanical. No finding in this pass challenges the
design, the phase structure or the split.

### Action taken after Pass 4

All critical, major and minor findings above have been applied, except the
POSIX-only structural change and the `**File**`/`**Changes**` reformatting, which
remain open judgement calls. Mutation D was added with its column in the table
and an explicit statement of which mutation each assertion is authored under.

## Re-Review (Pass 5) — 2026-08-12

**Verdict:** REVISE

Seven lenses were run (Architecture skipped: pass 4 found the structure, scope and
split sound and nothing since touched them). Six returned; **Standards did not
run** — it terminated on an org spend limit — so its coverage of this pass's
changes is missing, and the shell suite it would have re-verified has now been
wrong in two prior passes.

**No critical findings.** This is the first pass without one. The majors are
concentrated in a single defect found three ways, and in the shipped
documentation text.

### Previously Identified Issues

**Resolved (all 8 majors and the critical from pass 4)**

- 🔴 `assert_contains` argument order — Resolved and independently confirmed by
  Test Coverage, Portability and Correctness.
- 🟡 `--fail-safe` over-correction — Resolved. Correctness and Documentation both
  verified the current three-way split clause-by-clause against `mod.rs:203-228`,
  `core.rs:167-193/219-224` and `main.rs:206/215-226`. Right on the third attempt.
- 🟡 exit-2/block consequence omitted — Resolved.
- 🟡 `ACCELERATOR_<TOKEN>_BIN` remedy — Resolved by withdrawal; Documentation
  verified all four supports for the withdrawal.
- 🟡 memoisation canary never mutation-tested — Resolved by Mutation D, which
  Correctness verified compiles and borrows correctly.
- 🟡 `cli:check`/cargo-pup, `test(probe)` filter, `rg` sweep count — Resolved.
- 🔵 `CachedBinary` import — Resolved; Correctness verified `cache::CachedBinary`
  is the only path that resolves.

### New Issues Introduced

#### Major

- 🟡 **Correctness / Test Coverage / Documentation** (three lenses, independently):
  **Mutation D's column had a wrong cell.**
  `a_signature_read_io_error_propagates_the_refetch_error_verbatim` performs a
  prior in-process `harness.resolve()` at `resolution.rs:520`, outside the
  retrofitted bracket, so a process-wide `OnceLock` leaves the measured bracket at
  0 — the cell said 1, and the claim "D reddens it and nothing else" was false.
  **Found by pre-checking before the lenses returned, and fixed**; all three then
  confirmed it.
- 🟡 **Performance**: **the 0186 probe-cost citation was wrong.** The plan claimed
  the ~131 ms figure was "the addressable total including verify-shim hashing".
  0186's re-derivation (`meta/work/0186-…:569-589`) shows it is the *identical
  probe run in the repo's `bin/`* — a 23% location swing — while 107.15 ms is the
  `/tmp` figure the tests actually pay, and the 10.6 ms re-exec figure came from a
  Context table 0186's own plan distrusted and re-derived to 3.72 ms. **Verified
  and corrected.**
- 🟡 **Documentation**: the plan instructed saying "writable and exec-capable"
  where the remedy is given, but its own verbatim block said only "writable" — the
  block is what ships.
- 🟡 **Documentation**: the warming procedure said "run each subcommand you intend
  to use", omitting the ones the plugin dispatches for the operator — `vcs` on
  every Bash tool call via the `PreToolUse` guard, which is precisely the
  subcommand whose silent non-execution matters.
- 🟡 **Documentation**: the CHANGELOG splice boundary named line 28 as the entry's
  final line; the final line is 29 (`  writable).`), so a literal splice would
  orphan it.

#### Minor

- 🔵 **Code Quality / Test Coverage / Correctness / Performance**: the A/B/C
  criteria and the "three mutation passes" cost estimate were not swept when D was
  added; the authoring-pairing sentence mis-counted (ten assignments over nine
  brackets).
- 🔵 **Correctness**: Mutation D's column is only well-defined under
  `cargo nextest`; under a bare `cargo test` the memo is shared binary-wide and the
  column is non-deterministic.
- 🔵 **Test Coverage**: Phase 3's two new assertions had no red step, and the three
  fixture postconditions the plan calls load-bearing were never demonstrated
  capable of failing.
- 🔵 **Portability**: the guard's `${DOCS_SITE:-…}` defaulting was illusory — those
  variables are consumed by `npm run build` inside `docs-site/` and are not
  exported into the `test:integration:config` lane, and `DOCS_SITE` already names
  the `docs-site/` *directory* in `tasks/shared/paths.py:12`. `:-` also diverges
  from astro's `??` on an empty value, breaking the one fork case it was for.
- 🔵 **Portability**: "POSIX only" overstated the support matrix — `HOST_PLATFORM`
  has four arms, so the crate does not build on FreeBSD or 32-bit linux either, and
  the `#[cfg(not(unix))]` stub is unreachable dead code.
- 🔵 **Portability**: the root-privilege step did not name linux, where the hazard
  actually occurs.
- 🔵 **Code Quality**: the `probes_during` carve-out is a plan-only rule that the
  delivered code will not explain; `seed_cache`/`clear_cache` lacked
  `#[track_caller]`.
- 🔵 **Code Quality**: Phase 1 item 2 still contained an instruction executable only
  after item 3 — the same inversion the phase claims to have fixed.

### Assessment

The trend is clear. Pass 4 was the first without a design-level finding; pass 5 is
the first without a critical. The one major that recurred across three lenses was
caught by pre-checking rather than by the lenses, and the rest are documentation
wording and count-sweeping.

Two things temper that. **Standards did not run**, and it is the lens that has
caught the shell suite's defects twice. And the pattern of a fix batch introducing
new defects held again — the Mutation D column and the 0186 citation were both
introduced in the pass-4 edit.

Not yet approvable, for one reason: this pass's fixes are themselves unreviewed,
and Standards has no coverage of them at all.

### Action taken after Pass 5

All findings above applied. Additions: a fixture mutation (degraded seed) to
discharge the three postconditions, forced-red steps for Phase 3's two new
assertions, `#[track_caller]` on both fixture helpers, the anchor guard
simplified to a literal plus two assertions tying it back to
`docs-site/astro.config.mjs`, and the one-off cost re-accounted with `mise run
check` named as the dominant term.

## Re-Review (Pass 6, Standards only) — 2026-08-12

**Verdict:** APPROVE

A narrow pass, run because Standards had terminated early in pass 5 and was the
only lens with no coverage of two rounds of edits — and because it is the lens
that caught the shell suite's defects in passes 4 and 5.

**No criticals. No majors.** This is the first pass to clear both, and it clears
them on the lens with the strongest track record for finding defects in this
plan's newly-written text.

### What Standards verified

- **The verbatim `scripts/test-docs-anchors.sh` is correct on every axis**,
  re-derived from scratch: `assert_contains <test name> <haystack> <needle>`
  arity against `test-helpers.sh:33-34`, the closing `test_summary` (`:371-381`)
  present and load-bearing, the preamble matching `test-design.sh:1-5`,
  ShellCheck-clean under `.shellcheckrc`'s `enable=all` plus its documented
  disables, bash-3.2 clean, ≤80 columns, and shfmt-shaped like the existing
  continuation at `test-design.sh:112-114`.
- **Registration mechanics exact**: 17 `test-*.sh` under `scripts/`, minus
  `test-helpers.sh` (on `SHELL_LIBRARIES`, `tasks/lint/scripts.py:32`), gives 16
  discoverable against the floor of 15, so 17 is right. Independently confirmed
  by direct measurement during this pass. `tests/unit/tasks/test_integration.py`
  reads both constants via `getattr`, so no test edit is owed.
- **All four 0186 probe figures and the `:569-589` range** — 1.41 ms floor,
  3.72 ms re-exec, 107.15 ms `/tmp`, 131.97 ms `bin/` — and the ~23% swing.
- **Both splice boundaries**, including that `internals.md:277` genuinely begins
  `invocations. That exemption stops at the bootstrap:`.
- **The visibility precedent counts**: `pub(super)` exactly twice, `pub(crate)`
  56 times across 26 files, `#[doc(hidden)]` zero occurrences, and no
  `cargo doc`/rustdoc invocation anywhere in the repo.

### Findings (all applied)

- 🔵 The hook's `sh` block was shown flush-left; the real line is a `case` arm
  with two leading spaces, and `.editorconfig` sets `switch_case_indent`. As
  written it would have failed `format:scripts:check` — a criterion the same
  phase lists.
- 🔵 The `#[track_caller]` rationale had landed in Phase 3 item 3, which is about
  `cache_root.rs` re-homing; the helpers it describes are declared in Phase 2
  item 1. Moved.
- 🔵 `**File**`/`**Changes**` were run together in 14 blocks against
  `templates/plan.md:64-65`, and Phase 2 item 2 had no `**Changes**` label at
  all. Split and labelled.
- 🔵 The two shipped blocks were within 80 columns but not greedily wrapped, and
  the CHANGELOG entry was cited `:22-29` when the bullet begins at `:23`.
  Re-wrapped and corrected.
- 🔵 The exec-bit criterion said `chmod +x`; the invariant is two-part — the mode
  must be **committed**, or it passes locally and fails CI on a fresh checkout.
  Extended.

### Assessment

The plan is approved for implementation.

Six passes, and the shape of the findings tells the story: design findings
stopped at pass 4, criticals at pass 5, majors at pass 6. The residue this pass
was template formatting and one shell-indentation detail — the class of thing a
formatter would catch, not a reviewer.

Two items remain open by choice, not oversight: making the POSIX-only scope
structural (Portability notes `HOST_PLATFORM` has no non-unix arm, so the crate
does not build off unix anyway and the `#[cfg(not(unix))]` stub is unreachable
dead code), and whether the `[Unreleased]` changelog should gain a `### Fixed`
entry for the two pointer repoints. Neither blocks implementation.

The work item's own amendments are scheduled inside the plan as implementation
steps, so they land when it is executed.

**The sibling measurement plan is not covered by this review.**
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` has never been
reviewed and states on its face that it is not implementable until OD-1, OD-2 and
OD-3 are closed. It needs its own review-1.

---
*Review generated by /accelerator:review-plan*
