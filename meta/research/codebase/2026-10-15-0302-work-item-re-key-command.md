---
type: "codebase-research"
id: "2026-10-15-0302-work-item-re-key-command"
title: "Work Item Re-Key Command: implementation surface"
date: "2026-10-15T22:22:05+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0302"
parent: "work-item:0302"
relates_to: ["codebase-research:2026-09-26-0230-tracker-owned-work-item-id-generation", "codebase-research:2026-09-23-0241-migrate-false-dirty-tree-detection"]
topic: "Work Item Re-Key Command: implementation surface"
tags: ["research", "codebase", "work-cli", "work-adapters", "work", "tracker", "corpus", "migrate", "vcs", "retirement", "rekey", "id-pattern", "drafts"]
revision: "f6c58fed0595098159aab29675d90fa61981a175"
repository: "accelerator"
last_updated: "2026-10-15T22:22:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Work Item Re-Key Command: implementation surface

**Date**: 2026-10-15T22:22:05+00:00
**Author**: Toby Clemson
**Git Commit**: f6c58fed0595098159aab29675d90fa61981a175 (jj change `moyzmpksroqs`, unpushed working copy)
**Branch**: detached (jj workspace `visualisation-system`)
**Repository**: accelerator

## Research Question

What existing code does `accelerator work rekey` (work item 0302) compose,
where do the story's Technical Notes match or diverge from the live code, and
what has to be built new?

## Summary

0230's primitives cover roughly the per-item half of the story; the run-level
half (run plan, batch collision checks, ordering, fresh-number floor, draft
destination, shared dirty-tree preflight) is new. Seven findings change or
sharpen the Technical Notes:

1. **Retrieval strategy is a CLI flag, not configuration.** `RetrievalStrategy`
   (`cli/work-adapters/src/sync/fetch.rs:36-40`) is chosen by
   `--per-item-reads` on `work sync` (`cli/work-cli/src/cli.rs:357-359`,
   mapped at `cli/work-cli/src/sync.rs:1538-1542`). "The configured retrieval
   strategy" needs either a matching `work rekey --per-item-reads` flag or a
   new config key.
2. **Retirement cannot target `drafts/`.** `plan_retirement` and
   `retired_content` hard-code `work_dir/<new>-<slug>.md`
   (`cli/work/src/retirement.rs:377-381`, `:455-459`); `path_spellings`
   (`:327-341`) only handles drafts → root.
3. **`allocate` has no floor and `ItemIdentity` has no `date`.**
   `next_number::allocate` seeds `highest = 0` and sees only current-pattern
   filenames (`cli/work/src/next_number.rs:100-114`); `ItemIdentity`
   (`cli/work/src/identity.rs:5-11`) carries `path`, `id`, `aliases`,
   `external_id` only.
4. **`migrate` is a `product` context, so `work` cannot depend on it.** The
   preflight must move to the `vcs` platform domain or a new `shared`
   context (`tasks/lint/crate_dependencies.py:86-93`). The probe-failure
   policy is buried inside the adapter (`VcsWorkingCopy::observe`,
   `cli/migrate-adapters/src/working_copy.rs:41-51`) and must be surfaced so
   each caller chooses.
5. **`work sync` resumes unfinished retirement records; it never refuses on
   them.** `rekey` refusing is new behaviour, and `rekey` must not persist its
   own per-item progress as `RetirementRecord`s, or sync's `reconcile` and
   rekey's own refusal both trip over them.
6. **`shape_of` lives in `retirement.rs`, not `references.rs`.** It is private
   at `cli/work/src/retirement.rs:286-292`. Prefixed local IDs (`ACC-0042`)
   already classify as `Distinctive` through `corpus::is_tracker_key`.
7. **"Unreachable" mostly means a failed `locate`.** `fetch_all` returns `Err`
   only on a pre-flight failure; an attempted request that fails yields `Ok`
   with every key `indeterminate` (`cli/tracker/src/lib.rs:678-700`), which
   then fans out to one `locate` per key.

Line numbers below come from sub-agent reads at the revision above; I did not
re-verify each one.

## Detailed Findings

### ID retirement primitives

The pure planner diffs the whole corpus; the adapter applies the plan with
locks, a changed-since-snapshot check, recovery copies and a compensating
rollback.

- **`Retirement<'a> { old_id, new_id, new_external_id: Option<&str> }`**
  (`cli/work/src/retirement.rs:30-35`). The only constructor,
  `Retirement::onto(old, &TrackerKey)` (`:41-47`), sets `new_external_id =
  new_id`. Fields are `pub`, so a numeric target with `new_external_id: None`
  needs a struct literal or a new named constructor.
- **`plan_retirement(retirement, work_dir, items, corpus) ->
  Result<RetirementPlan, RetirementRefusal>`** (`:348-444`). It finds the
  retiring item case-insensitively, detects an interrupted target (an item
  with `id == new_id` and `old_id` in `aliases`), then computes `from`, `to`,
  rewrites, `retired_item`, `recovery`, `baseline_rename` and `resumes`.
  Refusals: `IdTaken`, `KeyLinked`, `TargetExists`, `ItemNotFound`
  (`:143-191`).
- **`retired_item_content`** (`:646-671`) sets `id` (or `work_item_id`), sets
  `external_id` only when `new_external_id` is `Some`, calls `append_alias`,
  and retitles the H1 only when it reads `# <old|new>: <title>`.
- **`append_alias`** (`:589-624`) appends the old ID to inline or block lists
  and never removes `new_id`, which the story requires for the `ACC-0042 →
  0042` round trip.
- **Adapter**: `apply_retirement` (`cli/work-adapters/src/retirement.rs:403-433`)
  locks touched paths, verifies plan-time bytes, writes recovery copies under
  `.accelerator/state/retirement-recovery/<old>--<new>/`, then runs
  rewrites → `Create to` → `Remove from` → `RenameBaseline`
  (`steps`, `:163-187`). `finish_retirement` (`:516-539`) re-plans once on
  rollback. `retirement_outstanding` (`:492-503`) is true while the plan still
  changes the corpus or the baseline still keys the old ID.
- **Lock**: `acquire_retirement_lock` (`:72-77`) returns a
  `RetirementLockGuard` that `finish_retirement` only borrows, so one guard
  can span a whole rekey run.

Resume is by re-planning: the interrupted-target rule exempts the half-written
target from collision checks and reconstructs `from` via `home_of(old_id)`
(`:319-325`). The planner resumes only when handed the same `old → new` pair,
which is why the story persists the run's mapping.

Gaps for `rekey`:

| Gap | Where | Consequence |
|---|---|---|
| Destination fixed to `work_dir` | `retirement.rs:377-381`, `:455-459` | `{tracker}` unsynced → `drafts/` needs a destination rule, e.g. `home_of(new_id)` |
| Path spellings drafts → root only | `retirement.rs:327-341` | root → drafts links lose the `drafts/` segment |
| Interrupted-target `to` vs fresh `to` | `retirement.rs:360-384` | must agree once drafts destinations exist |
| `new_id` kept in `aliases` | `retirement.rs:589-624` | round trips leave a self-alias |
| No cross-run collision check | `refuse_collisions`, `:471-500` | duplicate targets surface only on the second plan |
| Full corpus walk per retirement | `plan_from_corpus`, adapter `:463-471` | O(M × corpus) reads; 0230 plan left batching to 0302 |

Swaps (A ↔ B) cannot occur: a retired ID stays held as an alias and
`holder_of` (`cli/work/src/identity.rs:93-103`) counts aliases. Chains
(A → B while B → C) can, under numeric patterns, and need ordering or the
upfront in-run collision check the story specifies.

### Reference rewriting

`corpus::references::rewrite_references(content, &Renaming)`
(`cli/corpus/src/references.rs:29-42`) runs a frontmatter/`aliases`/body
state machine and never rewrites the `aliases` region.

- **Shapes**: `IdShape::{Distinctive, NumericOnly}` (`:4-12`). `Distinctive`
  rewrites every whole token; `NumericOnly` rewrites only
  `work-item:<old>` typed links (`typed_link`, `:102-104`).
- **Token boundary**: a match must not be adjacent to ASCII alphanumerics,
  `-` or `_` (`joins_a_token`, `:106-108`); matching ignores ASCII case. So
  `PP-760` inside `PP-7601`, `PP-760-notes` and `PP-760_x` are untouched;
  `(PP-760)` and `PP-760.` are rewritten. This satisfies the `PP-7601`
  acceptance criterion as is.
- **`shape_of`** (`cli/work/src/retirement.rs:286-292`) is
  `DraftId::parse(id).is_some() || corpus::is_tracker_key(id)`.
  `is_tracker_key` (`cli/corpus/src/work_item_id.rs:310-319`) accepts
  `[A-Za-z][A-Za-z0-9_]*-[0-9]+`, so `ACC-0042` is `Distinctive`.
- **Review links**: `rewrite_review_link` (`retirement.rs:521-545`) also
  rewrites `work_item_id` in `type: work-item-review` frontmatter.
- **Roots** are every configured doc-type directory, not literally `meta/`
  (`corpus_roots`, `cli/work-cli/src/sync.rs:173-181`).

### Identity, resolution and drafts

- `identity_of` (`cli/work/src/work_item_files.rs:37-47`) reads `id`
  (fallback `work_item_id`), `aliases`, `external_id`. Adding `date` for
  allocation order means a new read; `date` is only written at creation
  (`cli/work/src/create.rs:85-101`).
- `resolve_identity` (`cli/work/src/identity.rs:65-88`) matches id, alias or
  `external_id` case-insensitively; `work resolve` uses it
  (`cli/work-cli/src/resolve.rs:119-145`), so the "resolve old ID" criterion
  holds once the alias lands on exactly one item.
- `FilesystemWorkItemFiles::files()` includes `drafts/`; `canonical()`
  excludes it (`cli/work-adapters/src/filesystem.rs:49-106`).
- `DraftId` is `draft-` + 6 lowercase Crockford base-32 characters with at
  least one letter (`cli/work/src/draft_id.rs:13-28`). `mint_draft_ids(draws,
  items, count)` (`:85-103`) mints a batch without repeats and avoids every
  `id` and alias, so one call can serve all unsynced items in a `{tracker}`
  run.

### ID patterns and number allocation

- **Pattern DSL**: `{number[:0Nd]}`, `{key}` (deprecated `{project}`),
  `{tracker}` as sole token. Compiled in
  `cli/corpus-adapters/src/work_item_pattern.rs` to a scan regex
  (`compile_scan_regex`, `:332-337`) or a printf string
  (`compile_format_string`, `:345-350`). `WorkItemIdScheme::ownership()`
  (`cli/corpus/src/work_item_id.rs:97-103`) returns `Tracker` iff the pattern
  is exactly `{tracker}`.
- **`parse_full_id`** (`work_item_pattern.rs:465-495`) parses only against the
  current pattern and returns `TrackerHasNoNumber` under `{tracker}`.
- **Shape-agnostic sequence extraction**: `split_prefix_sequence`
  (`cli/work-adapters/src/sync/ordering.rs:38-48`, private) splits on the last
  `-`; m0002's `split_legacy_id` (`cli/migrate/src/migrations/m0002.rs:202-216`)
  accepts exactly four leading digits. Neither handles a bare `0042` and a
  prefixed `ACC-0042` uniformly; the rekey classifier needs its own
  domain function ("keeps its sequence number" for any non-tracker-key,
  non-draft ID).
- **`allocate(scheme, project, count, filenames, scanner)`**
  (`cli/work/src/next_number.rs:77-141`). Pure, reserves nothing, seeds
  `highest = 0`, scans top-level `.md` filenames with the current pattern's
  scanner, returns `Overflow { partial, .. }` past `10^w − 1`. A floor
  parameter is the minimal change for "fresh numbers start above the highest
  kept or aliased sequence number in the target shape".
- **Rendering**: `WorkItemIdScheme::canonicalise_id`
  (`cli/corpus/src/work_item_id.rs:189-213`) renders a number in the current
  shape; `allocate` already uses it.
- **Config**: `resolve_scheme` (`cli/work-cli/src/config.rs:54-91`) reads
  `work.id_pattern` and `work.key` (fallback `work.default_project_code`).

### Tracker refresh under `{tracker}`

```text
            ids with external_id
                     |
          Bulk       |        PerItem
     fetch_all(ids) -+- show(id) --Err--> locate(id)
          |                 |                 |
   Err -> exit 70    Ok: key differs?   Err -> exit 70
          |                 = moved     Found(issue.key) / NotFound
   found: unmoved
   absent/indeterminate --> locate(id)
                              Err -> exit 70
```

- **Port**: `RemoteTracker` (`cli/tracker/src/lib.rs:594-805`). `locate ->
  Result<Located, TrackerError>` (`:676`); `Located::{Found(RemoteIssue),
  NotFound}` (`:137-144`); a move is `Found` with `issue.key != requested`
  (`RemoteIssue.key` doc, `:107-110`). `FetchOutcome { found, absent,
  indeterminate, completeness }` (`:270-323`).
- **Sync's swallowing**: `Gathering::locate` (`fetch.rs:216-238`) maps
  `Err(_)` to the bulk verdict at `:236`; a bulk `Err` sets `read_failure`
  without calling `locate` (`:276-284`); a `show` error after a changed stamp
  records `Present` (`:364-371`). `rekey` needs its own refresh function
  rather than reusing `gather`: it needs neither stamps nor bodies, and every
  one of those branches must become "unreachable".
- **Move semantics**: `is_move` (`fetch.rs:172-174`) ignores case-only
  changes. `decide_key_change` (`cli/work/src/sync/key_change.rs:51-73`)
  returns `RetireKey` only under `Tracker` ownership when `id` equals the old
  key; under `{tracker}` rekey targets `external_id` regardless of `id`, so
  it uses the refreshed key directly rather than `decide_key_change`.
- **Adapters already satisfy the new obligation**: Linear indexes current
  `identifier` (`cli/linear-client/src/client.rs:927-996`); Jira matches
  returned keys exactly (`cli/jira-client/src/client.rs:708-785`). No test
  feeds Jira's `fetch_all` a moved key.
- **Contract**: `partitions_totally` (`cli/tracker-test-support/src/contract.rs:91-115`)
  checks totality only. `ContractSubject` (`:28-87`) has no "moved issue"
  hook; the new property needs one (e.g. a `moved_pair()` method) since live
  subjects cannot move issues on demand. `RecordingTracker::moving(old, new)`
  (`cli/tracker-test-support/src/lib.rs:291-304`) already models the
  behaviour; offline Jira/Linear subjects need a mock route returning the
  moved issue under its new key.
- **Fakes for "unreachable"**: `failing_locate`, `truncating`,
  `transient_keyed_read` on `RecordingTracker`; the only whole-call
  `fetch_all` `Err` fake is `FetchFailingTracker`
  (`cli/work-cli/src/sync.rs:3491-3568`).

### `work sync` state the guards read

- **`RetirementRecord`** (`cli/work/src/retirement.rs:59-91`), stored by
  `FileRetirementRecords` at `.accelerator/state/retirement-records/<old>--<new>.json`
  (`cli/work-adapters/src/retirement_records.rs:41-62`). `outstanding()`
  returns `Err` if any record is malformed (`:133-164`) — treat as
  unfinished. `prepare()` (`:64-85`) writes a verified `*` `.gitignore`: the
  precedent for `.accelerator/state/rekey/`.
- **Promotion records** in stage `retiring` (`kind: "retiring"`) live in
  `<integrations>/<integration>/pending-push/<draft-id>.json`
  (`cli/work-adapters/src/sync/pending_push.rs:130-138`, `:325-358`). These are
  the "promotion `work sync` recorded but has not completed" half of the
  refusal. `FilePromotionRecords::outstanding()`
  (`cli/work-adapters/src/promotion_records.rs:155-174`) enumerates them.
- **`pending_push` markers** per item: legacy `PendingPush::{Attempted,
  Created}` (`cli/work/src/sync/push_precondition.rs:22-30`) keyed by slug or
  id; read via `pending_push::read` or enumerate via `pending_push::outstanding`
  (`pending_push.rs:486-526`). Unreadable markers fail closed elsewhere and
  should here too.
- **Sync guard insertion**: in `run_sync` after `IdentityWorkspace::open`
  (`cli/work-cli/src/sync.rs:1487`) and before `confirm_remote_candidates`
  would need `state_dir` computed earlier (`repo_root.join(STATE_DIR)`,
  `:169`). The 0302 criterion "changes nothing" argues for placing it before
  any remote read (`:1458`) and before `IdentityWorkspace::open`, which
  creates directories. `work promote` and `work create --push` also retire
  (`promote.rs:228`, `create.rs:542-570`); the story names only `work sync`.
- **Baseline**: `<integrations>/<integration>/last-sync.json`, keyed by local
  `id`; retirement renames the entry last (`BaselineStore::rename`,
  `cli/work-adapters/src/sync/baseline_store.rs:95-103`).

### Dirty-tree preflight and the force override

- **Domain**: `Preflight` (`cli/migrate/src/preflight.rs:58-108`) takes the run
  lock, observes `SCOPES = ["meta/", ".claude/accelerator", ".accelerator/"]`
  (`:17`), drops runner-managed paths, classifies the rest as owned/unowned
  against the manifest, and returns `Clean`, `Resumed` or
  `PreflightError::UnownedChanges { paths, stale_run }`.
- **Port**: `WorkingCopy::observe(roots) -> Result<WorkingCopyObservation,
  MigrationError>` (`cli/migrate/src/ports.rs:364-381`).
- **Adapter**: `VcsWorkingCopy::observe` swallows probe errors to
  `WorkingCopyState::default()` with a `warn!` (`working_copy.rs:41-51`);
  `InProcessProbe` returns default for "no VCS" too
  (`cli/vcs-adapters/src/library.rs:557-570`). `work-adapters` already models
  the fail-closed alternative as `Dirtiness::Unknown`
  (`cli/work-adapters/src/sync/working_copy_status.rs:23-71`).
- **Force**: `force_requested()` reads `ACCELERATOR_MIGRATE_FORCE`
  (`cli/migrate-cli/src/main.rs:100-103`). 285 occurrences in 103 files;
  outside `meta/`: `migrate-cli` source (`main.rs:101`, `cli.rs:16`,
  `render.rs:14,16,18,315`), seven test files, ~30 golden fixtures plus
  `tests/fixtures/regenerate.sh`, `skills/config/migrate/SKILL.md`,
  `docs-site/src/content/docs/migrations.md:18`, `CHANGELOG.md`.
- **Where to lift it**: `vcs` is a `platform` domain (admits everyone) that
  both adapter crates already depend on; its pup rule allows only std,
  `kernel::Error`, `crate` (`cli/pup.ron:80-94`). The scope filter,
  `RunBase`, dirty-path observation and a probe-failure policy fit there; the
  run lock and manifest ownership are migrate-specific and stay. A new crate
  triggers the seven-step checklist in `tasks/README.md:790-858`.
- **Resume exemption**: migrate's owned/unowned split
  (`cli/migrate/src/manifest.rs:33-64`) is the same concept as rekey's
  "ignore dirty paths in the run plan's write set". A shared preflight that
  accepts an ownership predicate covers both.

### CLI wiring, exit codes and tests

- **Command**: add `Rekey(Box<RekeyArgs>)` to `Command`
  (`cli/work-cli/src/cli.rs:21-151`) with `--preview`, `--discard` (mutually
  exclusive) and possibly `--per-item-reads`, modelled on `SyncArgs`
  (`:336-390`). Dispatch in `main.rs:555-576`; `Intent::Write` for the
  personal-file gate (`main.rs:54-82`). No launcher, manifest or
  dispatch-coherence change (`tasks/README.md:769-788`); `Bash(accelerator
  work *)` already covers the skill.
- **Wiring precedent**: `promote.rs:171-268` (`IdentityWorkspace::open`,
  retirement ports, records, `finish_interrupted_retirements`, severity fold).
- **Exit codes**: `cli/work-cli/src/exit_codes.rs` is the single authority
  (constants `:125-140`, per-command docs `:1-121`); `RETRYABLE = 70`. New
  refusal codes need the collision test (`:156-179`),
  `exit_codes_parity.rs` and `keyword_exit_codes.rs`.
- **Tests to extend**: `cli_surface.rs` `SUBCOMMANDS` + golden;
  `personal_file.rs` writer list (`:60-65`); a `rekey_resolves_argv.rs` like
  `sync_resolves_argv.rs`; in-process fake-tracker tests via
  `test_support::StubRegistry` (`cli/work-cli/src/test_support.rs:20-29`);
  adapter tests beside `cli/work-adapters/tests/retirement.rs`;
  `RefusingProxy` (`cli/work-cli/tests/cli_sync.rs:301-363`) proves "no
  tracker call".

### `/rekey-work-items` skill

- Picked up automatically: `.claude-plugin/plugin.json:19` registers
  `./skills/work/`.
- `sync-work-items/SKILL.md` injects config with `!` reads (`:16-19`, each
  `--fail-safe`) and maps exit codes in prose, but its `--preview` is
  user-driven; the preview → `AskUserQuestion` → apply shape is in
  `skills/work/update-work-item/SKILL.md:234-261`. Decline must run
  `work rekey --discard`.
- Guards: `tasks/lint/skill_permissions.py` (`EXPECTED_INJECTION_SKILLS = 43`
  if context/instructions are injected), `tasks/lint/bare_invocation.py`,
  `tests/unit/tasks/test_skill_frontmatter_validation.py`
  (`IN_SCOPE_PRODUCERS` if the skill validates frontmatter after writing).
- Docs that state "legacy items keep their IDs" and need updating:
  `docs-site/src/content/docs/guides/sync-work-items.mdx:186-187`,
  `guides/configuration-cookbook.md:184-185`; tables in
  `skills/work-items.md` and `guides/which-skill.md:33-47`.

## Code References

- `cli/work/src/retirement.rs:30-91` — `Retirement`, `RetirementRecord`
- `cli/work/src/retirement.rs:286-341` — `shape_of`, `home_of`, `path_spellings`
- `cli/work/src/retirement.rs:348-444` — `plan_retirement`
- `cli/work/src/retirement.rs:471-500` — `refuse_collisions`
- `cli/work/src/retirement.rs:589-671` — `append_alias`, `retired_item_content`
- `cli/work-adapters/src/retirement.rs:403-539` — `apply_retirement`, `finish_retirement`
- `cli/work-adapters/src/retirement_records.rs:41-177` — record store and `*` `.gitignore`
- `cli/corpus/src/references.rs:4-138` — `IdShape`, `rewrite_references`, token boundary
- `cli/work/src/identity.rs:5-117` — `ItemIdentity`, `resolve_identity`, `holder_of`, `linker_of`
- `cli/work/src/draft_id.rs:13-103` — `DraftId`, `mint_draft_id(s)`
- `cli/work/src/next_number.rs:77-141` — `allocate`
- `cli/corpus-adapters/src/work_item_pattern.rs:332-590` — pattern compile, `parse_full_id`, `canonicalise_id`
- `cli/work-adapters/src/sync/ordering.rs:38-48` — `split_prefix_sequence`
- `cli/tracker/src/lib.rs:137-144,270-323,594-805` — `Located`, `FetchOutcome`, `RemoteTracker`
- `cli/tracker-test-support/src/contract.rs:91-115,226-247` — totality contract
- `cli/tracker-test-support/src/lib.rs:291-304,605-639` — `RecordingTracker::moving`, fake `fetch_all`
- `cli/work-adapters/src/sync/fetch.rs:36-40,216-373` — `RetrievalStrategy`, `gather`, `locate` swallow
- `cli/work-adapters/src/sync/identity_settlement.rs:194-277,541-554` — `finish_locked`, `reconcile`, `retire_key`
- `cli/work-adapters/src/sync/pending_push.rs:31-138,325-526` — markers and promotion records
- `cli/migrate/src/preflight.rs:17-142` — `Preflight`
- `cli/migrate-adapters/src/working_copy.rs:37-62` — probe failure treated as clean
- `cli/migrate-cli/src/main.rs:100-103,266-295` — force read, preflight wiring
- `tasks/lint/crate_dependencies.py:56-459` — context admission rules
- `cli/work-cli/src/cli.rs:21-151,336-411` — `Command`, `SyncArgs`, `PromoteArgs`
- `cli/work-cli/src/exit_codes.rs:1-179` — exit-code authority
- `cli/work-cli/src/sync.rs:1344-1684` — `run_sync` startup sequence
- `skills/work/sync-work-items/SKILL.md`, `skills/work/update-work-item/SKILL.md:234-261` — skill models

## Architecture Insights

- **Run plan vs `RetirementRecord`.** `RetirementRecord`s are owned by sync's
  `reconcile`; if rekey wrote them, any `work sync` or `work promote` would
  finish rekey's items, and rekey's own "unfinished retirement record" refusal
  would fire on its own interrupted run. The run plan in
  `.accelerator/state/rekey/` should be rekey's sole durable record, checked
  before the sync-record refusal.
- **Batch planning wants one corpus snapshot.** Classifying every item,
  allocating, checking in-run collisions and computing the write set all need
  the same snapshot; per-item `finish_retirement` then re-walks. Accepting the
  O(M × corpus) cost is simplest; a batch planner over one snapshot is the
  optimisation 0230 deferred.
- **Write set is knowable up front.** `plan_retirement` returns rewrites,
  `from`, `to` and recovery paths, so planning every retirement against the
  snapshot before the first write yields "every path the run will write" for
  the run plan. Plans for later items change as earlier ones land, so the
  recorded set is a superset bound, not an exact replay.
- **Domain split.** Classification (tracker key / draft / kept sequence
  number), target derivation, ordering by `date` then byte-wise `id`, and
  in-run collision detection are pure and belong in `work`; the tracker
  refresh, preflight, run-plan store and retirement application belong in
  `work-adapters`/`work-cli`.
- **Fail-closed is the house style in `work`.** Unreadable promotion records
  count as pending; malformed retirement records error; probe failure is
  `Dirtiness::Unknown`. Rekey's fail-closed probe matches `work`, not
  `migrate`.

## Historical Context

- `meta/decisions/ADR-0070-tracker-owned-work-item-identity.md` — `id` changes
  only through promotion, key change or explicit re-key, all via retirement;
  legacy IDs are kept and translation deferred to 0302.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  classify by presence of `external_id`, never by `id` shape.
- `meta/plans/2026-09-26-0230-tracker-owned-work-item-id-generation.md` —
  `plan_retirement` is the sole collision authority; dirtiness never blocks a
  retirement; "batching retirements is left to 0302"; lock order
  retirement → create → per-file.
- `meta/plans/2026-09-23-0241-migrate-false-dirty-tree-detection.md` —
  run-base ownership; fail-open probe kept unchanged with no stated rationale.
- `meta/reviews/work/0302-work-item-re-key-command-review-1.md` — approved;
  pass-3 minors deferred to planning (refresh triggers per invocation,
  fingerprint fields and body-only edits, undated items, prefixed IDs that
  differ from `external_id`, non-bulk strategy, preview line order, tracker
  rate limits, delivery order).
- `meta/work/0343-alias-normalisation-for-re-keyed-work-items.md` — joins the
  run plan, preview binding and dirty-tree refusal 0302 introduces.
- `meta/work/0346-remove-deprecated-accelerator-migrate-force.md` — removes the
  alias this item introduces.

## Related Research

- `meta/research/codebase/2026-09-26-0230-tracker-owned-work-item-id-generation.md`
- `meta/research/codebase/2026-09-23-0241-migrate-false-dirty-tree-detection.md`
- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- `meta/research/codebase/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`

## Open Questions

- **Retrieval strategy surface**: add `--per-item-reads` to `work rekey`, or a
  config key both commands read?
- **Undated items**: `date` is optional in practice; where do items without
  one sort in fresh-number allocation?
- **Fingerprint scope**: the story names path, `id`, `aliases`,
  `external_id`; whether a body-only edit (which changes the rewrite set)
  should invalidate a proposed plan is unresolved by the review.
- **Sync-guard reach**: should `work promote` and `work create --push`, which
  also retire, refuse during an in-progress run plan as `work sync` does?
- **Preflight home**: extend `vcs` with the observation/scope/policy pieces,
  or create a `shared` working-copy context with its own adapter crate.
- **`{tracker}` item with `id` already a draft but `external_id` set**: the
  story leaves `draft-` items for sync; confirm that holds when a promotion
  record is mid-flight (it refuses via the pending-push check).
- Not checked: whether every file under `.accelerator/state/` is committed in
  this repository (no `jj file list` run), and Jira `fetch_all` behaviour on a
  moved key beyond code reading.
