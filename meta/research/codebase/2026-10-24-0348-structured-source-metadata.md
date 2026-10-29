---
type: "codebase-research"
id: "2026-10-24-0348-structured-source-metadata"
title: "Structured Source Metadata (0348): current pipeline and implementation surface"
date: "2026-10-24T13:38:20+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0348"
parent: "work-item:0348"
relates_to: ["codebase-research:2026-09-23-0280-academic-source-profiles", "codebase-research:2026-09-26-0283-recursive-finding-deepening", "codebase-research:2026-09-08-0277-single-round-web-research-engine"]
topic: "Structured Source Metadata (0348): current pipeline and implementation surface"
tags: ["research", "codebase", "research-cli", "research-adapters", "sources", "metadata", "web-profile", "confinement"]
revision: "f979dc4a6d5e5bc9e38aa5486ed3163c6c317f7f"
repository: "accelerator"
last_updated: "2026-10-24T13:38:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Structured Source Metadata (0348): current pipeline and implementation surface

**Date**: 2026-10-24T13:38:20+00:00
**Author**: Toby Clemson
**Git Commit**: f979dc4a6d5e5bc9e38aa5486ed3163c6c317f7f (working copy on `1e01bff466`)
**Branch**: detached (`default@`)
**Repository**: accelerator

## Research Question

What does the codebase look like today against work item 0348 (Structured
Source Metadata), and what must change to deliver it: a `metadata` map on
every OpenAlex and arXiv record, a new `accelerator research fetch web
metadata <url>` command, profile and agent instructions that copy the map
into an optional block under each Sources entry, and byte-for-byte
carry-forward through composers and `synthesise`?

## Summary

0348 splits cleanly along the existing hexagonal seams, but the web metadata
command is substantially more work than the academic half and collides with
three existing policies.

- **Academic records** need small, local changes. A domain `Clock` with
  `wall_now()` already reaches both fetch workflows; the normalisers do not
  receive it. `WorkId` and `ArxivId` already hold the bare `openalex_id` and
  versioned `arxiv_id` but discard them. `OPENALEX_SELECT` lacks
  `publication_date`, and the arXiv Atom decoder never reads `<published>` or
  `<updated>`.
- **Domain purity** is enforced: `cli/research` has no dependencies and
  pup restricts it to `std|core|alloc`, `kernel::Error` and `crate`
  (`cli/pup.ron:95-112`). Date normalisation, DOI normalisation, scheme
  precedence and host classification must be std-only in the domain; HTML and
  JSON-LD parsing must sit in `research-adapters` behind a new decoder port.
- **The web command conflicts with the existing transport**: `HttpTransport`
  follows at most 3 same-origin redirects, never reads `content-type`, and
  maps 5xx and timeouts to `Unavailable` (exit 0). 0348 needs 5 cross-host
  hops with a per-hop host check, a non-HTML failure, and non-zero exit on
  every failure.
- **No HTML parser** is in `cli/Cargo.lock`, and `cli/deny.toml` denies
  multiple versions and unmaintained crates, so choosing one is a real
  decision.
- **Ordered JSON** requires a hand-written `Serialize`; `preserve_order` is
  enabled nowhere and must not be, since several crates rely on BTreeMap
  order.
- **Skills side**: no code parses Sources sections (confirmed). The research
  guard admits `fetch web metadata` unchanged, but URLs must be
  single-quoted. The web profile has no `allowed-tools` today — 0348's
  Technical Notes are wrong that every profile grants the fetch — though
  `research-topic`'s grant already reaches spawned researchers.
- **Item text vs. code**: 0348's Technical Notes cite `sources/fetch.rs`
  dispatch correctly but place decoding in `cli/research`; decoding lives in
  `cli/research-adapters/src/{openalex_json,arxiv_xml}.rs`.

## Detailed Findings

### Academic record pipeline

Data flow from response to stdout:

```text
HTTP body ─classify─▶ Verdict::Deliver(body)
   ─decoder port (research-adapters: serde_json / roxmltree)─▶ Work | Entry
   ─normalise (research)─▶ Record
   ─render (research-cli)─▶ RecordJson ─serde_json::to_string─▶ stdout
```

- `Record` (`cli/research/src/sources/record.rs:46-57`) has `pub(crate)`
  fields `title, authors, url, venue, venue_signals, abstract_excerpt, tier,
  retracted, withdrawn`, no `Serialize`, and is "built only by a source's
  normaliser" (`record.rs:42-45`). A `metadata` field needs filling in both
  normalisers and an accessor for `render.rs`.
- arXiv's DOI sits in `VenueSignals::Arxiv { doi }` (`record.rs:36-39`),
  copied verbatim from `entry.doi` (`arxiv.rs:89-92`): neither validated nor
  lowercased. OpenAlex's DOI exists only inside `url`
  (`openalex.rs:75-81`).
- `openalex::normalise` (`openalex.rs:133-153`) builds `WorkId` via
  `WorkId::from_openalex` (`request.rs:338-350`, which already strips
  `https://openalex.org/`) and uses it only for the fallback URL; the bare id
  is reachable via `WorkId::as_str()` (`request.rs:352`).
- `arxiv::normalise` (`arxiv.rs:79-102`) parses `ArxivId` with
  `unversioned` and `version` (`request.rs:210-213`); `Display` already
  prints `2305.12345v2` (`request.rs:254-260`), but `url` uses
  `abstract_url()` (`request.rs:241-243`), which drops the version
  (`arxiv.rs:84`). Lookups request the unversioned id and `listed_entries`
  matches by unversioned id (`fetch.rs:479-494`), so the feed `<id>` version
  is the version retrieved, as 0348 requires.
- `OPENALEX_SELECT` (`request.rs:11-13`) is
  `id,doi,display_name,type,authorships,primary_location,is_retracted,abstract_inverted_index`,
  used by all three OpenAlex request shapes (`request.rs:480-507`).
  `WorkJson` (`cli/research-adapters/src/openalex_json.rs:39-50`) and `Work`
  (`openalex.rs:14-24`) have no `publication_date`.
- The arXiv decoder (`cli/research-adapters/src/arxiv_xml/atom.rs:37-61`)
  reads `id`, `title`, authors, `comment`, `journal_ref`, `doi`,
  `primary_category` by namespace URI with roxmltree 0.21; entry-level
  `<published>` and `<updated>` exist in the recorded fixture
  (`cli/research-adapters/tests/fixtures/arxiv/lookup-2608.21129.xml:13,19`)
  but are unread. `child(node, ATOM, …)` only looks at direct children, so
  the feed-level `<updated>` cannot be mistaken for the entry's.
- `tier.rs` reads `Work` and a `withdrawn` bool, never `Record`; `classify.rs`
  deals only with responses. Neither is affected beyond new `Work`/`Entry`
  fields not changing tiering.

### `accessed` and the clock

- `research::sources::fetch::Clock` (`fetch.rs:77-83`) exposes `now()`,
  `wall_now() -> SystemTime` ("stamps values persisted for other processes to
  read") and `sleep()`. It is already in `OpenAlexPorts` and `ArxivPorts`
  (`fetch.rs:205-226`), but `normalise` is called without it at
  `fetch.rs:283` and `fetch.rs:470`, so the normaliser signatures change and
  every call in `cli/research/tests/{openalex,arxiv}.rs` moves.
- No research crate or `kernel` converts `SystemTime` to a UTC calendar date.
  `time` 0.3 is a workspace dependency (`cli/Cargo.toml:71`) but pup forbids
  it in the domain. Options: a std-only days-from-civil conversion in the
  domain (no precedent exists), or format in an adapter and inject a date.
  The nearest precedent for `YYYY-MM-DD` UTC formatting is
  `FilenameTimestampFormat::DateOnly` in
  `cli/corpus-adapters/src/metadata.rs:81-83`.
- Test clocks: `RecordingClock` (`cli/research/tests/support/mod.rs:116-130`)
  sits at epoch `1_800_000_000` s = 2027-01-15T08:00Z; the binary tests'
  `CLOCK_EPOCH_MS = "1800000000000"`
  (`cli/research-cli/tests/support/mod.rs:24`) through `LoggingClock`
  (`cli/research-cli/src/loopback.rs:98-127`); `VirtualClock`
  (`cli/research-cli/src/fetch_command.rs:361-374`) yields 1970-01-01.

### Ordered `metadata` serialisation

- `render.rs` (`cli/research-cli/src/render.rs:18-38, 111-178`) builds a
  `#[serde(tag = "status")]` `Document` with `RecordJson` derive-ordered
  fields and prints compact JSON (`main.rs:360-361`).
- `serde_json` has no `preserve_order` anywhere (`cli/Cargo.lock:4418-4428`
  lists no `indexmap` dependency). Enabling it is unsafe: Cargo unifies
  features and `work-adapters/src/sync/baseline.rs:436-439`,
  `remote-projection/src/lib.rs:48-54`, `corpus-adapters/src/jsonl.rs:158-164`
  and the guard test `jira-client/tests/projection_corpus.rs:75-85` rely on
  sorted `Map`s.
- Model: `document/src/value.rs:31-69, 157-190`, a `Vec<(String, _)>`
  mapping with a hand-written `serialize_map` impl. The domain owns the
  ordered pairs (recommended-table order, then byte-wise); `render.rs`
  serialises them.
- `Run::json()` (`cli/research-cli/tests/support/mod.rs:333`) parses into
  `serde_json::Value`, losing order, so order assertions must use the raw
  goldens (`assert_golden`, `support/mod.rs:419-434`) or the domain type.

### Fetch dispatch and the web metadata command

- Request model: `Family {OpenAlex, Arxiv}` (`request.rs:100-126`) and `Verb
  {Search, Lookup}` (`request.rs:137-161`) are closed; `FetchRequest::parse`
  (`request.rs:544-609`) rejects `web` as `UnknownFamily` ("expected openalex
  or arxiv", `request.rs:33-37`). `UpstreamRequest` (`request.rs:683-729`)
  has no public constructor for an arbitrary URL.
- clap (`cli/research-cli/src/cli.rs:45-64`) takes family, verb and terms as
  raw strings, so `fetch web metadata <url>` already parses; only the domain
  rejects it. `FETCH_HELP` (`cli.rs:11-36`) needs the new usage; the binary
  description names only OpenAlex and arXiv.
- Launcher dispatch is generic `external_subcommand` forwarding
  (`cli/launcher/src/launch/inbound/cli.rs:32-34`). Per `tasks/README.md:769-788`
  a new subcommand of an existing sub-binary needs no registration.
- Ports (`fetch.rs`): `Transport::send(&UpstreamRequest) -> Response`
  (39-41), the decoder ports (50-75), `Clock`, `PacingGate`, and
  per-family port bundles. `FetchOutcome` (191-203) is
  `Records | Unavailable | Failed | Waiting | Rejected`.
- `Received` (`cli/research/src/sources/classify.rs:21-27`) carries `status`,
  `retry_after`, two rate-limit fields and `body` — no `content_type`, no
  `location`, no final URL. A 3xx becomes `ClientRejection::Redirect`
  (`classify.rs:207-217`). `FetchError` codes are keyed by `Family`
  (`classify.rs:87-130`).
- `HttpTransport` (`cli/research-adapters/src/transport.rs`): reqwest
  0.12.28 blocking, rustls/ring, `pool_max_idle_per_host(0)`,
  `MAX_REDIRECTS = 3` with `follow_within_origin` (same scheme, host, port,
  `transport.rs:83-100`), an 8 MiB cap that clears rather than fails, and
  errors flattened to `TimedOut`/`ConnectionFailed` so no URL leaks.
- Exit codes (`main.rs:89, 350-374`): usage → 2, `Failed` → 1 with no stdout,
  `Records`/`Unavailable`/`Waiting` → 0 with JSON. Under today's
  classification a 5xx or timeout is `Unavailable`, which would violate
  0348's "exit non-zero, no map" rule; the web arm needs its own outcome
  mapping. The success JSON shape for the web command is unspecified in
  0348.
- No URL, host, IP or DNS checks exist in any research crate. The only
  loopback check is the test-only prefix match in
  `cli/research-cli/src/loopback.rs:21-22,53-70`. `hickory-dns` is a reqwest
  feature only; no custom `Resolve` exists in the workspace.

Two shapes satisfy the fetch rules; neither is verified:

1. **Domain-driven redirect loop.** The web transport uses `Policy::none()`
   and returns `location` and `content_type`; a new `HostResolver` port
   resolves each hop and the domain classifies every address before the
   next request. Testable entirely through injected fakes, as 0348's
   Verification requires. Residual risk: DNS rebinding between the check and
   reqwest's own resolution.
2. **Adapter-enforced.** A reqwest `dns_resolver` that filters non-public
   addresses plus a custom redirect policy. Closes the rebinding gap but
   puts the rule outside the domain and makes it hard to test without a
   network.

A hybrid — domain loop for scheme, hop count and literal-IP checks, plus a
filtering resolver in the adapter — gets both, at the cost of the rule
living in two places.

### Host classification precedent

`cli/design` already implements the chain 0348 needs, pre-resolution only:

- `design/src/source_location.rs:94-125`: `http`/`https` allow-list, named
  refusals for `file`, `javascript`, `data` and others.
- `design/src/host.rs:61-168`: `Host::canonicalise` rejects userinfo,
  control characters and numeric IPv4 encodings (`0x7f000001`), lowercases,
  strips port, brackets, zone id and trailing dot.
- `design/src/host_reach.rs:29-178`: `HostReach {Loopback, Private,
  LinkLocal, Reserved, Unspecified, Public}`, including CGNAT and v6 forms
  that embed v4 (6to4, Teredo, NAT64, mapped). The module doc
  (`host_reach.rs:5-7`) states a public hostname resolving to
  `169.254.169.254` still classifies as Public.
- Test corpus: `design/tests/host_classification_vectors.rs` with
  `design/tests/fixtures/host-classification-vectors.json`.

`research` cannot depend on `design` (pup and the crate-dependency lint
forbid product-domain edges), so this is a copy or a promotion into a shared
crate. Classifying resolved addresses, not just literals, is new either way.

### HTML and JSON-LD extraction

- No HTML parser exists in `cli/Cargo.lock` (no scraper, html5ever,
  lol_html, tl, kuchiki). roxmltree is strict XML and unusable on real HTML.
- `cli/deny.toml`: licence allow-list (52-64), `multiple-versions = "deny"`
  (117), `unmaintained = "all"` (24), and wrapper-only bans (173-181) that
  could confine a parser to `research-adapters`. The html5ever stack's
  transitive closure (`phf`, `string_cache`, `tendril`, `cssparser`) is
  likely to need skips; unverified.
- Pattern to follow: decoder adapter implements a domain port and returns
  typed declarations, as `arxiv_xml.rs:17-72` does. Extraction returns raw
  `(scheme, field, value)` declarations in document order plus JSON-LD
  nodes; precedence, the usable-value rule and normalisation stay in the
  domain so 0349 can extend per-scheme field lists.
- JSON-LD node walking has precedent in `adf/src/render.rs:53-83`
  (`Value::get(..).and_then(Value::as_str)` helpers).
- A new decoder module must join the
  `research_adapters_decoders_spawn_nothing` alternation
  (`cli/pup.ron:113-129`) with a probe pair in
  `tests/integration/pup/test_import_rule.py`.

### Identifier and date value types

- `Doi` (`request.rs:305-332`) validates `10.<4–9 digits>/<suffix>`, no
  whitespace, no `.`/`..` segment, but keeps the raw case and exposes only
  `encoded()` and `url()`. Prefix stripping (`doi:`, `https://doi.org/`)
  is duplicated in callers (`OpenAlexId::parse`, `request.rs:378-381`;
  `openalex.rs:78`). A lowercase bare form for `metadata` needs a new
  accessor or a constructor that accepts any declared form. Open: whether a
  DOI failing `Doi::parse` is omitted from `metadata` (0348 only says
  "starts with `10.`").
- Lowercasing changes nothing about `url`: 0348 keeps `doi.org/<doi>` as
  0280 set it, and `cli/research/tests/openalex.rs:59` already asserts the
  lowercase fixture DOI.
- No partial-date parser exists. Closest shape-checkers:
  `corpus/src/frontmatter_validation/mod.rs:104-139` (`is_iso_timestamp`);
  validating-parser model `corpus-adapters/src/metadata.rs:156-171`.
  `tracker/src/lib.rs:41-77` deliberately keeps timestamps verbatim, a
  contrast worth noting since 0348 discards offsets by design.

### Skills, agents and templates

- Sources format today is a single example line in
  `skills/research/outputters/finding-outputter/SKILL.md:65-73` and in
  `templates/topic-research-{finding,level-note,synthesis}.md`
  (`:31-32`, `:34-35`, `:32-33`).
- `level-note-outputter/SKILL.md:14-18` defers Shape, Body and Sources to
  `finding-outputter` "so the two cannot drift apart" (0283's design). 0348's
  criterion that both outputters document the block is in tension with this;
  documenting in `finding-outputter` and referencing it from the level-note
  outputter keeps 0283's rule. Its cited "untrusted-content contract" in
  `finding-outputter` does not exist; the contracts live in profiles and
  agents.
- Profiles:
  - `web-profile/SKILL.md:1-8` has no `allowed-tools`. Its Denied outcome
    (`:71-74`) means "write no file", so a failed metadata call must be
    stated to mean "no block", not Denied. 0311 edits Reputation Tiers
    (`:36-55`); 0348's instruction should go elsewhere to avoid a conflict.
  - `openalex-profile/SKILL.md:9-10` and `arxiv-profile/SKILL.md:9-10` grant
    `Bash(accelerator research fetch *)`. Citing instructions are at
    `openalex-profile:58` and `arxiv-profile:64`; Untrusted-Content
    Contracts at `:45-54` and `:51-60`.
- `research-topic/SKILL.md:19` grants `Bash(accelerator research fetch *)`
  and that grant reaches spawned researchers
  (`docs-site/src/content/docs/research.md:379-384`), so the web profile's
  new frontmatter grant is for coherence and tests, not for permission.
- `agents/researcher.md:29-32` already allows "no CLI except `accelerator
  research fetch`"; no change is required. Its body must not name source
  families (`tests/unit/tasks/test_research_structure.py:106-111`).
- `agents/composer.md:28-30` carries tier text and venue verbatim; it must
  widen to the whole block. Tools are `Read, Write` and the guard blocks all
  its commands.
- `synthesise` (`research-topic/SKILL.md:643-651`) carries tiers and domains;
  it must add the byte-for-byte block and keep block-less entries one-line.

### Researcher Bash guard

- `hooks/hooks.json:41-63` runs `accelerator research guard` on `Bash`
  `PreToolUse`. `confinement.rs:13` permits the prefix
  `accelerator research fetch ` with no family or verb list, so `fetch web
  metadata` passes unchanged.
- Unquoted newline, `$`, backtick, `;`, `|`, `&`, `(`, `)`, `{`, `}`, `<` and
  most `>` block (`confinement.rs:417-457`); single quotes pass everything
  (`:362-366`). Claude Code itself denies an unquoted `&`
  (`cli/research/tests/fixtures/claude-bash-baseline.tsv:12`). The web
  profile must instruct single-quoting; a URL containing `'` needs `%27`.
- No guard code or fixture change is required. An optional
  `guard-boundary.tsv` row would document the quoted form.

### Python structure tests

- `_academic_profiles()` (`tests/unit/tasks/test_research_structure.py:87-93`)
  defines "academic" as "grants the fetch". Granting it in the web profile
  silently enrols web; `test_every_skill_injecting_an_academic_profile_grants_the_fetch`
  (`:792-806`) still passes because only `research-topic` injects profiles.
  The naming becomes wrong.
- The `ACADEMIC_FAMILIES` parametrised tests (`:39`, `:114-152`) stay keyed to
  `{openalex, arxiv}`; switching them to `_academic_profiles()` would fail web.
- `tests/integration/research/test_profile_invocations.py:69-83` runs only
  openalex and arxiv; adding web needs ≥3 fenced invocations or an adapted
  threshold, and `WORKED_QUERY` (`:26`) assumes `search`.

## Code References

- `cli/research/src/sources/record.rs:26-57` — `VenueSignals`, `Record`
- `cli/research/src/sources/openalex.rs:14-38, 75-81, 133-153` — `Work`, `citation_url`, `normalise`
- `cli/research/src/sources/arxiv.rs:10-20, 79-102` — `Entry`, `normalise` (version dropped at `:84`)
- `cli/research/src/sources/request.rs:11-13` — `OPENALEX_SELECT`
- `cli/research/src/sources/request.rs:100-161, 544-609` — `Family`, `Verb`, `FetchRequest::parse`
- `cli/research/src/sources/request.rs:207-359` — `ArxivId`, `Doi`, `WorkId`
- `cli/research/src/sources/request.rs:683-729` — `UpstreamRequest`
- `cli/research/src/sources/classify.rs:14-27, 207-217` — `Response`, `Received`, redirect rejection
- `cli/research/src/sources/fetch.rs:39-83, 191-226, 283, 470` — ports, outcomes, normaliser call sites
- `cli/research/src/confinement.rs:13, 307-316, 417-457` — guard prefix and lexer
- `cli/research-adapters/src/transport.rs:21-22, 52-61, 83-158` — redirect policy, caps, error flattening
- `cli/research-adapters/src/openalex_json.rs:39-97` — `WorkJson`
- `cli/research-adapters/src/arxiv_xml/atom.rs:37-65` — entry decoding
- `cli/research-adapters/tests/http_transport.rs:123-247` — redirect and cap tests to model on
- `cli/research-cli/src/cli.rs:11-80` — `FETCH_HELP`, command tree
- `cli/research-cli/src/main.rs:72-92, 136-180, 282-387` — loopback guard, fetch, `source_call`, `selected_clock`
- `cli/research-cli/src/render.rs:18-38, 111-204` — `Document`, `RecordJson`
- `cli/research-cli/src/loopback.rs:15-127` — loopback overrides and `LoggingClock`
- `cli/research-cli/tests/fixtures/goldens/{openalex-search,arxiv-search}.json` — goldens that gain `metadata`
- `cli/research/tests/fixtures/public-api.txt` — snapshot regenerated by `mise run public-api:update`
- `cli/research/tests/support/mod.rs:69-220, 354-413` — `RecordingClock`, `ScriptedTransport`, stub decoders
- `cli/research-adapters/tests/fixtures/openalex/work-doi.json` — lacks `publication_date`
- `cli/pup.ron:95-129` — research domain import rule, decoder spawn rule
- `cli/deny.toml:24, 52-64, 117, 173-181` — advisories, licences, multiple-versions, wrapper bans
- `cli/design/src/{source_location,host,host_reach}.rs` — scheme, host and reach precedent
- `cli/document/src/value.rs:31-69, 157-190` — ordered mapping with custom `Serialize`
- `skills/research/outputters/finding-outputter/SKILL.md:65-77` — Sources spec
- `skills/research/outputters/level-note-outputter/SKILL.md:14-18` — deferral to finding-outputter
- `skills/research/profiles/web-profile/SKILL.md:1-8, 26-34, 36-55, 66-86` — frontmatter, contract, tiers, outcome
- `agents/composer.md:28-30` — carry rule
- `skills/research/research-topic/SKILL.md:19, 643-651` — fetch grant, `synthesise` carry-forward
- `tests/unit/tasks/test_research_structure.py:39, 87-93, 792-806` — academic-profile detection
- `tests/integration/research/test_profile_invocations.py:26, 69-83` — guard replay of profile invocations

## Architecture Insights

- **Hexagonal split is strict and machine-checked.** ADR-0053 and ADR-0069,
  enforced by pup and `tasks/lint/crate_dependencies.py`, put every parser
  and network call in `research-adapters` and wiring in `research-cli`. The
  web command therefore decomposes into a domain module (URL and host
  rules, precedence, normalisation, ordered map), new ports (page fetch,
  host resolution, HTML declaration decoder), adapters, and a composition
  arm.
- **CLI-only producer follows ADR-0045.** Moving extraction out of the
  model is consistent with deterministic work living in the compiled CLI.
- **Static distribution constrains dependencies.** ADR-0046 requires
  pure-Rust, rustls, musl-friendly crates; the workspace already uses
  `hickory-dns` because getaddrinfo fails under musl.
- **Error text never contains a URL** (`transport.rs:135-151`). The web
  command's errors should keep that rule, since page URLs are untrusted.
- **Ports are narrow and context-owned.** Each context defines its own
  `Clock`; a `HostResolver` or `PageFetcher` port belongs in `research`, not
  shared.
- **0349 extends what 0348 builds.** Precedence tables should be data
  (per-scheme field lists) and the byte-wise tail ordering should exist from
  the start, even though 0348 emits only core keys.

## Historical Context

- `meta/reviews/work/0348-structured-source-metadata-review-1.md` — verdict
  APPROVE after three passes. The pass 3 minors are still open: both
  injection criteria pass on the usable-value filter alone, so the
  agent-level tool-log check is the real test; nothing tests key order in
  the web map.
- `meta/reviews/work/0306-deterministic-rule-for-citing-a-source-once-review-1.md`
  — origin of 0348 (pass 2). 0306 declined an uppercase-DOI criterion
  because 0348 lowercases, and relies on `accessed` sorting lexically.
- `meta/work/0306-deterministic-rule-for-citing-a-source-once.md` — merges
  blocks key-by-key from the first citation and orders "as 0348 orders a
  metadata block"; 0348's order rule is 0306's merge order.
- `meta/work/0349-fill-bibliographic-source-metadata.md` — extends the web
  command and normalisers; needs extensible per-scheme field lists, Highwire
  `@type` inference and nested JSON-LD objects later.
- `meta/work/0305-tier-doi-links-by-venue-not-resolver.md` — keeps `doi`;
  its venue-discovery question stays open, since 0348 emits no venue.
- `meta/work/0121-topic-research-skillset.md:78, 87-89, 100` — additions
  must be backward-compatible because findings are immutable; Sources of
  syntheses are derived, never authored. 0348 must add the optional block
  to its Sources contract.
- `meta/plans/2026-09-23-0280-academic-source-profiles.md` — record shape,
  decoder ports, transport policy, exit codes, `Clock` port, guard prefix.
- `meta/plans/2026-09-26-0283-recursive-finding-deepening.md` — composer
  tools and carry rule; level-note outputter deferral.
- `meta/plans/2026-09-09-0277-single-round-web-research-engine.md` — web
  source family excludes `file://` and private ranges; the researcher was
  originally Bash-less because it reads attacker-controlled content.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md`,
  `ADR-0046-zero-setup-static-binary-distribution.md`,
  `ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`,
  `ADR-0055-inspect-as-the-skill-evaluation-harness.md`,
  `ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`.
  No ADR covers the research CLI or the guard specifically.

## Related Research

- `meta/research/codebase/2026-09-23-0280-academic-source-profiles.md`
- `meta/research/codebase/2026-09-26-0283-recursive-finding-deepening.md`
- `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`
- `meta/research/codebase/2026-10-10-0303-research-concurrency-subagent-limit.md`

## Open Questions

1. **HTML parser choice.** Which pure-Rust parser passes `deny.toml`
   (licence, `multiple-versions`, `unmaintained`)? Candidates (html5ever or
   scraper, lol_html, tl) have not been dependency-checked.
2. **Where the host rule lives.** Domain redirect loop with a
   `HostResolver` port, adapter-side filtering resolver, or both; and
   whether DNS rebinding is in scope.
3. **Loopback integration tests.** The host check blocks the
   `MockHTTPServer` on `127.0.0.1`. Either keep web coverage at the domain
   level with fakes, or add a `test-loopback`-only bypass under the existing
   release compile guard.
4. **Web command output and exit mapping.** The success JSON shape
   (`{"status":"ok","metadata":{…}}`?) is unspecified, and every failure,
   including 5xx and timeout, must exit non-zero with no stdout, unlike
   academic `Unavailable`.
5. **Calendar date from `SystemTime`.** Hand-rolled std-only conversion in
   the domain, or an adapter-formatted value injected through the port.
6. **DOIs that fail `Doi::parse`.** Omit them from `metadata` (consistent
   with the usable-value rule) or emit any `10.`-prefixed value as 0348's
   literal wording allows.
7. **Outputter documentation.** Document the block once in
   `finding-outputter` and reference it, preserving 0283's no-drift rule,
   and confirm that satisfies 0348's two-outputter criterion. Separately,
   whether the three templates should show an example block.
8. **Item corrections.** 0348's Technical Notes should say the web profile
   gains the fetch grant (it has none) and that decoders live in
   `research-adapters`; `test_research_structure.py`'s "academic" naming
   needs revisiting once web grants the fetch.
