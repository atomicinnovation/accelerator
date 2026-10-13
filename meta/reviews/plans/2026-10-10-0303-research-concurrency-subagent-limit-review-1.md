---
type: "plan-review"
id: "2026-10-10-0303-research-concurrency-subagent-limit-review-1"
title: "Plan Review: Research Concurrency Within the Subagent Limit Implementation Plan"
date: "2026-10-10T21:49:12+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-10-0303-research-concurrency-subagent-limit"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "usability", "compatibility", "documentation", "standards"]
review_number: 1
review_pass: 5
tags: ["research", "research-topic", "conduct", "concurrency"]
last_updated: "2026-10-10T23:37:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Research Concurrency Within the Subagent Limit Implementation Plan

**Verdict:** REVISE

The design holds up. The concurrency rule is a pure, std-only domain type
behind the existing config and environment ports. The clamp moves to the
layer that bounds every caller. Each retry round must accept at least one
spawn, so the rounds always terminate. The plan falls short in execution:
three major gaps (a primitive `Option<usize>` window interface that discards
the new type's invariant, no test that the clamp actually bounds offered
spawns, and a tick plan that drops the flag) reach the configured REVISE
threshold. Several omissions in the Phase 3 test inventory would also stop
`mise run` going green as written.

### Cross-Cutting Themes

- **The `Concurrency` type is unwrapped at the boundary** (flagged by:
  code-quality, architecture). `window` keeps `Option<usize>`, whose `None`
  (unlimited) branch is now dead, and the "at least 1" invariant is lost at
  the CLI shell.
- **Diagnostics ignore provenance** (flagged by: usability, documentation,
  standards, code-quality). The empty-value error and both warnings always
  name `research.topic.concurrency`, even for `--concurrency`. Every sibling
  `outstanding` error names the flag the user passed, and the domain
  `Provenance::Flag` currently changes nothing observable.
- **Phase 3's test inventory is incomplete** (flagged by: standards,
  correctness, test-coverage). Three things would fail outside the listed
  tests: `test_research_topic_plans_each_batch_at_the_resolved_depth`
  (`test_research_structure.py:248-261`) still pins `--limit {concurrency}`,
  3 whole-object `assert_eq!(plan, json!({...}))` tests lack `concurrency`,
  and `configure_concurrency` omits frontmatter fences. A proposed Python
  line also exceeds 80 columns.
- **Plain queries change behaviour unannounced** (flagged by: architecture,
  correctness). A query without run flags goes from unlimited to capped, and
  gains a config dependency that can exit 1. Combined with step 7.2
  dropping the flag, this breaks the tick.
- **Cap-parser parity with the harness is underspecified** (flagged by:
  compatibility, correctness, test-coverage). The plan does not say digits
  must be ASCII-only, saturation must apply only on overflow, or how
  zero-padded long values are handled.
- **The exhaustion remedy is not actionable** (flagged by: usability,
  documentation). "set `research.topic.concurrency` to the harness cap" asks
  for a value the user cannot see, in the one case where the CLI and the
  harness disagree.
- **Duplicated parsing helpers** (flagged by: code-quality, correctness,
  architecture). `positive_integer` duplicates the CLI's `positive` but
  differs on overflow, and the plan wrongly says they match. The 0282 knob
  rule now lives in both Rust and prose.

### Tradeoff Analysis

- **Harness parity vs a uniform input grammar**: compatibility wants the cap
  parser to mirror the JS harness exactly (`+8`, ` 8 `), while usability
  notes the flag rejects those same inputs. Keep each strict to its own
  source and document the asymmetry; unifying them would break one contract
  or the other.
- **Stable warning text vs a more helpful one**: usability wants the clamp
  warning to name the env var and the remedy. The text is fixed verbatim by
  work item 0303, so changing it means amending the work item. Recommend
  changing the provenance subject now and adding the remedy only if the work
  item is reopened.

### Findings

#### Major

- 🟡 **Correctness**: Tick plan drops the concurrency flag, so an empty configured value fails the tick after a successful run
  **Location**: Phase 3, §5 (Step 7.2)
  Step 7.2 runs the plain plan "without `{concurrency flag}`". With an empty `research.topic.concurrency` and `--concurrency 5`, steps 2 and 6 succeed, but the tick exits 1 with `E_TOPIC_RESEARCH_CONCURRENCY` after `end-run` has removed the ledger.
- 🟡 **Code Quality + Architecture**: `window` keeps an `Option<usize>` limit whose `None` branch becomes dead code
  **Location**: Phase 3, §4 Resolution and output
  Every caller now passes `Some(...)`, so the domain still allows the unbounded state the item exists to remove. Pass `Concurrency` into `window` and drop the `usize::try_from` fallback from the CLI.
- 🟡 **Test Coverage**: No test proves a clamped concurrency bounds the offered spawns
  **Location**: Phase 3, §2 Integration tests
  The clamp tests assert only `plan["concurrency"]` and the warnings. Feeding `window` the unclamped value while rendering the clamped one would pass every test.

#### Minor

- 🔵 **Standards**: A third structural test still pins `--limit {concurrency}`
  **Location**: Phase 3, §5
  `test_research_topic_plans_each_batch_at_the_resolved_depth` (`:248-261`) is not in the plan, and the `rg` check does not search `tests/`.
- 🔵 **Correctness**: Existing exact-JSON plan assertions break when `concurrency` is added
  **Location**: Phase 3, §2
  3 tests (`~:193`, `~:229`, `~:743`) compare the whole object.
- 🔵 **Correctness**: `configure_concurrency` writes config without frontmatter fences
  **Location**: Phase 3, §1 Test hygiene
  Config is read from YAML frontmatter (`config_with` wraps its body in `---`), so every config-driven test would see the default.
- 🔵 **Standards**: Proposed Python step string runs past 80 columns
  **Location**: Phase 3, §5
  It is about 82 columns, which fails E501 in `mise run check`.
- 🔵 **Usability + Documentation + Standards + Code Quality**: Errors and warnings name the config key even for `--concurrency`
  **Location**: Phase 3, §4; §7 exit-code table
  Use `Provenance` to choose the subject. This also gives `Flag` and `ConfigFile` an observable difference.
- 🔵 **Architecture**: Plain queries silently lose their uncapped mode and gain a config dependency
  **Location**: Phase 3 / Key Discoveries
  State this as a deliberate interface change, or resolve concurrency only for run modes.
- 🔵 **Correctness + Documentation**: The knob-block edit contradicts the pinned misplaced-flag rule, and the lead-in still promises three injected values
  **Location**: Phase 3, §5 Knob block
  Name which sub-rules lose `--concurrency`, and reword "Each knob's configured value … is below".
- 🔵 **Test Coverage**: The config-file seam is undertested
  **Location**: Phase 3, §2
  No case covers a malformed configured value, a personal-over-team value (`config.local.md`), or an unreadable config file.
- 🔵 **Test Coverage**: No-warning cases do not assert stderr is clean, and the order of concurrency warnings relative to the ledger notice is unpinned
  **Location**: Phase 3, §2
- 🔵 **Test Coverage**: Structural pins miss Step 1's flag definition, Step 2's warning reporting, and the retry ordering, stop and single-recording rules
  **Location**: Phase 3, §5; Phase 4, §1
- 🔵 **Standards**: `plan_wording::concurrency_warning` has no unit test beside its siblings
  **Location**: Phase 3, §4
- 🔵 **Standards**: The config knob is read outside `ProjectContext`
  **Location**: Phase 3, §4 `configured_concurrency`
- 🔵 **Architecture**: Flag validation is split between `OutstandingFlags::parse` and the later resolver
  **Location**: Phase 3, §3
  `parse` could produce `Option<RequestedConcurrency>` with `Provenance::Flag`.
- 🔵 **Architecture**: The 0282 knob rule now has two implementations with no shared guard
  **Location**: What We're NOT Doing / Phase 3, §5
- 🔵 **Code Quality + Correctness**: `positive_integer` duplicates the CLI's `positive` but differs on overflow, and the plan's "matches" claim is wrong
  **Location**: Phase 2, §2
- 🔵 **Compatibility + Correctness + Test Coverage**: Harness-parser parity is underspecified
  **Location**: Phase 2, §1-2
  Specify ASCII-only digits and saturation only on overflow. Add cases for a non-ASCII digit, a long zero-padded cap, and a `"020"` request.
- 🔵 **Code Quality**: `resolve` pushes a warning from inside an `unwrap_or_else` closure
  **Location**: Phase 2, §2
- 🔵 **Code Quality + Standards**: `concurrency.concurrency.agents()` stutters
  **Location**: Phase 2, §2 / Phase 3, §4
- 🔵 **Usability + Documentation**: The exhaustion remedy asks for a harness cap the user cannot see
  **Location**: Phase 4, §4 Reason table
- 🔵 **Usability**: The clamp warning does not name the cap's source or how to raise it
  **Location**: Phase 3, §2 warning strings
- 🔵 **Usability**: The flag and the cap accept different inputs, so `--concurrency +8` collapses to 1
  **Location**: Phase 2, §2
- 🔵 **Documentation**: The configure docs name the cap variable but not where to set it
  **Location**: Phase 3, §6
- 🔵 **Documentation**: The configure paragraph says every invalid value clamps, but an empty value exits 1
  **Location**: Phase 3, §6
- 🔵 **Compatibility**: The retry is keyed to the harness's exact refusal string
  **Location**: Phase 4, §2
- 🔵 **Compatibility**: The effect of the clamp on older harnesses and in ultracode is not documented
  **Location**: Migration Notes
- 🔵 **Compatibility**: Only 2 of the cap's settings sources are verified to reach the CLI
  **Location**: Phase 3 Manual Verification
- 🔵 **Correctness**: "The first round always runs" is not conditioned on a refusal having occurred
  **Location**: Phase 4, §2
- 🔵 **Correctness**: The full-hold expectation depends on the first batch reporting `remaining` 0
  **Location**: Phase 4 Manual Verification

#### Suggestions

- 🔵 **Architecture + Standards**: Import `Environment` from `config::consent`, not `credentials`
  **Location**: Key Discoveries; Phase 3, §4
- 🔵 **Architecture**: The default-equals-cap decision is encoded as two unrelated literals
  **Location**: Phase 1 / Phase 2, §2
- 🔵 **Code Quality**: Move the env var name onto `SubagentCap` (`SubagentCap::SETTING`)
  **Location**: Phase 3, §4
- 🔵 **Code Quality**: The cap parser takes a roundabout route through `i128` and two saturations
  **Location**: Phase 2, §2
- 🔵 **Usability**: The docs do not say wider batches need both settings raised
  **Location**: Phase 3, §6
- 🔵 **Usability**: A plain query reports a clamped value without saying why
  **Location**: Phase 3, §2 / §7
- 🔵 **Documentation**: The CHANGELOG `outstanding` entry omits the `concurrency` field and the new error code
  **Location**: Phase 3, §7
- 🔵 **Test Coverage**: The assertions for the removed flag and the empty-value exit are loose
  **Location**: Phase 3, §2

### Strengths

- ✅ The concurrency rule is a pure, std-only domain module that satisfies
  the pup import rule. Config and environment reads stay behind the
  existing `ConfigAccess` and `Environment` ports at the composition root.
- ✅ The private newtype constructor enforces the "at least 1" invariant,
  and validation-before-clamp makes `AboveCap` on a malformed value
  impossible by construction.
- ✅ The retry rounds always terminate, because a later round requires an
  acceptance. They need no ledger change, since the pending batch already
  excuses re-issued writes, and an existing `window.rs` test protects that.
- ✅ Clamping in the CLI bounds every caller of `outstanding`, not only the
  skill.
- ✅ The test-hygiene refactor removes the ambient
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` leak.
- ✅ The table-driven domain cases hit the right boundaries: cap equals
  request, the `1e2` mutation detector, the empty refusal, and saturation.
- ✅ Warnings reuse the replaced-ledger `--start`-only path. The
  `concurrency` JSON field is additive, and the renames touch only
  unreleased surface. Binary pinning rules out skill/CLI version skew.
- ✅ The phases are ordered so each leaves the system consistent, and the
  rename and skill change land together for a stated reason.

### Recommended Changes

1. **Pass `{concurrency flag}` to the step 7.2 tick plan** (addresses: Tick
   plan drops the concurrency flag). Update the Step 7.2 edit, and add a
   structural assertion for the tick command.
2. **Make `window` take `Concurrency`** (addresses: `window` keeps an
   `Option<usize>`). Add a Phase 3 red step that updates the `window.rs`
   unit tests, remove the `None` branch, and keep the usize conversion
   inside `Concurrency`.
3. **Assert offered spawns in the clamp tests** (addresses: No test proves
   a clamped concurrency bounds the offered spawns). Use the 25-item arXiv
   set: cap 8 gives 8 spawns and `remaining` 17; config 24 gives 20 and 5;
   a malformed value gives 1.
4. **Complete the Phase 3 test inventory** (addresses: third structural
   test, exact-JSON assertions, frontmatter fences, 80 columns). Update or
   delete `test_research_topic_plans_each_batch_at_the_resolved_depth`, add
   `"concurrency": 20` to the 3 whole-object assertions, fence
   `configure_concurrency`, split the long f-string, and add
   `tests/unit/tasks` to the `rg` check.
5. **Use provenance in the diagnostics** (addresses: errors name the config
   key). For `Flag`, the subject becomes `--concurrency`. Pin both texts,
   and make the docs exit-code row cover both sources.
6. **Decide and state plain-query behaviour** (addresses: plain queries
   lose their uncapped mode). Either list the cap as an accepted interface
   change in Migration Notes and the `outstanding` reference, or resolve
   concurrency only on run modes.
7. **Tighten the parser spec** (addresses: parity underspecified, duplicate
   `positive_integer`). Specify ASCII digits and saturation only on
   overflow, add the extra cases, and drop the "matches `positive`" claim.
8. **Fix the knob-block and configure prose** (addresses: knob-block
   contradiction, lead-in, empty value exits 1, where to set the cap,
   exhaustion remedy). Name which sub-rules change, reword the lead-in,
   note that an empty value is refused, and say where the variable is set.
   Rewrite the reason-table remedy around the reported `concurrency` and
   variable visibility.
9. **Strengthen the structural pins** (addresses: missing pins). Pin Step
   1's flag definition, Step 2's warning reporting, and Phase 4's ordering,
   stop and single-recording sentences. Open the round bullets with "When
   the batch message capacity-refuses any spawn:".
10. **Smaller tidy-ups**, at the author's discretion: a `plan_wording` unit
    test, rename the `resolved` binding, an explicit `match` in `resolve`,
    import `Environment` from `consent`, the full-hold `remaining` 0 rule,
    the extra config-seam and stderr-cleanliness tests, and the
    older-harness note in Migration Notes.

## Per-Lens Results

### Architecture

**Summary**: The design is sound. A pure, std-only domain type fits the pup
import rule, and I/O stays in the command layer behind the existing ports.
The warnings and the `--start`-only notices reuse precedent, and the retry
rounds terminate without a ledger change. The concerns are minor seams: the
`Concurrency` newtype is unwrapped back to `Option<usize>`, plain queries
lose their uncapped mode, and the 0282 knob rule lives in two places.

**Strengths**: functional core / imperative shell respected; the harness
contract is isolated in one type; provenance is modelled explicitly and
mapped at the edge; layered defences (clamp plus retry) with bounded rounds;
the clamp sits at the right layer; phase ordering keeps the system
consistent; tradeoffs are acknowledged.

**Findings**:
- 🔵 minor / medium — **`Concurrency` newtype unwrapped back to the
  `Option<usize>` window interface** (Phase 3, §4). The spawn window still
  admits unlimited batches that the application now forbids. Have `window`
  take `Concurrency`.
- 🔵 minor / medium — **Plain queries silently lose their uncapped mode and
  gain a config dependency** (Phase 3). A plain query can now exit 1 on an
  empty knob even though ticking does not use concurrency. Document the
  change, or resolve only for run modes.
- 🔵 minor / medium — **Flag validation split between
  `OutstandingFlags::parse` and the later resolver** (Phase 3, §3).
  `OutstandingFlags` becomes half-parsed. Have `parse` produce
  `Option<RequestedConcurrency>` with `Provenance::Flag`.
- 🔵 minor / medium — **The 0282 knob rule has two implementations with no
  shared guard** (What We're NOT Doing / Phase 3, §5). Add a wording-parity
  test, or name the authoritative home.
- 🔵 suggestion / low — **`Environment` is imported through the credentials
  module** (Phase 3, §4). Use `config::consent::Environment` or a neutral
  re-export.
- 🔵 suggestion / low — **Default-equals-cap is encoded as two unrelated
  constants** (Phase 1 / Phase 2). Optionally add a cross-crate equality
  test.

### Code Quality

**Summary**: The domain module is pure and injectable, and follows the
newtype and port conventions. The main gap is at the domain/CLI edge:
`Concurrency` becomes `Option<usize>`, leaving a dead `None` branch. Smaller
issues: duplicated parsing helpers, a stuttering field name, and a side
effect inside a closure.

**Strengths**: pure rule behind ports; the at-least-1 invariant is enforced
by the type; warnings are a domain enum with wording in `plan_wording`;
`E_TOPIC_RESEARCH_LIMIT` is removed rather than kept alongside; test
environment control is centralised; `_section` is given a `path` parameter.

**Findings**:
- 🟡 major / high — **`window` keeps an `Option<usize>` whose `None` branch
  becomes dead code** (Phase 3, §4). Change `window` to take `Concurrency`,
  remove the `None` branch, and update the window unit tests.
- 🔵 minor / medium — **Positive-integer parsing is duplicated between the
  domain and the CLI** (Phase 2, §2). The two copies differ on overflow.
  Expose one parser, or name the difference.
- 🔵 minor / medium — **`resolve` pushes a warning from inside
  `unwrap_or_else`** (Phase 2, §2). Use an explicit `match` or `let … else`.
- 🔵 minor / high — **`concurrency.concurrency.agents()` stutters** (Phase
  2/3). Rename the binding to `resolved`, or rename the field.
- 🔵 minor / low — **`Provenance::Flag` and `ConfigFile` are never
  distinguished** (Phase 2, §2). Carry provenance on the warnings and the
  error, or model explicit versus default directly.
- 🔵 suggestion / medium — **The cap parser takes a roundabout route through
  `i128`** (Phase 2, §2). Strip `+`, reject a leading `-`, and reuse an
  unsigned saturating parser.
- 🔵 suggestion / low — **The env var name lives in the CLI rather than with
  `SubagentCap`** (Phase 3, §4). Add `SubagentCap::SETTING`.

### Test Coverage

**Summary**: The plan is strongly test-led, with table-driven domain tests
and black-box integration tests for almost every criterion. The main gap is
that no test checks that a clamped value bounds the offered spawns. Smaller
gaps: the config seam, negative stderr assertions, and unpinned prose
rules.

**Strengths**: well-chosen domain boundaries; the env-leak hygiene fix;
exact warnings and stderr where expected, and the `1e2` mutation detector;
the retry window already has a regression test; the `_prose` helper keeps
pins robust to reflow; `--depth {depth}` in the step strings doubles the
regression signal.

**Findings**:
- 🟡 major / high — **No test proves a clamped concurrency bounds the
  offered spawns** (Phase 3, §2). Assert spawns and `remaining` on the
  25-item arXiv set for cap 8, for config 24, and for a malformed value.
- 🔵 minor / medium — **The config-file seam is undertested** (Phase 2/3).
  Add cases for a malformed configured value, personal over team, and a
  malformed config file.
- 🔵 minor / medium — **No-warning cases do not assert stderr is clean**
  (Phase 3, §2). Also pin the order of the concurrency warning relative to
  the ledger notice.
- 🔵 minor / medium — **Structural pins miss the pass-through definition and
  warning reporting, and the negative check is narrow** (Phase 3, §5).
  Widen it to `config get research.topic.concurrency` across the whole
  skill.
- 🔵 minor / medium — **The retry ordering, stop and single-recording rules
  are unpinned** (Phase 4, §1).
- 🔵 minor / low — **Missing parser cases: long leading zeros and
  leading-zero requests** (Phase 2, §1).
- 🔵 suggestion / medium — **Loose assertions on the removed flag and the
  empty-value exit** (Phase 3, §2). Assert that stderr names `--limit`, and
  that no ledger is written on `--start`.

### Correctness

**Summary**: The rule is logically sound. Validation runs before the clamp,
`Concurrency` is never 0, and the retry rounds terminate. Using `effective`
keeps the empty-value error reachable. There is one real logic gap (the
tick plan drops the flag) and several spec inconsistencies that would make
red tests fail for the wrong reason.

**Strengths**: malformed values cannot also trigger `AboveCap`; the
invariant holds on every path; termination is provable; provenance is
separated correctly; the ledger retry window is relied on correctly;
resolution happens before any ledger write.

**Findings**:
- 🟡 major / high — **The tick plan drops the concurrency flag** (Phase 3,
  §5, Step 7.2). With an empty configured value, the tick exits 1 after a
  successful run. Pass `{concurrency flag}` to the tick.
- 🔵 minor / high — **Exact-JSON plan assertions break** (Phase 3, §2). Add
  `"concurrency": 20` to the 3 tests.
- 🔵 minor / medium — **`configure_concurrency` lacks frontmatter fences**
  (Phase 3, §1).
- 🔵 minor / high — **The knob-rule edit contradicts the pinned
  misplaced-flag rule** (Phase 3, §5).
- 🔵 minor / medium — **"The first round always runs" is unconditioned**
  (Phase 4, §2). Open with "When the batch message capacity-refuses any
  spawn:".
- 🔵 minor / medium — **The full-hold expectation depends on `remaining` 0**
  (Phase 4 Manual).
- 🔵 suggestion / low — **The helper parsing spec leaves ASCII-only digits
  and overflow handling implicit** (Phase 2, §2). The "matches `positive`"
  claim is wrong.

### Usability

**Summary**: Concurrency is hard to misconfigure: the defaults work, and
only explicit values warn. The weak points are diagnostics: errors name the
config key even for the flag, the warning omits its remedy, and the
exhaustion advice is not actionable. The flag and the cap also accept
different inputs.

**Strengths**: zero-config defaults; `--start`-only warnings with
`concurrency` always visible; hand runs are bounded too; the help text
names the full chain; warnings appear before spawning; the error row pairs
cause with fix.

**Findings**:
- 🔵 minor / high — **The clamp warning does not say where the cap comes
  from or how to raise it** (Phase 3, §2). The wording is fixed by the work
  item.
- 🔵 minor / high — **A bad `--concurrency` is reported as a config-key
  problem** (Phase 3, §4).
- 🔵 minor / medium — **The exhaustion remedy needs a harness cap the user
  cannot see** (Phase 4, §4).
- 🔵 minor / medium — **The flag and the cap accept different inputs**
  (Phase 2, §2). Document the asymmetry.
- 🔵 suggestion / medium — **Wider batches need two settings changed
  together** (Phase 3, §6).
- 🔵 suggestion / low — **A plain query reports a clamped value without
  saying why** (Phase 3, §2).

### Compatibility

**Summary**: Compatibility is well handled. The renames touch only
unreleased surface, binary pinning prevents skew, and the new field is
additive. The residual risk is reliance on external harness behaviour: the
refusal text, the parser, and which settings sources reach Bash.

**Strengths**: unreleased renames are amended in place; plugin-version
binary pinning; an additive JSON field; the ledger format is unchanged; no
new dependencies; Migration Notes names the clamp; the test env is
isolated.

**Findings**:
- 🔵 minor / medium — **The retry is keyed to the exact refusal string**
  (Phase 4, §2). Describe it by meaning, and record the verified harness
  version.
- 🔵 minor / low — **Parser parity leaves digit and trim semantics
  underspecified** (Phase 2, §2). Specify ASCII digits, add a non-ASCII
  case, and decide on U+FEFF trimming.
- 🔵 minor / medium — **Older-harness and ultracode clamp effects are
  undocumented** (Migration Notes).
- 🔵 minor / medium — **Only 2 of the cap's settings sources are verified**
  (Phase 3 Manual). Add user-level `~/.claude/settings.json` and
  `settings.local.json` `env` cases, or document which sources are
  verified.

### Documentation

**Summary**: Documentation is treated as part of the change, and its key
claims are pinned by tests. The gaps are small accuracy issues: the
empty-value message and its recovery, a stale knob lead-in, and no word on
where to set the cap variable.

**Strengths**: every default site is updated and pinned; the reference and
exit-code table move with the rename; the changelog is amended in place;
the rustdoc follows crate habits; the help text explains the chain; no
hand-edits to generated mirrors.

**Findings**:
- 🔵 minor / high — **The empty-concurrency error and recovery assume a
  config source** (Phase 3, §4/§7).
- 🔵 minor / medium — **The knob-block lead-in still promises three
  injected values** (Phase 3, §5).
- 🔵 minor / medium — **The user docs name the cap variable but not where
  to set it** (Phase 3, §6).
- 🔵 minor / medium — **The configure paragraph says every invalid value
  clamps, but an empty value exits 1** (Phase 3, §6).
- 🔵 suggestion / low — **The capacity-reason recovery may not be
  actionable** (Phase 4, §4).
- 🔵 suggestion / medium — **The CHANGELOG `outstanding` entry omits the
  new field and code** (Phase 3, §7).

### Standards

**Summary**: The plan closely follows the codebase's conventions: module
placement, the newtype shape, the error-code family, test naming and
helpers, and in-place changelog edits. The deviations are small: one missed
structural test, one E501 line, flag-agnostic error wording, a config read
outside `ProjectContext`, and an untested wording function.

**Strengths**: module placement; the `Depth`/`Limit` accessor pattern; the
`E_TOPIC_RESEARCH_*` family via `failed(...)`; a single warning path; test
helpers match existing builders; changelog and JSON-additivity
conventions.

**Findings**:
- 🔵 minor / high — **A third structural test still pins `--limit
  {concurrency}`** (Phase 3, §5). Add `tests/unit/tasks` to the `rg` check.
- 🔵 minor / high — **A Python step string runs past 80 columns** (Phase 3,
  §5).
- 🔵 minor / medium — **The empty-value error names the config key for the
  flag** (Phase 3, §4).
- 🔵 minor / high — **The new wording function has no `plan_wording` unit
  test** (Phase 3, §4).
- 🔵 minor / medium — **The config knob read is outside `ProjectContext`**
  (Phase 3, §4).
- 🔵 suggestion / medium — **`ResolvedConcurrency.concurrency` stutters**
  (Phase 2, §2).
- 🔵 suggestion / low — **`Environment` is imported via `credentials`**
  (Key Discoveries; Phase 3, §4).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-10

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Correctness**: Tick plan drops the concurrency flag — Resolved
- 🟡 **Code Quality + Architecture**: `window` keeps an `Option<usize>` limit — Resolved
- 🟡 **Test Coverage**: No test proves a clamped concurrency bounds the offered spawns — Resolved
- 🔵 **Standards**: A third structural test still pins `--limit {concurrency}` — Resolved
- 🔵 **Correctness**: Exact-JSON plan assertions break — Resolved
- 🔵 **Correctness**: `configure_concurrency` lacks frontmatter fences — Resolved
- 🔵 **Standards**: Python step string past 80 columns — Resolved
- 🔵 **Usability + Documentation + Standards + Code Quality**: Diagnostics name the config key for the flag — Resolved
- 🔵 **Architecture**: Plain queries lose their uncapped mode silently — Resolved
- 🔵 **Correctness + Documentation**: Knob-block contradiction and stale lead-in — Resolved
- 🔵 **Test Coverage**: Config-file seam, clean stderr, structural pins, retry pins, parser edge cases, loose assertions — Resolved
- 🔵 **Standards**: `plan_wording` unit test; config read outside `ProjectContext`; `concurrency.concurrency` stutter; `credentials` import — Resolved
- 🔵 **Architecture**: Split flag validation; two homes for the knob rule; default-equals-cap — Resolved
- 🔵 **Code Quality + Correctness**: Duplicate parsing helper and wrong "matches `positive`" claim; `unwrap_or_else` side effect; roundabout `i128`; env var on `SubagentCap` — Resolved
- 🔵 **Compatibility**: Refusal string, parser parity, older-harness note, settings sources — Resolved
- 🔵 **Correctness**: Unconditioned first round; full-hold `remaining` 0 — Resolved
- 🔵 **Usability + Documentation**: Exhaustion remedy needs an invisible harness cap — Partially resolved
- 🔵 **Usability**: Clamp warning omits the remedy — Not addressed (declined; the warning wording is fixed by the work item apart from its source)

### New Issues Introduced

- 🔵 **Correctness**: The `--spawned 0` case of `concurrency_warnings_are_raised_only_when_a_run_starts` expects 20 offered and `remaining` 5. Acknowledgement moves the batch into `attempted` (`ledger.rs:87-96`), so it offers 5, with `remaining` 0 and 20 `unfinished`.
- 🔵 **Correctness**: Phase 3 changes `window`'s signature (`public-api.txt:145`) without regenerating the snapshot, so `public-api:check` fails.
- 🔵 **Correctness**: The `rg` no-`--limit` guard matches the lines the plan adds (`"outstanding no longer takes --limit"`, `limit_is_no_longer_an_outstanding_flag`).
- 🔵 **Correctness + Test Coverage**: The wording-parity test greps raw Rust source. A `\`-continued format string breaks it, and the file's own test strings can satisfy it falsely.
- 🔵 **Correctness**: The `match` in `resolve` probably trips `clippy::option_if_let_else` (`nursery` is set to warn, and `cli:check` runs with `-D warnings`).
- 🔵 **Test Coverage**: No integration test runs `--concurrency 5` over an empty configured value, or a plain query over an empty value. This is the path the tick fix relies on.
- 🔵 **Usability + Documentation**: The exhaustion remedy ignores a `--concurrency` override and departs from the work item's wording without saying so.
- 🔵 **Standards**: `plan_wording::empty_concurrency` builds `E_TOPIC_RESEARCH_*` text, which every sibling builds in `topic_command.rs`.
- 🔵 **Standards**: `_configure_research_section` never passes the new `_section` `path` argument.
- 🔵 **Code Quality**: The window and ledger tests keep the retired `limit` vocabulary.
- 🔵 **Documentation**: A maintainer recheck note sits in the user-facing CHANGELOG. The `outstanding` reference does not say a hand run reads the cap from its own environment.
- 🔵 Suggestions: concurrency resolution is spread across 3 files (architecture, code quality); an unused `SubagentCap` `Default` impl; trimming with `char::is_whitespace` also strips U+0085, which JavaScript keeps; the configure paragraph is dense.

### Assessment

All 3 major findings and almost every minor finding are resolved, and
compatibility reports no issues. Lowering the verdict to COMMENT reflects
that no critical or major findings remain. The new issues are minor
specification errors, but 3 of them would still stop Phase 3 reaching a
green `mise run`: the `--spawned 0` expectation, the missing public-API
regeneration, and the over-broad `rg` guard. The clippy lint might too.
Fix those 4 before implementation; the rest are polish.

## Re-Review (Pass 3) — 2026-10-10

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Correctness**: `--spawned 0` expectation contradicts ledger acknowledgement — Resolved
- 🔵 **Correctness**: Public-API snapshot not regenerated — Resolved
- 🔵 **Correctness**: `rg` guard matches the plan's own lines — Resolved
- 🔵 **Correctness + Test Coverage**: Wording-parity test greps raw Rust — Resolved
- 🔵 **Correctness**: `match` in `resolve` trips `option_if_let_else` — Resolved
- 🔵 **Test Coverage**: No flag-over-empty-config or plain-query empty test — Resolved
- 🔵 **Usability + Documentation**: Exhaustion remedy ignores `--concurrency` and departs from the work item — Resolved
- 🔵 **Standards**: `plan_wording` builds `E_TOPIC_RESEARCH_*` text — Resolved
- 🔵 **Standards**: `_section` `path` argument not passed — Resolved
- 🔵 **Code Quality**: Retired `limit` vocabulary in window tests — Resolved
- 🔵 **Documentation**: Maintainer note in the CHANGELOG; hand-run cap source — Resolved
- 🔵 **Suggestions**: Resolution spread across 3 files, unused `Default`, U+0085 trim, dense configure paragraph — Resolved

### New Issues Introduced

- 🔵 **Correctness + Standards**: `HARNESS_DEFAULT` uses `NonZeroU32::new(20).unwrap()`. `unwrap_used = "warn"` with `warnings = "deny"` (`cli/Cargo.toml:231,237`) makes that an error, so Phase 2's `cli:check` fails.
- 🔵 **Standards**: The rule that each expected wording string is "a single unbroken literal" conflicts with the 80-column `\`-continuation habit in `plan_wording.rs`, and is redundant now that `_rust_text` joins continuations.
- 🔵 Suggestions:
  - the end-state claim "unbounded batch is no longer representable" overstates the guarantee, because `Concurrency::new` is public (architecture);
  - a second `FixedEnvironment` duplicates the one in `fetch_command.rs:376-385`; rename the team config writer `configure_team_concurrency`; drop a redundant f-string (code quality);
  - no test drives `resolve_concurrency` propagating a config read error (test coverage);
  - the `outstanding` reference's JSON field list lacks a `concurrency` entry (documentation);
  - the empty-config error omits "or remove it" (usability).

### Assessment

Every pass-2 finding is resolved. Two new minors remain, and one of them,
the const `unwrap`, would fail Phase 2's `cli:check`. Both are one-line
plan fixes. The rest are suggestions. Once the `unwrap` is replaced, the
plan is ready for implementation.

## Re-Review (Pass 4) — 2026-10-11

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Correctness + Standards**: `HARNESS_DEFAULT` const `unwrap` — Resolved
- 🔵 **Standards**: "Single unbroken literal" rule — Resolved
- 🔵 **Architecture**: End-state claim overstated — Resolved (code quality rates it partially resolved; see below)
- 🔵 **Code Quality**: Duplicate `FixedEnvironment`; team writer name; redundant f-string — Resolved
- 🔵 **Test Coverage**: Config read error propagation untested — Resolved
- 🔵 **Documentation**: No `concurrency` in the JSON field list — Resolved
- 🔵 **Usability**: Empty-config error omits "or remove it" — Resolved

### New Issues Introduced

- 🔵 **Correctness**: Moving `FixedEnvironment` leaves `use config::credentials::Environment;` (`fetch_command.rs:289`) unused. With `warnings = "deny"` that breaks the refactor-first step.
- 🔵 **Code Quality**: `NonZeroU32::MIN.saturating_add(19)` hides the domain's 20.
- 🔵 **Documentation**: `concurrency` sits in the field list that `--depth N` introduces, which implies it appears only on deepened plans.
- 🔵 Suggestions:
  - `Concurrency::new` is public, so the cap is enforced by convention; `pub(crate)` would enforce it by visibility (code quality);
  - `the_cap_is_read_from_the_environment` asserts only "a warning" (test coverage).

### Assessment

3 of the 7 re-run lenses, architecture, standards and usability, report
nothing new, and compatibility has been clean since pass 2. The remaining items are 3 minors and 2 suggestions, each a
one-line plan edit. Only the unused import would break a build. The design
has converged.

## Re-Review (Pass 5) — 2026-10-11

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Correctness**: Unused `Environment` import after the test-double move — Resolved
- 🔵 **Code Quality**: `HARNESS_DEFAULT` hides the 20 — Resolved
- 🔵 **Documentation**: `concurrency` in the `--depth` field list — Resolved
- 🔵 **Code Quality**: Public `Concurrency::new` — Resolved
- 🔵 **Test Coverage**: `the_cap_is_read_from_the_environment` asserts only "a warning" — Resolved

### New Issues Introduced

- 🔵 **Documentation**: The extended research.md paragraph still opens "It prints JSON with four arrays", which the two new integer fields make wrong.
- 🔵 **Test Coverage** (suggestion): The plan does not name a single home for the `concurrency(n)` and `unbounded()` test helpers shared by the window and ledger tests.

### Assessment

Correctness and code quality report nothing new. Correctness confirmed that
the const `match` passes clippy (`option_if_let_else` skips const contexts),
that `Concurrency::new` is used by `resolve`, and that the black-box `admits`
cases are reachable through public API. One documentation wording fix and
one test-helper suggestion remain. Neither affects the build. The plan is
ready for implementation.

## Approval — 2026-10-11

**Verdict:** APPROVE

The two pass-5 items are addressed in the plan: the research.md lead-in
becomes "four arrays ... and two counts", and the `concurrency(n)` and
`unbounded()` test helpers share one `#[cfg(test)]` home. Every finding
across the five passes is resolved, apart from the clamp-warning remedy,
which was declined. The plan is approved for implementation.
