---
type: "plan"
id: "2026-09-10-0228-layered-configuration-key-model"
title: "Layered Configuration Key Model Implementation Plan"
date: "2026-09-10T09:26:07+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0228"
parent: "work-item:0228"
derived_from: ["codebase-research:2026-09-10-0228-layered-configuration-key-model"]
tags: ["configuration", "work-management", "migration", "tracker"]
revision: "2ea8ee94974a1449605702726016c9a8e0944dde"
repository: "accelerator"
last_updated: "2026-09-10T16:38:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Layered Configuration Key Model Implementation Plan

## Overview

Split the overloaded `work.default_project_code` field into two
independently-owned values and carry existing configs across a one-release
deprecation window. The **scope key** (`jira.project_key` / `linear.team_key`)
is integration-owned and resolves the tracker's creation-home entity and
discovery scope. The **local ID prefix** (`work.key`, spelled `{key}` in
`id_pattern`) is work-owned, required when `{key}` appears, and never derived
from the scope key. A read-time resolving alias keeps `work.default_project_code`
and `{project}` working until `m0009` materialises the new keys on disk; the
alias and the `{project}` token are deleted in 1.25.0.

## Current State Analysis

One config field serves two unrelated jobs today, which is what let bug 0220
hide. The split is clean in code because the two jobs are already read at
separate sites:

- **Minting prefix** flows through `WorkItemIdScheme.default_project_code`
  (`cli/corpus/src/work_item_id.rs:22`), populated by `resolve_scheme`
  (`cli/work-cli/src/config.rs:24`) from `work.default_project_code`.
- **Discovery scope** is a separate `effective_nonempty` read in
  `cli/work-cli/src/sync.rs:853`, feeding `SearchScope.project`.
- **Jira scope key** is read at `cli/jira-client/src/auth.rs:191` and
  `cli/jira-cli/src/resolve_fields.rs:174`.
- **Linear scope key** lives only in `linear/catalogue.json` (`/team/key`); no
  `linear.team_key` config exists. Config carries only the UUID
  (`linear.team_id`).

The naming gap is larger than the story assumes. Neither `jira.project_key`
nor `linear.team_key` is a shipped config key — both are introduced here. The
`{key}` token does not exist; every `{key}` in the tree today is a Rust
`format!` string, not a pattern token.

### Key Discoveries:

- **Two independent pattern implementations both branch on `{project}`.** The
  regex compiler `cli/corpus-adapters/src/work_item_pattern.rs` (used by
  adapters, visualiser, `next_number`, `create`, `canonicalise_id`) and the
  regex-free `WorkItemIdScheme::canonicalise_id`
  (`cli/corpus/src/work_item_id.rs:119`, used by the `work` crate's `filter`,
  `resolve`, `cluster`, and the indexer, which are walled off from
  `corpus_adapters`). `{key}` must land in both.
- **The read-time alias has no precedent and must resolve, not ignore.** The
  existing `legacy_alias_warning` (`cli/launcher/src/config_command/core/paths.rs:91`)
  warns and *ignores* its legacy value, safe only because the canonical key
  carries a benign default. The scope key has no benign default
  (`Default::Scalar("")`), so ignoring would produce a hard-migrate window
  where tracker tooling fails (`E_NO_PROJECT`, `BadProjectValue`, 0220's
  required-key error) until the user runs the migration. The alias must feed
  the legacy value into resolution in memory.
- **The alias fallback belongs in one shared, gated helper.** All three
  consuming crates (`jira-client`, `linear-client`, `work-cli`) already depend
  on `config`. A generic `canonical ?? deprecated` helper there, gated on
  `work.integration`, lets each caller pass its own key names without
  reintroducing a `work.*` read in the integration crates by name.
- **`init-linear` has no config handle.** `run_init`'s `Built`
  (`cli/linear-cli/src/context.rs:110`) does not expose the composed
  `ConfigAccess`; the service is built in `build_client` and dropped. Writing
  `linear.team_key` needs it threaded out or re-composed.
- **The local↔remote join is entirely on `external_id` (ADR-0044).** Neither
  the prefix nor the scope key touches it. `{key}` aligns the prefix only.

## Desired End State

`work.key`, `jira.project_key`, and `linear.team_key` are first-class config
keys. `id_pattern` uses `{key}`; `{project}` is a deprecated synonym.
Integration skills read only their own section's scope key and run with no
`work.*` present. `work.key` is required exactly when `{key}` is referenced,
with no silent fallback to the scope key. Existing `work.default_project_code`
/ `{project}` configs resolve unchanged through the deprecation window, emit a
warning naming 1.25.0, and are materialised on disk by `m0009` at the config
level they were read from. Verification: every acceptance criterion in
`meta/work/0228-layered-configuration-key-model.md` passes, and
`mise run` exits 0 end-to-end.

## What We're NOT Doing

- **Not** removing the alias or the `{project}` token — that is the 1.25.0
  release, a later change. This plan ships both the deprecation path and the
  materialising migration.
- **Not** supporting per-origin local prefixes (a pulled `ENG` item keeping an
  `ENG` local prefix). That is 0230. `work.key` is the single corpus-wide
  prefix for created and pulled items.
- **Not** building 0227's `config validate` command. This plan defines the
  `work.key`-required rule and the deprecation warning at load time; 0227
  surfaces them at command time.
- **Not** changing the `external_id` sync join (ADR-0044) or adding new
  team/personal layering mechanics (ADR-0047 is reused as-is).
- **Not** resolving multi-team Linear discovery (single-team catalogue stays;
  that is open under epic 0146).

## Implementation Approach

Seven phases, each independently mergeable and each leaving `mise run` green.
The ordering lets the deprecation aliases land with their first consumer, so no
phase breaks an existing config: catalogue keys first (inert), then the `{key}`
token, then the prefix and scope owners, then init writeback, then the
materialising migration. Follow red-green-refactor throughout — every
production change is demanded by a failing test written first.

The shared alias helper (Phase 3) is the spine. It is a scoped
`canonical ?? deprecated` fallback, not a generic config-layer feature:

```rust
pub struct AliasedScalar {
    pub value: Option<String>,
    pub deprecation: Option<String>,
}

pub fn resolve_with_deprecated_fallback(
    config: &dyn ConfigAccess,
    canonical: &str,
    deprecated: &str,
    expected_integration: Option<&str>,
) -> Result<AliasedScalar, ConfigError>;
```

The removal release is not a parameter — it is read from the single
`REMOVAL_RELEASE` const internally, so no call site can pass a divergent value.

When `canonical` resolves non-empty it wins. If `deprecated` is *also* set, a
distinct "legacy key present and ignored" deprecation warning is still emitted so
the user is told to remove it — matching the `paths.rs` precedent, which warns even
when it ignores, and closing the case where a user who adopted the canonical key
manually gets no signal that the legacy key lingers. Otherwise, if `deprecated` is
non-empty and either `expected_integration` is `None` (the prefix,
tracker-independent) or equals the effective `work.integration` (the scope key,
gated), the legacy value is returned with a warning naming `REMOVAL_RELEASE`. A
mismatched `work.integration` yields `None` — the legacy value is not this key's to
claim, closing the review's untested "both sections present" case.

`AliasedScalar.deprecation` is load-bearing (AC #9/#10 mandate the warning), so it
is never dropped: every consumer threads it into the existing stderr-first
`ScalarView.warnings` channel, and a single collection point dedupes on the
deprecated key name (not the formatted string) so one legacy field yields one
warning per command, whatever the number of read sites.

One legacy field can legitimately map to two canonical destinations — a
tracker-backed `{key}` repo materialises `work.default_project_code` into *both*
`work.key` and the scope key. So the surviving deduped message leads with `Run
/accelerator:migrate` (which materialises both) rather than naming a single
`set '<canonical>' instead`, so a user who follows it literally does not
under-migrate by setting only one of the two keys.

---

## Phase 1: Config schema — introduce the three keys

### Overview

Add `work.key`, `jira.project_key`, and `linear.team_key` to the catalogue.
Purely additive: no code reads them yet, so behaviour is unchanged and the
phase is safe to ship inert.

### Changes Required:

#### 1. Catalogue entries and count test

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add `work.key` to `WORK_KEYS` with an empty scalar default; add
`jira.project_key` and `linear.team_key` to `EXTRA_KEYS`; bump the count test.

Append `("work.key", Default::Scalar(""))` to `WORK_KEYS`:

```rust
pub const WORK_KEYS: &[(&str, Default)] = &[
    ("work.integration", Default::Scalar("")),
    ("work.id_pattern", Default::Scalar("{number:04d}")),
    ("work.default_project_code", Default::Scalar("")),
    ("work.key", Default::Scalar("")),
];
```

Insert `jira.project_key` and `linear.team_key` into `EXTRA_KEYS` alongside the
existing `linear.team_id` entry; the rest of the list is unchanged.

The count test `the_catalogue_holds_fifty_five_keys_across_six_groups`
(`catalogue.rs:257`) sums the six *counted* groups (PATH, TEMPLATE, WORK, REVIEW,
AGENT, VISUALISER) — `EXTRA_KEYS` is deliberately not counted. Only the `work.key`
addition to `WORK_KEYS` moves the sum, from `55` to `56`; the two `EXTRA_KEYS`
additions (`jira.project_key`, `linear.team_key`) leave it unchanged. Update the
assertion to `56` and rename the test `fifty_six`.

### Success Criteria:

#### Automated Verification:

- [x] `work.key` resolves to an empty scalar default: covered by a new test
      asserting `default_for("work.key")` is `Some(Value::Scalar(String("")))`.
- [x] Config crate tests pass: `cargo test -p config`
- [x] Component check passes: `mise run cli:check`

#### Manual Verification:

- [ ] `accelerator config get work.key` reports the empty default without error.

---

## Phase 2: `{key}` pattern token

### Overview

Make `{key}` the domain spelling of the ID prefix in both pattern
implementations, with `{project}` kept as a deprecated synonym producing
identical output. Rename the internal token vocabulary to key-centric terms
(DDD: the story renames the concept). No deprecation warning here — the
compiler stays pure; the warning is emitted at the config-read boundary in
Phase 3.

### Changes Required:

#### 1. Regex compiler

**File**: `cli/corpus-adapters/src/work_item_pattern.rs`
**Changes**: Recognise `{key}` everywhere `{project}` is handled, as a synonym
sharing the single prefix value channel. Rename `TokenKind::Project`→`Key`,
`is_valid_project`→`is_valid_key`, `PatternError::MissingProject`→`MissingKey`,
`BadProjectValue`→`BadKeyValue`, `ParsedId.project`→`ParsedId.key`. Both token
spellings map to the same kind.

The scan/format substitution below the guard is unchanged:

```rust
if token == "key" || token == "project" {
    if key_value.is_empty() {
        return Err(PatternError::MissingKey);
    }
    if !is_valid_key(key_value) {
        return Err(PatternError::BadKeyValue(key_value.to_string()));
    }
    return Ok(false);
}
```

Mirror the synonym in `build_full_id_regex` (`:403`) and the
`pattern.contains("{project}")` guard in `parse_full_id` (`:440`) and
`canonicalise_id` (`:533`) so a `{key}`-only pattern is not mishandled by the
numeric `else` branch. Replace the substring guard with a **brace-aware**
`references_key` that walks the tokeniser rather than matching raw text, so an
escaped `{{key}}` / `{{project}}` literal does not spuriously count (the predicate
becomes newly load-bearing for the `work.key`-required error in Phase 3, where a
false positive would wrongly reject a valid config):

```rust
fn references_key(pattern: &str) -> bool {
    tokens(pattern).any(|token| matches!(token, Token::Key))
}
```

Share the **recognised-spelling set** once (define it in the lower `corpus` crate and
re-export it) so the accepted spellings are a single source of truth, but have each
crate's `references_key` walk **its own tokeniser** — `corpus-adapters` compiles a
regex via its own scanner, `corpus` is regex-free, and the two are deliberately
walled apart, so a predicate bound to one crate's tokeniser could disagree with the
other's compiler on an edge case (escaped/nested braces, malformed tokens) and
wrongly accept or reject a config. The parity test is extended to assert the two
`references_key` results agree across that edge-case corpus, making the shared
vocabulary — not one crate's tokeniser — the contract between the two ID pipelines.

#### 2. Regex-free scheme implementation

**File**: `cli/corpus/src/work_item_id.rs`
**Changes**: Update `WorkItemIdScheme::canonicalise_id` (`:119`) and its
helpers (`is_canonical_id_token` `:47`, `normalise_id` `:105`,
`canonicalise_id` `:131`, `extract_id` `:176`) to treat `{key}` and `{project}`
as the prefix token via the same `references_key` predicate.

#### 3. Remove `{key}` from the unknown-token set

`unknown_token_is_rejected` (`work_item_pattern.rs:635`) must keep `{bogus}`
and `{number:}` unknown while `{key}` is now recognised.

### Success Criteria:

#### Automated Verification:

- [x] Every `{project}` test in `work_item_pattern.rs` (scan escape, format
      string, `parse_full_id`, `canonicalise_id` golden, error arms) has a
      `{key}` twin asserting identical output: `cargo test -p corpus-adapters`
- [x] A test asserts `{key}` and `{project}` yield byte-identical scan regex,
      format string, and parse for the same value.
- [x] Scheme-side tests mirror `{key}` in `work_item_id.rs` `mod tests`:
      `cargo test -p corpus`
- [x] Parity test still green: `cargo test -p corpus-adapters --test parity`
- [x] `references_key` returns `false` for an escaped `{{key}}` / `{{project}}`
      literal and `true` for a live `{key}` / `{project}` token.
- [x] Any pinned-crate public-api snapshot touched by the renamed `PatternError`
      variants / `ParsedId.key` field is regenerated (`mise run public-api:update`,
      diff reviewed per `tasks/README.md`) and `public-api:check` passes — this gate
      is outside `cli:check`.
- [x] Component check passes: `mise run cli:check`

#### Manual Verification:

- [ ] A repo with `id_pattern: "{key}-{number:04d}"` and `work.key: PP` mints
      `PP-0001` via `accelerator work create`.
- [ ] A repo with the legacy `id_pattern: "{project}-{number:04d}"` still mints
      identically.

---

## Phase 3: `work.key` as the local ID prefix

### Overview

Wire `work.key` as the minting prefix, introduce the shared deprecation-alias
helper, and enforce that `work.key` is required exactly when the pattern
references `{key}`. This is the tracker-independent half: the prefix always
comes from `work.key`, legacy value or not.

### Changes Required:

#### 1. Shared alias helper

**File**: `cli/config/src/legacy_alias.rs` (new module), `cli/config/src/lib.rs`
**Changes**: Add `resolve_with_deprecated_fallback` and `AliasedScalar` as
specified in Implementation Approach. Reads `canonical` via
`effective_nonempty`; on empty, reads `deprecated`; gates on `work.integration`
when `expected_integration` is `Some`. Declare the module (`pub mod legacy_alias;`)
in `lib.rs` and surface its public items through the crate's existing `pub use`
re-export convention, so callers reach them the same way as the rest of the `config`
API. This is a module inside the existing `config` crate, so the `tasks/README.md`
library-crate registration checklist does not apply.

The 1.25.0 release string is a single `pub const REMOVAL_RELEASE: &str = "1.25.0";`.
Because the AC-mandated warning names this release explicitly, guard it against a
schedule slip with a test that first **normalises `CARGO_PKG_VERSION` to its release
minor** (dropping any `-pre.N` pre-release suffix — the shipping version is
`1.24.0-pre.N`) and then asserts `REMOVAL_RELEASE` is a **later minor** than that
normalised version. A strict-adjacency assertion would break spuriously on a
deliberate window extension (a feature-only minor between introduction and removal)
and invert at the removal release itself; the later-minor lower bound catches a
genuine misnaming without coupling CI to a strictly-adjacent removal:

```rust
pub const REMOVAL_RELEASE: &str = "1.25.0";

fn deprecation_warning(deprecated: &str, canonical: &str) -> String {
    format!(
        "Warning: '{deprecated}' is deprecated and will be removed in \
         {REMOVAL_RELEASE}; set '{canonical}' instead. Run /accelerator:migrate"
    )
}
```

#### 2. Rename the scheme field

**File**: `cli/corpus/src/work_item_id.rs`
**Changes**: Rename `WorkItemIdScheme.default_project_code`→`key`. Update the
consumers the compiler will flag: `next_number::allocate`
(`cli/work/src/next_number.rs:96`), `create::allocate_id` and `try_run`
(`cli/work-cli/src/create.rs:637`), `sync_author::author_from_remote`
(`cli/work-cli/src/sync_author.rs:103`), `resolve::resolve_bare_number`
(`cli/work/src/resolve.rs:267,380,387`), `cluster.rs`, `slug.rs`, the visualiser
(`compose.rs:188`, `indexer.rs`), `migrate-adapters/src/context.rs:211`, and the
test fixtures that construct the scheme.

**Serde boundaries the compiler will NOT catch** — these are separate structs, so
the field rename leaves them silently on the old vocabulary and each is an explicit
decision, not a rename the compiler completes:

- `cli/visualiser/server/src/config.rs:69` — `RawWorkItemConfig.default_project_code`
  (`Deserialize`), fed by the launcher's `compose.rs` JSON emission.
- `cli/visualiser/server/src/api/work_item_config.rs:12` — `WorkItemConfigBody`,
  which serialises `defaultProjectCode` to the React frontend.

Decide the wire key deliberately: if the launcher's emitted JSON key is renamed,
`RawWorkItemConfig` must change in lock-step or deserialisation breaks silently; if
the wire key is kept for frontend stability, record that the wire vocabulary
intentionally lags the domain field. Whichever is chosen, the visualiser must not be
left half-renamed.

**Error strings** carrying project vocabulary (`E_PATTERN_MISSING_PROJECT` and
peers in `next_number.rs` / `create.rs`) move to key vocabulary alongside the field.

#### 3. `resolve_scheme` reads `work.key` with alias + validation

**File**: `cli/work-cli/src/config.rs`, `cli/work-cli/src/canonicalise_id.rs`
**Changes**: Resolve the prefix via the helper (prefix is tracker-independent,
so `expected_integration: None`). Return a config-validation error when the pattern
references `{key}`/`{project}` and no prefix resolves. Two correctness constraints:
the resolved prefix is carried into the scheme **only when the pattern references
the prefix token**, and the deprecation warning is surfaced through a real channel,
not dropped.

```rust
pub fn resolve_scheme(
    config: &dyn ConfigAccess,
) -> Result<Resolved<WorkItemIdScheme>, kernel::Error> {
    let id_pattern = effective_nonempty(config, "work.id_pattern")?;
    let pattern_uses_key = references_key(&id_pattern);
    let prefix = resolve_with_deprecated_fallback(
        config, "work.key", "work.default_project_code", None,
    )?;
    if pattern_uses_key && prefix.value.is_none() {
        return Err(work_key_required(&id_pattern));
    }
    let key = pattern_uses_key.then_some(prefix.value).flatten();
    let warnings = pattern_uses_key.then_some(prefix.deprecation).flatten();
    Ok(Resolved::new(WorkItemIdScheme { id_pattern, key }, warnings))
}
```

**Gate the field, not just the error.** `key` is set from the resolved prefix only
when `references_key` holds. Setting it unconditionally would leak a tracker prefix
into `normalise_id` / `extract_id` (which branch on the field, not the pattern), so
a bare-numeric legacy tracker repo (`{number:04d}` + `work.default_project_code: PP`)
would key `0001-foo.md` as `PP-0001`, corrupting `filter` / `resolve` / `cluster` /
the indexer and violating AC #7.

**Suppress the prefix-path warning when the pattern omits `{key}`.** For a
bare-numeric tracker repo the legacy value's real destination is the scope key, not
the prefix — the scope path (Phases 4–5) owns that warning. Emitting "set `work.key`
instead" here as well would give the user two contradictory remediation messages for
one field. So the prefix path only surfaces its deprecation warning when the pattern
actually uses `{key}`.

**Route the warning through a channel.** `resolve_scheme` returns `Resolved<T>`
(value plus warnings); callers funnel `warnings` into the existing stderr-first
`ScalarView.warnings` → `render::emit` plumbing. A single collection point dedupes
on the deprecated key name, so one legacy field yields one warning per command. The
prior sketch's bare `WorkItemIdScheme` return had no warnings channel and silently
dropped `prefix.deprecation` — the AC #9/#10 warning must not depend on each caller
remembering to thread it out.

**Fold the parallel path.** `canonicalise_id.rs` currently resolves the prefix
independently via `effective_nonempty(config, "work.default_project_code")`; route it
through the same resolver so `accelerator work canonicalise-id` honours `work.key`
and the alias uniformly, rather than reading the legacy key directly.

**`work_key_required` message** matches the richness of the existing `bad_integration`
error (`core/work.rs:53`): it names the offending `{key}` token, states the prefix
resolves from `work.key` and *not* from the tracker scope key (and why they are
independent), and tells the user to set `work.key` in `.accelerator/config.md`. A
generic "work.key is required" would invite the user to reach for
`jira.project_key` / `linear.team_key` — the exact fallback the design forbids.

#### 4. Config reference documentation

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add `work.key` to the key tables and document the `{key}` DSL token
(marking `{project}` deprecated), with a worked example contrasting a diverging
`work.key: PP` from a tracker scope key so the independence is taught, not
discovered through the required-key error — the four new "key" names (`work.key`,
`{key}`, `jira.project_key`, `linear.team_key`) otherwise invite the intuition that
`{key}` inherits from the scope key. This is the canonical place a user learns config
keys, so leaving it on the deprecated vocabulary would contradict the migration
message that tells them to "set `work.key`".

### Success Criteria:

#### Automated Verification:

- [x] `resolve_scheme` reads `work.key`: `cargo test -p work-cli`
- [x] Legacy `work.default_project_code` resolves into the prefix with a
      1.25.0 warning when `work.key` is absent (helper unit test in `config`).
- [x] `{key}` present with no `work.key` and no legacy value is a config error
      naming `work.key` (AC #2).
- [x] `work.key` equal to a scope-key value raises no error and no warning
      (AC #5) — asserted by a test with both set equal.
- [x] A pattern without `{key}` yields `scheme.key == None` even when a prefix
      resolves, and `extract_id` recognises `0001-foo.md` with no prefix (AC #7) —
      asserts the field is gated, not merely that minting looks right.
- [x] A legacy tracker-less `default_project_code: PP` + `{project}` pattern mints
      `PP-0001` end-to-end (AC #10), automated, not manual.
- [x] The 1.25.0 deprecation warning is emitted **exactly once** across a full
      command invocation that reads the legacy value at more than one site.
      (Once-per-command asserted e2e via `matches("1.25.0").count() == 1`; the
      dedup is process-global, so the guarantee holds across any number of read
      sites — the genuine two-site read is reinforced by the Phase 5 sync tests.)
- [x] `accelerator work canonicalise-id` honours `work.key` and the alias (routed
      through the shared resolver), not a direct legacy-key read.
- [x] The `config` public-api snapshot is regenerated for `resolve_with_deprecated_fallback`,
      `AliasedScalar`, and `REMOVAL_RELEASE`, and `public-api:check` passes.
- [x] A test asserts `REMOVAL_RELEASE` is a later minor than `CARGO_PKG_VERSION`
      normalised to its release minor (pre-release suffix dropped).
- [x] Full workspace check: `mise run cli:check`

#### Manual Verification:

- [ ] Tracker-less repo, `id_pattern: "{key}-{number:04d}"`, `work.key: PP`:
      `accelerator work create` mints `PP-0001` (AC #6).
- [ ] A legacy repo (`default_project_code: PP`, `{project}` pattern) still
      renders `PP-0001` and prints one deprecation warning naming 1.25.0.

---

## Phase 4: Jira scope-key ownership

### Overview

Invert ownership so the Jira integration reads `jira.project_key` and needs no
`work.*`. The gated alias resolves legacy `work.default_project_code` only when
`work.integration` is `jira`.

### Changes Required:

#### 1. Jira scope reads

**File**: `cli/jira-client/src/auth.rs`, `cli/jira-cli/src/resolve_fields.rs`
**Changes**: `project_code` (`auth.rs:190`) and `configured_default_project`
(`resolve_fields.rs:171`) resolve `jira.project_key` via the helper with
`expected_integration: Some("jira")`, legacy `work.default_project_code`,
`REMOVAL_RELEASE`.

```rust
pub fn project_code(config: &dyn ConfigAccess) -> Result<String, ClientError> {
    let resolved = resolve_with_deprecated_fallback(
        config, "jira.project_key", "work.default_project_code", Some("jira"),
    )?;
    resolved.value.ok_or(ClientError::NoProject)
}
```

`resolved.deprecation` is surfaced through the same warnings channel the caller
already uses for its own diagnostics, so the scope-path warning reaches stderr and
participates in the deprecated-key-name dedup shared with the prefix path.

#### 2. Retarget user-facing strings

**Files**: `cli/jira-client/src/error.rs:20` (`E_NO_PROJECT`),
`cli/jira-cli/src/main.rs:770,870` (`E_INIT_NEEDS_CONFIG`), Jira skill
prose (`search-jira-issues/SKILL.md:24`, `create-jira-issue/SKILL.md:24,75,147`,
`init-jira/SKILL.md:22,132,144`), and the canonical reference
`skills/config/configure/SKILL.md` — add `jira.project_key` to the Jira
recognised-key list (`:767`) and correct the now-false assertion that "No separate
`jira.default_project_key` exists" (`:748-750`) and the "auto-scope to
`default_project_code`" prose (`:445`) — all naming `jira.project_key`. Also sweep
any `skills/work/*` prose that invokes `accelerator config work default_project_code`.

### Success Criteria:

#### Automated Verification:

- [x] Jira resolves its scope key from `jira.project_key` with no `work.*`
      present (AC #1, Jira): `cargo test -p jira-client -p jira-cli`
- [x] A legacy `work.default_project_code` (with `work.integration: jira`)
      resolves into the scope key with a 1.25.0 warning (AC #8). (Value
      resolution asserted in `jira-client`; warning content at the helper level.)
- [x] A legacy value with `work.integration: linear` does *not* resolve into
      `jira.project_key` (gate test).
- [x] Full workspace check: `mise run cli:check`

#### Manual Verification:

- [ ] `accelerator jira search` and `show` run against a config carrying only
      `jira: { project_key: ... }` and no `work.*`.
- [ ] A legacy Jira config still search/shows and renders `PP-0001` IDs.

---

## Phase 5: Linear scope-key ownership and discovery-scope retarget

### Overview

Make `linear.team_key` the canonical Linear scope key, reconciled with the
catalogue, and retarget 0220's discovery-scope construction in `sync.rs` to
read the active tracker's scope key. One phase owns the scope key end-to-end.

### Changes Required:

#### 1. Linear team-key config precedence

**File**: `cli/linear-client/src/auth.rs`
**Changes**: `catalogue_team_key` (`:97`) gains a `config: &dyn ConfigAccess`
argument and reads `linear.team_key` first (via the helper,
`expected_integration: Some("linear")`), falling back to catalogue `/team/key`.
Both callers already hold config: `client.rs:162` (`from_config`) and
`context.rs:182` (`build_with_override`).

```rust
pub fn team_key(config: &dyn ConfigAccess, integrations_root: &Path)
    -> Result<Option<String>, ClientError> {
    let resolved = resolve_with_deprecated_fallback(
        config, "linear.team_key", "work.default_project_code", Some("linear"),
    )?;
    Ok(resolved.value.or_else(|| catalogue_field(integrations_root, "/team/key")))
}
```

**Precedence** is deliberate: `linear.team_key` → deprecated
`work.default_project_code` (gated on `integration = linear`) → catalogue
`/team/key`. The legacy field ranks above the catalogue during the window so a
legacy Linear repo (which since 0220 carries its scope in
`work.default_project_code`) both keeps resolving and emits the AC #9 deprecation
warning — a catalogue-first order would silently swallow the legacy value and skip
the required warning. The caveat: a repo that re-inits (refreshing the catalogue)
without migrating will resolve to the stale legacy value until `m0009` runs; this is
window-scoped and cleared by migration.

`CatalogueTeam::resolve` (`catalogue.rs:127`) is unchanged: it matches the
requested key against the catalogue's `/team/key` and returns the UUID.

#### 2. Discovery-scope dispatch

**File**: `cli/work-cli/src/sync.rs`
**Changes**: Replace the direct `work.default_project_code` read (`:853`) with a
call to a **named, extracted** resolver — `resolve_active_scope_key(config,
integrations_root)` — that dispatches on `work.integration`: `jira` → Jira
`project_code`, `linear` → the catalogue-aware `team_key` resolver from Section 1
(not the bare helper), and sets `SearchScope.project` from the result. Extracting
this "resolve the active tracker's scope key" operation as one function rather than
an inline match keeps `sync.rs`, the integration reads, and 0229's pull-scope layer
on a single abstraction, and is the seam 0229 extends.

**Specify the default arm** (`trello` / `github-issues` / unset `work.integration`)
explicitly: it preserves the current `effective_nonempty(config,
"work.default_project_code")` read, so a scope-key-less integration's discovery scope
is unchanged by the refactor. The replaced `sync.rs:853` read was
integration-agnostic, so an unhandled or `None`-returning default arm would crash
`work sync` or silently drop the scope for those repos — a branch-completeness
regression. The legacy read in the default arm falls under the same 1.25.0 removal
contract as the aliases, and a `work.integration: trello` sync-scope test pins that
its behaviour is unchanged.

Using the Section 1 `team_key` resolver (with its catalogue fallback) rather than
`resolve_with_deprecated_fallback` alone matters: a catalogue-only Linear repo (no
`linear.team_key` config yet, no legacy value) would otherwise resolve to `None`
here while `auth.rs` resolves it fine — two divergent answers for one value, losing
the discovery scope and reopening the 0220 regression. Both scope reads must share
one resolution order. This preserves 0220's required-scope behaviour with the
renamed source.

#### 3. Retarget Linear strings

**File**: `cli/linear-client/src/client.rs:680,689` — `E_SEARCH_NO_TEAM` /
`E_SEARCH_UNKNOWN_TEAM` name `linear.team_key`. Update Linear skill prose where
it names the scope source, and add `linear.team_key` to the Linear recognised-key
list in `skills/config/configure/SKILL.md` (`:830`).

### Success Criteria:

#### Automated Verification:

- [ ] Linear resolves its scope key from `linear.team_key`, falling back to the
      catalogue: `cargo test -p linear-client -p linear-cli`
- [ ] `sync.rs` scope construction dispatches on `work.integration` and feeds
      the resolved key to `SearchScope`; 0220 regression tests pass:
      `cargo test -p work-cli -p work-adapters`
- [ ] The scope passed to the tracker's `search` is **recorded and asserted** to
      carry the resolved key — for Linear, the compiled `{team:{id:{eq:UUID}}}`
      filter as 0220 pinned — for both the divergent (AC #4) and tracker-backed
      (AC #3) arms. (Requires implementing `RecordingTracker::search`, currently
      `unimplemented!`.)
- [ ] A catalogue-only Linear repo (no `linear.team_key` config, no legacy value)
      resolves the same non-`None` scope in `sync.rs` as in `auth.rs`.
- [ ] Both `jira:` and `linear:` sections present with `work.integration: linear`:
      the legacy value resolves only into `linear.team_key` and the `jira:` section
      stays inert — automated, not manual.
- [ ] Scope key in team `config.md`, `work.key` in personal `config.local.md`:
      `{key}` uses the personal `work.key`, discovery scopes from the team scope key,
      personal-over-team precedence holds (AC #8) — mirrors `personal_overrides_team`.
- [ ] `work.key` divergent from the scope key: local IDs carry `work.key`,
      discovery scopes from the scope key, no error/warning (AC #4).
- [ ] Tracker-backed `{key}` with `work.key` set: prefix from `work.key`,
      creation-home from the scope key (AC #3).
- [ ] Full workspace check: `mise run cli:check`

#### Manual Verification:

- [ ] `accelerator work sync` against a Linear repo scopes discovery from
      `linear.team_key` and imports untracked remotes.
- [ ] A repo with both `jira:` and `linear:` sections and
      `work.integration: linear` resolves the legacy value only into
      `linear.team_key`.

---

## Phase 6: Init writeback

### Overview

Have `init-jira` and `init-linear` write the discovered scope key into the
tracker section, overwriting any existing value.

### Changes Required:

#### 1. Jira init writeback

**File**: `skills/integrations/jira/init-jira/SKILL.md`, `cli/jira-cli/src/main.rs`
**Changes**: Retarget Step 5 (`SKILL.md:123`) and `init_prompt_default`
(`main.rs:854`) reporting to `jira.project_key`.

#### 2. Linear init writeback

**File**: `skills/integrations/linear/init-linear/SKILL.md`,
`cli/linear-cli/src/context.rs`, `cli/linear-cli/src/main.rs`
**Changes**: Thread the composed `ConfigAccess` out of `build_client` through
`context::Built` (`:110`), then in `run_init`'s `Discover` arm (`main.rs:404`),
after `cache.write_catalogue`, write `catalogue["team"]["key"]` to
`linear.team_key` via `ConfigAccess::set(&key, value, Level::Team)`.

**Overwrite is offered, not unconditional.** Mirror `init-jira`'s interactive Step 5
rather than a bare `set()`: when `linear.team_key` is already present with a
different value, the writeback is a confirmed action (the discovered key is shown
and the user accepts the overwrite), so a routine re-init to refresh the catalogue
cannot silently destroy a deliberately hand-set value. Writing into an absent key
stays automatic. This keeps the two init paths symmetric and satisfies AC #11/#12
(the existing key is overwritten with the discovered one) without an unrecoverable
clobber.

**Non-interactive default is fail-safe.** With no TTY (CI, scripted re-init) the
confirmation cannot prompt, so the default **preserves** the existing key and reports
that it was left intact — never a blocking prompt, never a silent clobber. A
`--force` flag (injected through the same confirmer seam the tests drive) performs
the unattended overwrite AC #12 describes for automation. The confirmer is an
injectable abstraction so both the confirmed-overwrite and preserved-on-deny arms are
deterministically testable in `cargo test`.

### Success Criteria:

#### Automated Verification:

- [ ] `init-linear` discover writes `linear.team_key` into a section with no key
      (AC #11): `cargo test -p linear-cli`
- [ ] `init-linear` overwrites an existing `linear.team_key` only via the confirmed
      path, and an unconfirmed re-init leaves the existing value intact (AC #12).
- [ ] `init_prompt_default` reports `jira.project_key` (not `work.default_project_code`):
      `cargo test -p jira-cli`. The Jira section *write* itself is skill-driven
      (`init-jira/SKILL.md` Step 5) and verified manually, unlike Linear's
      binary-level writeback — recorded here as a deliberate asymmetry, not a gap.
- [ ] Full workspace check: `mise run cli:check`

#### Manual Verification:

- [ ] `/init-jira` and `/init-linear` write the discovered key into
      `.accelerator/config.md`, overwriting a stale value.

---

## Phase 7: `m0009` migration

### Overview

Materialise the renamed keys on disk so the alias can be deleted in 1.25.0.
Model on `m0004`'s `rewrite_config_keys`, reading the legacy value and
`work.integration` via `m0001`/`m0002`'s `config_value` precedent.

### Changes Required:

#### 1. New migration

**File**: `cli/migrate/src/migrations/m0009.rs` (new)
**Changes**: A mechanical `Migration0009` (id `"0009-..."`) that, per
`CONFIG_FILES` (`["config.md", "config.local.md"]`) under `.accelerator/`,
rewrites in place at the level each key is found (satisfying "write at the level
read from").

**Read once, then write from the captured value.** Capture the legacy
`work.default_project_code` value, the effective `work.integration` (via
`ctx.config_value`), and whether the `id_pattern` referenced the prefix token —
all up front — before any mutation. The rename removes the legacy key, so no later
step may re-read it; every write below derives from the captured snapshot. This
closes the TOCTOU where an unconditional early rename deletes the value a later
step still needs.

Then, from the snapshot:

- Rewrite `id_pattern` `{project}`→`{key}` via `rewrite_one_key` / `detect_form` /
  `probe_value_in_content`.
- **Materialise `work.key` only when the captured pattern referenced the prefix
  token.** This is the single rule governing `work.key` (superseding a blanket
  `work.default_project_code`→`work.key` rename): a bare-numeric tracker repo whose
  legacy value was only a scope key must NOT gain a spurious `work.key`, or the
  on-disk config would re-introduce the AC #7 prefix leak. Where the pattern did not
  reference the prefix, the legacy key is simply removed (its value lives on only in
  the scope key below).
- When `work.integration` is set, materialise the legacy value into
  `jira.project_key` / `linear.team_key` (insert where absent, modelled on
  `insert_research_issues`) — **never overwriting an already-pinned scope key** (one
  a user or `init` set explicitly); an existing explicit scope key is authoritative
  and left untouched.

**Mixed-state guard — enumerate the guarded pairs.** Abort non-zero only on a true
rename collision: `work.default_project_code` and `work.key` both explicitly pinned
with divergent values. Do NOT treat these as mixed state: (a) a legacy value beside
an already-correct canonical scope key (init-written) — the scope key is
authoritative, the legacy key is a removable redundancy; (b) a legacy value beside an
equal `work.key`. In cases (a)/(b) the migration removes the redundant legacy key
rather than aborting, so a user who adopted the canonical key manually is not locked
out of migrating.

**Backup and outcome.** `.md.0009.bak` backup via `backup_config_once` before the
first mutation to each file; for `config.local.md` **both** the sidecar and the
rewritten live file are forced to `0600` to match the personal file's permission
guard — the default temp-file-plus-rename fresh mode is world-readable and would
otherwise expose personal config at rest on either the backup or the rewritten file. Return
`ApplyOutcome::Applied` (recording the ledger) whenever the migration completes —
**including when there is genuinely nothing to migrate**. `NoOpPending` must not be
returned by the highest-id migration: it never advances the ledger, so every clean
repo would sit behind the registry forever and the SessionStart nag would fire every
session, un-clearable. "Already up to date" is a terminal Applied state here, not a
pending one.

#### 2. Register

**File**: `cli/migrate/src/migrations/mod.rs`, `cli/migrate/src/registry.rs`
**Changes**: `pub mod m0009;` and append
`MigrationEntry::Mechanical(Box::new(Migration0009))`. The discoverability nag
and `hooks.json` need no change (lexicographic `.max()` over registry ids).

### Success Criteria:

#### Automated Verification:

- [ ] In-file unit tests cover the rename, the `{project}`→`{key}` rewrite, the
      pattern-conditional `work.key` materialisation (present for a `{project}`
      pattern, absent for a bare-numeric one), and per-integration scope-key
      materialisation on inline fixtures: `cargo test -p migrate`
- [ ] End-to-end `migration_0009.rs` (modelled on `migration_0002.rs`, ledger
      pre-seeded through `0008`) drives the real binary, mirroring the model's full
      arm set:
  - Tracker-backed legacy `{project}` config materialises the scope key and
    `work.key`, and IDs still render `PP-0001` end-to-end (AC #9).
  - Tracker-less legacy `{project}` config materialises `work.key` and renders
    `PP-0001` (AC #10).
  - A tracker-backed bare-numeric legacy config materialises the scope key but
    **no** `work.key`.
  - Idempotent on second run (each transform individually, not only the whole).
  - A `.0009.bak` is written per touched file, and both the `config.local.md`
    sidecar and the rewritten `config.local.md` retain `0600`.
  - Mixed state (both keys pinned, divergent) aborts non-zero and leaves both files
    **unmutated**.
  - A legacy key beside an already-correct canonical scope key migrates (removes the
    redundant legacy key) rather than aborting.
  - After a successful run — including a repo with nothing to migrate — the ledger
    advances to `0009` and the SessionStart discoverability nag is silent.
  - A partial failure between the two files re-runs to convergence with no
    corruption.
- [ ] Full local CI mirror: `mise run`

#### Manual Verification:

- [ ] `/accelerator:migrate` on a real legacy repo produces canonical keys at
      the original config level, leaves a `.0009.bak`, and the tracker tooling
      keeps working.

---

## Testing Strategy

### Unit Tests:

- Alias helper: canonical-wins, canonical-wins-but-legacy-also-present (ignored-legacy
  warning), legacy-fallback-with-warning, gate on `work.integration` (match, mismatch,
  absent), empty-both.
- Pattern token: `{key}`/`{project}` output parity across scan, format, parse,
  canonicalise, in both implementations; unknown-token set intact; `references_key`
  brace-aware (escaped `{{key}}` excluded).
- `resolve_scheme`: `work.key` read, required-when-`{key}` error, equal-value
  no-error, pattern-without-`{key}` yields `scheme.key == None` (field gated),
  deprecation warning surfaced and suppressed-when-no-`{key}`.
- `REMOVAL_RELEASE` equals the next minor after `CARGO_PKG_VERSION`.
- Migration rewrite helpers on inline fixtures, including pattern-conditional
  `work.key` materialisation and the enumerated mixed-state guard pairs.

### Integration Tests:

- Jira and Linear scope resolution with only their own section present.
- Both `jira:` and `linear:` sections present, `work.integration` selecting the
  target (the inactive section stays inert).
- Scope key in team config + `work.key` in personal config: personal-over-team
  precedence, split resolution (AC #8).
- The scope passed to `search` is recorded and asserted (Linear
  `{team:{id:{eq:UUID}}}`), for the divergent and tracker-backed arms.
- Catalogue-only Linear repo resolves the same scope in `sync.rs` as in `auth.rs`.
- The deprecation warning is emitted exactly once per command across multiple read
  sites.
- `sync.rs` discovery-scope dispatch preserving 0220 (untracked-remote
  discovery still gated and scoped).
- `init-linear` / `init-jira` config writeback (set, and confirmed overwrite).
- End-to-end `m0009` against the real binary: tracker-backed `{project}`,
  tracker-less `{project}`, tracker-backed bare-numeric (no `work.key`), idempotency,
  mixed-state abort (no mutation), redundant-legacy-removes-not-aborts, ledger
  advances / nag clears on a no-op run, `.0009.bak` created and `0600` for the
  personal file, and partial-failure re-run to convergence.

### AC Traceability:

Each work-item acceptance criterion (numbered per
`meta/work/0228-layered-configuration-key-model.md`) maps to a named test:

| AC | Covered by |
|----|------------|
| #1 | Jira/Linear scope resolution with only their own section present |
| #2 | `{key}` present, no prefix → `work_key_required` error |
| #3 | Tracker-backed `{key}` + `work.key`: prefix + independent creation-home |
| #4 | Divergent `work.key`: recorded `SearchScope`/filter assertion |
| #5 | `work.key` equal to scope key: no error, no warning |
| #6 | Tracker-less `{key}` + `work.key` mints from `work.key` |
| #7 | No-`{key}` pattern → `scheme.key == None`, prefix-less `extract_id` |
| #8 | Team scope key + personal `work.key`: precedence split |
| #9 | Tracker-backed legacy: scope key + `work.key` materialised, `PP-0001`, warning |
| #10 | Tracker-less legacy: `work.key` materialised, `PP-0001`, warning |
| #11 | Init writes scope key into a section with no key |
| #12 | Init overwrites an existing scope key via the confirmed path |

### Manual Testing Steps:

1. Fresh tracker-less repo with `{key}` + `work.key` mints `PP-0001`.
2. Legacy Jira and Linear repos run search/show/sync unchanged, one 1.25.0
   warning each.
3. `/init-jira` and `/init-linear` write the discovered key.
4. `/accelerator:migrate` materialises keys at the original level; a second run
   is a no-op.

## Migration Notes

The read-time aliases (Phases 3–5) keep every existing config working through
the deprecation window; `m0009` (Phase 7) is durability only. The window is
structural, not version-gated: `work.key` ships in 1.24.0, `m0009` materialises
the new keys, and the alias plus `{project}` token are deleted in 1.25.0. If the
introducing release slips, the removal shifts to the following minor to preserve
the one-release window (the `REMOVAL_RELEASE`-vs-`CARGO_PKG_VERSION` test in Phase 3
catches a slip mechanically).

**The removal is enforced, not assumed.** The only mechanism ensuring a repo
migrated before the alias vanished is the advisory SessionStart nag — nothing stops
a user upgrading 1.24.0 → 1.25.0 unmigrated, at which point `{project}` becomes a
hard `UnknownToken` and the scope key vanishes (`E_NO_PROJECT` / `BadKeyValue`), the
exact failures the resolving alias was built to prevent. So the 1.25.0 change must
**refuse to start with a clear `/accelerator:migrate` error** on detecting a legacy
`work.default_project_code` or a `{project}` pattern, rather than silently deleting
recognition. This is a required contract on the follow-up release, recorded here so
it is not inherited unguarded.

**Forward-only, mixed-version hazard.** `m0009` rewrites team `config.md` (like
`m0004`), landing a committed diff. Once migrated, the config is unreadable by any
pre-1.24.0 plugin (`{key}` is a hard `UnknownToken`; the new keys are unknown), so a
colleague, CI runner, or plugin rollback still on the older version fails to mint IDs
until they upgrade. This is a deliberate team-file rewrite (the like-for-like move,
since `default_project_code` lives in team config today); the `.0009.bak` sidecar is
the manual recovery path.

**Coverage is `jira`/`linear` only.** The alias and `m0009` dispatch handle
`jira.project_key` and `linear.team_key`; `work.integration: trello` /
`github-issues` carry no scope-key dependency today, so their legacy configs
materialise `work.key` (when the pattern references it) but no scope key. If those
integrations gain a scope key later, the dispatch and `m0009` destination table must
extend to cover them.

## References

- Work item: `meta/work/0228-layered-configuration-key-model.md`
- Research: `meta/research/codebase/2026-09-10-0228-layered-configuration-key-model.md`
- ADR-0044 (remote identity in `external_id`):
  `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md`
- ADR-0047 (multi-level configuration):
  `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md`
- Rename template: `cli/migrate/src/migrations/m0004.rs:442`
- Read-value-then-rewrite: `cli/migrate/src/migrations/m0001.rs:53`
