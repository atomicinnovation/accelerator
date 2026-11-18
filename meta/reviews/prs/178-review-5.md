---
type: "pr-review"
id: "178-review-5"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-17T19:44:39+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:178"
relates_to: ["pr-review:178-review-1", "pr-review:178-review-2", "pr-review:178-review-3", "pr-review:178-review-4"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "documentation", "security"]
review_number: 5
pr_number: 178
tags: []
last_updated: "2026-11-17T19:44:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #178 - [0319] Let work create take an explicit work item ID

**Verdict:** COMMENT

The response to review 4 holds up:
- Every lens confirms its review-4 findings were fixed in substance, not papered over.
- Create, `id next` and re-key now share one taken rule.
- ADR links have their own grammar again.
- `id_reading` breaks the convergence/re-key cycle.
- The indexer holds one `Trust` judged under the rescan lock.
- Each ID type publishes its own regex grammar, tested against its parser.
- Refusals convert with `From` rather than being unpacked and rebuilt.

Security found nothing beyond one unescaped path, and nothing is critical or major.

What remains is minor:
1. **Two correctness edges**, both on error paths: a failed rescan after a trust flip, and a directory listing failure read as one unreadable record.
2. **One allocator not yet brought under the new rule**: re-key's fresh numbering.
3. **Seams where the visualiser still holds CLI knowledge**: the gated-command list, the problem's layout and the grammar assembly.
4. **Doc and test drift** that this delta introduced.

### Cross-Cutting Themes

- **The visualiser still copies CLI-owned knowledge** (flagged by: Architecture, Usability, Code Quality, Test Coverage). Each ID type's grammar now comes from the type that owns it, but three things are still copied:
  - The banner hard-codes the six gated commands. Four lenses flagged this, and nothing ties it to `gated-commands.txt`.
  - `problemOf` gets the item list back by splitting `MalformedItems`' prose on newlines.
  - `config.rs` decides how a pattern's shape becomes an alternation, and how literals are case-folded (`regex_literal`), where `work::id_scheme` could own it.

  Serving `{ summary, items, gatedCommands }` and an `IdScheme::regex_source()` would close all three.
- **The unreadable retirement record's new blocker is half-finished** (flagged by: Correctness, Usability, Security, Test Coverage):
  - A `read_dir` failure is reported as an unreadable record named after the directory, with a per-file remedy.
  - Only the first bad record is named.
  - The path isn't passed through `escaped()`.
  - Only the `Malformed` arm is tested.
- **Two allocators, two free-number rules** (flagged by: Architecture, Correctness). `allocate_above` skips linked tracker numbers; re-key's `fresh_entries` does not, so re-key can refuse with `LinkedClash` where `id next` would have given a free number.
- **`AfterFixing` is used by `work list` but not by the visualiser** (flagged by: Code Quality, Test Coverage). The banner still says "then re-run this command", and the `E_REKEY_REQUIRED` arm of `warning()` is unpinned. The unreachable `map_or_else` fallback in `warning()` would also bring that text back.
- **Untrusted-corpus nulls are undocumented at the API boundary** (flagged by: Documentation, Compatibility):
  - `idToken`, `key`, `workItemId` and `workItemKey` are all null over an untrusted corpus, but `fetch.ts`, `types.ts` and the Rust `IndexEntry` docs don't say so.
  - `workItemKey` is not defaulted in `normaliseEntry`.

### Tradeoff Analysis

- **Server-served structure vs a thin, lockstep SPA**: Architecture and Code Quality want the server to serve the problem's items and the gated set as data. The SPA always ships embedded with its server, so the drift risk is moderate. The cheapest fix is a frontend test that reads `gated-commands.txt`. Serving the data costs one more API field, but it removes both the newline parsing and the hard-coded list. It is worth doing if the banner is expected to change again.
- **"Widen the pattern" remedy vs blast radius** (Usability): offering the widening first is consistent with re-key. Saying that it renames every item would stop a user picking it to fix one stray number.

### Strengths

- ✅ Every review-4 finding across eight lenses has a real fix.
- ✅ **One taken rule:** an `external_id` naming a tracker issue holds its number in `create --id`, `id next` and `refuse_collisions`. `allocate_above` handles overflow at the cap and at `u64::MAX`.
- ✅ **No more cycle:** `work::id_reading` imports nothing from `rekey` or `convergence`, so the two-way dependency is gone. `UnclassifiableCause::describe` and `remedy` keep behaviour next to its data.
- ✅ **One trust state:** `Indexer::trust` is a single `RwLock<Trust>`, judged only under `rescan_lock`. The config endpoint takes `key`, `idToken` and `problem` from one snapshot, and an untrusted corpus serves no grammar.
- ✅ **Grammars from their owners:** `IdKey::REGEX_SOURCE`, `DraftId::REGEX_SOURCE` and `TrackerKeys::regex_source()` are pinned to their parsers in `id_grammars.rs`. `regex_literal` escapes every metacharacter, and there are no capture groups or nested quantifiers.
- ✅ **ADR links fixed:** they have their own `(ADR)-(\d+)` alternative, so they match under `{tracker}` again.
- ✅ **Project facet:** the served `workItemKey` drives the facet on both server and client, and the `cfg` threading and `indexOf('-')` split are gone.
- ✅ **IDs shown as served:** `formatDocId` and `legacyIdToken` were deleted rather than worked around.
- ✅ **Refusal types:**
  - `From<&RequestedIdRefusal> for Refusal`, `NotSynced`, `Unopened`/`Unadmitted` and `From<Unadmitted> for RunOutcome` remove duplicated matches.
  - Create-batch no longer handles a refusal it can never get.
- ✅ **Refusal text:**
  - The banner lists each malformed item and gives accurate restart guidance.
  - `NumberAboveCap` says to widen the pattern.
  - The misnamed-item fix keeps the known slug.
  - `work list` no longer asks to re-run a command that succeeded.
  - The skills relay every malformed item verbatim, and `/sync-work-items` offers to run sync.
- ✅ **Tests and evals:**
  - The keyless resolve branch, the keyed `external_id` boundary (with a create-then-re-key test) and the visualiser's non-UTF-8 path are all pinned.
  - Benchmarks carry real evidence and digests across six skills.
  - The eval contract scans every work skill.
- ✅ **Docs:** the CHANGELOG visualiser, taken-rule, Migrations and `exit_codes` docs now match the code.

### General Findings

- 🔵 **Code Quality**: the `Unadmitted::Refused` variant name still clashes with `RunOutcome::Refused` and `Unopened::Untrusted` over different payloads. Renaming it `IdRefused`, as review 4 suggested, would read more clearly. Ignore this if it was declined deliberately.

### Additional Findings

- 🔵 `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:60` — the restart step uses `accelerator visualiser stop`, which needs the CLI on `$PATH`; `/accelerator:visualise stop` doesn't, and the URL changes after a restart (usability)
- 🔵 `cli/visualiser/server/src/server.rs:70` — `AppState::work_item_problem` has no callers left; `untrusted_corpus_problem` is now test-only (code-quality)
- 🔵 `cli/work/src/corpus_trust.rs:52` — `warning()`'s `map_or_else` fallback is unreachable, and would return the refusal text it exists to avoid (code-quality)
- 🔵 `cli/visualiser/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:80` — the comment still says the ID is formatted (code-quality)
- 🔵 `cli/visualiser/frontend/src/api/wiki-links.ts:24` — `wikiLinkOf` reads matches by positional capture groups; group 3 is unused. Use named groups (code-quality)
- 🔵 `cli/visualiser/frontend/src/api/fetch.ts:86` — the `idToken` doc omits the untrusted-corpus case, and `key` is undocumented (documentation)
- 🔵 `cli/visualiser/frontend/src/api/types.ts:130` — the `workItemId`/`workItemKey` docs miss that both are null over an untrusted corpus; same in `indexer.rs` `IndexEntry` (documentation)
- 🔵 `skills/work/sync-work-items/SKILL.md:263` — the `E_REKEY_REQUIRED` bullet says "relay … and stop", then asks a question; put the branch first (documentation)
- 🔵 `skills/work/create-work-item/evals/benchmark.md:3` — the Model and Date header lines omit the 2026-11-17 runs (documentation)
- 🔵 `tests/integration/evals/test_work_skill_eval_contract.py:238` — the contract checks exit codes only, so the reworded remedies quoted by create 75, list 1 and sync 1 are unchecked, and `warning:` lines are skipped (test-coverage)
- 🔵 `skills/work/extract-work-items/SKILL.md:360` — the changed malformed-item relay has no eval, and the skill is missing from `DIGEST_GUARDED` (test-coverage)
- 🔵 `cli/work/src/rekey/outcome.rs:367` — the remedy names only the first bad record; the directory fallback and the "unreadable record" lead read oddly (usability)
- 🔵 `cli/work/src/well_formed.rs:70` — the misnamed-item fix falls back to `<slug>` when the stem has no ID, though the whole stem is the slug (usability)
- 🔵 `cli/work/src/id_reading/unclassifiable.rs:42` — "Widen `work.id_pattern`" doesn't say re-key will then rename every item (usability)
- 🔵 `cli/visualiser/server/src/config.rs:178` — scheme-level grammar assembly lives in the visualiser; consider `IdScheme::regex_source()` (architecture)
- 🔵 `cli/visualiser/server/src/config.rs:229` — `regex_literal` also case-folds letters, which its name doesn't say (code-quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:1589` — the project facet keys on the key's case as written, while links and the CLI fold case (compatibility)
- 🔵 `cli/visualiser/frontend/src/api/types.ts:133` — `workItemKey` isn't defaulted in `normaliseEntry` like `draft` (compatibility)
- 🔵 `cli/work-adapters/src/rekey/guards.rs:66` — the unreadable record's path reaches refusal text unescaped (security; folded into the guards.rs inline comment)
- 🔵 `meta/prs/178-description.md:142` — "holds every refusal code" overstates the eval contract, which exempts six codes (documentation)
- 🔵 `CHANGELOG.md:897` — an orphaned one-word line was left after adding `id check` (documentation)


## Inline Comments

### `cli/visualiser/server/src/indexer.rs:470-480` — A failed rescan after a trust flip leaves the index stale until the next flip
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`refresh_one` stores the new trust with `hold_trust` before it calls `self.rescan()`. `rescan` also stores trust (line 381) before Phase 3, and Phase 3 returns early on any read error other than `NotFound`. If the rescan fails, the trust state already records the flip, but `entries` and the secondary maps were built under the old reading. Every later `refresh_one` then finds `before.is_trusted() == now.is_trusted()` and never rescans.

**Impact**: one transient I/O error during the rescan after a flip leaves the visualiser in either of two wrong states until trust flips again or the server restarts:
- it serves work-item IDs, keys and clusters from a corpus it reports as untrusted;
- or it reports trusted while serving no IDs.

**Suggestion**: store the judged trust only once the rescan's maps are swapped in. Either have `rescan` judge and hold trust in the same step that replaces the maps, with `refresh_one` deciding to rescan without storing first, or restore the previous trust when `rescan` returns `Err`. Pin it with a test whose rescan fails once.

---

### `cli/work-adapters/src/rekey/guards.rs:174-181` — Unreadable-record blocker: a directory listing failure gets a per-file remedy (Correctness + Usability + Security + Test Coverage)
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, usability, security, test-coverage

🔵 **Correctness**

`unreadable_record` treats every `StoreError::Io { path }` as naming a record file. But `FileRetirementRecords::outstanding` also returns `Io`, with `path` set to the `retirement-records` directory, when `read_dir` fails (e.g. a permissions error). The blocker then names the directory, and the remedy ("rename it so its name no longer ends in `.json`, or move it out of `retirement-records/`") tells the user to set aside every in-flight record that sync needs.

🔵 **Usability**: `outstanding()` stops at the first failure, so two corrupt records take two rounds to find. The remedy's lead, "Sync cannot read an unreadable record either", reads as a tautology (`rekey/outcome.rs:367`).

🔵 **Security**: `blocker_line` prints this path with `path.display()` rather than `escaped()`. Every other refusal that names a corpus or state file escapes it, so a record name containing a newline or control characters reaches the relayed refusal verbatim.

🔵 **Test Coverage**: only the `Malformed` arm is pinned. Neither the per-file `Io` arm nor the directory case has a test.

**Suggestion**:
- Return a path only for `Malformed` and for per-file `Io` errors.
- Report a directory that cannot be listed as its own blocker, with a remedy that fits a directory.
- Escape the path in `blocker_line`.
- Add tests for an unreadable `*.json` file and for an unlistable directory.

---

### `cli/work/src/rekey/mapping.rs:188-197` — Re-key's fresh-number allocator doesn't skip linked tracker numbers, though create's now does (Architecture + Correctness)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, correctness

🔵 **Architecture / Correctness**

The delta taught `next_number::allocate_above` to skip numbers held by an `external_id` that names a tracker issue (`linked_tracker_numbers`). Re-key's `fresh_entries` still hands out `floor + 1..=floor + needed` in order, then `refuse_collisions` runs `linker_of_under` over those targets.

Example: `work.key` is `ACC`, item A is `ACC-0001` with `external_id: ACC-2`, and a draft needs a fresh number.
- `work create` / `id next` skip 2 and give `ACC-0003`.
- Re-key gives the draft `ACC-0002`, then refuses with a `LinkedClash`. Its remedy says to change `work.key` or switch to `{tracker}`, although a free number exists.

**Impact**: the two allocators disagree about which numbers are free, which is the kind of disagreement review 4's taken-rule theme set out to remove. The failure is only a refusal, so nothing is corrupted.

**Suggestion**: move "the next N free numbers above a floor, skipping linked numbers, within the cap" into one `work` function that both `allocate_above` and `fresh_entries` call. Pin it with a re-key test in which a draft's fresh number would land on a linked key.

---

### `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:44-48` — The banner hard-codes the CLI's gated-command set (Architecture + Usability + Code Quality + Test Coverage)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, usability, code-quality, test-coverage

🔵 **Architecture / Usability / Code Quality / Test Coverage**

The banner names the six commands that refuse an untrusted corpus (`work create`, `create-batch`, `id next`, `id check`, `resolve`, `promote`) as literal JSX, and its test asserts the same literal. That set is work-cli's gating policy. It is pinned in `cli/work-cli/tests/fixtures/gated-commands.txt` and described in `exit_codes.rs` and the CHANGELOG, and nothing ties this fourth copy to the fixture.

**Impact**: when a command gains or loses the trust gate, the banner silently names the wrong commands (the drift review 4 raised), and no test fails.

**Suggestion**: serve the gated set from `/api/work-item/config`, sourced from a constant that work-cli's gate also uses. At minimum, have `WorkItemProblemBanner.test.tsx` read `gated-commands.txt` (via `fs` or a `?raw` import) and assert that the banner names exactly those commands.

---

### `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:12-21` — The banner recovers a list by splitting the CLI's refusal prose on newlines (Architecture + Code Quality)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`problemOf` treats the first line of `problem` as a summary and every later non-blank line as an item. That layout belongs to `work`'s `Display` for `MalformedItems` and `RekeyRequired`; the HTTP contract serves `problem` as an opaque string. The server already holds the structure: `MalformedItems` has items, each with a `what()` and a `remedy()`.

**Impact**: if the refusal gains a line after the list, or wraps an item over two lines, the SPA renders it as a bullet without complaint, and no server-side test fails.

**Suggestion**: serve `problem` as `{ summary, items: [...] }` built from `UntrustedCorpus`'s variants, or add a `problemItems` array next to the text, so the banner renders data instead of parsing prose.

---

### `cli/visualiser/server/src/indexer.rs:875` — The visualiser still renders the refusal's 're-run this command', contradicting its own banner
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

The delta adds `AfterFixing` and `UntrustedCorpus::warning()` for contexts where no command was refused, but `judge_trust` still renders the problem with `refusal.relative_to(..).to_string()`. The banner therefore tells the user to "then re-run this command", although the visualiser ran none and the banner itself says the problem clears by itself. The new indexer test pins that wording.

**Impact**: the new domain concept is used by `work list` but not by the other consumer it was made for, and the visualiser's text contradicts itself.

**Suggestion**: store `refusal.relative_to(&self.project_root).warning()` in `Trust::Untrusted` and update the indexer test to match.

---

### `cli/work-cli/tests/cli_rekey_required.rs:140-158` — The E_REKEY_REQUIRED branch of the list warning is never checked for dropping 're-run this command'
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`UntrustedCorpus::warning()` has two arms, both passing `AfterFixing::NothingMore`. Only the single-item malformed arm is pinned (`cli_list.rs` asserts `!stderr.contains("re-run this command")`); this test checks only the `E_REKEY_REQUIRED` warning's prefix. If the `RekeyRequired` arm is flipped back to `RerunThisCommand`, every test still passes. The multi-item malformed header ("Fix each one as shown:") is also untested.

**Impact**: review 4's fix could regress unnoticed for the case `work list` users meet most: an off-pattern corpus.

**Suggestion**: here, assert `!stderr.contains("re-run this command")` and check how the warning ends. Add a `corpus_trust` unit test that renders `warning()` over a two-item `MalformedItems`.

---

### `cli/work/src/next_number.rs:194-198` — No test pins that only the allocating key's linked tracker numbers are skipped
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

All three new allocation tests use an `external_id` under the key being allocated (`PP-2`, `PP-905`, `PP-99` while allocating `PP`). If `.filter(in_sequence)` is deleted, every test still passes, and allocation would then skip `PP-0002` because some item links `XY-2`.

**Impact**: `id next` and `create` could silently hand out non-contiguous IDs, and the overflow arithmetic near the cap would count another key's numbers.

**Suggestion**: add a test in which `PP-0001` has `external_id: "XY-2"`; allocating two under `PP` must give `["PP-0002", "PP-0003"]`.

---

### `cli/visualiser/frontend/src/api/wiki-links.test.ts:420` — The served idToken is never compiled as a JavaScript RegExp; frontend tests use stale hand copies
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The server tests compile `id_token()` with Rust's `regex` crate, but production compiles it with `new RegExp(...)` in `buildWikiLinkPattern`. The frontend tests use hand-copied tokens, and `trackerToken` here is already stale. The server now serves `[dD][rR][aA][fF][tT]-[0-9a-hjkmnp-tv-zA-HJKMNP-TV-Z]{6}` for drafts, not `draft-[0-9a-z]{6}`, and it builds literals as case-insensitive classes. No `/api/work-item/config` endpoint test asserts that `idToken` is served for a trusted corpus either; the only remaining assertions check that it is absent.

**Impact**: a server change that Rust accepts but JavaScript does not (an inline `(?i)`, `\z`, a class JS reads differently) would make `new RegExp` throw inside a `useMemo` and break every document view, with every test green.

**Suggestion**: have a server test write or compare a golden fixture of `{pattern, idToken, matching, non-matching}`. Have a vitest test read the same fixture, build `buildWikiLinkPattern(idToken)` and assert the same matches. Assert that `idToken` and `key` are present once the corpus converges in `the_problem_follows_the_corpus_after_boot`.

---

### `skills/work/update-work-item/evals/benchmark.json:163` — Benchmark provenance notes contradict the records they annotate
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

This note still ends "Records whose grading files were lost with the scratch directory carry the grader's verdict without its evidence". This delta replaced every placeholder evidence string in this file with real evidence. The note also attributes every run to `claude-opus-5-5` in a single pass, although update evals 3 and 4 were re-run later to carry grader evidence. The same stale sentence is in `review-work-item/evals/benchmark.json:183` and `refine-work-item/evals/benchmark.json:486`. Similarly, `create-work-item/evals/benchmark.md:3-4` says 78 evals, but its **Model** and **Date** lines leave out the 2026-11-17 `claude-sonnet-5-5` runs of evals 74 and 75.

**Impact**: these notes are the provenance record that the new `skill_digest` guard is meant to make trustworthy.

**Suggestion**:
- Drop the "lost grading files" sentence in all three files.
- Name the evals re-run on 2026-11-17, with the executor model used.
- Add those runs to the `benchmark.md` header.

---

## Per-Lens Results

### Architecture

**Summary**: All three architecture findings from review 4 are fixed:
- `id_reading` is a neutral module, so the two-way dependency is gone.
- The indexer holds one `Trust`, written only under `rescan_lock`.
- The config endpoint reads `key`, `idToken` and `problem` from one snapshot.

Two kinds of gap remain. Re-key's fresh-number allocator does not skip linked tracker numbers. The visualiser also still holds some CLI-owned rules: how a scheme's grammar is assembled, how the problem text is laid out, and which commands are gated.

**Strengths**:
- No dependency cycle: `id_reading` imports nothing from `rekey` or `convergence`.
- One `RwLock<Trust>`, judged under `rescan_lock`. `rescan` captures `reading` once per rebuild.
- The config endpoint uses one trust snapshot. `workItemProblem` is folded into `workItemConfig`.
- The served `workItemKey` drives the project facet. `formatDocId` is deleted, and the facet functions no longer take `&Config`.
- Each type publishes its own grammar (`REGEX_SOURCE`, `regex_source()`), checked in `id_grammars.rs`.
- `AfterFixing` and `UntrustedCorpus::warning()` are a named seam.
- The work-cli `Refusal` conversions and the `Unopened`/`Unadmitted` split.
- `StoreError::Malformed { path }` carries the failing record up to `SyncBlocker::UnreadableRecord`.

**Comments**:
- `cli/work/src/rekey/mapping.rs:188-197` (minor, medium): re-key's fresh-number allocator doesn't skip linked tracker numbers, though create's now does.
- `cli/visualiser/server/src/config.rs:178-202` (suggestion, medium): scheme-level grammar assembly lives in the visualiser, not `work::id_scheme`.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:12-21` (minor, medium): the banner gets the list's structure by splitting the CLI's refusal text on newlines.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:44-48` (suggestion, medium): the SPA hard-codes the CLI's gated commands.

**General Findings**: none.

### Correctness

**Summary**: Both correctness findings from review 4 are fixed:
- One taken rule across create, `id next` and re-key.
- ADR links have their own grammar alternative.

The trust rework is sound in the normal case. Two error-path edges remain:
- A failed rescan uses up a trust flip without rebuilding the index.
- A `read_dir` failure is reported as one unreadable record, with a per-file remedy.

**Strengths**:
- `allocate_above` skips linked numbers without raising the floor, with correct overflow at the cap and at `u64::MAX`.
- The capture-group indices are stable, because the served token has no groups and `regex_literal` escapes `(`.
- `refresh_one` compares the replaced trust with the new one inside one `rescan_lock` section.
- The config endpoint reads from a single trust snapshot.
- The draft alphabet class matches Crockford's `ALPHABET` exactly.
- The project facet uses the served `workItemKey` on both sides.

**Comments**:
- `cli/visualiser/server/src/indexer.rs:470-480` (minor, medium): a failed rescan after a trust flip leaves the index stale until the next flip.
- `cli/work-adapters/src/rekey/guards.rs:174-181` (minor, medium): a `read_dir` failure is reported as an unreadable record named after the directory, with a per-file remedy.

**General Findings**:
- (suggestion) Re-key's fresh numbering still ignores the linked tracker numbers that `allocate` now skips.

### Code Quality

**Summary**: Nearly all of review 4's code-quality findings are fixed:
- `From` conversions replace unpacking and rebuilding refusals.
- Create-batch gets the narrower `Unopened` error.
- The indexer holds a single `RwLock<Trust>`.
- `id_reading` is neutral, `highest_number` exists, and `Misnamed` replaces `NoSlug`.

The new defects are small leftovers from the refactors: dead code, redundant ternaries, a stale comment, positional capture groups, and an unreachable fallback.

**Strengths**:
- `Unopened` separates failures that happen before a request is read. `From<Unadmitted> for RunOutcome` removes the duplicated match.
- `From<&RequestedIdRefusal> for Refusal`, and `NotSynced` with `From`.
- One `RwLock<Trust>`, a pure `judge_trust`, and `reading_under(&trust)`.
- `UnclassifiableCause::describe` and `remedy` keep behaviour with its data.
- `work_item_key` removes the `cfg` parameter from the facet functions.
- Each ID grammar is published by its owning type and tested.
- `formatDocId` and `legacyIdToken` are deleted, and `AfterFixing` replaces a bool flag.

**Comments**:
- `cli/visualiser/server/src/server.rs:70-76` (minor, high): `AppState::work_item_problem` has no callers left.
- `cli/work/src/corpus_trust.rs:52-62` (minor, high): `warning()` falls back to the very text it exists to avoid.
- `cli/visualiser/server/src/indexer.rs:875` (minor, medium): the visualiser keeps the refusal wording although `AfterFixing` now models the alternative.
- `cli/visualiser/server/src/config.rs:229-243` (suggestion, medium): `regex_literal` also case-folds letters, which its name doesn't say.
- `cli/visualiser/frontend/src/api/wiki-links.ts:24-39` (minor, medium): `wikiLinkOf` reads the match by positional capture groups.
- `cli/visualiser/frontend/src/routes/kanban/WorkItemCardPresentation.tsx:39` (suggestion, high): a pointless ternary left over from removing `formatDocId`. The same shape is in `LifecycleClusterView.tsx` and `RelatedArtifacts.tsx`.
- `cli/visualiser/frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx:80-84` (minor, high): the comment still says the work-item ID is formatted.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:44-48` (minor, medium): the banner hard-codes the CLI's refusing commands.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:14-21` (suggestion, medium): `problemOf` gets the structure back by splitting prose.

**General Findings**:
- (suggestion) The `Unadmitted::Refused` rename to `IdRefused` from review 4 is still open.

### Test Coverage

**Summary**: Each of review 4's six test-coverage findings has a real fix, and most new behaviour has tests that would catch a plausible regression. The remaining gaps are small:
- **Mutants the tests miss:**
  - the list warning's `E_REKEY_REQUIRED` arm;
  - the same-key filter on linked numbers.
- **Contracts that cross a boundary untested:**
  - the served `idToken`, both on the wire and as a JavaScript `RegExp`;
  - the text quoted in evals;
  - the banner's list of commands.

**Strengths**:
- The keyless `resolve_bare_number` branch is covered under `{number:05d}` and `v{number:03d}`.
- The keyed `external_id` boundary is pinned both ways, with a create-then-re-key test.
- `id_grammars.rs` checks each grammar against its parser, and escaping is tested.
- The visualiser's non-UTF-8 path has a test.
- The eval contract scans every work skill, and its quote parser has a unit test.
- The benchmark guard spans six skills, and the records carry real evidence.
- New frontend tests cover the multi-item banner, `workItemKey` filtering, case-insensitive resolution, ADR beside tracker keys, and unpadded tracker keys.

**Comments**:
- `cli/work-cli/tests/cli_rekey_required.rs:140-158` (minor, high): the `E_REKEY_REQUIRED` branch of the list warning is never checked for dropping 're-run this command'.
- `cli/work/src/next_number.rs:194-198` (minor, high): no test pins that only the allocating key's linked numbers are skipped.
- `cli/visualiser/server/src/api/work_item_config.rs:130` (minor, medium): no endpoint test asserts `idToken` is served for a trusted corpus.
- `cli/visualiser/frontend/src/api/wiki-links.test.ts:420` (minor, medium): the served token is never compiled as a JavaScript `RegExp`, and the hand copies are stale.
- `tests/integration/evals/test_work_skill_eval_contract.py:238-243` (minor, medium): the eval contract checks exit codes only, so reworded remedies go unchecked.
- `skills/work/extract-work-items/SKILL.md:360-361` (minor, medium): `extract-work-items` changed its relay but has no eval and no digest guard.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:45-47` (suggestion, medium): nothing ties the banner's command list to the fixture.

**General Findings**:
- (suggestion) The `Io` and directory fallback paths in `unfinished_sync_work` are untested.

### Compatibility

**Summary**: Both of review 4's compatibility findings are fixed:
- The SPA facet filters on the served `workItemKey`, and `formatDocId` is gone.
- The Migrations step names `E_WORK_ID_PATTERN_INVALID`, the per-item fixes and the legacy-record escape hatch.

The case-sensitivity point is addressed too. Two low-impact nits remain on `workItemKey`.

**Strengths**:
- `work_item_key` is computed once and read by both the server and the SPA facet.
- `formatDocId` is removed, and the CHANGELOG states the unpadding.
- `idToken` honours the tracker's key rules, escapes literals and folds case.
- An untrusted corpus serves no `idToken` or `key`, and a test pins this.
- The `work list` warning text and the skills relaying it agree.
- The CHANGELOG Migrations step is complete.
- The public-API fixtures and the lock file are updated.

**Comments**:
- `cli/visualiser/server/src/indexer.rs:1589-1591` (suggestion, medium): the project facet keys on the key's case as written.
- `cli/visualiser/frontend/src/api/types.ts:133-135` (suggestion, low): `workItemKey` is not normalised at the wire boundary.

**General Findings**: none.

### Usability

**Summary**: Every review-4 usability finding is fixed properly:
- The banner lists items and names the commands that actually refuse.
- The cap remedy says to widen the pattern.
- The unreadable-record remedy names the file.
- `work list` no longer asks to re-run a command that succeeded.
- The misnamed-item fix keeps a known slug, and the skills relay every item.

What remains is small: the banner's restart steps, the precision of a few remedies, and the hard-coded command list.

**Strengths**:
- `AfterFixing` makes the 're-run this command' tail optional.
- The banner lists one entry per item.
- The banner's opening matches what really refuses.
- `NumberAboveCap` has its own remedy.
- The unreadable record is named by path.
- The misnamed-item fix reuses the existing slug.
- The skills relay every item as printed, and `/sync-work-items` offers to run sync.

**Comments**:
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:60-65` (minor, medium): the restart steps use a terminal command that needs extra setup.
- `cli/work/src/rekey/outcome.rs:367-369` (suggestion, medium): the remedy names only the first bad record, and the directory fallback reads oddly.
- `cli/work/src/well_formed.rs:70-74` (suggestion, medium): the misnamed-item fix falls back to `<slug>` when the whole stem is the slug.
- `cli/work/src/id_reading/unclassifiable.rs:42-45` (suggestion, low): "Widen `work.id_pattern`" doesn't say it re-keys every item.
- `cli/visualiser/frontend/src/components/WorkItemProblemBanner/WorkItemProblemBanner.tsx:44-48` (suggestion, low): the banner hard-codes the list of commands that refuse.

**General Findings**: none.

### Documentation

**Summary**: Nearly all of review 4's documentation findings are fixed:
- The CHANGELOG visualiser, taken-rule, malformed, scheme-list and Migrations entries match the code.
- The `exit_codes` doc and four skills describe the multi-file `E_WORK_ITEM_MALFORMED`.
- The retirement-record remedy is identical everywhere.

What remains is smaller drift that this delta introduced:
- Benchmark provenance notes contradict their records.
- The new API docs miss the untrusted case.
- The sync skill's recovery branch contradicts itself.

**Strengths**:
- The CHANGELOG visualiser entry now matches the code.
- The taken-rule and allocation wording matches `next_number.rs`.
- The retirement-record remedy is word-for-word consistent.
- The `exit_codes.rs` doc and the skills describe the multi-file malformed refusal.
- `configure/SKILL.md` and the Migrations step give the exact list of refusing commands.
- The stale `fetch.ts` 'older servers' comment is gone.
- The PR description's bullets are refreshed.

**Comments**:
- `skills/work/update-work-item/evals/benchmark.json:163` (minor, high): the notes say records lack evidence, and omit the later re-runs. The same applies to the review and refine benchmarks.
- `skills/work/create-work-item/evals/benchmark.md:3-4` (minor, high): the Model and Date lines omit the 2026-11-17 runs.
- `cli/visualiser/frontend/src/api/fetch.ts:86-87` (minor, high): the `idToken` doc omits the untrusted-corpus case.
- `cli/visualiser/frontend/src/api/types.ts:130-134` (minor, medium): the `workItemId`/`workItemKey` docs miss that both are null over an untrusted corpus.
- `skills/work/sync-work-items/SKILL.md:263-271` (minor, medium): "relay … and stop", then a question to the user.
- `meta/prs/178-description.md:142` (suggestion, medium): "holds every refusal code" overstates the lane.
- `CHANGELOG.md:897-900` (suggestion, high): an orphaned one-word line.

**General Findings**: none.

### Security

**Summary**: The trust boundaries hold, and review 4's hardening item is fixed:
- `idToken` is assembled from grammar constants owned and tested by their types.
- `regex_literal` escapes every metacharacter.
- An untrusted corpus serves neither `idToken` nor `key`.
- The banner renders React text nodes from text that has already been escaped.

The only new seam is a retirement-record path printed unescaped.

**Strengths**:
- `regex_literal` escapes every metacharacter, with a test.
- The served grammars are pinned to their parsers and have no nested quantifiers, so ReDoS is ruled out.
- The served grammars have no capturing groups, so the group indices cannot shift.
- An untrusted corpus serves no grammar or key, and a test pins this.
- The banner uses no `dangerouslySetInnerHTML`, and its item lines are built with `escaped()`.
- `UnclassifiableCause::describe` and `Malformation::Undecodable` escape paths and IDs.
- `workItemKey` is derived through `IdScheme::read_held`.

**Comments**:
- `cli/work-adapters/src/rekey/guards.rs:66` (minor, low): the unreadable retirement-record path reaches refusal text unescaped.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
