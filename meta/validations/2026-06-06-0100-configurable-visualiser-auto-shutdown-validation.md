---
type: plan-validation
id: "2026-06-06-0100-configurable-visualiser-auto-shutdown-validation"
title: "Validation Report: Configurable Visualiser Auto-Shutdown Implementation Plan"
date: "2026-06-06T21:19:15+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-06-06-0100-configurable-visualiser-auto-shutdown"
target: "plan:2026-06-06-0100-configurable-visualiser-auto-shutdown"
tags: [visualiser, server, configuration, lifecycle]
last_updated: "2026-06-06T21:19:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Configurable Visualiser Auto-Shutdown Implementation Plan

### Implementation Status

✓ Phase 1: Raise the default idle window to 8 hours — Fully implemented
✓ Phase 2: Parse a configurable timeout in Rust, fail-fast, disable token — Fully implemented
✓ Phase 3: Resolve precedence and emit the field in the launcher — Fully implemented
✓ Phase 4: Documentation — Fully implemented

All four phases landed as discrete, well-described commits on top of the
planning-artifacts commit:

- `cbcbd715` Raise visualiser idle auto-shutdown default to 8 hours
- `010dc489` Parse a configurable visualiser idle timeout in Rust
- `c02acf5a` Resolve and emit the visualiser idle timeout in the launcher
- `3e302760` Document the configurable visualiser idle timeout

Working copy is clean.

### Automated Verification Results

✓ Unit tests pass: `mise run test:unit:visualiser` — 404 passed; 0 failed
✓ Integration / shell config-writer tests pass: `mise run test:integration:visualiser` — 31 passed; 0 failed
✓ New cargo integration suites pass: `cargo test --test lifecycle_idle --test lifecycle_owner --test config_cli` — 7 passed (3 suites)
✓ Read-only format/lint checks pass: `mise run check` (shellcheck, bashisms, script format) — all green
✓ `humantime` resolves cleanly in `Cargo.lock`: `cargo tree -i humantime` → `humantime v2.3.0` with no transitive dependencies
✓ No stale "30 minute" idle references: `grep -rn "30 minute" README.md SKILL.md server/src` returns nothing

### Code Review Findings

#### Matches Plan:

- **Phase 1** — `lifecycle.rs:23-26` sets `idle_limit_ms: 8 * 60 * 60 * 1000`
  with the updated doc comment, exactly as specified. `README.md`,
  `skills/visualisation/visualise/SKILL.md` 30-minute wording all replaced.
- **Phase 2** — `config.rs` carries `DEFAULT_IDLE_TIMEOUT = "8h"`,
  `DISABLED_IDLE_LIMIT_MS = i64::MAX`, `MAX_IDLE_LIMIT_MS = sentinel - 1`,
  `idle_timeout: Option<String>` (`#[serde(default)]`), and
  `resolve_idle_limit_ms()` implementing the exact semantics from the plan:
  trim → `never`/`0` disable tokens → `humantime::parse_duration` →
  `is_zero()` disable → saturate-in-u128-then-floor-at-1ms. `ConfigError::InvalidIdleTimeout`
  carries the underlying `source`, matching the `InvalidScanRegex` precedent.
- **Phase 2 wiring** — `AppState.idle_limit_ms` field added (`server.rs:46`),
  resolved alongside `resolve_kanban_columns()?` (`server.rs:64`), threaded into
  `lifecycle::Settings { tick: Settings::DEFAULT.tick, idle_limit_ms: state.idle_limit_ms }`
  in `run` (`server.rs:329`). The `AppStateError::Config` wrapper message was
  neutralised to `"invalid configuration: {0}"` as planned (also corrects the
  pre-existing `EmptyKanbanColumns` misnomer).
- **Resolver unit tests** — all twelve case groups from the plan present in
  `config.rs` (`config.rs:718-812`): absent→8h + drift guard, `30m`, `1h30m`,
  case-insensitive `never`, bare `0`, `0s`/`0ms`, sub-ms floor to 1, overflow
  saturation, whitespace trim, and the `soon`/`00`/`0.0`/`   ` fail-fast set.
- **Lifecycle tests** — `disabled_idle_never_fires`,
  `idle_survives_below_threshold_then_fires` (two-sided boundary), and
  `owner_exit_still_fires_while_idle_disabled` (6c) all present and passing.
  `lifecycle_owner.rs` adopts `config::DISABLED_IDLE_LIMIT_MS` in place of the
  bare `i64::MAX`.
- **Resolve→wire→loop test** — `server.rs:698` asserts `state.idle_limit_ms ==
  30 * 60 * 1000` after building `AppState` from `idle_timeout: "30m"`.
- **Phase 3** — `write-visualiser-config.sh` resolves `env > config > omit`,
  trims (LANG=C), runs the coarse digit-led/disable-token shape guard erroring
  to the terminal on bad shape, and conditionally merges
  `{idle_timeout: $idle_timeout}` only when non-empty. Shell tests cover every
  listed case including env-over-config, empty-env-fallthrough, compound/spaced
  forms, `5 zonks` guard permissiveness, and `soon` terminal rejection.
- **Phase 4** — `configure/SKILL.md` adds the `idle_timeout` table row plus a
  full "Idle auto-shutdown" prose section; `README.md` customisation table adds
  both the env-var and config-key rows; `visualise/SKILL.md` updates the
  user-relayed line and Overrides section; `CHANGELOG.md` has the new `### Added`
  block (inserted before `### Changed`) plus the default-change `### Changed`
  line; `.accelerator/config.md` gains the commented `idle_timeout` example.

#### Deviations from Plan:

- **Boot fail-fast test uses an inline config, not a static fixture.** The plan
  (Phase 2 §7) called for a `tests/fixtures/config.invalid-idle-timeout.json`
  fixture. The implementation instead builds the config inline in `config_cli.rs`
  via `serde_json::json!` inside a `tempfile::tempdir()`
  (`config_cli.rs:11-46`). This is an improvement: it is self-contained (no
  fixture file to keep in sync), and it still asserts `code(1)` **specifically**
  (distinct from the missing-config `code(2)` case in the same file), satisfying
  the plan's "exit code 1, not merely non-zero" requirement.

#### Potential Issues:

- None blocking. The `idle_timeout` field is purely additive and
  `#[serde(default)]`, so older `config.json` files remain valid. The
  forward-compat edge documented in the plan's Migration Notes (a newer launcher
  emitting `idle_timeout` against a user-pinned pre-0100 server with
  `deny_unknown_fields`) remains an accepted, pre-existing class of behaviour.
- The follow-up noted in the plan — switching idle measurement from wall-clock
  `SystemTime` to monotonic `Instant` (whose priority the 16×-wider window
  raises) — remains correctly out of scope here and is worth tracking as a
  separate work item.

### Manual Testing Required:

The plan's automated coverage is thorough; the following remain genuinely
manual (timing/process-lifecycle end-to-end), left unchecked in the plan:

1. End-to-end idle shutdown:
  - [ ] Set `visualiser.idle_timeout: "10s"` in `.accelerator/config.local.md`,
    launch, leave idle, confirm `server-stopped.json` records `idle-timeout`
    shortly after (allowing up-to-one-tick granularity at the 60s production tick).
2. Disable + other triggers still fire:
  - [ ] `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=never <launch>` keeps the server
    alive past the window, but it still dies on owner-process exit and on
    `/accelerator:visualise stop`.
3. Visible fail-fast:
  - [ ] `visualiser.idle_timeout: "soon"` fails the launch with a visible
    terminal error naming the bad value (shell pre-flight guard); `server-info.json`
    never appears.
4. Env-over-config precedence:
  - [ ] `ACCELERATOR_VISUALISER_IDLE_TIMEOUT=2h` with a different config value —
    inspect the generated `config.json`'s `idle_timeout` to confirm the env wins.

### Recommendations:

- Ship as-is. All eight acceptance criteria in the plan's coverage map are
  backed by passing automated tests; the single deviation is a strict
  improvement over the plan.
- Run the four manual end-to-end checks once before release to confirm the
  real-process timing/lifecycle behaviour (the automated suites exercise the
  injectable `Settings` seam and the shell emission, but not a live 60s-tick
  shutdown).
- Open a follow-up work item for the monotonic-clock (`Instant`) migration
  flagged in the plan's Performance Considerations, now that the wider default
  raises its priority.
