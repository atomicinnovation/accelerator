---
type: codebase-research
id: "2026-08-17-0210-provider-client-crates-over-the-tracker-port"
title: "Research: Provider Client Crates over the RemoteTracker Port (0210), including build-vs-buy"
date: "2026-08-17T14:02:30+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0210"
parent: "work-item:0210"
relates_to: ["codebase-research:2026-08-17-0211-integration-binaries-and-bash-cluster-retirement"]
topic: "Implementation ground for jira-client and linear-client, and whether any existing crate simplifies the effort"
tags: [research, codebase, jira, linear, tracker, reqwest, graphql, adf, build-vs-buy]
revision: "5dd45e01ffcbd71e868ed8d84a588047cd899f84"
repository: "accelerator"
last_updated: "2026-08-17T14:02:30+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Provider Client Crates over the RemoteTracker Port (0210)

**Date**: 2026-08-17 14:02 UTC
**Author**: Toby Clemson
**Git Commit**: `5dd45e01ffcbd71e868ed8d84a588047cd899f84`
**Branch**: no bookmark (change `pmluwtrlktmo`)
**Repository**: accelerator

## Research Question

What does the codebase actually look like for work item 0210 — building `jira-client`
and `linear-client` as adapter crates over the frozen `RemoteTracker` port and wiring
them into `accelerator-work`'s composition root — and **are there existing crates that
would simplify the Jira and Linear integration effort?**

## Summary

**Build, not buy — but the reasoning differs per provider, and one existing crate is
worth reading rather than depending on.** Linear has no viable option at all. Jira has
exactly one credible candidate, `gouqi`, which is a bus-factor-1 project silent for ten
months. Every serious Rust Jira project surveyed published its own client rather than
adopting it, including six that appeared *after* `gouqi` shipped the v3 and ADF support
that would have made adoption attractive.

Separately, the research found **six factual defects in 0210**, two of which make
acceptance criteria unsatisfiable against the frozen port 0204 owns:

| # | Defect | Severity |
|---|---|---|
| 1 | AC 4 requires `fetch_all` to *fail* on transport failure; the port requires `Ok` | 🔴 unsatisfiable |
| 2 | AC 9 states partition totality as two buckets; `FetchOutcome` has three | 🔴 inverts a safety property |
| 3 | The exit-code tables are **not** keyed by curl exit codes | 🟡 misdirects the transcription |
| 4 | `accelerator-work` has neither a pup rule nor a public-API snapshot | 🟡 criterion has no referent |
| 5 | `wiremock` reverses a decision recorded in-line, twice | 🟡 needs an argument |
| 6 | AC 6's "every record" overstates the oracle by ~6× | 🟡 uncheckable as written |

The review's own Acceptance section already records two gaps carried into planning
unresolved: **no criterion covers HTTP-status or GraphQL error classification, or
auth**, and the **five port-less provider flows are owned by neither 0210 nor 0211**.
Both need a phase.

## Detailed Findings

### Build vs buy — Jira

The gate eliminates almost everything mechanically. `cli/deny.toml` bans `native-tls`,
`openssl` and `openssl-sys` outright; the licence allow-list is *pruned to exactly what
the closure carries*, so a new SPDX id is a hard failure; and `cli/Cargo.toml:61`
exact-pins `reqwest = "=0.12.28"`, so a crate wanting `^0.13` cannot unify.

| Crate | Licence | reqwest | Verdict |
|---|---|---|---|
| `gouqi` 0.20.0 | MIT | `default-features = false`, `["blocking","rustls-tls","json","multipart"]` | **only viable candidate** |
| `jira_v3_openapi` 1.6.1 | Apache-2.0 | clean, but async-only | typed transport, not a client |
| `jira_query` 1.7.4 | Apache-2.0 | ⚠️ **default features on** → native-tls → openssl | disqualified, and read-only |
| `jira-core` 2.8.3 | MIT/Apache-2.0 | clean | forces `tokio = ["full"]`, `anyhow` in public API |
| `atlassian-cli-api` 0.5.1 | MIT | ⚠️ `native-tls-vendored` | disqualified — compiles OpenSSL |
| `jira-api-v2` 1.0.1 | ⚠️ **GPL-3.0-or-later** | — | disqualified |
| `jira-issue-api` 0.7.2 | ⚠️ **Unlicense** | `^0.13` | disqualified twice |
| `goji` 0.2.4 | MIT | — | dead since 2018; `gouqi` is its fork |

I verified the two decisive dependency declarations directly against the crates.io API
rather than taking them on report:

```
jira_query  reqwest ^0.12  DEFAULT-FEATURES-ON  feats=['json']
gouqi       reqwest ^0.12  no-default-features  feats=['blocking','rustls-tls','json','multipart']
```

**`gouqi`'s case is genuinely strong on capability.** It covers issues, search,
transitions, comments, attachments; supports both v2 and v3; carries a real ADF node
model (`AdfDocument`, `AdfNode`, `AdfMark`); and — the non-obvious part — it **completed
the `/rest/api/3/search/jql` migration** that broke essentially every other Jira client
when Atlassian retired the old `/search` endpoint. Its `default = []`, so `tokio` is
opt-in and the default build is pure `reqwest::blocking`.

**The case against depending on it:** last release 2025-10-21, last commit 2025-10-20,
34 stars, single maintainer, 2 open issues, no archive notice — ten months of silence
that the commit record cannot distinguish from feature-complete stability. Its `full`
feature pulls `rsa` (RUSTSEC-2023-0071, the exact advisory `cli/Cargo.toml:138-142`
already swaps `jwt-aws-lc-rs` in to dodge) and `serde_yaml` (unmaintained, which trips
`unmaintained = "all"`). And `gouqi::Error` carries `reqwest::Error`, `url::ParseError`
and `http::StatusCode`, so an adapter boundary is needed regardless.

⚠️ **The ecosystem's own verdict is the strongest signal.** There is no community
discourse to find — no on-topic HN comments, no users.rust-lang.org thread, no 2024-2026
blog post. Every serious Rust Jira project surveyed (`jirust-cli`, `jira-commands`, `jc`,
`reposix`, `devboy-tools`, ThreatFlux) shipped its own client, six of them *after*
`gouqi` had v3 and ADF. Nobody chose it.

**Recommended shape**: hand-roll over the existing pinned `reqwest::blocking`, and read
`gouqi`'s `sync.rs` and ADF types as a reference for the two genuinely non-obvious
pieces — the `/search/jql` cursor pagination and the ADF node shapes. MIT permits
lifting shapes with attribution.

### Build vs buy — Linear

**No option exists.** Linear ships an official SDK for TypeScript only; for every other
language the docs say to point an HTTP client at `https://api.linear.app/graphql`. The
one Rust crate naming itself a Linear SDK, `linear_sdk`, is **v0.0.1, last commit
2022-10-30** — a weekend spike, four years stale against a schema Linear evolves
continuously.

For the GraphQL layer, codegen does not earn its cost at this query count:

| | `graphql_client` | `cynic` | hand-rolled |
|---|---|---|---|
| Licence | Apache-2.0 OR MIT | ⚠️ **MPL-2.0** | — |
| Committed schema | required (**1.28 MB**) | required | none |
| Build-time deps | ~58K SLoC | ~65K SLoC | none |
| Partial success | `Response{data, errors}` — clean | same, but `ErrorExtensions` defaults to discarding | yours to model |

⚠️ `cynic`'s MPL-2.0 is not a one-line allow-list edit. `cli/deny.toml:46-51` states:
*"Any copyleft / MPL / \*GPL license must be justified per-crate via
`[[licenses.exceptions]]`, **never added to this blanket allow**."* The sole existing
exception, `uluru`, earned it by being unavoidable (gix-pack's LRU cache, not
feature-gateable). A convenience crate would not clear that bar.

**The decisive argument against codegen is that it cannot catch our actual risk.**
Linear does no API versioning and, by its own deprecation policy, *"leaves a
non-functioning stub in the API to prevent breakage in queries and mutations"*. A stub
matches the committed schema, generates, type-checks, deserialises — and returns
nothing. Codegen validates against a snapshot, not production. The contract test that
*does* catch it is needed either way.

Linear constraints to encode: complexity scored at 0.1/property + 1/object, multiplied
by connection pagination with a **default of 50** and a hard 10,000-point rejection —
so always pass an explicit `first:`. Rate limits return **HTTP 400** with
`"code": "RATELIMITED"` in the body, so error classification must parse the body on 400,
not only on 200.

### Build vs buy — ADF, and the trap

Jira REST **v2 is not deprecated**, has no announced sunset, offers the same operation
set, and takes plain strings — which would eliminate the ADF problem almost entirely.
The 2024-2026 deprecations were **endpoint-scoped and hit v2 and v3 identically**
(`/rest/api/{2|3|latest}/search`), so choosing v2 buys the ADF saving at no extra
deprecation exposure. There is no conversion endpoint; the request for one
(JRACLOUD-77436) sits at Gathering Interest with 377 votes.

⚠️ **But v2 is foreclosed for 0210 by its own criterion.** Our corpus pins ADF into the
projection. Verified directly:

```
skills/work/scripts/test-fixtures/work-item-project-remote/case-jira/expected.txt
  integration=jira
  updated=2026-01-01T00:00:00.000+0000
  body=Test summary
  {"content":[{"content":[{"text":"hello","type":"text"}],"type":"paragraph"}],"type":"doc","version":1}
```

`work-item-project-remote.sh:72` runs `jq -cS '.fields.description // null'` over a v3
ADF object, and `remote_hash` is the sha256 of the normalised result. **Adopting v2
would change the body projection for every Jira item and reclassify the whole corpus as
`remotely-modified` on first sync** — exactly the failure AC 6 exists to prevent.

So v2 is a real option for a greenfield client and a trap for this one. Recording it
because "just use v2" is otherwise the obvious reading of the ADF research.

If ADF is kept, the Rust ecosystem is thin but non-empty and **entirely
permissive — no copyleft anywhere**: `htmltoadf` (106k downloads, HTML→ADF),
`adf2html` (MIT, ADF→HTML), `jc-adf` (MIT, markdown↔ADF, but 4 stars and 503
downloads — vendor rather than depend). Atlassian's `@atlaskit/adf-schema` is
Apache-2.0 and is the canonical porting source. Its published JSON schema is a
**superset spanning Jira and Confluence** — Atlassian's own docs warn *"Marks and nodes
included in the JSON schema may not be valid in this implementation."*

### The mocking decision — `wiremock` is not what 0210 assumes

0210 requires `wiremock`. Three findings complicate that.

**First, it reverses a decision recorded in-line, twice.** `cli/github/tests/common/mod.rs:5-10`:

> Ported from `cli/launcher/tests/common/mod.rs`'s `MockServer` structure … rather
> than shared as a crate, mirroring that file's own precedent for HTTP-level test
> stubbing in this workspace (**no `wiremock`/`mockito`**).

Two working std-only mock servers already exist (~6.1 KB and ~6.4 KB), one explicitly
ported from the other.

**Second, the licence surface is the opposite of what's assumed.** `cli/deny.toml` has
**no `exclude-dev`**, so dev-dependency trees are fully evaluated across all five
targets under `unmaintained = "all"`.

| | New licences | New packages | Extra tokio features |
|---|---|---|---|
| `wiremock` | **0** | ~5–8 | `macros` |
| `mockito` (defaults) | ⚠️ **MPL-2.0** via `colored` | ~5–8 | `sync`, `parking_lot` |
| `httpmock` | ⚠️ **`stringmetrics` = "non-standard"** | ~12–15 | `sync`, `macros`, `rt-multi-thread`, `signal` |

`wiremock` is the *safest* of the three on licence — the common assumption that it is
the heavy one does not survive contact with our graph. `httpmock`'s `stringmetrics` is
non-optional and its licence field cannot map to SPDX, needing a hand-written
`[[licenses.clarify]]`.

**Third, the async-bridge concern 0210 raises is a non-issue, and the real blocker is
elsewhere.** `wiremock` runs its server on its own thread with its own runtime
(`bare_server.rs`), so a blocking client cannot deadlock against it — a plain `#[test]`
with `rt.block_on(…)` around setup and the blocking call *outside* it costs about six
lines. What no library covers is **timeout testing**: none can hang the connect phase or
drop a connection mid-body. `mockito` cannot delay at all (open since 2018). Our existing
`Route::Stall(Duration)` does exactly that, with zero dependencies.

So a library would be paid for *and* the hand-rolled responder kept for the timeout
tests that AC 4 demands. The cheap move is extracting the two existing copies into one
dev-only crate.

⚠️ `wiremock` itself has had no release since 2025-08-24 (~12 months) with 32 open
issues.

### The frozen port, and two criteria that contradict it

`cli/tracker/src/lib.rs` is 343 lines, zero dependencies (enforced by
`cli/tracker/tests/structure.rs:54-65`), six items. Two of 0210's acceptance criteria
conflict with it. I verified both against the source.

**AC 4 versus `fetch_all`'s `# Errors`.** The criterion requires a never-responding
endpoint to make `show` **and `fetch_all`** *fail*. The port says:

> Once a request has been attempted, every outcome is an `Ok`. A partial retrieval puts
> its unproven ids in `indeterminate`; so does **a total transport failure, which is an
> `Ok` with every id indeterminate rather than an `Err`**.

A never-responding endpoint is post-attempt. The criterion asks for behaviour the frozen
port forbids. Fix: keep the T…1.35×T window but assert `fetch_all` *returns* an
all-indeterminate `Ok`, and reserve "fails" for `show`.

**AC 9 versus `FetchOutcome`.** The criterion says "every requested key lands in exactly
one of **found or missing**". There are three vectors:

```
cli/tracker/src/lib.rs:230   pub found: Vec<(ExternalId, RemoteTimestamp)>,
cli/tracker/src/lib.rs:233   pub absent: Vec<ExternalId>,
cli/tracker/src/lib.rs:235   pub indeterminate: Vec<ExternalId>,
```

Collapsing `absent` and `indeterminate` is precisely what the port's doc says *"makes a
sync delete an issue that still exists"*. `contract::partitions_totally` already asserts
three-way. Linear's 250-item truncation makes this live, not theoretical.

**Also worth pinning from the port**: `ExternalId::new` is `pub const fn` and infallible,
so the identifier-safety check is the client's; a client may return only `Reported` or
`NotReported` (`NotRead` is sync-engine-only); and `RemoteIssue.body` is the
*un-normalised* projection carrying the title line, so **push-then-read is not the
identity**.

### The exit-code tables are not keyed by curl exit codes

0210 states: *"The tables are keyed by `curl` transport exit code, not HTTP status — 34,
18, 23, 25, 27 and 29 are all `curl` exit values and none is a valid HTTP status."*
**Both halves are wrong.** Verified directly:

```bash
# skills/integrations/jira/scripts/jira-request.sh:340-348
curl_ok=true
printf 'user = "%s:%s"\n' … | curl --config - … || curl_ok=false
if ! $curl_ok || [ ! -s "$hdr_file" ]; then
  echo "E_REQ_CONNECT: curl failed to connect" >&2
  exit 21
fi
```

Curl's status is collapsed into a boolean; nothing anywhere captures `$?` from curl.
Every transport failure — DNS, refused, timeout, TLS — becomes script code **21**. Code
`34` is emitted by `jira-request.sh:370` for HTTP 400 and by six sites in
`linear-graphql.sh`, one of which is **an HTTP 200 body carrying `errors[]`**. The keys
are each integration's own layered namespace: the callee flow's band (100-108, 110-117),
the transport band propagated unchanged (11-23, 34-36), and for Linear the auth band
(25/27/29) re-exited by `linear-graphql.sh:481-489`. Numeric overlap with curl's exit
values is coincidence.

**The Jira pair does not diverge** — both mappers share a byte-identical transport
clause. **Linear diverges in both directions**, and only one is documented:

| Code | Linear create | Linear update | Documented? |
|---|---|---|---|
| `34` | pre-send / retryable | terminal | yes — a 200-body error may mean the mutation applied |
| `18, 23, 25, 27, 29` | post-send / terminal | retryable | ⚠️ **no** |

All five of the undocumented set are raised before a byte leaves the process, yet the
create side classes them "an issue may exist". That looks like drift. Port as two
policies or unify deliberately — do not assume symmetry.

### The composition root is a larger change than 0210 implies

`cli/work-cli/src/tracker_registry.rs:52-63` is the single substitution point — every
arm currently returns `Err`, and `jira`, `linear`, `trello`, `github-issues` are
indistinguishable (`NotAvailable` → exit 72).

⚠️ **`resolve` has no config access.**

```rust
pub trait TrackerRegistry {
    fn resolve(&self, name: &str) -> Result<Box<dyn RemoteTracker>, SelectionError>;
}
```

`ConfiguredTrackers` is a unit struct constructed inline at `main.rs:220`, `:270`,
`:377`. A Jira client needs `jira.site`, `jira.email` and a token; a Linear client needs
a team id — and **no `linear.team_id` key exists** (`cli/config/src/catalogue.rs:121-133`
has only `linear.token`/`linear.token_cmd`). So wiring requires widening the signature or
giving the registry state, rippling to three call sites. 0210 treats this as a one-line
substitution.

**0210's pup/snapshot requirement has no referent.** It says the composition-root edit
means *"its `cli/pup.ron` rules and public-API snapshot must accept them"*. Verified:

```
grep "work_cli\|accelerator_work" cli/pup.ron   → NONE
tasks/public_api.py:59                          → "work-cli": _COMPOSITION_ROOT
ls cli/work-cli/tests/fixtures/public-api.txt   → No such file
```

Neither exists. The matching acceptance criterion is unsatisfiable as written.

**Tests that break when real clients land**: `cli_sync.rs` (5 — `:72`, `:82`, `:90`,
`:127`, plus stdin), `cli_update_push.rs:114`, `cli_create_push.rs:157`,
`cli_surface.golden:134` if help text moves, and `contract.rs:249-267`'s `gated_calls()`
if new gated entry points appear.

**Naming constraint**: `cli/tracker/tests/structure.rs:67-77` asserts the workspace
manifest contains no substring `tracker-adapters`, with the comment *"provider clients
live in their own crates"*.

### The oracle is smaller than AC 6 claims

`skills/work/scripts/test-fixtures/` holds **68 files**. AC 6 requires byte-identical
projection "for every record" in it. Most of it has no projection:

| Set | Records | Assertion available |
|---|---|---|
| `work-item-project-remote/` | **3** (2 Jira, 1 Linear) | byte-identical projection |
| `work-item-sync-baseline/` | **11** (2 Jira, 2 Linear, 7 local) | sha256 **after** normalisation |
| everything else | ~54 files | none — decision tables, diff cases, normalise cases |

The two families have different shapes: `project-remote` `expected.txt` holds literal
projected bytes; `sync-baseline` `expected.json` holds a **hash of the normalised**
projection, and normalisation happens *above* the port. Byte-identity is not the
assertion available for the second family.

✅ **The absent-description case exists and is key-absent**, not `null`, not `""`:
`case-jira-no-description/remote.json` is `{"fields":{"updated":"…","summary":"No
description here"}}`. `jq -cS '… // null'` yields the four-byte literal `null`. A typed
deserialiser with `Option<Adf>` + `#[serde(default)]` gives `None` → empty string → a
different `remote_hash` → mass reclassification. `cli/work-adapters/src/project_remote.rs`
avoids it by keeping the payload as untyped `serde_json::Value`.

⚠️ **Key-order independence is load-bearing and free only by accident.** `serde_json`
without `preserve_order` backs objects with a `BTreeMap`, so `to_string` is key-sorted
and compact, matching `jq -cS`. A client enabling `preserve_order`, or round-tripping
ADF through a typed struct, fails `project_remote_parity.rs:96-118` and silently changes
every Jira `remote_hash`.

**Provenance is split**: `work-item-project-remote/` came from **0170**;
`work-item-sync-baseline/` from **0194**. 0210 attributes the whole corpus to 0194.

### The contract gate is not where 0210 says

0210 says to reuse *"whatever already excludes `tracker_contract` — the nextest filter
expression, cargo feature or `#[ignore]` convention wired in `tasks/test/integration.py`"*.
The actual mechanism is `cli/.config/nextest.toml`, keyed on **binary name**:

```toml
[profile.default]
default-filter = 'not binary(=contract)'
[profile.contract]
default-filter = 'binary(=contract)'
```

`tasks/test/integration.py:163-170` only *selects* that profile and sets
`ACCELERATOR_TRACKER_CONTRACT=1`. Consequences: each client's harness **must be named
`tests/contract.rs`** — name it `tracker_contract.rs` and it silently joins the default
run, making live API calls in `mise run`. Because the task is workspace-wide, a new
crate is picked up with **zero** task or config edits. `ACCELERATOR_TRACKER_CONTRACT` is a
second, independent gate owned by the harness, which **errors rather than skips**.

`ContractSubject` requires each client to nominate an id it will report `indeterminate`
(Linear: the 250-item truncation or complexity cap; Jira: the 50-key chunking / 20-page
cap) and an id whose `show` fails. Neither is free against a live tenant.

### Registration surface for two library crates

Both belong in `_EXEMPT_MEMBERS` as `_ADAPTER`, following `github`'s precedent
(`tasks/public_api.py:50-53`: *"Named for the forge rather than as
collaboration-adapters, but that is what it is"*). Exempt means no snapshot, no nightly
rustdoc cost. `tests/unit/tasks/test_rust.py:160-170` goes red the moment a crate joins
`[workspace].members` unclassified.

Each owes a `cli/pup.ron` rule plus a probe pair. Prefer the **`denied`-only adapter
shape** (`cli/pup.ron:236-248`) over an `allowed_only` permit list — the latter forces
one single-item `use` per line throughout the crate, because cargo-pup resolves
`use a::{b, c}` to an empty module name.

⚠️ Keeping `tracker` dependency-free is load-bearing beyond its own rule:
`work_domain_imports_only_permitted` permits `^tracker(::|$)` justified by *"Both are
zero-dependency port crates, so neither edge can drag a transitive graph into the
domain."*

**`reqwest` must be reused verbatim.** All three features are load-bearing against a
named gate — `default-features = false` against the openssl ban,
`rustls-tls-webpki-roots-no-provider` against `rustls-native-certs` (asserted absent by
`tests/integration/deny/test_launcher_feature_graph.py`), `hickory-dns` against musl
DNS. Note it does **not** include `json` today.

## Code References

- `cli/tracker/src/lib.rs:199-236` — `FetchOutcome`, three vectors, totality obligation
- `cli/tracker/src/lib.rs:319-341` — `fetch_all` `# Errors`; transport failure is `Ok`
- `cli/tracker/src/lib.rs:105-132` — `RemoteIssue.body`, the projection contract
- `cli/tracker/tests/structure.rs:67-77` — the `tracker-adapters` naming ban
- `cli/tracker-test-support/src/contract.rs:23-34` — `ContractSubject`
- `cli/tracker-test-support/src/contract.rs:249-267` — `gated_calls()`, hand-maintained
- `cli/.config/nextest.toml` — the authoritative `binary(=contract)` filter
- `cli/work-cli/src/tracker_registry.rs:52-63` — the single substitution point
- `cli/work-cli/src/exit_codes.rs:5-24` — the taxonomy; 72/73 resolve above the port
- `cli/config/src/catalogue.rs:121-133` — `EXTRA_KEYS`; no `linear.team_id`
- `cli/work-adapters/src/project_remote.rs:60-64` — the untyped-`Value` `null` trick
- `cli/work-adapters/tests/project_remote_parity.rs:96-118` — key-order independence
- `skills/work/scripts/work-item-project-remote.sh:65-93` — both projection recipes
- `skills/integrations/jira/scripts/jira-request.sh:340-348` — curl status → boolean
- `skills/work/scripts/work-item-create-remote.sh:62-87` — identifier safety
- `cli/deny.toml:46-64` — the pruned allow-list and its never-add-copyleft rule
- `cli/Cargo.toml:57-65` — the `reqwest` pin and its three load-bearing features
- `cli/github/tests/common/mod.rs:1-10` — the recorded "no wiremock/mockito" decision

## Architecture Insights

**The port pushes every hard problem into the client, deliberately.** Timeouts, retries,
backoff, identifier safety, projection fidelity, partition totality and per-operation
error classification are all stated as the implementing client's obligation, because the
port ships no logic and no dependencies. That is what makes `tracker` a seam rather than
a framework — and it is why a third-party client crate saves less than it appears to:
the adapter boundary must exist regardless, and it is where all the obligations land.

**Classification is per-operation, not per-status, and this is the repo's most
carefully-argued invariant.** It appears in the port's doc (*"a client must classify per
call rather than from one status table"*), in the 0204 research, in the bash comments,
and in the create/update asymmetry. Any client built from a single status→class table is
wrong by construction.

**Byte-fidelity beats type-fidelity throughout the sync path.** Un-normalised
projections, verbatim timestamps with no date-library round-trip, `BTreeMap` key
ordering, the literal `null`. Every one of these prefers a weaker type and a stronger
byte guarantee, because the consumer is a hash. A client written with idiomatic typed
serde would be wrong in at least four places.

**The workspace's test-double convention is hand-rolled and twice-documented.** Two
std-only mock servers, an explicit in-line rejection of `wiremock`/`mockito`, and a
`deny.toml` that evaluates dev-dependencies across five targets. 0210's mocking
requirement runs against the grain of all three.

## Historical Context

- `meta/reviews/work/0210-…-review-1.md` — APPROVE via Acceptance override, not a
  re-verdict; two REVISE passes stand. **No Correction section.** Its Acceptance records
  the findings *"accepted rather than resolved … they carry into planning rather than
  blocking it"*, and names the two latent gaps verbatim: **the non-port provider surface
  (five of eight flows) owned by neither 0210 nor 0211, and 0210 carrying no criterion
  for HTTP-status or GraphQL error classification or auth**. The reviewer's failure mode
  for the latter: *"A client that misclassifies an auth failure as retryable passes every
  criterion."*
- The same pass flagged that **two of three transcriptions still have no path** — only
  `adf-node-types.txt` is named — which is the requirement's own stated failure mode
  applied to itself.
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md` — why the port is
  synchronous (dyn-compatibility; `async-trait` forbidden by its import rule) and why
  72/73 are excluded from `TrackerError` on purpose.
- `meta/validations/2026-08-13-0194-…-validation.md` — verdict **partial**. Three items
  bear on 0210: `work_adapters::sync::run` has **no test coverage at all**, so AC 1 will
  be the first thing driving it with a non-fake; the `serde_json`-vs-`jq` numeric
  divergence remains uncovered and *"that is where a live Jira payload first meets the
  recipe"*; and `for_tracker_error` is dead and duplicated by a hand-rolled twin in
  `create.rs:346`.
- 0194 explicitly declined to lift the fixture tables behind the four bash bridge suites,
  recording that *"their behavioural content lands in 0171 as the contract harness run
  against each real client"* — a larger inheritance than 0210's four-table transcription
  list acknowledges. It also recorded an unowned hole: **no way to push an existing
  unsynced item**.
- `meta/decisions/ADR-0053` licenses the shape and names the technology ("HTTP over
  rustls"); `ADR-0045` forbids the clients prompting or exercising judgement, which
  collides with the accepted auth gap — credentials must resolve non-interactively, and
  the bash precedent (`jira-auth.sh`'s multi-source precedence and its 24/25/26 codes)
  has no home in 0210's text.

## Related Research

- `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`

## Open Questions

- **Do AC 4 and AC 9 get corrected before planning?** Both are unsatisfiable or unsafe
  against the frozen port. Neither is a judgement call.
- **Does `wiremock` stay?** It reverses a twice-recorded decision, and would be paid for
  while the hand-rolled responder is *still* needed for timeout testing. Extracting the
  two existing copies into one dev-only crate is the cheaper path.
- **How does `TrackerRegistry::resolve` get config?** Widen the signature, or give
  `ConfiguredTrackers` state. And what is the Linear team-id key called, given none
  exists?
- **Who owns the five port-less flows** — `comment`, `transition`, provider `search`,
  `attach`, `init`? Accepted unresolved by the review. Either 0210 is materially larger
  than stated, or 0211 edits crates it does not own while calling itself thin. 0194's
  unowned "push an existing unsynced item" hole is the same shape.
- **Is `gouqi` read as a reference, vendored, or ignored?** Its `/search/jql` pagination
  and ADF node model are the two pieces worth not re-deriving.
- **Does the ADF conversion get hand-built, or composed from `htmltoadf`/`adf2html`/
  `jc-adf`?** Licence is not a constraint — the whole ADF ecosystem is MIT/Apache-2.0.
- Carried from 0171 and still open: the credentialed target's secrets siting, the three
  port-less bridge capabilities, `EXIT_CODES.md` siting.

### Verification I did not do

- **No `cargo deny check licenses` was run against a scratch workspace** with `gouqi` or
  `wiremock` added. The licence closures above are reasoned from declared direct
  dependencies, not computed. That run is the only authoritative answer given the
  pruned-to-exact allow-list, and it takes minutes.
- **No `cargo tree -e features -i reqwest` was run** to confirm feature unification.
- **Jira REST v2's plain-string `description` was not tested against a live instance.**
  The documentary evidence is strong and consistent, but it is the load-bearing claim of
  the ADF section and deserves a five-minute curl before anyone acts on it.
- Fixture byte sizes were computed from content, not `stat`. File and line counts are
  exact.
