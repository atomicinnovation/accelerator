---
type: work-item
id: "0210"
title: "Provider Client Crates over the RemoteTracker Port"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: done
kind: story
priority: medium
parent: "work-item:0171"
blocks: ["work-item:0211", "work-item:0212"]
relates_to: ["work-item:0194", "work-item:0204", "codebase-research:2026-08-17-0210-provider-client-crates-over-the-tracker-port"]
tags: [rust, jira, linear, integrations, reqwest, tracker]
last_updated: "2026-08-17T14:10:59+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-740
---

# 0210: Provider Client Crates over the RemoteTracker Port

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build `jira-client` and `linear-client` as adapter crates over `reqwest` +
rustls + serde, each implementing the `RemoteTracker` port 0204 froze, and wire
both into `accelerator-work`'s composition root so the sync engine resolves real
providers rather than fakes. This child touches no bash, so every bash oracle the later
children delete is still available to verify against.

Precisely what changes for a user when this merges: **nothing**. No skill invokes
the new binaries or the sync engine's new provider resolution — the jira, linear
and work skills all still shell out to bash until 0211 and 0212 repoint them. What
does change is that `accelerator work sync`, already shipped by 0194, becomes
*able* to resolve real clients instead of fakes when invoked directly.

The build-versus-buy question is now researched rather than assumed — see
`## Build versus Buy`. The answer is still build, but for recorded reasons.

## Context

First of four children of 0171, and the blocker for two of them. Everything the
later children delete — the projection recipes, the four exit-code mapping
tables, the ADF node-type inventory, the eleven parity tests' baselines — is
still on disk while this lands, which is the only window in which those oracles
can be transcribed and verified against.

`RemoteTracker` is synchronous and six items wide (the trait, `ExternalId`,
`RemoteIssue`, `RemoteTimestamp`, `FetchOutcome`, `TrackerError`). A capability
needing a new port operation is out of scope here — it becomes a new work item
blocking 0171, not an amendment to a done and frozen 0204.

## Build versus Buy

Researched 2026-08-17. **Build both clients.** The evidence, per provider:

- **Linear**: no option exists. The official SDK is TypeScript-only; Linear's docs
  tell every other language to point an HTTP client at the GraphQL endpoint. The one
  Rust crate naming itself a Linear SDK, `linear_sdk`, is v0.0.1 with its last commit
  on 2022-10-30.
- **Jira**: exactly one candidate clears the gate. `jira_query` declares `reqwest`
  with **default features on**, which pulls `native-tls` → `openssl`, both banned by
  `cli/deny.toml`. `jira-api-v2` is GPL-3.0-or-later; `jira-issue-api` is `Unlicense`;
  `atlassian-cli-api` sets `native-tls-vendored`; three others require `reqwest ^0.13`
  against this workspace's `=0.12.28` pin. Only **`gouqi`** (MIT, `reqwest`
  `default-features = false` with `["blocking","rustls-tls","json","multipart"]`, v2+v3,
  real ADF node types) is viable.
- **`gouqi` is nonetheless not adopted.** Bus-factor 1, last release 2025-10-21, last
  commit 2025-10-20, no archive notice. Its `full` feature pulls `rsa`
  (RUSTSEC-2023-0071 — the advisory `cli/Cargo.toml:138-142` already swaps
  `jwt-aws-lc-rs` in to dodge) and `serde_yaml` (unmaintained, tripping
  `unmaintained = "all"`). Its `Error` exposes `reqwest::Error`, `url::ParseError` and
  `http::StatusCode`, so an adapter boundary is required regardless. The decisive
  signal is that **no one chose it**: every serious Rust Jira project surveyed shipped
  its own client, six of them after `gouqi` had v3 and ADF.
- **Read `gouqi` rather than depend on it.** It did the two genuinely non-obvious
  pieces — the `/rest/api/3/search/jql` cursor-pagination migration that broke most
  Jira clients, and a complete ADF node model. MIT permits lifting the shapes with
  attribution.
- **GraphQL codegen is not worth it** for this query count. `cynic` is **MPL-2.0**,
  which `cli/deny.toml:46-51` says must *never* be added to the blanket allow — it
  would need a per-crate exception justified like `uluru`'s. More decisively, Linear
  does no API versioning and leaves *"a non-functioning stub"* when removing
  functionality, so a stub still matches a committed schema, generates, type-checks and
  returns nothing. Codegen cannot catch the drift that actually threatens us; the
  contract test that can is needed either way. Hand-roll the query strings with serde.
- **ADF conversion may be composed rather than written.** The Rust ADF ecosystem is
  thin but **entirely permissive — no copyleft anywhere**: `htmltoadf` (HTML→ADF),
  `adf2html` (MIT), `jc-adf` (MIT, markdown↔ADF, but 4 stars — vendor rather than
  depend). Atlassian's `@atlaskit/adf-schema` is Apache-2.0 and is the canonical
  porting source. Licence is not a constraint on this decision.

⚠️ **Jira REST v2 is not deprecated and takes plain strings, but is foreclosed here.**
It would eliminate the ADF problem, and the 2024-2026 deprecations were endpoint-scoped
and hit v2 and v3 identically. But the committed corpus pins ADF into the projection:
`work-item-project-remote.sh:72` runs `jq -cS '.fields.description // null'` over a v3
ADF object and `remote_hash` is the sha256 of the normalised result. Adopting v2 would
change the body projection for every Jira item and reclassify the whole corpus as
`remotely-modified` on first sync — the exact failure the offline corpus criterion
exists to prevent. Record this as considered and declined; do not let a later reader
rediscover v2 and assume it is free.

## Requirements

### Client crates

- Implement `jira-client` (Jira REST v3 + Atlassian Document Format (ADF)↔markdown
  + auth) and `linear-client` (Linear GraphQL + auth) as adapter crates over
  `reqwest` + rustls + serde, each `impl RemoteTracker`. No native-tls, so the
  clients stay musl-static-friendly. Reuse the workspace `reqwest` entry
  **verbatim** — all three of its features are load-bearing against a named gate, and
  it does not currently include `json`.
- Name the crates so they do not contain the substring `tracker-adapters`:
  `cli/tracker/tests/structure.rs:67-77` asserts the workspace manifest contains no
  such string, with the comment *"provider clients live in their own crates"*.
- **Close the error-classification and auth gap.** The review accepted, unresolved,
  that no criterion covers HTTP-status or GraphQL-level classification or auth — so
  *"a client that misclassifies an auth failure as retryable passes every criterion"*.
  This child must therefore carry, per provider **and per operation**, a mapping from
  HTTP status (401, 403, 404, 429, 5xx) and from Linear's `200`-carrying-`errors` body
  to a `TrackerError` class, plus a non-interactive credential resolution design.
  Linear's rate limiting returns **HTTP 400** with `"code": "RATELIMITED"` in the body,
  so classification must parse the body on 400, not only on 200.
- Auth must be non-interactive per ADR-0045 — no prompting, no interactive OAuth. Port
  `jira-auth.sh`'s multi-source precedence and its `24 E_NO_TOKEN` /
  `25 E_TOKEN_CMD_FAILED` / `26 E_TOKEN_CMD_FROM_SHARED_CONFIG` outcomes, including the
  refusal to honour a `token_cmd` set in the shared config. `cli/collaboration-cli/src/auth.rs:27-91`
  already implements this precedence for GitHub and states in its own comments that it
  mirrors the jira/linear ban; copy it. Decide explicitly whether the `token_cmd`
  shell-out survives.
- **Settle the ownership of the five port-less provider flows** before planning
  commits to a shape. `comment`, `transition`, provider `search`, `attach` and `init`
  have no `RemoteTracker` operation, yet this child forbids `reqwest::` outside its two
  crates and 0211 calls itself thin. Either this child builds their request
  construction (and says so, with the crates sized accordingly), or a named work item
  owns them. 0194's recorded hole — **no way to push an existing unsynced item** — is
  the same shape and needs the same decision.

### Composition root

- Wire both clients into `accelerator-work`'s composition root under the
  `work.integration` config values 0194 exercised against fakes (`jira` and
  `linear`). The single substitution point is `cli/work-cli/src/tracker_registry.rs:52-63`,
  where every arm currently returns `Err` and all four tracker names are
  indistinguishable.
- ⚠️ **This is a signature change, not a one-line substitution.**
  `TrackerRegistry::resolve(&self, name: &str)` takes no config access, and
  `ConfiguredTrackers` is a unit struct constructed inline at `main.rs:220`, `:270` and
  `:377`. A Jira client needs `jira.site`, `jira.email` and a token; a Linear client
  needs a team id. Either widen `resolve` or give the registry state, and update all
  three construction sites.
- **`linear.team_id` does not exist.** `cli/config/src/catalogue.rs:121-133`'s
  `EXTRA_KEYS` carries only `linear.token` and `linear.token_cmd`. Adding a key means
  editing the catalogue and its bash mirror `scripts/config-defaults.sh`.
- Update the tests that pin the current not-available behaviour: `cli_sync.rs` (five
  — `:72`, `:82`, `:90`, `:127`, and the stdin case), `cli_update_push.rs:114`,
  `cli_create_push.rs:157`, and `cli_surface.golden:134` if the help text moves.
- Resolve the `for_tracker_error` duplication while the composition root is open. The
  0194 validation records that `work_cli::exit_codes::for_tracker_error` carries
  `#[allow(dead_code)]` while `create.rs:346` hand-rolls an identical twin, so *"the
  taxonomy the module exists to centralise is split in two"*. This child makes it live.

### Testing

- ⚠️ **`wiremock` reverses a decision this repository recorded in-line, twice.**
  `cli/github/tests/common/mod.rs:5-10` states the existing harness was hand-rolled
  *"mirroring that file's own precedent for HTTP-level test stubbing in this workspace
  (**no `wiremock`/`mockito`**)"*, and two working std-only mock servers already exist
  (`cli/launcher/tests/common/mod.rs`, `cli/github/tests/common/mod.rs`). Adopting a
  library is permitted but owes an argument. Three facts bear on it:
  - `cli/deny.toml` sets no `exclude-dev`, so a dev-dependency tree is evaluated for
    licences, advisories and bans across **five** targets under `unmaintained = "all"`.
  - On licence surface `wiremock` is the **safest** of the three candidates (zero new
    licences); `mockito`'s default features pull MPL-2.0 via `colored`; `httpmock`'s
    non-optional `stringmetrics` declares a `"non-standard"` licence that cannot map to
    SPDX. The common assumption that `wiremock` is the heavy one is wrong.
  - **No library covers the timeout case.** None can hang the connect phase or drop a
    connection mid-body; `mockito` cannot delay at all. The existing hand-rolled
    `Route::Stall(Duration)` does exactly what the timeout criterion needs. A library
    would be paid for *and* the hand-rolled responder kept.

  The cheapest path is extracting the two existing copies into one dev-only crate,
  adding request-body capture. Whichever is chosen, record it.
- The async-to-sync bridge is **not** a reason to avoid a library: `wiremock` runs its
  server on its own thread with its own runtime, so a blocking client cannot deadlock
  against it. A plain `#[test]` with `rt.block_on(…)` around setup and the blocking call
  *outside* it costs about six lines. Do not cite bridging as the deciding factor.
- Gate the credentialed contract lane by **reusing the existing mechanism**, which is
  the nextest binary-name filter in `cli/.config/nextest.toml`:
  `default-filter = 'not binary(=contract)'`. `tasks/test/integration.py:163-170` only
  *selects* the `contract` profile and sets `ACCELERATOR_TRACKER_CONTRACT=1`.
  Consequently each client's harness **must be named `tests/contract.rs`** — naming it
  `tracker_contract.rs` silently joins the default run and makes live API calls in
  `mise run`. The task is workspace-wide, so a new crate needs no task, `mise.toml` or
  filter edit. Introduce no second mechanism: no cargo feature, no `#[ignore]`, no
  per-crate `-E` expression.
- Implement `ContractSubject` for both clients. Each must nominate an id it will report
  `indeterminate` (Linear: the 250-item truncation or complexity cap; Jira: the 50-key
  chunking or 20-page cap) and an id whose `show` fails. Neither is free against a live
  tenant. If a new gated entry point is added to the harness, add it to
  `gated_calls()` (`cli/tracker-test-support/src/contract.rs:249-267`) or the
  gate-closure guard silently lapses.

### Oracle transcription

- **Transcribe the three oracles that later children destroy**, as part of this
  child's own change, each at a named committed path in a diffable format:
  - the four exit-code mapping tables, verbatim, into a fixture keyed by
    (code, provider, operation, class), at
    `cli/tracker/tests/fixtures/bridge-exit-code-tables.txt`;
  - the inventory of ADF node types the bash conversion handles, as a
    one-node-type-per-line list at
    `cli/jira-client/tests/fixtures/adf-node-types.txt`;
  - a baseline file at `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`
    recording, for each of the eleven parity tests 0212 converts, its fixture-case
    identifiers and its pre-conversion assertion count, plus the pre-change file count
    under `skills/work/scripts/test-fixtures/` — which is **68** — as a committed
    number.

  0211 and 0212 verify against these files, so a transcription with no path is a
  criterion neither sibling can check. All three paths are now named; the two that
  previously were not are the reason this bullet exists.
- Anchor the ADF node-type inventory to the bash source rather than authoring it
  free-hand. The inventory is derived from `jira-adf-render.jq` (render side),
  `jira-md-assemble.jq` and `jira-md-tokenise.awk` (assemble side). Record the
  round-trip asymmetry: the render direction accepts a strictly larger language and
  degrades to `[unsupported ADF node: …]` placeholders, while the assemble direction
  hard-rejects blockquotes, tables and nested lists with exit 41.
- Port **four** exit-code mapping tables, not two, each pairing one provider
  with one operation: Jira `create` is `_wicr_map_jira`, Jira `update` is
  `_wiur_map_jira`, Linear `update` is `_wiur_map_linear`, Linear `create` is
  `_linear_map_no_file_failure` (inside `linear-create-flow.sh`), whose name
  understates what it covers. Note `_linear_map_no_file_failure` emits 108/109 which
  `_wicr_map_linear` then re-maps, so the Linear create path is two layers.

  ⚠️ **Correction, 2026-08-17: the tables are NOT keyed by `curl` transport exit
  code.** The keys are each integration's own layered namespace — the callee flow's
  band (100-108, 110-117), the transport band propagated unchanged from
  `jira-request.sh` / `linear-graphql.sh` (11-23, 34-36), and for Linear the auth band
  (25/27/29) re-exited by `linear-graphql.sh:481-489`. A curl exit code **cannot reach
  any mapper**: `jira-request.sh:340-348` and `linear-graphql.sh:232-241` collapse
  curl's status into a boolean (`curl_ok=false`) and every transport failure becomes
  script code **21**; nothing captures `$?`. Code `34` is emitted by
  `jira-request.sh:370` for HTTP 400 and by six sites in `linear-graphql.sh`, one of
  which is an HTTP 200 body carrying `errors[]`. The numeric overlap with curl's own
  exit values is coincidence. `EXIT_CODES.md`'s assignments are correct.

  **The Jira pair does not diverge** — both mappers share a byte-identical transport
  clause `11 | 12 | 13 | 14 | 15 | 17 | 19 | 22 | 34`. **Linear diverges in both
  directions**, and only one direction is documented: `34` is pre-send/retryable on
  create and terminal on update (documented — a 200-body error may mean the mutation
  applied), while `18, 23, 25, 27, 29` run the other way with **no rationale anywhere**,
  despite all five being raised before a byte leaves the process. Port these as two
  genuinely different policies, or unify deliberately and record it; do not assume
  symmetry.

  The rule the tables encode: a failure that **provably occurred before the
  request left the client** is retryable; anything that may have reached the
  tracker is terminal. The tables are deliberately more conservative than that
  rule. Where the two disagree, **the tables win**.

### Fidelity

- Carry over the identifier-safety check the create bridge performs
  (`work-item-create-remote.sh:62-87,238-246`): reject an empty identifier, one
  carrying a control character, LF, CR or TAB anywhere, one whose first three
  characters are `---`, or one whose first non-whitespace character is `#`. `/`, `#`
  and `@` are explicitly permitted mid-token. `ExternalId::new` is `pub const fn` and
  infallible by freeze, so the type cannot carry it. An unsafe identifier is a
  `Terminal` failure, not an `Ok`.
- Bound the port's calls from inside each client crate, below its
  `RemoteTracker` impl — configured on the `reqwest` client, overridable at
  construction for tests. The bash values are **30s for both** Jira
  (`jira-request.sh:298`) and Linear GraphQL (`linear-graphql.sh:519`); the 60s figure
  belongs only to `linear-attach-flow.sh:172`, the binary PUT to the pre-signed asset
  host, which is not a port operation. Reproduce the `_WIFR_PAGE_CAP=20` pagination
  backstop and Linear's own `MAX_PAGES=20`; note both set a truncation flag and exit 0
  rather than failing, which is what routes unseen ids to `indeterminate`.
- Reproduce the per-provider projection recipes exactly. Jira: summary line then the
  description in ADF through key-sorted `jq -S`; Linear: title line then Markdown
  description verbatim. Title line, then description, with **no blank line between
  them** and a trailing newline, for both providers. What `show` and the projection
  helper place in `RemoteIssue.body` is the *un-normalised* projection; 0194's sync
  engine normalises before hashing, above the port.

  The case a JSON deserialiser will get wrong: an **absent** description
  projects as the literal token `null` for Jira (`jq -cS '… // null'`) and as an
  empty line for Linear (`// ""`). Neither is what `serde` produces naturally,
  and either wrong choice reclassifies every such item as `remotely-modified`
  on the first run after cutover.

  ⚠️ **Key-order independence is load-bearing and currently free only by accident.**
  `serde_json` without `preserve_order` backs objects with a `BTreeMap`, so
  `to_string` is key-sorted and compact, matching `jq -cS`. A client that enables
  `preserve_order`, or that round-trips ADF through a typed struct with
  declaration-order serialisation, changes every Jira `remote_hash` silently.
  `cli/work-adapters/src/project_remote.rs` avoids the whole class by keeping the
  payload as untyped `serde_json::Value`; do the same.
- Cover the `serde_json`-versus-`jq` numeric and control-character divergence with an
  explicit assertion. The 0194 validation flagged it as uncovered and noted this child
  is *"where a live Jira payload first meets the recipe"*.
- Map `RemoteTimestamp` per 0204's contract: a tracker's stamp to
  `Reported(bytes)` verbatim, a blank or null one to `NotReported` — never
  `Reported("")`. `NotRead` is unreachable through the port.

### Enforcement

- Own the `cli/pup.ron` import rules and public-API snapshots for the two crates
  this child creates. Prefer the **`denied`-only adapter shape**
  (`cli/pup.ron:236-248`) over an `allowed_only` permit list: cargo-pup resolves
  `use a::{b, c}` to an empty module name, so a permit list forces one single-item
  `use` per line throughout the crate. Each new rule owes a probe pair in
  `tests/integration/pup/test_import_rule.py` — a violation case asserting exit,
  message substring (`"is denied"` for a deny clause) **and the rule name**, plus a
  control that carries real imports.
- Classify both crates in `tasks/public_api.py` as `_EXEMPT_MEMBERS` / `_ADAPTER`,
  following `github`'s precedent (`:50-53`). `tests/unit/tasks/test_rust.py:160-170`
  goes red the moment a crate joins `[workspace].members` unclassified.
- ⚠️ **Correction: `accelerator-work` has neither a pup rule nor a public-API
  snapshot.** `cli/pup.ron` contains no rule matching `work_cli`/`accelerator_work`,
  and `tasks/public_api.py:59` classifies `work-cli` as `_COMPOSITION_ROOT` in
  `_EXEMPT_MEMBERS`, so no snapshot file exists. The composition-root edit therefore
  requires neither to "accept" anything. What *is* pinned and must not move is
  `cli/tracker/tests/fixtures/public-api.txt`.
- Keep `tracker` dependency-free. `cli/tracker/tests/structure.rs:54-65` asserts the
  manifest declares neither `[dependencies]` nor `[dev-dependencies]`, and
  `work_domain_imports_only_permitted` justifies its `^tracker(::|$)` allowance on the
  grounds that *"Both are zero-dependency port crates, so neither edge can drag a
  transitive graph into the domain."* Any third-party type exposed in `tracker`'s
  surface also enters its pinned snapshot.
- Clear `rustls` and the chosen mock layer's transitive trees through `deny.toml`'s
  licence and advisory policy, committing any allowance needed. Record whether either
  tree carries copyleft components; if it does, 0203's attribution artefact becomes a
  release-path dependency for 0211. Note the allow-list comment already anticipates
  *"the rustls/HTTP stack will re-introduce ISC/BSD/Zlib"*, and that any copyleft must
  go in `[[licenses.exceptions]]`, **never** the blanket allow.

## Acceptance Criteria

- [ ] Both client crates implement `RemoteTracker`, and the composition root of
      `accelerator-work` binds each real client under its `work.integration`
      value — verified by a test constructing the sync engine with each real
      client, not by the crates merely compiling.
- [ ] Request construction for a provider appears only in its client crate,
      checked mechanically: no `use reqwest` or `reqwest::` outside the two
      client crates, and no string literal matching `/rest/api/`, `/rest/agile/`
      or a leading `query ` / `mutation ` GraphQL document outside them. The
      tripwire itself is tested by planting a deliberate violation and observing
      it fail.
- [ ] The four tables are transcribed verbatim into a committed fixture at the named
      path (code, provider, operation, class), and a table-driven test asserts a
      `TrackerError` class for **every row** — a row with no assertion fails the
      build. Classification is per operation, and both directions of the Linear
      divergence are covered: `34` retryable on `create` and terminal on `update`, and
      `18, 23, 25, 27, 29` the other way. The fixture records that the keys are the
      integrations' own namespaced codes, not `curl` exit values.
- [ ] **Per provider and per operation**, a table-driven test maps each of HTTP 401,
      403, 404, 429 and 5xx — and, for Linear, a `200` carrying an `errors` array and a
      `400` carrying `"code": "RATELIMITED"` — to a `TrackerError` class. A row with no
      assertion fails the build. An auth failure classified as `Retryable` on a mutating
      call must fail this criterion.
- [ ] Credentials resolve non-interactively with the bash precedence reproduced, and a
      `token_cmd` set at team level is refused with the diagnostic
      `jira-auth.sh` emits. Verified by a test, not by inspection.
- [ ] With an injected timeout T, a never-responding endpoint causes `show` to **fail**
      no earlier than T and no later than 1.35×T, and causes `fetch_all` to **return
      `Ok` with every requested id in `indeterminate`** within the same window —
      because the port requires a post-attempt transport failure to be an `Ok`, not an
      `Err`. Verified at T = 400ms and T = 1s; the previously-specified T = 200ms left
      ~70ms of headroom and would flake under parallel CI load. A unit assertion
      confirms the constructed defaults are 30s for both providers and a page cap of
      20. A paginated fixture offering 21 or more pages stops after 20 and reports the
      unseen ids as `indeterminate`, never `absent`.
- [ ] Given a `create` response whose returned identifier is empty, or carries (a) a
      control character, (b) LF, CR or TAB, (c) a leading `---`, or (d) a leading `#`
      after optional whitespace, the client returns a `Terminal` failure. Asserted at
      the client boundary — the client writes no files, so "no value is written to
      frontmatter" is not an observation point available here.
- [ ] **Offline projection fidelity**, against the named records rather than the whole
      directory: for each of the **three** `work-item-project-remote/case-*` records
      (`case-jira`, `case-jira-reordered`, `case-linear`) the client's projection is
      **byte-identical** to the committed `expected.txt`; and for each of the **four**
      remote-recipe `work-item-sync-baseline/case-*` records (`case-jira-adf`,
      `case-jira-no-description`, `case-linear-empty-description`,
      `case-linear-markdown`) the sha256 of the client's projection **after
      `work::normalise`** equals the committed `remote_hash`. The two families carry
      different artefacts and admit different assertions; the directory's other ~54
      files carry no projection at all. Runs with no network target.
- [ ] A test asserts the Jira projection is invariant under input key reordering, by
      projecting `case-jira` and `case-jira-reordered` and comparing them to each other
      — so a client enabling `preserve_order` or routing ADF through a typed struct
      fails rather than silently rehashing the corpus.
- [ ] ADF↔markdown conversion has a committed fixture set exercising both
      directions and covering every entry in the node-type inventory this child
      records, with the inventory derived from the three bash conversion assets rather
      than authored free-hand; JQL composition and Linear's GraphQL document
      construction are each pinned by request-body assertions.
- [ ] **Offline**: given a `RemoteIssue` fixture with an **absent** description key,
      the Jira projection is byte-identical to a committed golden ending in the
      literal `null`, and given an empty-string description the Linear projection ends
      in an empty line — both with no blank line before the description and a trailing
      newline. This does not depend on any network target.
- [ ] Given a response whose timestamp field is absent, `null` or an empty
      string, each client returns `RemoteTimestamp::NotReported` — never
      `Reported("")`; given a populated stamp it returns `Reported` with the
      bytes unaltered, including Jira's colon-less `+0000` offset.
- [ ] `ContractSubject` is implemented for both real clients and `mise run
      test:integration:tracker-contract` exercises all four port operations —
      `create`, `update`, `show` and `fetch_all` — against each, plus the two
      cross-operation obligations: `fetch_all`'s partition totality (**every requested
      id lands in exactly one of `found`, `absent` or `indeterminate`** — three
      vectors, since collapsing `absent` into `indeterminate` is what makes a sync
      delete a live issue) and the rule that a read operation never returns a
      `Terminal` failure.
- [ ] The contract run has an enforcing route, not merely a recorded one:
      **either** a committed CI workflow whose job is required on the pull
      request, **or** a committed evidence file at a named path holding the
      harness output for both providers, dated no earlier than the final client
      commit. Recording that no gate exists does not satisfy this.
- [ ] The default `cargo test` / `cargo nextest run` invocation makes no network
      call, verified by running the default suite green in a network-disabled
      environment rather than by reading the filter expression. Each client's harness
      is named `tests/contract.rs`.
- [ ] The three oracle transcriptions are committed **as part of this child's
      change**, each at its named path. Verifiable by inspecting this change alone.
- [ ] `deny:check` is green with any needed allowance committed, and the copyleft
      question is answered by the committed verbatim output of a named
      reproducible command (a `cargo deny list` or `cargo about` licence listing
      over the new trees), not by a summary judgement — so a verifier can re-run it.
      The answer is recorded in 0171's `## Decisions`, and if it is positive, 0203 is
      added to 0211's `blocked_by` per 0211's trigger.
- [ ] Both client crates carry `cli/pup.ron` import rules with probe pairs, and are
      classified in `tasks/public_api.py`. `cli/tracker/tests/fixtures/public-api.txt`
      is unchanged, and `cli/tracker/Cargo.toml` still declares no dependencies.
- [ ] The build-versus-buy outcome, the mock-layer decision, and the Jira-v2
      consideration are each recorded in 0171's `## Decisions` with the reason —
      including that `gouqi` was evaluated and declined, and that v2 was declined
      because it would rehash the corpus.
- [ ] No Python enters `cli/`'s dev-dependencies, and `mise run` exits 0
      end-to-end at this child's merge boundary.

## Dependencies

- Blocked in practice by the **credentialed tracker target**: a scratch Jira
  project, a Linear team, API tokens, and repository secrets if 0171's secrets
  Open Question resolves to CI. Two criteria here — the contract run and its
  enforcing route — are unsatisfiable without it, so it gates *this* child's
  acceptance, not merely the eventual cutover. It has no work item of its own and
  is external account administration rather than a code change; **Toby Clemson
  owns provisioning it**, per 0171's Open Questions, and the secrets-siting answer
  decides whether a CI workflow change joins this child's scope.
- **External systems**: Jira REST and Linear GraphQL. Reachability, per-tenant
  rate limits, Linear's query-complexity cap (0.1/property + 1/object, multiplied by
  connection pagination defaulting to **50**, hard-rejected above 10,000 points — so
  always pass an explicit `first:`) and its 250-issue bulk truncation all bear on the
  contract lane; a red run is not automatically a defect in this change.
- Consumes 0204's frozen port and 0194's contract harness, both confirmed present at
  revision `5dd45e01`: `cli/tracker-test-support/src/contract.rs` exists,
  `accelerator work sync` is wired at `cli/work-cli/src/cli.rs:88`, and the corpus holds
  68 files.
- **Corpus provenance is split**, contrary to this item's earlier text:
  `work-item-project-remote/` was added by **0170**; `work-item-sync-baseline/` by
  **0194**. Both directories must survive until 0212.
- ⚠️ `work_adapters::sync::run` has **no test coverage at all** (0194 validation,
  finding 1), so this child's first acceptance criterion will be the first thing
  driving it with a non-fake. A `sync_run.rs` binary over `RecordingTracker` is cheap
  and de-risks this child; check whether it has landed before planning.
- Inherits three manual checks the 0194 validation listed as blocked until a client
  exists: `work sync` against a live tracker, `create --push` / `update --push`
  including the terminal-failure-that-succeeded shape, and the pending-push marker's
  crash-recovery path against a real interrupted create.
- Blocks 0211 and 0212: both delete bash this child must first be verified
  against. The ordering obligation is that no deletion in either sibling begins
  before this child's three transcriptions and its offline corpus criterion have
  landed — recorded here rather than inside an acceptance criterion, so this
  child's own gate can be closed by inspecting its own change.
- Parent: 0171.

## Assumptions

- The chosen mock layer can express both providers' error shapes faithfully enough to
  exercise all four tables plus the HTTP-status and GraphQL classification tables,
  including Linear's partial-success responses where a `200` carries an `errors` array.
  If it cannot, the affected cases move to the credentialed target rather than going
  uncovered. Note that **no** available library can express a never-responding or
  mid-body-dropped connection, so the timeout criterion needs the hand-rolled
  responder's stall capability whichever mock layer is chosen.
- The async-to-sync bridge, if a library is adopted, is a one-off test-layer pattern
  and does not force the production clients async. `wiremock` runs its server on its
  own thread with its own runtime, so no deadlock arises; the constraint is only that
  a blocking `reqwest` call must not execute inside an entered runtime.
- ADF remains the Jira wire format for this child. v2's plain-string alternative was
  considered and declined — see `## Build versus Buy`.

## Open Questions

- **Who owns the five port-less provider flows**, and does that make this child
  materially larger? Accepted unresolved by the review; it must be settled before
  planning commits to crate sizes.
- **Is a mock library adopted, or are the two existing hand-rolled servers extracted?**
  The timeout criterion needs the stall capability either way.
- **How does `TrackerRegistry::resolve` acquire config**, and what is the Linear
  team-id key called?
- **Is `gouqi` read, vendored, or ignored** for its `/search/jql` pagination and ADF
  node shapes?
- **Is the ADF conversion hand-built or composed** from `htmltoadf` / `adf2html` /
  `jc-adf`? Licence is not a constraint.
- Carried from 0171 and still open: the credentialed target's secrets siting, the fate
  of the three port-less bridge capabilities, and `EXIT_CODES.md` siting.

## Drafting Notes

- Enriched 2026-08-17 from
  `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`,
  measured at revision `5dd45e01`. `producer` left as `review-work-item`: it records
  where the item came from, and this pass enriched it rather than created it.
- **Six factual defects were corrected in this pass**, two of which made acceptance
  criteria unsatisfiable against the frozen port:
  1. The timeout criterion required `fetch_all` to *fail*; `cli/tracker/src/lib.rs:326-330`
     requires a post-attempt transport failure to be an `Ok` with every id
     `indeterminate`.
  2. The partition-totality criterion said "found or missing"; `FetchOutcome` has three
     vectors, and collapsing `absent` into `indeterminate` is the failure the port's
     own doc says *"makes a sync delete an issue that still exists"*.
  3. The exit-code tables are not keyed by `curl` transport exit codes — verified by
     tracing the propagation chain; curl's status is collapsed to a boolean at both
     transports.
  4. `accelerator-work` has neither a `cli/pup.ron` rule nor a public-API snapshot, so
     the criterion requiring both to "accept" the new edges had no referent.
  5. The `wiremock` requirement reversed a decision recorded in-line twice, and its
     cost profile is the opposite of what was assumed.
  6. The offline corpus criterion said "every record" in a 68-file directory; only
     three records admit byte-identity and four more admit a hash comparison after
     normalisation.
- Two review findings accepted-but-unresolved are now requirements with criteria: the
  HTTP-status/GraphQL classification and auth gap, and the ownership of the five
  port-less flows. The review's own words for the first: *"A client that misclassifies
  an auth failure as retryable passes every criterion."*
- The two unnamed oracle transcription paths are now named, closing a pass-2 major that
  the fix round had introduced.
- The timeout figures were corrected: both Jira and Linear GraphQL use `--max-time 30`;
  the 60s value belongs only to `linear-attach-flow.sh`'s binary PUT, which is not a
  port operation.
- The 1.35×T window at T = 200ms was flagged by the review as flake-prone (~70ms
  headroom). The criterion now uses T = 400ms and T = 1s.
- Build-versus-buy is newly researched and recorded as `## Build versus Buy`, following
  a question raised during the 0211 pass: the original text asserted `reqwest` + rustls
  + serde without weighing an alternative, and no `## Decisions` entry in 0171 covered
  it.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Research: `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- Review: `meta/reviews/work/0210-provider-client-crates-over-the-tracker-port-review-1.md`
- Related: 0194, 0204
- ADRs: ADR-0045, ADR-0046, ADR-0053
