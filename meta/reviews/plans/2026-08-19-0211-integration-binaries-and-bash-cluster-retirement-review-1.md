---
type: "plan-review"
id: "2026-08-19-0211-integration-binaries-and-bash-cluster-retirement-review-1"
title: "Plan Review: Integration Binaries and Bash Cluster Retirement"
date: "2026-08-19T02:16:22+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
parent: "work-item:0211"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "safety", "compatibility", "security"]
review_number: 1
review_pass: 4
tags: ["rust", "jira", "linear", "integrations", "cli", "cutover", "exit-codes", "registration"]
last_updated: "2026-08-19T09:35:03+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Integration Binaries and Bash Cluster Retirement

**Verdict:** REVISE

The plan is unusually rigorous — the capture-oracle-before-deletion sequencing, the two-track shared-floor reasoning, the same-commit dispatch-coherence rule, and the byte-exact-golden discipline are all sound, and reviewers found no critical defect that undermines the shape. But sixteen major findings cluster on three mechanisms whose specifications are self-defeating or incomplete as written: the exit-code parity model, the base-URL env seam, and the repointed skill bodies. Each needs a concrete design decision recorded in the plan before implementation, because each is the kind of gap that would surface only after the bash source is gone.

### Cross-Cutting Themes

- **The exit-code enforcement model is under-specified and, as written, self-contradictory** (flagged by: correctness, test-coverage, code-quality, safety) — the single largest cluster. The textual-equality parity test contradicts the deliberate 70–73 remap and 81/82/34 divergences (an equality assertion fails for exactly the names the plan intends to diverge); aggregating `readonly E_*=NN` across ~10–17 flow scripts plus behavioural runs into one fixture has no conflict-resolution rule for a name holding two integers; the parity test verifies constant *declarations*, not the runtime variant→code *routing* the skills branch on; and the two `main.rs` house shapes do not compose — `kernel::Error` collapses the variant information needed for ~45 codes, while port-op integers survive only as substrings inside `TrackerError.detail`.
- **The base-URL env seam is both a security hole and functionally incomplete** (flagged by: security, correctness) — read unconditionally in release `main.rs`, it lets an env var redirect the authenticated token past Jira's `*.atlassian.net` destination allowlist (SSRF/exfiltration); and even on the happy path it overrides only the GraphQL transport, leaving Linear's upload path (`UploadTransport::production()`, loopback-refusing) and the `team_key`/`states` reconstruction pointed at production — so the attach and transition mock goldens are uncapturable through the seam as described.
- **Repointed skill bodies carry contracts the crate test nets do not cover** (flagged by: compatibility, safety, test-coverage, correctness) — in-body exit-code tables (`search-jira-issues` Step 3, `create-jira-issue` WF-1) still cite pre-remap integers; the confirm-before-mutate gate is downgraded to SKILL.md prose plus a manual checkbox; stderr audit lines (`INFO: composed JQL`) and search JSON envelopes sit outside stdout goldens; and the witness invocation must be metacharacter-free, yet these are the skills that today pipe to `jq`.

### Tradeoff Analysis

- **Contract fidelity vs re-opening 0210**: Decision 2 declines the mutation-payload compose seam to avoid re-opening the client crates, accepting a preview/execution derivation gap. Defensible, but the reviewers converge on one condition — the divergence must be pinned by a *named, real, automated* test (the ledger's own governing rule), not by a manual checkbox. Either name the test or reclassify the row as manual-only so the ledger is honest.
- **Uniform taxonomy vs per-provider contract shape**: the plan imposes the full `exit_codes.rs` integer taxonomy + parity harness on both binaries, but Linear's skills key on symbolic `E_*` strings with exactly one meaningful integer (`107`). Anchoring Linear parity over the `E_*` names its `Display` impl already emits (plus `107`) would remove unread machinery; Jira genuinely needs the integer taxonomy.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness**: Base-URL seam overrides only the GraphQL transport, leaving the upload path pointed at production
  **Location**: Phase 1 item 2 (base-URL seam) & item 4 (attach flow)
  `from_config` also builds `UploadTransport::production()` (hardcoded `allow_loopback = false`, refuses hosts outside `*.linear.app`). A mock-backed attach test whose `fileUpload` response nominates a loopback URL is refused unless the seam also builds `UploadTransport::new(true, …)`. The attach golden is uncapturable through the seam as written.

- 🟡 **Correctness**: Textual equality parity contradicts the deliberate 70–73 remap and cross-provider divergences
  **Location**: Phase 1 item 3 / Phase 3 item 3 / Decision 6
  `exit_codes_parity.rs` asserts equality against the captured fixture, yet `E_SEARCH_*` (bash 70–73) is deliberately remapped and 81/82/34 restated per provider. Equality fails for exactly the diverging names — or the fixture is edited to hold remapped values and no longer records bash reality. The two are mutually exclusive as specified. Needs an explicit, count-pinned divergence allowlist.

- 🟡 **Correctness**: Aggregating `readonly E_*=NN` across many flow scripts plus behavioural runs has no conflict rule
  **Location**: Phase 1 item 3 / Phase 3 item 1
  work-cli parses a *single* file, so names are unique. Grepping ~10–17 flows plus recording behavioural codes into one fixture allows the same `E_*` name to hold different integers (the plan itself cites `jira-request.sh:207` exiting 1 where the doc says 2). No de-dup or precedence rule → contradictory rows or a masked collision.

- 🟡 **Correctness**: Witness invocation must be metacharacter-free, but the repointed read skills previously piped to `jq`
  **Location**: Phase 2 item 2 / Phase 4 item 2
  `dispatch_coherence.py:_bindings` counts a binding only when `not has_metacharacter(command)`. A repointed `accelerator linear search … | …` records the token as invoked-but-unbound → coherence fails at the exact same-commit merge boundary the plan relies on. The witness skill must invoke the token in a fenced step with no pipe/redirect.

- 🔴 **Security**: Base-URL env seam bypasses the credential-destination allowlist and redirects the authenticated token
  **Location**: Phase 1 item 2 / Phase 3 item 1
  `ACCELERATOR_{JIRA,LINEAR}_API_URL` is read unconditionally in `main.rs` (no `cfg(test)`/`debug_assertions` gate), so it ships in release. For Jira it deliberately bypasses `auth.rs::base_url` (the `*.atlassian.net` control); Linear accepts any `Url` including plain `http://`. Anyone influencing the process environment can exfiltrate the live token. Gate to test builds, or revalidate the override as a credential destination.

- 🔴 **Compatibility**: Search 70–73 remap not reconciled with the exit-code tables baked into repointed skill bodies
  **Location**: Phase 3 item 3 / Phase 4 item 2
  `search-jira-issues` Step 3 branches on `Exit 72`/`Exit 71`; `create-jira-issue` Step 10 + WF-1 hardcode a full code→message table. Phase 4 repoints bodies generically without enumerating that these in-body tables must be rewritten to the new taxonomy in the same commit. A stale table hands the user wrong recovery guidance for the exact failure the remap moved.

- 🟡 **Compatibility**: Stderr audit-line contracts (composed JQL / IssueFilter `INFO:` lines) fall outside the preservation nets
  **Location**: Testing Strategy / Phase 1 item 4 / Phase 3 subcommands
  Both search bodies surface an `INFO: composed JQL: …` stderr line for auditability; `--quiet` suppresses it. The nets are stdout goldens + exit-code parity — neither asserts stderr, and `--quiet` is not listed among reproduced flags. Add a stderr golden and list `--quiet`.

- 🟡 **Compatibility**: Search JSON-envelope shape the bodies parse must match the client's projection, not just a mock golden
  **Location**: Phase 1 item 4 / Phase 3 item 2
  Bodies read `.data.issues.nodes[]` + the merged-pages `.data.issues.truncated` (Linear) and `issues[]` + `nextPageToken` (Jira). `truncated` is a bash-flow construct, not necessarily what the client emits. If the client projection differs, the bash-captured golden won't match the binary and the render/pagination instructions break.

- 🔴 **Code Quality**: The two house shapes do not compose — rich taxonomy grafted onto a collapsing `kernel::Error` funnel
  **Location**: Phase 1 item 1 (main.rs shape) vs item 3 (taxonomy)
  Item 1 wants `Result<Outcome, kernel::Error>` + a `report` mapping the taxonomy; item 3 wants ~45 integers from `SurfaceError`/`TrackerError` variants. But `kernel::Error` only distinguishes `Refusal`/`Failed`, so funnelling through it erases the variant info. work-cli achieves its taxonomy by *not* funnelling — handlers return `ExitCode` inline. Commit to one shape (work-cli's).

- 🟡 **Code Quality**: Port-op exit integers are recoverable only by parsing them out of `TrackerError.detail`
  **Location**: Phase 3 item 1 / Current State Analysis
  `TrackerError` exposes only `Retryable`/`Terminal`; the numeric code is `format!`-ed *into* `detail` by `classify.rs::build`. Recovering the specific integer means substring-parsing `(NN)` back out — stringly-typed, no compile-time link to `classify.rs`, silently drifts if the format changes. Either record the two-code collapse as a tested divergence, or expose the code structurally.

- 🔴 **Test Coverage**: Exit-code parity test verifies constant declarations, not the runtime variant→code routing
  **Location**: Phase 1 item 3 / Phase 3 items 1 & 3
  The textual `pub const : u8` parse proves the constants hold the right integers, not that each error variant maps onto the correct constant at runtime — the load-bearing logic for the ~45 Jira codes. A mis-routed variant leaves the parity test green. Require a behavioural test per error class that drives the binary into each condition and asserts the observed exit code.

- 🟡 **Test Coverage**: Preview-resolved-intent divergence has no concrete automated pinning test
  **Location**: Decision 2 / Phase 2 item 2 / Phase 4 Manual Verification
  Decision 2 records the divergence "with a pinning test", and the ledger rule is "every row names a real, passing test" — but the preview/confirm gate lives in the repointed `SKILL.md` body, which has no Rust-suite harness, and the only stated check is a manual checkbox. Name the concrete automated test or reclassify the row as manual-only.

- 🟡 **Test Coverage**: Captured exit-code fixture is a self-referential oracle whose completeness rests on author-recorded capture
  **Location**: Phase 1 item 3 / Phase 3 item 1
  Unlike work-cli (which parses live bash at test time), this captures into a committed fixture and deletes the bash. Post-deletion the parity test only guards the Rust side against a frozen snapshot; the behavioural half is recorded by hand-chosen fixtures, so an incomplete-but-self-consistent capture passes vacuously. Make the capture scripted/exhaustive and derive the count from its output.

- 🟡 **Safety**: Remote-mutation confirm gate relies on SKILL.md prose plus a manual checkbox after downgrade to intent-preview
  **Location**: Decisions (2), Phase 2 item 2 / Phase 4 item 2
  The write flows perform not-trivially-reversible remote mutations; Decision 2 downgrades the preview and relocates the confirm step into body prose with the binary executing "atomically after confirm". The only enforcement is a manual checkbox — no automated guard that confirm precedes the mutation invocation. Add a skills-lane assertion (as 0213 did) that the confirm step lexically precedes the mutation.

- 🟡 **Safety**: Empty-stdout failure gates redesigned onto exit codes guard a local writeback and duplicate-create avoidance — must fail closed
  **Location**: Phase 4 item 2 (`create-jira-issue:113/:183`, `:65-66`), Decision 5
  `create-jira-issue` writes the returned key back into tracked frontmatter; its empty-stdout gate prevents that on a failed create, and Decision 5's exit-16 stops a retry double-creating. If the exit-code gating doesn't fail closed, a failed create can write an empty/stale key or trigger a duplicate. Add explicit tests that non-zero/exit-16 suppresses the writeback and blocks retry.

- 🟡 **Safety**: Behavioural exit-code oracle is a one-shot manual capture with weaker provenance than the declared half
  **Location**: Phase 1 item 3 / Phase 3 item 1, Decision 6/8
  The behavioural codes are recorded by manually running flows against error fixtures; once ~17,650 lines of bash are gone, the fixture is the sole oracle and both sides are authored artefacts. Weaker than 0210's D10 differential tests that *executed* the bash. This taxonomy prevents duplicate remote mutations, so a mis-capture is a durable silent hazard. Gate the capture behind a committed differential test while the bash exists.

#### Minor

- 🔵 **Correctness**: Override branch omits `team_key`/`states` reconstruction that `from_config` provides
  **Location**: Phase 1 item 2
  `LinearClient::new(transport, upload, team_key, states)` needs four args; `from_config` derives `team_key`/`states` from `integrations_root`. The seam branch is silent on these; the `transition` flow depends on a populated state catalogue. Specify that the branch reconstructs them identically, differing only in endpoint.

- 🔵 **Correctness**: Malformed override env var silently falls back to the production API
  **Location**: Phase 1 item 2 (`api_base_uri`)
  `Url::parse(…).ok()` returns `None` on a set-but-unparseable value, so a typo'd override silently hits real Linear/Jira. Treat present-but-unparseable as a hard error rather than falling through to `from_config`.

- 🔵 **Security**: Single assertion is thin coverage for the "init verify never prints the token" guarantee
  **Location**: Decision 3 / Phase 1 item 4 / Phase 3 Manual Verification
  `init verify` has many exit paths (success, each error variant, stringified transport errors); one happy-path assertion is thin. Seed a sentinel token, drive every path, assert it appears on neither stream, and record the `Secret`-redaction invariant as the reason the guarantee holds.

- 🔵 **Compatibility**: Dropped `--print-payload` leaves a stale advertised flag in the create skills' `argument-hint`
  **Location**: Decision 2 / Phase 2 item 2 / Phase 4 item 2
  `create-linear-issue` advertises `[--print-payload]` in `argument-hint`; the flag no longer exists on the binary. A user typing it hits an unrecognised-flag error. Drop it from the frontmatter in the same repoint commit.

- 🔵 **Compatibility**: Init-written cache-file formats not verified compatible with pre-existing bash-written state
  **Location**: Phase 1/3 subcommand surface: `init verify | refresh-fields`
  `init` subsumes `site.json`/field-cache production; the plan doesn't state whether the binary reads the existing bash-written format/paths. A user who ran the bash `init` before upgrading may have caches the binary can't consume. Confirm compatibility or record cache re-init as a required post-upgrade step.

- 🔵 **Test Coverage**: `last_body` retains only the final request per key, limiting mutation assertions on multi-POST Linear flows
  **Location**: Phase 1 item 4 / Testing Strategy
  The mock records only the last body per `RequestKey`; Linear posts everything to `/graphql`, so a two-POST flow (`create`+writeback, `transition`'s `resolve_state`+`transition`) overwrites the first body. The outgoing-mutation assertion silently degrades. Assert the hit count is 1, or capture per-hit bodies.

- 🔵 **Test Coverage**: The production `from_config` credential path is exercised only by manual verification
  **Location**: Phase 1 item 2 / Phase 3 item 1
  Every automated test injects the env override, taking the `new` branch; the `from_config` branch real users hit is never exercised automatically. Add one test driving `from_config` with a fixture config dir.

- 🔵 **Test Coverage**: Dual-use script classification guard retired with no substitute
  **Location**: Phase 4 item 3 (removing `_DUAL_USE_SCRIPTS`)
  The only pinned exemplar (`jira-fields.sh`) is gone, so the guard is deleted, leaving the dual-use concept unguarded rather than merely unexercised. Confirm the remaining exec-bit guards still handle a future dual-use script, and record what detects a misclassification.

- 🔵 **Code Quality**: Full integer taxonomy imposed on Linear whose contract is string-keyed
  **Location**: Phase 1 item 3 / Decision 6
  Linear skills key on `E_*` strings with one meaningful integer (`107`), which `SurfaceError` Display already emits. A parallel integer taxonomy is largely unread ceremony that duplicates the `E_*` symbol. Anchor Linear parity over the `E_*` names plus `107`.

- 🔵 **Architecture**: Reserved-band exit-code constants are redeclared in three binaries with no shared definition
  **Location**: Phase 1 item 3 / Phase 3 items 1&3 / Decision 6
  The 70–74 reserved band and its D11 non-idempotency semantics live as `pub const` in work-cli and are re-declared independently in jira-cli/linear-cli. A future band change must be hand-propagated across three crates. Consider homing the shared constants in `tracker-support`.

- 🔵 **Safety**: Mock servers and bash generators are deleted in the same phase as the goldens they anchor
  **Location**: Phase 2 item 3 / Phase 4 item 3, Migration Notes
  After deletion neither generator exists on the working revision; regenerating a golden requires reverting. Record the exact revision at which each generator last existed and each golden's provenance so a future maintainer can revive one quickly.

#### Suggestions

- 🔵 **Architecture**: Preview and execution are separately derived, with no structural guarantee they agree
  **Location**: Decision 2 / Phase 2 item 2 / Phase 4 item 2
  The previewed intent and the executed wire bytes are two derivations across two invocations with no coupling. Ensure the pinning test asserts the preview is derived from the *same* resolved-fields path the client consumes, so they cannot drift.

- 🔵 **Architecture**: A non-API config-resolution subcommand lives inside the Jira API-adapter binary
  **Location**: Decision 4 (`resolve-fields`)
  `jira resolve-fields` makes no API call yet sits among the API flows — a minor cohesion smell. Accept as domain-aligned, but factor its config-resolution to share the credential/config plumbing rather than duplicating it.

- 🔵 **Architecture**: CLI-shell scaffolding is duplicated across the two new crates
  **Location**: Phase 1 items 1-5 vs Phase 3 item 1
  Both crates reproduce the clap `Cli`, the seam, the `report` structure, and the help/parity harnesses — consistent with precedent, not a defect. If a third provider arrives, revisit extracting a shared `cli-support` helper.

- 🔵 **Code Quality**: Exit-code parity harness duplicated three ways with no shared helper
  **Location**: Phase 1 item 3 / Phase 3 item 1 / Testing Strategy
  The textual-parsing harness is copied per binary (and a third time in work-cli), and silently pins a single-line `pub const : u8` formatting convention. Factor the shared parse-and-compare into a test-support helper.

### Strengths

- ✅ Capture-oracle-before-deletion sequencing: the binary phases record the exit-code and stdout oracle into committed fixtures while the bash still exists, and the cutover phases consume it then delete the generators — capture provably precedes deletion (architecture, safety, code-quality, correctness, test-coverage).
- ✅ Thin functional-core/imperative-shell boundary: binaries own only arg parsing, the credential context, the env seam, error mapping and rendering; all provider behaviour, retry/backoff and timeouts stay in the 0210 client crates, following the corpus-cli/collaboration-cli precedent (architecture, code-quality).
- ✅ Byte-exact stdout goldens (`Vec<u8>`, never `from_utf8_lossy`) plus per-flow request/response assertions, with the strict contracts (tab-separated resolver, bare-key regex, the six `.data.issue.*` paths, ADF via `document_to_markdown`) named and preserved individually (test-coverage, compatibility, code-quality).
- ✅ Set-equality guards make partial deletion fail-safe: `SHELL_LIBRARIES`, `_RECONCILED_LIBRARIES`, `MOCK_*`, ruff excludes and the mise partition are exact pins, so a half-applied deletion reddens the build (safety).
- ✅ Inherited secrets hygiene: `tracker_support::Secret`/`CredentialError` redact under `Debug`, the token surfaces only via explicit `expose()`, transports set `redirect::Policy::none()`, and Decision 3 removes the cleartext-auth subcommands (security).
- ✅ Same-commit dispatch-coherence rule grounded in the real `dispatch_coherence.py` invariants; token names `jira`/`linear` correctly avoid the reserved/builtin collision sets (correctness, compatibility).
- ✅ The divergence ledger's governing rule — "a divergence nothing can detect is indistinguishable from a defect, so every row names a real, passing test" — carried verbatim from 0167 (code-quality, test-coverage).

### Recommended Changes

1. **Redesign the exit-code parity specification** (addresses: textual-equality-contradicts-remap, aggregation-conflict-rule, verifies-constants-not-routing, self-referential-oracle). Specify: (a) an explicit, count-pinned divergence allowlist where allowlisted rows assert the *remapped* Rust value while the fixture keeps the bash value, every allowlisted name appearing in the ledger; (b) a capture precedence rule (behavioural wins over declared, or names namespaced per flow) with a parse-time uniqueness assertion; (c) a behavioural test per error class that drives the binary into each condition and asserts the observed exit code, with the anti-vacuity count derived from the exhaustive capture rather than hand-picked.

2. **Commit to the work-cli error-flow shape and resolve the `TrackerError` code recovery** (addresses: two-house-shapes-don't-compose, port-op-codes-only-in-detail). State that handlers return `ExitCode` inline (not `Result<Outcome, kernel::Error>`), and decide explicitly how port-op integers are recovered — either record the two-code collapse as a tested divergence or expose the numeric code structurally from `jira-client` rather than substring-parsing `detail`.

3. **Gate and complete the base-URL seam** (addresses: seam-bypasses-allowlist, seam-overrides-only-graphql, omits-team_key-states, malformed-fallback). Gate the env override to test builds (or revalidate it as a credential destination, rejecting non-loopback/non-https before attaching credentials); extend it to build `UploadTransport::new(true, …)` and reconstruct `team_key`/`states` from `integrations_root` identically to `from_config`; and treat a present-but-unparseable value as a hard error.

4. **Add an explicit skill-body cutover sub-step and its guards** (addresses: search-remap-not-reconciled-in-bodies, confirm-gate-prose-only, empty-stdout-gates, witness-metacharacter-free, stale-print-payload-flag). Enumerate every hardcoded in-body exit-code reference to rewrite against the new taxonomy; require the witness skill to invoke the token in a fenced, metacharacter-free step; add a skills-lane assertion that the confirm step lexically precedes each mutation invocation and that a non-zero/exit-16 create suppresses the writeback and blocks retry; and drop `[--print-payload]` from the create `argument-hint`.

5. **Close the stderr/envelope/from_config coverage gaps** (addresses: stderr-audit-lines, search-json-envelope, from_config-untested, last_body-multi-post). Add a stderr golden for the `INFO:` composed-query line and list `--quiet`; pin the search JSON envelope as a named byte-exact golden and flag any client-vs-bash shape gap as a divergence; add one `from_config`-branch test; and assert a single `/graphql` POST per Linear flow (or capture per-hit bodies).

6. **Reconcile the honesty of the divergence ledger and the retired guards** (addresses: preview-divergence-no-test, dual-use-guard-retired, behavioural-oracle-provenance, generators-deleted-with-goldens). Either name the automated test pinning the preview divergence or reclassify it manual-only; record what detects a future dual-use misclassification; record each generator's last-existing revision and each golden's provenance; and gate the behavioural capture behind a differential test that executes the bash while it still exists.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan lands two thin inbound CLI adapters over the already-complete 0210 client crates, keeping the imperative shell cleanly separated from the client-owned functional core, and faithfully follows the per-binary crate precedent. Structural boundaries are sound, capture-before-deletion is a genuine evolutionary-fitness strength, and the two-track shared-floor coupling is explicitly reasoned. Residual concerns are acknowledged tradeoffs: a preview/execution derivation gap, exit-code constants redeclared per binary, and one non-API subcommand in an API-adapter binary.

**Strengths**:
- Thin-adapter boundary consistent with precedent (functional-core/imperative-shell split).
- Capture-the-oracle-before-deletion sequencing.
- Two-track single-shared-floor coupling explicitly identified and ordered; mixed intermediate state reasoned safe against 0167.
- Reserved-band codes anchored to shared `tracker::TrackerError` classification.

**Findings**:
- minor / medium — Preview and execution separately derived, no structural guarantee they agree (Decision 2 / Phase 2 item 2 / Phase 4 item 2).
- minor / medium — Reserved-band exit-code constants redeclared in three binaries with no shared definition (Phase 1 item 3 / Phase 3 items 1&3 / Decision 6).
- suggestion / medium — Non-API config-resolution subcommand inside the Jira API-adapter binary (Decision 4).
- suggestion / low — CLI-shell scaffolding duplicated across the two new crates (Phase 1 items 1-5 vs Phase 3 item 1).

### Correctness

**Summary**: Unusually rigorous about sequencing, the two error conventions, and the same-commit rule, and the base-URL seam type matches the real `Transport::new` signature. But three mechanisms are under-specified in ways that would produce wrong results: the seam overrides only the GraphQL transport (breaking attach/transition goldens); the textual equality parity contradicts the intentional 70–73 remap; and aggregating `readonly E_*=NN` across many scripts has no conflict-resolution rule.

**Strengths**:
- Correctly identifies the two error conventions and assigns the variant→integer mapping to the binary.
- Post-deletion enforcement model mirrors the work-cli precedent.
- Capture-before-delete sequencing logically sound.
- Same-commit witness requirement grounded in real `dispatch_coherence.py` invariants; token names avoid collision sets.

**Findings**:
- major / high — Base-URL seam overrides only the GraphQL transport, upload path stays production (Phase 1 items 2 & 4).
- major / high — Textual equality parity contradicts the 70–73 remap and cross-provider divergences (Phase 1 item 3 / Phase 3 item 3 / Decision 6).
- major / medium — Aggregating `readonly E_*=NN` across many scripts plus behavioural runs has no conflict rule (Phase 1 item 3 / Phase 3 item 1).
- major / medium — Witness invocation must be metacharacter-free, but repointed read skills piped to `jq` (Phase 2 item 2 / Phase 4 item 2).
- minor / medium — Override branch omits `team_key`/`states` reconstruction (Phase 1 item 2).
- minor / low — Malformed override env var silently falls back to production (Phase 1 item 2).

### Test Coverage

**Summary**: The testing skeleton is strong: oracle capture correctly precedes deletion, per-flow tests assert request/response/byte-exact stdout, and the help-surface and init-verify guards are well chosen. The principal gaps: the parity test verifies constant declarations rather than runtime variant→code routing, the preview divergence has no concrete automated test, and the captured fixture is a self-referential oracle whose completeness rests on author-recorded capture.

**Strengths**:
- TDD ordering correct (capture provably precedes deletion).
- Byte-exact `Vec<u8>` goldens with `from_utf8_lossy` prohibited.
- Fixed-count anti-vacuity assertion on parity.
- Per-flow triad plus whole-surface help golden.
- Decision 3's "init verify prints no token" is an explicit negative assertion.

**Findings**:
- major / high — Parity test verifies constant declarations, not runtime variant→code routing (Phase 1 item 3 / Phase 3 items 1&3).
- major / medium — Preview-resolved-intent divergence has no concrete automated pinning test (Decision 2 / Phase 2 item 2 / Phase 4).
- major / medium — Captured fixture is a self-referential oracle (Phase 1 item 3 / Phase 3 item 1).
- minor / medium — `last_body` retains only the final request per key, limiting multi-POST Linear assertions (Phase 1 item 4).
- minor / medium — `_DUAL_USE_SCRIPTS` guard retired with no substitute (Phase 4 item 3).
- minor / low — Production `from_config` path exercised only by manual verification (Phase 1 item 2 / Phase 3 item 1).

### Code Quality

**Summary**: Well-structured, follows the thin-CLI precedents, sequences oracle-capture before deletion with byte-exact goldens and a disciplined divergence ledger. The main maintainability risks are in the error-flow: the work-cli taxonomy grafted onto collaboration-cli's collapsing `report(&kernel::Error)` funnel (two shapes that don't compose), and port-op integers that live only as substrings inside `TrackerError.detail`. The per-binary machinery is also duplicated three ways and asymmetric to Linear's string-keyed contract.

**Strengths**:
- Binaries are genuinely thin inbound adapters.
- Oracle capture strictly before deletion.
- Byte-exact `Vec<u8>` goldens plus injectable mock/seam testability.
- The divergence ledger's governing rule carried verbatim from 0167.

**Findings**:
- major / high — Two house shapes don't compose: rich taxonomy grafted onto a collapsing `kernel::Error` funnel (Phase 1 item 1 vs item 3).
- major / high — Port-op exit integers recoverable only by parsing `TrackerError.detail` (Phase 3 item 1).
- minor / medium — Full integer taxonomy imposed on Linear whose contract is string-keyed (Phase 1 item 3 / Decision 6).
- suggestion / medium — Exit-code parity harness duplicated three ways with no shared helper (Phase 1 item 3 / Phase 3 item 1).

### Safety

**Summary**: A dev-tooling plugin where every destroyed artefact is VCS-tracked, so the data-loss risk is contained — recovery is a revert, and deletion ordering captures the oracle before removing generators. Residual concerns are about the fidelity of the surviving safeguards: the confirm gate is downgraded to prose plus a manual checkbox, the empty-stdout gates are redesigned onto exit codes, and the behavioural oracle is a one-shot manual capture — and that taxonomy is precisely what prevents duplicate remote mutations.

**Strengths**:
- Recovery model sound for the domain (VCS-tracked, per-phase green, revert recovery).
- Deletion ordering captures the oracle before removing generators.
- Partial-deletion fail-safe via set-equality pins.
- Binary lands and is validated as a separate merge before its bash is deleted.
- Decision 3 removes cleartext-credential subcommands with a no-token-emitted test.
- Mixed bash/accelerator inter-track state genuinely low-risk.

**Findings**:
- major / medium — Confirm gate relies on SKILL.md prose plus a manual checkbox (Decisions 2, Phase 2/4 item 2).
- major / medium — Empty-stdout gates redesigned onto exit codes must fail closed (Phase 4 item 2, Decision 5).
- major / medium — Behavioural exit-code oracle is a one-shot manual capture with weaker provenance (Phase 1 item 3 / Phase 3 item 1).
- minor / medium — Mock servers and bash generators deleted in the same phase as the goldens they anchor (Phase 2/4 item 3).

### Compatibility

**Summary**: The core contract-preservation strategy is sound: capture byte-exact goldens and an exit-code oracle before deletion, pin with parity tests, enumerate the strict contracts. The main exposure is at the seam between the deliberately-changed taxonomy (the 70–73 remap, dropped `--print-payload`) and the repointed skill bodies, which carry hardcoded exit-code tables and stdout/stderr parsing the plan repoints generically. Stderr audit-line contracts and search JSON envelopes sit outside the nets.

**Strengths**:
- Capture-before-delete methodology with fixed-count anti-vacuity parity.
- Strict output contracts named and preserved individually.
- Per-provider atomic phases; same-commit rule binds registration to repointing.
- Cross-provider collisions and the credential-flatten-to-22 divergence pinned per provider.

**Findings**:
- major / high — Search 70–73 remap not reconciled with the exit-code tables baked into repointed bodies (Phase 3 item 3 / Phase 4 item 2).
- major / medium — Stderr audit-line contracts fall outside the preservation nets (Testing Strategy / Phase 1 item 4 / Phase 3).
- major / medium — Search JSON-envelope shape must match the client projection, not just a mock golden (Phase 1 item 4 / Phase 3 item 2).
- minor / medium — Dropped `--print-payload` leaves a stale advertised flag in the create `argument-hint` (Decision 2 / Phase 2/4 item 2).
- minor / low — Init-written cache-file formats not verified compatible with pre-existing bash state (`init verify | refresh-fields`).

### Security

**Summary**: Ports two authenticated HTTP integrations from bash to Rust, inheriting strong secrets hygiene (the `Secret` type and `CredentialError` redact under `Debug`, tokens surface only via `expose()`, Jira's `auth.rs` validates the credential destination against a `*.atlassian.net` allowlist). The dominant concern is the base-URL seam: read unconditionally in `main.rs` (production-reachable) and, for Jira, deliberately bypassing that allowlist — turning an env var into a credential-exfiltration/SSRF vector.

**Strengths**:
- Inherited `Secret`/`CredentialError` redact under `Debug`; token only via `expose()`.
- Jira's `auth.rs` validates the credential destination (`*.atlassian.net` allowlist) on the non-seam path.
- Decision 3 (drop cleartext-auth, fold into no-token `init verify`) is a net improvement.
- Transports set `redirect::Policy::none()`.
- `jira.allowed_sites` refused from shared config.

**Findings**:
- major / high — Base-URL env seam bypasses the credential-destination allowlist and redirects the authenticated token (Phase 1 item 2 / Phase 3 item 1).
- minor / medium — Single assertion is thin coverage for the "init verify never prints the token" guarantee (Decision 3 / Phase 1 item 4 / Phase 3).

## Re-Review (Pass 2) — 2026-08-19

**Verdict:** REVISE

Six lenses (correctness, test-coverage, code-quality, safety, compatibility, security) were re-run against the revised plan. **Every Pass-1 major was resolved** — the exit-code parity now uses a count-pinned divergence allowlist, the base-URL seam reconstructs the whole client with the loopback-admitting upload transport and `team_key`/`states`, the metacharacter-free witness requirement is verified against `dispatch_coherence.py:103`, the confirm gate and fail-closed writeback are automated, and the `work-cli` inline-`ExitCode` shape removes the compose contradiction. The re-review then surfaced a cluster of **new/partial issues introduced or left by the Pass-1 edits**, the most serious being code-verified: **Decision 9's `bash_code(&TrackerError) -> u8` accessor is not implementable** (flagged independently by correctness, code-quality, and compatibility). A **second edit pass has since addressed every item below**; a Pass-3 verification run would confirm closure.

### Previously Identified Issues (Pass-1 majors)
- 🟡 **Correctness**: Base-URL seam overrides only the GraphQL transport — Resolved (upload transport + `team_key`/`states` reconstructed; verified `UploadTransport::new(true, …)` exists).
- 🟡 **Correctness**: Textual equality parity contradicts the remap — Resolved (count-pinned divergence allowlist).
- 🟡 **Correctness**: Aggregation has no conflict rule — Resolved (behavioural-wins precedence + per-`(flow,name)` uniqueness).
- 🟡 **Correctness**: Witness must be metacharacter-free — Resolved (verified against `dispatch_coherence.py:103`; pinned as a criterion).
- 🔴 **Security**: Base-URL seam bypasses the allowlist — Partially resolved at re-review (arbitrary-host closed; loopback-in-release remained) → **addressed in Pass 2** (loopback gated to `cfg(test)`/`debug_assertions`; https+allowlisted only in release).
- 🔴 **Code Quality / Correctness**: Two house shapes don't compose — Resolved (`work-cli` shape adopted wholesale).
- 🟡 **Code Quality**: Port-op codes only via `detail` parse — **Still present at re-review** (proposed accessor unsound) → **addressed in Pass 2** (reuse existing `bash_code(Outcome)` via surfaced `Outcome`).
- 🔴 **Test Coverage**: Parity verifies declarations not routing — Resolved (behavioural exit-code test per class).
- 🟡 **Test Coverage**: Preview divergence has no automated test — Partially resolved (guards named but under-specified) → **addressed in Pass 2** (observable seam + parse spec + failing fixture).
- 🟡 **Test Coverage**: Self-referential oracle — Resolved (scripted differential capture executing the bash).
- 🟡 **Test Coverage**: `last_body` multi-POST — Partially resolved (harness can't capture per-hit) → **addressed in Pass 2** (additive `Vec<Received>` change to `http-test-support`).
- 🟡 **Safety** (×3): confirm gate, fail-closed writeback, oracle provenance — All resolved.
- 🔴 **Compatibility**: Search remap not reconciled in bodies — Resolved (same-commit rewrite + whole-corpus doc-vs-binary parity).
- 🟡 **Compatibility**: Stderr audit lines / search envelope — Resolved (named stderr + envelope goldens).
- 🔵 **Compatibility**: `--print-payload` stale flag — Partially resolved (scoped to create only) → **addressed in Pass 2** (broadened to all write skills, both providers).

### New Issues Introduced (surfaced by the re-review, now addressed in Pass 2)
- 🔴 **Code Quality / Correctness / Compatibility**: Decision 9's `bash_code(&TrackerError) -> u8` accessor cannot be built additively — `TrackerError` carries only `Retryable`/`Terminal` (code baked into `detail`), and a `pub bash_code(Outcome) -> u16` already exists (name/signature collision). **Addressed**: Decision 9 rewritten to surface the `classify::Outcome` from the client's port-op path and reuse the existing `bash_code(Outcome)`; no `&TrackerError` accessor, no `detail` parse, no `tracker`-port change.
- 🔴 **Security**: Loopback admission shipped in the release binary — the anti-pattern `upload.rs`'s module doc refuses. **Addressed**: loopback gated to test/debug builds; release admits only https+allowlisted through a single shared `pub` admissibility helper.
- 🟡 **Correctness**: `api_base_uri` sketch used a non-existent `SurfaceError::bad_api_url()`. **Addressed**: routed through the binary's own `UsageError`/`exit_codes` usage path.
- 🟡 **Compatibility**: Linear stderr `E_*` net omitted ~nine binary-owned validation names (`E_CREATE_ALREADY_SYNCED`, …). **Addressed**: stderr goldens pin every `E_*` name any repointed body references, enumerated from the capture.
- 🔵 **Compatibility**: Known-consumer enumeration undercounted (7 Jira bodies, not 2). **Addressed**: all seven enumerated; parity test scans all sixteen bodies with anti-vacuity.
- 🔵 **Test Coverage**: doc-vs-binary parity and behavioural-test non-network classes under-specified. **Addressed**: parity test home/parse/anti-vacuity specified; behavioural test drives non-network classes via real triggers.
- 🔵 **Safety**: exit-16 leaves an orphaned remote issue; per-track provenance timing. **Addressed**: exit-16 surfaces an explicit "issue created remotely as `<key>`" message; Linear provenance recorded in Phase 2.
- 🔵 **Code Quality**: Decision list numbered out of order (9/10 before 7/8). **Addressed**: renumbered contiguously.

### Assessment
The plan is materially stronger: all fifteen Pass-1 findings are resolved, and the eight new/partial items the re-review surfaced — including the code-verified Decision 9 blocker and the loopback-in-release security regression — have been addressed by a second edit pass. The remaining suggestions (shared parity-harness helper, reserved-band constant home) stay deferred as precedent-consistent. Recommended next step: a Pass-3 verification run of correctness, code-quality, and security to confirm the Decision 9 and seam rewrites hold, after which the plan should reach APPROVE.

## Re-Review (Pass 3) — 2026-08-19

**Verdict:** REVISE

Correctness, code-quality, and security were re-run to verify the Pass-2 rewrites of Decision 9 and the base-URL seam. **Both Pass-2 targets verified resolved against the code**: Decision 9's `bash_code(&TrackerError)` accessor is gone (reviewers confirmed `TrackerError` has no structural code field and a `pub bash_code(Outcome) -> u16` already exists), the `api_base_uri` sketch routes through the binary's own `UsageError`, and loopback is out of the release path. The pass then found **three new majors** — one code-verified correctness gap that matters for the exit-16 safety semantic, and two security hardening points on the seam. A **third edit pass has addressed all of them**.

### Previously Identified Issues (Pass-2 targets)
- 🟡 **Code Quality / Correctness**: Decision 9 `bash_code(&TrackerError)` accessor unsound — Resolved (verified: reuses existing `bash_code(Outcome) -> u16`; `TrackerError` erasure confirmed at `tracker/src/lib.rs:144-179`).
- 🟡 **Correctness**: `api_base_uri` used non-existent `SurfaceError::bad_api_url()` — Resolved (now binary-owned `UsageError`).
- 🔴 **Security**: loopback shipped in release — Resolved (verified: loopback out of the release path; https+allowlisted only).
- 🔵 **Code Quality**: decisions out of order — Resolved (now 1..10).

### New Issues Introduced (surfaced by Pass 3, now addressed in a third edit pass)
- 🟡 **Correctness (major)**: Not every port-op failure computes a `classify::Outcome` — `create`'s post-create unusable-identifier (`client.rs:337-344`) and `fetch_all`'s unsafe-id (`:420-428`) build `TrackerError` inline. The post-create case is the exit-16 "created remotely but unwritable" condition. **Addressed**: Decision 9 now specifies a structured discriminant on the port-op error path covering *every* branch (an `Outcome`, or an explicit reason like `UnwritableIdentifier`/`UnsafeQueryId`), mapped directly by `exit_codes.rs`.
- 🟡 **Security (major)**: `debug_assertions` gate made loopback env-reachable in every debug build. **Addressed**: replaced with a default-off `test-loopback` cargo feature enabled only by the integration tests; `allow_loopback = cfg!(feature = "test-loopback")` stays a caller-supplied runtime bool.
- 🟡 **Security (major)**: Jira's promoted `host_is_admissible` is host-only (no https/userinfo/port guards), admitting cleartext `http://foo.atlassian.net`. **Addressed**: the Jira seam reuses the already-`pub`, fully-validated `auth.rs::base_url`; "shared helper" is clarified as each binary reusing its own client's complete destination check, not one cross-provider function.
- 🔵 **Correctness (minor)**: "work-cli-shaped report" phrasing contradicted the no-`report` decision. **Addressed**: reworded to inline `ExitCode::from(exit_codes::USAGE)`.
- 🔵 **Code Quality (minor)**: "compile-checked" over-claimed the no-`detail`-parse property. **Addressed**: reworded to a typed call plus a lint/grep guard.

### Assessment
The plan has converged at the architecture and contract level. Across three passes, every finding has been resolved or consciously deferred; the issues surfaced in this pass were progressively more granular (specific `client.rs` failure branches, a feature-gate mechanism, a helper-visibility detail) — the signature of diminishing returns. Nothing outstanding is structural: the remaining risk is ordinary implementation detail that TDD will surface. **Treat the plan as ready for implementation.** The frontmatter verdict stays REVISE only because this pass, like any, technically surfaced new majors before they were fixed; a clean Pass-4 of correctness + security would confirm closure and flip it to APPROVE, but is optional.

## Re-Review (Pass 4) — 2026-08-19

**Verdict:** APPROVE

Correctness and security were re-run to verify the Pass-3 rewrites (the Decision 9 structured discriminant, the `test-loopback` feature gate, and the `base_url` reuse). **Both Pass-3 targets are verified resolved against the code**, with reviewers confirming the specific facts:
- **Decision 9 discriminant is provably total** — verified that exactly two `TrackerError` branches per client build inline without an `Outcome` (`jira-client/src/client.rs:337-344`, `:420-428`; linear `client.rs:297`, `:364`), and every other error path routes through `classify()` carrying an `Outcome`, so the two-arm enum (`Outcome` | explicit reason) covers all branches.
- **Loopback is unreachable in ordinary builds** — the `test-loopback` feature is a caller-supplied runtime bool through the real `UploadTransport::new(bool)` / `url_is_allowed(url, bool)` signatures; no `debug_assertions` gate survives.
- **Jira `base_url` enforces https/userinfo/port** — cleartext `http://foo.atlassian.net` is rejected; `host_is_admissible` stays private.

The pass surfaced **one new major**, which was a documentation artifact of the Pass-3 fix itself: Decision 10 claimed a false symmetry — the Jira `base_url` has no `allow_loopback` parameter and hard-rejects http/ports, so it cannot admit a loopback mock the way Linear's `url_is_allowed` can. **Addressed** conservatively: `base_url` stays strict and unchanged; the Jira seam's release path routes through it unchanged, and the `test-loopback` mock path uses a dedicated gated branch that constructs `Credentials` at the override directly (as `tests/support/client.rs` already does), bypassing `base_url` — never relaxing it. This keeps the strictest possible destination check in release while giving the binary tests a loopback path. Two low/minor observations (the feature must be enabled via the test target, not a crate-dir `.cargo/config.toml`; the Jira-loopback wording) were also folded in.

### Previously Identified Issues (Pass-3 targets)
- 🟡 **Correctness**: Port-op `Outcome` gap — Resolved (discriminant verified total against the crates).
- 🟡 **Security**: `debug_assertions` gate — Resolved (verified: `test-loopback` feature; no `debug_assertions` remains).
- 🟡 **Security**: Jira host-only helper — Resolved (verified: `base_url` enforces https/userinfo/port; `host_is_admissible` private).
- 🔵 minors (report phrasing, compile-checked over-claim) — Resolved.

### New Issues Introduced (surfaced by Pass 4, now addressed)
- 🟡 **Correctness (major)**: Decision 10's Jira/Linear loopback symmetry was false — `base_url` can't admit loopback. **Addressed**: `base_url` stays strict; Jira `test-loopback` mock path constructs `Credentials` directly, bypassing (not weakening) `base_url`.
- 🔵 **Security/Correctness (low)**: feature-enablement path and Jira-loopback wording. **Addressed**.

### Assessment
**Approved — ready for implementation.** Four passes have driven the plan to convergence: every finding across architecture, contract, mechanism, and implementation detail is resolved or consciously deferred, and this pass verified the two most load-bearing rewrites (the exit-code discriminant and the credential-destination seam) directly against the crate code. The single new item this pass raised was self-inflicted by the Pass-3 edit and has been closed with the strictly-safest option (leave `base_url` untouched). The final conservative edit is reasoned-safe rather than independently agent-re-verified, but it removes rather than adds capability, so its risk is minimal. Remaining deferred suggestions (shared parity-harness helper, reserved-band constant home) are precedent-consistent and out of scope. No further review passes are warranted.
