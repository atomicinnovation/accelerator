---
type: "pr-description"
id: "171"
title: "[0348] Record structured metadata for every research source"
date: "2026-10-25T16:36:55+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0348"
parent: "work-item:0348"
relates_to: ["work-item:0281", "work-item:0305", "work-item:0306", "work-item:0347", "work-item:0349"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/171"
pr_number: 171
tags: ["research", "sources", "metadata", "cli", "network"]
revision: "f93687634052a701d73435b1db1ce58671eca6db"
repository: "accelerator"
last_updated: "2026-10-25T16:36:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0348] Record structured metadata for every research source

## Summary

A Sources entry in a level note, finding or dossier can now carry a
nested `key: value` metadata block — DOI, arXiv and OpenAlex ids,
`published`, `updated` and `accessed` — so a cited source can be matched
and checked without re-fetching it. The CLI is the only producer: OpenAlex
and arXiv records gain a `metadata` map, and a new
`accelerator research fetch web metadata '<url>'` reads what a public page
declares about itself, behind SSRF-safe host admission. 0306 (cite a
source once) needs the `doi` key to recognise a publisher URL and its
`doi.org` URL as one source.

## Changes

### Source metadata model (`cli/research`)

- `SourceMetadata` records keys only through typed recorders, emits them
  in `RECOMMENDED_KEYS` order, and keeps the first value recorded for a
  key.
- `PartialDate` parses ISO 8601, year-first slash (`2010/5/12`) and
  English month-named dates at the precision declared, and refuses
  anything else (`12/05/2010`, weekday- or time-bearing month-named
  dates) rather than guessing.
- `Doi::parse` reads a DOI from any label or resolver form, cuts at
  `?`/`#` before percent-decoding, and bounds it to 256 printable-ASCII
  characters.
- OpenAlex `select` now requests `publication_date`; OpenAlex and arXiv
  records emit `metadata` with `accessed` computed once per call. The
  field is additive, so JSON consumers that ignore unknown fields are
  unaffected.

### `fetch web metadata` command

- `web::extract` reads `doi`, `published` and `updated` from Highwire
  `citation_*`, schema.org JSON-LD, Dublin Core and OpenGraph, driven by
  one precedence table: first usable declaration wins, scheme by scheme,
  field order then document order.
- `research-adapters::web_html` tokenises with `html5gum`, exact-pinned at
  `=0.8.4` with `naively_switch_states(true)` so JSON-LD containing `<`
  stays text; `deny.toml` permits only `research-adapters` to wrap it.
- `web::fetch` admits only `http(s)` URLs whose host resolves solely to
  `Public` addresses, then pins the connection to those addresses via
  `PinnedResolver`. It follows at most 5 redirects, re-admitting each hop,
  reads at most 2 MiB, never uses a proxy, and fits inside the 100 s
  budget every `fetch` shares.
- `host_form::ascii_host` refuses any URL whose host reqwest would read
  differently from the one admitted: fullwidth digits, `%40`/`%2F`/`%23`/
  `%3F` hosts, IP-folding IDNs.
- Failures exit 1 with one `E_WEB_*` line naming no URL, host or address;
  an unfetchable URL exits 2 with `E_WEB_URL_MALFORMED`.
- DNS uses `hickory-resolver` `=0.25.2` (the version reqwest already
  resolves) so lookups are bounded by `LOOKUP_CAP` (5 s).

### Shared `network` crate

- `Host`, `HostReach`, `classify_address` and a new `Authority::parse`
  move from `design` into `cli/network`, so `design` and `research` share
  one classifier. `design`'s rejection wording stays in
  `access_policy`; the vector corpus is split by owner, and the Playwright
  `access-policy.js` mirror tests against both halves.
- ⚠️ The classifier now treats more ranges as internal: IPv4-translated
  `::ffff:0:a.b.c.d`, site-local `fec0::/10`, local-use NAT64
  `64:ff9b:1::/48`, discard-only `100::/64` and the documentation ranges
  (`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`,
  `3fff::/20`). `design` users reaching these now need
  `--allow-internal`; the CHANGELOG and `design.md` record it.

### Transport deadline

- ⏱️ Both the academic `HttpTransport` and the new `HttpPageTransport`
  drive reqwest's async client under one `tokio::time::timeout` over the
  whole exchange, body included. Previously reqwest's blocking client
  applied its timeout per `read()`, so a server trickling a byte every
  29 s could hold a call well past its budget.

### Skills, agents and docs

- The finding and level-note outputters and the three templates document
  the block, the 19 recommended keys, the core keys' value forms and the
  key order; a test reads that order from the Rust source.
- The OpenAlex and arXiv profiles copy a cited record's `metadata`; the
  web profile runs `fetch web metadata` per cited page and grants
  `Bash(accelerator research fetch *)`.
- The composer and `synthesise` carry each block byte-for-byte and never
  merge, reorder or complete one; merging duplicates is 0306's.
- `research.md` gains a Sources metadata section, the command's exit
  codes and fetch rules.

### Work-item refinement

- 0348 split: bibliographic keys (`authors`, `volume`, `pages`, …) move to
  new draft 0349.
- 0306 is rewritten from "conflicting source tiers" to "citing a source
  once", reviewed and marked ready; 0281 is refined, stress-tested and
  marked ready; 0347 (redo/deepen findings) is split off 0281; 0305 is
  now blocked by 0348.

### Housekeeping

- `tokio` bumped to 1.53.3 for RUSTSEC-2026-0386; docs-site `zod`
  override clears its high npm advisories.
- `deny.toml`'s hickory-proto advisory ignores now cite both reqwest and
  `research-adapters` as pinning 0.25 (review-by 2026-11-03 unchanged).

## Context

- Work item: `meta/work/0348-structured-source-metadata.md`
- Plan: `meta/plans/2026-10-24-0348-structured-source-metadata.md`
- Research: `meta/research/codebase/2026-10-24-0348-structured-source-metadata.md`
- Validation: `meta/validations/2026-10-24-0348-structured-source-metadata-validation.md`
  (result `pass` after fixing a date-parser panic on a non-ASCII offset
  and the trickle-body deadline gap found in the first pass)

## Testing

- [x] `mise run check` exits 0 (re-run while writing this description)
- [x] `cargo test -p network -p research -p research-adapters -p design`
      passes (re-run while writing this description)
- [x] `cargo test -p accelerator-research --features test-loopback`
      passes, including the `fetch_web` end-to-end suite against a mock
      server (re-run while writing this description)
- [x] Full default `mise run` exits 0, all 68 tasks, per the validation
      report after the final fixes (not re-run here; the
      skill-invocation lane alone takes ~58 min)
- [x] Edge cases covered by tests: unusable and non-ASCII dates, DOI
      label and resolver forms, JSON-LD `@graph` and markup-in-string,
      unparseable-then-valid JSON-LD, redirects into private space,
      hosts reqwest reads differently, proxy env vars ignored, stalled
      and trickling bodies, 2 MiB truncation
- [ ] Live checks, not re-run on the final commit: PLOS ONE yields
      `doi`/`published`; `169.254.169.254` is refused unsent; PeerJ's
      403 exits 1 with one `E_WEB_*` line

## Notes for Reviewers

- 🔒 The SSRF boundary is the part most worth reading:
  `cli/research/src/sources/web/fetch.rs` (admission and redirect loop),
  `cli/research-adapters/src/page_transport.rs` (pinning, no proxy, no
  reqwest redirects) and `cli/research-adapters/src/host_form.rs` (the
  host re-read check). The `test-loopback` seam in
  `cli/research-cli/src/loopback.rs` is `cfg`-gated, a `compile_error!`
  blocks it in release builds, and `tasks/build.py` rejects a release
  artefact carrying its marker.
- Known, accepted deviations: a bracketed name such as
  `http://[evil.com]/` is accepted as the name `evil.com` and admitted
  like any other; `research-cli` depends on `network` unconditionally,
  though `research-adapters` already pulls it into every build.
- Unknown keys would currently keep insertion order rather than the
  documented byte-wise order; no producer emits one yet, and 0349 owns
  that rule.
- Findings are immutable, so every existing finding stays in the one-line
  form; no CLI or visualiser code parses Sources sections yet.
- Around a third of the diff is `meta/` (plan, research, reviews,
  validation and refined work items) and regenerated `public-api.txt`
  pins.
