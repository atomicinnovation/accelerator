---
type: "codebase-research"
id: "2026-09-10-0228-layered-configuration-key-model"
title: "Research: Layered Configuration Key Model (0228)"
date: "2026-09-10T07:50:25+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0228"
parent: "work-item:0228"
relates_to: ["codebase-research:2026-04-28-configurable-work-item-id-pattern", "codebase-research:2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear"]
topic: "Layered Configuration Key Model"
tags: ["research", "codebase", "configuration", "tracker", "migration", "id-pattern", "sync"]
revision: "66ab109bb246c517b34a095e1c5a887a179aa59e"
repository: "accelerator"
last_updated: "2026-09-10T08:44:46+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Recorded the resolving-alias-vs-ignore-and-warn decision (chose resolving alias) with justification"
schema_version: 1
---

# Research: Layered Configuration Key Model (0228)

**Date**: 2026-09-10 07:50 UTC
**Author**: Toby Clemson
**Git Commit**: 66ab109bb246c517b34a095e1c5a887a179aa59e
**Branch**: ticket-management (jj workspace)
**Repository**: accelerator

## Research Question

What does the codebase look like today across the six surfaces work item 0228
touches, and where do its requirements meet existing machinery versus demand new
machinery? Specifically: the config model and `work.default_project_code`; the
`id_pattern` placeholder DSL; the tracker scope keys (`linear.team_key` /
`jira.project_key`) and their resolvers; how the integration skills read their
scope key; the migration and deprecation-alias machinery; and `init-linear` /
`init-jira` writeback.

## Summary

**0228 is mostly achievable by extending existing machinery, with one genuinely
new mechanism at its centre and two naming gaps larger than the story assumes.**

Three findings dominate the implementation shape:

- **The scope-key config fields the story names do not exist.** `jira.project_key`
  is not a config key — Jira reads `work.default_project_code` directly
  (`cli/jira-client/src/auth.rs:190`). `linear.team_key` is not a config key
  either — the Linear team key lives in `linear/catalogue.json` (`/team/key`),
  and only the UUID (`linear.team_id`) is in config. So "make `linear.team_key` /
  `jira.project_key` the canonical scope key" is partly *introduce* those keys,
  not *route through* them.
- **The tracker-aware read-time alias has no precedent.** The existing
  legacy-alias pattern (`cli/launcher/src/config_command/core/paths.rs:91`)
  **warns and ignores** the legacy value; it never resolves it into the new key.
  0228 needs the opposite — resolve `work.default_project_code` into a *different
  section's* key (`jira.project_key` / `linear.team_key`) conditioned on
  `work.integration`, or into `work.key` when tracker-less. Read-time value
  resolution across sections, branching on another key, is new code.
- **Jira's integration skills currently violate the target ownership.**
  `search-jira-issues` / `show-jira-issue` read `work.default_project_code` for
  their scope key. Linear is already clean (reads `linear.team_id` + catalogue,
  no `work.*`). Inverting ownership so integration skills read only their own
  section is a real change on the Jira side, a no-op on the Linear side.

The correctness spine holds up: ADR-0044 (remote identity in `external_id`,
presence-based sync classification) is exactly why `work.key` must not derive
from the scope key, and 0220 established that discovery must scope from the
tracker key — 0228's split preserves that fix. The story's own review approved it
with one live plan-time thread: **alias persistence semantics** (in-memory read
vs on-disk materialise). Research surfaces a second, sharper tension the review
did not: the original `id_pattern` design was built for *many* project codes per
corpus with a per-creation `--project` override, which sits awkwardly against
0228's *single* integration-owned scope key.

## Detailed Findings

### Config model — a dotted-key tree, not typed structs

Config is a generic dotted-key document tree (`Node` / `Key`), read by string
key. There is no `struct Work` / `struct Linear` / `struct Jira`; sections exist
only as key prefixes. Adding `work.key` is a catalogue entry plus
`effective_nonempty` reads — no deserialise change in the `config` /
`config-adapters` crates.

- **Catalogue (the schema registry)**: `cli/config/src/catalogue.rs`. `WORK_KEYS`
  (line 98) holds `work.integration`, `work.id_pattern`,
  `work.default_project_code` (line 101, `Default::Scalar("")`). `EXTRA_KEYS`
  (line 120) holds the flat `jira.*` / `linear.*` list — including
  `linear.team_id` (line 126) but **no `linear.team_key`, no `jira.project_key`**.
  `default_for` (line 230) resolves defaults; a key-count test at line 257 must be
  updated on any catalogue change.
- **Layering (team/personal precedence)**: `cli/config/src/level.rs:8`
  (`Level::{Team, Personal}`, filenames `config.md` / `config.local.md`). Merge
  logic in `cli/config/src/service.rs` — `effective()` (line 388),
  `effective_nonempty()` (line 425), `get()` reads personal-first (line 476).
  Tested by `personal_overrides_team()` (line 721). Filesystem read/write in
  `cli/config-adapters/src/store.rs` (`FileConfigStore`, `read()` line 208,
  0600 enforcement on the personal file, line 185).
- **`work.*` scalar resolver**: `cli/launcher/src/config_command/core/work.rs` —
  `resolve()` (line 17) returns `ScalarView { value, warnings }`, already emits an
  unknown-key warning (line 49) and a fail-closed refusal for bad
  `work.integration` (line 53). **This is where a `default_project_code`
  deprecation warning slots in.**
- **Typed projection**: the one place `default_project_code` becomes a struct
  field is `WorkItemIdScheme` (`cli/corpus/src/work_item_id.rs:23`), populated by
  `resolve_scheme()` (`cli/work-cli/src/config.rs:24`).

### `id_pattern` placeholder DSL — `{key}` does not exist yet

The pattern compiler is `cli/corpus-adapters/src/work_item_pattern.rs`. It
recognises exactly two token families: `{project}` (validated `[A-Za-z][A-Za-z0-9]*`,
line 173) and `{number}` / `{number:0Nd}` (bare `{number}` defaults to spec `04d`,
line 196). Any other token raises `PatternError::UnknownToken` (line 27); the test
`unknown_token_is_rejected` (line 636) locks this. **Every `{key}` hit under
`cli/` was a false positive** (Rust `format!` strings, not pattern tokens).

Adding `{key}` means extending `compile_token` (line 167) and the token-kind
handling — the compiler, the scan-regex builder (`compile_scan_regex`, line 309),
the format-string builder (`compile_format_string`, line 321), and the full-ID
parser (`parse_full_id`, line 436) all branch on token kind. The default config
pattern is `{number:04d}` (`catalogue.rs:100`), so a pattern without `{key}` must
keep yielding tracker-independent IDs.

ID minting flows: `cli/work/src/next_number.rs` (`allocate`, line 77, the pure
allocator) → `cli/work-cli/src/create.rs` (`allocate_id`, line 183, the minting
site) → `cli/work/src/create.rs` (`compose_frontmatter`, line 84). The
project/`{project}` value threads through `WorkItemIdScheme.default_project_code`
into `next_number.rs:96` and `resolve.rs:267,387`.

### Tracker scope keys and resolvers — naming gap

The story's `linear.team_key` / `jira.project_key` are proposed names, not shipped
keys. Reality:

| Concept | Story name | Actual location today | Kind |
|---|---|---|---|
| Jira scope key | `jira.project_key` | `work.default_project_code` (config) | ❌ not a `jira.*` key |
| Linear scope key | `linear.team_key` | `linear/catalogue.json` `/team/key` | ❌ not a config key |
| Linear team UUID | (n/a) | `linear.team_id` (config) | ✅ exists |
| Active integration | `work.integration` | `work.integration` (config) | ✅ exists |

- **Linear key → UUID resolver** (the "already exists" resolver the story cites):
  `cli/linear-client/src/auth.rs` — `catalogue_team_key` (line 97, reads
  `/team/key`), `resolve_team` / `configured` read `linear.team_id` (line 85).
  `cli/linear-client/src/catalogue.rs` — `CatalogueTeam::resolve` (line 127)
  matches `team_key == key` → returns `team_id` (line 129). ⚠️ This is a
  **single-team** artifact — there is no general key→UUID map, only single-field
  `/team/key` and `/team/id` reads.
- **Jira project resolution**: `cli/jira-client/src/auth.rs:190` (`project_code`
  reads `work.default_project_code`); `cli/jira-client/src/client.rs:375`
  (`resolve_project` validates against the live catalogue, used by create at 614
  and JQL search at 596). `cli/jira-cli/src/resolve_fields.rs:171`
  (`configured_default_project`).

### Integration skills — Jira couples to `work.*`, Linear does not

All four skills (`skills/integrations/{jira,linear}/{search,show}-*/SKILL.md`)
dispatch only through the launcher (`accelerator jira|linear search|show`); none
call a binary directly.

- ❌ **Jira depends on `work.default_project_code`** for its scope key:
  `cli/jira-cli/src/context.rs:192` → `jira_client::auth::project_code` →
  `work.default_project_code` (`auth.rs:191`). The JQL project clause is built
  from it (`cli/jira-cli/src/main.rs:497,534`). This is the exact `work →
  integration` coupling 0228 must break.
- ✅ **Linear reads no `work.*`** for scope: `cli/linear-cli/src/context.rs:182`
  → `catalogue_team_key` (catalogue) + `linear.team_id` (config). The
  `LinearClient` carries `team_key` and resolves it internally
  (`client.rs:673-685`); the search flag `team_id` is `None`
  (`main.rs:101`), scoping is injected from the catalogue.

Note: all four SKILL.md bodies contain prose telling the user to set
`work.integration` (and, for Jira, `work.default_project_code`). That is
documentation; the Linear binaries do not consume `work.*`, the Jira binaries do.

### Migration and deprecation-alias machinery

The migration engine is a compile-time registry, not a directory drop:
`cli/migrate/src/registry.rs` (`registry()`, line 69, sorted by ID). Migrations
`m0001.rs`–`m0008.rs` live in `cli/migrate/src/migrations/`, each a unit struct
implementing `MigrationMeta` (`id()` + `description()`) and `Migration`
(`apply(&self, ctx: &dyn MigrationContext) -> Result<ApplyOutcome, _>`).
Application in `cli/migrate/src/lifecycle.rs` (`run_pending`, line 37); a
SessionStart discoverability hook nags when the applied ledger lags the registry
(`cli/migrate-cli/src/discoverability.rs:48`, wired in `hooks/hooks.json:27`).

**Two closest precedents:**

- **Migration 0004** (`m0004.rs`) — the key-rename template. `rewrite_config_keys`
  (line 442) drives a table of `(prefix, old, new, append_suffix)` tuples over
  `["config.md", "config.local.md"]`, with YAML form detection (`detect_form`,
  line 320, handling nested `prefix:` blocks vs flat `prefix.key:` lines), a
  once-only `.bak` sidecar (`backup_config_once`, line 493), and a **mixed-state
  guard** that aborts if both old and new keys are pinned (line 107). A pure
  `work.default_project_code` → `work.key` rename is a `("work",
  "default_project_code", "key", false)` tuple through this machinery.
- **Migration 0001** (`m0001.rs`) — read-a-value-then-rewrite. `apply()` (line 33)
  reads `ctx.config_value("paths.tickets")` (line 53), decides default-vs-custom,
  and rewrites via a `text.rs` matcher cascade with `_default` variants (rename the
  key, and swap the value only when it still equals the old default).

**The `MigrationContext` port** (`cli/migrate/src/ports.rs:69`) gives migrations
`config_value` (line 98, full effective lookup incl. unrecognised legacy names),
`configured_path_override` (line 114, "is this explicitly pinned?" — the
mixed-state signal), `read` / `write` (write is the only manifest-tracked mutation
path), and `merge_move`.

⚠️ **The critical gap.** The read-time alias in
`cli/launcher/src/config_command/core/paths.rs:91` (`legacy_alias_warning`)
**ignores** the legacy value — the warning literally says "the legacy override is
being ignored" (line 108), and the value is resolved solely from the canonical key
(line 76). Materialisation happens only inside a *migration*, on disk. So the
current split is: read-time = warn-and-ignore, migration-time = rewrite-on-disk.
0228's tracker-aware alias needs three things none of these precedents provide:

1. **Read-time value resolution** — feed the legacy value into the target key's
   resolution, which no scalar resolver does.
2. **Cross-section** — `work.*` legacy value into a `jira.*` / `linear.*` slot;
   every existing rewrite is same-section `(prefix, old, new)`.
3. **Conditional on another key** — branch on `work.integration`; no existing
   alias branches on a second config value.

⚠️ **No semver feature-gating exists.** Plugin version is `1.24.0-pre.65`
(`.claude-plugin/plugin.json:3`, `cli/Cargo.toml:46`), surfaced via
`env!("CARGO_PKG_VERSION")`. Release-gating is structural — behaviour ships as
migrations; the hook nags on ledger lag. "Alias removed in 1.25.0" means the code
is deleted in that release, not a runtime version check. Naming a removal release
in a warning string is itself without precedent (the existing warning names the
*migration*, not a release).

### `init-linear` / `init-jira` writeback

- **Linear** (`cli/linear-cli/src/main.rs:358`, `run_init`) — `Discover` writes
  `catalogue.json` (`/team/id` + `/team/key`) via `cache.write_catalogue`
  (line 404), **not config**. The skill instructs the user to set
  `work.integration: linear` manually. So writing a `linear.team_key` into the
  tracker section (0228 requirement) is new writeback.
- **Jira** (`cli/jira-cli/src/main.rs:760`, `run_init`) — `init_discover` writes
  `projects.json` / `fields.json`; `init_prompt_default` (line 854) reports the
  resolved `work.default_project_code`. The skill's Step 5
  (`init-jira/SKILL.md:123-133`) already offers to write the chosen key into
  `config.md` as `work.default_project_code` — the closest existing writeback,
  retargeted to `jira.project_key` by 0228.

### Local↔remote join stays on `external_id` (ADR-0044)

`external_id` is written into frontmatter only when supplied
(`cli/work/src/create.rs:66,84`). The sync engine
(`cli/work-adapters/src/sync/`) joins purely on it: `run.rs` canonical-key matching
(lines 499-534), untracked-remote discovery (lines 670-673, 985), duplicate-guard
in `cli/work/src/sync/push_precondition.rs:28`, presence-based classification in
`cli/work/src/sync/classify.rs:55`. This is the machinery 0228's AC #7 protects —
`{key}` aligns the prefix only, never the join.

## Code References

- `cli/config/src/catalogue.rs:98` — `WORK_KEYS` (add `work.key` here; update
  count test at :257); `EXTRA_KEYS` at :120 (add `jira.project_key` /
  `linear.team_key`).
- `cli/launcher/src/config_command/core/work.rs:17` — `work.*` resolver; deprecation
  warning slots here.
- `cli/launcher/src/config_command/core/paths.rs:91` — `legacy_alias_warning`, the
  warn-and-ignore precedent (not a resolver).
- `cli/corpus-adapters/src/work_item_pattern.rs:167` — `compile_token`; extend for
  `{key}`.
- `cli/jira-client/src/auth.rs:190` — Jira reads `work.default_project_code`
  (retarget to `jira.project_key`).
- `cli/jira-cli/src/resolve_fields.rs:171` — second Jira read site.
- `cli/linear-client/src/auth.rs:97` — Linear catalogue team-key read (single-team).
- `cli/linear-client/src/catalogue.rs:127` — `CatalogueTeam::resolve` (key→UUID).
- `cli/work-cli/src/sync.rs:434` — 0220's scope construction from
  `work.default_project_code`; the touchpoint 0228 must reconcile.
- `cli/work-adapters/src/sync/run.rs:707` — the discovery gate 0220 fixed.
- `cli/migrate/src/migrations/m0004.rs:442` — `rewrite_config_keys`, the rename
  template.
- `cli/migrate/src/migrations/m0001.rs:53` — read-config-value-then-rewrite.
- `cli/migrate/src/ports.rs:69` — `MigrationContext` capabilities.
- `cli/migrate/src/registry.rs:69` — register a new `m0009`.
- `cli/linear-cli/src/main.rs:404` / `cli/jira-cli/src/main.rs:854` — init writeback
  sites.

## Architecture Insights

- **Config is stringly-typed by design.** Adding keys is cheap (catalogue entry +
  `effective_nonempty` read); the cost of 0228 is not schema plumbing but the new
  resolver semantics and the cross-section alias.
- **`work → integration` dependency is one-way for Linear, wrong-way for Jira.**
  Linear already models the target ownership; the story's ownership inversion is,
  in code terms, mostly a Jira refactor (`jira-client`/`jira-cli` stop reading
  `work.*`) plus introducing the two scope-key config fields.
- **Deprecation is structural, not versioned.** With no semver gate, the one-release
  window is enforced by shipping a migration in 1.24.0 that materialises the new
  keys and deleting the alias code in 1.25.0. The migration must materialise
  *before* the alias is removed, or existing configs break.
- **The single hardest piece is the read-time tracker-aware alias.** It is the only
  requirement with no precedent on either the read-time or migration side, and the
  review's one open thread (in-memory vs on-disk) is exactly about which side owns
  it. Resolving that first shapes everything else.

## Historical Context

- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — the
  team/personal last-writer-wins model 0228 resolves against. ⚠️ **No unset
  sentinel**: personal config cannot clear a team value back to a default, only
  override it with a concrete value. Arbitrary YAML nesting (dropped the old
  two-level cap) is what *permits* nested `linear.team_key` / `jira.project_key`.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` — remote
  identity is `external_id`, sync classification is presence-based, and the sync
  signal "must not depend on the shape of the local `id`". This is the direct
  reason `work.key` must not derive from the scope key.
- `meta/work/0220-untracked-remote-discovery-never-runs-on-linear.md` +
  `meta/research/codebase/2026-08-30-0220-...md` — 0220 promoted
  `work.default_project_code` to the *required* Linear discovery scope authority
  and fixed the gate at `run.rs:707`. It deliberately kept the old field name and
  deferred the rename reconciliation to "whichever sibling ships second" — that is
  0228. 0228 must retarget `sync.rs:434` to read the scope key, preserving the
  fix. ⚠️ 0220 left the Linear catalogue single-team and `all_projects` hardcoded
  `false`; multi-team discovery is still open under epic 0146.
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — the
  origin of `id_pattern` / `{project}` / `default_project_code`. **The field was
  designed as a purely local ID-prefix mechanism** (one default among *many*
  project codes per corpus, with a per-creation `--project` override and
  per-project counters), never as a sync scope. The overload was latent here and
  only became a bug when sync (0146/0220) reused the field. 0228 restores the field
  to its original local-prefix role.
- `meta/reviews/work/0228-...-review-1.md` — APPROVE, with one live plan-time
  thread: ❓ **alias persistence semantics** (read-time in-memory vs on-disk
  materialise, and the write trigger). Also flagged: discovery-scope ACs are not
  pinned to an observable emitted filter (a regression in rigour against 0220,
  which required `{team:{id:{eq:UUID}}}`), and the `work.integration`-vs-section-
  presence signal is never tested where the two disagree.

## Related Research

- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md` — id_pattern origin.
- `meta/research/codebase/2026-08-30-0220-untracked-remote-discovery-never-runs-on-linear.md` — the bug 0228 reconciles.
- `meta/research/codebase/2026-05-08-0046-work-management-system-configuration.md` — config model origin.
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md` — config crate internals.
- `meta/research/codebase/2026-04-29-jira-cloud-integration-skills.md` — Jira scope-key handling.

## Open Questions

All four resolved in the follow-up below (2026-09-10T08:27 UTC).

- ✅ **Alias persistence semantics** — in-memory read-time resolution; migration
  owns the on-disk write. See follow-up.
- ✅ **Multi-project origin vs single scope key** — multi-project origin supported;
  `work.key` is the single local prefix for created (and pulled) items. See
  follow-up.
- ✅ **`jira.project_key` and `linear.team_key` as config keys** — both become real
  config fields. See follow-up.
- ✅ **Tracker-backed signal under disagreement** — `work.integration` wins. See
  follow-up.

## Follow-up Research 2026-09-10T08:27 UTC

The four open questions were resolved by the work-item author. Each decision and
its concrete implementation implication is recorded below.

### 1. Alias persistence — in-memory resolving alias (chosen over ignore-and-warn)

**Decision**: the tracker-aware alias **resolves** the legacy
`work.default_project_code` value in memory at read time; it does not rewrite config
to disk. The migration is the separate, durable path that materialises the new keys
on disk. A simpler ignore-and-warn alias (mirroring
`paths.rs:legacy_alias_warning`) was considered and rejected — justification below.

**Two distinct pieces, cleanly separated:**

- A **read-time resolver** (extend the `work.*` scalar path,
  `cli/launcher/src/config_command/core/work.rs:17`, and the effective-config reads
  in `resolve_scheme` / `jira-client` / `linear-client`) that, when the new key is
  absent and the legacy key is present, returns the legacy value in memory —
  dispatching on `work.integration` to choose the target (`jira.project_key` /
  `linear.team_key` when tracker-backed, `work.key` when tracker-less) — and emits
  the deprecation warning naming 1.25.0.
- A **migration** (new `m0009`, modelled on `m0004`'s `rewrite_config_keys`) that
  writes the resolved values into their canonical slots on disk, so the alias can be
  deleted in 1.25.0.

**Why resolve rather than ignore-and-warn.** `paths.rs:legacy_alias_warning` can
safely ignore its legacy value because the canonical key carries a **benign
non-empty catalogue default** — drop the `design_inventories` override and
resolution falls back to `meta/research/...`, the intended new behaviour. The scope
key has **no benign default**: `work.default_project_code`'s catalogue default is
`Scalar("")` (empty) and the value is repo-specific (`PP`, `ENG`). If a read-time
alias ignored the legacy value, then between upgrading to 1.24.0 and running
`/accelerator:migrate` every existing tracker-backed repo would hit hard failures —
`work create` → `BadProjectValue`/`MissingProject`; Jira `search`/`show` →
`E_NO_PROJECT`; Linear discovery → the scope-key-required error 0220 introduced.
That is a **hard-migrate window** in which the tracker tooling is unusable, and it
contradicts AC #8/#9 as written ("when it is read after this change … IDs still
render `PP-0001`"). The resolving alias keeps existing configs working untouched
through the deprecation window; the migration is then only about durability for the
1.25.0 removal, not about restoring correctness.

Note the failures above would be **loud, not silent** — 0220 already converted an
empty Linear scope from silent-dark into a required-key error — so ignore-and-warn
would not resurrect the silent 0220 bug. The rejection rests on the unusable window
and the AC contradiction, not on a correctness regression.

**Cost is smaller than "new mechanism with no precedent" suggests.** The resolver is
not a generic config-layer feature; it is a scoped `new_key.or_else(|| legacy_with_warning)`
at the ~4 consumers that read the scope key or prefix — `resolve_scheme`
(`cli/work-cli/src/config.rs:24`), `jira_client::auth::project_code`
(`cli/jira-client/src/auth.rs:190`), the Linear resolver
(`cli/linear-client/src/auth.rs:97`), and 0220's scope construction
(`cli/work-cli/src/sync.rs:434`) — with the target chosen by `work.integration`.
Both options need the `m0009` migration regardless, so the entire delta between them
is these four fallbacks versus the hard-migrate window.

⚠️ The read-time resolver must NOT reuse `paths.rs:legacy_alias_warning`, which
ignores the legacy value — it needs the opposite behaviour. The stderr-first
warning plumbing (`ScalarView.warnings` → `render::emit`) is reusable as-is.

**Migration write target** (orthogonal knob, ADR-0047 last-writer-wins makes either
resolve): writing to team `config.md` fixes the repo once for everyone but lands a
committed diff on a shared file (`m0004` does exactly this, with a `.bak` sidecar);
writing to personal `config.local.md` is gitignored and non-invasive but requires
each developer to run the migration. Since `default_project_code` lives in team
config today, the team-target rewrite is the like-for-like move.

### 2. Multi-project origin — supported; `work.key` is the single created-item prefix

**Decision**: work items pulled from a remote may originate from projects other than
the create project, and must coexist on disk. `work.key` is the project prefix for
**created** work items.

**Implication — this matches the code exactly, and no 0228 change is needed to
support it.** The create-from-remote path (`cli/work-cli/src/sync_author.rs:97-160`)
mints a pulled item's local `id` through the **same** allocator as `work create`
(`allocate_id` → `next_number::allocate`), using the corpus-wide
`work.default_project_code` prefix. A remote `ENG-12` and `OPS-5` pulled into a
`PROJ` corpus both become `PROJ-0001`, `PROJ-0002`; the remote key survives only
inside the `external_id` string (`sync_author.rs:145`), never as a local `id`
prefix. So:

- Multi-project *origin* is already supported — items from different remotes coexist,
  joined to their remotes solely via `external_id` (ADR-0044), all carrying the one
  corpus-wide local prefix.
- 0228's rename simply renames that corpus-wide prefix source
  `work.default_project_code` → `work.key`. `work.key` becomes the single local
  prefix for both created and pulled items.
- The per-creation `work create --project` override (`create.rs:637`) remains an
  existing mechanism; the pull path never uses it (`sync_author.rs:103` takes the
  default directly).

⚠️ Distinct per-origin *local* prefixes (a pulled `ENG` item keeping an `ENG` local
prefix) are NOT supported today and are explicitly 0230's concern (0228 Assumptions).
0228 must not attempt per-origin prefixing.

### 3. Both scope keys become real config fields

**Decision**: `jira.project_key` and `linear.team_key` both become genuine config
fields.

**Implication**:

- Add both to `EXTRA_KEYS` (`cli/config/src/catalogue.rs:120`) alongside the existing
  `linear.team_id` / `jira.*` entries; update the key-count test.
- **Linear changes materially.** Today the Linear team key is read only from
  `linear/catalogue.json` (`/team/key`, `cli/linear-client/src/auth.rs:97`);
  `linear.team_key` config does not exist. After 0228, `linear.team_key` (config)
  becomes the canonical scope key, and the catalogue `team_key → team_id` lookup
  (`catalogue.rs:127`) resolves the configured key to the UUID. The resolver must
  read the config key first, reconciling it with the catalogue.
- **`init-linear` gains config writeback.** `run_init`'s `Discover`
  (`cli/linear-cli/src/main.rs:404`) currently writes only `catalogue.json`; it must
  also write the discovered `linear.team_key` into the tracker section (overwriting
  any existing value, per AC). `init-jira` already offers the analogous writeback for
  the project key (`init-jira/SKILL.md:123-133`), retargeted from
  `work.default_project_code` to `jira.project_key`.
- **Jira's integration skills stop reading `work.*`.**
  `cli/jira-client/src/auth.rs:190` and `cli/jira-cli/src/resolve_fields.rs:171`
  switch from `work.default_project_code` to `jira.project_key`, completing the
  ownership inversion (Linear already reads no `work.*`).

### 4. Tracker-backed signal — `work.integration` wins

**Decision**: the tracker-backed determination reads `work.integration`, not section
presence, so a repo may carry configuration for multiple trackers simultaneously
while only the active one governs resolution.

**Implication**: the read-time alias and the migration both dispatch on
`work.integration` (validated set at `catalogue.rs:106`). When
`work.integration: linear` but a `jira:` section is also present, the alias resolves
the legacy value into `linear.team_key` and the `jira:` section is inert for scope
resolution. This aligns with 0228's Assumptions and closes the review's untested
disagreement case — a test should pin exactly this (both sections present,
`work.integration` selects the target). The migration must read `work.integration`
via `ctx.config_value` (as `m0001` reads `paths.tickets`, `m0001.rs:53`) before
choosing the destination key.

### Net implementation shape after these decisions

| Piece | New or extend | Anchor |
|---|---|---|
| `work.key`, `jira.project_key`, `linear.team_key` catalogue entries | Extend | `catalogue.rs:98,120` |
| `{key}` pattern token | Extend | `work_item_pattern.rs:167` |
| In-memory tracker-aware read-time alias + 1.25.0 warning | New | `core/work.rs:17` + effective reads |
| `m0009` migration (materialise + rename, dispatch on `work.integration`) | New (from `m0004`) | `migrations/`, `registry.rs:69` |
| Jira skills read `jira.project_key` not `work.*` | Extend | `jira-client/auth.rs:190`, `jira-cli/resolve_fields.rs:171` |
| Linear resolver reads `linear.team_key` config | Extend | `linear-client/auth.rs:97` |
| `init-linear` writes `linear.team_key` to config | New | `linear-cli/main.rs:404` |
| `init-jira` writes `jira.project_key` (retarget) | Extend | `init-jira/SKILL.md:123` |
| Retarget 0220 scope construction to scope key | Extend | `work-cli/sync.rs:434` |

Multi-project pulled-item prefixing is out of scope (0230). No new layering
mechanics (ADR-0047 reused). The join stays on `external_id` (ADR-0044).
