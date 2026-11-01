---
type: "pr-review"
id: "176-review-1"
title: "[0281] Ask questions of and write reports from a research set"
date: "2026-11-01T21:35:35+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:176"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "usability", "compatibility", "standards", "documentation"]
review_number: 1
pr_number: 176
tags: []
last_updated: "2026-11-01T21:35:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #176 - [0281] Ask questions of and write reports from a research set

**Verdict:** COMMENT

The production surface is well built. Gate order, count derivation, freshness and report-slug allocation are pure domain code in `research::topic`, reads happen in `research-adapters`, and `research-cli` only renders. The lazy-closure gates and the shared `is_finding_entry` rule are tested at the unit, adapter and binary level. No lens raised a critical finding. The maintainability debt is in the new Python eval harness, which duplicates the turn-record replay, the parsing helpers, the criteria registry and the rule path. The one logic defect is the `conducted` refusal's `held_back` grouping, which can print a wrong diagnosis while still refusing correctly.

### Cross-Cutting Themes

- **`report_count` is written but never checked** (flagged by: architecture, code-quality) — `Freshness::assess` compares only round and finding counts, and `ManifestCount` has no report variant. A crash between the report write and the manifest bump, or a hand-deleted report, leaves `report_count` wrong with nothing to catch it. Either add a non-blocking mismatch to `counts` or document the field as informational and point consumers (0284) at `derived.report_count`.
- **Set layout and finding rules re-declared outside their owner** (flagged by: architecture, code-quality) — `reports`/`findings` path literals are spread across `slug.rs`, the adapter and `Documents::of`. `CONSULTABLE_STATUSES` repeats the manifest's `status_vocab` as strings. The eval's preflight and verdicts re-implement a looser finding filter than `is_finding_entry`.
- **`list_reports` and `CountedReports` disagree on what a report is** (flagged by: architecture, code-quality) — allocation treats every entry under `reports/` as taken, while counting includes only visible Markdown. That split is correct, but the `list_reports` name hides it. Rename it to something like `taken_report_names` so a later refactor does not unify the two.
- **Partial reflows break the 80-column rule** (flagged by: standards, documentation) — `SKILL.md:150`, `SKILL.md:579`, `consultable_command.rs:64-65` (inside `json!`, so rustfmt misses it) and an orphaned `skills` in `corpus.md:51`.
- **Permission-after-a-reply guidance** (flagged by: documentation, usability) — the CHANGELOG sentence is hard to parse. The docs recommend `Bash(accelerator corpus *)`, which is broader than the skill's own grant, and give no settings snippet. The PR description says the eval pre-grants the broad rules, but it passes the skill's narrower declared grant.

### Tradeoff Analysis

- **Compatibility vs schema strictness**: older binaries reject `kind: report` with `UNKNOWN-KIND`, so a shared corpus has to upgrade in lock-step. The upgrade note covers this release. A follow-up that downgrades an unknown `kind` of a known `type` to a warning would stop every future kind from forcing the same lock-step upgrade, at the cost of letting real typos through. Recommend a separate work item rather than changing it here.
- **Prose-phrase tests vs behavioural evals**: about 650 new lines in `test_research_structure.py` pin exact SKILL.md sentences. That catches deletions in CI, which the out-of-CI eval cannot, but it couples the tests to wording. Where an eval criterion already judges a contract, prefer the shared `formats.py` templates.

### Strengths

- ✅ `Consultation::judge` takes its readers as `FnOnce` closures. Tests show that a refused gate never reads the next gate's input and that a failed read past an open gate propagates. One binary test points `--profiles-dir` at a directory that does not exist, to prove freshness short-circuits.
- ✅ `is_finding_entry` is now shared by planning and counting, with a regression test asserting the two together, which closes the gap left by the old `is_markdown` check.
- ✅ The SKILL.md prose that counted files by hand in `conduct`, `synthesise` and `finalise` now defers to `research topic counts`, so prompt and binary share one source of truth.
- ✅ `StoredCount` and `Mismatch` model stored-count states as types. The mismatch wording names the document, field, stored value and derived value, and matches the docs table exactly.
- ✅ `ExtraVocab` closes a vocabulary as schema data, with `BAD-EXTRA-VALUE` shaped like `BAD-STATUS`. The rule that a written report is never `unanswered` sits in the schema, not only in prose.
- ✅ `ReportSlug` boundary cases are pinned (exactly 60 characters, hyphen as the 61st, no hyphen, non-ASCII input, case-insensitive collision, suffix pushing past 60). Allocation is safe on case-insensitive APFS.
- ✅ Every `consultable` refusal ends with a recovery line that depends on the set's status. Every new subcommand has a Code/Cause/Recovery exit table in the docs.
- ✅ Changes are additive for consumers: `report_count` is optional and outside freshness, `extra_vocabs` defaults to `{}`, the JSON outputs promise only to add fields, and the public-API fixtures are regenerated.
- ✅ The command allow-list tests cover `&&`, `;`, pipes, redirects, `$(...)`, backticks and a comment-hiding `#;`, plus a positive single-quoted case. `test:integration:eval-fixtures` pre-flights the reference corpus without a model in the test roll-up.
- ✅ The `headless` and `documents` helpers are pulled out of `citing_once` and shared. The eval tells errored sessions apart from failed ones and supports `--rerun-errored`.

### General Findings

- 🔵 **Test Coverage**: the SKILL.md write paths (writing the report, bumping `report_count`, in-place rewrite, appending only the next round) are exercised only by the out-of-CI eval, whose last full pass predates the six post-validation commits, and by the unticked manual smoke test. Tick the smoke test or re-run `--family R1,E2,P1` before merge.
- 🔵 **Usability**: `consultable` reports a mistyped `--profiles-dir` only once a set passes the status and freshness gates. Check that it names an existing directory when arguments are parsed.
- 🔵 **Documentation**: the `metadata derive` row in the noun-group table at `corpus.md:18` still lists only datetime, timestamp and repository/revision, with no author.
- 🔵 **Code Quality**: `preflight._output`, `_research_topic`, `_merged_sources` and `projects._override` call `repo_root()` instead of taking the plugin root as a parameter like the rest of the harness.
- 🔵 **Code Quality**: a turn's exit status is a bare string (`"timeout"`, `NOT_RUN`, `"0"`), compared by literal across modules. Add a `TIMEOUT` constant or a small `Literal` type.
- 🔵 **Architecture**: the reason `report-path` treats every entry as taken while `counts` only counts visible Markdown is not written down where the two meet.
- 🔵 **Documentation**: `consultable`'s read-scope description leaves out `reports/`, which `read_counts` lists past the status gate.
- 🔵 **Usability**: the `freshness` and `conducted` refusals do not end with "then {verb} {slug} {question} again" the way the proposal confirmation does.
- 🔵 **Standards**: the `outside_vocab` doc comment at `mod.rs:537` restates its one-line body. It sits outside the diff hunks, so it moved here from inline.

### Additional Findings

- 🔵 `tasks/eval/ask_and_report/verdicts.py:104` — `_one_of`/`_only` copied between verdicts.py and scenarios.py (code-quality)
- 🔵 `CHANGELOG.md:437` — the permission caveat sentence is hard to parse; same construction in the `report` entry (documentation)
- 🔵 `meta/prs/176-description.md:124` — the PR description misstates what the eval grants (documentation)
- 🔵 `cli/research-cli/src/consultable_command.rs:64` — lines inside `json!` exceed 80 columns (standards)
- 🔵 `skills/research/research-topic/SKILL.md:579` — an unreflowed prose line exceeds 80 columns (standards)
- 🔵 `skills/research/research-topic/SKILL.md:988` — `reject` and `decline` are near-synonyms with different outcomes (usability)
- 🔵 `tasks/eval/__init__.py:63` — `eval:citing-once --check-only` now demands the dev-launcher marker; ask-and-report's does not (usability)
- 🔵 `cli/research/src/topic/consultation.rs:10` — set lifecycle statuses re-declared as strings outside the schema (architecture)
- 🔵 `cli/research-cli/tests/topic_consultable.rs:206` — the plan-read failure test would also pass on a panic (test-coverage)
- 🔵 `cli/research/src/topic/layout/slug.rs:81` — the report slug can shrink to one leading word when the second word is very long (correctness)
- 🔵 `cli/research/src/topic/layout/slug.rs:55` — directory layout knowledge is split across slug, adapter and consultation (architecture)
- 🔵 `cli/research/src/topic/layout/slug.rs:12` — non-ASCII questions all slug to `report`, `report-2`, … (compatibility)
- 🔵 `tasks/eval/__init__.py:108` — eval-specific orchestration accumulates in the shared task module (architecture)
- 🔵 `skills/research/research-topic/SKILL.md:903` — a proposal can only be taken as printed (usability)
- 🔵 `tasks/eval/ask_and_report/preflight.py:124` — single-element `for x in [...]` clauses obscure `_template_problems` (code-quality)
- 🔵 `tasks/eval/ask_and_report/preflight.py:171` — preflight re-implements a looser finding-entry rule (code-quality)
- 🔵 `cli/research/src/topic/counts.rs:95` — `ManifestCount` names only two of the manifest's three counts (code-quality)
- 🔵 `cli/research-cli/src/counts_wording.rs:20` — the `Disagrees` wording hard-codes names `CountField` already provides (code-quality)
- 🔵 `cli/research-adapters/src/topic.rs:179` — `list_reports` lists every entry under `reports/`, not only reports (code-quality)
- 🔵 `cli/corpus/src/frontmatter_validation/mod.rs:335` — the extra-vocab and status checks treat a bare `key:` differently (code-quality)
- 🔵 `cli/research-cli/tests/topic_counts.rs:83` — `Project`/`copy_tree`/`tree` helpers copied into three new test files (test-coverage)
- 🔵 `docs-site/src/content/docs/corpus.md:49` — the Author addition makes a dense sentence and leaves an orphaned line (documentation, standards)
- 🔵 `docs-site/src/content/docs/research.md:592` — the allow-rule advice is broader than the skill's grant and gives no snippet (documentation)
- 🔵 `CHANGELOG.md:421` — the `report-path` entry reads as if the `-N` suffix replaces the cut (documentation)
- 🔵 `skills/research/research-topic/SKILL.md:1001` — the report id placeholder is named differently from the template's (standards)


## Inline Comments

### `tasks/eval/ask_and_report/runs.py:139-150` — TurnRecord reconstruction loop duplicated between run_conversation and load_records
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`load_records` re-implements the turn loop in `conversation.run_conversation`: both thread `before`/`after`/`earlier` through the turns, read `turn-N.jsonl`, compute `logged`, and build a ten-field `TurnRecord`. The two copies already differ in how `reply` is derived and in falling back to `before` when a snapshot is missing.

**Impact**: adding a `TurnRecord` field or changing how a turn's events or snapshots are read means editing both. If one is missed, live stop decisions (`breaks_a_turn_check`) and post-hoc judging (`judge`) see different records without any error.

**Suggestion**: extract one function that builds a `TurnRecord` from a session directory, turn number and prior state (e.g. `turns.record_from_disk(...)`). Call it from `run_conversation` after each turn and from `load_records`' replay.

---

### `cli/research/src/topic/consultation.rs:95-111` — held_back matches items by raw question; the planner groups by normalised question
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

`Refusal::conducted` matches outline items to unanswered pairs and skips with `pair.question == item.question`. `Planner::plan_item` (`plan.rs:215-250`) groups items by `NormalisedQuestion` and only pushes pairs and skips on an item's first sighting. Repeated questions across rounds are supported, which leaves two failure cases:

- Round 1 has `A? — profiles: web` (answered) and round 2 has `A? — profiles: web, openalex`. The round-2 item is incomplete, but its openalex pair is never pushed, so it lands in `held_back` with no skips.
- A repeat that differs only in case or spacing fails the raw comparison against the first item's outstanding pair, so the same focus area shows up again under `held_back` with no reasons.

**Impact**: the refusal prints `- {question}: ` with no reasons under "Held back by a profile the brief does not name…" and gives the wrong recovery (fix the brief). The set still refuses, so the gate outcome is unaffected.

**Suggestion**: compare by normalised question, or have the plan expose each incomplete item's outstanding and skipped pairs so consultation never re-derives the grouping. Add a test with a question repeated across rounds.

---

### `skills/research/research-topic/SKILL.md:982-994` — `accept` writes a new report at one prompt but is re-asked at the other
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

When no report is equivalent, `report` asks `Reply accept to write reports/{new}.md …`. When one is, the same action is `new`, and a bare `accept` "chooses none of its options" and the question is asked again. A user who has seen the first prompt will type `accept` at the second.

**Impact**: the same intent needs a different word depending on whether an equivalent report exists, which costs a wasted turn.

**Suggestion**: use one word for "write a new report" at both prompts, or treat a bare `accept` at the pick prompt as `new`, since `rewrite` already has to name a number. Update the matching `formats.py` templates with it.

---

### `cli/research/src/topic/counts.rs:152-170` — report_count is derived and stored but never checked
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture**

`CountsReading` carries `reports` and `stored.report_count`, but `Freshness::assess` compares only round count, finding count and rounds covered. The skill bumps `report_count` in a separate manifest edit after the report is written, so a crash between the two writes, or a hand-added or hand-deleted report, leaves the stored count wrong and nothing reports it.

🔵 **Code Quality**

`ManifestCount` has only `RoundCount` and `FindingCount`. Those two are the freshness-gating counts, not "the manifest's counts", so someone extending report handling could easily make `report_count` a freshness gate by mistake.

**Suggestion**: decide and encode it. Either add a non-blocking `ReportCount` mismatch to `counts` (reports are not evidence, so `consultable` stays unaffected), or rename `ManifestCount` to something like `FreshnessCount` and document that `report_count` is informational, so consumers such as 0284 read `derived.report_count`.

---

### `skills/research/research-topic/SKILL.md:149-151` — Preamble claims ask/report refusals name the set's status; line runs past 80 columns
**Severity**: minor | **Confidence**: high | **Lens**: documentation, standards

🔵 **Documentation**

The preamble says the refusals of `finalise`, `ask` and `report` "name the set's current status". That is true for `finalise` and the `status` gate. The `freshness` and `conducted` templates name mismatches and unanswered pairs instead. A model reading the preamble may add a status to lines the skill requires verbatim and the eval matches exactly.

🔵 **Standards**

Line 150 was edited without reflowing and is about 96 columns. The next line is left short.

**Suggestion**: e.g. "their refusals are owned by their own sections (finalise's names the set's current status), so the generic refusal does not apply to them", reflowed to 80 columns.

---

### `CHANGELOG.md:865-868` — Older binaries reject the new `report` kind with UNKNOWN-KIND
**Severity**: minor | **Confidence**: high | **Lens**: compatibility

🔵 **Compatibility**

The first committed `reports/*.md` (`kind: report`) makes whole-corpus `corpus frontmatter validate` fail on any binary from before this release. The upgrade note documents it, but any teammate or CI lane pinned to an older plugin breaks as soon as one person runs `report`. Every future kind will have the same problem.

**Suggestion**: keep the note. As a follow-up work item, consider having the validator report an unknown `kind` of a known `type` as a warning (or behind a lenient flag), so new kinds degrade gracefully on older binaries.

---

### `tasks/eval/ask_and_report/verdicts.py:649-681` — CRITERIA and _PARSERS are parallel registries kept in sync by hand
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`CRITERIA` (key → class) and `_PARSERS` (key → `Class.parse`) list the same keys twice. `sources` is only in one, which is why `parse_criteria` special-cases it, and `runs._KEYS` inverts `CRITERIA` a third time.

**Impact**: a criterion added to `CRITERIA` but not `_PARSERS` passes the `_only` key check, then fails with a bare `KeyError` at parse time.

**Suggestion**: derive `_PARSERS` from `CRITERIA`, or give `Sources.parse` the common signature by passing the whole `expect` mapping, so there is one registry.

---

### `tasks/eval/ask_and_report/conversation.py:28-35` — SKILL.md grant read by string-splitting; drift gives a bare IndexError
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

`_declared_grant` splits on `\n---\n` and `allowed-tools:\n` and keeps only lines indented exactly `  - `. A missing or reformatted key (inline list, other indent) gives an `IndexError` or an empty grant. The harness has three other ad-hoc frontmatter readers, and `tasks/shared/skill_parsing.py` already parses SKILL.md frontmatter.

**Impact**: a harmless SKILL.md reformat makes every session run with the wrong grant, or crash with an error that doesn't name the cause.

**Suggestion**: reuse `tasks/shared/skill_parsing`, or at least raise a `ValueError` naming SKILL.md and `allowed-tools`.

---

### `cli/research-adapters/src/topic.rs:314-326` — Bool, list, mapping and overflowing-string stored counts are untested
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`stored_count` renders a bool as `true`/`false`, a sequence as `"a list"` and a mapping as `"a mapping"`, and `whole_number` rejects a digit string that overflows `u32`. The adapter tests cover only a quoted word, a negative int, a float and absence, so none of these four branches runs.

**Impact**: these strings go straight into user-facing mismatch wording (`… records round_count 'a list', which is not a whole number`). Swapping the labels or letting overflow parse would go unnoticed.

**Suggestion**: add one parametrised adapter test over `true`, `[1]`, `{a: 1}` and `"99999999999"`, asserting `StoredCount::Unreadable(..)` with the exact rendered text.

---

### `tasks/eval/ask_and_report/checks.py:25` — Citing-once rule path constant defined four times
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`RULE = "skills/research/outputters/citing-once/SKILL.md"` is defined separately in `checks.py`, `conversation.py`, `preflight.py` and `citing_once/sessions.py`. `startup.py` imports the copy from `conversation.py`, which never uses it itself.

**Impact**: moving the rule means finding four literals. Missing one points the read-bound check, the probe or the preflight at a file that doesn't exist.

**Suggestion**: define it once beside `tasks/eval/documents/rule.py` and import it everywhere.

---

## Per-Lens Results

### Architecture

**Summary**: Clean functional-core/imperative-shell split: gates, counts, freshness and slug allocation are pure domain code in `research::topic`, reads are in `research-adapters`, and `research-cli` renders. The main concerns are scattered layout and status knowledge, and an unchecked `report_count`.

**Strengths**:
- Lazy-closure `Consultation::judge`, gate order unit-tested with no I/O.
- Count rule centralised in `layout::entry` and shared by planning, counts and consultable.
- Hand-counting in the SKILL.md prose replaced by `research topic counts`.
- `ExtraVocab` is data-driven on `SchemaRow`; report never `unanswered` enforced in schema.
- Author derivation goes through the existing `RepoFactsProbe` port.
- Shared `headless`/`documents` eval packages; errored distinguished from failed.

**Comments**:
- `cli/research/src/topic/counts.rs:152-170` (minor, high): `report_count` is derived and stored but never checked for freshness.
- `cli/research/src/topic/consultation.rs:10` (minor, medium): `CONSULTABLE_STATUSES` re-declares lifecycle states outside the schema's `status_vocab`. Introduce a `SetStatus` type, or add a test tying the two together.
- `cli/research/src/topic/layout/slug.rs:55-66` (suggestion, medium): `reports`/`findings` path literals are spread across slug, adapter and `Documents::of`. Give `layout` one owner.
- `tasks/eval/__init__.py:108-122` (suggestion, low): ~140 lines of ask-and-report orchestration in the shared task module. Move them into the eval's package.

**General Findings**:
- (minor) `report-path` treats every entry under `reports/` as taken, while counts include only visible Markdown. Correct, but the rationale is not written down; name the input `occupied_names`.

### Code Quality

**Summary**: The Rust surface is small, well named and testable without a filesystem. Most of the maintainability risk is in the Python eval harness, which duplicates the turn replay, parsing helpers, the criteria registry, the rule path and frontmatter parsing.

**Strengths**:
- `FnOnce` gate readers; `StoredCount`/`Mismatch` as domain types; wording separated in `counts_wording.rs`.
- `is_finding_entry` replaces ad-hoc `is_markdown`.
- `check_status` refactored to guard clauses sharing `outside_vocab`.
- Strict validation in `mutations.py` and `scenarios.py`.
- Headless/transcript/documents extraction removes duplication.

**Comments**:
- `tasks/eval/ask_and_report/runs.py:139-150` (major, high): TurnRecord reconstruction loop duplicated between `run_conversation` and `load_records`.
- `tasks/eval/ask_and_report/verdicts.py:104-126` (minor, high): `_one_of`/`_only` copied between verdicts.py and scenarios.py; `_typed` re-implemented.
- `tasks/eval/ask_and_report/verdicts.py:649-681` (minor, high): `CRITERIA` and `_PARSERS` are parallel registries.
- `tasks/eval/ask_and_report/checks.py:25` (minor, high): the RULE path constant is defined four times.
- `tasks/eval/ask_and_report/conversation.py:28-35` (minor, medium): `_declared_grant` string-splits SKILL.md frontmatter.
- `tasks/eval/ask_and_report/preflight.py:124-130` (suggestion, medium): single-element `for x in [...]` comprehension clauses.
- `tasks/eval/ask_and_report/preflight.py:171-172` (suggestion, medium): preflight's finding filter is looser than `is_finding_entry`.
- `cli/research/src/topic/counts.rs:95-98` (suggestion, medium): `ManifestCount` names only two of three manifest counts.
- `cli/research-cli/src/counts_wording.rs:20-35` (suggestion, medium): `Disagrees` arms hard-code document and key names.
- `cli/research-adapters/src/topic.rs:179-184` (suggestion, medium): `list_reports` returns every entry, not reports.
- `cli/corpus/src/frontmatter_validation/mod.rs:335-346` (suggestion, low): extra-vocab and status checks treat a bare `key:` differently.

**General Findings**:
- (minor) Several harness functions call `repo_root()` instead of taking the plugin root as a parameter.
- (suggestion) Turn exit status is an untyped string shared across modules.

### Test Coverage

**Summary**: Strong three-level coverage of the new CLI surface plus thorough harness unit tests and a model-free corpus pre-flight in the roll-up. Gaps: untested stored-count branches, one weak failure assertion, and SKILL.md write paths only covered outside CI.

**Strengths**:
- Gate short-circuit proven at unit and binary level.
- Precise `ReportSlug` boundary tests.
- Regression test for `is_markdown` → `is_finding_entry`.
- Every `check_extra_vocabs` branch tested, plus a golden stderr.
- Adversarial allow-list tests with a positive counter-case.
- Writes-nothing byte snapshot for every read-only command.
- `test:integration:eval-fixtures` with planted-problem tests.

**Comments**:
- `cli/research-adapters/src/topic.rs:314-326` (minor, high): bool/list/mapping/overflow stored counts untested.
- `cli/research-cli/tests/topic_consultable.rs:206-216` (minor, medium): plan-read failure test would also pass on a panic. Assert exit code 1 and the stderr text.
- `cli/research-cli/tests/topic_counts.rs:83-112` (suggestion, medium): `Project`/`copy_tree`/`tree` helpers copied into three new test files.

**General Findings**:
- (minor) SKILL.md write paths are verified only by the out-of-CI eval and an unticked smoke test.
- (suggestion) Much of the structure-test growth pins exact prose phrases.

### Correctness

**Summary**: Gate order and lazy reads are correct, the count rule is shared, stored-count parsing covers every scalar shape, and suffixing is case-safe. Two edge cases affect output, not the gate outcome.

**Strengths**:
- `FnOnce` readers; failed reads propagate.
- `is_finding_entry` shared by planner and counter.
- `stored_count` and `whole_number` reject all non-whole shapes.
- `free_report_path` checks every entry, case-insensitively.
- ASCII-only slicing cannot panic; 61-byte window is correct.
- No double violation between `BAD-EXTRA-VALUE` and `EMPTY-PLACEHOLDER`.

**Comments**:
- `cli/research/src/topic/consultation.rs:95-111` (minor, medium): `held_back` matches by raw question while the planner groups by normalised question and only records on first sighting. Verified against `plan.rs:215-250`.
- `cli/research/src/topic/layout/slug.rs:81-90` (suggestion, medium): a slug can shrink to one leading word (`a`, `how`) when the second word is very long.

**General Findings**: none.

### Usability

**Summary**: Refusals are fixed sentences with recovery paths, the JSON outputs are documented, and denial handling is designed for the user. The friction is in the reply vocabulary and in an asymmetry between the two evals' start-up gates.

**Strengths**:
- Recovery lines depend on status; the `conducted` refusal is ordered so the fixes can be worked through in sequence.
- Mismatch messages name document, field, stored and derived values.
- Exit-code tables with Recovery column; `--profiles-dir` reused from `outstanding`.
- Permission-denial recovery names the allow rules and next step.
- Fixed success confirmations that end with the path back to the question.
- Clear eval start-up errors; marker reminder in `finally`.

**Comments**:
- `skills/research/research-topic/SKILL.md:982-994` (minor, high): `accept` means "write new" at one prompt but is re-asked at the other.
- `skills/research/research-topic/SKILL.md:988` (minor, medium): `reject` and `decline` are near-synonyms with different outcomes.
- `tasks/eval/__init__.py:63` (minor, medium): `eval:citing-once --check-only` now requires the dev-launcher marker.
- `skills/research/research-topic/SKILL.md:903-907` (suggestion, low): a proposal can only be taken as printed.

**General Findings**:
- (minor) A wrong `--profiles-dir` goes unreported until the third gate.
- (suggestion) The stale/unconducted refusals omit the "then ask again" step.

### Compatibility

**Summary**: Almost entirely additive, with carefully handled contracts. The one real break is forward compatibility of the `report` kind on older binaries, which the CHANGELOG documents.

**Strengths**:
- `report_count` optional, outside freshness; old manifests stay valid and fresh.
- CLI finding rule matches the prose rule it replaces.
- `extra_vocabs` additive with `{}` default.
- JSON outputs promise only to add fields; exit 0 for any verdict.
- Case-insensitive collision check suits APFS.
- Scoped `Read` grant probed on 2.1.144 and 2.1.316.
- Public-API fixtures capture every contract change.

**Comments**:
- `CHANGELOG.md:865-868` (minor, high): older binaries reject `kind: report` with `UNKNOWN-KIND`.
- `cli/research/src/topic/layout/slug.rs:12` (suggestion, low): non-ASCII questions all slug to `report`, `report-2`, …; matches existing `QuestionSlug`, so not a regression.

**General Findings**: none.

### Standards

**Summary**: Follows repo conventions closely (module naming, error-code prefix, violation shape, regenerated fixtures, catalogue and dump golden, CHANGELOG headings, eval task declaration). The deviations are 80-column breaks and one restating doc comment.

**Strengths**:
- `*_command`/`*_wording` module naming.
- `E_TOPIC_RESEARCH_QUESTION` prefix; Code/Cause/Recovery tables.
- `BAD-EXTRA-VALUE` mirrors `BAD-STATUS`.
- Subcommand checklist surfaces covered.
- Report template matches its siblings.
- CHANGELOG headings and upgrade note.
- `eval:ask-and-report` declared like `eval:citing-once`.

**Comments**:
- `cli/research-cli/src/consultable_command.rs:64-65` (minor, high): lines inside `json!` exceed 80 columns.
- `skills/research/research-topic/SKILL.md:150` (minor, high): reflowed prose line runs past 80 columns. Merged inline with documentation.
- `skills/research/research-topic/SKILL.md:579` (minor, high): unreflowed prose line in conduct step 3.
- `docs-site/src/content/docs/corpus.md:51` (suggestion, high): orphaned word left by a partial reflow.
- `cli/corpus/src/frontmatter_validation/mod.rs:537` (suggestion, medium): doc comment restates `outside_vocab`. Outside the diff hunks, so moved to general findings.
- `skills/research/research-topic/SKILL.md:1001` (suggestion, low): `{file stem}` vs the template's `{report-stem}`.

**General Findings**: none.

### Documentation

**Summary**: Thorough and verified against the code. The gaps are a stale corpus.md table row, one inaccurate preamble sentence, a misstatement in the PR description, and some ambiguous CHANGELOG sentences.

**Strengths**:
- research.md documents each subcommand completely; mismatch sentences match `counts_wording.rs`.
- Slug rule described exactly as implemented.
- Upgrade notes cover both actions a consumer must take.
- tasks/README eval section restructured; ask-and-report subsection accurate.
- Permission limitation documented where users hit it.
- The few new code comments explain why.

**Comments**:
- `skills/research/research-topic/SKILL.md:149-151` (minor, high): preamble says ask/report refusals name the set's status; only the status gate's does.
- `CHANGELOG.md:437-440` (minor, medium): permission caveat sentence hard to parse.
- `CHANGELOG.md:421-422` (suggestion, medium): `report-path` entry reads as if the suffix replaces the cut.
- `docs-site/src/content/docs/corpus.md:49-52` (suggestion, medium): dense Author sentence; doesn't say the line can be absent.
- `meta/prs/176-description.md:124-125` (minor, high): PR description misstates what the eval grants.
- `docs-site/src/content/docs/research.md:592-598` (suggestion, low): allow-rule advice broader than the skill's grant; no snippet.

**General Findings**:
- (minor) corpus.md summary table not updated for the Author line.
- (suggestion) consultable's read-scope description omits `reports/`.

---
*Review generated by /accelerator:review-pr*
