---
type: "codebase-research"
id: "2026-09-26-0230-tracker-owned-work-item-id-generation"
title: "Tracker-Owned Work Item ID Generation: implementation surface"
date: "2026-09-26T00:53:23+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0230"
parent: "work-item:0230"
topic: "Tracker-Owned Work Item ID Generation: implementation surface"
tags: ["research", "codebase", "work-cli", "work-adapters", "tracker", "sync", "corpus", "id-pattern", "drafts", "pending-push"]
revision: "48b03a8c6bb4b6cc181f13a64be5ef0052b8e507"
repository: "accelerator"
last_updated: "2026-09-26T00:53:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tracker-Owned Work Item ID Generation: implementation surface

**Date**: 2026-09-26T00:53:23+00:00
**Author**: Toby Clemson
**Git Commit**: 48b03a8c6bb4b6cc181f13a64be5ef0052b8e507
**Branch**: none (detached jj working copy in the `ticket-management`
workspace)
**Repository**: accelerator

## Research Question

What does the codebase look like today across every surface work item 0230
(Tracker-Owned Work Item ID Generation) touches — `work.id_pattern`
parsing and validation, `work create --push` and its outcomes, `pending_push`
markers, `work sync` push/pull and remote fetch, the corpus readers
(`work resolve`, `work list`, `corpus frontmatter validate`, visualiser), the
work-item skills, and any reusable machinery for ID retirement — and where
does 0230's specification rest on assumptions the code does not yet satisfy?

## Summary

Nothing in 0230 is a small extension of an existing seam; most requirements
are new capability. The load-bearing findings:

1. **`{tracker}` cannot pass the current pattern grammar.** `compile()`
   rejects any pattern without `{number}` and any unknown token, and every
   downstream consumer (scan regex, `canonicalise_id`, `next-number` cap,
   visualiser) assumes a numeric token. There is also no load-time config
   validation at all — problems surface lazily at compile time — and
   `accelerator config validate` (0227) does not exist.
2. **An unreachable tracker is `loud-terminal`, not `local-save`, on create.**
   Both clients classify transport failures (connect, DNS, timeout) on a
   mutation as `Terminal` (71). `local-save` arises only from selection
   failures (72/73/74) or two provable rejections (70). 0230's "tracker
   unreachable → `local-save` → `draft — tracker unreachable`" does not match
   today's classification.
3. **Sync cannot observe a tracker-side key change.** `RemoteIssue` carries no
   key; neither adapter compares returned and requested keys; Linear's
   `fetch_all` never queries by key; `previousIdentifiers` is unused. A moved
   issue becomes `RemoteAbsent` (exit 4), and untracked discovery then
   imports the new key as a second local file.
4. **No multi-file transaction exists.** ID retirement's all-or-nothing
   guarantee is new code. `store::atomic_write` is per-file; the closest
   precedent (migration `m0002`) is non-transactional and relies on VCS revert.
5. **State outside `meta/` is keyed by local `id`.** The `last-sync.json`
   baseline and sync's own `pending_push` markers use the local `id`; 0230's
   retirement steps do not mention re-keying them.
6. **Every work-item reader except frontmatter validation scans `meta/work/`
   flat.** `work resolve`, `work list`, `work sync` discovery, allocation, and
   the visualiser would not see `meta/work/drafts/`.
7. **`aliases: []` fails validation today** (EMPTY-PLACEHOLDER exempts only
   `tags`), and adding it to the template trips `work create`'s drift guard.
8. **0230 contradicts accepted ADR-0044**, which fixes `id` as locally
   allocated and explicitly rejected reusing the remote key as `id`.

## Detailed Findings

### ID pattern and configuration

- `work.id_pattern` is a plain `String` carried in
  `WorkItemIdScheme { id_pattern, key }`
  (`cli/corpus/src/work_item_id.rs:19-39`); catalogue default `{number:04d}`
  (`cli/config/src/catalogue.rs:108-113`).
- The token DSL is compiled only on demand by
  `cli/corpus-adapters/src/work_item_pattern.rs`. `compile()` (`:218-297`)
  enforces: at least one `{number}` (`NoNumberToken`, rule 1), no hostile
  literals (rule 2), no adjacent dynamic tokens (rule 3), valid key value
  (rule 5), no unknown tokens. `PatternError` messages at `:17-93`.
- There are **two independent pattern implementations** and `{tracker}` must
  land in both: the regex compiler above (used by `next_number`, `create`,
  `canonicalise_id`, visualiser `compose.rs:197`) and the regex-free
  `WorkItemIdScheme::canonicalise_id`/`is_canonical_id_token`/`normalise_id`
  in `cli/corpus/src/work_item_id.rs:45-143` (used by filter, resolve,
  cluster, indexer). `cli/work/src/resolve.rs:114-164` has a third,
  greedy segment matcher (`match_full_id`).
- Number-width parsing is duplicated four times with differing defaults
  (`canonical_digit_width` → 0, `number_width` → 4, `explicit_width` in
  `work/src/next_number.rs:30-45` → 4, `explicit_number_width` in
  `corpus-adapters` → 4).
- **No load-time validation.** `resolve_scheme`
  (`cli/work-cli/src/config.rs:52-79`) is the shared resolver for create,
  next-number, resolve, list, sync and sync_author; it enforces only
  `E_WORK_KEY_REQUIRED`. Grammar errors surface when a consumer calls
  `compile_scan_regex`. `resolve_scheme` is the natural seam for the
  `{tracker}` only-token and jira/linear rules.
- `work.integration` values: `["jira","linear","trello","github-issues"]`
  plus empty (`catalogue.rs:117-125`). Checked by `config work integration`
  (`cli/launcher/src/config_command/core/work.rs:17-62`), `config dump`
  (`core/dump.rs:295-310`), and `SelectionError` at sync/tracker time
  (`cli/work-cli/src/tracker_registry.rs:27-67`, dispatch `:295-321`).
- `accelerator config validate` does not exist: no `Validate` in the launcher
  `Action` enum (`cli/launcher/src/config_command/inbound/cli.rs:31-112`).
  Per 0230's Dependencies, 0227 therefore carries the `{tracker}` rules; 0227
  requires they live where it can reuse them (the `PatternError` DSL and the
  integration check).
- `work.key` is the local prefix only, never derived from the scope key
  (`config.rs:28-47`); under `{tracker}` it is unused, so
  `E_WORK_KEY_REQUIRED` never fires.

### `work create` and `--push`

- `try_run` (`cli/work-cli/src/create.rs:609-712`): resolve scheme → take
  `.accelerator-work-create.lockdir` (held across the whole remote push,
  `:624-628`) → `allocate_id` (`:183-200`, flat filename scan) → resolve
  template and body, substituting `NNNN` and the title placeholder, which is
  how the H1 `# <id>: <title>` is produced (`resolve_body`, `:241-258`) →
  target `work_dir/{id}-{slug}.md`, refusing if present (`:645-651`) →
  optional `execute_push` → `render_frontmatter` with `external_id` only when
  `Some` (`work/src/create.rs:147-152`) → one `AtomicWrite::write` → delete
  marker with `remove_file(..).ok()`.
- `--dry-run` (`preview_push`, `create.rs:387-414`) never allocates or writes;
  Jira's `preview_create` does a live project existence check.
- `PushOutcome { WriteOnce, Retry, LocalSave, LoudTerminal }` and
  `push_decide(code, attempt, write_failed)`
  (`cli/work/src/sync/push_decide.rs:8-65`, golden
  `cli/work/tests/fixtures/work-item-push-decide.golden`):

  | Code | Condition | Outcome |
  |---|---|---|
  | 0 | write ok | `WriteOnce` |
  | 0 | write failed | `LoudTerminal` |
  | 70 | first attempt | `Retry` |
  | 70 | second attempt | `LocalSave` |
  | 72/73/74 | — | `LocalSave` |
  | 71 / other | — | `LoudTerminal` |

- `create.rs` always passes `write_failed = false` (`:437`, `:525-529`). A
  local write failure after a remote create is `Failed` → exit 1, not
  `LoudTerminal`, and is **not retried**; recovery is a rerun that hits the
  `created` marker's `ReuseId`. 0230's "retry the write once, then
  write-failed `loud-terminal`" is new behaviour.
- Today `LoudTerminal` **still writes the local file** (without
  `external_id`) and exits 71; `LocalSave` writes it and exits 0. 0230
  replaces both writes with drafts.
- Error classification: Jira (`jira-client/src/classify.rs:47-62`) and Linear
  (`linear-client/src/classify.rs:127-159`) both treat transport failures and
  5xx as `Terminal` on create. Only provable rejections are `Retryable`.
  Consequently "tracker unreachable" on create yields `LoudTerminal` with an
  `attempted` marker — see Open Questions.
- Creation home: Jira uses `auth::project_code`
  (`jira-client/src/auth.rs:194-210`, `jira.project_key` → deprecated
  fallback). Linear creates with `teamId: credentials.team_id`
  (`linear-client/src/client.rs:626-658`), resolved from `linear.team_id`
  then the catalogue base team (`linear-client/src/auth.rs:83-93`);
  `linear.team_key` drives scope, not the create target.
- Tests: unit tests in `create.rs:738-898`; subprocess tests in
  `cli/work-cli/tests/cli_create_push.rs`. **No test drives a successful or
  failing `tracker.create` through `work create --push`** — the registry is
  not injectable at the binary boundary, and the file header claiming no
  client is wired is stale.

### `pending_push` markers

- Path `<integrations>/<integration>/pending-push/<name>.json`
  (`cli/work-adapters/src/sync/pending_push.rs:32-39`); directory gets a
  self-written `*` `.gitignore` via `prepare_dir` (`:51-65`).
- JSON: `{"kind":"attempted"|"created","title","digest","attempted_at",
  "failure","external_id"?}` (`render`, `:155-183`; `read`, `:114-153`).
- Domain: `PendingPush`, `PushPrecondition { Proceed, ReuseId, Refuse }`,
  `RefusalReason { MarkerUnreadable, PriorAttemptUnknownOutcome,
  FingerprintMismatch, AlreadyWritten }`
  (`cli/work/src/sync/push_precondition.rs:11-95`).
- **Two naming namespaces:** `work create` names the marker by
  `slugify(title)` (`create.rs:479-483`); sync's `create_from_local` names it
  by the local `id` (`cli/work-adapters/src/sync/run.rs:747-751`). 0230's
  draft-ID naming unifies these for drafts.
- The fingerprint digest (`request_digest`, `push_precondition.rs:74-88`) is
  SHA-256 over title, **resolved body (with the local id already
  substituted)**, and kind. Under `{tracker}` the body cannot contain the
  final id before the create; the digest must be computed over the
  draft-ID-substituted or placeholder body and the tracker key substituted
  only after the create.
- `ReuseId` requires a matching digest; 0230 requires a `created` marker's key
  to be adopted **even after the draft was edited**, so promotion must bypass
  the digest check that `work create` enforces.
- Sync never adopts `work create`'s markers: pull ignores markers entirely
  (`discover_untracked`, `run.rs:546-573`), so an orphaned `created` marker's
  issue is imported as a fresh file and the marker left behind.
  `warn_outstanding_pushes` (`cli/work-cli/src/sync.rs:69-99`) only warns.

### `work sync`

- Flags (`cli/work-cli/src/cli.rs:268-314`): `--push-only`, `--pull-only`,
  `--preview`, `--resolve`, `--per-item-reads`, `--max-pulls`, `--max-pushes`,
  `--allow-unbounded`, `--target`. No `--no-promote`.
- `run_sync` (`sync.rs:1053-1386`) loads the corpus with `discover_items`
  (`:101-137`): flat `read_dir`, keyed on frontmatter `id` only.
- Baseline `<integrations>/<integration>/last-sync.json`
  (`cli/work-adapters/src/sync/baseline.rs:18-46,228-232`) is **keyed by local
  `id`**. An item whose `id` changes without its baseline entry moving loses
  its digests and surfaces as a conflict (the 0296 symptom).
- Engine (`run.rs`): `prepare_run` (`:898-979`) → `fetch::gather` → plan →
  `untracked_to_import` (`:814-887`) → bounds. `run` (`:994-1108`) applies
  create-from-remote, then planned actions, then create-from-local, then
  `finalise_baseline`. Exit precedence in `exit_code_for_report`
  (`sync.rs:245-283`): 71 > 4 > 74 > 70 > 0.
- **Pull of remote-only issues**: `ItemApplier::create_from_remote`
  (`apply.rs:272-311`) → `ConfiguredLocalAuthor::author_from_remote`
  (`cli/work-cli/src/sync_author.rs:98-160`), which takes the create lock and
  calls `create::allocate_id` (`:111-116`). **No H1 is written** — content is
  `"{frontmatter}\n{description}\n"` (`:149`). 0230's `{tracker}` adoption
  replaces the `allocate_id` call with `external_id`.
- **Fetch**: `fetch::gather` (`fetch.rs:153-270`) bulk mode matches by exact
  `ExternalId` equality. Jira `fetch_all` uses JQL `key IN (...)` and compares
  returned `key` with requested by exact string (`client.rs:631-708`, match at
  `:668-671`). Linear `fetch_all` pages all issues of catalogued teams and
  indexes by `identifier` (`linear-client/src/client.rs:810-879`). `show` in
  neither adapter compares returned and requested identifiers.
- **Key-change detection is absent.** `RemoteIssue { updated, body }`
  (`tracker/src/lib.rs:107-132`) has no key. A moved Jira issue returned under
  a new key misses the exact comparison → `absent`; a moved Linear issue is
  `absent` or `indeterminate`. Either way the item is `RemoteAbsent`
  (`work/src/sync/classify.rs:119-123`) → Noop, awaiting-human, exit 4. On a
  full run, discovery also imports the new key as a second file.
- "Not found under stored key" and "moved" are indistinguishable today;
  reads are always `Retryable` (`tracker/src/lib.rs:634-642`).
- Sync never renames or moves files.
- Test doubles: `RecordingTracker` in `cli/tracker-test-support/src/lib.rs`
  (builders `:121-295`, exact `ExternalId` matching `:371-429`); engine tests
  in `cli/work-adapters/tests/sync_run.rs`, `sync_create.rs`;
  real-client-over-mock-server in `sync_run_real_client.rs`.

### Corpus readers

| Reader | Location | Scan | Keys on |
|---|---|---|---|
| `work resolve` | `cli/work/src/resolve.rs`, `cli/work-cli/src/resolve.rs` | flat (`FilesystemLister`) | filename prefix, case-sensitive |
| `work list` | `cli/work-cli/src/list.rs:109-147` | flat | frontmatter `id` / `work_item_id` |
| `work sync` | `sync.rs:101-137` | flat | frontmatter `id` |
| allocation, `external_id` dup check | `create.rs:183-200,260-288` | flat | filename / `external_id` |
| `corpus frontmatter validate` | `corpus-adapters/src/fs.rs:60-113` | recursive | `type:id` index |
| visualiser | `visualiser/server/src/file_driver.rs:242-322` | flat (watcher recursive) | — |

- **`work resolve`** (`classify_input`, `resolve.rs:170-197`): tokens with `/`
  are Path (accepted if under the canonical work dir, so a drafts path
  resolves); others must match the pattern greedily or be all digits, else
  `E_RESOLVE_INVALID` (exit 1). `draft-k7mq3x` is Invalid under
  `{number:04d}`. FullId matches filename `starts_with("{input}-")`,
  case-sensitive. Ambiguity exits 2 with tagged candidates; not found exits 3.
  No alias or `external_id` matching. ADR-0068's `corpus resolve --type`
  shares this classification, so changes may ripple there.
- Hazard: the cross-project BareNumber source (`resolve.rs:304-325`) would
  pick up a flat `draft-0042-…` file as a `draft`-tagged candidate.
- **`work list` Sync column** (`classify_labels`, `list.rs:652-675`): shown
  whenever `work.integration` is set, baseline or not; with no baseline every
  item gets a presence-only label (`🟢 synced` / `⚪ unsynced`). The skill
  text (`list-work-items/SKILL.md:29-30,146-153`) claims the column is omitted
  without a baseline — code and skill disagree, and 0230's AC ("with the draft
  removed and no sync baseline, the column is absent") matches the skill, not
  the code.
- **`corpus frontmatter validate`**: work-item schema row at
  `cli/corpus/src/frontmatter_validation/schema.rs:20-43` (mirrored in
  `templates-schema.tsv`); typed-linkage keys `parent, blocks, blocked_by,
  derived_from, relates_to, source`. `id` shape is **not** validated beyond
  quoting (`check_id_quoting`, `mod.rs:267-276`), so legacy and draft IDs both
  pass. Typed-link shape `[A-Za-z0-9.-]+` admits `work-item:draft-k7mq3x`.
  Existence (`dangling_refs`, `mod.rs:485-521`) is exact-string against an
  index of each file's own `type:id`; aliases are not indexed.
- **`aliases: []`** trips `check_empty_placeholders` (`mod.rs:388-400`, only
  `tags` exempt), failing `this_repositorys_own_corpus_is_clean`
  (`cli/corpus-cli/tests/frontmatter_goldens.rs:390-404`). Adding `aliases`
  to `templates/work-item.md` also requires `KNOWN_FRONTMATTER_KEYS`
  (`cli/work/src/create.rs:50-71`) or `work create`'s drift guard
  (`assert_matches_template_schema`, `:222-251`) fails. `work update`
  list mutation needs it in `LIST_FIELDS` (`cli/work/src/update.rs:14-15`).
- `work_item_id` on reviews is only quote-checked, not reference-checked
  (`schema.rs:311`).

### Reusable machinery for ID retirement

- **Migration `m0002`** (`cli/migrate/src/migrations/m0002.rs:58-65`) is the
  direct precedent: collect renames → collision check before any write
  (`:112-140`) → rewrite own id then `merge_move` (`:142-156`) → corpus
  passes via `rewrite_each` (`:158-200`, pure `&str -> String` per file,
  write on change). It is non-transactional, deliberately skips bare prose
  IDs, and writes legacy bare values rather than `work-item:` refs.
- Whole-token boundary matching: hand-rolled in `m0002.rs:500-526`
  (`is_ref_boundary_char`); Rust `regex` lacks lookaround, so this is the
  local idiom for hyphenated IDs. `migrate/src/migrations/text.rs:9-31` has
  line helpers.
- H1 and title rewrite: `m0010.rs:128-268`.
- Frontmatter editing: byte-preserving `patch_status`
  (`cli/corpus-adapters/src/patcher.rs:48-94`, hard-coded to `status:`;
  parameterising the key gives a format-preserving `id:` patch), and
  structural parse → `mutate_list` → `document::render`
  (`cli/work-cli/src/update.rs:309-369`, `cli/work/src/update.rs:53-80`).
- Typed refs: `parse_typed_ref` (`cli/corpus/src/typed_ref.rs:30-70`).
- Atomic single-file writes: `store::atomic_write`
  (`cli/store/src/lib.rs:92-104`); `stage`/`persist` (`:226-274`) are private.
  **No multi-file transaction or rollback exists**; existing
  "all-or-nothing" is validate-then-write (`sync.rs:658-685`,
  `run.rs:75-82`) or VCS revert (migrate preamble,
  `migrate-cli/src/render.rs:14`).
- Random IDs: no base32/Crockford code; `rand = "0.9"`
  (`cli/Cargo.toml:107`). Redraw-until-unique template:
  `cli/jira-client/src/multipart.rs:66-80`.
- Parents-first ordering and cycle detection: private to
  `cli/work-cli/src/list.rs` (`cyclic_members` `:374-403`, `child_index`
  `:405-429`, `render_hierarchy` `:316-372`). No topological sort exists; lift
  into the `work` crate for extract-work-items (and 0291).

### Skills

- **`create-work-item`** (`skills/work/create-work-item/SKILL.md`): calls
  `work next-number` in step 5.1 (`:420-435`) though `work create` mints the
  real id; push preview `work create --push --dry-run` (`:502-525`); push gate
  options "Yes, push to [tracker] now" / "**No, save locally only**"
  (`:529-535`); outcome table (`:563-567`). The decline text becomes "No, save
  as draft" under `{tracker}`.
- **`extract-work-items`**: no integration injection and **no push offer**;
  allocates with `work next-number --project <code> --count <n>`
  (`:433-441`); writes with the Write tool, not `work create` (`:503-507`);
  `parent` only when the source names an existing item — **no intra-batch
  parent linking** (`:480-482`); summary table `| ID | Title | File |`
  (`:516-528`). 0230's batch requirements are new behaviour, not an edit.
- **`refine-work-item`** also allocates (`work next-number --count N`,
  `:177-178`) and writes children with Write (`:180-189`). 0230 does not
  mention it.
- **`sync-work-items`**: documents `pulled-untracked: remote key → new local
  id` (`:166-180,348-349`); step 5 summary (`:340-355`) will need promotion
  and key-change groups.
- **`list-work-items`**: states "filename is authoritative" for the ID
  (`:195-198`).
- Evals: `create-work-item/evals/evals.json` asserts `next-number` call counts
  (`:50,57,284-291,375-380`); `refine-work-item` eval 22 asserts
  `next-number --count 3`. No evals for sync or list; none cover push.
- Python conformance tests parse SKILL prose:
  `tests/unit/tasks/test_skill_frontmatter_population.py`,
  `tests/integration/conformance/test_conformance.py`.

## Code References

- `cli/corpus-adapters/src/work_item_pattern.rs:218-297` — pattern `compile()`
  and its rules
- `cli/corpus/src/work_item_id.rs:19-260` — `WorkItemIdScheme` and
  regex-free ID predicates
- `cli/work-cli/src/config.rs:52-79` — `resolve_scheme`, the shared config
  seam
- `cli/work-cli/src/create.rs:470-712` — `execute_push` and `try_run`
- `cli/work/src/sync/push_decide.rs:8-65` — outcome decision table
- `cli/work/src/sync/push_precondition.rs:11-95` — marker precondition
- `cli/work-adapters/src/sync/pending_push.rs:32-221` — marker I/O
- `cli/jira-client/src/classify.rs:47-62`,
  `cli/linear-client/src/classify.rs:127-159` — create error classes
- `cli/work-cli/src/sync.rs:101-137,1053-1386` — sync discovery and entry
- `cli/work-adapters/src/sync/run.rs:546-1108` — sync engine
- `cli/work-adapters/src/sync/baseline.rs:18-46` — id-keyed baseline
- `cli/work-cli/src/sync_author.rs:98-160` — pull-side local ID allocation
- `cli/tracker/src/lib.rs:107-132,152-199,574-771` — `RemoteIssue`, error
  classes, trait
- `cli/jira-client/src/client.rs:631-708`,
  `cli/linear-client/src/client.rs:810-879` — `fetch_all` matching
- `cli/work/src/resolve.rs:114-338` — resolve classification and candidates
- `cli/work-cli/src/list.rs:109-147,316-456,652-675` — list scan, hierarchy,
  Sync column
- `cli/corpus/src/frontmatter_validation/mod.rs:204-521` — per-file checks
- `cli/corpus/src/frontmatter_validation/schema.rs:20-43` — work-item schema
- `cli/work/src/create.rs:50-71,222-251` — template key drift guard
- `cli/migrate/src/migrations/m0002.rs:58-572` — rename-and-rewrite precedent
- `cli/store/src/lib.rs:92-274` — atomic write and private staging
- `cli/visualiser/server/src/file_driver.rs:242-322` — flat visualiser scan

## Architecture Insights

- **Hexagonal split holds the shape of the change.** Domain decisions
  (`push_decide`, `push_precondition`, `next_number::allocate`, `resolve`
  classification, sync `plan`/`classify`/`decide`) are pure functions in
  `cli/work` and `cli/corpus`; adapters and binaries in `work-adapters`,
  `work-cli`, `corpus-adapters`. Draft-ID minting, ID retirement planning,
  and alias/`external_id` resolution belong in the domain crates with
  injectable draw sources and listers; `work` may not depend on
  `corpus_adapters` (`cli/work/src/next_number.rs:1-9`).
- **No shared work-item repository.** Five readers each `read_dir` the work
  dir with their own key (filename vs `id` vs `external_id`). Introducing
  `drafts/` is the forcing function for a single corpus-discovery port; without
  one, each reader needs its own recursion change.
- **Identity is joined on `external_id`** (ADR-0044, 0228 plan). Sync state
  keys on local `id`, so any `id` change must carry the baseline entry and
  sync-side markers with it.
- **Error classification is conservative by design**: reads are never
  terminal, mutations whose effect is unknown are. 0230's outcome vocabulary
  must be mapped onto this, not the reverse.
- **All-or-nothing has two local idioms** — plan-and-check before writing, or
  VCS revert — and neither covers a multi-file operation run from a live
  command. ID retirement needs a staged-write/snapshot-restore unit, which
  0302's batch re-key also needs.

## Historical Context

- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  accepted; `id` always allocated locally, shape `(<project-code>-)?\d{4}`
  (`:80-81`); option "reuse `id` as remote key" explicitly rejected
  (`:69-70,99-100`). 0230 needs a superseding ADR.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — canonical
  `"doc-type:id"`; accepted ADRs immutable (`:27,68`), which collides with
  rewriting typed links inside ADR frontmatter; path-form refs are also
  accepted and would go stale on rename (`:85`).
- `meta/decisions/ADR-0068-general-slug-resolution-in-the-corpus-cli.md` —
  `corpus resolve` reuses `work resolve` classification.
- `meta/plans/2026-09-10-0228-layered-configuration-key-model.md` (done) —
  scope key owns creation home; per-origin prefixes deferred to 0230
  (`:103-105`); two pattern implementations (`:58-64`); `{project}` becomes an
  `UnknownToken` in 1.25.0.
- `meta/reviews/work/0230-tracker-owned-work-item-id-generation-review-1.md` —
  APPROVE after four passes; last three majors fixed but not re-reviewed;
  open minors include orphaned `created` marker adoption, literal text around
  `{tracker}`, and the undefined "sync baseline".
- `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-linear.md`
  — high-priority draft bug: `execute_push` records no baseline, so every
  `{tracker}` create and promotion would show as a conflict on the next sync.
  No dependency edge to 0230.
- `meta/work/0302-work-item-re-key-command.md` — expects batch-callable ID
  retirement, bidirectional `meta/work/` ↔ `drafts/` moves, and standalone
  key-change detection with a distinct "tracker unreachable" result.
- `meta/work/0291-parent-child-relationship-sync.md` — defers edges to
  `draft-` parents; shares parents-first ordering and cycle detection.
- `meta/work/0297-fully-qualified-artifact-references-in-prose.md` — relies
  on the `work-item:<id>` matcher running over the whole file, not just
  frontmatter.
- `meta/work/0227-accelerator-config-validate-command.md` — draft, blocked by
  0292; not shipped.

## Related Research

- `meta/research/codebase/2026-04-28-configurable-work-item-id-pattern.md`
- `meta/research/codebase/2026-09-10-0228-layered-configuration-key-model.md`
- `meta/research/codebase/2026-09-11-0229-per-tracker-pull-scope-configuration.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-08-19-0212-work-item-script-cutover.md`
- `meta/research/codebase/2026-09-08-0285-targeted-pull-of-remote-only-work-items.md`
- `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md`

## Open Questions

1. **Unreachable-tracker classification.** On create, a connect/DNS/timeout
   failure is `Terminal` → `loud-terminal`, so 0230's
   `draft — tracker unreachable` (`local-save`) AC is unreachable for the
   commonest "unreachable" case. Either reclassify pre-send transport
   failures as `Retryable` (provably no mutation) or re-word the ACs.
2. **ADR-0044 supersession.** Write a new ADR before planning; 0230 is the
   option ADR-0044 rejected.
3. **Accepted-ADR immutability vs retirement rewrites.** Rewrite typed links
   in accepted ADRs' frontmatter, or exempt `meta/decisions/`?
4. **Baseline and sync-marker re-keying.** ID retirement must move the
   `last-sync.json` entry (and any id-named sync marker) inside the same
   all-or-nothing unit.
5. **Sequencing against 0296.** Land 0296's baseline-on-push fix first, or
   absorb it into 0230.
6. **Key-change detection design.** Requires adding the returned key to
   `RemoteIssue`/`FetchOutcome`, a per-key read path for Linear (or
   `previousIdentifiers`), a distinct not-found outcome, and running before
   untracked discovery to avoid double import.
7. **`aliases` shape.** Omit when empty or exempt it from EMPTY-PLACEHOLDER;
   plain IDs or typed refs; indexed for reference existence or not.
8. **`work list` Sync column rule.** Code shows the column whenever an
   integration is configured; the skill and 0230's AC assume baseline-gated.
9. **`refine-work-item` decomposition** allocates with `next-number` and is not
   covered by 0230.
10. **Marker digest under `{tracker}`.** The digest includes the id-substituted
    body; decide the body form fingerprinted before the key is known, and how
    promotion adopts a `created` marker despite a digest mismatch.
11. **Shared corpus discovery.** Add a work-item repository port that includes
    `drafts/`, or patch each flat reader individually.
