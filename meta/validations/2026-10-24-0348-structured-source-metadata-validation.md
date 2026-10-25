---
type: "plan-validation"
id: "2026-10-24-0348-structured-source-metadata-validation"
title: "Validation Report: Structured Source Metadata Implementation Plan"
date: "2026-10-25T13:27:56+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-24-0348-structured-source-metadata"
tags: ["research", "sources", "metadata", "cli", "network"]
last_updated: "2026-10-25T14:52:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Revalidation after fixes

The result is now `pass`. Four commits after the first validation fixed
the two defects and the smaller findings below, and `mise run` exited 0
again, with all 68 tasks finished.

| Finding | Resolution |
|---|---|
| Date parser panic | `is_offset` splits with `split_at_checked` and refuses an offset it cannot split; `a_date_carrying_non_ascii_where_digits_belong_is_unusable` failed first |
| Body read past the deadline | The page transport drives reqwest's async client under one `tokio::time::timeout` over the whole exchange; `a_body_that_trickles_past_the_request_budget_is_a_timeout`, over a new `Route::Trickle`, failed first |
| Untested `%40`/`%2F`/`%23`/`%3F` hosts | Added to `a_host_the_client_would_read_differently_is_refused_unsent`; all refused unsent |
| Missing plan tests | OpenAlex url/title exclusion, every fixture's `publication_date`, `Mar. 21 2007` |
| `research.md` inaccuracies | Exit table row reads "Any other `E_WEB_*`"; URL rules list no host and bad port; transport refusal noted under `E_WEB_CONNECTION_FAILED` |
| Ambiguous carrier wording | The composer and `synthesise` name the `- {source-metadata}` slot line under each source, and both forbid merging. The outputter says the slot repeats under every entry. The web profile says a successful call always yields a block |

Left as is, by decision:
- **`network` in `research-cli`:** `research-adapters` already brings it
  into every build.
- **Byte-wise order for unknown keys:** belongs to 0349.
- **Classifier gaps:** these predate this work and are low risk; they are a
  candidate for their own work item.
- **Academic `transport.rs`:** its per-read timeout only ever faces
  trusted APIs.

The report below is the original validation, kept as written.

## Validation Report: Structured Source Metadata Implementation Plan

All five phases are built as planned, and `mise run` exits 0. The result
is `partial` rather than `pass` because two confirmed defects break
promises in the plan's Desired End State:
- a crafted date panics the CLI instead of being treated as unusable;
- a trickling web page can hold `fetch web metadata` well past the 100 s
  deadline.

Both are small fixes.

### Implementation Status

✓ Phase 1: Promote host classification into a shared `network` crate - Fully implemented
✓ Phase 2: Source metadata on academic records - Fully implemented (one defect, see Potential Issues)
✓ Phase 3: Web declaration extraction - Fully implemented
✓ Phase 4: The `fetch web metadata` command - Fully implemented (one defect, see Potential Issues)
✓ Phase 5: Skills, agents and documentation - Fully implemented

Every automated and manual criterion in the plan is ticked. The one
exception is Phase 3's manual item, which the plan marks "None beyond code
review".

### Automated Verification Results

✓ Full default task passes: `mise run`. All 66 tasks finished, it exited
  0, and it left the working copy unchanged (no formatter drift). The
  `test:integration:skill-invocation` lane alone ran for about 58 minutes.
✓ The default task includes the plan's per-phase commands: `check`,
  `public-api:check`, `lint:crate-dependencies:check`, `pup:check`,
  `notices:check`, `deny:check`, `docs:check`, and every Rust, Python and JS
  test lane.

### Code Review Findings

#### Matches Plan:

- **Phase 1.** `cli/network` owns `Host`, `HostReach`, `classify_address`
  and `Authority::parse`, with port rules exactly as planned:
  - `[::1]8080`, `+80`, `0` and `99999` are refused;
  - a bare `:` takes the scheme default.

  Design's rejection wording moved to `access_policy::reach_word` and
  `access_policy::host_error_message`. The vector corpus is split by owner,
  and the JS test merges both halves. The pup rules, probe pairs and
  public-API pins are all registered.
- **Phase 2.**
  - `SourceMetadata` writes keys only through typed recorders, orders them
    by `RECOMMENDED_KEYS`, and keeps the first value recorded.
  - `PartialDate` covers the ISO, slash and month-named forms, and clamps
    at the epoch.
  - `Doi::parse` cuts at `?`/`#` before percent-decoding, and bounds the
    DOI to 256 printable-ASCII characters.
  - `accessed` is computed once per call (`fetch.rs:280`, `:467`).
  - Both goldens pin key order.
- **Phase 3.**
  - The precedence table holds the whole rule, and extraction does no
    field-name string comparisons.
  - `html5gum` is pinned at `=0.8.4`, with `naively_switch_states(true)`.
  - Every acceptance-criterion test exists in `web_extract.rs`, plus the
    table walks.
- **Phase 4.** An adversarial SSRF review found no path to an address that
  is not `Public`:
  - **Pinning:** `PinnedResolver` answers only the admitted name, and
    reqwest has no lookup of its own.
  - **Proxies and redirects:** `.no_proxy()` is set, reqwest follows no
    redirect, and every hop is re-admitted.
  - **Hosts reqwest reads differently:** fullwidth digits, `%40` and `%2f`
    hosts, and IP-folding IDNs all fail closed through `ascii_host`.
  - **Mapped addresses:** IPv4-mapped, 6to4, Teredo and NAT64 `64:ff9b::/96`
    addresses are unwrapped before they are classified.
  - **Release builds:** the `test-loopback` seam is `cfg`-gated, and a
    `compile_error!` stops it being enabled in a release build.
  - **Error text:** no message carries a URL, host or address.
- **Phase 5.**
  - The outputter's key table and `research.md`'s key table each list all
    19 keys in `RECOMMENDED_KEYS` order, and a test reads that order from
    the Rust source.
  - The web profile grants `Bash(accelerator research fetch *)`.
  - The composer and `synthesise` carry blocks byte-for-byte.
  - All four CHANGELOG entries are present, and every `research.md` section
    the plan lists is updated.

#### Deviations from Plan:

- `PageUrl`'s `Display` does not always round-trip. `network::Host` stores
  the percent-decoded host, so `http://a%40127.0.0.1/` prints as a URL that
  reqwest reads as userinfo `a` and host `127.0.0.1`. It fails closed,
  because `url::Host::parse` refuses `@` and `ascii_host` returns `None`.
  No test pins this, though
  (`cli/research-adapters/tests/page_transport.rs:238` covers only `a%2Fb`
  and `exa%20mple.org`).
- A bracketed name such as `http://[evil.com]/` is accepted as the name
  `evil.com`. This is harmless, since the name is resolved and admitted
  like any other.
- `research-cli/Cargo.toml:29` depends on `network` unconditionally. The
  only consumer is the `loopback.rs` check, which builds only under
  `test-loopback` or `cfg(test)`.
- The composer's "never merge, reorder or complete a block"
  (`agents/composer.md:32-33`) has no counterpart in `synthesise`
  (`skills/research/research-topic/SKILL.md:650-653`).
- Some tests the plan lists are absent or narrower:
  - the OpenAlex "metadata never carries url/title" assertion (only arXiv
    has it, at `cli/research/tests/arxiv.rs:243`);
  - `publication_date` assertions for `work-no-doi.json` and
    `work-retracted.json`;
  - the no-comma `Mar. 21 2007` case.

#### Potential Issues:

- ❌ **Date parser panics on a non-ASCII offset.**
  `cli/research/src/sources/metadata/date.rs:229` slices
  `&magnitude[..2]` by bytes. `PartialDate::parse("2023-05-12T10:00+1é1")`
  reaches `is_offset`, where `magnitude` is the 4-byte `"1é1"`, and byte
  index 2 falls inside `é`. It panics (confirmed with a standalone repro of
  the slice).
  - **Reach:** the input is untrusted: an OpenAlex `publication_date`, an
    arXiv `<published>`/`<updated>`, or any web page's date declaration.
  - **Effect:** a panic exits 101 with the page's text in the panic message.
    That breaks "every other failure exits 1 with one `E_WEB_*` line" and
    "no URL in the message". For the academic families, it loses a whole
    search result.
  - **Fix:** guard with `magnitude.is_char_boundary(2)` or check for ASCII
    first. Add `+1é1` to the unusable-dates table.
- ⏱️ **The body read is not bounded by the call deadline.** reqwest
  0.12.28's blocking `Response::read`
  (`reqwest-0.12.28/src/blocking/response.rs:439-440`) applies the client
  timeout to each `read()`, not to the whole body.
  - **Effect:** a server that sends one byte every 29 s keeps
    `read_to_end` (`cli/research-adapters/src/page_transport.rs:86-88`)
    running until 2 MiB arrive. That breaks the 100 s promise for every
    `fetch`.
  - **Coverage:** the existing `a_body_that_stalls_past_the_request_budget_is_a_timeout`
    covers only a total stall.
  - **Inherited pattern:** the academic `transport.rs:107-108` reads the
    same way, though against trusted APIs.
  - **Fix:** read in a loop that checks an `Instant` deadline, or set a
    total timeout on the request. Add a trickle test.
- **Classifier gaps fall through to `Public`.** These gaps predate this
  work: `64:ff9b:1::/48` (local-use NAT64), `::ffff:0:a.b.c.d`
  (IPv4-translated) and `fec0::/10` (site-local). Each needs a translator
  or legacy routing to be reachable, so the risk is low.
- **`research.md` inaccuracies:**
  - The exit-code table's wildcard `E_WEB_*` exit-1 row (`:140`) also
    covers `E_WEB_URL_MALFORMED`, which exits 2 (`:128`).
  - Fetch rules (`:207-210`) omit the "no host" and "port not 1–65535"
    rules.
  - The docs don't say that a transport `Refused` surfaces as
    `E_WEB_CONNECTION_FAILED`.
- **Ambiguous agent wording.** The outputter defines the slot as the whole
  line `  - {source-metadata}`, but the composer (`:33-34`) and
  `synthesise` (`:652-653`) say only `{source-metadata}`. Read literally,
  "replace the slot" could double the bullet prefix, and "delete the slot"
  could leave a bare `  - `. Nothing says the slot repeats under every
  Sources entry. Nothing tells `synthesise` what to do when one source has
  different blocks in two findings.
- **Key-order claim not implemented.** The docs say non-recommended keys
  sort "byte-wise", but `SourceMetadata` would keep them in insertion order
  (`metadata.rs:87-92`). No producer can emit such a key yet, and 0349
  owns that rule.

### Manual Testing Required:

1. After the two fixes:
   - [ ] `accelerator research fetch web metadata` against a page that
     declares `citation_publication_date` as `2023-05-12T10:00+1é1` exits 0
     with only `accessed`. It must not panic.
   - [ ] Against a local server that trickles its body one byte every
     few seconds, the call ends with `E_WEB_TIMED_OUT` within the
     configured call budget.
2. Unchanged:
   - [ ] Re-run Phase 4's three live checks: PLOS ONE, `169.254.169.254`
     and PeerJ's 403.

### Recommendations:

- Fix the `is_offset` panic test-first, and add a property-style test that
  `PartialDate::parse` never panics on arbitrary UTF-8.
- Bound the page body read by a total deadline, with a trickle test over
  `MockHTTPServer`. Decide whether the academic transport gets the same
  treatment.
- Add end-to-end fail-closed tests for `%40` and `%2f` hosts.
- Correct the `research.md` exit-code table and the URL rules, and change
  the composer and `synthesise` to say "the slot line".
- Then re-run validation. The plan's `status` stays `in-progress` until it
  passes.
