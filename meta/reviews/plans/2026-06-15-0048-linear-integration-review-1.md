---
type: plan-review
id: "2026-06-15-0048-linear-integration-review-1"
title: "Plan Review: Linear Integration Implementation Plan"
date: "2026-06-15T00:28:39+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-15-0048-linear-integration"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, standards, portability]
review_number: 1
review_pass: 3
tags: [work-management, integrations, linear, graphql]
last_updated: "2026-06-15T08:51:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Linear Integration Implementation Plan

**Verdict:** REVISE

This is a strong, unusually disciplined plan: it mirrors a proven Jira
integration verbatim, enumerates its four genuine divergences up front, is
test-first per phase, and maps nearly every acceptance criterion to a named
automated assertion. The concerns are concentrated precisely at the
divergences — the net-new GraphQL error classifier, the transport-folded
pagination, the `work_item_id` frontmatter writeback, and the three-step
binary attach — where the design crosses trust boundaries and adds control
flow that has no Jira precedent and is currently under-specified on
validation, clamping, and failure semantics. Three critical issues (a
work-item-file corruption surface, an SSRF/credential-leak surface on the
binary upload, and a misrouted auth-error exit code) and several reinforcing
major findings warrant a revision pass before implementation.

### Cross-Cutting Themes

- **GraphQL error classifier is fragile and incomplete** (flagged by:
  correctness, code-quality, architecture, safety, portability) — the
  order-dependent, message-substring dispatch in `linear-graphql.sh` is the
  single most-flagged area. It omits an authentication-error branch (so a
  200-body auth error misclassifies as `E_GQL_BAD_REQUEST`), depends on an
  undocumented complexity-message string the plan itself flags as a known
  risk, and concentrates the integration's hardest logic in the shared
  transport with an invisible ordering constraint.
- **Rate-limit backoff is unbounded and non-portable** (flagged by:
  correctness, safety, portability) — backoff is `(reset_ms − now_ms)/1000`
  clamped only `≥ 0`, with "later of two resets" potentially yielding
  ~hour-long sleeps (Jira clamps `[1,60]s`), and `now_ms` has no portable
  source (`date +%s%N` is GNU-only; BSD/macOS — the bash 3.2 floor's reason
  for being — lacks `%N`).
- **`work_item_id` writeback crosses two trust boundaries unguarded**
  (flagged by: safety, security, architecture, code-quality, test-coverage) —
  the awk single-line replacer can silently corrupt a human-authored
  source-of-truth file (no post-transform integrity check, unlike
  `jira_atomic_write_json`'s `jq empty`), writes a server-returned identifier
  without validating its shape, sits in the wrong (integration-specific)
  layer, reinvents `config_extract_body`, and lacks failure/atomicity tests.
- **Binary attach is the deepest new surface and is under-specified**
  (flagged by: security, safety, architecture, correctness, test-coverage) —
  the PUT to a server-returned `uploadUrl` echoing every server-returned
  header risks SSRF and token leakage, partial-failure across
  fileUpload→PUT→attachmentCreate has no defined cleanup/idempotency, the
  step bypasses the transport's retry/timeout machinery, and the
  `expect_headers` assertion only fails at mock teardown.
- **Mock-server fidelity cannot verify single-endpoint ordering** (flagged
  by: test-coverage, architecture) — every request is `POST /graphql`, so the
  positional-consumption harness answers by order regardless of body content;
  `expect_body_contains` is added but is not specified to *fail* on mismatch,
  so multi-step scenarios can pass even when the wrong operation is sent.

### Tradeoff Analysis

- **Architecture (single responsibility) vs Usability (single result set)**:
  Folding pagination into the transport gives callers one merged result set
  but overloads `linear-graphql.sh` with multi-request orchestration the Jira
  template deliberately keeps caller-side. Recommendation: if kept, isolate
  the pagination loop in its own function with an explicit `MAX_PAGES` cap and
  defined partial-page-failure semantics, rather than inlining it into the
  retry/error path.
- **Security (validate/strip) vs Code simplicity (echo everything)**: The
  binary PUT's "echo every returned header" is simplest but is unbounded
  header injection. Recommendation: favour security — allow-list the signed
  upload headers, strip `Authorization`/`Host`, reject CR/LF in values.
- **Robustness (downshift on complexity) vs Scope (terminal error)**: The
  plan treats the complexity cap as terminal with a fixed `first: 50`,
  diverging from the research's halve-and-retry suggestion. This is a
  defensible scope call; recommendation is to keep it but record the
  "queries must stay under 10,000 points at `first: 50`" constraint as a
  first-class invariant and name `--first` downshift as the evolution path.

### Findings

#### Critical

- 🔴 **Safety**: Awk single-line frontmatter replacer can silently corrupt or truncate a human-authored work-item file
  **Location**: Phase 1 §2 (`linear_writeback_work_item_id`) + Phase 3 §1
  `atomic_write` makes the rename atomic but performs no validation of the
  awk output (unlike `jira_atomic_write_json`'s `jq empty`). A zero-match,
  body-match, or empty-output edge case silently overwrites the source file
  with corrupted/truncated content; recovery is VCS revert only.

- 🔴 **Security**: Binary-upload PUT to a server-returned `uploadUrl` must strip Authorization and validate the host
  **Location**: Phase 4 §2 (Attach binary, step 2 PUT)
  The transport hard-codes `Authorization: <LINEAR_TOKEN>`; a naive reuse for
  the PUT would forward the personal API key to an arbitrary server-supplied
  URL. A tampered `fileUpload` response turns the PUT into an SSRF /
  data-exfiltration primitive leaking the token and file bytes.

- 🔴 **Correctness**: Authentication-error (HTTP-200 `errors[]`) has no branch in the ordered classifier and falls through to `E_GQL_BAD_REQUEST`
  **Location**: Phase 1 §4 (Transport — error dispatch)
  `E_GQL_UNAUTHORIZED` (11) is defined for 401 *or* a 200-body
  `extensions.type == "authentication error"`, but the classifier's three
  branches (complexity, RATELIMITED, else bad-request) never match a 200-body
  auth error — it lands on exit 34 with a "bad request" message, breaking the
  Bearer-prefix AC if Linear returns auth errors that way.

#### Major

- 🟡 **Correctness / Safety / Portability**: Rate-limit backoff has only a lower clamp; "later of requests-reset and complexity-reset" can yield ~hour-long sleeps, and `now_ms` has no portable source
  **Location**: Phase 1 §4 (RATELIMITED backoff); Key Discoveries (epoch-ms)
  No upper clamp (Jira uses `[1,60]s`); the hourly complexity-reset window can
  drive multi-minute sleeps × 4 attempts. Separately, `date +%s%N` is
  GNU-only and emits a literal `N` on BSD/macOS, corrupting the arithmetic.

- 🟡 **Architecture / Code Quality / Correctness**: Folding pagination into the transport overloads it and lacks a cursor-progress/`MAX_PAGES` guard
  **Location**: Phase 1 §4 (Transport — pagination)
  Jira keeps pagination caller-side; folding it in couples per-request retry
  with cross-request accumulation, rewrites response shape, and (per
  correctness) has no defence against a non-advancing/null `endCursor` or
  unbounded page count — an infinite-loop / unbounded-memory risk. The
  in-repo precedent (`jira-comment-flow.sh`) caps with `MAX_PAGES=20`.

- 🟡 **Code Quality / Architecture / Safety**: GraphQL error classifier concentrates fragile, order-dependent branching on an undocumented message string in the shared transport
  **Location**: Phase 1 §4 (Transport — error dispatch)
  Correctness depends on complexity-before-RATELIMITED ordering and a
  message-substring (`complexity` / `10,?000`) the plan flags as unverified;
  a wording drift silently reclassifies a terminal complexity error as a
  retryable rate-limit.

- 🟡 **Correctness**: Routing 200-with-errors through the retrying RATELIMITED branch can retry non-idempotent mutations
  **Location**: Phase 1 §4 (classifier shared between 200-errors and 400)
  RATELIMITED is documented as HTTP 400; accepting it on a 200 body widens
  the retry trigger and risks duplicate side effects on create/comment/
  transition/attach. Restrict the retrying branch to genuine HTTP-400.

- 🟡 **Security / Architecture**: Binary PUT echoes every server-returned `headers[]` entry (unbounded header injection) and bypasses transport resilience
  **Location**: Phase 4 §2 (Attach binary, returned headers + direct curl)
  No allow-list / CR-LF rejection on echoed headers; the step also has no
  retry/timeout/connect-mapping the rest of the integration inherits, leaving
  a `fileUpload`-allocated asset orphaned on a transient failure.

- 🟡 **Security**: Hand-built `header = "Authorization: <token>"` curl-config directive can be corrupted/injected by token contents
  **Location**: Phase 1 §4 (Transport — Authorization via `curl --config -`)
  Unlike Jira's `user =` (curl-encoded), a token containing `"`, `\`, or a
  newline parses as additional/closing config directives. Reject control
  chars/quotes in `linear_resolve_credentials`; test a token with `"` + `\n`.

- 🟡 **Security / Safety**: Server-returned issue identifier is written into local frontmatter without validation
  **Location**: Phase 1 §2 + Phase 3 §1 (writeback)
  The existing `work_item_id` is validated against `^[A-Z][A-Z0-9]*-[0-9]+$`,
  but the *returned* identifier is not before being written — a tampered
  response could inject YAML/newlines into a tracked file. Validate before
  writeback.

- 🟡 **Safety**: Three-step binary attach has no defined partial-failure or cleanup semantics
  **Location**: Phase 4 §2 (Attach binary)
  A PUT-succeeds / attachmentCreate-fails path leaves an orphaned asset; a
  retry re-uploads with no idempotency key, accumulating orphans and
  potentially violating the "exactly one new attachment" AC.

- 🟡 **Architecture**: `linear_writeback_work_item_id` is a cross-cutting capability placed in the integration-specific common layer
  **Location**: Phase 1 §2 (Common helper)
  A generic frontmatter single-field setter for shared `meta/work/` files
  belongs beside `config_extract_frontmatter` in shared `scripts/`, not in
  `linear-common.sh`, where 0047/Jira would have to reach across or duplicate.

- 🟡 **Architecture**: Hard-coded `first: 50` + terminal complexity cap leaves future field-set expansion un-paginatable
  **Location**: What We're NOT Doing / Phase 1 §4 (pagination)
  Defensible scope call, but the under-10,000-points-per-page assumption is
  unenforced; record it as a first-class invariant and name `--first`
  downshift as the designated evolution path.

- 🟡 **Test Coverage**: Sequenced single-endpoint scenarios rely on positional ordering the harness can't verify by content
  **Location**: Phase 1 §7 (mock-linear-server.py `expect_body_contains`)
  The mock answers by order regardless of body; `expect_body_contains` must
  be specified to *fail* (push to `server.errors`) on mismatch, and
  multi-step scenarios should assert the operation name per step.

- 🟡 **Test Coverage**: `expect_headers` fails only at server teardown, so a SIGKILL'd mock can swallow header-echo failures
  **Location**: Phase 4 §3 (binary attach `expect_headers`)
  The load-bearing signed-PUT header echo could pass even if a header is
  dropped. Assert the mock's exit status / captured errors explicitly, or
  capture and assert the PUT headers directly in bash.

- 🟡 **Test Coverage**: No test that a RATELIMITED 400 followed by a 200 resumes successfully
  **Location**: Phase 1 §4 success criteria
  Only exhaustion (exit 35) and backoff value are asserted; the happy retry
  path (exit 0, 200 body, exactly one recorded sleep) is unverified. Jira
  covers the 429→200 case explicitly.

- 🟡 **Standards**: `init-linear/SKILL.md` is assigned broad `[Bash, Read, Write]` where its `init-jira` analogue uses narrowly-scoped `Bash(...)` globs
  **Location**: Phase 1 §5 (init-linear flow + skill)
  The broad grant breaks the mirrored init-skill convention. Scope to
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/linear/scripts/*)`,
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`, `Bash(jq)`, `Bash(curl)`.

#### Minor

- 🔵 **Correctness**: Merged-response synthesis on an empty/single-page connection is unspecified (null vs `[]` `nodes`).
  **Location**: Phase 1 §4 / Phase 2 (pagination merge)
- 🔵 **Correctness**: Already-synced regex must trim quotes/whitespace from the extracted `work_item_id` or a quoted synced id slips the guard → duplicate create.
  **Location**: Phase 3 §1 (already-synced guard)
- 🔵 **Correctness**: Transition state-name → UUID match case-sensitivity, trimming, and duplicate-name ambiguity are unspecified.
  **Location**: Phase 4 §1 (transition cache resolution)
- 🔵 **Correctness / Safety**: Binary attach partial-failure recovery (orphaned asset, re-run re-uploads) undefined.
  **Location**: Phase 4 §2
- 🔵 **Code Quality**: Create flow reinvents body extraction; `config_extract_body` already returns the markdown body and handles malformed-frontmatter edges.
  **Location**: Phase 3 §1
- 🔵 **Code Quality**: Decompose the three-call binary attach into named sub-steps with explicit failure propagation.
  **Location**: Phase 4 §2
- 🔵 **Code Quality**: Wide flat exit-code ranges risk drift between `EXIT_CODES.md` and `return NN` literals; name codes as readonly constants near each flow.
  **Location**: Phase 1 §1
- 🔵 **Test Coverage**: `E_GQL_UNAUTHORIZED` via the 200-body `errors[]` path is defined but not exercised (only `bearer-401`).
  **Location**: Phase 1 §4 + EXIT_CODES
- 🔵 **Test Coverage**: Writeback failure/atomicity and frontmatter edge cases (missing field, special chars, byte-identical remainder) untested.
  **Location**: Phase 3 §1 + Phase 1 §2
- 🔵 **Test Coverage**: `linear_with_lock` contention and stale-reclaim (exit 53) paths are ported but not enumerated for coverage.
  **Location**: Phase 1 §2
- 🔵 **Test Coverage**: Complexity-substring classifier is tested only against author-controlled fixtures; add a negative control (generic 400 → 34; RATELIMITED-coded 400 does not match complexity).
  **Location**: Phase 1 Testing Strategy
- 🔵 **Test Coverage**: Search state-name → catalogue resolution not directly asserted (mock answers positionally); capture and assert the composed filter.
  **Location**: Phase 2 §1
- 🔵 **Security**: Blanket echo of GraphQL error bodies to stderr may surface request-echoed input; prefer emitting only `errors[].message`/`extensions.code`.
  **Location**: Phase 1 §4
- 🔵 **Security**: Ensure each write SKILL.md (esp. attach) states the file path/URL must come from the user's current turn and previews the exact `uploadUrl` host before the confirm gate.
  **Location**: Phase 3 §1 / Phase 4
- 🔵 **Safety**: Double-create guard relies solely on local frontmatter, not remote verification — a failed writeback + re-run creates a duplicate.
  **Location**: Phase 3 §1
- 🔵 **Safety**: Local work-item file mutation is outside the state-dir lock scope (acceptable at this tool's scale; note the limitation).
  **Location**: Phase 1 §2 / Phase 3 §1
- 🔵 **Architecture**: Make `expect_body_contains` a hard match so ordering drift fails precisely per step rather than as cross-talk.
  **Location**: Phase 1 §7
- 🔵 **Standards**: Work item still references a non-existent shared `scripts/EXIT_CODES.md`; correct it alongside the rate-limit patch.
  **Location**: Phase 1 §6 / work item
- 🔵 **Standards**: `search-linear-issues` flow lists a `--team` flag that contradicts the "no per-invocation `--team`" exclusion.
  **Location**: Phase 2 §1
- 🔵 **Standards**: Linear transport omits Jira's 403/404/410/429 status mappings without documenting the GraphQL-collapses-these rationale (show-issue-404 will hit this).
  **Location**: Phase 1 §1
- 🔵 **Portability**: Attach file-validation mirror may inherit GNU-only `readlink -f`; keep the Jira `|| true` guard, avoid new GNU-only coreutils flags.
  **Location**: Phase 4 §2

#### Suggestions

- 🔵 **Portability**: Record the deep, unabstracted coupling to Linear's GraphQL API (hardcoded endpoint, proprietary error encoding) as accepted per-vendor lock-in; keep the discriminator in one classifier function.
  **Location**: Overview / Phase 1 §4

### Strengths

- ✅ Layering and the transport contract (body→stdout, errors→stderr, outcome
  via exit code) are mirrored verbatim from a proven template, so the eight
  new skills read identically to the eight a maintainer already knows.
- ✅ The four divergences from Jira are named, scoped, and individually
  justified up front rather than smeared implicitly across the design.
- ✅ Phase boundaries are genuinely independently mergeable and
  dependency-ordered (foundation → read → standard writes → divergent writes).
- ✅ Phase-by-phase TDD with "watch them fail, then implement", each phase
  ending on `mise run scripts:check` green; nearly every acceptance criterion
  maps to a named automated assertion.
- ✅ The three highest-risk edge cases (classifier ordering, ±2s backoff
  tolerance, pagination termination) have concrete, falsifiable assertions;
  retry timing uses the file-based sleep-seam, never real sleeps.
- ✅ Strong inherited secrets posture: token-out-of-argv, 4-tier precedence,
  shared-config `token_cmd` ban, 0600 fail-closed gate with symlink
  rejection, loopback-only test-mode base-URL override, name-restricted
  retry-sleep hook.
- ✅ The catalogue-resolved transition (no live lookup) is a clean coupling
  reduction over Jira's live `/transitions` GET, verifiable by stubbing the
  catalogue endpoint to fail.
- ✅ Naming, exit-code namespacing, the per-integration `EXIT_CODES.md`
  ownership model, and the bash 3.2 floor are honoured consistently; known
  risks are surfaced honestly in Migration Notes.

### Recommended Changes

1. **Add post-transform integrity guards to `linear_writeback_work_item_id`**
   (addresses: Safety critical; Security identifier-validation; Test Coverage
   writeback). Anchor the awk strictly between the first two `---` delimiters;
   require exactly one matched line; re-run `config_extract_frontmatter` on the
   candidate and require it parses and contains the new identifier; validate
   the returned identifier against `^[A-Z][A-Z0-9]*-[0-9]+$` before writing;
   fail closed (non-zero, file untouched) on any mismatch. Add fixtures for
   missing-field, special-char, malformed-identifier, and byte-identical-remainder.

2. **Harden the binary-attach PUT** (addresses: Security SSRF/header-injection;
   Safety partial-failure; Architecture resilience-bypass; Test Coverage
   `expect_headers`). Issue the PUT with NO `Authorization` header; gate
   `uploadUrl`/`assetUrl` to `https://` on an allow-listed host; restrict
   echoed headers to the signed-upload set and reject CR/LF values; give the
   PUT a timeout + bounded retry mapped to `E_ATTACH_UPLOAD_FAILED`; define
   per-step failure messaging and the orphaned-asset/re-run contract; assert
   the mock's exit status (or capture PUT headers in bash).

3. **Complete and isolate the GraphQL error classifier** (addresses:
   Correctness auth-branch + retry-routing; Code Quality fragility; Safety;
   Architecture). Extract `_linear_classify_gql_error` with the ordering
   rationale documented; add an auth-error branch (`extensions.type ==
   "authentication error"` → 11) ordered before bad-request; restrict the
   retrying RATELIMITED branch to genuine HTTP-400; keep the complexity
   substring in one labelled constant pinned by a negative-control fixture.

4. **Bound and make portable the rate-limit backoff** (addresses: Correctness
   clamp; Safety unbounded sleep; Portability `now_ms`). Apply an explicit
   upper clamp (mirror Jira's `[1,60]s`); reconsider "later of two resets"
   given the hourly complexity window; compute time at second granularity
   (`date +%s`) — the ±2s test tolerance already permits it — and avoid
   GNU-only `%N`.

5. **Make pagination safe and well-scoped** (addresses: Architecture
   responsibility; Correctness loop guard; Code Quality). Isolate the loop in
   its own function; break on null/empty/unchanged `endCursor`; add a
   `MAX_PAGES` cap (mirror `jira-comment-flow.sh`); always emit `nodes` as an
   array; define partial-page-failure (discard vs emit); add a zero-result
   fixture. Record the "under 10,000 points at `first: 50`" invariant.

6. **Escape the Authorization curl-config directive** (addresses: Security
   token-injection). Reject control chars/quotes/newlines in the resolved
   token, or build the header so the raw value is not embedded in a quoted
   config string; test a token containing `"` and `\n`.

7. **Fix the mock-server fidelity gap** (addresses: Test Coverage positional
   ordering; Architecture). Specify that `expect_body_contains` mismatches
   push to `server.errors` (hard fail), and have multi-step scenarios assert
   the operation name per step; capture and assert composed search filters and
   the RATELIMITED-then-200 resume (exit 0, one sleep).

8. **Align conventions and small fixes** (addresses: Standards; Code Quality;
   Correctness). Scope `init-linear` allowed-tools to match `init-jira`; drop
   the `--team` search flag (or reconcile with the exclusion); correct the
   work item's shared-`EXIT_CODES.md` reference; document the omitted 403/404
   mappings; use `config_extract_body` instead of bespoke awk; trim the
   already-synced `work_item_id` before matching; specify case-insensitive,
   ambiguity-rejecting transition state matching; name exit codes as readonly
   constants; hoist the writeback to shared `scripts/` (or note it as
   temporary placement for 0047).

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally disciplined: it mirrors a proven,
well-factored integration (transport / auth / common / flow / SKILL layering
with a strict stdout-body / stderr-error / exit-code contract) and confines
its four divergences to explicitly justified, isolated points. The principal
concern is that folding cursor pagination into the transport overloads
`linear-graphql.sh` with a responsibility the Jira design deliberately kept
in the caller, colliding with the complexity cap, the retry loop, and the
sequenced mock harness. Secondary concerns: the cross-cutting frontmatter
writeback helper lands in the integration-specific common layer, and the
binary-attach PUT bypasses the transport's resilience machinery.

**Strengths**:
- Layering and transport contract mirrored verbatim, preserving thin-flow /
  code-propagation architecture and each verb skill's single reason to change.
- The four divergences are named, scoped, and individually justified up front.
- Per-integration ownership of EXIT_CODES and state dir keeps Linear
  independently extractable; disjoint flow ranges mirror convention.
- Phase boundaries genuinely independently mergeable and dependency-ordered.
- Catalogue-resolved transition is a clean coupling reduction, verifiable by
  stubbing the catalogue endpoint to fail.

**Findings**:
- 🟡 major (high): Folding pagination into the transport gives it two
  responsibilities (single-request transport + multi-request accumulation)
  where Jira keeps pagination caller-side; no stated page bound. (Phase 1 §4)
- 🟡 major (medium): Hard-coded `first: 50` + terminal complexity cap leaves a
  future richer field-set un-paginatable; the under-10,000-points assumption
  is unenforced. (What We're NOT Doing / Phase 1 §4)
- 🟡 major (high): `linear_writeback_work_item_id` is a cross-cutting
  capability placed in `linear-common.sh` rather than shared `scripts/`.
  (Phase 1 §2)
- 🔵 minor (high): Binary-attach PUT bypasses transport retry/timeout/
  connect-mapping, leaving a `fileUpload` asset orphaned on transient failure.
  (Phase 4 §2)
- 🔵 minor (medium): The RATELIMITED-vs-complexity discriminator depends on an
  unstable external string in the transport's dispatch; fail safe to
  non-retried bad-request when neither branch confidently matches. (Phase 1 §4)
- 🔵 minor (medium): Same-endpoint sequenced scenarios couple fixtures to call
  ordering; make `expect_body_contains` a hard match. (Phase 1 §7)

### Code Quality

**Summary**: Unusually well-grounded — mirrors proven Jira shapes verbatim and
isolates its four genuine divergences, keeping the per-skill code readable and
consistent. The main risks cluster in `linear-graphql.sh`, which takes on a
fragile multi-branch error classifier (order-dependent, message-substring
heuristics the plan itself flags as undocumented) on top of newly-folded-in
pagination. Smaller smells: the writeback reinvents body extraction that
`config-common.sh` already provides, and exit-code namespaces are wide enough
to merit self-documenting constants.

**Strengths**:
- Strong consistency with the Jira integration (uniform `*-flow.sh`
  orchestrators, source-guards, `jq -n` conditional-merge, code propagation).
- Divergences explicitly enumerated and bounded.
- Stable, namespaced `E_*` exit codes preserve a testable error contract.
- Test seams designed in from the start, gated on `ACCELERATOR_TEST_MODE=1`.
- Known risks surfaced honestly in Migration Notes.

**Findings**:
- 🟡 major (high): GraphQL error classifier concentrates fragile,
  order-dependent branching in the shared transport; extract into a named
  function with the ordering rationale and a single discriminator constant.
  (Phase 1 §4)
- 🟡 major (medium): Folding pagination into `linear-graphql.sh` gives it two
  responsibilities; isolate the loop and define partial-page-failure. (Phase 1 §4)
- 🔵 minor (high): Create flow reinvents body extraction; use
  `config_extract_body`. (Phase 3 §1 / Phase 1 §2)
- 🔵 minor (medium): Decompose the three-call binary attach into named
  sub-steps with explicit failure propagation. (Phase 4 §2)
- 🔵 minor (low): Wide flat exit-code ranges risk doc/return drift; name codes
  as readonly constants near each flow. (Phase 1 §1)

### Test Coverage

**Summary**: Unusually disciplined — explicitly TDD per phase, maps almost
every acceptance criterion to a named assertion, and calls out the three risky
edge cases with concrete tolerances. The main gaps are test-harness fidelity
(the mock's body/header matching is weak and silent in ways the strongest
assertions depend on) and a few missing negative/error-path cases (RATELIMITED
resume, 200-body auth error, writeback atomicity/failure, lock contention).

**Strengths**:
- Phase-by-phase TDD with red-green discipline; each phase ends green.
- Nearly every acceptance criterion has a corresponding named assertion.
- The three highest-risk edge cases are tested with falsifiable assertions.
- Retry/backoff asserted via file-based sleep counter, never real sleeps.
- Transition cache-resolution tested negatively (stub catalogue endpoint).

**Findings**:
- 🟡 major (high): Sequenced single-endpoint scenarios rely on positional
  ordering the harness can't verify by content; `expect_body_contains` must
  fail on mismatch and scenarios should assert operation name per step. (Phase 1 §7)
- 🟡 major (high): `expect_headers` fails only at server teardown; a SIGKILL'd
  mock can swallow header-echo failures. (Phase 4 §3)
- 🟡 major (medium): No test that a RATELIMITED 400 → 200 resumes (exit 0, one
  sleep). (Phase 1 §4)
- 🔵 minor (medium): `E_GQL_UNAUTHORIZED` via 200-body `errors[]` defined but
  not exercised. (Phase 1 §4)
- 🔵 minor (medium): Writeback failure/atomicity and frontmatter edge cases
  untested. (Phase 3 §1 / Phase 1 §2)
- 🔵 minor (medium): `linear_with_lock` contention and stale-reclaim (53) not
  enumerated. (Phase 1 §2)
- 🔵 minor (medium): Message-substring classifier tested only against
  author-controlled fixtures; add negative controls. (Testing Strategy)
- 🔵 minor (low): Search state-name → catalogue resolution not directly
  asserted; capture the composed filter. (Phase 2 §1)

### Correctness

**Summary**: Logically well-structured and mirrors a proven transport, but the
one genuinely new piece of logic — the HTTP-400 / 200-with-errors classifier —
has several gaps: it omits an authentication-error branch, the backoff has no
upper clamp (and "later of two resets" can produce ~hour sleeps), and the
unified treatment of 200-with-errors and HTTP-400 risks misrouting
non-rate-limit errors into the retry path. The pagination fold, already-synced
regex, and cache-resolved transition are largely sound but have unspecified
edge cases.

**Strengths**:
- Correctly orders complexity before RATELIMITED so the two don't collapse.
- Correct epoch-ms arithmetic with `≥ 0` lower clamp matching the AC.
- bash 3.2 floor respected for the discriminator.
- Already-synced contract correctly distinguishes numeric from remote-format
  and short-circuits before any API call.

**Findings**:
- 🔴 critical (high): Auth-error (200-body `errors[]`) has no classifier branch
  and falls through to `E_GQL_BAD_REQUEST` (34) instead of 11. (Phase 1 §4)
- 🟡 major (high): Rate-limit backoff has only a lower clamp; "later of two
  resets" can yield ~hour sleeps. (Phase 1 §4)
- 🟡 major (medium): Routing 200-with-errors through the retrying RATELIMITED
  branch can retry non-idempotent mutations. (Phase 1 §4)
- 🟡 major (medium): Paginator has no guard against a non-advancing cursor or
  unbounded page count. (Phase 1 §4)
- 🔵 minor (medium): Merged-response synthesis on empty/single-page connections
  unspecified (null vs `[]`). (Phase 1 §4 / Phase 2)
- 🔵 minor (medium): Already-synced regex must trim quotes/whitespace or a
  quoted synced id slips the guard → duplicate create. (Phase 3 §1)
- 🔵 minor (medium): Transition state-name match case-sensitivity, trimming,
  and duplicate-name ambiguity unspecified. (Phase 4 §1)
- 🔵 minor (low): Binary attach has no defined recovery if PUT succeeds but
  attachmentCreate fails. (Phase 4 §2)

### Security

**Summary**: Faithfully inherits the Jira integration's strong
credential-handling controls (token-out-of-argv, 4-tier precedence with
shared-config `token_cmd` ban, 0600 fail-closed gate with symlink rejection,
loopback-only test-mode override) — preserve these verbatim. The net-new
surfaces (binary-upload PUT to a server-returned `uploadUrl` echoing
server-returned headers, and the `work_item_id` writeback of a server-returned
identifier) cross trust boundaries with no Jira precedent and are
under-specified on validation, creating SSRF/credential-leak and
content-injection exposure. The token directive format change also moves the
secret into a context where unescaped quotes/newlines could corrupt the curl
config.

**Strengths**:
- Token never reaches argv (`curl --config -` preserved).
- 4-tier precedence, shared-config `token_cmd` ban, 0600 fail-closed gate with
  symlink rejection ported verbatim.
- Base-URL override gated behind test mode and loopback-only.
- `catalogue.json` correctly assessed non-sensitive; `viewer.json` gitignored.
- `LINEAR_RETRY_SLEEP_FN` name-restricted and test-mode gated.

**Findings**:
- 🔴 critical (medium): Binary-upload PUT to a server-returned `uploadUrl`
  must strip `Authorization` and validate the host — otherwise SSRF /
  token+file exfiltration. (Phase 4 §2)
- 🟡 major (medium): Echoing every server-returned `headers[]` entry into the
  PUT is unbounded header injection. (Phase 4 §2)
- 🟡 major (medium): Hand-built `header = "Authorization: <token>"` directive
  can be corrupted/injected by token contents. (Phase 1 §4)
- 🟡 major (medium): Server-returned identifier written into frontmatter
  without validation. (Phase 1 §2 / Phase 3 §1)
- 🔵 minor (medium): Blanket echo of GraphQL error bodies to stderr may surface
  request-echoed input; emit only `message`/`extensions.code`. (Phase 1 §4)
- 🔵 minor (low): Ensure write SKILL.md gating language covers attach
  file/URL provenance and previews the `uploadUrl` host. (Phase 3 §1 / Phase 4)

### Safety

**Summary**: Inherits a well-proven Jira safety substrate (token-out-of-argv,
atomic JSON writes, mkdir locking with stale reclaim, preview→confirm→send
gates, dry-runs). The two net-new capabilities (the `work_item_id` writeback
that mutates local source files, and the three-step binary attach) introduce
the only genuinely new accidental-harm surfaces, and both are under-specified:
the writeback's atomicity covers the rename but not the awk transformation
that could silently corrupt frontmatter, and the binary attach's
partial-failure semantics are unaddressed. Blast radius is low, so the
local-file mutation deserves stronger integrity guarantees rather than heavy
operational machinery.

**Strengths**:
- Every mutating verb skill carries the preview→confirm→send gate and
  `disable-model-invocation: true`.
- No-API dry-run path on every write flow.
- Already-synced guard prevents accidental duplicate creation.
- State persistence uses atomic writes + locking with stale reclaim.
- Complexity cap treated as terminal with no partial result (no
  silent-truncation hazard).
- Credentials kept out of argv; 0600 gate and `token_cmd` ban preserved.

**Findings**:
- 🔴 critical (high): Awk single-line frontmatter replacer can silently corrupt
  or truncate a human-authored work-item file (no post-transform integrity
  check, unlike `jira_atomic_write_json`'s `jq empty`). (Phase 1 §2 / Phase 3 §1)
- 🟡 major (high): Three-step binary attach has no defined partial-failure or
  cleanup semantics; retries accumulate orphaned assets. (Phase 4 §2)
- 🟡 major (medium): Untrusted server-derived backoff (no upper clamp) and the
  undocumented message-substring discriminator can drive unsafe/unbounded
  retry. (Phase 1 §4 / Migration Notes)
- 🔵 minor (medium): Double-create guard relies solely on local frontmatter,
  not remote verification. (Phase 3 §1)
- 🔵 minor (medium): Local work-item file mutation is outside the state-dir
  lock scope (acceptable at this scale; note it). (Phase 1 §2 / Phase 3 §1)

### Standards

**Summary**: Exceptionally disciplined about mirroring the established Jira
conventions: helper naming, disjoint per-flow exit-code ranges, per-integration
EXIT_CODES ownership, test-seam gating, and read/write SKILL.md archetypes are
all reproduced faithfully. Two genuine inconsistencies stand out: `init-linear`
is assigned the broad write-archetype `allowed-tools` where `init-jira` uses
narrowly-scoped globs, and the work item still describes a non-existent shared
`scripts/EXIT_CODES.md` that the plan silently contradicts.

**Strengths**:
- Exit-code flow ranges reproduced verbatim; per-integration ownership preserved.
- Helper/function naming follows the prefix convention exactly.
- Module filenames avoid underscore-prefixing.
- bash 3.2 floor explicitly honoured in the discriminator.
- Test-seam naming/gating and `E_TEST_*_REJECTED` codes mirror Jira.
- Read vs write SKILL.md archetype split correctly applied.

**Findings**:
- 🟡 major (high): `init-linear/SKILL.md` uses broad `[Bash, Read, Write]`
  where `init-jira` uses narrowly-scoped `Bash(...)` globs. (Phase 1 §5)
- 🔵 minor (high): Work item still references a non-existent shared
  `scripts/EXIT_CODES.md`; correct it alongside the rate-limit patch. (Phase 1 §6)
- 🔵 minor (medium): `search-linear-issues` flow lists a `--team` flag
  contradicting the "no per-invocation `--team`" exclusion. (Phase 2 §1)
- 🔵 minor (medium): Linear transport omits Jira's 403/404/410/429 mappings
  without documenting the rationale. (Phase 1 §1)

### Portability

**Summary**: Inherits a well-engineered cross-platform substrate (POSIX
curl/jq/awk, loopback-gated override, version-guarded Python mock) and
correctly calls out the bash 3.2 floor for the new discriminator. The
principal new risk is the epoch-millisecond backoff arithmetic: obtaining
`now_ms` portably is non-trivial because BSD date (macOS) lacks GNU `%N`, and
the plan doesn't specify how it's computed. Linear is hard-coupled (single
endpoint, vendor-specific error discrimination) — an acknowledged, proportionate
per-vendor choice.

**Strengths**:
- Mirrors `jira-request.sh`'s cross-platform substrate (loopback-only override,
  token-out-of-argv, overridable-only-under-test endpoint).
- bash 3.2 floor explicitly acknowledged; discriminator mandated to avoid
  `${var,,}`.
- Dependencies externalised and version-checked; mock server is test-only,
  Python 3.9+ guarded.
- Configuration injected at runtime; catalogue persisted to a config-driven path.

**Findings**:
- 🔴 major (high): Epoch-millisecond backoff needs a portable `now_ms` —
  `date +%s%N` is GNU-only and emits a literal `N` on BSD/macOS. Use
  second-granularity `date +%s` (the ±2s tolerance permits it). (Phase 1 §4)
- 🔵 minor (medium): Attach file-validation mirror may inherit GNU-only
  `readlink -f`; keep the Jira `|| true` guard, avoid new GNU-only flags.
  (Phase 4 §2)
- 🔵 minor (high): Deep, unabstracted coupling to Linear's GraphQL API and
  proprietary error encoding — record as accepted lock-in; keep the
  discriminator in one classifier function. (Overview / Phase 1 §4)

---

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** REVISE

All 8 lenses were re-run against the revised plan. **Every pass-1 finding —
all 3 criticals and all ~14 majors — is resolved.** The revisions, however,
introduced **4 new major findings** (2 of them direct consequences of the
pass-1 fixes: assertions specified against a test harness that cannot perform
them as inherited). The major-count rule (≥3) holds the pass verdict at REVISE.
**All 4 new majors and the high-value new minors were then addressed in a
follow-up edit pass within this same iteration** (see Assessment).

### Previously Identified Issues (pass 1)

- 🔴 **Safety**: Awk frontmatter replacer can corrupt a work-item file —
  **Resolved** (hoisted to shared `config_set_frontmatter_field` with
  frontmatter-block anchoring, fail-closed on absent/duplicate, re-parse+verify
  before atomic rename, literal value; byte-identical-remainder test)
- 🔴 **Security**: Binary PUT SSRF / token leak — **Resolved** (separate
  no-`Authorization` path, https-only allow-listed host, `E_ATTACH_BAD_UPLOAD_URL`)
- 🔴 **Correctness**: Auth-error (200-body) misrouted to bad-request —
  **Resolved** (auth-first classifier branch + `graphql-auth-error-200` fixture)
- 🟡 **Correctness/Safety/Portability**: Unbounded / non-portable backoff —
  **Resolved** (`[1,60]s` clamp, second-granularity `date +%s`, dropped
  "later-of-two-resets")
- 🟡 **Architecture/Code Quality/Correctness**: Pagination overload + no loop
  guard — **Resolved** (isolated `_linear_paginate`, `MAX_PAGES`, cursor-progress
  break, always-array merge, no-partial-result)
- 🟡 **Code Quality/Architecture/Safety**: Fragile classifier — **Resolved**
  (isolated `_linear_classify_gql_error`, single `LINEAR_COMPLEXITY_PATTERN`
  constant, fail-safe path)
- 🟡 **Correctness**: 200-errors retry routing — **Resolved** (200-body errors
  terminal, never retried)
- 🟡 **Security/Architecture**: Unbounded echoed PUT headers + transport bypass
  — **Resolved** (header allow-list + CR/LF reject; timeout + bounded retry)
- 🟡 **Security**: Authorization directive injection — **Resolved**
  (`E_TOKEN_MALFORMED` token guard across all tiers)
- 🟡 **Security/Safety**: Unvalidated returned identifier — **Resolved**
  (validated against the strict regex before writeback)
- 🟡 **Safety**: Binary attach partial-failure undefined — **Resolved**
  (`E_ATTACH_REGISTER_FAILED`, orphaned-asset contract)
- 🟡 **Architecture**: Writeback in wrong layer — **Resolved** (hoisted to
  shared `scripts/config-common.sh`)
- 🟡 **Architecture**: Fixed `first: 50` invariant — **Resolved** (invariant +
  `--first` evolution path recorded)
- 🟡 **Test Coverage**: Positional-ordering harness — **Resolved**
  (`expect_body_contains` hard-fail)
- 🟡 **Test Coverage**: No RATELIMITED-then-200 resume — **Resolved** (criterion added)
- 🟡 **Standards**: Broad `init-linear` allowed-tools — **Resolved** (scoped to
  match `init-jira`)
- 🔵 All pass-1 minors (—`--team` flag, EXIT_CODES ref, 403/404 docs,
  `config_extract_body`, already-synced trim, transition case-insensitivity,
  readonly constants, stderr echo, attach gating, `readlink -f` guard) —
  **Resolved**

### New Issues Introduced (pass 2)

- 🟡 **Test Coverage**: Inherited `stop_mock` SIGTERMs and discards the exit
  code, so the "assert mock exit status" mechanism never fires — **Addressed**
  (mock writes `--captured-errors-file` on shutdown, asserted empty per scenario)
- 🟡 **Test Coverage**: Mock has no header capture, so the binary-PUT
  no-`Authorization`/dropped-header assertion is impossible — **Addressed**
  (added `captured_headers` to the mock extension)
- 🟡 **Correctness**: `MAX_PAGES`-hit outcome unspecified → silent
  truncation-as-complete — **Addressed** (`truncated: true` marker + `WARN:`;
  `paginate-runaway` fixture)
- 🟡 **Correctness**: No fallback when `X-RateLimit-Requests-Reset` is
  absent/non-numeric → 1s tight loop — **Addressed** (exponential-backoff
  fallback; `ratelimited-400-no-reset-header` fixture)
- 🔵 **Correctness**: `10,?000` could false-positive on a rate-limit message —
  **Addressed** (pattern requires the word `complexity`; other-direction fixture)
- 🔵 **Security**: Wildcard host allow-list — **Addressed** (anchored
  parsed-host match; look-alike-reject fixture; test-mode-gated loopback)
- 🔵 **Correctness**: Hard 0047 sequencing dependency (`work_item_id` field) —
  **Addressed** (made explicit in "What We're NOT Doing")
- 🔵 **Portability**: `bc` undeclared dep on attach path — **Addressed** (format
  with awk)
- 🔵 **Standards/Code Quality**: `readonly` constants diverge from Jira;
  code-27 reassignment; EXIT_CODES drift — **Addressed** (source-of-truth note +
  documented per-namespace divergences)
- 🔵 **Test Coverage**: create-succeeded/writeback-failed untested —
  **Addressed** (criterion + fixture added)
- 🔵 **Code Quality**: binary attach cognitive load — **Addressed** (extracted
  `_linear_upload_asset` helper)

Residual untouched items are low-value optional polish (header-comment
enumerations in `_linear_paginate`, a `config-common.sh` deferred-generalisation
note, printing the one-line manual-edit in the writeback-failed message) — noted
but not blocking.

### Assessment

The plan is now in strong shape. Pass 1's serious issues are fully resolved,
and the 4 new majors pass 2 surfaced — all narrow, mechanical, and two of them
artifacts of the pass-1 fixes — were addressed in the same iteration. As it now
stands the plan has no open criticals or majors and only optional polish
remains. A final confirmation pass could verify the pass-2 follow-up edits, but
the plan is implementation-ready.

---

## Re-Review (Pass 3) — 2026-06-15

**Verdict:** APPROVE

Final confirmation pass — all 8 lenses re-run against the post-pass-2 plan.
**Zero criticals and zero majors across every lens** (Safety and Portability
returned no findings at all). The pass-2 follow-up edits are confirmed sound:
the two pass-2 test-coverage majors (mock exit-code discard; missing header
capture) and the two correctness majors (MAX_PAGES silent-truncation; missing
backoff fallback) are all verified resolved against the actual Jira reference
primitives. Only minor/suggestion-level polish remained, and the substantive
ones were applied in this pass (see below). The plan is sound and ready for
implementation.

### Previously Identified Issues (pass 2 new majors)

- 🟡 **Test Coverage**: `stop_mock` discards mock exit code — **Resolved**
  (verified: `--captured-errors-file` readback replaces the discarded exit)
- 🟡 **Test Coverage**: no PUT header capture — **Resolved** (verified:
  `captured_headers` enables the no-`Authorization`/dropped-header assertions)
- 🟡 **Correctness**: `MAX_PAGES` silent-truncation — **Resolved** (verified:
  `truncated: true` + WARN, matching the `jira-comment-flow.sh` precedent)
- 🟡 **Correctness**: missing backoff fallback — **Resolved** (verified:
  exponential-with-jitter fallback matching `jira-request.sh`)

### New Issues Introduced (pass 3) — minor only, substantive ones applied

- 🔵 **Security**: binary PUT could follow a redirect off the allow-listed host
  → **Applied** (`--max-redirs 0`, `--proto =https`, no `-L`; redirect-bypass
  fixture)
- 🔵 **Security**: signed `uploadUrl`/`assetUrl` query could be logged verbatim
  → **Applied** (redact signed query string in failure messages)
- 🔵 **Test Coverage**: no fixture for the PUT upload-fail/retry path or the
  CR/LF-header value guard → **Applied** (`attach-binary-upload-fail`,
  `attach-binary-crlf-header`, `attach-binary-redirect` fixtures + criteria)
- 🔵 **Correctness**: `complex` stem could over-match; cursor sentinel init
  unstated → **Applied** (pattern requires full word `complexity`;
  previous-cursor sentinel / first-iteration semantics stated)
- 🔵 **Code Quality/Standards**: EXIT_CODES.md hand-sync hazard; code-26 and
  reserved-gap annotations → **Applied** (lightweight constants-vs-table sync
  check + the Jira-matching annotations)

Untouched residuals are pure optional prose-comment suggestions the lenses
themselves marked "no change needed now" (header-comment enumerations in
`_linear_paginate`, a `config-common.sh` deferred-generalisation note,
`captured_headers`-under-lock as an implementation reminder).

### Assessment

The plan is approved. Across three review passes it moved from 3 criticals +
~14 majors → 4 new majors (all consequences of the fixes, all closed) → zero
criticals/majors with only optional polish. The design is sound, the
divergences from the Jira template are each isolated and justified, the test
strategy is proportional and mutation-resistant, and the security/safety
posture is mature. Ready for `/implement-plan`.

---
*Review generated by /accelerator:review-plan*
