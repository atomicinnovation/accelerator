---
type: work-item
id: "0186"
title: "Remove the Exec Probe from the Bootstrap Warm Path"
date: "2026-07-31T10:41:51+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: task
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0182"]
blocks: ["work-item:0169"]
relates_to: ["work-item:0164", "work-item:0165", "work-item:0167"]
tags: [shell, performance, bootstrap]
last_updated: "2026-07-31T12:34:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0186: Remove the Exec Probe from the Bootstrap Warm Path

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

`bin/accelerator` runs an **exec probe** on every invocation — it writes a fresh
executable into the cache dir, `chmod +x`es it, runs it and removes it — to
detect a `noexec` mount. That probe costs **~108 ms of the ~149 ms** a warm
`bin/accelerator` invocation takes (bootstrap plus launcher), almost all of it
macOS's first-exec check on the newly written file. Remove the exec probe from
the warm path, keeping it on the cold path where it belongs.

## Context

Throughout this work item, **cold path** means an invocation that must fetch
and verify a launcher binary because no verified one is cached for the running
platform, and **warm path** means an invocation that finds one already cached
and execs it. "Cold branch" and "warm branch" refer to the code implementing
each. `bin/accelerator` probes on both today, because the `resolve_cache_dir`
call site runs unconditionally at `bin/accelerator:195` — before the branch.

Measured on darwin-arm64, warm cache, 20 iterations each (2026-07-30):

| Path | ms/call |
| --- | --- |
| `hooks/vcs-guard.sh` (today's shell guard) | 35.1 |
| `bin/accelerator version` (bootstrap + launcher) | 149.1 |
| launcher binary invoked directly | 3.0 |
| minisign verify of the 8 MB launcher | 2.3 |
| `probe_dir` write + chmod + exec + rm | **107.9** |
| re-exec of a *pre-existing* probe file | 10.6 |

The last two rows isolate the cause: executing a *freshly written* binary costs
~97 ms more than re-executing an existing one, so ~97 ms of the probe's 107.9 ms
is macOS's first-exec check and the rest is filesystem work. Everything the
fetch-verify-cache design is usually blamed for — minisign, dispatch, the
launcher itself — totals under 6 ms. Removing the probe alone should therefore
land the warm bootstrap near **149.1 − 107.9 ≈ 41 ms**; that is the figure the
latency gate is calibrated against.

Every SessionStart hook pays this cost today via `hooks/config-detect.sh`, and
every future CLI-backed hook will. This work item was extracted from 0169
(review-2, pass 4) as an independently deliverable change with its own risk
profile: the change touches the bash bootstrap under the 3.2 floor, not the
Rust CLI.

## Requirements

- The warm path performs no write-chmod-exec probe. The exec probe is redundant
  once a verified launcher binary is cached: the warm path already execs both
  the verify shim and the launcher binary from the cache dir, which are stronger
  exec tests than a synthetic probe, and a `noexec` directory makes
  `verify_launcher` fail into the cold branch where the probe belongs.
- Mechanism: split `probe_dir` (definition, `bin/accelerator:166-180`) into
  `ensure_dir` (the `mkdir -p`, which still runs on every invocation) and the
  write-chmod-exec probe, calling the latter only on the cold path before
  fetching. The probe's result never *chooses* a directory —
  `resolve_cache_dir` (definition, `bin/accelerator:184-193`) has no fallback —
  so moving it changes only where the `no writable, exec-capable cache
  directory` diagnostic fires. That diagnostic is emitted by the caller, at the
  `resolve_cache_dir` call site and the `fail` that follows
  (`bin/accelerator:195-197`).
- Extend `tests/integration/entrypoint/test_accelerator_entrypoint.py` (reached
  from `mise run test:integration:entrypoint`) with the cases the Acceptance
  Criteria name. The suite already provides the harness — a fake release
  server, an injected downloader, and the `_run_bootstrap` helper — and already
  runs green-path bootstraps end to end
  (`test_happy_path_forwards_args_and_exit_code`, which asserts a forwarded exit
  code through a real launcher exec, and
  `test_cache_hit_performs_no_further_fetch`, which warms then re-invokes), as
  well as the cold-path permission counterpart
  `test_readonly_root_without_override_is_a_named_error`. One capability is
  *not* already present: threading `bash -x` and a custom `PS4` through
  `_run_bootstrap` and capturing the trace, which the direct probe-absence
  criterion needs — a small helper extension inside the existing suite, in
  scope here.
- Keep `bin/accelerator` bash-3.2-safe.

**Already discharged during review (2026-07-31)**: a dated hand-off note was
appended to 0169's Dependencies recording the residual warm-path cost this item
does not remove and its consequence for 0169's latency criterion. It survives as
a **verification** step, not new work — see the corresponding criterion. The
deliverable is the note only; **changing 0169's threshold or its rationale is
0169's own work**, not this item's.

## Acceptance Criteria

Criteria 1-4 and 6 are **automated cases** in
`tests/integration/entrypoint/test_accelerator_entrypoint.py`, reached from
`mise run test:integration:entrypoint`, so they act as permanent regression
guards. Criteria 5, 9, 10 and 11 are **recorded checks** discharged by an entry
in Validation Results. Criterion 7 is a lint gate and criterion 12 is the
aggregate build.

Three automated criteria are permission-dependent (the `chmod 0o555` warm case,
the `chmod -x` cold case, and the `chmod -x` warm-to-cold case). One rule
governs all three and supersedes any per-criterion wording: each asserts
`id -u` ≠ 0 and **hard-fails rather than skips** when run as root, since root
bypasses both directory write permissions and the execute bit and would satisfy
the assertions regardless of the implementation. A lane structurally unable to
comply is **excluded explicitly by the implementer**, never skipped silently,
and the exclusion is justified by a recorded **privilege check** — `id -u`
returning 0, or, for a filesystem that ignores the permission bits, a temp-dir
check (`chmod 0o555` then assert file creation inside fails; `chmod -x` then
assert traversal fails). The check's command and output are pasted into
Validation Results for any exclusion claimed.

- [ ] **Warm path performs no fatal exec probe** — verified **behaviourally**,
      not by looking for the probe's residue (it is created and removed within
      one invocation, so a post-hoc check passes either way). Sequence: run a
      full bootstrap against the harness to populate the cache dir,
      `chmod 0o555` that directory (executable, not writable), then assert a
      second invocation exits 0 with the expected `version` output — the
      version the harness fixture builds, asserted exactly. This works because
      the warm path performs no writes (see Assumptions), so a *fatal*
      surviving probe fails its write and the invocation exits non-zero. It
      does **not** catch a probe kept and made non-fatal, which still exits 0;
      that variant is ruled out only by the next criterion.
- [ ] **Probe absent from the warm path, asserted directly** — the criterion
      that actually pins probe absence, and therefore load-bearing rather than
      supplementary. Run the bootstrap under `bash -x` with
      `PS4='+${FUNCNAME[0]}:'` and assert the **probe function's name** (as
      named after the split) is **absent** from the warm-run trace. The signal
      must be the function name, not the `.accelerator-probe-` filename and not
      a generic write or `chmod`: it stays stable across renaming the probe
      *file*, and — unlike a generic pattern — it does not also match the cold
      path's own shim staging (`cp` plus `chmod` at `bin/accelerator:257-260`)
      and launcher write, which occur into the same directory regardless of the
      probe. A generic pattern would make the positive control below vacuous.
      Note bash's xtrace prints expanded command words but **not** redirections,
      so a probe that creates its file via `>` emits no cache-dir path at all —
      another reason the function name is the only reliable observable.
- [ ] **Positive control for the trace assertion** — the same probe function
      name must be asserted **present** in the trace of the cold happy-path run
      (criterion 6), and the trace must additionally show the probe file being
      **executed**, not merely written and `chmod`ed. Without this the negative
      assertion above passes whenever the trace stops matching for unrelated
      reasons. The exec half also gives the retained cold-path probe its only
      real verification — see criterion 5 for why the `chmod -x` cases cannot
      supply it.
- [ ] **The `noexec` diagnostic survives on the cold path**: with an empty
      cache dir made non-executable (`chmod -x`), a cold invocation exits
      non-zero and its combined output contains the substring
      `no writable, exec-capable cache directory` (`bin/accelerator:196`).
- [ ] **The exec-vs-write coverage limitation is recorded.** Both `chmod -x`
      criteria create their failure by removing a directory's execute bit,
      which also blocks creating files inside it — so an implementation
      retaining only the *write* half of the probe would produce the same exit
      and the same diagnostic, and neither criterion distinguishes the two.
      Record this in Validation Results, noting that the exec half is instead
      covered by the positive control's execution assertion (criterion 3).
      Mounting a genuinely `noexec` filesystem (a `mount -o noexec` tmpfs, or
      an attached disk image on darwin) would close it completely and is
      **explicitly out of scope here** — raise it as a follow-up if the
      cold-path probe ever regresses.
- [ ] **Cold happy path after the split** — pins the always-run `ensure_dir`
      half: with `ACCELERATOR_CACHE_DIR` set to a path that does not yet exist,
      a cold invocation creates the directory and `bin/accelerator version`
      exits 0 with the expected output. This is the run that carries the
      positive control (criterion 3).
- [ ] **Diagnostic preserved when a warmed cache dir becomes non-executable**:
      given a *warmed* cache dir subsequently made non-executable, the
      invocation exits non-zero with the same `no writable, exec-capable cache
      directory` substring. Note this is an end-state guard, not proof of
      routing — today's bootstrap produces the same result by probing before
      the branch, so the criterion cannot by itself demonstrate the
      warm-to-cold fall-through. The routing claim is carried by criterion 3.
- [ ] `bin/accelerator` passes the bash-3.2 gate — `scripts/lint-bashisms.sh`
      plus the standard shfmt/ShellCheck checks report no findings.
- [ ] **The warm-path saving clears its gate.** Take the median of 20 warm
      `bin/accelerator version` invocations, on one darwin-arm64 host in a
      single session with no build running, using the same method for both
      figures (a bash loop over 20 runs taking the median, matching how the
      Context table was produced). The **before-median is re-measured on the
      post-0182 baseline immediately before the change, in the same session** —
      the Context table's 149.1 ms is a pre-0182 reference figure, not this
      gate's input. **The pass condition is `after ≤ 0.5 × before`**, which is
      host-relative and so survives a faster or slower machine; the expected
      landing point is ~41 ms against a ~149 ms baseline. The absolute delta
      (`before − after`) is **recorded but not gating**, because the ~108 ms
      probe cost is itself host-specific and a fast host could remove the whole
      probe yet miss a fixed 80 ms bar. Both medians, the delta, and the host
      and OS version go in Validation Results.
- [ ] `test:integration:entrypoint` is observed green on **both shipped lanes**
      (darwin and linux) for the new cases, not only locally — these are the
      most environment-sensitive criteria in the suite. Which lanes were
      observed is recorded in Validation Results.
- [ ] **The 0169 hand-off note is present and still accurate.** Confirm the
      dated note in 0169's Dependencies still holds after the rebase and after
      the measurement lands — in particular that the quoted residual and its
      consequence for 0169's `≤ 1.1 × B` gate match the measured after-median.
      Update the figure if it moved; do not change 0169's threshold.
- [x] The shim double-hash decision is recorded with its rationale — discharged
      during review-1; see the Validation Results entry and Open Questions.
- [ ] `mise run` is green end to end.

## Open Questions

None outstanding. The one question this item carried — whether to also drop the
verify shim's second `sha256_file` call — was **resolved on 2026-07-31 during
review-1: keep both hashes and the staging block unchanged**, and the shim
staging is explicitly out of scope here. The reasoning and evidence are recorded
once, in Validation Results.

## Dependencies

- **Blocked by**: 0182 (in-progress) also edits `bin/accelerator`, for
  plugin-root self-location. **0182's changes land first and this work item
  rebases onto them.** The edge is **discharged when 0182's `bin/accelerator`
  and entrypoint-suite changes are on `main`, not when 0182 reaches
  `complete`** — 0182's closure is gated on a manual pre-release check against a
  clean install of a signed release artifact, and blocking a bash micro-change
  (and transitively 0169 and its five children) on a release cut would be
  wrong. The two items share **two** artefacts:
  `bin/accelerator` itself, and
  `tests/integration/entrypoint/test_accelerator_entrypoint.py`, which 0182 also
  reworks. So every `bin/accelerator` line number *and* every test name or line
  range quoted here was read pre-0182 and must be re-resolved by name on
  pick-up. The rebase check must additionally confirm two premises this item
  rests on: that the self-located plugin root still yields a single `cache_dir`
  with no fallback, and that the entrypoint suite still supplies a launcher
  **without** introducing a `build:cli:dev` edge (0182 changes how the launcher
  reaches the suite). If either has changed, re-scope before starting.
- **Blocks**: 0169 — but the constraint binds at 0169's **acceptance, not its
  start**. Only 0169's warm-call latency criterion depends on a fixed
  bootstrap; its implementation work can proceed in parallel. **This work item
  is necessary but may not be sufficient for that criterion.** Removing the
  exec probe alone leaves the warm bootstrap at roughly 41 ms, while 0169
  requires a warm `accelerator vcs guard` at ≤ 1.1 × the 35.1 ms shell guard
  (≈ 38.6 ms) and pays a sub-binary exec and verify on top. The residual is the
  verify shim's staging block — **~11.7 ms for the second `sha256_file` alone,
  or ~23 ms if staging were skipped entirely**; neither is addressable without
  weakening a tested trust boundary (see Validation Results). So 0169 must
  either relax its threshold or accept the cost with a stated rationale. A
  dated hand-off note in 0169's Dependencies carries this, so the obligation
  does not die with this task.
- **Completed dependencies (not blocking)**: 0164 (fetch-verify-cache — created
  `probe_dir`, `resolve_cache_dir`, the staged verify shim and
  `verify_launcher`), 0165 (multi-binary distribution and release pipeline —
  produces the signed assets the measurement's first-choice launcher comes
  from) and 0167 (bootstrap invocation contract). All have landed on `main`;
  0167's work item is still `ready` despite its code shipping, so read the code
  as authoritative over its status.
- **External**: the **release-artefact host**. The behavioural criteria need
  none of it — the suite fetches from a fake release server via an injected
  downloader — but the latency gate measures a real `bin/accelerator version`,
  which needs a genuine cached launcher for the measuring host. If no matching
  release asset is published (routine mid-epic), the measurement uses a locally
  staged launcher or an already-cached binary; state which in Validation
  Results. **Both medians must use the same launcher binary, and it must be
  post-0182**: 0182 makes the bootstrap export only `ACCELERATOR_PLUGIN_ROOT`,
  so a pre-rename launcher finds no root and silently degrades — measuring
  either side against one would invalidate the comparison.
- **Execution environment**: the permission-dependent criteria require a
  **non-root** runner on a filesystem that honours the write bit and the
  directory execute bit. `test:integration:entrypoint` runs unprivileged on the
  current darwin and linux lanes. The exclusion rule and its privilege check
  are stated once in the Acceptance Criteria preamble.
- **Parent**: epic 0136.

Note on `status: ready` alongside a populated `blocked_by`: this is deliberate.
0182's code has landed and only its release-gated closing steps remain, and the
edge above is discharged on merge rather than on 0182's closure — so the item is
genuinely startable once the rebase baseline exists.

## Assumptions

- The ~108 ms figure is a macOS first-exec penalty with no Linux equivalent, so
  Linux savings will be materially smaller. Darwin is the worst case and the one
  worth measuring.
- **The warm path performs no writes**, which is what makes the removal
  observable as a behaviour change and the `chmod 0o555` criterion sound. It
  holds today: the shim-staging *body* (the `cp` and `chmod` at
  `bin/accelerator:257-260`) is skipped when the staged bytes already match the
  source digest, and the lock directory is acquired only on the cold path. Note
  the staging `if`'s **condition** (`:255-256`) is still evaluated on every
  invocation and contains the second `sha256_file` — that reads, it does not
  write, so the invariant stands while the ~11.7 ms cost remains. If the warm
  path proves not to be write-free on some configuration, that is a finding to
  raise, **not work to absorb here** — the staging block is out of scope.
- The latency gate assumes 0182's `bin/accelerator` changes leave warm-path
  latency broadly unchanged. Expressing the gate as a ratio against a freshly
  measured before-median means a materially different post-0182 baseline shifts
  both sides together rather than failing a correct implementation.

## Technical Notes

Line numbers below are indicative at time of writing and predate 0182; each is
paired with its enclosing function or distinguishing code, and labelled
definition or call site, so it can be re-resolved after the rebase.

- This item says "exec probe" on first use in each section and may shorten it
  thereafter. The qualifier matters across the epic because 0169 reserves
  "probe layer" for the shell VCS detection functions in
  `scripts/vcs-common.sh`. The environment check in the Acceptance Criteria
  preamble is deliberately called a **privilege check**, not a probe.
- `probe_dir` exists to catch a `noexec` mount that a write-only check would
  miss — a real hazard, which is why the cold-path exec probe is retained rather
  than deleted outright. That same property is what the `chmod -x` criteria
  cannot verify, which is why the positive control asserts the probe file is
  executed.
- `verify_launcher` (`bin/accelerator:310-312`) execs the staged shim from
  `cache_dir`, and the final `exec "${launcher}"` (`bin/accelerator:352`) execs
  the launcher binary from the same directory. Either failing is a stronger
  signal than the synthetic exec probe.
- The shim staging block spans `bin/accelerator:255-261`: its condition (`:255`
  and `:256`) hashes both the source and the staged shim on every invocation,
  and its body (`:257-260`) does the copy and `chmod` only when they differ. The
  second `sha256_file` at `:256` is what Validation Results resolves to keep.
- Attribution evidence: re-executing a *pre-existing* probe file costs 10.6 ms
  against 107.9 ms for writing a new one each time — the delta is the
  first-exec check, not filesystem work.

## Drafting Notes

Interpretations made while extracting this item from 0169 (review-2, pass 4)
and during its reviews (review-1, passes 1-3) — each is an author call open to
challenge:

- **The warm-path exec probe is redundant, not merely expensive.** The argument
  rests on `verify_launcher` and the final `exec` being strictly stronger exec
  tests of the same directory. If either could ever run from somewhere other
  than `cache_dir`, the argument weakens and the probe would need a narrower
  replacement rather than relocation.
- **The cold-path probe is retained rather than deleted.** A `noexec` mount is
  a real hazard and the diagnostic is worth keeping; the cheaper alternative
  (delete the probe outright and let the fetch fail with a less specific error)
  was rejected as a worse diagnostic for a rare but confusing failure.
- **The shim double-hash stays.** Resolved during review-1 on the strength of
  three existing tests asserting the planted-stub defence. This is the judgement
  most likely to be revisited if warm-path latency becomes binding again.
- **The exec branch is verified by trace, not by a `noexec` mount.** Asserting
  the probe file is executed in the cold-run xtrace costs nothing and
  distinguishes the exec half from the write half; a real `noexec` mount would
  be stronger but needs privileged, per-platform CI setup that would dwarf the
  production change. The residual difference is recorded, not hidden.
- **The latency gate is a ratio, not a fixed delta.** An absolute ≥ 80 ms
  saving would fail a correct implementation on a host whose first-exec penalty
  is smaller than darwin's ~97 ms. The delta is still recorded for comparison
  against the 2026-07-30 table.
- **0182 sequences ahead of this item** on the grounds that it is nearly
  complete, not on any technical ordering requirement, and the edge is
  discharged on merge rather than on 0182's release-gated closure. Reversing the
  order is viable if 0182 stalls; the cost is re-resolving this item's line and
  test references against a different baseline.
- **No successor item is raised for the residual staging cost.** Recording the
  shortfall against 0169's threshold, and carrying it to 0169 as a dated note,
  was preferred over creating a work item for a change that is a security
  trade-off rather than an optimisation.
- **The verification burden is accepted deliberately.** The criteria outweigh
  the production change several times over. That is proportionate for a file
  every SessionStart hook executes, but it means the measurement session and
  the cross-lane observation are genuine closure conditions, not formalities.

## Validation Results

- **Warm-path exec-probe-free check** (non-writable populated cache dir) —
  _pending_.
- **Direct probe-absence check** (`bash -x`, `PS4='+${FUNCNAME[0]}:'`, probe
  function name absent from the warm trace) — _pending_.
- **Positive control** (probe function name present in the cold trace, and the
  probe file observed being executed) — _pending_.
- **`noexec` cold-path check** — _pending_.
- **Exec-vs-write coverage limitation recorded** — _pending_.
- **Cold happy-path (`ensure_dir`) check** — _pending_.
- **Diagnostic preserved on a warmed-then-non-executable cache** — _pending_.
- **Lanes observed green** (darwin, linux) — _pending_.
- **Lane exclusions**, each with the privilege-check command and output that
  justified it — _pending_ (none expected).
- **Warm-path median, before** (re-measured post-0182) — _pending_; **after** —
  _pending_; gate `after ≤ 0.5 × before` — _pending_; delta (recorded, not
  gating) — _pending_; host and OS version — _pending_; launcher provenance
  (release asset, locally staged, or pre-cached) and confirmation both medians
  used the same post-0182 binary — _pending_.
- **0169 hand-off note** — appended 2026-07-31; **re-confirmation after rebase
  and measurement** — _pending_.
- **Shim double-hash decision** — **resolved 2026-07-31: keep both hashes and
  the staging block unchanged.** The source research (§12) framed this as an
  open trade-off worth ~23 ms via two distinct changes: dropping the second
  hash at `bin/accelerator:256` (~11.7 ms, one hash of the 475 K shim), or
  skipping staging entirely when `shim_source`'s directory and `cache_dir`
  resolve to the same path (~23 ms, the default configuration where both are
  `${plugin_root}/bin`). Neither is as cheap as it looked: the planted-stub
  defence the second hash provides is **asserted by three existing tests** —
  `test_planted_staged_shim_rehashed_then_succeeds`,
  `test_planted_staged_shim_is_not_trusted` and
  `test_planted_staged_shim_via_cache_dir_is_not_trusted`
  (`tests/integration/entrypoint/test_accelerator_entrypoint.py:584-644`) — so
  removing it deliberately weakens a tested trust boundary rather than clearing
  a redundancy, and skipping staging would drop the same check by running the
  shim from `shim_source`. That is a security decision with its own risk
  profile and belongs in its own work item if it is ever wanted. The cost stays
  on the warm path; the consequence for 0169's latency threshold is recorded in
  Dependencies.

## References

- Measurements and attribution:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §12
- Extracted from: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`, whose
  review-2 (pass 4) produced this split:
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`
- Review of this work item, in `meta/reviews/work/`:
  `0186-remove-exec-probe-from-bootstrap-warm-path-review-1.md`
- The fetch-verify-cache design this item edits:
  `meta/work/0164-launcher-and-git-style-dispatch.md`
- Test surface: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
  (`mise run test:integration:entrypoint`)
- Sequenced against:
  `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0049 (bash 3.2 compatibility floor)
