---
type: "plan-review"
id: "2026-08-19-0211-integration-binaries-and-bash-cluster-retirement-review-2"
title: "Plan Review: Integration Binaries and Bash Cluster Retirement"
date: "2026-08-22T19:58:19+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "work-item:0211"
target: "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
relates_to: ["plan-review:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement-review-1"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "safety", "compatibility", "security"]
review_number: 2
review_pass: 3
tags: ["rust", "jira", "linear", "integrations", "cli", "cutover", "exit-codes", "registration"]
last_updated: "2026-08-22T21:33:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Integration Binaries and Bash Cluster Retirement

**Verdict:** REVISE

This is a re-review of a plan that review-1 drove to APPROVE across four passes at revision `9d9c07ed`; the plan was then substantively revised at `45fe2827` (new Phase 0, band widened to `70–74`, keyword-discriminant contract, fixtures-into-Rust, Decisions 13–19). The revision holds up well where review-1 concentrated — the credential-destination seam is now verified sound against the crate code, the parity-allowlist model is coherent, and the fail-closed create redesign is intact. But seven lenses surface **one critical and thirteen major findings**, clustered on three mechanisms the revision either introduced or re-measured: the search subcommand's contract against the stamps-only client projection, Decision 9's port-op discriminant totality (now over-specified and under-enumerated after 0212 added three methods), and the still-undecided init-cache cross-era compatibility.

### Cross-Cutting Themes

- **The search subcommand binds to a client projection that cannot carry its rendering contract** (flagged by: compatibility, architecture) — the single most serious finding. The 0210 client search op returns `tracker::Discovery` — external ids and timestamps only — yet the repointed search bodies render State/Assignee/Status columns and Jira's `--page-token`/`nextPageToken` pagination round-trip. Neither state, assignee, status, nor a caller-visible cursor exists anywhere in the client surface, so Phase 1's "search JSON-envelope golden matches the binary's emission" criterion is unsatisfiable and the search skills would ship a degraded table. This contradicts the plan's "client surface is complete (discovery/search)" claim and its no-reopen posture.
- **Decision 9's discriminant totality is over-specified and under-enumerated** (flagged by: correctness ×2, architecture, code-quality) — the "all seven port methods" obligation is impossible because `validate_update` is infallible (six fallible methods), and the enumerated inline-`TrackerError`-without-`Outcome` branches miss real ones (jira search compose error `client.rs:282`, `surface_read_failure` `:452`). Architecture notes the discriminant is a second parallel error surface bypassing the `RemoteTracker` port; code-quality notes it is a second failure taxonomy to keep in sync. The 2026-08-22 widening from four to seven methods is what introduced both the over-claim and the newly-relevant branches — the review-1 Pass-4 "provably total" verification no longer holds against the revised text.
- **Init-cache cross-era compatibility is the one undecided contract, and it feeds live mutations** (flagged by: architecture, safety, compatibility) — the plan leaves read-compatibility as an either/or. Safety escalates it: `create`/`search` compose remote mutations from the cached field mappings, so a bash-era cache that parses but carries different semantics would submit wrong custom-field values to a live tenant — the plan's one genuinely irreversible surface.
- **The captured-fixture pins guard accounting, not consumption** (flagged by: test-coverage ×3, safety, correctness) — the scenario ledger count-pins files-as-rows but nothing asserts a "ported" row is loaded by a test; the shared multi-POST `http-test-support` change has no test at its own boundary; behavioural exit-code completeness is derived from the bash capture rather than the binary's own classes; and the `EXPECTED_CASE_COUNT` pin is unquantified against the ported `adf-samples`.

### Tradeoff Analysis

- **Search contract fidelity vs not reopening the 0210 client crates**: the critical forces a decision the plan's no-reopen posture tried to avoid. Either budget an additive read-side client surface (identifier/title/state/assignee + a pagination cursor) as an explicit scope item, or record dropping State/Assignee/Status and the pagination round-trip as an accepted contract narrowing in the divergences ledger with the search bodies rewritten before estimation. The plan cannot both bind to `Discovery` and preserve the table.
- **Dual-contract coherence vs machine-parity retention**: keeping both the stdout keyword (body contract) and the ~45 exit integers (machine parity) is defensible, but nothing enforces that the keyword partition and the integer partition describe the same error classes. Deriving both from one per-condition enum makes them two projections of one source rather than two hand-maintained tables.

### Findings

#### Critical

- 🔴 **Compatibility**: Search subcommand cannot reproduce the SKILL.md table-rendering contract — client search is a stamps-only projection
  **Location**: Phase 1 §5 (Search JSON envelope) / Phase 3 §1 search mapping / Decision 11
  The client search op returns `tracker::Discovery { found: Vec<(ExternalId, RemoteTimestamp)>, complete }` (`tracker/src/lib.rs`), and both clients request minimal fields (jira `"fields":["updated"]` at `jira-client/src/client.rs:335`; linear discards even the title into the stamp list). The repointed bodies render `.state.name`/`.assignee.name` (`search-linear-issues/SKILL.md:66-71`) and Key/Summary/Status/Assignee (`search-jira-issues/SKILL.md:84-87`) — none modelled in the client surface, and `RemoteIssue` carries only `updated` + a projected title/description, so no `show` fallback recovers them. Phase 1's envelope-golden criterion is unsatisfiable; the search skills ship a degraded table.

#### Major

- 🟡 **Architecture**: Init cache format/path contract is left as an unresolved either/or across two boundaries
  **Location**: Migration Notes → Init cache compatibility
  `init` subsumes `site.json`/field-cache production that `create`/`search` consume, spanning the init-producer↔consumer boundary and the retired-bash-writer↔new-Rust-reader boundary. This is the one system-wide interface the otherwise line-precise plan does not decide. Resolve the branch in the plan — a concrete cache-format contract, where it is defined (a shared crate), and fail-closed behaviour as a fixed requirement, not an alternative.

- 🟡 **Safety**: Init cache compatibility does not address a parse-succeeds-but-semantically-wrong bash-era cache
  **Location**: Migration Notes → Init cache compatibility
  The two offered branches guard the clean endpoints (format-identical, or cleanly rejected) but not a subtly-compatible cache that parses yet carries different field mappings. `create`/`search` produce remote mutations from those mappings, so a silently-misread stale cache submits wrong custom-field values to a live tenant — not VCS-recoverable. Commit to fail-closed and require semantic incompatibility detection (a cache-format version/schema marker validated before any mutation is composed), not merely structural.

- 🟡 **Architecture**: Decision 9 bypasses the `RemoteTracker` port, forcing a second parallel result surface per client
  **Location**: Decision 9 / Current State Analysis (error conventions)
  Because the shared port returns only the lossy two-class `TrackerError`, the granular codes require a second discriminant-carrying result surface on each concrete client across every fallible port method, which the binaries bind to directly. Each client now maintains two error surfaces for the same operations that must stay in agreement. Acknowledge this as a deliberate composition-root binding and ensure the trait `impl` and the discriminant derive from one internal classification path so they cannot silently diverge.

- 🟡 **Architecture**: Decision 11 discriminant emission collides with the strict machine-output subcommands
  **Location**: Decision 11 / Phase 3 §3 (strict stdout goldens)
  The `<keyword>\t<detail>` last-line discriminant's suppression policy is spelled out only for `create --emit key`. `jira resolve-fields` emits a strict tab-separated four-field line ("trailing newline load-bearing") parsed field-wise by `create-jira-issue:65-66`; appending a discriminant line would move the four-field line off the end and break that parser and its golden. Enumerate explicitly which subcommands emit vs suppress the discriminant (at minimum `resolve-fields` and every bare-identifier projection), pinned by golden.

- 🟡 **Correctness**: Port-op discriminant "across all seven port methods" is impossible for the infallible `validate_update`
  **Location**: Decision 9 (and Phase 1 §1, Phase 3 §1)
  `validate_update` returns `tracker::ValidationOutcome` directly, not `Result<_, TrackerError>` (`tracker/src/lib.rs:464-469`; both impls infallible). Only six of the seven methods are fallible, so the totality obligation cannot be met for `validate_update`, and `keyword_surface`'s "every error class maps to exactly one keyword" risks a vacuous arm or a phantom mapping. Scope the discriminant to the six fallible methods and state `validate_update` is infallible (its Valid/Rejected are success-path outcomes on a separate keyword).

- 🟡 **Correctness**: Enumeration of inline `TrackerError` branches (no `Outcome`) is incomplete against the real client code
  **Location**: Decision 9
  Decision 9 names only two inline-without-`Outcome` branches, but the jira client builds `TrackerError` inline with no `Outcome` in at least two further places: the search/discovery compose error (`jira-client/src/client.rs:282`) and `resolve_project`/`preview_create` via `surface_read_failure` (`:452`). If the discriminant handles only the cited reasons plus classify-routed arms, these collapse to a wrong/default code. Enumerate every inline branch per provider and implement the wrapper as an exhaustive match over the client's own construction sites so a missed branch is a compile error.

- 🟡 **Test Coverage**: Scenario-fixture ledger count-pins accounting, not test consumption
  **Location**: Decision 15 / Phase 1 §4 / Phase 3 §2
  The ledger is count-pinned against the pre-deletion file list (40/95), guaranteeing every file is a row, but nothing asserts a "ported" row is loaded by a test — unlike the ADF corpus (`cases()`) and `sync_baseline_corpus.rs` (iterated under its pin). A scenario can be ported, counted, and referenced by no test, reintroducing exactly the dead test surface Decision 15 sets out to avoid. Add a per-binary inventory test asserting every file under `tests/fixtures/scenarios/` is referenced by ≥1 test.

- 🟡 **Test Coverage**: Shared multi-POST change to `http-test-support` has no direct test
  **Location**: Phase 1 §5 ("Multi-POST flows") / Testing Strategy
  The `Vec<Received>` per-key change is listed as a Phase 1 file but no test in `http-test-support/tests/server.rs` is specified; it would be exercised only indirectly by linear flow tests. This is shared infrastructure for every client crate — an off-by-one or ordering bug silently weakens multi-POST assertions everywhere, and the "`last_body` remains for single-POST callers" backward-compat claim is unpinned. Add tests asserting per-hit bodies recorded in order across ≥2 POSTs and `last_body` unchanged after multiple hits.

- 🟡 **Test Coverage**: Behavioural exit-code completeness is capture-derived, not enumerated from the binary's own classes
  **Location**: Phase 1 §3 / Testing Strategy
  The anti-vacuity count is "derived from the exhaustive capture" of *bash* codes, but several classes are binary-owned and absent from bash (the ~9 argument-validation `E_*` names, `USAGE`, `BadApiUrl`). A binary-owned variant could be mapped in `exit_codes.rs` and `keyword_surface` yet never behaviourally driven, leaving its routing unverified. Derive the behavioural test's class set from the closed keyword set (or an exhaustive match over the error enums) so adding a class forces a driving case.

- 🟡 **Code Quality**: Reusable test scaffolding is triplicated across work-cli, jira-cli and linear-cli with no shared home
  **Location**: Phase 1 & Phase 3 ("Mirror Phase 1 for Jira") / Testing Strategy
  The exit-code textual parser (already verbatim in `work-cli/tests/exit_codes_parity.rs`), the declared+behavioural capture harness, and the scenario-JSON→`Route` loader are structurally identical non-trivial machinery; the default reading is copy-paste into both new crates. A fix to the parser or scenario schema must then be made three times. Decide explicitly (a Decision or Phase 1 file) whether the reusable *test* machinery lives in a shared test-support crate consumed by both `-cli` crates.

- 🟡 **Safety**: Jira generator provenance is deferred to Phase 5, breaking the per-track revival-anchor principle
  **Location**: Phase 4 vs Phase 5 §5
  Phase 2 records the Linear revival anchor at the Linear deletion boundary, explicitly because independently-mergeable tracks must each carry their own. Phase 4 deletes the Jira cluster, `mock-jira-server.py` and the ADF drivers but defers Jira provenance to Phase 5. Between a merged Phase 4 and an unmerged Phase 5, the Jira generator exists only in unindexed history with no recorded anchor. Move Jira provenance into Phase 4, mirroring Phase 2.

- 🟡 **Compatibility**: Jira search loses the `--page-token`/`nextPageToken` pagination contract, `--render-adf` and `--fields`
  **Location**: Phase 3 §1/§4 / `search-jira-issues/SKILL.md` Steps 3-5
  Jira's `discover` follows `nextPageToken` internally and collapses to a single `complete: bool` (`jira-client/src/client.rs:266-324`), exposing no cursor, but the body documents `--page-token TOK`, a next-page round-trip, `--render-adf`, and `--fields a,b,c` — none mapping onto the port surface, and the kept `argument-hint` still advertises them. The repointed skill would advertise flags the binary cannot honour and drop multi-page browsing. Enumerate the non-surviving flags, decide each (reproduce via client change, or drop-with-ledger-row), and strip dropped flags from the argument-hint in the same commit.

- 🟡 **Security**: No positive guard that the shipped release binary excludes `test-loopback`
  **Location**: Decision 10 / Phase 1 §2 / Phase 3 §1
  The loopback/cleartext admission and Jira's `base_url`-bypassing direct-`Credentials` branch hinge on the `test-loopback` feature being off in the shipped binary. The plan tests that the default build rejects loopback and warns against a crate-dir `.cargo/config.toml`, but nothing asserts `tasks/build.py`'s release invocation never passes `--features test-loopback`. A single build-recipe mistake would silently ship a binary with the destination control disabled. Add a `#[cfg(all(feature = "test-loopback", not(debug_assertions)))] compile_error!(...)` guard per `-cli` crate plus a build-system assertion over the release invocation.

#### Minor

- 🔵 **Architecture**: Two redundant outcome representations (keyword + integer) with no single-source enforcement
  **Location**: Decision 11 / Decision 6
  Every outcome is represented twice — the stdout keyword and the retained exit integer — each independently pinned across ~five sites in two crates plus the skill bodies, and nothing enforces that the two partitions describe the same error classes. Derive both from one per-condition enum so a single match arm defines both.

- 🔵 **Architecture**: Decision 17 does not state the two producers share resolution logic
  **Location**: Decision 17 (two-producer tab contract)
  The resolved-fields tuple has two producers reconciled only by a field-for-field agreement test. If each binary reimplements resolution, the test guards output byte-for-byte while the logic silently duplicates and drifts. State that both render one shared `config`/`config-adapters` resolution path (confirm `work create --push --dry-run` uses the same source), so the tab contract is a formatting difference over one computation.

- 🔵 **Correctness**: Seam snippet passes `&Url` to `url_is_allowed`, which takes `&str`
  **Location**: Phase 1 §2 (base-URL seam code snippet)
  The snippet calls `url_is_allowed(&uri, …)` but the real signature is `url_is_allowed(url: &str, allow_loopback: bool)` (`linear-client/src/upload.rs:189`) — it parses internally. As written it is a type error; an implementer may mis-split parse-versus-validate. Pass the raw string (keeping the parsed `Url` for the endpoint) or change the promoted signature to `&Url`; make the snippet match.

- 🔵 **Correctness**: Post-create "exit 16" is asserted, not derived from the captured bash oracle
  **Location**: Decision 5 / Phase 3 §3 (`create --emit key`)
  The condition is built as `TrackerError::Terminal` inline with no numeric code (`jira-client/src/client.rs:506-513`); 16 is merely `NonJsonBody` in `classify::bash_code`, while the bash create flow cites the 100-107 range. The prose pins a specific code unverified against the oracle the fixture captures. Derive the emit-key post-create code from the captured fixture and state it as captured with its distinct reason arm.

- 🔵 **Correctness**: `EXPECTED_CASE_COUNT` pin is unquantified relative to the ported `adf-samples`
  **Location**: Phase 0 §3 and §4
  §3 pins over the 56 committed cases; §4 then ports some of the 43 `adf-samples/` as new cases, growing the count. The criteria say "the case count is pinned" without the final number or whether the pin is set before or after reconciliation. State the final count (56 + ported) explicitly and set the manifest and pin once, after reconciliation.

- 🔵 **Test Coverage**: Frozen-oracle reader is not exercised by the self-test and has no fail-loud-on-empty guard
  **Location**: Phase 0 §2-3 / Success Criteria
  Post-conversion `frozen_oracle` is the sole oracle, but the self-test drives only the comparison helpers, and the live-anchor test degenerates into committed-vs-committed once its `run_oracle` calls become `frozen_oracle`; a present-but-empty `oracle.out` could pass vacuously. Add a test that `frozen_oracle` hard-fails on missing/empty files (mirroring `run_oracle`'s fail-not-skip) and asserts non-emptiness where output is expected.

- 🔵 **Code Quality**: Keyword set should be a typed enum projection, not a bare-string set enforced only by a test
  **Location**: Decision 11 / Phase 1 §3 (`keywords.rs`, `keyword_surface.rs`)
  The house precedent is a typed enum with a projection (`PushOutcome::keyword(self) -> &'static str`, compiler-exhaustive). The plan's phrasing suggests string constants whose exhaustiveness rests on a runtime test — a new variant compiles with no keyword until the test catches it. Model each subcommand's outcome as an enum with a `keyword()` projection; `keyword_surface` becomes a backstop and golden-pin.

- 🔵 **Code Quality**: Wholesale work-cli shape inherits per-handler setup boilerplate, now duplicated across two crates
  **Location**: Decision 6 / Phase 1 §1 main.rs
  The work-cli handlers repeat a ~12-line `current_dir()` + `compose(...)` + service/store setup block; replicating the shape across both crates means ~20 more copies. Keep the inline-`ExitCode` shape but extract the context composition into a per-crate helper returning the assembled context (or an early-return `ExitCode`), so each handler is parse→call→render.

- 🔵 **Code Quality**: The ~45-code Jira module-doc-of-record risks comment-heavy prose against the repo's low-comment culture
  **Location**: Decision 6 amended / Phase 3 §1
  Jira's `exit_codes.rs` must document ~45 codes across nine tables plus SurfaceError(11)/ClientError(13)/AdfError(7); a prose line per code restates what a self-descriptive const name conveys, in tension with the repo's very-low comment tolerance. Scope the module doc to the genuinely non-obvious (safety-critical classes, each remap's *why*, divergence rationale) and let const names + the parity fixture carry the mechanical mapping.

- 🔵 **Code Quality**: The port-op discriminant is a second failure taxonomy alongside `TrackerError` to keep in sync
  **Location**: Decision 9
  The discriminant is the right typed choice but introduces two representations of the same failure taxonomy on one path, duplicated across both client crates. Make explicit that the behavioural exit-code test enumerates every discriminant arm across all fallible methods so an out-of-sync arm reds a test, and consider a match structure that forces a compile touch-point on a new arm.

- 🔵 **Safety**: Doc-vs-binary keyword parity guard is one-directional (body→binary, not binary→body)
  **Location**: Phase 4 §2
  The guard asserts every keyword a body branches on exists in the binary's set, but not that every keyword the binary emits is handled by the body. For read flows this is a wrong-display risk; for write flows it is bounded by Decision 5's fail-closed default, but the guard does not itself guarantee that. Extend the guard to assert binary-keywords ⊆ body-handled-tokens, or document and test that every write body defaults to no-writeback on an unrecognised keyword.

- 🔵 **Compatibility**: Init cache-file format compatibility is deferred, not decided
  **Location**: Migration Notes → Init cache compatibility
  Because cutover is phased per provider, a user upgrading mid-sequence can hold a bash-era cache for the just-cutover provider. If the fail-closed branch is chosen without settling read-compatibility, existing users silently lose `@me` resolution / field caching until re-init, with no upgrade prompt surfaced. Resolve the branch during planning (prefer reading the bash format unchanged, verified by a fixture-cache test); if re-init is required, name the advertised step in the repointed init bodies, not only the ledger.

- 🔵 **Compatibility**: New trailing keyword line changes stdout shape for any last-line machine consumer
  **Location**: Decision 11
  Every non-projection subcommand gains a final `<keyword>\t<detail>` line. The sixteen bodies and strict goldens are handled, but the discriminant is asserted as the contract that "keeps a machine consumer working", so its trailing-line placement is the new contract. Document in the `exit_codes.rs` module doc that the discriminant is always the final line and payload consumers must not read the last line blindly.

- 🔵 **Security**: Jira override allowlist source is unspecified — could bypass shared-config/tracked-source refusals
  **Location**: Phase 3 §1 (Jira seam) / Decision 10 (Jira arm)
  `base_url(site, allowed)` takes an `allowed` list, and `allowed_sites()` refuses an allowlist from shared/team config or a tracked source. The plan says the override routes "through `base_url` unchanged" but never states which list is passed. If it passes an ad-hoc list, the override could reach a host plain config would refuse. State that the override reuses `allowed_sites(context)` and add a test that a self-hosted override host absent from personal `jira.allowed_sites` is rejected.

#### Suggestions

- 🔵 **Architecture**: Independently-mergeable tracks share the `_EXPECTED_INTEGRATIONS_SUITES` counter
  **Location**: Decision 1 / Implementation Approach
  Both Phase 2 and Phase 4 edit the same floor count (32→20, then removed), so whichever track lands second has a guaranteed rebase point, and Phase 5 depends on both clusters being gone. The "independent" framing understates a real ordering coupling. Make the sequencing constraint explicit in Migration Notes so the shared-counter edit is a deliberate serialization point, not a merge surprise.

- 🔵 **Safety**: The frozen ADF "never regenerate from Rust output" invariant is a documented rule, not a machine check
  **Location**: Phase 0 §3
  The property keeping the differential from collapsing into a tautology is enforced only by the manifest header. After Phase 4 the drivers are gone, so a maintainer facing a digest mismatch could regenerate from Rust output. Keep the capture script committed and self-contained (it is) and make the header explicit about checking out the driver revision, so the only sanctioned regeneration path stays discoverable.

- 🔵 **Test Coverage**: Parallel frozen representations of the same bash output with no cross-check
  **Location**: Phase 0 §1-3 (`oracle.out` vs existing `expected.md`/`expected.adf.json`)
  The corpus already commits `expected.*` (the raw oracle stdout), and Phase 0 adds a parallel `oracle.out`/`oracle-status.txt` for the same cases with no cross-check. Two frozen artefacts of one run can drift silently. Either reuse the existing `expected.*` as the frozen oracle, or assert they agree.

- 🔵 **Security**: No-token assertion should also cover the tracing/log sink and malformed-token diagnostics
  **Location**: Decision 3 / Phase 1 §5
  The sentinel test asserts absence from stdout and stderr. The transport logs via `tracing` and `validate_token`/`MalformedToken` name the offending byte class rather than the token — safe today, but a stdout/stderr-only assertion would not catch a future token added to a `tracing` event or an error `Display`. Extend the assertion to the `tracing` sink and add a malformed-token diagnostic case.

- 🔵 **Architecture**: A non-API config-resolution subcommand lives inside the Jira API-adapter binary
  **Location**: Decision 4 (`resolve-fields`)
  `jira resolve-fields` makes no API call yet sits among the API flows — a minor cohesion smell, accepted as domain-aligned. Factor its config resolution to share the credential/config plumbing rather than duplicate it.

### Strengths

- ✅ The credential-destination seam (Decision 10) is verified sound against the crates: `url_is_allowed` enforces https + userinfo-refusal + `*.linear.app` label matching, `base_url` enforces https/no-userinfo/no-query/no-fragment/default-port/`*.atlassian.net`, `host_is_admissible` is host-only, both transports pin `redirect::Policy::none()`, and the release path validates the override before any token attaches — no release path redirects the token (security).
- ✅ Phase ordering is enforced by the test suite, not convention: Phase 4's "`cargo nextest run -p jira-client` green with the cluster gone" means deleting the ADF drivers before Phase 0 lands reds `mise run`, so safe ordering is fail-closed (safety, correctness).
- ✅ Create flows fail closed against the orphaned-remote/exit-16 state: a non-success keyword suppresses the writeback and blocks retry, and post-create-unwritable surfaces an explicit "created remotely as `<key>`; reconcile manually" — directly preventing duplicate remote issues (safety, correctness).
- ✅ The confirm-before-mutate gate has real anti-vacuity: `test-skill-write-gate.sh` asserts the confirm step is present, lexically precedes the mutation, rejects both in one fenced block, with a committed reversed-body fixture proving it can fail (safety).
- ✅ Phase 0 preserves `adf_differential_self_test.rs` unedited through the frozen conversion, keeping the proof that the comparison can still reject — the property most at risk in a freeze (test-coverage, correctness, security).
- ✅ The widened `70–74` reservation is verified against the actual inlined dispatch oracle (`tracker/tests/errors.rs:29-57`, five rows incl. `E_DISPATCH_UNCONFIGURED`=74), and no genuine provider code falls in the band except the search codes the plan explicitly remaps (correctness, compatibility).
- ✅ The two-producer tab contract (Decision 17) faithfully matches the on-disk shape `work create --push --dry-run` already emits, making the four-field jira form a non-breaking tracker-prefix-stripped projection (compatibility).
- ✅ Partial-deletion states red the build via set-equality and count pins (`_RECONCILED_LIBRARIES`, exact-tuple `_SUBBINARY_DESCRIPTIONS`, `SHELL_LIBRARIES` 21→19→14→13, the fixture-ledger counts) (safety).

### Recommended Changes

1. **Resolve the search-subcommand contract before estimating the search phases** (addresses: search-stamps-only-projection, jira-search-pagination-flags). Decide, in the plan: either budget an additive read-side client surface returning identifier/title/state/assignee plus a caller-visible pagination cursor (an explicit scope item, not a Phase-1 golden reconciliation), or record dropping State/Assignee/Status and the `--page-token`/`--render-adf`/`--fields` capabilities as an accepted contract narrowing in the divergences ledger, rewrite the search bodies and strip the dropped flags from the `argument-hint` in the cutover commit, and correct the "client surface is complete (discovery/search)" claim.

2. **Rescope Decision 9 to the fallible methods and enumerate every inline branch** (addresses: d9-seven-methods-impossible, d9-inline-branches-incomplete, d9-second-taxonomy). State six fallible port methods (not seven) and that `validate_update` is infallible; enumerate every inline-`TrackerError`-without-`Outcome` branch per provider (add the search compose error and `surface_read_failure` reasons) and implement the discriminant as an exhaustive match over the client's own construction sites so a missed branch is a compile error; note that the behavioural test enumerates every arm.

3. **Decide the init-cache cross-era contract as a fixed requirement** (addresses: init-cache-undecided-arch, init-cache-semantic-safety, init-cache-deferred-compat). Commit to fail-closed with a cache-format version/schema marker validated before any mutation is composed, define where the format lives (a shared crate), and either verify read-compatibility with a bash-produced fixture cache or name the advertised re-init step in the repointed init bodies.

4. **Make the captured-fixture pins guard consumption, not just accounting** (addresses: ledger-consumption, multipost-untested, behavioural-completeness, case-count-unquantified, frozen-reader-fail-loud). Add a per-binary inventory test that every ported scenario is referenced by ≥1 test; add `http-test-support/tests/server.rs` tests for per-hit body ordering and `last_body` after multiple hits; derive the behavioural exit-code class set from the closed keyword set (or an exhaustive enum match); state the final ADF case count and set the pin once after reconciliation; and add a `frozen_oracle` fail-loud-on-missing/empty test.

5. **Close the remaining seam and provenance gaps** (addresses: no-release-loopback-guard, jira-allowlist-source, jira-provenance-timing, seam-snippet-type-error, exit-16-asserted). Add a `compile_error!` guard forbidding `test-loopback` in release plus a build-system assertion over the release invocation; state the Jira override reuses `allowed_sites(context)` with a rejection test; move Jira generator provenance into Phase 4; fix the `url_is_allowed(&str)` snippet; and derive the post-create exit code from the captured fixture rather than asserting 16.

6. **Reduce cross-crate duplication where cheap** (addresses: triplicated-test-scaffolding, keyword-typed-enum, per-handler-boilerplate, module-doc-comment-heavy, dual-representation-single-source). Decide whether the exit-code parser, capture harness, and scenario loader live in a shared test-support crate; model keywords as typed enum projections; extract the per-handler context composition; scope the Jira module doc to the non-obvious; and derive keyword and integer from one per-condition enum. These are maintainability items — group them so they do not each become a separate re-review point.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan applies the established composition-root pattern well — two thin imperative-shell `*-cli` binaries over existing client crates, with domain logic, timeouts and retry policy kept in the crates. Structural boundaries, the Phase-0 decoupling of the Rust test lane from doomed bash, and the functional-core/imperative-shell separation are sound. The main weaknesses are an under-specified cross-era data contract (the init cache), the bypass of the `RemoteTracker` port that forces a second parallel result surface per client, and the coherence burden of two redundant outcome-signalling contracts whose emission policy collides with the strict machine-output subcommands.

**Strengths**:
- Thin-adapter composition-root design consistent with collaboration-cli/work-cli/corpus-cli.
- Resilience correctly located in the client crates (port owns timeouts/retries); binaries add none.
- Phase 0 removes a genuine cross-language coupling before the scripts are deleted, using the proven frozen-corpus-plus-digest technique with capture-before-deletion explicit.
- The base-URL seam keeps loopback a caller-supplied runtime bool gated on a test-only feature, not an env/`debug_assertions` switch.
- The additive `Vec<Received>` change to shared `http-test-support` is backward-compatible.

**Findings**:
- major / high — Init cache format/path contract left as an unresolved either/or (Migration Notes: Init cache compatibility).
- major / medium — Decision 9 bypasses the port, forcing a second parallel result surface per client (Decision 9 / Current State Analysis).
- major / medium — Decision 11 discriminant emission collides with strict machine-output subcommands (Decision 11 / Phase 3 item 3).
- minor / medium — Two redundant outcome representations (keyword + integer) with no single-source enforcement (Decision 11 / Decision 6).
- minor / medium — Decision 17 does not state the two producers share resolution logic (Decision 17).
- suggestion / medium — Independently-mergeable tracks share the `_EXPECTED_INTEGRATIONS_SUITES` counter (Decision 1 / Implementation Approach).

### Correctness

**Summary**: Logically well-structured; the captured-oracle parity model, the count-pinned allowlist, the fail-closed create redesign, and capture-before-delete sequencing are sound. The most significant gaps concern the port-op discriminant totality (Decision 9): "all seven port methods" is factually over-specified because one method is infallible, and the enumeration of inline branches that bypass `Outcome` is incomplete against the real client code. The widened `70–74` band and its remap are internally consistent with the actual dispatch oracle.

**Strengths**:
- The parity oracle is independent of the constants it guards (captured bash values, not re-derived).
- Phase 0 preserves `adf_differential_self_test.rs` unedited, keeping the proof the comparison can reject.
- Capture-before-delete sequencing explicit and correct; Phase 0 ordering before Phase 4 stated.
- The create fail-closed redesign preserves the exit-16/orphaned-remote invariant.
- The widened `70–74` reservation matches `tracker/tests/errors.rs:29-57`; no genuine provider code falls in-band except the remapped search codes.

**Findings**:
- major / high — Discriminant "across all seven port methods" impossible for the infallible `validate_update` (Decision 9).
- major / medium — Enumeration of inline `TrackerError` branches (no `Outcome`) incomplete against real client code (Decision 9).
- minor / high — Seam snippet passes `&Url` to `url_is_allowed`, which takes `&str` (Phase 1 §2).
- minor / medium — Post-create "exit 16" asserted, not derived from the captured bash oracle (Decision 5 / Phase 3 §3).
- minor / medium — `EXPECTED_CASE_COUNT` pin unquantified relative to the ported `adf-samples` (Phase 0 §3-4).

### Test Coverage

**Summary**: Unusually strong on discipline: it preserves the ADF self-test that proves the comparison can reject, ties every count-pin and allowlist to an oracle independent of the constants, and routes non-network exit-code classes through their real sources. The principal gaps: the scenario ledgers pin accounting rather than consumption, the shared multi-POST change has no test of its own, and the frozen-oracle conversion removes the last live-bash anchor without a reader-integrity/fail-loud test.

**Strengths**:
- Phase 0 leaves the comparison helpers and the self-test unedited, so the property most at risk provably survives.
- Anti-vacuity designed in throughout (reversed-body and stale-keyword fixtures, non-zero match counts, per-`(flow,name)` uniqueness, count-pinned keyword sets, oracle-independent-of-constants).
- Behavioural exit-code test specified to cover non-network classes via real sources.
- Paths that would escape into manual verification are demanded as automated (`from_config`, init-verify no-token, multi-POST per-hit bodies).
- Faithful reuse of proven precedents (`corpus_hashes.rs`, `sync_baseline_corpus.rs`, work-cli parity, `http-test-support` Route/Sequence).

**Findings**:
- major / medium — Scenario-fixture ledger count-pins accounting, not test consumption (Decision 15 / Phase 1 §4 / Phase 3 §2).
- major / high — Shared multi-POST change to `http-test-support` has no direct test (Phase 1 §5 / Testing Strategy).
- major / medium — Behavioural exit-code completeness capture-derived, not enumerated from the binary's classes (Phase 1 §3).
- minor / medium — Frozen-oracle reader not exercised by the self-test and no fail-loud-on-empty guard (Phase 0 §2-3).
- suggestion / low — Parallel frozen representations (`oracle.out` vs `expected.*`) with no cross-check (Phase 0 §1-3).

### Code Quality

**Summary**: Rigorous, and its central typed-design choices are sound: the port-op discriminant (Decision 9) rejects stringly-typed detail-parsing, and folding the exit-code document of record into the module doc (Decision 6) removes a drift-prone second source. The main risks are duplication — jira-cli and linear-cli are near-mirror roots, and the plan mirrors substantial test scaffolding across both without deciding where reusable machinery lives — plus a bare-string keyword set that should be a typed enum projection and a ~45-code module doc that risks comment-heavy prose.

**Strengths**:
- Decision 9 is a genuinely clean typed design with a lint/grep guard forbidding any `detail` substring-parse.
- Decision 6 collapses the document of record into `exit_codes.rs` and the parity test parses that same file — single-source-of-truth.
- Testability seams are concrete and house-style (`from_config`-branch test, seam-revalidation, init-verify sentinel no-leak, loopback as a caller-supplied runtime bool).
- The parity oracle is deliberately independent of the constants (frozen literals), avoiding a tautology.

**Findings**:
- major / medium — Reusable test scaffolding triplicated across work-cli/jira-cli/linear-cli with no shared home (Phase 1 & 3 / Testing Strategy).
- minor / high — Keyword set should be a typed enum projection, not a bare-string set enforced only by a test (Decision 11 / Phase 1 §3).
- minor / medium — Wholesale work-cli shape inherits per-handler setup boilerplate duplicated across two crates (Decision 6 / Phase 1 §1).
- minor / medium — The ~45-code Jira module-doc-of-record risks comment-heavy prose against the repo's low-comment culture (Decision 6 amended / Phase 3 §1).
- minor / low — The port-op discriminant is a second failure taxonomy alongside `TrackerError` to keep in sync (Decision 9).

### Safety

**Summary**: A large but well-contained deletion plan for a dev-tooling plugin where every deleted artefact is VCS-tracked, so recovery is a revert and the irreversible surface is narrow (live tenant mutations, plus corpus revival). It handles its highest-stakes concerns well: phase ordering is enforced by the suite itself, create flows fail closed against the orphaned-remote state, the confirm gate carries genuine anti-vacuity, and set-equality/count pins red the build on partial deletion. Residual concerns are an asymmetric Jira revival anchor, the unresolved init-cache branch feeding mutations, and a one-directional keyword parity guard.

**Strengths**:
- Phase ordering fail-closed via the suite (Phase 4 can't merge green without Phase 0).
- Create flows fail closed against orphaned-remote/exit-16, preventing duplicate remote issues.
- Confirm-before-mutate gate has real anti-vacuity with a committed failing fixture.
- Partial-deletion states red the build via set-equality and count pins.
- ADF oracle captured and digest-pinned before deletion; self-test unedited; base-URL seam fail-safe on unparseable/non-admissible.

**Findings**:
- major / medium — Jira generator provenance deferred to Phase 5, breaking the per-track revival-anchor principle (Phase 4 vs Phase 5 §5).
- major / medium — Init cache compatibility does not address a parse-succeeds-but-semantically-wrong bash-era cache (Migration Notes).
- minor / medium — Doc-vs-binary keyword parity guard is one-directional (Phase 4 §2).
- suggestion / low — The frozen "never regenerate from Rust output" invariant is a documented rule, not a machine check (Phase 0 §3).

### Compatibility

**Summary**: The revision handles most contract surfaces well: the widened band is verified correct, the two-producer tab contract matches the on-disk shape, and the keyword-discriminant migration preserves the machine-parity integers while suppressing the discriminant for strict bare-identifier goldens. The one serious gap is the search flow: the 0210 client exposes search only as a sync-oriented stamps-only projection (keys + timestamps), which cannot reproduce the search bodies' rendering contract or Jira's pagination round-trip, yet the plan binds search to exactly that projection. Init cache format compatibility is left as an unresolved either/or.

**Strengths**:
- The `70–74` band verified correct against `tracker/tests/errors.rs:29-57`.
- The two-producer tab contract matches `create-work-item/SKILL.md:506-513` — a faithful non-breaking projection.
- The keyword-discriminant migration preserves the in-body integers as pinned machine parity; strict goldens suppress the discriminant for projections.
- Provider-facing diagnostics preserved (composed-query stderr line, `--quiet`, every referenced `E_*` name, init-verify no-token).

**Findings**:
- critical / high — Search subcommand cannot reproduce the SKILL.md table-rendering contract — client search is a stamps-only projection (Phase 1 §5 / Phase 3 §1 / Decision 11).
- major / high — Jira search loses the `--page-token`/`nextPageToken` pagination contract, `--render-adf` and `--fields` (Phase 3 §1/§4 / `search-jira-issues/SKILL.md`).
- minor / medium — Init cache-file format compatibility deferred, not decided (Migration Notes).
- minor / low — New trailing keyword line changes stdout shape for any last-line machine consumer (Decision 11).

### Security

**Summary**: The credential-destination seam (Decision 10) is fundamentally sound — verified that `url_is_allowed` enforces https + userinfo-refusal + `*.linear.app` matching, `base_url` enforces https/no-userinfo/no-query/no-fragment/default-port/`*.atlassian.net`, `host_is_admissible` is host-only, and both transports pin `redirect::Policy::none()`. In release the override is validated to an allowlisted TLS-only host before the token attaches, so no release path redirects the token. Residual concerns are defence-in-depth: nothing positively proves the shipped binary excludes `test-loopback`, and the Jira override's allowlist source is unspecified.

**Strengths**:
- The token destination is validated before any credential attaches, confined to `*.atlassian.net` / `*.linear.app` — correct trust-boundary placement.
- Loopback admission is a caller-supplied runtime bool driven by the feature, never an env read and deliberately not `debug_assertions`.
- Jira's `base_url` stays strict and unchanged; the test-only mock path is isolated to a gated direct-`Credentials` branch.
- Both transports refuse redirects; the upload path strips Authorization/Host, rejects CR/LF injection, redacts the signed query; `Secret` redaction plus the sentinel no-emit test close the disclosure surface.
- Phase 0's frozen conversion keeps the self-test unedited, preserving proof the comparison can reject.

**Findings**:
- major / medium — No positive guard that the shipped release binary excludes `test-loopback` (Decision 10 / Phase 1 §2 / Phase 3 §1).
- minor / medium — Jira override allowlist source unspecified — could bypass shared-config/tracked-source refusals (Phase 3 §1 / Decision 10).
- suggestion / low — No-token assertion should also cover the tracing/log sink and malformed-token diagnostics (Decision 3 / Phase 1 §5).

## Re-Review (Pass 2) — 2026-08-22

**Verdict:** APPROVE

All seven lenses were re-run against the edited plan. **Every Pass-1 finding — the critical and all thirteen majors — is resolved**, and several were verified directly against the crates: the search read-side op is feasible (`jira-client` `discover_page` already round-trips `nextPageToken`; linear's GraphQL selection can add `state`/`assignee`), `validate_update` is genuinely infallible (`tracker/src/lib.rs:464-469`), the base-URL seam types match, and the `test-loopback` release guard's `not(debug_assertions)` discriminator holds against `[profile.release]`. The pass then surfaced a cluster of **new majors, all consequences of the Pass-1 edits** — most agreed across lenses: Decision 21's marker rule was self-contradictory (three lenses), Decision 20's fixed projection dropped jira `--fields`/`--render-adf` and its query widening coupled the sync `fetch_all` read, the Decision 9 enumeration was still incomplete (jira `fetch_all:586` omitted, stale linear anchors), the behavioural-test axis was too coarse, the new `cli-test-support` crate was untested, and the Jira `allowed_sites` reuse was under-specified. A **second edit pass has addressed every one**.

### Previously Identified Issues (Pass-1 findings)
- 🔴 **Compatibility**: Search stamps-only projection — Resolved (Decision 20 additive read-side op; feasibility verified against both clients).
- 🟡 **Architecture / Safety**: Init cache undecided / semantic hazard — Resolved (Decision 21 committed contract).
- 🟡 **Architecture**: Decision 9 parallel result surface — Resolved (deliberate composition-root binding; single classification path via the funnel enum).
- 🟡 **Architecture**: Decision 11 resolve-fields collision — Resolved (suppression enumerated + golden-pinned).
- 🟡 **Correctness**: Decision 9 "seven methods" impossible — Resolved (rescoped to six fallible; `validate_update` infallible, verified).
- 🟡 **Correctness**: Decision 9 inline branches incomplete — Resolved this pass (completed in the second edit; see New Issues).
- 🟡 **Test Coverage**: Ledger consumption / multi-POST untested / behavioural completeness — Resolved (inventory test, `server.rs` boundary test, enum-driven class set).
- 🟡 **Code Quality**: Triplicated scaffolding — Resolved (`cli-test-support` crate).
- 🟡 **Safety**: Jira provenance timing — Resolved (moved into Phase 4 item 4).
- 🟡 **Compatibility**: Jira search flags/pagination — Resolved this pass (completed in the second edit; see New Issues).
- 🟡 **Security**: No release-loopback guard — Resolved (`compile_error!` + byte-level staged-binary grep; verified sound).
- 🔵 **Minors** (seam snippet type, exit-16 derivation, `EXPECTED_CASE_COUNT`, keyword typed enum, one-directional parity, allowlist source, tracing sink) — Resolved.

### New Issues Introduced (surfaced by Pass 2, now addressed in a second edit pass)
- 🔴 **Architecture / Safety / Compatibility**: Decision 21's "absent marker → fail closed" contradicted its own read-compatibility guarantee, since bash-era caches (`cache.rs`, `jira-init-flow.sh:113-117`) carry no marker. **Addressed**: absent marker = implicit bash-era version (reads unchanged); fail-closed only on a present-but-unrecognised/unparseable one; the check homed in the client crate's cache-read path (not provider-agnostic config).
- 🟡 **Compatibility**: Decision 20's fixed four-field projection dropped jira `--fields` passthrough and `--render-adf`. **Addressed**: the jira op returns each issue's requested `fields` map (raw, `description` included), not a fixed projection.
- 🟡 **Architecture / Correctness**: Decision 20 widening the shared linear `SEARCH` const coupled the sync `fetch_all` read. **Addressed**: a distinct search-projection query const, leaving `fetch_all` lean.
- 🟡 **Correctness**: Decision 9 enumeration still omitted jira `fetch_all` unsafe-id (`client.rs:586`) and carried stale linear anchors. **Addressed**: jira branch added; anchors corrected to `:340`/`:407`; reframed as a funnel-through-one-enum refactor (not purely additive).
- 🟡 **Test Coverage**: Behavioural-test axis ("keyword set") was coarser than the error-class taxonomy (many-to-one). **Addressed**: pinned to an exhaustive match over the error enums.
- 🟡 **Test Coverage**: New `cli-test-support` crate had no boundary tests. **Addressed**: its own parser/loader boundary tests + a unified scenario schema.
- 🟡 **Security**: Jira `allowed_sites` is private and the planned test missed the shared-config/tracked-source provenance refusals. **Addressed**: promote `allowed_sites` pub (or a pub validator); tests assert Team-level + tracked-source + host-absent rejection.
- 🟡 **Code Quality**: The ~45-code Jira module-doc-of-record duplicated the fixture with no human audience. **Addressed**: scoped to bands/classes; `bash-exit-codes.txt` named authoritative.
- 🔵 **Minors** (JSON-subcommand discriminant in-envelope, Phase 0 restore-always command, byte-level release grep, per-handler `context()` helper, inventory assertion-anchoring, ported-ADF double anchor, non-atomic writeback arm, sandbox live-tenant, tracing-sink bullet) — All addressed.

### Assessment
**Approved — ready for implementation.** Across the initial pass and this re-review, every finding is resolved or consciously deferred; the new items this pass raised were self-inflicted by the Pass-1 edits and progressively more granular (a specific `client.rs` line, a query-const split, a test-axis wording) — the signature of convergence. Nothing outstanding is structural: the search-contract critical is closed by a scoped, feasibility-verified additive client op, and the credential-destination and cache-marker seams are fail-closed. The two most load-bearing second-pass rewrites — Decision 21's marker semantics and Decision 20's jira `fields`-map projection — are reasoned-safe against the crate code but were applied this pass without an independent confirming agent run; a short Pass-3 of correctness + compatibility would verify them, but is optional. Remaining deferred suggestions (reserved-band constant home, `resolve-fields` cohesion) are precedent-consistent and out of scope.

## Re-Review (Pass 3) — 2026-08-22

**Verdict:** APPROVE

Correctness and compatibility were re-run as a narrow confirmation pass on the two load-bearing second-pass rewrites the Pass-2 assessment flagged as reasoned-safe but not independently verified: Decision 21's cache-marker semantics and Decision 20's jira `fields`-map projection. **Both passes returned zero findings**, with every claim traced to the crate and bash sources.

### Previously Identified Issues (Pass-2 second-edit targets)
- 🟡 **Correctness**: Decision 9 inline-branch enumeration — Verified complete. Every fallible port method's inline `TrackerError` construction (not routed through `classify`) is exactly the enumerated set: jira `client.rs:282`/`:452-453`/`:506-513`/`:585-593`, linear `:339-346`/`:406-410`; no omitted branch. `validate_update` infallible confirmed (`tracker/src/lib.rs:464-469`, both impls).
- 🟡 **Compatibility / Correctness**: Decision 20 distinct query const — Verified. The shared linear `SEARCH` const (`client.rs:64-70`) is genuinely used by `fetch_all` via `page_all`→`fetch_page`, so a distinct projection query is required and leaves the sync `fetch_all` request shape untouched; jira's `fetch_all` builds its body inline (`"fields":["updated"]`) with no shared const.
- 🟡 **Compatibility**: Decision 20 jira `--fields`/`--render-adf`/pagination — Verified against `jira-search-flow.sh`: returning the resolved `--fields` map (raw, `description` included) plus a surfaced `nextPageToken` faithfully reproduces the bash single-page-plus-token contract and `--render-adf` rendering; no search-contract regression survives.
- 🟡 **Compatibility**: Decision 20 linear State/Assignee — Verified against `linear-search-flow.sh:159` (`state { name }`, `assignee { name }`, title preserved).
- 🔴→🟡 **Architecture / Safety / Compatibility**: Decision 21 marker contradiction — Verified resolved against `cache.rs` (markerless writes) and `jira-init-flow.sh:113-117` (bare `{site, accountId}`): absent marker = implicit bash-era (reads unchanged, no forced re-init); fail-closed only on present-but-unrecognised/unparseable; the `LegacyPolicy` precedent and the fields-cache read site (`discovery.rs`) both exist.
- 🔵 **Compatibility**: Decision 11 carrier split — Verified: JSON subcommands gain an additive top-level `outcome` field (forward-compatible), text subcommands a trailing line, strict-positional suppressed — no subcommand's stdout stops being a single valid document.
- 🔵 **Correctness**: base-URL seam `url_is_allowed(&str, bool)` and Decision 5 captured-fixture code — Verified against `upload.rs:189` and `classify.rs:45` (16 is `NonJsonBody`, unrelated to post-create-unwritable, correctly taken from the fixture).

### New Issues Introduced
_None._

### Assessment
**Approved — confirmed ready for implementation.** The two rewrites left unverified at Pass 2 are now checked directly against the crates with no findings, and no new issues were introduced. Every finding across the initial review and both re-review passes is resolved or consciously deferred; the deferred items (reserved-band constant home, `resolve-fields` cohesion) are precedent-consistent. No further review passes are warranted.
