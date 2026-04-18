---
date: "2026-04-18T21:42:33Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, portability, usability]
review_pass: 2
status: complete
---

## Plan Review: Meta Visualiser — Phase 2: Server Bootstrap and Lifecycle

**Verdict:** REVISE

The plan is structurally sound and well-decomposed: the tri-precedence binary resolution is explicit, TDD discipline is carried through every sub-phase, the `mpsc`-centralised shutdown path converges three triggers into one deterministic exit, and atomic-rename discipline for lifecycle files is consistent. However, three critical issues and several high-confidence major findings cluster around the same pre-release security window (placeholder SHA-256 manifest + unconstrained config-override exec), the same concurrency class (PID reuse, concurrent launcher races, shutdown ordering), and repeated test-coverage gaps (fake-binary never exercises the end-to-end URL contract, dead-PID selection is probabilistic, key contract paths untested). The plan needs tightening on the pre-Phase-12 trust model, PID-identity verification, a handful of contract-level test gaps, and a few editorial cleanups (abandoned `option_env!` scaffolding, `test.integration` deletion without a migration shim) before implementation begins.

### Cross-Cutting Themes

Issues flagged by multiple lenses deserve the most attention.

- **Pre-release security window — placeholder SHA-256 manifest is a live footgun** (flagged by: security, correctness, safety). `bin/checksums.json` ships with `sha256:0…0` placeholders that aren't replaced until Phase 12. The launcher will happily verify and exec a binary whose hash *coincidentally* matches the placeholder (e.g., an attacker's file hashed to the sentinel, a downgrade that strips real hashes, a forked release). The sentinel is documented but the launcher itself never rejects it. This undermines the entire download-verify path from Phase 2 through Phase 12.

- **PID reuse / TOCTOU is systemic across four distinct call sites** (flagged by: architecture, correctness, security, safety). Both the launcher's reuse short-circuit (`kill -0 $EXISTING_PID`), `stop-server.sh`'s SIGTERM/SIGKILL escalation, and the Rust owner-PID watch (`nix::sys::signal::kill(pid, None)`) treat bare PID existence as proof of identity. On long-lived hosts and containers with high PID churn, a recycled PID can cause: stale URL hand-out, SIGKILL of unrelated processes, and owner-PID watchdog failing to trigger.

- **Concurrent launcher invocations are not serialised** (flagged by: architecture, safety). Nothing prevents two near-simultaneous `/accelerator:visualise` invocations from both observing "no server", both running the download path, both `nohup`-spawning, and both overwriting lifecycle files — leaving at least one orphan server with no tracked PID.

- **Shutdown cleanup ordering violates the "server-stopped.json is always present after clean shutdown" invariant** (flagged by: correctness, safety). `server-info.json` is removed *before* `server-stopped.json` is written; a write failure in the second step leaves neither file present. On forced-SIGKILL via `stop-server.sh`, no `server-stopped.json` is produced at all.

- **Test suite does not exercise what it claims to verify** (flagged by: test-coverage, correctness, safety). The fake-binary stub writes port 9 without binding — the launcher's "returns a live URL" contract is never proven end-to-end. Dead-PID selection probes `0xfffff` probabilistically (16-iteration walk). `config.json` assertions check cardinalities (9, 5) not key→value mappings. The `lifecycle_idle.rs` test's terminal `matches!` call lacks an `assert!` wrapper, making the assertion dead code.

- **`config.json` contract is duplicated across four artefacts with no single source of truth** (flagged by: architecture, test-coverage). Rust `Config` struct, bash `jq -n` writer, committed fixture, inline `shutdown.rs` JSON — any schema change requires coordinated edits in all four with no mechanical guard.

- **Editorial noise from abandoned design alternatives** (flagged by: architecture, code-quality, correctness). Phase 2.5 leaves the `option_env!` + `const_parse_u64` approach in the plan text alongside the adopted `Settings`-struct approach. The lib/bin split is buried as a parenthetical ("20-line refactor, not a new phase"). Implementers reading top-to-bottom may land the wrong pattern first.

### Tradeoff Analysis

- **Security vs. usability on `visualiser.binary`**: security flags the team-committed `.claude/accelerator.md` path as an unconstrained arbitrary-exec vector; usability flags the same key as undiscoverable. Tightening (restrict to `.local.md`, allowlist, prompt) closes the security gap but narrows the feature's usefulness for offline teams. Recommend: restrict team-committed path to `.local.md`, keep env var and `.local.md` as the two documented override layers, and surface both in the download-failed hint.
- **Atomic-rename safety vs. cross-filesystem portability**: safety and portability both flag that `tempfile::persist` can fail with EXDEV on container/overlay filesystems; the atomic-rename discipline only works when tmp and target share a device. Current plan creates the temp file in the target directory (good), but the launcher's download stages via `mktemp` (typically `/tmp`) then `mv`s cross-device. Recommend: stage downloads inside `$SKILL_ROOT/bin/` so both paths share a filesystem.

### Findings

#### Critical

- 🔴 **Security**: Placeholder checksum manifest is a negative oracle that makes the cache trivially tamperable until Phase 12
  **Location**: Phase 2.6 § Checksums manifest / Phase 2.7 § launch-server.sh step 7
  The launcher's SHA-256 gate compares against `sha256:0…0` from Phase 2 through Phase 12. Between those phases, a file matching the sentinel (or a manifest edit pointing at an attacker-controlled hash) passes verification trivially. The sentinel is documented but never enforced as a hard-fail.

- 🔴 **Security**: `.claude/accelerator.md` `visualiser.binary` is a committed arbitrary-exec vector with no visible prompt
  **Location**: Phase 2.7 § tri-precedence binary resolution (steps 6.1 and 6.2)
  `visualiser.binary` in the team-committed `.claude/accelerator.md` bypasses the SHA-256 check and is exec'd after only an `[ -x ]` probe. A single-line PR merged by a reviewer who doesn't realise this key is exec-valued achieves code execution on every teammate's machine.

- 🔴 **Safety**: Launcher interruption between cleanup, fork, and PID-file write leaves orphans
  **Location**: Phase 2.7 § launch-server.sh background-launch section
  The sequence `rm -f $INFO $STOPPED → nohup $BIN & → echo $! > $PID_FILE` is non-atomic. Interruption (Ctrl+C, SIGKILL, shell close, OOM) between `nohup` and the PID-file write leaves a running server with no tracked PID. `stop-server.sh` then reports `not_running` and the orphan is unreachable.

#### Major

Architecture (4):

- 🟡 **Architecture**: `config.json` contract is duplicated across four artefacts with no single source of truth
  **Location**: Phase 2.7 config.json writer · Phase 2.2 Config struct · Phase 2.4 shutdown.rs inline JSON · Phase 2.2 fixture
  Schema drift detection relies on tests happening to fail. Any later-phase field addition needs coordinated edits in four places with no automated verification.

- 🟡 **Architecture / Correctness / Security / Safety**: PID-reuse check is TOCTOU-vulnerable at four call sites; launcher has no lockfile against concurrent invocations
  **Location**: Phase 2.5 lifecycle owner_alive · Phase 2.7 reuse short-circuit · Phase 2.8 stop-server.sh · Phase 2.7 concurrent launchers
  No PID-identity cross-check (argv, comm, start-time) and no `flock` covering the launcher body. Stale URL hand-out, wrong-process SIGKILL, watchdog silent failure, and orphan-producing races all follow.

- 🟡 **Architecture**: `lifecycle` module depends on `server::ShutdownReason`, creating an awkward import direction
  **Location**: Phase 2.5 § lifecycle.rs importing crate::server::ShutdownReason
  `ShutdownReason` is a neutral enum used by both modules; hosting it inside `server` couples `lifecycle` to the HTTP surface and forces every future shutdown producer to cross-import. Should live in `src/shutdown.rs`.

- 🟡 **Architecture**: Lifecycle watch loop has no per-check timeout or independent scheduling
  **Location**: Phase 2.5 § Lifecycle-watch loop
  Single `tokio::spawn` doing owner-PID and idle checks sequentially on one ticker. Under macOS App Nap / VM suspension, the ticker can drift many minutes, delaying both shutdown triggers simultaneously.

Code Quality (4):

- 🟡 **Code Quality**: `launch-server.sh` becomes a ~180-line monolith mixing eight concerns with no decomposition
  **Location**: Phase 2.7 § full replacement
  Repo resolution, reuse detection, platform detection, tri-precedence, owner-PID, `jq -n` config write, nohup/disown, and poll-for-ready all in one linear block. Only `die_unsupported` is a named helper. `jq -n` construction with nine doc-path `--arg`s and five template triples is the most error-prone section.

- 🟡 **Code Quality / Correctness**: Abandoned `option_env!` + `const_parse_u64` scaffolding left in the plan alongside the adopted `Settings` approach
  **Location**: Phase 2.5 § Integration test: idle timeout fires
  Implementers reading the plan top-to-bottom will hit the rejected approach first. `const_parse_u64` also silently underflows on non-digit input.

- 🟡 **Code Quality**: `server::run` accumulates listener-bind, info-file write, signal spawn, lifecycle spawn, middleware, and graceful-shutdown with no extraction plan
  **Location**: Phase 2.3 → 2.5 § Server module growth
  `run` is >100 lines by end of Phase 2.5; shutdown closure has inlined filesystem side-effects; hard to unit-test any single concern. No refactor called out.

- 🟡 **Code Quality**: Fake-visualiser bash stub duplicated verbatim across two harnesses and again within `test-launch-server.sh` for the env-wins case
  **Location**: Phase 2.7 test-launch-server.sh · Phase 2.8 test-stop-server.sh
  Any `server-info.json` schema change requires three coordinated edits. No shared helper.

Test Coverage (8):

- 🟡 **Test Coverage**: Fake binary fabricates unreachable port 9 — launcher's "returns a live URL" contract never exercised end-to-end
  **Location**: Phase 2.7 § test-launch-server.sh fake-visualiser
  A regression where the launcher prints a plausible URL but the binary never listens would pass every assertion. Cargo tests cover real binding; bash harness stops short.

- 🟡 **Test Coverage / Correctness**: Dead-PID selection is probabilistic and documented to walk-and-hope
  **Location**: Phase 2.5 § reserved_pid_is_dead · Phase 2.8 § stale-PID test uses 999999
  On loaded CI hosts with raised `pid_max`, 16 walks can all land on live PIDs. Same fragility in `test-stop-server.sh` hardcoding 999999. Use a spawn-and-reap pattern for a deterministic dead PID.

- 🟡 **Test Coverage**: Idle-timeout test asserts the mpsc channel receives a message but does not prove the shutdown path runs end-to-end; terminal `matches!` lacks an `assert!` wrapper (dead assertion)
  **Location**: Phase 2.5 § lifecycle_idle.rs
  A regression that mis-routes `ShutdownReason` or fails to wire channel to disk is invisible. The assertion that should catch wrong-reason routing is a bare expression.

- 🟡 **Test Coverage**: Activity middleware has no test proving it actually updates the atomic when a request arrives
  **Location**: Phase 2.5 § activity.rs
  Unit test for `Activity::touch()` exists; axum middleware composition (layer order, State extraction) is untested. Silent middleware failure → idle-timeout firing under real traffic.

- 🟡 **Test Coverage / Security**: Checksum-mismatch refusal has no automated test; no automated sentinel-detection guard
  **Location**: Phase 2.7 § launch-server.sh
  The security-critical `[ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]` branch has only manual verification of the download-failure path. A `=` typo or sentinel-escape would slip through.

- 🟡 **Test Coverage**: `config.json` assertions only check cardinalities (9 doc_paths, 5 templates), not values
  **Location**: Phase 2.7 § test-launch-server.sh config.json shape checks
  Transposed mappings (`decisions` → `meta/plans`, `review_plans` ↔ `review_prs`) preserve cardinality. Every downstream phase's indexer depends on these mappings being correct.

- 🟡 **Test Coverage**: SIGTERM test asserts clean exit and file state but does not verify in-flight requests complete
  **Location**: Phase 2.4 § shutdown.rs
  The whole point of `with_graceful_shutdown` is untested. Phase 4's SSE will regress silently without this test.

- 🟡 **Test Coverage / Safety**: Stale-file test hardcodes PID 999999; reuse path never verifies PID identity
  **Location**: Phase 2.8 § test-stop-server.sh + launch-server.sh reuse probe
  A live process at PID 999999 inverts the test; a recycled PID is mistaken for a live visualiser.

Correctness (6):

- 🟡 **Correctness**: Lifecycle watcher task leaks after external shutdown
  **Location**: Phase 2.5 § lifecycle.rs spawn loop
  Loop only exits when *it* detects death/idle. If SIGTERM/SIGINT fires first, the task keeps ticking forever; mpsc channel never drops. Use a `CancellationToken` or select against a broadcast.

- 🟡 **Correctness / Safety**: Shutdown cleanup is not atomic — `server-info.json` can be removed while `server-stopped.json` write fails
  **Location**: Phase 2.4 § shutdown_signal future
  Reverse the order: write `server-stopped.json` first, then remove `server-info.json`. Preserves the post-shutdown invariant the spec commits to.

- 🟡 **Correctness**: Launcher races the server writing `server-info.json`; child PID may be reused before server writes its own info
  **Location**: Phase 2.7 § nohup/disown launch and PID file write
  Child crash before writing info leaves a stale `server.pid` with a potentially-recycled PID. Have the Rust server write `server.pid` after `server-info.json` lands.

- 🟡 **Correctness**: `OWNER_PID` resolution via `$PPID` grandparent is wrong when launcher is invoked from a subshell
  **Location**: Phase 2.7 § OWNER_PID resolution
  Test harness pushd/popd sub-shells make the grandparent a short-lived test process. Server auto-terminates within 60s of spawning. Accept `ACCELERATOR_VISUALISER_OWNER_PID` as explicit override.

- 🟡 **Correctness / Safety**: `find_repo_root` may succeed in test project dirs for the wrong reason (walks up past `$TMPDIR_BASE`)
  **Location**: Phase 2.7 § test-launch-server.sh
  On CI hosts where `$TMPDIR` is mounted inside another repo, tests can write `meta/tmp/visualiser/*` into the outer repo. Override `find_repo_root` in the harness or assert `$TMPDIR_BASE` is outside any VCS tree.

- 🟡 **Correctness / Safety**: `stop-server.sh` doesn't write `server-stopped.json` when SIGKILL escalation fires
  **Location**: Phase 2.8 § stop-server.sh
  Violates the documented invariant that `server-stopped.json` always exists after shutdown. Synthesise `{"reason":"forced-sigkill", ...}` from the script when escalation was used.

Security (5):

- 🟡 **Security**: Download staged via `mktemp` in world-writable `/tmp` with TOCTOU and symlink exposure
  **Location**: Phase 2.7 § step 7 curl download
  On shared hosts, the `mktemp → shasum → chmod +x → mv` window is race-exposed. Symlink at `$BIN_CACHE` redirects the `mv`. Stage in `$TMP_DIR`, use `install -m 0755`, bound redirects with `--max-redirs 3`, reject symlink cache.

- 🟡 **Security**: Shell-interpolated JSON error lines leak unquoted user-controllable values
  **Location**: Phase 2.7 § launch-server.sh error constructions
  `echo '{"error":"…","path":"'"$CONFIG_BIN"'"}'` is broken by `"`, newlines, or `$(…)` in the value. Worse, `$URL` from `server-info.json` is echoed as `**Visualiser URL**: $URL` without validation — control-char and prompt-injection vector. Use `jq -nc --arg` and regex-validate the URL.

- 🟡 **Security**: Lifecycle and config files written with default umask — no explicit `chmod 0600/0700`
  **Location**: Phase 2.7 § tmp/visualiser creation
  On macOS default umask, files are 0644 / dirs 0755. `server.log` and `config.json` become world-readable. Set `umask 077` at the top of the launcher; add `chmod 0700 "$TMP_DIR"`.

- 🟡 **Security**: `nohup` + stdout redirect sends untrusted process output into an unrotated log — DoS-by-disk-fill
  **Location**: Phase 2.7 § step 9 nohup
  Log grows unbounded; rotation deferred to Phase 10. Bound the `curl` with `--max-filesize 32M`, land a simple byte-count rotation now instead of waiting.

- 🟡 **Security**: No request-size/timeout/Host-header validation — DNS-rebinding surface for localhost-only listener
  **Location**: Phase 2.3 § axum handler
  Lock in `RequestBodyLimitLayer`, `TimeoutLayer`, and a 403-on-mismatched-Host middleware before Phase 4 adds SSE and Phase 8 adds writes.

Safety (5):

- 🟡 **Safety**: Deleting `tasks/test.py` breaks any external caller of `invoke test.integration` without a migration shim
  **Location**: Phase 2.9 § tasks/test.py removal
  Success criteria actively asserts `invoke test.integration` errors out. No grep inventory of current callers. Retain a deprecated shim task that prints a migration message and delegates.

- 🟡 **Safety**: In-tree mutation smoke tests regress Phase 1's tempdir-copy discipline
  **Location**: Phase 2.2–2.7 Manual Verification mutation steps
  "Rename `plugin_root` → `plugin_roots` in `config.rs`, observe fail, restore" corrupts the working tree if interrupted. Use `jj new` + `jj abandon` wrapping, or move mutations into a cargo integration test that operates on a tempdir copy.

- 🟡 **Safety**: Test cleanup does not reliably reap nohup'd fake-server children
  **Location**: Phase 2.7/2.8 § trap 'pkill -P $$ ...' EXIT
  `disown`'d fakes get reparented to init; `pkill -P $$` misses them. SIGKILL of the harness bypasses EXIT entirely. Track fake PIDs explicitly; run under `setsid` and kill the process group.

- 🟡 **Safety**: Atomic-rename assumption breaks if `<meta/tmp>` is on a different filesystem
  **Location**: Phase 2.3 / 2.4 write_server_info / write_server_stopped
  Container overlays, `TMPDIR` overrides, NFS shares cause `EXDEV`. Validate same-FS in the launcher; document the invariant near the Rust helper.

- 🟡 **Safety**: Subprocess integration tests with fixed 5-second timeouts — CI-flake and orphan-child risk
  **Location**: Phase 2.4/2.5 tests/shutdown.rs, lifecycle_idle.rs, lifecycle_owner.rs
  Under CI load, 5s is tight. Panics don't reap spawned children. Raise to 30s, wrap tests in `scopeguard`-style drop that explicitly `start_kill()` + waits.

Portability (3):

- 🟡 **Portability**: `shasum` is not universally available on Linux
  **Location**: Phase 2.7 § SHA-256 verification
  Missing from Alpine, distroless, minimal Debian. Add `sha256sum` fallback via a tiny shim function. Most Linux containers have `sha256sum`; most macOS have `shasum`.

- 🟡 **Portability**: `ps -o ppid= -p` is not portable to busybox
  **Location**: Phase 2.7 § owner-PID resolution
  Alpine-based devcontainers silently degrade: OWNER_PID falls back to `$PPID` (the immediate shell), so the server auto-exits within 60s of launch. Read `/proc/$PPID/status` on Linux as a busybox-safe fallback.

- 🟡 **Portability**: `curl` is a hard prerequisite with no `wget` fallback and no mirror escape hatch
  **Location**: Phase 2.7 § binary acquisition
  Debian slim / Fedora minimal may ship only `wget`. Add an `ACCELERATOR_VISUALISER_DOWNLOAD_URL` env-var override for mirrors/air-gaps, and detect curl-vs-wget.

Usability (4):

- 🟡 **Usability**: Download error hint omits the persistent `visualiser.binary` config key
  **Location**: Phase 2.7 § launch-server.sh error branches
  Users learn only about the one-shot env var. Mention both overrides in the `hint` field.

- 🟡 **Usability**: First-run download progress is invisible inside the slash-command flow
  **Location**: Phase 2 Overview (Gap 6) + Phase 2.7
  `Downloading visualiser server (first run, ~8 MB)…` goes to stderr. Claude Code's `!` preprocessor typically swallows stderr. First-time users see Claude sit silently for 3–10s. Emit on stdout as a separate line above the URL.

- 🟡 **Usability**: `visualiser.binary` config key is not discoverable without reading the plan
  **Location**: Phase 2.9 SKILL.md `Server lifecycle` block
  No user-facing surface documents the key — not SKILL.md, not `config-summary.sh`, not any template. Add a brief Overrides sub-block to SKILL.md naming both layers.

- 🟡 **Usability**: Owner-PID lifecycle wording doesn't match the CLI wrapper invocation mode
  **Location**: Phase 2.9 SKILL.md "It also stops on its own … when you exit Claude Code"
  Only correct for slash-command invocation. Rephrase to "when the process that launched it exits."

#### Minor

(Full text in per-lens sections below; grouped summary here.)

- 🔵 **Architecture**: `visualiser.binary` bypasses the `config-read-path.sh` extension point; Phase 2.5 documents a design iteration in-plan rather than the final design; activity middleware attached via `route_layer` has ambiguous scope; `checksums.json` `version` field coupled to plugin version creates dev-time churn; `test.integration.visualiser` bundles cargo+shell runners.
- 🔵 **Code Quality**: Bespoke assertion logic inlined instead of `assert_eq`; `reserved_pid_is_dead` probabilistic; bin/lib split called out as "20-line refactor"; `write_server_info` double-guard for impossible case; `unwrap_or(0)` timestamp silently masks clock errors; shutdown cleanup side-effects inlined in async block; tri-precedence double-checks env var; `AppState` wraps single field.
- 🔵 **Test Coverage**: `now_millis()` uses `SystemTime` (wall-clock); `handle.abort()` cleanup pattern problematic long-term; bare `[ -f ]` under `set -e` skips `test_summary`; no concurrent-shutdown-reasons test; no meta-test that every `test-*.sh` runs under exactly one component; serde tolerates unknown fields silently.
- 🔵 **Correctness**: Second SIGTERM/SIGINT silently dropped; fast-clock test doesn't prove 30-min arithmetic works; `const_parse_u64` accepts invalid input; `unwrap_or(Sigterm)` hides channel-closed bug; unsupported-platform test PATH override leaks; `cfg.host` parsed as `SocketAddr` rejects "localhost"; timestamp fallback to 0 ambiguous with real zero; `reserved_pid_is_dead` non-deterministic.
- 🔵 **Safety**: OWNER_PID fallback to `$PPID` under init-reparenting disables watchdog; downloaded binary cross-FS rename; Activity touched only on request arrival (SSE regression); manifest drift check documented but not implemented in launcher.
- 🔵 **Portability**: `jq` hard prerequisite; cross-FS `persist`; `disown` is bash-specific; stub `uname` silently matches unknown flags.
- 🔵 **Usability**: No status/inspect command; Stop command is long; inconsistent JSON envelope schema (launch `error` vs stop `status`); lost Phase 1 URL-line framing; silent reuse masks config-change ineffectiveness.

#### Suggestions

- 🔵 **Portability**: Validate `nix::sys::signal::kill` works on musl targets before Phase 12.
- 🔵 **Portability**: Add `ACCELERATOR_VISUALISER_RELEASES_URL` mirror/air-gap env var.
- 🔵 **Correctness**: `reserved_pid_is_dead` — expand probe budget or switch to spawn-and-reap.

### Strengths

- ✅ TDD discipline is carried through every sub-phase with red-then-green `jj` commits and mutation smoke tests where bundled.
- ✅ Atomic-rename pattern (`NamedTempFile::new_in(dir) + persist`) correctly sibling-scoped for `server-info.json`, `server-stopped.json`, `config.json`.
- ✅ Shutdown architecture converges three triggers (SIGTERM/SIGINT, owner-PID, idle) onto one `mpsc::Sender<ShutdownReason>` with a single deterministic shutdown path.
- ✅ Three-layer binary-resolution precedence (env > config > cached+hashed) is explicit and each layer's trust model is spelled out; the override bypass of SHA-256 is deliberate and called out.
- ✅ Clear module boundaries (`config`/`server`/`activity`/`lifecycle`) with dependency injection via the `Settings` struct for testability.
- ✅ Typed `ConfigError` with `thiserror`, source preserved, path attached; exit codes differentiate config-load (2) from runtime (1) failures.
- ✅ Level/component test-hierarchy split (`test.unit.<component>` / `test.integration.<component>`) avoids the unit-vs-integration ambiguity the flat `test.integration` task would have produced once cargo tests joined shell suites.
- ✅ Forward-fit scaffolding — `AppState { cfg }`, `build.rs` stub, minimal dep list — deliberately chosen to reduce later-phase migration cost.
- ✅ Platform detection normalises `uname` output to a small closed set, with an explicit unsupported-platform branch tested via PATH-shadowing.
- ✅ `owner_alive` correctly treats `EPERM` as "alive" (frequently missed when probing with signal 0).
- ✅ Placeholder SHA-256 values use a visually distinct sentinel (`sha256:0…0`) the release pipeline can grep for.

### Recommended Changes

Ordered by impact. Each references the finding(s) it addresses.

1. **Harden the pre-release trust model against placeholder-sentinel bypass** (addresses: Security critical #1, Safety downgrade, Correctness checksum-branch).
   In `launch-server.sh`, reject the download/cache path when `EXPECTED_SHA` equals the all-zeros sentinel — force users through `ACCELERATOR_VISUALISER_BIN` or `visualiser.binary` until Phase 12 publishes real hashes. Add a CI check that fails any build seeing the sentinel in a release manifest.

2. **Restrict `visualiser.binary` to `.claude/accelerator.local.md`** (addresses: Security critical #2, Usability discoverability).
   The key executes arbitrary binaries; hosting it in team-committed `.claude/accelerator.md` is a reviewed-PR RCE vector. Either refuse the key in `accelerator.md` (warning on detection), or require a first-use prompt for any unseen path. Update the Phase 2.7 test fixtures to model `.local.md` as the canonical location. Surface both override layers in the download-failed error `hint`.

3. **Rewrite the launcher background-launch sequence as a single atomic unit** (addresses: Safety critical #3, Correctness launcher/server race).
   Have the Rust server write `server.pid` itself alongside `server-info.json` via the existing atomic-rename helper. Let `stop-server.sh` fall back to reading `pid` from `server-info.json` when `server.pid` is absent. Take a `flock` around the entire launcher body to serialise concurrent invocations.

4. **Add PID-identity verification everywhere `kill -0` is consulted** (addresses: Architecture/Correctness/Security/Safety PID-reuse cluster).
   Record `(pid, start_time)` in `server-info.json`; validate both when probing liveness in the launcher's reuse path, `stop-server.sh`'s kill path, and the Rust owner-PID watch. On macOS use `ps -o lstart=`; on Linux use `/proc/<pid>/stat` field 22. Abort any action whose identity check fails with a clear error.

5. **Reverse the shutdown-cleanup order** (addresses: Correctness/Safety shutdown-atomicity cluster).
   Write `server-stopped.json` first, then remove `server-info.json`. Have `stop-server.sh` synthesise `server-stopped.json` when it has to escalate to SIGKILL (so the post-shutdown lifecycle invariant holds even on forced kills).

6. **Fill the test-coverage gaps that undermine the suite's claimed guarantees** (addresses: Test Coverage major cluster).
   - Make the fake-binary bind a real ephemeral port (`python3 -m http.server 0` or `nc -l`) and add `curl -fsS "$URL"` to the harness.
   - Replace probabilistic dead-PID selection with spawn-and-reap (mirror the existing `lifecycle_owner.rs` pattern).
   - Wrap `matches!` assertions in `assert!` (`lifecycle_idle.rs`).
   - Add explicit value-mapping assertions on `config.json` (`jq -r .doc_paths.decisions` == expected path) for all 14 keys.
   - Add an automated test for SHA-256 mismatch refusal using a local HTTP fixture server.
   - Add a test that sends a request mid-graceful-shutdown and asserts completion.

7. **Remove editorial noise** (addresses: Architecture/Code Quality/Correctness editorial cluster).
   - Delete the `option_env!` / `const_parse_u64` alternative from Phase 2.5; keep only the `Settings`-struct approach.
   - Promote the `lib.rs`/`main.rs` split to a first-class change in Phase 2.1 (or 2.2 when types first need exposing to tests) with its own success criteria.
   - Hoist `ShutdownReason` into `src/shutdown.rs`; both `server` and `lifecycle` depend on it, neither depends on the other.

8. **Decompose `launch-server.sh` into named functions** (addresses: Code Quality monolithic-launcher).
   Extract `resolve_project_root`, `detect_platform`, `resolve_binary` (with three sub-functions for env/config/cached), `write_config_json`, `start_server_background`, `wait_for_server_info`. Move `write_config_json` into a sibling `scripts/write-visualiser-config.sh` so `jq -n` construction is unit-testable. Add a shared `make_fake_visualiser()` helper in `scripts/test-helpers.sh` used by both bash harnesses.

9. **Provide a `test.integration` migration shim** (addresses: Safety CI-breakage).
   Retain a deprecated `test.integration` invoke task that prints a migration message naming the three component tasks and delegates to them. Keep for one release cycle, then remove. Grep the repo for `test.integration` callers in the same commit that performs the split.

10. **Harden cross-platform shell dependencies** (addresses: Portability cluster).
    Add a `sha256_of()` helper preferring `sha256sum` with `shasum -a 256` fallback. Replace `ps -o ppid= -p` with a busybox-compatible alternative (read `/proc/$PPID/status` on Linux). Add `curl`-vs-`wget` detection. Expose `ACCELERATOR_VISUALISER_DOWNLOAD_URL` for mirrors/air-gaps.

11. **Lock in a single-source-of-truth for the `config.json` contract** (addresses: Architecture schema-drift).
    Add a cargo test that reads the launcher's `jq -n` template output (or a committed canonical example written by the launcher in a dry-run mode) and deserialises through `Config`. A broken schema on either side then surfaces at `cargo test` time.

12. **Lock in default-deny HTTP baseline now** (addresses: Security request-limits).
    Add `RequestBodyLimitLayer::new(1_048_576)`, a `TimeoutLayer`, and a Host-header validation middleware to the Router in Phase 2.3. Phase 4's SSE and Phase 8's writes inherit the baseline.

13. **Improve user-visible feedback paths** (addresses: Usability minor cluster).
    Emit the "Downloading…" notice on stdout (not stderr) as a separate line before the URL so slash-command users see it. Add a `status` subverb or script to inspect state without touching it. Rephrase the SKILL.md lifecycle wording to be invocation-mode-agnostic. Keep a `**Visualiser**:` label in SKILL.md so error cases have framing.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally well-grounded: it preserves Phase 1's skill/preprocessor/server layering, introduces a clean `Config`-based contract between bash launcher and Rust binary, and funnels three distinct exit triggers (SIGTERM/SIGINT, owner-PID death, idle timeout) through one `mpsc` channel into a single shutdown path. Tradeoffs are mostly acknowledged. The main architectural risks are (a) the `config.json` contract is duplicated across four artefacts with no single source of truth, (b) the lifecycle watch and launcher reuse logic contain latent TOCTOU/single-thread concerns, and (c) the Phase 2.5 module split between `server` and `lifecycle` introduces a back-reference (`lifecycle` importing `server::ShutdownReason`) that will become awkward as both modules grow.

**Strengths**:
- Clear three-layer separation (skill markdown / bash preprocessor / Rust server) with stable interface boundaries.
- Shutdown architecture converges three divergent exit triggers onto a single `mpsc::Sender<ShutdownReason>` channel and one deterministic shutdown path.
- Binary-resolution tri-precedence is explicit; each layer's trust model is spelled out.
- Functional/imperative split is respected: pure data types with localised side effects.
- Forward-fit of `AppState { cfg }` minimal shape, `build.rs` stub, dep list pinning.
- Test-hierarchy restructuring to `<level>.<component>` avoids unit-vs-integration ambiguity.

**Findings** (major):
- `config.json` contract duplicated across four artefacts with no single source of truth.
- PID-reuse check is TOCTOU-vulnerable + no lockfile against concurrent launchers.
- `lifecycle` module depends on `server::ShutdownReason` — awkward import direction.
- Lifecycle watch loop has no per-check timeout or independent scheduling.

(Minor): `visualiser.binary` bypasses `config-read-path.sh` extension point; Phase 2.5 documents design iteration rather than final design; activity middleware via `route_layer` has ambiguous scope; `checksums.json` `version` field coupling creates dev-time churn; `test.integration.visualiser` bundles two runners.

### Code Quality

**Summary**: The plan is well-structured and thoughtful, with clear module boundaries, idiomatic Rust patterns (typed Config with thiserror, mpsc-driven shutdown, Arc<AppState>), and disciplined TDD. However, the launcher bash script concentrates eight concerns into a single ~180-line function; `server::run` is asked to grow into a multi-concern orchestrator across sub-phases without a planned refactor; and several code-quality smells appear (dead `option_env!` code, redundant fake-server duplication, bespoke inline assertions).

**Strengths**: Clear module boundaries; idiomatic error handling; atomic write pattern; in-plan self-correction on `Settings` vs `option_env!`; shared bash test helper; precedence rule documented once.

**Findings** (major):
- `launch-server.sh` becomes a ~180-line monolith mixing eight concerns.
- Abandoned `option_env!` scaffolding left alongside adopted `Settings` approach.
- `server::run` accumulates many concerns with no planned extraction.
- Fake-visualiser bash stub duplicated across two harnesses.

(Minor): bespoke assertions inlined instead of `assert_eq`; `reserved_pid_is_dead` probabilistic; bin/lib split hidden in parenthetical; `write_server_info` double-guard; `unwrap_or(0)` timestamp; shutdown side-effects inline; tri-precedence double-checks env var; `AppState` wraps single field.

### Test Coverage

**Summary**: The plan is well-structured around TDD with thoughtful red-then-green commits and multi-layered integration (cargo + bash + fake-binary). However several gaps undermine confidence: the launcher harness never proves a real HTTP 200 response, idle/owner tests use wall-clock with flaky assumptions, dead-PID selection is probabilistic, key contract paths (checksum mismatch, graceful draining, activity middleware feedback) have no tests, and config.json correctness is asserted only by cardinality.

**Strengths**: Strong TDD commitment; clear test-pyramid split; fast-clock `Settings` abstraction; dedicated config-precedence cases; level-composed mise tasks preserve failure determinism.

**Findings** (major): Fake binary writes unreachable port 9; dead-PID probabilistic selection; idle test dead `matches!` assertion + doesn't prove end-to-end shutdown; activity middleware has no real-request test; checksum-mismatch untested; config.json assertions only check cardinalities; SIGTERM test doesn't verify graceful draining; stale-file test hardcodes PID 999999.

(Minor): `now_millis` uses `SystemTime`; `handle.abort()` tear-down; bare `[ -f ]` under `set -e`; no concurrent-shutdown-reasons test; no meta-test for test-*.sh component coverage; serde tolerates unknown fields silently.

### Correctness

**Summary**: The plan lays out a thoughtful lifecycle design but contains several correctness defects: the lifecycle watcher leaks after external shutdown, owner-PID alive check is vulnerable to PID reuse, `write_server_stopped` is racy with `server-info.json` removal, and the fast-clock setting is only weakly representative. Shell-level race conditions (PID reuse, stale PID_FILE after stop, `$PPID` fallback in subshell-launched scripts) and error paths (find_repo_root in fake dirs, checksum-mismatch branch) also need attention.

**Strengths**: Atomic file writes correctly implemented; mpsc shutdown design; `ShutdownReason::StartupFailure` slot; config-precedence rules clear; `owner_alive` correctly treats EPERM as alive; Gap 2 schema locked early.

**Findings** (major): Lifecycle watcher leaks; PID-reuse race; shutdown cleanup non-atomic; launcher-server race on `server-info.json`; `OWNER_PID` wrong in subshell invocation; `find_repo_root` wrong in test dirs; checksum-mismatch cache leak / placeholder coincidence; `stop-server.sh` SIGKILL escalation no stopped record.

(Minor): Second signal silently dropped; fast-clock test doesn't prove 30-min arithmetic; `const_parse_u64` accepts invalid input; `unwrap_or(Sigterm)` hides channel-closed bug; unsupported-platform PATH override leaks; `cfg.host` rejects "localhost"; timestamp fallback to 0 ambiguous; `reserved_pid_is_dead` non-deterministic.

### Security

**Summary**: The Phase 2 plan introduces a binary-acquisition and background-process system with several sharp security edges: a checksum manifest committed with known-bad placeholder hashes that live in the repo from Phase 2 until Phase 12, two override layers that silently execute arbitrary binaries based on committed config, and a `curl | chmod +x | exec` flow whose TLS chain, TOCTOU behaviour, and symlink handling are under-specified. Most individual controls are reasonable in isolation, but the placeholder-manifest window and the `.claude/accelerator.md` → `visualiser.binary` → exec path are the serious issues.

**Strengths**: 127.0.0.1 bind only; SHA-256 gate + sentinel value; atomic rename for lifecycle files; no `reqwest` runtime dep; `owner_alive` treats EPERM as alive.

**Findings** (critical): Placeholder checksum manifest is a negative oracle; `.claude/accelerator.md` `visualiser.binary` is a committed RCE vector.

(Major): Download in `/tmp` has TOCTOU/symlink exposure; shell-interpolated JSON leaks user-controllable values; PID-reuse race; files written with default umask; `nohup` stdout to unrotated log; no request-size/timeout/Host-header validation.

(Minor): `host` field not validated against non-loopback; test harness models team-committed config path.

### Safety

**Summary**: Phase 2 introduces significant process-lifecycle complexity with several concurrency and failure-mode gaps that can leave orphaned processes, stale lifecycle files, or an unkillable state. Atomic-rename discipline is solid and tri-precedence is explicit; however, kill/PID handling has multiple TOCTOU hazards, concurrent launcher invocations are not serialised, and the test-task migration deletes `invoke test.integration` without any compatibility shim.

**Strengths**: Atomic writes consistent across three lifecycle files; single mpsc channel for all shutdown reasons; placeholder sentinel greppable; override bypass called out intentionally; reuse short-circuit cleans stale files.

**Findings** (critical): Launcher interruption between cleanup/fork/PID-file write leaves orphans.

(Major): PID-reuse dangerous for reuse-probe and SIGKILL; concurrent launchers race to spawn duplicates; `tasks/test.py` deletion breaks CI without shim; forced SIGKILL produces no stopped.json; in-tree mutation smoke tests regress Phase 1 discipline; test cleanup misses reparented children; atomic-rename breaks on cross-FS; disk-full during shutdown removes info + skips stopped; 5s subprocess timeouts flake; downgrade across manifest boundary loops.

(Minor): OWNER_PID fallback under init-reparenting; downloaded binary cross-FS rename; Activity timestamp only on arrival; manifest drift check documented not implemented.

### Portability

**Summary**: Phase 2 inherits accelerator's macOS+Linux scope cleanly and correctly gates platform, but assumes a rich coreutils+GNU toolset without gating or fallbacks. None of these are validated on minimal Linux distros (Alpine/busybox/distroless). The plan is also silent on GitHub Releases reachability (no mirror/offline option beyond `ACCELERATOR_VISUALISER_BIN`).

**Strengths**: Platform detection normalises `uname`; tri-precedence override supports air-gapped deployments; `Config` uses `PathBuf` not `String`; MSRV pinned at 1.80; cache path keyed on `<os>-<arch>`; Windows/WSL scope narrowed correctly.

**Findings** (major): `shasum` not universal on Linux; `ps -o ppid= -p` not portable to busybox; `curl` has no `wget` fallback and no mirror escape hatch.

(Minor): `jq` hard prerequisite without install-check; cross-FS `persist` on overlays; `disown` is bash-specific; stub `uname` test silently accepts unknown flags.

(Suggestions): Validate `nix` on musl; add `ACCELERATOR_VISUALISER_RELEASES_URL` for mirrors.

### Usability

**Summary**: The plan establishes a solid default path (three invocation modes, one-shot override env var, idle/owner-PID self-termination) and the happy-path stdout contract is crisp. The main usability gaps are around discoverability of the persistent `visualiser.binary` override, first-run download feedback vanishing into stderr, missing status/inspect affordances, and error hints that only document one of the two override mechanisms.

**Strengths**: Single bold-labelled stdout contract; structured JSON-ish errors with `hint`; three-layer override precedence correct; reuse semantics remove "am I running?" confusion; pre-resolved `Stop command`; SKILL.md separates Claude-only context from user-facing; fail-fast on bad `visualiser.binary`.

**Findings** (major): Download error hint omits `visualiser.binary`; first-run download invisible in slash-command flow; `visualiser.binary` not discoverable anywhere user-facing; owner-PID lifecycle wording wrong for CLI invocation.

(Minor): No status/inspect command; Stop command long and unmemorable; inconsistent `error` vs `status` JSON envelope; lost Phase 1 URL-line framing; silent reuse masks config-change ineffectiveness.

---
*Review generated by /review-plan*

---

## Re-Review (Pass 2) — 2026-04-18T21:42:33Z

**Verdict:** REVISE

The pass-1 edits materially improved the plan across every lens. Of the 3 criticals + ~45 majors raised in pass 1, the great majority are resolved — notably the PID-reuse cluster (via `(pid, start_time)` identity across all call sites), the concurrent-launcher race (via `flock`), the shutdown-ordering invariant for the happy path, atomic PID-file writes inside the Rust server, HTTP default-deny baseline, the editorial cleanup of `option_env!`/`lib.rs`-split/`ShutdownReason`, the tool-portability shims, the launcher decomposition, and the user-facing feedback paths (stdout download notice, `**Visualiser**:` label, invocation-mode-agnostic lifecycle wording, `status` subverb, overrides discoverability).

However, the edits introduce **one new critical** and **four new majors**, and several pass-1 items are only partially resolved. The verdict stays REVISE on the strength of the critical alone.

### Cross-Cutting New Concerns

- **macOS timezone drift between Rust and shell start-time computations** (Correctness critical). The Rust `process_start_time` on macOS parses `ps -o lstart=` as UTC wall-clock while the shell `start_time_of` uses BSD `date -j -f … +%s` which interprets local TZ. Every PID-identity cross-check on a non-UTC macOS host will compare values that differ by the TZ offset. The server self-terminates within 60s of launch and `stop-server.sh` refuses to kill its own server. This is a platform-wide blocker for macOS outside UTC.
- **Phase 2.4 `run` code block silently regresses Phase 2.3 invariants** (Architecture + Code Quality, flagged independently). The "Extend run" snippet is written as a full replacement and drops the loopback guard, middleware stack, `write_pid_file`, and `start_time` recording. Later sub-phases ("inside run()") assume the 2.3 shape is retained. An implementer following the plan literally regresses the entire hardening layer.
- **Phase 2.8 duplicates and contradicts Phase 2.7's reuse-detection block** (Architecture + Code Quality). The Phase 2.8 block is weaker (no `flock`, no `start_time` check, no URL regex) and the sub-section numbering is broken (two `#### 2.`, two `#### 3.`, non-linear ordering). Applying both yields a regressed reuse path; picking one requires guessing which is canonical.
- **Fake-visualiser heredoc still inlined in `test-stop-server.sh`** (Code Quality + Test Coverage, flagged independently). The shared `make_fake_visualiser` helper is declared in Testing Strategy and adopted in `test-launch-server.sh` but `test-stop-server.sh` retains the old heredoc with port 9 — the reachability coverage extension doesn't apply to the stop harness.
- **Stale-PID test still hardcodes `999999`** (Test Coverage). The `spawn_and_reap_pid` helper is declared for exactly this case but the `test-stop-server.sh` stale-file test wasn't migrated. Probabilistic flake survives.
- **Disk-full during shutdown still violates the post-shutdown invariant** (Safety). The shutdown path logs a warning on `write_server_stopped` failure but unconditionally proceeds to `remove_file(info)` + `remove_file(pid)` — yielding the `{no info, no stopped}` state the surrounding comment explicitly says it prevents.

### Previously Identified Issues

#### Critical

- 🔴 **Security**: Placeholder checksum manifest — **Dismissed** (user: no release until Phase 12; no window exists)
- 🔴 **Security**: `visualiser.binary` team-committed RCE — **Dismissed** (user: capability kept as deliberate trust decision; documented in "What We're NOT Doing")
- 🔴 **Safety**: Launcher interruption leaves orphans — **Resolved** (Rust server writes `server.pid` atomically; launcher no longer writes it; `flock` serialises)

#### Major

Architecture:
- 🟡 `config.json` contract duplicated across four artefacts — **Resolved** (`config_contract.rs` cargo test round-trips launcher output through `Config`)
- 🟡 PID-reuse TOCTOU + no launcher lockfile — **Resolved** (`flock` + `(pid, start_time)` identity check everywhere)
- 🟡 `lifecycle` depends on `server::ShutdownReason` — **Resolved** (hoisted to `src/shutdown.rs`)
- 🟡 Lifecycle watch has no per-check timeout — **Still present** (single-ticker + sequential checks; macOS `ps` fork could block idle check)

Code Quality:
- 🟡 `launch-server.sh` ~180-line monolith — **Resolved** (pipeline + `_launcher-helpers.sh` + `write-visualiser-config.sh`)
- 🟡 `option_env!` scaffolding — **Resolved** (deleted)
- 🟡 `server::run` accumulates many concerns — **Still present** (now ~50+ lines with 9 concerns; return type flips `Result<(), ServerError>` → `anyhow::Result<()>` mid-phase)
- 🟡 Fake-visualiser stub duplicated — **Partially resolved** (launcher harness adopted helper; stop harness did not — see New Issues)

Test Coverage:
- 🟡 Fake binary unreachable port 9 — **Resolved** for launcher harness; **Still present** for stop harness (see New Issues)
- 🟡 Dead-PID probabilistic walk — **Resolved** for lifecycle unit test; **Still present** in `test-stop-server.sh` stale-file case (see New Issues)
- 🟡 Idle test dead `matches!` — **Resolved** (`assert!` wrapped; codified in § Assertion discipline)
- 🟡 Activity middleware untested — **Resolved** (middleware integration test listed)
- 🟡 Checksum-mismatch untested — **Resolved** (local HTTP fixture + placeholder-sentinel refusal automated)
- 🟡 `config.json` cardinality-only — **Resolved** (all 14 keys value-asserted)
- 🟡 SIGTERM test doesn't verify graceful draining — **Resolved** (`graceful_draining.rs` with slow route)
- 🟡 Stale-file hardcoded PID 999999 — **Still present** (see New Issues)

Correctness:
- 🟡 Lifecycle watcher task leaks — **Still present** (no `CancellationToken`; runtime shutdown implicit only)
- 🟡 PID-reuse race — **Resolved** (with macOS TZ caveat — see Critical #1)
- 🟡 Shutdown cleanup non-atomic — **Partially resolved** (order reversed; disk-full case still breaks invariant — see New Issues)
- 🟡 Launcher races server writing info — **Resolved** (server writes its own PID)
- 🟡 `OWNER_PID` wrong in subshell — **Resolved** (init-reparent coerced to 0; watchdog guard skips)
- 🟡 `find_repo_root` wrong in test dirs — **Still present** (may walk past `$TMPDIR_BASE` into real workspace)
- 🟡 Checksum-mismatch cache leak — **Resolved** (placeholder sentinel refusal is a hard error)
- 🟡 `stop-server.sh` SIGKILL no stopped record — **Resolved** (synthesises `{reason:"forced-sigkill"}`)

Security:
- 🟡 `/tmp` TOCTOU download — **Resolved** (staged in `$SKILL_ROOT/bin` with `install -m 0755` + symlink refusal)
- 🟡 Shell-interpolated JSON leaks — **Resolved** (all errors via `jq --arg`; `$URL` regex-validated)
- 🟡 PID-reuse race — **Resolved**
- 🟡 Default-umask lifecycle files — **Resolved** (`umask 077` + 0o600 perms)
- 🟡 `nohup` uncapped log — **Partially resolved** (`curl --max-filesize` added; log rotation still deferred to Phase 10)
- 🟡 No request-size/timeout/Host-header validation — **Resolved** (default-deny middleware stack)

Safety:
- 🟡 PID-reuse makes kill dangerous — **Resolved**
- 🟡 Concurrent launchers race — **Resolved** (`flock`)
- 🟡 Deleting `tasks/test.py` breaks CI — **Dismissed** (user: shim not wanted)
- 🟡 Forced SIGKILL no stopped.json — **Resolved** (synthesised)
- 🟡 In-tree mutation smoke tests — **Still present** (Phase 2.2/2.3 Manual Verification still mutates live source)
- 🟡 Test reap by parent misses reparented fakes — **Partially resolved** (launcher harness switched to PID-walk; stop harness still `pkill -P $$`)
- 🟡 Atomic-rename EXDEV — **Resolved** (download same-FS; Rust writes `new_in(dir)`)
- 🟡 Disk-full shutdown — **Still present** (see New Issues)
- 🟡 5s subprocess timeouts flake — **Resolved** (raised to 30s)
- 🟡 Manifest downgrade loop — **Resolved** (version drift check + sentinel refusal)

Portability:
- 🟡 `shasum` not universal — **Resolved** (`sha256_of` shim)
- 🟡 `ps -o ppid=` not busybox-safe — **Resolved** (`/proc/$pid/status` fallback)
- 🟡 `curl` only — **Resolved** (`download_to` with wget fallback; mirror URL env var)

Usability:
- 🟡 Error hint omits `visualiser.binary` — **Resolved** (both layers named in `hint`)
- 🟡 First-run download invisible — **Resolved** (notice on stdout)
- 🟡 `visualiser.binary` undiscoverable — **Resolved** (SKILL.md Overrides sub-block)
- 🟡 Owner-PID wording wrong for CLI — **Resolved** (invocation-mode-agnostic)

### New Issues Introduced

#### Critical

- 🔴 **Correctness**: macOS timezone mismatch between Rust `process_start_time` and shell `start_time_of`
  **Location**: Phase 2.3 `process_start_time` (Rust) · Phase 2.7 `start_time_of` (shell helper)
  The Rust macOS parse treats `ps -o lstart=` as UTC; the shell uses `date -j -f` which interprets local TZ. Every identity cross-check fails on non-UTC macOS hosts — server self-terminates within 60s of launch; `stop-server.sh` refuses to kill its own server. Fix: both sides must use the same conversion. Simplest — have Rust shell out to `date -j -f "%a %b %d %H:%M:%S %Y" "$s" +%s` matching the shell, or switch both to `sysctl kern.proc.pid.<pid>` which returns UTC epoch directly. Add a cross-process round-trip test asserting Rust's recorded `owner_start_time` equals the shell's subsequent probe on the same PID.

#### Major

- 🟡 **Architecture + Code Quality**: Phase 2.4 `run` snippet silently regresses Phase 2.3 invariants
  **Location**: Phase 2.4 § 2 (shutdown machinery `run` body) vs Phase 2.3 § 1 (server module `run` body)
  Phase 2.4's "Extend run" code block is a full replacement that omits the loopback guard, middleware stack, `write_pid_file`, and `start_time` recording. Return type flips from `Result<(), ServerError>` to `anyhow::Result<()>`. Later sub-phases assume Phase 2.3's shape is retained. Fix: present Phase 2.4 as an additive diff (mpsc channel + signal handlers + `with_graceful_shutdown` call only), or embed the complete post-2.4 `run` body with every 2.3 invariant retained. Pick one error type (prefer `ServerError`) and keep it throughout 2.4/2.5.

- 🟡 **Architecture + Code Quality**: Phase 2.8 reuse-detection block duplicates and contradicts Phase 2.7
  **Location**: Phase 2.8 § 2 "Extend launch-server.sh with reuse detection" vs Phase 2.7 launcher body
  Phase 2.7 already has a complete reuse short-circuit with `flock`, `(pid, start_time)` identity check, and URL regex validation. Phase 2.8 then adds a second simpler block lacking all three. Sub-section numbering is broken (two `#### 2.`, two `#### 3.`, non-linear ordering). Fix: delete the Phase 2.8 reuse-detection sub-section entirely; the 2.7 block is canonical. Renumber remaining 2.8 sub-sections linearly.

- 🟡 **Code Quality + Test Coverage**: Fake-visualiser still inlined in `test-stop-server.sh` with port 9
  **Location**: Phase 2.8 § 3 `test-stop-server.sh`
  Testing Strategy declares `make_fake_visualiser` specifically to replace the heredoc, and `test-launch-server.sh` adopts it. `test-stop-server.sh` still has the full `cat > "$FAKE_BIN" <<'EOF' … EOF` block with hard-coded `"port":9,"url":"http://127.0.0.1:9"`. The reuse/stop tests therefore don't exercise real-port reachability, and any schema tweak to the fake requires edits in two places. Fix: replace the heredoc in `test-stop-server.sh` with `make_fake_visualiser "$FAKE_BIN"` and add `curl -fsS "$URL"` assertions after each launch.

- 🟡 **Test Coverage**: Stale-file test still hardcodes PID 999999
  **Location**: Phase 2.8 § 3 `test-stop-server.sh` stale-file case
  `spawn_and_reap_pid` helper is declared exactly to eliminate this, and § Integration Tests (bash) explicitly promises "using `spawn_and_reap_pid` … rather than `999999`". The script body wasn't updated. Fix: `STALE_PID="$(spawn_and_reap_pid)"` and `"pid":$STALE_PID` in the synthesised `server-info.json`. Add an assertion that the post-launch `server.pid` is not equal to `$STALE_PID`.

- 🟡 **Safety**: Post-shutdown invariant still breaks under disk-full
  **Location**: Phase 2.4 `shutdown_signal` closure in `server::run`
  `write_server_stopped` failure logs a warning, then the code unconditionally removes `server-info.json` and `server.pid`. The surrounding comment claims to prevent a `{no info, no stopped}` window but the implementation doesn't. Fix: skip the `remove_file` calls when `write_server_stopped` errors — leaving stale info is strictly safer than no audit trail. Add a test that makes the stopped-file write fail (pre-create a read-only directory at `stopped_path`) and asserts info.json remains.

### Additional Observations

These are minor and not all require action in Phase 2 — noted for visibility:

- **Correctness**: Lifecycle watcher task still has no external cancellation signal; relies on runtime shutdown. `rx.recv().await.unwrap_or(Sigterm)` still hides channel-closed state as a legitimate SIGTERM. Second signal bypasses graceful drain. Fast-clock test doesn't cover 30-minute-scale arithmetic. `find_repo_root` may walk past `$TMPDIR_BASE` into the real workspace. Shell `start_time_of` Linux path can fail on PIDs whose comm contains `)`.
- **Architecture**: `lifecycle` still depends on `server::process_start_time` (narrower than the original ShutdownReason coupling but still cross-module). Activity middleware still attached via `route_layer` with parallel state. Helpers defined in 2.7 then moved in 2.8 — no commit should land in the intermediate state.
- **Code Quality**: Three atomic writers repeat the same prologue pattern; could factor to one helper. `now_millis()` duplicated between `activity` and `lifecycle`; `.unwrap_or(0)` fallback still present in both.
- **Test Coverage**: `now_millis()` wall-clock vulnerability not addressed; `handle.abort()` teardown pattern still in Phase 2.3 test; no concurrent-shutdown-reasons test; no meta-test that every `test-*.sh` is claimed by exactly one component.
- **Security**: `wget` fallback lacks TLS floor (`--https-only`) and size cap (`--quota`) that `curl` enforces; `ACCELERATOR_VISUALISER_RELEASES_URL` accepts any scheme (pre-check could refuse non-`https://`); `host_header_guard` accepts empty Host; `server.log` remains uncapped and unrotated (explicit Phase 10 deferral, but worth a note in the SKILL.md lifecycle block that heavy use can fill disk).
- **Safety**: Phase 2.2/2.3 Manual Verification still asks for in-tree mutation smoke tests; `test-stop-server.sh` EXIT trap still uses `pkill -P $$` which won't catch disowned fakes; no EXDEV fallback in `write_server_stopped`/`write_server_info` for exotic filesystems.
- **Portability**: `disown` is bash-specific and the launcher shebang is `#!/usr/bin/env bash` — no bash presence check alongside the jq one. Stub `uname` fail-on-unknown-flag is prose-only, not codified. musl validation of `nix::sys::signal::kill` still Phase 12. `python3` / `nc -l 0` fallback chain in `make_fake_visualiser` is fragile on minimal images.
- **Usability**: Stop/Status commands remain long absolute paths (no CLI subverb); `error` vs `status` JSON envelope inconsistency still present; silent reuse masks config-change ineffectiveness; `stop-server.sh status` vocabulary (`running`/`stale`/`not_running`) doesn't match `stop`'s (`stopped`) — four state words across two related commands with no mapping; Overrides YAML snippet is frontmatter-fenced but doesn't explain placement in `.claude/accelerator.md`; error `hint` embeds literal `\n` that renders as backslash-n not a newline.

### Assessment

The plan is meaningfully tighter than it was. The new Correctness critical is a real blocker — macOS-in-non-UTC is most contributors' default state, and the identity check is a load-bearing defence. The Architecture/Code Quality majors are plan-level editorial inconsistencies rather than design flaws: they're fast to fix (present Phase 2.4 `run` as a diff, delete the Phase 2.8 reuse block, renumber 2.8 sub-sections, migrate `test-stop-server.sh` to the shared helpers). The remaining Safety issue (disk-full shutdown) is a ~3-line fix in the shutdown closure.

Recommend one more focused pass addressing the critical + the five new majors, then the plan is ready for implementation. The minor residuals from pass 1 can be handled opportunistically during implementation or deferred with explicit "What We're NOT Doing" entries.

---
*Re-review generated by /review-plan*
