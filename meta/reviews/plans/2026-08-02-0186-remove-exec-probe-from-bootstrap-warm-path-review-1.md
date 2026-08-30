---
type: "plan-review"
id: "2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path-review-1"
title: "Plan Review: Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-08-02T22:25:30+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "work-item:0186"
target: "plan:2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "test-coverage", "code-quality", "architecture", "performance", "portability", "standards", "documentation"]
review_number: 1
review_pass: 2
tags: ["shell", "performance", "bootstrap", "bash-3.2", "testing"]
last_updated: "2026-08-03T10:31:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Remove the Exec Probe from the Bootstrap Warm Path

**Verdict:** REVISE

The production change at the heart of this plan is sound and well-evidenced:
the `ensure_dir` / `probe_exec_capable` split is the right cohesion boundary,
Gate A plus Gate B is a logically complete cover of the cold branch, the
`probed` flag is safe under `set -uo pipefail`, the hoist of
`launcher`/`launcher_sig` is provably free of dependencies, and the plan
correctly supersedes its own research on the ~30 s `acquire_lock` regression.
Two things need fixing before implementation. First, the verification apparatus
does not verify what it claims: the exec-half positive control's regex also
matches the probe's own `probe=…` assignment line (so it passes on a write-only
implementation), and Gate B — the line the plan itself calls a hang-regression
guard — has no automated case, no code comment, and would fail as a 30 s stall
rather than a red test. Second, Phase 4's measurement instrument puts a full
`python3` interpreter startup *inside* every timed interval, biasing the
`after ≤ 0.5 × before` ratio toward failing a correct implementation and
inflating the absolute figure that 0169's ≈ 38.6 ms decision rests on.
Separately, three lenses independently found that the saving does not reach the
paths that motivated the work item — the Rust launcher runs the identical
write-chmod-exec probe on every external-subcommand dispatch, and Phase 4
measures `version`, the one command that never traverses it.

### Cross-Cutting Themes

- **Gate B is load-bearing, unguarded and unexplained** (flagged by:
  test-coverage, portability, documentation, correctness, architecture) — the
  plan's own Key Discoveries call the second probe call site "required to avoid
  a regression, not merely to preserve a nicer message", yet no automated case
  reaches it (all five cold cases route through Gate A; the sixth is warm), the
  proposed code carries no comment naming the residual it covers, and
  `run_bootstrap`'s `timeout` defaults to `None` — so removing it turns the
  suite from green into a 30 s stall, not into a failure. Portability adds that
  the uncovered shape is precisely the Linux-prevalent one: a real `noexec`
  mount leaves the mode bits intact, so `-x launcher` and `-f sig` both hold and
  only Gate B fires.

- **The `no_cache_dir` paragraph states an edit and retracts it** (flagged by:
  code-quality, architecture, standards, documentation, correctness) — five
  lenses independently flagged Phase 2 item 4's "is rewritten to call the same
  wording via `no_cache_dir` so the substring has one definition", immediately
  followed by "the `:195` call site keeps its own literal … message". An
  implementer working top-down performs the first edit before reaching the
  retraction. Correctness adds that the stated reason is also wrong: `cache_dir`
  is *set-but-empty* at `:195` (command substitution assigns on failure), not
  unset, so a hoisted `no_cache_dir` would emit an empty path rather than
  tripping `set -u`.

- **Phase 4's measurement cannot be trusted as written** (flagged by:
  performance, test-coverage, portability, architecture) — four distinct
  defects in one 10-line script: a `python3` startup inside the interval
  (~25–80 ms against a ~41 ms target), no per-iteration exit-status or
  warmth assertion (empty `start`/`end` arithmetic yields `0 ms` and passes the
  gate trivially), both batches taken sequentially across a `jj new`
  working-copy swap so drift aliases onto the result, and `version` — a clap
  built-in — chosen as the workload when every consumer uses external-subcommand
  dispatch.

- **The saving does not reach the paths that motivated it** (flagged by:
  architecture, portability) — `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
  runs the same write-chmod-exec-rm probe from `LazyProductionResolver::resolve`
  on every external subcommand, *before* the cache-hit test. The plan declares
  it out of scope on a *redundancy* argument — the same argument it uses to
  relocate the bash probe, which applies verbatim to the launcher — and says
  nothing about cost. Phase 3's "A warm start neither writes nor probes" is
  therefore false at the system level once a hook dispatches `vcs guard`.

- **Phase 1 is not the independently-green slice it claims** (flagged by:
  correctness, architecture, standards) — `_entered` calls `re.search` while
  `import re` is deferred to Phase 2; `F821` is live on the test tree, so
  Phase 1 fails its own `mise run build-system:check` and `mise run check`
  criteria. Architecture adds that three of Phase 1's four additions have no
  consumer until Phase 2 (`_traced`, `_ran_probe_file`, `_PROBE_FN` naming a
  shell function that does not yet exist).

- **The residual handed to 0169 is understated and mis-framed** (flagged by:
  performance, architecture, documentation) — Performance Considerations says
  "~11.7 ms for the second `sha256_file`", but the plan's own Current State
  table marks *both* hashes as running warm, and the 41 ms arithmetic only
  closes with ~23 ms in it. The ~11.7 ms figure is the size of an out-of-scope
  *change*, not of the residual. Worse, at ~40 MB/s for a 475 K file that cost
  is process startup (macOS `shasum` is a Perl script, plus two `awk` execs),
  not cryptography — so it is recoverable by tool selection without touching the
  planted-stub defence, which reframes 0169's choice from "relax the threshold"
  to "meet it".

- **The trace assertions are less load-bearing than claimed** (flagged by:
  correctness, portability, test-coverage) — three independent weaknesses in
  the same mechanism: the exec-half regex matches the `probe=` assignment
  (vacuous), all trace-shape evidence is from darwin/bash 3.2 while the
  assertions must also pass on the ubuntu lane's bash 5.2, and the production
  function names are hard-coded in the test with no fast guard, so a rename
  produces a slow misleading integration failure while silently voiding the
  negative assertion.

- **Both documentation phases claim gates that do not exist** (flagged by:
  standards, documentation) — `mise run check` composes format/lint over four
  components only; there is no markdown linter anywhere in the task tree and
  corpus-frontmatter validation lives in the `test:integration:config` shell
  suites. Phase 3 and Phase 4 each rest their sole automated criterion on it.

### Tradeoff Analysis

- **Performance vs Security (the retained double hash)**: the work item
  resolved during review-1 to keep both `sha256_file` calls because three tests
  assert the planted-stub defence — a correct call. The performance lens shows
  the framing is nonetheless wrong: most of the ~23 ms is interpreter and
  process startup, recoverable by preferring `openssl dgst -sha256` over
  macOS's Perl `shasum` and replacing `| awk '{print $1}'` with parameter
  expansion. **Recommendation**: keep the scope closure, but record the finding
  and raise the follow-up — both hashes and both defences survive untouched, and
  0169 needs to know the lever exists before it decides to relax its gate.

- **Coverage vs an already-disproportionate verification burden**: the work
  item explicitly accepts that its criteria outweigh the production change
  "several times over", and this review proposes yet more cases. **Recommendation**:
  add only the two that guard behaviour nothing else does — the Gate B anti-hang
  case and the `ensure_dir`-fails case covering the retained `:195` diagnostic —
  and take the cheap one-liners (probe-file cleanup, warm-up return code) inside
  existing cases rather than as new ones.

- **Hard-fail vs skip under uid 0**: correctness and the plan want a hard fail
  so a root lane cannot report green on assertions root satisfies regardless;
  test-coverage, portability and standards note that this turns a currently-green
  root container into six hard failures with no sanctioned way out, inviting the
  ad-hoc `-k` workaround the guard exists to prevent. **Recommendation**: keep
  the hard fail and add a registered `unprivileged` marker, with the assertion
  message naming `-m 'not unprivileged'` as the recordable exclusion — that is
  exactly what the work item's "excluded explicitly, never skipped silently"
  rule asks for.

- **Precondition gates vs failure classification**: architecture proposes
  probing *reactively* from the failure branches that already know the cache dir
  is suspect, which needs no hoist, no flag, no duplicated cache-hit predicate,
  and additionally removes the probe from the cold *happy* path. The plan's
  fail-fast shape matches the file's existing idiom and keeps a trust-root
  script linear. **Recommendation**: keep the plan's shape — but record why the
  alternative was rejected, so a future editor tempted to "simplify away" Gate B
  or the flag can see it was weighed.

### Findings

#### Critical

- 🔴 **Test Coverage / Portability**: Gate B — the anti-hang guard — has no
  automated coverage on either lane
  **Location**: Phase 2, item 4 (Gate B); Success Criteria — Manual Verification
  Gate B exists solely to stop a ~30 s `acquire_lock` spin in the residual case
  (launcher `-x`, signature present, verification fails, cache dir unwritable),
  yet no automated case reaches that state: cases 5 and 6 use `chmod 0o666`,
  which clears the search bit and makes `[[ -x "${launcher}" ]]` false, so they
  route through Gate A. Deleting Gate B leaves all six new cases and the entire
  existing suite green — and because `run_bootstrap`'s `timeout` defaults to
  `None`, the regression manifests as a stall rather than a failure. Portability
  adds that this is the Linux-prevalent shape: a real `noexec` mount leaves mode
  bits intact, so only Gate B fires.

- 🔴 **Performance**: the timing harness includes a full `python3` interpreter
  startup inside every measured interval
  **Location**: Phase 4, item 1: Measurement
  `start` is read at the end of the first interpreter's startup but `end` at the
  end of the *second*, so each interval is `bin/accelerator version` **plus one
  complete `python3` process start** — plausibly 25–80 ms on darwin against the
  ~41 ms being measured. An additive constant on both sides pulls the ratio
  toward 1.0 (0.40 at b=30 ms, 0.53 and a **failed gate** at b=80 ms), and the
  inflated absolute after-median is the figure Phase 4 §3 hands to 0169, whose
  ceiling is ≈ 38.6 ms and whose decision turns on a ~2.4 ms overrun.

#### Major

- 🟡 **Correctness**: the exec-half positive control is vacuous — its regex also
  matches the probe's own variable assignment
  **Location**: Phase 1, item 2 (`_ran_probe_file`); Phase 2 (`test_cold_path_enters_and_executes_the_probe`)
  `probe_exec_capable`'s first statement is `probe="$1/.accelerator-probe-$$"`,
  and xtrace emits assignments — the research records the exact line
  `+probe_dir:probe=/…/.accelerator-probe-68245`. `\S*` happily consumes
  `probe=/…/`, so the assertion passes even with the `if "${probe}"` block
  deleted. This is the *only* verification the retained cold-path probe's exec
  half gets, by the plan's own admission.

- 🟡 **Architecture / Portability**: the launcher re-incurs the identical probe
  on every warm sub-binary dispatch, so the saving misses the motivating paths
  **Location**: What We're NOT Doing (`cache_root.rs`); Phase 3; Phase 4 §1
  `cache_root::probe_writable_and_executable` runs from
  `LazyProductionResolver::resolve` (`main.rs:65`) on every external subcommand,
  *before* the cache-hit test. Built-ins (`version`, `config`) never reach it —
  which is why today's SessionStart hook escapes and why Phase 4's `version`
  measurement cannot see it. 0169 serves `vcs guard` as a dispatched sub-binary,
  so it pays a fresh-file first-exec penalty of the same shape and plausibly the
  same ~97 ms magnitude on darwin. The out-of-scope justification is a
  redundancy argument that applies verbatim to the launcher.

- 🟡 **Performance**: the residual warm-path cost is stated as ~11.7 ms when
  ~23 ms remains
  **Location**: Performance Considerations; Phase 4 §3 (0169 hand-off)
  The plan's own Current State table marks `:252-253` (`sha256_file` #1) *and*
  `:255-256` (the condition containing #2) as running warm, and research §12
  records "`sha256_file` ×2 … ~23 ms". The ~11.7 ms figure is the size of a
  different, out-of-scope change. The 41 ms arithmetic only closes with 23 ms in
  it. Phase 4 instructs the implementer to update 0169's quoted residual — with
  a number that halves the only remaining lever.

- 🟡 **Performance**: the retained ~23 ms is process-startup cost in
  `sha256_file`, not an unavoidable trust trade-off
  **Location**: What We're NOT Doing (shim staging / double hash)
  11.7 ms to SHA-256 a 475 KB file is ~40 MB/s — one to two orders below
  arm64's hardware-accelerated throughput, so essentially none of it is
  cryptographic. On darwin `sha256sum` is absent, so `sha256_file` falls to
  `shasum -a 256`, a **Perl script**, plus a command-substitution subshell and
  an `awk` exec per call. Roughly 15–20 ms is recoverable by tool selection
  alone, hashing the same files with the same algorithm and keeping every
  assertion intact — which would put the warm bootstrap near ~21 ms, inside
  0169's ceiling.

- 🟡 **Performance**: the latency gate can pass on a measurement that measured
  nothing
  **Location**: Phase 4, item 1 (Gate: `after ≤ 0.5 × before`)
  Under `set -uo pipefail` with no `-e`, a failed `python3` leaves `start`/`end`
  set to the empty string — `set -u` does not catch this — and
  `$(( (end - start) / 1000000 ))` evaluates empty operands as 0, so every
  iteration prints `0` and the gate passes trivially. Nothing verifies the timed
  invocation succeeded or was warm, and the launcher filename embeds
  `${version}` from `plugin.json`, so the "cached launcher survives the `jj new`
  swap" premise holds only if both revisions carry the same version string. The
  awk median reads `v[10]`/`v[11]` without checking 20 samples arrived.

- 🟡 **Code Quality / Architecture / Standards / Documentation / Correctness**:
  the `no_cache_dir` paragraph contradicts itself and the wording ends up
  duplicated anyway
  **Location**: Phase 2, item 4
  "is rewritten to call the same wording via `no_cache_dir` so the substring has
  one definition" is retracted by the next sentence. Only the second reading is
  implementable (the helper is defined past line 233, below `:195`), and the
  stated reason is wrong: `cache_dir` is set-but-empty at `:195`, so a hoisted
  helper would emit `… cache directory:  is not usable …` with an empty path
  rather than tripping `set -u`. Net result: two independent literal copies of a
  substring four tests assert, and a helper whose stated purpose is unmet.

- 🟡 **Code Quality / Documentation**: after the split the `:195` diagnostic
  names conditions its call site no longer tests
  **Location**: Phase 2, item 3
  `resolve_cache_dir` is reduced to `mkdir -p`, so
  `no writable, exec-capable cache directory` now fires for "could not be
  created" — neither a writability nor an exec-capability result — and the same
  string is reused by the gates for a genuinely different cause. Three root
  causes (unreachable path, unwritable directory, `noexec` mount) collapse into
  one message, with errno discarded by `2>/dev/null` at every site.

- 🟡 **Architecture / Code Quality / Documentation**: `bash_args` widens a
  deliberately strict shared funnel enough to bypass its own preconditions
  **Location**: Phase 1, item 1
  `installation.py`'s docstring states `run_bootstrap` "is the single funnel
  every invocation passes … a caller cannot opt out of [its preconditions]", and
  `assert_hermetic` specifically refuses the repo's own `bin/accelerator`.
  Because bash treats the first non-option operand as the script, any
  `bash_args` entry not starting with `-` silently becomes the script and demotes
  the validated `entry` to `$1` — the funnel then validates a path it did not
  execute. The plan's own research shows no interface change is needed
  (`extra_env={"SHELLOPTS": "xtrace"}` works at the 3.2 floor), and the plan
  never reconciles the two.

- 🟡 **Correctness / Architecture / Standards**: Phase 1 cannot meet its own
  exit criteria — `import re` is deferred to Phase 2
  **Location**: Phase 1, item 2 vs Phase 2, item 1
  `_entered` and `_ran_probe_file` call `re.search`; the module imports no `re`,
  and `pyproject.toml`'s `tests/**` ignores drop only `S/ANN/D/PLR2004/SLF001/PT/INP001`
  — `F821` is live. Phase 1 therefore fails `mise run build-system:check` and
  `mise run check`, breaking the stated "each phase is independently green and
  mergeable" invariant. Three of Phase 1's four additions also have no consumer
  until Phase 2.

- 🟡 **Test Coverage**: "all six new cases fail before the production change"
  is unachievable as written
  **Location**: Phase 2, Overview and Success Criteria
  Read against today's `bin/accelerator`, three of the six already pass:
  `test_cold_happy_path_creates_a_missing_cache_dir` (today's `mkdir -p` already
  creates the nested override), `test_cold_path_keeps_the_noexec_diagnostic` and
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` (the existing
  probe's *write* fails, producing exactly the asserted exit and substring). Of
  the rest, two red only because the new function names do not exist yet — a
  rename artefact. Only case 1 reds behaviourally.

- 🟡 **Test Coverage**: after the change the retained `:195` diagnostic is
  reachable by no test
  **Location**: Phase 2, items 3 and 4
  Its only current cover,
  `test_readonly_root_without_override_is_a_named_error`, reroutes through Gate A
  once `mkdir -p` starts succeeding on the existing `0o555` `bin/`. Every other
  test either creates its cache dir or uses the default `bin/`, which
  `make_installation` always creates. Mutating that message to garbage would
  leave `mise run check` green — and it is one of two hand-maintained copies of a
  user-facing string.

- 🟡 **Standards / Documentation**: the `docs/internals.md` replacement quotes a
  range that ends mid-sentence and orphans the release-base-URL trust warning
  **Location**: Phase 3, item 1
  Line 209 reads `at a directory you own and that is not group-writable. The
  release base URL`. A literal `:207-209` replacement leaves the survivor
  starting `should be a host you trust not to serve an older signed release: …`
  with no subject — deleting or mangling the mirror-downgrade warning, the more
  security-relevant half of the paragraph. The quoted prose is also not wrapped
  to 80 columns.

- 🟡 **Standards / Documentation**: `mise run check` performs neither markdown
  lint nor frontmatter validation, so Phases 3 and 4 have no real automated gate
  **Location**: Phase 3 and Phase 4 Success Criteria
  `check` folds `frontend|server|cli|deny|pup|build-system|scripts:check` only.
  No markdown formatter or linter exists in `mise.toml` or `tasks/`, and
  meta-corpus frontmatter validation lives in
  `scripts/test-validate-corpus-frontmatter.sh`, run from the
  `test:integration:config` shell suites.

- 🟡 **Documentation**: the rationales for Gate B and the hoisted launcher paths
  never reach the script
  **Location**: Phase 2, item 4 / Key Discoveries
  Both facts a future editor of `bin/accelerator` must know — that the second
  call site prevents a ~30 s lock spin, and that the hoist is load-bearing for
  Gate A — are destined only for the plan and Validation Results. In a file
  where every non-obvious construct already carries a why-comment (`${dir:-/}`,
  the 16-hop bound, `cd -P` vs `cd`, content-addressed staging), `probe_once`
  appearing twice unexplained is the single most likely thing here to be
  "tidied" into one call site.

- 🟡 **Portability**: all trace-shape evidence is darwin/bash 3.2; nothing
  verifies the assertions under the ubuntu lane's bash 5.2 before Phase 2 is
  declared green
  **Location**: Key Discoveries (PS4/trace bullets); Phase 1, item 2
  `BASH = "/bin/bash"` is 3.2.57 on darwin and 5.2 on ubuntu. The `set -u`
  failure mode differs in kind (bash 5 aborts a non-interactive shell on an
  unbound expansion), so `:-main` is load-bearing on Linux for a different
  reason than the one recorded. Phase 2's criteria list only local darwin tasks;
  cross-lane observation is deferred to Phase 4.

- 🟡 **Documentation**: the newly supported read-only warm cache is left
  undocumented in the section that exists for it
  **Location**: Phase 3
  After the change a read-only cache directory with a pre-populated launcher
  works warm — `test_warm_path_survives_a_non_writable_cache_dir` pins exactly
  that, and today the same invocation aborts. The section being edited is titled
  "Offline, mirrored and read-only installs", yet neither the replacement text
  nor the changelog states the operator-visible consequence.

- 🟡 **Performance**: `ensure_dir` forks an external `mkdir` on every warm
  invocation for a directory that always exists
  **Location**: Phase 2, item 2
  `mkdir` is not a builtin, so every warm call pays a fork+exec (~1–3 ms) to
  create a directory the launcher is about to be exec'd out of. Guarding it
  changes no behaviour — `mkdir -p` on an existing directory is already a no-op,
  and a path existing as a *file* still fails `[[ -d ]]`, still runs `mkdir -p`,
  and still returns 1. 1–3 ms is the same order as 0169's entire projected
  shortfall.

- 🟡 **Performance**: sequential batches across a `jj` working-copy swap alias
  drift onto the result and ease the gate
  **Location**: Phase 4, item 1 (Sequence)
  All 20 `before` samples are taken immediately after `jj new` rewrites and
  snapshots the working copy — peak fsevents/Spotlight activity — then all 20
  `after` samples once the machine has settled, biasing in the *permissive*
  direction. Bare medians truncated to whole milliseconds, no dispersion
  recorded, for a figure 0169 compares against a 38.6 ms ceiling.

#### Minor

- 🔵 **Correctness / Portability**: the `chmod 0o666` rationale contradicts the
  plan's own Key Discoveries — those cases never reach the exec branch
  **Location**: Phase 2, item 1 (paragraph after the code block)
  A directory with no search bit blocks all name resolution inside it, so the
  probe's `printf … >"$1/.accelerator-probe-$$"` fails with EACCES at the
  *write* step; `0o666` on a directory **is** `chmod -x`, making the stated
  contrast vacuous. Portability adds that whether `ensure_dir` succeeds at all
  is userland-dependent (BSD `mkdir -p` tolerates it; GNU's `chdir`-based
  ancestor walk may not), so the two lanes may discharge these cases through
  different code paths while both stay green.

- 🔵 **Correctness**: Gate A does not cover shim staging when the launcher is
  cached but the staged shim is not
  **Location**: Desired End State; Phase 2, item 4
  Cache dir `0o555`, launcher and signature present (Gate A quiet), staged shim
  absent or digest-mismatched: control reaches the staging `cp` *before* Gate B
  and dies with `could not stage the verify shim into …` plus a
  `could not record to …` line. Today the `:195` probe fires first with the
  cache-dir message. This falsifies the "fires on **every** path" claim. A third
  `probe_once` as the first statement of the staging `if` body is free on the
  warm path and no-ops via `probed`.

- 🔵 **Correctness**: the shared diagnostic asserts "no ACCELERATOR_CACHE_DIR
  override was given" on paths where one was
  **Location**: Phase 2, item 4 (`no_cache_dir` message body)
  Both gates run after `resolve_cache_dir`, so `${cache_dir}` *is* the override
  when set — the message names the override directory and simultaneously denies
  one exists. Both `noexec` cases hit this wording and pass only because they
  assert the leading substring. Carried forward from the pre-change message
  rather than newly introduced, but now propagated to a new site.

- 🔵 **Correctness / Test Coverage**: the warming invocations are unchecked or
  rest on an unpinned contract
  **Location**: Phase 2, item 1 (cases 1 and 2)
  Case 1 warms with a **bare** real-launcher invocation whose exit code depends
  on whether clap's derive implies `arg_required_else_help` at the root — nothing
  in the repo pins it, all three existing `real_launcher=True` tests pass an
  explicit subcommand — and the assertion carries no output. Case 2 does not
  assert its warm-up at all, so a failed warm-up silently turns the "warm" trace
  into a cold one and surfaces as a confusing "probe was entered". Warm with
  `args=("version",)` and attach `stdout + stderr`.

- 🔵 **Code Quality / Architecture**: Gate A adds a third copy of the
  cached-artefact predicate, in the least readable rendering
  **Location**: Phase 2, item 4
  `[[ -x launcher ]] && [[ -f sig ]]` already appears at `:336` and `:341-342`;
  Gate A adds a hand-maintained De Morgan negation ~30 lines away. Drift makes
  Gate A either probe on a warm call (giving back the latency win) or skip on a
  cold one (moving the diagnostic behind the staging `cp`) — silently.
  Extracting `cached_artefacts_present()` and reusing it at all three sites makes
  the plan's own word "approximation" literally true.

- 🔵 **Code Quality**: `_traced`'s `**kwargs: object` plus two `# type: ignore`s
  is generality neither call site uses
  **Location**: Phase 1, item 2
  Both call sites pass only `extra_env`. The suppressions blind the checker to
  the whole forwarded set, so a future keyword typo reaches `run_bootstrap` as a
  runtime `TypeError` rather than a pyrefly error.
  `def _traced(harness, downloader, *, extra_env: dict[str, str] | None = None)`
  needs neither.

- 🔵 **Code Quality / Standards**: `no_cache_dir` and `probe_once` read as a
  predicate and a cheap guard, but both terminate the process
  **Location**: Phase 2, item 4
  The file's abort helpers are `fail` and `fail_integrity`; every other
  bare-noun/verb helper returns a value or a status. `no_cache_dir` is also a
  noun phrase in a uniformly verb-led file. Rename to `fail_no_cache_dir` and
  something honest for the gate (`require_exec_capable_cache`).

- 🔵 **Code Quality**: both halves of the split are left untidy
  **Location**: Phase 2, item 2
  `ensure_dir`'s `|| return 1` is dead control flow (`mkdir`'s status is already
  the function's), and every caller writes `|| return 1` anyway;
  `probe_exec_capable` carries forward three separate `rm -f` sites for one
  temporary file. Collapse to `chmod +x … && "${probe}"`, capture `status=$?`,
  `rm -f`, `return "${status}"`. Also state why `ensure_dir` is a function at all
  (the `_entered(trace, "ensure_dir")` assertion needs a stable token).

- 🔵 **Test Coverage**: the trace cases hard-code production function names with
  no fast guard
  **Location**: Phase 2, item 1 (`_PROBE_FN`); Testing Strategy — Unit Tests
  A rename fails `test_cold_path_enters_and_executes_the_probe` only after a
  cargo build and a full fetch-verify-cache round trip, reporting "probe not
  entered" rather than "the name moved", while the negative assertion silently
  goes vacuous. `tests/unit/tasks/test_bootstrap_coverage.py` already uses
  exactly the right idiom (`assert _KEY in _BOOTSTRAP_SRC.read_text()`).

- 🔵 **Test Coverage**: nothing asserts the probe file is cleaned up
  **Location**: Phase 2, items 1 and 2
  The split rewrites cleanup across three exits and no assertion checks removal.
  A dropped `rm -f` would litter `.accelerator-probe-<pid>` into every user's
  plugin `bin/` on each cold start — and the plan explicitly declines a
  `.gitignore` entry for the pattern. One line in the existing cold case:
  `assert not list(cache.glob(".accelerator-probe-*"))`.

- 🔵 **Test Coverage**: the criterion-1 assertion is looser than the available
  evidence allows
  **Location**: Phase 2, item 1 (case 1); Phase 4 criterion-1 amendment
  `startswith("accelerator ")` plus three prefixes would pass on truncated,
  reordered or empty field values. `launcher_bin` is already in scope — run it
  directly with `version` and assert **stdout equality**, which satisfies the
  criterion's "exactly" without hard-coding a version and additionally proves
  the cached binary is the one the fixture built.

- 🔵 **Architecture**: `acquire_lock`'s inability to distinguish contention from
  an unusable directory is masked at the call site, not fixed
  **Location**: Key Discoveries (~30 s hang); Phase 2, item 4
  The loop treats "no pid file" as "a competitor is about to write one" and has
  no notion of an unrecoverable `mkdir`. Gate B fixes the one instance reachable
  today; any future call site, retry, or the narrow TOCTOU window between the
  gates re-exposes the hang. Note that with `probed` already set by a successful
  Gate A, Gate B is a no-op — so a directory that becomes unusable *between* the
  gates still spins.

- 🔵 **Architecture**: the hoist creates an ordering invariant the new tests
  deliberately cannot catch
  **Location**: Phase 2, item 4; Testing Strategy
  The design depends on hoisted assignments → Gate A → first write into
  `cache_dir`, spanning ~50 lines. Inserting a *new* write above Gate A — exactly
  the situation the research found with the staging `cp` — is silent, and the two
  `noexec` cases are constructed with `0o666` specifically so "any preceding
  write still succeed[s]". Only the pre-existing `readonly_root` test would catch
  it, incidentally.

- 🔵 **Architecture**: no alternative to the precondition-gate shape is recorded
  **Location**: Implementation Approach
  A failure-classification shape — probe only from the failure branches that
  already know the directory is suspect — needs no hoist, no flag, no duplicated
  predicate, and removes the probe from the cold *happy* path too. Whether or not
  it is adopted, recording why it was rejected is what protects Gate B and the
  flag from a later "simplification".

- 🔵 **Portability**: only the uid-0 half of the mandated environment check is
  implemented
  **Location**: Phase 1, item 3
  The work item requires `id -u` **and**, for filesystems that ignore permission
  bits, a temp-dir capability check. On WSL `drvfs` without `metadata`, Docker
  Desktop bind mounts, `vboxsf`/9p shares, or a macOS directory with an
  inherited ACL, `chmod(0o666)` silently succeeds and two cases fail on
  `returncode != 0` with no hint that the environment is at fault.

- 🔵 **Code Quality / Standards**: comment placement and stale cross-references
  **Location**: Phase 2, items 1, 3 and 4
  The "Gate on the cached artefacts…" comment is attached to `probe_once` but
  explains the `if` two lines below; three test comments name sibling tests by
  hand (which will go stale on rename exactly as AC references do); and the
  `resolve_cache_dir` snippet is shown *without* the existing `:182-183`
  no-XDG-fallback comment, which plan snippets get transcribed verbatim.

- 🔵 **Test Coverage / Portability / Standards**: the retrofit turns a
  root-green suite into six hard failures with no sanctioned deselect
  **Location**: Phase 1, items 3 and 4
  Two of the three retrofitted tests pass today under uid 0; the repo's existing
  idiom for the pre-existing set is `skipif`. In a root devcontainer — a normal
  local configuration — a contributor cannot get a meaningful signal from
  `mise run test:integration:entrypoint` at all, which invites the ad-hoc `-k`
  workaround the hard fail exists to prevent. Register an `unprivileged` marker
  and name `-m 'not unprivileged'` in the failure message.

- 🔵 **Standards / Documentation**: the changelog entry is mechanism-heavy and
  contradicts its own success criterion
  **Location**: Phase 3, item 2
  The phase's manual criterion reads "describes user-visible behaviour, not the
  mechanism", while the draft spends three clauses on the probe file, `chmod`,
  the first-exec check and the staged verifier — and omits both the size of the
  win and where a user notices it. Neighbouring entries lead with the observable
  effect; earlier releases use `### Improved` for exactly this kind of change.

- 🔵 **Documentation**: six of eleven pending Validation Results entries get no
  stated evidence shape, and the criterion-3 split is undocumented
  **Location**: Phase 4, item 2
  Phase 4 enumerates five recordings and leaves the six per-check entries
  unguided, with no criterion-to-test-name mapping anywhere. The work item's
  criterion 3 requires the positive control to ride on criterion 6's *cold
  happy-path run*; the plan splits it into a traced default-launcher run and an
  untraced `real_launcher` run without recording that as a deviation, though
  smaller deviations each get a bullet.

- 🔵 **Documentation**: the staging comment's "(cheap)" becomes misleading once
  the probe is gone
  **Location**: What We're NOT Doing; Performance Considerations
  `bin/accelerator:246-251` says a warm call "re-hashes (cheap)". After this
  change that re-hash is the *dominant* remaining warm-path cost and 0169's
  criterion turns on it, yet the block and its comment are left untouched —
  steering the next latency hunt away from the biggest remaining item.

- 🔵 **Standards**: the plan document carries 16 lines over 80 columns
  **Location**: Plan document (frontmatter through References)
  `.editorconfig` sets `max_line_length = 80` for `[*]`, and the work item this
  plan derives from has **zero** over-long lines, wrapping even its long
  `meta/…` path references. With no markdown linter, drift in the
  highest-traffic planning artefacts is how the convention erodes.

#### Suggestions

- 🔵 **Performance**: record an expected-composition budget — the `0.5×` gate
  leaves ~33 ms of unnoticed slack
  **Location**: Desired End State; Phase 4 Success Criteria
  Against a ~149 ms before, the ceiling is 74.5 ms versus a ~41 ms expectation.
  An implementation leaving a third of the probe behind, or adding 33 ms of new
  work, clears the gate unremarked — and there is no diagnostic path if the
  after-median lands at 60 ms. Record the predicted composition (~23 ms hashing,
  ~2.3 ms verify, ~3 ms launcher, ~12 ms bash+`uname`×2+`sed`) and require any
  >25% deviation to be attributed before the figure is recorded.

- 🔵 **Performance**: enumerate the remaining per-call fork inventory for 0169
  **Location**: Performance Considerations
  A warm call still performs ~6 forks / 5 execs before `exec "${launcher}"`:
  `mkdir`, `sed` over `plugin.json`, `uname -m` and `uname -s` as two separate
  substitutions, `shasum`+`awk` twice, and two nested subshells for plugin-root
  resolution. Two cheap levers worth naming for a follow-up: collapse to one
  `uname -sm` (keeping both test seams independent), and drop the two `awk`
  execs in favour of parameter expansion.

- 🔵 **Code Quality**: encode the mandatory permission-restore invariant once
  **Location**: Phase 2, item 1
  The plan calls the `finally` restore "mandatory — `tmp_path` teardown cannot
  remove an unwritable directory", then hand-writes the shape in three new cases
  on top of three existing ones. A small
  `@contextlib.contextmanager restored_mode(path, mode)` removes the sixth-site
  risk, whose failure mode is a leaked directory breaking teardown for the whole
  session.

- 🔵 **Code Quality**: hoisting two of three lines splits a cohesive block for
  no stated reason
  **Location**: Phase 2, item 4
  `launcher`, `launcher_sig` and `base_url` are one block at `:305-307`;
  `base_url` depends only on `version` and could move too. Move all three, or
  state why (keeping `base_url` adjacent to its only consumer).

- 🔵 **Standards**: invoke the bash-3.2 gate through its mise leaf
  **Location**: Phase 2 Success Criteria
  `scripts/lint-bashisms.sh` has a leaf (`lint:scripts:bashisms:check`) and is
  already folded into the next criterion's `mise run scripts:check`. The raw
  script also discovers files via `git ls-files '*.sh'`, blind inside a jj
  workspace — it happens to work only because `bin/accelerator` is appended
  unconditionally.

- 🔵 **Portability**: pin the extensionless entry point's formatter settings
  **Location**: Key Discoveries (tab indentation)
  The file's format is currently an accident of shfmt's defaults. An explicit
  `[bin/accelerator]` section in `.editorconfig` (`indent_style = tab`,
  `switch_case_indent = false`) makes today's shape intentional and stable
  without reformatting anything, and stops the next contributor rediscovering
  the trap.

### Strengths

- ✅ The bottleneck is identified by measurement, not speculation, and the
  attribution is genuinely causal: 107.9 ms for a freshly-written probe against
  10.6 ms to re-exec an existing one isolates macOS's first-exec check rather
  than blaming filesystem work or the fetch-verify design generally.
- ✅ The ~41 ms prediction is arithmetically supported end-to-end — research
  §12's decomposition sums to ~149 ms, so removing 108 leaves ~41 with no
  unexplained residue.
- ✅ Gate A and Gate B are a logically complete cover of the cold branch: Gate A
  is the correct De Morgan negation of the cheap half of the warm test at
  `:336`, and Gate B fires on a superset of Gate A's set, so nothing stages,
  locks or fetches without a probe having run.
- ✅ The plan independently re-derived and *corrected* its own research on the
  residual case: `acquire_lock` really does burn its full 300 × 0.1 s budget
  when `mkdir` can never succeed, so Gate B is a genuine regression guard rather
  than a cosmetic one.
- ✅ The `probed` flag is safe under `set -uo pipefail` — assigned before either
  read, both call sites top-level so the assignment persists, and `probe_once`
  can only return 0 or terminate.
- ✅ The hoist is provably free: `launcher`/`launcher_sig` depend only on
  `cache_dir`, `version` and `platform`, and their consumers are function bodies
  resolved at call time.
- ✅ Every claim about existing tests staying green checks out on reading them,
  including the three planted-shim cases, the dev-override family,
  `test_tampered_cached_launcher_is_refused_and_healed` (which newly traverses
  Gate B), and `test_stale_lock_is_reclaimed`.
- ✅ The probe-absence assertion is protected against vacuity from two
  directions — an in-run `ensure_dir` presence check and a cross-test positive
  control — so a silently broken xtrace or PS4 cannot turn it green.
- ✅ Line and test references are accurate against the current post-0182 tree:
  every `bin/accelerator` range, `installation.py:149-157` and `:324-369`, the
  entrypoint suite's `:253`/`:279`/`:584-644`/`:772`/`:787`/`:801`/`:1060`, the
  `skipif` at `test_launcher_link_refresh.py:275-293`, and `main.yml:55-91`.
- ✅ The unprivileged-lane assumption is *verified* rather than assumed — the
  `test-integration` matrix has no `container:` key, so both GitHub-hosted lanes
  run as a non-root user.
- ✅ The `.editorconfig` trap is correctly diagnosed: `[*.sh]` does not match an
  extensionless file, `tasks/shared/sources.py:110` adds `bin/accelerator` to
  scope explicitly, and shfmt runs with no formatting flags — so tabs and
  `>"${probe}"` with no space are both right.
- ✅ No construct anywhere in the diff is bash-4-only or on
  `lint-bashisms.sh`'s denylist, and `${FUNCNAME[0]:-main}` is valid at the 3.2
  floor.
- ✅ Phase 3's single documentation target is complete: an exhaustive grep for
  `probe`, `ACCELERATOR_CACHE_DIR`, `noexec` and `exec-capable` across `docs/`,
  `README.md`, `skills/`, `hooks/`, `scripts/` and the ADRs finds no other
  shipped statement this change falsifies.
- ✅ Every `mise run` task name quoted in the Success Criteria exists, and the
  plan correctly declines to add a `test:integration:*` leaf that would have
  needed the no-`build:cli:dev` invariant re-checked.
- ✅ The split gives each function one reason to change and names them the way a
  domain expert would, replacing a `probe_dir` whose name described neither half.
- ✅ `probed`/`probe_once` mirrors the file's existing `lock_held`/`acquire_lock`
  idiom, so the new mutable state is not a new pattern to absorb.
- ✅ The plan refuses to buy latency by weakening the staging block's tested
  planted-stub defence, and carries the shortfall forward as a dated hand-off
  rather than absorbing it silently.
- ✅ The latency gate is host-relative rather than a fixed delta, and the
  before-median is deliberately re-measured post-0182 rather than reusing the
  stale 149.1 ms reference.
- ✅ Placing Gate A after the dev-override block removes the 108 ms from the
  contributor dev-launcher path too — a real per-SessionStart win claimed
  correctly as a bonus.
- ✅ The incidental win is spotted and quantified in kind:
  `tests/integration/skill-invocation/` runs the real bootstrap once per
  `!`-site across 46 SKILL.md files, all warm after the first.
- ✅ `_require_unprivileged`'s docstring is aimed squarely at the future reader
  who would "fix" it back to the neighbouring `skipif`, and Phase 1 adds a
  manual criterion to that effect.
- ✅ Each new test carries an inline comment stating what it does *and does not*
  prove, which is this module's existing convention.
- ✅ Deliberate deviations are named and justified rather than taken silently —
  the PS4 fix, the real-launcher route, the hard-fail guard — each with a
  recording obligation in Phase 4.
- ✅ The shared-harness coupling with `tests/integration/skill-invocation/` is
  identified up front and the extension designed backwards-compatible.
- ✅ "What We're NOT Doing" is precise and keeps the production diff tiny, and
  Migration Notes correctly state that no on-disk format, cache layout or
  environment contract changes.

### Recommended Changes

1. **Add an automated Gate B case, and comment Gate B in the script**
   (addresses: the Gate B critical; the Gate-B-rationale major; the
   `acquire_lock` and ordering minors)
   Warm the cache, overwrite the cached launcher bytes so `verify_launcher`
   fails while `-x` and `-f` stay true, `chmod 0o555` the cache dir (**not**
   `0o666` — the search bit must stay so Gate A remains quiet), invoke with an
   explicit `timeout=10`, and assert non-zero plus the cache-dir substring under
   `_require_unprivileged()` with a `finally` restore. Pass an explicit
   `timeout=` to the other permission cases too, so any re-introduced lock spin
   reports as a hang. Add a one-line comment above Gate B naming the residual
   case and the ~30 s spin it prevents, one on `probed` noting both gates are
   reachable in a single cold run, and one at the hoisted assignments recording
   that Gate A reads them and must precede the first write into `cache_dir`.

2. **Fix `_ran_probe_file` so the exec-half control is not vacuous**
   (addresses: the exec-half major)
   Anchor the probe path as the whole command word —
   `rf"^\++{re.escape(_PROBE_FN)}:/\S*\.accelerator-probe-\d+$"` — so the
   `probe=…` assignment line no longer matches, and add a red-step check that
   the assertion fails when the `if "${probe}"` branch is stubbed out.

3. **Rebuild the Phase 4 measurement instrument**
   (addresses: the timing critical; the gate-can-pass-on-nothing, drift and
   `version`-workload majors; the composition-budget suggestion)
   Drive the whole batch from a single interpreter (`time.perf_counter` around
   `subprocess.run`), asserting per iteration that the call exits 0 and stdout
   begins with `accelerator `. Calibrate the harness against `/usr/bin/true`
   first (must read ≲ 2 ms) and paste that figure alongside the medians. Remove
   the revision switch: copy the pre-change script to `bin/accelerator-before`
   in the same directory so `plugin_root`, `version` and the cached launcher are
   identical, and interleave the two variants sample-by-sample in one revision.
   Raise n to 50, record min/median/p90 at 0.1 ms resolution, assert `NR == 50`
   in the reduction, record the launcher's `ls -li` on both sides, and record
   the predicted composition so a >25% deviation must be attributed.

4. **Resolve the `no_cache_dir` contradiction by parameterising the helper**
   (addresses: the `no_cache_dir` major; the stale-conditions major; the naming,
   override-wording and comment minors)
   Delete the retracted sentence. Define `fail_no_cache_dir() { fail "no
   writable, exec-capable cache directory: $1 …"; }` **above** `:195` and call it
   from all three sites with the relevant directory — that genuinely gives the
   asserted substring one definition, removes the hidden `${cache_dir}` global
   coupling, and lets each site append its real cause ("could not be created" vs
   "rejected an executable file (noexec?)") while keeping the substring
   byte-identical. Use `${ACCELERATOR_CACHE_DIR:+ (from ACCELERATOR_CACHE_DIR)}`
   so the message stops denying an override that was given. Keep the existing
   `:182-183` no-XDG comment in the snippet.

5. **Repair the phase boundary and the false gates**
   (addresses: the Phase-1-not-green major; the `mise run check` major)
   Move `import re` into Phase 1 with the helpers that need it. Either reduce
   Phase 1 to `_require_unprivileged` plus its retrofit (the one change with
   standalone value) and move the trace scaffolding into Phase 2, or state that
   Phase 1 is deliberately a test-infrastructure slice. Replace Phase 3's
   "Markdown format and lint pass: `mise run check`" with honest manual criteria,
   and name `mise run test:integration:config` (or the corpus validator
   directly) where Phase 4 currently claims `check` validates frontmatter.

6. **Correct the residual accounting handed to 0169**
   (addresses: the ~11.7 ms major; the process-startup major; the fork-inventory
   suggestion)
   State the residual as the full post-change composition — ~23 ms hashing
   (both `:252` and `:256`), ~2.3 ms verify, ~3 ms launcher, ~12 ms
   bash+`uname`×2+`sed` — and hand 0169 that composition rather than a single
   number. Record in Validation Results that most of the hashing cost is
   interpreter/process startup rather than cryptography, and raise a follow-up
   for a faster `sha256_file` backend plus parameter-expansion field slicing —
   both hashes and both defences survive untouched. Fix the Desired End State to
   say "no exec of a *freshly written* file".

7. **Address the launcher-side probe explicitly rather than by redundancy
   argument** (addresses: the launcher/dispatch major)
   Replace the out-of-scope justification with the cost one. Take a second
   median in the same session against a real external-subcommand dispatch,
   record both with an explicit note that the `version` figure covers bootstrap
   cost only, scope Phase 3's docs and changelog wording to the bootstrap
   ("resolving an external subcommand still probes the cache directory once"),
   raise a sibling work item for `cache_root.rs`, and add the launcher-side
   residual to the 0169 hand-off note.

8. **Fix the `docs/internals.md` edit and add the read-only-cache sentence**
   (addresses: the orphaned-sentence major; the read-only-cache major; the
   changelog minor)
   Quote the whole `:207-212` paragraph as it should read after the edit,
   including the unchanged release-base-URL sentence, pre-wrapped to 80 columns.
   Add one sentence stating that a cache directory populated once may afterwards
   be read-only for warm invocations, while cold starts still need it writable
   and exec-capable. Rewrite the changelog entry to lead with the observable
   effect and a figure, keep one sentence of mechanism, and mention the
   read-only case.

9. **Correct the `chmod 0o666` rationale and record what it actually proves**
   (addresses: the `0o666` minor; the Gate-A-vs-staging minor; the
   advisory-filesystem minor)
   Say plainly that clearing the search bit makes the probe fail at its *write*
   step in both cases, so the exec half is covered only by the trace control —
   keeping Phase 4's recorded limitation authoritative — and note that BSD and
   GNU `mkdir -p` may discharge these cases through different code paths. Add
   the mandated temp-dir capability check next to `_require_unprivileged` so an
   advisory-permission filesystem reports itself rather than looking like a
   product regression. Either add a third `probe_once` at the top of the staging
   `if` body or soften the "every path" claim and record the residual.

10. **Verify the trace assertions on bash 5 before declaring Phase 2 green**
    (addresses: the bash-5 major; the function-name-guard minor)
    Add a cross-interpreter step to Phase 2 — an opt-in interpreter override in
    the harness defaulting to `/bin/bash`, plus a recorded run of the two trace
    cases under a bash 5.x — and note in a comment why `SHELLOPTS` was rejected
    (it is exported, so it leaks into descendant shells whose identity differs
    by platform). Add the one-line `test_bootstrap_coverage.py` guard asserting
    both function names exist in `bin/accelerator`.

11. **Narrow the harness seam** (addresses: the `bash_args` major)
    Replace `bash_args: tuple[str, ...]` with a keyword-only `xtrace: bool =
    False` that the funnel translates into `-x`; if a generic passthrough is
    genuinely wanted, assert every element starts with `-` so the script operand
    can never be hijacked. Add one clause to `run_bootstrap`'s docstring naming
    the parameter and the shared-consumer constraint, and record why the
    discovered `SHELLOPTS` route was not taken.

12. **Tighten the test bodies and the recorded closeout** (addresses: the
    warming, cleanup, criterion-1, `:195`-coverage, red-step, marker,
    Validation-Results and comment minors)
    Warm case 1 with `args=("version",)` and assert stdout equality against a
    direct `launcher_bin version` run; assert case 2's warm-up return code; add
    the probe-cleanup glob assertion; add one `ensure_dir`-fails case (a
    `0o555` parent with a nested `ACCELERATOR_CACHE_DIR`) so the retained `:195`
    diagnostic is reachable. Reword the red-step criterion to name which cases
    red pre-change and which are green-before/green-after preservation guards.
    Register an `unprivileged` marker and name `-m 'not unprivileged'` in the
    guard's message. In Phase 4, list all eleven Validation Results entries with
    the test function discharging each, and record the criterion-3 split as a
    deviation. Drop the sibling-test cross-references from the test comments and
    move the gating rationale above the `if` it explains.

13. **Tidy the mechanical items** (addresses: the `ensure_dir` fork major; the
    `_traced` typing, predicate-duplication, cleanup-shape, `(cheap)`-comment
    and 80-column minors; the `restored_mode`, `base_url`, bashisms-leaf and
    `.editorconfig` suggestions)
    `ensure_dir() { [[ -d "$1" ]] || mkdir -p "$1" 2>/dev/null || return 1; }`;
    give `_traced` a real signature and drop both `# type: ignore`s; extract
    `cached_artefacts_present()` and reuse it at all three sites; collapse
    `probe_exec_capable` to one cleanup path; amend the staging comment's
    "(cheap)" to record that the re-hash is now the dominant warm-path cost and
    why it is retained; rewrap the plan's 16 over-long lines; add
    `restored_mode`; move `base_url` with the other two or say why not; call
    `mise run lint:scripts:bashisms:check` or drop it as redundant; and add a
    `[bin/accelerator]` section to `.editorconfig`.

## Per-Lens Results

### Correctness

**Summary**: The core routing logic is sound: Gate A (`¬(-x launcher ∧ -f
sig)`) plus Gate B (top of the cold branch) is a complete cover of the cold
branch, the `probed` idempotence flag behaves correctly under `set -uo
pipefail` because both call sites are top-level and the flag is initialised
before first read, and I independently confirmed the plan's correction of the
research — without Gate B the residual case does spin `acquire_lock`'s full
300 × 0.1 s budget (empty `owner` takes the `else` arm at
`bin/accelerator:295-300`). I also re-traced every existing test the plan
claims stays green and all of them do. The significant defect is in the
verification apparatus rather than the production change: the
`_ran_probe_file` regex also matches `probe_exec_capable`'s own
variable-assignment trace line, which makes the load-bearing positive control
for the probe's *exec* half vacuous. Several supporting paragraphs also
contradict the plan's own Key Discoveries and its source research about
directory-permission semantics, and one paragraph contradicts itself about
where `no_cache_dir` is used.

**Strengths**:

- The gate conditions are logically complete. Gate A is the correct De Morgan
  negation of the cheap half of the warm test at `bin/accelerator:336`, and
  Gate B fires on exactly the cold branch, which is a superset of Gate A's set
  — so no path that stages, locks, or fetches proceeds without a probe having
  run, and the warm path runs none.
- The plan correctly supersedes the research's framing of the residual case. I
  traced `acquire_lock` (`bin/accelerator:275-303`) by hand: with an unwritable
  cache dir `mkdir "${lock_dir}"` always fails, `owner` is empty, so control
  takes the `waited=$((waited + 1))` arm and burns the full 30 s budget before
  emitting a lock-timeout message. The research's claim that it surfaces as
  `could not fetch and verify` is wrong; the plan's is right, and Gate B is
  genuinely a regression guard rather than a cosmetic one.
- The `probed` idempotence flag is correct under `set -uo pipefail`: it is
  assigned `""` before either read so `[[ -z "${probed}" ]]` cannot trip
  `set -u`; both `probe_once` call sites are top-level (not in a subshell or
  command substitution) so `probed=1` persists; and `probe_once` can only
  return 0 or terminate the shell via `fail`, so no caller has to handle a
  non-zero return.
- The claims about existing tests staying green check out. I re-traced
  `test_readonly_root_without_override_is_a_named_error`,
  `test_readonly_root_with_override_runs_from_override`,
  `test_a_record_is_always_one_line`, the three planted-shim cases, plus
  `test_tampered_cached_launcher_is_refused_and_healed` (which newly routes
  through Gate B), `test_stale_lock_is_reclaimed`,
  `test_concurrent_cold_cache_slow_downloader_all_succeed` and the dev-override
  family — all still pass under the proposed ordering.
- Hoisting `launcher`/`launcher_sig` above shim staging is provably safe: they
  depend only on `cache_dir`, `version` and `platform`, all resolved earlier,
  and their only consumers (`verify_launcher`, `fetch_and_verify`) are function
  bodies whose expansions resolve at call time.
- The trace-observability reasoning is empirically grounded and correct —
  redirections are invisible to xtrace, so the function token is the only
  reliable observable, and anchoring on `\++` rather than a fixed prefix depth
  correctly accommodates the command-substitution frame that
  `resolve_cache_dir` runs in.

**Findings**:

- 🟡 **major** (confidence: high) — *The exec-half positive control is vacuous:
  its regex also matches the probe's own variable assignment*
  **Location**: Phase 1, Change 2: Trace helpers (`_ran_probe_file`) — and
  Phase 2's use of it in `test_cold_path_enters_and_executes_the_probe`
  The plan's matcher for "the probe file was executed" is
  `rf"^\++{re.escape(_PROBE_FN)}:\S*\.accelerator-probe-\S*$"`, but
  `probe_exec_capable`'s first statement is `probe="$1/.accelerator-probe-$$"`,
  and bash's xtrace emits variable assignments too. The plan's own source
  research records the measured trace verbatim:
  `+probe_dir:probe=/…/cdA/.accelerator-probe-68245` sits three lines above the
  exec line `+probe_dir:/…/cdA/.accelerator-probe-68245`. The regex matches the
  assignment line as well — `\S*` happily consumes `probe=/…/cdA/` — so the
  assertion passes even if the `if "${probe}" >/dev/null 2>&1` block were
  deleted entirely.
  **Impact**: This is the plan's only verification of the retained cold-path
  probe's exec half (Phase 2 comments and Phase 4's recorded coverage
  limitation both say so explicitly, because the `chmod 0o666` cases cannot
  distinguish write from exec). As written, an implementation that regressed
  `probe_exec_capable` to a write-only check would keep the whole suite green —
  precisely the regression the work item calls load-bearing.
  **Suggestion**: Require the probe path to be the entire command word by
  anchoring immediately after the PS4 colon, e.g.
  `rf"^\++{re.escape(_PROBE_FN)}:/\S*\.accelerator-probe-\d+$"` — `probe=`
  after the colon then fails to match, while `rm -f …` and `chmod +x …` already
  fail on the embedded space. Add a red-step check that the assertion fails
  when the `if "${probe}"` branch is stubbed out, so the control is proven
  non-vacuous.

- 🔵 **minor** (confidence: high) — *Phase 1 introduces `re.search` but defers
  `import re` to Phase 2, so Phase 1 is not independently green*
  **Location**: Phase 1, Change 2: Trace helpers vs Phase 2, Change 1 ("Add
  `import re` to the module imports")
  Phase 1 adds `_entered` and `_ran_probe_file` to
  `tests/integration/entrypoint/test_accelerator_entrypoint.py`, both of which
  call `re.search`, but the instruction to add `import re` appears only in
  Phase 2, Change 1. The module currently imports `concurrent.futures`,
  `hashlib`, `os`, `platform`, `shutil` and `subprocess` — not `re`.
  **Impact**: Phase 1's own success criteria include `mise run
  build-system:check` and `mise run check`, and `pyproject.toml` sets
  `select = ["ALL"]` with `"tests/**"` ignoring only
  `["S", "ANN", "D", "PLR2004", "SLF001", "PT", "INP001"]` — Pyflakes `F821`
  (undefined name) is live on the test tree. Phase 1 would therefore land red,
  breaking the plan's stated "each phase is independently green and mergeable"
  property.
  **Suggestion**: Move `import re` into Phase 1, Change 2 alongside the helpers
  that use it, and drop the sentence from Phase 2.

- 🔵 **minor** (confidence: high) — *The `no_cache_dir` paragraph contradicts
  itself, and its stated reason is factually wrong*
  **Location**: Phase 2, Change 4 (the `no_cache_dir` paragraph)
  The paragraph opens "The existing `fail` at `:195-197` **is rewritten** to
  call the same wording via `no_cache_dir` so the substring has one definition"
  and then, in the very next sentence, concludes "the `:195` call site **keeps
  its own literal** `${plugin_root}/bin` message and only the two probe gates
  route through `no_cache_dir`". Only the second reading is implementable,
  since the plan places `no_cache_dir` after the dev-override block (past line
  233) — far below the `:195` call site, where calling it would be a
  `command not found`. The reason the plan actually gives is also wrong:
  `cache_dir` is not *unset* at `:195`, because `cache_dir=$(resolve_cache_dir)`
  assigns the (empty) substitution output even when the command substitution
  exits non-zero, so under `set -u` it is set-but-empty.
  **Impact**: An implementer who takes the first sentence at face value and
  hoists `no_cache_dir` above `:195` to "fix" the contradiction gets a
  diagnostic reading `no writable, exec-capable cache directory:  is not usable
  …` with an empty path — silently degrading the message that three acceptance
  criteria assert on. The paragraph also claims a single definition of the
  substring while delivering two.
  **Suggestion**: Delete the first sentence, state plainly that there are two
  `fail` sites sharing one substring, and give the real reason (definition
  order relative to `:195`). If one definition is genuinely wanted, define
  `no_cache_dir` above `resolve_cache_dir` and pass the offending directory as
  `$1` rather than closing over `${cache_dir}`.

- 🔵 **minor** (confidence: high) — *The `chmod 0o666` rationale contradicts the
  plan's own Key Discoveries: those cases never reach the exec branch*
  **Location**: Phase 2, Change 1: Regression cases — the paragraph justifying
  `chmod 0o666`
  The plan justifies the two `noexec` cases with "Both reach the probe's exec
  branch, but `0o666` leaves the directory writable, so `ensure_dir`'s
  `mkdir -p` and any preceding write still succeed". A directory with mode
  `0666` has no search bit, so no pathname inside it can be resolved and
  `printf … >"$1/.accelerator-probe-$$"` fails with EACCES. The plan's own Key
  Discoveries say exactly this ("a `chmod -x` directory permits `mkdir -p` on
  an existing path but fails writes"), as does the measured table in the source
  research (`write into it: FAILED (permission denied)`). So the probe fails at
  its *write* step and the `if "${probe}"` branch is never evaluated, and
  `0o666` on a directory *is* `chmod -x`, making the stated contrast with
  "`chmod -x` on the plugin root's `bin/`" vacuous.
  **Impact**: Both tests still pass (non-zero exit, correct substring), so
  nothing breaks — but the paragraph asserts coverage the cases do not provide,
  and directly contradicts the exec-vs-write limitation Phase 4 is required to
  record as acceptance criterion 5. An implementer trusting this paragraph
  could reasonably weaken or drop that record.
  **Suggestion**: Rewrite the paragraph to say the probe fails at its *write*
  step in both cases (which is why the exec half needs the trace positive
  control), and keep Phase 4's recorded limitation as the authoritative
  statement. Note that no directory-permission combination can produce
  exec-without-write, so this gap is inherent short of a real `noexec` mount.

- 🔵 **minor** (confidence: medium) — *Gate A does not cover shim staging when
  the launcher is cached but the staged shim is not*
  **Location**: Desired End State / Implementation Approach (Gate A's
  condition) and Phase 2, Change 4
  The Desired End State claims the `no writable, exec-capable cache directory`
  diagnostic "still fires on **every** path that cannot use its cache dir". One
  reachable state escapes both gates: cache dir unwritable-but-searchable
  (`0o555`), launcher present and `-x`, signature present — so Gate A does not
  fire — but the content-addressed staged shim absent or byte-mismatched.
  Control then reaches the staging body at `bin/accelerator:257` *before* Gate
  B, `cp` fails, and the run dies with `could not stage the verify shim into …`
  plus a `could not record to …` line from `fail_integrity`'s best-effort
  append into the same unwritable directory. Today the probe at `:195` fires
  first and produces the cache-dir message. This is the exact failure mode the
  plan identifies as the reason Gate A must precede staging — Gate A just does
  not cover this variant of it.
  **Impact**: A diagnostic regression (no hang) in a narrow but reachable
  state, e.g. a cache directory whose staged shim was pruned or manually
  deleted and which has since become read-only. It also falsifies the plan's
  own "every path" invariant, which future readers will rely on.
  **Suggestion**: Either soften the Desired End State claim and record this
  residual explicitly alongside the others in Phase 4, or add a third
  `probe_once` as the first statement inside the staging `if` body (before
  `cp`). That is free on the warm path — the body is never entered — and no-ops
  via `probed` on any run where Gate A already fired, so it breaks none of the
  existing tests I traced.

- 🔵 **minor** (confidence: medium) — *The shared diagnostic asserts "no
  ACCELERATOR_CACHE_DIR override was given" on paths where it was*
  **Location**: Phase 2, Change 4: `no_cache_dir` message body
  `no_cache_dir` interpolates `${cache_dir}` and then states "… is not usable
  and no ACCELERATOR_CACHE_DIR override was given (no XDG fallback)". Because
  the two probe gates run after `resolve_cache_dir`, `${cache_dir}` is the
  override whenever one is set — so the message now names the override
  directory and simultaneously claims no override was given.
  `test_cold_path_keeps_the_noexec_diagnostic` and
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` both set
  `ACCELERATOR_CACHE_DIR` and both hit this wording; they pass only because
  they assert on the leading substring.
  **Impact**: A self-contradictory operator-facing diagnostic on the two paths
  the plan is specifically preserving the diagnostic for. The pre-change
  message was wrong in a different way (it hardcoded `${plugin_root}/bin`), so
  this is a carried-forward defect the plan is now propagating to a new call
  site rather than a new bug.
  **Suggestion**: Split the trailing clause on whether `ACCELERATOR_CACHE_DIR`
  is set — e.g. `${ACCELERATOR_CACHE_DIR:+ (from ACCELERATOR_CACHE_DIR)}` —
  keeping the asserted `no writable, exec-capable cache directory` prefix
  byte-identical so the three criteria still match.

- 🔵 **minor** (confidence: low) — *The warming call asserts exit 0 from a real
  launcher invoked with no subcommand*
  **Location**: Phase 2, Change 1:
  `test_warm_path_survives_a_non_writable_cache_dir`
  The test's first statement is
  `assert _run_bootstrap(root, server, downloader).returncode == 0` against
  `make_harness(real_launcher=True)` — i.e. the real `accelerator` binary
  invoked with **no arguments**. `Cli` declares a required subcommand
  (`pub command: Command`, `cli/launcher/src/launch/inbound/cli.rs:11-14`), so
  the exit code depends on whether clap's derive implies
  `arg_required_else_help` at the root: if it does, `handle_parse_error` maps
  `DisplayHelpOnMissingArgumentOrSubcommand` to `ExitCode::SUCCESS`; if it does
  not, the `_` arm returns `ExitCode::from(1)`. Nothing in the repo pins this —
  all three existing `real_launcher=True` tests pass explicit subcommands, and
  the launcher's own tests only exercise `try_parse_from` with a subcommand
  present.
  **Impact**: If the second reading holds, Phase 2's first regression case
  fails on its warming line, for a reason unrelated to the probe. Low impact
  (trivially fixed during the red step) but it costs a debug cycle on a test
  whose whole point is a clean before/after signal.
  **Suggestion**: Warm with `args=("version",)` — the same built-in the test
  then asserts on. That removes the dependency on clap's missing-subcommand
  exit code and on the root-help path entirely.

- 🔵 **suggestion** (confidence: medium) — *Gate B — the anti-hang guard — has
  no automated case, and a regression would hang the suite rather than fail it*
  **Location**: Testing Strategy / Phase 2 Manual Verification step 2
  Gate B exists solely to keep the residual case (launcher `-x`, signature
  present, verification fails, cache dir unwritable) from spinning
  `acquire_lock` for ~30 s, and the plan's Key Discoveries call it "required to
  avoid a regression, not merely to preserve a nicer message". None of the six
  automated cases exercises it: five run cold (Gate A fires first) and the
  sixth is warm. The property is left to Manual Verification step 2 only.
  **Impact**: `run_bootstrap`'s `timeout` parameter defaults to `None`, so if a
  later change removes Gate B the suite does not go red — it stalls for the
  full lock budget on every affected invocation, which is a much harder failure
  to attribute than an assertion. The plan's own Performance Considerations
  note the residual is the reasoning most likely to be "removed by someone
  tempted to", making an automated pin worth its cost.
  **Suggestion**: Add a seventh case modelled on
  `test_tampered_cached_launcher_is_refused_and_healed`: warm the cache,
  overwrite the cached launcher with garbage (preserving its execute bit so
  Gate A stays quiet), `chmod 0o555` the cache dir in a `try/finally`, invoke
  with `timeout=10`, and assert non-zero plus the cache-dir substring. That
  fails fast today with the timeout and would fail fast if Gate B were removed.

### Test Coverage

**Summary**: The plan's six cases give solid coverage of the *warm-path*
behaviour change and, unusually well, guard the trace assertions against
vacuity with both an in-run control (`ensure_dir` entered) and a cross-test
positive control (the probe file observed executing). But the coverage is
lopsided: Gate B — the second probe call site the plan itself says prevents a
~30 second `acquire_lock` spin — has no automated case at all and would survive
deletion with all six cases green, and three of the six cases already pass
against today's `bin/accelerator`, so the Phase 2 success criterion "all six
new cases fail before the production change" cannot be met as written. A
secondary gap: after the change the `:195` `resolve_cache_dir` failure
diagnostic (kept as a second, duplicated copy of the message string) is
reachable by no test in the suite.

**Strengths**:

- The probe-absence assertion is protected against vacuity from two
  directions: `_entered(trace, "ensure_dir")` in the same run proves the trace
  mechanism is live, and `test_cold_path_enters_and_executes_the_probe` proves
  the token can match at all. A silently broken xtrace or PS4 cannot turn the
  negative assertion green.
- `_ran_probe_file`'s anchored regex genuinely isolates the probe's *exec* half
  from `chmod +x …` and `rm -f …` (both of which contain a space before the
  path). [Reviewer's note: the correctness lens found it does **not** isolate
  it from the `probe=…` assignment line — see that lens's major finding, which
  supersedes this strength.]
- The plan's claims about which existing tests stay green hold on reading
  them: `test_readonly_root_without_override_is_a_named_error` reroutes cleanly
  through Gate A, `test_a_record_is_always_one_line` still reaches the staging
  diagnostic, `test_tampered_cached_launcher_is_refused_and_healed` traverses
  Gate B without incident, and the dev-override tests are untouched because
  Gate A sits below the dev block.
- `_require_unprivileged()` is applied to exactly the three
  permission-dependent new cases (0o555 warm, and both 0o666 cases) and to a
  complete retrofit list — `test_dev_override_refused_when_not_executable` is
  correctly left alone, since a 0644 file has no execute bit for `-x` to find
  even as root.
- Using `chmod 0o666` on an override cache dir rather than `chmod -x` on the
  plugin `bin/` is a deliberate isolation choice: `ensure_dir`'s `mkdir -p`
  still succeeds on an existing directory, so the probe is unambiguously the
  failing step.
- Choosing a `bash_args` seam over the research's `SHELLOPTS=xtrace` avoids
  exporting xtrace into the injected downloader and the shim/launcher children.
- Every permission case restores modes in a `finally`, matching the established
  idiom in this suite and keeping `tmp_path` teardown viable.
- The exec-vs-write coverage limitation is recorded as a limitation rather than
  papered over, with the substitute (the positive control's execution
  assertion) named explicitly.

**Findings**:

- 🔴 **critical** (confidence: high) — *Gate B has no automated coverage*
  **Location**: Phase 2, item 4: Gate B / Success Criteria — Manual Verification
  The plan identifies Gate B (a second `probe_exec_capable` call site at the
  top of the cold branch, before `acquire_lock`) as necessary to stop a ~30
  second lock-timeout spin in the residual case — cached launcher present and
  executable, signature present, verification failing, cache dir unwritable —
  yet gives it **only a manual verification step** ("A tampered cached launcher
  in an unwritable cache dir fails within a second") and manual testing step 2.
  None of the six automated cases reaches that state: cases 5 and 6 use
  `chmod 0o666`, which makes `[[ -x "${launcher}" ]]` false and therefore
  routes through Gate A, and the existing
  `test_tampered_cached_launcher_is_refused_and_healed` uses a writable cache
  dir so Gate B succeeds trivially. Deleting Gate B leaves all six new cases
  and the whole existing suite green.
  **Impact**: The single riskiest line of the production change — the one whose
  absence turns an instant, correctly-named failure into a 30 second hang, on a
  file every SessionStart hook executes — has zero regression protection, and
  the plan's own Desired End State claims "no new hang … Verified by six
  regression cases". A future reader with no test to red will delete Gate B as
  redundant (the plan's own code comment anticipates exactly that temptation).
  **Suggestion**: Add a seventh case; the state is cheap to construct and
  `run_bootstrap` already turns a hang into a named failure via its `timeout`
  parameter. Warm the cache, poison the launcher in place
  (`launcher.write_text("poisoned")` keeps its executable bit and leaves the
  `.minisig` present), then `bin_dir.chmod(0o555)` — 0o555 rather than 0o666 is
  essential so the directory stays traversable and `-x launcher` remains true,
  bypassing Gate A — and run with `timeout=15`, asserting a non-zero exit and
  the `no writable, exec-capable cache directory` substring. Restore the mode in
  `finally` and gate the case with `_require_unprivileged()`. While there, pass
  an explicit `timeout=` to the other permission cases so any future
  re-introduction of the lock spin reports as a hang rather than a slow pass.

- 🟡 **major** (confidence: high) — *"All six new cases fail before the
  production change" is unachievable*
  **Location**: Phase 2, Overview and Success Criteria
  The plan commits to writing the six cases first and observing each fail "for
  the right reason", and makes that a Phase 2 automated success criterion. Read
  against today's `bin/accelerator`, three of the six already pass:
  `test_cold_happy_path_creates_a_missing_cache_dir` passes because
  `probe_dir`'s `mkdir -p` already creates the nested override directory and the
  cold fetch already prints real `version` output;
  `test_cold_path_keeps_the_noexec_diagnostic` passes because `mkdir -p`
  succeeds on an existing 0o666 directory and the probe's *write* then fails,
  producing exactly the asserted exit code and substring; and
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` passes for the
  same reason (the plan admits this one, but not the other two). Of the
  remaining three, `test_warm_path_does_not_enter_the_probe` and
  `test_cold_path_enters_and_executes_the_probe` fail only because the function
  names `ensure_dir` and `probe_exec_capable` do not exist yet — a rename
  artefact, not a behavioural discriminator. Only
  `test_warm_path_survives_a_non_writable_cache_dir` reds for a behavioural
  reason.
  **Impact**: The stated red step is unachievable, so Phase 2 either stalls on
  a criterion that cannot be ticked or the record gets fudged. More importantly
  it obscures the real coverage shape: four of the six cases are
  characterisation tests that pin behaviour the change must *preserve*, not
  regression guards for the change itself, and the plan's framing ("six
  regression cases") over-states the confidence they buy.
  **Suggestion**: Reword the criterion to name which cases are expected to red
  pre-change (case 1 behaviourally; cases 2 and 3 on the new function names) and
  which are pre-existing-behaviour guards that must be green on both sides
  (cases 4, 5, 6) — the latter is a genuine and useful property to record, since
  a green-before/green-after case proves the change preserved the diagnostic
  rather than merely that it fires.

- 🟡 **major** (confidence: high) — *After the change the `:195` diagnostic is
  reachable by no test*
  **Location**: Phase 2, items 3 and 4: `resolve_cache_dir` and the two
  diagnostic call sites
  After the split, `resolve_cache_dir` fails only when `mkdir -p` fails, and
  the plan deliberately keeps the `:195` `fail` with its own hand-written copy
  of the `no writable, exec-capable cache directory` wording. No test in the
  suite reaches that branch any more: its only current cover,
  `test_readonly_root_without_override_is_a_named_error`, reroutes through Gate
  A's `no_cache_dir` once `mkdir -p` starts succeeding on the existing 0o555
  `bin/`, and every other test either creates its cache dir first or uses the
  default `bin/`, which `make_installation` always creates.
  **Impact**: A new behaviour (the diagnostic now firing on an *uncreatable*
  directory) ships with no coverage, and one of two duplicated definitions of a
  user-facing diagnostic string is unreachable by any assertion — it can
  silently drift from the other, or be broken outright, with nothing red.
  Mutating the `:195` message to garbage would leave `mise run check` green.
  **Suggestion**: Add one cheap case that makes `mkdir -p` fail: create
  `parent = tmp_path / "ro"`, `parent.mkdir()`, `parent.chmod(0o555)`, set
  `ACCELERATOR_CACHE_DIR=str(parent / "nested")`, restore in `finally`, and
  assert a non-zero exit plus the `no writable, exec-capable cache directory`
  substring under `_require_unprivileged()`. That covers both the retained
  `:195` diagnostic and `ensure_dir`'s failure return in a single test.

- 🔵 **minor** (confidence: high) — *Trace cases hard-code production function
  names with no fast guard*
  **Location**: Phase 2, item 1: trace helpers (`_PROBE_FN`) / Testing Strategy
  — Unit Tests
  The two trace cases assert on hard-coded production function names
  (`_PROBE_FN = "probe_exec_capable"` and the literal `"ensure_dir"`) with
  nothing tying those constants to `bin/accelerator`. The Testing Strategy
  leaves the unit tests "untouched". If the production function is renamed,
  `test_cold_path_enters_and_executes_the_probe` fails after a full cargo build
  and a real fetch-verify-cache round trip, reporting "probe not entered" rather
  than "the name moved", while `test_warm_path_does_not_enter_the_probe`'s
  negative assertion silently becomes vacuous.
  **Impact**: A rename produces a slow, misleading integration failure instead
  of a fast, self-explaining one, and half the pair degrades to a no-op in the
  meantime.
  **Suggestion**: Add a one-line guard to
  `tests/unit/tasks/test_bootstrap_coverage.py`, which already uses exactly
  this idiom (`assert _KEY in _BOOTSTRAP_SRC.read_text()`): assert that both
  `probe_exec_capable()` and `ensure_dir()` appear as function definitions in
  `bin/accelerator`, with a comment naming the trace assertions that depend on
  them. It runs in milliseconds and turns a rename into a pinpointed failure.

- 🔵 **minor** (confidence: medium) — *Warming invocations are barely checked*
  **Location**: Phase 2, item 1:
  `test_warm_path_survives_a_non_writable_cache_dir` and
  `test_warm_path_does_not_enter_the_probe`
  In the first, the warming call is
  `assert _run_bootstrap(root, server, downloader).returncode == 0` — a **bare**
  invocation of the *real* launcher with no subcommand, and with no output
  attached to the assertion. Its exit status rests on clap mapping a missing
  required subcommand to `DisplayHelpOnMissingArgumentOrSubcommand`, which
  `cli/launcher/src/main.rs` maps to `ExitCode::SUCCESS`; no test anywhere pins
  that bare-invocation contract. In the second the warming call's return code is
  not asserted at all, so a failed warm-up silently turns the "warm" trace into
  a cold trace and the failure surfaces as a confusing "probe was entered".
  **Impact**: Two of the three genuinely discriminating cases can fail or
  mislead for reasons unrelated to the probe, with no diagnostic output attached
  — expensive to debug in an integration suite that builds the launcher and runs
  a full fetch-verify-cache chain.
  **Suggestion**: Warm with `args=("version",)` in case 1 (the same invocation
  the case then asserts on) and attach `result.stdout + result.stderr` to the
  assertion message, as every other test in the file does; and assert the
  warming run's return code in case 2 the way
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` already does for
  its own first run.

- 🔵 **minor** (confidence: medium) — *Criterion-1 assertion is looser than it
  needs to be*
  **Location**: Phase 2, item 1 / Phase 4, criterion-1 amendment
  Acceptance criterion 1 asks for the `version` output to be "asserted
  exactly"; the plan weakens this to `stdout.startswith("accelerator ")` plus
  the presence of the `commit: `, `built:  ` and `target: ` prefixes, and
  records the weakening as a deviation on the grounds that the real launcher
  prints `CARGO_PKG_VERSION` rather than the fixture's `9.9.9-test`. That
  reasoning is sound, but the resulting assertion would pass on truncated,
  reordered or partially-empty field values.
  **Impact**: The case's job is to prove the warm path actually ran the cached
  launcher end to end; a prefix-and-substring check leaves room for a degraded
  launcher run to pass, and the recorded deviation reads as accepting less
  coverage than was available.
  **Suggestion**: The `launcher_bin` fixture is already in scope (the case sets
  `real_launcher=True`). Run `launcher_bin` directly with `version` once and
  assert **stdout equality** between that and the bootstrap's stdout. That
  satisfies the criterion's "exactly" without hard-coding a version, and
  additionally proves the cached binary is the same one the fixture built.

- 🔵 **minor** (confidence: medium) — *Nothing checks the probe file is removed*
  **Location**: Phase 2, item 2: `probe_exec_capable` / Phase 2, item 1:
  `test_cold_path_enters_and_executes_the_probe`
  The split rewrites the probe's cleanup entirely — `rm -f "${probe}"` now
  appears at three separate exits of the new `probe_exec_capable` — and no
  assertion anywhere checks that the probe file is actually removed. Nothing in
  the suite would catch a lost `rm -f`, because every test uses fixture roots
  (the session-scoped `repo_bin_is_untouched` fixture only guards the repo's own
  shipped `bin/`).
  **Impact**: A dropped cleanup path would leave `.accelerator-probe-<pid>`
  litter in every real user's plugin `bin/` on each cold start, and the plan
  explicitly declines to add a `.gitignore` entry for that pattern — so the
  litter would be visible in contributors' working trees with no test to have
  caught it.
  **Suggestion**: Add one line to
  `test_cold_path_enters_and_executes_the_probe`, which already owns the
  cold-probe run:
  `assert not list(cache.glob(".accelerator-probe-*")), "the probe must clean up after itself"`.

- 🔵 **minor** (confidence: medium) — *Measurement loop's clock reads add a
  constant to every sample*
  **Location**: Phase 4, item 1: Measurement
  The measurement loop reads the clock with a separate
  `python3 -c 'import time; print(time.time_ns())'` process on each side of the
  timed call. The interval therefore includes the *second* interpreter's startup
  (the clock is read only after Python has booted), adding a constant ~25-50 ms
  to every sample — comparable to the ~41 ms figure being measured.
  **Impact**: Both medians are inflated by roughly the same constant. The
  `after ≤ 0.5 × before` ratio is biased toward *failing* a correct
  implementation (0.41 rather than 0.28 on the plan's own expected figures), and
  the recorded before-median will visibly disagree with the Context table's
  149.1 ms, which the acceptance criterion asks the method to match — making the
  recorded delta and gate hard to interpret for whoever re-confirms 0169's
  hand-off note against them.
  **Suggestion**: Take the timestamps from a single process outside the loop —
  e.g. drive the 20 iterations from one `python3 -c` script using
  `subprocess.run` and `time.perf_counter` — or use `hyperfine` if available,
  and record the method verbatim in Validation Results so the next measurement
  is reproducible (the plan already notes the original methodology was never
  recorded).

- 🔵 **suggestion** (confidence: medium) — *The hard-fail retrofit offers no
  sanctioned exclusion path*
  **Location**: Phase 1, items 3 and 4: `_require_unprivileged` and the retrofit
  The hard-fail-over-skip decision is well argued for the three new permission
  cases, and the retrofit list is complete. But retrofitting the same hard fail
  onto three *existing* tests converts a suite that currently passes under uid 0
  into one that reds six tests there, and the plan provides no mechanism for the
  "excluded explicitly by the implementer" escape the work item's
  acceptance-criteria preamble requires — the only recourse is editing the file
  or ad-hoc `--deselect`.
  **Impact**: Anyone running the entrypoint suite as root (a Docker devcontainer
  is the common case) hits six hard failures whose message names no supported
  way out, and a future lane exclusion has to be improvised rather than
  recorded.
  **Suggestion**: Keep the hard fail, but make the exclusion a first-class,
  greppable operation: mark the six cases with a named marker (e.g.
  `@pytest.mark.unprivileged`, registered in `pyproject.toml`) and have
  `_require_unprivileged`'s failure message name `-m 'not unprivileged'` as the
  sanctioned exclusion. That satisfies the criterion's demand for explicit
  exclusion without reintroducing a silent skip.

### Code Quality

**Summary**: The plan is unusually well-reasoned for its size: the split into
`ensure_dir` + `probe_exec_capable` is the right shape, the new state (`probed`)
mirrors the file's existing `lock_held` mutable-flag idiom, the tab-indent and
no-`local` conventions of `bin/accelerator` are respected, and the trace
assertions are wrapped in small named predicates with a positive control so a
rename reddens rather than silently passing. The main maintainability problems
are concentrated in Phase 2's Change 4: a self-contradiction about whether the
`no writable, exec-capable cache directory` wording is shared or duplicated, a
diagnostic that no longer describes the failure it now reports, a third copy of
the `-x launcher && -f sig` condition, and two helper names (`no_cache_dir`,
`probe_once`) that hide the fact they terminate the process. On the test side
the proposed `_traced` helper carries `**kwargs: object` plus two
`# type: ignore` suppressions that its own two call sites do not need.

**Strengths**:

- The `probed` flag plus `probe_once` mirrors the file's existing `lock_held` /
  `acquire_lock` / `release_lock` idiom, so the new mutable module-level state is
  not a new pattern for a reader to absorb.
- The plan explicitly records that `bin/accelerator` is tab-indented
  (`.editorconfig`'s `[*.sh]` does not match an extensionless file) and that the
  new functions must match — a genuine trap caught before implementation.
- The negative trace assertion is paired with a positive control
  (`test_cold_path_enters_and_executes_the_probe`) and an `ensure_dir` presence
  assertion, so renaming either half of the split reddens instead of quietly
  turning the absence check vacuous.
- The trace observables are wrapped in two small named predicates (`_entered`,
  `_ran_probe_file`) rather than inline regexes at each assertion site, and the
  traps behind them (redirections invisible to xtrace, the `probe` substring
  appearing in the filename regardless, variable `+` depth under command
  substitution) are each recorded.
- `_require_unprivileged`'s docstring is aimed squarely at the future reader who
  would 'fix' it back to the neighbouring `skipif` — exactly the kind of
  extremely-non-obvious signal the project's low-comment standard is meant to
  leave room for.
- Test-level comments follow the existing style in
  `test_accelerator_entrypoint.py` (a leading comment inside the test body
  stating why the case exists, e.g. at `:608`, `:629`, `:1067`) rather than
  inventing a new convention.
- 'What We're NOT Doing' is precise and keeps the production diff tiny — the
  shim staging block and its second `sha256_file` stay untouched, with the cost
  consciously retained.

**Findings**:

- 🟡 **major** (confidence: high) — *The `no_cache_dir` helper's purpose is
  contradicted in adjacent sentences, and the outcome duplicates the diagnostic
  wording*
  **Location**: Phase 2, Change 4
  The plan first states "The existing `fail` at `:195-197` is rewritten to call
  the same wording via `no_cache_dir` so the substring has one definition", then
  in the very next sentence states the opposite: "the `:195` call site keeps its
  own literal `${plugin_root}/bin` message and only the two probe gates route
  through `no_cache_dir`". The stated blocker is that `no_cache_dir` interpolates
  the global `${cache_dir}`, which is unset at `:195` — a problem created purely
  by the helper reading a global instead of taking the directory as an argument.
  **Impact**: An implementer cannot tell which shape to build, and the shape the
  plan lands on leaves the load-bearing string
  `no writable, exec-capable cache directory` written out twice in a 350-line
  script — with four new tests asserting on it as a substring, so a future reword
  of one site silently degrades the other's coverage.
  **Suggestion**: Delete the first sentence and parametrise the helper —
  `fail_no_cache_dir() { fail "no writable, exec-capable cache directory: $1 …"; }`
  — defined before `:195` and called as `fail_no_cache_dir "${plugin_root}/bin"`
  there and `fail_no_cache_dir "${cache_dir}"` from the gates. That genuinely
  gives the substring one definition and removes the hidden global coupling.

- 🟡 **major** (confidence: medium) — *After the split the diagnostic describes a
  capability the failing code no longer tests*
  **Location**: Phase 2, Change 3
  The plan keeps the wording `no writable, exec-capable cache directory` at
  `:195` while `resolve_cache_dir` is reduced to `mkdir -p` only — so that
  message now fires for "the directory could not be created", which is neither a
  writability nor an exec-capability result. The same string is then reused by
  the two probe gates for a genuinely different cause, and both paths swallow the
  OS error (`2>/dev/null` plus a bare `return 1`), so no errno reaches the
  operator.
  **Impact**: Three distinct root causes (path unreachable/ENOTDIR, unwritable
  directory, `noexec` mount) collapse into one message with no distinguishing
  detail, which is the diagnostic an operator has to work from at 3am for a
  failure mode that only occurs in awkward environments.
  **Suggestion**: Keep the asserted substring as a shared prefix and append the
  actual cause per site — e.g. `… cache directory: <dir> could not be created`
  versus `… cache directory: <dir> rejected an executable file (noexec?)`. The
  tests assert a substring, so both still pass, and consider letting the
  underlying `mkdir`/`chmod` stderr through rather than discarding it.

- 🔵 **minor** (confidence: high) — *`_traced`'s `**kwargs: object` plus two
  `# type: ignore`s is generality neither call site uses*
  **Location**: Phase 1, Change 2
  The proposed `_traced(harness, downloader, **kwargs: object)` pops `extra_env`
  out of an untyped kwargs bag and needs two `# type: ignore[arg-type]`
  suppressions to forward it. Both of its call sites in Phase 2 pass only
  `extra_env` — nothing else is ever forwarded. (The pattern is copied from
  `_run_and_capture_env` at `test_accelerator_entrypoint.py:723-745`, which has
  the same suppressions, so it will read as established rather than accidental.)
  **Impact**: Two type-checker suppressions are added to a brand-new helper for
  flexibility that is never exercised, and they blind the checker to the whole
  forwarded argument set — so a future typo in a keyword reaches `run_bootstrap`
  as a `TypeError` at run time rather than a `pyrefly` error.
  **Suggestion**: Give it the signature it actually needs:
  `def _traced(harness: Harness, downloader: Path, *, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]`
  and build the env inline (`{**(extra_env or {}), "PS4": _PS4}`). No `pop`, no
  suppressions, fully checked.

- 🔵 **minor** (confidence: high) — *`no_cache_dir` and `probe_once` read as a
  predicate and a no-op guard, but both can terminate the process*
  **Location**: Phase 2, Change 4
  `bin/accelerator` has an established naming family for process-terminating
  helpers — `fail` and `fail_integrity` — and every other bare-noun/verb helper
  (`dir_of`, `dev_launcher_contained`, `verify_launcher`, `ensure_dir`) either
  returns a value or a status. The proposed `no_cache_dir` reads as a condition
  test but calls `fail` and exits; `probe_once` reads as a cheap memoised probe
  but exits the process when the probe fails.
  **Impact**: A reader scanning the two new gates (`probe_once` on its own line
  inside an `if`, and again as the first statement of the cold branch) gets no
  signal that control may not return, which is exactly the kind of hidden effect
  that makes later restructuring of this region risky.
  **Suggestion**: Rename to join the existing family — `fail_no_cache_dir` for
  the diagnostic, and something honest for the gate such as
  `require_exec_capable_cache` (keeping the once-only behaviour inside it).

- 🔵 **minor** (confidence: high) — *Gate A adds a third copy of the
  cached-artefact condition, written in De Morgan form*
  **Location**: Phase 2, Change 4
  The test `[[ -x "${launcher}" ]] && [[ -f "${launcher_sig}" ]]` already
  appears twice in `bin/accelerator` — at the cache-hit branch (`:336`) and again
  in the re-check under the lock (`:341-342`). Gate A adds a third instance,
  negated into `[[ ! -x "${launcher}" ]] || [[ ! -f "${launcher_sig}" ]]`, which
  is the least readable of the three renderings of the same idea.
  **Impact**: "The cached artefacts are present" — a trust-relevant condition —
  would be encoded in three places, so adding or renaming a cached artefact means
  finding all three, and the plan's own framing of Gate A as "the cheap,
  side-effect-free cache-hit approximation" is only true by inspection of two
  distant sites.
  **Suggestion**: Extract a predicate alongside `verify_launcher`, e.g.
  `cached_artefacts_present() { [[ -x "${launcher}" ]] && [[ -f "${launcher_sig}" ]]; }`,
  then write Gate A as `cached_artefacts_present || probe_once` and collapse
  `:336`/`:341` onto it too. The word "approximation" then becomes literally true
  — the gate and the branch share one definition.

- 🔵 **minor** (confidence: high) — *Both halves of the split are left untidy: a
  redundant `|| return 1` and three `rm -f` cleanup sites*
  **Location**: Phase 2, Change 2
  `ensure_dir() { mkdir -p "$1" 2>/dev/null || return 1; }` is a one-statement
  wrapper whose `|| return 1` is pure noise — `mkdir`'s status is already the
  function's status — and every caller writes `ensure_dir … || return 1` anyway.
  `probe_exec_capable` is copied verbatim from `probe_dir`, carrying forward
  three separate `rm -f "${probe}"` cleanup sites for one temporary file.
  **Impact**: The plan's headline abstraction is a name over a single builtin
  call with dead control flow attached, and the retained probe keeps a three-way
  cleanup shape that is easy to break when edited — both are being written fresh
  here, so the tidying is free now and awkward later.
  **Suggestion**: Drop the `|| return 1` from `ensure_dir`, and state in the plan
  why it is a function at all (the `_entered(trace, "ensure_dir")` assertion
  needs a stable token — otherwise inline the `mkdir -p` at its two call sites
  and assert on `resolve_cache_dir` instead). In `probe_exec_capable`, collapse
  to one cleanup path: run `chmod +x … && "${probe}"`, capture `status=$?`,
  `rm -f`, `return "${status}"`.

- 🔵 **minor** (confidence: medium) — *`bash_args` widens a deliberately strict
  shared funnel, and contradicts the plan's own discovery*
  **Location**: Phase 1, Change 1
  The plan's Key Discoveries state "`SHELLOPTS=xtrace` enables tracing on bash
  3.2 with no `-x` flag, **via the existing `extra_env`**", yet Phase 1 still
  adds a general `bash_args: tuple[str, ...]` passthrough to `run_bootstrap` — a
  helper whose module docstring says it "is the single funnel every invocation
  passes… a caller cannot opt out of [its preconditions]" and which is shared
  with the skill-invocation suite.
  **Impact**: An arbitrary-interpreter-flags hole is opened in a deliberately
  narrow shared API without the plan reconciling it against its own finding that
  no signature change is needed, so the next reader cannot tell whether the seam
  was necessary or incidental.
  **Suggestion**: Either use the existing `extra_env` seam, or keep `-x` and
  record the reason (`SHELLOPTS` is exported and would trace the shim, downloader
  and launcher too, polluting the trace and the timings) — and in that case
  prefer a narrow `trace: bool = False` over an open `bash_args`.

- 🔵 **minor** (confidence: medium) — *Comment placement and cross-references:
  one comment sits above the wrong construct, three name other tests, and an
  existing rationale comment is dropped*
  **Location**: Phase 2, Changes 1, 3 and 4 (proposed snippets)
  Judged against the project's very-low tolerance for comments, most of the
  proposed comments earn their place (the `noexec` rationale on
  `probe_exec_capable`, the `_PS4` bash-3.2 note, the positive-control note).
  Three details do not: the "Gate on the cached artefacts rather than on
  verification…" comment is attached to `probe_once` but explains the `if`
  condition two lines below it; three test comments name sibling tests
  (`test_warm_path_does_not_enter_the_probe`,
  `test_cold_path_enters_and_executes_the_probe`) which will go stale on rename
  exactly as ADR/AC references do; and the `resolve_cache_dir` snippet is shown
  without the existing `:182-183` comment explaining the no-XDG-fallback choice.
  **Impact**: Plan snippets get transcribed close to verbatim, so a misfiled
  comment and a silently deleted rationale comment both ship — and the dropped
  `no XDG fallback` note is the one piece of genuinely non-obvious context on
  that function.
  **Suggestion**: Move the gating rationale directly above the `if`, replace the
  test-name cross-references with what they actually convey ("a non-fatal
  surviving probe is ruled out by the trace assertion, not by this case"), and
  show the `:182-183` comment as retained in the snippet.

- 🔵 **suggestion** (confidence: high) — *The mandatory permission-restore
  invariant is left to six hand-written `finally` blocks*
  **Location**: Phase 2, Change 1
  The plan notes that restoring modes in a `finally` "is mandatory — `tmp_path`
  teardown cannot remove an unwritable directory", then hand-writes that `chmod`
  / `try` / `finally` / `chmod` shape in three new cases, on top of the three
  existing ones (`test_accelerator_entrypoint.py:265-276`, `:286-292`,
  `:1076-1085`).
  **Impact**: A mandatory invariant enforced by convention across six sites will
  eventually be forgotten in the seventh, and the failure mode is a leaked
  unwritable directory that breaks teardown for the whole session rather than a
  clean test failure.
  **Suggestion**: Encode it once — a small
  `@contextlib.contextmanager def restored_mode(path: Path, mode: int)` that
  chmods on entry and back to `0o755` in its own `finally` — and use
  `with restored_mode(cache, 0o666):` in the new cases (retrofitting the existing
  three is optional).

- 🔵 **suggestion** (confidence: medium) — *Hoisting two of three lines splits a
  cohesive assignment block for no stated reason*
  **Location**: Phase 2, Change 4
  `launcher`, `launcher_sig` and `base_url` are one three-line block at
  `bin/accelerator:305-307`. The plan moves the first two up past the
  dev-override block and explicitly leaves `base_url` behind, without saying why;
  `base_url` depends only on `version` (set at `:137`) and so could move with
  them.
  **Impact**: Release-artefact path construction ends up at two sites roughly 100
  lines apart, so a later change to the naming scheme has two places to find
  rather than one.
  **Suggestion**: Move all three together, or state the reason for splitting them
  (e.g. keeping `base_url` adjacent to its only consumer, `fetch_and_verify`) so
  the next reader does not treat the split as an oversight.

### Architecture

**Summary**: The core split (`ensure_dir` for the always-run `mkdir -p`,
`probe_exec_capable` for the cold-path-only write-chmod-exec) is a genuine
cohesion improvement: `resolve_cache_dir` stops conflating path selection with
capability validation, and the trust chain (shim staging, signature
verification) is deliberately left untouched. The dominant architectural concern
is not inside `bin/accelerator` but at the boundary the plan draws around it:
the Rust launcher's `cli/launcher/src/launch/outbound/resolve/cache_root.rs` runs
the *same* write-chmod-exec probe unconditionally on every external-subcommand
dispatch, before its own cache-hit test — so the ~108 ms this plan removes from
bash returns in Rust on exactly the path the epic (0169's hook migration, and
every sub-binary after it) needs fast, and the plan's chosen measurement command
(`version`, a built-in) never traverses it. Secondary concerns: the
two-gates-plus-flag shape introduces an unguarded ordering coupling and a
duplicated cache-hit predicate where a failure-classification design would need
neither, and Phase 1 is not the independently-green slice it claims.

**Strengths**:

- The split into `ensure_dir` and `probe_exec_capable` gives each function one
  reason to change and names them the way a domain expert would ('make the
  directory exist' vs 'prove it can execute'), replacing a function whose name
  (`probe_dir`) described neither half accurately.
- The plan correctly refuses to touch the shim staging block and its second
  `sha256_file`, keeping a latency optimisation from eroding a trust boundary
  that three existing tests assert — an explicitly acknowledged and
  well-reasoned tradeoff rather than an optimised-away one.
- The shared-harness coupling is identified up front
  (`tests/integration/support/installation.py` also serves
  `tests/integration/skill-invocation/`) and the extension is designed to be
  backwards-compatible via a keyword-only parameter with an empty default.
- The new helper functions follow the file's established idiom (module-scope
  functions closing over resolved globals, aborts routed through
  `fail`/`fail_integrity` so the `--fail-safe` contract holds), so the change is
  architecturally consistent with the surrounding bootstrap rather than
  introducing a new style.
- Gate B is justified by a measured failure mode (a ~30 s `acquire_lock` spin)
  rather than by taste, and the plan explicitly supersedes the research's weaker
  'cosmetic diagnostic narrowing' framing — a good example of a resilience
  regression being caught during planning.
- The latency gate is expressed host-relative (`after ≤ 0.5 × before`, both
  medians in one session on the same launcher binary) rather than as a fixed
  delta, so it survives a faster or slower host.

**Findings**:

- 🟡 **major** (confidence: high) — *The launcher re-incurs the identical probe
  on every warm sub-binary dispatch, so the saving does not reach the paths that
  motivated it*
  **Location**: What We're NOT Doing — "Not touching
  cli/launcher/src/launch/outbound/resolve/cache_root.rs"
  The plan removes the write-chmod-exec probe from the bash bootstrap's warm path
  but declares the Rust launcher's own copy out of scope on the grounds that "the
  launcher's own probe runs after exec capability is already proven". In
  `cli/launcher/src/main.rs:65`, `cache_root::resolve` — and therefore
  `probe_writable_and_executable`, which writes `.accelerator-probe-<pid>`,
  `chmod`s it, execs it and removes it (`cache_root.rs:80-94`) — runs inside
  `LazyProductionResolver::resolve` on **every** external-subcommand dispatch,
  *before* `FetchVerifyCacheResolver`'s cache-hit test (`resolve/mod.rs:190`).
  Today's SessionStart hook escapes this because `hooks/config-detect.sh:13`
  invokes `config summary`, an in-process built-in; but 0169 serves
  `vcs guard`/`vcs detect` as dispatched sub-binaries (0169 work item
  `:193-194`), as do 0170/0171/0173. The out-of-scope justification is a
  *redundancy* argument — which is precisely the argument this plan uses to
  relocate the bash probe, and which applies verbatim to the launcher (a warm
  dispatch proves exec capability by exec'ing the cached sub-binary) — and it
  says nothing about cost.
  **Impact**: The work item's stated rationale ("Every SessionStart hook pays
  this cost today … and every future CLI-backed hook will") is only
  half-discharged: after this change a warm `accelerator vcs guard` still pays a
  fresh-file first-exec penalty of the same shape and, on darwin, plausibly the
  same ~97 ms magnitude — which would put 0169's `G ≤ 1.1 × B ≈ 38.6 ms` gate out
  of reach by a multiple, not by the ~2 ms the hand-off note anticipates. Phase
  3's documentation change ("A warm start neither writes nor probes") also
  becomes false at the system level once sub-binary dispatch is involved.
  **Suggestion**: Keep the Rust change out of this item if desired, but replace
  the redundancy justification with the cost one and act on it: measure one
  dispatched-subcommand invocation during Phase 4, raise a sibling work item for
  `cache_root.rs` (the same `ensure_dir` / lazy-probe split applies, gated on the
  sub-binary cache miss inside `FetchVerifyCacheResolver::resolve`), and add the
  launcher-side residual to the 0169 hand-off note alongside the ~11.7 ms staging
  residual.

- 🟡 **major** (confidence: high) — *The measurement command exercises a
  built-in, not the dispatch path the epic depends on*
  **Location**: Phase 4, §1 Measurement and §3 0169 hand-off note
  Phase 4 measures the median of 20 `bin/accelerator version` invocations and
  uses that figure both to clear the `after ≤ 0.5 × before` gate and to
  re-confirm 0169's hand-off note. `version` is a clap built-in dispatched
  in-process (`cli/launcher/src/main.rs:139-144, 191-201`); it never constructs
  `LazyProductionResolver`'s cache root, never resolves a sub-binary, and never
  execs a second binary. Every consumer the work item cites as the beneficiary —
  0169's `vcs guard` hook, and the sub-binaries in 0170/0171/0173 — goes through
  the external-subcommand route instead, which adds cache-root resolution (with
  its own probe), manifest/signature re-verification and a sub-binary exec.
  **Impact**: The gate can pass and 0169's note can be "re-confirmed" against a
  figure that structurally under-represents the warm cost of every path that will
  actually run in production, so the obligation the note exists to carry forward
  would be discharged on the wrong evidence.
  **Suggestion**: Take a second median in the same session against an
  external-subcommand invocation (any dispatched token available at the time, or
  `--help` augmentation as a proxy for resolver construction), record both, and
  state explicitly in Validation Results that the `version` figure covers
  bootstrap cost only — so 0169 inherits a number it can compare against its own
  `G`.

- 🟡 **major** (confidence: high) — *`bash_args` widens the shared harness funnel
  enough to bypass its own preconditions*
  **Location**: Phase 1, §1 Trace capture seam (`installation.py` `bash_args`)
  Phase 1 adds an unconstrained `bash_args: tuple[str, ...]` to `run_bootstrap`
  and splices it as `[BASH, *bash_args, str(entry), *args]`.
  `tests/integration/support/installation.py`'s module docstring states that
  `run_bootstrap` "is the single funnel every invocation passes, and it carries
  the network and working-tree preconditions — a caller cannot opt out of them",
  and `assert_hermetic` (`:301-321`) specifically refuses to run the repo's own
  `bin/accelerator` because doing so fetches the real release into the working
  tree. Because bash treats the first non-option operand as the script to run,
  any `bash_args` entry that does not begin with `-` silently becomes the script
  and demotes the validated `entry` to `$1` — the funnel then validates a path
  that is not the one executed. The research also showed the capability needs no
  interface change at all (`extra_env={"SHELLOPTS": "xtrace"}` works on the 3.2
  floor).
  **Impact**: A shared seam consumed by two suites gains an interface broader
  than the one thing it is needed for, and the broader form can defeat the
  precondition that exists to stop a test suite writing into the shipped `bin/`.
  **Suggestion**: Narrow the parameter to intent — a keyword-only
  `xtrace: bool = False` that the funnel translates into `-x` — or, if a generic
  passthrough is wanted, assert inside `run_bootstrap` that every `bash_args`
  element starts with `-` so the script operand can never be hijacked.

- 🟡 **major** (confidence: high) — *Phase 1 cannot meet its own exit criteria
  and is not an independently valuable slice*
  **Location**: Phase 1 (Overview, §2 Trace helpers) and Implementation Approach
  ("each is independently green and mergeable")
  The plan claims "Phases are ordered so each is independently green and
  mergeable", but Phase 1's `_entered` helper calls `re.search` while the
  instruction to "Add `import re` to the module imports" appears in Phase 2 §1;
  `tests/integration/entrypoint/test_accelerator_entrypoint.py` imports no `re`
  today (its import block is lines 1-47). Phase 1 as specified therefore fails
  its own success criteria (`mise run build-system:check`, `mise run check`) on
  an undefined name. Beyond that, three of Phase 1's four additions have no
  consumer until Phase 2 — `_traced`, `_ran_probe_file`,
  `_PROBE_FN = "probe_exec_capable"` (naming a shell function that does not yet
  exist) and the `bash_args` parameter — so the slice ships scaffolding for
  behaviour that is not there.
  **Impact**: The phase boundary gives the appearance of an independently
  mergeable increment while actually being a broken prefix of Phase 2, which
  undermines the review/merge gate the phase decomposition exists to provide.
  **Suggestion**: Reduce Phase 1 to the genuinely independent change —
  `_require_unprivileged` plus its retrofit onto the three existing permission
  tests, which closes a real false-negative today — and move the trace helpers,
  `_PROBE_FN`, the `re` import and the harness seam into Phase 2 where their
  first consumer lives.

- 🔵 **minor** (confidence: high) — *Gate A duplicates the cache-hit predicate,
  giving two places that encode "is the cache warm?"*
  **Location**: Phase 2, §4 (Gate A)
  Gate A tests `[[ ! -x "${launcher}" ]] || [[ ! -f "${launcher_sig}" ]]` while
  the warm/cold branch at `bin/accelerator:336` tests
  `[[ -x "${launcher}" ]] && [[ -f "${launcher_sig}" ]] && verify_launcher`.
  After the change, the knowledge "which cached artefacts constitute a warm
  cache" lives in two textually independent places ~30 lines apart, one being a
  hand-maintained negation of the other's first two clauses. Any future change to
  the cached artefact set (an added file, a renamed launcher, a third
  precondition at `:336`) updates one and not the other.
  **Impact**: Drift makes Gate A either probe on a warm call (silently giving
  back the latency win) or skip the probe on a cold one (moving the diagnostic
  back behind the staging `cp`), and the failure is a slow, quiet regression
  rather than a hard break.
  **Suggestion**: Extract the shared predicate once — e.g.
  `cached_launcher_present() { [[ -x "${launcher}" ]] && [[ -f "${launcher_sig}" ]]; }`
  — and write Gate A as `cached_launcher_present || probe_once` and the branch as
  `if cached_launcher_present && verify_launcher; then`, so the artefact set has
  one definition and the branch reads as "present *and* verified".

- 🔵 **minor** (confidence: medium) — *No alternative to the precondition-gate
  shape is considered; failure-classification would need neither the hoist, the
  flag, nor two gates*
  **Location**: Implementation Approach (two gates plus the `probed` flag)
  The plan settles directly on "keep the probe as a precondition, replicated at
  two chokepoints, made idempotent by a mutable global `probed`", which requires
  hoisting `launcher`/`launcher_sig` past unrelated code, a duplicated cache-hit
  predicate, and a second gate whose only reachable scenario is the
  verification-failed residual. An alternative shape is not discussed: make the
  capability check *reactive* — leave `probe_exec_capable` uncalled on every
  success path, and invoke it only from the existing failure branches that
  already know the cache dir is suspect (the staging `cp`/`chmod` failures at
  `:257-260`, and `acquire_lock`'s unrecoverable `mkdir`), to classify the failure
  and emit the `no writable, exec-capable cache directory` diagnostic instead of
  the generic one. That form needs no hoist, no flag, no cache-hit approximation,
  and additionally removes the ~108 ms probe from the cold *happy* path, where it
  is equally redundant (a successful cold run proves write and exec capability by
  staging and running the shim).
  **Impact**: The chosen shape adds hidden state and an ordering constraint to a
  linear 352-line trust-root script and leaves the probe on the cold happy path,
  when a smaller-footprint design appears to satisfy the same acceptance
  criteria.
  **Suggestion**: Either adopt the failure-classification shape or record in the
  plan why it was rejected (e.g. more failure sites to touch, or a preference for
  fail-fast preconditions in a security-sensitive entry point) — so a future
  editor tempted to "simplify away" Gate B or the flag can see the alternative
  was weighed.

- 🔵 **minor** (confidence: high) — *The lock component's inability to
  distinguish contention from an unusable directory is masked at the call site,
  not fixed*
  **Location**: Key Discoveries ("A pre-staging gate alone introduces a ~30
  second hang") and Phase 2 §4 (Gate B)
  The plan discovered that `acquire_lock` (`bin/accelerator:275-303`) spins its
  full 300 × 0.1 s budget when `mkdir "${lock_dir}"` can never succeed, because
  the loop treats "no pid file" as "a competitor is about to write one" and has
  no notion of an unrecoverable `mkdir`. Gate B fixes the one instance reachable
  today by probing at the call site immediately before `acquire_lock`, leaving
  the defect inside the component untouched. Note also that with the `probed`
  flag set by Gate A, Gate B is a no-op whenever Gate A already probed
  successfully, so a directory that becomes unusable between the two gates still
  reaches the 30 s spin.
  **Impact**: A resilience defect stays latent in the locking component; any
  future path that reaches `acquire_lock` without passing a gate (a new call
  site, a retry, or the narrow TOCTOU window) re-exposes a 30 s hang in a script
  that runs at every SessionStart.
  **Suggestion**: Classify the failure where it occurs — on the first `mkdir`
  failure with no pid file present, distinguish "cannot create anything here"
  (fail immediately via the cache-dir diagnostic) from genuine contention — so
  the timeout budget only ever applies to contention; at minimum, record the
  residual as a known limitation next to `acquire_lock`.

- 🔵 **minor** (confidence: medium) — *The hoist creates an ordering invariant
  that the new tests deliberately cannot catch*
  **Location**: Phase 2, §4 (hoisting `launcher`/`launcher_sig` above shim
  staging) and Testing Strategy
  The design depends on a three-way ordering that spans ~50 lines of otherwise
  unrelated code: the hoisted `launcher`/`launcher_sig` assignments, then Gate A,
  then the first write into `cache_dir` (shim staging at `:252-261`). A
  reordering that leaves `${launcher}` unset would fail loudly under `set -u`,
  but the more likely edit — inserting a *new* write into `cache_dir` above Gate
  A, exactly the situation the research found with the staging `cp` — is silent.
  The plan's own two `noexec` cases deliberately use `chmod 0o666` so that
  "`ensure_dir`'s `mkdir -p` and any preceding write still succeed", i.e. they
  are constructed not to notice a write moved above the gate; only the
  pre-existing `test_readonly_root_without_override_is_a_named_error` (`0o555`,
  default cache dir) would catch it, and only incidentally.
  **Impact**: The invariant that makes the diagnostic correct is recorded in
  prose and a comment on the gate, but is not expressed where a future editor
  would look (the hoisted assignments) nor pinned by an intentional test — so it
  can regress into the exact bug this plan was written to avoid.
  **Suggestion**: Put a one-line comment at the hoisted assignments stating "must
  precede Gate A, which must precede the first write into `cache_dir`", and
  either add a `0o555` override-dir case that pins the ordering or name the
  existing `readonly_root` test in the Testing Strategy as the ordering guard so
  its role is visible.

- 🔵 **suggestion** (confidence: high) — *The `no_cache_dir` helper does not
  achieve the single definition it is introduced for, and the plan contradicts
  itself about it*
  **Location**: Phase 2, §4 (`no_cache_dir` helper)
  Phase 2 §4 introduces `no_cache_dir` and states "The existing `fail` at
  `:195-197` is rewritten to call the same wording via `no_cache_dir` so the
  substring has one definition", then in the next sentence concludes the
  opposite: "the `:195` call site keeps its own literal `${plugin_root}/bin`
  message and only the two probe gates route through `no_cache_dir`". The net
  result is two copies of the test-asserted substring
  `no writable, exec-capable cache directory` with different interpolations, plus
  a helper whose stated purpose is unmet.
  **Impact**: The plan reads as undecided at the point of implementation, and a
  diagnostic string asserted by several tests ends up with two owners that can
  drift independently.
  **Suggestion**: Pick one shape and say so — either parameterise the helper
  (`no_cache_dir "${1}"`, defined before `:195` and called from all three sites
  with the relevant directory) so the wording genuinely has one definition, or
  drop the helper and duplicate the literal deliberately with a comment naming
  the other copy.

### Performance

**Summary**: This is a measurement-led change that targets the right bottleneck:
the ~108 ms probe is isolated by a clean attribution experiment (107.9 ms for a
freshly-written probe vs 10.6 ms to re-exec an existing one), and the ~41 ms
landing point is arithmetically consistent with research §12's full warm-path
decomposition (108 + 23 + 2.3 + 3 + 12 ≈ 149). The production change itself is
sound and adds essentially zero new warm-path work. The weak link is Phase 4:
the measurement harness brackets each timed call with two `python3 -c`
interpreter startups, one of which falls *inside* the timed interval and is
plausibly comparable in size to the ~41 ms it is trying to measure; and the
plan's residual accounting hands 0169 half the real retained cost (~11.7 ms
instead of the ~23 ms of double hashing its own Current State table says runs
warm), while mis-framing that cost as a security trade-off when it is dominated
by interpreter/process startup in `sha256_file`.

**Strengths**:

- The bottleneck is identified by measurement, not speculation, and the
  attribution is genuinely causal: the 107.9 ms vs 10.6 ms pair isolates macOS's
  first-exec check on a freshly written file rather than blaming filesystem work
  or the fetch-verify design generally.
- The ~41 ms prediction is arithmetically supported end-to-end — research §12's
  decomposition (probe ~108, sha256 ×2 ~23, verify ~2.3, launcher ~3,
  bash+uname+sed ~12) sums to ~149, so removing 108 leaves ~41 with no
  unexplained residue.
- The gate is a host-relative ratio rather than a fixed ms delta, which is the
  correct choice given the probe cost is itself host-specific, and the
  before-median is deliberately re-measured post-0182 rather than reusing the
  stale 149.1 ms reference.
- The production change adds no measurable warm-path cost: Gate A is two builtin
  `[[ ]]` stat tests, the hoisted `launcher`/`launcher_sig` assignments are pure
  parameter expansion, and cold-path cost is preserved exactly once via the
  `probed` idempotence flag.
- Placing Gate A after the dev-override block removes the 108 ms from the
  contributor dev-launcher path as well — a real per-SessionStart and
  per-Bash-tool-call win the plan correctly claims as a bonus.
- The plan declines to buy latency by weakening the staging block's tested
  planted-stub defence, and carries the residual forward as a dated hand-off to
  0169 rather than silently absorbing it.
- The incidental win is spotted and quantified in kind:
  `tests/integration/skill-invocation/` runs the real bootstrap once per `!`-site
  across 46 SKILL.md files, all warm after the first, so the suite gets
  materially faster for free.

**Findings**:

- 🔴 **critical** (confidence: high) — *Timing harness includes a full python3
  interpreter startup inside every measured interval*
  **Location**: Phase 4, item 1: Measurement (the bash timing loop)
  The Phase 4 measurement loop brackets each timed call with
  `start=$(python3 -c '…time_ns()')` and `end=$(python3 -c '…time_ns()')`. The
  `start` timestamp is taken at the *end* of the first interpreter's startup, but
  the `end` timestamp is taken at the end of the *second* interpreter's startup —
  so the measured interval is `bin/accelerator version` **plus one complete
  `python3` process startup** (fork, exec, dynamic linking, `site` processing,
  and — under this repo's mise-pinned Python 3.14 with an auto-activated `.venv`
  — possibly a mise shim exec in front of it). That overhead is plausibly 25–80
  ms on darwin, i.e. comparable to or larger than the ~41 ms after-median the
  plan is trying to measure.
  **Impact**: Two consequences, both damaging. (1) An additive constant `b` on
  both sides pulls the ratio toward 1.0: with a true 149 → 41 ms improvement and
  `b` = 30 ms the gate reads 0.40, with `b` = 80 ms it reads 0.53 and **fails a
  correct implementation**, consuming exactly the slack the gate was calibrated
  with. (2) The *absolute* after-median is the figure Phase 4 §3 hands to 0169,
  whose acceptance ceiling is ≈ 38.6 ms and whose decision turns on a ~2.4 ms
  overrun — an after-median inflated by tens of milliseconds would make that
  ceiling look wildly unreachable and push 0169 to relax its threshold far
  further than the evidence warrants.
  **Suggestion**: Drive the whole loop from a single interpreter —
  `python3 - <<'PY'` with `time.perf_counter()` around
  `subprocess.run(["bin/accelerator","version"], …)` — so there is exactly one
  startup for the whole batch and the timed interval contains only fork+exec+run.
  Alternatively use `$EPOCHREALTIME` from a bash 5 (the measurement script is not
  shipped, so the bash 3.2 floor does not apply to it). Whichever is chosen,
  validate the instrument first by timing `/usr/bin/true` in the same harness: it
  must read ≲ 2 ms, and that calibration figure should be pasted into Validation
  Results alongside the medians.

- 🟡 **major** (confidence: high) — *Residual warm-path cost stated as ~11.7 ms
  when both `sha256_file` calls stay, ~23 ms*
  **Location**: Performance Considerations (and the Current State Analysis
  table); Phase 4 §3 (0169 hand-off)
  Performance Considerations states "The residual warm-path cost is the shim
  staging condition's second `sha256_file` at ~11.7 ms". The plan's own Current
  State Analysis table contradicts this: it lists `:252-253` `sha256_file` **#1**
  over `shim_source` as "runs warm: yes" *and* `:255-256` (the condition
  containing `sha256_file` #2) as "runs warm: yes". Research §12 agrees — its
  decomposition row reads "`sha256_file` ×2 over the 475 K shim (`:252`, `:256`)
  | ~23 ms". So the retained hashing cost after this change is ~23 ms, not ~11.7
  ms; the ~11.7 ms figure is the size of a *different, out-of-scope change*
  (dropping the second hash), not the size of the residual. The plan's own 41 ms
  arithmetic only closes with 23 ms in it (23 + 2.3 + 3 + 12 ≈ 40); with 11.7 ms
  there is ~12 ms unaccounted for. Relatedly, the Desired End State sentence
  "performs no write, no `chmod` and no exec beyond the staged shim and the
  launcher itself" is not literally true after the change — the warm path still
  execs `mkdir`, `sed`, `uname` ×2, `shasum`/`sha256sum` ×2 and `awk` ×2; the
  accurate and load-bearing claim is "no exec of a *newly created* file".
  **Impact**: Phase 4 §3 instructs the implementer to "update the quoted residual
  figure if it moved" in 0169's Dependencies note. Handing 0169 an 11.7 ms
  residual when the real retained cost is ~23 ms halves the apparent size of the
  only remaining lever, and 0169's choice is precisely between relaxing its
  `G ≤ 1.1 × B` gate and removing residual cost.
  **Suggestion**: State the residual as the full post-change budget — ~23 ms
  hashing (both `:252` and `:256`), ~2.3 ms `verify_launcher` shim exec, ~3 ms
  launcher exec, ~12 ms bash startup + `uname` ×2 + `plugin.json` `sed` — and
  hand that composition, not a single number, to 0169. Fix the Desired End State
  wording to "no exec of a freshly written file".

- 🟡 **major** (confidence: high) — *The retained ~23 ms is process-startup cost
  in `sha256_file`, not an unavoidable trust trade-off*
  **Location**: What We're NOT Doing (shim staging / double hash); Performance
  Considerations
  The plan, the work item and 0169's hand-off all frame the retained hashing cost
  as a security trade-off: recovering it means weakening the tested planted-stub
  defence, so it stays. But 11.7 ms to SHA-256 a 475 KB file is ~40 MB/s — one to
  two orders of magnitude below arm64's hardware-accelerated SHA-256 throughput,
  so essentially none of that time is cryptographic work. It is process startup:
  on darwin `sha256sum` does not exist (coreutils is not in `mise.toml`'s
  `[tools]`), so `sha256_file` falls through to `shasum -a 256`, which on macOS
  is a **Perl script**, and each call additionally forks a command-substitution
  subshell and execs `awk '{print $1}'` to slice one field.
  **Impact**: The epic has concluded that ~23 ms of a ~41 ms warm path is only
  recoverable by dropping a hash — i.e. by weakening a trust boundary — when
  roughly 15–20 ms of it is recoverable by *tool selection alone*, hashing
  exactly the same two files with exactly the same algorithm and keeping every
  existing assertion intact. That would put the warm bootstrap near ~21 ms,
  comfortably inside 0169's ≈ 38.6 ms ceiling, and it changes 0169's decision
  from "relax the threshold or accept the overrun" to "meet the threshold".
  **Suggestion**: Do not absorb it here — the scope closure is correct — but
  record the finding in Validation Results and in the 0169 hand-off, and raise a
  follow-up: add a faster backend to `sha256_file`'s existing detection chain
  (e.g. prefer `openssl dgst -sha256`, a C binary, ahead of `shasum`) and replace
  `| awk '{print $1}'` with pure parameter expansion on the captured output
  (`${out%%[[:space:]]*}` / `${out##* }`), eliminating two `awk` execs. Both
  hashes and both defences survive untouched.

- 🟡 **major** (confidence: high) — *The latency gate can pass silently on a
  measurement that measured nothing*
  **Location**: Phase 4, item 1 (Gate: `after ≤ 0.5 × before`)
  The measurement script runs under `set -uo pipefail` with no `-e`, discards
  both streams with `>/dev/null 2>&1`, and never checks an exit status. If
  `python3` fails or is unresolvable, `start` and `end` are set to the *empty
  string* — `set -u` does not catch this, because they are set — and
  `$(( (end - start) / 1000000 ))` evaluates empty operands as 0, so every
  iteration prints `0` and the awk median prints `0 ms`, trivially satisfying
  `after ≤ 0.5 × before`. Equally, nothing verifies that the timed invocation
  succeeded or was actually *warm*: the launcher filename embeds `${version}`
  read from `.claude-plugin/plugin.json`, so the plan's premise that the cached
  launcher survives the `jj new` revision switch ("measure the after-median
  without re-warming") holds only if both revisions carry the same version string
  — otherwise one batch times a cold fetch or a fast failure. The awk median also
  reads `v[10]`/`v[11]` without checking that 20 samples arrived.
  **Impact**: The gate is this plan's only quantitative acceptance condition and
  the sole input to a downstream decision in 0169; as written it can report a
  green pass on a harness that produced no valid samples, or on batches that
  measured different code paths.
  **Suggestion**: Assert per iteration that the invocation exits 0 and its stdout
  begins with `accelerator `; assert `NR == 20` in the awk stage and fail loudly
  otherwise; and before/after each batch record the cached launcher's path, size
  and inode (`ls -li`) so "same post-0182 binary, warm on both sides" is
  *verified* rather than assumed.

- 🟡 **major** (confidence: medium) — *Sequential batches across a jj working-copy
  swap alias drift onto the result and ease the gate*
  **Location**: Phase 4, item 1 (Sequence: `jj new`, warm, before; return, after)
  The protocol takes all 20 `before` samples immediately after `jj new` onto the
  pre-change revision — i.e. immediately after jj rewrites the working copy and
  snapshots it, when fsevents/Spotlight and any file-watcher activity are at
  their peak — then switches back and takes all 20 `after` samples once the
  machine has settled. Bare medians are recorded, truncated to whole
  milliseconds, with no dispersion.
  **Impact**: Any monotonic drift between the two batches is aliased entirely
  onto the before/after difference, and the specific ordering here biases in the
  *permissive* direction: post-swap background activity inflates `before`, making
  `after ≤ 0.5 × before` easier to clear. For the gate that is tolerable given a
  3.6× expected effect, but the absolute after-median is what 0169 compares
  against a 38.6 ms ceiling, where 1 ms quantisation and unknown run-to-run
  spread are not adequate.
  **Suggestion**: Remove the revision switch from the measurement entirely — copy
  the pre-change script to `bin/accelerator-before` (same directory, so
  `BASH_SOURCE`-derived `plugin_root`, `version` and the cached launcher path are
  all identical) and interleave the two variants sample-by-sample in one
  revision, deleting the copy afterwards. If the switch is kept, run the batches
  ABBA (before, after, after, before) and require the two `before` medians to
  agree within a few percent as a drift check, and let the working copy settle
  (`jj st`, brief pause) before timing. Raise n to 50 — the after batch is only
  ~2 s — and record min, median and p90 at 0.1 ms resolution rather than a single
  truncated integer.

- 🟡 **major** (confidence: medium) — *`ensure_dir` forks an external `mkdir` on
  every warm invocation for a directory that always exists*
  **Location**: Phase 2, Change 2 (`ensure_dir`)
  The plan's new `ensure_dir` is `mkdir -p "$1" 2>/dev/null || return 1`, and it
  is deliberately kept on the unconditional path via `resolve_cache_dir`. `mkdir`
  is an external binary, not a shell builtin, so every warm invocation pays a
  fork+exec (~1–3 ms on darwin) to create a directory that is guaranteed to exist
  — the launcher being execed out of it is proof. Guarding it costs nothing and
  changes no behaviour: `mkdir -p` on an existing directory is already a no-op,
  and a path existing as a *file* still fails `[[ -d ]]`, still runs `mkdir -p`,
  and still returns 1, so the failure mode and the diagnostic are identical.
  **Impact**: 1–3 ms of a ~41 ms budget is small in isolation, but this plan is
  authoring that exact function, and the downstream consumer (0169) is currently
  projected to miss its ≈ 38.6 ms ceiling by ~2.4 ms — this one guard is of the
  same order as the entire shortfall.
  **Suggestion**: Write
  `ensure_dir() { [[ -d "$1" ]] || mkdir -p "$1" 2>/dev/null || return 1; }` —
  bash-3.2-safe, tab-indented, behaviourally identical, and it removes the last
  fork the always-run half performs on a warm call.

- 🔵 **suggestion** (confidence: medium) — *No expected-composition budget, and
  the 0.5x gate leaves ~33 ms of unnoticed slack*
  **Location**: Desired End State; Performance Considerations; Phase 4 Success
  Criteria
  The plan predicts ~41 ms from `149.1 − 107.9` while simultaneously recording
  (via its research) that the Context table's methodology "was never recorded" —
  so the 107.9 ms subtrahend may itself carry unknown per-iteration harness
  overhead, giving the prediction several milliseconds of untracked uncertainty.
  Meanwhile the gate is `after ≤ 0.5 × before`: against a ~149 ms before that is
  a 74.5 ms ceiling versus a ~41 ms expectation, i.e. ~33 ms of slack. An
  implementation that left a third of the probe cost behind, or that introduced
  33 ms of new warm-path work, would clear the gate unremarked.
  **Impact**: A pass on the gate does not by itself confirm the probe was fully
  removed, and there is no diagnostic path if the after-median lands at, say, 60
  ms — it would be impossible to tell a partial removal from instrument overhead
  from a new cost.
  **Suggestion**: Record a predicted composition in Phase 4 (~23 ms hashing,
  ~2.3 ms verify, ~3 ms launcher, ~12 ms bash+`uname` ×2+`sed`, ≈ 41 ms total)
  and add a manual criterion that a deviation of more than ~25% from the
  prediction must be attributed before the figure is recorded — a `bash -x` run
  with per-line timestamps, or `sample`/`dtrace`, is enough. That turns the gate
  from a floor into a check that the expected cost actually left.

- 🔵 **suggestion** (confidence: medium) — *Remaining per-call fork inventory is
  unenumerated despite being the next lever for 0169*
  **Location**: Performance Considerations
  After this change a warm `bin/accelerator` call still performs roughly six
  forks and five execs before it reaches `exec "${launcher}"`: `mkdir` (inside
  `resolve_cache_dir`'s own command-substitution subshell), `sed` over
  `plugin.json`, `uname -m` and `uname -s` as two separate command substitutions,
  `shasum`/`sha256sum` + `awk` twice, and two nested subshells for the
  `cd -P`/`pwd -P` plugin-root resolution. Research §12 labelled the
  `bash startup + uname ×2 + sed` bucket (~12 ms) "partly" addressable and
  computed a ~18 ms fully-optimised floor, but neither the work item nor this
  plan enumerates what is left or how big each piece is.
  **Impact**: 0169 must decide whether to relax its `G ≤ 1.1 × B` gate on the
  strength of what this plan records. Without an inventory of the remaining
  levers and their sizes, that decision is made against a single aggregate
  number.
  **Suggestion**: Add the fork inventory to Phase 4's Validation Results entry
  alongside the medians, calling out the two obvious cheap ones for a future
  item: collapsing `uname -m` + `uname -s` into one `uname -sm` (or preferring
  bash's `$OSTYPE`/`$MACHTYPE` with a `uname` fallback) while keeping both
  `ACCELERATOR_UNAME_S`/`_M` test seams independent, and dropping the two `awk`
  execs from `sha256_file` in favour of parameter expansion. Neither belongs in
  this plan; both belong in the record 0169 reads.

### Portability

**Summary**: The plan is unusually careful about the environment it targets: it
verifies the CI matrix has no `container:` key before relying on unprivileged
lanes, spots the `.editorconfig` `[*.sh]` glob not matching the extensionless
`bin/accelerator` (so tabs are required), keeps the probe's `#!/bin/sh`
internals untouched, and writes a depth-tolerant trace regex plus a
`${FUNCNAME[0]:-main}` default instead of assuming one interpreter's xtrace
shape. I could not find a bash-4-only construct anywhere in the proposed diff —
nothing in it trips `scripts/lint-bashisms.sh`'s denylist, and the
backslash-continued double-quoted `fail` string matches the existing
bash-3.2-safe idiom at `bin/accelerator:196`. The residual risks are all about
the two environments the new tests must span: every empirical claim in Key
Discoveries was measured on darwin/bash 3.2 with BSD userland, the `chmod 0o666`
proxy for `noexec` behaves differently from a real `noexec` mount (the
Linux-prevalent case) and routes through a different gate, and the Phase 4
measurement uses `version` — the one command that by design never reaches the
launcher's own write-chmod-exec cache-root probe, which still pays the same
macOS first-exec penalty on every path a hook actually takes.

**Strengths**:

- The unprivileged-lane assumption is verified against the workflow rather than
  assumed: `.github/workflows/main.yml`'s `test-integration` matrix is
  `ubuntu-latest`/`macos-latest` with no `container:` key, so both GitHub-hosted
  lanes genuinely run as a non-root user.
- The `.editorconfig` interaction is correctly diagnosed: `[*.sh]`
  (space/2/switch_case_indent) does not match the extensionless
  `bin/accelerator`, `tasks/shared/sources.py:110` adds it to the shfmt/ShellCheck
  scope explicitly, and shfmt is invoked with no formatting flags — so shfmt
  defaults (tabs, no case indent) apply and the plan's tab-indented snippets are
  right. `>"${probe}"` with no space also matches shfmt's default `-sr=off`.
- No construct in the diff is bash-4-only or on `lint-bashisms.sh`'s denylist (no
  associative arrays, namerefs, `mapfile`, case-modification expansions, `&>>`,
  `|&`, negative subscripts, or escaped braces in a `:-` default), and
  `${FUNCNAME[0]:-main}` is valid in 3.2.
- The trace matcher allows one-or-more `+` rather than a fixed prefix depth,
  which is the right defence against bash's PS4 first-character replication
  differing by nesting context and interpreter version.
- The probe's internals are unchanged — same `printf '#!/bin/sh\nexit 0\n'`,
  `chmod +x`, exec, `rm` — so no new interpreter, shebang, or filesystem
  assumption is introduced; only the call sites move.
- Every permission-mutating test restores the mode in a `finally`, which is what
  keeps pytest's `tmp_path` teardown working identically on macOS and Linux.
- The new `version`-output assertions are triple-agnostic (`accelerator ` prefix
  plus `commit: `/`built:  `/`target: ` prefixes), and
  `cli/launcher/src/version/inbound/cli.rs:9` renders all four lines
  unconditionally with `unknown` fallbacks — so the same case passes on
  darwin-arm64 and linux-x64.
- Migration Notes are correct that no on-disk format or environment contract
  changes, so a cache populated before the change stays warm afterwards in any
  environment.

**Findings**:

- 🟡 **major** (confidence: high) — *Gate B — the only guard for a real noexec
  mount on a warm cache — gets no automated coverage on either lane*
  **Location**: Phase 2, Section 1 (the two `noexec` cases); and Section 4 (Gate
  B)
  The plan retains a second probe call site (Gate B, at the top of the cold
  branch) but no new automated case reaches it. Both `chmod 0o666` cases clear
  the directory's *search* bit, which makes `[[ -x "${launcher}" ]]` and
  `[[ -f "${launcher_sig}" ]]` fail to stat and therefore routes them through
  **Gate A**. A genuine `noexec` mount is the opposite shape: the mode bits are
  intact, so `[[ -x launcher ]]` is true and `[[ -f sig ]]` is true, Gate A does
  not fire, `verify_launcher`'s exec of the staged shim fails, and only Gate B
  produces the named diagnostic. Real `noexec` mounts (hardened `/tmp`, container
  volumes, NFS, `ACCELERATOR_CACHE_DIR` pointed at a noexec path) are far more
  common on Linux than on macOS, so the code path with zero automated coverage is
  precisely the Linux-relevant one — and the plan itself relegates it to Manual
  Testing Step 2.
  **Impact**: A future edit that removes or reorders Gate B would silently
  restore both the ~30 s `acquire_lock` spin and the loss of the
  `no writable, exec-capable cache directory` diagnostic on Linux hosts with a
  noexec cache dir, with the entire suite still green on both lanes.
  **Suggestion**: Add one automated case that reaches Gate B without touching the
  search bit — warm the cache, overwrite the launcher bytes so `verify_launcher`
  fails while `-x`/`-f` stay true, `chmod 0o555` the cache dir so the probe's
  write fails, then assert a non-zero exit with the cache-dir substring under a
  short `timeout=` (which also pins the anti-hang property the plan justifies
  Gate B with). That is the manual step, automated, and it is the only case that
  distinguishes Gate B from Gate A.

- 🟡 **major** (confidence: medium) — *All trace-shape evidence is from macOS bash
  3.2; nothing in Phase 2 verifies the assertions under the linux lane's bash 5.x*
  **Location**: Key Discoveries (PS4/trace bullets) and Phase 1, Section 2
  The test harness pins `BASH = "/bin/bash"`
  (`tests/integration/support/installation.py:41`), which is **bash 3.2.57 on the
  darwin lane and bash 5.2 on the ubuntu lane**. Every trace observation in Key
  Discoveries — the `set -u`/`FUNCNAME[0]` breakage, the `+probe_dir:printf …`
  shape, the exec line as its own command word, the `++` depth inside `$( )` — is
  measured on this darwin host only, yet the two trace assertions are the
  load-bearing criteria (work-item criteria 2 and 3) and must pass on both lanes.
  The `set -u` failure mode in particular is not merely noisier on bash 5: an
  unbound expansion in a non-interactive bash 5 aborts the shell, so the `:-main`
  default is load-bearing on Linux for a *different* reason than the one
  recorded, and Phase 2's Success Criteria list only local darwin tasks while
  cross-lane observation is deferred to Phase 4. Note also that the recorded
  alternative, `SHELLOPTS=xtrace`, is **not** equivalent to `bash_args=("-x",)`:
  `SHELLOPTS` is exported, so it is inherited by nested shells (including the
  probe's own `#!/bin/sh`, which is bash on macOS and dash on Linux) and appears
  in the launcher env dumps — making trace content platform-dependent in a way
  the `-x` flag is not.
  **Impact**: A bash-5-specific difference in traced-word quoting, prefix depth,
  or the set of traced lines would only surface as a red ubuntu lane after Phase
  2 has been declared "independently green and mergeable" on darwin.
  **Suggestion**: Add a cross-interpreter verification step to Phase 2 rather
  than Phase 4 — e.g. an opt-in interpreter override in the harness (defaulting
  to `/bin/bash` so the floor is still the default) and a recorded run of the two
  trace cases under a bash 5.x (Homebrew bash or a `bash:5` container). Keep `-x`
  and note in a comment that `SHELLOPTS` is rejected because it leaks into
  descendant shells whose identity differs by platform.

- 🟡 **major** (confidence: medium) — *The same write-chmod-exec probe still runs
  in the launcher for every external subcommand, so the docs claim and the
  `version`-based measurement do not describe the paths hooks take*
  **Location**: Phase 3, Sections 1 & 2; Phase 4, Section 1
  The cost being removed is a macOS-specific first-exec penalty, and an identical
  write-chmod-exec-rm probe remains in
  `cli/launcher/src/launch/outbound/resolve/cache_root.rs:80-94`, invoked by
  `LazyProductionResolver` in `cli/launcher/src/main.rs:65` for **every** external
  subcommand dispatch. Built-ins (`version`, `config`) deliberately never reach it
  ("built-ins never touch the cache root", `main.rs:50-53`). Phase 4 measures
  `bin/accelerator version` — the one command that skips the launcher-side probe
  — so the recorded after-median describes a path no hook uses, while
  `accelerator vcs guard` (0169's gate) pays the macOS first-exec penalty again
  inside the launcher. Phase 3's docs edit ("A warm start neither writes nor
  probes; it runs the already-staged shim and launcher") and the changelog entry
  ("Warm `accelerator` invocations are substantially faster") are therefore
  accurate only for the bash bootstrap layer and only for built-in commands.
  **Impact**: The recorded figure will overstate the improvement for the hook
  path on darwin, and 0169's `≤ 1.1 × 35.1 ms` criterion is likely still
  unreachable for a reason this plan's measurement cannot reveal.
  **Suggestion**: Scope the docs and changelog wording explicitly to the
  bootstrap ("the bootstrap no longer probes on a warm start; resolving an
  external subcommand still probes the cache directory once"), and record a
  second median for a real external-subcommand dispatch alongside the `version`
  figure in Validation Results, naming `cache_root::resolve` as the next residual
  so the 0169 hand-off note carries the whole picture.

- 🔵 **minor** (confidence: high) — *The `chmod 0o666` rationale misstates POSIX
  directory semantics, and the route it claims was measured only under BSD
  userland*
  **Location**: Phase 2, Section 1 — the paragraph justifying `chmod 0o666`
  The plan justifies `chmod 0o666` over `chmod -x` with "`0o666` leaves the
  directory writable, so `ensure_dir`'s `mkdir -p` and any preceding write still
  succeed and the probe is genuinely the thing that fails". Clearing a
  directory's execute (search) bit blocks **all** name resolution inside it
  regardless of the write bit, so `printf … > "$dir/.accelerator-probe-$$"` fails
  with `EACCES` on both platforms: the probe fails at its *write* step, never at
  its exec step. Worse, whether `ensure_dir` succeeds at all is
  userland-dependent — the "measured on this host" claim that `mkdir -p` tolerates
  an existing search-bit-less directory comes from BSD `mkdir` on darwin; GNU
  coreutils' `mkdir -p` uses a `chdir`-based ancestor walk and may fail instead,
  in which case the case is discharged by `resolve_cache_dir` returning 1 at
  `bin/accelerator:195` rather than by either probe gate. The substring assertion
  passes either way, so the cases stay green while proving different things on the
  two lanes.
  **Impact**: A future reader takes the recorded rationale at face value and
  believes the exec half is exercised, or believes both lanes exercise the same
  gate; the exec-vs-write gap is then wider than the recorded limitation admits.
  **Suggestion**: Correct the rationale to say the search bit is what fails
  (write, not exec), and pin the route rather than inferring it — e.g. assert
  `probe_exec_capable` appears in the trace for these two cases, or record
  explicitly in Validation Results that the discharging code path may differ
  between BSD and GNU `mkdir`.

- 🔵 **minor** (confidence: high) — *Only the uid-0 half of the mandated
  environment check is implemented; permission-advisory filesystems are left
  undiagnosed*
  **Location**: Phase 1, Section 3
  The work item's Acceptance Criteria preamble requires two environment checks —
  `id -u` **and**, "for a filesystem that ignores the permission bits, a temp-dir
  check (`chmod 0o555` then assert file creation inside fails; `chmod -x` then
  assert traversal fails)". `_require_unprivileged()` implements only
  `os.getuid() != 0`. On filesystems where `chmod` is advisory or ignored — WSL
  `drvfs` mounts without `metadata`, Docker Desktop bind mounts
  (virtiofs/gRPC-FUSE), VirtualBox `vboxsf`/`9p` shares, and macOS directories
  carrying an inherited ACL that grants write despite the mode —
  `cache.chmod(0o666)` silently succeeds and the bootstrap still works, so
  `test_cold_path_keeps_the_noexec_diagnostic` and
  `test_warmed_then_non_executable_cache_keeps_the_diagnostic` fail on
  `assert result.returncode != 0` with no hint that the environment, not the
  product, is at fault.
  **Impact**: Contributors whose `TMPDIR` lands on such a filesystem see two hard
  failures that look like a bootstrap regression, and the plan offers no
  diagnosis path.
  **Suggestion**: Add the mandated temp-dir capability check next to
  `_require_unprivileged` (create a dir, `chmod 0o555`, assert a write inside
  raises `PermissionError`) and call it from the three permission-dependent
  cases, failing with a message that names the filesystem as the cause and points
  at the recorded-exclusion rule.

- 🔵 **minor** (confidence: high) — *The measurement loop puts a `python3`
  interpreter start inside the timed interval, making both medians host-specific
  and the ratio gate harder to clear*
  **Location**: Phase 4, Section 1
  The loop reads the clock with
  `start=$(python3 -c 'import time; print(time.time_ns())')` before and
  `end=$(…)` after each call, so each measured interval contains one full
  `python3` process start (fork/exec plus interpreter init) in addition to the
  `bin/accelerator` call. That constant is highly host- and
  environment-specific: ~20-40 ms for a normal CPython, more when `python3`
  resolves through a `mise` shim or macOS's Xcode CLT stub. Because it is added
  to *both* medians, it inflates the ratio (`(41+c)/(149+c)`) — at c ≈ 60 ms the
  gate `after ≤ 0.5 × before` is marginal for a fully correct implementation.
  There is also no exit-status check per iteration (`set -uo pipefail`, no `-e`,
  output discarded), so a warm call that fails fast is timed as a fast call.
  **Impact**: The recorded figures are not comparable with the 2026-07-30 Context
  table or reproducible on another host, and the gate can fail a correct change
  or pass a broken one.
  **Suggestion**: Drive the whole loop from a single `python3` process
  (`subprocess.run` timed with `time.perf_counter_ns`, asserting
  `returncode == 0`), or keep bash and time with `perl -MTime::HiRes` once per
  run — noting in the recorded method *why* bash alone cannot do it (bash 3.2 has
  no `EPOCHREALTIME` and BSD `date` has no `%N`), so the next person does not
  reinvent the same biased loop.

- 🔵 **suggestion** (confidence: medium) — *Root containers lose the whole
  entrypoint suite with no documented deselect path*
  **Location**: Phase 1, Sections 3 and 4
  `_require_unprivileged()` hard-fails under uid 0 (deliberately, and correctly
  for the reason given), and Phase 1 retrofits it onto three cases that pass
  today. Running the integration suite as root is a normal local configuration —
  a plain `docker run` on the repo, many devcontainer base images, and any GitHub
  Actions job that later gains a `container:` key. In those environments six
  cases become hard failures, and the plan provides no supported way to run the
  remainder: the assertion is unconditional, so a root user cannot deselect it
  without editing the file.
  **Impact**: A contributor working in a root container cannot get a meaningful
  signal from `mise run test:integration:entrypoint` at all, which invites
  exactly the silent local `-k` workaround the hard-fail is meant to prevent.
  **Suggestion**: Put the guard behind a pytest marker (e.g.
  `@pytest.mark.requires_unprivileged`, registered in `pyproject.toml`) as well
  as the assert, so the exclusion is `-m 'not requires_unprivileged'` — explicit,
  greppable, and recordable in Validation Results per the work item's exclusion
  rule — while an unmarked default run still hard-fails under uid 0.

- 🔵 **suggestion** (confidence: high) — *Pin the extensionless entry point's
  formatter settings in `.editorconfig` instead of relying on shfmt defaults*
  **Location**: Key Discoveries — "`bin/accelerator` is tab-indented and must
  stay so"
  The plan correctly identifies that `.editorconfig`'s `[*.sh]` section does not
  match the extensionless `bin/accelerator`, so shfmt falls back to its own
  defaults (tabs, `switch_case_indent` off) for the single most-executed shell
  file in the repo — which is why its `case` arms sit at column 0 while every
  `.sh` file's are indented. The plan handles this by hand for its own snippets,
  but the underlying config gap stays: the file's format is an accident of
  shfmt's defaults rather than a declared setting, and a future shfmt default
  change or a well-meaning `[*.sh]` edit would reformat the trust-root entry
  point wholesale.
  **Impact**: The next contributor touching `bin/accelerator` re-discovers the
  trap the same way, or a toolchain bump produces a large unrelated diff in the
  file every hook executes.
  **Suggestion**: Add an explicit `[bin/accelerator]` section to `.editorconfig`
  declaring the current shape (`indent_style = tab`,
  `switch_case_indent = false`) with a one-line comment that shfmt's
  `.editorconfig` matching is extension-based and this file has none — making
  today's formatting intentional and stable without reformatting anything.

### Standards

**Summary**: The plan is unusually disciplined about project conventions: every
`mise run` task name it quotes exists, every line reference I spot-checked into
`bin/accelerator`, `installation.py`, the entrypoint suite, the hooks suite and
`main.yml` is accurate against the current tree, and the proposed Python and bash
both match their host files' established idioms (leading-underscore helpers,
harness-first helper signature, the `kwargs.pop("extra_env")` + `# type: ignore`
pattern, tab indentation reasoned correctly from `.editorconfig`'s `[*.sh]` not
matching an extensionless file). Three convention defects are load-bearing: the
`docs/internals.md` replacement text silently drops the release-base-URL trust
guidance that shares the quoted line range; Phase 1 introduces regex helpers
while deferring `import re` to Phase 2, so Phase 1 alone fails `ruff` and is not
independently mergeable as claimed; and `mise run check` is credited with
markdown lint and work-item frontmatter validation that it does not perform (no
markdown linter exists anywhere in the task tree), leaving Phase 3 with no real
automated gate. Smaller items: `no_cache_dir` breaks the file's verb-led
function-naming convention, the plan contradicts itself on whether the diagnostic
string gets a single definition, and the plan document itself carries 16
over-80-column lines where the work item it derives from has zero.

**Strengths**:

- Every `mise run` task name quoted in the Success Criteria exists in
  `mise.toml`: `test:integration:entrypoint`, `test:integration:skill-invocation`,
  `build-system:check`, `scripts:check` and `check` are all real leaves, and the
  plan correctly declines to add a new `test:integration:*` leaf (which would also
  have needed the no-`build:cli:dev` invariant re-checked).
- Line references are accurate against the current post-0182 tree —
  `bin/accelerator:166-180` (probe_dir), `:184-193` (resolve_cache_dir),
  `:195-197` (diagnostic), `:252-261` (shim staging), `:305-307`, `:336-348`,
  `:352`; `installation.py:149-157` (build_launcher) and `:324-369`
  (run_bootstrap); the entrypoint suite's `:253`, `:279`, `:584-644`,
  `:772`/`:787`/`:801`, `:1060`; `hooks/test_launcher_link_refresh.py:275-293`
  (the skipif it deliberately diverges from); and `main.yml:55-91` (the
  `test-integration` matrix, correctly noted as having no `container:` key).
- The proposed Python follows the entrypoint suite's conventions closely:
  leading-underscore module-level helpers, a harness-first
  `_traced(harness, downloader, **kwargs)` signature mirroring the existing
  `_run_and_capture_env`, the identical
  `dict(kwargs.pop("extra_env", None) or {})  # type: ignore[arg-type]` idiom,
  `os.getuid()` matching the neighbouring guard's expression,
  `finally`-restored modes matching the existing `chmod` tests, and snippets that
  are ruff-format-stable within 80 columns.
- The new section divider drops the stale `Phase 0:` prefix the four existing
  dividers carry, matching the phase-free `# ── Self-location ──` form —
  consistent with the project's stance against embedding planning references in
  code.
- The proposed bash matches `bin/accelerator`'s own idiom: tab indentation
  (correctly derived from `.editorconfig`'s `[*.sh]` section not matching an
  extensionless file, so shfmt falls back to tab defaults), bare globals with no
  `local`, a `probed=""` / `probed=1` boolean shape mirroring `lock_held`, a
  backslash-continued `fail` message mirroring `:196-197` and `:224-225`, and
  mid-script function definitions as the file already does for `sha256_file`,
  `acquire_lock` and `verify_launcher`.
- The CHANGELOG target exists exactly as claimed — `## [Unreleased]` with an open
  `### Changed` — and the proposed bullet's bold lead-in sentence followed by
  explanatory prose matches the shape of every neighbouring entry.
- Deliberate convention deviations are named and justified rather than taken
  silently: the `PS4='+${FUNCNAME[0]:-main}:'` fix against the criterion's literal
  wording, the real-launcher route against the fixture-version assertion, and the
  hard-fail guard against the neighbouring `skipif` — each with a recording
  obligation in Phase 4.

**Findings**:

- 🟡 **major** (confidence: high) — *The `docs/internals.md` replacement drops the
  release-base-URL guidance that shares its line range*
  **Location**: Phase 3, Change 1: Internals documentation
  The plan instructs the implementer to replace `docs/internals.md:207-209` with
  a three-sentence block, but the current text at 207-212 is *two* sentences
  sharing those lines: the probe sentence ends mid-line-209 ("…not
  group-writable.") and the release-base-URL trust guidance ("The release base URL
  should be a host you trust not to serve an older signed release: the cache key
  carries no content hash, so a mirror can hand back an older validly-signed
  launcher for the current version.") begins on the same line and runs to 212. The
  proposed replacement omits that second sentence entirely, and the quoted prose
  is also not wrapped to the repo-wide 80-column width set in `.editorconfig`
  `[*]` (its first line is 81 characters as written).
  **Impact**: Applied literally, the edit either deletes the mirror-downgrade
  warning for the other trust-root environment variable documented in the same
  paragraph, or leaves a mangled paragraph — and there is no markdown linter in
  the task tree to catch either.
  **Suggestion**: Quote the replacement as the full 207-212 paragraph including
  the unchanged release-base-URL sentence, pre-wrapped at 80 columns, and state
  the range as `:207-212` so the boundaries land on line ends.

- 🟡 **major** (confidence: high) — *Phase 1 adds regex helpers but defers
  `import re` to Phase 2, so Phase 1 fails ruff*
  **Location**: Phase 1, Change 2 (vs Phase 2, Change 1)
  Phase 1 adds `_entered` and `_ran_probe_file` to
  `tests/integration/entrypoint/test_accelerator_entrypoint.py`, both of which
  call `re.search`, but the instruction "Add `import re` to the module imports"
  appears only at the end of Phase 2's regression-case section. The module
  currently imports `concurrent.futures`, `hashlib`, `os`, `platform`, `shutil`,
  `subprocess` — not `re`. `lint:build-system:check` runs `uv run ruff check` from
  the repo root over the whole tree, and the `"tests/**"` per-file-ignores in
  `pyproject.toml` drop only `S`, `ANN`, `D`, `PLR2004`, `SLF001`, `PT`, `INP001`
  — `F` stays enabled, so `F821 undefined-name: re` fires.
  **Impact**: Phase 1 as specified fails its own `mise run build-system:check`
  and `mise run check` criteria, breaking the plan's stated invariant that each
  phase is independently green and mergeable.
  **Suggestion**: Move the `import re` instruction into Phase 1 alongside the
  helpers that need it, and drop it from Phase 2.

- 🟡 **major** (confidence: high) — *`mise run check` performs neither markdown
  lint nor frontmatter validation*
  **Location**: Phase 3 Success Criteria; Phase 4 Success Criteria
  Phase 3's sole automated criterion is "Markdown format and lint pass:
  `mise run check`" and Phase 4's is "Work item frontmatter validates:
  `mise run check`". Neither is true: `check` folds `frontend:check`,
  `server:check`, `cli:check`, `deny:check`, `pup:check`, `build-system:check` and
  `scripts:check` only — there is no markdown formatter or linter anywhere in
  `mise.toml` or `tasks/` (the `.md` references in `tasks/` are all SKILL.md
  content guards), and meta-corpus frontmatter validation lives in
  `scripts/test-validate-corpus-frontmatter.sh`, run from the
  `test:integration:config` shell suites, not from `check`.
  **Impact**: Phase 3 (a docs + changelog phase) has no real automated
  verification while appearing to have one, and Phase 4 claims a frontmatter gate
  that will never run — so a malformed `## [Unreleased]` bullet or work-item
  frontmatter would ship green.
  **Suggestion**: Drop the false attributions — make Phase 3's verification purely
  manual (or add `mise run test:integration:config` if the corpus validator is
  genuinely wanted), and for Phase 4 name the suite that actually validates meta
  frontmatter rather than `check`.

- 🔵 **minor** (confidence: high) — *`no_cache_dir` breaks the file's verb-led
  function-naming convention*
  **Location**: Phase 2, Change 4
  The new shell helper is named `no_cache_dir`, a noun phrase, in a file whose
  functions are uniformly verb-led — `fail`, `fail_integrity`, `dir_of`,
  `probe_dir`, `resolve_cache_dir`, `sha256_file`, `acquire_lock`, `release_lock`,
  `verify_launcher`, `fetch_and_verify`, `dev_launcher_contained` — and whose two
  existing abort helpers in particular are `fail` and `fail_integrity`. Read at
  the call site (`probe_exec_capable "${cache_dir}" || no_cache_dir`) it looks
  like a predicate rather than the fatal exit it is.
  **Impact**: A reader scanning `bin/accelerator` — the plugin's trust root, and a
  file every SessionStart hook executes — cannot tell from the name that control
  never returns.
  **Suggestion**: Name it `fail_no_cache_dir` (or `fail_cache_dir`), matching the
  existing `fail_*` abort-helper family.

- 🔵 **minor** (confidence: high) — *The same paragraph gives contradictory
  instructions about the diagnostic string*
  **Location**: Phase 2, Change 4
  It first says "The existing `fail` at `:195-197` is rewritten to call the same
  wording via `no_cache_dir` so the substring has one definition", then
  immediately retracts it: "`no_cache_dir` … interpolates `${cache_dir}` — which
  is unset at that point — so the `:195` call site keeps its own literal
  `${plugin_root}/bin` message". The end state is two independent literal copies
  of `no writable, exec-capable cache directory`, which four of the plan's tests
  assert as a substring.
  **Impact**: An implementer working top-down performs the first edit before
  reading the retraction, and the plan's stated "one definition" property is not
  what the design actually delivers — a later edit to one copy silently diverges
  from the other with tests still green.
  **Suggestion**: Delete the retracted sentence so only the final shape is
  stated, and say explicitly that the substring exists twice by construction (with
  the `${cache_dir}`-unset reason) so nobody "fixes" the duplication later.

- 🔵 **minor** (confidence: medium) — *The retrofit extends the hard-fail rule
  beyond the work item's scope, past the repo's own `skipif` idiom*
  **Location**: Phase 1, Change 4
  Phase 1 retrofits the hard-failing `_require_unprivileged()` onto three
  *pre-existing* tests (`test_readonly_root_with_override_runs_from_override`,
  `test_readonly_root_without_override_is_a_named_error`,
  `test_a_record_is_always_one_line`). The work item's hard-fail rule is scoped to
  "three automated criteria [that] are permission-dependent" among the new cases;
  the repo's established idiom for the pre-existing set is
  `@pytest.mark.skipif(os.getuid() == 0, reason="mode bits are advisory for uid 0")`
  (`tests/integration/hooks/test_launcher_link_refresh.py:275-277`), and two of
  the three retrofitted tests pass today under uid 0.
  **Impact**: `mise run test:integration:entrypoint` becomes a hard failure in any
  root context (a root Docker dev container, for example) where it currently runs
  green, and the file ends up carrying two competing privilege idioms without the
  work item having mandated the change for the existing cases.
  **Suggestion**: Either scope the hard-fail guard to the new cases the work
  item's rule governs and use the neighbouring `skipif` for the retrofit, or state
  explicitly in the plan that the retrofit deliberately extends the rule beyond
  the work item's scope and record the consequence for root-privileged lanes.

- 🔵 **minor** (confidence: medium) — *Test comments cross-reference sibling tests
  and restate acceptance-criteria rationale*
  **Location**: Phase 2, Change 1
  Three of the six proposed test bodies open with comments that cross-reference
  other tests by name — "`test_warm_path_does_not_enter_the_probe` covers that",
  "Routing is pinned by `test_cold_path_enters_and_executes_the_probe`", "see
  criterion 5" — and largely restate acceptance-criteria rationale already
  recorded in the work item and the plan's own Testing Strategy. The project's
  plan-authoring standard has a very low tolerance for comments and explicitly
  bans references that "can go stale fast"; a hard-coded sibling test name is
  exactly that.
  **Impact**: Renaming or splitting one case leaves dangling references in two
  others, and the duplicated rationale drifts from the work item that owns it.
  **Suggestion**: Keep only the comments that signal something genuinely
  non-obvious to a reader of the code (the bash 3.2 / `set -u` PS4 note
  qualifies), and drop the test-name cross-references and criterion numbers — the
  Testing Strategy section already carries the coverage argument.

- 🔵 **minor** (confidence: medium) — *The changelog entry is mechanism-heavy and
  cannot pass its own verification step*
  **Location**: Phase 3, Change 2
  The drafted `## [Unreleased]` / `### Changed` entry is mostly mechanism — it
  names the probe file, `chmod`, the macOS first-exec check, the cold/warm split
  and the staged verifier — while the same phase's own manual criterion reads "The
  changelog entry describes user-visible behaviour, not the mechanism".
  Neighbouring entries ("Interactive option panels replace typed confirmations
  across 15 skills", "Migration framework — more robust and resilient") lead with
  the observable effect and keep internals to a minimum.
  **Impact**: As drafted the entry cannot pass its own verification step, and it
  exposes bootstrap internals in the user-facing changelog where the surrounding
  entries do not.
  **Suggestion**: Reduce the entry to the observable change (warm invocations, and
  therefore every SessionStart hook, are substantially faster; a `noexec` cache
  directory still fails with the same named error) and move the probe/first-exec
  attribution to the work item's Validation Results, which already records it.

- 🔵 **minor** (confidence: high) — *The plan document carries 16 lines over 80
  characters*
  **Location**: Plan document (frontmatter through References)
  `.editorconfig` sets `max_line_length = 80` for `[*]`, and the work item this
  plan derives from has **zero** lines over 80 — including its own long
  `meta/work/...` and `meta/research/...` path references, which it wraps. The
  over-long lines here are the Current State Analysis table rows, several Success
  Criteria bullets such as "`mise run test:integration:skill-invocation`", the
  `docs/internals.md` quote, and most of the References list.
  **Impact**: The repo-wide 80-column convention is hand-maintained with no
  automated check for markdown, so drift in the highest-traffic planning artefacts
  is how it erodes; it also makes the plan harder to diff and review side-by-side.
  **Suggestion**: Rewrap the over-long lines the way the work item does — split
  table cells, put long task names on their own continuation line under the
  criterion, and wrap reference paths onto the following indented line.

- 🔵 **suggestion** (confidence: medium) — *The bash-3.2 gate is invoked raw
  rather than through its mise leaf, and is redundant with the next criterion*
  **Location**: Phase 2 Success Criteria
  The criterion "Bash 3.2 gate passes: `scripts/lint-bashisms.sh`" invokes the raw
  script when a mise leaf exists for it (`lint:scripts:bashisms:check`), and the
  very next criterion (`mise run scripts:check`) already folds it via
  `lint:scripts:check`. `CLAUDE.md` states all dev tasks run through
  `mise run <task>`, and the one sanctioned exception is dropping to the
  underlying runner to filter a single *test*. Note also that a bare
  `scripts/lint-bashisms.sh` discovers its file list via `git ls-files '*.sh'`,
  which is blind inside a jj workspace checkout (it still scans `bin/accelerator`,
  which is appended unconditionally, so the criterion happens to work — but for
  the wrong reason).
  **Impact**: A criterion that bypasses the task tree teaches the wrong entry
  point and hides the fact that the check is already covered by the following
  line.
  **Suggestion**: Replace it with `mise run lint:scripts:bashisms:check` if a
  standalone bash-3.2 gate is wanted, or drop it as redundant with
  `mise run scripts:check`.

### Documentation

**Summary**: The plan's documentation work is unusually well-targeted for its
size: I grepped the whole shipped tree (`docs/`, `README.md`, `skills/`,
`hooks/`, `scripts/`, ADRs, `CHANGELOG.md`) and `docs/internals.md:207-209`
really is the only prose site that this change falsifies, so Phase 3 has not
missed a second paragraph. The weaknesses are elsewhere: the durable record of
this change's two hardest-won rationales (why the second `probe_once` call site
exists, why the launcher-path hoist is load-bearing) lives only in the plan and
the work item, not in `bin/accelerator` — a file whose own convention is a dense
why-comment on every non-obvious construct; the `docs/internals.md` edit
instruction quotes a replacement block that stops mid-paragraph and would orphan
the trust-relevant release-base-URL sentence; and Phase 2 item 4 contains a
paragraph that states one edit and then retracts it, which an implementer reading
top-down will get wrong. The changelog entry is also inverted against its own
success criterion — mostly mechanism, no quantification, and silent about the
newly supported read-only-cache configuration.

**Strengths**:

- Phase 3's single documentation target is correct and complete: an exhaustive
  grep for `probe`, `ACCELERATOR_CACHE_DIR`, `noexec` and `exec-capable` across
  `docs/`, `README.md`, `skills/`, `hooks/`, `scripts/` and `meta/decisions/`
  finds no other shipped statement about the probe or per-invocation bootstrap
  behaviour, so the plan has not under-scoped the prose work.
- `bin/accelerator`'s file header (lines 3-20) documents the fetch-verify-cache
  contract and the test seams but never mentions the probe, so the plan is right
  that no header rewrite is forced by this change.
- The new `probe_exec_capable` comment is why-focused and states both the hazard
  it catches (a noexec mount a write-only check would pass) and why the warm path
  needs no equivalent (the staged shim and launcher prove the same capability for
  real).
- `_require_unprivileged`'s docstring records why it hard-fails instead of
  skipping, and Phase 1 adds a manual criterion that a later reader does not "fix"
  it back to the neighbouring `skipif` idiom — exactly the kind of decision that
  otherwise gets silently reverted.
- The bash-3.2 `PS4`/`set -u` discovery is captured as a comment at its point of
  use above the `_PS4` constant, not only in the plan.
- Each new test carries an inline comment stating what it does *and does not*
  prove (e.g. the warm `chmod 0o555` case cannot catch a non-fatal surviving
  probe; the warmed-then-non-executable case is an end-state guard, not proof of
  routing) — this is the project's existing convention in this module and it is
  followed.
- Migration Notes correctly and briefly states that no on-disk format, cache
  layout or environment contract changes, which is the right answer for the
  changelog audience.
- The new test section banner (`# ── Exec probe: cold-path only ──`) matches the
  module's existing banner convention, keeping the suite navigable.

**Findings**:

- 🟡 **major** (confidence: high) — *Phase 2 item 4 states an edit and then
  retracts it in the same paragraph*
  **Location**: Phase 2, Change 4
  The paragraph after the code block first says "The existing `fail` at
  `:195-197` is rewritten to call the same wording via `no_cache_dir` so the
  substring has one definition", then in the next sentence says the opposite: "so
  the `:195` call site keeps its own literal `${plugin_root}/bin` message and only
  the two probe gates route through `no_cache_dir`". An implementer reading the
  phase top-down performs the first edit before reaching the retraction, and the
  first sentence's stated goal (one definition of the
  `no writable, exec-capable cache directory` substring that six tests assert on)
  is not what the plan actually lands.
  **Impact**: The most delicate instruction in the production phase is
  self-contradictory, so the diagnostic wording — asserted as a literal substring
  by two of the six new tests and one existing test — can be edited into a state
  the plan did not intend, and the reviewer of the resulting diff cannot tell
  which shape was chosen deliberately.
  **Suggestion**: Delete the retracted first sentence and state the outcome once:
  two `fail` sites, the `:195` one keeping its literal `${plugin_root}/bin`
  message because `cache_dir` is not yet bound, and `no_cache_dir` covering both
  probe gates — then add the reason the duplication is accepted, and pair the two
  sites with a short cross-referencing comment in the script so the shared
  substring is not drifted apart by a later edit.

- 🟡 **major** (confidence: high) — *The rationales for Gate B and the hoisted
  launcher paths never reach the script*
  **Location**: Phase 2, Change 4 / Key Discoveries
  The plan's Key Discoveries record two facts that a future editor of
  `bin/accelerator` must know — that the second `probe_once` call site in the cold
  branch exists to stop a ~30 second `acquire_lock` spin in the
  verification-failed/unwritable-directory case, and that the
  `launcher`/`launcher_sig` assignments were hoisted above shim staging *because
  Gate A depends on them* — but both are destined only for the plan and the work
  item's Validation Results ("so the reasoning survives for anyone tempted to
  remove it"). The proposed code has no comment on Gate B, none on the `probed`
  flag explaining why idempotence matters (both gates can fire in one cold
  invocation, which would otherwise pay the ~108 ms probe twice), and none marking
  the hoisted assignments' position as load-bearing.
  **Impact**: `bin/accelerator` is a 350-line file where every non-obvious
  construct already carries a why-comment (the `dir_of` `${dir:-/}` arm, the
  16-hop bound, `cd -P` vs `cd`, the content-addressed shim staging); a maintainer
  will read the script and not the plan, so `probe_once` appearing twice with no
  explanation is the most likely thing in this change to be "tidied" back into a
  single call site, silently reintroducing the 30-second hang.
  **Suggestion**: Add three short comments to the Phase 2 snippets: one above Gate
  B naming the residual case and the lock-spin it prevents, one on `probed`
  stating that the two gates are mutually reachable in a single cold run, and one
  line on the hoisted assignments noting Gate A reads them (and that `base_url`
  deliberately stays behind). Consider also one clause in the file header contract
  summary recording the cold/warm capability-check asymmetry, since the header is
  the first thing a reader of the changed file sees.

- 🟡 **major** (confidence: medium) — *The quoted `internals.md` replacement would
  orphan the release-base-URL sentence*
  **Location**: Phase 3, Change 1
  Phase 3 says "At `:207-209`, qualify the probe as a cold-path behaviour" and
  then quotes a three-sentence replacement block that ends at "…runs the
  already-staged shim and launcher from that directory instead." But
  `docs/internals.md:209` reads
  `at a directory you own and that is not group-writable. The release base URL` —
  the line straddles a sentence boundary, and the quoted block does not carry the
  release-base-URL sentence forward. A literal 207-209 replacement leaves the
  surviving text starting
  `should be a host you trust not to serve an older signed release: …`, with no
  subject.
  **Impact**: The dropped sentence is the trust-root warning about mirrors serving
  an older validly-signed launcher — the more security-relevant half of the
  paragraph — and the resulting fragment would ship as broken prose in the
  user-facing internals guide.
  **Suggestion**: Quote the whole paragraph as it should read after the edit,
  including the unchanged release-base-URL sentence, rather than a partial block
  against a line range that ends mid-sentence.

- 🟡 **major** (confidence: medium) — *The newly supported read-only warm cache is
  left undocumented in the section that exists for it*
  **Location**: Phase 3: Documentation
  After this change a fully read-only cache directory with a pre-populated
  launcher works on the warm path — `mkdir -p` succeeds on an existing directory,
  Gate A does not fire, and no write occurs; the plan's own
  `test_warm_path_survives_a_non_writable_cache_dir` (`chmod 0o555`) pins exactly
  that, and today the same invocation fails with
  `no writable, exec-capable cache directory`. The section Phase 3 edits is titled
  "Offline, mirrored and read-only installs", yet the proposed replacement only
  says a warm start "neither writes nor probes" and never states the
  operator-visible consequence, and the changelog entry does not mention it
  either.
  **Impact**: The one audience most affected by this change — operators running
  mirrored or locked-down installs, who are already reading that exact section —
  cannot learn from the docs that a configuration which previously aborted is now
  supported, and will keep provisioning a writable cache directory they no longer
  need for steady-state use.
  **Suggestion**: Add one sentence to the edited paragraph stating that a cache
  directory populated once may afterwards be read-only for warm invocations (cold
  starts — a new plugin version, or a failed verification — still need it writable
  and exec-capable), and reflect the same fact in the changelog entry.

- 🟡 **minor** (confidence: high) — *The retained `:195` diagnostic now names
  conditions its call site no longer tests*
  **Location**: Phase 2, Change 3
  The plan keeps the `:195-197` message
  `no writable, exec-capable cache directory: … is not usable and no ACCELERATOR_CACHE_DIR override was given`
  unchanged, while reducing that call site to `ensure_dir`, i.e. a bare
  `mkdir -p`. After the change this failure can only mean "the directory could not
  be created"; writability of an existing directory and exec capability are tested
  later, at the probe gates, which emit the same words.
  **Impact**: A user hitting the `:195` failure is told the directory is not
  writable or not exec-capable when neither was checked, and a maintainer
  diagnosing the message cannot tell which of two call sites produced it since the
  wording is now identical at both.
  **Suggestion**: Keep the asserted substring (the tests depend on it) but
  distinguish the tails — e.g. append "could not be created" at the `:195` site —
  or, if the wording must stay byte-identical, add a comment at both sites
  recording that the substring is deliberately shared and which condition each
  site actually detects.

- 🔵 **minor** (confidence: high) — *Changelog entry is mechanism-heavy,
  unquantified, and contradicts its own success criterion*
  **Location**: Phase 3, Change 2
  The drafted entry opens with one user-facing sentence and then spends three
  clauses on mechanism (probe file, `chmod`, macOS first-exec check, `noexec`
  cache directory, staged verifier), while Phase 3's own manual criterion says
  "The changelog entry describes user-visible behaviour, not the mechanism". It
  also omits the two things a reader of `CHANGELOG.md` can act on: the size of the
  win (~108 ms of a ~149 ms warm call on darwin, per the work item's measured
  table) and where they will notice it (every SessionStart hook and every skill
  `!`-site that shells out to the CLI).
  **Impact**: The entry reads as an internal note rather than a release note, so
  users cannot judge whether the change matters to them, and the phase ships with
  a manual criterion its own artefact fails.
  **Suggestion**: Lead with the observable effect and a figure ("warm invocations
  drop from roughly 150 ms to roughly 40 ms on macOS, so session start and every
  skill's live-context command are noticeably faster"), keep one sentence of
  mechanism, and drop the rest. `### Changed` is a defensible home given the
  section already exists, though earlier releases in this file use `### Improved`
  for exactly this kind of entry.

- 🔵 **minor** (confidence: medium) — *The documentation phases' automated checks
  verify nothing about the files they edit*
  **Location**: Phase 3 and Phase 4: Success Criteria / Automated Verification
  Phase 3's only automated criterion is "Markdown format and lint pass:
  `mise run check`", and Phase 4 claims "Work item frontmatter validates:
  `mise run check`". `mise run check` composes `format:check` and `lint:check`
  over exactly the frontend, server, cli, build-system and scripts components,
  with no markdown formatter or linter and no corpus-frontmatter validation; the
  markdown 80-column convention is enforced by hand.
  **Impact**: Both documentation phases appear gated when they are not, so a
  mis-wrapped paragraph, a broken cross-reference, or a truncated
  `docs/internals.md` paragraph (see the related Phase 3 finding) would pass every
  stated check.
  **Suggestion**: Replace those criteria with honest manual ones — e.g. re-read
  the edited `docs/internals.md` section end-to-end for a complete paragraph and
  80-column wrapping, and confirm the changelog entry sits under the existing
  `## [Unreleased]` / `### Changed` heading — or name the specific task that
  actually validates the corpus if one exists.

- 🔵 **minor** (confidence: medium) — *New shared-harness seam added without
  recording its contract or the rejected alternative*
  **Location**: Phase 1, Changes 1-2
  `run_bootstrap` in `tests/integration/support/installation.py` carries a prose
  docstring that exists specifically to record non-obvious behaviour for its
  consumers, and Phase 1 adds a `bash_args` keyword to it without touching that
  docstring. Three constraints stay in the plan only: that the empty default must
  remain because `tests/integration/skill-invocation/` shares the funnel; that
  xtrace must never be a global mode because it lands on stderr and would break
  every existing `assert … in result.stderr`; and that the plan measured a simpler
  route it then did not take (`SHELLOPTS=xtrace` through the existing `extra_env`,
  needing no harness change at all) with no rationale for preferring the new
  parameter.
  **Impact**: The next author to add a trace-based case has no in-repo signal that
  per-call tracing is deliberate rather than incidental, and the funnel gains an
  undocumented parameter that a second suite silently depends on defaulting to
  empty.
  **Suggestion**: Add one clause to `run_bootstrap`'s docstring naming
  `bash_args` and the shared-consumer constraint, give `_traced` a one-line
  docstring stating why tracing is per-call and never global, and record in the
  plan why `bash_args` was chosen over the discovered `SHELLOPTS` route.

- 🔵 **minor** (confidence: medium) — *Six of the eleven pending Validation
  Results entries get no stated evidence shape*
  **Location**: Phase 4, Change 2
  Phase 4 says "Replace each _pending_ entry" and then enumerates only the
  measurement, the lanes, the exec-vs-write limitation, the PS4 deviation, the
  criterion-1 amendment and the second call site. The work item's other pending
  entries — warm-path exec-probe-free check, direct probe-absence check, positive
  control, `noexec` cold-path check, cold happy-path (`ensure_dir`) check, and
  diagnostic-preserved-on-warmed-then-non-executable — get no guidance on what to
  record, and nothing anywhere states the criterion-to-test-name mapping (six
  criteria, six new test functions with different names). The plan also splits the
  work item's criterion 3, which requires the positive control to ride on the
  *cold happy-path run* of criterion 6, into two separate tests (a traced
  default-launcher run and an untraced `real_launcher` run) without recording that
  as a deviation, although smaller deviations get their own bullet.
  **Impact**: Validation Results is the durable record a later reader consults to
  know which permanent guard discharges which criterion; without the mapping the
  closeout can be ticked with prose that no longer traces to a test, and the
  criterion-3 split is an undocumented reinterpretation of a load-bearing
  criterion.
  **Suggestion**: List the six per-check entries with the test function name that
  discharges each, and add a bullet recording that the positive control was
  implemented as its own cold run rather than sharing criterion 6's run, with the
  reason (the real-launcher run is not traced).

- 🔵 **minor** (confidence: medium) — *The staging comment's "(cheap)" becomes
  misleading once the probe is gone*
  **Location**: What We're NOT Doing / Performance Considerations
  The comment above shim staging (`bin/accelerator:246-251`) says the
  content-addressed scheme means "a warm call re-hashes (cheap) instead of
  re-copying 475KB". After this change that re-hash is the single largest
  remaining warm-path cost — the plan itself puts it at ~11.7 ms of a ~41 ms warm
  call, and 0169's latency criterion turns on it — yet the plan explicitly leaves
  the block and its comment untouched.
  **Impact**: A future reader hunting warm-path latency (0169 is already queued to
  do exactly that) is steered away from the dominant remaining cost by a
  parenthetical that was true only while the probe absorbed 108 ms, and the
  decision to keep the second hash for its planted-stub defence is recorded only
  in the work item.
  **Suggestion**: Amend the parenthetical to note the re-hash is now the dominant
  warm-path cost and retained deliberately for the planted-stub defence, with a
  pointer to the three tests that assert it — a one-line edit that belongs in
  Phase 3 alongside the other documentation corrections.

---

## Re-Review (Pass 2) — 2026-08-03

**Verdict:** REVISE

All eight lenses re-ran against the revised plan, briefed on which areas
changed but not on the previous findings or verdict. The pass-1 substance is
resolved: the redesigned two-gate placement was independently traced by both
correctness and test-coverage and found complete (no reachable state writes to
`cache_dir` or reaches `acquire_lock` without a probe), the anti-hang case was
verified to genuinely reach `verify_launcher` with staging skipped, the
anchored trace matcher was confirmed to discriminate the exec line from the
`probe=` assignment, and dropping the hoist removed an ordering invariant and a
duplicated predicate outright. The verdict stays REVISE because the revision
introduced a fresh crop of defects — five of them mechanical certainties I
verified by running the code — plus one substantive miss: the newly promised
read-only cache directory is contradicted by the launcher's own *writing*
probe on every external-subcommand dispatch.

### Previously Identified Issues

- 🔴 **Test Coverage / Portability**: Gate B has no automated coverage —
  **Resolved.** Both lenses independently traced
  `test_unverifiable_launcher_in_readonly_cache_fails_fast` and confirmed
  `0o555` keeps the search bit, the staged shim still hashes equal (staging
  skipped), `[[ -x launcher ]]` stays true, and the poisoned launcher fails at
  `verify_launcher` — so the case genuinely reaches the cold-branch gate.
- 🔴 **Performance**: interpreter startup inside the measured interval —
  **Resolved, but the replacement is broken.** Single-interpreter timing is
  right; the calibration line crashes (see New Issues).
- 🟡 **Correctness**: `_ran_probe_file` vacuous — **Resolved.** Verified
  empirically against a real trace: the old pattern matched 2 lines (including
  `probe=…`), the anchored one matches exactly the exec line.
- 🟡 **Code Quality / Architecture / Standards / Documentation / Correctness**:
  the `no_cache_dir` self-contradiction — **Resolved.** One parameterised
  definition above `:195`, three call sites, and the pre-existing
  hardcoded-`${plugin_root}/bin`-under-override bug fixed as a side effect.
- 🟡 **Architecture / Code Quality / Documentation**: `bash_args` widens the
  shared funnel — **Resolved.** Narrowed to `xtrace: bool`.
- 🟡 **Correctness / Architecture / Standards**: Phase 1 not independently
  green — **Resolved.** `import re` moved with its helpers; Phase 1 reduced to
  the privilege guard, which architecture confirmed is a genuine prerequisite
  for five of Phase 2's cases.
- 🟡 **Test Coverage**: "all six cases fail before the change" unachievable —
  **Partially resolved.** Reworded per-case, but case 7 is now misclassified
  (see New Issues).
- 🟡 **Test Coverage**: the `:195` diagnostic unreachable by any test —
  **Partially resolved.** A case was added; it is vacuous as written.
- 🟡 **Standards / Documentation**: `internals.md` orphans the
  release-base-URL sentence — **Resolved.** Verified as a faithful `:207-212`
  substitution with nothing dropped or duplicated.
- 🟡 **Standards / Documentation**: `mise run check` false gates — **Partially
  resolved.** The markdown and frontmatter attributions are now correct
  (`test:integration:config` verified to run the corpus validator by name), but
  Phase 3 now claims *no* automated verification while editing
  `bin/accelerator`.
- 🟡 **Documentation**: Gate/hoist rationale never reaches the script —
  **Partially resolved.** The cold-branch gate is commented; the staging gate —
  whose placement carries the load-bearing "first write into `cache_dir`"
  invariant — still has no comment. Flagged again by three lenses.
- 🟡 **Portability**: bash-5 trace evidence missing — **Partially resolved.** A
  manual check was added, but it has no mechanism and is largely redundant.
- 🟡 **Documentation**: read-only warm cache undocumented — **Resolved then
  over-promised.** Now documented, but incorrectly.
- 🟡 **Performance**: residual stated as ~11.7 ms — **Resolved in the plan,
  incompletely propagated.** Corrected to a measured ~8.4 ms, but 0186's own
  body keeps the old figures and the corrected number leaves the composition
  unexplained.
- 🟡 **Performance**: retained hashing framed as a trust trade-off —
  **Resolved.** Measured, `openssl` rejected on evidence, follow-up correctly
  not raised — though the conclusion is host-conditional and a different,
  untried lever surfaced.
- 🟡 **Performance**: gate can pass on a measurement that measured nothing —
  **Mostly resolved.** Per-sample and sample-count assertions added; they are
  `assert` statements, so `-O` silently removes them.
- 🟡 **Performance**: sequential batches alias drift — **Resolved.**
  Interleaved, and the `jj` swap removed entirely.
- 🟡 **Performance**: `ensure_dir` forks `mkdir` on warm calls — **Resolved.**
- 🟡 **Architecture / Portability**: the launcher re-incurs the probe —
  **Partially resolved.** Raised as a follow-up and the docs scoped, but
  overstated as decisive for 0169 and contradicted by the read-only promise.
- 🔵 Minor items from pass 1 largely resolved: the `chmod 0o666` rationale
  corrected to the write step, the Gate-A/staging gap closed by the redesign,
  the third predicate copy and hoist ordering invariant eliminated, `_traced`
  typed, naming joined the `fail_*` family, criterion-1 tightened to stdout
  equality, the probe-cleanup assertion added, `acquire_lock` masking recorded
  as a follow-up, the alternative-design rationale recorded, and the plan
  rewrapped to 80 columns.
- 🔵 **Still present from pass 1**: hard-coded production function names in the
  tests (the fast `test_bootstrap_coverage.py` guard is still only an optional
  manual bullet — re-flagged by three lenses); advisory-permission filesystems
  undiagnosed; `test_warm_path_does_not_enter_the_probe` still does not assert
  its *traced* run succeeded; `probe_exec_capable`'s write path still skips
  cleanup; the `(no XDG fallback)` clause still reads awkwardly and now drops
  the `ACCELERATOR_CACHE_DIR` remediation hint.

### New Issues Introduced

Verified mechanically (I ran these):

- 🟡 **Correctness / Performance**: **the measurement script crashes on its
  first statement.** `once()` asserts `stdout.startswith("accelerator ")`, and
  the calibration line is `once("/usr/bin/true")` — which prints nothing. Under
  `set -euo pipefail` the run dies before a single sample. Phase 4's only
  quantitative gate cannot be evaluated as written.
- 🟡 **Standards / Code Quality / Test Coverage**: **the launcher glob is not
  `ruff format`-stable and works by accident.** Confirmed: ruff reformats the
  generator expression (so `format:build-system:check` reds), and
  `Path("accelerator-launcher-9.9.9-test-darwin-arm64").suffix` is
  `'.9-test-darwin-arm64'` — the `!= ".minisig"` filter passes for a reason no
  reader would guess. The file already has the deterministic idiom at `:219`
  and `:272`: `cache / f"accelerator-launcher-{_VERSION}-{host_platform}"`.
- 🟡 **Architecture / Portability / Standards**: **`bin/.accelerator-before` is
  not gitignored, and jj auto-snapshots.** `.gitignore:42-48` enumerates every
  generated `bin/` artefact; this is not among them. jj snapshots the working
  copy on virtually every command, so any concurrent `jj` call during the
  multi-minute run commits an executable copy of a superseded trust-root
  bootstrap into `bin/`. The stated protection (a dot prefix) is not the
  operative mechanism either — shell-source discovery is an explicit allowlist.
  `bin/.tmp-accelerator-before` is already covered by the existing
  `bin/.tmp-*` rule.
- 🟡 **Correctness / Test Coverage**: **case 7 is misclassified as red-before.**
  Pre-change, `probe_dir` runs unconditionally in `resolve_cache_dir`, its write
  into the `0o555` dir fails, and the run already exits non-zero with the
  asserted substring. It is green-before, like the other preservation guards —
  so under the recorded procedure the cold-branch gate gets no verification at
  all.
- 🟡 **Test Coverage**: **`test_uncreatable_cache_dir_is_a_named_error` is
  vacuous.** If the `resolve_cache_dir` failure branch were deleted, the staging
  gate would emit the identical asserted substring. It needs
  `assert "could not be created" in output` — the only token distinguishing
  that site.

Found by reasoning, and I agree with each:

- 🟡 **Portability / Correctness / Test Coverage**: **the BSD/GNU `mkdir -p`
  hedge is now unreachable** — my own `[[ -d ]]` guard means `mkdir` never runs
  when the directory exists, so both lanes discharge the two `0o666` cases
  identically through the probe's write step. Phase 4 would write a false
  limitation into the work item, and it understates real coverage.
- 🟡 **Documentation**: **the read-only-cache promise is wrong for dispatch.**
  `cache_root.rs:80-94` *writes*, `chmod`s and execs a probe on every external
  subcommand, so a locked-down cache dir gives a working `version` and a hard
  failure on `vcs guard`. The internals paragraph mentions the launcher probe
  two sentences later without reconciling it; the changelog omits it entirely.
- 🟡 **Architecture / Standards / Documentation**: **the changelog commits to
  figures Phase 4 has not measured**, derived from the ~150 ms baseline the work
  item explicitly disclaims as a pre-0182 reference — with no Phase 4 step to
  reconcile them.
- 🟡 **Standards / Documentation**: **Phase 3 declares no automated
  verification while editing `bin/accelerator`**, which is in shell-source
  discovery (`tasks/shared/sources.py:110`) and gated by `scripts:check`.
- 🟡 **Correctness / Code Quality**: **the gate reports "noexec mount?" for
  what is a write failure in every tested scenario** — the plan's own analysis
  says the exec branch is never evaluated by any case, so the most reachable
  failure is misdiagnosed.
- 🟡 **Documentation**: **0186's own body keeps the ~11.7/~23 ms figures.**
  Phase 4 corrects only 0169's note — and 0169's corrected note will point
  readers back at 0186's uncorrected Dependencies, Assumptions and Validation
  Results.
- 🟡 **Correctness / Standards / Documentation**: **the criterion-9 method
  deviation is unrecorded, on a false premise.** Criterion 9 *does* state the
  method (a bash loop over 20 runs); the plan substitutes 50 interleaved
  single-process samples and asserts nothing was recorded — the one deviation
  on the *gating* criterion, and the only one missing from the deviation list.
- 🟡 **Performance**: **with hashing at ~8.4 ms, ~27–33 ms of the predicted
  41 ms is unattributed**, yet the plan still calls the residual "dominated" by
  hashing (~20% of ~41 ms). The composition check is nominated as the real
  confirmation but no composition budget is written down.
- 🟡 **Portability / Architecture**: **`/sbin/sha256sum` is a single-host,
  darwin-version-dependent observation stated as a darwin fact**, and it is
  load-bearing for the figure handed to 0169 — the Perl fallback swings the
  residual ~3× (~8.4 ms → ~26 ms).
- 🟡 **Portability**: **the cross-interpreter check has no mechanism** —
  `installation.py:41` pins `BASH` as a module constant precisely to hold the
  3.2 floor. It is also largely redundant: `ubuntu-latest`'s `/bin/bash` *is*
  bash 5.2, so the two trace cases already run under bash 5 on every CI run.
- 🟡 **Architecture**: **the launcher follow-up is overstated as decisive.**
  0169's own hand-off note already records the bootstrap alone at ~41 ms
  against a ≈38.6 ms gate, so the launcher fix is necessary but not sufficient
  — and framing it as decisive invites 0169 to defer an unavoidable decision.
- 🔵 Minor: `probe_exec_capable`'s write path returns before the single
  `rm -f` (partial-write leak); p90 index is off by one (`xs[45]` of 50 ≈ p92);
  sample-validity rests on `assert`, removed under `-O`; the interleave keeps a
  fixed within-pair order so `after` is always second; the floor is measured but
  never subtracted, so the composition check compares instrument-inclusive
  against instrument-exclusive quantities;
  `test_tampered_cached_launcher_is_refused_and_healed` is described as
  no-opping when it actually runs a full probe; `probed=1` is set before the
  probe succeeds; and four new comments embed staleness vectors (`~108ms`,
  `300 x 0.1s`, "after the split", a test-file line range) against the
  project's explicit standard.

### Assessment

The plan's *design* is now in good shape and materially better than pass 1 —
the two-gate placement was independently verified complete by two lenses, and
the redesign eliminated three pass-1 findings structurally rather than
patching them. What needs another iteration is execution detail, concentrated
in two places: the Phase 4 measurement script (which cannot run as written, and
carries four smaller instrumentation defects plus a working-tree hazard), and
the accuracy of user-facing text (a read-only promise the launcher contradicts,
changelog figures that precede their measurement, and a stale coverage
limitation that would be written into the permanent record).

Two substantive items are worth a decision rather than a fix: whether to hand
0169 a *range* for the hashing residual rather than a single host's figure, and
whether to raise a follow-up for batching the two hashes into one
`sha256_file` invocation — an option no earlier pass considered, measured at
roughly 4–4.5 ms of the ~41 ms landing, with no change to the trust boundary
since both digests are still computed and compared.

---

## Disposition — 2026-08-03

**Verdict moved to APPROVE by the author after the pass-2 findings were
addressed. No third review pass was run**, so the frontmatter verdict reflects a
judgement call rather than a lens result. Recorded explicitly so a later reader
does not read `verdict: APPROVE` as the output of a review.

The pass-2 findings were addressed in the plan (see its revision history), and
the fixes were verified mechanically rather than by re-review:

- The four new bash functions parse under bash 3.2.57, pass
  `scripts/lint-bashisms.sh`, are ShellCheck-clean and shfmt-stable.
- `probe_exec_capable` was exercised in all three outcomes — success, write
  failure, exec failure — returning 0/1/2 with no probe file left behind on any
  path, closing the partial-write leak the earlier single-early-return form had.
- `ensure_dir`'s `[[ -d ]]` guard was checked against an existing directory, an
  absent nested path, an unwritable parent, and a path occupied by a regular
  file; failure modes are unchanged from `probe_dir`.
- All six Python blocks in the plan are `ruff format`-stable as written (the
  earlier launcher-glob generator expression was not, and was replaced with the
  file's established deterministic path idiom).
- The revised measurement script was dry-run end to end: both instrument floors
  compute, the sample loop completes, and a deliberately failing variant aborts
  the run rather than being recorded as a fast sample. The p90 index was checked
  to be nearest-rank (index 44 of 50).
- The trace matcher was verified against a real captured trace: the anchored
  pattern matches exactly the exec line, where the previous pattern also matched
  the function's own `probe=…` assignment.
- The batched-hash follow-up carries a figure measured here (~2.5 ms), not an
  estimate inherited from a review agent.

**Known open assumption, deliberately carried into Phase 4 rather than closed:**
the ~108 ms probe cost and its ~97 ms "macOS first-exec check" attribution are
inherited from the work item's Context table and were not re-derived. The probe
writes a `#!/bin/sh` *script* rather than a Mach-O, so no binary code-signing is
involved, and the table's "re-exec of a pre-existing probe file | 10.6 ms" row is
roughly seven times the 1.48 ms fork+exec floor measured on this host. If that
penalty proves host-specific (a scanning agent rather than a platform
characteristic), the `after ≤ 0.5 × before` gate is unsatisfiable on hosts
without it and will need replacing. Phase 4 is instructed to re-derive the
attribution and record the host's security-agent status.

---

*Review generated by /accelerator:review-plan*
