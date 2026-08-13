---
type: plan-validation
id: "2026-08-11-0196-design-cli-migration-validation"
title: "Validation Report: accelerator-design: CLI Migration and Shell-Free Executor"
date: "2026-08-13T11:48:29+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "partial"
parent: "work-item:0196"
target: "plan:2026-08-11-0196-design-cli-migration"
tags: [rust, design, cli, sub-binary, executor, playwright]
last_updated: "2026-08-13T11:48:29+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: accelerator-design CLI Migration and Shell-Free Executor

**Result: partial.** All four phases landed, the aggregate gates are green, and the
executor port works end to end. Three confirmed defects sat behind the gates —
one making four documented executor commands unreachable, one respawning the
daemon on every invocation in containers, one leaving the retained integration
suite entirely red. All three are now fixed, test-first, and recorded under
"Resolution" below, along with a fourth — a pre-existing wall-clock defect in
retained `daemon.js` that the migration did not cause and that fixing D3
exposed. All four lanes are now green, including the opt-in runtime one at 23 of
23. The result stays `partial` rather than `pass` because the plan's success
criteria were not met by the implementation as delivered: they were met by
repairs made during validation, and the residue in the section below is still
outstanding.

### Implementation Status

✓ Phase 1: `corpus metadata derive --filename-timestamp-format` — fully implemented
✓ Phase 2: The `design` sub-binary and its five non-Playwright subcommands — fully implemented
✓ Phase 3: Retire the two metadata scripts — fully implemented
⚠️ Phase 6: `run.sh` → Rust — implemented, with three defects (D1, D2, D3) and one latent hazard (D4)
⚠️ Removal sweep — implemented, with residue (R1–R4)

Thirteen commits, `zkooxowlwxsv` (Phase 1) through `pywsnvpmvuzx`, all on the
working-copy ancestry with a clean tree.

### Automated Verification Results

✓ `mise run check` exits 0
✓ `mise run test` exits 0 (`test:unit:design-automation` 78 pass / 0 fail / 0 skip after the fixes; 76 before)
✓ `mise run cli:check`, `lint:dispatch-coherence:check`, `deny:check`, `docs:check` — all inside the green `check` roll-up
✓ `mise run test:integration:design-automation` — **23 of 23 pass** after the fixes below; 7 of 21 before
✓ Warm-path gate: shell median 108.38 ms → port 43.95 ms, **ratio 0.406** against a gate of ≤ 1.0, delta 64.4 ms/call

Not run: the bare default `mise run`. It is `build` + `fix` + `check` + `test`;
`check` and `test` both pass and `fix` is mechanical, so the gate is met in
substance, but the exact task was not executed.

### Code Review Findings

#### Matches plan

- **Phase 1** — `FilenameTimestampFormatArg` mirror plus `From` impl at
  `cli/corpus-cli/src/cli.rs:89-106`, threaded through `main.rs:72-87`; the
  argument→variant mapping asserted directly (`cli.rs:166-184`); no `FakeClock`
  test added for the renderer, exactly as the plan reasoned.
- **Phase 2** — `HostReach` carries six variants with the loopback carve-out
  applied before any flag check (`cli/design/src/access_policy.rs:40`) and
  `Unspecified` rejected under every flag combination (`:41-47`). The reserved set
  is hand-enumerated with 6to4, Teredo (RFC 4380 bitwise inversion), NAT64 and
  mapped/compatible forms unwrapped and re-classified
  (`cli/design/src/host_reach.rs:98-173`). Exit codes 0/1/2 land as designed and
  domain rejection is never a `kernel::Error`.
- **Phase 2** — the migration checklist is committed
  (`meta/migrations/0196-design-cli-migration-checklist.md`) with a mutation-evidence
  table, and every deliberate-drop row names a replacement property.
- **Phase 6** — nine ports with the named shapes; `RunClient` typed as
  `Box<Self> -> Result<Infallible, kernel::Error>` so no domain logic can be
  sequenced after `exec`; the reuse verdict is a `const fn` with a test per table
  row and no row signalling. The start-time probe is extracted into
  `cli/process-probe/` on `libc` alone, with the visualiser repointed and its own
  copy removed. Lock is a single `flock` backend with a `Drop` guard plus explicit
  release before hand-over. The identity pipe, request token, Origin/query-param
  refusals, `setsid`-not-double-fork, `umask(0o077)`, log truncation and
  `server-stopped.json` removal all landed with tests.
- **Removal sweep** — every re-home row reaches its new home; `test-design.sh` is
  down to 13 lines; follow-ups exist as `0205`, `0206`, `0207`; the v2.1.144
  rationale is restated without `browser-executor`.

Verified by direct execution: `validate-source` accepts `https://example.com`,
`about:blank`, `http://localhost:3000`, `http://[::1]:3000`; rejects
`http://example.com` at 1 (flipped to 0 by `--allow-insecure-scheme`), userinfo,
every numeric IPv4 encoding unconditionally, `169.254.169.254` and its
`::ffff:`/`64:ff9b::`/`2002:` encodings, `100.64.0.1`, `0.0.0.0` under every flag.
`scrub-secrets /nonexistent` and `audit-cue-phrases /nonexistent` exit 2;
`executor daemon` is refused. `executor ping` (cold and warm) and `daemon-stop`
succeed against a real runtime.

#### Deviations from plan

These read as taken decisions, not accidents:

- `cli/design/src/executor/` is a top-level module, not inside `runtime/` as the
  sub-domain argument specified; `runtime/` holds only `downgrade.rs`.
- The exit-code carrier split in two: a payload-free `Verdict<Reason>` in the
  domain (`cli/design/src/verdict.rs`) and `Report` in `design-cli`
  (`cli/design-cli/src/report.rs`). Observable behaviour matches the plan's table.
- `analyse-design-gaps/SKILL.md:146` asks for `date-only`, not `compact-time` —
  a correction, since the gap artefact path is `YYYY-MM-DD-…`. `compact-time`
  as written would have produced an unusable stamp.
- `FilenameTimestampFormatArg` exposes all three domain variants, including
  `DateOnly`, rather than the two the plan named.
- Unit-lane floors are stricter than planned: 9 suites / 76 cases against the
  plan's 8 / ≥55.
- The DST fall-back TZ case is not asserted; `cli/design-adapters/tests/start_time.rs:66-72`
  explains why it is unprovable for a live process — consistent with the plan's own §4.
- The plan reserved `test-design.sh:140`'s `SKILL=` assignment and `:153`'s
  `# shellcheck disable=SC2016` as adjacency traps to leave behind for the
  sibling. Both moved to `scripts/test-skill-frontmatter-conformance.sh:538,551-553`.
  The assertion still runs, but **the sibling plan's stated edit set is now wrong**:
  it must delete that assertion from the conformance suite, not from `test-design.sh`,
  which by then holds nothing but the delegation the sibling deletes.

#### Confirmed defects

**D1 — `RecordedStartTime::WriterUnavailable` is unreachable from disk, so the
container case respawns the daemon on every invocation.** `cli/design-adapters/src/state.rs:72-79`
returns `AbsentOrUnparseable` on an absent-or-null `start_time` *before* consulting
`start_time_source`. The writer emits `start_time: null` with
`start_time_source: 'writer-unavailable'` (`lib/identity-handoff.js:66-67` →
`lib/daemon.js:400-402`), so such a record reads as `AbsentOrUnparseable` →
`Reuse::Recover`. The verdict table's `Daemon(WriterUnavailable)` row is never
exercised in production, and the failure it exists to prevent — respawning on
every command and losing page state where `/proc` is unreadable — is live.

**D2 — four documented executor commands are unreachable.** `FORWARDABLE_COMMANDS`
(`cli/design/src/executor/forwardable.rs:16-24`) names seven commands. `daemon.js`
dispatches eleven, `PROTOCOL.md` documents all eleven under `### \`<command>\``,
and `agents/browser-analyser.md:47-49,73,105-106` instructs the agent to run
`click`, `type` and `wait_for`. Confirmed by execution — each exits 2 with
"unknown executor command":

| Command | Dispatched by `daemon.js` | Documented | Forwardable | Result |
|---|---|---|---|---|
| `click` | `:249` | ✓ | ❌ | exit 2 |
| `type` | `:255` | ✓ | ❌ | exit 2 |
| `wait_for` | `:262` | ✓ | ❌ | exit 2 |
| `daemon-status` | `:146` | ✓ | ❌ | exit 2 |

The `browser-analyser` agent's entire interaction capability is broken. The plan's
own safeguard ("a command added to `daemon.js` later is unreachable until the
allowlist moves with it") fired against commands that already existed, and the
re-homed `PROTOCOL.md` ↔ `daemon.js` sync test
(`lib/daemon.test.js:341-361`) does not tie `FORWARDABLE_COMMANDS` to either side
— which is precisely the extension the plan's Phase 6 §3 said it would gain.

**D3 — `test-run.js` was never updated for the two contracts Phase 6 introduced,
so the opt-in lane is red.** `spawnDaemon` (`test-run.js:58-70`) forks
`run.js daemon` with no `ACCELERATOR_PLAYWRIGHT_IDENTITY_FD` and no handoff pipe,
so `readIdentity` throws `MalformedIdentity` and the daemon exits before publishing
`server-info.json` — the designed crash-safety behaviour, firing against its own
test harness. `send` (`:41-56`) also sends no token header. All 14 of its tests
fail on the first `waitForInfo`. `daemon-runtime.test.js:63-67` and
`daemon.test.js` were both updated (`HANDOFF_FD`, `TEST_TOKEN`); `test-run.js` was
not. Reproduced: 7 pass (all in `daemon-runtime.test.js`), 13 fail, 1 cancelled.

Converting the 14 `skip:` gates to hard failures was the plan's own criterion, and
it worked exactly as intended — it turned a silent skip into a visible failure.
Nothing caught it because the lane is opt-in and no CI lane provisions a runtime.

#### Resolution of D1, D2 and D3

All three were fixed in this validation pass, test-first.

- **D2** — `FORWARDABLE_COMMANDS` now names all eleven dispatched commands
  (`cli/design/src/executor/forwardable.rs`), with `daemon` still refused as
  internal. A new test in `lib/daemon.test.js` reads the Rust allowlist and
  `daemon.js`'s own `cmd ===`/`case` labels and asserts the two sets are equal,
  so the cross-language drift that caused this fails at test time rather than at
  agent run time — the extension Phase 6 §3 promised and never landed. Verified
  by execution: all four commands now reach the daemon at exit 0.
- **D1** — `interpret_start_time` consults `start_time_source` before the value,
  so `writer-unavailable` maps to `RecordedStartTime::WriterUnavailable` instead
  of `AbsentOrUnparseable`. Because the existing tests hand-build their JSON and
  so cannot catch a writer/reader mismatch, the fix is pinned against a shared
  artefact instead: `lib/__fixtures__/server-info-writer-unavailable.json`
  records the shape the daemon publishes, `cli/design-adapters/tests/recorded_state.rs`
  asserts the reader's verdict on it, and `identity-handoff.test.js` asserts the
  writing side produces those same fields.
- **D3** — `test-run.js`'s `spawnDaemon` now opens the handoff pipe on fd 4 and
  writes the four identity fields, `send` carries the token header, and the
  namespace preflight moved into a shared `runtime-preflight.js` so both
  root-level suites resolve the lockhash namespace the way the launcher does
  rather than keeping two copies of that arithmetic. Its `withTmpDir` also
  waits out the SIGTERM'd daemon's last write, a teardown race that only became
  reachable once daemons started at all.

The lane went from 7 passing of 21 to **20 of 21**. One failure remains, and it
is a different defect — see below.

#### A fourth defect, pre-existing, also fixed

The remaining test failure was three separate problems in the retained
`daemon.js` wall clock, none caused by the migration and all hidden until now by
the suite's wholesale self-skip. `test-run.js:319`'s single either/or assertion
was replaced by three tests, one per property.

**Reproduced and fixed:**

- ❌ **The backstop pre-empted the graceful path it exists to protect.** Every
  bounded operation passes `WALL_CLOCK_MS` to Playwright as its own timeout, and
  `armWallClock` was armed for the same instant — earlier in the same tick. So
  the backstop always won: a `wait_for` whose caller timeout exceeded the budget
  killed the daemon instead of returning `wait-for-timeout` with
  `truncated: true`, and the documented capping behaviour was unreachable. The
  backstop now fires at budget + `WALL_CLOCK_GRACE_MS` (2000 ms), which is what
  leaves room for the operation's own envelope.
- ❌ **The expiry envelope never reached an HTTP client.** `armWallClock` wrote
  it with `res.write(...)` and then exited without `res.end()`, leaving the
  client waiting on a chunked body that never terminated — so the one caller who
  needed to hear about the timeout was the one caller who never did. It now
  sends a complete response through a `respond` helper that also makes the
  operation and its backstop mutually exclusive, since the backstop fires
  precisely while the operation is still running.

**Latent, not reproduced, fixed anyway:**

- ⚠️ `armWallClock` ran at `daemon.js:375`, before `ensureBrowser()` at `:169`,
  charging the Chromium cold start to the first operation's budget. I claimed
  this was live; measuring it says otherwise — a first `navigate` including the
  launch takes ~200 ms here, so even a 200 ms budget survives and no budget a
  caller would plausibly set reproduces it. It is still wrong in principle, and
  a genuinely cold host or a loaded CI runner is where it would bite, so the
  arming moved behind an `onBrowserReady` hook that fires after
  `ensureBrowser()`. Its test is a guard rather than a red-first driver, and
  passes both before and after on this machine.

`PROTOCOL.md`'s `ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS` row and its `BLOCKING_OPS`
note now state when the budget starts, that a compliant command answers with its
own envelope, and that `wall-clock-exceeded` is a backstop sitting deliberately
outside the cap.

With this, `test:integration:design-automation` is **23 of 23**.

#### Latent hazard

**D4 — `dup2(read_fd, IDENTITY_FD)` is a no-op when the read end already lands on
fd 3.** `cli/design-adapters/src/process.rs:138` relies on `dup2` clearing
`FD_CLOEXEC` on the duplicate, but POSIX makes `dup2(fd, fd)` a no-op that leaves
the flag untouched — and both pipe ends are given `FD_CLOEXEC` explicitly at
`:213-215`. In that case the child's fd 3 closes at `exec` and the daemon dies
with the "IDENTITY_FD is not set" class of failure. It currently works because the
lock fd and bootstrap log are opened first; it is ordering-dependent, not stable.

#### Residue

- **R1** — `skills/design/analyse-design-gaps/SKILL.md:14` grants
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/analyse-design-gaps/scripts/*)` for a
  directory that no longer exists. `inventory-design/SKILL.md:16`'s
  `scripts/playwright/*` grant likewise has no surviving call site. 🔒 Both are
  live permission grants broader than anything the skills now invoke.
- **R2** — `skills/design/inventory-design/evals/benchmark.json:1738-2067` still
  grades on `validate-source.sh` and `run.sh`. Outside the plan's stated grep set
  (`docs-site/`, `README.md`, `CHANGELOG.md`), which is why it was missed.
- **R3** — five migration-checklist rows name Rust tests under wrong names
  (`the_widened_loopback_set_no_longer_needs_allow_internal`,
  `the_widened_loopback_set_covers_the_expanded_and_ranged_forms`,
  `the_shell_s_own_classifications_survive`,
  `the_rfc1918_rejection_keeps_the_shell_s_wording`,
  `the_compiled_table_still_agrees_with_the_shell_s_message_file`), and row `:114`
  claims `test-design.sh` holds an assertion that now lives in the conformance
  suite. The checklist is the traceability artefact and nothing checks its links.
- **R4** — the dangling-call-site CI guard the plan promised (Removal sweep §1: no
  SKILL.md or agent body may name a nonexistent path under
  `skills/design/**/scripts/`) does not exist anywhere in `tasks/lint/`,
  `tasks/test/`, `tests/`, `scripts/` or `.github/`. R1 is exactly what it was
  meant to catch.

#### Criteria with no test

- Client-path exit-status and signal-death (128+n) propagation.
  `cli/design-cli/tests/executor_preflight.rs:5-7` explicitly defers it to the
  opt-in lane, where nothing asserts it either — and D3 means that lane cannot run.
  The plan's §1 stub harness specified this.
- The bootstrap log's non-leakage of the token and any resolved credential. Only
  truncation and mode are asserted (`spawn_properties.rs:122`).
- Byte-identical path-bearing envelopes across two different working directories;
  only one cwd is exercised (`executor_preflight.rs:131`).
- The `DIR_COUNT` marker invariant. `scripts/test-skill-frontmatter-conformance.sh:426-431`
  derives `EXPECTED_DIR_COUNT` from the same file it then asserts against, and
  defers the real invariant to `test-config.sh`, which no longer exists.
- Which format flag each skill passes. Nothing pins `compact-time` for
  `inventory-design` or `date-only` for `analyse-design-gaps`; a flip would
  silently change artefact filenames.

#### Plan text, not code

The Phase 2 manual criterion "`accelerator design validate-source 0x7f000001`
exits 1 with the numeric IPv4 message" is mis-specified. A schemeless argument was
classified as a path by the shell too (the deleted `validate-source.sh`'s
`*) scheme="path"` arm), so it correctly exits 1 with the path message. The
numeric-encoding rejection fires on `http://0x7f000001`, which was verified. The
criterion is met in substance; the example was written wrong.

### Manual Testing Required

1. Executor, after D1–D4 are addressed:
  - [ ] A live inventory crawl completes with page state preserved across
        consecutive executor commands
  - [ ] Both browser agents work end to end without `{browser-executor-script}` —
        this cannot pass until D2 is fixed
  - [ ] Two concurrent executor commands produce one daemon, loser reports
        `another-launcher-running`
  - [ ] A container or `hidepid`-hardened host does not respawn the daemon on
        every invocation (the D1 regression)

2. Skills end to end:
  - [ ] Both design skills run end to end in a live session on a machine with a
        bootstrapped Playwright namespace
  - [ ] A Playwright-driven inventory behaves as before — same prerequisite, same
        bootstrap, same downgrade reasons

### Recommendations

1. **Decide whether an opt-in lane no CI job runs is worth keeping in this
   shape.** Six separate defects lived in that suite untouched because nothing
   ever ran it. The plan's fail-rather-than-skip change is what made them
   visible, but only because someone ran the lane by hand — which no scheduled
   job does. A lane that must be remembered is a lane that will not be.
2. **Close D4** by guarding the `dup2` with an `if read_fd != IDENTITY_FD` and
   clearing `FD_CLOEXEC` explicitly in the equal case, rather than relying on
   descriptor-allocation order.
3. **Land R4's guard**, then R1 falls out of it. Fix R2's `benchmark.json` and
   R3's checklist names in the same pass.
4. **Correct the sibling plan's edit set** for the moved `SKILL=` / `SC2016` /
   `scripts/*` assertion before it is scheduled.

The plan's `status` is deliberately left at `ready` rather than advanced to `done`.
Every automated criterion now passes, but four of them passed only after repairs
made during validation rather than as delivered, the residue above is
outstanding, and the two live-session manual criteria have not been run.
