---
type: plan
id: "2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"
title: "Remove the Exec Probe from the Bootstrap Warm Path Implementation Plan"
date: "2026-08-02T22:01:31+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0186"
parent: "work-item:0186"
derived_from:
  ["codebase-research:2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"]
relates_to: ["work-item:0169", "work-item:0182", "work-item:0164"]
tags: [shell, performance, bootstrap, bash-3.2, testing]
revision: "4a68344cd2614f3bdd07223c8aeaf64583c036f0"
repository: "accelerator"
last_updated: "2026-08-03T10:31:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Remove the Exec Probe from the Bootstrap Warm Path Implementation Plan

## Overview

`bin/accelerator` runs a write-chmod-exec probe on every invocation, costing
~108 ms of a ~149 ms warm call — almost all of it macOS's first-exec check on a
freshly written binary. Split `probe_dir` into an always-run `ensure_dir` and a
cold-path-only `probe_exec_capable`, so the warm path pays neither the write nor
the exec.

## Current State Analysis

The probe reaches the warm path because `resolve_cache_dir` calls `probe_dir`
and is itself invoked unconditionally at `bin/accelerator:195`, before any
warm/cold decision. The execution order that matters:

| Line | What happens | Runs warm? |
| --- | --- | --- |
| `:166-180` | `probe_dir` — `mkdir -p`, write, `chmod +x`, exec, `rm` | yes |
| `:184-193` | `resolve_cache_dir` — override-or-default, calls it | yes |
| `:195-197` | call site + `fail "no writable, exec-capable …"` | yes |
| `:198` | `unverified_log` reassigned under `cache_dir` | yes |
| `:207-233` | dev-launcher override (execs, never returns when taken) | yes |
| `:252-253` | `sha256_file` #1 over `shim_source` | yes |
| `:255-256` | staging **condition** — `-x` test + `sha256_file` #2 | yes |
| `:257-260` | staging **body** — `cp` + `chmod` into `cache_dir` | cold only |
| `:305-307` | `launcher`, `launcher_sig`, `base_url` assignments | yes |
| `:336` | cache-hit test: `-x launcher` + `-f sig` + `verify_launcher` | yes |
| `:339-347` | cold branch — `acquire_lock`, `fetch_and_verify` | cold only |
| `:352` | `exec "${launcher}" "$@"` | yes |

The probe never *chooses* a directory: `resolve_cache_dir` takes
`ACCELERATOR_CACHE_DIR` if set, else `${plugin_root}/bin`, and returns 1 rather
than falling back. Only the diagnostic's firing point moves.

Verified against the post-0182 tree at revision `4a68344c`: every line number,
every quoted test name, and the no-`build:cli:dev` guard are current.

## Desired End State

A warm `bin/accelerator` invocation execs no **freshly written** file — no
probe write, no probe `chmod`, no probe exec. (It still execs `sed`, `uname`
twice, the sha256 backend twice, and the staged shim and launcher; none of those
pay macOS's first-exec penalty, which is the entire cost being removed.)
Expected landing near ~41 ms against a ~149 ms pre-0182 reference.

The `no writable, exec-capable cache directory` diagnostic still fires on every
path that cannot use its cache dir, with no new hang, and now reports which of
the three causes it actually detected. Verified by eight regression cases in the
entrypoint suite plus a recorded same-session latency measurement.

### Key Discoveries

- **The probe must run before the staging `cp`, and the cheapest way to
  guarantee that is to put it *inside* the staging `if` body.** Staging
  (`:252-261`) precedes the warm/cold branch (`:336`). Measured on this host: a
  `chmod -x` directory permits `mkdir -p` on an existing path but fails writes,
  fails `cp`, and makes `[[ -x dir/file ]]` false. A probe placed only in the
  `else` at `:339` never runs on that scenario — `cp` fails first with `could
  not stage the verify shim into …`, so the acceptance criteria asserting the
  cache-dir substring would fail.

  The staging body is the **first required write into `cache_dir`** on every
  path, so gating there covers every write without needing a separate
  pre-staging gate, without hoisting `launcher`/`launcher_sig`, and without a
  second copy of the cache-hit predicate. Two writes sit outside that
  invariant, both benign and both worth naming so the invariant can be
  re-verified by grepping for writes into `${cache_dir}`/`${unverified_log}`
  rather than by trusting prose: `fail_integrity`'s append to
  `.accelerator-unverified.log` (best-effort, failure paths only), and the
  dev-override block's append at `:228-231` (guarded by `|| true`, and it
  `exec`s immediately after).

- **A staging gate alone introduces a ~30 second hang.** In the residual case —
  launcher present and executable, signature present, staged shim present and
  matching (so the staging body is skipped), *verification fails*, directory
  unwritable — no staging gate fires and control reaches `acquire_lock` at
  `:339` before `fetch_and_verify`. `mkdir "${lock_dir}"` can never succeed, no
  pid file exists, so the loop takes the `else` arm and spins its full
  300 × 0.1 s budget before failing with a lock-timeout message. Measured at a
  reduced ceiling: `TIMEOUT after 31 iters, 3s`. Today that case fails
  instantly with the correct diagnostic, so a second call site at the top of
  the cold branch is required to avoid a regression, not merely to preserve a
  nicer message. This supersedes the research's framing of the residual as a
  cosmetic diagnostic narrowing.

- **`PS4='+${FUNCNAME[0]}:'` is broken under the bootstrap's `set -u` on bash
  3.2.** Measured: every top-level command emits `FUNCNAME[0]: unbound
  variable` to stderr and PS4 stays literally unexpanded.
  `'+${FUNCNAME[0]:-main}:'` is clean and labels the top-level frame usefully.
  The plan adopts the fixed string; this is a deliberate deviation from the
  acceptance criterion's literal wording. The failure mode differs by
  interpreter — an unbound expansion aborts a non-interactive bash 5 outright —
  so the `:-main` default is load-bearing on the linux lane for a different
  reason than the one measured here.

- **Trace depth is not stable.** `resolve_cache_dir` runs inside `$( )` at
  `:195`, so its lines carry `++`; a top-level call site carries `+`.
  Assertions must match the function token allowing one or more leading `+`,
  never a fixed count.

- **Redirections are invisible to xtrace.** The probe writes via `>` at `:169`,
  so `+probe_dir:printf '#!/bin/sh\nexit 0\n'` appears with no filename. The
  function name is the only reliable observable — confirmed empirically.

- **The probe's exec IS observable**, as a line whose whole command word is the
  probe path: `+probe_exec_capable:/…/.accelerator-probe-76236`. The matcher
  must anchor on the `/` immediately after the PS4 colon, because xtrace also
  emits the function's own `probe=/…/.accelerator-probe-76236` **assignment**
  line. Measured against a real trace of the proposed function: the
  unanchored form `…:\S*\.accelerator-probe-\S*$` matches **two** lines (the
  assignment and the exec), so it passes on an implementation that never execs
  anything; the anchored form matches exactly one. The `chmod +x …` and
  `rm -f …` lines are excluded by both, and the top-level call line
  (`+main:probe_exec_capable /dir`) by neither, since the function token must
  follow the leading `+`s.

- **A failed redirection reports through the shell, not the command.** The
  probe's `printf … >"${probe}" 2>/dev/null` still emits
  `bash: /…/.accelerator-probe-<pid>: Permission denied` on an unwritable
  directory: the `2>/dev/null` applies to `printf`, but the shell fails the
  redirection before `printf` runs. Pre-existing in `probe_dir` and unchanged
  here, but worth knowing — the new cases assert a substring of combined
  output, so this noise is harmless, and a reader should not expect the
  redirection to be silent.

- **`SHELLOPTS=xtrace` is not equivalent to `-x`** and is rejected. It is
  exported and honoured by every **bash** descendant — non-bash children (the
  injected Python downloader, the Rust shim and launcher) ignore it. The
  operative hazard is that the probe's own `#!/bin/sh` *is* bash in posix mode
  on macOS but dash on Linux, so a global trace mode would make trace content
  lane-dependent; and it pollutes the same stderr the existing cases assert on.
  `BASH_XTRACEFD` is bash 4.1+ and unusable at the floor. Hence the narrow
  `xtrace` seam in the harness.

- **The real-launcher route is free.** `launcher_bin` is module-scoped
  (`test_accelerator_entrypoint.py:68`) and three tests (`:772`, `:787`,
  `:801`) already trigger the cargo build, so `real_launcher=True` adds no
  build cost. This reverses the research's recommendation of the stub route.

- **`run_bootstrap` already converts a hang into a named failure.** Its
  `timeout` keyword is wired to `pytest.fail(f"the bootstrap did not
  terminate: {entry}")` (`installation.py:366-369`), so the anti-hang case
  needs no new harness machinery — only an explicit `timeout=`.

- **The cached-launcher path has an established idiom.**
  `test_accelerator_entrypoint.py:219` and `:272` both build it directly from
  the `host_platform` fixture (`:58`) as
  `f"accelerator-launcher-{_VERSION}-{host_platform}"`. A `glob` plus a
  `Path.suffix` filter is not equivalent: for the fixture's `9.9.9-test`
  version, `Path("accelerator-launcher-9.9.9-test-darwin-arm64").suffix` is
  `'.9-test-darwin-arm64'`, so a `!= ".minisig"` filter only works by accident,
  and ruff-format rewrites the multi-line generator expression, reddening
  `format:build-system:check`. Both verified.

- **No `build:cli:dev` edge exists or may be added.**
  `installation.py:149-157` builds the launcher in-fixture, and
  `tests/unit/tasks/test_mise.py:104-109` fails if the edge appears. Adding
  cases to the existing file needs no task wiring.

- **The harness is shared** with
  `tests/integration/skill-invocation/test_skill_invocation_conformance.py`, so
  any `run_bootstrap` signature change must be backwards-compatible.

- **`bin/accelerator` is tab-indented and must stay so.** `.editorconfig`'s
  shell section is keyed `[*.sh]`, which does not match an extensionless file,
  so shfmt applies its defaults. A space-indented function reddens
  `format:scripts:check`. Note `bin/accelerator` is nonetheless in shell-source
  scope: `tasks/shared/sources.py:110` adds it to `_EXTRA_SHELL_SOURCES` by
  name, so any edit to it — including a comment — is gated by `scripts:check`.

- **The sha256 residual depends on which backend resolves, and the figures in
  circulation describe the wrong one.** On *this* host (darwin 25.3),
  `command -v sha256sum` resolves to `/sbin/sha256sum`, an Apple-signed Mach-O
  binary, so `sha256_file` never reaches its `shasum` fallback. Measured
  against the real 465 K shim: `$(sha256sum … | awk …)` is **4.22 ms**; the Perl
  `$(shasum -a 256 … | awk …)` variant is **12.97 ms**; `sha256sum` alone is
  1.99 ms against a 1.48 ms fork+exec floor, so actual hashing is ~0.5 ms
  (≈900 MB/s) and the rest is process startup. The ~11.7 ms per-call figure the
  work item and 0169's hand-off quote matches the *Perl* variant.
  **This is a single-host observation, not a darwin fact** — `/sbin/sha256sum`
  is a comparatively recent macOS addition, so an older macOS or a minimal
  image may resolve only `shasum`, which swings the two-hash residual roughly
  3× (~8.4 ms versus ~26 ms). Phase 4 records the resolved backend and the OS
  version, and hands 0169 the range rather than a point estimate.

- **Batching the two hashes into one invocation saves ~2.5 ms**, measured on
  this host: two `$(sha256sum f | awk …)` substitutions cost 7.05 ms against
  4.57 ms for one `$(sha256sum f1 f2)` with no `awk` (bash-interpreter baseline
  2.02 ms). Both digests are still computed and compared, so the planted-stub
  defence is untouched — but it needs a branch to preserve today's
  short-circuit (the second hash currently runs only when the staged shim is
  executable, and a batched call would hash a nonexistent file on a cold run).
  Recorded, not adopted: see What We're NOT Doing.

- **Both CI lanes are verified unprivileged.** `.github/workflows/main.yml`
  contains no `container:` key at all; `test-integration` (`:55-91`) is a plain
  `runs-on: ${{ matrix.os }}` matrix over `ubuntu-latest`/`macos-latest`. The
  single `docker` reference (`:145`, `test:e2e:visualiser:docker`) belongs to
  the visual-regression job, which never reaches the entrypoint suite. So the
  root guard will not fire in CI, and no lane exclusion is expected.

- **The linux lane already exercises bash 5.** The harness pins
  `BASH = "/bin/bash"` (`installation.py:41`) deliberately, to hold the 3.2
  floor on darwin — and on `ubuntu-latest` that same path *is* bash 5.2. So the
  trace assertions run under both interpreters on every CI run; the risk is
  discovering a divergence in CI rather than locally, not an untested
  mechanism.

- **`v1.24.0-pre.21` has published signed darwin-arm64 release assets**
  (published 2026-08-02, after 0182 merged), and `bin/accelerator-launcher-*`
  is gitignored — so the measurement can use a genuine release asset without
  dirtying the tree. `.gitignore:48` also already covers `bin/.tmp-*`, which is
  why the measurement's pre-change copy is named `bin/.tmp-accelerator-before`
  rather than `bin/.accelerator-before`: in a jj working copy an unignored file
  is auto-snapshotted on virtually any `jj` command, so an executable copy of a
  superseded trust-root bootstrap must not be left unignored in `bin/`.

## What We're NOT Doing

- **Not touching the shim staging block's hashes** (`:252-256`) or its second
  `sha256_file`. Resolved during 0186's review-1: both hashes stay. Three tests
  at `test_accelerator_entrypoint.py:584-644` assert the planted-stub defence it
  provides. The measured ~8.4 ms cost remains on the warm path. The staging
  `if` **body** gains one probe call — that is a cold-path-only addition and
  does not touch the hashing or the copy.
- **Not batching the two hashes into one `sha256sum` invocation**, despite the
  measured ~2.5 ms saving above. It needs a branch to keep today's
  short-circuit, and ~2.5 ms is close enough to 0169's ~2.4 ms shortfall that
  it deserves its own item with its own before/after rather than riding along
  here. Recorded in Phase 4 as a candidate follow-up with the figure attached.
- **Not hoisting `launcher`/`launcher_sig`.** An earlier draft of this plan
  moved them above the staging block to feed a pre-staging cache-hit gate.
  Gating inside the staging body removes the need, so `:305-307` stays intact —
  no ordering invariant between the assignments and the gate, and no third copy
  of the `-x launcher` / `-f sig` predicate.
- **Not mounting a genuinely `noexec` filesystem.** The exec-vs-write coverage
  gap is recorded, not closed.
- **Not changing 0169's latency threshold or its rationale.** This plan only
  re-confirms the hand-off note's figures, correcting the residual it quotes.
- **Not touching `cli/launcher/src/launch/outbound/resolve/cache_root.rs`** in
  this work item — but see Phase 4's follow-up: the launcher runs the same
  write-chmod-exec probe on every external-subcommand dispatch, so the saving
  here does not reach `accelerator vcs guard`, and a read-only cache directory
  is only usable for warm *bootstrap* invocations.
- **Not fixing `acquire_lock`'s inability to distinguish contention from an
  unusable directory.** The cold-branch gate masks the one instance reachable
  today; a follow-up is raised in Phase 4.
- **Not changing `sha256_file`'s backend detection.** Measurement shows the
  native backend is already in use on this host and that `openssl dgst`
  (4.53 ms) is slower than it; dropping the two `awk` execs alone saves
  ~0.4 ms. The conclusion is host-conditional — see Key Discoveries — so Phase 4
  records the resolved backend rather than closing the question globally.
- **Not adding a `test:integration:*` leaf.** Cases go in the existing file.
- **Not adding a pytest marker for the root guard.** Both lanes are verified
  unprivileged, so no exclusion is expected; Phase 4 records the marker as the
  escape to add *if* a root lane ever appears, and Phase 1's guard message
  therefore does not promise a mechanism that does not exist.
- **Not adding a `.gitignore` entry.** The measurement's temporary copy uses the
  existing `bin/.tmp-*` rule; `bin/.accelerator-probe-*` is unchanged by this
  work and the probe cleans up after itself (asserted in Phase 2).

## Implementation Approach

Split `probe_dir` into `ensure_dir` (the `mkdir -p`, still reached on every
invocation via `resolve_cache_dir`) and `probe_exec_capable` (the
write-chmod-exec-rm, now returning distinct statuses for a write failure and an
exec failure), called from two cold-path sites through
`require_exec_capable_cache`:

- **The staging gate** — the first statement inside the shim-staging `if` body,
  before the `cp`. The body is the first required write into `cache_dir` on
  every path, so this covers every write, and it is never entered on a warm
  call.
- **The cold-branch gate** — at the top of the cold branch before
  `acquire_lock`, covering the verification-failed residual where staging was
  skipped.

Both go through one idempotence flag, so a cold run reaching both probes exactly
once. Neither is reached on a warm call, and because both sit below the
dev-override block the contributor dev path stops paying for a probe it never
needed.

`fail_no_cache_dir` is defined **above** the `resolve_cache_dir` call site and
takes the offending directory plus a cause clause, so the substring the tests
assert has exactly one definition while each site reports what it actually
detected — and the message keeps the `ACCELERATOR_CACHE_DIR` remediation hint
the pre-change wording carried.

Phases are ordered so each is independently green and mergeable. Phase 1 is the
root-privilege guard, which closes a real false negative today and needs
nothing from the rest. Phase 2 carries the trace seam, the trace helpers and
the production change with its regression cases, written test-first. Phase 3 is
documentation. Phase 4 is measurement and closeout.

---

## Phase 1: Root-Privilege Guard

### Overview

Add the root-privilege guard and retrofit it onto the three existing permission
tests that would otherwise pass as false negatives under uid 0. This phase
stands alone: it closes an existing false negative and needs nothing from the
later phases. No production behaviour changes; the suite stays green.

### Changes Required

#### 1. Root-privilege guard

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: A hard-failing guard, deliberately diverging from the `skipif`
idiom at `tests/integration/hooks/test_launcher_link_refresh.py:275-293`.
Docstring shaped like the file's others (`:79-83`).

```python
def _require_unprivileged() -> None:
    """Hard-fail rather than skip under uid 0.

    Root bypasses both the write permission and the execute bit, so these
    assertions would hold whatever the code did; a skip would report green on a
    lane that verified nothing.
    """
    assert os.getuid() != 0, (
        "these cases assert on permission bits, which are advisory for uid 0; "
        "run them unprivileged, or exclude them with a recorded privilege check"
    )
```

#### 2. Retrofit onto existing permission tests

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Call `_require_unprivileged()` as the first statement of
`test_readonly_root_with_override_runs_from_override` (`:253`),
`test_readonly_root_without_override_is_a_named_error` (`:279`) and
`test_a_record_is_always_one_line` (`:1060`).

The asymmetry worth knowing when reading these: the cases asserting **success**
under a restrictive mode are the ones that go silently green under uid 0, since
root's write succeeds regardless of the implementation. The cases asserting
**failure** would red under root instead — noisily, but for the wrong reason.
Both are covered by the same guard.

### Success Criteria

#### Automated Verification

- [x] Entrypoint suite passes: `mise run test:integration:entrypoint`
- [x] Python format, lint and types pass: `mise run build-system:check`
- [x] Full read-only gate passes: `mise run check`

#### Manual Verification

- [x] `_require_unprivileged` reads as a deliberate divergence from the
      neighbouring `skipif`, so a later reader does not "fix" it back

---

## Phase 2: Split the Probe and Relocate It Off the Warm Path

### Overview

The trace seam, the trace helpers, the production change and its eight
regression cases. Write the cases first, observe each behaving as recorded
below, then make them pass.

### Changes Required

#### 1. Trace capture seam

**File**: `tests/integration/support/installation.py`
**Changes**: Add a keyword-only `xtrace` flag to `run_bootstrap`, translate it
into the interpreter flag inside the funnel, and default `PS4` alongside it so
the capability is cohesive at the seam rather than split between a flag here and
an env var each consumer must remember. A narrow boolean rather than an open
`bash_args` passthrough: bash treats the first non-option operand as the script,
so an arbitrary argument list could demote the validated `entry` to `$1` and
defeat the `assert_hermetic` precondition this funnel exists to enforce. The
`False` default keeps the shared skill-invocation consumer unaffected.

```python
def run_bootstrap(
    root: Path,
    server: Path,
    downloader: Path,
    *,
    args: tuple[str, ...] = (),
    xtrace: bool = False,
    extra_env: dict[str, str] | None = None,
    path: str | None = None,
    entry: Path | None = None,
    cwd: Path | None = None,
    timeout: float | None = None,
) -> subprocess.CompletedProcess[str]:
```

In the body, before the `with`:

```python
    bash_flags = ("-x",) if xtrace else ()
    if xtrace:
        env.setdefault("PS4", "+${FUNCNAME[0]:-main}:")
```

and at the `subprocess.run` call:

```python
                [BASH, *bash_flags, str(entry), *args],
```

Extend the existing docstring with one clause naming `xtrace`: tracing is
per-call because the trace lands on stderr alongside the diagnostics every other
case asserts on; `PS4` defaults to `+${FUNCNAME[0]:-main}:` because a bare
`${FUNCNAME[0]}` is unbound at top level under the bootstrap's `set -u`; and
`SHELLOPTS` is rejected because it is exported into bash descendants, and the
probe's `#!/bin/sh` is bash on macOS but dash on Linux, which would make trace
content lane-dependent.

#### 2. Trace helpers and a mode-restoring context manager

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Add `import re` and `import contextlib` to the module imports, then
open the new section (see Change 3) and place these immediately under its
divider, ahead of the cases — matching the file's section-local helper
convention at `:334-383` and `:706-760`.

```python
_PROBE_FN = "probe_exec_capable"
_ENSURE_FN = "ensure_dir"


@contextlib.contextmanager
def _restricted(path: Path, mode: int) -> Iterator[None]:
    """Apply a mode and always restore it: `tmp_path` teardown cannot remove an
    unwritable directory, and an advisory-permission filesystem (a bind mount,
    WSL drvfs, an inherited ACL) would silently void the case, so the mode is
    verified to have bitten.
    """
    path.chmod(mode)
    try:
        assert not os.access(path, os.W_OK), (
            f"chmod {mode:#o} did not take effect on {path}; permission bits "
            "appear advisory on this filesystem — exclude it explicitly"
        )
        yield
    finally:
        path.chmod(0o755)


def _traced(
    harness: Harness,
    downloader: Path,
    *,
    extra_env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        xtrace=True,
        extra_env=extra_env,
        timeout=30,
    )


def _entered(trace: str, function: str) -> bool:
    pattern = rf"^\++{re.escape(function)}:"
    return re.search(pattern, trace, re.MULTILINE) is not None


def _probe_execs(trace: str) -> int:
    # The `:/` anchor is load-bearing: the function's own
    # `probe=/…/.accelerator-probe-<pid>` assignment is traced too, and a looser
    # pattern matches it — passing on an implementation that never execs.
    # Counting rather than searching also pins the idempotence flag.
    pattern = rf"^\++{re.escape(_PROBE_FN)}:/\S*\.accelerator-probe-\d+$"
    return len(re.findall(pattern, trace, re.MULTILINE))
```

The `_restricted` mode check is what the work item's acceptance-criteria
preamble asks for beyond `id -u`: on a filesystem where `chmod` is advisory the
permission cases would otherwise fail (or, worse for the success-asserting case,
pass) with no hint that the environment rather than the product was at fault.

**Deviation taken during implementation**: the single `not os.access(path,
os.W_OK)` assertion above is wrong for the two `0o666` cases. `0o666` keeps the
owner write bit and clears only the search bit, so the assertion fires on a
correctly-restricted directory — observed as a red on
`test_cold_path_keeps_the_noexec_diagnostic` and
`test_warmed_then_non_executable_cache_keeps_the_diagnostic` for the harness,
not the product. Implemented instead as a loop over the two owner bits, checking
`W_OK` only when the mode clears `0o200` and `X_OK` only when it clears `0o100`,
which is what the docstring's "verified to have bitten" always meant and covers
both shapes (`0o555` drops write, `0o666` drops search).

#### 3. Regression cases (written first)

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Eight cases under a new section divider.

```python
# ── Exec probe: cold-path only ───────────────────────────────────────────────


def test_warm_path_survives_a_non_writable_cache_dir(
    make_harness: Callable[..., Harness],
    downloader: Path,
    launcher_bin: Path,
) -> None:
    # Equality against a direct launcher run proves the cached binary is the one
    # the fixture built. It pins the vergen commit/build stamps too, so a
    # relink between fixture setup and this call would fail it.
    _require_unprivileged()
    harness = make_harness(real_launcher=True)
    root, server = harness.root, harness.server
    warm = _run_bootstrap(root, server, downloader, args=("version",))
    assert warm.returncode == 0, warm.stdout + warm.stderr
    with _restricted(root / "bin", 0o555):
        result = _run_bootstrap(root, server, downloader, args=("version",))
    output = result.stdout + result.stderr
    assert result.returncode == 0, output
    direct = subprocess.run(
        [str(launcher_bin), "version"],
        capture_output=True,
        text=True,
        check=True,
    )
    assert result.stdout == direct.stdout, output


def test_warm_path_does_not_enter_the_probe(
    make_harness: Callable[..., Harness], downloader: Path
) -> None:
    harness = make_harness()
    warm = _run_bootstrap(harness.root, harness.server, downloader)
    assert warm.returncode == 0, warm.stdout + warm.stderr
    traced = _traced(harness, downloader)
    assert traced.returncode == 0, traced.stdout + traced.stderr
    trace = traced.stderr
    # `verify_launcher` bounds the trace from the far end: `ensure_dir` alone
    # only proves the run reached `resolve_cache_dir`, which is upstream of both
    # gates, so a truncated trace would satisfy the negative assertion.
    assert _entered(trace, _ENSURE_FN), trace
    assert _entered(trace, "verify_launcher"), trace
    assert not _entered(trace, _PROBE_FN), trace


def test_cold_path_enters_and_executes_the_probe(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    harness = make_harness()
    cache = tmp_path / "fresh-cache"
    result = _traced(
        harness, downloader, extra_env={"ACCELERATOR_CACHE_DIR": str(cache)}
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert cache.is_dir(), "the always-run ensure_dir half must create it"
    assert _entered(result.stderr, _PROBE_FN), result.stderr
    # Exactly one: a cold run reaches both gates, so a broken idempotence flag
    # would probe twice.
    assert _probe_execs(result.stderr) == 1, result.stderr
    assert not list(cache.glob(".accelerator-probe-*")), "probe not cleaned up"


def test_cold_happy_path_creates_a_missing_cache_dir(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    harness = make_harness(real_launcher=True)
    cache = tmp_path / "absent" / "cache"
    result = _run_bootstrap(
        harness.root,
        harness.server,
        downloader,
        args=("version",),
        extra_env={"ACCELERATOR_CACHE_DIR": str(cache)},
    )
    output = result.stdout + result.stderr
    assert result.returncode == 0, output
    assert cache.is_dir(), output
    assert result.stdout.startswith("accelerator "), output


def test_cold_path_keeps_the_noexec_diagnostic(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    _require_unprivileged()
    harness = make_harness()
    cache = tmp_path / "noexec-cache"
    cache.mkdir()
    with _restricted(cache, 0o666):
        result = _run_bootstrap(
            harness.root,
            harness.server,
            downloader,
            extra_env={"ACCELERATOR_CACHE_DIR": str(cache)},
        )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "no writable, exec-capable cache directory" in output, output
    assert str(cache) in output, output
    assert "is not writable" in output, output


def test_warmed_then_non_executable_cache_keeps_the_diagnostic(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    _require_unprivileged()
    harness = make_harness()
    cache = tmp_path / "warm-cache"
    cache.mkdir()
    env = {"ACCELERATOR_CACHE_DIR": str(cache)}
    first = _run_bootstrap(
        harness.root, harness.server, downloader, extra_env=env
    )
    assert first.returncode == 0, first.stdout + first.stderr
    with _restricted(cache, 0o666):
        result = _run_bootstrap(
            harness.root, harness.server, downloader, extra_env=env
        )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "no writable, exec-capable cache directory" in output, output


def test_unverifiable_launcher_in_readonly_cache_fails_fast(
    make_harness: Callable[..., Harness],
    downloader: Path,
    tmp_path: Path,
    host_platform: str,
) -> None:
    # 0o555, not 0o666: keeping the search bit means the cached artefacts still
    # stat and the staged shim still hashes equal, so staging is skipped and
    # verification is genuinely reached. `timeout` sits between the sub-second
    # pass and the ~30s lock-timeout budget the gate prevents.
    _require_unprivileged()
    harness = make_harness()
    cache = tmp_path / "readonly-cache"
    cache.mkdir()
    env = {"ACCELERATOR_CACHE_DIR": str(cache)}
    first = _run_bootstrap(
        harness.root, harness.server, downloader, extra_env=env
    )
    assert first.returncode == 0, first.stdout + first.stderr
    launcher = cache / f"accelerator-launcher-{_VERSION}-{host_platform}"
    launcher.write_bytes(b"poisoned")
    with _restricted(cache, 0o555):
        result = _run_bootstrap(
            harness.root, harness.server, downloader, extra_env=env, timeout=15
        )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "no writable, exec-capable cache directory" in output, output


def test_uncreatable_cache_dir_is_a_named_error(
    make_harness: Callable[..., Harness], downloader: Path, tmp_path: Path
) -> None:
    # The cause clause is what distinguishes this site from the two probe
    # gates, which emit the same leading substring.
    _require_unprivileged()
    harness = make_harness()
    parent = tmp_path / "readonly-parent"
    parent.mkdir()
    with _restricted(parent, 0o555):
        result = _run_bootstrap(
            harness.root,
            harness.server,
            downloader,
            extra_env={"ACCELERATOR_CACHE_DIR": str(parent / "nested")},
        )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "no writable, exec-capable cache directory" in output, output
    assert "could not be created" in output, output
```

The two `0o666` cases clear the directory's search bit, which blocks *all* name
resolution inside it — so the probe fails at its **write** step and its exec
branch is never evaluated. No directory-permission combination can produce
exec-without-write, so the exec half is covered only by
`test_cold_path_enters_and_executes_the_probe`'s execution assertion, and
Phase 4 records that limitation. Both cases are deterministic on both lanes:
`ensure_dir`'s `[[ -d ]]` guard means `mkdir` never runs on an existing
directory, so no `mkdir` implementation difference is reachable.

#### 4. Split `probe_dir`

**File**: `bin/accelerator`
**Changes**: Replace `probe_dir` (`:166-180`) with two functions. Tab-indented,
matching the file. `probe_exec_capable` returns 1 for a write or `chmod`
failure and 2 for an exec failure, so the caller can report the cause it
actually detected; every exit path removes the probe file.

```bash
# Kept a named function because a warm-path trace assertion uses the token to
# prove the always-run half still runs; the -d guard keeps a warm call from
# forking mkdir for a directory the launcher is about to be exec'd out of.
ensure_dir() {
	[[ -d "$1" ]] || mkdir -p "$1" 2>/dev/null
}

# Catches a noexec mount, which a write-only check would pass. Cold-path only:
# a warm call proves the same capability for real by running the staged shim
# and then the launcher out of this directory. 1 = cannot write, 2 = cannot
# exec.
probe_exec_capable() {
	probe="$1/.accelerator-probe-$$"
	if ! printf '#!/bin/sh\nexit 0\n' >"${probe}" 2>/dev/null ||
		! chmod +x "${probe}" 2>/dev/null; then
		rm -f "${probe}"
		return 1
	fi
	"${probe}" >/dev/null 2>&1
	status=$?
	rm -f "${probe}"
	[[ "${status}" -eq 0 ]] || return 2
}
```

Verified against the project's gates and behaviourally: parses under bash
3.2.57, passes `lint-bashisms.sh`, ShellCheck-clean, shfmt-stable, and returns
0/1/2 with no probe file left behind in any of the three outcomes.

#### 5. Reduce `resolve_cache_dir` to the always-run half

**File**: `bin/accelerator`
**Changes**: At `:184-193`, swap both `probe_dir` calls for `ensure_dir`,
keeping the existing comment.

```bash
# ${plugin_root}/bin or the ACCELERATOR_CACHE_DIR override; no XDG fallback
# (an XDG-resident binary would break the allowed-tools glob match).
resolve_cache_dir() {
	if [[ -n "${ACCELERATOR_CACHE_DIR:-}" ]]; then
		ensure_dir "${ACCELERATOR_CACHE_DIR}" || return 1
		printf '%s\n' "${ACCELERATOR_CACHE_DIR}"
		return 0
	fi
	primary="${plugin_root}/bin"
	ensure_dir "${primary}" || return 1
	printf '%s\n' "${primary}"
}
```

#### 6. Give the diagnostic one definition and three causes

**File**: `bin/accelerator`
**Changes**: Define the helper **above** the `resolve_cache_dir` call site and
route every site through it. The substring the tests assert lives in one place,
each site supplies the cause it detected, and the `ACCELERATOR_CACHE_DIR`
remediation hint the pre-change wording carried is preserved. Note the
pre-change message named `${plugin_root}/bin` even when an override was the
thing that failed — the expansion below fixes that.

```bash
fail_no_cache_dir() {
	fail "no writable, exec-capable cache directory: $1 $2; set \
ACCELERATOR_CACHE_DIR to a writable, exec-capable directory (no XDG fallback)"
}

cache_dir=$(resolve_cache_dir) ||
	fail_no_cache_dir "${ACCELERATOR_CACHE_DIR:-${plugin_root}/bin}" \
		"could not be created"
```

#### 7. Add the two probe gates

**File**: `bin/accelerator`
**Changes**: Introduce the gate after `cache_dir` is resolved and below the
dev-override block, then call it from two places. `probed` is set only after a
successful probe, so the flag means what its name says rather than depending on
`fail` never returning.

```bash
probed=""

require_exec_capable_cache() {
	[[ -z "${probed}" ]] || return 0
	probe_exec_capable "${cache_dir}"
	case "$?" in
	0) probed=1 ;;
	2) fail_no_cache_dir "${cache_dir}" \
		"rejected an executable file — possibly a noexec mount" ;;
	*) fail_no_cache_dir "${cache_dir}" "is not writable" ;;
	esac
}
```

The staging gate goes inside the existing `if` at `:255`, as the first
statement of the body:

```bash
if [[ ! -x "${shim}" ]] ||
	[[ "$(sha256_file "${shim}" 2>/dev/null)" != "${shim_digest}" ]]; then
	# This body is the first required write into the cache dir, so gating here
	# covers every write. Anything that writes there earlier needs its own gate.
	require_exec_capable_cache
	cp "${shim_source}" "${shim}" 2>/dev/null ||
```

The cold-branch gate goes at the top of the `else` at `:339`:

```bash
else
	# Staging was skipped (the staged shim already matched) but verification
	# failed. Without this, an unwritable cache dir reaches acquire_lock, whose
	# mkdir can never succeed and whose loop treats a missing pid file as an
	# imminent competitor — burning its whole timeout budget and reporting a
	# lock timeout instead of the real cause.
	require_exec_capable_cache
	acquire_lock
```

#### 8. Pin the two function names cheaply

**File**: `tests/unit/tasks/test_bootstrap_coverage.py`
**Changes**: Two presence assertions, using the file's existing
`assert _KEY in _BOOTSTRAP_SRC.read_text()` idiom. The trace cases assert on
`ensure_dir` and `probe_exec_capable` by name; without this, a rename fails only
after a cargo build and a full fetch-verify-cache round trip, reporting "probe
not entered" rather than "the name moved" — and silently voids the warm-path
negative assertion in the meantime.

### Success Criteria

#### Automated Verification

- [x] Recorded during the red step, per case — the shape differs and the record
      should say so rather than claim a uniform red. Observed 2026-08-03,
      exactly as predicted: 5 failed, 3 passed. **Red before the change**:
      `test_warm_path_survives_a_non_writable_cache_dir` (today's fatal probe
      fails its write under `0o555`); `test_warm_path_does_not_enter_the_probe`
      and `test_cold_path_enters_and_executes_the_probe` (the new function
      names do not exist yet); `test_uncreatable_cache_dir_is_a_named_error`
      (the `could not be created` cause clause does not exist yet).
      **Green before and after** — preservation guards, which is a property
      worth recording rather than a defect:
      `test_cold_happy_path_creates_a_missing_cache_dir`,
      `test_cold_path_keeps_the_noexec_diagnostic` (its cause assertion is new,
      so it reds on that clause only),
      `test_warmed_then_non_executable_cache_keeps_the_diagnostic`, and
      `test_unverifiable_launcher_in_readonly_cache_fails_fast` — today's
      unconditional probe already produces its asserted exit and substring.
- [x] **After the change**, confirm by mutation which case guards which gate,
      and record it. Both mutations behaved exactly as predicted (2026-08-03):
      deleting the staging gate reds
      `test_cold_path_keeps_the_noexec_diagnostic`,
      `test_warmed_then_non_executable_cache_keeps_the_diagnostic` and
      `test_readonly_root_without_override_is_a_named_error`; deleting the
      cold-branch gate reds
      `test_unverifiable_launcher_in_readonly_cache_fails_fast` via its
      timeout. This is the only step that demonstrates the cold-branch gate is
      guarded at all, since that case passes before the change.
- [x] Entrypoint suite passes: `mise run test:integration:entrypoint` (54)
- [x] Skill-invocation suite unaffected:
      `mise run test:integration:skill-invocation` (128)
- [x] Launcher-edge guards still hold:
      `uv run pytest tests/unit/tasks/test_mise.py`
- [x] Bootstrap coverage guard passes, including the two new name assertions:
      `uv run pytest tests/unit/tasks/test_bootstrap_coverage.py`
- [x] Python format, lint and types pass: `mise run build-system:check`
- [x] Shell format, lint, bashisms and exec-bits pass:
      `mise run scripts:check` (which folds `lint:scripts:bashisms:check`)
- [x] `mise run check` is green

#### Manual Verification

- [x] A tampered cached launcher in an unwritable cache dir fails within a
      second, not after the ~30 s lock budget — measured **27 ms**, ending in
      `no writable, exec-capable cache directory: … is not writable`
- [x] The cold-run trace shows `probe_exec_capable` entered *and* the probe file
      executed as its own command word exactly once (1 match), with the
      `probe=` assignment line present in the same trace and **not** matched by
      the anchored pattern. `+main:require_exec_capable_cache` appears twice
      and the probe runs once, so the idempotence flag is exercised
- [x] Warm trace shows `ensure_dir` and `verify_launcher` but no
      `probe_exec_capable`
- [ ] Both interpreters are covered without extra work: the harness pins
      `/bin/bash`, which is 3.2.57 on darwin and 5.2 on `ubuntu-latest`, so the
      two trace cases run under both on every CI run. Confirm the ubuntu lane
      green on them specifically rather than assuming (Phase 4 records it)

---

## Phase 3: Documentation

### Overview

Correct the statements this change falsifies, record the newly supported
read-only cache with its real limit, and stop the staging comment misdirecting
the next latency hunt.

### Changes Required

#### 1. Internals documentation

**File**: `docs/internals.md`
**Changes**: Replace the paragraph at `:207-212` with two paragraphs. The
existing line 209 straddles a sentence boundary — the release-base-URL guidance
begins on it — so the replacement must carry that sentence forward verbatim or
it is orphaned. The trust guidance stays unconditional: the shim and the
launcher are executed from that directory on every call, warm or cold, so
ownership and group-writability matter permanently rather than only until the
cache is populated. Wrapped at 80 columns.

> Both are trust-root inputs rather than ordinary conveniences. The cache
> directory is where signed binaries are staged and executed from, so point it
> at a directory you own and that is not group-writable. The release base URL
> should be a host you trust not to serve an older signed release: the cache
> key carries no content hash, so a mirror can hand back an older
> validly-signed launcher for the current version.
>
> The bootstrap needs that directory to be writable and executable on a *cold*
> start — one where it has no verified launcher cached, which includes the first
> run after a version bump and any run where verification fails. It writes and
> *executes* a probe file there to check. A *warm* start neither writes nor
> probes; it runs the already-staged verifier and launcher instead, so a cache
> directory populated once may afterwards be read-only for warm bootstrap
> invocations. That exemption stops at the bootstrap: running any subcommand
> that dispatches to a separate binary makes the launcher probe the same
> directory, and that probe writes — so a permanently read-only cache directory
> is only viable if you never use those subcommands.

#### 2. Bootstrap header contract

**File**: `bin/accelerator`
**Changes**: The header block at `:1-20` records this file's contract and is the
first thing a reader of the changed code sees. It says nothing about the cache
directory's capability requirements, which this change makes asymmetric and
which `docs/internals.md` now promises. Add one clause: the cache directory must
be writable and exec-capable on a cold start (probed once); a warm start only
reads and execs from it, so a populated cache directory may be read-only.

#### 3. Staging comment

**File**: `bin/accelerator`
**Changes**: At `:246-251` the comment says a warm call "re-hashes (cheap)".
With the probe gone that re-hash is the largest remaining warm-path cost and
0169's latency criterion turns on it, so the parenthetical now misdirects.
Replace that clause with wording that records why the cost is retained, naming
the guarding tests by **function name** rather than a line range (line ranges go
stale, and this plan itself adds eight cases to that file):

> …a warm call re-hashes instead of re-copying 465KB — now the largest
> remaining warm-path cost, retained deliberately because the digest check is
> what makes a planted stub get re-staged rather than trusted by name
> (`test_planted_staged_shim_rehashed_then_succeeds`,
> `test_planted_staged_shim_is_not_trusted`,
> `test_planted_staged_shim_via_cache_dir_is_not_trusted`)…

Also correct `475KB` to `465KB` while the line is open — the shipped shim
measures 465,568 bytes.

**Deviation taken during implementation: not applied — the plan's correction is
wrong.** 465,568 bytes is the **linux-x64** shim. The four vendored shims
measure 486,672 (darwin-arm64), 496,896 (darwin-x64), 426,400 (linux-arm64) and
465,568 (linux-x64) bytes, so the figure is per-triple and the original `475KB`
was already right for the shipped darwin-arm64 shim (486,672 B = 475.3 KiB).
Written as `~475KB` instead, since the comment is generic across triples.

#### 4. Changelog

**File**: `CHANGELOG.md`
**Changes**: An entry under the open `## [Unreleased]` / `### Changed` section,
led by the observable effect. Deliberately **without** before/after
milliseconds: Phase 4 is what measures them, and the ~150 ms figure in
circulation is the pre-0182 reference the work item explicitly disclaims as a
gate input. A ratio is safe to state because the gate enforces one.

> - **Warm `accelerator` invocations are substantially faster** — better than
>   halved on macOS, so session start and every skill's live-context command are
>   noticeably quicker. The bootstrap now tests the cache directory only on a
>   cold start; a `noexec` cache directory still fails with the same named
>   error, and a cache directory populated once may afterwards be read-only for
>   warm invocations (dispatching a subcommand to a separate binary still
>   needs it writable).

### Success Criteria

#### Automated Verification

- [x] Shell format and lint pass: `mise run scripts:check` — changes 2 and 3
      edit `bin/accelerator`, which `tasks/shared/sources.py:110` puts in
      shfmt/ShellCheck/bashisms/exec-bits scope

There is no markdown formatter or linter anywhere in the task tree, so the
Markdown edits have no automated gate; the 80-column convention for Markdown is
maintained by hand.

#### Manual Verification

- [x] `docs/internals.md` no longer claims the probe runs on every invocation,
      both paragraphs read end-to-end as complete prose, the release-base-URL
      sentence survives verbatim, the trust guidance is not narrowed to cold
      starts, and every line is within 80 columns (the only over-80 lines in
      the section are the pre-existing variable table)
- [x] The read-only-cache statement names the dispatch limitation in both
      `docs/internals.md` and `CHANGELOG.md`
- [x] The changelog entry sits under the existing `## [Unreleased]` /
      `### Changed` heading, describes user-visible behaviour, and states no
      figure Phase 4 has not measured
- [x] The staging comment no longer calls the re-hash "cheap" without
      qualification, and cites test function names rather than line numbers

---

## Phase 4: Measurement and Closeout

### Overview

Take both medians on one host in one session, record them and the coverage
limitations in the work item's Validation Results, correct the stale residual
figures wherever they appear, raise the follow-ups this work surfaced, and
re-confirm 0169's hand-off note. No production source changes.

### Changes Required

#### 1. Measurement

**Method**: authored fresh. Acceptance criterion 9 specifies "a bash loop over
20 runs taking the median, matching how the Context table was produced"; this
deviates deliberately and the deviation is recorded below, because that method
cannot produce a usable figure. Two constraints drove the design.

First, the clock must be read from a single process: bracketing each call with
two `python3 -c` invocations puts a whole interpreter startup *inside* the
measured interval, which on darwin is comparable to the ~41 ms being measured
and biases the ratio toward failing a correct implementation. Second, the
before/after variants must not be separated by a working-copy swap: taking one
batch immediately after `jj new` rewrites and snapshots the tree aliases
fsevents and file-watcher drift onto the result.

So: copy the pre-change script alongside the new one and interleave the two
sample-by-sample in a single revision, alternating which goes first so no
variant is permanently second. `dir_of(self)/..` resolves the same plugin root,
`plugin.json` gives the same `version`, and the cached launcher path is
therefore identical — both variants provably exercise the same warm cache and
the same binary. `v1.24.0-pre.21`'s signed `accelerator-darwin-arm64` asset is
published and post-0182, so one warming call caches a genuine release binary.

The copy is named `bin/.tmp-accelerator-before` because `.gitignore:48` already
covers `bin/.tmp-*`. This matters more than tidiness: jj snapshots the working
copy on virtually any command, so an unignored executable copy of a superseded
trust-root bootstrap could be committed into `bin/` by any concurrent `jj`
invocation during the run.

```bash
#!/usr/bin/env bash
set -euo pipefail
before=bin/.tmp-accelerator-before   # bin/.tmp-* is already gitignored
jj file show -r "$1" bin/accelerator >"${before}" ||
	git show "$1:bin/accelerator" >"${before}"
chmod +x "${before}"
trap 'rm -f "${before}"' EXIT
bin/accelerator version >/dev/null   # warm the shared cache
ls -li bin/accelerator-launcher-*    # provenance, both sides share this
python3 - "${before}" <<'PY'
import statistics, subprocess, sys, time

VARIANTS = [("before", sys.argv[1]), ("after", "bin/accelerator")]
N = 50


def timed(argv):
    t = time.perf_counter()
    p = subprocess.run(argv, capture_output=True, text=True)
    return (time.perf_counter() - t) * 1000, p


def sample(path):
    dt, p = timed([path, "version"])
    if p.returncode != 0 or not p.stdout.startswith("accelerator "):
        raise SystemExit(
            f"invalid sample from {path}: rc={p.returncode} {p.stderr}"
        )
    return dt


# Two floors: /usr/bin/true isolates this harness's own per-call cost, and a
# trivial bash script adds the interpreter startup that is inside both variants.
floor_exec = statistics.median(timed(["/usr/bin/true"])[0] for _ in range(20))
with open("bin/.tmp-accelerator-floor", "w") as fh:
    fh.write("#!/usr/bin/env bash\nexit 0\n")
subprocess.run(["chmod", "+x", "bin/.tmp-accelerator-floor"], check=True)
floor_bash = statistics.median(
    timed(["bin/.tmp-accelerator-floor"])[0] for _ in range(20)
)

samples = {name: [] for name, _ in VARIANTS}
for i in range(N):
    order = VARIANTS if i % 2 == 0 else list(reversed(VARIANTS))
    for name, path in order:
        samples[name].append(sample(path))

print(f"harness floor (/usr/bin/true):     {floor_exec:7.2f} ms")
print(f"+ bash interpreter startup:        {floor_bash:7.2f} ms")
for name, xs in samples.items():
    if len(xs) != N:
        raise SystemExit(f"{name}: {len(xs)} samples, expected {N}")
    xs.sort()
    med = statistics.median(xs)
    p90 = xs[-(-9 * len(xs) // 10) - 1]
    print(
        f"{name:7s} min {xs[0]:7.2f}  median {med:7.2f}  p90 {p90:7.2f}  "
        f"median-minus-harness-floor {med - floor_exec:7.2f}  n={len(xs)}"
    )
PY
rm -f bin/.tmp-accelerator-floor
```

`/usr/bin/true` is timed without the output check — it prints nothing, so
validating stdout there would abort the run before a single sample. Its median
must sit within a millisecond or two of a bare fork+exec; if it does not, the
harness is measuring itself and the medians are not usable. The bash floor is
the second calibration point: it is on the same critical path as both variants,
so it converts part of the otherwise-unattributed residual into a measured term.

`median − harness floor` is the figure to compare against the composition
budget below; the raw median is the figure for the ratio gate, where the floor
appears on both sides and cancels.

**Gate**: `after ≤ 0.5 × before`. The delta is recorded but not gating. The
gate has roughly 33 ms of slack against the ~41 ms expectation, so a pass alone
does not prove the probe fully left — the composition check is what does.

**Composition budget** to check the after-median against (all figures measured
on this host unless marked): bash interpreter startup (measured by the bash
floor), `uname -m` + `uname -s` as two substitutions, `sed` over `plugin.json`,
two `cd -P`/`pwd -P` subshells for plugin-root resolution, the
`resolve_cache_dir` command substitution, two `sha256_file` calls at ~4.2 ms
each (**backend-dependent** — ~13.0 ms each if `shasum` resolves), the staged
shim exec plus minisign at ~2.3 ms, and the launcher exec at ~3.0 ms. Record
the observed total against this list; if the unexplained remainder exceeds ~25%
of the after-median, attribute it before recording — a `bash -x` run with
per-line timestamps is enough.

**Re-derive the headline attribution while the harness is set up.** The plan's
~108 ms probe cost and its ~97 ms "first-exec check" split are inherited from
the work item's Context table, whose methodology was not recorded and whose
"re-exec of a pre-existing probe file | 10.6 ms" row is seven times this
harness's measured 1.48 ms fork+exec floor. Time three things directly: exec of
a freshly written probe script, re-exec of the same file untouched, and the same
probe in `/tmp` versus the repo `bin/`. Record whether the host runs an
EndpointSecurity or anti-malware agent. This matters because a ~100 ms
first-exec penalty is more characteristic of a scanning agent than of macOS
generally — and if it is host-specific, the ratio gate is unsatisfiable on hosts
without it and the extrapolation to the launcher-side probe does not transfer.

#### 2. Validation Results

**File**: `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md`
**Changes**: Replace each _pending_ entry, naming the test function that
discharges each behavioural criterion so the record traces to a permanent
guard:

- Warm-path exec-probe-free check —
  `test_warm_path_survives_a_non_writable_cache_dir`.
- Direct probe-absence check — `test_warm_path_does_not_enter_the_probe`.
- Positive control — `test_cold_path_enters_and_executes_the_probe`.
- `noexec` cold-path check — `test_cold_path_keeps_the_noexec_diagnostic`.
- Cold happy-path (`ensure_dir`) check —
  `test_cold_happy_path_creates_a_missing_cache_dir`.
- Diagnostic preserved on a warmed-then-non-executable cache —
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic`. Note this is a
  code-path duplicate of the `noexec` cold-path case: clearing the search bit
  makes the staged shim unresolvable either way, so the populated cache has no
  observable effect. The genuinely distinct warmed-then-unusable scenario is the
  `0o555` case below.
- Beyond the criteria: `test_unverifiable_launcher_in_readonly_cache_fails_fast`
  (the cold-branch gate and its anti-hang property — the only cover for either)
  and `test_uncreatable_cache_dir_is_a_named_error` (the retained
  `resolve_cache_dir` failure, pinned by its `could not be created` cause).

And record:

- **Both medians, the delta, both instrument floors, min/median/p90, host model
  and OS version**, plus launcher provenance with `ls -li` confirming both
  variants used the same post-0182 binary.
- **The measured composition** against the budget above, and **which sha256
  backend `command -v sha256sum` resolved to**, with the macOS version. This is
  load-bearing for 0169: `$(sha256sum … | awk …)` measured 4.22 ms on darwin
  25.3 where `/sbin/sha256sum` exists, against 12.97 ms for the Perl `shasum`
  fallback — so the two-hash residual is **~8.4 ms or ~26 ms depending on the
  host**, and the ~11.7 ms figure previously quoted describes the fallback. Hand
  0169 the range and the resolved backend, not a point estimate. Recording
  `command -v sha256sum` on both CI lanes costs nothing and says whether the
  fallback is reachable in CI at all.
- **The re-derived probe attribution** from the third measurement above, with
  the host's security-agent status.
- **The lanes observed green** (darwin and linux), from the `test-integration`
  matrix job in `.github/workflows/main.yml:55-91`. Confirmed to contain no
  `container:` key, so both lanes run unprivileged and **no exclusion is
  expected**. Record that the ubuntu lane's `/bin/bash` is 5.2, so the two trace
  cases are covered on both interpreters by the standard CI run. If a root lane
  is ever introduced, the escape to add is a registered `unprivileged` pytest
  marker, so exclusion stays explicit and greppable rather than a silent skip.
- **The exec-vs-write coverage limitation**: both `noexec` criteria create their
  failure by clearing a directory's search bit, which blocks name resolution and
  so fails the probe's *write* step — its exec branch is never evaluated, and no
  directory-permission combination can produce exec-without-write. The exec half
  is covered instead by the positive control's assertion that the probe file is
  executed as its own command word. Both cases are lane-deterministic:
  `ensure_dir`'s `[[ -d ]]` guard means `mkdir` never runs on an existing
  directory, so no BSD/GNU `mkdir -p` difference is reachable. A genuine
  `mount -o noexec` filesystem would close the gap and is explicitly out of
  scope.
- **The PS4 deviation**: criterion 2 mandates `PS4='+${FUNCNAME[0]}:'`, which is
  broken under `set -u` on bash 3.2 — it emits `FUNCNAME[0]: unbound variable`
  per top-level command and leaves PS4 unexpanded, and aborts a non-interactive
  bash 5 outright. Implemented as `'+${FUNCNAME[0]:-main}:'`, defaulted inside
  `run_bootstrap` when `xtrace` is set.
- **The criterion-1 amendment**: the harness's default launcher prints nothing
  and the real one prints `CARGO_PKG_VERSION`, not the fixture's `9.9.9-test`,
  so "the version the harness fixture builds" was unachievable as literally
  worded. Implemented against the real launcher, asserting stdout **equality**
  with a direct `launcher_bin version` run.
- **The criterion-3 split**: the criterion asks the positive control to ride on
  criterion 6's cold happy-path run. Implemented as its own traced cold run
  instead, because criterion 6's run uses the real launcher and is not traced.
- **The criterion-9 method deviation**: 50 interleaved samples from a single
  Python process with `perf_counter`, two instrument floors and
  order-alternation, rather than a bash loop over 20. Reasons: a per-call
  `python3` clock read puts an interpreter startup inside the interval; batching
  either side of a `jj` working-copy swap aliases drift onto the difference; and
  a fixed within-pair order biases whichever variant is always second. Note the
  Context table's figures are therefore not method-comparable to these medians.
- **The two probe call sites**: the staging gate covers every write into
  `cache_dir` (the staging body being the first such write, with the two benign
  exceptions named in Key Discoveries); the cold-branch gate covers the
  verification-failed residual and prevents a ~30 s `acquire_lock` spin. Record
  the reasoning so it survives anyone tempted to collapse them, and record the
  post-change mutation results from Phase 2 showing which case guards which.
- **The gate slack**: `after ≤ 0.5 × before` leaves ~33 ms of headroom against
  the ~41 ms expectation, so the composition check is what confirms the probe
  left.

#### 3. Correct the stale residual figures in 0186 itself

**File**: `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md`
**Changes**: The ~11.7 ms / ~23 ms figures appear three more times in this work
item — Dependencies ("~11.7 ms for the second `sha256_file` alone, or ~23 ms if
staging were skipped entirely"), Assumptions ("while the ~11.7 ms cost
remains"), and the double-hash entry in Validation Results. 0169's corrected
hand-off note will point readers back at exactly these, so correct them here
too, with the measured figure, its backend dependence and the range.

#### 4. Follow-ups to raise

**Changes**: Create work items and cross-reference them from 0186's Validation
Results.

- **The launcher runs the same probe on every external-subcommand dispatch.**
  `cache_root::resolve` — and therefore `probe_writable_and_executable`, which
  writes, `chmod`s, execs and removes `.accelerator-probe-<pid>` — runs from
  `LazyProductionResolver::resolve` (`cli/launcher/src/main.rs:65`) *before*
  the sub-binary cache-hit test. Built-ins (`version`, `config`) never reach
  it, which is why today's SessionStart hook escapes and why this plan's
  `version` measurement cannot see it. 0169 serves `vcs guard` as a dispatched
  sub-binary, so it pays a fresh-file first-exec penalty of the same shape. The
  same `ensure_dir` / lazy-probe split applies, gated on the cache miss inside
  `FetchVerifyCacheResolver::resolve`. This is also what makes a read-only cache
  directory unusable for dispatched subcommands, which Phase 3 documents.
  **Necessary but not sufficient for 0169**: that item's own hand-off note
  records the bootstrap alone landing near 41 ms against a ≈38.6 ms gate, so the
  threshold decision is 0169's regardless and must not be deferred pending this.
  While the measurement harness is set up, time
  `probe_writable_and_executable` directly so the follow-up carries a figure
  rather than an extrapolation.
- **`acquire_lock` cannot distinguish contention from an unusable directory.**
  The loop treats "no pid file" as "a competitor is about to write one" and has
  no notion of an unrecoverable `mkdir`, so it burns its full budget. There is a
  worse arm: when a pid file names a dead process, `:291-294` does
  `rm -f`/`rmdir` then `continue` with no `sleep` and no `waited` increment — so
  if the lock directory cannot be removed (created by another user), the loop
  spins **unbounded** with no timeout at all, and neither gate prevents it (the
  probe passes on a writable cache dir whose lock directory happens to be
  foreign). The fix is small: after a failed `mkdir`, `[[ -d "${lock_dir}" ]]`
  distinguishes EEXIST from a permission failure. Scope the item to classifying
  `mkdir`/`rmdir` failures, not to re-wording the timeout.
- **Batch the two shim hashes into one `sha256sum` invocation.** Measured on
  this host at ~2.5 ms (7.05 ms for two `$(sha256sum f | awk …)` substitutions
  versus 4.57 ms for one `$(sha256sum f1 f2)` with no `awk`), with both digests
  still computed and compared so the planted-stub defence is untouched. Needs a
  branch to preserve today's short-circuit, since the second hash currently runs
  only when the staged shim is executable. ~2.5 ms is essentially 0169's whole
  ~2.4 ms shortfall, so it deserves its own before/after rather than riding
  along here. Confirm the multi-file output format and missing-file exit
  semantics on both the Apple `/sbin/sha256sum` and GNU coreutils backends.

Not raised, deliberately: a faster `sha256_file` *backend*. The native
`sha256sum` is already in use on this host, actual hashing is ~0.5 ms,
`openssl dgst` is slower, and dropping the two `awk` execs saves ~0.4 ms. If
Phase 4 finds a lane resolving the Perl fallback, reopen it.

#### 5. 0169 hand-off note

**File**: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
**Changes**: Confirm the dated note in Dependencies still holds against the
measured after-median. Correct the quoted residual — its figure derives from the
Perl-`shasum` cost, not the backend the code uses on a host with
`/sbin/sha256sum` — and hand over the measured **composition** plus the
backend-dependent range rather than a single number, together with the
launcher-side probe as the dominant unaddressed cost and the batched-hash lever
as the cheapest remaining one. **Do not change 0169's threshold or its
rationale** — that is 0169's own work.

#### 6. Work item status

**File**: `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md`
**Changes**: Tick the discharged criteria and move `status` to `complete`.

### Success Criteria

#### Automated Verification

- [ ] `mise run` is green end to end
- [ ] Work item frontmatter validates:
      `mise run test:integration:config` — the corpus validator
      (`scripts/test-validate-corpus-frontmatter.sh`) is a by-name required
      suite there (`tasks/test/integration.py:33-41`). It is **not** part of
      `mise run check`

#### Manual Verification

- [ ] Both instrument floors are within expectation; if `/usr/bin/true` does not
      read within a millisecond or two of a bare fork+exec, the medians are
      discarded and the harness fixed
- [ ] `after ≤ 0.5 × before` holds; both medians, the delta, both floors,
      min/median/p90, host, OS version and launcher provenance are recorded
- [ ] The measured composition is recorded against the budget, the resolved
      sha256 backend is named, and any unexplained remainder over ~25% is
      attributed
- [ ] The probe attribution is re-derived on this harness and the host's
      security-agent status recorded
- [ ] Both CI lanes observed green on the new cases, and which were observed is
      recorded — including the ubuntu lane's bash 5.2 coverage of the trace
      cases
- [ ] Every _pending_ entry in Validation Results is resolved, each behavioural
      one naming the test function that discharges it
- [ ] The stale ~11.7 ms / ~23 ms figures are corrected in 0186's Dependencies,
      Assumptions and Validation Results, not only in 0169's note
- [ ] All three follow-ups are raised and cross-referenced
- [ ] 0169's note re-confirmed against the measured figure and handed the
      backend-dependent range, with its threshold untouched
- [ ] `bin/.tmp-accelerator-before` and `bin/.tmp-accelerator-floor` are gone
      and `jj status` is clean of them

---

## Testing Strategy

### Unit Tests

`tests/unit/tasks/test_mise.py` and
`tests/unit/tasks/test_bootstrap_coverage.py` act as guards: no
`build:cli:dev` edge may appear, `bin/accelerator` stays
discoverable and executable, and it exports exactly `ACCELERATOR_PLUGIN_ROOT`.
Phase 2 adds two presence assertions to the latter, pinning `ensure_dir` and
`probe_exec_capable` so a rename fails in milliseconds rather than via a cargo
build and a full fetch-verify-cache round trip.

### Integration Tests

Eight new cases in
`tests/integration/entrypoint/test_accelerator_entrypoint.py`, run via
`mise run test:integration:entrypoint`. Two assert on the xtrace, six on exit
codes and diagnostics. Six are permission-dependent and hard-fail under uid 0.

Coverage of the two gates is deliberate and asymmetric:

- The **staging gate** is covered by
  `test_cold_path_keeps_the_noexec_diagnostic`,
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` (a code-path
  duplicate of the former — see Phase 4) and the pre-existing
  `test_readonly_root_without_override_is_a_named_error`, all via the probe's
  write step.
- The **cold-branch gate** is covered *only* by
  `test_unverifiable_launcher_in_readonly_cache_fails_fast`, the one case
  constructed to keep the search bit intact so the cached artefacts still stat
  and staging is skipped. Note that case passes *before* the change too, so its
  guard value is demonstrated by the post-change mutation step in Phase 2's
  criteria rather than by a red step.

The existing cases that exercise the changed region and must stay green:
`test_readonly_root_without_override_is_a_named_error` (no staged shim, so the
staging gate fires, the probe's write fails, same substring with a new cause
clause), `test_readonly_root_with_override_runs_from_override` (fresh writable
override, probe passes), `test_a_record_is_always_one_line` (probe passes,
staging still fails as today, still one line),
`test_tampered_cached_launcher_is_refused_and_healed` (staging is skipped, so
this now reaches the cold-branch gate and runs a full probe, which *passes* on
its writable cache dir — every verification failure pays one probe before
fetching, which is the intended cost), and the three planted-shim tests (all
cold, unaffected).

A trace case must never be a global mode — xtrace goes to stderr, mixed with the
bootstrap's own diagnostics, and would break every existing
`assert … in result.stderr`. That is also why the seam is a per-call `xtrace`
flag rather than `SHELLOPTS`, which is exported into bash descendants whose
identity (`#!/bin/sh`) differs by platform.

### Manual Testing Steps

1. Warm the cache, `chmod 0o555` it, run `bin/accelerator version` — exits 0
   with output identical to the launcher run directly.
2. Poison a cached launcher, `chmod 0o555` the cache dir, invoke — fails within
   a second with the cache-dir diagnostic, not after ~30 s with a lock timeout.
3. Run a cold bootstrap under `bash -x` with `PS4='+${FUNCNAME[0]:-main}:'` —
   trace shows `probe_exec_capable` entered and the probe file executed exactly
   once as its own command word, with the `probe=` assignment line present but
   unmatched.
4. Re-run warm under the same trace — `ensure_dir` and `verify_launcher`
   present, `probe_exec_capable` absent.
5. Point the cache dir at a `noexec` mount if one is available and confirm the
   cause clause reads `rejected an executable file` rather than
   `is not writable` — the one path no automated case can construct.

## Performance Considerations

The whole point. Expected warm-path landing ~41 ms against a ~149 ms pre-0182
reference, from removing ~108 ms of probe cost. That figure and its ~97 ms
"first-exec check" attribution are inherited rather than re-derived, which is
why Phase 4 re-measures them on its own harness — a ~100 ms penalty for exec'ing
a newly created *script* (not a Mach-O; no code-signing is involved, and
`/bin/sh` is already-signed) is at least as characteristic of a scanning agent
as of macOS generally, and if it is host-specific then the ratio gate is
unsatisfiable elsewhere. Linux savings will be materially smaller regardless, so
darwin is the worst case and the one measured.

The largest single residual is the shim staging condition's two `sha256_file`
calls, deliberately retained — but at a measured ~4.2 ms each on this host they
are roughly 20% of the ~41 ms landing, not the dominant term, and the figure is
backend-dependent (~26 ms total if the Perl `shasum` fallback resolves). Actual
hashing is ~0.5 ms; the rest is process startup. Phase 4 records a composition
budget precisely because the remainder — bash interpreter startup, two `uname`
substitutions, the `plugin.json` `sed`, two `cd -P`/`pwd -P` subshells and the
`resolve_cache_dir` substitution — is larger than any single named term and was
never measured.

Remaining per-call forks and execs before `exec "${launcher}"`, for whoever
picks up 0169: the `env` exec from the shebang; two `cd -P`/`pwd -P` subshells;
`sed` over `plugin.json`; `uname -m` and `uname -s` as separate substitutions;
the `resolve_cache_dir` command substitution (avoidable outright by assigning a
global instead of capturing stdout); `mkdir` only when the cache dir is absent,
after the `[[ -d ]]` guard; two `sha256_file` substitutions, each forking the
backend plus `awk`; the staged shim exec; and the launcher exec. The cheapest
untaken levers are collapsing the two `uname` calls into one `uname -sm`
(keeping both `ACCELERATOR_UNAME_S`/`_M` seams independent) and batching the two
hashes (~2.5 ms measured, raised as a follow-up).

Warm-path cost of the change itself is zero: both gates sit in branches a warm
call never enters, and the `[[ -d ]]` guard removes the `mkdir` fork the split
would otherwise have left behind. Cold-path probe count is unchanged — at most
one, via `probed` — and a cold run with an existing cache dir is one fork
cheaper than today. One reordering is worth noting: because the staging gate
sits inside the staging body, a cold invocation that is going to fail with the
cache-dir diagnostic now spends ~8 ms hashing first, where previously
`resolve_cache_dir` failed at `:195` before any hashing.

**The saving does not reach the paths that motivated the work item.** The
launcher runs the same write-chmod-exec probe on every external-subcommand
dispatch, so a warm `accelerator vcs guard` still pays a first-exec penalty of
the same shape, and `FetchVerifyCacheResolver::resolve` additionally re-verifies
the cached sub-binary's signature on every dispatch. Phase 4 raises the probe as
a follow-up and measures it; this plan's `version` measurement covers the
bootstrap only, and says so.

`tests/integration/skill-invocation/` also speeds up incidentally — it runs the
real bootstrap once per `!`-site across 46 SKILL.md files, all warm after the
first.

## Migration Notes

None. No on-disk format, cache layout or environment contract changes. A cache
directory populated before the change is warm after it, and is now additionally
allowed to be read-only for warm bootstrap invocations (dispatched subcommands
still need it writable — see Phase 3). The change is invisible to the launcher
and to every caller except in latency.

## References

- Original work item:
  `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md`
- Research:
  `meta/research/codebase/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`
- Measurements and attribution:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §12
- Work item review:
  `meta/reviews/work/0186-remove-exec-probe-from-bootstrap-warm-path-review-1.md`
- Plan review:
  `meta/reviews/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path-review-1.md`
- Design being edited: `meta/work/0164-launcher-and-git-style-dispatch.md`
- Downstream consumer of the measurement:
  `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- Rebase baseline:
  `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADR-0049 (bash 3.2 compatibility floor), ADR-0046 (static binary
  distribution)
- Probe to split: `bin/accelerator:166-180`
- Resolution and diagnostic: `bin/accelerator:184-197`
- Shim staging, which hosts the first gate: `bin/accelerator:252-261`
- Warm/cold branch, which hosts the second: `bin/accelerator:336-348`
- Lock loop the second gate protects: `bin/accelerator:275-303`
- Harness funnel: `tests/integration/support/installation.py:324-369`
- Cached-launcher path idiom:
  `tests/integration/entrypoint/test_accelerator_entrypoint.py:219`, `:272`
- No-`build:cli:dev` invariant:
  `tests/integration/support/installation.py:149-157`,
  `tests/unit/tasks/test_mise.py:104-109`
- Launcher-side probe (follow-up):
  `cli/launcher/src/launch/outbound/resolve/cache_root.rs:80-94`,
  `cli/launcher/src/main.rs:65`
- CI lanes: `.github/workflows/main.yml:55-91`
- Corpus frontmatter validator: `tasks/test/integration.py:33-41`
- Shell-source scope for the extensionless entrypoint:
  `tasks/shared/sources.py:110`
