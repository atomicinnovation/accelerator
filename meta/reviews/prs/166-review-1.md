---
type: "pr-review"
id: "166-review-1"
title: "[0304] Cap web researchers at a per-level search budget and halt on exhausted quota"
date: "2026-10-18T08:04:01+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:166"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "usability", "documentation", "standards"]
review_number: 1
pr_number: 166
tags: []
last_updated: "2026-10-18T08:04:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #166 - [0304] Cap web researchers at a per-level search budget and halt on exhausted quota

**Verdict:** COMMENT

The domain work is solid. Budget sizing is a pure, property-tested function of a `WorstCaseTree` and a `SearchCap`, and the correctness lens found no logic errors in the binary search, the top-up or the worst-case walk. The ledger extensions are additive and tolerate older ledgers. Most findings sit at the edges: the halt-recovery prose is duplicated and incomplete, `topic_command::planned` dispatches on run mode in three places, the plan-JSON and ledger shapes are inconsistent before the additive-only contract freezes them, and a few CHANGELOG and docs passages are imprecise.

### Cross-Cutting Themes

- **Recovery guidance is duplicated and diverges** (flagged by: usability, code-quality, documentation) — The quota-exhausted recovery is written in 5 places: `search_wording.rs`, `configure`, both `research-topic` tables and docs-site. The halt-table copy, which is the one users see, leaves out the Claude Code restart and the lower-depth option, and no copy explains why `/clear` recovers and waiting does not. Keep one canonical sentence and pin it with a structure test.
- **`web` as the searching profile is a rule restated in many places** (flagged by: architecture, code-quality) — `SEARCHING_PROFILE` is compared inline in `unsearched_tree` and again in `PlanOutput::search_budget`, and the skills restate it. Give the rule one domain owner, such as `searches(profile)` or `SearchBudgets::for_pair`.
- **The withhold protocol is a prose state machine** (flagged by: architecture, usability) — The orchestrator has to repeat `--withhold-profile` until `withheld` echoes it back, and it may drop the flag after that. The ledger already holds the durable fact. Reduce this to "pass it on the next continue plan; stop if `withheld` doesn't name it", or "always pass it once halted".
- **Empty and optional fields follow mixed conventions** (flagged by: code-quality, standards, usability) — An empty `withheld` is left out of `.conduct-run.json`, while `owed_quarantine` is always written. On plans, `withheld` is always rendered but `search_budgets` and `search_demand` are left out when empty. `search_budgets` and `search_demand` are also separate top-level fields, while the ledger nests them. Choose one convention before the plan JSON's additive-only contract locks this in.
- **The sized-once snapshot does not describe itself** (flagged by: architecture, correctness, test-coverage) — The ledger does not record the cap its budgets were sized against. `SearchBudgets::at` falls back to 1 without a word. Empty first-plan budgets permanently satisfy "already sized". None of these states shows up in the plan output.

### Tradeoff Analysis

- **Compatibility vs Correctness, for legacy ledgers**: compatibility counts it as a strength that a ledger written before this change gets its budgets sized on its next continue plan. Correctness points out that this sizing uses the full cap after the session may already have spent searches, which is the mid-run resize the design rules out. Sizing a legacy ledger at the floor of 1, with a warning, would satisfy both. A plugin upgrade part-way through a run is rare, so either choice is defensible if it is documented.
- **Usability vs Autonomy, on the predicted halt**: usability suggests pausing when the first plan already warns that budgets of 1 overrun, since the user has not spent any quota yet. Proceeding still makes non-web progress. If proceeding is the intended behaviour, say so explicitly in the skill.

### Strengths

- ✅ Sizing is pure and injectable. `SearchBudgets::sized` and `unsearched_tree` are pure functions, and `search_cap::from_environment` takes a `lookup` closure, so the core is property-tested with nothing mocked.
- ✅ The property test checks fit, monotonic shape and local maximality for every cap from 1 to 260, plus a huge cap for overflow. The unit tests pin the work item's numeric acceptance cases exactly.
- ✅ The binary search is sound. It starts from a lower bound known to fit (scale 1) and an upper bound that can never fit. Demand grows monotonically with the scale, and the u64 arithmetic cannot overflow even at a cap of `u32::MAX`.
- ✅ `WorstCaseTree::below_frontier` is a true upper bound. It counts on a cloned `known` set and shares `capped_follow_ups` with the real walk, so it records no trims and cannot drift from the walk.
- ✅ The shared `harness_setting` parse removes duplicated code, and its tests moved with it. ADR-0073 plus the coupling guard follow the existing process for admitting a harness variable.
- ✅ The ledger keeps older runs working. The new fields default when absent, `level_from` rejects non-canonical keys such as `"01"` and `"+1"`, and a round-trip test covers the whole ledger.
- ✅ `Derivation`, `PlanRequest` and `Withheld` replace positional tuples, out-params and bare sets with named domain types.
- ✅ The diagnostics can be acted on. The floor warning gives demand, what is available, the cap and its source, and the fix. `--withhold-profile` fails fast with a coded error. The depth ceiling clamps or refuses, following the existing pattern.
- ✅ The `QUOTA_EXHAUSTED` phrase is pinned in both the web-profile tests and the halt-table tests, which guards the string contract between the researcher and `conduct`.

### General Findings

- 🔵 **Architecture**: The halt depends on the orchestrator re-sending `--withhold-profile` until the ledger acknowledges it. Simplify that to a single step.
- 🔵 **Usability**: The recovery wording for the search cap differs across 5 places. Choose the floor warning's version and pin it with a structure test.
- 🔵 **Documentation**: The quota halt is documented only in the CLI-reference subsection of docs-site. The user-facing `conduct` bullet should link to it.
- 🔵 **Code Quality**: `SearchBudgets::from_parts` accepts any `demand`, unrelated to the levels. Consider naming it for its persistence role, or restricting it to that role.
- 🔵 **Test Coverage**: The researcher's behaviour on the search-limit notice was verified only by a manual run. Record the procedure as a repeatable smoke checklist.
- 🔵 **Compatibility**: An older build reading a ledger from this build silently drops `withheld`. That is acceptable while the binary is locked to the plugin version. A ledger schema marker would let an older reader refuse the file instead.

### Additional Findings

- 🔵 `skills/research/research-topic/SKILL.md:283` — A predicted halt is reported, then the run proceeds anyway (usability)
- 🔵 `skills/research/research-topic/SKILL.md:567` — Halt summary: vague `ended with it:` label, `web` hard-coded in a generic template, entries listed twice (usability)
- 🔵 `skills/research/research-topic/SKILL.md:273` — The withhold-flag instructions leave the model optional behaviour to track (usability)
- 🔵 `cli/research-cli/src/topic_command.rs:714` — `unanswered` is close in name to `unfinished`, `remaining` and `pairs` (usability)
- 🔵 `cli/research-cli/src/topic_command.rs:816` — The rule that only web pairs get a budget is re-implemented in the renderer (code-quality)
- 🔵 `cli/research-cli/src/topic_command.rs:719` — Emptiness of `SearchBudgets` is checked through an iterator, and empty run-plan fields are rendered inconsistently (code-quality)
- 🔵 `cli/research-cli/src/topic_command.rs:706` — Run plans report the live search cap next to budgets sized against a stored snapshot (architecture)
- 🔵 `cli/research/src/conduct/search_budget.rs:68` — Empty budgets permanently satisfy "sized once", so later web nodes silently get 1 (correctness)
- 🔵 `cli/research/src/conduct/ledger.rs:82` — A mid-run ledger with no stored budgets is sized against the full cap (correctness)
- 🔵 `cli/research/src/conduct/search_budget.rs:11` — Which profile searches is hard-coded in the domain and repeated across the skills (architecture)
- 🔵 `cli/research/src/conduct/search_budget.rs:91` — An unsized level silently gets a budget of 1 (architecture)
- 🔵 `cli/research/src/conduct/search_budget.rs:180` — `rising_level` returns an anonymous triple and uses index arithmetic (code-quality)
- 🔵 `cli/research-cli/src/search_wording.rs:14` — A second, wording-local mapping of `SearchCapSource` (code-quality)
- 🔵 `cli/research-cli/src/search_cap.rs:26` — `SearchCapSource` and `EnvironmentSearchCap` differ in shape from their `CapSource` and `EnvironmentCap` siblings (standards)
- 🔵 `cli/research-adapters/src/conduct/ledger.rs:149` — The ledger leaves out an empty `withheld`, unlike its `owed_quarantine` precedent (standards)
- 🔵 `cli/research/src/topic/tree.rs:24` — The depth ceiling is set in Rust and restated in the skill prose (architecture)
- 🔵 `skills/config/configure/SKILL.md:468` — The search-cap coupling is not tied to the Claude Code version it was verified on (compatibility)
- 🔵 `templates/topic-research-level-note.md:16` — Project overrides of the template get no migration for `search_budget` (compatibility)
- 🔵 `docs-site/src/content/docs/research.md:579` — The recovery text assumes readers know the quota refills and that `/clear` resets it (documentation)
- 🔵 `docs-site/src/content/docs/research.md:543` — The paragraph breaks short mid-sentence at "When the run" (standards)
- 🔵 `meta/decisions/ADR-0073-admit-the-session-search-cap-as-an-optional-harness-variable.md:31` — The ADR context leaves out the quota refill and its variable (documentation)
- 🔵 `tests/unit/tasks/test_research_structure.py:404` — A verbatim snapshot of the step 2 report detects any change rather than checking a behaviour (test-coverage)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `skills/research/research-topic/SKILL.md:634` — Halt recovery omits the restart and duplicates line 613 (usability, code-quality)
**Severity**: major | **Confidence**: high | **Lens**: usability, code-quality

🟡 **Usability**

The halt summary prints this row's Recovery as `{recovery}`. It, and the matching failure-reason row at line 613, say "for larger runs in later sessions, raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`". They don't mention restarting Claude Code or lowering the depth. The floor warning (`search_wording.rs`), `configure` and docs-site all say the variable only takes effect after a restart, and they offer a lower depth as the other fix. A user who follows the halt summary literally (export the variable, `/clear`, re-run) gets no larger cap and hits the same halt again.

🔵 **Code Quality**

The same recovery text appears word for word here and at line 613. When it changes, for example once 0310 adds recovery from an interrupted run, one copy can be updated without the other, and the orchestrator then reads two conflicting instructions.

**Suggestion**: Keep the recovery in one place and have the other row refer to it ("see **Halting failures**"). Use one sentence everywhere, e.g. "run `/clear`, then re-run `conduct <slug>`; for larger runs later, raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` and restart Claude Code, or lower the depth".

---

### `cli/research-cli/src/topic_command.rs:368-382` — `planned` scatters run-mode behaviour across three dispatches
**Severity**: major | **Confidence**: medium | **Lens**: code-quality

🟡 **Code Quality**

`planned` is now about 110 lines and decides per-mode behaviour in three places: the `match &request.mode` that loads the ledger, an `if matches!(request.mode, RunMode::Start(_))` for the unhonoured-cap notice, and a second `match` that builds the ledger. Warnings go through a shared `&mut Vec<String>` that both `planned` and `budgeted` push into. `unsearched_tree(&plan.pairs)` is computed twice.

**Impact**: 0307, 0309 and 0310 all hand their integration points to this function, so each one is likely to add another mode check or another push into the side channel.

**Suggestion**: Pull the per-mode work into one step that returns a value (e.g. `OpenedRun { ledger, replaced, notices }`). Have `budgeted` return its notice instead of mutating a shared vector, and compute `unsearched_tree` once.

---

### `CHANGELOG.md:135` — CHANGELOG line exceeds the 80-column limit
**Severity**: minor | **Confidence**: high | **Lens**: standards

🔵 **Standards**

The edited line ``  (1 to 10) reports each pair's `stage`, the level-note `nodes` it still needs or the`` is about 86 columns. `.editorconfig` sets `max_line_length = 80` for `[*]`. Reflow the paragraph.

---

### `CHANGELOG.md:159-176` — Search-budget changes buried in a ~40-line bullet
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The search cap, the budgets, withholding and `unanswered` are all appended to the existing `research topic outstanding` bullet, which is now about 43 lines. The one action a consumer has to take ("A project that overrides `topic-research-level-note.md` should add the `search_budget` line") sits in the middle of it.

**Suggestion**: Give per-level `WebSearch` budgets and the quota halt their own bolded bullet under Added, and put the template-override step under `### Migrations`, where this release's other manual upgrade notes already are.

---

### `cli/research-cli/src/topic_command.rs:243-251` — `--withhold-profile` accepts unknown profile names
**Severity**: minor | **Confidence**: medium | **Lens**: usability

🔵 **Usability**

Only empty values are rejected. A typo such as `Web` or `webs` is persisted to the ledger for the rest of the run and echoed in `withheld`, but nothing is actually withheld. The skill's "`withheld` names it" check still passes. The command already takes `--profiles-dir`.

**Suggestion**: Validate each value against the profiles found there (`E_TOPIC_RESEARCH_WITHHOLD_PROFILE: unknown profile 'Web'; known: arxiv, openalex, web`). At minimum, warn when a withheld profile matches no outstanding pair.

---

### `cli/research-cli/tests/topic_outstanding.rs:3414-3418` — The production halt call is executed but not asserted
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

This test makes the call `conduct` makes after an exhaustion: `--spawned N`, acknowledging the batch that holds the exhausting web spawn, together with `--withhold-profile web`. It then discards the output. No integration test asserts that the acknowledged web spawn lands in `unfinished` and `attempted`, that no web spawn is offered, that `remaining` is 0 and that `withheld` is `["web"]` on that plan.

**Suggestion**: Capture the halted plan and assert on it, e.g. `assert_eq!(halted["unfinished"], json!([{"spawn": "01-a-web:1", "rejected": null}]))`, `assert!(spawns_of(&halted)?.is_empty())`, `assert_eq!(halted["withheld"], json!(["web"]))`.

---

### `cli/research/src/conduct/window.rs:32-34` — "Held back" now names two different spawn sets
**Severity**: minor | **Confidence**: medium | **Lens**: standards

🔵 **Standards**

`Window::held_back` holds spawns whose profile is withheld, and these are explicitly not counted in `remaining`. docs-site describes `remaining` as "the spawns held back by the concurrency", and the `--withhold-profile` help says "Hold back researcher spawns". One term now names two sets, which cuts against the ubiquitous-language rule.

**Suggestion**: Rename the field to `withheld_spawns` (to match `Withheld` and `--withhold-profile`), and use "withhold" in the CLI help. Keep "held back" or "deferred" for concurrency only.

---

### `CHANGELOG.md:196-200` — "re-issues none refused for capacity" overstates the halt
**Severity**: minor | **Confidence**: medium | **Lens**: documentation

🔵 **Documentation**

Read on its own, this says no capacity-refused spawn of any profile is retried after a halt. `research-topic` says only spawns of the halted profile are not re-issued, and "every other refused spawn follows the round rules unchanged".

**Suggestion**: Change it to "re-issues no web spawn refused for capacity".

---

### `skills/research/profiles/web-profile/SKILL.md:59-61` — No instruction for when no search budget is injected
**Severity**: minor | **Confidence**: medium | **Lens**: documentation

🔵 **Documentation**

The researcher is told to make at most `{search budget}` `WebSearch` calls, but the section doesn't cover the case where no budget is injected. That happens on query plans, or when another skill injects the profile. The level-note outputter does cover it ("When no search budget is injected, delete the `search_budget` line"). Without a fallback, the model is left with an unresolved placeholder and may guess anything from zero searches to unlimited.

**Suggestion**: State the intended fallback in one sentence.

---

### `cli/research-cli/src/topic_command.rs:731-732` — `search_budgets` and `search_demand` are split, unlike the grouped `search_cap`
**Severity**: minor | **Confidence**: medium | **Lens**: usability

🔵 **Usability**

`subagent_cap` and `search_cap` are rendered as `{value, source}` objects, and the ledger stores budgets as `{levels, demand}`. The plan instead emits a top-level `search_budgets` array next to a top-level `search_demand`. The plan JSON is additive-only, so this layout cannot change once it ships.

**Suggestion**: Before release, nest them: `"search_budgets": {"levels": [{"level": 1, "budget": 3}], "demand": 190}`.

---

## Per-Lens Results

### Architecture

**Summary**: The PR keeps the functional-core / imperative-shell split. Sizing is pure domain code in `cli/research`, the environment read sits behind an injected lookup, and the ledger is the single persistent store, following the `owed_quarantine` optional-field precedent. Withholding is profile-generic, while budgeting is tied to a hard-coded profile, which the plan acknowledges. The remaining concerns are small: the snapshot does not describe itself, and some rules are duplicated between code and skill prose.

**Strengths**:
- The shared `harness_setting` parse gives both harness caps one model of how Claude Code reads its variables.
- Sizing is a pure function of `WorstCaseTree` and `SearchCap`, with environment access injected.
- `Derivation` replaces a positional tuple and a `&mut RoundPlan` side effect.
- `Withheld` is generic per-profile state in `RunMemory`, applied in the window and passed to `RunLedger::record` as `held_back`.
- The dependency direction stays clean: `conduct::search_budget` depends on `topic::tree` and `topic::plan`, never the reverse.
- `PlanRequest` replaces a growing list of positional parameters.

**Comments**:
- `cli/research/src/conduct/search_budget.rs:11` (minor, medium) — Which profile searches is hard-coded in the domain and repeated across the skills. Put a `searches(profile)` predicate behind both call sites.
- `cli/research-cli/src/topic_command.rs:706` (minor, medium) — Run plans report the live `search_cap` next to budgets sized against a stored snapshot. Store the sizing cap and source in the ledger's `search_budgets`.
- `cli/research/src/conduct/search_budget.rs:91-93` (suggestion, low) — `SearchBudgets::at` merges "sized to 1" with "never sized". Return `Option` and let the shell apply the fallback and raise a notice.
- `cli/research/src/topic/tree.rs:24` (suggestion, low) — `Depth::DEEPEST = 10` is restated in the skill. Pin it with a structure test, or report the ceiling from the CLI.

**General Findings**:
- (minor) The halt protocol depends on the orchestrator re-sending the withhold flag until the ledger acknowledges it. Simplify that to a single step.

### Code Quality

**Summary**: The code is well structured. The pure, injectable sizing module and the `Derivation`, `PlanRequest` and `harness_setting` refactors make it clearer than before. The main maintainability cost is `topic_command::planned`, which dispatches on run mode in three places and pushes warnings through a shared `&mut Vec`. Smaller issues: the web-only budget rule is re-implemented in the renderer, there are readability nits in the sizing code, and the recovery text is duplicated.

**Strengths**:
- The JavaScript-compatible parse moved out of `subagent_cap.rs`, and its tests moved with it.
- `Derivation` replaces the `(Stage, Vec<Trim>)` tuple and makes `derive` pure.
- `PlanRequest` bundles the plan inputs, and `PlanOutput` borrows it.
- `from_environment` takes a `lookup` closure, and `sized` and `unsearched_tree` are pure.
- `Withheld` is a domain newtype with intent-revealing methods.
- Named render helpers make `PlanOutput::render` shorter.
- `declared_follow_ups` counts on a clone of `known`, and `capped_follow_ups` is shared with the real walk.

**Comments**:
- `cli/research-cli/src/topic_command.rs:368-382` (major, medium) — `planned` scatters run-mode behaviour across three dispatches and mutates a shared notices vector.
- `cli/research-cli/src/topic_command.rs:816-824` (minor, medium) — The rule that only web pairs get a budget is re-implemented in the renderer.
- `cli/research-cli/src/topic_command.rs:719-733` (minor, medium) — Emptiness is checked through an iterator, and empty run-plan fields are rendered inconsistently. Add `SearchBudgets::is_empty`.
- `cli/research/src/conduct/search_budget.rs:180-196` (suggestion, medium) — `rising_level` returns an anonymous triple and uses index arithmetic. Return a named `Raise` struct and zip each level with the one above it.
- `cli/research-cli/src/search_wording.rs:14-17` (suggestion, low) — A second mapping of `SearchCapSource` to prose. Move it next to `reported_as`.
- `skills/research/research-topic/SKILL.md:634` (minor, medium) — The quota-exhausted recovery text is duplicated across two tables.

**General Findings**:
- (suggestion) `SearchBudgets::from_parts` lets callers pair budgets with any demand. Name it for its persistence role, or restrict it to that role.

### Test Coverage

**Summary**: Coverage is strong and proportionate to the risk. The sizing has exact unit tests and a property test over 256 trees × 260 caps, the ledger has round-trip and corrupt-shape tests, and the CLI is covered end to end. The gaps are small: the main halt call is executed but not asserted, and the fallback budget of 1 for an unstored level is pinned only at the domain level.

**Strengths**:
- `search_budget.rs` pins the exact budgets and demand for every acceptance case.
- The property test checks fit, shape and maximality for every cap from 1 to 260, plus overflow.
- The ledger round-trip and table-driven corrupt-shape tests include non-canonical level keys.
- The `WorstCaseTree` tests cover single-pass roots, capped growth, dedup, the frontier at the depth limit, and that counting records no trims.
- The window tests cover withheld spawns not being offered, composing, freeing slots and staying unfinished, and a ledger test reproduces the re-offered batch bug.
- The CLI tests cover notice ordering, sizing only on the first plan, the floor warning firing once, legacy ledgers, and query plans.
- The parser tests moved intact into `harness_setting.rs`.
- The `QUOTA_EXHAUSTED` phrase is pinned on both sides of the contract.

**Comments**:
- `cli/research-cli/tests/topic_outstanding.rs:3414-3418` (minor, medium) — The production halt call (acknowledge plus withhold) is executed but not asserted.
- `cli/research-cli/src/topic_command.rs:816-824` (suggestion, medium) — The fallback budget of 1 for an unstored level is not pinned at the CLI level.
- `tests/unit/tasks/test_research_structure.py:404-409` (suggestion, low) — A verbatim snapshot of the step 2 report detects any change rather than checking a behaviour.

**General Findings**:
- (suggestion) The researcher's behaviour on the search-limit notice was verified only manually. Record the procedure as a repeatable checklist.

### Correctness

**Summary**: No logic errors in the core algorithms. The binary search starts from a bound known to fit and a bound that can never fit, demand grows monotonically, the top-up preserves non-increasing budgets because the worst-case levels are contiguous, the worst-case walk is a true upper bound that records no trims, withholding keeps held-back spawns unattempted, and the ledger round-trips. The 2 concerns are low-confidence edge cases in the "sized once" rule.

**Strengths**:
- The invariants of `largest_fitting_scale` hold, and its u64 arithmetic cannot overflow at `u32::MAX`.
- `topped_up` terminates, because every raise adds at least 1 and the total is bounded by `available`.
- `below_frontier` is an upper bound, counted on a cloned `known` set.
- The window partitions spawns in the right order: attempted, then withheld, then fresh.
- `level_from` accepts only the canonical key form, and the new fields default when absent.
- `Depth::requested` vs `Depth::new` bounds the walk without breaking deeper stamped findings.
- A quota-exhausted spawn reappears in `unfinished` and is recorded once.

**Comments**:
- `cli/research/src/conduct/search_budget.rs:68-70` (minor, low) — Empty budgets permanently satisfy "sized once", so later web nodes silently get 1. Store `None` when the tree is empty, or render `None`.
- `cli/research/src/conduct/ledger.rs:82-87` (minor, low) — A mid-run ledger with no stored budgets is sized against the full cap. Size only on `RunMode::Start`.

**General Findings**: none.

### Compatibility

**Summary**: The contract changes are additive and tolerate older data. The ledger fields are optional, the plan JSON only gains fields, and the binary is locked to the plugin version. The `--depth` ceiling is the only tightened contract, and it is acceptable because depth above 1 had no effect in 1.23.0 and the skill clamps before calling. 2 small concerns: the Claude Code version the coupling was verified on is not documented, and template overrides have no migration.

**Strengths**:
- Older ledgers keep working: the new keys default when absent.
- `Depth::requested` is kept separate from `Depth::new`, so findings stamped deeper than 10 still read.
- `search_budget` is an optional level-note extra, and the golden is updated.
- The harness parse is shared, with the parity tests intact. ADR-0073 admits the variable.
- `public-api.txt` is regenerated for every changed signature.

**Comments**:
- `skills/config/configure/SKILL.md:468-471` (minor, medium) — The search-cap coupling is not tied to the Claude Code version it was verified on (2.1.301).
- `templates/topic-research-level-note.md:16` (minor, medium) — Project overrides of the template get no migration for `search_budget`.

**General Findings**:
- (suggestion) A ledger written by this build loses its withheld profiles if an older build reads it. Consider a schema marker.

### Usability

**Summary**: The diagnostics name the value, the source and the next step, the new flag fails fast, and the defaults work with no configuration. The friction is in the recovery guidance (the restart is missing, the wording is generic in name but `web`-specific in content, and halt entries are duplicated) and in plan-JSON naming and grouping, which matters more because the contract is additive-only.

**Strengths**:
- The floor warning can be acted on in full.
- The unhonoured-cap warning quotes the value, and fires only on `--start` when the run has web pairs.
- Budgets are sized from Claude Code's default of 200, and `search_cap` is reported with its source.
- `--withhold-profile` fails fast with a coded error, and the ledger remembers it.
- The depth ceiling clamps or refuses, following the existing pattern.
- The web profile replaces the harness's misleading advice with a checkable outcome string.

**Comments**:
- `skills/research/research-topic/SKILL.md:634` (major, high) — Halt recovery says to raise the cap but omits restarting Claude Code.
- `skills/research/research-topic/SKILL.md:283-289` (minor, medium) — A predicted halt is reported, then the run proceeds anyway.
- `skills/research/research-topic/SKILL.md:567-581` (minor, medium) — Halt summary: vague `ended with it:` label, `web` hard-coded in a generic template, duplicated entries.
- `skills/research/research-topic/SKILL.md:273-281` (suggestion, medium) — The withhold-flag instructions leave the model optional behaviour to track.
- `cli/research-cli/src/topic_command.rs:243-251` (minor, medium) — `--withhold-profile` accepts unknown profile names.
- `cli/research-cli/src/topic_command.rs:731-732` (minor, medium) — `search_budgets` and `search_demand` are split top-level fields.
- `cli/research-cli/src/topic_command.rs:714` (suggestion, low) — `unanswered` is close in name to `unfinished`, `remaining` and `pairs`.

**General Findings**:
- (minor) The recovery wording for the search cap differs across 5 places.

### Documentation

**Summary**: The documentation is thorough and accurate against the code on the details that matter: the reserve rounding, the shape rule, the JSON field names and the error codes. The gaps are about readability and context: the budget feature and its upgrade step are buried in the CHANGELOG, one phrase overstates the halt, the refill caveat assumes knowledge the reader may not have, and the web profile has no instruction for when no budget is injected.

**Strengths**:
- The docs-site claims check out against the code.
- The sizing description matches `SearchCap::reserve` and `rising_level`.
- ADR-0073 records its options and its departures from ADR-0071.
- The doc comments explain why, not what.
- The Halting failures table puts the reason, what it halts, the flag and the recovery in one place.
- The configure skill's search-cap block covers the setup step that is easy to miss.

**Comments**:
- `CHANGELOG.md:159-176` (minor, high) — The search-budget changes are buried in a ~40-line bullet.
- `CHANGELOG.md:196-200` (minor, medium) — "re-issues none refused for capacity" overstates what the halt suppresses.
- `docs-site/src/content/docs/research.md:579-581` (minor, medium) — The recovery text assumes readers know the quota refills and that `/clear` resets it.
- `skills/research/profiles/web-profile/SKILL.md:59-61` (minor, medium) — No instruction for when no budget is injected.
- `meta/decisions/ADR-0073-...md:31-36` (suggestion, low) — The context leaves out the quota refill and `CLAUDE_CODE_WEB_SEARCH_REFILLS_PER_HOUR`.

**General Findings**:
- (minor) The quota halt is documented only in the CLI-oriented docs-site subsection.

### Standards

**Summary**: Close adherence to project conventions: domain-led names, `E_TOPIC_RESEARCH_*` codes, `search_cap` mirroring `subagent_cap`, the coupling guard and ADR updated together, and nearly every comment explaining a why. Small consistency gaps: "held back" now has two meanings, the search-cap module differs in shape from its sibling, the ledger serialises empty `withheld` differently from `owed_quarantine`, and there are 2 wrapping issues.

**Strengths**:
- The shared `harness_setting` parser.
- New error codes follow the existing naming and message shape.
- Admitting the variable follows the established process (allowlist, ADR, pinned tests).
- The plan JSON stays additive-only, and `search_cap` mirrors `subagent_cap`.
- Domain naming is strong, and `SEARCHING_PROFILE` follows the `DEFAULT_PROFILE` precedent.
- The doc comments capture why-context.

**Comments**:
- `cli/research/src/conduct/window.rs:32-34` (minor, medium) — "Held back" now names two different spawn sets.
- `cli/research-cli/src/search_cap.rs:26-42` (minor, medium) — `SearchCapSource` and `EnvironmentSearchCap` differ in shape from `CapSource` and `EnvironmentCap`.
- `cli/research-adapters/src/conduct/ledger.rs:149-151` (minor, medium) — The ledger leaves out an empty `withheld`, unlike `owed_quarantine`.
- `CHANGELOG.md:135` (minor, high) — The line exceeds 80 columns.
- `docs-site/src/content/docs/research.md:543-544` (suggestion, high) — The paragraph breaks short mid-sentence.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
