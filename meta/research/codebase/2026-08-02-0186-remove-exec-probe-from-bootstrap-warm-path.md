---
type: codebase-research
id: "2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"
title: "Research: Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-08-02T21:21:52+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0186"
parent: "work-item:0186"
relates_to:
  [
    "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration",
    "codebase-research:2026-07-27-0182-plugin-root-self-location-implementation-surface",
    "codebase-research:2026-07-03-0164-launcher-and-git-style-dispatch",
  ]
topic: "Remove the exec probe from the bin/accelerator warm path"
tags: [research, codebase, bootstrap, shell, performance, testing, bash-3.2]
revision: "dcf0eff40119220db91dd607de3b9089aa479b6b"
repository: "accelerator"
last_updated: "2026-08-02T21:21:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Remove the Exec Probe from the Bootstrap Warm Path

**Date**: 2026-08-02T21:21:52+00:00
**Author**: Toby Clemson
**Git Commit**: `dcf0eff40119220db91dd607de3b9089aa479b6b`
**Branch**: working copy on `main` (no bookmark; `main` @ `9b3eab92b3`)
**Repository**: accelerator

## Research Question

What is the implementation surface for work item 0186 — removing the
write-chmod-exec probe from `bin/accelerator`'s warm path while retaining it on
the cold path — and does the work item's stated mechanism, its acceptance
criteria, and its premises hold against the post-0182 codebase?

## Summary

The change itself is small and the work item's core argument is sound: the
probe is redundant on the warm path, `resolve_cache_dir` has no fallback, and
the diagnostic is the only thing that moves. All three of the premises 0186
flagged for re-checking after the 0182 rebase were checked. **Two hold, one is
wrong.**

Six findings materially affect the plan. Three are defects in the work item as
written:

1. **The stated mechanism breaks two of its own acceptance criteria.** "Call the
   probe only on the cold path before fetching" is under-specified: the shim
   staging block (`cp` + `chmod` into `cache_dir`) runs *before* the warm/cold
   branch is even decided. On a `chmod -x` cache dir the `cp` fails first and
   emits `could not stage the verify shim into …`, not `no writable,
   exec-capable cache directory` — so criteria 4 and 6 fail. The probe must be
   gated on a cheap cache-hit pre-test placed **above** the staging block, which
   requires hoisting the `launcher`/`launcher_sig` assignments. See
   [Finding 1](#finding-1-the-probe-must-move-above-shim-staging-not-into-the-cold-branch).

2. **`PS4='+${FUNCNAME[0]}:'` — the exact string criterion 2 mandates — is
   broken under the bootstrap's `set -u` on the bash 3.2 floor.** Measured: it
   emits `FUNCNAME[0]: unbound variable` to stderr for every top-level command
   and leaves PS4 literally unexpanded on those lines. `PS4='+${FUNCNAME[0]:-main}:'`
   works cleanly. The criterion needs the two-character fix. See
   [Finding 2](#finding-2-the-mandated-ps4-string-is-broken-under-set--u-on-bash-32).

3. **Criterion 1's assertion target does not exist.** It asks for "the expected
   `version` output — the version the harness fixture builds, asserted exactly".
   The harness's default launcher is a Python stub that prints **nothing**; the
   opt-in real launcher prints `CARGO_PKG_VERSION` (`1.24.0-pre.21`), not the
   fixture's `9.9.9-test`. Neither reading is achievable. See
   [Finding 3](#finding-3-criterion-1s-version-output-assertion-is-unachievable-as-worded).

Three are premise checks and capability findings:

4. **The `build:cli:dev` premise is wrong, and there is an active guard against
   it.** The suite builds its launcher in-fixture via `installation.build_launcher()`.
   `tests/unit/tasks/test_mise.py:104-109` fails if `test:integration:entrypoint`
   ever gains the edge. The 0186 review's own suggestion of a pre-warmed harness
   "needing `build:cli:dev`" must be re-litigated. See [Finding 4](#finding-4-no-buildclidev-edge--and-a-guard-forbidding-one).

5. **The cache-dir premise holds**, with a wording caution: `resolve_cache_dir`
   has two candidates in strict precedence (override, else `${plugin_root}/bin`)
   and returns 1 rather than falling through. "Override-or-default, no fallback"
   is accurate; "a single cache dir" reads as wrong. See [Finding 5](#finding-5-cache-dir-premise-holds-with-a-wording-caution).

6. **The trace capability needs no harness change at all.** `SHELLOPTS=xtrace`
   threaded through the existing `extra_env` parameter enables xtrace in bash 3.2
   — verified. A `bash_args` keyword on `run_bootstrap` is a cleaner two-line
   alternative. `BASH_XTRACEFD` is bash 4.1+ and therefore unusable. See
   [Finding 7](#finding-7-trace-capture-needs-no-harness-change-though-two-lines-would-be-cleaner).

**Every line number in the work item is still correct.** The work item warns
they "predate 0182" and must be re-resolved — but research §12 was written on
the 0182 lineage, and 0182 did not shift the region. `probe_dir` is at
`:166-180`, `resolve_cache_dir` at `:184-193`, the call site at `:195-197`,
staging at `:255-261`, `verify_launcher` at `:310-312`, the final `exec` at
`:352`. All six test names quoted in the work item still exist verbatim.

## Detailed Findings

### The production change surface

`bin/accelerator` is 352 lines. The relevant sequence, in execution order:

| Line | What happens | Warm? |
| --- | --- | --- |
| `:110-115` | self-locate `plugin_root`, `export ACCELERATOR_PLUGIN_ROOT` | yes |
| `:158-163` | verify-shim + public-key gates (`fail_integrity`) | yes |
| `:166-180` | **`probe_dir`** — `mkdir -p`, write, `chmod +x`, exec, `rm` | — |
| `:184-193` | `resolve_cache_dir` — calls `probe_dir`, no fallback | yes |
| `:195-197` | call site + `fail "no writable, exec-capable cache directory: …"` | yes |
| `:198` | `unverified_log` reassigned to `${cache_dir}/…` | yes |
| `:207-233` | dev-launcher override (execs and never returns when taken) | yes |
| `:252-253` | `sha256_file` #1 over `shim_source` (~11.7 ms) | yes |
| `:255-256` | staging **condition** — `-x` test + `sha256_file` #2 (~11.7 ms) | yes |
| `:257-260` | staging **body** — `cp` + `chmod` into `cache_dir` | cold only |
| `:305-307` | `launcher`, `launcher_sig`, `base_url` assignments | yes |
| `:336` | cache-hit test: `-x launcher` && `-f sig` && `verify_launcher` | yes |
| `:339-347` | cold branch — `acquire_lock`, `fetch_and_verify` | cold only |
| `:352` | `exec "${launcher}" "$@"` | yes |

The probe body writes via `>` redirection (`bin/accelerator:169`), which is why
xtrace cannot see the filename — bash prints expanded command words but not
redirections. Confirmed empirically (Finding 2).

#### Finding 1: the probe must move above shim staging, not into the cold branch

The work item's Requirements say to call the probe "only on the cold path before
fetching". Read literally — inside the `else` at `:339` — this breaks criteria 4
and 6.

Measured primitives on darwin (this host, bash 3.2):

```
chmod -x dir:  mkdir -p on existing dir: OK
               write into it:            FAILED (permission denied)
               cp into it:               FAILED
               [ -x dir/file ]:          false  (cannot stat through it)
chmod 0555:    mkdir -p:                 OK
               write new file:           FAILED
               [ -f dir/existing ]:      true
```

So on criterion 4's scenario — empty cache dir, `chmod -x`, cold invocation —
control reaches the staging block at `:255` before the branch at `:336`. The
`-x "${shim}"` test is false, the `sha256_file` mismatches, and the body's `cp`
at `:257` fails, producing `fail_integrity "could not stage the verify shim into
${cache_dir}"`. The criterion asserts the substring `no writable, exec-capable
cache directory`, which never appears. Criterion 6 (warmed-then-`chmod -x`)
fails the same way: `-x "${launcher}"` goes false, so the run is cold and hits
staging first.

The probe therefore has to fire **before** `:252`. Warm/cold is not yet known
there — but a cheap, side-effect-free approximation is:

```bash
launcher="${cache_dir}/accelerator-launcher-${version}-${platform}"
launcher_sig="${launcher}.minisig"

# The exec probe is a cold-path concern: a warm call proves the same capability
# for real by running the staged shim and then the launcher out of this
# directory. Gate on the cached artefacts, not on verification, because the
# verifier itself has to be staged into the directory first.
if [[ ! -x "${launcher}" ]] || [[ ! -f "${launcher_sig}" ]]; then
	probe_exec_capable "${cache_dir}" || fail_no_cache_dir
fi
```

`launcher`/`launcher_sig` depend only on `cache_dir` (`:195`), `version`
(`:134`) and `platform` (`:130`), so hoisting them from `:305-306` to just below
`:198` is safe. Placing the gate *after* the dev-override block (`:207-233`)
additionally stops the dev path paying for a probe it never needed — a small
improvement over today.

Walked against every existing test in the suite, this shape keeps all of them
green:

- `test_readonly_root_without_override_is_a_named_error` — `bin/` at `0o555`,
  no launcher → gate fires → probe's write fails → same diagnostic. ✓
- `test_readonly_root_with_override_runs_from_override` — override dir is fresh
  and writable → probe passes → cold fetch. ✓
- `test_a_record_is_always_one_line` — cache dir writable, a `0o555` *directory*
  planted at the staging path → probe passes, staging still fails as today,
  still one line. ✓
- concurrency, stale-lock, host-detection, dev-override, planted-shim tests —
  all cold or unaffected. ✓

**One residual diagnostic narrowing**, worth recording rather than fixing
silently: launcher present and executable, signature present, but *verification
fails* (tampered) **and** the directory is unwritable. The gate does not fire,
so the failure surfaces at `fetch_and_verify` as `could not fetch and verify the
accelerator launcher` instead of today's cache-dir message. Restoring it costs a
second call site in the `else` at `:339`, made idempotent by a `probed=""` flag
so a genuinely cold run does not pay ~108 ms twice. Both call sites stay strictly
off the warm path, so criterion 2 is unaffected either way.

#### Naming constraints for the split

Criterion 2 keys the assertion on the probe function's name, so the names chosen
by the split are load-bearing:

- `ensure_dir` (the `mkdir -p` half) **does** appear in every warm trace. The
  criterion must name the probe half, not this one.
- The probe's own temp file is `.accelerator-probe-$$` — the string `probe`
  appears in the trace as a path component regardless. A naive `"probe" not in
  trace` assertion would be wrong. Assert on the PS4-delimited function token
  (`+probe_exec_capable:`), not a bare substring.
- Keeping the name `probe_dir` for the split-off half is workable but muddier:
  it shares a prefix with the file, and the name now describes only part of what
  it used to do. `probe_exec_capable` reads better and greps unambiguously.

#### Shell-tooling constraints on the edit

- **`bin/accelerator` is tab-indented, and must stay so.** `.editorconfig`'s
  shell section is keyed `[*.sh]` (`.editorconfig:36-39`), which does not match
  an extensionless file — so shfmt formats it under its defaults: tabs, and
  `case` arms *not* extra-indented. The file has 132 tab-indented lines and zero
  two-space ones. A new function written with spaces reddens
  `format:scripts:check`.
- Discovery is by literal name: `tasks/shared/sources.py:106-110` carries
  `_EXTRA_SHELL_SOURCES = ("bin/accelerator",)`, and `scripts/lint-bashisms.sh:22-34`
  has its own fallback append. Both are pinned by
  `tests/unit/tasks/test_bootstrap_coverage.py:29-35`.
- `scripts/lint-bashisms.sh` denylist (`:48-55`), first match per line:
  `declare/local/typeset -A`; namerefs `-*n`; escaped brace in a
  `${x:-…}` default; `mapfile`/`readarray`; case-modification `${x^^}`/`${x,,}`;
  `&>>`; `|&`; negative array subscripts. Per-line escape hatch:
  `# lint-bashisms: ignore`. The header says it is `KNOWN-INCOMPLETE` — it
  cannot prove 3.2 compatibility. The suite's `_BASH = /bin/bash` pin plus
  `test_the_suite_runs_the_bootstrap_on_the_bash_floor` is the real enforcement.
- `lint:claude-coupling:check` (`tasks/lint/claude_coupling.py:31-38`) fails on
  **any** `CLAUDE_[A-Z0-9_]*` token in `bin/accelerator`, and
  `test_bootstrap_coverage.py:54-67` asserts the file exports **exactly**
  `{ACCELERATOR_PLUGIN_ROOT}`.
- Route every new abort through `fail()` or `fail_integrity()` — never a bare
  `exit 1`. `abort_status` (`:28-39`) is the `--fail-safe` contract.
- `unverified_log` is assigned twice (`:113` pre-resolution, `:198`
  post-resolution) because the shim and key gates fire *before*
  `resolve_cache_dir`. Any restructuring must keep both coherent.

### The test surface

The harness is **not** in the test file. It lives in
`tests/integration/support/installation.py` (379 lines) and is **shared with
`tests/integration/skill-invocation/test_skill_invocation_conformance.py`** —
so a harness change touches two suites.

`run_bootstrap` (`installation.py:324-369`):

```python
def run_bootstrap(
    root: Path, server: Path, downloader: Path, *,
    args: tuple[str, ...] = (),
    extra_env: dict[str, str] | None = None,
    path: str | None = None,
    entry: Path | None = None,
    cwd: Path | None = None,
    timeout: float | None = None,
) -> subprocess.CompletedProcess[str]:
```

- Builds a **complete replacement** environment (`PATH`, `HOME`,
  `ACCELERATOR_BOOTSTRAP_DOWNLOADER`, `ACCELERATOR_RELEASE_BASE_URL`,
  `SERVER_DIR`, `DL_LOG`), then `env.update(extra_env)`. Neither root variable
  is injected — the bootstrap self-locates from `entry`.
- Invokes `[BASH, str(entry), *args]` — argv list, `shell=False`. `BASH` is
  `/bin/bash` where it exists (`installation.py:41`), i.e. macOS's 3.2.
- **stdout and stderr are captured separately.** Tests concatenate by hand
  (`result.stdout + result.stderr`).
- `assert_hermetic` (`:301-321`) enforces the injected downloader, a `.invalid`
  base-URL hostname, and that the entry is not the repo's own bootstrap. A
  session-autouse `repo_bin_is_untouched` fixture
  (`test_accelerator_entrypoint.py:77-87`) fails if the suite writes into the
  shipped `bin/`.

There is no HTTP server: `serve_launcher` writes files into a directory and the
injected downloader copies from it, logging each URL to `DL_LOG`. Minisign is
genuinely exercised — real `minisign` CLI keygen and signing, real cargo-built
`accelerator-verify` shim doing verification. `require()` (`:97-104`) **skips
locally but `pytest.fail`s in CI** when a tool is missing.

#### Finding 2: the mandated PS4 string is broken under `set -u` on bash 3.2

Measured on this host (`GNU bash 3.2.57(1)-release, arm64-apple-darwin25`),
running a script that carries the bootstrap's `set -uo pipefail`:

```
$ PS4='+${FUNCNAME[0]}:' /bin/bash -x t1.sh …
+:set -uo pipefail
t1.sh: line 11: FUNCNAME[0]: unbound variable
+${FUNCNAME[0]}:echo 'top level before'
t1.sh: line 12: FUNCNAME[0]: unbound variable
+${FUNCNAME[0]}:probe_dir /…/cdA
+probe_dir:mkdir -p /…/cdA
+probe_dir:probe=/…/cdA/.accelerator-probe-68245
+probe_dir:printf '#!/bin/sh\nexit 0\n'
+probe_dir:chmod +x /…/cdA/.accelerator-probe-68245
+probe_dir:/…/cdA/.accelerator-probe-68245
+probe_dir:rm -f /…/cdA/.accelerator-probe-68245
+probe_dir:return 0
```

At top level `FUNCNAME` is unset, so under `set -u` each top-level command emits
an error line to stderr and PS4 stays literally unexpanded. The run does not
abort (there is no `set -e`), but for a 352-line script this is dozens of
spurious stderr lines interleaved with the bootstrap's own diagnostics. The fix
is a default expansion:

```
$ PS4='+${FUNCNAME[0]:-main}:' /bin/bash -x t1.sh …
+main:set -uo pipefail
+main:probe_dir /…/cdA
+probe_dir:mkdir -p /…/cdA
…
```

Zero unbound-variable errors, and `main` labels the top-level frame usefully.

Three further observations from the same traces, all bearing on criteria 2 and 3:

- **The probe's execution IS observable**, as a trace line whose whole command
  word is the probe path: `+probe_dir:/…/.accelerator-probe-68245`. That is the
  signal criterion 3's exec half needs — a line matching
  `^\++<probe-fn>:.*\.accelerator-probe-`.
- **The redirection is NOT observable**: `+probe_dir:printf '#!/bin/sh\nexit 0\n'`
  — the `>"${probe}"` is absent. Pass 3's reasoning is confirmed empirically.
- **`FUNCNAME[0]` is the innermost frame**, and command substitution adds a `+`
  to the prefix depth. Today `resolve_cache_dir` runs inside `$( )` at `:195`,
  so its trace lines are `++resolve_cache_dir:` / `++exec_probe:`. If the split
  moves the probe to a top-level call site the depth becomes `+`. **Assertions
  must not anchor on the number of `+` characters** — match the function token,
  allowing one or more leading `+`.

#### Finding 3: criterion 1's "version output" assertion is unachievable as worded

The default launcher served by the harness is a **Python stub**
(`installation.py:55-71`) that writes its argv to `LAUNCHER_ARGS_OUT`, its
environment to `LAUNCHER_ENV_OUT`, and exits with `LAUNCHER_EXIT`. It prints
nothing to stdout or stderr for any argv, `version` included.

The opt-in real launcher (`make_harness(real_launcher=True)`, used by exactly
three tests) prints, per `cli/launcher/src/version/inbound/cli.rs:7-20`:

```
accelerator {version}
commit: {sha}
built:  {date}
target: {triple}
```

where `version` is `env!("CARGO_PKG_VERSION")`
(`cli/launcher/src/version/outbound/build_metadata.rs:17`) — currently
`1.24.0-pre.21` from `cli/Cargo.toml:7`. The fixture's `VERSION = "9.9.9-test"`
(`installation.py:45`) only reaches `plugin.json` and the cached-artefact
filenames; it is never printed.

So "the version the harness fixture builds, asserted exactly" is wrong on both
readings, and asserting the real launcher's exact version would couple the test
to a bumping workspace version. Two workable amendments:

- **Stub route** (cheapest, no cargo launcher build): assert `returncode == 0`
  and `args_out.read_text().splitlines() == ["version"]`. This still proves the
  warm path completed through to `exec`.
- **Real-launcher route**: `make_harness(real_launcher=True)` and assert
  `result.stdout.startswith("accelerator ")` plus the presence of the
  `commit: ` / `built:  ` / `target: ` line prefixes.

The stub route is sufficient for what criterion 1 actually tests (a fatal probe
would fail its write and exit non-zero) and avoids a cargo launcher build.

#### Finding 4: no `build:cli:dev` edge — and a guard forbidding one

`mise.toml:183-186` declares `test:integration:entrypoint` with
`depends = ["deps:install:python"]` only; `tasks/test/integration.py:106-113`
runs `uv run pytest tests/integration/entrypoint -v`. The launcher and shim are
built **in-fixture** by `installation.py:123-157` (`cargo build --quiet`,
`--manifest-path cli/Cargo.toml`), lazily, from module-scoped fixtures.

The docstring at `installation.py:149-157` states the invariant:

> Built here rather than behind a `mise` build edge so a suite using it still
> runs standalone under a bare `uv run pytest`. A suite that calls this must
> therefore *not* also gain a `build:cli:dev` dependency: the two would contend
> on cargo's target lock and the asserted edge would be inert.

And it is asserted: `tests/unit/tasks/test_mise.py:36-56` classifies
`"test:integration:entrypoint": "builds it in-fixture, mirroring shim_bin"` in
`_NO_LAUNCHER_NEEDED`, with `test_task_needing_no_launcher_omits_the_build_edge`
(`:104-109`) failing if the edge appears.

The 0186 review (`…-review-1.md:133`, `:332`) suggests a pre-warmed-cache
harness "needs `build:cli:dev`". **That suggestion conflicts with the invariant
and should be re-litigated as `build_launcher()`** — or avoided entirely by the
stub route in Finding 3.

Consequence for planning: adding **cases to the existing file** requires no task
wiring at all. Adding a new `test:integration:*` **leaf** would require
classification in both `test_mise.py` sets plus `test:integration.depends` (or
`_NOT_IN_INTEGRATION_ROLLUP` with a reason). There is no reason to add one.

#### Finding 5: cache-dir premise holds, with a wording caution

`resolve_cache_dir` (`:184-193`) changed only by the `${CLAUDE_PLUGIN_ROOT}` →
`${plugin_root}` rename; `probe_dir` (`:166-180`) is byte-for-byte pre-0182.
0182's plan explicitly declined to touch the resolution
(*"Removing `cache_root.rs`'s refusal of an XDG fallback … `<root>/bin` always
exists; only the variable name changes."*).

The premise — the probe never *chooses* a directory — holds. But the phrasing
"a single `cache_dir` with no fallback" invites a reviewer's correction: there
are two candidates in **strict precedence** (`ACCELERATOR_CACHE_DIR`, else
`${plugin_root}/bin`), and if the override's probe fails the function returns 1
rather than falling back to the default. "Override-or-default, no fallback" is
the accurate form.

#### Finding 6: three permission tests exist; the root guard diverges from precedent

Existing permission manipulation in the suite, all with `finally` restore (which
is mandatory — `tmp_path` teardown cannot remove an unwritable directory):

- `:253-276` `test_readonly_root_with_override_runs_from_override` — `bin/` at `0o555`
- `:284-292` `test_readonly_root_without_override_is_a_named_error` — `bin/` at `0o555`
- `:1070-1085` `test_a_record_is_always_one_line` — a `0o555` directory planted at
  the staging path
- `:505-508` `test_dev_override_refused_when_not_executable` — `chmod 0o644` on a file

**There is no root guard anywhere in `tests/integration/entrypoint/` or in
`installation.py`** — so today's three `0o555` tests would silently pass as
false negatives under uid 0. The precedent lives in a neighbouring suite,
`tests/integration/hooks/test_launcher_link_refresh.py:275-293`:

```python
@pytest.mark.skipif(os.getuid() == 0, reason="mode bits are advisory for uid 0")
```

0186's preamble mandates the opposite disposition — **hard-fail, not skip** —
so the new cases must assert rather than copy this idiom. Worth a comment at the
call site explaining the deliberate divergence, or the next reader will
"fix" it back to a skipif. Whether to retrofit the same assertion onto the three
existing tests is a judgement call the plan should make explicitly; it is
arguably in scope as they share the exact hazard, and arguably scope creep.

CI runs unprivileged: `.github/workflows/main.yml:55-91` (`test-integration`,
matrix `[ubuntu-latest, macos-latest]`, `fail-fast: false`) has **no `container:`
key anywhere in the workflow**, so both lanes run as the `runner` user. Criterion
10's "both shipped lanes" is satisfied by that one job. `id -u` on this darwin
host returns `501`.

#### Finding 7: trace capture needs no harness change, though two lines would be cleaner

Measured: bash 3.2 honours `SHELLOPTS` from the environment at startup, for
non-interactive scripts, with no `-x` flag:

```
$ env SHELLOPTS=xtrace PS4='+${FUNCNAME[0]:-main}:' /bin/bash t2.sh
+main:set -uo pipefail
+main:f
+f:echo 'in f'
in f
```

So a trace test can be written today with `extra_env={"SHELLOPTS": "xtrace",
"PS4": "+${FUNCNAME[0]:-main}:"}` and no harness edit. The cleaner alternative
the work item anticipates is a keyword-only `bash_args` on `run_bootstrap` —
two lines (`installation.py:334` signature, `:360` argv → `[BASH, *bash_args,
str(entry), *args]`), safe for the shared skill-invocation consumer because of
the empty default.

Constraints either way:

- **Trace goes to stderr**, mixed with the bootstrap's own `accelerator: …`
  diagnostics. A trace case must be its own test — never a global mode — or
  every existing `assert … in result.stderr` breaks.
- **`BASH_XTRACEFD` is bash 4.1+**, so "trace to a separate fd, keep stderr
  clean" is not portable to the 3.2 floor. Filtering stderr lines by the PS4
  prefix is the portable approach.
- Tracing stops at `exec "${launcher}"` (`:352`) — exactly the desired scope.
- `PS4` must be passed as a **literal single-quoted string** so bash expands it
  per-command; pre-expanding it in Python would freeze one value.

### Latency measurement

**The methodology behind the Context table was never recorded.** Research §12
says only "Measured on darwin-arm64, warm cache, 20 iterations each" — there is
no bash loop, no `hyperfine` call, no script anywhere in `:617-708`. The
document does not even say the statistic is a median; "median" enters the record
retroactively via 0186's criterion, which describes it as "matching how the
Context table was produced". Host model, OS version and launcher provenance are
likewise unrecorded.

This is precisely why criterion 8 re-measures the before-median in the same
session and makes the gate a pure ratio (`after ≤ 0.5 × before`). Practical
consequence for the plan: **the measurement command must be authored fresh and
written into Validation Results** — there is nothing to reproduce.

Numbers not to confuse:

| Quantity | Value | Source |
| --- | --- | --- |
| Shell guard baseline B | 35.1 ms | research §12 `:624` |
| 0169's gate ceiling | 1.1 × B ≈ 38.6 ms | 0169 `:331` |
| Warm bootstrap, pre-0182 | 149.1 ms | §12 `:625` |
| Probe: fresh vs re-exec | 107.9 vs 10.6 ms → ~97 ms is first-exec | §12 `:628-629` |
| Expected after-median | ~41 ms | 149.1 − 107.9 |
| Residual retained | **~11.7 ms** (second hash) — *not* ~23 ms | §12 `:630` |
| Fully-addressable ceiling (rejected) | ~131 of 149 ms → ~18 ms | §12 `:644-645` |

The ~23 ms figure belongs to a *different* change (skipping staging entirely
when `shim_source`'s directory and `cache_dir` coincide), which is out of scope
and closed. Both medians must use the same **post-0182** launcher binary: the
bootstrap now exports only `ACCELERATOR_PLUGIN_ROOT`, so a pre-rename launcher
finds no root and silently degrades.

Incidental: the change also speeds up `tests/integration/skill-invocation/`,
which runs the real bootstrap once per `!`-site across 46 SKILL.md files — all
warm after the first.

### Surfaces the acceptance criteria do not mention

- **`docs/internals.md:208`** documents the behaviour being changed: *"…is where
  the bootstrap writes and *executes* a probe file, so point it at a directory
  you own and that is not group-writable."* After the split this is true only of
  the cold path. Small edit; not currently in any criterion.
- **`CHANGELOG.md`** has an open `## [Unreleased]` section with `### Added` and
  `### Changed`. A user-visible warm-path latency improvement plausibly belongs
  under `### Changed`; the criteria are silent on it.
- **`.gitignore`** has no `bin/.accelerator-probe-*` entry (the `bin/.tmp-*`
  pattern does not match it). The probe is created and removed within one
  invocation so this has never mattered — and it is unchanged by this work —
  but a probe leaked by a `SIGKILL` mid-cold-run would be untracked and
  un-ignored. Noting for completeness, not proposing action.
- **`cli/launcher/src/launch/outbound/resolve/cache_root.rs`** is the Rust mirror
  of `resolve_cache_dir`/`probe_dir`, with its own write+exec probe. It is *not*
  in scope — the launcher's probe is on a different (already-exec'd) path — but
  it is the obvious place a reader will ask "shouldn't that change too?". The
  answer is no: by the time the launcher runs, exec capability is proven.

## Code References

- `bin/accelerator:166-180` — `probe_dir`, the function to split
- `bin/accelerator:184-193` — `resolve_cache_dir`, override-or-default, no fallback
- `bin/accelerator:195-197` — call site and the `no writable, exec-capable cache directory` diagnostic
- `bin/accelerator:198` — post-resolution `unverified_log` reassignment
- `bin/accelerator:252-261` — shim staging; `:255-256` condition (two hashes), `:257-260` body
- `bin/accelerator:305-307` — `launcher`/`launcher_sig`/`base_url` (hoist candidates)
- `bin/accelerator:336-348` — warm/cold branch
- `bin/accelerator:352` — final `exec`
- `tests/integration/support/installation.py:324-369` — `run_bootstrap`, the single funnel
- `tests/integration/support/installation.py:301-321` — `assert_hermetic` preconditions
- `tests/integration/support/installation.py:55-71` — the Python launcher stub (prints nothing)
- `tests/integration/support/installation.py:149-157` — `build_launcher` and the no-`build:cli:dev` invariant
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:100-134` — `make_harness` factory
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:253-292` — the two `0o555` permission tests
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:584-644` — the three planted-shim tests protecting the second hash
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:709-720` — the bash-3.2-floor self-check
- `tests/integration/hooks/test_launcher_link_refresh.py:275-293` — the `os.getuid() == 0` skipif precedent
- `tests/unit/tasks/test_mise.py:36-56,104-117` — launcher-edge classification guards
- `tests/unit/tasks/test_bootstrap_coverage.py:29-67` — discovery, exec bit, single `*PLUGIN_ROOT` export
- `tasks/shared/sources.py:106-135` — `_EXTRA_SHELL_SOURCES` discovery of the extensionless bootstrap
- `scripts/lint-bashisms.sh:22-55` — fallback discovery and the bash-4 denylist
- `mise.toml:183-186` — `test:integration:entrypoint`
- `.github/workflows/main.yml:55-91` — the two-lane integration job
- `docs/internals.md:204-214` — the documented probe behaviour and env-var table
- `cli/launcher/src/version/inbound/cli.rs:7-20` — real launcher `version` output shape

## Architecture Insights

- **The probe was never a selection mechanism.** The 0164 plan
  (`meta/plans/2026-07-03-0164-…:145-153`, `:662-669`) specifies the cache root
  as "`${CLAUDE_PLUGIN_ROOT}` only, **probed** for writable+exec-capable", with
  an override as the only alternate terminus and no XDG fallback. The probe
  buys a *named error at resolution time* instead of an obscure downstream
  failure. That is the whole of what moves.
- **The warm path's real exec tests are stronger than the probe.** `verify_launcher`
  (`:310-312`) execs the staged shim from `cache_dir`, and `:352` execs the
  launcher from the same directory. A `noexec` mount fails both, routing to the
  cold branch where the probe belongs. The redundancy argument depends on both
  running from `cache_dir` and nowhere else — which is still true post-0182.
- **The trust boundary and the latency budget are in genuine tension**, and the
  resolution is recorded rather than optimised away: the second `sha256_file`
  costs ~11.7 ms on every warm call and is asserted by three tests. 0186
  deliberately leaves it, and hands the consequence to 0169 as a dated note
  rather than a work item.
- **Verification-by-trace is a deliberate second-best.** A real `noexec` mount
  would close the exec-vs-write gap completely but needs privileged per-platform
  CI setup dwarfing the production change. The gap is recorded, not hidden.
- **The suite deliberately runs on the oldest bash available.** 0182 discovered
  the entrypoint suite had been running on Homebrew bash 5.3 and pinned it to
  `/bin/bash`; the 3.2 floor is now genuinely enforced for anything touching
  this file.

## Historical Context

- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §12 (`:617-708`) — the measurement table, the attribution, the
  "split `probe_dir` into `ensure_dir` + probe" suggestion (`:654-664`), and the
  fallback position *"if in doubt, fix `probe_dir` alone (the ~108 ms) and leave
  the ~23 ms"* (`:665-678`), which is what 0186 became.
- `meta/reviews/work/0169-…-review-2.md` — pass 4 (`:735-826`) carved 0186 out.
  The decisive measurement: *"Eleven of pass 4's fourteen majors are defects in
  pass 3's fixes"*, leading to *"stop editing this document. Split it."* The
  grounds are scope and risk profile, not technical dependency.
- `meta/reviews/work/0186-…-review-1.md` — three passes, `verdict: APPROVE` set
  by the author on 2026-08-01 over a mechanical REVISE. Pass 3's headline finding
  (`:1196-1207`) killed a vacuous positive control: a generic
  `chmod`-or-write trace pattern also matches the cold path's own shim staging.
  That is why criterion 2 keys on the function name — and why this research
  checked whether the mandated `PS4` actually works (it does not, quite).
- `meta/work/0169-…:395-406` — the dated hand-off note, present and accurate as
  written; `:327-334` — the `G ≤ 1.1 × B` criterion, whose B is measured live per
  session rather than fixed at 35.1 ms.
- `meta/plans/2026-07-27-0182-…` and its validation — 0182 fully landed
  (`status: done`, `mise.local.toml` closing step done 2026-07-29); only
  release-gated manual checks remain, and none of them block this item.
- `meta/decisions/ADR-0049` (bash 3.2 floor), `ADR-0046` (zero-setup static
  binary distribution) — the constraints behind the shape of the file.

## Related Research

- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md` — source of §12
- `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md` — the rebase baseline
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md` — the design being edited
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md` — release-asset provenance for the measurement

## Open Questions

1. **Should the second probe call site be added to the cold branch?** Without
   it, a tampered launcher in an unwritable cache dir reports "could not fetch
   and verify" instead of the cache-dir diagnostic. With it, a `probed=""` flag
   is needed to avoid probing twice. Small either way; the plan should decide
   deliberately rather than leaving it to fall out of the edit.
2. **Should the three existing `0o555` tests gain the root assertion?** They
   share the exact hazard the criteria preamble identifies, and today they would
   pass as false negatives under uid 0. In scope by principle, scope creep by
   letter.
3. **Which criterion-1 route?** The stub route avoids a cargo launcher build and
   tests the same thing; the real-launcher route exercises more. Recommend the
   stub, but the work item's author intent may have been the latter.
4. **Does the `PS4` unbound-variable behaviour differ on bash 5 (the Linux
   lane)?** Not testable on this host. `${FUNCNAME[0]:-main}` is safe on both, so
   the question is moot if the fix is adopted — but if the criterion's literal
   string is kept, the linux lane's behaviour must be checked before criterion 10
   can be discharged.
5. **`docs/internals.md:208` and the `CHANGELOG` entry** — in scope for this item
   or deferred? Neither appears in any criterion.
