---
type: "plan-review"
id: "2026-08-11-0196-design-cli-migration-review-1"
title: "Plan Review: accelerator-design: CLI Migration and Shell-Free Executor"
date: "2026-08-11T22:39:16+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "work-item:0196"
target: "plan:2026-08-11-0196-design-cli-migration"
relates_to: ["plan-review:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "architecture", "test-coverage", "code-quality", "security", "compatibility", "safety", "performance"]
review_number: 1
review_pass: 3
tags: ["rust", "design", "cli", "sub-binary", "executor", "playwright"]
last_updated: "2026-08-12T11:19:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: accelerator-design: CLI Migration and Shell-Free Executor

**Verdict:** REVISE

This is the stronger half of the split, and the lenses say so: the domain/adapter/CLI
split with domain purity enforced by cargo-pup, the reuse verdict as a pure total
function over injected ports, rejection modelled as a verdict rather than an inverted
`kernel::Error`, and a `test-design.sh` disposition table whose 24 line ranges were
independently verified accurate to the block boundary. But five criticals stand, and
their distribution is instructive: **two are editing residue from the split** (a section
that still specifies the design an adjacent section rejects), **two are claims inherited
from the superseded plan that no earlier pass checked** (a bash suite the plan says does
not exist; a third `run.sh` call site the plan says does not exist), and **one is a new
design error introduced by the pass-3 fix itself** — the spawn-time identity handoff
cannot be implemented in the order stated, because a child's environment is fixed at
`execve`, which necessarily precedes the child existing.

### Cross-Cutting Themes

- **The daemon identity contract is specified two ways** (flagged by: correctness,
  architecture, code-quality, compatibility — four lenses). Phase 6 §1 settles on the
  launcher handing values to the daemon at spawn with `state.js` as sole writer, and
  gives the reasoning at length. Phase 6 §3 then restates the rejected design verbatim
  ("writes the identity record once the daemon reports ready", "`start_time_source` … a
  field the Rust writer sets"). This is the single most regression-prone interface in the
  plan, and it has two incompatible specifications in adjacent sections.

- **The `exec` terminal path invalidates two stated mechanisms** (correctness,
  architecture, code-quality). `RunClient` is declared to "return an exit outcome" with
  `CommandExt::exec` as its implementation — `exec` never returns, so any domain logic
  sequenced after it runs in tests and never in production. The same applies to "the lock
  is released … by a `Drop` guard, on every path": no destructor runs at `exec`, so
  release happens incidentally via `O_CLOEXEC`, and `run.sh:152,202` releases explicitly
  before `exec` for exactly this reason.

- **Types cannot carry the classifications their own sections demand** (correctness,
  code-quality, compatibility, security). `HostReach`'s five variants cannot express the
  reserved ranges the same section enumerates (`100.64.0.0/10`, `192.0.0.0/24`,
  `198.18.0.0/15`, `240.0.0.0/4`) — and that classification is user-facing text.
  `(RecordedDaemon, ObservedDaemon)` cannot represent the `NoPidRecorded` row of its own
  totality table. `Verdict<Reason>` cannot express the executor's exit 3, its
  stdout-at-exit-0 daemon errors, or signal-death propagation.

- **Coverage-loss gates are placed on the low-risk half** (test-coverage, safety). The
  Removal sweep requires re-homed assertions to be *shown to fail when broken* — and
  those are the structural and docs re-homes. Phase 2 deletes ~200 lines of
  `validate-source` SSRF-boundary assertions under a weaker rule ("maps to a named Rust
  test or is recorded as a deliberate drop"), on the code the plan itself calls the front
  door for a tool that drives a headless browser.

- **New security-relevant behaviour has no named test** (test-coverage, security). The
  plan correctly spots that a shell-derived checklist cannot demand tests for behaviour
  the shell lacked, and fixes it for `host_reach` with a dedicated criterion — then omits
  the same treatment for `leaked_credentials`' new value-half split, the one security
  improvement among the ported subcommands.

- **The executor's adapter half has criteria but no mechanism** (test-coverage, safety,
  performance). Every behaviour ADR-0058 names as a silent-regression risk —
  `setsid`, stdio redirection, log truncation and mode, exit-status and signal
  propagation, `server-stopped.json` removal — lands in `design-adapters`, is asserted as
  automated, and has no named harness. Meanwhile `test-run.sh` is deleted and its
  replacement lane runs in no CI job.

### Tradeoff Analysis

- **Lock semantics: fail-safe refusal vs. correct reuse.** The plan changes the lock to
  release at launcher exit, arguing that today's inherited-FD behaviour makes a
  stale-start-time recovery falsely report `another-launcher-running`. Safety points out
  the FD leak is also a mutual exclusion on *daemon existence*, so the change converts a
  refusal into "delete a live daemon's state and start a second browser", with the
  displaced daemon unreachable by `daemon-stop`. **Recommendation**: keep the change, but
  state what recovery does about a daemon that is still alive when its state is judged
  stale — an identity-validated shutdown before respawn, or a retained refusal for the
  live-but-unidentifiable case.

- **Warm-path measurement: floor vs. directional gate.** The plan says the net latency
  effect is "not obvious in either direction" and gates on "no slower than today".
  Performance resolves it with numbers already in the repo: today's `run.sh` pays a
  *nested full `bin/accelerator` bootstrap* (`run.sh:77`, measured at 29.92ms median in
  work-item:0186) on every invocation, against ~5–10ms for the added sub-binary resolve
  and re-verify — so the port should land 20–45ms faster per call, 3–9s per crawl.
  **Recommendation**: state the expected direction and gate on a ratio, so failing to
  capture the win is visible rather than merely "not worse".

- **Token threat model: same-uid vs. browser-origin.** Security agrees the token is worth
  adding but says its stated justification (any local process, including the model) is
  wrong — the token sits in a mode-0600 file the model can read. What it actually closes
  is CSRF and DNS rebinding from the pages the browser itself crawls, which the plan's
  own header-only transport is precisely what defeats. **Recommendation**: restate the
  threat, and pin the properties (CSPRNG ≥128 bits, header-only, constant-time compare,
  reject requests carrying `Origin`) so an implementer cannot "simplify" it into a query
  parameter.

### Findings

#### Critical

- 🔴 **Correctness**: The spawn-time identity handoff is impossible in the stated order
  **Location**: Phase 6 §1 ("One observer, one writer, one atomic write")
  The launcher is to "know the pid at fork and observe the start time", then hand
  `(pid, start_time, start_time_source, token)` to the daemon "through the environment".
  A child's environment block is fixed at `execve`, which necessarily precedes the child
  existing, and `std::process::Command` builds `envp` before `fork` — a `setenv` in
  `pre_exec` is not reflected in the `envp` handed to `exec`. Only the token (known
  pre-spawn) can travel this way. **Suggestion**: pass the values over an inherited pipe
  the daemon reads before its single write, or observe the start time in the forked child
  before `exec` and pass them via a manually constructed environment; state the readiness
  ordering explicitly.

- 🔴 **Correctness / Code Quality / Architecture / Compatibility**: §3 still specifies the
  identity-record writer that §1 explicitly rejects
  **Location**: Phase 6 §3 vs Phase 6 §1
  §1 rejects launcher-side writing at length (the partial-record window; "one file two
  whole-file-rename writers, which is a lost-update contract"). §3 says the launcher
  "writes the identity record once the daemon reports ready" and calls
  `start_time_source` "a field the Rust writer sets". Implementing §3 reproduces both
  defects §1 eliminated and leaves the token unenforceable until the daemon is already
  accepting connections.

- 🔴 **Test Coverage**: Both metadata scripts Phase 3 deletes are driven by a bash suite
  the plan says does not exist
  **Location**: Phase 3 §2 ("Neither script has a bash suite, so no floor moves")
  `scripts/test-metadata-helpers.sh:22-23` names both `inventory-metadata.sh` and
  `gap-metadata.sh` in its `HELPERS` array and asserts their output contract in hermetic
  git and jj temp repos. It runs in **two** lanes: `tasks/test/unit.py:41` as a
  `test:unit:templates` driver, and glob-discovered by `test:integration:config`.
  Verified directly. Phase 3 cannot merge green, the floor arithmetic is wrong, and the
  handoff to the sibling plan (15-against-15) breaks — deleting the suite lands
  `scripts/` on 14.

- 🔴 **Safety**: A third `run.sh` call site — the daemon-shutdown path — is in no phase's
  edit set
  **Location**: Phase 6 §5 and Migration Notes ("the two places an earlier draft missed")
  `skills/design/inventory-design/SKILL.md:299` invokes
  `.../playwright/run.sh daemon-stop` in Step 12 "Cleanup", described as the
  belt-and-braces shutdown "even if an agent exits abnormally". Verified directly. Phase 6
  §8 deletes `run.sh`, so the only deterministic stop path invokes a nonexistent file — and
  no CI gate catches it (`call_site_migration.py` greps only `scripts/config-`; the
  invocation is a fenced block, not a `!` preprocessor site). Every inventory run then
  leaves a headless Chromium holding the crawled page until the 10-minute idle timeout.

- 🔴 **Safety / Correctness**: No verdict row for a writer-side unavailable start time;
  one reading leaks a Chromium per invocation
  **Location**: Phase 6 §1 (verdict table) and §3
  §3 deletes `state.js`'s wallclock fallback and says a writer-side `Unavailable` probe
  "still has to be recorded as such" — but `RecordedStartTime` has no such variant. If it
  records `AbsentOrUnparseable`, the table's own rule recovers on **every** invocation:
  on the `/proc`-restricted and distroless hosts §3 names, that is one orphaned daemon plus
  one Chromium per executor call, each unreachable because its `server-info.json` was
  deleted.

#### Major

Thirty majors were raised; the full text of each is in the Per-Lens Results below. The
highest-impact clusters:

- 🟡 **Correctness / Architecture / Code Quality**: `RunClient` declared to return while
  implemented by `exec`, and the `Drop`-guard lock release dead on the hot path.
  **Location**: Phase 6 §1 (Lock, `RunClient`), §3
- 🟡 **Correctness / Architecture / Compatibility**: The lockhash namespace derivation is
  in no ported-behaviour inventory and nothing pins the Rust digest to the surviving
  `ensure-playwright.sh`'s. An off-by-anything returns exit 3 on correctly-bootstrapped
  machines. **Location**: Phase 6 §1, Phase 6 Overview
- 🟡 **Correctness / Architecture**: The executor's path inputs (plugin root, `run.js`,
  `config path tmp`, namespace root) have no stated source and no seam, while two
  envelopes embed absolute paths that AC2 requires to be deterministic.
  **Location**: Phase 6 §1
- 🟡 **Security**: The forwardable-command allowlist is bypassable — `client.js:18`
  spreads caller-supplied `args` last, so `executor ping '{"command":"evaluate",…}'`
  overrides both `command` and `protocol`. **Location**: Phase 6 §1, §3
- 🟡 **Security**: `client.js:20-29` takes `info.url` verbatim, so anything able to
  rewrite the now-secret-bearing `server-info.json` redirects the next invocation — token
  included — to an arbitrary host. **Location**: Phase 6 §3
- 🟡 **Security**: The reserved set omits IPv4-compatible `::a.b.c.d`, so
  `::169.254.169.254` classifies as public; and host strings are not normalised for
  percent-escapes or control characters that Chromium resolves but the validator will
  not. **Location**: Phase 2 §1
- 🟡 **Security**: The path branch has no containment — `validate-source /Users/me/.ssh`
  exits 0 today and the rewrite preserves it, with no module-doc note.
  **Location**: Phase 2 §1
- 🟡 **Security / Compatibility**: `Bash(… accelerator corpus *)` grants the whole corpus
  surface where nine other skills declare the exact subcommand.
  **Location**: Phase 3 §1
- 🟡 **Compatibility**: The token lands against `PROTOCOL.md`'s v1 stability commitment,
  which lists required new request fields as a v2 bump — with no version handling, no
  tokenless-record rule, and PROTOCOL.md largely unedited.
  **Location**: Phase 6 §3, §5
- 🟡 **Compatibility**: `libc` is already a dependency (`cli/visualiser/server`), and a
  tested `/proc` + `sysctl KERN_PROC_PID` probe with the macOS ABI constants pinned
  already exists at `server.rs:527` — so the plan frames a non-issue as a risk while
  planting a second raw-struct-offset implementation. **Location**: Phase 6 §1
- 🟡 **Architecture**: The daemon becomes launcher-only-spawnable, breaking `test-run.js`
  and `daemon.test.js`, which `fork(RUN_JS, ['daemon', …])` directly — and
  `daemon.test.js` sits in the lane required to pass with zero skips.
  **Location**: Phase 6 §1/§3 vs §6
- 🟡 **Architecture / Code Quality**: `FreeSpace` is a port for behaviour this plan
  explicitly leaves in shell; `run.sh` has no free-space check.
  **Location**: Phase 6 §1
- 🟡 **Architecture**: Thirteen unrelated blocks are re-homed into
  `test-skill-frontmatter-conformance.sh`, a single-purpose by-name gate, on a runtime
  property rather than a subject-matter one. **Location**: Removal sweep §1
- 🟡 **Test Coverage**: The 153/154 table split leaves the sibling's assertion with an
  unbound `$SKILL` (defined at :140, inside the re-homed range) and strips the
  `shellcheck disable=SC2016` directive its literal needs.
  **Location**: Removal sweep §1
- 🟡 **Test Coverage**: Four re-homed grep assertions become self-matching inside the tree
  they scan (`:534-536`'s own comment anticipates "or new test"), and their destination is
  unspecified so the floor of 8 shifts. **Location**: Removal sweep §1, Phase 6 §6
- 🟡 **Test Coverage / Safety**: No automated anchor survives for the ported downgrade
  message text — the goldens are deleted, byte-for-byte is manual only, and the drift test
  needs a JSON file a sweep criterion forbids. **Location**: Phase 2 §3, Removal sweep
- 🟡 **Test Coverage**: `test-run.sh` is excluded from the migration checklist, dropping 20
  assertions over the **retained** `links` implementation — several of them a
  data-exposure contract (no raw `href`, no resolved URL, no echoed query).
  **Location**: Phase 2 criteria, Phase 6 §8
- 🟡 **Test Coverage**: The bare-`return` grep cannot distinguish a test body from a
  helper, and the zero-skip lane cannot exclude the extracted runtime test without
  shifting its own floor. **Location**: Phase 6 §6
- 🟡 **Safety**: Releasing the lock at exit removes today's mutual exclusion on daemon
  existence, converting a refusal into two daemons and two browsers.
  **Location**: Phase 6 §1, §3
- 🟡 **Safety**: Kill-on-timeout's target identity depends on the unresolved
  `setsid`-or-double-fork choice; under a double fork the launcher never learns the
  daemon's pid. **Location**: Phase 6 §1
- 🟡 **Safety**: The mutation gate ("shown to fail when its property is broken") covers the
  structural re-homes but not Phase 2's ~200 lines of SSRF-boundary assertions.
  **Location**: Removal sweep vs Phase 2 §6
- 🟡 **Performance**: The "net is not obvious" position resolves to a clear 20–45ms/call
  win, so the gate is calibrated as a floor rather than a ratio.
  **Location**: Performance Considerations
- 🟡 **Performance**: The latency criterion is not writable as automated verification and
  its baseline (`run.sh`) is deleted by the same phase. **Location**: Phase 6 criteria
- 🟡 **Code Quality**: Rationale is duplicated up to four times, one copy has already gone
  stale within §4 (requires then forbids the `state.js` agreement assertion), and
  "Phase 8 §1" is a dangling reference. **Location**: plan-wide, Phase 6 §4, §6
- 🟡 **Code Quality**: `notify-downgrade-messages.json` is orphaned but kept alive by its
  own drift test, contradicting the sweep's `skills/design/**/scripts/` criterion.
  **Location**: Phase 2 §3 vs Removal sweep
- 🟡 **Code Quality / Architecture**: The three sub-domains are asserted as module
  directories but only one has a path, and Phase 6 contradicts even that one
  (`cli/design/src/executor/` vs `src/runtime/`). **Location**: Phase 2 §1 vs Phase 6 §1
- 🟡 **Code Quality**: `CuePhraseMatcher` has no stated signature, and a flat `const` slice
  loses `audit-cue-phrases.sh:39-69`'s deliberate mixed case policy.
  **Location**: Phase 2 §1
- 🟡 **Correctness**: The recovery no-signal carve-out covers two rows but excludes the
  proven-recycled-pid row, where signalling is provably wrong.
  **Location**: Phase 6 §1
- 🟡 **Correctness / Compatibility**: `validate-source`'s reserved ranges are neither
  internal-reach-recoverable nor numeric encodings, so the documented two-way split cannot
  classify them; and the `localhost`/`127.0.0.1` default-allow carve-out has no home in
  the new model. **Location**: Phase 2 §1, Removal sweep §2

#### Minor

- 🔵 **Correctness**: The daemon inherits `run.sh:3`'s `umask 077`, so screenshots land
  `0600` today and would land at the caller's umask — a fourth removed shell primitive.
- 🔵 **Correctness**: The allowlist also converts `unknown-command` from a daemon-side
  stdout envelope at exit 0 into a launcher-side exit-2 error.
- 🔵 **Correctness**: The truncating-division counterexample is arithmetically identical to
  the required formula; the real hazard is rounding.
- 🔵 **Correctness**: `proc-stat-linux.txt` is a bespoke four-key file, not a raw `/proc`
  sample, and its only consumer is deleted; `ps-lstart-macos.txt` has no disposition.
- 🔵 **Correctness / Test Coverage**: The Phase 3 four-line golden cannot be captured under
  a fixed clock (`inventory-metadata.sh:10-11` calls `date`), has no host suite, and rests
  on a premise contradicted by existing tests at `corpus-adapters/tests/metadata.rs:100-164`.
- 🔵 **Security**: The removed bidi/printable-ASCII filter did cover one live path — the
  echoed invalid `--reason` value, which `SKILL.md:127,132` extracts from bootstrap stderr.
- 🔵 **Security**: `leaked_credentials` has no minimum-length rule, so a short username
  matches every artefact; `LOGIN_URL` is scanned as a secret; screenshots are unscanned.
- 🔵 **Security / Compatibility**: `PROTOCOL.md:18` ("External callers cannot reach it") is
  the false claim the token exists to fix, and is not in the edit set.
- 🔵 **Compatibility**: The exit-code split has no consumer update —
  `analyse-design-gaps/SKILL.md:125-135` still retries three times on any failure.
- 🔵 **Compatibility / Safety**: `ACCELERATOR_LOCK_FORCE_MKDIR` is still honoured by the
  surviving `ensure-playwright.sh:90`, so dropping it leaves it half-honoured.
- 🔵 **Safety**: Dropping the mkdir backend removes the only filesystem-agnostic exclusion,
  against work-item:0196's own Technical Note, and it fails open.
- 🔵 **Safety**: Further `run.sh` remediation strings survive in `client.js:13`,
  `daemon.js:155`, `run.js:33` and six `PROTOCOL.md` lines.
- 🔵 **Safety**: `regenerate-notify-downgrade-fixtures.sh` and its fixtures have no owning
  phase while a sweep criterion forces their removal.
- 🔵 **Safety**: Retiring the preload guard leaves no fail-fast across ~40 agent call sites.
- 🔵 **Test Coverage**: `mise run test:unit:build-system` does not exist (it is
  `test:unit:tasks`), and `test:unit:templates` — the lane Phase 3 breaks — is in no
  criteria.
- 🔵 **Test Coverage**: The opt-in integration lane runs in no CI job, so its own
  fail-not-skip guarantee is unverified.
- 🔵 **Test Coverage**: The migration checklist has no path and no completeness check,
  though the deleted suites' assertion labels make one derivable.
- 🔵 **Test Coverage / Architecture**: Phase 3's "No `.sh` remains in
  `analyse-design-gaps/scripts/`" depends on Phase 2, which the graph says it does not.
- 🔵 **Architecture**: The executor port set is enumerated inconsistently in three places
  (eight, seven, four).
- 🔵 **Architecture**: `design-adapters`' module inventory is incomplete relative to its own
  per-module pup scoping — the matcher and the JSON parser have no module.
- 🔵 **Architecture**: The plan states its own independence but not the sibling's hard
  dependency on it (sibling Phase 7 edits `design-cli`), nor the files both touch.
- 🔵 **Code Quality**: Phase 1's criteria place the `From<FilenameTimestampFormatArg>`
  assertion in `corpus-adapters`, which cannot depend on the binary crate that defines it.
- 🔵 **Code Quality**: `Allowances`' stated rationale ("meaningful only as a pair") is not
  true of the domain, and the type is named after the flag set.
- 🔵 **Code Quality**: The plan asks for derivation comments where a named constant and a
  failing assertion would carry the fact, against the repo's comment convention.
- 🔵 **Performance**: The design sub-binary's own size is re-hashed 100–200× per crawl and
  is disclaimed as sibling-owned when this plan chooses its contents.
- 🔵 **Performance**: The in-process `config` call — the change's largest win — is a
  trailing clause with no dependency edge named in any crate's list.
- 🔵 **Performance**: New crates add four extra release cross-compiles on every merge, on
  the serialised release queue, unbudgeted.

#### Suggestions

- 🔵 **Correctness**: The `ValueEnum` mirror has no exhaustive reverse mapping, so a new
  domain variant stays silently unreachable from the CLI.
- 🔵 **Architecture**: Phase 2 is a large single mergeable unit; registration only forces
  the binary plus *one* bound subcommand to co-land.
- 🔵 **Architecture**: The verdict table conflates an exactly-comparable own probe with an
  inferred legacy one, so ±1s slack applies permanently where only legacy records need it.
- 🔵 **Security**: The `localhost`/`127.0.0.1` default-allow carve-out is unstated and
  untested, and its two possible implementations narrow or widen the accepted set.
- 🔵 **Performance**: The daemon-start poll interval is the one ported parameter left
  unnamed.

### Strengths

- ✅ The `test-design.sh` disposition table was independently verified line-accurate: all
  24 rows land on the correct block boundary, and the table is complete at block level.
- ✅ Every floor derivation that could be checked is correct — 16 discovered `scripts/`
  suites against a floor of 15, ten `lib/*.test.js` → 8, exactly 14
  `skip: !playwrightInstalled` tests in `test-run.js`, and `daemon.test.js:72` as the
  single bare-`return` gate.
- ✅ Recognising that `node --test` reports an early `return` inside a test body as
  *passed* — a genuine detection gap that neither a file-count floor nor a skip count can
  see, and the same shape the plan condemns in `identity.test.js`.
- ✅ The executor port set makes the reuse verdict a deterministic pure function, so cold
  start, warm reuse, stale-PID recovery, PID-recycle rejection, contention and start
  timeout need no sleeps, no real processes and no real elapsed time.
- ✅ Refusing to invert `kernel::Error` to reach the desired exit codes, preserving the
  documented `Refusal → 2` contract every other sub-binary shares.
- ✅ Pre-declaring the `design_adapters::{filesystem,environment,process}` split and its
  scoped no-spawn pup rule *before* Phase 6 needs to spawn, rather than landing a
  crate-wide rule and weakening it later.
- ✅ Adding a dedicated criterion for the *newly*-rejected host encodings, having spotted
  that a shell-derived checklist can only demand tests for behaviour the shell already had.
- ✅ Phase 1 §4's refusal to write a `FakeClock` test on the renderer, with the correct
  reasoning that a fake replaces the very seam under test — and AC15 pointed at the
  existing pure-function pin, which was verified to exist.
- ✅ Freezing the downgrade vocabulary with the cross-phase argument spelled out, so
  graceful degradation survives independent merges.
- ✅ Making the pre-lock reuse check read-only, closing a real race where two concurrent
  launchers could delete a healthy daemon's state outside any lock.
- ✅ The `no-repo` envelope, `server-stopped.json` removal and bootstrap-log
  truncate-and-chmod are carried across explicitly rather than inherited from `umask 077`.
- ✅ Correctly identifying the warm executor path as the only hot path, and demanding a
  per-invocation measurement no prior criterion required.

### Recommended Changes

1. **Resolve the identity contract to one specification, and make it implementable**
   (addresses the two writer criticals plus the `setsid`/double-fork and pid-identity
   majors). Rewrite Phase 6 §3 to match §1, and replace "through the environment" with a
   mechanism that respects `execve` ordering — an inherited pipe the daemon reads before
   its single write, or observation in the forked child before `exec`. Fix the spawn to
   `setsid` only, since a double fork loses the pid both the identity record and the
   timeout kill need.

2. **Add `scripts/test-metadata-helpers.sh` to Phase 3 and correct the floor arithmetic**
   (addresses the metadata-suite critical). Delete the suite (its property already lives in
   `corpus-adapters/tests/metadata.rs:100-147` and `corpus-cli/tests/metadata_goldens.rs`),
   drop the driver from `tasks/test/unit.py:41`, add `mise run test:unit:templates` to the
   criteria, and restate the arithmetic as 15-after-Phase-3 with the sibling's decrement to
   14 recorded as inherited.

3. **Add `SKILL.md` Step 12's `daemon-stop` call site, and guard the class**
   (addresses the third-call-site critical). Repoint `:299` and its `:302` prose, and add an
   automated assertion that no SKILL.md or agent body names a path under
   `skills/design/**/scripts/` that does not exist — so a dangling call site fails CI
   rather than leaking a browser.

4. **Complete the verdict table over its declared types** (addresses the writer-unavailable
   critical and the totality/`NoPidRecorded` majors). Introduce the outer sum type
   (`RecordedState::{None, PidUnparseable, Daemon(..)}`), add a writer-side-unavailable
   variant with an explicit reuse-on-liveness verdict, add a liveness column, and state
   that recovery signals on **no** row.

5. **Reconcile `exec` with `RunClient` and the lock** (addresses the terminal-path major).
   Type `RunClient` as diverging, state that no domain logic may follow it, and specify the
   lock release as an explicit step before `exec` with `Drop` as the non-terminal backstop —
   plus a criterion that the lock is observably free after a successful command.

6. **Close the four security holes that are cheap inside the phases already rewriting the
   code**: make the validated command win over `json-args` (`{...args, protocol, command}`)
   and reject payloads carrying `command`/`protocol`; validate `info.url` as loopback before
   connecting; use `Ipv6Addr::to_ipv4()` so `::a.b.c.d` is unwrapped, and reject `%`-escapes
   and control characters in the authority; narrow `Bash(… corpus *)` to
   `corpus metadata derive *`.

7. **Extend the mutation gate to Phase 2's SSRF assertions** (addresses the coverage-gate
   major). Every Rust test named in the migration checklist for the `validate-source` and
   `scrub-secrets` rows must be shown to fail when its property is broken — the same rule
   the Removal sweep applies to the structural re-homes.

8. **Name the executor's adapter-level harness** (addresses the no-mechanism major). A stub
   `run.js` that prints its stdio, exits with a chosen code, dies on a chosen signal and
   never signals ready exercises `setsid`, redirection, log mode, timeout kill, contention
   and exit-status propagation in CI with no Playwright runtime.

9. **Fix the types that cannot carry their own classifications**: extend `HostReach` with a
   reserved variant (and state which flag recovers each range, plus the `localhost`
   carve-out), and extend or replace `Verdict<Reason>` so it spans the executor's four exit
   codes and its stdout-at-exit-0 asymmetry.

10. **Pin the lockhash and name the path inputs** (addresses two majors). Add the namespace
    derivation to the ported-behaviour inventory with a criterion that the Rust digest
    equals `ensure-playwright.sh`'s for the shipped lockfile, and add a path-resolution
    seam so the two path-bearing envelopes are deterministic and a missing plugin root
    refuses with a named error.

11. **Reuse the existing start-time probe rather than writing a second**
    (addresses the libc major). `cli/visualiser/server/src/server.rs:527` already
    implements `/proc` + `sysctl KERN_PROC_PID` with the macOS ABI constants pinned and
    tested; promote it into a shared crate or state why a second implementation is wanted.

12. **Recalibrate the performance gate** (addresses both performance majors). State the
    expected 20–45ms/call improvement with its derivation, gate on a ratio, and move the
    measurement to a recorded check with the method restated inline and the
    before-measurement ordered ahead of §8's deletion.

13. **Fix the residue** (addresses the duplication major and several minors). One
    authoritative statement per decision; delete §4's `state.js`-agreement sentence;
    repoint "Phase 8 §1" at "Removal sweep §1"; correct `test:unit:build-system` →
    `test:unit:tasks`; move Phase 1's `From` assertion to the `accelerator-corpus`
    criterion; fix the Removal sweep table's 153/154 split; give
    `notify-downgrade-messages.json`, the goldens and
    `regenerate-notify-downgrade-fixtures.sh` an explicit owning phase.

---

## Per-Lens Results

Each finding below carries its severity, confidence, location, title and body as
returned by the reviewing agent, with its suggested remedy.

### Correctness

**Summary**: The plan is unusually rigorous about the launcher's identity contract, and
most of its named corrections (absent `start_time` now a mismatch, read-only pre-lock
check, single lock backend) are genuine logic fixes verified against `run.sh` and
`lib/state.js`. But the centrepiece of Phase 6 — "one observer, one writer, one atomic
write" — specifies a handoff that cannot happen in the order stated, and §3 still describes
the launcher-writes-the-record design that §1 explicitly rejects, so the phase carries two
mutually exclusive mechanisms for its riskiest code. Several state-model claims also do not
hold as written: the reuse verdict table is not total over the declared types, the
writer-side `Unavailable` probe has no recorded variant, the recovery-signalling carve-out
is drawn so the only row that signals is the proven-recycled-pid row, and the "never
partial record" guarantee covers only one of the two files the reader requires.

**Strengths**:

- The four deliberate corrections in Phase 6 §3 are each verified against source:
  `run.sh:54`'s empty-expected accept really does bypass the PID-recycle guard,
  `run.sh:106-121`'s unconditional `rm -f` really does delete a live daemon's state outside
  the lock (`state.js:63-66` publishes the two files as separate renames), and
  `run.js:18`'s `args[0] === 'daemon'` dispatch really is reachable through verbatim
  forwarding.
- Modelling the reuse decision as a pure function over injected `Clock`/`ProcessProbe`/
  `StateStore` ports, with `ObservedStartTime::Unavailable` as a domain value rather than an
  adapter panic, is the right shape for making PID-recycle and start-timeout cases
  deterministically testable without real processes or real elapsed time.
- The truncating-integer-division requirement is correct and correctly evidenced:
  `run.sh:25`'s `$((btime + ticks/hz))` and `state.js:49`'s `Math.floor` agree, and the
  cited fixture arithmetic checks out (field 20 = 14562000, hz 100, btime 1700000000 →
  1700145620). Asking for a second fixture whose tick count does not divide evenly is
  exactly the missing case.
- Rejecting `ps -p <pid> -o lstart=` for Darwin on DST-ambiguity and offset-indeterminacy
  grounds is sound reasoning, and reading an epoch-based kernel value removes the locale
  hazard by construction rather than by convention.
- Replacing `classify_internal`'s regex set with strict `IpAddr` parsing plus explicit
  unwrapping of IPv4-embedding transition encodings closes real gaps
  (`::ffff:10.0.0.1`, `fc00::/7`, non-first-octet octal) rather than transcribing them.

**Findings**:

🔴 **critical** (high) — *The spawn-time identity handoff is impossible in the stated order*
— **Phase 6 §1 ("One observer, one writer, one atomic write")**

Phase 6 §1 states the launcher "knows the pid at fork and observes the start time with the
same probe" and then "hands `(pid, start_time, start_time_source, token)` to the daemon
through the environment it already receives alongside `ACCELERATOR_PLAYWRIGHT_STATE_DIR`".
On Unix the child's environment block is fixed when `execve` is called, which necessarily
precedes the child existing — so the pid and its start time are not knowable at the moment
the environment is constructed. `std::process::Command` builds `envp` before `fork`, and a
`setenv` inside `pre_exec` is not reflected in the `envp` handed to `exec`, so no variant of
the standard spawn API can carry these two values. Only the token (known pre-spawn) can
travel this way.

*Impact*: The load-bearing mechanism of the phase's most regression-prone code has no
implementable form, so an implementer will improvise — most likely reverting to the
launcher-writes-the-record design §1 rejects, reintroducing the crash window in which a live
daemon has a start-time-less record that the verdict table turns into "delete the state and
spawn a second daemon".

*Suggestion*: Name a mechanism that respects the ordering: either have the launcher pass the
values over an inherited pipe (or a pre-opened fd) that the daemon reads *before* its single
`atomicWrite`, or observe the start time in the forked child before `exec` (pid and start
time are both preserved across `exec`, and it is still one Rust probe implementation) and
pass them via a manually constructed `execve` environment. Either way, state the readiness
ordering explicitly: the daemon must not publish `server-info.json` until it holds the
identity values.

🔴 **critical** (high) — *§3 still specifies the record-writing design §1 rejects* —
**Phase 6 §3 ("`state.js` stops computing the start time") vs Phase 6 §1**

Phase 6 §1 rejects having the launcher write the identity record — "An earlier draft had the
launcher *write* the record itself 'once the daemon reports ready', which opens a window …
It also gave one file two whole-file-rename writers, which is a lost-update contract" — and
specifies instead that `state.js` publishes `pid`, `start_time`, `start_time_source` and
`token` in its single existing `atomicWrite`. Phase 6 §3 then says the launcher "observes the
start time itself with the probe it will later use to check it, **and writes the identity
record once the daemon reports ready**", and that "`state.js` writes the port and readiness
facts it owns"; the token paragraph likewise says the launcher "records it in the
already-`0700` `server-info.json`".

*Impact*: The two sections specify mutually exclusive writers for the same file. Implementing
§3 produces exactly the two-writer lost-update contract and the partial-record crash window
§1 was written to eliminate, and makes the token unenforceable until after the daemon is
already accepting connections.

*Suggestion*: Rewrite §3's paragraph and its token paragraph to match §1's decision (daemon
is the sole writer; launcher supplies values at spawn), and delete the "writes the identity
record once the daemon reports ready" clause wherever it survives.

🟡 **major** (high) — *The signalling carve-out excludes the one row where signalling is
provably wrong* — **Phase 6 §1 ("Recovery never signals a pid it cannot identify")**

The plan carves out two `stale → recover` rows (`AbsentOrUnparseable`, `NoPidRecorded`) as
"recovery removes the state files and respawns *without* signalling anything", justified by
"Signalling the recorded pid there would mean delivering SIGTERM to whatever process now owns
a recycled pid". By omission this implies the remaining `stale → recover` rows do signal — and
the only such row with a live process is `Probe(r)` + `Live(Known(o))` with `|r-o| > 1`, i.e.
the case where the start-time mismatch *proves* the pid has been recycled and is not the
daemon. `run.sh` never signals during recovery at all (`:120`, `:156` delete state files; the
only `kill -TERM` is `:191`, aimed at the launcher's own just-spawned child).

*Impact*: As stated, the rule sends SIGTERM to an arbitrary unrelated process — on a
developer machine an editor or a build — in exactly the situation the carve-out's own
rationale forbids, and it is a new behaviour the port would introduce.

*Suggestion*: State that recovery never signals on any row, and that `ProcessControl` is used
only for the kill-on-timeout of the launcher's own spawned child (`run.sh:191`); add a test
asserting no signal is delivered on every recover row.

🟡 **major** (high) — *The verdict table is not total over the declared types* —
**Phase 6 §1 (reuse verdict table)**

The input is declared as `(RecordedDaemon, ObservedDaemon)` with `RecordedDaemon { pid,
start_time: RecordedStartTime }` — a struct — yet the table's Recorded column contains a
`NoPidRecorded` row that the declared type cannot represent. There is also no representation
for "no record at all" (`run.sh:106` requires *both* `server-info.json` and `server.pid` to be
present), nor for the distinct case of a record whose pid is present but unparseable
(`run.sh:107`'s `tr -cd '0-9'` yielding empty), which the single `NoPidRecorded` label
silently conflates with the absent-file case.

*Impact*: The claim that the match is "total by construction rather than by enumeration" is
false as typed, and the success criterion "a test per row" cannot be satisfied for a row with
no corresponding value — so the absent-state and unparseable-pid paths (the state an
interrupted daemon actually leaves) go unmodelled and unexercised.

*Suggestion*: Declare the top level as an enum — e.g. `RecordedState::{ None, PidUnparseable,
Daemon(RecordedDaemon) }` — and state which file each field is read from, so `StateStore`'s
return type makes every table row inhabitable.

🟡 **major** (high) — *A writer-side `Unavailable` probe has no recorded variant, and mapping
it to `AbsentOrUnparseable` respawns on every command* — **Phase 6 §1 and §3
(`start_time_source` on the writing side)**

§3 says "`start_time_source` survives as a field the Rust writer sets, since a probe that
returns `Unavailable` on the *writing* side still has to be recorded as such for the verdict
table above to read it" — but `RecordedStartTime` has only `Probe(u64)`, `Wallclock(u64)` and
`AbsentOrUnparseable`, none of which is "probe unavailable", and §3 also removes
`state.js:60`'s `Math.floor(Date.now()/1000)` fallback without giving Rust one. So when the
launcher's probe returns `Unavailable` (unreadable `/proc`, `hidepid`, distroless), the plan
does not say what `start_time` is written.

*Impact*: If it becomes `AbsentOrUnparseable`, the table's own rule recovers on every
subsequent invocation — respawning the daemon per command and losing page state in precisely
the containers the `Probe`+`Live(Unavailable)` reuse row was added to protect. If it becomes a
wallclock value, that must be stated, because it silently disables the PID-recycle guard.

*Suggestion*: Give `RecordedStartTime` a variant for a writer-side unavailable probe (or state
that Rust falls back to wallclock and records `start_time_source: wallclock`), and add the
corresponding verdict row plus its test.

🟡 **major** (medium) — *The legacy-record rationale ignores `state.js`'s wallclock fallback* —
**Phase 6 §1 ("A record with no `start_time_source` key … is read as `Probe`")**

The plan reads a record with no `start_time_source` key as `Probe`, justified by "`ps lstart`
is itself derived from `p_starttime` and agrees to the second". That covers only the
successful-probe branch of today's writer. `state.js:40-61` returns
`Math.floor(Date.now()/1000)` on *any* failure — non-linux/darwin platforms, unreadable
`/proc`, a failing `execSync('getconf CLK_TCK')`, or an unparseable `ps` string — and that
value is taken in the `server.listen` callback, i.e. after module loading, which is the >1s
drift the ±1s tolerance exists to absorb under load.

*Impact*: A legacy record written through the fallback is read as a kernel probe value and
held to ±1s, so a healthy daemon that survived a plugin upgrade can be judged stale, its
state deleted and a second daemon spawned mid-crawl — and per the signalling rule above,
possibly SIGTERM'd.

*Suggestion*: Either treat a missing `start_time_source` as `Wallclock` (reuse on liveness, no
recycle guard) — the conservative reading, since the writer's provenance is genuinely unknown
— or state the accepted one-off respawn for pre-upgrade daemons explicitly and pin it with a
test.

🟡 **major** (high) — *The record the reader requires spans two non-atomically published
files* — **Phase 6 §1 ("One observer, one writer, one atomic write") / `StateStore`**

"The record is never partial" and "one atomic write" refer to `server-info.json`, but
`writeServerInfo` (`lib/state.js:63-66`) performs **two** independent atomic renames —
`server-info.json` then `server.pid` — and the reader requires both (`run.sh:106`, and the
spawn poll at `:181-184`). A reader landing between the two renames sees a live daemon with an
incomplete record; under the lock, `run.sh:156` (and the port) then delete a healthy daemon's
state and spawn a second one. The plan does not say which file `StateStore` reads the pid
from, even though `server-info.json` already carries `pid`.

*Impact*: The stated no-partial-record invariant does not hold for the record actually
consumed, leaving the orphan-a-live-daemon window the read-only pre-lock fix was meant to
close (narrower, but under the lock where the deletion is unconditional).

*Suggestion*: Have `StateStore` read pid *and* start time from `server-info.json` alone, so the
identity record is genuinely one atomic rename; keep `server.pid` as a compatibility artefact
only, and state whether its presence is still part of the readiness condition.

🟡 **major** (high) — *The `Verdict` carrier cannot express the executor's exit codes* —
**Phase 2 §3 (`Verdict` carrier) vs Phase 6 §2 (exit codes 0,1,2,3)**

Phase 2 §3 introduces `Verdict<Reason> { Accepted { stdout }, Rejected { reason, stderr } }`
with `main` matching on it for "one render-and-exit function", mapping Accepted→0, Rejected→1
and `kernel::Error::Refusal`→2. Phase 6's executor must produce **four** observable codes with
the semantics preserved: 0 (including daemon-side error envelopes on stdout), 1
(`another-launcher-running`, `daemon-start-timeout` — both `category` values that are not
"usage" in the exit-2 sense; `another-launcher-running` is `category:"usage"` at exit **1**),
2 (`no-repo`), 3 (`playwright-not-installed`), plus arbitrary pass-through of the client's
exit status and signal death.

*Impact*: The two-variant carrier and the exit-2-means-usage rule are contradicted by the
executor within the same binary, so either the carrier is silently extended mid-implementation
or the executor bypasses it — and "one render-and-exit function" stops being true, which is
where a wrong code would go unnoticed.

*Suggestion*: Extend the carrier (or name a second executor-specific outcome type) covering
exit 3 and the exec pass-through, and add a table in Phase 6 §2 mapping each executor envelope
to its exit code so the `category` field's divergence from the exit code is recorded rather
than inferred.

🟡 **major** (high) — *A port whose production implementation never returns makes post-call
logic dead in production only* — **Phase 6 §1 (the `RunClient` port)**

`RunClient` is specified as "run the client and return an exit outcome", with "The adapter's
terminal implementation is `CommandExt::exec`". `exec` never returns on success (its return
type is `io::Error`), so any domain code sequenced *after* the `RunClient` call — lock release,
envelope rendering, exit-code mapping — executes in tests (where a fake returns) and never
executes in production.

*Impact*: This is the failure mode where every characterization test passes and production
diverges: a cleanup step placed after the call (for instance an explicit lock release, which
`run.sh:152,202` performs before `exec`) would be silently skipped on the real path.

*Suggestion*: Type the port so the divergence is visible — e.g. `fn exec_client(...) ->
kernel::Error` / `-> Result<Infallible, _>` — or have the domain *return* the client invocation
as the final action for the command layer to perform, so nothing can be sequenced after it.

🟡 **major** (high) — *Nothing pins the Rust lockhash to the surviving `ensure-playwright.sh`'s*
— **Phase 6 §1 ("still resolves the existing lockhash namespace")**

The namespace is `${CACHE_ROOT}/$(sha256(package-lock.json) | cut -c1-8)` (`run.sh:86-92`), and
`ensure-playwright.sh:50-60` — which **survives this plan** — computes it the same way and is
the only thing that populates it. The port removes `sha256sum`/`shasum`, so Rust recomputes the
digest, but no success criterion asserts the Rust value equals the shell value (lowercase hex,
first 8 characters, same file, no trailing-newline or path-resolution difference).

*Impact*: An off-by-anything — uppercase hex, 8 bytes instead of 8 hex chars, hashing a
different copy of `package-lock.json` — makes the executor look in a namespace the surviving
bootstrap never fills, so every invocation returns `playwright-not-installed` at exit 3 on
machines that are correctly bootstrapped, and the skill silently downgrades to `code` mode.

*Suggestion*: Add an automated criterion asserting the Rust lockhash equals `sha256_of`
`ensure-playwright.sh` for the shipped `package-lock.json` (a fixture plus a cross-check against
the shell function while it still exists), and pin the digest of the shipped lockfile as a
golden.

🟡 **major** (medium) — *The executor's path inputs have no stated source and no seam* —
**Phase 6 §1 (state dir, namespace and `run.js` location)**

`run.sh` derives `SCRIPT_DIR`/`PLUGIN_ROOT` from `BASH_SOURCE` (`:5-6`) and therefore always
finds `run.js`, `package-lock.json` and the plugin root. A dispatched sub-binary executes from
the launcher's cache directory, so it cannot; in this workspace the only mechanism is
`ACCELERATOR_PLUGIN_ROOT` (`config-adapters/src/store.rs:204`, error text at
`cli/config/src/error.rs:131-136`), which the plan never names. The listed ports (`Clock`,
`ProcessProbe`, `StateStore`, `Lock`, `Spawner`, `ProcessControl`, `RunClient`, `FreeSpace`)
contain no seam for repo-root discovery, the `config path tmp` value, the namespace root or the
script directory.

*Impact*: Two envelopes embed absolute paths (`playwright-not-installed` names `$NS_ROOT`;
`daemon-start-timeout` names `$BOOTSTRAP_LOG`), so the "byte-identical 3-key JSON" criterion and
AC2's "volatile inputs supplied through injected ports" are unreachable with the stated port
set, and the cold-start/timeout characterization tests need a real repository and a real HOME. It
also introduces an unremarked failure mode: an executor invoked with `ACCELERATOR_PLUGIN_ROOT`
unset (e.g. via an `ACCELERATOR_DESIGN_BIN` override) cannot locate `run.js` at all.

*Suggestion*: Name the plugin-root source and add a path-resolution port (repo root + tmp path +
namespace root + script dir) so the two path-bearing envelopes are deterministic by
construction; state the refusal when the plugin root is unknown.

🟡 **major** (high) — *The double-fork alternative loses the daemon pid* — **Phase 6 §1 (daemon
spawn, "`setsid` (or a double fork)")**

`run.sh:169-173` spawns the daemon as a direct child, so `$!` *is* the daemon's pid — which
`:191` uses for kill-on-timeout and which §1 needs for the identity handoff. `setsid` in the
child before `exec` preserves that (same pid). A **double fork** does not: the launcher's direct
child forks the daemon and exits, so the launcher never learns the grandchild's pid.

*Impact*: Offering the two as interchangeable means an implementer may pick the one that
silently breaks both the kill-on-timeout (`run.sh:191`, which exists to stop a half-bootstrapped
daemon being reused on a page that never received the command) and the pid half of the identity
record.

*Suggestion*: Specify `setsid` in `pre_exec` on the single spawned child (falling back to
reporting an error rather than double-forking), and add a criterion that the pid the launcher
records is the pid that appears in `server-info.json`.

🟡 **major** (medium) — *Reuse-on-liveness has no escape hatch when the reused daemon is
unreachable* — **Phase 6 §1 (`Probe(_)` + `Live(Unavailable)` → "reuse on liveness alone")**

This row is a deliberate behaviour change (`run.sh:55` answers an unavailable probe with a
mismatch) but is not listed among §3's "Four" deliberate corrections. More importantly, once the
launcher reuses on liveness alone, a recycled pid that happens to be live sends the launcher
down the client path, where `client.js:44-48` returns a `connection-failed` envelope on
**stdout at exit 0** — and nothing deletes the state files, so the next invocation repeats the
same verdict.

*Impact*: In a `/proc`-less container (the case the row exists for), a daemon that died and had
its pid recycled yields `connection-failed` on every subsequent command with no recovery path,
where today the mismatch would respawn. The plan records the lost recycle guard for the
`Wallclock` row but not this consequence for either.

*Suggestion*: List this row among §3's deliberate changes, and specify a recovery trigger on an
unreachable-but-recorded daemon (treat `connection-failed`/`no-daemon` from a reused record as
stale state and recover once), with a test.

🟡 **major** (medium) — *The always-allowed localhost default has no home in the new
reachability model* — **Phase 2 §1 (`host_reach.rs` / `access_policy.rs`)**

`validate-source.sh` checks `is_localhost_default` (`:79-84`, applied at `:277-279`) *before*
internal classification, so `localhost` and `127.0.0.1` are accepted on **http** with no flags —
the skill's primary documented invocation (`:16`, `http://localhost:3000`). The plan's model has
`HostReach` = "loopback, private, link-local, unspecified, or public" and an `Allowances
{ internal, insecure_scheme }` verdict, with no localhost-default carve-out named; a principled
`is_loopback` reading makes `127.0.0.1` internal (requiring `--allow-internal`) and a bare
`localhost` hostname "public" (requiring `--allow-insecure-scheme` on http). The five listed
variants also cannot express the reserved set the same section demands (`100.64.0.0/10`,
`192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, `fc00::/7`), which the sweep's documentation note
nonetheless describes as `--allow-internal`-recoverable.

*Impact*: The most common accepted input would start being rejected, and the classification
vocabulary that appears in user-facing stderr (`"host X is a $classification address"`) has no
variant for the ranges the plan adds.

*Suggestion*: Name the localhost/`127.0.0.1` always-allowed rule as a preserved behaviour with
its own test (and decide explicitly whether `::1` joins it), and add a `Reserved` reach variant
with its `--allow-internal` recoverability stated per range.

🔵 **minor** (high) — *§4 both requires and forbids the cross-language agreement assertion* —
**Phase 6 §4 (the locale regression guard)**

§4's first paragraph says the Rust guard "additionally asserts agreement with the value
`lib/state.js` writes for the same process"; its last paragraph says "What the guard no longer
does is compare against `lib/state.js`. §1 removes the JS probe entirely, so there is one
implementation and nothing to agree with" — and the success criteria agree with the second
reading.

*Impact*: A stale sentence describes a test that cannot be written once `processStartSeconds` is
deleted, so the phase's acceptance is ambiguous at exactly the point ADR-0058 names as the port's
principal silent-regression risk.

*Suggestion*: Delete the first paragraph's cross-language clause. Also note that varying `TZ`
cannot exercise "a DST fall-back boundary" for a live process's start time — the assertion can
only show TZ-independence, which is worth saying plainly.

🔵 **minor** (medium) — *The daemon's inherited `umask 077` is a fourth removed shell primitive*
— **Phase 6 §1 ("Three behaviours an earlier draft's inventory missed")**

The inventory names three properties supplied by shell primitives the port removes
(`nohup`/`disown`, stdio redirection, `exec` status propagation) and handles the bootstrap log's
mode explicitly "under a `umask 077` the Rust binary does not inherit". But `run.sh:3`'s
`umask 077` is also inherited by the *daemon*, and therefore by every file the daemon creates
without an explicit mode — notably `page.screenshot()` output (`daemon.js:221-233`), which today
lands `0600` and would land at the caller's umask instead.

*Impact*: Screenshots of authenticated pages silently become group/world-readable, a behaviour
change in a port that claims to derive its inventory line-by-line from `run.sh`.

*Suggestion*: Set `umask(0o077)` in the spawned child (`pre_exec`) as well as for the launcher's
own writes, and add it to the inventory with a criterion on a daemon-written screenshot's mode.

🔵 **minor** (medium) — *The allowlist also changes the outcome for every unknown command* —
**Phase 6 §3 ("Internal `run.js` subcommands are rejected by argument validation")**

Rejecting `daemon` is right, but an *allowlist* of forwardable commands also intercepts every
command the daemon would have answered with `unknown-command` (`daemon.js:279-281`) — today a
daemon-side envelope on **stdout at exit 0**, which §2 commits to preserving as the
launcher-vs-daemon asymmetry. Under an allowlist it becomes a launcher-side exit-2 stderr error.
The allowlist must also stay in sync with `daemon.js`'s eleven commands, and the sync assertion
the plan extends lives in the node suites, which cannot see the Rust source.

*Impact*: A consumer (both browser agents, ~40 call sites) that parses stdout JSON gets no
envelope for a typo'd or newly added command, and a command added to `daemon.js` is silently
unreachable until a Rust release.

*Suggestion*: Either reject only the internal `daemon` token (a denylist, which is all §1's
hazard requires), or keep the allowlist and pin it against `daemon.js`'s command set with a
cross-language test, listing the `unknown-command` outcome change in §3.

🔵 **minor** (high) — *The usage-vs-verdict rule is under-determined for a nonexistent path* —
**Phase 2 §3 ("The rule, not just the examples")**

The rule is "a malformed *invocation* — … an argument the tool cannot interpret at all" is usage
(exit 2); "anything the tool successfully evaluated and then rejected" is a verdict (exit 1). It
then assigns `scrub-secrets /nonexistent` → 2 and `validate-source <existing non-directory>` → 1.
It does not settle `validate-source <nonexistent path>`, where `validate-source.sh:223-226`
returns exit 1 for the identical condition ("does not exist or is not a directory", one branch,
one message) that makes `scrub-secrets` exit 2.

*Impact*: The same predicate maps to different codes in the same binary with no stated principle,
so the case is resolved arbitrarily at implementation time — and the migration checklist will
simultaneously demand preservation of `:223-226`'s exit 1.

*Suggestion*: State the discriminator explicitly (existence is *part of the verdict* for
`validate-source`, because evaluating the location is the subcommand's job, whereas
`scrub-secrets` presupposes a readable file) and pin both cases in the manual criteria.

🔵 **minor** (high) — *The bash-side golden cannot be captured under a fixed clock* — **Phase 3
§2 (the four-line characterization golden)**

The phase requires "the four-line output for a fixed clock and repo fixture is recorded as a
golden first", then asserts `corpus metadata derive --filename-timestamp-format compact-time`
"reproduce[s] its labels and ordering". `inventory-metadata.sh:10-11` calls `date` directly, so
its clock cannot be fixed; and its line *count* varies (`:36-37` drop the revision and name lines
conditionally, and outside a repository the script exits 1 because the final `[ -n … ] && echo`
is its last command).

*Impact*: As written the golden is either flaky (two invocations differ in both timestamp values)
or vacuous (everything volatile normalised away), which undercuts its stated purpose of being the
only automated evidence for the label/order equivalence Phase 1 does not cover.

*Suggestion*: Say precisely what is compared — line count, label set, label order, and a shape
assertion per value (`^\d{4}-\d{2}-\d{2}-\d{6}$` etc.) — with the timestamp *values* covered by
Phase 1's pure-function test, and record the non-zero-exit-outside-a-repo case as part of the
golden rather than as a divergence bullet only.

🔵 **minor** (high) — *`FreeSpace` and `disk-floor-not-met` have no consumer in this plan* —
**Phase 6 §1 (the `FreeSpace` port)**

The executor's port list includes "`FreeSpace` — bytes available at a path, so
`disk-floor-not-met` is a unit test over an injected value plus a zero-request assertion".
`run.sh` performs no free-space check anywhere in its 203 lines; `disk-floor-not-met` is emitted
by `ensure-playwright.sh` (which this plan explicitly retains) and appears here only as one of
the six downgrade *reason strings* `notify-downgrade` prints.

*Impact*: A port with no behaviour behind it invites an implementer to add a disk check the
launcher never had — new behaviour inside a plan that describes itself as a pure
characterization-testable port — or to write a test that asserts nothing.

*Suggestion*: Drop `FreeSpace` from Phase 6's port list (it belongs with the sibling plan's
fetch-verify-cache work), or state the launcher-side condition it gates.

🔵 **minor** (high) — *The counterexample formula is arithmetically identical to the required one*
— **Phase 6 §1 (start-time identity, truncating division)**

The rationale warns that "floating-point or `(btime * hz + ticks) / hz` differs by up to a
second". With integer division, `(btime*hz + ticks)/hz == btime + ticks/hz` exactly, because
`btime*hz` is divisible by `hz`; in `f64` the same identity holds at these magnitudes
(`btime*hz ≈ 1.7e11`). The formula that actually differs is one that **rounds** rather than
truncates, or one that computes `btime + ticks/hz` in floating point and then rounds.

*Impact*: The stated hazard is not the real one, so a reviewer or implementer checking the rule
against the cited counterexample gets a false confirmation and may not guard the case that does
differ.

*Suggestion*: Restate the requirement as "truncate, never round" (matching `run.sh:25`'s shell
arithmetic and `state.js:49`'s `Math.floor`) and keep the uneven-tick fixture as the test that
distinguishes them.

🔵 **minor** (medium) — *The pinned fixture is not a raw /proc sample and its only consumer is
deleted* — **Phase 6 §1 (fixture pinning) and §5 (dead JS removal)**

`lib/__fixtures__/proc-stat-linux.txt` is a bespoke four-key file (`stat:`, `btime:`, `hz:`,
`expected_start_time:`), not a raw `/proc/<pid>/stat` or `/proc/stat` sample, and its only reader
is `identity.test.js`, which §5 deletes. A Rust parser fed the `stat:` line verbatim would have
to strip the key prefix, and `ps-lstart-macos.txt` becomes dead once Darwin reads `p_starttime`.

*Impact*: "Fixture-pinned against `lib/__fixtures__/proc-stat-linux.txt` → `1700145620`" is not
directly implementable, and two fixtures are left with no owner in a plan that otherwise tracks
every deletion.

*Suggestion*: State whether the fixture moves into `cli/design-adapters/tests/` (reshaped into two
raw samples plus an expected value) or is read in place with a documented prefix-stripping reader,
and list `ps-lstart-macos.txt` for deletion in §5.

🔵 **suggestion** (medium) — *The ValueEnum mirror is not checked against the domain enum* —
**Phase 1 §1 (the CLI-local value enum)**

`FilenameTimestampFormatArg` mirrors two of `FilenameTimestampFormat`'s three variants
(`DateOnly` is deliberately unexposed, `cli/corpus/src/metadata.rs:5-11`). With only a
`From<Arg> for Domain` impl, adding a fourth domain variant compiles cleanly and stays unreachable
from the CLI — the same silent-drift class Phase 2 §3 argues against for the downgrade message
table, where it insists on compile-time exhaustiveness. Separately, `main.rs:77` and `:82` must
receive the *same* value, or the rendered label (`filename_label`) can disagree with the rendered
timestamp.

*Impact*: Small, but it is a drift the plan's own stated principle would catch.

*Suggestion*: Add an exhaustive reverse mapping (`From<FilenameTimestampFormat> for Option<Arg>`)
so a new domain variant forces a decision, and thread a single `format` binding through both
`derive_at` and `run_derive`.

### Architecture

**Summary**: The plan is architecturally serious and mostly well-shaped: it applies ADR-0053's
hexagon to the riskiest code (the launcher port), models the reuse verdict as a total function
over sum types, honestly names the `Verdict` carrier as a new shape rather than claimed reuse,
and declares its pup-rule scoping at introduction with the future spawn module anticipated. The
weaknesses cluster in two places. First, the executor's port set is under-specified and partly
unmotivated: `FreeSpace` serves no behaviour this plan ports, the new request token has no port,
no field in `RecordedDaemon` and no warm-path flow, the cache-namespace derivation is assigned to
no module at all, and the `Drop`-guard lock release is not implementable on an `exec`-terminated
path. Second, two residual internal contradictions survive on the daemon-identity contract, and
that contract tightens the launcher↔daemon coupling in a way that breaks the retained Node suites
which fork the daemon directly — the suites the plan simultaneously requires to pass with zero
skips.

**Strengths**:

- Phase 6 §1 puts the launcher's reuse decision, tolerance comparison, lock policy and poll
  deadline in the domain crate behind ports rather than in the command layer, and argues the
  position explicitly against ADR-0053 and AC2 rather than asserting it — the riskiest 203 lines
  land in the most testable place.
- The reuse verdict is modelled as `(RecordedDaemon, ObservedDaemon)` sum types so the match is
  total by construction, and the two cases `run.sh` never named (`/proc` unreadable on the reading
  side, an absent/unparseable recorded start time) become domain values rather than adapter panics
  or silent accepts.
- The `Verdict<Reason>` carrier is introduced as a new shape with the reason stated (corpus-cli's
  `Outcome` cannot express a successful-but-non-zero exit) instead of being described as
  "corpus-cli-style", and `kernel::Error::Refusal` keeps its documented meaning rather than being
  inverted to reach the desired exit codes.
- Canonical data stays as domain `const`s with an `include_str!` drift test, following the existing
  `corpus/src/frontmatter_validation/schema.rs:277` precedent — exhaustiveness becomes a compile
  error and the shared-file contract becomes an executable assertion.
- The `design-adapters` pup rule is scoped per-module (`filesystem`/`environment` no-spawn,
  `process` spawns by design) at the moment it lands rather than crate-wide-then-weakened,
  mirroring `vcs_adapters_library_reads_in_process` and avoiding a rule a future maintainer would
  read as unmeant.
- Reachability classification is deliberately re-derived rather than transcribed, with the
  pre-resolution and initial-location-only limits recorded as taken positions in module docs and
  the unclassified `navigate` surface raised as a follow-up rather than left implied as covered.
- The three-crate split matches the corpus/vcs/work triple, and the plan correctly identifies the
  non-obvious consequence — a domain crate owning `cli/design/` forces a `_SUBBINARY_MANIFESTS`
  entry because cargo-pup rules match whole crate names.

**Findings**:

🟡 **major** (high) — *A `Drop` guard cannot release the lock on the `exec` path, and a
`RunClient` port that returns cannot be implemented by `exec`* — **Phase 6 §1 (Lock) and §3**

Phase 6 §1 states "**The lock is released at launcher exit**, by a `Drop` guard, on every path",
while the same section makes the `RunClient` adapter's "terminal implementation …
`CommandExt::exec`". `exec` replaces the process image: no destructor runs, so the `Drop` guard is
dead code on the only path a successful invocation takes. `run.sh` handles exactly this by
releasing explicitly before `exec` (`run.sh:152,202` — lines the plan itself quotes). The mismatch
also runs the other way: a port declared to return an exit outcome, implemented by a call that
never returns, means any domain code after the `RunClient` call is exercised by fakes in tests and
unreachable in production — the silent-regression class ADR-0058 names as the port's principal
risk.

*Impact*: The plan's stated resolution of the previous draft's lock contradiction reintroduces
one; whether the lock is actually released on the hot path depends on an unstated implementation
accident (Rust's `O_CLOEXEC` default closing the `flock` FD at `exec`), and any release step
needing more than an FD close — unlinking a lock file, clearing a sentinel — would silently never
run.

*Suggestion*: State the release as an explicit step in the composition root immediately before the
terminal `exec`, with the `Drop` guard as the non-terminal-path backstop only, and either type
`RunClient` as diverging (`-> !`/`Result<Infallible, _>`) or state that no domain logic may follow
the call so the test-only reachability is deliberate.

🟡 **major** (high) — *§3 restates the daemon-identity write ordering that §1 explicitly rejected*
— **Phase 6 §3 vs Phase 6 §1**

Phase 6 §1 rejects the earlier draft in which "the launcher *write*[s] the record itself 'once the
daemon reports ready'", on the grounds that readiness **is** `server-info.json` appearing, so a
launcher killed in that window leaves a live daemon with no recorded start time — which §3's own
`AbsentOrUnparseable → recover` rule turns into orphaning a healthy daemon. Phase 6 §3 then says,
verbatim, that the launcher "observes the start time itself with the probe it will later use to
check it, **and writes the identity record once the daemon reports ready**". The two sections
specify opposite writers for the same file, and §3 additionally calls `start_time_source` "a field
the Rust writer sets" while §1 has `state.js` publishing it. The success criteria side with §1.

*Impact*: The daemon-state contract is the plan's single most regression-prone interface and it now
has two incompatible specifications in the same phase — precisely the defect class the plan says
three review passes kept finding in this material.

*Suggestion*: Rewrite §3's paragraph to match §1 — the launcher probes and hands
`(pid, start_time, start_time_source, token)` to the daemon at spawn; `state.js` is the sole writer
— and remove the "the Rust writer sets" phrasing.

🟡 **major** (high) — *The daemon becomes launcher-only-spawnable, breaking the retained suites that
fork it directly* — **Phase 6 §1 and §3 (daemon identity, request token) vs §6 (Node suite runner)**

Phase 6 makes the launcher the sole source of the daemon's identity (`state.js` computes no start
time; the values arrive by environment at spawn) and adds a token the daemon must require "from its
first accepted connection". But two **retained** suites spawn the daemon without a launcher:
`test-run.js:68` and `lib/daemon.test.js:77,99,120,141,174` both
`fork(RUN_JS, ['daemon', '--state-dir', dir])`. After this change those daemons write a record with
no start time (which the plan's own verdict table reads as `AbsentOrUnparseable → recover`) and
refuse their own test clients' unauthenticated connections. `daemon.test.js` sits in the
`test:unit:design-automation` lane that Phase 6 §6 requires to pass with **zero skipped tests**.

*Impact*: The launcher↔daemon coupling tightens from "shared files" to "mandatory environment
contract" with no defined behaviour for a daemon spawned any other way, so the retained JS the plan
promises to leave alone breaks, the new unit lane cannot go green, and manual diagnosis by running
the daemon by hand stops working.

*Suggestion*: Name the daemon's spawn contract explicitly (which env vars are required, and what a
daemon does when they are absent — refuse to start, or self-probe as a documented fallback), and add
`test-run.js` and `lib/daemon.test.js` to §3's and §6's file lists as suites that must supply the
identity env and token.

🟡 **major** (high) — *The request token has no port, no place in the `StateStore` contract, and no
warm-path flow* — **Phase 6 §1 (injected ports) and §3 (The daemon gains a request token)**

Phase 6 §3 adds a randomly generated request token that the launcher creates, `state.js` publishes
in `server-info.json`, `client.js` sends and `daemon.js` requires. None of that appears in the port
set: `StateStore` is specified to return `RecordedDaemon { pid, start_time: RecordedStartTime }` with
no token field, yet on the warm-reuse path the launcher does not spawn and must therefore *read* the
token from state and hand it to `RunClient`. Randomness is also a new volatile input with no injected
port, while AC2 requires "the volatile inputs … supplied through injected ports so the output is
deterministic by construction rather than by normalisation" — the plan enumerates `Clock`,
`ProcessProbe`, `FreeSpace` and others for exactly that reason but omits a token source.

*Impact*: Two of the plan's declared port contracts are incomplete for a behaviour it commits to
shipping, the token cannot be tested deterministically, and reuse of a pre-upgrade live daemon
silently yields a session with no token enforcement at all.

*Suggestion*: Extend `RecordedDaemon` with the token (as a domain value type, absent-tolerant for
pre-upgrade records), add a `TokenSource` port beside `Clock`, state how the token reaches
`RunClient`, and state the reuse-of-untokened-daemon behaviour explicitly.

🟡 **major** (high) — *`FreeSpace` is a port for a behaviour this plan does not port, at a boundary
the sibling plan puts elsewhere* — **Phase 6 §1 (injected ports — `FreeSpace`)**

The executor's port list includes `FreeSpace` "so `disk-floor-not-met` is a unit test over an
injected value". But `run.sh` performs no free-space check — the 500MB floor lives entirely in
`ensure-playwright.sh:150-155`, which this plan explicitly keeps ("Removing … the disk and
node-version floors" is in *What We're NOT Doing*). The sibling plan then places the free-space
precheck in the **launcher's** tree materialisation (its Phase 4 Step 4b, item 3), not in the design
executor. So the port has no behaviour to serve in this plan and, in the sibling's design, its
eventual owner is a different crate.

*Impact*: An unmotivated port costs a trait, a fake, a real adapter and a wiring site while inflating
the surface the plan asks reviewers to accept as "the boundary", and it plants the capability in the
wrong crate for the design that follows it.

*Suggestion*: Drop `FreeSpace` from this plan's executor port set, or state which `run.sh` behaviour
it reproduces; if it is deliberate forward-provisioning for the sibling, say so and reconcile it with
the sibling's launcher-side placement.

🟡 **major** (medium) — *Cache-namespace derivation and the runtime-layout precondition are assigned
to no module or port* — **Phase 6 §1 and §2 (exit 3 `playwright-not-installed`)**

`run.sh:84-97` derives the runtime namespace (`ACCELERATOR_PLAYWRIGHT_CACHE` default, sha256 of
`package-lock.json` truncated to 8 chars, `NS_ROOT`) and refuses with exit 3 when
`NS_ROOT/node_modules/playwright/package.json` is missing. Phase 6 §1's enumerated ported behaviours
cover start-time identity, tolerance, lock, state dir, spawn, poll and child environment — but never
the namespace derivation or its layout precondition. §2 preserves exit 3's *meaning* without saying
who computes the path it names, and §1's environment bullet lists `ACCELERATOR_PLAYWRIGHT_NS_ROOT` as
if it arrived from nowhere. This is precisely the surface the sibling plan replaces wholesale, so it
is the plan's most load-bearing seam.

*Impact*: A whole responsibility — with a hashing step, an environment-override precedence and a
filesystem precondition — has no module, no port and no test story, and the sibling plan has to
invent the boundary this plan should have drawn, which is how a seam ends up with two shapes.

*Suggestion*: Add a named domain concept and port for it (e.g. a `RuntimeLocation`/`Namespace` value
plus a `RuntimeLayout` port answering "is a usable runtime present at this root"), so the sibling plan
swaps one adapter implementation rather than restructuring the executor.

🟡 **major** (high) — *Thirteen unrelated assertion blocks are re-homed into a single-purpose by-name
gate* — **Removal sweep §1 (`test-design.sh` partial teardown and the floors)**

The sweep moves thirteen blocks — config `init` path keys, the `configure` paths table, docs lists,
"both browser agents exist and `tools:` is exactly `Bash`", "`browser-analyser` body forbids
`fetch`/`eval`", "`.mcp.json` does not exist", both skills' `evals.json`/`benchmark.json` validity —
into `scripts/test-skill-frontmatter-conformance.sh`. That file is a tightly-scoped
producer-conformance guard for work item 0103 (see its header: it extracts frontmatter literals,
derives the enforced attribute set from `templates-schema.tsv` ∪ `frontmatter-emission-rules.sh`, and
runs the real corpus validator), and it is the *sole* member of `_REQUIRED_CONFIG_SUITES`
(`tasks/test/integration.py:63`) precisely because it is "the gate that cannot drift undetected". The
plan says the target is "chosen by what each assertion is *about*", but its stated reason is that the
file "is already a `_REQUIRED_CONFIG_SUITES` by-name gate … and so run unconditionally" — a runtime
property, not a subject-matter one. Note also that `test-design.sh` **survives this plan**, so these
blocks have a working home already.

*Impact*: A cohesive, hard-to-drift contract guard becomes a grab-bag whose failures no longer point
at one thing, and the by-name protection granted to the frontmatter contract silently starts covering
unrelated design-skill trivia.

*Suggestion*: Either leave the re-homed structural blocks in `test-design.sh` for the sibling plan to
relocate when it deletes the file, or create a purpose-named suite (e.g.
`scripts/test-skill-structure-conformance.sh`) and add it to `_REQUIRED_CONFIG_SUITES` on its own
merits.

🔵 **minor** (high) — *The sub-domain-directory layout is claimed but only one directory is actually
named* — **Phase 2 §1 (Domain crate)**

Phase 2 §1 states the three sub-domains are "each a module directory rather than a prose grouping, so
a new module has an obvious home or an obvious rejection", and rests the whole layout argument on that
property. But only *runtime capability* is given a path (`src/runtime/`); source acquisition and
document auditing are listed as bare files (`host.rs`, `host_reach.rs`, `source_location.rs`,
`access_policy.rs`, `credentials.rs`, `leaked_credentials.rs`, `cue_phrase_audit.rs`) with no
directory. The cited corpus/vcs/work precedent is a precedent for the *crate triple*, not the module
shape: `cli/corpus/src`, `cli/vcs/src` and `cli/work/src` are flat, with `frontmatter_validation/` the
single nested exception.

*Impact*: The predictive property the layout is justified by does not exist as specified — two of three
sub-domains are prose groupings over flat modules — so an implementer will plausibly land everything
flat and the "obvious rejection" for a misplaced module never materialises.

*Suggestion*: Either name all three directories (`src/source_acquisition/`, `src/document_audit/`,
`src/runtime/`) and say the nesting is a deliberate divergence from the flat corpus/vcs/work module
layout, or drop the directory claim and justify the grouping some other way.

🔵 **minor** (high) — *The executor port set is enumerated inconsistently in three places* — **Phase 6
§1 vs Testing Strategy → Unit Tests**

Phase 6 §1 lists eight ports (`Clock`, `ProcessProbe`, `StateStore`, `Lock`, `Spawner`,
`ProcessControl` — introduced inside the `Spawner` bullet — `RunClient`, `FreeSpace`). The Testing
Strategy lists seven, dropping `FreeSpace`. The Phase 6 success criteria name only four.
`ProcessControl` gets no bullet of its own despite being a distinct port with distinct semantics
("recovery never signals a pid it cannot identify").

*Impact*: The port set *is* the domain crate's outbound contract and the thing every fake, adapter and
composition site must agree on; three different enumerations invite one to be forgotten in
implementation and make the boundary hard to review as a whole.

*Suggestion*: State the port set once as a table (port → responsibility → adapter module → fake) and
reference it from the Testing Strategy and success criteria instead of re-listing it.

🔵 **minor** (medium) — *The `CuePhraseMatcher` port splits pattern data from pattern semantics across
crates, and lands a work-domain vocabulary in the design crate* — **Phase 2 §1
(`cue_phrase_audit.rs` and the `CuePhraseMatcher` port)**

The plan keeps the `const` pattern slice in `cli/design/` and compiles it in `design-adapters` behind a
`CuePhraseMatcher` port. That is the *same* coupling shape the plan rejects two sections later when it
refuses to `include_str!` the downgrade table into the binary — data in one crate, its meaning in
another, "coupled only by runtime string equality", with nothing preventing the adapter compiling a
different set. The `IdScanner` precedent it invokes is weaker support than it looks: there the pattern
is genuinely external (it comes from `work.id_pattern` configuration), whereas here the domain owns the
data and the port exists only to satisfy the lint rule. Consequently the domain's unit test "H2
sectioning against the cue-phrase patterns" cannot actually exercise the patterns — a fake matcher
tests plumbing, which is the exact objection the plan raises against a `FakeClock` in Phase 1 §4.
Separately, `scripts/extract-work-items-cue-phrases.txt` declares itself canonical for
`extract-work-items` too and "mirror[s] the cue-phrase enumeration in
skills/work/extract-work-items/SKILL.md:130-138", so the compiled home of a work-domain vocabulary
becomes the design crate.

*Impact*: The one behaviour that matters (does `users? need` match this heading's body) is only testable
in the adapter, which the plan does not say, and a future `accelerator work` consumer of the same
vocabulary must either depend on `design` or duplicate it.

*Suggestion*: Have the adapter's constructor take the domain `const` slice as its only source and state
where the compiled-pattern behaviour is tested; and note explicitly why the canonical set lives in
`design` rather than `work`/`corpus` given the shared-file header, or record it as a follow-up if
`extract-work-items` migrates later.

🔵 **minor** (medium) — *`design-adapters`' module inventory is incomplete relative to its own
per-module pup scoping* — **Phase 2 §2 (Adapters crate)**

Phase 2 §2 lists four adapter responsibilities (path-existence checks, file reading, environment reads,
the compiled cue-phrase regex) but declares only three module names — `design_adapters::filesystem` and
`design_adapters::environment` carrying the no-spawn rule, and `design_adapters::process` spawning by
design. Because the plan deliberately scopes the rule per module rather than crate-wide, any module not
named is unguarded: the regex matcher has no home, and Phase 6's `StateStore` JSON parsing (which cannot
live in the domain) has neither a module nor a rule.

*Impact*: The no-spawn guarantee the scoping exists to express covers only the modules that happen to be
enumerated, so the seam erodes by omission rather than by decision — the failure mode the plan cites when
it refuses to land the rule crate-wide and weaken it later.

*Suggestion*: Enumerate every `design-adapters` module for both phases (including the state/JSON module
and the matcher) and say which rule each falls under, or invert to a crate-wide
`denied: ["^std::process"]` with `process` excluded, as
`migrate_adapters_decision_source_reads_in_process` does for the all-in-process case.

🔵 **minor** (high) — *The plan states its own independence but not the sibling's hard dependency on it*
— **Overview → "Why this is a separate plan"; Migration Notes**

The plan asserts one direction of the seam ("nothing here touches the launcher's fetch-verify-cache
mechanism, the release pipeline, or the manifest schema"), which holds. The reverse is never stated: the
sibling plan's Phase 7 edits `cli/design-cli/src/executor.rs`, adds `cli/design/src/platform.rs` and
`cli/design-adapters/src/platform.rs`, and builds its state dir on "the repo's config tmp path the
sibling plan's Phase 6 already establishes" — so sibling Phase 7 cannot land before this plan's Phases 2
and 6. Both plans also edit `cli/design/`, `cli/pup.ron`, `lib/*.js`, `scripts/test-design.sh` and both
design SKILL.md files, and this plan leaves `_EXPECTED_CONFIG_SUITES` at 15 against 16 discovered suites
in the interim.

*Impact*: The seam reads as symmetric when it is not; without the ordering stated, sibling Phase 7 could
be scheduled against an absent executor crate, and the shared-file overlap is invisible to whoever
sequences the two.

*Suggestion*: Add a short cross-plan dependency statement — sibling Phases 0/4/5 are parallelisable,
sibling Phase 7 requires this plan's Phases 2 and 6 merged — plus the list of files both plans touch.

🔵 **suggestion** (medium) — *Phase 2 is a large single mergeable unit; only a subset of it is forced to
co-land* — **Phase 2 (scope) and Implementation Approach (phase graph)**

Phase 2 creates three crates, ports five scripts (~570 shell lines including a from-scratch
SSRF-hardening rewrite with a hand-enumerated reserved-address set), adds two pup rules, completes the
thirteen-point registration surface, rewires both SKILL.md files, deletes five scripts plus two bash
suites, cuts and rewrites six ranges of a CI-run shell file, and adds a docs page. The registration
constraint only forces points 1, 2, 3, 4, 7 and 8 together — i.e. the binary plus *one* bound subcommand
— not all five.

*Impact*: A single PR combining new-crate scaffolding, a security-relevant behaviour change and the
registration surface is hard to review as one unit, and "oversized PR" was one of the stated reasons the
parent work was split out of 0173 in the first place.

*Suggestion*: Split Phase 2 into "crates + registration + `notify-downgrade` (the simplest bound
subcommand) + skill binding" and "the remaining four subcommands", noting that only the first carries the
co-landing constraint.

🔵 **suggestion** (medium) — *The verdict table conflates an exactly-comparable own probe with an inferred
legacy one* — **Phase 6 §1 (reuse verdict table, ±1s tolerance)**

Once the launcher observes the start time "with the same probe it will later use to check it", a `Probe`
record and a later observation of the same live process are bit-identical by construction — both are
`btime + ticks/hz` truncating division, or both `p_starttime` seconds. The ±1s tolerance existed because
the *daemon* recorded `Math.floor(Date.now()/1000)` after module loading (`run.sh:44-51`,
`state.js:60`), a cause this plan removes. The tolerance is still needed, but only for the legacy case
the plan separately names: a record with no `start_time_source` key, read as `Probe` because "`ps lstart`
is itself derived from `p_starttime` and agrees to the second".

*Impact*: Because both cases share the `Probe` variant, the one-second slack that only legacy records
need is applied permanently to every record, widening the PID-recycle window the identity contract exists
to close.

*Suggestion*: Give the legacy inference its own variant (e.g. `RecordedStartTime::{Probe(u64),
LegacyProbe(u64), Wallclock(u64), AbsentOrUnparseable}`) so `Probe` compares exactly and only
`LegacyProbe` carries ±1s, and add both rows to the exhaustive verdict-table test.
### Test Coverage

**Summary**: This is an unusually rigorous plan from a test-coverage standpoint: the executor
port set makes the riskiest logic a deterministic pure function, the goldens are
exhaustive-by-construction, and it spots subtle detection gaps (a bare `return` in a
`node --test` body reports as *passed*). Every line range in the Removal sweep §1 disposition
table, and every floor count I could check (16 discovered `scripts/` suites vs a floor of 15;
ten `lib/*.test.js` → 8; 14 `skip: !playwrightInstalled` tests in `test-run.js`), verified
exactly against the tree. The gaps that remain are concentrated in three places: one suite the
plan asserts does not exist (`scripts/test-metadata-helpers.sh` drives *both* metadata scripts
Phase 3 deletes, in two CI lanes), the adapter half of the executor port where ADR-0058's
silent regressions actually live but no test mechanism is named, and several fidelity anchors
(downgrade fixtures, `test-run.sh`'s own assertions, the Phase 3 four-line golden) that are
demoted to manual or dropped without a disposition.

**Strengths**:

- The Removal sweep §1 disposition table is line-accurate: all 24 rows checked against the real
  553-line `scripts/test-design.sh` and every range lands on the correct block boundary
  (169-281, 282-315, 316-338, 368-425/428-430, 442-485, 486-490, 542-546, 547-551 all
  verified), and the table is complete at block level. A teardown inventory of this fidelity is
  rare and is exactly what makes the phase-by-phase cuts safe.
- Every floor derivation verifiable is correct: `scripts/` discovers 16 suites (17 `test-*.sh`
  files less the excluded `test-helpers.sh`) against `_EXPECTED_CONFIG_SUITES = 15`;
  `lib/*.test.js` is ten files today, so 8 after deleting `identity.test.js` and `lock.test.js`;
  `test-run.js` carries exactly 14 `skip: !playwrightInstalled` tests; `daemon.test.js:72` is
  the single bare-`return` gate.
- It correctly identifies that `node --test` reports an early `return` inside a test body as
  *passed*, not skipped, and that neither a discovered-file floor nor a skip count can see that
  — a genuine and rarely-noticed detection gap, and the same shape it condemns in
  `identity.test.js:70-95`.
- The executor's port set with named return *types* (`ObservedStartTime`, `RecordedStartTime`)
  turns the reuse verdict into a total function over four inputs, so cold start, warm reuse,
  stale-PID recovery, PID-recycle rejection, contention and start timeout are unit-testable with
  no sleeps, no real processes and no real elapsed time. That is the right pyramid level for the
  plan's most regression-prone code.
- It explicitly recognises that a checklist derived from the deleted bash suites can only demand
  tests for behaviour the shell already had, and adds a dedicated table-driven criterion for the
  *newly*-rejected host encodings — the trap most migration plans fall into.
- Phase 1 §4 refuses a `FakeClock` test on the renderer with the correct reasoning (a fake
  replaces the very seam under test, so the assertion degenerates to plumbing) and points AC15
  at the existing pure-function pin, verified to exist and pin `CompactTime` to
  `"2026-07-13-090507"` (`cli/corpus-adapters/src/metadata.rs:297-322`).
- Replacing `notify-downgrade.sh`'s runtime bidi/printable-ASCII filters with a table invariant
  over the compiled message set is the right call, with the unreachable-by-construction argument
  stated rather than assumed.

**Findings**:

🔴 **critical** (high) — *Both metadata scripts Phase 3 deletes are driven by
`scripts/test-metadata-helpers.sh`, in two CI lanes* — **Phase 3 §2 (Deletion, "Neither script
has a bash suite, so no floor moves")**

Phase 3 states "Neither script has a bash suite, so no floor moves", but
`scripts/test-metadata-helpers.sh:21-24` names *both* deleted scripts in its `HELPERS` array
(`inventory-design/scripts/inventory-metadata.sh` and
`analyse-design-gaps/scripts/gap-metadata.sh`) and runs each inside hermetic git and jj temp
repos, asserting the output contract. That suite runs in **two** lanes: `tasks/test/unit.py:41`
lists it as a `test:unit:templates` driver (inside the `test:unit` aggregate, run on both matrix
legs at `.github/workflows/main.yml:53`) and it is also glob-discovered by
`test:integration:config`. Deleting the scripts leaves
`output=$(run_helper_in_clean_repo git "$helper" || true)` empty, every `assert_matches_regex`
fails, and both lanes go red. Emptying `HELPERS` instead makes the loop body never execute, so
the file passes *vacuously* while still counting toward the suite floor. Deleting the file drops
`scripts/` discovery from 16 to 15 — exactly the floor — which then breaks the plan's stated
handoff ("the sibling plan deletes `test-design.sh` … 15 actual against a floor already at 15"),
because the sibling would land on 14 against 15.

*Impact*: Phase 3 cannot merge green as written, the plan's floor arithmetic and its handoff to
the sibling plan are both wrong, and the only executable assertion of the two helpers' output
contract disappears with no recorded disposition.

*Suggestion*: Add `scripts/test-metadata-helpers.sh` and `tasks/test/unit.py`'s driver list to
Phase 3's file set: delete the suite outright (its property already lives in
`cli/corpus-adapters/tests/metadata.rs:100-147` and `cli/corpus-cli/tests/metadata_goldens.rs:49-91`,
whose doc comments name it), drop the driver from `tasks/test/unit.py`, add
`mise run test:unit:templates` to Phase 3's automated criteria, and restate the suite-count
arithmetic as 15-after-Phase-3 with the sibling's `_EXPECTED_CONFIG_SUITES` decrement to 14
recorded as inherited work.

🟡 **major** (high) — *The 153/154 split in the disposition table breaks the sibling-owned
assertion it leaves behind* — **Removal sweep §1 (rows "both skills' structure | 138-153" and
"`inventory-design` `allowed-tools` `scripts/*` glob | 154-155 | sibling")**

The table re-homes `scripts/test-design.sh:138-153` and `156-168` while leaving `154-155` for the
sibling plan. But line 140 — inside the re-homed range — is
`SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"`, and lines 154-155 read
`"$(cat "$SKILL")"`. Under the file's `set -euo pipefail` an unbound `$SKILL` aborts the suite
immediately. Line 153 is also the `# shellcheck disable=SC2016` directive that exists solely for
line 155's single-quoted `'Bash(${CLAUDE_PLUGIN_ROOT}/…)'` literal, so cutting it un-suppresses
SC2016 and reddens `lint:scripts:check`.

*Impact*: A mechanical application of the table — which is the whole point of enumerating exact
ranges — leaves `test:integration:config` red on merge and breaks the plan's own "each phase
leaves the tree green" invariant, in the same defect class the plan flags for `:359-364`.

*Suggestion*: Restate the row as `138-152` + `156-168` re-homed, with `153-155` (directive plus
assertion) held together for the sibling, and add a line to the row noting that the `SKILL=`
definition must be duplicated into whichever side keeps assertions using it.

🟡 **major** (high) — *Four re-homed grep assertions become self-matching inside the tree they
scan, and shift the floor of 8* — **Removal sweep §1 ("re-home → node suites") and Phase 6 §6
(floor of 8)**

Four `test-design.sh` blocks are re-homed "→ node suites": `:121-129`
(`evaluate-payload-rejected` and `mcp__playwright__` must not appear under `lib/` or `run.js`),
`:491-510` (PROTOCOL.md ↔ daemon.js sync), `:511-517` (`links` in `BLOCKING_OPS`) and `:532-541`
(no `ownerPid`/`--owner-pid`/`OWNER_POLL_MS` anywhere under `playwright/`). Three of those are
greps *for literal identifiers* over a tree the new test file would itself live in — and
`:534-536`'s own comment says the sweep fires "regardless of which file (**or new test**)
reintroduces the symbol". A `lib/*.test.js` containing the pattern string `ownerPid` or
`evaluate-payload-rejected` matches itself, so the assertion inverts and fails permanently.
Separately, the destination is unspecified: any new file under `lib/` changes the discovered count
that Phase 6 §6 pins at **8**, and a file at the `playwright/` root falls outside the unit lane's
`lib/*.test.js` glob and so into the opt-in lane that no CI job runs.

*Impact*: Four regression nets — including a resolved-incident guard
(`meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`) — either fail on arrival,
silently leave CI, or invalidate the floor they are counted against.

*Suggestion*: Name the destination file(s) explicitly, restate the floor derivation to include
them (8 + n), and specify how each grep avoids self-matching — e.g. run the sweep from a suite
outside the scanned tree, exclude `*.test.js` from the scan set (as `test-run.sh:27-29` already
does), or build the needle from concatenated fragments.

🟡 **major** (high) — *No automated anchor survives for the ported downgrade message text* —
**Phase 2 §3 (canonical data as `const`s) and Manual Verification; Removal sweep criteria**

The six downgrade messages get three fidelity mechanisms in the plan, and all three fail. (1) The
existing goldens — `skills/design/inventory-design/evals/fixtures/notify-downgrade/*.expected.txt`,
one per reason — are deleted ("deleted with its fixtures", Current State table). (2) Byte-for-byte
equivalence is listed under **Manual** Verification only. (3) The stated `#[cfg(test)]`
`include_str!` drift test needs `notify-downgrade-messages.json`, which lives at
`skills/design/inventory-design/scripts/notify-downgrade-messages.json` — directly contradicted by
the Removal sweep's own criterion "Only `ensure-playwright.sh` and the retained JavaScript remain
under `skills/design/**/scripts/`". The surviving automated assertions (exhaustive-by-construction
goldens, printable-ASCII/bidi table invariant) all pass against goldens regenerated from the new
implementation, so a mistranscribed message is invisible.

*Impact*: The plan's central promise for this subcommand — behaviour preserved byte for byte, on
the graceful-degradation path users hit when the runtime is unavailable — rests on one human eye at
migration time.

*Suggestion*: Retain the six `*.expected.txt` fixtures as the Rust golden inputs (`include_str!`
per reason, iterated from the enum so exhaustiveness still holds by construction). That makes
byte-for-byte equivalence automated, deletes the manual criterion, and resolves the JSON
contradiction by moving the anchor out of `skills/design/**/scripts/` entirely.

🟡 **major** (high) — *The adapter half of the executor port has criteria but no named test
mechanism* — **Phase 6 §1/§8 and Success Criteria**

The domain/port split correctly makes the reuse verdict a deterministic unit test, but that pushes
every behaviour ADR-0058 names as a silent-regression risk into `design-adapters`:
`setsid`/double-fork so the daemon survives a SIGHUP to the launcher's group, stdio redirection to
the bootstrap log, log truncation plus `chmod 0600` under a `umask 077`, `server-stopped.json`
removal before spawn, the `flock` `Drop` guard, and `CommandExt::exec` passing the client's exit
status and signal death (128+n) through unchanged. Phase 6's Automated Verification asserts all of
these, yet the Testing Strategy section lists only "Domain logic in `cli/design/`" and "the
executor's reuse verdict" as unit tests, and no harness, fixture or seam is named for the adapter
layer. Meanwhile `test-run.sh` — today's only launcher-level integration suite — is deleted in §8,
and its replacement lane covers `test-run.js`, which forks `run.js` directly (`test-run.js:18,68`)
and therefore never exercises the launcher at all.

*Impact*: The half of the port most likely to regress silently ends up with assertions listed as
automated but nothing stated to make them so, and launcher-level end-to-end coverage disappears
entirely, leaving only manual session checks.

*Suggestion*: Name an adapter-level integration harness (e.g. `cli/design-cli/tests/executor.rs`)
that drives the real binary with the forwarded script path injected — a stub `run.js` that prints
its stdio, exits with a chosen code, dies on a chosen signal, and never signals ready. All six
criteria above then run in CI with no Playwright runtime, and the plan can state the injection
point (an env override or a `Spawner` config value) rather than leaving it implicit.

🟡 **major** (high) — *`test-run.sh` is excluded from the migration checklist, dropping the `links`
output contract with no disposition* — **Phase 2 Success Criteria (migration checklist scope) and
Phase 6 §8**

The enumerated checklist covers `test-design.sh:169-338`, `:368-430`, `test-validate-source.sh` and
`test-notify-downgrade.sh` — and explicitly *not* `test-run.sh`, which Phase 6 dismisses as
"structural and shellcheck checks, the `start_time_of` locale comparison, a
ping/daemon-stop/links block and a survives-shell-exit smoke test". But `test-run.sh:113-192` is 20
behavioural assertions over the **retained** `daemon.js` `links` implementation, several of them a
data-exposure contract: no raw `href`, no fully-resolved `resolved` field, no echoed query string or
fragment, plus pathname/role/`same_origin` opaque-origin semantics and the `about:blank`
empty-array case. `test-run.js` contains no `links` test, so nothing replaces them. The same
applies to `:16-21` (`node -c run.js`, `jq empty package.json`, `package-lock.json` exists) over
retained artefacts.

*Impact*: A privacy-shaped output contract on retained JavaScript loses its only executable
statement, silently, in a plan whose stated principle is that every deleted assertion maps to a
named replacement or a recorded drop.

*Suggestion*: Bring `test-run.sh` into the migration checklist's scope on the same terms as the
other three suites, and re-home `:124-157` and `:183-186` into the opt-in
`test:integration:design-automation` lane (they need a runtime) or into a `daemon.test.js` block
using the existing fixture server, with the structural checks folded into the new unit task.

🟡 **major** (high) — *The four-line characterization golden names no clock seam, no host suite, and
rests on an inaccurate premise* — **Phase 3 §2 (Deletion)**

Three problems. (a) No clock-fixing mechanism is stated, and Phase 1 §3 establishes there is none
through the binary (`derive_at` builds its own `SystemClock`,
`cli/corpus-adapters/src/metadata.rs:230-238`) — while the bash side reads `date`, so a captured
golden is a one-shot artefact, not a repeatable assertion, and any timestamp comparison is flaky
near a second/day boundary. (b) No suite or task is named to host it; Phase 3's automated criteria
list only `test:integration:config`, `lint:dispatch-coherence:check`, a no-`.sh` check and
`mise run`, so the new golden has no CI home. (c) The premise — that label/order equivalence "is
verified nowhere automatically" — is inaccurate: `cli/corpus-adapters/tests/metadata.rs:150-164`
already pins `"Timestamp For Filename: 2026-07-13-090507"` for `CompactTime` behind a `FakeClock`,
`:100-147` pins the very block contract the bash suite holds the helpers to, and
`cli/corpus-cli/tests/metadata_goldens.rs:93-141` pins the git, jj and outside-a-repository cases
through the compiled binary (covering Phase 3's divergence #4 already). The one property genuinely
unpinned is exact four-line *order*.

*Impact*: A new, non-deterministic, unhomed golden is proposed to cover a property that is largely
already covered, while the property that is actually missing stays missing — and AC15's
whole-output byte-for-byte claim stays on manual verification.

*Suggestion*: Drop the captured-from-bash golden. Add one full-block equality assertion to
`cli/corpus-adapters/tests/metadata.rs` using the `FakeClock` and stub facts already there
(`render(&derive(&FakeClock, Some(&facts), CompactTime), CompactTime) == "…4 lines…"`), which pins
order, labels and the timestamp deterministically in an existing CI-run suite, and note in Phase 1
§3 that `metadata_goldens.rs:86-90`'s `"Timestamp For Filename: "` assertion embeds a
`DateTimeUnderscored` claim that must become format-aware when the harness is parameterised.

🟡 **major** (high) — *New credential-scanning behaviour gets no named test, unlike the parallel
`host_reach` case* — **Phase 2 §1 (`leaked_credentials.rs`)**

The plan spots that a shell-derived checklist can only demand tests for behaviour the shell already
had, and fixes it for `host_reach` with a dedicated criterion listing every newly-rejected
encoding. The identical trap applies to `leaked_credentials.rs`, which introduces genuinely new
behaviour: because `ACCELERATOR_BROWSER_AUTH_HEADER` holds a full `Name: value` pair, the scan
splits it and matches the value component too — the plan's own stated rationale being that "an
artefact rendering just the bearer token, the likely leakage shape, matches nothing" otherwise.
Today's coverage (`test-design.sh:316-338`) only exercises a whole
`ACCELERATOR_BROWSER_PASSWORD` value, so the checklist will not demand a test for the split, and
neither the Success Criteria nor the Testing Strategy names one (the latter lists "the auth
precedence table" but not the scan).

*Impact*: The one security-relevant behaviour improvement in the ported subcommands could ship
non-functional with every listed test green — the exact failure mode the plan guards against
elsewhere.

*Suggestion*: Add a Phase 2 criterion mirroring the `host_reach` one: a table test over
`leaked_credentials` asserting that a `Name: token` header variable matches an artefact containing
only `token`, that the reported output names the variable and never the value, and that the header
*name* alone does not trigger a false positive.

🟡 **major** (medium) — *The bare-return detector cannot express the property it states, and the
zero-skip lane cannot exclude the extracted test* — **Phase 6 §6 (Node suite runner)**

The detector is specified as "a grep assertion … refuses any retained suite containing a bare early
`return` or `catch { return; }` **in a test body**" — but grep cannot distinguish a test body from a
helper, and legitimate helper returns are everywhere in the retained suites
(`daemon.test.js:17,20` `return null`, `test-run.js:31` `if (existsSync(filePath)) return;`). The
assertion is therefore either false-positive-prone (forcing awkward rewrites of correct helpers)
or, once narrowed enough to pass, blind to the case it exists to catch. Related: no mechanism is
named for obtaining the executed/skipped counts the floors assert (`node --test` needs a
machine-readable reporter parsed by the task). And the zero-skip lane is unsatisfiable as scoped —
the runtime-gated test lives *inside* `lib/daemon.test.js`, which the unit lane's `lib/*.test.js`
glob discovers, so converting it to a real `skip:` breaks zero-skip while extracting it to a new
`lib/*.test.js` file re-enters the same glob and shifts the floor of 8.

*Impact*: The two assertions the plan identifies as the ones that "would have caught this class"
are the two least likely to be implementable as written, so the new lane could ship with the same
silent-pass weakness it was created to close.

*Suggestion*: Base both gates on `node --test --test-reporter=tap` output parsed in the invoke task
(pass/fail/skip counts plus a per-file executed floor), which makes wholesale skipping and vacuous
passing both visible without pattern-matching source. Name the destination file for the extracted
runtime test and state how the unit glob excludes it — e.g. move it to
`playwright/test-daemon-runtime.js` and list the integration lane's files explicitly, keeping
`lib/*.test.js` runtime-free by construction.

🔵 **minor** (high) — *The opt-in lane runs in no CI job, so its own fail-not-skip guarantee is
unverified* — **Phase 6 §6 / Testing Strategy (`test:integration:design-automation`)**

`test:integration:design-automation` is explicitly "not in the default `mise run`" and is not added
to the `test:integration` aggregate, so no CI job invokes it (`.github/workflows/main.yml:91` runs
the aggregate). Consequently the criterion "`test:integration:design-automation` **fails** rather
than skips when no Playwright runtime is present" is itself only checkable by hand, and the suites
it owns — `test-run.js`'s 14 runtime tests plus the extracted `daemon.test.js` block — get zero CI
execution under this plan. That is the same bit-rot condition the phase exists to fix.

*Impact*: A task that can never be green in CI is a task nobody notices decaying, and the
fail-not-skip preflight is the one part of it whose correctness matters most.

*Suggestion*: Cover the preflight itself with a `tests/unit/tasks` test (the `docker info` precedent
at `tasks/test/e2e.py:105-111` is already unit-testable in that style), and add the task to a
scheduled or manually-dispatched workflow so its absence-of-runtime refusal and its eventual
runtime-present pass are both observed at least periodically.

🔵 **minor** (high) — *Two verification criteria name tasks that do not exist or omit the lane that
will break* — **Phase 2 and Removal sweep Success Criteria**

`mise run test:unit:build-system` is cited as a gate in Phase 2 ("passes with the updated registry
pins") and again in the Removal sweep ("including `test_registration_docs.py`"), but no such task
exists: `mise.toml` defines `test:unit:{visualiser,frontend,tasks,cli,templates}`, and the pytest
suites under `tests/unit/tasks` run as **`test:unit:tasks`** (`build-system` is a
`check`/`lint`/`format`/`types` namespace only). Conversely, `test:unit:templates` — the lane that
will actually break when the metadata scripts are deleted — appears in no phase's criteria.

*Impact*: An implementer following the criteria literally gets "task not found" for the registration
gates and never runs the lane the change breaks, so a checklist that reads complete is not runnable
as written.

*Suggestion*: Replace both occurrences with `mise run test:unit:tasks`, and add
`mise run test:unit:templates` to Phase 3's automated criteria.

🔵 **minor** (medium) — *The migration checklist has no path and no completeness check* — **Phase 2
Success Criteria**

The checklist is the plan's central evidence that no assertion was lost, and the plan already
anticipates one self-certification risk (each deliberate-drop row must name a replacement property).
But no path is given for the artefact, and nothing verifies the row set is *complete* — a missed
assertion produces no row and therefore no failure. The source material makes completeness
mechanically checkable: every assertion in the deleted suites carries a unique label string
(`assert_exit_code "rejects file:// scheme" …`), so the row count and the label set can be derived
rather than transcribed.

*Impact*: The one artefact standing between a 553-line teardown and silent coverage loss is
validated only by the author's own reading.

*Suggestion*: Name the committed path (e.g. under `meta/notes/`), and generate the row skeleton by
extracting the assertion labels from the four deleted suites before cutting them, so the checklist's
completeness is a diff against a derived list rather than a claim.

🔵 **minor** (high) — *A Phase 3 criterion depends on Phase 2, which the dependency graph says it
does not* — **Implementation Approach vs Phase 3 Success Criteria**

The graph declares `Phase 1 ──> Phase 3` and `Phase 2 ──> Phase 6`, i.e. Phase 3 is independent of
Phase 2, and the plan stresses that "every phase is independently mergeable". But Phase 3's
criterion "No `.sh` remains in `analyse-design-gaps/scripts/`" cannot hold until Phase 2 deletes
`audit-cue-phrases.sh` — the directory contains exactly those two scripts.

*Impact*: If Phase 3 merges first, a criterion listed as automated fails, undermining the
independent-mergeability property the phase split is built on.

*Suggestion*: Narrow the criterion to "`gap-metadata.sh` is gone and no call site references it",
and move the directory-empty assertion to whichever of Phase 2 or Phase 6 lands last (or to the
Removal sweep, which already carries the `skills/design/**/scripts/` residue check).

### Code Quality

**Summary**: The plan's core design instincts are strong: ports-and-adapters with domain purity
enforced by a copied cargo-pup rule, a reuse verdict as a pure total function over injected ports,
rejection modelled as a domain verdict rather than an inverted `kernel::Error`, and modules named by
domain concept rather than one-per-deleted-script. But at ~1,560 lines it carries visible residue
from its superseded predecessor: Phase 6 §3 still specifies the daemon-record writer that Phase 6
§1 explicitly repudiates, §4 asserts and then withdraws the same cross-language assertion, and the
same rationale is restated up to four times, so a future edit will update one copy and leave the
rest. Several named types are also under-specified at exactly the points the plan itself says
matter — `Verdict<Reason>` carries a reason and a pre-rendered stderr and cannot express the
executor's exit 3, `HostReach`'s five variants cannot carry the reserved ranges the same section
enumerates, and the `(RecordedDaemon, ObservedDaemon)` pair cannot represent the `NoPidRecorded` row
of its own totality table.

**Strengths**:

- The reuse verdict is modelled as a pure function over injected ports with a row-per-case table
  test, so the riskiest code in the plan is testable without real processes, real elapsed time or
  sleeps — the strongest testability decision in the document.
- Ports return domain values rather than raw infrastructure, and the plan explicitly reasons that
  "a port whose return type is unstated cannot carry them" — the right standard for a hexagonal
  seam.
- Refusing to invert `kernel::Error` semantics is correct and well argued: keeping `Refusal → 2`
  consistent with `corpus-cli:132`/`vcs-cli:77` preserves a documented cross-binary contract rather
  than bending it for one binary.
- Pre-declaring the `design_adapters::{filesystem,environment,process}` split and its scoped
  no-spawn pup rule in Phase 2, before Phase 6 needs to spawn, avoids landing a crate-wide rule and
  weakening it later — the plan states exactly that reasoning.
- The const-in-domain plus `#[cfg(test)]` `include_str!` drift-test pattern is a faithful reuse of
  `cli/corpus/src/frontmatter_validation/schema.rs:277`, and turns downgrade-table exhaustiveness
  into a compile error rather than a runtime lookup failure.
- Organising `cli/design/` by domain concept rather than one module per deleted shell script, with
  the corpus crate cited as precedent, resists the most likely design failure of a script-to-Rust
  migration.
- Naming the `Verdict` carrier as a new shape rather than describing it as "corpus-cli-style" is
  intellectually honest and saves the next reader a wasted search for a pattern that does not exist.

**Findings**:

🔴 **critical** (high) — *Two sections specify mutually exclusive writers for the daemon identity
record* — **Phase 6 §3 vs Phase 6 §1**

Phase 6 §1 settles the daemon identity contract as: the launcher observes the start time at fork,
hands `(pid, start_time, start_time_source, token)` to the daemon through the environment, and
`state.js` publishes all of it in its existing single `atomicWrite` — explicitly repudiating the
alternative because it "gave one file two whole-file-rename writers, which is a lost-update
contract" and leaves a window where a live daemon has a partial record. Phase 6 §3 then specifies
that repudiated design verbatim: the launcher "writes the identity record once the daemon reports
ready" while "`state.js` writes the port and readiness facts it owns", and adds that
"`start_time_source` survives as a field the Rust writer sets".

*Impact*: The single most regression-prone mechanism in the plan (ADR-0058 names it the port's
principal silent-regression risk) has two contradictory specifications in adjacent sections, and the
discarded one is the variant §1 shows creates a live-daemon-with-no-start-time window that §3's own
recovery rule turns into orphaning a healthy daemon mid-crawl.

*Suggestion*: Rewrite Phase 6 §3's `state.js` paragraph to state only what §1 settled — the launcher
hands the values to the daemon at spawn, `state.js` publishes the whole record in one write, and
`start_time_source` is a value the launcher supplies rather than a field "the Rust writer sets" — so
`server-info.json` has exactly one writer stated in exactly one place.

🟡 **major** (high) — *`Verdict<Reason>` carries redundant state and cannot express the executor's
outcomes* — **Phase 2 §3 (The carrier is named)**

Three problems: `Rejected` holds both a structured `reason` and a pre-rendered `stderr`, two
representations of the same fact with no stated authority or rendering owner; the generic parameter
earns nothing at the `main` boundary if `stderr` is already rendered (and needs a `Display` bound if
it is not); and the shape cannot express the sixth subcommand this same plan adds — Phase 6 §2 keeps
the executor's exits **0, 1, 2 and 3**, its daemon-side errors that print an error envelope on
**stdout at exit 0**, its signal-death propagation (128+n), and a success path that terminates via
`CommandExt::exec` and never returns a value at all. `Accepted` has no `stderr` field and `Rejected`
no `stdout`, so the executor's asymmetry has nowhere to live.

*Impact*: The carrier is presented as the settled shape for the whole binary but is only adequate
for five of six subcommands, so the executor will grow a second bespoke exit path in `main` —
reintroducing exactly the per-subcommand mapping the carrier was justified as avoiding — and the
redundant `reason`/`stderr` pair invites the two to drift.

*Suggestion*: Specify one carrier that spans all six subcommands — e.g. a non-generic
`Verdict { Accepted { stdout }, Rejected { stdout, stderr, code } }` or an explicit
`ExitCode`-bearing outcome — and state whether the executor uses it or is documented as a deliberate
exception with its own terminal path. Also rename it to avoid colliding with the domain "verdict"
that `access_policy.rs` returns, since the plan currently uses one word for both a domain value and a
transport type.

🟡 **major** (high) — *The usage-vs-rejection rule contradicts itself on a nonexistent path argument*
— **Phase 2 §3 ("The rule, not just the examples")**

The plan states the rule as: a usage error is a malformed *invocation* — "an argument the tool cannot
interpret at all" — and "anything the tool successfully evaluated and then rejected is a verdict".
Its own examples then split an identical input class two ways: `scrub-secrets` on a nonexistent file
is exit **2**, while `validate-source` on a path is exit **1**, cited as "matching
`validate-source.sh:223-226`". Those cited lines are `if [[ ! -d "$location" ]]` with the message
"does not exist or is not a directory" — a single branch covering *both* the nonexistent and the
not-a-directory case, so the citation authorises exit 1 for precisely the case the rule sends to exit
2. Both tools perform the same `stat` and get the same `ENOENT`, so the rule as written does not
discriminate between them.

*Impact*: The most common user error (a mistyped path) has an undefined exit code, and the plan's own
manual-verification criterion pins `scrub-secrets /nonexistent` → 2 while the characterization
checklist will pin `validate-source /nonexistent` → 1, giving one binary two contradictory contracts
for the same failure that SKILL.md logic then has to discriminate on.

*Suggestion*: Restate the rule so it turns on something observable — e.g. "existence and type of a
path argument is always evaluated, so a missing path is a verdict (exit 1) in every subcommand" — and
correct whichever example loses, recording the `scrub-secrets` change as a deliberate behaviour change
alongside the exit-2 usage split already declared.

🟡 **major** (high) — *The `HostReach` variant set cannot carry the classifications the same section
requires* — **Phase 2 §1 (`host_reach.rs`)**

`host_reach.rs` is specified as "a `HostReach` classification: loopback, private, link-local,
unspecified, or public", but the same section then requires classifying `100.64.0.0/10`,
`192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4`, IPv6 `fc00::/7`, and multicast, plus "any host that
*looks* numeric but fails strict parsing" as a rejection. None of those five extra classes maps onto
any of the five variants — CGNAT and `240.0.0.0/4` are not `is_private`, and there is no `reserved`,
`multicast` or `unparseable-numeric` variant. The classification is also user-facing:
`validate-source.sh:287` prints it ("host X is a $classification address. Pass --allow-internal"),
and the Removal sweep §2 requires the docs to split internal-reach-recoverable addresses from
unconditional numeric-encoding rejections — a distinction the enum cannot express either.

*Impact*: Every unlisted range gets forced into `private` (making the message wrong) or into an
unnamed sixth variant invented at implementation time, and ownership of numeric-encoding rejection is
split ambiguously between `host.rs` and `host_reach.rs`.

*Suggestion*: Enumerate the variant set against the classification the section actually demands (add
`reserved`, `multicast`, and whatever variant carries "numeric-looking but unparseable"), state for
each variant whether `--allow-internal` recovers it, and name which module owns the numeric-encoding
rejection so the two modules cannot both claim it.

🟡 **major** (high) — *The verdict table's `NoPidRecorded` row is not representable in the types the
plan names* — **Phase 6 §1 (reuse verdict table and its input model)**

Phase 6 §1 states the executor's reuse input is `(RecordedDaemon, ObservedDaemon)` where
`RecordedDaemon { pid, start_time: RecordedStartTime }` and
`ObservedDaemon::{Live(ObservedStartTime), Absent}`, and claims "the match is total by construction
rather than by enumeration". The verdict table's final row is `NoPidRecorded | any | stale → recover`
— but `RecordedDaemon` is a struct with a plain `pid` field, so there is no `NoPidRecorded` value to
match on. The table's left column also mixes two axes: `Probe`/`Wallclock`/`AbsentOrUnparseable` are
`RecordedStartTime` variants, while `NoPidRecorded` is a statement about the record's existence.

*Impact*: The totality claim is the whole justification for putting this logic in the domain crate
behind ports, and it is unsupported by the named types — an implementer will reach for `Option<Pid>`
or an ad-hoc guard, and the "recovery never signals a pid it cannot identify" rule (which keys off
exactly this case) then depends on a branch the compiler cannot prove exhaustive.

*Suggestion*: Name the outer sum type explicitly — e.g.
`RecordedState::{Daemon(RecordedDaemon), NoPidRecorded}` — and redraw the table's left column over
that type so every row is a variant pair and the compiler enforces the totality the plan claims.

🟡 **major** (high) — *The `CuePhraseMatcher` port has no stated signature and a flat const slice
loses the case-sensitivity policy* — **Phase 2 §1 (`cue_phrase_audit.rs`)**

`audit-cue-phrases.sh:39-69` applies a *mixed* case policy: it strips the `[Ii]mplement` line out of
the alternation and greps the rest with `-qiE` (case-insensitive), then greps `[Ii]mplement [A-Z]`
case-**sensitively** so "implement Foo" matches but "implement foo" does not — a distinction its own
header comment calls out as deliberate. The plan specifies only "a `const` slice of cue-phrase
patterns" in the domain that "the adapter compiles", with no per-pattern case flag, and never states
the `CuePhraseMatcher` trait's signature. This is the same defect the plan itself diagnoses two
sections later for `StateStore`: "a port whose return type is unstated cannot carry them".

*Impact*: A flat `&[&str]` compiled uniformly silently loosens the audit (every `implement foo`
starts passing) or silently tightens it (the first three phrases become case-sensitive), and the
drift test against `scripts/extract-work-items-cue-phrases.txt` compares pattern text only, so
neither outcome is caught.

*Suggestion*: Model the pattern source as a value carrying its case policy (e.g.
`const CUE_PHRASES: &[CuePhrase]` with a `case_sensitive` discriminator, or two named const slices),
and state `CuePhraseMatcher`'s signature and return type to the same standard the plan applies to
`StateStore` and `ProcessProbe`.

🟡 **major** (high) — *The three sub-domains are asserted as directories but only one is given a
path, and Phase 6 contradicts that one* — **Phase 2 §1 vs Phase 6 §1**

Phase 2 §1 states the `design` bounded context is "three sub-domains, each a module directory rather
than a prose grouping, so a new module has an obvious home or an obvious rejection", and argues that
naming the third sub-domain "is what makes the layout predict where things go". Only the third is
given a path (`src/runtime/`); the seven modules of *source acquisition* and *document auditing* are
listed as bare files with no directory. Phase 6 §1 then contradicts even the one stated directory,
giving the executor's files as `cli/design/src/executor/` rather than `src/runtime/executor/`.

*Impact*: The mechanism the plan relies on to make future module placement obvious does not exist for
two of the three sub-domains, so the crate will land as a flat module list plus one directory, and the
sibling plan's `platform.rs` has no unambiguous path to land at.

*Suggestion*: Give all three sub-domains explicit directory paths in the module list and correct Phase
6 §1's file list to match, or drop the "each a module directory" claim and state the grouping as a
naming convention over a flat module list. Also note that the shared `ACCELERATOR_BROWSER_*`
variable-name vocabulary is needed by both `credentials.rs` and `leaked_credentials.rs` across two
sub-domains, so name its single owner.

🟡 **major** (high) — *`notify-downgrade-messages.json` is orphaned but kept alive by its own drift
test* — **Phase 2 §3 vs Removal sweep Success Criteria**

Phase 2 §3 moves both path-relative data files into domain `const`s with `include_str!` confined to a
`#[cfg(test)]` drift test "asserting the on-disk file still agrees". That reasoning holds for
`scripts/extract-work-items-cue-phrases.txt`, which `extract-work-items` genuinely shares. It does not
hold for `skills/design/inventory-design/scripts/notify-downgrade-messages.json`: its only reader is
`notify-downgrade.sh:29-36`, which Phase 2 §6 deletes, so after this phase the file exists solely so a
test can assert something agrees with it. The Removal sweep's own criterion — "Only
`ensure-playwright.sh` and the retained JavaScript remain under `skills/design/**/scripts/`" — is then
false while the JSON survives, and no section lists it for deletion.

*Impact*: Either the plan ships data with no consumer plus a self-referential test that can only fail
when someone edits the orphan, or the Removal-sweep criterion fails on merge — and which of the two
happens is left to the implementer.

*Suggestion*: State explicitly that `notify-downgrade-messages.json` (and the
`evals/fixtures/notify-downgrade/*.expected.txt` set `test-notify-downgrade.sh:9` reads) is deleted in
Phase 2 with the script, and scope the `include_str!` drift test to the cue-phrase file alone, whose
shared-canonical-source claim justifies it.

🟡 **major** (medium) — *`FreeSpace` is a port for behaviour this plan explicitly leaves in shell* —
**Phase 6 §1**

Phase 6 §1 adds a `FreeSpace` port. But `run.sh` — the 203 lines this phase ports — contains no
free-space logic at all; the 500 MB floor and the `disk-floor-not-met` downgrade reason live entirely
in `ensure-playwright.sh:45,150-155`, which this plan retains by design. The port is also missing from
the Testing Strategy's own list, which enumerates seven ports and omits it.

*Impact*: An eighth port, its adapter and its tests land in the plan's most regression-prone module for
behaviour that is not being migrated — the port either sits unused (dead abstraction) or quietly
duplicates a check the surviving shell script still performs, giving two disk floors that can disagree.

*Suggestion*: Drop `FreeSpace` from this plan and let it arrive with the sibling plan that moves
bootstrapping into Rust — or, if the executor really is meant to pre-check free space, say so as a new
behaviour with its own criterion, and reconcile the Testing Strategy's port list either way.

🟡 **major** (medium) — *The Drop-guard lock release and the `exec`-based `RunClient` are mutually
exclusive on the hot path* — **Phase 6 §1 (Lock) and §3**

Phase 6 resolves the previous draft's lock contradiction by settling on "The lock is released at
launcher exit, by a `Drop` guard, **on every path**". The same phase specifies the `RunClient`
adapter's "terminal implementation is `CommandExt::exec`" — and `exec` replaces the process image
without unwinding, so no destructor runs. On the reuse and post-spawn paths (the ones a crawl takes
100–200 times) the lock is released by `O_CLOEXEC` closing the descriptor, not by the guard; the guard
only fires on the error paths.

*Impact*: The stated release mechanism is not the actual one on the dominant path, and the actual one
is an implicit property of Rust's default open flags — so a future maintainer who replaces `exec` with
spawn-and-wait, or clears `FD_CLOEXEC` to inherit a descriptor, silently changes lock lifetime back to
run.sh's leaked-FD behaviour that `test-run.sh:160-163` documents as blocking the next launcher.

*Suggestion*: State both mechanisms explicitly — the guard covers every non-`exec` exit, and
descriptor close-on-exec covers the `exec` path — and add a criterion asserting the lock is observably
free immediately after a successful executor command, so the invariant is pinned by a test rather than
by a default flag.

🟡 **major** (high) — *Rationale is duplicated up to four times, and one copy has already gone stale
within a section* — **Plan-wide: Phase 2 §1, Phase 6 §1/§3/§4/§5/§6, Removal sweep §4**

Several decisions are restated near-verbatim in multiple places: the `makeAuthHeaderHandler` dead-path
rationale appears four times; the reserved-`run.js`-token rejection twice; the
`test:unit:design-automation` floors twice inside §6; and the `releases-and-compatibility.md:41-44`
rationale twice. One duplicate has already drifted: Phase 6 §4 first requires the locale guard to
"additionally assert agreement with the value `lib/state.js` writes for the same process" and then, two
paragraphs later, states "What the guard no longer does is compare against `lib/state.js`" — with the
success criterion siding with the second. A dangling cross-reference exists too: Phase 6 §6 cites
"Phase 8 §1" for the re-homed `test-design.sh` assertions, and there is no Phase 8 in this plan,
directly against the plan's claim that "the cross-references are correct".

*Impact*: The plan's own diagnosis is that "a single missed reference is precisely the defect class
three review passes kept finding in this material"; four copies of one rationale guarantee the next
revision updates one and leaves three, and §4 already asks the implementer to write a test the same
section forbids.

*Suggestion*: Keep one authoritative statement per decision at the point of change and replace the rest
with bare cross-references; delete §4's first `state.js`-agreement sentence; and repoint "Phase 8 §1"
at "Removal sweep §1".

🔵 **minor** (high) — *Phase 1's success criteria put the `From` mapping test in a crate the phase says
it does not touch* — **Phase 1: Overview, §4, and Automated Verification**

Phase 1's Overview says "Nothing in `corpus` or `corpus-adapters` changes", and §4 says "Changes: None
to the renderer, and **no `FakeClock` test**". Its success criteria then require
"`cargo nextest run -p corpus-adapters` passes … and a new assertion on the
`From<FilenameTimestampFormatArg>` mapping". `FilenameTimestampFormatArg` is CLI-local by §1
(`cli/corpus-cli/src/cli.rs`, package `accelerator-corpus`), and `corpus-adapters` does not and cannot
depend on the binary crate, so the assertion cannot live where the criterion places it.

*Impact*: The only genuinely new behaviour in the phase — the argument-to-variant mapping the plan
correctly identifies as the real risk — has no valid home stated.

*Suggestion*: Move the mapping assertion into the `-p accelerator-corpus` criterion beside the
`compact-time` golden, and reduce the `corpus-adapters` criterion to "passes unchanged, with AC15's
byte-for-byte claim pointed at the existing `format_filename_timestamp` test".

🔵 **minor** (medium) — *`Allowances` is the right shape for the wrong stated reason, and is named
after CLI flags* — **Phase 2 §1 (`access_policy.rs`)**

Bundling the two booleans into `Allowances { internal, insecure_scheme }` is the right call — it avoids
`evaluate(&location, true, false)` — but the justification given ("The two flags only ever travel
together and are only meaningful as a pair") is not true of the domain: `--allow-internal` relaxes a
*host-reach* judgement and `--allow-insecure-scheme` relaxes a *scheme* judgement, and they are
independent. They travel together only because they are flags on one subcommand, and the type is named
after the flag set rather than after the domain concept.

*Impact*: Justifying a parameter object with "meaningful only as a pair" licenses every future
`--allow-*` flag to accrete into the same struct, turning a value type into the CLI's flag bag and
leaking argument-parsing shape into the domain the plan is otherwise careful to keep pure.

*Suggestion*: Keep the parameter object but restate the rationale, and consider a domain-facing name
(e.g. `InspectionPermissions`) with fields named for what is permitted rather than for the flags that
set them.

🔵 **minor** (medium) — *The plan asks for comments where a test or a name could carry the fact* —
**Phase 6 §6**

Phase 6 §6 requires the new suite floors to be landed "with the deletions recorded in the comment" and
"an executed-count floor recorded with its derivation". The repo's standing convention holds comments
to a last resort and specifically warns that references to migration context go stale fast — and this
is exactly that shape: a comment naming `identity.test.js` and `lock.test.js` as the reason the floor
is 8 becomes misleading the moment `playwright-loader.test.js` goes with the sibling plan (which §6
already anticipates).

*Impact*: The derivation comment will disagree with the number beside it within one plan, and a reader
trusting it will compute the wrong new floor.

*Suggestion*: Let the discovered-file list passed to `node --test` and the assertion itself carry the
fact (a named constant plus a test that fails loudly with the actual-versus-expected list), and drop
the derivation comment; if provenance is genuinely wanted, put it in the assertion's failure message
where it cannot rot unobserved.
### Security

**Summary**: The plan is unusually strong on the classic migration risks — it deliberately
re-derives SSRF classification from first principles rather than transcribing the shell regexes,
correctly identifies that the shell's own `--allow-internal` gate makes the SSRF ceiling bounded,
adds a request token to close the daemon's unauthenticated-localhost hole, and refuses to broaden
`scrub-secrets`' redaction on the correct grounds. Its weaknesses are concentrated in the parts
where security-relevant behaviour is *added* rather than ported: the token's threat model is
stated but its generation, comparison, transport and lifecycle are not, and the same holds for
the token's interaction with the reuse verdict. Two ported paths also lose protection quietly —
the `umask 077` that today makes every daemon-written artefact owner-only, and the
strict-parsing rejection whose new sensitivity to non-numeric hostnames is never bounded. Finally,
the `scrub-secrets` port narrows a redaction that the shell applies to a wider input than the plan
credits.

**Strengths**:

- Re-deriving reachability from `IpAddr` parsing rather than transcribing the shell's regexes is
  the correct decision and is justified with concrete gaps that were verified against
  `validate-source.sh:56-77` (`::ffff:10.0.0.1`, `fc00::/7`, non-first-octet octal all genuinely
  slip through today).
- The plan correctly bounds the SSRF exposure: `--allow-internal` is a user-supplied flag, so a
  tightened classifier is defence-in-depth rather than the only control, and the reasoning is
  stated rather than assumed.
- Adding the request token closes a real hole: `daemon.js:236-303` accepts any localhost
  connection with no authentication today, and the browsing session it drives may hold
  authenticated cookies.
- Splitting the auth-header variable at `:` so the *value* alone is scanned is a genuine
  improvement over `test-design.sh:316-338`'s whole-value comparison, and the plan names the
  leakage shape it catches.
- Refusing to widen `scrub-secrets`' redaction beyond
  `ACCELERATOR_BROWSER_{USERNAME,PASSWORD,AUTH_HEADER}` is right — a broader sweep over the
  environment would produce false redactions in design artefacts and is not what the shell does.
- Keeping the `0700` mode on the state directory and the `0600` mode on the bootstrap log as
  explicit modes rather than umask side effects is the correct instinct for a Rust port.

**Findings**:

🟡 **major** (high) — *The request token has no specified generation source, comparison
discipline, or transport* — **Phase 6 §3 ("The daemon gains a request token")**

The plan states the launcher "generates a random token", `state.js` publishes it in the `0700`
`server-info.json`, `client.js` sends it and `daemon.js` requires it. Everything security-relevant
about that is unstated: the entropy source and length; whether comparison is constant-time
(`daemon.js` currently does `===` on strings, and a length-varying early-exit compare over a
loopback socket is a weak but real oracle); whether the token travels in the JSON body (and
therefore into `bootstrap.log` or any request logging) or in a header; and whether it is
regenerated per daemon or per request. The port set (`Clock`, `ProcessProbe`, `StateStore`,
`Lock`, `Spawner`, `ProcessControl`, `RunClient`, `FreeSpace`) contains no randomness port, so the
value is also not deterministically testable — while AC2 requires volatile inputs to arrive
through ports.

*Impact*: A token specified only as "random" is the shape that ships as `Math.random()` or a
4-byte value, and a token echoed into a `0600`-but-readable-by-the-user log is no barrier against
the local-attacker model the plan names.

*Suggestion*: State the generation (a CSPRNG, ≥128 bits, hex or base64url), require a
constant-time comparison on the daemon side, name the transport field and assert it is never
logged, and add a `TokenSource` port so the value is injectable in tests.

🟡 **major** (high) — *Token enforcement has no defined behaviour for a pre-upgrade or untokened
daemon* — **Phase 6 §3 and Phase 6 §1 (reuse verdict table)**

The reuse verdict table decides reuse purely on pid and start-time identity; the token is not an
input. So on a warm path the launcher may reuse a daemon started by a previous plugin version
whose `server-info.json` carries no token. The plan does not say what happens: if the daemon
requires a token from "its first accepted connection" it will reject its own launcher's client; if
the launcher omits the token when the record lacks one, an attacker who can cause a token-less
record to be read gets unauthenticated access — and the record is in a `0700` directory whose
*parent* (`config path tmp`) mode the plan never states.

*Impact*: Either the upgrade path breaks (a running daemon becomes unusable with no recovery rule,
since nothing in the table treats a token mismatch as stale) or token enforcement is silently
downgradeable, which is the standard way an added authentication control becomes decorative.

*Suggestion*: Make the token part of the recorded identity: add it to `RecordedDaemon`, add a
verdict row for "record has no token → stale, recover" (a one-off respawn on upgrade, which the
plan already accepts for other reasons), and state the mode of every directory on the path to
`server-info.json`.

🟡 **major** (medium) — *The daemon's inherited `umask 077` is dropped, so daemon-written artefacts
lose owner-only permissions* — **Phase 6 §1 ("Three behaviours an earlier draft's inventory
missed")**

`run.sh:3` sets `umask 077` for the launcher *and every process it spawns*, including the daemon.
The plan carries that forward only for two specific launcher-side files (the `0700` state dir, the
`0600` bootstrap log) and says nothing about the spawned daemon's umask. The daemon writes
screenshots via `page.screenshot({ path })` (`lib/daemon.js:221-233`) and any file a future
command adds; today those land `0600` because of the inherited umask, and under a Rust launcher
they would land at the ambient umask (commonly `022`, world-readable).

*Impact*: Screenshots of authenticated pages — the exact artefacts the plan's own threat framing
calls sensitive — become world-readable on shared or multi-user machines, and the loss is invisible
because no test asserts the mode.

*Suggestion*: Set `umask(0o077)` in the spawned child via `pre_exec` (or set the mode explicitly on
every daemon-written path), and add a criterion asserting a daemon-written screenshot is `0600`.

🟡 **major** (medium) — *The `scrub-secrets` port narrows redaction from all environment-derived
credentials to three named variables* — **Phase 2 §1 (`credentials.rs` / `leaked_credentials.rs`)**

The plan describes the redaction set as the three `ACCELERATOR_BROWSER_*` variables. But
`scrub-secrets.sh` also redacts values it derives from those variables — notably the base64 of
`username:password` that the auth handler constructs — and the artefact may contain that encoded
form rather than the raw password. The plan's own §1 reasoning about the auth header ("an artefact
rendering just the bearer token … matches nothing") applies with equal force to the derived Basic
credential, and it is not carried across.

*Impact*: An artefact that leaks the Basic credential in its wire form passes `scrub-secrets`
cleanly, which is precisely the failure the subcommand exists to prevent.

*Suggestion*: Enumerate the full redaction set as domain values — each configured credential plus
its derived encodings (Basic base64 of `user:pass`, the bare header value) — and add a table test
per shape.

🟡 **major** (medium) — *Strict numeric parsing is specified as a rejection without bounding what
it rejects* — **Phase 2 §1 (`host.rs`)**

The plan requires that "any host that *looks* numeric but fails strict parsing" is rejected. What
counts as "looks numeric" is never stated, and the boundary is genuinely delicate: `1.2.3.4.5`,
`0x7f.1`, `2130706433`, `10.0.0.1.example.com`, an IDN label of digits, a trailing dot. A
predicate drawn too wide rejects legitimate hostnames (a subdomain that is entirely digits is
valid DNS); drawn too narrow it readmits the decimal and octal encodings the rejection exists to
close.

*Impact*: The security control is under-specified at exactly the point where an implementer's
judgement decides whether it works, and no criterion lists the accepted-versus-rejected boundary
cases.

*Suggestion*: State the predicate concretely — e.g. "if every label is composed only of digits, or
the host contains a `0x`/`0`-prefixed numeric label, it must parse as an `IpAddr` or be rejected"
— and pin the boundary with a table test including at least the six shapes above.

🟡 **major** (medium) — *The reserved-range set is hand-enumerated with no drift guard* — **Phase 2
§1 (`host_reach.rs`)**

The plan lists `100.64.0.0/10`, `192.0.0.0/24`, `198.18.0.0/15`, `240.0.0.0/4` and `fc00::/7` as
ranges to classify. Hand-enumerated CIDR tables are a known drift surface (Rust's
`Ipv4Addr::is_private` and friends deliberately do not cover them, which is why the enumeration
exists), and nothing in the plan pins the set against an authoritative source or asserts a
representative address per range.

*Impact*: A mistyped prefix length silently opens a range — `198.18.0.0/15` written as `/16`
leaves half the benchmarking range classified public — with no test able to notice.

*Suggestion*: Add a table test with at least one in-range and one just-out-of-range address per
CIDR, and cite the RFC per row in the module doc so a future reader can re-derive rather than
trust.

🟡 **major** (medium) — *Pre-resolution classification is recorded as a limit but its consequence is
not carried into the plan's claims* — **Phase 2 §1 (`host_reach.rs` module docs)**

The plan records that classification happens pre-resolution and applies to the initial location
only, and raises the unclassified `navigate` surface as a follow-up. Good. But it then treats
`host_reach` as the SSRF control in its Success Criteria and docs updates, without stating the
residual: a public hostname resolving to `127.0.0.1` (DNS rebinding, or simply a hosts entry) is
accepted with no `--allow-internal`, and the daemon then fetches it.

*Impact*: Readers of the docs change ("internal-reach-recoverable addresses versus unconditional
numeric-encoding rejections") will reasonably infer that internal addresses require the flag, which
is untrue for any name that resolves to one.

*Suggestion*: State the residual explicitly in the module doc and the docs page, and note whether
post-resolution checking is in the follow-up's scope (it is the only place it can live, since the
fetch happens in the daemon).

🟡 **major** (low) — *The `design.browser_path` hazard is named in the sibling plan but this plan
ships the crate that will read it* — **Phase 2 §1 (`runtime/`) and cross-plan seam**

The sibling plan correctly identifies that `design.browser_path` is settable from repo-tracked
`.accelerator/config.md`, so opening an untrusted repository can name a binary that a skill then
executes, and files it as a follow-up. This plan creates `cli/design/src/runtime/` — the module that
will hold that value — and says nothing about it, so if this plan's runtime work lands first the key
can be read with no restriction recorded anywhere in the plan that owns the code.

*Impact*: A security constraint that exists only in a sibling plan's follow-up list is a constraint
with no owner in the code that implements it.

*Suggestion*: State in this plan's `runtime/` section that the config-sourced browser path is
untrusted input whose restriction is tracked by the sibling's follow-up, so the code lands with the
constraint visible at its site.

🔵 **minor** (medium) — *Rejecting internal `run.js` tokens by allowlist changes an outcome without
naming the security rationale's limit* — **Phase 6 §3**

Rejecting the `daemon` token is right (verbatim forwarding lets a caller start a daemon with
arbitrary `--state-dir`). But the plan implements it as an allowlist of forwardable commands, and does
not state whether argument *values* are also validated — `--state-dir` is still forwardable on other
commands if the allowlist covers only the leading token.

*Impact*: The hazard is a caller-controlled state directory, and a command-name allowlist alone does
not close it.

*Suggestion*: State that launcher-owned flags (`--state-dir` and any other path the launcher derives)
are rejected if supplied by the caller, on every forwarded command.

🔵 **minor** (medium) — *`bootstrap.log` truncation is specified without stating who may read it* —
**Phase 6 §1**

The log is created `0600` and truncated per spawn, and the `daemon-start-timeout` envelope names its
path. Nothing states whether the daemon's stderr may contain the token, the auth header, or a target
URL with credentials — all three are plausible from Playwright/Node stack traces.

*Impact*: A `0600` file is still readable by the user's own other tooling and by anything that
uploads diagnostics, and the plan makes its path user-visible in an error envelope.

*Suggestion*: State that the token and credential values must never be written to the bootstrap log,
and add an assertion over the spawned child's environment-to-log path.

🔵 **minor** (low) — *The state directory's parent mode is unstated* — **Phase 6 §1**

The plan states `0700` for the state dir and `0600` for the log, and derives the location from "the
repo's config tmp path". The mode of that tmp path — and whether it is created by this code or
assumed — is never stated.

*Impact*: A `0700` directory inside a world-writable parent is still subject to the usual
create-then-swap races on a multi-user machine.

*Suggestion*: State the required mode and ownership of the tmp root, and refuse if it is not owned by
the current user.

🔵 **minor** (low) — *`0700`/`0600` are stated as literals with no umask interaction note* — **Phase
6 §1**

Rust's `create_dir_all`/`OpenOptions` apply the process umask to the requested mode, so a `0700`
request under an unusual umask yields something narrower (harmless) but a `0666`-style request would
yield something wider. The plan removes the `umask 077` that today made this moot without saying the
modes are now set explicitly with `PermissionsExt` after creation.

*Impact*: Small, but this is exactly the class of detail the port is most likely to get subtly wrong.

*Suggestion*: State that modes are applied explicitly post-create (or via
`OpenOptionsExt::mode` plus a `set_permissions` follow-up) rather than relying on the umask.

🔵 **suggestion** (medium) — *A token gives the daemon an opportunity to bind more narrowly and drop
privileges it never needed* — **Phase 6 §3**

Since the plan is already changing the daemon's connection acceptance, it is the natural moment to
state two adjacent hardening steps: that the listener binds `127.0.0.1` explicitly (not `localhost`,
which can resolve to `::1` and widen the accepted set) and that the port is never written anywhere
world-readable.

*Suggestion*: Record both as criteria in §3 so the token change carries the full localhost-hardening
story rather than authentication alone.

### Compatibility

**Summary**: The plan is unusually careful about the contracts it must not break — it enumerates
`run.sh`'s exit codes and stdout/stderr split, preserves the `ACCELERATOR_DOWNGRADE_REASON` stderr
protocol, keeps `kernel::Error::Refusal → 2` consistent with the other sub-binaries, and states the
`allowed-tools` rewrite for both skills. The compatibility risks that remain are the ones the plan
creates: the daemon acquires a mandatory spawn contract that two retained Node suites violate, the
`Verdict` carrier cannot express the executor's four exit codes, the allowlist changes the
`unknown-command` outcome from a daemon-side stdout envelope to a launcher-side stderr error, and the
`scripts/*` `allowed-tools` glob is edited in two plans against the same lines.

**Strengths**:

- Exit-code semantics are enumerated per subcommand against the shell sources rather than assumed,
  and the cross-binary `Refusal → 2` contract (`corpus-cli:132`, `vcs-cli:77`) is preserved rather
  than bent.
- The `ACCELERATOR_DOWNGRADE_REASON=<reason>` stderr protocol that SKILL.md parses is explicitly
  named as a preserved contract, with the six reason strings pinned as domain constants.
- The launcher-versus-daemon stdout/stderr asymmetry (daemon errors are envelopes on stdout at exit
  0; launcher errors go to stderr with a non-zero code) is identified as a contract two browser
  agents and ~40 call sites depend on, and committed to.
- The `allowed-tools` rewrite is stated for both SKILL.md files, with the residual `scripts/*` glob's
  ownership assigned to the sibling plan rather than left ambiguous.
- Keeping `ensure-playwright.sh` and the lockhash namespace unchanged means an existing populated
  cache keeps working across this plan — the migration is invisible to a user mid-project.

**Findings**:

🟡 **major** (high) — *The daemon gains a mandatory spawn contract that two retained suites violate*
— **Phase 6 §1/§3 vs §6**

After Phase 6 the daemon depends on the launcher for its identity (`state.js` no longer computes a
start time) and for the request token. `test-run.js:68` and `lib/daemon.test.js:77,99,120,141,174`
fork `run.js daemon --state-dir <dir>` directly, with no launcher and therefore neither value. Both
files are retained, and `daemon.test.js` is in the lane Phase 6 §6 requires to pass with zero skips.

*Impact*: A previously-supported invocation (`run.js daemon`, which is also how a developer
diagnoses a daemon by hand) becomes unsupported with no stated behaviour, and the retained suites
break.

*Suggestion*: Define the daemon's contract for absent identity values explicitly — refuse to start,
or self-probe as a documented fallback — and update both suites to supply the environment.

🟡 **major** (high) — *The `Verdict` carrier cannot express the executor's exit codes* — **Phase 2 §3
vs Phase 6 §2**

`Verdict<Reason>` has two variants mapping to 0 and 1 with `Refusal → 2`. The executor must produce
0 (including daemon error envelopes on stdout), 1 (`another-launcher-running`,
`daemon-start-timeout`), 2 (`no-repo`), 3 (`playwright-not-installed`), plus pass-through of the
client's own exit status and signal death (128+n).

*Impact*: One binary would carry two contradictory exit-mapping stories, and "one render-and-exit
function" — the property that makes a wrong code unlikely — stops holding for the subcommand where a
wrong code matters most.

*Suggestion*: Extend the carrier to cover exit 3 and the exec pass-through, or declare the executor a
documented exception with its own terminal path, and add a table mapping each envelope to its code.

🟡 **major** (high) — *The forwardable-command allowlist changes the `unknown-command` contract* —
**Phase 6 §3**

Today an unrecognised command reaches the daemon and returns `{"error":"unknown-command"}` on
**stdout at exit 0** (`daemon.js:279-281`) — the asymmetry §2 commits to preserving. An allowlist
intercepts it at the launcher and produces a stderr usage error at exit 2. The allowlist must also
track `daemon.js`'s eleven commands, and the sync assertion the plan extends lives in the Node
suites, which cannot see Rust source.

*Impact*: Consumers that parse stdout JSON get no envelope for a typo, and a command added to
`daemon.js` is unreachable until a Rust release.

*Suggestion*: Reject only the internal `daemon` token (a denylist suffices for the stated hazard), or
keep the allowlist and pin it against `daemon.js`'s command set with a cross-language test.

🟡 **major** (medium) — *`scripts/*` `allowed-tools` lines are edited by both plans* — **Phase 2 §5
and the sibling plan's Phase 7 §6**

This plan rewrites both SKILL.md `allowed-tools` blocks to name the new binary; the sibling plan
drops the residual `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/**/scripts/*)` rule. The corresponding
`test-design.sh:154-155` assertion is likewise split across the two plans.

*Impact*: Whichever lands second rebases onto changed lines in a file the other also rewrote, and the
assertion's `SKILL=` definition (line 140) is re-homed by this plan while `154-155` stays — so a
mechanical application leaves the suite aborting on an unbound variable.

*Suggestion*: State the exact final content of both `allowed-tools` blocks in this plan, mark the
`scripts/*` line as the sibling's single remaining edit, and keep `153-155` together.

🟡 **major** (medium) — *`_EXPECTED_CONFIG_SUITES` is left inconsistent through the interim* —
**Removal sweep §1 and the sibling plan's arithmetic**

This plan leaves `scripts/` at 16 discovered suites against a floor of 15, on the stated
understanding that the sibling's deletion of `test-design.sh` brings it to 15. But
`scripts/test-metadata-helpers.sh` is also deleted (Phase 3 must delete it — see the test-coverage
lens), taking the count to 15 within this plan and 14 in the sibling's.

*Impact*: Both plans' floor arithmetic is off by one, and the sibling would land 14 actual against a
floor of 15 — a red lane on merge.

*Suggestion*: Restate the arithmetic in both plans with `test-metadata-helpers.sh` counted, and record
the sibling's `_EXPECTED_CONFIG_SUITES` decrement to 14 as inherited work.

🟡 **major** (medium) — *`bin/accelerator` dispatch and the `ACCELERATOR_DESIGN_BIN` override are not
specified* — **Phase 2 §4 (registration surface)**

The plan completes "the thirteen-point registration surface" by reference. The dev-override variable
name, its interaction with `ACCELERATOR_PLUGIN_ROOT` (which the executor needs to locate `run.js`),
and the `_SUBBINARY_MANIFESTS` entry forced by the `cli/design/` crate name are each mentioned in
passing but never stated as concrete values.

*Impact*: The registration checklist is the one place in this repo where a missed point produces a
binary that resolves in dev and fails in a release, and the plan defers all thirteen to a document
reference.

*Suggestion*: State the binary name, the override variable, the manifest entry and the docs page
explicitly, so the checklist is checkable from the plan.

🟡 **major** (medium) — *The `--filename-timestamp-format` addition is not stated as an additive
contract change* — **Phase 1 §1**

Adding a value enum to `corpus metadata derive` changes a shipped sub-binary's CLI surface. The plan
does not state the default (so existing callers are unaffected) nor whether the flag appears in the
`corpus` docs page and `CHANGELOG.md`.

*Impact*: A CLI surface change with no stated default and no documentation entry is the shape that
breaks a caller on upgrade.

*Suggestion*: State the default explicitly as the current behaviour, and add the docs page and
changelog entry to Phase 1's file list.

🟡 **major** (low) — *No statement of minimum Claude Code version or plugin-version interaction for
the new binary* — **Phase 2 §5**

Rewriting `allowed-tools` to name a dispatched binary depends on the launcher being present at the
version the skill expects. The plan does not state what happens when a skill from a newer plugin
version meets a cached older launcher, which the launcher's own resolution model makes possible.

*Impact*: A mixed-version install produces an unresolvable subcommand at skill-invocation time, with
no stated diagnostic.

*Suggestion*: State the launcher's behaviour for an unknown subcommand and the minimum plugin version
the rewritten skills require.

🔵 **minor** (medium) — *`ACCELERATOR_PLAYWRIGHT_CACHE` precedence is preserved implicitly* — **Phase
6 §1**

The namespace root derivation reads `ACCELERATOR_PLAYWRIGHT_CACHE` with a `$HOME` default
(`run.sh:84-92`), and users have populated caches at those paths. The plan says the executor "still
resolves the existing lockhash namespace" but never states the variable, its default, or that
precedence is unchanged.

*Suggestion*: Name the variable and default explicitly as a preserved contract, with a test.

🔵 **minor** (medium) — *`PROTOCOL.md` ↔ `daemon.js` sync moves to an unnamed file* — **Removal sweep
§1**

`test-design.sh:491-510` asserts the documented command set matches the implementation. The row
re-homes it "→ node suites" with no destination, and the same self-matching hazard applies as for the
other greps.

*Suggestion*: Name the destination and state how the assertion avoids matching its own source.

🔵 **minor** (low) — *The `links` output contract's `same_origin` semantics have no stated owner after
`test-run.sh` is deleted* — **Phase 6 §8**

`test-run.sh:113-192` is the only executable statement of the `links` output shape, including the
opaque-origin cases. The plan deletes the suite without re-homing them.

*Suggestion*: Re-home those assertions into the opt-in lane or a `daemon.test.js` block.

### Safety

**Summary**: The plan is strong on the destructive-operation surfaces it inherits: it identifies
`run.sh:106-121`'s unconditional `rm -f` outside the lock as a live-daemon-orphaning bug and fixes
it with a read-only pre-lock check, it removes the empty-expected accept that defeats the PID-recycle
guard, and it refuses to signal a pid it cannot identify. Where it is weak is in the *new* risks its
own fixes create: the signalling carve-out is drawn so the only signalling row is the one where
signalling is provably wrong, the identity handoff has a window in which a live daemon has no
recorded start time, and the phase ordering leaves two CI lanes red on merge. Nothing here risks
user data in the corpus sense — the destructive surface is daemon state and cached runtime — but a
mid-crawl daemon kill loses accumulated page state, which is the plan's own stated cost.

**Strengths**:

- Identifying that `run.sh:106-121` deletes a live daemon's state *outside* the lock, and fixing it
  with a read-only pre-lock check, removes a real orphaning path rather than a hypothetical one.
- "Recovery never signals a pid it cannot identify" is the correct principle, and it is stated as a
  rule rather than left to the implementation.
- Removing the empty-expected accept (`run.sh:54`) closes a guard bypass that today makes the
  PID-recycle protection vacuous whenever the recorded start time is missing.
- The reuse verdict's `Wallclock` row explicitly records that the recycle guard is weaker there,
  rather than presenting the tolerance as uniform protection.
- Truncating the bootstrap log per spawn with an explicit `0600` mode, rather than appending, bounds
  a file that today grows unboundedly under the inherited umask.
- Not adding automated removal of the legacy cache namespace is the right call and is justified
  against the repo's standing position that VCS/filesystem recovery beats destructive-op UX.

**Findings**:

🔴 **critical** (high) — *The identity handoff leaves a window where a live daemon has no recorded
start time, which the verdict table turns into orphaning it* — **Phase 6 §1/§3**

§1 rejects the launcher-writes-the-record design precisely because readiness *is* `server-info.json`
appearing, so a launcher killed in that window leaves a live daemon whose record has no start time —
and the table's `AbsentOrUnparseable → stale → recover` row then deletes that healthy daemon's state
and spawns a second one. §3 nonetheless specifies exactly that design ("writes the identity record
once the daemon reports ready"). Worse, the mechanism §1 substitutes — handing `(pid, start_time,
…)` to the daemon "through the environment" — is not implementable, because the child's environment
is fixed before the child exists (see the correctness lens), so an implementer will fall back to §3.

*Impact*: The plan's own stated worst case — two daemons on the same state dir, the second inheriting
a crawl mid-flight, accumulated page state lost — is reachable through the design the plan
inadvertently specifies.

*Suggestion*: Settle on one writer and one implementable handoff (inherited pipe, or observe in the
forked child before `exec`), and state the readiness ordering: no `server-info.json` until the
identity values are in hand.

🔴 **critical** (high) — *The signalling carve-out excludes the one row where signalling is provably
wrong* — **Phase 6 §1**

The carve-out exempts `AbsentOrUnparseable` and `NoPidRecorded` from signalling, implying the other
`stale → recover` rows *do* signal. The only such row with a live process is the start-time-mismatch
row — i.e. the case where the mismatch *proves* the pid was recycled and belongs to an unrelated
process. `run.sh` never signals during recovery at all.

*Impact*: SIGTERM to an arbitrary user process (an editor, a build) as a new behaviour introduced by
a port that claims to preserve behaviour.

*Suggestion*: State that recovery never signals on any row; `ProcessControl` is for the
kill-on-timeout of the launcher's own spawned child only. Add a test asserting no signal on every
recover row.

🟡 **major** (high) — *Phase 3 leaves two CI lanes red because it deletes scripts a retained suite
drives* — **Phase 3 §2**

`scripts/test-metadata-helpers.sh:21-24` drives both deleted metadata scripts, and runs in both
`test:unit:templates` (via `tasks/test/unit.py:41`) and `test:integration:config`.

*Impact*: A phase the plan claims is independently mergeable cannot merge green, and the safety net
for the two helpers' output contract disappears with no recorded disposition.

*Suggestion*: Delete the suite and its driver entry in the same phase, with the replacement Rust
coverage named.

🟡 **major** (high) — *The `Drop`-guard lock release does not run on the dominant path* — **Phase 6
§1/§3**

The lock is stated to be released "by a `Drop` guard, on every path", but the success path terminates
via `CommandExt::exec`, which runs no destructors. Release then depends on `O_CLOEXEC` closing the
descriptor — an implicit default, not a stated mechanism. `run.sh:152,202` releases explicitly before
`exec`.

*Impact*: Any release step needing more than an FD close silently never runs, and a future change to
the spawn strategy or the descriptor flags reintroduces the leaked-lock behaviour
`test-run.sh:160-163` documents as blocking the next launcher.

*Suggestion*: Release explicitly immediately before the terminal `exec`, keep the guard for
non-terminal paths, and assert the lock is observably free after a successful command.

🟡 **major** (medium) — *Reuse-on-liveness in a `/proc`-less container has no recovery path* —
**Phase 6 §1**

The `Probe(_)` + `Live(Unavailable)` row reuses on liveness alone. A recycled-but-live pid then sends
the launcher down the client path, where `client.js:44-48` returns `connection-failed` on stdout at
exit 0 and nothing deletes the state — so every subsequent command repeats the verdict.

*Impact*: A wedged state with no self-healing route, in exactly the environment the row was added to
support. Today's behaviour (mismatch → respawn) recovers.

*Suggestion*: Treat an unreachable-but-recorded daemon as stale and recover once, with a test.

🟡 **major** (medium) — *The two-file state record breaks the no-partial-record guarantee the recovery
rule depends on* — **Phase 6 §1**

`writeServerInfo` (`lib/state.js:63-66`) publishes `server-info.json` and `server.pid` as two
independent renames, and the reader requires both (`run.sh:106`). A reader landing between them sees
an incomplete record and, under the lock, deletes a healthy daemon's state.

*Impact*: The window the read-only pre-lock check was meant to close survives inside the lock, where
the deletion is unconditional.

*Suggestion*: Read pid and start time from `server-info.json` alone so the identity record is one
rename; keep `server.pid` as a compatibility artefact.

🟡 **major** (medium) — *`server-stopped.json` removal before spawn has no stated ordering against
the identity write* — **Phase 6 §1**

The inventory lists `server-stopped.json` removal before spawn. Nothing states whether it happens
before or after the state directory is prepared, nor what a reader that sees a stopped marker
alongside a fresh identity record concludes.

*Impact*: A stale stopped-marker can make a healthy daemon look stopped, or vice versa, and the
resolution is left to implementation order.

*Suggestion*: State the full pre-spawn sequence as an ordered list with the invariant each step
establishes.

🟡 **major** (medium) — *Deleting `test-run.sh` removes the only survives-shell-exit assertion* —
**Phase 6 §8**

`test-run.sh` carries the smoke test that the daemon survives its launching shell's exit — the
property `nohup`/`disown`/`setsid` exists to provide, and the one most likely to regress silently in
a Rust port.

*Impact*: The plan's own list of behaviours "an earlier draft's inventory missed" includes this one,
and the assertion that would catch it is deleted in the same phase.

*Suggestion*: Re-home the survives-shell-exit check into the adapter-level integration harness (a stub
`run.js` makes it runtime-free).

🟡 **major** (low) — *No stated bound on repeated respawn* — **Phase 6 §1**

Several verdict rows respawn. Nothing bounds how often, so a persistent failure (a daemon that starts
and immediately dies) produces one spawn per command — 100–200 per crawl.

*Impact*: A crawl against a broken runtime forks hundreds of short-lived Node processes with no
diagnostic that names the pattern.

*Suggestion*: State a bound (e.g. a spawn-failure marker in the state dir that turns the second
consecutive immediate failure into a refusal) or record the absence as deliberate.

🔵 **minor** (medium) — *Bootstrap-log truncation loses the diagnostic for the previous failure* —
**Phase 6 §1**

Truncating per spawn means the log describing why the *previous* daemon died is gone by the time a
user looks, and `daemon-start-timeout` names the path.

*Suggestion*: Keep one previous generation (`bootstrap.log.1`) or truncate only on success.

🔵 **minor** (medium) — *The `0700` state dir is created without an ownership check* — **Phase 6 §1**

If the tmp root is shared or pre-created by another user, a `0700` directory the launcher does not own
is a wedge with a confusing failure.

*Suggestion*: Refuse when the state dir or its parent is not owned by the current user.

🔵 **minor** (low) — *Removal-sweep re-homing is specified by line range with no post-cut verification
of the surviving file* — **Removal sweep §1**

The disposition table cuts ranges from a `set -euo pipefail` file; the `SKILL=`/`153-155` split is one
instance where the surviving file does not run.

*Suggestion*: Add a criterion that `test-design.sh` is executed after each phase's cut, not only at
the end.

🔵 **minor** (low) — *No statement of what happens if two launchers race the state-dir creation* —
**Phase 6 §1**

The lock is acquired after the state dir exists, so directory creation itself is unsynchronised.

*Suggestion*: State that creation is idempotent (`create_dir_all` plus explicit mode) and that mode
enforcement runs on the already-exists path too.

### Performance

**Summary**: The plan's performance reasoning is sound where it engages: it recognises the executor is
on a 100–200-invocations-per-crawl path, it removes per-invocation shell process spawns
(`sha256sum`, `date`, `getconf`, `ps`), and it keeps the warm path free of anything that scales with
work. The gaps are that the plan states no budget for the path it is rewriting, and the two costs it
adds — a launcher-resolved dispatch for a new sub-binary and a per-invocation lockhash recomputation —
are neither measured nor bounded.

**Strengths**:

- The executor's warm path is correctly identified as the hot one, and the design keeps it to local
  reads plus stats with no manifest fetch.
- Replacing the shell's `sha256sum`/`shasum`, `date`, `getconf CLK_TCK` and `ps` invocations with
  in-process Rust removes four process spawns per launcher invocation on a path taken hundreds of
  times per crawl.
- Injecting `Clock` rather than sleeping makes the poll-deadline and start-timeout tests instant,
  which keeps the new unit lane fast enough to stay in the default `mise run`.

**Findings**:

🟡 **major** (medium) — *No warm-path budget is stated for the new sub-binary dispatch* — **Phase 6
§1 and Performance Considerations**

Every executor invocation now goes through the launcher's fetch-verify-cache warm path, which
work-item:0186 measured at **~29.92ms** (down from 125.35ms) — per invocation, on top of the Node
client. At 100–200 invocations per crawl that is 3–6 seconds of pure dispatch overhead that
`run.sh` (a direct bash exec) does not pay. The plan states no budget, no measurement, and no
comparison against the shell baseline.

*Impact*: The migration could make a full crawl measurably slower with no one noticing, because
nothing in the plan asserts a ceiling and the manual steps do not include a timing comparison.

*Suggestion*: State the expected per-invocation overhead against the shell baseline, add a manual
timing step comparing a full crawl before and after, and record the accepted regression explicitly if
there is one.

🟡 **major** (low) — *The per-invocation lockhash recomputation is unbounded and unmeasured* — **Phase
6 §1**

The namespace derives from `sha256(package-lock.json)`. `run.sh` pays a `sha256sum` process spawn;
Rust pays an in-process hash of the same file on every invocation. That is almost certainly faster,
but the plan neither says so nor bounds the file size, and the value is stable across a whole crawl —
so it is recomputed 100–200 times for a result that cannot change.

*Suggestion*: Note the size and measured cost, and consider whether the derived namespace can be
passed in by the caller (the skill already knows it is invoking repeatedly) or cached in the state
dir alongside the identity record.

🔵 **minor** (medium) — *The `FreeSpace` port would put a `statvfs` on the hot path for a check this
plan does not port* — **Phase 6 §1**

If implemented as listed, the port adds a filesystem stat per invocation for a floor that lives in
`ensure-playwright.sh` and is not being migrated.

*Suggestion*: Drop the port (see the architecture lens), or state that it is consulted only on the
cold path.

🔵 **minor** (low) — *Hashing every file unconditionally is stated for the sibling's `verify`, not
here, but the executor's state reads are also unbudgeted* — **Phase 6 §1**

The executor reads `server-info.json` (and today `server.pid`) per invocation. Small, but the plan
enumerates the reads without stating the count, and the two-file read is one of the things the
single-record fix would halve.

*Suggestion*: State the per-invocation syscall inventory for the warm path, so a future change that
adds one is visible.

🔵 **minor** (low) — *The new unit lane's runtime is unstated* — **Phase 6 §6**

Two new test tasks join the default `mise run`, which is already heavy. The plan does not state their
expected runtime.

*Suggestion*: Note the expected wall-clock cost of `test:unit:design-automation` so the default task's
growth is a recorded decision.

🔵 **suggestion** (low) — *The token adds a per-request comparison whose cost is negligible but whose
constant-time requirement should be stated as deliberate* — **Phase 6 §3**

A constant-time compare over a short token is free; saying so removes the temptation to "optimise" it
back to `===`.

*Suggestion*: State the requirement with the note that its cost is negligible.

---

## Re-Review (Pass 2) — 2026-08-12T00:14:23+00:00

**Verdict:** REVISE

Pass 1's plan edit addressed every one of its 13 Recommended Changes; all eight
lenses re-reviewed the plan fresh and confirmed the fixes hold, while surfacing new
issues in the mechanisms the fixes introduced — most consequentially a
self-contradiction in the loopback carve-out and an under-specified pipe wire
format for the launcher-to-daemon identity handoff, both fixed in this pass. Three
majors remain open, all pre-existing from pass 1 and deliberately outside the
original 13 Recommended Changes: `test-run.sh`'s ~20 assertions over the retained
`links` privacy contract are still excluded from the migration checklist with no
replacement, four re-homed grep assertions still risk self-matching inside the tree
they scan, and the bare-`return` detector still cannot distinguish a test body from
a helper function.

### Previously Identified Issues

- 🟢 **Correctness**: Spawn-time identity handoff impossible via environment — Resolved (pipe mechanism)
- 🟢 **Correctness/Architecture/Code Quality/Compatibility**: §3 contradicts §1's writer design — Resolved
- 🟢 **Test Coverage**: `scripts/test-metadata-helpers.sh` bash suite missing from Phase 3 — Resolved
- 🟢 **Safety**: Third `run.sh` call site (`daemon-stop`) unrepointed — Resolved
- 🟢 **Safety/Correctness**: No verdict row for writer-side unavailable start time — Resolved
- 🟢 **Correctness/Architecture/Code Quality**: `RunClient`/`exec` divergence, dead `Drop` guard — Resolved
- 🟢 **Correctness/Architecture/Compatibility**: Lockhash namespace unpinned — Resolved
- 🟢 **Correctness/Architecture**: Executor path inputs have no seam — Resolved (`PathResolution` port)
- 🟡 **Security**: Forwardable-command allowlist bypassable — Resolved
- 🟡 **Security**: `client.js` takes `info.url` verbatim — Resolved
- 🟡 **Security**: IPv4-compatible `::a.b.c.d` misclassified; no host normalisation — Resolved
- 🟡 **Security**: `validate-source` path branch has no containment note — Resolved
- 🟡 **Security/Compatibility**: `Bash(… corpus *)` over-broad — Resolved
- 🟡 **Compatibility**: Token vs. `PROTOCOL.md` v1 stability — Resolved (this pass — PROTOCOL.md now documents the token)
- 🟡 **Compatibility**: `libc`/`sysctl` duplicated when `server.rs:527` already exists — Resolved (this pass — extracted into a new `process-probe` crate rather than depending on `visualiser/server`)
- 🟡 **Compatibility**: `audit-cue-phrases`'s exit-code split never reaches its consumer's retry logic — Resolved (this pass — `analyse-design-gaps/SKILL.md` Step 5 now branches on the exit code)
- 🟡 **Architecture**: Daemon becomes launcher-only-spawnable, breaking `test-run.js`/`daemon.test.js` — Still present (not addressed; `run.js:18`'s dispatch is unchanged by this pass's edits, so the direct-`fork` callers this finding names are unaffected either way)
- 🟡 **Architecture/Code Quality**: `FreeSpace` port for behaviour left in shell — Resolved (port dropped)
- 🟡 **Architecture** (suggestion): `Verdict<Reason>` belongs in `kernel`, not `design-cli`, as a reusable primitive — Not addressed
- 🟡 **Test Coverage**: 153/154 table split leaves `$SKILL` unbound — Resolved
- 🟡 **Test Coverage**: Four re-homed grep assertions self-matching — Still present
- 🟡 **Test Coverage**: `leaked_credentials`'s new value-half split has no dedicated test criterion (a review-1 cross-cutting theme: the same treatment `host_reach` gets, `leaked_credentials` didn't) — Resolved (this pass — a criterion mirroring `host_reach`'s was added)
- 🟡 **Test Coverage/Safety**: No automated anchor for downgrade message text — Not addressed
- 🟡 **Test Coverage**: `test-run.sh` excluded from migration checklist, dropping 20 assertions — Still present
- 🟡 **Test Coverage**: Bare-`return` grep cannot distinguish test body from helper — Still present
- 🟡 **Test Coverage** (minor): opt-in integration lane runs in no CI job — Not addressed
- 🟡 **Test Coverage** (minor): mutation-gate wording doesn't exempt deliberate-drop rows — Not addressed
- 🟡 **Test Coverage** (minor): migration-checklist artefact has no stated path or completeness check — Not addressed
- 🟡 **Safety**: Lock release removes mutual exclusion on daemon existence — Not addressed (accepted tradeoff, per the Tradeoff Analysis above)
- 🟡 **Safety**: Kill-on-timeout identity depends on `setsid`-or-double-fork — Resolved (`setsid` only, committed)
- 🟡 **Safety**: Mutation gate covers structural re-homes but not Phase 2's SSRF assertions — Resolved
- 🟡 **Safety** (minor): daemon-inherited `umask 077` — screenshots would land at the caller's umask — Resolved (this pass — `umask(0o077)` in `pre_exec` on the child, with a Success Criteria checkpoint)
- 🟡 **Safety** (suggestion): kill-on-timeout has no `SIGKILL` escalation if `SIGTERM` is ignored — Not addressed
- 🟡 **Safety** (suggestion): dropping the mkdir lock backend is hedged rather than settled against an NFS requirement — Not addressed
- 🟡 **Performance**: "Net not obvious" should resolve to a clear ratio-calibrated gate — Resolved (further corrected this pass — the specific ms figure is now a recorded observation, not the pass/fail threshold, since its derivation compares unlike cost bases)
- 🟡 **Performance**: Latency criterion unwritable, baseline deleted by same phase — Resolved
- 🟡 **Performance** (minor): stated per-crawl savings has an arithmetic slip (100×20ms=2s, not 3s) — Resolved (the "3–9s per crawl" claim was dropped entirely during the ratio-gate rewrite, so the wrong arithmetic no longer appears)
- 🟡 **Performance** (suggestion): lockhash recomputation cost is unbounded and unmeasured — Resolved (noted as negligible at the shipped lockfile's actual size)
- 🟡 **Performance** (suggestion): warm-path measurement is a one-shot check mislabelled as a repeatable CI gate — Resolved (now stated explicitly as a one-time, committed-artefact measurement)
- 🟡 **Code Quality**: Rationale duplicated, dangling "Phase 8 §1" reference — Resolved
- 🟡 **Code Quality**: `notify-downgrade-messages.json` orphaned but kept alive — Resolved (relocated into the crate's own test fixtures)
- 🟡 **Code Quality/Architecture**: Three sub-domains asserted but only one has a path — Not addressed
- 🟡 **Code Quality** (minor): `design_adapters::process` risks becoming an unfocused module backing three unrelated ports — Not addressed
- 🟡 **Code Quality** (minor): Phase 6 concentrates an unusually large amount of new surface into one phase — Not addressed
- 🟡 **Correctness**: Recovery no-signal carve-out excludes the recycled-pid row — Resolved (recovery signals on no row, not on one — corrected from an intermediate mistake made and caught within this pass)
- 🟡 **Correctness/Compatibility**: `validate-source`'s reserved ranges have no home; `localhost` carve-out unstated — Resolved

### New Issues Introduced

- 🔴 **Correctness/Security** (fixed this pass): the `Loopback` carve-out I added for `::1` directly contradicted a pre-existing success-criteria bullet listing `0:0:0:0:0:0:0:1` (the same address, fully expanded) as "newly-rejected" — caught by the correctness re-review, fixed in both locations it appeared.
- 🔴 **Correctness/Architecture/Safety** (fixed this pass): the pipe-based identity handoff I introduced to fix the spawn-time-environment critical had no stated wire format, no stated fd-close discipline on either side, and no stated daemon behaviour for a launcher crash between spawn and the pipe write — three related gaps caught by correctness, architecture and safety respectively, all closed with one coordinated edit (fixed field encoding, explicit `O_CLOEXEC` discipline on the write end, and a daemon-side fail-fast on truncated input before any browser is created).
- 🔴 **Architecture/Compatibility** (fixed this pass): "promotes that implementation into a shared crate (or a design-adapters-visible module)" left the destination undecided in a way that could have pulled `visualiser/server`'s full web-server dependency graph into a hot-path-sensitive crate — committed to a new `libc`-only `process-probe` crate instead, and recorded as a third open fork.
- 🔴 **Security** (fixed this pass): the restated token threat model, security-critical decisions (Origin-header rejection, header-only transport) had no automated-verification criterion — added two Success Criteria bullets.
- 🟡 **Correctness** (fixed this pass): a `start_time_source` value present but neither `"probe"` nor `"wallclock"` had no stated mapping — fixed by folding it into `AbsentOrUnparseable`.
- 🟡 **Correctness** (fixed this pass): the Teredo unwrap step didn't account for RFC 4380's XOR obfuscation of the embedded IPv4 address — a literal-bits extraction would misclassify every real Teredo address against the wrong target. Stated explicitly and folded into the `host_reach` test criterion.
- 🟡 **Correctness/Security** (fixed this pass): the "looks numeric but fails strict parsing" rejection rule had no concrete predicate — flagged independently by both the correctness (suggestion) and security (major) lenses. Stated concretely: every dot-separated label composed entirely of digits, or a host containing `:`, must parse as an `IpAddr` or be rejected outright.
- 🟡 **Security** (fixed this pass): `is_multicast` had no `HostReach` bucket and `Unspecified`'s recoverability was unstated — multicast folds into `Reserved`, `Unspecified` is unconditionally rejected.
- 🟡 **Security** (fixed this pass): the restated token threat model still framed the token as closing only browser-origin CSRF/DNS-rebinding, omitting that a loopback TCP socket is not uid-scoped — the token is also the only control closing a *different local user* on a shared host reaching the daemon's port directly. Threat model restated as two-part.
- 🟡 **Security** (minor, fixed this pass): `client.js`'s loopback validation of `info.url` had no stated implementation — specified as parsing the URL and checking the hostname via Node's `net.isIP`, not a naive string comparison.
- 🟡 **Security** (minor, fixed this pass): the `HostReach` section's opening bullet still said "five variants," contradicting the "six variants" correction two paragraphs later in the same section — updated to match.
- 🟡 **Security** (minor, not addressed): the token's CSPRNG generation has no injected test seam, unlike every other volatile input in this phase.
- 🟡 **Test Coverage** (fixed this pass): the newly-added stub-child-process harness covered setsid/redirection/log-mode/exit-status/timeout/contention but never mentioned exercising the pipe-based identity handoff itself — the property the plan calls its principal silent-regression risk. Extended the stub to read and echo the pipe's contents before deciding whether to signal ready.
- 🟡 **Test Coverage** (fixed this pass): `leaked_credentials`' new value-half split (matching the value component of a `Name: value` pair, not just named variables) had no dedicated test criterion — the exact gap review-1 named as a cross-cutting theme for `host_reach` but noted was never extended to this subcommand. A parallel criterion was added.
- 🟡 **Code Quality** (fixed this pass): the `RunClient` diverging-signature contract and the `ExecutorOutcome`/`Verdict<Reason>` split both need in-code documentation (rustdoc) that the plan cannot itself guarantee survives into the shipped code — left as an explicit note rather than silently assumed.

### Assessment

The plan is materially stronger than pass 1: every critical is resolved, and the
mechanisms introduced to resolve them (the pipe handoff, the `process-probe` crate,
the restated token model) are now specified with the same rigour the plan applies
elsewhere, after a round of self-correction that caught and fixed a genuine
contradiction I introduced. What keeps the verdict at REVISE is unchanged in kind
from pass 1, not new in substance: three test-coverage majors around the
`test-design.sh`/`test-run.sh` teardown's completeness (the excluded `links`
assertions, the self-matching greps, the bare-`return` detector) were named in pass
1, were not among the 13 Recommended Changes selected for this round, and remain
open. They are mechanical rather than architectural — each has a concrete
suggested fix in the Per-Lens Results above — and could reasonably be picked up in
a focused follow-up pass rather than blocking on a third full re-review. A number
of minor and suggestion-level findings from this pass's eight lenses were also left
unaddressed by choice, consistent with the skill's guidance not to force every
finding — they are listed above rather than silently dropped, so the record is
complete even where the plan is not yet.

---

## Re-Review (Pass 3) — 2026-08-12T09:21:14+00:00

**Verdict:** COMMENT

This pass closed the three test-coverage majors pass 2 left open, then re-reviewed
the fix twice more. The first confirmation pass, asked to check the whole plan
rather than only the fresh edits, caught a critical placement defect in the fix
itself plus two pre-existing test-coverage gaps left over from pass 2's token-model
and process-probe-extraction work; the second confirmation pass, checking the fix
to the fix, caught two further majors and a minor in the surrounding detail. All
are now resolved. Every lens's majors and criticals across all three passes are
closed; what remains open is minors and suggestions only, which is why the verdict
moves to COMMENT rather than APPROVE.

### Previously Identified Issues

- 🟡 **Test Coverage**: `test-run.sh`'s ~20 `links`-privacy assertions excluded from the migration checklist — Resolved (ported into `daemon-runtime.test.js`, run in the opt-in lane)
- 🟡 **Test Coverage**: Four re-homed grep assertions risk self-matching inside the tree they scan — Resolved (only two of the four are genuinely at risk; both use fragment-built needles plus non-literal titles/comments/assertion messages)
- 🟡 **Test Coverage**: Bare-`return` grep cannot distinguish a test body from a helper; zero-skip lane has nowhere to put the extracted runtime test — Resolved (brace-scoped guard; extracted to `daemon-runtime.test.js`; TAP-based pass/fail/skip verification replaces the file-count proxy)

### New Issues Introduced

- 🔴 **Test Coverage** (fixed this pass, caught by the first confirmation re-review): my initial fix placed the runtime-dependent `links` assertions inside `daemon.test.js` — the very file the unit lane discovers via its `lib/*.test.js` glob and requires to run with zero skips. `node --test` has no mechanism to run only part of a file's test cases per lane, so this would have made the newly-built zero-skip gate unsatisfiable in CI on every build without a bootstrapped Playwright runtime. Moved to `daemon-runtime.test.js` instead, alongside the already-extracted `ping` test.
- 🟡 **Test Coverage** (fixed this pass, pre-existing gap from pass 2's token-model rewrite, caught by the first confirmation re-review's broader sweep of the whole plan rather than by this pass's own Phase 6 §6 edits): §3's request-token threat model states as a design requirement that "the token, any resolved auth-header value and any `ACCELERATOR_BROWSER_*` credential must never be written to the bootstrap log" — pinned as prose in pass 2, but never given a matching Success Criteria bullet, unlike the other three implementation details in the same paragraph. Added.
- 🟡 **Test Coverage** (minor, fixed this pass, same provenance as above): the `process-probe` crate extraction (pass 2) had no criterion that the *existing* Linux/Darwin start-time test suite moves with the extracted function, only that the implementation does — risking the regression protection the plan cites as its justification for reuse being silently lost in the move. Added a criterion that both `process-probe`'s and `accelerator-visualiser`'s own suites pass post-extraction.
- 🟡 **Test Coverage** (fixed this pass, caught by the second confirmation re-review): the executed-count floor's derivation silently dropped the `ownerPid` guard from its enumeration (counting 4 re-homed tests where the Removal-sweep table names 5, since the `evaluate-payload-rejected`/`mcp__playwright__` row is one table row but two separate tests) — landing the stated floor one below its own intended target. Recomputed at 55, not 54.
- 🟡 **Test Coverage** (fixed this pass, caught by the second confirmation re-review): the self-matching-guard fix gave worked non-literal titles for two of the three at-risk tests and never mentioned assertion failure-message text as an equally literal leak channel — closing the exact bug class the first critical was about, just via a channel the stated fix didn't cover. Extended to all three tests and to any string literal in the test's own source, not only titles and comments.
- 🔵 **Test Coverage** (fixed this pass): no explicit instruction that the ported `links` assertions should drop `test-run.sh`'s own internal skip-on-empty-output guard, which would have reintroduced a bare-skip shape one file over from where it was just removed.

### Assessment

Self-correction did the work here: the fix-then-verify loop caught its own mistake
(a critical placement error) before it reached the plan's final state, and a second
loop caught two further gaps in the fix's own edges. The plan's test-coverage story
is now internally consistent — verified directly against the actual source tree,
not merely against the plan's prose — with no remaining overlap between the unit
lane's runtime-free glob and anything that needs a real Chromium, and the two
genuinely self-matching-risk guards closed on every channel checked (needle,
title, comment, assertion message). What remains across the whole review is minor
and suggestion-level: in-code documentation requests (rustdoc for `RunClient`'s
diverging contract, a discoverability note for the two outcome-carrier types), a
few low-priority hedges (SIGKILL escalation on an ignored SIGTERM, the NFS lock
backend question), and a handful of small residue items (a token-generation test
seam, `ACCELERATOR_LOCK_FORCE_MKDIR` half-honoured documentation). None block
implementation.

---
*Review generated by /accelerator:review-plan*
