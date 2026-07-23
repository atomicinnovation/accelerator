---
type: plan
id: "2026-07-22-0167-config-command-refactoring"
title: "0167 config Command Refactoring Implementation Plan"
date: "2026-07-22T21:45:15+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0167"
parent: "work-item:0167"
derived_from:
  - "codebase-research:2026-07-22-0167-config-command-refactoring-opportunities"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, launcher, store, hexagon, refactoring]
revision: "644b255f3b380739cbdc949a08da0d6688f1c4d0"
repository: "build-system"
last_updated: "2026-07-22T23:29:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167 config Command Refactoring Implementation Plan

## Overview

The 0167 migration shipped the `accelerator config` command family as a clean
three-layer hexagon and consolidated `atomic_write` into the `store` crate. The
big-ticket decisions are sound. This plan addresses the eleven refactoring
findings that surfaced afterwards, almost all of which trace to a single missing
domain seam: `config` exposes value resolution (`ConfigAccess::get`) and
catalogue defaults (`catalogue::default_for`) separately, but nothing combines
them — so the resolve → default → render tail is re-implemented ~10× across
`core/` and inline in `inbound/cli.rs`, the scalar subcommands resolve in the
inbound adapter while the block subcommands delegate to `core/`, and the one
genuine correctness bug (agent empty-value handling) has drifted between the two.

The plan closes that seam first (a richer domain `Resolution` operation), fixes
the drift, then works outward: collapse the duplicated tails, make the scalar
path symmetric with the block path, source domain data from the catalogue,
restore `core`↔`render` purity, unify the identifier validator, consolidate the
adapter read-half and umask handling, and finish with low-risk doc/literal
tidy-ups.

## Current State Analysis

### The removal set is gone, so there is no bash-parity gate

0167 deleted the shell removal set and its suites. The regression contract for
this plan is therefore **the existing Rust CLI test suite and committed goldens
staying byte-identical**, exercised by `mise run test:unit:cli` (which runs
`cargo nextest --workspace --all-features`). Two phases change user-visible
strings deliberately: Phase 1 changes committed-golden output to fix a bug, and
Phase 8 rewords the `ConfigError::Io` Display (an error string not exercised by
any committed golden). Every other phase is behaviour-neutral and its goldens
must not move.

### The missing seam and its consequences

- `ConfigAccess::get(&Key, Option<Level>) -> Resolved`
  (`cli/config/src/service.rs:283-287`) returns the value only, and collapses
  which level won when `level` is `None` (`service.rs:319-335`).
- `catalogue::default_for(&str) -> Option<Value>` (`cli/config/src/catalogue.rs:219-235`)
  returns the default only.
- Nothing combines them, so the shape `Key::parse` → `get` →
  `Found(v) => render_value(v)` / `Absent => default` recurs in `core/dump.rs`
  (`config_get` `:76-86`, `defaulted_row` `:101-115`, `work_row` `:135-163`),
  `core/init.rs` (`resolve_path` `:42-53`), `core/paths.rs` (`resolve_or_default`
  `:87-98`), `core/review.rs` (`resolve` `:507-517`), `core/summary.rs` (`tmp_dir`
  `:89-95`), `core/template.rs` (`scalar` `:99-111`), and inline in
  `inbound/cli.rs` (`resolve_get` `:610-626`, `resolve_path` `:628-649`,
  `resolve_agent` `:752-759`, `resolve_work` `:766-785`, `path_fallback`
  `:724-738`, `work_fallback` `:789-799`).

The behavioural variants of the tail must each be preserved through the
refactor:

- **catalogue-backed default on absence** — `dump`, `init`, `paths`.
- **no-catalogue, explicit-or-empty** — `config get` (`resolve_get`
  `:610-626`) never consults the catalogue: on absence it returns the
  caller-supplied `--default` or empty. It must **not** be routed through
  `effective` (which would inject the catalogue default).
- **explicit caller-supplied default outranks the catalogue** — `config get`
  and `config path` carry a `--default` (`launch/mod.rs:43,57`); the precedence
  is config-value > explicit `--default` > catalogue > empty+warning
  (`path_fallback` `:730-737`). `review::resolve` (`:507-517`) is the internal
  form — an explicit default that bypasses the catalogue entirely.
- **hard-coded literal default** — `summary::tmp_dir` → `".accelerator/tmp"`
  (`:93`), `template::templates_dir` → `".accelerator/templates"` (`:86`). These
  values already live in the catalogue (`catalogue.rs:46,43`), so they are
  latent duplication.
- **empty-collapse** — `template::scalar` treats a rendered-empty value as
  absent (`:107`), and `core/agents.rs:36-44` falls back to `default_agent` when
  the resolved value renders empty.

### The agent drift bug (correctness)

`core/agents.rs:35-44` — a resolved value that renders empty falls back to
`default_agent(name)` (its doc comment at `:4-5` states this matches the bash
reader). `inbound/cli.rs:752-759` (`resolve_agent`) keeps an explicit-empty
value **empty**, falling back only on `Resolved::Absent`. So with
`agents.reviewer: ""` set, `config agents` renders `accelerator:reviewer` while
`config agent reviewer` renders nothing. The prefixed default
`format!("{}{name}", catalogue::AGENT_PREFIX)` exists three times:
`catalogue.rs:229-233` (inside `default_for`), `core/agents.rs:57-59`
(`default_agent`), and `inbound/cli.rs:756`.

### The scalar/block asymmetry

Block handlers are thin: `resolve_agents` (`inbound/cli.rs:761-764`) calls a
`core/` assembler then a `render/` function. Scalar handlers
(`resolve_get`/`resolve_path`/`resolve_agent`/`resolve_work`) have **no `core/`
counterpart** — they run `Key::parse` + `get` + `Resolved` matching + fallback +
`render_value` inline and hand-build a `Rendered` literal. Three subtleties any
extraction must keep: `resolve_agent`'s empty-not-coalesced (the drift above),
`path_fallback`/`work_fallback` running eagerly (they can push a warning even
when the value is `Found`), and the `work.integration` refusal being fail-closed
(`inbound/cli.rs:775-779` returns `Failure::Refusal(bad_integration(...))`).

The clap→domain mapping is contained in `cli/launcher/src/launch/mod.rs`
(`to_action` `:32-162`) and never reaches `core/`; every `core/` assembler takes
only `config::*` ports. So new scalar assemblers slot behind the same
`config_cli::Action` → `core/` seam the block handlers already use.

### Prose in core, computation in render

- `core/review.rs`: `verdict_lines` (`:437-480`) and `revise_verdict`
  (`:482-505`) emit literal output lines; `core_lenses_note` (`:153-183`) builds
  the "Note: built-in work-item lens(es)…" block. `render/review.rs:49-52` just
  pushes them verbatim.
- `core/summary.rs:46-85` assembles the entire human-facing body; `render/summary.rs`
  only wraps the JSON envelope — inverted from every other subcommand.
- `core/dump.rs:148-157` (`work_row`) builds `"{value} (invalid: must be …)"`;
  `core/agents.rs:47` does `name.replace('-', " ")`; `core/paths.rs:61-67`
  builds the blank-path note.
- Conversely, `render/template.rs:163-211` (`unified_diff`, `lcs_lengths`,
  `push_line`) is an LCS computation living in the render layer, and
  `render/review.rs:10-11` hard-codes `DEFAULT_CORE`, duplicating
  `catalogue.rs:131-139`.

### Duplicated data and validators

- `min_lenses "3"/"4"`, `max_lenses "8"`, `max_inline_comments "10"`,
  `dedup_proximity "3"`, three severity `"critical"` keys, `plan_revise_major_count
  "3"`, `work_item_revise_major_count "2"` are typed as literals in
  `core/review.rs` (`:69-72,197,204,225,240,583,591`) while the catalogue already
  declares them (`catalogue.rs:126-152`). **The work-item-mode `min_lenses "3"`
  is the exception** — it is mode-specific and the catalogue models only the
  single value `"4"` (`catalogue.rs:128`), so it stays a launcher literal.
- `WORK_INTEGRATION_VALUES` (`catalogue.rs:106-107`) has no validator; the
  membership check is duplicated at `inbound/cli.rs:775-777` (refusal) and
  `core/dump.rs:148-149` (annotation).
- `validate_skill_name` (`core/context.rs:82-96`) and `validate_name`
  (`core/template.rs:113-127`) are byte-identical `^[a-z0-9][a-z0-9-]*$` bodies
  differing only in the error noun. (`Key::parse` at `key.rs:19-27` has its own,
  different, segment validation — no regex — which is unrelated and stays.)
- `[Level::Team, Level::Personal]` is walked at `core/agents.rs:65`,
  `core/summary.rs:136`, `core/context.rs:27`, and `inbound/cli.rs` (`explain_lines`);
  the `Level`→filename mapping is at `inbound/cli.rs:715-720` (`level_file`),
  `core/summary.rs:49,51` (inline), and `core/dump.rs` (`source_of`). `Level`
  (`cli/config/src/level.rs:8-20`) has only a `Display` impl today.

### Adapters and store

- `store` exposes no read function, but `ensure_contained`'s doc
  (`cli/store/src/lib.rs:97-102`) frames it as the shared read/write containment
  basis. The read half is a named `read_within` in `config-adapters`
  (`store.rs:646-658`) and is open-coded twice in `corpus-adapters`
  (`store.rs:118-128`, `:141-151`).
- Umask: `config-adapters` recomputes `0o666 & !store::current_umask()` **per
  team write** (`store.rs:190-195`); `corpus-adapters` reads it **once at
  construction** into `fresh_mode` (`store.rs:19-28,44`) with a documented race
  rationale. `store::current_umask()` already exists (`lib.rs:239-252`).
- `render_node`/`render_scalar` (`config-adapters/src/store.rs:618-644`) duplicate
  the scalar/sequence stringification of `config::render_value`
  (`cli/config/src/render.rs:9-29`); used only by `lens_field`.

### Cross-cutting facts and non-goals

- Domain crates may import only `std`, `kernel::Error`, and `crate` (`pup.ron`
  `config_domain_imports_only_permitted` `:40-56`). The new `Resolution`
  operation lives in `config` and needs only in-crate `catalogue` — no boundary
  crossing. `store` is imported by the `*-adapters` crates and, from Phase 8, by
the launcher for the single `TEMP_PREFIX` constant — a deliberate const-only edge
(see Phase 8) that removes a silent drift risk; no other launcher → `store` use
is permitted.
- `to_config_error` (`config-adapters/src/store.rs:703-720`) and `to_store_error`
  (`corpus-adapters/src/store.rs:87-100`) are **not** mergeable — `ConfigError`
  lacks `NotWritable`/`CrossFilesystem`, so the config translator is a lossy
  projection while the corpus one is near-identity, and the orphan rule + pup
  forbid a shared `From`. Left as-is.
- `extract_body`/`is_fence` (`config-adapters/src/store.rs:660-690`) are
  deliberate bash-parity and belong to 0174. **Out of scope.**
- The three structurally-identical value trees (`document::Yaml` / `config::Node`
  / `corpus::FrontmatterValue`) and the allowlisted temp-rename exceptions
  (`cache.rs`, `lock.rs`) are the boundary working as designed. **Not touched.**

## Desired End State

`config` owns one resolution operation that combines precedence with the
catalogue default and reports the winning source; the catalogue-backed tails call
it. The subcommands whose semantics genuinely diverge keep their own tail —
`config get` (no catalogue tier), the explicit-`--default` paths, and the
internal `review::resolve` — and the plan is explicit about each.
The scalar and block subcommands are structurally symmetric — resolution lives in
`core/`, `inbound/cli.rs` is arg-mapping plus fail-safe dispatch, and no clap type
reaches `core/`. `core/` returns data views and `render/` produces bytes, in both
directions. Domain data (review defaults, the `work.integration` allow-set, the
identifier rule, the `Level`→filename mapping) has a single home in `config`. The
adapter read-half and umask handling are shared through `store`, and the
low-priority literal/doc drifts are closed.

Verified by: `mise run` exits 0; the committed CLI goldens are byte-identical to
their pre-refactor bytes for every subcommand except the agent-empty case fixed
in Phase 1; `config agents` and `config agent <name>` agree on an explicit-empty
agent value.

## What We're NOT Doing

- Merging `to_config_error` / `to_store_error` (deliberately divergent taxonomies).
- Consolidating `extract_body`/`is_fence` into `document::split` (0174).
- Merging the three value trees or folding `cache.rs`/`lock.rs` into `store`.
- Centralising the per-domain filesystem-error taxonomy into `kernel` (kept
  per-domain by the pup rules).
- Adding a mode dimension to the catalogue for the work-item `min_lenses "3"`.

## Implementation Approach

Eight phases, each an independent PR that is green under `mise run` on its own.
Phases 2–5 consume Phase 1's `effective` operation and land after it; because they
also share edit surface (`dump.rs`, `review.rs`, `summary.rs`, `template.rs`),
land them in the linear order 1→2→3→4→5 — in particular 4 before 5 (Phase 5
relocates the verdict/default prose Phase 4 rewires) and 4 after 3 (Phase 4 edits
the scalar assemblers Phase 3 introduces). **Phases 6, 7 and 8 are independent**
of Phase 1 and of each other, and may land in any order. TDD where it applies: Phase 1's bug fix gets a genuinely failing test
written first; behaviour-neutral phases add characterization/pinning tests shown
green against the current code **before** the implementation moves — committed as
a separate earlier commit within each PR (or with the pre-move green run recorded
in the PR description) so they demonstrably pin pre-refactor behaviour rather than
the refactored result. Every phase that claims "goldens unchanged" proves it by
running the existing golden tests untouched.

The keystone is Phase 1's domain `Resolution`. Its shape:

```rust
// cli/config/src/service.rs (new public API on the ConfigAccess trait)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Personal,
    Team,
    Catalogue,
    Unset,
}

#[derive(Debug, Clone)]
pub struct Resolution {
    value: Value,
    source: Source,
}

impl Resolution {
    #[must_use]
    pub fn rendered(&self) -> String { render_value(&self.value) }
    #[must_use]
    pub const fn source(&self) -> Source { self.source }
    #[must_use]
    pub const fn is_from_config(&self) -> bool {
        matches!(self.source, Source::Personal | Source::Team)
    }
    #[must_use]
    pub fn configured_value(&self) -> Option<String> {
        self.is_from_config().then(|| self.rendered())
    }
}
```

`effective` resolves precedence, then the catalogue default on absence, and
reports which side won. `effective_nonempty` additionally treats a
config-present-but-rendered-empty value as absent (the empty-collapse policy the
agent and template paths need). The trait doc at the definition site states this
one-line contrast (catalogue-default vs catalogue-default-plus-empty-collapse) so
the choice is self-documenting at each call site. Both read personal **and** team eagerly when
`level` is `None`, preserving `get`'s fail-loud-on-either-level posture while
attributing the winner:

```rust
fn effective(
    &self,
    key: &Key,
    level: Option<Level>,
) -> Result<Resolution, ConfigError>;

fn effective_nonempty(
    &self,
    key: &Key,
    level: Option<Level>,
) -> Result<Resolution, ConfigError>;
```

---

## Phase 1: Domain `Resolution` operation and the agent drift fix

### Overview

Add the combined resolution operation to the `config` domain and route both
agent paths through it, fixing the one correctness bug and establishing the seam
every later phase depends on. This is the only phase that changes observable
output.

### Changes Required

#### 1. The `Resolution` operation

**File**: `cli/config/src/service.rs`
**Changes**: add the `Source` enum, the `Resolution` struct, and `effective` /
`effective_nonempty` as **provided default methods** on the `ConfigAccess` trait
(`:274-302`) — their bodies are expressed purely in terms of `self.get` plus
in-crate `catalogue`/`render_value`, so the precedence-plus-catalogue derivation
lives in one place and every implementor inherits it (a future double could still
override a default method, so single-sourcing is by inheritance, not enforcement);
`ConfigService` inherits them unchanged. A doc comment on `Resolution` notes its
distinction from the existing `Resolved` enum (`get`'s return): `Resolved` is raw
presence, `Resolution` is presence-plus-default-plus-source. Each new
`Result`-returning public item (`effective`/`effective_nonempty` here,
`validate_identifier` in Phase 6, `store::read_within` in Phase 7) carries a
`# Errors` rustdoc section matching the adjacent `get`/`set` docs — clippy pedantic
(`missing_errors_doc`, `warnings = deny`) fails `cli:check` otherwise. The full-stack branch reads both
levels eagerly (matching today's `get(None)` at `:326-334`) and records the
winner:

```rust
fn effective(
    &self,
    key: &Key,
    level: Option<Level>,
) -> Result<Resolution, ConfigError> {
    let found = match level {
        Some(one) => match self.get(key, Some(one))? {
            Resolved::Found(value) => Some((source_of(one), value)),
            Resolved::Absent => None,
        },
        None => {
            let personal = self.get(key, Some(Level::Personal))?;
            let team = self.get(key, Some(Level::Team))?;
            match (personal, team) {
                (Resolved::Found(value), _) => Some((Source::Personal, value)),
                (Resolved::Absent, Resolved::Found(value)) => {
                    Some((Source::Team, value))
                }
                (Resolved::Absent, Resolved::Absent) => None,
            }
        }
    };
    Ok(match found {
        Some((source, value)) => Resolution { value, source },
        None => default_resolution(key),
    })
}
```

`effective_nonempty` wraps `effective`: when the result `is_from_config()` and
`rendered().is_empty()`, it replaces the result with `default_resolution(key)`.
`default_resolution` calls `catalogue::default_for(&key.to_string())`, mapping
`Some(v) => { value: v, Catalogue }` and `None => { value: Scalar::Null, Unset }`
(a new `use crate::catalogue;` in `service.rs`). `source_of(Level)` maps
`Team → Source::Team`, `Personal → Source::Personal`. Note that
`render_value(Scalar::Null)` is `""`, so an `Unset` resolution renders
identically to a config empty string; absence is authoritative only via
`source()`/`is_from_config()`, never `rendered().is_empty()`. Consumers that need
the config-supplied value (and must treat a catalogue default or `Unset` as
absent) use `configured_value()`, which returns `Some(rendered)` only for a
`Personal`/`Team` source and `None` otherwise — so the empty-vs-absent
distinction cannot be lost to a naive `.is_empty()` check.

#### 2. Route both agent paths through the domain

**File**: `cli/launcher/src/config_command/core/agents.rs`
**Changes**: replace the inline resolve+empty-check+`default_agent` block
(`:35-44`) with a single `config.effective_nonempty(&key, None)?.rendered()`.
Delete `default_agent` (`:57-59`).

**File**: `cli/launcher/src/config_command/inbound/cli.rs`
**Changes**: `resolve_agent` (`:752-759`) resolves
`config.effective_nonempty(&Key::parse(&format!("agents.{name}"))?, None)?`. It
takes `configured_value()` when present, otherwise falls back to
`format!("{}{name}", catalogue::AGENT_PREFIX)`. This keeps the
empty-collapse fix for known agents (an explicit-empty value coalesces to the
prefixed default) **and** preserves today's unconditional prefixed fallback for
**any** agent name — including names outside `AGENT_KEYS`, for which
`catalogue::default_for` returns `None` (so the resolution is `Source::Unset` and
would otherwise render empty). The prefixed default therefore lives in exactly two
places, both keyed off the single `catalogue::AGENT_PREFIX` constant:
`catalogue::default_for` (known keys) and this fallback (arbitrary names).

### Success Criteria

#### Automated Verification

- [x] Failing-test-first: a `config` unit test asserting
      `effective(agents.reviewer)` on `agents.reviewer: ""` returns
      `Source::Personal` with empty rendered value, while
      `effective_nonempty(agents.reviewer)` returns `Source::Catalogue` /
      `accelerator:reviewer`
- [x] A launcher test asserts `config agents` and `config agent reviewer` render
      the **same** value for `agents.reviewer: ""` (the drift is gone)
- [x] A launcher test asserts `config agent <name-not-in-AGENT_KEYS>` on an unset
      key still renders `accelerator:<name>` (the unconditional prefixed fallback
      is preserved, not regressed to empty)
- [x] `effective`/`effective_nonempty` unit tests cover: personal wins over team;
      team wins on personal-absent; catalogue on both-absent; `Unset` on an
      unknown key; fail-loud when the non-winning level is malformed
- [x] A unit test asserts `source()` is authoritative independent of the rendered
      value: `effective(agents.reviewer)` on `agents.reviewer: ""` reports
      `Source::Personal` with an empty rendered value, while an absent key reports
      `Source::Catalogue`/`Unset`
- [x] `mise run test:unit:cli` passes (all `--all-features` suites)
- [x] `mise run cli:check` and `mise run` exit 0

#### Manual Verification

- [x] No existing committed golden moves: the block `config agents` path already
      coalesces empty to the prefixed default, and the scalar `config agent
      <name>` fix is proven by the new agreement test (there is no `agent.golden`).
      Confirm every committed golden is byte-identical via the diff of the
      test-fixtures tree

---

## Phase 2: Collapse the `core/` resolve+default tails onto `effective`

### Overview

Replace the catalogue-backed and literal-default tails in `core/` with
`effective`, and derive `dump`'s source attribution from `Resolution.source`
rather than repeated level probes. Adds the domain `Level::filename()`.
Behaviour-neutral; goldens unchanged.

### Changes Required

#### 1. Domain `Level::filename()`

**File**: `cli/config/src/level.rs`
**Changes**: add an inherent method beside the `Display` impl:

```rust
impl Level {
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            Self::Team => ".accelerator/config.md",
            Self::Personal => ".accelerator/config.local.md",
        }
    }
}
```

#### 2. Collapse the tails

**File**: `cli/launcher/src/config_command/core/dump.rs`
**Changes**: `defaulted_row`/`work_row` (`:101-163`) take the value from
`effective`; `config_get` (`:76-86`) keeps its raw present/absent contract — it
also feeds `optional_row`/`extra_row`, whose "not set" signal must survive, so it
stays on raw `get()`/`configured_value()` and is **not** routed through
`effective`. `source_of` (`:88-99`) reads `effective(key, None)?.source()` and
maps `Personal → Local`, `Team → Team`, `Catalogue|Unset → Default`, removing the
two `config_get(..., Some(level))` probes and the `Level`→filename inline strings
(adopting `Level::filename()`).

**File**: `cli/launcher/src/config_command/core/init.rs`
**Changes**: `resolve_path` (`:42-53`) → `effective(&parsed, None)?.rendered()`.

**File**: `cli/launcher/src/config_command/core/paths.rs`
**Changes**: `resolve_or_default` (`:87-98`) → `effective`.

**File**: `cli/launcher/src/config_command/core/summary.rs`
**Changes**: `tmp_dir` (`:89-95`) → `effective(paths.tmp)?.rendered()`, dropping
the `".accelerator/tmp"` literal (now catalogue-sourced). Adopt `Level::filename()`
where the summary body names config files (`:49,51`) — the strings are identical,
so no golden moves. The touched `tmp_dir` consumer is display-only; any future
consumer that treats a resolved path destructively (delete/cleanup) must reject an
empty/`Unset` resolution rather than defaulting to the current directory, so the
catalogue-drift guard is enforced at the dangerous call site, not only by the
Phase 2 catalogue test.

**File**: `cli/launcher/src/config_command/core/template.rs`
**Changes**: `templates_dir` (`:84-87`) → `effective_nonempty(paths.templates)`,
dropping the `".accelerator/templates"` literal — `effective_nonempty`, **not**
`effective`, because `templates_dir` delegates to `scalar` today, which collapses
a config-present-but-empty value to the default; `effective` would render `""`, so
`paths.templates: ""` must still resolve to `.accelerator/templates` (its
catalogue default). `scalar` (`:99-111`) also uses `effective_nonempty` but must
preserve its `Option<String>` contract: `None` means "no configured override" and
drives the override/plugin fall-through in `resolve_template`. `templates.<name>`
keys have **no** catalogue default, so `effective_nonempty` yields `Source::Unset`
(rendered `""`), not `None`; `scalar` returns
`effective_nonempty(...).configured_value()` so a non-config (`Unset`/`Catalogue`)
resolution becomes `None` and the fall-through is unchanged.

### Success Criteria

#### Automated Verification

- [x] `Level::filename()` unit test (both variants)
- [x] A `config` test asserts `catalogue::default_for` for `paths.tmp` and
      `paths.templates` is present and non-empty, so a future catalogue drift that
      emptied these path defaults fails loudly rather than silently resolving to
      the project root
- [x] All existing `dump`/`init`/`paths`/`summary`/`template` golden tests pass
      unchanged
- [x] A `dump` test asserts source attribution (Local/Team/Default) is unchanged
      across a fixture with personal, team, and default-only keys (the committed
      `dump.golden` — team `min_lenses`, local `max_lenses`, default rest — is the
      byte-exact guarantee)
- [x] A test asserts `paths.templates: ""` in config still resolves
      `templates_dir` to `.accelerator/templates` (the empty-collapse is preserved
      by `effective_nonempty`)
- [x] A test sets `templates.demo: ""` in config and asserts `config template demo`
      still resolves to the plugin default (the `configured_value()` → `None`
      fall-through is preserved)
- [x] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

#### Manual Verification

- [x] No committed golden bytes change in this phase

---

## Phase 3: Scalar subcommands into `core/`

### Overview

Give `get`/`path`/`agent`/`work` `core/` view-assemblers symmetric with the
block subcommands, reducing `inbound/cli.rs` to arg-mapping plus `finish`
dispatch. Behaviour-neutral (the agent-empty fix already landed in Phase 1).

### Changes Required

#### 1. New scalar assemblers, co-located with their block siblings

**Files**: `cli/launcher/src/config_command/core/agents.rs` (add the `config
agent` assembler beside the existing `config agents` block assembler),
`core/paths.rs` (add `config path` beside `config paths`), and new `core/get.rs`
and `core/work.rs` for the subcommands with no block sibling — all registered in
`core/mod.rs`.
**Changes**: move the resolution logic out of `inbound/cli.rs` — `resolve_get`
(`:610-626`), `resolve_path` (`:628-649`), `resolve_agent` (`:752-759`),
`resolve_work` (`:766-785`), and their helpers `path_fallback` (`:724-738`),
`work_fallback` (`:789-799`), `legacy_alias_warning` (`:653-674`),
`unknown_path_key_warning` (`:740-750`), `explain_lines` (`:678-713`),
`bad_integration` (`:805-814`). Each assembler takes only the ports it uses
(`&dyn ConfigAccess`, plus the level-reading port where a raw per-level walk
survives) — matching the block assemblers, not the composite `ConfigStack` — and
returns a view carrying the rendered value plus accumulated warnings. `level_file`
(`:715-720`) is replaced by `Level::filename()`. Each scalar assembler is its
module's `resolve` function (`get::resolve`, `work::resolve`, `agents::resolve`,
`paths::resolve`), sitting beside the block `assemble`/`configured` where one
exists so the scalar/block pair is obvious when navigating `core/`.

Resolution is **not** uniformly `effective`; each subcommand keeps its own tail
semantics (see the behavioural-variants list in Current State):

- **`get`** stays on raw `get()` and returns the config value when present,
  otherwise the caller-supplied `--default` or empty — **never** the catalogue
  default. It is not routed through `effective`.
- **`path`/`work`** use `effective` only when the result `is_from_config()`; on a
  non-config result they apply the explicit `--default` first (preserving
  explicit-default-over-catalogue precedence) and fall to the catalogue value or
  the empty+warning path only when no `--default` is supplied.
- **`agent`** uses `effective_nonempty` (the Phase 1 empty-collapse).

Four subtleties are preserved and each pinned by a test:

- `path_fallback`/`work_fallback` run **before** the value is known and may push
  a warning even when the value resolves from config.
- an explicit `--default` on `get`/`path` outranks the catalogue on an absent
  key, and each subcommand's empty-`--default` predicate is preserved verbatim:
  `get` uses `--default` as-is (an empty `--default` yields empty), while `path`
  ignores an empty `--default` and falls through to the catalogue.
- the `work.integration` refusal stays fail-closed: the assembler returns
  `ConfigError::Invalid` (via `bad_integration`), which `From<ConfigError>`
  (`inbound/cli.rs:412-419`) classifies as `Failure::Refusal`, never degraded by
  `finish`.
- `resolve_get` parses the raw key verbatim (no section prefix); the others
  prefix `paths.`/`agents.`/`work.`.

`--explain` provenance keeps its per-level `get(Some(..))` probe: `explain_lines`
reports the set/not-set status of **both** levels, which `Resolution.source()`
(winner only) cannot reconstruct. `source()` attributes the winning level; the
per-level presence still comes from the probe.

#### 2. Thin the inbound adapter

**File**: `cli/launcher/src/config_command/inbound/cli.rs`
**Changes**: the scalar handlers become two-liners (`core/` assembler → `render/`
or a direct `Rendered`), matching the block-handler shape at `:761-764`. The
fail-safe machinery (`finish` `:428-447`, `Degrade`, `From<ConfigError>`) is
untouched. No clap type is introduced into the co-located assemblers
(`core/get.rs`, `core/work.rs`, and the `agent`/`path` additions to
`core/agents.rs`/`core/paths.rs`).

### Success Criteria

#### Automated Verification

- [x] Characterization test (added green against the current code, before the
      move): a `core/` scalar test pinning the eager-warning subtlety (a warning
      is emitted on a `Found` value where `path_fallback` would warn)
- [x] A test asserts `config get <catalogue-backed key>` on an unset key stays
      **empty** (the catalogue default is not injected)
- [x] A test asserts `config path <catalogue-backed key> --default X` on an unset
      key returns `X` (explicit default outranks the catalogue)
- [x] A test asserts `config work integration` with a bad enum exits non-zero
      with empty stdout (fail-closed), and `--fail-safe` does not degrade it
- [x] `--explain` provenance for a key set in `config.md` and overridden in
      `config.local.md` names both levels and attributes the win to personal
- [x] The golden contract covers **stdout only**, and the existing stderr checks
      are substring/non-empty, not byte-exact; each relocated stderr-emitting
      helper therefore gets a dedicated **byte-exact** test, exhaustive over every
      warning path — the migration-0004 `legacy_alias_warning` (set a legacy alias
      key; assert the exact nudge), `unknown_path_key_warning`/
      `unknown_work_key_warning`, the `--default`-present-suppresses-unknown-key
      interaction, and the **order** of the legacy-alias → fallback → `--explain`
      sequence on the multi-warning path
- [x] A test pins `config path <key> --default ""` falling through to the
      catalogue while `config get <key> --default ""` returns empty (each
      subcommand's empty-`--default` predicate is preserved verbatim)
- [x] All existing scalar golden/behaviour tests pass unchanged
- [x] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

> Deviation: `path`/`work` were relocated verbatim on `config.get()` + eager
> `path_fallback`/`work_fallback` rather than routed through `effective`. Routing
> through `effective` + `configured_value()` would suppress the eager unknown-key
> warning on a config-set value, breaking the eager-warning subtlety this phase is
> required to pin. `agent` uses `effective_nonempty` (Phase 1); `get` keeps raw
> `get()` (no catalogue). Behaviour is byte-identical; the single-seam goal is
> served by Phase 2's `core/` collapse, not by re-resolving the scalar tails.

#### Manual Verification

- [x] `inbound/cli.rs` contains no `Key::parse`/`config.get`/`Resolved` matching
      for the scalar subcommands — resolution lives in `core/` (the sole remaining
      `Key::parse` is `run_set`, the write path)

---

## Phase 4: Catalogue-sourced defaults and the `work.integration` validator

### Overview

Read review defaults and the `## Review Configuration` core-lens default from
the catalogue, and give `work.integration` a single validator. Behaviour-neutral.

### Changes Required

#### 1. Review defaults from the catalogue

**File**: `cli/launcher/src/config_command/core/review.rs`
**Changes**: replace the literal defaults with `catalogue::default_for` lookups —
`max_lenses "8"` (`:72,78,92`), `max_inline_comments "10"` (`:197`),
`dedup_proximity "3"` (`:204`), `plan_revise_major_count "3"` (`:225`),
`work_item_revise_major_count "2"` (`:240`), and the three severity `"critical"`
keys (`:583,591,632`). The **work-item-mode `min_lenses "3"`** (`:69`) stays a
launcher literal — the catalogue models only the single value `"4"`
(`catalogue.rs:128`) and adding a mode dimension is out of scope; a short comment
is not added (the literal is self-evident), but the plan records the reason here.
`review::resolve` (`:507-517`) stays as a thin internal helper rather than being
folded onto `effective`: its `default` argument is reused verbatim in
warning/display text, so it needs the raw default string in hand. This is a
deliberate exception to the single-seam goal, noted here and in Desired End State.

#### 2. `DEFAULT_CORE` from the catalogue

**File**: `cli/launcher/src/config_command/render/review.rs`
**Changes**: replace the `DEFAULT_CORE` const (`:10-11`) with a value derived
from `catalogue::default_for("review.core_lenses")` by extracting the sequence's
scalar items and joining them with `", "` — **not** via `config::render_value`,
which renders sequences in bracketed `[a, b]` form and would move the golden. The
render layer no longer re-declares domain data.

#### 3. `work.integration` validator

**File**: `cli/config/src/catalogue.rs`
**Changes**: add `#[must_use] pub fn is_valid_work_integration(value: &str) ->
bool` beside `WORK_INTEGRATION_VALUES` (`:106-107`): empty is permitted, else
membership in the slice.

**File**: `cli/launcher/src/config_command/core/work.rs`,
`cli/launcher/src/config_command/core/dump.rs`
**Changes**: the `work.integration` refusal check (moved to `core/work.rs` in
Phase 3) and `work_row`'s annotation (`dump.rs:148-149`) both call
`catalogue::is_valid_work_integration`. This change edits the `core/work.rs`
introduced in Phase 3, so Phase 4 lands after Phase 3.

### Success Criteria

#### Automated Verification

- [x] `catalogue::is_valid_work_integration` unit test (empty ok; each allowed
      value ok; an unknown value rejected)
- [x] A test asserts each catalogue-sourced review default equals the value the
      catalogue declares (so a catalogue change propagates to the command)
- [x] A new review fixture sets `review.core_lenses` (a non-default value) with a
      committed golden pinning the exact `Core lenses` line and the
      `(default: architecture, code-quality, test-coverage, correctness)` bytes —
      the baseline goldens leave this branch unexercised, so it must be added here
      (not deferred to Phase 5) to actually verify the `DEFAULT_CORE` rewrite
- [x] A dump fixture with an invalid `work.integration` value and a committed
      golden pin the exact `"<value> (invalid: must be …)"` annotation cell, so the
      validator swap here and the Phase 5 string relocation are byte-guarded (the
      baseline dump fixture uses a valid integration, leaving this branch
      unexercised)
- [x] All existing review/dump golden tests pass unchanged
- [x] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

> Note: the "committed golden" for the two newly-exercised branches is realised as
> byte-exact substring assertions (`review-core-lenses` fixture for the `Core
> lenses` default line; the existing `bad-integration` fixture for the dump
> invalid-annotation cell) rather than full `.golden` files — the assertions pin
> exactly the bytes the DEFAULT_CORE rewrite and validator swap touch.

#### Manual Verification

- [x] No review/dump golden bytes change

---

## Phase 5: Restore `core`↔`render` purity

### Overview

Move output-prose assembly out of `core/` into `render/`, move the LCS diff
computation out of `render/` into `core/`, and normalise the two divergent
"unavailable" idioms into one helper. Behaviour-neutral — the same bytes, emitted
from the correct layer. Because the review verdict/revise prose branches are not
exercised by the baseline fixtures, this phase first adds goldens for them (see
Success Criteria) so the relocation is guarded byte-for-byte.

### Changes Required

#### 1. Prose out of core

**File**: `cli/launcher/src/config_command/core/review.rs`,
`cli/launcher/src/config_command/render/review.rs`
**Changes**: `core/review.rs` returns structured verdict data (severity, count,
mode, the disabled/filtered-lens sets) instead of the literal
`"- **Verdict**: …"` strings built in `verdict_lines`/`revise_verdict` (`:437-505`)
and the `core_lenses_note` block (`:153-183`). `render/review.rs` formats those
strings from the view.

**File**: `cli/launcher/src/config_command/core/summary.rs`,
`cli/launcher/src/config_command/render/summary.rs`
**Changes**: invert the current arrangement — `core/summary.rs` returns a summary
view (which config files are present, `configured_sections`, initialised flag,
project-context flag); `render/summary.rs` assembles the human-facing body
(`:46-85`) and the JSON envelope.

**File**: `cli/launcher/src/config_command/core/dump.rs`,
`cli/launcher/src/config_command/core/agents.rs`,
`cli/launcher/src/config_command/core/paths.rs`
**Changes**: move the `"{value} (invalid: must be …)"` string (`dump.rs:148-157`),
the `name.replace('-', " ")` display transform (`agents.rs:47`), and the
blank-path note (`paths.rs:61-67`) into the corresponding `render/` modules; the
core views carry the raw data (the raw value + a validity flag; the raw agent
name; the fact that a default was substituted for a blank path).

#### 2. LCS diff into core

**File**: `cli/launcher/src/config_command/core/template.rs`,
`cli/launcher/src/config_command/render/template.rs`
**Changes**: move `unified_diff`, `lcs_lengths`, `push_line` (`render/template.rs:163-211`)
and the content-equality decision (`:114`) into `core/template.rs`; the template
diff view carries the computed diff lines (or an "identical" marker), and
`render/template.rs` only emits them.

#### 3. One `unavailable` helper

**File**: `cli/launcher/src/config_command/render/mod.rs`
**Changes**: add `#[must_use] pub fn unavailable(header: &str) -> Rendered`
producing `format!("{header}\n")`. The six `render_unavailable()` functions
(`agents.rs:31-34`, `dump.rs:41-44`, `paths.rs:35-38`, `review.rs:83-86`,
`template.rs:58-61`, and summary's equivalent) call it with their exact header;
`context.rs`/`instructions.rs` keep their bare consts but the handler's
newline-adding path (`inbound/cli.rs` `emit_sections`) is left unchanged so the
multi-block join semantics do not move.

### Success Criteria

#### Automated Verification

- [x] Characterization goldens (added green before the move): **before**
      relocating the review prose, add review fixtures/goldens covering **every**
      currently-unexercised branch of
      `verdict_lines`/`revise_verdict`/`core_lenses_note` — the Pr-mode
      `REQUEST_CHANGES disabled` line (`pr_request_changes_severity: none`), the
      plan/work-item `severity-based REVISE disabled` line (`*_revise_severity:
      none`), a custom non-`critical` severity in **both** the Pr and revise
      branches, and a non-default `*_revise_major_count` — so the whole branch set,
      not three examples, is golden-pinned prior to the move
- [x] All existing review/summary/dump/agents/paths/template golden tests pass
      **byte-identical** — this is the phase's core guarantee
- [x] A `render/template` test asserts the unified diff for a known
      default-vs-user pair matches the committed golden (as a byte-exact
      `core/template::diff` unit test — the LCS now lives there)
- [x] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

#### Manual Verification

- [x] `core/` modules contain no user-facing literal output lines for
      review/summary; `render/template.rs` contains no LCS computation (only
      backticked doc comments name the verdict tokens)

---

## Phase 6: Identifier validator into the `config` domain

### Overview

Unify the two byte-identical name validators into one domain helper. Independent
of Phases 1–5.

### Changes Required

**File**: `cli/config/src/lib.rs` (new small module, e.g. `identifier.rs`)
**Changes**: add `pub fn validate_identifier(kind: &str, name: &str) ->
Result<(), ConfigError>` enforcing `^[a-z0-9][a-z0-9-]*$` and returning
`ConfigError::Invalid { detail: format!("invalid {kind} '{name}'") }` on failure,
re-exported from `lib.rs`.

**File**: `cli/launcher/src/config_command/core/context.rs`,
`cli/launcher/src/config_command/core/template.rs`
**Changes**: `validate_skill_name` (`context.rs:82-96`) and `validate_name`
(`template.rs:113-127`) are deleted; callers use
`config::validate_identifier("skill name", skill)` and
`config::validate_identifier("template name", name)`. The error strings are
byte-identical to today's, so no error-path golden moves.

### Success Criteria

#### Automated Verification

- [x] `config::validate_identifier` unit tests (valid; leading hyphen; uppercase;
      empty; embedded space) with the exact error string per `kind`
- [x] Existing context/template invalid-name tests pass unchanged
- [x] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

#### Manual Verification

- [x] `Key::parse`'s own segment validation (`key.rs:19-27`) is untouched

---

## Phase 7: Adapters/`store` consolidation

### Overview

Give `store` the read-half companion its `ensure_contained` doc anticipates,
adopt it in both adapters, align config-adapters onto corpus's umask-read-once,
and replace `render_node`/`render_scalar` with `config::render_value`.
Independent of Phases 1–6.

### Changes Required

#### 1. `store::read_within`

**File**: `cli/store/src/lib.rs`
**Changes**: add
`pub fn read_within(path: &Path, bounds: &WriteBounds<'_>) -> Result<Option<Vec<u8>>, WriteError>`
— `ensure_contained` then a `NotFound`-tolerant read, returning `Ok(None)` on
absence. The `ensure_contained` doc (`:97-102`) already frames the shared
contract; this is its read half. The write-centric `WriteError`/`WriteBounds`
names are retained intentionally: they are `store`'s shared containment types,
now spanning both halves of the read/write contract, and renaming them is out of
scope.

**File**: `cli/config-adapters/src/store.rs`
**Changes**: `read_within` (`:646-658`) delegates to `store::read_within(...)?`,
mapping the `WriteError` through `to_config_error`; it decodes with
`String::from_utf8(...)` (mapping the error to `ConfigError::Io`), **not**
`from_utf8_lossy`, preserving today's `read_to_string` fail-on-invalid-UTF-8
behaviour for its two callers (`config_body` `:205`, `read_skill_file` `:232`).

**File**: `cli/corpus-adapters/src/store.rs`
**Changes**: in `append_record` (`:118-128`) and `remove_by_key` (`:141-151`) the
containment check and the tolerant read straddle the `create_dir_all` and the
lock, so they cannot be fused into one `store::read_within` call. Keep a
standalone `store::ensure_contained` **before** `create_dir_all` (so a symlinked
component cannot redirect the built tree) and use `store::read_within` only for
the **post-lock** read (its internal containment re-check is a harmless
redundancy). The lock acquisition and `create_dir_all` stay where they are,
between the two. `append_record` reads bytes today (`fs::read`) so it consumes
`read_within`'s `Vec<u8>` directly; `remove_by_key` reads with `fs::read_to_string`
today (fail-loud on invalid UTF-8), so it decodes the returned bytes with
`String::from_utf8` (**not** `from_utf8_lossy`) — a lossy decode would silently
mangle a tampered or partially-written JSONL file and then atomically rewrite it,
destroying the original bytes.

#### 2. Umask read-once in config-adapters

**File**: `cli/config-adapters/src/store.rs`
**Changes**: resolve `0o666 & !store::current_umask()` **once at construction**
into a `fresh_mode` field, mirroring `corpus-adapters` (`store.rs:19-28,44`), and
use it in `WriteConfigLevel::write` (`:190-195`) instead of recomputing per write.
This narrows the per-write umask race to a single construction-time read (matching
corpus-adapters); the residual process-global set/restore window inside
`store::current_umask` remains and is out of scope.

#### 3. `render_node` → `config::render_value`

**File**: `cli/config-adapters/src/store.rs`
**Changes**: `lens_field` (`:611-616`) projects the `config::Node` to a
`config::Value` and calls `config::render_value` (`cli/config/src/render.rs:9-29`);
`render_node`/`render_scalar` (`:618-644`) are deleted. Both are in the config
subdomain, so this crosses no pup boundary.

### Success Criteria

#### Automated Verification

- [ ] `store::read_within` unit tests: reads a contained file; returns `None` on
      absence; refuses a symlink-escaping path (mirrors `atomic_write`'s refusal)
- [ ] A corpus-adapters test asserts `remove_by_key` on a file with invalid UTF-8
      returns an error and leaves the file byte-identical (no lossy rewrite)
- [ ] A config-adapters test asserts `config_body`/`read_skill_file` on a file
      with invalid UTF-8 returns an error (fail-loud), mirroring the corpus test —
      the `String::from_utf8` decode must not degrade to lossy
- [ ] A corpus-adapters test asserts `append_record`/`remove_by_key` refuse a path
      whose **intermediate** directory component is a symlink escaping the root,
      and that no directory is created outside the root — pinning that
      `ensure_contained` runs before `create_dir_all`
- [ ] Existing config-adapters and corpus-adapters store tests pass unchanged
- [ ] The existing `PreserveOr` mode tests stay green (the resulting on-disk mode
      is unchanged by moving the umask read to construction)
- [ ] `lens_field` output is unchanged by the `render_value` swap (existing lens
      tests pass)
- [ ] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

#### Manual Verification

- [ ] `cargo tree -p store` shows no new dependency (still std, tempfile, rustix)
- [ ] The `store` duplication check (`tasks/lint/store_duplication.py`) still
      passes — `read_within` is a read, not a temp-then-rename
- [ ] The umask read-once change is structural: `fresh_mode` is a field resolved
      in the constructor and `WriteConfigLevel::write` reads no umask per call
      (confirmed by inspection — the `PreserveOr` tests cannot observe read count).
      If per-call read-count coverage is later wanted, inject the umask via a
      counted port and assert it is read exactly once across N writes

---

## Phase 8: Low-risk tidy-ups

### Overview

Close the remaining doc/literal drifts. Independent of every other phase.

### Changes Required

#### 1. Strip the exit-code fact from `kernel::Error::Refusal`

**File**: `cli/kernel/src/lib.rs`
**Changes**: the `Refusal` doc (`:15-16`) currently states "Mapped to exit code 2
at the boundary." — an assertion about `main.rs`, the staleness class the project
bans. Reword to describe only what the variant *is* (a subcommand-scoped,
caller-actionable refusal whose meaning is defined per subcommand), with no
exit-code claim.

#### 2. `cache.rs` uses `store::TEMP_PREFIX`

**File**: `cli/launcher/src/launch/outbound/resolve/cache.rs`,
`cli/launcher/Cargo.toml`
**Changes**: `cache.rs:94-95` builds its temp names from `store::TEMP_PREFIX`
(`cli/store/src/lib.rs:19-22`) instead of the hard-coded `".tmp-"`. This adds a
direct `store` dependency to the launcher (accepted: one edge to remove a silent
drift risk between the two temp-prefix definitions). `cache.rs` stays outside
`store` — only the constant is shared, not the publication semantics — and the
Cross-cutting facts note records this const-only launcher → `store` edge as the
one permitted exception to the store-boundary rule.

#### 3. De-duplicate the render tests

**File**: `cli/config-adapters/src/render.rs`
**Changes**: the `renders_scalar_kinds` / `renders_a_sequence_in_bracketed_form`
tests (`:30-51`) are byte-identical to `config/src/render.rs:38-59`. Since Phase 7
routes `lens_field` through `config::render_value`, the config-adapters copies no
longer test adapter-local code — delete them, leaving the canonical pair in
`config/src/render.rs`.

#### 4. Reword `ConfigError::Io`

**File**: `cli/config/src/error.rs`
**Changes**: the `Io` Display (`:90-92`) hard-codes "I/O error on config file
'{path}'", but its ports now surface skill files, lens dirs, template overrides,
and `init`'s directories. Reword to a file-kind-neutral form
(`"I/O error on '{path}': {detail}"`) so a wide failure names the offending file
accurately. This is the phase's only user-visible string change — any test
pinning the old wording is updated in the same commit with the reason in the
message.

### Success Criteria

#### Automated Verification

- [ ] `kernel` doc-test/build passes with the reworded `Refusal` doc; no exit-code
      string remains in it
- [ ] A launcher test asserts `cache.rs` temp names begin with `store::TEMP_PREFIX`
- [ ] The canonical render tests in `config/src/render.rs` still pass; the
      config-adapters duplicates are gone
- [ ] `ConfigError::Io` Display test updated and passing
- [ ] A black-box test triggers an I/O error on a non-config path (e.g. a skill
      context file that is itself a directory) and asserts the emitted message
      uses the new file-kind-neutral `I/O error on '{path}'` form
- [ ] `mise run test:unit:cli`, `mise run cli:check`, `mise run` exit 0

#### Manual Verification

- [ ] `grep -rn '"\.tmp-"' cli/launcher/src` returns nothing (the literal is gone
      from `cache.rs`)
- [ ] The `ConfigError::Io` message reads sensibly for a non-config path (e.g. a
      skill file or a template override)
- [ ] `grep -rn "I/O error on config file" skills hooks scripts` (the pre-reword
      substring) returns nothing — no skill/hook/script parses the old wording

---

## Testing Strategy

### Unit Tests

- `config` crate: `effective`/`effective_nonempty` precedence, catalogue
  fallback, empty-collapse, source attribution, fail-loud-on-either-level;
  `Level::filename()`; `validate_identifier`; `is_valid_work_integration`;
  `store::read_within`.
- Launcher `core/`: scalar assemblers (eager warnings, `config get` no-catalogue
  on absence, explicit-`--default`-over-catalogue, `work.integration` refusal,
  `--explain` provenance); source attribution in `dump`; the agents-vs-agent
  empty-value agreement.

### Integration Tests

- The existing CLI golden suite (`mise run test:unit:cli`, `--all-features`) is
  the primary regression gate for every behaviour-neutral phase: goldens must be
  byte-identical except the Phase 1 agent-empty case.

### Manual Testing Steps

1. Set `agents.reviewer: ""` in a fixture and confirm `config agents` and
   `config agent reviewer` agree (Phase 1).
2. Diff the committed test-fixtures tree after each of Phases 2–7 and confirm no
   golden bytes moved.
3. Inspect a non-config I/O error message after Phase 8 for a skill/template path.

## Performance Considerations

`effective(key, None)` reads both config levels, exactly as `get(key, None)` does
today, so the collapse of ~10 call sites onto it does not change the read count.
The launcher is on the skill-load hot path; none of these changes add I/O.

## Migration Notes

No data migration. The only user-visible behaviour change is Phase 1's agent
empty-value fix (`config agent <name>` now falls back to the prefixed default on
an explicit-empty value, matching `config agents`). No config file format
changes; `store`'s on-disk write behaviour is unchanged (Phase 7 changes only
where the umask is read, not the resulting mode).

## References

- Research: `meta/research/codebase/2026-07-22-0167-config-command-refactoring-opportunities.md`
- Work item: `meta/work/0167-config-command-and-invocation-contract-migration.md`
- Implemented plan: `meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- Domain seam: `cli/config/src/service.rs:274-358`, `cli/config/src/catalogue.rs:219-235`
- Agent drift: `cli/launcher/src/config_command/core/agents.rs:35-59`,
  `cli/launcher/src/config_command/inbound/cli.rs:752-759`
- Scalar/block asymmetry: `cli/launcher/src/config_command/inbound/cli.rs:610-814`
- Store read-half: `cli/store/src/lib.rs:97-102`,
  `cli/config-adapters/src/store.rs:646-658`, `cli/corpus-adapters/src/store.rs:118-151`
- Boundary rules: `cli/pup.ron:40-72`
