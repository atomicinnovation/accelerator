---
type: "plan-review"
id: "2026-09-26-0230-tracker-owned-work-item-id-generation-review-1"
title: "Plan Review: Tracker-Owned Work Item ID Generation Implementation Plan"
date: "2026-09-26T17:00:27+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-26-0230-tracker-owned-work-item-id-generation"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "compatibility", "usability", "standards"]
review_number: 1
review_pass: 5
tags: ["sync", "tracker", "id-generation", "drafts", "promotion"]
last_updated: "2026-09-27T13:00:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Tracker-Owned Work Item ID Generation Implementation Plan

**Verdict:** REVISE

The plan is well sequenced and keeps the hexagonal split. Decisions are pure functions in `work`/`corpus` and effects sit behind ports. Every phase is test-first, with behaviour-named tests, and ID retirement is one reusable plan-then-apply unit. The weaknesses show up where the parts meet during a live sync run. Retirement applied mid-run leaves the engine's precomputed plan stale (critical). Several paths reintroduce the duplicate-remote-issue and double-binding hazards that the `pending_push` design exists to prevent. Retirement is all-or-nothing only against in-process errors: it has no lock, no crash recovery, and no respect for dirty files. Machine-readable output also drifts across `sync`, `create` and `create-batch`, and there are notable test gaps around rollback, pure decisions and pattern consumers.

### Cross-Cutting Themes

- **Retirement is not durable or serialised** (flagged by: architecture, safety, correctness)
  - Rollback lives in memory only, so a killed process leaves both files behind and later promotions refuse forever.
  - No lock guards retirement or promotion.
  - Retirement rewrites dirty working-copy files, which the sync engine deliberately never does.
  - Applied mid-run in Phase 4, it invalidates the engine's plan, index and digests.
- **Duplicate-remote-issue and double-binding hazards** (flagged by: safety, compatibility, correctness)
  - Draft-ID-named markers lose the slug marker's rerun idempotency.
  - An interrupted create orphans an `attempted` marker that no draft owns.
  - `NotSent` after an earlier 5xx attempt deletes the guard.
  - Pull-side marker cleanup removes the `created` marker promotion relies on.
  - `is_taken` ignores `external_id`.
- **Deterministic pre-send failures classified as retryable** (flagged by: code-quality, compatibility, correctness, test-coverage) — `BadPath`, serialisation and ADF conversion become "tracker unreachable" / `local-save`, which hides real bugs and makes sync retry forever.
- **Inconsistent machine-readable output** (flagged by: usability, standards, code-quality) — promotion rows break sync's `<id>\t<action>\t<state>\t<detail>` shape, and `not-found` is free prose. `create-batch` prints prose with em-dashes, and `work create` signals "write failed" only by whether the key column is empty.
- **Orchestration duplicated and outside the engine** (flagged by: architecture, code-quality, test-coverage)
  - Promotion runs in `work-cli`'s `run_sync`, ahead of the engine's push budget, pre-flight and preview.
  - The create → re-key → baseline → marker sequence is hand-built twice (`try_run` and `promotion.rs`).
  - As a result, sync-level promotion tests land in a crate that cannot reach the code.
- **Interim phases ship a half-supported `{tracker}`** (flagged by: compatibility, usability, standards) — Phase 5 documents `{tracker}`, but the skills keep `next-number` + Write until Phase 8. Three skills branch on `{tracker}` without injecting `work.id_pattern`.
- **Tracker key grammar is too narrow** (flagged by: compatibility, correctness) — `[A-Za-z0-9]+-[0-9]+` rejects Jira `MY_PROJ-12` and mis-scans `0296-2026-review.md`.

### Tradeoff Analysis

- **Durability vs simplicity**: an intent journal or resumable retirement adds a state file and a recovery path. Its cost is small compared with the unrecoverable partial `meta/` rewrite it prevents. The recommendation is resumable retirement: recognise a completed move where `to` exists and its `aliases` contain the old ID, which needs no journal.
- **Lock scope: safety vs availability**. Safety wants promotion and retirement under a lock. Architecture wants the create lock released during network calls. These are compatible: hold a dedicated promotion lock across the draft lifecycle, and take the create lock only around minting and local writes.
- **Rewrite dirty files vs automatic promotion**: refusing to retire when touched files are dirty keeps the engine's invariant, but promotion then stalls in an active working copy. Recommendation: stop before the remote create and report the dirty paths, with `--no-promote` as the escape hatch.

### Findings

#### Critical

- 🔴 **Correctness**: Retirement mid-run leaves the precomputed plan, index and item snapshots stale
  **Location**: Phase 4: run.rs changes
  `RetireKey` is applied in `run` after `prepare_run` has built the plan, the index and the `LazyItemDigests`. Planned pulls and pushes then target moved paths or retired IDs, and can overwrite freshly rewritten references.

#### Major

- 🟡 **Architecture / Safety / Correctness**: Retirement has no crash recovery
  **Location**: Phase 3: Transactional applier
  A process killed between writing `to` and removing `from` leaves both files. The next promotion then hits `TargetExists`/`IdTaken`, and `meta/` stays half-rewritten.
- 🟡 **Architecture / Safety**: No lock serialises promotion or retirement
  **Location**: Phase 3 applier; Phase 7 promotion service
  A concurrent `work sync` and `work promote` both see an Absent marker, so both create remote issues and race the retirement.
- 🟡 **Safety**: Retirement rewrites files with uncommitted changes
  **Location**: Phase 3; Phase 7 sync integration
  This breaks the engine's never-overwrite-dirty invariant (`work/src/sync/decide.rs:12-15`) on the default `work sync` path.
- 🟡 **Safety / Compatibility**: Draft-ID-named markers lose rerun idempotency and orphan `attempted` markers
  **Location**: Phase 6: `{tracker}` create flow
  Each invocation mints a new draft ID. Rerunning after a kill or after two failed writes therefore creates a second issue, and an `attempted` marker has no draft to stop promotion.
- 🟡 **Safety**: `NotSent` can hide a create already sent on an earlier 5xx attempt
  **Location**: Phase 1: Transport error shape
  A 503 followed by a connect failure in the retry loop becomes `NotSent`, so the marker is removed while the issue may exist.
- 🟡 **Correctness**: Pull marker cleanup defeats the `created` marker kept after a failed promotion
  **Location**: Phase 5 §5; Phase 7 §2
  Untracked discovery imports the key in the same run, and Phase 5 cleanup deletes the marker. The next sync then creates a duplicate.
- 🟡 **Correctness / Safety**: Collision checks ignore other items' `external_id`
  **Location**: Phase 2 `is_taken`; Phase 4; Phase 5; Phase 7 step 2
  `--adopt PP-760`, a key change or pull adoption can bind a second local item to an issue that legacy `0230` already links.
- 🟡 **Correctness**: Moved keys are never detected under per-item reads
  **Location**: Phase 4 §4 `gather`
  `show(old)` returns the issue under its new key and is recorded as `Present`, so `locate` is never reached.
- 🟡 **Safety**: Promotion overwrites an adopted issue's description
  **Location**: Phase 7 §2 step 5
  The H1 update on `--adopt`/`AdoptRecordedKey` replaces remote edits wholesale, with no read-back or conflict check.
- 🟡 **Code Quality / Correctness / Compatibility**: Deterministic pre-send failures are classified as retryable
  **Location**: Phase 1 §2
  `BadPath`, serialisation and ADF failures become `local-save` / "tracker unreachable", which silently changes exit 71 to 0 and makes sync retry forever.
- 🟡 **Architecture / Code Quality**: Promotion runs outside the sync engine, inline in an oversized `run_sync`
  **Location**: Phase 7 §3
  This splits `--max-pushes`, pre-flight refusal and preview authority, so drafts can be pushed before an engine refusal.
- 🟡 **Architecture / Code Quality**: The tracker-keyed creation sequence is orchestrated twice, and `try_run` grows into a god function
  **Location**: Phase 6 §2, §4; Phase 7 §2; Phase 8
  Only `create_remote_issue` is shared. Marker, H1, write, baseline and cleanup are sequenced separately in `work-cli` and `work-adapters`.
- 🟡 **Code Quality**: Two overlapping decision tables over the same marker state
  **Location**: Phase 6 `MarkerPolicy`; Phase 7 `first_step`
  The adopt-on-`created` decision is made in two places that can drift.
- 🟡 **Code Quality / Architecture**: New file ports duplicate `corpus::scan`/`corpus::store`, and where corpus enumeration lives is left undecided
  **Location**: Phase 2 §3; Phase 3 §3–4
  `WorkItemFiles` and `RetirementFiles` add parallel port families with different error types. `corpus_files` placement is left conditional.
- 🟡 **Usability / Standards**: Promotion, key-change and not-found output breaks sync's TSV record shape
  **Location**: Phase 7 §3; Phase 4 §4
  `promoted\t<draft>\t<key>` is keyword-first with three fields, and `not-found` is prose in the same stream.
- 🟡 **Usability / Standards / Code Quality**: One create outcome is reported in three vocabularies
  **Location**: Phase 6 `main.rs`; Phase 8 `create-batch`
  `loud-terminal\tPP-900` relies on the key column being empty or not. `create-batch` prints `created PP-904 — local write failed` prose, and line 1 is a path that does not exist.
- 🟡 **Usability**: No message or recovery guidance for `RestoreIncomplete`
  **Location**: Phase 3 §3
  The worst reachable state does not list the unrestored paths or give a remedy.
- 🟡 **Compatibility**: Skills and binary disagree under `{tracker}` between Phases 5 and 8
  **Location**: Implementation Approach; Phase 5 docs; Phase 8 skills
  `extract-work-items` and `refine-work-item` write `draft-` files into the canonical directory, and `create-work-item` hits the interim refusal.
- 🟡 **Compatibility**: Tracker key grammar excludes valid Jira keys
  **Location**: Phase 5 scan regex; Phase 3 `Distinctive`
  `MY_PROJ-12` is classified `NumericOnly`, so its prose references are left stale.
- 🟡 **Standards**: Skills branch on `{tracker}` without injecting `work.id_pattern`
  **Location**: Phase 7 §4; Phase 8 §4
  `extract-work-items`, `refine-work-item` and `sync-work-items` cannot see the pattern they branch on.
- 🟡 **Test Coverage**: Sync-level promotion tests are placed in a crate that cannot reach the promotion loop
  **Location**: Phase 7 §1 (`work-adapters/tests/sync_create.rs`)
  `--no-promote`, continue-past-failure and max-pushes live in `work-cli`'s `run_sync`.
- 🟡 **Test Coverage**: Rollback fault injection cannot reach the baseline rename or the file removal
  **Location**: Phase 3 §1
  A fake that fails the nth write misses `remove` and the separate `BaselineStore` ports, and the plan defers these to a manual check.
- 🟡 **Test Coverage**: Pure decisions have no named unit tests
  **Location**: Phase 4 `key_change`; Phase 7 `promotion`; Phase 6 `push_precondition`
  None of the nine `first_step` cells, the `decide_key_change` branches or the `MarkerPolicy` cases is named.
- 🟡 **Test Coverage**: Pattern-consumer changes for `{tracker}` are untested
  **Location**: Phase 5 §2
  `compile()` embedded-token error, the scan regex, `id_is_token_safe`, and the other `work_item_pattern` entry points.
- 🟡 **Test Coverage**: No round-trip test from written `aliases` to resolution
  **Location**: Phase 2 `identities`; Phase 3
  The hand-rolled `identities` parser has no tests of its own, and nothing retires an ID and then resolves the old one.
- 🟡 **Test Coverage**: `create-batch` manifest validation, exit precedence and the non-`{tracker}` path are untested
  **Location**: Phase 8 §1

#### Minor

- 🔵 **Architecture / Safety**: The create lock is held across up to three remote calls
  **Location**: Phase 6 §4; Performance Considerations
- 🔵 **Architecture**: Key moves change the port's `absent` and `show` semantics without the docs or the contract suite changing
  **Location**: Phase 4 §2
- 🔵 **Architecture**: `validate_id_pattern` hard-codes integration names, and the sole-token rule is duplicated in `corpus-adapters`
  **Location**: Phase 5 §2
- 🔵 **Architecture**: Corpus-wide reference rewriting sits in the work-item domain
  **Location**: Phase 3 §2
- 🔵 **Code Quality**: Readers share file enumeration but keep separate frontmatter parsing, with divergent quote and fallback rules
  **Location**: Phase 2 §4
- 🔵 **Code Quality**: No injection seam is specified for the fault-injecting writer in `try_run`
  **Location**: Phase 6 §1
- 🔵 **Code Quality**: New public types use stringly-typed and tuple fields (`retired_item`, `baseline_rename`, `cause: String`)
  **Location**: Phase 3 §2–3
- 🔵 **Code Quality / Correctness**: Promotion checks `is_taken` before the created key exists, and has no rule for a failed H1 update
  **Location**: Phase 7 §2
- 🔵 **Correctness**: File-path references to the retired item are left dangling
  **Location**: Phase 3 §2 token boundary
- 🔵 **Correctness**: Skipping only the `aliases:` line lets block-style alias entries be rewritten
  **Location**: Phase 3 §2
- 🔵 **Correctness / Usability**: The `create-batch` exit rule contradicts itself, and its manifest errors are unspecified
  **Location**: Phase 8 §3
- 🔵 **Safety**: Snapshot-then-write can lose concurrent edits, and `to` is not written with `exclusive_write`
  **Location**: Phase 3 §3
- 🔵 **Safety**: Recovering an orphaned `created` marker depends on discovery scope and loses local metadata
  **Location**: Phase 6 §4; Migration Notes
- 🔵 **Compatibility**: Key-change following relies on tracker behaviour for old keys that has not been verified
  **Location**: Phase 4 §2–3
- 🔵 **Compatibility**: Extending the port breaks nine `RemoteTracker` implementors, and the plan lists only three
  **Location**: Phase 4 §2
- 🔵 **Usability**: Promoting an already-promoted draft ID returns an unhelpful "not a draft" error
  **Location**: Phase 7 §3
- 🔵 **Usability / Standards**: The interim `--push` refusal has no specified message, and `E_TRACKER_PUSH_PENDING` is easily confused with `E_PUSH_PENDING`
  **Location**: Phase 5 §4
- 🔵 **Usability**: `NotPromoted` reason keywords and per-reason recovery hints are undefined
  **Location**: Phase 7 §2
- 🔵 **Standards**: One concept goes by several names across the key-change vocabulary (Missing/NotFound/losing; Moved/FollowKey/key-changed)
  **Location**: Phase 4 §2, §4
- 🔵 **Standards**: The `draft` Sync label skips the glyph-and-text label convention in `work::sync::label`
  **Location**: Phase 2 §6
- 🔵 **Standards**: "alias" and "draft" already mean other things (`own_identity_alias`, `status: draft`)
  **Location**: Phase 2 §2; Phase 5 §3
- 🔵 **Standards**: New `corpus` modules share a name with the `corpus` crate
  **Location**: Phase 2 §3
- 🔵 **Standards**: The new read adapter sits outside the pup-guarded `work_adapters::filesystem` module
  **Location**: Phase 2 §3; Phase 3 §3
- 🔵 **Standards**: New exit semantics bypass `exit_codes.rs` (a manifest cycle exits 1, which means `ERROR`)
  **Location**: Phase 7 §3; Phase 8 §3
- 🔵 **Test Coverage**: Promotion preview, the `PullOnly` skip, id ordering and `E_PROMOTE_NOT_A_DRAFT` are untested
  **Location**: Phase 7 §3
- 🔵 **Test Coverage**: `a_pull_consumes_no_local_number` has an assertion that always passes
  **Location**: Phase 5 §1
- 🔵 **Test Coverage**: `locate` error paths, Linear's `null` issue and `--max-pulls` accounting are untested
  **Location**: Phase 4 §4
- 🔵 **Test Coverage**: The traceability table is grouped by phase and claims coverage for untestable ACs (`config validate`)
  **Location**: Testing Strategy
- 🔵 **Test Coverage**: The pre-send `NotSent` mappings and the update path are uncovered, and the refused-port test is prone to port reuse
  **Location**: Phase 1

#### Suggestions

- 🔵 **Architecture**: Route `FilesystemLister` through `WorkItemFiles` with an explicit canonical-only filter
  **Location**: Phase 2 §3–4
- 🔵 **Code Quality / Standards**: `hierarchy::Ordering` shadows `std::cmp::Ordering`, and `PromotionMode::Default` is not domain language
  **Location**: Phase 8 §2; Phase 7 §2
- 🔵 **Usability**: `next-number` returning unreserved draft IDs is surprising for custom scripts
  **Location**: Phase 5 §4
- 🔵 **Test Coverage**: Pin the draft-mint attempt limit exactly (15 collisions succeed, 16 exhaust)
  **Location**: Phase 5 §1
- 🔵 **Compatibility**: State the minimum plugin version needed for a shared `{tracker}` config
  **Location**: Phase 5 docs
- 🔵 **Standards**: Name the fixture `issue-not-found.golden.json`
  **Location**: Phase 4 §3
- 🔵 **Standards**: Add black-box `cli_create_batch.rs` and `cli_promote.rs` tests for tracker-free paths
  **Location**: Phase 7 §1; Phase 8 §1

### Strengths

- ✅ Decisions are pure and live in `work`/`corpus` (`resolve_identity`, `plan_retirement`, `decide_key_change`, `first_step`, `mint_draft_id`, `parents_first`), with effects behind narrow ports. This preserves the functional-core / imperative-shell split and the pup rules.
- ✅ ID retirement is one reusable plan-then-apply unit that promotion, key-change following and the future 0295 re-key all compose. Refusals happen at plan time, before any write.
- ✅ Phases build pattern-independent identity foundations before anything configures `{tracker}`. Each phase is independently mergeable and revertable.
- ✅ The not-sent fix lives at the transport layer, where the knowledge is, and the "sent then failed is Terminal" rule is pinned as a regression.
- ✅ Draft minting is deterministic through `SuffixDraws`. The Crockford class is exact, and rejecting all-digit suffixes keeps `parse` and `mint` consistent.
- ✅ Token-boundary rewriting has precise edge-case tests (`PP-76` vs `PP-760`, hyphen/letter/digit adjacency, case folding, numeric-only IDs).
- ✅ A `Missing` remote never deletes or retires local data. Automatic changes are bounded by `--max-pulls` / `--max-pushes`, and `--preview` / `--no-promote` give control.
- ✅ Domain naming (`DraftId`, `IdOwnership`, `KeyChange`, `NotPromoted`) mirrors the work item. Test names are behaviour sentences, and the sketches carry no comments.
- ✅ The superseding ADR is recorded before any code lands, and contract pins (`public-api`, `cli_surface.golden`, pup) are updated deliberately in each phase.
- ✅ Configuration errors carry exact, rule-naming messages from one validator shared with the future `config validate`.

### Recommended Changes

1. **Move key-change application out of the engine's apply loop** (addresses: retirement mid-run leaves snapshots stale)
   Apply `FollowExternalId`/`RetireKey` as a separate pre-pass: gather → detect → apply → re-run `discover_items` → reload the baseline → normal plan/apply. Add a test in which a moved item and a referencing item both have planned actions.
2. **Make retirement resumable, locked and dirty-aware** (addresses: no crash recovery; no lock; rewrites dirty files; snapshot-then-write races)
   - Recognise a completed move (`to` exists and its `aliases` contain the old ID) and finish the remaining steps instead of refusing.
   - Hold a promotion/retirement lock from reading the marker until the marker is removed.
   - Stop before the remote create when a touched file is Dirty/Unknown, and report the paths.
   - Write `to` with `exclusive_write`.
   - Add a kill-after-`to` test and a concurrency test.
3. **Close every duplicate-issue and double-binding path** (addresses: draft-ID markers; `NotSent` after 5xx; pull cleanup; `external_id` collisions; adopt overwrite)
   - Write the draft before sending a `{tracker}` create, so every `attempted` marker has an owner. Alternatively, look up outstanding markers by digest before minting.
   - Return `NotSent` only when no earlier attempt got a response.
   - Exclude keys held by a draft's `created` marker from untracked discovery and from pull cleanup.
   - Add an `is_linked` check over `external_id` for adopt, promotion, pull adoption and both key-change actions.
   - On adopt, read back the remote and skip the H1 update if it has been edited.
4. **Narrow `NotSent` to connect failures** (addresses: deterministic pre-send failures)
   Map `BadPath`, serialisation and ADF failures to a provably-unapplied Terminal class that reports its real cause.
5. **Detect moved keys wherever `show` is called** (addresses: per-item reads)
   Compare `issue.key` with the requested key in every `gather` branch. Add a `PerItem` + `.moving()` test.
6. **Unify orchestration** (addresses: promotion outside engine; duplicated create sequence; `try_run` god function; overlapping decision tables; test placement)
   - Extract one tracker-keyed creation service in `work-adapters`. Its only variation is local placement (write at the key path vs retire the draft).
   - Make `first_step` the sole marker authority and drop `MarkerPolicy`.
   - Run promotion inside the engine, sharing the push budget, pre-flight checks and preview.
   - Move the sync-level promotion tests to wherever the promotion loop ends up.
7. **Define one output contract** (addresses: TSV shape; three vocabularies; key-change/not-found; `NotPromoted` keywords; `RestoreIncomplete` message; exit taxonomy)
   - Promotion rows become `<draft-id>\tpromoted\t-\t<key>` and key-change rows `<id>\tkey-changed\t<state>\t<old>→<new>`; `not-found` becomes a record.
   - `create-batch` prints `<ref>\t<path>\t<keyword>\t<key>` using the `PushOutcome` keywords plus `declined` and `created-unwritten`; the skill renders the prose.
   - Pin the `NotPromoted` reason keywords, the `RestoreIncomplete` rendering and the exit codes in `exit_codes.rs`.
8. **Fix interim-phase skew and skill injection** (addresses: skills/binary skew; missing `id_pattern` injection; interim refusal)
   - Defer the `{tracker}` user docs to Phase 8, or make `next-number` refuse under `{tracker}` until then.
   - Rename the interim refusal to `E_TRACKER_PUSH_UNSUPPORTED`, with a remedy message.
   - Inject `work id_pattern` into `extract-work-items`, `refine-work-item` and `sync-work-items`.
9. **Define one tracker-key grammar** (addresses: Jira underscore keys; digit-prefixed scan)
   Use `[A-Za-z][A-Za-z0-9_]*-[0-9]+` for the scan regex, `is_project_prefixed` under `{tracker}`, and `id_is_token_safe`.
10. **Fill the named test gaps** (addresses: rollback reach; pure decisions; pattern consumers; alias round trip; `create-batch`; preview/`PullOnly`; vacuous pull test; `locate` paths; traceability)
    - Parameterise the rollback test over every operation index.
    - Add table tests for `first_step` and `decide_key_change`.
    - Add scan-regex and dossier tests.
    - Add an `identities` parser suite and a retire-then-resolve test.
    - Replace the traceability table with one row per AC.
11. **Tidy domain language and structure** (addresses: naming minors)
    - Settle on one term each for Missing and Moved.
    - Rename `IdentityField::Alias` to `RetiredId`.
    - Rename `hierarchy::Ordering` to `BatchOrder`.
    - Rename the `work::corpus` module to something that does not shadow the crate.
    - Put the read adapter under `work_adapters::filesystem`.
    - Make `draft` a glyph-prefixed `RenderableState`.
    - Rewrite path references during retirement.
    - Skip block-style `aliases` spans.

## Per-Lens Results

### Architecture

**Summary**: The plan fits the hexagonal split well: new decisions are pure modules in `work`/`corpus`, new I/O sits behind ports in `work-adapters`, and phases are independently mergeable. There are two structural weaknesses. Promotion is orchestrated in `work-cli` outside the sync engine that owns write bounds, preview and pre-flight refusal. The tracker-keyed creation sequence is assembled twice. The main resilience gap is ID retirement: its rollback is in-process only, it has no crash recovery, and it does not coordinate with the existing locks.

**Strengths**:
- Decisions stay pure; effects sit behind narrow ports (`WorkItemFiles`, `RetirementFiles`, `SuffixDraws`, `locate`).
- The not-sent classification is fixed at the transport layer, consistent with the `TrackerError::Retryable` contract.
- ID retirement is one reusable plan-then-apply unit shared by promotion, key-following and 0295.
- The phase ordering builds pattern-independent foundations first.
- A superseding ADR is recorded before any code, with its tradeoffs explicit.
- `IdOwnership` is introduced in Phase 4, ahead of the Phase 5 derivation.

**Findings**:
- 🟡 major / high — **Promotion runs outside the sync engine, splitting write-bound and preview authority** (Phase 7: Sync integration). The engine owns `--max-pushes` refusal, pre-flight aborts and preview. Promotion in `run_sync` creates remote issues before the engine plans, so "refused before any write" no longer holds under `{tracker}`. Make promotion a phase of the engine run, sharing its push budget and pre-flight checks.
- 🟡 major / medium — **The tracker-keyed creation sequence is orchestrated in two places** (Phases 6, 7, 8). Only `create_remote_issue` is extracted. The marker, H1 update, placement, baseline and cleanup steps are sequenced in both `try_run` and `promotion.rs`. Extract one service with local placement as the only injected variation.
- 🟡 major / medium — **Multi-file retirement has no crash recovery or lock coordination** (Phase 3: Transactional applier). Snapshots live in memory only, and the create lock and per-file update lockdirs are not acquired. Write an intent record, detect and roll it forward or back on the next run, and hold the create lock.
- 🔵 minor / high — **Where corpus enumeration lives is left open** (Phase 3 §4; Phase 4). `SyncPorts` has no corpus-files or `RetirementFiles` port. Decide in Phase 3: a `CorpusFiles` port implemented in `work-cli` over the `corpus-adapters` walk.
- 🔵 minor / medium — **Key moves change the meaning of `FetchOutcome.absent` and `show`** (Phase 4 §2). Restate `absent` as "not found under the requested key", document `locate` as the authority, and extend the contract suite.
- 🔵 minor / medium — **`{tracker}` rules are split across `corpus` and `corpus-adapters`, and `corpus` names integrations** (Phase 5 §2). Pass a capability flag instead of an integration name, and have `compile()` reuse the sole-token rule.
- 🔵 minor / low — **Corpus-wide reference rewriting sits in the work-item domain** (Phase 3 §2). Put the rewrite function in `corpus` and compose it from `work::retirement`.
- 🔵 minor / medium — **The create lock is held across up to three remote calls** (Phase 6 §4). Hold it only for minting and for the local write.
- 🔵 suggestion / medium — **Several enumeration paths remain, with different draft visibility** (Phase 2 §3–4). Route `FilesystemLister` through `WorkItemFiles` with an explicit canonical-only filter.

### Correctness

**Summary**: The domain is decomposed into sound, testable parts, and fine details such as the Crockford class, the letter-required rule and token boundaries are right. The weak points are where the parts interact inside a sync run. Phase 4 retires IDs after the plan, index and item snapshots are computed. Phase 5's pull-side marker cleanup undoes Phase 7's `created`-marker guarantee. Per-item reads never see a moved key. Collision checks ignore `external_id`. The retirement transaction is not all-or-nothing against interruption.

**Strengths**:
- The Crockford class `[0-9a-hjkmnp-tv-z]` is exact, and the all-digit rejection is consistent between parse and mint.
- `BaselineStore` re-reads before every mutation, so a mid-run `rename` is not clobbered.
- `resolve_identity` separates `Unique` from `Conflicting`, so resolve never guesses.
- Token boundaries and the `PP-76`/`PP-760` test cover over- and under-rewriting.
- Phase 7 re-runs `discover_items` after promotion.
- The Phase 6 H1-failure digest fallback makes the next sync push the correction.

**Findings**:
- 🔴 critical / medium — **Retirement mid-run leaves the precomputed plan, index and item snapshots stale** (Phase 4: run.rs). `plan`, `index`, `LazyItemDigests` and `per_id` are all built before `RetireKey` applies. Planned actions then read moved paths, record baselines under retired IDs, or overwrite rewritten references. Apply key changes as a pre-pass, then re-discover and re-plan.
- 🟡 major / high — **Pull marker cleanup defeats the `created` marker kept after a failed promotion** (Phase 5 §5; Phase 7 §2). Untracked discovery imports the key in the same run, and cleanup removes the marker, so the next sync creates a duplicate. Exclude marker-held keys from discovery, or limit the cleanup.
- 🟡 major / high — **Moved keys are never detected under the per-item retrieval strategy** (Phase 4 §4). `show(old)` returns the issue under its new key and is recorded as `Present`. Compare `issue.key` everywhere `show` is used.
- 🟡 major / medium — **Key-change collision checks ignore other items' `external_id`** (Phase 4 §4; Phase 2 §2). Refuse when another item already links the new key, and report both.
- 🟡 major / medium — **An interrupted retirement leaves both files, and later promotions refuse forever** (Phase 3 §3). Make resumption idempotent, or journal the intent.
- 🟡 major / medium — **Deterministic client-side failures are classified as retryable "tracker unreachable"** (Phase 1 §2). Restrict `NotSent` to `is_connect()`.
- 🔵 minor / high — **Promotion checks for a key collision before the key exists, and leaves H1-update failure unspecified** (Phase 7 §2). Reorder the check, and apply Phase 6's digest rule.
- 🔵 minor / medium — **File-path references to the retired item are left dangling** (Phase 3 §2). Add an exact-match rewrite of the old path or basename.
- 🔵 minor / medium — **Skipping only the `aliases:` line lets block-style entries be rewritten** (Phase 3 §2). Skip the whole value span.
- 🔵 minor / medium — **The tracker-key shape excludes valid Jira keys and over-matches numeric filenames** (Phase 5 §2; Phase 3). Use one shared predicate: a letter first, then `[A-Za-z0-9_]*-[0-9]+`.
- 🔵 minor / high — **The `create-batch` exit-code rule contradicts itself** (Phase 8 §3). State it as the per-item code with the greatest precedence, and name each outcome's code.

### Code Quality

**Summary**: The hexagonal shape is strong: pure decisions sit behind injected ports, and domain vocabulary is used throughout. The main risks are in composition. `try_run` and `run_sync` are already over clippy's length limits and absorb more branches. New file ports duplicate existing `corpus` ports. Marker decisions are split across two tables. Phase 1 makes deterministic client bugs look like network outages. Several new public types are loosely typed.

**Strengths**:
- Decision logic is pure and injectable (`decide_key_change`, `plan_retirement`, `mint_draft_id`, `push_precondition`).
- Retirement is split into plan and apply, with explicit `RolledBack`/`RestoreIncomplete` outcomes.
- Domain naming is rich and consistent.
- Existing duplication is consolidated: hierarchy walking moves to `work`, and per-item create is extracted.
- `drafts_dir` and `is_taken` are single definitions.

**Findings**:
- 🟡 major / high — **`try_run` grows into a multi-branch god function** (Phases 5–6). Dispatch to named strategies (`create_local_item`, `save_draft`, `create_tracker_keyed_item`) that return a common `CreationOutcome`.
- 🟡 major / high — **Promotion logic is added inline to an already-oversized `run_sync`** (Phase 7 §3). Extract a `promote_drafts` stage, and map `NotPromoted` to a failure class.
- 🟡 major / high — **Deterministic client-side failures are classified as `NotSent`/`Retryable`** (Phase 1 §2). Add a `RequestInvalid` variant that is provably unapplied but not retryable.
- 🟡 major / medium — **Two overlapping decision tables over the same marker state** (Phase 6 `MarkerPolicy`; Phase 7 `first_step`). Make `first_step` the sole authority for promotion and drop `MarkerPolicy`.
- 🟡 major / high — **New file ports duplicate the existing `corpus::scan`/`corpus::store` ports** (Phases 2–3). Build retirement I/O from `FileReader` + `AtomicWrite` plus a narrow remove, and enumerate the corpus through `CorpusWalker` in `work-cli`.
- 🔵 minor / medium — **Readers share enumeration but keep separate frontmatter parsing** (Phase 2 §4). Make `ItemIdentity` the single parse.
- 🔵 minor / high — **The same condition is encoded implicitly in one command and explicitly in another** (Phases 6, 8). Add a `CreatedButUnwritten` outcome.
- 🔵 minor / medium — **No injection seam is specified for the fault-injecting writer** (Phase 6 §1). Pass a `CreationPorts` struct.
- 🔵 minor / medium — **New public domain types use stringly-typed and tuple fields** (Phase 3 §2–3). Use `retired_item_content`, `BaselineRename { from, to }` and a typed cause.
- 🔵 minor / medium — **Most of the promotion sequence lives in the adapter, and the step order is unclear** (Phase 7 §2). Reorder the steps, and move the failure-to-reason mapping into the pure module.
- 🔵 suggestion / high — **`Ordering` collides with `std::cmp::Ordering`** (Phase 8 §2). Use `BatchOrder`, and rename `PromotionMode::Default`.

### Test Coverage

**Summary**: The plan is strongly test-first, with behaviour-named tests mapped to most ACs. There are real gaps. Phase 7's sync-level promotion tests are in a crate that cannot reach the code. Rollback fault injection cannot reach the baseline write or the file removal. The new pure decisions have no named tests. Pattern-consumer, manifest-validation and preview paths are untested.

**Strengths**:
- Every phase lists its failing tests first, with names that carry the AC data.
- Pyramid levels are sensible across unit, adapter, client and golden tests.
- Minting is made deterministic through `SuffixDraws`.
- Regression pins cover existing behaviour.
- Token-boundary tests are precise.
- 0296 Defect 1 gets an explicit regression test.

**Findings**:
- 🟡 major / high — **Sync-level promotion tests are placed in a crate that cannot reach the promotion loop** (Phase 7 §1). Move three of them into the `work-cli` `sync.rs` tests.
- 🟡 major / high — **Rollback fault injection cannot reach the baseline rename or the file removal** (Phase 3 §1). Parameterise the test over every operation index, using a faulting `AtomicWrite` for the baseline.
- 🟡 major / high — **The new pure decision functions have no named unit tests** (Phases 4, 6, 7). Add table tests for `decide_key_change`, all nine `first_step` cells, and `MarkerPolicy`.
- 🟡 major / high — **Pattern-consumer changes for `{tracker}` have no named tests** (Phase 5 §2). Cover `compile()`, the scan regex against key, draft and legacy filenames, dossiers, and the other entry points.
- 🟡 major / medium — **Nothing tests that aliases written by retirement are read back by resolution** (Phases 2–3). Add an `identities` parser suite and a retire-then-resolve adapter test.
- 🟡 major / medium — **`create-batch` manifest validation, exit precedence and the non-`{tracker}` path are untested** (Phase 8 §1).
- 🔵 minor / high — **Promotion preview, the `PullOnly` skip and ordering are untested** (Phase 7 §3).
- 🔵 minor / high — **`a_pull_consumes_no_local_number` has an assertion that always passes** (Phase 5 §1). Move it to the pull-adoption test.
- 🔵 minor / medium — **`locate` error paths, Linear's `null` issue and `--max-pulls` accounting are untested** (Phase 4 §4).
- 🔵 minor / medium — **The traceability table is grouped by phase and claims coverage for untestable ACs** (Testing Strategy). Use one row per AC, and mark the `config validate` AC as deferred to 0227.
- 🔵 minor / medium — **The pre-send `NotSent` mappings and the update path are not covered** (Phase 1).
- 🔵 suggestion / medium — **Pin the attempt limit exactly** (Phase 5 §1).

### Safety

**Summary**: Data safety is taken seriously where it is most visible. Retirement is designed to be all-or-nothing, with fault-injection tests. Markers are named after the draft ID. A `Missing` remote never deletes anything. Promotion and key changes are bounded. The protections are thinner at the edges that matter most. No lock serialises promotion or retirement. Rollback is in memory only. Retirement ignores the dirty-file invariant. Several paths can still create a duplicate remote issue or bind two items to one issue.

**Strengths**:
- Retirement is all-or-nothing, with fault-injection tests asserting byte equality.
- Refusals happen at plan time, and a `created` marker is kept on retirement failure.
- Draft-ID-named markers resist retitling.
- A `Missing` remote only warns.
- Changes are bounded by `--max-pulls`/`--max-pushes`, with preview and `--no-promote` available.
- Writes are atomic, fsynced and contained, and a test checks that files outside `meta/` are untouched.
- Phase 1 keeps "sent then failed" Terminal.

**Findings**:
- 🟡 major / high — **No lock serialises promotion or retirement, so concurrent runs can create duplicate remote issues** (Phases 3, 7). Hold a lock from reading the marker until it is removed, and add a two-promoter test.
- 🟡 major / high — **An interrupted `{tracker}` create leaves an `attempted` marker no draft owns, so a retry creates a duplicate** (Phase 6). Write the draft before sending, or surface orphaned `attempted` markers.
- 🟡 major / high — **`NotSent` can hide a create already sent on an earlier attempt of the 5xx retry loop** (Phase 1 §2). Return `NotSent` only if no earlier attempt got a response, and test a 503 followed by a refused connection.
- 🟡 major / medium — **Promotion overwrites the description of an existing remote issue when adopting** (Phase 7 step 5). Read back first, and skip the update if the remote has been edited.
- 🟡 major / medium — **Collision checks ignore `external_id`, so two local items can be bound to one remote issue** (Phases 2, 4, 5, 7). Add `is_linked`.
- 🟡 major / medium — **Retirement rewrites files with uncommitted changes, breaking sync's never-overwrite-dirty rule** (Phases 3, 7). Pass `WorkingCopyStatus` in, and stop before creating.
- 🟡 major / medium — **Rollback exists only in memory, so a crash can leave two copies of one item** (Phase 3). Make retirement resumable, or keep an intent record.
- 🔵 minor / medium — **Snapshot-then-write can lose concurrent edits and overwrite a target created meanwhile** (Phase 3). Compare bytes before each write, use `exclusive_write` for `to`, and take the update lock on `from`.
- 🔵 minor / medium — **Recovering an orphaned `created` marker depends on discovery, and local metadata is lost** (Phase 6; Migration Notes). Resolve markers directly, and store the intended content.
- 🔵 suggestion / low — **The creation lock is held across several network calls** (Phase 6).

### Compatibility

**Summary**: Most changes are additive or opt-in. `{tracker}` is gated, `aliases` is omitted when empty, legacy IDs are kept, and public-API changes go through `public-api:update`. The risks are in contracts the plan changes implicitly: key grammar, skill/binary skew between Phases 5 and 8, the loss of the rerun duplicate guard, and assumed tracker behaviour for moved keys.

**Strengths**:
- The identity foundations are pattern-independent, so legacy patterns are unchanged.
- An empty `aliases` is omitted, so older validators tolerate the files.
- `show` keeps its contract, and `locate` is added separately.
- The `work create` stdout shape is preserved.
- The golden and public-API snapshots are updated deliberately.
- The slug-marker transition is named in Migration Notes.

**Findings**:
- 🟡 major / medium — **Tracker key grammar excludes valid Jira project keys** (Phase 5; Phase 3). Define one grammar, `[A-Z][A-Z0-9_]*-[0-9]+`, case-insensitive.
- 🟡 major / high — **Skills and binary disagree under `{tracker}` between Phases 5 and 8** (Implementation Approach; Phases 5, 8). Defer the docs or make `next-number` refuse, and specify `--project` and canonical-directory drafts.
- 🟡 major / medium — **Naming markers by draft ID removes the rerun duplicate guard** (Phase 6; Migration Notes). Look up by digest before minting, and cover slug-named `attempted` markers.
- 🔵 minor / medium — **Key-change following relies on unverified tracker behaviour for old keys** (Phase 4). Capture moved-issue fixtures, and classify on structured fields.
- 🔵 minor / medium — **Deterministic pre-send failures change from exit 71 to a silent `local-save`** (Phase 1).
- 🔵 minor / high — **Extending the port breaks every `RemoteTracker` implementor, not only the ones listed** (Phase 4). List all nine sites.
- 🔵 suggestion / medium — **Older plugin versions fail hard on a shared `{tracker}` config** (Phase 5). Document the minimum version.

### Usability

**Summary**: Developer experience is well considered. Config errors are exact, resolve is case-insensitive and names the field behind an ambiguous match, and drafts have a clear lifecycle with recovery commands. The weaker spot is machine-readable output: four surfaces describe overlapping outcomes in different formats, and some signals are implicit. Several new failure paths have no specified message or recovery guidance.

**Strengths**:
- `{tracker}` rules produce exact messages, shared with `config validate`.
- Resolve matching is case-insensitive and names the field behind an ambiguity.
- Drafts promote by default and come with preview, `--no-promote` and recovery commands.
- An unreachable tracker becomes `local-save`.
- The skill's decline text becomes "No, save as draft", and a batch gets one push offer.

**Findings**:
- 🟡 major / high — **Promotion report lines break the existing sync TSV column order** (Phase 7 §3).
- 🟡 major / high — **Key-change and not-found output formats are unspecified or inconsistent** (Phase 4 §4).
- 🟡 major / medium — **The same create outcomes are named in three vocabularies, one of which depends on an empty column** (Phases 6, 8).
- 🟡 major / medium — **No user-facing message or recovery guidance for a partially restored retirement** (Phase 3 §3).
- 🔵 minor / high — **Promoting an already-promoted draft ID gives an unhelpful "not a draft" error** (Phase 7 §3).
- 🔵 minor / medium — **The interim `--push` refusal has no specified message, and the skill still offers push** (Phase 5 §4).
- 🔵 minor / medium — **`NotPromoted` reason keywords and recovery hints are undefined** (Phase 7 §2).
- 🔵 minor / medium — **`create-batch` manifest errors and the exit-code rule are underspecified** (Phase 8 §3).
- 🔵 suggestion / low — **`next-number` returning unreserved draft IDs is surprising** (Phase 5 §4).

### Standards

**Summary**: The plan mostly follows repo conventions: crate split, pup and public-API pinning, behaviour-sentence test names, comment-free sketches, kebab-case subcommands and ADR frontmatter. It drifts in machine-readable output, where promotion rows and `create-batch` outcomes break the keyword/TSV shape. It also drifts in skill config injection and in several domain-language collisions.

**Strengths**:
- Crate placement follows the pup rules, and each widening runs `public-api` and `pup:check`.
- Test names are behaviour sentences, with tests first.
- The sketches carry no comments.
- ADR-0069 numbering and frontmatter match ADR-0068.
- New subcommands and flags are kebab-case, with golden updates.
- The injection line and template comment match existing style.

**Findings**:
- 🟡 major / high — **Skills branch on `{tracker}` without injecting `work.id_pattern`** (Phases 7–8).
- 🟡 major / high — **Promotion report lines break sync's TSV record shape** (Phase 7 §3; Phase 4 §4).
- 🟡 major / medium — **`create-batch` prints prose outcomes instead of the CLI's outcome keywords** (Phase 8 §3).
- 🔵 minor / high — **One concept goes by several names across the key-change vocabulary** (Phase 4).
- 🔵 minor / high — **The `draft` Sync label skips the glyph-and-text convention** (Phase 2 §6).
- 🔵 minor / medium — **"alias" and "draft" already mean other things in this codebase** (Phases 2, 5).
- 🔵 minor / medium — **New `corpus` modules share a name with the `corpus` crate** (Phase 2 §3).
- 🔵 minor / medium — **The new read-only adapter sits outside the pup-guarded `filesystem` module** (Phases 2–3).
- 🔵 minor / high — **`E_TRACKER_PUSH_PENDING` is easily confused with `E_PUSH_PENDING`** (Phase 5 §4).
- 🔵 minor / medium — **New exit semantics bypass the exit-code taxonomy** (Phases 7–8).
- 🔵 suggestion / medium — **`hierarchy::Ordering` shadows `std::cmp::Ordering`** (Phase 8 §2).
- 🔵 suggestion / medium — **The fixture name drops the `.golden.json` suffix** (Phase 4 §3).
- 🔵 suggestion / low — **New subcommands have no `cli_<subcommand>.rs` black-box tests** (Phases 7–8).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🔴 **Correctness**: Retirement mid-run leaves the precomputed plan stale — Resolved
- 🟡 **Architecture / Safety / Correctness**: Retirement has no crash recovery — Partially resolved (resumption is designed, but callers' pre-checks make it unreachable)
- 🟡 **Architecture / Safety**: No lock serialises promotion or retirement — Resolved
- 🟡 **Safety**: Retirement rewrites files with uncommitted changes — Resolved, but the fix introduced the new critical finding below
- 🟡 **Safety / Compatibility**: Draft-ID markers lose rerun idempotency and orphan `attempted` markers — Partially resolved (the orphan is fixed; a rerun still sends a second create)
- 🟡 **Safety**: `NotSent` after an earlier 5xx attempt — Resolved
- 🟡 **Correctness**: Pull marker cleanup defeats the `created` marker — Resolved
- 🟡 **Correctness / Safety**: Collision checks ignore `external_id` — Resolved
- 🟡 **Correctness**: Moved keys undetected under per-item reads — Resolved
- 🟡 **Safety**: Promotion overwrites an adopted issue's description — Partially resolved (the skip branch's baseline hides the divergence)
- 🟡 **Code Quality / Correctness / Compatibility**: Deterministic pre-send failures treated as retryable — Resolved, but `Rejected` now conflicts with the exit-code taxonomy (new finding)
- 🟡 **Architecture / Code Quality**: Promotion outside the engine / oversized `run_sync` — Partially resolved (the pre-pass shares the budget; preview and refusal authority are still split)
- 🟡 **Architecture / Code Quality**: Creation sequence written twice / `try_run` god function — Resolved
- 🟡 **Code Quality**: Overlapping marker decision tables — Resolved
- 🟡 **Code Quality / Architecture**: New ports duplicate `corpus` ports; corpus enumeration unplaced — Still present (§4 is still conditional; no `corpus-adapters` dependency has been decided)
- 🟡 **Usability / Standards**: Sync TSV shape — Partially resolved (four columns now, but `-` sits in the state column and details are free text)
- 🟡 **Usability / Standards / Code Quality**: Three create-outcome vocabularies — Partially resolved (`loud-terminal` still conflates "may exist" with "rejected")
- 🟡 **Usability**: `RestoreIncomplete` guidance — Resolved
- 🟡 **Compatibility**: Phase 5–8 skill/binary skew — Resolved
- 🟡 **Compatibility**: Tracker key grammar — Resolved
- 🟡 **Standards**: Skills don't inject `id_pattern` — Partially resolved (create-work-item's `{tracker}` wording lands in Phase 6, but its injection only in Phase 8)
- 🟡 **Test Coverage**: Sync-level promotion tests misplaced — Partially resolved (the `run_sync` composition has no test at its own level)
- 🟡 **Test Coverage**: Rollback fault reach, pure decisions and `create-batch` gaps — Resolved
- 🟡 **Test Coverage**: Pattern consumers and the alias round trip — Partially resolved
- 🔵 Most minors and suggestions — Resolved; see the per-lens notes above

### New Issues Introduced

- 🔴 **Correctness / Architecture / Usability / Safety**: The uncommitted-changes guard refuses every freshly written draft. `VcsWorkingCopyStatus` counts untracked files as dirty (`cli/vcs-adapters/src/library/dirty_paths.rs`). As a result, draft-first `work create --push` and routine promotion always stop at `uncommitted-changes`. The tests pass only because their fakes always report clean.
- 🟡 **Correctness / Architecture / Safety**: Resumable retirement is unreachable. The `is_taken`/`is_linked` pre-checks in `promote` and `settle_identities` refuse the half-retired target first. A crash after `from` is removed has no trigger to finish the baseline rename or remove the marker.
- 🟡 **Correctness**: A key change the pre-pass does not apply is imported as a duplicate. This happens under preview, on any refusal, on uncommitted changes, and on a budget refusal. `IdentityReport` should carry every detected new key into `untracked_to_import`.
- 🟡 **Correctness / Safety**: The adopt-path H1 rule compares a body digest with `request_digest(title, body, kind)`, which can never match. The skip branch then records a baseline that hides the divergence.
- 🟡 **Compatibility / Standards / Usability**: `TrackerError::Rejected` maps to `loud-terminal` / 71, whose documented meaning is "a remote issue may exist". It exits 71 from `create` but 4 from sync. `for_tracker_error` and the other `TrackerError` match sites are not listed in the plan.
- 🟡 **Usability**: The create and `create-batch` outcome tables do not cover every `NotPromoted` value. Missing: `UncommittedChanges`, `IdTaken`, `KeyLinked` after a create, and `RetirementIncomplete`.
- 🟡 **Code Quality**: `NotPromoted` references `RetirementCause`, which is defined in `work-adapters`, a crate `work` cannot import. It also mirrors the retirement refusal and failure variants. Separately, step 1 plans a retirement before the new ID exists.
- 🟡 **Architecture**: The two-stage run still splits preview and refusal authority. The engine previews over an unsettled corpus, and promotions that already created remote issues stand when the engine refuses.
- 🟡 **Architecture / Standards**: Corpus enumeration, the retirement lock and the `FileCorpusStore` wrapper have no decided home. `work-adapters` does not depend on `corpus-adapters`, and `SyncPorts` has no fields for them.
- 🟡 **Test Coverage**: The `run_sync` composition (settle → rediscover → reload → remaining budget) is tested only through hand-built adapter harnesses.
- 🟡 **Test Coverage**: Traceability rows 34 and 54 name tests that no phase defines.
- 🔵 **Minors across lenses**:
  - **Concurrency and recovery**:
    - lock ownership is unclear when `settle_identities` calls `promote`;
    - the working-copy status is not refreshed after the pre-pass;
    - `RetirementIncomplete` should stop the run;
    - a rerun after a tracker error still creates a duplicate;
    - a create and a concurrent sync can race;
    - the order of the path-rewrite rules is unspecified;
    - the key-change and promotion budgets should be checked together.
  - **Contracts**:
    - the hash-pinned push-decide golden is not updated;
    - a not-found item gets both a not-found row and a remote-absent row;
    - 0227 cannot reach the `validate_id_pattern` capability flag;
    - Phases 1 and 8 lack the public-api step;
    - the pup allow-list needs `kernel::Error`.
  - **Naming**:
    - `WorkingCopyStatus` duplicates `Dirtiness`;
    - `📝` breaks the circle label set;
    - `CreationPorts` has an optional tracker;
    - `is_`-prefixed functions return `Option`;
    - "identities" names three different things.
  - **Unspecified test seams**:
    - a route that serves a 503 then shuts down;
    - the spy numeric allocator;
    - expected values for the pattern entry points;
    - the positive branches of the H1 rule;
    - `locate` returning an error on auth failures.

### Assessment

The revision fixed the pass-1 critical finding and most of the majors. The design is now coherent: one promotion service, a settle-then-plan sync, and typed failures. It is not yet ready. Draft-first creation combined with the uncommitted-changes guard blocks the headline `{tracker}` flow. Resumption, deferred key changes and the adopt path each break where two new mechanisms meet.

A third pass should cover:

1. Scope the dirty guard to exempt the retired item's own file, and preserve uncommitted bytes for recovery.
2. Fold the collision checks into `plan_retirement`, and add a post-move reconciliation.
3. Carry unapplied moved keys into discovery.
4. Fix the adopt digest comparison and the baseline it records.
5. Give `Rejected` its own keyword and exit code.
6. Make the outcome tables cover every `NotPromoted` value.
7. Decide the `corpus-adapters` dependency.

## Re-Review (Pass 3) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🔴 **Correctness / Architecture / Usability / Safety**: Dirtiness guard refuses every fresh draft — Resolved (replaced by recovery copies)
- 🟡 **Correctness / Architecture / Safety**: Resumable retirement unreachable — Resolved for the collision checks, which now sit in `plan_retirement`, and for reconciliation. The new reconciliation step raises its own findings (below).
- 🟡 **Correctness**: Unapplied key change imported as a duplicate — Resolved (`followed_keys`)
- 🟡 **Correctness / Safety**: Adopt-path digest comparison — Partially resolved (`created_remote_hash` fixes the comparison, but when it is written and refreshed is underspecified)
- 🟡 **Compatibility / Standards / Usability**: `Rejected` overloads 71 — Resolved in design (exit 75); the oracles and the sync skill are not yet updated
- 🟡 **Usability**: Outcome tables not total — Resolved
- 🟡 **Code Quality**: `NotPromoted` crate direction; retirement planned before key exists — Resolved
- 🟡 **Architecture**: Preview/refusal authority split — Partially resolved (the ceilings are checked together, but the preview projection is incomplete for moved items)
- 🟡 **Architecture / Standards**: No home for corpus enumeration, lock or store — Resolved (`corpus` ports, `corpus-adapters` impls, `work-cli` wiring)
- 🟡 **Test Coverage**: `run_sync` composition untested; traceability rows 34 and 54 — Resolved
- 🔵 Pass-2 minors — Mostly resolved. Two are partial: the `-` state still appears in the `already-promoted` row, and "identities" still names three things.

### New Issues Introduced

- 🟡 **Correctness / Safety / Usability**: The rerun guard can never match. `request_digest` hashes the body after the ID is substituted (`cli/work-cli/src/create.rs:642` → `:489`), so each draft-first rerun digests a different draft ID. The guard also ignores `created` markers. (Verified in source.)
- 🟡 **Architecture / Correctness**: Retirement writes are not serialised with `work update`. `FileCorpusStore`'s `AtomicWrite` takes no lock (`cli/corpus-adapters/src/store.rs:123-127`), and `work update` takes its own `<path>.lockdir`. The apply-time snapshot also misses edits made in the plan → network → apply window. (Verified in source.)
- 🟡 **Safety / Architecture / Correctness**: Reconciliation has three problems. It runs without the retirement lock, so it can finish a live promotion. It reimplements promotion's tail. It records a fresh read-back baseline, which loses the H1 skip or failure override and hides divergence.
- 🟡 **Correctness / Code Quality**: The baseline's source is ambiguous. `record_created_baseline` both does its own read-back and "reuses" the pre-H1 one, so a fresh create would look remotely changed. The digest selection lives in adapter prose and relies on a sentinel. The lifecycle of `created_remote_hash` is unspecified: it needs a second marker write, it goes stale after a successful H1 update, and its `None` case is undefined.
- 🟡 **Architecture / Test Coverage / Safety**: Recovery copies are not write-once. A resumed retirement can overwrite the original copies and then delete them on rollback. No test asserts that the copies survive an incomplete restore. The directory also lacks a fail-closed `.gitignore`.
- 🟡 **Architecture / Correctness**: The engine re-gathers and receives `Moved`/`NotFound` facts it cannot classify. This happens under preview and on refusal, and it produces duplicate `remote-absent` rows. Each sync also reads the remote twice, which the performance notes don't acknowledge.
- 🟡 **Code Quality / Architecture / Standards**: `corpus::lock::ExclusiveLock` returns a `LockGuard` that exists only in `corpus-adapters`. The new ports' error types (`kernel::Error`) also differ from their sibling `AtomicWrite` (`StoreError`).
- 🟡 **Compatibility**: The identity-first `resolve_with` silently changes how `work sync --target` resolves, and makes the `LocalCollision` and `AmbiguousExternal` diagnostics unreachable.
- 🟡 **Compatibility**: The identity pre-pass has no scope for targeted runs. A `--target X` sync could promote every draft and rewrite references across the whole of `meta/`.
- 🟡 **Compatibility / Standards**: Phase 1 doesn't carry exit 75 into the push-decide golden and its parity hash, `FROZEN_DISPATCH_CODES`, the band doc in `exit_codes.rs`, or `sync-work-items/SKILL.md`.
- 🟡 **Compatibility**: Line 1 of the legacy-pattern `created-unwritten` and `rejected` output is unspecified, but the skill assumes line 1 is an existing file.
- 🟡 **Usability**: A collision after a create reads as a transient `created-unwritten` from `create` but as exit 4 from sync, so the guidance points to the wrong fix.
- 🔵 **Minors across lenses**:
  - A promotion killed after writing `to` makes `work promote` ambiguous.
  - The reconciliation trigger doesn't check the draft alias, and its behaviour under `--preview` and against the budget is unspecified.
  - Rollback restores snapshots without checking for concurrent edits.
  - Exclusive creation of `to` bypasses the ports.
  - `settle_identities` absorbs orchestration; `SettledView` is temporally coupled to `SyncRequest`; `SyncPorts` and `PromotionPorts` duplicate fields.
  - `TargetExists`, `ItemNotFound` and `adopt-conflicts-with-recorded-key` have no keywords, and there is a stale bullet saying Phase 6 `Rejected` maps to `loud-terminal`.
  - Resolve's filename fallback under `{tracker}` is untested.
  - The batch `rejected` precedence is untested, as are the budget spent by failed actions and markers that lack a hash.
  - The report's `#\tdetail` ordering conflicts with the sort, and several new keywords and lines are missing from the sync skill.
  - The concurrency tests don't say how their interleaving is forced.
  - The skill-guard counts are misstated.
  - Assorted naming nits.

### Assessment

Pass 2's critical finding and most of its majors are resolved. No lens raised a critical in pass 3. Each pass, though, finds a new set of majors at the seams of the previous fixes. Here those seams are reconciliation, the baseline source, recovery copies, rerun digests and concurrent writers, and nearly all of them are in the promotion and retirement lifecycle when a process crashes or two processes run at once. The pattern suggests the lifecycle state is spread across too many places:

- the draft;
- the `created` marker;
- the baseline;
- the recovery directory;
- in-memory step-5 decisions.

Recommended direction for pass 4:

1. **One promotion record.** Make the `pending_push` marker the single durable record of an in-flight promotion. Write it atomically at each transition with everything needed to finish deterministically: the ID-independent request digest, the key, `created_remote_hash` after any H1 update, the H1 outcome, the intended baseline hashes, and the recovery directory. Promotion, reconciliation and `work promote` then all call one `finish_promotion` under the retirement lock.
2. **Plan-time digests.** Retirement verifies each file against the digest recorded at planning, and takes the same per-file lock `work update` uses.
3. **Write-once recovery copies.** Recovery copies are never overwritten, are removed only on completion, and sit behind a fail-closed ignore rule.
4. **Settled facts for the engine.** Pass the settled facts to the engine instead of re-gathering, and scope the pre-pass to `--target`.
5. **Contract follow-through.** Complete the updates for exit 75, legacy line 1, `--target` resolution, and the keyword tables.

## Re-Review (Pass 4) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness / Safety / Usability**: Rerun guard can never match — Resolved in design (ID-independent digest, any stage, record-less drafts). Legacy recompute is impossible (see below), and the Created-stage branch is untested.
- 🟡 **Architecture / Correctness**: Retirement writes not serialised with `work update`, and the window between planning and applying — Resolved (per-file locks, plan-time digests)
- 🟡 **Safety / Architecture / Correctness**: Reconciliation without the lock, a duplicated tail, and a fresh baseline — Resolved (records only, `finish_promotion` under the lock, `intended_baseline`)
- 🟡 **Correctness / Code Quality**: Baseline source and `created_remote_hash` lifecycle — Mostly resolved (`bodies_agree` undefined; read-back failures unspecified)
- 🟡 **Architecture / Test Coverage / Safety**: Recovery copies not write-once — Resolved in design. No port can write them outside the corpus root (see below).
- 🟡 **Architecture / Correctness**: Engine re-gathers — Resolved (`prepare_run_with`). Promoted items now lack facts (see below).
- 🟡 **Code Quality / Architecture / Standards**: Lock guard and error types — Resolved (`HeldLock`, `StoreError`)
- 🟡 **Compatibility**: `--target` resolution; unscoped pre-pass; exit-75 follow-through; legacy line 1 — Resolved
- 🟡 **Usability**: Collision after create — Resolved (`created-blocked`, exit 4 from both commands)
- 🔵 Pass-3 minors — Mostly resolved

### New Issues Introduced

- 🔴 **Correctness / Architecture / Safety**: A `Retiring` record with nothing applied is "finished" without retiring. This happens after a kill before `to` is written, after two clean rollbacks, or after a re-plan refusal. `finish_promotion` re-applies only when an interrupted target exists, so it records a baseline for a non-existent item and deletes the record. The next sync then promotes the draft again and creates a duplicate issue. `RetirementRecord` has the same gap after a rollback.
- 🟡 **Architecture**: Items promoted during settlement reach the engine with no gathered fact. `GatheredFacts::plan_inputs` `expect`s every item (`cli/work-adapters/src/sync/fetch.rs:94-97`), so it would panic. (Verified.)
- 🟡 **Architecture**: No port can write recovery copies outside the corpus root. `FileCorpusStore` bounds writes to its root (`cli/corpus-adapters/src/store.rs:47-52`), and `.accelerator/state/` lies outside `meta/`. Every retirement of a fresh (dirty) draft would fail. (Verified.)
- 🟡 **Compatibility / Correctness**: Normalising `request_digest` breaks `ReuseId` for existing `created` markers on the sync and legacy create paths. The promised legacy recompute is impossible, because markers store only `title`, `digest`, `attempted_at` and `failure` (`cli/work-adapters/src/sync/pending_push.rs:90-101`). (Verified.)
- 🟡 **Compatibility / Code Quality**: `PromotionRecord` and `PendingPush` model one file with no schema or discriminator. `outstanding` aborts on the first unreadable marker (`read(...)?`), which blinds every guard and warning. `RetirementRecord` has no location or schema. (Verified.)
- 🟡 **Compatibility**: From Phase 5 to Phase 7, sync pushes drafts through `create_from_local` and links them without retiring. Phase 7 then promotes them again, creating duplicate issues.
- 🟡 **Code Quality / Correctness / Architecture**: `next_step(Option<&PromotionRecord>)` cannot represent an unreadable record, which its own table test expects and `MarkerState` deliberately models.
- 🟡 **Correctness**: `bodies_agree` has no defined inputs, source or result. A kill between the H1 update and `RemoteRetitled` then yields a spurious conflict, or a draft-ID H1 stays while the item reports as synced.
- 🟡 **Safety / Usability / Compatibility**: Rerunning a partly completed `create-batch --push` re-creates the items that already succeeded. The guard refusals also have no batch keyword.
- 🟡 **Usability**: The create-work-item skill doesn't learn `created-blocked`, `retirement-incomplete`, `E_PUSH_PENDING` or `E_DRAFT_EXISTS`. Its `loud-terminal` row gives wrong advice under `{tracker}`.
- 🟡 **Usability**: The batch outcome strings need the holder and recovery directory, but the batch line doesn't carry them. `retirement-incomplete` points only at VCS.
- 🟡 **Test Coverage**: The Created-stage and legacy-fallback branches of the rerun guard, and the digest's independence from the ID, are untested.
- 🔵 **Minors across lenses**:
  - `RemoteKept` name collision;
  - partial decision functions;
  - key-change reconciliation keeps its own tail;
  - `create-batch` names 2 of 3 strategies;
  - `HeldLock` has no constructor;
  - `RetirementPorts` has no home and omits the baseline store;
  - Phase 3 depends on Phase 4/6/7 constructs;
  - the engine reads records directly beside `SettledView`;
  - read-back failures in `RetitleRemote`;
  - `ItemNotFound` has no create row;
  - `RestoreIncomplete` line 1;
  - the update body is not rewritten;
  - `per_file_lock_name` duplicates `corpus-adapters::lockdir`;
  - `IdPatternError`/`PatternError` bridging;
  - "rejected" vocabulary drift;
  - `exit_codes_parity.rs` cannot hold keyword mappings;
  - a pup probe pair is missing;
  - `#\tdetail` field parsing;
  - row-id convention;
  - a concurrent sync can import during the `Attempted` window;
  - whole-document baseline restore;
  - the recovery directory is deleted while a human restore is pending;
  - test seams (`failing_locate`, `parking_create` handshake), adapter tests, test placement.

### Assessment

The record-based redesign held up. Every pass-3 lifecycle major is resolved in design, lock ordering is deadlock-free, and there are no findings left about divergence hidden by the baseline. The remaining problems fall into three groups:

- **The critical.** One rule: `finish_promotion` must re-plan the full retirement while the draft ID is still an item's `id`, and a rollback must rewind the record.
- **Missing seams.** The engine needs promoted items' facts. The recovery-copy store has no port. The on-disk schema is unspecified. A tolerant `outstanding` is needed.
- **Compatibility and phasing.** Keep `request_digest` and add a separate ID-free digest field. Move the draft filter into Phase 5. Give `create-batch` a resume journal.

Most of these are specification gaps rather than design flaws. Several were verified against the source.

## Re-Review (Pass 5, narrow: correctness, safety, compatibility) — 2026-09-27

**Verdict:** REVISE

### Previously Identified Issues

- 🔴 **Correctness / Architecture / Safety**: A `Retiring` record with nothing applied was finished without retiring — Resolved. `finish_promotion` re-plans while the draft ID is still an `id`, a rollback rewinds the record, and a `RetirementRecord` is removed on a clean rollback.
- 🟡 **Architecture**: Promoted items had no engine facts — Resolved for this run's promotions (`promoted_keys`). A variant remains for items written concurrently (below).
- 🟡 **Architecture**: No port for recovery copies — Resolved (`RecoveryCopies`)
- 🟡 **Compatibility / Correctness**: `request_digest` normalisation — Resolved (unchanged; `content_digest` added)
- 🟡 **Compatibility / Code Quality**: Marker schema and intolerant `outstanding` — Resolved. The source confirms that older readers accept the `Attempted`/`Created` records. The tolerant reader now fails open for guards (minor, below).
- 🟡 **Compatibility**: Phases 5–7 duplicate issues via `create_from_local` — Resolved (filter moved to Phase 5)
- 🟡 **Code Quality**: `next_step` couldn't represent an unreadable record — Resolved (`RecordState`)
- 🟡 **Correctness**: `bodies_agree` undefined — Resolved (`remote_untouched`, `read_back_matches_*`)
- 🟡 **Safety / Usability / Compatibility**: A `create-batch` rerun duplicated items — Partially resolved (see below)
- 🟡 **Usability / Test Coverage**: Create skill table, batch detail lines, rerun-guard tests — not re-checked in this pass (usability and test-coverage were not run)
- Correctness and safety each traced every `RecordState` × stage × crash × rollback × concurrent create/sync/promote. Neither found a remaining path to a duplicate remote issue or a silently lost remote edit. Lock ordering is deadlock-free.

### New Issues Introduced

- 🟡 **Correctness / Safety / Compatibility**: The batch digest file is all-or-nothing and matches only byte-identical manifests. After an interruption the unreached items are stranded, and `work sync` has no drafts to promote for them. `extract-work-items` regenerates its manifest, so a real rerun rarely matches, and the items that already succeeded are created again.
- 🟡 **Correctness**: `finish_retirement` returns `RetirementFailure`, which cannot carry a planning `RetirementRefusal`. After a refusal, the `RetirementRecord` (or a `Retiring` promotion record) is left behind and reconciled on every run.
- 🟡 **Correctness**: An item written by another process between settlement's gather and the re-discovery reaches `plan_inputs` with no gathered fact, and `plan_inputs` `expect`s one (`cli/work-adapters/src/sync/fetch.rs:94-97`), so it panics.
- 🟡 **Correctness**: The rule for a user-named `--adopt` is contradictory. Phase 7 says "H1 never rewritten", but `intended_baseline` makes the next sync push the key H1 when the bodies match. The `promoted` row also says `synced` while a push is pending (`UpdateFailed` has the same problem).
- 🟡 **Correctness**: "Retire, retrying once" (AC 12) is defined only for `ChangedSinceSnapshot`. A `RolledBack` from a store failure stops after one attempt.
- 🟡 **Compatibility**: Phase 1's list of match sites omits `cli/jira-cli/src/exit_codes.rs` and `cli/linear-cli/src/exit_codes.rs`. Both match `ClientError::Transport`/`BadPath` exhaustively (`jira-cli/src/exit_codes.rs:186-189`, verified), and their exit codes and `E_REQ_*` messages are pinned by fixtures.
- 🔵 **Minors**:
  - **Guards and records**
    - The tolerant `outstanding` fails open for three guards: an unreadable `<draft-id>.json` should count as pending for its draft.
    - A promotion record whose draft vanished is never cleaned up.
    - The pull re-check lacks the draft-exists condition.
    - `item-not-found` exits 4 from sync but 71 from create.
    - A `Retiring` record that is re-planned can nest its `before` stage, and `resumes` misses a crash that happened after the rewrites.
    - `--adopt`/`--create` don't handle an unreadable record.
    - `AlreadyDone` short-circuits before finishing an outstanding record.
    - Test wording still says "created marker".
  - **Read-back failures**
    - `TrackerUnreachable` is overloaded after a create, so it reports `local-save` although an issue exists.
    - Failure of the Retire step's read-back is unspecified.
  - **Recovery and locking**
    - `RESTORE-PENDING` outlives a later completion, and its "restore" remedy then reverts good content.
    - `author_from_remote` nests the create lock and then the retirement lock, which cascades lock timeouts into `work create`. The global lock order is undocumented.
  - **Naming, notes and exit codes**
    - A no-hash test name contradicts `remote_untouched`.
    - The Migration Notes' legacy-marker rule contradicts Phase 6 §7.
    - The same guard exits 1 from create but 4 from batch, and `E_BATCH_ALREADY_PUSHED` wrongly uses 2 (`USAGE`).
    - The later-stage `kind` strings are unnamed.
    - A downgrade silently suppresses warnings.

### Assessment

The lifecycle is now sound. Two independent traces found no path to a duplicate issue or a silently lost remote edit, and lock ordering is deadlock-free. What remains is one design gap (the batch rerun) and specification fixes: the refusal type, gather coverage for newcomers, the adopt H1 rule, the retry count, and the standalone CLIs' codes. The count of majors crosses the REVISE threshold, but none is a critical.

## Verdict Update — 2026-09-27

**Verdict:** APPROVE

All pass-5 findings were applied to the plan: the per-entry `create-batch`
journal, the user-named `--adopt` H1 rule, `FinishFailure`, deferral of
items without gathered facts, the two-attempt retirement, the standalone
`jira-cli`/`linear-cli` exit-code contract, and every minor. The reviewer
approved the plan without a further pass. Two work item amendments recorded
in the plan remain to be applied to 0230 before Phases 5 and 6.

