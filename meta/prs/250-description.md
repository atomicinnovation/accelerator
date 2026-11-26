---
type: "pr-description"
id: "250"
title: "[PP-893] Pass conduct's spawn context by reference"
date: "2026-11-19T19:33:56+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-893"
parent: "work-item:PP-893"
relates_to: ["work-item:PP-867", "work-item:PP-889", "work-item:PP-890", "work-item:PP-894", "work-item:PP-896"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/250"
pr_number: 250
tags: ["research", "deep-research", "conduct", "cli", "run-ledger", "research-guard", "config-template"]
revision: "a58251423037ea2e9e147b62e8e722dbd63df894"
repository: "accelerator"
last_updated: "2026-11-26T17:57:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-893] Pass conduct's spawn context by reference

## Summary

`research-topic`'s `conduct` used to write every spawn's whole context into
its Task prompt: the level-note or finding template, the node's question, its
known questions and its search budget. A deep run's prompts grew with every
level. This PR passes that context by reference instead. A research-node
researcher now gets a slug, a run and a spawn ref, and fetches its own context
from a new read-only CLI query. Each researcher also fetches its own template.
Only a composer, which can run no command, still receives its template inline.
The orchestrator now writes the same amount per spawn at every depth. In a
depth-2 smoke run, each node prompt was 1.3 KB at both levels.

#176 has merged, so this PR is now rebased onto `main` and targets it. A
second review added 27 commits, each answering one finding; the PR's
review-summary comment maps findings to commits.

## Changes

- **The research guard gives each role a list of permitted commands**
  (`cli/research/src/confinement.rs`).
  - A command is permitted either `Exactly` or `Prefixed`.
  - The researcher may run the fetch, the new node query, and exactly
    `accelerator config template topic-research --kind finding` and
    `… --kind level-note`. Anything after an exact command is blocked, a
    flag, a chain, a pipe or a redirection alike.
  - The composer may still run nothing.
  - The refusal text is built from the role's list.
  - `ConductTemplate` owns the topic-research template set and its kinds,
    and maps each kind to its guard command. The researcher's list takes a
    command from every `ConductTemplate`, and a test checks that each
    command fetches its own template.
  - No permitted command's text is a prefix of another's, so the list's
    order decides nothing; a test holds this.
  - The node query's prefix keeps its trailing space, so every other
    `topic` subcommand stays blocked, `end-run` and `outstanding --spawned`
    included.
  - The guard now blocks a subagent's call it cannot judge, with
    `E_RESEARCH_GUARD_UNJUDGED`: one with no working directory, or one from
    an agent type other than the plugin's own two names when the config
    cannot be read. A guard binary that cannot start still fails open.
- **Offered-node snapshot in the run ledger.**
  - Each `outstanding` offer records each offered node's profile, question
    and known questions under `pending.offered_nodes`, keyed by spawn ref.
    A ledger with the earlier top-level `offered_nodes` key still decodes.
  - The pending batch owns the snapshot: a node still pending keeps its
    first context, a newly offered node is snapshotted from the tree, and
    acknowledging a batch clears it.
  - A ledger without the key decodes with an empty snapshot. A malformed
    one, an empty profile or question included, decodes as unreadable
    rather than as a corrupt ledger, which would lose the run's claims and
    withheld profiles. The next `--run` plan's `warnings` says it was
    discarded.
- **New `accelerator research topic node <slug> --run <run> --node <stem>:<lineage>`.**
  - It prints `question`, `lineage`, `level`, `cap`, `known_questions`,
    `id`, `path`, and `search_budget` for a web node.
  - It answers only for the run's current offered batch and writes nothing.
  - It refuses with an `E_` code for an unknown set, an unreadable config,
    a missing, corrupt or unreadable ledger, a superseded or unknown run, a
    node not offered, or a malformed ref. The three new codes are
    `E_TOPIC_RESEARCH_NODE`, `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` and
    `E_TOPIC_RESEARCH_PROJECT`.
  - The web-node budget rule now lives in the domain (`node_search_budget`),
    shared by the plan and the query.
  - `end-run`, the query, and `outstanding --run` and `--start` share one
    ledger refusal, so an unreadable ledger reports
    `E_TOPIC_RESEARCH_RUN_LEDGER` on every path.
  - The plan and the query build a node's lineage, level, cap, id and path
    with one function (`node_address.rs`), so a note agrees with both.
  - `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` names its cause, from what the
    ledger records: `conduct` moved past the node's batch, the run never
    offered it, the ledger discarded its context, or it was offered before
    the ledger kept contexts. The troubleshooting row gives each its own
    next step.
- **Each outputter fetches its own template.**
  - The inlined **Finding template** and **Level-note template** sections
    leave `research-topic`.
  - The finding and level-note outputters each name their exact template
    command, and a researcher runs it before any search. They carry no
    `allowed-tools`: they are read, not invoked, so the injecting skill's
    grant is what reaches the spawn.
  - Before the first plan, `conduct` runs the finding template command, and
    above depth 1 the level-note one, itself. It stops before any spawn on a
    failure or a permission denial, and keeps the finding template to pass
    inline to each composer. Before each batch holding a compose pair it
    reads the finding template again, and its summary warns when it changed
    mid-run.
  - A failed command is quoted by its error line, the first stderr line
    that is not a warning, since the launcher can write a warning first.
  - `config template` fences a template in a fence longer than any backtick
    run inside it, so a code block in an override cannot close it early.
  - Template resolution moved into the `config` crate (`config::template`),
    so `outstanding --start` can name a configured template that falls back
    to its default among its `warnings`. `ResolvedTemplate` now carries the
    fallback as a `TemplateFallback` value holding only the configured path,
    and the launcher and `conduct` each write their whole sentence from it.
    A missing kind-specific path is kept as the fallback when the general
    template answers. `--start` leaves out a template it cannot resolve at
    all, because `conduct`'s own template check stops on that.
- **Node context leaves the plan.**
  - `outstanding`'s node entries no longer carry `question` or
    `known_questions`.
  - Step 4's node bullet injects only references, plus one fixed line that
    names the node query and calls its question data.
  - The level-note outputter, which every node researcher reads, holds the
    query step and its two failures, "node query denied by permissions" and
    "node query failed". `agents/researcher.md` runs that step first and
    treats the node question as untrusted.
  - The web profile is the one place that says how `{search budget}`
    resolves, so an `agents.researcher` override that follows its profile
    meets the run's search quota.
  - Step 7's summary groups nodes under their pair's question, because the
    orchestrator no longer sees a node's own question.
- **`research-topic` grants only the config reads it runs.** It had
  `Bash(accelerator config *)`, which every spawn inherited. A researcher
  whose guard could not run could therefore use `config set` to switch off
  confinement for later runs. There is now one rule per read, from
  `config agent` to `config template`, so no spawn can run `config set` or
  `config init`. `Bash(accelerator research topic *)` is likewise one rule
  per subcommand the skill runs, so a subcommand added later reaches no
  spawn unasked. Neither can be narrower than the orchestrator's own needs,
  since one `allowed-tools` list serves both.
- **Docs, troubleshooting and changelog.**
  - `research.md` documents the node query, each role's permitted commands
    and the allow rules for them.
  - It now says the `outstanding` and node query JSON are additive-only
    from the first stable release.
  - It warns that the researcher's project allow rules apply in every
    session. The guard narrows them only for confined researchers.
  - It documents two timing facts. A headless session can lose its grants
    while researchers run, so a second allow-rule block lists every rule
    `research-topic` grants, held to its frontmatter by a test. A composer
    gets the template `conduct` read at the start, while a researcher
    fetches its own when it starts.
  - `conduct`'s troubleshooting table gains one row per failure: six for
    the node query, seven for the template commands, and one for a composer
    given no template.
  - `CHANGELOG.md` lists the node query under Added, the researcher
    contract changes under Changed, and the guard's full boundary and its
    unjudged-call block under Security.
- **Test support.** The topic command tests share one `topic_project`
  module, which runs the binary with the caps removed and
  `ACCELERATOR_PLUGIN_ROOT` set. A Python helper (`tests/support/guard_commands.py`)
  reads the guard's commands from the Rust constants, so the structure tests
  never copy them, and checks which `Bash(…)` allow rules cover a command.
  The lexer differential now runs its whole corpus after both permitted
  prefixes.
- **The GitHub client trusts only the bundled roots.** Copied from the
  visualisation-system work: octocrab gains `rustls-webpki-tokio`, so it no
  longer reads the macOS host certificate store. That read timed out under
  the test suite's load and failed every `github::octocrab_client` test.
  The bundled-roots guard now covers octocrab too.

### Breaking changes (unreleased feature)

`conduct` has not shipped in a release yet, so no migration is provided:

- `outstanding`'s node entries drop `question` and `known_questions`.
- An `agents.researcher` override must follow the level-note outputter's
  node query step when given a spawn ref, and run its outputter's template
  command. An `agents.composer` override still receives the template inline.
- `research-topic`'s spawns inherit its grants. A skill that spawns
  researchers without them needs `Bash(accelerator research topic node *)`
  and the two exact template-command rules.
- The guard blocks every non-default subagent's `Bash` and write calls
  while the config cannot be read.

## Context

- Work item: `meta/work/PP-893-pass-conduct-spawn-context-by-reference.md`
  (done; synced to Linear)
- Plan: `meta/plans/2026-11-18-PP-893-pass-conduct-spawn-context-by-reference.md`.
  Each phase records its manual checks and any deviation.
- Research: `meta/research/codebase/2026-11-18-PP-893-pass-conduct-spawn-context-by-reference.md`
- Validation: `meta/validations/2026-11-18-PP-893-pass-conduct-spawn-context-by-reference-validation.md`
  (result: pass)

## Testing

- [x] At the head after the second review, rebased onto `main`, the full
      `mise run` passes. `test:unit:cli` passes all 8475 tests, with line
      coverage at 93.81%, and `test:unit:tasks` passes all 6049.
- [x] `topic_node.rs` pins the query against a fixture with literal
      questions, known questions and budgets of 8, 5 and 3. It covers:
      - the web and openalex key sets;
      - a budget equal to the plan entry's;
      - a mid-batch sibling note leaving `known_questions` unchanged;
      - a repeat plan keeping a pending node's first context;
      - a level-3 node seeing its parent's follow-up;
      - a node from an earlier batch refused;
      - a re-issued spawn answering identically;
      - nodes withheld after a search halt refused;
      - each refusal's `E_` code, one test per refusal, a malformed run id
        included;
      - the query's address matching its plan entry's;
      - the query writing nothing.
- [x] Domain and adapter tests cover the snapshot:
      - carry-over and acknowledgement;
      - withheld and pair spawns never snapshotted;
      - the concurrency-narrowed offer;
      - a legacy ledger decoding with an empty snapshot;
      - every malformed snapshot case.
- [x] Guard tests cover each role's commands, an exact template command
      refusing any suffix or a trailing carriage return, every other
      `topic` subcommand staying blocked, a tab after `topic node`, the
      permitted commands not overlapping, and chaining blocked after both
      prefixes. Black-box tests block a subagent call with an unparseable
      config or no working directory.
- [x] Second-review tests also cover: a parent note rewritten mid-batch,
      a test that fails if `replace` prefers the new context (checked by
      mutation); each `E_TOPIC_RESEARCH_NODE_NOT_OFFERED` cause; the
      discarded-snapshot warning; `E_TOPIC_RESEARCH_PROJECT`; a
      kind-specific template fallback; a template found in no tier; and a
      template holding a code block.
- [x] Structure tests read each spawn bullet's injected items, and check the
      agent, outputter, profile, troubleshooting and docs prose against the
      work item's acceptance criteria.
- [x] Manual, each a headless `claude -p` session against debug builds:
      - a depth-2 `conduct` over one web pair. Each node prompt held no
        question, known questions, template or budget. Each note's `id`,
        path, question and `search_budget` matched its node query, and no
        note repeated a known question;
      - a finding template override outside the project, followed with no
        extra allow rule;
      - a level-note template path naming a missing file, reported as a
        fallback warning before the first spawn;
      - an unreadable template override, where `conduct` reported
        "template unavailable" and spawned nothing.
- [x] Manual: with `deny` rules for exactly the two template commands,
      a headless `conduct` printed "template command denied by
      permissions" with its next step, spawned nothing, and wrote no ledger.
      A `deny` on `Bash(accelerator config template *)` instead stops
      `research-topic` from loading, since its `!` preprocessor injects
      its own templates through the same command.

## Notes for Reviewers

- **Rebased onto `main` again after the second review.** `main` had
  dropped the zod advisory ignore this branch re-dated, so that commit is
  gone. Where the branch's Linear syncs conflicted, `main`'s newer
  `last-sync.json` was kept. For the PP-893 work item, the branch's content
  was kept over `main`'s lossy Linear round trip of it.
- **Rebased onto `main`.** #176 has merged, so the diff holds only PP-893's
  commits. They include the work item, research, plan, review and
  validation documents under `meta/`. `main` now resolves research profiles
  from `ACCELERATOR_PLUGIN_ROOT` instead of `--profiles-dir`, and the
  rebase moved this PR's tests onto that.
- **First review addressed.** `meta/reviews/prs/250-review-1.md` records
  it. The commits after it each answer one finding.
- **Second review addressed.** It was made at `673863a` with a review
  page. The 27 commits after "Add the PR #250 description" each answer one
  finding, and the PR's review-summary comment maps them. A 28th commit
  carries the GitHub client's trust-roots fix.
- **Where to look hardest:**
  - `command_decision` in `confinement.rs`, the security boundary for what
    a confined subagent may run;
  - `PendingBatch::replace` and `acknowledge` in `conduct/memory.rs`, which
    keep the snapshot in step with the pending spawns;
  - `offered_nodes_from` in the ledger adapter, the decoder's one lenient
    path;
  - `run_node` in `node_command.rs`, which checks ownership before
    answering, and `RunLedger::offered_context`, which names why a node has
    no context;
  - `Project::confinement` in `guard.rs`, which now blocks a subagent call
    it cannot judge rather than letting it through.
- **Accepted limitations**, recorded in the plan's **What We're NOT Doing**:
  - A still-pending node keeps its first context even if its parent note is
    rewritten mid-batch. Its note is then refused as
    `QuestionDisagreesWithCandidate`, and the rewrite is already reported
    as unexpected.
  - A missing allow rule fails each spawn of the batch, each reported,
    rather than halting the run.
  - A run whose batch was offered by a binary from before the snapshot gets
    `E_TOPIC_RESEARCH_NODE_NOT_OFFERED`, naming that cause. The next
    `conduct` run offers those nodes again.
- **Follow-ups not in this PR:**
  - A headless session can lose `research-topic`'s `allowed-tools` once the
    orchestrator's turn ends while background researchers still run.
    `research.md` documents this, and `draft-01raev`
    (`meta/work/drafts/draft-01raev-keep-conduct-s-grants-through-a-headless-session.md`)
    is the work item for it.
  - Before the rebase, `test:unit:cli`'s cleanup of old build artefacts
    refused once inside a full `mise run`. Listing the build units forced
    22 of them to rebuild, probably because other cargo builds were running
    at the same time. The tests had all passed and the task passes on its
    own, so this is a build-system issue, not part of this change.
