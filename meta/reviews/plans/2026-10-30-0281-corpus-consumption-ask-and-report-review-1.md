---
type: "plan-review"
id: "2026-10-30-0281-corpus-consumption-ask-and-report-review-1"
title: "Plan Review: Corpus Consumption: Ask and Report Implementation Plan"
date: "2026-10-30T02:09:25+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-30-0281-corpus-consumption-ask-and-report"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "usability", "compatibility", "standards"]
review_number: 1
review_pass: 5
tags: ["research", "research-topic", "ask", "report", "eval"]
last_updated: "2026-10-31T10:59:22+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Corpus Consumption: Ask and Report Implementation Plan

**Verdict:** REVISE

Every lens rated the product design sound. The per-extra vocabulary is schema
data guarded by invariant tests, `ReportSlug` is a pure CLI-owned type, one
shared consulting section names the citing-once rule without restating it, and
the manifest is written last from disk. The arithmetic also checks out: the C7
and C8 slugs, 102 sessions, the row and key counts and almost every line
citation. The risk sits in the Phase 4 eval harness, the only behavioural
evidence for about thirty acceptance criteria. As specified, it runs
pre-flight against whichever `accelerator` is on `PATH`, hides the 0330 grant
lapse behind a blanket per-turn `--allowedTools`, can pass turns that did
nothing, and has an `appended_round` criterion that fails every confirm turn.
There are no critical findings and 18 major ones after deduplication.

### Cross-Cutting Themes

- **The eval hides the 0330 grant lapse** (flagged by: architecture,
  compatibility, security). Both verbs run `metadata derive` and `validate`
  after a user reply, when the skill's `allowed-tools` grant has cleared.
  Passing `--allowedTools Bash(accelerator:*)` on every turn makes the eval
  pass on behaviour real users won't get. 0330 still lists the non-`conduct`
  research-topic verbs as unaffected.
- **Pre-flight, sessions and judges run different CLI builds** (flagged by:
  correctness, architecture, compatibility, usability). In the eval,
  pre-flight resolves `accelerator` from `PATH`, which means released binaries
  without the `report` row, so the equivalence and rewrite variants fail
  `UNKNOWN-KIND` before any session runs. `citing_once/runs.py:165` sets the
  precedent this inherits.
- **`appended_round` always fails and checks too little** (flagged by:
  correctness, test-coverage, security). It requires `outline.md` to equal its
  pre-turn text plus one block, while the prose advances `last_updated` and
  `last_updated_by`. It also never compares the appended items with the
  proposal the user confirmed.
- **Eval sessions are not isolated** (flagged by: security, architecture,
  test-coverage, compatibility, usability). Sessions run deliberately injected
  content under `acceptEdits`, with broad grants, the developer's `~/.claude`
  settings, hooks and MCP servers, the default model, and only the prerelease
  plugin ID disabled. `tasks/probe/claude_permissions.py` already shows how to
  isolate a session.
- **Global checks can pass sessions that did nothing or followed an injection**
  (flagged by: security, test-coverage). A turn judged only on negative
  criteria passes when nothing ran. The outside-the-set write guard applies only
  when a turn declares `writes`. The snapshot excludes `.claude/` and `.git/`.
  Denied actions and stray Bash commands are never inspected.
- **Multi-turn mechanics are unproven until Phase 5** (flagged by:
  compatibility, architecture, test-coverage, code-quality, usability). The
  only Phase 4 real run is the single-turn G1 family. The turn loop has no
  offline test, and timeout and non-zero-exit handling, plus whether
  infrastructure failures are kept apart from behavioural ones, are not
  specified.
- **The headless extraction is partial** (flagged by: code-quality,
  architecture, standards, compatibility, test-coverage). The moved helpers
  take `citing_once.projects.Project`. There would be two stream-json parsers.
  `_with_fields` is needed in Phase 3 before the refactor happens. The old
  import paths break the "tests pass unchanged" claim. The new packages have no
  `__init__.py`.
- **Duplicated rule and slug logic** (flagged by: code-quality, architecture,
  standards, correctness, test-coverage). The consult section restates
  `finalise`'s freshness gate. `ReportSlug` copies `QuestionSlug`'s hyphenation.
  Its `is_taken: Fn(&str) -> bool` probe turns I/O errors into "taken".
- **Breadth resolution contradicts the misplaced-flag rule** (flagged by:
  usability, correctness). The proposal step resolves breadth through
  "flag > resolved value" while the knob rules say a `--breadth` on these verbs
  is ignored.
- **`--family` is too coarse for Phase 5's own check** (flagged by:
  usability, test-coverage). The G, S1 and S2 families mix `ask` and `report`
  sessions, so the Phase 5 check "every `ask` session passes" can't be
  selected.
- **No CHANGELOG entries** (flagged by: standards, compatibility). There are
  two verbs, a subcommand, a validator code, a `print-schema` field, a template
  key and a manifest-template change, and no `[Unreleased]` or Migrations
  entries.

### Tradeoff Analysis

- **Representative eval vs passable eval**: Architecture and compatibility
  want the eval to expose the 0330 grant lapse. Security wants narrower
  per-turn grants. Dropping per-turn grants entirely would make every
  post-reply turn fail headless until 0330 lands. Recommendation: narrow the
  per-turn `--allowedTools` to the documented commands, record the masking as
  an accepted limitation tied to 0330, and add one interactive manual check in
  Phase 6.
- **ADR-0045 consistency vs the work item's scope**: Architecture wants a CLI
  gate verdict so the mechanical gates stop living in prose and in a Python
  mirror. The work item says both verbs reuse `finalise`'s gate "as shipped",
  and the plan's What We're NOT Doing excludes a CLI freshness gate.
  Recommendation: keep the scope, record the tradeoff in Decisions, and pin
  the Python `count_rule` against the same fixtures as the Rust helpers.
- **Richer user output vs exact-match judging**: Usability wants
  status-keyed recovery lines and a `Covered:` block. Each new fixed line adds
  surface for the judges to match, and test-coverage already flags drift
  between the judges and the skill. Recommendation: adopt the recovery-line
  changes, which are still fixed forms, and define the format strings once in
  a module that both the structure tests and `judges.py` import.
- **Shared eval plumbing vs Phase 4 scope**: Code quality and architecture
  would move the plan, pool, summary and save plumbing into `headless`.
  Recommendation: do the minimum now (Path-typed helpers, one parser,
  `with_fields`, explicit imports) and leave the orchestration extraction for
  the third eval.

### Findings

#### Major

- 🟡 **Architecture + Compatibility**: Post-reply `accelerator` commands lose
  the skill's grant (0330), and the eval hides it
  **Location**: Phase 4 §2–3 turn commands; Phases 5–6 confirm and accept paths
  Every write in both verbs happens after a user reply, when the
  `allowed-tools` grant has cleared, so real users get 3–5 prompts per accepted
  report or confirmed proposal, or a denial when headless. The per-turn
  `--allowedTools Bash(accelerator:*)` makes the eval blind to this.
- 🟡 **Correctness + Architecture + Compatibility + Usability**: Eval pre-flight
  runs the `PATH`-resolved `accelerator`, not the dev binaries
  **Location**: Phase 3 §2 `preflight.py`; Phase 4 §3 eval task
  Released binaries reject `kind: report`, so the task exits 1 at pre-flight,
  or checks gates against a schema other than the one the sessions use, until
  0281 ships.
- 🟡 **Correctness + Test Coverage + Security**: `appended_round` contradicts
  the required `last_updated` advance and never checks the appended items
  **Location**: Phase 4 §3 judges table; Phase 5 §2 Propose research
  All 12 confirm-turn sessions (A1 C3, R1 C2, P1) fail every run. A session
  that appends items other than those it proposed, or more than breadth
  allows, would pass the content check.
- 🟡 **Security**: The eval silently creates and keeps the dev-launcher consent
  marker
  **Location**: Phase 4 §3 `tasks/eval/__init__.py`
  One run leaves the checkout permanently armed for unverified launchers.
  Verified: `test_dev_launcher_marker_is_gitignored_and_unshipped` asserts that
  the marker is absent from the repo root, so the next `mise run` fails.
- 🟡 **Security + Test Coverage**: Global checks can pass sessions that did
  nothing or followed an injection
  **Location**: Phase 4 §2 Global checks; §3 `writes`, `runs.py`
  Negative-only turns pass vacuously, the outside-the-set guard depends on a
  declared `writes`, the snapshot skips `.claude/` and `.git/`, and denied or
  stray actions go unseen.
- 🟡 **Usability + Test Coverage**: `--family` is the only filter, so Phase 5's
  check cannot be expressed
  **Location**: Phase 4 §3 task options; Phase 5 Manual Verification
  Mixed-verb families make the contributor pay for `report` sessions that are
  expected to fail and then sort `results.json` by hand.
- 🟡 **Standards**: Adding the corpus build to `test:integration:research`
  breaks the pinned lane-build map
  **Location**: Phase 3 §1 `mise.toml`
  `_CLI_DEV_BUILDS` pins the lane to `{"build:cli:research:dev"}` (verified),
  and `test:integration:research-exhaustive` runs the same directory without
  the corpus build.
- 🟡 **Standards + Compatibility**: No CHANGELOG entries or Migrations note
  **Location**: Whole plan; Migration Notes
  Recent research-topic plans all schedule `[Unreleased]` entries, and
  overriders of `topic-research-manifest.md` need to know about
  `report_count: 0`.
- 🟡 **Test Coverage**: Hand-authored expectations are not cross-checked
  mechanically
  **Location**: Phase 3 §1–2; Phase 3 Manual Verification
  The `sources` tiers, G2 mismatch strings and N1/E1 paths could be derived
  from `rule.py`, `count_rule` and `report-path`. A fixture slip otherwise
  shows up as 3/3 failures after a paid run.
- 🟡 **Test Coverage**: Compound criteria get one planted failure each
  **Location**: Phase 4 §2 Criteria and Global checks
  The other clauses of `refusal`, `report`, `sources`, `offers`, `manifest`,
  `proposal` and `next_steps` can be dropped unnoticed. The rule-read check
  never covers a session that never reads the rule at all.
- 🟡 **Usability**: The status refusal's only recovery step is wrong for
  `briefed`, `outlined` and most `researching` sets
  **Location**: Phase 5 §2 Status gate
  `synthesise` itself refuses on `briefed` and `outlined`. After this feature's
  own confirmed proposal, the advice wastes a re-synthesis and then hits a
  second refusal.
- 🟡 **Usability**: The fully-conducted refusal names only `conduct`, which
  guarantees a second refusal
  **Location**: Phase 5 §2 Fully conducted gate
  `conduct` reopens a closed set to `researching`, so the next `ask` is refused
  by the status gate. The freshness refusal and the `Next:` line both name the
  full chain.
- 🟡 **Security + Architecture + Test Coverage + Compatibility + Usability**:
  Injection-laden eval sessions run with broad grants under the developer's
  live Claude config
  **Location**: Phase 4 §1 `headless/claude.py`; §3 `conversation.py`
  An injection followed by the model could reach user-scope allow rules, MCP
  servers and environment secrets. Results depend on who runs the eval and on
  their model, and 102 persisted sessions accumulate in `~/.claude`.
- 🟡 **Compatibility + Architecture**: `--session-id`/`--resume` is not
  exercised against the installed Claude Code until Phase 5
  **Location**: Phase 4 Manual Verification; Key Discoveries: Multi-turn sessions
  If resume behaves differently, every multi-turn scenario fails for harness
  reasons only after the verbs exist, which blurs red-for-the-right-reason.
- 🟡 **Test Coverage + Code Quality + Usability + Architecture**: Multi-turn
  orchestration has no offline test, and its failure handling is unspecified
  **Location**: Phase 4 §3 `conversation.py`, `runs.py`
  `TimeoutExpired` propagates through the pool. Nothing stops a conversation
  after a failed turn. Infrastructure failures look the same as regressions,
  and snapshot alignment is never tested.
- 🟡 **Test Coverage**: Reaching the `open` gate does not prove S2, P1 and N2
  set up what they claim
  **Location**: Phase 3 §2 `preflight.py`; §4 Scenario setups
  A `replace` that misses, a mis-keyed breadth config or a misplaced override
  still reaches `open`, so S2 could pass with no skipped pair at all.
- 🟡 **Architecture**: ADR-0045 is applied selectively, and the gate logic
  gains a parallel Python implementation
  **Location**: Decisions table; What We're NOT Doing; Phase 3 §2
  The slug cut moves to the CLI while the equally mechanical freshness compare
  stays in prose and is mirrored in Python, so gate semantics live in three or
  four places.
- 🟡 **Usability**: The equivalence pick prompt is ambiguous for a destructive
  rewrite
  **Location**: Phase 6 §2 report step 4
  A bare `accept` at the pick prompt leaves the model to guess between a
  rewrite and a new report, and no rule makes an unclear reply re-ask.

#### Minor

- 🔵 **Correctness**: The prescribed consult prose contains the
  `RULE_SIGNATURE` "joined by `; `"
  **Location**: Phase 5 §2 gate 3
  `test_no_carrier_restates_the_citing_once_rule[consult-…]` fails as
  written (verified at `test_research_structure.py:1446`).
- 🔵 **Correctness + Test Coverage**: A bare `coverage:` passes validation, and
  the empty and bare boundaries are untested
  **Location**: Phase 1 §1 `check_extra_vocabularies`
  `EMPTY-PLACEHOLDER` fires only on `""`. The manifest cases pass today, so
  they never go red.
- 🔵 **Correctness**: Coverage and Sources merge tiers over different citation
  scopes
  **Location**: Phase 5 §2 Judge coverage; Phase 6 §2 step 5
  A gap's evidence could show `tier-1` in Sources while coverage judged it
  `tier-3`.
- 🔵 **Correctness**: `report`'s mid-verb manifest edit conflicts with the
  unamended "manifest last" rule
  **Location**: Validate every write; Phase 6 §2 steps 6–7
  A model following the general rule could fold `report_count` into the
  reopen edit, which breaks the ordering criterion.
- 🔵 **Correctness**: `Ok(None)` from `FileReader::read` does not mean the path
  is free
  **Location**: Phase 2 §2
  A directory or a dangling symlink at the bare name is reported as free.
- 🔵 **Correctness**: "76 of them multi-turn" is wrong; the turn table gives 52
  **Location**: Performance Considerations
- 🔵 **Usability + Correctness**: Breadth resolution for proposals contradicts
  the misplaced-flag rule
  **Location**: Phase 5 §2 knob rules and Propose research
- 🔵 **Usability**: A skipped-only refusal is headed "has research
  outstanding", and its two lists are unlabelled
  **Location**: Phase 5 §2 Fully conducted gate
- 🔵 **Usability**: The coverage output does not show why a part counts as
  covered or as a gap
  **Location**: Phase 5 §2 Present
- 🔵 **Usability + Standards**: `report-path` takes the question as one
  argument, unlike `fetch`, and is documented inside the `outstanding` section
  **Location**: Phase 2 §2
- 🔵 **Usability**: The eval replies are full sentences, not the one-word
  replies the prompts teach
  **Location**: Phase 4 §3 reply table
- 🔵 **Security + Test Coverage**: The untrusted-data contract is an
  enumerated list, and its extension to reports is untested
  **Location**: Phase 5 §2 Read; Phase 6 §2 steps 2 and 5
- 🔵 **Security**: The model escapes the question for shell and YAML, and no
  test feeds it hostile input
  **Location**: Phase 6 §2 steps 3 and 5
- 🔵 **Code Quality + Architecture**: The consult section restates the
  freshness gate and the rejection rule
  **Location**: Phase 5 §2 Freshness, Propose research
- 🔵 **Code Quality + Architecture + Standards**: `ReportSlug` copies
  `QuestionSlug`'s hyphenation and departs from its module conventions
  **Location**: Phase 2 §1
- 🔵 **Code Quality + Architecture + Standards + Compatibility**: The headless
  extraction is partial, its moved helpers depend on `citing_once`, and the
  old imports break
  **Location**: Phase 4 §1
- 🔵 **Code Quality**: The `set` mutation re-implements
  `projects._with_fields` before the shared refactor exists
  **Location**: Phase 3 §2
- 🔵 **Code Quality**: The scenario model stays untyped until judging, so a
  typo surfaces only after the sessions have run
  **Location**: Phase 3 §2; Phase 4 §3
- 🔵 **Code Quality**: Report step 5 packs four responsibilities into one
  paragraph
  **Location**: Phase 6 §2 step 5
- 🔵 **Test Coverage + Architecture**: The judge templates duplicate the
  skill's fixed lines with no cross-check
  **Location**: Implementation Approach; Phase 5–6 structure tests
- 🔵 **Test Coverage**: The gate scenarios run once, and the three-run rule is
  pinned only as session counts
  **Location**: Phase 4 §3 scenario table
- 🔵 **Test Coverage**: The slug tests cannot tell the 61-character window
  from a 60-character one, and none slugs to exactly 60 characters
  **Location**: Phase 2 §1
- 🔵 **Test Coverage**: The "unreadable means taken" branch of `report-path`
  is untested
  **Location**: Phase 2 §2
- 🔵 **Test Coverage**: Several prose changes have no red structure test
  **Location**: Phases 5–6 §1
- 🔵 **Standards + Code Quality**: The new "vocabulary" naming diverges from
  the established "vocab"
  **Location**: Phase 1 §1

#### Suggestions

- 🔵 **Code Quality**: The flag argument `first` in `turn_command` should
  become `opening_turn`/`resumed_turn` (Phase 4 §3).
- 🔵 **Code Quality**: Drop the running count of later additions from the
  `violation.rs` module doc (Phase 1 §1).
- 🔵 **Code Quality**: Rename the existing `_report()` to `_run_report()` and
  call the new slicer `_report()` (Phase 6 §1).
- 🔵 **Test Coverage**: Pin the report template's `##` headings with a
  structure test (Phase 1 §3).
- 🔵 **Test Coverage**: Assert that citing-once still passes
  `--no-session-persistence` after the refactor (Phase 4 §1).
- 🔵 **Standards**: Reuse citing-once's module names (`projects.py`,
  `verdicts.py`) and avoid `family.family` (Phases 3–4).
- 🔵 **Standards**: Give the shared section a plain heading,
  `### Consulting the corpus` (Phase 5 §2).
- 🔵 **Standards**: Give `report-path` its own docs section (Phase 2 §2).
- 🔵 **Standards**: Put the measured time in the mise description, and place
  the task under the opt-out comment (Phase 4 §4, Phase 6).
- 🔵 **Standards**: Wrap the slicer snippets to 80 columns (Phases 5–6 §1).
- 🔵 **Architecture**: Define the fixed output formats once, in a module the
  structure tests and `judges.py` both import.
- 🔵 **Architecture**: Note the SKILL.md growth, now seven verbs and seven
  injections (Phase 6 §2).
- 🔵 **Correctness**: Specify the `## Set` anchor for a manifest with no
  Synthesis line (Phase 6 §2 step 6).
- 🔵 **Usability**: Add a `--model` option and record the model in
  `results.json` (Phase 4 §4).

### Strengths

- ✅ The per-extra vocabulary is modelled as schema data (`ExtraVocabulary`
  on `SchemaRow`). Its invariant and bounded-scope tests mirror the
  `empty_list_extras` guards, and the exhaustive `row_json` destructure turns a
  forgotten field into a compile error.
- ✅ The deterministic slug, the 60/61 cut and the collision suffixes live in
  a pure `ReportSlug` behind a `report-path` that writes nothing. Correctness
  checked the C7 (54), C8 (60) and rewrite-variant (60) slugs by hand.
- ✅ One **Consulting the corpus** section owns the gates, reading, judgement
  and proposal. `consult` and `ask` join `RULE_READERS` and `report` joins
  `RULE_CARRIERS`, which extends "name, never restate" enforcement instead of
  copying rule text.
- ✅ The manifest stays the aggregate root: content is written first,
  `report_count` is taken from disk, and the reopen is the final write.
- ✅ Reading inline with no subagents makes the `.levels/` exclusion and the
  untrusted-data contract hold by construction. The slug's ASCII-only output
  rules out path traversal from the question.
- ✅ The eval is written before the verbs as their failing acceptance test,
  and a mechanical pre-flight runs the real `validate` and `outstanding` before
  any paid session.
- ✅ The fixed output lines let the eval judge by exact match without a model
  grader. The global checks observe the `.levels/` exclusion through
  path-segment checks plus the `Lanternfish` canary.
- ✅ Session counts sum to 102. The SCHEMA 19→20, catalogue 67→68 and nearly
  every line citation check out against the code.
- ✅ CLI changes ship before the prose that calls them, so every published
  prerelease stays self-consistent. `report_count` is optional, so existing
  manifests stay valid.
- ✅ The plan lists every surface a new template must be registered on:
  schema row, `TEMPLATE_KEYS`, `dump.golden`, glyph map, conformance, the 0065
  row and the public-API snapshots.

### Recommended Changes

1. **Fix `appended_round`** (addresses: appended_round contradiction; nothing
   checks the appended round)
   Compare `outline.md` with its `last_updated` and `last_updated_by` lines
   masked, and assert separately that they advanced. Parse the appended
   `- [ ]` items and require them to equal the previous turn's
   `Proposed Round` items. Add planted-bad tests for an extra item, a changed
   profile and an edited earlier round.
2. **Run one toolchain through pre-flight, sessions and judges** (addresses:
   PATH-resolved pre-flight; manual steps omit the dev binaries)
   In the eval, pass `cli/target/debug/accelerator-research` and
   `accelerator-corpus` to `preflight_problems` and to the `report` judge's
   validation. List the four dev-launcher variables in Phase 1's manual check
   and in Manual Testing Steps.
3. **Stop auto-creating `.accelerator-dev-launcher`** (addresses: consent
   marker)
   When the marker is missing, refuse at pre-flight with the CONTRIBUTING.md
   instructions. Otherwise create it and remove it in a `finally`, with a test
   that the marker's state is the same before and after the run.
4. **Confront 0330 explicitly** (addresses: post-reply grant lapse)
   Relate the plan to 0330 and add `ask`/`report` to its affected list. Narrow
   the per-turn `--allowedTools` to the documented commands and drop
   `Bash(accelerator:*)`, Agent and Task. Record the masking under Decisions,
   and add a Phase 6 manual check of an interactive `report … accept` with no
   allow rules.
5. **Isolate eval sessions** (addresses: broad grants under the live config;
   model and config inherited)
   Pass `--setting-sources project`, `--strict-mcp-config` and `--model`.
   Disable both marketplace plugin IDs. Record the model and
   `claude --version` in `results.json`, and fail below v2.1.144.
6. **Harden the global checks** (addresses: vacuous negative turns; blind
   snapshot)
   On every turn:
   - fail any changed path outside the set;
   - fail on a missing log, missing exit code or no main-conversation text;
   - fail on a Bash command off the documented allow-list;
   - fail on a non-empty `permission_denials`;
   - assert that `.claude/` is unchanged.

   The loader should reject a turn that declares neither `writes` nor
   `refusal`.
7. **Prove the multi-turn mechanics in Phase 4** (addresses: resume
   unproven; orchestration untested)
   - Add a real two-turn smoke check, such as A2 `ask` C1 → `reject`, that
     asserts the same `session_id`.
   - Unit-test the turn loop with a fake `claude`.
   - Stop a conversation at its first failed or timed-out turn.
   - Catch `TimeoutExpired`, and count errored sessions apart from failed
     ones.
   - Print the output directory before the first session starts.
8. **Derive expectations mechanically in the pre-flight** (addresses:
   hand-authored expectations; `open` gate setups)
   Check the `sources` expectations through `headless/sources.py` and
   `rule.py`. Derive the G2 mismatch strings from `count_rule`, and compare N1
   and E1 paths with a real `report-path` run. Check S2's skipped pair, P1's
   `config get research.topic.breadth` and N2's template resolution with the
   real binaries.
9. **Parametrise planted failures per clause** (addresses: compound criteria)
   Parametrise each criterion's planted failures per clause. Add a
   rule-never-read case and a Coverage-in-a-later-turn case. Add a check that
   every Sources entry appears in some finding.
10. **Add `--verb` and `--scenario` filters** (addresses: `--family` too
    coarse)
    Restate Phase 5's criterion as `--family G1,…,A2 --verb ask` exiting 0,
    and print the planned session count first.
11. **Rework the refusal recovery lines** (addresses: status refusal
    recovery; fully-conducted refusal; skipped-only header)
    Key the status gate's recovery to the current status, keeping
    `synthesise {slug}` in every variant. End the conducted groups with
    `…, then synthesise {slug}.`. Use a neutral header with a lead-in for each
    group, and key G3's `omits` on phrases specific to each gate.
12. **Make the equivalence pick unambiguous** (addresses: pick prompt;
    full-sentence replies)
    Number the equivalent reports and accept replies such as `rewrite 1`,
    `new` or `reject`. Any unclear reply re-asks, and no rewrite happens
    without a named report. Use the prompts' own vocabulary in the scripted
    replies, and add a bare-`accept` E1 scenario that expects `writes: []`.
13. **Register the pre-flight lane correctly** (addresses: lane-build pin)
    Red first: update `_CLI_DEV_BUILDS`, then handle
    `research-exhaustive`, or give the pre-flight its own lane. Update the
    lane description and docstring to match.
14. **Schedule CHANGELOG entries in every shipping phase** (addresses: no
    CHANGELOG)
    - Phase 1: `BAD-EXTRA-VALUE`, `extra_vocabularies`, the template key, and
      a Migrations note for overriders of `topic-research-manifest.md`.
    - Phase 2: `report-path`.
    - Phases 5 and 6: the `ask` and `report` verbs.
15. **Record the ADR-0045 tradeoff and give the freshness gate one owner**
    (addresses: ADR-0045 selective; freshness restated)
    Move the freshness comparisons and their mismatch phrases into
    **Validate every write**, and have `finalise` and the consult section name
    them. State the rejection rule only once. Pin the Python `count_rule`
    against the Rust helpers' fixtures.
16. **Finish the headless extraction** (addresses: partial extraction;
    `_with_fields`; import paths)
    - Move the refactor to the start of Phase 3.
    - Make the helpers take a `Path` and add a public `with_fields`.
    - Use one `events(log)` parser, with `tool_calls` as a filter over it.
    - Update the imports explicitly, with no shims, and add an `__init__.py`
      to each package.
    - Assert that citing-once still passes `--no-session-persistence`.
17. **Tighten `ReportSlug` and its probe** (addresses: hyphenation
    duplication; `Ok(None)` is not free; slug and probe tests)
    Share a private `hyphenate` in `slug.rs`, cut with `.get()`, and update
    the module docs. List `reports/` once and treat only `NotFound` as free.
    Add tests for exactly 60 characters, for a 61st-character hyphen with an
    earlier hyphen, and for a fake reader returning `Err`.
18. **Correct the smaller prose points** (addresses: the RULE_SIGNATURE,
    breadth, manifest-last, Sources scope, the anchor, bare `coverage:`,
    76→52)
    - Write "separated by `; `" and add "joined by `; `" to the avoid-list.
    - Resolve breadth from config only.
    - Add a manifest-last exception for a partial report with a confirmed
      proposal.
    - Give Sources tiers the same citation scope as coverage.
    - Anchor the `reports/` line on the Synthesis line, or the last `## Set`
      entry.
    - Flag a bare `coverage:`.
    - Correct the multi-turn count to 52.
19. **Close the remaining red-test gaps** (addresses: prose without red
    tests; gate scenarios run once; judge template drift; template headings)
    Add phrase tests for every new prose sentence. Add a loader invariant that
    judgement scenarios have `times == 3`, and run G2 and G4 three times. Share
    the format strings between the structure tests and `judges.py`, and pin the
    report template's headings.

## Per-Lens Results

### Architecture

**Summary**: Structurally the plan is sound. The per-extra vocabulary is
schema data with invariant tests, the slug moves into a pure domain type
behind a CLI command that writes nothing, both verbs share one Consulting
section that names the citing-once rule, and the manifest stays the aggregate
root. The main risks: every write happens after a user reply, which collides
with the one-turn grant (0330) and is hidden by the eval's blanket
`--allowedTools`. ADR-0045 is applied selectively. The harness extraction is
partial, and resume and transient-failure handling are not de-risked.

**Strengths**:
- The per-extra vocabulary is schema data, guarded by
  `every_extra_vocabulary_names_an_extra_of_its_row` and
  `only_a_report_closes_an_extra_vocabulary`.
- `ReportSlug` is a pure type with an injected collision probe, a clean
  functional core and imperative shell.
- The shared Consulting section has one owner, and the readers and carriers
  extend the existing enforcement.
- Writes are ordered with the manifest as aggregate root, and `report_count`
  is idempotent from disk.
- Reading inline with no subagents makes the `.levels/` and untrusted-data
  contracts hold by construction, and the cost is stated.
- The report `id` scheme prevents a corpus-wide DUPLICATE-ID clash.
- Each phase's dependencies are explicit, and the eval is written before the
  verbs.

**Findings**:
- 🟡 major / high — **Every write in both verbs runs after a user reply, when
  the Bash grant has lapsed (0330); the eval hides this.** *Location:* Phase 4
  §3; Key Discoveries; Phases 5–6. After `accept` or `confirm` the verbs run
  `metadata derive` and `validate`, but the grant clears when the user
  replies. 0330 lists the non-`conduct` verbs as unaffected; this plan changes
  that silently. *Suggestion:* State the constraint, relate the plan to 0330,
  add `ask`/`report` to its affected list, and either add a post-reply turn
  without the blanket grant or record the masking as an accepted tradeoff.
- 🟡 major / medium — **ADR-0045 is applied selectively; the gate logic stays
  in prose and gains a parallel Python implementation.** *Location:* Decisions;
  What We're NOT Doing; Phase 3 §2. The freshness compare, `report_count` and
  the reading of `outstanding` JSON stay model-executed, then reappear as
  `count_rule` and `gate_reached` in Python, beside the Rust test helpers.
  An unreadable concurrency config makes a read-only `ask` refuse.
  *Suggestion:* Add a read-only CLI gate verdict, or record the tradeoff and
  pin the Python `count_rule` against the Rust fixtures.
- 🔵 minor / high — **The Consulting section restates `finalise`'s freshness
  gate.** *Location:* Phase 5 §2 gate 2. *Suggestion:* Make **Validate every
  write** the single owner, which both `finalise` and the Consulting section
  name.
- 🔵 minor / high — **The headless extraction is partial, and `headless` would
  depend on `citing_once`.** *Location:* Phase 4 §1, §3. The helpers take
  `citing_once.projects.Project`, there are two stream-json parsers, the
  `runs.py` plumbing is copied, `_with_fields` is reused, and old imports
  imply shims. *Suggestion:* Take a `Path`, use one event parser, move the
  generic plumbing, and state the import policy.
- 🔵 minor / medium — **Pre-flight, sessions and judges may run against
  different CLI builds.** *Location:* Phase 3 §2; Phase 4 §3. *Suggestion:*
  Pass one toolchain value through all three.
- 🔵 minor / medium — **The red acceptance run is single-turn, so `--resume` is
  first exercised in Phase 5.** *Location:* Phase 4 Manual Verification.
  *Suggestion:* Add a two-turn smoke check, such as A2 `ask` C1 → `reject`.
- 🔵 minor / medium — **A 102-session run that must pass 3/3 has no recovery
  from transient failures.** *Location:* Phase 4 §3; Performance.
  *Suggestion:* Record infrastructure failures separately, and add
  `--rerun-failed` into the same `--out`.
- 🔵 minor / medium — **Eval sessions are not isolated from the developer's
  environment.** *Location:* Phase 4 §3. *Suggestion:* Pin `--model`,
  restrict the setting sources, and record the environment.
- 🔵 minor / medium — **`ReportSlug` duplicates `QuestionSlug`'s hyphenation,
  and its probe cannot report I/O errors.** *Location:* Phase 2.
  *Suggestion:* Share the hyphenation, and make the probe return
  `Result<bool, E>`.
- 🔵 suggestion / medium — **The fixed output formats are a three-way contract
  with no shared source.** *Location:* Implementation Approach; `judges.py`;
  structure tests. *Suggestion:* Define the format strings once, in a Python
  module the tests and judges import.
- 🔵 suggestion / low — **One SKILL.md now loads seven verbs and seven
  template injections on every invocation.** *Location:* Phase 6 §2.
  *Suggestion:* Note it under Performance, and consider per-verb files
  before 0347.

### Code Quality

**Summary**: The plan is carefully designed for maintainability. Every phase
starts red, and the new code follows existing patterns. The remaining risks
are duplication (slug hyphenation, restated gate prose, two stream-json
parsers, a re-implemented frontmatter rewriter), where the shared helpers are
split, naming collisions around "report", and unspecified error handling in
the multi-turn harness. There are no critical or major findings.

**Strengths**:
- Red-green ordering holds in every phase, with the refactor guarded before
  and after the move.
- The injected `is_taken` closure and the command prefixes keep the logic pure
  and testable.
- The shared Consulting section keeps both verbs thin, with
  `{verb}`-parameterised refusals.
- The fixed output lines give deterministic, diagnosable judging.
- The schema extension mirrors existing patterns, with invariant tests and an
  exhaustive destructure.
- Slug computation moves into the CLI, as ADR-0045 asks.
- The mutation and reply keys are closed sets.

**Findings**:
- 🔵 minor / medium — **The Consulting section restates the freshness gate
  and the rejection semantics it claims to reuse.** *Location:* Phase 5 §2.
  The rejection rule appears three times. *Suggestion:* Give the freshness
  gate a named block in **Validate every write**, and state the rejection
  rule only in the Consulting section.
- 🔵 minor / high — **`ReportSlug` copies `QuestionSlug`'s hyphenation
  pipeline verbatim.** *Location:* Phase 2 §1. *Suggestion:* Put it in
  `slug.rs` with a shared private `hyphenate`, and update the `layout` module
  doc.
- 🔵 minor / medium — **`report-path` folds I/O errors into "taken" and leaves
  the `reports/` layout to the adapter.** *Location:* Phase 2 §2.
  *Suggestion:* Return a set-relative path, as `Stem::finding_path` does.
  List `reports/` once with `DirReader::list`, and propagate errors.
- 🔵 minor / medium — **The `headless` package mixes concerns, and its moved
  helpers depend on citing-once's `Project`.** *Location:* Phase 4 §1.
  *Suggestion:* Name the package for what it holds, keep the citation grammar
  separate, and type the helpers on `root: Path`.
- 🔵 minor / medium — **Two parsers for the same stream-json log.**
  *Location:* Phase 4 §1–2. *Suggestion:* Use one `events(log)` parser, with
  `tool_calls` as a filter.
- 🔵 minor / medium — **The `set` mutation re-implements
  `projects._with_fields` before the refactor exists.** *Location:* Phase 3
  §2. *Suggestion:* Move the refactor to the start of Phase 3, with a public
  `with_fields`.
- 🔵 minor / medium — **The scenario model stays untyped until use.**
  *Location:* Phase 3 §2; Phase 4 §3. *Suggestion:* Parse scenarios at load
  time into a `Gate` Literal and per-mutation and per-criterion dataclasses.
- 🔵 minor / medium — **Per-turn failure and timeout handling is
  unspecified.** *Location:* Phase 4 §3. *Suggestion:* Record each turn's
  status at once, stop at the first failed turn, catch `TimeoutExpired`, and
  print the output directory first.
- 🔵 suggestion / high — **`first` is a flag argument.** *Location:* Phase 4
  §3. *Suggestion:* Use `opening_turn` and `resumed_turn`.
- 🔵 suggestion / high — **The closed-vocabulary concept is modelled twice,
  with diverging names.** *Location:* Phase 1 §1. *Suggestion:* Use one term,
  and share a private `outside_vocabulary`.
- 🔵 suggestion / high — **The module doc keeps a running changelog tally.**
  *Location:* Phase 1 §1. *Suggestion:* Describe the taxonomy without counts.
- 🔵 suggestion / high — **The `_report()` name collision is worked around
  rather than fixed.** *Location:* Phase 6 §1. *Suggestion:* Rename the old
  helper `_run_report()`.
- 🔵 minor / medium — **Step 5 packs four responsibilities into one
  paragraph.** *Location:* Phase 6 §2. *Suggestion:* Split it into
  **Frontmatter**, **Rewrite**, **Body** and **Sources** sub-steps.

### Test Coverage

**Summary**: The test approach is strong. The eval comes before the verbs, the
deterministic logic is in Rust, pre-flight runs the real binaries, and every
criterion is mapped. The main risks are in the harness. Negative-only turns
can pass vacuously, compound criteria get a single planted failure each,
hand-authored expectations are not checked mechanically, the turn
orchestration has no offline test, and `appended_round` skips breadth.
Smaller gaps are in slug boundaries, validator edges and prose without red
tests.

**Strengths**:
- Phase 4 is a real failing acceptance test for Phases 5 and 6.
- The slug and collision rules are Rust-tested and don't depend on the model.
- Pre-flight runs the real binaries, and mutations raise on a missing target.
- The global checks use solid proxies (a canary, path segments, Agent/Task),
  and the canary is confined by a fixture test.
- The validator tests assert exact equality and add invariants and a golden
  line.
- The refactor is guarded, and the eval is pinned out of CI.
- Every 0281 criterion maps to a test or scenario.

**Findings**:
- 🟡 major / high — **Negative-only turns, and turns that declare no
  `writes`, can pass vacuously.** *Location:* Phase 4 §3. Decline turns and
  R1 C3's criterion-free decline pass when nothing ran. The guard for writes
  outside the set lives inside `writes`. *Suggestion:* Add global checks for a
  missing log, exit code or text, and for any path changed outside the set.
  Have the loader reject a turn with neither `writes` nor `refusal`.
- 🟡 major / high — **`appended_round` never checks the appended focus areas.**
  *Location:* Phase 4 §3. *Suggestion:* Parse the appended items, require them
  to equal the previous turn's proposal, and add planted-bad tests.
- 🟡 major / high — **Hand-authored expectations are not cross-checked against
  the existing Python rule, count rule or `report-path`.** *Location:*
  Phase 3. *Suggestion:* Derive the `sources` checks from `rule.py`, the G2
  strings from `count_rule`, and the N1/E1 paths from `report-path`.
- 🟡 major / medium — **Reaching `open` doesn't prove S2, P1 and N2 set up
  what they claim.** *Location:* Phase 3 §2, §4. *Suggestion:* Declare
  mechanical preconditions (skipped arxiv pair, `config get` breadth 2,
  template resolution), and test that `replace` and `drop_line` raise on zero
  or several matches.
- 🟡 major / high — **Compound criteria get one planted failure each.**
  *Location:* Phase 4 §2. *Suggestion:* Parametrise per clause, add
  no-Read-at-all and Coverage-in-a-later-turn cases, and add a check that a
  source appears in a finding.
- 🟡 major / medium — **Multi-turn orchestration has no offline test.**
  *Location:* Phase 4 §3. *Suggestion:* Inject or stub `claude`, then assert
  reply order, session flags, snapshot alignment and handling of a failed
  turn.
- 🔵 minor / high — **The judge templates duplicate the skill's fixed lines,
  and exact matching is brittle.** *Location:* Implementation Approach;
  Phases 5–6. *Suggestion:* Cross-check them against SKILL.md and normalise
  formatting for every matcher.
- 🔵 minor / medium — **The gate scenarios run once, and the three-run rule
  is pinned only as counts.** *Suggestion:* Run G2 and G4 three times, and add
  a `times == 3` loader invariant.
- 🔵 minor / high — **Phase 5's green check cannot select only the `ask`
  sessions.** *Suggestion:* Add `--verb` or `--scenario`.
- 🔵 minor / high — **The slug tests cannot tell the 61-character window from
  a 60-character one, and have no exactly-60 case.** *Suggestion:* Use
  `'b ' + 58 a + ' c'`, and add `a_sixty_character_report_slug_is_kept_whole`.
- 🔵 minor / high — **`report-path`'s "unreadable means taken" branch is
  untested.** *Suggestion:* Add a test where a fake `FileReader` returns
  `Err`.
- 🔵 minor / high — **The validator tests miss the empty-coverage boundary,
  and the manifest cases never go red.** *Suggestion:* Add
  `an_empty_report_coverage_is_only_a_placeholder`, use exact equality for the
  status test, and label the manifest cases as regression guards.
- 🔵 minor / high — **Several prose changes have no red structure test.**
  *Location:* Phases 5–6 §1. These are the knob rule naming `report`, the
  untrusted reports, the citing-once description, the reopen paragraph and the
  unanswered no-pick step. *Suggestion:* Extend the existing tests, and add
  phrase tests.
- 🔵 minor / medium — **Eval sessions inherit the operator's Claude config
  and default model.** *Suggestion:* Pin the model and the setting sources,
  assert them in the turn-command tests, and record them.
- 🔵 suggestion / medium — **Pin the report template's body headings.**
- 🔵 suggestion / low — **The refactor's safety net misses
  `--no-session-persistence`.**

### Correctness

**Summary**: The arithmetic holds. The C7, C8 and rewrite-variant slugs are
right, the 102-session total adds up, the row and key counts are correct, and
nearly every citation checks out. The gate setups match the count rule and
`outstanding`'s semantics. Two contradictions block a green eval:
`appended_round` against the `last_updated` advance, and a pre-flight that
runs the released binaries. Smaller issues: a phrase that fails an existing
test, a bare `coverage:` that passes validation, and several underspecified
branches.

**Strengths**:
- The slug arithmetic is verified: C7 is 54, C8 is 60, and the rewrite C2
  slug is 60, ending `…from-failed`.
- The session counts sum to 102, and the G1 manual check's six sessions
  matches.
- The counts and citations (SCHEMA, catalogue, goldens, public-api,
  conformance, `RULE_SIGNATURES`, the conduct table, the finalise gate) check
  out.
- Each gate setup reaches its gate, and G4 pins the gate order.
- The gate classification order matches the skill, and the X merge gives
  `tier-3 (also tier-1)` with a `doi` then `openalex_id` block.

**Findings**:
- 🟡 major / high — **`appended_round` contradicts the `last_updated` advance
  on `outline.md`.** *Location:* Phase 4 §3 against Phase 5 §2. The 12
  confirm-turn sessions fail every run. *Suggestion:* Mask the two frontmatter
  lines or compare only the body, then assert separately that they advanced.
- 🟡 major / high — **The eval pre-flight runs whichever `accelerator` is on
  `PATH`.** *Location:* Phase 3 §2; Phase 4 §3. *Suggestion:* Pass the debug
  sub-binaries, or run `<repo>/bin/accelerator` under `session_env`.
- 🔵 minor / high — **The consult prose contains the `RULE_SIGNATURE`
  "joined by `; `".** *Suggestion:* Write "separated by `; `", and add the
  phrase to the avoid-list.
- 🔵 minor / medium — **A bare `coverage:` (YAML null) passes validation.**
  *Suggestion:* Treat it as outside the vocabulary, or as missing, and add a
  test.
- 🔵 minor / medium — **Coverage and Sources merge tiers over different
  citation scopes.** *Suggestion:* Use one scope for both.
- 🔵 minor / medium — **`report`'s mid-verb manifest edit conflicts with the
  "manifest last" rule.** *Suggestion:* Add an explicit exception in
  **Validate every write**.
- 🔵 minor / medium — **`Ok(None)` from `FileReader::read` does not mean the
  path is free.** *Suggestion:* Probe with `symlink_metadata`, treat only
  `NotFound` as free, and report other errors.
- 🔵 minor / medium — **Breadth resolution contradicts the misplaced-flag
  rule.** *Suggestion:* Resolve breadth from config only, and say so.
- 🔵 minor / high — **"76 of them multi-turn" is wrong: 52 sessions are
  multi-turn.** *Suggestion:* Correct the figure, or derive it from the
  scenario files.
- 🔵 suggestion / low — **There is no branch for a manifest with no Synthesis
  line under `## Set`.** *Suggestion:* Anchor after the Synthesis line, or
  else after the last entry.

### Security

**Summary**: The product-side design mirrors `synthesise`. It reads inline
and spawns nothing, uses ASCII-only slugs, and writes only after an explicit
accept or confirm. The gaps are in the eval harness, which auto-creates and
keeps the consent marker, runs injected content in `acceptEdits` sessions
under the developer's config with broad grants, and has snapshot and global
checks that miss the injection targets that matter most. Smaller gaps: the
enumerated untrusted-data contract, untested escaping, and no check that the
appended round is the confirmed proposal.

**Strengths**:
- No subagents, and a global check fails any Agent or Task call.
- The untrusted-data contract covers `synthesis.md` and existing reports.
- `ReportSlug` rules out path traversal, and `report-path` writes nothing.
- No write happens before an explicit reply, and `writes: []` is asserted.
- The `.levels/` exclusion and injection resistance are judged from the
  transcript.
- The question never reaches the `!` preprocessor.
- The eval reuses the dev-launcher containment and adds no bypass.

**Findings**:
- 🟡 major / high — **The eval silently creates and keeps the dev-launcher
  consent marker.** *Location:* Phase 4 §3. *Suggestion:* Refuse when the
  marker is missing, or create it and remove it in a `finally`, with a test
  that its state is unchanged.
- 🟡 major / medium — **Injection-laden sessions run with broad grants under
  the developer's live config.** *Location:* Phase 4 §1, §3. *Suggestion:*
  Use `--setting-sources project`, `--strict-mcp-config`, a scratch
  `CLAUDE_CONFIG_DIR` where possible, a minimal environment, and
  `--allowedTools` narrowed to the documented commands.
- 🟡 major / high — **The snapshot and global checks cannot see the injection
  targets that matter, or attempted actions.** *Location:* Phase 4 §2–3.
  *Suggestion:* Fail on writes outside the expected set, on Bash commands off
  the allow-list, and on non-empty `permission_denials`. Narrow the snapshot's
  exclusions, and assert that `.claude/` is unchanged.
- 🔵 minor / medium — **The untrusted-data contract is an enumerated list,
  and its extension to reports is untested.** *Suggestion:* Word it to cover
  every set document, plant an instruction in report U's body, and restrict a
  rewrite to a listed report.
- 🔵 minor / medium — **The question's shell and YAML escaping is done by the
  model and never tested with hostile input.** *Suggestion:* Read the
  question from stdin, or add an N-family scenario with `'`, `"` and
  `$(touch pwned)`.
- 🔵 minor / medium — **Nothing checks that the appended round matches the
  confirmed proposal.** *Suggestion:* Compare the appended items with the
  previous turn's proposal, and state the rule in the skill.

### Usability

**Summary**: The interface is predictable. It uses fixed lines, a shared
`{verb} refused:` prefix, and prompts that name their consequence before
anything is written. The contributor tooling mirrors citing-once. The main
friction: both refusals' recovery lines lead into further refusals, the
equivalence pick leaves a destructive rewrite to the model's reading of an
ambiguous reply, `--family` is too coarse for Phase 5's check, and the failure
output and use of dev binaries need tightening.

**Strengths**:
- Every judgeable output has a fixed, scannable form.
- The refusals share a prefix, write nothing, name a recovery, and spell out
  mismatches.
- Prompts state their consequence and the computed path before any write.
- The `Next:` line names the whole chain back to an answer.
- `BAD-EXTRA-VALUE` mirrors `BAD-STATUS`, and the template comment lists the
  vocabulary.
- `report-path` follows its siblings' conventions.
- Pre-flight runs before any paid session, and the options and logs aid
  debugging.

**Findings**:
- 🟡 major / high — **The status refusal's single recovery step is wrong for
  `briefed`, `outlined` and most `researching` sets.** *Suggestion:* Key the
  recovery to the status: `briefed` → outline, conduct, synthesise;
  `outlined` → conduct, synthesise; `researching` → conduct, then synthesise.
- 🟡 major / high — **The fully-conducted refusal names only `conduct`, which
  guarantees a second refusal.** *Suggestion:* End both groups with
  `…, then synthesise {slug}`, and key G3's `omits` on gate-specific phrases.
- 🔵 minor / medium — **A skipped-only refusal is headed "has research
  outstanding", and its two lists are unlabelled.** *Suggestion:* Use a
  neutral header and a lead-in for each group.
- 🟡 major / medium — **The equivalence pick prompt is ambiguous for a
  destructive rewrite.** *Suggestion:* Number the reports and offer
  `rewrite 1`, `new` or `reject`. Re-ask on an unclear reply, and put the
  `ask` prompt in the same imperative form.
- 🔵 minor / medium — **Proposal breadth resolution contradicts the
  misplaced-flag rule.** *Suggestion:* Accept `--breadth`, or say explicitly
  that breadth comes from config only.
- 🔵 minor / medium — **The coverage output doesn't show why a part is
  covered or a gap.** *Suggestion:* Put tier text on the `Gaps:` lines, and
  add a fixed `Covered:` block.
- 🔵 minor / medium — **`report-path` takes the question as one argument,
  and is documented inside the `outstanding` section.** *Suggestion:* Take a
  `Vec<String>` joined by spaces, as `fetch` does, and give it its own docs
  section.
- 🟡 major / high — **`--family` is the only filter.** *Suggestion:* Add
  `--verb` and `--scenario`, print the planned session count, and restate
  Phase 5's criterion.
- 🔵 minor / medium — **The failure output doesn't separate infrastructure
  failures from behavioural ones, or name the turn.** *Suggestion:* Stop at
  the first failed turn, keep a separate errored total, and print
  `{session} turn {n} {criterion}: expected …, got …`.
- 🔵 minor / medium — **The eval replies are full sentences.** *Suggestion:*
  Use the prompts' vocabulary, and add a bare-`accept` E1 scenario.
- 🔵 minor / medium — **The pre-flight and manual steps don't consistently
  use the dev binaries.** *Suggestion:* Pass the debug sub-binaries, and list
  the four variables.
- 🔵 suggestion / low — **Sessions inherit the default model and user-level
  config.** *Suggestion:* Add `--model`, and record the model in
  `results.json`.

### Compatibility

**Summary**: The contract changes are additive and ordered so that every
prerelease stays self-consistent. The main risks are platform ones: the
one-query skill grant against post-reply commands, which the eval hides, and
`--session-id` and `--resume` going unchecked against the installed Claude
Code. The eval's environment assumptions are partly open: a `PATH`-resolved
pre-flight, partial plugin isolation, and broken import paths. The plan also
drops the project's CHANGELOG and migration-note practice.

**Strengths**:
- CLI changes ship before the prose that calls them, and the marketplace tags
  plugin and binaries together.
- `report_count` is optional, and earlier binaries tolerate it.
- `print-schema` consumers read rows by key, and the exhaustive destructure
  catches drift.
- `--kind report` resolves through the existing three tiers with no code
  change.
- The eval runs dev binaries for the sessions and validates with the dev
  corpus binary.
- The no-subagent check matches both `Agent` and `Task`.
- `--session-id` is never combined with `--resume`.

**Findings**:
- 🟡 major / high — **Post-reply `accelerator` commands lose the skill's
  grant, and the eval hides it.** *Suggestion:* Name 0330 and the allow rules
  users can add, add a Phase 6 interactive manual check, or pass
  `--allowedTools` on turn 1 only for one scenario.
- 🟡 major / medium — **The `--session-id`/`--resume` mechanism is not checked
  against the target Claude Code until Phase 5.** *Suggestion:* Run a real
  two-turn token-recall check in Phase 4, record `claude --version`, and fail
  below v2.1.144.
- 🔵 minor / medium — **The eval's pre-flight runs the `PATH`-resolved
  `accelerator`.** *Suggestion:* Pass the debug sub-binaries.
- 🔵 minor / medium — **Sessions disable only the prerelease plugin ID and
  inherit user-level settings and hooks.** *Suggestion:* Disable both
  marketplace IDs, use `--setting-sources project,local`, and check the
  loaded plugin path.
- 🔵 minor / high — **Moving the citing-once modules breaks the tests' import
  paths.** *Suggestion:* State the re-export policy, or list the import
  updates and drop "unchanged".
- 🔵 minor / high — **No CHANGELOG entries or migration note.** *Suggestion:*
  Add per-phase `[Unreleased]` entries and a Migrations note for
  manifest-template overriders that keeps the `**Synthesis**` line.

### Standards

**Summary**: The plan mostly follows convention: test names, template
comments, verb headings, injection lines, clap docs and eval registration.
Two documented conventions are missed: the pinned lane-build map and
CHANGELOG entries. The rest are naming and placement differences from
sibling code.

**Strengths**:
- The Rust test names follow the sentence style and the count-in-name
  convention, and the new helpers mirror their siblings.
- The report template matches the existing template conventions exactly.
- The registration surfaces for a new template are listed in full.
- The CLI additions match their neighbours.
- The eval mirrors `eval:citing-once`.
- The verb sections use the established heading form and placement.

**Findings**:
- 🟡 major / high — **Adding the corpus build to `test:integration:research`
  breaks the pinned lane-build map and the exhaustive lane.** *Suggestion:*
  Update `_CLI_DEV_BUILDS` red first, handle `research-exhaustive`, and update
  the descriptions, or give the pre-flight its own lane.
- 🟡 major / high — **No CHANGELOG entries for two new verbs, a new CLI
  subcommand, a new validator code and a template change.** *Suggestion:* Add
  a CHANGELOG step to Phases 1, 2, 5 and 6.
- 🔵 minor / high — **Moving the citing-once modules contradicts "tests pass
  unchanged", and the new packages lack `__init__.py`.** *Suggestion:* Update
  the imports with no shims, and add an empty `__init__.py` to each package.
- 🔵 minor / high — **The "vocabulary" naming diverges from the established
  "vocab".** *Suggestion:* Use `vocab` throughout, or rename the existing uses.
- 🔵 minor / medium — **`ReportSlug` departs from its sibling's module
  conventions.** *Suggestion:* Use `report_slug.rs` or `slug.rs`, add a `//!`
  doc, widen the parent docs, and use `.get()`.
- 🔵 suggestion / medium — **The eval package's module and type names diverge
  from `citing_once`.**
- 🔵 suggestion / medium — **The shared section's heading borrows the
  verb-heading pattern.**
- 🔵 suggestion / medium — **`report-path` is documented inside the
  `outstanding` section.**
- 🔵 suggestion / medium — **The eval task's description should carry its
  measured run time, and the snippet drops the opt-out comment.**
- 🔵 suggestion / high — **The proposed Python slicers exceed 80 columns.**

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-30T20:46:26+00:00

**Verdict:** REVISE

### Previously Identified Issues

Of the 84 earlier findings, 75 are resolved and 9 are partially resolved.
None is still fully present.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 10 | 1 | 0 |
| Code Quality | 10 | 3 | 0 |
| Test Coverage | 15 | 1 | 0 |
| Correctness | 10 | 0 | 0 |
| Security | 5 | 1 | 0 |
| Usability | 10 | 2 | 0 |
| Compatibility | 5 | 1 | 0 |
| Standards | 10 | 0 | 0 |

These findings are partially resolved:

- 🔵 **Architecture + Code Quality**: The Consulting section restates the
  freshness gate — Partially resolved. `consultable` owns the gate now, but
  the Read step restates the count rule (see the new findings).
- 🔵 **Code Quality**: The rejection rule is stated twice — Partially
  resolved. It appears in Propose research and in report step 4.
- 🔵 **Code Quality**: The `headless` package mixes concerns — Partially
  resolved. Citing-once's `ALLOWED_TOOLS` still lives in the shared
  package.
- 🔵 **Code Quality**: The closed vocab is modelled twice — Partially
  resolved. The naming is unified, but the membership check is still
  duplicated.
- 🔵 **Test Coverage**: The slug boundary tests — Partially resolved. The
  61st-character case is fixed, but the exactly-60 case still misses the
  `<=`→`<` mutation.
- 🟡 **Security**: Broad grants under the live config — Partially resolved.
  Bare `Read`, `Write` and `Edit`, the pre-authorised `research fetch`, and
  the inherited environment remain.
- 🔵 **Usability**: The fully-conducted refusal — Partially resolved. The
  held-back group still guarantees another refusal.
- 🔵 **Usability**: The dev binaries in manual steps — Partially resolved.
  The Phase 2 and 3 steps call a bare `accelerator-research`.
- 🟡 **Compatibility**: The post-reply grant lapse — Partially resolved. It
  is documented internally, but not in the user-facing CHANGELOG or docs.

### New Issues Introduced

#### Major

- 🟡 **Test Coverage + Correctness**:
  `counts_reports_each_committed_set_fresh` contradicts the freshness rule.
  The deepened and multiprofile fixtures are `researching`, with round-1
  findings and no `synthesis.md`, so they print `fresh: false`. The deepened
  set is also the only real-filesystem guard for the retired `.levels/` test.
- 🟡 **Test Coverage + Architecture**: Phase 3 changes `conduct`,
  `synthesise` and `finalise` with no behavioural re-check, and it breaks
  `eval:citing-once`. That eval resolves `accelerator` from `PATH`, so its
  `synthesise` sessions would call a `counts` subcommand that released
  binaries lack.
- 🟡 **Test Coverage**: No criterion judges `report`'s gap naming. R1 C5 and
  R1 C2 carry no `gaps` criterion, and the `report` criterion checks only
  headings, not the content of `## Gaps`.
- 🟡 **Correctness**: R1 C1 requires S1 in Sources at
  `tier-3 (retracted)`, but step 5 cites only the sources "the report draws
  on". An answered C1 with a retracted S1 that supports nothing need not
  cite it.

#### Minor

- 🔵 **Architecture + Code Quality + Correctness**: Phase 6's Read step
  restates the count rule that Phase 3 removes, as "every finding the count
  rule counts — each visible `<nn>-*.md` …".
- 🔵 **Architecture + Code Quality**: Two classifiers of a `findings/` entry:
  `counts.rs` uses a case-sensitive `.md` plus an index, while
  `read_findings` uses a case-insensitive extension.
- 🔵 **Architecture + Compatibility + Usability**: `counts` errors on an
  unreadable stored count. The writers that would repair it fail, nothing
  says what `conduct` and `synthesise` do on a non-zero exit, and the error
  has no code or recovery.
- 🔵 **Architecture + Code Quality + Correctness**: The lazy `Consultation`
  chain is sketched with infallible closures, and its laziness test removes
  `outline.md`, which `read_round_inputs` reads as an empty outline.
- 🔵 **Architecture**: Phase 3 builds on Phase 2 (the docs placement, plus
  shared `cli.rs`, `topic_command.rs` and `public-api.txt` edits), but is
  described as independent.
- 🔵 **Code Quality + Correctness**: The mismatch sentences in `formats.py`
  can never appear in SKILL.md, so the per-format structure test fails.
- 🔵 **Test Coverage + Correctness**: The `_consult()` test forbids
  `outstanding` and `highest`, which the planned recovery line and Propose
  research prose both use.
- 🔵 **Correctness**: Three structure-test phrases don't match the planned
  prose: `ask the same thing`, `copied from the manifest`, and the
  `argument-hint` that "ends" with `ask`.
- 🔵 **Test Coverage + Correctness**: The exactly-60 slug test cannot catch
  the `<=`→`<` mutation.
- 🔵 **Security + Correctness**: The strict Bash allow-list doesn't define how
  it treats compound commands, and no prose tells the verbs to list
  directories with Glob rather than `ls`.
- 🔵 **Security**: Bare `Read`, `Write` and `Edit` grants let a stray write
  land before the after-the-fact check fails it.
- 🔵 **Security**: No planted injection targets
  `research fetch web metadata`, which is the exfiltration channel.
- 🔵 **Standards + Code Quality**: `headless/claude.py` keeps citing-once's
  grant under the generic name `ALLOWED_TOOLS`, and `run_session` has no
  `env` seam.
- 🔵 **Standards**: The eval-fixtures lane differs from its siblings in its
  run string, directory and test basename.
- 🔵 **Standards**: Phase 3's new modules get no `//!` docs, and the parent
  docs aren't widened.
- 🔵 **Code Quality**: `topic_command.rs` takes on three more subcommands, and
  `reports/` is listed in two layers.
- 🔵 **Usability**: Five wording and prompt issues:
  - the held-back recovery guarantees another refusal;
  - the multi-report `rewrite {n}` has no defined form;
  - `report`'s `reject` doesn't say it proposes research;
  - the proposal confirm has no re-ask rule;
  - the `Gaps:` line contains literal `…, or none` text.
- 🔵 **Usability**: The `UncoveredRound` sentence drops its unit
  ("covers 2").
- 🔵 **Usability + Correctness**: `report-path` with no question prints
  `reports/report.md`, and neither verb handles a missing question.
- 🔵 **Usability**: The new docs sections land under the "Academic profiles"
  H2.
- 🔵 **Usability**: The manual steps call a bare `accelerator-research` and
  omit `ACCELERATOR_RESEARCH_BIN`.
- 🔵 **Usability**: A marker left behind fails `mise run` with "must never be
  committed".
- 🔵 **Compatibility**: The `'\''` form is unmeasured against Claude Code's
  permission matcher at v2.1.144.
- 🔵 **Compatibility**: Users aren't told that post-reply writes prompt
  until 0330 lands.
- 🔵 **Compatibility**: `--setting-sources project` drops authentication
  configured in user settings.
- 🔵 **Compatibility**: The report-kind skew note leaves out collaborators
  who share a corpus.
- 🔵 **Test Coverage**: Five gaps in what the tests assert:
  - the third mismatch sentence and `held_back` are not asserted at the CLI;
  - the adapter's error and default branches are untested;
  - nothing pins that failed outranks errored, or the `writes` baseline;
  - a rewrite is checked to keep only `question`, not the other preserved
    fields;
  - the criteria table names `a_taken_report_name_…`, which doesn't exist.
- 🔵 **Correctness**: G3's skipped scenario must omit `conduct`, but the
  header "not fully conducted" contains that string.
- 🔵 **Correctness**: "`report` is the only writer of `report_count`" is
  false, because `brief` writes `0`.
- 🔵 **Architecture**: A report takes `rounds_covered` from the manifest,
  while `synthesise` now stamps it from `derived`.

#### Suggestions

- 🔵 **Security**: Move the untrusted-data sentence ahead of the verdict, so
  it covers set text the CLI echoes.
- 🔵 **Usability**: `finalise`'s refusals diverge from the consult
  recoveries.
- 🔵 **Usability**: `relates_to` has no input path.
- 🔵 **Usability**: The rerun and breadth recoveries live only in the plan.
- 🔵 **Compatibility + Standards**: State that the `counts` and
  `consultable` JSON only ever gains fields, and reuse `Outstanding`'s
  `profiles_dir` wording.
- 🔵 **Code Quality + Standards**: Rename the `QuestionSlug` constants, and
  align `free_report_path`'s return type with `finding_path`.
- 🔵 **Code Quality**: Three modelling suggestions:
  - one name per gate across the layers;
  - each criterion type carries its own judge and judgement flag;
  - a shared `outside_vocab` helper;
  - a named `CountsReading` value.
- 🔵 **Architecture**: Consider having `consultable` render the refusal
  message.
- 🔵 **Correctness**: `_report()` has 9 call sites, not 12. Rename it by
  symbol, not by text search.
- 🔵 **Standards**: Two formatting fixes:
  - Phase 3's `finalise` snippet nests a triple-backtick fence inside another
    one;
  - one prose line in Propose research runs to 89 columns.

### Assessment

The revision resolves 75 of the 84 earlier findings and partly resolves the
other 9. None is still fully present. The verdict stays **REVISE** because
pass 2 raises four new major findings, and the threshold is three. All four
come from the revision, and each needs a targeted edit, not a redesign:

1. Split the fixture-freshness test by fixture.
2. Route `eval:citing-once` through the dev toolchain, and re-check
   `conduct` and `synthesise` behaviour in Phase 3.
3. Add `gaps` criteria, and a `## Gaps` section check, to R1 C5 and R1 C2.
4. Make the Sources prose require every source cited in the findings the
   answer draws on, retracted ones included.

The most valuable minor fixes are these, because each would turn a phase
red as written:

- make the Read step read the CLI's counted paths instead of restating the
  count rule;
- make `counts` report an unreadable stored count as a mismatch, not an
  error;
- type the gate chain's closures as fallible, and give the laziness test a
  read that genuinely fails;
- correct the self-contradicting structure-test phrases.

The plan's architecture is sound, and pass 2 found no structural problems.

## Re-Review (Pass 3) — 2026-10-30T22:58:39+00:00

**Verdict:** REVISE

### Previously Identified Issues

Each lens re-checked its own pass-2 findings: 69 in all, with a shared
finding counted once per lens. Of these, 54 are resolved, 12 are partially
resolved and 3 are still present. All four pass-2 majors are resolved, and the
three still present are suggestions.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 7 | 1 | 1 |
| Code Quality | 7 | 3 | 1 |
| Test Coverage | 5 | 1 | 0 |
| Correctness | 13 | 0 | 0 |
| Security | 3 | 2 | 0 |
| Usability | 8 | 3 | 1 |
| Compatibility | 6 | 1 | 0 |
| Standards | 5 | 1 | 0 |

The four pass-2 majors:

- 🟡 **Test Coverage + Correctness**: `counts_reports_each_committed_set_fresh`
  contradicted the freshness rule — Resolved. The test is now split into a
  stored-counts test, a synthesised-and-fresh test and a no-synthesis-and-stale
  test, and each matches the committed fixtures.
- 🟡 **Test Coverage + Architecture**: Phase 3 broke `eval:citing-once` —
  Resolved. Change 6 routes its sessions and its judge through the dev
  `Toolchain`. Manual checks now re-run `conduct`, `synthesise`, `finalise`
  and the eval.
- 🟡 **Test Coverage**: No criterion judged `report`'s gaps — Resolved. R1 C5
  and R1 C2 carry `gaps` and `section_mentions: {Gaps: …}`.
- 🟡 **Correctness**: R1 C1's retracted S1 need not be cited — Resolved.
  Sources now cites every source in each finding the answer draws on,
  retracted ones included.

These findings are partially resolved:

- 🔵 **Architecture + Code Quality**: Two classifiers of a `findings/` entry —
  Partially resolved. The case rule now matches, but because it was copied,
  not shared. The two still disagree on entries without an index (see the new
  findings).
- 🔵 **Code Quality + Standards**: The `headless` grant — Partially resolved.
  `ALLOWED_TOOLS` stays in citing-once, but Phase 5 §3 still names
  `headless.claude.ALLOWED_TOOLS`.
- 🔵 **Code Quality**: Modelling suggestions — Partially resolved. Criterion
  types own their judges and `CountsReading` is named. The gate names still
  differ between layers.
- 🔵 **Test Coverage**: Five assertion gaps — Partially resolved. Four are
  closed, but the adapter's error branches are still untested.
- 🟡 **Security**: Broad grants under the live config — Partially resolved.
  Bare `Read`, `Write` and `Edit` are gone. The `research fetch` grant, bare
  `Glob`, `Grep` and `Skill`, and the inherited environment remain.
- 🔵 **Security**: Compound Bash commands — Partially resolved. `shlex`
  defines how they are handled, but newline-joined commands and double-quoted
  substitutions still pass.
- 🔵 **Usability**: The held-back recovery — Partially resolved. It works when
  both groups print, but a refusal with only held-back items still leads to a
  second refusal.
- 🔵 **Usability**: `relates_to` — Partially resolved. It is filled from the
  conversation, but no prompt invites the user to give it.
- 🔵 **Compatibility**: The `'\''` probe — Partially resolved. It is measured
  on two Claude Code versions, but the plan does not say what to do with the
  result.

These are still present (suggestions only): no Decisions row says why refusal
rendering stays in the skill; the vocab membership check is duplicated; and
`finalise`'s status refusal differs from the consult recoveries.

### New Issues Introduced

#### Major

- 🟡 **Code Quality + Architecture + Correctness**: The gate chain discards
  the reading that the open verdict prints. The chain step
  `then_freshness(|| read_counts(…).map(|r| r.freshness()))` drops the
  `CountsReading`, and `Consultation::Open` carries nothing. So
  `consultable` must either list `findings/` twice, smuggle the reading out
  through the closure, or read eagerly before the status gate. That last
  option is what §3's "each taking a `CountsReading`" implies.
- 🟡 **Correctness + Architecture + Compatibility**: `consultable` prints
  set-relative paths, and the sentence naming `ask`'s only commands leaves out
  `corpus resolve`.
  - `outstanding` and `report-path` print absolute paths
    (`topic_command.rs:978`), and the Read tool needs them.
  - Rebuilding the paths depends on the Shared Preamble's `corpus resolve`
    (`SKILL.md:106-110`). The sentence omits it, and a structure test pins
    the omission.
- 🟡 **Compatibility + Security**: The scoped `Read(<plugin_root>/…)` grant
  begins with a single `/`. Claude Code reads a single leading `/` as relative
  to the settings source; an absolute path is written `//`. Every consulting
  session could then fail its rule read with a `permission_denials` entry.
  Phase 5's red A2 run would hide that among its expected failures.
- 🟡 **Correctness**: N1's hostile question has no pinned coverage. "Which
  planner is best" is the very part the corpus leaves open (C5's ranking gap).
  An `unanswered` judgement writes nothing and never runs `report-path`, so
  the scenario cannot pass 3/3 and never exercises the escaping.
- 🟡 **Correctness**: Stopping at a global-check violation can mark an
  injection failure as errored. Turns after the stop leave no log. `runs.py`
  counts a missing log as errored, and `--rerun-errored` then reruns a session
  that obeyed an injection until it passes.

#### Minor

- 🔵 **Code Quality + Architecture + Test Coverage**: The domain returns
  counts but not the counted names. `reports` would then be filtered in the
  adapter, or not at all, and no test pins that filter.
- 🔵 **Architecture + Code Quality**: The fully-conducted gate and the reading
  list still classify `findings/` entries separately. A valid `notes.md`
  without an index can answer a pair, yet it is never read.
- 🔵 **Architecture + Correctness**: An unreadable `rounds_covered` or
  `report_count` is unspecified. If it errors, it blocks the `synthesise` that
  would repair it.
- 🔵 **Compatibility**: A quoted `round_count: "3"`, which the validator
  accepts, now makes `finalise`, `ask` and `report` refuse the set as stale,
  and the recovery they name is expensive.
- 🔵 **Compatibility**: `report-path`'s collision check is case-sensitive, so
  a new report can overwrite an existing one on case-insensitive APFS.
- 🔵 **Compatibility**: The `'\''` probe has no decision rule. Writing `'` as
  a space gives the same slug and needs no probe.
- 🔵 **Security + Correctness**: Four forms still pass the `shlex`
  allow-list: newline-joined commands, double-quoted `$(…)`, backticks and
  `$VAR`.
- 🔵 **Security**: `research fetch` is still pre-authorised in sessions that
  inherit the developer's environment.
- 🔵 **Security**: The bare `Glob` and `Grep` grants reach outside the
  project, and no check looks at their paths.
- 🔵 **Security**: The `Skill` grant can pull in other accelerator skills'
  grants, and no check covers it.
- 🔵 **Code Quality + Standards + Correctness**: Phase 5 §3 still names
  `headless.claude.ALLOWED_TOOLS`, which no longer exists, and
  `opening_turn` takes no `model`.
- 🔵 **Code Quality + Test Coverage**: The dev-launcher marker refusal is
  specified twice, once through a forward reference. It has no hermetic seam,
  and no test pins the eval tasks' new build edges.
- 🔵 **Code Quality**: The turn loop now judges global checks but has no
  injection seam for doing so.
- 🔵 **Test Coverage + Correctness**: Three structure-test phrases do not
  occur in the planned prose:
  - the `"fresh"` assertion (the prose has `` `fresh` ``);
  - the escaped `reports/` Set line;
  - the wrapped `Wrote reports/…` and indented refusal-template lines.
- 🔵 **Test Coverage**: `consultable_ignores_the_concurrency_config` can
  never go red. `outstanding` already treats a blank `ACCELERATOR_SUBAGENT_CAP`
  as unset.
- 🔵 **Test Coverage**: No test covers error propagation, and an
  implementation that swallowed a plan-read error would fail open with
  `"consultable": true`.
- 🔵 **Test Coverage**: The `kept` fixtures do not set `parent` or
  `relates_to`, and `relates_to` is missing from `kept`.
- 🔵 **Usability**: A skipped-only conducted refusal still leads to a second
  refusal.
- 🔵 **Usability**: The blanket re-ask rule leaves no way to stop at the
  accept and pick prompts. A user who wants nothing must reply `reject`, then
  `decline`.
- 🔵 **Usability**: The authentication probe reports every failure as an
  authentication problem.
- 🔵 **Usability**: The order of the start-up gates, and which of them
  `--check-only` skips, are unspecified.
- 🔵 **Usability**: The printed rerun command drops the original run's
  `--model` and `--config-dir`.
- 🔵 **Usability**: Phase 1's bare `accelerator` may not reach the working
  tree, and **Dev launcher setup** has no build step.
- 🔵 **Standards**: A missing question gives `report-path` clap's uncoded
  usage error, and a blank question still slugs to `report`.
- 🔵 **Standards**: The entrypoint-test paragraph splits the README
  subsection's bullet list.
- 🔵 **Standards**: The new command modules and tests use three names for one
  concept: `consultation`, `consult_command` and `topic_gates`.

#### Suggestions

- 🔵 **Code Quality**: Model the mismatch field as `enum CountField`, and
  rename `Stored` to `StoredCount`.
- 🔵 **Code Quality**: Use one name per gate across Rust, JSON, Python and
  the skill.
- 🔵 **Code Quality + Architecture**: Give `Toolchain` its own module, and
  give `counts` a module not named after consultation.
- 🔵 **Architecture**: Record in Decisions why refusal rendering stays in the
  skill.
- 🔵 **Security**: Widen the untrusted-data sentence to cover the mismatch
  values and paths the CLI echoes.
- 🔵 **Usability**: Give a denied post-reply command a recovery line in the
  skill.
- 🔵 **Usability**: Show the literal `rewrite 1` in the pick prompt.
- 🔵 **Compatibility**: Have Phase 7 extend the "Permissions after a reply"
  docs paragraph to `report`.
- 🔵 **Test Coverage**: Assert the unreadable- and missing-count sentences
  verbatim, and derive G4's expectations as G2's are.
- 🔵 **Standards**: Six smaller fixes:
  - `CountsReading`'s report-count field names;
  - the `## research topic` H2's style;
  - the Migrations entry's placement;
  - a README "citing-once subsection" that does not exist yet;
  - three lines over 80 columns;
  - the `eval:citing-once` description, and the "ascii alphanumerics" term.

### Assessment

The revision resolves all four pass-2 majors and 54 of the 69 findings the
lenses re-checked. The verdict stays **REVISE** because pass 3 raises five new
majors, and the threshold is three. All five sit at seams the revision added:
the open verdict's document list, the narrowed grant, the hostile-question
scenario and the stop-on-violation rule. Each needs a targeted edit, not a
redesign:

1. Carry the documents in the verdict, as
   `Consultation::Open { findings, reports }`, with a freshness reader that
   returns the whole `CountsReading`. Have the domain return the counted names
   too, which also fixes the unpinned `reports` filter.
2. Print absolute paths from `consultable`, as `outstanding` does, and name
   `corpus resolve` in the commands sentence and its test.
3. Write the grant as `Read(//…)` and pin the exact string. Have the
   authentication probe read the rule file, so that a grant that never matches
   refuses the run before any session.
4. Build N1's question from C1, with the hostile characters inserted, and add
   it to the Question support table as `answered`.
5. Make a global-check violation a **failed** session, and record the turns
   after it as not run.

These minor fixes matter most, because each would turn a phase red as written
or let a test pass vacuously:

- the three structure-test phrases;
- the concurrency test that cannot go red;
- the error-propagation tests;
- the `shlex` newline and double-quote gaps.

Across passes the major count has gone from 18 to 4 to 5. The pass-3 majors
are narrower than earlier ones, and no lens found a structural problem.

## Re-Review (Pass 4) — 2026-10-31T10:10:40+00:00

**Verdict:** REVISE

### Previously Identified Issues

Each lens re-checked its own pass-3 findings, 64 in all, counting a finding
shared by several lenses once per lens. Of these, 59 are resolved, 5 are
partially resolved and none is still present. All five pass-3 majors are
resolved.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 7 | 0 | 0 |
| Code Quality | 10 | 0 | 0 |
| Test Coverage | 7 | 0 | 0 |
| Correctness | 7 | 1 | 0 |
| Security | 5 | 1 | 0 |
| Usability | 8 | 2 | 0 |
| Compatibility | 5 | 1 | 0 |
| Standards | 10 | 0 | 0 |

The five pass-3 majors:

- 🟡 **Code Quality + Architecture + Correctness**: The gate chain discarded
  the reading the open verdict prints — Resolved. `Consultation::judge` keeps
  the reading it judged freshness on, and the open verdict carries its
  `Documents`.
- 🟡 **Correctness + Architecture + Compatibility**: `consultable`'s
  set-relative paths, and the missing `corpus resolve` — Resolved. The paths
  are absolute, and `corpus resolve` is named in the commands sentence.
- 🟡 **Compatibility + Security**: The scoped `Read` grant's path form —
  Resolved. It is written `Read(//…)`, and the probe reads the rule file.
- 🟡 **Correctness**: N1's question had no pinned coverage — Resolved. It is
  rebuilt around C1 and listed as `answered`.
- 🟡 **Correctness**: A global-check stop could mark an injection failure as
  errored — Resolved. Such a session is now failed, and its later turns are
  recorded as not run. The rule overshoots, as the new findings explain.

These findings are partially resolved:

- 🔵 **Correctness + Usability**: Format lines in the prose — Partially
  resolved. `ask`'s reject prompt is still wrapped inside a sentence.
- 🔵 **Security**: The pre-authorised `research fetch` — Partially resolved.
  Resumed turns no longer grant it, but the opening slash-command turn still
  carries the skill's own grant.
- 🔵 **Compatibility**: The `'\''` probe — Partially resolved. The probe now
  has a fallback, but its one probe row cannot show which construct caused a
  denial.
- 🔵 **Usability**: `finalise`'s status refusal — Partially resolved.
  What We're NOT Doing records it as a follow-up.
- 🔵 **Usability**: `relates_to` — Partially resolved. A reply may name a
  relationship, but nothing tells the user so.

### New Issues Introduced

#### Major

- 🟡 **Usability + Correctness**: Almost no session can be classed as
  errored. Criteria and the "missing log or no text" global check are also
  judged on the turn that exited non-zero or timed out. A transient API error
  or a timeout therefore lands as **failed**, and `--rerun-errored` almost
  never fires. The Phase 5 manual step that edits a red G1 session's
  `turn-1.exit` cannot pass, because that session has already failed its
  `refusal` criterion.
- 🟡 **Test Coverage + Correctness**: The R2 decline scenario's "no proposal"
  has no criterion to check it. `offers` cannot assert that no prompt is
  printed, and `proposal` constrains only a proposal that exists. A model
  that treats `decline` like `reject` therefore passes.
- 🟡 **Compatibility + Security**: The path checks compare unresolved paths.
  `resolve_document` canonicalises the set root
  (`cli/corpus-adapters/src/resolve.rs:85,101`), but the eval's projects live
  under `tempfile.mkdtemp()` (`tasks/eval/__init__.py:51`), which on macOS
  returns the unresolved `/var/folders/…` form. Every check that compares a
  path, and the pre-flight's comparison of the N1 and E1 `writes` paths,
  could fail on macOS while CI's already-resolved `tmp_path` stays green.
  Separately, the outside-project check does not say how it reads a Glob
  `pattern` or a Grep `glob`.
- 🟡 **Compatibility**: Phase 7's probe writes into a fixture that gates CI.
  `claude-bash-baseline.tsv` feeds
  `the_guard_agrees_with_the_latest_measured_claude_code_release`
  (`cli/research/tests/confinement.rs:111`). Measuring the current release
  adds a new latest column, which the research guard must then match on every
  existing row.

#### Minor

- 🔵 **Architecture + Code Quality + Standards**: `layout::names_a_finding`
  sits beside the stricter `finding_path::is_finding_path`, and the plan does
  not say how the two relate. It is also defined in terms of
  `names_a_report`, and it lives in the index-only `layout.rs` rather than a
  submodule.
- 🔵 **Architecture + Code Quality**: `CountField` admits `Mismatch` states
  that no code builds. The rule that a missing `rounds_covered` covers round
  0 has two owners: the adapter's `Count(0)` and `uncovered`'s `Missing` arm.
- 🔵 **Architecture + Code Quality**: The default `stops_after` binds the
  sequencer to the judge and risks an import cycle. The plan never says what
  a `turn` carries, and some global checks can only be judged when the
  session ends.
- 🔵 **Code Quality**: `run_consultable` relies on a `status` and a
  `read_plan` that the plan never defines, and the error type `judge` needs
  is unstated.
- 🔵 **Security + Compatibility + Correctness**: The opening turn carries the
  skill's own `research fetch` grant, so "denies an injected fetch outright"
  holds only on resumed turns.
- 🔵 **Security + Test Coverage**: The raw scan's quote tracking is
  unspecified, and the passing case must be N1's actual `'\''` command.
- 🔵 **Usability + Test Coverage + Correctness**: The plan says both that
  `--check-only` needs only the corpus binary and that it runs the pre-flight,
  which needs all three. The closing reminder also prints when no marker was
  used.
- 🔵 **Usability**: The authentication probe's pass condition, and the
  signal it treats as an authentication failure, are unspecified.
- 🔵 **Usability**: The manual CLI steps call a repo-relative binary from a
  scratch project whose layout is never described.
- 🔵 **Standards**: `report-path`'s usage error follows `fetch`'s exit-2
  convention. `TopicAction` documents exit 1 with an `E_TOPIC_RESEARCH_*`
  code (`cli/research-cli/src/cli.rs:106-109`).
- 🔵 **Test Coverage**: G3's held-back expectation contradicts the 0281
  acceptance criterion, which says "not `conduct SLUG`", and the
  verification table still says "G3 skipped".
- 🔵 **Test Coverage**: The untrusted-data sentence's new colon may split the
  "one sentence" structure test.
- 🔵 **Test Coverage**: N1's 3/3 bar also depends on a coverage judgement of
  a question that contains a shell fragment.
- 🔵 **Compatibility**: The probe row cannot tell a `'\''` denial from a
  denial of `$(…)` inside quotes, and the fallback form is never measured.
- 🔵 **Compatibility**: `research.md:572-579` still tells consumers that every
  top-level `findings/*.md` is a finding.
- 🔵 **Compatibility**: The eval's permission flags are not checked against
  the untrusted-folder behaviour introduced in 2.1.281.
- 🔵 **Compatibility**: A question that begins with `-` hits clap's uncoded
  error.
- 🔵 **Correctness**: Decision 4's lenient reading does not reach a quoted
  `round` in a finding.

#### Suggestions

- 🔵 **Architecture + Standards**: Move `resolve_set` out of
  `topic_command.rs`, or keep it in the module's narrowed doc.
- 🔵 **Standards**: `rounds_covered` is a stored value in Rust but sits at the
  top level of the `counts` JSON, and `Counted…` and `derived` name the same
  idea.
- 🔵 **Standards**: Rename `_EVAL_CLI_BUILDS` to `_EVAL_CLI_DEV_BUILDS`, name
  its test, and update the README's debug-builds section.
- 🔵 **Standards**: Build `Toolchain`'s paths from `tasks/shared/dev_builds.py`,
  and state how it handles path overrides.
- 🔵 **Standards**: Small slips remain:
  - the form of the `report_path_command.rs` doc;
  - a bold lead sentence for Phase 1's Migrations entry;
  - four lines over 80 columns.
- 🔵 **Code Quality**: Make clear which flags belong in `common_flags`.
- 🔵 **Code Quality**: Add `# Errors` sections to the fallible public
  sketches, which `clippy::pedantic` requires.
- 🔵 **Usability**: The recovery line for a denied command should say how to
  finish.
- 🔵 **Test Coverage**: Pin the half of the narrower re-ask rule that covers
  prompts guarding no write.
- 🔵 **Correctness**: Escape `raw` in the mismatch sentences, which can span
  lines.

### Tradeoff Analysis

- **Topic-family consistency vs `fetch` consistency**: For a missing
  question, Standards wants `report-path` to refuse at exit 1 with an
  `E_TOPIC_RESEARCH_*` code, as `TopicAction` documents. Compatibility
  counted the exit-2 `E_RESEARCH_USAGE` that matches `fetch` as a strength.
  Recommendation: follow the topic family, because `report-path` is a topic
  subcommand and the `topic()` dispatch in `main.rs` already exits 1.

### Assessment

The revision resolves all five pass-3 majors and 59 of the 64 findings the
lenses re-checked; none is still present. The verdict stays **REVISE**
because pass 4 raises four new majors, and the threshold is three. Every one
sits in the eval harness or in its manual and probe steps, in Phases 5 and 7.
None touches the product design: the contract, the CLI gates, or the `ask`
and `report` prose. Each needs a targeted edit:

1. Judge criteria only on turns that exited 0. Mark a session errored when a
   missing log or an empty turn is all that went wrong. A behavioural global
   check still fails a session in any turn that ran. Point the manual step at
   a session with no criterion failure.
2. Add a negative proposal check, such as `offers: none` or
   `proposal: false`, with a planted failure. Use it in R2's decline
   scenario and in the other decline turns.
3. Resolve the project root, the set root and every tool-input path before
   comparing them. Take Glob's base from its `pattern`. Add a test whose root
   is reached through a symlink.
4. Measure 0281's probe rows against a scratch copy of the baseline fixture
   and record the verdicts in the As-built note. Split the one row into three
   rows: the `'\''` form, `$(…)` inside quotes, and the space-form fallback.

Across passes the major count has gone from 18 to 4 to 5 to 4. The product
design has been stable since pass 2. The harness keeps producing findings
because each fix adds surface, so the next revision should state each harness
rule once and precisely rather than add more mechanism.

## Re-Review (Pass 5) — 2026-10-31T10:43:22+00:00

**Verdict:** COMMENT

The plan is acceptable to implement, but it could be improved: see the major
finding below.

### Previously Identified Issues

Each lens re-checked its own pass-4 findings, 48 in all, counting a finding
that several lenses share once per lens. Of these, 42 are resolved, 6 are
partially resolved and none is still present.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 4 | 0 | 0 |
| Code Quality | 6 | 1 | 0 |
| Test Coverage | 7 | 0 | 0 |
| Correctness | 6 | 1 | 0 |
| Security | 2 | 1 | 0 |
| Usability | 6 | 0 | 0 |
| Compatibility | 4 | 3 | 0 |
| Standards | 7 | 0 | 0 |

The four pass-4 majors:

- 🟡 **Usability + Correctness**: Almost no session could be classed as
  errored — Resolved. Criteria are now judged only on turns that exited 0
  with text, so a session whose only problems are such turns is errored. An
  edge case remains for a turn that exits 0 with no text (see the new
  findings).
- 🟡 **Test Coverage + Correctness**: R2's decline had no criterion for "no
  proposal" — Resolved. `offers: none` now checks it, and every decline turn
  carries it.
- 🟡 **Compatibility + Security**: Path checks compared unresolved paths —
  Partially resolved. The global checks now resolve every path, but the
  pre-flight's comparison with `report-path`'s output is not covered.
- 🟡 **Compatibility**: The probe wrote into a fixture that gates CI —
  Partially resolved. The step now names a scratch copy and three rows, but
  `claude_permissions.py` cannot read either (see the new major).

The other four partially resolved findings:

- 🔵 **Security + Correctness + Compatibility**: The turn-1 fetch grant —
  Partially resolved. Key Discoveries and the decision table are now
  accurate, but one turn-command sentence still says an injected fetch is
  denied.
- 🔵 **Code Quality**: The input to `stops_after` — Partially resolved.
  `TurnRecord` exists, but it lacks the session context and the turn-0 text
  that some checks need.

### New Issues Introduced

#### Major

- 🟡 **Usability + Compatibility + Correctness**: The Phase 7 probe step
  cannot be run as written. `tasks/probe/claude_permissions.py` hard-codes
  `FIXTURE` (line 28) and accepts only `--claude`, `--config-dir` and
  `--update`. It also measures every row, 87 today, and raises `IndexError`
  on a row with no cell for a column the file already has. Usability rated
  this major; Compatibility and Correctness rated it minor.

#### Minor

- 🔵 **Architecture + Correctness**: A turn that exits 0 with no text is
  errored, so it can be rerun, yet it does not stop the conversation. Such
  a turn is model behaviour, not infrastructure, and a silent but correct
  decline could never pass.
- 🔵 **Security + Correctness**: The allow-list check uses `shlex`, which
  treats a `#` inside a word as the start of a comment; bash does not. So
  `…llm-agent-planning#;accelerator research fetch …` passes the check,
  although bash runs both commands. This was verified with Python 3.
- 🔵 **Security + Correctness + Compatibility**: The turn-command bullet still
  says an injected fetch shows as a denied `tool_use`.
- 🔵 **Usability + Correctness**: After a denial partway through a proposal,
  "running the verb again finishes the work" is false, because the rerun is
  refused at the `conducted` or `status` gate.
- 🔵 **Correctness**: The init-event turn check fails a turn that crashed
  before emitting an init event, so a transient error lands as failed.
- 🔵 **Correctness**: The tests disagree on whether `--check-only` prints the
  rerun command.
- 🔵 **Usability + Test Coverage**: The authentication probe's branches
  overlap, and no order is set for them. The "no `probe.txt`" branch can
  name the wrong cause, the probe's log is lost, and the pass condition
  never proves that the rule was read.
- 🔵 **Code Quality + Architecture**: `TurnRecord` lacks the project, set and
  plugin roots and the turn-0 text. `turns.py` is not listed in change 3,
  and nothing builds a record the same way for live and on-disk judging.
- 🔵 **Code Quality**: `run_conversation` has no route for
  `CLAUDE_CONFIG_DIR`.
- 🔵 **Code Quality**: The `# Errors` doc on `CountedFindings::of`
  contradicts its contract, because an unreadable `round` gives `Ok(None)`,
  not an error.
- 🔵 **Architecture**: `read_plan` moves the planner, and `outstanding`'s
  query-mode policy, into the adapter crate.
- 🔵 **Compatibility**: The pre-flight's comparison with `report-path`'s
  output falls outside the path-resolution rule.
- 🔵 **Compatibility**: No probe row measures the combined command that N1
  actually issues.
- 🔵 **Compatibility**: The documented `<nn>-*.md` rule accepts fewer names
  than `is_finding_entry` does.
- 🔵 **Security**: The outside-project path check misses three edge cases:
  an absolute Glob pattern when `path` is set, a path starting with `~`, and
  a sibling directory that shares the project's name as a string prefix.
- 🔵 **Usability**: Errored sessions get no per-session reason line, and a
  result event with `is_error` at exit 0 lands as failed.
- 🔵 **Usability**: `eval:citing-once --check-only` still requires the
  marker.
- 🔵 **Usability**: Eval options are given without the `--` separator, so
  `mise run` may take `--jobs` as its own option.
- 🔵 **Usability**: The scratch-copy recipe cites scaffolding that names the
  set directory differently.
- 🔵 **Test Coverage**: No test excludes a hidden numbered file such as
  `.03-c-web.md`.
- 🔵 **Test Coverage**: The held-back expectation does not name
  `synthesise`. The scenarios with only unanswered pairs do not omit
  `Held back by`, and the 0281 Requirements bullet (:88-92) is stale.
- 🔵 **Test Coverage**: The failure path of `read_set_status` is untested.
- 🔵 **Test Coverage**: No test covers an item with both an unanswered pair
  and a skipped pair.
- 🔵 **Standards**: The three new command modules open with two different
  docstring forms.
- 🔵 **Standards**: Phases 2 and 3 document the topic exit codes in
  different places.

#### Suggestions

- 🔵 **Architecture + Code Quality**: Fold the session check into the
  rule-read turn check, since it catches nothing the turn check misses.
- 🔵 **Code Quality**:
  - name `ManifestCount` for its role in freshness;
  - give the directory listing one type throughout.
- 🔵 **Security**:
  - if `$(…)` inside single quotes is denied, extend the space fallback to
    `$` and the backtick rather than weakening N1;
  - have the untrusted-folder refusal name a safe remedy, and state that the
    harness never runs with a bypass permission mode.
- 🔵 **Compatibility**:
  - say in the docs that `null` means missing or unreadable;
  - name a write fallback for the probe's failure branch.
- 🔵 **Test Coverage**:
  - guard that every eval task has an `_EVAL_CLI_DEV_BUILDS` entry;
  - pin the condition under which the reminder prints, for each gate.
- 🔵 **Standards**:
  - write the Migrations sentences in the declarative voice;
  - build `Toolchain`'s paths on `CLI_TARGET_DIR`;
  - check the question before `resolve_set` runs;
  - add `allow_hyphen_values` to the clap sketch;
  - rewrap one 83-column line.
- 🔵 **Usability**:
  - reword the reminder;
  - give `relates_to` a concrete example.

### Assessment

The verdict moves to **COMMENT**. Pass 5 raises one major and no critical
finding, so it is below the threshold of three. The plan is now in good shape
to implement. Across passes the major count has gone 18, 4, 5, 4 and now 1.
Pass 5 raised no finding against the product design: the contract, the slug,
the CLI gates and the `ask` and `report` prose.

The one major is a manual Phase 7 step that the probe script cannot run.
Either add a `--fixture PATH` option to `claude_permissions.py`, with a red
test that the read and `--update` both honour, or prescribe calling its
`measure()` directly for the rows. Add a fourth row in N1's exact shape at
the same time.

Six of the minor findings are worth fixing before implementation starts,
because each is a contradiction or a gap an implementer would hit at once:

1. Treat an exit-0 turn with no text as failing its criteria, and keep
   "errored" for a non-zero exit, a timeout, a missing log, no init event or
   `is_error`.
2. Set `commenters = ""` on the shlex lexer, or fail on any `#` outside
   quotes, and plant an `x#;` failure.
3. Reword the turn-command sentence about the injected fetch.
4. Branch the denied-command recovery on what was already written.
5. Correct the `# Errors` doc on `CountedFindings::of`.
6. Settle whether `--check-only` prints the rerun command.

The rest can be absorbed during Phase 5's red-test work, where the harness
tests make each point concrete.

## Verdict Changed — 2026-10-31T10:59:22+00:00

**Verdict:** APPROVE

After pass 5, the plan was revised to fix the pass-5 major (the probe
script gains a `--fixture PATH` option, and the manual step measures four
rows in a scratch TSV) and the six minors the pass-5 assessment named. The
author approved the plan for implementation without a further review pass.
The remaining pass-5 minors and suggestions are harness details that Phase
5's red tests settle.
