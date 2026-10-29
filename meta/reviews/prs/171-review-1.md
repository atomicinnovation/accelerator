---
type: "pr-review"
id: "171-review-1"
title: "[0348] Record structured metadata for every research source"
date: "2026-10-25T16:54:00+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:171"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "security", "correctness", "test-coverage", "code-quality", "compatibility", "documentation", "standards"]
review_number: 1
pr_number: 171
tags: []
last_updated: "2026-10-25T16:54:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #171 - [0348] Record structured metadata for every research source

**Verdict:** COMMENT

The PR is solid across all eight lenses. The SSRF boundary admits every hop, pins connections to the admitted addresses, re-reads the host with reqwest's own parser, and caps redirects, body size and the whole exchange's time; no lens found an exploitable bypass. The shared `network` crate is a pure core that both `design` and `research` use, and the `cargo-pup` and `cargo-deny` rules enforce its boundaries. Nothing is blocking. Two findings are major: transport-guard refusals are reported as connection failures, and the docs-site `zod` override forces a major-version jump. The rest are small duplications, untested branches, and edge cases where valid page metadata is silently dropped.

### Cross-Cutting Themes

- **Duplicated sync-over-async timeout plumbing** (flagged by: code-quality, architecture). `HttpTransport`, `HttpPageTransport` and `bounded_lookup` each bridge blocking callers to async code with their own runtime and `timeout`. The PR's headline whole-exchange deadline rests on all three copies staying in step.
- **`FetchRequest` re-split into a CLI-only `ScholarlyRequest`** (flagged by: architecture, code-quality). The difference between scholarly requests and web requests is a domain concept, but it only shows up in the composition root. A new scholarly source has to be added to two parallel enums.
- **Error categories that hide the real cause** (flagged by: code-quality, correctness, compatibility). `Refused`, a local client or runtime build failure, a non-UTF-8 `Location` on a 200, and a missing mandatory proxy all report as `E_WEB_CONNECTION_FAILED` or `E_WEB_HOST_UNRESOLVED`. An operator cannot tell a guard refusal or a local fault from an outage.
- **`PageUrlError` rewrites `network`'s host-error text** (flagged by: code-quality, standards). Two variants copy `network`'s text and two use different words for the same condition, rather than delegating to `HostError`'s `Display`.
- **The documented byte-wise order for unknown keys is not implemented** (flagged by: documentation, and the PR's own notes). `finding-outputter` and `research.md` state it as current behaviour.
- **The new IPv6 reserved ranges are under-tested** (flagged by: test-coverage, security). The ranges have no vectors just outside their edges, and the denylist approach leaves `5f00::/16` and `2001:2::/48` classified as public.

### Tradeoff Analysis

- **Security vs Compatibility (no proxy)**: `.no_proxy()` stops an environment proxy undoing the address pinning, but `fetch web metadata` then fails silently behind any mandatory egress proxy. Keep the SSRF choice and document the limitation in `research.md`.
- **Shared kernel vs independent policy**: one classifier stops `design` and `research` drifting apart, but any range research reclassifies also changes what `design` refuses, as this PR already shows. That is acceptable; route any future context-specific exceptions through `access_policy` or `RejectedReach`, not the classifier.

### Strengths

- ✅ DNS rebinding is closed: `PinnedResolver` answers only the admitted name with the admitted addresses and fails closed on any other name. IP-literal URLs must exactly equal the admitted address.
- ✅ `host_form` re-reads the host with the `url` crate (reqwest's own parser) and refuses fullwidth hosts, IP-folding IDN hosts and percent-encoded delimiters, so the admitted host and the connected host cannot differ.
- ✅ Every redirect hop is re-parsed and re-admitted. There are at most 5 redirects, the body is capped at 2 MiB, one `tokio::time::timeout` covers the whole exchange, proxies are off (a test proves it), and no decompression features are enabled.
- ✅ No `E_WEB_*` line contains the URL, host or address, so the CLI cannot be used to probe internal names.
- ✅ The `network` crate is pure and no-I/O, and each context keeps its own rejection wording. It completes every point of the tasks/README library-crate checklist, and its `cargo-pup` and `cargo-deny` rules move with the code.
- ✅ `fetch_page_metadata` depends only on the `HostResolver`, `PageTransport`, `PageDecoder` and `Clock` ports, so admission, redirects and deadline clipping are unit-tested without sockets.
- ✅ `SourceMetadata` accepts values only through typed recorders. The `WEB_KEYS` precedence table is data, and its tests iterate the production tables.
- ✅ Both validation defects have regression tests (the `Trickle` route and the non-ASCII offset). Negative SSRF tests check that `hits() == 0` and that the resolver was never asked.
- ✅ The `metadata` field and the Sources block are additive and optional, and the hand-written `Serialize` keeps the documented key order.
- ✅ The classifier is recorded consistently as a breaking change in CHANGELOG and `design.md`, with `--allow-internal` as the recovery. The JS mirror is tested against both halves of the corpus.
- ✅ `research.md`'s precedence table, JSON-LD type list and `E_WEB_*` table match the Rust source almost exactly.

### General Findings

- 🔵 **Architecture**: The shared classifier couples `design`'s and research's network policies. Keep `HostReach` neutral and handle any context-specific exception in the policy layers.
- 🔵 **Code Quality**: Each metadata producer repeats "parse, then record if `Some`" for every key. This will grow per key once 0349 lands; consider recorders that take `Option<T>`.
- 🔵 **Compatibility**: The docs-site lockfile refresh also bumps `@astrojs/compiler-binding`, raises vite to `^8.3`, and raises astro's `engines.node` floor to `>=22.22.0`. The PR description doesn't mention these.
- 🔵 **Correctness**: Every status in 300–399 is treated as a redirect, so a 300 or 304 without a `Location` surfaces as `E_WEB_REDIRECT_WITHOUT_LOCATION` rather than as a status error.
- 🔵 **Security**: `loopback::STAND_IN` is a real public IP (`93.184.216.34`). Nothing ships with it today, but a note on why it must stay public would stop a later "fix" to a documentation address, which the classifier now treats as reserved.
- 🔵 **Standards**: `.accelerator/state/integrations/linear/last-sync.json` changes in this PR, but nothing in the description or commits relates it to 0348.
- 🔵 **Test Coverage**: Extraction is tested only against handwritten HTML, and the live PLOS ONE and PeerJ checks were not re-run on the final commit. A recorded real `<head>` fixture would cover real pages without needing network access.

### Additional Findings

- 🔵 `docs-site/src/content/docs/research.md:740` — The Sources "Order" rule documents unimplemented byte-wise ordering (documentation)
- 🔵 `cli/research/src/sources/request.rs:680` — `OpenAlexRequest::parse` and `ArxivRequest::parse` are structurally identical (code-quality)
- 🔵 `cli/research/src/sources/web/page_url.rs:22` — `PageUrlError` restates `HostError`'s wording instead of delegating to it (code-quality, standards)
- 🔵 `cli/research-adapters/src/host_resolver.rs:69` — Search-list expansion lets single-label names reach corporate-internal hosts (security)
- 🔵 `cli/network/src/host_reach.rs:173` — The IPv6 denylist misses `5f00::/16`, `2001:2::/48` and future allocations (security)
- 🔵 `cli/research/src/sources/web/extract.rs:20` — Only the first article-typed JSON-LD node is read, and `WebPage` counts as one (correctness)
- 🔵 `cli/research/src/sources/metadata/date.rs:28` — A space-separated ISO datetime is sent to the month-name parser and refused (correctness)
- 🔵 `cli/research-cli/tests/fetch_web.rs:152` — The fullwidth-address test doesn't assert which `E_WEB_*` line it gets (test-coverage)
- 🔵 `cli/research/src/sources/metadata.rs:7` — Source identifiers live in the `request` module, which is becoming a hub (architecture)
- 🔵 `cli/research-adapters/src/transport.rs:92` — The academic fetch timeout semantics changed without a CHANGELOG entry (compatibility)
- 🔵 `cli/research-cli/Cargo.toml:8` — The crate description exceeds 80 columns (standards)
- 🔵 `cli/research/src/sources/arxiv.rs:40` — An arXiv entry that declares several DOIs records none (correctness)
- 🔵 `cli/research-adapters/src/page_transport.rs:92` — A non-UTF-8 `Location` fails even a 200, and that branch is untested (correctness, test-coverage)
- 🔵 `cli/research/src/sources/web/fetch.rs:249` — Any port on a public address is admitted, including on redirect hops (security)
- 🔵 `cli/research-adapters/src/page_transport.rs:74` — Web metadata ignores proxies, unlike the academic fetches; document it (compatibility)
- 🔵 `cli/research-adapters/src/web_html.rs:26` — Page bytes are always decoded as UTF-8 regardless of the declared charset (compatibility)
- 🔵 `cli/research-cli/src/main.rs:433` — The production web adapters are constructed in both `cfg` variants (code-quality)
- 🔵 `cli/design/src/access_policy.rs:28` — `reach_word` and `host_error_message` are `pub` with no external consumer (architecture)
- 🔵 `cli/research-adapters/tests/host_resolver.rs:40` — The real-DNS tests depend on the host's resolver configuration (test-coverage)
- 🔵 `docs-site/src/content/docs/research.md:226` — "Each lookup" can be confused with the `lookup` verb (documentation)
- 🔵 `docs-site/src/content/docs/research.md:237` — The `E_WEB_HOST_REJECTED` template does not render as emitted for "unspecified" (documentation)
- 🔵 `skills/research/profiles/web-profile/SKILL.md:38` — A ragged mid-sentence line break (standards)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/research/src/sources/web/fetch.rs:284-286` — Transport guard refusals are reported as connection failures
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`PageResponse::Refused` means the transport's own guards refused to send the request: a host that reqwest would read differently from the admitted one, a non-UTF-8 `Location`, or an unparseable URL. `next_hop` merges it with `ConnectionFailed`, so the user sees `E_WEB_CONNECTION_FAILED: the page could not be reached` even though nothing was sent.

**Impact**: When a live page fails, the one `E_WEB_*` line the CLI emits doesn't let an operator tell a deliberate SSRF-guard refusal from a network outage. The transport keeps the distinction, but this line drops it.

**Suggestion**: Add a `WebFailure::Refused` variant (e.g. `E_WEB_REFUSED: the page's address could not be pinned safely`) and map `PageResponse::Refused` to it. The message still names no URL, host or address.

---

### `docs-site/package.json:24` — zod override forces a major version onto a plugin declaring zod ^3
**Severity**: major | **Confidence**: medium | **Lens**: compatibility

🟡 **Compatibility**

The override `"zod": "^4.6.6"` under `astro-rehype-relative-markdown-links` replaces the plugin's declared `"zod": "^3.23.8"`, and the lock now resolves zod 4.6.7. The plugin's `dist/index.js` calls `z2.record(CollectionConfigSchema)` with one argument, a form zod 4 dropped. The build passes today only because `astro.config.mjs` never passes `collections`, so the record schema only ever sees its `.default({})`.

**Impact**: Configuring `collections`, or a plugin release that reaches another zod-3-only API, would break the docs build at runtime. Nothing pins this, because the override sits outside the dependency's declared range.

**Suggestion**: If the advisory has a patched 3.x release, override to that instead of crossing the major. Otherwise, record why the override is safe (only the default-options path runs), or upgrade or replace the plugin once it supports zod 4.

---

### `cli/research/src/sources/web/fetch.rs:214-217` — Request budget re-clipped after the lookup is untested and obscured (Test Coverage + Code Quality)
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage, code-quality

🔵 **Test Coverage**

The bounds are recomputed after the DNS lookup, so the lookup's time comes out of the request timeout. No test checks this: `ScriptedResolver` never advances the clock, so removing the second `remaining_bounds` call, or reusing the first `bounds`, passes the whole suite. A lookup can take up to `LOOKUP_CAP` (5 s), so losing the recompute would let the request outrun the 100 s budget. Let `ScriptedResolver` take clock time, as `RecordingTransport::taking` does, then test (a) that 20 s left minus a 4 s lookup gives a 16 s request timeout, and (b) that a lookup using up all remaining time returns `TimedOut` with `hits() == 0`.

🔵 **Code Quality**

`bounds` is computed twice and shadowed, and only `.lookup` from the first call and `.request` from the second are used. Splitting it into `lookup_bound(...)` and `request_bound(...)`, or naming the two `before_lookup` and `before_request`, would show why the clock is read again.

---

### `cli/research-adapters/src/page_transport.rs:59-82` — The "block on a future under a whole-exchange timeout" pattern is repeated three times (Code Quality + Architecture)
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, architecture

🔵 **Code Quality**

`HttpPageTransport::get`, `HttpTransport::send` and `host_resolver::bounded_lookup` each build a current-thread runtime and run `block_on(timeout(budget, fut))`. One keeps a long-lived runtime and two build one per call. The capped body readers also differ subtly (`<` with truncate versus `<=` with clear).

🔵 **Architecture**

The whole-exchange deadline is the PR's headline guarantee, and it rests on three copies staying correct. Extract one crate-level helper (e.g. `within(runtime, budget, future, timed_out)`) plus a shared `read_capped`. Keep the choice between a held runtime and a per-call one explicit at each call site, with the reason for it.

---

### `cli/research-cli/src/main.rs:160-166` — Domain FetchRequest mixes two workflows, so the CLI re-splits it into a mirror enum (Architecture + Code Quality)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture**

`FetchRequest` gains a `Web` variant, but web requests share none of the scholarly workflow (tickets, pacing, credentials, endpoints). `main.rs` immediately re-splits the request into a CLI-local `ScholarlyRequest` that copies the domain's `OpenAlex` and `Arxiv` variants.

🔵 **Code Quality**

Each new scholarly source needs a variant in both enums plus a new arm in this mapping. Model the split in the domain as `FetchRequest::{Scholarly(ScholarlyRequest), Web(WebRequest)}`, and the mirror enum and the conversion both disappear.

---

### `cli/network/src/host_reach.rs:173-180` — New reserved IPv6 ranges have no case just outside their edge
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The corpus checks addresses inside `3fff::/20`, `100::/64` and `64:ff9b:1::/48`, but none just outside them. Removing the `segments[1] & 0xf000` mask, the `segments[1..4]` all-zero check, or the `segments[2] == 0x0001` check fails no test. `198.51.100.0/24` also has no public neighbour vector.

**Impact**: This classifier now gates both `design`'s `--allow-internal` and research's SSRF admission. A range widened by mistake would refuse real public hosts unnoticed, and the JS mirror could drift the same way.

**Suggestion**: Add public vectors such as `[3fff:1000::1]`, `[100:0:0:1::1]`, a `64:ff9b:2::` neighbour and `198.51.101.1`. The shared corpus covers both implementations.

---

### `cli/research/src/sources/request.rs:967-980` — DOI percent-decoding failure paths are untested
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`percent_decode` returns `None` for a truncated escape (`%2`), a non-hex escape (`%zz`) and an invalid UTF-8 sequence (`%C3`). None of these is tested, and neither is lowercase hex or an encoded non-ASCII character (`%C3%A9`, which should then fail the printable-ASCII rule).

**Impact**: The canonical DOI is the key 0306 will use to recognise one source cited twice. A decoder that silently let a bad escape through would produce DOIs that never match.

**Suggestion**: Add a table test: `.../a%2`, `.../a%zz`, `.../a%C3` and `.../caf%C3%A9` should each give `None`, and `.../a%2fb` should give `10.1000/a/b`.

---

### `skills/research/outputters/finding-outputter/SKILL.md:98-99` — Documented byte-wise ordering for unknown keys is not implemented and cannot occur
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

"The CLI emits recommended keys in table order, then any other key in byte-wise order" is stated as current fact. `SourceMetadata` only records the recommended keys through typed recorders, and `rank` gives every unknown key the same rank, which keeps them in insertion order. The PR description admits this. `docs-site/src/content/docs/research.md:740-741` has the same wording.

**Suggestion**: State only what the CLI emits today, or mark the byte-wise rule as reserved for 0349.

---

### `cli/research-cli/src/cli.rs:160-164` — Help-text test passes on incidental mentions of each word
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The test checks only that `FETCH_HELP` contains each family code and each verb code somewhere. `web`, `metadata` and `lookup` each appear in several unrelated sentences, so deleting "web takes metadata" would not fail it.

**Suggestion**: Assert the pairing itself, e.g. build the `family → verbs` phrase from `Family::verbs()` and check that it is present.

---

### `cli/research-adapters/src/page_transport.rs:56-64` — Local client and runtime build failures are reported as connection failures
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

When the reqwest `Client` or the tokio runtime can't be built, `HttpPageTransport::get` returns `PageResponse::ConnectionFailed`. Nothing has touched the network at that point. `HttpTransport::new` treats the same failure as `TransportUnavailable`.

**Impact**: A broken TLS provider or runtime looks like "the page could not be reached", which sends the person diagnosing it to look at the remote site.

**Suggestion**: Give local failures their own outcome, or reuse the `Refused` category proposed for `fetch.rs:284`.

## Per-Lens Results

### Architecture

**Summary**: The structure is sound. The pure `network` crate is shared, each context keeps its own policy wording, and `cargo-pup` and `cargo-deny` enforce the new edges. The web fetch follows the hexagonal pattern already used elsewhere. The remaining concerns are local cohesion issues.

**Strengths**: The `network` crate is pure and no-I/O, and its pup rule is tested. The web fetch is a functional core behind three narrow ports. Deadlines run end to end. The boundary rules move with the code. `host_form` makes pinning fail closed when the two URL parsers disagree. The broader internal ranges are recorded wherever `design` users will see them.

**Comments**:
- `cli/research-cli/src/main.rs:160-166` (minor, high) — The domain `FetchRequest` mixes two workflows, so the CLI re-splits it.
- `cli/research/src/sources/metadata.rs:7-9` (minor, medium) — Source identifiers live in `request`, creating a `request` ↔ `web` module cycle. Extract a `sources::identifier` module.
- `cli/research-adapters/src/page_transport.rs:86-108` (suggestion, medium) — There are three different sync-to-async patterns in one adapter crate.
- `cli/design/src/access_policy.rs:28-44` (suggestion, high) — `reach_word` and `host_error_message` widen `design`'s public API for no reason; make them `pub(crate)`.

**General Findings**: The shared classifier couples `design`'s and research's policies. Keep exceptions in the policy layers.

### Security

**Summary**: The SSRF boundary is well built. Each hop is admitted only if every resolved address is public, the connection is pinned to those addresses, proxies and reqwest's own redirects are off, and the host is re-read with reqwest's parser. Percent-encoded, folded, bracketed, trailing-dot and IDNA forms were all traced, and no exploitable bypass was found. The findings are defence in depth.

**Strengths**: DNS rebinding is closed. Host-form disagreement is guarded at both ends. Every hop is re-admitted, and redirects, body size and time are capped. Proxies are disabled and a test proves it. There are no decompression features, and rustls verifies certificates. No error line leaks the URL, host or address. Untrusted content reaches the agent only as typed values. The JS mirror is updated in step. The test-loopback seam is gated three ways. The PreToolUse guard covers quoted URLs.

**Comments**:
- `cli/research-adapters/src/host_resolver.rs:69-74` (minor, medium) — System search-list expansion lets single-label names reach corporate-internal hosts. Set `ndots = 0` and resolve the name fully qualified, or refuse dotless hosts.
- `cli/research/src/sources/web/fetch.rs:249-269` (suggestion, medium) — Any port on a public address is admitted, including on redirect hops. Allowlist 80, 443, 8080 and 8443.
- `cli/network/src/host_reach.rs:173` (minor, medium) — The IPv6 denylist misses non-global special-purpose ranges such as `5f00::/16` and `2001:2::/48`. Treat only `2000::/3` as public by default.

**General Findings**: The test stand-in address is a real public IP. Add a note explaining why it must stay public.

### Correctness

**Summary**: The core logic holds up. The redirect loop allows exactly 5 hops, every hop is re-admitted, pinning fails closed, the deadline arithmetic is correct, the IPv6 masks are correct, and dot-segment removal is correct. The issues are narrow extraction gaps where valid declarations get dropped.

**Strengths**: The loop bounds `0..=MAX_REDIRECTS` are correct. All-public admission and the pin work together to close rebinding. `step_bounds` clips each step to the time left. `without_dot_segments` handles its edge cases. The IPv4-translated check comes before `to_ipv4()`. The DOI is cut at `?` and `#` before decoding. `is_offset` uses `split_at_checked`. `SourceMetadata::record` keeps entries sorted with `partition_point`.

**Comments**:
- `cli/research/src/sources/web/extract.rs:20-23` (minor, medium) — Only the first article-typed JSON-LD node is ever read, and `WebPage` counts as one. Nested `mainEntity` nodes and IRI-form types are missed.
- `cli/research/src/sources/metadata/date.rs:28-31` (minor, medium) — A space-separated ISO datetime is sent to the month-name parser and refused.
- `cli/research/src/sources/arxiv.rs:40` (minor, low) — An arXiv entry that declares several DOIs records none. Split on whitespace and keep the first DOI that parses.
- `cli/research-adapters/src/page_transport.rs:92` (minor, low) — A non-UTF-8 `Location` fails even a non-redirect response.

**General Findings**: A 3xx without a `Location` is reported as a redirect failure rather than a status error.

### Test Coverage

**Summary**: Coverage is strong and proportional to the risk. It is layered from scripted fakes through mock servers to a binary-level loopback seam, and both validation defects have regression tests. The gaps are narrow.

**Strengths**: There are regression tests for the trickling body and the non-ASCII offset. The tests are well layered. Table-driven tests iterate the production tables. SSRF negatives are checked by side effect. The proxy test runs alone in its own binary. The Rust and JS classifiers share one corpus.

**Comments**:
- `cli/research/src/sources/web/fetch.rs:214-217` (minor, high) — No test exercises re-clipping the request budget after a slow lookup.
- `cli/network/src/host_reach.rs:173-180` (minor, high) — The new reserved IPv6 ranges have no case just outside their edge.
- `cli/research/src/sources/request.rs:967-980` (minor, high) — The DOI percent-decoding failure paths are untested.
- `cli/research-cli/tests/fetch_web.rs:152-162` (minor, medium) — The fullwidth-address CLI test doesn't assert which failure it got.
- `cli/research-cli/src/cli.rs:160-164` (minor, high) — The help-text test passes on incidental mentions of each word.
- `cli/research-adapters/src/page_transport.rs:92-101` (suggestion, high) — The invalid-UTF-8 `Location` and body-error branches are untested. Use `Route::Truncated`.
- `cli/research-adapters/tests/host_resolver.rs:40-72` (suggestion, medium) — The real-DNS tests depend on the host's resolver configuration.

**General Findings**: Extraction is tested only on synthetic fixtures, and the live checks were not re-run on the final commit.

### Code Quality

**Summary**: The code is well structured, with ports-and-adapters seams, a data-driven precedence table, and a pure `network` crate. The main concerns are guard refusals collapsed into connection failures, timeout plumbing repeated three times, and parallel parse paths and request enums.

**Strengths**: The wording lives with each consumer. The fetch loop depends only on narrow traits. Typed recorders make a wrongly formed value unrepresentable. Precedence is defined as data. The fragile `is_timeout` downcast is gone. `Doi::parse` absorbs the ad-hoc DOI stripping callers used to do. No comment references a work item, plan or ADR.

**Comments**:
- `cli/research/src/sources/web/fetch.rs:284-286` (major, high) — Transport guard refusals are reported as connection failures.
- `cli/research-adapters/src/page_transport.rs:56-64` (minor, high) — Local client and runtime build failures are reported as connection failures.
- `cli/research-adapters/src/page_transport.rs:59-82` (minor, high) — The "block on a future under a whole-exchange timeout" pattern is repeated three times.
- `cli/research/src/sources/request.rs:680-737` (minor, high) — `OpenAlexRequest::parse` and `ArxivRequest::parse` are structurally identical.
- `cli/research-cli/src/main.rs:160-166` (minor, medium) — `FetchRequest` is re-mapped variant by variant into a parallel enum.
- `cli/research/src/sources/web/page_url.rs:22-42` (suggestion, medium) — `PageUrlError` restates `network`'s host and port wording.
- `cli/research/src/sources/web/fetch.rs:214-217` (suggestion, medium) — The step bounds are computed twice, and each call uses only half the result.
- `cli/research-cli/src/main.rs:433-452` (suggestion, medium) — The production web adapters are constructed in both `cfg` variants.

**General Findings**: Each metadata producer repeats "if the parse succeeds, record it" for every key.

### Compatibility

**Summary**: The contracts consumers see are additive and well managed. The main risk is outside the Rust code: the docs-site zod override crosses a major version. The academic timeout semantics change without a note, and the web fetch ignores proxies and charsets, which makes it depend on the environment.

**Strengths**: The `metadata` field is additive, and the help text says fields are only ever added. The custom `Serialize` keeps key order. The Sources block is optional. The classifier change is recorded under Unreleased. The JS mirror moves in step. Only internal APIs move. The exact pins avoid duplicate crates.

**Comments**:
- `docs-site/package.json:24` (major, medium) — The zod override forces a major version onto a plugin that declares zod ^3.
- `cli/research-adapters/src/transport.rs:92-96` (minor, medium) — The academic fetch timeout semantics changed without a CHANGELOG entry.
- `cli/research-adapters/src/page_transport.rs:74-77` (suggestion, medium) — The web metadata fetch ignores proxies, unlike the academic fetches.
- `cli/research-adapters/src/web_html.rs:26` (suggestion, low) — Page bytes are always decoded as UTF-8, whatever the declared charset.

**General Findings**: The docs-site lockfile churn goes beyond the stated zod fix and raises the Node floor to 22.22.

### Documentation

**Summary**: The docs are thorough and match the code closely: the precedence table, the type list, the fetch rules, the exit codes and nearly all the `E_WEB_*` messages. The problems are small: an unimplemented ordering rule, an ambiguous "lookup", and one error-message template.

**Strengths**: The precedence table and type list match the source. The error table can be audited. The breaking change is recorded consistently. The CHANGELOG entries tell users how to act on them. The byte-for-byte contract is the same in every place it is consumed. The doc warns that a JSON parser may reorder keys.

**Comments**:
- `skills/research/outputters/finding-outputter/SKILL.md:98-99` (minor, high) — The documented byte-wise ordering for unknown keys is not implemented.
- `docs-site/src/content/docs/research.md:740-741` (minor, high) — The Sources metadata Order rule documents unimplemented byte-wise ordering.
- `docs-site/src/content/docs/research.md:226-227` (suggestion, medium) — "Each lookup" can be confused with the `lookup` verb.
- `docs-site/src/content/docs/research.md:237` (suggestion, high) — The `E_WEB_HOST_REJECTED` template does not render as emitted.

**General Findings**: None.

### Standards

**Summary**: The PR follows documented conventions closely. The `network` crate completes the library-crate checklist, and the error codes follow the existing scheme. The findings are minor wording and formatting issues.

**Strengths**: The library-crate checklist is complete. The domain pup rule widenings are pinned by tests. The `html5gum` wrapper rule follows existing precedent. The `E_WEB_*` naming matches the existing codes. The tool grant matches the sibling profiles. The CHANGELOG and JS mirror were updated. No comment references a work item.

**Comments**:
- `cli/research/src/sources/web/page_url.rs:30-39` (minor, medium) — `PageUrlError` writes its own text for host errors instead of using `HostError`'s `Display`.
- `cli/research-cli/Cargo.toml:8` (minor, medium) — The crate description exceeds the 80-column limit.
- `skills/research/profiles/web-profile/SKILL.md:38-40` (suggestion, high) — A ragged line break is left mid-sentence.

**General Findings**: An unrelated Linear sync state file is included in the PR.

---
*Review generated by /accelerator:review-pr*
