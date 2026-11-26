---
type: "plan-validation"
id: "2026-11-18-PP-893-pass-conduct-spawn-context-by-reference-validation"
title: "Validation Report: Pass Conduct Spawn Context by Reference Implementation Plan"
date: "2026-11-19T18:39:40+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-11-18-PP-893-pass-conduct-spawn-context-by-reference"
tags: ["research", "deep-research", "conduct", "cli", "run-ledger", "research-guard", "config-template"]
last_updated: "2026-11-19T18:39:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Pass Conduct Spawn Context by Reference Implementation Plan

All five phases are implemented as planned, every test the plan names
exists, and the full suite is green. The deviations are small, and the
one that touches an acceptance criterion is recorded in the plan. The
remaining issues are stale wording in the outputters and docs: none is
functional, and the phase 5 smoke run passed despite them.

The implementation spans the commits from `rwxknoymzswu` ("Permit the
researcher its template commands in the research guard") to `zwmkqkxk`
("Drop doc comments that restate offers and the template stub"), 44 files
in all.

### Implementation Status

✓ Phase 1: The guard permits each role its template command - Fully implemented
✓ Phase 2: Offered-node snapshot in the run ledger - Fully implemented
✓ Phase 3: The node query and the guard - Fully implemented
✓ Phase 4: Each outputter fetches its own template - Fully implemented
✓ Phase 5: Node context and search budget by reference - Fully implemented

### Automated Verification Results

✓ Full local CI mirror: `mise run`. Every task passed except
  `test:unit:cli`, whose 8193 tests all passed but whose coverage fell
  below the floor (lines 87.41% against 92%). After
  `cargo llvm-cov clean --workspace`, `mise run test:unit:cli` exits 0 with
  8193 passed and coverage of lines 94.37%, functions 93.80% and regions
  93.12%. The first failure was stale coverage data: the run swept 9.21 GB
  of superseded artefacts.
✓ Format, lint and type checks across all four components, inside `mise run`
✓ `docs:check`, inside `mise run`
✓ Rust integration tests (`topic_node`, `topic_outstanding`, `guard`,
  `confinement`) and public-API snapshots, inside `mise run`
✓ Python structure, guard-command and lexer-differential tests, inside
  `mise run`
✓ Working copy unchanged after the run, so nothing was left unformatted

### Code Review Findings

#### Matches Plan:

- **Phase 1: the guard.**
  - `PermittedCommand`, `FETCH`, `FINDING_TEMPLATE`, `LEVEL_NOTE_TEMPLATE`
    and `Role::permitted_commands` are in
    `cli/research/src/confinement.rs`, and `PERMITTED_PREFIX` is gone.
  - An `Exactly` command must equal the whole command, so trailing
    arguments, chains, pipes and redirections are all refused.
  - The refusal text is built from the role's list, and the composer is
    still blocked with `NoCommands`.
  - `tests/support/guard_commands.py` reads the Rust constants across
    rustfmt rewraps, and `rule_covers` backs the grant tests.
- **Phase 2: the snapshot.**
  - `OfferedNodes`, `NodeContext`, `Offer` and `Window::offer` are added,
    and `PendingBatch` keeps its invariant through private fields.
  - `replace` keeps a still-pending node's first context and drops a
    dropped node's.
  - `acknowledge` clears contexts only on a matching batch number.
  - Withheld and pair spawns are never snapshotted.
  - In the adapter, `offered_nodes_from` is the only lenient decode path.
    One bad entry discards the whole snapshot, and a table test covers
    every malformed case the plan lists.
- **Phase 3: the node query.**
  - `node_search_budget` and `UNSIZED_NODE_BUDGET` live in the domain, and
    the plan and the query share them through `RunLedger::search_budget`.
  - `run_node` (`cli/research-cli/src/node_command.rs`) checks ownership,
    then the offered context, renders the planned keys and writes nothing.
  - `owned_ledger` is shared with `end-run`. Its handling of a corrupt
    ledger is unchanged, and an unreadable ledger now reports with
    `E_TOPIC_RESEARCH_RUN_LEDGER`.
  - `NODE_QUERY` keeps its trailing space, so `topic nodex`, a bare
    `topic node` and every other topic subcommand stay blocked.
  - All 14 planned `topic_node.rs` tests exist, with literal expectations.
  - The lexer differential runs the whole corpus after both prefixes.
- **Phase 4: templates.**
  - The two inlined template sections are gone, and the intro reads "The
    five artifact templates".
  - The template check runs both commands without `--fail-safe` before
    the first plan, keeps only the finding block for compose prompts, and
    stops on a failure or a denial.
  - Each outputter grants its exact command in frontmatter and has a
    **Template** step, and `agents/composer.md` is unchanged.
  - Template resolution moved into `config::template`. Fallbacks are
    reported on a `--start` plan only, and three tests pin that.
- **Phase 5: references.**
  - `PlanOutput::node` drops `question` and `known_questions`.
  - The node bullet injects exactly the planned references plus the fixed
    query line.
  - `agents/researcher.md` runs the query first, states the budget's
    three-way source once, reports both query outcomes and treats the node
    question as data.
  - Step 7 groups nodes under their pair question.
  - Docs and `CHANGELOG.md` match the plan.
- Each phase's manual checks were run and are recorded in the plan. The
  phase 5 smoke run gave 1.3 KB node prompts at both levels, and every
  note's id, path, question and budget matched its query.

#### Deviations from Plan:

- **The template check leaves fallback warnings to the first plan.**
  Work-item AC "reports any other output as a warning while proceeding" is
  instead met by `outstanding --start` naming a fallen-back template among
  its `warnings`. This is recorded in the plan (phase 4, §7) and was
  verified manually.
- **The exit codes have a separate table.** `E_TOPIC_RESEARCH_NODE` and
  `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` are in a new table under the
  `research topic node` docs section
  (`docs-site/src/content/docs/research.md:821-832`), not in the existing
  `outstanding` table.
- **Some CLI guard tests are thinner than planned.**
  - `cli/research-cli/tests/guard.rs` names its composer test
    `the_composer_may_run_no_template_command` and checks only the finding
    template.
  - Its `a_template_command_must_match_exactly` has one row. The full
    table lives in `cli/research/tests/confinement.rs`.
- **Names and signatures differ.** The dispatch function is
  `offered_node`, not `node`. `budgeting` takes no `run` parameter.
  `set_slug` and `owned_ledger` are `pub`, which in a binary crate is the
  same as `pub(crate)`.
- **One structure test was renamed.**
  `test_the_web_profile_limits_searches_to_the_injected_budget` became
  `test_the_web_profile_limits_searches_to_its_search_budget`.
  `test_research_topic_routes_compose_pairs_to_the_composer` needed no
  rewrite.
- **One test is stronger than planned.** The legacy-ledger test
  decodes `well_formed()` plus pending node spawns, which also shows that
  nodes without context are accepted.
- **Changes the plan does not list, all following from planned moves.**
  - The `claude_coupling` lint and its test point at
    `topic_project/mod.rs`, after the move of the shared test helpers.
  - The fetch command's test builds `ProjectContext` with a template
    store.
  - `topic_consultable.rs` uses the shared helpers.

#### Potential Issues:

- **The level-note outputter's Sink contradicts the node query.**
  `skills/research/outputters/level-note-outputter/SKILL.md:25-26` says to
  write "the single note path injected into your task prompt". The node
  bullet no longer injects a path, and the same file's **Injected Values**
  takes the path from the node query. The smoke run wrote every note to
  the queried path, but the stale sentence invites a custom researcher to
  look for a path that is not there.
- **The finding outputter still says "injected question".**
  `skills/research/outputters/finding-outputter/SKILL.md:57-58` makes
  `question` "the injected question byte for byte", and the level-note
  outputter applies that Shape unchanged. A node's question is now
  queried.
- **Two outputter descriptions are stale.** Both frontmatter descriptions
  still say each is "Injected into the researcher", though the composer
  now reads the finding outputter's **Template** step.
- **The guard text names only the fetch.** `confinement.rs:70` and
  `research.md:467` say "The fetch needs neither", but the node query is
  lexed under the same rules. The syntax refusal hint, "pass the query as
  one single-quoted argument" (`confinement.rs:573`), reads oddly after a
  node query.
- **"Granting the commands" says one thing twice.** `research.md:506-508`
  and `:519-521` each say the commands "must each be granted by a
  permission rule that reaches the researcher subagent". It reads as two
  edits stacked rather than merged.
- **One error row does not name the new cause.** The existing
  `E_TOPIC_RESEARCH_RUN_LEDGER` row (`research.md:791`) says "missing,
  corrupt, or cannot be written". `end-run` now also uses that code for an
  unreadable ledger.
- **A guard refusal of a template command reads as "template
  unavailable".** An override that adds `--fail-safe`, for example, is
  refused with `E_RESEARCH_GUARD_COMMAND`, and that troubleshooting row does
  not name the cause. Only the generic `E_*` row covers it.
- **The unreadable-ledger test depends on not running as root.** The row
  in `topic_node.rs:385` relies on mode `0o000`, so it would not fail as
  intended under root.

### Manual Testing Required:

Every manual check in the plan is ticked and its outcome recorded. None
are outstanding. One is worth repeating once phase 4's `allowed-tools`
lapse is fixed:

1. **Inherited grant**:
  - [ ] Re-run the phase 5 smoke run without
        `Bash(accelerator research topic outstanding *)`. Confirm that
        re-plans after a turn boundary no longer stall, and that
        researchers still run the node query on the grant they inherit.

### Resolved after validation:

Every potential issue above but two was fixed after this report was
first written, with new structure tests pinning the outputter wording:

- The level-note outputter's **Sink** names the node query's path, and
  its node-query values take the question in place of the finding
  outputter's injected question
  (`test_a_level_note_takes_its_path_and_question_from_the_node_query`).
- The finding outputter's description names the composer as a reader too
  (`test_the_finding_outputter_names_both_its_readers`).
- Both outputters count a research-guard refusal of the template command
  as a non-zero exit, and the "template unavailable" row names
  `E_RESEARCH_GUARD_COMMAND`
  (`test_a_guard_refusal_of_a_template_command_is_troubleshot`).
- **Granting the commands** is one passage listing each inherited grant.
  "The fetch needs neither" now covers every permitted command, in the
  docs and in `confinement.rs`. The `E_TOPIC_RESEARCH_RUN_LEDGER` row
  names an unreadable ledger.
- Work item PP-893's acceptance criteria are ticked and it is `done`.
  The "reports any other output as a warning" criterion is met through
  the first plan's `warnings`, as the plan records.

Two were left as they are. The syntax refusal hint is a pinned,
user-facing message that still fits the fetch. The root-sensitive test
matches how the suite runs.

### Recommendations:

- Raise the `allowed-tools` lapse noted in phase 4 as its own work item,
  as the plan intends.
- If `test:unit:cli` fails on the coverage floor with every test
  passing, run `cargo llvm-cov clean --workspace` before treating it as
  a regression.
