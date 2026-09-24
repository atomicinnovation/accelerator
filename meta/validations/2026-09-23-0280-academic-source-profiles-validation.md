---
type: "plan-validation"
id: "2026-09-23-0280-academic-source-profiles-validation"
title: "Validation Report: Academic Source Profiles Implementation Plan"
date: "2026-09-24T22:34:47+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-23-0280-academic-source-profiles"
tags: ["research", "sources", "config", "cli", "hooks", "openalex", "arxiv"]
last_updated: "2026-09-24T22:34:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Academic Source Profiles Implementation Plan

Phases 1–9 are implemented as planned, with minor deviations only. The
result is `partial` because phase 10 (the human-judged output-quality gate)
has not run, and most of phase 8's attended manual matrix remains unticked.

### Implementation Status

✓ Phase 1: Amend 0280 — fully implemented (items 1–12 in 0280, item 13 in 0284)
✓ Phase 2: Credential resolution in `config` — fully implemented
✓ Phase 3: Config hygiene — fully implemented
✓ Phase 4: `research` domain crate — fully implemented
✓ Phase 5: `research fetch openalex` — fully implemented
✓ Phase 6: `research fetch arxiv` — fully implemented
✓ Phase 7: Researcher confinement — fully implemented
✓ Phase 8: One finding per (focus area, profile) — code complete; attended matrix pending
✓ Phase 9: Brief scoping and documentation — fully implemented
⚠️ Phase 10: Output-quality gate — not started (manual sign-off)

### Automated Verification Results

✓ Working copy clean before and after the run (no formatter drift)
✓ Bare `mise run` — every leaf exits `0` except `test:e2e:visualiser`, which
  failed in global setup on a stale `.e2e-port` left by an orphaned
  `start-server.mjs` from another workspace (environmental, not this change)
✓ `E2E_HEALTH_PORT=19187 mise run test:e2e` — 355 passed, 1 skipped
✓ `accelerator config help` lists a "Recognised keys:" block including
  `openalex.api_key` and `openalex.api_key_cmd` (dev launcher)
✓ Live: `research fetch openalex search 'graph neural networks' --limit 2`
  returns tiered records keyless
✓ Live: `research fetch arxiv lookup 2608.21129` returns `tier-3`,
  `withdrawn: true`, `retracted: false`
✓ Guard: researcher `…search 'x'; ls` → exit `2`,
  `E_RESEARCH_GUARD_SYNTAX`; researcher write to `.accelerator/config.md` →
  exit `2`, `E_RESEARCH_GUARD_WRITE`; main-thread `ls` → exit `0`

### Code Review Findings

#### Matches Plan:

- `config::credentials` holds the ladder behind four ports
  (`Environment`, `Provenance`, `FileFacts`, `TokenCommandRunner`); `pup.ron`
  denies `std::(fs|process|env)` in `config` and `std::process` in
  `tracker-support`, each with probe pairs.
- The four `_cmd` variants render the key verbatim; `TokenFromTrackedFile`
  refuses a tracked personal value and maps to Jira exit `24`.
- `config dump` hides `token`, `token_cmd`, `api_key`, `api_key_cmd` leaves;
  `config::agent_name` backs the launcher's `agents::resolve`.
- Domain: tier table, per-source classification (OpenAlex budget exhaustion
  via `Credits-Required`, arXiv `403`/`406`/`429` retried), 3/6/12 s schedule
  with 30 s clamp, pure `Deadline` built at process entry, 600-character
  excerpt.
- Adapters: ≤3 same-origin redirects, 8 MiB cap, `pool_max_idle_per_host(0)`,
  URL-free errors; namespace-aware `roxmltree` decoding; `FilePacingGate`
  spacing from finish with forward-only `not_before`; version-keyed
  confirmation cache written under the lock.
- Key-command timeout is the deadline's remaining budget
  (`research-cli/src/fetch_command.rs:172-219`).
- Guard: raw `tool_input` envelope, panic hook after identification,
  `STRICTER_THAN_MEASURED` for `$` and `<`, `SHELL_BLANKS` trim, symlink walk
  below the canonical topics root; both `PreToolUse` matchers registered;
  `DispatchFailurePolicy` covers every table cell.
- `topic-research outstanding` verb, round planning (26 cases), multi-profile
  fixture, `research-topic` prose, outputter, templates, docs page, configure
  skill, README and CHANGELOG entries.

#### Deviations from Plan:

- `CredentialPorts::system` takes a `Box<dyn Provenance>` rather than `root`,
  since `config-adapters` cannot depend on the VCS adapter
  (`cli/config-adapters/src/credentials.rs:39`); extra public
  `PERSONAL_CONFIG_RELATIVE` (`:24`).
- The `TokenFromTrackedFile` exit-code parity test is a unit test in
  `cli/jira-cli/src/exit_codes.rs:264-281`, not in `tests/exit_codes_parity.rs`.
- `Researchers::includes` is named `identify` (`cli/research/src/confinement.rs:74`).
- `dispatch_failure_policy.rs:14` redefines `DEAD_RELEASE_URL` instead of
  reusing `help.rs`'s.
- Redirect `same_origin` compares against the original scheme rather than
  requiring `https` (`cli/research-adapters/src/transport.rs:96-100`); the
  off-origin test switches http→https because the loopback server is plain
  http.
- The confirmation cache's "two concurrent writers" test interleaves writers
  sequentially (`cli/research-adapters/tests/confirmations.rs:63`).

#### Potential Issues:

- `source_profiles: []` in a brief yields an empty list, not `["web"]`
  (`cli/corpus-adapters/src/topic_research.rs:78-81`), so every pair is
  skipped as `NotInBrief`. The plan defaults only a missing value; decide
  whether empty should also default.
- A question repeated in a later round with an added profile is allocated
  only at its first item (`cli/corpus/src/topic_research/round.rs:379,398`),
  so the later item's new pair is never allocated and stays incomplete.
  Matches the plan's text; latent trap.
- `corpus-cli/src/main.rs:143-149` reports an ambiguous slug as
  `E_TOPIC_RESEARCH_UNRESOLVED`, dropping the candidates.
- The allocated `path` is canonical only if the set directory itself is not
  a symlink (`resolve.rs:73`).
- `.MD` findings are read as markdown, but `is_finding_path` accepts only
  lowercase `.md`, so a researcher could not write one; harmless today.
- An OAI-PMH `<error>` (for example `idDoesNotExist` while OAI lags a new
  listing) fails the whole arXiv call with exit `1` (`cli/research/src/fetch.rs:356-364`).
- `Retry-After` is parsed as integer seconds only; an HTTP-date falls back
  to backoff (`cli/research-adapters/src/transport.rs:131-133`).
- Guard residuals outside the plan's model: zsh global aliases from the
  user's shell snapshot expand after the lexer judges the line; `#` comment
  passing assumes non-interactive comment handling; globs and `~` expand to
  local file names that can reach a query (as measured and accepted).
- Guard fail-open before identification: a panic while resolving the
  configured researcher name exits `101` (non-blocking), and unreadable
  config confines only `accelerator:researcher`, not a custom name
  (`cli/research-cli/src/guard.rs:74-75,170-184`). Accepted by the plan.
- `BashTokenCommandRunner` kills only `bash` on timeout and then joins the
  reader, so a helper that backgrounds a child holding stdout can outlast the
  timeout; truncated over-cap output can be accepted as a token. Both
  predate this plan (moved unchanged from `tracker-support`).
- A symlinked or non-regular personal config is refused as
  `LocalPermsInsecure { mode: 0 }`, whose message suggests `chmod 600`,
  which does not fix a symlink (`cli/config/src/credentials.rs:451-456`).
- CI installs `zsh` with `apt-get install` without `apt-get update`
  (`.github/workflows/main.yml:95`).

### Manual Testing Required:

1. Phase 1:
  - [ ] Confirm 0280's items 1–12 read correctly and no superseded sentence
        remains (agent read found none).
2. Phase 3 / 5:
  - [ ] `accelerator config help` block reads well (listing verified present).
  - [ ] `research fetch openalex search … --limit 3` with a real key.
3. Phase 7:
  - [ ] Scratch session: researcher `ls`, chained fetch, and config write
        blocked; main-thread `ls` runs.
  - [ ] Warm `PreToolUse` p95 latency ≤ 50 ms added on both matchers.
4. Phase 8: the attended matrix (18 items), especially the headless
   `--permission-prompts none` run, the interactive foreground run, and the
   model-invoked run.
5. Phase 9:
  - [ ] `brief` on an academic subject offers and records the academic profiles.
6. Phase 10:
  - [ ] Release install, OpenAlex key, three-profile round, synthesis, four
        judgements, and sign-off in 0280's Technical Notes.

### Recommendations:

- Kill the orphaned `node e2e/start-server.mjs` and remove
  `cli/visualiser/frontend/.e2e-port` so the bare `mise run` is green without
  the port override.
- Decide whether `source_profiles: []` should default to `["web"]`, and
  whether a repeated question with new profiles should allocate its pairs.
- Consider killing the token helper's process group on timeout and
  rejecting over-cap output as a follow-up.
- Add `apt-get update` before the CI `zsh` install.
- Complete phase 8's attended matrix and the phase 10 gate before marking the
  plan `done`.
