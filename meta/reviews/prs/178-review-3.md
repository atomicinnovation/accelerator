---
type: "pr-review"
id: "178-review-3"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-15T22:55:57+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:178"
relates_to: ["pr-review:178-review-1", "pr-review:178-review-2"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "documentation", "security"]
review_number: 3
pr_number: 178
tags: []
last_updated: "2026-11-15T22:55:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #178 - [0319] Let work create take an explicit work item ID

**Verdict:** COMMENT

The response to the second review holds up across every lens. Convergence and re-key now share one test of whether an ID is in shape. The trust check lives in one place (`work::corpus_trust`). Every calling skill handles the corpus refusals. The CHANGELOG gives upgraders a path. Security found nothing new, and correctness found no fault in the taken rule, allocation, the grammar or lock ordering.

Three things remain, and none is critical:

1. One last place where convergence and re-key disagree, now confined to per-tracker key rules.
2. Skill-side test coverage: the new exit-4 and fold branches have no evals, and the benchmark guard cannot see that a skill changed after it was graded.
3. A set of small usability and doc-accuracy edges around the new gate.

### Cross-Cutting Themes

- **Re-key reads tracker keys under looser rules than convergence** (flagged by: Architecture, Correctness). Convergence reads a stored tracker key with `TrackerKey::parse_under(key, scheme.tracker_keys())`. Re-key's `why_unclassifiable`, `tracker_mapping` and `keys_to_refresh` (`rekey/mapping.rs:160`, `:306`, `:345`) use `TrackerKey::parse`, which falls back to `AnySupported`.
  - Example: under `{tracker}` with Linear, an item with `id` and `external_id` both `MY_PROJ-7`. Convergence refuses it. Re-key names no fix by hand and leaves the item alone, so the gate never opens.
  - Root cause: `TrackerKey::parse` and `IdScheme::new` default to the permissive rules.
- **Skill branches added for review 2 are unguarded by evals** (flagged by: Test Coverage, Usability). There are no evals for:
  - `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED` from `resolve` or `id check` in `/create-work-item`;
  - the `--from-key` fold dialogue in `/rekey-work-items`, whose "preview again for each key" wording could fold only the last key picked;
  - the new exit-4 branches in `update-work-item` and `review-work-item`.

  The `/create-work-item` benchmark predates the last `SKILL.md` edits, and its guard cannot detect that.
- **The gate's edges leak CLI shape to users** (flagged by: Usability, Documentation, Compatibility).
  - The CLI remedy `accelerator work rekey` applies with no preview.
  - `E_WORK_ITEM_MALFORMED` names one item, and `list` refuses too, so nothing can list the rest.
  - The visualiser banner shows raw CLI text and never says that a pattern error needs a restart.
  - Two docs still say "every" ID command refuses, though `sync` and `rekey` are exempt.
  - `update-work-item` swallows an exit-1 scheme refusal as "unrecognised input".
- **The exit code is still read from message text where the typed refusal is in hand** (flagged by: Code Quality, Test Coverage). The real create's admission arm and `id check`'s `Failed` arm still go through `of_failure`. The eval contract checks against the string mapper too.

### Tradeoff Analysis

- **Single read of the corpus vs simplicity in the visualiser.** Architecture would compute the banner's trust verdict from the indexer, which the file watcher already keeps live. That would avoid a second, blocking filesystem read on every SSE-triggered refetch. The current per-request rescan is simple and is what makes the banner clear without a restart. Recommendation: at minimum run it in `spawn_blocking`, and move it onto the indexer if corpora grow.

### Strengths

- ✅ Review 2's findings are resolved across lenses:
  - one in-shape definition for convergence and re-key (literal case, aligned `id == external_id`, any key);
  - re-key keeps other keys unless `--from-key` names them;
  - one shared `corpus_trust` gate;
  - a `WorkItemScheme` enum in the visualiser;
  - typed `of_requested_id_refusal`;
  - a CHANGELOG Migrations entry with the `sync` escape hatch;
  - every refusing skill handles exit 4.
- ✅ `FoldedKeys::keeps` is defined in terms of `is_converged`, so with nothing folded, re-key moves exactly the items convergence refuses — by construction rather than by parallel logic.
- ✅ Two layers of defence in retirement: an ID-alphabet gate in `plan_retirement`, and one escaping `double_quoted` YAML writer. `RecordedId` and `RecoveryDir` can only be built from parsed, contained values.
- ✅ The grammar compiles no regex from config. Input is bounded and width capped, there is one tokeniser, and a value splits at most one way. The arithmetic is safe throughout (`checked_pow`, `saturating_add`).
- ✅ The visualiser degrades instead of refusing to boot:
  - `problem` is additive and holds paths relative to the root;
  - React renders it as a text node;
  - the endpoint recomputes it per request, and a test proves it.
- ✅ `work id check` has a clean, tab-separated verdict contract that shares codes and exits with `create --id`. The `/create-work-item --id` flow settles the ID before drafting and resumes with the approved draft after a refusal.
- ✅ The Rust behaviour added in response to review 2 is tested at the right levels:
  - the convergence/re-key cross-check;
  - `--from-key` from unit to CLI;
  - the `corpus_trust` ordering;
  - typed exit codes against every `RequestedIdRefusal` variant;
  - the problem endpoint through the real router.

### General Findings

- 🔵 **Architecture**: The general trust gate depends on the re-key subsystem (`corpus_trust` → `rekey::convergence` → `rekey::mapping`). Consider one `work::convergence` module that both the gate and re-key call, so they read one predicate.
- 🔵 **Compatibility**: `work list` now reads the scheme before the missing-directory check. So a bad pattern with no `meta/work/` exits 1, where the skill documents the `paths.work` guidance and exit 0.
- 🔵 **Code Quality**: The "stderr line and exit code" pair is a separate type or variant in each command. One shared `Refusal { line, code }` type with `From` conversions would turn many match arms into `?`.
- 🔵 **Test Coverage**: The skills changed for review 2 (`update-work-item`, `review-work-item`, `list-work-items`, `sync-work-items`, `refine-work-item`) were not re-evaluated. Add an exit-4-from-`resolve` eval wherever a skill has an eval suite.
- 🔵 **Test Coverage**: Which unclassifiable item `E_REKEY_REQUIRED` names is not pinned. No test has two such items, so `min_by` → `max_by` survives.

### Additional Findings

- 🔵 `cli/work/src/rekey/convergence.rs:35` — the same tracker-key disagreement, seen from the convergence side (correctness; merged into the `mapping.rs:160` inline comment)
- 🔵 `cli/visualiser/server/src/server.rs:175` — the live trust check reads the work directory a second way, bypassing the indexer (architecture)
- 🔵 `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:14` — the banner shows raw CLI text and doesn't say a pattern error needs a restart (usability)
- 🔵 `skills/work/sync-work-items/SKILL.md:263` — recovering a draft promotion bounces through three refusals before it reaches `sync` (usability)
- 🔵 `skills/config/configure/SKILL.md:714` — changing `work.key` gives no signal that the old key's items need `--from-key` (usability)
- 🔵 `CHANGELOG.md:341` — a garbled sentence in the `/create-work-item --id` Added entry, plus lines over 80 columns (documentation)
- 🔵 `CHANGELOG.md:876` — the list of scheme-validating commands leaves out `id check` (documentation)
- 🔵 `CHANGELOG.md:1133` — the first Migrations step doesn't cover `E_WORK_ID_PATTERN_INVALID`, which `work list` may give first (documentation)
- 🔵 `cli/visualiser/server/src/indexer.rs:1404` — a comment justifies a filename/`id` mismatch the PR now classes as malformed (documentation)
- 🔵 `skills/work/update-work-item/SKILL.md:65` — every exit 1 from `resolve` is treated as unrecognised input, hiding scheme refusals (compatibility)
- 🔵 `cli/visualiser/frontend/src/api/fetch.ts:82` — the frontend's wiki-link grammar still assumes `<key>-<digits>`, so links to IDs like `PP_0042`, `v001` or tracker keys never resolve (compatibility)
- 🔵 `cli/work-cli/src/create_batch.rs:246` — `AdmittedRequest` has public fields, and `create-batch` builds one by hand, skipping `admit_request` (code-quality)
- 🔵 `cli/work-cli/src/id.rs:99` — `work id check` maps `Failed` through the string classifier, which can only ever return `ERROR` (code-quality)
- 🔵 `cli/work-adapters/src/creation.rs:711` — the `# Errors` doc on `refuse_requested_id` still describes a message, not the typed `RequestedIdBlock` (code-quality)
- 🔵 `cli/visualiser/server/src/server.rs:186` — `unconverged` also reports malformed items, and the doc above it names a parameter that isn't there (code-quality)
- 🔵 `cli/work/src/well_formed.rs:38` — `MalformedItem::what` takes a boolean that switches between two sentence forms (code-quality)
- 🔵 `cli/work/src/rekey/convergence.rs:155` — `refuse_divergence` filters the off-pattern items twice (code-quality)
- 🔵 `cli/visualiser/server/src/api/work_item_config.rs:171` — the configured key is read through the same four-step chain in two places (code-quality)
- 🔵 `cli/work-cli/tests/cli_rekey_required.rs:129` — the sync-exemption test passes however early sync fails (test-coverage)
- 🔵 `tests/unit/tasks/test_untrusted_corpus_refusals.py:16` — the refusing-command list is copied by hand from the CLI, and the test only checks that the code names appear (test-coverage)
- 🔵 `cli/work-cli/src/eval_contract.rs:24` — the eval contract checks only `E_ID_*` refusals against the CLI's exit codes (test-coverage)
- 🔵 `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.test.tsx:34` — no test shows the banner clearing when its query is invalidated (test-coverage)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/work/src/rekey/mapping.rs:160-165` — Re-key reads tracker keys under looser rules than convergence, so some E_REKEY_REQUIRED refusals can't be cleared (Architecture, Correctness)
**Severity**: major | **Confidence**: high | **Lens**: architecture, correctness

🟡 **Architecture / Correctness**

The PR moves key shape into the tracker (`TrackerKeys::{Jira, Linear, AnySupported}`). Convergence follows it: `is_converged` uses `TrackerKey::parse_under(key, scheme.tracker_keys())`. Re-key does not. `why_unclassifiable` here, and `tracker_mapping` and `keys_to_refresh` (lines 306 and 345), call `TrackerKey::parse`, which silently uses `AnySupported` even though `scheme` is in hand.

Take `{tracker}` with `work.integration: linear`, and an item with `id: MY_PROJ-7` and `external_id: MY_PROJ-7` (for example, a corpus moved from Jira):
- Convergence refuses it, and `under_tracker_only_drafts_and_items_named_by_their_issue_converge` pins that.
- `why_unclassifiable` finds no fault, so the refusal names no fix by hand and just says "Run `/rekey-work-items`".
- `tracker_mapping` then leaves the item alone, because its `id` already equals its key.

The corpus stays refused after every re-key. This is review 2's disagreement over what "in shape" means, now at the tracker-key level.

The root cause is structural: `TrackerKey::parse` and `IdScheme::new` both default to `AnySupported`, so the permissive reading is the easiest one to reach. `run_plan_store::decode_fingerprint` also rebuilds its scheme with `IdScheme::new`.

**Suggestion**:
- Use `TrackerKey::parse_under(stored, scheme.tracker_keys())` in `why_unclassifiable`, `tracker_mapping` and `keys_to_refresh`. The item then surfaces as `ExternalIdNotATrackerKey`, with its fix by hand.
- Add a `{tracker}` + `TrackerKeys::Linear` underscored-key case to the convergence/re-key cross-check.
- Remove the defaulting constructors, or rename them (e.g. `parse_any_supported`), so each caller chooses a rule set explicitly.

---

### `cli/visualiser/server/src/indexer.rs:576-590` — refresh_one repeats the entry-removal sequence, and the PR added the new handover step to every copy
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`refresh_one` is about 200 lines. Its "outside every root" branch (around lines 449–491) and its `NotFound` branch here run the same removal sequence word for word:
1. remove from adr, from identity key and from plans;
2. take the snapshots;
3. remove from reviews and from work-item refs;
4. remove the entry;
5. re-home the reviews.

The update branch repeats the snapshot-and-re-home tail as well. This PR added `holders_before` and `rehome_reviews_if_holders_moved` to all three places by hand.

**Impact**: the next secondary index, or the next change to the identity-key handover, has to be written three times in the same order. Missing one copy gives an index that goes stale only on deletion or on a move out of every root, which is hard to notice.

**Suggestion**: extract `async fn forget(&self, entries: &mut HashMap<..>, previous: &IndexEntry)`, which runs the removal sequence and the re-home. Both removal branches then call it, and `refresh_one` reads as: read, build, then update or forget.

---

### `skills/work/create-work-item/SKILL.md:193-195` — No eval covers E_REKEY_REQUIRED / E_WORK_ITEM_MALFORMED from resolve or id check
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

After an upgrade, the most likely first failure is the resolver or `work id check` exiting 4 with `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`. None of the 75 evals quotes either code; the only exit-4 identity evals cover `E_REKEY_IN_PROGRESS`.

The branch is easy for a model to get wrong:
- `id check` documents "exit 4, `E_ID_TAKEN` → *Requested ID recovery*", and the skill relies on *Identity-run refusals* being applied first. Without that, an untrusted corpus is routed to recovery, and the user is offered another ID that can never succeed.
- Resolve's exit 4 sits next to the exit-3 "treat as topic" branch.

**Impact**: the only guard is `test_untrusted_corpus_refusals.py`, which checks that the code names appear somewhere in the file. A regression that sends exit 4 to topic drafting or to ID recovery would pass every check.

**Suggestion**: add two evals, then re-run the benchmark.
1. `work resolve` exits 4 with `E_REKEY_REQUIRED`. Expect the printed remedy relayed verbatim, `/rekey-work-items` named, and no drafting or `work create`.
2. `work id check` exits 4 with `E_WORK_ITEM_MALFORMED`. Expect a stop rather than *Requested ID recovery*.

---

### `skills/work/rekey-work-items/SKILL.md:73-80` — The "Other keys first" fold dialogue has no eval, and its wording could fold only the last key picked (Test Coverage, Usability)
**Severity**: major | **Confidence**: high | **Lens**: test-coverage, usability

🟡 **Test Coverage**

The response to review 2 made this dialogue the only route for a user who renamed `work.key` to fold their old items. The skill must:
1. spot the `note:` line;
2. ask with a `multiSelect` (one option per key, plus **Keep them all**);
3. re-preview with `--from-key`;
4. branch on the new preview.

The rekey-work-items evals were not touched by this PR, and no eval mentions `--from-key` or other keys. The CLI side is well tested, but nothing checks that the skill offers the choice, keeps `--project` keys by default, or passes every picked key to one preview.

🔵 **Usability**

Each `work rekey --preview` stores its plan as *the* proposed plan. "For each key the user picks, preview again with `--from-key <KEY>`" can be read as one preview per key, in which case only the last key picked would be folded.

**Suggestion**:
- Reword to "preview once more, with a `--from-key <KEY>` for every key the user picked".
- Add evals for (a) a note naming PP and XY where the user keeps both: no `--from-key`, the apply proceeds; and (b) the user picks both: exactly one `accelerator work rekey --preview --from-key PP --from-key XY`.

---

### `tests/unit/tasks/test_create_work_item_id_evals.py:41-45` — The benchmark guard cannot see that SKILL.md changed after the run it vouches for
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`failing_runs` flags an eval whose expectations changed after it was graded. It does not flag a skill that changed after the benchmark ran.

The PR description says the response to review 2 changed create-work-item's exit-4 wording and resolver table without re-running the evals. At least three evals exercise those sections:
- `a_rekey_in_progress_at_step_0_points_to_rekey_work_items`
- `a_config_refusal_from_resolve_stops_before_drafting`
- `a_busy_identity_run_at_step_0_is_retried_not_aborted`

`test_every_create_work_item_eval_passes_fully` still passes, so the green 75/75 describes an earlier `SKILL.md`.

**Impact**: a skill edit that breaks routing for existing evals ships with a passing guard and a benchmark claiming a 1.0 mean. That is exactly the regression this guard was written to catch.

**Suggestion**: record a digest of `SKILL.md` (and `evals.json`) in `benchmark.json` when the benchmark runs. Have `failing_runs` report "skill changed since benchmark" on a mismatch, with a synthetic test like the existing one. Then re-run the evals the review-2 wording affects.

---

### `cli/work/src/rekey/convergence.rs:136-140` — The CLI remedy `accelerator work rekey` applies straight away, with no preview
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

The `E_REKEY_REQUIRED` remedy says "Run `/rekey-work-items` (or `accelerator work rekey`)". With no stored plan, a bare `accelerator work rekey` works out the mapping and applies it at once (`Mapping::Computed`). It renames files and rewrites references across `meta/`, with no preview.

A CLI user who follows the message never sees what will change. They also never see the `note:` line about other keys, which only `--preview` prints, so they cannot choose `--from-key`.

**Suggestion**: name the preview step in the CLI route, e.g. "(or `accelerator work rekey --preview`, then `accelerator work rekey`)".

---

### `cli/work/src/well_formed.rs:113-121` — `E_WORK_ITEM_MALFORMED` says to "fix the others likewise", but nothing can list them
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

The refusal gives a count and says "fix the others likewise", but names only the first item. `work list`, the command a user would reach for to find the rest, refuses with this same code. `MalformedItems::items()` already holds every offending path, yet the user has to fix one file and re-run to find the next, once per file. `E_REKEY_REQUIRED` has `rekey --preview` to list its items; the malformed refusal has nothing like it.

**Suggestion**: print every malformed path, one per indented line under the refusal, as the re-key refusals already do for their path lists. Or cap the list, e.g. the first 10 plus "and N more".

---

### `skills/work/create-work-item/SKILL.md:79-83` — Skill says `work resolve` refuses during a re-key and on a busy lock, which it never does
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The *Identity-run refusals* section says `accelerator work resolve` refuses "while an `accelerator work rekey` run is unfinished or another identity command is running". But `cli/work-cli/src/resolve.rs` takes no identity-run lock: it only calls `refuse_untrusted_corpus`, and `exit_codes.rs` leaves `resolve` out of the lock holders. So from this list, `resolve` can only give `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`, never `E_REKEY_IN_PROGRESS`, `E_REKEY_PLAN_UNREADABLE` or `E_IDENTITY_RUN_BUSY`.

The heading also now covers the two corpus-trust refusals, which have nothing to do with the identity run.

**Suggestion**: split the sentence. `id check`, `id next` and `create` refuse during a re-key or while the lock is busy; all four, `resolve` included, refuse an untrusted corpus. Or rename the section, e.g. "Refusals before anything is read".

---

### `docs-site/src/content/docs/guides/configuration-cookbook.md:187-188` — "Every `work` command that reads or writes IDs" refuses, though sync and rekey are exempt
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The cookbook says that until legacy items are converged, "every `work` command that reads or writes IDs refuses with `E_REKEY_REQUIRED`". But `work sync` and `work rekey` read and write IDs, and are deliberately exempt so the remedy stays reachable. Review 2 fixed the same overstatement in `sync-work-items.mdx`. It survives here, and in `skills/config/configure/SKILL.md` lines 730–731.

**Impact**: a user told that every ID command is blocked may never try `work sync`, the documented way out when re-key refuses over an unfinished sync.

**Suggestion**: name the gated commands (`create`, `create-batch`, `id next`, `id check`, `list`, `resolve`, `promote`), as the sync guide now does. Or add "except `work sync` and `work rekey`" in both places.

---

### `cli/work-cli/src/create.rs:809-811` — The real create turns a typed admission refusal into a string that the dry run keeps typed
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

In `run_with`, `Err(Unadmitted::Refused(refusal))` becomes `RunOutcome::Failed(refusal.to_string())`, and `main.rs` then works out the exit code from the message prefix through `exit_codes::of_failure`. The dry run's arm for the same `Unadmitted::Refused` (line 759) calls `refused(&refusal)` and takes its code from `of_requested_id_refusal`. So one value is handled two ways in one file.

The PR notes accept the string path only for the taken check under the lock, not for this step.

**Suggestion**: use `Err(Unadmitted::Refused(refusal)) => return refused(&refusal)` here too. `of_failure` is then needed only for the string coming back from `create_local_item`.

---

## Per-Lens Results

### Architecture

**Summary**: Review 2's architecture findings are mostly fixed:
- the corpus-trust policy is one domain operation, shared by work-cli and the visualiser;
- `WorkItemScheme` is one enum;
- tracker keys are read through `IdScheme::tracker_keys()`;
- the typed `of_requested_id_refusal` serves `id check` and the dry run.

Two problems remain. Per-tracker key rules are applied by convergence but bypassed by re-key, because `TrackerKey::parse` and `IdScheme::new` default to `AnySupported`. And the visualiser's live trust check rescans the filesystem on every request instead of reading its indexer.

**Strengths**:
- `work::corpus_trust::refuse_untrusted` returns a typed `UntrustedCorpus`, through one adapter.
- `WorkItemScheme::{Default, Configured, Unreadable}` makes the visualiser's "serve but name the problem" choice explicit in a type.
- `admit_request` plus `refuse_requested_id` keep the ordering knowledge in one place.
- `TrackerKeys` lives in `tracker`, with `for_integration` and `owns_ids()`. `IdentityFingerprint::staleness_since` deliberately compares only pattern and key.
- The convergence, well-formedness and trust checks are pure functions, with I/O in the adapters.
- The remaining tradeoffs are stated in the PR notes.

**Comments**:
- `cli/work/src/rekey/mapping.rs:160-165` (major, high) — tracker-owned key rules are applied by convergence but not by re-key. See the inline comment.
- `cli/visualiser/server/src/server.rs:175-194` (minor, medium) — the live trust check reads the work directory a second way, bypassing the indexer. It does blocking I/O in an axum handler on every SSE-triggered refetch. Compute it from the indexer's work-item entries, cache it and recompute on change events, or at least use `spawn_blocking`.

**General Findings**:
- (suggestion) The general trust gate depends on the re-key subsystem's internals. Consider one `work::convergence` module that both call.

### Correctness

**Summary**: Review 2's correctness findings are fixed:
- convergence and re-key share one shape test;
- the key-last correction drops the right literal;
- an identity-key handover re-homes reviews.

The taken rule, allocation, the grammar's single split and the create-lock ordering all hold up. One disagreement remains: under `{tracker}` + Linear, an underscored tracker key is refused by convergence, but re-key reads it under the any-tracker rules and leaves it alone.

**Strengths**:
- `FoldedKeys::keeps` is defined via `is_converged`, so re-key moves exactly the items convergence refuses, by construction.
- `segments_without_key` picks the joining literal by whether the number follows the key (`{number:04d}-{key}v` corrects `400v` to `0400-PPv`).
- Each refresh path snapshots the identity-key holders before and after, and re-homes reviews when a holder changes.
- `refuse_requested_id` runs the pending-push check, then the taken rule, under the create lock. Allocation and the taken rule read the same holders.
- The arithmetic is safe: width cap 19, `checked_pow`, `saturating_add`.
- `RecoveryDir::parse` and `RecordedId` reject traversal, absolute paths and nested paths.

**Comments**:
- `cli/work/src/rekey/convergence.rs:35-42` (minor, high) — under `{tracker}` + Linear, convergence refuses an item that re-key leaves untouched. Merged into the `mapping.rs:160` inline comment.

**General Findings**: none.

### Code Quality

**Summary**: Review 2's code-quality findings are mostly fixed:
- one shared trust check;
- `WorkItemScheme`;
- de-duplicated `TrackerIds`/`PatternIds` helpers;
- `FilenameIdentity`;
- typed `MalformedItem` and retirement records.

What remains: the indexer's removal sequence is copied three times; the string exit-code path survives where the typed refusal is in hand; `AdmittedRequest` claims a guarantee its public fields don't enforce; and some names and docs are stale.

**Strengths**:
- The shared trust policy and its typed refusals (`UntrustedCorpus`, `RekeyRequired`, `MalformedItems`, with `CODE` constants).
- `IdReadings` dispatch through `Owner` keeps `TrackerIds` and `PatternIds` small.
- `RequestedId::admit`/`propose` and `Proposal` model settlement in the domain's language.
- `WorkItemScheme` and the `NoWorkItemIds` null object.
- `admit_request` gives one opening order.

**Comments**:
- `cli/visualiser/server/src/indexer.rs:576-590` (major, high) — `refresh_one` repeats the removal sequence. See the inline comment.
- `cli/work-cli/src/create.rs:809-811` (minor, high) — the real create stringifies a typed refusal. See the inline comment.
- `cli/work-cli/src/create_batch.rs:246-249` (minor, medium) — `AdmittedRequest` has public fields, and create-batch builds one by hand. Make the fields private and give it a constructor that locates and trusts.
- `cli/work-cli/src/id.rs:99-102` (minor, high) — `id check` maps `Failed` through `of_failure`, which can only return `ERROR`. The test helper also compares against `of_failure`.
- `cli/work-adapters/src/creation.rs:711-713` (minor, high) — the `# Errors` doc on `refuse_requested_id` still describes a message, not `RequestedIdBlock`.
- `cli/visualiser/server/src/server.rs:186-194` (minor, high) — `unconverged` also reports malformed items, so rename it to `untrusted_corpus_problem`. The doc on `work_item_problem` names a `work_dir` parameter it doesn't take.
- `cli/work/src/well_formed.rs:38-53` (suggestion) — `MalformedItem::what(as_clause: bool)` should be split into `statement` and `clause`.
- `cli/work/src/rekey/convergence.rs:155-178` (suggestion) — `refuse_divergence` filters the off-pattern items twice; collect them once.
- `cli/visualiser/server/src/api/work_item_config.rs:171-176` (suggestion) — the configured-key chain is repeated in `indexer.rs`; add `WorkItemScheme::key()`.

**General Findings**:
- (suggestion) The `{ message, code }` refusal shape is repeated across seven command outcomes; one shared `Refusal` type with `From` conversions would replace them.

### Test Coverage

**Summary**: Review 2's test-coverage findings are fixed, and the Rust behaviour added in response to review 2 is tested at the right levels. What remains is on the skill side:
- the new exit-4 branches and the rekey fold dialogue have no evals;
- the benchmark predates the latest `SKILL.md` edits, and nothing detects that;
- the sync-exemption test is weaker than it looks.

**Strengths**:
- The convergence/re-key cross-check covers literal-text and literal-case patterns, other keys and aligned `id == external_id`.
- `--from-key` is covered from unit to CLI.
- `corpus_trust` has unit tests for its ordering and its messages.
- `of_requested_id_refusal` is tested against every variant, with keyword_exit_codes pinning the frozen prefixes.
- The problem endpoint is tested through the real router, including recomputation after boot.
- well_formed tests kill the mutants review 2 named.

**Comments**:
- `skills/work/create-work-item/SKILL.md:193-195` (major, high) — no eval covers the corpus refusals from `resolve` or `id check`. See the inline comment.
- `skills/work/rekey-work-items/SKILL.md:73-80` (major, high) — the fold dialogue has no eval. See the inline comment.
- `tests/unit/tasks/test_create_work_item_id_evals.py:41-45` (major, high) — the benchmark guard is blind to `SKILL.md` changes. See the inline comment.
- `cli/work-cli/tests/cli_rekey_required.rs:129-144` (minor, medium) — the sync-exemption test passes however early sync fails. Assert exit 74 at least, or drive sync in-process with a fake tracker and a pending-push marker, then re-key.
- `tests/unit/tasks/test_untrusted_corpus_refusals.py:16-19` (minor, medium) — the refusing-command list is copied by hand and is only checked for name presence. Share one fixture with the CLI test.
- `cli/work-cli/src/eval_contract.rs:24-40` (suggestion) — only `E_ID_*` quotes are checked against exit codes, via the string mapper.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.test.tsx:34-47` (suggestion) — no test shows the banner clearing on invalidation.

**General Findings**:
- (minor) The skills changed for review 2 were not re-evaluated.
- (suggestion) Which unclassifiable item `E_REKEY_REQUIRED` names is not pinned.

### Compatibility

**Summary**: Review 2's compatibility findings are resolved:
- one in-shape definition;
- re-key keeps other keys;
- every refusing skill handles exit 4, and a test guards that;
- a Migrations entry;
- the linkage-extract refusal is documented and tested.

Two smaller consumer gaps remain. `update-work-item` treats every exit 1 from `resolve` as bad input. And the frontend's wiki-link grammar cannot follow the ID shapes the server now reports.

**Strengths**:
- Convergence checks shape before tracker alignment and ignores literal case.
- Re-key keeps other keys unless `--from-key` names them.
- `test_untrusted_corpus_refusals.py` guards the skills, and sync routes exit 4 to the ungated `work sync`.
- The Migrations entry gives a concrete path, and the configure skill matches the gate.
- `problem` is additive and `skip_serializing_if`.
- The retired verbs fail loudly with `E_VERB_RETIRED`.
- The decoders check what earlier versions wrote.

**Comments**:
- `skills/work/update-work-item/SKILL.md:65-67` (minor, medium) — exit 1 from `resolve` is always "unrecognised input", hiding `E_WORK_ID_PATTERN_INVALID` and `E_WORK_KEY_INVALID`. Split on `E_RESOLVE_INVALID:`, as create-work-item does.
- `cli/visualiser/frontend/src/api/fetch.ts:82-85` (minor, medium) — the frontend's `buildBareIdPattern` still assumes `<key>-\d+|\d+`, so `[[WORK-ITEM-PP_0042]]`, `v001` and tracker keys don't link. Expose the pattern (or a regex source the server builds) additively, or record the limit in the CHANGELOG.

**General Findings**:
- (suggestion) `work list` reads the scheme before the missing-directory check, which contradicts the list-work-items contract for a bad pattern with no work directory.

### Usability

**Summary**: Review 2's usability findings are mostly fixed:
- every calling skill handles the exit-4 refusals;
- `E_REKEY_REQUIRED` names the fix by hand;
- the malformed refusal gives a count;
- the banner clears after a re-key;
- re-key no longer folds other keys silently.

What remains is friction at the edges of the gate: a CLI remedy with no preview, no way to list malformed items, banner text shaped for a terminal, `work.key` renames that get no prompt, and recovery chains of several refusals.

**Strengths**:
- `work id check`'s verdict contract shares codes and exits with `create --id`.
- The `/create-work-item --id` flow settles the ID before drafting, confirms corrections and resumes drafts.
- The preview's `note:` names the kept keys with counts and the exact `--from-key` follow-up.
- Refusals consistently give a code, what went wrong and the remedy, with paths relative to the root and hostile IDs escaped.

**Comments**:
- `cli/work/src/rekey/convergence.rs:136-140` (minor, high) — the CLI remedy applies with no preview. See the inline comment.
- `cli/work/src/well_formed.rs:113-121` (minor, high) — nothing can list the malformed items. See the inline comment.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:14-19` (minor, medium) — the banner shows raw CLI text ending "re-run this command". It doesn't say what is degraded, or that a scheme error needs a restart, since `cfg.work_item.problem()` is fixed at boot.
- `skills/work/sync-work-items/SKILL.md:263-267` (minor, medium) — promotion recovery over an off-pattern corpus bounces `promote` → re-key → `sync`. The `E_REKEY_REQUIRED` bullet says both "re-run this skill" and "Stop".
- `skills/config/configure/SKILL.md:714-716` (minor, medium) — renaming `work.key` triggers no refusal and starts a parallel sequence. Document the `--from-key <old key>` fold.
- `skills/work/rekey-work-items/SKILL.md:79-80` (suggestion) — "For each key the user picks, preview again" could fold only the last key. Merged into the rekey inline comment.

**General Findings**: none.

### Documentation

**Summary**: Review 2's documentation findings are resolved:
- a Migrations entry with an upgrade path and the sync escape hatch;
- the configure skill corrected;
- the skills that call `resolve` or `list` handle exit 4;
- the `id check` help covers key-case correction.

What remains is small accuracy drift: `resolve` described as refusing during a re-key, "every" ID command said to refuse, a garbled CHANGELOG sentence, `id check` missing from one list, and a stale visualiser comment.

**Strengths**:
- The Migrations entry gives a concrete path.
- The refusal codes, exit codes, verb names and `id check` columns in the docs match the code.
- The configure skill's grammar rules match the CHANGELOG.
- The rekey skill documents `--from-key` and the `note:` line.
- The sync guide now names exactly which commands refuse.

**Comments**:
- `skills/work/create-work-item/SKILL.md:79-83` (minor, high) — `resolve` is said to refuse during a re-key or while the lock is busy. See the inline comment.
- `docs-site/src/content/docs/guides/configuration-cookbook.md:187-188` (minor, high) — "every" ID command refuses, though sync and rekey are exempt; the configure skill at 730–731 says the same. See the inline comment.
- `CHANGELOG.md:341-343` (minor, high) — a garbled sentence in the `--id` Added entry. Lines 199, 341–343 and 365 run past 80 columns.
- `CHANGELOG.md:876-877` (suggestion) — the list of scheme-validating commands leaves out `id check`.
- `CHANGELOG.md:1133-1139` (suggestion) — the first Migrations step doesn't cover `E_WORK_ID_PATTERN_INVALID` from `work list`.
- `cli/visualiser/server/src/indexer.rs:1404-1405` (suggestion) — the comment justifies a filename/`id` mismatch that is now malformed; delete it.

**General Findings**: none.

### Security

**Summary**: Both of review 2's security findings are fixed. `plan_retirement` checks every old and new ID against `is_in_id_alphabet`, and every value written goes through one escaping `double_quoted` YAML writer, including the `external_id` that `retired_from` carries over. No new exploitable gap was found across:
- user `--id` values and tracker keys;
- retirement records and recovery dirs;
- the grammar;
- the visualiser's `problem` field and banner.

**Strengths**:
- Two layers of defence in retirement: the alphabet gate and the escaping YAML writer.
- `RetirementRecord` can only be built from parsed `RecordedId`/`RecoveryDir`, with tests for traversal and injection.
- Requested IDs must match the pattern exactly. Literals are limited to `[A-Za-z0-9._-]`, and keys are typed `IdKey`, `--from-key` included.
- Each tracker owns its key shape, so tracker responses are shape-checked.
- The grammar compiles no regex from config, which removes the ReDoS and ambiguous-split risk.
- Refusals escape non-printable characters as `U+XXXX`.
- `problem` uses paths relative to the root and is rendered as a React text node, with no `dangerouslySetInnerHTML`.

**Comments**: none.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
