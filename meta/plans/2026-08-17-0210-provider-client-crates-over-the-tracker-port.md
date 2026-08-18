---
type: plan
id: "2026-08-17-0210-provider-client-crates-over-the-tracker-port"
title: "Provider Client Crates over the RemoteTracker Port Implementation Plan"
date: "2026-08-17T14:32:39+00:00"
author: Toby Clemson
producer: create-plan
status: in-progress
work_item_id: "work-item:0210"
parent: "work-item:0210"
derived_from: ["codebase-research:2026-08-17-0210-provider-client-crates-over-the-tracker-port"]
relates_to: ["work-item:0171", "work-item:0194", "work-item:0204", "work-item:0211", "work-item:0212", "plan-review:2026-08-17-0210-provider-client-crates-over-the-tracker-port-review-1"]
tags: [rust, jira, linear, integrations, reqwest, tracker, adf, graphql]
revision: "7fbc11853805ac90798eb0b0923855a2d3380c22"
repository: "accelerator"
last_updated: "2026-08-18T11:40:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Provider Client Crates over the RemoteTracker Port Implementation Plan

## Overview

Build `jira-client` and `linear-client` as adapter crates over the workspace's
pinned `reqwest` + rustls + serde stack, each implementing the frozen
`RemoteTracker` port, and wire both into `accelerator-work`'s composition root
so `accelerator work sync` resolves real providers. Beyond the four port
operations this plan also builds the complete provider surface — `comment`,
`transition`, `attach` and `init`'s discovery calls — so that 0211 finds no
provider request construction still living in bash.

No *skill* changes when this merges — the jira, linear and work skills all still
shell out to bash until 0211 and 0212 repoint them. The `accelerator work`
binary does change: from Phase 7 onwards `work sync`, `create --push` and
`update --push` resolve real clients and issue live network calls for `jira` and
`linear`, and the exit code for a configured-but-credential-less run moves off
72. That is a new failure surface — network, auth, rate limits, partial
fetches — on a user-invocable path, and Phase 7 verifies what a user sees on
each.

## Current State Analysis

The port is frozen and complete. `cli/tracker/src/lib.rs` is 343 lines with
zero dependencies, six items, and `cli/tracker/tests/structure.rs:54-65`
asserts the manifest declares neither `[dependencies]` nor `[dev-dependencies]`.
`cli/tracker/tests/structure.rs:67-77` additionally asserts the workspace
manifest contains no substring `tracker-adapters`.

The sync engine above it works and is tested. `cli/work-adapters/src/sync/run.rs:129`
carries `pub fn run`, and `cli/work-adapters/tests/sync_run.rs` (14.6 KB) already
drives it over `tracker_test_support::RecordingTracker`. The 0194 validation's
"no test coverage at all" finding is closed; this plan inherits a driven engine
rather than an undriven one.

Everything below the port is bash. Ten flow scripts across the two integration
namespaces implement the eight provider flows, over two shared transports
(`jira-request.sh`, 442 lines; `linear-graphql.sh`, 535 lines) that own timeouts,
a 4-attempt retry loop with jittered backoff, and per-status exit-code
classification.

The composition root is a stub. Every arm of
`cli/work-cli/src/tracker_registry.rs:52-63` returns `Err`, and `jira`, `linear`,
`trello` and `github-issues` are indistinguishable.

### Key Discoveries

**The registry needs state, not a widened signature.** At all three
`ConfiguredTrackers` construction sites — `cli/work-cli/src/main.rs:220`, `:270`
and `:377` — a `service: &dyn ConfigAccess` binding is already in scope on the
immediately preceding lines. `resolve(&self, name: &str)` keeps its signature;
`ConfiguredTrackers` gains a lifetime and a config reference. The research's
"widen the signature or give the registry state, rippling to three call sites"
resolves to the cheaper branch.

**`cli/github` is not the `reqwest` precedent.** It wraps **octocrab**
(`cli/github/src/octocrab_client.rs`). The workspace's actual `reqwest` users are
`cli/launcher` (`src/launch/outbound/resolve/fetcher.rs`, via
`reqwest = { workspace = true }`) and `cli/visualiser/server`, which declares its
**own** non-workspace entry at `cli/visualiser/server/Cargo.toml:61` carrying
`json`. A tripwire that greps for `reqwest` without an allowlist fails on both.

**The rustls licence surface is already paid for.** `reqwest`, `rustls` and
`hickory` are in the closure today via `launcher`, and `cli/deny.toml`'s allow-list
already carries `ISC`, `BSD-2-Clause`, `BSD-3-Clause` and `Zlib` — the very
licences its comment anticipates "the rustls/HTTP stack will re-introduce". The
production side of this plan adds no new SPDX id. Only `multipart`'s `mime` /
`mime_guess` and the removal of a dev-dependency mock library bear on
`deny:check`.

**`rustls-tls-webpki-roots-no-provider` installs no crypto provider.** The
workspace `reqwest` entry selects that variant deliberately, and its only current
consumer works because `cli/launcher/src/launch/outbound/tls.rs:10` calls
`rustls::crypto::ring::default_provider().install_default()` and the crate
declares `rustls = { workspace = true }`. A client that omits both fails at
handshake time — and no plain-HTTP mock can catch it, because the whole offline
harness is `TcpListener` over cleartext.

**webpki-roots is not curl's trust store.** The bash transports run `curl`,
which uses the OS trust store; the pinned `reqwest` bundles webpki-roots and
consults no host store. Users behind a TLS-intercepting proxy, or on a
self-hosted Jira with a private CA, lose a path that works today. This is a
deliberate divergence, recorded in 0171's `## Decisions`.

**The `accelerator-` prefix is reserved for dispatched binaries.** No library
crate in `cli/` carries it — libraries are `tracker`, `work-adapters`,
`tracker-test-support`, `github`. `tasks/public_api.py` further requires that a
pinned crate's directory name *is* its package name, because the snapshot path
and the `-p` argument both derive from it. New libraries therefore take bare
names, and `-p` arguments name real packages (`accelerator` for launcher).

**The config key catalogue has three more consumers than the two lists.**
`cli/launcher/tests/fixtures/dump/dump.golden` pins the dumped key set, and
`skills/config/configure/SKILL.md` documents it. Adding a key without touching
those reds the first `mise run`.

**The workspace `reqwest` entry has no `json` feature.** `cli/Cargo.toml:61-65`
carries exactly `blocking`, `rustls-tls-webpki-roots-no-provider` and
`hickory-dns`. Requests must therefore set `Content-Type` explicitly and pass a
`String` body from `serde_json::to_string`, not call `.json(&value)`. This is
also the shape that keeps key ordering under our control.

**Auth is a near-mechanical port.** `cli/collaboration-cli/src/auth.rs:43-91`
implements exactly the required precedence for GitHub — env, then config value,
then a `Level::Team` refusal, then a `Level::Personal` `token_cmd` run through
`bash -c`. `ConfigAccess::get(key, Some(Level::Team))` is the provenance
mechanism the shared-config ban needs. The `token_cmd` shell-out survives.

**`_wicr_map_linear` is not a table.** It is two arms: `108` retryable,
everything else terminal
(`skills/work/scripts/work-item-create-remote.sh:92-97`). The substance of the
Linear create policy lives one layer down in `_linear_map_no_file_failure`
(`skills/integrations/linear/scripts/linear-create-flow.sh:177-182`), which
emits `108 E_CREATE_PRE_SEND` / `109 E_CREATE_POST_SEND`.

**The mappers emit their class as an exit status.** `E_DISPATCH_RETRYABLE` is
**70** and `E_DISPATCH_TERMINAL` is **71**
(`skills/work/scripts/work-item-bridge-codes.sh:30-33`), which are the same
values as `work_cli::exit_codes::RETRYABLE` and `TERMINAL`.

**Linear emits no 403, 404, 410 or 429 at all.** Those statuses are Jira-only;
Linear returns their equivalents as HTTP 200 or 400 bodies carrying `errors[]`
(`linear-graphql.sh:269-312`). Codes 12-15, 17 and 19 are explicitly *reserved*
in Linear's `EXIT_CODES.md` for that reason. A status-only classifier cannot
reproduce Linear codes 11, 34, 35 or 36.

**The ADF renderer has two placeholder strings, not one.** Which one fires
depends on position, not node type: `[unsupported ADF node: \(.type)]` at
`jira-adf-render.jq:77` for anything in `doc.content`, and
`[unsupported ADF inline: \($node.type)]` at `:42` for anything inside a
`paragraph` / `heading` / `taskItem` content array.

**Provider `search` is not port-less.** `skills/work/scripts/work-item-fetch-remote.sh:120`
and `:161` call `jira-search-flow.sh` and `linear-search-flow.sh` to implement
`fetch_all`. Both providers' search request construction is required by the port
regardless of how the flow-ownership question resolves.

**Both `init` flows prompt interactively** (`jira-init-flow.sh:189-191`,
`linear-init-flow.sh:254`), and their caches are inputs to `search` and
`transition`: Jira's `search` reads `site.json` for `@me` and `fields.json` for
custom-field tokens; Linear's `search` and `transition` read `catalogue.json`
for state-name-to-UUID resolution.

**The contract gate is a binary-name filter.** `cli/.config/nextest.toml` sets
`default-filter = 'not binary(=contract)'` on the default profile and
`'binary(=contract)'` on the `contract` profile. Each client's harness must be
named exactly `tests/contract.rs`. Because the task is workspace-wide, a new
crate needs no `mise.toml`, task or filter edit.

**`_TEST_SUPPORT` is an existing classification.** `tasks/public_api.py` already
exempts `tracker-test-support` and `vcs-test-support` under it, establishing both
the slot and a `*-test-support` naming convention.

## Desired End State

Two provider client crates exist, each implementing `RemoteTracker` and each
carrying the complete provider surface for its tracker. `accelerator work sync
--integration jira` and `--integration linear` resolve real clients from
configuration. No provider request construction remains in Rust outside those two
crates. The three oracle transcriptions are committed, so 0211 and 0212 can begin
deleting bash.

Verified by: `mise run` green end-to-end; the default `cargo nextest run` making
no network call in a network-disabled environment; the port's contract properties
enforced offline against a mock for both providers on every run; the differential
tests agreeing with the running bash oracle; and two committed contract-run
evidence files produced against a live credentialed tenant.

## What We're NOT Doing

- **Not adopting a mock library** (D3), **not depending on `gouqi`** (D1),
  **not adopting Jira REST v2** (D2), **not using GraphQL codegen** (D4), and
  **not composing ADF conversion from a third-party crate** (D5).
- **Not putting an interactive prompt in a client crate.** ADR-0045 forbids it.
  `init`'s team and project *selection* prompt stays in the skill layer for 0211
  to wire; the crates own the discovery calls and the cache shapes.
- **Not adding a CI workflow or repository secrets.** The contract lane's
  enforcing route is a committed evidence file produced from a local credentialed
  run.
- **Not widening the workspace `reqwest` or `serde_json` features** (D6).
- **Not touching `cli/tracker`'s source or its pinned snapshot.**
  `cli/tracker/tests/fixtures/public-api.txt` must be unchanged at the end of
  this plan, and `cli/tracker/Cargo.toml` must still declare no dependencies.
- **Not repointing any skill.** Every SKILL.md still shells out to bash.
- **Not adding a `cli/pup.ron` rule or public-API snapshot for
  `accelerator-work`.** Neither exists; `tasks/public_api.py:59` classifies
  `work-cli` as `_COMPOSITION_ROOT`, so no snapshot file is generated.

## Decisions

Every cross-cutting rationale lives here once. Phases reference a decision by its
identifier rather than restating it — a rationale with several copies is a
rationale that gets updated in some of them and not the others. Phase 10 copies
this register onto 0171's `## Decisions` verbatim; it does not re-argue it.

**D1 — Build both clients, don't buy.** Linear has no viable crate. Jira has
exactly one, `gouqi`, declined: bus-factor 1, silent since 2025-10-20, `full`
pulls `rsa` (RUSTSEC-2023-0071) and `serde_yaml` (unmaintained), and its `Error`
exposes `reqwest::Error` so an adapter boundary is needed anyway. Read as a
reference for `/search/jql` cursor pagination and ADF node shapes, attributed
under MIT; nothing vendored.

**D2 — Jira REST v2 declined.** It would eliminate ADF, but the committed corpus
pins ADF into the projection, so adopting it rehashes every Jira item and
reclassifies the corpus as `remotely-modified` on first sync.

**D3 — No mock library.** `wiremock`, `mockito` and `httpmock` all rejected: none
can hang a connect phase or drop a connection mid-body, so the hand-rolled stall
responder would survive regardless and the dependency would replace nothing. The
two existing hand-rolled servers are unioned into `cli/http-test-support`.

**D4 — GraphQL codegen declined.** `cynic` is MPL-2.0; `graphql_client` needs a
1.28 MB committed schema. Neither catches Linear's non-functioning-stub
deprecation mode, where a stub matches a committed schema, generates,
type-checks and returns nothing. Only the contract test catches that.

**D5 — ADF hand-built.** The bash dialect is a bespoke subset — no text escaping,
a fixed `code → em → strong → link` pipeline, `attrs.order` always 1, seeded
`localId`, silent drops for `strike`, `underline` and a `listItem`'s second and
later children. No third-party crate reproduces it byte-for-byte.

**D6 — Cargo unifies features per resolved package version**, across the whole
selected graph, so a crate-local entry at the same pin is not an escape hatch.
Two consequences, both binding:

- `reqwest/multipart` is **not** enabled anywhere. The multipart body is
  hand-rolled over the existing byte-body path, which also keeps `mime` and
  `mime_guess` out of the closure.
- `serde_json/arbitrary_precision` is **not** enabled. Number fidelity comes from
  a local raw-token preserving re-serialiser in `remote-projection`. The feature
  would change `Value::Number`'s representation and its `untagged`/`flatten`
  behaviour in `launcher` — the binary that verifies signed artefacts — plus six
  other crates.

Applying this rule to one dependency and not the other would be arbitrary.

**D7 — `rustls-tls-webpki-roots-no-provider` installs no crypto provider.** Every
client calls `rustls::crypto::ring::default_provider().install_default()` in its
constructor and declares `rustls`, as `cli/launcher/src/launch/outbound/tls.rs:10`
already does. Omitting it fails every HTTPS request at handshake, and the offline
harness is cleartext over `TcpListener` so no mock detects it. Verified by
asserting `CryptoProvider::get_default()` is `Some` after construction — process
state, needing no server, certificate or dev-dependency.

**D8 — webpki-roots replaces curl's system trust store.** A user-visible
narrowing: corporate TLS interception and private-CA self-hosted Jira lose a path
that works today. The connect-failure diagnostic names certificate verification
distinctly so the cause is readable from the error.

**D9 — Shared policy lives in `cli/tracker-support`.** Credential resolution, the
bounded-retry policy, `TransportConfig`, the identifier-safety predicate and the
`port_body` newline adapter are common to both providers and to any third. The
pup rules forbid the clients importing *each other*, which a common downward
dependency does not do. Admission criterion: policy shared by two or more
provider clients, no transport, no provider specifics.

**D10 — Transcriptions are checked against the running bash.** A fixture
transcribed by hand and code written to satisfy it agree with each other whether
or not either agrees with the oracle. While the scripts exist, differential tests
*execute* them — Phase 2 for the five mappers, Phase 4 for the ADF pipeline. Both
assert a non-zero comparison count, fail rather than skip when bash or jq is
absent, expose their comparison function to a committed sibling test that proves
they can fail, and are deleted by 0212 with the assets they drive.

**D11 — `Unconfigured` is exit 74, not 71.** 70 and 71 (`E_DISPATCH_RETRYABLE` /
`E_DISPATCH_TERMINAL`) answer *whether a remote mutation may have applied*, and
`push_decide` plus the SKILL.md tables emit non-idempotency guidance on 71.
Unresolvable credentials provably touched nothing. 70 and 71 stay derived
exclusively from `TrackerError`.

**D12 — Credential-bearing config is a trust boundary, and `Level::Personal`
alone is not one.** `.accelerator/config.local.md` is repository-relative, so a
hostile repository can simply *track* it — `.gitignore` does not apply to an
already-tracked file — and thereby supply a `token_cmd` that
`accelerator work sync` executes through `bash -c` in a fresh clone, or a
`jira.allowed_sites` entry blessing an attacker-controlled `jira.site`.

Command-valued and allowlist-valued keys (`*.token_cmd`, `jira.allowed_sites`)
are therefore **refused when their provenance file is VCS-tracked**, with a
distinct diagnostic. The repo already has the primitive: `jira-auth.sh`'s
`_jira_is_vcs_tracked` gates the `ACCELERATOR_ALLOW_INSECURE_LOCAL` marker on
exactly this property, and `cli/vcs` provides it natively. The helper also runs
with a scrubbed environment (`PATH`, `HOME`, `TERM` only) and a defined working
directory, so the one deliberately-executed foreign code path is no more
privileged than it needs to be.

**D13 — A team-level `token_cmd` is refused, not ignored.** Both auth scripts
warn and continue; `collaboration-cli` refuses. This plan follows
`collaboration-cli`, because a silently-ignored credential source is worse than a
loud one.

**D14 — `jira.site` is a credential destination and is validated as one.** It is
where the token is sent. Refused unless absolute `https://`, no userinfo, no
query, no fragment, and a host matching the allow shape — matched **at a label
boundary**, ASCII-lowercased, punycode-normalised, default port only. Suffix
matching would accept `atlassian.net.evil.com` and `evil-atlassian.net`, which is
the same defect the Linear upload allowlist is written to avoid. `*.atlassian.net`
plus `jira.allowed_sites`, whose entries are exact hostnames with no wildcard
expansion.

**D15 — Bounding is the client's obligation, and the page cap does not provide
it.** The port states a caller cannot supply it. Each request is bounded at 30s
with 4 attempts on 429/5xx only; a transport failure resolves on the first
attempt with no retry, matching bash. An operation-level deadline bounds the
whole call, because 20 pages — multiplied again by Jira's 50-id chunks — puts a
degraded tracker in the tens of minutes. Deadline expiry degrades exactly as a
cap-hit does: truncation flagged, `Ok`, unseen ids `indeterminate`.

**D16 — Timing assertions are asymmetric.** Tight lower bound (the call must not
return before T — the property carrying signal), generous 3×T upper bound, and
the error variant asserted. A 1.35×T bound leaves 140ms of slack at T = 400ms,
inside scheduler jitter on a loaded runner, and this repo has a documented flake
history. Retry timing is asserted as *data* through an injected `Sleeper` and
seeded `Jitter`, never by wall clock.

## Implementation Approach

Ten phases, with Phase 6 split into two independently mergeable halves (6a, 6b)
so Linear's classification — the subtlest logic in the plan — is reviewed at the
same granularity as Jira's. The first two carry no client code, which front-loads
the transcription obligation so 0211 and 0212 unblock as early as possible. Each
phase is independently mergeable: a phase either refactors tests with no
production change, or adds a registered but as-yet-unresolved library, or wires
something whose dependencies already landed.

Test-driven throughout. Every classification table, every conversion direction
and every projection recipe is driven from a committed fixture, and the fixture
is written before the code that satisfies it. Three tests are themselves tested
by planting a deliberate violation: the import tripwire, the pup probe pairs, and
the row-coverage guard on each table-driven test.

The transcriptions are checked against the running bash (D10), and shared policy
is extracted into `cli/tracker-support` (D9).

Ordering constraint on the siblings, stated per bash asset rather than per phase
so 0211 cannot legitimately delete an oracle whose Rust counterpart has not
landed:

| Bash asset | Blocked until |
|---|---|
| `jira-auth.sh`, `jira-auth-cli.sh` | Phase 3 |
| `jira-adf-render.jq`, `jira-md-assemble.jq`, `jira-md-tokenise.awk` | Phase 4 |
| `jira-adf-to-md.sh`, `jira-md-to-adf.sh` | Phase 4 |
| `jira-search-flow.sh`, `jira-show-flow.sh`, `jira-jql.sh`, `jira-jql-cli.sh` | Phase 5 |
| `jira-create-flow.sh`, `jira-update-flow.sh`, `jira-emit-key.sh` | Phase 5 |
| `linear-auth.sh`, `linear-auth-cli.sh` | Phase 6a |
| `work-item-{create,update}-remote.sh`, `linear-create-flow.sh` | Phase 6a |
| `work-item-bridge-codes.sh` | Phase 6a |
| `linear-graphql.sh`, `linear-search-flow.sh`, `linear-show-flow.sh` | Phase 6b |
| `linear-update-flow.sh` | Phase 6b |
| `work-item-fetch-remote.sh`, `work-item-project-remote.sh` | Phases 5 and 6b |
| `jira-request.sh`, `jira-common.sh` | Phase 8 |
| `jira-{comment,transition,attach,init}-flow.sh` | Phase 8 |
| `jira-fields.sh`, `jira-custom-fields.sh`, `jira-resolve-fields.sh`, `jira-render-adf-fields.sh`, `jira-body-input.sh` | Phase 8 |
| `linear-{comment,transition,attach,init}-flow.sh`, `linear-common.sh` | Phase 9 |

Derived from the full inventory of `skills/integrations/*/scripts/` and
`skills/work/scripts/`, not from the assets this plan happens to name in prose —
an earlier draft omitted `jira-adf-to-md.sh` (which Phase 4's own differential
invokes), both `show` flows, `linear-create-flow.sh` (home of the fifth mapper)
and `work-item-bridge-codes.sh` (sourced by all four others).

Three rows are later than a first reading suggests. `jira-request.sh` is not free
after Phase 5 — Phase 8's attach transcribes its `X-Atlassian-Token` handling at
`:315-320`. The mappers are not free after Phase 2, because the classifiers that
consume the transcription land in Phases 3 and 6a. And each `test-*.sh` beside a
listed asset goes with it, not before it.

**Every asset a differential test executes is additionally blocked until 0212
deletes that test**, since the tests invoke the scripts directly and would red
the suite the moment the script vanished.

This table is reproduced in 0211's deletion gate so the sibling can check it off.

---

## Implementation Progress

Phases 1 to 6b are implemented and committed; each landed with `mise run` green
end to end at the time of its commit. Phase 7 is next.

| Phase | Status | Commit | Tests added |
|---|---|---|---|
| 1 — HTTP test-support crate | done | `bf7e7192` | 10 in `http-test-support` |
| 2 — Shared crates and transcriptions | done | `7bb170ed` | 37 in `tracker-support`, 2 guards |
| 3 — `jira-client` foundation | done | `9dd6aff2` | 38 |
| 4 — `jira-client` ADF conversion | done | `ee93deec` | 30 + 56 fixture cases |
| 5 — `jira-client` `impl RemoteTracker` | done | `8fe8521a` | 110 total, 26 in `remote-projection` |
| 6a — `linear-client` foundation | done | `7adda8a3` | 34 |
| 6b — `linear-client` `impl RemoteTracker` | done | `caa75991` | 67 total |
| 7 — Composition root | not started | | |
| 8 — Jira provider surface | not started | | |
| 9 — Linear provider surface | not started | | |
| 10 — Enforcement close-out | not started | | |

One commit sits outside the phase sequence: `91129dfb` takes
`test:integration:tracker-contract` out of the `test:integration` roll-up and
out of `default`. Phase 5 created the first `tests/contract.rs`, and until then
`binary(=contract)` selected nothing, so the lane passed trivially inside the
local CI mirror; afterwards a bare `mise run` needed a live Jira tenant. The
lane is now opt-in, recorded in `_NOT_IN_INTEGRATION_ROLLUP` with a guard
asserting it is unreachable from the transitive closure of `default`, `test` and
`check`, and the harness fails rather than skips when no tenant is configured.

### Deviations from the plan as written

Each is a place the implementation diverged from the plan's text, with the
reason. Nothing here was a preference; every item is either a plan error the
running oracle corrected, or a constraint the plan did not know about.

**Corrections the oracle made to the plan's transcription.** Four in the ADF
inventory alone, all found by executing `jq`/`awk` rather than reading it:

- There are **three** render-abort conditions, not two. A `bulletList`,
  `orderedList` or `taskList` with no `content` key aborts as well, because
  those arms read `.content` with no `// []` fallback.
- `a*b` yields three text nodes whose **middle one is an em-marked empty
  string**, not a literal asterisk: the catch-all matches the single `*`, and
  stripping one delimiter from each end of a one-character token leaves nothing.
- `[](https://x)` is **dropped entirely** — neither the link nor its brackets
  survive — because the assembler's capture requires a non-empty label.
- A nested list as a `listItem`'s **first** child renders one inline placeholder
  per `listItem`; only in the second-and-later position is it silently dropped.

Two more in the JQL surface, found by running `jql_compose`: the bash emits
`IN`/`NOT IN` **uppercase**, and `text ~ "…"` in **double** quotes with `\`
escaped before `"`. Multi-value quoting is single quotes with interior quotes
doubled, not the double-quoted form the plan described.

**`jira.site` accepts two shapes.** D14 describes an absolute `https://` URL,
but the bash stores a bare Cloud subdomain and builds
`https://<site>.atlassian.net` (`jira-request.sh:240`). Refusing the subdomain
form would break every existing configuration at Phase 7, so both are accepted
and both validated; only the URL form can reach `jira.allowed_sites`.

**`TransportConfig` carries no `base: Url`.** A `Url` reaches `tracker-support`
only through `reqwest`, which its own pup rule bars. The bounds live there; each
client's transport takes its base URL directly.

**Linear team scope needs the team key, not the UUID.** "An id outside the
configured team is indeterminate" is decidable only from the `<TEAM_KEY>-`
prefix, and `linear.team_id` is the UUID alone — only `catalogue.json` carries
the key. `LinearClient` therefore holds `team_key: Option<String>`, and when it
is `None` no absence is provable and every unfound id is indeterminate, which
is the port's own instruction for an implementation that cannot distinguish the
two.

**`port_body` must not append to a projection that already ends in a newline.**
Linear's empty-string description projects as an empty line, so the projection
*is* `"<title>\n"` — exactly the bytes the committed `remote_hash` covers.
`work::normalise::trim_lines` pops trailing blank lines, so the collapse is
hash-neutral in every case, which is what has been masking the difference.

**The assemble direction is compared canonically, not byte-for-byte.** `jq`
emits object keys in insertion order and `serde_json` emits them sorted, and D6
forbids `preserve_order`; both sides pass through one serialiser. The render
direction *is* byte-identical. Render-abort cases assert the class and the
oracle's exit status rather than jq's message, which embeds an input line number
and exits 5 rather than 40; the four assemble rejections do assert exact codes
and message text.

**Three accepted `jq`-versus-`serde_json` divergences**, listed in
`cli/remote-projection/tests/fixtures/adversarial-scalars.txt` with rationale:
jq 1.7.1 re-renders exponent notation through its decimal library (`1e999` →
`1E+999`, `1e-5` → `0.00001`) and escapes U+007F where `serde_json` emits it
raw. The other 21 rows match byte for byte. Jira payloads carry no exponent
notation, so a real body cannot reach the divergence.

**The baseline's `(6, 6, 15, 27, totalling 68)` figures do not reconcile** —
they sum to 54, and the corpus holds 3/3/5/12 case entries per directory. The
committed baseline records the case-name sets and the guard derives counts from
them, as the plan says the guard should; the numbers themselves are not
committed.

**Additions to `http-test-support` the plan's union did not anticipate.**
`Route::Headers`, because a mock had no way to send a `Retry-After` and the
honoured-as-a-duration property was otherwise unassertable; and
`Route::Sequence`, because Linear posts every operation to `/graphql` and a
`(method, path)` key cannot tell a create from the `show` that follows it.

**Registration landed in Phase 3 and 6a rather than Phase 5 §8.** A workspace
member must be classified in `tasks/public_api.py` the moment it exists, or the
coverage guard reds. Phase 5 §8 had nothing left to do.

**Narrower dependency sets than the plan listed.** Neither client takes
`kernel` (nothing routes through it) and neither takes `serde` (only
`serde_json`). `ClientError` has no `Unclassified` variant — an unclassified
status is a `TrackerError`, not a client error — and gained `TlsUnavailable`,
`AllowlistFromSharedConfig` and `ConfigUnreadable`.

**Test doubles are duplicated per client, deliberately.** Sharing
`RecordingSleeper`/`NoJitter` would mean widening `tracker-test-support`'s
std-only pup rule to admit `tracker_support`. These are test doubles rather than
the policy D9 exists to keep from drifting.

### Outstanding

- **One automated criterion is deliberately unticked** (Phase 5): the
  behavioural `cargo nextest list --message-format json` assertion. It costs a
  full workspace test-binary build — over ten minutes cold, ~56s warm per
  invocation — which would blunt either the default suite or a 0.77s
  integration lane. What holds the property instead: the exact-match filter is
  pinned by `tests/unit/tasks/test_nextest_filter.py`, each `contract` binary
  fails closed on its own `ACCELERATOR_TRACKER_CONTRACT` gate, and
  `contract_offline` is observably selected by the default profile.
- **Every manual-verification item in Phases 5 and 6b remains open.** All of
  them need a credentialed tenant, which this machine does not have.
- Phases 7 to 10 are unstarted. Phase 10's `## Decisions` copy onto 0171 should
  carry this deviation list alongside the D1-D16 register.

### A note on the local CI mirror

Two full `mise run` invocations during Phases 6a and 6b failed on
**pre-existing, load-sensitive flakes in code this plan does not touch**, and
both lanes pass standalone:

- `test:integration:integrations` → `jira_with_lock` test (a), where the lock
  directory is removed between `mkdir` succeeding and the holder writing
  `holder.pid`/`holder.start` (`jira-common.sh:157-170`). It also failed on a
  tree that predated `linear-client` entirely.
- `test:integration:dev` → `test_detach_readiness_and_log_routing`, where
  circusd writes no pidfile under load.

Neither is caused by a provider client; a new crate only makes the machine
busier, which shifts the timing. Treat a single full-run failure in either lane
as suspect and re-run that lane alone before investigating the change under
review.

---

## Phase 1: HTTP Test-Support Crate

### Overview

Union `cli/launcher/tests/common/mod.rs` and `cli/github/tests/common/mod.rs`
into one dev-only crate, adding the request-body capture that three acceptance
criteria need. The two servers are complementary rather than duplicated:
launcher's is path-keyed and has `Route::Stall(Duration)`; github's is
(method, path)-keyed and captures `Authorization` but discards bodies.

### Changes Required

#### 1. The crate

**File**: `cli/http-test-support/Cargo.toml`
**Changes**: New library crate, no dependencies beyond `std`.

```toml
[package]
name = "http-test-support"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[lib]
name = "http_test_support"
path = "src/lib.rs"

[lints]
workspace = true
```

The package name matches the directory, per the invariant `tasks/public_api.py`
relies on, and carries no `accelerator-` prefix — that prefix belongs to
dispatched binaries. Every new manifest in this plan inherits `[lints] workspace
= true` and declares intra-workspace edges through `[workspace.dependencies]`.

**File**: `cli/http-test-support/src/lib.rs`
**Changes**: The unioned server. Routes keyed on `(method, path)`; the path is
taken before `?` as github's does, with the query string retained separately so a
test can assert on it.

```rust
#[derive(Clone)]
pub enum Route {
    Json { status: u16, body: String },
    Bytes { status: u16, body: Vec<u8> },
    Status(u16),
    Redirect { status: u16, location: String },
    FlakyThenOk { fail_times: usize, body: Vec<u8> },
    Stall(Duration),
}

pub struct RequestKey { pub method: String, pub path: String }

pub struct MockServer { /* port, shared */ }

impl MockServer {
    pub fn start() -> Self;
    pub fn base_url(&self) -> String;
    pub fn route(&self, key: RequestKey, route: Route);
    pub fn hits(&self, key: &RequestKey) -> usize;
    pub fn last_body(&self, key: &RequestKey) -> Option<Vec<u8>>;
    pub fn last_query(&self, key: &RequestKey) -> Option<String>;
    pub fn last_header(&self, key: &RequestKey, name: &str) -> Option<String>;
}
```

`last_header` is keyed per route rather than server-global, because Phase 9's
three-step upload sends three requests to one server and must assert that step 2
carries **no** `Authorization` while step 3 does — an assertion a global header
map cannot express.

The handler records method, path, query, headers and body under a mutex
**before** writing any response bytes, and every accessor takes the same lock.
Recording after the flush would let a test observe stale state immediately after
a client call, which is exactly the shape the retry-count assertions take.

`Route::Stall` keeps launcher's exact behaviour — write
`HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n`, flush,
sleep, return without a body — because that is what makes the client's body read
block, and no mock library can express it.

Header capture generalises github's `last_authorization` to a lowercased map, so
the Jira client's `X-Atlassian-Token: no-check` and the Linear upload's echoed
`x-amz-*` set are both assertable.

#### 2. Registration

**File**: `cli/Cargo.toml`, `cli/Cargo.lock`
**Changes**: Add `"http-test-support"` to `[workspace].members`. Consuming crates
depend on it by path, as they do on every other local crate;
`[workspace.dependencies]` stays third-party-only.

⚠️ **Sync the lockfile in the same commit**, with
`cargo metadata --manifest-path cli/Cargo.toml --format-version 1` — the minimal
update, never `cargo generate-lockfile`. `tasks/README.md` makes this the first
obligation when registering a member, because clippy runs `--locked` and an
unsynced lockfile surfaces as a clippy failure pointing nowhere near the cause.
`cli/Cargo.lock` also carries a per-member version copy that the version-coherence
check depends on. **Every phase in this plan that adds a member — 1, 2, 3 and
6a — carries the same step**, with a criterion that the lockfile holds an entry
for the new package at the workspace version.

**File**: `tasks/public_api.py`
**Changes**: Add `"http-test-support": _TEST_SUPPORT` to `_EXEMPT_MEMBERS`,
beside `tracker-test-support` and `vcs-test-support`.

**File**: `cli/pup.ron`
**Changes**: A `denied`-only rule mirroring
`tracker_test_support_imports_only_permitted`'s intent — this crate must not
reach for `reqwest` or any HTTP client, because its whole value is being
std-only.

```ron
Module((
    name: "http_test_support_is_std_only",
    matches: Module("^http_test_support($|::)"),
    rules: [
        RestrictImports(
            allowed_only: None,
            denied: Some([
                "^reqwest(::|$)",
                "^hyper(::|$)",
                "^tokio(::|$)",
            ]),
            severity: Error,
        ),
    ],
)),
```

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: A probe pair for the new rule — a violation case importing
`reqwest::Client` asserting a non-zero exit, the substring `is denied`, and the
rule name `http_test_support_is_std_only`; plus a control carrying real
std-only imports.

#### 3. Refactor the two existing suites

**File**: `cli/launcher/tests/common/mod.rs`, `cli/github/tests/common/mod.rs`
**Changes**: Delete both; replace each crate's `mod common;` usage with the new
dev-dependency. Launcher's call sites gain a method, and each registration must
name the method the client actually sends — launcher's server was path-keyed, so
a registration that silently assumes `GET` where the client sends `HEAD` would
now 404 while the assertion could still read green. Github's
`last_authorization()` becomes `last_header(&key, "authorization")`.
`cli/github/tests/common/mod.rs`'s header comment recorded the "no
`wiremock`/`mockito`" decision — carry that reasoning into the new crate's module
doc so it is not lost with the file.

#### 4. Self-tests for the semantics that changed

**File**: `cli/http-test-support/tests/`
**Changes**: The union is the foundation for every client test in Phases 3-9, so
the behaviours that differ from either original are pinned directly rather than
inferred from "the existing suites still pass":

- an unmatched `(method, path)` returns a distinguishable status a test can
  assert on, so a mis-keyed registration fails loudly rather than silently
- `hits` counts exact-key matches only
- `last_body`, `last_query` and `last_header` return `None` — never a stale
  value — for a key never requested
- `Route::Stall` writes its headers, flushes, then blocks without a body
- a recorded request is visible to the accessors the instant the client call
  returns

### Success Criteria

#### Automated Verification

- [x] The new crate builds: `mise run cli:check`
- [x] Launcher's resolution suite passes unchanged in behaviour:
      `cd cli && cargo nextest run -p accelerator`
- [x] Github's octocrab suite passes unchanged:
      `cd cli && cargo nextest run -p github`
- [x] `last_body` returns the exact bytes sent, asserted by a test in the new
      crate posting a known body
- [x] An unmatched key returns the distinguishable status; `hits` counts exact
      matches only; the three accessors return `None` for an unrequested key
- [x] `Route::Stall` still causes a client read to block, asserted by the
      launcher timeout test that already covers it
- [x] The pup probe pair passes:
      `mise run test:integration:pup`
- [x] `tests/unit/tasks/test_rust.py` is green with the new member classified:
      `mise run test:unit:build-system`
- [x] `deny:check` green — no new dependency was added:
      `mise run lint:cli:deny:check`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [x] The three copies of the mock server are now one; `rg -l "TcpListener::bind"
      cli/*/tests` returns only the new crate

---

## Phase 2: Shared Crates and Oracle Transcriptions

### Overview

Extract the projection recipe and the shared client policy into two crates the
clients can depend on, commit two of the three oracle transcriptions with a
differential test that runs the bash they transcribe, add the `linear.team_id`
config key, and unify the duplicated `for_tracker_error`. No client code; every
change is to an existing, already-tested surface.

### Changes Required

#### 1. The projection crate

**File**: `cli/remote-projection/Cargo.toml`, `cli/remote-projection/src/lib.rs`
**Changes**: Move `cli/work-adapters/src/project_remote.rs` verbatim, including
its `canonicalise` doc explaining why `serde_json`'s `BTreeMap` backing already
matches `jq -cS`. Dependencies: `serde_json` only.

The moved surface is unchanged:

```rust
pub enum Integration { Jira, Linear }
pub enum Op { Updated, Body }
pub fn parse_integration(value: &str) -> Option<Integration>;
pub fn project(integration: Integration, op: Op, remote_json: &Value) -> String;
```

⚠️ `project` returns **no trailing newline** — `format!("{summary}\n{}", …)` —
where the bash `printf '%s\n%s\n'` emits one, and where the port doc
(`cli/tracker/src/lib.rs:112-114`) requires `RemoteIssue.body` to carry one. The
asymmetry is pre-existing and load-bearing for the current parity tests, so
`project` keeps its shape and **the client appends the newline** when populating
`RemoteIssue.body`, which is what makes the field honour the port contract.

⚠️ `work-item-project-remote/case-*/expected.txt` is **not** a raw body file —
it is a keyed metadata file (`integration=`, `updated=`, `body=<first line>`,
then the canonicalised description), and
`cli/work-adapters/tests/project_remote_parity.rs:47-55` reconstructs the
expected body line-wise as `format!("{body_first}\n{body_second}")`, with no
trailing newline. Comparing `project(...) + "\n"` against that value fails.

The obligation is therefore two distinct assertions, not one:

- the moved parity test compares `project(...)` — no newline — against the
  fixture's line-reconstructed body, unchanged from today
- a separate client-level test asserts `RemoteIssue.body == project(...) + "\n"`,
  which is the port contract

The damage from getting this wrong is currently masked
because `work::normalise::trim_lines` re-emits a newline per line — a future
consumer hashing `body` unnormalised would reclassify every synced item, so the
obligation is pinned here rather than relied upon downstream.

The package name is `remote-projection`, matching its directory — the invariant
`tasks/public_api.py` relies on for the snapshot path and the `-p` argument.

**File**: `cli/work-adapters/src/lib.rs`, `cli/work-adapters/Cargo.toml`
**Changes**: Repoint the callers at the new crate and add the dependency. No
re-export survives — two reachable names for one recipe leaves the next
contributor unable to tell which is canonical, and the `remote-projection`
snapshot would then not describe the surface actually in use.
`cli/work-adapters/tests/project_remote_parity.rs` moves to the new crate so the
tests sit with the code they exercise; the key-order-independence assertion runs
there.

**File**: `cli/pup.ron`
**Changes**: A `denied`-only rule for the new crate; it must not reach `std::process`
or any HTTP client, being pure JSON field extraction.

**File**: `tasks/public_api.py`
**Changes**: Classify `remote-projection`. It is a genuine shared library rather
than an adapter or a composition root, so it takes a **snapshot** rather than an
exemption.

⚠️ A snapshot needs a `_PINNED_CRATES` entry, not merely a committed file.
`check()` and `update()` both iterate that tuple, and
`tests/unit/tasks/test_rust.py` fails on any member in neither collection — so
adding nothing to either list reds `test:unit:build-system` while the committed
snapshot is never read. Add `"remote-projection"` (and `"tracker-support"`) to
`_PINNED_CRATES` alphabetically beside `migrate`, `store` and `tracker`, commit
`cli/remote-projection/tests/fixtures/public-api.txt` generated by
`mise run public-api:update`, and accept the nightly-rustdoc cost that pinning
entails.

#### 2. The shared client-policy crate

**File**: `cli/tracker-support/Cargo.toml`, `cli/tracker-support/src/`
**Changes**: New library carrying the three policies both providers share, so a
change to any of them is one edit rather than two that can drift silently. The
pup rules forbid `jira_client` importing `linear_client` and vice versa; a common
downward dependency does not violate that and is added to neither denied list.

```rust
pub struct TokenKeys {
    pub env: &'static str,
    pub env_command: &'static str,
    pub value: Key,
    pub command: Key,
}

pub fn resolve_token(
    config: &dyn ConfigAccess,
    keys: TokenKeys,
) -> Result<String, CredentialError>;

pub struct RetryPolicy {
    pub max_attempts: usize,
    pub cap: Duration,
    pub jitter: f64,
}

impl RetryPolicy {
    pub fn delay_for(
        &self,
        attempt: usize,
        retry_after: Option<Duration>,
        jitter: &mut dyn Jitter,
    ) -> Option<Duration>;
}

pub fn identifier_is_safe(candidate: &str) -> Result<(), IdentifierRefusal>;
```

`delay_for` is per attempt, returning `None` once attempts are exhausted, because
`Retry-After` arrives on each individual response — attempt 2 and attempt 3 can
carry different values or none. A whole-sequence API would force the caller to
recompute and discard, or to pin the first response's hint across every
subsequent backoff, and the retry loop would then grow its own arithmetic beside
the shared policy. Tests still assert the full sequence by folding over attempts.

`resolve_token` carries the five-rung ladder transcribed in Phase 3 §2 —
two environment sources, then `config.local.md`'s `token` and `token_cmd` behind
the mode-0600 gate, then `config.md`'s `token` only when the local file is
absent — parameterised so each provider supplies its own keys. The config-backed
keys are typed `config::Key` values constructed once at the client boundary,
because `ConfigAccess::get` takes `&Key` and `Key::parse` is fallible; parsing
inside the credential path would need an `unwrap` the workspace lints forbid.
`CredentialError`'s variants mirror the transcribed bash conditions
(`NoToken`, `TokenCmdFailed`, `TokenCmdFromSharedConfig`) so a caller can tell
them apart, and it implements `std::error::Error` with a working `source()`.

**The VCS-tracked-provenance refusal (D12) lives here**, because it must apply to
every command-valued and allowlist-valued key in both providers. `resolve_token`
takes a `Provenance` port — satisfied by `cli/vcs` in production — and refuses a
`token_cmd` whose resolved provenance file is VCS-tracked, with a distinct
diagnostic naming the file and the reason. `jira.allowed_sites` is held to the
same rule at its own resolution site.

The bash has the primitive but not the rule: `jira-auth.sh`'s
`_jira_is_vcs_tracked` already gates the `ACCELERATOR_ALLOW_INSECURE_LOCAL`
override on exactly this property, so the check is a port of an existing idea to
a place it was not applied. This is a deliberate divergence from bash, recorded
in D12 and on 0171.

Four hardening obligations the bash never carried, each with its own test:

- `token_cmd` runs under an explicit wall-clock timeout and an output-size cap,
  so a hanging or unbounded credential helper cannot stall or exhaust a sync
- only the trailing newline is trimmed from its output
- the helper runs with a **scrubbed environment** (`PATH`, `HOME`, `TERM` only)
  and a defined working directory, so the one deliberately-executed foreign code
  path cannot read the other providers' tokens out of its own environment
- `CredentialError` implements a **redacting** `Debug`, the helper's stdout is
  never folded into any error, and the command is reported by config key name
  only — a helper that prints a secret and exits non-zero must not leak it into
  CI logs. `Credentials` is defined per client, so each of Phases 3 and 6a
  carries the same obligation for its own struct; `tracker-support` owns a
  `Secret` newtype with the redacting `Debug` that both embed

`identifier_is_safe` is the single home for the rule transcribed from
`work-item-create-remote.sh:62-87,238-246`: reject an empty identifier, one
carrying a control character, LF, CR or TAB anywhere, one whose first three
characters are `---`, or one whose first non-whitespace character is `#`. `/`,
`#` and `@` are explicitly permitted mid-token. Both clients call it; the fixture
of accepted and rejected identifiers lives beside it.

`RetryPolicy::delay_for` returns the delay as data rather than sleeping, so a
test asserts it directly instead of inferring it from hit counts and wall-clock.
`Jitter` and `Sleeper` are constructor parameters on the clients' transports, not
internals — nothing in the retry tests waits on real time.

The crate also owns `TransportConfig` (timeout, operation deadline, response cap,
page cap) and a `port_body` adapter appending the trailing newline
`RemoteIssue.body` requires, since both are shared obligations rather than
per-provider ones.

Its admission criterion, so it does not become a utility grab-bag: policy shared
by two or more provider clients, with no transport and no provider specifics.

**File**: `tasks/public_api.py`, `cli/pup.ron`, `tests/integration/pup/test_import_rule.py`
**Changes**: `tracker-support` is a genuine shared library, so it takes a
**snapshot** rather than an exemption, with a `denied`-only pup rule barring
`reqwest` and any HTTP client — it is policy, not transport — plus a probe pair.

#### 3. The exit-code table transcription

**File**: `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`
**Changes**: New committed fixture, keyed (code, provider, operation, class). It
lives in `tracker-support` — which already owns shared client policy and hosts
the differential test that drives it — **not** in `cli/tracker`. A bash-oracle
transcription read by three sibling crates is not port material, and putting it
in the frozen crate would contradict both the no-touch commitment and Phase 5's
no-diff gate.
The file records five mappers, not four, and states in its header that the keys
are each integration's own namespaced codes — **not** `curl` exit values, because
`jira-request.sh:340-348` and `linear-graphql.sh:232-241` collapse curl's status
into a boolean and every transport failure becomes script code 21.

Content is transcribed from:

| Mapper | Source |
|---|---|
| `_wicr_map_jira` | `work-item-create-remote.sh:105-111` |
| `_wicr_map_linear` | `work-item-create-remote.sh:92-97` |
| `_wiur_map_jira` | `work-item-update-remote.sh:51-57` |
| `_wiur_map_linear` | `work-item-update-remote.sh:66-72` |
| `_linear_map_no_file_failure` | `linear-create-flow.sh:177-182` |

The rows, with `retryable` = `E_DISPATCH_RETRYABLE` (70) and `terminal` =
`E_DISPATCH_TERMINAL` (71):

```
code  provider  operation  class      source
11    jira      create     retryable  _wicr_map_jira
12    jira      create     retryable  _wicr_map_jira
13    jira      create     retryable  _wicr_map_jira
14    jira      create     retryable  _wicr_map_jira
15    jira      create     retryable  _wicr_map_jira
17    jira      create     retryable  _wicr_map_jira
19    jira      create     retryable  _wicr_map_jira
22    jira      create     retryable  _wicr_map_jira
34    jira      create     retryable  _wicr_map_jira
100   jira      create     retryable  _wicr_map_jira
101   jira      create     retryable  _wicr_map_jira
102   jira      create     retryable  _wicr_map_jira
103   jira      create     retryable  _wicr_map_jira
104   jira      create     retryable  _wicr_map_jira
105   jira      create     retryable  _wicr_map_jira
106   jira      create     retryable  _wicr_map_jira
107   jira      create     retryable  _wicr_map_jira
108   jira      create     retryable  _wicr_map_jira
109   jira      create     terminal   _wicr_map_jira (falls to *)
```

and correspondingly for `jira update` (110-117 plus the identical transport
clause), `linear create` (the two-layer path: `_linear_map_no_file_failure`'s
`11 | 22 | 34 | 35 | 36` → `E_CREATE_PRE_SEND` 108 → `_wicr_map_linear` →
retryable; everything else → `E_CREATE_POST_SEND` 109 → terminal), and
`linear update` (`110-114` plus `11 | 18 | 22 | 23 | 25 | 27 | 29 | 35 | 36`).

The fixture header records three verified facts about the divergence:

- **The Jira pair does not diverge.** `work-item-create-remote.sh:108` and
  `work-item-update-remote.sh:54` are byte-identical including indentation:
  `11 | 12 | 13 | 14 | 15 | 17 | 19 | 22 | 34) return "$E_DISPATCH_RETRYABLE" ;;`
- **Linear diverges in both directions.** `34` is retryable on create and
  terminal on update — documented, because a 200-body error may mean the mutation
  applied (`work-item-update-remote.sh:62-65`). `18, 23, 25, 27, 29` run the
  other way with no rationale anywhere, despite all five being raised before a
  byte leaves the process.
- **Two further codes 0210's text omits.** `35` and `36` are retryable on
  **both** Linear operations, as are `11` and `22`. They are part of
  `_wiur_map_linear`'s clause and of `_linear_map_no_file_failure`'s pre-send
  set, and must appear in the fixture.

#### 4. The differential test against the running mappers

**File**: `cli/tracker-support/tests/mapper_differential.rs`
**Changes**: The fixture above and the Rust written against it agree with each
other whether or not either agrees with the bash. While the scripts are still on
disk, this test removes the doubt: it invokes each of the five mappers with every
code 0-130 and compares the resulting exit status to what the Rust classifier
returns for the same (code, provider, operation).

It needs no network and no credentials. ⚠️ It does **not** skip silently when
bash is missing: a gate that passes when the tool is absent is the failure mode
`cli/tracker-test-support/src/contract.rs` was built to avoid, and this test is
the plan's only structural defence against a mis-transcribed cell. It asserts a
**non-zero count of cases actually compared**, and treats a missing bash or an
unlocatable script as a failure on the platforms CI runs — macOS and Linux both
ship bash. A disagreement names the code, the mapper and both classifications.

Its comparison function is public, and a committed sibling test feeds it a
deliberately wrong classification and asserts the failure names the offending
code — the same tested-test discipline the import tripwire and pup probes get,
rather than a one-off manual plant that protects nothing after it is reverted.

0212 deletes this test in the same commit that deletes the scripts it drives.

#### 5. The bash-parity baseline transcription

**File**: `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`
**Changes**: New committed fixture recording, for each of the eleven parity tests
0212 converts, its fixture-case identifiers and its pre-conversion assertion
count; plus the committed **set of case names per directory** under
`skills/work/scripts/test-fixtures/` — `work-item-normalise/`,
`work-item-project-remote/`, `work-item-section-diff/`,
`work-item-sync-baseline/` (including `regenerate.sh`) and the loose root files.

The counts 0212 needs for attribution (6, 6, 15, 27, totalling 68) are **derived
from those name sets** by the guard and printed in its failure message, not
committed as separate numbers. Storing both representations when only one is
asserted is how the numeric copy goes stale and 0212 reads a wrong figure.

A test asserts the baseline still describes the corpus, so it cannot silently
drift before 0212 reads it. It compares the committed **set of case directory
names per subdirectory**, not the scalar 68: a bare count is simultaneously too
strict — an unrelated work item adding a fixture reds the build with a
misleading message — and too loose, since a delete plus an add nets to 68 and
passes while the drift it exists to catch goes through. The failure message names
which case appeared or vanished and says that 0212's baseline needs updating.

#### 6. The `linear.team_id` key

**File**: `cli/config/src/catalogue.rs`
**Changes**: Add `"linear.team_id"` to `EXTRA_KEYS`, ordered with the existing
`linear.*` entries.

```rust
pub const EXTRA_KEYS: &[&str] = &[
    "jira.allowed_sites",
    "jira.site",
    "jira.email",
    "jira.token",
    "jira.token_cmd",
    "linear.team_id",
    "linear.token",
    "linear.token_cmd",
    "github.token",
    "github.token_cmd",
    "visualiser.editor",
    "visualiser.editor_project",
    "visualiser.binary",
];
```

**File**: `scripts/config-defaults.sh`
**Changes**: Mirror the addition in the bash `EXTRA_KEYS`. The two lists have no
automated cross-check, so a test asserting the Rust list and the bash list agree
is added if one does not already exist.

Both new keys land here: `linear.team_id` and `jira.allowed_sites` (D14). Phases
are independently mergeable, so registering `jira.allowed_sites` in Phase 3 —
where it is *used* — would let Phase 2 merge without it and leave the escape
hatch invisible to `config dump` and undocumented for exactly the self-hosted
users Phase 7 breaks.

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`
**Changes**: Regenerate. The golden pins the dumped key set and will red on the
first `mise run` of this phase otherwise. It is exercised by
`cli/launcher/tests/config_read.rs`, not by the shell integration suite.

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add both keys to the documented key list, so the documented
configuration surface does not diverge from the shipped one for the users who
most need it. Note that `config-defaults.sh:199-207` describes the bash
`EXTRA_KEYS` registry as keys "read ad-hoc by their own consumers" — neither new
key has a bash consumer, so the registry now also carries Rust-only keys and its
comment says so.

#### 7. Unify `for_tracker_error`

**File**: `cli/work-cli/src/exit_codes.rs`
**Changes**: Remove `#[allow(dead_code)]` from `for_tracker_error`.

**File**: `cli/work-cli/src/create.rs`
**Changes**: Delete the hand-rolled twin at `:346` and call
`exit_codes::for_tracker_error` instead.

### Success Criteria

#### Automated Verification

- [x] `remote-projection` and `tracker-support` build and their snapshots are
      committed: `mise run lint:cli:public-api:check`
- [x] The moved projection's existing tests all pass, including
      `jira_body_canonicalisation_is_independent_of_key_order`:
      `cd cli && cargo nextest run -p remote-projection`
- [x] No `work-adapters` re-export of the projection remains, and its callers
      still pass: `cd cli && cargo nextest run -p work-adapters`
- [x] The exit-code fixture has a row for every arm of all five mappers,
      asserted by a test counting rows per (provider, operation) against the
      committed expected counts
- [x] The differential test agrees with the running bash for every code 0-130
      across all five mappers, asserts a non-zero comparison count, and fails
      when bash is unavailable rather than skipping
- [x] Its committed sibling test proves it can fail, by feeding the comparison
      function a wrong classification and asserting the message names the code
- [x] `resolve_token` covers every precedence branch, and a `token_cmd` that
      prints a sentinel secret and exits non-zero leaks it into neither the
      error's `Display`, its `Debug`, nor stderr
- [x] A `token_cmd` that hangs is abandoned at the timeout; one that prints
      unbounded output is truncated rather than buffered without limit
- [x] `identifier_is_safe` accepts and rejects exactly the committed fixture set
- [x] A `token_cmd` resolved from a VCS-tracked provenance file is **refused**
      with its distinct diagnostic rather than executed, and so is a
      `jira.allowed_sites` entry from the same source
- [x] A sentinel variable exported by the parent process is not visible to the
      helper, proving the environment scrub
- [x] Folding `RetryPolicy::delay_for` over attempts yields the expected sequence
      with a seeded jitter source, honours a `Retry-After` as a duration, caps at
      60s, and returns `None` once attempts are exhausted — asserted without
      sleeping
- [x] The baseline guard compares case-name sets per subdirectory, names the case
      that appeared or vanished, and derives the per-directory counts from those
      sets rather than from a committed number
- [x] `linear.team_id` is dumpable: `accelerator config dump` lists it
- [x] `dump.golden` matches after regeneration, with both new keys present:
      `cd cli && cargo nextest run -p accelerator config_read` (a suite with a
      documented flake history under parallel load — re-run before treating a
      failure as a golden mismatch)
- [x] The Rust and bash `EXTRA_KEYS` lists agree, asserted by a test
- [x] `for_tracker_error` has no `#[allow(dead_code)]` and `create.rs` has no
      match on `TrackerError` of its own:
      `rg -n "allow\(dead_code\)" cli/work-cli/src/exit_codes.rs` returns nothing
- [x] `cli/work-cli` tests pass: `cd cli && cargo nextest run -p accelerator-work`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [x] The exit-code fixture reads as a table a human can diff against the five
      bash `case` statements side by side — a cross-check the differential test
      now backs rather than substitutes for
- [x] The bash-parity baseline names all eleven tests 0212 will convert

---

## Phase 3: `jira-client` Foundation

### Overview

The crate, its credential resolution, its bounded transport with the 4-attempt
retry loop, and both of its classification tables. No `RemoteTracker` impl yet —
this phase ends with a registered library that nothing resolves, fully unit-tested
against the Phase 1 mock server.

### Changes Required

#### 1. The crate

**File**: `cli/jira-client/Cargo.toml`
**Changes**: New library, package name `jira-client` matching its directory.
`reqwest` reused from the workspace **verbatim** — `reqwest = { workspace = true
}` — because all three of its features are load-bearing against a named gate, and
no phase widens them.

```toml
[dependencies]
reqwest = { workspace = true }
rustls = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracker = { path = "../tracker" }
tracker-support = { path = "../tracker-support" }
remote-projection = { path = "../remote-projection" }
config = { path = "../config" }
kernel = { path = "../kernel" }

[dev-dependencies]
http-test-support = { path = "../http-test-support" }
tracker-test-support = { path = "../tracker-test-support" }

[lints]
workspace = true
```

Intra-workspace edges are **path** dependencies, matching every existing member
(`cli/work-adapters/Cargo.toml:19-24` and the rest). `cli/Cargo.toml`'s
`[workspace.dependencies]` table carries only third-party crates today and this
plan does not change that — declaring local crates as `{ workspace = true }`
without registering them there simply fails to resolve, and registering them
would introduce a second convention alongside the established one.

`rustls` is declared and its provider installed in the constructor per D7; the
trust-store narrowing is D8.

#### 2. Credential resolution

**File**: `cli/jira-client/src/auth.rs`
**Changes**: The ladder comes from `tracker_support::resolve_token` (D9); this
module supplies the keys and validates the other two values.

Jira needs three values, not one — `jira.site`, `jira.email` and a token:

```rust
pub struct Credentials { pub site: String, pub email: String, pub token: String }

pub enum TokenSource { Env, EnvCmd, Local, LocalCmd, Shared }

pub fn resolve_credentials(
    config: &dyn ConfigAccess,
) -> Result<Credentials, ClientError>;
```

It returns `ClientError`, not `kernel::Error` — `Refusal(String)` carries no
variants and no `source()`, so routing credential failures through it destroys
the structured `CredentialError` and makes the `source()`-chain criteria here and
in Phase 7 unsatisfiable. Each bash condition maps to a variant whose `Display`
carries the bash diagnostic text.

⚠️ **The ladder has five rungs and two environment sources, not the three-rung
shape an earlier draft described.** Read off `jira-auth.sh:165-239`
(`linear-auth.sh:177-250` is identical in shape), in order:

| # | Source | Notes |
|---|---|---|
| 1 | `ACCELERATOR_JIRA_TOKEN` | |
| 2 | `ACCELERATOR_JIRA_TOKEN_CMD` | a **second** env source; `TokenKeys` needs an `env_command` field to express it |
| 3 | `config.local.md` `token` | behind the permissions gate below |
| 4 | `config.local.md` `token_cmd` | behind the same gate |
| 5 | `config.md` `token` | **only when `config.local.md` is absent** |

Two consequences an earlier draft had backwards: the personal `token_cmd`
outranks the shared `token` value, and the shared file is consulted only when the
personal one does not exist at all — not merely when it lacks a token.

The permissions gate is a sixth failure mode the draft omitted entirely:

| Bash code | Condition | Rust variant |
|---|---|---|
| `24 E_NO_TOKEN` | nothing resolves | `Credential(NoToken)` |
| `25 E_TOKEN_CMD_FAILED` | `token_cmd` non-zero or unrunnable | `Credential(TokenCmdFailed)` |
| `26 E_TOKEN_CMD_FROM_SHARED_CONFIG` | `token_cmd` in `config.md` | `Credential(TokenCmdFromSharedConfig)` — refused, per D13 |
| `27 E_AUTH_NO_SITE` | `jira.site` unset | `NoSite` |
| `28 E_AUTH_NO_EMAIL` | `jira.email` unset | `NoEmail` |
| `29 E_LOCAL_PERMS_INSECURE` | `config.local.md` is a symlink, or its mode is looser than 0600 | `Credential(LocalPermsInsecure)` |

Code 29 is ported in full, including its override: `ACCELERATOR_ALLOW_INSECURE_LOCAL=1`
is honoured **only** when `.claude/insecure-local-ok` is a regular non-symlink
file that is VCS-tracked. That VCS-tracked test is also the primitive D12
requires, and `cli/vcs` provides it natively.

`jira.site` validation and `jira.allowed_sites` follow D14; the trust-boundary
rule for both `jira.allowed_sites` and `*.token_cmd` follows D12. Both keys are
registered in Phase 2 §6 alongside `linear.team_id`.

⚠️ This narrows working behaviour. `jira-request.sh` imposes no host restriction
today, so every self-hosted Jira user's current configuration becomes a refusal
at Phase 7 until they set `jira.allowed_sites`. The diagnostic names the key and
the offending host, and the narrowing is recorded in 0171's `## Decisions` as a
user-visible change.

**The test seam is the constructor, not an environment escape hatch.** The
allowlist is applied by `from_config` only. `Transport::new` takes a base `Url`
directly, so `contract_offline.rs`, the timeout, pagination, JQL-body and corpus
suites can point a client at a cleartext `MockServer` on `127.0.0.1` without any
process state — the same shape Phase 9 requires for the upload transport's
loopback admission, and for the same reason. A criterion asserts the loopback
allowance is unreachable through `from_config` regardless of environment.

The resolved token is rejected if it carries CR, LF or any control byte,
mirroring Linear's `E_TOKEN_MALFORMED` check — an unvalidated token
concatenated into a header is an injection vector on every subsequent request,
and the bash never guarded the Jira side.

⚠️ `26` is a **warning, not a fatal error** in `jira-auth.sh` — the bash ignores
the shared-config `token_cmd` and continues. `collaboration-cli`'s GitHub port
made it a `Refusal`. This plan follows the **`collaboration-cli` precedent** and
refuses, because a silently-ignored credential source is worse than a loud one,
and 0210's acceptance criterion requires the value be "refused". The divergence
from bash is deliberate and is recorded in 0171's `## Decisions`.

#### 3. The transport

**File**: `cli/jira-client/src/transport.rs`
**Changes**: A `reqwest::blocking::Client` built once, with a **30s** timeout
transcribed from `jira-request.sh:298`, overridable at construction for tests.
Bodies are serialised with `serde_json::to_string` and sent with an explicit
`Content-Type: application/json`, because the workspace `reqwest` entry carries
no `json` feature.

```rust
impl Transport {
    pub fn new(
        credentials: Credentials,
        config: TransportConfig,
        clock: Box<dyn Sleeper>,
        jitter: Box<dyn Jitter>,
    ) -> Result<Self, ClientError>;
}
```

`TransportConfig`, `Sleeper` and `Jitter` all come from `tracker-support`, not
from this crate — the bounds a third provider would most need to inherit belong
with the retry policy that uses them, and restating them per client is the drift
`tracker-support` exists to prevent:

```rust
pub struct TransportConfig {
    pub base: Url,
    pub timeout: Duration,          // default 30s
    pub deadline: Duration,         // whole-operation bound
    pub max_response_bytes: usize,  // default 8 MiB, unit-asserted
    pub max_pages: usize,           // default 20
}
```

⚠️ `Sleeper` and `Jitter` are constructor parameters, not internals. Phase 2
promises that "nothing in the retry tests waits on real time", and that is only
true if the seam actually reaches the transport — otherwise the mock-backed
retry suites execute real exponential backoff, roughly 7s per case and doubled
across the two providers, reintroducing exactly the sleep-based tests the seam
was designed to remove.

⚠️ The timeout is a constructor **parameter**, not a post-construction setter. A
`reqwest` client's timeout is fixed when the client is built, so a
`with_timeout(self, …)` applied after `new` had already built the client would be
a silent no-op — and the Phase 5 and 6 tests inject T = 400ms through this seam,
so they would quietly wait on the 30s default and either hang or pass for the
wrong reason. The "default is 30s" unit assertion reads the value actually handed
to the builder.

Both API transports set `redirect::Policy::none()`. Jira's and Linear's APIs
never legitimately redirect, and following one would defeat the path validator
below, which inspects only the initial path — a compromised or misconfigured
`jira.site` could otherwise bounce an authenticated request to an unintended
host.

Response bodies are read through `take(max_response_bytes)` before
deserialisation and rejected if they exceed it, so no endpoint the client already
talks to can exhaust memory with an oversized response. Linear's classifier
parses bodies on 200, 400 and 2xx-non-JSON paths, so the cap applies there too.

The retry loop reproduces `jira-request.sh:332-430` using
`tracker_support::RetryPolicy`: `max_attempts = 4`, backoff from `Retry-After`
where present, else exponential with ±30% jitter, capped at 60s.

⚠️ Retried statuses are **429 and 5xx only**; a transport failure resolves on the
first attempt with no retry (D15), matching `jira-request.sh:345-348` and
`linear-graphql.sh:238-241`, which exit 21 immediately. This is what makes the
Phase 5 timeout window valid — folding timeouts into the retry loop would give a
wall-clock of roughly 4T plus backoffs.

The operation-level deadline (D15) is checked before each page and each chunk.

Observability, through `tracing` so `ACCELERATOR_LOG` controls it and with no
token value ever in a field: `debug` per attempt carrying method, path, status,
attempt number and the backoff about to be taken; `warn` on retry exhaustion, on
page-cap truncation and on deadline expiry. Without it, an `indeterminate` batch
at 3am is indistinguishable between cap-hit, rate limit, exhausted retries and a
dropped connection.

Path validation is carried over from `jira-request.sh:75-145`: reject a path not
matching `^/rest/api/3/[A-Za-z0-9._/?=&,:%@-]*$`, reject `..` traversal and `//`,
and reject after URL-decoding with an 8-round decode cap. These are the
conditions behind bash code 17.

#### 4. The error taxonomy

**File**: `cli/jira-client/src/error.rs`
**Changes**: Four error types are in play across the three layers —
`tracker_support::CredentialError`, this crate's `ClientError`, the port's
`TrackerError`, and Phase 7's `SelectionError` — and the conversions between them
are stated here rather than left implicit in the classification tables.

`ClientError` is one `thiserror` enum whose variants mirror the transcribed
conditions (`NoSite`, `BadSite`, `NoEmail`, `Credential(CredentialError)`,
`BadPath`, `Transport`, `OversizedResponse`, `Unclassified`), implements
`std::error::Error` with a working `source()`, and carries a redacting `Debug`.

Two mappings, both tested: `ClientError → TrackerError` is the classification
table below; `ClientError → SelectionError::Unconfigured` keeps the error as a
**boxed source** rather than flattening it to a `String`, so a caller can still
tell a missing site from a failed credential helper from a shared-config refusal.
Collapsing all four categories into one opaque string is what makes a failure
undiagnosable at the CLI and untestable in the registry.

#### 5. Classification

**File**: `cli/jira-client/src/classify.rs`
**Changes**: Two tables, both per-operation, because
`cli/tracker/src/lib.rs:141-143` states a single status-to-class table is wrong by
construction.

The **status table**, transcribed from `jira-request.sh:363-442`:

| Status | Bash code | create | update | show / fetch_all |
|---|---|---|---|---|
| 400 | 34 | Retryable | Retryable | Retryable |
| 401 | 11 | Retryable | Retryable | Retryable |
| 403 | 12 | Retryable | Retryable | Retryable |
| 404 | 13 | Retryable | Retryable | Retryable |
| 410 | 14 | Retryable | Retryable | Retryable |
| 429 exhausted | 19 | Retryable | Retryable | Retryable |
| 5xx exhausted | 20 | **Terminal** | **Terminal** | Retryable |
| 2xx non-JSON | 16 | **Terminal** | **Terminal** | Retryable |
| connect / DNS / timeout | 21 | **Terminal** | **Terminal** | Retryable |
| other (1xx, 3xx, other 4xx) | 20 | **Terminal** | **Terminal** | Retryable |

The reads column is uniformly `Retryable` because
`cli/tracker/src/lib.rs:143` states a read never produces `Terminal`. The mutating
columns follow the bash tables: a status the transport proves was rejected
pre-application is retryable; anything that may have reached Jira is terminal.
Where the tables and the "provably pre-send" rule disagree, the tables win.

The **bash-code table** is driven directly from Phase 2's committed fixture: a
table-driven test reads
`cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`, filters to
`provider = jira`, and asserts a `TrackerError` class for every row. A row with
no assertion fails the build — the test counts rows consumed and compares against
rows present.

### Success Criteria

#### Automated Verification

- [x] Every precedence branch has a test against Jira's `TokenKeys`, mirroring
      `collaboration-cli/src/auth.rs`'s nine: env wins over config, config wins
      over personal `token_cmd`, a team `token_cmd` is refused, nothing
      configured is a refusal, and a missing `site` or `email` is a refusal
- [x] The team-level `token_cmd` refusal carries the diagnostic text, asserted on
      the message string not just the variant
- [x] Every row of the status table is asserted per operation; the test fails if
      a status is added to the table without an assertion
- [x] Every `provider = jira` row of the committed fixture is asserted; the
      row-coverage guard fails on an unconsumed row
- [x] The retry loop makes exactly 4 attempts on a persistent 503, asserted with
      `MockServer::hits`
- [x] The injected `Sleeper` records the durations it was asked to sleep, and the
      recorded sequence matches expectation while the real clock is never
      consulted — this, not a wall-clock ceiling, is what proves the seam is
      wired. A "single-digit milliseconds" bound would be an order of magnitude
      tighter than the 1.35×T bound D16 already rejects as flake-prone
- [x] `Retry-After` is honoured as a **duration**, not merely as a trigger: with
      an injected clock and seeded jitter, `Retry-After: 7` yields exactly 7s
      where the default backoff would have been something else
- [x] A connect, DNS or timeout failure makes exactly **one** attempt — no retry
- [x] The constructed default timeout is 30s, asserted as the value handed to the
      client builder; an injected 400ms timeout demonstrably takes effect
- [x] The Jira transport refuses a 302 rather than following it (Linear's is
      asserted in Phase 6a, where that crate first exists)
- [x] A response exceeding `max_response_bytes` is rejected before
      deserialisation, not buffered
- [x] Constructing a client leaves `rustls::crypto::CryptoProvider::get_default()`
      as `Some`, proving the provider is installed. This is a direct assertion on
      process state needing no server, no certificate and no new dev-dependency —
      the plain-HTTP mock structurally cannot show it, and an https stub would
      breach both the std-only rule on `http-test-support` and Phase 10's
      unchanged-dev-closure expectation
- [x] `jira.site` is refused for `http://`, for a userinfo-bearing URL, for one
      carrying a query or fragment, and for a host outside the allow shape — each
      before any request is built
- [x] A `jira.site` at `Level::Team` failing the shape is refused
- [x] A `jira.allowed_sites` entry present at `Level::Team` is refused and does
      **not** widen the accepted host set
- [x] A loopback base URL is reachable through `Transport::new` but unreachable
      through `from_config`, regardless of process environment
- [x] `max_response_bytes` defaults to 8 MiB, asserted as a unit value
- [x] A token carrying CR, LF or a control byte is refused
- [x] Path validation rejects traversal, `//`, a non-`/rest/api/3/` prefix, and a
      double-encoded traversal
- [x] `ClientError`'s variants survive to the caller with a working `source()`
      chain — a missing site, a failed helper and a shared-config refusal are
      distinguishable, not one string
- [x] `cd cli && cargo nextest run -p jira-client`
- [x] `deny:check` green: `mise run lint:cli:deny:check`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [ ] The status table in `classify.rs` reads against
      `jira-request.sh:363-442` line by line with no unexplained difference

---

## Phase 4: `jira-client` ADF Conversion

### Overview

Both conversion directions and the node-type inventory transcription. Pure
computation, no network. The render direction has no port caller — `show` returns
canonicalised ADF JSON, not markdown — but it is built here because the bash
oracle is on disk only until 0212 deletes it, and it is the harder direction to
reconstruct from a transcription.

### Changes Required

#### 1. The inventory transcription

**File**: `cli/jira-client/tests/fixtures/adf-node-types.txt`
**Changes**: New committed fixture, one node type per line, derived from
`jira-adf-render.jq`, `jira-md-assemble.jq` and `jira-md-tokenise.awk` rather
than authored free-hand. Each line records the type and its handling in each
direction.

Node types, with `R` = `jira-adf-render.jq` and `A` = `jira-md-assemble.jq`:

| Node type | Render | Assemble | Note |
|---|---|---|---|
| `doc` | handled (R:81-87) | handled (A:191) | assemble always emits `{"version":1,"type":"doc",…}` |
| `paragraph` | handled (R:55-56) | handled (A:69) | |
| `heading` | handled (R:57-59) | handled (A:122-126) | render reads `.attrs.level` with **no default**; assemble accepts H1-H6 only |
| `bulletList` | handled (R:60-61) | handled (A:77) | assemble emits no `attrs` |
| `orderedList` | handled (R:62-66) | handled, lossy (A:79) | render honours `.attrs.order // 1`; assemble **always** emits `order: 1` |
| `listItem` | handled, lossy (R:50-51) | handled (A:132-133) | render uses `.content[0].content` only — 2nd+ children silently dropped |
| `taskList` | handled (R:67-71) | handled (A:81-84) | assemble adds `attrs.localId` |
| `taskItem` | handled (R:68-71) | handled (A:150-153) | content is a bare inline array, not paragraph-wrapped |
| `codeBlock` | handled, lossy (R:72-75) | handled (A:92-93) | render uses `.content[0].text` only |
| `text` | handled (R:27-40) | handled (A:35, A:50) | |
| `hardBreak` | handled (R:41) | handled (A:118) | assemble drops it when `.para == null` |
| `blockquote` | block placeholder (R:77) | **hard-reject 41** | |
| `table` | block placeholder (R:77) | **hard-reject 41** | |
| nested list in `listItem` | **silently dropped** (R:51) | **hard-reject 41** | render does *not* placeholder it |
| `rule` | block placeholder (R:77) | absent | `---` becomes paragraph text |
| `panel`, `expand`, `decisionList`, `layoutSection`, `blockCard`, `embedCard`, `extension`, `bodiedExtension`, `mediaSingle`, `mediaGroup` | block placeholder (R:77) | absent | |
| `emoji`, `mention`, `date`, `status`, `inlineCard`, `inlineExtension`, `placeholder` | **inline** placeholder (R:42) | absent | |
| `tableRow`, `tableCell`, `tableHeader`, `media`, `caption`, `layoutColumn`, `decisionItem`, `nestedExpand` | unreachable | absent | render never descends into their parents |

Mark types:

| Mark | Render | Assemble | Note |
|---|---|---|---|
| `code` | handled (R:30) | handled (A:34-35) | innermost; assemble regex is `` `[^`]+` `` |
| `em` | handled (R:31) | handled (A:40-41) | `_x_` is **not** em |
| `strong` | handled (R:32) | handled (A:38-39) | |
| `strong`+`em` | `***x***` | handled (A:36-37), array `[strong, em]` | |
| `link` | handled (R:34-40) | handled (A:42-48) | render applies an href allowlist; assemble appends the mark **last** |
| `strike`, `underline`, `subsup`, `textColor`, `backgroundColor`, `annotation`, `alignment`, `indentation`, `breakout`, `border`, `dataConsumer`, `fragment` | **silently ignored** | absent | no placeholder, no warning |

Both placeholder strings are recorded verbatim, because which one fires depends
on position rather than type:

```
[unsupported ADF node: <type>]      block position   jira-adf-render.jq:77
[unsupported ADF inline: <type>]    inline position  jira-adf-render.jq:42
```

The round-trip asymmetry is recorded: the render direction accepts a strictly
larger language and degrades to placeholders, while the assemble direction
hard-rejects with exit 41. The four rejections, verbatim from
`jira-md-tokenise.awk`:

```
41  E_ADF_UNSUPPORTED_BLOCKQUOTE: blockquote is not supported          T:71-72
41  E_ADF_UNSUPPORTED_TABLE: pipe tables are not supported             T:78-79
41  E_ADF_UNSUPPORTED_NESTED_LIST: nested lists are not supported      T:85-86
42  E_ADF_BAD_INPUT: input contains control byte \x1e or \x1f          T:45-47
```

The fixture additionally records a **render aborts** class — the three inputs on
which the jq raises and the whole document fails, rather than degrading to a
placeholder. These are shapes a real tenant can return and the fixture corpus
does not otherwise contain, so leaving them unspecified means the Rust invents a
default (H1, or an empty string) and diverges precisely where the oracle is about
to be deleted:

```
heading with no .attrs.level     [range(null)] raises   jira-adf-render.jq:58
root node whose type is not doc  error("E_BAD_JSON")    jira-adf-render.jq:81
```

⚠️ There are **two** abort conditions, not three. A text node with no `.text`
does not abort in either variant: jq defines `null + x == x`, so a marked node
yields the mark delimiters around an empty string (`` `` `` for `code`) and the
link branch at `:37` yields `[](href)`, while an unmarked node's `null` flows
through `render_inline` into `render_inlines`'s `join("")`, where jq treats a
null element as an empty string.

Both no-`.text` variants are therefore fixtured as rendering — not as typed
errors. Making either an error would reject a document the oracle renders
successfully, the divergence direction the differential test exists to catch, and
would red that test on the very fixture this section tells it to commit. Their
expected output is **generated by running the pinned jq**, not read off the
source.

#### 2. The render direction

**File**: `cli/jira-client/src/adf/render.rs`
**Changes**: ADF to markdown, over untyped `serde_json::Value` so no typed
struct can reorder keys.

Byte-fidelity rules, transcribed from `jira-adf-render.jq`:

- Bullet marker `- `, zero indent, items joined `\n` (R:61)
- Ordered marker `<n>. ` where `n = index + (attrs.order // 1)`, zero indent,
  no padding (R:63-65)
- Task marker `- [x] ` / `- [x] `, exactly one space after the bracket (R:69)
- **No indentation anywhere** — nested structure is unrepresentable
- Code fence exactly three backticks, language concatenated with **no
  separator**: `"```" + (.attrs.language // "")` (R:73)
- Hard break is literally `"  \n"` — two spaces then newline (R:41)
- Blocks joined `"\n\n"`; empty `doc.content` produces **zero bytes**, not a
  newline (R:84)
- Inlines joined `""` (R:47)
- **No text escaping whatsoever** (R:27-40). A literal `*`, `_`, `` ` ``, `[`,
  `#` or `|` in ADF text is emitted bare, so the round trip is not injective
- Headings are ATX only, no closing hashes, no level clamping (R:58-59)
- Marks applied as a fixed pipeline innermost-first: `code`, `em`, `strong`,
  `link`. Tests are membership, not array order, so ADF `marks` order is
  irrelevant and output nesting is always this order (R:30-40)
- `link` href allowlist (R:14-23): trim leading whitespace, lowercase, allow
  `http://`, `https://`, `mailto:`, reject anything else matching
  `^[a-z][a-z0-9+.\-]*:`, allow schemeless / relative / fragment /
  protocol-relative. On rejection the mark is **dropped and the inner text
  emitted bare** — not a placeholder. The emitted href is the **untrimmed**
  original, so leading whitespace survives (R:37)

#### 3. The assemble direction

**File**: `cli/jira-client/src/adf/assemble.rs`,
`cli/jira-client/src/adf/tokenise.rs`
**Changes**: Markdown to ADF, reproducing the tokeniser and assembler as two
stages so the rejection points stay where bash has them.

- `localId` on `taskList`: the deterministic
  `00000000-0000-4000-8000-00000000000N` form when a seed is supplied, else the
  bare counter integer as a string (A:60, A:83). The seed is a constructor
  parameter, not an environment read, so tests need no process state
- Inline alternation priority, fixed (A:26): code span, `***`, `**`, `*`,
  `[t](u)`, bare `[t]`, plain run `[^`*\[]+`, single-char catch-all
- Marks **cannot nest** except inside link text, which recurses and then appends
  the link mark to each child (A:46-47), giving array order `[inner…, link]`
- A bare `[text]` with no `(...)` keeps its brackets literally (A:42, A:50)
- Unbalanced emphasis degrades to one text node per character; **adjacent text
  nodes are never merged**, so `a*b` yields three nodes
- Soft-wrapped paragraph lines join with a single space (T:148)
- A trailing double-space is a hard break **only** in the paragraph rule (T:142);
  on a heading or list item those spaces are retained inside the text
- The non-fatal notice, exit stays 0 (T:131-132):
  `Notice: '__...__' is not emphasis in this subset; use **...** for bold`
- The three exit-41 rejections and the exit-42 control-byte rejection become
  typed errors. ⚠️ The tokeniser's table guard is **narrow**: a line must start
  `|` *and* end `|` with optional trailing space, so `| a | b` is *not* rejected
  and becomes paragraph text. The nested-list guard has **no space requirement**
  after the marker, so an indented `-word` continuation line falsely rejects.
  Both quirks are reproduced, and each has a test naming it as intentional
  fidelity rather than a bug

#### 4. Fixtures

**File**: `cli/jira-client/tests/fixtures/adf/`
**Changes**: A record per case, exercising both directions and covering every
entry in the inventory — including the two placeholder positions, the four
rejections, the three render-abort conditions, the `strike`-silently-dropped
case, the `listItem` second-child drop, the `orderedList` order-always-1
asymmetry, and the href allowlist's drop-mark-keep-text behaviour.

#### 5. The differential test against the running pipeline

**File**: `cli/jira-client/tests/adf_differential.rs`
**Changes**: The inventory and the fixtures were transcribed from the jq and awk
by hand, and the Rust was written to satisfy them — so the pair agrees with
itself whether or not it agrees with the oracle. While the assets are on disk,
this test drives `jira-adf-to-md.sh` and the tokenise/assemble pipeline over
every case in `tests/fixtures/adf/` and asserts byte-identity with the Rust
output in both directions, including the exit codes on the rejection cases.

It runs in the default suite with no network and no credentials, and — like the
mapper differential — asserts a non-zero count of cases compared, failing rather
than skipping when bash or jq is unavailable. Its comparison function is public
and driven by a committed sibling test that plants a wrong rendering and asserts
the failure names the rule. 0212 deletes it alongside the assets it drives. This
is what turns the manual "confirm the render output is byte-identical by running
both" criterion into something that keeps holding.

#### 6. The fidelity quirks record

**File**: `cli/jira-client/tests/fixtures/adf-fidelity-quirks.txt`
**Changes**: Several reproduced behaviours are knowingly wrong but faithful: the
table guard that misses `| a | b`, the nested-list guard that falsely rejects an
indented `-word` continuation, `listItem`'s dropped second child, `codeBlock`
reading only `.content[0].text`, `attrs.order` always 1, and the marks ignored
with no placeholder.

Each is recorded once here with its bash source line and the reason it is
preserved, and each quirk's test references this file rather than restating the
rationale. Given how little tolerance the codebase has for comments, a test name
alone is too thin a signal that the false nested-list rejection is intended —
someone will "fix" it, and the fix silently reclassifies corpus items.

### Success Criteria

#### Automated Verification

- [x] The inventory fixture has a line for every node type and mark type in the
      table above, asserted by a test that cross-checks the fixture against the
      set of types the render and assemble code actually handles — an unlisted
      type fails the build
- [x] Both placeholder strings are asserted verbatim, in the correct positions
- [x] Every one of the four rejections is asserted with its exact code and
      message text
- [x] A round-trip test asserts the asymmetry rather than identity: render
      accepts every node type; assemble rejects three and is absent for the rest
- [x] `attrs.order` is 1 on assemble even for input `3. foo`
- [x] Empty `doc.content` renders to zero bytes
- [x] `a*b` assembles to three unmerged text nodes
- [x] A rejected href drops the mark and keeps the text bare
- [x] Each of the three render-abort conditions produces its typed error rather
      than an invented default
- [x] The differential test agrees with the running jq and awk on every fixture
      case in both directions, asserts a non-zero comparison count, and fails
      when bash or jq is unavailable rather than skipping
- [x] Its committed sibling test proves it can fail, by feeding the comparison
      function a wrong rendering and asserting the message names the rule
- [x] Every quirk test names `adf-fidelity-quirks.txt` as its rationale
- [x] `cd cli && cargo nextest run -p jira-client`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [ ] Read `adf-node-types.txt` beside the three bash assets and confirm no type
      appearing in either is missing from the file — a cross-check the
      differential test now backs rather than substitutes for

---

## Phase 5: `jira-client` `impl RemoteTracker`

### Overview

The four port operations, with JQL composition, `/search/jql` cursor pagination,
the page cap, identifier safety, timestamp mapping, and the offline projection
assertions against the committed corpus. The contract harness lands here but is
gated out of the default run.

### Changes Required

#### 1. The port implementation

**File**: `cli/jira-client/src/lib.rs`
**Changes**: `impl RemoteTracker for JiraClient`, over the Phase 3 transport.

```rust
impl RemoteTracker for JiraClient {
    fn create(&self, title: &str, body: &str, kind: &str)
        -> Result<ExternalId, TrackerError>;
    fn update(&self, id: &ExternalId, title: &str, body: &str)
        -> Result<(), TrackerError>;
    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError>;
    fn fetch_all(&self, ids: &[ExternalId])
        -> Result<FetchOutcome, TrackerError>;
}
```

⚠️ `show` returns `Result<RemoteIssue, TrackerError>`, **not** `Option`. The
frozen port at `cli/tracker/src/lib.rs:317` declares exactly this, and its doc
states absence is deliberately not discoverable here. A 404 is
`TrackerError::Retryable` (bash code 13), never `Ok(None)` — a `show` reporting
`None` for a failed or truncated read is what lets a sync delete a live issue,
which is why `FetchOutcome` partitions `absent` from `indeterminate`.

`create` and `update` convert `body` from markdown to ADF through Phase 4's
assembler. `kind` is mapped onto a Jira issue type, with the empty string meaning
"use the tracker's configured default", per the port's doc.

#### 2. Identifier safety

**File**: `cli/jira-client/src/identifier.rs`
**Changes**: `ExternalId::new` is `pub const fn` and infallible by freeze, so the
check is the client's. The rule itself is `tracker_support::identifier_is_safe`,
so both providers share one implementation and one fixture rather than two copies
that drift.

An unsafe identifier from a `create` response is a `TrackerError::Terminal` — the
issue exists remotely, so a repeat would duplicate it.

⚠️ The check applies to **every id entering a request**, not only to ids coming
back from `create`. Ids read from the local corpus are equally untrusted: they
were written by a previous sync, by hand, or by a tracker that may since have
been compromised. Phase 3's path class permits `/`, `%`, `?`, `&` and `=`, so an
`external_id` of the form `ISSUE/../../../../rest/api/3/mypermissions` or
`ISSUE?x=` interpolated into one of Phase 8's paths re-targets the request to a
different endpoint under the user's credentials.

Two mechanisms, both required: every interpolated path segment is
percent-encoded with a path-segment encoder, and the Phase 3 path validator runs
over the **final composed path** for every request shape in Phases 5, 8 and 9 —
not only over the template.

⚠️ The ordering matters, because the two mechanisms otherwise contradict each
other. `identifier_is_safe` permits `/` and `@` mid-token, so a legitimate id
containing `/` encodes to `%2F`; the validator's decode-and-recheck pass would
then see `//` or a traversal-shaped path and reject an id the rule explicitly
allows. The first legitimate id to trip that looks like a bug, and the cheapest
fix is to drop one of the two layers.

The validator therefore checks the **template plus its decoded segments
individually**, not the flattened composed string: structure is validated on the
encoded path, and traversal on each decoded segment in isolation. A criterion
asserts an id containing a legitimate `/` is accepted end to end while
`ISSUE/../../rest/api/3/mypermissions` and a double-encoded traversal are both
refused.

#### 3. Projection and timestamps

**File**: `cli/jira-client/src/lib.rs`
**Changes**: `RemoteIssue.body` is the **un-normalised** projection, produced by
calling `remote_projection::project(Integration::Jira, Op::Body, &value)` on the
raw response `Value` and then appending the trailing newline through
`tracker_support::port_body` — `project` deliberately emits none, and the port
doc requires one. Phase 6b does the same. A criterion in each phase asserts
`RemoteIssue.body` ends in exactly one newline. The payload stays untyped `serde_json::Value` end to end —
never round-tripped through a typed ADF struct — because
`serde_json`'s `BTreeMap` backing is what makes `to_string` match `jq -cS`, and a
typed struct with declaration-order serialisation would silently rehash every
Jira item.

`RemoteTimestamp` maps per the port: a populated stamp to `Reported(bytes)`
verbatim including Jira's colon-less `+0000` offset; a blank, absent or `null`
one to `NotReported`, never `Reported("")`. `NotRead` is unreachable through the
port.

#### 4. Search, JQL and pagination

**File**: `cli/jira-client/src/jql.rs`, `cli/jira-client/src/search.rs`
**Changes**: `fetch_all` posts to `/rest/api/3/search/jql` with a body of
`{jql, fields, fieldsByKeys, maxResults, nextPageToken}`, composing
`key in (…)` over the requested ids in chunks of 50 with `maxResults` 100, and
following the cursor `nextPageToken` — the migration `gouqi` completed and most
Jira clients did not, read as a reference and attributed.

The port states four totality obligations (`cli/tracker/src/lib.rs:199-236`) that
the request shape alone does not satisfy. Each is a stated behaviour with its own
test:

- **An empty `ids` request makes no remote call** and yields an empty outcome.
  Composing `key in ()` produces malformed JQL that fails the entire sync.
- **Ids are deduplicated** before composition, as `_wifr_linear_keys` does with
  `| unique`.
- **An id that cannot be safely embedded is a pre-flight `Err`**, not a silently
  corrupted query.
- **The page cap is per chunk**, matching `work-item-fetch-remote.sh:110-141`,
  with that chunk's keys marked `indeterminate` on a cap-hit or failure. A global
  cap would mark whole chunks indeterminate for large corpora.

⚠️ **The key clause is the sole filter.** `work-item-fetch-remote.sh:26-30`
documents that the key-scoped read passes `--all-projects` specifically so no
injected `project = <default>` clause drops cross-project keys. `fetch_all`
therefore composes the key clause only and does not reuse `jql_compose`'s flag
families. An out-of-project key that came back unfound from a believed-complete
read would be reported `absent`, and the sync would unlink a live issue.

⚠️ **JQL values are escaped, not concatenated.** Identifiers reach this composer
from work-item files, having originally come from a remote tracker. One
containing `"`, `)` or ` OR ` breaks out of the `key in (...)` clause and changes
which issues the query returns — turning a targeted fetch into a project dump, or
hiding issues that exist. Every interpolated value is emitted as a quoted JQL
string literal with `\` and `"` escaped; values carrying control bytes are
rejected outright; flag and field tokens go through a validated allowlist rather
than concatenation.

`jql.rs` carries the composer ported from `jira-jql.sh`'s `jql_compose` for the
*search* surface: ten flag families with `~` negation, plus `@me`-to-accountId
and field-token-to-`customfield_NNNNN` resolution. Those resolutions are injected
behind small ports (`AccountResolver`, `FieldResolver`) from the outset, so
Phases 5 and 8 do not rewrite the constructor between them: Phase 5 injects a
fixed-map implementation, Phase 8 adds the cache-backed one with no signature
change. The composer is driven from a committed fixture of (flags in, expected
JQL) covering all ten families, each negation, `@me` resolution and an
unresolvable field token, under the same row-coverage guard the classification
tables use — a silently dropped family yields valid JQL over the wrong issue set,
which is a silent-wrong-answer failure rather than a crash.

Because `from_config` returns a `Box<dyn RemoteTracker>` with no lifetime, it
resolves every configuration value eagerly into owned `Credentials` and owned
resolver maps. A client retaining a borrow of the registry's `&dyn ConfigAccess`
could not be boxed into the returned type.

#### 5. The contract harness

**File**: `cli/jira-client/tests/contract.rs`
**Changes**: Named exactly `contract.rs` — `tracker_contract.rs` would silently
join the default run and make live API calls in `mise run`. No cargo feature, no
`#[ignore]`, no per-crate `-E` expression; the binary-name filter in
`cli/.config/nextest.toml` is the only mechanism.

`impl ContractSubject for JiraClient`, nominating `unaccountable_id` (an id
beyond the 20-page cap or outside a 50-key chunk) and `unreadable_id` (an id
whose `show` fails).

**File**: `cli/tracker-test-support/src/contract.rs`
**Changes**: ⚠️ The conformance functions cannot currently be called offline.
Each begins with `ensure_opted_in()?` and returns `ContractGateError::NotOptedIn`
unless `ACCELERATOR_TRACKER_CONTRACT=1`, and that crate's own test
`every_tracker_touching_entry_point_refuses_when_the_gate_is_closed` pins the
behaviour deliberately, so a caller reaching a property directly cannot thereby
reach a live provider. Setting the variable inside a default-profile test would
reopen exactly that hole.

Each property is therefore split in two: an **ungated** `fn *_property(subject)`
carrying the assertions, and the existing gated wrapper that calls it after
`ensure_opted_in()`. `gated_calls()` continues to cover only the wrappers, so the
gate-closure guard keeps its meaning, and the ungated halves become callable by
any subject the caller has already constructed — mock-backed or live.

**File**: `cli/jira-client/tests/contract_offline.rs`
**Changes**: The `*_property` functions run against a client pointed at a
`MockServer`, in an ordinary test binary that the default profile **does**
select. The mock serves the truncation and read-failure scenarios
`unaccountable_id` and `unreadable_id` nominate, so partition totality,
read-never-terminal and the create/show round-trip are enforced continuously and
offline.

The binary asserts a **non-zero count of properties actually executed**, so a
future regression that made every property a no-op is distinguishable from a real
run — the failure mode the gate itself was built to avoid.

Without this, the port's core invariants are checked only by a live credentialed
run whose output is a committed text file — and a text file cannot fail. A
refactor that reclassified a failed read as `Terminal` would ship green while the
stale transcript signalled the opposite. The live run stays as additional
assurance, not as the enforcing route.

#### 6. Offline corpus assertions

**File**: `cli/jira-client/tests/projection_corpus.rs`
**Changes**: Byte-identity against the two Jira records in
`skills/work/scripts/test-fixtures/work-item-project-remote/` — `case-jira` and
`case-jira-reordered` — and a sha256-after-`work::normalise` comparison against
the two Jira records in `work-item-sync-baseline/`: `case-jira-adf` and
`case-jira-no-description`. Runs with no network target.

The key-order invariance test projects `case-jira` and `case-jira-reordered` and
compares them **to each other**, so a client enabling `preserve_order` fails
rather than silently rehashing the corpus.

⚠️ `case-jira-no-description/remote.json` is **key-absent**, not `null` and not
`""`. `jq -cS '… // null'` yields the four-byte literal `null`. A typed
deserialiser with `Option<Adf>` and `#[serde(default)]` gives `None`, then an
empty string, then a different `remote_hash`, then mass reclassification. The
golden for this case ends in the literal `null`.

**The `serde_json`-versus-`jq` divergence needs a chosen policy, not just an
assertion.** The 0194 validation flagged it as uncovered and named this work as
"where a live Jira payload first meets the recipe". The two serialisers genuinely
differ on numbers: `serde_json` parses into `f64`/`i64`/`u64` and re-renders, so
a literal outside those ranges loses precision and a float's rendered form need
not match its input token. `RemoteIssue.body` feeds `remote_hash`, and a
formatting difference on a single numeric custom field mass-reclassifies every
such item as `remotely-modified` on first live sync.

Per D6, the fix is a local raw-token preserving re-serialiser in
`remote-projection` — `serde_json::value::RawValue` on the numeric paths, or an
equivalent — leaving every other crate's `serde_json` semantics untouched.

⚠️ It parses remote-controlled JSON, so it states explicit nesting-depth and
token-length limits and returns a typed error rather than panicking or recursing
without bound. A small adversarial fixture (deeply nested arrays, a very long
numeric literal, a truncated raw token) asserts that.

⚠️ **The parity claim is version-bound and must be generated, not recalled.**
`mise.toml:16` pins jq 1.7.1, which preserves the literal of numbers it does not
operate on; the widely-cited `1.0` → `1` canonicalisation is jq **1.6**
behaviour. The bash oracle, however, runs on whatever jq the *user* has.

The committed table of adversarial scalars — `1e999`, `9007199254740993`, a
whole-valued float, the escaped control bytes U+0000, U+001F and U+007F, a lone
surrogate, and a non-BMP character — therefore records the jq version it was
generated against in its header, and each expected string is produced by
**running the pinned `jq -cS`** rather than asserted from memory. Rows carry the
same row-coverage guard the classification tables use; the offline fixtures may
contain no numbers at all, so without this table the assertion passes vacuously.
Any row where the Rust cannot match jq is listed explicitly as an accepted
divergence with its rationale rather than silently reconciled.

#### 7. Timeout behaviour

**File**: `cli/jira-client/tests/timeouts.rs`
**Changes**: With an injected timeout T against a `Route::Stall` endpoint, `show`
**fails** and `fetch_all` **returns `Ok` with every requested id in
`indeterminate`** — because `cli/tracker/src/lib.rs:326-330` requires a
post-attempt transport failure to be an `Ok`, not an `Err`.

The assertions are asymmetric per D16, verified at T = 400ms and T = 1s.

A unit assertion confirms the constructed default is 30s and the page cap 20. A
paginated fixture offering 21 or more pages stops after 20 and reports the unseen
ids as `indeterminate`. A separate fixture holds every page slow enough to expire
the operation deadline while each individual request stays inside its timeout,
asserting the deadline fires and degrades the same way a cap-hit does.

#### 8. Registration

**File**: `cli/pup.ron`, `tests/integration/pup/test_import_rule.py`,
`tasks/public_api.py`, `cli/Cargo.toml`
**Changes**: A `denied`-only rule — preferred over an `allowed_only` permit list
because cargo-pup resolves `use a::{b, c}` to an empty module name, so a permit
list forces one single-item `use` per line throughout the crate.

```ron
Module((
    name: "jira_client_is_the_only_jira_transport",
    matches: Module("^jira_client($|::)"),
    rules: [
        RestrictImports(
            allowed_only: None,
            denied: Some([
                "^work(::|$)",
                "^work_adapters(::|$)",
                "^linear_client(::|$)",
            ]),
            severity: Error,
        ),
    ],
)),
```

Classified `_ADAPTER` in `_EXEMPT_MEMBERS`, following `github`'s precedent — no
snapshot, no nightly rustdoc cost. A probe pair asserting exit, the `is denied`
substring and the rule name, plus a control carrying real imports.

### Success Criteria

#### Automated Verification

- [x] All four port operations implemented against the port's own signatures —
      `show` returns `Result<RemoteIssue, TrackerError>` and a 404 is `Retryable`,
      not an absence: `cd cli && cargo nextest run -p jira-client`
- [x] Each of the four malformed-identifier shapes returns `Terminal`: empty, a
      control character, LF/CR/TAB, leading `---`, leading `#` after optional
      whitespace
- [x] `/`, `#` and `@` mid-token are accepted
- [x] Identifier safety runs on ids from the local corpus, not only on `create`
      responses; a traversal-bearing and a `?`-bearing issue key are each
      rejected before any request is sent
- [x] Every interpolated path segment is percent-encoded, and the path validator
      runs over the final composed path
- [x] An empty `ids` request makes zero remote calls
- [x] Duplicate ids are deduplicated before composition
- [x] The page cap is applied per 50-id chunk, not globally
- [x] Hostile identifiers (`X") OR project = FOO --`) leave exactly one bounded
      `key in` clause in the composed body
- [x] `fetch_all`'s JQL carries no `project =` clause
- [x] The JQL fixture covers all ten flag families, each negation, `@me`
      resolution and an unresolvable field token, under a row-coverage guard
- [x] The adversarial-scalar table matches the pinned `jq -cS` for every row, with
      the jq version recorded in the fixture header and any accepted divergence
      listed explicitly
- [x] `serde_json`'s feature set is unchanged workspace-wide — `arbitrary_precision`
      is absent from the graph
- [x] `RemoteIssue.body` ends in exactly one newline, while the moved parity test
      still compares `project(...)` without one
- [x] An id containing a legitimate `/` survives encoding and validation, while a
      traversal and a double-encoded traversal are both refused
- [x] Equality against `case-jira` and `case-jira-reordered`'s line-reconstructed
      bodies (the fixtures are keyed metadata, not raw bodies), with the trailing
      newline asserted separately on `RemoteIssue.body`
- [x] Key-order invariance: the two projections equal each other
- [x] sha256-after-normalise equals the committed `remote_hash` for
      `case-jira-adf` and `case-jira-no-description`
- [x] The absent-description golden ends in the literal `null`, with no blank
      line before the description and a trailing newline
- [x] `NotReported` for absent, `null` and empty-string stamps; `Reported` with
      bytes unaltered for `2026-01-01T00:00:00.000+0000`
- [x] Neither `show` nor `fetch_all` returns before T at T = 400ms and T = 1s,
      each returns within 3×T, and the failure is a timeout variant — `show`
      failing, `fetch_all` `Ok` with all ids `indeterminate`
- [x] Default timeout asserts 30s; page cap asserts 20
- [x] A 21-page fixture stops at 20 and reports unseen ids `indeterminate`, not
      `absent`
- [x] The operation deadline fires on a fixture whose pages are individually
      inside the request timeout, degrading to `Ok` all-`indeterminate`
- [x] JQL composition is pinned by request-body assertions via
      `MockServer::last_body`
- [x] The offline contract conformance run passes for `JiraClient` against a
      mock, in the default profile — partition totality, read-never-terminal and
      the create/show round-trip — and asserts a non-zero count of properties
      actually executed
- [x] `ContractSubject`'s gated wrappers still refuse when the gate is closed;
      `every_tracker_touching_entry_point_refuses_when_the_gate_is_closed` and
      `gated_calls()` both still pass after the ungated split
- [ ] The default profile selects no binary named exactly `contract`, and **does**
      select `contract_offline` — asserted on `cargo nextest list
      --message-format json` filtered by exact binary name. A substring grep
      cannot express this, since `nextest.toml` filters on `binary(=contract)`
      by design and `contract_offline` legitimately contains the substring
- [x] The pup probe pair passes: `mise run test:integration:pup`
- [x] `cli/tracker/src`, `cli/tracker/Cargo.toml` and
      `cli/tracker/tests/fixtures/public-api.txt` are all unchanged — the gate is
      those paths, not the whole directory, since no phase adds anything under
      `cli/tracker/`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [ ] `mise run test:integration:tracker-contract` passes against a live Jira
      tenant, exercising all four operations plus partition totality and
      read-never-terminal
- [ ] `accelerator work sync` against a live Jira project behaves as the bash
      bridge does

---

## Phase 6a: `linear-client` Foundation

### Overview

The crate, auth, the transport, and the body-parsing classification. Linear needs
no ADF layer, but its classification is strictly harder than Jira's — the body is
parsed on 200 *and* 400, the create path maps through two layers, and the
retryable/terminal divergence runs in both directions. That is where a
misclassified auth failure would hide, and the plan's own Testing Strategy names
it as the defect that "passes every criterion", so it gets its own reviewable
phase rather than arriving inside a phase that also carries the port impl,
pagination, corpus assertions and registration.

### Changes Required

#### 1. The crate and auth

**File**: `cli/linear-client/Cargo.toml`, `cli/linear-client/src/auth.rs`
**Changes**: Same dependency set and manifest conventions as `jira-client` minus
the ADF surface, including `rustls` and the `install_default()` call (D7). The
ladder is `tracker_support::resolve_token` with Linear's `TokenKeys`; identifier
safety is `tracker_support::identifier_is_safe` (D9). Credentials are a token
plus a team id:

```rust
pub struct Credentials { pub token: String, pub team_id: String }
```

Precedence ported from `skills/integrations/linear/scripts/linear-auth.sh`:
environment, then `linear.token`, then a `Level::Team` `linear.token_cmd`
refusal, then a `Level::Personal` `token_cmd` run.

⚠️ `team_id` is **not** a config key today. `linear-create-flow.sh:97-110` reads
`.team.id` from the `catalogue.json` that `linear-init-flow.sh:184-196` writes,
so every already-onboarded user has a populated catalogue and no
`linear.team_id`. Requiring the key outright would make `work sync --integration
linear` report `Unconfigured`/74 from Phase 7 onward for users whose bash path
works fine, and would leave one fact with two disagreeing sources of truth.

Resolution order is therefore the `linear.team_id` key Phase 2 added, falling
back to `catalogue.json`'s `.team.id` when the key is unset. Both sources and
their precedence get a test, and the fallback is removed only when 0211 makes the
key authoritative.

The auth band differs from Jira's: Linear has **no 28** (token only, no site or
email), and its **27 is `E_TOKEN_MALFORMED`** — a token that would corrupt
`curl --config -` — rather than Jira's `E_AUTH_NO_SITE`. The malformed-token check
is reproduced because codes 25, 27 and 29 are re-exited verbatim by
`linear-graphql.sh:481-489` and appear in `_wiur_map_linear`'s retryable clause.

#### 2. The transport

**File**: `cli/linear-client/src/transport.rs`
**Changes**: A single `POST https://api.linear.app/graphql`, 30s timeout
transcribed from `linear-graphql.sh:519`, overridable at construction.
`max_attempts = 4` with the same jittered backoff. Query documents are
hand-rolled strings with `serde_json` variables, per the recorded decision
against codegen.

#### 3. Classification

**File**: `cli/linear-client/src/classify.rs`
**Changes**: ⚠️ Linear's classification **must parse the response body**, not
only the status. Transcribed from `linear-graphql.sh:257-336`:

| Condition | Status | Bash code | create | update | reads |
|---|---|---|---|---|---|
| `errors[]` classified `auth` | **200** | 11 | Retryable | Retryable | Retryable |
| `errors[]` classified `complexity` | **200** | 36 | Retryable | Retryable | Retryable |
| `errors[]` other | **200** | 34 | Retryable | **Terminal** | Retryable |
| 2xx non-JSON body | 2xx | 16 | Terminal | Terminal | Retryable |
| unauthorised | 401 | 11 | Retryable | Retryable | Retryable |
| `errors[]` `auth` | 400 | 11 | Retryable | Retryable | Retryable |
| `errors[]` `complexity` | 400 | 36 | Retryable | Retryable | Retryable |
| `"code": "RATELIMITED"` exhausted | **400** | 35 | Retryable | Retryable | Retryable |
| `errors[]` unclassified | 400 | 34 | Retryable | **Terminal** | Retryable |
| 5xx exhausted | 5xx | 20 | Terminal | Terminal | Retryable |
| connect / DNS / timeout | — | 21 | Terminal | Terminal | Retryable |

⚠️ The 200-body `auth` row is **Retryable on update**, correcting an earlier
draft that made it Terminal. `_wiur_map_linear`
(`work-item-update-remote.sh:66-72`) lists code `11` in its retryable clause, and
the comment above it names only `34` as the terminal 200-body error. The plan's
own 401 and 400-auth rows — which also yield code 11 — are Retryable for update,
so the Terminal cell was internally inconsistent as well. Making it Terminal
would have changed a push failure's exit code from 70 to 71 and told the caller a
provably-unapplied auth rejection may have mutated the remote.

Rate limiting returns **HTTP 400** with `"code": "RATELIMITED"` in the body, so
the body is parsed on 400, not only on 200. Complexity rejection is detected by
the message containing `complexity` (`linear-graphql.sh:46`).

The divergence is ported as **two genuinely different policies**, not unified:
`34` is retryable on create and terminal on update — documented, because a
200-body error may mean the mutation applied — while `18, 23, 25, 27, 29` run the
other way with no rationale anywhere. `35` and `36` are retryable on both. Where
a policy and the "provably pre-send" rule disagree, the table wins.

The bash-code table is driven from Phase 2's fixture filtered to
`provider = linear`, covering both layers of the create path:
`_linear_map_no_file_failure`'s pre-send set → 108 → retryable, and everything
else → 109 → terminal.

### Success Criteria

#### Automated Verification

- [x] Every row of the status-and-body table is asserted per operation, including
      a `200` carrying an `errors` array and a `400` carrying
      `"code": "RATELIMITED"`
- [x] The 200-body auth row is Retryable on update, matching `_wiur_map_linear`
- [x] Every `provider = linear` row of the committed fixture is asserted, both
      layers of the create path included; the row-coverage guard fails on an
      unconsumed row
- [x] The differential test agrees with the running Linear mappers
- [x] Both directions of the divergence are asserted: `34` retryable on create
      and terminal on update; `18, 23, 25, 27, 29` the other way
- [x] A malformed token is refused, and a team-level `token_cmd` is refused with
      its diagnostic
- [x] Constructing a `LinearClient` leaves `rustls::crypto::CryptoProvider::get_default()`
      as `Some` (D7) — the same assertion Phase 3 makes, since Linear's
      `install_default()` call is a copy of Jira's and therefore the likelier to
      be omitted
- [x] The Linear transport carries the full Phase 3 bound set, asserted here
      because 6a is independently mergeable: 4 attempts on a persistent 503,
      `Retry-After` honoured as a duration, exactly one attempt on a connect, DNS
      or timeout failure, a 302 refused rather than followed, an oversized
      response rejected before deserialisation, and an injected timeout taking
      effect
- [x] The identifier fixture is exercised through `linear-client` as well, so
      both providers are held to the one shared rule
- [x] `team_id` resolves from `linear.team_id` when set and from
      `catalogue.json`'s `.team.id` when not, with precedence asserted — an
      already-onboarded user with no key configured does not hit exit 74
- [x] `cd cli && cargo nextest run -p linear-client`
- [x] Full local mirror: `mise run`

---

## Phase 6b: `linear-client` `impl RemoteTracker`

### Overview

The four port operations, `IssueFilter` composition, cursor pagination, the
corpus assertions, timeouts, the contract harnesses and registration — built on
the foundation 6a landed.

### Changes Required

#### 1. The port implementation

**File**: `cli/linear-client/src/lib.rs`
**Changes**: `impl RemoteTracker for LinearClient`, against the port's own
signatures — `show` returns `Result<RemoteIssue, TrackerError>`, never an
`Option`, for the reason Phase 5 states. Bodies pass through
verbatim — Linear is Markdown-native, so there is no conversion layer.
`RemoteIssue.body` comes from
`remote_projection::project(Integration::Linear, Op::Body, &value)`.

⚠️ An **empty-string** description projects as an empty line via `// ""`, where
Jira's absent description projects as the literal `null` via `// null`. The two
providers differ, and either wrong choice reclassifies every such item.

Identifier safety and the `port_body` newline adapter both come from
`tracker-support` (D9), as in Phase 5.

#### 2. Search, filter and pagination

**File**: `cli/linear-client/src/filter.rs`, `cli/linear-client/src/search.rs`
**Changes**: `fetch_all` issues
`query($cursor: String, $filter: IssueFilter, $first: Int) { issues(…) }` and
follows `pageInfo` cursors.

Linear's complexity is scored at 0.1 per property plus 1 per object, multiplied by
connection pagination whose default is **50**, with a hard rejection above 10,000
points — so an explicit `first:` is **always** passed, never omitted. `MAX_PAGES`
is 20; on hitting it the client sets a truncation flag and returns `Ok` with the
unseen ids `indeterminate`. The 250-issue bulk truncation resolves the same way,
which is what makes `unaccountable_id` nominable against a live tenant.

The four totality obligations Phase 5 states apply here unchanged; Linear's dedup
oracle is `_wifr_linear_keys`'s `| unique`.

⚠️ `_wifr_linear_keys` runs one **team-wide** search and treats anything missing
from a non-truncated result as `absent`. An id belonging to a different Linear
team therefore comes back unfound from a read the client believes was complete.
The plan resolves this explicitly: an id outside the configured team is
**`indeterminate`**, not `absent`, because the client has no evidence about an
issue it never had scope to see — and reporting it `absent` would unlink a live
issue. Both cases get a test.

The filter composer is driven from a committed fixture of (flags in, expected
`IssueFilter` JSON) under the same row-coverage guard as Jira's JQL fixture.
State-name-to-UUID resolution is injected behind a `StateResolver` port from the
outset — a fixed map here, the cache-backed implementation in Phase 9, with no
constructor change between them. `from_config` resolves every value eagerly into
owned data, since the boxed trait object is implicitly `'static`.

#### 3. Contract harness, corpus assertions, timeouts, registration

**File**: `cli/linear-client/tests/contract.rs`,
`cli/linear-client/tests/projection_corpus.rs`,
`cli/linear-client/tests/timeouts.rs`
**Changes**: As Phase 5, against the Linear records: `case-linear` for
byte-identity, and `case-linear-empty-description` and `case-linear-markdown` for
sha256-after-normalise. `impl ContractSubject` nominating an id truncated by the
250-item cap or the complexity cap, plus the offline
`tests/contract_offline.rs` running the same conformance functions against a mock
in the default profile — the enforcing route, with the live run as additional
assurance.

Timeout assertions take Phase 5's asymmetric shape: tight lower bound, 3×T upper
bound, timeout variant asserted, at T = 400ms and T = 1s. The operation deadline
gets the same slow-pages fixture.

Registration mirrors Phase 5: a `denied`-only pup rule
(`linear_client_is_the_only_linear_transport`) with a probe pair, `_ADAPTER` in
`_EXEMPT_MEMBERS`, and the workspace member entry.

### Success Criteria

#### Automated Verification

- [x] All four port operations implemented against the port's own signatures;
      `cd cli && cargo nextest run -p linear-client`
- [x] An empty `ids` request makes zero remote calls; duplicates are deduplicated
- [x] An id outside the configured team is `indeterminate`, not `absent`
- [x] The `IssueFilter` fixture covers every flag family under a row-coverage
      guard
- [x] Equality against `case-linear`'s line-reconstructed body, with the trailing
      newline asserted separately on `RemoteIssue.body`
- [x] sha256-after-normalise matches for `case-linear-empty-description` and
      `case-linear-markdown`
- [x] An empty-string description projects to an empty line, with no blank line
      before it and a trailing newline
- [x] Every GraphQL request carries an explicit `first:`, asserted via
      `MockServer::last_body`
- [x] The document construction is pinned by request-body assertions
- [x] Neither operation returns before T at T = 400ms and T = 1s, each returns
      within 3×T with a timeout variant; `fetch_all` returns `Ok`
      all-`indeterminate`
- [x] The operation deadline fires on the slow-pages fixture
- [x] Default timeout asserts 30s; `MAX_PAGES` asserts 20
- [x] The offline contract conformance run passes for `LinearClient` in the
      default profile
- [ ] The default profile selects no binary named exactly `contract`, and does
      select `contract_offline`
- [x] The pup probe pair passes: `mise run test:integration:pup`
- [x] Full local mirror: `mise run`

#### Manual Verification

- [ ] `mise run test:integration:tracker-contract` passes against a live Linear
      team
- [ ] The 250-item truncation genuinely produces `indeterminate` against the live
      tenant, not `absent`

---

## Phase 7: Composition Root

### Overview

Give the registry config access, resolve both real clients, flip the eight tests
that pin not-available, and add the import tripwire. This is the phase that makes
`accelerator work sync --integration jira` do something.

### Changes Required

#### 1. The registry

**File**: `cli/work-cli/src/tracker_registry.rs`
**Changes**: `ConfiguredTrackers` gains a lifetime and a config reference.
`resolve`'s signature is unchanged, because `&dyn ConfigAccess` is already in
scope at all three construction sites.

```rust
pub struct ConfiguredTrackers<'a> {
    config: &'a dyn ConfigAccess,
}

impl<'a> ConfiguredTrackers<'a> {
    pub const fn new(config: &'a dyn ConfigAccess) -> Self {
        Self { config }
    }
}

impl TrackerRegistry for ConfiguredTrackers<'_> {
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
        match name {
            "" => Err(SelectionError::Unset),
            "jira" => JiraClient::from_config(self.config)
                .map(|client| Box::new(client) as Box<dyn RemoteTracker>)
                .map_err(|error| SelectionError::Unconfigured {
                    name: name.to_owned(),
                    source: Box::new(error),
                }),
            "linear" => LinearClient::from_config(self.config)
                .map(|client| Box::new(client) as Box<dyn RemoteTracker>)
                .map_err(|error| SelectionError::Unconfigured {
                    name: name.to_owned(),
                    source: Box::new(error),
                }),
            "trello" | "github-issues" => {
                Err(SelectionError::NotAvailable {
                    name: name.to_owned(),
                })
            }
            other => Err(SelectionError::Unrecognised {
                name: other.to_owned(),
            }),
        }
    }
}
```

A fourth `SelectionError` variant is needed: a recognised, wired tracker whose
credentials or config are absent is neither `NotAvailable` (exit 72, "no client
wired yet") nor `Unrecognised` (73).

```rust
Unconfigured {
    name: String,
    source: Box<dyn std::error::Error + Send + Sync>,
},
```

The client's error is carried as a **boxed source**, not flattened to a `String`,
so the four failure categories — missing site, malformed site, failed credential
helper, shared-config refusal — stay distinguishable at the CLI and in tests.
`SelectionError::message` renders this arm from the source chain rather than from
a stored `detail` field.

⚠️ `Unconfigured` gets its **own exit code — 74 — not 71.** 70 and 71 are
`E_DISPATCH_RETRYABLE` and `E_DISPATCH_TERMINAL`, and in this taxonomy they
answer *whether a remote mutation may have applied*
(`cli/tracker/src/lib.rs:144-179`), not whether retrying is worthwhile.
`skills/work/scripts/work-item-bridge-codes.sh:13-19` defines 71 as "failure
AT/AFTER a mutation (request sent)", and
`skills/work/create-work-item/SKILL.md:555` instructs the agent to emit loud
non-idempotency guidance on it — warning the user a remote issue may already
exist. Unresolvable credentials are the canonical provably-no-mutation case:
nothing left the machine. Routing them to 71 would produce that alarming branch
for a purely local misconfiguration, and make it indistinguishable from a genuine
post-mutation failure. 70 and 71 stay derived exclusively from `TrackerError`.

⚠️ **74 is a shared contract, not a Rust constant.** It spans the Rust CLI, the
bash bridge, the SKILL.md dispatch tables, a parity test and a golden, and adding
it to a third of those is worse than not adding it at all — the consumer most
easily missed is the one that inverts the whole decision. The complete edit set:

**File**: `cli/work/src/sync/push_decide.rs` and its bash twin
`skills/work/scripts/work-item-push-decide.sh`, plus the golden
`skills/work/scripts/test-fixtures/work-item-push-decide.golden`
**Changes**: ⚠️ **This is the load-bearing one.** `push_decide.rs:51-63` routes
0, 70, 72 and 73 explicitly and sends every other code to
`PushOutcome::LoudTerminal`, on the stated reasoning that "a known terminal
failure and an unknown dispatcher code both mean a remote issue may exist". An
unhandled 74 therefore falls into exactly the alarming non-idempotency branch
that moving off 71 exists to prevent, and the registry would look correct while
the user-facing behaviour got worse. Add `74 => PushOutcome::LocalSave` to both
implementations and regenerate the golden.

**File**: `cli/work-cli/tests/exit_codes_parity.rs`
**Changes**: `:58-63` hard-asserts `bash.len() == 4` over the `readonly
E_DISPATCH_*` lines; the count becomes 5. The test also matches each stripped
bash name against a `pub const <NAME>: u8` in `exit_codes.rs`, so both spellings
are fixed here: `readonly E_DISPATCH_UNCONFIGURED=74` and
`pub const UNCONFIGURED: u8 = 74;`. A mismatch panics with a message about a
missing constant rather than about a naming disagreement, so naming it in one
place only is how that hour gets lost.

**File**: `cli/work-cli/src/cli.rs`, `cli/work-cli/tests/cli_sync.rs`
**Changes**: `cli.rs:78-87` is the `sync` help text and enumerates the codes by
hand; `sync_help_names_every_exit_code` (`cli_sync.rs:137-147`) asserts the help
names each of `0,1,2,4,5,70,71,72,73`. Both gain 74 — so `cli_surface.golden`
**does** move and is regenerated.

**File**: `skills/work/scripts/work-item-bridge-codes.sh`,
`skills/work/scripts/EXIT_CODES.md` (three tables plus the routing sections at
`:62` and `:71`), the header blocks of `work-item-update-remote.sh:29-30`,
`work-item-fetch-remote.sh:43-44` and `work-item-push-decide.sh:14-15`,
`skills/work/create-work-item/SKILL.md` and
`skills/work/sync-work-items/SKILL.md:53`
**Changes**: Add 74 with its meaning — configuration or credentials missing,
nothing sent — so the skill layer routes it to a "fix your config" branch rather
than a reconciliation branch when 0211 repoints.

**File**: `cli/tracker/tests/fixtures/dispatch-codes.txt`
**Changes**: Add `E_DISPATCH_UNCONFIGURED=74 above-the-port`. An earlier draft
excluded it on the grounds that 74 is a composition-root code rather than a
dispatch code — but that fixture's own header already describes 72 and 73 as
"dispatch-routing outcomes that resolve ABOVE the port", and pins both as
`above-the-port` rows. 74 is the same species, from the same registry match, so
excluding it would leave the one artefact claiming to pin the taxonomy's
membership silently incomplete.

This is permitted by the no-touch rule, which Phase 5 narrows to
`cli/tracker/src`, `Cargo.toml` and `public-api.txt`. `cli/tracker/tests/errors.rs`
is unaffected: its only numeric guard, `exactly_two_dispatch_codes_reach_the_port`,
counts rows tagged with a `TrackerError` class, and an `above-the-port` row
carries none.

**File**: `cli/work-cli/src/create.rs`, `cli/work-cli/src/update.rs`,
`cli/work-cli/src/sync.rs`
**Changes**: The new variant breaks three exhaustive matches — `create.rs:337`,
`update.rs:203-207` and `sync.rs:262-275` — each routing to its own outcome enum
and exit path. All three gain an `Unconfigured` arm, and each outcome enum gains
the corresponding case.

**File**: `cli/work-cli/src/main.rs`
**Changes**: Three sites — `:220`, `:270`, `:377` — change from
`&tracker_registry::ConfiguredTrackers` to
`&tracker_registry::ConfiguredTrackers::new(service)`. `service` is already bound
on the preceding line at each.

#### 2. The tests that pin not-available

**File**: `cli/work-cli/tests/cli_sync.rs`, `cli_update_push.rs`,
`cli_create_push.rs`
**Changes**: Five flips, not seven. `cli_sync.rs` asserts exit 72 at `:75`
(with its message assertion at `:77`), `:85`, `:93` and `:132` — but `:85` is
`trello_also_exits_72_not_73`, which must **stay** at 72, and the "stdin case"
is `:132` itself rather than a further site. So: `cli_sync.rs:75`/`:77`, `:93`
and `:132` flip; `:85` does not; plus `cli_update_push.rs:120` and the
`cli_create_push.rs` site.

They become: `trello` and `github-issues` still assert 72 unchanged; `jira` and
`linear` assert the `Unconfigured` diagnostic and exit 74 when no credentials are
configured.

⚠️ "No credentials configured" is **not** the default state of a test process.
The credential ladder puts the **environment first**, and these tests spawn the
binary with the inherited environment — so a developer or CI runner with
`ACCELERATOR_JIRA_TOKEN` or `ACCELERATOR_LINEAR_TOKEN` exported (exactly the
state this plan's own manual verification steps require) would resolve a real client, and `sync` would
make live network calls from the default suite. That directly contradicts Phase
10's network-free goal.

Each flipped test therefore scrubs the provider environment explicitly with
`Command::env_remove` and points any client that does resolve at an unroutable
base. The variables are `ACCELERATOR_JIRA_TOKEN`, `ACCELERATOR_JIRA_TOKEN_CMD`,
`ACCELERATOR_LINEAR_TOKEN`, `ACCELERATOR_LINEAR_TOKEN_CMD` and
`ACCELERATOR_ALLOW_INSECURE_LOCAL` — read off `jira-auth.sh:169-181` and
`linear-auth.sh:182-189`. The scrubbed set is derived from the crates' own
`TokenKeys` constants rather than written out again, so it cannot drift from the
ladder it guards, and that derivation is its own success criterion.

#### 3. The import tripwire

**File**: `cli/work-cli/tests/provider_isolation.rs`
**Changes**: ⚠️ A bare grep for `reqwest` cannot work. `cli/launcher` and
`cli/visualiser/server` both legitimately use it, the latter with its own
non-workspace entry carrying `json`. The tripwire is therefore an **allowlist**,
and it fails on a new offender rather than on the existing two.

It walks every `Cargo.toml`-bearing directory under `cli/`, honouring
`.gitignore`, and asserts:

- no `use reqwest` or `reqwest::` outside `PERMITTED_HTTP_CRATES`
- no `danger_accept_invalid_certs` or `danger_accept_invalid_hostnames` anywhere
- no `redirect::Policy::limited` or `::default` in the client crates
- no `std::env::var` inside `linear-client`'s attach and upload modules

The last three exist because each guards a control this plan added *in response
to a security review* and that no other test would notice disappearing: the
redirect refusal, the constructor-parameter loopback admission, and — paired with
the behavioural criteria — the response bound. A one-line edit could otherwise
remove any of them with a green build.

The matching key is the **workspace-relative crate directory path**, and the
allowlist is normalised to that form:

```rust
const PERMITTED_HTTP_CRATES: &[&str] = &[
    "jira-client",
    "linear-client",
    "launcher",
    "visualiser/server",
    "http-test-support",
];
```

⚠️ The walk root is every `Cargo.toml`-bearing directory under `cli/`, not
`cli/*/src/**/*.rs` — the latter never descends into `cli/visualiser/server/src`,
so that entry would be unreachable and the crate silently unguarded rather than
deliberately allowlisted, with the "tripwire passes on the tree as it stands"
criterion unable to tell the two apart.

The import rule is the **enforcing** mechanism, because it is structural. Two
further textual scans — a string literal containing `/rest/api/` or
`/rest/agile/` outside `jira-client`, and one whose first non-whitespace token is
`query ` or `mutation ` outside `linear-client` — are heuristics over source
text, not import analysis: a doc string, an error message or a fixture path trips
them. They are reported as warnings, scoped to non-test, non-doc lines, and their
message states the rule's intent — provider request construction belongs in the
provider client crate — so a future contributor can judge a false positive
instead of reaching for the cheapest fix, which is to weaken the guard.

The walk uses the tree-walking approach `shell_sources()` established rather than
`git ls-files`, which is blind inside a jj workspace.

The tripwire is itself tested: a sibling test writes a deliberate violation into
a temporary source tree rooted at a `TempDir`, runs the same check function over
it, and asserts it fails. Fixtures use `tempfile::TempDir` for self-cleaning,
per the pid-reuse convention.

#### 4. The sync engine constructed with a real client

**File**: `cli/work-adapters/tests/sync_run_real_client.rs` and
`cli/work-cli/tests/sync_resolves_real_client.rs`
**Changes**: The first acceptance criterion requires a test that constructs the
sync engine with each real client rather than relying on the crates compiling.
The existing `sync_run.rs` over `RecordingTracker` is the template.

⚠️ It has to be **two** tests, because one crate cannot reach both halves.
`ConfiguredTrackers` lives in `cli/work-cli/src/tracker_registry.rs` and the exit
codes in `cli/work-cli/src/exit_codes.rs`; `work-adapters` sits below `work-cli`
and cannot depend on it. Putting the whole thing in `work-adapters` would force
either an inverted dependency the pup rules forbid, or a silent re-scoping that
drops the registry and exit-code assertions — which were the point.

- `work-adapters`: drives each client directly against a `MockServer`, asserting
  the `remote_hash` and sync classification for a corpus item on a known payload,
  and that a truncated page leaves unseen ids `indeterminate` with nothing
  deleted.
- `work-cli`: resolves through `ConfiguredTrackers` against a fixed config naming
  `jira` then `linear`, asserting resolution succeeds and that a 401 yields the
  expected process exit code. It sits beside the existing `cli_sync.rs` suite and
  scrubs the provider environment variables as those tests do.

"Reaches the provider" is too weak an observable to assert. This seam is where a
`TrackerError` class becomes a sync classification, a `FetchOutcome` becomes
locally- or remotely-modified or indeterminate, and an exit code is chosen — the
one place bugs from this plan surface to a user — and a hit count catches none of
it. The test asserts end-to-end outcomes, for both providers:

- a mock returning a known payload yields the expected `remote_hash` and sync
  classification for a corpus item
- a mock returning 401 yields the expected process exit code
- a mock truncating a page leaves unseen ids `indeterminate` and deletes nothing

### Success Criteria

#### Automated Verification

- [ ] `ConfiguredTrackers::new(service)` compiles at all three sites; `mise run
      cli:check`
- [ ] Resolving `jira` with credentials configured returns a client, not an error
- [ ] Resolving `jira` with nothing configured returns `Unconfigured`, exit 74,
      with the client's error reachable through `source()` — a missing site, a
      failed helper and a shared-config refusal are distinguishable
- [ ] **74 decides `local-save` in `push_decide`, not `loud-terminal`**, in both
      the Rust and the bash twin, with the golden regenerated
- [ ] `exit_codes_parity.rs` passes with five `E_DISPATCH_*` constants
- [ ] `sync_help_names_every_exit_code` passes with 74 in the help text, and
      `cli_surface.golden` is regenerated
- [ ] Exit 74 is documented in `work-item-bridge-codes.sh`, `EXIT_CODES.md`, the
      four script headers and both SKILL.md dispatch tables; `dispatch-codes.txt`
      is deliberately unchanged and its header says why
- [ ] The flipped tests scrub the provider environment — `ACCELERATOR_JIRA_TOKEN`,
      `ACCELERATOR_JIRA_TOKEN_CMD`, `ACCELERATOR_LINEAR_TOKEN`,
      `ACCELERATOR_LINEAR_TOKEN_CMD`, `ACCELERATOR_ALLOW_INSECURE_LOCAL` — with
      the set derived from the crates' `TokenKeys` constants, so a credentialed
      machine cannot make them resolve a real client
- [ ] `trello` and `github-issues` still return `NotAvailable`, exit 72
- [ ] An unrecognised name still returns `Unrecognised`, exit 73
- [ ] All three exhaustive match sites handle `Unconfigured`
- [ ] All seven flipped assertions pass: `cd cli && cargo nextest run -p
      accelerator-work`
- [ ] In `work-adapters`, for both providers: the expected `remote_hash` and
      classification on a known payload, and `indeterminate` with no deletion on
      a truncated page
- [ ] In `work-cli`, for both providers: resolution through `ConfiguredTrackers`
      succeeds, and a 401 yields the expected process exit code
- [ ] The tripwire passes on the tree as it stands, and fails on a planted
      `danger_accept_invalid_certs`
- [ ] The tripwire **fails** on a planted violation, asserted by its sibling test
- [ ] `cli_surface.golden` is regenerated with 74 in the sync help text
- [ ] Full local mirror: `mise run`

#### Manual Verification

- [ ] `accelerator work sync` with `work.integration = jira` and real credentials
      syncs against a live tenant
- [ ] `create --push` and `update --push` against a live tracker, including the
      terminal-failure-that-nonetheless-succeeded shape
- [ ] The pending-push marker's crash-recovery path against a real interrupted
      create
- [ ] What a user sees when the tracker is unreachable, when it rate-limits, and
      when a fetch truncates — this phase is where those failure modes reach a
      user-invocable command for the first time, so each is exercised
      deliberately rather than discovered in the field

---

## Phase 8: Jira Provider Surface

### Overview

The four flows beyond the port: `comment` (4 request shapes), `transition`
(2 shapes plus id lookup), `attach` (multipart), and `init`'s discovery calls and
cache production. Additive library surface; nothing resolves it until 0211.

⚠️ This surface is deliberately **concrete**, with no trait analogous to
`RemoteTracker`, so 0211's composition root will `match` on provider name and
name both client types — coupling the tripwire cannot catch, since it greps for
transport symbols rather than provider names.

An accepted deferral, not an oversight: the right shape for a `comment` port is
only visible once a caller exists, and inventing one against the bash flow
scripts would ossify it around what was convenient to transcribe. 0211 owns it.
The concrete call sites 0211 will use are named per function as each is built.

### Changes Required

#### 1. Comment

**File**: `cli/jira-client/src/comment.rs`
**Changes**: Four shapes, transcribed from `jira-comment-flow.sh`:

| Operation | Endpoint | Source |
|---|---|---|
| add | `POST /rest/api/3/issue/{key}/comment` | `:193` |
| list | `GET /rest/api/3/issue/{key}/comment` | `:276` |
| edit | `PUT /rest/api/3/issue/{key}/comment/{id}` | `:461` |
| delete | `DELETE /rest/api/3/issue/{key}/comment/{id}` | `:538` |

Bodies pass through Phase 4's md-to-ADF assembler (`:154`); responses render back
through the ADF renderer (`:207`). The visibility `role:` / `group:` object is
carried over (`:167`).

`list` needs **offset pagination** with a 20-page cap (`:266-311`) — distinct
from `fetch_all`'s cursor pagination, because the comment endpoint uses
`startAt` / `maxResults`.

⚠️ `$EDITOR` and stdin body resolution (`:144`, `:187`) stay **out** of the
crate. Those are an interactive surface ADR-0045 forbids; the crate takes a body
string. The editor invocation stays in the skill layer for 0211.

#### 2. Transition

**File**: `cli/jira-client/src/transition.rs`
**Changes**: Two shapes, transcribed from `jira-transition-flow.sh`:
`GET /rest/api/3/issue/{key}/transitions` (`:84`) then
`POST /rest/api/3/issue/{key}/transitions` (`:377`).

The lookup is case-insensitive name matching with explicit zero-match and
ambiguous-match handling (`:79-116`). An optional comment folds into
`update.comment[].add` through the ADF assembler (`:328`, `:348`).
`notifyUsers=false` and the resolution field are carried over.

#### 3. Attach

**File**: `cli/jira-client/src/attach.rs`, `cli/jira-client/Cargo.toml`
**Changes**: `POST /rest/api/3/issue/{key}/attachments` as
`multipart/form-data`, with `X-Atlassian-Token: no-check`
(`jira-request.sh:315-320`) and one part per file. Multiple files in one request.

The body is hand-rolled per D6 — a boundary string plus per-part headers over the
byte-body path the transport already has, keeping `mime` and `mime_guess` out of
the closure and leaving `deny:check` and `test_launcher_feature_graph.py`
untouched. The MIME type per part comes from the same hand-rolled sniffer Phase 9
needs.

⚠️ **Replacing a vetted encoder means restating its safety contract, not just its
output shape.** Injection safety is most of what `reqwest/multipart` provides,
and a byte-identity test against a benign fixture would not notice its absence.
The encoder's contract, each clause with an adversarial test:

- The boundary carries at least 64 bits of entropy from a CSPRNG and is
  **verified absent from every part body** before the request is built; a
  collision otherwise truncates or splits the upload, driven by file content
  rather than by the caller
- Filenames reaching `Content-Disposition: form-data; name="file"; filename="…"`
  are quoted-string escaped, and any filename carrying `"`, CR, LF or a control
  byte is **refused** outright rather than escaped — a filename like
  `a"\r\nContent-Type: text/html\r\n\r\n<script>` otherwise injects part headers
  and can smuggle an additional part into an authenticated upload
- Part names are fixed constants, never caller-supplied
- The sniffed content type is emitted from a closed set, never echoed from input

File pre-checks are carried over (`:115-144`): existence, a symlink-to-device
refusal, readability, and a size warning at 10 MB.

⚠️ The bash performs these as separate stat-then-open steps, which is a TOCTOU
window: between the symlink check and the open, the link can be repointed at
`~/.ssh/id_ed25519` or `.accelerator/config.local.md` and the secret uploaded to
the tracker. The Rust opens the file **once** and runs the device and size checks
on that handle's metadata — no re-stat, no re-open. Attachment paths are also
confined to the repository root, so a skill-supplied relative path cannot address
arbitrary readable files.

#### 4. Init discovery

**File**: `cli/jira-client/src/discovery.rs`
**Changes**: Three shapes, transcribed from `jira-init-flow.sh` and
`jira-fields.sh`: `GET /rest/api/3/myself` (`:98`),
`GET /rest/api/3/project` (`:138`), `GET /rest/api/3/field`
(`jira-fields.sh:53`).

The crate returns the **cache shapes** as values — `site.json`'s
`{site, accountId}`, `projects.json`, `fields.json` — and does not write them.
Atomic writes, the advisory lock (`jira_with_lock`, `:157`), `.gitignore` and
`.gitkeep` upkeep, and the `accelerator config work` shell-out are composition
concerns, so they land in a small `cli/jira-client/src/cache.rs` taking an
injected filesystem port rather than reaching for `std::fs` inside the transport
module.

That port is **not** newly invented here. The workspace already has atomic-write
and mkdir-lock primitives whose on-disk `owner.<nonce>` /
`reclaiming.<pid>.<nonce>` sentinel contract is shared with
`scripts/atomic-common.sh`; `cache.rs` delegates to them.

⚠️ Building the write path here rather than deferring it to 0211 — as
`comment` and `transition` are deferred, on the argument that the right shape is
only visible once a caller exists — needs its own justification, and this is it:
a second, independent implementation of that locking contract is a silent
corruption path for as long as both the bash and the Rust `init` can run against
the same cache directory, which is precisely the 0210-to-0211 window. Deferring
the writer means 0211 writes one under time pressure beside a bash writer still
in service. The deferral reasoning applies to shapes with no concurrency
hazard; this one has one.

The write path is tested, not just the returned shapes, against a fake
filesystem port: a write is atomic under an injected mid-write failure (no
partially-visible file), a held lock produces a typed contention error rather
than a clobber, and `.gitignore` / `.gitkeep` upkeep is idempotent across two
runs. A corrupted `fields.json` or `catalogue.json` breaks search and transition
resolution, so the half the port exists to make testable is tested.

⚠️ The interactive `work.default_project_code` prompt (`:189-191`) is **not**
ported. ADR-0045 forbids a client prompting. It stays in the skill layer for 0211
to wire, and the crate exposes `list_projects()` so the skill can render choices.

### Success Criteria

#### Automated Verification

- [ ] All four comment shapes assert their exact method, path and body via
      `MockServer::last_body` and `last_query`
- [ ] Comment `list` stops at 20 pages and reports truncation
- [ ] Transition resolves a name case-insensitively; zero matches and ambiguous
      matches each return a distinct typed error
- [ ] A transition comment reaches `update.comment[].add` as ADF
- [ ] Attach sends `multipart/form-data` with `X-Atlassian-Token: no-check`,
      asserted on the captured headers, and one part per file
- [ ] A symlink-to-device path is refused; a missing file is refused; a path
      outside the repository root is refused
- [ ] The device and size checks run on a single open handle — no re-stat
- [ ] `test_launcher_feature_graph.py` passes **unchanged**, and `multipart`,
      `mime` and `mime_guess` are all absent from the graph:
      `mise run test:integration:deny`
- [ ] `deny:check` green with no new crate in the closure:
      `mise run lint:cli:deny:check`
- [ ] `rustls-native-certs` still absent from the launcher feature graph
- [ ] The hand-rolled multipart body is byte-asserted against a known fixture,
      including the boundary and per-part headers
- [ ] A filename carrying `"`, CR, LF or a control byte is **refused**, not
      escaped, and no part header can be injected through it
- [ ] A file whose bytes contain the candidate boundary forces a fresh boundary
      rather than a truncated body
- [ ] All three discovery calls assert their endpoints; the returned cache shapes
      match the committed golden JSON
- [ ] The cache write path holds against a fake filesystem: atomic under an
      injected mid-write failure, typed contention error on a held lock,
      idempotent `.gitignore` / `.gitkeep` upkeep
- [ ] `cache.rs` uses the existing lock primitives, asserted by the sentinel
      names it writes matching the shared contract
- [ ] No `std::fs` in the transport or discovery modules, enforced by extending
      the crate's pup rule
- [ ] Full local mirror: `mise run`

#### Manual Verification

- [ ] Attach a real file to a live Jira issue and confirm it appears
- [ ] Run discovery against a live tenant and diff the produced cache shapes
      against what `jira-init-flow.sh discover` writes

---

## Phase 9: Linear Provider Surface

### Overview

Linear's four flows. `comment` and `transition` are one request shape each;
`attach` is the most expensive item in this plan — a three-step upload with a
second, unauthenticated transport and a trust boundary around a server-supplied
URL.

### Changes Required

#### 1. Comment and transition

**File**: `cli/linear-client/src/comment.rs`,
`cli/linear-client/src/transition.rs`
**Changes**: One mutation each.

```
mutation($input: CommentCreateInput!) { commentCreate(...) }
    linear-comment-flow.sh:126-128
mutation($id: String!, $input: IssueUpdateInput!) { issueUpdate(...) }
    linear-transition-flow.sh:149-151
```

No conversion layer — Linear comment bodies are Markdown-native
(`linear-comment-flow.sh:14`). `transition` reuses the same `issueUpdate`
mutation the port's `update` uses, with a different input field, and resolves a
state name to a UUID through the catalogue (`:105-134`).

#### 2. Attach

**File**: `cli/linear-client/src/attach.rs`
**Changes**: Two modes. Link mode is one mutation:
`mutation($input: AttachmentCreateInput!) { attachmentCreate(...) }` (`:272-274`).

Binary mode is three steps (`:340-345`, `:172-175`, `:382-384`):

1. `mutation($contentType: String!, $filename: String!, $size: Int!) {
   fileUpload(...) }` returns an `uploadUrl` and echoed headers
2. A raw `PUT` of the file bytes **to that URL**, which is not a Linear API
   endpoint
3. `attachmentCreate` with the resulting `assetUrl`

⚠️ Step 2 needs a **second transport** with a deliberately different policy,
because it sends bytes to a host the server nominated:

| Property | Port transport | Upload transport |
|---|---|---|
| Authorization | bearer token | **none** |
| Redirects | refused | **refused** (`--max-redirs 0`) |
| Timeout | 30s | **60s** (`linear-attach-flow.sh:172`) |
| Protocol | https | **pinned** via `--proto` |
| Body | JSON string | raw binary |
| Response bound | 8 MiB | 8 MiB + `Content-Length` sanity check |

The response bound is stated for the upload transport deliberately: it is the one
request in the plan that talks to a host the *server* nominated, so leaving the
least-trusted transport as the only unbounded one would invert the trust
gradient.

The trust boundary is carried over in full, and three of its properties are
easily lost in summary — each is a deliberate guard in
`linear-attach-flow.sh`, not an incidental detail:

- **Host allowlist** (`_attach_upload_url_ok`, `:99-120`): `uploads.linear.app`
  or any `*.linear.app`, matched at a **label boundary**. A naive
  `ends_with(".linear.app")` accepts `uploads.linear.app.evil.com` and
  `evil-linear.app`, which is exactly the SSRF the bash guard was written to
  close.
- **`assetUrl` is validated too**, not only `uploadUrl` (`:358-368`), and before
  any bytes move. An unvalidated `assetUrl` lets a compromised response point the
  attachment record at an attacker host.
- **URLs are redacted in every diagnostic** (`_attach_redact_url`, `:94-97`):
  scheme, host and path only, query stripped. The pre-signed query carries a
  short-TTL bearer-grade capability; unredacted it leaks into logs and terminal
  scrollback.
- Echoed-header allowlist restricted to `x-amz-*`, with a CRLF-injection filter
  (`:139-160`)
- A bounded 3-attempt retry (`:169-182`)
- MIME sniffing (`file -b --mime-type` in bash, `:318`) reproduced with a
  hand-rolled sniffer rather than a new dependency, so `deny:check` is unaffected
- Non-atomic three-step failure semantics, including the orphaned-asset error
  (`:392`) when step 3 fails after step 2 succeeded, emitted as a `warn` through
  `tracing` as well as returned, since it leaves remote state a later run must
  reconcile

⚠️ **The loopback exemption is a constructor parameter, never an environment
read.** The bash implements the test escape hatch as `ACCELERATOR_TEST_MODE=1`;
carried into a compiled binary that turns the allowlist from a security control
into an advisory one, since any hostile repo hook, wrapper script or CI step that
can set an environment variable disables SSRF protection entirely. The upload
transport admits loopback only when constructed with it enabled, which only tests
do.

The allowlist is driven from a committed fixture of adversarial URLs —
`uploads.linear.app.evil.com`, `evil.com/?x=uploads.linear.app`, a userinfo `@`
form, an IDN homoglyph, `http://`, an IPv6 literal — each with its expected
accept or reject, because an allowlist tested only against cases the
implementation already handles tends to pass while the interesting ones go
unwritten. The sniffer gets its own table of (leading bytes → expected MIME)
including the unknown-type fallback; an untested sniffer sends a wrong
`contentType` to `fileUpload` and the attachment renders incorrectly with nothing
to catch it.

Note this PUT is the one place the 60s figure belongs. It is **not** a port
operation, so it does not contradict the 30s port default asserted in Phase 6a.

#### 3. Init discovery

**File**: `cli/linear-client/src/discovery.rs`, `cli/linear-client/src/cache.rs`
**Changes**: Three queries, transcribed from `linear-init-flow.sh`:
`query { viewer { id name } }` (`:118`),
`query { teams { nodes { id name key } } }` (`:151`), and
`query($id: String!) { team(id: $id) { … states { nodes { id name type position
} } } }` (`:176`).

Returns `viewer.json` and `catalogue.json` as shapes; writing, locking and
gitignore upkeep go through the same injected filesystem port and the same
existing lock primitives Phase 8 uses, with the same write-path tests. The
interactive team selection (`:254`) is **not** ported — `list_teams()` is exposed
so the skill can render choices.

### Success Criteria

#### Automated Verification

- [ ] The comment and transition mutations assert their exact document text and
      variables via `MockServer::last_body`
- [ ] A state name resolves to a UUID through the catalogue; an unknown name is a
      typed error
- [ ] Link-mode attach sends one `attachmentCreate`
- [ ] Binary mode makes exactly three requests in order, asserted with
      `MockServer::hits`
- [ ] The upload PUT carries **no** `Authorization` header, asserted on captured
      headers
- [ ] Every row of the adversarial-URL fixture resolves as expected, including
      `uploads.linear.app.evil.com` and `evil-linear.app` being **refused** —
      label-boundary matching, not suffix matching
- [ ] A non-`*.linear.app` `uploadUrl` is refused before any bytes are sent
- [ ] A `uploadUrl` on a non-https scheme is refused
- [ ] An `assetUrl` on a foreign host is refused, before step 2
- [ ] No diagnostic retains a URL query string, asserted on the error text
- [ ] A loopback `uploadUrl` is refused through the production constructor
      regardless of process environment
- [ ] The MIME sniffer matches its table, including the unknown-type fallback
- [ ] An echoed header outside `x-amz-*` is dropped; one containing CRLF is
      refused
- [ ] A 30x response to the PUT is refused rather than followed
- [ ] The upload timeout asserts 60s while the port transport still asserts 30s
- [ ] A step-3 failure after a successful step 2 produces the orphaned-asset
      error, not a generic failure
- [ ] All three discovery queries assert their documents; the produced cache
      shapes match committed goldens
- [ ] `deny:check` green — no MIME crate was added:
      `mise run lint:cli:deny:check`
- [ ] Full local mirror: `mise run`

#### Manual Verification

- [ ] Upload a real binary file to a live Linear issue and confirm the attachment
      resolves
- [ ] Run discovery against a live team and diff `catalogue.json` against what
      `linear-init-flow.sh discover` writes

---

## Phase 10: Enforcement Close-Out

### Overview

Answer the copyleft question reproducibly, commit the contract evidence, and
prove the default suite makes no network call. Verification only — no new
behaviour.

### Changes Required

#### 1. The licence answer

**File**: `cli/licence-audit/new-trees.txt`
**Changes**: The committed verbatim output of a named reproducible command, so a
verifier can re-run it rather than trust a summary:

```bash
cd cli && cargo deny list --layout crate --format human
```

The file records the cargo-deny version in its header, and the **enforcing**
assertion is not a byte diff. Diffing the full listing is sensitive to the
lockfile, to the five configured `[graph].targets` and to the cargo-deny version,
none of which are frozen against it — so any unrelated dependency bump reds a
check that carries no licence information, and the reflexive fix is to regenerate
it, which defeats its purpose. The test asserts the invariant that matters: the
licence **set** across the closure is a subset of `cli/deny.toml`'s allow-list
plus its declared exceptions. The listing stays as committed human-readable
evidence beside it.

The expectation, from the closure as it stands: `reqwest`, `rustls` and
`hickory` are already present via `launcher`, and `ISC`, `BSD-2-Clause`,
`BSD-3-Clause` and `Zlib` are already in `cli/deny.toml`'s allow-list — the very
licences its comment anticipates. Because the multipart body is hand-rolled,
`mime` and `mime_guess` never enter the closure, so the production side of this
plan adds **no** new crate and no new SPDX id at all.

The answer to "does either tree carry copyleft" is therefore expected to be
**no**, and the committed output is what proves it. If it is unexpectedly yes,
any copyleft goes in `[[licenses.exceptions]]` with a per-crate justification —
**never** the blanket allow — and 0203's attribution artefact becomes a
release-path dependency, which means adding 0203 to 0211's `blocked_by` per
0211's trigger.

Because extracting the mock servers added no dev-dependency, the dev-tree side of
the question needs no separate answer beyond this listing — and the listing
covers it, since `cli/deny.toml` sets no `exclude-dev` and evaluates dev trees
across five targets under `unmaintained = "all"`.

#### 2. Contract evidence

**File**: `cli/jira-client/tests/evidence/contract-run.txt`,
`cli/linear-client/tests/evidence/contract-run.txt`
**Changes**: A **reduced** record of the harness run for both providers, dated no
earlier than the final client commit, produced by `mise run
test:integration:tracker-contract` against the credentialed target.

⚠️ Not a raw transcript. A verbatim run against a live tenant exercising `show`
and `fetch_all` carries real issue keys, summaries, ADF bodies, account ids,
email addresses and site hostnames — and potentially an echoed `Authorization` in
a failure diagnostic — into the repository permanently, with the obvious
disclosure and data-protection consequences. The harness emits the reduced form
directly: test name, pass or fail, counts and duration, no payloads.

The guard is **structural first, denylist second**: it asserts the committed
files contain only that reduced field set, so an unknown or rotated token format
cannot appear at all, and additionally that no secret-shaped pattern (`ATATT`,
`lin_api_`, `Bearer `, an address-shaped token) is present. A denylist alone
would pass anything whose prefix nobody thought of.

The continuously-enforcing route is the offline conformance run added in Phases 5
and 6b, which the default profile selects. This evidence is the live-tenant
assurance beside it, not the gate.

#### 3. Network-free default suite

**File**: `cli/work-cli/tests/no_network_by_default.rs`
**Changes**: The criterion requires this be verified by *running* the default
suite in a network-disabled environment, not by reading the filter expression. A
documented procedure plus a committed transcript is what satisfies it; the test
itself asserts the weaker mechanical property that no `contract` binary is
selected by the default profile.

#### 4. Decisions recorded on the parent

**File**: `meta/work/0171-jira-and-linear-integrations.md`
**Changes**: Copy this plan's `## Decisions` register (D1-D16) onto the work
item, verbatim and without re-argument. The register is the single home for every
cross-cutting rationale in this plan; restating it here in different words is how
the two copies diverge.

Add alongside it only what the register cannot know: the copyleft answer with the
committed listing's path, and the date of the contract evidence runs.

### Success Criteria

#### Automated Verification

- [ ] `deny:check` green with any allowance committed:
      `mise run lint:cli:deny:check`
- [ ] The licence set across the closure is a subset of `cli/deny.toml`'s
      allow-list plus its declared exceptions, asserted as a set rather than as a
      byte diff of the listing
- [ ] No binary named exactly `contract` is selected by the default profile,
      asserted by exact name rather than by a substring pattern
- [ ] `cli/tracker/tests/fixtures/public-api.txt` unchanged and
      `cli/tracker/Cargo.toml` still declares no dependencies, asserted by
      `cli/tracker/tests/structure.rs` passing untouched
- [ ] No Python in `cli/`'s dev-dependencies:
      `rg -n "python" --glob 'cli/**/Cargo.toml'` returns nothing — recursive, so
      nested members like `cli/visualiser/server` are actually scanned
- [ ] The committed evidence files carry no payloads and match no secret-shaped
      pattern, asserted by the guard
- [ ] Both client crates classified: `mise run test:unit:build-system`
- [ ] Every pup rule added by this plan has a probe pair:
      `mise run test:integration:pup`
- [ ] Full local mirror green end-to-end: `mise run`

#### Manual Verification

- [ ] The default suite runs green with networking disabled, and the transcript
      is committed
- [ ] Both evidence files are dated no earlier than the final client commit
- [ ] 0171's `## Decisions` reads as a record a later reader can act on without
      rediscovering `gouqi` or v2

---

## Testing Strategy

### Unit Tests

- **Auth precedence**, per provider: every branch, mirroring
  `collaboration-cli/src/auth.rs`'s nine tests. The team-level `token_cmd`
  refusal asserts on the message text, not just the variant.
- **Classification**, per provider and per operation: table-driven from the
  committed fixture and from the status table, with a row-coverage guard that
  fails the build on an unconsumed row. The specific failure this guards is the
  reviewer's: "a client that misclassifies an auth failure as retryable passes
  every criterion".
- **ADF conversion**: every node type and mark type in the inventory, both
  placeholder positions, all four rejections, and each deliberate lossy
  behaviour asserted as intentional.
- **Projection**: equality against the three `project-remote` records'
  line-reconstructed bodies, the trailing newline asserted on `RemoteIssue.body`;
  sha256-after-normalise against the four `sync-baseline` records; key-order
  invariance by comparing `case-jira` and `case-jira-reordered` to each other.
- **Identifier safety**: the four malformed shapes as `Terminal`; `/`, `#`, `@`
  mid-token accepted.
- **Timestamps**: absent, `null` and empty-string to `NotReported`; a populated
  stamp to `Reported` with bytes unaltered, including `+0000`.

### Integration Tests

- **Transport behaviour** against `cli/http-test-support`: retry counts,
  `Retry-After` honouring, page caps, request-body assertions for JQL and
  GraphQL documents, header assertions for `X-Atlassian-Token` and the absent
  upload `Authorization`.
- **Differential against the running bash**, while it exists: the five exit-code
  mappers over every code 0-130, and the ADF pipeline in both directions over the
  fixture corpus. Gated on bash availability, not credentials, so it runs by
  default. This is what stops a transcription error from producing a
  mutually-consistent fixture-plus-code pair that every other guard passes.
- **Contract conformance offline**, against a mock, in the default profile, for
  both providers — partition totality, read-never-terminal, create/show
  round-trip, and unaccounted-is-indeterminate.
- **Timeouts**: `Route::Stall` at T = 400ms and T = 1s, asserting each call does
  not return before T, returns within 3×T, and fails with a timeout variant. The
  lower bound carries the signal; a tight upper bound was rejected because 140ms
  of slack at T = 400ms is inside scheduler jitter on a loaded runner. T = 200ms
  was rejected outright.
- **Retry and backoff without sleeping**: the delay sequence asserted as data
  through an injected clock and seeded jitter, so `Retry-After: 7` is shown to
  yield 7s rather than merely to trigger a retry, and the suite runs in
  milliseconds.
- **The tripwire**, tested by planting a violation in a `TempDir`-rooted tree.
- **The sync engine with real clients**, resolved through `ConfiguredTrackers`
  against a mock base URL, asserting `remote_hash`, classification, exit codes
  and the no-deletion-on-truncation property rather than that a request happened.
- **Adversarial fixtures** where a guard's value depends on the cases nobody
  thought to write: the upload-URL allowlist, JQL identifier escaping, and the
  `serde_json`-versus-`jq` scalar table.
- **The contract harness**, gated by the `binary(=contract)` filter and the
  independent `ACCELERATOR_TRACKER_CONTRACT` variable the harness enforces by
  erroring rather than skipping. Any new gated entry point must be added to
  `gated_calls()` (`cli/tracker-test-support/src/contract.rs:249-267`) or the
  gate-closure guard silently lapses.

### Manual Testing Steps

1. Configure `jira.site`, `jira.email` and `jira.token` in
   `.accelerator/config.local.md`; run `mise run
   test:integration:tracker-contract` and capture the output.
2. Repeat for `linear.token` and `linear.team_id`.
3. Run `accelerator work sync` against each live tracker and compare the
   resulting `remote_hash` values against what the bash bridge produces for the
   same items — the corpus criterion is offline, but this is the check that the
   live payload also lands on the recipe.
4. `create --push` and `update --push` against a live tracker, including the
   terminal-failure-that-nonetheless-succeeded shape.
5. Interrupt a create mid-flight and confirm the pending-push marker's
   crash-recovery path.
6. Attach a real file to a live Jira issue and a real binary to a live Linear
   issue.
7. Run discovery against both live tenants and diff the produced cache shapes
   against the bash `init` output.
8. Disable networking and run the default suite; confirm green and commit the
   transcript.

## Performance Considerations

Each **request** is bounded at 30s with 4 attempts and jittered backoff capped at
60s, so a worst-case single request is roughly 30s plus three backoffs — matching
the bash bridge exactly, not a regression. The **operation** is bounded
separately by the deadline in D15, because the page cap bounds result size rather
than time.

`fetch_all` chunks Jira ids 50 at a time and caps at 20 pages; Linear always
passes an explicit `first:` to keep complexity below the 10,000-point rejection,
scored at 0.1 per property plus 1 per object. Both cap-hits set a truncation flag
and return `Ok`, which is what routes unseen ids to `indeterminate` rather than
failing the whole sync.

The clients are `reqwest::blocking`, matching the port's synchronous
dyn-compatible design. No async runtime enters the production graph.

## Migration Notes

Nothing migrates in this plan. The corpus under
`skills/work/scripts/test-fixtures/` is read but never written, and both its
directories must survive until 0212 — `work-item-project-remote/` came from
**0170**, `work-item-sync-baseline/` from **0194**, contrary to 0210's earlier
attribution of the whole corpus to 0194.

The ordering obligation towards the siblings is stated per bash asset in the
Implementation Approach above, not per phase. A phase-level gate naming only
Phases 1, 2, 4 and 5 would let 0211 legitimately delete `linear-graphql.sh`, the
comment, attach and init flows, and `linear-attach-flow.sh` — destroying the only
reference for behaviour whose Rust counterpart lands in Phases 6b, 8 and 9, and
taking the differential tests' oracle with it.

## References

- Original work item: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`
- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Research: `meta/research/codebase/2026-08-17-0210-provider-client-crates-over-the-tracker-port.md`
- Sibling research: `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- Port research: `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- 0194 validation: `meta/validations/2026-08-13-0194-tracker-crate-and-remote-sync-engine-validation.md`
- ADRs: ADR-0045 (non-interactive), ADR-0046, ADR-0053 (HTTP over rustls)
- The frozen port: `cli/tracker/src/lib.rs:105-343`
- Auth template: `cli/collaboration-cli/src/auth.rs:43-91`
- Projection recipe: `skills/work/scripts/work-item-project-remote.sh:65-93`
- Existing Rust projection: `cli/work-adapters/src/project_remote.rs:41-75`
- Contract harness: `cli/tracker-test-support/src/contract.rs:23-34,249-267`
- The gate: `cli/.config/nextest.toml`, `tasks/test/integration.py:163-170`
