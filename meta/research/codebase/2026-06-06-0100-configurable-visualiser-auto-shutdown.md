---
type: codebase-research
id: "2026-06-06-0100-configurable-visualiser-auto-shutdown"
title: "Research: Configurable Visualiser Auto-Shutdown"
date: "2026-06-06T13:06:49+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0100"
parent: "work-item:0100"
topic: "Configurable Visualiser Auto-Shutdown"
tags: [research, codebase, visualiser, server, lifecycle, configuration, idle-timeout]
revision: "719910e7852565ad829715320048523384599d53"
repository: "accelerator"
last_updated: "2026-06-06T13:06:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Configurable Visualiser Auto-Shutdown

**Date**: 2026-06-06T13:06:49+00:00
**Author**: Toby Clemson
**Git Commit**: 719910e7852565ad829715320048523384599d53
**Branch**: HEAD (detached; work in progress)
**Repository**: accelerator

## Research Question

For work item 0100 (`meta/work/0100-configurable-visualiser-auto-shutdown.md`):
how does the visualiser server's idle auto-shutdown work today, how is the
existing `visualiser:` config (`visualiser.binary` / `ACCELERATOR_VISUALISER_BIN`)
plumbed through to the server, what testing seams exist for verifying timeout
boundaries without a real-time wait, and what would adding a configurable
`visualiser.idle_timeout` (default 30m → 8h, with `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`
override and a disable token) actually touch?

## Summary

The pieces the work item assumes all exist and are exactly where it says:

- **The idle tracker** is `Activity` — a single `AtomicI64` of last-request
  epoch-millis, touched by an axum middleware on every request
  (`server/src/activity.rs`).
- **The 30-minute timeout is a hard-coded compile-time `const`** in
  `lifecycle.rs` (`Settings::DEFAULT.idle_limit_ms = 30 * 60 * 1000`), with **no
  runtime/config/env override path today**. This is the single value the work
  item raises to 8h and makes configurable.
- **The verification seam already exists.** `lifecycle::spawn` takes a
  `Settings { tick, idle_limit_ms }` struct; production passes `Settings::DEFAULT`
  and the existing integration test `tests/lifecycle_idle.rs` passes
  `{ tick: 50ms, idle_limit_ms: 200 }`. No `Clock` trait is needed — tests shrink
  the *threshold*, not the clock.
- **Config does NOT reach the server the way `visualiser.binary` does.**
  `visualiser.binary` is consumed entirely launcher-side (it selects which
  executable to exec) and never reaches the Rust process. Every value the server
  actually consumes travels through a generated `config.json` (`--config` CLI
  arg) deserialised by `serde` with `#[serde(deny_unknown_fields)]`. The right
  precedent to copy is **`visualiser.kanban_columns`**, not `visualiser.binary`.

So implementing 0100 is a four-stage plumbing change (shell read → `config.json`
field → `serde` struct field → `lifecycle::Settings`), plus a `humantime`-style
duration parser, plus a fail-fast validation at boot, plus doc updates in five
places. There is one important nuance the work item's precedence rule
(`env > config > default`) collides with: the env var must be read by the shell
launcher (the server gets no env), so the env-vs-config resolution happens in
shell, while duration *parsing/validation* and the disable-token semantics
belong in Rust at boot. The natural split is **shell resolves precedence and
passes one raw string; Rust parses, validates, and fails fast**.

> **Important caveat — canonical paths.** All server source lives under
> `skills/visualisation/visualise/server/`. The `workspaces/*/skills/...` copies
> are jj workspace checkouts (per repo convention) and are ignored throughout.

## Detailed Findings

### 1. The idle tracker — `activity.rs`

`skills/visualisation/visualise/server/src/activity.rs`

- `pub struct Activity(AtomicI64)` (line 10) — a newtype over a single
  `AtomicI64` holding last-activity Unix epoch-millis, initialised to "now" at
  construction (`AtomicI64::new(now_millis())`, line 20).
- `touch()` (lines 22-24) stores `now_millis()` with `Ordering::Relaxed`;
  `last_millis()` (lines 25-27) reads symmetrically. Plain last-writer-wins, no
  CAS — concurrent requests harmlessly race to write near-identical timestamps.
- `middleware()` (lines 37-44) calls `state.touch()` before forwarding every
  request, so the timestamp updates on **every inbound HTTP request regardless of
  route or outcome**. Wired as a global axum layer at `server.rs:226-229`.
- `Activity` itself never decides "idle" — it only stores/reads the timestamp.
  The idle decision lives in the lifecycle loop (below). Doc comment (lines 1-3)
  confirms: "Middleware updates the atomic on every request; no request-path
  changes needed."

This is the work item's "natural verification seam": the idle comparison reads
this atomic, so injecting a short `idle_limit_ms` plus calling `touch()` lets
boundary criteria be tested without a real wait.

### 2. Where the 30-minute timeout is hard-coded

`skills/visualisation/visualise/server/src/lifecycle.rs:23-26`

```rust
pub const DEFAULT: Settings = Settings {
    tick: Duration::from_secs(60),
    idle_limit_ms: 30 * 60 * 1000,   // line 25 — the value 0100 changes
};
```

- The exact value: `idle_limit_ms: 30 * 60 * 1000` (= 1,800,000 ms = 30 min) at
  `lifecycle.rs:25`. Tick interval (60s) at line 24.
- This `const` is the **only** place the production value is set, and it is the
  only thing passed into the loop (`server.rs:321`). There is **no config or env
  override path** — it is genuinely compile-time hard-coded.
- For 0100: this becomes `8 * 60 * 60 * 1000` as the built-in default, and the
  value must become overridable from `config.json` (see §4).

### 3. The idle loop, shutdown channel, and the other two triggers

The timer loop — `lifecycle::spawn` (`lifecycle.rs:29-52`):

```rust
let idle = now_millis() - activity.last_millis();
if idle >= settings.idle_limit_ms {
    let _ = tx.send(ShutdownReason::IdleTimeout).await;
    return;
}
```

- A detached `tokio::spawn` task driven by `tokio::time::interval(settings.tick)`.
  The first immediate tick is **discarded** (line 38), so the earliest possible
  fire is after `2 * tick`. The comparison is `>=`, so the boundary value is
  inclusive.
- **Granularity note:** because the check only runs on the 60s tick, real idle
  shutdown fires between 30:00 and 31:00, not precisely at 30:00. This matters for
  the work item's "±5s tolerance" criteria — at production tick (60s) the
  resolution is coarse; tests use a small tick (50ms) to get fine resolution.

All three shutdown triggers converge on one `mpsc::channel::<ShutdownReason>(4)`
(`server.rs:315`) feeding a single graceful-shutdown future:

1. **Idle timeout** → `ShutdownReason::IdleTimeout` from the loop
   (`lifecycle.rs:47`).
2. **Launching-process (owner-PID) exit** → `ShutdownReason::OwnerPidExited`
   (`lifecycle.rs:41-44`), via `owner_alive(pid, start_time)` (lines 60-80) which
   probes with `kill(pid, None)` (signal 0) and cross-checks process start-time to
   defend against PID reuse. Disabled entirely when `owner_pid == 0` (orphaned).
3. **Explicit `stop`** → there is **no in-process HTTP stop route**. Stop is
   launcher-side: `stop-server.sh` → `stop_server_stop` in `launcher-helpers.sh`
   (lines 135-192) sends SIGTERM (escalating to SIGKILL after 2s), received by
   `spawn_signal_handlers` (`server.rs:395-410`) → `ShutdownReason::Sigterm`.

The receiver (`server.rs:358-387`) writes `server-stopped.json` (atomic, mode
0600, recording the kebab-case reason + timestamp), removes `server-info.json`
and `server.pid` on success, then `axum::serve(...).with_graceful_shutdown(...)`
returns and the process exits `ExitCode::SUCCESS`. `ShutdownReason` is defined in
`shutdown.rs:8-17` as deliberately shared plumbing so neither `server` nor
`lifecycle` depends on the other.

**Implication for 0100's disable token (`"never"`/`0`):** disabling idle
shutdown means the lifecycle loop must simply *not* fire `IdleTimeout` — triggers
2 and 3 are untouched and continue to work, exactly as the work item requires.
The cleanest representation is a sentinel (e.g. `idle_limit_ms` absent / `i64::MAX`);
note `tests/lifecycle_owner.rs:26` already uses `idle_limit_ms: i64::MAX` to
disable idle so the owner trigger can be tested in isolation — a ready-made
precedent for "disabled" semantics.

### 4. How config reaches the server — and why `binary` is the wrong template

There are **two different classes** of `visualiser.*` key, and `idle_timeout`
must follow the second:

**Class A — `visualiser.binary` (launcher-only, NOT a template to copy).**
Resolved entirely in `launch-server.sh:100-164` (the "tri-precedence binary
resolution"). It selects which executable to exec (`nohup "$BIN" --config "$CFG"`,
line 196) and **never enters `config.json`** — the Rust server is unaware of it.
The precedence block (lines 102-118) is a strict if/else:

```sh
if [ -n "${ACCELERATOR_VISUALISER_BIN:-}" ]; then
  BIN="$ACCELERATOR_VISUALISER_BIN"                 # env wins, short-circuits
else
  CONFIG_BIN="$(.../config-read-value.sh visualiser.binary ...)"   # config second
  ...
fi
# else: checksum-verified cached/downloaded binary (default)
```

**Class B — server-consumed keys (e.g. `visualiser.kanban_columns`) — the
correct template.** Every value the Rust server consumes is funnelled through one
generated `config.json`:

- `.accelerator/config.md` / `config.local.md` are parsed by the **generic** shell
  reader: `scripts/config-common.sh` (`config_find_files` lines 26-48, team file
  first then local; `config_extract_frontmatter` lines 73-85) and
  `scripts/config-read-value.sh` (nested `section.subkey` awk parse, lines 60-91).
- `launch-server.sh:179-189` assembles CLI args and calls
  `write-visualiser-config.sh ... > "$CFG"` (where `CFG="$TMP_DIR/config.json"`).
- `write-visualiser-config.sh` reads further keys (e.g. `visualiser.kanban_columns`
  at line 151 via `config-read-value.sh`) and emits the final JSON via `jq -n`
  (lines 181-234).
- The server receives **only** `--config <path>` (`main.rs:9-13`, clap `Cli`),
  loaded by `Config::from_path` (`config.rs:257-268`) via `serde_json::from_slice`
  into the `Config` struct (lines 13-38).
- **Critical constraint:** `Config` has `#[serde(deny_unknown_fields)]`
  (`config.rs:14`). Any new JSON key MUST have a matching struct field or the
  server fails to boot. `kanban_columns` (emitted at `write-visualiser-config.sh`,
  deserialised at `config.rs:36-37`, resolved at `config.rs:291-303`) is the exact
  precedent.

**No env vars are forwarded to the server process** — `ACCELERATOR_VISUALISER_BIN`
is consumed by the launcher itself. So `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` must
likewise be read shell-side and threaded into `config.json` (or a new CLI arg).

### 5. The four-stage change to add `visualiser.idle_timeout`

Mirroring `kanban_columns` (for the server-consumed routing) plus `binary`'s
env-override shape (for precedence):

1. **Resolve precedence in shell** — in `launch-server.sh` (or
   `write-visualiser-config.sh`), apply `${ACCELERATOR_VISUALISER_IDLE_TIMEOUT:-}`
   > `config-read-value.sh visualiser.idle_timeout` > built-in default, using the
   env-precedence if/else idiom from `launch-server.sh:103-118`. Pass the resolved
   **raw string** onward (do not parse in shell).
2. **Emit a JSON field** in the `jq -n` object at `write-visualiser-config.sh`
   (~lines 205-234), e.g. `idle_timeout: "8h"`.
3. **Add a `serde` field** on `Config` (`config.rs:15-38`) — required because of
   `deny_unknown_fields`. A `#[serde(default)]` keeps older configs valid.
4. **Thread into `lifecycle::Settings`** at `server.rs:317-323`, parsing the
   string into `idle_limit_ms` (or the disable sentinel) and replacing
   `Settings::DEFAULT`.

**Where parsing/validation belongs:** in Rust at boot (step 3/4), so an
unparseable value fails fast with a clear error before the server starts (work
item AC8). The `humantime` crate (`humantime::parse_duration` for `"8h"`,
`"30m"`, `"1h30m"`) is not yet a dependency — it would be added to
`server/Cargo.toml` `[dependencies]` (and `Cargo.lock`). Disable-token handling
(`"never"`/`0`, case-insensitive on the textual token only) is a small pre-check
before `humantime` parsing.

**Design tension to resolve in planning:** the work item frames precedence as
`env > config > default` and fail-fast on invalid values. Since the server never
sees the env var, precedence is resolved in shell but validation happens in Rust.
Two viable shapes:
- (a) Shell resolves the single winning string and passes it; Rust parses +
  validates + fails fast. (Simplest; matches `kanban_columns` routing.)
- (b) Shell validates too (another `humantime` equivalent in shell — there isn't
  one), duplicating logic. Not recommended.
Option (a) is the natural fit and keeps a single source of truth for parsing.

### 6. Testing seams and how to verify the boundaries

`skills/visualisation/visualise/server/tests/lifecycle_idle.rs` is the template
(`idle_timeout_fires_with_fast_clock`, 28 lines):

```rust
lifecycle::spawn(activity, /*owner_pid*/ 0, None,
    Settings { tick: Duration::from_millis(50), idle_limit_ms: 200 }, tx);
// assert ShutdownReason::IdleTimeout arrives within tokio::time::timeout(3s, rx.recv())
```

- **No `Clock` trait, no `tokio::time::pause()`.** Both `now_millis()` helpers
  (`activity.rs:30-35`, `lifecycle.rs:82-87`) call `SystemTime::now()` directly
  and are **duplicated** (not shared). `tokio::time::pause()` would NOT freeze
  them (it only affects `tokio::time` primitives). Tests use **small real
  durations** bounded by a 3s `tokio::time::timeout` so a hang fails fast.
- **Boundary tests** (just-under vs just-over): call `activity.touch()` between
  sleeps to control the measured idle delta against a small `idle_limit_ms`.
  Comparison is `idle >= idle_limit_ms` (inclusive). For the work item's
  "still alive at D−tolerance, dead at D+tolerance" assertions, drive `touch()`
  and assert the channel is empty before the boundary.
- **Disable / `0` / compound** criteria: parse-level unit tests (string → ms or
  sentinel) belong inline in `src/` (run under `--lib`, `test:unit:visualiser`);
  loop-firing behaviour belongs in `tests/lifecycle_idle.rs` (run under `--tests`,
  `test:integration:visualiser`). `tests/lifecycle_owner.rs:26` shows the
  `idle_limit_ms: i64::MAX` "disabled" pattern.
- **Fail-fast** criterion: a CLI/boot test in the style of
  `tests/config_cli.rs` / `tests/config_contract.rs` (assert non-zero exit + clear
  stderr on a bad `idle_timeout`).

Test wiring:
- `test:unit:visualiser` → `cargo test --manifest-path .../Cargo.toml --lib`
  (`tasks/test/unit.py:19-24`, `mise.toml:76-79`).
- `test:integration:visualiser` → `cargo test ... --tests` + shell suites
  (`tasks/test/integration.py:19-24`, `mise.toml:104-107`).
- CI: `.github/workflows/main.yml:31` (unit) and `:49` (integration).
- Shell-level launcher tests already exist:
  `scripts/test-launch-server.sh`, `scripts/test-write-visualiser-config.sh` —
  the place to cover env-over-config precedence and the disable-token pass-through.

## Code References

- `skills/visualisation/visualise/server/src/activity.rs:10-44` — `Activity`
  (`AtomicI64`), `touch()`/`last_millis()`, request middleware (the seam).
- `skills/visualisation/visualise/server/src/lifecycle.rs:12-27` — `Settings`
  struct + `Settings::DEFAULT` (30-min hard-code at line 25).
- `skills/visualisation/visualise/server/src/lifecycle.rs:29-52` — idle/owner
  timer loop; `idle >= idle_limit_ms` fires `IdleTimeout` (line 47).
- `skills/visualisation/visualise/server/src/lifecycle.rs:60-87` — `owner_alive`
  + duplicated `now_millis()` (`SystemTime`-based, no injection seam).
- `skills/visualisation/visualise/server/src/shutdown.rs:8-17` —
  `ShutdownReason` enum (`IdleTimeout`, `OwnerPidExited`, `Sigterm`, …).
- `skills/visualisation/visualise/server/src/server.rs:315-323` — mpsc channel +
  production wiring of `Settings::DEFAULT` into `lifecycle::spawn`.
- `skills/visualisation/visualise/server/src/server.rs:358-410` — graceful-shutdown
  future + signal handlers (SIGTERM/SIGINT → reasons).
- `skills/visualisation/visualise/server/src/config.rs:13-38` — `Config` struct +
  `#[serde(deny_unknown_fields)]` (line 14); `from_path` (lines 257-268);
  `kanban_columns` field (36-37) / resolution (291-303) — the precedent.
- `skills/visualisation/visualise/server/src/main.rs:9-18` — clap `Cli`
  (`--config` only) → `Config::from_path`.
- `skills/visualisation/visualise/server/Cargo.toml:23-62` — deps (no `humantime`
  yet; `chrono` present but unused by idle path); dev-deps for tests.
- `skills/visualisation/visualise/scripts/launch-server.sh:100-196` — binary
  tri-precedence + `config.json` write + background launch.
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:147-234` —
  reads `visualiser.kanban_columns`, emits `config.json` via `jq -n`.
- `scripts/config-read-value.sh:60-91` — nested `section.subkey` reader.
- `scripts/config-common.sh:26-85` — config-file discovery + frontmatter extract.
- `skills/visualisation/visualise/scripts/launcher-helpers.sh:135-192` —
  `stop_server_stop` (SIGTERM → SIGKILL).
- `skills/visualisation/visualise/server/tests/lifecycle_idle.rs` — fast-clock
  idle test (the template to copy).
- `skills/visualisation/visualise/server/tests/lifecycle_owner.rs:26` —
  `idle_limit_ms: i64::MAX` "disabled" precedent.

## Architecture Insights

- **One channel, three triggers.** All shutdown paths converge on a single
  `mpsc<ShutdownReason>` and one graceful-shutdown future. Disabling idle shutdown
  is a local change to the lifecycle loop's emission; the other two triggers are
  structurally independent and untouched — exactly what the work item's disable
  semantics require.
- **`config.json` is the server's only config surface.** The `--config` JSON,
  guarded by `deny_unknown_fields`, is the hard contract between the shell
  launcher and the Rust process. New server-consumed keys MUST be added in
  lockstep (shell emit + serde field) or the server won't boot. `visualiser.binary`
  is a red herring as a template — it never crosses this boundary.
- **The verification seam is the value, not the clock.** `lifecycle::Settings`
  being injectable at `spawn` is a deliberate testability seam (doc comment
  `lifecycle.rs:19-22`): "tests pass `Settings { tick: 50ms, idle_limit_ms: 200 }`
  without any test-only conditional in the module itself." 0100 does not need to
  introduce a `Clock` abstraction; it needs to keep `idle_limit_ms` injectable
  after wiring config through.
- **Parse in Rust, resolve precedence in shell.** Because the env var is invisible
  to the server, the clean division is: shell picks the winning raw string
  (env > config > default), Rust parses/validates/fails-fast. This keeps a single
  parsing implementation and satisfies fail-fast.
- **Wall-clock, not monotonic.** Idle measurement is `SystemTime`-based, so an NTP
  step or manual clock change perturbs the computed idle duration. Pre-existing
  behaviour, not introduced by 0100, but worth noting given the 8h window.

## Historical Context

- `meta/work/0055-sidebar-activity-feed.md` and
  `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md:148` — located
  the HTTP-activity idle-timeout tracker at `server/src/activity.rs` (the seam
  0100 builds on).
- `meta/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap.md` — origin of
  the hard-coded 30-min idle and the `ACCELERATOR_VISUALISER_BIN` >
  `visualiser.binary` > cached-binary precedence; the lifecycle/config design 0100
  extends.
- `meta/decisions/ADR-0024-visualiser-kanban-column-config.md` — establishes the
  `visualiser.kanban_columns` key read from the `visualiser:` block, with
  boot-time validation and defaults. The **closest precedent** for adding
  `visualiser.idle_timeout`; a parallel ADR would mirror its structure.
- `meta/decisions/ADR-0016-userspace-configuration-model.md` /
  `ADR-0017-configuration-extension-points.md` — the `.accelerator/config.md`
  override-layering model and config-key conventions.
- `meta/reviews/plans/2026-04-18-meta-visualiser-phase-2-server-bootstrap-review-1.md`
  — flags shutdown cleanup ordering, concurrent-launcher races, the `config.json`
  contract duplication, and the `visualiser.binary` arbitrary-exec concern —
  context for lifecycle/config-precedence design.

### Docs/artefacts to update when the key lands (work item Dependencies check)

- `skills/config/configure/SKILL.md:546-577` — the canonical "Accelerator
  Configuration Reference"; `### visualiser` table currently lists only
  `kanban_columns`. Add an `idle_timeout` row + example. (There is **no** separate
  `config-reference.md`; this SKILL.md is the reference.)
- `skills/visualisation/visualise/SKILL.md:39,63` — user-facing doc stating the
  30-minute idle behaviour.
- `README.md:474` — "auto-exits after 30 minutes idle" → 8h / configurable.
- `CHANGELOG.md` `## [Unreleased]` — feature entry; existing idle behaviour at
  line 74, the Playwright idle-timeout env-var precedent at lines 139-142.
- `.accelerator/config.md` (`visualiser:` block, lines 2-3) — optionally add a
  canonical `idle_timeout` example.

## Related Research

- `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md` — server
  module map; located the activity tracker.
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md` —
  original implementation-context (server, binary acquisition, lifecycle).
- `meta/research/codebase/2026-06-04-changelog-1.21.0-cleanup.md` — visualiser
  config-key renames / changelog state.

## Review Context (work item already approved)

`meta/reviews/work/0100-configurable-visualiser-auto-shutdown-review-1.md` —
three review passes, final verdict **APPROVE** (reviewer override after COMMENT),
status `draft → ready`. All eight acceptance behaviours are covered: 8h default,
config-over-default, env-over-config, compound `"1h30m"` parsing, mixed-case
`"Never"` disable, numeric `0` disable, and fail-fast on unparseable input.
Remaining findings are optional polish (e.g. the "case-insensitively" phrasing
applied to `0`, grammar-by-example deferral to `humantime`).

## Open Questions

- **Disable representation in `config.json`/`Config`.** `idle_timeout` is a string
  in config (`"8h"`, `"never"`), but the numeric disable token `0` is a YAML
  integer. After the shell emits JSON, will `Config`'s field be a string (so `0`
  arrives as `"0"`), or does the shell normalise? `serde` typing must accommodate
  both `"never"` and `0` — likely a `String` field with Rust-side interpretation,
  or a custom deserialiser. (Work item review flagged the `0` type/quoting
  ambiguity as an unresolved minor.)
- **Where exactly the env var is read.** `launch-server.sh` (then threaded as a
  new `--idle-timeout` CLI arg, parsed in `write-visualiser-config.sh:15-46`) vs
  reading `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` directly inside
  `write-visualiser-config.sh`. The former matches the existing `--owner-pid`
  threading pattern.
- **Default representation.** Work item says the 8h default is "expressed
  internally as the same duration-string format." Is the default literally the
  string `"8h"` run through `humantime` (single code path), or a Rust `Duration`
  constant with the string only at the config layer? The former gives one parsing
  path and is the cleaner reading.
- **`humantime` vs alternatives.** Work item names `humantime` as "natural fit";
  it parses `"8h"`/`"30m"`/`"1h30m"`. Confirm licensing/supply-chain at planning
  (flagged in work item Dependencies). `humantime::parse_duration` returns
  `std::time::Duration`; the disable tokens are handled before/around it.
- **ADR needed?** ADR-0024 set a precedent for documenting a `visualiser.*` key.
  Does 0100 warrant a parallel ADR (config key + default change + validation), or
  is it small enough to ride the work item + changelog? (Consistent with the
  "tightly scoped ADRs" preference.)
