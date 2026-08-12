---
type: plan
id: "2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"
title: "At-Most-Once Cache-Root Probe Guarantee Implementation Plan"
date: "2026-08-11T15:57:41+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0189"
parent: "work-item:0189"
derived_from:
  ["codebase-research:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"]
relates_to: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement",
  "work-item:0169", "work-item:0186"]
tags: [cli, launcher, bootstrap]
revision: "9fb90f8a26d91d640cf0f6ab8b272b6039d7bdbd"
repository: "accelerator"
last_updated: "2026-08-12T00:38:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# At-Most-Once Cache-Root Probe Guarantee Implementation Plan

## Overview

Pin the launcher's cache-root probe to at most one invocation per resolution
with tests, delete the dead second composition and narrow the probe's visibility
to the resolver module so reaching it from outside is a compile error, and
correct the documentation 0169's split falsified.

This is one of two plans against work item 0189. The warm-dispatch latency
measurement 0169 deferred lives in
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`: it shares no
code, no file and no test with this plan, and bundling the two would have made a
CI-verifiable refactor's closure depend on a one-shot single-host measurement.
This plan closes on CI evidence alone.

The property this plan protects already holds. 0169's Phase 5 moved the probe
off the warm path; nothing today calls it twice. The work is to make that
observable, make it structural, and prove the observation would notice a
regression.

## Current State Analysis

`cache_root::verify_writable`
(`cli/launcher/src/launch/outbound/resolve/cache_root.rs:88-99`) writes a script
into the cache directory, chmods it, executes it and removes it. It is the
launcher's only filesystem side effect before a fetch.

**One production call site**, the unconditional first statement of
`FetchVerifyCacheResolver::fetch_verify_store`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:141`). `main.rs:65` calls
`cache_root::candidate`, which is selection only — no filesystem access.

**`FetchVerifyCacheResolver::resolve` cannot reach `fetch_verify_store` twice.**
Three syntactic call sites (`mod.rs:211`, `:227`, `:231`). The first two are
arms of one `match self.reverify(&cached)`, so at most one runs. All three arms
of that match are `return` expressions, so control never falls through the `if
let Some(cached)` block to the tail call at `:231`. No loop, no recursion, no
retry. The only retry loop on this path lives inside `Fetcher::get`
(`resolve/fetcher.rs:116-125`), which re-issues HTTP requests and never
re-enters `fetch_verify_store`.

**`cache_root::resolve` (`cache_root.rs:73-77`) has no production caller.** It
is `pub`, so `dead_code` never flagged it. Its only four call sites are its own
unit tests, each of which performs a real probe.

**Nothing counts probes.** The `SEQUENCE` atomic inside
`probe_writable_and_executable` (`cache_root.rs:109`) exists for filename
uniqueness — two threads in one process would otherwise collide on a PID-only
probe path — and its `fetch_add` sits at `:114`, *after* the `create_dir_all`
early return at `:111-113`. It therefore counts probes that got as far as
writing, not `verify_writable` invocations. The work item's definitional
sentence ("one `verify_writable` call … increments the `SEQUENCE` counter
exactly once") is false against the current code.

**Test isolation comes from the runner, not from the code.** The `cli/`
workspace runs under `cargo nextest` (`tasks/test/cli.py:11-38`) and there is no
`nextest.toml` anywhere in the repo, so the default profile applies: one OS
process per test function. A `static AtomicU64` starts at zero in every test,
and `two_concurrent_first_use_resolves_both_succeed` (`resolution.rs:594-609`)
cannot perturb a counting test. The work item's stated ordering rationale — that
`cache_root::resolve`'s probing unit tests could run concurrently with the
counting tests in the same process — does not hold under nextest. The ordering
is retained because it is the right TDD sequence, not for that reason.

That isolation is an environmental property of the runner, invisible in the
source, so it is deliberately *not* what the counting assertions rest on. Under
a bare `cargo test` libtest runs test functions as parallel *threads of one
process* — an invocation `tasks/test/cli.py:26-29` keeps supported so the suite
"stays runnable on a machine without the toolchain" — and there a process-wide
counter would absorb probes from other tests in the same binary, with
`two_concurrent_first_use_resolves_both_succeed` alone contributing two. Making
the counter thread-local removes that exposure entirely: no runner precondition,
no `nextest.toml`, and no assertion whose failure shape is ambiguous between a
regression and cross-test interference.

**The refetch fault-injection seam already exists**, in a different shape from
the one the work item's Open Question anticipates. There is no verifier port —
verification is free functions called statically at `mod.rs:102` and `:160` —
and building one is the resolver restructure the item puts out of scope. It is
not needed: `resolution.rs:478-514` already produces both refetch outcomes by
poisoning the cached *bytes*, which works because the expected sha256 is parsed
out of the cache filename (`cache.rs:51-73`) rather than recomputed. Byte
poisoning is not "filesystem permissions on the cache root", so it discharges
what the acceptance criteria actually ask for.

**No fixture seeds the cache without resolving.** Every warm-cache test calls
`harness.resolve()` first (`resolution.rs:216, 463, 482, 498, 520, 553`), which
performs its own probe. The building block for a non-resolving seed is public
and already imported into the file's namespace: `cache::store`
(`cache.rs:81-116`).

**The work item's release blocker is stale**, though this plan is not the one it
blocks. The Dependencies section states 0189 cannot close before an epic-0136
release cut produces a signed `accelerator-vcs` asset; both `v1.24.0-pre.36` and
`v1.24.0-pre.35` ship it. That retraction belongs to the measurement plan, which
is the half the blocker was ever about.

**One shipped documentation statement is already wrong.**
`docs-site/src/content/docs/internals.md:277-280` tells users that dispatching
to a separate binary always makes the launcher probe the cache directory. 0169's
Phase 5 falsified this, and
`resolve_succeeds_from_a_read_only_cache_root_on_a_hit` pins the contrary.

### Key Discoveries:

- The counter must be new, not the existing `SEQUENCE`. `SEQUENCE.fetch_add`
  (`cache_root.rs:114`) runs after the `create_dir_all` early return
  (`:111-113`), so it cannot count invocations without changing its meaning.
- Byte poisoning (`resolution.rs:483`, `:499`) already produces both refetch
  outcomes; no verifier port is needed and none will be built.
- One process per test under nextest, with no config file to change it. Delta
  and absolute readings coincide there. The delta form is still required, but
  not for runner-independence — it is required because a single test process
  performs several resolutions
  (`each_of_two_cold_misses_probes_the_cache_root_once`), so an absolute read is
  meaningless the moment a second resolution enters the process. Neither form is
  sound under a bare `cargo test`, where libtest parallelises across threads in
  one process.
- Doctests are not run by nextest at all, so no assertion may live in a `///`
  example.
- `cache::store` (`cache.rs:81-116`) is the seeding primitive, but
  `happy_harness` computes `asset_sig` at `resolution.rs:166` and drops it — the
  `Harness` struct does not retain it.
- The workspace has twice refused a cargo feature as a test seam in writing
  (`cli/vcs-test-support/src/lib.rs:4-6`, `cli/vcs-adapters/Cargo.toml:27-30`),
  both times because CI passes `--all-features` workspace-wide. A bare `pub fn`
  sidesteps the argument; `cli/launcher/Cargo.toml` has no `[features]` block at
  all.
- Clippy runs pedantic + nursery with `warnings = "deny"`
  (`cli/Cargo.toml:133-147`). `missing_docs` is off, and `must_use_candidate` is
  explicitly allowed, so a `pub fn -> u64` reading an atomic trips nothing.
- `bin/.tmp-*` is gitignored (`.gitignore:56`), and the shell linter walks the
  tree honouring `.gitignore`, so a recovered baseline script parked there is
  invisible to both jj's auto-snapshot and the exec-bit invariant.

## Desired End State

`verify_writable` has one production call site and is `pub(super)`, so reaching
it from outside the resolver package — including from the composition root in
`main.rs`, a separate crate — is a compile error rather than a review question.
Inside that package the invariant stays test-enforced. Eight delta assertions in
`cli/launcher/tests/resolution.rs` — six new tests plus two retrofitted onto
existing ones — cover every branch that can reach the probe, and recorded
mutations demonstrate that each assertion can fail. `cache_root::resolve` is
gone, every assertion its tests made is discharged by a named test, and the
module doc describes the surface that survives. `internals.md` and
`CHANGELOG.md` describe warm sub-binary dispatch correctly.

The guard covers the resolver, not the whole process: the invariant the work
item states is per-dispatch, and these assertions are per-resolver. Narrowing
`verify_writable`'s visibility is what closes the gap, by making the composition
root unable to probe at all.

Verified by: `mise run` exits 0 end-to-end, and the Validation Results section
below is complete.

## What We're NOT Doing

- **No memoisation of the probe result.** The invariant is asserted by test and
  bounded by visibility. A process-wide cache would also change behaviour for
  `two_concurrent_first_use_resolves_both_succeed`, which deliberately resolves
  from two threads. This ban is correct only while a launcher process performs a
  single resolution — the assumption the work item records. Should a subcommand
  ever resolve several sub-binaries in one process (a prefetch or warm-cache
  command, given the growing `DISPATCHED_SUBBINARIES` set), per-process
  memoisation becomes the right design and
  `each_of_two_cold_misses_probes_the_cache_root_once` is the criterion to
  retire deliberately rather than fight.
- **No verifier port, and no injection of the probe behind a port the resolver
  holds.** Both are resolver restructures the work item puts out of scope.
  Weighed rather than merely deferred: an injected, per-resolver counter would
  remove both the public accessor and Phase 2's new coupling to `cache::store`
  (the warm-hit test could resolve once, then count against a second resolver),
  so the global counter is the more coupled option, not the less. It is chosen
  because the restructure is out of scope, and the cost is recorded here rather
  than presented as a win. Narrowing `verify_writable`'s visibility takes the
  part of the benefit that needs no restructure.
- **No cargo feature gate on the accessor.** Twice refused in writing elsewhere
  in this workspace, for a reason that applies here unchanged.
- **No `nextest.toml` test-group serialising the counting tests, and no recorded
  runner precondition.** Both existed to manage cross-test interference in a
  process-wide counter. The thread-local counter makes that interference
  impossible, so neither is needed — the assertions are sound under `cargo test`
  and nextest alike.
- **No `#[cfg(unix)]` gates added to the existing read-only tests, but the
  platform scope is stated.** The `cache_root` test module carries only
  `#[cfg(test)]` (`cache_root.rs:142`); it is unix-only merely because two
  unconditional `use std::os::unix::fs::PermissionsExt` statements inside test
  bodies would fail to compile elsewhere — and Phase 3 deletes one of them.
  Rather than add gates, Phase 3 records the POSIX-only scope of the probe in
  the module doc, so the `#[cfg(not(unix))]` arm at `:137-140` is a declared
  untested-platform stub rather than an implied promise.
- **No latency measurement.** It is the sibling plan's whole subject
  (`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`). Nothing
  here is gated on it, and this plan closes on CI evidence alone.
- **No change to `SEQUENCE`.** It keeps its filename-uniqueness meaning and its
  position after the `create_dir_all` guard.

## Implementation Approach

Four phases, stacked in this order. Each is individually shippable and leaves
the tree green, but they are not order-free: Phase 2's tests reference Phase 1's
accessor and will not compile without it. Phases 3 and 4 are genuinely
independent of each other and of Phase 2.

The counting seam is a new private **thread-local** count incremented as the
first statement of `verify_writable`, plus one `pub fn` reading it. This
separates two concepts currently at risk of being conflated — "probe sequence,
for path uniqueness" and "probe attempts made" — and makes the count equal the
invocation count on every path, including the one where directory creation fails
and no probe file is ever written. It deviates from the work item's stated
default, which names `SEQUENCE`; the deviation is deliberate and the
discriminating test is written first.

**Thread-local, not a process-wide atomic.** `verify_writable` runs
synchronously on its caller's thread, and every assertion in this plan reads the
count from the same thread that drove the calls — so a thread-local satisfies
all of them while a process-wide counter would additionally be incremented by
every other test sharing the process. That difference is not academic: under a
bare `cargo test`, libtest runs test functions as parallel threads in one
process, and `two_concurrent_first_use_resolves_both_succeed`
(`resolution.rs:594-609`) alone contributes two probes to any concurrent window.
A process-wide counter would make these assertions unsound under an invocation
`tasks/test/cli.py:26-29` explicitly keeps supported, and would then need that
dependency documented on the accessor, repeated in six assertion messages, and
recorded as a non-goal. A thread-local removes the problem rather than managing
it.

The one thing a thread-local cannot observe is a probe performed on another
thread. Nothing in this plan asserts that: the concurrent-first-use test makes
no count assertion, and the production invariant is per-resolution on the
dispatching thread.

The accessor is named for the invocation, not the outcome. `probes_performed`
would assert exactly what the discriminating test disproves — that a counted
call performed a probe — so it is `probe_attempts`, matching the name of the
cell it reads.

Every count assertion is a delta captured either side of the call under test,
permanently: a single test process performs several resolutions on one thread,
so an absolute read stops meaning anything the moment a second resolution enters
it.

Phase 2 applies its mutations **before** writing the tests, so each assertion is
observed red and then green. The invariant already holds, so a test written
against passing code is never forced to fail, and a mis-wired assertion — wrong
constant, `before` captured after the call, a silent minisign skip — would be
invisible. Mutating first turns the mutation exercise from post-hoc evidence
into the red step of a genuine red-green loop, and produces the same recording
the work item's criterion 6 requires.

## Phase 1: The Probe Counter Seam

### Overview

Add a per-thread count of `verify_writable` invocations and one public accessor,
driven by a unit test that discriminates invocation-counting from
write-counting.

### Changes Required:

The four items below run in the order given. An earlier draft put the `SEQUENCE`
discrimination first, but it instructs running a test defined in the item after
it, so the dependency was inverted.

#### 1. The discriminating unit tests (red)

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:
Two tests in the existing `mod tests`.

```rust
#[test]
fn each_verify_writable_call_counts_one_attempt(
) -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let before = probe_attempts();
    verify_writable(temp.path())?;
    verify_writable(temp.path())?;
    assert_eq!(probe_attempts() - before, 2);
    Ok(())
}

#[test]
fn a_probe_against_an_uncreatable_directory_still_counts(
) -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let blocker = temp.path().join("blocker");
    std::fs::write(&blocker, b"not a directory")?;
    let target = blocker.join("cache");
    let before = probe_attempts();
    assert!(
        verify_writable(&target).is_err(),
        "a directory beneath a regular file cannot be created"
    );
    assert!(
        !target.exists(),
        "create_dir_all must have failed, or this test no longer discriminates"
    );
    assert_eq!(probe_attempts() - before, 1);
    Ok(())
}
```

The second test is the one that discriminates: it counts a call where
`create_dir_all` fails and no probe file is written, chmodded or executed. It
passes with a counter incremented inside `verify_writable` and fails against
`SEQUENCE`, as item 2 records.

The uncreatable directory is a path beneath a regular file inside a temp dir, so
`create_dir_all` fails with `ENOTDIR` for **every** user including root, and
nothing is written outside the temp dir. Targeting a literal path under `/` and
relying on the runner not being root would fail two ways: under a root runner
the create succeeds, so the assertion fails for a reason unrelated to counting,
and it succeeds by creating a directory that
`candidate_performs_no_filesystem_write_or_process_spawn` (`:210-228`) asserts
does not exist — permanently breaking a neighbouring test on that machine or
image layer, and only on linux, since macOS denies the create even for root.

The non-existence assertion keeps the test discriminating. Asserting `is_err()`
alone would also pass on a host that mounts `TMPDIR` `noexec`, where the
directory *is* created and the probe fails later at the exec step — and in that
state a `SEQUENCE`-based counter would read a delta of 1 too, so the test would
silently stop distinguishing the thing it exists to distinguish.

Extend the test-module import at `cache_root.rs:149` to bring `probe_attempts`
into scope.

#### 2. Record the `SEQUENCE` discrimination (throwaway, recorded)

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs` (temporarily)
**Changes**: The evidence that `SEQUENCE` cannot serve is the sole justification
for deviating from the work item's named default, so it is produced explicitly
rather than asserted. `SEQUENCE` is declared *inside*
`probe_writable_and_executable`'s body (`:109`), so reading it requires hoisting
the static to file scope — which is why this is a throwaway step and not a
success criterion about the final state.

Hoist `SEQUENCE` to file scope, add a temporary `pub fn probe_attempts() -> u64`
reading it, and run the two tests from item 1. Expected:
`a_probe_against_an_uncreatable_directory_still_counts` fails with a delta of 0,
because `create_dir_all` returns at `:111-113` before `SEQUENCE.fetch_add` at
`:114`, while `each_verify_writable_call_counts_one_attempt` passes. Record both
outcomes in Validation Results, then revert the hoist entirely.

`each_verify_writable_call_counts_one_attempt` passes against both counters, so
this step never forces it red. Force it once by deleting the increment statement
from item 3 after that item lands, confirming it fails with a delta of 0, then
restore — otherwise the plan's only evidence for it is a compile error, and a
mis-wired assertion would be indistinguishable from a correct one.

#### 3. The counter and its accessor (green)

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:
A file-level thread-local beside the existing imports, an accessor, and one
added statement in `verify_writable`.

```rust
thread_local! {
    static PROBE_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
}

/// Calls to `verify_writable` on this thread, including those that fail
/// before writing anything — unlike `SEQUENCE`, whose increment sits after
/// the `create_dir_all` guard and so counts only probes that reached the
/// write stage.
///
/// A test-only observation point, `pub` because the launcher's integration
/// tests are a separate crate. Read as a delta either side of the call
/// under test.
#[must_use]
pub fn probe_attempts() -> u64 {
    PROBE_ATTEMPTS.with(Cell::get)
}
```

```rust
pub fn verify_writable(dir: &Path) -> Result<(), ResolutionError> {
    PROBE_ATTEMPTS.with(|attempts| attempts.set(attempts.get() + 1));
    if probe_writable_and_executable(dir) {
        Ok(())
    } else {
        // ... unchanged
    }
}
```

The signature stays `pub` here: narrowing it to `pub(super)` is Phase 3's change
and is announced there. Phase 1 adds one statement.

Add `use std::cell::Cell;` to the existing imports. `SEQUENCE` keeps its
`AtomicU64` — it guards filename uniqueness *across* threads and must stay
process-wide; the two are not interchangeable in either direction, which is the
distinction the doc comment records.

No memory ordering is involved at all, which is the point: a `Cell<u64>` on the
calling thread cannot be perturbed by any other test in the process, so the
assertions hold under `cargo test` as well as under nextest and need no recorded
runner precondition.

**Limit of the memoisation canary.** Because the increment is the first
statement of `verify_writable`, a memo placed *below* it inside the same
function would still record one attempt per call and leave
`each_of_two_cold_misses_probes_the_cache_root_once` green. The criterion
catches a memo at or above `verify_writable`'s entry — including the natural
placement, a guard at the `fetch_verify_store` call site. Stated here so the
criterion is not credited with more reach than it has. Mutation D exercises the
placement it does catch.

**Limit of the thread-local seam.** The bound is asymmetric. A probe *moved*
onto another thread still reddens the cold-miss deltas, because they would read
0 — the lower bound holds. A probe *added* on another thread (a prefetch, a
background warm, a `doctor` built-in spawning work) is invisible to every one of
the eight assertions, because the counter only sees the resolving thread. None
of the four mutations exercises that shape, so it is out of the guard's reach
and recorded as such rather than implied by the absence of a mutation.

#### 4. Correct the work item's counter definition

**File**: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
**Changes**: The discrimination recorded in item 1 falsifies three passages, all
of which are amended here rather than in the sibling measurement plan, because
they are consequences of this phase:

- The definitional sentence at `:37-38`, which says one `verify_writable` call
  "increments the `SEQUENCE` counter exactly once; 'probe count' always means
  that counter". False — `SEQUENCE.fetch_add` sits after the `create_dir_all`
  early return. "Probe count" means the `PROBE_ATTEMPTS` invocation count. Every
  count acceptance criterion is phrased against this definition, so leaving it
  stale means validating those criteria against a counter the implementation
  does not use.
- The Open Question at `:178-182` and Technical Notes at `:241-245`, both of
  which name `SEQUENCE` as the seam.
- The Requirements ordering rationale at `:114-119`, which justifies landing the
  deletion last because `cache_root::resolve`'s unit tests "can run concurrently
  with the counting tests in the same process". They cannot under nextest, and
  with a thread-local counter it would not matter if they did. The ordering is
  kept as the right TDD sequence; only its stated reason is wrong.
- The Requirements bullet at `:96-101`, which says "the counter is process-wide,
  so an absolute read is never the right observation". The counter is
  thread-local. The delta requirement survives, but its reason changes: a single
  test process performs several resolutions on one thread, so an absolute read
  stops meaning anything once a second resolution enters it.
- The isolation paragraph at `:125-129`, which requires every count criterion to
  be captured with "no other probe in flight in the same process — each counting
  test runs in its own test process, or is serialised against the
  concurrent-first-use tests and the `cache_root` unit tests". A thread-local
  makes cross-test interference impossible, and this plan explicitly declines to
  build that serialisation, so the precondition is retracted rather than left
  qualifying criteria the delivered tests do not need.

Retractions take the dated-note form the item itself records at `:274-277`.
`:96-101` and `:125-129` are consequences of the thread-local decision rather
than of the `SEQUENCE` discrimination, but they belong in the same edit: leaving
them would have the item's normative text demand a runner precondition the plan
refuses to provide.

While here, note against acceptance criteria 3 and 4 that "a test-only failing
verifier" is discharged by byte poisoning. The criteria's parenthetical only
excludes provoking failure via cache-root permissions, which byte poisoning
satisfies, and building a verifier port is out of scope by the item's own Open
Question — but a validator reading the criteria literally would look for a
mechanism that was deliberately not built.

### Success Criteria:

#### Automated Verification:

- [x] Both new tests fail to compile before **item 2** lands (`probe_attempts`
      is absent). Between items 2 and 3 they compile against the hoisted
      `SEQUENCE`; after item 3 both pass
- [x] Against the hoisted `SEQUENCE` accessor from item 2,
      `a_probe_against_an_uncreatable_directory_still_counts` fails with a delta
      of 0 while `each_verify_writable_call_counts_one_attempt` passes
- [x] With item 3's increment statement temporarily deleted,
      `each_verify_writable_call_counts_one_attempt` fails with a delta of 0
- [x] Unit tests pass: `ACCELERATOR_COVERAGE=off mise run test:unit:cli`
- [x] Single-test loop: `cargo nextest run --manifest-path cli/Cargo.toml -p
      accelerator --all-features -E 'binary(accelerator) and test(cache_root)'`
- [x] Rust format and clippy pass: `mise run cli:check` (cargo-deny and
      cargo-pup are siblings of it under `check`, not children —
      `mise.toml:575-577`)
- [x] `mise run check` exits 0

#### Manual Verification:

- [x] The `SEQUENCE` discrimination from item 2 is recorded in Validation
      Results with both outcomes, and the hoist is fully reverted
- [x] `verify_writable` is still `pub` at the end of this phase — the narrowing
      is Phase 3's change
- [x] `SEQUENCE` is unchanged in the final state — same declaration inside
      `probe_writable_and_executable`, same position after the `create_dir_all`
      guard, same doc comment
- [x] `a_probe_against_an_uncreatable_directory_still_counts` passes as root
      **on linux** (a root container is the intended environment; GitHub's macOS
      runners never run as root, and macOS refuses writes root would get on
      linux, so a darwin `sudo` run proves less and counts only as
      corroboration), confirming the fixture is privilege-independent. Compile
      as the normal user first (`cargo nextest run … --no-run`) and invoke the
      built test binary under `sudo` by absolute path, or run the check in a
      root container. Do **not** run `sudo cargo`: it compiles as root into the
      shared `cli/target/` and leaves root-owned artefacts that break the next
      non-root `mise run check`, and on Debian/Ubuntu `Defaults secure_path`
      overrides `PATH` even with `-E`, so a mise-provisioned `cargo` is not
      found

---

## Phase 2: The At-Most-Once Invariant Tests

### Overview

Assert the probe count delta across a single `resolve` call on every branch that
can reach the probe, with each assertion observed red under mutation before it
is made green.

### Changes Required:

#### 1. Harness seeding, clearing and offline resolution

**File**: `cli/launcher/tests/resolution.rs`
**Changes**: Retain the asset
signature `happy_harness` currently drops, and add three harness methods.

Add `asset_sig: String` **and `sha: String`** to the `Harness` struct
(`:101-113`), populating both from values `happy_harness` already computes and
drops — the signature at `:166` and the digest at `:165`. Retaining the digest
is what lets `seed_cache` reuse it rather than recomputing `sha256_hex` over the
fixture, which would repeat the very defect being fixed for the signature.

```rust
impl Harness {
    #[track_caller]
    fn seed_cache(&self) -> Result<cache::CachedBinary, Box<dyn Error>> {
        let cached = cache::store(
            &self.cache,
            BINARY,
            VERSION,
            &self.sha,
            &self.fixture_bytes,
            &self.asset_sig,
        )?;
        assert!(
            cache::find(&self.cache, BINARY, VERSION).is_some(),
            "a seeded entry must be findable, or the warm-path tests silently \
             degrade into cold-miss duplicates"
        );
        Ok(cached)
    }

    #[track_caller]
    fn clear_cache(&self) -> Result<(), Box<dyn Error>> {
        std::fs::remove_dir_all(&self.cache)?;
        std::fs::create_dir_all(&self.cache)?;
        assert!(
            cache::find(&self.cache, BINARY, VERSION).is_none(),
            "the cache must be empty, or the next resolution is a hit"
        );
        Ok(())
    }

    fn resolver_for(&self, base_url: String) -> FetchVerifyCacheResolver {
        FetchVerifyCacheResolver::with_fetcher(
            self.config(base_url),
            self.keys(),
            Fetcher::with_backoff(std::time::Duration::from_millis(1))
                .expect("fetcher"),
        )
    }

    fn resolve_offline(&self) -> Result<PathBuf, ResolutionError> {
        self.resolver_for("http://127.0.0.1:1".to_owned())
            .resolve(&ExternalCommand {
                name: OsString::from(BINARY),
                args: vec![],
            })
    }
}
```

`Harness::resolver` (`:132-148`) becomes
`self.resolver_for(self.server.base_url())`, so fetcher construction lives in
one place.

`seed_cache`'s return type is spelled `cache::CachedBinary` rather than imported
bare. The type is `pub` at `cache.rs:19`, but `resolve/mod.rs:15` re-imports it
privately (`use self::cache::CachedBinary;`), so it is not reachable through the
`resolve::{…}` group — only `resolve::cache::CachedBinary` resolves from the
test crate, and the `cache` import added below already covers it.

`seed_cache` writes a genuinely verifiable entry — real fixture bytes, their
real sha256, a real signature over them by the trusted key — with zero prior
probes, which is what lets the warm-hit delta of 0 mean what it claims. Its
postcondition is asserted rather than assumed: three of the new tests expect a
delta that a cold miss would also produce, so a seed that stopped being findable
would leave them passing while testing something else.

Both fixture helpers carry `#[track_caller]` for the same reason `probes_during`
does: five tests share `seed_cache`, and a findability-postcondition failure —
the assertion the plan calls load-bearing — should name the test that seeded,
not the helper.

`clear_cache` replaces the directory rather than unlinking entries from a live
`read_dir` stream. POSIX leaves it unspecified whether entries removed after the
stream was opened are still returned, and a skipped binary entry would turn the
second resolution in `each_of_two_cold_misses_probes_the_cache_root_once` into a
cache *hit* — failing with a message pointing at the probe invariant rather than
at the helper. It is `clear_cache`, not `empty_cache`, because it is a command
and `empty`/`is_empty` reads as a predicate.

`resolve_offline` collapses the unreachable-server construction that
`resolution.rs` currently repeats verbatim three times (`:465-473`, `:501-510`,
`:531-539`); the new failed-refetch test would have been the fourth. **Update
all three existing sites to use it** — otherwise the file gains an abstraction
that abstracts nothing and still carries four copies.

`seed_cache` returns the `CachedBinary` that `cache::store` already hands back
rather than narrowing to `path`; the sidecar test then writes to
`seeded.signature_path` directly instead of re-querying with a second
`cache::find` and an `ok_or`. Narrowing here would repeat the very defect the
plan flags in `happy_harness`, which computes `asset_sig` and drops it.

Both new fallible helpers return `Box<dyn Error>`, matching every test signature
in the file — `seed_cache` performs no resolution, so leaking `ResolutionError`
out of a fixture would be arbitrary.

Add `cache` to the existing grouped import at `:23-25` and collapse the
fully-qualified `cache::find` call at `:521` to match, so the module is not
addressed two ways in one file. Add an import for
`accelerator::launch::outbound::resolve::cache_root::probe_attempts`.

#### 2. The mutations (applied first, to make the red step real)

**File**: `cli/launcher/src/launch/outbound/resolve/mod.rs` (temporarily)
**Changes**: Four temporary probe mutations, applied and reverted in turn.

The invariant already holds, so a test written against unmutated code passes on
first run and is never forced to fail. The mutations are therefore applied
*before* the tests are written, each test is observed red against them, and only
then are they reverted to green. This is the red step, and it is also the
evidence the work item's criterion 6 asks for.

**Mutation A — a second probe in `fetch_verify_store`** (duplicate the call at
`:141`):

```rust
fn fetch_verify_store(&self, name: &str) -> Result<PathBuf, ResolutionError> {
    cache_root::verify_writable(&self.config.cache_root)?;
    cache_root::verify_writable(&self.config.cache_root)?;
    // ... unchanged
```

**Mutation B — a probe on the warm path** (first statement of
`ResolveBinary::resolve`, `:181`):

```rust
fn resolve(
    &self,
    command: &ExternalCommand,
) -> Result<PathBuf, ResolutionError> {
    cache_root::verify_writable(&self.config.cache_root)?;
    // ... unchanged
```

**Mutation C — a retry around the probe** (replace the call at `:141`):

```rust
if cache_root::verify_writable(&self.config.cache_root).is_err() {
    cache_root::verify_writable(&self.config.cache_root)?;
}
```

**Mutation D — memoise the probe result** (wrap the call at `:141`):

```rust
static PROBED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
let mut result = Ok(());
PROBED.get_or_init(|| {
    result = cache_root::verify_writable(&self.config.cache_root);
});
result?;
```

Four mutations, because each pins a bound the others cannot. A alone pins only
the upper bound on the success path. B pins the warm-hit zero — the property
0169 actually delivered, and the regression this item exists to prevent. C pins
the *failing*-probe delta, which neither A nor B can touch: under A the first
probe returns `Err` and `?` propagates before the second call runs, and under B
the injected probe fails against the `0o555` root and returns early, so
`an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` records 1 under
both and its assertion would otherwise never be demonstrated capable of failing.
C is also the shape of the plausible real regression the plan names — "repair
the cache root before retrying". D pins the memoisation canary: under A and B,
`each_of_two_cold_misses_probes_the_cache_root_once` reddens for the same reason
as six other rows, so nothing yet shows it catches anything
`a_cold_miss_probes_the_cache_root_exactly_once` does not. Under D it reddens
for a reason unique to it.

D reddens exactly the two tests that perform **more than one resolution in one
process** — `each_of_two_cold_misses_…` and, because it resolves once before the
measured bracket (`resolution.rs:520`),
`a_signature_read_io_error_propagates_the_refetch_error_verbatim`. Every other
row stays green: each nextest process starts with a fresh `OnceLock`, so a
single resolution probes normally. That second red is expected and is recorded
as such, not treated as noise.

**A fifth, fixture mutation — a degraded seed** (in `seed_cache`, store under a
wrong version). It is deliberately not lettered into the A–D table: those four
mutate the probe and are read as deltas, whereas this one makes tests fail on a
*postcondition* rather than on a count, so it has no column.
The three fixture postconditions the plan calls load-bearing — `seed_cache`'s
`is_some()`, `clear_cache`'s `is_none()` and the retrofit's `is_some()` — are
asserted but never demonstrated capable of failing, and all four probe mutations
leave them untouched. One run discharges all three: expect
`a_warm_hit_never_probes_the_cache_root`, both byte-poisoning refetches and the
sidecar refetch to fail on the *seed postcondition* rather than on their deltas,
which is exactly the silent-degradation mode the postcondition exists to catch.
Record it and revert.

Mutation D's column is only well-defined under `cargo nextest`'s
one-process-per-test model. Under a bare `cargo test` the memo is shared across
the whole binary, so whichever test reaches `fetch_verify_store` first wins it
and the rest read 0 non-deterministically. Record D's sweep under nextest and
name the command.

Expected results across all eight delta-bearing tests. Rows marked ✗ are red
under that mutation:

| Test | Baseline | A | B | C | D |
| --- | --- | --- | --- | --- | --- |
| `a_cold_miss_probes_the_cache_root_exactly_once` | 1 | 2 ✗ | 2 ✗ | 1 | 1 |
| `a_warm_hit_never_probes_the_cache_root` | 0 | 0 | 1 ✗ | 0 | 0 |
| `a_successful_refetch_probes_the_cache_root_exactly_once` | 1 | 2 ✗ | 2 ✗ | 1 | 1 |
| `a_failed_refetch_probes_the_cache_root_exactly_once` | 1 | 2 ✗ | 2 ✗ | 1 | 1 |
| `a_refetch_after_a_benign_cache_io_error_probes_exactly_once` | 1 | 2 ✗ | 2 ✗ | 1 | 1 |
| `each_of_two_cold_misses_probes_the_cache_root_once` | 1, 1 | 2 ✗ | 2 ✗ | 1, 1 | 1, 0 ✗ |
| `a_signature_read_io_error_propagates_the_refetch_error_verbatim` | 1 | 2 ✗ | 2 ✗ | 1 | 0 ✗ |
| `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` | 1 | 1 | 1 | 2 ✗ | 1 |

**Which mutation each assertion is authored under.** "Mutations first" is not a
single pass: no one mutation reddens every row. Enumerated by bracket, since
counts are ambiguous — under **A**: `a_cold_miss_…`, `a_successful_refetch_…`,
`a_failed_refetch_…`, `a_refetch_after_a_benign_cache_io_error_…`,
`each_of_two_cold_misses_…`'s *first* bracket, and the sidecar retrofit's delta.
Under **B**: the warm-hit delta. Under **C**: `an_unwritable_cache_root_…`'s
delta. Under **D**: `each_of_two_cold_misses_…`'s *second* bracket. Then run the
full 4 × 8 sweep for the recorded table. Without this pairing, whichever
assertions happen to be green under the mutation in force at authoring time are
written against passing code — the exact failure mutations-first exists to
prevent.

Two collateral results to record rather than treat as noise. Mutation A reddens
`a_signature_read_io_error_propagates_the_refetch_error_verbatim`, so the claim
that A perturbs no existing test is wrong — it reaches `fetch_verify_store` and
therefore doubles. Mutation B reddens the pre-existing
`resolve_succeeds_from_a_read_only_cache_root_on_a_hit` (`resolution.rs:549`):
with the cache chmodded `0o555` the injected probe cannot write and the
resolution errors out. That one is corroborating — it shows the warm-path
property already has a behavioural guard independent of the new counter.

Note that `an_unwritable_cache_root_…` stays green under B for a reason
unrelated to the invariant: the injected probe fails before `cache::find` is
reached, so the count is 1 by short-circuit rather than by preservation. Record
it that way.

All four probe mutations and the fixture mutation are reverted before the phase
closes.

#### 3. The delta assertions

**File**: `cli/launcher/tests/resolution.rs`
**Changes**: Six new tests, one per
branch that can reach the probe plus the two-resolution memoisation guard, and
delta assertions retrofitted onto two existing tests — **eight** in total.

The before/after bookkeeping is extracted rather than repeated:

```rust
#[track_caller]
fn probes_during<T>(
    branch: &str,
    expected: u64,
    action: impl FnOnce() -> T,
) -> T {
    let before = probe_attempts();
    let result = action();
    assert_eq!(
        probe_attempts() - before,
        expected,
        "{branch}: expected {expected} probe attempt(s)"
    );
    result
}
```

`#[track_caller]` makes a failure report the call site rather than the helper's
own line — without it all eight failures point at one line and name no branch.
The `branch` label is what the message carries.

```rust
#[test]
fn a_cold_miss_probes_the_cache_root_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    probes_during("cold miss", 1, || harness.resolve())?;
    Ok(())
}

#[test]
fn a_warm_hit_never_probes_the_cache_root() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    probes_during("warm hit", 0, || harness.resolve())?;
    Ok(())
}

#[test]
fn a_successful_refetch_probes_the_cache_root_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.path, b"poisoned")?;
    let healed = probes_during("refetch (checksum)", 1, || harness.resolve())?;
    assert_eq!(
        std::fs::read(&healed)?,
        harness.fixture_bytes,
        "replace-in-place self-heal: the poisoned bytes must be replaced"
    );
    Ok(())
}

#[test]
fn a_failed_refetch_probes_the_cache_root_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.path, b"poisoned")?;
    let result =
        probes_during("refetch failed", 1, || harness.resolve_offline());
    assert!(matches!(
        result,
        Err(ResolutionError::CorruptCacheAndRefetchFailed { .. })
    ));
    Ok(())
}

#[test]
fn a_refetch_after_a_benign_cache_io_error_probes_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.signature_path, [0xFF, 0xFE, 0xFD])?;
    probes_during("refetch (cache I/O)", 1, || harness.resolve())?;
    Ok(())
}

#[test]
fn each_of_two_cold_misses_probes_the_cache_root_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    probes_during("first cold miss", 1, || harness.resolve())?;
    harness.clear_cache()?;
    probes_during("second cold miss", 1, || harness.resolve())?;
    Ok(())
}
```

`each_of_two_cold_misses_…` asserts **per resolution**, not a total of 2. A
single bracket around both resolutions would pass an implementation that probed
0 then 2, which its name denies; only the co-existence of the cold-miss test
would have made the per-resolution reading true, and that coupling is invisible
from the test.

Both `seed_cache` consumers use the returned `CachedBinary` — `seeded.path` for
the byte-poisoning pair, `seeded.signature_path` for the sidecar test — so no
test re-queries with `cache::find` for something the helper already computed.

`a_refetch_after_a_benign_cache_io_error_probes_exactly_once` covers the third
`fetch_verify_store` call site. Both byte-poisoning tests fail the sha256 check
and so always route through the `ChecksumMismatch | SignatureMismatch` arm at
`:203-217`, leaving the plain cache-I/O arm at `:222-228` uncounted. That arm is
a distinct `return self.fetch_verify_store(name)`, it reads and rewrites the
cache, and it is a plausible place for someone to add a "repair the cache root
before retrying" step. The arrangement is the one
`a_signature_read_io_error_propagates_the_refetch_error_verbatim` (`:517-546`)
already uses — invalid UTF-8 in the sidecar — but seeded and resolved against
the live server so the refetch succeeds.

**Also add a delta to that existing test**, which is what actually *pins* the
arm. The new test's observable outcomes (delta 1, `Ok`) are shared with a cold
miss and with the checksum arm, so it reaches `:222-228` today only by virtue of
`reverify` mapping an unreadable sidecar to `ResolutionError::Cache`; a change
mapping it to `SignatureMismatch` would silently relocate the test onto an
already-covered arm.
`a_signature_read_io_error_propagates_the_refetch_error_verbatim` asserts
`Err(ResolutionError::Fetch { .. })`. That distinguishes `:222-228` from `:211`,
which maps every refetch failure to `CorruptCacheAndRefetchFailed` — but not
from the cold-miss tail call at `:231`, which also yields `Fetch` when the
manifest fetch fails
(`a_persistent_server_error_gives_up_after_bounded_retries`,
`resolution.rs:413-424`). The findability assertion below is what excludes that,
and is therefore load-bearing rather than belt-and-braces. Wrapping its offline
`resolve` call in `probes_during("sidecar I/O refetch", 1, …)` both pins the
branch by error shape and covers the failing-refetch half that no other test
reaches. Add `assert!(cache::find(&harness.cache, BINARY, VERSION).is_some())`
after the sidecar is corrupted, so the branch claim is enforced rather than
inferred: a future `find` that validated the sidecar would make the entry
unfindable, route the test through the cold-miss tail call at `:231`, and still
produce `Fetch` with a delta of 1.

**And add a delta to
`an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss`** (`:568`). All
six new tests run against a writable cache root and therefore count only
*successful* probes; the failing-probe path is asserted at resolver level
nowhere. A retry around `verify_writable` inside `fetch_verify_store` —
precisely the "repair the cache root before retrying" shape named above, and
Mutation C — would leave every happy-path delta at its expected value and be
caught by nothing.

**Do not use `probes_during` here.** That test chmods the cache root to `0o555`,
calls `resolve`, and restores `0o755` *before* asserting, so a failed assertion
cannot leave a directory the temp-dir guard is unable to unlink. The helper
asserts inside the closure's window, so a delta mismatch panics before the
chmod-back runs. The general invariant is that a delta assertion must never fire
inside a permissions window, because the restore is not executed on the
unwinding path. (For this particular test the leak is benign — resolution fails
at the probe, so the cache directory is empty and an empty directory is
removable given write permission on `TMPDIR`. The hazard is real for
`resolve_succeeds_from_a_read_only_cache_root_on_a_hit`, whose cache holds the
binary and its sidecar and so cannot be unlinked at `0o555`.) Capture
`before`/`after` around the `resolve` call and assert **after** the permission
restore, alongside the existing assertions. The same applies to
`resolve_succeeds_from_a_read_only_cache_root_on_a_hit` if a delta is ever added
there.

`a_successful_refetch_…` asserts the healed bytes as well as the delta, but on
its own merits — it pins replace-in-place self-healing. It is **not** what
distinguishes this test from a cold miss: `cache::store` derives the cache path
deterministically from `{name}-{version}-{sha256}`, so a degraded seed would
make `resolve` a cold miss that writes the *same* path with the *same* fixture
bytes, and the assertion would pass unchanged. The discriminator is
`seed_cache`'s findability postcondition, which is why that assertion lives in
the helper and must not be trimmed as redundant.

The refetch tests seed rather than resolve, even though only the warm-hit
criterion demands it. A seeded start makes each expected delta exactly 1 with no
prior probe anywhere in the test, so a failure localises to the call under test.

`each_of_two_cold_misses_probes_the_cache_root_once` is the criterion that fails
under memoisation; every other delta above passes against a memoising
implementation. `Harness::resolver` builds a fresh resolver on every `resolve()`
call (`resolution.rs:132-148`), so nothing resolver-held carries across the two
resolutions and the pair is a genuine test of the probe path rather than of
resolver state.

The existing poisoning tests (`:478-514`) keep their behavioural assertions
untouched. They are not re-homed onto the seed helper — they assert refetch and
diagnostic behaviour, and their use of a prior resolve is part of what they
describe.

### Success Criteria:

#### Automated Verification:

- [x] Every cell of the Mutation A/B/C/D table is observed and recorded,
      including the greens: each of the eight delta-bearing tests against each
      of the four mutations, under `cargo nextest`. Command and full output
      recorded in Validation Results
- [x] The two collateral results are recorded as such — Mutation A reddening
      `a_signature_read_io_error_propagates_the_refetch_error_verbatim`, and
      Mutation B reddening
      `resolve_succeeds_from_a_read_only_cache_root_on_a_hit`
- [x] The recorded mutation output contains **no** `skipping: minisign not on
      PATH` line — the macro returns `Ok(())`, which nextest reports as PASS, so
      without this check the whole exercise can be a false negative that looks
      like valid evidence. Run with `--no-capture` so per-test stderr is visible
- [x] All four mutations are reverted and the suite is green again
- [x] Integration tests pass: `cargo nextest run --manifest-path cli/Cargo.toml
      -p accelerator --all-features -E 'binary(resolution)'`
- [x] Whole cli suite passes: `ACCELERATOR_COVERAGE=off mise run test:unit:cli`
      (nextest runs every target in the workspace, integration binaries
      included)
- [x] `mise run cli:check` exits 0
- [x] `mise run check` exits 0

#### Manual Verification:

- [x] `jj diff` after the mutation exercise shows no residue in `mod.rs`
- [x] `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` asserts its
      delta **after** restoring `0o755`, not inside `probes_during`
- [x] `seed_cache` returns `CachedBinary` and no test re-queries with
      `cache::find` for a value the helper already returned

---

## Phase 3: Remove the Second Probe Entry Point

### Overview

Delete `cache_root::resolve`, shrink the probe's reachable surface to the
resolver package so the composition root cannot probe, re-home the unit tests
that assert something not otherwise covered, and correct the two documentation
statements 0169 falsified.

The narrowing is a boundary change, not an invariant proof: inside the package
the at-most-once property stays test-enforced. Phase 3 item 1 states exactly
what it does and does not buy.

### Changes Required:

#### 1. Delete the wrapper and narrow the probe

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:
Remove `resolve` and its doc comment (`:66-77`), and drop `resolve` from the
test-module import at `:149`.

Deleting the wrapper is dead-code removal — it has no production caller, so on
its own it constrains nothing. Narrowing the probe does constrain something:

```rust
pub(super) fn verify_writable(dir: &Path) -> Result<(), ResolutionError> {
```

`pub(super)` rather than `pub(in crate::launch::outbound::resolve)`:
`cache_root` is a direct child of `resolve`, so the two are exactly equivalent,
`pub(super)` fits on one line, and it is the only *module-scoped*
restricted-visibility form used in the `cli/` workspace
(`cli/vcs-adapters/src/library/dirty_paths.rs:30`, `:71`). `pub(crate)` is used
widely — 56 times across 26 files — but crate scope is not the constraint wanted
here.

The unit tests are in-module, and the only reference from the integration crate
is an assertion *string* at `resolution.rs:589`, so nothing breaks. The gain is
bounded and worth stating precisely: it shrinks the probe's reachable surface
from crate-public to this one adapter package, which closes the composition-root
gap — `LazyProductionResolver::resolve` in `main.rs:65` already calls
`cache_root::candidate` and would need one more line to probe, and because
`main.rs` is a separate crate from the `accelerator` lib that line now cannot
compile.

It does **not** make the at-most-once invariant compiler-enforced. Every call
site that can violate it — `fetch_verify_store` and `ResolveBinary::resolve`,
both in `resolve/mod.rs` — is inside the permitted scope, and both of Phase 2's
mutations compile unchanged under the narrowing. Within this package the
invariant remains test-enforced by the eight delta assertions. What the
narrowing buys is that `FetchVerifyCacheResolver` becomes the sole owner of
cache-root writability, so a future `prefetch`, `warm-cache` or `doctor`
built-in must route through the resolver rather than re-widen the probe.

Demote the intra-doc link at `cache_root.rs:41` — which references
`verify_writable` from the still-public `candidate`'s doc comment — to a plain
code span. Unconditionally, not "if rustdoc objects": nothing in this repo runs
`cargo doc` or rustdoc (`mise run cli:check` is rustfmt plus clippy), so the
lint can never fire and a conditional resolves to doing nothing, leaving a
public doc comment linking to a non-public item.

`probe_attempts` stays `pub`: the integration tests are a separate crate.

#### 2. Rewrite the module doc

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:
Replace the module header (`:1-6`).

The header describes the function being deleted — "Resolves the runtime cache
directory. `${ACCELERATOR_PLUGIN_ROOT}/bin` when writable and exec-capable, else
the `ACCELERATOR_CACHE_DIR` override, else a named error… Read-only/noexec roots
are probed." No grep catches it, because it names no symbol. Its stated
precedence is also backwards: `candidate` checks the override *first*,
unconditionally. After the deletion no function in the module implements the
writability-conditioned fallback it advertises, and "read-only/noexec roots are
probed" is the same class of claim 0169's Phase 5 falsified and that item 4
below corrects in `internals.md`.

The replacement, verbatim:

```rust
//! Selects and probes the runtime cache directory.
//!
//! [`candidate`] selects: the `ACCELERATOR_CACHE_DIR` override when set, else
//! `${ACCELERATOR_PLUGIN_ROOT}/bin`. Selection only — no filesystem access —
//! so a warm cache hit pays nothing here. There is no XDG fallback: an
//! XDG-resident binary would break the plugin-root `allowed-tools` glob match.
//!
//! Cache-root writability is owned by the resolver — new callers route
//! through it rather than probing directly.
//!
//! Builds only for the platforms `HOST_PLATFORM` names — linux and macOS on
//! x86_64 and aarch64. The `#[cfg(not(unix))]` `make_executable` arm is
//! unreachable dead code, retained only as a marker.
```

Deliberately shorter than an earlier draft, which restated four things the items
already document and two it cannot support. `candidate`'s precedence is in its
own doc comment (`:37-41`), the noexec rationale in
`probe_writable_and_executable`'s (`:101-102`), and the write-path restriction
in `verify_writable`'s (`:81-82`) — a header duplicating them can drift from
them in four independent ways.

Two clauses are dropped outright rather than shortened. "Module-scoped so no
caller outside this package can reach it" is wrong twice: `pub(super)` covers
the module *and its descendants*, and `main.rs` is in the same Cargo package —
"package" is the wrong word for the boundary, and the modifier on the item is
self-documenting anyway. "At most once per resolution" is a property of
`resolve/mod.rs`'s control flow that this file can neither enforce nor verify,
which is structurally the same mistake as the old header's "read-only/noexec
roots are probed" — a claim about behaviour elsewhere that went stale silently.

What survives is the ownership sentence, which is intent rather than mechanism
and so outlives a refactor: it tells the next person adding a `prefetch`,
`warm-cache` or `doctor` built-in to route through the resolver rather than
re-widen the probe.

**Also update the sibling header** at `resolve/mod.rs:1-2`, which reads
"composed from a fetcher, verifier, cache store, and **resolved** cache root".
After the deletion nothing resolves a cache root: `main.rs:65` composes
`ResolverConfig` from `candidate` (selection only) and the probe happens later
inside `fetch_verify_store`. "Selected cache root" is accurate. The same
reasoning that requires the `cache_root` rewrite applies here — no grep catches
it, because it names no symbol.

#### 3. Re-home two tests, replace one, delete one

**File**: `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
**Changes**:

`unset_plugin_root_with_no_override_is_a_named_error` (`:164`) moves onto
`candidate` unchanged apart from the call — the error originates at
`cache_root.rs:56-62` and `resolve` only forwarded it.

```rust
let result = candidate(&config());
```

`an_override_is_honoured` (`:256`) moves onto `candidate`. Since `candidate`
performs no filesystem access, the temp directory is dropped in favour of a
literal path, and the test is renamed to say why that is safe: the override is
used verbatim, with no `bin` suffix appended and no filesystem touched. If
`candidate` ever gained a canonicalisation or existence check, the name is what
surfaces the coupling.

```rust
#[test]
fn an_override_is_used_verbatim_without_touching_the_filesystem(
) -> Result<(), Box<dyn Error>> {
    let override_dir = PathBuf::from("/some-override-dir");
    let resolved = candidate(&CacheRootConfig {
        cache_dir_override: Some(override_dir.clone()),
        ..config()
    })?;
    assert_eq!(resolved, override_dir);
    assert!(!override_dir.exists(), "candidate must not create any directory");
    Ok(())
}
```

The non-existence assertion is what makes the name true — without it the name
promises a property nothing checks, and a `candidate` that grew a
`create_dir_all` would pass under a root runner *and* leave a directory at the
filesystem root, the same residue hazard the Phase 1 fixture was rearranged to
avoid. It mirrors `candidate_performs_no_filesystem_write_or_process_spawn`
(`:223-226`).

`a_writable_plugin_root_is_used` (`:179`) is **replaced, not deleted**. Two of
its three assertions are indeed already discharged — the resolved path by
`candidate_performs_no_filesystem_write_or_process_spawn` (`:210`), the
writability by `verify_writable_accepts_a_writable_directory` (`:230`). The
third is not: it passed a bare temp dir as `plugin_root`, so the probed
`temp/bin` **did not exist**, and the test passed only because
`probe_writable_and_executable` calls `create_dir_all` at `:111`. Every other
candidate discharger passes an already-existing directory, so deleting this
outright would leave the documented create-if-needed behaviour (`:79`) untested
— and removing or reordering that `create_dir_all` would break first-run
resolution under an `ACCELERATOR_CACHE_DIR` override pointing at a fresh path
with the whole suite green.

```rust
#[test]
fn verify_writable_creates_a_missing_directory(
) -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let target = temp.path().join("bin");
    verify_writable(&target)?;
    assert!(target.is_dir(), "the probe must create a missing cache root");
    Ok(())
}
```

`a_read_only_plugin_root_with_no_override_is_a_named_error` (`:191`) is deleted.
It is substantially a `verify_writable` test and
`verify_writable_rejects_a_read_only_directory` (`:238`) already asserts it —
which the work item explicitly permits.

**Force the replacement test red once.** Phases 1 and 2 apply a mutations-first
discipline on the grounds that an assertion written against passing code is
never proved capable of failing; `verify_writable_creates_a_missing_directory`
is otherwise the one new assertion in the plan exempt from it — and it is the
sole guard on behaviour the plan says would otherwise break first-run resolution
silently. Delete the `create_dir_all` at `cache_root.rs:111` (or make it a no-op
returning `true`), confirm it is the only test that reddens and note *which*
assertion fires, restore, and record it. Do the same for the
`assert!(!override_dir.exists(), …)` added to the re-homed override test by
temporarily giving `candidate` a `create_dir_all`.

The honest mapping is two re-homed, one replaced by a narrower test naming the
assertion that would otherwise have been lost, and one already covered. It is
recorded that way in Validation Results rather than padded to four with
redundant tests.

#### 4. Correct the read-only install documentation

**File**: `docs-site/src/content/docs/internals.md`
**Changes**: In the
"Offline, mirrored and read-only installs" paragraph, replace from `That
exemption` on line 277 through line 280. Line 277 begins mid-sentence —
`invocations. That exemption stops at the bootstrap: running any subcommand
that` — so the leading `invocations.` completes the previous sentence and must
survive; taking `:277-280` wholesale would ship broken prose. The block below
therefore carries that fragment at its head and is wrapped with it included, so
the plan shows what the file will literally contain rather than a block that
breaks 80 columns once spliced.

```markdown
invocations. Sub-binary dispatch follows the same rule, with warm meaning what
it does for the bootstrap: a cached binary that re-verifies successfully. Such a
dispatch resolves from the cache and re-verifies what it finds there without
writing or probing, so it too tolerates a read-only cache directory. Only a cold
dispatch probes — a first use of that subcommand, the first run after a version
bump, or a run where re-verification fails and the binary must be refetched.

A cold dispatch against a cache directory that is not writable and exec-capable
fails at the probe with a `no usable cache directory` error naming the
directory. What that means for the caller depends on why the dispatch went cold,
and on `--fail-safe` — a flag the plugin's own hooks and skills pass so a
launcher failure degrades rather than breaks the session. Under it, a first-use
or version-bump miss exits 0: the subcommand simply does not run, with only a
warning on stderr. So does a cached copy that could not be *read*. But a cached
copy that fails its checksum or signature check and cannot then be refetched is
reported as confirmed tampering — a `cached copy failed verification` message
with the probe error nested inside it — which `--fail-safe` never swallows: it
exits 2, and for the `PreToolUse` guard that blocks the tool call rather than
letting it through.

A cache directory can therefore only be kept read-only for a fixed set of
subcommands at a fixed version. Make it writable and exec-capable, run every
subcommand you intend to use — including the ones the plugin dispatches for you,
such as `vcs`, which the git guard runs on every Bash tool call — then set it
read-only again, and repeat after each plugin upgrade, since a version bump
makes every subcommand cold again. A subcommand left cold does not fail loudly
under `--fail-safe`; it silently does not run. If that upkeep is impractical,
point `ACCELERATOR_CACHE_DIR` at a writable, exec-capable directory instead.
```

Hard-wrapped at 80, matching the surrounding file and `.editorconfig`. Anchoring
"warm" on successful re-verification rather than on being a cache hit matters: a
hit whose re-verification fails *does* write, so the looser reading would be
wrong.

Four details the closing advice has to get right. The launcher's error is *not*
the bootstrap's — `no usable cache directory: … is not writable+exec-capable`
(`core.rs:153-155`, `cache_root.rs:92-98`) against the bootstrap's `no writable,
exec-capable cache directory …` (`bin/accelerator:208-211`) — so the paragraph
must name it rather than say "the same".

**`--fail-safe` swallows two of the three cold cases, and the split is not where
it first appears.** `reverify` has two failure arms and they diverge. A checksum
or signature failure routes through `mod.rs:203-217`, which wraps **any**
refetch error — including the `CacheRootUnavailable` from a read-only root —
into `CorruptCacheAndRefetchFailed`, mapping to `kernel::Error::Refusal`
(`core.rs:170-177`), which `swallow_under_fail_safe` never swallows: exit 2, and
for a `PreToolUse` hook that is a block. A *plain cache I/O* failure — an
unreadable binary, or the invalid-UTF-8 sidecar that
`a_signature_read_io_error_propagates_the_refetch_error_verbatim` exercises —
routes through the sibling arm at `mod.rs:222-228`, which returns the refetch
error **unwrapped**, so it stays `CacheRootUnavailable` → `Failed`
(`core.rs:186-189`) → swallowed (`core.rs:219-224`) → exit 0. A first-use or
version-bump miss is swallowed the same way.

Both over-broad readings are therefore wrong, and the shipped sentence must
avoid each: "the guard degrades silently" is false for the integrity case, and
"a failed re-verification is never swallowed" is false for the I/O case. The
wording above names the integrity check specifically and states the exit-2/block
consequence, which is the part that matters to an operator.

Warming is per sub-binary and per version, since `cache::find` keys on
`expected_version`, so "warm it once" understates what a read-only cache costs
to maintain. Say "writable and exec-capable" rather than just "writable" where
the remedy is given: the probe also fails on a `noexec` mount, and the error the
operator will read says `is not writable+exec-capable` (`cache_root.rs:92-98`).

**`ACCELERATOR_<TOKEN>_BIN` is deliberately *not* offered as a remedy**, though
it looks apt for this audience. It returns its path unverified, before any
fetch, checksum or signature check (`launch/outbound/mod.rs:12-23`) — which is
why the plugin's own text warns it "bypasses SHA-256 verification; use for local
dev builds" (`skills/visualisation/visualise/SKILL.md:94`). Recommending it in a
section framed around "both are trust-root inputs rather than ordinary
conveniences" would undercut that framing. The pages documenting it
(`corpus.md:90`, `collaboration.md:71`) do so under `## Local development`, not
as an offline remedy. And adding it would contradict the section's own lead
sentence ("Two environment variables cover the awkward cases") and its two-row
table, which `tasks/README.md:429-435` reserves for launcher-wide inputs, with
per-sub-binary overrides belonging on each sub-binary's page. An air-gapped
remedy here is a separate change that updates lead sentence, table and trust
caveat together.

#### 5. Correct the same claim in the changelog

**File**: `CHANGELOG.md`
**Changes**: In the `[Unreleased]` → Changed entry (`:23-29`), replace the
parenthetical "(dispatching a subcommand to a separate
binary still needs it writable)" with:

Replace `CHANGELOG.md:28-29` — from `(dispatching` on line 28 through
`writable).` on line 29. Line 28 begins mid-sentence, so the replacement starts
after `invocations ` and is shown here with that retained fragment included, so
what the file will literally contain is what the plan shows:

```markdown
  invocations (dispatching a subcommand to a separate binary needs it
  writable only on a cold dispatch — a first use of that subcommand, the first
  run after a version bump, or a run where the cached copy fails
  re-verification).
```

Written out verbatim rather than described, because this is the wording that
ships: a criterion of "agrees with the `internals.md` paragraph" would be
satisfied by text inheriting that paragraph's inaccuracies into a second file.
The third trigger is included for the same reason — an enumeration that reads as
exhaustive but omits the self-heal case supports exactly the wrong conclusion
("warm it once and it stays read-only forever"). The closing full stop is
restored; the text being replaced carries one.

It is false for exactly the reason `internals.md:277-280` is, and 0186
documented this behaviour in both files as a pair — so correcting one leaves
them contradicting each other. The changelog is also what a user reads at
upgrade time, and the claim ships with 1.24.0 if it is not fixed before the
release cut.

### Success Criteria:

#### Automated Verification:

- [ ] `rg -n 'fn resolve'
      cli/launcher/src/launch/outbound/resolve/cache_root.rs` returns nothing,
      and the `use super::{…}` import at `:149` no longer names `resolve`. The
      previously-planned `rg -n 'cache_root::resolve'` sweep is retained only as
      a cross-crate check — it returns nothing *today*, because the module is
      only ever referenced unqualified from its own tests, so on its own it
      certifies nothing
- [ ] `mise run test:unit:cli` passes with the re-homed and replacement tests
- [ ] `mise run cli:check` exits 0 (clippy would flag the now-unused import if
      `:149` were missed, and the narrowed visibility would fail the build if
      any caller outside the resolver module remained)
- [ ] `mise run docs:check` exits 0
- [ ] `mise run check` exits 0

#### Manual Verification:

- [ ] The old-test → discharging-test mapping is recorded, including the
      create-if-missing assertion that needed a replacement test rather than a
      pre-existing discharger
- [ ] The rewritten module doc describes only the surface that survives, and its
      precedence matches `candidate` (override first)
- [ ] The rewritten `internals.md` paragraph reads correctly in context against
      the preceding bootstrap paragraph, which makes the same warm/cold
      distinction for the bootstrap
- [ ] The `CHANGELOG.md` entry and the `internals.md` paragraph now agree

---

## Phase 4: Correct the Stale `docs/internals.md` References

### Overview

Two shipped files point users at `docs/internals.md`, a path that has never
existed in this repo. Both are user-facing at runtime, so both are repointed.

A full search finds exactly two stale references. Two further hits name
`docs-site/src/content/docs/internals.md` in full and are correct as they stand:
`scripts/test-design.sh:9` and `tasks/README.md:433`, both repo-internal tooling
that legitimately addresses the source file.

**Why this rides along with the probe work.** It is not derived from 0189's
acceptance criteria, and it touches `skills/` and `hooks/` rather than the
launcher. It is kept here because Phase 3 is already correcting staleness in the
same page for the same reason — 0169 moved behaviour and left the documentation
behind — so one reviewer holds the whole picture of what `internals.md` now
says, and both land in one docs CI run.

An earlier draft set a tripwire — "if this phase grows beyond the two pointers
it should be lifted out" — and then immediately crossed it by adding a guard.
The guard stays, because a repointing with nothing stopping it rotting again is
the failure this phase exists to fix, but the tripwire is restated honestly:
three changes is the ceiling, and anything further goes to its own item.

### Changes Required:

#### 1. The visualiser skill body

**File**: `skills/visualisation/visualise/SKILL.md:162`
**Changes**: Repoint the
closing sentence of the terminal-invocation paragraph at the published URL.

```markdown
To run the visualiser from a terminal, link the accelerator CLI onto your
`$PATH` and invoke `accelerator visualiser [stop | status]`. The link is a
two-hop chain so it survives plugin upgrades — see [Terminal
Invocation](https://atomicinnovation.github.io/accelerator/internals/#terminal-invocation)
for the setup steps.
```

**Why not the site-relative form.** SKILL.md bodies are rendered verbatim into
docs-site pages (`tasks/shared/skill_pages.py:174`), and the four existing
cross-links to this anchor — `visualiser.md:96`,
`releases-and-compatibility.md:37`, `corpus.md:22`, `collaboration.md:17` — do
use `](internals.md#terminal-invocation)`. But those four files sit at
`docs-site/src/content/docs/`, siblings of `internals.md`. This body lands three
directories deeper, at
`docs-site/src/content/docs/reference/skills/visualisation/visualise.md`
(`tasks/shared/paths.py:13` sets `DOCS_GENERATED_DIR`; `skill_pages.py:125-128`
writes `<generated_dir>/<category>/<name>.md`), so
`astro-rehype-relative-markdown-links` — which resolves against the containing
file — would produce `reference/skills/visualisation/internals.md`, which does
not exist. `../../../internals.md` would resolve on the site but is meaningless
in the rendered prompt, which is the other half of this string's audience.

Nothing would catch the mistake: `starlightLinksValidator({
errorOnRelativeLinks: false })` (`docs-site/astro.config.mjs:74`) skips relative
links. That is demonstrable today —
`skills/integrations/jira/init-jira/SKILL.md:26` already carries an unresolvable
`](../../config/configure/SKILL.md#work)` and `mise run docs:check` passes.

The absolute URL works in both places: the plugin user reading the prompt has no
checkout, and the generated page's link resolves. It does hardcode the hosting
decision `docs-site/astro.config.mjs:9-12` externalises via
`DOCS_SITE`/`DOCS_BASE` — accepted as the same conscious inheritance
`README.md:34-61` already makes, including `README.md:54` for this exact page,
and recorded in item 3 so a domain move has one place to look.

#### 2. The launcher link-refresh hook

**File**: `hooks/launcher-link-refresh.sh:42`
**Changes**: Repoint the URL
inside the `CLAUDE_PLUGIN_DATA` fallback message, leaving the leading diagnostic
text untouched.

```sh
  *) finish "CLAUDE_PLUGIN_DATA unavailable; no terminal link refreshed. See https://atomicinnovation.github.io/accelerator/internals/#terminal-invocation" ;;
```

Both pointers therefore adopt the same published URL, for the same reason: each
reaches a plugin *user* at runtime — one as a SKILL.md body rendered into a
prompt, the other as a hook message printed into a session — and someone running
an installed plugin has no checkout. It matches the convention `README.md:34-61`
already uses, including `README.md:54` for this exact page. The duplication of
the `DOCS_SITE`/`DOCS_BASE` hosting decision is a conscious inheritance from
README rather than a new one; item 3's guard defaults from the same two
variables so a fork can check its own build.

The anchor is valid as written: `## Terminal Invocation` sits at
`docs-site/src/content/docs/internals.md:145`, and Starlight derives
`#terminal-invocation` from it unchanged. Nothing in CI guards it, so item 3
adds a cheap assertion.

#### 3. Guard the anchor

**File**: `scripts/test-docs-anchors.sh` (new, executable)
**Changes**: A small
suite asserting that `internals.md` still carries `## Terminal Invocation`, and
that the two repointed files still hold the published URL for it.

Not added to `scripts/test-design.sh`: that suite is scoped end to end to the
design-skills domain, and a developer asking what stops the terminal-invocation
anchor rotting would not search it — the same discoverability failure that let
`docs/internals.md` survive in two shipped files.

The suite, verbatim — it must follow the directory's fixed shape, and the
closing `test_summary` is load-bearing, not decorative: the `assert_*` helpers
only increment a counter, and `test_summary` (`scripts/test-helpers.sh:371-381`)
is what turns a non-zero `FAIL` into a non-zero exit. A suite written without it
exits 0 with failing assertions, which is exactly the silently-vacuous guard
this one exists to prevent.

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

DOCS_INTERNALS="$PLUGIN_ROOT/docs-site/src/content/docs/internals.md"
# docs-site/astro.config.mjs owns the hosting decision; the two shipped pointers
# and this constant are copies of it, so assert the owner still agrees.
ASTRO_CONFIG="$PLUGIN_ROOT/docs-site/astro.config.mjs"
ANCHOR_PAGE="https://atomicinnovation.github.io/accelerator/internals/"
ANCHOR_URL="${ANCHOR_PAGE}#terminal-invocation"

echo "=== terminal-invocation anchor ==="
assert_contains "internals.md keeps the heading the shipped pointers target" \
  "$(cat "$DOCS_INTERNALS")" "## Terminal Invocation"

echo "=== shipped pointers ==="
for f in \
  "$PLUGIN_ROOT/skills/visualisation/visualise/SKILL.md" \
  "$PLUGIN_ROOT/hooks/launcher-link-refresh.sh"; do
  assert_contains "$(basename "$f") points at the published anchor" \
    "$(cat "$f")" "$ANCHOR_URL"
done

echo "=== hosting decision still matches ==="
assert_contains "astro.config.mjs still defaults to the copied origin" \
  "$(cat "$ASTRO_CONFIG")" "https://atomicinnovation.github.io"
assert_contains "astro.config.mjs still defaults to the copied base" \
  "$(cat "$ASTRO_CONFIG")" "/accelerator"

test_summary
```

Note the helper's argument order: `assert_contains <test name> <haystack>
<needle>` (`scripts/test-helpers.sh:33-34`), not haystack-first. Every call site
in the directory follows that order — e.g. `scripts/test-design.sh:14-15`.

Shape and helpers follow `scripts/test-hash-common.sh` and
`scripts/test-design.sh`. It is auto-discovered by `run_shell_suites(context,
"scripts", …)` (`tasks/test/integration.py:344-351`) with no wiring, and per the
executable-bit invariant needs `chmod +x`.

**Guard it by name, not by count.** Add `scripts/test-docs-anchors.sh` to
`_REQUIRED_CONFIG_SUITES` (`tasks/test/integration.py:63`), which is the
fail-closed mechanism the file already provides. The `_EXPECTED_CONFIG_SUITES`
floor is a weaker check and is currently *behind*: `scripts/` holds 16
discoverable executable suites against a floor of 15, so bumping it to 16 would
still pass with this suite's exec bit dropped — the exact failure the floor
exists to prevent. Set the floor to the real post-change count (17) and extend
the running rationale comment at `:31-41`, which annotates every prior movement,
with the reason for this one.

Asserting the URL as well as the heading is the part that would have caught the
site-relative form rejected in item 1: a heading guard alone says nothing about
whether the pointers still resolve. `ANCHOR_URL` is defined once and both files
checked against it, so the guard proves the copies agree rather than adding a
third independent one — and the final pair of assertions ties that constant back
to `astro.config.mjs`, which owns the hosting decision, so a domain move breaks
the guard loudly instead of leaving it green against a stale literal.

The URL is written literally rather than composed from
`${DOCS_SITE}`/`${DOCS_BASE}`. Those are consumed by `npm run build` inside
`docs-site/` and are not exported into the `test:integration:config` lane, so
defaulting from them would look fork-aware while always reading the upstream
values; `DOCS_SITE` also already names the `docs-site/` *directory* in
`tasks/shared/paths.py:12`, so borrowing the name invites a confusing collision.

### Success Criteria:

#### Automated Verification:

- [ ] `rg -n 'docs/internals\.md' -g '!meta/**' -g '!workspaces/**' .` returns
      only full-path hits — after this phase, three: `scripts/test-design.sh:9`,
      `tasks/README.md:433` and the new `scripts/test-docs-anchors.sh`, whose
      `DOCS_INTERNALS` assignment contains the same substring. No bare
      `docs/internals.md` pointer remains
- [ ] Shell lint and format pass: `mise run scripts:check`
- [ ] The hook tests pass: `mise run test:integration:hooks`. They assert on
      `"CLAUDE_PLUGIN_DATA unavailable"` alone
      (`tests/integration/hooks/test_launcher_link_refresh.py:328`, `:342`), not
      on the docs pointer, so the message change should not touch them — but
      neither `scripts:check` nor `mise run check` runs any test task
      (`mise.toml:575-577`), so the suite has to be named explicitly
- [ ] The new anchor suite passes: `mise run test:integration:config` (which
      discovers executable `test-*.sh` under `scripts/`); `bash
      scripts/test-docs-anchors.sh` for the inner loop. `scripts:check` is
      format + lint only and does **not** run it
- [ ] `scripts/test-docs-anchors.sh` is listed in `_REQUIRED_CONFIG_SUITES`, and
      `_EXPECTED_CONFIG_SUITES` is set to the real post-change discovered count
      (17, not 16 — the floor is currently one behind at 15) with its rationale
      comment extended
- [ ] The suite fails closed: with one assertion deliberately broken it exits
      non-zero, confirming `test_summary` is present and reached
- [ ] The suite is executable (`chmod +x`) **and the mode is committed**, per
      the exec-bit invariant — the guard reads the working-copy mode, so a
      local-only chmod passes here and fails CI on a fresh checkout
- [ ] `mise run docs:check` exits 0. This covers the source-path removal only —
      it does **not** validate either repointed URL, since
      `starlightLinksValidator` runs with `errorOnRelativeLinks: false` and does
      not resolve absolute off-site links either. The new anchor suite is what
      guards them
- [ ] `mise run check` exits 0

#### Manual Verification:

- [ ] The `#terminal-invocation` anchor resolves on the built site, reached from
      the generated `reference/skills/visualisation/visualise` page
- [ ] The rewritten SKILL.md sentence reads naturally; the URL line exceeds 80
      columns unavoidably, as `README.md` accepts for its own links

---

## Testing Strategy

### Unit Tests:

- `each_verify_writable_call_counts_one_attempt` — two calls, delta 2
- `a_probe_against_an_uncreatable_directory_still_counts` — the case that
  discriminates invocation-counting from write-counting, and the reason the
  counter is not the existing `SEQUENCE`. Arranged beneath a regular file so
  `create_dir_all` fails with `ENOTDIR` for every user including root
- `verify_writable_creates_a_missing_directory` — the create-if-needed assertion
  that `a_writable_plugin_root_is_used` carried and that no other test
  discharges
- Re-homed `candidate` tests: unset plugin root with no override, override used
  verbatim without touching the filesystem

### Integration Tests:

**Eight** delta assertions in `cli/launcher/tests/resolution.rs` — six new tests
and two retrofitted onto existing ones.

Six new, one per branch of `FetchVerifyCacheResolver::resolve` that can reach
the probe plus the memoisation guard: cold miss (1), warm hit (0), refetch after
byte poisoning succeeding (1), refetch after byte poisoning failing into
`CorruptCacheAndRefetchFailed` (1), refetch after a benign sidecar-I/O error (1)
— the third `fetch_verify_store` call site, at `mod.rs:222-228`, which byte
poisoning can never reach — and two cold misses in one process (1 each).

Two retrofitted, and they are the load-bearing pair rather than an afterthought:
`a_signature_read_io_error_propagates_the_refetch_error_verbatim` (1) is what
*pins* the `:222-228` arm by error shape, since the new sidecar test's
`Ok`/delta-1 outcome is shared with a cold miss; and
`an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` (1) is the only
assertion covering a **failing** probe, the shape Mutation C exercises.

Every one is a delta around a single call. The delta form is required because a
single test process performs several resolutions, so an absolute read stops
meaning anything once a second resolution enters the process; and because the
work item requires it permanently. Under nextest's one-process-per-test default
the two coincide.

**No runner precondition.** The counter is thread-local, so the eight assertions
are sound under `cargo test` (parallel threads in one process) and `cargo
nextest` (one process per test) alike. Nothing another test does can perturb a
delta.

**Tool prerequisite:** all six open with `skip_if_no_minisign!`, which returns
`Ok(())` and is reported as PASS. Without `minisign` on `PATH` the guarantee is
unenforced while the suite reports green — which is why the mutation exercise
requires positive proof of execution rather than merely observing red.

**Environment prerequisite:** `verify_writable` proves exec-capability by
writing a `#!/bin/sh` script into the target directory and running it, so every
counting test requires an exec-capable `TMPDIR`. On a host that mounts `/tmp`
`noexec` these fail with a `CacheRootUnavailable`-shaped error inside a test
named after probe counting. The coupling is pre-existing; it is recorded here so
the failure is diagnosable, and the remedy is one line: `tempfile` honours
`TMPDIR`, so point it at an exec-capable path.

### Manual Testing Steps:

1. Write the two unit tests, hoist `SEQUENCE`, confirm
   `a_probe_against_an_uncreatable_directory_still_counts` fails with a delta of
   0 while `each_verify_writable_call_counts_one_attempt` passes; record both,
   revert the hoist. After the counter lands, delete its increment once to force
   the second test red, then restore.
2. Author each assertion under the mutation that reddens it (see the pairing in
   Phase 2 item 2), then apply each of Phase 2's four mutations in turn, run all
   eight delta assertions against each, and record every cell of the A/B/C/D
   table including the greens and the two collateral reds. Confirm no `skipping:
   minisign not on PATH` line appears in any recording. Revert all four.
3. Confirm `a_probe_against_an_uncreatable_directory_still_counts` passes as
   root, verifying the fixture is privilege-independent. Scope the run to that
   test by name: `verify_writable_rejects_a_read_only_directory` and
   `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` cannot pass as
   root, because the superuser bypasses the `0o555` mode they depend on.
4. Confirm by search that `verify_writable` has exactly one production call
   site, is no longer reachable outside `crate::launch::outbound::resolve`, and
   that `cache_root::resolve` is gone from the crate.
5. Read the rewritten module doc against the surviving surface, and the
   rewritten `internals.md` paragraph against the bootstrap paragraph above it
   and against the corrected `CHANGELOG.md` entry.
6. Confirm the `#terminal-invocation` anchor resolves on the built docs site
   from the generated visualise reference page.

## Performance Considerations

The counter adds one thread-local `Cell` increment per `verify_writable` call —
no atomic, no memory ordering — against a call that already writes a file,
chmods it, forks a process and unlinks the file. 0186's re-derived attribution
(`meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md:569-589`,
measured 2026-08-03) puts the full write + chmod + exec + rm cycle at **107.15
ms in `/tmp`**, against 3.72 ms to re-exec a pre-existing file and a 1.41 ms
bare fork+exec floor — so ~103 ms of it is macOS's first-exec check. `/tmp` is
the right figure here because the tests probe under `TMPDIR`; the 131.97 ms row
in the same table is the identical probe run in the repo's `bin/`, a ~23%
location-dependent swing, not a different operation. (Do not cite the older
107.9/10.6 pair from that item's Context table — 0186's own plan distrusted its
unrecorded methodology and mandated the re-derivation above.) The increment is
immeasurable against any of these.

Two properties keep the access a bare TLS load/store, and both are load-bearing:
the `const { Cell::new(0) }` initialiser suppresses the lazy-initialisation
branch `thread_local!` otherwise emits on every `with`, and `Cell<u64>` has no
`Drop`, so no TLS destructor is registered. A non-`const` initialiser or a
payload with a destructor would silently reintroduce both.

**Recurring cost, paid every CI run.** Eight new real probes — six across the
integration tests (cold miss 1, warm hit 0, both byte-poisoning refetches 1
each, sidecar refetch 1, two cold misses 2) and two in
`each_verify_writable_call_counts_one_attempt` — less the one Phase 3 removes by
re-homing the override test onto `candidate`, which performs no filesystem
access. Net **+7**, ≈0.75 s of probe time *summed* on darwin — and the six new
integration tests each also build a `happy_harness` (three `minisign` spawns,
fixture read and hash, mock server), so the combined recurring addition is of
order **one second summed**, dominated by probes. Both figures are extrapolated
from 0186's single developer host; no macOS CI-runner measurement exists, and
0186 recorded a 23% swing in the same probe on the same host between
directories, so treat them as order-of-magnitude. Note that
`a_probe_against_an_uncreatable_directory_still_counts` costs microseconds, not
108 ms: `create_dir_all` fails before anything is written, which is the whole
point of the test. Wall-clock is lower again under nextest's
one-process-per-test parallelism, and near-zero on linux — the ~108 ms is almost
entirely macOS's first-exec check on a freshly written file, which has no Linux
equivalent.

Probes are not the only added cost. Each of the six new integration tests calls
`happy_harness`, which spawns `minisign` three times (one `-G`, two `-S`), reads
and hashes the fixture and starts a mock server — roughly 30–60 ms per test
before any probe. That is inherent to the existing harness, not introduced here.
One avoidable part is not: `seed_cache` as drafted recomputes
`sha256_hex(&self.fixture_bytes)` although `happy_harness` already computes it
at `resolution.rs:165` and drops it. Retain it as `sha: String` on `Harness`
alongside `asset_sig` and have `seed_cache` reuse it — the same
drop-and-recompute defect this plan fixes for the signature, and it should not
be reintroduced for the digest.

**One-off cost, paid at implementation time.** The evidence the plan mandates is
substantial: four authoring passes (one per mutation, each an apply/edit/revert
cycle), then a baseline plus five mutation sweeps over the eight delta
assertions (A–D plus the degraded-seed run), the `SEQUENCE`-hoist run, the
increment-deleted run, the two Phase 3 forced-red runs and a root-privileged
run. Rebuilds of the launcher lib and every integration-test binary dominate the
probe time by a wide margin — and the four phase-closing `mise run check` passes
(plus `docs:check` in Phases 3 and 4, and the bare `mise run` the repo defines
as done) dominate those in turn. Probe time is a rounding error against the
compile time this evidence costs.

Nothing is added to the warm dispatch path, which does not call
`verify_writable` at all: `resolve` reaches it only through
`fetch_verify_store`, which a cache hit never enters.
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` measures that
path; this plan does not change it.

## Migration Notes

`cache_root::resolve` is `pub` in a binary crate whose library target exists to
serve its own integration tests. Nothing outside `cli/launcher` depends on it,
so removal is not a breaking change for any consumer.

Narrowing `verify_writable` to `pub(super)` is likewise not a consumer-visible
change, for the same reason. Within the crate it is a deliberate constraint:
`main.rs`, a separate crate, can no longer probe — which is the point.

`probe_attempts` grows the public surface by one function. It is production code
by compilation and test-support by intent, which is the cost the work item
already weighed and accepted when it named a public accessor as the default. Its
doc comment says so; no `#[doc(hidden)]` is applied, since the attribute has no
precedent in the `cli/` workspace and nothing here runs rustdoc, so it would be
decorative. The launcher's public API is not a distribution surface — the
shipped artifact is a binary.

The net public surface therefore shrinks by one: two `pub fn`s removed from it
(`cache_root::resolve` deleted, `verify_writable` narrowed to `pub(super)`)
against one added (`probe_attempts`).

## Validation Results

### Pick-up premise confirmation (recorded 2026-08-11, before work began)

Both premises the scope rests on were re-confirmed against revision
`9fb90f8a26d91d640cf0f6ab8b272b6039d7bdbd`.

**Premise A — `cache_root::resolve` has no production caller.** Confirmed. Its
only call sites are its own four unit tests plus its own definition.

**Premise B — no branch or loop in `FetchVerifyCacheResolver::resolve` reaches
`fetch_verify_store` more than once per call.** Confirmed by reading
`mod.rs:180-233`. Three syntactic call sites: `:211` and `:227` are arms of one
`match self.reverify(&cached)`, so at most one runs; all three arms of that
match are `return` expressions, so control cannot fall through to the tail call
at `:231`; `:231` therefore runs only on the `cache::find` → `None` path. No
loop, no recursion, no retry in the body.

Neither has changed. No re-scope needed.

### Crate search for probe call sites

```
$ rg -n 'verify_writable|cache_root::resolve|cache_root::candidate' cli/
cli/launcher/src/launch/outbound/resolve/cache_root.rs:41  (doc link)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:75  (inside resolve)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:88  (definition)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:149 (test import)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:231 (test)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:234 (test)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:239 (test)
cli/launcher/src/launch/outbound/resolve/cache_root.rs:247 (test)
cli/launcher/src/launch/outbound/resolve/mod.rs:141        (production)
cli/launcher/src/main.rs:65                                (candidate)
cli/launcher/tests/resolution.rs:589                       (assertion message)
```

One production call site for `verify_writable`: `mod.rs:141`. To be re-run and
re-recorded after Phase 3, when `cache_root.rs:75` and `:149` disappear, using a
pattern that matches the unqualified call form — the module is only ever
referenced unqualified from its own tests, so `rg 'cache_root::resolve'` returns
nothing even before the deletion and certifies nothing on its own.

### `SEQUENCE` discrimination

Recorded 2026-08-12 on darwin-arm64. Command throughout:

```
$ cargo nextest run --manifest-path cli/Cargo.toml -p accelerator \
    --all-features -E 'binary(accelerator) and test(cache_root)' --no-fail-fast
```

**Before item 2 — both tests fail to compile**, confirming `probe_attempts` did
not previously exist:

```
error[E0432]: unresolved import `super::probe_attempts`
   --> launcher/src/launch/outbound/resolve/cache_root.rs:150:20
    |
150 |         candidate, probe_attempts, resolve, verify_writable, CacheRootConfig,
    |                    ^^^^^^^^^^^^^^ no `probe_attempts` in `launch::outbound::resolve::cache_root`
```

**Against the hoisted `SEQUENCE` accessor.** `SEQUENCE` lifted to file scope
with `pub fn probe_attempts() -> u64 { SEQUENCE.load(Ordering::Relaxed) }`:

```
thread '…::a_probe_against_an_uncreatable_directory_still_counts' panicked at
launcher/src/launch/outbound/resolve/cache_root.rs:291:9:
assertion `left == right` failed
  left: 0
 right: 1

     Summary [   0.843s] 9 tests run: 8 passed, 1 failed, 70 skipped
        FAIL [   0.015s] (2/9) … a_probe_against_an_uncreatable_directory_still_counts
        PASS [   0.842s] (9/9) … each_verify_writable_call_counts_one_attempt
```

The discriminating outcome: a `verify_writable` call whose `create_dir_all`
fails increments `SEQUENCE` **zero** times, because `fetch_add` sits after the
early return. `each_verify_writable_call_counts_one_attempt` passes against both
counters, as predicted, so it never forced red here.

**With the thread-local counter in place** — all nine cache_root tests pass:

```
     Summary [   3.872s] 9 tests run: 9 passed, 70 skipped
```

**With the increment statement temporarily deleted**, both counting tests go red
at delta 0, confirming neither assertion is mis-wired:

```
thread '…::each_verify_writable_call_counts_one_attempt' panicked at
launcher/src/launch/outbound/resolve/cache_root.rs:283:9:
assertion `left == right` failed
  left: 0
 right: 2

     Summary [   1.345s] 9 tests run: 7 passed, 2 failed, 70 skipped
        FAIL … a_probe_against_an_uncreatable_directory_still_counts
        FAIL … each_verify_writable_call_counts_one_attempt
```

**Both throwaways reverted.** `SEQUENCE` is back inside
`probe_writable_and_executable`'s body, declared and positioned exactly as
before, and the increment statement is restored. The final state carries only
the thread-local `PROBE_ATTEMPTS`, its accessor, and the one added statement.

### Mutation exercise

Recorded 2026-08-12 on darwin-arm64. Command for every sweep, run over the whole
integration binary so greens are observed and not inferred:

```
$ cargo nextest run --manifest-path cli/Cargo.toml -p accelerator \
    --all-features -E 'binary(resolution)' --no-fail-fast --no-capture
```

Each assertion was authored under the mutation that reddens it, per the pairing
in Phase 2 item 2 — the assertion was written while the mutation was in force,
observed red, and only then made green by reverting.

**Observed table.** ✗ = red under that mutation; every other cell was observed
PASS in the same run.

| Test | Baseline | A | B | C | D |
| --- | --- | --- | --- | --- | --- |
| `a_cold_miss_probes_the_cache_root_exactly_once` | PASS | ✗ | ✗ | PASS | PASS |
| `a_warm_hit_never_probes_the_cache_root` | PASS | ✗ | ✗ | PASS | PASS |
| `a_successful_refetch_probes_the_cache_root_exactly_once` | PASS | ✗ | ✗ | PASS | PASS |
| `a_failed_refetch_probes_the_cache_root_exactly_once` | PASS | ✗ | ✗ | PASS | PASS |
| `a_refetch_after_a_benign_cache_io_error_probes_exactly_once` | PASS | ✗ | ✗ | PASS | PASS |
| `each_of_two_cold_misses_probes_the_cache_root_once` | PASS | ✗ (1st bracket) | ✗ | PASS | ✗ (2nd bracket) |
| `a_signature_read_io_error_propagates_the_refetch_error_verbatim` | PASS | ✗ | ✗ | PASS | ✗ |
| `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss` | PASS | PASS | PASS | ✗ | PASS |

Every predicted cell was observed as predicted. Per-mutation totals over the
25-test binary: A — 6 failed, 18 passed (the warm-hit test did not yet exist
when A was in force; it was authored under B). B — 8 failed, 17 passed. C — 1
failed, 24 passed. D — 2 failed, 23 passed. Baseline — 25 passed, 0 skipped.

`each_of_two_cold_misses_…` fails on the **first** bracket under A and on the
**second** under D, which is what distinguishes D's reach from A's:

```
assertion `left == right` failed: first cold miss: expected 1 probe attempt(s)   [A]
assertion `left == right` failed: second cold miss: expected 1 probe attempt(s)  [D]
```

**Collateral results, recorded rather than treated as noise.** Mutation A
reddens `a_signature_read_io_error_propagates_the_refetch_error_verbatim` — it
reaches `fetch_verify_store` and therefore doubles, so the claim that A perturbs
no existing test is wrong. Mutation B reddens the pre-existing
`resolve_succeeds_from_a_read_only_cache_root_on_a_hit`: with the cache chmodded
`0o555` the injected probe cannot write and resolution errors out. That one is
corroborating — the warm-path property already has a behavioural guard
independent of the new counter.

`an_unwritable_cache_root_…` stays green under B **by short-circuit, not by
preservation**: the injected probe fails before `cache::find` is reached, so the
count is 1 for a different reason than under baseline.

**Fixture-postcondition mutations.** The plan claimed one degraded-seed run
discharges all three fixture postconditions. It does not — the degraded seed
only reaches `seed_cache`'s `is_some()`. Three separate demonstrations were
therefore run:

- **Degraded seed** (`seed_cache` storing under `"0.0.0-degraded-seed"`): the
  four seed consumers — `a_warm_hit_never_probes_the_cache_root`, both
  byte-poisoning refetches and the sidecar refetch — fail on `a seeded entry
  must be findable, or the warm-path tests silently degrade into cold-miss
  duplicates`, not on their deltas. That is the silent-degradation mode the
  postcondition exists to catch.
- **`clear_cache` not clearing** (`remove_dir_all` removed):
  `each_of_two_cold_misses_probes_the_cache_root_once` fails on `the cache must
  be empty, or the next resolution is a hit`.
- **Retrofit findability** (cached binary unlinked before the assertion):
  `a_signature_read_io_error_propagates_the_refetch_error_verbatim` fails on
  `assertion failed: cache::find(&harness.cache, BINARY, VERSION).is_some()`,
  confirming the assertion that excludes the cold-miss tail call is load-bearing
  rather than belt-and-braces.

**No recording contains `skipping: minisign not on PATH`.** `minisign` 0.12 is
on `PATH` via mise; every sweep was run with `--no-capture` and grepped for that
string, which never appeared.

**All mutations reverted.** `jj diff` over
`cli/launcher/src/launch/outbound/resolve/mod.rs` is empty, and the baseline
sweep above is green at 25/25.

### Root-runner confirmation

Recorded 2026-08-12. **What was run is narrower than the criterion asked for,
and the deviation is stated rather than glossed.** The criterion names the test
binary run under root on linux. The launcher crate does not build inside a
container from this jj workspace — there is no `.git` here for `vergen-gitcl` —
and building it would have cost a full linux dependency compile to exercise a
five-line temp-dir arrangement. Instead the arrangement itself was reproduced
verbatim as uid 0 on linux (`rust:1.90-slim`, `docker run -u 0`):

```
uid=0
create_dir_all -> Err(Os { code: 20, kind: NotADirectory, message: "Not a directory" })
target exists -> false
OK: fixture is privilege-independent
```

That establishes exactly the property the criterion exists to check — that
`create_dir_all` beneath a regular file fails with `ENOTDIR` for the superuser
too, so the fixture is privilege-independent and leaves no residue outside the
temp dir. It does **not** establish that the assembled test binary passes as
root; nothing else in that test is privilege-sensitive, but that was not run.

No darwin `sudo` corroboration was taken: `sudo` on this host requires an
interactive password, and the plan already records that a darwin root run proves
less than a linux one.

### Old-test → discharging-test mapping

_Pending Phase 3._ Four rows, one per assertion the deleted
`cache_root::resolve` tests made, each naming its discharging test and whether
that test is re-homed, newly written, or pre-existing.

### Documentation corrections

_Pending Phases 3 and 4._ Slots: the `internals.md` paragraph, the
`CHANGELOG.md` entry, the `cache_root` and `resolve/mod.rs` module docs, the two
repointed runtime pointers, and the new `scripts/test-docs-anchors.sh` suite
with its `_REQUIRED_CONFIG_SUITES` registration.

## References

- Original work item:
  `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
- Related research:
  `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
- `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` — the
  sibling plan against this work item, carrying the deferred latency
  measurement, the 0169 backfill and the Dependencies retraction
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — delivered the probe
  split; closed with its Phase 10 latency gate unmeasured
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 5's
  split, which moved the probe off the warm path and is the reason the property
  this plan pins already holds
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  the mutation-recording pattern this plan's Phase 2 follows
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:66-77` — the wrapper
  Phase 3 deletes
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:108-128` — the probe;
  `SEQUENCE` at `:109`, incremented `:114`, after the early return `:111-113`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:137-177` —
  `fetch_verify_store`, the sole production call site
- `cli/launcher/src/launch/outbound/resolve/mod.rs:180-233` — `resolve`, where
  the at-most-once property lives
- `cli/launcher/src/launch/outbound/resolve/cache.rs:81-116` — `cache::store`,
  the seeding primitive
- `cli/launcher/tests/resolution.rs:478-514` — the poisoning tests that already
  produce both refetch outcomes
- `docs-site/src/content/docs/internals.md:253-283` — "Offline, mirrored and
  read-only installs", stale at `:277-280`; `## Terminal Invocation` at `:145`
  is the anchor Phase 4 links to
- `skills/visualisation/visualise/SKILL.md:162` and
  `hooks/launcher-link-refresh.sh:42` — the two stale `docs/internals.md`
  pointers Phase 4 repoints
- `README.md:34-61` — the published-docs URL convention both repointed
  references adopt
