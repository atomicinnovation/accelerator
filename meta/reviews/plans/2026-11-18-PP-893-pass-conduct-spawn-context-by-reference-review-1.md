---
type: "plan-review"
id: "2026-11-18-PP-893-pass-conduct-spawn-context-by-reference-review-1"
title: "Plan Review: Pass Conduct Spawn Context by Reference Implementation Plan"
date: "2026-11-18T21:37:00+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-18-PP-893-pass-conduct-spawn-context-by-reference"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "compatibility", "usability"]
review_number: 1
review_pass: 3
tags: ["research", "conduct", "run-ledger", "research-guard", "config-template"]
last_updated: "2026-11-19T00:21:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Pass Conduct Spawn Context by Reference Implementation Plan

**Verdict:** REVISE

The plan's structure is sound. The snapshot is captured at the single point
where an offer happens. The budget rule moves into the domain. The guard gains
one exact prefix with no denylist. The phases are ordered so the guard permits
the query before any prompt relies on it, and the code references are
accurate. It needs revision before implementation: Phase 5 orphans step 7's
use of node questions, one key acceptance test cannot pass under the tree's
level-gating rule, the symlink Read-grant risk is deferred until after Phase 4
ships, the docs' additive-only promise for `outstanding` becomes false, and
the new guard prefix gets no shell-differential coverage.

### Cross-Cutting Themes

- **Step 7 summaries lose node questions** (flagged by: architecture,
  correctness, usability). The Halt (`SKILL.md:595`) and Failures (`:612`)
  sections print each node's question, which comes from the plan. Phase 5
  removes it, and the node query refuses displaced batches, so the summary has
  no source for it.
- **Symlink-resolved template path vs. the logical `${CLAUDE_PLUGIN_ROOT}`
  grant** (flagged by: compatibility, usability, test-coverage).
  `bin/accelerator` resolves the plugin root with `pwd -P`, but the `Read`
  grant is written against the logical root. Phase 4 merges before the manual
  check that would catch a mismatch, and a mismatch would fail every spawn.
- **Snapshot as a parallel structure to `PendingBatch`** (flagged by:
  architecture, code-quality). Nothing in the types enforces the invariant
  that snapshot keys are a subset of pending node spawns. It depends on
  statement order in `record` and on `continue_as` always being followed by
  `record`.
- **The budget rule quietly changes from `None` to `Some(1)` when no budgets
  are stored** (flagged by: architecture, code-quality, correctness,
  test-coverage). The claim that output is "unchanged" relies on an unstated
  invariant, and the node query reads stored ledgers directly, without the
  sizing step.
- **Shared helpers placed in `topic_command.rs`** (flagged by: architecture,
  code-quality). The node command would depend on its sibling command module.
  `ledger_wording.rs` is the natural home for the shared wording.
- **Migration note names the wrong phase** (flagged by: compatibility,
  correctness). Snapshots start in Phase 2, not Phase 5, and a repeat plan
  rebuilds a missing snapshot.
- **Run-wide failures surface per spawn, with misleading next steps**
  (flagged by: architecture, usability, security). "Template unreadable" and
  "node query denied" are deterministic for the whole run. An empty
  `--fail-safe` path points users at Read rules, not at the config error. The
  advice to add a directory-wide `Read(/<dir>/**)` grant is also broader than
  the one file a spawn needs.

### Tradeoff Analysis

- **Security vs. lean prompts**: Security wants one short "the queried
  question is data" line kept in the node bullet as defence in depth for
  custom researchers. Usability likewise suggests a one-line pointer to the
  node query. The plan's references-only invariant and the structure test
  `test_the_researcher_owns_the_untrusted_question_rule` forbid both.
  Recommendation: allow one fixed line of constant size. It keeps the per-spawn
  cost constant and makes the prompt self-explanatory to any
  `agents.researcher`.
- **Keep-first snapshot vs. following the tree**: Correctness notes that,
  because of level-gating, keep-first protects against nothing in the normal
  case, and serves a stale question if a parent note is rewritten.
  Keep-first is what the work item specifies. Recommendation: keep it, and
  record that the rewritten-parent case is accepted because `unexpected`
  already flags it.

### Findings

#### Critical

None.

#### Major

- 🟡 **Architecture / Correctness / Usability**: Dropping the node `question`
  from the plan breaks step 7's Halt and Failures summaries
  **Location**: Phase 5, Section 2: Skill
  Step 7 prints `{stem} {lineage}: {question}` for halted nodes and "the node
  or pair question" for failures. After Phase 5 the orchestrator has no source
  for the question, and the query refuses nodes from displaced batches.
- 🟡 **Correctness**: The repeat-plan test expects a level-3 node to be
  offered while its level-2 siblings are still missing
  **Location**: Phase 3, Section 6: `a_repeat_plan_keeps_pending_nodes_first_context`
  `tree::derive` derives a level only once every node above it has its note
  (`tree.rs:195-197`), so `3-1` cannot be offered while `2-2` and `2-3` are
  pending. The ref is also malformed: a level-3 lineage is `3-1-1`. The
  sibling-note test also passes without a snapshot.
- 🟡 **Compatibility / Usability**: The printed plugin-default path is
  symlink-resolved, but the Read grant uses the logical `CLAUDE_PLUGIN_ROOT`
  **Location**: Phase 1 / Phase 4: Manual Verification
  The check is deferred to "decide before phase 5", after Phase 4 has shipped.
  A mismatch fails every spawn with "template unreadable", and the
  troubleshooting row then gives the wrong fix.
- 🟡 **Compatibility**: Removing node fields breaks the documented
  additive-only contract for the `outstanding` JSON
  **Location**: Phase 5, Section 5: Docs
  `research.md:629` promises that fields are "never renamed or removed", and
  the plan leaves that statement in place.
- 🟡 **Security**: The shell-differential smuggling corpus never exercises the
  new node prefix
  **Location**: Phase 3, Section 6: `test_lexer_differential.py`
  The rows at `:310-313` are harness controls that never ask the guard for a
  verdict. The guard-judged baseline, boundary and mutant rows are built only
  from the fetch prefix.

#### Minor

- 🔵 **Code Quality**: A third copy of the read-ledger-then-check-owner match
  **Location**: Phase 3, Section 3. Extract an `owned_ledger(set_root, run,
  fs)` helper for `run_node` and `run_end_run` to share.
- 🔵 **Architecture / Code Quality**: The snapshot is a parallel structure to
  `PendingBatch`, with an invariant nothing enforces
  **Location**: Phase 2, Sections 1–2. Fold the contexts into `PendingBatch`,
  or clear acknowledged entries in `continue_as`.
- 🔵 **Correctness / Code Quality / Architecture / Test Coverage**: The domain
  `search_budget` answers `Some(1)` where today's code answers `None`
  **Location**: Phase 3, Section 1. Pin the no-budgets case in a test and
  state the chosen semantics. Consider putting the rule in `search_budget.rs`.
- 🔵 **Code Quality / Architecture**: The destination of the shared helpers is
  left unspecified
  **Location**: Phase 3, Section 3. Move `superseded` and `no_usable_ledger`
  into `ledger_wording.rs`, and put `set_slug` in a neutral module.
- 🔵 **Code Quality**: A planned doc comment restates the code
  **Location**: Phase 3, Section 1. Drop the `search_budget` doc comment, or
  name the floor as a constant.
- 🔵 **Compatibility / Correctness**: The migration note pins the snapshot gap
  to the wrong phase
  **Location**: Migration Notes. The gap is in batches offered before Phase 2,
  and a repeat plan heals it.
- 🔵 **Compatibility**: No changelog entries for contract changes
  **Location**: Implementation Approach. Add Unreleased bullets covering the
  new commands, the removed fields and the obligations on
  `agents.researcher` and `agents.composer` overrides.
- 🔵 **Compatibility**: Template configs outside the project now need a new
  permission
  **Location**: Phase 4, Section 3. Document it in `configuration.md`
  alongside `--path`, or add a check before the batch.
- 🔵 **Usability**: The reused "superseded" error gives misleading advice for
  a run that never started
  **Location**: Phase 3, Section 3. Use neutral "not this set's current run"
  wording, or map each error code to its own next step.
- 🔵 **Usability**: An empty template path under `--fail-safe` leads to the
  wrong troubleshooting advice
  **Location**: Phase 1 and Phase 4 troubleshooting. Add a diagnostic step:
  run `--path` without `--fail-safe`.
- 🔵 **Usability**: The node-query failure categories overlap existing rows
  and leave the guard-refusal case unassigned
  **Location**: Phase 5: Researcher outcomes. Map `E_RESEARCH_GUARD_COMMAND`
  to an outcome, and give precedence over the generic `E_*` row.
- 🔵 **Architecture**: Run-wide failures surface per spawn rather than once
  per run
  **Location**: Phase 4/5 troubleshooting. Classify them as halting reasons,
  or check before the first spawn.
- 🔵 **Security**: The untrusted-question framing moves into a single agent
  file
  **Location**: Phase 5, Section 2 and the structure test. Keep one line in
  the bullet and relax the test.
- 🔵 **Security**: The troubleshooting advice recommends a broad directory
  Read grant
  **Location**: Phase 4, Section 3. Recommend the exact file path printed by
  `--path`.
- 🔵 **Correctness**: Keep-first carry-over can serve a question the tree no
  longer asks
  **Location**: Phase 2, Section 1. Refresh the entry when the questions
  differ, or document the case as accepted.
- 🔵 **Correctness**: The printed template path is not normalised for
  project-relative config values
  **Location**: Phase 1, Section 2. Normalise `..` segments, and specify that
  path mode carries the fallback warning.
- 🔵 **Test Coverage**: The domain tests need builders that `window.rs` keeps
  private and hard-codes
  **Location**: Phase 2, Section 4. Plan a shared test-support module with
  builders that take a profile, a question and known questions.
- 🔵 **Test Coverage**: The id test cannot tell the set-directory name from
  the slug argument
  **Location**: Phase 3, Section 6. Add rows that pass the set directory path
  and a sub-document path.
- 🔵 **Test Coverage**: The refusal table asserts only "an E_ line"
  **Location**: Phase 3, Section 6. Pin the error code for each case, and add
  rows for a corrupt ledger and an unparseable `--node`.
- 🔵 **Test Coverage**: No test pins that only the offer narrowed by
  concurrency is snapshotted
  **Location**: Phase 2, Section 4. Add a bounded-concurrency domain test.
- 🔵 **Test Coverage**: No test for a pending node that has no snapshot entry
  **Location**: Phase 2, Section 4. Add a case covering a legacy ledger
  mid-run.
- 🔵 **Test Coverage**: The per-construct lexer table exercises only the fetch
  prefix
  **Location**: Phase 3, Section 6. Iterate over `PERMITTED_PREFIXES`.
- 🔵 **Test Coverage**: Structure tests could pass vacuously
  **Location**: Phases 4/5. Assert that exactly three bullets are found, and
  unit-test `_injected_items`.

#### Suggestions

- 🔵 **Code Quality**: The budget-source rule is repeated across three prose
  files. State it once, in the researcher definition.
- 🔵 **Code Quality**: Return the `Degrade` choice and the renderer from one
  match on `TemplateShown`.
- 🔵 **Usability**: Put a one-line pointer to the node query in the
  references-only prompt.
- 🔵 **Usability**: Describe a reliable way to capture the query output during
  the manual smoke run.
- 🔵 **Security**: Pin the write-guard invariant that a researcher write to
  `.conduct-run.json` is blocked.
- 🔵 **Test Coverage**: Add path-mode tests for a read failure without
  `--fail-safe` and for a symlinked plugin root.
- 🔵 **Compatibility**: Update the `--fail-safe` and subcommand help text for
  path mode.

### Strengths

- ✅ The snapshot is captured inside `RunLedger::record`, the single point
  where an offer happens. It reuses the existing lifecycle, and `continue_as`
  already clears acknowledged batches.
- ✅ The search-budget rule moves from the CLI renderer into the domain, so
  the plan and the query share one source.
- ✅ The guard widens by one exact, space-terminated prefix and stays
  default-deny with no denylist. Near-miss commands (`topic nodex`, a bare
  `topic node`, chaining) are tested.
- ✅ Phase ordering means no merged prerelease ships a researcher that the
  guard blocks. Each phase can merge independently.
- ✅ The ledger change is additive and backward-compatible in both directions,
  and a codec test covers a legacy ledger.
- ✅ Only values that cannot be derived are stored. Lineage, level, cap, id
  and path are derived again from the ref.
- ✅ The `topic_node.rs` tests map one-to-one onto the acceptance criteria and
  use literal expected values. `the_query_writes_nothing` checks the bytes.
- ✅ `--path` maps to a domain enum (`TemplateShown`) at the clap boundary.
  Path mode under `--fail-safe` suppresses output rather than injecting a
  notice.
- ✅ Each named failure outcome has its own troubleshooting row, following
  the "fetch denied" precedent.
- ✅ The plan's code references were checked against the source and are
  accurate.

### Recommended Changes

1. **Decide how step 7 names nodes after Phase 5, and update it**
   (addresses: the step 7 summaries finding)
   Add a Phase 5 change to step 7's Halt and Failures formats. The simplest
   option is `<stem> <lineage>` plus the pair question from `unanswered`.
   Alternatively, the researcher's summary echoes its queried question. Pin
   the choice with a structure test.
2. **Rewrite the repeat-plan test to respect level-gating** (addresses: the
   repeat-plan test finding)
   Split it in two. First: a repeat plan after `2-1`'s note keeps `2-2` and
   `2-3` answering, and `2-1` exits 1. Second: once every level-2 note is
   written and acknowledged, `03-a-web:3-1-1` answers with Q4. Note in the
   work item that the combined acceptance criterion cannot be met.
3. **Resolve the symlink grant question before Phase 4 merges** (addresses:
   the symlink grant finding, and the symlinked-root test suggestion)
   Add an automated launcher test with a symlinked plugin root. Either print
   the logical `CLAUDE_PLUGIN_ROOT`-based path for the plugin-default tier,
   or widen the grant. Make the check a Phase 1 or Phase 4 exit criterion.
4. **Amend the additive-only statement in `research.md`** (addresses: the
   additive-only contract finding)
   Scope the statement, or note that the fields moved to the node query, and
   add a structure test.
5. **Parameterise the lexer differential over both prefixes** (addresses: the
   lexer differential finding and the per-construct table finding)
   Emit the baseline, boundary and mutant rows under each of
   `PERMITTED_PREFIXES`, and correct the plan's claim about the control rows.
   Do the same for the Rust per-construct table.
6. **Pin the budget fallback semantics** (addresses: the budget fallback
   findings)
   Add a unit test for a ledger with no stored budgets, and state the rule.
   Consider placing it in `search_budget.rs`, and drop the doc comment that
   restates the code.
7. **Tighten the ledger and helper structure** (addresses: the parallel
   snapshot structure, the third copy of the owner check and the unspecified
   helper destination)
   Extract `owned_ledger`, move the wording into `ledger_wording.rs`, and
   either fold the contexts into `PendingBatch` or clear acknowledged entries
   in `continue_as`.
8. **Strengthen the test cases** (addresses: the Test Coverage minor
   findings)
   Pin the error code for each refusal row, and add the set-directory slug
   rows, a bounded-concurrency snapshot test, a pending node with no
   snapshot entry, shared test builders and guards against vacuous structure
   tests.
9. **Fix the migration note and add changelog entries** (addresses: the
   migration note finding and the missing changelog finding)
10. **Sharpen the troubleshooting** (addresses: the usability and security
    findings on troubleshooting)
    Add the diagnostic step without `--fail-safe`. Recommend an exact-file
    `Read` rule. Give a neutral not-current-run wording, map the guard refusal
    to an outcome, and consider making run-wide failures halt the run.
11. **Optionally keep one constant line in the node bullet** (addresses: the
    untrusted-question framing finding and the query-pointer suggestion)
    The line would point to the node query and mark its question as data.
    Relax the structure test to match.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. The domain rule for the search
budget moves into the research crate, the snapshot is captured at the single
point where an offer happens, the node query copies the existing read-only
command shape, and the phases are ordered so the guard permits the query
before any prompt depends on it. The most significant gap is a consumer the
plan misses: step 7's Halt and Failures summaries. There are also minor
concerns about cohesion and about how failures surface.

**Findings**:
- 🟡 major / high, Phase 5 §2: dropping the node `question` breaks the Halt
  and Failures summaries. Options: (a) print `{stem} {lineage}` with the
  pair's question; (b) have the researcher's summary echo the question; (c)
  keep a short question in the plan for reporting.
- 🔵 minor / medium, Phase 2 §1–2: the snapshot is a parallel structure with
  an invariant nothing enforces. Make it part of `PendingBatch`, and keep the
  adapter key separate for 0310.
- 🔵 minor / medium, Phase 4 §3 and Phase 5 §2: run-wide failures surface per
  spawn. Classify them as halting reasons, or check before the first spawn.
- 🔵 suggestion / medium, Phase 3 §3: put the shared helpers in
  `ledger_wording.rs` or a neutral module.
- 🔵 suggestion / low, Phase 3 §1: the consolidated budget rule differs from
  the old one when no budgets are stored. Preserve `None`, or pin `Some(1)`
  in a test.

### Code Quality

**Summary**: The plan is well designed for maintainability, with good domain
names and duplication removed. The concerns are small: the snapshot is a
parallel structure, the ledger-owner match is copied a third time, the budget
rule sits on `RunLedger`, and some planned doc comments restate the code.

**Findings**:
- 🔵 minor / high, Phase 3 §3: a third copy of the read-ledger-then-check-owner
  match. Extract `owned_ledger`.
- 🔵 minor / medium, Phase 2 §1–2: the snapshot is a parallel structure to
  `PendingBatch`. Clear it in `continue_as`, or merge it into the batch.
- 🔵 minor / medium, Phase 3 §1: the budget rule is on `RunLedger` but only
  touches `SearchBudgets`, and its semantics change.
- 🔵 minor / medium, Phase 3 §3: the destination of the shared helpers is
  unspecified.
- 🔵 minor / high, Phase 3 §1: the planned doc comment restates the code.
- 🔵 suggestion / medium, Phase 5 §4: the budget-source rule is repeated in
  three prose files.
- 🔵 suggestion / low, Phase 1 §2: the `Degrade` choice is coupled to the
  output form only implicitly.

### Test Coverage

**Summary**: The test-first approach is unusually thorough, and the tests
use literal expected values. Several gaps would let plausible mutations
through: slug equals directory name, only unbounded concurrency is tested,
refusal codes are not pinned, the builders are private, and the no-budgets
case is untested.

**Findings**:
- 🔵 minor / high, Phase 2 §4: needs private `window.rs` builders that
  hard-code fields. Plan a shared test-support module.
- 🔵 minor / high, Phase 3 §6: the id test cannot tell the directory name
  from the slug argument.
- 🔵 minor / high, Phase 3 §6: the refusal table asserts only "an E_ line".
- 🔵 minor / medium, Phase 2 §4: no test with concurrency narrowing the offer.
- 🔵 minor / medium, Phase 2 §4: no test for a pending node with no snapshot
  entry.
- 🔵 minor / medium, Phase 3 §6: the per-construct lexer table covers only the
  fetch prefix.
- 🔵 minor / medium, Phase 3 §1: the no-stored-budgets case is untested.
- 🔵 minor / medium, Phases 4/5: structure tests could pass vacuously.
- 🔵 suggestion / medium, Phase 1 §3: no tests for a read failure without
  `--fail-safe` or for a symlinked plugin root.

### Correctness

**Summary**: The code references are accurate, and the snapshot lifecycle is
sound. The repeat-plan test is infeasible under level-gating, and Phase 5
orphans step 7's node questions. Smaller issues: the migration note, the
budget fallback, the keep-first behaviour, and path normalisation.

**Findings**:
- 🟡 major / high, Phase 3 §6: the repeat-plan test expects a level-3 offer
  while level-2 nodes are pending. `3-1` is also an invalid level-3 lineage,
  and the sibling-note test passes without a snapshot.
- 🟡 major / high, Phase 5 §2: the summary still prints node questions the
  plan no longer carries.
- 🔵 minor / high, Migration Notes: the missing-snapshot gap is attributed to
  the wrong phase.
- 🔵 minor / medium, Phase 3 §1: the domain budget answers 1 where the plan
  path answers nothing.
- 🔵 minor / low, Phase 2 §1: keep-first can serve a stale question after a
  parent is rewritten.
- 🔵 minor / low, Phase 1 §2: project-relative paths keep `..` segments, and
  the fallback warning's routing is not specified.

### Security

**Summary**: The guard stays default-deny, with one exact prefix, the same
lexer, and chaining blocked. A researcher cannot forge the ledger. The main
gap is that the shell-differential corpus never exercises the new prefix.

**Findings**:
- 🟡 major / high, Phase 3 §6: the smuggling rows named in the plan are
  harness controls, not guard verdicts. Parameterise the corpus over both
  prefixes.
- 🔵 minor / medium, Phase 5 §2: the untrusted-question framing moves into a
  single agent file.
- 🔵 minor / medium, Phase 4 §3: the troubleshooting row recommends a broad
  directory Read grant.
- 🔵 suggestion / medium, Phases 2/3: pin the invariant that the guard blocks
  writes to `.conduct-run.json`.

### Compatibility

**Summary**: The ledger key is backward- and forward-compatible, and the guard
is widened before any prompt depends on it. Gaps: the additive-only docs
promise, the symlink path versus the grant, the migration note and the
missing changelog entries.

**Findings**:
- 🟡 major / high, Phase 5 §5: removing node fields breaks the documented
  additive-only contract.
- 🟡 major / medium, Phases 1/4: the printed path is symlink-resolved, but the
  grant uses the logical root.
- 🔵 minor / high, Migration Notes: the note pins the gap to the wrong phase.
- 🔵 minor / medium, Implementation Approach: no changelog entries.
- 🔵 minor / medium, Phase 4 §3: existing template configs outside the
  project need a new permission.
- 🔵 suggestion / high, Phase 1 §1: the `--fail-safe` help text will
  misdescribe path mode.

### Usability

**Summary**: The query reuses familiar conventions, and the failure outcomes
are named. The friction is on the operator's side: step 7 loses its node
questions, several diagnostic paths mislead, and the symlink risk is resolved
late.

**Findings**:
- 🟡 major / high, Phase 5: the final summary loses node questions.
- 🟡 major / medium, Phase 4: the out-of-the-box Read-grant risk is only
  checked after Phase 4 ships.
- 🔵 minor / high, Phase 3 §3: the "superseded" wording misleads for a run
  that never started.
- 🔵 minor / medium, Phases 1/4: an empty `--fail-safe` path leads to the
  wrong advice.
- 🔵 minor / medium, Phase 5: the failure categories overlap, and the guard
  refusal is unassigned.
- 🔵 suggestion / medium, Phase 5 §2: the references-only prompt is opaque to
  custom researchers.
- 🔵 suggestion / low, Testing Strategy: checking the query by hand needs a
  live batch.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-18T23:41:24+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture / Correctness / Usability**: Dropping the node `question` breaks step 7's summaries — Resolved (nodes named `{stem} {lineage} (of: {pair question})`, pinned by a structure test)
- 🟡 **Correctness**: Repeat-plan test expects a level-3 node while level-2 nodes are pending — Resolved (split into two scenarios; work item amended)
- 🟡 **Compatibility / Usability**: Symlink-resolved path vs logical Read grant — Resolved (the CLI now prints the template; no path or Read grant)
- 🟡 **Compatibility**: Removing node fields breaks the additive-only contract — Resolved (scoped to the first stable release)
- 🟡 **Security**: Shell-differential corpus never exercises the node prefix — Resolved
- 🔵 **Code Quality**: Third copy of the read-ledger-then-check-owner match — Partially resolved (`owned_ledger` extracted, but its absence rule breaks `end-run`; see new issues)
- 🔵 **Architecture / Code Quality**: Snapshot is a parallel structure to `PendingBatch` — Resolved
- 🔵 **Correctness / Code Quality / Architecture / Test Coverage**: Budget fallback `None` vs `Some(1)` — Resolved
- 🔵 **Code Quality / Architecture**: Shared helper destination unspecified — Resolved
- 🔵 **Code Quality**: Doc comment restates the code — Resolved
- 🔵 **Code Quality**: Budget-source rule repeated across three prose files — Resolved
- 🔵 **Compatibility / Correctness**: Migration note names the wrong phase — Resolved
- 🔵 **Compatibility**: No changelog entries — Resolved
- 🔵 **Compatibility**: Template configs outside the project need a new permission — Resolved by the design change
- 🔵 **Usability**: Reused "superseded" error misleads for a never-started run — Still present (accepted by user decision; the per-code row was chosen over rewording)
- 🔵 **Usability**: Empty `--fail-safe` path leads to wrong advice — Resolved by the design change
- 🔵 **Usability**: Node-query failure categories overlap; guard refusal unassigned — Resolved
- 🔵 **Architecture**: Run-wide failures surface per spawn — Resolved (template pre-check; permission denials accepted and documented)
- 🔵 **Security**: Untrusted-question framing in one file — Resolved (fixed line in the node bullet)
- 🔵 **Security**: Broad directory Read grant advice — Resolved by the design change
- 🔵 **Security**: Pin the ledger write-guard invariant — Resolved
- 🔵 **Correctness**: Keep-first can serve a stale question — Resolved (documented as accepted)
- 🔵 **Correctness**: Template path not normalised — Resolved by the design change
- 🔵 **Test Coverage**: All nine prior findings — Resolved
- 🔵 **Usability**: References-only prompt opaque to custom researchers — Resolved
- 🔵 **Usability**: Checking the query by hand needs a live batch — Partially resolved (the procedure given cannot work; see new issues)

### New Issues Introduced

- 🟡 **Code Quality / Correctness**: `owned_ledger` maps an absent ledger to `no_usable_ledger`, but `run_end_run` treats absence as success (`topic_command.rs:562`, pinned by `end_run_deletes_only_its_own_ledger_and_tolerates_absence`), and `continued_ledger` checks ownership through `continue_as`. Return `Option` and let each caller decide absence, or narrow the extraction.
- 🟡 **Architecture / Security / Test Coverage / Compatibility**: The composer's new `Bash` tool replaces a structural boundary with a guard that fails open (`--fail-safe --non-blocking`). The citing-once eval fails any composer Bash call (`tasks/eval/citing_once/logs.py:20`, `test_a_composer_that_runs_a_command_fails`), still builds the old inline-template prompt (`sessions.py:37-47`), and `test_the_composer_agent_grants_only_read_and_write` (`:642`) is not listed for rewrite. A composer override without `Bash` has no named outcome.
- 🟡 **Usability / Correctness**: A `templates.topic-research-*` value naming a missing file falls back to the default and exits 0 with only a stderr warning (`store.rs:402-436`), so the pre-check discards it, the Phase 4 manual check cannot pass, and the "template unavailable" row targets the wrong cause.
- 🟡 **Usability**: The template command runs inside the outputter, after research, so a missing template allow rule surfaces only after every researcher in the batch has spent its searches. Run it first, beside the node query.
- 🔵 **Correctness**: `config template` failures carry no `E_` code; "quote its `E_` line" should be "quote its stderr", or Phase 1 adds a code.
- 🔵 **Compatibility**: The plan says spawns run under the project's allow rules, not the skill's; the repo documents and tests the opposite (`research.md:504-506`). Correct the pre-check rationale and the changelog wording, and extend the skill-grant contract test to the template commands and the node query.
- 🔵 **Compatibility / Code Quality**: Replacing the `const fn PendingBatch::new` with a fallible `restored` misses callers (`replaced_run.rs:501`, `window.rs:574`, `memory.rs`), and decoding a malformed `offered_nodes` as corrupt discards 0310's replaced-run report for a rebuildable cache.
- 🔵 **Code Quality**: `PendingBatch::replace` takes two parallel arguments projected from one window; introduce an `Offer` value.
- 🔵 **Code Quality / Test Coverage**: The exact command strings are hand-copied across Rust, Python, skills and docs; read the constants from `confinement.rs` in the Python tests.
- 🔵 **Architecture**: `offered_nodes -> window -> memory -> offered_nodes` module cycle; keep `OfferedNodes` a leaf and build it from `Window`.
- 🔵 **Architecture**: The work item still specifies template-by-path; record the change and amend its criteria.
- 🔵 **Usability**: The `NOT_OFFERED` next step misattributes its cause, and `E_TOPIC_RESEARCH_NODE` and the set-resolution code have no next step.
- 🔵 **Usability**: Step 7 node lines repeat the pair question per node; group by pair.
- 🔵 **Usability**: The manual capture procedure asks `conduct` to continue a hand-started run, which it cannot.
- 🔵 **Test Coverage**: The corrupt-snapshot table omits several listed cases; the template-failure test covers only the finding outputter.
- 🔵 **Security**: Pin that the node query's slug cannot resolve outside the topic-research directory.
- 🔵 **Usability** (suggestion): Consider one `Bash(accelerator config template topic-research *)` rule, relying on the guard for exactness.

### Assessment

The first pass's major findings are all resolved and the plan is markedly stronger: the snapshot invariant is structural, the budget rule has one home, and the guard is a per-role list. But the switch to CLI-printed templates introduced four new major issues. Two are mechanical: `owned_ledger`'s absence rule, and running the template command before research. Two need a design decision: whether the composer should gain `Bash` at all, and how a misconfigured template should be detected given that resolution falls back silently. Those decisions should be settled before implementation.

## Re-Review (Pass 3) — 2026-11-18T23:57:47+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Code Quality / Correctness**: `owned_ledger` breaks `end-run`'s tolerance of absence — Resolved (returns `Option`; each caller decides absence)
- 🟡 **Architecture / Security / Test Coverage / Compatibility**: Composer `Bash` behind a guard that fails open — Resolved (composer stays `Read`/`Write`; template inline from the check)
- 🟡 **Usability / Correctness**: Missing configured template falls back silently — Resolved (check warns and proceeds; non-zero exits stop the run)
- 🟡 **Usability**: Template command runs after research — Resolved (runs first)
- 🔵 **Correctness**: No `E_` code on `config template` failures — Resolved (quote the first stderr line)
- 🔵 **Compatibility**: Permission model mischaracterised — Resolved for rationale, docs and changelog; partially resolved for the contract test (node query not covered)
- 🔵 **Compatibility / Code Quality**: `PendingBatch::new` replaced; malformed snapshot corrupt — Resolved (caller list still slightly wrong; see below)
- 🔵 **Code Quality**: Parallel `replace` arguments — Resolved (`Offer`)
- 🔵 **Code Quality / Test Coverage**: Hand-copied command strings — Resolved (`tests/support/guard_commands.py`)
- 🔵 **Architecture**: Module cycle — Resolved
- 🔵 **Architecture**: Work item still specifies template-by-path — Resolved
- 🔵 **Usability**: `NOT_OFFERED` cause and missing codes — Partially resolved (`E_TOPIC_RESEARCH_RUN` and `E_TOPIC_RESEARCH_UNRESOLVED` unnamed)
- 🔵 **Usability**: Step 7 repeats the pair question — Resolved (grouped; formats now differ from pair lines)
- 🔵 **Usability**: Manual capture procedure — Partially resolved (`--concurrency 1` captures one node per batch)
- 🔵 **Test Coverage**: Corrupt-snapshot table and level-note outcomes — Resolved
- 🔵 **Security**: Slug outside the topic-research directory — Resolved
- 🔵 **Usability**: "superseded" wording for a never-started run — Still present (accepted by user decision)

### New Issues Introduced

- 🔵 **Correctness / Code Quality**: `const fn PendingBatch::new` needs a const `OfferedNodes::empty()`, and the caller list wrongly says `ledger.rs:173` (the decoder) stays on `new`.
- 🔵 **Correctness**: A missing snapshot is not re-snapshotted within the same `conduct` run: `--spawned` moves the affected nodes to `attempted`. Only a fresh `conduct` recovers; the migration note overstates in-run recovery.
- 🔵 **Architecture / Code Quality**: A malformed snapshot is discarded silently; add "an unreadable snapshot" to the `NOT_OFFERED` causes and isolate the lenient decode in a named helper.
- 🔵 **Architecture / Test Coverage / Compatibility**: The template check matches the CLI's uncoded fallback-warning prose, which no test pins; relay any zero-exit stderr line as a warning, or pin the text on both sides.
- 🔵 **Correctness**: The check must separate the fenced template from the stderr warning in the combined tool output before injecting it into compose prompts.
- 🔵 **Test Coverage**: The check's structure test does not pin that the finding command's output is kept, nor that the warning reaches the summary.
- 🔵 **Test Coverage**: Nothing ties the citing-once eval's hand-built compose prompt to the compose bullet.
- 🔵 **Architecture / Code Quality**: The finding outputter's two-path Template step gives a template-less composer no outcome, leaks the given-template path into the level-note outputter, and duplicates the run-first ordering rule; part of its Shape paragraph (`:36-37`) is left stale.
- 🔵 **Compatibility**: The skill-grant contract test covers the template commands but not the node query, and needs a `rule_covers` glob helper.
- 🔵 **Usability**: The template check has no outcome for its own permission denial (a `deny` rule blocks the orchestrator first).
- 🔵 **Usability**: Manual step 4 (remove allow rules → denial) contradicts the inheritance model.
- 🔵 **Usability / Correctness**: The "node query failed" row omits `E_TOPIC_RESEARCH_RUN`, leaves `E_TOPIC_RESEARCH_UNRESOLVED` unnamed, gives `SUPERSEDED` no override check, and maps an unreadable ledger to a code the query never emits.
- 🔵 **Usability / Correctness**: Step 7's grouped node lines differ from pair lines in the same Halt list, and the pair question's source plan is unspecified when the tick plan is unavailable.
- 🔵 **Usability / Correctness**: The `--concurrency 1` capture procedure verifies one node per batch; capture once per batch for every node.
- 🔵 **Security**: The docs should say that the inherited skill grants are broad and the guard narrows them; pin `config set`, `config templates eject` and `topic end-run` by name in the guard tables.
- 🔵 **Usability**: The "template unavailable" row omits the config-file error family.
- 🔵 **Code Quality / Architecture** (suggestions): Single owner for the stray-context filter (`OfferedNodes::retained_for`, or `restored(number, Offer)`); `permitted_command` should match across whitespace and return the variant; consistent constant names; one-bad-entry row in the malformed-snapshot table.

### Assessment

All four pass-2 major findings are resolved, and no lens raised a major or critical finding. The design is now sound: the composer keeps its structural boundary, the snapshot invariant has one owner, the guard is a per-role list, and template faults are caught before any spawn. What remains is minor precision work: one compile-level detail (`const` construction), a migration note that overstates in-run recovery, tighter handling of the template check's combined output, and several troubleshooting rows and manual steps to align with the inheritance model. The plan is acceptable for implementation, and these are best folded in before or during Phase 2 and Phase 4.


## Approval — 2026-11-19T00:21:26+00:00

**Verdict:** APPROVE

The minor findings from pass 3 were folded into the plan and work item. The plan is approved for implementation.
