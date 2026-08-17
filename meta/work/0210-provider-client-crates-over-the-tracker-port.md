---
type: work-item
id: "0210"
title: "Provider Client Crates over the RemoteTracker Port"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0171"
blocks: ["work-item:0211", "work-item:0212"]
relates_to: ["work-item:0194", "work-item:0204"]
tags: [rust, jira, linear, integrations, reqwest, tracker]
last_updated: "2026-08-17T11:17:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0210: Provider Client Crates over the RemoteTracker Port

**Kind**: Story
**Status**: Ready
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

## Requirements

- Implement `jira-client` (Jira REST + Atlassian Document Format (ADF)↔markdown
  + auth) and `linear-client` (Linear GraphQL + auth) as adapter crates over
  `reqwest` + rustls + serde, each `impl RemoteTracker`. No native-tls, so the
  clients stay musl-static-friendly.
- Wire both clients into `accelerator-work`'s composition root under the
  `work.integration` config values 0194 exercised against fakes (`jira` and
  `linear`). This edits 0194's binary and adds crate-graph edges, so its
  `cli/pup.ron` rules and public-API snapshot must accept them.
- Test against `wiremock` (`wiremock-rs`), which runs an in-process HTTP server
  on a random local port. No Python enters `cli/`'s test dependencies; the mock
  servers retire with the bash suites in 0211.
- Bridge `wiremock`'s async API to the synchronous port once, in the test layer
  — runtime for setup, the client call outside `block_on` or via
  `spawn_blocking` — and reuse that pattern rather than solving it per test.
- **Transcribe the three oracles that later children destroy**, as part of this
  child's own change, each at a named committed path in a diffable format:
  - the four `curl` exit-code mapping tables, verbatim, into a fixture keyed by
    (code, provider, operation, class);
  - the inventory of ADF node types the bash conversion handles, as a
    one-node-type-per-line list at
    `cli/jira-client/tests/fixtures/adf-node-types.txt`;
  - a baseline file recording, for each of the eleven parity tests 0212 converts,
    its fixture-case identifiers and its pre-conversion assertion count, plus the
    pre-change file count under `skills/work/scripts/test-fixtures/` as a
    committed number.

  0211 and 0212 verify against these files, so a transcription with no path is a
  criterion neither sibling can check.
- Port **four** exit-code mapping tables, not two, each pairing one provider
  with one operation: Jira `create` is `_wicr_map_jira`, Jira `update` is
  `_wiur_map_jira`, Linear `update` is `_wiur_map_linear`, Linear `create` is
  `_linear_map_no_file_failure` (inside `linear-create-flow.sh`), whose name
  understates what it covers.

  The tables are keyed by **`curl` transport exit code**, not HTTP status — 34,
  18, 23, 25, 27 and 29 are all `curl` exit values and none is a valid HTTP
  status. Confirm the key domain against each table while porting. Within a
  provider, `create`'s and `update`'s retryable sets are **not nested in either
  direction**: for Linear, 34 is retryable on `create` and terminal on
  `update`, while 18, 23, 25, 27 and 29 run the other way. Confirm whether the
  Jira pair diverges too.

  The rule the tables encode: a failure that **provably occurred before the
  request left the client** is retryable; anything that may have reached the
  tracker is terminal. The tables are deliberately more conservative than that
  rule. Where the two disagree, **the tables win**.
- Carry over the identifier-safety check the create bridge performs
  (`work-item-create-remote.sh:62-87,238-246`): reject a returned identifier
  carrying control characters, a newline, or a leading `---` or `#`, because the
  value is written unquoted into a work item's YAML frontmatter.
  `ExternalId::new` is infallible by freeze, so the type cannot carry it, and
  the bash dispatcher that did has no counterpart once the port replaces it. An
  unsafe identifier is a `Terminal` failure, not an `Ok`.
- Bound the port's calls from inside each client crate, below its
  `RemoteTracker` impl — configured on the `reqwest` client, overridable at
  construction for tests. Reproduce `curl --max-time 30` for Jira,
  `--max-time 60` for the Linear flows, and the `_WIFR_PAGE_CAP=20` pagination
  backstop. A caller above the port cannot add them, and `/list-work-items`
  relies on the read path not hanging.
- Reproduce the per-provider projection recipes exactly, verified against the
  bash-generated baseline corpus 0194 committed at
  `skills/work/scripts/test-fixtures/` while it is still on disk — this child is
  the only window in which that oracle exists. Jira: summary line then the
  description in ADF through key-sorted `jq -S`; linear: title line then Markdown
  description verbatim. Title line, then description, with **no
  blank line between them** and a trailing newline, for both providers; Jira's
  `summary` field supplies its title line. What `show` and the projection helper
  place in `RemoteIssue.body` is the *un-normalised* projection; 0194's sync
  engine normalises before hashing, above the port.

  The case a JSON deserialiser will get wrong: an **absent** description
  projects as the literal token `null` for Jira (`jq -cS '… // null'`) and as an
  empty line for Linear (`// ""`). Neither is what `serde` produces naturally,
  and either wrong choice reclassifies every such item as `remotely-modified`
  on the first run after cutover.
- Map `RemoteTimestamp` per 0204's contract: a tracker's stamp to
  `Reported(bytes)` verbatim, a blank or null one to `NotReported` — never
  `Reported("")`. `NotRead` is unreachable through the port.
- Implement `ContractSubject` for both clients and run 0194's shared harness
  (`cli/tracker-test-support/src/contract.rs`, driven by `mise run
  test:integration:tracker-contract`) against both, so the fake 0194 verified
  against and the real clients are held to one contract.
- Gate the credentialed contract lane out of the default test run by **reusing**
  whatever already excludes `tracker_contract` — the nextest filter expression,
  cargo feature or `#[ignore]` convention wired in `tasks/test/integration.py` —
  and name it in the implementation. Do not introduce a second mechanism; two
  ways to gate the network lane leaves a later maintainer unsure which is
  authoritative.
- Own the `cli/pup.ron` import rules and public-API snapshots for the two crates
  this child creates, `jira-client` and `linear-client`. 0211 owns them for the
  two binary crates it creates. The composition-root edit additionally requires
  `accelerator-work`'s existing pup rules and public-API snapshot to accept the
  new crate-graph edges.
- Clear `wiremock-rs`, `rustls` and their transitive trees through `deny.toml`'s
  licence and advisory policy, committing any allowance needed. Record whether
  either tree carries copyleft components; if it does, 0203's attribution
  artefact becomes a release-path dependency for 0211.

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
- [ ] The four tables are transcribed verbatim into a committed fixture (code,
      provider, operation, class), and a table-driven test asserts a
      `TrackerError` class for **every row** — a row with no assertion fails the
      build. Classification is per operation, and the divergent Linear cases are
      covered: 34 retryable on `create` and terminal on `update`, 18, 23, 25, 27
      and 29 the other way.
- [ ] With an injected timeout T, a never-responding `wiremock` endpoint causes
      `show` and `fetch_all` to fail no earlier than T and no later than 1.35×T,
      verified at T = 200ms and T = 400ms; and a unit assertion confirms the
      constructed defaults are exactly 30s for Jira, 60s for Linear, and a page
      cap of 20. A paginated fixture offering 21 or more pages stops after 20.
- [ ] Given a `create` response whose returned identifier carries (a) a control
      character, (b) a newline, (c) a leading `---`, or (d) a leading `#`, the
      client returns a `Terminal` failure and no value is written to a work
      item's frontmatter.
- [ ] **Offline, whole corpus**: for every record in
      `skills/work/scripts/test-fixtures/`, the client's projection is
      byte-identical to that record's committed corpus entry, per provider,
      including Jira's key-sorted ADF ordering. Runs with no network target and
      before any deletion, so populated-description fidelity is pinned while the
      oracle still exists.
- [ ] ADF↔markdown conversion has a committed fixture set exercising both
      directions and covering every entry in the node-type inventory this child
      records; JQL composition and Linear's GraphQL document construction are each
      pinned by request-body assertions. These verify this child's client code, so
      they sit here rather than in 0211.
- [ ] **Offline**: given a `RemoteIssue` fixture with an absent description, the
      Jira projection is byte-identical to a committed golden ending in the
      literal `null`, and the Linear projection to a golden ending in an empty
      line — both with no blank line before the description and a trailing
      newline. This does not depend on any network target.
- [ ] Given a response whose timestamp field is absent, `null` or an empty
      string, each client returns `RemoteTimestamp::NotReported` — never
      `Reported("")`; given a populated stamp it returns `Reported` with the
      bytes unaltered.
- [ ] `ContractSubject` is implemented for both real clients and `mise run
      test:integration:tracker-contract` exercises all four port operations —
      `create`, `update`, `show` and `fetch_all` — against each, plus the two
      cross-operation obligations: `fetch_all`'s partition totality (every
      requested key lands in exactly one of found or missing) and the rule that
      a read operation never returns a `Terminal` failure.
- [ ] The contract run has an enforcing route, not merely a recorded one:
      **either** a committed CI workflow whose job is required on the pull
      request, **or** a committed evidence file at a named path holding the
      harness output for both providers, dated no earlier than the final client
      commit. Recording that no gate exists does not satisfy this.
- [ ] The default `cargo test` / `cargo nextest run` invocation makes no network
      call, verified by running the default suite green in a network-disabled
      environment rather than by reading the filter expression.
- [ ] The three oracle transcriptions are committed **as part of this child's
      change**, each at its named path: the four-table fixture, the ADF node-type
      inventory, and the baseline file carrying the eleven tests' fixture-case
      identifiers, their pre-conversion assertion counts, and the pre-change
      fixture file count. Verifiable by inspecting this change alone.
- [ ] `deny:check` is green with any needed allowance committed, and the copyleft
      question is answered by the committed verbatim output of a named
      reproducible command (a `cargo deny list` or `cargo about` licence listing
      over the `wiremock-rs` and `rustls` trees), not by a summary judgement — so
      a verifier can re-run it. The answer is recorded in 0171's `## Decisions`,
      and if it is positive, 0203 is added to 0211's `blocked_by` per 0211's
      trigger.
- [ ] `jira-client` and `linear-client` carry `cli/pup.ron` import rules and
      public-API snapshots, and `accelerator-work`'s existing pup rules and
      snapshot accept the new composition-root edges.
- [ ] The credentialed contract lane is gated by the **existing**
      `tracker_contract` exclusion mechanism, named in the implementation, with no
      second mechanism introduced.
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
  rate limits, Linear's query-complexity cap and its 250-issue bulk truncation
  all bear on the contract lane; a red run is not automatically a defect in this
  change.
- Consumes 0204's frozen port and 0194's contract harness. Confirm 0194's
  artefacts exist rather than trusting its status field — see 0171.
- Blocks 0211 and 0212: both delete bash this child must first be verified
  against. The ordering obligation is that no deletion in either sibling begins
  before this child's three transcriptions and its offline corpus criterion have
  landed — recorded here rather than inside an acceptance criterion, so this
  child's own gate can be closed by inspecting its own change.
- Parent: 0171.

## Assumptions

- `wiremock` can express both providers' error shapes faithfully enough to
  exercise all four tables, including Linear's partial-success GraphQL
  responses where a `200` carries an `errors` array. If it cannot, the affected
  cases move to the credentialed target rather than going uncovered.
- The async-to-sync bridge is a one-off test-layer pattern and does not force
  the production clients async.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Related: 0194, 0204
- ADRs: ADR-0045, ADR-0046, ADR-0053
