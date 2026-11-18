---
type: "pr-review"
id: "178-review-4"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-17T06:30:55+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:178"
relates_to: ["pr-review:178-review-1", "pr-review:178-review-2", "pr-review:178-review-3"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "documentation", "security"]
review_number: 4
pr_number: 178
tags: []
last_updated: "2026-11-17T06:30:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #178 - [0319] Let work create take an explicit work item ID

**Verdict:** COMMENT

The response to review 3 holds up. Every architecture, code-quality, usability, compatibility and test-coverage finding has a fix in the delta, and the one correctness finding is closed: re-key now reads stored tracker keys under the same rules as convergence. Security found nothing exploitable. No finding is critical.

What remains is new, mostly in what the delta added, and falls into three groups:
1. **Two correctness regressions** from the narrower taken rule and the visualiser's new `idToken`.
2. **A CHANGELOG claim the code contradicts.**
3. **Consumers not yet brought in line with the new shapes:** the SPA facet filter, the skills' wording of `E_WORK_ITEM_MALFORMED`, and the eval contract.

### Cross-Cutting Themes

- **The visualiser's ID grammar is the weakest seam** (flagged by: Correctness, Architecture, Code Quality, Security, Documentation, Compatibility). `WorkItemConfig::id_token` hand-writes the key, tracker-key and draft grammars as JavaScript regex. That causes several problems:
  - Under `{tracker}`, ADR wiki links stop matching, because the token is shared with the `ADR` prefix and has no `\d+`.
  - Under Linear it admits `_`, which the scheme refuses.
  - Literals escape only `.`.
  - Matching is case-sensitive where the CLI isn't.

  Separately, the SPA still derives the project facet by splitting `workItemId` at `-`, and `formatDocId` still pads IDs. Building the token from the owning types, and serving the parsed key, would close all of these together.
- **The taken rule now differs between create and re-key** (flagged by: Correctness, Test Coverage, Documentation).
  - `fields_taking` and `allocate` drop an `external_id` that names a tracker issue, but re-key's `refuse_collisions` still counts it through `linker_of_under`. `create` and `id next` can therefore hand out an ID that a later mandatory re-key refuses.
  - No test pins the in-shape keyed `external_id` case. The test that would have was deleted.
  - The docs describe "the configured tracker's" issue key, but with no integration configured the rule falls back to `AnySupported`.
- **`E_WORK_ITEM_MALFORMED`'s new shape hasn't reached its consumers** (flagged by: Usability, Documentation, Test Coverage). The refusal now lists every item with its own fix and covers non-UTF-8 and ID-only files. But:
  - four skills, the `exit_codes` module doc and the CHANGELOG Migrations step still describe one file with the two old fixes;
  - the banner collapses the multi-line list into one paragraph;
  - no visualiser test covers a non-UTF-8 work item.
- **Some review-3 documentation items are still open** (flagged by: Documentation, Compatibility):
  - the scheme-validating list still omits `id check`;
  - the first Migrations step doesn't mention `E_WORK_ID_PATTERN_INVALID` or a legacy retirement record that blocks both re-key and sync;
  - CHANGELOG lines 201, 253, 353 and 684 run past 80 columns;
  - "`work` commands that read or write IDs refuse" survives in three places.

### Tradeoff Analysis

- **Architecture vs Correctness on the indexer's trust state.** Correctness judges the new indexer-held trust verdict sound: every flip triggers a fresh rescan, so a stale judgement corrects itself. Architecture points out that `judge_trust` writes `reading` and `untrusted_corpus_problem` outside `rescan_lock`, and that `rescan` re-reads `work_item_cfg()` per entry. One rebuild can therefore mix two schemes until the next rescan. Code Quality adds that the two fields are kept in step by hand. Recommendation: at least capture `reading` once per rescan. Holding trust as one `RwLock<Trust>` enum, judged under the rescan lock, would settle both concerns cheaply.

### Strengths

- ✅ Review 3's findings are resolved across every lens:
  - `work::convergence` is now its own module;
  - the visualiser judges trust from the indexer's own files through the pure `corpus_trust::refuse_untrusted`, with no second filesystem read;
  - work-cli has one `Refusal` type;
  - `AdmittedRequest` has private fields and named constructors;
  - `Indexer::forget` replaces three copies of the removal sequence;
  - the permissive tracker reading is named as a choice wherever it is used.
- ✅ Convergence, `why_unclassifiable`, `tracker_mapping` and `keys_to_refresh` all read stored tracker keys under `scheme.tracker_keys()`. The cap check in `is_in_target_shape` closes a real convergence/re-key divergence.
- ✅ The remedies are better:
  - `E_REKEY_REQUIRED` names `rekey --preview` before `rekey`;
  - `E_WORK_ITEM_MALFORMED` lists every item;
  - `work list` lists with a warning instead of refusing;
  - the banner says what is switched off and whether it needs a restart (`restartNeeded`);
  - the configure skill documents the `--from-key` fold;
  - update-work-item relays config refusals instead of treating them as unrecognised input.
- ✅ The tests are stronger:
  - the gated-command set is one fixture, compared two ways;
  - the eval contract runs against the built binary;
  - the benchmark records a `skill_digest`;
  - a four-item test kills the `min_by`/`max_by` mutants;
  - the sync exemption test requires exit 74;
  - trust flips are tested in both directions through `refresh_one`.
- ✅ Security stays tight:
  - `Retiring::recovery_dir` is a parsed `RecoveryDir` both when written and when read;
  - a tracker-returned key can't escape the recovery parent;
  - non-UTF-8 files are a typed `Undecodable`, not decoded lossily;
  - the banner renders `problem` as a text node.

### General Findings

- 🔵 **Code Quality**:
  - The `Refusal` type landed without conversions. It is unpacked and rebuilt in `update.rs`, `promote.rs` and `sync.rs`, and `sync.rs` keeps a second private type called `Refusal`. Also, `create::refused` and `id::check::refused` are identical wrappers over the misnamed `refused_line`.
  - The real create and its dry run map `Unadmitted` with the same three-arm match (`create.rs:813`).
  - create-batch handles a refusal `without_requested_id` can never return (`create_batch.rs:214`).
- 🔵 **Architecture**:
  - `convergence` and `rekey` now depend on each other (`convergence.rs:15`). Move `UnclassifiableCause`/`why_unclassifiable` into a neutral module.
  - The config endpoint takes `problem` from the indexer but `key`/`idToken` from the boot-time scheme (`work_item_config.rs:32`).
- 🔵 **Test Coverage**:
  - The new benchmark records for update, review and refine carry placeholder evidence. Only create-work-item has the `skill_digest` guard.
  - `/list-work-items`'s warning branch and `/sync-work-items`' promotion-recovery route have no evals.
  - The `skill_digest` guard can't tell a re-run from a pasted digest.
- 🔵 **Documentation**:
  - The PR description still describes the deleted `eval_contract.rs`.
  - Its *Taken rule* bullet predates `fields_taking`.
- 🔵 **Code Quality**: the frontend's `legacyIdToken` project-code fallback can't be reached in production.
- 🔵 **Compatibility**: `idToken` is case-sensitive, while the CLI reads keys and literal text in any case.

### Additional Findings

- 🔵 `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:24` — the banner collapses the multi-item malformed list into one run-on paragraph (usability)
- 🔵 `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:20` — the banner's opening overstates which commands refuse, and "restart" doesn't say how (usability)
- 🔵 `cli/work-cli/src/list.rs:586` — `work list`'s warning says "re-run this command" about a command that succeeded (usability)
- 🔵 `cli/work/src/well_formed.rs:69` — the NoSlug fix gives a `<slug>` placeholder when the slug is known (usability)
- 🔵 `skills/work/sync-work-items/SKILL.md:263` — the skill sends the user to the CLI for `work sync` instead of offering to run it (usability)
- 🔵 `meta/prs/178-description.md:317` — repeats the bare-number display claim the code contradicts (documentation)
- 🔵 `meta/prs/178-description.md:133` — still describes the deleted `eval_contract.rs` (documentation)
- 🔵 `CHANGELOG.md:884` — the scheme-validating list still omits `id check`, as in review 3 (documentation)
- 🔵 `CHANGELOG.md:1162` — the first Migrations step misses `E_WORK_ID_PATTERN_INVALID`, the new malformed fixes and the legacy retirement record that blocks sync (documentation, compatibility)
- 🔵 `CHANGELOG.md:247` — the malformed-item entry is a run-on sentence; lines 201, 253, 353 and 684 run past 80 columns (documentation)
- 🔵 `CHANGELOG.md:286` — "configured tracker's issue key" also covers repositories with no tracker (documentation)
- 🔵 `CHANGELOG.md:1186` — "`work` commands that read or write IDs refuse" survives in three places (documentation)
- 🔵 `cli/work-cli/src/exit_codes.rs:193` — the module doc describes the old malformed and rekey-required texts (documentation)
- 🔵 `cli/work-cli/src/create.rs:813` — duplicate `Unadmitted` → `RunOutcome` match (code-quality)
- 🔵 `cli/work-cli/src/create_batch.rs:214` — handles a refusal that can't happen (code-quality)
- 🔵 `cli/work-cli/src/update.rs:327` — `Refusal` is destructured and rebuilt instead of converted (code-quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:795` — the trust state is two locks kept in step by hand; `judge_trust` both changes state and returns a bare bool (code-quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:449` — trust state changes outside `rescan_lock` (architecture)
- 🔵 `cli/visualiser/server/src/server.rs:99` — the boot warning repeats `work_item_problem`'s chain (code-quality)
- 🔵 `cli/visualiser/server/src/api/work_item_config.rs:32` — mixes the composed and effective schemes (architecture)
- 🔵 `cli/work/src/convergence.rs:15` — two-way module dependency with `rekey` (architecture)
- 🔵 `cli/work/src/well_formed.rs:23` — `NoSlug` now means "filename doesn't name its ID" (code-quality)
- 🔵 `cli/work/src/rekey/sequence.rs:94` — `cap().unwrap_or(u64::MAX)` is spelled out in several places (code-quality)
- 🔵 `cli/work/src/requested_id.rs:1009` — a deleted test case leaves the keyed `external_id` boundary unpinned (test-coverage)
- 🔵 `cli/visualiser/server/src/indexer.rs:351` — no test of the visualiser's non-UTF-8 path (test-coverage)
- 🔵 `skills/work/update-work-item/evals/benchmark.json:105` — new eval records carry no evidence and no digest (test-coverage)
- 🔵 `skills/work/list-work-items/SKILL.md:180` — no evals for the list warning or the sync promotion recovery (test-coverage)


## Inline Comments

### `cli/work/src/identity.rs:80-98` — Create and allocation now free a number that re-key's collision check still counts as held
**Severity**: major | **Confidence**: medium | **Lens**: correctness

🟡 **Correctness**

`fields_taking` here, and the matching filter in `next_number::allocate` (`next_number.rs:239`), ignore an `external_id` that `names_a_tracker_issue`. Re-key was not changed: `rekey/collision.rs:61` calls `linker_of_under`, which goes through `fields_holding` and still counts that `external_id` as holding the number. It then refuses with a `LinkedClash`.

**Scenario**: under `{key}-{number:04d}`, with `work.key` `PP` and a tracker whose keys are also `PP-…`, item A has `external_id: PP-905`.
- `work id next` (or `create --id PP-0905`) now creates item B as `PP-0905` beside A.
- Later the width changes to `{number:05d}`, and the gate demands a re-key.
- Re-key refuses B's `PP-0905 → PP-00905`, because A's `PP-905` holds 905 under `PP`, and points the user at changing `work.key` or switching to `{tracker}`.

A bare `PP-905` reference now also joins to the new local item, not to the item linked to the issue.

**Impact**: `create` and `id next` can build a corpus that the mandatory re-key then refuses. The same is true with no integration configured, because `names_a_tracker_issue` falls back to `AnySupported`.

**Suggestion**: have one taken rule for both directions. Either keep tracker-issue `external_id`s as takers in create and allocation, or apply the same `names_a_tracker_issue` exclusion in `linker_of_under`, `refuse_collisions` and `shared_key`. Pin it with a test that runs `id next` and then a width-change re-key over such a corpus.

---

### `cli/visualiser/server/src/config.rs:178-181` — Under `{tracker}` the idToken has no bare-number alternative, so `[[ADR-NNNN]]` links stop matching (Correctness, Documentation)
**Severity**: major | **Confidence**: high | **Lens**: correctness, documentation

🟡 **Correctness**

The frontend puts `idToken` in a capture group that both prefixes share: `\[\[(ADR|WORK-ITEM)-(${innerWorkItem})\]\]` in `wiki-links.ts:30`, and the same in `buildBareIdPattern` at `:62`. Every local pattern's token ends in `|\d+`, and that is what lets `ADR-0017` match. The tracker-owned token returned here is `[A-Za-z][A-Za-z0-9_]*-\d+|draft-[0-9a-z]{6}`, with no `\d+`. So under `{tracker}`, `[[ADR-0017]]` and bare `ADR-0017` frontmatter values no longer match. The tests only exercise `WORK-ITEM-` under the tracker token.

**Impact**: in any repository on `{tracker}` IDs, every ADR wiki link in the visualiser renders as plain text, a regression from the legacy grammar.

**Suggestion**: give ADRs their own grammar, e.g. `\[\[(?:ADR-(\d+)|WORK-ITEM-(${idToken}))\]\]`, rather than relying on the work-item token to carry `\d+`. Add a test that `[[ADR-0017]]` links under the tracker token.

🔵 **Documentation**: the rustdoc above (lines 171-173) says the token matches "a bare number", but it doesn't under `{tracker}`. The frontend comment `fetch.ts:87` says `idToken` is "absent from older servers", but the server and frontend ship together, and it is really absent when the scheme is unreadable.

---

### `CHANGELOG.md:238-246` — CHANGELOG promises a bare-number workItemId display the visualiser never shows
**Severity**: major | **Confidence**: high | **Lens**: documentation

🟡 **Documentation**

The entry says a bare number now shows as the pattern renders it: `ENG-0042` for `42` under `{key}-{number:04d}`, `0042` under `{number:04d}`, and `v001` for `1`. But `workItemId` comes only from an item's frontmatter `id`, and a bare `42` is off the pattern's exact width. So the corpus is untrusted and the indexer reads no IDs at all. The PR's own test `id_frontmatter_bare_digits_is_off_pattern_and_shows_no_id` (`indexer.rs:4208`) asserts `work_item_id == None`. The PR description's ⚠️ *Visualiser `workItemId`* bullet (`meta/prs/178-description.md:317-320`) makes the same claim.

**Impact**: consumers of `workItemId` are told to expect `0042`. After upgrading they will see something bigger: no ID on any item, and an `E_REKEY_REQUIRED` banner.

**Suggestion**: say that an `id` the pattern does not read, a bare `42` included, makes the visualiser read no work-item IDs and report `E_REKEY_REQUIRED` until the corpus is re-keyed. Drop the `0042`/`ENG-0042`/`v001` examples, or scope them to `displayed_id` callers, and make the same fix in the PR description.

---

### `cli/visualiser/server/src/config.rs:175-198` — `id_token` restates ID grammars that `work` and `tracker` own, and escapes only `.` (Architecture, Code Quality, Security)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality, security

🔵 **Architecture / Code Quality**

`id_token` builds the page's link grammar from `IdPattern::shape()`, but writes the alphabets as regex literals here:
- the key, `[A-Za-z][A-Za-z0-9]*`, owned by `IdKey`;
- the tracker key, `[A-Za-z][A-Za-z0-9_]*-\d+`. This is always the any-tracker shape, so under Linear it admits underscores the scheme refuses. It is owned by `TrackerKeys`;
- the draft ID, `draft-[0-9a-z]{6}`, owned by `DraftId`, which is Crockford base32 with at least one letter.

The `ShapePart::Tracker => String::new()` arm can't be reached after the `is_tracker_owned()` early return. The frontend test copies the same strings a third time.

🔵 **Security** (hardening)

Literals are escaped with `text.replace('.', r"\.")` and then compiled in the browser with `new RegExp(...)`. That is safe only because `IdPattern::parse`, in another crate, restricts literals to `[A-Za-z0-9._-]`. `work.id_pattern` can come from an untrusted collaborator, so widening that alphabet would quietly open regex injection or ReDoS in every visualiser tab.

**Impact**: the PR moved identity rules into `work`/`tracker` so there is one place to change them. These copies will drift without anyone noticing, as the old `<key>-<digits>` assumption did.

**Suggestion**:
- Have the owning types expose their grammar (`IdKey::REGEX_SOURCE`, `TrackerKeys::regex_source()`, `DraftId::REGEX_SOURCE`) and pin each one to its parser with a test.
- Assemble the token from those, using `scheme.tracker_keys()`.
- Escape every regex metacharacter in literals.
- Add a parity test that every ID the scheme renders matches the token.

---

### `cli/visualiser/server/src/indexer.rs:89-93` — The server's project facet reads the key through the scheme, but the SPA still splits workItemId at '-'
**Severity**: minor | **Confidence**: high | **Lens**: compatibility

🔵 **Compatibility**

The server now builds the `project` facet with `scheme.read_held(shown)?.key()`. But the SPA's filter, `matchesSelection` in `LibraryTypeView.tsx:95-98`, still works the key out with `workItemId.slice(0, workItemId.indexOf("-"))`.
- Under `{key}_{number:04d}`, the server offers `PP` as an option, the client computes `null` for every item, and selecting `PP` hides the whole list.
- Under `X-{key}-{number}`, the client computes `X`.
- `formatDocId`, used on kanban cards and in the library, lifecycle and related-artifacts views, pads any `<prefix>-<digits>` value to 4 digits. So under `{tracker}`, `ENG-7` shows as `ENG-0007`, and `PP-001` under `{key}-{number:03d}` shows as `PP-0001`, undoing the pattern-faithful IDs the server now serves.

**Impact**: the CHANGELOG tells `workItemId` consumers they need updating, but this in-repo consumer wasn't updated.

**Suggestion**: serve the parsed key for each entry and filter on that. Reduce `formatDocId` to the identity. Add tests under `{key}_{number:04d}` and `{tracker}`.

---

### `cli/work/src/resolve.rs:202-208` — The rewritten keyless 'pattern-shape' bare-number branch has no test
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The delta rewrote the keyless branch of `resolve_bare_number` to read filenames through `IdParts::parse_prefix`. No test asserts the `pattern-shape` tag, or resolves a bare number under any keyless pattern other than `{number:04d}`. There, the legacy branch above already finds `0042-x.md` and hides this one. Deleting the branch, or flipping `is_some()` to `is_none()`, passes every test.

**Impact**: the PR says `work resolve` finds a bare number's items through the pattern. A regression here would turn `/update-work-item 42` into a not-found under `{number:05d}`, `v{number:03d}` or `a.{number}`, with no failing test.

**Suggestion**: add unit tests that `resolve("42", BareNumber, …)` under `{number:05d}` finds `00042-x.md`, and that `resolve("1", …)` under `v{number:03d}` finds `v001-x.md` and not `v002-y.md`.

---

### `cli/work/src/rekey/outcome.rs:603-605` — The gate's fix for a number above the cap says to renumber by hand; re-key's own refusal says to widen the pattern
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

Take an item whose number is above the pattern's cap, e.g. `12345` under `{number:04d}`. `E_REKEY_REQUIRED` says "Give the item an `id` in the shape of `work.id_pattern` by hand", because `NumberAboveCap` shares the `NoNumber` fix. The same situation inside re-key (`KeptNumberAboveCap`) says "Widen `work.id_pattern`".

Renumbering by hand also means picking a free number and fixing every reference to the old ID. Once the `id` is in shape, re-key treats the item as converged and rewrites nothing.

**Impact**: one cause gets conflicting advice on two surfaces. The gate, which the user meets first, steers them to the costlier fix that leaves references stale.

**Suggestion**: give `NumberAboveCap` its own fix, e.g. "Widen `work.id_pattern` to hold {number}, or give the item an `id` in its shape by hand and update the references to it".

---

### `cli/work/src/rekey/outcome.rs:371-379` — 'Move each unreadable record aside' names the records directory, not the record
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

`unfinished_sync_work` (`work-adapters/src/rekey/guards.rs:62-66`) throws away the `StoreError` from `records.outstanding()`, although that error holds the failing file's path. The blocker is built as `UnreadableRecord(state_dir.join("retirement-records"))`. So the refusal reads "unreadable record .accelerator/state/retirement-records": it doesn't say which record is bad, and it doesn't say what "aside" means. Only `*.json` files are read.

**Impact**: told about a directory, the obvious move is to set the whole `retirement-records` directory aside. That drops every readable in-flight retirement record along with the bad one.

**Suggestion**: carry the failing path from `StoreError` into `UnreadableRecord`. Spell out the move, e.g. "rename `<file>` so it no longer ends in `.json`, or move it out of `retirement-records/`".

---

### `skills/work/create-work-item/SKILL.md:96-97` — Skills still paraphrase E_WORK_ITEM_MALFORMED as one file with two fixes (Usability, Documentation)
**Severity**: minor | **Confidence**: high | **Lens**: usability, documentation

🔵 **Usability / Documentation**

This bullet says to "relay the named file and its remedy (rename it after its `id`, or give it an `id`)". The same wording appears in `extract-work-items/SKILL.md:360-361` and `refine-work-item/SKILL.md:196-197`, and `sync-work-items/SKILL.md:270` says "the named file is fixed by hand".

The refusal now lists *every* malformed file, each with its own fix, and two kinds have fixes this omits:
- non-UTF-8: "Re-save it as UTF-8, or move it out of the work directory";
- named only by an ID: add an `id` and rename, "or move it out".

**Impact**: prompted with "the named file" and a list of fixes in brackets, Claude is likely to relay one file with the old fixes. That recreates the one-at-a-time loop review 3 flagged, and may tell a user to "give it an `id`" for a file that is not UTF-8.

**Suggestion**: in all four skills, replace the bracketed fixes with "relay every file it lists with its printed fix, as is", matching the other skills' "relay the printed remedy as is".

---

### `tests/integration/evals/test_create_work_item_eval_contract.py:25-27` — The eval contract silently skips eval 46's quotes and every non-E_ID_ code
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`QUOTED_REFUSAL` only matches `exiting N with this stderr: [Error: ]E_ID_*`, so several quotes are never checked:
- Eval 46 quotes seven refusals in another form ("exits 1 with the stderr given" and numbered lines), including the suite's only quote of `E_ID_DRAFT_RESERVED`.
- Evals 73 and 74, and the new update/review/refine evals, quote `E_REKEY_REQUIRED` and `E_WORK_ITEM_MALFORMED` at exit 4, and the regex never looks at those codes.

`assert quoted` needs only one match across the suite, and only exit codes are compared, never message text.

**Impact**: if the CLI rewords a refusal, or changes a corpus refusal's exit code, evals quoting the old output keep passing. Catching that is what the lane is for, and review 3 raised its scope.

**Suggestion**:
- Match any quoted `E_[A-Z_]+` code, and draw `E_REKEY_REQUIRED` and `E_WORK_ITEM_MALFORMED` out of the real binary through fixtures, as the five `E_ID_*` codes already are.
- Assert that every `E_` code a prompt mentions is covered by some quote form, so a new phrasing fails loudly instead of being skipped.

---

## Per-Lens Results

### Architecture

**Summary**: All three review-3 architecture findings are resolved. Convergence is a top-level `work::convergence` module used by both the trust gate and re-key's folding. The visualiser judges trust from the files its indexer already reads, through the pure `work::corpus_trust::refuse_untrusted`. work-cli has one `Refusal`. The fixes bring smaller problems: `convergence` and `rekey` now depend on each other; `idToken` hard-codes grammars that `work` and `tracker` own; and the indexer's new trust state changes outside its `rescan_lock`.

**Strengths**:
- `rekey::folding` defines what re-key keeps in terms of `convergence::is_converged`, so the gate and re-key share one predicate.
- `refuse_untrusted` takes a `WorkItemListing` port type, so it stays pure and each caller supplies its own I/O.
- `AdmittedRequest` has private fields and two named constructors, so create-batch can no longer skip locating and trusting.
- The permissive tracker reading is named wherever it is used, and `SchemeReading::announced` is shared by work-cli and corpus-cli.
- `IdScheme::names_a_tracker_issue` centralises the rule that a tracker issue in `external_id` takes no local number.

**Comments**:
- 🔵 suggestion (high) `cli/work/src/convergence.rs:15-18`: lifting convergence out of re-key left a two-way module dependency. Move `UnclassifiableCause`, `why_unclassifiable` and their text helpers into a neutral module.
- 🔵 minor (high) `cli/visualiser/server/src/config.rs:175-198`: the visualiser hard-codes ID grammars that `work` and `tracker` own *(merged into inline comment)*.
- 🔵 minor (medium) `cli/visualiser/server/src/indexer.rs:449-453`: trust state and the reading scheme change outside `rescan_lock`, and `rescan` re-reads `work_item_cfg()` per entry.
- 🔵 suggestion (medium) `cli/visualiser/server/src/api/work_item_config.rs:32-34`: the endpoint takes `problem` from the indexer but `key`/`idToken` from the boot-time scheme.

**General Findings**: none.

### Correctness

**Summary**: The review-3 correctness finding is resolved. Convergence, `why_unclassifiable`, `tracker_mapping` and `keys_to_refresh` all read stored tracker keys under `scheme.tracker_keys()`, and the remaining `parse_any_supported` callers don't decide classification. The cap check is now applied consistently. The indexer's trust judgement corrects itself. The delta brings two new problems: the narrower taken rule lets `create` and `id next` hand out IDs that re-key's collision check still treats as held, and the `{tracker}` idToken drops the bare-number alternative that ADR wiki links depend on.

**Strengths**:
- Re-key reads stored tracker keys under the configured tracker's rules everywhere convergence does.
- The cap check in `is_in_target_shape` closes the `12345`-under-`{number:04d}` divergence.
- `refuse_divergence` names the first unclassifiable item by path from the already-sorted list.
- Every trust flip from `refresh_one` triggers a fresh rescan.
- `id next` and `id check` take the identity-run lock before reading the scheme.
- Only `Undecodable` reads count as malformed; I/O failures still reach the taken check.

**Comments**:
- 🟡 major (medium) `cli/work/src/identity.rs:80-98`: create and allocation free a number that re-key's collision check still counts as held *(inline)*.
- 🟡 major (high) `cli/visualiser/server/src/config.rs:178-181`: under `{tracker}`, idToken has no `\d+`, so `[[ADR-NNNN]]` links stop matching *(inline)*.

**General Findings**: none.

### Code Quality

**Summary**: Review 3's code-quality findings are all fixed. What remains is smaller:
- the new `Refusal` is destructured and rebuilt instead of converted;
- the create's two admission matches are copies of each other;
- `without_requested_id` forces create-batch to handle a refusal it can never get;
- the visualiser's trust state and ID-token grammar each repeat things that live elsewhere.

**Strengths**:
- `work-cli/src/refusal.rs` gives the stderr line and exit code one name and one `report()`.
- `AdmittedRequest` uses private fields and named constructors.
- `Indexer::forget` replaces three copies of the removal sequence.
- `MalformedItems` lists every malformed item with its own fix, and the `as_clause` boolean is gone.
- `refuse_divergence` works in a single pass.
- The renames make the permissive rule set an explicit choice.
- `SchemeReading::announced` sits next to the data it prints.

**Comments**:
- 🔵 minor (high) `cli/work-cli/src/create.rs:813-822`: the real create and its dry run map `Unadmitted` with the same three-arm match. Add `From<Unadmitted> for RunOutcome` and rename `Unadmitted::Refused` to `IdRefused`.
- 🔵 minor (high) `cli/work-cli/src/create_batch.rs:214-219`: create-batch handles a requested-ID refusal `without_requested_id` can never return. Give it a narrower `Unopened` error type.
- 🔵 minor (high) `cli/work-cli/src/update.rs:327-329`: the shared `Refusal` is destructured and rebuilt, not converted, and sync keeps a second `Refusal` type. Add the `From` conversions and replace `refused_line`.
- 🔵 minor (medium) `cli/visualiser/server/src/indexer.rs:795-844`: trust state is two separately locked fields kept in step by hand, and `judge_trust` both changes state and returns a bare bool. Use one `RwLock<Trust>` enum and a named `TrustChange`.
- 🔵 suggestion (high) `cli/visualiser/server/src/server.rs:99-106`: the boot warning repeats `work_item_problem`'s chain.
- 🔵 minor (medium) `cli/visualiser/server/src/config.rs:175-198`: `id_token` restates the key, tracker-key and draft grammars *(merged into inline comment)*.
- 🔵 suggestion (medium) `cli/work/src/well_formed.rs:23-24`: `Malformation::NoSlug` now mostly means "the filename does not name the item's ID". Rename it.
- 🔵 suggestion (medium) `cli/work/src/rekey/sequence.rs:94`: `pattern.cap().unwrap_or(u64::MAX)` is spelled out in several places. Add `IdPattern::holds_number`.

**General Findings**:
- 🔵 suggestion: the frontend's `legacyIdToken` project-code fallback can't be reached in production, because the server omits `key` whenever it omits `idToken`.

### Test Coverage

**Summary**: Every review-3 test-coverage finding has a fix in the delta, and the new Rust behaviour is mostly tested at the right level by tests that would catch a regression. What remains:
- the eval-contract lane skips quotes written in another form, and every non-`E_ID_` code;
- one rewritten resolve branch has no test;
- the `external_id` boundary narrowed when a test case was deleted;
- the newest benchmark records for update, review and refine can't be verified.

**Strengths**:
- Targeted evals now exist:
  - create-work-item 73 and 74;
  - rekey-work-items 13 and 14;
  - exit-4 evals in update, review and refine.
- `skill_digest` hashing comes with synthetic tests.
- A four-item test kills the `min_by`/`max_by`, unsorted and reversed mutants.
- The sync exemption test requires exit 74.
- `gated-commands.txt` is compared two ways.
- The eval contract runs against the built binary.
- `well_formed.rs` pins the exact refusal text for each variant.
- Visualiser trust flips are tested in both directions.
- The id_pattern walker is cross-checked against `IdParts::parse`.

**Comments**:
- 🔵 minor (high) `tests/integration/evals/test_create_work_item_eval_contract.py:25-27`: the eval contract silently skips eval 46's quotes and every non-`E_ID_` code *(inline)*.
- 🔵 minor (high) `cli/work/src/resolve.rs:202-208`: the rewritten keyless pattern-shape bare-number branch has no test *(inline)*.
- 🔵 minor (medium) `cli/work/src/requested_id.rs:1009-1015`: a deleted test case leaves the in-shape keyed `external_id` boundary unpinned, and the `Local` arm of `names_a_tracker_issue` is never exercised.
- 🔵 minor (medium) `cli/visualiser/server/src/indexer.rs:351-356`: the visualiser's non-UTF-8 work-item path has no test.
- 🔵 minor (high) `skills/work/update-work-item/evals/benchmark.json:105-116`: the new corpus-refusal eval records carry placeholder evidence and no digest.
- 🔵 minor (medium) `skills/work/list-work-items/SKILL.md:180-186`: the new list-warning and sync promotion-recovery branches have no evals.

**General Findings**:
- 🔵 suggestion: the `skill_digest` guard can't tell a re-run from a pasted digest. Record the digest on each run instead.

### Compatibility

**Summary**: All three review-3 compatibility findings are fixed, and the CLI contract changes hold together with what the skills quote. One consumer gap remains: the SPA still filters the project facet by splitting `workItemId` at the first `-`, and `formatDocId` still pads IDs. A smaller upgrade-path gap sits in the CHANGELOG Migrations step.

**Strengths**:
- update-work-item only treats `E_RESOLVE_INVALID:` as unrecognised input.
- `idToken` is an additive field with a legacy fallback.
- The `work list` warn-instead-of-refuse change is reflected everywhere it is documented.
- The gated-command set is one fixture.
- The eval contract runs against the real binary.
- `restartNeeded` gives the banner a machine-readable split.

**Comments**:
- 🔵 minor (high) `cli/visualiser/server/src/indexer.rs:89-93`: the server's project facet reads the key through the scheme, but the SPA still splits `workItemId` at `-` *(inline)*.
- 🔵 suggestion (medium) `CHANGELOG.md:1168-1171`: the Migrations escape hatch omits a legacy retirement record that blocks both re-key and sync.

**General Findings**:
- 🔵 suggestion: `idToken` is case-sensitive, while the CLI reads keys and literal text in any case.

### Usability

**Summary**: All seven review-3 usability findings are fixed. What remains are smaller problems in the new messages:
- the banner squashes the multi-line malformed list into one paragraph;
- the gate's fix for a number above the cap points at renumbering by hand, while re-key says to widen the pattern;
- the unreadable-record remedy names a directory, not the record;
- four skills still paraphrase the malformed refusal as one file with the old fixes.

**Strengths**:
- `E_REKEY_REQUIRED` gives a CLI route with a preview step.
- `E_WORK_ITEM_MALFORMED` lists every item.
- `work list` lists with a warning instead of refusing.
- The new malformations come with fixes a user can act on.
- The banner explains its effect and whether it needs a restart.
- The fold dialogue previews every picked key once.
- The configure skill documents the `--from-key` fold.
- update-work-item relays config refusals.

**Comments**:
- 🔵 minor (high) `WorkItemProblemBanner.tsx:24`: the banner squashes the multi-item malformed list into one run-on paragraph. Add `white-space: pre-wrap`, or render the items as a list.
- 🔵 suggestion (medium) `WorkItemProblemBanner.tsx:20-29`: the banner's opening overstates which commands refuse, and "restart" doesn't say how.
- 🔵 minor (high) `cli/work/src/rekey/outcome.rs:603-605`: the gate's fix for a number above the cap conflicts with re-key's *(inline)*.
- 🔵 minor (high) `cli/work/src/rekey/outcome.rs:371-379`: "move each unreadable record aside" names the directory, not the record *(inline)*.
- 🔵 minor (high) `skills/work/create-work-item/SKILL.md:96-97`: skills still paraphrase `E_WORK_ITEM_MALFORMED` as one file with two fixes *(inline, merged with documentation)*.
- 🔵 minor (medium) `cli/work-cli/src/list.rs:586`: `work list`'s warning says "then re-run this command" about a command that succeeded.
- 🔵 suggestion (medium) `cli/work/src/well_formed.rs:69`: the NoSlug fix gives a `<slug>` placeholder when the slug is known, and assumes the `id` is the correct half.
- 🔵 suggestion (low) `skills/work/sync-work-items/SKILL.md:263-269`: the skill tells the user to run `accelerator work sync` by hand instead of offering to run it.

**General Findings**: none.

### Documentation

**Summary**: Most review-3 documentation findings are fixed, but three remain:
- the scheme-validating list still omits `id check`;
- the first Migrations step still skips `E_WORK_ID_PATTERN_INVALID`;
- some CHANGELOG lines still run past 80 columns.

The delta also brings new drift:
- the CHANGELOG and PR description promise a bare-number `workItemId` display the visualiser can never show;
- the skills and Migrations still quote `E_WORK_ITEM_MALFORMED` fixes that predate the new malformation kinds;
- the PR description still describes the removed `eval_contract.rs`.

**Strengths**:
- The "every ID command refuses" overstatement is fixed across the cookbook, the configure skill, the work-items page and the sync guide.
- The garbled `--id` sentence is fixed, and the stale indexer comment is deleted.
- The `work list` behaviour change is covered consistently.
- `/rekey-work-items` and the configure skill document the fold.
- The `refuse_malformed` rustdoc lists all four kinds.

**Comments**:
- 🟡 major (high) `CHANGELOG.md:238-246`: the CHANGELOG promises a bare-number `workItemId` display the visualiser never shows *(inline)*.
- 🔵 minor (high) `meta/prs/178-description.md:317-320`: the PR description repeats that claim.
- 🔵 minor (high) `meta/prs/178-description.md:133-138`: the PR description still describes the deleted `eval_contract.rs`.
- 🔵 minor (high) `CHANGELOG.md:884-893`: the scheme-validating command list still omits `id check` (open since review 3).
- 🔵 minor (high) `CHANGELOG.md:1162-1172`: the first Migrations step misses the pattern refusal and the newer malformed-item fixes.
- 🔵 minor (high) `CHANGELOG.md:247-256`: the malformed-item entry is a run-on sentence, and lines 201, 253, 353 and 684 run past 80 columns.
- 🔵 minor (medium) `CHANGELOG.md:286-293`: "configured tracker's issue key" also covers repositories with no tracker. Match the rustdoc.
- 🔵 minor (medium) `CHANGELOG.md:1186-1190`: "`work` commands that read or write IDs refuse" survives here, at lines 189-190, and in `configure/SKILL.md` lines 719 and 832.
- 🔵 minor (high) `skills/work/create-work-item/SKILL.md:96-97`: the skills paraphrase a single-file, two-fix `E_WORK_ITEM_MALFORMED` *(merged into inline comment)*.
- 🔵 minor (high) `cli/work-cli/src/exit_codes.rs:193-196`: the module doc describes the old `E_WORK_ITEM_MALFORMED` and `E_REKEY_REQUIRED` texts.
- 🔵 suggestion (medium) `cli/visualiser/server/src/config.rs:171-173`: the idToken docs claim a bare number under `{tracker}`, and give "older servers" as the reason for absence *(merged into inline comment)*.

**General Findings**:
- 🔵 minor: the PR description's *Taken rule* bullet predates `fields_taking`.

### Security

**Summary**: The changes after review 3 keep the trust boundaries tight, and no issue is exploitable in the threat model.
- `RecoveryDir` is parsed both when written and when read.
- Non-UTF-8 files are a typed `Undecodable` with an escaped path.
- `idToken` and `problem` reach the page as a server-built regex source and React text nodes.

One hardening suggestion: `idToken` escapes only `.` in pattern literals.

**Strengths**:
- `Retiring::recovery_dir` is typed as `RecoveryDir`, and a tampered record naming `..` is refused.
- A hostile tracker key can't become a path component outside the recovery parent.
- Tracker-key reads on network input go through `parse_any_supported`, with traversal-shaped tokens tested.
- New refusal text passes paths through `escaped`.
- The banner renders a text node.
- The grammar rules out catastrophic backtracking.

**Comments**:
- 🔵 suggestion (medium) `cli/visualiser/server/src/config.rs:186`: `idToken` escapes only `.` in pattern literals, and its safety rests on an alphabet rule enforced in another crate *(merged into inline comment)*.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
