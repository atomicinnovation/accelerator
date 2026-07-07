---
type: plan
id: "2026-07-07-0178-config-crates-native-yaml-reader"
title: "config and config-adapters Crates with Native YAML Reader Implementation Plan"
date: "2026-07-07T01:13:37+00:00"
author: Toby Clemson
producer: create-plan
status: draft
work_item_id: "work-item:0178"
parent: "work-item:0178"
derived_from: ["codebase-research:2026-07-07-0178-config-crates-native-yaml-reader"]
tags: [rust, config, config-adapters, serde, yaml, cargo-deny, cargo-pup, hexagonal]
revision: "0d38ece1e17e9aee360574399fb61efe3bffd89a"
repository: "accelerator"
last_updated: "2026-07-07T12:55:12+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# config and config-adapters Crates with Native YAML Reader Implementation Plan

## Overview

Build a two-crate hexagon in the `cli/` workspace — `config` (serde-free domain +
application + ports) and `config-adapters` (native serde/YAML frontmatter 
reader + filesystem store) — that replaces the bash 2-level awk reader with a 
native reader supporting arbitrary YAML nesting, and is the first-mover 
activation of the workspace's cargo-deny infra-out-of-domain ban and cargo-pup 
domain-import rule.

The design mirrors the near-complete reference implementation at
[`cli/config`](https://github.com/atomicinnovation/luminosity/tree/main/cli/config) + 
[`cli/config-adapters`](https://github.com/atomicinnovation/luminosity/tree/main/cli/config-adapters)
almost directly, with three
accelerator-specific additions luminosity lacks: the recognised-key catalogue +
defaults modelled as domain concepts, a fail-closed legacy-layout guard, and a
committed cargo-deny canary that confirms the ban bites.

## Current State Analysis

- The shipping config reader is bash: `scripts/config-read-value.sh` splits a key
  on the first dot and runs one of two awk paths (`:56-110`), so genuine 3-level
  nesting is structurally impossible. Precedence is a last-writer-wins loop over
  team-then-local (`:114-130`), and a present-but-empty key counts as found and
  shadows both the team value and the default (`:73-89`). Inline arrays are a
  second string-splitting stage, `config_parse_array` (`config-common.sh:318-331`).
- The recognised-key catalogue spans two files: `scripts/config-defaults.sh`
  holds `PATH_KEYS` (17), the `DOC_TYPE_NAMES`/`DOC_TYPE_PATH_KEYS` parallel
  arrays (13 each), `TEMPLATE_KEYS` (6), `WORK_KEYS`/`WORK_DEFAULTS` (3); and
  `scripts/config-dump.sh` holds `REVIEW_KEYS` (9) + `AGENT_KEYS` (7). Total 42
  keys across 5 groups + 2 doc-type arrays. Three further review keys
  (`review.work_item_revise_severity`, `review.work_item_revise_major_count`, and
  the mode-dependent `review.min_lenses` default) are read directly by
  `scripts/config-read-review.sh` with hard-coded defaults, outside `REVIEW_KEYS`.
- The legacy guard `config_assert_no_legacy_layout` (`config-common.sh:55-67`)
  fires only when the team file `.accelerator/config.md` is **absent** and
  `.claude/accelerator.md` is **present**, printing two stderr lines then
  `exit 1`. It early-returns under `ACCELERATOR_MIGRATION_MODE=1`.
- The `cli/` workspace already ships the hexagon template (the `version` slice:
  `cli/launcher/src/version/core.rs`, its outbound adapter, and the Model-1
  composition root at `cli/launcher/src/main.rs:88-92`) and the enforcement
  scaffolding, both inert: `cli/deny.toml:67-73` (empty `skip`/`skip-tree` with a
  comment that the ban waits for the config split) and `cli/pup.ron:10-39`
  (module-rooted `RestrictImports` rules). `cli/Cargo.toml` has `serde` +
  `serde_json` in `[workspace.dependencies]` but **no YAML crate**.
- The enforcement-regression harnesses already exist and are the exact templates
  to extend: `tests/integration/deny/test_native_tls_ban.py` runs the **real**
  `cli/deny.toml` (`--config`) against committed offline fixture workspaces
  (`--frozen`), and `tests/integration/pup/test_import_rule.py` runs cargo-pup
  against a probe workspace shaped like the real `cli/`. Both are wired as
  `mise run test:integration:deny` / `test:integration:pup`.
- The visualiser's `config.rs` is JSON-only and schema-specific (confirmed
  non-reusable); the only YAML in the repo is the visualiser server's
  `serde_yml`, used through a `catch_unwind` because libyml panics on adversarial
  input — a concrete argument for the chosen pure-Rust `serde-saphyr`.

### Key Discoveries:

- Luminosity is a direct, near-complete ancestor: `config/src/{node,key,level,
  error,service}.rs` and `config-adapters/src/{frontmatter,document,store}.rs` can
  be lifted with accelerator naming. It resolves the YAML Open Question with
  **`serde-saphyr 0.0.29`** (pure-Rust, order-preserving), consumed only by
  `config-adapters` ([`cli/config-adapters/Cargo.toml:12-15`](https://github.com/atomicinnovation/luminosity/blob/main/cli/config-adapters/Cargo.toml#L12-L15)).
- Luminosity already carries the same `serde-saphyr` +
  `wrappers = ["config-adapters"]` ban semantics ([`cli/deny.toml`](https://github.com/atomicinnovation/luminosity/blob/main/cli/deny.toml)) —
  though accelerator's `deny.toml` expresses bans as an inline array, so the ban is
  appended as an inline table there, not as a `[[bans.deny]]` block (Phase 4 §2).
  And luminosity commits **no violating canary** — AC-8 is stricter and requires one.
- Luminosity's `service.rs` resolves precedence **per key at read time**
  (personal over team), with a present `null`/empty string counting as `Found`
  (`service.rs:83-99`, tests `present_null_resolves_to_found`,
  `present_empty_string_resolves_to_found`) — the exact bash "presence, not
  value" semantic. This maps onto bash team→local last-writer-wins.
- Luminosity treats a sequence as an **opaque leaf** — a dotted key landing on a
  `Node::Sequence` resolves `Absent` (`service.rs:129-132`). AC-4 requires inline
  arrays to resolve to a **typed element list**, so this is the one place the
  reader must extend the reference `resolve` semantics.
- The bash reader's project-root discovery walks up to the nearest `.git`
  (`config_project_root` → `find_repo_root`). The differential parity harness must
  therefore copy each fixture into an **isolated tempdir** (no ancestor
  `.git`/`.accelerator`), or both readers would read the accelerator repo's own
  config instead of the fixture.

## Desired End State

Two new crates (`config`, `config-adapters`) build clean, are members of the
`cli/` workspace, and are fully unit-tested. A committed shared fixture suite
drives a differential test proving depth-≤2 parity with the bash reader and
direct-value verification at depth ≥3 and for inline arrays. A composition-root
example wires concrete adapters into the reader and enforces the fail-closed
legacy guard. `cli/pup.ron` carries a whole-crate rule matching `^config($|::)`
and `cli/deny.toml` carries the `wrappers = ["config-adapters"]` ban on
`serde-saphyr`, both proven by regression tests that drive the real gate config
plus a committed violating canary. `mise run` (the bare default task) exits 0
end-to-end.

Verify: `mise run cli:check`, `cd cli && cargo test --workspace`,
`mise run test:integration:deny`, `mise run test:integration:pup`,
`mise run deny:check`, `mise run pup:check`, and finally `mise run` all green.

## What We're NOT Doing

- **Not** wiring any real consumer binary (the launcher `config` subcommand is
  0167; sub-binary consumers are 0169–0173). This task ships the crates plus one
  composition-root entry point — a dedicated, non-shipped `config-adapters-fixture`
  `[[bin]]` in `config-adapters` — and nothing more.
- **Not** porting the `ACCELERATOR_MIGRATION_MODE` read fallback in
  `config_find_files` (deferred to 0172), and deliberately **not** reproducing the
  guard's `ACCELERATOR_MIGRATION_MODE=1` early return — the Rust guard fails
  closed even with that env set.
- **Not** touching the visualiser's `config.rs`/`serde_yml`, nor extracting
  `WorkItemConfig` (that is corpus work, 0179).
- **Not** removing or modifying the bash reader — it remains the shipping reader
  and the parity oracle. Cutover is a later consumer concern.
- **Not** extending the deny/pup rules to other domains (siblings 0179/0180 do
  that); this task only activates them for `config`.

## Implementation Approach

Mirror luminosity's crate pair, layer the accelerator catalogue + legacy guard
on top, and extend the two existing enforcement-regression harnesses rather than
inventing new ones. Serde lives **only** in `config-adapters`; the `config` core
depends on `kernel` alone, which is what makes the cargo-pup rule and the
cargo-deny `wrappers` ban meaningful.

Work test-first throughout: the luminosity crates ship with comprehensive unit
tests that port directly as the specification for each type, and the fixture
suite is authored before the differential harness that consumes it.

The task splits into four phases, each leaving `mise run` green and independently
mergeable. Phases 1 → 2 are sequential (the adapters depend on the domain).
Phases 3 (composition-root entry point + legacy guard) and 4 (first-mover
enforcement) both depend only on Phases 1–2 and are **independent of each other**,
so they may merge in either order or in parallel — the enforcement work is a
distinct discipline (deny/pup + offline fixtures + the nightly lane) with a
different review lens and risk profile than the application entry point.

---

## Phase 1: `config` domain crate

### Overview

The serde-free domain + application + ports: the order-preserving value tree, the
dotted key, the two levels, the error taxonomy with its `kernel::Error` boundary,
the application service performing per-key team→local precedence over arbitrary
depth with sequence-addressable resolution, the recognised-key catalogue +
defaults as domain constants, and the pure legacy-layout predicate. Nothing
depends on this crate yet, so it merges independently.

### Changes Required:

#### 1. Workspace registration

**File**: `cli/Cargo.toml`
**Changes**: Add `config` to `members`. The crates are named unprefixed (`config`,
`config-adapters`) — deliberately matching `kernel` and the luminosity reference
rather than the prefixed `accelerator-verify`; they are `publish = false` internal
crates, so the mixed naming is an intentional choice, not an oversight.

```toml
members = ["launcher", "kernel", "verify", "config"]
```

#### 2. Crate manifest

**File**: `cli/config/Cargo.toml`
**Changes**: Minimal manifest; domain depends on `kernel` only, opts into
workspace lints.

```toml
[package]
name = "config"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
kernel = { path = "../kernel" }
```

#### 3. Value tree, key, level, error

**Files**: `cli/config/src/{node,key,level,error}.rs`
**Changes**: Port luminosity's types verbatim in shape, adjusting the error
`Display` strings to name `.accelerator/config.md`. `Node`/`Scalar`/`Mapping`
(order-preserving `Vec`-backed mapping), `Key::parse` (rejects empty/leading/
trailing/consecutive-dot segments), `Level { Team, Personal }`, and `ConfigError`
with `impl From<ConfigError> for kernel::Error` → `Failed(error.to_string())`
(mirroring `cli/launcher/src/launch/core.rs:167-171`). Add one new variant for
the legacy-layout guard. `ConfigError`, `Value`, and `Scalar` are the consumer-facing
types the six-plus downstream crates (0167, 0169–0173) will `match` on, so each
carries `#[non_exhaustive]` — a later variant (a new error case, a `Value::Mapping`,
a new scalar kind) stays a backward-compatible addition rather than a breaking
change across the shared layer. `Resolved` is **deliberately not** `#[non_exhaustive]`:
it is a closed two-variant `Found(Value)`/`Absent` contract (the single `Found` arm
is load-bearing for the "presence, not value" rule), and all forward-compatible
growth is absorbed by the `#[non_exhaustive]` `Value` it wraps — so consumers
exhaustively match `Found`/`Absent` as a stable contract. The enum-level attribute
protects *variant* growth but not *field* growth on struct-like variants, so
consumers destructure `ConfigError`'s struct variants (`MalformedFrontmatter`,
`NotFound`, …) with a trailing `..` — noted so a future field addition (e.g. a
line/column on `MalformedFrontmatter`) stays non-breaking too.

```rust
#[non_exhaustive]
pub enum Scalar { String(String), Bool(bool), Int(i64), Float(f64), Null }

#[non_exhaustive]
pub enum ConfigError {
    NotFound { key: Key, level: Option<Level> },
    PathConflict { key: Key, at: String, existing: Existing },
    MalformedFrontmatter { path: String, detail: String },
    Io { path: String, detail: String },
    InvalidKey { key: String },
    LegacyLayout,
}
```

`LegacyLayout`'s `Display` renders the two-line migrate directive at parity with
`config-common.sh:55-67`:

```rust
Self::LegacyLayout => write!(
    formatter,
    "Accelerator: legacy config detected at .claude/accelerator.md.\n\
     Run /accelerator:migrate to update the layout, then retry."
),
```

#### 4. Application service with sequence-addressable resolution

**File**: `cli/config/src/service.rs`
**Changes**: Port `ConfigService<R, W>`, the driven ports `ReadConfigLevel` /
`WriteConfigLevel`, the driving port `ConfigAccess` (`get` full-stack personal-
over-team or single-level; `set` with nested insert + `PathConflict`), and the
`resolve`/`insert` walk. Extend the resolution outcome so a sequence value is
addressable (the one deviation from luminosity), keeping presence a **single**
variant so the "presence, not value" precedence rule has exactly one arm.
`Value` (returned inside `Found`) and `Scalar` carry `#[non_exhaustive]` for the
same forward-compatibility reason as `ConfigError`: mapping addressability, if ever
needed, grows `Value` (a `Value::Mapping`) without breaking the six consumers'
`match`. `Resolved` itself stays a closed `Found`/`Absent` enum (no
`#[non_exhaustive]`) — growth lives in `Value`, and the single `Found` arm is
preserved:

```rust
#[non_exhaustive]
pub enum Value {
    Scalar(Scalar),
    Sequence(Vec<Scalar>),
}

pub enum Resolved {
    Found(Value),
    Absent,
}
```

`resolve` returns `Found(Value::Sequence(_))` when the walk lands on a
`Node::Sequence` **whose elements are all scalars**, and `Found(Value::Scalar(_))`
for a scalar leaf. A present value of either shape counts as found for precedence,
so `matches!(resolved, Resolved::Found(_))` stays the single precedence
short-circuit and personal wins over team for a sequence exactly as for a scalar
(a locally-set `review.core_lenses` shadows the team list).

**Present-but-non-addressable nodes resolve `Found`, not `Absent`, for bash
parity.** The bash awk matches the key *line*, so a key that merely opens a nested
block (a `Mapping`) — or a `Sequence` containing a non-scalar element — reads as
found-empty and shadows the team value/default. To match that, `resolve` maps such
a node to `Found(Value::Scalar(Scalar::Null))`: it counts as present (single-arm
precedence holds, personal shadows team) and projects to `""` exactly as bash's
found-empty does. `Absent` is reserved for a key path that does not exist at all.
Fixtures pin both the precedence (personal mapping-node shadows a team scalar → the
`""` projection wins) and the depth-≤2 parity of this case against the bash oracle.

**`get` reads both levels eagerly.** A full-stack `get` reads and parses *both*
Personal and Team up front (propagating a parse error from either) before applying
personal-over-team precedence — it does **not** short-circuit on a `Found` personal
value. This is what makes the fail-loud malformed-team-file divergence fire even
when the personal level would resolve; the malformed fixture guards that ordering.

#### 5. Recognised-key catalogue + defaults as domain constants

**File**: `cli/config/src/catalogue.rs`
**Changes**: Model the 42-key catalogue + defaults as domain data, mirroring the
bash parallel arrays idiomatically. The doc-type arrays stay index-aligned pairs;
each group carries its defaults (template keys have none). The `accelerator:`
agent prefix is a constant. No source-line-number comments — the row-equality
test below is the non-rotting form of "index-aligned with the bash arrays".

```rust
pub const AGENT_PREFIX: &str = "accelerator:";

/// A catalogue default: a scalar or a sequence of scalars. Each maps directly
/// to the `Value` shape the YAML parser yields for the corresponding present
/// value — no YAML source fragment is stored or re-parsed.
pub enum Default {
    Scalar(&'static str),
    Seq(&'static [&'static str]),
}

pub const PATH_KEYS: &[(&str, Default)] = &[
    ("paths.plans", Default::Scalar("meta/plans")),
    ("paths.research_codebase", Default::Scalar("meta/research/codebase")),
    // … remaining path keys
];

pub const DOC_TYPES: &[(&str, &str)] = &[
    ("work-item", "work"),
    ("plan", "plans"),
    // … remaining doc-type pairs
];

pub const TEMPLATE_KEYS: &[&str] = &[ "templates.plan", /* … */ ];

pub const WORK_KEYS: &[(&str, Default)] = &[
    ("work.integration", Default::Scalar("")),
    ("work.id_pattern", Default::Scalar("{number:04d}")),
    ("work.default_project_code", Default::Scalar("")),
];

pub const REVIEW_KEYS: &[(&str, Default)] = &[
    ("review.core_lenses",
     Default::Seq(&["architecture", "code-quality",
                    "test-coverage", "correctness"])),
    ("review.disabled_lenses", Default::Seq(&[])),
    // … remaining scalar review keys, e.g.
    // ("review.min_lenses", Default::Scalar("4")),
];

pub const AGENT_KEYS: &[&str] = &[ "reviewer", /* … prefixed at read */ ];
```

A `default_for(key: &str) -> Option<Value>` helper resolves a key to its catalogue
default (applying `AGENT_PREFIX` for agent keys), giving production resolution and
the tests a single source of the defaults. It maps each `Default` variant to a typed
`Value` — `Default::Scalar` → `Value::Scalar(_)`, and `Default::Seq` →
`Value::Sequence([...])` (an empty sequence for `review.disabled_lenses`) — the *same
shape a present value resolves to*, so a consumer sees one type across the presence
boundary rather than a typed sequence when set and a bracketed string when unset.
Modelling the array defaults as an actual sequence of strings — not a YAML source
fragment re-parsed at read — keeps them compile-checked and removes the
string-splitting the catalogue would otherwise re-introduce for array defaults.

**Where defaults apply is explicit.** `ConfigService::get` returns `Resolved`
(`Found`/`Absent`) and does **not** apply catalogue defaults — resolution and
default-substitution are separate steps. A caller wanting a value-or-default calls
`default_for(key)` on an `Absent`. The `config-adapters-fixture` bin therefore
exits 1 on an `Absent` `paths.work` (it applies no default), while a test asserts
`default_for` returns the catalogue default for the same key; the two are consistent
because defaulting lives in `default_for`, not `get`.

**Mode-dependent defaults are out of scope for the reader.** `review.min_lenses`
has a mode-dependent effective default (4 for pr/plan, 3 for work-item), computed
by the bash `config-read-review.sh`, not `config-read-value.sh`. The native reader
is mode-agnostic: `default_for("review.min_lenses")` encodes the single catalogue
default (`4`), and mode-specific resolution stays a consumer concern (0167+). The
domain models the recognised-key *set* and its static defaults; contextual
defaults belong to the mode-aware caller.

**Drift is caught by a bidirectional test, not a manual read.** A domain test
asserts the Rust catalogue's key set and per-key defaults are identical to the bash
catalogue's by **driving `config-dump.sh`** and reading its output — a behavioural
extraction robust to bash-file reformatting, not a fragile parse of the shell
source. The doc-type arrays are compared as **ordered `(name, path-key)` pairs**
(a zip-equality against `DOC_TYPE_NAMES`/`DOC_TYPE_PATH_KEYS`), not two independent
set checks — so a transposed pairing (`work-item → plans`) fails the test even
though both key sets are unchanged. The test also sources the three defaults read
outside those arrays from `config-read-review.sh` (`review.work_item_revise_severity`
→ `critical`, `review.work_item_revise_major_count` → `2`, `review.min_lenses` → `4`)
rather than restating them as fixture constants, so a drift in *either* reader —
including those three — fails a test rather than relying on a manual read-through.

#### 6. Pure legacy-layout predicate

**File**: `cli/config/src/legacy.rs`
**Changes**: The guard *decision* as a pure function over two booleans, so the
adapter only supplies filesystem facts and the composition root only prints/exits.
No `ACCELERATOR_MIGRATION_MODE` awareness anywhere in the crate.

```rust
pub const fn is_blocked(team_present: bool, legacy_present: bool) -> bool {
    !team_present && legacy_present
}
```

#### 7. Crate root

**File**: `cli/config/src/lib.rs`
**Changes**: Module declarations + re-exports (`Node`, `Scalar`, `Mapping`, `Key`,
`Level`, `ConfigError`, `Existing`, `ConfigAccess`, `ConfigService`,
`ReadConfigLevel`, `WriteConfigLevel`, `Value`, `Resolved`, catalogue items,
`legacy`), with the module doc noting no serde/YAML/fs enters its closure.

### Success Criteria:

#### Automated Verification:

- [x] Crate compiles workspace-wide: `cd cli && cargo build --workspace`
- [x] Domain unit tests pass (ported luminosity suites + `Found(Value::Sequence)`
      resolution and single-arm precedence + non-scalar-node found-empty resolution
      + catalogue-count + catalogue drift + `is_blocked` truth table):
      `cd cli && cargo test -p config`
- [x] Format + clippy clean: `mise run cli:check`
- [x] Catalogue count is exactly 42 keys across the five groups plus the two
      13-entry doc-type arrays, asserted by a domain test iterating the tables.
- [x] Catalogue drift test passes: the Rust key set and per-key defaults are
      identical to the bash `config-defaults.sh`/`config-dump.sh` arrays **and** the
      three `config-read-review.sh` defaults, sourced not restated (drift in either
      reader fails the test).
- [x] `default_for` returns a typed `Value`; an array-key default resolves to the
      same `Value::Sequence` shape as its present value (present/absent shape parity).
- [x] A personal-sequence-over-team precedence test passes (a locally-set
      `review.core_lenses` shadows the team list via the single `Found(_)` arm), and
      a personal non-scalar-node shadows a team scalar (found-empty).
- [x] `mise run deny:check` still green (no new dependency introduced yet).
- [ ] Full local CI mirror green: `mise run`

#### Manual Verification:

- [x] The catalogue tables read row-for-row against `config-defaults.sh` and
      `config-dump.sh` (keys, defaults, doc-type pairings) with no drift.

---

## Phase 2: `config-adapters` crate + shared fixture suite

### Overview

All serde/YAML/filesystem concerns: the frontmatter split, the serde-saphyr
document boundary, `FileConfigStore` (read + write ports, `.accelerator/` rooting,
project-root discovery, atomic write), and the filesystem side of the legacy
guard. Plus the committed shared fixture suite and the tests that bind to it: the
differential bash-parity test (depth ≤2), the direct declared-value tests (depth
≥3 and inline arrays), and the typed-sequence test. Depends on Phase 1.

### Changes Required:

#### 1. Workspace dependency + registration

**Files**: `cli/Cargo.toml`
**Changes**: Add `config-adapters` to `members`; add `serde-saphyr` (pinned) to
`[workspace.dependencies]` alongside the existing `serde`.

**Verify the licence closure before assuming coverage.** `deny.toml`'s allow-list
is deliberately pruned to exactly the licences the current closure carries and
fails on any outside it. Run `cargo deny --config cli/deny.toml check licenses`
against the resolved graph (`serde-saphyr` → saphyr, saphyr-parser, and their
deps) and record the actual licence set; add any missing permissive licence to the
allow-list **as part of this phase**. Do not treat "MIT/Apache, already covered"
as a given — an unlisted permissive licence (0BSD, MIT-0, …) would fail
`deny:check` and block every downstream consumer behind the red gate. The same step
confirms the resolved `serde-saphyr`/saphyr/saphyr-parser graph builds on the
workspace `rust-version` (`1.90.0`) — with resolver 3 and an exact pin there is no
MSRV-compatible float, so a transitive dep requiring a newer Rust would be a hard
build break blocking all consumers; the confirmed floor is recorded.

The `=0.0.29` exact pin carries a rationale comment matching the convention every
other `=` pin in the workspace follows (clap, vergen, reqwest, …):

```toml
members = ["launcher", "kernel", "verify", "config", "config-adapters"]

# in [workspace.dependencies]
# 0.0.x: every patch is semver-breaking, so pin exactly and review upgrades at the
# config-adapters adapter boundary.
serde-saphyr = "=0.0.29"
```

**Dependency-fragility note.** With `deny.toml`'s `yanked = "deny"` /
`unmaintained = "all"` policy, an exact `0.0.x` pin leaves cargo no in-range
float: a yank of `0.0.29`, or a RustSec advisory on it or a transitive dep, turns
the whole `cli` deny gate red with no automatic recovery and blocks all consumers
until a manual (possibly API-breaking) bump. The pin is the right conservative
default for a `0.0.x` crate; the `config-adapters` boundary is what localises the
blast radius of that inevitable upgrade. The two red-gate failure classes need
different recovery levers, both `[advisories] ignore` entries: a RustSec **advisory**
clears with a justified advisory-id entry (`RUSTSEC-…`), whereas a **yank** has no
advisory id and clears only with a PackageSpec entry (`serde-saphyr@0.0.29`) — a
form only recent cargo-deny supports, so the pinned cargo-deny version is confirmed
to accept PackageSpec-in-ignore. Either is a justified, documented stopgap while the
boundary bump is prepared, so a yank/advisory does not leave the gate red
indefinitely.

#### 2. Crate manifest

**File**: `cli/config-adapters/Cargo.toml`
**Changes**: Same `[package]` workspace inheritance and `[lints] workspace = true`
block every other crate carries, plus the adapter dependencies:

```toml
[package]
name = "config-adapters"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
config = { path = "../config" }
serde = { workspace = true }
serde-saphyr = { workspace = true }
```

#### 3. Frontmatter split + serde-saphyr document boundary

**Files**: `cli/config-adapters/src/{frontmatter,document}.rs`
**Changes**: Port luminosity's `frontmatter::split` (CRLF-aware, body-preserving,
only the first two `---` fences delimit, unterminated block is an error) and the
`document` boundary — the serde-side `Parsed` enum with its hand-written `Visitor`
(`deserialize_any`) and `Serialize`, plus `to_node`/`to_parsed`, `parse`, and
`render`. This is where `serde-saphyr` is imported; the `config` core never sees it.

**No panic sandbox.** Unlike the visualiser's `serde_yml`/libyml (wrapped in
`catch_unwind` because the C backend panics on adversarial input), `serde-saphyr`
is pure-Rust and the parse is called directly — no `catch_unwind`, no adapter-side
recursion-depth cap. This is a deliberate lightest-touch choice: the adversarial
fixtures in the suite below **characterise** the reader's behaviour on
deeply-nested and malformed YAML (documenting whatever `serde-saphyr` does) rather
than asserting a hard fail-safe. The revisit trigger covers the failure modes a
recursive-descent parser actually has on hostile input — **a catchable panic, a
stack-overflow abort, an OOM, or a hang** (billion-laughs-style alias expansion) —
not just a catchable panic; and the effective control for the abort/OOM/hang cases
is a **depth/input-size bound in the adapter**, since `catch_unwind` cannot stop an
abort. If a fixture surfaces any of these, add that bound before the first real
consumer (0167) cuts over.

#### 4. Filesystem store (read + write)

**File**: `cli/config-adapters/src/store.rs`
**Changes**: Port `FileConfigStore` implementing `ReadConfigLevel` +
`WriteConfigLevel`, with `.accelerator/config.md` (team) and
`.accelerator/config.local.md` (personal) paths, root discovery, and the atomic
temp-file-then-rename write. `render` preserves the existing body verbatim.

`ReadConfigLevel` returns an **empty level** for a non-existent file (the common
case — a repo with only a team file has no `config.local.md`), reserving `Io` for a
present-but-unreadable file and `MalformedFrontmatter` for a present-but-unparseable
one — matching bash `config_find_files`, which simply omits absent files. The eager
both-level read (Phase 1 §4) therefore does not error on the ubiquitous
team-file-only layout; a unit test asserts an absent personal file yields an empty
level, not an error.

Root discovery is factored so the composition root can obtain the root **once**
and reuse it for both the legacy guard and the store (see Phase 3): expose the
walk as a `discover_root(start) -> PathBuf` and construct the store `at(root)`.
The walk stops at the nearest ancestor holding `.accelerator/`, `.git`, **or
`.jj`**; with no marker it falls back to `start` (the cwd), matching bash
`config_project_root`'s `$PWD` fallback. The `.jj` marker matters because this repo
is jj-colocated and jj-workspace checkouts can lack a colocated `.git` — the bash
oracle roots via `find_repo_root`, which stops at `.jj` or `.git`
(`scripts/vcs-common.sh:11`), so omitting `.jj` would root a jj-only checkout
differently from bash. `.accelerator/` remains a Rust-only stop marker (bash never
roots on it); this is a deliberate, accepted divergence recorded under Platform
scope, and it is harmless in practice because a project root carrying
`.accelerator/config.md` is at or below its VCS root. A store unit test pins the
marker precedence when a nested `.accelerator/` sits under an ancestor `.git`,
adds a `.jj`-only (no `.git`) case, and asserts the no-marker fallback equals the
start dir — so discovery is tested, not assumed.

The atomic write creates its temp file in the **target's own directory** (so the
rename is same-filesystem and atomic) and fsyncs it before the rename — carried
over from the reference. It does not fsync the containing directory after the
rename, so a crash immediately after rename could still lose it on some
filesystems; this and crash/round-trip integrity are flagged for the first writing
consumer (0167) rather than covered here (this task is read-focused, and the write
path is exercised only by ported unit tests).

#### 5. Filesystem legacy guard

**File**: `cli/config-adapters/src/legacy.rs`
**Changes**: The adapter that stats the two paths at a root and applies the domain
predicate, returning `Err(ConfigError::LegacyLayout)` when blocked. No
`ACCELERATOR_MIGRATION_MODE` read.

```rust
pub fn assert_no_legacy_layout(root: &Path) -> Result<(), ConfigError> {
    let team = root.join(".accelerator/config.md").exists();
    let legacy = root.join(".claude/accelerator.md").exists();
    if config::legacy::is_blocked(team, legacy) {
        return Err(ConfigError::LegacyLayout);
    }
    Ok(())
}
```

#### 6. Shared fixture suite

**Directory**: `cli/config-adapters/tests/fixtures/configs/`
**Changes**: A committed fixture set (nested under `configs/` so the data fixtures
sit apart from the `config_adapters_fixture.rs` bin source that shares
`tests/fixtures/`), each fixture a project root containing
`.accelerator/config.md` (and `.local.md` where relevant), plus a manifest
(`expectations.toml` or a Rust table) declaring per-key expected values and
declared nesting depth. Coverage:

- At least one fixture per recognised key across the five groups + both doc-type
  arrays.
- A team/local precedence-conflict fixture (a key set differently in both files,
  local expected to win).
- A **presence-not-value matrix** exercising every shadowing variant the bash
  reader has (`config-read-value.sh:73-89`), each asserting the present value
  shadows both the other level and the default: local empty-string (`key: ""`),
  local YAML null (`key:`), and a team-level present-empty case (empty at *team*
  shadows the default even without a local key). This replaces the single
  present-empty fixture — the variants are load-bearing and luminosity splits them
  into distinct tests (`present_null_resolves_to_found`,
  `present_empty_string_resolves_to_found`).
- A **non-scalar-node fixture**: a recognised scalar key whose personal value opens
  a mapping (or a block sequence of mappings), team holds a plain scalar. Asserts
  the Rust reader resolves it `Found`-empty (`render_resolved` → `""`) and shadows
  the team scalar — matching bash's found-empty (Phase 1 §4), verified against the
  oracle in the §7 scalar loop.
- A default-fallback fixture for each key that declares a default — the three
  `WORK_DEFAULTS` keys (incl. `work.id_pattern` → `{number:04d}`) and the two
  inline-array `REVIEW_KEYS` defaults. **Absence only**: these fixtures omit the
  key so the default path is taken; the defaults themselves are verified against
  the bash catalogue by the drift test in Phase 1 §5, not by feeding the Rust
  value into the oracle (see §7). One of the inline-array default fixtures also
  asserts the **default-path shape**: an absent `review.core_lenses` yields
  `Value::Sequence([...])`, identical to the present-value shape (not a bracketed
  string), so a consumer sees one type across the presence boundary.
- **Value-encoding fixtures** — the cases where a real YAML parser diverges from
  bash's opaque-string handling, each with the intended result declared and listed
  in Documented Divergences (not accidental):
  - `work.id_pattern: {number:04d}` **present** (not just absent-default): bash
    reads the literal string; YAML sees `{` as a flow-mapping indicator. Declare
    the intended Rust result and, if it is `Absent`/error, document this as a
    known divergence the cutover (0167) must handle (e.g. quote-on-write).
  - one further flow-indicator-leading value (leading `[`).
  - a double-quoted scalar (`key: "value"` → `value`; bash strips one quote pair).
  - a quoted array element (`review.core_lenses: ["a", b]`): bash keeps `"a"`
    literally in `config_parse_array`; serde parses it to `a`. Declare the
    intended typed-sequence result and mark the divergence explicitly.
  - an explicit null token (`key: null` / `key: ~`), a boolean-case value
    (`key: True`), a non-canonical number (`key: 007` / `1.0` / `1e3` / `+5`), and
    a value with a trailing YAML comment (`key: meta/work # note`), each declaring
    the `render_resolved` result versus bash's literal so the divergence is pinned.
- Depth-≥3 fixtures with declared expected values: one 3-level scalar, one 4-level
  scalar, one nested inline-array.
- Inline-array fixtures for `review.core_lenses` and `review.disabled_lenses`
  resolving to typed element lists (and `[]` → empty sequence). Inline form is
  parity: bash's `config_parse_array` yields the same element list the Rust reader
  does.
- A **block-sequence fixture** for `review.core_lenses` (the `- item` block form,
  not inline `[…]`): bash matches only the `core_lenses:` line, reads it found-empty
  (`config_parse_array("")` → empty → default applied), while the Rust reader parses
  the block into `Value::Sequence([...])`. Asserts the Rust typed-sequence result
  against the bash found-empty result and lists the divergence in Documented
  Divergences — block-authored arrays are the one array shape where the readers part.
- The three extra review keys read outside `REVIEW_KEYS`
  (`review.work_item_revise_severity` → `critical`,
  `review.work_item_revise_major_count` → `2`), each with a declared expected
  value. **`review.min_lenses` is mode-agnostic here**: a fixture asserts the
  reader returns the catalogue default (`4`) or `Absent` when the key is unset —
  the 4-vs-3 mode behaviour lives in the consumer (0167+), so no fixture claims to
  exercise it.
- A **malformed-frontmatter fixture pair** — one malformed in the **team** file
  (valid local value), one malformed in the **personal** file (valid team value).
  Both pin the deliberate **fail-loud** divergence: bash warns and skips the bad
  file (resolving the other level), the native reader errors the whole `get`
  (`Err(MalformedFrontmatter)`), and the eager both-level read propagates the error
  from *either* level. The personal-file case matters most — the local override is
  the file individuals edit most freely. Documented alongside the
  `ACCELERATOR_MIGRATION_MODE` fail-closed decision as a chosen semantic, not an
  accident (see What We're NOT Doing).
- **Adversarial-input fixtures** — deeply-nested and structurally-malformed YAML —
  that **characterise** the reader's behaviour (per the no-sandbox choice in §3):
  assert it returns a `ConfigError` where it does, and document the observed
  outcome otherwise, so the read path's response to hostile input is a recorded
  fact before any consumer cuts over.
- A legacy-layout fixture: `.claude/accelerator.md` present, no
  `.accelerator/config.md`.
- The **normal-layout positive-control fixture** (used by Phase 3's black-box
  test) must actually set `paths.work`, so the `config-adapters-fixture` bin resolves
  it and exits 0 — the control means "ran and resolved", not "short-circuited". A
  companion **normal-layout-without-`paths.work`** fixture drives the bin's `Absent`
  arm (exit 1), so the miss path is exercised, not just asserted in prose.

#### 7. Differential + declared-value tests

**File**: `cli/config-adapters/tests/parity.rs`
**Changes**: For each fixture the harness materialises the committed config files
into a fresh `env!("CARGO_TARGET_TMPDIR")` subdir seeded with a `.git` marker —
mirroring luminosity's `launcher/tests/config.rs:20-28`. Both readers then root
**inside** that dir: `FileConfigStore::discover_root` stops at the seeded `.git`,
and the bash reader's `find_repo_root` stops there too, so the upward walk can
never escape into the accelerator repo's own `.git`/`.accelerator`.

The two readers speak different value types (bash always emits a string; the Rust
reader emits a typed `Value`), so parity is defined against **two exported
projection functions** in `config-adapters` (named distinctly from
`document::render`, which round-trips a document body). `render_value(&Value) ->
String` is total over `Value`: `Scalar(Null)`→`""`; `Scalar(Bool)`→lowercase
`true`/`false`; `Scalar(Int)`→canonical decimal; `Scalar(Float)`→canonical decimal;
`Scalar(String)`→the string as-is; `Sequence`→the bracketed comma-space form
(`[a, b, c]`), so the array arm is defined, not improvised. `render_resolved(&Resolved)
-> String` layers on it: `Absent`→the sentinel below; `Found(v)`→`render_value(v)`.
`parity.rs` calls `render_resolved` (it must handle the `Absent` miss) and the Phase
3 bin calls `render_value` on its `Found` value, so the shipped output and the
parity oracle share one projection and cannot drift.

- **Resolution parity (depth ≤2, scalar keys)**: for each recognised **scalar**
  key, run the bash oracle `scripts/config-read-value.sh <key> <SENTINEL>` (via
  `std::process::Command`, cwd = the seeded dir) and the Rust `ConfigService::get`,
  asserting `render_resolved(resolved)` equals the bash value. The default argument is a
  **distinctive sentinel, not the Rust catalogue default** — for a present key both
  readers read the file (the default is irrelevant), and for a genuine miss both
  echo the sentinel. This proves resolution/precedence/presence-shadowing parity
  *independent of the catalogue defaults*, which the Phase 1 §5 drift test owns
  (feeding the Rust default in would make the assertion tautological).
  **Each recognised scalar key is asserted against a fixture where it is genuinely
  set** (a resolved-value case), not only a miss — otherwise both readers echo the
  sentinel and the assertion degrades to `SENTINEL == SENTINEL`, verifying no
  resolution logic. The harness fails if any key's parity check only ever exercised
  the sentinel-miss branch. The matrix includes the precedence-conflict, the
  presence-not-value fixtures, and the **non-scalar-node** case (a key whose
  personal value opens a mapping → `render_resolved` yields `""`, matching bash's
  found-empty, and shadows a team scalar — Phase 1 §4).
  Array-valued keys (`review.core_lenses`, `review.disabled_lenses`) are
  **excluded** from this string loop — a sequence has no faithful bash-string
  oracle — and routed to the typed-sequence assertions below.
- **Catalogue-default application (Rust-side)**: assert `default_for(key)` returns
  the catalogue default for an absent key that declares one, **as a typed `Value`
  of the same shape the present value resolves to** — a fixture pins that an absent
  `review.core_lenses` yields `Value::Sequence([...])` (not a bracketed string),
  identical to the present-value shape. The Phase 1 §5 drift test proves that
  default equals the bash catalogue array, so the two together cover the default
  path without the tautology.
- **Malformed divergence, both sides**: the malformed-frontmatter fixture asserts
  the Rust reader errors (`Err(MalformedFrontmatter)`) **and** runs the bash oracle
  on the same fixture, asserting bash resolves the local value (exit 0), so both
  halves of the documented divergence are pinned rather than only the Rust side.
- **Doc-type arrays**: the doc-type name→path-key mapping (`DOC_TYPES`) is modelled
  as catalogue data with **no resolution step in this task** — resolving a doc-type
  to its directory is a higher-level bash concern (`config_resolve_*`) not owned
  here. The suite asserts the Rust `DOC_TYPES` table matches the bash
  `DOC_TYPE_NAMES`/`DOC_TYPE_PATH_KEYS` arrays (part of the §5 drift test), and the
  underlying `paths.*` keys the pairs reference are covered by the scalar loop
  above. No fixture claims a doc-type *resolution* parity the harness can't run.
- **Value-encoding + depth ≥3 + typed sequence (declared-value)**: assert the Rust
  reader against the fixture manifest's declared expected values directly (bash
  cannot represent these). `review.core_lenses` resolves to
  `Found(Value::Sequence([architecture, code-quality, test-coverage, correctness]))`,
  `review.disabled_lenses` (`[]`) to an empty sequence, and the value-encoding cases
  to their declared (possibly divergent) results: the `{number:04d}` /
  quoted-scalar / quoted-element cases, plus the classes where `render_resolved`'s
  parse-then-format diverges from bash's literal passthrough — explicit null tokens
  (`key: null` / `key: ~` → bash `null`/`~`, `render_resolved` `""`), boolean case
  (`True`/`TRUE` → bash literal, `render_resolved` `true`), non-canonical numbers
  (`007`, `1.0`, `1e3`, `+5` → bash literal, `render_resolved` canonical decimal),
  and a trailing
  YAML comment (`meta/work # note` → bash keeps the literal, serde strips it). Each
  is asserted with its declared result and listed in Documented Divergences, so
  they are tested decisions rather than latent surprises. The adversarial fixtures
  assert their characterised outcomes with a **hard "must not panic/abort/hang"**
  floor: each adversarial parse runs under a **bounded-time guard** — the parse is
  driven on a worker thread joined with a `recv_timeout` (or via the
  `config-adapters-fixture` subprocess with a wall-clock kill), so a
  billion-laughs-style alias-expansion **hang** deterministically fails the test
  instead of wedging the suite (`cargo test` has no default per-test timeout), and
  a catchable panic or an abort/OOM in the child also fails rather than being
  re-characterised away.

**Determinism and platform-independence.** The child `Command` pins `LANG=C`,
`LC_ALL=C`, **and `PWD`** to the seeded directory (`.current_dir` chdirs but leaves
`PWD` inherited from the parent, and the bash reader's `find_repo_root` walks up
from `$PWD`) so the oracle's rooting cannot depend on stale inherited parent state;
the awk (POSIX classes, quote/whitespace stripping) is thereby locale-independent
too. Parity fixtures are restricted to values whose bash resolution
is awk-implementation-independent (plain scalars, no printf/locale-sensitive
formatting), since the same bash script runs under BSD awk / bash 3.2 on macOS and
mawk/gawk on Linux CI and the Rust output cannot match two different oracles. The
script path is resolved from `CARGO_MANIFEST_DIR` → `../../scripts/…` and asserted
to exist at test start with a diagnostic naming the expected `scripts/` location,
so a layout change fails loudly rather than as an opaque "command not found".

**bash availability.** Rust's stable test harness has no runtime skip — an early
return registers as a green PASS, not a skip. So the harness probes `bash` on
`PATH` and: **`panic!` (fails the test) when `CI` or `GITHUB_ACTIONS` is set**; and
otherwise `eprintln!`s that parity was skipped and returns, **explicitly noting
this is a silent pass, not a true skip**, while still running the declared-value
assertions (which need no bash). This gives the `_require_tools` intent — never
silently green in CI — with the mechanism Rust actually supports.

### Success Criteria:

#### Automated Verification:

- [x] Adapters compile: `cd cli && cargo build --workspace`
- [x] Adapter unit tests pass (ported frontmatter/document/store suites):
      `cd cli && cargo test -p config-adapters`
- [x] Resolution parity holds at depth ≤2 over every recognised scalar key —
      precedence, presence-not-value shadowing, the non-scalar-node found-empty
      case, and the sentinel-default miss — with `render_resolved` equal to the bash
      oracle, **each key asserted against a fixture where it is genuinely set** (the
      harness fails if a key's check only hit the sentinel-miss branch):
      `cd cli && cargo test -p config-adapters --test parity`
- [x] `render_resolved` is a single exported function called by both `parity.rs`
      and the Phase 3 bin (a wrong projection fails a typed fixture), and
      array-valued keys are verified as typed sequences, not via the string loop.
- [x] The store discovery test covers marker precedence, a `.jj`-only root, and the
      no-marker `$PWD` fallback: `cd cli && cargo test -p config-adapters`
- [x] Depth-≥3 scalars (3- and 4-level), the nested inline-array, the typed
      sequences, and the value-encoding cases (`{number:04d}` present, quoted
      scalar/element, null tokens, bool case, non-canonical numbers, trailing
      comment) resolve to their declared expected values (same test binary).
- [x] The malformed-frontmatter fixture asserts **both sides** of the fail-loud
      divergence (Rust `Err(MalformedFrontmatter)`; bash resolves the local value),
      and the adversarial fixtures assert their characterised outcomes under a hard
      no-panic/no-abort floor.
- [x] Format + clippy clean: `mise run cli:check`
- [x] `mise run deny:check` green with `serde-saphyr` in the graph — its
      transitive licence closure verified and any missing permissive licence added
      to the allow-list this phase (ban not yet added).
- [ ] Full local CI mirror green: `mise run`

#### Manual Verification:

- [x] The fixture suite visibly covers every recognised key (spot-check the
      manifest against the catalogue), and the bash oracle is genuinely exercised
      (a deliberately-wrong expected value fails the parity test).
- [ ] Running the parity test in a shell with `bash` removed from `PATH` while
      `CI=1` is set fails (not silently passes), confirming the availability guard.

---

## Phase 3: Composition-root entry point + legacy guard

### Overview

The config-reader entry point — a dedicated, non-shipped `[[bin]]` named
`config-adapters-fixture` in `config-adapters` that demonstrates Model-1 wiring
(each sub-binary constructs its own adapters at its composition root) and enforces
the fail-closed legacy guard — plus the Rust black-box test that runs the compiled
binary and asserts its exit code + stderr. Depends on Phases 1–2; independent of
Phase 4 (they may merge in either order).

The entry point is a `[[bin]]`, not a cargo `examples/` target, because
`env!("CARGO_BIN_EXE_<name>")` — the compile-time path a Rust integration test
uses to locate and execute a compiled binary — is set only for `[[bin]]` targets
in the test's own package. It follows this repo's own fixture-bin convention set
by `accelerator-fixture` (`cli/launcher/Cargo.toml:19-21`): the source lives under
`tests/fixtures/` with an explicit `[[bin]] path`, and the name is
`<crate>-fixture` (`config-adapters-fixture`). Placing it under `tests/fixtures/`
rather than `src/bin/` keeps it out of the crate's normal build surface and signals
test-only intent; `config-adapters` is `publish = false` and release staging is an
explicit allowlist (`DISPATCHED_SUBBINARIES` in `tasks/shared/paths.py`), so the
bin cannot be globbed into a release.

### Changes Required:

#### 1. Composition helper + entry point (the config-adapters-fixture `[[bin]]`)

**Files**: `cli/config-adapters/src/{compose,render}.rs` (the wiring helper and the
value projection, in dedicated modules so these non-adapter concerns stay legible
and cheaply extractable — a noted cohesion tradeoff for `config-adapters` absorbing
imperative-shell wiring + presentation alongside its fs/YAML adapters),
`cli/config-adapters/Cargo.toml`, `cli/config-adapters/tests/fixtures/config_adapters_fixture.rs`
**Changes**: Expose the Model-1 wiring protocol as a **single tested helper** so it
is not hand-copied across six consumers. Its three primitives —
`FileConfigStore::discover_root`, `legacy::assert_no_legacy_layout`,
`FileConfigStore::at` — stay individually public, so 0172's migration-aware consumer
(which must soften the guard) composes the same primitives rather than re-copying
the ordering `compose` encapsulates:

```rust
pub fn compose(cwd: &Path) -> Result<ConfigService<FileConfigStore, FileConfigStore>, ConfigError> {
    let root = FileConfigStore::discover_root(cwd);
    legacy::assert_no_legacy_layout(&root)?;
    let store = FileConfigStore::at(root);
    Ok(ConfigService::new(store.clone(), store))
}
```

`compose` discovers the root **once**, runs the legacy guard against that root,
then builds the store and `ConfigService` rooted at the *same* directory — the
subtle ordered protocol that keeps the guard and reader from ever disagreeing about
which directory they inspect. Each sub-binary (0167, 0169–0173) still calls
`compose` at its own composition root (Model-1 is preserved — no shared global
registry), but the ordering is one unit tested once rather than a recipe re-typed
six times. The projection is exported once (Phase 2 §7): `parity.rs` calls
`render_resolved` (handling the `Absent` miss) and this bin calls `render_value` on
its `Found` value — both share the same `render_value` core, so the shipped output
and the parity oracle can never drift (both named distinctly from
`document::render`).

The bin declares the `[[bin]]` with a manifest comment marking it a
composition-root demonstration + black-box entry point (not a shipped artifact),
and folds every fallible step into the same `eprintln!` + `ExitCode::FAILURE` path.
It carries the same crate-level clippy-allow header `accelerator-fixture` uses
(`#![allow(clippy::exit, clippy::print_stdout, clippy::print_stderr,
clippy::restriction)]`), and the integration tests `parity.rs` / `config_reader.rs`
carry `#![allow(clippy::expect_used, clippy::unwrap_used)]` per the existing
`cli/*/tests` convention — otherwise the workspace's warn-level restriction lints
would fail the `cli:check` clippy-clean `--all-targets` gate:

```toml
[[bin]]
name = "config-adapters-fixture"
path = "tests/fixtures/config_adapters_fixture.rs"
```

```rust
fn main() -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let service = match config_adapters::compose(&cwd) {
        Ok(service) => service,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    match service.get(&Key::parse("paths.work").expect("constant key parses"), None) {
        Ok(Resolved::Found(value)) => {
            println!("{}", config_adapters::render_value(&value));
            ExitCode::SUCCESS
        }
        Ok(Resolved::Absent) => {
            eprintln!("paths.work not set");
            ExitCode::FAILURE
        }
        Err(error) => { eprintln!("{error}"); ExitCode::FAILURE }
    }
}
```

The `Resolved` variants are matched **explicitly**: `Found` prints and exits 0,
`Absent` exits 1 (the bin applies no catalogue default — a genuine miss is a
failure, so the positive control means "ran and resolved", not "short-circuited").
The one `expect` is on a compile-time-constant key that always parses (the sole
infallible step); every runtime-fallible step routes to the `FAILURE` arm. A
black-box test case with a normal layout that does **not** set `paths.work` asserts
exit 1, so the Absent arm is exercised.

#### 2. Legacy-guard black-box test via the compiled binary

**File**: `cli/config-adapters/tests/config_reader.rs`
**Changes**: A pure-std Rust integration test mirroring
`luminosity/cli/launcher/tests/config.rs` — locate the binary via
`const READER: &str = env!("CARGO_BIN_EXE_config-adapters-fixture");`, run it with
`Command::new(READER).current_dir(dir).env(...).output()` against a per-test dir
under `env!("CARGO_TARGET_TMPDIR")` seeded with a `.git` marker, and assert:

- Legacy layout (no team file, `.claude/accelerator.md` present) → exit code 1 and
  stderr contains `/accelerator:migrate`.
- The same layout **with `ACCELERATOR_MIGRATION_MODE=1` set in the child env** →
  still exit code 1 (fails closed; the bypass is deliberately not ported).
- **The same legacy layout, run from a nested subdirectory** of the seeded root
  (cwd = `root/sub`, `.git` and `.claude/accelerator.md` at `root`) → still exit
  code 1. This exercises the guard's rooting: because `compose` discovers the root
  before guarding, the subdir case must fire exactly as the root-cwd case does. A
  guard that inspected the cwd instead would pass here — this test is what catches
  that regression.
- **A `.jj`-only variant of the subdirectory case** (root seeded with `.jj` and no
  `.git`, `.claude/accelerator.md` at root, cwd = `root/sub`) → still exit code 1.
  This exercises the `.jj` root marker, matching the bash `find_repo_root` rooting a
  jj-workspace checkout relies on; without `.jj` in the marker set the walk would
  escape the root and the guard would miss the legacy file.
- A normal layout (`.accelerator/config.md` present, setting `paths.work`) → exit
  code 0 and the resolved value on stdout (positive control, so a pass means "ran
  and resolved", not "guard short-circuited").

### Success Criteria:

#### Automated Verification:

- [x] The bin builds and is clippy-clean (`--all-targets`): `mise run cli:check`
- [x] Black-box guard tests pass: `cd cli && cargo test -p config-adapters --test config_reader`
- [x] Legacy layout exits 1 with `/accelerator:migrate` on stderr, still exits 1
      under `ACCELERATOR_MIGRATION_MODE=1`, and still exits 1 when run from a
      nested subdirectory (both a `.git`-rooted and a `.jj`-only root); a normal
      layout (with `paths.work` set) exits 0.
- [ ] Full local CI mirror green: `mise run`

#### Manual Verification:

- [x] Running `config-adapters-fixture` by hand in a legacy-layout repo prints the
      two-line directive and exits 1 (including from a subdirectory); in a normal
      repo it resolves and prints a value.

---

## Phase 4: First-mover enforcement activation

### Overview

Activation of the two enforcement gates with regression coverage: the whole-crate
cargo-pup rule matching `^config($|::)`, the cargo-deny `wrappers` ban on
`serde-saphyr`, and the committed violating canary that confirms the ban bites.
Depends on Phases 1–2 (the crate boundary must exist); independent of Phase 3.

### Changes Required:

#### 1. cargo-pup crate-rooted rule

**File**: `cli/pup.ron`
**Changes**: Add a third `Module` lint. Unlike the module-rooted version/launch
rules (`^accelerator::version::core`), this matches the **whole `config` crate**:
the entire crate is domain (no adapter modules live inside it), so matching the
crate root — rather than enumerating today's module names — is both simpler and
future-proof, a new domain module (e.g. `defaults.rs`) can never silently escape
the import restriction. The self-reference allowance stays crate-relative and
`kernel::Error` is permitted.

```ron
Module((
    name: "config_domain_imports_only_permitted",
    matches: Module("^config($|::)"),
    rules: [
        RestrictImports(
            allowed_only: Some([
                "^(std|core|alloc)(::|$)",
                "^kernel::Error(::|$)",
                "^crate(::|$)",
            ]),
            denied: None,
            severity: Error,
        ),
    ],
)),
```

#### 2. cargo-deny wrappers ban

**File**: `cli/deny.toml`
**Changes**: Add the infra-out-of-domain ban and drop the now-satisfied
scaffolding comment. `serde-saphyr` becomes reachable only through
`config-adapters`; a direct `config`-domain import violates. `deny.toml` already
defines bans as an **inline array** under `[bans]` (`deny = [ { crate = … }, … ]`)
for the TLS crates — append an inline table to that array. Do **not** introduce a
`[[bans.deny]]` array-of-tables block: TOML cannot define `bans.deny` as both an
inline-array value and an array-of-tables, so that form is a redefinition conflict
that fails to parse rather than activating the ban.

```toml
# appended to the existing [bans] `deny = [ … ]` array
{ crate = "serde-saphyr", wrappers = ["config-adapters"] },
```

#### 3. Committed cargo-deny canary + clean control

**Directory**: `tests/integration/deny/fixtures/`
**Changes**: Add a fixture pair mirroring the existing `banned`/`clean` layout,
using a **local path stub crate named `serde-saphyr`** (so the ban matches on
crate name with no network) and committed `Cargo.lock`s for `--frozen`:

- **banned**: a package **not** named `config-adapters` (e.g. `config`) depending
  directly on the local `serde-saphyr` stub — bypassing the wrapper.
- **clean**: a package named `config-adapters` depending on the same stub — the
  permitted wrapper.

**File**: `tests/integration/deny/test_serde_saphyr_ban.py`
**Changes**: Mirror `test_native_tls_ban.py` — run the real `cli/deny.toml`
(`--config`) `--frozen` against each fixture, asserting the banned fixture exits
non-zero and names `serde-saphyr`, and the clean fixture exits zero (so a pass
means "evaluated and allowed", not "evaluated nothing").

#### 4. cargo-pup crate-rooted probe regression

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: Add a probe whose crate is literally named `config` with real domain
modules (e.g. a `service` module), driven by the **real `cli/pup.ron`** (via
`--config`, exactly as the deny test runs the real `deny.toml`) — not a
hand-written probe RON. Assert a `config::service` import of an adapter/serde
module is rejected (`is not allowed`, names the rule) and a compliant
`config::service` passes. Driving the shipped rule is what makes the regression
guard the real regex: a typo in the rule (or its deletion) makes this test fail,
where a self-contained probe RON would not. The existing
`test_real_cli_pup_ron_loads` continues to guard that `cli/pup.ron` parses, but
parseability alone does not prove discrimination — this probe does. (Note: with
the whole-crate `^config($|::)` match from §1, any real module works as the probe
subject; no `core` module is required or created.)

### Success Criteria:

#### Automated Verification:

- [x] The `config` domain stays serde-free and the crate-rooted rule loads on the
      real graph: `mise run pup:check`
- [x] The wrappers ban bites — the committed canary makes cargo-deny exit
      non-zero and the clean control passes: `mise run test:integration:deny`
- [x] The crate-rooted pup rule discriminates (violation rejected, compliant
      passes): `mise run test:integration:pup`
- [x] The real gates stay green on the real graph: `mise run deny:check`
- [x] Format + clippy clean: `mise run cli:check`
- [ ] Full local CI mirror green: `mise run`

#### Manual Verification:

- [x] Temporarily reverting the `deny.toml` ban makes `test:integration:deny`
      fail (the test guards the rule, not just its presence), and reverting the
      `pup.ron` rule makes `test:integration:pup` fail.

---

## Testing Strategy

### Unit Tests:

- **`config`**: the ported luminosity suites (key parsing, precedence, nested
  walk, set/insert conflicts, fail-loud reads, error `Display` + `kernel::Error`
  mapping) plus new tests for `Found(Value::Sequence)` resolution and single-arm
  precedence (personal sequence over team), the non-scalar-node found-empty
  resolution, the catalogue count/contents, the catalogue drift test against the
  bash arrays (incl. the three `config-read-review.sh` defaults), `default_for`
  returning a typed `Value` with present/absent shape parity for array keys, and
  the `is_blocked` truth table.
- **`config-adapters`**: the ported frontmatter/document/store suites (CRLF split,
  body preservation, typed scalars, integer-beyond-i64-as-string, sequence parse,
  malformed-frontmatter, atomic write, discovery — incl. the marker-precedence,
  `.jj`-only root, and no-marker `$PWD`-fallback cases) plus the legacy-guard
  adapter, the `compose` wiring helper, and the `render_value`/`render_resolved`
  projection.

### Integration Tests:

- **Differential parity** (`config-adapters/tests/parity.rs`): bash oracle vs Rust
  reader at depth ≤2 over the shared fixture suite (via the `Resolved`→string
  projection, sentinel default, scalar keys only); declared-value assertions at
  depth ≥3 and for inline arrays/typed sequences, the value-encoding cases, the
  fail-loud malformed divergence, and the adversarial-input characterisation.
- **Legacy guard** (`config-adapters/tests/config_reader.rs`): a pure-std Rust
  black-box test that runs the compiled `config-adapters-fixture` `[[bin]]` (located
  via `env!("CARGO_BIN_EXE_config-adapters-fixture")`) and asserts exit code +
  stderr for the legacy, `ACCELERATOR_MIGRATION_MODE=1`, nested-subdirectory, and
  normal-layout cases.
- **Enforcement** (`tests/integration/deny`, `tests/integration/pup`): the
  serde-saphyr canary/clean pair and a pup probe driven by the **real** `cli/pup.ron`
  (Python pytest, matching the existing native-tls/pup harnesses).

### Manual Testing Steps:

1. Run `cargo run -p config-adapters --bin config-adapters-fixture` from a repo with
   a normal `.accelerator/config.md` (with `paths.work` set) — it prints the
   resolved value and exits 0.
2. Run it from a repo with only `.claude/accelerator.md` — two-line directive on
   stderr, exit 1; repeat with `ACCELERATOR_MIGRATION_MODE=1` — still exit 1; and
   repeat from a subdirectory of that repo — still exit 1.
3. Break a fixture's expected value and confirm `parity.rs` fails; restore it.
4. Comment out the `deny.toml` ban and confirm `test:integration:deny` fails.

## Performance Considerations

Negligible. The reader parses two small frontmatter files per resolution; the
lazy composition-root pattern (as in luminosity's `LazyConfigAccess`) is available
to consumers but out of scope here. `serde-saphyr` is pure-Rust with no C backend,
so the plan deliberately adds no `catch_unwind` sandbox (unlike the visualiser's
`serde_yml`); the adversarial-input fixtures (Phase 2 §3, §6) characterise the
parser's behaviour on hostile input rather than asserting a hard fail-safe.

## Migration Notes

None. The bash reader remains the shipping reader; this task adds the native
reader alongside it. Consumer cutover and any mid-migration `config_find_files`
fallback are later work items (0167, 0172).

**The fail-closed guard does not block migrations.** `/accelerator:migrate` is
pure bash: it runs each migration script as a bash child with
`ACCELERATOR_MIGRATION_MODE=1` set inline (`skills/config/migrate/scripts/
run-migrations.sh:632`, `interactive-lib.sh:434,745`), and the only consumer of
that flag is bash `scripts/config-common.sh` (the guard early-return `:56` and the
`config_find_files` legacy fallback `:41`) — neither of which this task touches.
Nothing in the migrate flow or `hooks/` invokes the Rust reader, launcher, or the
example, so the Rust guard is never on the migration path. New users never carry
`.claude/accelerator.md`, so the guard never fires for them; existing legacy-layout
users are correctly directed to `/accelerator:migrate` (bash) once Rust consumers
are cut over. The one scenario the fail-closed choice reserves for later is a
future Rust consumer that must read *during* a migration — that mid-migration
fallback is deferred to 0172, and failing closed (refusing to read a
half-migrated layout) is the safer default until then.

**0172 is a hard prerequisite for any Rust config-consumer cutover that can
execute during a migration.** The fail-closed guard has no `ACCELERATOR_MIGRATION_MODE`
escape hatch, so wiring the Rust reader into a path a migration triggers before
0172 restores the mid-migration fallback would abort the migration or the
consumer. Nothing in this task enforces that ordering (no consumer is wired here),
so it is recorded as an explicit dependency rather than left to reviewer memory.

### Documented behavioural divergences from the bash reader

Two places where the native reader deliberately does **not** match bash, each
pinned by a fixture so the difference is a tested decision, not an accident:

- **Fail-closed on legacy layout even under `ACCELERATOR_MIGRATION_MODE=1`** — the
  bash guard early-returns under that flag; the Rust guard does not (see above).
- **Fail-loud on a malformed frontmatter file** — bash warns and skips a malformed
  file, resolving the other level or the default; the native reader errors the
  whole `get` (`Err(MalformedFrontmatter)`). This trades bash's graceful
  degradation for a loud refusal to read partially-broken config; the cutover
  (0167) owns whatever fallback UX a real consumer wants. Pinned by the
  malformed-frontmatter parity fixture (Phase 2 §6).
- **`.accelerator/` as an extra root marker** — `discover_root` stops at `.jj` or
  `.git` (matching bash `find_repo_root`) **and additionally** at `.accelerator/`,
  which bash never roots on. Harmless in practice (an `.accelerator/`-bearing
  directory is at or below the VCS root), and recorded so the marker-set difference
  is a known constraint for the cutover, not a latent surprise. The non-scalar-node
  found-empty case, by contrast, is now **parity** (matched, not diverged) — see
  Phase 1 §4.
- **Typed value encoding** — because the reader parses YAML into typed scalars, a
  handful of value forms come back differently from bash's opaque literal:
  explicit null tokens (`null`/`~` → `""`), boolean case (`True` → `true`),
  non-canonical numbers (`007`→`7`, `1.0`→`1`, `1e3`, `+5`), and trailing YAML
  comments (stripped, not kept). Each is pinned by a value-encoding fixture (Phase 2
  §7) with its declared result. The cutover (0167) should prefer quote-on-write for
  values a user might type in these forms; recognised keys do not use them today.
- **Block-authored arrays** — an array written in block form (`- item` lines)
  reads found-empty in bash (→ default) but parses to a typed `Value::Sequence` in
  the native reader. Inline-array form (`[a, b]`) is parity; only the block form
  diverges. Pinned by the block-sequence fixture (Phase 2 §6); recognised array
  keys default to inline form.

### Platform scope

Native (non-WSL) Windows is out of scope and inherited-POSIX-only: the parity
oracle and legacy-guard verification shell out to `bash`, the shipping reader this
mirrors is bash, and the store's temp-then-rename write is not atomic-over-existing
on Windows. The pure-Rust `config` core could run anywhere, but the delivered
reader targets the macOS + Linux CI matrix only — the lock-in is acknowledged, not
accidental.

## References

- Original work item: `meta/work/0178-config-crates-native-yaml-reader.md`
- Related research:
  `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md`
- Reference implementation:
  [`cli/config`](https://github.com/atomicinnovation/luminosity/tree/main/cli/config),
  [`cli/config-adapters`](https://github.com/atomicinnovation/luminosity/tree/main/cli/config-adapters),
  [`cli/deny.toml`](https://github.com/atomicinnovation/luminosity/blob/main/cli/deny.toml)
- Hexagon template: `cli/launcher/src/version/core.rs`,
  `cli/launcher/src/main.rs:88-92`,
  `cli/launcher/src/launch/core.rs:167-171`
- Enforcement scaffolding + regression templates: `cli/deny.toml:55-73`,
  `cli/pup.ron:10-39`, `tests/integration/deny/test_native_tls_ban.py`,
  `tests/integration/pup/test_import_rule.py`
- Bash parity target: `scripts/config-read-value.sh`, `scripts/config-common.sh`,
  `scripts/config-defaults.sh`, `scripts/config-dump.sh`,
  `scripts/config-read-review.sh`
- ADRs: ADR-0047 (config model), ADR-0053 (thin CLI over hexagonal core)
