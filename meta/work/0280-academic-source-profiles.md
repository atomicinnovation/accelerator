---
type: "work-item"
id: "0280"
title: "Academic Source Profiles"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0283"]
relates_to: ["work-item:0278", "work-item:0284"]
tags: ["research", "skills", "sources", "config"]
last_updated: "2026-09-22T20:10:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-864"
---

# 0280: Academic Source Profiles

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As a researcher using `/research-topic`, I want focus areas answered from
scholarly literature — OpenAlex and arXiv — as well as the open web, with every
source tiered by venue standing, so that a dossier on an academic or technical
subject rests on peer-reviewed and preprint evidence I can audit.

This slice adds two academic source profiles beside Slice 1's web profile,
reusing its reputation-tier mechanism and the generic `researcher`'s injection
seam, and carries the epic's sole output-quality gate.

## Context

Reputation tagging already exists from Slice 1's web profile; this slice adds
academic source families and proves the `researcher` is specialised by injected
profile alone. The dedicated final citation pass stays deferred — a safe drop-in
later because the immutable tagged findings preserve attribution.

The epic assumed both APIs were keyless with a `mailto` polite pool. As of 2026
that no longer holds: OpenAlex has required a free API key since 13 Feb 2026
(usage-priced against a $1/day free budget, ~$0.10/day keyless) and ignores
`mailto`; arXiv remains keyless but caps clients at one request per three
seconds over a single connection, and has returned capacity-driven `429`s even
to compliant clients since late 2025.

The researcher's `WebFetch` cannot set headers, returns a small model's summary
rather than the raw body, and cannot pace requests across parallel researchers.
Academic access therefore goes through a deterministic CLI fetcher; `WebFetch`
remains the web profile's tool.

## Requirements

- An `accelerator research fetch <openalex|arxiv> <search|lookup> …` command
  returning structured JSON records: title, authors, canonical URL
  (`openalex.org/W…`, `doi.org/…`, or `arxiv.org/abs/…`), venue name, venue
  signals, abstract excerpt, and derived tier.
- The academic tier mapping lives in the CLI, so the model never judges venue
  standing:
  - `tier-1` — a `journal` or `conference` source with a published or accepted
    version, or a source that is `is_core` / `listed_in` a recognised index.
  - `tier-2` — a `repository` source or `preprint` work type, including arXiv
    entries with no `journal_ref` or `doi`.
  - `tier-3` — an unidentifiable venue, or `is_retracted` work, with the
    retraction noted.
- OpenAlex requests carry the API key in an `Authorization: Bearer` header,
  never the URL. The key is optional; without it requests run on the keyless
  allowance.
- The key resolves through the same chain as `jira.token`:
  `ACCELERATOR_OPENALEX_API_KEY`, `ACCELERATOR_OPENALEX_API_KEY_CMD`,
  `config.local.md` `openalex.api_key`, then `config.local.md`
  `openalex.api_key_cmd`. `openalex.api_key_cmd` in the team-shared `config.md`
  is never honoured and emits `E_TOKEN_CMD_FROM_SHARED_CONFIG`.
- arXiv responses are parsed from Atom 1.0 into the same record shape. Requests
  use `GET` only and pass through a per-repository cross-process lock enforcing
  one connection and at least three seconds between requests.
- On `429`, `503`, or `5xx` the fetcher retries with bounded exponential
  backoff; on OpenAlex budget exhaustion (`409`, or `429` with
  `X-RateLimit-Remaining: 0`) it stops immediately. Either way it exits with a
  documented "source unavailable" outcome rather than an error.
- A researcher facing an unavailable source family still writes its finding from
  whatever it reached and names the unreachable family; `conduct` never fails on
  a throttled source.
- `openalex-profile` and `arxiv-profile` direct the researcher to
  `research fetch`, never `WebFetch`, for their APIs. The researcher gains
  `Bash`; its "run no CLI" rule narrows to "run only `accelerator research
  fetch`".
- A plugin `PreToolUse` `Bash` hook confines the researcher: for calls whose
  `agent_type` is the researcher, it blocks (exit `2`) any command that is not a
  bare `accelerator research fetch …` invocation, including any containing shell
  control operators, substitutions, redirections, or newlines. The guard fails
  closed for the researcher.
- The brief's `source_profiles` accepts `openalex` and `arxiv` beside `web`;
  `conduct` assigns one profile per spawned researcher; each finding's
  `source_profile` records the profile used, replacing the hardcoded `web` in
  `finding-outputter`, `conduct`, and the finding template.
- `synthesise` carries academic tiers forward unchanged.

## Acceptance Criteria

- [ ] Given a brief with `source_profiles: ["web", "openalex", "arxiv"]`, when
      `conduct` runs a round, then each researcher receives exactly one profile,
      the round mixes web and academic researchers, and each finding's
      `source_profile` names its profile.
- [ ] Given OpenAlex and arXiv responses covering a journal with a published
      version, an `is_core` source, a repository preprint, arXiv with and without
      `journal_ref`, an unidentifiable venue, and a retracted work, when
      `research fetch` normalises them, then each record carries the tier the
      mapping prescribes and a canonical URL.
- [ ] Given a key in `ACCELERATOR_OPENALEX_API_KEY`, its `_CMD` variant, or
      `config.local.md`, when `research fetch openalex` runs, then the request
      sends `Authorization: Bearer <key>` and no output contains the key; given
      `openalex.api_key_cmd` only in `config.md`, then it is ignored with an
      `E_TOKEN_CMD_FROM_SHARED_CONFIG` warning.
- [ ] Given two concurrent `research fetch arxiv` calls in the same repository,
      then their upstream requests are serialised at least three seconds apart.
- [ ] Given an upstream `429`, `503`, or `5xx`, when retries are exhausted, then
      `research fetch` exits with the documented "source unavailable" outcome;
      given OpenAlex budget exhaustion, then it returns that outcome without
      retrying.
- [ ] Given an unavailable source family, when the researcher completes, then a
      validated finding is still written from reached sources and names the
      unavailable family, and `conduct` does not fail.
- [ ] Given a `Bash` call from the researcher, when the command is anything other
      than a bare `accelerator research fetch …` — including one chaining
      `;`, `&&`, `|`, `$(…)`, backticks, or a redirection — then the
      `PreToolUse` hook blocks it; `Bash` calls from other agents and the main
      thread are unaffected.
- [ ] Given `agents/researcher.md`, then it contains no source-specific branches;
      web versus academic behaviour comes entirely from the injected profile.
- [ ] Given a reference subject researched end-to-end with a mixed-profile brief
      in a consuming repo, when a human reviews `synthesis.md` against the brief,
      then every claim traces to a tiered source, the tiers are defensible, and
      the dossier answers the brief's questions; the reviewer records a dated
      sign-off line in this work item's Technical Notes. This is the epic's only
      output-quality gate and is deliberately human-judged.

## Open Questions

- Which Claude Code version first supplies `agent_type` in hook input, and does
  it exceed the plugin's v2.1.144 floor? If so, the floor rises or the guard
  must fail closed when `agent_type` is absent in a researcher context.

## Dependencies

- Blocked by: 0277 (the generic `researcher` and web reputation-tier mechanism).
- Blocks: 0283 (the recursion engine lands only after this output-quality gate
  validates the premise).
- Related: 0284 owns all tier rendering; 0278's expectation that this slice
  renders tiers in `LibraryDocView` needs correcting.

## Assumptions

- A free OpenAlex key's $1/day budget covers a typical round (~1k searches).
- Serialising a round's arXiv requests at one per three seconds keeps round time
  acceptable.

## Technical Notes

- "Reputation tier" is reputation-only — the standing of the publishing venue,
  not any assessment of a claim's correctness.
- `synthesis.md`'s and each report's Sources are derived from the findings'
  tiers, never authored independently.
- `research fetch` is a new dispatched sub-binary in `cli/`; follow the
  thirteen-point registration checklist in `tasks/README.md`.
- Key resolution reuses `cli/tracker-support/src/credentials.rs`; register the
  `openalex.*` keys in `cli/config/src/catalogue.rs` and document them in the
  configure help.
- The researcher guard sits beside `vcs guard` under `PreToolUse` `Bash` in
  `hooks/hooks.json`. Plugin agents cannot carry their own `hooks`, and the
  agent `tools` field cannot scope `Bash` by command.

## Drafting Notes

- `openalex.*` is a top-level namespace, like `jira.*`, because it is an
  external-service credential rather than a research behaviour knob under
  `research.topic.*`.
- `research.contact_email` is dropped: OpenAlex ignores `mailto` and arXiv
  requires no identification.
- Tiers are derived in the CLI so the mapping is deterministic and testable.
- The arXiv lock is per repository. Concurrent research in separate repos on
  one machine can jointly exceed arXiv's limit, which its terms count across all
  machines under a client's control; this is an accepted risk.
- Tier rendering is left entirely to 0284.
- The quality gate runs in a consuming repo; the evidence is the recorded
  sign-off, not a committed set.
- 0121's Slice 3 still records the keyless and `mailto` premises and needs the
  same correction.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 3)
- Parent epic: 0121
- Related: 0277, 0278, 0284
- OpenAlex authentication and pricing:
  https://help.openalex.org/api-reference/authentication,
  https://help.openalex.org/access/pricing/
- OpenAlex `mailto` deprecation: https://help.openalex.org/api/deprecations/
- OpenAlex errors and rate-limit headers:
  https://help.openalex.org/api-reference/errors
- arXiv API terms of use: https://info.arxiv.org/help/api/tou.html
- arXiv API user manual: https://info.arxiv.org/help/api/user-manual.html
- Claude Code hooks (subagent `agent_type`, exit-2 blocking):
  https://code.claude.com/docs/en/hooks
