---
type: "codebase-research"
id: "2026-11-20-PP-891-tier-doi-links-by-venue-not-resolver"
title: "Tier DOI Links by Venue, Not by the doi.org Resolver"
date: "2026-11-20T15:12:37+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-891"
parent: "work-item:PP-891"
relates_to: ["codebase-research:2026-10-06-PP-882-fair-arxiv-fetch-queue", "codebase-research:2026-09-23-PP-864-academic-source-profiles", "codebase-research:2026-10-24-PP-936-structured-source-metadata", "codebase-research:2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once", "codebase-research:2026-09-24-PP-756-unify-the-trust-barrier-for-consent-config-keys"]
topic: "Tier DOI Links by Venue, Not by the doi.org Resolver"
tags: ["research", "codebase", "research-cli", "queue", "arxiv", "crossref", "openalex", "web-profile", "config", "metadata"]
revision: "8d6f8a8a284b4fe633ddf6357d4753dae99f0668"
repository: "accelerator"
last_updated: "2026-11-20T15:12:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tier DOI Links by Venue, Not by the doi.org Resolver

**Date**: 2026-11-20T15:12:37+00:00
**Author**: Toby Clemson
**Git Commit**: 8d6f8a8a284b4fe633ddf6357d4753dae99f0668 (working copy on
205d3d118a)
**Branch**: detached (jj working copy)
**Repository**: accelerator

## Research Question

What does the codebase look like today for every part of PP-891? That means:
the project-wide arXiv queue to be generalised, a new Crossref-backed
`fetch doi metadata` command, a `crossref.mailto` setting, the web profile and
outputter text, and an OpenAlex search/lookup parity test. Where does the
current code fit the item's requirements, and where does it conflict with
them?

## Summary

Most of what PP-891 needs already exists as seams, but several requirements
conflict with how the code works today. Each needs a deliberate design choice
in the plan.

1. **The queue is mostly service-agnostic in policy but arXiv-shaped in
   mechanics.**
   - The ticket rules, the 100 s bound, the 900 s cap, the front rule, the
     serving window and the carried last-retryable reason are all generic.
   - The binding (`Binding::Search{ArxivQuery,Limit}|Lookup(ArxivId)`), the
     port names, every file name and the help text are arXiv-specific.
   - **Concurrency is hard-wired to 1**, through one exclusive `flock` on
     `arxiv.lock`.
   - **Pacing spaces each request from the previous one's *finish***, not from
     its start. Crossref's limits are requests-per-second and concurrency, so
     the engine needs N serving slots and a start-to-start interval mode.
2. **Tickets carry no family, and an unknown ticket silently re-joins.** As
   designed, a ticket nonce from a wiped queue makes the ticket unknown, so it
   joins afresh. PP-891 instead wants two cases to exit 2 with a mismatch, not
   to be re-queued:
   - a Crossref ticket shown to arXiv (and the reverse);
   - a pre-upgrade ticket.

   That needs a family-tagged ticket format, and a rule that maps the old
   `n-hex` form to `E_ARXIV_TICKET_MISMATCH` rather than `…_MALFORMED`.
3. **Response classification is per service.** arXiv treats 404 and other
   non-throttling 4xx as a client failure (exit 1), and treats 403 and 406 as
   throttling. Crossref instead needs:
   - 404 → `ok` with empty metadata;
   - any other 4xx except 429 → `unavailable`/`upstream_error`, exit 0;
   - 429 → throttling.
4. **There is no project-scoped test clock shared across processes.** The
   loopback `LoggingClock` is a per-process virtual offset over a fixed epoch.
   Multi-process tests run in real time. Several PP-891 acceptance criteria
   (rate windows, concurrency, cross-process Retry-After, mixed pools) assume
   one shared virtual clock, so a new test seam is needed.
5. **Config.** No setting is resolved the way PP-891 asks (env →
   `config.local.md` → `config.md`, first non-empty wins):
   - the credential ladder reads `config.md` only when `config.local.md` is
     absent;
   - `ConfigAccess` reads no environment variables.

   A new `EXTRA_KEYS` entry ripples into the pinned catalogue tests, the
   `config help` test and the `config dump` golden.
6. **The web profile is simpler than PP-891 assumes.** It has no "Never
   re-judge a tier" rule, no metadata template and no worked examples. The
   structure test also requires **every** fenced block in it to be a
   `web metadata` call, so new `doi metadata` and `--ticket` examples need that
   test changed.
7. **The OpenAlex parity test is straightforward**, because search and lookup
   share `OPENALEX_SELECT`, the `WorkJson` decoder and `openalex::normalise`.
   But `search` is a `title_and_abstract.search` text query, not `filter=doi:`
   as the acceptance criterion says.

## Detailed Findings

### 1. The project-wide queue today

#### Crate split
These follow ADR-0069 and ADR-0053.

- **`research` (pure domain; no serde or I/O, enforced by cargo-pup).**
  - `sources/queue.rs` holds the ticket rules:
    - constants `EXPIRY = 300s`, `CAP = 900s`, `LONGEST_INVOCATION = 100s` and
      `ABANDONED_AFTER = CAP + LONGEST_INVOCATION` (lines 17-24);
    - the `Nonce` and `Ticket` types (`<u64>-<6 hex>`, lines 28-106);
    - `Binding` (110-114, arXiv-typed);
    - `Presence`, `Standing`, `Position` and `Contender`;
    - `Queue::present` (258-294) and `is_front_given` (345-358);
    - the ports `ArxivQueue` (412-422), `Joining` (424-439) and `Place`
      (442-456).
  - `sources/fetch.rs` holds:
    - the ports `Transport`, `Clock`, `PacingGate`/`ServingTurn` and
      `ContentionLog`;
    - the generic `admitted` (370-389, 100 ms poll) and
      `Attempts::until_settled` (579-634);
    - the arXiv-specific `fetch_arxiv` (299-348) and
      `upstream_reason_or_contention` (352-364).
  - `sources/schedule.rs` holds the retry schedule and `Deadline`;
    `sources/classify.rs` holds response classification per service.
- **`research-adapters`.**
  - `queue.rs`: `FileArxivQueue` and `FilePlace`, with JSON records and
    `flock`s.
  - `pacing.rs`: `FilePacingGate`, `ArxivTurn`, `PacingState`, and `NoPacing`
    for OpenAlex.
  - `contention.rs`: `FileContentionLog`.
  - `scratch.rs`: `ScratchDir`, which confines writes to
    `<root>/<paths.tmp>/research`.
- **`research-cli`.**
  - `main.rs` defines `CALL_BUDGET = 100s` (90) and asserts at compile time
    that `CALL_BUDGET <= LONGEST_INVOCATION` (91-92). It also defines
    `REQUEST_BUDGET = 30s` (93) and builds the adapters in `source_call`
    (273-348).
  - `fetch_command.rs` defines `ArxivCall` and `ArxivAdapters`.
  - `render.rs` builds the documents and the `E_ARXIV_TICKET_*` lines.

#### On-disk state
Everything lives under `.accelerator/tmp/research/`:

```
arxiv.lock              serving flock (one request at a time, project-wide)
arxiv-pacing            {"last_sent_ms","last_finish_ms","not_before_ms"}  (no schema version)
arxiv-requests.log      one ms timestamp per send
arxiv-contention.log    "<ms> ticket_past_cap <ticket>" | "<ms> queue_unusable"
arxiv-withdrawals.json  confirmation cache
arxiv-queue/
  queue.lock            bookkeeping flock
  <n>-<nonce>.json      {"schema_version":1,"issued_ms","presented_ms","ended_ms","last_retryable","verb",...binding}
  <n>-<nonce>.lock      liveness flock held by the owning call
```

Record format: `research-adapters/src/queue.rs:640-730`. Housekeeping:
`queue.rs:260-313`.

#### Pacing
`research-adapters/src/pacing.rs`:
- `SPACING = 3s` (57), `SEND_MARGIN = 100ms` (60), `DEFERRAL_CEILING = 30s`
  (62).
- Concurrency 1 comes from a non-blocking exclusive `flock` on `arxiv.lock` in
  `try_serve` (140-156). The turn is held for the whole call: every attempt,
  backoff and withdrawal confirmation (`fetch.rs:403-419`).
- **The wait is measured from `last_finish + spacing`** (`wait_at`, 209-224).
  When the last send has no recorded finish, it is `last_sent + 3.1s`.

#### Retry-After across processes
- Only a throttling verdict sets the shared deferral. For arXiv that is 403,
  406 or 429, via `Attempts::deferral` (`fetch.rs:636-648`).
- `ArxivTurn::paced` stores `not_before` (`pacing.rs:165-187`, capped at
  now + 30 s).
- 5xx Retry-After is honoured within the process but not persisted.
- Only delta-seconds are parsed (`research-adapters/src/transport.rs:141-153`).

#### Waiting vs unavailable
In `fetch_arxiv`:
- A queued call that runs out of budget **always** prints `waiting`, even
  mid-retry. `step_aside` stores `last_retryable` (`queue.rs:563-573`).
- An over-cap re-presentation reports the stored reason, or
  `rate_limited` + `cause: lock_contention` when none is stored (`fetch.rs:352-364`).
- An *unqueued* call reports `lock_contention`, never `waiting`. An unqueued
  call is one whose queue cannot be locked, listed or written.

#### Ticket presentation order
`Queue::present`, `queue.rs:258-294`:
1. unknown or expired → Join (fresh number);
2. differing arguments → `Mismatch`;
3. live → `AlreadyLive`;
4. more than 900 s since issue → `OverCap`;
5. otherwise Resume.

Expiry (300 s after the last end) is checked **before** the cap, so a ticket
left absent for more than 300 s re-joins rather than reaching `unavailable`.

#### Retry and classification
- Retries: `BACKOFF = [3s, 6s, 12s]`, so at most four attempts. Retry-After
  replaces the backoff, clamped to 30 s (`schedule.rs:28-59`).
- `classify::arxiv` (`classify.rs:190-205`):

  | Answer | Verdict |
  |---|---|
  | Connection failed or timed out | `UpstreamError` retry |
  | 403, 406, 429 | `RateLimited` retry (throttling) |
  | 5xx | `UpstreamError` retry |
  | 3xx | Fail |
  | Any other 4xx, including 404 | `Fail(ClientError::Status)`, i.e. `E_RESEARCH_CLIENT_ERROR`, exit 1 |

#### Tickets and families
- `parse_ticket` refuses any family other than `Family::Arxiv` with
  `E_RESEARCH_USAGE: --ticket applies only to arxiv fetches`
  (`request.rs:89-91, 123-134`).
- The ticket text carries no family.
- `E_ARXIV_TICKET_MALFORMED` is a `RequestError` Display (`request.rs:84-88`).
  `E_ARXIV_TICKET_MISMATCH` and `…_LIVE` are formatted in the CLI
  (`render.rs:57-86`), through the arXiv-specific `call_as_typed` (88-95).

#### Fit against PP-891's queue requirements

| Requirement | Today | Gap |
|---|---|---|
| Per-service pacing | Spacing read only via `gate.spacing()`; serving window derived from it (PP-882 plan) | Gate is arXiv-only; spacing is finish-based |
| Crossref public 5/s, 1 concurrent | — | Finish-based spacing of 200 ms with 50 ms responses gives one start every 250 ms, so the last of 10 calls starts at 2.25 s and misses the AC's "all 10 within [0, 2 s)". A start-to-start interval is needed |
| Crossref polite 10/s, 3 concurrent | — | Needs N serving slots (e.g. N lock files) instead of one `flock` |
| Pools paced independently | — | Separate gate state per pool (e.g. `crossref-public`, `crossref-polite`) |
| Separate ticket families | No family in ticket or record | Tag the family in the ticket text or the record, and look up across families to report a mismatch instead of a Join |
| Old tickets → `E_ARXIV_TICKET_MISMATCH` | Unknown → Join; a wrong shape → MALFORMED | Explicit legacy-format rule |
| New state kept apart from the old release | `arxiv-queue/`, `SCHEMA_VERSION = 1` | Older binaries **delete** records with an unknown schema version that probe free (`queue.rs:280-297, 344`), so a new directory is safer than a schema bump |
| Crossref 404 → ok-empty; other 4xx → `unavailable` | arXiv fails these with exit 1 | New `classify::crossref` |
| 100 s / 900 s / waiting carrying the reason | Implemented generically | Reuse; constants stay module-level |

Two compatibility notes from the PP-882 review:
- **Mixed versions and `arxiv.lock`.** arXiv's one-connection guarantee holds
  across plugin versions only because every version takes `arxiv.lock`.
  PP-891 accepts that pacing is not shared during the upgrade overlap. If the
  new engine keeps `arxiv.lock` and the `arxiv-pacing` format while moving
  only the queue directory, that risk disappears for arXiv.
- **Outcome modelling.** `FetchOutcome::Waiting`/`Rejected` sit on the shared
  outcome type. PP-888 item 4 asks whether to split them; adding Crossref makes
  that question live.

### 2. Existing arXiv queue test coverage

This is the PP-891 prerequisite mapping. `R` = `cli/research/tests/`, `A` =
`cli/research-adapters/tests/`, `C` = `cli/research-cli/tests/` (binary tests,
which need `--features test-loopback`).

| Behaviour | Covering tests | Gaps |
|---|---|---|
| Pacing spacing / concurrency across processes | `C/arxiv_pacing.rs:62` `a_second_process_waits_for_the_first_processs_response_to_finish`, `:86` `sequential_calls_are_spaced_three_seconds_apart`, `:118`, `:150`; `C/fetch_arxiv.rs:361`; `A/pacing.rs:124-154, 291-342, 356-411`; stress `C/arxiv_queue_stress.rs:67` (`#[ignore]`) | — |
| Retry-After shared across processes | `C/fetch_arxiv.rs:378` `a_throttled_call_defers_the_next_calls_first_request`; `A/pacing.rs:180, 190, 211, 221`; `R/arxiv_fetch.rs:455` | Two processes run **one after the other**, never concurrently |
| `waiting` and `--ticket`, position | `C/fetch_arxiv.rs:404, 450, 472, 485, 504, 768`; `C/arxiv_queue.rs:113, 133, 157`; `R/queue.rs:278-655` | — |
| 100 s bound | `R/arxiv_fetch.rs:550-628` (serving-window edges on a hand-built 100 s `Deadline`); compile-time assert | The production 100 s is not pinned by any binary test |
| 900 s cap → `unavailable` | `R/queue.rs:380, 390, 418`; `R/arxiv_fetch.rs:809, 864`; `A/queue.rs:430, 1041`; `C/fetch_arxiv.rs:706, 722` | — |
| `upstream_error` | `C/fetch_arxiv.rs:181` (OAI 503 ×4); `R/arxiv_fetch.rs:380, 906` | No CLI test with the main `/api/query` returning 5xx until it gives up |
| `rate_limited` | `C/fetch_arxiv.rs:313` (403/406); `R/arxiv_fetch.rs:724` | — |
| `lock_contention` | `C/fetch_arxiv.rs:722, 854`; `R/arxiv_fetch.rs:809, 877`; `A/contention.rs:18` | — |
| Last retry's reason carried past the cap | `C/fetch_arxiv.rs:873` (`upstream_error`); `R/queue.rs:403`; `A/queue.rs:451, 472` | No CLI test carrying `rate_limited` |
| Ticket cap | Time cap only (900 s; see above). No cap on ticket count exists | — |
| `E_ARXIV_TICKET_MALFORMED` | `C/fetch_arxiv.rs:784`; `R/queue.rs:187, 210` | — |
| `E_ARXIV_TICKET_MISMATCH` | `C/fetch_arxiv.rs:547, 604, 619, 631`; `R/queue.rs:329`; `A/queue.rs:295` | — |
| `E_ARXIV_TICKET_LIVE` | `C/fetch_arxiv.rs:816`; `R/queue.rs:317`; `A/queue.rs:317, 334` | — |
| Classification (4xx/404/429/5xx) | `R/classify.rs:169, 176, 186`; `R/schedule.rs:17, 33` | 404 and other 4xx tested only at domain level |
| Release-captured queue state | None. Records are written inline as JSON (`C/support/mod.rs:202-227`, `A/support/mod.rs:124-142`) | The upgrade fixture is entirely new |

The test harness is reusable for Crossref:
- `http_test_support::MockHTTPServer` (std-only; routes keyed by method and
  path with the query string ignored; `Route::Headers`, `Sequence`, `Delayed`,
  `Stall`; `hit_instants`, `last_header`).
- `C/support/mod.rs`: `Project`, `seed_ticket`, `hold`, `spawn_in_real_time`,
  `fetch`, `Run`, `assert_golden`.
- `A/support/mod.rs` and `R/support/mod.rs`: `RecordingClock`,
  `ScriptedTransport`, `RecordingGate`, `ScriptedQueue`.
- Recorded bodies live in
  `cli/research-adapters/tests/fixtures/{arxiv,openalex,web}/`, loaded with
  `adapter_fixture(family, name)`.

The loopback clock (`research-cli/src/loopback.rs:194-247`):
- `ACCELERATOR_RESEARCH_TEST_CLOCK_LOG` turns on a non-sleeping clock: each
  wait adds to a virtual offset and is appended to a log.
- `…_CLOCK_EPOCH` fixes the wall time.
- `…_CALL_BUDGET_MS` shrinks the 100 s budget.

**Nothing shares a virtual time between processes.** PP-891's "project-scoped
test clock read by at least two separate processes" therefore needs a new
seam, such as a clock file in the project's scratch directory that every
process reads and advances. The alternative is real-time multi-process tests
like `arxiv_pacing.rs`.

### 3. The fetch CLI and adding a `doi` family

#### Structure
- clap collects raw strings (`research-cli/src/cli.rs:66-81`).
  `FetchRequest::parse` (`research/src/sources/request.rs:632-647`) owns every
  usage message.
- Families: `enum Family { OpenAlex, Arxiv, Web }`, with `Family::ALL: [Self;
  3]` (137-145) and `verbs()` (168-173).
- `UnknownFamily` hard-codes "(expected openalex, arxiv or web)" (38-42).
  `LimitNotApplicable` and `TicketNotQueued` also name families literally.
- `FetchRequest` and `SourceCall` are exhaustive enums
  (`fetch_command.rs:53-57`). Unit tests build one for every variant (692-746).
- `Fetched { Records, Page }` (`fetch_command.rs:91-94`). The `doi` output
  `{"status":"ok","metadata":{…},"venue":"…"}` is neither shape, so it needs a
  new document variant in `render.rs`.

#### Help
- `FETCH_HELP` (`cli.rs:11-52`) has a "Families and their verbs:" paragraph.
- The test `the_fetch_help_pairs_each_family_with_exactly_its_verbs`
  (`cli.rs:153-200`) requires one `;`-separated `"<family> takes <verbs>"`
  clause per `Family::ALL` entry.
- The `--ticket` paragraph (`cli.rs:39-52`) hard-codes `"source":"arxiv"` and
  `E_ARXIV_TICKET_LIVE`.

#### `Doi`
`request.rs:355-409`, `943-980`:
- It already accepts bare DOIs, `doi:` labels, `http(s)://doi.org/…` and
  `dx.doi.org/…` (case-insensitive). It strips a query or fragment and
  percent-decodes.
- `parse` returns `Option<Doi>` with no error of its own.
  `E_OPENALEX_ID_MALFORMED` is the only DOI-related code, so
  `E_DOI_MALFORMED` is a new `RequestError`.
- `canonical()` lowercases. `encoded()` percent-encodes for paths.
- Tests: `research/tests/request.rs:304-445`.

#### Endpoints and transport
- `Endpoint` (`request.rs:814-838`) has no Crossref base URL.
- Test overrides are `ACCELERATOR_*_API_URL` variables, loopback-only
  (`loopback.rs:26-82`), so Crossref needs `ACCELERATOR_CROSSREF_API_URL`.
- `UpstreamRequest { url, headers: Vec<(&'static str, String)>, bearer }`
  (`request.rs:840-888`). Header values are `String`, so a per-call
  User-Agent fits.
- `USER_AGENT` is a constant: `"accelerator-research/<ver>"` (10-11).
  `fetch_openalex.rs:533-546` pins it exactly for OpenAlex only.

#### Metadata
- `RECOMMENDED_KEYS` (`research/src/sources/metadata.rs:13-33`) is:
  `doi, arxiv_id, openalex_id, pmid, issn, published, updated, accessed, type,
  authors, publisher, …`.
- `web metadata` prints `doi`, `published`, `updated` and `accessed`, at
  indices 0, 5, 6 and 7. `publisher` is at index 10, so the "doi keys after web
  keys" invariant holds for `publisher`.
- `SourceMetadata` enforces order on insert (`metadata.rs:76-92`).
- There is **no `record_publisher`** today; recorders exist only for doi,
  arxiv_id, openalex_id and the three dates.
- `MetadataJson` serialises in order (`render.rs:181-196`).
- Tests: `research/tests/metadata.rs`.

#### `web metadata` behaviour
- Extraction: `research/src/sources/web/extract.rs:19-54` and
  `precedence.rs:51-105`.
- It follows up to five redirects across hosts and admits only public
  addresses (`web/fetch.rs:221-330`).
- Failures go to stderr as `E_WEB_*` with exit 1 and nothing on stdout. Exit 2
  is used for `E_WEB_URL_MALFORMED`.
- It records `doi` via `canonical()`, i.e. lowercased.

#### Architecture enforcement
- `cli/pup.ron:95-114` restricts `research` imports to `std`, `kernel::Error`,
  `network` and `crate`. A Crossref JSON decoder therefore belongs in
  `research-adapters`.
- If it should be covered by the decoders-spawn-nothing rule, add it to that
  rule's alternation (`pup.ron:115-131`).
- Every new public domain item moves
  `cli/research/tests/fixtures/public-api.txt`; refresh it with
  `mise run public-api:update`.

#### Guard
- `research/src/confinement.rs::command_decision` judges shell constructs, not
  family names. `fetch doi metadata '…'` passes with no guard change.
- `research/tests/fixtures/guard-boundary.tsv:80` has a `web metadata` row that
  could be mirrored.

#### Errors and warnings
- `usage()` exits 2 and `failure()` exits 1 (`main.rs:393-401`).
- `warning:` lines are printed with `eprintln!("warning: …")`
  (`fetch_command.rs:234-238`).
- Precedents for "invalid value → warn and fall back" are
  `subagent_cap.rs:54-79`, `search_cap.rs:45-61` and
  `conduct/concurrency.rs:115-144`. In all three a blank value counts as unset.

### 4. Config resolution for `crossref.mailto`

- **OpenAlex keys.** They are `EXTRA_KEYS` entries
  (`cli/config/src/catalogue.rs:263-272`), resolved by the credential ladder
  (`config/src/credentials.rs:194-256`).
  - Order: env plaintext, env command, `config.local.md` plaintext,
    `config.local.md` command, then `config.md` plaintext **only when
    `config.local.md` is absent**.
  - That differs from PP-891's first-non-empty-of-three rule.
- **`ConfigAccess`** (`config/src/service.rs:391-403, 429-479`).
  - `get(key, None)` gives personal over team, and `effective()` reports a
    `Source`.
  - It **reads no environment variables**; env overrides exist only inside the
    consent/credential code.
  - `environment_candidate` and `non_blank` (`consent.rs:1400-1488`) are
    `pub(crate)`.
- **Frontmatter shape.** Keys are dotted paths through nested YAML, so the
  setting is written `crossref:\n  mailto: …` (`service.rs:581-595`).
  `config-adapters/src/store.rs:169-256` maps the levels to
  `.accelerator/config{,.local}.md`. The personal file must be owner-only.
- **A registration in `EXTRA_KEYS`** (e.g. `ExtraKey::overridden(
  "crossref.mailto", Trust::Open, &["ACCELERATOR_CROSSREF_MAILTO"])`) touches:
  - `every_key_pins_its_overrides_and_recovery` (`catalogue.rs:646-692`);
  - the `config help` "Recognised keys" list
    (`launcher/src/launch/help.rs:52-78`, test
    `launcher/tests/config_help.rs:62-72`);
  - the `config dump` golden (`launcher/tests/fixtures/dump/dump.golden`).
    `mailto` is not in `CREDENTIAL_LEAVES`, so its value is shown, which is
    fine for a non-secret.
- **SessionStart "Configured sections"** lists the files' top-level keys with
  no registry (`launcher/src/config_command/core/summary.rs:161-174`). No change
  is needed.
- **Fetch plumbing.** The arXiv branch of `source_call` reads config only for
  `paths.tmp`. A Crossref branch needs the project, for scratch and config, but
  no credential ports.
- **Test helpers.**
  - `config_with(section)` (`C/support/mod.rs:325-327`) is hard-wired to
    `openalex:`.
  - `command()` removes `ACCELERATOR_OPENALEX_API_KEY{,_CMD}` (430-449). A new
    `env_remove("ACCELERATOR_CROSSREF_MAILTO")` keeps the tests hermetic.
- **`/configure` docs.** The `### openalex` section
  (`skills/config/configure/SKILL.md:1247-1300`) has:
  - an intro naming the consumer;
  - a settings table;
  - a numbered resolution chain;
  - a transport paragraph;
  - an example YAML block;
  - "Recognised keys".

  A `### crossref` section would mirror it. Sections sit alphabetically
  between `### shortcut` and `### templates`, so `crossref` goes earlier.

### 5. Research skill text

- **`skills/research/profiles/web-profile/SKILL.md`** (105 lines).
  - Frontmatter grants only `Bash(accelerator research fetch *)`.
  - Sections: Source Family (28-40: run `web metadata` for every cited page and
    copy its map; a non-zero exit leaves the entry without a block), then
    Untrusted-Content Contract, Reputation Tiers (54-73), Search Budget and
    Outcome.
  - Reputation Tiers is a domain-or-venue vocabulary with tier-1/2/3
    descriptions, "derived from venue identity only", and the lookalike-domain
    rule.
  - **No `doi.org` mention, no "Never re-judge a tier", no metadata template,
    no worked examples, no waiting or `E_*` handling.**
- **arXiv profile** (`skills/research/profiles/arxiv-profile/SKILL.md`). These
  are the passages the web profile should mirror:
  - "Run one fetch at a time" (44);
  - Outcome (76-99): Unavailable returns `source`/`reason`/`cause`;
    `E_ARXIV_TICKET_LIVE` → write no file and return the `E_*` line verbatim;
    other usage errors → correct and continue;
  - the Waiting section (101-113): re-run at once with `--ticket` set to the
    latest ticket, never change the arguments, and after 20 re-presentations
    end Unavailable with reason `waiting`;
  - every preprint is tier-2 (63-74).

  PP-891 differs: after 20 `waiting` results the web researcher handles the
  call as `unavailable` and "records no `reason`". This contrasts with arXiv's
  `reason waiting`.
- **Finding outputter.** `skills/research/outputters/finding-outputter/SKILL.md:88-139`
  says "Block lines are the CLI's `metadata` entries for the cited source,
  copied unchanged and in emitted order". It names no command. Its key table
  must equal `RECOMMENDED_KEYS`.
- **Citing-once.** `skills/research/outputters/citing-once/SKILL.md:54-62`
  ("Carrying a DOI") already reads a DOI from a `doi.org`/`dx.doi.org` URL.
  The metadata merge (112-117) takes the union of keys and the block order
  (128-156) is canonical, so the combined block needs no rule change, as
  PP-891 says.
- **Text checks.** `tests/unit/tasks/test_research_structure.py` constrains
  the change:
  - `:1784-1792` requires **every** fenced block in the web profile to
    fullmatch `accelerator research fetch web metadata '[^']+'`. This must
    change for `doi metadata`, `--ticket` and template examples.
  - `:1795-1814` pins existing web sentences and the order of the Untrusted
    Contract and Reputation Tiers headings.
  - `:1525-1551`: any new backticked `` `tier-N (…)` `` suffix must appear in
    citing-once.
  - `:1636-1680`: the finding outputter must not contain citing-once's
    RULE_SIGNATURES (including "`dx.doi.org`", "percent-decoded" and
    "case-insensitively"). The outputter's multi-command wording must avoid
    them.
  - `:819-874`: the arXiv waiting phrases. A web equivalent could be pinned the
    same way.
- **Profile invocations.** `tests/integration/research/test_profile_invocations.py`
  runs every fenced web command through the built `guard`
  (`FEWEST_INVOCATIONS["web"] = 1`).
- **Docs.** `docs-site/src/content/docs/research.md` documents the families.
  Line 127 says `--ticket` on a non-arXiv call is `E_RESEARCH_USAGE`; line 144
  says `web metadata` never retries; 262-275 cover CLI tiering. This page is
  checked by the structure tests and `docs:check`.

### 6. OpenAlex search/lookup parity

- **Shared path.** All request kinds use `OPENALEX_SELECT`
  (`request.rs:13-15`) and the `WorkJson` decoder
  (`research-adapters/src/openalex_json.rs`; `works()` for a page, `work()` for
  a single work). All then go through `openalex::normalise`
  (`research/src/sources/openalex.rs:152-173`, called from `fetch.rs:258-286`).
- **Derivation.**
  - `tier`: `tier.rs:8-18`.
  - `venue`: `primary_location.source.display_name`.
  - `venue_signals`: `openalex.rs:102-114`, with fields `source_type`,
    `version`, `is_core`, `listed_in`, `type` and `is_retracted`.
- **Paths** (`request.rs:553-580`).
  - Search: `/works?filter=title_and_abstract.search:<q>&per_page&select`.
    This is **not `filter=doi:`**.
  - Lookup by DOI and by resolver URL: both become
    `/works/doi:<encoded>`, because `Doi::parse` strips the resolver.
  - Lookup by W-id: `/works/W…`.
- **Test location.** `cli/research-cli/tests/fetch_openalex.rs` (loopback, mock
  server, `openalex_fixture`, `page_of`, `assert_golden`) is the right home.
  - `a_work_is_looked_up_by_id_by_openalex_url_and_by_doi` (246-272) asserts
    only counts.
  - No test uses a `https://doi.org/` lookup or compares tier, venue or signals
    across paths.
  - Fixtures are raw API JSON in `cli/research-adapters/tests/fixtures/openalex/`
    (`work-doi.json` is PeerJ, tier-1).
  - A recorded `10.1145/3448016.3457556` work would be served bare at
    `/works/doi:10.1145/3448016.3457556` and `/works/W…`, and wrapped by
    `page_of` at `/works`.
- `cli/research/tests/openalex_fetch.rs`, named in the work item, is a domain
  unit test with a stub decoder (no JSON), so it is a weaker home for a parity
  test.

## Code References

- `cli/research/src/sources/queue.rs:17-24` — EXPIRY, CAP, LONGEST_INVOCATION,
  ABANDONED_AFTER
- `cli/research/src/sources/queue.rs:60-114` — Ticket format and arXiv
  `Binding`
- `cli/research/src/sources/queue.rs:258-294` — `Queue::present` decision order
- `cli/research/src/sources/queue.rs:412-456` — `ArxivQueue`, `Joining`, `Place`
  ports
- `cli/research/src/sources/fetch.rs:299-364` — `fetch_arxiv`,
  `upstream_reason_or_contention`
- `cli/research/src/sources/fetch.rs:579-648` — `until_settled`, deferral
- `cli/research/src/sources/classify.rs:159-217` — per-service classification
- `cli/research/src/sources/schedule.rs:28-59` — backoff and Retry-After clamp
- `cli/research/src/sources/request.rs:10-15` — `USER_AGENT`, `OPENALEX_SELECT`
- `cli/research/src/sources/request.rs:84-91, 123-184` — ticket errors,
  `parse_ticket`, `Family`
- `cli/research/src/sources/request.rs:355-409, 943-980` — `Doi`
- `cli/research/src/sources/request.rs:553-580` — OpenAlex request paths
- `cli/research/src/sources/request.rs:617-663, 741-767` — `FetchRequest`,
  `WebRequest`
- `cli/research/src/sources/request.rs:814-888` — `Endpoint`, `UpstreamRequest`
- `cli/research/src/sources/metadata.rs:13-92` — `RECOMMENDED_KEYS`,
  `SourceMetadata`
- `cli/research/src/sources/openalex.rs:102-173` — venue signals, `normalise`
- `cli/research/src/sources/web/extract.rs:19-54` — declared DOI and dates
- `cli/research-adapters/src/pacing.rs:53-284` — arXiv gate, file names,
  finish-based spacing
- `cli/research-adapters/src/queue.rs:52-60, 260-313, 340-348, 640-733` — queue
  directory, housekeeping, schema filter, records
- `cli/research-adapters/src/contention.rs:16-65` — contention log
- `cli/research-adapters/src/transport.rs:80-153` — HTTP transport,
  Retry-After parsing
- `cli/research-cli/src/cli.rs:11-81, 153-200` — fetch help, `--ticket`, help
  pairing test
- `cli/research-cli/src/main.rs:90-171, 273-401` — budgets, dispatch, exit
  codes
- `cli/research-cli/src/fetch_command.rs:53-133, 202-260` — `SourceCall`,
  `Fetched`, key resolution
- `cli/research-cli/src/render.rs:21-196` — documents, rejections, metadata
  JSON
- `cli/research-cli/src/loopback.rs:26-247` — test endpoint and clock seams
- `cli/config/src/catalogue.rs:135-281, 646-692` — `Trust`, `EXTRA_KEYS`, the
  pinned test
- `cli/config/src/credentials.rs:194-256` — credential ladder
- `cli/config/src/service.rs:391-479` — `ConfigAccess`
- `cli/pup.ron:95-131` — research import rules
- `skills/research/profiles/web-profile/SKILL.md:28-73` — metadata fetch,
  Reputation Tiers
- `skills/research/profiles/arxiv-profile/SKILL.md:40-113` — call rules,
  Outcome, Waiting
- `skills/research/outputters/finding-outputter/SKILL.md:88-139` — Sources
  Metadata Block
- `skills/research/outputters/citing-once/SKILL.md:54-62, 112-156` — Carrying a
  DOI, merge, block order
- `skills/config/configure/SKILL.md:1247-1300` — `### openalex` section
- `tests/unit/tasks/test_research_structure.py:1784-1792` — web fenced-block
  constraint
- `cli/research-cli/tests/support/mod.rs:87, 185-252, 325-449` — fixtures,
  seeding, config, command env
- `cli/research-cli/tests/fetch_openalex.rs:246-272, 533-546` — lookup test, UA
  pin
- `cli/http-test-support/src/lib.rs` — `MockHTTPServer`

## Architecture Insights

- **Policy in the domain, mechanics in the adapters.** Queue rules,
  classification, schedule and request building live in the pure `research`
  crate. Files, `flock`, serde and HTTP live in `research-adapters`. Only
  `research-cli` wires them. A Crossref service splits the same way:
  - domain: `CrossrefRequest`/`DoiRequest`, `classify::crossref`, a venue rule
    on the Crossref `type`, a `record_publisher` recorder;
  - adapters: a Crossref JSON decoder, and the gate and queue parameterised by
    service;
  - CLI: `source_call` wiring, the document variant, help text.

  ADR-0069 discourages a new crate for the generic engine, because queue policy
  is research-domain policy.
- **Seams that already generalise.**
  - Spacing flows from `PacingGate::spacing()` into the serving window and
    `join`.
  - `ContentionLog`, `Queue`/`Place`, and the fail-open degradations are
    service-neutral.
  - PP-888 item 2 (a domain `ServingWindow` type) is the natural vehicle for
    per-service pacing.
- **Usage messages are domain Display impls.** Every `E_*` line is a domain
  `Display`, except the ticket rejections, which `render.rs` formats.
  Prefixing ticket codes per family means moving that formatting next to the
  family, or parameterising it.
- **Metadata order is a contract enforced by construction.** `SourceMetadata`
  sorts on insert and `MetadataJson` serialises in order. That makes PP-891's
  "`doi` keys after `web` keys" property testable directly against
  `RECOMMENDED_KEYS` ranks.
- **Skill text is tested like code.** The structure tests pin phrases and
  fenced-block shapes. Profile changes must update them in step.

## Historical Context

- `meta/plans/2026-10-06-PP-882-fair-arxiv-fetch-queue.md` and
  `meta/research/codebase/2026-10-06-PP-882-fair-arxiv-fetch-queue.md` explain
  the queue design:
  - whole-call admission under `arxiv.lock`;
  - the serving window of spacing + per-request budget;
  - tickets `n-nonce`, where the nonce makes a ticket from a wiped queue
    unknown so it joins;
  - the 100/300/900/1000 s bounds;
  - lock order;
  - throttling-only deferral sharing;
  - fail-open degradations.
- `meta/reviews/prs/148-review-1.md` lists the unresolved findings PP-891
  should design around:
  - a resumed ticket whose `own()` fails turns into `Unqueued`, then
    `lock_contention`;
  - older binaries prune records with a future schema version;
  - the `is_front`/`try_serve` FIFO inversion;
  - spacing across mixed versions depends on the shared `arxiv.lock`.
- `meta/work/PP-888-fair-arxiv-queue-follow-ups-from-the-pr-148-review.md`
  (draft, all open) has items that overlap the generalisation:
  - split `queue.rs`;
  - a `ServingWindow` type;
  - a `TicketError`;
  - the `FetchOutcome` vs `ArxivOutcome` decision;
  - `Binding` as a newtype.
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  and `ADR-0053-…` set the crate roles and import rules.
- `meta/plans/2026-10-24-PP-936-structured-source-metadata.md` covers
  `RECOMMENDED_KEYS` and the typed recorders.
- `meta/plans/2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once.md`
  covers DOI identity from resolver URLs and the metadata merge.
- `meta/plans/2026-09-25-PP-756-unify-the-trust-barrier-for-consent-config-keys.md`
  covers the consent and credential ladder, and why `api_key_cmd` is a consent
  key.
- `meta/work/PP-897-canonical-domain-classes-for-web-tiers.md` is a draft that
  also rewrites the web profile's Reputation Tiers. Coordination is noted in
  PP-891.

## Related Research

- `meta/research/codebase/2026-10-06-PP-882-fair-arxiv-fetch-queue.md`
- `meta/research/codebase/2026-09-23-PP-864-academic-source-profiles.md`
- `meta/research/codebase/2026-10-24-PP-936-structured-source-metadata.md`
- `meta/research/codebase/2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once.md`
- `meta/research/codebase/2026-09-24-PP-756-unify-the-trust-barrier-for-consent-config-keys.md`
- `meta/research/codebase/2026-10-06-PP-881-crate-dependencies-adr-0069.md`
- `meta/research/codebase/2026-10-03-cli-surface-inventory.md`

## Open Questions

1. **Pacing model.** Should the engine gain a start-to-start interval with N
   serving slots alongside arXiv's finish-based single slot? Or should arXiv
   move to the new model with parameters that preserve its observable spacing?
   The arXiv tests (`arxiv_pacing.rs`) pin "3 s after the previous response
   finishes".
2. **Ticket family encoding.** Should the family be a prefix in the ticket text
   (e.g. `doi-1-9f1c2a`)? Alternatively it could live in the record, with a
   lookup across family directories. Either way:
   - a legacy `n-hex` ticket must give `E_ARXIV_TICKET_MISMATCH`, not
     `MALFORMED`;
   - an arXiv ticket shown to `doi` must give `E_DOI_TICKET_MISMATCH`, not a
     silent Join.
3. **What "queue state apart" covers.** Does it include `arxiv.lock` and
   `arxiv-pacing`? Keeping those shared preserves arXiv's one-connection
   guarantee across the upgrade overlap at no cost. Only the ticket directory
   needs to move.
4. **Shared test clock.** What form should it take: a file in scratch that
   `LoggingClock` reads and advances under a lock, or real-time tests? The
   rate, concurrency, mixed-pool and cross-process Retry-After criteria depend
   on it.
5. **Throttling set for Crossref.** Crossref's 403 counts as
   `upstream_error` under PP-891's "any other 4xx" rule, while arXiv treats 403
   as throttling. Should a non-404, non-429 4xx retry, or settle `unavailable`
   at once?
6. **Parity criterion wording.** The criterion names a `filter=doi:` endpoint,
   but `search` uses `title_and_abstract.search`. Should the item be corrected
   to "the `/works` search endpoint"?
7. **Context claim to correct.** PP-891's Context says "The profiles say 'Never
   re-judge a tier'", but the web profile does not. That sentence holds only
   for the composers' carry-forward behaviour.
8. **Unqueued-path behaviour.** Should a Crossref call that cannot use the
   queue report `lock_contention`, as arXiv does? Or should PP-891 fix the
   PP-882 review finding that a resumed ticket failing `own()` loses its
   `waiting`?
