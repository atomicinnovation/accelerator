---
type: "pr-review"
id: "250-review-1"
title: "[PP-893] Pass conduct's spawn context by reference"
date: "2026-11-19T19:46:48+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:250"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "security", "test-coverage", "code-quality", "correctness", "compatibility", "documentation", "standards"]
review_number: 1
pr_number: 250
tags: []
last_updated: "2026-11-19T19:46:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #250 - [PP-893] Pass conduct's spawn context by reference

**Verdict:** COMMENT

A well-modelled change. The research guard now gives each role an explicit list of permitted commands (`PermittedCommand::Exactly`/`Prefixed`). `PendingBatch` owns the offered-node snapshot as pure domain logic. `node_search_budget` puts the budget rule in one place. The read-only node query is well tested at the domain, adapter and black-box levels. No lens found a critical issue. The findings that matter most are three:
- The node query depends on when the orchestrator acknowledges a batch, and on the grants each spawn inherits. Both used to be incidental and are now load-bearing.
- `outstanding` reports an unreadable ledger without the `E_` code the docs now promise.
- A few contracts are duplicated or now defined in only one place: the node JSON, the template identity and `{search budget}`.

### Cross-Cutting Themes

- **Unreadable ledger reported without its code on `outstanding`** (flagged by: correctness, compatibility, standards, code quality, documentation). `owned_ledger` now returns `E_TOPIC_RESEARCH_RUN_LEDGER: could not read …`, but `continued_ledger` (`topic_command.rs:617`) and `replaced_ledger` (`:727`) still return a bare `error.to_string()`. The `outstanding` exit-code table in `research.md` says this code covers a ledger that "cannot be read".
- **A node's JSON is rendered in two places** (flagged by: architecture, code quality). `run_node` and `PlanOutput::node` each build `lineage`/`level`/`cap`/`id`/`path` their own way, although a note must agree with both.
- **The conduct template identity is written out in several layers** (flagged by: architecture, code quality). `CONDUCT_TEMPLATE_SET`/`KINDS` in `context.rs`, the `FINDING_TEMPLATE`/`LEVEL_NOTE_TEMPLATE` guard constants, the outputters' `allowed-tools` and the skill prose all name the same templates, and nothing ties them together.
- **`ProjectContext` holds conduct-specific template policy and wording** (flagged by: architecture, code quality, standards). It strips the launcher's `"Warning: "` prefix, builds the warning text outside the `*_wording` modules, and builds a template store for every command.
- **Researchers now depend on what happens around them at run time** (flagged by: architecture, compatibility, security). Every node researcher must run the node query and its template command before it writes anything. So when the orchestrator acknowledges the batch, the grants each spawn inherits, and the known problem of headless sessions losing their grants all decide whether a node succeeds at all. Before, they only decided whether a note was degraded.
- **The `research topic node` entry is in the wrong CHANGELOG section** (flagged by: documentation, standards). It sits under Changed; the other new `research topic` subcommands are under Added.
- **Very long troubleshooting cells** (flagged by: code quality, documentation). The "node query failed" and "template unavailable" rows each pack many codes and their next steps into one table cell.
- **Composers and researchers get the finding template at different times** (flagged by: architecture, correctness). Composers get the copy taken at the start of the run; researchers resolve it when they spawn. Findings in one set can therefore differ in shape if config changes mid-run.

### Tradeoff Analysis

- **Least privilege vs operability**: the security lens wants the node-query allow rule kept off project-wide settings (`Bash(… topic node *)` applies to sessions the guard does not cover). The compatibility lens wants those same rules documented as the workaround for headless runs that lose their grants. Recommendation: keep the rules in the docs, but say they are a fallback for headless runs and that they apply to every session.
- **Lenient snapshot decode vs failing loudly**: decoding a malformed `offered_nodes` as empty keeps the run's claims, at the cost of an `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` that is harder to diagnose. Every lens accepted this trade-off as deliberate and documented.

### Strengths

- ✅ `PermittedCommand` (`Exactly`/`Prefixed`) with `decide` and `Display` turns the guard into a list of permitted commands per role. The refusal text is built from the same list, a composer's empty list means `NoCommands`, and exact template commands refuse any suffix.
- ✅ The node-query prefix keeps its trailing space, so every other `topic` subcommand stays blocked, including `end-run`, `outstanding --spawned` and the near-miss `nodex`. Tests pin this.
- ✅ `PendingBatch` owns the snapshot as pure functions. `replace` keeps the first context of a node that is still pending, `Offer::new` drops contexts outside the offer, and `acknowledge` clears the snapshot, so it can never outlive the pending spawns.
- ✅ `node_search_budget` puts the web-node budget rule in the domain, so the plan and the query cannot drift apart.
- ✅ The node query writes nothing, checks the run owns the ledger, and answers only for the current offered batch. A tampered snapshot cannot change what a researcher is told. Node questions no longer pass through the orchestrator's context, and the researcher treats them as data.
- ✅ Old and new ledgers stay compatible: a ledger with no `offered_nodes`, or a malformed one, decodes with an empty snapshot and keeps the run's claims and withheld profiles. The comment explaining why this decode is lenient is a good use of a comment.
- ✅ Template resolution moved cleanly into `config::template`, re-exported by the launcher, with both public-API fixtures updated.
- ✅ `owned_ledger` gives `end-run` and the node query one ownership check, and the shared refusals moved into `ledger_wording`.
- ✅ Thorough tests: the guard is tested at unit, hook and lexer-differential level across both prefixes; 14 malformed snapshot shapes are tested; full JSON answers are asserted for web and openalex nodes; and the query is checked to write nothing by comparing the whole directory tree.
- ✅ `tests/support/guard_commands.py` reads the guard's commands from the Rust constants, so the Python tests never copy them.
- ✅ The docs gain a complete reference for `research topic node` and an honest section on least-privilege allow rules.

### General Findings

- 🟡 **Compatibility**: Researchers now need inherited Bash grants to write anything, so the headless grant-loss behaviour is now fatal. Before, a denied optional `fetch` only left out metadata. Now a denied node query or template command fails every node in the batch. Raise the follow-up before this lands, or document the workaround of adding the four allow rules for headless runs.
- 🔵 **Security**: The recommended project rule `Bash(accelerator research topic node *)` also applies to sessions the guard does not confine. Claude Code's lenient wildcard matching then lets `$VAR` expansion and `<` redirection through. Warn about this next to the JSON, or prefer skill-scoped grants. (re-anchored from an out-of-range line to `research.md:535`)
- 🔵 **Security**: A researcher must now reach the broad `Bash(accelerator config *)` / `Bash(accelerator research topic *)` grants to do normal work, and only a guard that lets calls through when it cannot run narrows them. If the guard is down, a prompt-injected researcher could run `config set agents.researcher` and turn off confinement for later runs. Consider making the guard refuse calls for confined agent types when it cannot resolve its own config. (re-anchored from an out-of-range line to `research.md:517-518`)
- 🔵 **Architecture / Correctness**: Composers receive the finding template captured once at the start of the run, while researchers resolve it when they spawn. Accept and document this, or record the resolved template's digest in the ledger.
- 🔵 **Test Coverage**: Nothing executes the step where `conduct` stops when the template command is denied; the PR's one unchecked manual item covers it. Run it before merging.
- 🔵 **Security**: The outputters' `allowed-tools` frontmatter probably grants nothing, because the files are read rather than invoked. In practice the template commands run under the broad inherited `config *` grant. Say so in the docs.
- 🔵 **Test Coverage**: Structure tests pin many sentences of prose word for word, and the order of doc lines. Prefer key-phrase assertions where the intent is "the concept is stated once".
- 🔵 **Code Quality / Documentation**: The "node query failed" and "template unavailable" troubleshooting cells are long decision procedures. Consider one row per code, or a sub-list.

### Additional Findings

- 🔵 `cli/research-cli/src/context.rs:56` — swallowed resolution errors in `conduct_template_fallbacks` are untested (test-coverage)
- 🔵 `skills/research/research-topic/SKILL.md:620` — the Halt notice mixes the `{pair question}` and `{question}` placeholders (documentation)
- 🔵 `skills/research/outputters/level-note-outputter/SKILL.md:49` — the template-failure contract is repeated word for word in both outputters (code-quality)
- 🔵 `skills/research/research-topic/SKILL.md:471` — a line injected "verbatim" still contains `<slug>`/`<run>`/`<spawn ref>` placeholders (correctness)
- 🔵 `cli/research-cli/tests/topic_node.rs:349` — one sequential test covers nine refusal scenarios (test-coverage)
- 🔵 `cli/research/src/conduct/memory.rs:42` — `PendingBatch::new` is public but only tests use it (code-quality)
- 🔵 `cli/research/src/confinement.rs:7` — the module doc's `Bash(… *)` matching sentence now applies only to prefixed commands (documentation)
- 🔵 `cli/research-adapters/src/conduct/ledger.rs:150` — the snapshot is persisted as a sibling of `pending`, not inside it (architecture)
- 🔵 `cli/research/src/conduct/ledger.rs:753` — a test function name runs past 80 columns (standards)
- 🔵 `skills/research/research-topic/SKILL.md:327` — an edited paragraph was not reflowed (standards)
- 🔵 `skills/research/research-topic/SKILL.md:680` — the "node query failed" troubleshooting row is very dense (documentation)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/research/src/conduct/memory.rs:68` — Researcher context now depends on the orchestrator's acknowledgement timing
**Severity**: major | **Confidence**: medium | **Lens**: architecture

🟡 **Architecture**

Acknowledging a batch (`--spawned N`) now clears every offered node's context. Before this change, each researcher carried its whole context in its prompt and did not depend on the ledger's lifecycle. Now a researcher that runs `accelerator research topic node` after the orchestrator reaches step 6 (`--spawned`), or after `end-run` deletes the ledger, gets `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` or `E_TOPIC_RESEARCH_RUN_LEDGER`. Correctness therefore rests on one line of SKILL.md prose: "each waited on in full before the next step". The PR itself notes that researchers can still be running in the background after the orchestrator's turn ends.

**Impact**: If the harness runs subagents in the background, or a custom orchestrator acknowledges early, every node that starts late fails. The failures look like ledger or override faults rather than an ordering problem.

**Suggestion**: Make the lifetime explicit. One option is to keep the acknowledged batch's contexts until the next `replace` rather than clearing them on `acknowledge`. Another is to have the node query's refusal name the ordering cause. At minimum, record the ordering invariant in the troubleshooting row for `E_TOPIC_RESEARCH_NODE_NOT_OFFERED`.

---

### `skills/research/profiles/web-profile/SKILL.md:77-78` — {search budget} is now defined only in the default researcher agent
**Severity**: major | **Confidence**: medium | **Lens**: documentation

🟡 **Documentation**

web-profile used to define `{search budget}` itself and to say what to do with no budget. Both rules now live only in `agents/researcher.md`'s new Search Budget section, and level-note-outputter also defers to "your researcher definition". An `agents.researcher` override is a supported configuration. A custom researcher that reads web-profile now finds an undefined placeholder and an unexplained "when you have a search budget" condition. Neither the CHANGELOG override notes nor research.md mention this new obligation.

**Impact**: A custom researcher may ignore the queried budget, or invent its own limit. The search-quota sizing the run relies on then breaks without any error.

**Suggestion**: Keep a one-line definition in web-profile, for example "{search budget} is the node query's `search_budget`, else the injected search budget". Or add the requirement to the CHANGELOG's `agents.researcher` override note and to research.md.

---

### `cli/research-cli/src/topic_command.rs:617` — Unreadable ledger still reported without its E_ code on `outstanding` (Correctness, Compatibility, Standards, Code Quality, Documentation)
**Severity**: minor | **Confidence**: high | **Lens**: correctness, compatibility, standards, code-quality, documentation

🔵 **Correctness**: `owned_ledger`, a few lines above, now turns `LedgerError::Unreadable` into `E_TOPIC_RESEARCH_RUN_LEDGER: could not read <path>: …`. This arm, on the continue path `conduct` takes every batch, still returns the raw `StoreError` text with no code, and so does `replaced_ledger` (line 727). The same failure is now reported two ways depending on which subcommand hits it.

🔵 **Documentation / Compatibility**: the `outstanding` exit-code table in `research.md` (line 790) now says `E_TOPIC_RESEARCH_RUN_LEDGER` covers a ledger that "cannot be read", but `outstanding --run`/`--start` never emits that code for a read failure. A consumer keying on the documented code, such as `conduct`'s troubleshooting rows or an override, won't match it.

**Suggestion**: Move the "could not read" wording into `ledger_wording`, next to `superseded` and `no_usable_ledger`, and use it in all three readers. Otherwise, narrow the doc row to the commands that actually emit the code.

---

### `CHANGELOG.md:874-881` — New `research topic node` subcommand is listed under Changed, not Added (Standards, Documentation)
**Severity**: minor | **Confidence**: high | **Lens**: standards, documentation

🔵 **Standards**: every other new `research topic` subcommand (`outstanding`, `report-path`, `counts`, `consultable`) is listed under `### Added`. This one sits under `### Changed`, so readers scanning Added for new commands will miss it.

🔵 **Documentation**: the entries that drop `question`/`known_questions` from `outstanding` (researchers fetching their own templates and querying their own context) sit under Added, though they change existing behaviour. The placement looks swapped.

**Suggestion**: Move the `topic node` bullet into Added, and the two entries that remove `outstanding` fields into Changed. If prerelease users matter, also add a short Breaking note.

---

### `cli/research-cli/src/main.rs:202-204` — No test of a malformed --run for the node query
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The new `TopicAction::Node` dispatch validates `--run` with `topic_command::parse_run` before it parses `--node`. Every `topic_node.rs` case passes a well-formed run id. The researcher prose and the troubleshooting row list `E_TOPIC_RESEARCH_RUN` among the node query's refusals, but no black-box test produces it from `topic node`.

**Impact**: A dispatch change that skipped or reordered run validation for `node` would go unnoticed.

**Suggestion**: Add `project.node("bad run!", "03-a-web:2-1")` and assert the exact `E_TOPIC_RESEARCH_RUN: --run must be …` message. Optionally add a case where both flags are malformed, to pin which refusal wins.

---

### `cli/research-cli/src/node_command.rs:51-60` — The node JSON contract has two independent renderers (Architecture, Code Quality)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`run_node` builds `lineage`, `level`, `cap`, `id` and `path` itself. `PlanOutput::node` in `topic_command.rs` builds the same fields its own way: `follow_up_cap(level)` here against `node.cap()` there, and `set_root.join(at.path())` against `self.absolute(&address.path())`. A researcher's note must match the plan entry it is checked against, so these two outputs are one contract with two implementations.

**Impact**: A change to how a node's cap, id or path is derived has to be made in both places. A divergence would show up only as notes refused when the plan is re-run.

**Suggestion**: Extract one function that renders a node's addressing fields from a `NoteRef` and the set root, for example `node_address_fields(set_root, at)`. Use it from both `PlanOutput::node` and `run_node`, each adding only its own extra keys.

---

### `cli/research-cli/src/context.rs:20-22` — The conduct template identity is written out in several layers (Architecture, Code Quality)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

The template set `topic-research` and the kinds `finding`/`level-note` are written out separately in four places: here (`CONDUCT_TEMPLATE_SET`/`CONDUCT_TEMPLATE_KINDS`), in `research::confinement` (`FINDING_TEMPLATE`/`LEVEL_NOTE_TEMPLATE` as full command strings), in each outputter's `allowed-tools`, and in the `research-topic` SKILL.md. Nothing ties `CONDUCT_TEMPLATE_KINDS` to the guard's permitted commands.

**Impact**: Adding or renaming a template kind means coordinated edits across two crates and three skills. A missed edit shows up only at run time, as a guard refusal or as a fallback warning that never appears.

**Suggestion**: Give the domain one source, for example a `ConductTemplate` enum or a const list in `research::conduct`. Derive both the guard's `Exactly` commands and the fallback-warning loop from it, and assert in a structure test that the outputters' `allowed-tools` match it.

---

### `cli/research-cli/src/context.rs:48-74` — ProjectContext takes on conduct-specific template policy and wording (Architecture, Code Quality, Standards)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality, standards

🔵 **Architecture**: `ProjectContext` is shared by every research command. It now builds a `ReadTemplate` store up front and hosts `conduct_template_fallbacks`, which is conduct-only policy. `fetch` and every other command therefore build a store they never use.

🔵 **Code Quality**: the method strips a literal `"Warning: "` prefix that `config-adapters/src/store.rs` formats for display. If that wording changes, the plan's `warnings` silently get a doubled or missing prefix.

🔵 **Standards**: every other plan warning is worded in a `*_wording` module (`ledger_wording`, `search_wording`, `concurrency_wording`). This one is built inline with `format!`.

**Suggestion**: Have `ResolvedTemplate` carry the fallback as a structured value, not a display string. Move `conduct_template_fallbacks` to the conduct/outstanding path as a free function over `&dyn ConfigAccess` and `&dyn ReadTemplate`, and build the notice in a wording function.

---

### `cli/research-cli/src/context.rs:56-66` — Swallowed resolution errors in conduct_template_fallbacks are untested
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`conduct_template_fallbacks` drops both a resolution `Err` and an `Ok(None)` with `.ok()??`. The doc comment presents this as deliberate: `--start` must still succeed. All three fallback tests set `ACCELERATOR_PLUGIN_ROOT` and use a missing configured path, so they only ever reach the `warning` branch.

**Impact**: Turning `.ok()??` into `?`, which would make a template problem fail `outstanding --start`, would pass every current test.

**Suggestion**: Add a `topic_outstanding.rs` case with `ACCELERATOR_PLUGIN_ROOT` removed (or pointed at an empty directory), asserting that `--start` exits 0 and `warnings` holds no template entry.

---

### `docs-site/src/content/docs/research.md:535` — The recommended project rule for the node query also applies to sessions the guard does not cover
**Severity**: minor | **Confidence**: medium | **Lens**: security

🔵 **Security**

The docs tell users to add `Bash(accelerator research topic node *)` to the project's allow rules. Project rules apply to every session, including the main conversation and any agent the research guard does not confine. This page itself says Claude Code's wildcard matching accepts `$VAR` expansion and `<` redirection, which is why the guard is stricter. Outside the guard, `accelerator research topic node "$SECRET" --run r --node x` echoes the secret back in `E_TOPIC_RESEARCH_UNRESOLVED` without asking. The fetch rule already has this exposure; this adds a second wildcard rule with it.

**Suggestion**: Say this rule is needed only where `research-topic`'s grant does not reach the spawn, and prefer a skill-scoped grant. At least warn next to the JSON that the rule applies to every session, not only to confined researchers.

---

## Per-Lens Results

### Architecture

**Summary**: The PR changes spawn context from inline prompt text to a reference, backed by a ledger snapshot and a read-only query, and the domain modelling is sound. The main structural cost is new timing coupling: a researcher's context is valid only until the orchestrator acknowledges the batch. Smaller concerns are the template identity repeated across layers and the two renderers of the node JSON.

**Strengths**: the snapshot is modelled in the domain and owned by `PendingBatch`; `node_search_budget` is shared; the guard's role-indexed list follows the open–closed principle; template resolution moved into the `config` library; the `owned_ledger` read is shared; the lenient decode is deliberate and documented; the Python helper keeps the Rust constants the single source.

**Comments**:
- `cli/research/src/conduct/memory.rs:68` (major, medium) — researcher context now depends on when the batch is acknowledged.
- `cli/research-cli/src/context.rs:21-22` (minor, high) — the conduct template identity is duplicated across four layers.
- `cli/research-cli/src/context.rs:48-74` (minor, medium) — `ProjectContext` takes on conduct-specific template policy.
- `cli/research-cli/src/node_command.rs:51-60` (minor, medium) — the node JSON contract has two independent renderers.
- `cli/research-adapters/src/conduct/ledger.rs:150` (suggestion, low) — the snapshot is persisted outside the pending batch that owns it.

**General Findings**:
- (minor) Composers and researchers get the finding template from different sources, so findings in one run can differ in shape if config changes mid-run.

### Security

**Summary**: The guard change is sound. Exact template commands are compared byte for byte, the node-query prefix keeps its trailing space, the lexer covers both prefixes, and the composer still runs nothing. The node query only reads and is narrowly scoped. The remaining concerns are about defence in depth around broad inherited grants and a guard that lets calls through when it cannot run.

**Strengths**: exact matching of the template commands; the prefix boundary pinned against `end-run`/`--spawned`/`nodex`; an explicit allow-list per role; the lexer differential runs across both prefixes; the node query reads only, checks ownership and is scoped to the current batch; node questions no longer pass through the orchestrator and are treated as data; the docs are candid about broad grants.

**Comments**:
- `docs-site/src/content/docs/research.md` (reported at 4463-4468, outside the diff; re-anchored to 535) (minor, medium) — the project-wide node-query rule applies to sessions the guard does not cover.
- `docs-site/src/content/docs/research.md` (reported at 4449-4454, outside the diff; moved to general findings, nearest relevant 517-518) (minor, medium) — the researcher now needs the broad `config *`/`topic *` grants for normal work, and only a guard that lets calls through narrows them.

**General Findings**:
- (suggestion) The outputters' `allowed-tools` frontmatter probably grants nothing.

### Test Coverage

**Summary**: Test coverage is strong and matches the risk. The guard, the snapshot and the node query each have targeted tests with exact assertions. The gaps are small: the swallowed-error branch, the wiring of `--run` validation, and one long scenario test.

**Strengths**: `command_decision` is tested from every side; 14 malformed snapshot shapes are tested; each domain snapshot rule has its own test; full JSON answers are asserted and the query is checked to write nothing; the Python helper is self-tested; the moved template tests kept their safety net; there are negative tests for fallback warnings.

**Comments**:
- `cli/research-cli/src/context.rs:56-66` (minor, medium) — swallowed resolution errors are untested.
- `cli/research-cli/src/main.rs:202-204` (minor, high) — no test of a malformed `--run` for the node query.
- `cli/research-cli/tests/topic_node.rs:349-398` (suggestion, medium) — one sequential test covers nine refusal scenarios.

**General Findings**:
- (suggestion) Structure tests pin many sentences word for word, and the order of doc lines.
- (minor) No test confirms `conduct` stops when the template command is denied (the manual item is unchecked).

### Code Quality

**Summary**: The domain changes are well modelled and easy to follow. The remaining issues are small maintainability risks: duplicated rendering and identity, inconsistent reporting of ledger errors, fallback warnings that depend on a display string, and the outputters repeating the same contract.

**Strengths**: `PermittedCommand` removes the role branching; `PendingBatch::replace` is readable; the lenient decode is commented with its reason and fully tested; the ledger wording is centralised; `node_search_budget` and `UNSIZED_NODE_BUDGET` name the rule; the Python helpers read the Rust constants.

**Comments**:
- `cli/research-cli/src/node_command.rs:52-60` (minor, medium) — node JSON fields rendered in two places.
- `cli/research-cli/src/context.rs:20-21` (minor, medium) — template identity duplicated between `context.rs` and `confinement.rs`.
- `cli/research-cli/src/context.rs:60-70` (minor, medium) — fallback warnings depend on a display string and drop errors silently.
- `cli/research-cli/src/topic_command.rs:617` (minor, high) — `continued_ledger` still reports an unreadable ledger without its `E_` code.
- `cli/research/src/conduct/memory.rs:42-59` (suggestion, medium) — `PendingBatch` has two public constructors, and only tests use `new`.
- `skills/research/outputters/level-note-outputter/SKILL.md:49-58` (minor, medium) — the template-failure contract is repeated word for word in both outputters.

**General Findings**:
- (suggestion) The troubleshooting rows have become dense decision procedures.

### Correctness

**Summary**: The core logic holds up. The guard's lists keep the old fetch behaviour and add matches that cannot overlap. `replace` and `acknowledge` keep the snapshot in step with the pending spawns. The node query derives the same budget, cap, id and path as the plan. The issues found are small: the unreadable-ledger code on `outstanding --run`, and two contract wordings that can be read more than one way.

**Strengths**: `node_search_budget` gives the same answer on both paths; `acknowledge` clears the snapshot atomically; the `find_map` over non-overlapping matches is safe; `run_node` uses the same derivations as the plan; the fallback warning uses the same resolve path as the `config template` command.

**Comments**:
- `docs-site/src/content/docs/research.md:790` (minor, high) — the `outstanding` exit-code table promises `E_TOPIC_RESEARCH_RUN_LEDGER` for an unreadable ledger.
- `cli/research-cli/src/topic_command.rs:617` (minor, high) — `owned_ledger` and `continued_ledger` now handle an unreadable ledger differently.
- `skills/research/research-topic/SKILL.md:471-473` (minor, low) — a line injected "verbatim" still contains placeholders.

**General Findings**:
- (suggestion) Composers and researchers take the finding template at different times.

### Compatibility

**Summary**: The breaking changes to the `outstanding` JSON and the researcher contract are treated as safe because `conduct` is unreleased, and the internal Rust API changes have no consumers outside the workspace. Ledgers stay compatible. The risks are the mismatch with the documented exit codes, researchers' heavier reliance on inherited grants, and no Breaking-section notice for prerelease users.

**Strengths**: old and new ledgers stay compatible; the launcher re-exports the moved template API and the API fixtures are updated; the exact-match commands follow Claude Code's semantics; the fallback warnings only add entries.

**Comments**:
- `cli/research-cli/src/topic_command.rs:617` (minor, high) — `outstanding --run` still returns an error without its code, contrary to the updated exit-code table.
- `docs-site/src/content/docs/research.md:651-653` (minor, medium) — the documented additive-only JSON guarantee is weakened after the fact.

**General Findings**:
- (minor) The researcher-override and `outstanding`-JSON breaks are not listed under the CHANGELOG's Breaking section, although prerelease builds already ship `conduct`.
- (major) Researchers now need inherited Bash grants to write anything, which exposes them to the known headless grant-loss behaviour.

### Documentation

**Summary**: The documentation keeps up with the change: a full node-query reference, honest least-privilege rules, CHANGELOG notes for overrides, and new troubleshooting rows. There are accuracy gaps: `{search budget}` is now defined only in the default agent, one exit-code row overstates what the code does, and the placeholders and CHANGELOG placement are inconsistent.

**Strengths**: a self-contained `research topic node` reference; candid notes on grants; migration notes for overrides; the lenient-decode comment explains why; the troubleshooting strings match the agent and outputter wording.

**Comments**:
- `skills/research/profiles/web-profile/SKILL.md:77-78` (major, medium) — `{search budget}` is now defined only in the default researcher agent.
- `docs-site/src/content/docs/research.md:790` (minor, medium) — `E_TOPIC_RESEARCH_RUN_LEDGER` is now claimed for "cannot be read" on `outstanding`.
- `skills/research/research-topic/SKILL.md:620-623` (minor, medium) — the Halt notice mixes the `{pair question}` and `{question}` placeholders.
- `cli/research/src/confinement.rs:7-8` (suggestion, low) — the module doc's `Bash(… *)` matching sentence now applies only to prefixed commands.
- `CHANGELOG.md:874-881` (suggestion, medium) — the new `research topic node` command is filed under Changed rather than Added.
- `skills/research/research-topic/SKILL.md:680` (suggestion, low) — the "node query failed" troubleshooting row is very dense.

**General Findings**: none.

### Standards

**Summary**: The PR mostly follows the project's conventions: file placement, `E_` code naming, JSON key vocabulary, and a comment policy kept clear of references to work items or plan phases. It has minor inconsistencies in CHANGELOG categorisation, ledger error formatting, where wording lives, and line width.

**Strengths**: new files sit where expected; the new codes follow the flag-named scheme; the JSON keys reuse the plan's vocabulary; `config::template` follows the crate's style and dropped stale comments; the outputters' `allowed-tools` follow the profile pattern; ledger refusals moved into `ledger_wording`.

**Comments**:
- `CHANGELOG.md:874-881` (minor, high) — the new `research topic node` subcommand is listed under Changed, not Added.
- `cli/research-cli/src/topic_command.rs:617` (minor, medium) — an unreadable ledger produces a coded error on one path and an uncoded one on others.
- `cli/research-cli/src/context.rs:70` (suggestion, medium) — the template fallback warning is worded outside the `*_wording` modules.
- `cli/research/src/conduct/ledger.rs:753` (suggestion, high) — a test function name runs past the 80-column limit.
- `skills/research/research-topic/SKILL.md:327-331` (suggestion, high) — an edited paragraph was not reflowed.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
