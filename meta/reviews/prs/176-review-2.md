---
type: "pr-review"
id: "176-review-2"
title: "[0281] Ask questions of and write reports from a research set"
date: "2026-11-22T13:37:27+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:176"
relates_to: ["pr-review:176-review-1"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "usability", "compatibility", "documentation", "standards"]
review_number: 2
pr_number: 176
tags: []
last_updated: "2026-11-22T13:37:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #176 - [0281] Ask questions of and write reports from a research set

**Verdict:** COMMENT

The second pass closed review 1's findings across all eight lenses: `SetStatus`, a per-item `HoldBack`, `ReportCountDrift` and `TurnHistory` now carry intent in the types, and the Rust slug, gate-order and freshness logic held up at every boundary the correctness lens traced. Two major findings remain. The skill's re-verdict rule, read literally, aborts a confirmed proposal after a partial `report`, and `eval:ask-and-report` exits green when its filters select nothing. The rest are minor: held-back reporting across repeated askings, plugin-root handling, wording that has drifted out of the `*_wording` modules, and documentation gaps.

### Cross-Cutting Themes

- **Held-back reporting across repeated askings and index exhaustion** (flagged by: correctness, test coverage, usability, code quality) — a repeated question whose index has run out is likely listed twice, while a repeated skip-only question reports only its first asking's reasons. The index-exhausted reason is worded outside `plan_wording`, pinned by no test, and the skill pairs it with a recovery line that cannot fix it.
- **`ACCELERATOR_PLUGIN_ROOT` handling** (flagged by: architecture, code quality, documentation, test coverage, standards, usability) — the variable is read directly rather than via `config_adapters::plugin_root_from_env()` and spelled in four places. `outstanding`'s failure without it is neither documented nor binary-tested. Its sibling missing-profiles error has no code, and the `E_RESEARCH_` prefix departs from `E_TOPIC_RESEARCH_*`.
- **Level-note paths assembled outside `Documents`** (flagged by: architecture, code quality) — `consultable` strips the `findings/` prefix `Documents::of` just added and the adapter re-adds it, so the set layout crosses the domain/adapter boundary as a string contract.
- **Optimistic-concurrency re-check** (flagged by: correctness, architecture) — the rule compares against "the first verdict", which the session's own report write invalidates, and its outline-round half lives in prose rather than in `consultable`'s output.
- **Why `consultable` needs an allow rule** (flagged by: documentation, usability) — the permissions paragraph lists four rules but justifies three. The PR description also says "three rules" in one place and "four" in another.

### Tradeoff Analysis

- **Compatibility vs documentation on `outstanding --profiles-dir`**: compatibility wants a hidden, ignored flag for one release or a CHANGELOG line, because pre-release users who scripted the documented form now get a clap usage error. Documentation judged no Breaking entry necessary because `outstanding` itself is still Unreleased. A one-line Changed entry costs nothing and covers pre-release users. The shim is optional.
- **Moving the outline-round check into the CLI vs keeping `consultable` narrow**: architecture suggests `consultable` print the outline's highest round so the whole concurrency token comes from one command. That widens `consultable`'s JSON contract, which is add-only, so the cost is low. It would also make the correctness fix easier to state precisely.

### Strengths

- ✅ Every review-1 finding across the eight lenses is resolved, and the fixes are structural rather than patches.
- ✅ The planner now records each item's `HoldBack`, so consultation no longer re-derives the planner's grouping and the raw-vs-normalised question drift is gone.
- ✅ `SetStatus` is a closed domain type pinned to the schema's `status_vocab` by an adapter test, without making `research` depend on `corpus`.
- ✅ `FreshnessCount` vs `ReportCountDrift` makes it a type-level fact that `report_count` never gates freshness. Drift is tested at domain, freshness and binary levels.
- ✅ `cut_before_the_overflowing_word` is correct at every boundary (exactly 60, a hyphen at 61, word-cut at 41 vs 40, no hyphen), and the ASCII-only slug cannot split a character.
- ✅ `Consultation::judge` takes `FnOnce` readers, so a refused gate never reads the next gate's input.
- ✅ `TurnHistory` is the single builder of turn records for live and replayed judging, and tests compare the two by equality.
- ✅ `TurnExit` is a closed union, and `RULE`, `one_of`/`only` and the criteria registry are each defined once.
- ✅ Every refusal ends with a concrete `{verb} {slug} {question} again` retry line, and every write-guarding prompt lists its options and re-asks on an unclear reply.
- ✅ Both intentional breaks (a bare `status:` is now `BAD-STATUS`; older binaries reject `kind: report`) are in CHANGELOG Breaking and Upgrade notes. Other contract changes are additive.
- ✅ The documented command names, JSON shapes, mismatch sentences, slug rule and allow-rule snippet match the code. The four allow rules are identical in the CHANGELOG, `research.md` and `SKILL.md`.
- ✅ No added code comment references work items, ADRs or plan phases, and the public-api fixtures, catalogue count, dump golden and mise lane pins move together.

### General Findings

- 🔵 **Compatibility**: The finding-stem CHANGELOG note (Changed) could say that the old-stem `.levels/` tree stays on disk and can be deleted once the pair is re-researched. It could also fold the stem change into the "upgrade a shared corpus together" note, since teammates on binaries either side of the change compute different stems and redo each other's deep research. (Moved from inline: `CHANGELOG.md:95-101` is outside the diff hunks.)
- 🔵 **Correctness**: `is_finding_entry` accepts an upper-case `.MD`, but `level_notes_of` strips a case-sensitive `.md`, so a hand-renamed finding is counted while its level notes are never listed.
- 🔵 **Code Quality**: `manifest.md` and `synthesis.md` remain literals across `read_counts`, `read_set_status`, `CountField::document()` and `counts_wording`, while `findings`/`reports` got `layout` constants.
- 🔵 **Documentation**: The PR description says the CHANGELOG lists "three rules" in Notes for Reviewers but "four narrow allow rules" in the Skill section. The CHANGELOG, `research.md` and `SKILL.md` all name four.

### Additional Findings

- 🔵 `cli/research-cli/src/counts_wording.rs:55` — unreadable `report_count` notice text is untested (test coverage)
- 🔵 `cli/research-cli/src/main.rs:214` — `outstanding` without a plugin root has no binary test (test coverage)
- 🔵 `cli/research-cli/src/consultable_command.rs:46` — plan warnings escape the gate through a mutating closure (code quality)
- 🔵 `cli/research-cli/src/consultable_command.rs:38` — missing-profiles-directory error has no code or recovery (usability)
- 🔵 `cli/research-cli/src/cli.rs:113` — `outstanding --profiles-dir` removed with no shim or change note (compatibility)
- 🔵 `skills/research/research-topic/SKILL.md:775` — command-shape rule allows redirects and no-op commands, the cause of the 3 eval failures (usability)
- 🔵 `tasks/eval/ask_and_report/preflight.py:240` — "new report" predicate written twice (code quality)
- 🔵 `tasks/eval/ask_and_report/preflight.py:173` — sources matching duplicated with the `Sources` judge (code quality)
- 🔵 `tasks/eval/ask_and_report/verdicts.py:62` — two more hand-rolled frontmatter readers (code quality)
- 🔵 `cli/research/src/topic/layout/stem.rs:6` — `crate` import splits the `std` import group (code quality, standards)
- 🔵 `cli/corpus/src/frontmatter_validation/mod.rs:344` — 83-column doc comment (standards)
- 🔵 `cli/research-cli/tests/topic_counts.rs:133` — `json!` string literals over 80 columns (standards)
- 🔵 `skills/research/research-topic/SKILL.md:801` — partial reflows leave orphaned short lines (standards)
- 🔵 `tasks/README.md:693` — "the task refuses without it" contradicts the `--check-only` exemption (documentation)
- 🔵 `cli/research/src/topic/plan.rs:159` — `held_back` builds every `ItemStatus`; name misleads (code quality)
- 🔵 `cli/research-cli/src/topic_command.rs:1` — shared `Printed`/`resolve_set` live in the `outstanding`/`end-run` module (architecture)
- 🔵 `cli/research-adapters/src/topic.rs:270` — `level_notes_of` claims lineage order but sorts as strings (correctness)
- 🔵 `tasks/shared/skill_parsing.py:80` — inline `allowed-tools` edge cases untested; `[]` yields `['']` (test coverage)
- 🔵 `tasks/eval/ask_and_report/verdicts.py:219` — criterion parsers validate scenario JSON unevenly (code quality)
- 🔵 `tasks/eval/ask_and_report/orchestration.py:37` — three run modes encoded as two booleans (code quality)
- 🔵 `cli/research-adapters/src/topic.rs:632` — test placed above the test module's imports (code quality)
- 🔵 `cli/research-cli/src/installed_profiles.rs:20` — `E_RESEARCH_PLUGIN_ROOT` breaks the `E_TOPIC_RESEARCH_*` prefix (standards)
- 🔵 `cli/research/src/topic/layout/slug.rs:214` — test name too long for 80 columns and uses older wording (standards)
- 🔵 `CHANGELOG.md:666` — partial reflows in new entries (standards)
- 🔵 `tasks/README.md:768` — partial reflows in the `eval:ask-and-report` section (standards)


## Inline Comments

### `skills/research/research-topic/SKILL.md:789-794` — Re-verdict rule aborts the partial-report → confirm-proposal flow on the session's own report
**Severity**: major | **Confidence**: medium | **Lens**: correctness, architecture

🟡 **Correctness**

Before the first write after a reply, the rule takes the verdict again and stops if "its `reports` differ from the first verdict's". In the `partial` report flow, the verb writes a new report, records `report_count`, then proposes research. When the user replies `confirm`, the re-verdict's `reports` now include the report this session just wrote, so a literal reading says another session changed the set: stop, write nothing, rerun. Eval scenario R1's partial case (accept → confirm) expects the round to be appended and the set reopened. That it passes suggests the model reads past the contradiction, which is not something to rely on.

**Suggestion**: compare against the most recent verdict this session took, e.g. "its `reports` differ from the last verdict's other than by a report this verb wrote", or take the baseline again after the manifest edit.

🔵 **Architecture**

Two of the three compared values come from `consultable`. The third, "`outline.md`'s highest `## Round` heading", the model must find and compare itself. That half decides which round number a confirmed proposal appends, so a slip produces a duplicate or misnumbered round rather than a refusal. Having `consultable`'s open verdict also print the outline's highest round would put the whole concurrency token in one command and make the fix above easier to state.

---

### `tasks/eval/ask_and_report/orchestration.py:122-139` — The eval exits 0 when its filters or `--check-only` select no sessions
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

`plan_sessions` filters `--family`, `--verb` and `--scenario` by exact, case-sensitive match and never reports a name that matched nothing. `--check-only` without `--out`, or with a mistyped `--out`, judges a fresh empty temp directory. Either way `results` is empty, `any(result.status != "passed" ...)` is false, and the task exits 0 after printing `Planned sessions: 0`. A full run with `--family r1,E2` even passes the version and auth gates first.

**Impact**: a developer checking a fix, or re-judging a run with a misspelt path, sees a green exit when nothing was judged.

**Suggestion**: reject filter names that match no scenario (listing the valid ones), require `--out` for `--check-only` as `--rerun-errored` already does, and exit non-zero when no session is planned or judged.

---

### `skills/research/research-topic/SKILL.md:861` — One held-back recovery line for every held-back reason, wrong for some
**Severity**: minor | **Confidence**: high | **Lens**: usability

🔵 **Usability**

The `conducted` refusal always prints `Add the profile to the brief's source_profiles, or fix its name in the outline.` under the held-back group. A `held_back` item can carry three reasons: a profile not in the brief, `no '<p>-profile' skill is installed`, or `no finding index is left to allocate to it`. For the not-installed reason, editing the brief changes nothing. For index exhaustion no profile is involved, so neither half applies, and the closing `Run conduct {slug}` cannot fix it either. A user who follows the lines in order gets the same refusal back.

**Suggestion**: choose the recovery from the reasons present (install the profile skill, or merge focus areas or start a new set), or have `consultable` emit a `recovery` per `held_back` item so the wording lives beside the reason in the CLI.

---

### `cli/research/src/topic/plan.rs:168-175` — Repeated askings are held back inconsistently: skips deduped and truncated, exhaustion duplicated
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, test coverage

🔵 **Correctness**

Only a question's first all-skipped asking gets `HoldBack::Skipped(planning.skips)`, because `held_back_by_skips.insert` returns false for later askings. With `- [ ] A? — profiles: arxiv` then `- [ ] A? — profiles: crossref`, the refusal lists only the arxiv reason, so a user who fixes it and re-runs `conduct` can still be refused. Collect skips per normalised question across all askings and attach the union to the first held-back item.

🔵 **Test Coverage**

The opposite happens for exhaustion: every asking whose `planning.exhausted` is true is held back as `IndexesExhausted`. A repeat finds its pair already in `seen` and its index from `claimed` (`None`) while `outstanding` is non-empty, so it is exhausted too. `consultable` would then list the same question twice, although the warning is pushed once. `an_item_left_without_an_index_is_held_back_by_exhaustion` covers only a single asking. Add a test with `- [ ] M?` asked twice under exhausted levels and pin the intended result, and one with two askings naming different skipped profiles.

---

### `cli/research-cli/src/consultable_command.rs:172-181` — Index-exhausted reason is worded outside `plan_wording` and pinned by no test
**Severity**: minor | **Confidence**: high | **Lens**: code quality, test coverage

🔵 **Code Quality**

`hold_back_reasons` hard-codes "no finding index is left to allocate to it", while `plan_wording.rs:21-22` already words `Warning::IndexesExhausted`, and `Skipped` reasons go through `plan_wording::skip_reason`. One domain event now has two independently maintained sentences. Move `hold_back_reasons` into `plan_wording` and build both from one phrase.

🔵 **Test Coverage**

The string reaches the user through the `conducted` refusal, but the domain test checks only the enum and no binary test triggers exhaustion. Add a `topic_consultable.rs` case that plants a levels directory claiming index `4294967295` (or a `rendered` unit test) and asserts the `held_back` reason.

---

### `docs-site/src/content/docs/research.md:603-613` — Permissions paragraph never says why `consultable` needs an allow rule
**Severity**: minor | **Confidence**: high | **Lens**: documentation, usability

🔵 **Documentation** / **Usability**

The paragraph names the post-reply commands as metadata derive, validate and (for `report`) `research topic counts`, and says the rules "cover only the commands that run after a reply". The snippet then adds a fourth, `Bash(accelerator research topic consultable *)`, which the text never justifies: both verbs re-take the `consultable` verdict before their first write after a reply.

**Impact**: a reader trimming the list to what the prose justifies drops the `consultable` rule, then hits a prompt, or a headless denial, at the re-check.

**Suggestion**: add the clause, e.g. "…and both re-run `research topic consultable` before their first write after a reply". Separately, the PR description says "three rules" in Notes for Reviewers. It should say four.

---

### `docs-site/src/content/docs/research.md:783` — `outstanding` exit-code table omits the new `E_RESEARCH_PLUGIN_ROOT`
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

`outstanding` now calls `installed_profiles_dir()` first and exits 1 with `E_RESEARCH_PLUGIN_ROOT` when `ACCELERATOR_PLUGIN_ROOT` is unset, but its table and the "exits `1` with `E_TOPIC_RESEARCH_UNRESOLVED`" line still list only the older codes. The `consultable` table does list it. The binary test suite also has no `outstanding_without_a_plugin_root_fails_naming_it` to pin this path, while `consultable` has one (`cli/research-cli/src/main.rs:214`).

**Suggestion**: add `| E_RESEARCH_PLUGIN_ROOT | ACCELERATOR_PLUGIN_ROOT is not set | run accelerator through its launcher, which sets it |`, matching the `consultable` row, and mirror the `consultable` binary test.

---

### `cli/research-cli/src/consultable_command.rs:86-101` — Level-note paths are assembled outside the gate-owned `Documents` by string round trip
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code quality

🔵 **Architecture**

`Documents` is documented as the paths a consultation reads, built only by `Consultation::judge` "so a set is opened only through its gates". The CLI then adds level notes after the verdict, so that guarantee now holds for only part of the read scope, and a future command reusing `Documents` gets findings without the level notes the skill also reads.

🔵 **Code Quality**

To do it, the command strips the `findings/` prefix `Documents::of` just added (`finding.rsplit('/')`), and `level_notes_of` re-adds `FINDINGS` and strips `.md`. The set-relative path format becomes a string contract across `consultation.rs`, the adapter and the CLI. Nesting findings later breaks the `rsplit` silently.

**Suggestion**: give `judge` a lazy level-note reader, or have `Documents` carry finding names (or a `Stem`/`NoteRef`) so the adapter builds typed paths and the CLI only renders.

---

### `cli/research-cli/src/main.rs:289-294` — `ACCELERATOR_PLUGIN_ROOT` read directly instead of via `config_adapters::plugin_root_from_env()`
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code quality

🔵 **Architecture** / **Code Quality**

`installed_profiles_dir` calls `std::env::var_os("ACCELERATOR_PLUGIN_ROOT")` with its own literal. `installed_profiles.rs` keeps a separate `PLUGIN_ROOT` constant used only in the error message, and the test support spells it twice more. `research-cli` already depends on `config-adapters`, whose `plugin_root_from_env()` is documented as the one place the plugin root is read.

**Impact**: if one copy is renamed, the binary reads one variable and names another in its error. If the launcher contract gains a fallback, `research` silently diverges from the other sub-binaries.

**Suggestion**: have `installed_profiles::profiles_dir` take the `Option<PathBuf>` from `config_adapters::plugin_root_from_env()`, drop the local constant and the literal, and keep the pure core testable by passing the root explicitly.

---

### `cli/research-cli/src/counts_wording.rs:49-61` — `report_count_drift` hard-codes the document and key names `CountField` otherwise owns
**Severity**: minor | **Confidence**: high | **Lens**: code quality

🔵 **Code Quality**

`mismatch` takes every document and key name from `CountField::document()`/`key()`, which was review 1's fix for `Disagrees`. The new `report_count_drift` writes `manifest.md` and `report_count` as literals in both arms. That brings back the drift just removed. Renaming the field means editing literals no type points to.

**Suggestion**: give the report count a name source too, a `CountField::ReportCount` variant outside `FreshnessCount` or a `ReportCountDrift::field()`, and build the sentence like `mismatch` does. The `Unreadable` arm's text is also exercised by no test (`counts_wording.rs:55-59`), so a case with `report_count: "x\ny"` asserting the escaped notice would pin both.

---

## Per-Lens Results

### Architecture

**Summary**: The second pass fixes review 1's structural findings: set statuses are a `SetStatus` type pinned to `status_vocab`, `FINDINGS`/`REPORTS` are defined once, `report_count` drift is a non-gating `ReportCountDrift`, the plan exposes each item's `HoldBack`, and the eval's orchestration has its own package. The functional core / imperative shell split holds across `research`, `research-adapters` and `research-cli`. What remains is small: level-note read paths outside the gate-owned `Documents`, the plugin root read outside `config_adapters`, and two cohesion points.

**Strengths**:
- The plan records each item's `HoldBack`, so consultation no longer repeats the planner's grouping.
- `SetStatus` is closed and pinned to the schema by an adapter test without a `research` → `corpus` dependency.
- `ReportCountDrift` is distinct from the freshness `Mismatch`, so `report_count` cannot become a gate by accident.
- `TakenNames` vs `CountedReports` separates "what holds a name" from "what counts as a report".
- `DirectoryEntries` is shared between planning and counting, so they cannot disagree on what a finding is.
- `ExtraVocab` is schema data, so closing a new vocabulary is a data change.
- `conduct`, `synthesise` and `finalise` take counts from the CLI rather than prose.

**Comments**:
- `cli/research-cli/src/consultable_command.rs:86-101` (minor, medium) — Level notes widen the read scope outside the gate-owned `Documents`. *(merged inline)*
- `cli/research-cli/src/main.rs:289-293` (minor, high) — Plugin root read from the environment directly instead of `config_adapters::plugin_root_from_env()`. *(merged inline)*
- `cli/research-cli/src/topic_command.rs:1-5` (suggestion, medium) — Shared `Printed`/`resolve_set` live in the `outstanding`/`end-run` module; move them to a `topic_set` module.
- `skills/research/research-topic/SKILL.md:789-793` (suggestion, medium) — Optimistic-concurrency check is half in the CLI, half in prose; have `consultable` print the outline's highest round. *(merged inline)*

**General Findings**: none.

### Correctness

**Summary**: The Rust surface is sound. The slug cut is right at every boundary, suffixing compares names case-insensitively, gate order is lazy, and the per-item `HoldBack` and first-asking index reuse handle review 1's repeated-question cases. `BAD-STATUS` and `BAD-EXTRA-VALUE` split bare from quoted-empty values without double-reporting. The one substantive defect is the `SKILL.md` re-verdict rule, which compares `reports` against the first verdict and so, read literally, aborts the partial-report → confirm-proposal flow.

**Strengths**:
- `cut_before_the_overflowing_word` is correct at exactly 60, a hyphen at index 60, word cut at 41 vs 40, and no hyphen.
- `Consultation::judge` takes `FnOnce` readers, so a refused gate never reads the next gate's input.
- `plan_item` tracks first sightings via `seen`, and `index_for` reuses the first asking's index.
- `Freshness::assess` covers Missing, Unreadable, Disagrees and UncoveredRound independently.
- `vocab_value` treats quoted-empty and bare values consistently for `status` and closed extras.

**Comments**:
- `skills/research/research-topic/SKILL.md:790-795` (major, medium) — Re-verdict rule aborts the partial-report → confirm-proposal flow on the session's own report. *(merged inline at 789-794)*
- `cli/research/src/topic/plan.rs:168-174` (minor, medium) — Held-back reasons come only from a question's first all-skipped asking. *(merged inline)*
- `cli/research-adapters/src/topic.rs:270` (suggestion, high) — `level_notes_of` claims lineage order but sorts names as strings; `2-10.md` sorts before `2-2.md`.

**General Findings**:
- (minor) A finding with an upper-case `.MD` extension is counted but its level notes are never listed, because `level_notes_of` strips a case-sensitive `.md`.

### Test Coverage

**Summary**: All of review 1's test-coverage findings are fixed: stored counts of other shapes are pinned, the plan-read failure test asserts exit code, stderr and empty stdout, `topic_project` holds shared helpers, and `report_count` drift is tested at three levels. What is left is narrow: a repeated question whose index has run out, two unpinned user-facing strings, and `outstanding`'s missing-plugin-root path.

**Strengths**:
- Every review-1 test finding is closed, including shape coverage of stored counts.
- Planner tests cover every repeated-question case review 1 raised, confirmed end to end by a binary test.
- `report_count` drift is tested at domain, freshness and binary levels.
- `the_set_lifecycle_is_the_manifest_s_status_vocabulary` pins `SetStatus` to the schema.
- Replayed and live turn records are compared by equality.
- `frontmatter_allowed_tools` is tested across list layouts and a missing key.
- Every `TurnExit` variant round-trips, and unknown exits are refused.

**Comments**:
- `cli/research/src/topic/plan.rs:168-175` (minor, medium) — No test of a repeated question whose index has run out; it is likely listed twice. *(merged inline)*
- `cli/research-cli/src/consultable_command.rs:177-178` (minor, high) — The index-exhausted held-back reason is never asserted. *(merged inline at 172-181)*
- `cli/research-cli/src/counts_wording.rs:55-59` (minor, high) — The unreadable `report_count` notice text is untested. *(folded into inline at 49-61)*
- `cli/research-cli/src/main.rs:214` (minor, medium) — `outstanding`'s new plugin-root requirement has no binary test. *(folded into inline at research.md:783)*
- `tasks/shared/skill_parsing.py:80-87` (suggestion, medium) — Inline `allowed-tools` edge cases untested; `[]` yields `['']`.

**General Findings**: none.

### Code Quality

**Summary**: Review 1's code-quality findings are mostly fixed, and soundly: `TurnHistory`, the `TurnExit` union, single definitions of `RULE`/`one_of`/`only`, merged criteria registries, and the new domain types. The new problems are smaller: wording and literals copied beside code that owns them, plan warnings escaping through a closure side effect, and repeated logic or two-boolean modes in the eval's preflight, verdicts and orchestration.

**Strengths**:
- `TurnHistory.record` is the only place a `TurnRecord` is built from disk.
- A turn's exit is a closed union with `read_exit` and `recorded()`.
- `SetStatus` replaces the string list of consultable statuses.
- The planner computes `HoldBack`, removing the raw-vs-normalised mismatch structurally.
- `FreshnessCount` plus `ReportCountDrift` make the gating rule explicit in types.
- Layout constants, `TakenNames` and `DirectoryEntries` give the set's layout a single owner.
- `RULE`, `one_of`/`only` and the criteria registry are each defined once.

**Comments**:
- `cli/research-cli/src/consultable_command.rs:46-66` (minor, high) — Plan warnings escape the gate through a mutating closure.
- `cli/research-cli/src/consultable_command.rs:86-101` (minor, high) — Finding paths are built and taken apart again as strings. *(merged inline)*
- `cli/research-cli/src/consultable_command.rs:172-181` (minor, high) — Hold-back wording sits in the command and repeats `plan_wording`'s sentence. *(merged inline)*
- `cli/research-cli/src/counts_wording.rs:49-61` (minor, high) — `report_count_drift` hard-codes names `CountField` owns. *(inline)*
- `cli/research-cli/src/main.rs:290-294` (suggestion, high) — `ACCELERATOR_PLUGIN_ROOT` spelled twice. *(merged inline)*
- `cli/research/src/topic/plan.rs:159-186` (suggestion, medium) — `held_back` builds every `ItemStatus`; rename to `item_statuses`.
- `tasks/eval/ask_and_report/preflight.py:240-245` (minor, high) — "New report" predicate written twice.
- `tasks/eval/ask_and_report/preflight.py:173-193` (minor, medium) — Sources entry matching duplicated between preflight and the `Sources` judge.
- `tasks/eval/ask_and_report/verdicts.py:62-73` (minor, medium) — Two more hand-rolled frontmatter readers that disagree on where the block starts.
- `tasks/eval/ask_and_report/verdicts.py:219-223` (suggestion, medium) — `Gaps`, `Sources` and `Manifest` parsers validate scenario JSON less strictly than siblings; `bool("false")` is `True`.
- `tasks/eval/ask_and_report/orchestration.py:37-59` (suggestion, medium) — Three run modes encoded as two booleans; parse once into a `RunMode`.
- `cli/research-adapters/src/topic.rs:632-643` (suggestion, high) — Test placed above the test module's imports.
- `cli/research/src/topic/layout/stem.rs:6-7` (suggestion, high) — `crate` import wedged between `std` imports.

**General Findings**:
- (suggestion) `manifest.md`/`synthesis.md` literals remain while `findings`/`reports` got constants.

### Usability

**Summary**: Review 1's usability findings are fixed: refusals end with a retry line, the permission guidance has a narrow, copyable snippet, `--check-only` on citing-once no longer needs the marker, and `--profiles-dir` is gone. Gaps remain: the eval exits green on empty selections, the held-back recovery line is wrong for some reasons, the skill does not rule out redirects or no-op commands, and the permission rationale is inconsistent.

**Strengths**:
- Every `consultable` refusal ends with a concrete retry chain including the original question.
- The docs give a ready-to-paste `permissions.allow` snippet narrower than the skill's own grant.
- Each write-guarding prompt lists its options and re-asks on an unclear reply.
- Dropping `--profiles-dir` for `ACCELERATOR_PLUGIN_ROOT` removes a flag nobody should type.
- `counts` sentences name document, field, stored and on-disk values, and match the docs.
- `--rerun-errored` checks its preconditions and never reruns a failed session.
- `report-path` apostrophe quoting has a worked example.

**Comments**:
- `tasks/eval/ask_and_report/orchestration.py:122-139` (major, high) — The eval exits 0 when its filters or `--check-only` select no sessions. *(inline)*
- `skills/research/research-topic/SKILL.md:861` (minor, high) — One held-back recovery line for every reason, wrong for some. *(inline)*
- `skills/research/research-topic/SKILL.md:775-777` (minor, medium) — Command-shape rule bans joining commands but not redirects or no-op commands, the cause of the 3 eval failures.
- `docs-site/src/content/docs/research.md:607-613` (minor, medium) — Permission rationale does not say why `consultable` is among the four rules. *(merged inline)*
- `cli/research-cli/src/consultable_command.rs:38-43` (minor, medium) — Missing-profiles-directory error has no code and no recovery step.

**General Findings**: none.

### Compatibility

**Summary**: The contract surface is mostly additive: `extra_vocabs`, optional `report_count`, the `report` row, the `Author:` line and three new commands extend existing outputs. Older binaries accept `report_count` because the validator has no unknown-key check. Both intentional breaks are documented. What remains: `outstanding --profiles-dir` was removed with no shim or note, and the stem-change note could say more about orphaned `.levels/` trees.

**Strengths**:
- A bare `status:` failing `BAD-STATUS` is a documented Breaking entry, and `''` adds no new failure.
- `report_count` is optional and outside the freshness gate; older binaries still validate it.
- `extra_vocabs` is read by key in its one in-repo consumer.
- The `Author:` line is appended last and omitted when unset.
- `counts`/`consultable` JSON is declared add-only, and `gate` is a documented closed set.
- Old-stem findings still match by frontmatter question, with a `pair_ending` fallback.
- Public-api fixtures show only additions plus the expected `SCHEMA` length change.
- The launcher always exports `ACCELERATOR_PLUGIN_ROOT`.

**Comments**:
- `cli/research-cli/src/cli.rs:113-115` LEFT (minor, medium) — `outstanding --profiles-dir` removed with no deprecation shim or change note.
- `CHANGELOG.md:95-101` (suggestion, medium) — Stem-change note could cover the orphaned `.levels/` tree and mixed-version conducts. *(moved to general findings: outside diff hunks)*

**General Findings**: none.

### Documentation

**Summary**: The second pass resolved review 1's documentation findings, and the documented command names, flags, JSON shapes, mismatch wording, slug rules and allow rules match the code. Two small gaps remain: `outstanding`'s exit-code table lacks `E_RESEARCH_PLUGIN_ROOT`, and the permissions paragraph never justifies the `consultable` rule.

**Strengths**:
- `research.md`'s `topic` section matches `slug.rs`, `consultable_command.rs` and `cli.rs` closely.
- The four allow rules are identical across CHANGELOG, `research.md` and `SKILL.md`, and narrower than the skill's grant.
- The `--profiles-dir` removal is carried through clap help, usage block, docs and CHANGELOG.
- CHANGELOG coverage spans Breaking, Added, Changed and Upgrade notes.
- `tasks/README.md`'s eval section now separates shared and per-eval material.

**Comments**:
- `docs-site/src/content/docs/research.md:783` (minor, high) — `outstanding` exit-code table omits `E_RESEARCH_PLUGIN_ROOT`. *(inline)*
- `docs-site/src/content/docs/research.md:603-609` (minor, medium) — Permissions paragraph doesn't say why `consultable` needs an allow rule. *(merged inline)*
- `tasks/README.md:693` (suggestion, medium) — "The task refuses without it" contradicts the new `--check-only` exemption.

**General Findings**:
- (suggestion) PR description says "three rules" in Notes for Reviewers and "four narrow allow rules" in the Skill section.

### Standards

**Summary**: Review 1's standards findings are fixed and the new code follows repository conventions: no code comments reference work items, tests have no `__init__.py`, and fixtures, catalogue count, lane pins and eval registration move together. What is left is mechanical: one misplaced import, an 83-column doc comment, over-width `json!` literals, partial-reflow residue, an error-code prefix and a long test name.

**Strengths**:
- No added comment references work items, ADRs or plan phases.
- New domain types read in domain language, and `FINDINGS`/`REPORTS` replace scattered literals.
- Pure logic in `research`, reads in `research-adapters`, rendering in `research-cli`.
- Public-api fixtures, catalogue key-count, dump golden and lane pins are updated together.
- Test layout follows the pytest importlib convention.
- The report template matches its siblings and is registered everywhere it needs to be.

**Comments**:
- `cli/research/src/topic/layout/stem.rs:7-8` (minor, high) — `crate` import inserted inside the `std` import group.
- `cli/corpus/src/frontmatter_validation/mod.rs:344` (minor, high) — Doc comment on `vocab_value` runs to 83 columns.
- `cli/research-cli/tests/topic_counts.rs:133` (minor, high) — String literals inside `json!` exceed 80 columns (also 113, 168, 182).
- `skills/research/research-topic/SKILL.md:801-802` (minor, high) — Partial reflows leave orphaned short lines (also 11, 770, 929–930, 1061–1062).
- `CHANGELOG.md:666` (suggestion, high) — Partial reflows in new entries (also 672, 684–685).
- `tasks/README.md:768` (suggestion, high) — Partial reflows in the `eval:ask-and-report` section (also 780–781).
- `cli/research-cli/src/installed_profiles.rs:20` (suggestion, medium) — `E_RESEARCH_PLUGIN_ROOT` breaks the `E_TOPIC_RESEARCH_*` prefix.
- `cli/research/src/topic/layout/slug.rs:214` (suggestion, medium) — Test name too long for 80 columns and uses older cut-rule wording.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
