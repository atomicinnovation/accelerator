---
type: "plan-review"
id: "2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently-review-1"
title: "Plan Review: Sync Pulls of Tracked Work Items Fail Silently Implementation Plan"
date: "2026-10-11T07:43:59+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "security", "compatibility", "usability"]
review_number: 1
review_pass: 9
tags: ["sync", "work-cli", "work-adapters", "store", "sync-work-items"]
last_updated: "2026-10-12T03:03:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Sync Pulls of Tracked Work Items Fail Silently Implementation Plan

**Verdict:** REVISE

The plan's structure is sound. It splits the writer into two bounded ports bound in `work-cli` (ADR-0069 holds). It makes classification total, which the compiler enforces. Phase 2's seam refactor is behaviour-preserving, and Phase 1 fails closed on `..`. Three of its stated criteria cannot hold as designed, though. The permission test asserts an OS message that the store discards. No classification path makes a converged conflict non-conflicted. `conflicted()` excludes `SkipConflict`, so single-direction runs still delete live dossiers. The plan also makes the dormant pull path live without hardening it against concurrent edits and unreadable files. And its classification table leaves several terminal sites, including every `hash-local` site, untested.

### Cross-Cutting Themes

- **`conflicted()` conflates "dossier built this run" with "still conflicted"** (flagged by: architecture, correctness, test-coverage, safety). `SkipConflict` (under `--pull-only`/`--push-only`), `Indeterminate` and `RemoteAbsent` items fall outside `conflicted()`, so retirement deletes their dossiers, which recreates the 0300 bug on another path. Inferring "under resolution" from `state == Conflict` also misses a resolve on a dirty `RemotelyModified` item (`decide` yields `Prompt` there too).
- **Failure class depends on which constructor each call site picks** (flagged by: architecture, code-quality, test-coverage, correctness). The table omits push and create-from-remote `hash-local`/`baseline-set`, all of `ReuseId`'s post-link steps, and `record_terminal_failure`'s marker write. It also claims `hash-local` cannot be tested, yet fence-less content makes `digest::local` fail. A single `ItemProgress` parameter threaded into `link_and_baseline` invites misclassifying everything after the link.
- **Pull path goes live unhardened** (flagged by: safety, correctness). `pull_outcome` does `read_to_string(..).unwrap_or_default()` (`run.rs:743`), and reconstruction falls back to empty frontmatter, so a failed read writes `---\n\n---\n\n<body>` and drops `external_id`. The pull takes no `ForFile` lock and never re-checks the classified digest before overwriting.
- **The `PushRefused` Display breaks the stated stderr contract** (flagged by: usability, code-quality, compatibility). `"{id}: create refused: {reason}"` does not match the documented `error: <id>: <operation> failed: <cause>`, and it changes today's `create failed:` shape.
- **Seam surface grows piecemeal** (flagged by: architecture, code-quality, usability). `work_item_writer` first appears inside a Phase 4 test bullet. `run_sync` grows to seven parameters, departing from the cited `create.rs` `run`/`run_with` precedent, while the 340-line function is rewritten without being decomposed.
- **Deletions and the `.gitignore` write bypass the bounded writers** (flagged by: architecture, code-quality, safety). Dossier retirement, marker removal and `.gitignore` use raw `std::fs`, so "every sync write lands under one bounded writer" overstates the invariant.
- **Terminal rows give no recovery path** (flagged by: usability, safety). A terminal `baseline-set` after a pull leaves a phantom conflict on the next run. Resolving it to `local` would push 0296's degraded body upstream.

### Tradeoff Analysis

- **Safety vs scope**: the safety lens wants the `ForFile` lock, a pre-write staleness check and a recovery copy before an over-dirty resolution in this plan, because Phase 3 is the first point at which pulls overwrite files. Adding them widens 0300 beyond the work item. Recommendation: the lock, staleness check and fallible-read fix belong here, since this plan makes them reachable. The recovery copy can be deferred.
- **Containment strictness vs blast radius**: Phase 1 tightens shared `store` code that config-, migrate-, research- and corpus-adapters (and reads) depend on. Compatibility wants that acknowledged and audited, while security values the global fix. Recommendation: keep the global fix, but state its reach, add a `read_within` test and record a changelog line.

### Findings

#### Critical

None.

#### Major

- 🟡 **Correctness / Architecture / Safety / Test Coverage**: `conflicted()` excludes `SkipConflict`, so its dossiers are retired
  **Location**: Phase 5, §2 Engine and §4 CLI persistence
  `--pull-only --target a` on a conflicted `a` deletes `a.md`, and whole-corpus single-direction or budget-limited runs do the same for `SkipConflict`, `Indeterminate` and `RemoteAbsent` items. Retirement must remove only items whose conflict settled.
- 🟡 **Correctness**: no classification path makes a converged conflict non-conflicted
  **Location**: Phase 5, §3 (“`a`'s local edit now matches remote”)
  `classify` compares each side against the baseline only (`classify.rs:128-133`), and conflict runs blank `local_hash`. The test as written cannot pass without a convergence rule.
- 🟡 **Correctness**: the `Permission denied` assertion cannot pass
  **Location**: Phase 4, §4 `a_non_writable_work_directory_fails_the_pull_as_retryable`
  `stage` maps `PermissionDenied` to `WriteError::NotWritable { path }` (`store/src/lib.rs:301-307`) and drops the OS text.
- 🟡 **Safety**: pulls overwrite work items without the `ForFile` lock or a staleness check
  **Location**: Phase 3, §4–5
  `work update` and `link_external_id` both lock, but the pull does not. An edit made between planning and apply is silently overwritten.
- 🟡 **Safety / Correctness**: an unreadable local file at pull time is replaced with empty frontmatter
  **Location**: Phase 3 (pull path now reachable); Phase 4 classification table
  `unwrap_or_default` on the read and on the frontmatter split writes a file with no `id` or `external_id`.
- 🟡 **Test Coverage / Correctness**: the classification table omits terminal call sites
  **Location**: Phase 4, §2
  Push `baseline-set`, create-from-remote `baseline-set`, `ReuseId` post-link `read-local`/`baseline-set`, local-create post-link `read-local` and the terminal-tracker marker write all lack rows.
- 🟡 **Test Coverage**: `hash-local` is testable, and the totality test does not cover it
  **Location**: Phase 4, §2 (hash-local note)
  Fence-less `PullRequest.content` or a seeded file without a fence makes `digest::local` fail after a write. `classification_is_total` proves only that the variants are total.
- 🟡 **Test Coverage**: the `Fake` mechanics cannot produce several table rows
  **Location**: Phase 4, §2 (test mechanism)
  The `Attempted` and `Created` markers share one path. `Fake::create` is `unimplemented!`, post-write reads bypass the fake, and caller reads live in `run.rs`.
- 🟡 **Architecture**: failure class depends on which constructor each call site picks, not on what happened
  **Location**: Phase 4, §3
  Each new step that 0296 adds must re-derive its constructor by hand. A per-apply progress tracker would derive the class instead.
- 🟡 **Code Quality**: `run_sync` keeps growing with no decomposition
  **Location**: Phase 2
  The function is already 340 lines with `#[allow(clippy::too_many_lines)]`. Every phase adds to its `Ok` arm.
- 🟡 **Compatibility**: the store containment change affects reads and every store consumer
  **Location**: Phase 1
  `read_within` and permitted-root resolution share the canonicaliser. That contradicts the plan's "containment for reads" non-goal, with no audit and no changelog entry.
- 🟡 **Usability**: terminal rows say "needs attention" but give no recovery path
  **Location**: Phase 6, Failed items subsection
  Recovery differs by operation (for example `baseline-set` → resolve `remote`; `marker-write`/`link-external-id` → do not re-create).

#### Minor

- 🔵 **Usability / Code Quality / Compatibility**: the `PushRefused` Display contradicts the `<operation> failed:` stderr contract — Phase 4 §3 / Phase 6.
- 🔵 **Safety**: a resolution pull of a dirty `RemotelyModified` item loses its dossier even on failure — Phase 5 §2.
- 🔵 **Safety / Architecture / Code Quality**: deletions and the `.gitignore` write bypass the bounded writers — Desired End State / Phase 5 §4.
- 🔵 **Correctness**: overwriting in place leaves a stale dossier when the rewrite fails — Phase 5 §4.
- 🔵 **Security**: both writers drop the project-root check against a symlinked root — Phase 3 (`FileCorpusStore::new(root)` sets `project_root == permitted_root`).
- 🔵 **Security**: dossiers move from a `conflicts/`-rooted writer to the broader integration-state root, leaving `id_is_token_safe` as the only guard — Phase 3 §5 / Phase 5 §4.
- 🔵 **Architecture / Code Quality / Usability**: the work-item writer seam is introduced informally in a Phase 4 test bullet — Phase 3 / Phase 4.
- 🔵 **Architecture**: `EarlierWriteLanded` also covers remote tracker mutations, which the name hides — Phase 4 §3.
- 🔵 **Architecture**: the `Console` seam misses output from `ConfiguredLocalAuthor` and the tracker registry — Phase 2.
- 🔵 **Code Quality**: `let _ = writeln!` at ~35 sites; put the discard on `Console` methods instead — Phase 2.
- 🔵 **Code Quality**: `awaits_resolution`, `conflicted` and `awaiting_human` have near-synonymous names — Phase 5 §2.
- 🔵 **Code Quality**: `ReuseId` progress is threaded as a parameter; prefer one `io(progress, …)` constructor — Phase 4 §3.
- 🔵 **Test Coverage**: the root-run skip passes silently, and the retry test depends on another test's state — Phase 4 §4.
- 🔵 **Test Coverage**: the shared canonicaliser is tested only on the write path — Phase 1.
- 🔵 **Test Coverage**: untested dossier edge cases: write-then-retire order, failed resolution push, dossier-write failure leaving the exit code unchanged — Phase 5 §3.
- 🔵 **Test Coverage**: `sync_settled.rs` keeps a repo-wide store that masks routing bugs — Phase 3 §6.
- 🔵 **Test Coverage**: the per-operation run-level criterion is covered only by composition — Phase 4 §4.
- 🔵 **Usability**: "gate the closing summary" could hide which items did succeed — Phase 6 Step 5.
- 🔵 **Usability**: the docs guide has no exit-code prose to mirror, so it gains nothing — Phase 6 §2.
- 🔵 **Usability**: the dossier fix depends on a `--target` that the skill's template omits — What We're NOT Doing / Phase 5 manual check.
- 🔵 **Usability**: the changelog does not warn that exit 4 can hide a retryable failure — Phase 6 §3.

#### Suggestions

- 🔵 **Architecture**: gather conflict-lifecycle predicates into one `ConflictStatus` classification — Phase 5 §2.
- 🔵 **Architecture / Usability**: follow the `create.rs` `run`/`run_with` shape, and fold `registry`/`finaliser` into `SyncSeams` — Phase 2.
- 🔵 **Code Quality**: put `FailureClass::keyword()` on the type — Phase 4 §5.
- 🔵 **Correctness**: record `PushRefused` → terminal as an explicit exception to the progress rule — Phase 4 §3.
- 🔵 **Safety**: keep a recovery copy before a resolution pull over a dirty file — Phase 3.
- 🔵 **Safety**: give the 0296 release gate a mechanical guard — Migration Notes.
- 🔵 **Security**: strip control characters from tracker detail in `failure_lines`; have the skill treat it as data — Phase 4 §5 / Phase 6.
- 🔵 **Compatibility**: describe the new dossier lifecycle in the changelog — Phase 6 §3.
- 🔵 **Test Coverage**: assert the follow-up preview positively (`synced\t1`, exit 0) — Phase 3 §1.
- 🔵 **Test Coverage**: assert that a warning goes to `err` and not to `out` — Phase 2.

### Strengths

- ✅ Both writers stay `&dyn AtomicWrite` ports bound in `work-cli`, so `work-adapters` gains no `corpus-adapters` edge (ADR-0069).
- ✅ The `..` fix lands once in `cli/store` and fails closed on an unresolvable tail before `create_dir_all`, with tests for every shape.
- ✅ `class()` becomes total with an exhaustive, wildcard-free match, so the compiler enforces "every failure is visible".
- ✅ `ItemProgress` names the retryable/terminal distinction in domain terms. `another_items_successful_pull_does_not_make_a_failure_terminal` guards against the likeliest mutation.
- ✅ The tracker-mapping pin is committed green before the type change. Exit precedence is tested as a pure function, and the golden report fixture is pinned.
- ✅ The Phase 3 regression test drives `run_sync` over production-rooted `FileCorpusStore`s, which closes the spy-writer gap that hid the bug.
- ✅ `DossierAuthority` turns retirement scope into an explicit domain rule. Retiring after writes removes the clear-before-outcome race, and the fail-closed `.gitignore` check stays.
- ✅ The breaking exit-code change is called out, precedence is unchanged, and release is gated on 0296.
- ✅ `PushRefused` gets its own variant instead of hiding inside `Io`, and dead `ItemApplier::finalise` is removed.

### Recommended Changes

1. **Separate "dossier built" from "conflict settled"** (addresses: `conflicted()` excludes `SkipConflict`; dirty `RemotelyModified` resolution; whole-corpus retirement of unobserved items; near-synonymous predicates)
   Retire only items whose conflict settled this run: an `Applied` resolution, or a planned item whose state is now non-`Conflict` and observable. Retain `SkipConflict`, `SkipDirty`, `Indeterminate` and `RemoteAbsent`. Mark `--resolve`-rewritten actions in the planner instead of inferring them from `state == Conflict`. Add end-to-end tests for `--pull-only --target a` and a budget-limited full run.
2. **Settle the "converged" dossier criterion** (addresses: no classification path for convergence)
   Either add a convergence rule in `classify` (local digest == remote digest → `Synced`) with its own red test, or rewrite the scenario and get the work-item criterion amended.
3. **Fix the permission assertion** (addresses: `Permission denied` cannot pass)
   Either carry the OS error text through `WriteError::NotWritable`/`StoreError::NotWritable` with a store-level red test, or assert `not writable` and amend the criterion.
4. **Harden the pull path before it goes live** (addresses: no lock or staleness check; empty-frontmatter overwrite)
   Take the `ForFile` lock and re-verify `digest::local` under it. Make the local read and frontmatter split fallible, classed as `before_any_write` `read-local`. Add red tests for a mid-run edit and for an invalid-UTF-8 file.
5. **Complete the classification table and specify its mechanics** (addresses: omitted terminal sites; `hash-local`; `Fake` mechanics; constructor-by-hand)
   Add one row per failing call site, `hash-local` included, driven by fence-less content. Say how each row is produced: an Nth-write fault, a create-capable tracker and author fake, or `sync_create.rs`/`sync_run.rs` for the `run.rs` reads. Prefer a per-apply progress tracker, or at least state that every post-link step uses `after_a_write`.
6. **Acknowledge and test Phase 1's full reach** (addresses: store change affects reads and all consumers; canonicaliser tested only on writes)
   Audit the other `store` consumers, add `read_within` and missing-root tests, and add a changelog line.
7. **Unify the stderr shape** (addresses: `PushRefused` Display)
   Render it as `<id>: create failed: refused — <reason>`, or have the skill relay any `error: <id>:` line verbatim.
8. **Define seams once, in Phase 2 or 3** (addresses: piecemeal seams; `run_sync` growth; `Console` noise)
   Use `run_sync` and `run_sync_with`, and put both writer factories in `SyncSeams` from the start. Extract the post-report `Ok` arm into its own function, and give `Console` `report`/`warn` methods that discard write errors once.
9. **Add recovery guidance for each terminal operation in the skill** (addresses: terminal rows give no recovery path)
   Give a recovery table keyed by operation. Always print the grouped summary; only the "complete" statement waits for both groups to be empty. Add `needs-retry`/`needs-attention` to the docs guide and note in the changelog that exit 4 can mask failures.
10. **Close the containment gaps** (addresses: symlinked root; dossiers on a broader root; deletions bypassing writers)
    Build both writers with `project_root = repo_root`, and keep the `id_is_token_safe` check with a test. Route retirement and marker removal through `FileRemove`, or narrow the end-state wording.

## Per-Lens Results

### Architecture

**Summary**: Structurally sound: two ports bound in `work-cli`, a total `class()` and an explicit `DossierAuthority`. The weaknesses are that `conflicted()` conflates two meanings, classification depends on which constructor each site picks, and the boundaries are partly enforced (the console seam misses some output; conflicts-directory housekeeping bypasses the writers).

**Strengths**: ADR-0069 direction preserved; each writer refuses the other's root; the containment fix is centralised in `cli/store`; class totality is compiler-enforced; Phase 2 is behaviour-preserving; `DossierAuthority` is a named domain rule; the release dependency and breaking change are stated.

**Findings**:
- 🟡 major/high — Phase 5 Engine/CLI persistence — Retirement treats "no dossier built" as "no longer conflicted", so `SkipConflict` dossiers are deleted. Split "dossier refreshed" from "conflict settled"; retire only settled items; add a `--pull-only --target a` case.
- 🟡 major/medium — Phase 4 Domain model — Class depends on which constructor each site picks, not on what happened. Track per-apply progress and have a single `io_error` read it.
- 🔵 minor/medium — Phase 4 — The `EarlierWriteLanded` name hides that remote mutations count; consider `Untouched`/`PartiallyApplied`.
- 🔵 minor/high — Phase 3/4 — The work-item writer seam appears informally in a Phase 4 test bullet; define both seams in Phase 3.
- 🔵 minor/medium — Phase 2 Console — `ConfiguredLocalAuthor` (`sync_author.rs:152`) and the registry (`tracker_registry.rs:252-255`) print directly.
- 🔵 minor/medium — Desired End State/Phase 5 — The `.gitignore` write and dossier deletion bypass the bounded writers.
- 🔵 suggestion/medium — Phase 5 Engine — A fourth overlapping predicate; consider an enum `ConflictStatus`, and state how a failed resolution counts towards exit 4.
- 🔵 suggestion/low — Phase 2 — Seam grouping diverges from `create.rs`; use `run_sync`/`run_sync_with`.

### Code Quality

**Summary**: Maintainability is good: test-first, new domain types, dead code deleted and totality enforced by the compiler. The risks are continued growth of `run_sync`, piecemeal seams, near-synonymous predicates, and contract inconsistencies.

**Strengths**: exhaustive totality test; `PushRefused` as its own variant; `ItemProgress` instead of a bool; `ItemApplier::finalise` removed; one predicate drives both dossier build and `conflicted()`; behaviour-preserving Phase 2; the tracker pin taken before the type change.

**Findings**:
- 🟡 major/medium — Phase 2 — `run_sync` (~340 lines) keeps growing; extract the post-report `Ok` arm.
- 🔵 minor/high — Phase 2 §2 — ~35 `let _ = writeln!`; put the discard in `Console` methods.
- 🔵 minor/medium — Phases 2–4 — Seam surface grows piecemeal; define the final `SyncSeams` shape once.
- 🔵 minor/high — Phase 5 §2 — `awaits_resolution`, `conflicted` and `awaiting_human` are confusingly close; rename to something like `is_conflict_under_resolution`.
- 🔵 minor/medium — Phase 4 §3 — Classification relies on per-site constructor choice; provide `ApplyError::io(progress, …)`; have `write_marker` take the request.
- 🔵 minor/high — Phase 4 §3 vs End State — The `PushRefused` Display breaks the stderr contract.
- 🔵 minor/medium — Phase 5 §4 — Dossier I/O is split between the writer port and raw `std::fs`.
- 🔵 suggestion/medium — Phase 4 §5 — Move the keyword mapping onto `FailureClass`.

### Test Coverage

**Summary**: Test-first throughout, with a strong production-rooted regression test. The classification table is the weak spot: it omits terminal sites, wrongly calls `hash-local` untestable, and depends on `Fake` mechanics that cannot produce several rows.

**Strengths**: production-rooted regression test; tracker pin before the type change; a per-item vs run-wide mutation guard; pure precedence tests including mixed cases; every `..` shape tested; dossier criteria mapped one-to-one to tests; confinement tested via `SyncWriters::bounded`.

**Findings**:
- 🟡 major/high — Phase 4 §2 — The table omits push/create-from-remote `baseline-set`, `ReuseId` post-link steps, local-create post-link `read-local`, and the terminal-tracker marker write.
- 🟡 major/high — Phase 4 §2 — `hash-local` is testable with fence-less content; the totality test does not cover individual sites.
- 🟡 major/high — Phase 4 §2 — The `Fake` cannot distinguish the `Attempted`/`Created` marker writes; `Fake::create` is `unimplemented!`; post-write reads use `std::fs`; caller reads live in `run.rs`.
- 🔵 minor/medium — Phase 4 §4 — The per-operation run-level criterion is met only by composition; say so, or add a `run.rs`-constructed end-to-end case.
- 🔵 minor/medium — Phase 4 §4 — The root skip passes silently; separate tests share no state.
- 🔵 minor/medium — Phase 1 — The canonicaliser is tested only on writes; add `read_within` and missing-root cases.
- 🔵 minor/medium — Phase 5 §3 — Retire order, failed resolution push, and a dossier-write failure leaving the exit code unchanged are untested.
- 🔵 minor/low — Phase 5 §1 — `SkipConflict` exclusion is pinned without an end-to-end test of its dossier effect.
- 🔵 minor/medium — Phase 3 §6 — Root `sync_settled.rs`'s writers as production does.
- 🔵 suggestion/low — Phase 3 §1 — Assert the follow-up preview positively.
- 🔵 suggestion/low — Phase 2 — Assert stream routing for one warning.

### Correctness

**Summary**: The writer split, the `..` fix and progress-based classification are correct in outline, but three planned behaviours cannot hold: the `Permission denied` assertion, convergence-based dossier removal, and `SkipConflict` retention.

**Strengths**: `unresolved_tail` catches every post-missing `..` before `create_dir_all`; `ItemProgress` is scoped per item; `--resolve` keeps `state: Conflict`, so status can be derived after apply; exhaustive totality; `Faults { left: 1 }` correctly targets the per-item `set`.

**Findings**:
- 🟡 major/high — Phase 4 §4 — `NotWritable` drops the OS message, so the `Permission denied` assertion cannot pass.
- 🟡 major/high — Phase 5 §3 — No classification path clears a converged conflict (`classify.rs:125-133`).
- 🟡 major/high — Phase 5 §2 — `conflicted()` excludes `SkipConflict` (`decide.rs:108-113`) and `Indeterminate`, so live dossiers are retired.
- 🔵 minor/medium — Phase 4 §2–3 — Several error sites are unclassified, including `ReuseId` post-link steps.
- 🔵 minor/medium — Phase 5 §4 — A failed rewrite leaves a stale dossier that looks current.
- 🔵 minor/low — Phase 3 — The `unwrap_or_default` read writes empty frontmatter now that pulls land.
- 🔵 suggestion/low — Phase 4 §3 — Record `PushRefused` → terminal as an explicit exception.

### Safety

**Summary**: Overall a safety improvement: the containment escape closes, exits become non-zero, and dossier deletion is narrowed. But Phase 3 is the first point at which pulls overwrite tracked files, and that path has no lock, no staleness check and no guard against an empty read.

**Strengths**: fail-closed `..` refusal before `create_dir_all`; mutual writer refusal; retire-after-write and never retiring conflicted ids; the `.gitignore` gate stays; retryable/terminal avoids a blind re-run; a failed `Created` marker keeps `Attempted`; permission tests restore `0o755`.

**Findings**:
- 🟡 major/high — Phase 3 §4–5 — Pulls overwrite without the `ForFile` lock or a digest re-check.
- 🟡 major/medium — Phase 4 table — An unreadable local file is replaced with an empty-frontmatter file.
- 🔵 minor/high — Phase 5 §2 — A resolution of a dirty `RemotelyModified` item loses its dossier even on failure.
- 🔵 minor/medium — Phase 5 §4 — Whole-corpus runs retire dossiers for `Indeterminate`/`RemoteAbsent`/`SkipConflict` items.
- 🔵 minor/medium — Phase 6 — Terminal failures have no recovery path; the phantom conflict on the next run risks pushing a degraded body.
- 🔵 minor/medium — End State / Phase 5 §4 — Deletions, the `.gitignore` write and marker removal bypass the bounded writers.
- 🔵 suggestion/medium — Phase 3 — Keep a recovery copy before a resolution pull over a dirty file.
- 🔵 suggestion/low — Migration Notes — The 0296 release gate is procedural only.

### Security

**Summary**: Sound, and it shrinks the write surface. One gap: both writers are built with `project_root == permitted_root`, which disables the symlinked-root guard just as tracker content starts being written into tracked files.

**Strengths**: fail-closed `..` refusal tested across shapes; writers tested to refuse each other's roots and `..` escapes; the fail-closed `.gitignore` check is kept; retirement matches only id-shaped names.

**Findings**:
- 🔵 minor/medium — Phase 3 — `FileCorpusStore::new(root)` skips the project-root check (`corpus-adapters/src/store.rs:48-53`); bound both writers by `repo_root`, and test with `meta/work` as a symlink.
- 🔵 minor/medium — Phase 3 §5 / Phase 5 §4 — Dossiers move to a broader root, leaving `id_is_token_safe` as the only guard; keep it explicitly and test it.
- 🔵 suggestion/low — Phase 4 §5 / Phase 6 — Tracker detail now reaches stderr and the model's context; strip control characters and treat it as data.

### Compatibility

**Summary**: Consumer contracts are handled well: the breaking change is named, precedence is kept, the TSV shape is stable and the golden fixture is pinned. The gaps are Phase 1's reach into all `store` consumers and the `PushRefused` stderr shape.

**Strengths**: the Breaking changelog entry and migration notes; unchanged precedence, with the skill line corrected; detail-token-only change, with the golden fixture pinned; the tracker pin; workspace-internal API changes with listed call sites; Phase 2 gated on subprocess goldens; the 0296 release gate.

**Findings**:
- 🟡 major/medium — Phase 1 — The containment change affects `read_within`, permitted-root resolution, and config-, migrate-, research- and corpus-adapters; audit them, and add a changelog line or narrow the scope.
- 🔵 minor/high — Phase 4 §3 / Phase 6 — The `PushRefused` format contradicts the contract and changes today's `create failed:` shape.
- 🔵 suggestion/medium — Phase 6 §3 — The changelog omits the new dossier lifecycle.

### Usability

**Summary**: A large improvement for operators: a class on every row, an `error:` stderr line and non-zero exits. The weak spots are terminal-row recovery guidance and consistency across the documented surfaces.

**Strengths**: a class on every row, with id/operation/cause on stderr; the skill handles failure rows whatever the exit code; the retryable/terminal split matches operator mental models; a visible skip reason under root; the breaking change is called out.

**Findings**:
- 🟡 major/high — Phase 6 — Terminal rows give no recovery path; add a recovery table per operation.
- 🔵 minor/high — Phase 4 / Phase 6 — The documented stderr shape does not match `PushRefused`.
- 🔵 minor/medium — Phase 6 Step 5 — "Gate the closing summary" could hide successes; always print the groups.
- 🔵 minor/high — Phase 6 §2 — The docs guide has no exit-code prose; add `needs-retry`/`needs-attention` to step 6.
- 🔵 minor/medium — Not Doing / Phase 5 — The skill template omits `--target`, so the manual check cannot exercise the fix.
- 🔵 minor/medium — Phase 6 §3 — The changelog should warn that exit 4 can mask a retryable failure.
- 🔵 suggestion/medium — Phase 2 / Phase 4 — Follow `run`/`run_with`; define both writer factories up front.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-11T08:58:54+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness / Architecture / Safety / Test Coverage**: `conflicted()` excludes `SkipConflict`, so its dossiers are retired — Resolved (`ConflictStatus::Unobserved`)
- 🟡 **Correctness**: no classification path makes a converged conflict non-conflicted — Partially resolved (the `Converged` state exists, but see the new `local_body` findings)
- 🟡 **Correctness**: the `Permission denied` assertion cannot pass — Partially resolved (store `NotWritable` carries the OS text, but the new `ForFile` lockdir fails first with a detail-less `LockError::NotWritable`)
- 🟡 **Safety**: pulls overwrite without the `ForFile` lock or a staleness check — Partially resolved (the lock is resolved; the recheck digest is often never computed)
- 🟡 **Safety / Correctness**: an unreadable local file is replaced with empty frontmatter — Resolved
- 🟡 **Test Coverage / Correctness**: the classification table omits terminal call sites — Resolved
- 🟡 **Test Coverage**: `hash-local` is testable, and the totality test does not cover it — Partially resolved (the pull `hash-local` row is now unreachable)
- 🟡 **Test Coverage**: the `Fake` mechanics cannot produce several table rows — Partially resolved (the faults are assigned to `sync_apply.rs`'s `Fake`, but the rows live in `sync_create.rs`)
- 🟡 **Architecture**: failure class depends on which constructor each call site picks — Resolved (`ItemApply`)
- 🟡 **Code Quality**: `run_sync` keeps growing — Resolved
- 🟡 **Compatibility**: the store containment change affects reads and all consumers — Resolved
- 🟡 **Usability**: terminal rows give no recovery path — Partially resolved (the table is keyed on context the skill cannot see)
- 🔵 Security (3), Compatibility (2), and most Code Quality, Test Coverage, Usability and Safety minors — Resolved
- 🔵 **Code Quality**: seam surface grows piecemeal — Partially resolved
- 🔵 **Safety**: the 0296 release gate is procedural only — Still present (deliberately skipped: a red gate test would break per-phase green)

### New Issues Introduced

#### Major

- 🟡 **Safety / Architecture / Correctness / Usability**: pull recovery copies have no viable lifecycle. `mark_completed` errors when no `RESTORE-PENDING` marker exists (`recovery.rs:150-162`). `sweep_recoveries` deletes every `Completed` directory at the start of the next sync (`retirement.rs:570`). Reporting and sweeping look only at `retirement-recovery`, and print retirement wording. `prepare` (which writes the fail-closed ignore rule) is never called before `write_once`. The epoch-second directory name collides when two runs start in the same second.
- 🟡 **Correctness**: the `local_body` recipe cannot equal `remote_body` for real files. The body after the closing fence starts with a blank line, and `trim_lines` strips only trailing ones. The projected remote body also carries a title line. Convergence would then fire only in tests built by hand, and the documented recovery path through `converged` would never trigger.
- 🟡 **Correctness**: a `local_body` error aborts the whole sync. `classify`'s `?` propagates out of `plan()` into `RunError::Internal`, so one malformed conflicted file blocks every item.
- 🟡 **Correctness / Test Coverage**: the recheck digest is often vacuous. `LazyItemDigests` is dropped before apply, and `classify` skips `local()` through the mtime short-circuit or a blanked `local_hash`. That is exactly the case of a resolution pull over local edits. No test would catch a recomputed digest.
- 🟡 **Correctness**: the `ForFile` lockdir fails before the write under `chmod 0o555`, so the permission test hits `lock-local` with no OS detail.
- 🟡 **Correctness**: `WholeCorpus` retirement deletes dossiers of items the `Settled` selection deferred (unsettled, promoting, no fact this pass), because "absent from the report" does not mean "not a live conflict".
- 🟡 **Architecture / Correctness**: `ConflictStatus` leaves a `NotApplied` (preview) resolution unmapped, and dossier persistence also runs in preview, so `--preview --resolve` can retire a live dossier.
- 🟡 **Compatibility**: convergence collides with the frozen, hash-pinned classify oracle (`work-item-sync-classify.json`, identical bodies → `Converged`), and `sync_classify.rs` is pinned at one `#[test]` while the plan adds three tests there.
- 🟡 **Test Coverage / Correctness**: the pull `hash-local` row is unreachable, because a successful split guarantees that the rebuilt content has fences.
- 🟡 **Test Coverage**: the fault capabilities are assigned to the wrong double. `sync_create.rs`'s `Spy` and `RecordingAuthor` cannot fail, and only the `Unreadable` `RefusalReason` is seeded.
- 🟡 **Test Coverage**: the pull's read path is unspecified (no reader port, `std::fs` against the in-memory fake), so "file untouched" can pass with the guard missing. The lock-order log needs a `Send` shared log.
- 🟡 **Usability**: the terminal recovery table is keyed on context the stderr line lacks, and it promises convergence for cases where the file is missing or malformed.

#### Minor

- 🔵 **Safety**: `Dirtiness::Unknown` is not mapped to keeping a recovery copy.
- 🔵 **Safety / Correctness**: convergence rebaselining at finalise can absorb a concurrent edit or a pending title change (bodies are compared, but push also sends the title).
- 🔵 **Correctness / Test Coverage**: it is unstated whether a recovery copy counts as `landed()`, and no test pins it.
- 🔵 **Architecture**: recovery copies and lockdirs break the "exactly one of two roots" invariant as stated.
- 🔵 **Architecture / Code Quality**: `ConflictStatus` and `awaiting_human` derive the same question independently; `build_dossiers` and `dossier_holders` filter differently.
- 🔵 **Architecture**: "`FileExclusiveLock`" does not exist (it is `LockdirLock`); the lock owner should come from `IdentityWorkspace`.
- 🔵 **Compatibility**: `work list` labelling of `Converged` is unspecified (`label.rs` exhaustive `TryFrom`); public-api snapshots for `store`, `corpus` and `work` are not named; the `LockError::NotWritable` detail is unspecified.
- 🔵 **Code Quality**: `ItemApplier::new` takes eight positional ports, two of the same type; `SyncStores::bounded` takes the whole `SyncSeams` and same-typed paths; `landed()` is still per-site discipline (an `apply.mutate(..)` wrapper would make it structural).
- 🔵 **Test Coverage**: status table lacks failed plain pull/push → `NotInConflict`; terminal `baseline-set` recovery is never exercised by a second run.
- 🔵 **Usability**: the marker recovery row gives no path; converged items vanish silently; the new operation labels have no specified cause text; `keep-recovery` may loop as retryable; no opt-out for root containers.
- 🔵 **Security**: pull recovery copies are not guaranteed an ignore rule (same root cause as the lifecycle major).

### Assessment

The first pass's structural problems are fixed. Every lens confirmed the classification model (`ItemApply`), `ConflictStatus`/`Unobserved`, the bounded project-rooted stores, removals routed through `FileRemove`, the `run_sync_with` seams and the store-wide containment fix. Security and the earlier compatibility findings are fully resolved.

The REVISE verdict now comes almost entirely from the three mechanisms added in revision, which were designed without close enough reference to the code:

1. **Recovery copies**: they need a lifecycle of their own (a separate parent, `prepare` first, no automatic sweep, a pull-specific notice, a unique directory name, and `Unknown` treated as dirty).
2. **Convergence**: it needs a `local_body` recipe that drops leading blank lines and agrees with the title projection, fails safe to `Conflict` on error, rebaselines from the classification-time digest, includes the title, and leaves the frozen oracle untouched (a sentinel in `FakeDigests` and a new test file).
3. **The recheck digest**: it needs a defined source carried from planning to apply (the baseline `local_hash` for a clean pull; a digest captured at planning for a resolution), with a race test.

The remaining items are mechanical: make the lock detail-bearing or drive the permission test through the write; map `NotApplied` to `AwaitingResolution` and skip retirement in preview; base `WholeCorpus` authority on the reconciled set; drop the unreachable `hash-local` row; specify the doubles and the read path. One more revision pass should be enough.

## Re-Review (Pass 3) — 2026-10-11T09:46:22+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Safety / Architecture / Correctness / Usability**: pull recovery copies have no viable lifecycle — Partially resolved (they now have their own parent, `prepare` runs first, there is a 7-day sweep, a pull-specific note, a unique directory name, and `Unknown` counts as dirty; but a new listing-overwrite defect was found, see below)
- 🟡 **Correctness**: the `local_body` recipe cannot match real files — Resolved (payload comparison, blank lines dropped, title split)
- 🟡 **Correctness**: a `local_body` error aborts the whole sync — Resolved for convergence; the same failure is reintroduced by the planning digest (see below)
- 🟡 **Correctness / Test Coverage**: the recheck digest is often vacuous — Resolved (`PlannedAction.local_digest`, with a race test)
- 🟡 **Correctness**: the lockdir fails before the write with no OS detail — Resolved (`LockError::NotWritable` carries the detail)
- 🟡 **Correctness**: `WholeCorpus` retires the dossiers of deferred items — Resolved (`corpus_ids`)
- 🟡 **Architecture / Correctness**: `ConflictStatus` leaves a preview resolution unmapped — Partially resolved (now mapped, but the mapping contradicts the invariant test that pins it; see below)
- 🟡 **Compatibility**: convergence collides with the frozen classify oracle — Resolved
- 🟡 **Test Coverage / Correctness**: the pull `hash-local` row is unreachable — Resolved
- 🟡 **Test Coverage**: fault capabilities are on the wrong double — Partially resolved (`Fake::on_update`, `Fake: FileRemove` and the fourth `RefusalReason` seed are still missing)
- 🟡 **Test Coverage**: the pull read path is unspecified — Resolved
- 🟡 **Usability**: the recovery table relies on context the line lacks — Partially resolved (it is now keyed on state plus operation, but see below)
- 🔵 Every security finding — Resolved
- 🔵 The other compatibility minors (label, public-api snapshots, lock detail) — Resolved
- 🔵 Code-quality minors (positional ports, `SyncStores::bounded` signature, per-site `landed()`) — Resolved; the tracker-mutation entry point is still unspecified
- 🔵 **Safety**: `Dirtiness::Unknown` — Resolved
- 🔵 **Safety / Correctness**: convergence rebaselining absorbs edits — Mostly resolved; a narrow two-read window remains
- 🔵 **Architecture**: the two-root invariant — Partially resolved; the identity-settlement stores are not mentioned
- 🔵 **Usability**: converged items vanish silently — Still present
- 🔵 **Safety**: the 0296 release gate is procedural only — Accepted (a deliberate decision)

### New Issues Introduced

#### Major

- 🟡 **Architecture / Compatibility / Correctness / Test Coverage**: `local_digest = item.digests.local()?` sits inside `plan()`. A clean pull target that is unreadable or has no frontmatter fence (classification skips hashing it today) now aborts the whole run with `RunError::Internal`, and `list.rs:209-216` drops every item's `work list` label. The applier's per-item `read-local` guard can no longer be reached end to end, and no test covers the case.
- 🟡 **Architecture / Safety / Correctness**: each pull calls `mark_restore_pending(dir, [path])` on a recovery directory shared across the run, and `fs::write` replaces the listing each time (`recovery.rs:134-144`). After two dirty pulls only the last path is listed, so the sweep can delete the first copy. A failure between `write_once` and the marker leaves a directory that `kept()` never reports or sweeps.
- 🟡 **Correctness**: a preview resolution maps to `AwaitingResolution`, but `--resolve` has already turned its action into `Pull`/`Push`, so `awaiting_human` is false. The pinned test `every_awaiting_resolution_item_is_awaiting_human` therefore cannot pass. Making it pass would change the preview's exit code to 4.
- 🟡 **Test Coverage**: `a_converged_item_is_rebaselined_from_its_planning_digests` injects its edit through the lock-fake hook, but a `Noop` item never takes a lock. A re-read at finalise would still pass the test.
- 🟡 **Test Coverage**: Phase 3's end-to-end `a_dirty_pull_reports_where_the_local_edits_were_kept` cannot see a pull land until Phase 4. It either pins a "kept" note next to a failed row, or stays red.
- 🟡 **Usability**: the `remotely-modified`/`conflict` + `baseline-set` recovery row assumes a pull. A `--resolve <id>=local` push produces the same row, and the advice `=remote` would overwrite the user's chosen side.
- 🟡 **Usability**: when the `Created` marker write fails, the marker recovery row leaves an `Attempted` marker that names no issue, although the issue was created. The row's "no issue → delete marker and sync" advice then creates a duplicate.

#### Minor

- 🔵 **Code Quality / Architecture / Correctness**: `ItemApply` has no primitive for tracker mutations (`mutate` would collapse their class into `Io`).
- 🔵 **Correctness**: `finalise_run` cannot set hashes or create entries, which the converged rebaseline and the first-sync case need. Extend it atomically.
- 🔵 **Correctness**: `local_payload` and `local()` hash separate reads, a narrow window in which an edit can be absorbed.
- 🔵 **Correctness**: the `ConflictStatus` table is not total (no rows for `Noop`, `Unsynced`, or the create rows). Make it an exhaustive match.
- 🔵 **Correctness**: `split_projected` is private to `work-cli`, and structured (Jira ADF) descriptions never converge. Neither point is in Migration Notes.
- 🔵 **Safety**: the 7-day expiry deletes copies silently, runs under `--preview`, and depends on the clock. A future or unparsable epoch is not handled.
- 🔵 **Safety**: a failed `mark_restore_pending` does not stop the pull.
- 🔵 **Safety**: a `Clean` result treats VCS-ignored files as recoverable.
- 🔵 **Safety**: a conflicted file that is temporarily malformed drops out of `corpus_ids`, so its dossier is retired as an orphan.
- 🔵 **Architecture**: `ResolutionFailed` ignores progress, so after a terminal failure the rebuilt dossier is misleading.
- 🔵 **Architecture**: dossier policy is split between `work-adapters` and `work-cli`; classification evidence travels by two carriers (`PlannedAction`, `PreparedRun`).
- 🔵 **Compatibility**: `NeverCalled`/`Agreeing` in `sync_plan.rs` also implement `ItemDigests` (and `Agreeing` could flip to `Converged`).
- 🔵 **Compatibility**: the `NotWritable` ripple misses `lock.rs:710-715,746` and `corpus/src/store.rs:198-203`, and the `WriteError` wording changes.
- 🔵 **Compatibility**: `PlannedAction` literals outside `plan.rs` are not listed; the variant arrays in `decide.rs`/`sync_label_parity.rs` are not listed either.
- 🔵 **Code Quality**: `guarded_local` hides a recovery write; the final `pull` body with `ItemApply` is not shown; `ItemApply` and `ItemApplier` are too close as names; `StoreRoot`'s vocabulary is inconsistent; `retire_settled_dossiers` also retires orphans; the location of the recovery module is unclear.
- 🔵 **Test Coverage**: existing `sync_apply.rs` pull tests need rewriting; recovery ordering relative to lock and recheck is not pinned; recovery edge cases (marker failure, the 7-day boundary, partial settle, the date) are untested; the deferred-item setup cannot be built; a skill-content check is suggested.
- 🔵 **Usability**: "restore from version control" does not fit new or freshly linked files; kept-edits notes repeat for 7 days without restore steps; "kept for 7 days" overstates what happens for committed edits; converged items are still silent.
- 🔵 **Security**: `note:` lines skip sanitisation and the treat-as-data framing; the marker row acts on a path taken from stderr without bounds or confirmation.

### Assessment

The plan is converging. The structural issues from passes 1 and 2 hold up, all security and compatibility findings are resolved, and none of the new majors calls the architecture into question. Each one is narrow:

1. **Planning digest:** make the digest per-item and fallible (for example `PlannedLocal::{Digest, Unreadable}`), so the applier fails the row with `read-local`.
2. **Recovery listing:** give each pulled item its own recovery directory (or accumulate the listing), and write the marker before `write_once`.
3. **Preview invariant:** decide whether a preview resolution counts as awaiting a human. This changes the exit code, so it needs a decision.
4. **Converged race test:** fire the hook from a second, pulled item.
5. **Phase 3 test:** test the report function directly in Phase 3, and move the end-to-end assertion to Phase 4.
6. **Recovery table, push direction:** state which direction the `baseline-set` failure followed in its cause, and key the row on it.
7. **Duplicate issue:** put the created key in every post-create cause, and tell the user to search the tracker before deleting a marker.

Items 3, 6 and 7, plus the minors on expiry, VCS-ignored files and malformed-file orphans, need decisions. The rest is mechanical.

## Re-Review (Pass 4) — 2026-10-11T15:15:43+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, test-coverage, safety, usability (security, code-quality and compatibility had no pass-3 majors).

### Previously Identified Issues

- 🟡 **Architecture / Compatibility / Correctness / Test Coverage**: a planning `local()?` aborts the run — Resolved for the pull digest (`PlannedLocal`); the underlying gap is that unreadable files vanish before discovery (see below)
- 🟡 **Architecture / Safety / Correctness**: the shared recovery listing is overwritten — Resolved (per-item directories)
- 🟡 **Correctness**: a preview resolution contradicts `awaiting_human` — Resolved (`ResolutionPending`)
- 🟡 **Test Coverage**: the converged race hook never fires — Resolved
- 🟡 **Test Coverage**: the Phase 3 end-to-end kept-note test — Resolved
- 🟡 **Usability**: the `baseline-set` row assumes a pull — Resolved (cause prefix)
- 🟡 **Usability**: the `Created`-marker failure risks a duplicate issue — Resolved (`created <KEY>; `)
- 🔵 Safety minors (expiry, a failed `mark_restore_pending`, VCS-ignored files, malformed-file orphans, marker path from stderr) — Resolved
- 🔵 Correctness minors (`finalise_run`, two-read window, `ConflictStatus` totality, `split_projected`) — Mostly resolved; see below for the `ConflictStatus` match key and tracker progress
- 🔵 `ItemApply` has no tracker-mutation primitive — Resolved (`mutate_tracker`); a read-only tracker call is still uncovered (see below)
- 🔵 Converged items vanish silently — Resolved (`conflict settled` note)
- 🔵 **Safety**: the 0296 release gate — Accepted

### New Issues Introduced

#### Major

- 🟡 **Correctness / Test Coverage**: unreadable or unidentifiable work files never reach discovery. `markdown_files_in` drops files it cannot read (`filesystem.rs:87-93`) and `identity_of` drops fence-less ones, so `Discovery` reports itself complete and a full run retires their dossiers as orphans. The "invalid-UTF-8 before push" end-to-end case produces no row and exits 0. The end-state and changelog claim that "one unreadable work file fails only its own row" is false in production. `WorkItemFiles` must report unreadable paths too.
- 🟡 **Correctness**: under `--push-only`, a dirty `RemotelyModified` item decides `Noop` (`decide.rs:91-92`), which maps to `NotInConflict`, so its live `Prompt` dossier is retired (`--pull-only` keeps it as `SkipDirty`). The `conflict_status` match key `(Action, SyncState)` cannot express resolution, outcome or dirtiness.
- 🟡 **Architecture**: `ItemAttempt` has no primitive for a read-only tracker call. `show` in `create_from_remote` either loses its tracker class (`check`) or marks the item as partially applied (`mutate_tracker`), which makes a later `author-local` failure terminal.
- 🟡 **Architecture / Correctness**: the `RecoveryCopies` port cannot enumerate `sync-recovery/*` or recover the original of a directory that has no record. The planned sweep needs a new port method (a `corpus` public-API change that is not listed). Parsing the id back out of `<epoch>-<suffix>-<id>` needs a suffix with no `-` and an id that is safe to use in a path.
- 🟡 **Architecture / Correctness**: nothing marks a description as structured (`RemoteIssue.body` is a plain `String`), so `RemotePayload::NotComparable` would rest on content sniffing across a crate boundary.
- 🟡 **Usability**: the skill derives the marker from the slug, but sync writes `pending-push/<id>.json` (`run.rs:832-836`); only `work create` uses a slug. The plan's Current State repeats the error.
- 🟡 **Usability**: the recovery rows split on "after authoring" versus "after linking", but no `read-local`/`hash-local` cause carries that prefix, so the skill may advise deleting a pre-existing work item.
- 🟡 **Usability / Safety**: an `AlreadyWritten` refusal can lead the skill to offer linking a key that another item already carries. Refusal reasons have no stable token, and the existing CLI remedy text contradicts the skill's inspect-only rule.
- 🟡 **Usability**: the skill and guide present `retryable` as "just re-run", but `read-local`, `lock-local`, `write-local` and `keep-recovery` will fail the same way until the user fixes the cause.

#### Minor

- 🔵 **Safety**: no test pins that a blanked or missing baseline `local_hash` keeps a copy (make it `Option`).
- 🔵 **Safety**: a partially applied resolution failure keeps an outdated dossier without flagging it as stale.
- 🔵 **Safety**: `copy_settled` is undefined for a missing or renamed item file.
- 🔵 **Correctness**: `ApplyError::Tracker` has no defined progress, so a terminal resolution-push failure may rebuild its dossier.
- 🔵 **Test Coverage**: Phase 3 asserts a `retryable` class that only exists from Phase 5.
- 🔵 **Test Coverage**: `sync_plan.rs` cannot call `digest::local`, and no existing double short-circuits on mtime.
- 🔵 **Test Coverage**: the deferred-item failure mechanism and `report.reported` observation are unspecified.
- 🔵 **Test Coverage**: refresh-versus-keep assertions can't tell the two apart (change the remote between runs).
- 🔵 **Test Coverage**: the doubles needing `FileRemove` are under-listed (`Fake`, `RealWrite`, `FailingWrite`, `sync_run.rs` `Spy`).
- 🔵 **Test Coverage**: the 7-day tests have no stated clock source.
- 🔵 **Architecture**: `Discovery` collides with `tracker::Discovery`; its shape is undefined.
- 🔵 **Architecture**: per-action evidence is held in optional `PlannedAction` fields plus `PreparedRun`.
- 🔵 **Architecture**: `finalise_run` grows to five positional parameters (use `RunSettlement`).
- 🔵 **Usability**: the prefix order when both `created <KEY>; ` and `after link: ` apply is undefined.
- 🔵 **Usability**: after manual linking, the leftover marker is unaddressed.
- 🔵 **Usability**: summary groups overlap, `(age unknown)` has no group, and `needs-retry` already exists.
- 🔵 **Usability**: the restore-by-copy instruction overwrites the remote changes that were just pulled.
- 🔵 **Usability**: the skip printed by `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS` is hidden by cargo's output capture.
- 🔵 **Test Coverage**: add a pytest check of the skill's content (suggested again).

### Assessment

The plan is close to done, but the run-level mechanics are not yet finished. Every pass-3 major is resolved, and no lens questions the architecture. The nine new majors fall into three groups:

1. **The CLI's view of the corpus**: files that cannot be read or identified must be reported, not dropped. `WorkItemFiles` needs to return them, `Discovery` (renamed, for example to `CorpusInventory`) needs to count them, and they must surface as rows. Push-only dirty prompts must count as `Unobserved`.
2. **Interfaces at crate boundaries**: `ItemAttempt::observe_tracker`, a `RecoveryCopies` directory-listing method, and an explicit description kind on `RemoteIssue` (or dropping `NotComparable` in favour of byte equality).
3. **The skill's recovery contract**: the marker path `pending-push/<id>.json`, `after author: `/`after link: ` prefixes on read and hash causes, reason tokens on refusals with an `already-written` branch, and retryable advice keyed on the operation.

Group 1, and the structured-description choice in group 2, need decisions; the rest is mechanical.

## Re-Review (Pass 5) — 2026-10-11T20:45:20+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, test-coverage, safety, usability.

### Previously Identified Issues

- 🟡 **Correctness / Test Coverage**: unreadable or unidentifiable work files never reach discovery — Partially resolved (`WorkItemListing`/`CorpusInventory` fix the design; the other `WorkItemFiles` callers and the routing into `RunReport` are unspecified, see below)
- 🟡 **Correctness**: `--push-only` dirty `RemotelyModified` retires a live dossier — Resolved
- 🟡 **Architecture**: no read-only tracker primitive — Resolved (`observe_tracker`)
- 🟡 **Architecture / Correctness**: `RecoveryCopies` cannot enumerate `sync-recovery/*` — Resolved (`directories`, base32 suffix); `id_is_token_safe` reachability is a new minor
- 🟡 **Architecture / Correctness**: `NotComparable` content sniffing — Resolved (byte equality)
- 🟡 **Usability**: marker derived from slug — Resolved
- 🟡 **Usability**: no `after author`/`after link` prefix — Resolved
- 🟡 **Usability / Safety**: `already-written` could offer linking; no reason tokens — Resolved
- 🟡 **Usability**: retryable presented as "just re-run" — Resolved (minor gap: no `hash-local` row or fallback)
- 🔵 Safety minors (`Option` baseline hash, stale dossier warning) — Resolved; `copy_settled` for a missing file — Partially resolved (see below)
- 🔵 Test-coverage minors (Phase 3 class, `ShortCircuited`, refresh assertions, `FileRemove` doubles, clock, skill pytest) — Resolved; deferred-item observation — Partially resolved
- 🔵 Architecture minors (`finalise_run` → `RunSettlement`, `ApplyError::Tracker` progress) — Resolved; `Discovery` naming — Partially resolved
- 🔵 Usability minors (prefix order, leftover marker, merge restore, root-guard banner) — Resolved; summary groups — Partially resolved
- 🔵 Evidence-carrier unification — Declined (invariants documented)
- 🔵 **Safety**: the 0296 release gate — Accepted

### New Issues Introduced

#### Major

- 🟡 **Correctness**: unclassifiable items are reported with state `indeterminate`, which `RunReport::awaiting_human` (`run.rs:307-317`) counts regardless of outcome. A lone local read failure therefore exits 4, not 70. The synthetic `Action` is unnamed and has no `conflict_status` row.
- 🟡 **Architecture / Correctness**: changing `WorkItemFiles::files()` to return `WorkItemListing` reaches nine non-sync callers (`list`, `next_number`, `promote`, `resolve`, `create_batch`, `sync_author`, `creation.rs` twice) with no stated policy, several of them uniqueness guards.
- 🟡 **Architecture / Correctness**: the route from `CorpusInventory` through `CorpusDiscovery`/`DiscoveredCorpus` and `run_settled` (up to three discoveries, identity pass renaming ids) into `RunReport.unidentified` and `DossierAuthority::WholeCorpus` is unspecified; `LocalItem` has no content field for `LazyItemDigests`.
- 🟡 **Usability**: `needs-attention` is defined as terminal rows "plus the groups above", so informational kept-edits and settled notes would block "sync complete" for up to 7 days.
- 🟡 **Usability**: unreadable files and kept-edit paths exist only as free-text stderr, contradicting the skill's "stdout is authoritative" contract and its "never open a path taken from stderr" rule.

#### Minor

- 🔵 **Safety**: the sweep reuses `copy_settled`, which returns `true` when the copy cannot be read, so an unsettled copy can be deleted before 7 days.
- 🔵 **Safety**: linking checks only that no *other* item carries the key, not the target item's own `external_id`.
- 🔵 **Correctness**: `id_is_token_safe` is private to `work-cli` (`sync.rs:522`) and needs a scheme the applier lacks.
- 🔵 **Correctness**: `already-written` cannot always name the carrier (`corpus_carries` is a boolean and also covers identity-followed keys).
- 🔵 **Correctness / Usability**: unidentified files raise exit 70 on targeted and preview runs; a README in the work directory would fail every sync.
- 🔵 **Correctness**: no `(Noop, Unsynced)` status row; impossible tuples have no stated default.
- 🔵 **Architecture**: four near-identical "cannot read" types; `CorpusInventory` belongs beside `identity_of` in `work`.
- 🔵 **Architecture**: `directories()` overlaps `kept()`; pull copies borrow retirement's restore-pending vocabulary.
- 🔵 **Architecture**: no home for the reason-token mapping (`RefusalReason::token`).
- 🔵 **Usability**: "a conflicted run exits 4 even when items failed" is wrong for terminal failures (71); `1` undefined.
- 🔵 **Usability**: post-push `read-local`/`hash-local` lack an `after push:` prefix and fall to "restore from version control"; no terminal `update` row; create-from-remote `baseline-set` prefix unstated.
- 🔵 **Usability**: retryable table has no `hash-local` row and no fallback.
- 🔵 **Usability**: `fingerprint-mismatch` has no way forward.
- 🔵 **Usability**: kept-edit note shapes inconsistent and prefix-ambiguous; earlier copies have no paths.
- 🔵 **Test Coverage**: the deferred-item "no row for `a`" assertion contradicts the identity-refusal row.
- 🔵 **Test Coverage**: run-level unreadable fixtures fall into the unclassifiable path, not the sites they name.
- 🔵 **Test Coverage**: the crash/re-run test reads a real file but writes to the `Fake`, so the recheck passes trivially.
- 🔵 **Test Coverage**: the read-only-directory test expects `lock-local` where the acceptance criterion names `write-local`; the deviation is unrecorded.
- 🔵 **Test Coverage**: artefact and non-canonical survivors move into a pure `may_retire` test that cannot observe files.
- 🔵 **Test Coverage**: `Fake::on_update` has no path to pass (suggestion).

### Assessment

The architecture and the per-item mechanics are now settled: safety and test coverage found no majors, and every lens confirmed the pass-4 interfaces. The five majors are all at the corpus and report boundary:

1. **Unclassifiable exit code**: exclude failed items from `awaiting_human`, or route unclassifiable items through the path channel.
2. **`WorkItemFiles` ripple**: add a separate listing method for sync, or state each caller's policy.
3. **Inventory routing**: carry the inventory on `DiscoveredCorpus`, taken from the final rediscovery.
4. **`needs-attention` scope**: make the informational groups non-blocking.
5. **Stdout records**: emit unreadable files and kept copies as stdout records.

Items 1, 2, 5 and the exit-70-on-targeted-runs minor need decisions; the rest is mechanical.

## Re-Review (Pass 6) — 2026-10-12T02:10:38+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, usability (safety and test-coverage had no pass-5 majors).

### Previously Identified Issues

- 🟡 **Correctness**: unclassifiable items exit 4 via `awaiting_human` — Resolved (`RunReport.unclassifiable`); its row shape is a new major
- 🟡 **Architecture / Correctness**: `WorkItemFiles` ripple to nine callers — Resolved (`listing()`)
- 🟡 **Architecture / Correctness**: inventory routing through `run_settled` — Resolved; the `list.rs` digest site lacks content (minor)
- 🟡 **Usability**: `needs-attention` blocking on informational groups — Resolved; the completion gate is now too narrow (new major)
- 🟡 **Usability**: path-keyed failures stderr-only — Resolved for paths; settled and stale-dossier ids remain stderr-only (minor)
- 🔵 **Safety**: sweep reuses `copy_settled` — Partially resolved (own predicate specified, but the port cannot read copies; see below)
- 🔵 **Correctness**: `id_is_token_safe` reachability — Partially resolved (target placement does not compile; see below)
- 🔵 **Correctness**: `already-written` carrier — Partially resolved (`carrier_of` on the wrong request)
- 🔵 **Correctness / Usability**: unidentified files on targeted runs — Resolved; id-less Markdown on whole-corpus runs is a new major
- 🔵 Exit 4/71 wording, `after push:` prefix, `update` row, retryable fallback, `fingerprint-mismatch`, note shapes, pre-link `external_id` check, `(Noop, Unsynced)`, reason-token home, `directories`/`kept` — Resolved
- 🔵 Accepted or declined: the 0296 release gate, evidence-carrier unification, non-sync `WorkItemFiles` callers

### New Issues Introduced

#### Major

- 🟡 **Architecture / Correctness**: `WorkItemIdScheme` is defined in `corpus` (`corpus/src/work_item_id.rs:76`), and the predicate needs `work::draft_id::DraftId`. `WorkItemIdScheme::is_token_safe` therefore cannot live in `work`, and putting it in `corpus` would create a cycle. Carrying the scheme on `ApplierPorts` also mixes policy into a ports struct.
- 🟡 **Correctness**: `PullRecoveryRetention`'s own settled predicate cannot be built: `RecoveryCopies` exposes no copy bytes or copy path (`mirrored`/`location` are not on the trait), and the stdout `kept-edits` records need the copy path.
- 🟡 **Correctness / Usability**: a well-formed Markdown file with no `id` (a README or template) counts as `NoIdentity`, fails every whole-corpus sync with a "retryable" 70 that a re-run never clears, is mislabelled `read-local failed`, and permanently disables orphan retirement. `work list` already treats such files as "not a work item" (`list.rs:117-119`).
- 🟡 **Usability / Correctness**: the unclassifiable row `<id>\tfailed\t-\t<reason>` is byte-shaped like an identity refusal (`sync.rs:372-376`), puts free text in the class column, and skips `single_line`.
- 🟡 **Usability**: the "sync complete" gate requires only `needs-retry` and `needs-attention` to be empty, omitting `rejected`/`unconfigured` rows, identity failures, unresolved conflicts, `remote-absent` and `not-found`.

#### Minor

- 🔵 **Architecture / Correctness**: `carrier_of` belongs on `CreateFromLocalRequest`, replacing the boolean `corpus_carries` (derive it as `is_some()`).
- 🔵 **Architecture / Correctness**: `failure_lines` is scope-blind, so targeted runs print `error:` for unrelated unidentified files; one `RunReport::failures(scope)` should feed lines, prefixes and exit code.
- 🔵 **Architecture / Usability**: kept-edits `#\tdetail` records get a second formatter after `#\tsummary`, and change the fifth field's meaning versus the skill's existing detail paragraph; add `DetailSource` variants and amend the paragraph.
- 🔵 **Architecture**: `CorpusScope` and `DossierAuthority` are derived side by side from one selection.
- 🔵 **Architecture**: `files()` could be a provided method over `listing()`; the inventory and local items should come from one identity pass.
- 🔵 **Correctness**: `list.rs`'s `ScannedItem` has no content for `LazyItemDigests`.
- 🔵 **Correctness**: Phase 6's content-based digests make the Phase 3 run-level unclassifiable fixture unreachable.
- 🔵 **Correctness**: hashing discovery content before `run_start_epoch` is read widens a window in which an edit is buried by the watermark.
- 🔵 **Correctness**: a failed `finalise_run` still reports converged items as `Settled` and retires their dossiers.
- 🔵 **Usability**: settled conflicts and stale dossiers reach the skill only through stderr.
- 🔵 **Usability**: `after create:` has no producing site.
- 🔵 **Usability**: the pre-link `external_id` check sits among the reason-token branches, and its lookups are unnamed.

### Assessment

Five majors remain, all narrow. Two are my round-5 edits not matching the code (`is_token_safe` placement, the recovery port lacking a copy read), and are mechanical to fix. The other three need decisions: how to treat id-less Markdown, the unclassifiable row's shape, and the definition of the "sync complete" gate.

## Re-Review (Pass 7) — 2026-10-12T02:26:04+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, usability.

### Previously Identified Issues

- 🟡 **Architecture / Correctness**: `WorkItemIdScheme::is_token_safe` placement — Resolved (free function in `work`); `run.rs` has no scheme or suffix source (minor)
- 🟡 **Correctness**: the recovery port could not read copies — Resolved (`copy_path`, `compare_copy`)
- 🟡 **Correctness / Usability**: id-less Markdown fails every sync — Partially resolved (a README with frontmatter is `no-id`; a fence-less README is still `Malformed`, see below)
- 🟡 **Usability / Correctness**: unclassifiable row shape — Resolved
- 🟡 **Usability**: "sync complete" gate too narrow — Resolved (residual: `needs-attention` and frontmatter validation at exit 0)
- 🔵 `carrier_of`, `failures(scope)`, single stdout owner, `DossierAuthority::for_run`, `files()` provided, `ScannedItem` content, Phase 6 fixture, watermark window, `after create:`, settled/stale records — Resolved (placement gaps below)
- 🔵 Failed `finalise_run` reported as settled — Partially resolved (see the new major)

### New Issues Introduced

#### Major

- 🟡 **Correctness / Architecture / Usability**: the failed-finalise gate keys on `RunReport.finalised`, which is `false` on every preview (`run.rs:1187`) and built from `.is_ok()` (`run.rs:354`), so it has no reason to print; `ReportedItem::conflict_status` cannot see it; it bypasses `RunReport::failures`; it is called terminal although an atomic finalise landed nothing; and the skill has no rule for `error: -:` lines, so it would advise against the re-run that fixes it.
- 🟡 **Correctness**: a Markdown file with no frontmatter at all (an ordinary README) is classed `Malformed` and fails every whole-corpus run; `work list` only warns for it (`list.rs:132-139`).

#### Minor

- 🔵 **Architecture**: `RunReport::failures`, `RunFailure` and `CorpusScope` are placed in `work-cli` but must live in `work-adapters` beside `RunReport` and `DossierAuthority`.
- 🔵 **Architecture / Correctness**: `DetailSource` is defined in `work-adapters/src/promotion.rs`, not `promotion_report.rs`; sync-run sources do not belong in the promotion enum, and kept-edits need a second path field.
- 🔵 **Correctness**: neither `SyncRequest` nor `SyncPorts` carries the id scheme or a recovery suffix source for `run.rs`.
- 🔵 **Correctness**: the sweep has no stated way to identify this run's directories; an `Equal` copy made this run could be deleted.
- 🔵 **Correctness**: the unclassifiable row's skip should use `render_report`'s `settled_ids`.
- 🔵 **Correctness**: on preview `render_report` has no sweep outcome, so carried-over copies vanish from preview output.
- 🔵 **Usability**: `work show --field external_id` exits 1 when the field is absent; the carrier search needs an exact frontmatter-line match.
- 🔵 **Usability**: kept-copy guidance both forbids and offers a copy-over.
- 🔵 **Usability**: completion should also require an empty `needs-attention` and passing frontmatter validation.
- 🔵 **Usability**: resumed-promotion identity rows can carry a bare `rejected` token; pin that no identity detail equals a class keyword and route promotion tokens to the promotion table.
- 🔵 **Architecture**: `copy_settled` should be a provided method over `compare_copy`.
- 🔵 **Usability**: the `no-id` note repeats every run and is absent from the guide (suggestion).

### Assessment

Two majors remain. The failed-finalise handling added in pass 6 was specified against a boolean that cannot carry it; all three lenses converge on a tri-state `Finalisation` routed through `RunReport::failures`. The fence-less README completes the pass-6 id-less Markdown decision. Both need short decisions (the failure class for a failed finalise; how to tell a broken work item from a plain note); the rest is mechanical.

## Re-Review (Pass 8) — 2026-10-12T02:36:36+00:00

**Verdict:** REVISE

Lenses re-run: correctness, architecture, usability.

### Previously Identified Issues

- 🟡 **Correctness / Architecture / Usability**: failed-finalise gate on a boolean — Resolved (`Finalisation` tri-state through `RunReport::failures`, retryable); the `conflict_status` signature is not yet updated everywhere (minor)
- 🟡 **Correctness**: a fence-less README classed `Malformed` — Resolved (`NamesWorkItem`); draft filenames are not recognised (minor)
- 🔵 `failures`/`CorpusScope` placement, `RunDetail`, scheme and suffix source, preview survey, unclassifiable skip via `settled_ids`, `work show` exit 1, exact carrier match, copy-over contradiction, completion gate, promotion-token routing, `no-id` guide bullet — Resolved
- 🔵 Sweep identifies this run's directories — Partially resolved (`kept_recovery` is lost on failed pulls)
- 🔵 `copy_settled` over `compare_copy` — Resolved, with a precedence gap and a placement concern (minors)

### New Issues Introduced

#### Major

- 🟡 **Architecture**: Phase 5 cannot merge green on its own. `a_conflicted_work_item_file_fails_the_run` asserts every dossier is kept, but until Phase 6 `clear_stale_dossiers` (`sync.rs:528-566`) deletes every id-shaped dossier on each `Ok` run. The README test's orphan assertion passes only by accident, and Phase 5's prose relies on Phase 6's `DossierAuthority`.
- 🟡 **Usability**: Phase 7 spreads failure handling over about ten edits to an already ~480-line `SKILL.md`. Handling one `failed` row means cross-referencing five or more sections, and `#\tunidentified` and group membership are defined in several places.

#### Minor

- 🔵 **Architecture**: between Phases 3 and 5, an unclassifiable item exits 0 although its row says `failed`.
- 🔵 **Architecture**: `RefCell<dyn SuffixDraws>` on `SyncRequest` puts a stateful source in a data struct; a `RecoveryLabels` port would fit `SyncPorts`.
- 🔵 **Architecture**: `copy_settled` as a provided port method offers retirement's policy to pull retention; move it into `retirement.rs`.
- 🔵 **Architecture / Correctness**: `conflict_status` is still declared `(&self)` and its match tuple omits the finalisation; prefer `RunReport::conflict_status_of(item)`.
- 🔵 **Architecture**: `run.rs` (1,399 lines) keeps absorbing report types; extract `sync/report.rs`.
- 🔵 **Correctness**: `kept_recovery` is not carried when a pull fails after keeping its copy.
- 🔵 **Correctness**: `compare_copy` must read the copy first, so `CopyUnreadable` takes precedence and `copy_settled` keeps its both-missing behaviour.
- 🔵 **Correctness**: `NamesWorkItem` over `extract_id` does not recognise `draft-xxxxxx-…` filenames; a scan-regex compile failure has no defined outcome.
- 🔵 **Correctness**: record-less recovery directories have no item path for their entries.
- 🔵 **Usability**: a preview says expired copies "were removed".
- 🔵 **Usability**: `needs-retry` rendering for paths and the run-level entry, and the guide's wording for them, are unspecified.
- 🔵 **Usability**: `Malformed` has no fixed cause text, and the advice ignores VCS conflict markers.
- 🔵 **Usability**: a work-item-named file whose `id` line is deleted is informational `no-id`.
- 🔵 **Usability**: the copy-over prompt is not pinned and can revert frontmatter written since the pull.
- 🔵 **Architecture**: suggestions — `sweep` consuming `survey`; a neutral detail-record formatter; `CorpusScope` stored on the report.

### Assessment

The design has converged: correctness found no majors, and every pass-7 major is resolved. The two remaining majors concern the plan's structure, not the design: a cross-phase dependency that moves cleanly into Phase 6, and the shape of the skill's failure guidance, which needs a decision on consolidation.

## Re-Review (Pass 9, final, all lenses) — 2026-10-12T02:56:28+00:00

**Verdict:** REVISE

Lenses re-run: architecture, security, test-coverage, correctness, code-quality, usability, compatibility, safety — each read the whole plan.

### Previously Identified Issues

- 🟡 **Architecture**: Phase 5 depended on Phase 6 retirement — Resolved
- 🟡 **Usability**: skill failure guidance scattered — Resolved (one "Read the run's output" section plus the `sync-work-items-recovery` reference skill)
- 🔵 Every pass-8 minor — Resolved, except `kept_recovery` on failure (carrier undefined, see below) and the stale id-deletion Migration Note
- 🔵 Every earlier safety, security, compatibility and code-quality finding — Resolved or accepted (0296 release gate; evidence-carrier unification and `CorpusScope` on `RunReport` declined; non-sync `WorkItemFiles` callers out of scope)

### New Issues Introduced

#### Major

- 🟡 **Architecture**: Phase 3 relies on Phase 5's `ItemAttempt` to carry `kept_recovery`, and no phase defines how a failed pull hands its kept copy back (`ItemOutcome::Failed(ApplyError)` has no slot).
- 🟡 **Security**: new stdout records interpolate raw filenames, ids and recorded paths without `single_line` or control-character filtering, yet the skill trusts stdout as its only path source; a committed filename with a newline can forge a record.
- 🟡 **Test Coverage**: the `settled` and `stale-dossier` stdout records (and `settled`'s absence under `Finalisation::Failed`) are never asserted.
- 🟡 **Test Coverage**: the skill content check does not pin the retryable-offers-re-run and terminal-never-re-runs instructions the acceptance criteria name.
- 🟡 **Code Quality**: `conflict_status_of`'s exhaustive match over a 6-tuple with every impossible tuple listed is infeasible to write or review.
- 🟡 **Compatibility**: the `work sync --help` contract (`cli.rs:126-140`, pinned in `cli_surface.golden`) still defines 70 narrowly and says to check only `unresolved` lines.

#### Minor

- 🔵 **Stale text**: Desired End State bullets (read-local for all unidentified files, class keyword on every failed row, retirement scope); the id-deletion Migration Note; `SyncPorts` final shape omits `recovery_labels` (four lenses); Phase 4's `id_is_token_safe`; `FailureClass` said to live in `work`; Phase 6's lost-fence test expects `read-local`.
- 🔵 **Security**: pending-push marker paths are built from unvalidated ids (`../last-sync` reaches the baseline); recorded recovery paths are trusted without containment; `Console::err()` bypasses sanitising; widen the treat-as-data rule to opened files and validate key shape.
- 🔵 **Safety**: a concurrent run's sweep can delete another run's fresh copy (take the `ForFile` lock); the durable copy widens the recheck-to-write window (recheck again before the write); VCS conflict markers below the fence are undetected; destructive recovery advice lacks confirmation; the copy-over lacks a missing-target rule; same-second mtimes against a `<=` watermark.
- 🔵 **Correctness**: failed items suppressed by `settled_ids` produce no `failed` row; `SweepOutcome` versus `Survey` as the rendering input, and removal failures unreported.
- 🔵 **Usability**: preview "settled" tense; section placement and a targeted-run membership row; dismissing age-unknown/unrecorded directories; id-to-path derivation; recovery commands unnamed; guide's "never overwritten" bullet; changelog record types.
- 🔵 **Test Coverage**: `an_unclassifiable_conflicted_item_keeps_its_dossier` belongs in pure tests; `KeptEditsUnrecorded` variant and record tests target the wrong unit; `Finalisation` wiring untested until Phase 6; unclassifiable row test belongs in Phase 3; impossible tuples and targeted unclassifiable unexercised; production `NamesWorkItem`/`RecoveryLabels` untested; `copy_settled` test needs a home; `LocalItem.content` needs a helper.
- 🔵 **Code Quality**: two `Survey` types and overlapping "unreadable" names; `render_report` inputs (`FinishedRun`); `ItemAttempt` should own created key, step and kept copy; `ApplyOperation` enum; retention entry names; `Malformed(MalformedCause)`; `is_complete` naming; `ItemProgress` vocabulary; doc comments standing in for constructors; `warn` naming.
- 🔵 **Compatibility**: `printable` may flatten deliberately multi-line warnings; changelog omits finalise 0→70, classification 1→70 and new record types; public-API lists incomplete and name the unpinned `work-adapters`; golden criterion runs the wrong target; containment audit misses the visualiser and tracker caches.
- 🔵 Suggestions: split Phase 3 into safeguards and retention; skip the recovery copy whenever the baseline hash matches, whatever the VCS status.

### Assessment

The design is settled: safety, usability and correctness found no majors, and every lens confirms the architecture. The six majors are all mechanical — a missing carrier, output sanitisation, two test gaps, a match decomposition and a help-text update — and the minors are largely drift from nine revisions. Two suggestions need decisions (the copy-skip VCS gate, a Phase 3 split); everything else can be applied directly. After that revision the plan should be ready for approval without another full pass.

## Approval — 2026-10-12T03:03:33+00:00

**Verdict:** APPROVE

The plan was revised for every pass-9 finding. The six majors were applied as the reviewers proposed: a carrier for the kept copy, sanitised stdout records, the `settled`/`stale-dossier` assertions, the skill's re-run instructions, the decomposed conflict-status decision and the updated `work sync --help` contract. The minors were applied too. Not applied: the safety suggestion to drop the VCS-clean condition on the copy skip (the author's round-3 decision stands, with its rationale recorded in the plan), `CorpusScope` on `RunReport` (declined earlier) and renaming `Console::warn` (cosmetic). Approved without a further pass.
