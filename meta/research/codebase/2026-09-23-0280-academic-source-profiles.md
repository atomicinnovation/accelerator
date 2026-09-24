---
type: "codebase-research"
id: "2026-09-23-0280-academic-source-profiles"
title: "Academic source profiles: codebase landing points and OpenAlex/arXiv API requirements"
date: "2026-09-23T16:45:45+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0280"
parent: "work-item:0280"
topic: "Academic source profiles: codebase landing points and OpenAlex/arXiv API requirements"
tags: ["research", "codebase", "research-topic", "openalex", "arxiv", "cli", "hooks", "credentials"]
revision: "aabd6f05e7785051c371168fa453a362633e7783"
repository: "accelerator"
last_updated: "2026-09-23T16:45:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Academic source profiles: codebase landing points and OpenAlex/arXiv API requirements

**Date**: 2026-09-23T16:45:45+00:00
**Author**: Toby Clemson
**Git Commit**: aabd6f05e7785051c371168fa453a362633e7783 (parent
6c3eec363d61, unpushed)
**Branch**: detached working copy above `main`
**Repository**: accelerator

## Research Question

For work item 0280 (`meta/work/0280-academic-source-profiles.md`): where does
the change land in the codebase, and — through web research and live probes —
what do the OpenAlex and arXiv APIs actually require of the
`accelerator research fetch` CLI?

## Summary

The codebase has every seam 0280 assumes except three: an XML parser, a pacing
lock primitive suited to arXiv, and a retry policy matching 0280's schedule.
`resolve_token` is already parameterised by key namespace, the `researcher` is
already profile-agnostic, and `conduct` hardcodes `web` in a small, enumerable
set of places.

The API research found that **several of 0280's upstream assumptions do not
match live behaviour on 2026-09-23**. Each is a requirement amendment, not an
implementation detail:

| # | 0280 says | Live / documented behaviour | Evidence |
|---|---|---|---|
| A1 | OpenAlex budget exhaustion is `409`, or `429` with `X-RateLimit-Remaining: 0` | `409` appears nowhere in OpenAlex docs; exhaustion is a `429`. `X-RateLimit-Remaining` is a credit count and a search costs 10 credits, so a search can be refused with 1–9 remaining. `X-RateLimit-Credits-Required` is exposed for comparison | Live headers; help.openalex.org/api/errors |
| A2 | arXiv `lookup` miss is a `404` → `ok`, no records | Unknown, malformed, and non-existent-version IDs all return `200` with an empty feed (`totalResults` 0) | Live probe |
| A3 | arXiv entries carry a detectable `withdrawn` flag | The Atom API has no withdrawal field. The only signal is `arxiv:comment` text (heuristic, false positives exist); OAI-PMH `arXivRaw` exposes `source_type` `I` / `size` `0kb` on the withdrawn version | Live probe of 2608.21129 |
| A4 | Non-429 `4xx` exits non-zero; `401`/`403` means the OpenAlex key was rejected | arXiv throttles with `406`, `403`, and `503` (empty bodies) as well as `429`. The `401`/`403` rule must be scoped to OpenAlex, and arXiv `403`/`406` are throttling | arXiv API group threads, community issues |
| A5 | Retry at 3 s, 6 s, 12 s | arXiv capacity `429`s persist for minutes; community clients back off ≥ 60 s. `Retry-After` on arXiv is unconfirmed | arXiv API group threads |
| A6 | Keyless OpenAlex "exhausts its allowance within a single round" | Keyless = $0.10/day = 1,000 credits = 100 searches; singleton lookups are free. A round of 8 focus areas at ~5 searches each costs ~$0.04. The keyless budget is shared per client IP across all use that day | Live headers; help.openalex.org/access/example-costs |

Confirmed as written: Bearer-header auth, `mailto` retirement, `401` on a bad
key, `404` on an OpenAlex lookup miss, the `listed_in`/`is_core`/`version`
fields and their location, arXiv's 3-second single-connection terms, and the
author-supplied status of `journal_ref`/`doi`.

## Detailed Findings

### OpenAlex API

Documentation moved to **help.openalex.org** (old `docs.`/`developers.` hosts
301 there). The backend was rewritten as "Walden" (default from 3 Nov 2025)
and moved to USD usage pricing in Feb 2026. Older GitHub docs and the Jan 2026
announcement quote superseded credit numbers.

#### Authentication

- ✅ `Authorization: Bearer <key>` is honoured: an invalid Bearer key yields
  `401` where the same request keyless yields `200` (live probe). The docs say
  header and `?api_key=` "work identically"; the OpenAPI spec lists only the
  query parameter, so the docs and live behaviour are the stronger evidence.
- `401` body (live): `{"error":"Invalid or missing API key","message":"API key
  not found"}`. `401` is undocumented; the documented error shape is
  `{error, message}`.
- `mailto` is retired and not treated as identification
  (help.openalex.org/api/deprecations).
- The blog calls keys "mandatory"; the docs and live behaviour say keyless
  requests work on a $0.10/day allowance. Treat keyless as supported but
  fragile.

#### Pricing and budget

| Call | Cost | Credits | Relevance to `research fetch` |
|---|---|---|---|
| Singleton `/works/{id}` | $0 | 0 | `lookup` is free |
| List with `filter` only | $0.0001 | 1 | — |
| `search=` / `search.exact` / `search.semantic` | $0.001 | 10 | every `search` |
| Content (PDF/TEI) | $0.01 | 100 | not used |

- Allowance: keyless $0.10/day (1,000 credits), free key $1/day (10,000
  credits); resets at midnight UTC. Errors cost nothing.
- `select=` and `per_page` do not change the cost of a call (empirical).
- Per-second limit: 100 req/s per key; `search.semantic` 1 req/s.

#### Rate-limit headers (live, 2026-09-23)

Every response carries these, and `Access-Control-Expose-Headers` names the
full set, including two not seen on a success:

```text
x-ratelimit-cost-usd: 0.001            # this call
x-ratelimit-credits-used: 10           # this call
x-ratelimit-limit: 1000                # daily credits
x-ratelimit-limit-usd: 0.1
x-ratelimit-remaining: 909             # credits left today
x-ratelimit-remaining-usd: 0.0909
x-ratelimit-onetime-remaining: 0
x-ratelimit-prepaid-remaining-usd: 0
x-ratelimit-reset: 26063               # seconds to midnight UTC
# exposed, absent on success:
X-RateLimit-Credits-Required, X-RateLimit-Cost-Required-USD, Retry-After
```

- Budget exhaustion is best detected as a `429` whose
  `X-RateLimit-Remaining` is below `X-RateLimit-Credits-Required` (the
  official CLI's rule, `ourresearch/openalex-official` `api_client.py`), or
  whose `Retry-After` runs to the UTC reset. The `Remaining: 0` rule in 0280
  misses the 1–9-credit case.
- The keyless probe showed 909 remaining after one 10-credit search, i.e.
  other keyless traffic from the same IP that day. The keyless allowance is
  per client, not per repository or process.
- Per-round spend (the quality gate's metric) can be measured two ways:
  `GET /rate-limit` before and after (keyed only — keyless returns `401`;
  fields `daily_used_usd`, `credits_used` etc., unverified against a keyed
  call), or by summing `x-ratelimit-cost-usd` per call.

#### Search

- `GET /works?search=<q>` now searches title, abstract, **and full text**
  (`meta.x_query` maps it to `fulltext.search`). A live comparison gave
  191,142 hits against 31,061 for `filter=title_and_abstract.search:<q>`, at
  identical cost. Relevance-sorted either way.
- Paging: `per_page` 1–100 (default 25), so 0280's `--limit` 1–25 maps
  directly to `per_page`.
- `select=` takes top-level fields only. A sufficient projection:
  `id,doi,display_name,type,authorships,primary_location,is_retracted,abstract_inverted_index,publication_year`.
- `corpus=core|expansion|all` (Aug 2026, default `core`) replaced
  `include_xpac`.
- Query length: ~4 KB URL limit (docs) vs 8 kB (blog).

#### Lookup

- Accepted forms: `W2741809807`, `https://openalex.org/W…`,
  `doi:10.7717/peerj.4375`, `https://doi.org/10.7717/peerj.4375`. Both DOI
  forms and a `W…` ID verified live.
- ✅ Unknown ID → `404` (live), matching 0280's "`404` on lookup → `ok`, no
  records".
- Merged works return `301` to the surviving ID. The existing Jira transport
  disables redirects (`cli/jira-client/src/transport.rs:80-88`); the OpenAlex
  transport must follow same-host redirects (reqwest keeps `Authorization` on
  same-host redirects and strips it cross-host).

#### Work schema

- `id` and `doi` are already URL forms (`https://openalex.org/W…`,
  `https://doi.org/…`), so 0280's canonical `url` is `doi` when non-null,
  else `id`.
- `type` has 25 documented values (`article`, `preprint`, `review`,
  `book-chapter`, `conference-paper`, `dataset`, `retraction`, …) and
  should be treated as an open set.
- `is_retracted` comes from Retraction Watch.
- `abstract_inverted_index` is `{word: [positions]}` or `null`; rebuild by
  sorting (position, word) pairs and joining with spaces. No plaintext
  abstract is provided.
- `authorships[].author.display_name`, capped at 100 authors.
- `primary_location` (may be `null`) has `version`
  (`publishedVersion` | `acceptedVersion` | `submittedVersion` | `null`) and
  `source` (may be `null`) with `type` (`journal`, `repository`,
  `conference`, `ebook platform`, `book series`, `other`, and an
  unregistered `metadata`), `is_core`, and `listed_in`.
- ✅ `listed_in` lives on the location's `source`, and its 22 values include
  `cwts-core`, `medline`, `doaj`, `jufo-*`, `norway-*`, `abdc-*`. Live
  example: PeerJ lists `["cwts-core","doaj","jufo-1","ki-jl-1","medline","norway-1"]`.
  `is_core` is kept for backward compatibility; `listed_in` is its general
  replacement.
- Text fields should be treated as untrusted when displayed
  (help.openalex.org/api).

### arXiv API

The legacy `export.arxiv.org/api/query` API remains the only search API. Its
implementation was replaced on 11 Nov 2025 with the same URL, parameters, and
Atom format, but stricter query parsing and different error behaviour from the
manual.

#### Requests

- Use `https://export.arxiv.org/api/query` with `GET` (POST bypasses the
  Fastly cache and is throttled sooner). Live responses show a Varnish cache
  layer (`x-cache: HIT`, `age: 219`).
- Parameters: `search_query`, `id_list` (comma-separated), `start`,
  `max_results` (default 10), `sortBy` (`relevance` | `lastUpdatedDate` |
  `submittedDate`), `sortOrder`.
- Query syntax: field prefixes `ti`, `au`, `abs`, `co`, `jr`, `cat`, `rn`,
  `all`; `AND`, `OR`, `ANDNOT`; space → `+`, quotes `%22`, parentheses
  `%28`/`%29`. Since the migration, loose queries are auto-corrected or
  rejected (`raw=1` disables auto-correction), and unbalanced parentheses
  error. The fetcher must translate the researcher's free text into a valid
  `search_query` (e.g. `all:` terms joined with `AND`).
- Send a descriptive `User-Agent` and `Accept: application/atom+xml`;
  community reports tie missing headers to `406`s.

#### Errors and throttling

| Situation | Response (live unless noted) |
|---|---|
| Unknown / malformed ID, non-existent version | `200`, empty feed, `totalResults` 0 |
| Invalid parameter (`sortBy=bogus`) | `400`, Atom feed with one `<entry>` titled `Error`, message in `<summary>` |
| Mixed valid + invalid `id_list` | `500` |
| Very large `max_results` | `500` (manual says `400`) |
| Capacity throttling | `429` "Rate exceeded.", `503`; also `406`/`403` with empty bodies (reported) |

- Terms of use: at most one request every three seconds, one connection at a
  time, counted across all machines under the client's control. No
  `User-Agent` requirement.
- Since Nov 2025 staff describe `429` as a global capacity signal, not a
  per-client breach; compliant clients still receive it. Threads through
  Sept 2026 report `406`/`429` on the first request after an idle period.
- No source confirms `Retry-After` on arXiv `429`/`503`. Community fixes back
  off ≥ 60 s, capped ~600 s; one project added a 30-minute cooldown after
  `406`s.

#### Atom format

- Namespaces: Atom `http://www.w3.org/2005/Atom`, OpenSearch
  `http://a9.com/-/spec/opensearch/1.1/`, arXiv `http://arxiv.org/schemas/atom`.
  Parse by namespace URI; prefix and declaration placement changed in the
  migration.
- Entry fields: `id` (`http://arxiv.org/abs/<id>v<N>`, still `http`),
  `title`, `summary` (leading spaces and embedded newlines — collapse
  whitespace), `author/name`, `published` (v1 date), `updated`, `link`
  (`alternate`, `pdf`, `doi`), `arxiv:primary_category`, `arxiv:comment`,
  `arxiv:journal_ref`, `arxiv:doi`. Element order varies between entries.
- 0280's canonical `https://arxiv.org/abs/<id>` requires stripping the
  `v<N>` suffix and upgrading the scheme.
- A lookup should check that the returned ID (version-stripped) equals the
  requested one, and validate ID format client-side (new-style `YYMM.NNNNN`,
  old-style `archive/NNNNNNN`, optional `vN`).

#### Withdrawals

- A withdrawal creates a new version with no PDF or source; the reason goes
  in Comments; earlier versions stay public (info.arxiv.org/help/withdraw).
- Live check of 2608.21129 (withdrawn as v2): the API entry keeps the full
  abstract and still has a `pdf` link. The only Atom signal is
  `arxiv:comment` "This paper has been withdrawn by the authors…".
- `co:"has been withdrawn"` returns 5,025 entries, including false positives
  such as "supersedes arXiv:2510.26642, which has been withdrawn". An
  anchored pattern (`^\s*(This (paper|article|submission|manuscript) (has
  been|is) withdrawn|Withdrawn)`) narrows but does not eliminate error.
- OAI-PMH `arXivRaw` (`https://oaipmh.arxiv.org/oai?verb=GetRecord&identifier=oai:arXiv.org:<id>&metadataPrefix=arXivRaw`)
  shows the withdrawn version with `<size>0kb</size><source_type>I</source_type>`.
  The meaning of `I` is inferred, not documented. It costs a second
  rate-limited request per record.

#### Metadata provenance

- `journal_ref` and `doi` are entered by authors through the journal-reference
  facility with no described validation (info.arxiv.org/help/jref) —
  consistent with 0280's decision never to raise an arXiv tier on them.

### `research-topic` skill: where 0280 lands

- **`conduct` hardcodes the profile.** It injects
  `skills/research/profiles/web-profile/SKILL.md` unconditionally
  (`skills/research/research-topic/SKILL.md:212`), and the brief's
  `source_profiles` is read nowhere downstream.
- **Hardcoded `web`, all six sites:** `SKILL.md:146` (brief writes
  `["web"]`), `:212` (profile path), `:246` (`source_profile: web`),
  `skills/research/outputters/finding-outputter/SKILL.md:43`,
  `templates/topic-research-finding.md:12`,
  `templates/topic-research-brief.md:10`. Fixtures under
  `cli/corpus-cli/tests/fixtures/topic-research-*/` also carry `web`.
- **Finding naming.** `conduct` allocates `findings/<nn>-<slug>.md` scanning
  both `<nn>-*.md` and quarantine markers "so an index is never reused"
  (`SKILL.md:197-200`), and quarantines as `.<nn>-<slug>.md.invalid`
  (`:226-235`). 0280's per-focus-area `<nn>` reuse and per-profile marker
  names both require rewording this rule.
- **Outstanding detection.** Reconcile ticks a checkbox when "its finding"
  validates (`SKILL.md:191-195`); outstanding means an unticked checkbox.
  0280 changes this to per-(`question`, `source_profile`) matching, which
  requires stripping the `— profiles:` suffix before comparing to a finding's
  `question`.
- **Schema.** `source_profile` and `source_profiles` are required extras
  checked for presence only (`cli/corpus/src/frontmatter_validation/mod.rs:371-386`,
  `schema.rs:222-230`, `templates-schema.tsv`). No value check exists, so
  `openalex`/`arxiv` validate with no schema change — and so does a typo.
- **Researcher.** `agents/researcher.md:7` grants `WebSearch, WebFetch,
  Write, Read`; step 3 (`:26-28`) forbids any CLI, repeated at
  `SKILL.md:221`. The skill's `allowed-tools` (`SKILL.md:12-16`) cover only
  `accelerator config` and `corpus` commands.
- **`web-profile` shape** (`skills/research/profiles/web-profile/SKILL.md`):
  frontmatter `user-invocable: false`, `disable-model-invocation: true`;
  sections Source Family, Untrusted-Content Contract, Reputation Tiers; no
  scripts. The academic profiles mirror this.
- **`synthesise`** passes tier text through verbatim (`SKILL.md:250-259`); no
  change is needed for suffixed tiers.

### CLI workspace

#### Credentials

- `resolve_token(&CredentialContext, &TokenKeys)`
  (`cli/tracker-support/src/credentials.rs:243-309`) takes `TokenKeys { env,
  env_command, value, command }` (`:88-94`), so `openalex.api_key` plugs in
  like Jira (`cli/jira-client/src/auth.rs:49-56`) and Linear
  (`cli/linear-client/src/auth.rs:45-51`).
- The ladder matches 0280 exactly: env → `_CMD` env → (`config.local.md`
  exists) insecure-perms refusal, personal value, personal command with
  VCS-tracked refusal → (absent) shared `_cmd` refused with
  `E_TOKEN_CMD_FROM_SHARED_CONFIG`, else shared value → `NoToken`
  (`:247-308`).
- ⚠️ Latent bug: `TokenCmdFailed`/`TokenCmdTimedOut`/`TokenCmdFromSharedConfig`
  are built with the `_cmd` key name and `Display` appends `_cmd` again
  (`:172-184`, `:256`, `:285`, `:296`), rendering `jira.token_cmd_cmd`. 0280's
  `E_TOKEN_CMD_FROM_SHARED_CONFIG` criterion would render
  `openalex.api_key_cmd_cmd`.

#### Config catalogue

- Credential keys live in `EXTRA_KEYS` (`cli/config/src/catalogue.rs:131-148`),
  beside `jira.token(_cmd)`.
- The count test `the_catalogue_holds_sixty_five_keys_across_seven_groups`
  (`catalogue.rs:282-292`) excludes `EXTRA_KEYS`, so adding `openalex.*`
  there leaves 65 unchanged. `cli/config/tests/extra_keys_mirror.rs` and
  `cli/launcher/tests/fixtures/dump/dump.golden` do change;
  `cli/config/tests/fixtures/public-api.txt` does not unless a new const is
  added.
- 🔒 `config dump` hides only leaves named `token` or `token_cmd`
  (`cli/launcher/src/config_command/core/dump.rs:327-332`). `openalex.api_key`
  would print in plain text, violating 0280's "no output contains the key"
  unless the redaction widens.
- `accelerator config help` is clap-derived from `ConfigAction` doc comments
  (`cli/launcher/src/launch/inbound/cli.rs:79-260`) and lists no keys; the
  per-key documentation lives in `skills/config/configure/SKILL.md`. 0280's
  "`accelerator config help` lists `openalex.api_key`" needs either a help
  change or retargeting to the configure skill.

#### HTTP, retry, and clock

- `reqwest =0.12.28` blocking with rustls/ring (`cli/Cargo.toml:69-74`); each
  transport installs the ring provider.
- Shared retry pieces (`cli/tracker-support/src/retry.rs`): `Sleeper` +
  `SystemSleeper` (`:48-59`), `Jitter` (`:21-44`), `RetryPolicy { max_attempts:
  4 }` (`:61-71`). `delay_for` (`:80-104`) computes `2^(n-1)` s ±30% jitter,
  `Retry-After` clamped to 1–60 s. 0280's 3/6/12 s with a 30 s clamp needs its
  own delay policy; `Sleeper` is the injectable clock seam 0280 requires.
- The Jira transport retries `429 | 5xx` but not connection errors or
  timeouts (`cli/jira-client/src/transport.rs:193-196`, `:292-294`), and
  parses `Retry-After` as integer seconds only (`:296-306`). 0280 retries
  connection failures.
- 🔒 `connect_detail` uses `error.to_string()` (`transport.rs:345`), which
  includes the URL. Bearer auth keeps the key out of the URL; the Linear
  `redact()` helper (`cli/linear-client/src/upload.rs:233`) is the precedent
  if a query parameter is ever used.
- Tests use the in-house `cli/http-test-support` `MockServer` with `Status`,
  `Headers`, `Sequence`, `FlakyThenOk`, and `Stall` routes — enough for every
  retry criterion. Base URLs are injected through an env seam admitted only
  under a `test-loopback` feature (`cli/jira-cli/src/context.rs:56-84`,
  `cli/jira-cli/src/main.rs:44-48`).

#### Locking and XML

- No `fs2`/`fd-lock`/`fs4`. Available: the workspace mkdir-lock
  (`cli/corpus-adapters/src/lock.rs:30-68`, stale-PID reclaim, 300 s ceiling)
  and `flock` via `rustix::fs::flock`
  (`cli/launcher/src/launch/outbound/resolve/tree/lease.rs`) or libc
  (`cli/design-adapters/src/lock.rs:22-78`). The arXiv pacing gate needs a
  held lock plus a persisted last-request timestamp; `.accelerator/state/`
  (`paths.integrations`) hosts similar state. Whole-file writes must go
  through `store::atomic_write` (enforced by `tasks/lint/store_duplication.py`).
- No XML crate is in `cli/Cargo.lock`. Adding `quick-xml` or `roxmltree`
  (both MIT/Apache-2.0) passes the `cli/deny.toml` licence allowlist
  (`:52-64`) but is subject to `multiple-versions = "deny"`.

#### Sub-binary registration

- No `research` token or `cli/research*` crate exists. Follow the
  thirteen-point checklist at `tasks/README.md:444-611` (points at 456, 463,
  470, 486, 491, 497, 505, 534, 547, 557, 562, 572, 581); the library-crate
  checklist at `:613-671` applies to a `research` domain crate.
- Registration lists: `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py:29-39`),
  `_SUBBINARY_MANIFESTS` (`tasks/manifest.py:87`), `_CLI_RELEASE_BINARIES`
  (`tasks/build.py:38-49`), dev `--bin` list (`tasks/build.py:~313-323`),
  `.gitignore:48-56`, `cli/Cargo.toml` members (`:4-43`), `cli/pup.ron`,
  `.github/workflows/main.yml` upload and attest blocks.
- Crate pattern: `cli/<domain>/`, `cli/<domain>-adapters/`, `cli/<domain>-cli/`
  (e.g. `cli/design*`); `cli/jira-cli/Cargo.toml` is the template for a
  networked CLI with `test-loopback`.

#### Hook

- `vcs guard` is registered at `hooks/hooks.json:41-51` and reads
  `.tool_input.command` from stdin JSON (`cli/vcs-cli/src/main.rs:61-67`).
- ⚠️ It blocks by emitting a `permissionDecision: "deny"` JSON envelope and
  exiting `0` (`cli/kernel/src/hooks.rs:27-34`), not by exiting `2`. 0280
  specifies exit `2`; both are valid Claude Code blocking mechanisms, so the
  new guard diverges from house style unless 0280 changes.
- Agent-name lookup: `agents_view::resolve`
  (`cli/launcher/src/config_command/core/agents.rs:58-71`) falls back to
  `accelerator:<name>`; the guard can call it in-process.

## Code References

- `skills/research/research-topic/SKILL.md:146,197-200,212,221,226-235,246` — hardcoded `web`, index allocation, quarantine, no-CLI rule
- `skills/research/outputters/finding-outputter/SKILL.md:43` — `source_profile` always `web`
- `skills/research/profiles/web-profile/SKILL.md` — profile shape to mirror
- `agents/researcher.md:7,26-28` — tools list, no-CLI rule
- `templates/topic-research-finding.md:12`, `templates/topic-research-brief.md:10` — hardcoded `web`
- `cli/corpus/src/frontmatter_validation/mod.rs:371-386` — presence-only extras check
- `cli/tracker-support/src/credentials.rs:88-94,172-184,243-309` — `TokenKeys`, `_cmd_cmd` rendering, ladder
- `cli/tracker-support/src/retry.rs:48-104` — `Sleeper`, `RetryPolicy`, `delay_for`
- `cli/jira-client/src/transport.rs:80-88,193-196,292-306,345` — redirects off, no connect retry, `Retry-After`, URL in errors
- `cli/config/src/catalogue.rs:131-148,282-292` — `EXTRA_KEYS`, count test
- `cli/launcher/src/config_command/core/dump.rs:327-332` — redaction by leaf name
- `cli/corpus-adapters/src/lock.rs:30-68` — mkdir-lock
- `cli/kernel/src/hooks.rs:27-34`, `cli/vcs-cli/src/main.rs:61-67` — hook deny envelope, stdin parsing
- `tasks/README.md:444-671` — sub-binary and library-crate checklists
- `tasks/shared/paths.py:29-39` — `DISPATCHED_SUBBINARIES`

## Architecture Insights

- Deterministic work belongs in the CLI (ADR-0045/0053/0054), which is why
  tiers are derived there. The API findings reinforce this: withdrawal
  detection, empty-feed-as-miss, and throttle classification are all
  source-specific rules better tested once in Rust than described to a model.
- The retry, clock, and lock logic fit a hexagonal core: a pure
  `classify(response) → {Records, Retry(delay), Unavailable(reason), Fail}`
  per source, with the transport, `Sleeper`, and pacing lock as ports. OpenAlex
  and arXiv diverge enough (A1, A2, A4, A5) that classification should be
  per-source rather than one shared status table.
- The researcher's genericity survives: every source-specific rule lives in a
  profile or in the CLI, and `agents/researcher.md` only gains `Bash`.

## Historical Context

- `meta/plans/2026-09-09-0277-single-round-web-research-engine.md` — chose a
  `Bash`-less researcher because agent `tools:` cannot scope `Bash`; a test
  pins that tool list. 0280's `PreToolUse` guard is the answer, and the pin
  must be rewritten deliberately. Web-profile researchers also gain `Bash`.
- The `conduct` write-scope assertion remains deferred (0277, 0279). 0280 adds
  an outbound channel (queries to OpenAlex/arXiv) comparable to `WebFetch`;
  the accepted risk is unchanged but should be restated.
- `meta/plans/2026-09-19-0279-iterative-accretion-and-finalise.md` — counts
  derived from disk; model-behaviour criteria verified by attended manual runs
  because no `research-topic` eval harness exists. 0280's "3 of 3 scripted
  runs" and `PATH` shim assume a runner that does not exist.
- `meta/plans/2026-09-20-0282-tunable-depth-and-breadth.md` — `breadth` is
  enforced only by `outline`; `conduct` honours a hand-edited outline.
- `meta/reviews/work/0280-academic-source-profiles-review-1.md` — approved
  after four passes; sibling amendments to 0121/0281/0283/0284 are present.
- No earlier research covered either API; this document is the first.

## Related Research

- `meta/research/codebase/2026-09-19-0279-iterative-accretion-and-finalise.md`

## Open Questions

1. **Withdrawal detection (A3).** Choose between an anchored `arxiv:comment`
   heuristic documented as best-effort, or an OAI-PMH `arXivRaw` confirmation
   per candidate (a second paced request). The criterion "withdrawn arXiv
   entry → `tier-3`, `withdrawn: true`" needs a fixture definition of
   "withdrawn" either way.
2. **Budget-exhaustion rule (A1).** Replace "`409`, or `429` with
   `X-RateLimit-Remaining: 0`" with "`429` with `X-RateLimit-Remaining` below
   `X-RateLimit-Credits-Required`" (or absent credits header plus a
   `Retry-After` beyond the clamp). Drop `409` or keep it as defensive.
3. **arXiv lookup miss (A2).** Replace "`404` on `lookup`" for arXiv with
   "empty feed, or a returned ID that does not match".
4. **arXiv throttle statuses and schedule (A4, A5).** Classify arXiv
   `403`/`406`/`503` as throttling, scope the key-rejected error to OpenAlex,
   and decide whether 3/6/12 s is long enough given the 2-minute-per-focus-area
   wall-clock assumption, or whether arXiv should fail fast to `rate_limited`
   and leave recovery to gap-fill.
5. **Keyless assumption (A6).** Restate the Assumptions: keyless allows ~100
   searches/day per client IP, shared with any other use.
6. **OpenAlex `search=` scope.** Default `search=` now matches full text; the
   narrower `title_and_abstract.search` costs the same and may give more
   relevant top-10s. Worth deciding before the quality gate.
7. **`brief` population of `source_profiles`.** Nothing lets a user reach the
   academic profiles without editing `brief.md` by hand.
8. **Hook exit convention.** Exit `2` (0280) vs the JSON deny envelope used by
   `vcs guard`.
9. **Key hygiene.** Widen `config dump` redaction to `api_key`/`api_key_cmd`,
   fix the `_cmd_cmd` rendering, and retarget or implement the `config help`
   key-listing criterion.
10. **Model-behaviour harness.** Build a minimal scripted-run mechanism, or
    accept attended manual runs as 0279 did.

## External Sources

OpenAlex:
- https://help.openalex.org/api/authentication/
- https://help.openalex.org/api-reference/authentication
- https://help.openalex.org/openapi.json
- https://help.openalex.org/api/errors/
- https://help.openalex.org/api/deprecations/
- https://help.openalex.org/access/pricing/
- https://help.openalex.org/access/example-costs/
- https://help.openalex.org/access/agents/
- https://help.openalex.org/api-reference/rate-limits/check-rate-limit-status
- https://help.openalex.org/api/searching/
- https://help.openalex.org/api/semantic-search/
- https://help.openalex.org/api/paging/
- https://help.openalex.org/api/selecting-fields/
- https://help.openalex.org/api/sorting/
- https://help.openalex.org/api/get-single-entities/
- https://help.openalex.org/data/works/attributes/
- https://help.openalex.org/data/work-types/
- https://help.openalex.org/data/locations/
- https://help.openalex.org/data/source-types/
- https://help.openalex.org/data/source-lists/
- https://help.openalex.org/data/works/corpus/
- https://blog.openalex.org/openalex-api-new-features-and-usage-based-pricing/
- https://blog.openalex.org/openalex-rewrite-walden-launch/
- https://groups.google.com/g/openalex-users/c/rI1GIAySpVQ
- https://raw.githubusercontent.com/ourresearch/openalex-official/main/src/openalex_cli/api_client.py
- https://github.com/J535D165/pyalex/issues/100
- https://github.com/hunter-heidenreich/academic-tools-mcp/pull/144
- https://github.com/uzak0209/AI-Research/issues/51
- https://github.com/Burton-David/oalex/pull/1

arXiv:
- https://info.arxiv.org/help/api/user-manual.html
- https://info.arxiv.org/help/api/tou.html
- https://info.arxiv.org/help/api/index.html
- https://info.arxiv.org/help/withdraw.html
- https://info.arxiv.org/help/jref.html
- https://info.arxiv.org/help/oa/index.html
- https://groups.google.com/a/arxiv.org/g/api/c/-WpHNxbaxU0 (Nov 2025 migration)
- https://groups.google.com/a/arxiv.org/g/api/c/pNB3lnxf4mQ (429 semantics)
- https://groups.google.com/a/arxiv.org/g/api/c/ycq8giRdZsQ (rate-limit guidance, GET vs POST)
- https://groups.google.com/a/arxiv.org/g/api
- https://github.com/sdewell/code-quorum/issues/2
- https://github.com/kasahart/audio-ai-weekly/issues/43
- https://github.com/PeterGracar/mathpr-digest/pull/21

Live probes (2026-09-23): `api.openalex.org/works?search=…`,
`/works/W2741809807` keyless and with an invalid Bearer key;
`export.arxiv.org/api/query?sortBy=bogus`, `?id_list=2101.99999`,
`?id_list=hep-th/9901001`, `?id_list=2608.21129v1`; `arxiv.org/abs/2608.21129v2`;
`oaipmh.arxiv.org/oai?verb=GetRecord&identifier=oai:arXiv.org:2608.21129&metadataPrefix=arXivRaw`.
