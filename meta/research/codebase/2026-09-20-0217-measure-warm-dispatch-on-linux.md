---
type: "codebase-research"
id: "2026-09-20-0217-measure-warm-dispatch-on-linux"
title: "Research: Measuring warm dispatch on linux (work item 0217)"
date: "2026-09-20T20:05:40+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0217"
parent: "work-item:0217"
topic: "Measuring warm dispatch on linux (work item 0217)"
tags: ["research", "codebase", "cli", "launcher", "measurement", "performance", "sha2", "calibration"]
revision: "50a9aa5e4034b0e0c4dde17b03d123f89e944e4b"
repository: "accelerator"
last_updated: "2026-09-20T20:05:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Measuring warm dispatch on linux (work item 0217)

**Date**: 2026-09-20T20:05:40+00:00
**Author**: Toby Clemson
**Git Commit**: 50a9aa5e4034b0e0c4dde17b03d123f89e944e4b
**Branch**: jj working copy `qkostqwxmsyz` (anonymous, no bookmark; main is `main`)
**Repository**: accelerator

## Research Question

What does the codebase require to discharge work item 0217 — run the committed
warm-dispatch harness on a native aarch64 linux guest, add a calibrated
`("Linux", "aarch64")` musl entry to `PLATFORM_TABLE`, derive linux-specific
ceilings and floors, capture the aarch64-musl `verifier::sha256_hex` throughput
under the `sha2` 0.11 hardware backend, and keep the README constants block and
its lockstep test in sync? Where is each mechanism, and what will bite the
operator?

## Summary

0217 is a bounded measurement task, not a harness re-authoring. The harness is
committed across two files — the driver `tasks/measure.py` and the pure core
`tasks/shared/measurement.py` — and its per-platform gate numbers are
**pre-registered constants stored as data**, not values computed at run time.
Adding a platform is therefore: run the harness on the target host, read the
measured statistics off the record, hand-derive the round-number ceilings and
floor gates from the plan's rules, and enter them as a new `PlatformEntry`
alongside four calibration-provenance fields. The README block and a
bidirectional lockstep test then bind those numbers.

Five findings will shape the implementation, three of them footguns:

- ⚠️ **An uncalibrated linux host writes no record.** `warm_dispatch` prints an
  advisory promising figures "recorded as uncalibrated context (branch 7)", but
  `run_session` then raises `PreconditionFailureError` when `entry is None` and
  writes nothing (`tasks/measure.py:1646-1651`). You cannot do an exploratory
  "context-only" linux run first — the entry must exist before any record is
  produced, so the ceilings are seeded from an entry whose numbers you refine
  after the fact, or via `--platform-key` against a placeholder.
- ⚠️ **`power_state` collapses probes that share a leading token.** It keys the
  record on `probe[0]` (`tasks/shared/measurement.py:698-713`); the three darwin
  `pmset` probes already collapse to one key. Linux probes that reuse `cat` over
  several `/sys` files will keep only the last — each probe needs a distinct
  leading command.
- ⚠️ **The Rust throughput harness is a hand-maintained replica.** `warm_terms.rs`
  copies the private `reverify` method (`cli/launcher/.../mod.rs:91-110`); a
  refactor of `reverify` silently invalidates the figure
  (`cli/launcher/tests/warm_terms.rs:11-13`).
- **Ceilings and floors are constants, not derivations.** The 18%-headroom and
  40–80%-floor-band rules live only in the plan prose; the code stores the
  resulting round numbers and gates with plain `<=` comparisons. You compute the
  numbers by hand.
- **The fallback farm needs Perl `shasum`.** `shasum` is a Perl program and is
  the sole `FALLBACK_BACKEND`; a minimal musl image with coreutils `sha256sum`
  but no Perl cannot build the fallback farm, so C3/C4/C6 record branch 7.

The `sha2` 0.11 hardware backend (shipped by 0216) is confirmed in
`cli/Cargo.lock` with no feature flags — runtime detection is internal to
`sha2` + `cpufeatures 0.3.0`. The `asset_bytes` persistence the throughput
figure needs is already wired. Whether the darwin result transfers is genuinely
open; the harness gives you the instrument, not the answer.

## Detailed Findings

### The harness: two files, driver and core

The harness splits cleanly. `tasks/measure.py` (~2837 lines) is the driver —
process, clock, filesystem, session lifecycle, sampling rig, farm construction,
and the `PLATFORM_TABLE` data. `tasks/shared/measurement.py` (~1180 lines) is the
pure analysis core — the `Branch`/`Validity`/`CellKind` enums, `classify`,
`resolve_platform_key`, `resolve_cpu_count`, `power_state`, the `Calibration`
and `PlatformEntry` dataclasses, and `unconfirmed_calibration_fields`. The
driver imports the core at `tasks/measure.py:32-80`.

Task wiring: `mise.toml:352-366` maps `measure:warm-dispatch` →
`invoke measure.warm-dispatch`, `measure:teardown` → `measure.teardown`, and
`test:integration:measure` → `measure.smoke-check`. The CI job
`check-measure-harness` (`.github/workflows/main.yml:~373-415`) runs only the
n=2 smoke check with `--engine=git`; it never runs a real measurement and never
checks the README block.

### PLATFORM_TABLE and calibration provenance

`PLATFORM_TABLE: dict[tuple[str, str], PlatformEntry]` is defined at
`tasks/measure.py:148-199` with a single `("Darwin", "arm64")` entry. The key is
the raw `(platform.system(), platform.machine())` pair — confirmed at
`tasks/shared/measurement.py:885`. `PlatformEntry` (frozen dataclass,
`tasks/shared/measurement.py:793-805`) carries eleven fields: `key`,
`path_tools`, `power_probes`, the four ceilings, `bash_floor_ms`,
`true_floor_ms`, `reference_bash`, `calibration`.

⚠️ **The dotted README/constants prefix is `entry.key`, not the tuple.** The
darwin entry sets `key="darwin-arm64"` (`tasks/measure.py:150`). Per
`UNAME_TO_ALIAS` in `tasks/shared/targets.py:24-25`, `("linux", "aarch64")` maps
to the alias `linux-arm64`, and `host_platform()`/`entry_platform()` use that
alias to predict cache-entry names in `warm_cache_gaps`. The linux entry's `key`
must be `"linux-arm64"` or the predicted names diverge from what the launcher
writes.

The `Calibration` dataclass (`tasks/shared/measurement.py:777-790`) has four
fields: `session`, `chip`, `bash`, `shasum`. The shipped darwin block
(`tasks/measure.py:141-146`) records `session="0205"`, `chip="Apple M4 Max"`,
and leaves `bash=None` and `shasum=None` because the 0205 session never recorded
them. This is the exact gap 0217 must close for linux: **all four fields**, with
the `musl` libc identity and SHA-extension fact carried as free text inside
`chip`/`shasum` (the open question resolved 2026-09-15 for the free-text route,
`0217:163-167`).

### Ceilings and floors: pre-registered constants, hand-derived

⚠️ The "smallest multiple of 10 ms leaving ≥ 18% headroom" rule (C1–C4) and the
"40–80% above the measured floor" rule (the two floor gates) are **not in the
code**. They are the pre-registration rationale in the plan
(`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md:70-96` and
`:1769-1784`). The code holds only the resulting round numbers and applies them
as thresholds. The operator computes the numbers.

The rule checks out against the darwin numbers. Ceilings:

| Cell | Statistic | Base (ms) | ×1.18 | Ceiling | Headroom |
| --- | --- | --- | --- | --- | --- |
| C1 | median, fast | 42.28 | 49.9 | 50 | +18.3% |
| C2 | p90, fast | 46.51 | 54.9 | 60 | +29.0% |
| C3 | median, fallback | ~59.2 | 69.9 | 70 | +18.2% |
| C4 | p90, fallback | ~63.4 | 74.8 | 80 | +26.2% |

C2 lands on 60 not 50 because 50 would give only +7.5%, below the 18% floor.
Floor gates sit 46–75% above the valid session's measured floors (4.449 ms bash,
1.339 ms `true`): `7.8 / 4.449 = +75%`, `1.95 / 1.339 = +46%`.

Where the numbers act:
- `cells_for(entry)` (`tasks/measure.py:812-863`) assigns each absolute cell its
  ceiling from the entry: C1←`median_ceiling_fast_ms` (819), C2←`p90_ceiling_fast_ms`
  (827), C3←`median_ceiling_fallback_ms` (835), C4←`p90_ceiling_fallback_ms` (843).
- Acceptance is the bootstrap interval's **upper bound ≤ ceiling** in `classify()`
  (`tasks/shared/measurement.py:558-570`).
- Floors are measured by `measure_floors()` (`tasks/measure.py:2594-2626`,
  `FLOOR_SAMPLES=50`, returns the median of each), gated by `gate_floors()`
  (`tasks/measure.py:1590-1619`, plain `<=`, retry cap `FLOOR_RETRY_CAP=3`),
  taken pre and post sampling (`tasks/measure.py:1652`, `:1695`). A pre breach on
  a non-rehearsal run raises "the host is not quiet (branch 5a)"
  (`tasks/measure.py:1654-1658`).

### The criterion: G, B, the six cells, the seven branches

0217 inherits 0189's criterion **unchanged** (`0217:44-45`). 0189
(`meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`, status done)
is authoritative for the text; `PLATFORM_TABLE` is authoritative for the numbers.

`G` = the full warm dispatch of the Rust guard
(`bin/accelerator vcs guard --format=hook --fail-safe`), cache warm. `B` = the
deleted shell baseline `hooks/vcs-guard.sh`, recovered from git — an artefact no
CI lane can reproduce, which is why the ratio cells are demoted beneath the
absolute budget. `G` and `B` do not do comparable work: `B` is a bash
directory-walk, `G` loads the repository through jj-lib behind a verified
signature chain. **Fast** backend means `sha256sum` resolves; **fallback** means
only Perl `shasum -a 256` does.

The six cells (`cells_for`, `tasks/measure.py:812-863`):

| Cell | Statistic | Backend | Ceiling | Gates | Darwin result (ms) | Branch |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | median(G) | fast | ≤ 50 | yes | 35.531 [35.467, 35.584] | 1 |
| C2 | p90(G) | fast | ≤ 60 | yes | 38.230 [37.979, 38.427] | 1 |
| C3 | median(G) | fallback | ≤ 70 | yes | 51.496 [51.411, 51.616] | 1 |
| C4 | p90(G) | fallback | ≤ 80 | yes | 55.291 [54.889, 55.666] | 1 |
| C5 | median(G)/median(B) | fast | ≤ 1.4 | yes | 1.3260 [1.3236, 1.3279] | 1 |
| C6 | median(G)/median(B) | fallback | recorded | no | 1.9218 [1.9172, 1.9266] | 2, ungated |

C1–C4 use an unpaired percentile bootstrap; C5 uses a paired bootstrap on the
ratio of medians over interleaved pairs, ≥ 10,000 resamples, seeded. C5 has a
two-part test — the raw-median interval upper bound gates, the `true`-floor-
subtracted point estimate is the robustness check. The darwin figures are the
transfer target for 0217 AC 4: C1–C4 within 10% of each figure above, C5 within
10% of 1.3260 or inside `[1.3236, 1.3279]`.

The branch taxonomy is the `Branch` StrEnum (`tasks/shared/measurement.py:502-512`):

| Branch | Label | Meaning | Driver reference |
| --- | --- | --- | --- |
| 1 | PASS | `upper ≤ ceiling` (+ C5 robustness) | classifier |
| 2 | FAIL | `lower > ceiling` | classifier |
| 3 | INDETERMINATE | interval straddles; escalate once | classifier |
| 4 | TERMINAL | escalation spent, still undecided | classifier |
| 5a | INVALID_PRE | floor breach / precondition | `measure.py:1657` |
| 5b | INVALID_POST | drift or teardown-verify failure | `measure.py:1712-1737` |
| 6a | INFEASIBLE | no n fits the wall-clock budget | sizing |
| 6b | BUDGET | 35-min budget exhausted mid-run | `measure.py:2296` |
| 7 | NOT_APPLICABLE | no entry, or no fallback farm | `measure.py:1648`, `:2230-2234` |

Selection is an ordered cascade, first match wins (`classify`,
`tasks/shared/measurement.py:547-570`). For 0217 the operative branches are 7
(no calibrated entry, or no Perl → C3/C4/C6 unconstructible) and 5
(invalidation on drift or artefact tampering). `closure_verdict`
(`tasks/shared/measurement.py:581-595`) holds iff every gating cell is PASS, or
NOT_APPLICABLE with a recorded `accepted_by`.

### Unknown-key handling and the calibration honesty note

Two layers. `platform_constants(key, table)`
(`tasks/shared/measurement.py:808-815`) returns `table.get(key)` — `None` when
absent. `warm_dispatch` prints a branch-7 advisory (`tasks/measure.py:1064-1070`)
but ⚠️ `run_session` then **raises before any record is written**
(`tasks/measure.py:1646-1651`); the exception propagates through the session's
teardown and no `warm-dispatch-N.json` is produced. The advisory is misleading —
recording uncalibrated context in practice requires a matching entry or
`--platform-key`.

Calibration is a note, not a gate. `unconfirmed_calibration_fields()`
(`tasks/shared/measurement.py:839-869`) treats a recorded `None` field as
unconfirmable, so darwin's `bash=None`/`shasum=None` always read "unconfirmed"
even on the calibrating chip. `calibration_note()` (`tasks/measure.py:2575-2584`)
returns `"calibrated"` only when nothing is unconfirmed. Crucially, `analyse()`
computes validity **solely from drift** (`tasks/measure.py:2150-2154`) and the
classifier takes no calibration argument — a full verdict is produced regardless,
with the calibration result riding along as `provenance["calibration"]`. A linux
entry recording real `bash`/`shasum` versions and run on the matching chip earns
the "calibrated" note that 0217 AC 2 demands.

### The verifier and the sha2 0.11 backend

`verifier::sha256_hex` (`cli/launcher/src/launch/outbound/resolve/verifier.rs:12-21`)
is a one-shot `Sha256::digest(bytes)` plus lowercase hex encode. It is called by
`verify_binary` (`verifier.rs:29-50`) — the corruption gate before the minisign
check — reached on both the warm cache-hit re-verify (`reverify`,
`mod.rs:91-110`, invoked at `mod.rs:199` on every cache hit) and the cold fetch
path (`fetch_verify_store`, `mod.rs:138-178`).

The `sha2` backend is confirmed:
- `cli/Cargo.toml:76` pins `sha2 = "0.11"` — a bare caret, **no feature flags,
  no `asm`, no `force-soft`**. `cli/Cargo.lock` resolves `sha2 0.11.0` with
  `cpufeatures 0.3.0` and `digest 0.11.3`.
- There is **no `sha2-asm` crate** and no first-party detection code anywhere in
  `cli/**`. Backend selection is entirely internal to `sha2` + `cpufeatures`: on
  aarch64 the `cpufeatures` probe checks the ARMv8 SHA-2 extension (via
  `getauxval`/HWCAP on Linux, `sysctl` on Darwin) and dispatches to intrinsics,
  else the soft path.
- `cli/deny.toml:121-127` skips a duplicate `cpufeatures@0.2.17` (the RustCrypto
  0.10/0.11 straddle); the musl triples 0217 cares about are declared at
  `cli/deny.toml:12-14` and `cli/about.toml:37-39`.

⚠️ Darwin resolves the feature via `sysctl`, not the `getauxval`/HWCAP path the
`crt-static` musl binary uses — which is exactly why 0216 deferred conclusive
aarch64-musl runtime selection to 0217. A throughput in the ~555 MB/s soft band
on linux would fire 0216's open "silent soft-backend-selection" trigger.

### Throughput recording and the asset_bytes prerequisite

There is no Rust criterion bench. The figure is a test-harness term.
`warm_terms.rs` (an `#[ignore]` operator-run test, `cli/launcher/tests/warm_terms.rs:103-153`)
reads the cached asset via `ACCELERATOR_MEASURE_CACHE_ROOT`, times
`verifier::sha256_hex` over `SAMPLES=200`, and emits latency-only JSON plus a
trailing `{"asset_bytes": <len>}` line. MB/s is derived downstream in Python.

`tasks/measure.py` runs that test via `decompose_terms()`
(`tasks/measure.py:2637-2676`, `cargo test --release --test warm_terms -- --ignored`),
parses `asset_bytes` (`parse_asset_bytes`, `:2679-2689`) and the per-term
intervals (`parse_term_report`, `:2692-2705`), and persists `asset_bytes`
alongside `cache_root_bytes` (`assemble_terms_report`, `:1858-1881`). The
throughput is a **documented derivation**, not a stored figure:
`asset_bytes / (median_ms × 1000)` decimal MB/s (README `:291-300`,
`parse_asset_bytes` docstring `:2682`). 0217 AC 6 wants this ≥ 700 MB/s —
deliberately loose against the ~555 soft band; darwin's hardware figure is
~2944 MB/s for comparison (see Historical Context). The `asset_bytes`
persistence was shipped by 0216 as the prerequisite 0217 inherits.

### The launcher: tool detection, digest backends, warm dispatch

`bin/accelerator` (bash 3.2 floor) hard-fails without a fetcher at
`bin/accelerator:159-160` ("need curl or wget on PATH"), unless the
`ACCELERATOR_BOOTSTRAP_DOWNLOADER` seam is set. The digest backend is
detection-based (`sha256_files`, `bin/accelerator:274-280`): `sha256sum` else
`shasum -a 256`.

The required tool set is **mechanically derived, not transcribed**.
`spawned_executables` (`tasks/shared/measurement.py:1162-1179`) regex-scans the
shell source for command-position tokens and `command -v` lookups, intersects a
closed vocabulary, and excludes shell functions. The darwin `path_tools` tuple
(`tasks/measure.py:155-184`, 28 tools) is asserted against that derivation by
`test_the_bootstrap_spawns_nothing_the_farm_lacks`
(`tests/unit/tasks/test_measure.py:2113-2134`) — the check that caught the
historical `chmod` omission.

⚠️ That completeness test is keyed only on the darwin entry. A linux
`path_tools` set will be **unverified** unless a parallel test keyed on the
Linux entry is added.

The two digest "farms" mirror the launcher: `build_farm(..., include_fast_backend)`
(`tasks/measure.py:932-958`) symlinks tools and skips `sha256sum` when building
the fallback farm, forcing `sha256_files` down the `shasum` branch. `assert_backends`
(`tasks/measure.py:2214-2234`) asserts both directions and raises the branch-7
"no shasum → C3/C4/C6 not applicable" error when Perl is absent. This is the
direct linux risk: minimal musl images ship coreutils `sha256sum` but often no
Perl.

"Warm dispatch" means the cache is populated so nothing is fetched — the launcher
and sub-binary are read and exec'd straight from the cache (`bin/accelerator:433-455`).
The summed warm-path terms are listed at `tasks/measure.py:1847-1855`. The
once-per-dispatch cache-root probe (0189's guarantee) has a Rust layer:
`cache_root::candidate` does selection with no filesystem access
(`cli/launcher/.../cache_root.rs:56-72`), and `verify_writable`
(`cache_root.rs:101-113`) — the actual write+exec probe — is called only on the
write path (`mod.rs:142`), so a warm hit never pays it.

### power_state probing

Darwin declares three `pmset` probes on the entry (`tasks/measure.py:185-189`),
read by `power_state()` (`tasks/shared/measurement.py:698-713`), which records
`"unknown"` on `FileNotFoundError`/`PermissionError`. The mechanism is
OS-neutral; only the per-entry `power_probes` tuple changes.

Two gotchas for the linux port:
- ⚠️ **Key collapse.** `state` is keyed on `probe[0]`; the three `pmset` probes
  already collapse to a single `"pmset"` key. Linux probes reusing `cat` over
  several `/sys` files collapse the same way — give each a distinct leading
  command, or record only the last.
- **The plan names the intended probes** (`plan:1785-1790`):
  `cpufreq/scaling_governor`, `intel_pstate/no_turbo`, and
  `power_supply/*/status`. AC 7 requires a real reading from a named source such
  as `/sys/class/power_supply`, or a record of the source probed and that it
  returned nothing — never a bare `unknown`.

`observed_chip()` (`tasks/measure.py:2490-2507`) already probes both OSes —
`sysctl` then `lscpu -J` — so the `chip` provenance field needs no new code on
linux.

### README ↔ criterion_constants ↔ lockstep test

The numeric source of truth is the module constants plus `PLATFORM_TABLE`.
`criterion_constants()` (`tasks/measure.py:213-250`) projects them into a flat
`dict[str, float]`: 19 scalar gate constants plus, per platform, six dotted keys
prefixed by `entry.key` (`.median_ceiling_fast_ms`, `.p90_ceiling_fast_ms`,
`.median_ceiling_fallback_ms`, `.p90_ceiling_fallback_ms`, `.bash_floor_ms`,
`.true_floor_ms`). **There is no README generator** — the `### Criterion constants`
block (`tasks/README.md:302-335`) is hand-authored and only validated.

`TestCriterionConstantsLockstep` (`tests/unit/tasks/test_measure.py:1147-1184`)
parses the README block with `^- \`([^\`]+)\` = (-?[\d.]+)$` and asserts
bidirectional set-equality of keys plus value equality via `pytest.approx`, and
that at least one per-platform-prefixed key exists. Any drift — a new constant,
a changed number, a renamed key, a broken bullet format, or removing the
terminating `### ` heading — fails an assertion. `TestMeasureNamespaceDocs`
(`:1187-1203`) separately guards the prose prerequisites.

Adding `("Linux", "aarch64")` therefore touches, in one change:
1. `PLATFORM_TABLE` (`tasks/measure.py:148-199`) — a new `PlatformEntry` with
   `key="linux-arm64"`, all fields populated, and a four-field `Calibration`.
2. The README block (`tasks/README.md:335`) — exactly six new bullets, one per
   derived field, values matching the entry exactly (integer floats render
   without a decimal point).
3. Optionally, parallel tests keyed on the Linux entry — the four lockstep
   assertions self-adjust, but the farm-tool completeness guarantee for the
   Linux `path_tools` is otherwise unverified.

## Code References

- `tasks/measure.py:148-199` — `PLATFORM_TABLE`; the single darwin entry.
- `tasks/measure.py:141-146` — `_DARWIN_CALIBRATION` with `bash=None`, `shasum=None`.
- `tasks/measure.py:213-250` — `criterion_constants()` projection.
- `tasks/measure.py:812-863` — `cells_for()`; C1–C6 construction and gating flags.
- `tasks/measure.py:1646-1651` — `run_session` refuses an absent entry before writing.
- `tasks/measure.py:1590-1619`, `:2594-2626` — floor gate and floor measurement.
- `tasks/measure.py:2214-2234` — `assert_backends`; branch-7 on missing `shasum`.
- `tasks/measure.py:2637-2705` — `decompose_terms`/`parse_asset_bytes`/`parse_term_report`.
- `tasks/shared/measurement.py:502-570` — `Branch` enum and `classify` cascade.
- `tasks/shared/measurement.py:698-713` — `power_state`; keys on `probe[0]`.
- `tasks/shared/measurement.py:777-805` — `Calibration` and `PlatformEntry` dataclasses.
- `tasks/shared/measurement.py:839-869` — `unconfirmed_calibration_fields`.
- `tasks/shared/measurement.py:872-894` — `resolve_platform_key` and `--platform-key` precedence.
- `tasks/shared/targets.py:24-25` — `UNAME_TO_ALIAS`; `("linux","aarch64") → "linux-arm64"`.
- `cli/launcher/src/launch/outbound/resolve/verifier.rs:12-50` — `sha256_hex`, `verify_binary`.
- `cli/launcher/src/launch/outbound/resolve/mod.rs:91-178` — `reverify`, `fetch_verify_store`.
- `cli/launcher/tests/warm_terms.rs:103-153` — the throughput/term harness (replica of `reverify`).
- `cli/Cargo.toml:76`, `cli/Cargo.lock` — `sha2 = "0.11"` / resolved `sha2 0.11.0` + `cpufeatures 0.3.0`.
- `bin/accelerator:146-161`, `:274-280` — fetcher hard-fail and `sha256_files` backend.
- `tasks/README.md:302-335` — the `### Criterion constants` block.
- `tests/unit/tasks/test_measure.py:1147-1184`, `:2113-2134` — lockstep and farm-completeness tests.

## Architecture Insights

- **Gate numbers are data, derivation is documentation.** The portability
  decision (plan `:1262-1289`) was to make the environment pin and every gate
  constant per-platform data keyed on `(system, machine)`, so the linux hand-off
  runs a task rather than re-authoring a script. The 18%/40–80% rules never
  entered the code; the operator applies them and enters round numbers.
- **Calibration is honesty, not enforcement.** Provenance rides alongside the
  verdict as a note; the gate depends only on drift and interval position. This
  is deliberate (harness deep-dive; `calibration_holds` exists but is
  test-only). 0217 earns the "calibrated" note by recording all four fields on
  the matching chip, not by satisfying a gate.
- **One harness, two OSes, additive probes.** Quietness is recorded portably —
  `os.getloadavg()` for load, cgroup-v2 `cpu.max` for CPU count
  (`resolve_cpu_count`, `tasks/shared/measurement.py:680-695`) — and power probes
  are additive with `"unknown"` on absence, so the same harness runs on darwin
  and linux with only a new entry.
- **The absolute budget outranks the ratio by construction.** C1–C4 are the
  only re-runnable cells because `B` is a deleted artefact; C5/C6 are historical.
  This is why 0217 derives fresh linux ceilings rather than importing darwin's,
  and why 0219 will re-run C1–C4 on a schedule.

## Historical Context

- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — authoritative
  for the criterion text (G, B, C1–C6, seven branches) and the darwin validation
  results; inherited unchanged by 0217. Status done.
- `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` — the harness
  design: PLATFORM_TABLE as data (`:1262-1289`), ceiling/floor derivation
  (`:70-96`, `:1769-1784`), portable quietness (`:1754-1767`), and the named
  linux power probes (`:1785-1790`).
- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — established
  non-transferability: the sha256-vs-BLAKE2b inversion (`sha256_hex` 4.4895 ms →
  555 MB/s software; BLAKE2b 1.7184 ms → 1451 MB/s) is a property of that chip
  and crate build. "Per-platform figures must be recorded against (architecture,
  SHA-extension support, libc)." Status done.
- `meta/work/0216-close-the-sha2-hardware-intrinsics-gap.md` — shipped `sha2`
  0.11 across four targets, measured darwin-arm64 before/after, and deferred the
  aarch64-musl throughput to 0217 as an explicit obligation (reciprocal edge,
  SHA-extension-keyed criterion, `asset_bytes` prerequisite). Status done.
- `meta/measurements/2026-09-11-0216-sha256-hex-{before,after}.json` — darwin
  before 4.0791 ms → 611.3 MB/s (soft); after 0.8470 ms → 2944.4 MB/s (hardware).
- `meta/measurements/warm-dispatch-{1,2,3,4}.json` — the darwin sessions;
  `warm-dispatch-3.json` is 0189's valid record (attempts 1–2 invalidated on
  drift).
- `meta/work/0219-own-the-recurring-absolute-budget-check.md` — the recurring
  lane that consumes 0217's calibrated linux entry as its calibration source
  (0217 `blocks` 0219). Status draft.
- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` — parent epic; the
  release cut 0217 depends on is owned by "whoever performs epic-0136 releases"
  (0165 owns the release pipeline).
- `meta/reviews/work/0217-measure-warm-dispatch-on-linux-review-1.md` — an
  existing review of the work item (not deep-read here).
- `meta/decisions/ADR-0061-signed-content-addressed-tree-generations.md` — the
  only ADR referencing warm dispatch; no ADR covers measurement methodology,
  calibration, or the sha2 backend choice.

## Related Research

- `meta/research/codebase/2026-09-10-0216-close-the-sha2-hardware-intrinsics-gap.md`
- `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
- `meta/research/codebase/2026-08-22-0191-batch-shim-hashes.md`
- `meta/research/codebase/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`

## Open Questions

- ❓ **Seeding the first linux record.** Because `run_session` raises on an
  absent entry, the operator cannot capture a context-only run to read the
  statistics from. The likely path is a placeholder entry (loose ceilings) run
  once, then the numbers refined from that record before the entry is finalised —
  but the intended workflow is not documented. Confirm whether `--platform-key`
  against the darwin entry, or a provisional linux entry, is the sanctioned
  bootstrap.
- ❓ **Perl availability on the chosen guest.** Whether C3/C4/C6 are measured or
  recorded branch 7 depends entirely on whether the aarch64 musl guest ships
  Perl `shasum`. Decide the guest image before the run; a Perl-less image
  discharges only C1/C2/C5.
- ❓ **Linux `path_tools` verification.** The farm-completeness test only covers
  darwin. Decide whether to add a parallel test keyed on the Linux entry, or
  accept the Linux tool set as unverified.
- ❓ **Does the darwin result transfer?** Genuinely open (0217 Context). Two
  effects push opposite ways — cheaper linux spawns lower `G`; universal
  `sha256sum` making the fast backend the norm lowers `B` proportionally less, so
  the ratio may rise. The harness measures it; it does not predict it.
