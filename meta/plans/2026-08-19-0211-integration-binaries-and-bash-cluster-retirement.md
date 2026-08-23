---
type: plan
id: "2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
title: "Integration Binaries and Bash Cluster Retirement Implementation Plan"
date: "2026-08-19T02:05:51+00:00"
author: Toby Clemson
producer: create-plan
status: approved
work_item_id: "work-item:0211"
parent: "work-item:0211"
derived_from: ["codebase-research:2026-08-17-0211-integration-binaries-and-bash-cluster-retirement"]
relates_to: ["work-item:0171", "work-item:0210", "work-item:0212", "work-item:0174", "work-item:0165"]
tags: [rust, jira, linear, integrations, cli, cutover, exit-codes, registration]
revision: "45fe2827ec4eab9404ea4fb851de05fcbd9b87b3"
repository: "accelerator"
last_updated: "2026-08-22T21:33:25+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Integration Binaries and Bash Cluster Retirement Implementation Plan

## Overview

Ship `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
over 0210's `jira-client` / `linear-client` crates, repoint the sixteen jira and
linear `SKILL.md` bodies at them, register both dispatch tokens end to end, then
delete both bash script clusters, their suites, their Python mock servers, their
shared test assets and the last bash residue the work-item cutover left behind.

The work is organised as **six independently-mergeable phases**: a decoupling
phase that frees `cli/jira-client`'s test lane from the Jira cluster, then two
provider tracks (Linear leading), then a residue-and-artefacts phase. Because
the integrations suite floor is a single shared count, the first track to land
decrements it and the second removes it outright.

## Revision note — 2026-08-22

This plan was approved at revision `9d9c07ed` (four review passes). It was
revised at `45fe2827` against the research's 2026-08-22 follow-up, taken
after **0212 merged**, and **re-reviewed (review-2, three passes) → APPROVE**;
status is `approved`, ready for implementation.

What changed, and why:

- **A new Phase 0.** `cli/jira-client`'s ADF differential shells out to two
  cluster scripts. Deleting the Jira cluster reds `mise run test` today. The
  approved plan classified those scripts as "internal, no subcommand" — right
  about the product surface, wrong about the deletion surface.
- **The dispatch band is `70`–`74`, not `70`–`73`.** 0212 added
  `E_DISPATCH_UNCONFIGURED` = 74. Every reservation clause and the "never emit"
  test widen by one code.
- **Write-skill bodies branch on a keyword, not on integers** (Decision 11).
  0212 moved the repository's skill contract off integer branching; the ~45
  integers survive as a machine-parity artefact, not as a body contract.
- **Fixtures move into the Rust tests** (Decision 15). The approved plan deleted
  188 fixture files and authored goldens fresh; the 95 Jira and 40 Linear
  scenario JSONs are `http-test-support` route fixtures in all but syntax.
- **Three bash residues enter scope** (Decisions 13, 14, 18):
  `scripts/work-common.sh`, the zero-suite `test:integration:work` task, and the
  dead bash-shelling helpers in `cli/tracker-support`.
- **The document of record folds into the module doc** (Decision 6, amended). No
  `cli/*/EXIT_CODES.md` exists anywhere; `work-cli` — the enforcement model this
  plan adopts wholesale — puts the table in `exit_codes.rs`'s module doc.
- **Line anchors, counts and evidence dates re-measured** throughout.

Addendum after review-2 (same day): a fresh seven-lens review of the `45fe2827`
revision surfaced one critical and thirteen major findings. The plan now carries
**Decision 20** (search binds an additive read-side client op — the port `search`
returns a stamps-only `Discovery` that cannot render State/Assignee/Status or
jira's pagination), **Decision 21** (init cache is read-compatible with a
fail-closed format marker), a **Decision 9 rescope** to the six fallible port
methods with `validate_update` recognised as infallible and every inline
`TrackerError` branch enumerated, a shared **`cli-test-support`** crate, a
**release-build guard** that `test-loopback` is never compiled in, Jira generator
provenance moved into Phase 4, a bidirectional write-flow keyword parity guard,
and a scenario **consumption** inventory test. A **second edit pass** (after the
review-2 re-review) then corrected the new majors those edits surfaced: Decision
21's marker semantics (absent marker = implicit bash-era version; fail-closed
only on a present-but-unrecognised one; check homed in the client cache-read
path), Decision 20's Jira op returning the requested `fields` map (so
`--fields`/`--render-adf` survive) over a **distinct** projection query (not the
shared `SEARCH` the sync `fetch_all` uses), the Decision 9 enumeration completed
(jira `fetch_all` `:586`, linear anchors corrected to `:340`/`:407`) and reframed
as a funnel-through-one-enum refactor, the behavioural test pinned to an
exhaustive error-enum match, `cli-test-support`'s own boundary tests, the Jira
override reusing a promoted `allowed_sites` with provenance-refusal tests, the
Jira module doc scoped to bands, and the discriminant carried in-envelope for
JSON subcommands.

## Current State Analysis

0210 (merged, PR #70) absorbed the entire provider surface into the client
crates, so the binaries are genuinely thin: parse args, assemble a credential
context, call a client method, render JSON or an error. 0212 (merged, PR #73)
deleted `skills/work/scripts/` entirely, discharging both blockers — 0211 is the
last of 0171's children.

Verified at revision `45fe2827`:

- **The mutation and read-detail surface is complete and unpinned, but search is
  a sync-only projection.** Both crates expose every write flow
  (comment/transition/attach) plus `show`, and are `cargo-public-api`
  **exempt** (`_ADAPTER`, `tasks/public_api.py:52-59`) — the binaries bind
  whatever the crates expose, with no snapshot to maintain on the clients.
  `RemoteTracker` is now **seven** methods, not four (`cli/tracker/src/lib.rs`),
  because 0212 added `search`, `preview_create` and `validate_update`. ⚠️ **The
  port `search` op returns `tracker::Discovery { found: Vec<(ExternalId,
  RemoteTimestamp)>, complete }` — external ids and timestamps only.** Both
  clients request minimal fields (jira `"fields":["updated"]` at
  `cli/jira-client/src/client.rs:335`; linear fetches `id identifier title
  updatedAt` and discards the title into the stamp list,
  `cli/linear-client/src/client.rs:51-67`), and jira's `discover` follows
  `nextPageToken` internally, collapsing the whole result to one `complete:
  bool`. The search `SKILL.md` bodies render State/Assignee/Status columns
  (`search-linear-issues/SKILL.md:66-71`, `search-jira-issues/SKILL.md:84-87`)
  and jira's `--page-token`/`--render-adf`/`--fields` browsing UX — none of which
  the port surface can carry. This is an additive read-side client change
  (Decision 20), not a Phase-1 golden reconciliation.
- **Two error conventions must be bridged.** Port ops return
  `tracker::TrackerError` (only `Retryable`/`Terminal`) whose numeric bash code
  is computed by each crate's `classify.rs`; the port-less flows return a
  `SurfaceError` carrying `E_*` symbolic codes. The binary obtains the port-op
  integer by receiving the `classify::Outcome` from the client's port-op path
  and calling the existing `bash_code(Outcome) -> u16` (Decision 9) — never by
  parsing `TrackerError.detail`. ⚠️ `SurfaceError::status` is `pub(crate)` in
  both crates: a binary can match on variants but cannot construct one.
- **The base-URL test seam does not exist in the crates, and is defended
  against.** `cli/jira-client/tests/auth.rs:326-352` sets the bash-era override
  env vars plus `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` and still expects
  `BadSite`: *"No environment escape hatch exists: the test seam is the
  constructor."* Each binary adds env→constructor plumbing itself (Decision 10),
  which is consistent with that posture; an env read inside a client crate would
  not be.
- **The bash exit-code landscape is asymmetric.** Jira skills branch on ~45
  distinct integers across nine tables; the Linear skills cite exactly one
  (`107` at `create-linear-issue/SKILL.md:92`) and are otherwise symbolic `E_*`
  names. Jira's `EXIT_CODES.md` is prose-only and **already wrong**
  (`:12` says usage errors exit `2`, `jira-request.sh:207` exits `1`); Linear's
  is machine-pinned by `test-linear-paths.sh:71-103` — a pin that lives inside
  the deletion set, so only the `work-cli` model survives.
- **⚠️ `cli/jira-client` depends on the Jira cluster at test time.**
  `tests/support/adf_oracle.rs:14-17` hard-codes `jira-adf-to-md.sh` and
  `jira-md-to-adf.sh`; `run_oracle` (`:62-96`) spawns `bash` against them from
  five call sites in `adf_differential.rs` (`:58`, `:114`, `:180`, `:193`,
  `:216`) plus `adf_differential_self_test.rs`, over 56 case directories, in the
  **default** nextest profile. Transitively this makes all three `.jq`/`.awk`
  data assets load-bearing for a live Rust test. Phase 0 exists for this.
- **`scripts/work-common.sh` is orphaned by the deletion, silently.** Its only
  consumers are `jira-common.sh:61`, `linear-common.sh:60` and two
  `work_resolve_default_project` calls in `jira-create-flow.sh:214` /
  `jira-search-flow.sh:253`. Because the file survives, the stale-entry guard
  never fires — it becomes dead code pinned at `tasks/lint/scripts.py:23` and
  `tests/unit/tasks/test_exec_bits.py:249`.
- **`test:integration:work` is a husk.** `tasks/test/integration.py:392-394`
  discovers zero suites (`skills/work/` holds no `test-*.sh`) and passes green
  running nothing.
- **The deletion set is 263 files / 21,422 lines**, not the ~17,650 the work
  item records: production `.sh` 34/7,994, data assets 5/746, suites 33/9,204,
  `test-fixtures/` 188/3,079, `test-helpers/` 3/399.

## Desired End State

`accelerator-jira` and `accelerator-linear` are registered dispatched
sub-binaries; every jira and linear `SKILL.md` body invokes `accelerator jira …`
/ `accelerator linear …` and branches on a structured stdout keyword; no
`SKILL.md` anywhere declares `jq`, `curl` or a jira/linear `scripts/` grant;
both bash clusters, their suites, their mock servers and their data assets are
gone, with every fixture a Rust test consumes carried into the Rust corpus and
every unported file ledgered; `scripts/work-common.sh`, the `integrations` and
`work` integration tasks, the seven cluster `SHELL_LIBRARIES` entries,
`_DUAL_USE_SCRIPTS`, the mock-server tripwire and `cli/tracker-support`'s dead
bash helpers are retired; and `mise run` exits 0 end to end.

Verification: `mise run` green at Phase 5's merge boundary, plus the recorded
artefacts under `meta/inventories/` and the `## Decisions` updates in 0171.

### Key Discoveries

- **Registration is the `design`-token diff minus the domain work.** The client
  crates already carry their `cli/pup.ron` rules (`:194-262`), probe pairs and
  public-API classification (0210). What is new is two composition-root `*-cli`
  crates plus the token wiring: `DISPATCHED_SUBBINARIES`
  (`tasks/shared/paths.py:29-37`, seven tokens), `_SUBBINARY_DESCRIPTIONS` + the
  exact-tuple pin (`tests/integration/tasks/test_github.py:36-51`, `:530-541`),
  `_SUBBINARY_MANIFESTS` with a **custom `cli/jira-cli/Cargo.toml` path**
  (`tasks/manifest.py:55-65` — the crate is not at `cli/jira/`),
  `_CLI_RELEASE_BINARIES` (`tasks/build.py:36-45`), `cli/Cargo.toml` members (35
  today) + `Cargo.lock`, `.gitignore` (`bin/{jira,linear}-*`, inserting after
  `bin/design-*` at `:53`), `public_api.py` `_EXEMPT_MEMBERS` for the two `-cli`
  crates, and docs. ⚠️ **No `*-cli` crate carries a pup rule and no composition
  root carries a public-API snapshot** — the work item's ninth criterion is
  stale on both counts.
- **The checklist is itself guarded.**
  `tests/unit/tasks/test_registration_docs.py` asserts thirteen points and that
  every named identifier resolves, so editing `tasks/README.md:499-663` can red
  the build.
- **The same-commit rule binds registration to repointing.**
  `tasks/shared/dispatch_coherence.py` fails a registered token with no witness
  (`:196-202`) and a skill invoking an unregistered token (`:210-216`);
  `tasks/README.md` requires checklist points 1, 2, 3, 4, 7 and 8 together.
- **Only six skills can witness a token.** The ten write skills declare bare
  `Bash`, disqualifying them (`dispatch_coherence.py:57-65`). The six read/init
  skills carry path-scoped rules and are the only candidates. 0212 landed the
  witness shape to copy — `sync-work-items/SKILL.md:8-10`, no bare `Bash`, no
  `Read`/`Write`, fenced-block invocations.
- **`http-test-support` is the inherited harness, and the bash scenarios are
  its fixtures in all but syntax.** A scenario JSON is
  `expectations[{method, path, capture_body, expect_body_contains, response:
  {status, headers, body}}]` — a near one-to-one map onto `Route::Json`,
  `Route::Headers`, `Route::Redirect` and `Route::Sequence`.
- **0212's frozen-corpus technique is the deletion pattern for a bash
  differential.** Both of 0212's bash differentials were replaced by a
  committed-corpus reader guarded by digests *before* their scripts went
  (`cli/work-adapters/tests/corpus_hashes.rs`,
  `cli/work-adapters/tests/sync_baseline_corpus.rs` with its
  `EXPECTED_CASE_COUNT` pin). Phase 0 applies it verbatim.
- **The exit-code oracle must be an independent frozen fixture.**
  `cli/work-cli/tests/exit_codes_parity.rs` no longer touches bash: it parses
  `src/exit_codes.rs` textually and compares against literals committed in the
  test file, under the stated rule that the oracle must not be *"re-derived from
  the `exit_codes.rs` constants it guards, which would be a tautology no
  accidental renumbering could red"*.
- **`main.rs` has two house shapes, and they do not compose.**
  `corpus-cli`/`collaboration-cli` use `Result<Outcome, kernel::Error>` + a
  shared `report` collapsing to `Refusal→2` / `1`; `work-cli` uses a rich `pub
  const : u8` taxonomy with inline `ExitCode` returns. Once a handler funnels a
  domain error through `kernel::Error` the variant information the ~45 codes
  need is erased. This child adopts the **`work-cli` shape wholesale** and
  borrows only the `collaboration-cli` HTTP-adapter *structure*. Clients are
  **synchronous** (blocking `reqwest`) — no `tokio`, no async bridge.
- **The precedent artefact set is 0167.** `meta/inventories/0167-{removal-set,
  suite-audit,divergences}.md` are the direct templates, and their governing
  rule transfers verbatim: *"A divergence nothing can detect is
  indistinguishable from a defect, so every row names a real, passing test."*
  ⚠️ `0167-suite-audit.md:31` points at a `0167-removal-set-references.md` that
  does not exist; this child folds the consumer sweep into the removal set
  rather than repeat a dangling reference.

### Decisions taken during planning

Recorded here and to be mirrored into 0171's `## Decisions`.

1. **Merge granularity — one work item, six phases.** Jira and Linear are
   independently-mergeable phase groups within 0211; the provider-seam split
   into sibling children is declined. The refuted size assumption is absorbed by
   the phases, not by decomposition.
2. **Preview resolved intent, not wire bytes.** The client crates expose no
   mutation-payload composer, so the write flows' `--print-payload`/`--describe`
   wire preview is **not** reproduced. Repointed bodies preview the resolved
   human-facing intent (title, target, resolved fields) and the binary executes
   atomically after confirm. The divergence is pinned by two automated tests,
   not a manual checkbox. **(a) Binary-level, observable seam**: run the write
   subcommand against a mock whose mutation route fails, and assert the
   resolved-fields preview line is already on stdout while the mutation route
   still has zero hits. **(b) `test-skill-write-gate.sh` (new, skills-lane)**:
   for each write skill it locates the confirm step and the `accelerator
   <provider>` mutation line, asserts the confirm step is **present** (a missing
   confirm fails, so ordering cannot pass vacuously) and lexically precedes the
   mutation, and fails when both sit in the same fenced block; a committed
   reversed-body fixture proves the guard fails. The `--print-payload`/
   `--describe` `argument-hint` and body preview steps are dropped from **every
   write skill that advertises them**.
3. **Drop the cleartext-credential subcommand.** `jira-auth-cli.sh` /
   `linear-auth-cli.sh` are not reproduced; credential validation folds into
   `init verify`, which resolves and checks the token without printing it. A
   test asserts `init verify` never emits the token.
4. **`resolve-fields` is an `accelerator jira` subcommand.** `jira
   resolve-fields` reproduces the tab-separated four-field contract and exit
   codes 108/109, reading config through the `config`/`config-adapters` crates
   (no shell-out), even though it makes no Jira API call. See Decision 17 for
   the two-producer reconciliation.
5. **`jira-emit-key.sh` → `jira create --emit key`** (a projection, carrying the
   distinct post-create non-retryable "created remotely but unwritable"
   semantic). Its exit code is **taken from the captured `bash-exit-codes.txt`
   fixture**, not asserted as a literal — the condition is built inline as
   `TrackerError::Terminal` with no numeric code (`cli/jira-client/src/client.rs:506-513`),
   and 16 is merely `NonJsonBody` in `classify::bash_code`, while the bash create
   flow cites the 100-107 range; the behavioural test then asserts the observed
   code and its distinct discriminant reason arm. **`jira-jql-cli.sh` dropped**
   (orphan, invoked only by its own test).
6. **Exit-code enforcement — the `work-cli` model, with a divergence
   allowlist, and the module doc as the document of record.** Handlers return
   `ExitCode` inline, **not** `Result<Outcome, kernel::Error>`. Each binary
   carries `exit_codes.rs` + a fixture-anchored parity test.

   *Amended 2026-08-22*: the document of record is the **`exit_codes.rs` module
   doc**, not a committed `cli/{jira,linear}-cli/EXIT_CODES.md`. No
   `cli/*/EXIT_CODES.md` exists anywhere in the workspace; `work-cli` — the
   model this decision adopts wholesale — states its table in the module doc,
   where the parity test that textually parses the same file cannot drift from
   it. A separate file would be a second place to go stale, and under Decision
   11 its audience is machine parity rather than skill authors.

   Because Decision 11 removed the human (skill-author) audience for the integers,
   the Jira module doc is **scoped to bands, classes and the genuinely
   non-obvious** — the safety-critical classes, each remap's *why*, the divergence
   rationale — and **names `bash-exit-codes.txt` as the authoritative name→integer
   contract** rather than re-enumerating all ~45 codes in prose. A line per
   mechanical code would duplicate the const names and the parity fixture and drift
   against the fixture that is the real contract; self-descriptive const names plus
   the fixture carry the mechanical mapping.

   The parity test is **not** a blanket textual-equality assertion: the
   deliberate divergences (search `70-73` remap, `81`/`82`/`34` per-provider
   restatements) live in a count-pinned allowlist where each allowlisted row
   asserts the *remapped* Rust value while the fixture keeps the original bash
   value, and every allowlisted name must appear in the divergences ledger.
   Non-allowlisted names assert equality. The count pin makes a silent allowlist
   addition fail.
7. **Write skills stay bare-`Bash`.** They need not witness a token (the six
   read/init skills do). Their load-bearing uses survive: `wc -c`
   (`attach-jira-issue/SKILL.md:70`) and the `source config-common.sh` writeback
   (`create-jira-issue/SKILL.md:113`) remain shell steps.
8. **Golden provenance — mock-served, live-anchored.** Bash-flow goldens are
   captured against the deterministic mock servers; 0210's committed
   live-tenant contract evidence is the reality anchor. *Amended 2026-08-22*:
   the evidence is dated **2026-08-21** and carries **jira 6 / linear 7**
   conformance records (not 2026-08-18, 4/5). Jira runs six because its
   `LiveClient` declares `can_nominate_indeterminate()` and
   `can_induce_truncation()` false (`cli/jira-client/tests/contract.rs:82`,
   `:90`). Each fixture's provenance is recorded in 0171's `## Decisions`.
9. **Port-op integers are read structurally, not parsed from `detail`.** Each
   crate already computes the granular code with `classify::bash_code(Outcome)
   -> u16` — but once a port op collapses to `tracker::TrackerError` the value
   is gone and `TrackerError` carries no structural code field. So the binary
   obtains the code **before** the collapse. The additive change is a
   **structured discriminant on the client's port-op surface, covering every
   failure branch of the six fallible port methods** — not only the
   `classify()`-routed ones: some branches build `TrackerError` inline without
   ever computing an `Outcome`. Achieving the compile-error-on-omission property
   is **not purely additive**: today `TrackerError` is built inline at scattered
   `.map_err(|_| TrackerError::…)` sites, so exhaustiveness only holds once every
   port-op error site is **funnelled through one internal rich reason enum** from
   which `TrackerError` is derived — an internal refactor of each client's error
   pipeline, not a bolt-on wrapper. With that funnel, adding a variant forces an
   `exit_codes.rs` arm; a new error site reusing an existing variant is caught by
   the behavioural exit-code test, not the compiler. The enumerated
   inline-without-`Outcome` branches per provider are:
   - **Jira**: `create`'s post-create unusable-identifier
     (`cli/jira-client/src/client.rs:507`), the search/discovery **compose error**
     (`:282`, `compose(...).map_err(Retryable)`), `resolve_project`/`preview_create`
     **read failure** via `surface_read_failure` (`:452-453`), and `fetch_all`'s
     unsafe-identifier pre-flight (`:586`, `identifier_is_safe(id).map_err(...)`).
   - **Linear**: `create`'s post-create unusable-identifier (`client.rs:340`) and
     `fetch_all`'s unsafe-identifier pre-flight (`client.rs:407`).
   The post-create case is exactly the "created remotely but unwritable" condition
   Decisions 2/5 must distinguish. The port-op wrappers return, on the error
   path, an enum carrying **either** a `classify::Outcome` **or** an explicit
   binary-relevant reason (`UnwritableIdentifier`, `UnsafeQueryId`,
   `ComposeRejected`, `ReadFailure`, …); `exit_codes.rs` maps both arms
   directly. No new `&TrackerError` accessor, no substring-parse. Contained
   inside the `_ADAPTER`-exempt client crate, touching neither the shared
   `tracker` port type nor the mutation surface.

   *Amended 2026-08-22*: the coverage obligation is **six fallible** port methods
   (`create`, `update`, `show`, `fetch_all`, `search`, `preview_create`).
   **`validate_update` is infallible** — it returns `tracker::ValidationOutcome`
   directly, not `Result<_, TrackerError>` (`cli/tracker/src/lib.rs:464-469`;
   both impls infallible at `cli/jira-client/src/client.rs:648` and
   `cli/linear-client/src/client.rs:479`), so its `Valid`/`Rejected` are
   success-path outcomes carried on a separate keyword, not an error class the
   discriminant covers.
10. **The base-URL seam is a validated credential destination; loopback is
    behind a test-only feature.** The `ACCELERATOR_{JIRA,LINEAR}_API_URL`
    override ships in the release binary but is admitted only when it is **https
    with an allowlisted host**. **Loopback admission is gated by a dedicated
    test-only cargo feature (`test-loopback`), enabled only by the integration
    tests' build config — never by `debug_assertions`**, which is on for any
    ordinary `cargo build`. The caller passes `allow_loopback = cfg!(feature =
    "test-loopback")` into the existing runtime-parameter
    `UploadTransport::new(allow_loopback, …)` / `url_is_allowed(url,
    allow_loopback)`; loopback stays a runtime bool the caller supplies, never a
    static/env switch inside a shared helper. A present-but-unparseable or
    non-admissible value is a hard usage error returned inline as
    `ExitCode::from(exit_codes::USAGE)`, never a silent fallthrough to
    `from_config`. The seam reconstructs `team_key`/`states` (Linear) and the
    upload transport identically to `from_config`, differing only in the
    endpoint. Destination admissibility reuses **each client's own complete
    https-destination check** — not a cross-provider function, not the host-only
    fragment — and the two providers are **asymmetric**:
    - **Linear**: `upload.rs::url_is_allowed(url, allow_loopback)` (promoted
      `pub`; enforces https, userinfo refusal, `*.linear.app` label match).
    - **Jira**: `auth.rs::base_url` stays **strict and unchanged** — no loopback
      parameter, rejects http, explicit ports and non-`*.atlassian.net` hosts.
      The release seam routes the override through it unchanged. Under
      `test-loopback`, the Jira seam reaches a mock by a dedicated gated branch
      that **constructs `Credentials` pointed at the override directly** (as
      `cli/jira-client/tests/support/client.rs` already does) and calls
      `JiraClient::new`, **bypassing** `base_url` — never by relaxing it.
11. **The repointed bodies branch on a structured stdout keyword; the integers
    survive as machine parity.** *(New, 2026-08-22.)* 0212 moved the
    repository's skill contract off integer branching:
    `sync-work-items/SKILL.md:108-109` declares *"The stdout report is
    authoritative. Read it for `unresolved` lines regardless of exit code"*, and
    `create-work-item/SKILL.md:549-569` has `work create --push` print
    `<keyword>\t<external_id>` with *"the authoritative outcome is the
    keyword"*.

    Every `accelerator jira` / `accelerator linear` subcommand therefore emits,
    as its **last stdout line**, a tab-separated discriminant
    `<keyword>\t<detail>` from a **closed, per-subcommand keyword set** declared
    in the crate and pinned by a golden. The set is a **typed enum with a
    `keyword(self) -> &'static str` projection** (the `PushOutcome::keyword`
    precedent, `cli/work/src/sync/push_decide.rs`), so exhaustiveness is
    compiler-checked and a new outcome variant cannot compile without a keyword;
    `keyword_surface.rs` is a golden-pin and backstop, not the sole guarantee.
    The repointed bodies branch on the keyword; the nine Jira in-body integer
    tables collapse to keyword tables and the Linear bodies' `E_*` names collapse
    likewise.

    The discriminant's **carrier depends on the subcommand's stdout shape**, so no
    subcommand's stdout stops being a single valid document:
    - **Text-emitting subcommands**: the discriminant is the trailing
      `<keyword>\t<detail>` stdout line.
    - **JSON-emitting subcommands** (`search`, `show`, `list-projects`/
      `list-fields`/`list-teams`): the discriminant is a **top-level `outcome`
      field inside the JSON envelope**, not a trailing non-JSON line, so stdout
      stays a single parseable JSON document robust to a future `jq` pipe or
      machine consumer.
    - **Strict positional subcommands** whose stdout is parsed field-wise —
      `jira resolve-fields` (the tab-separated four-field line, trailing newline
      load-bearing) and every bare-identifier projection (`create --emit key`) —
      **suppress** the discriminant entirely.
    Each carrier and each suppression is pinned by its byte-exact golden, and the
    suppressed/JSON-embedded/trailing set is recorded in the `exit_codes.rs`
    module doc so the split is discoverable rather than implicit.

    The integers do **not** go away. `exit_codes.rs` still reproduces every
    captured bash value, the parity fixture still pins it, and the behavioural
    exit-code test still asserts the observed code per class — that is what
    satisfies the third acceptance criterion and what keeps a machine consumer
    (a future bridge, a CI wrapper) working. What changes is only which of the
    two contracts the *skill bodies* read.

    Three consequences. The keyword set is closed and count-pinned, so an
    unmapped condition is a test failure rather than an unhandled body branch. A
    doc-vs-binary parity guard enumerates all sixteen repointed bodies and
    asserts every keyword they branch on exists in the binary's declared set,
    with an anti-vacuity match count and a committed stale-keyword fixture
    proving it fails. And the strict stdout contracts of Decision 2 and the
    goldens are unaffected: the discriminant is an additional final line, and
    the bare-identifier projections (`create --emit key`) suppress it, which a
    golden pins.
12. **The ADF differential is converted to a frozen oracle before deletion.**
    *(New, 2026-08-22.)* `cli/jira-client/tests/support/adf_oracle.rs` spawns
    the two cluster driver scripts. Phase 0 captures the oracle's observed
    output per case into the committed corpus and rewrites `run_oracle` to read
    it, applying 0212's technique (`mapper_differential.rs`,
    `sync_baseline_shellout_parity.rs` were each retired this way). The
    comparison helpers and `adf_differential_self_test.rs` are untouched, so the
    proof that the comparison can fail survives the conversion unedited. This is
    a phase, not a line item, and it must precede Phase 4.
13. **`scripts/work-common.sh` is retired by 0211, not handed to 0174.** *(New,
    2026-08-22.)* Its four consumers are all inside the two clusters, so it is
    dead the moment they go — and because the file survives deletion, the
    stale-entry guard does not fire and 0174 would inherit a silently-dead entry
    pinned in two lists. It goes in Phase 5, after both clusters, with its
    `SHELL_LIBRARIES` and `_RECONCILED_LIBRARIES` members.
14. **The `test:integration:work` husk is retired alongside `integrations`.**
    *(New, 2026-08-22.)* 0212 left the task wired at
    `tasks/test/integration.py:392-394` discovering zero suites and passing
    green over nothing. 0211 is already removing its sibling `integrations`
    task and touching the same three mirrors, so retiring both in one edit costs
    nothing and leaves no green-over-nothing task behind. It has no `_GUARDED`
    entry (no floor), so the edit is the task, the `mise.toml` leaf (`:364-367`)
    and roll-up member (`:387`), and the `_LAUNCHER_DEPENDENTS` member
    (`tests/unit/tasks/test_mise.py:50-57`).
15. **Fixture migration bar — port what a Rust test consumes, ledger the
    rest.** *(New, 2026-08-22.)* Every scenario JSON becomes an
    `http-test-support` route fixture **where a `*-cli` test drives that
    condition**; the 43 `adf-samples/` files reconcile against the 56 committed
    `cli/jira-client/tests/fixtures/adf/` cases. Every unported file carries a
    row in `meta/inventories/0211-fixture-reconciliation.md` naming why —
    superseded by an existing Rust case, duplicate of another scenario, or
    already-dead (the ten `api-responses/` files have zero consumers today). The
    ledger is count-pinned against the pre-deletion file list, so silence is
    impossible. A per-binary **inventory test asserts every file under
    `tests/fixtures/scenarios/` is referenced by at least one test**, so a row
    marked "ported" provably means "consumed": count-pinning accounts for files,
    the inventory test forces consumption — without it a scenario could be ported
    and counted yet driven by nothing, re-creating the dead surface this bar
    exists to prevent. "Referenced" means **anchored to a meaningful request/
    response or byte-exact golden assertion**, not merely loaded into a mock — the
    test "if this scenario's response changed, would a test fail?" must answer yes
    for every ported file. Porting all 188 mechanically is declined: it would
    re-create the dead set and add test surface nothing drives.
16. **The reserved dispatch band is `70`–`74`.** *(New, 2026-08-22.)* 0212
    inlined the dispatch oracle at `cli/tracker/tests/errors.rs:29-57` with five
    rows, adding `E_DISPATCH_UNCONFIGURED` = **74** (`AboveThePort`). A binary
    emitting 74 for a provider condition would read as "tracker wired but
    unconfigured" at the composition root. The "never emit" test covers
    `70`–`74`, and the search remap moves off the whole widened band.
17. **The two-producer tab contract is reconciled by prefix.** *(New,
    2026-08-22.)* After this child the same resolved-fields tuple has two
    producers under two tokens: `accelerator jira resolve-fields` emits the
    four-field form (Decision 4) and `accelerator work create --push --dry-run`
    emits a five-field form prefixed by the tracker name
    (`create-work-item/SKILL.md:504-513`). They are **not** unified — the work
    form is tracker-agnostic and carries a `linear` arm the jira form cannot.
    Instead the jira form is defined as the work form's tail with the tracker
    prefix removed, a test asserts the two agree field-for-field on the same
    fixture config, and `create-jira-issue/SKILL.md`'s "single source of truth"
    prose is rewritten to name the relationship rather than deny it. Both
    producers render **one shared resolution path** in `config`/`config-adapters`
    (jira-cli reads it with no shell-out; `work create --push --dry-run` reads the
    same source), so the tab contract is a formatting difference over one
    computation, not two computations kept in sync by a test.
18. **`cli/tracker-support`'s dead bash helpers go.** *(New, 2026-08-22.)*
    `run_bash` (`tests/support/mod.rs:136`) and its only caller-target
    `repo_root` (`:46`) have had no consumer since 0212 deleted
    `mapper_differential.rs`; the module doc (`:1-5`) still names that file.
    `mapper_differential_self_test.rs` uses the rest of the module and is
    unaffected.
19. **The shared-asset sweep is recorded with a declared exclusion list.**
    *(New, 2026-08-22.)* The sixth acceptance criterion expects the grep to
    return only in-cluster hits; it cannot, because `CHANGELOG.md` (immutable
    release record), `skills/work/create-work-item/evals/benchmark.json` (frozen
    eval transcript) and the gitignored generated docs-site mirror pages all
    name the paths. This is 0212's own lesson — *"a literally-empty grep is
    unreachable; declare the exclusions up front"*. The recorded sweep names its
    exclusions and asserts the residual set is empty.
20. **Search gets an additive read-side client surface.** *(New, 2026-08-22.)*
    The port `search` op returns `tracker::Discovery` (external ids +
    timestamps), which cannot carry the State/Assignee/Status columns the search
    bodies render nor jira's `--page-token` cursor round-trip (Current State).
    Reproducing search over `Discovery` would ship a degraded table — a
    user-visible regression — so each client crate gains a **read-only additive
    op** over a **distinct search-projection query, not the shared `SEARCH`
    const** that `fetch_all`/`fetch_page` (the sync engine's bulk read) use — so
    widening the projection never changes the port `fetch_all` request shape or
    the linear complexity/rate-limit budget the sync path spends. This is additive
    inside the `_ADAPTER`-exempt client crates (no public-API snapshot, no pup
    rule), touches neither the shared `tracker` port nor any mutation surface, and
    does **not** re-open the mutation-payload compose seam Decision 2 declines.
    - **Linear**: a dedicated projection query selecting `identifier`, `title`,
      `state { name }`, `assignee { name }` and the page cursor — the title no
      longer discarded.
    - **Jira**: the op returns **each issue's requested `fields` map** (the raw
      Jira `fields` object for the resolved `--fields` set, `description`
      included), **not a fixed four-field projection** — so arbitrary `--fields`
      passthrough and `--render-adf` description/custom-ADF rendering both survive
      — plus the `nextPageToken` cursor surfaced to the caller for `--page-token`.
    The search subcommands bind this op, not the port `Discovery`. The Phase 1/3
    envelope goldens pin the richer JSON the binary now emits. This lands in the
    binary phase (Phase 1 for linear, Phase 3 for jira), before the corresponding
    cutover.
21. **Init cache is read-compatible with bash-era state, and fails closed on an
    unrecognisable one.** *(New, 2026-08-22.)* `init` subsumes cache production
    (`site.json` for `@me`, the refresh-fields custom-field cache that
    `create`/`search` read to compose live mutations). Bash-era caches carry **no
    version envelope** — `site.json` is bare `{site, accountId}`
    (`jira-init-flow.sh:113-117`), `fields.json` raw field JSON
    (`jira-fields.sh:75`), matching today's markerless Rust writes
    (`cli/jira-client/src/cache.rs`). So an **absent marker is classified as the
    implicit bash-era version and reads unchanged** — no existing install must
    re-initialise on the happy path. Fail-closed fires only on a **present-but-
    unrecognised marker, or a shape that does not parse as the known bash-era
    layout** — a stale future-versioned or corrupt cache that would otherwise feed
    wrong custom-field values into a live-tenant mutation (the one
    non-VCS-recoverable surface). Because that cache is provider-specific and is
    read at the compose site inside the client crate, **the marker check fires in
    the client crate's cache-read path** (reusing the `LegacyPolicy` fail-closed
    precedent), not in the provider-agnostic `config`/`config-adapters`, which
    holds only the marker constant and a generic version-check helper. Three test
    arms: a real **markerless bash fixture passes** (the migration population), a
    present-but-unrecognised marker **fails closed before any mutation route is
    hit**, and a first `init` run stamps the marker so subsequent reads are
    versioned.

## What We're NOT Doing

- **Not** splitting 0211 into sibling children (Decision 1).
- **Not** reproducing the wire-payload preview or the cleartext-auth subcommand
  (Decisions 2, 3).
- **Not** unifying the two resolved-fields producers into one (Decision 17).
- **Not** touching the sixteen generated docs-site reference pages
  (`docs-site/src/content/docs/reference/skills/` is gitignored, `.gitignore:26`
  — rebuilt from `SKILL.md`).
- **Not** changing `EXPECTED_INJECTION_SKILLS = 42`
  (`tasks/lint/skill_permissions.py:48`) or `.claude-plugin/plugin.json:16-17`
  — the skills survive; only their bodies repoint and their `scripts/` dirs go.
- **Not** adding a mutation-payload compose seam to the client crates (that
  would re-open 0210).
- **Not** retiring the remaining thirteen repo-root `scripts/*.sh`
  `SHELL_LIBRARIES` entries or the exec-bit guard itself — those are 0174's,
  unblocked by this child. Only `work-common.sh` is taken (Decision 13).
- **Not** narrowing the ten write skills' bare `Bash` grant (Decision 7).
- **Not** porting the 188 bash fixtures mechanically (Decision 15).

## Implementation Approach

Phase 0 decouples `cli/jira-client`'s test lane from the Jira cluster, so the
later deletion cannot red the tree. Each provider track then runs binary-first,
then cutover-plus-deletion:

1. **Binary phase** (Linear = Phase 1, Jira = Phase 3): build the `*-cli` crate
   over the client crate, TDD per subcommand against the migrated scenario
   fixtures, capture the pre-deletion exit-code and stdout oracle **while the
   bash still exists**, and pin the exit-code mapping against the committed
   fixture. The crate is a workspace member and `public_api` exempt but **not
   registered** — it ships no skill binding yet, which is coherent.
2. **Cutover + retirement phase** (Linear = Phase 2, Jira = Phase 4): register
   the token and repoint that provider's eight `SKILL.md` bodies **in one
   commit**, drop `jq`/`curl` + the script glob from its three read/init
   frontmatters, then delete that provider's bash cluster and retire its
   provider-specific guards. Linear decrements the shared floor 32→20; Jira
   removes it outright.
3. **Residue phase** (Phase 5): the bash that only dies once both clusters are
   gone, the whole-repository assertions, and the artefacts.

The binary phases capture the oracle; the cutover phases consume it and delete
the generators. Sequencing guarantees capture precedes deletion.

---

## Phase 0: Freeze the ADF oracle (`cli/jira-client`)

### Overview

Convert `cli/jira-client`'s bash-shelling ADF differential to a frozen-corpus
reader, so deleting `jira-adf-to-md.sh`, `jira-md-to-adf.sh`,
`jira-adf-render.jq`, `jira-md-tokenise.awk` and `jira-md-assemble.jq` in Phase
4 cannot red `mise run test`. Independently mergeable and independent of both
binaries; it must land before Phase 4.

This is 0212's technique applied verbatim. Its two bash differentials were each
replaced by a committed-corpus reader guarded by digests before their scripts
went, and 0212's validation records the residual risk this inherits: with the
live oracle retired, byte-identity rests on the digest manifest plus the rule
that the corpus is never regenerated from Rust output.

### Changes Required

#### 1. Capture the oracle output into the corpus

**File**: `cli/jira-client/tests/support/capture-adf-oracle.sh` (new, committed,
executable).

A committed script — not a hand transcription — that walks every case directory
under `cli/jira-client/tests/fixtures/adf/`, runs the matching driver over the
case input, and writes the observed result beside it:

- `render-*` cases: `bash jira-adf-to-md.sh < adf.json`, unseeded, writing
  `oracle.out` (raw bytes) and `oracle-status.txt`.
- `assemble-*` cases: `bash jira-md-to-adf.sh < input.md` with
  `JIRA_ADF_LOCALID_SEED=1`, writing the same pair.

The script's own output — command, revision, per-case status — is the
provenance record folded into the Phase 5 artefacts. Its exit-status handling
mirrors `run_oracle`: a missing `bash` or `jq` is a failure, never a skip.

#### 2. Rewrite the oracle module to read the frozen corpus

**File**: `cli/jira-client/tests/support/adf_oracle.rs`.

`run_oracle(script, input, seeded) -> Run` becomes `frozen_oracle(case: &Path)
-> Run`, reading `oracle.out` and `oracle-status.txt`. The two `pub const`
script paths (`:14-17`), `repo_root` (`:19-24`) and the `std::process::Command`
machinery (`:62-96`) are deleted. `cases`, `case_name`, `render_disagreement`
and `assemble_disagreement` are **unchanged**, so
`adf_differential_self_test.rs` needs no edit and continues to prove the
comparison can reject — the property most at risk in a conversion like this.

The five call sites in `adf_differential.rs` (`:58`, `:114`, `:180`, `:193`,
`:216`) swap `run_oracle(ADF_TO_MD, adf.as_bytes(), false)` for
`frozen_oracle(&case)`. The module docs at `adf_oracle.rs:1-4` and
`adf_differential.rs:1-7` currently describe a *running* pipeline and are
rewritten to describe a captured one, naming the capture script and the
never-regenerate rule.

#### 3. Digest-pin the frozen outputs

**Files**: `cli/jira-client/tests/fixtures/adf/oracle-manifest.txt`,
`cli/jira-client/tests/adf_oracle_manifest.rs`.

A sha256-per-file manifest over every `oracle.out`/`oracle-status.txt`, checked
by a test on the `cli/work-adapters/tests/corpus_hashes.rs` model, plus an
`EXPECTED_CASE_COUNT` pin on the `sync_baseline_corpus.rs` model so a silently
dropped case fails. The pin's value is the **final** case count — the 56
committed cases plus the `adf-samples/` ported in §4 — and both the manifest and
the pin are set **once, after §4 reconciliation completes**, so a pin fixed at 56
before the port cannot spuriously red the augmented corpus. `frozen_oracle`
**hard-fails on a missing or empty `oracle.out`/`oracle-status.txt`** (mirroring
`run_oracle`'s fail-not-skip posture), so a truncated corpus file cannot turn a
case's differential into a silent no-op that a bare digest-of-empty would accept.
The manifest header states the capture command, the revision the capture ran at,
and the rule that the corpus is regenerated only by re-running the capture script
against the bash **at that checked-out driver revision**, never from Rust output.

#### 4. Reconcile `adf-samples/` into the corpus

**Files**: `cli/jira-client/tests/fixtures/adf/` (new cases),
`meta/inventories/0211-fixture-reconciliation.md` (ADF rows).

The cluster's 43 `test-fixtures/adf-samples/` files are `.adf.json`/`.md` pairs.
Each is classified: already represented by one of the 56 committed cases
(recorded, with the case named), ported as a new case with its oracle output
captured in the same run, or ledgered as dropped with a reason (Decision 15).
The ledger's row count is pinned against 43. A ported sample's own committed `.md`
becomes its `expected.*`, so every ported case carries the **same double anchor**
(the captured `oracle.out` cross-checked against an independent committed
expectation) as the legacy 56 — not a single capture-only anchor a mis-capture
could freeze as ground truth.

### Success Criteria

#### Automated Verification:

- [x] The differential passes with the drivers **absent**, proving the
      decoupling rather than asserting it. The restore must run unconditionally so
      a failing run (the exact case this proves) cannot leave the tree displaced:
      `mv skills/integrations/jira/scripts/jira-adf-{to-md,render}.* /tmp;
      cargo nextest run -p jira-client; rc=$?; git checkout
      skills/integrations/jira/scripts/; exit $rc` (from `cli/`) — proven with all
      five deleted-in-Phase-4 assets absent (jj tree, restore via `mv`), 159 pass
- [x] `cargo nextest run -p jira-client` green in the default profile
- [x] The oracle manifest matches every frozen file and the case count is pinned
      to the final total (56 + ported `adf-samples/`), set once after §4 — **57**
- [x] `frozen_oracle` hard-fails on a missing or empty corpus file, and any
      case's `oracle.out` agrees with its existing committed `expected.*`
- [x] `adf_differential_self_test.rs` is unedited and still fails on a planted
      wrong rendering
- [x] No `Command::new("bash")` remains anywhere under `cli/jira-client/tests/`
- [x] Full read-only gate: `mise run check`

#### Manual Verification:

- [x] The ADF ledger accounts for all 43 `adf-samples/` files, each row naming
      its disposition and, for a "already represented" row, the case that
      represents it

---

## Phase 1: Linear binary (`cli/linear-cli`)

### Overview

Build `accelerator-linear` over `linear-client` with every subcommand, a
scenario-backed golden per flow, the exit-code taxonomy pinned to a captured
fixture, the keyword discriminant, and the preview-resolved-intent gate. Not
registered; the linear bash cluster stays in place as the capture source.

### Implementation progress (2026-08-23) — IN PROGRESS

Landed and committed (all green through the gates run at each step):

- `736162a3` — `http-test-support` per-hit body log (`bodies()`); the boundary
  test in `server.rs` is done.
- `9982aad9` — `linear-client` Decision 9 (`LinearFailure` funnel over
  create/update/show, `From<LinearFailure> for TrackerError`) and Decision 20
  (`search_detailed`). Also added `show_detailed` (a plan gap: Decision 20
  covered search only; `show` has the same stamps-vs-detail problem) and
  promoted `url_is_allowed` to `pub`.
- `35097e05` — `cli-test-support` crate (scenario→Route loader + exit-code
  parser + boundary tests) and the `accelerator-linear` skeleton: all eight
  subcommands, the base-URL seam (`test-loopback` feature + compile guard +
  marker static), `exit_codes.rs` (document of record), the typed keyword
  discriminant. Registered as a workspace member and `public_api` exempt; no
  token yet.
- `cd01a844` — exit-code capture (`capture-bash-exit-codes.sh` →
  `bash-exit-codes.txt`) + `exit_codes_parity.rs` (search remap `70-73`→`75-78`
  count-pinned allowlist) + `keyword_surface.rs` + `cli_surface.rs` help golden.
- `5c51eb73` — per-flow subprocess harness (`tests/support/mod.rs`) driving the
  binary against a mock through the seam; `flow_show` proven end to end.
- `fcf48ca9` — `flow_search` (envelope + projection + stderr audit + `--quiet`)
  and `flow_init` (no-token guarantee).

Landed since (all green through the per-step gates):

- Mutation flow tests — `flow_comment`/`flow_create`/`flow_transition`/
  `flow_update`/`flow_attach`. `resolve_state` is a **local** catalogue lookup,
  so create and transition are single-POST; the binary-attach three-step
  (`fileUpload` POST → raw PUT → `attachmentCreate` POST) is the genuine linear
  multi-POST case, asserted per hit.
- Behavioural exit-code test (`flow_errors`) and the seam + `from_config` tests
  (`flow_seam`), the scenario-inventory test (`scenario_inventory`), the stderr
  `E_*` diagnostics golden (`stderr_diagnostics`), byte-exact stdout goldens for
  the JSON subcommands (`stdout_goldens`), and the 40-row linear fixture ledger.
- The release-binary byte-scan (`tasks/build.py::_assert_no_test_loopback`) with
  its `test_build.py` coverage.

Remaining in this phase: the `mise run check` full read-only gate and the manual
live-tenant spot-check. One design divergence to fold into Phase 2's divergences
ledger: errors route to exit codes + `E_*` stderr, not to keywords, so the
"every error class → one keyword" criterion is reframed (keywords carry success
outcomes only).

### Subcommand surface and the reconciliation mapping

Ten executables + two libraries map as:

| Bash executable | Disposition |
|---|---|
| `linear-create-flow.sh` | `linear create` |
| `linear-update-flow.sh` | `linear update` |
| `linear-show-flow.sh` | `linear show` |
| `linear-search-flow.sh` | `linear search` (`filter::compose` + the read-side projection op, Decision 20 — not the stamps-only `Discovery`) |
| `linear-comment-flow.sh` | `linear comment add` |
| `linear-transition-flow.sh` | `linear transition` (`resolve_state` + `transition`) |
| `linear-attach-flow.sh` | `linear attach --url \| --file` |
| `linear-init-flow.sh` | `linear init verify \| list-teams \| discover` |
| `linear-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `linear-graphql.sh` | dropped — internal transport, subsumed by `linear-client` |
| `linear-common.sh`, `linear-auth.sh` (libs) | subsumed by the crate |

⚠️ Two counts the work item gets wrong and the reconciliation must use: Linear
has **9** `SKILL.md`-reachable entrypoints, not 10 — `linear-graphql.sh` is
named by no `SKILL.md`, not even in prose, and is reachable only through the
wildcard glob. And Linear's dispatch-mode count is **6**, not "roughly 15": nine
of its ten executables are flag-and-positional only.

⚠️ **`linear-init-flow.sh`'s bare mode does not block on `read`** — there is no
`read` in the file; it runs `_linear_verify`, prints the team list to stderr and
returns a "re-run with `--team-id`" instruction (`:254-255`). The TTY-policy
obligation is **Jira-only** (Phase 3).

### Changes Required

#### 1. Crate scaffold

**Files**: `cli/linear-cli/Cargo.toml`, `cli/linear-cli/src/main.rs`,
`cli/linear-cli/src/cli.rs`, `cli/linear-cli/src/outcome.rs`,
`cli/linear-cli/src/exit_codes.rs`, `cli/linear-cli/src/keywords.rs`, plus one
module per flow under `cli/linear-cli/src/`; `cli/linear-client/` to return
a structured discriminant on the port-op error path across all six fallible port
methods (Decision 9) **and to add the read-side search projection op**
(`state { name }` + `assignee { name }`, title preserved, Decision 20); and a
new shared **`cli/cli-test-support`** crate homing the reusable test machinery —
the `exit_codes.rs` textual parser, the scripted-capture skeleton, and the
scenario-JSON→`http-test-support` `Route` loader — consumed as a dev-dep by both
`*-cli` crates so that parser/loader/capture logic lives in one place rather than
being mirrored (`cli/Cargo.toml` members += `"cli-test-support"`,
`tasks/public_api.py` `_EXEMPT_MEMBERS` += it).

`Cargo.toml` mirrors `cli/collaboration-cli/Cargo.toml`: package + `[[bin]]`
`name = "accelerator-linear"`, mandatory `description = "The linear create|
update|show|search|comment|transition|attach|init sub-binary."`, `[lints]
workspace = true`; deps `linear-client`, `tracker`, `tracker-support`, `config`,
`config-adapters`, `kernel`, `clap = { workspace = true }`; dev-deps `tempfile`,
`http-test-support`, `cli-test-support`; and a `[features]` `test-loopback`
(default-off) enabled via the test target's dev path — never a crate-dir
`.cargo/config.toml` that could bleed into a release build invoked from the
directory. A compile guard `#[cfg(all(feature = "test-loopback",
not(debug_assertions)))] compile_error!("test-loopback must never be enabled in a
release build")` makes the never-in-release invariant enforced rather than
assumed. Because the `compile_error` rests on `[profile.release]` keeping
`debug-assertions` off, the plan adds a **byte-level guard on the staged release
binary** mirroring `tasks/build.py`'s `_assert_no_e2e_insecure` (`:333-347`): a
`test-loopback`-only marker symbol is grepped out of each staged
`accelerator-{jira,linear}`, so the guarantee is build-mechanism-independent
rather than resting on the cargo invocation string (which builds the whole
workspace with no per-binary `--features`).

`main.rs` follows the **`work-cli` inline-`ExitCode` shape** (Decision 6): a
two-level clap `Cli` (`#[command(name = "accelerator-linear",
disable_version_flag = true)]`), and handlers that match on the domain
outcome/error enums and return `ExitCode::from(exit_codes::X)` directly. It does
**not** funnel domain errors through `kernel::Error` + a collapsing `report`.
Only the crate layout, the base-URL seam and the synchronous client wiring are
borrowed from `collaboration-cli`. The repeated per-handler
credential-context/seam preamble that `work-cli`'s `main.rs` inlines (~13 lines ×
each handler) is extracted into **one per-crate `context(...)` helper** returning
the assembled context or an early-return `ExitCode`, so each handler stays
parse→call→render rather than replicating the setup across ~19 handlers.

#### 2. The base-URL seam (validated credential destination)

**File**: `cli/linear-cli/src/main.rs`

The override ships in release but is admitted only as a validated credential
destination before any token attaches (Decision 10). The revalidation and the
unparseable/non-admissible hard error are **binary-owned** — returned inline as
`ExitCode::from(exit_codes::USAGE)` — and loopback admission is gated behind the
test-only `test-loopback` feature:

```rust
fn api_base_uri() -> Result<Option<Url>, UsageError> {
    let Some(raw) = std::env::var_os("ACCELERATOR_LINEAR_API_URL") else {
        return Ok(None);
    };
    let raw = raw.to_string_lossy();
    if !url_is_allowed(&raw, cfg!(feature = "test-loopback")) {
        return Err(UsageError::BadApiUrl);
    }
    Ok(Some(Url::parse(&raw).map_err(|_| UsageError::BadApiUrl)?))
}
```

`UsageError::BadApiUrl` is the binary's own type; `url_is_allowed(url: &str,
allow_loopback: bool)` is Linear's own complete https-destination check, promoted
`pub` (`cli/linear-client/src/upload.rs:189`) — it takes the **raw string** and
parses internally, so the seam validates the string and keeps the parsed `Url`
only for the endpoint; `allow_loopback` is a **caller-supplied runtime bool**
fixed to `cfg!(feature = "test-loopback")`.

When present, the seam branch reconstructs the **whole** client the way
`from_config` does, differing only in the GraphQL endpoint — overriding
`Transport` alone is not enough:

- `Transport::new(endpoint, credentials, …)` for the GraphQL endpoint.
- `UploadTransport::new(allow_loopback, retry_delay)` in place of
  `UploadTransport::production()`, so the attach flow's server-nominated upload
  host is admitted under the test feature and never in an ordinary build.
- `team_key` via `catalogue_team_key(integrations_root)` and `states` via
  `CatalogueStates::load(integrations_root)`, identical to `from_config`, so
  `transition`'s `resolve_state` sees a populated catalogue.
- `LinearClient::new(transport, upload, team_key, states)`.

Otherwise `LinearClient::from_config(context, integrations_root)`. Note the
constructor asymmetry: Linear's `from_config` takes an extra `integrations_root:
&Path` (`cli/linear-client/src/client.rs:106`) that Jira's does not.

#### 3. The exit-code taxonomy, the keyword set, and the captured oracle

**Files**: `cli/linear-cli/src/exit_codes.rs`, `cli/linear-cli/src/keywords.rs`,
`cli/linear-cli/tests/exit_codes_parity.rs`,
`cli/linear-cli/tests/keyword_surface.rs`,
`cli/linear-cli/tests/fixtures/bash-exit-codes.txt`.

**Capture — scripted, exhaustive, differential, while the bash exists.** The
capture is a committed script, not a hand transcription:

- **Declared half**: grep every `linear-*-flow.sh` for `readonly E_*=NN` (the
  `test-linear-paths.sh:95` idiom).
- **Behavioural half**: a differential harness that *executes* each flow against
  every error-scenario fixture and records the observed exit code (0210's D10
  differential precedent). The anti-vacuity count is derived from the capture
  output, not hand-picked.
- **Conflict rule**: **behavioural wins over declared**; names are namespaced
  per flow where a genuine collision remains, and the parse asserts each
  `(flow, name)` key is unique. Declared-vs-behavioural disagreements are
  reconciled as named divergence rows, never collapsed by the grep.

`exit_codes.rs` declares `pub const : u8` classes and maps `SurfaceError` / the
port-op code (from the surfaced `classify::Outcome`, Decision 9) / `ClientError`
variants onto them. Its **module doc is the document of record** (Decision 6,
amended). Three enforcement layers guard the mapping:

- `exit_codes_parity.rs` compares `exit_codes.rs` against `bash-exit-codes.txt`.
  Non-allowlisted names assert equality; a **count-pinned divergence allowlist**
  holds rows asserting the *remapped* Rust value, and every allowlisted name
  must appear in the divergences ledger. Following
  `cli/work-cli/tests/exit_codes_parity.rs`, the oracle is independent of the
  constants it guards — never re-derived from them.
- A **behavioural exit-code test per error class** drives the binary into each
  condition and asserts the *observed* exit code. Its class set is pinned to the
  **error-code taxonomy directly — an exhaustive `match` over the error enums**,
  **not** the keyword set and **not** the bash capture. The keyword axis is too
  coarse (Decision 11 permits many classes → one keyword, so two classes with
  different exit codes could collapse to one driving case), and several classes
  are binary-owned and absent from bash (the ~nine argument-validation `E_*`
  names, `USAGE`, `BadApiUrl`). The exhaustive match makes a new variant a
  compile error until it is driven. Network classes come from mock
  responses; the **non-network classes** (usage errors, unparseable/
  non-admissible `ACCELERATOR_LINEAR_API_URL`, missing-config/`from_config`
  failures, the post-create exit-16 semantic) are triggered via their real
  sources, so the routing guarantee covers every class, not only mock-reachable
  ones.
- `keyword_surface.rs` pins the closed keyword set per subcommand (Decision 11)
  and asserts every error class maps to exactly one keyword — so an unmapped
  condition fails here rather than leaving a body branch unreachable.

Linear-specific contract: the bash skills key on **symbolic `E_*` names in
stderr**. Under Decision 11 the repointed bodies branch on the stdout keyword
instead, but the `E_*` names remain the stderr diagnostic and are still pinned:
the stderr golden set covers **every `E_*` name any current body references**
(enumerated from the `readonly E_*=` capture, asserted verbatim), including the
~nine binary-owned argument-validation names the surface never emits —
`E_CREATE_ALREADY_SYNCED`, `E_CREATE_BAD_FRONTMATTER`, `E_COMMENT_NO_BODY`,
`E_UPDATE_BAD_STATE`, `E_UPDATE_NO_OPS`, `E_ATTACH_BOTH_TARGETS`,
`E_SHOW_NOT_FOUND`, `E_NO_TOKEN`, plus the integer `107`
(`E_CREATE_WRITEBACK_FAILED`). ⚠️ Never emit `70`–**`74`** for a provider
condition (Decision 16); the search flow's `E_SEARCH_*` (bash `70-73`) is
remapped off the widened band as the allowlisted divergence.

#### 4. Migrate the Linear scenario fixtures

**Files**: `cli/linear-cli/tests/fixtures/scenarios/`,
`meta/inventories/0211-fixture-reconciliation.md` (linear rows).

The cluster's 40 `test-fixtures/scenarios/` files (including the `.json.tmpl`
templated variants) are mock-server expectation sets whose shape maps onto
`http-test-support`'s `Route`: `{method, path, response:{status, headers,
body}}` → `Route::Json` / `Route::Headers` / `Route::Redirect`, `consume` → a
`Route::Sequence` element, and `expect_body_contains` → the request assertion
the flow test already makes. Each scenario is ported where a `linear-cli` test
drives that condition and ledgered otherwise (Decision 15), with the row count
pinned against 40.

The attach scenarios are the highest-value set — `attach-binary-bad-upload-url`,
`-crlf-header`, `-redirect`, `-register-fail`, `-upload-fail` encode the
three-step upload's failure surface, which no Rust test drives today.

#### 5. Subcommands over `linear-client`, TDD per flow

**Files**: one module + one test file per flow under `cli/linear-cli/`.

Each subcommand: parse args → build the credential context → call the client
method → render JSON, then the keyword discriminant line (Decision 11), or the
`E_*` error to stderr + exit code. Per flow, a scenario-backed test asserts the
outgoing request (method, `/graphql` document, variables via the per-hit body
log) and the parsed response against a fixture, plus a byte-exact stdout golden
(`Vec<u8>`, never `from_utf8_lossy`).

Additional contracts the goldens must cover, each keyed on by a repointed body:

- **Search stderr audit line**: `search` echoes the composed `IssueFilter` to
  stderr (`INFO: composed IssueFilter: …`); a stderr golden pins it, and
  `--quiet` (reproduced as a flag) suppresses it.
- **Search JSON envelope**: the body reads `.data.issues.nodes[]`
  (`.identifier`, `.title`, `.state.name`, `.assignee.name`) and the
  merged-pages `.data.issues.truncated` flag. The linear client's additive
  read-side op (Decision 20) now selects `state { name }` and `assignee { name }`
  and preserves the title, so the binary can emit this shape; a named byte-exact
  golden pins the envelope. Any residual client-vs-bash shape gap is flagged as
  an explicit divergence with a body update, not assumed away by the mock golden.
- **Multi-POST flows**: because Linear posts everything to `/graphql`,
  `http-test-support` today records only `last: Option<Received>` per
  `(method, path)` key, so the first mutation's body is overwritten. This
  requires an **additive change to `cli/http-test-support/src/lib.rs`** to
  record a `Vec<Received>` per key (a per-hit body log alongside the existing
  `hits()` count), listed as a Phase 1 file. Any flow issuing two POSTs
  (`create`+writeback, `transition`'s `resolve_state`+`transition`) then asserts
  the hit count is exactly the expected number and each hit's body. The change
  is backward-compatible — `last_body` remains for single-POST callers — and is
  tested at its own boundary in `cli/http-test-support/tests/server.rs`: per-hit
  bodies recorded in order across ≥2 POSTs to one key, and `last_body` still
  returning the most recent after multiple hits, so a shared-infra off-by-one
  does not silently weaken every consumer's multi-POST assertions.
- **Production `from_config` path**: at least one test drives
  `from_config(context, integrations_root)` with a fixture config directory, so
  the path real users hit is not covered by manual verification alone.
- **Keyword discriminant**: every subcommand's golden includes its final
  `<keyword>\t<detail>` line, and the bare-identifier projection suppresses it.

`init verify` validates credentials without printing them (Decision 3), asserted
exhaustively: a recognisable sentinel token is seeded and every exit path
(success and each error/transport-failure variant, plus a malformed-token
diagnostic) is driven, asserting the sentinel appears on **neither stdout, stderr
nor the captured `tracing` sink**; the `Secret`-redaction invariant is recorded
as the reason the guarantee holds.

#### 6. Whole-surface help golden

**Files**: `cli/linear-cli/tests/cli_surface.rs`,
`cli/linear-cli/tests/fixtures/cli_surface.golden`.

The `=== accelerator-linear <sub> --help ===` section-header pattern from
`cli/work-cli/tests/cli_surface.rs`.

#### 7. Workspace registration (crate only, no token)

**Files**: `cli/Cargo.toml` (`members` += `"linear-cli"` and
`"cli-test-support"`, 35 → 37), `cli/Cargo.lock` (resync via `cargo metadata`,
never `generate-lockfile`), `tasks/public_api.py` (`_EXEMPT_MEMBERS` +=
`linear-cli` with reason `_COMPOSITION_ROOT`, and `cli-test-support` as a
test-support crate on the `http-test-support` model).

`cli-test-support` follows the **plain library-crate registration checklist**
(`tasks/README.md:665-722`) — it ships no `[[bin]]` and no dispatch token.
`tests/unit/tasks/test_rust.py:159-217` forces the `public_api.py` edit in the
same change as the member addition. No `cli/pup.ron` rule is added — no `*-cli`
crate has one, and the Linear rules already landed in 0210 at `:194-262`.

### Success Criteria

#### Automated Verification:

- [x] Workspace builds locked: `mise run cli:check` — green (exit 0)
- [x] Linear CLI tests pass (per-flow request/response/stdout goldens, exit-code
      parity, keyword surface, help surface): `cargo nextest run -p linear-cli
      --features test-loopback` (from `cli/`) — 26 pass with the feature, 14
      without; CI runs `--all-features`, which enables `test-loopback`
- [x] Behavioural exit-code test drives the binary into each error class and
      asserts the *observed* exit code; its class set is an exhaustive match over
      the error enums (not the keyword set, not the bash capture), so a new
      variant is a compile error until driven and binary-owned classes absent
      from bash are still driven — `flow_errors.rs` (network 401/400, not-found,
      the argument-validation refusals, missing token); the `for_surface`/
      `for_client`/`for_failure` maps are wildcard-free exhaustive matches, so a
      new variant is a compile error at the mapping
- [ ] Every error class maps to exactly one keyword in the closed, count-pinned
      per-subcommand keyword set — **design divergence**: errors route to exit
      codes + `E_*` stderr, not keywords; keywords carry success outcomes only
      (`keyword_surface.rs` pins the closed set). To record in Phase 2's
      divergences ledger.
- [x] Parity divergence allowlist is count-pinned, ledger-backed, and its oracle
      is independent of the constants it guards — `exit_codes_parity.rs`
- [x] `bash-exit-codes.txt` parse asserts each `(flow, name)` key is unique
- [x] No binary emits `70`–`74` for a provider condition (test over the whole
      subcommand set) — `exit_codes_parity.rs::no_code_lands_on_the_reserved_dispatch_band`
- [x] Search stderr audit line (`INFO: composed IssueFilter`) golden holds and
      `--quiet` suppresses it — `flow_search.rs`
- [x] Search JSON-envelope golden (`.data.issues.nodes[]` with `.state.name` +
      `.assignee.name` from the read-side op, Decision 20, + `.truncated`)
      matches the binary's emission — `stdout_goldens.rs::search_stdout_matches_the_golden`
- [x] Multi-POST flows assert the `/graphql` hit count and per-hit bodies, and
      the additive `Vec<Received>` change is tested at its own boundary in
      `cli/http-test-support/tests/server.rs` (ordered per-hit bodies; `last_body`
      unchanged after multiple hits) — the binary-attach three-step is the linear
      multi-POST case (`flow_attach.rs`); create/transition are single-POST
      because `resolve_state` is a local catalogue lookup
- [x] Every ported scenario under `tests/fixtures/scenarios/` is referenced by at
      least one test (inventory test), so "ported" means "consumed" (Decision 15)
      — `scenario_inventory.rs`
- [x] The `test-loopback` feature is off in the release build: the per-crate
      `compile_error!` guard holds and a build-system assertion proves
      `_CLI_RELEASE_BINARIES` carries no `--features test-loopback` — staged-binary
      byte scan `tasks/build.py::_assert_no_test_loopback` (covers jira/linear
      once they join the release set), tested in `test_build.py`
- [x] The `from_config` branch has an automated test — `flow_seam.rs::the_from_config_branch_resolves_config_without_an_override`
- [x] `init verify` sentinel-token test proves no token on stdout or stderr
      (success path; the every-exit-path variants remain) — `flow_init.rs`
- [x] A set-but-unparseable or non-admissible `ACCELERATOR_LINEAR_API_URL`
      hard-errors via the binary's own usage exit path before credentials
      attach; a loopback/plain-http override is rejected in any build without
      the `test-loopback` feature — `flow_seam.rs` (non-admissible host and
      unparseable → 2; the `not(feature)` loopback-refusal case; the underlying
      `url_is_allowed(_, false)` rejection is pinned in `linear-client`'s
      `upload.rs` tests)
- [x] Every stderr `E_*` name any current linear body references is pinned by a
      golden — `stderr_diagnostics.rs` (the binary-owned argument-validation and
      seam tokens, asserted verbatim)
- [x] The fixture ledger accounts for all 40 linear scenario files —
      `meta/inventories/0211-fixture-reconciliation.md`
- [x] Full read-only gate: `mise run check` — green (exit 0)

#### Manual Verification:

- [ ] `accelerator-linear` run against a live Linear team returns the same issue
      shapes as `linear-*-flow.sh` for create/show/search/comment/transition/
      attach/init (spot-check against 0210's 2026-08-21 contract evidence)

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
exact-tuple pin `:530-541`), `tasks/manifest.py` (`_SUBBINARY_MANIFESTS` +=
`"linear": CLI_DIR / "linear-cli/Cargo.toml"`), `tasks/build.py`
(`_CLI_RELEASE_BINARIES` += `"accelerator-linear"`), `.gitignore` (`bin/
linear-*`, after `bin/design-*` at `:53`).

⚠️ If `tasks/README.md:499-663` is touched,
`tests/unit/tasks/test_registration_docs.py` asserts thirteen points and that
every named identifier resolves.

#### 2. Repoint the eight linear `SKILL.md` bodies

**Files**: `skills/integrations/linear/{init-linear,show-linear-issue,
search-linear-issues,create-linear-issue,update-linear-issue,
comment-linear-issue,transition-linear-issue,attach-linear-issue}/SKILL.md`.

Rewrite each fenced execution step from a `linear-*-flow.sh` invocation to
`accelerator linear …`, and rewrite every in-body outcome branch onto the
**keyword discriminant** (Decision 11) — including the one integer the linear
bodies cite (`107` at `create-linear-issue/SKILL.md:92`) and the `E_*` names the
bodies branch on. The doc-vs-binary parity guard asserts every keyword a body
reads exists in the binary's declared set, with an anti-vacuity match count.

Read/init skills (`init-linear`, `show-linear-issue`, `search-linear-issues`)
additionally drop `Bash(jq)`, `Bash(curl)` and the
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)` glob from
frontmatter, keeping `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` and
adding `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear *)` — the shape 0212
landed at `sync-work-items/SKILL.md:8-10`.

**`search-linear-issues` is the token witness**, and its witnessing invocation
must be **metacharacter-free**: `dispatch_coherence.py:_bindings` counts a
binding only when `not has_metacharacter(command)` (`:103`), so a piped
`accelerator linear search … | …` records the token as invoked-but-unbound and
fails coherence. It invokes `accelerator linear search` in a fenced step with no
pipe/redirect/`&&` — the binary renders final JSON itself, so no downstream `jq`
pipe is needed.

Write skills keep bare `Bash` (Decision 7). The confirm gate previews resolved
intent (Decision 2) and its lexical precedence over the mutation invocation is
enforced by the new `test-skill-write-gate.sh` skills-lane guard.
`create-linear-issue`'s empty-stdout dependence is redesigned to gate on the
create keyword **and to fail closed** — a non-success keyword suppresses the
writeback and blocks retry (Decision 5), asserted by test. Any
`--print-payload`/`--describe` advertised in the `argument-hint` or a body
preview step of any write skill is dropped in this commit.

#### 3. Delete the linear cluster and retire its guards

**Files deleted**: `skills/integrations/linear/scripts/*.sh` (10 executables + 2
libraries), `EXIT_CODES.md`, the 12 `test-linear-*.sh` suites,
`test-helpers/mock-linear-server.py`, `test-fixtures/` (40 scenarios). Total 66
files / 6,701 lines.

**Guards edited**:

- `tasks/lint/scripts.py` — drop the two linear `SHELL_LIBRARIES` members
  (`linear-common.sh`, `linear-auth.sh`), 21 → 19.
- `tests/unit/tasks/test_exec_bits.py` — drop the same two from
  `_RECONCILED_LIBRARIES` (`:260-266`); `test_exact_membership` stays green
  because both literals change together.
- `tasks/test/integration.py` — decrement `_EXPECTED_INTEGRATIONS_SUITES`
  32 → 20 (`:51`). The `integrations` task survives (jira suites remain).
- `tests/unit/tasks/test_python_coverage.py` — remove `MOCK_LINEAR` (`:34-35`)
  and its usages; adjust `RUFF_JUSTIFIED_EXCLUDES` (set-equality).
- `pyproject.toml` — strip the `mock-linear-server.py` `extend-exclude` line
  (`:80`), in the same change as the `RUFF_JUSTIFIED_EXCLUDES` edit.

#### 4. Record linear artefacts

**Files**: `meta/inventories/0211-removal-set.md`,
`meta/inventories/0211-suite-audit.md`,
`meta/inventories/0211-reconciliation.md`,
`meta/inventories/0211-fixture-reconciliation.md` (linear rows),
`meta/inventories/0211-divergences.md` (linear rows),
`meta/work/0171-jira-and-linear-integrations.md` (`## Decisions`).

Record the **Linear generator provenance here**, at this track's merge boundary
(not deferred): the last-existing revision of `mock-linear-server.py` and the
linear bash cluster, the scripted capture command and its full output for
`bash-exit-codes.txt`, and each linear golden's provenance. Because the tracks
are independently mergeable, a Linear-only merge must carry its own revival
anchor.

### Success Criteria

#### Automated Verification:

- [ ] Dispatch coherence green both directions, with `search-linear-issues` as a
      metacharacter-free witness (bound, not merely invoked): `mise run
      build-system:check`
- [ ] `test-skill-write-gate.sh` asserts each linear write skill's confirm step
      lexically precedes its `accelerator linear …` mutation invocation
- [ ] Doc-vs-binary parity: every keyword a repointed linear body branches on
      exists in the binary's declared set; no body cites a bash exit integer
- [ ] `ls skills/integrations/linear/scripts/*.sh` matches nothing;
      `mock-linear-server.py` does not exist
- [ ] Linear integration floor holds at 20 and the `integrations` task runs the
      surviving jira suites: `mise run test:integration:integrations`
- [ ] Exec-bit + stale-library guards green: `mise run lint:scripts:check`
- [ ] Python coverage + ruff-exclude equality green: `mise run test:unit`
- [ ] Full run green end to end: `mise run`

#### Manual Verification:

- [ ] Every linear `SKILL.md` body invokes `accelerator linear …`; no linear
      skill declares `jq`, `curl` or a `scripts/` grant
- [ ] The confirm gate still previews the resolved intent before a write
- [ ] Reconciliation table reconciles to 10 executables + 2 libraries, and
      records the 9 (not 10) `SKILL.md`-reachable entrypoints

---

## Phase 3: Jira binary (`cli/jira-cli`)

### Overview

Mirror Phase 1 for Jira, plus ADF handling, the `resolve-fields` subcommand
(Decision 4), the `create --emit key` projection (Decision 5), the
tab-separated resolver golden, the ~45-integer exit-code parity, the search
`70-73` remap off the widened band, and the Jira-only TTY policy. Not
registered; the jira bash cluster stays as the capture source.

### Subcommand surface and the reconciliation mapping

Seventeen executables + five libraries + three data assets map as:

| Bash executable | Disposition |
|---|---|
| `jira-create-flow.sh` | `jira create` |
| `jira-update-flow.sh` | `jira update` |
| `jira-show-flow.sh` | `jira show` (`--render-adf` flag) |
| `jira-search-flow.sh` | `jira search` (`jql::compose` + the read-side projection op with cursor + `--render-adf`/`--fields`, Decision 20; codes remapped off `70-74`) |
| `jira-comment-flow.sh` | `jira comment add \| list \| edit \| delete` |
| `jira-transition-flow.sh` | `jira transition` |
| `jira-attach-flow.sh` | `jira attach` (multipart) |
| `jira-init-flow.sh` | `jira init verify \| discover \| prompt-default \| refresh-fields \| list-projects \| list-fields` |
| `jira-fields.sh` | `jira fields refresh \| resolve \| list` |
| `jira-resolve-fields.sh` | `jira resolve-fields` (Decision 4) |
| `jira-emit-key.sh` | `jira create --emit key` projection (Decision 5) |
| `jira-render-adf-fields.sh` | `--render-adf` flag on `show`/`search` |
| `jira-adf-to-md.sh` | internal — `document_to_markdown`; its test-oracle role retired in Phase 0 |
| `jira-md-to-adf.sh` | internal — `markdown_to_document`; ditto |
| `jira-request.sh` | internal transport — `transport.rs`, no subcommand |
| `jira-auth-cli.sh` | dropped — subsumed by `init verify` (Decision 3) |
| `jira-jql-cli.sh` | dropped — orphan (Decision 5) |
| `jira-common/-auth/-jql/-body-input/-custom-fields.sh` (5 libs) | subsumed by the crate |
| `jira-adf-render.jq`, `jira-md-tokenise.awk`, `jira-md-assemble.jq` | no subcommand — bash-pipeline artefact; test-oracle role retired in Phase 0 |

⚠️ The three data assets' classification is right about the *product* surface
and was wrong about the *deletion* surface — Phase 0 is what makes it true.

⚠️ Jira's dispatch-mode count is **21**, not "roughly 25": 14 named verbs across
four scripts, plus four validated HTTP-method tokens in
`jira-request.sh:300-309`, plus two `resolve-fields` modes, plus one bare init.

**TTY policy (Jira only).** `jira-init-flow.sh:191` blocks on `read -r` when
`work.default_project_code` is unset and `--non-interactive` was not passed.
`jira init` reproduces this as an explicit refusal with a usage exit code when
stdin is not a TTY, never a silent block; a test drives it with stdin closed.

### Changes Required

#### 1. Crate scaffold, seam, taxonomy, keyword set, subcommands

**Files**: `cli/jira-cli/` mirroring Phase 1 (same `work-cli` inline-`ExitCode`
shape, same scripted/differential capture, same parity-allowlist and
keyword-surface enforcement), plus `cli/jira-client/` returning a structured
discriminant on the port-op error path across all six fallible port methods — a
`classify::Outcome`, or an explicit reason for inline-`TrackerError` branches
such as post-create unwritable-identifier, `fetch_all` unsafe-id, the search
compose error and `surface_read_failure` (Decision 9) — **and adding the
read-side search projection op** (widened `fields` set, `nextPageToken` cursor
surfaced to the caller, Decision 20), with `description = "The jira create|update|
show|search|comment|transition|attach|init|fields|resolve-fields sub-binary."`
and dev-dep `cli-test-support`.

The `ACCELERATOR_JIRA_API_URL` seam overrides `Credentials.base` (Jira has no
explicit-endpoint constructor). The override is admitted only through Jira's
already-`pub` `auth.rs::base_url` (Decision 10) — the **fully-validated**
destination check (https, no userinfo, no query/fragment, default port,
`*.atlassian.net`), not the host-only `host_is_admissible` fragment — before the
email+token attach; a set-but-unparseable or non-admissible value is a hard
usage error returned inline as `ExitCode::from(exit_codes::USAGE)`. Reusing
`base_url` is what prevents a cleartext `http://foo.atlassian.net` slipping
through. `base_url` stays **strict and unchanged**. The override is validated
against the **same allowlist `resolve_credentials` builds** — never an ad-hoc or
broader list. `allowed_sites` is **private today** (`cli/jira-client/src/auth.rs:158`);
this promotes it `pub` (or adds a `pub` override-validator composing
`allowed_sites` + `base_url`) so the seam reaches the real list rather than
reconstructing it. That matters because `allowed_sites` performs two
token-exfiltration **provenance refusals** a host-membership check would miss:
`AllowlistFromSharedConfig` for a Team-level `jira.allowed_sites`
(`auth.rs:162-164`) and `refuse_tracked_source` for a tracked (committed) value
(`auth.rs:169-173`). Tests assert the override path rejects **all three** — a
Team-level allowlist entry, a tracked-source entry, and a host absent from the
personal allowlist — not only the host-membership case. For the binary's mock-backed
tests, a dedicated `test-loopback`-gated branch constructs `Credentials` pointed
at the override directly and calls `JiraClient::new` (as
`cli/jira-client/tests/support/client.rs` does), bypassing `base_url`; it is
compiled out of every ordinary build.

Capture the Jira oracle with the same scripted differential harness as Phase 1
(declared `readonly E_*=NN` grep across all jira flows + behavioural exit codes
from executing each flow against its error fixtures, behavioural-wins
precedence, per-`(flow, name)` uniqueness) into
`cli/jira-cli/tests/fixtures/bash-exit-codes.txt`. `exit_codes.rs` maps
`SurfaceError`/`ClientError`/`AdfError` variants and the port-op code → the ~45
integers, with its module doc as the document of record; `exit_codes_parity.rs`
pins them with the count-pinned divergence allowlist against an oracle
independent of the constants; a behavioural exit-code test asserts the observed
code per class; and `keyword_surface.rs` pins the closed keyword set. **Capture
behaviour, not the doc**: `jira-request.sh:207` exits `1` where
`EXIT_CODES.md:12` claims `2` — a named declared-vs-behavioural divergence, not
silently collapsed.

⚠️ Variant counts to cover: Jira `SurfaceError` 11, `ClientError` 13, `AdfError`
7 (codes 40/41/42). `SurfaceError::status` is `pub(crate)`, so the mapping
matches on variants and never constructs one.

#### 2. Migrate the Jira scenario fixtures

**Files**: `cli/jira-cli/tests/fixtures/scenarios/`,
`meta/inventories/0211-fixture-reconciliation.md` (jira rows).

The cluster's 95 `test-fixtures/scenarios/` files port onto `http-test-support`
routes on the Phase 1 model — `{method, path, capture_body, capture_url,
response:{status, body}}` maps directly, and the `retry-after-*` scenarios need
`Route::Headers`. Ported where a `jira-cli` test drives the condition, ledgered
otherwise, count pinned against 95.

The ten `api-responses/` files are ledgered as **already dead** — zero consumers
today, before this change — and deleted without porting.

The `*-print-payload-guard` scenarios (e.g.
`comment-edit-print-payload-guard.json`, `attach-describe-guard.json`) are
ledgered as **superseded by Decision 2**: the wire-payload preview is not
reproduced, and the resolved-intent gate is covered by the stdout-before-
mutation assertion and `test-skill-write-gate.sh` instead.

#### 3. Strict stdout goldens

**Files**: `cli/jira-cli/tests/fixtures/*.golden`.

Byte-exact goldens for the strict contracts: `jira resolve-fields` emits the
tab-separated four-field line (`<type>\t<type_source>\t<project>\t
<project_source>\n`, trailing newline load-bearing); `jira create --emit key`
emits the bare validated key (`^[A-Z][A-Z0-9]+-[0-9]+$`) with the post-create
non-retryable "created remotely but unwritable" semantic (its code taken from the
captured fixture, Decision 5) and **suppresses the keyword discriminant**
(Decision 11); `jira show` renders the ADF description via `document_to_markdown`.

A further test asserts the Decision 17 relationship: `jira resolve-fields` and
`accelerator work create --push --dry-run` agree field-for-field on the same
fixture config, the work form differing only by its leading tracker token.

#### 4. The search remap and the cross-provider collisions

**Files**: `cli/jira-cli/src/exit_codes.rs`,
`cli/jira-cli/tests/exit_codes_parity.rs`.

`jira search`'s `E_SEARCH_BAD_PAGE_TOKEN/BAD_LIMIT/NO_SITE_CACHE/BAD_FLAG` (bash
`70-73`) are remapped off the reserved band — now **`70`–`74`** (Decision 16) —
as a recorded divergence with a test. The `81`/`82`/`34` cross-provider
collisions each resolve to a stated per-provider Jira behaviour with a test. The
credential-resolution divergence (Jira flattens to `22`) is already encoded in
`jira-client`'s `error.rs`/`classify.rs`.

#### 5. Workspace registration (crate only, no token)

**Files**: `cli/Cargo.toml` (`members` += `"jira-cli"`), `cli/Cargo.lock`,
`tasks/public_api.py` (`_EXEMPT_MEMBERS` += `jira-cli`, `_COMPOSITION_ROOT`).

### Success Criteria

#### Automated Verification:

- [ ] Workspace builds locked: `mise run cli:check`
- [ ] Jira CLI tests pass (request/response/stdout goldens incl. tab-separated
      resolver + bare key + ADF render, exit-code parity, keyword surface, help
      surface): `cargo nextest run -p accelerator-jira` (from `cli/`)
- [ ] Behavioural exit-code test drives the binary into each error class and
      asserts the observed code; parity divergence allowlist is count-pinned and
      ledger-backed; `(flow, name)` keys are unique in `bash-exit-codes.txt`
- [ ] Every error class maps to exactly one keyword in the closed, count-pinned
      per-subcommand keyword set
- [ ] No binary emits `70`–`74` for a provider condition; search codes proven
      off the widened band (test)
- [ ] `81`/`82`/`34` each resolve to a stated per-provider behaviour (tests)
- [ ] `jira search` composed-JQL stderr audit line (`INFO: composed JQL`) golden
      holds, `--quiet` suppresses it, and the read-side envelope golden
      (Key/Summary/Status/Assignee + `nextPageToken` cursor, Decision 20) matches
      the binary's emission; `--page-token` fetches the next page, and
      `--render-adf`/`--fields` survive against the read-side op
- [ ] `jira resolve-fields` and `work create --push --dry-run` agree
      field-for-field on the same fixture config (Decision 17)
- [ ] `jira init`'s bare mode refuses explicitly with a usage code when stdin is
      not a TTY, rather than blocking
- [ ] Port-op codes are read from the structured discriminant the client's
      port-op path returns across all six fallible port methods, never parsed from
      `detail`; the absence of a `detail` parse is enforced by a lint/grep guard
      over `exit_codes.rs`; no new `&TrackerError` accessor
- [ ] A set-but-unparseable or non-admissible `ACCELERATOR_JIRA_API_URL`
      hard-errors before the token attaches; the release seam rejects a
      loopback/plain-http override through the unchanged strict `base_url` in
      every build; the mock-backed tests reach loopback only via the
      `test-loopback`-gated direct-`Credentials` branch; the `from_config`
      branch has an automated test
- [ ] The fixture ledger accounts for all 95 jira scenario files and the 10
      dead `api-responses/` files
- [ ] Full read-only gate: `mise run check`

#### Manual Verification:

- [ ] `accelerator-jira` against a live Jira project matches `jira-*-flow.sh`
      for every flow (spot-check against 0210's 2026-08-21 contract evidence)
- [ ] `jira resolve-fields` output is byte-identical to the bash resolver line
- [ ] `init verify` prints no credential

---

## Phase 4: Jira cutover + retirement

### Overview

Register the `jira` token and repoint the eight jira `SKILL.md` bodies in one
commit, drop `jq`/`curl` from the three read/init frontmatters, delete the jira
cluster, and remove the integrations floor and its four dependents outright.

Phase 0 must have landed: this phase deletes the two ADF drivers and the three
`.jq`/`.awk` assets that `cli/jira-client`'s differential drove.

### Changes Required

#### 1. Token registration (same commit as repointing)

**Files**: as Phase 2 item 1, for `jira`: `DISPATCHED_SUBBINARIES`,
`_SUBBINARY_DESCRIPTIONS` + tuple pin, `_SUBBINARY_MANIFESTS`
(`"jira": CLI_DIR / "jira-cli/Cargo.toml"`), `_CLI_RELEASE_BINARIES`
(`"accelerator-jira"`),
`.gitignore` (`bin/jira-*`).

Also `cli/deny.toml:73-78`, whose prose says "all six dispatched sub-binaries"
and omits `accelerator-design` from its `uluru` symbol-count table — stale at
seven today and staler at nine. The licence closure itself is unchanged (the
reqwest/rustls tree rides the existing permissive allow), so checklist point 13
remains a no-op beyond the prose.

#### 2. Repoint the eight jira `SKILL.md` bodies

**Files**: `skills/integrations/jira/{init-jira,show-jira-issue,
search-jira-issues,create-jira-issue,update-jira-issue,comment-jira-issue,
transition-jira-issue,attach-jira-issue}/SKILL.md`.

As Phase 2 item 2, for `jira` — including the same-commit rewrite of every
in-body outcome branch onto the keyword discriminant (Decision 11). **Seven Jira
bodies cite exit integers, not two**: `search-jira-issues` Step 3 (`Exit
72`/`Exit 71`), `create-jira-issue` Step 10 + WF-1 (100-107, 108/109,
11/12/22/19/20/21/34), `show-jira-issue` (80/81/82), `transition-jira-issue`
(122/123/124), `attach-jira-issue` (132/133), and the `update-`/
`comment-jira-issue` Step-9 tables. All nine tables collapse to keyword tables.

The doc-vs-binary parity guard **enumerates every repointed body** (all sixteen
jira+linear), extracts the outcome tokens each branches on, asserts each exists
in the binary's declared keyword set (body→binary), asserts a non-zero
matched-reference count (anti-vacuity) so an under-matching extractor cannot pass
silently, and fails on any residual bash exit integer or
`--print-payload`/`--describe` reference. It also asserts the **reverse for write
flows** (binary→body): every keyword a write binary can emit is either branched
on by the body or provably falls through to the no-writeback default, so a new or
renamed binary keyword cannot silently reach a mutation the body does not gate.
Read-flow bodies need only body→binary (an unhandled keyword there is at worst a
display gap). A
committed stale-keyword fixture proves it fails. It lives as a build-system
guard beside `dispatch_coherence`.

The three read/init skills drop `jq`/`curl` + the script glob and gain the
`jira` token grant; **`search-jira-issues` is the witness**, with a
metacharacter-free invocation (no `jq` pipe). The write skills keep bare `Bash`
— `attach-jira-issue:70`'s `wc -c` and `create-jira-issue:113`'s `source
config-common.sh` writeback survive. `create-jira-issue` calls `jira
resolve-fields` then `jira create`; its empty-stdout gates (`:183`) and the
resolver-line parse (`:65-66`) are repointed onto the new subcommands and **fail
closed** — a non-success create keyword suppresses the frontmatter writeback and
blocks retry, and the post-create-unwritable keyword surfaces an explicit "issue
created remotely as <key>; reconcile manually" message so the orphaned-remote
state is visible (Decision 5), asserted by test. The **create-then-writeback
sequence is non-atomic** (remote `jira create`, then the surviving bash `source
config-common.sh` writeback), so the success-but-local-writeback-failed arm is
handled too: it surfaces the created key and steers the operator to reconcile
rather than inviting a blind re-create, with a test asserting a writeback failure
after a successful create does not silently admit a duplicate. Its "single source of truth"
prose (`:60-68`) is rewritten per Decision 17.

#### 3. Delete the jira cluster and retire its guards

**Files deleted**: `skills/integrations/jira/scripts/*.sh` (17 executables + 5
libraries), the three data assets, `EXIT_CODES.md`, the 21 `test-jira-*.sh`
suites, `test-helpers/mock-jira-server.py`, `test-fixtures/` (148 files incl.
the already-dead `api-responses/`). Total 197 files / 14,721 lines.

**Guards edited**:

- `tasks/lint/scripts.py` — drop the five jira `SHELL_LIBRARIES` members
  (`:34-38`), 19 → 14.
- `tests/unit/tasks/test_exec_bits.py` — drop the five from
  `_RECONCILED_LIBRARIES`; remove `_DUAL_USE_SCRIPTS` (`:274` + its comment) and
  `test_dual_use_scripts_are_entrypoints` (`:289-298`) — the only pinned
  dual-use exemplar (`jira-fields.sh`, sourced by `jira-init-flow.sh:32` and
  path-invoked from four flows) is gone with no substitute. The divergence row
  must name what still detects a future dual-use script: the exec-bit invariant
  guard (`lint:scripts:exec-bits:check`) still classifies any new `.sh` as
  entrypoint-or-library and fails an unclassified one, so a future dual-use
  script is rejected rather than silently misclassified. Record this as
  exemplar-coverage loss with detection retained, not an unqualified deletion.
- `tasks/README.md:78-111` — retire the dual-use prose that documents the
  classification entirely through `jira-fields.sh`.
- `tasks/test/integration.py` — remove `_EXPECTED_INTEGRATIONS_SUITES`
  (`:49-51`) and the `integrations` task (`:400-410`) outright.
- `tasks/test/helpers.py` — drop `"test-jira-scripts.sh"` from
  `EXCLUDED_HELPER_NAMES` (`:10`).
- `tests/unit/tasks/test_integration.py` — remove the `integrations` `_GUARDED`
  entry (`:68`) (else `AttributeError`).
- `mise.toml` — remove the `test:integration:integrations` leaf (`:369-372`) and
  its roll-up entry (`:388`).
- `tests/unit/tasks/test_mise.py` — remove the `_LAUNCHER_DEPENDENTS` entry
  (`:55`) (partition equality).
- `tests/unit/tasks/test_python_coverage.py` — remove `MOCK_JIRA` (`:33`) and
  its usages; `RUFF_JUSTIFIED_EXCLUDES` → `{"workspaces"}`.
- `pyproject.toml` — strip the `mock-jira-server.py` `extend-exclude` line
  (`:79`), leaving `extend-exclude = ["workspaces"]`; clean the stale
  mock-server comments (`:72-76`).

#### 4. Record the jira generator provenance at this track's boundary

**Files**: `meta/inventories/0211-{removal-set,suite-audit,divergences,
fixture-reconciliation}.md` (jira rows), `meta/work/0171-…md` (`## Decisions`).

Record the **jira generator provenance here**, at Phase 4's deletion boundary
(not deferred to Phase 5), mirroring Phase 2's treatment of Linear: the
last-existing revision of `mock-jira-server.py`, the jira bash cluster and the
two ADF drivers; the scripted capture command and its full output for
`bash-exit-codes.txt` and the Phase 0 oracle corpus; and each jira golden's
provenance (mock-served, live-anchored against 0210's 2026-08-21 contract
evidence, Decision 8). Because the tracks are independently mergeable, a
Jira-only merge must carry its own revival anchor — otherwise a digest mismatch
in the window before Phase 5 has only an unindexed git-history bisect to recover
from.

### Success Criteria

#### Automated Verification:

- [ ] `ls skills/integrations/jira/scripts/*.sh` matches nothing;
      `mock-jira-server.py` does not exist
- [ ] `cargo nextest run -p jira-client` green with the cluster gone — the Phase
      0 conversion holds under real deletion, not a simulated one
- [ ] Dispatch coherence green both directions, `search-jira-issues` a
      metacharacter-free witness: `mise run build-system:check`
- [ ] `test-skill-write-gate.sh` green for every jira write skill
- [ ] Doc-vs-binary parity over all sixteen bodies: every keyword exists in a
      declared set, the matched count is non-zero, no residual bash integer and
      no `--print-payload`/`--describe` reference
- [ ] `_EXPECTED_INTEGRATIONS_SUITES`, the `integrations` task, its `mise` leaf,
      its `_GUARDED` entry and its `test_mise` member are gone;
      `_DUAL_USE_SCRIPTS` and its test are retired
- [ ] Stale-library, exec-bit, python-coverage, ruff-equality, mise-partition
      guards all green: `mise run check`
- [ ] Full run green end to end: `mise run`

#### Manual Verification:

- [ ] Every jira `SKILL.md` body invokes `accelerator jira …`; the write flows
      still gate before a mutation
- [ ] The reconciliation table reconciles to 17 executables + 5 libraries + 3
      data assets, every "internal helper" naming its subsuming subcommand

---

## Phase 5: Residue retirement, whole-repository assertions, final artefacts

### Overview

Retire the bash that only dies once both clusters are gone, land the
whole-repository assertions, and finalise the artefact set. This is the child's
merge boundary — `mise run` exits 0 end to end here.

### Changes Required

#### 1. Retire `scripts/work-common.sh` (Decision 13)

**Files deleted**: `scripts/work-common.sh`.

**Guards edited**: `tasks/lint/scripts.py:23` and
`tests/unit/tasks/test_exec_bits.py:249` — drop the member from both, 14 → 13.

Its four consumers (`jira-common.sh:61`, `linear-common.sh:60`,
`jira-create-flow.sh:214`, `jira-search-flow.sh:253`) are all gone by Phase 4,
so this is the first moment the deletion is safe and the last moment before
0174 would inherit a silently-dead entry. Record in the divergences ledger that
`work_resolve_default_project`'s behaviour now lives in the `config` crate path
`jira resolve-fields` reads, naming the test that covers it.

#### 2. Retire the `test:integration:work` husk (Decision 14)

**Files**: `tasks/test/integration.py` (remove the `work` task, `:392-394`),
`mise.toml` (remove the leaf `:364-367` and the roll-up member `:387`),
`tests/unit/tasks/test_mise.py` (remove the `_LAUNCHER_DEPENDENTS` member
`:54`).

It carries no floor and no `_GUARDED` entry, so the partition assertion is the
only mirror. `skills/work/` holds no `test-*.sh`, so nothing stops running that
was running.

#### 3. Retire `cli/tracker-support`'s dead bash helpers (Decision 18)

**File**: `cli/tracker-support/tests/support/mod.rs` — delete `run_bash`
(`:136`) and `repo_root` (`:46`, its only caller-target), drop the now-unused
`std::process::Command` / `Path` / `PathBuf` imports, and rewrite the module doc
(`:1-5`), which still names the deleted `mapper_differential.rs`.
`mapper_differential_self_test.rs` consumes `classify`, `disagreement`, `table`,
`Class`, `RETRYABLE_STATUS` and `TERMINAL_STATUS` and is unaffected.

#### 4. Whole-repository assertions

- **The `jq`/`curl` survivor set is empty.** `grep -rn "Bash(jq\|Bash(curl"
  skills/` returns nothing (12 hits across 6 files today, every one a bare
  parenthesised form in a jira/linear read/init skill).
- **The shared-asset sweep, with its declared exclusion list** (Decision 19):
  the grep over the four cluster `test-helpers`/`test-fixtures` paths plus
  `mock-jira-server`/`mock-linear-server`, excluding `meta/`, `CHANGELOG.md`
  (immutable release record),
  `skills/work/create-work-item/evals/benchmark.json` (frozen eval transcript)
  and `docs-site/src/content/docs/reference/skills/` (gitignored generated
  mirror), returns an empty residual set. The command, its
  exclusions and its output are the recorded result.
- **No Python remains in the `cli/` test lane** — the mock servers are gone and
  neither client crate's dev-deps nor `tasks/` reference them.

#### 5. Final artefacts and the 0171 reconciliation

**Files**: `meta/inventories/0211-{removal-set,suite-audit,reconciliation,
fixture-reconciliation,divergences}.md`, `meta/work/0171-…md` (`## Decisions`),
`meta/work/0211-…md` (criteria corrections).

**Both generator provenances already landed at their own track boundaries** —
linear in Phase 2, jira in Phase 4 — so each independently-mergeable track
carries its own revival anchor. Phase 5 only **consolidates** them into the final
artefact set and confirms completeness; it does not defer either recording. After
Phase 4 no generator exists except via VCS history, so the per-track anchors are
the sole revival path.

Fold the consumer sweep into the removal set rather than reference a separate
file — `0167-suite-audit.md:31` points at a `0167-removal-set-references.md`
that does not exist, and repeating that shape would repeat the dangling
reference.

Mirror all twenty-one decisions into 0171's `## Decisions`, closing the six
`pending` and three `open` entries this child owns: `linear-graphql.sh`'s
production-script classification (9), the reverse cross-cluster sweep (11, now
obsolete by construction — its source directory is gone, which is itself the
record), the flow-coverage mapping (12), fixture provenance (16) and the
`jq`/`curl` audit (17).

⚠️ **Correct 0171's three stale statements** attributing the whole-repository
`jq`/`curl` equality to 0212 — the dangerous one is in `## Scope` (`:135-136`),
because that section declares the children normative; the others are drafting
notes (`:687-688`, `:773-776`). The 0211 review's `## Correction` settled this:
0211 owns the equality, 0212 asserted only the work-skill half.

**Correct 0211's own criteria** to what this plan implements and the research
measured: the dispatch reservation and AC 4 from `70`–`73` to `70`–`74`; AC 9
from "carry pup rules and public-API snapshots" to "are classified in
`tasks/public_api.py`"; AC 6 to name its exclusion list; the verb counts from
~25/~15 to 21/6 dispatch modes; the deletion total from ~17,650 lines to 21,422
lines across 263 files; the `init` TTY policy to Jira-only; the "client surface
is complete (discovery/search)" assumption to record that **search is a
stamps-only projection needing an additive read-side client op** (Decision 20);
the init-cache handling to the read-compatible-plus-marker contract (Decision 21);
and the Assumptions' `.jq`/`.awk` clause to record that it was true of the product
surface and false of the test surface until Phase 0.

Two stale strings found in passing and swept here: both
`cli/{jira,linear}-client/tests/evidence/README.md:8` still say *"The file is
not committed yet"* beside committed data, and
`skills/work/create-work-item/SKILL.md:27` still names the deleted
`config-read-work.sh` (a 0212 leftover).

### Success Criteria

#### Automated Verification:

- [ ] `scripts/work-common.sh` does not exist and no guard list names it;
      `SHELL_LIBRARIES` is 13 members and `_RECONCILED_LIBRARIES` matches
- [ ] Neither `test:integration:integrations` nor `test:integration:work`
      exists in `tasks/`, `mise.toml` or the `test_mise.py` partition; the
      partition assertion is exact and green
- [ ] No `Command::new("bash")` remains under `cli/tracker-support/tests/` and
      `mapper_differential_self_test.rs` is green and unedited
- [ ] `grep -rn "Bash(jq\|Bash(curl" skills/` returns nothing
- [ ] The recorded shared-asset sweep's residual set is empty under its declared
      exclusions
- [ ] No Python remains in the `cli/` test lane
- [ ] Stale-library, exec-bit, python-coverage, ruff-equality, mise-partition
      guards all green: `mise run check`
- [ ] **`mise run` exits 0 end to end** — the child merge boundary

#### Manual Verification:

- [ ] The reconciliation table reconciles to 17 executables + 5 libraries + 3
      data assets (jira) and 10 + 2 (linear), every "internal helper" naming its
      subsuming subcommand
- [ ] The fixture ledger accounts for all 188 fixture files (95 + 40 scenarios,
      43 ADF samples, 10 dead `api-responses/`), each row naming its disposition
- [ ] The divergences ledger names a real, passing test per row: search remap
      off `70`–`74`; preview-intent (`test-skill-write-gate.sh` + the
      stdout-before-mutation assertion); dropped auth cleartext; dual-use
      exemplar-coverage loss with detection retained; declared-vs-behavioural
      exit-code disagreements; Jira usage-code behaviour; ADF record-stream
      removal and the frozen-oracle conversion; the two-producer tab contract;
      `work_resolve_default_project`'s relocation; any search-envelope
      client-vs-bash shape gap
- [ ] 0171's `## Decisions` carries all twenty-one decisions and its three stale
      `jq`/`curl` attributions are corrected
- [ ] 0211's criteria are corrected on all seven points above

---

## Testing Strategy

### Unit / crate tests

- Per-subcommand request assertion (method, path/GraphQL document, body) against
  a `http-test-support` mock built from the migrated scenario fixtures, and the
  parsed response against a fixture. Flows issuing two POSTs assert the
  `/graphql` hit count and per-hit bodies (not just `last_body`).
- Byte-exact stdout goldens (`Vec<u8>`) for every subcommand; the strict
  contracts (tab-separated resolver, bare key/identifier, six `.data.issue.*`
  paths) preserved exactly, and the keyword discriminant line pinned per
  subcommand. Stderr goldens for the composed-query `INFO:` audit line, its
  `--quiet` suppression, and every `E_*` name a current body references.
- `exit_codes_parity.rs` per binary: equality against the captured
  `bash-exit-codes.txt` for non-allowlisted names, a count-pinned divergence
  allowlist asserting remapped values, per-`(flow, name)` uniqueness,
  fixed-count anti-vacuity, plus the "never `70`–`74`" and collision
  (`81`/`82`/`34`) assertions. The oracle is independent of the constants it
  guards.
- `keyword_surface.rs` per binary: the closed keyword set, count-pinned, with
  every error class mapping to exactly one keyword.
- A **behavioural exit-code test** per binary: drives the binary into each error
  class and asserts the *observed* exit code, so a variant mis-routed to the
  wrong constant fails a test rather than only a const-declaration mismatch.
- An `init verify` **no-token test**: sentinel token, every exit path (plus a
  malformed-token diagnostic), asserted absent from stdout, stderr **and the
  captured `tracing` sink**.
- A `from_config`-branch test per binary, and a seam-revalidation test
  (unparseable → hard error; non-https/non-allowlisted → rejected before
  credentials attach; loopback rejected without `test-loopback`).
- `cli_surface.rs` help golden per binary.
- The reusable machinery these tests share — the `exit_codes.rs` textual parser,
  the scripted-capture skeleton, and the scenario-JSON→`Route` loader — lives in
  the shared `cli-test-support` crate, consumed by both `*-cli` crates rather than
  copied three ways. **The crate carries its own boundary tests** (mirroring
  `http-test-support/tests/`): the parser round-tripped over representative
  `pub const` forms plus a deliberately-malformed line it must reject, and the
  loader over a scenario JSON exercising each `Route` variant plus the
  sequence/body-capture cases — so a parser or loader bug cannot silently weaken
  every downstream parity/keyword/request assertion in both binaries at once. Its
  scenario schema is a **single unified superset** (optional per-provider fields:
  jira `capture_url`/`Route::Headers`, linear `consume`/`Route::Sequence`), a
  small shared core rather than dialect-branched.
- A per-binary **scenario inventory test** asserting every file under
  `tests/fixtures/scenarios/` is referenced by ≥1 test (Decision 15).
- The read-side search projection op (Decision 20): a request/response test per
  provider asserting the widened field selection (jira `fields`; linear `state`/
  `assignee`) and the cursor round-trip, with a byte-exact envelope golden.
- Phase 0: the frozen-oracle differential, its digest manifest, its case-count
  pin, the fail-loud-on-empty reader guard, and the unedited self-test proving
  the comparison can reject.

### Integration / build-system tests

- `lint:dispatch-coherence:check` both directions, per cutover phase, with the
  named witness skill's invocation metacharacter-free (bound, not merely
  invoked).
- `test-skill-write-gate.sh` (new): each write skill's confirm step lexically
  precedes its `accelerator <provider> …` mutation invocation, with a committed
  reversed-body fixture proving the guard fails.
- Doc-vs-binary keyword parity over all sixteen repointed bodies, with an
  anti-vacuity match count and a committed stale-keyword fixture.
- `lint:scripts:check` (stale-library + exec-bit) after each deletion.
- `test:unit` (python coverage, ruff equality, mise partition, exec-bit
  reconciliation) after each guard edit.

### Manual testing

1. Run each subcommand against a **disposable/sandbox** Jira project / Linear
   team (never a shared or production one — the write flows leave real issues,
   comments and transitions that are not VCS-recoverable) and diff against 0210's
   committed 2026-08-21 contract evidence.
2. Exercise a full write flow through the repointed `SKILL.md` and confirm the
   resolved-intent preview gate fires before the mutation and the body branches
   on the keyword.
3. Confirm `init verify` never prints a credential.

## Performance Considerations

Synchronous blocking `reqwest` (no async runtime), matching the client crates.
Timeout precedent for small JSON bodies is 10s connect / 30s read / 30s write
(`cli/github/src/octocrab_client.rs:8-10`); Linear's upload path is 60s
(`cli/linear-client/src/upload.rs:35`). Registering two tokens adds 16 upload
assets, 8 signing targets and 2 manifest entries across the four targets in
`tasks/shared/targets.py:3-8` — all derived from `DISPATCHED_SUBBINARIES`, with
no per-platform CI matrix to extend.

## Migration Notes

A mixed bash/`accelerator` state on `main` is safe between phases (0167's
validation precedent). Each phase is green independently; recovery from any
phase is a VCS revert. The transient floor value is 20 (after Phase 2) and
removed (after Phase 4).

**Phase ordering constraint.** Phase 0 must precede Phase 4 — it is the only
thing that keeps the Jira cluster's deletion from redding `cli/jira-client`'s
default-profile test lane. It has no other ordering dependency and can land
first.

**Init cache compatibility (Decision 21).** The `init` subcommands subsume cache
production (`site.json` for `@me` resolution, the refresh-fields custom-field
cache that `create`/`search` read to compose live mutations). Bash-era caches
carry no version envelope, so an **absent marker is classified as the implicit
bash-era version and reads unchanged** — no existing install must re-initialise
on the happy path (confirmed by a test over a real markerless bash fixture).
Fail-closed fires only on a **present-but-unrecognised marker or an
unparseable/unknown shape** — the stale-future or corrupt cache that would
otherwise feed wrong custom-field values into a live-tenant mutation (the one
non-VCS-recoverable surface). The marker check fires **in the client crate's
cache-read path** (the compose site, reusing the `LegacyPolicy` fail-closed
precedent); `config`/`config-adapters` holds only the marker constant and a
generic version-check helper. A first `init` run stamps the marker so subsequent
reads are versioned.

**Release path.** 0203 becomes a release-path dependency only if a copyleft
component is recorded; 0210 introduced none
(`cli/licence-audit/new-trees.txt`), so it is not blocking. Caveat: that was
measured on the client crates, not on the two new `*-cli` binaries.

**Downstream.** 0174 is unblocked by Phase 4 (the integrations floor) and Phase
5 (the seven cluster `SHELL_LIBRARIES` entries plus `work-common.sh`, leaving it
thirteen repo-root entries rather than fourteen).

## References

- Work item:
  `meta/work/0211-integration-binaries-and-bash-cluster-retirement.md`
- Research:
  `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
  (follow-up passes 2026-08-19 and 2026-08-22)
- Parent epic: `meta/work/0171-jira-and-linear-integrations.md`
- Blockers, both merged: `meta/work/0210-…md` (PR #70),
  `meta/work/0212-work-item-script-cutover.md` (PR #73)
- Precedent: `meta/inventories/0167-{removal-set,suite-audit,divergences}.md`,
  `meta/validations/2026-07-19-0167-config-command-and-invocation-contract-migration-validation.md`
- Registration checklist: `tasks/README.md:499-663`; library crate `:665-722`
- Exit-code precedent: `cli/work-cli/src/exit_codes.rs`,
  `cli/work-cli/tests/exit_codes_parity.rs`; dispatch band
  `cli/tracker/tests/errors.rs:29-57`
- Frozen-corpus precedent: `cli/work-adapters/tests/corpus_hashes.rs`,
  `cli/work-adapters/tests/sync_baseline_corpus.rs`
- Keyword-discriminant precedent:
  `skills/work/create-work-item/SKILL.md:549-569`,
  `skills/work/sync-work-items/SKILL.md:108-109`
- Thin-CLI precedent: `cli/collaboration-cli/`, `cli/work-cli/`
- Client crates: `cli/jira-client/`, `cli/linear-client/`,
  `cli/http-test-support/src/lib.rs`
