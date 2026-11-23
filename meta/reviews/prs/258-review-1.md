---
type: "pr-review"
id: "258-review-1"
title: "[PP-891] Tier DOI-cited web sources by their Crossref venue, not by doi.org"
date: "2026-11-23T01:44:54+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:258"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "security", "performance", "documentation"]
review_number: 1
pr_number: 258
tags: []
last_updated: "2026-11-23T01:44:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #258 - [PP-891] Tier DOI-cited web sources by their Crossref venue, not by doi.org

**Verdict:** COMMENT

The queue-engine extraction is well built: pacing is a pure domain core (`PacingState::reserve/confirm/release/trust`) behind narrow ports, the correctness lens found no defect in the reserve/confirm loop or lock ordering, and the arXiv behaviour was pinned by characterisation tests before the refactor. The two major findings both sit in the web-profile skill text, not the Rust: a page-declared DOI can lift any page to tier-1, and `E_DOI_TICKET_LIVE` on an auxiliary metadata call ends the whole node as Failed. The PR also adds no `CHANGELOG.md` entry. The remaining findings are minor: invariants held by convention rather than by type, a few untested ladder and retry paths, and pacing margins.

### Cross-Cutting Themes

- **Service identity held by convention, not by type** (flagged by: architecture, code-quality, documentation) — `DoiAdapters` names its `Service` apart from the `ServiceAdapters` bundle built to name it once; `credential: bool` sits beside `Trust` and admits meaningless combinations; `parse_ticket` hard-codes which families are queued; `Service::code()` is both the state-directory key and the diagnostic prefix.
- **A Crossref pool flip silently loses a DOI ticket's place** (flagged by: architecture, correctness) — the `crossref-public` and `crossref-polite` queues share the `Doi` family but not a directory, so a ticket re-presented after `crossref.mailto` resolves differently joins the other pool afresh, with no rejection or diagnostic.
- **Ports remain reachable through `fetch`** (flagged by: architecture, code-quality) — `sources::fetch` re-exports all twelve ports and the adapters still import them from there, while `include_str!` tests guard the engine against the text `sources::fetch`. A `pup.ron` rule would enforce this properly.
- **Web-profile rules disagree with themselves** (flagged by: security, documentation) — rules 1–2 trust any DOI a fetched page declares; `E_DOI_TICKET_LIVE` contradicts "a failed call never changes the outcome"; rule 3's grammar and rule 4's venue-less entry line are ambiguous.
- **Upgrade overlap shares only the lock** (flagged by: compatibility, documented as an accepted risk in the PR) — old and new releases ignore each other's arXiv spacing and 429/403 deferrals; reading the frozen legacy `arxiv-pacing` read-only as an extra floor would close this cheaply.

### Tradeoff Analysis

- **Rate safety vs throughput**: the polite interval (102 ms) leaves ~2% slack, so connection-setup jitter can put limit + 2 arrivals into one second, while holding the serving slot through 3/6/12 s backoff stalls the single public slot for up to 21 s. Widening the polite interval costs little because 3 slots ÷ Crossref latency usually bounds throughput first; releasing the slot during backoff for `PreviousStart` services is worth it, keeping arXiv's behaviour unchanged.
- **Test reliability vs detection**: nextest retries keep the real-time tests green under load, but a retry that rescues a timing-sensitive spread assertion also rescues the per-second rate assertion, so an intermittent race reports FLAKY and passes. Splitting the exact rate check (no retry) from the spread check (retried) keeps both.
- **Extensibility vs YAGNI**: the closed `Binding` enum and the paired `document`/`doi_document`, `report`/`report_doi` functions grow one arm per family. With two families this is acceptable; plan the seam before a third.

### Strengths

- ✅ `PacingState` is a pure value type taking `now` explicitly, so the riskiest logic is tested to the millisecond without I/O; `FilePacingGate` only locks, persists and sleeps.
- ✅ Spacing never depends on the reservation cursor: `confirm` re-checks the last actual send and any deferral under `pacing.lock`, reading the clock only after taking it. Ghost reservations cost throughput, never the rate.
- ✅ Lock ordering is acyclic — slot and companion locks are non-blocking, and `pacing.lock` and `queue.lock` are never held together.
- ✅ `queued<T>`/`Queued<T>` gives the join/serve/step-aside/contention flow one home; `fetch_arxiv` and `fetch_doi` shrink to a `serve` closure.
- ✅ Rich domain vocabulary: `SpacedFrom::{PreviousFinish, PreviousStart}`, `Confirmation::{Send, WaitUntil, Deferred}`, `Provenance`, `CrossrefContact`, `ContactAddress`.
- ✅ Defensive inputs: `ContactAddress` limits the mailto to visible ASCII without comment delimiters (no header injection); `Doi::validated` rejects dot segments and the DOI is percent-encoded in the path; state paths come only from static service codes.
- ✅ The upgrade overlap is careful: a new `services/arxiv/` directory, the legacy `arxiv.lock` as a companion lock with matching open flags, read-only detection of old tickets, and a captured 1.24.0-pre.105 fixture.
- ✅ Tests reach every `confirm` branch, cover all four companion-lock states, assert exactly three in flight for the polite pool, and include a regression test for the mock server's routes-lock fix.
- ✅ `research.md` and `/configure` match the CLI precisely: exit table, `E_DOI_*` codes, admission budgets, plausibility rules.

### General Findings

- 🟡 **Documentation**: No `CHANGELOG.md` entry. `[Unreleased]` records `fetch web metadata` and the arXiv queue, but not `fetch doi metadata` and its `E_DOI_*` codes, `crossref.mailto` / `ACCELERATOR_CROSSREF_MAILTO`, the change in web-profile tiering, the move of arXiv state to `services/arxiv/` (old files deletable), or previous-release tickets now getting `E_ARXIV_TICKET_MISMATCH`.
- 🔵 **Architecture**: `Service::code()` doubles as the persistence key and the diagnostic label, so one DOI call writes stderr lines with two prefixes (`crossref-public ...` and `doi metadata ...`). Give storage its own accessor.
- 🔵 **Code Quality**: The `research fetch: {code} ...` diagnostic prefix is formatted separately in three adapters; the arXiv and DOI arms of `source_call` repeat the same project/context/transport setup.
- 🔵 **Test Coverage**: The nextest overrides select real-time tests by name substring with no guard; a rename silently drops a test out of its isolation or retry override.
- 🔵 **Security**: A `crossref.mailto` committed to `config.md` is sent to Crossref on every collaborator's calls. Worth one line in `/configure` pointing personal addresses at `config.local.md` or the env var.
- 🔵 **Compatibility**: Crossref's advertised rate-limit headers are not consulted, and an HTTP-date `Retry-After` falls back to the client's own backoff. Low priority.
- 🔵 **Documentation**: A few new doc comments in `render.rs` and `main.rs` still restate their items (`unserved_document`, `failure`, `doi_document`, `queued_context`).

### Additional Findings

- 🔵 `cli/research/src/sources/queue.rs:122` — Shared queue engine still closed over every family's `Binding` (architecture)
- 🔵 `cli/research/src/sources/queue.rs:283` — CLI usage-line wording moved from `render.rs` into the domain queue (architecture)
- 🔵 `cli/research/src/sources/serving.rs:58` — Nested `Result` alias obscures the three ways a turn can end (code-quality)
- 🔵 `cli/research/src/sources/pacing.rs:274` — "No finish since the send" relies on `Option`'s `None < Some` ordering (code-quality)
- 🔵 `cli/research/src/sources/fetch.rs:288` — Unreachable `Settled::Failed` arm silently discards a `FetchError` (code-quality)
- 🔵 `cli/research/src/sources/service.rs:13` — Polite interval has ~2% slack; limit + 2 arrivals per second possible (performance)
- 🔵 `cli/.config/nextest.toml:10` — Retries on the rate tests can mask an intermittent pacing race (test-coverage)
- 🔵 `cli/.config/nextest.toml:16` — Real-time tests add a 30–45 s serial tail to the CLI suite (performance)
- 🔵 `cli/research-cli/tests/fetch_doi.rs:368` — Tight `Retry-After` upper bounds with no retry (test-coverage)
- 🔵 `cli/research-adapters/src/services.rs:67` — Old and new releases ignore each other's arXiv deferrals and spacing (compatibility)
- 🔵 `cli/research-adapters/src/crossref_json.rs:28` — Non-string container titles are dropped before the first is chosen (correctness)
- 🔵 `cli/research-adapters/src/pacing.rs:325` — Blocking `flock` on `pacing.lock` is not bounded by the call's deadline (correctness)
- 🔵 `cli/research-cli/src/loopback.rs:69` — Loopback override check is a string prefix, bypassable with userinfo (security)
- 🔵 `skills/research/profiles/web-profile/SKILL.md:166` — Rule 3's wording is ambiguous; rule 4's venue-less entry line has no format (documentation)
- 🔵 `docs-site/src/content/docs/research.md:368` — Pacing paragraph describes only the public pool (documentation)
- 🔵 `docs-site/src/content/docs/research.md:165` — "Its bodies over 8 MiB" now has an ambiguous referent (documentation)
- 🔵 `cli/research-cli/src/render.rs:114` — Parallel record/DOI function pairs in `render` and `main` (code-quality)
- 🔵 `cli/research-cli/src/crossref_contact.rs:106` — Escaping misses bidi and format characters (security)
- 🔵 `cli/research/tests/doi_fetch.rs:137` — DOI's retry-then-serve path exercised only by slow real-time tests (test-coverage)
- 🔵 `cli/research-adapters/src/queue.rs:586` — Every admission poll lists the queue directory, at 25 ms for the polite pool (performance)
- 🔵 `cli/research-adapters/src/pacing.rs:346` — A pacing file with another schema version is treated as empty, then overwritten (compatibility)
- 🔵 `cli/research/src/sources/service.rs:50` — DOI documents name `source: "crossref"` while every other identifier says `doi` (compatibility)
- 🔵 `cli/config/src/catalogue.rs:165` — `credential: bool` beside `Trust` admits meaningless combinations (architecture, code-quality)
- 🔵 `docs-site/src/content/docs/research.md:1080` — "Cited by DOI" is undefined (documentation)
- 🔵 `cli/research/src/sources/service.rs:64` — Doc comment makes up for the vague name `code()` (documentation)


## Inline Comments

### `skills/research/profiles/web-profile/SKILL.md:49-51` — A page-declared DOI lets any page take a tier-1 venue
**Severity**: major | **Confidence**: high | **Lens**: security

🟡 **Security**

Step 3 runs `fetch doi metadata` on the `doi` that `web metadata` printed "whatever the page's host". That DOI is whatever the fetched, untrusted page declares in its meta tags. Rules 1 and 2 (lines 158–165) then give the Crossref venue tier-1, or the publisher tier-2, and "override every domain rule". So a low-reputation or lookalike page (say `stripe.com.evil.io` or an SEO blog) can declare `citation_doi` for a well-known Nature or ACM paper and be recorded as `— tier-1 — Nature`, with its own URL still as the link. This is exactly the attack the lookalike-domain paragraph sets out to block.

The rules also make no distinction between a DOI the researcher chose to cite (a resolver URL) and one a page asserted about itself. Only the second is attacker-influenced, yet both get the same trust, and nothing in the entry marks the venue as unverified.

**Suggestion**: For a non-resolver page, trust a declared DOI only when it is tied to the page. For example, `WebFetch` its resolver URL and apply rules 1–2 only when the landing host matches the page's host. Otherwise tier the page by its own domain and keep the DOI as plain metadata. A CLI-side host check would enforce this more reliably than prose.

---

### `skills/research/profiles/web-profile/SKILL.md:87` — E_DOI_TICKET_LIVE ends the whole web node as Failed, against the profile's own rule
**Severity**: major | **Confidence**: medium | **Lens**: documentation

🟡 **Documentation**

Lines 67–70 say a failed metadata call only leaves that entry without a block and "never makes the outcome Denied or Unreachable". This line, though, ends the whole node as Failed when a single auxiliary `doi metadata` call gets `E_DOI_TICKET_LIVE`, and Failed writes no file, discarding every page already researched. The arXiv profile justifies the same rule with "since you run one fetch at a time". The web profile gives no reason and sets no one-fetch-at-a-time rule.

**Impact**: One live ticket on one source's metadata throws away the node's research, and agents may apply the two contradictory rules inconsistently.

**Suggestion**: Either handle `E_DOI_TICKET_LIVE` as a printed `unavailable` for that source, or state why it must end the node and add the one-fetch-at-a-time rule that makes the code meaningful.

---

### `cli/research-cli/src/fetch_command.rs:108-119` — DoiAdapters restates the service ServiceAdapters was built to name once
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`services.rs` says gate, queue and contention are built together "so one call names the service once and the three can never disagree". `DoiAdapters` then carries a separate `service: Service` beside the three boxes, and `main.rs:400` sets it to `Service::Crossref(pool)` a second time. `main.rs` derives the pool from `CrossrefContact` three times in all: `for_crossref(pool, ..)`, `Service::Crossref(pool)` and `contact.address()`. The doc comment here is the only thing asserting they agree.

**Impact**: A later edit could render `source` and the ticket family for one pool while pacing against another, or send a `mailto` while queueing in the public pool, and it would still compile.

**Suggestion**: Have `ServiceAdapters` expose the `service` it was built for, and let `DoiAdapters`/`ArxivAdapters` hold the bundle rather than three loose boxes. Derive the pool from `Option<&ContactAddress>` in one place (for example `CrossrefPool::for_contact`), then delete the doc comment.

---

### `cli/research/src/sources/request.rs:146` — Which families are queued is hard-coded apart from Service
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`parse_ticket` decides whether `--ticket` applies with `matches!(family, Family::Arxiv | Family::Doi)`, and `TicketNotQueued`'s message hard-codes "arxiv and doi". Meanwhile `Service::ticket_family` is the authoritative mapping from queued services to ticket families.

**Impact**: A third queued service has to update `Service`, this `matches!` and the error text separately. Missing one gives a confusing usage error, or accepts a ticket for a family that isn't queued.

**Suggestion**: Derive it from the domain, for example `Family::is_queued()` as `Service::ALL.iter().any(|s| s.ticket_family() == self)`, and build the message from the same list with `alternatives(...)`, as `UnknownFamily` now does.

---

### `cli/research-cli/src/main.rs:349-355` — Composition root hard-codes the previous release's storage layout
**Severity**: minor | **Confidence**: high | **Lens**: architecture

🔵 **Architecture**

The CLI builds `PreviousRelease { queue: research.nested("arxiv-queue"), lock: research.path("arxiv.lock") }` itself. The new layout (`services/<code>/queue`, `serving-<i>.lock`, `pacing.lock`) belongs to `research-adapters` (`ScratchDir::service_state`, `FileServiceQueue::directory_of`, the `pacing` constants). So two crates now share knowledge of the on-disk layout.

**Impact**: The upgrade overlap depends on these literals matching what the old release wrote, but they live away from the code that defines the layout. Retiring the overlap will mean changing the CLI as well as the adapters.

**Suggestion**: Move the legacy paths into `research-adapters::services`, for example `PreviousRelease::arxiv_layout(&research)`, so the CLI only opts in and the adapters own every path.

---

### `cli/config/tests/open_setting.rs:194-203` — No test that a higher rung short-circuits an unreadable lower level
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

The ladder promises that a lower level is read only when no higher rung holds a value. The only unreadable-level test makes `config.local.md` unreadable with no environment value, and the in-memory `Project` fake cannot make the team file unreadable at all. A mutation that reads every level up front, or checks the files before the environment, passes every ladder test. The same gap exists in the `CrossrefContact` tests.

**Impact**: If the short-circuit regressed, a user with `ACCELERATOR_CROSSREF_MAILTO` set and a broken `config.local.md` would silently drop to the public pool with a warning.

**Suggestion**: Add two cases: an environment value plus an unreadable personal file resolves to the environment value, and a personal value plus an unreadable team file resolves to the personal value. Give the fake a `team_unreadable` option for the second.

---

### `skills/config/configure/SKILL.md:1336-1337` — An unreadable config.local.md stops config.md being read, but the docs imply otherwise
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The section says `config.md` "is read whenever no higher rung holds a non-blank value, even when `config.local.md` exists". `open_value` actually returns `UnreadableLevel` as soon as it cannot read `config.local.md`, so a valid address in `config.md` is never read and the call goes to the public pool. The test `an_unreadable_level_calls_anonymously` pins this.

**Impact**: A team that commits `crossref.mailto` to `config.md` will be surprised to land in the public pool when one member's `config.local.md` is unreadable.

**Suggestion**: Add to the Unreadable bullet: "Lower rungs are not read, so the call uses the public pool even if `config.md` holds an address."

---

### `cli/research-adapters/src/services.rs:98-107` — A DOI ticket re-presented after the Crossref pool changes silently joins afresh
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, architecture

🔵 **Correctness / Architecture**

`other_family_queues` skips services with the same ticket family, so `crossref-public` and `crossref-polite` are not foreign to each other, yet each has its own queue directory. If `crossref.mailto` resolves differently between a `waiting` result and its re-presentation, the ticket is missing from this pool's snapshot and from the foreign list, so `Queue::present` returns `Join`. That happens when the env var isn't passed to a subagent's shell, `config.local.md` becomes unreadable, or the value becomes implausible. The call then takes a new ticket at the back of the other pool's queue, with no `E_DOI_TICKET_MISMATCH` and no diagnostic, while its old place lingers until it expires.

**Impact**: The caller loses its place without any signal. Every other cross-queue re-presentation in this PR becomes an explicit rejection.

**Suggestion**: When a presented ticket isn't found, also list the sibling pool's queue read-only, then either resume the ticket there or reject it with a provenance naming the other pool. At minimum, write a diagnostic, or document that changing `crossref.mailto` mid-run resets queued DOI calls.

---

### `cli/research/src/sources/fetch.rs:5-16` — Ports re-exported through fetch while the engine is string-tested not to depend on fetch
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

The ports moved to `sources::ports`, but `fetch.rs` re-exports all twelve with `pub use`. The adapters (`pacing.rs`, `contention.rs`, `queue.rs`, `services.rs`, `crossref_json.rs`) and the CLI still import `research::sources::fetch::{Clock, PacingGate, ...}`. Meanwhile `ports.rs`, `serving.rs` and `admission.rs` each carry an `include_str!` test asserting that their source never contains `"sources::fetch"`.

**Impact**: Every port has two public paths, and consumers still reach the ports through the workflow module the refactor set out to separate them from. The text tests are brittle: a doc comment mentioning the path fails them, and `use crate::sources::{fetch}` or a `super::` path slips past.

**Suggestion**: Point the adapters and the CLI at `research::sources::ports` and drop the re-exports. Express the dependency rule in `cli/pup.ron`, for example a `RestrictImports` on `^research::sources::(ports|serving|admission)`, instead of the `include_str!` tests.

---

### `cli/research/src/sources/serving.rs:207-216` — Serving slot held through retry backoff stalls the public pool's only slot for up to 21 s
**Severity**: minor | **Confidence**: medium | **Lens**: performance

🔵 **Performance**

`until_settled` sleeps the 3/6/12 s backoff inside the serving turn, so the `serving-<i>.lock` slot stays held while nothing is in flight. That suits arXiv's one-connection rule, but Crossref's slots limit requests in flight. In the public pool, which has one slot, a DOI that draws a 5xx or transport error blocks every other public caller for up to 21 s, when their own spacing would let them send every 210 ms. A 429 already defers every caller through the shared `not_before`, so holding the slot adds nothing there either.

**Impact**: During a web-profile fan-out, one bad response can push many concurrent DOI lookups into `waiting` and force ticket re-runs.

**Suggestion**: For `PreviousStart` services, give the slot back during backoff (end the turn, sleep, then re-admit while keeping the queue place). Keep the current behaviour for arXiv's `PreviousFinish`.

---

## Per-Lens Results

### Architecture

**Summary**: The arXiv-only queue now sits behind a shared `queued` engine. Dependencies still point the right way: the `research` domain crate holds the pure pacing and queue rules and the port traits, `research-adapters` implements them with files and `flock`, and `research-cli` wires them up at the composition root. The weak spots are small. The engine's `Binding` is still a closed list of every family, the composition root hard-codes the legacy storage layout, a few invariants rest on doc comments rather than types, and the domain has picked up some presentation strings.

**Strengths**:
- `PacingState` is a pure functional core over plain `SystemTime` values; the adapter only does locking, I/O and sleeping.
- The port traits now live in one place, `sources::ports`, and `Diagnostics` belongs to the domain instead of an adapter.
- `queued<T>`/`Queued<T>` separate the queue lifecycle from what each family does once served; `render::queued_ending` reads every queue state in one place.
- `Service`/`Pacing`/`SpacedFrom` speak the domain's language, and each service declares its pacing once.
- `ServiceAdapters::of` builds gate, queue and contention log from one `Service`.
- The generic `open_value` ladder knows nothing of Crossref; the `credential` marker keeps secrets off the open ladder.
- `pup.ron` covers `crossref_json`.

**Comments**:
- `cli/research/src/sources/queue.rs:122-126` (minor, medium) — The shared engine's `Binding` is still a closed enum of every family; each new service edits the engine and its on-disk codec.
- `cli/research/src/sources/queue.rs:283-327` (minor, medium) — The `RejectedTicket`/`Binding` `Display` impls render CLI usage text that used to live in `render.rs`; `fetch_doi` formats its own diagnostic line.
- `cli/research-cli/src/main.rs:349-355` (minor, high) — The composition root hard-codes the previous release's storage layout.
- `cli/research-cli/src/fetch_command.rs:108-119` (minor, medium) — Service/pool agreement is held by a doc comment, not a type.
- `cli/research/src/sources/fetch.rs:5-16` (suggestion, medium) — Ports re-exported from `fetch`; the `include_str!` guards are easy to get around.
- `cli/config/src/catalogue.rs:165` (suggestion, medium) — `credential: bool` beside `Trust` admits meaningless combinations.
- `cli/research-adapters/src/services.rs:98-107` (suggestion, low) — Pool-keyed queues orphan a DOI ticket when `crossref.mailto` changes.

**General Findings**:
- (suggestion) `Service::code()` doubles as the persistence key and the diagnostic label.

### Code Quality

**Summary**: The refactor from an arXiv-only queue to a shared, per-service engine is well designed, with pure pacing, narrow ports and rich domain language. The remaining concerns are local: the DOI path restates its service, which families are queued is hard-coded, a nested `Result` hides control flow, two places lean on `Option` ordering, and `render` and `main` have paired record/DOI functions.

**Strengths**:
- `PacingState` transitions take `now` explicitly, so they test without a filesystem or clock.
- `serving::queued` gives the join/serve/step-aside/contention flow one home.
- `ServiceAdapters::of` and `other_family_queues` derive from `Service::ALL`.
- Rich vocabulary: `SpacedFrom`, `Confirmation`, `Provenance`, `CrossrefContact`; comments explain why (for example the 9 × 102 ms derivation).
- `MalformedTicket` decouples ticket parsing from `RequestError`.
- `fenced_block_commands` is now a projection of `fenced_blocks`.

**Comments**:
- `cli/research-cli/src/main.rs:400` (minor, high) — `DoiAdapters` restates the service `ServiceAdapters` was built to name once.
- `cli/research/src/sources/request.rs:146` (minor, high) — Which families are queued is hard-coded apart from `Service`.
- `cli/research/src/sources/serving.rs:58` (minor, medium) — `Served<T> = Result<Result<T, Unavailable>, OutOfBudget>` hides three outcomes; prefer a named enum.
- `cli/research/src/sources/pacing.rs:274` (minor, medium) — "No finish since the send" relies on `None < Some`; name the rule once.
- `cli/research/src/sources/fetch.rs:5-16` (minor, medium) — Ports re-exported through `fetch` while string tests forbid depending on it.
- `cli/research/src/sources/fetch.rs:288-290` (minor, medium) — The unreachable `Settled::Failed` arm discards a `FetchError` silently.
- `cli/research-cli/src/render.rs:114-125` (suggestion, medium) — Paired record/DOI functions in `render` and `main`.
- `cli/config/src/catalogue.rs:165` (suggestion, low) — `credential: bool` beside `trust`.

**General Findings**:
- (suggestion) The diagnostic prefix is duplicated across three service adapters.
- (suggestion) `source_call`'s arXiv and DOI arms repeat the same setup.

### Test Coverage

**Summary**: Coverage is strong and fits the risk. The pure pacing domain, the file gate, Crossref classification, contact plausibility and the mailto ladder all have specific tests that would catch mutations, and the arXiv refactor was pinned by characterisation tests before it began. The gaps are small: retries that can hide a race, tight bounds with no retry, an untested ladder short-circuit, and DOI retry-to-success covered only by slow tests.

**Strengths**:
- Every `confirm` branch is pinned to the exact millisecond, including the rounding and cursor bounds.
- `now_is_read_only_once_the_pacing_lock_is_held` makes the stale-`now` race deterministic.
- A refused call is shown to leave no state behind at both refusal points.
- The companion-lock tests cover all four states.
- The polite pool is asserted at exactly three in flight.
- Every Crossref status class is covered, with fallback ordering checked in both directions.
- The mock-server fix has a regression test, and the PR records a manual mutation check.

**Comments**:
- `cli/.config/nextest.toml:10-25` (minor, medium) — Retries on the rate tests can mask an intermittent pacing race.
- `cli/research-cli/tests/fetch_doi.rs:368-375` (minor, low) — Tight `Retry-After` upper bounds with no retry.
- `cli/config/tests/open_setting.rs:194-203` (minor, high) — No test that a higher rung short-circuits an unreadable lower level.
- `cli/research/tests/doi_fetch.rs:137-147` (suggestion, medium) — DOI's retry-then-serve path is exercised only by slow real-time tests.

**General Findings**:
- (suggestion) The nextest overrides select tests by name substring with no guard.

### Correctness

**Summary**: No correctness defects in the pacing core. Spacing safety depends only on `confirm`, which runs under `pacing.lock` and reads the clock after taking it. Lock ordering has no cycles, and the slot counting in `is_served_given` is not off by one. The minor findings are edge cases: a pool flip, null container titles, and an unbounded blocking `flock`.

**Strengths**:
- Spacing never depends on the reservation cursor.
- The clock is read only after `pacing.lock` is taken.
- Every give-up path hands its reservation back.
- Lock ordering cannot deadlock.
- `trust` repairs a backward clock step conservatively.
- DOI bindings compare on `canonical()`, so case-insensitive matching holds.

**Comments**:
- `cli/research-adapters/src/services.rs:104` (minor, medium) — A DOI ticket re-presented after a pool change silently joins afresh.
- `cli/research-adapters/src/crossref_json.rs:28` (minor, low) — A null first container title shifts the venue to the second.
- `cli/research-adapters/src/pacing.rs:325` (minor, low) — The blocking `flock` on `pacing.lock` is not bounded by the deadline.

**General Findings**: none.

### Compatibility

**Summary**: The upgrade overlap holds up. arXiv state moves to a new directory, the legacy `arxiv.lock` is taken as a companion lock, and old tickets are detected read-only and rejected with a code the profile already recovers from. The main gap is that the two releases don't share pacing or deferral state; the PR accepts this, but a cheap read-only mitigation exists. Everything else is additive or confined to the workspace.

**Strengths**:
- The new state directory means the two releases never parse each other's files.
- The companion lock uses the same open flags as the old release.
- Old tickets are listed read-only and rejected with `E_ARXIV_TICKET_MISMATCH`, pinned by a 1.24.0-pre.105 fixture.
- arXiv JSON output is unchanged.
- The pacing file carries a `schema_version`, and all its fields are required.
- The config changes are additive, and the credential marker is tested.
- The public-API fixtures are regenerated, and every `fenced_blocks` caller is updated.
- The Crossref request matches the published contract.
- The `url` 2.5.9 bump is already on the base branch.

**Comments**:
- `cli/research-adapters/src/services.rs:67` (minor, medium) — Old and new releases ignore each other's arXiv deferrals and spacing.
- `cli/research-adapters/src/pacing.rs:346-367` (suggestion, low) — A pacing file with another schema version is treated as empty, then overwritten.
- `cli/research/src/sources/service.rs:50-55` (suggestion, medium) — DOI documents say `source: "crossref"` while other identifiers say `doi`.

**General Findings**:
- (suggestion) Crossref's advertised rate-limit headers are not consulted.

### Security

**Summary**: The CLI side is defensively built: no header injection, DOIs validated and percent-encoded, strict ticket parsing, static state paths, and credentials kept off the open ladder. The main gap is in the web-profile skill: a DOI declared by an untrusted page reaches Crossref, and the resulting venue outranks every domain rule. Separately, the loopback override check is a bypassable string prefix.

**Strengths**:
- `ContactAddress::parse` blocks CR/LF and comment breakout.
- `Doi::validated` and `encoded()` prevent path traversal.
- The credential flag is enforced at both ends.
- Warnings escape control characters.
- Tickets round-trip, and every state path comes from a static `Service::code()`.
- The untrusted-content contract extends to `doi metadata` output.

**Comments**:
- `skills/research/profiles/web-profile/SKILL.md:49-51` (major, high) — A page-declared DOI lets any page take a tier-1 venue.
- `skills/research/profiles/web-profile/SKILL.md:158-165` (minor, medium) — The venue rules don't tell resolver-cited and page-declared DOIs apart.
- `cli/research-cli/src/loopback.rs:69-70` (minor, medium) — The loopback override check is a string prefix and can be bypassed with userinfo; this also affects the OpenAlex key.
- `cli/research-cli/src/crossref_contact.rs:106-119` (suggestion, low) — Escaping misses bidi and format characters.

**General Findings**:
- (suggestion) A team-file mailto is sent on every collaborator's calls.

### Performance

**Summary**: The pacing design is efficient. Lock hold times are short, sleeps happen outside the lock, state writes skip `fsync`, admission scans are bounded, and foreign queues are listed only on a mismatch. The concerns are margins and throughput: about 2% slack on the polite interval, a slot held through retry backoff, and the real-time tests' share of suite time.

**Strengths**:
- `PacedCall` holds `pacing.lock` only for a read-compute-write.
- `replace_without_sync` keeps `fsync` off every request.
- `is_served_given` stops at `nth(slots - 1)`.
- `foreign_tickets()` runs only on a mismatch.
- `admission_poll` scales with the service.
- The mock server drops the routes guard before sleeping.

**Comments**:
- `cli/research/src/sources/service.rs:13-18` (minor, medium) — The polite interval has about 2% slack; limit + 2 arrivals per second are possible.
- `cli/research/src/sources/serving.rs:207-216` (minor, medium) — The serving slot is held through retry backoff.
- `cli/research-adapters/src/queue.rs:586-592` (suggestion, low) — Every admission poll lists the queue directory, every 25 ms in the polite pool.
- `cli/.config/nextest.toml:16-39` (minor, medium) — The real-time tests add a 30–45 s serial tail; cut `ANSWERED_AFTER` to about 500 ms.

**General Findings**: none.

### Documentation

**Summary**: The user-facing docs mostly match the CLI. The main gaps are a missing `CHANGELOG.md` entry and the web profile's `E_DOI_TICKET_LIVE` rule contradicting its metadata-failure rule. A few phrasings could mislead a reader or an agent.

**Strengths**:
- `research.md` was updated in step with the CLI: exit table, `E_DOI_*` codes, budgets.
- The `/configure` `crossref` section matches `ContactAddress::parse` exactly.
- The upgrade guidance is concrete.
- The `research-topic` recovery row matches the CLI's real `warning:` prefix.
- Most doc comments explain why.
- The worked examples cover all four tiering branches.

**Comments**:
- `skills/research/profiles/web-profile/SKILL.md:87` (major, medium) — `E_DOI_TICKET_LIVE` ends the whole node as Failed.
- `skills/research/profiles/web-profile/SKILL.md:166-170` (minor, medium) — Rule 3's wording is ambiguous; rule 4's entry line has no format.
- `docs-site/src/content/docs/research.md:368-374` (minor, medium) — The Pacing paragraph describes only the public pool.
- `docs-site/src/content/docs/research.md:165-168` (minor, medium) — "Its bodies over 8 MiB" has an ambiguous referent, and the DOI outcome for an oversized body is unstated.
- `skills/config/configure/SKILL.md:1336-1337` (minor, high) — An unreadable `config.local.md` stops `config.md` being read.
- `docs-site/src/content/docs/research.md:1080-1083` (suggestion, medium) — "Cited by DOI" is undefined.
- `cli/research/src/sources/service.rs:64-65` (suggestion, medium) — A doc comment makes up for the vague name `code()`.

**General Findings**:
- (major) No CHANGELOG entry for the new command, the config key or the upgrade step. Verified: `CHANGELOG.md` has no Crossref, `doi metadata` or `crossref.mailto` entry.
- (suggestion) A few new Rust doc comments still restate their items.

---
*Review generated by /accelerator:review-pr*
