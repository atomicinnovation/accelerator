---
type: "work-item"
id: "0280"
title: "Academic Source Profiles"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocked_by: ["work-item:0277", "work-item:0279"]
blocks: ["work-item:0283"]
relates_to: ["work-item:0278", "work-item:0281", "work-item:0282", "work-item:0284"]
tags: ["research", "skills", "sources", "config"]
last_updated: "2026-09-24T12:11:42+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-864"
---

# 0280: Academic Source Profiles

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a researcher using `/research-topic`, I want focus areas answered from
scholarly literature — OpenAlex and arXiv — as well as the open web, with every
source tiered by venue standing, so that a dossier on an academic or technical
subject rests on peer-reviewed and preprint evidence I can audit.

This slice adds two academic source profiles beside the web profile built in
0277 (the epic's Slice 1), reached through a new `accelerator research fetch`
CLI. It reuses 0277's reputation-tier mechanism and the generic `researcher`'s
profile injection — `conduct` passes a profile skill's path into each spawned
researcher — and carries the epic's (0121) sole output-quality gate.

It is not purely additive: it changes the unit of research. `outline` chooses
one or more profiles per focus area, a finding becomes one per (focus area,
profile) rather than one per focus area, and 0279's gap-fill works per pair.
Amending 0121's artifact contract, and the contracts of siblings 0281, 0283,
and 0284, to match is part of this story.

## Context

Reputation tagging already exists from 0277's web profile; this slice adds
academic source families and proves the `researcher` is specialised by injected
profile alone. The dedicated final citation pass stays deferred — a safe drop-in
later because the immutable tagged findings preserve attribution.

The epic assumed both APIs were keyless with a `mailto` polite pool. As of 2026
that no longer holds. OpenAlex introduced API keys on 13 Feb 2026: a free key
buys a $1/day usage-priced budget, keyless requests get a reduced ~$0.10/day
allowance, and `mailto` is ignored. arXiv remains keyless but caps clients at
one request per three seconds over a single connection, and has returned
capacity-driven `429`s even to compliant clients since late 2025.

The researcher's `WebFetch` cannot set headers, returns a small model's summary
rather than the raw body, and cannot pace requests across parallel researchers.
Academic access therefore goes through a deterministic CLI fetcher; `WebFetch`
remains the web profile's tool.

A focus area may need more than one source family to answer. `outline` — where
the model already applies judgement and the user can edit before conducting —
chooses each focus area's profiles; `conduct` executes that choice
deterministically, one researcher and one finding per (focus area, profile).

## Requirements

### Profile selection and findings

- The brief's `source_profiles` accepts `openalex` and `arxiv` beside `web`.
- `brief`'s scoping interview offers `web`, `openalex`, and `arxiv`, and writes
  the chosen subset to `source_profiles`, defaulting to `["web"]`.
- `outline` assigns each focus area one or more profiles, chosen by the model
  from the nature of the question and always a subset of the brief's
  `source_profiles`. Each outline item records them as a suffix:
  `- [ ] <question> — profiles: web, openalex`, which becomes
  `- [x] <question> — profiles: web, openalex` once complete. Items carry no
  finding links. The user may edit the profiles before conducting.
- `conduct` treats an outline item with no `— profiles:` suffix as
  `profiles: web`, so outlines written before this slice, and focus areas
  proposed by 0281's `ask`/`report`, need no change.
- `breadth` continues to cap focus areas per round, not researchers. Researchers
  per round are the sum of profiles across the round's focus areas, at most
  `breadth × |source_profiles|`.
- `conduct` spawns exactly one researcher per outstanding (focus area, profile)
  pair, injecting that one profile. Each researcher writes at most one finding
  to `findings/<nn>-<question-slug>-<profile>.md`, where `<question-slug>` is
  derived from the focus area's question (distinct from the set's subject
  slug). A focus area's first finding takes the next `<nn>` unused across the
  set, including quarantine markers; every later finding of that focus area
  reuses the `<nn>` of its retained findings.
- A (focus area, profile) pair is outstanding until a retained finding — one
  that validates and has not been quarantined as `.invalid` — exists with that
  `question` and `source_profile`. A focus area's checkbox flips only when every
  assigned profile has one; a gap-fill `conduct` spawns only the missing
  profiles.
- Each finding's `source_profile` records the injected profile, replacing the
  hardcoded `web` in `finding-outputter`, `conduct`, and the finding template.
- A researcher's outcome depends on whether its `research fetch` calls
  returned any record relevant to the question:
  - **Records** — at least one relevant record: it writes its finding from the
    relevant records, even if other calls were `unavailable`.
  - **Unavailable** — no relevant record and at least one call `unavailable`
    or exiting non-zero: it writes no file; `conduct` names the pair as
    outstanding in its summary, as it already does for a researcher that
    writes no file, and a later gap-fill retries it.
  - **None found** — every call returned `status: "ok"` and no record was
    relevant: it writes a finding stating that the family holds no relevant
    literature, with `None found.` under Sources. The pair is then complete
    and is not re-spawned.
- `conduct` never fails on a throttled or unavailable source.
- `synthesise` carries each finding's tiers forward unchanged.

### Academic profiles

- `openalex-profile` and `arxiv-profile` mirror the structure of 0277's
  `web-profile`. Each instructs the researcher to:
  - query its family only through `accelerator research fetch`, using `search`
    to discover works and `lookup` to resolve a known ID or DOI;
  - cite each record by its canonical `url` with the CLI-supplied `tier`, never
    re-judging it;
  - write a record with `retracted: true` as `tier-3 (retracted)` and one with
    `withdrawn: true` as `tier-3 (withdrawn)`;
  - handle its outcome as the Profile selection rules above prescribe.

### `research fetch` command

- `accelerator research fetch <openalex|arxiv> search <query> [--limit N]`
  returns up to `N` records, where `N` defaults to 10 and must be 1–25.
  `accelerator research fetch <openalex|arxiv> lookup <id>` returns one record;
  `openalex lookup` accepts an OpenAlex `W…` ID or a DOI, `arxiv lookup` an
  arXiv ID. `--help` documents the families, verbs, `--limit` range, and output
  shape.
- On success it exits `0` with `{"status": "ok", "records": [...]}` on stdout.
  Each record carries: `title`, `authors`, `url`, `venue`, `venue_signals`,
  `abstract` (excerpt), `tier`, `retracted`, and `withdrawn` (booleans;
  `withdrawn` is always `false` for OpenAlex and `retracted` always `false` for
  arXiv). The canonical `url` of an OpenAlex work is `doi.org/<doi>` when it has
  a DOI, else `openalex.org/W…`; of an arXiv entry, `arxiv.org/abs/<id>`.
- `venue_signals` carries the raw inputs to the tier: for OpenAlex, the primary
  location's `source.type`, `version`, `is_core`, and `listed_in`, and the
  work's `type` and `is_retracted`; for arXiv, `journal_ref` and `doi`.
- When a source is unavailable it exits `0` with
  `{"status": "unavailable", "source": "<openalex|arxiv>", "reason":
  "<rate_limited|budget_exhausted|upstream_error>"}` on stdout. A `--limit`
  outside 1–25, an unknown family or verb, a missing query or ID, and a
  credential refusal each exit non-zero before any upstream request.
- Other upstream outcomes: a `404` on OpenAlex `lookup` returns `status: "ok"`
  with no records. A `401` or `403` on a keyed OpenAlex request exits non-zero
  with an error naming the OpenAlex key as rejected, so a bad key never reads
  as "None found"; on a keyless OpenAlex request it exits non-zero saying
  OpenAlex refused an unauthenticated request. Any other OpenAlex `4xx` exits
  non-zero. A connection failure or timeout is retried like a `5xx`.
- An arXiv `lookup` whose `200` carries an empty feed, or an entry whose
  version-stripped ID differs from the requested one, returns `status: "ok"`
  with no records. A versioned ID is looked up without its version. A
  malformed arXiv ID exits non-zero before any request.
- arXiv `403`, `406`, and `429` are throttling: they are retried on the same
  schedule as an OpenAlex `429` and end `rate_limited`. An arXiv `503` is a
  `5xx`.
- On a `5xx`, or a `429` that is not budget exhaustion, the fetcher retries at
  most 3 times, waiting 3, 6, then 12 seconds, or the `Retry-After` value
  instead when present, clamped to 30 seconds, for both sources. A retry that
  succeeds returns its records as normal. When retries are exhausted, the final
  attempt decides the reason: `rate_limited` after a `429`, `upstream_error`
  after a `5xx`, connection failure, or timeout. On OpenAlex budget exhaustion
  — a `409`, or a `429` whose `X-RateLimit-Remaining` is below
  `X-RateLimit-Credits-Required` (or is `0` when that header is absent) — it
  returns `reason: "budget_exhausted"` without retrying.
- Every `research fetch` call completes within 100 seconds of process start,
  including credential resolution, lock waits, waits inside the lock, and
  withdrawal confirmations. When the next wait plus a 30-second request would
  overrun that deadline, the call returns `status: "unavailable"` with the last
  retryable attempt's reason, or `rate_limited` if none was made.
- Waits go through an injectable clock so tests assert the schedule without
  sleeping.
- OpenAlex `search` sends the query as
  `filter=title_and_abstract.search:<query>`, with `,`, `|`, `!`, and `:`
  replaced by spaces. arXiv `search` translates free text into `all:<term>`
  clauses joined by `AND`, dropping parentheses, quotes, and the operator words
  `AND`, `OR`, and `ANDNOT`. A query that is empty after normalisation is a
  usage error and exits non-zero before any request.
- OpenAlex follows at most 3 same-origin `https` redirects, since a merged work
  `301`s to its surviving ID; any other redirect exits non-zero without
  forwarding the key.

### Tier mapping

The CLI derives every record's tier, so the model never judges venue standing.
Rules apply in order and the first match wins. "Version" and "source" always
mean the OpenAlex work's primary location's.

1. `tier-3` — an OpenAlex work whose `is_retracted` is true (the record's
   `retracted` is then `true`), or a withdrawn arXiv entry (`withdrawn` is then
   `true`; see arXiv access for when an entry is withdrawn).
2. `tier-1` — an OpenAlex work whose type is not `preprint` and either whose
   source is a `journal` or `conference` with a `publishedVersion` or
   `acceptedVersion`, or whose source is `is_core` or has `medline` in
   `listed_in`, whatever its version.
3. `tier-2` — an OpenAlex work whose type is `preprint`, or whose source is a
   `repository`, or whose source is a `journal` or `conference` with a
   `submittedVersion`; and every arXiv entry. arXiv's `journal_ref` and `doi`
   are author-supplied and unvalidated, so they are reported in `venue_signals`
   but never raise the tier.
4. `tier-3` — anything else, such as a non-`preprint` work with no primary
   location source, or a versionless work in a source that is neither core nor
   `medline`-listed.

### OpenAlex credentials

- OpenAlex requests carry the API key in an `Authorization: Bearer` header,
  never the URL, and no output or error contains the key. The key is optional;
  when no rung yields one, requests send no `Authorization` header and run on
  the keyless allowance.
- The key resolves through the ladder `resolve_token` implements for
  `jira.token`, highest precedence first: `ACCELERATOR_OPENALEX_API_KEY`,
  `ACCELERATOR_OPENALEX_API_KEY_CMD`, then — only when the `config.local.md`
  file exists — its `openalex.api_key` and `openalex.api_key_cmd`, else —
  only when it does not exist — `config.md`'s `openalex.api_key`.
- The ladder reaches the shared rung only when neither environment variable
  (including the `_CMD` one) is set and `config.local.md` does not exist. There,
  `config.md`'s `openalex.api_key_cmd` is refused with a non-zero exit and
  `E_TOKEN_CMD_FROM_SHARED_CONFIG`, even when `config.md` also holds
  `openalex.api_key`. When the ladder stops earlier, `config.md` is never read.
  An insecurely permissioned `config.local.md` is refused as before.
- A VCS-tracked `config.local.md` that supplies `openalex.api_key` or
  `openalex.api_key_cmd` is refused with a non-zero exit, whatever its mode;
  one that supplies neither leaves the call keyless.

### arXiv access

- arXiv responses are parsed from Atom 1.0 into the record shape. Requests use
  `GET` only, send a descriptive `User-Agent` and
  `Accept: application/atom+xml`, and pass through a per-repository
  cross-process lock enforcing one connection and at least three seconds
  between requests.
- An arXiv entry is withdrawn only when both hold:
  - its `arxiv:comment` matches, case-insensitively, the anchored pattern
    `^\s*(this (paper|article|submission|manuscript) (has been|is) withdrawn|withdrawn)`;
  - an OAI-PMH `arXivRaw` `GetRecord` for it shows the latest `<version>` with
    `<size>0kb</size>` and `<source_type>I</source_type>`.

  The confirmation request is paced by the same lock. If it is unavailable,
  the whole call returns `status: "unavailable"` with that reason, so an
  unconfirmed withdrawal is never cited as `tier-2`.

### Researcher confinement

- The researcher gains `Bash`; its "run no CLI" rule narrows to "run only
  `accelerator research fetch`".
- A plugin `PreToolUse` hook confines the researcher. It applies to a call only
  when the hook input's `agent_id` is non-empty and its `agent_type` equals
  either the researcher agent name, resolved at call time through the same
  lookup as `accelerator config agent researcher`, or `accelerator:researcher`,
  which is always confined; every other call passes untouched. Once a call is
  identified as the researcher's, an unreadable command or path blocks.
- For an applicable `Bash` call it allows only a command that, after trimming
  surrounding whitespace, begins with `accelerator research fetch ` (including
  the trailing space) and whose remainder Claude Code's own `Bash(… *)` allow
  rule would accept, as measured on the latest Claude Code release, tightened
  for `$` and `<`. After the prefix, a command is blocked if, outside the
  quoting that neutralises it, it:
  - chains, with `&&`, `||`, `;`, `|`, `&`, a newline, or a backslash-newline;
  - redirects output other than to `/dev/null` or by descriptor duplication
    `[N]>&M`, so `>|`, `&>`, `>&word`, and `>&-` block;
  - redirects input at all: any unquoted `<` outside comments, including
    `<(…)`, `<>`, heredocs, and `</dev/tcp/…`;
  - expands a parameter at all: any unescaped `$` outside single quotes and
    comments, including `$VAR`, `$(…)`, `${…}`, `$[…]`, `$'…'`, and zsh
    subscripts on any parameter (`$HOME[…]`, `$0[…]`, `$=HOME[…]`);
  - substitutes with backticks or `=(…)`;
  - groups, with `{…}`, `(`, or `)`.

  Quoted text without `$`, a single-quoted `$`, globs, `~`, word-initial `#`
  comments, `[N]>` or `>>` to `/dev/null`, and `[N]>&M` pass. Blocking `$` and
  `<` is stricter than Claude Code, which accepts `$VAR` and `<`. Everything
  blocked exits `2`; a query that needs a blocked construct is rephrased by the
  researcher.
- A second `PreToolUse` matcher, for `Write|Edit|MultiEdit|NotebookEdit`,
  blocks an applicable call unless its path is absolute (or relative to the
  hook's `cwd`), has no `.` or `..` component, crosses no symlink, and names
  `<topics>/<set>/findings/<name>.md` with no leading dot in `<name>`, where
  `<topics>` is the configured research topics directory.
- `research-topic`'s `allowed-tools` grant `Bash(accelerator research fetch *)`
  so the researcher's fetch runs without a prompt; a documented project allow
  rule is the fallback.

### Documentation and registration

- The `openalex.*` keys are registered in the config catalogue and described in
  the configure help; `research fetch`, the academic profiles, and the
  `— profiles:` outline suffix are documented; the bare `mise run` exits `0`.

### Sibling contracts

- No statement in 0121 implies one finding or one researcher per focus area,
  that `conduct` chooses profiles, that outline items link their findings, or
  that the researcher guard keys on `agent_type` alone. This covers at least
  its Summary, goal 3, glossary Finding, artifact contract and outline format,
  Slice 1 description and its `conduct` and `breadth` criteria, Slice 3
  description and criteria, and its depth/breadth and Academic providers
  Technical Notes.
- 0283 records that its recursion carries a finding's `source_profile` into
  deeper levels, that its worst-case cost includes the per-profile multiplier,
  and that it unblocks on a passing gate sign-off rather than 0280's merge.
- 0284 records that its set-level page groups a focus area's findings by their
  `question` and `source_profile` frontmatter rather than by filename, accepts
  legacy `<nn>-<slug>.md` findings, reads the `— profiles:` suffix whether
  written with `—`, `–`, or `--`, and displays the `(retracted)` and
  `(withdrawn)` tier suffixes.
- 0281 records that `ask` and `report` read every finding of a focus area and
  accept suffixed tiers.

## Out of Scope

- The dedicated final citation pass.
- Crossref and Semantic Scholar profiles.
- Tier display in the visualiser (0284).
- Recursion within a finding (0283).
- Coordinating arXiv's rate limit across repositories on one machine.

## Acceptance Criteria

Criteria that stub `research fetch` use an `accelerator` shim earlier on `PATH`
that returns fixture JSON for `research fetch` and delegates everything else, so
the confinement hook still sees `accelerator research fetch …`. Criteria whose
outcome depends on model behaviour (`outline`'s choices, a researcher's
citations or outcome) pass only on 3 of 3 scripted runs; the rest are
deterministic and run once.

### Outline and conduct

- [ ] Given `brief`'s scoping interview, then it offers `web`, `openalex`, and
      `arxiv`, and writes the chosen subset to `source_profiles`; given no
      choice, then it writes `source_profiles: ["web"]`.
- [ ] Given a brief with `source_profiles: ["web", "openalex", "arxiv"]` and
      `breadth: 2`, when `outline` runs, then it writes at most 2 focus areas,
      each matching `- [ ] <question> — profiles: <p>(, <p>)*` with every `<p>`
      drawn from that list.
- [ ] Given a brief on an academic subject (e.g. "transformer attention
      mechanisms") with all three profiles, when `outline` runs, then at least
      one focus area is assigned `openalex` or `arxiv`.
- [ ] Given `breadth: 3` and an outline whose focus areas record `web`,
      `web, openalex`, and `arxiv`, when `conduct` runs, then it spawns exactly
      four researchers, each receiving one profile; each writes
      `findings/<nn>-<question-slug>-<profile>.md` whose `source_profile`
      names its profile; the `web` and `openalex` findings of the shared focus
      area carry the same `<nn>`; and each item becomes
      `- [x] <question> — profiles: …` only once all its profiles' findings
      validate.
- [ ] Given an outline item with no `— profiles:` suffix, when `conduct` runs,
      then it spawns one `web` researcher for it.
- [ ] Given a focus area `- [ ] <question> — profiles: web, openalex` with
      `findings/03-<question-slug>-web.md` retained and no `openalex` finding,
      when `conduct` runs again, then it spawns only the `openalex` researcher,
      which writes `findings/03-<question-slug>-openalex.md`, and the item
      becomes `- [x] <question> — profiles: web, openalex`.
- [ ] Given a focus area whose `openalex` finding exists only as a quarantined
      `.invalid` file, when `conduct` runs again, then it spawns the `openalex`
      researcher and the checkbox stays unticked until a valid finding is
      written.
- [ ] Given `research fetch openalex` stubbed to return
      `{"status": "unavailable", "source": "openalex",
      "reason": "budget_exhausted"}` for every call,
      when `conduct` runs, then no `*-openalex.md` finding exists, `conduct`'s
      summary names each (focus area, `openalex`) pair as outstanding, those
      checkboxes stay unticked, the round's other findings are written, and
      `conduct` exits successfully.
- [ ] Given `research fetch openalex` stubbed so some calls return `ok` with
      only irrelevant or no records and the rest `unavailable`, when `conduct`
      runs, then no
      `*-openalex.md` finding is written and the pair stays outstanding.
- [ ] Given `research fetch arxiv` stubbed to return `status: "ok"` with no
      records, when `conduct` runs, then the `arxiv` researcher writes a
      validated finding whose Sources reads `None found.`, and a further
      `conduct` does not re-spawn it.
- [ ] Given `research fetch openalex` stubbed so some calls return records and
      others `unavailable`, when `conduct` runs, then the `openalex` finding is
      written citing only the returned records.
- [ ] Given `research fetch openalex` stubbed to return records tiered
      `tier-1`, `tier-2`, and `tier-3`, one with `retracted: true`, and
      `research fetch arxiv` stubbed to return one record with
      `withdrawn: true`, when `conduct` runs, then each source in the findings'
      Sources carries exactly its stubbed record's `url` and `tier`, the
      retracted one reads `tier-3 (retracted)`, and the withdrawn one reads
      `tier-3 (withdrawn)`.
- [ ] Given findings carrying academic tiers, when `synthesise` runs, then each
      source cited in `synthesis.md` carries the same tier as in its finding.

### `research fetch`

- [ ] Given recorded OpenAlex and arXiv responses, when `research fetch`
      normalises them, then each record carries every listed field; an
      OpenAlex work with a DOI has `url` `https://doi.org/<doi>`, one without
      has `https://openalex.org/W…`, and an arXiv entry has
      `https://arxiv.org/abs/<id>`; an OpenAlex record's `venue_signals` holds
      `source.type`, `version`, `is_core`, `listed_in`, `type`, and
      `is_retracted`, and an arXiv record's holds `journal_ref` and `doi`; and
      each has this tier:
      - `article`, `journal` source, `publishedVersion` → `tier-1`
      - `article`, `conference` source, `acceptedVersion` → `tier-1`
      - `article`, `is_core` `journal` source, `submittedVersion` → `tier-1`
      - `article`, source `listed_in` `medline`, no version → `tier-1`
      - `preprint`, `journal` source, `publishedVersion` → `tier-2`
      - `preprint`, `is_core` `repository` source, `submittedVersion` →
        `tier-2`
      - `article`, `repository` source, not core or `medline` → `tier-2`
      - `article`, `journal` source, `submittedVersion`, not core or
        `medline` → `tier-2`
      - `article`, `conference` source, `submittedVersion` → `tier-2`
      - retracted `article`, `is_core` `journal` source, `publishedVersion` →
        `tier-3`, `retracted: true`
      - `article`, `journal` source, no version, not core or `medline` →
        `tier-3`
      - `article` with no primary location source → `tier-3`
      - `preprint` with no primary location source → `tier-2`
      - `book-chapter`, `book series` source, `publishedVersion`, not core or
        `medline` → `tier-3`
      - arXiv entry with `journal_ref` and `doi` → `tier-2`
      - arXiv entry with neither → `tier-2`
      - withdrawn arXiv entry — a withdrawal `arxiv:comment` confirmed by an
        `arXivRaw` latest version of `0kb` with source type `I` → `tier-3`,
        `withdrawn: true`, `retracted: false`
- [ ] Given `search` with no `--limit` against an upstream fixture holding 30
      results, then exactly 10 records return; given `--limit 0` or
      `--limit 26`, an unknown family, an unknown verb, a `search` with no
      query, or a `lookup` with no ID, then the command exits non-zero with no
      upstream request; given `openalex lookup` with a DOI and with a `W…` ID,
      and `arxiv lookup` with an arXiv ID, then each returns exactly one
      record; and `--help` lists both families, both verbs, and the 1–25
      `--limit` range.
- [ ] Given an injected clock, immediate upstream responses, and the 100-second
      deadline: given a `5xx` on every attempt, then `research fetch` makes
      exactly 4 attempts with waits of 3, 6, and 12 seconds and exits `0` with
      `reason: "upstream_error"`; given a `429` that is not budget exhaustion
      with `Retry-After: 120` on every attempt, then it makes 3 attempts with
      two 30-second waits and returns `reason: "rate_limited"`; given a `429`
      with `Retry-After: 5` then a `200`, then it waits 5 seconds and returns
      `status: "ok"` with the records; given a 30-second timeout on every
      attempt, then it makes 3 attempts and returns
      `reason: "upstream_error"`.
- [ ] Given a `409` from OpenAlex, a `429` with `X-RateLimit-Remaining: 0` and
      no `X-RateLimit-Credits-Required`, or a `429` with
      `X-RateLimit-Remaining: 5` and `X-RateLimit-Credits-Required: 10`, then
      it makes exactly one attempt and returns `reason: "budget_exhausted"`.
- [ ] Given arXiv answering `403`, `406`, or `429` on every attempt, then it
      retries on the schedule and returns `reason: "rate_limited"`; given
      arXiv answering `503` on every attempt, then it returns
      `reason: "upstream_error"`.
- [ ] Given an injected clock whose elapsed time since process start leaves
      less than the next wait plus 30 seconds before the 100-second deadline,
      then the call makes no further attempt and returns
      `status: "unavailable"` with the last retryable attempt's reason, or
      `rate_limited` if no attempt was made.
- [ ] Given an OpenAlex `lookup` answered `404`, then it exits `0` with
      `status: "ok"` and no records; given `401` or `403` on a keyed request,
      then it exits non-zero naming the rejected key and makes no retry; given
      `401` or `403` on a keyless request, then it exits non-zero saying
      OpenAlex refused an unauthenticated request; given `400`, then it exits
      non-zero.
- [ ] Given an OpenAlex `lookup` answered with a same-origin `https` `301` to
      the surviving work, then it follows it and returns that record; given a
      fourth redirect, a cross-origin redirect, or an `http` redirect, then it
      exits non-zero and the redirect target receives no `Authorization`
      header.
- [ ] Given an arXiv `lookup` whose `200` carries an empty feed, or an entry
      whose version-stripped ID differs from the requested one, then it exits
      `0` with `status: "ok"` and no records; given a versioned ID such as
      `2101.00001v2`, then the request asks for `2101.00001`; given a
      malformed arXiv ID, then it exits non-zero with no upstream request.
- [ ] Given an arXiv request, then it carries a descriptive `User-Agent` and
      `Accept: application/atom+xml`.
- [ ] Given an entry whose `arxiv:comment` reads "This paper has been
      withdrawn" and whose `arXivRaw` record's latest version has
      `<size>0kb</size>` and `<source_type>I</source_type>`, then it is
      `tier-3` with `withdrawn: true`; given an entry whose comment matches but
      whose latest version is non-empty, then it is `tier-2` with
      `withdrawn: false`; given the confirmation request unavailable, then the
      call returns `status: "unavailable"` with that reason.
- [ ] Given an OpenAlex `search` query containing a comma, a colon, or `|`,
      then the request's `title_and_abstract.search` filter carries spaces in
      their place; given an arXiv `search` query containing a parenthesis, a
      quote, or the word `OR`, then the request's `search_query` is `all:`
      clauses joined by `AND` with those dropped; given a query that is empty
      after normalisation, then it exits non-zero with no upstream request.
- [ ] Given two concurrent `research fetch arxiv` processes in the same
      repository, and separately two sequential calls issued within one
      second, then in each case their upstream `GET` requests are at least
      three seconds apart.

### Credentials

- [ ] Given distinct keys in the environment variable, the `_CMD` environment
      variable, and `config.local.md`'s `openalex.api_key` and
      `openalex.api_key_cmd`, when `research fetch openalex` runs, then it sends
      `Authorization: Bearer` with the environment variable's key; removing the
      highest remaining source each time yields the next source's key in turn.
- [ ] Given no environment key, no `config.local.md`, and `config.md` with
      `openalex.api_key`, then the request sends that key.
- [ ] Given `config.local.md` present without an `openalex` key and `config.md`
      with `openalex.api_key`, and no environment keys, then the request
      carries no `Authorization` header.
- [ ] Given no key anywhere, then the request carries no `Authorization`
      header; given no environment key, no `config.local.md`, and `config.md`
      with `openalex.api_key_cmd`, then the command exits non-zero with
      `E_TOKEN_CMD_FROM_SHARED_CONFIG` and makes no upstream request.
- [ ] Given `ACCELERATOR_OPENALEX_API_KEY` set and `config.md` holding
      `openalex.api_key_cmd`, then the request uses the environment key and
      exits `0`.
- [ ] Given `config.local.md` holding `openalex.api_key` or
      `openalex.api_key_cmd` that is VCS-tracked, even at mode `0600`, or
      holding `openalex.api_key` and readable by group or others, then
      `research fetch openalex` exits non-zero with no upstream request; given
      a VCS-tracked `config.local.md` holding neither key, then the request
      carries no `Authorization` header.
- [ ] Given any configured key, then the request URL does not contain it; and
      given a key with an upstream `5xx`, and a failing
      `openalex.api_key_cmd`, then neither stdout nor stderr contains any key.

### Researcher confinement

- [ ] Given the `PreToolUse` `Bash` hook and input with the researcher's
      `agent_type` and a non-empty `agent_id`, then it allows
      `accelerator research fetch arxiv search "graph neural networks"`,
      `accelerator research fetch openalex lookup W1858542512`, and
      `  accelerator research fetch arxiv search x  `; and blocks with exit `2`
      each of: `ls`, `accelerator research`, `accelerator research fetchx`,
      `echo accelerator research fetch arxiv search x`, and a `research fetch`
      call followed by `; ls`, `&& ls`, `|| ls`, `& ls`, `| cat`, `> out`,
      a newline and `ls`, `$(ls)`, `` `ls` ``, or `<(ls)`.
- [ ] Given a `research fetch` call under the same input, then the hook blocks
      with exit `2` one carrying each of: `;`, `$(ls)`, `=(ls)`, `(a)`,
      `${HOME}`, `$HOME`, `"$HOME"`, `$0[x]`, `$'\'' ; ls`, `x#;ls`,
      `</dev/tcp/example.org/80`, `< file`, and an unquoted URL containing
      `&`; and allows one carrying each of: `'a;b'`, `'$HOME'`, `\$HOME`, and
      a single-quoted URL containing `?`, `&`, and `#`.
- [ ] Given the `PreToolUse` `Write|Edit|MultiEdit|NotebookEdit` hook and input
      with the researcher's `agent_type` and a non-empty `agent_id`, then it
      blocks with exit `2` writes to `.accelerator/config.md`,
      `config.local.md`,
      `<topics>/s/findings/new/../../../../.accelerator/config.md`, a dangling
      symlink at the target, `<topics>/findings/x.md`, and
      `<topics>/s/findings/sub/x.md`; and allows a write to
      `<topics>/<set>/findings/01-x-web.md`.
- [ ] Given either hook, when the input has no `agent_id`, an empty `agent_id`
      with the researcher's `agent_type`, an empty `agent_type`, or another
      agent's `agent_type`, then every call passes; given the researcher's
      `agent_type` with an unreadable command or path, then it blocks.
- [ ] Given the researcher agent name overridden in config, then the hooks
      confine calls whose `agent_type` is the overridden name and calls whose
      `agent_type` is `accelerator:researcher` alike.
- [ ] Given `research-topic`'s `allowed-tools`, then they include
      `Bash(accelerator research fetch *)`.
- [ ] Given `agents/researcher.md`, then its body contains none of `openalex`,
      `arxiv`, or `web-profile`; and `openalex-profile` and `arxiv-profile`
      contain no instruction to use `WebFetch`.
- [ ] Given one focus area researched once with `web-profile` injected and once
      with `arxiv-profile` injected, then the session transcript shows the
      first researcher calling only `WebFetch`/`WebSearch` and the second
      calling only `Bash` with `accelerator research fetch arxiv …`.

### Documentation and sibling contracts

- [ ] Given `accelerator config help`, then it lists `openalex.api_key` and
      `openalex.api_key_cmd`; the `research-topic` documentation describes the
      academic profiles and the `— profiles:` suffix; and the bare `mise run`
      exits `0`.
- [ ] Given 0121, 0281, 0283, and 0284, then each carries the statements the
      Sibling contracts requirement prescribes.

### Output-quality gate

- [ ] Given a reference subject chosen by the reviewer and researched
      end-to-end with a brief listing all three profiles in a consuming repo,
      using the outline exactly as `outline` wrote it, with no hand edits to
      profiles, when a human reviews `synthesis.md` against the brief, then the
      reviewer
      records a pass or fail for each of four judgements:
      1. **Academic coverage** — it cites at least one `tier-1` OpenAlex source
         and at least one arXiv source.
      2. **Traceability** — every claim traces to a tiered source.
      3. **Tier defensibility** — of at least five cited sources spot-checked
         across tiers against their OpenAlex or arXiv records, the reviewer
         would move none to another tier.
      4. **Answers the brief** — the dossier answers the brief's questions.

      The reviewer records a dated sign-off line in this work item's Technical
      Notes naming the subject, the brief path, the four results, and each
      round's focus-area count, wall-clock time, and OpenAlex spend, taken as
      the difference in the key's reported usage before and after the
      round. The gate passes only when all four pass; a
      failed judgement keeps this story open and raises a follow-up for the
      cause. A round exceeding the Assumptions' bounds does not fail the gate
      but is recorded as invalidating that assumption, with a follow-up raised.
      This is the epic's only output-quality gate and is deliberately
      human-judged.

## Open Questions

None.

## Dependencies

- Blocked by: 0277 (the generic `researcher` and the web reputation-tier
  mechanism) and 0279, done (the gap-fill `conduct` and outline reconciliation
  this slice extends to (focus area, profile) pairs).
- Builds on: 0282, done — its tunable `breadth` bounds focus areas here. This
  slice lands after it and reconciles the config key-count test, `dump.golden`,
  and `public-api.txt` when registering `openalex.*`.
- Blocks: 0283 (the recursion engine lands only after this output-quality gate
  validates the premise). 0283 unblocks only on a sign-off where all four
  judgements pass, not on merge.
- Related: 0284 owns tier display in the visualiser and consumes the layout and
  suffixes introduced here (see Sibling contracts). It is refined only after
  this story's sibling amendments land; if it is built first, this story owns
  updating its implementation.
- Related: 0281's proposed focus areas need no change, since `conduct` defaults
  an item without profiles to `web`; its `ask`/`report` read the multi-finding
  layout (see Sibling contracts). The same ordering rule as 0284 applies.
- Related: 0278 co-lands with blocker 0277 and edits the same `research-topic`
  `outline`/`conduct` verbs this story changes.
- Parent: 0121's contract is amended as part of this story (see Sibling
  contracts).
- External: OpenAlex and arXiv availability, rate limits, and terms. An OpenAlex
  pricing or terms change invalidates the budget assumption below.
- Platform: Claude Code supplies `agent_id` and `agent_type` in `PreToolUse`
  input from v2.1.69, beneath the plugin's v2.1.144 floor; no floor change is
  needed.
- Quality gate: a plugin release carrying `research fetch` installed in the
  consuming repo, a nominated reviewer, and an OpenAlex key there.

## Assumptions

- With a free OpenAlex key, a round's OpenAlex spend stays under $1. Keyless use
  allows about 100 searches a day per client IP, shared with every other use
  from that IP, while lookups are free; a round can exhaust it and yield
  `budget_exhausted`, so configuring a key is recommended.
- Serialising arXiv requests at one per three seconds keeps a round's
  wall-clock time at or under 2 minutes × its focus-area count.

## Technical Notes

- "Reputation tier" is reputation-only — the standing of the publishing venue,
  not any assessment of a claim's correctness.
- `synthesis.md`'s and each report's Sources are derived from the findings'
  tiers, never authored independently.
- The tier text researchers write into a finding's Sources, including the
  `(retracted)` and `(withdrawn)` suffixes, belongs here; 0284 owns only how the
  visualiser displays tiers.
- `research fetch` is a new dispatched sub-binary in `cli/`; follow the
  thirteen-point registration checklist in `tasks/README.md`.
- Key resolution reuses the ladder `resolve_token` implements in
  `cli/tracker-support/src/credentials.rs`, relocated to a neutral
  `config::credentials` so no research crate depends on `tracker-support`,
  and treats its `NoToken` outcome as keyless rather than an error; register
  the `openalex.*` keys in `cli/config/src/catalogue.rs`.
- The researcher guard sits beside `vcs guard` under `PreToolUse` `Bash` in
  `hooks/hooks.json`, with a second registration for
  `Write|Edit|MultiEdit|NotebookEdit`. Plugin agents cannot carry their own
  `hooks`, and the agent `tools` field cannot scope `Bash` by command.
- The guard keys on `agent_id` as well as `agent_type` because `agent_type` is
  also present on main-thread calls in a session started with `--agent`. Plugin
  agents report their plugin-scoped name.
- The finding schema needs no change: `source_profile` stays a scalar.
- Quality gate sign-off: _pending_ — `<date> · <subject> · <brief path> ·
  coverage <pass|fail> · traceability <pass|fail> · tiers <pass|fail> ·
  brief <pass|fail> · per round: <focus areas>, <wall-clock>, <spend>`.

## Drafting Notes

- `openalex.*` is a top-level namespace, like `jira.*`, because it is an
  external-service credential rather than a research behaviour knob under
  `research.topic.*`.
- `research.contact_email` is dropped: OpenAlex ignores `mailto` and arXiv
  requires no identification.
- Tiers are derived in the CLI so the mapping is deterministic and testable.
- arXiv entries are always `tier-2` (2026-09-23): `journal_ref` routinely holds
  "submitted to", workshop, and thesis values, `doi` may be a non-publisher DOI,
  and OpenAlex's preprint-to-published merging mis-merges often enough that
  upgrading via it would overstate tiers. Understating a published paper is the
  safe direction; the OpenAlex profile finds the published version.
- A `preprint`-typed work is never `tier-1`, even in a journal location with a
  published version, for the same reason: OpenAlex's type is the stronger
  signal of what the record actually is.
- `medline` is the only `listed_in` value recognised for `tier-1` beyond
  `is_core` (itself `cwts-core`); national lists (`jufo-*`, `norway-*`) are
  excluded as regionally scoped.
- One finding per (focus area, profile) (2026-09-23) keeps every finding
  single-authored, single-profile, and independently degradable, at the cost of
  amending 0121's contract. The alternatives — one researcher carrying several
  profiles, or per-profile researchers merged into one finding — were rejected
  for weakening the single-profile injection proof and for a model-authored
  merge that risks tier attribution respectively.
- Outline items carry no finding links, matching what 0277 built; the epic's
  `→ findings/…` link format was never implemented and is dropped.
- A researcher writes "None found" only when every call succeeded, so a
  partially unreachable family is retried rather than closed with a false
  claim of no literature.
- Defaulting unannotated outline items to `web` keeps pre-existing outlines and
  0281's proposals valid without coordinating a change to 0281.
- arXiv keeps OpenAlex's 3/6/12-second schedule and 30-second `Retry-After`
  clamp (2026-09-23). Persistent arXiv capacity `429`s therefore surface as
  `unavailable` and are recovered by a later gap-fill `conduct`, rather than
  by a longer in-call backoff that would overrun the call deadline.
- Claude Code's own `Bash(… *)` matching is the confinement baseline
  (2026-09-24), replacing a forbidden-character rule that also blocked
  harmless quoted text. `$` and `<` are blocked beyond it because together
  they let a researcher echo an environment secret back through a usage error
  or send it to a remote host with `</dev/tcp/…`; the fetch needs neither.
- The arXiv lock is per repository. Concurrent research in separate repos on
  one machine can jointly exceed arXiv's limit, which its terms count across all
  machines under a client's control; this is an accepted risk.
- The quality gate runs in a consuming repo; the evidence is the recorded
  sign-off, not a committed set.
- Kept as one story despite its size, and with the gate inside it (2026-09-23
  review). A failed gate judgement keeps the story open until its follow-up
  resolves; its completion is therefore not bounded by merge, which is
  accepted. If delivery runs long, two seams split cleanly: the
  `research fetch` CLI together with the researcher confinement hook, and the
  per-(focus area, profile) redesign, which can land first on its own because
  unannotated items default to `web`.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 3)
- Parent epic: 0121
- Related: 0277, 0278, 0279, 0281, 0282, 0283, 0284
- Review: `meta/reviews/work/0280-academic-source-profiles-review-1.md`
- OpenAlex authentication and pricing:
  https://help.openalex.org/api-reference/authentication,
  https://help.openalex.org/access/pricing/
- OpenAlex `mailto` deprecation: https://help.openalex.org/api/deprecations/
- OpenAlex errors and rate-limit headers:
  https://help.openalex.org/api-reference/errors
- OpenAlex locations and versions: https://help.openalex.org/data/locations/
- arXiv API terms of use: https://info.arxiv.org/help/api/tou.html
- arXiv API user manual: https://info.arxiv.org/help/api/user-manual.html
- arXiv journal references and DOIs: https://info.arxiv.org/help/jref.html,
  https://info.arxiv.org/help/prep.html
- Claude Code hooks (subagent `agent_id`/`agent_type`, exit-2 blocking):
  https://code.claude.com/docs/en/hooks
- Claude Code v2.1.69 release notes (hook `agent_type`):
  https://github.com/anthropics/claude-code/releases/tag/v2.1.69
