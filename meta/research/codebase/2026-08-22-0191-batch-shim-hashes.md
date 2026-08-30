---
type: "codebase-research"
id: "2026-08-22-0191-batch-shim-hashes"
title: "Research: Batch the bootstrap's two shim hashes into one sha256 invocation (0191)"
date: "2026-08-22T22:37:37+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0191"
parent: "work-item:0191"
relates_to: ["work-item:0186", "work-item:0189", "work-item:0205", "work-item:0169"]
topic: "Batching the two shim sha256 hashes in bin/accelerator into one backend invocation"
tags: ["research", "codebase", "shell", "bootstrap", "bash-3.2", "performance", "sha256"]
revision: "9fa29fb6a49c5538d81fbb91d168a3dd89d4bcdc"
repository: "accelerator"
last_updated: "2026-08-22T22:37:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Batch the bootstrap's two shim hashes into one sha256 invocation (0191)

**Date**: 2026-08-22T22:37:37+00:00
**Author**: Toby Clemson
**Git Commit**: 9fa29fb6a49c5538d81fbb91d168a3dd89d4bcdc
**Branch**: detached HEAD (0191 work branch)
**Repository**: accelerator

## Research Question

For work item 0191, batch the two `sha256_file` calls in `bin/accelerator`'s
shim-staging condition into one backend invocation without weakening the
planted-stub trust boundary or paying to hash a nonexistent file on a cold run.
What is the exact code, how do the pinning tests work, which measurement gate
does the change feed, and what backend behaviour must the parse tolerate?

## Summary

The change is small, well-bounded, and the one blocking empirical unknown is now
closed on the Apple backend. The two calls live in `bin/accelerator:292` (source
shim, always run) and `bin/accelerator:296` (staged shim, guarded behind a
`[[ ! -x "${shim}" ]] ||` short-circuit). Both feed a content-addressed
staging check: the staged copy is named for the source digest, and the warm path
re-hashes its bytes to force a re-stage on mismatch.

**The recommended shape is the guarded batch, not the always-batch.** Keep the
short-circuit exactly as-is and batch both files into one `sha256sum f1 f2`
(no `awk`) only inside the block that already knows the staged shim exists; on a
cold run, hash only the source in a single call. This gives one backend fork and
zero `awk` on both paths, preserves the short-circuit the work item requires, and
sidesteps the missing-file parse entirely. Key the parse on the path column, not
output position, per the AC — even though the guarded form never hits the
missing-input case.

**The saving is ~2.48 ms on this host's fast backend** (7.05 ms → 4.57 ms for
the two substitutions), larger on the Perl `shasum` fallback where its case now
primarily rests. It is **not** a co-requisite of any latency gate — 0189 closed
at C5 = 1.3260 against a 1.4 ceiling — but it is the recorded evidence route to
tightening 0189's C5 threshold back from 1.4 to 1.3, needing only ~0.75 ms of the
2.48 ms it delivers.

**No trace-based fork-count test exists yet** — the seam does (`xtrace` in
`run_bootstrap`), and an analogous test exists for the exec probe, so a
`+sha256_file:`/`+awk:` count test follows a known pattern.

## Detailed Findings

### The two call sites and the short-circuit (`bin/accelerator`)

`sha256_file` is a two-branch detection-based helper, inlined deliberately
because the root-of-trust entry point sources nothing (`bin/accelerator:270-279`):

```bash
sha256_file() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{print $1}'
	else
		shasum -a 256 "$1" | awk '{print $1}'
	fi
}
```

Each call is two forks: the backend plus an `awk` to strip the path column. The
staging condition (`bin/accelerator:292-312`) uses it twice:

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

Control flow, precisely:

- **`sha256_file "${shim_source}"`** (line 292) always runs. Its output
  `shim_digest` also names the staged path `shim` (line 294).
- **`sha256_file "${shim}"`** (line 296) runs only when `[[ ! -x "${shim}" ]]`
  is false. The two `[[ … ]]` tests are joined by `||`, so on a cold run
  (staged shim missing/non-executable) the left test is true, the condition
  short-circuits, and the second hash never forks. The second hash fires only on
  the warm path, to compare staged bytes against `shim_digest`.

Variable names: source path `shim_source` (line 163), staged path `shim`
(line 294), source digest `shim_digest` (line 292), staging temp
`shim_tmp` (line 305). ⚠️ The guarded call redirects backend stderr to
`/dev/null` (line 296); the always-run call does not — a batched form must
preserve stderr suppression on the path that can hit a missing file.

### Bash 3.2 floor constraints (`bin/accelerator:11`)

The file declares the floor at line 11 (`no associative arrays, ${var,,},
mapfile`) and runs under `set -uo pipefail` (line 26, no `set -e` — gates use
explicit `|| fail`). ⚠️ For batched-output parsing this means **no
`mapfile`/`readarray`** — read the two lines with `read -r digest path` over a
here-string or a `while` loop. The file already uses the base-10 guard idiom
`$((10#${max_wait}))` (line 330) and `command -v … >/dev/null 2>&1` detection
(lines 148, 154, 274).

### The three planted-stub tests (`tests/integration/entrypoint/test_accelerator_entrypoint.py:723-784`)

All three depend only on the digest *value* and the equality comparison at
`bin/accelerator:296`. Each computes the expected digest independently with
Python `hashlib.sha256` over the source shim via `_source_shim_digest`
(lines 512-514). As long as `sha256_file` keeps emitting the standard hex sha256
of the file — whatever its internal forking — the content-addressed name and the
re-hash comparison are unchanged and the tests stay green.

| Test (line) | Plants | Asserts | Pins |
| --- | --- | --- | --- |
| `test_planted_staged_shim_rehashed_then_succeeds` (723) | garbage at digest-named path in `bin/` | exit 0; planted bytes replaced by source | re-hash forces re-stage even when file exists+executable |
| `test_planted_staged_shim_is_not_trusted` (740) | permissive `#!/bin/sh\nexit 0` stub, attacker-signed launcher | exit ≠ 0; planted bytes replaced | digest check is a security boundary, not a freshness cache |
| `test_planted_staged_shim_via_cache_dir_is_not_trusted` (761) | same stub in caller-chosen `ACCELERATOR_CACHE_DIR` | exit ≠ 0; planted bytes replaced | caller cache path cannot smuggle a trusted-by-name verifier |

### The trace seam and the missing fork-count test

The seam 0186 added is the `xtrace` parameter to `run_bootstrap`
(`tests/integration/support/installation.py:275-334`): when `xtrace=True` it runs
one call under `bash -x` and sets `PS4='+${FUNCNAME[0]:-main}:'` (lines 315-317).
The `:-main` default is required because a bare `${FUNCNAME[0]}` is unbound at top
level under `set -u` on bash 3.2. The interpreter is pinned to `/bin/bash`
(bash 3.2 floor), so the trace reflects the shipping shell, not a contributor's
bash 5.

Test-side helpers already exist (`test_accelerator_entrypoint.py:1263-1290`):
`_traced` runs an xtrace bootstrap; `_entered(trace, fn)` matches
`^\++<fn>:`; `_probe_execs` counts anchored probe-exec lines. These drive
`test_warm_path_does_not_enter_the_probe` (1319) and
`test_cold_path_enters_and_executes_the_probe` (1336).

❌ **No sha256 fork-count test exists.** There are no references to `sha256`,
`sha256sum`, `shasum`, or `awk` in any test body. A new AC-1 test follows the
`_entered`/`_probe_execs` pattern exactly: assert a warm-path trace enters
`sha256_file` (and that its `awk` frame is absent, and the backend appears once
not twice). Because the digest logic is a named function, the `+sha256_file:`
frame label makes this countable.

### Backend behaviour — Apple `/sbin/sha256sum` confirmed this session

Ran on darwin-arm64, `command -v sha256sum` → `/sbin/sha256sum`:

- ✅ **Both inputs present**: two `<digest>␣␣<path>` lines in argument order,
  exit 0.
- ✅ **Second input missing**: surviving digest for the present file on stdout
  *with its path* (exactly one line), `sha256sum: <path>: No such file or
  directory` on stderr, exit 1.
- ✅ `read -r digest path` splits each line cleanly (two spaces collapse in
  default IFS).

This closes the Apple half of the work item's blocking Open Question. The
missing-input path yields a stdout line keyed to the surviving path, so a
path-column parse cannot mis-assign the surviving digest — satisfying AC-4's
requirement. ❓ **GNU coreutils and the Perl `shasum` fallback remain
unconfirmed** — not installed on this host. GNU coreutils is what the linux CI
lane resolves (AC-5); `shasum` matters only if the batched form is used on the
fallback path.

### Measurement harness (`tasks/measure.py`, `meta/measurements/`)

- `mise run measure:warm-dispatch` → `invoke measure.warm-dispatch`
  (`mise.toml:324`), implemented at `tasks/measure.py:1047`. Teardown at
  `mise.toml:329`; smoke check at `mise.toml:334`.
- Digest-bracket microbenchmark: `measure_digest_bracket`
  (`tasks/measure.py:2661`), "cost of the bootstrap's two digest substitutions";
  backend detection `digest_backend_population` (line 1112); constants
  `FAST_BACKEND = "sha256sum"`, `SHA256_HEX_LENGTH` (lines 208-209).
- Baseline record the item is measured against: `meta/measurements/warm-dispatch-3.json`
  (+ `-samples.json`). Earlier runs `warm-dispatch-1.json`, `-2.json`.
- Tests: `tests/unit/tasks/test_measure.py`, `test_mise.py`;
  `cli/launcher/tests/warm_terms.rs`.

## Code References

- `bin/accelerator:270-279` — `sha256_file`, inlined, two-fork (backend + `awk`)
- `bin/accelerator:292-294` — always-run source hash; names the staged path
- `bin/accelerator:295-296` — the `[[ ! -x "${shim}" ]] ||` short-circuit + guarded staged hash
- `bin/accelerator:297-312` — staging body (cp/chmod/mv through per-process temp)
- `bin/accelerator:11,26,330` — bash 3.2 floor, `set -uo pipefail`, base-10 guard
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:723-784` — three planted-stub tests
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:512-514` — `_source_shim_digest` (Python oracle)
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:1263-1350` — xtrace helpers + probe fork-count tests
- `tests/integration/support/installation.py:275-334` — `run_bootstrap` + xtrace/PS4 seam
- `tasks/measure.py:1047,2661` — `warm-dispatch` task; digest-bracket microbenchmark
- `meta/measurements/warm-dispatch-3.json` — baseline C5 = 1.3260 record

## Architecture Insights

- **Content-addressing is the trust mechanism, not a cache.** The staged shim is
  named for the source digest and re-hashed on every warm start; the digest
  comparison is a security boundary (proven by the two attacker-signed tests),
  so both digests must survive any batching. The change is a fork-count
  optimisation strictly inside an unchanged trust check.
- **Named functions are the fork-counting seam.** Keeping digest logic in
  `sha256_file` (rather than inlining the pipe) is what lets `bash -x` + `PS4`
  attribute forks to a frame. A batched helper should stay a named function for
  the same reason, and stay inline in the file (no `source`).
- **The guarded batch preserves the short-circuit for free.** Batching only
  inside the `if` body — where the staged shim provably exists — means the cold
  path hashes one file in one call and the warm path hashes two in one call.
  Both paths reach one backend fork and zero `awk` without ever hashing a
  missing file, which is why it is preferable to the always-batch-and-tolerate-
  stderr shape the Technical Notes list as the alternative.

## Historical Context

- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — measured the
  saving (7.05 → 4.57 ms = 2.48 ms), added the xtrace seam, and declined to
  absorb the batch because it "needs a branch to keep today's short-circuit" and
  deserves its own before/after. Warm-path after-median 29.92 ms, of which the
  two `sha256_file` calls are ~7.1 ms on the fast backend.
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  "What We're NOT Doing" records the batch as a deferred follow-up with the
  figure and the branch caveat attached.
- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the
  measurement method (n interleaved B/G pairs, ratio of medians, paired
  bootstrap) and the baseline `median(B)`/`median(G)`; lever table sizes 0191 at
  −2.48 ms.
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — defines C5
  (`median(G)/median(B)` on the fast backend), raised the ceiling 1.3 → 1.4 on
  author decision, and records 0191 as the deferred lever to tighten back to 1.3
  on evidence. `warm-dispatch-3.json` is 0189's first valid run: C5 = 1.3260
  [1.3236, 1.3279].

## Related Research

- `meta/work/0215-remove-the-cache-hit-sha256-from-warm-dispatch.md` and
  `meta/work/0216-close-the-sha2-hardware-intrinsics-gap.md` — the other
  warm-path levers; 0191 is intentionally sequenced independent of both.
- `meta/work/0217-measure-warm-dispatch-on-linux.md` — the linux-lane
  measurement that would confirm the GNU coreutils backend behaviour AC-5 needs.

## Open Questions

- ❓ **GNU coreutils multi-file format + missing-input exit** — confirm one
  `<digest>␣␣<path>` line per input in argument order, and sane exit/stderr on a
  missing second input, on the backend the linux CI lane resolves (AC-5). Not
  testable on this darwin host.
- ❓ **`shasum` fallback** — only needs confirming if the batched form is used on
  the fallback path. `shasum -a 256 f1 f2` output format is unverified here.
- ❓ **Guarded vs always-batch decision** — this research recommends the guarded
  batch (one fork on both paths, no missing-file case). Confirm the AC-4
  path-keyed-parse requirement is still satisfied by a guarded form that never
  hits the missing input (it is: the source digest from a single-file call equals
  its standalone `sha256_file` value trivially).
