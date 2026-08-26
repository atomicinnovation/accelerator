---
type: plan
id: "2026-08-22-0191-batch-shim-hashes"
title: "Batch the bootstrap's two shim hashes into one sha256 invocation Implementation Plan"
date: "2026-08-22T23:01:48+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0191"
parent: "work-item:0191"
derived_from: ["codebase-research:2026-08-22-0191-batch-shim-hashes"]
relates_to: ["work-item:0186", "work-item:0189", "work-item:0205", "work-item:0169"]
tags: [shell, performance, bootstrap, bash-3.2, sha256]
revision: "05965018b8090c6c8da5220313f2226f103145cc"
repository: "accelerator"
last_updated: "2026-08-26T23:03:02+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Batch the bootstrap's two shim hashes into one sha256 invocation Implementation Plan

## Overview

Collapse the bootstrap's two per-invocation `sha256_file` calls in
`bin/accelerator` into a single backend fork with zero `awk`, saving a measured
~2.48 ms on the warm path on the fast backend. The staged shim's path embeds the
source digest, so the source cannot be hashed first without forcing a second
fork; the change therefore discovers the staged candidate by **glob** (path
independent of the digest), then hashes source plus candidate in one
`sha256sum f1 f2` invocation and verifies both the name-suffix and the bytes
against the fresh source digest. The trust check is preserved exactly — the three
planted-stub tests stay green unmodified.

**The candidate glob is constrained to a strict 64-character lowercase-hex
suffix.** The batched output is parsed by keying on the path column, and a
candidate filename that could contain a newline, space, or backslash would let a
crafted name inject a forged output line that the parse then trusts by name
(backends other than GNU coreutils do not escape such filenames). Admitting only
`accelerator-verify-${platform}-<64-hex>` names structurally excludes every
injectable character, so no filename can forge a line, and the parse over the
path column is safe. The constraint also excludes `.staging.$$` temps from the
hashed set as a side effect, since they are not bare hex.

## Current State Analysis

`sha256_file` is inlined at `bin/accelerator:273-279` (the root-of-trust entry
point sources nothing) and is a two-fork helper — the backend plus an `awk` to
strip the path column:

```bash
sha256_file() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{print $1}'
	else
		shasum -a 256 "$1" | awk '{print $1}'
	fi
}
```

The staging condition calls it twice (`bin/accelerator:292-312`):

```bash
shim_digest=$(sha256_file "${shim_source}") ||
	fail_integrity "could not hash the verify shim source: ${shim_source}"
shim="${cache_dir}/accelerator-verify-${platform}-${shim_digest}"
if [[ ! -x "${shim}" ]] ||
	[[ "$(sha256_file "${shim}" 2>/dev/null)" != "${shim_digest}" ]]; then
	require_exec_capable_cache
	shim_tmp="${shim}.staging.$$"
	cp "${shim_source}" "${shim_tmp}" 2>/dev/null || fail_integrity …
	chmod +x "${shim_tmp}" 2>/dev/null || fail_integrity …
	mv -f "${shim_tmp}" "${shim}" 2>/dev/null || fail_integrity …
fi
```

The warm path is therefore **two backend forks plus two `awk` forks**: source at
line 292 (always), staged at line 296 (only when `[[ ! -x "${shim}" ]]` is
false).

**The data dependency that shapes the design.** `shim`'s path is
`f(shim_digest)` — its name literally contains the source digest — so its path
is unknowable until the source fork completes. One backend fork on the warm path
(AC-1) is unreachable while the source is hashed first. The staged candidate must
be found by a means independent of the digest, which is a glob.

### Key Discoveries

- **The two call sites and the short-circuit** — `bin/accelerator:292`
  (source, always) and `bin/accelerator:296` (staged, guarded behind
  `[[ ! -x "${shim}" ]] ||`).
- **Content-addressing is the trust mechanism** — the staged shim is named for
  the source digest and re-hashed every warm start; the digest comparison is a
  security boundary, proven by `test_planted_staged_shim_is_not_trusted` and
  `test_planted_staged_shim_via_cache_dir_is_not_trusted`
  (`tests/integration/entrypoint/test_accelerator_entrypoint.py:740,761`).
- **`sha256_file` has exactly two callers** (verified by grep) — line 292 and
  line 296, plus the definition. Nothing else depends on it, so it can be
  replaced by a variadic form.
- **Both backends emit identical multi-file output** — Apple `/sbin/sha256sum`
  and Perl `/usr/bin/shasum -a 256` both print `<hex>␣␣<path>` per input in
  argument order, exit 0 (verified this session on darwin-arm64). GNU coreutils
  is the same standard format, confirmed by the linux CI lane.
- ⚠️ **Empty-array-under-`set -u` gotcha** — `"${cands[@]}"` on an empty indexed
  array throws `unbound variable` on bash 3.2 (verified `3.2.57`). The glob loop
  must branch on `${#cands[@]}`, never expand blindly.
- **The trace seam** — `run_bootstrap(..., xtrace=True)` runs one call under
  `bash -x` with `PS4='+${FUNCNAME[0]:-main}:'`
  (`tests/integration/support/installation.py:315-317`). No sha256 fork-count
  test exists yet; the exec-probe tests at
  `test_accelerator_entrypoint.py:1319,1336` are the pattern to follow.

## Desired End State

The bootstrap forks the sha256 backend **once** and forks **no `awk`** on both
the warm and cold paths. The staged shim is discovered by glob, hashed together
with the source in one invocation, and trusted only when a candidate exists whose
path is `accelerator-verify-${platform}-${source_digest}` and whose bytes hash to
that same digest. Verify by: the new fork-count test passes; the three
planted-stub tests pass unmodified; a forced-`shasum`-fallback test passes; and
`mise run` exits 0.

## What We're NOT Doing

- **Not** changing the content-addressed naming scheme — staged files are still
  written as `accelerator-verify-${platform}-${digest}`, preserving the
  concurrent-reader safety a unique name gives.
- **Not** cleaning up stale staged shims from prior source versions — the glob
  may return several; the change hashes the hex-named ones and trusts only the
  one matching the current source digest, exactly as today leaves old files in
  place. Active unlinking is out of scope (it adds a race on a concurrently
  accessed root-of-trust path); the input set stays bounded because the hex
  filter excludes orphaned `.staging.$$` temps and release churn bounds the rest.
- **Not** removing the cache-hit sha256 (0215) or cheapening the digest at
  source (0216) — sequenced independently per the work item.
- **Not** gating on reaching a warm-dispatch ratio of 1.3 — this item's case is
  the millisecond saving; the re-measurement (AC-7) is the deliverable, the
  ratio target is an opportunity for 0189, not a pass condition here.
- **Not** adding a `sha256_file`-parity comparison in production code — the
  planted-stub tests already pin correctness against a Python `hashlib` oracle.

## Implementation Approach

Replace `sha256_file` (singular) with a variadic `sha256_files` that runs one
backend call over all its arguments and echoes the raw `<digest>␣␣<path>` lines
(no `awk`). At the call site, glob the cache dir for candidates for this platform,
admitting only a regular executable file whose name-suffix is a strict
64-character lowercase-hex digest; run one `sha256_files` call over the source
plus the candidates; parse the source digest from the output line whose path
column equals `${shim_source}`, compute the expected staged name, and trust the
candidate at that path whose digest equals the source digest. Otherwise re-stage
through the existing per-process-temp `cp`/`chmod`/`mv` path unchanged.

The batched call's exit status is **not** treated as fatal. All three backends
exit non-zero if any single input fails, so a candidate deleted between the glob
and the hash (a concurrent bootstrap's `mv`) would otherwise abort a healthy warm
start. The output is captured unconditionally with stderr suppressed. **Both**
the source and the staged digest are keyed to their path column, never to output
position: a partial-failure call can drop the source line while still printing
candidate lines, so reading the source from line 1 by position could promote a
candidate's digest to the trust anchor. Keying the source on `${shim_source}`
makes the `[[ -n "${shim_digest}" ]]` guard abort precisely when the source line
is absent — a genuinely unhashable source — while a vanished candidate falls
through to re-stage, matching today's graceful behaviour. The one-input-missing
parse concern (AC-4) is discharged by this path-keying and asserting the batched
source digest equals its standalone `hashlib` value.

Phase 1 is the whole functional change and is independently mergeable — its tests
prove correctness on their own. Phase 2 records the before/after measurements and
is a records-only commit, mergeable separately once Phase 1 has landed.

## Phase 1: Glob-discovered batched digest

### Overview

Replace the two single-file hashes with one glob-discovered batched call, driven
red-green by a new fork-count test and a forced-fallback test, with the three
planted-stub tests as the standing regression guard.

### Changes Required

#### 1. Fork-count test (red first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Add a warm-path xtrace test asserting the backend forks once and no
`awk` runs. Follow the `_traced`/`_entered` pattern; add a small counter for the
backend-invocation line, anchored on an absolute-path argument to exclude the
`command -v sha256sum` detection line.

```python
def _backend_execs(trace: str) -> int:
    # `command -v sha256sum` is also traced; anchor on the path argument so only
    # a real hashing invocation is counted.
    pattern = r"^\++\S*:(?:sha256sum|shasum(?: -a 256)?) /"
    return len(re.findall(pattern, trace, re.MULTILINE))


def test_warm_path_forks_the_sha256_backend_once_without_awk(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    warm = _run_bootstrap(harness.root, harness.server, downloader)
    assert warm.returncode == 0, warm.stdout + warm.stderr
    traced = _traced(harness, downloader)
    assert traced.returncode == 0, traced.stdout + traced.stderr
    assert _backend_execs(traced.stderr) == 1, traced.stderr
    assert not re.search(r"^\++\S*:awk\b", traced.stderr, re.MULTILINE), (
        traced.stderr
    )
```

#### 2. Forced-fallback test (red first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Add a **warm** test that curates `PATH` so `command -v sha256sum`
misses but `shasum` resolves, then bootstraps twice against the same cache dir so
the second run finds a staged candidate and exercises the batched
`shasum -a 256 f1 f2` multi-input form — the parse that is otherwise untested and
where the saving is largest. It runs the second call under xtrace and asserts the
`shasum` branch was taken (one `shasum -a 256 /` line, no `sha256sum` line, no
`awk`), and that the staged shim is trusted (its 64-hex name-suffix equals the
`hashlib` digest of the source bytes). A single cold run would take the
zero-candidate single-input branch and never reach the batched parse, so warming
first is load-bearing.

Two helpers this and change 6 rely on do not exist yet; add them beside the
existing shim helpers. `_staged_shim_path(root, host_platform)` returns the
`accelerator-verify-${platform}-<digest>` path under the cache dir, and
`_staged_shim_digest(root, host_platform)` returns its name-suffix. Both must
apply the **same strict 64-lowercase-hex suffix filter** the production glob
uses, so a surviving `.staging.$$` temp is never mistaken for the staged shim.
When more than one hex candidate is present (as in the stale-shim test), they
resolve the shim matching the current `_source_shim_digest` rather than relying on
glob ordering. The existing `_source_shim_digest(root, host_platform)` is reused
as the oracle — note its real signature takes `root` and the `host_platform`
fixture, not a `harness`, so the new tests must request the `host_platform`
fixture.

```python
def test_warm_path_on_the_shasum_fallback_batches_and_trusts(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    shasum = shutil.which("shasum")
    if shasum is None:
        pytest.skip("no shasum backend to exercise the fallback")
    shim_bin = tmp_path / "fallback-bin"
    shim_bin.mkdir()
    (shim_bin / "shasum").symlink_to(shasum)
    for tool in ("cp", "chmod", "mv", "mkdir", "rmdir", "rm", "command"):
        resolved = shutil.which(tool)
        if resolved:
            (shim_bin / tool).symlink_to(resolved)
    harness = make_harness()
    warm = _run_bootstrap(
        harness.root, harness.server, downloader, path=str(shim_bin)
    )
    assert warm.returncode == 0, warm.stdout + warm.stderr
    traced = _run_bootstrap(
        harness.root, harness.server, downloader, path=str(shim_bin), xtrace=True
    )
    assert traced.returncode == 0, traced.stdout + traced.stderr
    assert re.search(r"^\++\S*:shasum -a 256 /", traced.stderr, re.MULTILINE), (
        traced.stderr
    )
    assert not re.search(r"^\++\S*:sha256sum /", traced.stderr, re.MULTILINE), (
        traced.stderr
    )
    assert _backend_execs(traced.stderr) == 1, traced.stderr
    assert not re.search(r"^\++\S*:awk\b", traced.stderr, re.MULTILINE), (
        traced.stderr
    )
    assert _staged_shim_digest(harness.root, host_platform) == _source_shim_digest(
        harness.root, host_platform
    )
```

⚠️ The curated `PATH` must still resolve the coreutils the staging body shells
out to (`cp`, `chmod`, `mv`) and the lock path (`mkdir`, `rmdir`, `rm`); confirm
the exact set from a `bash -x` trace and adjust the symlink list rather than
guessing. If a hermetic-env assertion in `run_bootstrap` rejects a curated
`PATH`, fall back to prepending `shim_bin` and shadowing `sha256sum` with a
non-executable stub in it so detection misses.

#### 3. The variadic batched helper

**File**: `bin/accelerator`
**Changes**: Replace `sha256_file` with `sha256_files`, one backend fork over all
arguments, no `awk`, raw `<digest>␣␣<path>` lines on stdout.

```bash
sha256_files() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$@"
	else
		shasum -a 256 "$@"
	fi
}
```

#### 4. Glob-discovered staging condition

**File**: `bin/accelerator`
**Changes**: Replace lines 292-296 with a hex-constrained glob discovery, one
batched call, and a path-column parse. Keep the staging body (lines 297-312)
unchanged, and update the rationale comment above it (see change 5).

```bash
shim_prefix="${cache_dir}/accelerator-verify-${platform}-"
shim_candidates=()
for candidate in "${shim_prefix}"*; do
	[[ -f "${candidate}" && -x "${candidate}" ]] || continue
	suffix=${candidate#"${shim_prefix}"}
	case "${suffix}" in
	"" | *[!0-9a-f]*) continue ;;
	esac
	[[ ${#suffix} -eq 64 ]] || continue
	shim_candidates+=("${candidate}")
done

if [[ ${#shim_candidates[@]} -eq 0 ]]; then
	shim_digests=$(sha256_files "${shim_source}") ||
		fail_integrity "could not hash the verify shim source: ${shim_source}"
else
	shim_digests=$(sha256_files "${shim_source}" "${shim_candidates[@]}" 2>/dev/null)
fi

shim_digest=""
while read -r line_digest line_path; do
	[[ "${line_path}" == "${shim_source}" ]] || continue
	shim_digest="${line_digest}"
	break
done <<<"${shim_digests}"
[[ -n "${shim_digest}" ]] ||
	fail_integrity "could not hash the verify shim source: ${shim_source}"

shim="${cache_dir}/accelerator-verify-${platform}-${shim_digest}"
staged_digest=""
while read -r candidate_digest candidate_path; do
	[[ "${candidate_path}" == "${shim}" ]] || continue
	staged_digest="${candidate_digest}"
	break
done <<<"${shim_digests}"

if [[ "${staged_digest}" != "${shim_digest}" ]]; then
	require_exec_capable_cache
	# … unchanged cp/chmod/mv staging body …
fi
```

⚠️ **The hex-suffix constraint is the trust-safety boundary, not decoration.**
The parse keys the staged digest on the path column of the batched output. A
candidate whose name embeds a newline could inject a forged line
`<source_digest>␣␣${shim}` that the loop then trusts by name — and Apple
`/sbin/sha256sum` and Perl `shasum` (unlike GNU coreutils) do not escape such
filenames. Admitting only `<64-hex>` name-suffixes structurally excludes newline,
space, and backslash, so no candidate filename can forge a line. The `[[ ==
"${shim}" ]]` exact-path match plus the `${#suffix} -eq 64` length check are the
real guard; the `[0-9a-f]` range in the `case` is a cheap ASCII prefilter, not
the boundary, so its collation sensitivity is immaterial. Keep the whole check;
loosening it reopens the bypass that
`test_planted_staged_shim_via_cache_dir_is_not_trusted` guards.

⚠️ **Both digests are keyed on the path column, not on output position.** The
batched (else) call runs with `2>/dev/null` and no `|| fail_integrity`, because
all three backends exit non-zero if any single input fails and a candidate that
vanishes mid-run (a concurrent bootstrap's `mv`, TOCTOU) must not abort a healthy
warm start. But that means a partial-failure call can still print candidate lines
while dropping the source line — so reading the source digest from line 1 by
position would silently promote a candidate's digest to the trust anchor. The
source digest is therefore matched by the line whose path column equals
`${shim_source}`, exactly as the candidate loop matches `${shim}`; the
`[[ -n "${shim_digest}" ]]` guard then aborts precisely when the source line is
absent (an unhashable source), a vanished candidate falls through to re-stage,
and no candidate can ever stand in for the source. The zero-candidate branch also
keeps `|| fail_integrity` on its single-input call.

⚠️ **Directory-path assumption.** The path-column match compares the full
`${cache_dir}/…` path, so it assumes the cache dir and plugin-root paths contain
no newline or backslash — a newline splits an input across lines and GNU
coreutils escapes a backslash with a leading `\`, either of which makes the match
fail. A space *within* a path segment is handled (`read -r a b` puts the remainder
in the last field), but a leading or trailing space on a path segment is not:
`read` strips trailing IFS whitespace from the last field, so such a path would no
longer equal `${shim}`/`${shim_source}` verbatim. On any pathological path
(newline, backslash, or edge whitespace) the effect is a benign
re-stage-every-time, or a source-abort only if the plugin-root path itself is
pathological; real platform-derived cache dirs contain none of these. Document it
as a domain constraint, as the filename residual is.

#### 5. Update the staged-shim rationale comment

**File**: `bin/accelerator`
**Changes**: The comment at `bin/accelerator:281-291` describes the mechanism
being removed — "skip-if-exists verifies the staged bytes against that digest",
the `[[ ! -x "${shim}" ]] || [[ hash != digest ]]` short-circuit. On a
root-of-trust boundary a comment that misdescribes how trust is established is
worse than none. Rewrite it to state the genuinely non-obvious *why*: the
content-addressed name plus re-hash is what makes a planted stub get re-staged
rather than trusted by name; the staged file is discovered by a hex-constrained
glob so the batch forks the backend once; and the hex constraint is what keeps
the path-column parse unforgeable. Trim any restatement of the control flow, and
drop the three planted-stub test-name references the current comment enumerates —
per the repo convention that test references in comments go stale fast.

#### 6. Stale-shim rejection and AC-4 digest-keying tests (red first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: The glob's multi-candidate selection is new code the old path never
had, so it needs a standing guard. Add two tests:

The stale-shim test **is** the AC-4 observation. Warm the cache once, then plant a
second executable `accelerator-verify-${platform}-<other-64-hex>` (a valid-shaped
but stale name) holding **garbage bytes** beside the real staged shim, and warm
again. The second run's batched call now hashes the source plus two hex
candidates, so it exercises exactly the multi-input, path-keyed parse. Assert the
run exits 0, the current shim is not re-staged (same inode/mtime), and the stale
file is neither trusted nor removed. The inode/mtime no-re-stage assertion is the
load-bearing one: if the second run mis-keyed the source digest by output
position, it would compute a wrong `${shim}`, fail to match the real candidate,
and spuriously re-stage — changing the inode and failing the test. The distinct
garbage bytes are a secondary safeguard: they make a wrongful overwrite of the
current shim detectable rather than silent.

This discharges AC-4. As worded in the work item AC-4 is the one-input-missing
path; the glob-guarded design never produces a missing input, so the equivalent
property is that the source digest is keyed correctly among multiple inputs — which
the no-re-stage assertion above verifies directly. A separate name-suffix-equals-
oracle assertion was considered and dropped: the staged file is named by the
earlier single-input staging run, so such an assertion would re-check that run's
trivially-correct digest, not the multi-input parse.

```python
def test_stale_staged_shim_is_ignored_not_trusted_or_removed(
    make_harness: Callable[..., Harness], downloader: Path, host_platform: str
) -> None:
    harness = make_harness()
    warm = _run_bootstrap(harness.root, harness.server, downloader)
    assert warm.returncode == 0, warm.stdout + warm.stderr
    shim = _staged_shim_path(harness.root, host_platform)
    before = shim.stat()
    stale = shim.with_name(shim.name[:-64] + "0" * 64)
    stale.write_bytes(b"#!/bin/sh\nexit 3\n")
    stale.chmod(0o755)
    again = _run_bootstrap(harness.root, harness.server, downloader)
    assert again.returncode == 0, again.stdout + again.stderr
    after = shim.stat()
    assert (after.st_ino, after.st_mtime_ns) == (before.st_ino, before.st_mtime_ns)
    assert stale.exists()
```

#### 7. Cold-path fork count and AC-3 stderr tests (red first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: The Desired End State claims one backend fork and no `awk` on the
**cold** path too, and AC-3 requires a cold run to emit no missing-input stderr;
both are cheaply automatable rather than left to manual inspection. Add a
cold-cache xtrace test (fresh `ACCELERATOR_CACHE_DIR`, so the zero-candidate
single-input branch runs) asserting exactly one backend fork, no `awk`, and no
missing-input line (e.g. `No such file`, `unbound variable`) in the combined
output. Mirror the existing cold-path probe test's fresh-cache setup.

### Success Criteria

#### Automated Verification

- [x] The warm-path fork-count test fails before the code change and passes
      after: `mise run test:integration -- -k forks_the_sha256_backend_once`
- [x] The cold-path fork-count / no-missing-input test passes (AC-3):
      `mise run test:integration -- -k cold_path`
- [x] The forced-fallback test exercises the batched `shasum` form and passes:
      `mise run test:integration -- -k on_the_shasum_fallback`
- [x] The stale-shim rejection test (which discharges AC-4) passes:
      `mise run test:integration -- -k stale_staged_shim`
- [x] The three planted-stub tests pass unmodified:
      `mise run test:integration -- -k planted_staged_shim`
- [x] The existing newline-cache-dir test stays green under the new path-column
      parse: `mise run test:integration -- -k newline`
- [x] The full entrypoint suite passes:
      `mise run test:integration -- tests/integration/entrypoint`
- [x] `scripts/lint-bashisms.sh` reports no findings on `bin/accelerator`
- [x] `mise run check` exits 0 (shfmt + ShellCheck across `scripts`, plus the
      other components)

The cold-path fork count / AC-3 stderr and the version-change (stale-shim) cases
are now automated (changes 6-7), so the manual lane keeps only the eyeball trace
sanity check the automation cannot fully substitute for.

#### Manual Verification

- [ ] A `bash -x` warm-path trace shows exactly one `sha256sum`/`shasum`
      invocation line and no `awk` line — inspected by eye, not only asserted.

---

## Phase 2: Measurement and evidence

### Overview

Record the before/after warm-path digest-bracket saving and the before/after
warm-dispatch ratio, both on one host in one session, with the resolved backend
named. Records-only; no production code.

### Changes Required

#### 1. Digest-bracket before/after (AC-6)

**File**: `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` (or a
measurement note under `meta/measurements/`)
**Changes**: Run the digest-bracket microbenchmark on the pre-change baseline and
on the post-change tree in the same session, **interleaving** before/after
samples (alternate the two trees rather than measuring in two blocks) so
monotone within-session drift — thermal throttling, background load — does not
bias whichever tree runs second. Record both medians, the sample count `n`, a
dispersion figure (IQR or the confidence interval), and the resolved backend
(`command -v sha256sum` output). The after-median must be strictly less than the
before-median on the resolved backend.

Take a second after-measurement against a cache dir **seeded with a growing
number of stale hex-named candidates**, so AC-6 characterises the steady state a
long-lived `ACCELERATOR_CACHE_DIR` reaches, not only the clean single-shim best
case. Record the **break-even N** — the candidate count at which the after-median
meets the before-median on the resolved backend — since each added ~475 KB file
costs an in-process read+hash (~0.2-0.4 ms on the fast Apple backend, more on the
~3× slower `shasum` fallback) against a saving that is mostly the three removed
forks. Compare that N to a realistic release-churn bound so the boundedness claim
rests on a number, not an adjective. Record the same figure for a deliberately
large N (the adversarial cache-dir-write worst case) so the DoS ceiling is a known
number, not just the release-churn case. Stale candidates are the only
accumulation source that survives the hex constraint (orphaned `.staging.$$` temps
are excluded by it).

```console
$ command -v sha256sum          # record the resolved backend
$ mise run measure:warm-dispatch # or the digest-bracket task directly
```

#### 2. Warm-dispatch ratio re-measure (AC-7)

**File**: the work item
**Changes**: Run `mise run measure:warm-dispatch` after the change and record the
before/after `median(G) / median(B)` beside the millisecond figures. The
`warm-dispatch-3.json` baseline (C5 = 1.3260 [1.3236, 1.3279]) is the "before"
ratio; capture the "after" run and note whether it clears 1.3 — evidence for
tightening 0189's C5 threshold from 1.4 back to 1.3, not a pass condition here.

#### 3. Cross-backend confirmation (AC-5)

**File**: the work item
**Changes**: Record that GNU coreutils is confirmed by the linux CI lane running
the Phase 1 tests green (its resolved `sha256sum`), and that the Perl `shasum`
fallback multi-file format is confirmed locally by the forced-fallback test and
the this-session check (`<hex>␣␣<path>`, argument order, exit 0).

### Success Criteria

#### Automated Verification

- [x] `mise run measure:warm-dispatch` completes and writes a measurement record
- [x] `mise run` (bare default task) exits 0 end-to-end (AC-8), including the
      linux CI lane confirming the GNU coreutils backend — bare `mise run` passed
      locally, and the ubuntu-latest integration lane passed green on PR #81

#### Manual Verification

- [x] Before/after digest-bracket medians, `n`, a dispersion figure, and the
      resolved backend are recorded in the work item, from interleaved samples;
      after < before on the resolved backend (AC-6)
- [x] A seeded-accumulation after-measurement (several stale hex-named candidates)
      is recorded and still shows after < before (AC-6, steady state)
- [x] Before/after `median(G)/median(B)` recorded beside the millisecond figures
      (AC-7); the after-ratio and whether it clears 1.3 are noted
- [x] The GNU coreutils and `shasum` observed formats are recorded (AC-5) —
      `shasum` and Apple `sha256sum` recorded locally; GNU coreutils confirmed by
      the ubuntu-latest integration lane passing green on PR #81

---

## Testing Strategy

All new tests drive the full `bin/accelerator` bootstrap through
`run_bootstrap` (several under `bash -x`), so they are integration tests, not
unit tests.

### Integration Tests

- Warm-path fork count: exactly one backend invocation, zero `awk`
  (new `test_warm_path_forks_the_sha256_backend_once_without_awk`).
- Cold-path fork count and AC-3: one invocation, zero `awk`, no missing-input
  stderr on a fresh cache (new cold-path test, change 7).
- Fallback batching: the warm second run takes the `shasum` branch, forks it
  once, uses the batched multi-input form, and trusts the staged shim
  (new `test_warm_path_on_the_shasum_fallback_batches_and_trusts`).
- Stale-shim rejection and AC-4 digest-keying: a valid-shaped stale candidate is
  ignored (not trusted, not removed) and the batched source digest equals the
  `hashlib` oracle (new tests, change 6).
- The three planted-stub tests
  (`test_accelerator_entrypoint.py:723,740,761`) remain the trust-boundary guard
  and must pass **unmodified** — the content-addressed name and the re-hash
  comparison are unchanged.
- The cold-path probe test and warm-path probe test are unaffected but must stay
  green (the staging block is upstream of the probe gates).
- The existing newline-cache-dir test
  (`test_accelerator_entrypoint.py:1210`) must stay green — it constructs a
  `cache\nwith-newline` cache dir, which is exactly the pathological input the new
  path-column match assumes absent, so it is the standing guard for the
  documented directory-path domain constraint (benign re-stage, not an abort).

### Manual Testing Steps

1. Warm-start twice against a throwaway cache dir; confirm the second run stages
   nothing and the `bash -x` trace shows one backend fork, no `awk`.
2. Delete `sha256sum` from `PATH` (leave `shasum`); confirm a warm start still
   stages and exits 0.
3. Plant `accelerator-verify-${platform}-<current-digest>` with garbage bytes;
   confirm it is re-staged and the run behaves as before.
4. Leave a stale `accelerator-verify-${platform}-<old-digest>` in the cache;
   confirm the current shim is staged and the stale file is ignored.

## Performance Considerations

The warm path drops from two backend forks plus two `awk` forks to one backend
fork and zero `awk` — a measured ~2.48 ms on the fast Apple backend, ~3× larger
on the Perl `shasum` fallback. The batched call hashes every candidate admitted
by the hex-constrained glob; in steady state that is one file, so the extra bytes
are negligible.

**The hashed set is bounded; the scanned set is not, but its cost is.** Two
dimensions differ. The **hashed** input — what the single backend fork reads — is
bounded: the strict `<64-hex>` suffix filter excludes orphaned
`accelerator-verify-${platform}-<digest>.staging.$$` temps (a `.staging.$$` name
is not bare hex), so the one unbounded fork+hash source is gone; stale staged
shims from prior versions are valid hex names and still add to the hashed input,
but their count is bounded by release churn, and the Phase 2 seeded measurement
records the break-even N. The **scanned** set — the glob enumeration plus the
per-candidate `[[ -f && -x ]]` stat — is still O(cache-dir entries), so a cache
subjected to repeated SIGKILLs in the narrow chmod-to-mv window grows a monotonic
set of `.staging` temps that lengthen the loop; this is pure in-process work
(sub-millisecond over hundreds of files) and never reaches the backend, so it is
negligible but not formally bounded. Active cleanup of stale shims is deliberately
**not** added here — unlinking on a shared, concurrently accessed root-of-trust
path adds a race for a cost that release churn keeps small; leave it to
VCS-recoverable manual cache clearing or a separate item.

**Adversarial framing (accepted residual).** Because the batched call reads every
hex-named candidate, an attacker who can write to the cache dir can seed many
valid-shaped ~475 KB files and amplify the per-warm-start hashing cost — a denial
of service, never a trust breach (content-addressing still forces a re-stage of
anything whose bytes do not match its name). This is accepted rather than
mitigated: an attacker-writable cache dir is an already-degraded posture with
stronger, easier DoS vectors (delete the staged shim to force a 475 KB re-copy on
every start, hold the lock, exhaust the disk), so the marginal amplification is
small, and bounding it would cost either the single-fork guarantee (AC-1) or added
capping complexity on the root-of-trust hot path. The Phase 2 seeded measurement
records the adversarial worst case (a large N) alongside the release-churn case so
the cost is a known number rather than an assumption.

## Migration Notes

None. The on-disk staged-shim naming and the lock contract are unchanged; a
mixed fleet of old and new `bin/accelerator` copies interoperates because they
read and write the same `accelerator-verify-${platform}-${digest}` files.

## References

- Original work item:
  `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md`
- Related research:
  `meta/research/codebase/2026-08-22-0191-batch-shim-hashes.md`
- The two call sites and short-circuit: `bin/accelerator:273-279,292-312`
- Planted-stub tests: `tests/integration/entrypoint/test_accelerator_entrypoint.py:723-784`
- Trace seam: `tests/integration/support/installation.py:275-334`
- Fork-count test pattern: `tests/integration/entrypoint/test_accelerator_entrypoint.py:1263-1350`
- Measurement harness: `tasks/measure.py:1047,2661`; baseline
  `meta/measurements/warm-dispatch-3.json`
