---
type: plan
id: "2026-06-06-0100-configurable-visualiser-auto-shutdown"
title: "Configurable Visualiser Auto-Shutdown Implementation Plan"
date: "2026-06-06T13:41:38+00:00"
author: "Toby Clemson"
producer: create-plan
status: complete
work_item_id: "work-item:0100"
parent: "work-item:0100"
derived_from: ["codebase-research:2026-06-06-0100-configurable-visualiser-auto-shutdown"]
tags: [visualiser, server, configuration, lifecycle]
revision: "e5f8b4b59e58392c80373edcc442b78ba2cacfce"
repository: "accelerator"
last_updated: "2026-06-06T19:12:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Configurable Visualiser Auto-Shutdown Implementation Plan

## Overview

Make the visualiser server's idle auto-shutdown timeout configurable via a
duration string in the `visualiser:` config block (with an environment-variable
override), and raise the built-in default from 30 minutes to 8 hours so a
working review session left open in a browser tab no longer outlives the server.

The change is a four-stage plumbing job (shell precedence resolution →
`config.json` field → `serde` struct field → `lifecycle::Settings`), plus a
`humantime` duration parser with a disable token, plus fail-fast validation at
boot, plus doc updates. It mirrors the established `visualiser.kanban_columns`
precedent end-to-end, which is the correct template (not `visualiser.binary`,
which is launcher-only and never reaches the Rust process).

## Current State Analysis

- **The idle tracker** is `Activity` — a single `AtomicI64` of last-request
  epoch-millis, touched by an axum middleware on every inbound request
  (`server/src/activity.rs:10-44`). It only stores/reads the timestamp; it never
  decides "idle".
- **The 30-minute timeout is a hard-coded compile-time `const`** at
  `server/src/lifecycle.rs:23-26` (`Settings::DEFAULT.idle_limit_ms =
  30 * 60 * 1000`). This is the **only** place the production value is set, and
  there is **no runtime/config/env override path** today.
- **The idle loop** (`lifecycle::spawn`, `lifecycle.rs:29-52`) is a detached
  `tokio::spawn` driven by `tokio::time::interval(settings.tick)`. It fires
  `ShutdownReason::IdleTimeout` when `idle >= settings.idle_limit_ms`
  (`lifecycle.rs:45-49`, `>=` is inclusive). The first immediate tick is dropped
  (`lifecycle.rs:38`).
- **All three shutdown triggers** (idle timeout, owner-PID exit, SIGTERM/SIGINT)
  converge on one `mpsc::channel::<ShutdownReason>(4)` (`server.rs:315`) and a
  single graceful-shutdown future (`server.rs:358-391`). Disabling idle shutdown
  is a purely local change to the idle comparison — the other two triggers are
  structurally independent and remain in force.
- **Config reaches the server only through `config.json`.** `.accelerator/config.md`
  / `config.local.md` are read shell-side by `config-read-value.sh`;
  `launch-server.sh` calls `write-visualiser-config.sh` to assemble a `config.json`
  via `jq -n`; the server is launched with `--config <path>` only
  (`main.rs:9-13`) and deserialises into `Config` via `serde_json`
  (`config.rs:257-268`). `Config` carries `#[serde(deny_unknown_fields)]`
  (`config.rs:14`), so any new JSON key MUST have a matching struct field.
- **`visualiser.kanban_columns` is the exact precedent**: read in
  `write-visualiser-config.sh:147-167`, emitted in the `jq -n` object
  (`:204,233`), deserialised as `Option<Vec<String>>` with `#[serde(default)]`
  (`config.rs:36-37`), resolved + validated by `Config::resolve_kanban_columns()`
  (`config.rs:291-303`), invoked with `?` propagation in `AppState::build`
  (`server.rs:59`). Its boot-time rejection of an empty list
  (`ConfigError::EmptyKanbanColumns`) is the fail-fast template.
- **No env vars are forwarded to the server.** `ACCELERATOR_VISUALISER_BIN` is
  consumed entirely by `launch-server.sh`. So `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`
  must likewise be read shell-side.
- **The verification seam already exists.** `lifecycle::spawn` takes an
  injectable `Settings { tick, idle_limit_ms }`. `tests/lifecycle_idle.rs` drives
  it with `{ tick: 50ms, idle_limit_ms: 200 }`; `tests/lifecycle_owner.rs:23-27`
  drives it with `idle_limit_ms: i64::MAX` to neutralise the idle trigger. No
  `Clock` trait is needed — tests shrink the *threshold*, not the clock.

## Desired End State

When this plan is complete:

- The server idles for **8 hours** by default before auto-shutting-down (was 30
  minutes), with no config required.
- Setting `visualiser.idle_timeout: "30m"` (or `"1h30m"`, etc.) in
  `.accelerator/config.md` / `config.local.md` changes the idle window; the
  value is a `humantime`-style duration string.
- Setting `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h` overrides the config key for
  one-shot, shell-scoped use. Precedence is **env > config > 8h default**.
- Setting the value to `"never"` (case-insensitive), `0`, or any zero-length
  duration (`"0s"`, `"0ms"`) disables idle auto-shutdown entirely; owner-PID exit
  and explicit `stop` still terminate the server.
- An unparseable value (e.g. `"soon"`) makes the server fail fast at boot with a
  clear error naming the bad value, rather than silently defaulting.

Verified by: the automated test suites in each phase below (`humantime` parse
unit tests, lifecycle integration tests using the injectable `Settings`, shell
launcher tests for precedence + disable pass-through, and a boot fail-fast test),
plus a manual end-to-end launch with a short configured timeout.

### Key Discoveries:

- The disable sentinel is `idle_limit_ms: i64::MAX` — already used by
  `tests/lifecycle_owner.rs:25` and inert against the `idle >= idle_limit_ms`
  comparison (`lifecycle.rs:46`).
- `config-read-value.sh` returns config scalars as text, so YAML `idle_timeout: 0`
  arrives shell-side as the string `"0"` and as a JSON string `"0"` in
  `config.json` — an `Option<String>` field receives it cleanly, no custom
  deserialiser required.
- `humantime` is **not yet a dependency** (`Cargo.toml:23-54`); `chrono` is
  present but unused by the idle path. `humantime::parse_duration` returns
  `std::time::Duration` and parses `"8h"`, `"30m"`, `"1h30m"`.
- Fail-fast wiring exists: `ConfigError` → `AppStateError::Config`
  (`server.rs:146`) → `ServerError::Startup` (`server.rs:169`) → `ExitCode::from(1)`
  (`main.rs:46-55`). `resolve_kanban_columns()?` at `server.rs:59` is the call
  site to copy.
- `write-visualiser-config.sh` is invoked directly by tests
  (`tests/config_contract.rs`, `scripts/test-write-visualiser-config.sh`) without
  the env var, so the "omit-when-unset → Rust default" path is naturally exercised.

## What We're NOT Doing

- **No CLI flag** on `/accelerator:visualise` — configuration is via the
  `visualiser:` block and the env-var override only (work item Out of scope).
- **Not changing the other two shutdown triggers** (launching-process exit,
  explicit `stop`) — they remain exactly as-is.
- **No ADR** — per decision, this rides the work item + CHANGELOG rather than a
  parallel ADR (ADR-0024 set the kanban precedent but this is small enough to
  ride the work item).
- **Not broadening the disable token set** to `"off"`/`"none"` — only `"never"`
  and `0` (work item Assumptions; current answer is "no").
- **No `Clock` abstraction** — the existing injectable `Settings` seam is
  sufficient; introducing a clock trait is unnecessary scope.
- **Not switching idle measurement to monotonic time** — the pre-existing
  `SystemTime` (wall-clock) basis is retained (noted, not changed by 0100).

## Implementation Approach

Resolve **precedence in shell, parse/validate in Rust**. Since the env var is
invisible to the server process, `write-visualiser-config.sh` resolves
`env > config > (omit)` and emits a single raw `idle_timeout` string into
`config.json` (omitting the key entirely when neither env nor config is set).
Rust owns the canonical 8h default and the single `humantime` parse path: an
absent field defaults to `"8h"`, and every value — including the default — flows
through one parser that also recognises the disable tokens and fails fast on
garbage.

Sequenced so each phase is independently integratable/mergeable, TDD-first:

1. **Phase 1** raises the default 30m → 8h (independently valuable; mergeable
   alone).
2. **Phase 2** adds the `humantime` dependency, the `Config.idle_timeout` field,
   the parser + disable sentinel + fail-fast, and rewires production to the
   resolved value. Behaviour-preserving (still 8h, since the shell doesn't emit
   the field yet).
3. **Phase 3** makes `write-visualiser-config.sh` resolve precedence and emit the
   field, completing the end-to-end path, with shell launcher tests.
4. **Phase 4** updates user-facing docs + CHANGELOG for the new key.

---

## Phase 1: Raise the default idle window to 8 hours

### Overview

Change the only hard-coded production value from 30 minutes to 8 hours and bring
the user-facing "30 minutes" doc references into line. No configurability yet —
this is the independently valuable slice the work item permits landing first.

### Changes Required:

#### 1. The default constant

**File**: `skills/visualisation/visualise/server/src/lifecycle.rs`
**Changes**: Change `idle_limit_ms` in `Settings::DEFAULT` from `30 * 60 * 1000`
to `8 * 60 * 60 * 1000`, and update the doc comment.

```rust
impl Settings {
    /// Production defaults: 60s tick, 8-hour idle window.
    /// Tests pass shortened values via `Settings { tick: 50ms,
    /// idle_limit_ms: 200 }` without any test-only conditional
    /// in the module itself.
    pub const DEFAULT: Settings = Settings {
        tick: Duration::from_secs(60),
        idle_limit_ms: 8 * 60 * 60 * 1000,
    };
}
```

> Note on the two 8h representations: after Phase 2, production builds
> `Settings` from the resolved `DEFAULT_IDLE_TIMEOUT = "8h"` string and only
> borrows `Settings::DEFAULT.tick` — so `Settings::DEFAULT.idle_limit_ms`
> becomes a second, test-and-fallback-only expression of the same 8h truth.
> To stop the two drifting, Phase 2's resolver unit tests assert that the
> absent-field default (`resolve_idle_limit_ms()` with `idle_timeout: None`)
> equals `Settings::DEFAULT.idle_limit_ms`, tying the string default and the
> const together.

#### 2. User-facing default references

**File**: `skills/visualisation/visualise/SKILL.md` (lines ~39, ~63)
**Changes**: Replace the "30 minutes idle" wording with "8 hours idle".

**File**: `README.md` (line ~474)
**Changes**: "auto-exits after 30 minutes idle" → "auto-exits after 8 hours
idle".

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:visualiser`
- [x] Integration tests pass: `mise run test:integration:visualiser`
- [x] No stale "30 minutes"/"30-minute" idle references remain:
  `grep -rn "30 minute" README.md skills/visualisation/visualise/SKILL.md skills/visualisation/visualise/server/src` returns nothing idle-related

#### Manual Verification:

- [x] `SKILL.md` and `README.md` read coherently with the new 8-hour figure.

---

## Phase 2: Parse a configurable timeout in Rust, with fail-fast and a disable token

### Overview

Add the `humantime` dependency, the `idle_timeout` field on `Config`, a single
resolver that turns the duration string (or absent field) into `idle_limit_ms`
(or the `i64::MAX` disable sentinel), and rewire production to use it. The shell
does not emit the field yet, so this phase is behaviour-preserving (absent →
`"8h"` default → 8h, identical to Phase 1) while making every new code path
unit-testable now.

### Changes Required:

#### 1. Add the `humantime` dependency

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: Add to `[dependencies]`.

```toml
humantime = "2"
```

`humantime` (Paul Colomiets) is dual MIT/Apache-2.0 with no transitive
dependencies — acceptable for supply-chain/licensing per the work item
Dependencies check. `Cargo.lock` updates accordingly.

#### 2. Add the `idle_timeout` field to `Config`

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: Add a field mirroring `kanban_columns` (`config.rs:36-37`).

```rust
    /// Idle auto-shutdown window as a humantime duration string
    /// (`"8h"`, `"30m"`, `"1h30m"`), or a disable token (`"never"`, `0`,
    /// or any zero-length duration). Absent → the built-in 8h default.
    /// Resolved + validated at boot by `resolve_idle_limit_ms`.
    #[serde(default)]
    pub idle_timeout: Option<String>,
```

#### 3. Add the resolver, default constant, and error variant

**File**: `skills/visualisation/visualise/server/src/config.rs`
**Changes**: Add a `DEFAULT_IDLE_TIMEOUT` constant, a `DISABLED_IDLE_LIMIT_MS`
sentinel, a `Config::resolve_idle_limit_ms` method (mirroring
`resolve_kanban_columns`), and a `ConfigError::InvalidIdleTimeout` variant.

```rust
/// Canonical default idle window, expressed in the same duration-string
/// form accepted from config so there is a single parse path.
const DEFAULT_IDLE_TIMEOUT: &str = "8h";

/// Sentinel meaning "idle auto-shutdown disabled". Inert against the
/// `idle >= idle_limit_ms` comparison in lifecycle.rs (the production loop
/// just compares the value it is handed; the sentinel never appears there
/// literally).
///
/// The disable tests (the owner-death test in `tests/lifecycle_owner.rs` and the
/// new `disabled_idle_never_fires`) reference this exported constant rather than a
/// bare `i64::MAX`, so the disable contract is named in one place and shared by
/// import — a future change to the idle comparison cannot silently break the
/// disable assumption without the named constant showing up in the diff.
pub const DISABLED_IDLE_LIMIT_MS: i64 = i64::MAX;

/// Largest finite idle window we store: one below the disable sentinel,
/// so an over-large configured duration clamps here and can never be
/// mistaken for "disabled".
const MAX_IDLE_LIMIT_MS: i64 = DISABLED_IDLE_LIMIT_MS - 1;

impl Config {
    /// Resolve the idle window into milliseconds, or the disable sentinel.
    ///
    /// Semantics:
    /// - Absent field → the built-in `"8h"` default, parsed through the
    ///   same path as user input.
    /// - `"never"` (case-insensitive), the bare `"0"`, or *any* zero-length
    ///   duration (`"0s"`, `"0ms"`, …) → `DISABLED_IDLE_LIMIT_MS`, so the
    ///   "zero idle window" case is uniform regardless of spelling.
    /// - Any other value → parsed by `humantime`; an unparseable value is
    ///   rejected (`ConfigError::InvalidIdleTimeout`, carrying the
    ///   underlying parse error) so the server fails fast at boot rather
    ///   than silently defaulting.
    pub fn resolve_idle_limit_ms(&self) -> Result<i64, ConfigError> {
        let raw = self.idle_timeout.as_deref().unwrap_or(DEFAULT_IDLE_TIMEOUT);
        let trimmed = raw.trim();
        // Disable tokens handled before parsing: the textual "never" and the
        // bare "0" (which humantime cannot parse, lacking a unit).
        if trimmed.eq_ignore_ascii_case("never") || trimmed == "0" {
            return Ok(DISABLED_IDLE_LIMIT_MS);
        }
        let dur = humantime::parse_duration(trimmed).map_err(|source| {
            ConfigError::InvalidIdleTimeout { value: raw.to_string(), source }
        })?;
        // A zero-length window ("0s", "0ms", …) also disables, matching the
        // bare-"0" token above.
        if dur.is_zero() {
            return Ok(DISABLED_IDLE_LIMIT_MS);
        }
        // Saturate in u128 *before* the i64 cast (an over-large duration must
        // clamp to MAX_IDLE_LIMIT_MS, never wrap negative), and floor at 1ms so a
        // sub-millisecond-but-non-zero value ("1ns", "500us") — which is NOT
        // is_zero() yet truncates to 0 ms — stays a tiny finite window (fires on
        // the next tick) rather than collapsing to `idle_limit_ms == 0`, which
        // the loop would treat as fire-on-first-tick.
        Ok(dur.as_millis().min(MAX_IDLE_LIMIT_MS as u128).max(1) as i64)
    }
}
```

The error variant carries the underlying `humantime` parse error as
`source`, mirroring the established `ConfigError::InvalidScanRegex { pattern,
source }` pattern (`config.rs:318-321`) so the boot log says *why* the value
was rejected, not just that it was:

```rust
    // The accepted-format guidance is duplicated in write-visualiser-config.sh's
    // pre-flight error — keep the two messages in sync.
    #[error("invalid visualiser.idle_timeout '{value}': expected a duration like \"8h\", \"30m\", \"1h30m\", or \"never\"/0 to disable: {source}")]
    InvalidIdleTimeout {
        value: String,
        source: humantime::DurationError,
    },
```

#### 4. Store the resolved value on `AppState` and wire it into the lifecycle loop

**File**: `skills/visualisation/visualise/server/src/server.rs`
**Changes**:
- Add an `idle_limit_ms: i64` field to `AppState` (struct at `server.rs:40-52`).
- Resolve it in `AppState::build` alongside `resolve_kanban_columns()?`
  (`server.rs:59`), propagating `ConfigError` via the existing
  `AppStateError::Config` `#[from]`.
- In `run` (`server.rs:317-323`), replace `crate::lifecycle::Settings::DEFAULT`
  with a `Settings` built from the resolved value, keeping `Settings::DEFAULT.tick`.
- **Neutralise the wrapper error message.** `AppStateError::Config`'s
  `#[error(...)]` is currently hard-coded to `"invalid work-item config: {0}"`
  (`server.rs:145`), so an idle-timeout failure would be logged as `…invalid
  work-item config: invalid visualiser.idle_timeout 'soon'…` — misattributing
  the failure. Rename the wrapper message to a domain-neutral `"invalid
  configuration: {0}"` (the inner `ConfigError` Display already names the
  specific key). This also corrects the pre-existing misnomer for
  `EmptyKanbanColumns`.
- **Adopt the named sentinel in the existing disable test.** Update
  `tests/lifecycle_owner.rs:25` to use `config::DISABLED_IDLE_LIMIT_MS` in place of
  the bare `i64::MAX`, so the disable contract is shared by import (the new
  `disabled_idle_never_fires` test does the same).

```rust
// in AppState::build, next to resolve_kanban_columns()?:
let idle_limit_ms = cfg.resolve_idle_limit_ms()?;
// ...store on AppState { idle_limit_ms, .. }

// in run(), replacing the Settings::DEFAULT argument:
crate::lifecycle::spawn(
    activity.clone(),
    state.cfg.owner_pid,
    state.cfg.owner_start_time,
    crate::lifecycle::Settings {
        tick: crate::lifecycle::Settings::DEFAULT.tick,
        idle_limit_ms: state.idle_limit_ms,
    },
    tx.clone(),
);
```

#### 5. Unit tests for the resolver (TDD — write first)

**File**: `skills/visualisation/visualise/server/src/config.rs` (`#[cfg(test)]`)
**Changes**: Add tests mirroring the kanban resolver tests (`config.rs:582-629`),
using the existing `bare_config_json()` helper (no `idle_timeout` field) and
small JSON literals with the field.

- absent field → 8h (`8 * 60 * 60 * 1000` ms)
- `"30m"` → `30 * 60 * 1000`
- `"1h30m"` → `90 * 60 * 1000` (compound parsing)
- `"never"` / `"Never"` / `"NEVER"` → `DISABLED_IDLE_LIMIT_MS`
- `"0"` → `DISABLED_IDLE_LIMIT_MS` (disables, not "immediate")
- `"0s"` / `"0ms"` → `DISABLED_IDLE_LIMIT_MS` (zero-length window disables too —
  locks the bare-`"0"`-vs-`"0s"` boundary so they cannot diverge)
- `"1ns"` / `"500us"` (sub-millisecond, non-zero) → `1` ms (floored to a tiny
  finite window — **not** `0` (fire-immediately) and **not** the disable sentinel)
- `"100000000000years"` (or any duration whose millis exceed `i64::MAX`) →
  `MAX_IDLE_LIMIT_MS` (saturates near the sentinel; must **not** wrap to `0`)
- `"  8h  "` (surrounding whitespace) → 8h
- absent field → exactly `28_800_000` ms (absolute 8h assertion, pinning the unit)
  **and** equal to `Settings::DEFAULT.idle_limit_ms` (relative drift guard)
- `"soon"`, `"00"`, `"0.0"`, `"   "` (whitespace-only) →
  `Err(ConfigError::InvalidIdleTimeout { .. })` (fail fast — not disable, not
  default)

#### 6. Lifecycle integration tests (TDD — write first)

**File**: `skills/visualisation/visualise/server/tests/lifecycle_idle.rs`

**(a) Disable sentinel.** Add a test (sibling to
`idle_timeout_fires_with_fast_clock`) asserting that with `idle_limit_ms:
DISABLED_IDLE_LIMIT_MS` and a fast tick, **no** `ShutdownReason` arrives within a
short window — confirming the disable sentinel neutralises the idle trigger.
(Owner trigger is skipped with `owner_pid: 0`.) This is a *smoke check* (it proves
"did not fire within ~15 ticks", not "never fires"); it is paired with the
resolver unit assertion that the disable tokens return `DISABLED_IDLE_LIMIT_MS`
exactly, which is the authoritative guarantee.

```rust
#[tokio::test]
async fn disabled_idle_never_fires() {
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity, 0, None,
        lifecycle::Settings { tick: Duration::from_millis(20), idle_limit_ms: config::DISABLED_IDLE_LIMIT_MS },
        tx,
    );
    // Several ticks elapse; nothing should fire.
    let res = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
    assert!(res.is_err(), "disabled idle must not fire, got {res:?}");
}
```

**(b) Boundary survival (TDD — write first).** The existing
`idle_timeout_fires_with_fast_clock` only asserts the timeout *fires by* a
deadline; nothing asserts the server is *still alive* just below it, so an
off-by-tick or fire-unconditionally mutation of `idle >= settings.idle_limit_ms`
(`lifecycle.rs:46`) would pass. Add a two-sided test: with a finite
`idle_limit_ms` comfortably above several ticks, assert **no** shutdown arrives
in a window below the threshold, **then** that one arrives shortly after — the
"within tolerance of D" boundary the work item's acceptance criteria require.

```rust
#[tokio::test]
async fn idle_survives_below_threshold_then_fires() {
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity, 0, None,
        lifecycle::Settings { tick: Duration::from_millis(20), idle_limit_ms: 400 },
        tx,
    );
    // Below the threshold: must still be alive.
    let early = tokio::time::timeout(Duration::from_millis(150), rx.recv()).await;
    assert!(early.is_err(), "must not fire before the threshold, got {early:?}");
    // After the threshold: must fire.
    let late = tokio::time::timeout(Duration::from_millis(1000), rx.recv()).await;
    assert!(matches!(late, Ok(Some(ShutdownReason::IdleTimeout))), "must fire after the threshold, got {late:?}");
}
```

**(c) Disable does not suppress the owner trigger (TDD — write first).** AC5/AC7
require that with idle disabled the server **still** exits on owner-process exit
(and `stop`). The disable smoke test above uses `owner_pid: 0`, which
short-circuits the owner check, so it proves nothing about that clause. Add a test
mirroring the existing owner-death test (`tests/lifecycle_owner.rs`) but with
`idle_limit_ms: config::DISABLED_IDLE_LIMIT_MS` and a **dead** owner PID, asserting
`OwnerPidExited` still arrives — pinning the "disable idle, not all triggers"
contract. (`stop`/SIGTERM is structurally independent and covered elsewhere; the
owner path is the one adjacent to the disable change.)

> **Test-file imports:** `lifecycle_idle.rs` and `lifecycle_owner.rs` already
> import `std::time::Duration` and `ShutdownReason`, so the only genuinely new
> symbol is `config` (the crate-root module) for `config::DISABLED_IDLE_LIMIT_MS`;
> add it to the grouped `use accelerator_visualiser::{...}` line, matching the
> existing import style.
>
> **Determinism:** these are negative, wall-clock-timed assertions, and this repo's
> visualiser/config suites are known to flake under parallel CI load. Prefer
> `tokio::time::pause()` + `advance()` (the loop's `tokio::time::interval` honours
> paused time) over real sleeps for the disable and boundary-survival checks, or
> give the quiet windows generous slack relative to the tick.

#### 6b. Resolve→wire→loop test (TDD — write first)

**File**: a focused `AppState::build` unit test — prefer a `#[cfg(test)]` module in
`server.rs` (lightest seam; the resolver is in-crate) over a heavy integration test
that stands up the full indexer just to read one scalar. Confirm `AppState.idle_limit_ms`
is `pub` (or expose a `pub(crate)` accessor) so the test can read the stored value.
**Changes**: The resolver unit tests verify the parsed millisecond value, but
nothing verifies the value actually reaches the lifecycle loop — a regression that
passed `Settings::DEFAULT.idle_limit_ms` instead of `state.idle_limit_ms` in `run`
would pass every other test. Add a test that builds `AppState` from a config with
a known `idle_timeout` (e.g. `"30m"`) and asserts `AppState.idle_limit_ms` equals
the resolved value (`30 * 60 * 1000`) — pinning the resolve→store→`Settings` wiring
that Section 4 introduces.

#### 7. Boot fail-fast test (TDD — write first)

**File**: `skills/visualisation/visualise/server/tests/config_cli.rs` (or a
fixture-driven sibling) plus a fixture `tests/fixtures/config.invalid-idle-timeout.json`
(a copy of an **otherwise-valid** config with only `"idle_timeout": "soon"`
added — naming mirrors the existing `config.missing-required.json` descriptor
style).
**Changes**: Assert the binary exits with **exit code 1 specifically** (via the
`ConfigError → AppStateError → ServerError::Startup` path, `ExitCode::from(1)`)
when launched with `--config` pointing at the fixture. Assert `code == 1`, **not
merely non-zero**: `config_cli.rs`'s `exits_2_when_config_missing` shows the
missing/unreadable-config path uses code 2, so a too-loose "non-zero" assertion
plus a malformed fixture would green-light a regression where the resolver never
runs. The fixture must deserialise cleanly and fail *only* at
`resolve_idle_limit_ms`, isolating the resolver's fail-fast behaviour.

> Note: stderr is redirected to `/dev/null` after config load
> (`main.rs:34`), so the descriptive message lands in the tracing log file, not
> stdout/stderr — identical to how `EmptyKanbanColumns` behaves today. The test
> asserts on the **exit code**, not stderr text. (User-facing visibility of the
> bad value is handled by the shell-side pre-flight validation in Phase 3.)

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `mise run test:unit:visualiser`
- [x] Integration tests pass: `mise run test:integration:visualiser`
- [x] Compile check passes: `cargo check --manifest-path
  skills/visualisation/visualise/server/Cargo.toml` (no `typecheck` mise task
  exists; `mise run check` runs the repo's read-only format/lint checks)
- [x] `humantime` resolves in `Cargo.lock`: `cargo tree --manifest-path
  skills/visualisation/visualise/server/Cargo.toml -i humantime` lists it, with
  **no transitive dependencies** (humantime v2.3.0, no deps)
- [x] `humantime` license + MSRV verified: the resolved `2.3.0` version is dual
  MIT/Apache-2.0 and declares no `rust-version` (does not raise our MSRV); the
  pinned-toolchain `cargo check` is the backstop for an MSRV regression once the
  lockfile is committed
- [x] The bad-idle fixture causes a non-zero exit (covered by the new
  `config_cli` test, which asserts exit code 1 specifically)

#### Manual Verification:

- [x] With no config change, a real launch still idles at 8h (unchanged from
  Phase 1) — behaviour-preserving: the shell does not emit the field yet, so
  `resolve_idle_limit_ms` returns the `"8h"` default (verified by the
  absent-field resolver unit test pinning `28_800_000` ms).

---

## Phase 3: Resolve precedence and emit the field in the launcher

### Overview

Make `write-visualiser-config.sh` resolve `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`
(env) > `visualiser.idle_timeout` (config) > (omit), and emit an `idle_timeout`
string into `config.json` only when one of the first two is set. This completes
the end-to-end path. The key is omitted when unset so Rust's 8h default applies
(single source of truth for the default).

### Changes Required:

#### 1. Resolve precedence and emit the field

**File**: `skills/visualisation/visualise/scripts/write-visualiser-config.sh`
**Changes**: After the kanban block (~line 173), resolve the idle timeout
(env > config > empty), **validate it on the terminal before launch**, then
thread it into the `jq -n` object, omitting the key when empty.

```sh
# Idle auto-shutdown window. Precedence: env var > visualiser.idle_timeout
# config key > (omit → Rust applies the 8h default).
#
# Note on the empty env var: `:-` treats ACCELERATOR_VISUALISER_IDLE_TIMEOUT=""
# (set-but-empty) identically to unset, so an empty env value falls through to
# the config key rather than overriding it with "".
IDLE_TIMEOUT="${ACCELERATOR_VISUALISER_IDLE_TIMEOUT:-}"
if [ -z "$IDLE_TIMEOUT" ]; then
  IDLE_TIMEOUT="$("$PLUGIN_ROOT/scripts/config-read-value.sh" "visualiser.idle_timeout" "" 2>/dev/null || true)"
fi
```

**Shell-side pre-flight validation (visible-error UX).** Because the server
redirects stderr to `/dev/null` after config load, the Rust `InvalidIdleTimeout`
message only reaches the tracing log — so a mistyped duration would otherwise
look like a silent launch failure. The launcher therefore validates a non-empty
`IDLE_TIMEOUT` *before* spawning the server and errors to the **terminal** on an
obviously-bad value:

```sh
if [ -n "$IDLE_TIMEOUT" ]; then
  # Trim surrounding whitespace so the shell's accept-set is a superset of Rust's
  # (resolve_idle_limit_ms trims before parsing); bash-3.2 safe. LANG=C keeps the
  # whitespace class deterministic across locales, matching the launcher's
  # existing locale-hardening. NOTE: this is a *separate* block from the guard
  # below on purpose — a whitespace-only value collapses to empty here and is then
  # treated as unset (falls through to the 8h default), rather than reaching the
  # guard and erroring.
  IDLE_TIMEOUT="$(printf '%s' "$IDLE_TIMEOUT" | LANG=C sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
fi
if [ -n "$IDLE_TIMEOUT" ]; then
  # Coarse typo-guard. A duration-SHAPED value starts with a digit; a disable
  # token is `never`/`0`. Anything else (e.g. "soon", "off", "in a bit") clearly
  # is not a duration and is rejected here, on the user's terminal, where the
  # Rust error is invisible (stderr is /dev/null'd after config load).
  #
  # This is deliberately a *shape* check, NOT the humantime grammar: every valid
  # humantime duration begins with a digit, so this can never reject a value Rust
  # accepts (including compound "1h30m" and spaced "1h 30m"). Rust's
  # resolve_idle_limit_ms is the authoritative parser and fail-fast backstop for
  # anything the guard waves through (e.g. "5 zonks", "0.0").
  # Keep the example list in the message in sync with
  # ConfigError::InvalidIdleTimeout (config.rs).
  case "$IDLE_TIMEOUT" in
    [Nn][Ee][Vv][Ee][Rr] | 0) : ;;  # disable tokens
    # Zero-length durations (0s/0ms) are digit-led, so they pass here as
    # duration-shaped; Rust resolves them to the disable sentinel.
    [0-9]*) : ;;                     # duration-shaped: starts with a digit
    *)
      echo "error: invalid visualiser.idle_timeout '$IDLE_TIMEOUT': expected a duration like \"8h\", \"30m\", \"1h30m\", or \"never\"/0 to disable" >&2
      exit 1
      ;;
  esac
fi
```

> The guard rejects only the unambiguously-wrong shape (no leading digit, not a
> disable token). It **accepts** any digit-led value — including ones Rust will
> still reject (`"5 zonks"`, `"0.0"`) — because the shell deliberately does not
> reimplement the humantime grammar; Rust remains the single source of truth and
> fails fast on those at boot.

> **Accepted tradeoff (per review decision):** this is a deliberate *second*
> validation site, duplicating the disable-token knowledge that also lives in
> Rust. The architecture lens flagged the single-parse-path purity cost; we
> accept it to make the error visible at launch. The shell guard is coarse on
> purpose (it does not reimplement the full humantime grammar — e.g. it does not
> validate unit names or reject overflow); Rust's `resolve_idle_limit_ms` is the
> source of truth and still fails fast on anything the shell waves through.

Then add `--arg idle_timeout "$IDLE_TIMEOUT"` to the `jq -n` invocation and
conditionally merge the key:

```jq
'{
   # ...existing fields (plugin_root … kanban_columns)...
 }
 + (if $idle_timeout == "" then {} else {idle_timeout: $idle_timeout} end)'
```

#### 2. Shell launcher tests (TDD — write first)

**File**: `skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
**Changes**: Add cases mirroring the kanban cases (`:86-124`), asserting on the
emitted JSON via `assert_json_eq` / absence checks:

- no env, no config → `idle_timeout` key **absent** from `config.json`
  (`.idle_timeout` is null)
- `visualiser.idle_timeout: "30m"` in config, no env → `.idle_timeout == "30m"`
  (config-over-default)
- `visualiser.idle_timeout: 0` (numeric) in config → `.idle_timeout == "0"`
  (numeric token survives as a string)
- `visualiser.idle_timeout: "Never"` in config → `.idle_timeout == "Never"`
  (mixed-case token passes through untouched; case-folding is Rust's job)
- `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h` set **and** a different
  `visualiser.idle_timeout` in config → `.idle_timeout == "2h"`
  (env-over-config)
- `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=""` (set-but-empty) **and** a
  `visualiser.idle_timeout: "30m"` in config → `.idle_timeout == "30m"`
  (empty env falls through to config, does not override with "")
- `visualiser.idle_timeout: "0s"` in config → `.idle_timeout == "0s"`
  (zero-length duration passes the coarse shell guard; Rust resolves it to the
  disable sentinel)
- **compound `visualiser.idle_timeout: "1h30m"`** → `.idle_timeout == "1h30m"`
  (the guard must NOT reject digit-after-unit forms — regression guard) plus the
  spaced form `"1h 30m"` → `.idle_timeout == "1h 30m"`
- **whitespace-padded `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=" 8h "`** →
  `.idle_timeout == "8h"` (shell trims before the guard, so its accept-set is a
  superset of Rust's)
- guard-accepted-but-Rust-invalid (`ACCELERATOR_VISUALISER_IDLE_TIMEOUT="5 zonks"`)
  → passes the coarse shell guard and is emitted (`.idle_timeout == "5 zonks"`);
  Rust fails fast at boot (asserts the guard is permissive, not authoritative)
- quoted vs unquoted config form (`idle_timeout: "8h"` vs `idle_timeout: 8h`)
  both yield the expected raw string (verifies `config-read-value.sh` scalar
  handling)
- **invalid shape rejected on the terminal**: `visualiser.idle_timeout: "soon"`
  (or `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=soon`) → the script exits non-zero and
  prints the `invalid visualiser.idle_timeout 'soon'` message to **stderr**, and
  no `config.json` is emitted (asserts the shell-side pre-flight guard fires)

#### 3. Contract test stays green

**File**: `skills/visualisation/visualise/server/tests/config_contract.rs`
**Changes**: No change expected — the script omits the key when unset, and
`Config` deserialises a config without `idle_timeout` via `#[serde(default)]`.
Confirm the test still passes; if the contract test is extended to assert the
field's presence-when-set, add a case there.

### Success Criteria:

#### Automated Verification:

- [x] Shell config-writer tests pass:
  `bash skills/visualisation/visualise/scripts/test-write-visualiser-config.sh`
- [x] Integration tests pass: `mise run test:integration:visualiser`
- [x] Full launcher tests pass:
  `bash skills/visualisation/visualise/scripts/test-launch-server.sh`
- [x] A config with `visualiser.idle_timeout: "30m"` produces `config.json` with
  `.idle_timeout == "30m"` (covered by the new shell test)

#### Manual Verification:

- [ ] End-to-end: set `visualiser.idle_timeout: "10s"` in
  `.accelerator/config.local.md`, launch the visualiser, issue no requests, and
  confirm it auto-shuts-down shortly after 10s (check `server-stopped.json`
  records `idle-timeout`). Note: at the production 60s tick the actual fire is
  on the next tick boundary, so use a value comfortably above one tick or accept
  up-to-one-tick granularity.
- [ ] `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=never <launch>` keeps the server alive
  past the configured window but it still dies on owner-process exit / `stop`.
- [ ] An invalid value (`visualiser.idle_timeout: "soon"`) makes the launch fail
  **with a visible terminal error** naming the bad value (shell-side pre-flight
  guard); server-info.json never appears. (If the value slips past the coarse
  shell guard, Rust still fails fast at boot with the error in the server log.)

---

## Phase 4: Documentation

### Overview

Document the new `visualiser.idle_timeout` key and env-var override in the
configuration reference and CHANGELOG. (The 30m → 8h default-change wording was
already handled in Phase 1.)

### Changes Required:

#### 1. Configuration reference

**File**: `skills/config/configure/SKILL.md` (the canonical "Accelerator
Configuration Reference"; `### visualiser` table ~lines 550-577)
**Changes**: Add an `idle_timeout` row to the `visualiser` table.

> **Correction (review):** that table currently contains **only** a
> `kanban_columns` row — there is no `binary` row and no
> `ACCELERATOR_VISUALISER_BIN` note to mirror (the `binary` key / env override are
> documented in `skills/visualisation/visualise/SKILL.md` and `README.md`, not
> here). So mirror only the `kanban_columns` row style, and add the
> `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` env override as a short prose note beneath
> the table (the table has no env-var column to extend). For the note's style,
> follow the existing env-var documentation already in this same file — the Jira
> token env vars (`ACCELERATOR_JIRA_TOKEN` / `ACCELERATOR_JIRA_TOKEN_CMD`) — rather
> than inventing a new convention.

The row + note must document:
- **Format**: a humantime duration string (`"8h"`, `"30m"`, `"1h30m"`); **default
  `8h`** when absent.
- **Disable**: `never` (case-insensitive), `0`, or any zero-length duration
  (`0s`, `0ms`) disables idle auto-shutdown. State explicitly that this is
  **disable, not "shut down immediately"**, and that the server **still
  terminates** on owner-process exit and on `/accelerator:visualise stop`.
- **Env override**: `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` takes precedence over the
  config key (precedence: env > config > 8h default).
- **Granularity**: the timeout is honoured to within one ~60s polling tick — the
  shutdown lands on the next tick after the window elapses, so sub-minute values
  are effectively rounded up to the next tick (mirror the kanban row's
  "read once at boot" caveat style).

#### 2. Env-var override, documented where users look

The env override must appear beside its sibling `ACCELERATOR_VISUALISER_BIN`, in
the two places developers already discover visualiser env vars:

**File**: `README.md` (customisation table, ~line 496 — the rows for
`ACCELERATOR_VISUALISER_BIN` and `visualiser.binary`)
**Changes**: That table pairs each env var with its persistent config key (as it
does for `binary`). Add **both**: an `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` row
("One-shot override of the idle auto-shutdown window (duration string, or
`never`/`0` to disable)") **and** a `visualiser.idle_timeout` config-key row, so a
reader sees the persistent key beside the one-shot override.

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**:
- `### Overrides` section (~line 83): add a short entry for
  `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` alongside the existing
  `ACCELERATOR_VISUALISER_BIN` entry, noting it overrides `visualiser.idle_timeout`
  for one-shot, shell-scoped use.
- User-relayed line (~line 63, "auto-exits after 8 hours idle…"): once Phase 3
  ships configurability, qualify this so it does not assert a flat figure — note
  the window is configurable via `visualiser.idle_timeout` /
  `ACCELERATOR_VISUALISER_IDLE_TIMEOUT` and can be disabled. (Phase 1 leaves the
  bare "8 hours" swap; this Phase-4 step adds the "configurable" qualification once
  the feature exists, keeping the Claude-only comment at ~39 consistent.)

#### 3. CHANGELOG

**File**: `CHANGELOG.md` (`## [Unreleased]`)
**Changes**: Two entries — the `[Unreleased]` block currently has only a
`### Changed` section, so **insert `### Added` before it** (Keep-a-Changelog
section ordering: Added → Changed → …):
- **`### Added`**: configurable visualiser idle auto-shutdown via
  `visualiser.idle_timeout` + `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`, `never`/`0`
  (or any zero-length duration) to disable, fail-fast on invalid values.
- **`### Changed`**: visualiser idle auto-shutdown default raised from 30 minutes
  to 8 hours. Record this as its own line (not buried in the feature entry): it
  is a user-observable behaviour change (servers now live ~16× longer before idle
  shutdown), relevant to anyone who relied on the 30-minute auto-cleanup.

#### 4. Canonical config example

**File**: `.accelerator/config.md` (`visualiser:` block)
**Changes**: Add a commented `idle_timeout` example to the `visualiser:` block.

> **Correction (review):** the block currently holds a `kanban_columns` key, not
> `binary`, and has no comments today — so add the example alongside
> `kanban_columns` (e.g. `# idle_timeout: "8h"   # never / 0 to disable`).
> Promote this from "optional" to **done**: a commented example in the file
> developers actually edit is the highest-leverage discoverability lever for a
> low-ceremony per-project setting.

### Success Criteria:

#### Automated Verification:

- [x] Read-only format/lint checks pass: `mise run check` (the repo's CI check
  task; no `typecheck` task exists); no broken internal references.

#### Manual Verification:

- [x] The `configure` SKILL.md `visualiser` table documents `idle_timeout`
  accurately (format, tokens, env override, default), consistent with the
  shipped behaviour.
- [x] CHANGELOG entry reads clearly and names both the key and the default
  change.

---

## Testing Strategy

### Unit Tests (`mise run test:unit:visualiser`, `cargo test --lib`):

- `Config::resolve_idle_limit_ms` for: absent → 8h; `"30m"`; compound `"1h30m"`;
  `"never"`/`"Never"`/`"NEVER"` → disabled; `"0"` → disabled; `"0s"`/`"0ms"`
  (zero-length) → disabled; whitespace trimming (`"  8h  "`); `"soon"`, `"00"`,
  `"0.0"`, whitespace-only → `InvalidIdleTimeout`.
- Edge: a duration whose millis exceed `i64::MAX` saturates to `MAX_IDLE_LIMIT_MS`
  (near the sentinel) and is **not** mistaken for "disabled" nor wrapped to `0`.
- Drift guard: absent-field default equals `Settings::DEFAULT.idle_limit_ms`
  (ties the `"8h"` string default to the lifecycle const).

### Integration Tests (`mise run test:integration:visualiser`, `cargo test --tests`):

- `lifecycle_idle.rs`: existing fast-clock idle-fires test (unchanged) +
  `disabled_idle_never_fires` (`DISABLED_IDLE_LIMIT_MS` sentinel → no fire,
  smoke check) + `idle_survives_below_threshold_then_fires` (two-sided boundary:
  alive below D, fires after).
- Resolve→wire→loop: `AppState::build` with `idle_timeout: "30m"` →
  `AppState.idle_limit_ms == 30 * 60 * 1000` (the value actually reaches `spawn`).
- `config_cli.rs`: invalid-idle-timeout fixture → exit code **1** specifically
  (fail-fast, distinct from the missing-config code 2).
- `config_contract.rs`: script output still deserialises (field omitted when
  unset).

### Shell Tests:

- `test-write-visualiser-config.sh`: env-over-config, empty-env-falls-through,
  config-over-omit, numeric `0` pass-through, `"0s"` pass-through, mixed-case
  `"Never"` pass-through, omit-when-unset, quoted vs unquoted scalar handling,
  **invalid value (`"soon"`) → non-zero exit + terminal error + no `config.json`**.
- `test-launch-server.sh`: still green (no regression in the launch path).

### Acceptance-criteria coverage map (work item 0100):

| AC | Behaviour | Covered by |
|----|-----------|------------|
| 1 | 8h default | resolver unit (absent → 8h) + drift guard + boundary-survival lifecycle test |
| 2 | `"30m"` config-over-default | resolver unit + shell config-over-omit + resolve→wire→loop test |
| 3 | compound `"1h30m"` | resolver unit (`90 * 60 * 1000`) |
| 4 | `2h` env-over-config | shell env-over-config test |
| 5 | `"never"` disable + still exits on owner/stop | resolver unit + `disabled_idle_never_fires` + owner-exits-while-idle-disabled (6c) |
| 6 | mixed-case `"Never"` disable | resolver unit (`"Never"`/`"NEVER"`) + shell pass-through |
| 7 | numeric `0` disables (not immediate) | resolver unit (`"0"`/`"0s"` → disabled) + owner-exits-while-idle-disabled (6c) |
| 8 | invalid `"soon"` fails fast | shell terminal error + `config_cli.rs` exit code 1 |

### Manual Testing Steps:

1. Set `visualiser.idle_timeout: "10s"` in `.accelerator/config.local.md`,
   launch, leave idle, confirm shutdown + `server-stopped.json` reason
   `idle-timeout`.
2. Set `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h` with a different config value;
   confirm the env value wins (inspect the generated
   `config.json`'s `idle_timeout`).
3. Set `visualiser.idle_timeout: "never"`; confirm no idle shutdown, but `stop`
   and owner-exit still terminate.
4. Set `visualiser.idle_timeout: "soon"`; confirm the launch fails fast and the
   server log names the bad value.

## Performance Considerations

None. The resolver runs once at boot; the loop comparison is unchanged. The 8h
window means a server can live ~16× longer than before — acceptable and the
explicit intent. Idle measurement remains wall-clock (`SystemTime`-based), so an
NTP step or manual clock change perturbs the computed idle duration; this is
pre-existing behaviour, merely more visible over an 8h window. Because the 16×
wider window proportionally widens the chance a forward clock step crosses the
threshold and shuts the server down mid-review (the exact scenario this work item
set out to prevent), switching the idle delta to monotonic time (`Instant`) is
worth a **follow-up** — explicitly out of scope here (see "What We're NOT Doing"),
but the larger default raises its priority.

## Migration Notes

No data migration. Older `config.json` files (and direct callers of
`write-visualiser-config.sh` in tests) simply omit `idle_timeout`;
`#[serde(default)]` + the Rust default keep them valid and 8h-defaulted. No
config-file migration is required of users — the key is purely additive and
optional.

**One accepted forward-compat edge:** launcher and server normally ship
co-versioned, but `ACCELERATOR_VISUALISER_BIN` / `visualiser.binary` let a user
pin an arbitrary older server binary. A newer launcher emitting `idle_timeout`
against a pre-0100 server (which has `#[serde(deny_unknown_fields)]` but no
`idle_timeout` field) would make that server fail config parse at boot. This is
accepted behaviour — `deny_unknown_fields` intentionally fails fast on schema
drift, exactly as it already would for any newer key against an older pinned
binary; it is not introduced by this change.

## References

- Original work item: `meta/work/0100-configurable-visualiser-auto-shutdown.md`
- Related research:
  `meta/research/codebase/2026-06-06-0100-configurable-visualiser-auto-shutdown.md`
- Work item review: `meta/reviews/work/0100-configurable-visualiser-auto-shutdown-review-1.md`
- Hard-coded default: `lifecycle::Settings::DEFAULT` in
  `skills/visualisation/visualise/server/src/lifecycle.rs` (symbol anchor — Phase 1
  edits shift the line numbers)
- Idle loop / sentinel: `lifecycle::spawn` (the `idle >= settings.idle_limit_ms`
  comparison) in `lifecycle.rs`, and `tests/lifecycle_owner.rs` (`idle_limit_ms:
  i64::MAX`, to become `config::DISABLED_IDLE_LIMIT_MS`)
- Config precedent (`kanban_columns`):
  `server/src/config.rs:36-37,291-303`, `server/src/server.rs:59`,
  `scripts/write-visualiser-config.sh:147-167,204,233`
- Fail-fast path: `server/src/server.rs:142-182`, `server/src/main.rs:46-55`,
  `tests/config_cli.rs`
- Test seam: `server/tests/lifecycle_idle.rs`, `server/tests/config_contract.rs`
- Docs to update: `skills/config/configure/SKILL.md:546-577`,
  `skills/visualisation/visualise/SKILL.md:39,63`, `README.md:474`,
  `CHANGELOG.md`
