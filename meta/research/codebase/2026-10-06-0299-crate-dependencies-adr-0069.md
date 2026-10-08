---
type: "codebase-research"
id: "2026-10-06-0299-crate-dependencies-adr-0069"
title: "Bringing the CLI workspace's crate dependencies into line with ADR-0069"
date: "2026-10-06T08:19:41+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0299"
parent: "work-item:0299"
topic: "Bringing the CLI workspace's crate dependencies into line with ADR-0069"
tags: ["research", "codebase", "cli", "architecture", "dependencies", "consent-adapters", "corpus-adapters", "migrate", "tracker-support", "config", "vcs", "build-system"]
revision: "27dfc3a27d24f17364ab10cae7820619270989c8"
repository: "accelerator"
last_updated: "2026-10-06T08:19:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bringing the CLI workspace's crate dependencies into line with ADR-0069

**Date**: 2026-10-06 09:19 BST
**Author**: Toby Clemson
**Git Commit**: `27dfc3a27d24f17364ab10cae7820619270989c8`
**Branch**: detached working copy (jj workspace `build-system`)
**Repository**: accelerator

## Research Question

What does the live `cli/` workspace look like against work item 0299, and
where does the work item's prescribed shape (ports, symbol landings,
removals, lint, measurement) meet friction in the code?

## Summary

The live `cargo metadata` graph matches 0299's violations table edge for
edge, and no crate declares `[package.metadata.accelerator]` yet. Most of the
work item's design holds, but the code contradicts or extends it in six
places:

1. **`canonicalise_id` cannot land in `corpus` as written.** It needs
   `regex`, and `corpus`'s pup rule admits only `std`/`core`/`alloc`,
   `kernel::Error` and `crate` (`cli/pup.ron:63-77`).
2. **The `corpus` parsing port must cover `split` as well as `parse`.**
   `frontmatter_validation` and m0008 both use `document::split`.
3. **m0008 needs more than canonical rendering.** It also compares parsed
   values before and after rendering, and splits for its loss check.
4. **The repository-facts port must cover working-copy state.**
   `migrate-adapters` and `work-adapters` call `InProcessProbe::
   working_copy_state`/`dirty_paths`, which no `vcs` trait expresses.
5. **`corpus::RecordStore` has no read and no locked replace.**
   `migrate-adapters` bypasses it with raw `std::fs` + `jsonl::parse_record`
   and the inherent `FileCorpusStore::replace_locked`.
6. **`store` deliberately has no `kernel` dependency.** Moving `TEMP_PREFIX`
   to `kernel` forces either a new `store` → `kernel` edge or a duplicated
   literal.

The build-system side has a direct precedent (`config_test_support.py`), but
no harness exists for launcher binary size or SessionStart summary latency.
The `deny.toml` `uluru` table already omits `accelerator-research` and the
launcher, and its "no gix" rows for `design`, `jira` and `linear` look stale.

## Detailed Findings

### Current crate graph

Intra-workspace normal dependencies from `cargo metadata --no-deps` (dev
edges omitted):

| Crate | Normal workspace dependencies | Violations (0299 table) |
|---|---|---|
| `accelerator` (launcher) | `config`, `config-adapters`, `kernel`, `store`, `tracker-support` | rule 6 via `tracker-support` |
| `consent-adapters` | `config`, `config-adapters`, `vcs`, `vcs-adapters` | rules 4, 7 |
| `corpus-adapters` | `corpus`, `document`, `kernel`, `store`, `vcs-adapters` | rule 4 |
| `research-adapters` | `corpus`, `corpus-adapters`, `kernel`, `research`, `store` | rule 4 |
| `jira-client` / `linear-client` | `config`, `corpus`, `corpus-adapters`, `remote-projection`, `store`, `tracker`, `tracker-support` | rule 4 |
| `migrate-adapters` | `config`, `config-adapters`, `corpus`, `corpus-adapters`, `kernel`, `migrate`, `store`, `vcs`, `vcs-adapters`, `work-adapters` | rules 2, 4 |
| `work-adapters` | `corpus`, `kernel`, `tracker`, `vcs`, `vcs-adapters`, `work` | rule 4 |
| `migrate` | `corpus`, `document`, `kernel` | rule 1 |
| `work` | `config`, `corpus`, `kernel`, `tracker` | rule 3 (dead `config`) |
| `collaboration` | `kernel`, `vcs` | rule 3 judgement |

Other relevant facts from the graph:

- The workspace has 43 members. `cli/licence-audit/` is a directory that holds
  only `new-trees.txt`; it is not a member.
- `work-adapters`' dev-dependencies reach `jira-client`, `linear-client` and
  `tracker-support`. They are legal only because the lint ignores dev edges.
- `remote-projection` has a dev-dependency on `work`, which is a product
  domain. It is ignored as a dev edge.

### Consent and tracking (`consent-adapters`, launcher)

`consent-adapters` exports four items (`cli/consent-adapters/src/lib.rs:12-15`):

- **`credential_ports(config_root, cwd)`** (`credentials.rs:13-19`) builds
  `CredentialPorts::system` from `VcsConfigFileTracking` and `command_runner`.
- **`command_runner(config_root, cwd)`** (`runner.rs:15-22`) wraps
  `BashCommandRunner` over `repository_roots`. This is a third consumer of the
  roots question, which 0299's Tracking-port note does not name.
- **`repository_roots(config_root, cwd)`** (`roots.rs:11-14`) applies
  `beside_config_root` to `vcs_adapters::repository_roots(cwd)`.
- **`VcsConfigFileTracking`** (`tracking.rs:9-17`) implements
  `config::consent::ConfigFileTracking` through `translated(vcs_adapters::
  file_tracking(path))`. It never overrides `check`.

The value types `FileTracking` and `RootsAnswer` already live in the `vcs`
domain (`cli/vcs/src/tracking.rs:6-38`). No port trait exists yet;
`vcs_adapters::file_tracking` (`cli/vcs-adapters/src/tracking.rs:25-29`) and
`repository_roots` (`cli/vcs-adapters/src/roots.rs:20-52`) are free functions.
A `RepositoryTracking` port only needs to wrap them.

There are **6 non-launcher root crates and 7 non-launcher call sites**.
Adding the launcher gives 0299's "7 roots with 8 call sites".

| Site | Call |
|---|---|
| `cli/collaboration-cli/src/main.rs:112` | `credential_ports(&root, start)` |
| `cli/jira-cli/src/context.rs:143` | `credential_ports(&root, &start)` |
| `cli/linear-cli/src/context.rs:128` | `credential_ports(&root, &start)` |
| `cli/research-cli/src/main.rs:138-141` | `credential_ports(&project.root, &project.root)` |
| `cli/work-cli/src/tracker_registry.rs:185` | `credential_ports(&self.root, &self.root)` |
| `cli/design-cli/src/config.rs:59-76` | `VcsConfigFileTracking` + `repository_roots` (production) |
| `cli/design-cli/src/config.rs:264-280` | the same pair inside `#[cfg(test)]` |

The launcher's dispatched tracking flows as follows:

```text
summary.rs:62 consent::audit
  └─ ConfigFileTracking::check                    (config/src/consent.rs:97-106)
       └─ DispatchedTracking::check                (launch/outbound/tracking.rs:43-65)
            ├─ resolve fails ──> TrackingCheck::Unchecked
            │                     └─ AuditFinding::PersonalFileUnchecked
            │                          └─ context note "was not checked" (summary.rs:95-99)
            └─ UnixCapture "vcs tracking --path" (capture.rs, 2 s deadline, 4 KiB cap)
                 └─ kernel::TrackingAnswer::from_str ──> Known(..)
```

- `capture.rs` and the `Captured`/`CaptureFailure`/`CaptureBinary` port types
  (`launch/core.rs:312-340`) have no user other than tracking.
- The `rustix` `event` feature (`cli/launcher/Cargo.toml:43`) exists only for
  `capture.rs`.
- `FetchBudget::HelpListing` is reached only through `tracking_resolver`
  (`main.rs:98-105`). It and its pin test (`main.rs:787-790`) become dead.
- The launcher never computes `RepositoryRoots`. It needs one path:
  `FileConfigStore::discover_root(start).join(PERSONAL_CONFIG_RELATIVE)`.
- `file_tracking` asks only the nearest `.jj` and the nearest `.git`. In a git
  repository nested inside another git checkout, only the inner one is
  consulted.
- `kernel::TrackingAnswer` (`cli/kernel/src/tracking.rs`) is used only by
  `vcs-cli` tracking, the launcher adapter and `accelerator_fixture.rs:57-58`.

### Config catalogue and tracker blocks

`cli/config/src/catalogue.rs` can describe only scalar and sequence defaults
(`Default`, `:13-16`) and flat extra keys (`ExtraKey`, `:150-156`, presence
only). It has no mapping, block or per-tracker dimension, and `jira.pull`,
`linear.pull` and the push blocks are absent.

The pull field list exists in three places today:

- `Tracker::accepted_top_level_keys` (`cli/tracker-support/src/pull.rs:311-319`)
- `Tracker::noun` (`pull.rs:297-304`)
- `pull_fields` (`cli/launcher/src/config_command/core/dump.rs:87-106`)

The launcher's push placeholder hard-codes `max_items` (`dump.rs:158-164`).
`work-cli/src/sync.rs:742-774` reimplements `block::read_block` inline.

To keep behaviour, a structured-block description must express all of the
following:

- **Block placement:** `<tracker>.{pull,push}`, gated on `work.integration`
  being jira or linear. A non-mapping is an error; an empty mapping is treated
  as unset.
- **Ordered fields per tracker:** order is load-bearing for the dump golden
  (`cli/launcher/tests/fixtures/dump/dump.golden:90-95`) and for the
  "accepted:" lists in error text.
- **Field kinds:**
  - string list with scalar coercion (`additional_*`)
  - truthy flag (`all_*`)
  - ceiling token, zero allowed (`max_items`)
  - scalar-or-sub-block of positive ceilings (`max_pages.{default,discovery,keyed_read}`)
  - open sub-mapping with per-tracker accepted keys plus the reserved
    `all`/`any` (`filters`)
- **Noun aliasing:** a role (Additional/All) and an owning tracker, which
  drive the wrong-noun hint.
- **Cross-field rule:** `all_*` excludes `additional_*`.
- **Error attribution:** errors name the level that supplied the block
  (Personal/Team filename).

Defaults (`DEFAULT_MAX_ITEMS` = 25, `DEFAULT_MAX_PAGES` = 50) live in
`cli/tracker/src/lib.rs:427,433` as `Ceiling` values, not catalogue strings.

Coverage that characterises today's behaviour:

- **Built-binary dump tests:** golden, rendering and nine refusal cases in
  `cli/launcher/tests/config_read.rs:916-1104`. All are Linear; none uses Jira.
- **In-process parser tests:** `pull.rs:547-1086` and `push.rs:149-251`.

### `TEMP_PREFIX`

- `store::TEMP_PREFIX` (`cli/store/src/lib.rs:21-24`) is the launcher's only
  use of `store`, at `launch/outbound/resolve/cache.rs:11`.
- `store` deliberately avoids `kernel` (`store/src/lib.rs:8-10`) and uses the
  constant itself at `:233`.
- The launcher already has a second private `.tmp-` constant at
  `resolve/tree/layout.rs:35`.
- Other users of `store::TEMP_PREFIX`:
  - `config-adapters/src/store.rs:14,605,815`, which writes it into a
    `.gitignore` rule
  - `work-cli/src/sync.rs:302,2135,2157`
  - `visualiser/server/src/orchestration/state.rs:100,111`

### `corpus-adapters` consumers across contexts

| Consumer | Production use | Notes |
|---|---|---|
| `research-adapters` | `validate_path`, `validate_text`, `parse`, `FrontmatterState` (`src/topic.rs:15-18,223,320,346`) | `FileCorpusStore` only in `#[cfg(test)]` (`conduct/ledger.rs:261`) |
| `jira-client` | `lock::{acquire, LockOptions}` (`src/cache.rs:16-17,307`) | already uses `store::atomic_write` |
| `linear-client` | `lock::{acquire, LockOptions}` (`src/cache.rs:20-21,307`) | mirrors `jira-client` |
| `migrate-adapters` | lock, `jsonl`, `FileCorpusStore`, `RealFs`, `SystemClock`, validation pipeline, `canonicalise_id` | every value constructed internally; `migrate-cli` injects nothing |

The dependencies of each symbol decide where it can land:

| Symbol | Depends on | Fits `corpus` domain? |
|---|---|---|
| `corpus_files`, `Checks`, `TargetOutcome` | `corpus` ports only | yes |
| `build_index`, `validate_*` | `document` via `parsed_frontmatter_text` (`frontmatter_validation.rs:52-58`: `parse` + `document::split`) | yes, once generic over a port that covers `parse` and `split` |
| `parse`, `FrontmatterState`, `ParsedDocument` | `document`; the types already wrap `corpus::Mapping` (`document.rs:13-24`) | types yes, `parse` behind a port |
| `jsonl::{parse_record, compose_record}` | `serde_json` | no; stays an adapter |
| `canonicalise_id` | `regex` (`work_item_pattern.rs:15,440`) | ⚠️ no, unless `corpus`'s pup rule admits `regex` or it goes behind a port |
| `lock::{acquire, LockOptions}` | `std::fs`, `rand`, `rustix`, `corpus::StoreError` | moves to `store` with its own error type and a new `rand` dependency |

The existing `corpus` ports are:

- `AtomicWrite`, `FileRemove` and `RecordStore` (`cli/corpus/src/store.rs:59-88`).
  `RecordStore` offers only `append_record` and `remove_by_key`.
- `DirReader`, `FileReader`, `DirectoryProbe` and `CorpusWalker`
  (`cli/corpus/src/scan.rs`).
- `Clock` and `RepoFactsProbe` (`cli/corpus/src/metadata.rs:15-39`).
- `IdScanner` (`work_item_id.rs:15-17`).

There is no frontmatter-parsing port. `corpus/src/frontmatter_validation/mod.rs:17-21`
says that parsing happens in `corpus_adapters::document::parse` before the
domain is called.

Moving the lock has three knock-on effects:

- The allowlist entry for `lock.rs` in `tasks/lint/store_duplication.py:27`
  goes stale.
- `migrate-adapters/src/run_lock.rs:75-96` duplicates the private sentinel
  format.
- `work-cli` calls `acquire` directly in `create.rs:626`, `update.rs:305` and
  `sync_author.rs:109`.

`FileSessionLogRewriter` (`migrate-adapters/src/session_log.rs`) is built
only by its own tests. No composition root constructs it.

### Migrations and `document`

- **m0001** (`cli/migrate/src/migrations/m0001.rs:43`) uses `document::parse`
  only as a well-formedness gate on `.claude/accelerator*.md`.
- **m0008** (`m0008.rs`) uses:
  - `parse` + `render` (`render_canonical`, `:152-155`)
  - `parse` equality before and after rendering (`:119-125`)
  - `split` followed by `corpus::frontmatter_validation::validate_file`
    (`:127-142`)
  - `split` for loss detection (`:171`)
- `MigrationContext` (`cli/migrate/src/ports.rs:70-274`) already adds
  capabilities as methods with default bodies:
  - `canonicalise_work_item_id`
  - `validate_frontmatter`
  - `realign_sync_baseline`
- A canonical-render capability fits the same pattern. If the equality and
  loss checks stay in `migrate`, they must run over `corpus::FrontmatterValue`
  returned by the parsing port. That is only byte-identical if the
  `document::Yaml` → `FrontmatterValue` translation (`corpus-adapters/src/
  document.rs:69-96`) preserves every distinction `Yaml: PartialEq` sees.
- **`migrate` → `corpus` evidence:**
  - `Record` and `Outcome` in the session log and engine
  - `DocTypeKey`, `linkage::*` and `frontmatter_validation::*` throughout
    m0007
  - `validate_file` in m0008 and `references_key` in m0009
- **`migrate-adapters` dependencies on other contexts' adapters:**
  - `config-adapters` only in `context.rs:42-55`, which builds `Composed` over
    `FileConfigStore` with `LegacyPolicy::Allow`.
  - `vcs-adapters` only in `working_copy.rs:31-55`, via
    `InProcessProbe.working_copy_state`. It does not use `facts`.
  - `work-adapters` only in `realign_sync_baseline` (`context.rs:297-413`),
    which only m0008 calls. It uses `digest::{split_frontmatter_and_body,
    local}`, `baseline::{path, Entry}` and `BaselineStore::{new, load, set}`.
- **Built-binary coverage:**
  - `cli/migrate-cli/tests/migration_0001.rs`
  - `cli/migrate-cli/tests/migration_0008.rs`, including the only two
    sync-baseline tests (`:117`, `:142`)
  - `full_registry_e2e.rs`

### `vcs` repository facts

- **`vcs` domain:** defines `RepoFacts`, `VcsKind`, the ports `RepoRoot`,
  `VcsProbe`, `UserIdentityProbe`, `VcsReporter` and `OriginRemote`, and the
  composers `facts` and `user_name` (`cli/vcs/src/lib.rs:22-157`). It has
  **no port for dirty paths or working-copy state**. Those are inherent
  `InProcessProbe` methods (`cli/vcs-adapters/src/library.rs:402-428`)
  returning the adapter-local `WorkingCopyState`.
- **`vcs_adapters::facts` callers:**
  - `corpus-adapters/src/metadata.rs:216` (`VcsBackedRepoFactsProbe`)
  - `work-adapters/src/sync/working_copy_status.rs:33`
  - `design-cli/src/executor.rs:380`, a composition root and so legal
- **`work-adapters/src/author.rs:28-31`** calls `vcs::user_name` with
  `InProcessProbe` in three roles.
- **`work-cli` commands that reach VCS facts:**
  - `create` (`create.rs:636-640`)
  - `sync` (`sync.rs:1197-1200`, `sync_author.rs:120-124`)
  - `list` (`list.rs:688`, only when a baseline exists)
- **Narrow consumer-side ports already exist:**
  - `corpus::RepoFactsProbe`
  - `work::create::VcsIdentityProbe`
  - `work_adapters::sync::fetch::WorkingCopyStatus`
  - `migrate::ports::WorkingCopy`

### Domain → domain dependencies (rule 3)

| Edge | Upstream types in the downstream model | Assessment |
|---|---|---|
| `work` → `corpus` | `WorkItemIdScheme`, `IdScanner` in public signatures (`filter.rs`, `next_number.rs`, `resolve.rs`; public-API fixture `:62-161`) | justified |
| `work` → `tracker` | `ExternalId`, `RemoteTimestamp` in public sync structs (`sync/classify.rs`, `sync/plan.rs`, `sync/push_precondition.rs`) | justified |
| `collaboration` → `vcs` | only the `OriginRemote` port (`lib.rs:44-46`, `base_repo.rs`, `update_body.rs`) | weakest; a `collaboration`-owned port would remove it |
| `work` → `config` | no `config::` use anywhere in `cli/work` | dead; only `pup.ron:163` and `Cargo.toml:13` keep it |

### Build-system: lint, checklists, licences, measurement

**Lint precedent.** `tasks/lint/config_test_support.py` provides the pattern:

- a pure `violations(packages)`
- `workspace_packages()` running `cargo metadata --format-version 1 --no-deps
  --locked`, which already includes `package["metadata"]`
- an `@task check` that raises `invoke.Exit`

Its tests (`tests/unit/tasks/test_config_test_support.py`) use synthetic
`cargo metadata`-shaped dicts plus one real-tree test. Committed fixture
workspaces exist under `tests/integration/deny/fixtures/` if real cargo is
preferred.

**Wiring.** A new lint touches four places:

1. `tasks/lint/__init__.py`
2. `tasks/__init__.py:90-108`
3. a `mise.toml` leaf
4. both `cli:check` (`mise.toml:612-614`) and `lint:check` (`:674-676`)

It needs both roll-ups because `default` reaches `lint:check` but not `check`
(`tasks/README.md:34-44`). `_CLI_CHECK_GATES` in `tests/unit/tasks/test_mise.py:30-35`
pins placement; `config-test-support` is missing from it today.

**Checklists.**

- The sub-binary checklist (`tasks/README.md:498-665`) is pinned at exactly
  thirteen points by `tests/unit/tasks/test_registration_docs.py:110`.
- The library checklist (`:702-764`) says "five things".
- Adding a declaration step changes both counts.
- The `consent-adapters` placement rule is at `:761-764`.

**`deny.toml`.** The `uluru` comment (`cli/deny.toml:67-104`) lists `vcs`,
`work`, `collaboration`, `migrate` and `corpus` as linking `uluru`. It has
three problems:

- It has no row for `accelerator-research` or the launcher.
- It claims `design`, `jira` and `linear` link no `gix`/`jj-lib`/`uluru`, yet
  all three call `consent-adapters`, which reaches `vcs_adapters::
  file_tracking`/`repository_roots` and `InProcessProbe`. This is inferred,
  not measured: no current release binaries were on disk.
- `tests/unit/tasks/test_vcs_pin_lockstep.py:135-144` pins only that the
  comment contains `MPL-2.0` and `Re-check`.

**Notices.** `tasks/notices.py` renders over the whole workspace, so
`licenses/accelerator-third-party-notices.txt` changes only if `uluru` leaves
the workspace closure.

**Measurement.**

- `measure:warm-dispatch` (`tasks/measure.py:1149-1173`) times `vcs guard`,
  needs a published signed release, and is deliberately outside `check`.
- The only size harness is the fixture size-ratio guard in
  `tasks/build.py:196-241`.
- No harness exists for launcher binary size or SessionStart
  `config summary --format=hook` latency (`hooks/hooks.json:18`).
- The only recorded summary latency is 0226's: 5.0 ms median before
  dispatched tracking, about 12–14 ms after (plan 0226, Phase 6
  Implementation Notes).

**Removal ties.**

- `VCS_SUBCOMMANDS` (`tasks/lint/skill_cli_refs.py:26-28`) is pinned to the
  clap enum by `tests/unit/tasks/test_skill_cli_refs.py:97-108`.
- `public_api.py:18` pins `consent-adapters`.
- `tasks/README.md:667-700` uses `vcs tracking` as its worked example.
- `docs-site/src/content/docs/configuration.md:129-136` documents
  `ACCELERATOR_VCS_BIN` for the session-start check.
- `pup.ron:451-473` (`only_the_consent_policy_names_the_command_runner`) and
  `:434-450` (`vcs_cli_is_free_of_config`) both reference the consent and
  tracking split.

### Characterisation-test inventory

| Behaviour | Existing built-binary coverage | Gap |
|---|---|---|
| `jira.token_cmd` tracked | `cli/jira-cli/tests/flow_consent.rs:384-407` (`test-loopback`) | `jira.allowed_sites` has no binary test |
| `linear.token_cmd` tracked | `cli/linear-cli/tests/flow_consent.rs:80-103` | none |
| `github.token_cmd` tracked | `cli/collaboration-cli/tests/end_to_end.rs:493-540` | none |
| `openalex.api_key_cmd` tracked | `cli/research-cli/tests/fetch_openalex.rs:647-792` | none |
| `design.browser_path` | in-process only (`design-cli/src/config.rs:134-496`) | no binary test |
| `work-cli` consent | team-level only (`work-cli/tests/consent.rs:40-81`) | no tracked-file case |
| launcher summary audit | `cli/launcher/tests/config_read.rs:2872-2959` through `ACCELERATOR_FIXTURE_VCS_TRACKING` | ⚠️ depends on the fixture being removed; needs real-repository equivalents in git and jj |
| tracker block dump | `config_read.rs:916-1104` | no Jira cases |
| m0001/m0008, sync baseline | `migrate-cli/tests/migration_000{1,8}.rs` | no case covering the canonical render of a differing value |

Two points follow from the inventory:

- Most consent binary tests run only in git. 0299 requires both git and jj.
- The launcher's `fail`/`hang` cases have no in-process analogue once
  tracking moves in-process.

## Code References

- `cli/consent-adapters/src/{credentials,runner,roots,tracking}.rs` - the crate being merged
- `cli/config/src/consent.rs:90-106,1019,1151-1185` - `TrackingCheck`, `ConfigFileTracking`, `PersonalFileUnchecked`, `audit`
- `cli/launcher/src/launch/outbound/{capture,tracking}.rs` - dispatched tracking
- `cli/launcher/src/main.rs:98-105,300-345` - tracking resolver and `compose_stack`
- `cli/launcher/src/config_command/core/summary.rs:62,70-103` - audit and the "was not checked" note
- `cli/kernel/src/tracking.rs` - `TrackingAnswer`
- `cli/vcs-cli/src/{cli.rs:55-60,tracking.rs}`, `cli/vcs-cli/tests/tracking.rs` - the subcommand
- `cli/vcs/src/tracking.rs:6-38` - `FileTracking`, `RootsAnswer`
- `cli/vcs-adapters/src/{tracking.rs:25-29,roots.rs:20-52,library.rs:209-604}` - tracking, roots, `InProcessProbe`
- `cli/config/src/catalogue.rs:13-16,150-156,203-265` - catalogue model
- `cli/tracker-support/src/{pull.rs,push.rs,block.rs}` - block parsers
- `cli/launcher/src/config_command/core/dump.rs:87-263` - duplicated field list and block rows
- `cli/store/src/lib.rs:8-24` - `TEMP_PREFIX`, no-kernel stance
- `cli/corpus-adapters/src/{frontmatter_validation.rs:52-58,document.rs,lock.rs,jsonl.rs,store.rs,fs.rs,work_item_pattern.rs:521-556,metadata.rs:103-238}`
- `cli/corpus/src/{store.rs:59-88,scan.rs,metadata.rs:15-39}` - existing `corpus` ports
- `cli/migrate/src/ports.rs:70-274`, `cli/migrate/src/migrations/{m0001.rs:43,m0008.rs:40-171}`
- `cli/migrate-adapters/src/{context.rs:42-413,working_copy.rs,run_lock.rs,session_log*.rs,corpus_index.rs}`
- `cli/migrate-cli/src/main.rs:44-289` - what the migrate root constructs
- `cli/work-adapters/src/{author.rs,sync/working_copy_status.rs}`
- `cli/pup.ron:63-77,80-94,147-172,419-473` - domain allow-lists and consent rules
- `cli/deny.toml:67-107` - `uluru` exception and symbol-count table
- `tasks/lint/config_test_support.py`, `tests/unit/tasks/test_config_test_support.py` - lint precedent
- `mise.toml:597-600,612-614,674-696` - lint wiring
- `tasks/README.md:34-44,498-764` - roll-up rationale, registration checklists
- `tests/unit/tasks/{test_mise.py:30-35,test_registration_docs.py:110,test_skill_cli_refs.py:97-108}`

## Architecture Insights

- **Narrow consumer-side ports are the established idiom.** `corpus`,
  `work` and `migrate` each declare the slice of VCS they need. The 0299
  "repository-facts port in `vcs`" could equally be these existing narrow
  ports with implementations moved to composition roots. The two shapes
  differ in where the adapter code lives, not in rule compliance.
- **`MigrationContext` capability methods with default bodies** are how
  `migrate` already reaches adapter-only behaviour. They are the natural
  shape for canonical rendering and sync-baseline refresh.
- **Domain pup rules are stricter than ADR-0069 rule 1.** Rule 1 constrains
  workspace edges; pup also bars third-party crates such as `regex` from
  `corpus` and `vcs`. Any landing in a domain crate must satisfy both.
- **`cargo-pup` and the new lint are complementary.** pup cannot see Cargo
  edges and the lint cannot see `use` paths. Several pup rules (`:163`,
  `:419-433`, `:451-473`) encode consent and work-config facts this change
  removes.

## Historical Context

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`:
  - It sets the eight rules and the role and kind taxonomy.
  - It names `consent-adapters` as the first packaging crate to merge away.
  - It says "10 adapter → adapter edges across 7 crates" without listing them.
  - It flags launcher size, warm-dispatch latency and the `uluru` notice as
    costs of in-process VCS.
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  places ports in the domain, so a tracking port belongs in `vcs`.
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`:
  its launcher "never on a subdomain" clause is superseded by ADR-0069 rule 6.
- `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md`:
  - It created `consent-adapters` to keep `gix`/`jj-lib` out of
    `config-adapters`' 14 dependents.
  - It routed launcher tracking through `vcs tracking` to keep the launcher
    VCS-free.
  - It introduced `Unchecked` for cold-cache sessions.
  - It recorded summary latency (5.0 ms → about 12–14 ms) but no launcher size,
    and left the "SessionStart wall-time delta acceptable" criterion unticked.
  - Its `signal-hook` pin matches `gix`'s optional dependency so
    `multiple-versions = "deny"` holds. Re-check this when the launcher links
    `gix`.
- `meta/research/codebase/2026-09-24-0226-unify-the-trust-barrier-for-consent-config-keys.md`
  is the predecessor research.

## Related Research

- `meta/research/codebase/2026-09-24-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md`

## Open Questions

- **`canonicalise_id` landing:**
  - relax `corpus`'s pup rule to admit `regex`
  - or put it behind a port (as `IdScanner`/`RegexScanner` do)
  - or keep it in `corpus-adapters` and inject a `canonicalise` capability
- **m0008's guards:** move behind the rendering port, or run over
  `corpus::FrontmatterValue`. The second needs proof that the `Yaml`
  translation is lossless for equality.
- **Repository-facts port:** one `vcs` port covering facts, user name and
  working-copy state, or the existing narrow consumer ports with
  implementations supplied from roots.
- **`TEMP_PREFIX`:** add `store` → `kernel` (reversing a stated design
  choice), or accept a duplicated literal pinned by a test.
- **0299's "launcher's repository-root resolution with a nested
  repository":** the launcher does not resolve repository roots today, only
  `discover_root`. Should the criterion characterise `discover_root` plus
  `file_tracking`'s nearest-only walk instead?
- **Lost failure modes:** launcher `fail`/`hang` tracking tests rely on
  `ACCELERATOR_FIXTURE_VCS_TRACKING`. Once tracking is in-process, which
  real-repository failure stands in for `TRACKING_UNKNOWN` (for example a
  corrupted index, as `vcs-cli/tests/tracking.rs:92-106` uses)?
- **`deny.toml` rows:** confirm by symbol count whether `design`, `jira`,
  `linear` and `research` link `gix`/`uluru` today. This sets the true
  "before" for the licence record.
- **`FileSessionLogRewriter`:** unused by any root. Delete it rather than
  port it?
- **`collaboration` → `vcs`:** keep it under rule 3 (the model names
  `OriginRemote`), or replace it with a `collaboration`-owned port?
