---
type: plan
id: "2026-08-11-0189-warm-dispatch-latency-measurement"
title: "Warm-Dispatch Latency Measurement Implementation Plan"
date: "2026-08-11T19:43:42+00:00"
author: "Toby Clemson"
producer: create-plan
status: draft
work_item_id: "work-item:0189"
parent: "work-item:0189"
blocked_by: ["work-item:0204"]
derived_from:
  ["codebase-research:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"]
relates_to:
  ["plan:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee",
   "work-item:0169", "work-item:0186", "work-item:0188", "work-item:0191",
   "work-item:0199", "work-item:0204"]
tags: [cli, launcher, performance, bootstrap, measurement]
revision: "2bb98478e7f7a4d2cf1cfa9c18bb3d7541961451"
repository: "accelerator"
last_updated: "2026-08-12T11:40:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Warm-Dispatch Latency Measurement Implementation Plan

## Overview

Run the warm-dispatch latency measurement 0169's Phase 10 deferred — warm-call
latency `G` against shell baseline `B` on one darwin host in one session, gated
on `G ≤ 1.1 × B` — using the method **work item 0204 closes**, and discharge the
recording obligations it leaves across 0169 and 0189.

**This plan specifies a measurement, not a methodology.** Three attempts to
specify the method inside a plan failed review, each because the design was
authored ahead of the evidence that would settle it. Work item 0204 is a spike
that answers the decomposition route, the clock domain, the residual definition
and the statistical design against real measurements, and measures the gating
`reverify` term. This plan consumes those answers; it does not re-derive them.
Where a step below says "per 0204", the spike's recorded answer is the
specification and this plan deliberately does not restate it.

This is the second plan against work item 0189. The first,
`meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`,
delivers the at-most-once probe guarantee and its documentation corrections. The
two share no code, no test and no source file — they overlap only on 0189's own
work item, in disjoint passages. The sibling is `ready` and lands first; this
plan's edits to 0189 are anchored by quoted text rather than line numbers, so
the sibling's edits cannot shift them.

## Current State Analysis

**The measurement is no longer release-blocked.** 0189's Dependencies section
states the item cannot close before an epic-0136 release cut produces a signed
`accelerator-vcs` asset. That is stale. Both `v1.24.0-pre.36` and
`v1.24.0-pre.35` ship `accelerator-vcs-darwin-arm64` alongside its `.minisig`,
and `v1.24.0-pre.36`'s signed `manifest.json` carries a `vcs` entry with
`darwin-arm64`, `darwin-x64`, `linux-arm64` and `linux-x64` platform records at
`schema_version: 1`. The real bootstrap → launcher → sub-binary path is
measurable today with no dev override.

**The baseline's subject was deleted but recovers cleanly.** 0169's Phase 9
deleted `hooks/vcs-guard.sh` in `cf42441e2aad`. It recovers from
`cf42441e2aad-`, and its single dependency `scripts/vcs-common.sh` is still in
the tree and byte-identical to the revision the guard sourced (verified by an
empty `jj diff -r 'cf42441e2aad-..@' --summary scripts/vcs-common.sh`), so
recovery is one file.

That byte-identity is a live assumption, not a fixed fact: work item 0199,
`retire-vcs-common-sh-residue-and-launcher-link-refresh`, is scoped to decide
whether `classify_checkout` is deleted from `scripts/vcs-common.sh`. If 0199
lands first, the recovered guard's behaviour changes underneath the baseline.
Phase 1 records the sha256 of both files, and any re-measurement must compare
them: a changed `vcs-common.sh` invalidates comparison with the recorded `B`
rather than replacing it.

**The shell baseline's cost is dominated by subprocess spawns whose count
depends on the checkout.** The recovered guard sources `scripts/vcs-common.sh`,
whose `classify_checkout` (`:177`) spawns `jj workspace root`, two `git
rev-parse` forms, `realpath`, `jq` and repeated `command -v` probes — and which
of those run depends entirely on the checkout kind. (`command -v` is a bash
builtin, so it costs no spawn; it is listed because it still costs `PATH`
resolution.) 0188's **delivered** figures
(`meta/work/0188-library-backed-vcs-adapter.md:1084-1085`) put `jj log -r @ -T
commit_id` at 23.84 ms and `git rev-parse` at 5.34 ms on this host. The guard's
call is `jj workspace root`, not that command, so 23.84 ms is an **upper bound**
on the guard's `jj` spawn, not its cost. (Not 0188's plan-table rows of 7.05 ms
and 4.40 ms — those are prototype figures superseded by the delivered ones.)

**The digest backend varies by host by more than the gate margin.**
`bin/accelerator:272-278` selects `sha256sum` when present and otherwise falls
back to `shasum -a 256`, mirroring `scripts/hash-common.sh:18-21`. 0186
delivered a range of roughly 7 to 24 ms across this pair. This is a
**within-darwin host difference** — which of the two is on `PATH` — and 0186
explicitly declined to call it a platform fact; the 11.7 ms figure it originally
quoted was retired.

Because that spread exceeds the gate's whole margin, the backend is not merely
recorded: since the harness already pins `PATH`, `G` is measured under **both**
backends and the verdict is stated as conditional on which is resolved. A gate
decided by whether `/sbin/sha256sum` happens to exist on one laptop must not be
recorded in three documents as a platform-neutral result.

### Key Discoveries:

- `swallow_under_fail_safe` (`core.rs:219-224`) swallows only
  `kernel::Error::Failed`, and `handle_dispatch_error` (`main.rs:215-226`) then
  exits 0 without ever exec'ing the sub-binary. A degraded sample therefore
  skips both `reverify` and the sub-binary run and records a spuriously *low*
  latency. This is the failure the per-sample validity gate exists to catch.
- **The repo's dual-shape envelope mapping is in Rust, not Python.** The shell
  guard emits the legacy `{"decision":"block","reason":…}` shape; the Rust guard
  emits `hookSpecificOutput.permissionDecision` (`kernel/src/hooks.rs:29-31`).
  `cli/vcs-cli/tests/guard_decision_table.rs:99-141` maps
  `permissionDecisionReason` onto the label **`block`** and is the mapping to
  reuse. `hooks/test-fixtures/vcs-guard/generate_decision_table.py:160`
  `normalise()` is **shell-only** — its docstring says so, and it silently maps
  the Rust deny envelope to `("allow","")`, the same value an empty stdout from
  a swallowed fail-safe produces. It must not be used for this comparison.
- The two guards' reason strings are byte-identical
  (`cli/vcs-cli/src/guard.rs:19-20` against the golden `decision-table.json`),
  so pinning the expected reason text across variants is achievable once the
  decision label is mapped.
- `bin/.tmp-*` is gitignored (`.gitignore:56`), and the shell linter's tree walk
  honours `.gitignore` via `tasks/shared/sources.py`. That cuts both ways: a
  gitignored artefact is invisible to jj's auto-snapshot **and** to any `jj
  diff`/`jj status` cleanup gate. Cleanup is therefore asserted positively,
  never by absence of a VCS diff.
- The dev-launcher marker is plugin-root-relative
  (`dev_launcher_marker="${plugin_root}/.accelerator-dev-launcher"`,
  `bin/accelerator:225`), not root-absolute.

## Desired End State

`B` and `G` are measured on one darwin-arm64 host in one session under both
digest backends, with the dispersion, gate statistic and interval 0204
pre-registered. A composition budget accounts for `G` per 0204's decomposition,
with its residual computed as 0204 defines it. The verbatim harness, the stdin
envelope, the fixture, the baseline's provenance, the host/OS/chip, the plugin
version, the digest backends and the machine state are all recorded. 0169's
obligation is discharged in all four of its locations, and 0189's stale
Dependencies premise is replaced — not merely removed — by a closure guard
keyed on the measured outcome.

**"Discharged" is deliberate, not "ticked".** 0169 is already closed `done`, so
a measured overrun means a completed story shipped failing its own gate. Phase 3
branches on the outcome 0204's taxonomy selects and defines a procedure for each
branch, including the branches where no ratio exists.

Verified by: the Validation Results section below is complete, and every
throwaway artefact is positively asserted absent.

**The gate is verified on darwin-arm64 only**, and of the four shipped
platforms **darwin-x64 and linux-arm64 are exercised by no CI lane at all**.
That scope is inherited from 0169's definition rather than chosen here; a linux
measurement is a named hand-off.

**The direction of platform transfer is unknown and is recorded as such.** An
earlier draft asserted linux was the harder host on the grounds that `B` is
spawn-dominated and `G` is IO/hash-dominated. That does not survive 0186's own
breakdown: `G`'s bootstrap term is itself overwhelmingly spawn cost, and linux
ships coreutils `sha256sum` universally while darwin risks the slower `shasum`
— pushing the transfer the other way. Both variants are spawn-dominated with
differing spawn counts, so the direction is derived from the Phase 2 budget if
it can be derived at all, and the hand-off records it as an open question.

## What We're NOT Doing

- **No methodology.** SQ-1 to SQ-5 belong to work item 0204. This plan does not
  choose a decomposition route, a clock domain, a residual definition, a sample
  count or an outcome taxonomy.
- **No committed benchmark tooling** in this plan — but the harness is committed
  as the linux hand-off's first task, so the transcript is a one-generation cost
  rather than a recurring one. This is the epic's third hand-rolled warm-path
  harness.
- **No mutation of `keys/accelerator-release.pub`.** Signing a locally built
  launcher was considered and rejected: `bin/accelerator:165` reads that file
  and `cli/launcher/build.rs:31-32,:43` embeds it, so the bootstrap's
  verification key and the launcher's embedded key are one artefact; `mise run
  keys:generate` force-overwrites it; and the only assertions on it are that it
  parses and is not a placeholder. No route in this plan touches it.
- **No linux measurement in this plan.** Named as a hand-off.
- **No relaxation of the threshold after seeing the number**, and no sampling
  beyond what 0204 pre-registered.

## Implementation Approach

**Blocked on work item 0204.** Phase 1 cannot begin until 0204 records its
answers, because `n`, the gate statistic, the budget's term set and the outcome
taxonomy are all its outputs.

A pre-flight step, then three phases.

**Phases 1 and 2 are one session-scoped unit, not independently integratable.**
`B`/`G` comparability rests on a single session, and Phase 2's budget consumes
per-sample data only Phase 1's harness can emit — so Phase 1's harness carries
Phase 2's per-sample emission requirements, and Phase 2 is its analysis step.
Phase 3 is independently integratable and lands the record.

### Pre-flight

Runs once, before anything is created, and its output is a Validation Results
slot. The abort checklist compares against it, so without it three of the
checklist's items have no referent:

1. `sha256(keys/accelerator-release.pub)`, taken **from the committed
   revision** (`jj file show -r @ keys/accelerator-release.pub | shasum -a
   256`) rather than the working copy, so a key already substituted cannot be
   certified unchanged.
2. Digests of the cached launcher, its `.minisig` and the staged shim.
3. The byte count of `.accelerator-unverified.log`, and its full contents
   captured verbatim.
4. The operator's pre-existing dev-launcher state, if any, so restoration is to
   the recorded state rather than to "absent".

### Abort checklist

Expressed as **one throwaway script**, invoked from the harness's `finally` and
from a shell `trap … EXIT INT TERM`. It is not prose: a checklist with no
trigger leaves the dev-override gates open on any crash or interrupt.

1. `bin/.tmp-vcs-guard-baseline` and the fixture root do not exist.
2. `${PLUGIN_ROOT}/.accelerator-dev-launcher` matches its pre-flight state, and
   `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` and `ACCELERATOR_LAUNCHER_BIN` are
   unset.
3. `sha256(keys/accelerator-release.pub)` matches the pre-flight value.
4. The cached launcher, its `.minisig` and the staged shim match their
   pre-flight digests, and `.accelerator-unverified.log`'s byte count is
   unchanged from pre-flight.
5. Any lines appended to `.accelerator-unverified.log` during the session are
   recorded verbatim into Validation Results, and **only** those attributable
   to the measurement are removed. An unattributable line aborts the cleanup
   and is escalated rather than truncated away — this log is the trust chain's
   only durable alarm, and blind byte-count truncation would destroy a genuine
   integrity record or a concurrent session's.
6. `jj diff --summary cli/` is empty, and `mise run cli:check` is green.

Items 1 to 5 are positive assertions precisely because the artefacts they name
are gitignored, so no VCS gate can see them. Item 4 runs before item 5, since
item 5 mutates the file item 4 measures.

---

## Phase 1: The Comparative Measurement

### Overview

Measure `B` and `G` on one darwin host in one session, under both digest
backends, at the `n` and with the statistic 0204 pre-registered, emitting the
per-sample data Phase 2's budget needs.

### Changes Required:

#### 1. Recover the baseline's subject

**Files**: none tracked. A gitignored `bin/.tmp-vcs-guard-baseline`, or a
temp-dir arrangement per the note below.

```bash
jj file show -r cf42441e2aad- hooks/vcs-guard.sh \
  > bin/.tmp-vcs-guard-baseline
chmod +x bin/.tmp-vcs-guard-baseline
```

No `.sh` extension. The reason is not `bin/` glob scope: `shell_sources()`
(`tasks/shared/sources.py`) picks up `*.sh` plus a hard-coded
`_EXTRA_SHELL_SOURCES = ("bin/accelerator",)` allowlist, and `walk_files`
honours `.gitignore`, so the file is already invisible to the linters and — not
being a tracked `.sh` — outside the exec-bit invariant. The extension is
dropped because 0186 dropped it for `bin/.tmp-accelerator-before`, and 0186's
stated reason was jj's working-copy auto-snapshot.

Record the **resolved git commit id** of `cf42441e2aad-` and the **sha256** of
both the recovered script and `scripts/vcs-common.sh`. That is the one input
making `B` reproducible, and without the sha256 the recovery is verifiable only
inside this jj workspace. On a plain git clone the equivalent is `git show
<commit>:hooks/vcs-guard.sh`, which needs unshallowed history; a short hex
prefix can be a jj *change* id, which is why the resolved commit id is recorded
rather than the revset alone. The `chmod +x` makes the step POSIX-only.

The recovered script sources `"$SCRIPT_DIR/../scripts/vcs-common.sh"`, so it
needs a parent directory containing `scripts/`. `bin/` satisfies that but is
the launcher's **live cache root**: parking there puts an unreviewed executable
under the `store::TEMP_PREFIX` namespace reserved for in-flight atomic writes,
and adds an entry to the very directory whose `cache::find` scan cost Phase 2
budgets. Prefer a temp dir `T` outside the plugin root with
`T/scripts -> <repo>/scripts` symlinked and the baseline at `T/bin/`, so the
live `vcs-common.sh` still resolves. If `bin/` is used instead, record the
cache-root entry count both with and without the parked baseline.

#### 2. The harness

Shape inherited from 0186
(`meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md:1073-1135`)
but adapted, not reused: 0186's script passes no stdin, checks stdout with
`startswith("accelerator ")`, and swaps variants by `jj file show` of a
bootstrap revision. The adapted harness is recorded verbatim in Validation
Results alongside its output.

- **Interleaved sampling at 0204's `n`**, not batched per variant — batching
  either side of a swap aliases drift onto the difference — with order
  alternating within each pair.
- **One Python process** reading the clock around each `subprocess.run`. A
  per-call `python3` clock read puts an interpreter startup inside the measured
  interval.
- **Per-sample emission for Phase 2.** The harness records whatever per-sample
  quantities 0204's decomposition needs, not only the wall-clock bracket.
  Phase 2 cannot reconstruct them afterwards without breaking the one-session
  constraint.
- **Two instrument floors**, a trivial bash script and `true` resolved via
  `shutil.which('true')` against the **subprocess's** environment (busybox puts
  it at `/bin/true`). Assert the floor binary was found before sampling. Whether
  either is subtracted is 0204's decision; record both regardless, and note that
  the bash floor contains bash interpreter startup (~4.5 ms in 0186), which is a
  real bootstrap cost.
- **Dispersion.** Report `n`, min, median, p90 and IQR per variant. Medians
  alone cannot carry a 10% decision: 0186's `before` variant ran min 119.02 /
  median 125.35 / **p90 234.15**, and even its quiet `after` variant spanned
  27.18–32.44 around a 29.92 median. Record `p90(G)/p90(B)` as non-gating
  context — for a hook on every Bash tool call, the tail is what users feel.
- **Both digest backends.** Run the full sequence twice, once with the fast
  backend on the pinned `PATH` and once with it removed so `shasum` is reached,
  and record both ratios.

`B` is the recovered baseline; `G` is `${PLUGIN_ROOT}/bin/accelerator vcs guard
--format=hook --fail-safe`, dispatched through the real bootstrap with the cache
warm. Both are invoked by **absolute path** — the subprocess cwd is the fixture,
so a repo-relative invocation would resolve the baseline's `SCRIPT_DIR` against
the wrong directory.

Both receive **byte-identical stdin**: a PreToolUse `Bash` envelope naming one
of the guard's blocked `git` subcommands. 0169's criterion names `git status`
specifically; if a different subcommand is used, that is a deviation and is
recorded as one. The guard reads only `.tool_input.command`, so any other field
is inert and is recorded as such. The envelope is recorded verbatim.

#### 3. Fixture

A fresh pure-jj scratch repository, created in an OS temp directory **outside
the plugin root and outside any jj workspace** via a context manager so an
abnormal exit still removes it, with its resolved path recorded. A fixture
nested inside the live workspace would let the outer auto-snapshot pull it into
the working copy — the hazard `.gitignore:30` already documents.

This is load-bearing: `classify_checkout`'s spawn set depends on the checkout
kind, and at up to ~24 ms per `jj` spawn a colocated or live-workspace cwd adds
tens of milliseconds to `B` while `G` barely moves, shifting the ratio by more
than the gate margin in the direction that makes the gate easier to pass.

**Pinned and asserted.** `jj git init` colocates by default at 0.43, and a
colocated fixture emits **warn**, not the blocked decision — so the harness
would silently measure the wrong path. Create it with `jj --config
git.colocate=false git init --quiet`, matching what
`generate_decision_table.py:130` already does. Set `GIT_CEILING_DIRECTORIES`
from an `os.path.realpath`-canonicalised root — git ignores non-canonical
entries and does not resolve symlinks by default, and macOS `$TMPDIR` sits under
a `/var → /private/var` symlink, so a naive entry is silently ignored on exactly
the primary host. `GIT_CEILING_DIRECTORIES` has no jj equivalent, so the
pre-sampling decision-shape probe is the authority, not the ceiling.

Record the spawn count and per-spawn cost observed on the fixture alongside `B`.

#### 4. Per-sample validity

Assert on every sample that both variants produce the expected **blocked**
decision on the same stdin, and abort on the first mismatch.

The two guards emit structurally different envelopes, so compare **after
normalisation using the Rust-side mapping** at
`cli/vcs-cli/tests/guard_decision_table.rs:99-141`, which maps
`permissionDecisionReason` onto the label `block` — the repo's own vocabulary
for this outcome. Do **not** use `generate_decision_table.py:160`: it handles
only the legacy shell shape and maps the Rust deny envelope to `("allow","")`,
which is also what an empty stdout from a swallowed fail-safe produces, so it
would pass exactly the failures this gate exists to catch. Pin the expected
reason text and record both raw envelope shapes verbatim.

The exit code carries no decision information — 0 for block, allow and
degradation alike — so it is asserted as a liveness check only.

Assert after the run that the cached asset's, the cached launcher's, its
`.minisig`'s and the staged shim's inode and mtime are unchanged. Every non-hit
route ends in `cache::store`, which renames a fresh inode over the entry, so
this is a cheap branch witness — but restricted to the sub-binary it would let a
re-fetched launcher or re-staged shim inflate the sample undetected.

#### 5. Preconditions

Asserted by the harness over the exact environment handed to `subprocess.run`:

- No `ACCELERATOR_*` override is set — matching on key *names* (`[k for k in env
  if k.startswith("ACCELERATOR_")] == []`) rather than grepping `env` output,
  which also matches values and is line-oriented. This covers
  `ACCELERATOR_VCS_BIN`, `ACCELERATOR_BIN`, `ACCELERATOR_CACHE_DIR`,
  `ACCELERATOR_PLUGIN_ROOT`, `ACCELERATOR_RELEASE_BASE_URL` (`main.rs:38-40`)
  and the `ACCELERATOR_UNAME_S`/`_M` seams (`bin/accelerator:17-18`). Print the
  observed keys rather than only asserting absence. Since no override is set,
  the cache root is always `${plugin_root}/bin` (`bin/accelerator:201`) — record
  that rather than a scoping choice that is not available.
- `${PLUGIN_ROOT}/.accelerator-dev-launcher` is absent.
- No other Claude Code session is active against this plugin root. A concurrent
  session appends to `.accelerator-unverified.log` and can flip the launcher or
  shim inode, failing the branch witness for a benign reason.
- One discarded warm-up dispatch has completed, so the launcher takes the
  cache-hit branch.
- **`PATH` is pinned and printed**, twice — once per digest backend. Record the
  absolute path and version of `jj`, `git`, `bash`, `realpath`, `jq` and the
  resolved digest backend, plus `sys.executable`, `sys.version` and
  `time.get_clock_info('perf_counter')`. The pinned `PATH` is a host-specific
  value; the linux hand-off constructs an equivalent resolving the same named
  tools rather than reusing it.
- Host, OS version, chip and plugin version.
- Machine state, captured and gated. Record the raw load and the CPU count as
  **two separate values**, not a derived ratio: on linux `/proc/loadavg` is
  host-scoped regardless of cgroup membership, so dividing it by a container's
  quota yields a meaningless number. Resolve the count through a chain that
  terminates safely — cgroup v2 quota if resolvable, else
  `os.process_cpu_count()` if present (3.13+), else `os.sched_getaffinity`
  if present (**Linux-only**), else `os.cpu_count()` — recording which rung
  fired. A chain ending at `sched_getaffinity` raises `AttributeError` on the
  primary darwin host. If the cgroup rung is implemented it must resolve the
  process's own cgroup path from `/proc/self/cgroup`, take the minimum quota
  across ancestors, treat `max` as unlimited and fall back to v1 `cpu.cfs_*`;
  otherwise record `cpu.max`'s raw contents as context and do not claim quota
  awareness.
- Power probes are **additive**, each recording `unknown` when absent so one
  harness runs on both OSes: `pmset -g ps` plus `pmset -g therm` and the
  low-power-mode flag on darwin (AC-versus-battery alone misses the thermal
  throttling that actually perturbs a 10% margin);
  `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`,
  `intel_pstate/no_turbo` and `power_supply/*/status` on linux. All go `unknown`
  on the headless hosts the linux hand-off will use.

### Success Criteria:

#### Automated Verification:

- [ ] The recovered baseline runs and exits cleanly against the test envelope
- [ ] The fixture's blocked decision shape is asserted on a probe sample before
      sampling begins
- [ ] Preconditions pass over the environment actually handed to
      `subprocess.run`, and the observed `ACCELERATOR_*` keys, both pinned
      `PATH`s and every resolved tool path and version are printed
- [ ] Every sample's normalised decision matches the expected `block` for both
      variants, using the Rust-side mapping; the run aborts on first mismatch
- [ ] The cached asset's, launcher's, `.minisig`'s and staged shim's inode and
      mtime are unchanged after the run

#### Manual Verification:

- [ ] `B`, `G` and the ratio are recorded **per digest backend**, with `n`, min,
      median, p90, IQR per variant and `p90(G)/p90(B)` as context
- [ ] The per-sample quantities Phase 2's budget needs were emitted and retained
- [ ] Baseline provenance is recorded: resolved git commit id, sha256 of the
      recovered script and of `scripts/vcs-common.sh`
- [ ] The envelope, fixture path, spawn count and per-spawn cost, host/OS/chip,
      plugin version, tool paths and versions, interpreter and clock info,
      machine load with its CPU-count rung, and power state are recorded
- [ ] Both instrument floors are recorded, with 0204's subtraction policy
      restated
- [ ] The harness is recorded verbatim, not by cross-reference
- [ ] `bin/.tmp-vcs-guard-baseline` and the fixture root do not exist, asserted
      positively
- [ ] The abort checklist script ran and passed

---

## Phase 2: The Composition Budget

### Overview

Explain `G` rather than merely reporting it, using 0204's term set, cross-check
set and residual definition. This is Phase 1's analysis step, in the same
session-scoped unit.

### Changes Required:

#### 1. The budget

Account for `G` across the terms 0204 named, each measured by the method 0204
recorded, with the cross-checks 0204 specified and the residual computed as 0204
defined it — including the **cross-checked fraction of `G`** stated as a number,
so a residual computed over a minority of the measurement is visible rather than
flattering.

This plan does not restate the term boundaries. An earlier draft listed a
bootstrap row and a separate launcher-startup row without stating their
boundaries, and the two overlap by construction: a built-in dispatch already
contains the launcher's `execve`, dynamic loading, `logging::init` and clap
parse. Whether those are separable at all is SQ-1's question, and the budget
here uses whatever answer it recorded.

Two properties carry over regardless of 0204's answer:

- **Medians are not additive.** Report the median and IQR of each term
  alongside the residual, and state explicitly that term medians are not
  expected to sum to the `G` median.
- **A negative residual is evidence of double-counting or a misplaced
  boundary**, not of a fast implementation.

Record the cache-root entry count and total size: `cache::find`
(`cache.rs:51-73`) scans a directory the module header declares needs no
eviction (`cache.rs:1-6`), so the scan term is history-dependent and not
comparable between a fresh install and a long-lived plugin root.

#### 2. Apply 0204's outcome taxonomy

Classify the result using the branches 0204 pre-registered, which are disjoint
on stated arithmetic and cover every reachable outcome — including an
invalidated session and a design-infeasible finding, neither of which yields a
ratio.

Record in every branch: both medians with dispersion and the interval, per
digest backend; the budget with its residual as 0204 defines it; and the
cross-checked fraction of `G`.

Where the branch is an overrun, record the follow-up it triggers. Note that
0191's saving is hard-capped at ~2.48 ms — it buys **process-spawn overhead**,
not hashing throughput; its own table shows 7.05 ms for two `$(sha256sum f |
awk …)` substitutions against 4.57 ms for one `$(sha256sum f1 f2)` over the same
~475 KB — so for any wider gap, naming a 0191 figure is unsatisfiable and the
shortfall is attributed elsewhere. If `reverify` alone exceeds the gap, raise
the warm-dispatch verification cost as its own work item, with levers ordered
by 0204's separated sub-operation figures rather than by assumption, and note
that the cache-hit `sha256` removal must be scoped to that call site —
`verify_binary` is shared with `fetch_verify_store`, where the digest comes from
the signed manifest and does bind the bytes — and that an mmap is **not**
behaviour-preserving, since two passes over a mapping of a user-writable file
can observe different bytes and truncation raises SIGBUS rather than a clean
`Cache` error.

Any frontmatter edit the branch requires on 0189 is made in Phase 3, which owns
0189's edits and runs the corpus gate. Phase 2 records the required dependency
as an outcome.

### Success Criteria:

#### Manual Verification:

- [ ] Every term 0204 named is recorded with its measurement method
- [ ] Every cross-check 0204 specified is recorded, and the cross-checked
      fraction of `G` is stated as a number
- [ ] The residual is computed as 0204 defines it and reported with the
      dispersion treatment 0204 specified
- [ ] The outcome is classified into exactly one of 0204's branches, and that
      branch's actions are carried out and recorded
- [ ] The cache-root entry count and total size are recorded
- [ ] The abort checklist script ran and passed

---

## Phase 3: Record and Discharge

### Overview

Land the figures where they stop the obligation being orphaned a third time,
and replace the stale dependency premise with a guard keyed on the outcome.

### Changes Required:

#### 1. Discharge 0169 — four locations across three documents

- Fill the five `_pending_` slots (B, G, ratio, payload + fixture, host + OS)
  in **`meta/work/0169-vcs-subdomain-and-hooks-migration.md`**, under its
  `## Validation Results` heading, with a pointer here.
- Resolve the work item's own unchecked latency acceptance criterion at
  **`meta/work/0169-vcs-subdomain-and-hooks-migration.md:382-389`** — distinct
  from the Validation Results slots.
- Resolve the Phase 10 latency criterion in
  **`meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md:1507`**.
- Resolve the unchecked "Warm-call latency" item in
  **`meta/validations/2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation.md`**.

The work-item-versus-plan split matters: the 0169 *plan* has no
`## Validation Results` section and no `_pending_` markers, so a step aimed
there would find nothing to fill.

**Also retract the stale release-asset premise where those same documents
assert it** — `meta/validations/2026-08-05-0169-…:163-167` ("requires a
published, minisign-signed `accelerator-vcs` release asset that does not yet
exist") and `meta/plans/2026-08-05-0169-…:1514-1518` ("neither exists
pre-release"). Both documents are being edited by this phase anyway; leaving
the premise live means the corpus keeps asserting in two more places the claim
this plan's Current State Analysis opens by disproving.

**Branch on Phase 2's outcome:**

- **The branch where the gate holds** — resolve each criterion and backfill.
- **Every other branch** — backfill whatever figures exist, and against each
  criterion record a dated resolution naming the outcome as a **re-opened
  obligation with a named owner and a follow-up work item**. An "accepted
  deviation" is available only with an approver named outside this measurement
  and a rationale that does not appeal to the observed number; otherwise the
  post-hoc relaxation the plan pre-registers against re-enters at the point of
  discharge. Leave the latency acceptance criteria **unticked** on any branch
  where the gate did not hold.

Each tick carries an inline note wherever the method differed from what the
criterion states — `n`, the gate statistic, and the payload are all candidates.

#### 2. Replace 0189's stale dependency claim

**File**: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`

0189's `blocked_by: ["work-item:0169"]` edge is already satisfied — 0169 is
`done` — so the Dependencies bullet is its only prose closure guard. But the
**stronger** guard is the unticked latency acceptance criterion at `:168-170`,
which this item must not unblock unconditionally.

So: retract the release-cut premise as stale, citing the `v1.24.0-pre.36`
assets by name, and in the same edit install a closure guard **keyed on the
outcome, not on the recording** — 0189 may not close while the measured result
falls in any branch where the gate did not hold, absent a recorded, owner-named
acceptance. Keying it on "the figures are recorded" would make it born
discharged, since this phase records them.

The stale premise appears in **five** passages, not four. Enumerate them by
quoted text — `:112-113` ("release-gated (see Dependencies)"), `:168-170`
("Blocked until a signed `accelerator-vcs` release asset exists"), `:197-203`
("**This item cannot close before that release.**"), `:256-258` ("cannot close
until the epic-0136 release cut produces a signed `accelerator-vcs` asset") and
`:268-270` ("the measurement half is release-gated rather than urgent") — and
retract each as a dated note beside the original text, not a silent rewrite.
Record the found set, so a sixth occurrence added later is caught by search
rather than by a hard-coded count.

#### 3. Record 0189's own figures

Add a `## Validation Results` section to 0189's work item carrying the outcome,
and amend 0189's acceptance criterion 11 to name which sibling plan records
which artefact.

Note the corpus convention: `## Validation Results` otherwise appears in work
items, not plans. This plan keeps the harness listing and the raw record here
because they are too large for the work item, and makes the **work item the
authoritative summary** with the plan as its appendix — stated explicitly so
the two cannot be read as competing records.

#### 4. Record the follow-ups

Raise as named work items with ids, parent `work-item:0189`, frontmatter and
cross-links per the repo's conventions:

- **The linux measurement hand-off.** State the darwin-only scope, that
  darwin-x64 and linux-arm64 have no CI lane, and that the transfer direction is
  an open question rather than a known one. Its **first task is to commit the
  harness**, so the transcript is not re-typed a third time. Prerequisites for
  *measuring*: a linux host with `jj` at the pinned version, `git`, `jq`,
  `realpath`, `bash` and network egress to the release base URL — no build,
  since the shipped musl artefact is fetched and verified. For *decomposing*:
  `rustup target add <musl triple>` plus a musl-capable linker natively;
  `cargo-zigbuild` and `ziglang` are the cross-from-darwin mechanism, not a
  native requirement.
  `reverify` ms-per-MB must be re-measured per platform alias, recorded against
  (architecture, SHA-extension support, libc) rather than the OS name.
- **The committed-harness / measurement-policy decision**, with an owner.
- **The warm-dispatch verification cost**, only if Phase 2's outcome raised it.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-corpus
      -E 'test(this_repositorys_own_corpus_is_clean)'` is green — the
      unconditional repo-corpus frontmatter gate covering every meta document
      this phase edits
- [ ] `mise run check` is green

#### Manual Verification:

- [ ] All four 0169 record locations are discharged, and the stale
      release-asset premise is retracted in the two 0169 documents that assert
      it
- [ ] Each discharge follows the branch Phase 2's outcome selects, with the
      latency criteria left unticked on any branch where the gate did not hold
- [ ] Each tick carries an inline note wherever the method differed from the
      criterion as written
- [ ] 0189's five stale passages are each retracted as dated notes, the found
      set is recorded, and the release-cut blocker is replaced by a closure
      guard keyed on the **outcome**
- [ ] 0189 carries a `## Validation Results` section, named as the
      authoritative summary, and criterion 11 names which plan records which
      artefact
- [ ] `last_updated`/`last_updated_by` refreshed on every meta document touched
- [ ] The two unconditional follow-up work items exist with ids, parent and
      cross-links, plus the verification-cost item if Phase 2's outcome raised
      it — recorded as not-applicable otherwise
- [ ] **The Deviations section is complete, or explicitly records "none"**
- [ ] The abort checklist script ran and passed

---

## Testing Strategy

This plan adds no tests. It is a measurement, and its correctness rests on the
harness's own assertions:

- Per-sample decision equivalence between `B` and `G` after normalisation
  through the Rust-side mapping, aborting on first mismatch — the guard against
  a PASS manufactured by fail-safe short-circuits.
- Fixture decision shape asserted on a probe sample before sampling, so a
  colocated fixture cannot silently measure the warn path.
- Post-run inode and mtime invariance across the cached asset, the cached
  launcher, its `.minisig` and the staged shim.
- Precondition assertions over the exact subprocess environment, including two
  pinned `PATH`s and every resolved tool path and version.
- Instrument floors measured in the same run rather than assumed.
- The abort checklist script, invoked from a `trap` and a `finally` rather than
  relied on as prose.

The one automated gate this plan relies on is the repo's own corpus frontmatter
validator over the meta documents Phase 3 edits.

### Manual Testing Steps:

1. Confirm work item 0204 is closed and its answers recorded.
2. Run the pre-flight capture.
3. Recover the baseline, run the harness under both digest backends on a quiet
   darwin host in one session.
4. Compute the budget and residual per 0204's definitions; classify the outcome
   into exactly one of its branches.
5. Discharge 0169's four locations and 0189's five passages on the branch the
   outcome selects; raise the follow-ups.
6. Run the abort checklist script and confirm every throwaway artefact is
   positively asserted absent.

## Performance Considerations

This plan changes no production code and adopts no instrumentation — SQ-1's
route is 0204's decision, and any instrumentation it recommends is 0204's to
scope and clean up. If a route reaching the launcher through the dev override is
ever adopted, note that its residual risk outlives the source revert: the
compiled binary sits in gitignored `cli/target/`, which is exactly where a
contributor's normal dev-launcher build lives, so the artefact must be removed
or rebuilt rather than merely left unreferenced. Any such instrumentation must
write to stderr or a dedicated file, **never stdout** — stdout is the guard's
decision envelope (`hooks/hooks.json:47`, `kernel/src/hooks.rs:29-31`), and an
unparseable decision from a fail-safe guard lets the blocked `git` command
proceed.

## Migration Notes

None. No shipped artefact changes.

## Validation Results

### Pre-flight capture

_Pending._ Slots: `sha256(keys/accelerator-release.pub)` from the committed
revision; digests of the cached launcher, its `.minisig` and the staged shim;
`.accelerator-unverified.log` byte count and full contents; the operator's
pre-existing dev-launcher state.

### Method (from work item 0204)

_Pending 0204._ Slots: a pointer to 0204's recorded answers for SQ-1 to SQ-5,
and the `reverify` figures it delivered, so this record is readable standalone.

### Latency figures

_Pending Phase 1._ Slots: `B`, `G` and ratio **per digest backend**, with the
interval 0204 specified; `n`/min/median/p90/IQR per variant; `p90(G)/p90(B)`;
both instrument floors with 0204's subtraction policy; stdin envelope verbatim;
both raw envelope shapes; fixture path, description, spawn count and per-spawn
cost; baseline provenance (resolved git commit id, sha256 of the recovered
script and of `scripts/vcs-common.sh`); harness verbatim; host/OS/chip; plugin
version; both pinned `PATH`s with every resolved tool path and version including
`jq`; interpreter and `perf_counter` clock info; machine load with its CPU-count
rung; power state.

### Composition budget

_Pending Phase 2._ Slots: every term 0204 named with its measurement method;
every cross-check with its result; the cross-checked fraction of `G` as a
number; the residual per 0204's definition with its dispersion treatment; the
cache-root entry count and total size; the outcome branch and its actions.

### Cleanup evidence

_Pending._ Slots: the post-run inode/mtime witness; positive absence of
`bin/.tmp-vcs-guard-baseline` and the fixture root; any lines appended to
`.accelerator-unverified.log` recorded verbatim with which were removed and why;
the abort checklist script's output per phase.

### Deviations

_Pending._ Slots: any departure from 0169's stated 20 samples; the gate
statistic where it differs from 0169's median rule, which is a **deliberate
strengthening** rather than the inherited criterion; the stdin payload where it
differs from `git status`; any 0204 answer that departed from its own
pre-registered default; any success criterion discharged differently from as
written. Mirrors 0186's deviation-recording pattern
(`meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md:608-616`).
Records "none" explicitly if empty.

### Discharge record

_Pending Phase 3._ Slots: 0169's four locations discharged and the premise
retracted in its two documents; 0189's five stale passages retracted with the
found set recorded; the outcome-keyed closure guard installed; 0189's
Validation Results section added and criterion 11 amended; the follow-up work
items raised or recorded not-applicable.

## References

- Work item: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
- **Blocking spike**:
  `meta/work/0204-close-the-warm-dispatch-measurement-method.md` — owns SQ-1 to
  SQ-5; this plan consumes its answers
- Sibling plan:
  `meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md` —
  lands first
- Reviews of this plan:
  `meta/reviews/plans/2026-08-11-0189-warm-dispatch-latency-measurement-review-1.md`
  — pass 2's assessment raised 0204
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — five `_pending_` slots
  under `## Validation Results`, and its own criterion at `:382-389`
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 10's
  criterion at `:1507`, the stale premise at `:1514-1518`
- `meta/validations/2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation.md`
  — the third record location; stale premise at `:163-167`
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  **delivered** figures: 29.92 ms warm bootstrap median (`:517`), the 35.1 ms
  shell-guard row (`:66`) and its method-incomparability note (`:615`), the
  10.6 ms row (`:71`) re-derived at 3.72 ms (`:577`), and the
  deviation-recording pattern (`:608-616`)
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  the harness shape (`:1073-1135`), the per-sample stdout guard (`:1096-1102`),
  and the same incomparability note at `:1252`
- `meta/work/0188-library-backed-vcs-adapter.md:1084-1085` — delivered
  subprocess figures (23.84 ms for `jj log -r @ -T commit_id`, 5.34 ms
  `git rev-parse`)
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` — buys spawn
  overhead, not hashing throughput; capped at ~2.48 ms
- `meta/work/0199-retire-vcs-common-sh-residue-and-launcher-link-refresh.md` —
  scoped to decide whether `classify_checkout` leaves `scripts/vcs-common.sh`
- `cli/vcs-cli/tests/guard_decision_table.rs:99-141` — the Rust-side envelope
  normaliser, mapping `permissionDecisionReason` onto the label `block`
- `cli/kernel/src/hooks.rs:29-31` — the Rust guard's deny envelope
- `hooks/test-fixtures/vcs-guard/generate_decision_table.py:130`, `:160` — the
  fixture's `git.colocate=false` incantation, and the **shell-only** normaliser
  that must not be used for cross-variant comparison
- `cli/corpus-cli/tests/frontmatter_goldens.rs:309` —
  `this_repositorys_own_corpus_is_clean`, in package `accelerator-corpus`
- `cli/launcher/src/launch/core.rs:219-224`,
  `cli/launcher/src/main.rs:38-40`, `:215-226` — the fail-safe swallow, the
  environment seams and `handle_dispatch_error`
- `cli/launcher/src/launch/outbound/resolve/cache.rs:1-6`, `:51-73` — the
  never-evicted cache root and its scan
- `cli/launcher/build.rs:31-32`, `:43` — the embed of
  `keys/accelerator-release.pub` and its `cargo:rerun-if-changed`
- `bin/accelerator:165`, `:201`, `:225`, `:272-278` — the verification key, the
  cache root, the dev-launcher marker and the digest backend
- `scripts/vcs-common.sh:177` — `classify_checkout`'s checkout-dependent spawns
- `tasks/shared/sources.py` — `shell_sources()`'s `*.sh` plus
  `_EXTRA_SHELL_SOURCES` allowlist, and `walk_files`'s gitignore honouring
