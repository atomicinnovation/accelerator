---
type: "plan"
id: "2026-09-11-0229-per-tracker-pull-scope-configuration"
title: "Per-Tracker Pull Scope Configuration Implementation Plan"
date: "2026-09-11T16:27:02+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0229"
parent: "work-item:0229"
derived_from: ["codebase-research:2026-09-11-0229-per-tracker-pull-scope-configuration"]
tags: ["sync", "scoping", "tracker", "configuration", "discovery", "jira", "linear"]
revision: "60e8f0c5188d4219c128d2af9f6dcef934ae65b1"
repository: "accelerator"
last_updated: "2026-09-18T10:49:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Per-Tracker Pull Scope Configuration Implementation Plan

## Overview

Introduce a per-tracker `pull` configuration block that broadens pull discovery
beyond the single keyed base entity — `additional_teams` / `additional_projects`,
an `all_teams` / `all_projects` flag, a normalised `filters` bag, and configurable
`max_items` / `max_pages` ceilings — each structurally validated at `configure`
time. Broadened discovery is deduplicated by remote identifier and reconciled in a
total, deterministic order (identifier prefix, then numeric sequence). Bounded,
fail-loud ceilings replace the two surviving silent truncations so wider discovery
cannot flood, and a page-cap hit fails loud (a transport-deadline cutoff still degrades
to the transient path). Jira and Linear only; pull-side only.

## Current State Analysis

The story's central seam is real and dormant: `SearchScope.filters`
(`cli/tracker/src/lib.rs:268`) is a wired-but-empty `Vec<(String, String)>` both
clients already lower, and production always builds it empty
(`cli/work-cli/src/sync.rs:888-889`). Promoting it to a validated config surface is
well-founded, but the surrounding machinery is thinner than the story assumes.

- **The config layer cannot surface a structured block.** The resolution `Value`
  (`cli/config/src/service.rs:18-21`) is `Scalar | Sequence` and is
  `#[non_exhaustive]`; `project()` collapses any `Node::Mapping` to `Null`
  (`service.rs:544`). The raw parse tree has `Node::Mapping`
  (`cli/config/src/node.rs:4-9`), but no consumer can read a block as data today.
  There is no merge function — precedence is per-dotted-path last-writer-wins,
  personal over team (`service.rs:476-492`). The catalogue `Default` is
  `Scalar | Seq(scalars)` (`cli/config/src/catalogue.rs:10-29`); no structured
  default is representable.
- **No shape validation exists.** The only value-content check at `configure` is
  the `work.integration` enum membership test
  (`cli/launcher/src/config_command/core/work.rs:29-31`), fail-closed as
  `ConfigError::Invalid` and non-degradable even under `--fail-safe`
  (`cli/launcher/src/config_command/inbound/cli.rs:453-472`).
- **OR-within-a-key is unexercised on both trackers.** Jira builds one single-value
  `Family` per filter pair (`cli/jira-client/src/client.rs:270-277`), so
  `{label:[a,b]}` lowers to two AND'd `IN` clauses (an empty result), not one
  merged `IN`. Linear's `Search` fields are single `Option<String>` with no `in`
  operator (`cli/linear-client/src/filter.rs:82-112`). Filter keys are unvalidated
  — Jira interpolates them raw into JQL (`cli/jira-client/src/jql.rs:213-215`),
  Linear silently drops unknown keys (`cli/linear-client/src/client.rs:719`).
- **`SearchScope` is single-entity, with no dedup or ordering.**
  `discover_untracked` dedups discovered-minus-local only
  (`cli/work-adapters/src/sync/run.rs:526-547`) via `canonical_external_key`
  (`cli/work-adapters/src/sync/create.rs:113-120`); there is no
  discovered-vs-discovered dedup and no ordering (page order preserved).
  `ExternalId` is `String` with no `Ord` (`cli/tracker/src/lib.rs:19-32`);
  identifier parsing is lexical only (`cli/corpus/src/work_item_id.rs:87-109`,
  `cli/corpus-adapters/src/work_item_pattern.rs:434-464`).
- **Ceilings are fixed and cannot represent "unlimited".** `max_pulls` /
  `max_pushes` are `usize` CLI args (default 25, `cli/work-cli/src/cli.rs:277-284`),
  enforced as a post-discovery zero-write refusal
  (`cli/work-adapters/src/sync/run.rs:879-891`). `max_pages` is a `usize` transport
  cap (default 20, `cli/tracker-support/src/transport.rs:14-31`), never sourced from
  config — every production site uses `TransportConfig::default()` (four sites).
- **The pull path already fails loud on incomplete discovery**
  (`run.rs:795-819`, pinned by
  `cli/work-adapters/tests/sync_create.rs:794-827`). Two silent truncations survive:
  the standalone `search` subcommands and the keyed reconcile reads.

The base-scope dependency is settled: **0228 (Layered Configuration Key Model) is
implemented** (its plan `meta/plans/2026-09-10-0228-layered-configuration-key-model.md`
is `done`). `resolve_active_scope_key` (`cli/work-cli/src/sync.rs:702-721`) already
resolves the canonical scope key per active integration
(`jira.project_key` / `linear.team_key`, legacy alias fallback). 0229 layers the
`pull` block on top of this resolved base.

## Desired End State

A `<tracker>.pull` block, valid in team or personal config, broadens a pull's
discovery scope with correctly-validated, bounded, deterministic behaviour:

- Configuring an unsupported filter key, a reserved grouping key (`all` / `any`), a
  malformed ceiling value, or `all_*` together with `additional_*` fails at
  `configure`, fail-loud and non-degradable.
- A personal `<tracker>.pull` block wholly replaces the team block for that tracker
  (no field-level merge).
- A pull emits a per-tracker search targeting the keyed base entity plus each
  configured `additional_*` entity, or every visible entity under `all_*` (an
  explicit enumerated `IN` list, never an unbounded query), with `filters` applied
  keys-AND'd and values-within-a-key OR'd.
- `max_items` bounds the pull-direction write count (tracked-item updates plus
  newly-discovered creates, counted after dedup and local subtraction) and `max_pages`
  bounds transport pagination per operation (a general cap with independent `discovery` /
  `keyed_read` overrides, each defaulting to 50), both config-sourced with an `unlimited`
  sentinel; crossing either errors with zero writes.
- No pull path, standalone `search`, or keyed reconcile read truncates silently on a
  page-cap hit. A transport-deadline cutoff degrades to the transient/indeterminate path on
  the keyed read, but still hard-aborts (loudly) on discovery, differing from a cap-hit only
  in the message.
- The discovered set is deduplicated by remote identifier and reconciled in a total
  order (prefix lexical, then numeric sequence).

Verify by configuring a `pull` block for the active tracker and running
`accelerator work sync --preview` (and `accelerator config dump`), plus the full
automated suite green under `mise run`.

### Key Discoveries

- `SearchScope` is single-entity and `filters` is dormant
  (`cli/tracker/src/lib.rs:257-269`); `resolve_scope` diverges per adapter — Jira
  clones through with a no-target guard (`cli/jira-client/src/client.rs:592-604`),
  Linear substitutes key→UUID with three guards `E_SEARCH_NO_TEAM` /
  `E_SEARCH_UNKNOWN_TEAM` / `E_SEARCH_UNRESOLVED_SCOPE`
  (`cli/linear-client/src/client.rs:674-708`). Every scope change is made twice.
- `Value` is `#[non_exhaustive]` (`cli/config/src/service.rs:16-21`), so adding a
  `Mapping` variant is safe for cross-crate consumers (they already carry a wildcard
  arm); only the three in-crate exhaustive matches must handle it —
  `as_string_sequence` (`service.rs:29-34`), `project` (`service.rs:539-544`),
  `render_value` (`cli/config/src/render.rs:10-14`).
- Whole-block replacement falls out of the existing per-key resolution once a
  subtree is readable: `get` returns personal's `<tracker>.pull` if present, else
  team's (`service.rs:476-492`). No new merge logic is needed.
- Live-enumeration endpoints exist and are already wired for init: Jira
  `discover_projects` over `/rest/api/3/project`
  (`cli/jira-client/src/discovery.rs:49-61`), Linear `list_teams` over
  `teams { nodes }` (`cli/linear-client/src/discovery.rs:54-61`).
- Discovery endpoints and both `resolve_scope` impls are exercised through the
  `RecordingTracker` fake (`cli/tracker-test-support/src/lib.rs`), which offers
  `discovering(found, complete)`, `truncating`, `refusing_scope`, and
  `failing_search` seams.
- Fixture-golden test harnesses exist for both lowering surfaces:
  `cli/jira-client/tests/jql.rs` + `tests/fixtures/jql-composition.txt` (assert JQL
  string), `cli/linear-client/tests/filter.rs` + `tests/fixtures/issue-filter.txt`
  (assert `IssueFilter` JSON). Config rejection is covered by black-box process
  tests in `cli/launcher/tests/config_read.rs` (the `Fixture` harness).
- `config`, `tracker`, `tracker-support`, and `work` are public-API-pinned
  (`tasks/public_api.py:15-29`); changing their surface regenerates a snapshot via
  `mise run public-api:update`. `jira-client` / `linear-client` are not pinned.

## What We're NOT Doing

- **GitHub and other trackers.** Deferred to 0050 under the 0181
  additional-integrations epic. Jira and Linear only.
- **Push-side scope.** Strictly pull discovery; push is untouched. (Phase 4's
  fail-loud reconcile-read abort is run-wide and can halt a bidirectional run, but
  changes no push scope.)
- **Proactive config-time remote-existence validation.** Named-entity existence on
  the remote is 0227's `config validate` (config-time) and this story's sync-time
  resolution. 0229 adds only structural validation at `configure`; it exposes a
  reusable validator that 0227 later calls, but does not implement 0227's command.
- **Full AND/OR filter nesting.** `all` / `any` are reserved and error now
  ("nested filters not yet supported"); the recursive filter type is a future story.
- **Field-level merge of `pull` blocks.** Replacement is whole-block per tracker.
- **Changing the `max_items` default.** Unset `max_items` stays 25. (`max_pages` *is*
  deliberately raised from 20 to 50 for keyed-read safety — see Migration Notes.)
- **Per-operation `max_items` override.** `max_pages` has independent `discovery` /
  `keyed_read` overrides (a general cap plus per-operation overrides). `max_items` stays a
  single aggregate write bound across both paths; a per-operation `max_items` split is
  deferred to a future story with no config migration.
- **New id-allocation or push-time field validation.** Out of scope.

## Implementation Approach

Eight vertically-sliced phases, each independently mergeable and CI-green. Phase 1
lands the config-read foundation the surface phases consume; validation, ceilings,
and fail-loud truncation follow; then the filters and scope-broadening capability;
then dedup/ordering. Every phase is test-driven: a failing test first, minimum code
to pass, refactor under the net.

Two facts constrain sequencing. The ceiling reads share the whole-block replacement
of the `pull` block, so they resolve from the Phase 1 typed parse rather than as
independent leaves. And the two ceiling behaviours are mutually exclusive per run —
a `max_pages` truncation sets `complete: false` and short-circuits
(`run.rs:803`) before the `max_items` count (`run.rs:882`) — so a single run
exercises at most one ceiling error.

Domain naming follows the trackers' own vocabulary at the config surface
(`additional_projects` / `all_projects` for Jira, `additional_teams` / `all_teams`
for Linear) while the port stays entity-neutral (one "whole workspace" boolean plus
a resolved entity list the adapters interpret).

---

## Phase 1: Structured config block read and typed `pull` parse

### Overview

Give the config layer a way to surface a `<tracker>.pull` subtree as structured
data, and parse it into a typed `PullConfig`. Whole-block replacement is inherited
from the existing per-key resolution. No sync behaviour changes yet; the parse is
exercised by tests and surfaced read-only in `accelerator config dump`.

### Changes Required

#### 1. Broaden the resolution `Value` with a mapping variant

**File**: `cli/config/src/service.rs`, `cli/config/src/render.rs`
**Changes**: Add a `Mapping` variant and make `project` build it recursively;
update the three in-crate exhaustive matches.

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(Scalar),
    Sequence(Vec<Scalar>),
    Mapping(Vec<(String, Value)>),
}

pub fn project(node: &Node) -> Value {
    match node {
        Node::Scalar(scalar) => Value::Scalar(scalar.clone()),
        Node::Sequence(items) => scalar_sequence(items)
            .map_or(Value::Scalar(Scalar::Null), Value::Sequence),
        Node::Mapping(mapping) => Value::Mapping(
            mapping
                .entries()
                .map(|(key, child)| (key.clone(), project(child)))
                .collect(),
        ),
    }
}
```

`as_string_sequence` treats a mapping as the empty list; `render_value` renders a
mapping to the empty string (a mapping has no scalar rendering). Cross-crate `match`
sites already carry a wildcard arm under `#[non_exhaustive]`, so they compile
unchanged. Confirm no scalar-path consumer (including the `render_value` parity oracle) is
handed a `Value::Mapping` — `dump` renders blocks via a separate row-group path — so the
empty projection is an unreachable-in-practice default, not a silent swallow of
block-valued keys.

#### 2. Typed `pull` block model and parser

**File**: `cli/work/src/pull.rs` — `PullConfig`, `parse`, and (Phase 2) `validate` all
live in the `work` domain crate, which gains a `config` dependency for the `Value` input
type; `cli/pup.ron` (relax the `work` import allow-list)
**Changes**: A `PullConfig` value object, entity- and tracker-neutral in shape (the
config surface's per-tracker nouns are normalised to `additional_entities` /
`all_entities` at parse time). `max_pages` accepts either a scalar (the general cap for
both operations) or a block with optional per-operation `discovery`/`keyed_read` overrides
— a scalar `n` normalises to `PageCaps { default: Some(n), .. }`. Ceiling fields are parsed
as raw tokens here and interpreted in Phase 3. `parse` maps a `Value::Mapping` into `PullConfig` in `work`;
`config::Value` is pure data (no I/O), so `work` stays free of filesystem/subprocess
concerns while both `configure` (launcher) and the sync path (work-cli) reuse the one
parser via their existing `work` dependency. `work → config` is a new, acyclic edge
(`config` carries only a `kernel` edge — already within `work`'s permitted transitive set —
so the edge is acyclic), but `cli/pup.ron`'s `work_domain_imports_only_permitted` allow-list
currently excludes it (`work` may import only zero-dependency port crates). This phase
deliberately relaxes that boundary: add `^config(::|$)` to the allow-list and revise its
rationale to permit the config model — a conscious loosening so the pure domain crate may
read the config representation, not a mere "confirm". The revised rationale must not restate
the "zero-dependency port crates" wording verbatim, since `config` is not zero-dependency;
note instead that its single `kernel` edge is already permitted to `work`.

```rust
pub struct PullConfig {
    pub additional_entities: Vec<String>,
    pub all_entities: bool,
    pub filters: Vec<(String, Vec<String>)>,
    pub max_items: Option<CeilingToken>,
    pub max_pages: PageCaps,
}

pub struct PageCaps {
    pub default: Option<CeilingToken>,
    pub discovery: Option<CeilingToken>,
    pub keyed_read: Option<CeilingToken>,
}

pub fn parse(block: &Value) -> Result<PullConfig, PullConfigError> { /* ... */ }
```

`parse` is total over shape: it accepts any structurally-valid mapping block and
carries unresolved concerns (filter-key acceptance, ceiling-token validity,
`all`/`additional` exclusivity) into typed fields for Phase 2 to reject. A non-mapping
`Value` (a scalar or sequence, e.g. a typo'd `jira.pull: "oops"`) is a structural shape
error returned as `PullConfigError`, never silently an empty config — otherwise a
malformed personal block would shadow a valid team block under whole-block replacement.
Unrecognised top-level keys are collected into a typed field (not dropped) for Phase 2
to reject. It does not itself validate acceptance — parse and validate stay separate so
both `configure` and the sync path reuse one parser.

#### 3. Read the block with whole-block replacement

**File**: `cli/work-cli/src/sync.rs` (read site), config access via `ConfigAccess`
**Changes**: Resolve the single key `<tracker>.pull` (tracker from the active
integration) through `config.get(key, None)`. A personal block shadows the team
block wholesale because `get` returns the first level where the path resolves.
Absent → an empty `PullConfig` (base-only discovery, today's behaviour).

#### 4. Surface the block read-only in `config dump`

**File**: `cli/launcher/src/config_command/core/dump.rs`,
`cli/launcher/src/config_command/render/`
**Changes**: Render the resolved `<tracker>.pull` block as a read-only row group so
Phase 1 has an observable deliverable. When a personal block shadows a present team
block, annotate the group — the team block is fully overridden, naming the dropped
team-only fields (e.g. `filters`) rather than a bare count — so whole-block replacement is
discoverable rather than inferred from every row reading `local`. When no `pull` block is
configured, render an unset-but-available placeholder listing the accepted fields
(matching the catalogue's `*(not set)*` scalars) so the surface is discoverable before it
is used. The resolution/`PullConfig` layer produces a prepared view (resolved block,
overridden-field list, ceiling meaning like `0 (refuses all)`) that `dump` renders, so the
override and ceiling semantics live in the domain layer, not the renderer. No validation
yet — a malformed block renders as-configured here and is rejected in Phase 2.

### Success Criteria

#### Automated Verification

- [x] Config crate unit tests pass: `cargo test -p config`
- [x] `project` round-trips a nested mapping to `Value::Mapping`: new test in
      `cli/config/src/service.rs`
- [x] The two behaviourally-changed match arms are pinned — `as_string_sequence`
      renders a `Mapping` as the empty list and `render_value` renders it as the empty
      string: new tests in `cli/config/src/`
- [x] `parse` maps a Jira block (`additional_projects`) and a Linear block
      (`additional_teams`) both to `additional_entities`, and an absent block to the
      empty config: new tests in `cli/work/src/pull.rs`
- [x] Whole-block replacement drops team-only fields: team `additional_teams:[X]` +
      `filters:{label:[a]}` with personal `additional_teams:[Y]` and no `filters` resolves
      to Y-only with team `X` *and* `filters` gone (a field-level merge must fail this):
      new test in `cli/work-cli/tests/` or `cli/config` service test
- [x] `parse` rejects a non-mapping block with the shape error, and an unrecognised
      top-level key lands in the typed "unknown keys" field (not silently dropped): new
      unit tests in `cli/work/src/pull.rs`
- [x] `config` and `work` public-api snapshots regenerated and matching (the
      `Value::Mapping` variant and the `PullConfig` value object):
      `mise run public-api:update && mise run public-api:check`
- [x] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [x] `accelerator config dump` shows a configured `pull` block for the active
      tracker, and an unset-but-available placeholder listing the accepted fields when
      unconfigured.
- [ ] A personal `<tracker>.pull` block visibly replaces the team block in `dump`, with
      an annotation naming the overridden team block and the dropped fields.

---

## Phase 2: Configure-time structural validation of the `pull` block

### Overview

Reject a structurally-invalid `pull` block at `configure`, fail-loud and
non-degradable, following the `work.integration` precedent. Validation is a reusable
domain function so 0227's `config validate` can call the same logic later.

### Changes Required

#### 1. Per-tracker filter schema

**File**: `FilterSchema` type in `cli/tracker/src/lib.rs`; per-tracker `const` instances
in `cli/jira-client/src/pull.rs`, `cli/linear-client/src/pull.rs` (new)
**Changes**: The `FilterSchema` type lives in the low `tracker` port crate (both clients
and `work` already depend on it), so `validate` can take `&FilterSchema` without `work`
depending on the client crates. Each client crate declares only its accepted filter keys
as a `const` instance (indicatively `label`, `state`, `assignee`; the concrete set is
fixed here). `FilterSchema` carries only `accepted` for now — a required-key slot is
deferred until a tracker actually needs one and added test-first at that point (per the
repo's TDD/YAGNI rule), keeping the surface to exactly what the validators exercise.

```rust
// `FilterSchema` type defined in `tracker`; instance in each client crate:
pub const FILTER_SCHEMA: FilterSchema = FilterSchema {
    accepted: &["label", "state", "assignee"],
};
```

#### 2. Structural validator

**File**: `cli/work/src/pull.rs` (the `work` domain crate, alongside `PullConfig`)
**Changes**: A pure `validate(&PullConfig, &FilterSchema) -> Result<(), PullConfigError>`
(the `FilterSchema` type from `tracker`) over the effective (post-override) block. It
rejects, each with a distinct error:

- an unsupported filter key (not in `accepted`);
- a reserved grouping key `all` / `any` used as a filter field →
  "nested filters not yet supported";
- a `max_items` value that is neither a non-negative integer nor `unlimited`, or any
  `max_pages` value (the scalar, or `default`/`discovery`/`keyed_read` in the block form)
  that is neither a *positive* integer nor `unlimited` — `max_pages: 0` is rejected because
  a 0-page cap makes the `1..=cap` loop return a silent complete-empty result (marking live
  items absent), unlike `max_items: 0` which meaningfully refuses all writes; and an
  unrecognised key under the `max_pages` block;
- `all_entities` set together with a non-empty `additional_entities`;
- an unrecognised top-level block key, including a wrong-tracker noun (e.g.
  `additional_teams` under an active Jira integration), against the per-tracker
  accepted-key set — and when the offending key is the *other* tracker's known noun, the
  message adds a targeted hint ("`additional_teams` is a Linear key; this integration is
  Jira — did you mean `additional_projects`?").

Each error follows the existing `bad_integration` precedent
(`config_command/core/work.rs:53-62`): it names the offending value/key, enumerates the
accepted set (accepted filter keys; for `max_items`, a non-negative integer — `0` refuses
all — or `unlimited`; for `max_pages`, a positive integer or `unlimited`, since `0` is not
a valid page cap; "remove one of `all_*`/`additional_*`"), and names the config level where
the effective block resolved
(team `.accelerator/config.md` vs personal `.accelerator/config.local.md`, via
`Level::filename()`) — when a personal block shadows a team block, the personal file, not
`bad_integration`'s hardcoded team path.

Validation is structural only; remote existence of named entities is out of scope
(0227 / sync-time).

#### 3. Wire validation into the config command and the sync read path

**File**: `cli/launcher/src/config_command/core/`, dispatched from
`cli/launcher/src/config_command/inbound/cli.rs`; `cli/work-cli/src/sync.rs` (sync read
site)
**Changes**: On a `pull`-touching config read/dump, resolve and validate the block
for the active tracker; map a `PullConfigError` to `ConfigError::Invalid` so it
routes to `Failure::Refusal` — fatal even under `--fail-safe`. Pick the tracker's
schema by dispatching on `work.integration`. Run the same `validate` on the sync read
path before any consumer (ceilings, filters, scope) touches the block, so a `work sync`
with an invalid `pull` block fails loud before discovery — mirroring `configure` and
closing the raw-JQL key path (Jira interpolates filter keys raw, `jql.rs:213-215`). The
Phase 1 `parse` never reaches lowering unvalidated.

### Success Criteria

#### Automated Verification

- [x] Validator unit tests cover each rejection branch and the accepting case:
      `cargo test -p work pull`
- [x] Ceiling-token classes are enumerated as separate validator cases — accept `0` for
      `max_items` (refuse-all) but reject `0` for `max_pages`, accept `unlimited`, reject
      negative, reject float, reject non-numeric: new unit tests in `cli/work/src/pull.rs`
- [x] Black-box rejection tests pass (unsupported filter key, reserved `all`/`any`, bad
      ceiling, `all_*` with `additional_*`, unrecognised top-level key, wrong-tracker
      noun, non-mapping block), each asserting non-zero exit and an actionable message
      (offending value, accepted set, config file) following the `bad_integration`
      precedent, plus a `--fail-safe` twin proving non-degradation: new cases in
      `cli/launcher/tests/config_read.rs`
- [x] A `work sync` with an invalid `pull` block fails loud before discovery (not only
      `configure` rejects it): new test in `cli/work-cli/tests/`
- [x] `work` and `tracker` public-api snapshots regenerated and matching (the `validate`
      function and the `FilterSchema` type):
      `mise run public-api:update && mise run public-api:check`
- [x] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [x] `accelerator config dump` with an unsupported filter key exits non-zero and
      names the offending key.
- [x] A block with both `all_projects` and `additional_projects` is rejected at
      `configure`, not deferred to sync.

---

## Phase 3: `unlimited` sentinel and config-sourced ceilings

### Overview

Make `max_items` and `max_pages` config-sourced from the `pull` block, with an
`unlimited` sentinel. Unset `max_items` keeps today's 25; `max_pages` is a general
per-operation page cap with optional `discovery`/`keyed_read` overrides, defaulting to
`Bounded(50)` for both — raised from today's 20 so the keyed reconcile read does not
cap-abort on organic corpus growth. `max_items` supplies the default for the reconcile
bound; `max_pages` for the transport caps.

### Changes Required

#### 1. A ceiling value object

**File**: `Ceiling` type in `cli/tracker/src/lib.rs` (the port crate `work-adapters` and
both clients already depend on in production, avoiding a new engine→transport dependency
edge); used by `cli/tracker-support/src/transport.rs` and `cli/work-adapters`
**Changes**: A named type replacing the bare `usize` at both ceilings, deriving the
traits `TransportConfig` relies on so its pinned `Copy`/`Eq` surface is retained.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {
    Bounded(usize),
    Unlimited,
}
```

`TransportConfig` carries two `Ceiling` caps — a discovery cap and a keyed-read cap —
replacing the single bare-`usize` `max_pages`; the paging loops treat `Unlimited` as no cap
(never setting `complete: false` / `truncated` on page exhaustion). The four
`for page in 1..=cap` loops (Jira `discover`/`fetch_chunk`, Linear
`page_all`/`search_detailed`) become unbounded under `Unlimited`, still bounded by the
transport deadline; confirm each breaks unconditionally on cursor exhaustion (empty/None
cursor) before the `1..=cap` bound is removed, so termination under `Unlimited` rests on
results exhaustion plus the deadline, not a misbehaving cursor. Siting `Ceiling` in
`tracker` adds a `tracker-support → tracker` edge — acyclic (`tracker` has no
dependencies); confirm no cargo-pup import rule forbids it. Retyping the transport caps
updates every `TransportConfig { .. }` literal (~15 across both client sources and their
tests) — enumerate them. A single token→`Ceiling` conversion is the sole authority on a
valid ceiling string: Phase 2 validation and this interpretation both call it, so a token
can never pass validation yet fail to interpret. The conversion itself rejects a page-cap
`Bounded(0)` (returning `PullConfigError`), so no entry point that skips the Phase 2
validator — a hand-edited config, or the standalone `search` path — can hand a 0-page cap
to a paging loop.

`max_pages` resolves per operation from `PageCaps`: the discovery cap is
`discovery ?? default ?? Bounded(50)` and the keyed-read cap is
`keyed_read ?? default ?? Bounded(50)` — a scalar `max_pages` (or the `default` block key)
sets both, a per-operation override sets one, and both fall back to `Bounded(50)` when
unset. Each paging loop takes its resolved `Ceiling` as a call argument (Linear's
`page_all` is shared by discovery `search` and the keyed reconcile read, so it cannot read
a single `TransportConfig` field). `max_items` stays a single
aggregate write bound across both paths (no per-operation split).

#### 2. Source the transport cap from config

**File**: `cli/jira-cli/src/context.rs`, `cli/linear-cli/src/context.rs` (client
construction sites)
**Changes**: Resolve the discovery and keyed-read caps from `<tracker>.pull.max_pages`
(`PageCaps`) and construct `TransportConfig` with both instead of
`TransportConfig::default()` — discovery/`search` reads the discovery cap, the keyed
reconcile read the keyed-read cap. Each unset cap resolves to a single named
`DEFAULT_MAX_PAGES` (`Ceiling::Bounded(50)`), not a re-typed literal.

#### 3. Source the reconcile bound from config

**File**: `cli/work-cli/src/sync.rs`, `cli/work-adapters/src/sync/run.rs`
**Changes**: `SyncRequest.max_pulls` becomes `Ceiling`, defaulted from
`<tracker>.pull.max_items`. Both the `--max-pulls` and `--max-pushes` CLI flags become
`Option<usize>` with no clap default, so a flag's *presence* — not its value — signals
an override; today's `default_value_t = 25` cannot distinguish "passed `25`" from
"unset". For `--max-pulls`: `Some(n)` beats config, `None` resolves to
`<tracker>.pull.max_items`, and an absent ceiling resolves to a single named
`DEFAULT_MAX_ITEMS` (`Bounded(25)`), not a re-typed literal. For
`--max-pushes`: `Some(n)` overrides, `None` resolves to `Bounded(25)` — push gains no
config source (push-side scope stays a non-goal); only the flag's default-vs-unset
ambiguity is fixed, for symmetry. The refusal in `prepare_run` compares the pull
ceiling against `Ceiling`, treating `Unlimited` as no bound. `max_items` bounds the same
quantity `max_pulls` bounds today — `plan.pull_count() + untracked.len()` (tracked-item
pull updates plus newly-discovered creates), counted after the Phase 8 dedup and the
discovered-minus-local subtraction — so the config key denotes pull-direction writes,
not raw discovery volume. `max_items` accepts `0` as refuse-all — the opposite of
`unlimited` — so the documentation and the bad-ceiling error contrast `0` (refuse all)
with `unlimited` (no bound), and `config dump` surfaces the resolved meaning (e.g.
`0 (refuses all)`). `max_pages: 0` is rejected at validation (a 0-page cap would silently
return an empty complete result, not refuse), so the `0`/`unlimited` contrast applies to
`max_items` only. Update the existing runtime refusal message (`RunError::Refused`, today
"raise the limit with `--max-pulls`/`--max-pushes`") to also name the resolved
`<tracker>.pull.max_items` key, the config level/file it resolved from, and that
`unlimited` lifts the bound. Retyping `SyncRequest.max_pulls`/`max_pushes` to `Ceiling`
ripples through the `execute`/`run_sync` test helpers
(`cli/work-adapters/tests/sync_run.rs`, `sync_create.rs`) that pass bare `usize` —
enumerate those call sites alongside the `TransportConfig` literals. Since dropping
`default_value_t` removes the `[default: 25]` hint from `--help`, update the
`--max-pulls`/`--max-pushes` doc-comments to state the precedence (flag present overrides
`<tracker>.pull.max_items`; unset defers to config, else 25) and that `--max-pushes` has
no config source.

### Success Criteria

#### Automated Verification

- [ ] `max_items: 3` refuses a 5-issue discovery whereas the default 25 completes:
      new test in `cli/work-adapters/tests/sync_run.rs` (via `RecordingTracker`)
- [ ] A config-sourced `<tracker>.pull.max_items: 0` refuses a 1-issue discovery with zero
      writes, distinct from `unlimited` completing and the default 25 completing: new test
      in `cli/work-adapters/tests/sync_run.rs`
- [ ] `max_pages: 5` reaches the cap where the default 50 completes: new client/transport
      paging test (not the page-less `RecordingTracker`, which models no pagination)
- [ ] A `keyed_read` override sets the keyed-read cap independently of `discovery` (and
      vice versa) — a low `keyed_read` cap-aborts the reconcile read while `discovery`
      completes, and the reverse: new client/transport paging tests
- [ ] `Ceiling::Unlimited` never truncates a page set larger than the default cap: new
      client/transport paging test
- [ ] `max_items: unlimited` completes a discovery larger than the default 25: new test
      in `cli/work-adapters/tests/sync_run.rs` (the fake is adequate for `max_items`)
- [ ] A general `<tracker>.pull.max_pages` plus a per-operation override each reach the
      correct `TransportConfig` cap (discovery vs keyed-read), not `TransportConfig::default()`:
      new wiring test in `cli/jira-cli/` / `cli/linear-cli/`
- [ ] `--max-pulls <n>` (present) overrides `<tracker>.pull.max_items`; `--max-pulls`
      unset (`None`) defers to the configured `<tracker>.pull.max_items`: new tests in
      `cli/work-cli/tests/`
- [ ] `--max-pushes` unset resolves to the built-in `Bounded(25)` unchanged (the flag
      becomes `Option<usize>` with no behavioural change to push): new test in
      `cli/work-cli/tests/`
- [ ] The refusal message names the resolved `<tracker>.pull.max_items` key and the config
      level (team vs personal) it resolved from — the personal file when a personal block
      shadows: new test
- [ ] `tracker` and `tracker-support` public-api snapshots regenerated (the `Ceiling`
      type and the `max_pages` retype): `mise run public-api:update && mise run public-api:check`
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] A configured `max_items` below the discovery count refuses a preview with
      zero writes; `unlimited` lifts the bound.

---

## Phase 4: Fail-loud on the two surviving silent truncations

### Overview

Promote the two remaining silent truncations to hard errors, with the now-configurable
`max_pages` as the release valve. The pull path already refuses on incomplete
discovery; this phase covers the standalone `search` subcommands and the keyed
reconcile reads. The untracked-pull discovery still hard-aborts on *both* a cap-hit and a
deadline — it has no per-id degrade path (unlike the keyed read), so "degrade to the
transient path" has no meaning here; the tri-state cause only changes the message: the
`DiscoveryIncomplete` message (today "Scope the search to a single project or team") names
`<tracker>.pull.max_pages`, the config file, and the `unlimited` valve on a cap-hit, and
reports incomplete-transient on a deadline, without blaming the cap.

### Changes Required

#### 1. Standalone `search` subcommands

**File**: `cli/linear-cli/src/main.rs`, `cli/jira-cli/src/main.rs`, the *per-binary*
exit-code taxonomies (`cli/jira-cli/src/exit_codes.rs`, `cli/linear-cli/src/exit_codes.rs`
— each parity-pinned; pick a shared cap-hit integer off the reserved 70–74 dispatch band
and clear of the existing `SEARCH_*` codes);
`skills/integrations/jira/search-jira-issues/SKILL.md`,
`skills/integrations/linear/search-linear-issues/SKILL.md`
**Changes**: Both standalone `search` subcommands honour the discovery `max_pages` cap
consistently — they paginate internally up to it (Jira gains an internal loop; it issues a
single `--limit` page today), exit-0 on completing within the cap, and exit non-zero with a
dedicated cap-hit code on a genuine cap-hit. The resume cursor is preserved in the JSON
alongside the non-zero exit so explicit `--page-token` resumption and machine detection
still work — Linear keeps its existing `truncated` field, while Jira (which has no
`truncated` field today, only `outcome` + `nextPageToken`) gains an explicit completeness
field. Because Jira now merges pages internally up to the cap, specify the new envelope
(merged results; the cursor appears only on a cap-hit) and rewire the skill's `--page-token`
guidance to the cap-hit-only resume model. A cap-hit is distinguished from a transient deadline/wire cutoff: the message
names the discovery cap (`<tracker>.pull.max_pages`, or its `discovery` override), the
config file, and the `unlimited`/higher-value valve only on a cap-hit, and reports
incomplete-transient otherwise rather than mis-blaming the cap. The dedicated cap-hit exit
code is distinct from credential/transport failures so consumers can branch on it. Add both
search skills to this phase: `search-jira-issues` today maps *any* non-zero exit to a
credential error and `search-linear-issues` treats `truncated` as a normal exit-0 flow —
update both to honour/surface `max_pages` and recognise the cap-hit code with the
`max_pages`/`unlimited` remedy instead.

#### 2. Keyed reconcile reads

**File**: `cli/jira-client/src/client.rs` (`fetch_all` / `fetch_chunk`),
`cli/linear-client/src/client.rs` (`fetch_all` / `page_all`),
`cli/tracker/src/lib.rs` (`FetchOutcome`), reconcile path in
`cli/work-adapters/src/sync/`, the exit-code taxonomy in `cli/work-cli/src/exit_codes.rs`;
`skills/.../sync-work-items/SKILL.md`
**Changes**: Neither keyed path can distinguish truncation from a transient failure
today: Jira's `fetch_chunk` returns one `Err(String)` for both the page-cap hit and
every transient failure (deadline, non-2xx, non-JSON, transport), folded into
`indeterminate`; Linear's `page_all` collapses cap-hit, deadline, and wire failure
into one `truncated: Option`, and separately routes an out-of-scope id
(`in_scope(id) != Some(true)`) into `indeterminate`. The abort needs a typed signal
first.

Introduce a structured cap-hit signal distinct from a transient `TrackerError`: split
Jira's `fetch_chunk` error so a `max_pages` cap-hit is its own variant; separate
Linear's `page_all` cap-hit from deadline/wire failure; and carry a tri-state completeness
signal (`complete` / `cap-hit` / `transient`) on the keyed-read outcome (`FetchOutcome`) —
and the same tri-state on the discovery result (`Discovery`) — so each path can tell a
cap-hit truncation from a genuine out-of-scope `indeterminate` and from a transient/deadline
error. A boolean would conflate cap-hit with deadline. The keyed read is chunked (Jira, 50
ids/chunk) and multi-team (Linear, one `page_all` per catalogued team, §5), so several
sub-reads each yield a signal; the fold to one `FetchOutcome` is **cap-hit-dominant** — a
`cap-hit` in any chunk/team wins over a co-occurring `transient` (so a mixed outcome
deterministically aborts rather than silently degrading), and `transient` in turn wins over
`complete`.

Then route only a cap-hit truncation to a hard abort surfaced through the engine
(all-or-nothing, zero writes), distinct from a genuine `absent`. Thread it as a distinct
hard-abort field on `GatheredFacts` (separate from the existing soft `read_failure`
channel, which is carried and never fatal) that `prepare_run` maps to a zero-write
`RunError` before any planning; a cap-hit also empties `absent` (unseen ids →
indeterminate) so the "`absent` only from a provably-complete retrieval" invariant holds
even before the abort is consulted. A transient read failure — including deadline expiry —
remains a transient error, not this abort, and the out-of-scope `indeterminate` branch
keeps its current non-aborting `Action::Noop`. A deadline-driven incompleteness surfaces a
soft signal on the run report so an operator can see the read was budget-limited, rather
than passing entirely silently. To keep `prepare_run` legible as these channels accrete,
centralise the fold and the cap-hit→zero-write mapping in one named `classify_completeness`
step feeding a single decision point (rather than adding another parallel field checked
inline), and pin the routing — soft-vs-hard, cap-hit-vs-transient-vs-out-of-scope — with a
table-style test. `FetchOutcome`/`tracker` is public-API-pinned, so this regenerates the
`tracker` snapshot (`mise run public-api:update`).

⚠️ This trades availability for fail-loud behaviour on the keyed path: a cap-hit keyed
read now aborts the whole sync — including any queued pushes — rather than proceeding
with the affected items marked awaiting-human. The run-wide scope is by design: the
keyed reconcile read feeds both pull and push planning inside `prepare_run`, so a
truncated read leaves the un-read items' remote state untrustworthy for both directions,
and aborting is safer than pushing against unknown remote state. It changes no push
*scope* (the "push untouched" non-goal concerns push discovery, not the shared
reconcile read's completeness), and because the keyed read feeds push too, `--push-only`
is not a safe bypass and none is added. The abort names the keyed-read cap
(`<tracker>.pull.max_pages`, or its `keyed_read` override), the config level/file it
resolved from, and the page count reached, and that a higher value or `unlimited` lifts the
cap; a corpus that outgrows the cap fails every sync until an operator raises it —
the intended fail-loud, not a regression. The abort exits with a dedicated, documented exit
code (distinct from the exit-4 indeterminate code); add `sync-work-items` to this phase to
document the new code, its `max_pages` remedy, and that `--push-only` does not bypass it.
The abort is deliberately complete — it also halts create-from-local items (which carry no `external_id`, are absent from the
keyed-read set, and are technically unaffected by a truncated read) — chosen for a simple
all-or-nothing guarantee over a narrower carve-out. At the code level no path deletes a
local file today (both `Indeterminate` and `RemoteAbsent` are `Action::Noop`), so this
changes abort-vs-proceed, not deletion safety.

### Success Criteria

#### Automated Verification

- [ ] A standalone `search` (Jira and Linear) that cap-hits the discovery `max_pages` exits
      non-zero with the cap-hit code; one completing within the cap exits zero: new tests in
      `cli/jira-cli/tests/` and `cli/linear-cli/tests/`
- [ ] Both standalone searches honour `<tracker>.pull.max_pages` — Jira paginates internally
      to the cap rather than issuing a single page: assertion in the Jira search test
- [ ] Update `search_echoes_the_envelope_and_audits_the_jql` to expect a non-zero exit on a
      cap-hit, keeping a within-cap (complete) control asserting exit-zero as a negative
      control against always-abort: `cli/jira-cli/tests/flow_search.rs`
- [ ] A truncated standalone `search` still emits the `truncated` JSON field alongside the
      non-zero exit: assertions in the Linear/Jira search tests
- [ ] The cap-hit truncation and the keyed-read abort each use a dedicated exit code
      distinct from credential/transport/exit-4-indeterminate codes: exit-code unit tests
- [ ] The updated `search-jira-issues`/`search-linear-issues`/`sync-work-items` skills
      branch on the cap-hit code and surface the `max_pages`/`unlimited` remedy (not the
      credential path); `sync-work-items` documents the abort code and the no-`--push-only`
      note: skill-doc review / eval
- [ ] A cap-hit keyed reconcile read aborts the sync with zero writes: new test in
      `cli/work-adapters/tests/sync_run.rs` via a new completeness/cap-hit seam that models
      `complete: false` on the keyed read and populates the new `FetchOutcome` completeness
      flag (distinct from the existing `RecordingTracker::truncating`→`indeterminate` path,
      whose inline `fetch_all` must also gain the flag)
- [ ] A transient keyed-read failure still degrades to indeterminate and the sync
      proceeds (not aborted): new test in `cli/work-adapters/tests/sync_run.rs`
- [ ] A genuine out-of-scope indeterminate still proceeds with the watermark held:
      update `an_indeterminate_items_watermark_is_left_unadvanced` so it and the new
      cap-hit abort test both pass
- [ ] Existing incomplete-discovery pull refusal still passes:
      `cargo test -p work-adapters an_incomplete_discovery_is_refused_with_guidance`
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] `accelerator jira search` / `accelerator linear search` on a result set past
      the cap exits non-zero and names `max_pages`.
- [ ] A sync whose keyed read truncates aborts rather than reporting the item as
      awaiting-human.

---

## Phase 5: Filters end-to-end (OR-within-a-key)

### Overview

Lower the `filters` bag so keys are AND'd and values within a key are OR'd (`IN`) on
both trackers, and wire the parsed `PullConfig.filters` into the search scope so a
configured filter actually reaches discovery.

### Changes Required

#### 1. Jira: merge same-key values into one `IN` family

**File**: `cli/jira-client/src/client.rs` (filters→families), `cli/jira-client/src/jql.rs`
**Changes**: Group `SearchScope.filters` by key into one `Family` per key so
`{label:[a,b]}` lowers to one multi-value `IN`, not two AND'd clauses. `family_clauses`
already emits a multi-value `IN`; the fix is the grouping at construction. Define the
config-key→JQL-field mapping explicitly (config `label`/`state`/`assignee` → the JQL
fields actually emitted, e.g. `labels`/`status`/`assignee`) rather than interpolating the
config key verbatim; the golden asserts the emitted field names, not the config keys. As
defence in depth beside the upstream key allow-list, the Jira composer asserts each
interpolated field is a safe identifier (alphanumeric / `customfield_NNNNN`), rejecting
anything else with `BadJql`, so the `format!` field position is guarded at the sink. This
is the first production use of the filter-value seam, so ground `quote()`'s escape strategy
in Atlassian's documented JQL string grammar rather than assuming SQL-style quote-doubling:
JQL escapes quotes with a backslash (`\'`), so today's `'`→`''` plus an ad-hoc backslash
rule could still mis-parse at the remote. A fixture golden pins only the string *we emit*,
not that Jira treats a hostile value as inert — so add a client contract/integration check
that a break-out payload (interior `'`, trailing `\`, ` OR `) returns the same single,
contained result set, not just an emitted-string assertion.

#### 2. Linear: an `in`-capable lowering

**File**: `cli/linear-client/src/filter.rs`, `cli/linear-client/src/client.rs`
**Changes**: Let a filter key carry multiple values and lower to the Linear `in`
operator (e.g. `labels: { name: { in: [...] } }`), keys AND'd in the single
`IssueFilter` object. A single value keeps its current `eq` form. The golden harness's
`parse_spec`/`Search` (single `Option<String>` fields) needs a grammar extension before
a multi-value row can be expressed in the golden; until then the through-the-seam
coverage lives in the client contract test. Confirm Linear's `IssueFilter` supports `in`
on each targeted field (`labels.name`, `state.id`, and — for Phases 6–7 — `team.id`) via
a client contract/integration check before relying on it; golden fixtures assert only
what we emit, not that the remote accepts it.

#### 3. Wire filters into the scope

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Flatten `PullConfig.filters` into `SearchScope.filters` at the
construction site (`sync.rs:882-890`), replacing the hard-coded `Vec::new()`.

### Success Criteria

#### Automated Verification

- [ ] Jira fixture golden: config `{label:[a,b], state:[open]}` composes to the emitted
      JQL under the config-key→field mapping (e.g. `status = 'open' AND labels IN ('a',
      'b')`), asserting the mapped field names, not the config keys: new rows in
      `cli/jira-client/tests/fixtures/jql-composition.txt`
- [ ] Adversarial Jira filter-value goldens — a value with an interior `'`, `)`, ` OR `,
      and a trailing `\` composes without breaking out of its clause; confirm `quote()`
      handles backslash (extend it if not): new rows in
      `cli/jira-client/tests/fixtures/jql-composition.txt`
- [ ] Linear fixture golden: the same filters compose to an `IssueFilter` with
      `state` `eq` and `labels` `in [a, b]`: new rows in
      `cli/linear-client/tests/fixtures/issue-filter.txt`
- [ ] A configured `filters` bag reaches `SearchScope.filters`: new engine test
      asserting the recorded `Call::Search` scope
- [ ] A flat multi-value `SearchScope.filters` (`[(label,a),(label,b),(state,open)]`, as
      production builds it) drives `client.search(&scope)` to a wire query grouping
      same-key values into one `IN` (`label IN ('a','b')` /
      `labels: { name: { in: [...] } }`): new client contract tests (MockServer harness)
      exercising the grouping-at-construction seam the goldens bypass
- [ ] Linear's `IssueFilter` accepts `in` on `labels.name`/`state.id` (and `team.id` for
      Phases 6–7): client contract/integration check against the live schema
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] A pull with a configured `filters` bag returns only issues matching the
      AND/OR semantics on each tracker.

---

## Phase 6: `additional_*` multi-scope discovery

### Overview

Carry a resolved multi-entity scope (base plus `additional_*`) on `SearchScope`, and
have each adapter emit an explicit `IN` list. Named entities are resolved against the
credential's live-enumerated visible set at pull time; an entity the credential
cannot see aborts the pull, distinct from a transient enumeration failure.

### Changes Required

#### 1. Port change: an exclusive entity-scope sum type

**File**: `cli/tracker/src/lib.rs`
**Changes**: Replace `SearchScope`'s flat `project: Option<String>` with an exclusive
entity-scope sum type so illegal combinations are unrepresentable — e.g. `enum
EntityScope { Keyed { base: Option<String>, additional: Vec<String> }, WholeWorkspace }`
alongside `filters`. The `Keyed` variant broadens the base with `additional` (never
narrows); `WholeWorkspace` subsumes the base entirely. The old `project`-wins precedence
becomes structural — there is no `project` field left to coexist with a whole-workspace
flag — rather than a documented convention enforced at the construction site. This is a
breaking change to the public-API-pinned `tracker` surface (larger than an additive
field), so every `SearchScope` construction and both adapters' scope reads move to the
enum; the Phase 6 snapshot regen covers it. `TrackerError` is left unchanged: a genuine
entity miss reuses the existing `RunError::DiscoveryUnconfigured` (exit-74) channel
(see §2), so the deliberately-closed `TrackerError` enum gains no variant and no new exit
code enters the taxonomy.

#### 2. Resolve entities pre-search, then emit an enumerated `IN` list

**File**: shared resolver in `cli/work-adapters/src/sync/` (the engine drives it as a
pre-search step via a new dyn-compatible `RemoteTracker::enumerate_visible_entities` port
method — a pinned-`tracker` addition, covered by the Phase 6 snapshot regen; only the value
types `Ceiling`/`EntityScope` and this method live on the `tracker` port);
`cli/jira-client/src/client.rs` + `jql.rs` + `discovery.rs`,
`cli/linear-client/src/client.rs` + `filter.rs` + `discovery.rs`
**Changes**: Resolve the scope entities *before* `search`, in one shared unit both
trackers drive: it enumerates the credential's visible entities, confirms the base and
each additional entity are members, and returns the resolved entity list (or an abort).
`resolve_scope` stays pure and `search` receives the already-resolved list and only lowers
it. A genuine miss maps to the existing `RunError::DiscoveryUnconfigured` (exit-74) — the
same channel a base-key miss already uses, so the closed `TrackerError` enum gains no
variant and no new exit code enters the taxonomy; a transient enumeration failure stays a
transient `TrackerError::Retryable`. Each adapter implements `enumerate_visible_entities`
(Jira over `/rest/api/3/project`, Linear over `list_teams`), its key→identifier step (Jira
lowers project keys directly; Linear resolves each key to its UUID from the live
enumeration), and its string emission (`project IN (...)` / `team in [...]`), each entity
composed through the existing `quote()` on Jira (matching the base `project =` clause,
never a naive string join) and emitted as a structured `IssueFilter` `in` list bound as a
GraphQL variable on Linear. The enumeration is paginated to exhaustion — Linear's
`list_teams` is a single unpaginated query today, so it gains `first:`/`after:`/`pageInfo`
cursor-following, while Jira's `/rest/api/3/project` returns all projects in one array — and
an incomplete enumeration routes to the same fail-loud abort as a capped discovery, never a
silent subset (for `all_*`) or a false not-visible abort (for a named `additional_*` beyond
the first page). Single-sourcing the resolver keeps the membership/abort rule from drifting
between the two adapters.

#### 3. Grow the committed entity index on pull

**File**: `cli/linear-client/src/catalogue.rs` (multi-entry `CatalogueTeam`),
Jira `projects.json` writer in `cli/jira-client/`
**Changes**: Extend the single-team `CatalogueTeam` to a multi-entry map and grow
`catalogue.json` / `projects.json` as a pull imports items from new entities. This
backs offline resolution/validation of local items; discovery-time resolution uses
the live enumeration above. The index grows only for entities from which items are
actually imported (never the full enumerated set), so `all_*` does not commit metadata
for the whole visible workspace. The index write is ordered so it cannot outrun the
import — grow an entry only after the corresponding item is authored (or write the index
once at run finalisation) — so an interrupted broadened pull leaves the index and the
corpus consistent. Preserve the existing single-entity fields (`/team`, Jira's current
`projects.json` shape) and add the multi-entry collection under a new key, so an older
plugin binary reading a newer file degrades gracefully; perform the whole read-merge-write *inside a single* `with_lock` closure, not a bare
`AtomicWrite` and not a read-then-locked-write. The existing writers
(`LinearCache::write_catalogue` / `JiraCache::write_discovery`) lock only the write of a
pre-formed shape, so a read-merge outside the lock still lets two concurrent broadened pulls
each read the pre-growth index and the second clobber the first's entry — extend the writer
to take a merge closure (or add a lock-scoped read-modify-write helper) so the read, merge,
and write are atomic, keeping Jira's `.cache-version` marker consistent. The merge preserves
`workflowStates` and the base team rather than rewriting. When `all_*` first commits
metadata for previously-unseen entities, emit an operator-visible warning naming them,
since the committed index inherits repo-wide visibility.

⚠️ Reconciliation note: the work item's Technical Notes describe Linear
`additional_teams` resolution as catalogue-backed, but the 2026-09-11 research
walkthrough supersedes this — resolution is live enumeration at pull time, with the
committed catalogue growing lazily for offline use. This plan follows the research
decision.

#### 4. Wire base and `additional` into the scope

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Build the `Keyed { base, additional }` variant from the resolved base
entity and `PullConfig.additional_entities`.

#### 5. Broaden the Linear keyed reconcile read to imported teams

**File**: `cli/linear-client/src/client.rs` (`fetch_all` / `page_all` / `in_scope`)
**Changes**: The Linear keyed reconcile read pages only the base team today
(`team_search { team_id: base }`) and `in_scope` accepts only the base `team_key` prefix,
so an item imported from an `additional_*`/`all_*` team would fall to `indeterminate` on
every later sync and never reconcile. Page every team the corpus spans — drive `page_all`
per team in the committed catalogue (§3) and widen `in_scope` to accept any catalogued
team's prefix — so imported additional-team items reconcile like base-team items. Jira is
unaffected (project-agnostic `key IN (...)`). Paging N teams multiplies the keyed-read page
count against the same cap — see the keyed-read page-budget note in Phase 4 / Phase 3 §1.

### Success Criteria

#### Automated Verification

- [ ] With `additional_*` configured, the emitted search targets base plus each
      additional entity: Jira/Linear fixture goldens for the `IN` list
- [ ] An entity containing a quote/paren is safely escaped in the emitted
      `project IN (...)` list (no clause break-out): new row in
      `cli/jira-client/tests/fixtures/jql-composition.txt`
- [ ] A named `additional_*` (or base) entity absent from the visible set aborts the
      pull via the existing `RunError::DiscoveryUnconfigured` (exit-74), the same channel
      as a base-key miss (zero writes); a transient enumeration failure routes to the soft
      `Retryable` path instead: new tests via the shared resolver + an enumeration seam
- [ ] `catalogue.json` gains a newly-imported team on pull: new test in
      `cli/linear-client/tests/`
- [ ] A synced `additional_*`-team item reconciles on a later run (remote edits pulled)
      rather than sticking at `indeterminate`: new `linear-client` contract test exercising
      per-team `page_all` + `in_scope` widening (not `RecordingTracker`, which models no
      per-team scope)
- [ ] Linear enumeration paginates to exhaustion — a visible-team set exceeding one page is
      fully enumerated so `all_teams` does not under-scope and a first-page-beyond
      `additional_teams` is not falsely aborted; an incomplete enumeration fails loud: new
      `linear-client` contract test
- [ ] `tracker` public-api snapshot regenerated (the `SearchScope` sum type and the new
      `enumerate_visible_entities` port method):
      `mise run public-api:update && mise run public-api:check`
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] A pull with `additional_projects` / `additional_teams` discovers issues from
      the base entity and each additional entity.
- [ ] A misspelled additional entity aborts the pull with a clear message.

---

## Phase 7: `all_*` whole-workspace discovery

### Overview

Under `all_teams` / `all_projects`, enumerate the credential's visible entities live
and emit an explicit enumerated `IN` list covering the whole workspace — never an
unbounded, constraint-free query. The base entity is subsumed by the enumerated set.

### Changes Required

#### 1. Construct the whole-workspace variant

**File**: `cli/work-cli/src/sync.rs`
**Changes**: When the effective block sets `all_entities`, build the scope as the
`WholeWorkspace` variant; the enumerated whole-workspace set subsumes the base entity.
Exclusivity is structural — `WholeWorkspace` carries no base or additional entities — so
the old `project`-wins collision cannot arise.

#### 2. Adapters enumerate and emit the full `IN` list

**File**: `cli/jira-client/src/client.rs` + `jql.rs`,
`cli/linear-client/src/client.rs` + `filter.rs`, the unbounded-write gate in
`cli/work-adapters/src/sync/` + `cli/work-cli/src/cli.rs` (`--allow-unbounded`);
`skills/.../sync-work-items/SKILL.md`
**Changes**: For the `WholeWorkspace` variant, the adapter enumerates visible entities
and emits `project IN (<all>)` / `team in [<all>]`, bounded only by `max_items` /
`max_pages`, reusing the Phase 6 shared resolver's enumeration and string emission
(every visible entity is a member, so membership confirmation is a no-op). Linear's
whole-workspace path relaxes the two flood-guards under the variant while still emitting
an enumerated list (not the empty filter 0220 forbids). An empty `Keyed` variant (no
base, no additional) still refuses via the unkeyed guard, and `WholeWorkspace` never
emits the empty filter.

⚠️ `max_items: unlimited` over a *broadened* scope lifts the write bound — the discovered
set is enumerated into memory and mass-created in one run, bounded (if `max_pages` is also
`unlimited`) only by the transport deadline. The gate keys on the *hazard*, not the `all_*`
flag alone: it fires when `max_items: unlimited` combines with any broadened scope —
`all_*` **or** a non-empty `additional_*` — since a large `additional_*` list has the same
blast radius. This case is gated, not merely warned: the binary refuses (fail-safe) unless
`--allow-unbounded` is passed, and the confirmation is skill-driven, not a binary TTY prompt
— `sync-work-items` owns the interaction, mirroring its existing exit-5 pull-overwrite gate
(the binary refuses with a dedicated code; the skill drives an `AskUserQuestion` and re-runs
with `--allow-unbounded`). Thread `--allow-unbounded` into the skill's argument-hint and
flag list and into `cli.rs --help`. The fail-safe defaults (25 items / 50 pages) remain the
real protection; unbounded-over-broadened-scope is an expert-only, explicitly-acknowledged
path.

### Success Criteria

#### Automated Verification

- [ ] With `all_*` set, the emitted search enumerates the visible set and targets
      every entity (`project IN (...)` / `team in [...]`), never an empty/unbounded
      filter: Jira/Linear fixture goldens + client contract tests
- [ ] `all_*` with an empty enumerated visible set refuses rather than emitting an
      empty/unbounded `IN` list (the 0220 flood boundary): new engine/client contract test
- [ ] `all_*` discovery halts on whichever of `max_items` / `max_pages` binds first — a
      `max_pages` cap-hit pre-empts the `max_items` count, so reaching a `max_items` bound
      on a large `all_*` workspace presupposes `max_pages: unlimited`: engine test
- [ ] An unkeyed scope with neither `all_*` nor a base key still refuses: existing
      guard tests remain green
- [ ] `max_items: unlimited` over a broadened scope — `all_*` *or* a non-empty
      `additional_*` — refuses unless `--allow-unbounded` is passed (binary-level,
      skill-driven confirmation); a bounded broadened pull, and an unbounded *base-only*
      pull, are unaffected: new tests
- [ ] The `sync-work-items` skill surfaces the gate refusal and re-runs with
      `--allow-unbounded` (mirroring the exit-5 pull-overwrite gate): skill-doc review / eval
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] A pull with `all_projects` / `all_teams` discovers across the whole visible
      workspace, halting on `max_items` / `max_pages` rather than flooding.

---

## Phase 8: Discovered-set dedup and total ordering

### Overview

Deduplicate the discovered set by remote identifier (one issue reached via several
scopes appears once) and reconcile in a total, deterministic order: ascending by
identifier prefix (lexical), then by numeric sequence within a prefix.

### Changes Required

#### 1. Discovered-vs-discovered dedup

**File**: `cli/work-adapters/src/sync/run.rs` (`discover_untracked`)
**Changes**: Dedup the discovered ids by `canonical_external_key` before the
existing discovered-minus-local subtraction, so a single issue reached via multiple
scopes survives once. Two distinct identifiers are both retained.

#### 2. Total ordering comparator

**File**: `cli/work-adapters/src/sync/` (a reconciliation-local module)
**Changes**: A free comparator ordering discovered ids by `(prefix, numeric sequence,
raw id)` — the prefix compared lexically, the trailing digit run parsed to an integer
so `PP-2` precedes `PP-10` and all `PP-*` precede all `XX-*`, and the raw id string as a
final tie-break so the order is total over distinct ids (zero-padded variants like
`PP-02` vs `PP-2` that survive dedup never compare `Equal`). An id with no trailing digit
run sorts with a zero sequence and is disambiguated by the raw-id tie-break; the digit
run parses with saturation to `usize::MAX` (not the existing `unwrap_or(0)`, which would
sort an overflowing id *before* `PP-2`) so a very long sequence cannot overflow or panic
and still orders after smaller numbers. `split_prefix_sequence` is a generic decomposition
independent of the local `work.id_pattern` (whose default keyless numeric scheme
`{number:04d}` does not match a hyphenated *remote* id like `PP-2`, so
`parse_full_id`/`WorkItemIdScheme` cannot be reused for this). It splits on the *final*
`-`-delimited numeric segment — everything before it is the prefix, the trailing number the
sequence — so a key with interior digits (`ABC2-5`) keeps `ABC2` as its prefix rather than
collapsing to `ABC`; and it folds case/whitespace in the prefix comparison to agree with the
dedup `canonical_external_key` fold, so mixed-case survivors of one logical prefix order by
sequence, not by ASCII case. The raw-id tie-break remains the fallback for any residual
shape.
Ordering stays a reconciliation concern; `ExternalId` gains no `Ord` and the port is
unchanged. Sort the deduped discovered set with it before reconciliation.

```rust
fn discovered_order(a: &ExternalId, b: &ExternalId) -> Ordering {
    let (ap, an) = split_prefix_sequence(a);
    let (bp, bn) = split_prefix_sequence(b);
    ap.cmp(&bp).then(an.cmp(&bn)).then_with(|| a.as_str().cmp(b.as_str()))
}
```

### Success Criteria

#### Automated Verification

- [ ] The same identifier reached via multiple scopes appears once; two distinct
      identifiers are both retained; and a cosmetic-variant pair (`ENG-12` vs `eng-12` /
      `" ENG-12 "`) folds to one via `canonical_external_key` (raw-string equality must
      fail this): new test in `cli/work-adapters/tests/sync_run.rs`
- [ ] `XX-3, PP-10, PP-2` reconcile as `PP-2, PP-10, XX-3`: new comparator unit test
- [ ] The comparator is a total order over degenerate ids — a no-digit id, a
      zero-padded pair (`PP-02` vs `PP-2`), a large-sequence boundary, and a case-mixed
      pair each order deterministically with no `Equal` between distinct ids: new
      comparator unit tests
- [ ] `max_items` counts the *post-dedup, post-local-subtraction* set: a discovery whose
      raw found count exceeds `max_items` but whose deduped count does not *proceeds*, and
      its mirror (deduped count still over) refuses with zero writes: new tests in
      `cli/work-adapters/tests/sync_run.rs`
- [ ] Base-only discovery (no `pull` block) stays bounded to the base entity and
      unchanged in order: existing tests remain green
- [ ] Workspace check passes: `mise run cli:check`

#### Manual Verification

- [ ] A multi-scope pull spanning several prefixes reconciles in the documented
      order with no duplicate imports.

---

## Testing Strategy

### Unit Tests

- `Value::Mapping` projection and the three in-crate match updates (Phase 1).
- `PullConfig::parse` shape coverage and `validate` rejection branches (Phases 1–2).
- Jira/Linear filter lowering via the existing fixture goldens, extended with
  OR-within-a-key and multi-entity `IN` rows (Phases 5–7).
- The ordering comparator across prefixes and numeric sequences (Phase 8).

### Integration Tests

- Black-box `configure` rejection (`cli/launcher/tests/config_read.rs`) for every
  validation branch, each with a `--fail-safe` non-degradation twin (Phase 2).
- Engine runs through `RecordingTracker` asserting ceiling refusals, truncation
  aborts, scope contents, dedup, and ordering (Phases 3, 4, 6, 8).
- Shared-resolver unit tests for the abort-vs-transient classification (single-sourced),
  plus thin client contract tests that each adapter's `enumerate_visible_entities`
  implementation (paginated to exhaustion) and string emission wire through it (Phases 6–7).

### Manual Testing Steps

1. Configure a `<tracker>.pull` block (team, then a personal override) and confirm
   `accelerator config dump` shows whole-block replacement.
2. Configure an unsupported filter key / reserved `all` / bad ceiling / `all_*` with
   `additional_*` and confirm each is rejected at `configure`.
3. Run `accelerator work sync --preview` with `additional_*`, then `all_*`, and
   confirm the discovered scope, ceilings, dedup, and ordering behave as specified.

## Performance Considerations

`all_*` and `additional_*` add a live enumeration call per pull (Jira
`/rest/api/3/project`, Linear `teams { nodes }`) — endpoints already wired for init,
so no new remote surface. Discovery is bounded by `max_items` / `max_pages`; `all_*`
enumerates rather than issuing an unbounded query, keeping 0220's flood-guards
effective. Dedup and ordering are O(n log n) over the discovered set, negligible
against the network cost. The one unbounded case is `all_* + unlimited + unlimited`,
which removes both ceilings and leaves the transport deadline (300s default) as the sole
guard, accumulating the whole discovered set in memory (O(n)) for a single reconcile
pass; confirm this is acceptable for the largest intended workspaces or steer users to a
non-`unlimited` ceiling for `all_*`; this case is gated at Phase 7 (interactive
confirmation, or `--allow-unbounded` for automation, else a fail-safe refusal), so it
cannot run unacknowledged.

## Migration Notes

No config migration. Unset `max_items` retains today's 25; `max_pages` now defaults to 50
pages per operation — a deliberate raise from today's fixed 20 so the keyed reconcile read
does not cap-abort on organic corpus growth (a `discovery`/`keyed_read` override tunes
either independently). An absent `pull` block is base-only discovery, unchanged. ⚠️ Phase 4 is nonetheless an
upgrade-breaking behavioural change with no config action: standalone `search` flips from
exit 0 (emitting `truncated: true`) to a non-zero exit, and the keyed reconcile read
flips from degrade-to-indeterminate to a hard abort — both at the new default
`max_pages: 50` (raised from today's fixed 20), so a corpus exceeding 50 pages — which
silently truncates today — starts failing loud on upgrade (a corpus of 21–50 pages, which
truncates today, now completes under the higher default). Call this out in the
release notes; a structural completeness field lets machine consumers still detect
truncation (Linear keeps its existing `truncated`; Jira gains an explicit field, having
none today). The committed entity index grows
lazily on pull (additive), needing no backfill — but reviewers should note a broadened
pull (especially `all_*`) can commit entity metadata (keys, names, IDs) for
newly-imported projects/teams into version-controlled files, wider than today's
single-entity seed (only for entities from which items are imported). Public-API snapshots regenerate in
Phases 1–4 and 6: `config` in Phase 1 (the `Value::Mapping` variant); `work` in
Phases 1–2 (`PullConfig`, then `validate`); `tracker` in Phases 2, 3, 4, and 6 (the
`FilterSchema` type, the `Ceiling` type, the `FetchOutcome` completeness signal, and the
`SearchScope` entity-scope sum type); `tracker-support` in Phase 3 (the `max_pages`
retype to `Ceiling`).

## Open Items (deferred to implementation)

Finishing detail surfaced in review pass 4 and judged non-blocking — resolve TDD-style
during the relevant phase, not before:

- **Test-infra ripple (Phase 4):** the `Discovery` `bool`→tri-state change touches the
  `RecordingTracker::discovering(found, complete)` seam and the `search_reports_truncation`
  contract property — enumerate those call sites when retyping.
- **Interactive-gate harness (Phase 7):** keep the `--allow-unbounded` confirmation
  skill-driven, but inject a confirmer port so the interactive branch is CI-testable rather
  than TTY-dependent.
- **`tracker` cohesion (Phases 2–3):** `Ceiling`/`FilterSchema` sit in `tracker` for reach,
  not cohesion — acceptable, but reconsider `kernel` or a small value-types crate if the
  port's logic-free role matters more than the extra edge.
- **exit-74 prose (Phase 6):** update the taxonomy comment to note the scope-target check
  may now be enumeration-confirmed at pull time (no longer strictly "nothing was sent").
- **Catalogue reverse-read (Phase 6 §3):** add a fixture that a newer binary reading a
  pre-upgrade file (no multi-entry key) resolves base-only and merges additively.
- **Keyed-read budget (Phases 4/6):** the keyed-read cap is shared across catalogued teams;
  surface the team count in the abort message so the `keyed_read` override remedy is obvious.
- **Committed metadata (Phase 6 §3):** weigh a gitignored cache path against the tracked
  index, given `all_*` can commit whole-workspace entity metadata into VCS.
- **DX polish (Phase 1 §4 / Phase 2 §1):** render the `pull` block as flat
  `<tracker>.pull.<field>` rows in `dump` (consistent with existing dotted keys), and list
  the accepted filter keys in the unset placeholder so they are discoverable before a
  rejection.
- **`PullConfig` shape (Phases 1–2, optional):** consider a distinct validated projection so
  the post-validation type cannot carry the unknown-keys field; and assert-unreachable on the
  `Value::Mapping` scalar-path arms rather than a silent empty default.

## References

- Work item: `meta/work/0229-per-tracker-pull-scope-configuration.md`
- Research: `meta/research/codebase/2026-09-11-0229-per-tracker-pull-scope-configuration.md`
- Base-scope dependency (done):
  `meta/plans/2026-09-10-0228-layered-configuration-key-model.md`
- Sibling `ItemSelection` lever (done):
  `meta/plans/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
- Config model: `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md`
- Key code: `cli/tracker/src/lib.rs:257-269`, `cli/work-cli/src/sync.rs:702-721,882-890`,
  `cli/config/src/service.rs:18-21,539-544`, `cli/jira-client/src/jql.rs:164-238`,
  `cli/linear-client/src/filter.rs:82-112`, `cli/work-adapters/src/sync/run.rs:526-547,795-891`,
  `cli/tracker-support/src/transport.rs:14-31`, `cli/linear-client/src/catalogue.rs:96-137`,
  `cli/jira-client/src/discovery.rs:49-61`, `cli/linear-client/src/discovery.rs:54-61`
