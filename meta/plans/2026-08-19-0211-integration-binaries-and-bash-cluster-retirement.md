---
type: plan
id: "2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
title: "Integration Binaries and Bash Cluster Retirement Implementation Plan"
date: "2026-08-19T02:05:51+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0211"
parent: "work-item:0211"
derived_from: ["codebase-research:2026-08-17-0211-integration-binaries-and-bash-cluster-retirement"]
relates_to: ["work-item:0171", "work-item:0210", "work-item:0212", "work-item:0165"]
tags: [rust, jira, linear, integrations, cli, cutover, exit-codes, registration]
revision: "9d9c07ed56c8125e97430d0a0e731151723e63f2"
repository: "accelerator"
last_updated: "2026-08-19T09:35:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Integration Binaries and Bash Cluster Retirement Implementation Plan

## Overview

Ship `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
over 0210's `jira-client` / `linear-client` crates, repoint the sixteen jira and
linear `SKILL.md` bodies at them, register both dispatch tokens end to end, then
delete both bash script clusters, their suites, their Python mock servers and
their shared test assets.

The work is organised as **two independently-mergeable provider tracks**, Linear
leading, each carrying its own binary, cutover and deletion. Because the
integrations suite floor is a single shared count, the first track to land
decrements it and the second removes it outright.

## Current State Analysis

0210 (merged, PR #70) absorbed the entire provider surface into the client
crates, so the binaries are genuinely thin: parse args, assemble a credential
context, call a client method, render JSON or an error. The clusters this child
deletes are dead weight on the skill path the moment their bodies are repointed —
their only remaining consumers are the nine `skills/work/scripts/` call sites,
which **0212 removes first** (this child is blocked by 0212).

Verified at revision `9d9c07ed` by five research sweeps:

- **The client surface is complete and unpinned.** Both crates expose every
  flow (comment/transition/attach/discovery/search) and are `cargo-public-api`
  **exempt** (`_ADAPTER`, `tasks/public_api.py:52-59`) — the binaries bind
  whatever the crates expose, with no snapshot to maintain on the clients.
- **Two error conventions must be bridged.** Port ops (`create`/`update`/`show`/
  `fetch_all`) return `tracker::TrackerError` (only `Retryable`/`Terminal`)
  whose numeric bash code is computed by each crate's `classify.rs`; the
  port-less flows return a `SurfaceError` carrying `E_*` symbolic codes
  (`cli/{jira,linear}-client/src/surface.rs`). The binary obtains the port-op
  integer by receiving the `classify::Outcome` from the client's port-op path
  and calling the existing `bash_code(Outcome) -> u16` (Decision 9) — never by
  parsing `TrackerError.detail` — and owns only the `SurfaceError`-variant →
  integer mapping.
- **The base-URL test seam does not exist in the crates.** Neither reads
  `ACCELERATOR_*_API_URL`. Linear's `Transport::new(endpoint, …)` takes an
  explicit endpoint (`cli/linear-client/src/transport.rs:91`); Jira derives it
  from `credentials.base`. Each binary adds the env→constructor plumbing itself
  and bypasses `from_config` (via `LinearClient::new` / `JiraClient::new`).
- **No mutation-payload compose seam exists.** The only public composers are
  `jql::compose` and `filter::compose` (search). Mutation bodies are built
  inside the client methods and never exposed — so the write flows' wire-payload
  preview cannot be reproduced byte-for-byte (see Decisions).
- **The bash exit-code landscape is asymmetric.** Jira skills branch on ~45
  distinct integers; the Linear skills cite exactly one (`107` at
  `create-linear-issue/SKILL.md:92`) and are otherwise symbolic `E_*` names.
  Jira's `EXIT_CODES.md` is prose-only and **already wrong** (`:12` says usage
  errors exit `2`, `jira-request.sh:207` exits `1`); Linear's is machine-pinned
  by `test-linear-paths.sh:71-103` but only over the eight flow scripts.
- **Seven deletion tripwires** fire on removal, all verified with current line
  numbers (see Phase 2 and Phase 4). The `mise.toml` integrations leaf is
  `369-372` (the research's `:350-353` is now the `decisions` block); the
  `test_mise.py` `_LAUNCHER_DEPENDENTS` entry is line `55`.

## Desired End State

`accelerator-jira` and `accelerator-linear` are registered dispatched
sub-binaries; every jira and linear `SKILL.md` body invokes `accelerator jira …`
/ `accelerator linear …`; no `SKILL.md` anywhere declares `jq`, `curl` or a
jira/linear `scripts/` grant; both bash clusters, their suites, their mock
servers and their data assets are gone; the integrations floor, its task, its
four dependents, the seven `SHELL_LIBRARIES` entries, `_DUAL_USE_SCRIPTS` and
the mock-server tripwire are retired; and `mise run` exits 0 end to end.

Verification: `mise run` green at the second track's merge boundary, plus the
recorded artefacts under `meta/inventories/` and the `## Decisions` updates in
0171.

### Key Discoveries

- **Registration is the `design`-token diff minus the domain work.** The client
  crates already carry their `cli/pup.ron` rules, probe pairs and public-API
  classification (0210). What is new is two composition-root `*-cli` crates plus
  the token wiring: `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py:29-37`),
  `_SUBBINARY_DESCRIPTIONS` + the exact-tuple pin
  (`tests/integration/tasks/test_github.py:36-51`, `:533-541`),
  `_SUBBINARY_MANIFESTS` with a **custom `cli/jira-cli/Cargo.toml` path**
  (`tasks/manifest.py:55-65` — the crate is not at `cli/jira/`),
  `_CLI_RELEASE_BINARIES` (`tasks/build.py:36-45`), `cli/Cargo.toml` members +
  `Cargo.lock`, `.gitignore` (`bin/{jira,linear}-*`), `public_api.py`
  `_EXEMPT_MEMBERS` for the two `-cli` crates, and docs.
- **The same-commit rule binds registration to repointing.**
  `tasks/shared/dispatch_coherence.py` fails a registered token with no witness
  (`:196-202`) and a skill invoking an unregistered token (`:210-216`);
  `tasks/README.md:637-640` requires checklist points 1, 2, 3, 4, 7 and 8
  together.
- **Only six skills can witness a token.** The ten write skills declare bare
  `Bash`, disqualifying them (`dispatch_coherence.py:57-65`). The six read/init
  skills carry path-scoped rules and are the only candidates.
- **`http-test-support` is the inherited harness.** Both client crates already
  dev-depend on it (`cli/{jira,linear}-client/Cargo.toml`); the `*-cli` crates
  add it too. `Route::Sequence` is the Linear-all-POST-to-`/graphql` workaround;
  `Route::Stall` exercises read-timeout paths (`cli/http-test-support/src/
  lib.rs`).
- **The exit-code parity anchor must be a committed fixture.** The bash source
  is deleted, so the only enforcement model that survives is `work-cli`'s
  `exit_codes.rs` + a textual parity test against a pre-deletion fixture
  (`cli/work-cli/src/exit_codes.rs`, `cli/work-cli/tests/exit_codes_parity.rs`).
  0210 already committed the classify-layer half at
  `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt` (74 rows).
- **`main.rs` has two house shapes, and they do not compose.**
  `corpus-cli`/`collaboration-cli` use `Result<Outcome, kernel::Error>` + a
  shared `report` collapsing to `Refusal→2` / `1`; `work-cli` uses a rich `pub
  const : u8` taxonomy with inline `ExitCode` returns. A rich taxonomy cannot be
  layered over the collapsing `report`, because once a handler funnels a domain
  error through `kernel::Error` the variant information the ~45 codes need is
  erased. This child therefore adopts the **`work-cli` shape wholesale** —
  handlers return `ExitCode`, matching on domain outcome/error enums and
  returning `ExitCode::from(exit_codes::X)` — and borrows only the
  `collaboration-cli` HTTP-adapter *structure* (crate layout, the base-URL seam,
  synchronous client wiring), not its `report` funnel. Clients are
  **synchronous** (blocking `reqwest`) — no `tokio`, no async bridge.
- **The precedent artefact set is 0167.** `meta/inventories/0167-{removal-set,
  suite-audit,divergences}.md` are the direct templates. Their governing rule
  transfers verbatim: *"A divergence nothing can detect is indistinguishable
  from a defect, so every row names a real, passing test."*

### Decisions taken during planning

Recorded here and to be mirrored into 0171's `## Decisions`:

1. **Merge granularity — one work item, two provider tracks.** Jira and Linear
   are independently-mergeable phase groups within 0211; the provider-seam split
   into sibling children is declined. The refuted size assumption is absorbed by
   the tracks, not by decomposition.
2. **Preview resolved intent, not wire bytes.** The client crates expose no
   mutation-payload composer, so the write flows' `--print-payload`/`--describe`
   wire preview is **not** reproduced. Repointed bodies preview the resolved
   human-facing intent (title, target, resolved fields) and the binary executes
   atomically after confirm. The divergence is pinned by two concretely-specified
   automated tests, not a manual checkbox. **(a) Binary-level, observable seam**:
   run the write subcommand against a mock whose mutation route fails, and assert
   the resolved-fields preview line is already on stdout while the mutation route
   still has zero hits — so "preview precedes mutation" is checked against a real
   ordering seam. **(b) `test-skill-write-gate.sh` (new, skills-lane)**: for each
   write skill it locates the confirm step and the `accelerator <provider>`
   mutation line, asserts the confirm step is **present** (a missing confirm
   fails, so ordering cannot pass vacuously) and lexically precedes the mutation,
   and fails when both sit in the same fenced block; a committed reversed-body
   fixture proves the guard fails. The `--print-payload`/`--describe`
   `argument-hint` and body preview steps are dropped from **every write skill
   that advertises them** (create/update/comment, Linear and Jira) in the same
   repoint commit, and the write-gate + doc-vs-binary parity fail on any residual
   reference. The outgoing-request assertion still holds via the mock, and the
   three empty-stdout gates are redesigned to gate on the binary's exit **and to
   fail closed** (a non-zero or exit-16 create suppresses the frontmatter
   writeback and blocks retry, and the exit-16 branch surfaces an explicit "issue
   created remotely as <key>; reconcile manually" message so the orphaned-remote
   state is visible — asserted, Decision 5).
3. **Drop the cleartext-credential subcommand.** `jira-auth-cli.sh` /
   `linear-auth-cli.sh` are not reproduced; credential validation folds into
   `init verify`, which resolves and checks the token without printing it. A
   test asserts `init verify` never emits the token.
4. **`resolve-fields` is an `accelerator jira` subcommand.** `jira
   resolve-fields` reproduces the tab-separated four-field contract and exit
   codes 108/109, reading config through the `config`/`config-adapters` crates
   (no shell-out), even though it makes no Jira API call.
5. **`jira-emit-key.sh` → `jira create --emit key`** (a projection, carrying the
   distinct post-create non-retryable exit 16 semantic); **`jira-jql-cli.sh`
   dropped** (orphan, invoked only by its own test).
6. **Exit-code enforcement — the `work-cli` model, with a divergence
   allowlist.** Handlers return `ExitCode` inline (the `work-cli` shape), **not**
   `Result<Outcome, kernel::Error>` — `kernel::Error` collapses to `Refusal`/
   `Failed` and would erase the variant information the ~45 Jira codes need. Each
   binary carries `exit_codes.rs` + a fixture-anchored parity test; the document
   of record is a committed `EXIT_CODES.md` beside each crate
   (`cli/jira-cli/EXIT_CODES.md`, `cli/linear-cli/EXIT_CODES.md`). The parity
   test is **not** a blanket textual-equality assertion: the deliberate
   divergences (search `70-73` remap, `81`/`82`/`34` per-provider restatements)
   live in a count-pinned allowlist where each allowlisted row asserts the
   *remapped* Rust value while the fixture keeps the original bash value, and
   every allowlisted name must appear in the divergences ledger. Non-allowlisted
   names assert equality. The count pin makes a silent allowlist addition fail.
7. **Write skills stay bare-`Bash`.** They need not witness a token (the six
   read/init skills do). Their load-bearing uses survive: `wc -c`
   (`attach-jira-issue/SKILL.md:70`) and the `source config-common.sh` writeback
   (`create-jira-issue/SKILL.md:113`) remain shell steps.
8. **Golden provenance — mock-served, live-anchored.** Bash-flow goldens are
   captured against the deterministic mock servers; 0210's committed
   live-tenant contract evidence (dated 2026-08-18) is the reality anchor. Each
   fixture's provenance is recorded in 0171's `## Decisions`.
9. **Port-op integers are read structurally, not parsed from `detail`.** Each
   crate already computes the granular code with `classify::bash_code(Outcome)
   -> u16` — but once a port op collapses to `tracker::TrackerError` (only
   `Retryable`/`Terminal`, the integer baked into a formatted `detail` string,
   `cli/tracker/src/lib.rs`), that value is gone, and `TrackerError` carries no
   structural code field. So the binary obtains the code **before** the collapse.
   The additive change is a **structured discriminant on the client's port-op
   surface, covering every failure branch** — not only the `classify()`-routed
   ones: some branches build `TrackerError` inline without ever computing an
   `Outcome` (e.g. `create`'s post-create unusable-identifier at
   `cli/jira-client/src/client.rs:337-344`, `fetch_all`'s unsafe-identifier
   pre-flight at `:420-428`), and the post-create case is exactly the exit-16
   "created remotely but unwritable" condition Decisions 2/5 must distinguish.
   The client's port-op wrappers therefore return, on the error path, an enum
   carrying **either** a `classify::Outcome` (mapped by the existing
   `bash_code(Outcome) -> u16`) **or** an explicit binary-relevant reason
   (`UnwritableIdentifier`, `UnsafeQueryId`, …); `exit_codes.rs` maps both arms
   directly. There is no new `&TrackerError` accessor (that signature both
   collides with the existing `pub bash_code` and could only re-parse `detail`)
   and no substring-parse. This is an additive change inside the `_ADAPTER`-exempt
   client crate (no public-API snapshot), touching neither the shared `tracker`
   port type nor the mutation surface (contrast Decision 2).
10. **The base-URL seam is a validated credential destination; loopback is
    behind a test-only feature.** The `ACCELERATOR_{JIRA,LINEAR}_API_URL`
    override ships in the release binary but is admitted only when it is **https
    with an allowlisted host** — the same destination bar `from_config`
    credentials must clear. **Loopback admission is gated by a dedicated
    test-only cargo feature (`test-loopback`), enabled only by the integration
    tests' build config — never by `debug_assertions`.** `debug_assertions` is on
    for any ordinary `cargo build`, so gating loopback on it would leave every
    debug binary env-switchable into loopback — the very anti-pattern
    `cli/linear-client/src/upload.rs`'s module doc refuses. The caller passes
    `allow_loopback = cfg!(feature = "test-loopback")` into the existing
    runtime-parameter `UploadTransport::new(allow_loopback, …)` /
    `url_is_allowed(url, allow_loopback)`; loopback stays a runtime bool the
    caller supplies, never a static/env switch inside a shared helper, so no
    ordinary debug or release binary can be env-switched into loopback. A
    present-but-unparseable or non-admissible value is a hard usage error
    returned inline as `ExitCode::from(exit_codes::USAGE)` (the `work-cli`
    inline-`ExitCode` shape, not a `report` funnel and not a client-crate
    `SurfaceError`), never a silent fallthrough to `from_config`. The seam
    reconstructs `team_key`/`states` (Linear) and the upload transport
    identically to `from_config`, differing only in the endpoint (and, under
    `test-loopback`, loopback admission). Destination admissibility reuses **each
    client's own complete https-destination check** — **not** a single
    cross-provider function (the allowlists genuinely differ) and **not** the
    host-only `host_is_admissible` fragment — but the two providers are
    **asymmetric** on loopback:
    - **Linear**: `upload.rs::url_is_allowed(url, allow_loopback)` (promoted
      `pub`; enforces https, userinfo refusal, `*.linear.app` label match) takes
      a runtime `allow_loopback` bool the seam passes as
      `cfg!(feature = "test-loopback")`.
    - **Jira**: `auth.rs::base_url` stays **strict and unchanged** — it has no
      loopback parameter and rejects http, explicit ports, and non-
      `*.atlassian.net` hosts. The release seam routes the override through it
      unchanged. Under `test-loopback`, the Jira seam reaches a mock by a
      dedicated gated branch that **constructs `Credentials` pointed at the
      override directly** (as `cli/jira-client/tests/support/client.rs` already
      does) and calls `JiraClient::new`, **bypassing** `base_url` — never by
      relaxing it. So `base_url` is never weakened, and Jira has no
      loopback-through-`base_url` path.

## What We're NOT Doing

- **Not** splitting 0211 into sibling children (Decision 1).
- **Not** reproducing the wire-payload preview or the cleartext-auth subcommand
  (Decisions 2, 3).
- **Not** touching the sixteen generated docs-site reference pages
  (`docs-site/src/content/docs/reference/skills/` is gitignored, `.gitignore:26`
  — rebuilt from `SKILL.md`).
- **Not** changing `EXPECTED_INJECTION_SKILLS = 42`
  (`tasks/lint/skill_permissions.py:48`) or `.claude-plugin/plugin.json:16-17`
  — the skills survive; only their bodies repoint and their `scripts/` dirs go.
- **Not** adding a mutation-payload compose seam to the client crates (that
  would re-open 0210).
- **Not** retiring the fourteen repo-root `scripts/*.sh` `SHELL_LIBRARIES`
  entries or the exec-bit guard itself — those are 0174's, unblocked by this
  child.
- **Not** narrowing the ten write skills' bare `Bash` grant (Decision 7).

## Implementation Approach

Each provider track runs binary-first, then cutover-plus-deletion:

1. **Binary phase** (Linear = Phase 1, Jira = Phase 3): build the `*-cli` crate
   over the client crate, TDD per subcommand against mock-backed goldens,
   capture the pre-deletion exit-code and stdout oracle **while the bash still
   exists**, and pin the exit-code mapping against the committed fixture. The
   crate is a workspace member and `public_api` exempt but **not registered** —
   it ships no skill binding yet, which is coherent.
2. **Cutover + retirement phase** (Linear = Phase 2, Jira = Phase 4): register
   the token and repoint that provider's eight `SKILL.md` bodies **in one
   commit**, drop `jq`/`curl` + the script glob from its three read/init
   frontmatters, then delete that provider's bash cluster and retire its
   provider-specific guards. Linear decrements the shared floor 32→20; Jira
   removes it outright and lands the whole-repository assertions and the final
   artefacts.

The binary phases capture the oracle; the cutover phases consume it and delete
the generators. Sequencing guarantees capture precedes deletion.

---

## Phase 1: Linear binary (`cli/linear-cli`)

### Overview

Build `accelerator-linear` over `linear-client` with every subcommand, a
mock-backed golden per flow, the exit-code taxonomy pinned to a captured
fixture, and the preview-resolved-intent gate. Not registered; the linear bash
cluster stays in place as the capture source.

### Subcommand surface and the reconciliation mapping

Ten executables + two libraries map as:

| Bash executable | Disposition |
|---|---|
| `linear-create-flow.sh` | `linear create` |
| `linear-update-flow.sh` | `linear update` |
| `linear-show-flow.sh` | `linear show` |
| `linear-search-flow.sh` | `linear search` (`filter::compose` + `fetch_all`) |
| `linear-comment-flow.sh` | `linear comment add` |
| `linear-transition-flow.sh` | `linear transition` (`resolve_state` + `transition`) |
| `linear-attach-flow.sh` | `linear attach --url \| --file` |
| `linear-init-flow.sh` | `linear init verify \| list-teams \| discover` |
| `linear-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `linear-graphql.sh` | dropped — internal transport, subsumed by `linear-client` |
| `linear-common.sh`, `linear-auth.sh` (libs) | subsumed by the crate |

### Changes Required

#### 1. Crate scaffold

**Files**: `cli/linear-cli/Cargo.toml`, `cli/linear-cli/src/main.rs`,
`cli/linear-cli/src/cli.rs`, `cli/linear-cli/src/outcome.rs`,
`cli/linear-cli/src/exit_codes.rs`, plus one module per flow under
`cli/linear-cli/src/`; and `cli/linear-client/` to return a structured
discriminant on the port-op error path — a `classify::Outcome` (reusing the
existing `bash_code(Outcome)`) or an explicit reason for the inline-`TrackerError`
branches — so every failure branch yields a code without a `detail` parse
(Decision 9 — additive at the point the code is already computed, `_ADAPTER`
public-API exempt, no `tracker`-port change).

`Cargo.toml` mirrors `cli/collaboration-cli/Cargo.toml`: package + `[[bin]]`
`name = "accelerator-linear"`, mandatory `description = "The linear create|
update|show|search|comment|transition|attach|init sub-binary."`, `[lints]
workspace = true`; deps `linear-client`, `tracker`, `tracker-support`, `config`,
`config-adapters`, `kernel`, `clap = { workspace = true }`; dev-deps `tempfile`,
`http-test-support` (both `{ workspace = true }` / path); and a `[features]`
`test-loopback` (default-off) that the integration tests enable to admit the
loopback destination (Decision 10) — never in the default feature set, enabled
via the test target's dev path (not a crate-dir `.cargo/config.toml` that could
bleed into a release build invoked from the directory), so no ordinary build
carries it.

`main.rs` follows the **`work-cli` inline-`ExitCode` shape** (Decision 6, Key
Discoveries): a two-level clap `Cli` (`#[command(name = "accelerator-linear",
disable_version_flag = true)]`), and handlers that match on the domain
outcome/error enums and return `ExitCode::from(exit_codes::X)` directly. It
does **not** funnel domain errors through `kernel::Error` + a collapsing
`report` — that would erase the variant information the taxonomy needs. Only the
crate layout, the base-URL seam and the synchronous client wiring are borrowed
from `collaboration-cli`.

#### 2. The base-URL seam (validated credential destination)

**File**: `cli/linear-cli/src/main.rs`

The override ships in release but is admitted only as a validated credential
destination before any token attaches (Decision 10). The revalidation and the
unparseable/non-admissible hard error are **binary-owned** — returned inline as
`ExitCode::from(exit_codes::USAGE)`, not through a client-crate error type — and
loopback admission is gated behind the test-only `test-loopback` feature:

```rust
fn api_base_uri() -> Result<Option<Url>, UsageError> {
    let Some(raw) = std::env::var_os("ACCELERATOR_LINEAR_API_URL") else {
        return Ok(None);
    };
    let uri = Url::parse(&raw.to_string_lossy()).map_err(|_| UsageError::BadApiUrl)?;
    // release: https + allowlisted host only; `test-loopback` also admits loopback.
    if !url_is_allowed(&uri, cfg!(feature = "test-loopback")) {
        return Err(UsageError::BadApiUrl);
    }
    Ok(Some(uri))
}
```

`UsageError::BadApiUrl` is the binary's own type, returned inline as the usage
`ExitCode` at the top-level handler (the `work-cli` inline-`ExitCode` shape —
there is no `report` funnel); `url_is_allowed` is Linear's own complete
https-destination check, promoted `pub` (Decision 10), and `allow_loopback` is a
**caller-supplied runtime bool** fixed to `cfg!(feature = "test-loopback")`, so
no ordinary debug or release binary admits loopback.

When present, the seam branch must reconstruct the **whole** client the way
`from_config` does, differing only in the GraphQL endpoint (and, under
`test-loopback`, loopback admission) — it is not enough to override `Transport`
alone:

- `Transport::new(endpoint, credentials, …)` for the GraphQL endpoint.
- `UploadTransport::new(allow_loopback, retry_delay)` in place of
  `UploadTransport::production()`, where `allow_loopback` is
  `cfg!(feature = "test-loopback")` — so the attach flow's server-nominated
  upload host (a mock loopback URL) is admitted under the test feature but never
  in an ordinary debug or release build.
- `team_key` via `catalogue_team_key(integrations_root)` and `states` via
  `CatalogueStates::load(integrations_root)`, identical to `from_config`, so the
  `transition` flow's `resolve_state` sees a populated catalogue.
- `LinearClient::new(transport, upload, team_key, states)`.

Otherwise `LinearClient::from_config(context, integrations_root)`. Mirrors
`collaboration-cli/src/main.rs:41-48,123-128` for the endpoint override,
extended with the upload/catalogue reconstruction and the test-gated loopback
admission.

#### 3. The exit-code taxonomy and the captured oracle

**Files**: `cli/linear-cli/src/exit_codes.rs`,
`cli/linear-cli/tests/exit_codes_parity.rs`,
`cli/linear-cli/tests/fixtures/bash-exit-codes.txt`.

**Capture — scripted, exhaustive, differential, while the bash exists.** The
capture is a committed script, not a hand transcription, so its provenance is
reviewable and its completeness is checkable:

- **Declared half**: grep every `linear-*-flow.sh` for `readonly E_*=NN` (the
  `test-linear-paths.sh:95` idiom).
- **Behavioural half**: a differential harness that *executes* each flow against
  every error-scenario fixture and records the observed exit code (mirroring
  0210's D10 differential precedent — do not hand-record). The capture
  enumerates every error branch; the anti-vacuity count is derived from the
  capture output, not hand-picked.
- **Conflict rule**: a name may appear in more than one flow and a declared
  value may disagree with the behavioural one (the plan cites
  `jira-request.sh:207` exiting `1` where the doc says `2`). Precedence is
  **behavioural wins over declared**; names are namespaced per flow where a
  genuine collision remains, and the parse asserts each `(flow, name)` key is
  unique in `bash-exit-codes.txt`. Declared-vs-behavioural disagreements are
  reconciled as named divergence rows, never collapsed by the grep.

`exit_codes.rs` declares `pub const : u8` classes and maps `SurfaceError` /
the port-op code (from the surfaced `classify::Outcome` via the existing
`bash_code(Outcome)`, Decision 9) / `ClientError` variants onto them. Two
enforcement layers guard the mapping:

- `exit_codes_parity.rs` compares `exit_codes.rs` against `bash-exit-codes.txt`.
  It is **not** blanket textual equality: non-allowlisted names assert equality;
  a **count-pinned divergence allowlist** (search `70-73` remap) holds rows that
  assert the *remapped* Rust value while the fixture keeps the bash value, and
  every allowlisted name must appear in the divergences ledger.
- A **behavioural exit-code test per error class** drives the binary into each
  condition and asserts the *observed* exit code — so a variant mis-routed to the
  wrong constant fails a test, not just a const-declaration mismatch. Network
  classes are triggered via mock responses; the **non-network classes** (usage
  errors, unparseable/non-admissible `ACCELERATOR_*_API_URL`, missing-config/
  `from_config` failures, the post-create exit-16 semantic) are triggered via
  their real sources (arg fixtures, env overrides, config fixtures, seeded
  post-create conditions), so the routing guarantee covers all ~45 classes, not
  only mock-reachable ones. The class count is tied to the capture-derived
  enumeration.

Linear-specific contract: the skills key on **symbolic `E_*` names in stderr**,
so parity for Linear is anchored over the `E_*` names the bodies actually branch
on, not just the subset `SurfaceError` Display emits. `SurfaceError` emits the
transport/attach/transition subset (`E_GQL_ERRORS`, `E_REQ_*`,
`E_TRANSITION_STATE_*`, `E_ATTACH_*`), but the repointed bodies also branch on
~nine **binary-owned argument-validation names** the surface never emits —
`E_CREATE_ALREADY_SYNCED`, `E_CREATE_BAD_FRONTMATTER`, `E_COMMENT_NO_BODY`,
`E_UPDATE_BAD_STATE`, `E_UPDATE_NO_OPS`, `E_ATTACH_BOTH_TARGETS`,
`E_SHOW_NOT_FOUND`, `E_NO_TOKEN`, plus the integer `107`
(`E_CREATE_WRITEBACK_FAILED`). The `*-cli` crate reproduces these, and the stderr
golden set pins **every `E_*` name any repointed body references** (enumerated
from the `readonly E_*=` capture, asserting each is emitted verbatim) — not only
the `SurfaceError` subset — rather than a parallel full integer taxonomy no
consumer reads. Never emit `70`–`73` for a provider condition; the search flow's
`E_SEARCH_*` (bash `70-73`) is remapped out of the reserved band (the allowlisted
divergence, with a test over the whole subcommand set).

#### 4. Subcommands over `linear-client`, TDD per flow

**Files**: one module + one test file per flow under `cli/linear-cli/`.

Each subcommand: parse args → build the credential context → call the client
method → render JSON (or the `E_*` error to stderr + exit code). Per flow, a
mock-backed test asserts the outgoing request (method, `/graphql` document,
variables via `last_body`) and the parsed response against a fixture, and a
byte-exact stdout golden (`Vec<u8>`, never `from_utf8_lossy`). `Route::Sequence`
distinguishes the create→show sequence on the shared `/graphql` key.

Additional contracts the goldens must cover, each keyed on by a repointed body:

- **Search stderr audit line**: `search` echoes the composed `IssueFilter` to
  stderr (`INFO: composed IssueFilter: …`); a stderr golden pins it, and
  `--quiet` (reproduced as a flag) suppresses it.
- **Search JSON envelope**: the body reads `.data.issues.nodes[]`
  (`.identifier`, `.title`, `.state.name`, `.assignee.name`) and the
  merged-pages `.data.issues.truncated` flag. A named byte-exact golden pins the
  exact envelope the binary emits; any client-vs-bash shape gap (e.g. no
  `truncated`) is flagged as an explicit divergence with a body update, not
  assumed away by the mock golden.
- **Multi-POST flows**: because Linear posts everything to `/graphql`,
  `http-test-support` today records only `last: Option<Received>` per
  `(method, path)` key, so the first mutation's body is overwritten. This
  requires an **additive change to `cli/http-test-support/src/lib.rs`** to record
  a `Vec<Received>` per key (a per-hit body log alongside the existing `hits()`
  count), listed as a Phase 1 file. Any flow issuing two POSTs (`create`+
  writeback, `transition`'s `resolve_state`+`transition`) then asserts the hit
  count is exactly the expected number and each hit's body, so the first
  mutation's variables are actually asserted (Decision 2's outgoing-request
  guarantee). The change is backward-compatible — `last_body` remains for
  single-POST callers.
- **Production `from_config` path**: at least one test drives the
  `from_config(context, integrations_root)` branch with a fixture config
  directory (asserting the resolved endpoint/credentials or a defined
  unconfigured error), so the path real users hit is not covered by manual
  verification alone.

`init verify` validates credentials without printing them (Decision 3), and its
no-token guarantee is asserted exhaustively: a recognisable sentinel token is
seeded and every exit path (success and each error/transport-failure variant) is
driven, asserting the sentinel appears on neither stdout nor stderr; the
`Secret`-redaction invariant is recorded as the reason the guarantee holds. The
write flows expose no wire-payload preview (Decision 2); the confirm gate and
its lexical-ordering guard are the repointed body's concern in Phase 2.

#### 5. Whole-surface help golden

**Files**: `cli/linear-cli/tests/cli_surface.rs`,
`cli/linear-cli/tests/fixtures/cli_surface.golden`.

The `=== accelerator-linear <sub> --help ===` section-header pattern from
`cli/work-cli/tests/cli_surface.rs`.

#### 6. Workspace registration (crate only, no token)

**Files**: `cli/Cargo.toml` (`members` += `"linear-cli"`), `cli/Cargo.lock`
(resync via `cargo metadata`, never `generate-lockfile`), `tasks/public_api.py`
(`_EXEMPT_MEMBERS` += `linear-cli` with reason `_COMPOSITION_ROOT`).

`tests/unit/tasks/test_rust.py:160-170` forces the `public_api.py` edit in the
same change as the member addition.

### Success Criteria

#### Automated Verification:

- [ ] Workspace builds locked: `mise run cli:check`
- [ ] Linear CLI tests pass (per-flow request/response/stdout goldens, exit-code
      parity, help surface): `cargo nextest run -p accelerator-linear` (from
      `cli/`)
- [ ] Behavioural exit-code test drives the binary into each error class and
      asserts the *observed* exit code (not just const declarations); the
      anti-vacuity count is derived from the exhaustive capture
- [ ] Parity divergence allowlist is count-pinned and every allowlisted name
      appears in the divergences ledger
- [ ] `bash-exit-codes.txt` parse asserts each `(flow, name)` key is unique
- [ ] No binary emits `70`–`73` for a provider condition (test over the whole
      subcommand set)
- [ ] Search stderr audit line (`INFO: composed IssueFilter`) golden holds and
      `--quiet` suppresses it
- [ ] Search JSON-envelope golden (`.data.issues.nodes[]` + `.truncated`)
      matches the binary's emission
- [ ] Multi-POST flows assert the `/graphql` hit count and per-hit bodies
- [ ] The `from_config` branch has an automated test
- [ ] `init verify` sentinel-token test proves no token on stdout or stderr
      across every exit path
- [ ] A set-but-unparseable or non-admissible `ACCELERATOR_LINEAR_API_URL`
      hard-errors via the binary's own usage exit path before credentials
      attach; a loopback/plain-http override is rejected in any build without the
      `test-loopback` feature (loopback admission is `test-loopback`-gated, never
      `debug_assertions`)
- [ ] Rust public-API classification green: `mise run cli:check` covers
      `test_rust.py`
- [ ] Full read-only gate: `mise run check`

#### Manual Verification:

- [ ] `accelerator-linear` run against a live Linear team returns the same issue
      shapes as `linear-*-flow.sh` for create/show/search/comment/transition/
      attach/init (spot-check against 0210's contract evidence)

---

## Phase 2: Linear cutover + retirement

### Overview

Register the `linear` token and repoint the eight linear `SKILL.md` bodies in
one commit, drop `jq`/`curl` + the script glob from the three read/init
frontmatters, then delete the linear bash cluster and retire its
provider-specific guards. Decrement the shared floor 32→20.

### Changes Required

#### 1. Token registration (same commit as repointing)

**Files**: `tasks/shared/paths.py` (`DISPATCHED_SUBBINARIES` += `"linear"`),
`tests/integration/tasks/test_github.py` (`_SUBBINARY_DESCRIPTIONS` entry +
exact-tuple pin `:533-541`), `tasks/manifest.py` (`_SUBBINARY_MANIFESTS` +=
`"linear": CLI_DIR / "linear-cli/Cargo.toml"`), `tasks/build.py`
(`_CLI_RELEASE_BINARIES` += `"accelerator-linear"`), `.gitignore` (`bin/
linear-*`).

#### 2. Repoint the eight linear `SKILL.md` bodies

**Files**: `skills/integrations/linear/{init-linear,show-linear-issue,
search-linear-issues,create-linear-issue,update-linear-issue,
comment-linear-issue,transition-linear-issue,attach-linear-issue}/SKILL.md`.

Rewrite each fenced execution step from a `linear-*-flow.sh` invocation to
`accelerator linear …`. This same commit must also rewrite every **hardcoded
in-body exit-code reference** to the binary's new taxonomy — the repointed body
is itself a consumer that branches on these integers (e.g. a search step citing
`Exit 72`/`Exit 71`) — and a doc-vs-binary parity assertion covers the remapped
codes so a stale in-body table fails a test.

Read/init skills (`init-linear`, `show-linear-issue`, `search-linear-issues`)
additionally drop `Bash(jq)`, `Bash(curl)` and the
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)` glob from
frontmatter, keeping the existing `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator
config *)` grant and adding the token grant `Bash(${CLAUDE_PLUGIN_ROOT}/bin/
accelerator linear *)`. At least one read/init skill becomes the token witness —
and its witnessing invocation must be **metacharacter-free**: because
`dispatch_coherence.py:_bindings` counts a binding only when
`not has_metacharacter(command)` (`:103`), a piped `accelerator linear search …
| …` records the token as invoked-but-unbound and fails coherence. The witness
skill invokes `accelerator linear <sub>` in a fenced step with no pipe/redirect/
`&&` (the binary renders final JSON itself, so no downstream `jq` pipe is
needed). This specific skill is verified during Phase 2, not just "one becomes
the witness".

Write skills keep bare `Bash` (Decision 7). The confirm gate previews resolved
intent (Decision 2) and its lexical precedence over the mutation invocation is
enforced by the new `test-skill-write-gate.sh` skills-lane guard, not a manual
checkbox. `create-linear-issue`'s empty-stdout dependence is redesigned to gate
on the binary's exit **and to fail closed** — a non-zero create suppresses the
writeback and blocks retry (Decision 5), asserted by test. Any `--print-payload`/
`--describe` advertised in the `argument-hint` or a body preview step of **any**
write skill (`create-`, `update-`, `comment-linear-issue`) is dropped in this
commit; the write-gate + doc-vs-binary parity fail on any residual reference.

#### 3. Delete the linear cluster and retire its guards

**Files deleted**: `skills/integrations/linear/scripts/*.sh` (10 executables + 2
libraries), the 12 `test-linear-*.sh` suites,
`skills/integrations/linear/scripts/test-helpers/mock-linear-server.py`,
`skills/integrations/linear/scripts/test-fixtures/`.

**Guards edited**:

- `tasks/lint/scripts.py` — drop the two linear `SHELL_LIBRARIES` members
  (`linear-common.sh`, `linear-auth.sh`, `:40-41`).
- `tests/unit/tasks/test_exec_bits.py` — drop the same two from
  `_RECONCILED_LIBRARIES` (`:266-267`); `test_exact_membership` (`:279-282`)
  stays green because both literals change together.
- `tasks/test/integration.py` — decrement `_EXPECTED_INTEGRATIONS_SUITES`
  32 → 20 (`:57`). The `integrations` task survives (jira suites remain).
- `tests/unit/tasks/test_python_coverage.py` — remove `MOCK_LINEAR` (`:34-36`)
  and its usages (`:41`, `:104`, `:113`); adjust the in-scope arithmetic.
- `pyproject.toml` — strip the `mock-linear-server.py` `extend-exclude` line
  (`:80`); `RUFF_JUSTIFIED_EXCLUDES` in `test_python_coverage.py` drops
  `MOCK_LINEAR` in the same change (set-equality).

#### 4. Record linear artefacts

**Files**: `meta/inventories/0211-removal-set.md`,
`meta/inventories/0211-suite-audit.md`,
`meta/inventories/0211-reconciliation.md`,
`meta/inventories/0211-divergences.md` (linear rows),
`meta/work/0171-jira-and-linear-integrations.md` (`## Decisions`).

Record the **Linear generator provenance here**, at this track's merge boundary
(not deferred to Phase 4): the last-existing revision of `mock-linear-server.py`
and the linear bash cluster, the scripted capture command and its full output
for `bash-exit-codes.txt`, and each linear golden's provenance. Because the
tracks are independently mergeable, a Linear-only merge must still carry its own
revival anchor.

### Success Criteria

#### Automated Verification:

- [ ] Dispatch coherence green both directions, and the witness invocation is
      metacharacter-free (bound, not merely invoked): `mise run
      build-system:check` (exercises `lint:dispatch-coherence:check`)
- [ ] `test-skill-write-gate.sh` asserts each linear write skill's confirm step
      lexically precedes its `accelerator linear …` mutation invocation
- [ ] Repointed-body exit-code references match the binary taxonomy
      (doc-vs-binary parity over the remapped codes); no body cites a pre-remap
      integer
- [ ] `ls skills/integrations/linear/scripts/*.sh` matches nothing;
      `mock-linear-server.py` does not exist
- [ ] Linear integration floor holds at 20 and the `integrations` task runs the
      surviving jira suites: `mise run test:integration:integrations`
- [ ] Exec-bit + stale-library guards green: `mise run lint:scripts:check`
      (no stale `SHELL_LIBRARIES` entry)
- [ ] Python coverage + ruff-exclude equality green: `mise run test:unit`
- [ ] Full run green end to end: `mise run`

#### Manual Verification:

- [ ] Every linear `SKILL.md` body invokes `accelerator linear …`; no linear
      skill declares `jq`, `curl` or a `scripts/` grant
- [ ] The confirm gate still previews the resolved intent before a write
- [ ] Reconciliation table reconciles to 10 executables + 2 libraries

---

## Phase 3: Jira binary (`cli/jira-cli`)

### Overview

Mirror Phase 1 for Jira, plus ADF handling, the `resolve-fields` subcommand
(Decision 4), the `create --emit key` projection (Decision 5), the tab-separated
resolver golden, the ~45-integer exit-code parity, and the search `70-73` remap.
Not registered; the jira bash cluster stays as the capture source.

### Subcommand surface and the reconciliation mapping

Seventeen executables + five libraries + three data assets map as:

| Bash executable | Disposition |
|---|---|
| `jira-create-flow.sh` | `jira create` |
| `jira-update-flow.sh` | `jira update` |
| `jira-show-flow.sh` | `jira show` (`--render-adf` flag) |
| `jira-search-flow.sh` | `jira search` (`jql::compose`; codes remapped off `70-73`) |
| `jira-comment-flow.sh` | `jira comment add \| list \| edit \| delete` |
| `jira-transition-flow.sh` | `jira transition` |
| `jira-attach-flow.sh` | `jira attach` (multipart) |
| `jira-init-flow.sh` | `jira init verify \| discover \| prompt-default \| refresh-fields \| list-projects \| list-fields` |
| `jira-fields.sh` | `jira fields refresh \| resolve \| list` |
| `jira-resolve-fields.sh` | `jira resolve-fields` (Decision 4) |
| `jira-emit-key.sh` | `jira create --emit key` projection (Decision 5) |
| `jira-render-adf-fields.sh` | `--render-adf` flag on `show`/`search` |
| `jira-adf-to-md.sh` | internal — `document_to_markdown`, no subcommand |
| `jira-md-to-adf.sh` | internal — `markdown_to_document`, no subcommand |
| `jira-request.sh` | internal transport — `transport.rs`, no subcommand |
| `jira-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `jira-jql-cli.sh` | dropped — orphan (Decision 5) |
| `jira-common/-auth/-jql/-body-input/-custom-fields.sh` (5 libs) | subsumed by the crate |
| `jira-adf-render.jq`, `jira-md-tokenise.awk`, `jira-md-assemble.jq` | no subcommand — bash-pipeline artefact, gone in Rust |

### Changes Required

#### 1. Crate scaffold, seam, taxonomy, subcommands

**Files**: `cli/jira-cli/` mirroring Phase 1 (same `work-cli` inline-`ExitCode`
shape, same scripted/differential capture and parity-allowlist enforcement),
plus `cli/jira-client/` returning a structured discriminant on the port-op error
path (a `classify::Outcome` reusing the existing `bash_code(Outcome)`, or an
explicit reason for inline-`TrackerError` branches such as post-create
unwritable-identifier `client.rs:337-344` and `fetch_all` unsafe-id `:420-428`),
Decision 9,
with `description = "The jira create|update|show|search|comment|transition|
attach|init|fields|resolve-fields sub-binary."`, dep `jira-client`, and the
`ACCELERATOR_JIRA_API_URL` seam overriding `Credentials.base` (Jira has no
explicit-endpoint constructor — build a `Credentials` pointed at the mock and
call `JiraClient::new`, per `cli/jira-client/tests/support/client.rs`). The
override is admitted only through Jira's already-`pub` `auth.rs::base_url`
(Decision 10) — the **fully-validated** destination check (https, no userinfo,
no query/fragment, default port, `*.atlassian.net`/allowlist), not the host-only
`host_is_admissible` fragment — before the email+token attach; a set-but-
unparseable or non-admissible value is a hard usage error returned inline as
`ExitCode::from(exit_codes::USAGE)`. Reusing `base_url` (rather than the host
matcher alone) is what prevents a cleartext `http://foo.atlassian.net` from
slipping through. `base_url` stays **strict and unchanged** — it admits no
loopback. For the binary's mock-backed tests, a dedicated `test-loopback`-gated
branch constructs `Credentials` pointed at the override directly and calls
`JiraClient::new` (as `cli/jira-client/tests/support/client.rs` does), bypassing
`base_url`; it is compiled out of every ordinary build. This closes the seam's
bypass of the `*.atlassian.net` credential-destination control without weakening
`base_url` and keeps any loopback path out of every ordinary debug and release
binary.

Capture the Jira oracle with the same scripted differential harness as Phase 1
(declared `readonly E_*=NN` grep across all jira flows + behavioural exit codes
from executing each flow against its error fixtures, behavioural-wins precedence,
per-`(flow, name)` uniqueness) into `cli/jira-cli/tests/fixtures/
bash-exit-codes.txt`. `exit_codes.rs` maps `SurfaceError`/`ClientError` variants
and the port-op code (from the surfaced `classify::Outcome` via the existing
`bash_code(Outcome)`, Decision 9 — never parsed from `detail`) → the ~45
integers;
`exit_codes_parity.rs` pins them with the count-pinned divergence allowlist, and
a behavioural exit-code test asserts the observed code per class. **Capture
behaviour, not the doc**: `jira-request.sh:207` exits `1` where
`EXIT_CODES.md:12` claims `2` — recorded as a named declared-vs-behavioural
divergence, not silently collapsed.

#### 2. Strict stdout goldens

**Files**: `cli/jira-cli/tests/fixtures/*.golden`.

Byte-exact goldens for the strict contracts: `jira resolve-fields` emits the
tab-separated four-field line (`<type>\t<type_source>\t<project>\t
<project_source>\n`, trailing newline load-bearing); `jira create --emit key`
emits the bare validated key (`^[A-Z][A-Z0-9]+-[0-9]+$`) with the post-create
exit-16 semantic; `jira show` renders the ADF description via
`document_to_markdown`.

#### 3. The search `70-73` remap and the cross-provider collisions

**Files**: `cli/jira-cli/src/exit_codes.rs`, `cli/jira-cli/tests/exit_codes_
parity.rs`, `cli/jira-cli/EXIT_CODES.md`.

`jira search`'s `E_SEARCH_BAD_PAGE_TOKEN/BAD_LIMIT/NO_SITE_CACHE/BAD_FLAG` (bash
`70-73`) are remapped off the reserved band (recorded divergence + test). The
`81`/`82`/`34` cross-provider collisions each resolve to a stated per-provider
Jira behaviour with a test. The credential-resolution divergence (Jira flattens
to `22`) is already encoded in `jira-client`'s `error.rs`/`classify.rs`.

#### 4. Workspace registration (crate only, no token)

**Files**: `cli/Cargo.toml` (`members` += `"jira-cli"`), `cli/Cargo.lock`,
`tasks/public_api.py` (`_EXEMPT_MEMBERS` += `jira-cli`).

### Success Criteria

#### Automated Verification:

- [ ] Workspace builds locked: `mise run cli:check`
- [ ] Jira CLI tests pass (request/response/stdout goldens incl. tab-separated
      resolver + bare key + ADF render, exit-code parity, help surface):
      `cargo nextest run -p accelerator-jira` (from `cli/`)
- [ ] Behavioural exit-code test drives the binary into each error class and
      asserts the observed code; parity divergence allowlist is count-pinned and
      ledger-backed; `(flow, name)` keys are unique in `bash-exit-codes.txt`
- [ ] No binary emits `70`–`73` for a provider condition; search codes proven
      off the reserved band (test)
- [ ] `81`/`82`/`34` each resolve to a stated per-provider behaviour (tests)
- [ ] `jira search` composed-JQL stderr audit line (`INFO: composed JQL`) golden
      holds, `--quiet` suppresses it, and the `issues[]` + `nextPageToken`
      envelope golden matches the binary's emission
- [ ] Port-op codes are read from the structured discriminant the client's
      port-op path returns (a `classify::Outcome` via the existing
      `bash_code(Outcome)`, or an explicit reason for inline-`TrackerError`
      branches like post-create unwritable / unsafe-id), never parsed from
      `detail` — the reuse is a typed call; the absence of a `detail` parse is
      enforced by a lint/grep guard over `exit_codes.rs`, not the compiler; no
      new `&TrackerError` accessor
- [ ] A set-but-unparseable or non-admissible `ACCELERATOR_JIRA_API_URL`
      hard-errors via the binary's own usage exit path before the token
      attaches; the release seam rejects a loopback/plain-http override through
      the unchanged strict `base_url` in every build, and the mock-backed tests
      reach a loopback endpoint only via the dedicated `test-loopback`-gated
      direct-`Credentials` branch (never by relaxing `base_url`, never
      `debug_assertions`); the `from_config` branch has an automated test
- [ ] Full read-only gate: `mise run check`

#### Manual Verification:

- [ ] `accelerator-jira` against a live Jira project matches `jira-*-flow.sh`
      for every flow (spot-check against 0210 contract evidence)
- [ ] `jira resolve-fields` output is byte-identical to the bash resolver line
- [ ] `init verify` prints no credential

---

## Phase 4: Jira cutover + retirement (child merge boundary)

### Overview

Register the `jira` token and repoint the eight jira `SKILL.md` bodies in one
commit, drop `jq`/`curl` from the three read/init frontmatters, delete the jira
cluster, and — as the second and final track — remove the integrations floor
outright, retire `_DUAL_USE_SCRIPTS`, land the whole-repository assertions and
finalise the artefacts.

### Changes Required

#### 1. Token registration (same commit as repointing)

**Files**: as Phase 2 item 1, for `jira`: `DISPATCHED_SUBBINARIES`,
`_SUBBINARY_DESCRIPTIONS` + tuple pin, `_SUBBINARY_MANIFESTS` (`"jira": CLI_DIR /
"jira-cli/Cargo.toml"`), `_CLI_RELEASE_BINARIES` (`"accelerator-jira"`),
`.gitignore` (`bin/jira-*`).

#### 2. Repoint the eight jira `SKILL.md` bodies

**Files**: `skills/integrations/jira/{init-jira,show-jira-issue,
search-jira-issues,create-jira-issue,update-jira-issue,comment-jira-issue,
transition-jira-issue,attach-jira-issue}/SKILL.md`.

As Phase 2 item 2, for `jira` — including the same-commit rewrite of every
hardcoded in-body exit-code reference to the new taxonomy. Seven Jira bodies cite
exit integers, not two: `search-jira-issues` Step 3 (`Exit 72`/`Exit 71`),
`create-jira-issue` Step 10 + WF-1 (100-107, 108/109, 11/12/22/19/20/21/34),
`show-jira-issue` (80/81/82 — 81/82 are Decision 6 per-provider restatements),
`transition-jira-issue` (122/123/124), `attach-jira-issue` (132/133), and the
`update-`/`comment-jira-issue` Step-9 tables. All are rewritten and covered by
the doc-vs-binary parity assertion, which **enumerates every repointed body**
(all sixteen jira+linear), greps their fenced `Exit NN`/code→message references,
maps each to the binary taxonomy, and asserts a non-zero matched-reference count
(anti-vacuity) so an under-matching extractor cannot pass silently; a committed
stale-integer fixture proves it fails. It lives as a build-system guard beside
`dispatch_coherence`. The three read/init skills drop `jq`/`curl` +
the script glob and gain the `jira` token grant; one becomes the witness with a
**metacharacter-free** witnessing invocation (no `jq` pipe). The write skills
keep bare `Bash` — `attach-jira-issue:70`'s `wc -c` and
`create-jira-issue:113`'s `source config-common.sh` writeback survive.
`create-jira-issue` calls `jira resolve-fields` then `jira create`; its
empty-stdout gates (`:183`) and the resolver-line parse (`:65-66`) are repointed
onto the new subcommands and **fail closed** — a non-zero or exit-16 create
suppresses the frontmatter writeback and blocks retry (Decision 5), asserted by
test. The `test-skill-write-gate.sh` guard covers the jira write skills'
confirm-before-mutation ordering, and any `--print-payload`/`--describe` in the
`argument-hint` or a body preview step of any jira write skill (`create-`,
`update-`, `comment-jira-issue`) is dropped; the write-gate + doc-vs-binary
parity fail on any residual reference.

#### 3. Delete the jira cluster and retire its guards

**Files deleted**: `skills/integrations/jira/scripts/*.sh` (17 executables + 5
libraries), the three data assets (`jira-adf-render.jq`, `jira-md-tokenise.awk`,
`jira-md-assemble.jq`), the 21 `test-jira-*.sh` suites,
`skills/integrations/jira/scripts/test-helpers/mock-jira-server.py`,
`skills/integrations/jira/scripts/test-fixtures/` (incl. the already-dead
`api-responses/`).

**Guards edited**:

- `tasks/lint/scripts.py` — drop the five jira `SHELL_LIBRARIES` members
  (`:35-39`).
- `tests/unit/tasks/test_exec_bits.py` — drop the five from
  `_RECONCILED_LIBRARIES` (`:261-265`); remove `_DUAL_USE_SCRIPTS` (`:275` + its
  comment) and `test_dual_use_scripts_are_entrypoints` (`:289-298`) — the only
  pinned dual-use exemplar (`jira-fields.sh`) is gone with no substitute. The
  divergence row must confirm what still detects a future dual-use script: the
  exec-bit invariant guard (`lint:scripts:exec-bits:check`) still classifies any
  new `.sh` as entrypoint-or-library and fails an unclassified one, so a future
  dual-use script is rejected (not silently misclassified) even though the
  positive-exemplar test is gone. Record this as the detection mechanism, not an
  unqualified deletion (recorded divergence: exemplar-coverage loss, detection
  retained).
- `tasks/README.md:90-94` — retire the dual-use prose that documents the
  classification through `jira-fields.sh`.
- `tasks/test/integration.py` — remove `_EXPECTED_INTEGRATIONS_SUITES` (`:53-57`)
  and the `integrations` task (`:404-410`) outright (jira was the last surviving
  suites).
- `tasks/test/helpers.py` — drop `"test-jira-scripts.sh"` from
  `EXCLUDED_HELPER_NAMES` (`:10`).
- `tests/unit/tasks/test_integration.py` — remove the `_GUARDED` entry (`:69`)
  (else `AttributeError`).
- `mise.toml` — remove the `test:integration:integrations` leaf (`:369-372`) and
  its roll-up entry (`:388`).
- `tests/unit/tasks/test_mise.py` — remove the `_LAUNCHER_DEPENDENTS` entry
  (`:55`) (partition equality).
- `tests/unit/tasks/test_python_coverage.py` — remove `MOCK_JIRA` (`:33`) and
  its usages; `RUFF_JUSTIFIED_EXCLUDES` → `{"workspaces"}`.
- `pyproject.toml` — strip the `mock-jira-server.py` `extend-exclude` line
  (`:79`), leaving `extend-exclude = ["workspaces"]`; clean the stale mock-server
  comments (`:74-76`, `:118-119`, `:132-135`).

#### 4. Whole-repository assertions and final artefacts

**Files**: `meta/inventories/0211-{removal-set,suite-audit,reconciliation,
divergences}.md` (jira rows + finalisation), `meta/work/0171-…md` (`##
Decisions`).

Record the shared-asset sweep (the grep command and its output, excluding
`meta/`, over the four cluster `test-helpers`/`test-fixtures` paths plus
`mock-jira-server`/`mock-linear-server`, returning only in-cluster hits), and
the whole-repository `jq`/`curl` audit (expected **empty** set). Assert the
empty survivor set now that both providers are repointed.

Also record the **jira generator provenance** here (the linear provenance is
recorded in Phase 2 item 4 at that track's boundary) so a future maintainer can
revive a generator without reconstructing it: the exact revision at which
`mock-jira-server.py` and the jira bash cluster last existed on `main`, the
scripted capture command and its full output for the jira `bash-exit-codes.txt`,
and each committed jira golden's provenance (mock-served, live-anchored against
0210's 2026-08-18 contract evidence per Decision 8). This is the working-tree
substitute for the deleted generators — after Phase 4 neither exists except via
VCS history.

### Success Criteria

#### Automated Verification:

- [ ] `ls skills/integrations/jira/scripts/*.sh` and
      `ls skills/integrations/linear/scripts/*.sh` each match nothing;
      `mock-jira-server.py`/`mock-linear-server.py` do not exist
- [ ] The whole-repository `jq`/`curl` `allowed-tools` survivor set is empty:
      `grep -rn "Bash(jq\|Bash(curl" skills/` returns nothing
- [ ] Dispatch coherence green both directions and both tokens witnessed by a
      metacharacter-free invocation: `mise run build-system:check`
- [ ] `test-skill-write-gate.sh` green for every jira and linear write skill
      (confirm precedes mutation); no repointed body cites a pre-remap exit code
      (doc-vs-binary parity)
- [ ] Stale-library, exec-bit, python-coverage, ruff-equality, mise-partition
      guards all green: `mise run check`
- [ ] `_EXPECTED_INTEGRATIONS_SUITES`, the `integrations` task, its `mise` leaf,
      its `_GUARDED` entry and its `test_mise` member are gone; `_DUAL_USE_
      SCRIPTS` and its test are retired
- [ ] No Python remains in the `cli/` test lane (mock servers gone; neither
      client crate's dev-deps nor `tasks/` reference them)
- [ ] **`mise run` exits 0 end to end** — the child merge boundary

#### Manual Verification:

- [ ] Every jira and linear `SKILL.md` body invokes `accelerator jira …` /
      `accelerator linear …`; the write flows still gate before a mutation
- [ ] The reconciliation table reconciles to 17 executables + 5 libraries + 3
      data assets (jira) and 10 + 2 (linear), every "internal helper" naming its
      subsuming subcommand
- [ ] The divergences ledger names a real, passing test per row (search `70-73`
      remap, preview-intent [`test-skill-write-gate.sh` + the stdout-before-
      mutation assertion], dropped auth cleartext, dual-use exemplar-coverage
      loss [detection retained], declared-vs-behavioural exit-code
      disagreements, Jira usage-code behaviour, ADF record-stream removal, any
      search-envelope client-vs-bash shape gap)
- [ ] 0171's `## Decisions` records `linear-graphql.sh`'s production-script
      classification, the six-declarer `jq`/`curl` audit (post-change empty),
      the reverse cross-cluster sweep, and every fixture's provenance

---

## Testing Strategy

### Unit / crate tests

- Per-subcommand request assertion (method, path/GraphQL document, body) against
  a `http-test-support` mock, and the parsed response against a fixture. Flows
  issuing two POSTs assert the `/graphql` hit count and per-hit bodies (not just
  `last_body`).
- Byte-exact stdout goldens (`Vec<u8>`) for every subcommand; the strict
  contracts (tab-separated resolver, bare key/identifier, six `.data.issue.*`
  paths) preserved exactly. Stderr goldens for the composed-query `INFO:` audit
  line and its `--quiet` suppression; search JSON-envelope goldens
  (`.data.issues.nodes[]` + `.truncated` for Linear; `issues[]` +
  `nextPageToken` for Jira).
- `exit_codes_parity.rs` per binary: equality against the captured
  `bash-exit-codes.txt` for non-allowlisted names, a count-pinned divergence
  allowlist asserting remapped values for the deliberate divergences, per-`(flow,
  name)` uniqueness, fixed-count anti-vacuity, plus the "never `70`–`73`" and
  collision (`81`/`82`/`34`) assertions.
- A **behavioural exit-code test** per binary: drives the binary into each error
  class via the mock and asserts the *observed* exit code (guards runtime
  variant→code routing, not just const declarations).
- An `init verify` **no-token test**: sentinel token, every exit path, asserted
  absent from stdout and stderr.
- A `from_config`-branch test per binary (the production credential path), and a
  seam-revalidation test (unparseable → hard error; non-loopback/non-https →
  rejected before credentials attach).
- `cli_surface.rs` help golden per binary.

### Integration / build-system tests

- `lint:dispatch-coherence:check` both directions, per cutover phase, with the
  witness invocation metacharacter-free (bound, not merely invoked).
- `test-skill-write-gate.sh` (new): each write skill's confirm step lexically
  precedes its `accelerator <provider> …` mutation invocation.
- Doc-vs-binary exit-code parity: no repointed body cites a pre-remap integer.
- `lint:scripts:check` (stale-library + exec-bit) after each deletion.
- `test:unit` (python coverage, ruff equality, mise partition, exec-bit
  reconciliation) after each guard edit.

### Manual testing

1. Run each subcommand against the live Jira project / Linear team and diff
   against 0210's committed contract evidence.
2. Exercise a full write flow through the repointed `SKILL.md` and confirm the
   resolved-intent preview gate fires before the mutation.
3. Confirm `init verify` never prints a credential.

## Performance Considerations

Synchronous blocking `reqwest` (no async runtime), matching the client crates.
Timeout precedent for small JSON bodies is 10s connect / 30s read / 30s write
(`cli/github/src/octocrab_client.rs:8-10`); Linear's upload path is 60s
(`cli/linear-client/src/upload.rs:35`). No new cross-compilation cost — the
release upload set is derived from `DISPATCHED_SUBBINARIES`.

## Migration Notes

A mixed bash/`accelerator` state on `main` is safe between phases (0167's
validation precedent). Each phase is green independently; recovery from any
phase is a VCS revert. The transient floor value is 20 (after Phase 2) and
0/removed (after Phase 4) — the AC's "retire outright" is the end state, reached
in Phase 4.

**Init cache compatibility.** The `init` subcommands subsume cache production
(`site.json` for `@me` resolution, the refresh-fields custom-field cache that
`create`/`search` read). Either the binary reads the existing bash-written
format and paths unchanged (confirmed by a test over a bash-produced fixture
cache), or cache re-initialisation is an advertised post-upgrade step recorded
in 0171's `## Decisions` and the divergences ledger **and** a test asserts
`create`/`search` fail closed (a clear error, not silent misbehaviour) against an
incompatible bash-era cache. Whichever branch is taken carries a test — the plan
does not leave the format compatibility unstated or unguarded. 0203 becomes a release-path dependency only if a copyleft component
is recorded; 0210 introduced none (`cli/licence-audit/new-trees.txt`), so it is
not currently blocking.

## References

- Work item: `meta/work/0211-integration-binaries-and-bash-cluster-retirement.md`
- Research:
  `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- Parent epic: `meta/work/0171-jira-and-linear-integrations.md`
- Blocked by: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`,
  `meta/work/0212-work-item-script-cutover.md`
- Precedent: `meta/inventories/0167-{removal-set,suite-audit,divergences}.md`,
  `meta/validations/2026-07-19-0167-config-command-and-invocation-contract-migration-validation.md`
- Registration checklist: `tasks/README.md:399-663`
- Exit-code precedent: `cli/work-cli/src/exit_codes.rs`,
  `cli/work-cli/tests/exit_codes_parity.rs`
- Thin-CLI precedent: `cli/collaboration-cli/`, `cli/corpus-cli/`
- Client crates: `cli/jira-client/`, `cli/linear-client/`,
  `cli/http-test-support/src/lib.rs`
