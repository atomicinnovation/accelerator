---
type: "plan-review"
id: "2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys-review-1"
title: "Plan Review: Unify the Trust Barrier for Consent Config Keys Implementation Plan"
date: "2026-09-25T08:47:23+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys"
relates_to: ["work-item:0226", "work-item:0298"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "security", "test-coverage", "code-quality", "compatibility", "safety", "documentation"]
review_number: 1
review_pass: 6
tags: ["security", "config", "consent", "credentials", "design", "session-start"]
last_updated: "2026-09-27T08:17:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Unify the Trust Barrier for Consent Config Keys Implementation Plan

**Verdict:** REVISE

The domain model is sound, and every lens credited it. The policy in
`config::consent` is pure and sits behind ports, tracking is tri-state and
fails closed, team values are read raw so a blank personal value cannot mask
them, there is one refusal vocabulary, and Jira's frozen exit codes are mapped
exhaustively. Phase 1 is close to implementable as written. The revisions
needed are in three areas. Phase 5's daemon restart introduces a new signal
path that is not safe as designed. The Phase 2 runner does not yet bound
everything it claims to bound. The "one policy" is only partly one: the
command-key flow and the session-start audit are assembled by their callers
rather than provided by the policy. Documentation and CHANGELOG work lags
behind the phases it describes, and the test plan leaves consumer-boundary
behaviour unguarded. There are no critical findings, but there are about 20
distinct majors, far above the threshold of 3.

### Cross-Cutting Themes

- **Daemon stop can signal a process whose identity was never verified**
  (flagged by: safety, security, correctness). This was verified:
  `identity_holds` (`cli/design/src/executor/reuse.rs:61-76`) returns `true`
  without comparing anything for `Wallclock`, `WriterUnavailable` and
  `Probe` + `Unavailable`. Phase 5's `stop(pid)` on a mismatch, `Unrecorded`
  included, can therefore SIGTERM a recycled pid. The state file also lives in
  a directory the repository can write.
- **Restarting on a mismatch disrupts shared or concurrent daemons**
  (flagged by: safety, compatibility, correctness). The launcher lock does not
  protect an in-progress crawl. Two sessions with different vetted browsers
  (an env override in one shell, a tracking flip, or a mixed plugin version
  treating `Unrecorded` as a mismatch) will keep stopping each other's
  daemons. There is no SIGKILL escalation after the 10s timeout, so a wedged
  Chromium blocks every later crawl.
- **The recorded browser is read separately from the daemon record**
  (flagged by: architecture, security, correctness). A second read of
  `server-info.json` can tear against the atomic rename and pair one daemon's
  pid with another daemon's browser. This design is driven by a public-API
  fixture rather than by the domain.
- **Runner reader joins remain unbounded on the success path**
  (flagged by: correctness, safety, security, compatibility). A command such
  as `echo tok; sleep 60 &`, a helper that daemonises, or a descendant that
  calls `setsid` keeps the pipes open after `bash` exits. The joins then block
  past the deadline, which is today's bug in a different place.
- **The command-key flow and the session-start audit bypass the policy**
  (flagged by: architecture, code-quality). `resolve_token` and
  `resolve_github_token` each hand-assemble resolve → `for_key` → run → map.
  Phase 6 re-derives the team-level and tracked rules itself, and 0227 would
  add a third copy. Adding `consent::resolve_command` and `consent::audit`
  would make the unification structural.
- **Runner failures have no place in the warning or refusal model**
  (flagged by: code-quality, correctness). `ResolvedToken.refusals` is a
  `Vec<Refusal>`, so a non-fatal `E_TOKEN_CMD_FAILED` is lost. "First refusal"
  as the fatal error reports `TEAM_LEVEL` (exit 24) when the real cause was a
  personal command timing out (exit 25).
- **The scrubbed environment breaks common credential helpers**
  (flagged by: compatibility, documentation). Under the new environment:
  - `gh auth token` on a Linux keyring needs `DBUS_SESSION_BUS_ADDRESS`,
    `XDG_RUNTIME_DIR` and `XDG_CONFIG_HOME`;
  - `secret-tool`, the AWS CLI, `op` sessions and `pass` lose the variables
    they need.

  Meanwhile the configure skill still promises that these helpers work
  "without plugin-side knowledge".
- **Docs and CHANGELOG lag behind the behaviour**
  (flagged by: documentation, compatibility). Every doc edit waits for
  Phase 7, although each phase merges on its own and a prerelease is
  published from `main` on every push. No phase adds `CHANGELOG.md`
  `[Unreleased]` entries for the breaking changes or the code renames.
- **Fail-closed tracking needs a recovery route**
  (flagged by: compatibility, safety). `Unknown` refuses every personal
  credential when the pinned gix or jj-lib cannot read the repository format
  (reftable, sha256, or a newer jj store). The refusal text does not name the
  `ACCELERATOR_*` escape hatch.
- **TMPDIR can place the "outside the repository" cwd inside it**
  (flagged by: security, correctness).
- **Path vetting leaves some nonexistent-path cases undefined**
  (flagged by: security, correctness). The unhandled cases are a dangling
  final symlink, a parent that is also missing, and a `None` from
  `canonicalise`.

### Tradeoff Analysis

- **Security and correctness vs daemon availability.** Safety and security
  want to signal only when identity is proven, and never while a crawl is
  running. Compatibility and correctness want an `Unrecorded` or mismatched
  daemon replaced promptly. The recommended resolution is a graceful stop over
  the authenticated loopback protocol, which proves identity without a
  signal. Escalate to SIGKILL on the process group only after identity is
  proven. As an alternative, key the state slot by browser identity so that
  nothing needs to be stopped.
- **Fail-closed vs usability on repository formats gix or jj-lib cannot
  read.** Keeping `Unknown` fatal matches Requirement 2. The refusal should
  still name the environment-override route and the class of library error,
  and Migration Notes should list the case. A CLI fallback (`git ls-files`)
  would weaken the `std::process` ban on `consent-adapters`, so it is better
  deferred.
- **Scrubbed environment vs helper compatibility.** Widening the admitted
  environment (the XDG and D-Bus variables) trades away a little of
  Requirement 5's strictness. Documenting an `env VAR=… cmd` workaround keeps
  the strictness. This is a decision for the user. The Linux `gh` keyring
  case argues for admitting at least the D-Bus and XDG variables for
  `github.token_cmd`, which would amend the work item.
- **Launcher purity (ADR-0054) vs a single session-start channel.** Linking
  `gix`/`jj-lib` into the launcher is simplest, but it breaks a recorded
  decision. Moving the tracked-file warning into an already-dispatched
  sub-binary hook, such as `vcs detect --format=hook`, keeps the ADR but
  splits the warnings across two hook invocations.

### Findings

#### Critical

None.

#### Major

- 🟡 **Safety + Security + Correctness**: `stop(pid)` can SIGTERM an unrelated
  process on liveness-only reuse verdicts
  **Location**: Phase 5 §2
  `identity_holds` accepts `Wallclock`, `WriterUnavailable` and `Unavailable`
  without any comparison, and the state file sits in a directory the
  repository can write. Signal only on a proven `Probe` match, or stop the
  daemon over the authenticated protocol.
- 🟡 **Safety + Compatibility**: A restart on mismatch kills a daemon that
  another session is using, and cycles daemons across plugin versions
  **Location**: Phase 5 §2 (under the lock; `Unrecorded`)
  The lock serialises only launching. Differing vetted browsers, or an
  old-version executor spawning `Unrecorded` daemons, lead to repeated
  restarts in both directions.
- 🟡 **Safety**: A daemon that ignores SIGTERM blocks every later invocation
  **Location**: Phase 5 §2 (stop timeout)
  There is no escalation. `daemon.js` arms its exit backstop only after
  `browser.close()` resolves.
- 🟡 **Architecture**: The recorded browser is read separately from the
  daemon identity it belongs to
  **Location**: Phase 5 §2; Key Discoveries
  The two reads can tear. Parse pid, start time and `custom_browser` in one
  read.
- 🟡 **Correctness + Safety + Security**: Reader joins are unbounded when
  `bash` exits but a descendant holds the pipes (including `setsid` escapes)
  **Location**: Phase 2 §2
  Make the deadline cover reader EOF, kill the group on every exit path, and
  add a `sleep 60 & echo ok` test.
- 🟡 **Correctness**: The output-cap verdict can race a successful exit
  **Location**: Phase 2 §2
  Decide the verdict only after both readers have been joined, and read the
  counter after the join.
- 🟡 **Code Quality + Correctness**: Runner failures have no warning channel,
  and the "first refusal" rule masks the real cause
  **Location**: Phase 3 §1; Phase 1 §5 (`base_url`)
  Widen the warning type, or add `Refusal::CommandFailed`. Make the fatal
  error the failure of the highest-precedence rung actually attempted.
- 🟡 **Architecture + Code Quality**: The command-key flow is reimplemented in
  two ladders
  **Location**: Phase 2 §1; Phase 3 §1, §4
  Add `consent::resolve_command`, mirroring `resolve_executable_path`.
- 🟡 **Architecture + Code Quality**: SessionStart builds its own consent
  audit
  **Location**: Phase 6 §2
  Add `consent::audit(&ProvenanceContext) -> Vec<Refusal>`. Drop the
  undefined `ConsentFacts`, and name `audit` as 0227's entry point.
- 🟡 **Architecture**: Linking `consent-adapters` into the launcher
  contradicts ADR-0054
  **Location**: Phase 6 §1
  This was verified: ADR-0054:112-113 says the launcher depends on `kernel`
  and `config`, "never on a subdomain". Either keep VCS out of the launcher,
  or amend the ADR with limits on size and latency.
- 🟡 **Safety**: Consent collection can take down the whole SessionStart
  summary
  **Location**: Phase 6 §2
  `assemble` propagates the new errors with `?`, and a panic in gix or jj-lib
  would not be caught. Make consent collection infallible from the summary's
  point of view.
- 🟡 **Security**: A `.jj` marker committed by a hostile git repository
  hijacks the tracking query and shrinks the repository roots
  **Location**: Phase 1 §3; Phase 4 §2
  `.jj` beats `.git`, and git allows committing `.jj/`. Also consult gix's
  own discovery, and treat a `.jj` tracked by git as `Unknown`.
- 🟡 **Security**: The runner resolves `bash` and the user's command through a
  `PATH` the repository can influence
  **Location**: Phase 2 §2
  Spawn `/bin/bash`, and drop `PATH` entries that are relative or that fall
  inside the repository.
- 🟡 **Code Quality**: `ConsentKey` does not enforce its trust invariant or
  distinguish kinds
  **Location**: Phase 1 §2; Phase 2 `for_key`; Phase 4 `vet_executable_path`
  Its fields are public. Make them private behind a test-support
  constructor, and add narrowing accessors for each kind.
- 🟡 **Compatibility + Safety**: Fail-closed tracking depends on gix and
  jj-lib reading the repository format, and gives no recovery route
  **Location**: Phase 1 §3; Phase 3 §1
  Name the environment-override route and the error class in the refusal,
  add a test, and list the case in Migration Notes.
- 🟡 **Compatibility + Documentation**: The scrubbed environment breaks `gh`
  on Linux and other helpers, and the runner contract is undocumented
  **Location**: Phase 1 §1; Phase 3 §4; Phase 7 §2
  Decide whether to admit the XDG and D-Bus variables. Document the contract
  and the workaround.
- 🟡 **Documentation + Compatibility**: User-facing docs lag behaviour for
  Phases 1–6
  **Location**: Phase 7
  Move each doc edit into the phase whose behaviour it documents.
- 🟡 **Documentation + Compatibility**: Breaking changes and code renames
  have no CHANGELOG entries
  **Location**: Migration Notes
  Add `[Unreleased]` Breaking, Security and Removed entries in each phase.
- 🟡 **Documentation**: The configure skill still presents
  `jira.allowed_sites` as a team-shared key
  **Location**: Phase 7 §2
  Move it to the personal table explicitly, and fix the "One key" heading.
- 🟡 **Test Coverage**: linear-cli and work-cli have no behavioural tests
  **Location**: Phase 3 Tests
- 🟡 **Test Coverage**: Consumer `warning:` output is verified only by hand
  **Location**: Phase 1 §6
- 🟡 **Test Coverage**: The executor wiring from vetted canonical path to
  daemon spawn environment is untested
  **Location**: Phase 4 §3; Phase 5 §2
- 🟡 **Test Coverage**: The design-cli test seam is undecided (`--dry-run`
  does not exist), and inside-repository is untested at the consumer
  **Location**: Phase 4 Tests
- 🟡 **Test Coverage**: The timeout and pidfile tests are timing-sensitive
  and racy (a 3s bound, and zombies answer `kill -0`)
  **Location**: Phase 2 Tests
- 🟡 **Test Coverage**: The command rows of the severity matrix and the
  env-command fallback are not enumerated
  **Location**: Phase 3 Tests

#### Minor

- 🔵 **Security + Correctness**: `tempdir_in(temp_dir())` honours `TMPDIR`,
  which the repository can set to a directory inside itself.
  **Location**: Phase 2 §2
- 🔵 **Security + Correctness**: Several nonexistent-path cases are
  undefined: a dangling final symlink, a missing parent, `..`, and `None` from
  canonicalisation. Canonicalise the nearest existing ancestor instead.
  **Location**: Phase 4 §1–2
- 🔵 **Correctness**: `facts() == None` does not always mean no VCS. A
  non-UTF-8 root name or a stat permission error both give `None`, so these
  cases fail open.
  **Location**: Phase 1 §3
- 🔵 **Correctness**: `repository_roots` silently omits a main root it fails
  to discover. A raw `/var` root never prefixes a canonical `/private/var`
  path.
  **Location**: Phase 4 §2
- 🔵 **Correctness**: What happens after a refused winner differs between path
  keys (blocks lower rungs) and command keys (falls through).
  **Location**: Phase 3 §1; Phase 4 §1
- 🔵 **Correctness + Test Coverage**: When a usable allowlist omits the host,
  `base_url` reports a provenance refusal instead. The acceptance criterion
  names an `ACCELERATOR_*` override of `jira.allowed_sites`, but none exists.
  **Location**: Phase 1 §5
- 🔵 **Code Quality**: Plaintext `Unknown` collapses into
  `TokenFromTrackedFile`, which then needs a hedged message.
  **Location**: Phase 3 §1
- 🔵 **Code Quality**: `ProcessControl::stop` duplicates the existing SIGTERM
  `terminate`.
  **Location**: Phase 5 §2
- 🔵 **Code Quality**: `Consented<T>` uses an anonymous tuple, and
  `into_usable` is never specified.
  **Location**: Implementation Approach; Phase 1 §2
- 🔵 **Code Quality**: The environment-first rule is split between `resolve`
  and the ladder, and the plan never says which callers pass `Some`.
  **Location**: Phase 1 §4; Phase 3 §1
- 🔵 **Architecture**: The pup rule for the config-command seam does not deny
  `consent_adapters`.
  **Location**: Phase 6 §1
- 🔵 **Architecture**: The phase-independence claim overstates the
  dependency graph. The plan should state the DAG and the fail-open window
  for command keys during Phases 1–2.
  **Location**: Migration Notes
- 🔵 **Safety**: The ordering between killing the process group and reaping
  the leader, and the validity of the pgid, are not pinned on the
  output-exceeded path.
  **Location**: Phase 2 §2
- 🔵 **Safety**: The upgrade restarts every warm daemon once, including those
  mid-crawl. List this in Migration Notes.
  **Location**: Phase 5
- 🔵 **Compatibility**: The Jira exit code for a team allowlist with a
  non-Atlassian site changes from 1 to 24, and this is not listed.
  **Location**: Phase 1 §6
- 🔵 **Compatibility**: Trimming narrows from `.trim()` to removing one
  `\n`, and stderr now counts towards the cap for GitHub helpers.
  **Location**: Phase 2 §2; Phase 3 §4
- 🔵 **Compatibility**: `process_group(0)` stops helpers that prompt on a tty
  with SIGTTIN.
  **Location**: Phase 2 §2
- 🔵 **Compatibility**: Public-API fixture regeneration is missing from
  Phases 2, 3 and 5.
  **Location**: Success Criteria
- 🔵 **Compatibility**: The plan gives the wrong path for `PROTOCOL.md`
  (verified: the file is `skills/design/inventory-design/PROTOCOL.md`).
  **Location**: Phase 5 §1
- 🔵 **Documentation**: The OpenAlex doc changes go beyond the refusal
  table: the keyless caveat, the widened `E_TOKEN_FROM_TRACKED_FILE`, and the
  runner constraints.
  **Location**: Phase 7 §2
- 🔵 **Documentation**: `ACCELERATOR_DESIGN_BROWSER_PATH` and the path rules
  have no user-facing documentation, and the remediation at
  `PROTOCOL.md:628` is stale.
  **Location**: Phase 4; Phase 7
- 🔵 **Documentation**: The retired-name grep will fail on six test files
  the plan does not list, and the grep scope differs from the acceptance
  criterion's (`docs-site/` vs `docs-site/src`).
  **Location**: Phase 7 Success Criteria
- 🔵 **Documentation**: The "ignored" check matches single lines only and
  misses variant wording.
  **Location**: Phase 7 Success Criteria
- 🔵 **Test Coverage**: A recording fake cannot verify the temporary cwd of
  `github.token_cmd`, and nothing checks that the real runner is wired in.
  **Location**: Phase 3 §4
- 🔵 **Test Coverage**: The admitted environment for the non-GitHub command
  keys, and `TRACKER_TIMEOUT`, are not pinned.
  **Location**: Phase 1 Tests
- 🔵 **Test Coverage**: Deleting the override tests leaves the insecure-file
  criterion without a regression test.
  **Location**: Phase 7 §1
- 🔵 **Test Coverage**: The output-cap tests do not show that stderr counts,
  or that the group is killed on overflow.
  **Location**: Phase 2 Tests
- 🔵 **Test Coverage**: The environment-scrub test mutates the process-global
  environment. Inject the parent environment instead.
  **Location**: Phase 2 Tests
- 🔵 **Test Coverage**: The adapter edge cases have no tests: a
  non-canonicalisable root, the parent-join fallback, and
  `HostControl::stop`.
  **Location**: Phases 4–5

#### Suggestions

- 🔵 **Architecture**: `consent-adapters` is named after the concern, but what
  actually decides its placement is its VCS dependency. Rename it, or document
  that rule.
- 🔵 **Architecture**: `TRACKER_TIMEOUT` is a consumer-specific constant
  living in the policy.
- 🔵 **Security**: Print a `notice:` when a consent key wins with
  `Origin::Environment`.
- 🔵 **Documentation**: Make the Consent keys section the single reference
  for code, meaning and remedy.
- 🔵 **Test Coverage**: Introduce the "one policy" parameterised test in
  Phase 1 and grow it each phase, so it can go red. Make the check for
  remaining `Provenance` copies permanent.
- 🔵 **Compatibility**: Decide how a case-variant path on a case-insensitive
  APFS volume should behave.

### Strengths

- ✅ The policy is pure behind ports (`ConfigAccess`, `ConfigFileTracking`,
  `ExecutablePaths`, `CommandRunner`) and stays within the existing pup rule
  for `config`.
- ✅ Tracking is tri-state and fails closed. Every error path maps to
  `Unknown`, which closes four copies that failed open with
  `unwrap_or(false)`.
- ✅ The team level is read raw on every call, so a blank personal value
  cannot hide the team refusal.
- ✅ The new `consent-adapters` crate keeps `gix` and `jj-lib` out of
  `config-adapters` and the visualiser.
- ✅ The Jira mapping is one exhaustive `for_refusal` with no new exit-code
  numbers.
- ✅ The runner kills the process group, refuses output past the cap instead
  of truncating it, scrubs the environment, and runs in a temporary cwd whose
  lifecycle is tested on every exit path.
- ✅ The design domain stays independent of config: only `HatchDecision`
  crosses into it.
- ✅ The pre-lock daemon path stays read-only, the lock is released on the
  new failure path, and the pid-recycling test is kept.
- ✅ The SessionStart envelope is byte-identical when there are no warnings,
  pinned by a golden test. Warnings carry only static key names, so there is
  no channel for prompt injection.
- ✅ Test-only `ConsentKey` descriptors check that a future key inherits the
  barrier. Tests use real git and jj repositories through `Hermetic`.
- ✅ Retired names are enforced by a grep success criterion, and the stale
  fixture comment and `ProcessControl` docs are corrected.

### Recommended Changes

1. **Redesign the Phase 5 stop path.** This addresses the liveness-only
   SIGTERM, the killing of in-use daemons, the missing escalation, the
   cycling across mixed versions, and the torn read.
   - Parse `custom_browser` in the same read as pid and start time.
   - Stop the daemon over the authenticated loopback protocol, falling back to
     a signal only on a proven `Probe` identity. Escalate to SIGKILL on the
     daemon's process group once identity is proven.
   - End the wait when `evaluate` no longer returns `Reuse(pid)`.
   - Classify an `Unrecorded` daemon by the `chromium` value its `ping`
     reports, or restart it only when the vetted browser is `Custom`.
   - Decide how to treat a busy daemon: key the state slot by browser, or
     defer the restart until idle.
   - Add harness cases where a liveness-only mismatch never signals, and where
     two browser decisions alternate.
2. **Harden the Phase 2 runner.** This addresses the unbounded joins, the cap
   race, `PATH`, `TMPDIR`, pgid ordering and the racy tests.
   - Apply the deadline to reader EOF, and kill the group on every exit path
     before reaping.
   - Decide the verdict only after the joins.
   - Spawn `/bin/bash`, and filter `PATH` against the repository roots.
   - Check the temporary directory against the roots.
   - Inject the parent environment.
   - Relax the timing bounds, and poll for pidfile death.
3. **Add `consent::resolve_command` and `consent::audit`, and widen the
   warning channel.** This addresses the duplicated command flow, the
   SessionStart re-derivation, the swallowed runner failures, the
   first-refusal masking, and the environment precedence. Have both ladders
   and the summary call the policy, and specify `into_usable`. Choose one
   fall-through rule for refused winners.
4. **Tighten `ConsentKey`.** Make its fields private behind a test-support
   constructor, and add narrowing accessors for each kind.
5. **Decide the SessionStart host.** This addresses ADR-0054, summary
   robustness and the pup rule. Either keep VCS out of the launcher, or amend
   ADR-0054 with thresholds. Make consent collection infallible in either
   case, and add `consent_adapters` to the denied list for the config
   command.
6. **Close the fail-open corners in tracking and roots.** This addresses the
   hostile `.jj`, `facts()` returning `None`, the dropped roots, and path
   vetting.
   - Also consult gix discovery, and treat a `.jj` that git tracks as
     `Unknown`.
   - Walk markers directly.
   - Treat an undeterminable root as a refusal.
   - Canonicalise the nearest existing ancestor, and refuse dangling
     symlinks.
7. **Settle the admitted environment and the recovery text.** This addresses
   `gh` on Linux, helper compatibility, and the `Unknown` recovery route.
   Decide whether to admit the XDG and D-Bus variables, which may amend the
   work item. Make `TrackingUnknown` name the `ACCELERATOR_*` route.
8. **Move docs and CHANGELOG into each phase.** This addresses the docs lag,
   the missing CHANGELOG, `allowed_sites`, the runner contract, the OpenAlex
   docs, the design environment variable, the `PROTOCOL.md` path, the grep
   scope and the "ignored" check.
9. **Fill the gaps in the test plan.** Add:
   - binary-level tests for linear-cli and work-cli;
   - tests that assert `warning:` on stderr;
   - tests of the spawn-environment wiring;
   - a concrete design-cli seam;
   - table-driven severity-matrix rows;
   - the admitted-environment pins;
   - a regression test for the insecure-file case;
   - the adapter edge cases;
   - the "one policy" test, grown from Phase 1 onwards.
10. **Fix the bookkeeping.** Add fixture regeneration to Phases 2, 3 and 5,
    add the Jira 1 → 24 change and the other accepted changes to Migration
    Notes, and state the phase DAG.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound at its core: a pure policy, adapters behind
ports, and `gix`/`jj-lib` kept out of `config-adapters`. Phase 6 contradicts
ADR-0054. The command flow and the audit are assembled by callers, so "one
policy" is only partly enforced by structure. The recorded browser is read
separately from the daemon's identity.

**Strengths**: The policy is pure within the pup rule. `consent-adapters`
isolates the VCS dependencies. Severity stays with consumers while the
vocabulary is centralised. The design domain stays independent of config. The
fail-closed port walks upward. Test-only descriptors prove the design can
evolve.

**Findings**:
- 🟡 major / high — Phase 6 §1 — Linking `consent-adapters` into the launcher
  contradicts ADR-0054 without acknowledgement. The launcher is on the hot
  path of every hook. Keep VCS out of it, for example through the tracking
  warning of a dispatched sub-binary. Otherwise amend the ADR and set limits
  that would reverse the choice.
- 🟡 major / medium — Phase 6 §2 — SessionStart builds its own consent audit.
  Add `consent::audit`, which 0227 would also use.
- 🟡 major / medium — Phases 2–3 — The command-key flow is assembled by each
  consumer. Add `consent::resolve_command`.
- 🟡 major / medium — Phase 5 §2 — The recorded browser is read separately
  from the daemon's identity. Carry it inside the recorded daemon, dropping
  `Copy` if needed.
- 🔵 minor / high — Phase 6 §1 — The config-command seam rule does not deny
  `^consent_adapters`.
- 🔵 minor / medium — Phase 1 §2 — `ConsentKey`'s public fields let callers
  bypass the catalogue.
- 🔵 minor / medium — Migration Notes — Phase independence is overstated.
  State the DAG (1→2→3; 1→4→5; 1→6; all→7) and the window in which command
  keys still fail open.
- 🔵 minor / medium — Phase 5 — The stop path gives up after 10s with no
  escalation and no recorded rationale for choosing SIGTERM over
  `daemon-stop`.
- 🔵 suggestion — The `consent-adapters` name is misleading. What decides its
  placement is its VCS dependency.
- 🔵 suggestion — `TRACKER_TIMEOUT` is consumer-specific and also serves
  GitHub. Give it a neutral name.

### Correctness

**Summary**: The model is sound. The risks cluster in three areas:
- the runner's reader joins, overflow race and success-path kill;
- the ladder's choice of error and the missing warning channel;
- the daemon's stop and wait semantics.

Smaller fail-open corners exist in `facts()`, the roots and path vetting.

**Strengths**: Tri-state tracking with every error mapped to `Unknown`. Raw
per-level reads. An upward walk with jj winning. The group is killed before
reaping. The git main-worktree root is handled. Destructive steps happen only
under the lock.

**Findings**:
- 🟡 major / high — Phase 2 §2 — Reader joins are unbounded when `bash` exits
  normally but a background child holds a pipe.
- 🟡 major / medium — Phase 2 §2 — The output-cap verdict can race a
  successful exit. Decide after the joins.
- 🟡 major / high — Phase 3 §1 — "First refusal" masks the real cause (24
  instead of 25), `TokenCmdFailed` has no warning channel, and `base_url`
  drops later refusals.
- 🟡 major / high — Phase 5 §2 — Stop signals liveness-only pids, and the
  wait for `Absent` can spin until the timeout on a recycled or EPERM pid.
- 🔵 minor / medium — Phases 3–4 — Fall-through after a refused winner is
  inconsistent between path and command keys.
- 🔵 minor / medium — Phase 5 §2 — The separate read of the recorded browser
  can straddle a rename, and a pre-lock handover can be killed by another
  executor.
- 🔵 minor / medium — Phase 1 §3 — `facts()` returning `None` is not always
  "no VCS" (non-UTF-8 root name, stat permission error).
- 🔵 minor / medium — Phase 4 §2 — `repository_roots` drops a root it cannot
  discover. A raw `/var` root does not prefix `/private/var`.
- 🔵 minor / medium — Phase 4 §1 — There is no outcome when the parent is
  also missing, or for `..`.
- 🔵 minor / medium — Phase 2 §2 — The temporary cwd follows `TMPDIR`.
- 🔵 suggestion — Phase 1 §5 — `base_url` reports a provenance refusal when a
  usable allowlist omits the host. The override named in the acceptance
  criterion does not exist.

### Security

**Summary**: The main gaps are closed. Four vectors remain where the
repository can steer the outcome:
- a committed `.jj` marker;
- a `PATH` the repository can influence;
- `setsid` escaping the group kill;
- a Phase 5 SIGTERM decided from a state file the repository can write, with
  an identity check that compares nothing.

**Strengths**: Every tracking failure maps to `Unknown`. The adapter
canonicalises before walking. Team values are read raw. The environment
allowlist excludes `BASH_ENV` and `GH_TOKEN`. Output past the cap is refused,
not truncated. SessionStart text carries only static key names. Roots that
cannot be canonicalised are included. `std::process` is denied in
`consent-adapters`.

**Findings**:
- 🟡 major / high — Phases 1 §3 and 4 §2 — A `.jj` marker committed by a
  hostile git repository hijacks the tracking query and shrinks the roots.
  Only the 0600 gate under umask 022 stands in the way.
- 🟡 major / medium — Phase 2 §2 — `bash` and the user's command are resolved
  through a `PATH` the repository can influence. Use `/bin/bash` and filter
  `PATH`.
- 🟡 major / medium — Phase 2 §2 — A `setsid` descendant escapes the group
  kill, and the reader joins can still hang. The success path does not kill
  the group.
- 🟡 major / medium — Phase 5 §2 — The new SIGTERM relies on an identity
  check that compares nothing and a state file the repository can write
  (`paths.tmp` can be set by the team).
- 🔵 minor / medium — Phase 2 §2 — `temp_dir()` follows `TMPDIR` into the
  repository.
- 🔵 minor / low — Phase 4 — A dangling final symlink passes vetting (a
  TOCTOU window), and `None` from canonicalisation is unspecified.
- 🔵 minor / medium — Phase 5 §2 — The separate browser read opens a TOCTOU
  window on reuse.
- 🔵 suggestion — Print a `notice:` when a consent key wins from the
  environment. Document that trusting `mise.toml`/`.envrc` extends consent.

### Test Coverage

**Summary**: The plan is test-first throughout, with a well-balanced pyramid.
The gaps sit at the boundaries with consumers and composition roots: linear
and work-cli, stderr warnings, and design wiring. The severity matrix is only
partly enumerated. The runner timing tests are racy.

**Strengths**: A pure core tested with in-memory ports. Test-only
descriptors. Real `Hermetic` repositories, including a corrupted index.
Boundary-exact byte counts. The `TickingClock` harness. Retired assertions
rewritten explicitly. A byte-identical golden for the hook envelope.

**Findings**:
- 🟡 major / high — Phase 3 — No behavioural tests for linear-cli or
  work-cli.
- 🟡 major / high — Phase 1 §6 — Consumer warning output is verified only by
  hand.
- 🟡 major / high — Phases 4–5 — The executor wiring from canonical path to
  spawn environment is untested. Add a shared fixture with a string
  `custom_browser`.
- 🟡 major / medium — Phase 4 — The design-cli seam is undecided
  (`--dry-run` does not exist). Inside-repository and a personal relative
  path are untested at the consumer.
- 🟡 major / high — Phase 2 — The timeout and pidfile tests are racy (a 3s
  bound, zombies, and a pidfile write race).
- 🟡 major / medium — Phase 3 — The command rows of the severity matrix and
  the env-command fallback are not enumerated.
- 🔵 minor / high — Phase 3 §4 — A recording fake cannot verify the cwd, and
  nothing tests that the real runner is wired in.
- 🔵 minor / high — Phase 1 — The admitted environment for the non-GitHub
  keys and `TRACKER_TIMEOUT` are unpinned. The Phase 7 test is tautological.
- 🔵 minor / medium — Phase 1 §5 — The override route for
  `jira.allowed_sites` is untested.
- 🔵 minor / high — Phase 7 §1 — The insecure-file criterion loses its
  regression test.
- 🔵 minor / medium — Phase 2 — The cap tests do not prove that stderr counts
  or that the group is killed on overflow.
- 🔵 minor / medium — Phase 2 — The environment-scrub test mutates the global
  environment.
- 🔵 minor / medium — Phases 4–5 — Adapter edge behaviours are untested.
- 🔵 suggestion — Phase 7 §3 — The unification test is written last and
  cannot go red. Grow it from Phase 1.

### Code Quality

**Summary**: A strong domain model and rich domain language. "One policy" is
only partly one: the command sequence is rewritten in two ladders and the
provenance rules again in SessionStart. Runner failures are split across two
error vocabularies, and the key types do not enforce their invariants.

**Strengths**: Domain names mirror the work item. Tri-state tracking. A pure
policy. An exhaustive `for_refusal`. Duplicates are deleted, not left in
place. `RecordedBrowser::Unrecorded` is named explicitly. No comments in the
sketches.

**Findings**:
- 🟡 major / high — Phases 2–3 — Non-fatal runner failures have no place in
  the refusal model.
- 🟡 major / high — Phase 3 — The provenance-then-run sequence is
  reimplemented in two ladders.
- 🟡 major / medium — Phase 6 §2 — SessionStart re-derives the rules, and
  `ConsentFacts` is undefined.
- 🟡 major / medium — Phases 1, 2 and 4 — `ConsentKey` does not enforce its
  trust invariant or distinguish kinds.
- 🔵 minor / high — Phase 3 §1 — Plaintext `Unknown` collapses into
  `TokenFromTrackedFile`.
- 🔵 minor / high — Phase 5 §2 — `stop` duplicates `terminate`.
- 🔵 minor / medium — `Consented<T>` uses an anonymous tuple, and
  `into_usable` is unspecified.
- 🔵 minor / medium — Environment-first precedence is split between
  `resolve` and the ladder.

### Compatibility

**Summary**: The wire contracts are handled carefully: frozen exit codes, an
additive `custom_browser` field, and a byte-identical envelope. The risks
depend on platform and version. Tracking depends on the repository formats
gix and jj-lib can read, the environment scrub breaks `gh` on Linux, and
daemons cycle under mixed plugin versions. Breaking changes lack CHANGELOG
entries.

**Strengths**: No new exit-code numbers. The new field is additive for both
readers. The public-API churn in `design` is minimised. The golden envelope
is unchanged. `/var` canonicalisation and worktree roots are handled. Old
daemons handle SIGTERM.

**Findings**:
- 🟡 major / medium — Phases 1 and 3 — Fail-closed tracking depends on the
  pinned gix and jj-lib reading the user's repository format (reftable,
  sha256, a newer jj store).
- 🟡 major / medium — Phases 1 and 3 — The scrubbed environment breaks `gh`
  on Linux keyrings and with a custom `XDG_CONFIG_HOME`.
- 🟡 major / medium — Phase 5 — Restarting on `Unrecorded` cycles daemons
  under mixed plugin versions. Classify by the `chromium` value `ping`
  reports.
- 🟡 major / high — Migration Notes and Phase 7 — No CHANGELOG `### Breaking`
  entries, and the docs lag.
- 🔵 minor / high — Phase 1 §6 — The Jira exit code changes from 1 to 24
  unlisted.
- 🔵 minor / medium — Phases 2–3 — Trimming narrows, and stderr now counts
  towards the cap.
- 🔵 minor / low — Phase 2 — `process_group(0)` causes SIGTTIN for helpers
  that prompt on a tty, and `setsid` escapes the group.
- 🔵 minor / high — Fixture regeneration is missing from Phases 2, 3 and 5.
- 🔵 minor / high — Phase 5 §1 — Wrong `PROTOCOL.md` path.
- 🔵 suggestion — Phase 1 §3 — Case-insensitive APFS versus the exact-match
  index lookup.

### Safety

**Summary**: Most of the plan is safety-conscious. Phase 5 is the risk: its
new signal path runs on liveness-only verdicts, does not protect a crawl in
progress, and has no recovery for a wedged daemon. The runner's success path
is still unbounded, and SessionStart becomes exposed to failures in the new
VCS code.

**Strengths**: Fail-closed tracking is tested at every layer. The group is
killed and the temporary directory is tested on every path. The pre-lock path
stays read-only. The lock is released on stop timeout. The pid-recycling test
is kept. The golden envelope is unchanged. The breaking changes are confined
to Phases 2–3.

**Findings**:
- 🟡 major / high — Phase 5 — `stop(pid)` can SIGTERM an unrelated process on
  liveness-only verdicts.
- 🟡 major / high — Phase 5 — A restart can kill a daemon another executor is
  using mid-crawl, and sessions can restart each other's daemon repeatedly.
- 🟡 major / medium — Phase 5 — A daemon that ignores SIGTERM leaves every
  later invocation stuck. Escalate to SIGKILL on the pgid, and arm the
  `daemon.js` backstop earlier.
- 🟡 major / medium — Phase 2 — Reader joins are unbounded on the success
  path.
- 🟡 major / medium — Phase 6 — Consent collection can take down the whole
  SessionStart summary, whether through an error, a panic in gix or jj-lib,
  or latency on a large monorepo.
- 🔵 minor / medium — Phase 2 — The ordering of group kill and reap, and pgid
  validity, are not pinned on the overflow path.
- 🔵 minor / medium — Phases 3 and 7 — A fail-closed `Unknown` with no bypass
  needs a recovery route in the refusal text.
- 🔵 minor / medium — Phase 5 — The upgrade restarts every warm daemon once.

### Documentation

**Summary**: The doc work is well targeted, with line ranges and a grep
criterion. However, every doc edit waits for Phase 7 while prereleases ship
per push, there are no CHANGELOG entries, the runner contract is
undocumented (so the claims about password managers become false), and
`allowed_sites` is still presented as team-shared.

**Strengths**: A concrete list of targets. Mechanical enforcement of retired
names. The `visualiser.editor` exemption is covered, and the generated mirror
is correctly identified. `PROTOCOL.md` is updated in the same phase as the
daemon change. Stale comments are corrected.

**Findings**:
- 🟡 major / high — Phase 7 — User-facing docs lag behaviour for Phases 1–6
  on the prerelease channel.
- 🟡 major / high — Migration Notes — No CHANGELOG entries.
- 🟡 major / high — Phases 2 and 7 — The runner contract is undocumented, and
  the password-manager claims become false (`secret-tool`, AWS, `op`,
  `pass`).
- 🟡 major / high — Phase 7 §2 — The configure skill presents
  `jira.allowed_sites` as team-shared (`SKILL.md:739-742`).
- 🔵 minor / high — Phase 7 §2 — The OpenAlex doc changes are broader than
  the table (the keyless caveat, and research-topic `SKILL.md:150-152`).
- 🔵 minor / medium — Phases 4 and 7 — `ACCELERATOR_DESIGN_BROWSER_PATH` has
  no documentation, and the remediation at `PROTOCOL.md:628` is stale.
- 🔵 minor / high — Phase 7 — The retired-name grep fails on unlisted test
  files in work-cli, jira-client, linear-client and research-cli, and its
  scope differs from the acceptance criterion's.
- 🔵 minor / medium — Phase 7 — The "ignored" check is bound to single lines.
- 🔵 suggestion — Make the Consent keys section the single reference for
  code, meaning and remedy. Mention the SessionStart warnings in
  `configuration.md`.

---
*Review generated by /accelerator:review-plan*

## Revisions After Pass 1 — 2026-09-25

The recommended changes were walked through in groups, with a decision taken
on each before moving on. Applied to the plan:

1. **Phase 5 redesigned as one daemon per browser** (option A of two). The
   state directory is keyed by a slot derived from the vetted browser
   (`bundled`, or `custom-<sha256 prefix>`). A browser change spawns into its
   own slot, and the old daemon idles out, so no daemon is ever signalled.
   This resolves the liveness-only SIGTERM, the killing of in-use daemons,
   the missing escalation, the torn read and version cycling. **Work item
   amended**: Requirement 10 and its two restart acceptance criteria now
   describe per-browser binding.
2. **Runner hardened.** One deadline covers reader end-of-file, the group is
   killed on every exit path, and the verdict is decided after the joins.
   `bash` is resolved through a `PATH` filtered of in-repository entries,
   because the plugin already requires bash and `/bin/bash` is absent on
   NixOS. `TMPDIR` is checked against the roots, the parent environment is
   injected, and `repository_roots` moves forward into Phase 2.
3. **One policy made structural.** A single precedence and fall-through
   rule, `Admitted`/`Usable`/`or_fallback`, `resolve_command` with opaque
   `ConsentedCommand`, `Refusal::CommandFailed`, and `consent::audit` as
   0227's entry point.
4. **Type hygiene.** Private `ConsentKey` with kind narrowing, `Tracking`
   carried on the plaintext refusal, `DEFAULT_TIMEOUT`, a placement rule for
   `consent-adapters`, and a pup-rule addition.
5. **SessionStart host: option B** (a dispatched `vcs tracking` subcommand)
   over linking VCS libraries into the launcher, keeping ADR-0054.
6. **Fail-closed tracking and roots.** The walk queries every enclosing VCS
   with a `.jj` tripwire. Roots are the union with an `is_complete` flag.
   Path vetting has six steps, and cases that cannot be established reuse
   `PathInsideRepository`.
7. **Helper compatibility: option (c).** The XDG and D-Bus locator variables
   are admitted for every command key. **Work item amended**: Requirement 5
   and its two exact-environment criteria. Also whitespace trimming,
   documented tty limits, a recovery hint in refusals, and an environment
   `notice:` line.
8. **Docs and CHANGELOG moved into each phase.** The retirement grep was
   fixed, and a multi-line "ignored" check added.
9. **Test gaps filled.** `ACCELERATOR_JIRA_ALLOWED_SITES` added (option (b)
   of two; the acceptance criterion already named an override), plus
   fixture regeneration and a phase dependency graph.

## Re-Review (Pass 2) — 2026-09-25T14:58:47+00:00

**Verdict:** REVISE

### Previously Identified Issues

Of the 25 original majors, 21 are resolved, 4 are partially resolved and none
are still present. The Phase 5 redesign, the rebuilt runner and the fail-closed
tracking and roots account for most of the resolutions.

- 🟡 **Safety + Security + Correctness**: `stop(pid)` SIGTERM on liveness-only verdicts — Resolved
- 🟡 **Safety + Compatibility**: a restart kills a daemon in use; version cycling — Resolved
- 🟡 **Safety**: no escalation for a wedged daemon — Resolved
- 🟡 **Architecture**: recorded browser read separately — Resolved
- 🟡 **Correctness + Safety + Security**: unbounded reader joins; `setsid` — Resolved (kill-on-leader-exit sequencing needs pinning; see below)
- 🟡 **Correctness**: output-cap verdict race — Resolved
- 🟡 **Code Quality + Correctness**: runner failures have no warning channel; first-refusal masking — Resolved
- 🟡 **Architecture + Code Quality**: command flow duplicated in two ladders — Partially resolved (`resolve_command` owns provenance and the runner; the ladder and GitHub still hand-roll severity)
- 🟡 **Architecture + Code Quality**: SessionStart builds its own audit — Resolved
- 🟡 **Architecture**: launcher contradicts ADR-0054 — Resolved
- 🟡 **Safety**: consent collection can take down the summary — Resolved
- 🟡 **Security**: hostile `.jj` hijacks tracking — Resolved (the tripwire's mechanism is wrong; see below)
- 🟡 **Security**: `PATH` the repository can influence — Partially resolved (entries that cannot be canonicalised, and incomplete roots, are unspecified)
- 🟡 **Code Quality**: `ConsentKey` invariant — Resolved
- 🟡 **Compatibility + Safety**: fail-closed on unreadable formats, no recovery route — Partially resolved (the bypass text names a variable that doesn't exist for `github.token_cmd`)
- 🟡 **Compatibility + Documentation**: scrubbed environment breaks helpers; runner contract undocumented — Resolved
- 🟡 **Documentation + Compatibility**: docs lag; no CHANGELOG — Resolved
- 🟡 **Documentation**: `allowed_sites` presented as team-shared — Resolved
- 🟡 **Test Coverage**: linear-cli and work-cli untested — Resolved
- 🟡 **Test Coverage**: consumer warnings checked only by hand — Partially resolved (research-cli and collaboration-cli stderr not asserted)
- 🟡 **Test Coverage**: executor spawn wiring untested — Resolved
- 🟡 **Test Coverage**: design-cli seam undecided — Resolved
- 🟡 **Test Coverage**: racy runner tests — Partially resolved (the pidfile is written into the deleted cwd)
- 🟡 **Test Coverage**: severity matrix command rows — Resolved
- 🔵 Minors and suggestions: most resolved. The "ignored" check, the retired-name grep and the adapter edge tests are partially resolved; see below.

### New Issues Introduced

#### Major

- 🟡 **Architecture + Compatibility + Correctness + Safety + Security**: `DispatchedTracking` cannot use the launcher's dispatch, which execs and may fetch
  Phase 6 §2 says "through the launcher's own dispatch", but `run_external`
  `exec`s (`launch/core.rs:293-322`) and `FetchVerifyCacheResolver` fetches
  on a cache miss. Four things need specifying:
  - a spawn-and-capture port;
  - cache-only resolution, where a miss renders its own warning rather than `TRACKING_UNKNOWN`;
  - the 2s bound covering resolution;
  - kill-and-reap of the child on timeout.

  The security lens adds that the binary must never be resolved through
  `PATH`.
- 🟡 **Architecture**: vcs-cli gains a `config` dependency through `consent-adapters`
  The VCS context is config-free today. Move the fail-closed enclosing-repository
  walk into `vcs-adapters`, returning a VCS tri-state that `consent-adapters`
  adapts. A pup rule should bar `^config` from `accelerator_vcs`.
- 🟡 **Code Quality + Architecture**: severity is still decided in several places
  The ladder, GitHub and the design hatch each hand-roll the fatal/warnings
  split. They should assemble a `Consented<String>` and call `or_fallback`.
- 🟡 **Code Quality**: the shape of `CredentialError::Consent` and `ClientError::Consent` disagrees across phases
  Phase 2 uses a tuple, Phase 3 uses `{ fatal, warnings }`, and Phase 1 leaves
  the warnings' storage unspecified. Fix one shared payload from Phase 2
  onward.
- 🟡 **Correctness**: the plaintext tracked-file refusal cannot travel in the `Refusal`-typed channels
  `TokenFromTrackedFile` is not a `Refusal`, so an early return loses the
  `TeamLevel` refusal and falling through has nothing to push. The ladder
  needs a wider refusal type.
- 🟡 **Correctness + Test Coverage**: symlinks with relative targets are refused as relative, and symlink recursion is unbounded
  `read_link` returns relative targets, as with Homebrew's links, and step 1
  refuses them. A link cycle recurses forever before canonicalisation. Resolve
  targets against the link's parent, cap the hops at 40, and add
  real-filesystem tests for chains and loops.
- 🟡 **Test Coverage**: the launcher's session-start tests need a real `vcs` sub-binary
  `nextest -p accelerator` does not build `vcs-cli`. Use `accelerator-fixture`
  to impersonate `vcs tracking` through `ACCELERATOR_VCS_BIN`, and pin the
  existing goldens that write `config.local.md`.
- 🟡 **Test Coverage**: the `notice:` line has no test
  Add binary-level tests that assert the exact line, that the value is absent,
  and that no notice appears when the personal value wins.
- 🟡 **Documentation**: the multi-line "ignored" check can never pass
  It matches "gitignored" and misses `**never** honoured`. Add word boundaries
  and allow for emphasis markers.

#### Minor

- 🔵 **Safety + Compatibility + Documentation**: the `ACCELERATOR_<KEY>` bypass does not exist for `github.token_cmd` (its bypass is `GH_TOKEN`), and the plaintext `Unknown` text names no bypass. Put the bypass variable on the descriptor.
- 🔵 **Correctness**: the `.jj` tripwire cannot fire, because `entry_by_path` is exact-match. It needs a prefix query over the index.
- 🔵 **Correctness**: `Tracking::combine` precedence is unstated. It should be Tracked > Unknown > Untracked.
- 🔵 **Correctness**: a crafted `.jj` marks the roots incomplete, which disables every `design.browser_path` and contradicts "only makes checks stricter".
- 🔵 **Correctness + Safety**: the kill-before-reap step needs `waitid(WNOWAIT)`. A successful helper with an escaped `setsid` descendant becomes `TimedOut`. Readers' fds cannot safely be closed from another thread.
- 🔵 **Correctness + Security**: the `PATH` filter keeps entries it cannot canonicalise. Its behaviour with incomplete roots is a partial filter.
- 🔵 **Code Quality**: `credential_ports()` in Phase 1 has no `provenance` parameter, yet the prose passes one.
- 🔵 **Code Quality + Architecture**: `SystemExecutablePaths` lands in Phase 2 in `consent-adapters`, but it needs no VCS, and its trait arrives in Phase 4.
- 🔵 **Code Quality**: `Tracking` embedded in findings admits an impossible `Untracked`.
- 🔵 **Code Quality**: `Usable::Value` drops the origin the notice needs.
- 🔵 **Code Quality**: kind keys come from a string plus an `Option`.
- 🔵 **Code Quality**: the runner takes a boxed closure alongside the `Environment` port.
- 🔵 **Code Quality + Architecture**: slot hashing sits in `design`, which lacks `sha2`.
- 🔵 **Architecture**: "the only route to the runner" is not enforced. `CommandPolicy` fields are public.
- 🔵 **Security**: `ACCELERATOR_JIRA_ALLOWED_SITES` has no notice until Phase 3, the notice omits non-secret values, and mise trusts by path.
- 🔵 **Security**: the locator variables are not filtered for in-repository values.
- 🔵 **Security**: slot directories could be symlinks in a directory the repository can write.
- 🔵 **Security**: the text of an audit error could inject repository-controlled content into `additionalContext`.
- 🔵 **Compatibility**: the documented environment workarounds can't pass session-dynamic secrets. Name "export the resolved token" as the workaround.
- 🔵 **Compatibility**: `ESRCH` from the success-path group kill on macOS, and caching agents killed along with the group.
- 🔵 **Safety**: filesystems that can't honour 0600 have no recovery route once the override is removed.
- 🔵 **Test Coverage**: pidfile location, `setsid` missing on macOS, `Tracking::combine` table, colocated jj+git, `PATH` relative/empty and missing-`bash` cases, the group kill on overflow, the tracker timeout pin, the custom→bundled direction, the Phase 1 table using a runner that doesn't exist yet, and a conflict between the pup rule and `DispatchedTracking`.
- 🔵 **Documentation**: the marker-deletion guidance conflicts with the retired-name grep, some override test code is unlisted, the `notice:` line and the `ACCELERATOR_JIRA_ALLOWED_SITES` syntax are undocumented, `E_COMMAND_OUTPUT_EXCEEDED` gets no row in Phase 2, the GitHub note has no home, and the `visualiser.md` range is off (it should be `:123-126`).

### Assessment

The plan is substantially better. All four structural problems from pass 1 are
resolved: the daemon signalling, the unbounded runner, the launcher linking VCS,
and "one policy" in name only. There are no critical findings. The new majors
are narrower and mostly specification gaps in surfaces this revision added:
- Phase 6's dispatch mechanism, and where the tracking walk lives;
- the symlink-target handling in path vetting;
- the ladder's refusal channel for plaintext;
- one consistent severity and error shape;
- three test and doc mechanics.

None of them calls for redesigning a phase. One more focused revision pass
should reach COMMENT or APPROVE.

## Revisions After Pass 2 — 2026-09-25/26

All pass-2 minors were applied in one batch, with the lenses' recommended
defaults. The majors were then walked through in four groups:

- **Group A (Phase 6 dispatch and tracking walk).** A `CaptureBinary` port
  was added. Resolution was first proposed cache-only, then changed to the
  normal fetch-and-verify resolver after the user pointed out that fetch
  cost and check coverage fall on the same rare session. The walk moves into
  `vcs_adapters::file_tracking`, and a pup rule bars `^config` from `vcs-cli`.
  Launcher tests use `accelerator-fixture`. **Work item 0298 raised**
  (`meta/work/0298-consolidate-the-sessionstart-hooks-behind-accelerator-hooks.md`)
  to consolidate the SessionStart hooks behind `accelerator hooks
  session-start`. Hook sequencing was kept out of 0226 (option 3).
- **Group B (severity, error shape, plaintext channel).** The policy-owned
  `Ladder`, a single `Rejection` payload, `Notice` in `Consented`/`Usable`,
  `Refusal::PlaintextFromUntrustedFile`, and consumers that only map
  `Usable`.
- **Group C (symlinks).** Relative link targets resolve against the link's
  parent, with a 40-hop bound.
- **Group D (docs check).** The "ignored" pattern uses word boundaries and
  tolerates bold markers. It was verified against the tree, and a baseline
  is recorded.

## Re-Review (Pass 3) — 2026-09-26

**Verdict:** REVISE

### Previously Identified Issues

All 9 pass-2 majors are resolved or partially resolved, and none is still
present. Of the pass-2 minors, 20 are resolved and 4 are partially resolved.

- 🟡 **Arch + Compat + Correctness + Safety + Security**: `DispatchedTracking` can't use the exec-only dispatch — Partially resolved (the `CaptureBinary` port, kill-and-reap and the fail-closed mapping are all in place, but resolution runs under dispatch's full fetch budget; see below)
- 🟡 **Architecture**: `vcs-cli` gains a `config` dependency — Resolved
- 🟡 **Code Quality + Architecture**: severity decided in several places — Resolved (`Ladder` + `or_fallback`)
- 🟡 **Code Quality**: the `Consent` error shape disagrees across phases — Resolved (`Rejection`)
- 🟡 **Correctness**: the plaintext refusal can't travel in the channel — Resolved (though the variant can't render its bypass; see below)
- 🟡 **Correctness + Test Coverage**: relative symlink targets; unbounded recursion — Resolved
- 🟡 **Test Coverage**: launcher tests need a real `vcs` binary — Resolved (`accelerator-fixture`)
- 🟡 **Test Coverage**: the `notice:` line has no test — Resolved
- 🟡 **Documentation**: the "ignored" check can never pass — Resolved (the baseline is verified)
- 🔵 Minors partially resolved:
  - the only-route-to-runner rule (now contradicted by Phase 3);
  - `credential_ports` wiring (Phase 2 uses it before Phase 3 introduces it);
  - slot aliasing (regular committed state and symlinked parents);
  - the list of remnants of the override.

### New Issues Introduced

#### Major

- 🟡 **Safety + Compatibility + Architecture + Correctness**: `config summary` can block on a fetch that the 2s deadline doesn't cover
  Verified in `fetcher.rs`: dispatch allows 3 attempts, each with a 10s
  connect timeout and a 300s whole-request timeout. On a cache miss that is
  three requests: the manifest, its signature and the asset. A slow or
  blackholed link after an upgrade can therefore outlast Claude Code's hook
  timeout and lose the whole summary. Performance Considerations wrongly says
  "bounded at 2s". The options are:
  - the existing `Fetcher::for_help` budget (3s connect, 5s total);
  - resolve from the cache only, with a separate "not checked" warning;
  - one overall deadline around resolution plus capture.
- 🟡 **Architecture + Code Quality + Security**: the `CommandRunner` import rule contradicts Phase 3
  `GitHubCredentialContext` holds a `&dyn CommandRunner`, and so do `Ladder::attempt` and the tests' runner fakes. A trait-object call needs no import, and `for_key` and `CommandKey::declared` are public, so the rule doesn't enforce the invariant either. Enforce it in the types instead, with an opaque runner handle whose run method is `pub(crate)`, or with a `run` that takes a `&ConsentedCommand`. Restrict the pup rule to *implementations* of the trait.
- 🟡 **Correctness**: reading the personal level eagerly breaks the environment-override recovery route
  Verified at `store.rs:208-210`: every personal read refuses an insecure file. Three paths read it even after an environment value has won:
  - `resolve_command` runs on every call;
  - `resolve` and `resolve_executable_path` gather the personal candidate unconditionally;
  - the `Ladder`'s existence gate runs after an environment rung has been admitted.

  `GH_TOKEN`, `ACCELERATOR_JIRA_TOKEN` and the rest therefore stop working on a 0644 file. That is a regression from today, and it contradicts Phase 7's documented recovery route.
- 🟡 **Code Quality**: the environment override is named in three places that can drift
  The catalogue's `bypass`, the consumers' literals and `TokenKeys` each name it. `bypass` also means two different things: the key's own override, and `GH_TOKEN` for GitHub. The catalogue should be the single source, with the policy reading the declared override through an `Environment` port on `ProvenanceContext`.
- 🟡 **Documentation**: the fall-through behaviour change is undocumented
  Today a failing `ACCELERATOR_*_TOKEN_CMD` is fatal. Under the `Ladder` it falls through to the next rung. Four chains still say "first non-empty wins" (configure `:794`, `:877`, `:931`; research.md `:173`), and neither the CHANGELOG nor the Migration Notes mention the change.
- 🟡 **Test Coverage**: the test-only path and command descriptors can't reach the kind-specific functions
  `CommandKey` and `ExecutablePathKey` have no `for_test` constructor, so the (b) and (c) cases of the unification acceptance criterion can't be written.
- 🟡 **Test Coverage**: no test pins `Untracked` for an ignored file in a pure jj or colocated repository
  If the tripwire fired falsely, it would refuse every personal value for jj users, and every current test would still pass.
- 🟡 **Test Coverage**: output where a fatal refusal and warnings appear together is untested in jira-cli, linear-cli, work-cli and collaboration-cli

#### Minor

- 🔵 **Correctness + Code Quality + Test Coverage**: `PlaintextFromUntrustedFile` can't render its bypass. It has no field for it, and plaintext keys have `bypass: None`.
- 🔵 **Correctness + Code Quality**: `resolve` accepts path and command keys and returns their values unvetted, because `ConsentKey::declared` only rejects `Open` keys.
- 🔵 **Correctness**: for path keys, value refusals are appended after `TeamLevel`, which breaks the ordering invariant.
- 🔵 **Correctness + Code Quality**: Phase 2 uses `credential_ports` and `CommandKey` before Phase 3 introduces them.
- 🔵 **Correctness**: the poll/`waitid` loop needs `WNOHANG`, a bounded tick, and removal of hung-up fds.
- 🔵 **Correctness**: a dangling intermediate symlink is rejoined lexically. The walk should use `symlink_metadata`.
- 🔵 **Code Quality**: `Distrust` is modelled two ways inside `Refusal`. `Tracked`/`TrackingUnknown` could be collapsed into one variant.
- 🔵 **Code Quality**: nothing in production consumes `Source`.
- 🔵 **Code Quality + Test Coverage**: `HatchDecision` has no field for the notice.
- 🔵 **Code Quality**: the vocabulary each phase introduces disagrees with the Desired End State. Each variant should arrive in the phase whose red step needs it.
- 🔵 **Code Quality**: the sketches omit `Consented::map` and constructors for `RepositoryRoots`.
- 🔵 **Architecture**: `repository_roots` walks VCS markers itself in `consent-adapters`. It should reuse `vcs-adapters`.
- 🔵 **Architecture**: stale End State wording. Phase 1's nextest run should cover `vcs-adapters` and `vcs`, and the `vcs` public-API fixture needs regenerating.
- 🔵 **Architecture** (suggestion): `config::consent` is turning into a general credential engine.
- 🔵 **Compatibility**: `rustix::event::poll` needs the `event` feature, which the workspace doesn't enable.
- 🔵 **Compatibility**: the GitHub tracked-file refusals aren't listed as breaking. The plan should also decide whether a tracked plaintext `github.token` is refused.
- 🔵 **Safety**: an offline first session raises a false `TRACKING_UNKNOWN` alarm.
- 🔵 **Safety**: SIGKILL on leader exit cuts off helpers' background writes. SIGTERM, then grace, then SIGKILL is gentler.
- 🔵 **Safety** (suggestions): `CaptureBinary` should use a null stdin and cap its output, and the message for a symlinked slot should name the remedy.
- 🔵 **Security**: the notice prints environment values unescaped (CR, ANSI and newlines).
- 🔵 **Security**: committed regular slot state, and symlinked parent directories, can still alias a slot.
- 🔵 **Test Coverage**:
  - the allowlist exit code 24 isn't pinned, nor are the usable-fallback rows;
  - how the fixture chooses its answer, and wiring it across the whole summary;
  - the `TMPDIR` case mutates the process environment;
  - `GH_CONFIG_DIR` and `XDG_RUNTIME_DIR` aren't covered by the filter tests;
  - `audit` has no negative cases;
  - suggestions: tighten the timeout bound, and give each pup rule a red step.
- 🔵 **Documentation**:
  - the `rg` checks match gitignored generated files in jj workspaces (use `--no-require-git` or globs);
  - the cited `tasks/README.md` subcommand checklist doesn't exist;
  - the Phase 2 GitHub note has no content yet;
  - the Consent keys table's coverage of the two surviving codes is unclear;
  - SessionStart's documented limit has no home in the docs;
  - the `STATE_DIR` row in `PROTOCOL.md` is stale, and `design.md` misses the notice and the fallback;
  - suggestions: configuration.md still says "no bypass", and a few override remnants are unlisted.

### Assessment

The plan continues to converge. The structural redesigns from pass 2 held up:
- `Ladder` and `Rejection`;
- the tracking walk in `vcs-adapters`;
- per-browser slots;
- symlink resolution.

No finding asks for a phase to be redesigned. Two of the new majors reopen or
refine decisions made in pass 2:
- The fetch budget reverses the premise that fetching is cheap. The actual
  budget allows up to 300s per request.
- The eager personal read is a real regression that the unified policy
  introduced.

The other six majors are local fixes: the runner-route enforcement, one source
for override names, a doc entry for fall-through, and three test gaps. All 8
majors are specification or test gaps rather than design flaws. Another
focused pass should reach COMMENT.

## Revisions After Pass 3 — 2026-09-26/27

The majors were walked through in five groups, and the minors were then
applied in one batch:

1. **Fetch budget: option (a).** `DispatchedTracking` resolves with the
   `Fetcher::for_help` budget. The user noted that `vcs guard` already does
   the same fetch.
2. **Eager personal reads.** Personal reads are lazy in the policy and the
   `Ladder` closures, and the team level is always read. A consumer no
   longer mentions a tracked personal file when the environment wins.
3. **Runner route.** An opaque `Runner` handle, and a `pub(crate)`
   `CommandPolicy::for_key`.
4. **Override names.** The catalogue's `environment`/`recovery` fields are
   the single source, `Refusal` carries `&'static ExtraKey`, and an `rg`
   criterion has a verified baseline.
5. **Tests and docs.** `for_test` on the key kinds, jj `Untracked` pins,
   fatal-plus-warnings tests, and fall-through documented.

The minors batch covered: `UntrustedPersonalFile`; removal of `Source`;
variants introduced per phase; `map` and the root constructors;
`repository_roots` in `vcs-adapters`; SIGTERM, then a grace period, then
SIGKILL; a `WNOHANG` poll tick; the `symlink_metadata` ancestor walk; the
refusal of a tracked GitHub plaintext token; `--no-require-git` on every rg
check; the `tasks/README.md` subcommand section; and a set of test and doc
minors. Deliberately not applied: a separate note for an offline first
session (Group 1 had kept `Unknown`), and SessionStart listing consent keys
supplied by the environment (the hook's environment can differ from the
shell's).

## Re-Review (Pass 4) — 2026-09-26

**Verdict:** REVISE (3 majors, exactly the threshold)

### Previously Identified Issues

Seven of the eight pass-3 majors are resolved, and the eighth is partially
resolved:

- 🟡 **Safety + Compat + Arch + Correctness**: `config summary` fetch unbounded — Resolved (`Fetcher::for_help`). The 5s-per-request asset download is now raised as a minor.
- 🟡 **Arch + CQ + Security**: the `CommandRunner` import rule contradicts Phase 3 — Resolved in the types (opaque `Runner`, `pub(crate)` `for_key`, compile-fail doctest). Phase 2's direct wiring now breaks the pup rule instead (new major).
- 🟡 **Correctness**: eager personal reads break the environment-override recovery route — Partially resolved. The policy's reads are now lazy, but consumers still read ordinary keys from the personal level (new major).
- 🟡 **Code Quality**: override names in three places — Resolved (catalogue `environment`/`recovery`, plus an rg criterion).
- 🟡 **Documentation**: fall-through undocumented — Resolved.
- 🟡 **Test Coverage**: test-only descriptors can't reach kind functions — Resolved.
- 🟡 **Test Coverage**: jj `Untracked` pin — Resolved.
- 🟡 **Test Coverage**: fatal-plus-warnings output — Resolved.
- 🔵 Minors: nearly all resolved. Partially resolved: the test-support scope, `Origin` without a consumer, per-phase vocabulary, the temporary-base signature, the `paths.tmp` chain, and the remaining remnant comments.

### New Issues Introduced

#### Major

- 🟡 **Correctness + Test Coverage + Compatibility**: environment overrides cannot recover from an insecure `config.local.md`
  Verified: `ConfigService::get(key, None)` reads `Level::Personal` first (`service.rs:487`), and jira-client reads `jira.site` before the token (`auth.rs:67`). The same holds for `jira.email`, `linear.team_id` and research's `paths.tmp`. The route fails end to end today and after this plan alike, so pass 3's "regression" framing was wrong. The lazy reads are harmless, but on their own they don't deliver the route. Phase 1/3/4 0644 binary tests, Phase 7's `LocalPermsInsecure` text and the Migration Notes promise something unachievable. **Needs a decision**: narrow the claim, or make non-consent reads tolerate the refusal.
- 🟡 **Architecture (+ Security minor)**: Phase 2's composition roots build `BashCommandRunner` directly, breaking Phase 2's own pup rule. Suggested fix: a `consent_adapters` runner factory in Phase 2.
- 🟡 **Architecture (+ Test Coverage minor)**: the rule "no crate outside `vcs-adapters` calls `is_tracked`" can't be expressed as an import restriction. It's an inherent method, and `InProcessProbe` is imported legitimately elsewhere. Suggested fix: make `is_tracked`/`tracks_any_under` `pub(crate)`, keep `file_tracking` as the public entry, and reword the criterion. A related point: `file_tracking` itself should be restricted to `consent-adapters` and `vcs-cli`.

#### Minor

- 🔵 **Safety + Compatibility**: the 5s help budget likely can't download the multi-MB `vcs` asset, so the false `TRACKING_UNKNOWN` alarm hits ordinary links too, not only offline users. Options: a distinct "not checked" note, cache-only asset resolution, or a sized asset budget.
- 🔵 **CQ + Arch + Test + Correctness + Security**: the scope of the `test-support` feature contradicts config-adapters' use of `CommandPolicy::for_test`, and nothing guards it. Suggested: restrict `ConsentKey::for_test` to `Consent`, list the permitted enablers, and add a cargo-deny or `tasks` check.
- 🔵 **CQ + Test**: `BashCommandRunner::new` has two parameters in the prose but three in the tests (temporary base).
- 🔵 **CQ + Arch**: refusal ordering is implemented three times (`resolve`, `resolve_executable_path`, `Ladder::finish`), and Phase 4 cites `Ladder`, which it doesn't depend on. Suggested: one ordering constructor in Phase 1.
- 🔵 **Code Quality**:
  - `ConsentKey` should wrap `&'static ExtraKey`, and `trust_of` is unused;
  - `Origin`/`Origin::Team` have no consumer;
  - the nested `Result<Option<Result<..>>>` rung type (suggest a `Rung<T>` enum);
  - `CommandFailed` drops its cause;
  - `environment` and `admitted_environment` look alike, and "bypass" wording survives;
  - three context structs repeat the same port clump;
  - path variants arrive in Phase 1, ahead of their producer.
- 🔵 **Correctness**:
  - the `Ladder` error channel loses `LocalPermsInsecure` and drops refusals already gathered;
  - `audit` has no existence probe for `config.local.md`;
  - the grace period isn't capped at the deadline, and output written during it joins the token.
- 🔵 **Safety**:
  - `CaptureBinary` doesn't bound the read after the leader exits, and stderr handling is unspecified;
  - the grace period is defined by pipe state, not by whether the group is still alive;
  - the symlink chain check fails deliberately symlinked tmp bases, and its remedy text misleads.
- 🔵 **Security**:
  - a committed team `github.token` silently displaces a personal `github.token_cmd` (predates this plan);
  - nothing guards against a consumer reading a consent key raw;
  - the slot chain check is undefined for an absolute or `..` `paths.tmp`;
  - suggestion: ship Phases 4 and 5 in the same release.
- 🔵 **Compatibility**:
  - where `E_TOKEN_MALFORMED` sits under the `Ladder` is unspecified;
  - `process_group(0)` stops SIGINT/SIGTERM reaching the helper, so a killed CLI can orphan it;
  - `consent-adapters`, `vcs` and `launcher/tests/config_help.rs` fixture updates are missing;
  - suggestion: treat a zero-byte read, `POLLERR` and `POLLNVAL` as closure.
- 🔵 **Test Coverage**:
  - no exit-code rows for Phase 2/3 variants;
  - the intermediate chain symlink is untested;
  - suggestion: tighten the `/dev/tty` and output-cap assertions.
- 🔵 **Architecture**:
  - `vcs_adapters::repository_roots` is tested only through consent-adapters, and where `RootsAnswer` lives is unstated;
  - suggestion: give the `vcs tracking` stdout protocol a shared constant or a contract test.
- 🔵 **Documentation + Compatibility**: CHANGELOG and Migration gaps:
  - Phase 1: allowlist code renames, and the tracked-Atlassian relaxation;
  - Phase 2: truncation becomes refusal, dropped `PATH` entries, and the widened trimming;
  - Phase 3: a GitHub personal command beside a team command now runs;
  - Phase 4: browser-path fallback in the Migration Notes;
  - research.md `:103` exit table, and the `E_TOKEN_CMD_FAILED` row split;
  - collaboration.md ladder specifics;
  - stale remnant comments, and the `fetch_command.rs:192-198` citation.

### Assessment

The plan has converged structurally. No lens proposes a redesign, and every
pass-3 major except one is resolved. Of the three new majors, two are
enforcement mechanics with obvious fixes (a runner factory, and visibility
instead of a pup rule). The third, the recovery route, is a scope decision
exposed by verifying a pass-3 claim: the environment-override route for an
insecure personal file has never worked end to end for Jira, Linear or
research. After that decision and a batch of minors, a fifth pass should
reach COMMENT.

## Revisions After Pass 4 — 2026-09-27

Pass 4's claim that the recovery route did not work was verified in code:
`ConfigService::get(key, None)` reads `Level::Personal` first, and
jira-client reads `jira.site` before the token. Every finding was then
walked through one item at a time:

1. **Recovery route: option (b), service-wide (i).** Every config reader
   ignores an insecure `config.local.md` with an `E_LOCAL_PERMS_INSECURE`
   warning (`ConfigError::InsecurePersonalFile`, a service record, and
   `Refusal::InsecurePersonalFile` mapped to exit 29). The store still never
   reads the file or writes to it, SessionStart warns in both fields, and
   "Softening the SessionStart hook" was removed from the non-goals. **Work
   item amended**: Requirement 2, a new acceptance criterion, and the open
   question resolved.
2. **Runner wiring.** A `consent_adapters::command_runner` factory in
   Phase 2, `BashCommandRunner::new(roots, parent, temp_base)`, and a
   `test-support` feature enabled only by `config` and `config-adapters`,
   enforced by a `tasks/` lint.
3. **Tracking rule.** `is_tracked` and `tracks_any_under` become
   `pub(crate)`, an `rg` guard on `file_tracking` is added, and the plan
   records how the acceptance criterion maps onto the implementation.
4. **`vcs` download: option (a).** `TrackingCheck::Unchecked` gives a
   `PersonalFileUnchecked` note in `additionalContext` only.
5. **Type model.** `ConsentKey(&'static ExtraKey)` with `catalogue::declared`,
   no `Origin`, `Rung<T>`, `overrides`, one `CredentialContext`, variants in
   the phases that produce them, and `FailureCause`.
6. **Policy mechanics.** `Consented::from_candidates` owns ordering,
   `Aborted` keeps its warnings, and `files` in `ProvenanceContext` gives
   `audit` an existence probe.
7. **Process lifecycle, with option (b) for interrupts.** A grace period
   capped at the deadline and ended by group liveness; extra end-of-file
   signals; `SIGINT`/`SIGTERM`/`SIGHUP` forwarded through `signal-hook`
   (a new dependency) and re-raised; and a tighter `CaptureBinary`.
8. **State slot.** The symlink check covers only the leaf and slot, a
   symlinked tmp base and an absolute `paths.tmp` are allowed, and
   Phases 4 and 5 ship in the same release.
9. **Policy bypasses: option (a).** The GitHub ladder matches the trackers,
   with the team token last and gated (Breaking). An `rg` guard on raw
   consent-key reads has a recorded baseline.
10. **Malformed tokens: option (a).** `Refusal::MalformedToken` falls
    through. Exit codes are preserved, and GitHub gains the check.
11. **Verification.** Per-phase exit-code tables, bounded assertions, root
    tests in `vcs-adapters`, the fixture list corrected (`kernel`, `vcs`,
    `consent-adapters`), and a `kernel::TrackingAnswer` contract test.
12. **Docs and CHANGELOG.** research.md tables, Phase 1–4 CHANGELOG and
    Migration gaps, collaboration.md specifics, and stale comments and
    citations.

**State:** every pass-4 finding is resolved in the plan, which has not yet
been re-reviewed. The frontmatter verdict stays at pass 4's REVISE until a
fifth pass confirms these revisions. The largest new surfaces to re-check
are item 1 (service-wide insecure-file handling) and item 7 (interrupt
forwarding).

## Re-Review (Pass 5) — 2026-09-27T07:10:08+00:00

**Verdict:** REVISE (10 distinct majors, against a threshold of 3)

### Previously Identified Issues

Most pass-4 findings are resolved, and so are the three pass-4 majors in
their original form. What stays open are consequences of how two of the
fixes were designed: the service-wide ignore of an insecure file, and
interrupt forwarding.

- 🟡 **Correctness + Test Coverage + Compatibility**: environment overrides cannot recover from an insecure `config.local.md` — Partially resolved. The service-wide ignore reaches the `jira.site`, `jira.email`, Linear and research `paths.tmp` reads, all verified to go through `ConfigService`. It does not reach the launcher's raw-store views, it removes `set`'s only write refusal, and the policy cannot see it (new majors below).
- 🟡 **Architecture**: Phase 2's roots build `BashCommandRunner` directly — Resolved (`consent_adapters::command_runner`).
- 🟡 **Architecture**: the `is_tracked` rule can't be an import restriction — Resolved (`pub(crate)`, plus an `rg` guard on `file_tracking`).
- 🔵 **Safety + Compatibility**: the 5s help budget can't download `vcs` — Resolved (`TrackingCheck::Unchecked` becomes a note in `additionalContext` only). The binary test for it is unreachable (new major).
- 🔵 **Cross-lens**: `test-support` scope — Resolved (`tasks/` lint with a red step). A forwarding-feature gap is raised as a suggestion.
- 🔵 **Code Quality**: type model — Mostly resolved (`ConsentKey`, `Rung<T>`, `from_candidates`, per-phase variants). Partially resolved: `FailureCause` against `CommandFailure`, and the context-struct clump, where `run` and `attempt` re-split the runner and timeout.
- 🔵 **Correctness**: `Ladder` error channel, `audit` existence probe and grace-period cap — Resolved.
- 🔵 **Safety**: `CaptureBinary` bounds, and the symlink chain on the tmp base — Resolved. The grace period ended by group liveness is Partially resolved: the check works on macOS only.
- 🔵 **Security**: committed team `github.token` displacing a personal `token_cmd`, raw consent-key reads, and the `paths.tmp` chain — Resolved or Partially resolved. The team-token gate admits an ignored insecure file (new minor).
- 🔵 **Compatibility**: `process_group(0)` orphaning is Partially resolved (the signal-hook defect below). Fixtures and `POLLERR`/`POLLNVAL` handling are Resolved. The `E_TOKEN_MALFORMED` placement is Partially resolved: Linear is contradictory.
- 🔵 **Test Coverage**: exit-code rows, `/dev/tty` and output-cap bounds, and root tests — Resolved.
- 🔵 **Documentation + Compatibility**: CHANGELOG and Migration gaps — Mostly resolved. The code renames and the tracked-Atlassian relaxation are still missing from the Migration Notes, and `resolve_github_token`'s rustdoc is unscheduled.

### New Issues Introduced

#### Major

- 🟡 **Safety + Compatibility + Architecture + Correctness**: unregistering signal-hook handlers leaves `SIGINT`/`SIGTERM`/`SIGHUP` ignored for the rest of the process
  Verified: the `signal-hook-registry` 1.4.8 docs (`lib.rs:694-701`) say `unregister` does not restore the previous disposition. After the first `*_cmd` run, a long `work sync` or crawl can't be stopped with Ctrl-C or a supervisor's SIGTERM. Registration also overrides an inherited `SIG_IGN`, so `nohup` and background jobs now die. Fix: register once per process with a "run active" condition (`flag::register_conditional_default` or equivalent), or save and restore each `sigaction`. Skip signals that are `SIG_IGN`, re-check the flag after the loop, and test default termination after a completed run.
- 🟡 **Architecture + Correctness**: the launcher's block views read the raw store and bypass the service's ignore
  Verified: `summary.rs:45,127`, `dump.rs:51` and `context.rs:28` call `ReadConfigLevel`/`ReadContent` on `FileConfigStore`, which calls `require_secure_personal_file` (`store.rs:210,260`). `agents.rs:69` does the same. SessionStart, `config dump`, `config agents` and `config context` would still fail on a 0644 file. Fix: a tolerant decorator over the level and content ports, composed once in `compose` for both the service and the block views.
- 🟡 **Correctness**: the ignore removes the only write refusal, so `config set --personal` would overwrite an insecure file
  Verified: `FileConfigStore::write` (`store.rs:231-254`) never checks the mode, and `atomic_write` clamps an existing file to 0600. The refusal comes solely from `ConfigService::set`'s `self.reader.read(level)?` (`service.rs:503`). If that read goes through the tolerant path, `set` sees `None` and writes a single-key frontmatter over the user's file. The plan's claim that "the store still refuses writes" is wrong. Fix: keep `set`'s read strict, or add the mode check to `write` for `Level::Personal`, and correct the text.
- 🟡 **Code Quality + Architecture + Correctness**: the ignored-file fact travels through four channels, and the policy's port sees none of them
  The four channels are `ConfigError`, the service record, `Refusal` and `personal_config_exists`. `ignored_personal_file()` is inherent on `ConfigService`, not on `ConfigAccess`, so through `&dyn ConfigAccess` the policy and `Ladder` see `Absent`. The fatal `E_LOCAL_PERMS_INSECURE` (exit 29) would never fire in production, while `FixedConfig` tests pass. Fix: one fact on the port (for example `ConfigAccess::ignored_personal_file()` or a `Resolved::Ignored`), fed by the decorator above. Derive every `InsecurePersonalFile` from it, including the ladder's existence gate and `audit`, and add one test through the real service.
- 🟡 **Safety + Compatibility (+ Architecture minor)**: the service-wide ignore turns fail-closed into silent fallback for writers and unwired consumers
  `work sync` push and pull, `work create`/`update`, migrate (`migrate-adapters/src/context.rs`), design-cli and corpus-cli would act on team-only `jira.site`, `linear.team_id`, `work.integration`, `paths.*` and `max_items`. Only four consumers are wired to print the record. Fix, a decision: either make mutating commands fail closed on the ignored record, or surface the record once from `config_adapters::compose` with an `rg` or test guard. Either way, list every composition root and pin `work sync` behaviour.
- 🟡 **Test Coverage + Compatibility + Documentation**: Linear's exit mapping contradicts itself
  Phase 3 §3 says "Linear keeps flattening to `NO_TOKEN`". The Phase 1 table and tests expect 29 for an insecure file and 25 for command variants, and Phase 3 promises 27 for `MalformedToken`. Today `linear-cli/src/exit_codes.rs:161-164` gives 24 for all of these, and once linear-client's own check is deleted, nothing can produce 27. Fix: decide on an exhaustive linear-cli `for_refusal`, or keep flattening. Then pin the table and record every moved code in the CHANGELOG, including collaboration-cli's 1 → 2 for an insecure file.
- 🟡 **Test Coverage**: Phase 1 is not green on its own
  Existing tests asserting `LocalPermsInsecure` survive until Phase 7: `config/tests/credentials.rs:430-512`, `config-adapters/tests/credentials.rs:260-341` and `research-cli/tests/fetch_openalex.rs:659-666`. Ladder behaviour in Phases 1–2 for a 0644 file beside a team token is unpinned. Fix: rewrite these tests in Phase 1's red step and add interim ladder cases. Relatedly (Correctness minor), remove the `ACCELERATOR_ALLOW_INSECURE_LOCAL` bypass from `personal_config_exists` in Phase 1, not Phase 7.
- 🟡 **Test Coverage**: the Phase 6 `Unchecked` binary test cannot reach `Unchecked`
  A missing `ACCELERATOR_VCS_BIN` resolves unverified (`outbound/mod.rs:21-47`) and fails at spawn, and the plan maps a spawn failure to `Unknown`. Fix: reach `Unchecked` through an injected `ResolveBinary` in a launcher-level test, and assert that the missing override gives `TRACKING_UNKNOWN` in both fields.
- 🟡 **Compatibility (+ Safety, Correctness, Test Coverage minors)**: on Linux the grace period never ends early
  The deliberately unreaped zombie leader keeps `kill(-pgid, 0)` succeeding, so every Linux helper run waits the full `min(1s, remaining)`. Fix: exclude the leader from the liveness check, or reap it once no other member remains. Add a bounded test that a trivial helper returns in under ~300ms on both platforms.
- 🟡 **Documentation**: the work item still says a team `github.token` wins over `github.token_cmd`
  This contradicts Phase 3 §4's Breaking reorder. The work item's out-of-scope list also claims GitHub has a tracked-file refusal today, which it lacks. Fix: amend the work item's Assumptions and Out of scope, and its `last_updated_note`.

#### Minor

- 🔵 **Security**: the team plaintext rung treats an ignored insecure file as absent, so a committed team token comes back silently. That is the account-substitution risk Phase 3 §4 cites for the GitHub reorder. Treat an ignored file as present for this gate, or warn and add a Migration Notes entry.
- 🔵 **Correctness**:
  - A Phase 1 test wants `InsecurePersonalFile` beside an environment winner, which the lazy reads rule out.
  - Rungs 3 and 4 push the refusal twice, which prints as both a warning and a fatal line.
  - `audit` skips the tracking check for an insecure file, although a committed 0644 file is the common tracked case.
- 🔵 **Safety**:
  - Interrupt windows: a signal before spawn or after the last tick, `poll` returning `EINTR`, and concurrent runners.
  - `CaptureBinary`'s kill-before-reap order is unstated, and `vcs tracking` has no deadline of its own.
- 🔵 **Compatibility**: `ConfigError::InsecurePersonalFile` has no stated `is_refusal` classification (`--fail-safe` absorption), and `config set --personal`'s exit code is unpinned.
- 🔵 **Architecture**: `InsecurePersonalFile` uses `RefusalReason::Provenance` but maps to 29, so §6's reason-based exit rule leaks. Give it its own reason, or map by variant.
- 🔵 **Code Quality**:
  - `FailureCause` can't be built from `CommandFailure::Failed(String)`, and `CouldNotRun` and `CouldNotStart` name one event two ways.
  - `tracking` and `check` are overlapping port methods.
  - `AuditFinding` lacks an ignored-file variant, and the SessionStart warning has two sources.
  - `run`/`attempt` and the design seam re-split the port clump.
  - Stale text: "with its origin", "propagates as a `ConfigError`", `resolve` for `resolve_executable_path`, and "bypasses".
- 🔵 **Test Coverage**:
  - The `from_candidates` test sits in an integration crate but targets a `pub(crate)` item.
  - Only `SIGINT` is tested, handler unregistration is untested, and the harness binary's home is unnamed.
  - The GitHub tracked-plaintext and team-token-beside-ignored-file branches are untested.
  - `effective*`, dedup and visualiser startup are untested.
- 🔵 **Documentation**:
  - The refusal-code count should be eleven, since `E_TOKEN_MALFORMED` is missing.
  - Phase 1 leaves six docs saying every read of an insecure file is refused.
  - Interrupt forwarding is missing from the CHANGELOG and Migration Notes.
  - The leaf/slot symlink refusal is missing from the CHANGELOG and `PROTOCOL.md`.
  - The `resolve_github_token` rustdoc still describes the old ladder.
  - The linear-client `auth.rs:153-161` citation is ambiguous about the quote and backslash checks.
  - The Migration Notes omit the code renames and several changes listed in the CHANGELOG.
- 🔵 **Suggestions**:
  - Narrow the raw-read `rg` lookbehind so it no longer exempts `catalogue::declared`.
  - Make the `test-support` lint scan `[features]` forwarding.
  - Promote the `rg` guards to `cli:check` lints.
  - Rename `CommandCandidates::refusals()` to `team_level_refusals()`.
  - Qualify "byte-identical" to cover context notes as well.

### Assessment

The pass-4 majors in their original form are resolved, and the policy
core (the key types, `from_candidates`, the runner factory, `pub(crate)`
tracking and the `kernel::TrackingAnswer` contract) is now stable. No lens
proposes redesigning it. The remaining majors come from two new surfaces
that pass 4 introduced.

**Service-wide ignore.** Five of the ten majors come from placing the
insecure-file ignore in `ConfigService` rather than at the port boundary.
Moving it into a tolerant decorator over `ReadConfigLevel`/`ReadContent`
that exposes one "ignored" fact on `ConfigAccess` would resolve the
launcher bypass, the policy's blindness and the four-channel split
together. It still needs a strict write path and a decision on writers
failing closed.

**Interrupt forwarding.** The design needs a register-once, conditional
handler. Its Linux grace-period check needs a leader-excluding probe.

The rest are contradictions within the plan (Linear exit codes, the work
item's GitHub assumption) and test sequencing (Phase 1 green, the
`Unchecked` route). A sixth pass after these fixes should reach COMMENT.

## Revisions After Pass 5 — 2026-09-27

The majors were worked through in four groups, then the minors in one
batch.

1. **Insecure-file surface.** Four decisions:
   - an eager probe in `compose` gives one immutable `PersonalFile` fact on
     `ConfigAccess`, and a stateless `ScreenedStore` serves both the
     service and the launcher's block views;
   - project-tree writers (`migrate`, work `create`/`update`/`sync`,
     `config set --personal`) fail closed;
   - the team plaintext rung runs only when the personal file is `Absent`;
   - `FileConfigStore::write` refuses an insecure personal file.

   `personal_config_exists` is replaced by the fact, which makes the
   override inert from Phase 1. `RefusalReason::PersonalFile` and
   `AuditFinding::PersonalFileIgnored` are added. `audit` also checks
   tracking for an ignored file. Each composition root prints the warning
   once, and consumers drop the duplicate. The Phase 1 red step rewrites
   the existing `LocalPermsInsecure` tests and pins the interim ladder.
   Six docs move to Phase 1.
2. **Linear exits: align with Jira's reasons.** linear-cli gains an
   exhaustive `for_refusal` in Phase 3 (24/25/27/29). The moves from 24 to
   25 and from 24 to 29 are recorded as Breaking, alongside collaboration's
   move from 1 to 2. Only the control-character branch of `validate_token`
   moves.
3. **Interrupts and the grace period.** Handlers are installed once and
   never unregistered, and emulate the default action while idle. Inherited
   `SIG_IGN` is skipped. Handlers register before spawn, the flag is
   re-checked after the loop, `EINTR` counts as a tick, and a process-wide
   lock serialises runs. The harness is the re-executed test binary, and
   the tests cover all three signals. The grace period reaps the leader
   after `SIGTERM`, so `ESRCH` ends it on Linux too, and a 300ms bound pins
   this. Interrupt forwarding is added to the CHANGELOG and Migration Notes.
4. **Tests and spec.** `Unchecked` is reached through an injected resolver
   in a launcher unit test, and a missing `ACCELERATOR_VCS_BIN` pins
   `TRACKING_UNKNOWN`. The work item's GitHub assumption, out-of-scope note,
   Requirement 2 and a writer criterion are amended.

The minors batch covered:
- a structured `FailureCause`/`StartFailure` shared with `CommandFailure`;
- `CommandExecution`;
- `team_level_refusals`;
- the `ProvenanceContext` design seam;
- stale text;
- eleven codes;
- a note that `tracking` answers `Unknown` for `DispatchedTracking`;
- the `from_candidates` unit-test location;
- the GitHub branch tests;
- the narrowed `rg` lookbehind;
- `[features]` forwarding in the `test-support` lint;
- the `resolve_github_token` rustdoc;
- the symlink refusal in the CHANGELOG and `PROTOCOL.md`;
- `CaptureBinary` kill-before-reap;
- the Migration Notes renames;
- "byte-identical" qualified.

Deliberately not applied:
- promoting the `rg` guards to `cli:check` lints, because the type
  guarantees carry regression protection;
- a deadline inside `vcs tracking`, now documented as a limit instead;
- a single-method tracking port.

**State:** the frontmatter verdict stays at pass 5's REVISE until a sixth
pass confirms these revisions. The largest new surfaces to re-check are
`ScreenedStore` and the writer gating, the once-installed signal handlers,
and Linear's moved exit codes.

## Re-Review (Pass 6, narrow) — 2026-09-27T07:44:01+00:00

**Verdict:** REVISE (5 majors against a threshold of 3; one of them
challenges a pass-5 decision)

Scope: Correctness, Safety and Compatibility only, reviewing the three
surfaces pass 5 introduced: the insecure-file screen, signal handling, and
Linear's exit codes.

### Previously Identified Issues

- 🟡 **Safety + Compat + Arch + Correctness**: signal-hook `unregister` leaves signals ignored — Resolved. Handlers are installed once, with a conditional default. Error-path gaps remain (new major below).
- 🟡 **Arch + Correctness**: the launcher's block views bypass the ignore — Resolved (`ScreenedStore`). The `Composed` shape still needs specifying (minor).
- 🟡 **Correctness**: `set --personal` could overwrite an insecure file — Resolved. The service reads through the screen and writes through the checked store, verified against `service.rs:497-517` and `store.rs:231-254`.
- 🟡 **CQ + Arch + Correctness**: four channels, with the policy blind to them — Resolved (one `PersonalFile` fact on `ConfigAccess`).
- 🟡 **Safety + Compat**: writers and unwired consumers fall back silently — Partially resolved. See the new majors on collaboration-cli, migrate-cli scoping and tracker writes.
- 🟡 **Test + Compat + Doc**: Linear exit mapping — Partially resolved. The 24→27 move for a control character is undisclosed (new major).
- 🟡 **Compat**: Linux grace period — Partially resolved. Fixed on normal exit, but the interrupt path still reaps only after `SIGKILL` (minor).

### New Issues Introduced

#### Major

- 🟡 **Safety (+ Correctness minor)**: the run state is not restored on error or panic, and a late signal can be lost or go stale. A spawn failure, a setup error or a panic after the run is marked active leaves the idle flag false, so from then on `SIGINT`/`SIGTERM`/`SIGHUP` are only recorded. A signal that arrives between the last check and going idle is swallowed, or kills the next run's healthy helper. Fix: make the active state an RAII guard. It clears the record on entry. On drop, including during unwinding, it sets idle, then swaps the record and emulates the default if a signal was recorded. Tests: after a spawn failure, and after `TimedOut`/`OutputExceeded`, a signal still ends the process.
- 🟡 **Correctness**: collaboration-cli uses a committed team `github.token` beside an ignored file throughout Phases 1–2. Verified: `resolve_github_token` reads `github.token` through `ConfigAccess` at any level (`auth.rs:60`). Once the store is screened, the team value wins. This contradicts Phase 1's Security CHANGELOG line. Fix: in Phase 1, gate collaboration-cli's plaintext config read on `personal_file()` being `Absent` (or fail closed on `Ignored` with no `GH_TOKEN`/`GITHUB_TOKEN`), add a binary test, and move the note about collaboration's 1→2 exit move to Phase 1.
- 🟡 **Compatibility + Correctness**: Linear's control-character token moves from 24 to 27 without being recorded. Verified: today `accept` (`credentials.rs:398-402`) refuses it before linear-client's check runs, and `for_client` maps it to 24. The plan says Linear "keeps" 27. Fix: record it as a third Breaking move in the CHANGELOG and Migration Notes, pin 24 in the red step, and rewrite `linear-client/tests/auth.rs:190-207` down to the quote and backslash rows.
- 🟡 **Compatibility**: migrate-cli's fail-closed gate is not scoped. The SessionStart hook runs `migrate --discoverability-hook --format=hook --fail-safe` (verified, `hooks.json:27`), and `fail_safe` is never read. A gate on `run` would break session start and the read-only `--list`. Fix: gate only `run_default`, and add a binary test that the hook exits 0 beside a 0644 file. migrate-cli has no `compose` call, so name where it prints its warning.
- 🟡 **Safety** (challenges pass-5 decision 2): tracker-mutating jira-cli and linear-cli commands act on team-only routing keys. Verified by the lens: today `jira-cli/src/resolve_fields.rs:172-187` gives `E_RESOLVE_NO_PROJECT` on an insecure file. With the screen, it returns the team `jira.project_key`, so `create`, `update` and `comment` can land in the team's project or Linear team. Meanwhile `work create`/`update`/`sync`, which reach the same trackers, refuse. **Needs a decision**: gate the tracker-mutating subcommands like work-cli (moving the file aside makes it `Absent`, and the environment overrides still work), or keep decision 2 and document the fallback.

#### Minor

- 🔵 **Safety**: `config templates eject --force` and `reset --confirm` resolve `paths.templates` on team-only values and can overwrite or delete a template. Add them to the gated writers.
- 🔵 **Compatibility**: `Composed`/`ConfigService`/`ConfigAccess` shape changes. `ScreenedStore` lacks `with_plugin_root` and the other launcher ports, the `ConfigService` constructor that carries the fact is unnamed, and `personal_file` has no default, which breaks 17 fakes. A careless fix could reopen the raw-store bypass. State the new shape.
- 🔵 **Compatibility**: `libc` is an unlisted new dependency. `signal-hook` is not in the lock; pin `0.4` with default features off to match gix, and confirm that `register_conditional_default` and `emulate_default_handler` exist in it.
- 🔵 **Correctness**: on the interrupt path the leader is reaped only after `SIGKILL`, so on Linux an interrupted run waits the full grace period. Apply the reap-on-exit rule whenever the leader's exit is observed.
- 🔵 **Correctness**: "one warning per root" breaks where a binary composes twice (jira-cli `search`: `context.rs:153` and `resolve_fields.rs:174`; visualiser). Use a once-guard in the `kernel` helper.

### Assessment

The pass-5 structure holds. Every lens confirms that the screen, the single
fact, the write guard and the installed-once handlers work against the real
code, and no lens proposes a redesign. The majors are narrower than pass 5's:
- two are gaps in coverage of the new gate (collaboration-cli in Phases
  1–2, and migrate's scope);
- one is an undisclosed exit move;
- one is a lifecycle hole in the guard, which an RAII guard closes;
- one reopens decision 2 on new evidence.

Each has a local, mechanical fix. Per the agreed stopping rule, apply these
and hand the remainder to implementation, with no seventh pass.

## Revisions After Pass 6 — 2026-09-27

Every pass-6 finding was applied. Per the agreed stopping rule, no
seventh pass follows, and anything that remains is left to
implementation's red steps.

1. **Tracker writes gated (pass-5 decision 2 reversed).** jira-cli and
   linear-cli `create`, `update`, `comment add`/`edit`/`delete`,
   `transition`, `attach` and `init` fail closed. So does jira-cli
   `resolve-fields` when it falls back to config for the project, and so do
   `config templates eject --force` and `reset --confirm`. Read-only
   commands proceed. The route out is to move `config.local.md` aside. The
   work item is amended to match.
2. **`ActiveRun` guard.** It clears the record on entry. On every exit path,
   including unwinding, it goes idle first, then swaps the record, then
   emulates the default. The interrupt path reaps the leader when it
   observes its exit. New tests cover the spawn failure, `TimedOut` and
   `OutputExceeded`, and an interrupt that completes within 300ms.
3. **collaboration-cli in Phase 1.** The team `github.token` is read only
   for `Absent`/`Readable`. For `Ignored` it refuses and exits 2. That exit
   move now sits in Phase 1.
4. **Linear 24→27** for a control-character token is recorded as the third
   Breaking move. A red-step binary test covers it, and
   `a_malformed_token_is_refused` is rewritten.
5. **migrate-cli** gates only `run_default`, and the hook and `--list` are
   pinned to exit 0.
6. **Minors:**
   - the `Composed { service, store, screened }` shape, with a pin that the
     launcher's `levels` and `content` use `screened`;
   - `ConfigService::with_personal_file`, with `new` and the concrete type
     names kept;
   - a default `ConfigAccess::personal_file`;
   - a process-wide once-guard for the warning, with tests on jira-cli
     `search` and the visualiser;
   - `libc` listed, and `signal-hook` pinned to 0.4 with default features
     off.

## Manual Approval — 2026-09-27T08:17:48+00:00

**Verdict:** APPROVE (manual)

Toby Clemson approved the plan manually after the pass-6 revisions, under
the stopping rule agreed after pass 5: a narrow sixth pass, its findings
applied, and no seventh pass. The pass-6 revisions were not re-reviewed.
Anything they leave is for implementation's red steps to surface. The plan
is marked `ready`.
