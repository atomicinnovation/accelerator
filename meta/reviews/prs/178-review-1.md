---
type: "pr-review"
id: "178-review-1"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-02T19:47:12+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:178"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "security"]
review_number: 1
pr_number: 178
tags: []
last_updated: "2026-11-02T19:47:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #178 - [0319] Let work create take an explicit work item ID

**Verdict:** COMMENT

The PR is well built. Identity moves into `work` behind narrow `corpus` ports, so dependencies point inward. `RequestedId` gives every refusal a typed code and a remedy, and tests cover each refusal with and without `--push`, plus a race against the held create lock. No lens found anything critical or exploitable. Three major findings matter most: refusing previously accepted `work.id_pattern` values can break existing repositories on upgrade with no warning; the `--dry-run --id` check order is kept in two places; and `/create-work-item` skips confirmation for a mistyped key. Beside those, several correctness edges arise where the one grammar meets legacy IDs, other separators and key-after-number patterns.

### Cross-Cutting Themes

- **Readings that sit outside the one grammar** (flagged by: Correctness, Code Quality, Architecture). `id_scheme.rs` still has hand-written `-`-separated parsers (`keyed_digits`, `tracker_key_prefix`, `legacy_prefix`) and joins the `identity_key` fallback with `-`. The taken rule reads fewer prior-pattern shapes than re-key's `read_existing_id`, and `legacy_ids::references_key` is a lenient second reading. This explains the open question in the PR description (`PP-42` never joins under `{key}_{number:04d}`) and the two re-key collisions Correctness found. Recommendation: render the fallback through the pattern whenever the pattern has `{key}`, and have the taken rule use re-key's fallback readings.
- **Knowledge kept in two places** (flagged by: Code Quality, Architecture, Correctness). The dry-run copies `claim_id`'s check order. `migrate-cli` builds a scheme beside `work_adapters::config`, and its test double copies it again. `tokens_of` is a second brace tokeniser beside `segments_of`, and `DRAFT_SUFFIX_LENGTH` copies `draft_id::SUFFIX_LENGTH`. Correctness confirms the dry-run order matches today; the risk is drift.
- **Stricter grammar, wider blast radius** (flagged by: Compatibility, Architecture). The visualiser now refuses to boot on a pattern the new grammar refuses, even for plans and research. It also links all of `work-adapters` to read the scheme. Live `corpus linkage extract` stays on the frozen `LegacyNumericIds`, so it can disagree with the visualiser in keyed repositories.
- **Error vocabulary across the `work id` group** (flagged by: Usability, Test Coverage). `work id canonicalise` reports a bad `work.key` as `E_PATTERN`, where its siblings use `E_WORK_KEY_INVALID`. Those `E_PATTERN:` arms also lost their only tests when the `corpus-adapters` suite was deleted.

### Tradeoff Analysis

- **Fail fast vs availability**: Compatibility wants the visualiser to fall back to no work-item clustering on a refused pattern. The single-scheme design wants one strict reader for everything. Recommendation: keep the hard refusal for writing `work` commands and degrade the read-only visualiser, surfacing the `SchemeError` in its log or UI.
- **Exit-code consistency**: Usability wants `E_ID_TAKEN` to exit `4` like other collision refusals. The PR deliberately matches `work create`'s other argument refusals, which exit `1`. Either is defensible; the open point is that "ID taken" now exits `1` or `4` depending on the verb.

### Strengths

- ✅ `corpus` declares narrow ports (`IdConvention`, `LinkageIds`) that `work` implements, enforced by pup `allowed_only` and the crate-dependencies rules. `regex` leaves `corpus-adapters` and the visualiser, removing a config-to-regex surface.
- ✅ `RequestedId::admit` is the only gate between a caller's string and a filename. It accepts only ASCII in the pattern's exact shape under a 255-byte bound, with tests for `../0400`, `sub/0400`, NUL, newline, full-width and Arabic-Indic digits. Every echoed value goes through `escaped()`.
- ✅ `RequestedIdRefusal` is a closed enum whose messages carry a code, cause and remedy; a compile-time `covered()` guard forces new variants into the tests. `E_ID_TAKEN` names the holder's path, and `E_ID_PATTERN_MISMATCH` offers a "Did you mean".
- ✅ The taken check runs on a listing read under the create lock, and a test writes the colliding file while the lock is held. Refusal tests assert both that `meta/` is unchanged and that the tracker saw no calls.
- ✅ The grammar's arithmetic is sound: a width cap of 19 keeps the cap within u64, numbers must be at least 1, allocation uses `saturating_add`/`checked_add`, and key and digit runs are read longest-first with backtracking.
- ✅ Migrations 0007/0009 read through a frozen `LegacyNumericIds`, so pending migrations keep their historical output. The retired verbs fail with a pointer to their replacement, and every caller in the repo has moved.
- ✅ `work id check` lets `/create-work-item` settle an ID before drafting, and every refusal after drafting resumes with the approved draft unchanged.

### General Findings

- 🔵 **Code Quality**: `WorkItemIdScheme` offers seven overlapping readings (`normalise_id`, `canonicalise`, `identity_key`, `read_held`, `read_held_prefix`, `is_canonical_id_token`, `is_any_key_id_token`), each with slightly different trimming, folding and padding rules. Group them by purpose at the type level.
- 🔵 **Architecture**: two `references_key` readings with different strictness. The lenient one lives in `legacy_ids` beside the typed-linkage rules; name it for migrations so live code doesn't pick it up.
- 🔵 **Usability**: the retired verbs print a bare sentence with no `E_*` prefix, unlike every other refusal. `E_VERB_RETIRED:` would let tooling detect them.
- 🔵 **Security**: `recovery_dir` in retirement records and the pending-push marker decodes without the path checks this PR adds elsewhere. It drives no filesystem operation today.
- 🔵 **Test Coverage**: the new `work-item:` inbound linkage is tested only via `parent:`, not `related:`.
- 🔵 **Compatibility**: repositories that skipped migration 0002 will see the next ID jump (`PP-0005` → `PP-0401`) as legacy bare numbers join the keyed sequence. Mention this in 0002's skip guidance.

### Additional Findings

- 🔵 `cli/work/src/id_scheme.rs:292` — `identity_key` fallback always joins with `-` (Correctness)
- 🔵 `cli/work/src/id_scheme.rs:99` — Ad-hoc `-`-separated ID parsers outside the one grammar (Code Quality)
- 🔵 `cli/work/src/id_scheme.rs:130` — `DRAFT_SUFFIX_LENGTH` duplicates `draft_id::SUFFIX_LENGTH` (Code Quality)
- 🔵 `cli/work/src/id_scheme.rs:330` — `is_tracker_owned` branching repeated across eight methods (Code Quality)
- 🔵 `cli/work-cli/src/create.rs:440` — `allocation_key` called only for its error side effect (Code Quality)
- 🔵 `cli/work/src/id_pattern.rs:203` — A second brace tokeniser beside `segments_of` (Code Quality)
- 🔵 `cli/work-adapters/src/config.rs:138` — Key setting found by re-reading config; typed as `String` (Code Quality)
- 🔵 `cli/visualiser/server/src/compose.rs:198` — Visualiser boot now depends on the full work-item grammar (Compatibility)
- 🔵 `cli/corpus-cli/src/main.rs:102` — Live linkage extraction ignores the configured scheme (Architecture)
- 🔵 `cli/work/src/legacy_ids.rs:1` — One frozen `LinkageIds` adapter serves migrations and live linkage (Architecture)
- 🔵 `cli/migrate-cli/src/work_item_identity.rs:18` — A second scheme-construction path beside `work_adapters::config` (Architecture)
- 🔵 `cli/work-adapters/src/retirement_records.rs:94` — Retirement record IDs are not checked against the ID grammar before reaching frontmatter (Security)
- 🔵 `cli/work/src/id_pattern.rs:940` — 100ms wall-clock assertions can flake under parallel runs (Test Coverage)
- 🔵 `tests/unit/tasks/test_create_work_item_id_evals.py:1` — Eval guard reads a recorded benchmark over a hard-coded ID range (Test Coverage)
- 🔵 `cli/work-cli/src/id/check.rs:66` — A key differing only in case is reported as `exact` (Usability)
- 🔵 `cli/work-cli/src/cli.rs:203` — `work id check` help text is hard to parse for its output contract (Usability)
- 🔵 `cli/work-cli/src/id/canonicalise.rs:15` — `work id` siblings report the same bad config with different codes (Usability)
- 🔵 `skills/work/create-work-item/SKILL.md:97` — `--id` mid-argument is silently treated as topic text (Usability)
- 🔵 `cli/work-cli/src/create.rs:628` — `Result`-typed parameter and an unreachable branch (Code Quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:1258` — Work-item identity reading inlined into `build_entry` (Code Quality)
- 🔵 `cli/work/src/id_pattern.rs:1022` — Test names and messages refer to the removed regex and numbered rules (Code Quality)
- 🔵 `cli/work/src/id_pattern.rs:664` — `IdParts::of` and `from_reading` duplicate construction (Code Quality)
- 🔵 `cli/work/src/requested_id.rs:215` — `+1`/`-1` arithmetic around `slug_start` (Code Quality)
- 🔵 `cli/work-cli/src/id/check.rs:53` — `Proposal` lacks an accessor for its ID (Code Quality)
- 🔵 `cli/work-cli/src/config.rs:44` — `announced_scheme_with_key` undersells what it validates (Code Quality)
- 🔵 `cli/work-cli/src/create.rs:1799` — Race test's power depends on a 300ms sleep (Test Coverage)
- 🔵 `cli/migrate-adapters/tests/common/mod.rs:32` — Test composition duplicates the production `WorkItemIdentity` adapter (Architecture)
- 🔵 `cli/migrate/src/ports.rs:223` — `MigrationContext` forwards `WorkItemIdentity` method by method (Architecture)
- 🔵 `tasks/lint/corpus_identity.py:18` — Corpus-identity guard is a name denylist; pair it with a structural rule (Architecture)
- 🔵 `cli/visualiser/server/src/compose.rs:13` — Visualiser links all of `work-adapters` for one config reader (Architecture)
- 🔵 `docs-site/src/content/docs/skills/work-items.md:46` — User docs give no example or shape guidance for `--id` (Usability)

## Inline Comments

### `cli/work/src/id_pattern.rs:414-437` — Patterns accepted since 1.20.0 now fail every work command and the visualiser, with no warning ahead of time
**Severity**: major | **Confidence**: medium | **Lens**: Compatibility

🟡 **Compatibility**

`IdPattern::parse` now refuses `work.id_pattern` values every stable release since 1.20.0 accepted: literal text outside `[A-Za-z0-9._-]` (`#`, `+`, `~`, `@`, space, `'`), the formerly documented `{{`/`}}` escapes, and a literal digit beside a token (`v2{number:03d}`). Through `read_scheme` that is a hard stop at the start of `create`, `create-batch`, `id next`, `resolve`, `list`, `promote`, `rekey` and `sync` (`work list` used to fall back to the default scheme), and the visualiser refuses to boot even for users who only browse plans and research.

**Impact**: a 1.23.0 user with such a pattern upgrades and finds the work-item skills and the visualiser broken. The only notice is the CHANGELOG `### Migrations` paragraph; no migration, `config summary` hook or discoverability-hook output flags it first.

**Suggestion**: detect the refused pattern ahead of time, e.g. a no-op migration or discoverability-hook check that runs `IdPattern::parse` on the configured pattern and reports the `E_WORK_ID_PATTERN_INVALID` reason with the re-key remedy, or a SessionStart warning from `config summary --format=hook`.

---

### `cli/work-cli/src/create.rs:625-638` — The dry-run re-implements the real create's --id check order
**Severity**: major | **Confidence**: medium | **Lens**: Code Quality

🟡 **Code Quality**

`refuse_requested_id_before_preview` copies by hand the order the real create uses (admit, then the pending-push marker check under `--push`, then the taken check). The real order lives separately in `Creation::claim_id` (`work-adapters/src/creation.rs`), and the two even use different marker checks (`refuse_unfinished_push` here, `check_pending_push` there). The doc comment is the only thing tying them together. The order matches today; nothing keeps it matching.

**Impact**: a check added, removed or reordered in `claim_id` silently breaks `--dry-run --id` parity, which is the behaviour the PR promises.

**Suggestion**: encode the sequence once, e.g. a `refuse_before_write(listing, marker_check)` in `work-adapters` that both paths call, with the marker check passed in (lock-free reader vs digest-aware). While there: admit the ID once in `preview_requested_id` and pass the `RequestedId` in, which removes the `Result<&IntegrationState, &String>` parameter and the unreachable `let Some(requested) = … else` branch.

---

### `skills/work/create-work-item/SKILL.md:122-130` — Confirmation depends on padding, not on whether the key differs, so a mistyped key goes through silently
**Severity**: major | **Confidence**: medium | **Lens**: Usability

🟡 **Usability**

Under `{key}-{number:04d}` with `work.key: PP`, typing `PO-400` gets a confirmation prompt (`corrected`). Typing `PO-0400` returns `other-key` and the skill uses it with no question, only mentioning the difference. The riskier part, a key that doesn't match the configuration, is the one that skips confirmation.

**Impact**: a one-letter key typo starts a new key sequence and writes an item the user didn't mean to create, while the harmless padding case is the one that asks.

**Suggestion**: treat `other-key` like `corrected`, proposing the ID and continuing only on confirmation. Or document why a key that differs from `work.key` is assumed intentional.

---

### `cli/work/src/id_scheme.rs:334-343` — The taken rule counts fewer prior-pattern IDs than re-key reads, so --id or allocation can claim a re-key target
**Severity**: minor | **Confidence**: medium | **Lens**: Correctness

🔵 **Correctness**

`read_held` (the `--id` taken rule and allocation) reads a value only through the current pattern, or as a bare legacy number under `work.key`. Re-key's `read_existing_id` also accepts `<key>-<digits>` and a single digit run. So after `work.id_pattern` changes from `{key}-{number:04d}` to `{key}_{number:04d}`, an existing `PP-0400` does not hold `PP_0400`; under `v{number:03d}` a legacy `0001` does not hold `v001`.

**Impact**: `work create --id PP_0400` (or allocation of `v001`) succeeds beside an item that re-key will later map onto the same ID. The re-key then refuses with `E_REKEY_ID_COLLISION` and the user untangles it by hand, which undercuts the guarantee that `--id` never takes an ID in use.

**Suggestion**: have the taken rule and allocation's sequence reading use the same fallback readings as `read_existing_id`, or document that the taken rule covers only IDs written under the current pattern.

---

### `cli/work/src/id_scheme.rs:366-369` — A pattern reading always wins over the legacy reading, misreading legacy files under key-after-number patterns
**Severity**: minor | **Confidence**: medium | **Lens**: Correctness

🔵 **Correctness**

`read_held_prefix` falls back to the legacy bare-number reading only when `parse_prefix` fails. Under `{number:04d}-{key}`, a legacy `0400-my-item.md` parses as key `my`, number 400, so the fallback never runs and the file counts as `MY-0400`, not `work.key`'s 0400.

**Impact**: allocation and the `--id` taken check both skip that file for `work.key`'s sequence, so `--id 0400-PP` is admitted and allocation can hand out 0400 although a legacy item holds it; a later re-key collides.

**Suggestion**: when a pattern reading yields a key other than `work.key`, also try the legacy reading and treat the file as holding both, or prefer the legacy reading when the text after the digits is plain slug. Add a key-after-number test.

---

### `cli/work/src/id_pattern.rs:538-550` — segments_without_key drops the wrong literal when {key} follows the number
**Severity**: minor | **Confidence**: high | **Lens**: Correctness

🔵 **Correctness**

`segments_without_key` drops `{key}` and the literal after it. That is right for `{key}-{number:04d}` but wrong for the valid `{number:04d}-{key}`, where the separator comes before the key: the key-less segments become `[Number, Literal("-")]`. So `work id check 400` refuses with no suggestion, while `400-` is accepted and corrected to `0400-PP`. The same reading feeds the `--project` remedy example.

**Suggestion**: drop the literal that joins the key to the number token: after the key when the key comes first, before it when the key comes last.

---

### `cli/work-cli/src/id/canonicalise.rs:43-49` — Canonicalise's E_PATTERN arms lost their only tests with the corpus-adapters suite
**Severity**: minor | **Confidence**: high | **Lens**: Test Coverage

🔵 **Test Coverage**

The deleted `cli/corpus-adapters/tests/work_item_id_canonicaliser.rs` was the only test of the `NoMatch` refusal (a digit run too long for u64, e.g. `99999999999999999999`) and the malformed-key refusal (`1PROJ`). Nothing in `work::id_scheme` or `cli_id_canonicalise.rs` now reaches `CanonicaliseError::NoMatch`, and the `E_PATTERN:` arms of `scheme_failure` are untested; only `E_PATTERN_BAD_FORMAT_SPEC` has a CLI test.

**Impact**: a refactor that remaps these codes, or makes overflow panic or wrap, would pass, though `work id canonicalise` is a scripted contract the skills depend on.

**Suggestion**: add `canonical(&numeric, "99999999999999999999") == Err(CanonicaliseError::NoMatch)` to the unit table, and two CLI cases asserting exact stderr for an overflowing input and for a hostile-literal pattern such as `x/{number:04d}`.

---

### `cli/work-cli/src/exit_codes.rs:176-179` — E_ID_* refusals exit 1, the internal-error band, while other collision refusals exit 4
**Severity**: minor | **Confidence**: high | **Lens**: Usability

🔵 **Usability**

The doc concedes `E_ID_*` exits `1` although that band reads "an internal error" and other collision refusals exit `4`. The module already lists an `id-taken` outcome that exits `4`, so "ID taken" now maps to two exit codes depending on the verb.

**Impact**: a script calling `work create --id` or `work id check` can't tell a bad or taken ID (a caller-fixable error) from a genuine failure without parsing stderr.

**Suggestion**: exit `4` for `E_ID_TAKEN` at least, matching the collision band. If `1` must stay for consistency with `E_PATTERN_*`, state that callers must branch on the stderr code.

---

### `cli/visualiser/server/src/indexer.rs:1358-1360` — work_item_id_from_entry now returns the identity key
**Severity**: minor | **Confidence**: high | **Lens**: Code Quality

🔵 **Code Quality**

`IndexEntry` now carries two concepts: `work_item_id` (the normalised ID that is shown) and `identity_key` (the folded join key, never served). `work_item_id_from_entry` returns `identity_key`, and the `work_item_by_id` index and its public lookup are keyed by identity key while still saying "id".

**Impact**: the names say the opposite of the code on exactly the distinction this PR introduces. A caller that passes an entry's `work_item_id` to `work_item_by_id` misses whenever folding or padding differs.

**Suggestion**: rename to `identity_key_of_entry` and `work_item_by_identity_key` (and the lookup method to match).

---

### `CHANGELOG.md:677-680` — CHANGELOG misstates the reach of the .MD walker change
**Severity**: minor | **Confidence**: high | **Lens**: Compatibility

🔵 **Compatibility**

The entry says `.MD` is now treated like `.md` for work items and that "`corpus frontmatter validate` and migrations walk it too". The `RealFs::walk_markdown` change in `cli/corpus-adapters/src/fs.rs` is not limited to work items: it applies to every doc type that `corpus frontmatter validate`, retirement reference rewriting and the migration `FileCorpusIndex` walk. Meanwhile `MigrationContext::list_md_files` (`cli/migrate-adapters/src/context.rs`) still matches only `ext == "md"`, so migrations do not rewrite `.MD` files.

**Impact**: a repo with a `.MD` plan whose frontmatter is invalid newly fails `corpus frontmatter validate` in CI with no entry explaining why, and readers are told migrations rewrite `.MD` files when they don't.

**Suggestion**: reword to "`corpus frontmatter validate` and re-key/retirement reference rewriting now read a `.MD` file of any doc type; migrations still rewrite only `.md`", or make the migration walker match.

---

## Per-Lens Results

### Architecture

**Summary**: The identity move is structurally sound. `corpus` declares small ports (`IdConvention`, `LinkageIds`) that `work` implements, so dependencies point inward and `corpus`/`corpus-adapters` lose their regex scanner. The visualiser and the work CLI share one scheme reader in `work_adapters::config`. Remaining concerns: one frozen `LegacyNumericIds` serves two clients that change at different rates; migration-side scheme construction is duplicated; `MigrationContext` forwards each identity method one at a time; and the lint guard catches only identifiers it knows by name.

**Strengths**:
- Proper dependency inversion: narrow ports with in-crate fakes, enforced by pup `allowed_only` and the crate-dependencies UPSTREAM/INJECTION rules.
- `work_adapters::config` breaks scheme reading into `read_scheme`/`require_key`/`resolve_scheme` with one `SchemeError` vocabulary; the visualiser's read-only subset is documented.
- One typed `IdPattern` grammar replaces the regex scanners; pattern validation fails at boot instead of at scan time.
- Migrations reach identity through `migrate::ports::WorkItemIdentity`, wired in `migrate-cli`; `migrate` stays pure.
- The tracker-key grammar moves into the zero-dependency `tracker` crate.
- The `lint:corpus-identity:check` guard is wired into both `cli:check` and `lint:check`.

**Comments**:
- `cli/work/src/legacy_ids.rs:1-3` (minor, medium) — One frozen `LinkageIds` adapter serves both migrations and live `corpus linkage extract`, and brings a `corpus-cli -> work` dependency. Give live linkage its own named adapter, or record that it stays on legacy rules.
- `cli/corpus-cli/src/main.rs:102-108` (minor, medium) — Live linkage hard-wires `LegacyNumericIds` while the visualiser and `work` use `WorkItemIdScheme`, so they can disagree on a `work-item:` reference in keyed repositories.
- `cli/migrate-cli/src/work_item_identity.rs:18-39` (minor, medium) — Builds `WorkItemIdScheme` by hand beside `work_adapters::config`; a bad key surfaces as a bare `KeyError` without `E_WORK_KEY_INVALID` framing. Add a shared `scheme_from_settings(pattern, key, key_setting)`.
- `cli/migrate-adapters/tests/common/mod.rs:32-64` (suggestion, high) — The test copy of the `WorkItemIdentity` adapter differs in error mapping, so tests exercise a composition that is not the shipped one.
- `cli/migrate/src/ports.rs:223-246` (suggestion, medium) — Expose the port once (`fn work_item_identity(&self) -> Result<&dyn WorkItemIdentity, MigrationError>`) instead of forwarding methods one by one.
- `tasks/lint/corpus_identity.py:18-40` (suggestion, medium) — The guard is a 21-name denylist; add a structural rule such as `corpus-adapters` must not depend on `regex`.
- `cli/visualiser/server/src/compose.rs:13-15` (suggestion, low) — The visualiser links all of `work-adapters` for one config reader; consider a small crate if `work-adapters` keeps growing.

**General Findings**:
- 🔵 (minor) Two `references_key` readings with different strictness; name the lenient one for migrations so live code doesn't use it.

### Correctness

**Summary**: The core holds up: the bounded walk, the width cap of 19, the number ≥ 1 gate, overflow handling with saturating and checked adds, and the taken check under the create lock are all sound, and the dry-run runs its checks in the real create's order. Three issues remain: correction and legacy reading go wrong for key-after-number patterns, the taken rule and allocation read fewer prior-pattern IDs than re-key, and the `identity_key` separator question raised in the description is real.

**Strengths**:
- `read_exact` requires the exact width and `in_exact_shape` requires a number of at least 1; `cap()` uses `checked_pow`, and width 19 fits in u64.
- Key and digit runs are tried longest-first with backtracking, so keys ending in the following literal's letter read back correctly.
- The real create checks for a taken ID on a listing read after taking the create lock.
- `allocate_above` uses `saturating_add`/`checked_add`, so a stray filename near u64::MAX gives an overflow error rather than a panic.
- The dry-run order matches the real create's.
- `holds` under `{tracker}` requires digits to match as written; `holder_of_under` deliberately leaves out `external_id` for re-key and retirement.

**Comments**:
- `cli/work/src/id_pattern.rs:538-550` (minor, high) — `segments_without_key` drops the wrong literal when `{key}` follows the number. See inline comment.
- `cli/work/src/id_scheme.rs:366-369` (minor, medium) — A pattern reading always wins over the legacy reading, misreading legacy files under key-after-number patterns. See inline comment.
- `cli/work/src/id_scheme.rs:334-343` (minor, medium) — The taken rule reads fewer prior-pattern IDs than re-key. See inline comment.
- `cli/work/src/id_scheme.rs:292-294` (suggestion, medium) — The `identity_key` fallback always joins with `-`, so under `{key}_{number:04d}` a `PP-42` reference joins as `PP-0042` and never meets `PP_0042`. When the pattern has `{key}`, render the fallback through the pattern; keep the `-` join only for keyless patterns.

**General Findings**: none.

### Code Quality

**Summary**: The new identity core is well designed, with domain-named types, typed refusals and `WorkItemLocation` simplifying `create.rs`. The main maintainability risks are knowledge kept in two places: the dry-run's check order, a second brace tokeniser and ad-hoc `-`-separated parsers in `id_scheme.rs`. Some names also blur the shown-ID / identity-key distinction.

**Strengths**:
- `RequestedIdRefusal` is a closed, typed enum; Display is built from `what()`/`remedy()`, and a compile-time `covered` exhaustiveness guard sits in the tests.
- `RequestedId::admit` reads as flat guard clauses in the documented order; `propose` reuses `admit`.
- `WorkItemLocation` removes repeated three-field plumbing.
- `LocalId` (`Allocate` vs `Requested`) avoids a flag argument.
- Test fixtures moved into `test_support` and are reused by `id/check.rs`.
- `SchemeError` replaces string errors with variants that keep the setting, the value and the cause.

**Comments**:
- `cli/work-cli/src/create.rs:625-638` (major, medium) — The dry-run re-implements the real create's `--id` check order. See inline comment.
- `cli/work-cli/src/create.rs:440-444` (minor, high) — `allocation_key` is called only for its error side effect, and passes the absolute `work_dir`. Extract `refuse_project_without_key_token`.
- `cli/work-cli/src/create.rs:628-632` (minor, medium) — A `Result`-typed parameter and an unreachable `let … else` branch; folded into the inline comment above.
- `cli/work-cli/src/id/check.rs:53-57` (suggestion, high) — Add `Proposal::id()` instead of matching all three variants in the adapter.
- `cli/work/src/id_pattern.rs:203-232` (minor, medium) — `tokens_of` is a second brace tokeniser beside `segments_of`; make `tracker` a segment and drop it.
- `cli/work/src/id_pattern.rs:664-678` (suggestion, high) — `IdParts::of` and `from_reading` duplicate construction; `read_correctable` repeats the byte bound.
- `cli/work/src/id_scheme.rs:99-128` (minor, medium) — Ad-hoc `-`-separated parsers outside the one grammar; name a `LegacyKeyedForm` or route through `IdPattern`.
- `cli/work/src/id_scheme.rs:130` (minor, high) — `DRAFT_SUFFIX_LENGTH` duplicates `draft_id::SUFFIX_LENGTH`.
- `cli/work/src/id_scheme.rs:330-385` (minor, medium) — `is_tracker_owned` branching is repeated across eight methods; split into `TrackerIds`/`PatternIds`.
- `cli/work/src/requested_id.rs:215-221` (suggestion, medium) — `+1`/`-1` arithmetic around `slug_start`; return a `HeldPrefix { parts, id_end, slug_start }`.
- `cli/work-adapters/src/config.rs:138-152` (minor, medium) — Re-reads config to learn which key setting supplied the value; use a `KeySource` enum, not `String`, and use the setting constants in messages.
- `cli/work-cli/src/config.rs:44` (suggestion, medium) — `announced_scheme_with_key` undersells what it validates; one concept has two names (`LEGACY_KEY_SETTING`/`LEGACY_PREFIX_KEY`).
- `cli/visualiser/server/src/indexer.rs:1358-1360` (minor, high) — `work_item_id_from_entry` returns the identity key. See inline comment.
- `cli/visualiser/server/src/indexer.rs:1258-1287` (suggestion, medium) — Extract a `work_item_identity` helper that trims once and owns the warning.
- `cli/work/src/id_pattern.rs:1022` (suggestion, medium) — Test names refer to the removed regex and to `rule five`; `(rule N)` suffixes are inconsistent across error variants.

**General Findings**:
- 🔵 (minor) `WorkItemIdScheme` offers seven overlapping ID readings; group them by purpose (display, join, occupancy, rendering).

### Test Coverage

**Summary**: Coverage is very strong and proportionate to the risk: dense, table-driven tests for the grammar, `requested_id` and `id_scheme`; every `work create --id` refusal with and without `--push`; ordering, the lock-held race and dry-run parity. Gaps: the canonicaliser's overflow and `E_PATTERN:` arms are no longer exercised, some tests rely on wall-clock bounds or a sleep, and one Python guard checks a recorded benchmark rather than the skill.

**Strengths**:
- `requested_id.rs` tests every refusal and their precedence; a const `covered()` match forces new variants into the tests.
- Refusal tests run under `push: false` and `push: true`, asserting `meta/` is unchanged and the tracker saw no calls.
- Ordering is pinned by dedicated tests (config errors first, re-key guard first, `--project` interactions).
- The race test holds the real create lock and writes the colliding file while holding it.
- Dry-run parity is checked both ways, and neither command waits on the create lock (via `recv_timeout`).
- Regex-era cases are ported onto the new grammar with exact expected text.
- Hostile-input edges are covered: escaping, Unicode digits, path traversal, u64 overflow, 255-byte bounds, unreadable holders.
- Visualiser tests assert behaviour, not regex strings; each Python guard has positive and negative tmp-tree tests.

**Comments**:
- `cli/work-cli/src/id/canonicalise.rs:43-49` (minor, high) — The `E_PATTERN` arms lost their only tests. See inline comment.
- `cli/work/src/id_pattern.rs:940-962` (minor, medium) — 100ms wall-clock assertions can flake; assert the length gate or use a loose bound such as 2s.
- `cli/work-cli/src/create.rs:1799-1826` (suggestion, low) — The race test relies on a 300ms sleep and may not detect a pre-lock check on a slow runner; signal on lock contention instead.
- `tests/unit/tasks/test_create_work_item_id_evals.py:1-14` (minor, medium) — Reads `benchmark.json` over a hard-coded `range(39, 73)`, so a `SKILL.md` regression without re-running evals stays green; derive the set from `evals.json`, or reword the PR claim.

**General Findings**:
- 🔵 (suggestion) Only `parent:` is tested for the new `work-item:` inbound linkage, not `related:`.

### Compatibility

**Summary**: The intentional breaks are handled carefully: the retired verbs point to their replacements, every caller in the repo moved, and migrations and live linkage use a frozen `LegacyNumericIds`. The main risk is the stricter `work.id_pattern` grammar: patterns accepted since 1.20.0 now hard-fail every `work` command and stop the visualiser at boot, with no warning ahead of time. The CHANGELOG also misdescribes the reach of the `.MD` walker change.

**Strengths**:
- Retired verbs are hidden, argument-tolerant stubs that exit 2 naming the replacement; `test_retired_id_verbs.py` stops the old names coming back.
- Migrations 0007/0009 and `corpus linkage extract` keep their historical output.
- `legacy_ids::references_key` keeps 0009 working on patterns the new grammar refuses, with a regression test.
- The `pulldown-cmark` bump is recorded in the golden-digest recipe history, so existing digests stay valid.
- The `{project}` token and `work.default_project_code` still resolve during their deprecation window.
- The CHANGELOG has explicit entries for each observable change.
- Key-case folding fits case-insensitive filesystems.

**Comments**:
- `cli/work/src/id_pattern.rs:414-437` (major, medium) — Previously accepted patterns now fail everywhere with no warning ahead of time. See inline comment.
- `cli/visualiser/server/src/compose.rs:198-203` (minor, medium) — Visualiser boot now depends on the full work-item grammar; degrade to no work-item clustering and surface the `SchemeError`.
- `CHANGELOG.md:677-680` (minor, high) — Misstates the reach of the `.MD` walker change. See inline comment.

**General Findings**:
- 🔵 (suggestion) Allocation jumps for repositories that skipped migration 0002; mention it in 0002's skip guidance.

### Usability

**Summary**: The `--id` surface is well designed: stable `E_ID_*` codes, what/why/remedy messages, a "Did you mean", and the holder's path. `work id check` gives the skill a small verdict to branch on. Main friction: confirmation depends on padding rather than on a differing key, and `--id` is recognised only in fixed positions. There are smaller inconsistencies in exit and error codes across the `work id` group.

**Strengths**:
- Every refusal renders as `CODE: what. remedy`; `E_ID_TAKEN` names the holder's repo-relative path and field.
- `work id check` writes and reserves nothing and returns the code `work create --id` would.
- Retired verbs accept any arguments, point to the replacement and exit 2.
- `--id` and `--dry-run` help text explain their effects.
- `E_PATTERN_OVERFLOW` names the item at the cap, with a concrete remedy.
- Every abort after drafting preserves the approved draft.
- Identity-run refusals now live in one shared skill section.

**Comments**:
- `skills/work/create-work-item/SKILL.md:122-130` (major, medium) — Confirmation depends on padding, not on a differing key. See inline comment.
- `cli/work-cli/src/id/check.rs:66` (minor, medium) — A key that differs only in case (`pp-0400` with `work.key: PP`) is reported as `exact`, producing a mixed-case corpus.
- `cli/work-cli/src/cli.rs:203-216` (minor, medium) — The `work id check` help text makes the field count of each verdict hard to parse; list the output contract in `long_about`.
- `cli/work-cli/src/exit_codes.rs:176-179` (minor, high) — `E_ID_*` exits 1 while other collision refusals exit 4. See inline comment.
- `cli/work-cli/src/id/canonicalise.rs:15-24` (minor, medium) — `work id canonicalise` reports a bad `work.key` as `E_PATTERN`, where its siblings use `E_WORK_KEY_INVALID`; it also omits the `Error: ` prefix.
- `skills/work/create-work-item/SKILL.md:97-101` (minor, medium) — `--id` in the middle of the argument is silently treated as topic text.
- `docs-site/src/content/docs/skills/work-items.md:46-48` (suggestion, medium) — No example, no shape rules, no `{tracker}` exclusion and no pointer to `work id check`.

**General Findings**:
- 🔵 (minor) The retired-verb message has no `E_*` prefix.

### Security

**Summary**: The new user-controlled input path is well guarded: an ID reaches a filename or frontmatter only through `read_exact` (exact-shape ASCII, 255-byte bound), and patterns are bounded so walking stays cheap. The new retirement and run-plan decoder checks are real defence in depth. No exploitable vulnerability; two minor hardening gaps, both requiring local write access to gitignored state.

**Strengths**:
- `RequestedId::admit` refuses separators, `..`, NUL, newlines, whitespace and non-ASCII digits, all tested.
- The taken check folds case, so `xy-0400` can't sit beside `XY-0400-x.md` on APFS.
- `escaped()` blocks terminal-escape and newline injection into messages relayed to the user or model.
- `/create-work-item` refuses `'`, single-quotes the ID after `--`, and from then on uses only the settled ID.
- `plan_retirement` and the run-plan decoder confine paths to the repository.
- A config-supplied pattern can't cause resource exhaustion.
- The visualiser no longer compiles a regex from config.
- `pulldown-cmark` is pinned to `=0.13.6` for RUSTSEC-2026-0433.

**Comments**:
- `cli/work-adapters/src/retirement_records.rs:94-105` (minor, medium) — `old`/`new` are checked as path components but not as IDs before reaching an unescaped frontmatter write on resume; a `"` or newline could inject YAML. Check them as `TrackerKey`/`DraftId`/scheme IDs, as `decode_entry` does.

**General Findings**:
- 🔵 (suggestion) `recovery_dir` decodes without path checks in retirement records and the pending-push marker; validate it, or derive it from `old--new`.

---
*Review generated by /accelerator:review-pr*
