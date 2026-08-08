---
type: plan-review
id: "2026-08-08-0197-accelerator-collaboration-pr-helper-cli-review-1"
title: "Plan Review: accelerator-collaboration: PR Helper CLI Implementation Plan"
date: "2026-08-08T16:30:32+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: ""
target: "plan:2026-08-08-0197-accelerator-collaboration-pr-helper-cli"
relates_to: []
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, security, standards, compatibility, documentation]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-08T21:49:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: accelerator-collaboration: PR Helper CLI Implementation Plan

**Verdict:** REVISE

The plan's phase sequencing, crate structure, and adherence to house conventions (ports-and-adapters, TDD, hand-written fakes, explicit divergence rationale) are strong and closely mirror proven precedent (`vcs`, `work`). However, two independently-confirmed critical defects sit at the core of the design: the base-repo resolver's REST-field choice is circular and cannot deliver the cross-fork-safety the source bash script provides, and the sync domain ports built in Phase 3 are never shown to actually connect to the async-only adapter built in Phase 4. Several major findings compound these — a dropped credential-file permission check, an AC requirement (repointed test suites) silently swapped for deletion, and two of three skill call sites that would be invisible to the dispatch-coherence guard as literally specified. The plan needs another pass before implementation begins.

### Cross-Cutting Themes

- **Base-repo resolution loses cross-fork-safety** (flagged by: Compatibility [critical], Correctness [critical, plus a related major finding]) — Compatibility and Correctness independently identified the same root defect: the REST response's top-level `url` field is a self-link that always echoes the queried `{owner}/{repo}` back — it cannot carry information about a different (upstream/base) repository. The source `pr-base-repo.sh` used `gh pr view --json url`, which relies on `gh`'s own fork-aware resolution and is explicitly documented in the script as "cross-fork-safe." The plan's replacement — parsing the *local* `origin` remote, then re-deriving owner/repo from a REST field that cannot differ from what was already queried — cannot reproduce that property. For any contributor working from a fork, this will most likely 404 or silently resolve to the fork's own repo instead of upstream. This is the single most consequential finding in the review: two lenses reached it independently via different reasoning paths, which is strong convergent evidence.
- **Sync/async boundary is never bridged** (flagged by: Architecture [critical], Code Quality [major], Correctness [related major]) — Phase 3 builds synchronous domain ports (`PullRequestLookup`/`PullRequestBodyUpdate`) and TDD's 12 characterization branches against them. Phase 4 then states the `octocrab`-backed adapter deliberately does *not* implement these sync traits, exposing only `async fn` inherent methods instead. Phase 6 describes `main.rs` awaiting `OctocrabClient` calls directly inside a single `block_on`, but never scaffolds the shim that would let the tested Phase 3 composition functions actually receive real data from that adapter. As written, either the domain crate's 12 tested branches are never exercised by production code, or `main.rs` silently duplicates the same branching logic against the async client untested.
- **The plan's literal edits don't fully satisfy the guarantees they claim to preserve** (flagged by: Test Coverage, Standards, Documentation) — three lenses each found a place where the plan's stated intent and its literal implementation detail diverge: Test Coverage found the AC's "repointed suites" verification strategy silently replaced by outright deletion (Phase 7) with no reconciliation; Standards found that two of the three rewritten skill call sites, as specified, would not be recognised by the dispatch-coherence guard's fenced-block parser; Documentation found that Phase 7's own success-criteria grep would fail against a stale `skills/github/scripts/README.md` the plan doesn't touch. None of these is individually as severe as the two critical findings, but together they suggest the plan's phase-by-phase detail needs a closer pass against its own stated constraints before implementation.
- **Credential handling has quietly weaker guarantees than its bash precedent** (flagged by: Security, Architecture) — Security found the new Rust resolver has no equivalent of `jira-auth.sh`/`linear-auth.sh`'s fail-closed local-config-file permission check (0600), and that neither the plan nor `FileConfigStore` enforces one on read. Architecture and Security separately both flagged the absence of any timeout/retry/backoff or redirect-host-allowlist on the new `octocrab`/`reqwest` client, unlike the workspace's one existing outbound-HTTP precedent (`Fetcher`), which is more consequential here because a leaked redirect could carry the `Authorization` header.

### Tradeoff Analysis

- **Literal AC compliance vs. design judgment (repointed vs. deleted test suites)**: The work item's AC states behaviour is "verified via repointed suites," but the plan's Phase 7 deletes the bash suites outright rather than repointing them, with no explanation. Deleting and replacing with Rust-native characterization tests may well be the better engineering call — bash suites stubbed via a PATH `gh` shim are poor scaffolding for testing an in-process HTTP client — but the plan should say so explicitly and show that the ~38 deleted bash cases are provably subsumed by the 16 new Rust tests, rather than silently diverging from the AC's literal wording.

### Findings

#### Critical

- 🔴 **Compatibility / Correctness**: Base-repo resolution derives owner/repo from a REST field that cannot carry that information
  **Location**: Phase 3: `collaboration` Domain Crate (`base_repo.rs`)
  The top-level `url` field on a `GET /repos/{owner}/{repo}/pulls/{pull_number}` response is always a self-link echoing the queried `{owner}/{repo}` — it cannot reveal a different base repository. The source bash script's cross-fork-safety came from `gh pr view --json url`'s own fork-aware resolution, which this design does not replicate.

- 🔴 **Architecture / Code Quality**: Sync domain ports and the async-only adapter are not shown to actually connect
  **Location**: Phase 4: `collaboration-adapters` / Phase 6: `collaboration-cli` Sub-binary
  Phase 3's `PullRequestLookup`/`PullRequestBodyUpdate` are synchronous traits; Phase 4's `OctocrabClient` deliberately doesn't implement them, exposing only async inherent methods instead. No bridging shim is scaffolded anywhere in the plan, so it's unclear whether Phase 3's 12 tested characterization branches are ever exercised by production code.

#### Major

- 🟡 **Correctness**: Raw origin-URL parsing may not replicate `gh`'s fork-aware repository resolution
  **Location**: Phase 2: `vcs`/`vcs-adapters` — Origin-Remote-URL Parsing
  The bash script's own comments credit `gh pr view` with fork-aware resolution distinct from a naive local-remote read; the plan's `resolve_origin_owner_repo` has no equivalent mechanism and the gap isn't acknowledged.

- 🟡 **Security**: New Rust credential resolver drops the local-config-file permission fail-closed check jira/linear's bash equivalent enforces
  **Location**: Phase 5: `github.*` Credential Resolver
  `jira-auth.sh`/`linear-auth.sh` refuse to read a token from a `config.local.md` looser than 0600; `FileConfigStore::read` performs no such check, and this is the first Rust code path to resolve and use a live secret end-to-end.

- 🟡 **Security / Architecture**: No timeout, retry, or redirect-host-allowlist configured for the new octocrab/reqwest client
  **Location**: Phase 4: `collaboration-adapters`
  The workspace's one existing outbound-HTTP precedent (`Fetcher`) sets explicit timeouts and a redirect-host-allowlist specifically to stop credential-leaking redirects to look-alike hosts; nothing in Phase 4 configures either for the new client, and the risk is amplified here because the `Authorization` header carries a live GitHub token.

- 🟡 **Test Coverage**: AC's "repointed suites" verification strategy is silently replaced by deletion, unreconciled
  **Location**: Phase 7: Remove the Legacy Bash Scripts and Suites
  The work item's AC and Technical Notes explicitly call for the two bash suites to be repointed with HTTP-level stubbing; Phase 7 deletes them outright with no phase reconciling this deviation against the AC's literal wording.

- 🟡 **Test Coverage**: No test verifies the resolved token is actually wired into outbound octocrab requests
  **Location**: Phase 4 / Phase 5
  Token-resolution precedence and REST-shaped adapter branches are each tested in isolation, but nothing asserts the mock server actually receives the configured token as an `Authorization` header — the AC's "Authentication" criterion is never proven end-to-end.

- 🟡 **Standards**: Two of three rewritten skill call sites won't be recognised by the dispatch-coherence guard's parser
  **Location**: Phase 6, item 4: Skill call-site rewiring
  `review-pr/SKILL.md`'s new invocation would be the second line of a multi-command fenced block, and `describe-pr/SKILL.md`'s would be inline single-backtick text — neither matches the guard's "first line of a fenced block" convention, so only `respond-to-pr`'s binding would actually be guard-visible.

- 🟡 **Code Quality**: `#[cfg(test)]`-gated test constructor won't be visible to the crate's own `tests/` integration suite
  **Location**: Phase 4: `collaboration-adapters`
  `OctocrabClient::with_base_uri` is sketched as `#[cfg(test)]`, but the mock-server tests live in `tests/common/mod.rs`, a separate compilation unit that cannot see `#[cfg(test)]` items — the established precedent (`Fetcher::with_backoff`) is a plain `pub fn` for exactly this reason. As specified, the mock-server tests won't compile.

- 🟡 **Code Quality / Correctness**: `UpdateBodyOutcome::BaseRepoResolutionFailed` makes an illegal state representable
  **Location**: Phase 3: `collaboration` Domain Crate
  The variant wraps the entire `BaseRepoOutcome` enum, including its `Resolved` success arm, so a value like `BaseRepoResolutionFailed(BaseRepoOutcome::Resolved(_))` is constructible though semantically nonsensical.

- 🟡 **Correctness**: Outcome-to-exit-code mapping is incomplete, and a structural mismatch exists for branch 3
  **Location**: Phase 6: `collaboration-cli` — `main.rs`
  The `main.rs` sketch only handles `LookupFailed`/`PatchFailed`; `MalformedResponse`/`EmptyUrl`/`UnexpectedUrlShape` aren't mapped. Separately, update-body's "branch 3" (missing origin remote) is signalled via the outer `kernel::Error`, not a `BaseRepoOutcome` value, so `BaseRepoResolutionFailed`'s payload type structurally can't represent it.

- 🟡 **Documentation**: `skills/github/scripts/README.md` left stale, and the plan's own removal grep would fail on it
  **Location**: Phase 7: Remove the Legacy Bash Scripts and Suites
  The README names `pr-base-repo.sh` in its own prose; Phase 7's changes-required list never mentions it, so the phase's own success-criteria grep (`grep -rn 'pr-base-repo.sh\|pr-update-body.sh' skills/...`) would fail as written.

- 🟡 **Documentation**: Existing user guide's auth prerequisites become incomplete after the migration
  **Location**: Phase 8: User Documentation
  `docs-site/.../guides/review-a-pr.mdx` states `gh auth login` as the auth prerequisite; after this migration, `base-repo`/`update-body` authenticate independently via `github.token`/env vars, which `gh auth login` doesn't populate. Phase 8 adds a new page but doesn't touch this existing guide.

- 🟡 **Compatibility**: `octocrab` dependency version left as an unpinned placeholder
  **Location**: Phase 4: `collaboration-adapters` — Workspace dependency additions
  The `cli/Cargo.toml` snippet leaves `version = "..."` as a literal ellipsis, inconsistent with this workspace's exact-pinning discipline for behaviour-sensitive dependencies (`clap`, `reqwest`, `rustls`, `serde-saphyr`), and risky for a young, actively-evolving crate whose builder API the plan's own tests depend on.

#### Minor

- 🔵 **Architecture**: HTTP status code modelled directly in the domain crate's error type
  **Location**: Phase 3: `collaboration` Domain Crate
  `GitHubApiError::Status { code: u16, ... }` puts a transport-layer concept into the otherwise I/O-free domain crate — likely an acceptable, deliberate exception given the AC's stderr-format requirement, but worth a one-line acknowledgment.

- 🔵 **Security**: `token_cmd` failure-handling stderr/output leakage policy is unspecified
  **Location**: Phase 5: `github.*` Credential Resolver
  The bash precedent deliberately suppresses `token_cmd`'s stderr to avoid leaking vault paths/secret identifiers on failure; the plan doesn't state whether the Rust implementation does the same.

- 🔵 **Security**: Origin-remote URL parser's host-matching strictness is unstated
  **Location**: Phase 2: `vcs`/`vcs-adapters`
  Whether `parse_github_remote_url` validates the host component strictly (exact `github.com` match) versus a looser match isn't specified — relevant given a git remote URL is attacker-influenceable via a cloned malicious repository.

- 🔵 **Standards**: Cargo `[package] name` not called out alongside `[[bin]] name`
  **Location**: Phase 6, item 1: Crate scaffold and registration
  The checklist requires the package name to be `accelerator-<token>`, matching `vcs-cli`/`work-cli`; the plan's Cargo.toml snippet only shows `[[bin]] name`.

- 🔵 **Correctness**: Distinguishing "no origin remote" from "probe failure" by exit code/stderr shape is fragile
  **Location**: Phase 2: `vcs`/`vcs-adapters` — Subprocess adapter
  `run_capped` as it exists today folds all failures to `None` without exposing exit code or stderr text; a new/extended helper is needed but isn't identified, and stderr-text matching is locale-sensitive without a stated `LANG=C` fix (a bug class this codebase has hit before with the Playwright launcher).

- 🔵 **Compatibility**: Potential duplicate reqwest/rustls stack from octocrab's own transitive pin
  **Location**: Phase 4: `collaboration-adapters` — Workspace dependency additions
  `octocrab` bundles its own `reqwest` dependency; the plan doesn't address whether its version range is satisfiable against the workspace's exact `reqwest` pin, risking a silently duplicated TLS/HTTP stack.

- 🔵 **Test Coverage**: No stated automated test for `main.rs`'s full composition or its error-to-stderr rendering
  **Location**: Phase 6: `collaboration-cli` Sub-binary
  The composition glue (config → adapter → auth → `block_on` → client) and the `"{code}: {message}"` stderr format are covered only by manual verification steps.

- 🔵 **Test Coverage**: `resolve_origin_owner_repo`'s "unsupported URL shape" branch isn't explicitly named as its own test case
  **Location**: Phase 2, Success Criteria
  Only "no origin configured" and "probe failure" are explicitly enumerated for the composed function; the unsupported-shape composed path isn't named separately from the underlying parser's own test.

- 🔵 **Documentation**: `visualiser.md` may be the wrong structural precedent for the new docs page
  **Location**: Phase 8: User Documentation
  `collaboration` is closer in nature to `corpus` (skill-invoked plumbing, manual invocation secondary) than to `visualiser` (a rich, directly-invoked feature); `corpus.md`'s subcommand-table shape may fit better.

- 🔵 **Documentation**: Adjacent error-handling bullets risk conflating two different "no remote" concepts
  **Location**: Phase 6: Skill call-site rewiring
  `review-pr/SKILL.md`'s unmodified `gh repo set-default` bullet sits directly above the rewritten "no origin remote configured" bullet, without disambiguating the two different mechanisms.

#### Suggestions

- 🔵 **Architecture**: A second hand-rolled TCP mock server duplicates `cli/launcher`'s pattern — worth a follow-up note once a third consumer appears.
- 🔵 **Code Quality**: `report()`'s exit-code-mapping helper is now duplicated verbatim a third time; consider factoring into `kernel` on this addition rather than a fourth future copy.
- 🔵 **Code Quality**: Derive requirements (`Debug`/`Clone`/`PartialEq`) for the new outcome/error types are unstated but assumed by the characterization tests.
- 🔵 **Test Coverage**: The ported mock-server test harness is a second, independently-drifting copy of `cli/launcher`'s; track a follow-up to extract a shared crate once a third consumer appears.
- 🔵 **Security**: No stated guard against verbose `octocrab`/`reqwest` tracing leaking the `Authorization` header at debug/trace log levels.
- 🔵 **Compatibility**: The error-mapping sketch assumes a specific `octocrab::Error` shape that hasn't been verified against a pinned version.
- 🔵 **Documentation**: `docs-site/.../skills/vcs-and-pr.md`'s blanket "wrap the GitHub CLI" claim becomes imprecise once part of the workflow bypasses `gh` entirely.

### Strengths

- ✅ The bottom-up phase ordering (config catalogue → `vcs` port → domain composition against fakes → adapters against a mock server → credential resolver → CLI wiring → deletion → docs) is a textbook TDD-friendly sequence that keeps every phase independently mergeable and `mise run`-green.
- ✅ The three-crate split mirrors the proven `vcs`/`work` structural pattern closely, including keeping I/O concrete-typed only in the adapters crate and using hand-written fakes rather than a mocking framework, consistent with house style.
- ✅ Convention deviations are explicitly documented with rationale rather than left implicit — e.g. `OriginRemote`'s fallible-rather-than-fold-to-`None` contract, and the config-first vs. jira/linear's env-first credential precedence order.
- ✅ The `octocrab`/`deny.toml`/native-tls risk is surfaced and gated with an explicit `mise run deny:check` step immediately after the dependency addition, rather than deferred.
- ✅ Explicitly carries over the jira/linear shared-config `token_cmd` ban, closing a real command-injection vector.
- ✅ The `base-repo` subcommand's exact `owner/repo\n` stdout contract is identified as a real external contract and Phase 6 explicitly commits to preserving it.
- ✅ The plan verifies its own claim of satisfying the sub-binary registration checklist item-by-item against the actual checklist text rather than asserting it wholesale, and correctly bundles the "must land together" items into Phase 6.
- ✅ The "What We're NOT Doing" section shows good scope discipline — explicitly declining a general-purpose GitHub client, a new shared mock-server crate, and unrelated `vcs` fixture-machinery retrofits.
- ✅ Phase 8 fully satisfies the sub-binary registration checklist's documentation item (docs-site page, sidebar registration, README entry, env-var override row).

### Recommended Changes

1. **Redesign base-repo resolution around `base.repo.full_name`, not the top-level `url` field** (addresses: Compatibility/Correctness critical finding, Correctness's fork-resolution major finding). Either explicitly scope out fork support (state that `origin` is assumed to already be the upstream repo, and drop "cross-fork-safe" framing from Requirements/docs), or parse the response's `base.repo.owner.login`/`base.repo.name` fields, which actually carry independent information. Redesign branches 4-6's validation logic against whichever field is actually used.

2. **Scaffold the sync-to-async bridge explicitly, or restructure the domain functions to be async** (addresses: Architecture/Code Quality critical finding). Either add a thin synchronous adapter in `collaboration-cli` that implements the Phase 3 ports by `block_on`-ing `OctocrabClient`'s async methods, or make `resolve_base_repository`/`update_pull_request_body` themselves async and move the `block_on` boundary to `main.rs` alone.

3. **Drop `#[cfg(test)]` from `OctocrabClient::with_base_uri`** (addresses: Code Quality major finding), matching `Fetcher::with_backoff`'s precedent, so the crate's `tests/` integration suite can actually construct it.

4. **Either implement or explicitly waive the local-config-file permission check for `github.token`** (addresses: Security major finding). State the decision either way rather than leaving it as a silent gap relative to jira/linear.

5. **Configure explicit timeouts and a redirect-host-allowlist on the `octocrab`/`reqwest` client** (addresses: Security/Architecture major finding), mirroring `Fetcher`'s pattern, given the `Authorization` header's exposure risk on an unrestricted redirect.

6. **Reconcile Phase 7's suite deletion against the AC's "repointed suites" wording** (addresses: Test Coverage major finding) — either justify the deviation explicitly (cross-checking the 38 bash cases against the 16 new Rust tests) or add a repointing phase before deletion.

7. **Add an `Authorization`-header assertion to the Phase 4 mock-server tests** (addresses: Test Coverage major finding) so credential resolution and HTTP dispatch are proven to compose, not just each tested in isolation.

8. **Restructure the two non-compliant skill call sites into independently guard-visible fenced blocks** (addresses: Standards major finding) — split `review-pr/SKILL.md`'s two-line block and convert `describe-pr/SKILL.md`'s inline invocation into its own fenced block.

9. **Narrow `UpdateBodyOutcome::BaseRepoResolutionFailed`'s payload type** (addresses: Code Quality/Correctness major finding) to only the genuine failure arms, and write out the full `main.rs` match coverage for every outcome variant including the outer `kernel::Error` case (addresses: Correctness major finding on incomplete exit-code mapping).

10. **Add `skills/github/scripts/README.md` to Phase 7's changes-required list, and add a Phase 8 update to `review-a-pr.mdx`'s prerequisites** (addresses: both Documentation major findings).

11. **Pin `octocrab` to an exact version with a rationale comment** (addresses: Compatibility major finding), matching this workspace's pinning convention for behaviour-sensitive dependencies.

## Per-Lens Results

### Architecture

**Summary**: The plan follows the established three-crate sub-binary pattern (domain → adapters → thin CLI) faithfully and sequences phases so each layer is proven with fakes before the next depends on it — a genuinely sound incremental delivery strategy. The most significant structural risk is that the domain crate's synchronous ports and the composition functions built against them in Phase 3 appear structurally incompatible with the async-only adapter the plan commits to in Phase 4, creating a real risk that production `collaboration-cli` never actually exercises the tested domain logic. Secondary concerns are the lack of any documented timeout/retry/backoff strategy for the new external GitHub REST dependency and a couple of smaller leaky-abstraction/duplication points.

**Strengths**:
- The phase ordering is a textbook bottom-up, TDD-friendly sequence keeping every phase independently mergeable and `mise run`-green.
- Async/`tokio` is deliberately confined to `collaboration-cli`'s `main()`, keeping the domain crate free of concrete I/O and async types.
- The plan is explicit about where it diverges from existing conventions, stating rationale inline.
- The `octocrab`/`deny.toml`/native-tls risk is surfaced and gated with an explicit `mise run deny:check` step immediately after the dependency addition.

**Findings**: See Critical ("Sync domain ports and the async-only adapter are not shown to actually connect"), Major ("No documented timeout, retry, or backoff strategy"), Minor ("HTTP status code modelled directly in the domain crate's error type"), Suggestion ("A second hand-rolled TCP mock server duplicates cli/launcher's pattern") above.

### Code Quality

**Summary**: The plan follows the codebase's established ports-and-adapters shape closely and is generally well-organised and proportionate to the problem. However, two concrete technical gaps undermine its own design goals: the bridge between the domain crate's composition functions and the async-only adapter is never specified, and the test-only constructor pattern for the mock-server adapter tests doesn't match how this workspace exposes test-injection points to `tests/` integration suites. A smaller design smell (an illegal state representable in `UpdateBodyOutcome`) is also present.

**Strengths**:
- The three-crate split mirrors the proven `vcs`/`work` structural pattern closely.
- Convention deviations are explicitly documented with rationale rather than left implicit.
- `GitHubApiError` models `Transport`/`Status` as distinct variants rather than a generic catch-all string.
- The "What We're NOT Doing" section shows good YAGNI discipline.
- The bottom-up phase sequencing proves each layer in isolation before the layer above depends on it.

**Findings**: See Critical/Major ("Sync domain ports..."), Major ("`#[cfg(test)]`-gated test constructor won't be visible..."), Major ("`UpdateBodyOutcome::BaseRepoResolutionFailed` makes an illegal state representable"), Suggestion ("`report()`'s exit-code-mapping helper is duplicated verbatim a third time"), Minor ("Derive requirements for the new outcome/error types are unstated") above.

### Test Coverage

**Summary**: The plan's bottom-up, fakes-before-adapters testing strategy is well-structured and traces its 16-characterization-test target directly to the work item's AC. However, it silently drops the AC's explicit "repointed suites" requirement by deleting rather than repointing the two bash test suites, and leaves the token-resolution-to-HTTP-request wiring and the CLI-layer error-rendering/composition path without a clearly stated automated test.

**Strengths**:
- The phase-by-phase approach proves each layer's logic against hand-written fakes before the layer above depends on it.
- Explicitly maps its 16 planned tests back to the work item's AC.
- Consistent use of the codebase's existing test idioms rather than a new mocking framework.
- TDD (red-green) is called out explicitly as required for every phase's Success Criteria.

**Findings**: See Major ("Bash test suites are deleted, not repointed"), Major ("No test verifies the resolved token is actually wired into outbound octocrab requests"), Minor ("No stated automated test for main.rs's full composition"), Minor ("resolve_origin_owner_repo's 'unsupported URL shape' branch isn't explicitly named"), Suggestion ("Mock server test harness is duplicated") above.

### Correctness

**Summary**: The plan's bottom-up, TDD-per-layer structure is logically sound for the parts it gets right, but the central resolver design — deriving the PR's "base (upstream) owner/repo" from the REST response's top-level `url` field — is circular against the real GitHub REST API, so it cannot deliver the cross-fork-safety property the plan explicitly claims to preserve. A second, related gap is that swapping `gh`'s implicit repository inference for a raw `origin`-remote-URL parse silently drops whatever fork-parent resolution `gh` performs. Several secondary issues (incomplete branch-to-exit-code mapping, an illegal-state type, fragile locale-sensitive stderr matching) round out the concerns.

**Strengths**:
- The bottom-up phase ordering proves each layer against fakes before the layer above depends on it.
- The plan is honest about what isn't reusable rather than assuming false equivalence.
- The 12+4 characterization-test checklist gives a concrete, enumerable target for domain-level branch coverage.

**Findings**: See Critical ("Deriving the base owner/repo from the REST response's url field is circular"), Major ("Raw origin-URL parsing may not replicate gh's fork-aware repository resolution"), Major ("Outcome-to-exit-code mapping is incomplete"), Minor ("Distinguishing 'no origin remote' from 'probe failure'... is fragile"), Minor ("BaseRepoResolutionFailed(BaseRepoOutcome) can represent an invalid state") above.

### Security

**Summary**: The plan is a well-scoped, defence-conscious migration that correctly treats the GitHub REST response as untrusted input, preserves the jira/linear shared-config token_cmd-injection ban, and pins TLS to rustls. However, it is also the first Rust code path in this codebase to resolve and use a live secret end-to-end, and silently drops two protections the bash precedent it mirrors provides: the local-config-file permission fail-closed check, and (by not configuring the client) the timeout/redirect-host-allowlist hardening the workspace's other outbound HTTP client treats as mandatory.

**Strengths**:
- Explicitly carries over the jira/linear shared-config `token_cmd` ban.
- Pins `octocrab` to `rustls-ring`, avoiding the `opentls`/`native-tls` default `cli/deny.toml` bans, with an immediate `deny:check` gate.
- Domain-level `BaseRepoOutcome` treats the GitHub API response as an untrusted boundary.
- Keeps the `octocrab` `BaseUriLayer` test seam `#[cfg(test)]`-gated.
- Explicitly scopes out token-content shell-hostile-character validation with a sound, stated rationale.

**Findings**: See Major ("New Rust credential resolver drops the local-config-file permission fail-closed check"), Major/Architecture ("No timeout or redirect-host-allowlist configured"), Minor ("token_cmd failure-handling policy... is unspecified"), Minor ("Origin-remote URL parser's host-matching strictness is unstated"), Suggestion ("No stated guard against verbose octocrab/reqwest tracing leaking the Authorization header") above.

### Standards

**Summary**: The plan is unusually rigorous about tracing itself against the sub-binary registration checklist, correctly bundling the "must land together" items into Phase 6 and citing exact line numbers throughout. Structural conventions are followed closely and verified against real precedent. The one substantive gap is that the plan's literal skill call-site edits place two of the three new invocations in a form the dispatch-coherence guard's parser cannot see.

**Strengths**:
- Verifies its own claim of satisfying the sub-binary registration checklist item-by-item against the actual checklist text.
- Correctly honours the checklist's "items 1, 2, 3, 4, 7, 8 must land in the same change" constraint.
- `main.rs`'s `report()` function and naming are lifted near-verbatim from `vcs-cli`'s established pattern.
- Explicitly preserves the exact `owner/repo\n` stdout contract and flags the deliberate config-first precedence departure.

**Findings**: See Major ("Two of the three rewritten skill call sites won't be recognised by the dispatch-coherence guard's parser"), Minor ("Cargo `[package] name` not called out alongside `[[bin]] name`") above.

### Compatibility

**Summary**: The plan is careful about the contracts it explicitly calls out — stdout format, env-var fallback order, TLS feature selection — all reasoned through correctly against real consumers and `deny.toml`. However, the core design for resolving a PR's base repository appears to misread the semantics of the GitHub REST API field it relies on, risking the cross-fork-safety guarantee the source bash script was hardened to provide. A couple of dependency-pinning gaps are lower-severity but worth tightening given this workspace's otherwise disciplined exact-pinning convention.

**Strengths**:
- TLS feature selection explicitly reasoned against `deny.toml`'s native-tls ban and the existing `reqwest`/`rustls` pin.
- The base-repo subcommand's `owner/repo\n` stdout contract is identified as a real external contract and its preservation explicitly committed to.
- The `GH_TOKEN`-before-`GITHUB_TOKEN` fallback order is verified against `gh`'s own documented precedence.
- The exit-code contract change lands in the same phase as the SKILL.md updates that document it.
- The config-first precedence departure from jira/linear is called out explicitly as deliberate.

**Findings**: See Critical ("Base-repo resolution derives owner/repo from a REST field that cannot carry that information"), Major ("octocrab dependency version left as an unpinned placeholder"), Minor ("Potential duplicate reqwest/rustls versions"), Suggestion ("Error-mapping code sketch assumes a specific octocrab::Error shape") above.

### Documentation

**Summary**: The plan documents its own primary new surface well — Phase 8 explicitly satisfies the registration checklist's documentation item, and code snippets carry purposeful rustdoc explaining non-obvious contract distinctions. However, the plan is incomplete on documentation that already exists and references the code being deleted or restructured: it never touches `skills/github/scripts/README.md` (which describes a script the plan deletes) nor `review-a-pr.mdx` (whose stated auth prerequisites become incomplete).

**Strengths**:
- Phase 8 fully satisfies the sub-binary registration checklist's documentation item.
- Proposed Rust code carries purposeful rustdoc explaining the "why" behind non-obvious API choices.
- Phase 6's skill call-site changes specify exact before/after text, removing ambiguity for the implementer.

**Findings**: See Major ("skills/github/scripts/README.md left stale"), Major ("Existing user guide's auth prerequisites become incomplete"), Suggestion ("visualiser.md may be the wrong structural precedent"), Suggestion ("vcs-and-pr.md's blanket 'wrap the GitHub CLI' claim becomes imprecise"), Minor ("Adjacent error-handling bullets risk conflating two different 'no remote' concepts") above.

---

## Re-Review (Pass 2) — 2026-08-08T21:49:24+00:00

**Verdict:** APPROVE (updated from the suggested COMMENT verdict — author
override, approving the plan for implementation as-is; the remaining open
minor/suggestion items below are accepted as follow-up work, not
implementation blockers)

All 8 lenses were re-run (every lens had findings in Pass 1). Both critical findings and all 10 major findings from Pass 1 were independently confirmed resolved by the lenses that raised them — the base-repo resolution redesign (two-step: repository-metadata parent-check, then PR-existence confirmation) and the `BlockingGitHubClient` sync/async bridging shim held up under fresh, adversarial scrutiny from every angle each lens checked. One finding (the personal-config-file permission check) was resolved for its original scope but reopened as **Partially resolved** when three lenses (security, documentation, compatibility) independently converged on the same gap from different directions — addressed in this pass (see below).

### Previously Identified Issues

- 🟢 Base-repo resolution circularity (Compatibility + Correctness, critical) — Resolved
- 🟢 Sync/async boundary bridge (Architecture + Code Quality, critical/major) — Resolved
- 🟢 Fork-aware origin resolution (Correctness, major) — Resolved
- 🟡 Dropped local-config-file permission check (Security, major) — Partially resolved → now fully resolved (see New Issues)
- 🟢 No timeout/redirect protection on octocrab client (Security + Architecture, major) — Resolved
- 🟢 Bash suites deleted not repointed, AC mismatch (Test Coverage, major) — Resolved
- 🟢 No test proves token reaches outbound request (Test Coverage, major) — Resolved
- 🟢 Dispatch-coherence guard visibility (Standards, major) — Resolved
- 🟢 `#[cfg(test)]` visibility bug (Code Quality, major) — Resolved
- 🟢 Illegal state in `BaseRepoResolutionFailed` (Code Quality + Correctness, major) — Resolved
- 🟢 Stale README + stale user guide (Documentation, major ×2) — Resolved
- 🟢 `octocrab` version unpinned (Compatibility, major) — Resolved

All older minor/suggestion-level findings not addressed in the first fix pass (HTTP status modelled in the domain error type, `report()` duplication, unstated derives, the vcs "unsupported URL shape" composed-branch test, locale-sensitive stderr matching in the subprocess adapter, `token_cmd` stderr-leakage policy, origin-URL host-matching strictness, tracing/`Authorization`-header redaction, `visualiser.md` vs `corpus.md` docs precedent, `vcs-and-pr.md`'s imprecise claim) remain open at the same low priority — none were raised again as blocking by this pass.

### New Issues Introduced (this pass) — all now addressed

- 🔴 **Correctness**: `BaseRepoFailure::MalformedRepositoryResponse` (branch 4) was unreachable — `GitHubApiError` only had `Transport`/`Status`, and deserialization failures folded into `Transport`. **Fixed**: added a distinct `GitHubApiError::Malformed` variant, wired from octocrab's `Serde`/`Json` error variants specifically, with `MalformedRepositoryResponse` now carrying the message.
- 🔴 **Correctness**: the library (`gix`) `OriginRemote` adapter had no stated Err-vs-`Ok(None)` contract, risking silent copy of `git_user_name`'s fold-to-`None` convention and divergence from the subprocess adapter. **Fixed**: explicit note added, plus a new cross-backend test requirement (broken-repo fixture must yield `Err`, not `Ok(None)`, for both adapters).
- 🔴 **Test Coverage**: Phase 3's "all 13 domain-level branches covered" claim contradicted its own text deferring 3 branches to Phase 6; the missing/unreadable `--body-file` branch had no test anywhere. **Fixed**: corrected to "10 of 13," with the 3 CLI-layer branches (including the previously-untested `--body-file` case) now named explicitly in Phase 6's Success Criteria.
- 🔴 **Test Coverage**: no automated test covered `main.rs`'s error-rendering, the `owner/repo` stdout contract, or the `BlockingGitHubClient` shim — only manual verification. **Fixed**: added an explicit Phase 6 end-to-end test requirement against the Phase 4 mock server.
- 🟡 **Security + Documentation + Compatibility** (three-lens convergence): the Phase 1 permission check only covered `ReadConfigLevel::read`, leaving `ReadContent::config_body` (already used to inject personal-config prose into skill context) as an unpatched bypass; had no user-facing documentation despite being a workspace-wide behavioural change; and had no migration/blast-radius framing for existing non-compliant `config.local.md` files. **Fixed**: the check is now a shared helper both read paths call; a Migration Notes callout and two docs-site updates (`configuration.md`, `configuration-cookbook.md`) were added, framing this explicitly as an accepted, documented breaking change (remediation: `chmod 600`).
- 🔵 Minor: `GitHubApiError::Transport` rendering was ambiguous (no `code` field, but the stated format assumed one for all variants). **Fixed**: per-variant rendering now specified.
- 🔵 Minor (carried over from Pass 1, not newly introduced): Cargo `[package] name` still not called out alongside `[[bin]] name`. **Fixed**: added with rationale.
- 🔵 Minor: Phase 1's `ConfigError` file citation was wrong (`service.rs` vs the actual `error.rs`). **Fixed**, plus the `is_refusal()` classification decision now noted.
- 🔵 Minor: dropping `follow-redirect` diverges from `gh`'s handling of a renamed/transferred upstream repository — untested edge case. **Accepted as a documented trade-off**, not fixed: a doc-comment note now explains the behavioural regression and its remediation (update the stale `origin` remote) explicitly, rather than leaving it implicit.
- 🔵 Minor: `cli/deny.toml`'s `multiple-versions` policy is `"warn"`, not `"deny"`, so the plan's stated `deny:check` mitigation for a duplicate `reqwest`/`rustls` stack is weaker than implied. **Accepted as a documented trade-off**: a note now states this explicitly and recommends `cargo tree` as the reliable confirmation instead of relying on the exit code alone.

### Assessment

The plan is now in good shape. Every critical and major finding raised across two full review passes (8 lenses × 2 rounds) has either been resolved with a verifiable design change or explicitly accepted and documented as a deliberate trade-off — none remain silent. What's left is a set of low-priority minor/suggestion items (mostly documentation polish and small hardening opportunities) that don't block implementation and can reasonably be addressed during implementation or as follow-up work rather than requiring a third review pass. Recommend proceeding to implementation.

---
*Review generated by /accelerator:review-plan*
