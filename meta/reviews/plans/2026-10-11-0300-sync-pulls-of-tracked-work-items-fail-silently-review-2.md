---
type: "plan-review"
id: "2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-review-2"
title: "Plan Review: Sync Pulls of Tracked Work Items Fail Silently Implementation Plan"
date: "2026-10-19T19:31:10+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently"
relates_to: ["plan-review:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-review-1"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "security", "compatibility", "usability"]
review_number: 2
review_pass: 6
tags: ["sync", "work-cli", "work-adapters", "store", "sync-work-items"]
last_updated: "2026-10-21T19:55:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Sync Pulls of Tracked Work Items Fail Silently Implementation Plan

**Verdict:** REVISE

The 0296 rebase held up. Every lens confirmed the core design, and nearly every code reference checked out on this branch: two bounded stores, safeguards before pulls go live, the landing-based failure class, and dossier retirement as an allow-list. The 14 majors (after deduplication) cluster in three places:

- **Phase 5's `ItemAttempt` mechanics**: refusal routing, the kept-copy hand-off, and operation derivation.
- **Rules that newly bite real configurations**: repository-bounded roots, malformed files dropping out of the duplicate-import guard, and 7-day expiry of unmerged edits.
- **Documentation and skill text** that still describes 0296-era semantics.

Review 1 ended APPROVE on the pre-0296 revision. None of its findings recur as written. The `CorpusScope`-beside-`RunReport` point that review 1 declined is raised again, now by two lenses.

### Cross-Cutting Themes

- **Exit 70 is still documented as "safe to retry"** (flagged by: usability, compatibility). The plan rewrites only the 75 paragraph of `exit_codes.rs`, the authoritative taxonomy. The 70/71 paragraphs ("provably before any remote mutation", "a read never emits 71") and the precedence line (missing `1`) become false. The skill's 70 row still says "Re-run; it is safe to retry", although 70 now covers permission and malformed-file failures that a blind retry repeats.
- **Repository-bounded sync stores break out-of-repo roots** (flagged by: correctness, compatibility). `within_project(root, repository)` refuses an absolute `paths.work`/`paths.integrations` outside the repository, which `configure.md` says is supported. It also refuses a `meta/work` symlinked out of the repository. `work create`/`update` keep writing to both. Every baseline, marker, dossier and pull write would be refused as `UnsafePath`.
- **Run scope travels beside the report** (flagged by: architecture, code-quality). `CorpusScope` and preview-ness are passed separately to `failures`, `exit_code_for_report`, `failure_lines` and `DossierAuthority::for_run`, so a mismatched argument silently changes exit codes or retirement.
- **`ApplyError::Tracker.progress` is never read** (flagged by: architecture, code-quality). `class()` ignores it, and `mutate_tracker` carries special-case logic for no reader. The Desired End State claims tracker failures follow the landing rule, but they do not.
- **`conflict_status_of` is per-item policy placed on `RunReport`** (flagged by: architecture, code-quality). It depends only on `ReportedItem`, and `build_dossiers` must call it inside `execute` before any report exists.
- **Conflict-marker detection is too loose and too thinly tested** (flagged by: correctness, test-coverage). A single line-start `<<<<<<< ` anywhere, including inside a fenced example, makes a healthy item permanently `malformed`, which fails every full run with exit 70.
- **Marker-path safety rests on one check, landing a phase late** (flagged by: security, safety). `pending-push/../last-sync.json` resolves inside the root, so only Phase 5's token-safety refusal stops it. Phase 4 already routes markers through the store and claims the guarantee.

### Tradeoff Analysis

- **Safety vs noise**: safety wants unmerged pull copies kept until settled or deleted by the user, with recurring `kept-edits-earlier` records as the nag. Usability flags those same recurring notes as noise that trains people to skim stderr. Recommendation: keep copies until settled (data loss is irreversible), keep the stdout records, and emit the stderr note only when the set of kept copies changes.
- **Retryable vs needs-repair for broken files**: classing malformed/unreadable files as 70 keeps "nothing landed" consistent, but usability points out they need a fix first, unlike 0296's `unparseable` (75). Recommendation: keep the exit code, which is governed by the landing rule, but group them under `needs-attention` (or a `needs-repair` group) in the skill so the summary never suggests a bare retry.

### Findings

#### Major

- 🟡 **Usability + Compatibility**: Exit 70/71 definitions in `exit_codes.rs` and the skill contradict the landing rule
  **Location**: Phase 5 §4 and §8; Phase 7 §1
  Only the 75 paragraph is updated. The 70 text still promises a safe retry and "a read never emits 71", and the precedence line omits `1`.
- 🟡 **Correctness + Compatibility**: Repository-bounded stores refuse supported out-of-repo work/integrations roots
  **Location**: Phase 4 §5–§6
  Absolute `paths.*` values and symlinked work directories outside the repository stop syncing while other commands keep working. The plan has no changelog entry for this and no decision on it.
- 🟡 **Correctness**: Malformed or unreadable files leave the corpus that guards create-from-remote
  **Location**: Phase 5 §5
  A jj-conflicted tracked item drops out of `request.corpus`, so its `external_id` is no longer carried, and `create_from_remote` can author a duplicate.
- 🟡 **Safety**: 7-day expiry permanently deletes unmerged local edits without confirmation
  **Location**: Phase 3 §6
  This diverges from retirement's keep-until-settled rule. It runs on any apply, unrelated `--target` runs included, and `expired-edits` is non-blocking in the skill.
- 🟡 **Security**: Repository-controlled paths spliced into the skill's Bash commands without quoting rules
  **Location**: Phase 7 §1 (paths rule; recovery skill)
  A filename slug such as `0001-$(…).md` can reach `accelerator work show/update <path>` or `rm`. Only the row id is constrained.
- 🟡 **Code Quality**: `ItemAttempt`'s `E: Display` wrappers re-wrap refusal variants as `Io`
  **Location**: Phase 5 §4
  `attempt.mutate_local(WriteLocal, || self.write_item(request))` turns `LocalWriteRefused` into `Io write-local` and loses its class. The same applies to `lock_item`, `read_local` and `keep_recovery_unless_synced`.
- 🟡 **Code Quality**: The `?`-driven pull body cannot return `Attempted<T> { result, kept_copy }`
  **Location**: Phase 5 §4
  Early return drops the `ItemAttempt` holding `kept_copy`, which loses the kept-edits report on exactly the failure paths that need it.
- 🟡 **Code Quality**: `ApplyError::operation()` cannot recover the refused action, and `ApplyOperation` lacks `recheck-local`/`pull`
  **Location**: Phase 5 §4
  `RemoteUnrenderable` is raised from create, pull and push/retitle sites but carries only an id.
- 🟡 **Architecture**: The new `FinishedRun` collides with `finaliser::FinishedRun`
  **Location**: Phase 3 §6
  `work-cli/src/finaliser.rs:22` already defines `FinishedRun<'a>` as the `RunFinaliser` input, and the plan never mentions it.
- 🟡 **Architecture + Code Quality**: Run scope and mode passed beside `RunReport` instead of carried by it
  **Location**: Phase 5 §5/§7; Phase 6 §4
  Three sources for the same run-level facts, and four call sites that must agree.
- 🟡 **Usability**: The moved 0296 paragraph misstates what a retryable pull or `failed\tsynced` row means
  **Location**: Phase 7 §1
  "The local file changed mid-run" stops being true once `lock-local`, `keep-recovery`, `hash-local` and permission writes are retryable.
- 🟡 **Usability**: Broken frontmatter is 70/`needs-retry` or 75/`needs-attention` depending on where it is detected
  **Location**: Phase 5 §5
  An unclosed fence is `malformed`, while a closed fence over bad YAML is `unparseable`. The same user mistake gets two different reports.
- 🟡 **Test Coverage**: The run-level computation of `local_unchanged_since_sync` is never tested
  **Location**: Phase 3 §2, §5
  Applier tests inject the bool, so an always-`true` mutation survives and silently skips the copy for committed edits resolved `=remote`.
- 🟡 **Test Coverage**: The terminal-baseline re-run test cannot detect a rewrite and accepts two outcomes
  **Location**: Phase 4 §1
  An identical-bytes re-pull is indistinguishable from `record-baseline`, and "(or `synced`)" loosens it further.

#### Minor

- 🔵 **Architecture + Code Quality**: `Tracker.progress` computed but never read by `class()`
  **Location**: Phase 5 §4
  Drop it, or make `class()` use it and say so.
- 🔵 **Architecture + Code Quality**: `conflict_status_of` belongs on `ReportedItem` or in `dossiers.rs`, not on `RunReport`
  **Location**: Phase 6 §4
  `build_dossiers` runs inside `execute`, where `RunKnowledge` lives, before the report exists. `ConflictShape` is unlisted.
- 🔵 **Correctness + Test Coverage**: Conflict-marker detection fires on any single line-start marker, including fenced examples
  **Location**: Phase 5 §5
  Require a complete marker block outside code fences, and make the test table-driven (git and jj markers, out-of-block, fenced).
- 🔵 **Security + Safety**: Marker token-safety refusal lands in Phase 5, after Phase 4 routes markers through the store
  **Location**: Phase 4 §4; Phase 5 §3
  Move the refusal into Phase 4. Optionally, make sync stores refuse any non-`Normal` component even when it resolves inside the root.
- 🔵 **Security**: Recovery-state entries are followed through symlinks, and a planted copy can be restored and pushed
  **Location**: Phase 3 §6; Phase 7 copy-over prompt
  Use `symlink_metadata`, treat symlinked entries as `Unrecorded`, and bound the recovery root like `within_project`.
- 🔵 **Security**: Stderr `printable` keeps `\n`, so paths or tracker text can forge `error:`/`note:` lines
  **Location**: Phase 2 §2; Phase 5 §7
  Pass every `RunFailure` subject/cause and note field through `single_line`.
- 🔵 **Safety**: The copy skip for VCS-ignored files rests on a normalised recipe-1 hash
  **Location**: Phase 3 §5
  Distinguish `Ignored` from `Clean`, or compare an exact-content digest.
- 🔵 **Correctness**: "Nothing landed" is false when `atomic_write` fails in `sync_parent_dir` after the rename
  **Location**: Phase 5 §3–§4
  An `Attempted` marker reported as retryable is followed by a terminal `prior-attempt-unknown` refusal.
- 🔵 **Correctness**: Failure handling for marker removal through the store is unspecified
  **Location**: Phase 4 §4
  A failed `Attempted` removal in `abandon_attempt` turns a retryable failure into the next run's terminal refusal.
- 🔵 **Correctness**: Unclassifiable rows suppressed by `settled_ids` when an identity row exists
  **Location**: Phase 3 §7 vs Phase 5 §6
  Apply Phase 5's "failed rows always render" rule to them.
- 🔵 **Correctness**: The sweep acts on a stale review, and unrecorded directories have no lock target
  **Location**: Phase 3 §6
  Re-compare under the lock, and specify lock-free expiry for `Unrecorded`.
- 🔵 **Correctness**: Whether `--resolve <id>=skip` records a resolution is unspecified
  **Location**: Phase 6 §2
- 🔵 **Architecture**: The recovery-directory name is written in `work-cli` and parsed in `work-adapters`
  **Location**: Phase 3 §6
  Make `RecoveryLabel` own both `format` and `parse`.
- 🔵 **Architecture**: Public `with_resolution` does not "enforce" the Prompt-only invariant
  **Location**: Implementation Approach; Phase 6 §2
  Make it `pub(crate)`, or provide `PlannedAction::resolved(..)` that refuses `Skip`.
- 🔵 **Code Quality**: `ItemAttempt::check` breaks the observe/mutate naming grid
  **Location**: Phase 5 §4
- 🔵 **Code Quality**: Doc comments the plan makes false are not scheduled for rewrite
  **Location**: Phases 3, 5, 6
  `prepare_conflicts_dir`, `persist_conflict_dossiers`, `local_write_error` and `pull`.
- 🔵 **Code Quality**: Three vocabularies for one concept (recovery copy, kept edits, restore-pending)
  **Location**: Phase 3 §6
- 🔵 **Code Quality**: `render_report` accretes four record families without a stated decomposition
  **Location**: Phases 3, 5, 6
- 🔵 **Compatibility**: The changelog claims all "not writable" errors carry the OS reason, but config-adapters and the visualiser drop it
  **Location**: Phase 1 §3; Phase 7 §5
- 🔵 **Compatibility**: Skill dispatch models promotion reasons as `failed` rows, but they are `not-promoted` rows
  **Location**: Phase 7 §1
  `#\tdiscovery` and `#\tnote` also have no entry.
- 🔵 **Compatibility**: `TsvRecord` changes `work promote`/`create-batch` detail output, and the changelog does not say so
  **Location**: Phase 3 §6; Phase 7 §5
- 🔵 **Usability**: CLI users lose the pending-push remedy, and the guide offers no replacement
  **Location**: Phase 5 §3; Phase 7 §3
- 🔵 **Usability**: Preview prints past-tense `error: … failed:` for operations never attempted
  **Location**: Phase 5 §6–§7
- 🔵 **Usability**: Recurring `no-id` and `kept-edits-earlier` stderr notes add noise
  **Location**: Phase 3 §6; Phase 5 §5
- 🔵 **Usability**: The `#\tdetail` record is overloaded with two field layouts
  **Location**: Phase 3 §6; Phase 7 §1
- 🔵 **Usability**: Unclassifiable items always report operation `read-local`
  **Location**: Phase 3 §7
- 🔵 **Test Coverage**: `a_read_only_work_item_file_fails_the_write_as_retryable` needs a `Faults` mode that does not exist, and its name overstates the test
  **Location**: Phase 5 §2, §6
- 🔵 **Test Coverage**: Mixed conflict-plus-failure runs are tested for exit code only, not for both rows
  **Location**: Phase 5 §6
- 🔵 **Test Coverage**: The sweep's lock-held-means-kept rule has no test
  **Location**: Phase 3 §6
- 🔵 **Test Coverage**: Several store doubles needing `FileRemove` are unlisted
  **Location**: Phase 4 §7
  `sync_run_real_client.rs` `Spy`, `sync_recipe_migration.rs` `Spy` and `tests/support/repo.rs` `Store`.
- 🔵 **Test Coverage**: The root-guard helper has no home before Phase 7, and permissions are restored inline instead of by a drop guard
  **Location**: Testing Strategy; Phase 1 §2; Phase 7 §4

#### Suggestions

- 🔵 **Architecture**: Group the local file ports as `SyncFiles`, mirroring `RetirementFiles`
  **Location**: Implementation Approach
- 🔵 **Architecture**: Split `Console` and the store factories out of `work-cli/src/sync.rs`
  **Location**: Phases 2–6
- 🔵 **Architecture**: Stale `create_from_remote` range (actually `apply.rs:427-463`), and `IdentityWorkspace` needs a `locks()` accessor
  **Location**: Current State Analysis; Phase 3 §7
- 🔵 **Code Quality**: Four types to build two stores, and `SyncStores`' fields are unstated
  **Location**: Phase 4 §6
- 🔵 **Compatibility**: Note that removing `RecoveryCopies::copy_settled` is an intentional pinned-API break
  **Location**: Phase 3 §6
- 🔵 **Safety**: The restore-pending record is not durable, so a crash leaves an `Unrecorded` directory
  **Location**: Phase 3 §6
- 🔵 **Correctness**: Between 3a and 4, refused pulls still create copies that nothing sweeps
  **Location**: Phase 3
- 🔵 **Security**: Accept a row id for path derivation only when it matches the canonical token shape, matched as a literal prefix
  **Location**: Phase 7 §1
- 🔵 **Usability**: Use `accelerator work link-external-id` rather than `work update --set external_id=`
  **Location**: Phase 7 §1 recovery skill
- 🔵 **Test Coverage**: Two Phase 1 "red" tests are characterisation tests, and permitted-root `..` behaviour is unpinned
  **Location**: Phase 1 §1
- 🔵 **Test Coverage**: The hash-fault mechanism and the preview-resolve exit code are unstated
  **Location**: Phase 3 §2; Phase 6 §5

### Strengths

- ✅ Phase ordering never ships an unguarded pull. Phase 3 adds lock, double recheck, recovery copy and hash-before-write while production pulls are still refused, and Phase 4 makes them land.
- ✅ The Phase 1 containment fix sits in the shared canonicaliser, refuses rather than lexically resolves `..` in a missing tail, and audits every consumer.
- ✅ The two sync stores are confined to their own roots and to the repository and obtained only through `StoreFactories`, so "every write lands under one of two roots" becomes structural rather than convention.
- ✅ Functional core, imperative shell throughout: `identify`, `PullRecoveryRetention::review`, `may_retire` and `RunReport::failures` are pure over ports, and ADR-0069's crate edges hold.
- ✅ Vocabularies have one spelling: `FailureClass::keyword`, `ApplyOperation`, `RefusalReason::token` and `TsvRecord`.
- ✅ Dossier retirement is an allow-list (`Settled`/`NotInConflict` only), impossible tuples map to `Unobserved`, and orphan retirement is gated on `proves_absence()`.
- ✅ Every work-item acceptance criterion maps to a named test, and the regression test runs real stores rooted as in production.
- ✅ The skill gains one dispatch section with quoting, paths and confirmation rules, plus a checkable completion rule.
- ✅ Exit-code changes are folded into 0296's changelog bullet rather than a parallel entry, and public-API diffs are listed for each pinned crate.
- ✅ The plan's claims about current code check out on the rebased branch: 13 `ItemApplier::new` call sites, 36 print sites, 6 `ItemDigests` implementors, the `PlannedAction` literal locations and the `SyncPorts` shape.

### Recommended Changes

1. **Rewrite the 70/71 contract everywhere** (addresses: exit 70 "safe to retry"; moved 0296 paragraph). Add `exit_codes.rs`'s 70/71 paragraphs and precedence line to Phase 5. Define 70 as "nothing for the item landed; re-run once the stderr cause is fixed" and 71 as "something may have landed". Mirror this in `--help` and the skill's 70 row. Replace the moved 0296 state-based paragraph with the `recheck-local` advice row, and assert in the skill test that the old sentences are gone.
2. **Decide the out-of-repo root policy** (addresses: repository-bounded stores). Either bound by the repository only when the root lies inside it, refusing only a symlinked root, or refuse out-of-repo roots as a documented breaking change. Add a red test for an absolute out-of-repo `paths.integrations`.
3. **Keep malformed files in the duplicate-import guard** (addresses: duplicate imports). Gate `create_from_remote` and targeted remote-only pulls on `inventory.proves_absence()`, or carry any parseable `external_id` from malformed files. Add an end-to-end test where a conflicted tracked item is not re-imported.
4. **Keep unsettled pull copies until settled** (addresses: expiry deletes unmerged edits; noise). Drop time-based expiry for restore-pending copies, matching retirement. Keep the recurring stdout records, and emit stderr notes only when the kept set changes.
5. **Specify `ItemAttempt`'s error and hand-off contract** (addresses: refusal re-wrap; `Attempted<T>`; `operation()`; `Tracker.progress`; `check` naming). Helpers return raw port errors or a `StepError::{Refused(ApplyError), Failed(cause)}` that the wrapper passes through. Show `pull` as `pull_steps(..)` plus `attempt.finish(result)`. Derive the refusal operation from the planned action in `failures()`. Add `RecheckLocal`/`Pull` to `ApplyOperation`. Drop `Tracker.progress`. Rename `check` to state the not-landed rule.
6. **Put run scope on `RunReport`** (addresses: `CorpusScope` beside the report). The engine stamps `scope` and mode once, and `failures()`, the exit code and `DossierAuthority::for_run(report)` read them.
7. **Rename the rendering input** (addresses: `FinishedRun` collision). Use something like `RunOutcome`, and state which type goes to `finaliser.finalise`.
8. **Harden the skill's shell use** (addresses: path splicing; id glob). Pass every path as a single-quoted argv token, refuse paths containing shell or glob metacharacters, and accept ids for derivation only in canonical token shape. Pin both rules in the skill test.
9. **Close the test gaps** (addresses: `local_unchanged_since_sync`; terminal baseline re-run). Add run-level copy-decision tests (recipe 1, legacy entry, blanked hash, missing entry). Pin `record-baseline` exactly, detect writes with a recording work-item store, and finish with a preview that plans nothing.
10. **Group broken work files under `needs-attention`/`needs-repair` in the skill** (addresses: malformed vs unparseable). Keep exit 70, but never suggest a bare retry.
11. **Apply the minors in one pass**: move marker token-safety into Phase 4; tighten conflict-marker detection; sanitise stderr fields with `single_line`; refuse symlinked recovery entries; move `conflict_status_of` to the item; rewrite the stale doc comments; fix the skill's promotion dispatch; widen the changelog to cover `TsvRecord` and config-adapters; list every `FileRemove` double; add the shared root-guard helper with a drop guard.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. It replaces one store that was over-broad and rooted in the wrong place with two stores, each confined to its own root. Every new policy is a pure function behind ports, and ADR-0069's rule that `work-adapters` has no edge to `corpus-adapters` is kept. The weak points are in how run-level facts and policy are owned: scope travels beside the report, `RunReport` accretes per-item policy, the recovery-label format is split across crates, and there is a `FinishedRun` name clash.

**Strengths**: Two roots enforced by the structure itself, through `within_project` and `StoreFactories`. A functional core with an imperative shell. `copy_settled` moved out of the port. `ItemAttempt` records landing as a fact. The containment fix is made once in the shared canonicaliser. No intermediate phase ships an unguarded pull. Code claims verified.

**Findings**:
- 🟡 major / high, Phase 3 §6 and Phase 2 §4: the new `FinishedRun` collides with `finaliser.rs:22`'s `FinishedRun<'a>`, the `RunFinaliser::finalise` input. Rename the rendering input or deliberately extend the existing type.
- 🟡 major / medium, Phase 5 §5/§7 and Phase 6 §4: `CorpusScope` and preview-ness are passed beside the report to four consumers. Stamp them onto `RunReport` once in the engine.
- 🔵 minor / medium, Phase 6 §4: dossier policy is placed on `RunReport` but must run inside `execute` before a report exists. Make these free functions in `dossiers.rs` over `ReportedItem`.
- 🔵 minor / high, Phase 5 §4: `Tracker` gains `progress`, which `class()` never reads. Drop it, or use it.
- 🔵 minor / medium, Phase 3 §6: the `<epoch>-<suffix>-<id>` format is written in `work-cli` and parsed in `work-adapters`. Make `RecoveryLabel` own both.
- 🔵 minor / medium, Implementation Approach and Phase 6 §2: a public `with_resolution` does not enforce the Prompt-only invariant. Make it `pub(crate)` or use `resolved(..)`.
- 🔵 suggestion / medium: group `SyncFiles` the way `RetirementFiles` is grouped.
- 🔵 suggestion / low: split `Console` and the store factories out of `sync.rs`.
- 🔵 suggestion / high: the `create_from_remote` range is stale (`apply.rs:427-463`). `IdentityWorkspace` needs a `locks()` accessor.

### Code Quality

**Summary**: A careful, test-first decomposition that opens real seams and gives each vocabulary a single owner. The claims checked out against the code. The main risk is Phase 5's `ItemAttempt` as sketched: its generic wrappers cannot carry refusal variants, the `Attempted<T>` hand-off is at odds with a `?`-driven body, and `operation()` is asked for information the variants do not hold.

**Strengths**: The `create.rs` seam precedent. `Console` centralises 36 print sites. Single-spelling vocabularies. The named `ApplierPorts`. Landing recorded structurally. No wildcards in `conflict_status_of`. `files()` provided over `listing()`. Reviewable splits (3a/3b, the `report.rs` move).

**Findings**:
- 🟡 major / high, Phase 5 §4: `write_item` and the helpers cannot route refusals through `E: Display` wrappers, so a `LocalWriteRefused` would be re-wrapped as `Io`. Define helper return types or a `StepError` pass-through.
- 🟡 major / medium, Phase 5 §4: a `?`-driven pull cannot return `Attempted<T>`, and `kept_copy` is lost. Show `pull_steps` plus `finish`.
- 🟡 major / high, Phase 5 §4: `operation()` cannot derive the refused action, and `recheck-local`/`pull` are missing from `ApplyOperation`. Derive the operation from `planned.action`.
- 🔵 minor / medium: `Tracker.progress` is unused.
- 🔵 minor / medium: `check` breaks the observe/mutate grid. Rename it, for example `without_landing`.
- 🔵 minor / high: doc comments made false are not scheduled for rewrite (`sync.rs:596-601`, `647-650`, `apply.rs:246-248`, `368-370`).
- 🔵 minor / medium: `CorpusScope` is a data clump.
- 🔵 minor / medium: `conflict_status_of` belongs on `ReportedItem`, and `ConflictShape` should be private and listed.
- 🔵 minor / medium: three vocabularies for recovery copies. Pick "kept edits".
- 🔵 minor / medium: `render_report` should compose per-family renderers.
- 🔵 suggestion / low: four types for two stores, and `SyncStores` fields unstated.

### Test Coverage

**Summary**: Thorough and test-first. Every acceptance criterion maps to a named test, and the regression runs real production-rooted stores. The referenced helpers and doubles exist as described. The gaps: the run-level copy decision, one loose re-run assertion, and missing doubles and fault modes.

**Strengths**: The production-rooted regression. The tracker pin before the type change, plus the exhaustive totality test. A shared event log, so each recheck is detectable. The root guard fails loudly. Fixed-epoch retention boundaries. Retirement tested both pure and end to end. Per-item landing scope covered.

**Findings**:
- 🟡 major / high, Phase 3 §2/§5: `local_unchanged_since_sync` is never computed in a test. Add run-level tests for recipe 1, legacy, blanked and missing entries.
- 🟡 major / medium, Phase 4 §1: the terminal-baseline re-run test cannot detect a rewrite and accepts two outcomes. Pin `record-baseline` and detect writes.
- 🔵 minor / high: the `Faults` `NotWritable` mode does not exist, and the test name overstates it.
- 🔵 minor / high: mixed conflict-plus-failure runs are tested for exit code only.
- 🔵 minor / medium: conflict-marker detection has a single test.
- 🔵 minor / high: the sweep's lock-held rule is untested.
- 🔵 minor / high: unlisted `FileRemove` doubles (`sync_run_real_client.rs:110`, `sync_recipe_migration.rs:113`, `tests/support/repo.rs:80`).
- 🔵 minor / medium: the root-guard helper has no home before Phase 7. Use a drop guard (`design-adapters/tests/tree_leases.rs:138-152`).
- 🔵 suggestion / high: two Phase 1 tests are characterisation tests, and the permitted-root `..` behaviour is unpinned.
- 🔵 suggestion / medium: the hash-fault mechanism (malformed `PullRequest.content`) and the preview-resolve exit code are unstated.

### Correctness

**Summary**: The current-behaviour claims hold, and the new pull ordering, landing classification and retirement rules are sound at their core. There are two cross-cutting problems. Repository-bounded stores break supported absolute `paths.*`, and malformed files leave the double-import guard. The remaining findings are edge-case gaps.

**Strengths**: `unresolved_tail` is correct for every listed shape. The pull sequence preserves 0296's re-run rule. Landing state lives in one place. `Unobserved` fallback and `proves_absence()` gating. Retention boundaries handled explicitly.

**Findings**:
- 🟡 major / high, Phase 4 §5–§6: repository-bounded stores refuse absolute out-of-repo `paths.work`/`paths.integrations` (`sync.rs:86-98`; `configure.md`).
- 🟡 major / medium, Phase 5 §5: malformed or unreadable files leave `request.corpus`, so a conflicted tracked item can be re-imported as a duplicate. Gate on `proves_absence()`.
- 🔵 minor / medium: `sync_parent_dir` can fail after the rename, so the content landed but is classed as not landed.
- 🔵 minor / medium: failure of marker removal through the store is unspecified, and an `Attempted` leftover escalates the next run.
- 🔵 minor / medium: unclassifiable rows are suppressed by `settled_ids`.
- 🔵 minor / medium: a single conflict marker makes an item permanently malformed.
- 🔵 minor / low: `--resolve =skip` resolution recording is unspecified.
- 🔵 minor / low: the sweep acts on a stale review, and unrecorded directories have no lock target.
- 🔵 suggestion / low: between 3a and 4, refused pulls create copies that nothing sweeps.

### Safety

**Summary**: Strong. Pulls are hardened before they become reachable, and retirement fails safe. The main gap is the 7-day expiry, which deletes the only copy of unmerged edits without confirmation. Two smaller issues remain: the copy skip for ignored files, and the marker-id guard landing late.

**Strengths**: Safe phase ordering. Lock, double recheck and hash-before-write. A conservative copy skip. A fail-safe sweep in most respects. Allow-list retirement. The containment fix. The conflict-marker guard. Confirmation gates in the skill.

**Findings**:
- 🟡 major / high, Phase 3 §6: time-based expiry deletes unmerged edits without confirmation, unlike retirement's `sweep_recoveries` (`retirement.rs:564-588`).
- 🔵 minor / medium, Phase 3 §5: the copy skip for VCS-ignored files rests on a normalised hash. Distinguish `Ignored`.
- 🔵 minor / high, Phase 4 §4 and Phase 5 §3: the marker token-safety guard lands one phase late.
- 🔵 suggestion / medium: the restore-pending record is not durable (`recovery.rs:134-144`), and the item path should be recovered from the mirrored copy.

### Security

**Summary**: The plan closes a real containment escape and adds solid defences: token-safe ids, `TsvRecord`, project-bounded stores, and the skill's rules. Gaps remain where it trusts repository-controlled input further on: paths in the skill's Bash commands, symlinked recovery entries, newlines on stderr, and single-check marker safety.

**Strengths**: Refuse-don't-resolve `..` in the shared canonicaliser. Token-safe marker and recovery labels. One sanitising record builder. The symlinked-root guard made live. Containment of recorded paths in the review. Quoting, key-shape and fixed-string search rules in the skill.

**Findings**:
- 🟡 major / medium, Phase 7 §1: repository-controlled paths are spliced into the skill's Bash commands. Single-quote them and refuse metacharacters.
- 🔵 minor / medium: recovery entries are followed through symlinks, and a planted copy can be restored and pushed.
- 🔵 minor / high: stderr `printable` keeps `\n`, which allows forged lines. Use `single_line` on fields.
- 🔵 minor / medium: `..` inside existing directories still resolves within the root, and marker safety rests on one check.
- 🔵 suggestion / low: path derivation from an id should require the canonical token shape and a literal prefix match.

### Compatibility

**Summary**: Careful about the CLI contract. Exit-code changes are gathered into 0296's changelog bullet and the public-API diffs are listed. The gaps are the authoritative `exit_codes.rs` text and a quiet behaviour change for symlinked work directories, plus smaller drifts between the changelog, the code and the skill.

**Strengths**: Exit-code changes consolidated with Migration Notes. Precedence unchanged and pinned. Additive stdout changes, with the golden asserted unchanged. `NotWritable { detail }` traced to every affected pattern. Public-API diffs listed, including the `work-adapters` exemption. `files()` provided over `listing()`.

**Findings**:
- 🟡 major / high, Phase 5 §4/§8: the 70/71 definitions and precedence line in `exit_codes.rs` contradict the new rule.
- 🟡 major / medium, Phase 4 §5–§6: a work directory symlinked out of the repository silently stops syncing, while `work create`/`update` continue to work.
- 🔵 minor / high: the changelog's OS-reason claim is false for config-adapters and the visualiser.
- 🔵 minor / medium: the skill dispatch mis-models promotion tokens, and `not-promoted`, `#\tdiscovery` and `#\tnote` have no entries.
- 🔵 minor / medium: `TsvRecord` changes `work promote`/`create-batch` output, and the changelog does not mention it.
- 🔵 suggestion / medium: removing `copy_settled` is an intentional pinned break; note it.

### Usability

**Summary**: Strong. There is one consistent stderr shape, single-word classes, and one dispatch section with explicit rules. The friction is around what exit 70 means: it now includes failures that need a fix first, yet the docs and the skill still say "safe to retry", and a moved 0296 paragraph misattributes causes. Smaller issues are CLI-only remedies, recurring notes and overloaded records.

**Strengths**: A single `error:` shape with single-owner keywords. A fixed cause order. Dispatch on record type, with quoting, paths and confirmation rules. A checkable completion rule. README-like files distinguished. A merge-by-default copy-over prompt. A loud root guard.

**Findings**:
- 🟡 major / high, Phase 5 §4 and Phase 7 §1: exit 70 is still documented as "safe to retry".
- 🟡 major / high, Phase 7 §1: the moved 0296 paragraph misstates retryable-pull causes.
- 🟡 major / medium, Phase 5 §5: broken frontmatter gets 70/`needs-retry` or 75/`needs-attention` depending on where it is detected.
- 🔵 minor / medium: CLI users lose the pending-push remedy, and the guide should gain a "needs attention" subsection.
- 🔵 minor / medium: preview `error: … failed:` should read "would fail".
- 🔵 minor / medium: recurring notes add noise.
- 🔵 minor / medium: `#\tdetail` is overloaded with two layouts.
- 🔵 minor / low: unclassifiable items always report `read-local`.
- 🔵 suggestion / medium: use `work link-external-id` in the recovery skill.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-21T11:38:26+00:00

**Verdict:** REVISE

Lenses re-run: architecture, code-quality, test-coverage, correctness, safety, security, compatibility, usability. Each lens read the whole revised plan.

### Previously Identified Issues

- 🟡 **Usability + Compatibility**: Exit 70/71 docs promise "safe to retry" — Resolved
- 🟡 **Correctness + Compatibility**: Repository-bounded stores refuse out-of-repo roots — Resolved (relative `..` roots remain open; see minors)
- 🟡 **Correctness**: Malformed files leave the duplicate-import guard — Partially resolved (the import gate is added, but it uses the wrong predicate and has no data path; see new majors)
- 🟡 **Safety**: 7-day expiry deletes unmerged edits — Resolved
- 🟡 **Security**: Paths spliced into the skill's Bash commands — Partially resolved (the move-aside, delete and copy-over actions remain; see new majors)
- 🟡 **Code Quality**: `ItemAttempt` re-wraps refusals — Resolved
- 🟡 **Code Quality**: `?` drops the kept copy — Resolved
- 🟡 **Code Quality**: `operation()` cannot recover the refused action — Resolved
- 🟡 **Architecture**: `FinishedRun` collision — Resolved
- 🟡 **Architecture + Code Quality**: Scope passed beside the report — Resolved
- 🟡 **Usability**: The moved 0296 paragraph misstates causes — Resolved
- 🟡 **Usability**: Broken frontmatter reported two ways — Partially resolved (malformed files now exit 75; unreadable files with invalid UTF-8 still group as `needs-retry`)
- 🟡 **Test Coverage**: Run-level copy decision untested — Resolved
- 🟡 **Test Coverage**: Terminal-baseline re-run test is loose — Resolved
- 🔵 Minors and suggestions: 47 of 52 resolved. Partially resolved: `NotDurable` (prose only), recovery vocabulary (still "recovery" in `pull_recovery.rs`, `keep_recovery_unless_synced`, `keep-recovery`), the four store types (fields now stated, but `BoundedRoot.repository` is misnamed), symlinked recovery entries (regular planted entries remain), and the store-type count.

### New Issues Introduced

#### Major

- 🟡 **Correctness + Test Coverage + Architecture**: The import gate blocks every first sync, and has no data path
  `proves_absence()` is false for an empty inventory, so a full run or `--target <KEY>` into an empty work directory never imports anything. It exits 70 on every targeted run. The inventory also reaches only `RunReport`, yet `untracked_to_import` runs in `prepare_run` before any report exists. Gate imports on completeness alone (no unreadable or malformed entry), carry the inventory on `SyncRequest`, and test that an empty, healthy corpus imports.
- 🟡 **Correctness**: Adopt verifies with `show`, which cannot report not-found
  `RemoteTracker::show` returns only `Retryable` (`tracker/src/lib.rs:729-744`), so a mistyped or deleted key loops as retryable. Use `locate`: `NotFound` becomes `OrderRefused`, and a moved issue links its current key.
- 🟡 **Safety**: An adopted key is never checked against the item, then both sides are recorded as synced
  A typo that names an existing, unrelated issue links it and records a baseline over divergent bodies. The next push then overwrites that issue. Refuse when the titles differ or a `Created` marker records a different key, and leave the baseline unrecorded unless the bodies are equivalent.
- 🟡 **Safety + Correctness**: `--resolve <id>=create` discards a `Created` marker
  `fingerprint-mismatch` comes only from a `Created { external_id }` marker, so the issue is known to exist and `create` makes a duplicate. Refuse `create` over a `Created` marker, and point the user to `adopt:<that key>`.
- 🟡 **Correctness**: An adopt order with no marker falls through to a fresh create
  "Ignored with a warning" lets a non-idempotent `create` run against the user's statement that the issue exists. Adopt anyway after the same checks, or refuse.
- 🟡 **Correctness**: Adopt and `ReuseId` miss carriers that are malformed or were imported this run
  `carrier_of` reads `request.corpus`, which now excludes malformed files and is fixed before apply. Refuse both when the inventory is incomplete, and exclude adopted or marker-named keys from this run's import set.
- 🟡 **Usability + Security**: The skill's paths rule contradicts its own move-aside, delete, VCS-status and copy-over flows
  These actions still need repository-controlled paths in mutating commands, an embedded `'` breaks single-quoting, and the read allow-list omits `#\tkept-edits`, `#\tstale-dossier` and the marker. Route the remaining mutations through CLI orders or the Read/Write tools, and refuse metacharacter paths.
- 🟡 **Usability**: A refused order is classed `rejected`, which the skill dispatches as a tracker rejection
  Route `adopt`/`create` refusals to the recovery skill, or give them their own class keyword.
- 🟡 **Usability**: The kept-edits notes say "delete them" without naming `--discard-kept-edits`
  Deleting a copy by hand leaves a record that recurs on every run. Name the flag in the notes and the guide, and settle a directory whose copy is gone.
- 🟡 **Usability**: A symlinked work or integrations root fails item by item after remote mutations
  Check all three roots in a pre-flight step, refuse once with a configuration exit, and name the right `paths.*` key.
- 🟡 **Code Quality**: `land_local` cannot see `NotDurable` (also architecture, correctness, test-coverage)
  The `E: Display` bound erases the kind, and the `?` returns before progress is set. Add `StepError::FailedAfterPublish(E)`, show it in the sketch, and add `kept_edits`/`after`/`created` setters.
- 🟡 **Code Quality**: Error `Display` shapes give `RunFailure.cause` no defined source
  0296's refusals already prefix `<id>: `, so `failure_lines` would print `error: <op> failed: 0001: …`. Give `ApplyError` `item_id()`, `cause()` and `operation(planned)`, and compose `Display` from them.
- 🟡 **Test Coverage**: `a_conflicted_tracked_item_is_not_re_imported` asserts exit 70, but malformed now exits 75
- 🟡 **Test Coverage**: `BoundedRoot::for_configured` has no unit table over its boundary cases
  The cases are: in-repo, in-repo symlinked out, relative `..` escaping, absolute outside, absolute inside, and an integrations-root symlink.

#### Minor

- 🔵 **Compatibility + Correctness + Security**: A relative `paths.work: ../shared` counts as "lexically inside" and is refused with a misleading cause. Define the check after lexical `..` normalisation, or refuse `..`, and decide whether team-config absolute roots are trusted.
- 🔵 **Usability + Compatibility**: `--resolve` mixes two vocabularies, and its argument help, warning, the skill's `argument-hint`, Step 1/2 templates and the changelog still list only `remote|local|skip`. Unrecognised values still skip silently, and two orders for one id are unspecified.
- 🔵 **Usability**: The refusal hint offers `=create` as an equal alternative without "search the tracker first".
- 🔵 **Usability**: Unreadable (invalid UTF-8) files are grouped `needs-retry`, although a retry reproduces them.
- 🔵 **Safety**: `=create` removes the old marker before writing the new `Attempted` one. Replace it in place.
- 🔵 **Architecture**: Pending-push orders don't appear on `PlannedAction`, so rows and previews can't tell an adopt from a create.
- 🔵 **Architecture**: A second `FileRecoveryCopies` sits over retirement's state root, with an unnamed bounded constructor. Reuse `IdentityWorkspace::recovery()`.
- 🔵 **Security**: Planted regular-file recovery entries can still reach the restore offer, and mirrored-path matching widens this. Treat VCS-tracked entries as `Unrecorded`.
- 🔵 **Security**: `FileRecoveryCopies` stays lexical below the state root, so a symlinked `sync-recovery` is followed. Refuse it.
- 🔵 **Code Quality**: `class()` sketch omits `OrderRefused`; `LocalUnparseable → read-local` mislabels a parse failure; `ApplyOperation` names non-apply operations (`SyncOperation`); `BoundedRoot.repository` holds a non-repository bound; `RunRecord::KeptEdits` means this-run only; land and `after(step)` must be paired by convention.
- 🔵 **Correctness**: A `sync-recovery` directory whose name doesn't parse has no id to report or discard. The 71 rewrite omits pending-push refusals, and the 75 "Distinct from 71" sentence is stale. Lengthened jj markers and fence-parity flips evade the conflict rule.
- 🔵 **Compatibility**: `tracked_paths` on the pinned `WorkingCopyStateProbe` has no work-directory scope for migrate and corpus consumers. The changelog and help omit `would fail` and `skipped\tincomplete-corpus`, and the 0296 "two new rows" changelog sentence is stale. config-adapters' wildcard arm drops `NotDurable`'s path. The `store::test_support` snapshot is hedged.
- 🔵 **Test Coverage**: Untested branches: adopt and create failures, an unsafe order id, conflicting orders, `OrderRefused` exit 75, the `already-written` no-hint case; discard's failure, no-match, lock and symlink cases; the vacuous stderr-forging test plus tracker-text forging; `tracked_paths` for jj and out-of-repo roots; the `NotDurable` injection seam, mapping and pull-write row; symlinked records and roots; the durability test's observable assertion; and composed ordering of the new record families.

### Assessment

Every structural finding from the first pass is resolved, and safety, compatibility and architecture confirm the core design. Two lenses found no new majors (compatibility) or only data-flow gaps (architecture). The new majors cluster in two surfaces this revision introduced:

- **Pending-push orders (6 majors).** They need tracker-backed verification (`locate`, title and key match) and refusal over `Created` markers, and must never fall through to a create.
- **The import gate (1 merged major).** It needs a completeness predicate and a `SyncRequest` route.

The remaining majors are a skill rule that contradicts its own flows, three usability gaps, two `ItemAttempt` contract holes, and two test fixes. All have concrete remedies. The design is stable; another targeted revision pass is needed before approval.

## Re-Review (Pass 3) — 2026-10-21T18:07:21+00:00

**Verdict:** REVISE

Lenses re-run: all eight, each reading the whole revised plan.

### Previously Identified Issues

- 🟡 All 14 pass-2 majors — Resolved, except two partials. The import gate is fixed, but `ReuseId` is still ungated on corpus completeness. The `ApplyError` cause source is fixed except for three variants, whose `Display` cannot know their operation.
- 🔵 Pass-2 minors — about 90% resolved. The partials are recovery vocabulary (`KeepRecovery`, `RecoveryLabel`), the kept-edits note naming the broad discard form, and the extra test branches.

### New Issues Introduced

#### Major

- 🟡 **Architecture + Compatibility**: Treating untracked files as `Unknown` dirtiness changes what `decide()` plans, so pulls of ignored or out-of-repo work files become `Prompt`/`SkipDirty`. Tracking belongs in the copy decision only.
- 🟡 **Security**: The personal-config-only rule for out-of-repo roots can be faked by committing `config.local.md`. Use the existing `config::consent` tracking check (`Tracked`/`Unknown` fails closed).
- 🟡 **Safety**: Settling a `CopyMissing` copy depends on the recomputed mirror path. A differently spelled root can delete a directory that still holds the real copy. Record the copy's path in the record, and settle only when the directory holds nothing else.
- 🟡 **Compatibility**: The symlinked `retirement-recovery` refusal reaches `work promote` and `work create --push`, which have no pre-flight check, so they fail after the remote create.
- 🟡 **Correctness**: A moved issue behind a `Created` marker leaves no order that can settle it: `adopt:OLD`, `adopt:NEW` and `create` are all refused.
- 🟡 **Correctness**: Orders are recorded on the Unsynced `Noop` action, not on the `create_report` row (`run.rs:1124-1138`), so the row never reads `adopt`. An order that reaches no local create (`--pull-only`, a draft, an already-linked item) is silently inert.
- 🟡 **Usability**: The exit-74 symlink remedy ("configure an absolute path") leads straight into the team-config refusal. It should name `config.local.md`.
- 🟡 **Usability**: A mistaken adopt has no recovery path. The conflict that follows offers only `remote`/`local`/`skip`, and there is no unlink.
- 🟡 **Code Quality**: `Display` cannot name the operation for `RemoteBodyUnavailable`, `RemoteUnrenderable` or `LocalPullResidue`. Carry the `SyncOperation` on every variant.
- 🟡 **Test Coverage**: Phase 2's stderr-forging tests need `error:` lines that only exist from Phase 5.
- 🟡 **Test Coverage**: An invalid-UTF-8 read at apply is pinned as retryable (70), but D10's rule says wrong content exits 75.

#### Minor (summarised)

- 🔵 **Types**: `Step` is undefined; `FailedAfterPublish` is accepted by `observe_local`; the setters are named like getters; `Undecodable` is nested under `Unreadable`; `PlannedAction` has two mutually exclusive optional fields; and the Implementation Approach still sequences `new` in Phase 6.
- 🔵 **Plumbing**: `CorpusInventory::of_items` would make `work` depend on `work-adapters`; `BoundedRoot` has no config-origin input and the root rule is evaluated twice; `ConflictedPaths` has no failure semantics (`VcsKind::None`, `Err`); and the `configure` operation sits outside `SyncOperation`/`RunFailure`.
- 🔵 **Exit codes**: A deferred targeted import should take its class from its cause (75 for content). `ReuseId` is ungated on an incomplete corpus. `FailedAfterPublish` prefixes `after pull`, contrary to the table.
- 🔵 **Security hardening**: Marker `external_id`s are unvalidated and tracked markers trusted; `! mv` needs `--` and no `..`; roots could overlap.
- 🔵 **Safety and usability text**:
  - The `mv` to `.bak` can overwrite an earlier backup, and a discard needs a preview list before confirming.
  - An `Unknown` tracking answer is labelled "not made by this tool", and this run's copies are not exempt from that.
  - The migration's untrack advice needs to warn about markers and collaborators.
  - A refused recovery parent should say to move its contents.
- 🔵 **Usability text**:
  - Order-refusal causes are incomplete and the recovery skill doesn't cover them.
  - The kept-edits note names the broad discard form.
  - The skill's 74 row still offers `--push-only`.
  - A team-config outside path still works for other commands.
  - Some `--resolve` exit-2 messages are unspecified.
  - The untrack commands aren't specific to the VCS.
- 🔵 **Test coverage**: There's no two-run real-VCS kept-copy test and no `UnreadableFile` guard. Config override rows and recovery-parent run tests are missing, as are some order branches. Recovery and conflict-marker edges are untested, and the dossier end-to-end tests don't state `--target`.
- 🔵 **Compatibility**: The meaning of 74 needs widening in the help, `exit_codes.rs` and the skill. Public-API lists miss `work` (`PlannedAction` fields), `migrate` (`Migration0011`, `MigrationContext`) and `config` (provenance). The ignore rules hard-code the default integrations path. The changelog and Migration Notes omit `order-refused`, `adopt` rows, `undecodable` and the state-root 74.

### Assessment

Every pass-2 major is resolved or nearly so, and the core design is stable across all lenses. The 11 new majors are narrower than pass 2's:
- **Structural, with no design question:** the `decide()` coupling, `CopyMissing`, the `Display` source and the test placement.
- **Applications of decisions already made:** consent provenance, the promote/create pre-flight, the moved-issue key, the remedy text and the UTF-8 class.
- **Needing a decision:** wrong-adopt recovery, and how an inert order is reported.

## Re-Review (Pass 4) — 2026-10-21T18:48:11+00:00

**Verdict:** REVISE

Lenses re-run: all eight.

### Previously Identified Issues

- 🟡 **Pass-3 majors:** 7 of 11 resolved: `decide()` coupling, `CopyMissing` safety, `Display` source, test placement, remedy text, moved-issue adopt, and orders on the row. 4 partially resolved:
  - **Consent provenance:** the mechanism chosen doesn't fit.
  - **Promote/create pre-flight:** both commands map the failure to exit 1, and `create --push` has a keyword contract.
  - **Wrong-adopt recovery:** see the new majors.
  - **UTF-8 class:** the rows now contradict each other.
- 🔵 **Pass-3 minors:** most resolved. The rest are listed below.

### New Issues Introduced

#### Major

- 🟡 **Safety + Security + Compatibility + Correctness + Architecture**: "Unconfirmed link" is inferred from a missing baseline entry. That also matches correct links: a degraded or absent `last-sync.json`, a terminal `baseline-set` after author or link, `work update --push`'s deliberate clearing (`update.rs:264-289`), or a hand-set id. So `create` can duplicate a real issue. Record unconfirmed links positively, with a machine-local record that only a differing adopt writes.
- 🟡 **Correctness + Test Coverage**: Orders on an unconfirmed link have no execution path. Such an item is never `Unsynced` (`classify.rs:545`), so `plan()` and `unsynced_creates` never route its order.
- 🟡 **Security + Architecture + Compatibility + Correctness**: `config::consent` can't express "consent only when outside the repository". `Trust` attaches only to `EXTRA_KEYS`, a team value is always refused, the session audit flags every team value, and `PathConsent` rejects relative and in-repo paths. A dedicated provenance query is needed.
- 🟡 **Safety**: Reading a tracked or malformed-key marker as `marker-unreadable` drops the `Created`-key guard on `create`.
- 🟡 **Code Quality**: `land_*` requires a `Step` that marker writes and the tracker create cannot supply, and `record_created` is paired with it only by convention.
- 🟡 **Usability**: Wrong-adopt guidance lives only in the recovery skill, which the conflict loop never loads.
- 🟡 **Compatibility**: The new exit 74 from `work create --push` falls outside that command's keyword outcome contract (an unconfigured tracker is `local-save`, exit 0) and outside its skills.
- 🟡 **Test Coverage + Correctness**: The UTF-8 rows contradict each other (75 vs terminal after landing), and no variant yields `read-local`/`unparseable`.
- 🟡 **Test Coverage**: The targeted-deferral test expects 70 for a malformed blocker that the rule makes 75, and the deferral has no variant that can carry its class.
- 🟡 **Test Coverage**: The three-run real-VCS kept-copy test sits in Phase 3b, where pulls cannot land.

#### Minor (summarised)

- 🔵 **Exit-74 plumbing:** promote and create map `open()` failures to exit 1. The consent rule's reach beyond sync contradicts its own text. `open()` creates directories before checking, and ownership between `open` and `assemble_run` is unclear. Phase 4 renders through Phase 5 types.
- 🔵 **Types and ports:** the sketch still builds stores from `StoreRoots`, and `BoundedRoot` borrows a computed path. The tracking port is absent from the final `SyncPorts` though it has 3 consumers. `MigrationContext`'s query needs a default and a migrate-local type (pup boundary). `UnidentifiedReason::class()` can't return a `work-adapters` type. `OrderRefused` causes should be an enum. `fail`/`failed` are homonyms. `with_order_of` should copy `Order` only, with the same id.
- 🔵 **Recovery copies:** `copy_path`/`compare_copy` still take the original path. Untracked files accumulate copies that never settle. The import exclusion misses a moved marker key.
- 🔵 **Skill and text:** the inert causes are missing from the recovery table, and unknown vs deferred ids need splitting. The untrack wording needs the specific VCS command and the `Unknown` case. The Migration Notes are stale (the symlink remedy, `--resolve` exit 2, `order-refused`, the promote/create 74 refusals). The confirmation rule still names `<path>.bak`. The skill's key-shape rule is missing, and a moved-to key isn't parsed. The overlap check needs canonical comparison, and state-root containment needs its own cause.
- 🔵 **Tests:** the `create --push` pre-flight, nested overlaps, the `Unknown` fail-closed cases, `CopyMissing` with other entries present, the deferred-id order, and the default for the tracking double.

### Assessment

The new majors cluster in three places:
- **Pass-3 rules that rely on inference:** a missing baseline, a fit-for-purpose consent API, and marker distrust that's too broad.
- **The routing E1 needs:** an execution path for orders on unconfirmed links.
- **Test-table consistency.**

The architecture core is stable. Most fixes are structural; two need a decision: `create --push`'s exit-74 contract, and copies of untracked files.

## Re-Review (Pass 5) — 2026-10-21T19:22:34+00:00

**Verdict:** REVISE

Lenses re-run: all eight.

### Previously Identified Issues

- 🟡 **Pass-4 majors:** 8 of 10 resolved: missing-baseline inference, the `Relink` execution path, provenance, the untrusted-marker guard, `land_*`/`Step`, wrong-adopt reachability, `create --push`'s contract, and UTF-8 consistency. 2 are partially resolved: the adoption-record lifecycle, and the skill text for `local-save`.
- 🔵 **Pass-4 minors:** most resolved. Signature drift (`RecordedCopy` in §6) and pre-flight ownership remain partial.

### New Issues Introduced

#### Major

- 🟡 **Architecture + Correctness + Safety**: Removing the adoption record is a safety step that depends on every baseline-writing path, with no single owner. Fix: a link is unconfirmed only with a trusted record *and* no baseline entry, so removal becomes housekeeping.
- 🟡 **Test Coverage**: The `Tracked` default of the tracking double contradicts the distrust rules for state files. The default should depend on the path.
- 🟡 **Test Coverage + Architecture + Correctness + Code Quality**: Adopt on an incomplete corpus is specified as both `CorpusIncomplete` and `OrderRefused`.
- 🟡 **Test Coverage**: No test pins that a later baseline confirms an adopted link.
- 🟡 **Test Coverage + Architecture + Correctness**: `Relink` is absent from the conflict-status and classification tables, and its corpus and direction gates are unstated.
- 🟡 **Usability**: The banner-case conflict still uses the remote-recommending prompt and the remote/local/skip normaliser.
- 🟡 **Compatibility + Usability**: The create and extract skills render a root-refusal `local-save` as "tracker unreachable", and point to `/sync-work-items`, which refuses again.

#### Minor (summarised)

- 🔵 **Sketch consistency:** `land_local` composes the error before setting progress. `Step` collides with the closure and field. `landed(None)` keeps a stale prefix. `Relink` holds its order twice. `BlockingKind` duplicates `Fault`. `PathSource` can represent invalid states. Two owners are still named for the root check.
- 🔵 **Adoption records:** partial writes, identity renames and key follows. The `adopted/` ignore line isn't threaded through 0011. `Relink`'s payload breaks `Action: Copy`.
- 🔵 **Roots:** normalised roots don't reach discovery, the lock or the baseline. `confirm_remote_candidates` calls the tracker before `open`. `ValidatedRoots` overstates promote's partial check. Overlap comparison is case-sensitive.
- 🔵 **Copies and import:** `Unknown` tracking in the copy decision is unstated. Import exclusion doesn't cover a failed `locate` or keyless `Attempted` markers.
- 🔵 **Text:** sanitisation of the adopted title, the confirmed-wrong-link remedy, the `Unknown`-provenance remedy, the adopt-note dispatch entry, inert causes in the recovery table, `### Migrations` placement, and action keywords in help and Step 5.
- 🔵 **Tests:** `LocalUnparseable` operation per cause, the legacy-entry copy test's location, `path_source` unit tests, mixed blockers, and two Phase 4 behaviours without red tests.

### Assessment

Security, safety and code quality found no new majors. The remaining majors are internal consistency around the adoption record and `Relink`, plus two skill-text gaps. All but one minor are structural. One behaviour question remains: whether untracked import should wait while an `Attempted` marker is outstanding.

## Re-Review (Pass 6, focused) — 2026-10-21T19:42:46+00:00

**Verdict:** COMMENT

Lenses re-run: correctness, test-coverage and usability. These are the lenses whose pass-5 majors sat in the areas edited since, plus decision G1.

### Previously Identified Issues

- 🟡 **All 7 pass-5 majors resolved.** Adoption-record ownership, the path-aware tracking double, the single `CorpusIncomplete` variant, baseline-confirms-link tests, `Relink` table rows, the banner prompt, and the create/extract local-save text. The last is mostly resolved; minors remain.
- 🔵 **Most pass-5 minors resolved.** The remainder carry forward below.

### New Issues Introduced

No new majors.

#### Minor (summarised)

- 🔵 **Correctness:**
  - A relink create runs in a separate `ItemAttempt`, so the clear's landing is lost.
  - A stale baseline entry can silently confirm a differing adopt.
  - Confirmation can revert while a record survives.
  - A leftover `Attempted` marker on a linked item defers import forever.
  - The `previous_key` trust and follow rules are under-specified.
  - The first discovery runs before the roots are validated.
  - `failures()` omits the discover deferrals.
  - `CopyGuard` duplicates the boolean.
  - The sketch references `step` after its rename.
- 🔵 **Test coverage:**
  - The content check omits the banner prompt and the create/extract sentence.
  - The `Relink` row keyword is ambiguous (`relink` vs `adopt`/`create-from-local`).
  - The adoption-record write failure has no class.
  - 0011 says "four lines" although the list has five.
  - The records golden lacks `#\tadopt` and `pending-push`.
- 🔵 **Usability:**
  - The banner prompt doesn't spell out what `remote`/`local` overwrite.
  - The pending-push deferral has one message for three causes.
  - The `#\tadopt` dispatch is mis-described.
  - The `adopted` group is missing from the membership table.
  - Create's step-8 confirmation and extract's outcome string are unchanged.

### Assessment

The plan has converged: six passes in, no lens finds a defect that would cause wrong behaviour, data loss or contradictory tests at major severity. The remaining minors are edge cases in the adoption-record and `Relink` machinery, plus skill-text precision. All are structural and can be applied directly.

## Approval — 2026-10-21T19:55:24+00:00

**Verdict:** APPROVE

The plan was revised for every pass-6 minor finding, with no further review pass:
- **Adoption records:** the trust rule is stated once, a differing adopt clears any stale entry, confirmation is monotonic, and the key follow handles both keys.
- **`Relink`:** the create runs inside the relink's attempt, and the row keyword is fixed as `relink`.
- **Import deferral:** it is scoped to `Unsynced` items, carries two causes, and is routed through `failures()`.
- **Roots and copies:** roots are validated before discovery, and a recipe-only `UnchangedSinceSync` input replaces `CopyGuard`.
- **Smaller fixes:** the `adopt-record` class, migration 0011's five lines, the records golden, the banner prompt's per-option text, the adopt dispatch split, the `adopted` membership row, create/extract confirmation text, and the content checks for all of it.

Across six passes, the six-pass history in this file shows majors falling from 14 to 0. The core design (two bounded stores, the landing-based failure class, scoped dossier retirement, id-keyed recovery orders, kept edits until settled, and in-process VCS reads) held throughout.
