---
type: codebase-research
id: "2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms"
title: "Research: Removing Visualiser-Server Legacy Linkage Fallback Arms (Story 0102)"
date: "2026-06-15T22:02:48+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0102"
parent: "work-item:0102"
relates_to: ["codebase-research:2026-06-07-0070-meta-corpus-unified-schema-migration"]
topic: "Removing the visualiser server's legacy linkage fallback arms (work-item:/ticket:/work_item_id:/filename) and adding a migration-completion gate"
tags: [research, codebase, visualiser, frontmatter, linkage, migration, cleanup]
revision: "23ce3d841b36cc7fb4c39ac2243604cdd6b9de81"
repository: accelerator
last_updated: "2026-06-15T22:02:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Removing Visualiser-Server Legacy Linkage Fallback Arms (Story 0102)

**Date**: 2026-06-15T22:02:48+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 23ce3d841b36cc7fb4c39ac2243604cdd6b9de81
**Branch**: HEAD (detached / unpushed — local references used below)
**Repository**: accelerator

## Research Question

What does it take to implement work item 0102 — removing the visualiser
server's retained legacy linkage fallback arms (`work-item:`, `ticket:`, legacy
`work_item_id:`, filename fallback) and their pinning tests, renaming
`parent_or_legacy_id`, and adding an observable migration-completion gate — with
exact, current source coordinates and the consumer/dependency facts needed to do
it safely?

## Summary

The work item's Technical Notes (verified against revision `20e5760`) are
**accurate against current source `23ce3d8`** — all named functions, arms, warns,
and tests exist unrenamed, at the line ranges 0102 records. Fresh analysis
confirms the four removal sites, the four-to-delete pinning tests, the two
survivor tests to retarget, and — critically — **resolves the work item's one
open decision rule**:

> **The frontmatter `work_item_id:` *key* must be retained.** 0102's Requirements
> say to remove the legacy `work_item_id:` *identity arm* in `indexer.rs` only if
> "no non-legacy reader still consumes it." It does have such consumers:
> `read_ref_keys` (`frontmatter.rs:346`) treats `work_item_id:` as the
> **preferred** cross-reference key, and `cluster_key.rs` + `patcher.rs` read it
> too. So only the *identity-resolution arm* (`indexer.rs:1384-1390`) and the
> *clustering arm* (`cluster_key.rs:138-155`) are removed; the key itself stays
> supported. The default-to-removal clause does **not** fire.

The scope splits cleanly into four code sites, all in
`skills/visualisation/visualise/server/src/`, plus a new shell/task gate over
`meta/`. The two naming traps 0102 flags are both real: `cluster_key.rs` imports
the macro as bare `warn!` (still needed after removal — a second `warn!` call at
`cluster_key.rs:61` is unrelated), and `indexer.rs:1372-1377` carries an
unrelated **shape-validation** warn that must not be removed.

## Detailed Findings

### Site 1 — `frontmatter.rs:read_ref_keys` (`work-item:` + `ticket:` arms)

`read_ref_keys` (`frontmatter.rs:321-391`) tries a single mutually-exclusive
`if / else if` chain (`:346-380`), then unconditionally aggregates `parent:`
(`:383-385`) and `related:` (`:386-388`). The chain in priority order:

1. `work_item_id:` — **preferred, current** — `frontmatter.rs:346-349`. **KEEP.**
2. `work-item:` — **legacy, REMOVE** — `frontmatter.rs:350-364`; its
   `tracing::warn!` is at `:357-362`.
3. `ticket:` — **oldest legacy, REMOVE** — `frontmatter.rs:365-368`. Confirmed:
   **no deprecation warn, no comment** — it resolves silently.
4. `target:` — **typed ADR-0034, KEEP** — `frontmatter.rs:369-379`. Parses via
   `crate::typed_ref::parse_typed_ref` (`typed_ref.rs:32`), matching only
   `TypedRef::WorkItem(id)`.

**Chain surgery**: after removing arms 2 and 3, the `target:` arm (`:369`) must
re-chain its `else if` off the `work_item_id:` arm (`:346-349`) directly.

**Prose to revise** (all describe the legacy keys): the function doc comment
`:312-320`, the preference comment `:343-345`, and the `work-item:` arm comment
`:351-355` (which itself names this as "the story-0070 follow-on contract
story").

**Tests** (test module from `:394`):
- DELETE (capture_logs deprecation test):
  `read_ref_keys_legacy_work_item_arm_emits_deprecation_warning`
  (`frontmatter.rs:572-586`) — the only `capture_logs` consumer in the file.
- DELETE (assert removed legacy resolution):
  `read_ref_keys_reads_legacy_work_item_key_via_transitional_fallback`
  (`:562-570`); `read_ref_keys_reads_legacy_ticket_key` (`:588-593`);
  `read_ref_keys_numeric_ticket_value_is_stringified` (`:659-664`, breaks
  outright — sole dependency on the `ticket:` arm).
- DELETE or reduce-to-vacuous (precedence tests whose legacy value goes inert):
  `read_ref_keys_prefers_work_item_id_over_transitional_work_item` (`:602-608`);
  `read_ref_keys_with_both_legacy_and_current_keys_prefers_current` (`:595-600`).
- KEEP (target/typed + base behaviour): the six `target:` tests (`:610-650`),
  plus `read_ref_keys_reads_work_item_id_key` (`:555-560`) and the
  parent/related/empty/equivalence tests (`:548-553`, `:652-722`).

**Trap**: `typed_ref.rs:38-46` recognises the `work-item:` *value prefix* inside
a `target:` form (`target: "work-item:0042"`). That is unrelated to the
`work-item:` *frontmatter key* arm and must NOT be touched.

### Site 2 — `cluster_key.rs:parent_or_legacy_id` (legacy `work_item_id:` branch + rename)

`parent_or_legacy_id` (`cluster_key.rs:128-157`, private fn; doc `:123-127`):

- `parent:` branch — **canonical, KEEP** — `cluster_key.rs:132-137`. Resolves via
  `id_from_value` (`:164-178`, shared — keep).
- legacy `work_item_id:` branch — **REMOVE** — `cluster_key.rs:138-155`; the
  `warn!` is at `:148-152`. Removing it leaves `None` at `:156` directly after
  the `parent:` block.

**`warn!` import**: bare `use tracing::warn;` at `cluster_key.rs:16`, called
bare. **Keep the import** — a second, unrelated `warn!` (MAX_DEPTH truncation)
lives at `cluster_key.rs:61`.

**Rename**: `parent_or_legacy_id` has exactly **two references in the whole
server tree** — the declaration (`:128`) and a single call site in `walk`
(`:77`, in the `Plans | Research | PrDescriptions` arm). Both update together.

**Tests** (module `:180-616`; all drive public `resolve_cluster_key`):
- DELETE (capture_logs): `legacy_work_item_id_branch_emits_deprecation_warning`
  (`cluster_key.rs:293-317`) — the only `capture_logs` consumer in the file.
- DELETE / retarget (assert the removed legacy branch — go red after removal):
  `plan_with_work_item_id_frontmatter_resolves` (`:278`) and
  `plan_with_path_shape_work_item_id_resolves` (`:320`). **Note**: 0102's
  Technical Notes call the run at `:278/:320/:336` the "retained-plan block" that
  stays green — that is **only partly right**. `:336`
  (`plan_with_empty_work_item_id_and_no_parent_resolves_none`) stays green
  (empty value fails `id_from_value` regardless), but `:278` and `:320` assert
  *legacy-branch* resolution and will break. Treat them as retarget-to-`parent:`
  or delete, not as green survivors.
- KEEP (genuinely retained — resolve via `parent:` branch or WorkItems own-id,
  not the legacy arm): `plan_with_typed_work_item_parent_resolves` (`:210`),
  `plan_with_bare_parent_id_resolves` (`:226`),
  `parent_typed_form_resolves_same_as_bare_id` (`:242`), and the
  review/validation transitive tests.

**Survivors to retarget** (named in 0102): `cluster_key.rs:402`
(`work_item_review_target_path_resolves_to_work_item_id`) and `:432`
(`work_item_review_typed_work_item_target_short_circuits`) resolve via the
`target:` walk (`:79-113`), not `parent_or_legacy_id` — they already assert
canonical resolution and need only naming/comment alignment, not behavioural
change.

**Discriminator** (why "legacy arm" ≠ "retained plan resolution"): the legacy
arm fires only for a `work_item_id:` *frontmatter key on a Plans/Research/
PrDescriptions entry* (reached from `walk` `:74-78`). A `work_item_id` *field on
a WorkItems entry* (`:73`) and a review's `target:` resolving to a work-item path
are separate paths that survive removal. The match keys on `entry.r#type`
(`:72`).

### Site 3 — `indexer.rs` work-item identity chain (filename fallback + legacy arm)

The identity chain is in `build_entry`'s `DocTypeKey::WorkItems` branch
(`indexer.rs:1346-1401`); doc comment `:1356-1364`; the `read_fm_id` closure
`:1365-1381`. The if/else-if at `:1382-1400`:

- `id:` primary — **KEEP, becomes sole resolver** — `indexer.rs:1382-1383`.
- legacy `work_item_id:` arm — **REMOVE** — `indexer.rs:1384-1390` (warn
  `:1385-1389`).
- filename fallback — **REMOVE** — `indexer.rs:1391-1397` (warn `:1392-1396`).
- `else { None }` — `:1398-1400`.

After removal the chain is `id:` → `None`.

**Do NOT remove** the shape-validation `tracing::warn!` at
`indexer.rs:1372-1377` (inside `read_fm_id`) — it fires on *any* key's
shape-invalid value (guards the surviving `id:` path too), not a legacy arm.

**`work_item_cfg.extract_id`** is used at `:1391` here; grep before assuming it
is dead — it is a `WorkItemConfig` method likely used elsewhere.

**Tests**:
- DELETE (capture_logs): `legacy_work_item_id_key_emits_deprecation_warning`
  (`indexer.rs:3466-3483`) and `filename_fallback_emits_deprecation_warning`
  (`:3485-3502`) — the two named in 0102.
- DELETE / retarget to `id:` (assert legacy-key or filename identity — break
  after removal): `work_item_id_uses_frontmatter_when_present` (`:3393`),
  `work_item_id_falls_back_to_filename_when_frontmatter_absent` (`:3420`),
  `work_item_id_frontmatter_bare_digits_applies_project_code` (`:3504`),
  `work_item_id_frontmatter_foreign_prefix_passes_through` (`:3536`),
  `work_item_id_frontmatter_shape_invalid_falls_back_to_filename` (`:3570`).
  Several of these (project-code / foreign-prefix / shape coverage) can be
  *re-pointed to `id:`* to retain coverage on the surviving path rather than
  deleted.
- REASSESS: `work_item_id_none_when_neither_frontmatter_nor_filename_matches`
  (`:3599`) — `None` is still correct, but the test's intent (all three sources
  fall through) changes.
- KEEP (primary-path proof): `work_item_identity_resolves_via_unified_id_key`
  (`indexer.rs:3451-3464`) — proves `id:` wins over filename; this is the
  post-removal contract.
- KEEP (target resolution, unaffected): all `target_path_from_entry` tests at
  `:3629`, `:3863`, `:3887`, `:3907`, `:3932`, `:3948`, `:3961`, `:3985`,
  `:4013`, `:4033`, `:4049`, plus `:3007`.

### `target:` is independently load-bearing — KEEP (all three sites)

`target_path_from_entry` (`indexer.rs:974-998`) reads the `target:` key
(`:985`), parses via `parse_typed_ref` (`:986`), and dispatches `Plan(id)` /
`WorkItem(id)` / `Path(p)`. Its **non-test call sites**: `indexer.rs:384`
(reviews_by_target reverse index), `:849` (linked_for / cross-ref), `:1107`,
`:1115`, `:1171` (refresh diff), and `cluster_key.rs:95` (review/validation
transitive resolution). This confirms 0102's "do not touch `target:`" — it is
the ADR-0034 destination the legacy arms migrate toward, not a legacy peer.

### The frontmatter `work_item_id:` key — retained (resolves 0102's decision rule)

Distinguish two same-named things: (1) the `IndexEntry.work_item_id` **struct
field** (`indexer.rs:170` decl, `:1442` assignment) — the *resolved output*, read
broadly (`indexer.rs:91`, `:357`, `:1058-1059`, `:1149`, `:1295`;
`cluster_key.rs:73`; `clusters.rs:475`; `related.rs:59`) — always stays; and (2)
the frontmatter `work_item_id:` **key**. The key has confirmed **non-legacy
consumers** beyond the arms being removed:

- `frontmatter.rs:346` — `read_ref_keys` **preferred** cross-ref key (feeds
  `work_item_refs` at `indexer.rs:1415`).
- `cluster_key.rs:140` — read by the clustering resolver (its own legacy branch
  is Site 2, removed separately; the *key read* remains valid for the canonical
  path).
- `patcher.rs:280,285,288,438,446` — preserved/relocated during frontmatter
  patching.

**Conclusion**: keep the `work_item_id:` key support; remove only its two legacy
*resolution arms* (identity in `indexer.rs`, clustering in `cluster_key.rs`).

### Site 4 — the migration-completion gate (new)

The gate is a recursive grep over this repo's `meta/` returning zero surviving
legacy own-identity shapes, with **two clauses**: (a) `^\s*work_item_id:` (0070
lineage) and (b) `^\s*ticket_id:` (migration `0001` lineage). Anchoring to the
frontmatter key at line-start is what distinguishes legacy *own-identity* from
canonical typed *references* (`parent: "work-item:0057"` etc.), which
legitimately carry the `work-item:` token and must NOT match.

**Where it lives** (infrastructure that already exists to model on):
- Closest analogue: `scripts/validate-corpus-frontmatter.sh` (the "AC-1 corpus
  validator", referenced at `tasks/test/integration.py:12`) with test suite
  `scripts/test-validate-corpus-frontmatter.sh`.
- Custom shell-linter registration pattern: `scripts/lint-bashisms.sh` wired via
  `tasks/lint/scripts.py:37-47` (`lint.scripts.bashisms`).
- Natural task home: `tasks/test/integration.py` — `_REQUIRED_CONFIG_SUITES`
  (`:21`) hard-lists required shell suites; `integration.config` (`:46`) runs
  them. A new gate suite slots in here so `mise run check` enforces it.
- Frontmatter parsing in shell: `scripts/config-common.sh`
  (`config_extract_frontmatter()` `:73`); typed-linkage parsing
  `scripts/linkage-parser.sh`.
- Source enumeration honouring `.gitignore`: `tasks/shared/sources.py`
  (`shell_sources()`).

**`meta/` subdirectories the grep must span**: `work/`, `plans/`, `research/`
(+ `codebase/`, `issues/`, `design-gaps/`, `design-inventories/`), `decisions/`,
`notes/`, `reviews/` (+ `plans/`, `work/`, `prs/`), `validations/`, `prs/`,
`specs/`, `talks/`, `diagrams/`. **Ignore `workspaces/*/`** (jj checkouts, not
source).

**Gate test fixtures**: migration suites seed synthetic corpora under
`skills/config/migrate/scripts/test-fixtures/<NNNN>/meta/...` — the gate's test
can follow the same layout to assert it fires on a planted legacy shape and
passes on a clean corpus.

### Migration lineage (gate clause provenance)

- **Clause (a)** `work_item_id:` / `work-item:` — 0070 lineage. The migration
  script is `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`.
  Note the numbering: the **work item is `0070`** but the **migration script is
  `0007`** (4-digit local). Tests: `test-migrate-0007.sh`.
- **Clause (b)** `ticket:` / `ticket_id:` — older, distinct guarantee from
  migration `0001`
  (`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`, the
  `ticket`→`work-item` BREAKING rename recorded in `CHANGELOG.md`). Not in 0102's
  `blocked_by` because no work-item blocker carries it; it gates the `ticket:`
  removal exactly as 0070 gates `work-item:`.
- Migrate-on-use is **advisory, not enforced** (`migrate/SKILL.md`: skills don't
  gate on pending migrations), which is precisely why the reference-corpus grep
  is the *accepted, definitive proxy* for "every consuming repo has migrated" —
  no separate release-gate signal is required.

## Code References

- `skills/visualisation/visualise/server/src/frontmatter.rs:321-391` —
  `read_ref_keys`; legacy `work-item:` arm `:350-364` (warn `:357-362`),
  `ticket:` arm `:365-368` (no warn), `target:` arm `:369-379` (KEEP).
- `skills/visualisation/visualise/server/src/frontmatter.rs:562-586` — legacy
  pinning + deprecation tests to delete; `:610-650` target tests to keep.
- `skills/visualisation/visualise/server/src/cluster_key.rs:128-157` —
  `parent_or_legacy_id`; legacy branch `:138-155` (warn `:148-152`); call site
  `:77`; `warn!` import `:16` (keep — also used `:61`).
- `skills/visualisation/visualise/server/src/cluster_key.rs:293-317` —
  deprecation test to delete; `:278`/`:320` legacy-branch tests to retarget;
  `:402`/`:432` survivors to align.
- `skills/visualisation/visualise/server/src/indexer.rs:1382-1400` — identity
  chain; legacy `work_item_id:` arm `:1384-1390`, filename fallback `:1391-1397`;
  shape-validation warn (KEEP) `:1372-1377`.
- `skills/visualisation/visualise/server/src/indexer.rs:974-998` —
  `target_path_from_entry` (KEEP); call sites `:384`, `:849`, `:1107`, `:1115`,
  `:1171`, `cluster_key.rs:95`.
- `skills/visualisation/visualise/server/src/indexer.rs:3466-3502` — two
  capture_logs tests to delete; `:3451-3464` primary-`id:` test to keep.
- `skills/visualisation/visualise/server/src/typed_ref.rs:32-46` —
  `parse_typed_ref`; the `work-item:` *value prefix* (do not touch).
- `scripts/validate-corpus-frontmatter.sh` — corpus validator to model the gate
  on; `tasks/test/integration.py:12,21,46` — gate task registration point.
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`,
  `…/0007-unify-meta-corpus-frontmatter.sh` — the two migrations the gate proxies.

## Architecture Insights

- **Expand/migrate/contract**: 0102 is the *contract* (third) phase of the
  pattern 0070 introduced — 0070 expanded the reader to accept both shapes and
  added per-arm deprecation warns; 0102 removes the now-dead arms once the
  reference corpus is clean. The deprecation warns (`tracing::warn!`) are the
  expand-phase signal; their pinning tests die with their arms.
- **Corpus-as-proxy**: because migrate-on-use is advisory and this repo cannot
  observe external/userspace corpora, the in-repo `meta/` grep is the accepted
  definitive proxy for global migration completion. This is a deliberate scoping
  decision, not a gap.
- **Key vs arm vs field**: the cleanest mental model is three distinct things
  sharing the `work_item_id` name — the frontmatter *key* (kept), its legacy
  *resolution arms* (removed), and the resolved *struct field* (kept). 0102's
  Requirements note (lines 62-67) maps onto ADR-0033's own-identity (`id:`) vs
  foreign-reference (`<type>_id:`) distinction.
- **Naming traps survive removal**: bare `warn!` in `cluster_key.rs` and the
  `work-item:` value-prefix in `typed_ref.rs` and the shape-validation warn in
  `indexer.rs` all *look* like legacy-arm code but are load-bearing or unrelated.

## Historical Context

- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` and its plan
  `meta/plans/2026-06-07-0070-meta-corpus-unified-schema-migration.md` (Phase 5,
  §5 lines 1237-1253) — explicitly raised 0102 as the contract follow-on so the
  sequence "can't ossify in the expanded-but-never-contracted state". 0070's
  AC-12/AC-13 were split across two releases; the removal half migrated to 0102.
- `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
  — **predates the plan's revision**; describes same-release four-site removal
  and carries **stale line refs** (`frontmatter.rs:334-341`, `cluster_key.rs:119`,
  `indexer.rs:1233`). Use 0102's coordinates (re-derived live here), not the
  research doc's.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — defines `target:` as
  the first-class "what this artifact is about" key (review→subject); the
  destination the legacy arms migrate toward.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` (§ Identity-value
  shape contract, lines 147-173) — own identity is keyed `id:`; foreign refs use
  `<snake_case_type>_id:`. Grounds the filename-fallback removal (`id:` becomes
  primary) and the key-vs-arm distinction.

## Related Research

- `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
  — the originating 0070 research (stale coordinates; useful for contracts and
  the cross-ref data-flow `read_ref_keys` → `work_item_refs` → reverse index →
  `related.rs`).

## Open Questions

- **`work_item_cfg.extract_id` liveness**: confirm via grep whether it has
  callers other than `indexer.rs:1391` before deciding if it becomes dead after
  the filename fallback is removed (likely still used — it is a `WorkItemConfig`
  method).
- **`crate::log::test_support::capture_logs` liveness**: after deleting all four
  capture_logs tests (frontmatter 1, cluster_key 1, indexer 2), confirm whether
  `capture_logs` / `test_support` has any remaining consumer before assuming it
  is dead.
- **Gate task placement**: confirm whether the gate belongs as a new suite in
  `_REQUIRED_CONFIG_SUITES` (`tasks/test/integration.py:21`) vs a standalone
  `lint`-namespace task — both are viable; integration.config is the closer
  precedent given `validate-corpus-frontmatter.sh` already lives there.
- **Both feature modes**: AC-3 requires `mise run test:unit:visualiser` green in
  both `embed-dist` and `dev-frontend` — confirm the deleted/retargeted tests
  are not feature-gated such that one mode skips them.
