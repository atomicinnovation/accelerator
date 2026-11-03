---
type: "pr-review"
id: "169-review-1"
title: "[0300] Land sync pulls into tracked work items and report every failure"
date: "2026-10-24T19:18:10+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:169"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "security", "compatibility", "usability"]
review_number: 1
pr_number: 169
tags: []
last_updated: "2026-10-24T19:18:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #169 - [0300] Land sync pulls into tracked work items and report every failure

**Verdict:** COMMENT

All eight lenses agree the core of this change is strong. The pull path holds its lock for the whole attempt and re-checks the file before writing. `ItemAttempt` gives each failure its class from where it happened, not from guesswork at each call site. Containment is checked on every path, and every failure leaves a visible record. Nothing critical turned up. The remaining risks are concentrated in four places:
- **Newer relink and order paths**: the item's landing state comes from the error variant, not from the attempt.
- **Writers outside the bounded stores**: the store boundary does not yet cover every writer in a sync run.
- **One copy-skip rule**: it trusts a formatting-insensitive hash.
- **The `sync-work-items` skill**: it asks the LLM to interpret a lot, and it already promises things the known gaps 0351 and 0352 break.

The orchestration hubs (`sync.rs`, `run.rs`, `apply.rs`) have also grown to a size that will tax every future change.

### Cross-Cutting Themes

- **Landing state comes from the error variant, not the attempt** (flagged by: correctness, code-quality) — `ApplyError::progress()` returns `NothingLanded` for every variant except `Io` and terminal `Tracker`.
  - Three concrete cases follow: a relink `create` unlinks before it settles the order, the adoption record is observed rather than landed, and author-side `NotDurable` is lost.
  - The `unreachable!` panics in `settle_order`, `relink_steps` and `adopted_key` lean on invariants held in another crate.
  - Carrying the attempt's progress on every `ApplyError`, and the facts in the types, would make the classification total by construction.
- **The bounded-store boundary covers only the engine's apply path** (flagged by: architecture, code-quality, security) — the same run still writes through unbounded `FileCorpusStore` roots:
  - `assemble_run` builds project, state and integrations stores outside the bound;
  - retirement and promotion records write through those stores;
  - `SyncAuthor::rewrite_frontmatter` link and unlink writes use `FileCorpusStore::new(work_dir)`;
  - `RootedStore` is private to `work-cli`;
  - reads go through raw `std::fs`.

  The next writer will copy whichever pattern is nearest.
- **The orchestration hubs keep growing** (flagged by: architecture, code-quality):
  - `work-cli/src/sync.rs` is about 9.2k lines (about 2.4k production) and still allows `too_many_lines` on `run_sync_with` (about 305 lines);
  - `run.rs` is about 2.8k lines and `apply.rs` about 1.75k;
  - the extraction to continue is report rendering, dossier persistence, order parsing and pending-push handling.
- **The skill contract carries heavy LLM interpretation and is already drifting** (flagged by: usability, test-coverage, compatibility):
  - the skill promises that a preview shows every failure (contradicted by 0351);
  - the membership table has no rule for the duplicate `noop\tunsynced` row (0352);
  - terminal recovery depends on parsing an `after <step>:` prefix out of free text;
  - `adopt-record` gets no retryable advice, yet the structure test's hand-copied vocabulary still passes;
  - the skill still describes the removed `--resolve` warn-and-skip behaviour.
- **The outside-root rule is broader than documented** (flagged by: compatibility, usability) — `BoundedRoot::for_configured` refuses an out-of-repository `paths.work` from team config as well as `paths.integrations`. The CHANGELOG, `configure`, the skill's exit-74 row and migration `0011` describe only `paths.integrations`.

### Tradeoff Analysis

- **Safety vs usability (kept copies)**: safety wants a copy whenever the file's bytes differ from what the last sync wrote, so untracked formatting-only edits are not lost. That produces more `kept-edits` noise for users. I recommend safety: untracked items have no VCS fallback, and the sweep already tidies copies that turn out to be settled.
- **Code quality vs PR scope**: the hub-file splits and the `prepare_sync` refactor are well-founded, but this PR is already about 33k lines. I recommend filing them as follow-ups, but fixing the correctness-adjacent ones here: `progress()`, the relink order, and the `Unclassifiable` class keyword.
- **Test determinism vs real-world coverage (visualiser)**: injected watcher events removed real flakiness from the default lane, but the OS adapter now runs in no CI leg. A Linux-only un-ignored OS test keeps both benefits.

### Strengths

- ✅ The pull path holds the item's `ForFile` lock for the whole of `pull_steps` and goes lock → read → recheck → keep copy → hash → re-read → recheck → write. It refuses whenever a copy cannot be kept, and the tests edit the file at exactly the two recheck points.
- ✅ `ItemAttempt` (`observe_*` vs `land_*`) puts the retryable/terminal classification in one small, testable type. `classification_is_total` and `tracker_failures_keep_their_named_class` keep it total.
- ✅ `SyncOperation`, `FailureClass::keyword` and `RunReport::failures()` give one vocabulary. Exit codes and stderr lines both derive from `failures()`, and every failure uses the same `error: <subject>: <operation> failed: <cause>` shape.
- ✅ Containment is substantially tighter:
  - `unresolved_tail` refuses `..` through missing directories;
  - the recovery store checks every path component for a link;
  - every path built from an id or tracker key is checked to be a single plain component;
  - tracked or foreign-key markers and adoption records are distrusted;
  - `TsvRecord`/`single_line` prevent forged records.
- ✅ `BoundedRoot`/`ValidatedRoots` model the root rules as domain types whose refusals tell the user how to fix the problem, checked before any tracker call.
- ✅ `DossierAuthority` and the kept-edits sweep keep data when in doubt: a preview retires and sweeps nothing, and an unexpected combination of states maps to `Unobserved`.
- ✅ `work::inventory` is a pure functional core: one pass yields both items and inventory, so the two cannot disagree.
- ✅ Breaking changes are listed in the CHANGELOG and every in-plugin consumer skill was updated in step. The new `#`-prefixed records are pinned by `sync-report-records.golden`, and migration `0011` is idempotent and only appends rules.
- ✅ `server::run_with` with an injected watcher is a real dependency-injection seam, and it removes sleep-and-retry flakiness from the SSE tests.

### General Findings

- 🟡 **Code Quality**: `sync.rs` has grown to about 9.2k lines, mixing orchestration, rendering, dossier persistence and a 6.7k-line test module. Continue the split into `sync/report_lines.rs`, `sync/dossiers.rs` and `sync/orders.rs`.
- 🟡 **Usability**: The `sync-work-items` skill leans heavily on the LLM to interpret output. Move more of the classification into the CLI (a group and a blocks-completion field, or a `#\tsummary\tfailed\t<N>` record) so the skill relays rather than infers.
- 🔵 **Architecture**: `sync.rs`, `run.rs` and `apply.rs` keep taking on responsibilities. Extract render, dossier-file and pending-push-order modules.
- 🔵 **Code Quality**: Several restating doc comments on private helpers go against the project's comment policy:
  - `row_detail`, `create_notices`, `ids_to_discard` and `FailureClass::keyword` only restate their names;
  - the `Contents` comment is misleading;
  - `utc_date` returns an RFC 3339 instant and should be renamed.
- 🔵 **Code Quality**: Bounded stores handle writes while raw `std::fs` handles reads and listing in the same flows (`prepare_conflicts_dir`, `retire_dossiers`, `read_marker`).
- 🔵 **Correctness**: `ApplyError::progress()` is derived from the variant, not from the attempt (see the cross-cutting theme).
- 🔵 **Test Coverage**: The post-validation fixes were tested only against `RecordingTracker`. Re-run the `show`/`discover` run-level error path against a real client.
- 🔵 **Safety**: On every apply run, the conflicts-folder cleanup removes any `TEMP_PREFIX` file. Confirm a run-level lock rules out deleting a concurrent run's in-flight dossier.
- 🔵 **Compatibility**: The `sync-work-items` skill (about line 557, outside the diff) still describes `--resolve` warn-and-skip, which now exits 2.
- 🔵 **Usability**: The exit code no longer signals failure reliably (4 outranks 70/74/75), and `#\tsummary\tsynced\t<N>` is undocumented (0353).
- 🔵 **Test Coverage**: The skill-content tests pin full sentences. Narrow them to the tokens that matter: record types, keywords, command forms and section order.
- 🔵 **Security**: The check-to-use windows that remain can only be raced by the same user, and per-write re-checks mitigate them. No action needed beyond the `sync_author.rs` inline comment.

### Additional Findings

- 🟡 `skills/work/sync-work-items/SKILL.md:437` — Membership table has no rule for the duplicate `noop\tunsynced` row (0352) (usability)
- 🟡 `skills/work/sync-work-items-recovery/SKILL.md:21` — Terminal recovery depends on parsing a free-text cause prefix (usability)
- 🟡 `cli/work-cli/src/cli.rs:418` — `--discard-kept-edits <id>` without `--target` runs a full bidirectional sync (usability)
- 🔵 `cli/work-adapters/src/sync/apply.rs:1402` — Adoption record and baseline removal are observed, not landed (correctness)
- 🔵 `cli/work-adapters/src/sync/apply.rs:1051` — Author-side writes cannot report `FailedAfterPublish` (correctness)
- 🔵 `cli/work-adapters/src/sync/apply.rs:873` — Re-check accepts a different content from the one the copy decision used (correctness)
- 🔵 `cli/migrate/src/migrations/m0011.rs:105` — An integrations root under `.accelerator/state` not named `integrations` gets no ignore rules (correctness)
- 🔵 `cli/work-adapters/src/sync/apply.rs:914` — Kept copy's folder entries are not flushed before the overwrite (safety)
- 🔵 `cli/work-adapters/src/retirement.rs:565` — Retirement sweep deletes a folder whose copy is unreadable (safety)
- 🔵 `cli/work-adapters/src/sync/adoption_records.rs:233` — `follow`/`rename` can turn a distrusted tracked record into a trusted one (security)
- 🔵 `cli/work-cli/src/sync_author.rs:259` — Frontmatter link and unlink writes bypass the bounded work-item store (security)
- 🔵 `cli/work-cli/src/sync.rs:2387` — `report_finished_run` re-resolves the scheme it already holds (code-quality)
- 🔵 `cli/work-cli/src/sync.rs:592` — Unclassifiable rows hard-code `"retryable"` (code-quality)
- 🔵 `cli/work-cli/src/sync.rs:2353` — `single_line(..)` applied by hand at every warning site (code-quality)
- 🔵 `cli/work-cli/src/sync.rs:744` — `discard_note` wildcard arm over the 10-variant `KeptEdits` (code-quality)
- 🔵 `cli/work-adapters/src/sync/apply.rs:1103` — Adopt-order preamble duplicated between `relink_steps` and `settle_order` (code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:2344` — `execute` builds the 19-field `RunReport` twice and swallows marker-removal failures (code-quality)
- 🔵 `cli/work-adapters/src/sync/report.rs:222` — `ApplyError` → `RunFailure` mapping repeated three times (code-quality)
- 🔵 `cli/work-adapters/src/sync/kept_edits.rs:71` — `KeptEdits` mixes review states and sweep outcomes in one recursive enum (code-quality)
- 🔵 `cli/work-cli/src/sync/work_files.rs:126` — VCS-status warning formats the failure line by hand, bypassing `RunFailure` (architecture)
- 🔵 `cli/work-adapters/src/sync/report.rs:25` — The new report module forms dependency cycles with `run` and `apply` (architecture)
- 🔵 `cli/work-cli/src/bounded_roots.rs:14` — Root validation used by every command depends on sync's report types (architecture)
- 🔵 `cli/work-adapters/src/sync/apply.rs:232` — Adapter-layer error types embed CLI flag syntax (architecture)
- 🔵 `cli/work-cli/src/sync/stores.rs:52` — The root-confining store wrapper lives in the binary, not the store layer (architecture)
- 🔵 `cli/work-cli/src/sync.rs:794` — `ids.dedup()` in the earlier-copies note survives mutation (test-coverage)
- 🔵 `CHANGELOG.md:157` — The Breaking entry covers `paths.integrations` only (compatibility)
- 🔵 `cli/migrate/src/migrations/m0011.rs:97` — Migration 0011 does not warn about the new 74 refusal (compatibility)
- 🔵 `docs-site/package.json:24` — The override forces zod 4 onto a dependency that declares zod 3, and `^4.6.6` is a range, not a pin (compatibility)
- 🔵 `cli/work-adapters/src/filesystem.rs:113` — Work-item listing follows symlinks that resolve outside the repository (security)
- 🔵 `cli/work-adapters/src/sync/kept_edits.rs:189` — `--discard-kept-edits` also removes `Unrecorded` folders, including tracked ones (safety)
- 🔵 `skills/work/sync-work-items/SKILL.md:356` — The kept-edits record's fields are named differently across surfaces (usability)
- 🔵 `skills/work/sync-work-items/SKILL.md:217` — Exit 1 mixes internal bugs with a problem the user can fix (usability)
- 🔵 `cli/work-cli/src/bounded_roots.rs:214` — Overlap refusals don't show the resolved paths that overlapped (usability)
- 🔵 `docs-site/src/content/docs/guides/sync-work-items.mdx:156` — The guide's completion criteria list fewer blocking groups than the skill applies (usability)
- 🔵 `cli/work-cli/src/sync.rs:798` — "1 earlier copies still kept" (0354) (usability)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/work-cli/src/bounded_roots.rs:372-380` — Case-insensitive root-overlap check is never exercised
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`Comparable::form_of` lowercases resolved roots when `ignores_case` reports a case-insensitive filesystem. No test exercises either the probe or the folding. If `folds_case` were hard-wired to `false`, or `ignores_case` always returned `false`, every test in `bounded_roots.rs` and `sync.rs` would still pass.

**Impact**: On macOS's default case-insensitive APFS, a regression here would let the two bounded stores alias each other's files with no failing test.

**Suggestion**: Add a test with two roots that differ only in case. Assert `RootRefusal::Overlapping` when `ignores_case(repository)` is true and no refusal when it is false. Also pin `ignores_case` itself against a known directory.

---

### `tests/unit/tasks/test_sync_work_items_skill_structure.py:54-68` — Skill-content test copies the operation vocabulary by hand and has already drifted (`adopt-record`)
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`RETRYABLE_OPERATIONS` (and likewise `FAILURE_CLASSES`, `UNIDENTIFIED_KINDS`, `REFUSAL_REASONS`) copies keywords defined in Rust by hand. `SyncOperation::AdoptRecord` (`adopt-record`) is a local step whose failure `sync_create.rs` asserts is `Retryable`. Yet neither this list nor the skill's **Retryable advice** table mentions it, and the test still passes.

**Impact**: An `error: <id>: adopt-record failed: …` line reaches the skill with no advice, and future operations will drift the same way unseen.

**Suggestion**: Derive the expected vocabulary from the source of truth, for example a golden emitted by a Rust test. Assert that every local operation appears in the advice table, and add the missing `adopt-record` row to `skills/work/sync-work-items/SKILL.md`.

---

### `cli/visualiser/server/src/watcher.rs:967-970` — OS watcher adapter now has no CI coverage on any platform
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

Every watcher and SSE test now feeds scripted events. The only `os_watcher` test is `#[ignore]`d and runs only in the opt-in `test:integration:visualiser-os-watcher` lane, which no CI leg runs. The scripted tests also emit uncanonicalised `tmp` paths, so no test now covers the macOS `/var` vs `/private/var` divergence.

**Impact**: A regression in the notify integration or in path normalisation would ship with every lane green.

**Suggestion**: Run the OS contract test un-ignored on Linux, where inotify is prompt (e.g. `cfg_attr(target_os = "macos", ignore)`). Add one scripted test that injects a canonicalised `/private/var`-style path under a non-canonical project root.

---

### `cli/work-cli/src/sync.rs:1883-1884` — `run_sync_with` is about 305 lines of repeated match-warn-return blocks
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`run_sync_with` keeps `#[allow(clippy::too_many_lines)]`. Most of its body repeats `match step { Ok(v) => v, Err(e) => { console.warn(e); return ExitCode::from(CODE); } }` about 15 times, with preflight, assembly and reporting interleaved.

**Impact**: Adding or reordering a preflight check means editing a 300-line function in which each failure's exit code sits far from its cause.

**Suggestion**: Introduce a `Refusal { message, code }` and a `fn prepare_sync(...) -> Result<PreparedSync, Refusal>` that uses `?` with `.map_err(Refusal::usage)`. `run_sync_with` then reads prepare → assemble → report, and the allow can go.

---

### `cli/work-adapters/src/sync/apply.rs:1284-1311` — Cross-module invariants enforced by `unreachable!` panics instead of types
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`settle_order` looks up the carrier a second time through a closure that panics (`unreachable!("the order was refused because {key} is carried")`), and panics again if `AlreadyWritten` arrives without a created marker. Both depend on `work::sync::order_precondition` in another crate. The same pattern recurs in `relink_steps` (`request.order` must be `Some`), in `adopted_key` (the order must be `Adopt`), and in run.rs's `apply_local_create` (an adoption-record path must exist alongside a marker path).

**Impact**: A harmless-looking change to `order_precondition` becomes a runtime panic in `work sync` rather than a compile error.

**Suggestion**: Make the types carry the facts:
- `OrderPrecondition::KeyCarried(ExternalId)` / `AlreadyWritten(ExternalId)`;
- a relink request holding `&PendingPushOrder`, not an `Option`;
- `adopt` receiving the key rather than the order;
- one helper that returns both the marker path and the record path.

---

### `cli/work-cli/src/bounded_roots.rs:84-98` — Undocumented break: an out-of-repository `paths.work` in the team config now refuses `work sync` with 74
**Severity**: major | **Confidence**: high | **Lens**: compatibility

🟡 **Compatibility**

`BoundedRoot::for_configured` does not care which key it is checking, and `ValidatedRoots::check` calls it for `RootKey::Work` as well as `RootKey::Integrations`. A team config with `paths.work: ../shared-work` that synced before this PR now fails every `work sync` with exit 74. Several surfaces describe the rule for `paths.integrations` only: the CHANGELOG, the PR notes, the skill's exit-74 row and `configure`. The `shapes_outside` tests also exercise only `RootKey::Integrations`.

**Impact**: Teams that keep work items outside the repository hit a 74 refusal after upgrading, with no warning in the CHANGELOG.

**Suggestion**: If this is intended, document it for `paths.work` everywhere and add a `RootKey::Work` case to `shapes_outside`. If it is not, restrict the provenance gate to `RootKey::Integrations`.

---

### `skills/work/sync-work-items/SKILL.md:249-250` — Skill promises a preview shows every failure, but pending-push refusals are invisible in a preview (0351)
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

The skill and the `work sync` help say every failure prints `error: … would fail` on a preview. Known gap 0351 shows that a create-from-local whose pending-push marker would be refused previews cleanly with exit 0, then fails `terminal` with exit 71 on the real run.

**Impact**: A user who previews first, as the skill tells them to, is told all is well and then hits a terminal failure that needs manual recovery.

**Suggestion**: Fix 0351 before release. Until then, qualify the claim here and in `cli.rs`, and tell the skill to treat an unsynced item with a marker under `pending-push/` as needing the pending-push recovery even when the preview is clean.

---

### `cli/work-adapters/src/sync/apply.rs:1133-1147` — Relink `create` clears the link before checking the order, so a refusal is misclassed and leaves the item changed
**Severity**: major | **Confidence**: medium | **Lens**: correctness

🟡 **Correctness**

For `PendingPushOrder::Create`, `relink_steps` runs `unlink_external_id` through `land_local` before `create_from_local_steps` → `settle_order`. Several outcomes can still create nothing: `AlreadyWritten`, `CreateUnlessIssueExists` finding the recorded issue, or a failed `locate`. A plausible trigger is a leftover `Created{K0}` marker. In each case `external_id` is already gone on disk, but `OrderRefused`/`Tracker` report `NothingLanded` through `ApplyError::progress()`. The row exits 75 or 70, and `dossiers_to_refresh` rewrites the dossier, which still says the item is linked, instead of flagging it stale.

**Impact**: A refused relink silently leaves the work file unlinked while reporting that nothing changed.

**Suggestion**: Evaluate `order_precondition` and any `locate` lookups before the unlink. Alternatively, carry the attempt's progress on every `ApplyError` raised inside an attempt, so a refusal after a landing is `MayHaveLanded`.

---

### `cli/work-adapters/src/sync/apply.rs:940` — Untracked files with a matching recipe-1 baseline are overwritten without a copy, though recipe 1 ignores formatting
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`captured_by_last_sync` returns true for an untracked file whenever its recipe-1 fingerprints match the baseline, and the pull then overwrites the file without keeping a copy. The recipe-1 body fingerprint is `canonical_hash(dialect.as_pushed(body))`, which records what the tracker would store, not the bytes on disk. Formatting changes, and content the dialect drops on push (HTML comments, unsupported markup), leave it unchanged.

**Impact**: Untracked items, often new ones synced before they are committed, have no VCS history. Such edits are lost permanently when a remote change is pulled over them.

**Suggestion**: Let a baseline vouch for an untracked file only on a byte-exact match, for example against the legacy `digest::local` raw digest. Add a test where a formatting-only local edit must produce a `kept-edits` record.

---

### `cli/work-cli/src/sync.rs:2235-2245` — Bounded-store boundary covers only two of the run's writers
**Severity**: major | **Confidence**: medium | **Lens**: architecture

🟡 **Architecture**

`assemble_run` builds the two validated `SyncStores`. In the same run, `project_store` (rooted at the whole repository), `state_store` and `integrations_store` are still plain `FileCorpusStore::new(...)` over unvalidated roots. Retirement ports and promotion records write through those stores. `for_identity_change` checks only the state root, and `SyncAuthor::rewrite_frontmatter` writes through `FileCorpusStore::new(work_dir)` (security lens).

**Impact**: The claim that every write in a sync goes through a bounded store holds only for the apply path. Future writers will copy the nearest pattern.

**Suggestion**: Build the retirement, promotion-record and frontmatter-link stores from `ValidatedRoots`. Alternatively, name the deliberate exceptions in a type, for example `RetirementStores`, and say why they sit outside the bound.

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound. Domain logic stays I/O-free in `work`, new concerns are split into cohesive adapter modules, and the CLI reaches I/O through injected seams. The main gap is that the bounded-store boundary covers only the two new sync stores. There is also some coupling between modules, and some formatting is done by hand rather than through `RunFailure`.

**Strengths**: the `work::inventory` pure core; `ItemAttempt`; one failure vocabulary through `failures()`; the `SyncFiles`/`ApplierPorts` grouping; the `run_sync_with` + `SyncSeams` + `StoreFactories` + `Console` testable shell; `DossierAuthority`/`ConflictStatus` separating policy from mechanics; the visualiser `run_with` seam.

**Comments**:
- `cli/work-cli/src/sync.rs:2235-2245` (major, medium) — The bounded-store boundary covers only two of the run's writers. See the inline comments.
- `cli/work-cli/src/sync/work_files.rs:126-131` (minor, high) — `VcsConflicts::read` formats `"warning: -: identify-local failed: ..."` by hand, bypassing `failure_line`/`SyncOperation::IdentifyLocal`. Return a `RunFailure` instead.
- `cli/work-adapters/src/sync/report.rs:25-27` (minor, medium) — `report` imports `ConflictDossier`/`DiscoveryStatus`/`RunMode` from `run`, while `run` re-exports report types. Similar cycles exist between report/apply, apply/attempt and dossiers/report/run. Move the shared value types into a leaf `sync::outcome` module and drop the `pub use` re-exports.
- `cli/work-cli/src/bounded_roots.rs:14-18` (minor, medium) — Root validation used by `create`/`promote` depends on `sync::apply::FailureClass`, `sync::report::RunFailure` and `recovery_label::PARENT`. Introduce a neutral failure-line type.
- `cli/work-adapters/src/sync/apply.rs:232-235` (minor, low) — `OrderRefusal` `Display` and `ApplyError::cause()` hard-code CLI remedies (`--resolve`, `accelerator work promote`). Keep the refusal structured in the adapter and render the remedy in work-cli.
- `cli/work-cli/src/sync/stores.rs:52-57` (suggestion, medium) — `RootedStore` confinement is private to `work-cli::sync`. Promote it to `corpus-adapters`, e.g. `FileCorpusStore::confined_to(root, bound)`.

**General Findings**:
- (minor) `sync.rs`, `run.rs` and `apply.rs` keep taking on responsibilities. Extract render, dossier-file and pending-push-order modules.

### Correctness

**Summary**: The pull path is well built. It takes the lock, reads, checks, keeps a copy, hashes, re-reads, re-checks and writes, all under one `ForFile` lock. Classification through `ItemAttempt` is sound for the main create and pull flows, and the containment, inventory and dossier logic check out. The gaps are in the newer order and relink paths, where `ApplyError::progress()` works out landing from the variant rather than from the attempt.

**Strengths**: the lock is released through Drop on every path; the copy decision and hash are made before the write and re-checked just before it; `land_local` separates `FailedAfterPublish`; `unresolved_tail`; `DossierAuthority` scoping with the inventory refreshed after identity settlement; `blocking_fault()` precedence; the kept-edits sweep re-checks under the lock.

**Comments**:
- `cli/work-adapters/src/sync/apply.rs:1133-1147` (major, medium) — Relink `create` clears the link before checking the order. See the inline comments.
- `cli/work-adapters/src/sync/apply.rs:1402-1407` (minor, medium) — `record_adoption` runs through `observe_local` but persists the baseline removal and the adoption record. A later `link_external_id` failure is therefore classed retryable (70). Run it through `land_local`, or remove the baseline only after the link lands.
- `cli/work-adapters/src/sync/apply.rs:1051-1053` (minor, medium) — `author_from_remote`, `link_external_id` and `unlink_external_id` map `kernel::Error` to `StepError::Failed`, losing `NotDurable`. An authored file can exist on disk while the run reports retryable. Keep `NotDurable` through the `LocalAuthor` port.
- `cli/work-adapters/src/sync/apply.rs:873-884` (minor, low) — The re-read accepts either `classified_content` or `content`. If the first read saw the pulled content and the file then reverted, the write proceeds with no copy. Require `second == first`.
- `cli/migrate/src/migrations/m0011.rs:105` (minor, high) — `moved_integrations_root` drops any override under `.accelerator/state`, but `STATE_RULES` only cover `state/integrations/*/…`. `paths.integrations: .accelerator/state/trackers` gets no ignore rules. Skip only an exact `.accelerator/state/integrations`.

**General Findings**:
- (minor) `ApplyError::progress()` is derived from the variant, not from the attempt. Carry the attempt's progress on every `ApplyError` so classification is total by construction.

### Code Quality

**Summary**: The new building blocks are well modelled (`ItemAttempt`, `PublishError`/`StepError`, the `ApplyError` accessors, `TsvRecord`/`Console`, `BoundedRoot`, `CorpusInventory`). Three things hurt maintainability:
- the orchestration functions are long: `run_sync_with` is about 305 lines and `execute` about 180;
- invariants are enforced by `unreachable!` panics rather than by the types;
- several pieces are duplicated or prone to drift.

**Strengths**: `ItemAttempt` `observe_*`/`land_*`; the `ApplyError` accessors feed one `Display`; `SyncOperation` and `FailureClass::keyword()` replace strings; `pull_steps` reads as the domain procedure; `Console`/`TsvRecord` centralise escaping; the root rules are explicit domain types; comments are mostly about why, with no references to work items.

**Comments**:
- `cli/work-cli/src/sync.rs:1883-1884` (major, high) — `run_sync_with` is about 305 lines of match-warn-return. See the inline comments.
- `cli/work-adapters/src/sync/apply.rs:1284-1311` (major, high) — Invariants enforced by `unreachable!`. See the inline comments.
- `cli/work-cli/src/sync.rs:2387-2403` (minor, high) — `report_finished_run` re-resolves a scheme the context already carries, so its Err arm can never run. Pass `context.scheme` and drop `config` from `FinishedRunContext`.
- `cli/work-cli/src/sync.rs:592` (minor, high) — Unclassifiable rows hard-code `"retryable"` while `failures()` classes them separately. Use one source of truth.
- `cli/work-cli/src/sync.rs:2353-2366` (minor, medium) — `single_line(&x.display().to_string())` is applied by hand at every warning site. Add a `Shown<T>` wrapper, or give `Console::warn` structured records.
- `cli/work-cli/src/sync.rs:744-762` (minor, medium) — The `discard_note` wildcard over the 10-variant `KeptEdits` defeats exhaustiveness checking. Match `NothingToDiscard` explicitly.
- `cli/work-adapters/src/sync/apply.rs:1103-1116` (minor, high) — The adopt-order preamble (the `refused` closure and the `CorpusIncomplete` deferral) is duplicated between `relink_steps` and `settle_order`. Extract helpers.
- `cli/work-adapters/src/sync/run.rs:2344-2369` (minor, high) — `execute` builds the 19-field `RunReport` twice. `let _ = remove(marker)` and `retire_dossiers` swallow failures that sibling paths report.
- `cli/work-adapters/src/sync/report.rs:222-259` (minor, high) — The `ApplyError` → `RunFailure` mapping is repeated three times. Add `impl From<&ApplyError> for RunFailure`.
- `cli/work-adapters/src/sync/kept_edits.rs:71-87` (minor, medium) — `KeptEdits` mixes review, sweep and request stages, with recursive `Box<Self>` variants. Split it into `ReviewedCopy` + `DiscardOutcome`.

**General Findings**:
- (major) `sync.rs` is about 9.2k lines (2.46k production, about 6.7k inline tests). Split out report lines, dossiers, orders and tests.
- (minor) Restating doc comments on private helpers; the misleading `Contents` comment; `utc_date` should be renamed `rfc3339_instant`.
- (minor) Mixed I/O channels: bounded stores for writes, raw `std::fs` for reads and listing in the same flows.

### Test Coverage

**Summary**: Coverage is unusually thorough. The TOCTOU safeguards, landing classes, `NotDurable`, pending-push refusals, the kept-copy lifecycle, dossier criteria, inventory, root refusals and order parsing all have direct behavioural tests. Mutation probing found three real gaps: case folding in the overlap check, a skill-vocabulary test that has already drifted, and an OS watcher adapter that no CI leg runs.

**Strengths**: the TOCTOU hooks at both recheck points; tests of `ItemAttempt` at two levels; the totality tests; conflict-marker edge cases; the permission-test guards fail loudly as root; every root refusal is tested end to end and asserts no tracker call; scripted visualiser events remove flakiness.

**Comments**:
- `cli/work-cli/src/bounded_roots.rs:372-380` (major, high) — The case-insensitive overlap check is never exercised. See the inline comments.
- `tests/unit/tasks/test_sync_work_items_skill_structure.py:54-68` (major, high) — The hand-copied vocabulary has drifted (`adopt-record`). See the inline comments.
- `cli/visualiser/server/src/watcher.rs:967-970` (major, high) — The OS watcher has no CI coverage. See the inline comments.
- `cli/work-cli/src/sync.rs:794-796` (minor, high) — `ids.dedup()` in `earlier_copies_note` survives mutation. Add a case with two copies of the same id.

**General Findings**:
- (suggestion) The skill-content assertions pin exact prose. Narrow them to the tokens that matter.
- (minor) The post-validation fixes were tested only against `RecordingTracker`. Re-run against a real client before release.

### Safety

**Summary**: The pull path is well guarded, and the sweep and dossier retirement keep data when in doubt. Three narrow risks remain: the untracked copy-skip trusts a formatting-insensitive hash, the kept copy's folder entries are not flushed before the overwrite, and the retirement sweep deletes a folder whose copy cannot be read.

**Strengths**: lock → check → copy → recheck → atomic write, refusing when no copy can be kept; the copy is skipped only when edits are provably safe; `create_new` + `sync_all` copies and the RESTORE-PENDING record written through `atomic_write`; the sweep removes only `Equal` or bookkeeping-only folders and re-checks under the lock; `DossierAuthority` safe defaults; link-safe recovery-store containment; migration 0011 only appends rules and warns about untracking; the skill confirms every destructive action.

**Comments**:
- `cli/work-adapters/src/sync/apply.rs:940` (major, medium) — The untracked recipe-1 match skips the copy. See the inline comments.
- `cli/work-adapters/src/sync/apply.rs:914-926` (minor, medium) — `write_durably` fsyncs the copy file but not the folders created for it. A crash can leave the overwrite on disk without the copy. Fsync each folder up to the recovery directory.
- `cli/work-adapters/src/retirement.rs:565-567` (minor, medium) — Retirement's `copy_settled` treats `CopyUnreadable` as settled and removes the folder. `KeptEditsRetention` deliberately does not. Settle only on `Equal`/`CopyMissing`.
- `cli/work-adapters/src/sync/kept_edits.rs:189-204` (suggestion, low) — `named_for_discard` also discards `Unrecorded` folders, including tracked ones. Exclude them, or label them clearly in the preview.

**General Findings**:
- (minor) The conflicts-folder cleanup removes any `TEMP_PREFIX` file on apply runs. Confirm a run-level lock prevents deleting another run's temp file.

### Security

**Summary**: Path containment is substantially stronger. There are no critical or major vulnerabilities. Three defence-in-depth gaps remain: frontmatter link writes go through an unbounded store, a tracked adoption record can be rewritten as a trusted untracked one, and symlinked work items are read from outside the repository.

**Strengths**: `unresolved_tail`; the per-component link refusal in the recovery store, with link-safe `remove_dir`; paths built from ids and keys are checked to be plain components; tracked or foreign markers, records and kept-edits folders are distrusted; root trust is tied to where the value came from; `TsvRecord`/`single_line`/`printable` prevent forged records; the tokio RUSTSEC bump.

**Comments**:
- `cli/work-cli/src/sync_author.rs:259` (minor, medium) — `rewrite_frontmatter` uses `FileCorpusStore::new(work_dir)`, so a work directory swapped for an outbound symlink after validation is still written. Use `within_project(work_dir, repository)`.
- `cli/work-adapters/src/sync/adoption_records.rs:233-262` (minor, medium) — `follow()`/`rename()` read without the tracking check and republish the record untracked, turning a distrusted record into a trusted one. Gate `read()` on `Untracked`.
- `cli/work-adapters/src/filesystem.rs:113-120` (suggestion, low) — Listing follows symlinked `*.md` files outside the repository, and their contents could be pushed to the tracker. Report them as unidentified or unreadable, or read through `store::read_within`.

**General Findings**:
- (suggestion) The check-to-use windows that remain can only be raced by the same user and are adequately mitigated.

### Compatibility

**Summary**: The `work sync` contract changes in many ways, and almost all of them are documented, with every in-plugin consumer updated. The Rust API breaks are confined to `publish = false` crates and pinned by snapshots. One break is undocumented: an out-of-repository `paths.work` in the team config now exits 74.

**Strengths**: the CHANGELOG Breaking and Migrations sections; consumer skills updated in step; `#`-prefixed records pinned by goldens; the precedence change is to the docs only; migration 0011 and `config init` are idempotent; the Rust API breaks stay internal; the dependency bumps are low risk.

**Comments**:
- `cli/work-cli/src/bounded_roots.rs:84-98` (major, high) — The `paths.work` outside-root refusal is undocumented. See the inline comments.
- `CHANGELOG.md:157-162` (minor, high) — The Breaking entry names only `paths.integrations`. Cover both keys and mirror the change in Migrations.
- `cli/migrate/src/migrations/m0011.rs:97-106` (suggestion, medium) — `moved_integrations_root` silently skips an outside path. Print a `note: 0011:` warning about the 74 refusal when the value comes from team or tracked config.
- `docs-site/package.json:24` (minor, medium) — The override forces zod 4 onto a zod-3 dependency, and `^4.6.6` is a range, not the stated pin to 4.6.7. Pin it exactly and track removing the override.

**General Findings**:
- (minor) `sync-work-items` (about line 557) still describes `--resolve` warn-and-skip. Reword it to say exit 2, and regenerate the reference page.

### Usability

**Summary**: Error handling is much better: one `error:` line shape, root refusals that say how to fix the problem, and kept-edits notes that give the exact command. The main risks:
- the skill asks the LLM to interpret a lot;
- `--discard-kept-edits` runs a full sync unless `--target` is also given;
- the known gaps 0351 and 0352 undercut what the skill promises.

**Strengths**: one stderr line shape; root refusals that say how to fix the problem; kept-edits notes with the exact command and a preview; `--resolve` typos now exit 2; `local-save` relays the cause and de-duplicates it in a batch; explicit `--resolve` orders are documented across help, skill, guide and CHANGELOG.

**Comments**:
- `skills/work/sync-work-items/SKILL.md:249-250` (major, high) — A preview does not show pending-push refusals (0351). See the inline comments.
- `skills/work/sync-work-items/SKILL.md:437` (major, medium) — There is no rule for the duplicate `noop\tunsynced` row beside a failed create (0352). Add an interim rule to ignore it when the same id has a `failed`/`create-from-local`/`adopt` row.
- `skills/work/sync-work-items-recovery/SKILL.md:21-34` (major, medium) — Terminal recovery is keyed on an `after <step>:` prefix parsed from free text. Emit the landing step as a structured token, e.g. `#\tlanded\t<id>\t<step>[\t<KEY>]`.
- `cli/work-cli/src/cli.rs:418-422` (major, medium) — `--discard-kept-edits <id>` without `--target` runs a full bidirectional sync. Treat the discard ids as targets, refuse with exit 2, or add a dedicated subcommand.
- `skills/work/sync-work-items/SKILL.md:356-359` (minor, high) — The kept-edits field names differ across the help, the skill and the CHANGELOG, and the "sixth field" wording is ambiguous.
- `skills/work/sync-work-items/SKILL.md:217` (minor, medium) — Exit 1 conflates internal bugs with a record that cannot be written. Make sure the latter always prints an `error:` line, and consider moving it to 70.
- `cli/work-cli/src/bounded_roots.rs:214-218` (minor, medium) — The `Overlapping`/`ContainsState` refusals omit the resolved paths. Include both.
- `docs-site/src/content/docs/guides/sync-work-items.mdx:156-167` (minor, medium) — The guide lists fewer blocking groups than the skill applies, and omits `adopted`.
- `cli/work-cli/src/sync.rs:798` (suggestion, high) — "1 earlier copies" (0354). This is a few-line fix.

**General Findings**:
- (major) The skill asks the LLM to interpret heavily. Move the group and blocks-completion fields into the CLI, and remove restatements.
- (minor) The exit code no longer signals failure reliably, and `#\tsummary\tsynced\t<N>` is undocumented (0353).
