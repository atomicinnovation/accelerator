---
type: "pr-description"
id: "134"
title: "[0280] Academic source profiles for topic research via OpenAlex and arXiv"
date: "2026-09-24T22:52:35+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0280"
parent: "work-item:0280"
relates_to: ["work-item:0121", "work-item:0282", "work-item:0283", "work-item:0284"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/134"
pr_number: 134
tags: ["research", "sources", "config", "cli", "hooks", "openalex", "arxiv"]
revision: "c6b0d230b7bf3a55e84e52e0b35f78449ca385e8"
repository: "accelerator"
last_updated: "2026-09-24T22:52:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0280] Academic source profiles for topic research via OpenAlex and arXiv

## Summary

Lets `/accelerator:research-topic` research the scholarly literature as well as
the web. Two new source profiles, `openalex` and `arxiv`, reach their sources
through a new deterministic `accelerator research fetch` sub-binary that
derives each record's reputation tier, and a `PreToolUse` guard confines the
researcher's newly granted `Bash` to that command and its writes to finding
files. `conduct`'s unit of work changes from one finding per focus area to one
finding per (focus area, profile), allocated by a new
`accelerator corpus topic-research outstanding` verb rather than by skill prose.

## Changes

### `accelerator research fetch` (new `research`, `research-adapters`, `research-cli` crates)

- **Grammar**: `research fetch <openalex|arxiv> <search|lookup> …` returns
  normalised records as JSON, each carrying a tier derived from venue,
  version, retraction, and confirmed arXiv withdrawal, with a 600-character
  excerpt.
- **Degradation, not errors**: throttling, OpenAlex budget exhaustion
  (`Credits-Required`), or expiry of the 100 s deadline (measured from process
  entry) yields `status: "unavailable"`. Retries follow a 3/6/12 s schedule
  clamped to 30 s; `Retry-After` is honoured.
- **arXiv pacing**: a file-locked `FilePacingGate` spaces requests three
  seconds apart across every process in the project; withdrawals are confirmed
  through OAI-PMH `arXivRaw` and cached per version under the same lock.
- **Transport hardening**: at most three same-origin redirects, an 8 MiB body
  cap, no idle pooling, and URL-free errors so the OpenAlex key never leaks.
  XML is decoded namespace-aware with `roxmltree`.
- **Profiles**: `skills/research/profiles/openalex-profile` and `arxiv-profile`
  sit beside `web-profile` and provide the dispatch-coherence binding for the
  `research` token.

### Researcher confinement (`accelerator research guard`)

- **Registration**: `hooks/hooks.json` registers the guard under `PreToolUse`
  for `Bash` and for `Write|Edit|MultiEdit|NotebookEdit`, with
  `--fail-safe --non-blocking`.
- **Command rule**: a subagent of the configured researcher type may run only
  `accelerator research fetch …`, matched as Claude Code matches a
  `Bash(… *)` allow rule. The guard is stricter on two constructs: every
  unescaped `$` outside single quotes and comments, and every unquoted `<`,
  since together they let a researcher echo an environment secret back or
  send it out over `</dev/tcp/…`.
- **Write rule**: writes are limited to `<topics>/<set>/findings/<name>.md`,
  with a symlink walk below the canonical topics root. This closes the
  config-rewrite escalation.
- **Launcher `--non-blocking`**: a new `DispatchFailurePolicy` makes an
  integrity refusal report and exit non-blocking instead of exiting `2`, so an
  unresolvable `research` binary does not block every tool call. A guard panic
  on a crafted command still blocks.
- **Measured baseline**: `cli/research/tests/fixtures/claude-bash-baseline.tsv`
  records Claude Code's own verdicts per release;
  `tasks/probe/claude_permissions.py` re-measures them on a new release. A
  lexer differential runs the guard's verdicts against real `bash` and `zsh`
  (`test:integration:research`, fixed-seed sample; an exhaustive variant is
  opt-in).

### `research-topic` per-pair rounds

- **`brief`** offers `web`, `openalex`, and `arxiv`; **`outline`** assigns
  profiles per focus area with a `— profiles: web, openalex` suffix. An
  unsuffixed item is researched through `web` alone, so existing sets are
  unchanged.
- **`conduct`** asks `accelerator corpus topic-research outstanding` for the
  round's outstanding (focus area, profile) pairs and their paths, then spawns
  one researcher per pair writing `findings/<nn>-<slug>-<profile>.md`.
- **`agents/researcher.md`** gains `Bash`; `research-topic`'s `allowed-tools`
  grant the fetch, with a documented project allow rule as the fallback.
- **Templates** (`topic-research-brief`, `-outline`, `-finding`) and the
  finding outputter drop their hard-coded `web`.

### Credentials and config

- **`config::credentials`**: the token ladder moves out of `tracker-support`
  into `config` as a pure resolver behind four ports, with its adapters in
  `config-adapters`, so research need not depend on a tracker crate. `pup.ron`
  now denies `std::(fs|process|env)` in `config` and `std::process` in
  `tracker-support`.
- **OpenAlex key**: `openalex.api_key` / `openalex.api_key_cmd` (and
  `ACCELERATOR_OPENALEX_API_KEY[_CMD]`) are catalogued and resolved through the
  same ladder as the tracker tokens. The key is optional; OpenAlex works
  keyless within its free budget.
- **`config help`** lists every recognised key; **`config dump`** now redacts
  `api_key` and `api_key_cmd` leaves alongside `token` / `token_cmd`.
- **Fixes**: errors name `jira.token_cmd` once rather than
  `jira.token_cmd_cmd`, and a version-tracked `config.local.md` supplying a
  plain `token` or `api_key` is now refused with `E_TOKEN_FROM_TRACKED_FILE`
  whatever its mode, as a `token_cmd` already was.

### Docs and work items

- A new [Research CLI](https://atomicinnovation.github.io/accelerator/research/)
  docs page, README link, `configure` skill section, and CHANGELOG entries.
- 0280 amended to the contracts as built; 0121, 0282, 0283, and 0284 carry the
  matching sibling amendments.

## Context

- Work item: `meta/work/0280-academic-source-profiles.md`
- Research: `meta/research/codebase/2026-09-23-0280-academic-source-profiles.md`
- Plan: `meta/plans/2026-09-23-0280-academic-source-profiles.md`
- Validation: `meta/validations/2026-09-23-0280-academic-source-profiles-validation.md`
  (result `partial`: phases 1–9 implemented, the phase 10 quality gate not yet
  run)

## Testing

- [x] Bare `mise run` at validation: every leaf green except
      `test:e2e:visualiser`, which failed in global setup on a stale
      `.e2e-port` from an orphaned server in another workspace; re-run with
      `E2E_HEALTH_PORT=19187 mise run test:e2e`: 355 passed, 1 skipped
- [x] Live: `research fetch openalex search 'graph neural networks' --limit 2`
      returns tiered records keyless
- [x] Live: `research fetch arxiv lookup 2608.21129` returns `tier-3`,
      `withdrawn: true`, `retracted: false`
- [x] Guard: researcher `…search 'x'; ls` exits `2`
      (`E_RESEARCH_GUARD_SYNTAX`); researcher write to
      `.accelerator/config.md` exits `2` (`E_RESEARCH_GUARD_WRITE`);
      main-thread `ls` exits `0`
- [x] `accelerator config help` lists `openalex.api_key` and
      `openalex.api_key_cmd`
- [ ] `research fetch openalex search … --limit 3` with a real OpenAlex key
- [ ] Scratch Claude Code session: researcher `ls`, chained fetch, and config
      write blocked; main-thread `ls` runs
- [ ] Warm `PreToolUse` p95 latency ≤ 50 ms added on both matchers
- [ ] Phase 8 attended matrix (18 items), including the headless
      `--permission-prompts none`, interactive foreground, and model-invoked
      runs
- [ ] `brief` on an academic subject offers and records the academic profiles
- [ ] Phase 10 output-quality gate: release install, OpenAlex key,
      three-profile round, synthesis, four judgements, sign-off recorded in
      0280's Technical Notes

## Notes for Reviewers

- **Size**: ~28k lines across 193 files, most of it the three research crates
  and their tests. Suggested reading order: `cli/research/src` (tier,
  classify, schedule, fetch, confinement), then `research-adapters`
  (transport, pacing, confirmations), then `research-cli/src/guard.rs`, then
  the `research-topic` SKILL.md.
- **Confinement is not fail-closed**: if the `research` binary cannot be
  resolved or fails integrity, the guard reports on stderr and the call
  proceeds unconfined, with Claude Code's permission rules as the backstop.
  The researcher cannot induce that state because its writes are confined.
  Its unconfined `Read`, and overwriting a sibling finding, are accepted
  residuals. Point `agents.researcher` only at a dedicated agent.
- **Deviations from the plan**: `CredentialPorts::system` takes a
  `Box<dyn Provenance>` because `config-adapters` cannot depend on the VCS
  adapter; `Researchers::includes` is named `identify`; redirect same-origin
  checks compare against the original scheme rather than requiring `https`.
- **Open decisions from validation**:
  - `source_profiles: []` in a brief yields no pairs rather than defaulting to
    `["web"]` (`cli/corpus-adapters/src/topic_research.rs:78-81`).
  - A question repeated in a later round with an added profile only allocates
    at its first item, so the new pair stays incomplete
    (`cli/corpus/src/topic_research/round.rs:379,398`).
  - An OAI-PMH `<error>` (e.g. `idDoesNotExist` while OAI lags a new listing)
    fails the whole arXiv call with exit `1`.
- **CI**: the Linux lane now installs `zsh` for the lexer differential, via
  `apt-get install` without a preceding `apt-get update`.
- **Out of scope**: Crossref, Semantic Scholar, and the final citation pass;
  tier display in the visualiser (0284); recursion within a finding (0283);
  cross-repository arXiv rate coordination.
