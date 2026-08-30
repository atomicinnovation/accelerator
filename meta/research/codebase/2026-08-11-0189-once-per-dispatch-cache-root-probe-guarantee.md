---
type: "codebase-research"
id: "2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"
title: "Research: Implementation surface for 0189's at-most-once cache-root probe guarantee"
date: "2026-08-11T15:44:00+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0189"
parent: "work-item:0189"
relates_to: ["codebase-research:2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path", "codebase-research:2026-07-03-0164-launcher-and-git-style-dispatch", "codebase-research:2026-08-05-0169-vcs-subdomain-and-hooks-migration"]
topic: "Implementation surface for 0189's at-most-once cache-root probe guarantee"
tags: ["research", "codebase", "cli", "launcher", "cache-root", "probe", "testability", "latency"]
revision: "537892b5ac52bc1d0dd911a7276c682f3ee6448c"
repository: "accelerator"
last_updated: "2026-08-11T15:44:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementation surface for 0189's at-most-once cache-root probe guarantee

**Date**: 2026-08-11 15:44 UTC
**Author**: Toby Clemson
**Git Commit**: `537892b5ac52bc1d0dd911a7276c682f3ee6448c`
**Branch**: workspace `visualisation-system`, working-copy change `pkspkmzvovuo`, no bookmark
**Repository**: accelerator

## Research Question

What does the codebase actually look like around work item 0189 ("At-Most-Once
Guarantee for the Launcher's Cache-Root Probe"), so that a plan can be written
against real code rather than the work item's description of it? Specifically:
are the two pick-up premises still true; where can a probe counter live; what
test isolation exists; does a failing-verification seam exist; what does
deleting `cache_root::resolve` touch; and what is actually required to take the
inherited warm-dispatch latency measurement?

## Summary

**Both pick-up premises hold.** `cache_root::resolve` has zero production
callers — its only four call sites are its own unit tests. And
`FetchVerifyCacheResolver::resolve` cannot reach `fetch_verify_store` twice: the
three call sites are one `match` arm each plus one tail expression, every arm is
a `return`, and there is no loop, retry or recursion in the body. The invariant
0189 wants to pin is structurally true today. Re-scoping is not needed.

**One finding changes the counting seam's design.** The work item's definitional
sentence — "One `verify_writable` call performs exactly one probe and increments
the `SEQUENCE` counter exactly once" — is **false against the current code**.
`SEQUENCE.fetch_add` sits at `cache_root.rs:114`, *after* the `create_dir_all`
early return at `cache_root.rs:111-113`. A `verify_writable` call against a
directory whose creation fails increments nothing. No listed acceptance
criterion counts on that path, so nothing is blocked, but the seam has to be
fixed (a one-line hoist, or a second dedicated counter) or the definition
rewritten.

**Test isolation is already structural, for a different reason than the item
assumes.** The `cli/` workspace runs under `cargo nextest`, which spawns **one
OS process per test function**. There is no nextest config file anywhere in the
repo, so the default profile applies unmodified. A `static AtomicU64` therefore
starts at zero in every test, and the concurrent-first-use test cannot perturb a
counting test. The item's isolation precondition is satisfied by the runner, not
by ordering — though the delta convention is still right, because it is the only
form that also survives a bare `cargo test`.

**The fault-injection seam the item worries about already exists, in a different
shape.** There is no verifier port and no way to inject a failing verifier — but
`resolution.rs` already produces both refetch outcomes today by *poisoning the
cached bytes*, which is not a filesystem-permission mechanism and therefore
satisfies what the criteria actually ask for. Open Question 2 can be closed
without building anything.

**The latency half is release-blocked and methodologically contested.** It needs
a real minisign-signed `accelerator-vcs` release asset (no dev override, by
construction). Beyond that: the shell baseline `B`'s own script,
`hooks/vcs-guard.sh`, was **deleted by 0169** and must be recovered from
history; 0169's criterion says 20 samples while 0186's proven harness uses 50
interleaved with two instrument floors; and no benchmark tooling is committed
anywhere in the repo — 0186's harness survives only as a heredoc inside a plan
document.

**One documentation surface is already wrong** and this item is the natural place
to fix it: `docs-site/src/content/docs/internals.md:277-280` still tells users
that sub-binary dispatch always probes the cache directory. 0169's Phase 5 made
that false.

## Detailed Findings

### 1. The two pick-up premises — both confirmed

**Premise A: `cache_root::resolve` has no production caller.** Confirmed by a
full grep of `cli/`. The complete call graph:

| Symbol | Production callers | Test callers |
| --- | --- | --- |
| `cache_root::candidate` | `cli/launcher/src/main.rs:65` | `cache_root.rs:218` |
| `cache_root::verify_writable` | `cli/launcher/src/launch/outbound/resolve/mod.rs:141` | `cache_root.rs:234`, `:247` |
| `cache_root::resolve` | **none** | `cache_root.rs:166`, `:183`, `:202`, `:260` |

`resolve` is `pub`, so `dead_code` never flagged it. No other crate in the `cli/`
workspace mentions `cache_root` at all, and no integration test imports it —
`resolution.rs` sets `ResolverConfig { cache_root: ... }` directly.

**Premise B: `fetch_verify_store` is reached at most once per resolution.**
Confirmed by reading `FetchVerifyCacheResolver::resolve`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:180-233`). Three syntactic
call sites, `mod.rs:211`, `:227`, `:231`:

- `:211` and `:227` are arms of a single `match self.reverify(&cached)`, so at
  most one runs.
- All three arms of that `match` (including the `Ok(())` arm at `:199`) are
  `return` expressions, so control can never fall through the `if let
  Some(cached)` block to the tail call at `:231`.
- `:231` therefore only runs on the `cache::find` → `None` path, which executed
  neither of the others.
- No loop, no recursion, no `?`-driven retry anywhere in the body. The only
  retry loop on this path is inside `Fetcher::get`
  (`resolve/fetcher.rs:116-125`, `MAX_ATTEMPTS = 3`), which re-issues HTTP
  requests and never re-enters `fetch_verify_store`.

Per-branch probe counts under today's code: cache miss = 1, warm hit = 0,
refetch after integrity failure = 1, refetch after cache I/O failure = 1,
non-UTF-8 command name = 0.

The command to record for the AC ("`verify_writable` has exactly one production
call site, confirmed by a recorded search of the crate"):

```
rg -n 'verify_writable|cache_root::resolve|cache_root::candidate' cli/
```

### 2. The counter does not count what the work item says it counts

`cli/launcher/src/launch/outbound/resolve/cache_root.rs:108-128`:

```rust
fn probe_writable_and_executable(dir: &Path) -> bool {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    if std::fs::create_dir_all(dir).is_err() {
        return false;                                  // <- no increment
    }
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    ...
}
```

`SEQUENCE` exists for **filename uniqueness**, not for counting — its doc
comment (`:104-107`) says so, and it was added by 0169's Phase 10 to fix a
PID-only probe-path collision between the two threads in
`two_concurrent_first_use_resolves_both_succeed`. Because the early return
precedes the `fetch_add`, `SEQUENCE` counts *probes that got as far as writing*,
not `verify_writable` invocations.

None of 0189's count criteria are affected — all of them run against a writable
cache root, where `create_dir_all` succeeds. But the item's own definition of
"probe count" is wrong as written, and a future test on the unwritable-root path
would silently observe zero. Two ways out:

- **Minimal (honours the item's stated default):** hoist the `fetch_add` above
  the `create_dir_all` guard. Uniqueness is unaffected — nothing depends on
  sequence numbers being contiguous — and count then equals invocation count
  exactly. One line.
- **Separates the two concepts:** leave `SEQUENCE` alone as the filename
  sequence, and add a private `static PROBES: AtomicU64` incremented as the
  first statement of `verify_writable` itself. Public surface growth is still
  exactly one accessor. This reads better against the repo's DDD line — "probe
  sequence for path uniqueness" and "probes performed" are two different domain
  concepts currently sharing one variable — at the cost of deviating from the
  work item's stated default, which names `SEQUENCE` explicitly.

Either way the accessor is a plain `pub fn` in `cache_root`. There is no
precedent for a `_for_test` suffix on genuinely public API here (the only
`*_for_test` naming in the workspace is behind `#[cfg(test)]`, in
`cli/visualiser/server/src/test_support.rs`), so a domain name reads better than
a test-flavoured one.

### 3. Where the seam can live, and what the workspace will accept

The integration tests are a separate crate, so `#[cfg(test)]` is unavailable and
the accessor must be `pub`. The module is already fully public —
`accelerator::launch::outbound::resolve::cache_root` — via four bare `pub mod`
declarations (`lib.rs:7`, `launch/mod.rs:7`, `launch/outbound/mod.rs:4`,
`resolve/mod.rs:5`), and five items are already reachable externally
(`CacheRootConfig` with public fields, `from_env`, `candidate`, `resolve`,
`verify_writable`). Adding a sixth is not a new kind of exposure.

**A cargo feature would work mechanically but contradicts written-down
decisions.** `--all-features` is passed unconditionally by both
`tasks/test/cli.py:11-14` and `tasks/lint/cli.py:7`, so a `test-support` feature
would compile and run, and release builds (`tasks/build.py:317`) pass no
features so it would not ship. But the workspace has twice refused this shape in
writing:

- `cli/vcs-test-support/src/lib.rs:4-6` — "A crate rather than a feature on
  `vcs-adapters`, so that crate's `[features]` stays at exactly `bash-parity`
  and CI's `--all-features` cannot turn a fixture feature on workspace-wide."
- `cli/vcs-adapters/Cargo.toml:27-30` — "A second `[[bin]]` rather than a `stub`
  feature: the crate must gain no `[features]` entry beyond `bash-parity`, and
  CI's `--all-features` would turn a fixture feature on workspace-wide."

`cli/launcher/Cargo.toml` has no `[features]` block at all, and `rg 'cfg\(.*feature'`
over the launcher returns nothing. A bare `pub fn` — which is what the work item
already permits — avoids the argument entirely.

Existing counting precedent in the workspace, none of which fits directly: a
per-path `AtomicUsize` inside the test-local `MockServer`
(`cli/launcher/tests/common/mod.rs:35-89`); an on-disk marker written by stub
binaries (`cli/vcs-test-support/src/stubs.rs:84-144`); `RefCell<Vec<_>>`
recording doubles on injected ports (`cli/migrate/tests/engine.rs:83-90`); and
an injected-closure counter read inside one call
(`cli/corpus-adapters/src/lock.rs:363-377`, the closest analogue, but a unit
test against a private `_with` seam). **No production atomic in `cli/` is read
by any test today, and no delta-around-a-call assertion exists anywhere.** This
would be the first.

### 4. Test isolation: nextest gives one process per test

`tasks/test/cli.py:11-38` runs:

```
cargo llvm-cov nextest --manifest-path cli/Cargo.toml --workspace \
  --exclude accelerator-visualiser --all-features --summary-only
```

(or plain `cargo nextest run` with `ACCELERATOR_COVERAGE=off`). There is **no
`nextest.toml` anywhere in the repo** — verified by glob and by grepping for
`test-threads|threads-required|test-group|serial_test` — so nextest 0.9.138's
default profile applies: one process per test function, `test-threads =
num-cpus` (processes, not threads), `retries = 0`.

Consequences for the plan:

- A `static AtomicU64` is **per-test state**, reinitialised for every test. The
  concurrent-first-use test (`resolution.rs:594-609`, two threads, count
  nondeterministic between 1 and 2) runs in its own process and cannot
  contaminate a counting test. So can `cache_root::resolve`'s four probing unit
  tests, which is worth noting: the item's stated ordering rationale ("until
  they are gone they can run concurrently with the counting tests in the same
  process") does not hold under nextest. The ordering is still sensible as TDD;
  the justification should be corrected rather than the ordering changed.
- The delta convention is still the right call, because it is the only form that
  survives a bare `cargo test` (shared process, N threads) and because the item
  requires it permanently. Under nextest, delta and absolute happen to coincide
  — expect a reviewer to ask why, and answer with the `cargo test` case.
- **Doctests are not run by nextest at all.** A `///` example asserting on the
  counter would silently never execute.
- `--no-capture` implicitly forces `test-threads = 1`; it serialises tests, it
  does not merge processes.

Single-test command for the inner loop:

```
cargo nextest run --manifest-path cli/Cargo.toml -p accelerator --all-features \
  -E 'binary(resolution) and test(<name>)'
```

Note `-p accelerator` — the launcher's package name is `accelerator`
(`cli/launcher/Cargo.toml:2`) — and that the suite skips cleanly when the
`minisign` CLI is absent.

### 5. The failing-verification seam already exists

There is **no verifier port**. Verification is free functions —
`verifier::verify_binary` called statically at `mod.rs:102` (warm-hit re-verify,
via `FetchVerifyCacheResolver::reverify` at `mod.rs:90-109`) and `mod.rs:160`
(post-fetch). The resolver's only injectable collaborators are `ResolverConfig`,
`TrustedKeys` and `Fetcher`. Building a verifier port is exactly the resolver
restructure the item puts out of scope.

It is not needed. `resolution.rs` already arranges both refetch outcomes by
corrupting cache *contents*, which is not "filesystem permissions on the cache
root" and so satisfies what the criteria ask for:

| Criterion | Existing arrangement | Branch |
| --- | --- | --- |
| Refetch succeeds | `a_poisoned_cache_entry_is_replaced_in_place_and_reexecs` (`resolution.rs:478-492`) — `fs::write(&path, b"poisoned")`, server alive | `ChecksumMismatch` → `mod.rs:211` |
| Refetch fails → `CorruptCacheAndRefetchFailed` | `a_poisoned_cache_entry_offline_is_a_distinct_diagnostic` (`resolution.rs:494-514`) — same poisoning, second resolver pointed at `http://127.0.0.1:1` | `ChecksumMismatch` → `mod.rs:211`, `map_err` wraps |

The poisoning works because the expected sha256 is parsed out of the cache
filename (`cache.rs:51-73`), not recomputed, so rewriting the bytes guarantees a
mismatch. Two further seams exist if a *non*-integrity refetch is ever wanted:
an invalid-UTF-8 signature sidecar (`resolution.rs:530`, reaches `mod.rs:227`
instead), and constructing the resolver with an unrelated `TrustedKeys` via
`Harness::keys()` (`resolution.rs:125-130`) — though the latter also breaks the
refetch's manifest check, so it can only produce the failure outcome.

The plan should say plainly that the criteria's phrase "a test-only failing
verifier" is discharged by content poisoning, since a literal reading implies a
port that will not be built.

### 6. The warm-hit criterion needs a new fixture helper

AC2 requires the cache "pre-populated by the fixture writing a verified binary
directly to disk (not by a prior resolution)" — precisely so the observed delta
of 0 is not confounded by the seeding resolve's own probe. **No such helper
exists**: every warm-cache test in the suite calls `harness.resolve()` first
(`resolution.rs:216, 463, 482, 498, 520, 553`).

The building block is public and already reached from this file's imports:
`cache::store(root, name, version, sha256, bytes, signature)`
(`cli/launcher/src/launch/outbound/resolve/cache.rs:81-116`). A
`Harness::seed_cache()` calling it with `sha256_hex(&self.fixture_bytes)` and a
freshly-signed asset signature gives a genuinely verifiable entry with zero
prior probes.

One wrinkle: `happy_harness` computes `asset_sig` locally (`resolution.rs:167`)
and drops it — the `Harness` struct (`resolution.rs:101-113`) does not keep it.
Either add a field or re-sign via the retained `minisign` and `trusted_secret`
fields.

For AC5 (two cold misses in one process, total 2), the second miss is arranged
by emptying the cache directory between resolutions; the same helper inverted.
Note the harness builds a **fresh resolver on every `resolve()` call**
(`resolution.rs:132-148`), so no resolver-held state carries across.

### 7. Deleting `cache_root::resolve` — the full blast radius

Deleted: the function and its doc comment (`cache_root.rs:66-77`), and `resolve`
from the test-module import at `cache_root.rs:149`. Nothing else in the crate,
no other crate, and **no documentation** references it.

The four bound unit tests and their discharging replacements:

| Existing test | Assertion | Discharged by |
| --- | --- | --- |
| `unset_plugin_root_with_no_override_is_a_named_error` (`:164`) | error names both `ACCELERATOR_PLUGIN_ROOT` and "no `ACCELERATOR_CACHE_DIR` override was given" | re-home onto `candidate` — the error originates at `cache_root.rs:56-62`, `resolve` only forwards it |
| `a_writable_plugin_root_is_used` (`:179`) | resolves to `plugin_root.join("bin")` | split: the path is already asserted by the existing `candidate_performs_no_filesystem_write_or_process_spawn` (`:210`), the writability by the existing `verify_writable_accepts_a_writable_directory` (`:230`) |
| `a_read_only_plugin_root_with_no_override_is_a_named_error` (`:191`) | read-only root errors | the existing `verify_writable_rejects_a_read_only_directory` (`:238`) — the item explicitly permits this |
| `an_override_is_honoured` (`:256`) | override used verbatim, no `bin` suffix appended | re-home onto `candidate` |

So only two tests are genuinely re-homed; two are already covered. Worth
recording that mapping honestly rather than writing two redundant tests to make
a four-row table.

Both read-only tests rely on the process not being root, and both use
`std::os::unix::fs::PermissionsExt` **without a `#[cfg(unix)]` gate**
(`:194`, `:241`) — the test module is already unix-only even though production
carries a `#[cfg(not(unix))]` arm (`:137-140`). Preserve that as-is rather than
"fixing" it.

### 8. The mutation criterion

Introduce a second `cache_root::verify_writable(&self.config.cache_root)?;` into
`fetch_verify_store` (`mod.rs:141`), run the counting tests, record command and
output, revert. Expected: cold-miss delta 2, both refetch deltas 2,
two-resolution total 4, warm hit unchanged at 0.

0186 set the precedent for how this is recorded, and its wording is worth
copying: mutation results are recorded per case, naming which test each mutation
reds, with an explicit note where a test stays green (a preservation guard,
"a property worth recording rather than a defect"). See
`meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`,
Phase 2 success criteria, and the corresponding Validation Results in 0186's
work item.

### 9. The latency measurement — four obstacles, only one of them the release

**(a) The release gate.** `G` must be measured through the real
bootstrap → launcher → sub-binary path with no `ACCELERATOR_VCS_BIN` override,
which needs a minisign-signed `accelerator-vcs` release asset. 0169's plan is
explicit: "Deferred, not merely unattempted … no dev workaround satisfies 'no
override' by construction."

**(b) `B`'s subject no longer exists.** The criterion defines `B` as the median
of 20 `hooks/vcs-guard.sh` invocations — and 0169's Phase 9 **deleted that
script**. The recorded `B = 35.1 ms` is a 2026-07-30 figure produced by a method
0186's closeout explicitly declared "not method-comparable" to its own medians.
`B` must be re-measured from a recovered copy; 0186's harness already contains
the trick for exactly this, `jj file show -r "$1" bin/accelerator > bin/.tmp-…`
(the `bin/.tmp-*` name matters — `.gitignore:48` covers it, so jj's
auto-snapshot does not pick up a resurrected trust-root binary).

**(c) The sample-count collision.** 0169's criterion says 20 runs per side;
0186's harness — the one that survived scrutiny — uses 50 interleaved samples
per variant from a single Python process using `perf_counter`, with order
alternation and two instrument floors (`/usr/bin/true` and a trivial bash
script). The reasoning is in the plan: a per-call `python3` clock read puts an
interpreter startup *inside* the measured interval, comparable to the quantity
being measured; batching either side of a working-copy swap aliases jj snapshot
drift onto the difference; a fixed within-pair order biases whichever variant is
always second. Take 0186's harness shape, keep 0169's `G ≤ 1.1 × B` gate, and
record the deviation the way 0186 recorded its criterion-9 deviation.

**(d) Nothing is committed.** No `benches/`, no criterion or divan, no
hyperfine (not pinned in `mise.toml`, never invoked), no mise task, no timing
code in `.github/`. 0186's harness exists only as a fenced heredoc at
`meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md:1073-1135`.
The closest reusable apparatus is `tests/integration/support/installation.py`,
which reproduces the whole trust chain locally — generated keypair, signed
launcher, real verify shim, stubbed downloader, `run_bootstrap` as a single
funnel — with no network and no dev override, and `cli/launcher/tests/resolution.rs`
for the in-process resolver path.

**A ratio pass alone will not be convincing.** 0186's real evidence was the
composition budget: measured terms summing to within ~10% of the median, against
a ~25% attribution threshold. Port that. Budget `G` as warm bootstrap (~29.92 ms
measured 2026-08-03) plus launcher exec (~2.4 ms) plus a cache-hit `reverify`
plus `accelerator-vcs` startup and classify (3.6–4.7 ms cold per-process, from
0188's figures). Against `1.1 × B ≈ 38.6 ms` that is tight, which is exactly why
0189 names 0191 (~2.5 ms, "essentially this story's whole shortfall") as a
co-requisite. Plan for the outcome 0186's hand-off anticipated: relax the
threshold, or accept the overrun with a stated rationale.

**Where the evidence goes.** 0189's AC says the implementation plan's Validation
Results. 0169's five `_pending_` slots (B, G, ratio, payload+fixture, host+OS)
live in the **work item**. Pick one and say so — leaving 0169's slots pending
while filling a plan section elsewhere reproduces the orphaning the review
flagged.

### 10. Enforcement and tooling constraints

- **Clippy**: `warnings = "deny"` plus pedantic + nursery
  (`cli/Cargo.toml:133-147`), invoked with `-D warnings` and `--all-targets
  --all-features`. `missing_docs` is *not* enabled, so an undocumented `pub fn`
  compiles; but `missing_errors_doc` / `missing_panics_doc` (pedantic) bite any
  `pub fn` returning `Result` or able to panic. A `pub fn -> u64` reading an
  atomic trips neither. `must_use_candidate` is explicitly allowed, so no
  `#[must_use]` is required.
- **cargo-pup** (`cli/pup.ron`): every rule is a module-import restriction; none
  touches `launch::outbound::resolve`. No rule constrains public API surface or
  visibility. The known gotcha — a grouped `use a::{b, c}` resolves to an empty
  module name and is rejected by an `allowed_only` list — applies only inside
  `launch::core` and `version::core`, not here. `pup:check` is in `check` and
  the bare default, but **not** in `cli:check`.
- **cargo-deny** and `Cargo.lock`: no new dependency is needed, so neither
  reacts. Worth stating explicitly, because `lint:cli:check` runs clippy
  `--locked` and a stale lock surfaces as a confusing drift error.
- **Verify-shim marker**: covers `cli/verify/**` plus the `minisign-verify` pin
  and two lock blocks only. A launcher-side change cannot trip it.
- **`mise run` (bare default) is the done bar.** Inner loop:
  `ACCELERATOR_COVERAGE=off mise run test:unit:cli`, then `mise run cli:check`,
  then `mise run check`.

### 11. The documentation surface is already stale

`docs-site/src/content/docs/internals.md:277-280` (note: **not** `docs/internals.md`
— that path does not exist, though 0189's References and several other work
items still cite it):

> That exemption stops at the bootstrap: running any subcommand that dispatches
> to a separate binary makes the launcher probe the same directory, and that
> probe writes — so a permanently read-only cache directory is only viable if
> you never use those subcommands.

0169's Phase 5 falsified this. A warm sub-binary dispatch now never probes —
pinned by `resolve_succeeds_from_a_read_only_cache_root_on_a_hit`. A permanently
read-only, already-populated cache directory *is* viable for warm dispatch. This
item is the natural place to correct it, and 0189 already lists the section in
its References.

Separately, two **shipped** files carry the stale `docs/internals.md` path —
`skills/visualisation/visualise/SKILL.md:162` and
`hooks/launcher-link-refresh.sh:42`. Out of 0189's scope; worth raising.

## Code References

- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:48-64` — `candidate`, selection only, no filesystem access
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:66-77` — `resolve`, the doomed wrapper and its doc comment
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:88-99` — `verify_writable`, the sole delegate to the probe
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:108-128` — `probe_writable_and_executable`; `SEQUENCE` declared `:109`, incremented `:114`, **after** the `create_dir_all` early return `:111-113`
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs:142-267` — the seven unit tests; the four bound to `resolve` at `:164`, `:179`, `:191`, `:256`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:137-177` — `fetch_verify_store`; `verify_writable` is the unconditional first statement at `:141`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:180-233` — `resolve`; the three `fetch_verify_store` call sites at `:211`, `:227`, `:231`, every arm a `return`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:90-109` — `reverify`, the warm-hit verification path
- `cli/launcher/src/launch/outbound/resolve/cache.rs:51-73` — `cache::find`; the expected sha comes from the filename, which is why byte poisoning works
- `cli/launcher/src/launch/outbound/resolve/cache.rs:81-116` — `cache::store`, the seeding primitive for the warm-hit fixture
- `cli/launcher/src/main.rs:61-67` — the composition root: override first, then `cache_root::candidate`
- `cli/launcher/tests/resolution.rs:101-187` — `Harness` and `happy_harness`
- `cli/launcher/tests/resolution.rs:478-514` — the two poisoning tests covering both refetch outcomes
- `cli/launcher/tests/resolution.rs:548-592` — the two already-satisfied criteria from 0169 Phase 5
- `cli/launcher/tests/resolution.rs:594-609` — `two_concurrent_first_use_resolves_both_succeed`, two threads in one process
- `cli/launcher/tests/common/mod.rs:35-89` — `MockServer` hit counters, the workspace's existing counting idiom
- `tasks/test/cli.py:11-38` — the nextest invocation, `--all-features`, llvm-cov by default
- `cli/Cargo.toml:133-147` — workspace lint levels
- `cli/pup.ron:25-39, 219-232` — the only launcher-scoped pup rules, both import restrictions
- `docs-site/src/content/docs/internals.md:253-283` — "Offline, mirrored and read-only installs", stale at `:277-280`

## Architecture Insights

- **The at-most-once property is already structural, and the item's job is to
  keep it that way.** Every collaborator in this resolver is a free function
  rather than a port — verifier, cache, cache-root probe — which is why there is
  nothing to count against and why the counter has to be a static. That design
  is also what makes the property fragile: nothing but reading the code stops a
  future refactor adding a second call site, and nothing but this item's test
  will notice.
- **The workspace has a settled position on test seams and it is not cargo
  features.** Twice, in manifest comments, the alternative was chosen and the
  reasoning written down: a separate `-test-support` crate, or a second
  `[[bin]]` under `tests/fixtures/`. The driver both times was that CI passes
  `--all-features` workspace-wide. A bare `pub fn` sidesteps this; a feature
  gate would restart the argument.
- **Isolation comes from the runner, not from test design.** One process per
  test under nextest is doing a lot of quiet work in this codebase — it is why
  `config_read.rs`'s counter is safe, why `auth.rs`'s env mutex is defensive
  rather than load-bearing, and why the item's ordering rationale is weaker than
  it reads. The one place it does not help is threads inside a single test,
  which is exactly the case `SEQUENCE` was created for.
- **Measurement discipline is inherited from 0186 and is not optional theatre.**
  Order alternation, two instrument floors, single-process clock reads, and a
  composition budget checked against a ≤25% unexplained residual — those are the
  reason 0186's numbers are believable and the reason its ratio gate was not
  vacuous. A `G ≤ 1.1 × B` pass without the composition check would prove very
  little, particularly given how tight the budget is.
- **Nothing about benchmarking is written down anywhere authoritative.** There
  is no ADR on measurement policy, on test seams, or on test-only public API.
  The gate lives only inside 0169's artefacts. If this item wants the method to
  outlive it, an ADR is the missing artefact — though that is a scope decision,
  not a requirement.

## Historical Context

- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — closed `done`, with
  its Phase 10 latency criterion unchecked and B, G, ratio, payload, fixture and
  host all recorded as `_pending_`. Its 2026-08-11 retraction confirms 0189 now
  carries only the regression guard plus this measurement.
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 5
  item 2 is the split that already landed (rename to `verify_writable`, `main.rs`
  moved onto `candidate`, probe hoisted to the top of `fetch_verify_store`);
  Phase 10 carries the deferred gate and the hand-off record that 0189's own
  amendment removal has since falsified.
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  the measurement harness (`:1073-1135`), the composition-budget discipline, and
  the mutation-recording pattern. The single most reusable prior artefact.
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the measured
  figures: before 125.35 ms, after 29.92 ms, ratio 0.239; probe cost 131.97 ms
  in the repo's `bin/` against a 3.72 ms re-exec; sha256 backend figures.
- `meta/reviews/work/0189-once-per-dispatch-cache-root-probe-guarantee-review-1.md`
  — verdict REVISE across two passes; all pass-2 findings were applied to 0189
  and **not re-reviewed**, so the ordering rationale, the isolation precondition
  and AC5's fixture precondition are live design decisions rather than settled
  inputs.
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` — the
  ~2.5 ms co-requisite, still `draft`.
- `meta/decisions/ADR-0054`, `ADR-0046` — the dispatch and distribution
  decisions any change here must stay inside.

## Related Research

- `meta/research/codebase/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`
  — the closest existing survey of the probe and the bootstrap/launcher split
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
  — the resolver's original design research
- `meta/research/codebase/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
  — dispatch-cost material feeding the latency gate

## Open Questions

- **Which counter shape?** Hoisting `SEQUENCE`'s `fetch_add` above the
  `create_dir_all` guard honours the work item's stated default in one line; a
  separate `PROBES` static incremented inside `verify_writable` separates two
  conflated concepts at the cost of a documented deviation. Both keep public
  surface growth to one accessor.
- **Where do the latency figures land** — 0169's five `_pending_` work-item
  slots, or 0189's plan Validation Results? The ACs say the plan; the orphaning
  the review flagged argues for closing 0169's slots too.
- **Should `internals.md:277-280` be corrected in this item or its own?** It is
  a two-sentence fix to a statement that is currently wrong, and 0189 already
  references the section. The stale `docs/internals.md` paths in
  `skills/visualisation/visualise/SKILL.md:162` and
  `hooks/launcher-link-refresh.sh:42` are clearly separate.
- **Is `G ≤ 1.1 × B` reachable at all?** The composition budget suggests not
  without 0191, and possibly not with it. The plan should decide in advance what
  a principled overrun record looks like, rather than discovering it at
  measurement time.
- **Does `B` get re-measured from a recovered `hooks/vcs-guard.sh`, or does the
  gate get redefined against something that still exists?** The criterion as
  written references a deleted file.
