---
type: "pr-review"
id: "158-review-1"
title: "[0303] Clamp research concurrency to the subagent cap"
date: "2026-10-12T03:02:13+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:158"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "documentation", "standards"]
review_number: 1
pr_number: 158
tags: []
last_updated: "2026-10-12T03:02:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #158 - [0303] Clamp research concurrency to the subagent cap

**Verdict:** COMMENT

This is a well-built change. Concurrency resolution moves out of skill prose into a pure domain module. `Concurrency` can be built only through resolution, so the spawn window can no longer run unbounded. The environment read sits behind an injected lookup, and the correctness lens found no defects. The remaining findings are about the edges users see. The clamp warning says the value was clamped but not how to lift the cap. A malformed `--concurrency` flag degrades quietly where `--depth` fails. The harness-specific parsing rules sit in the domain crate, and nothing detects when they drift from Claude Code.

### Cross-Cutting Themes

- **The cap's source is invisible to the user** (flagged by: usability, compatibility, documentation) — The clamp warning, `--help` and the JSON output all say nothing about where the cap came from: `ACCELERATOR_SUBAGENT_CAP`, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` or the built-in 20. A malformed `ACCELERATOR_SUBAGENT_CAP` is ignored silently. On Claude Code 2.1.144–2.1.216, which enforces no cap, the warning points users towards a variable their harness ignores. Carrying the cap's provenance through `SubagentCap`, and naming it in the warning, would address all four findings together.
- **A malformed `--concurrency` flag degrades quietly** (flagged by: standards, usability, documentation) — `--concurrency abc` clamps to 1, and warns only on `--start`. `--depth abc` and the removed `--limit abc` both exit 1. The docs-site page says only that the value "is validated". The 0282 knob rule applies to config values, and an explicit flag is the clearest signal of what the user intended.
- **Claude Code internals in the domain crate** (flagged by: architecture, compatibility) — `HARNESS_DEFAULT`, `harness_slots` and `is_javascript_whitespace` copy undocumented harness behaviour inside `research::conduct::concurrency`, and that surface is pinned in `public-api.txt`. Nothing records the Claude Code version they were checked against, so a parser change upstream would quietly bring back over-offering.
- **Plain `outstanding` queries now depend on the concurrency knob** (flagged by: architecture, usability, compatibility) — An empty `research.topic.concurrency` now fails the tick query too, and it hides set-resolution errors because it runs first. The PR notes acknowledge this. The CHANGELOG does not yet say that plain queries are now bounded by concurrency.

### Tradeoff Analysis

- **Harness parity vs user feedback**: Silent fall-through on an unparseable cap value exactly mirrors Claude Code (compatibility, correctness). For the Accelerator-owned `ACCELERATOR_SUBAGENT_CAP`, though, a user who sets it deliberately and then gets no signal is worse off (usability). Recommendation: keep silent fall-through for the harness variable, and warn for the Accelerator-owned one.
- **Integration confidence vs suite cost**: Re-running the cap-parser table through the binary proves the environment wiring end to end, but costs about 17 process launches and means updating the parser cases in three places (test-coverage). Two representative values would prove the wiring.

### Strengths

- ✅ `Concurrency` has only a `pub(crate)` constructor and comes only from `RequestedConcurrency::resolve`. The bound is enforced by the type system, not by an `Option<usize>` / `usize::MAX` convention.
- ✅ Functional core, imperative shell: `subagent_cap::from_environment` takes an injected lookup, and the config read is a lazy `FnOnce`. A behavioural test proves the flag never reads config.
- ✅ Correctness checked branch by branch: the resolution order, saturating digit parsing, the U+0085/U+FEFF differences between JS `trim` and Rust whitespace, no underflow on `remaining`, and retry rounds that always terminate.
- ✅ The coupling guard admits by name and file together. A banned name on the same line is still flagged, and a missing reader raises. ADR-0071 records why the `--subagent-cap "$VAR"` design was dropped.
- ✅ A test ties the catalogue default to `SubagentCap::HARNESS_DEFAULT`, which turns an implicit agreement into a checked one.
- ✅ The integration tests clear both cap variables on every binary call, and they assert exact `warnings` arrays and stderr ordering.
- ✅ Naming is consistent across the flag, config key, JSON field and domain type (`concurrency`). The breaking rename only touches `[Unreleased]` surface, and the binary is pinned to the plugin version.

### General Findings

- 🔵 **Architecture**: ADR-0071 is still `proposed`, but the coupling guard and `subagent_cap.rs` already put it into effect. Accept it before merging.
- 🔵 **Usability**: An empty concurrency now blocks the plain tick query, and the error doesn't say whether `config.md` or `config.local.md` holds the empty value.
- 🔵 **Compatibility**: The CHANGELOG should say that plain `outstanding` queries are now bounded by `concurrency` as well.
- 🔵 **Code Quality**: `the_built_in_default_sits_at_the_harness_cap` lives in `context.rs`, but it tests neither code in that module nor anything near it.
- 🔵 **Test Coverage**: Only the positive-integer wording is checked against the skill prose. The clamp and empty-value wording could drift.
- 🔵 **Documentation**: The PR description says the docs-site page covers the retry rounds, but it does not. They are documented only in the skill and the CHANGELOG.

### Additional Findings

- 🔵 `cli/research/src/conduct/concurrency.rs:140` — A second positive-integer parser sits beside `topic_command`'s `positive`, with different overflow semantics (code-quality)
- 🔵 `cli/research-cli/src/topic_command.rs:245` — Clamp and invalid-value warnings never appear outside `--start` (usability)
- 🔵 `docs-site/src/content/docs/research.md:464` — The JSON-shape sentence is fixed for `remaining` but still lists `depth`/`unfinished`/`trims`/`shallow` as depth-only (documentation)
- 🔵 `cli/research-cli/src/main.rs:214` — Concurrency is resolved before set resolution and in every mode (architecture)
- 🔵 `cli/research-cli/src/plan_wording.rs:1` — `plan_wording` takes on a second responsibility (architecture)
- 🔵 `tasks/lint/claude_coupling.py:67` — `_in_scope` calls `_named_file` only for its side effect (code-quality)
- 🔵 `tasks/lint/claude_coupling.py:18` — The "default stated where it is read" rule does not visibly hold for the admitted reader (documentation)
- 🔵 `cli/research-cli/src/cli.rs:105` — The `--help` text doesn't say how to raise the cap or that it defaults to 20 (documentation)
- 🔵 `skills/config/configure/SKILL.md:463` — Users of pre-2.1.217 harnesses should be pointed at `ACCELERATOR_SUBAGENT_CAP` (compatibility)
- 🔵 `skills/research/research-topic/SKILL.md:485` — The capacity-refusal remedy row packs four fixes into one sentence (usability)
- 🔵 `cli/research-cli/tests/topic_outstanding.rs:1486` — The cap-parser table re-run through the binary duplicates the domain tests (test-coverage)
- 🔵 `tests/unit/tasks/test_claude_coupling.py:225` — The missing-file check covers only the reader, not the admitted test file (test-coverage)
- 🔵 `cli/research-cli/tests/topic_outstanding.rs:156` — The `command()` helper relies on a comment where a name could say what it does (standards)
- 🔵 `.accelerator/skills/implement-plan/instructions.md:1` — The new instruction lines carry trailing whitespace (standards)

---
*Review generated by /review-pr*

## Inline Comments

### `cli/research-cli/src/plan_wording.rs:101-112` — Above-cap warning doesn't say how to raise or silence the cap
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

The clamp warning reads `research.topic.concurrency is 24, above the subagent cap of 20 — clamping to 20`. It says what happened, but not where the cap of 20 came from (`ACCELERATOR_SUBAGENT_CAP`, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, or the built-in fallback), or what to change. As the PR notes say, every project configured above 20 will see this on every `conduct` run, including on harnesses that enforce no cap, until they set an environment variable they may never have heard of.

**Suggestion**: Carry the cap's source in `SubagentCap`, and add a short remedy to the warning, e.g. `… above the subagent cap of 20 (Claude Code's default) — clamping to 20; lower it, or raise the harness's subagent cap`. When the cap came from `ACCELERATOR_SUBAGENT_CAP`, name that variable instead.

---

### `cli/research-cli/src/topic_command.rs:118-125` — A malformed `--concurrency` flag clamps to 1 while `--depth` fails
**Severity**: minor | **Confidence**: medium | **Lens**: standards, usability

🔵 **Standards**

`OutstandingFlags::parse` rejects a malformed `--depth` with `E_TOPIC_RESEARCH_DEPTH`. Its sibling `--concurrency` accepts any non-empty string and then warns and clamps it to 1, and only on `--start`. The removed `--limit` used the strict rule. One command now validates two numeric flags of the same shape in two different ways.

🔵 **Usability**

A typo in the explicit flag, which is the clearest signal of what the user intended, turns a 20-wide run into a serial one. On a plain query or a continuation, nothing is printed to explain it.

**Suggestion**: Keep clamp-to-1 for config-file values, as the 0282 knob rule requires. Reject an invalid `Provenance::Flag` value the way `--depth` is rejected, e.g. `E_TOPIC_RESEARCH_CONCURRENCY: --concurrency must be a positive integer, got 'abc'`.

---

### `cli/research/src/conduct/concurrency.rs:11-31` — Claude Code parsing rules live in the domain crate with no drift marker
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, compatibility

🔵 **Architecture**

`SubagentCap::first_honoured`/`from_setting`, `harness_slots`, `is_javascript_whitespace` and `HARNESS_DEFAULT` put Claude Code's JS parsing rules inside `research::conduct::concurrency`, and that surface is pinned in `public-api.txt`. The domain only needs "a positive number of slots". ADR-0071 lists "the CLI should not be tied to one agent harness" as a driver, yet the agent-neutral `ACCELERATOR_SUBAGENT_CAP` is also parsed by Claude Code's rules. Consider moving the parsing and fall-through into `cli/research-cli/src/subagent_cap.rs`, and leaving `SubagentCap::new(NonZeroU32)` in the domain.

🔵 **Compatibility**

The default and the parser copy behaviour Claude Code has not published as a contract. If the harness changes either (for example to `parseInt`, under which `"8abc"` would be honoured), the CLI quietly over-offers again, and the retry rounds hide the problem. Record the Claude Code version this was checked against, and consider reporting the cap and its source in the `outstanding` JSON so that drift shows up.

---

### `cli/research/src/conduct/concurrency.rs:56-57` — Doc comments restate the type and function names
**Severity**: minor | **Confidence**: medium | **Lens**: standards

🔵 **Standards**

`/// The most agents `conduct` spawns at once.` on `Concurrency`, `/// How many subagents the harness runs at once.` on `SubagentCap` (line 6), and the first line of `first_honoured`'s doc (line 16) all restate the item's name. CLAUDE.md treats comments as a last resort. The sibling public items in this module have none, and this PR's own history already drops a restating comment on `from_environment`.

**Suggestion**: Delete these three. Keep the JS `String.prototype.trim` paragraph and the `is_javascript_whitespace` note, which record a real external constraint.

---

### `tasks/lint/claude_coupling.py:86-87` — No test pins exact-name admission against a prefix-extended name
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`_admitted` relies on equality and on `_SHAPE` matching greedily. No test writes `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS_OVERRIDE` into the reader. If someone switched to `startswith`, or made `_SHAPE` non-greedy, every existing test would still pass, and the guard would quietly widen.

**Suggestion**: Add a `TestOptionalHarnessVariables` case that writes `f'"{_CAP}_OVERRIDE"'` into `_CAP_READER` and asserts that it is flagged.

---

### `cli/research-cli/tests/topic_outstanding.rs:1630-1641` — Unreadable-config test passes on any exit-1 failure
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`an_unreadable_config_exits_1` checks only for exit 1, a non-empty stderr and an empty stdout. Later steps also read config and fail with exit 1. If `configured_concurrency` started swallowing its error, this test would still pass.

**Suggestion**: Assert on a fragment of the expected config error message, and add `assert!(!project.ledger().exists())` as the empty-value sibling does.

---

### `cli/research-cli/src/subagent_cap.rs:9-16` — A malformed `ACCELERATOR_SUBAGENT_CAP` is ignored without any signal
**Severity**: minor | **Confidence**: medium | **Lens**: usability

🔵 **Usability**

Silent fall-through makes sense for Claude Code's own variable. But `ACCELERATOR_SUBAGENT_CAP` belongs to Accelerator, and a user who sets it to `0` or `abc` did so on purpose. They then get the default cap with no indication that their setting was ignored.

**Suggestion**: Warn when an Accelerator-owned variable is set but not honoured, e.g. `ACCELERATOR_SUBAGENT_CAP is 'abc', not a positive integer — ignoring it`. Keep the silent fall-through for the harness variable.

---

### `CHANGELOG.md:126-130` — Hard-to-parse sentence in the `outstanding` entry
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

One sentence carries five facts. The "read from" clause attaches ambiguously, so it reads as though the environment variables set `--concurrency` rather than cap it. The cap's precedence is also repeated in the `research.topic.concurrency` entry below.

**Suggestion**: Split it, e.g. "`--concurrency` falls back to `research.topic.concurrency` and is reported as `concurrency`. An empty value exits 1 with `E_TOPIC_RESEARCH_CONCURRENCY`." Leave the cap's precedence to the concurrency entry.

---

### `docs-site/src/content/docs/research.md:492-499` — "is validated" does not say what happens to an invalid value
**Severity**: minor | **Confidence**: medium | **Lens**: documentation

🔵 **Documentation**

A non-positive or non-integer value is not rejected: it is clamped to 1 with a warning, and only on `--start`. Only an empty value exits 1. A reader who sees that the exit-code table lists just the empty case would expect other invalid values to be rejected too.

**Suggestion**: State the rule: "a value that is not a positive integer is clamped to 1 with a warning (an empty one exits 1)". If the flag behaviour changes per the `topic_command.rs` comment, update this sentence to match.

---

### `cli/research-cli/src/plan_wording.rs:116-125` — `empty_concurrency` has wording for a built-in default that can never be empty
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

`Provenance::BuiltInDefault` shares the "set it to a positive integer or remove it" arm, but the catalogue default is `"20"`. That branch can never be reached, and its advice would be wrong if it ever were, because a default cannot be removed.

**Suggestion**: Let the type rule the case out. Give `EmptyConcurrency` a narrower source with only `Flag | ConfigFile`, or treat an empty built-in default as a programming error in `resolve`.

---

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound. Resolution moves into the CLI, the type system enforces that the window is bounded, the environment read is a thin adapter, and the coupling guard is loosened by name and file under ADR-0071. Minor concerns: Claude Code parsing rules in the domain crate, `plan_wording` taking on a second responsibility, and ADR-0071 still `proposed`.

**Strengths**: `pub(crate)`-only `Concurrency`; functional core and imperative shell with an injected lookup and a lazy config read; dependencies point inward (the domain knows no env var names); the guard admits by name and file and raises on a missing reader; a test ties the default to the cap; tradeoffs are recorded openly.

**Comments**:
- `cli/research/src/conduct/concurrency.rs:17-31` (minor, medium) — Harness-specific parsing rules live in the domain crate.
- `cli/research-cli/src/plan_wording.rs:1-3` (suggestion, low) — `plan_wording` takes on a second responsibility.
- `cli/research-cli/src/main.rs:214-222` (suggestion, low) — Concurrency is resolved before set resolution and for every mode; an empty value hides slug errors.

**General Findings**:
- (minor) ADR-0071 is still `proposed` while the code depends on it.

### Correctness

**Summary**: No correctness defects. The resolution order, empty and malformed handling, the cap clamp, the JS-parity parser, the retry rounds always terminating, and the coupling-guard admission all check out.

**Strengths**: The resolution order rules out `NotAPositiveInteger` and `AboveCap` both being raised; `saturating_digits` cannot overflow; U+0085/U+FEFF are handled correctly; `first_honoured` falls through in the right order; `admits` cannot underflow; the lazy config read; provenance tells an empty value from an absent key; retry rounds shrink monotonically and record each refusal once; a mixed-name line is flagged.

**Comments**: none.

**General Findings**: none.

### Code Quality

**Summary**: The domain module is clean and intention-revealing. Minor points: a duplicated positive-integer parser, an unreachable wording arm, and a check called only for its side effect.

**Strengths**: `window` takes `Concurrency`, and tests opt into `test_support::unbounded()`; the env lookup is injected; the config read is lazy and tested behaviourally; domain and presentation are separated; `_named_file` is extracted and `_admitted` is pure; behaviour-named tests.

**Comments**:
- `cli/research/src/conduct/concurrency.rs:140-153` (minor, medium) — Second positive-integer parser beside `topic_command`'s `positive`.
- `cli/research-cli/src/plan_wording.rs:116-125` (minor, medium) — Unreachable `BuiltInDefault` wording.
- `tasks/lint/claude_coupling.py:67-70` (suggestion, medium) — `_in_scope` calls `_named_file` only for its side effect.

**General Findings**:
- (suggestion) The default-matches-cap test lives in `context.rs`, which it does not exercise.

### Test Coverage

**Summary**: Coverage is strong and fits the risk: mutation-resistant domain tables, exact integration assertions, env isolation, and structural prose tests. Small gaps in the unreadable-config test, exact-name admission and missing-file parametrisation, plus a duplicated parser table at the binary level.

**Strengths**: Mutation-resistant `concurrency.rs` tests; a default-to-cap tie test; `command()` clears both cap vars; `assert_warned` pins warning ordering; negative paths covered (empty, `--start`-only, rejected `--limit`/`--subagent-cap`); a behavioural laziness test; coupling-guard admission coverage.

**Comments**:
- `cli/research-cli/tests/topic_outstanding.rs:1630-1641` (minor, medium) — The unreadable-config test passes on any exit-1 failure.
- `cli/research-cli/tests/topic_outstanding.rs:1486-1502` (suggestion, medium) — The cap-parser table re-run through the binary duplicates the domain tests.
- `tasks/lint/claude_coupling.py:86-87` (minor, medium) — No test for a prefix-extended name.
- `tests/unit/tasks/test_claude_coupling.py:225-229` (suggestion, low) — The missing-file check covers only the reader.

**General Findings**:
- (suggestion) Only the positive-integer wording is checked against the prose.

### Compatibility

**Summary**: Low risk. The breaking rename, the error-code swap and the default change all touch only `[Unreleased]` surface, the binary is version-pinned, and JSON output is additive only. The remaining risk is coupling to undocumented Claude Code internals.

**Strengths**: An additive `concurrency` JSON field; the breaking changes are unreleased; the launcher pins the binary to the plugin version; the public-api fixture is updated; JS `trim` parity; both cap variables are optional with a stated default.

**Comments**:
- `cli/research/src/conduct/concurrency.rs:11-14` (minor, medium) — The harness default and parser mirror undocumented internals with no drift detection.
- `skills/config/configure/SKILL.md:463-464` (suggestion, medium) — The clamp applies on supported Claude Code versions that enforce no cap; point those users at `ACCELERATOR_SUBAGENT_CAP`.

**General Findings**:
- (minor) Plain `outstanding` queries now depend on config and can exit 1; add this to the CHANGELOG.

### Usability

**Summary**: Clear developer-experience gains from consistent naming, provenance-aware diagnostics and the env-read cap. The main gap is that the clamp warning gives no remedy. A mistyped flag throttling quietly and a malformed `ACCELERATOR_SUBAGENT_CAP` being ignored silently are both surprising.

**Strengths**: Flag, conduct flag and knob share one name; diagnostics name their source and tailor the hint to it; the env read keeps commands auto-approvable; the built-in default clamps silently; `concurrency` is visible in the JSON; the configure skill answers the follow-up questions.

**Comments**:
- `cli/research-cli/src/plan_wording.rs:101-112` (major, high) — The above-cap warning doesn't say how to raise or silence the cap.
- `cli/research/src/conduct/concurrency.rs:117-124` (minor, medium) — A mistyped `--concurrency` quietly throttles to 1.
- `cli/research-cli/src/topic_command.rs:245-250` (minor, medium) — Warnings are invisible outside `--start`.
- `cli/research-cli/src/subagent_cap.rs:9-16` (minor, medium) — A malformed `ACCELERATOR_SUBAGENT_CAP` is ignored silently.
- `skills/research/research-topic/SKILL.md:485` (suggestion, low) — The remedy row packs four fixes into one sentence.

**General Findings**:
- (minor) An empty concurrency blocks the tick query, and the error doesn't name the config file.

### Documentation

**Summary**: Good and consistent across the configure skill, docs-site, CHANGELOG, the skill and the ADR, with no stale `--limit` or 24. Minor issues: a dense CHANGELOG sentence, a vague "is validated", a half-corrected JSON-shape sentence, and an unclear admission rule.

**Strengths**: The rename is made in place in `[Unreleased]`; the exit-code row is swapped, not added; the configure-skill claims are pinned by tests; ADR-0071 records the rejected design; the new comments meet the policy; the remedy row matches the failure reason word for word.

**Comments**:
- `CHANGELOG.md:126-130` (minor, high) — Hard-to-parse `outstanding` sentence.
- `docs-site/src/content/docs/research.md:492-499` (minor, medium) — "is validated" is vague.
- `docs-site/src/content/docs/research.md:464-468` (minor, medium) — The JSON-shape sentence is only half corrected.
- `tasks/lint/claude_coupling.py:18-21` (suggestion, low) — "Default stated where it is read" does not hold visibly.
- `cli/research-cli/src/cli.rs:105-107` (suggestion, low) — `--help` gives no route to raising the cap.

**General Findings**:
- (suggestion) The docs-site page does not mention capacity-refusal retries, though the PR description says it does.

### Standards

**Summary**: Follows conventions closely: the error-code family, module layout, public-api fixture, env-var naming and ADR frontmatter all match. Issues: the two numeric flags are validated differently, and a few doc comments restate names.

**Strengths**: `E_TOPIC_RESEARCH_CONCURRENCY` naming; one word across flag, key, field and type; module layout matches `conduct`; `ACCELERATOR_<AREA>_<SETTING>` naming; `plan_wording` naming and the `# Errors` doc shape.

**Comments**:
- `cli/research-cli/src/topic_command.rs:118-125` (minor, medium) — `outstanding` validates its two numeric flags differently.
- `cli/research/src/conduct/concurrency.rs:56-57` (minor, medium) — Doc comments restate names.
- `cli/research-cli/tests/topic_outstanding.rs:156-158` (suggestion, medium) — A comment explains a helper whose name could carry the intent.
- `.accelerator/skills/implement-plan/instructions.md:1-3` (suggestion, medium) — Trailing whitespace.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
