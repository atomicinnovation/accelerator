---
type: "plan"
id: "2026-09-20-0217-measure-warm-dispatch-on-linux"
title: "Measure Warm Dispatch on Linux Implementation Plan"
date: "2026-09-20T20:46:56+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0217"
parent: "work-item:0217"
derived_from: ["codebase-research:2026-09-20-0217-measure-warm-dispatch-on-linux"]
relates_to: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement", "work-item:0216", "work-item:0219", "work-item:0205"]
tags: ["cli", "launcher", "performance", "measurement", "calibration"]
revision: "0f4ced25f0eb414b0b072aa9f054631943b929f8"
repository: "accelerator"
last_updated: "2026-09-21T21:55:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Measure Warm Dispatch on Linux Implementation Plan

## Overview

Produce a calibrated `("Linux", "aarch64")` musl platform entry for the
committed warm-dispatch harness by running it on a native aarch64 linux guest,
deriving linux-specific ceilings and floor gates from the measured record, and
recording the aarch64 musl `verifier::sha256_hex` throughput that work item 0216
deferred here. The harness is data-driven: adding a platform is a new
`PlatformEntry` plus six README bullets held in lockstep, not a re-authoring.

Three decisions taken during planning widen the code surface beyond a pure
measurement task. The throughput figure must be a genuine
`aarch64-unknown-linux-musl` build, taken by exporting `CARGO_BUILD_TARGET` for
the session. The harness must **degrade gracefully** when the fallback
digest backend (`shasum`, a Perl program) is absent: today it aborts session
setup, whereas acceptance criterion 5 requires C3/C4/C6 recorded not applicable
(branch 7). And the harness must **earn the `calibrated` note honestly on
linux**: `observed_chip` returns the first line of `lscpu -J` (`{`) rather than
the CPU brand, and `chip`/`shasum` are exact-match calibration keys, so the libc
identity needs its own non-matched field rather than riding in `chip`. All three
are self-contained, darwin-testable code changes that land in Phase 1,
independently of the linux run.

## Current State Analysis

The harness lives in two files. `tasks/measure.py` is the driver (process,
clock, filesystem, session lifecycle, sampling rig, farm construction, and the
`PLATFORM_TABLE` data). `tasks/shared/measurement.py` is the pure analysis core
(the `Branch`/`Validity`/`CellKind` enums, `classify`, `power_state`, the
`Calibration` and `PlatformEntry` dataclasses). `PLATFORM_TABLE`
(`tasks/measure.py:148-199`) holds a single `("Darwin", "arm64")` entry.

The gate numbers are pre-registered constants stored as data, not values
computed at run time. `cells_for(entry)` (`tasks/measure.py:812-863`) reads the
four ceilings off the entry; `gate_floors` (`tasks/measure.py:1590-1619`) gates
on the two floor constants with a plain `<=`. The
"smallest multiple of 10 ms leaving ≥18% headroom" and "40–80% above the
measured floor" rules exist only in 0189's plan prose; the operator applies them
by hand and enters the resulting round numbers.

Three current behaviours shape the work:

- **A host with no calibrated entry writes no record.** `warm_dispatch` prints a
  branch-7 advisory (`tasks/measure.py:1066-1070`), then `run_session` raises
  `PreconditionFailureError` when `entry is None` (`:1646-1651`) before any
  record is produced. `--platform-key` only relabels which key is looked up
  (`resolve_platform_key`, `tasks/shared/measurement.py:872-885`); it does not
  bypass the entry requirement. The first linux record therefore needs *some*
  `("Linux", "aarch64")` entry to exist first.
- **A host with no fallback backend aborts rig setup.** `build_rig` builds both
  farms unconditionally (`tasks/measure.py:1320-1321`); `build_farm` raises the
  moment any requested tool — `shasum` included — fails to resolve on PATH
  (`:944-957`), *before* `assert_backends` (`:2214-2234`) is even reached. So the
  work item's AC 5 ("recorded not applicable") is not the harness's actual
  behaviour; as written it aborts.
- **The throughput term is captured inside the session.** `decompose_terms`
  (`tasks/measure.py:2637-2676`) runs `cargo test --release --test warm_terms`
  and inherits `os.environ`; `assemble_terms_report` persists `asset_bytes`
  beside `record["terms"]` (`:1858-1881`). The throughput is a documented
  derivation off the record, not a separate run.

## Desired End State

A committed, valid, **calibrated** `warm-dispatch-N.json` under
`meta/measurements/` taken on a native aarch64 linux guest, and a
`PLATFORM_TABLE` entry for `("Linux", "aarch64")` with all four original
calibration provenance fields plus the new `libc` field (`musl`) recorded, and
`chip` recording the parsed `lscpu` CPU brand so the `calibrated` note is earned.
The README `### Criterion constants` block and its lockstep test carry the linux
entry's six derived constants. A Perl-less host records C3/C4/C6 as branch 7
instead of aborting. The record's `power_state` names a real linux power source
or, where the guest exposes none, the source it probed and that it returned no
reading. The aarch64 musl `verifier::sha256_hex` throughput is recorded and is
≥ 700 MB/s, confirming the ARMv8 SHA-2 hardware path engaged.

Verification of the end state:

- `mise run test:unit:tasks` passes, including the graceful-degradation cases
  and the lockstep and linux farm-completeness tests.
- `mise run check` exits 0.
- A `warm-dispatch-N.json` exists under `meta/measurements/` whose
  `analysis.validity` is `valid`, whose `provenance.calibration.note` reads
  `calibrated`, and whose `closure_verdict` is `true`.

### Key Discoveries

- `PlatformEntry` carries 11 fields; `Calibration` carries 4 today — `session`,
  `chip`, `bash`, `shasum` (`tasks/shared/measurement.py:777-805`) — and Phase 1
  adds a fifth, `libc`. The darwin entry leaves `bash`/`shasum` `None`
  (`tasks/measure.py:141-146`); AC 2 requires the four originals plus `libc`
  recorded for linux.
- `entry_platform()` derives the cache-name alias from the **host** uname via
  `host_platform()`/`UNAME_TO_ALIAS` (`tasks/measure.py:1032-1040`,
  `tasks/shared/targets.py:19-28`), *not* from `entry.key`. On an aarch64 guest
  it returns `linux-arm64` unaided. Setting the entry's `key="linux-arm64"`
  still matters — it is the dotted prefix in `criterion_constants()`
  (`tasks/measure.py:236-249`) and hence the README block — but the research's
  claim that a wrong key diverges cache-name prediction is incorrect.
- `closure_verdict` (`tasks/shared/measurement.py:581-595`) already holds when a
  gating cell is `NOT_APPLICABLE` **with** a recorded `accepted_by`. The
  graceful-degradation path reuses this: C3/C4 not applicable with an acceptance
  reason do not block closure.
- `classify_cell` (`tasks/measure.py:866-896`) constructs the not-applicable
  `CellOutcome` with `accepted_by` defaulting to `None`; no code path sets it
  today.
- `power_state` (`tasks/shared/measurement.py:698-713`) keys on `probe[0]` and
  records `"unknown"` only on `FileNotFoundError`/`PermissionError`. The ambient
  diagnostic runner uses `check=False` and returns `stdout.strip()`
  (`tasks/measure.py:488-495`), so a probe of a **missing** `/sys` file returns
  an empty string — neither `unknown` nor a named source. AC 7 needs a change so
  a no-reading probe names its source.
- `observed_chip` (`tasks/measure.py:2490-2507`) runs `sysctl` (a darwin MIB
  that fails silently on linux under the `check=False` runner) then `lscpu -J`,
  and returns `splitlines()[0]` — the first line of the JSON, `{`, never the
  brand. Because `chip` is exact-match compared by `unconfirmed_calibration_fields`
  (`tasks/shared/measurement.py:839-869`), the recorded chip could never equal
  `{`, so the `calibrated` note could never hold. Phase 1 fixes `observed_chip`
  to parse the `lscpu -J` "Model name" and records the libc identity in a new
  non-matched `libc` field rather than appending it to `chip`.
- The farm-completeness test (`tests/unit/tasks/test_measure.py:2113-2134`) is
  keyed only on the darwin entry; a parallel linux-keyed test is otherwise
  absent.
- `sha2 = "0.11"` with no feature flags (`cli/Cargo.toml:76`); backend selection
  is internal to `sha2` + `cpufeatures`, which reads the ARMv8 SHA-2 extension
  via `getauxval`/HWCAP on linux for any libc.

## What We're NOT Doing

- **Not extending `PLATFORM_TABLE` keying.** The architecture-only
  `(system, machine)` key is unchanged; SHA-extension facts ride in `shasum`
  provenance and the libc identity rides in a new non-matched `Calibration.libc`
  field. This refines the work item's 2026-09-15 free-text resolution, which
  assumed `chip`/`shasum` were free-text when they are in fact exact-match keys.
- **Not wiring a scheduled or CI arm64 lane.** The recurring absolute-budget lane
  is work item 0219's scope; this entry is its calibration source.
- **Not measuring `Linux/x86_64`.** A valuable secondary, but it does not
  discharge the 0216 aarch64-musl hand-off.
- **Not re-authoring the harness or the criterion.** `G`, `B`, the six cells and
  the seven branches are inherited from 0189 unchanged.
- **Not producing bare-metal ceilings.** The VM's absolute ceilings are accepted
  as VM-scoped and provisional per the work item's Assumptions and AC 3.
- **Not backfilling darwin's `None` `bash`/`shasum` provenance.** Out of scope;
  0205 never recorded them.

## Implementation Approach

Phase 1 hardens the harness in three darwin-testable ways: a missing fallback
backend degrades to branch 7 rather than aborting; a no-reading power probe names
its source; and `observed_chip` parses the real CPU brand while a new
`Calibration.libc` field carries the libc identity, so the `calibrated` note can
be earned on linux. It is pure Python, driven by unit tests, and needs no linux
guest — so it lands and merges first, de-risking AC 5, AC 7 and AC 2 before any
VM work.

Phases 2–4 are a necessary chain: you cannot calibrate before you measure. Phase
2 provisions the guest and takes a bootstrapping pass against a **local,
uncommitted placeholder** entry with loose gates, purely to read the measured
statistics. Phase 3 derives the round-number constants from those statistics and
lands the calibrated entry, README bullets and linux farm-completeness test as
one atomic change. Phase 4 re-runs against the finalised entry, confirms a clean
calibrated gating verdict, derives and checks the throughput, states the transfer
verdict, and commits the record.

## Phase 1: Degrade gracefully when the fallback backend is absent

### Overview

Make a host without `shasum` record C3/C4/C6 as branch 7 with an acceptance
reason, rather than aborting anywhere in the session. Make a no-reading power
probe record the source it probed. Fix `observed_chip` to parse the CPU brand and
add a `Calibration.libc` field so the `calibrated` note can hold on linux. All
are generic core changes, validated by unit tests on darwin, independent of the
linux run.

### Changes Required

#### 1. Detect fallback-backend availability and thread it through the rig

**File**: `tasks/measure.py`

`build_rig` computes availability once and records it on the `Rig`. When the
fallback backend is absent it builds the fast farm from the tool set minus
`FALLBACK_BACKEND`, skips the fallback farm, and asserts only the fast
direction.

```python
@dataclass(frozen=True)
class Rig:
    ...
    fallback_available: bool = True
```

The field defaults to `True` so existing `Rig` constructors and the darwin path
stay source-compatible; only `build_rig` on a Perl-less host sets it `False`.

```python
    fallback_available = shutil.which(FALLBACK_BACKEND) is not None
    farm_tools = tools if fallback_available else tuple(
        tool for tool in tools if tool != FALLBACK_BACKEND
    )
    build_farm(fast_farm, farm_tools, include_fast_backend=True)
    if fallback_available:
        build_farm(fallback_farm, farm_tools, include_fast_backend=False)
    assert_backends(fast_farm, fallback_farm, fallback_available=fallback_available)
```

#### 2. Relax the backend assertion when the fallback is not expected

**File**: `tasks/measure.py`

`assert_backends` gains a `fallback_available` flag defaulting to `True`; it
asserts the fallback direction only when the backend is expected. The default
keeps the existing both-present call sites source-compatible, so the only test
that must change semantically is `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable`,
which is **inverted** from asserting a raise to asserting the
`fallback_available=False` no-raise (see Success Criteria).

```python
def assert_backends(
    fast_farm: Path, fallback_farm: Path, *, fallback_available: bool = True
) -> None:
    if not (fast_farm / FAST_BACKEND).exists():
        raise PreconditionFailureError(...)
    if not fallback_available:
        return
    if (fallback_farm / FAST_BACKEND).exists():
        raise PreconditionFailureError(...)
    if not (fallback_farm / FALLBACK_BACKEND).exists():
        raise PreconditionFailureError(...)
```

#### 3. Skip fallback samples and accept the fallback cells

**File**: `tasks/measure.py`

`sample_blocks` drops scheduled `Variant.FALLBACK` samples when the rig reports
no fallback backend, leaving that arm empty.

```python
    schedule = [
        sample
        for sample in schedule
        if sample.pilot == pilot
        and sample.block in blocks
        and (rig.fallback_available or sample.variant is not Variant.FALLBACK)
    ]
```

`analyse` gains a `fallback_available: bool` keyword parameter, threaded from
`rig.fallback_available` at its `run_session` call site (`:1700-1705`).
`classify_cell` gains an `accepted_by: str | None = None` parameter it sets on
the not-applicable outcome. The reason is bound to a cell that has **no interval
on a host with no fallback backend** — not to `fallback_available` alone — so it
reaches exactly C3/C4/C6 (the only cells left without an interval once the
fallback arm is dropped) without a separate cell-name list to keep in sync, and
an unexpectedly empty arm on a healthy host is never relabelled as an intended
degradation:

```python
        accepted_by=(
            FALLBACK_ABSENT_REASON
            if interval is None and not fallback_available and fast_arm_populated
            else None
        ),
```

`fast_arm_populated` is whether the fast/baseline arm produced samples this run;
gating on it is what makes the "fast arm populated" guarantee below hold in code,
not just in prose.

The reason is a single named constant,
`FALLBACK_ABSENT_REASON = "no fallback digest backend on this host"`. On darwin
`fallback_available` is `True`, so every cell's `accepted_by` stays `None` and
the record is unchanged. The reason is applied only when the fast/baseline arm is
populated, so a non-standard block selection that empties a gated fast cell can
never be relabelled as an intended degradation and close spuriously.

#### 4. Guard the remaining fallback-farm consumers so nothing aborts

**File**: `tasks/measure.py`

Skipping the fallback farm in `build_rig` leaves two other consumers reaching for
a directory that no longer exists, so without this change the Perl-less host
still aborts — just at a later line than `build_rig`. `record_provenance`
(`:1644`) calls `tool_provenance(rig.fallback_farm, ...)`, which runs
`farm.iterdir()` on the never-created directory; and `close_the_budget` (`:1697`)
→ `measure_shell_terms` → `measure_digest_bracket(farm=rig.fallback_farm, ...)`
(`:2004-2011`) runs `shasum` through it. Both raise before `analyse` is reached.

Both skip the fallback direction when `rig.fallback_available` is `False`:
`tool_provenance` records an empty fallback-tool set and `measure_shell_terms`
omits the fallback digest bracket (its `digest_backend_cross_check` becomes
`None`). The two consumers are guarded by targeted unit tests driven through the
injected runner with a fake `Rig` (see Success Criteria) — not by a live
full-session run, which drives real `jj`/`cargo`/network and cannot run as a unit
test.

#### 5. Name the probed source when a power probe returns no reading

**File**: `tasks/shared/measurement.py`

`power_state` records a source-naming string when a probe yields no reading —
whether the executable is absent, permission is denied, or the output is empty —
instead of a bare `"unknown"` or empty string.

```python
def power_state(
    diagnostic_runner: Callable[[Sequence[str]], str],
    probes: Sequence[Sequence[str]],
) -> dict[str, str]:
    state = {}
    for probe in probes:
        try:
            reading = diagnostic_runner(probe).strip()
        except (FileNotFoundError, PermissionError):
            reading = ""
        state[probe[0]] = reading or f"no reading from {' '.join(probe)}"
    return state
```

#### 6. Parse the CPU brand and record the libc identity separately

**File**: `tasks/measure.py`, `tasks/shared/measurement.py`

`observed_chip` returns `splitlines()[0]` of `lscpu -J` — the literal `{` — so no
recorded `chip` can ever match it and the `calibrated` note can never hold. Fix
`observed_chip` to parse the `lscpu -J` "Model name" field (falling back to the
plain `lscpu` "Model name:" line), returning the brand string alone. This changes
only the linux path; darwin's `sysctl machdep.cpu.brand_string` branch is
untouched.

Because `chip`/`shasum` are exact-match compared by
`unconfirmed_calibration_fields` (`tasks/shared/measurement.py:839-869`), the
libc identity cannot ride in `chip`. Add a fifth `Calibration` field, `libc`,
that `unconfirmed_calibration_fields` does **not** compare — pure provenance,
carrying `musl` for the linux entry and defaulting to `None` for darwin:

```python
@dataclass(frozen=True)
class Calibration:
    session: str
    chip: str | None = None
    bash: str | None = None
    shasum: str | None = None
    libc: str | None = None
```

#### 7. Fail loudly when the throughput term is missing

**File**: `tasks/measure.py`

`decompose_terms` (`:2637-2676`) today raises only on a non-zero `cargo`
returncode; `parse_asset_bytes` returns `None` and `parse_term_report` returns
`{}` when the output is absent, and `close_the_budget` then quietly returns — so a
cross-target build that compiles but runs no tests persists `asset_bytes: null`
with a valid-looking session, silently dropping AC 6. Site the guard where that
"built but not run" outcome originates: after `decompose_terms` returns,
`close_the_budget` calls a small pure check that raises a
`PreconditionFailureError` when `asset_bytes` is `None` or the
`verifier::sha256_hex` term is absent, and that also asserts the recorded build
triple (from the Change 8 build block) ends in `-linux-musl`, so a forgotten
`CARGO_BUILD_TARGET` that builds the guest's default glibc toolchain cannot
persist a glibc figure mislabelled `musl`. The pure `assemble_terms_report` stays
tolerant, so its three existing `TestTermsReport` cases (and any legitimately
throughput-free path) keep passing; `parse_asset_bytes` is untouched (its
None-return test stands). The pure check is unit-tested directly on fabricated
`(asset_bytes, terms, triple)` inputs.

#### 8. Record the build and provisioning provenance

**File**: `tasks/measure.py`

`record_provenance` today captures the runtime farm-tool versions and
`platform.system/machine/release`, but not the build toolchain the musl
throughput figure depends on. Extend the recorded provenance with a build block —
the rust target triple, the resolved `cargo`/`rustc` version, the musl linker, the
guest-local cross-target runner config, and the `mise`-provisioned toolchain and
apt package versions — so the committed record can reconstruct how the throughput
was built. The VM identity the guest cannot probe portably (Colima version,
base-image digest) is recorded in the work item's `## Validation Results` instead.
A unit test drives the recorded values through a fake `session.diagnostics`
returning known cargo/rustc/target/linker strings and asserts them, not merely
that the keys exist.

### Success Criteria

#### Automated Verification

- [x] The Python component is clean: `mise run build-system:check`
- [x] The unit suite passes: `mise run test:unit:tasks`
- [x] New unit tests pass (single-file runner):
      `uv run pytest tests/unit/tasks/test_measure.py`
  - `classify_cell` with `interval=None` and an `accepted_by` reason returns a
    `NOT_APPLICABLE` outcome carrying that reason.
  - `closure_verdict` holds over C1/C2 PASS + C3/C4 not-applicable-with-reason +
    C5 PASS, and does **not** hold when C3/C4 are not applicable without a
    reason (regression guard).
  - A `fallback_backend_available()` helper (extracted from `build_rig`) returns
    `True`/`False` as `FALLBACK_BACKEND` resolves on PATH (availability faked);
    `build_farm` over the fallback-trimmed tool set and
    `assert_backends(..., fallback_available=False)` do not raise. `build_rig`'s
    own wiring drives real `jj`, so it is exercised by the manual Perl-less run,
    not a unit test.
  - `sample_blocks` with `fallback_available=False` yields an empty
    `Variant.FALLBACK` arm, and with `fallback_available=True` still yields a
    populated one (positive direction).
  - `analyse(..., fallback_available=False)` over an empty fallback arm returns
    C3/C4/C6 each `NOT_APPLICABLE` carrying `FALLBACK_ABSENT_REASON` — including
    non-gating C6, which `closure_verdict` cannot catch — while C1/C2/C5 carry
    `accepted_by is None` on that same degraded run, and `closure_verdict` holds;
    with `fallback_available=True` no cell carries a reason. `closure_verdict`
    stays **False** when the fast arm is empty on a fallback-absent host (the
    `fast_arm_populated` guard, a `--blocks=B` regression).
  - `tool_provenance` records an empty fallback-tool set and `measure_shell_terms`
    omits the fallback digest bracket and its `digest_backend_cross_check` when
    `rig.fallback_available` is `False` — both through the injected runner with a
    fake `Rig` (the cleanly-injected seams). Their callers `record_provenance` and
    `close_the_budget` drive real `bash`/`cargo`, so their fallback guarding rests
    on these sub-functions plus the manual Perl-less run, not a full unit test.
  - The Change 7 pure check (called from `close_the_budget`, not the assembler)
    raises on `asset_bytes is None`, on an absent `verifier::sha256_hex` term, and
    on a build triple not ending `-linux-musl`; it is unit-tested directly on
    fabricated inputs. `assemble_terms_report` is untouched, so its three existing
    `TestTermsReport` cases pass unchanged.
  - `power_state` records `no reading from <argv>` for a probe whose executable
    is missing, whose output is empty, and whose command errors; and records the
    real output when a probe succeeds.
  - `observed_chip` returns the brand alone from an `lscpu -J` fixture (and from
    the plain-`lscpu` fallback), not `{`; and the darwin `sysctl` branch still
    returns its brand string verbatim (the `lscpu` parse is a separate branch).
  - `unconfirmed_calibration_fields` ignores `libc`: a recorded `libc` never
    marks an otherwise-matching entry uncalibrated.
- [x] Existing tests are migrated, not merely added to:
      `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable` is
      inverted to the `fallback_available=False` no-raise semantics, the other
      `assert_backends` call sites rely on the `=True` default, and
      `test_an_absent_probe_yields_unknown_rather_than_propagating` moves to the
      source-naming string.
- [x] Darwin behaviour is unchanged: with both backends present, all six cells
      are still measured, `assert_backends` still asserts both directions, and
      C3/C4/C6 carry `accepted_by is None`.

#### Manual Verification

- [x] The graceful-degradation path reads as a modelled host property
      (fallback-backend availability), not as scattered conditionals, and adds no
      explanatory comments.

---

## Phase 2: Provision the guest and capture a completing pass

### Overview

Stand up a native aarch64 linux guest, install the toolchain, and take one
non-gating measurement pass against a local, uncommitted placeholder entry with
deliberately loose ceilings and floor gates. The pass exists only to read the
measured statistics; nothing here is committed.

### Changes Required

#### 1. Provision a native aarch64 Debian/Ubuntu guest

**Where**: a Colima `vz` (or `qemu` with HVF) aarch64 VM on the darwin-arm64
host, so execution is genuine aarch64/musl, never cross-arch emulation.

Install and verify:

- `mise` itself (it provisions uv, python, rust, jj and node at the repo pins),
  then `jj` at the `mise.toml` pin, `git`, `jq`, `realpath`, `bash`, `awk`,
  `chmod`, a fetcher (`curl` or `wget`), and the rest of the mechanically derived
  tool set.
- A resolvable sha256 backend (`sha256sum` from coreutils) **and** Perl `shasum`
  (`perl` is present in the Debian/Ubuntu base; confirm `shasum` resolves) so all
  six cells measure.
- `rustup`, the `aarch64-unknown-linux-musl` target
  (`mise run deps:install:rust-targets` adds it), and a musl-capable linker
  (`apt-get install musl-tools`; set
  `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc` if cargo does not
  pick it up).
- Network egress to the release base URL, crates.io (for the `warm_terms`
  build), the apt mirrors, and the rustup/mise toolchain mirrors; and a
  published, minisign-signed release whose version carries 0216's `sha2` 0.11
  backend. Without that release the warm-cache fetch has nothing to warm from.

Pin the environment so the run is reproducible. The probeable versions — the
`mise`-provisioned toolchain (uv, python, rust, jj, node) and the apt package
versions — are recorded in the measurement record's provenance by Phase 1 Change
8; the VM identity the guest cannot probe portably (Colima version, base-image
digest) is recorded in the work item's `## Validation Results`. Configure
`target.aarch64-unknown-linux-musl.runner` — **guest-local and gated to the musl
target**, so it does not change cross-target execution for other developers or CI
— so `cargo test --target` always executes the static binary natively, closing
the "built but not run" gap Phase 4 otherwise has to detect by hand; record its
use alongside the build block. This is a Debian/Ubuntu provisioning path
(`apt-get`, `musl-gcc`); a non-Debian aarch64 host would use different package and
linker names.

#### 2. Seed a local placeholder entry (uncommitted)

**File**: `tasks/measure.py` (local working copy only — reverted before Phase 3)

Add a `("Linux", "aarch64")` entry with loose ceilings (well above any plausible
measurement, e.g. `10000.0`) and loose floor gates (e.g. `1000.0`), so nothing
gates spuriously and the session runs to completion:

```python
    ("Linux", "aarch64"): PlatformEntry(
        key="linux-arm64",
        path_tools=(...),
        power_probes=(...),
        median_ceiling_fast_ms=10000.0,
        p90_ceiling_fast_ms=10000.0,
        median_ceiling_fallback_ms=10000.0,
        p90_ceiling_fallback_ms=10000.0,
        bash_floor_ms=1000.0,
        true_floor_ms=1000.0,
        reference_bash="<from jj --version pin / bash --version>",
        calibration=Calibration(session="0217", chip="<lscpu>", bash="<bash --version>", shasum="<shasum --version>", libc="musl"),
    ),
```

The lockstep test would fail with this placeholder present, so Phase 2 runs the
measurement task directly and does **not** run the full `check`/`test` gate; the
placeholder never leaves the working copy.

#### 3. Take the bootstrapping pass

**Command**: on the guest, in the plugin root:

```bash
CARGO_BUILD_TARGET=aarch64-unknown-linux-musl mise run measure:warm-dispatch
```

Read off the emitted `meta/measurements/warm-dispatch-N.json`:

- `analysis.intervals.C1..C4` — the medians/p90s of `G` under each backend.
- `floors_pre.attempts[-1]` — the measured `bash_ms` and `true_ms` floor
  medians (`FLOOR_SAMPLES = 50`).
- `terms.asset_bytes` and `terms.terms["verifier::sha256_hex"].point` — the
  throughput inputs.
- `analysis.ratios.raw_gates` and the C5 interval — the ratio for the transfer
  verdict.

Then `mise run measure:teardown` and delete **only the records this pass wrote**
— capture the filename each run prints (`record written to ...`) and delete those
exact paths and their `-samples.json` sidecars, never a bare
`warm-dispatch-*.json` glob (which would also match the committed darwin records
`warm-dispatch-1..4.json`). Revert the placeholder, then verify as the last
Phase 2 action, so nothing is assumed: `jj diff tasks/measure.py` is empty,
`jj status meta/measurements/` shows no deletions or modifications to tracked
records and no untracked records remain, and the lockstep test
(`-k CriterionConstantsLockstep`) passes.

### Success Criteria

#### Automated Verification

- [ ] `mise` resolves and provisions the pinned toolchain: `command -v mise`.
- [ ] The guest resolves both digest backends:
      `command -v sha256sum && command -v shasum`
- [ ] The musl target and linker are present:
      `rustup target list --installed | grep aarch64-unknown-linux-musl`
- [ ] The bootstrapping session completes and writes a `warm-dispatch-N.json`
      whose `analysis.validity` is `valid`.
- [ ] After teardown and revert, `jj diff tasks/measure.py` is empty,
      `jj status meta/measurements/` shows no tracked-record deletions or
      modifications and no untracked records remain, and the lockstep test passes.

#### Manual Verification

- [ ] The VM is genuinely aarch64 under native virtualisation (not emulated):
      `uname -m` reports `aarch64` and `lscpu` shows the host CPU brand.
- [ ] `cargo test --target aarch64-unknown-linux-musl` actually **runs** the
      `warm_terms` test on the guest (musl static binaries execute natively on
      aarch64 linux); if cargo declines to run the cross target, the built test
      binary is executed directly or a runner is configured.
- [ ] The measured C1–C4 statistics and floor medians are recorded for use in
      Phase 3.

---

## Phase 3: Register the calibrated linux entry

### Overview

Derive the round-number ceilings and floor gates from Phase 2's statistics, land
the finalised `PlatformEntry`, add the six README bullets that match it exactly,
and add a linux-keyed farm-completeness test. This is the sole substantial code
change besides Phase 1 and lands as one atomic, lockstep-clean unit.

### Changes Required

#### 1. Hand-derive the constants

Apply the pre-registration rules to Phase 2's measured statistics:

- **Ceilings (C1–C4)**: the smallest multiple of 10 ms that leaves at least 18%
  headroom over the **linux measured statistic** — median for C1/C3, p90 for
  C2/C4 — exactly the rule in the work item's AC 3. Do **not** try to reproduce
  darwin's 50/60/70/80 as a check: per 0189's cell table those ceilings were
  pre-registered from 0205's higher base figures (C2 from a fast p90 of 46.51 ms
  at +29% headroom, C4 from a predicted ~63.4 ms at +26%), not from applying this
  rule to 0189's validation statistics — over which the rule gives 50/50/70/70.
  Apply the ≥18% rule directly to the linux figures; ≥18% is a floor, so a noisier
  VM host lands higher headroom, which is expected and acceptable per AC 3.
- **Floor gates**: set `bash_floor_ms` and `true_floor_ms` 40–80% above the
  measured floor medians, matching the band darwin's 7.8/1.95 encode over its
  4.449/1.339 (≈75% and ≈46%).

#### 2. Add the finalised entry

**File**: `tasks/measure.py` (`PLATFORM_TABLE`, `:148-199`)

```python
    ("Linux", "aarch64"): PlatformEntry(
        key="linux-arm64",
        path_tools=(...),
        power_probes=(
            ["cat", "/sys/class/power_supply/AC/online"],
            ["head", "-n1", "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"],
            ["grep", "-h", "", "/sys/class/power_supply/BAT0/status"],
        ),
        median_ceiling_fast_ms=<C1>,
        p90_ceiling_fast_ms=<C2>,
        median_ceiling_fallback_ms=<C3>,
        p90_ceiling_fallback_ms=<C4>,
        bash_floor_ms=<bash floor gate>,
        true_floor_ms=<true floor gate>,
        reference_bash="<bash --version first line>",
        calibration=Calibration(
            session="0217",
            chip="<lscpu Model name>",
            bash="<bash --version>",
            shasum="<shasum --version>",
            libc="musl",
        ),
    ),
```

Two constraints on the fields:

- `path_tools` is re-derived, not blindly copied. Because `bin/accelerator` is
  the same script on both OSes, the linux set equals the darwin 28-tool union;
  the linux farm-completeness test below pins that.
- Each `power_probes` entry has a **distinct leading command** (`cat`, `head`,
  `grep`), because `power_state` keys on `probe[0]` and equal keys collapse; a
  unit test (below) pins this distinctness so a future collision fails loudly.
  The probes are **best-effort per architecture key**: device names such as
  `AC`/`BAT0` vary across hosts and a VM often exposes none, so a "no reading"
  result is expected and satisfies AC 7 by naming the probed source. The libc
  identity (`musl`) rides in the new non-matched `Calibration.libc` field and
  `chip` records the parsed CPU brand, so the `calibrated` note holds (AC 2).

#### 3. Add the six README bullets

**File**: `tasks/README.md` (`### Criterion constants`, after `:335`)

```text
- `linux-arm64.median_ceiling_fast_ms` = <C1>
- `linux-arm64.p90_ceiling_fast_ms` = <C2>
- `linux-arm64.median_ceiling_fallback_ms` = <C3>
- `linux-arm64.p90_ceiling_fallback_ms` = <C4>
- `linux-arm64.bash_floor_ms` = <bash floor gate>
- `linux-arm64.true_floor_ms` = <true floor gate>
```

Integer-valued ceilings render without a decimal point (`50`, not `50.0`) to
match the lockstep parser and the darwin bullets. The block's free-text intro (the
non-parsed lines above the bullets) notes the `linux-arm64` ceilings are VM-scoped
and provisional, so they are not read as bare-metal-authoritative alongside
darwin's.

#### 4. Add linux-keyed farm-completeness and probe tests

**File**: `tests/unit/tasks/test_measure.py`

`path_tools` is the union of **both** dispatch variants' spawns, so both darwin
completeness tests need a linux parallel — guarding only the `bin/accelerator`
dispatch side would let a baseline-only tool (e.g. from `vcs-guard.sh`) slip
through. Mirror `test_the_bootstrap_spawns_nothing_the_farm_lacks` (`:2113-2122`)
and `test_the_recovered_baseline_spawns_nothing_the_farm_lacks`, keyed on the
linux entry; assert the floor binaries `bash` and `true` are present (as darwin's
`chmod` test does, since `spawned_executables` returns neither); and pin the
power-probe leaders distinct:

```python
    def test_the_bootstrap_spawns_nothing_the_linux_farm_lacks(self):
        text = (REPO / "bin/accelerator").read_text()
        missing = spawned_executables(text) - set(
            PLATFORM_TABLE[("Linux", "aarch64")].path_tools
        )
        assert not missing

    def test_the_recovered_baseline_spawns_nothing_the_linux_farm_lacks(self):
        missing = baseline_spawned_executables() - set(
            PLATFORM_TABLE[("Linux", "aarch64")].path_tools
        )
        assert not missing

    def test_the_linux_farm_carries_the_floor_binaries(self):
        tools = set(PLATFORM_TABLE[("Linux", "aarch64")].path_tools)
        assert {"bash", "true"} <= tools

    def test_the_linux_power_probes_have_distinct_leaders(self):
        probes = PLATFORM_TABLE[("Linux", "aarch64")].power_probes
        assert len({probe[0] for probe in probes}) == len(probes)
```

Reuse whatever helper the darwin baseline completeness test uses to enumerate the
recovered-baseline spawns rather than duplicating its extraction.

#### 5. Record linux-arm64 in the measure-namespace operator prose

**File**: `tasks/README.md` (`### The measure namespace`, "What a run requires")

The run guide still reads "A quiet darwin-arm64 host". Add a line recording that
`linux-arm64` is now a calibrated, supported measurement platform, with its run
path — a native aarch64 guest, and, **for the throughput term decomposition only**
(the warm-dispatch artefact is still fetched, not built), `musl-tools`,
`CARGO_BUILD_TARGET` and the musl linker env var. State that the linux-arm64
ceilings are **VM-scoped and provisional** pending a bare-metal or CI-hosted
aarch64 confirmation, since the lockstep-parsed constants block cannot itself
carry that caveat. This puts the reproduction knowledge and the scope caveat 0219
needs in the README, not just this soon-archived plan.

### Success Criteria

#### Automated Verification

- [ ] The lockstep test passes both directions:
      `uv run pytest tests/unit/tasks/test_measure.py -k CriterionConstantsLockstep`
- [ ] Both linux farm-completeness tests, the floor-binary assertion, and the
      power-probe distinctness guard pass:
      `uv run pytest tests/unit/tasks/test_measure.py -k linux`
- [ ] The full Python unit suite passes: `mise run test:unit:tasks`
- [ ] The read-only gate exits 0: `mise run check`

#### Manual Verification

- [ ] The ≥ 18% headroom rule is applied to the linux measured statistics
      directly; darwin's 50/60/70/80 are not used as a confirmation (they derive
      from 0205's base figures, not from this rule over 0189's figures).
- [ ] Each derived ceiling leaves ≥ 18% headroom over its measured statistic,
      and is the smallest multiple of 10 ms that does.
- [ ] Each floor gate sits 40–80% above its measured floor median.
- [ ] The README bullet values match the entry fields under the lockstep
      integer-float rendering (the parser float-coerces both sides, so `50`
      renders for `50.0`); the floor gates keep their decimals (e.g. `7.8`).
- [ ] The `### The measure namespace` "What a run requires" prose names
      linux-arm64 as a supported platform with its run path and the VM-scoped /
      provisional caveat (this free-text prose cannot be auto-pinned).

---

## Phase 4: Confirm the calibrated gating run and commit the record

### Overview

Re-run the harness against the finalised entry, confirm a clean calibrated
verdict, derive and check the throughput, state the transfer verdict, and commit
the record.

### Changes Required

#### 1. Take the calibrated gating pass

**Command**: on the guest (the host key resolves `("Linux", "aarch64")`
directly — no `--platform-key`):

```bash
CARGO_BUILD_TARGET=aarch64-unknown-linux-musl mise run measure:warm-dispatch
mise run measure:teardown
```

Confirm on the emitted record:

- `analysis.validity` is `valid` (not branch 5a/5b), and teardown restore +
  verify both pass (`teardown` empty, `closure_verdict` not forced false).
- `provenance.calibration.note` reads `calibrated` — all five fields recorded
  (session, chip, bash, shasum, libc), with `chip` equal to the parsed
  `observed_chip` brand so the exact-match holds.
- `floors_pre.holds` and `floors_post.holds` are true.
- C1–C4 are **branch 1** (measured, not accepted-not-applicable) and no cell
  carries an `accepted_by` reason; `closure_verdict` is true. The branch-7
  acceptance path is reserved for the explicitly Perl-less scenario, so a
  degraded run cannot pass here masquerading as a full six-cell calibration —
  on this guest all six measure.

#### 2. Derive and check the throughput (AC 6)

Compute `asset_bytes / (median_ms × 1000)` in decimal MB/s from
`terms.asset_bytes` and `terms.terms["verifier::sha256_hex"].point`. Confirm it
is ≥ 700 MB/s — well clear of the ~555 MB/s soft band, confirming the ARMv8
SHA-2 hardware path engaged rather than the soft backend.

The Phase 1 throughput guard (Change 7) makes `assemble_terms_report` fail loudly
when `asset_bytes` or the `verifier::sha256_hex` term is missing, so a "built but
not run" cross-target outcome (which the Phase 2 runner config prevents) cannot
silently drop the figure with a passing-looking run.

#### 3. State the transfer verdict (AC 4)

Against 0189's darwin figures — C1 35.531, C2 38.230, C3 51.496, C4 55.291 ms;
C5 1.3260 [1.3236, 1.3279] — state per cell whether the linux figure lands
within 10% (C1–C4) or within 10% of 1.3260 or inside [1.3236, 1.3279] (C5).
Attribute any difference, as narrative context, to the composition-budget term
carrying the largest share (spawn cost and digest backend being the expected
drivers). Record this in the work item's `## Validation Results` section,
alongside the throughput figure and the VM identity from Change 8.

#### 4. Commit the record

Capture the printed filename of **every** run that emits one (including
invalidated runs, which write and print their record before raising). Delete only
the **other** records this session wrote — by their exact printed paths and
`-samples.json` sidecars, never a `warm-dispatch-*.json` glob (which matches the
committed darwin records and the final record itself). Before committing, run
`jj status meta/measurements/` and confirm the only change is the single intended
record + sidecar addition, with no pre-existing tracked record deleted or
modified. Then commit the captured final record and its `samples` sidecar by that
exact filename, never "the latest N", so no placeholder-gated, superseded, or
committed-darwin record is swept in or destroyed.

### Success Criteria

#### Automated Verification

- [ ] The committed record reports `analysis.validity == "valid"`,
      `provenance.calibration.note == "calibrated"`, and
      `closure_verdict == true`.
- [ ] The record's C1–C4 are branch 1 and no cell carries an `accepted_by`
      reason — a full six-cell calibration, not a degraded run.
- [ ] `terms.asset_bytes` and `terms.terms["verifier::sha256_hex"]` are present
      (the Change 7 guard fails loudly if either is missing) and the derived
      throughput is ≥ 700 MB/s.
- [ ] The recorded build triple ends in `-linux-musl` (matching the entry's
      `libc`), so the figure cannot be a mislabelled glibc build.
- [ ] `jj status meta/measurements/` immediately before the commit shows only the
      single intended record + sidecar added, with no pre-existing tracked record
      deleted or modified.
- [ ] Only the single final `warm-dispatch-N.json` and its sidecar are committed
      under `meta/measurements/`; no intermediate or placeholder-gated records
      remain.

#### Manual Verification

- [ ] The power_state field carries a real linux reading, or names the source it
      probed and that it returned no reading — never a bare `unknown`.
- [ ] The transfer verdict is stated per cell against the darwin figures, with
      the dominant term attributed.
- [ ] The throughput clearing the soft band confirms hardware SHA-2, not the
      soft backend.

---

## Testing Strategy

### Unit Tests

- The graceful-degradation behaviour (Phase 1): `classify_cell` acceptance,
  `closure_verdict` over accepted not-applicable cells, `analyse` attaching the
  reason to C3/C4/C6 (including non-gating C6) while leaving C1/C2/C5 unaccepted,
  the fallback-farm consumers (`record_provenance`, `measure_shell_terms`)
  skipping their fallback direction when the backend is absent,
  `build_rig`/`build_farm` and `assert_backends` tolerating an absent fallback
  backend, `sample_blocks` dropping the fallback arm (and keeping it when
  present), `Rig.fallback_available` computed correctly, and `power_state` naming
  a no-reading source.
- The calibration and throughput fixes (Phase 1): `observed_chip` parsing the
  `lscpu -J` brand (darwin `sysctl` branch unchanged), `unconfirmed_calibration_fields`
  ignoring the new `libc` field, and `assemble_terms_report` failing loudly when
  `asset_bytes` or the `verifier::sha256_hex` term is missing.
- The lockstep test, both linux farm-completeness tests, the floor-binary
  assertion, and the power-probe distinctness guard (Phase 3), which self-adjust
  to the new entry but pin the README values, the linux tool set, and the probe
  leaders.
- Existing tests migrated, not merely added to: the missing-`shasum` assertion
  inverted to the no-raise semantics, and the absent-probe `power_state` test
  moved to the source-naming string.

### Integration Tests

- `mise run test:integration:measure` — the n=2 smoke check — still passes with
  the graceful-degradation change (it uses `--engine=git` and asserts no gating
  figure).

### Manual Testing Steps

1. On a scratch container without Perl, confirm a session now records C3/C4/C6
   as branch 7 with an acceptance reason rather than aborting rig setup.
2. On the aarch64 guest, run the calibrated pass and inspect the record's
   validity, calibration note, floors, cells, closure verdict, throughput, and
   power_state.
3. Confirm `mise run measure:teardown` restores and verifies cleanly.

## Performance Considerations

The VM's absolute ceilings are VM-scoped and provisional (VM and container
overhead inflate them relative to bare metal); the work item accepts this, and
0219 may later tighten them from a bare-metal or CI-hosted aarch64 run. The
throughput figure is CPU-bound and valid under native virtualisation because the
crypto HWCAP passes through from the host CPU.

## Migration Notes

The platform-additive changes are behaviour-preserving; the darwin entry, its
constants and its gating are untouched. One behaviour change is deliberate: the
Change 7 throughput guard converts a silent `asset_bytes: null` drop into a loud
precondition failure (fixing a latent bug), which also fires on darwin in that
edge case. Three additive serialisation shifts land on new records, darwin
included: the new `Calibration.libc` field (`recorded.libc: null` on darwin), the
Change 8 `provenance.build` block, and `power_state` naming a probed source
instead of the bare `unknown` sentinel on a no-reading probe. The `observed_chip`
fix adds an `lscpu` parse branch, keeping the darwin `sysctl` return
byte-identical, and the graceful-degradation change is a no-op on any host that
resolves both digest backends.

## Risks and Mitigations

- ⚠️ **Native musl build of the full crate.** The `warm_terms` test links the
  whole `accelerator` crate; building it for `aarch64-unknown-linux-musl`
  natively requires the crate's C dependencies to build musl-static. If the
  native linker struggles, fall back to `cargo-zigbuild` (zig links musl on
  linux too) rather than abandoning the musl figure.
- ⚠️ **`cargo test --target` execution.** Cargo may treat the musl target as
  cross and decline to run the test. Mitigation: musl static aarch64 binaries
  run natively on the guest; if cargo still declines, run the built test binary
  directly.
- ⚠️ **VM exposes no power source.** A guest may expose neither
  `/sys/class/power_supply` batteries nor cpufreq. The Phase 1 `power_state`
  change makes this satisfy AC 7 by naming the probed source and that it
  returned no reading.
- ⚠️ **Placeholder leakage.** The Phase 2 placeholder entry must never be
  committed — it would fail the lockstep test and record wrong constants — and
  neither must the loose-gate record it writes, which reports
  `valid`/`closure_verdict: true` and is caught by no test. Delete the
  intermediate records **by their exact printed filenames** — never a
  `warm-dispatch-*.json` glob, which also matches the committed darwin records
  `warm-dispatch-1..4.json` — and revert the entry before Phase 3. Verify the
  revert (empty `jj diff` on `measure.py`; `jj status meta/measurements/` showing
  no tracked-record deletions or modifications and no untracked records; lockstep
  green) as the last Phase 2 action.

## References

- Original work item: `meta/work/0217-measure-warm-dispatch-on-linux.md`
- Research: `meta/research/codebase/2026-09-20-0217-measure-warm-dispatch-on-linux.md`
- Criterion and darwin figures:
  `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
- Harness design and derivation rules:
  `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`
- `PLATFORM_TABLE` and the entry dataclasses: `tasks/measure.py:148-199`,
  `tasks/shared/measurement.py:777-805`
- Entry refusal, farm build, backend assertion: `tasks/measure.py:1646-1651`,
  `:944-957`, `:2214-2234`
- Throughput term harness: `cli/launcher/tests/warm_terms.rs:103-153`,
  `tasks/measure.py:2637-2705`
- README constants block and lockstep test: `tasks/README.md:302-335`,
  `tests/unit/tasks/test_measure.py:1147-1184`
