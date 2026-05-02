---
date: "2026-05-02T14:00:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-02-jira-integration-phase-2-read-skills.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, security, usability, compatibility]
review_pass: 3
status: complete
---

## Plan Review: Jira Integration Phase 2 — Read Skills

**Verdict:** REVISE

The plan is well-structured, builds cleanly on Phase 1's conventions
(JSON-first stdout, BASH_SOURCE-guarded flow helpers, mock-server
testing, namespaced exit codes), and articulates a sensible six-milestone
TDD path with well-judged scope boundaries. However, the pseudocode
contains several concrete defects — a bash array-splat that doesn't
produce paired arguments, an internal contradiction on
`expand=renderedFields`, and a comments-shape mismatch between M4's
wrapping and M1's walker rules — that would surface as failing tests
during implementation. A second cluster of issues centres on a JQL
composition seam that bypasses Phase 1's safe `jql_compose`/`jql_in`
helpers for skill-only flags (issuetype, component, reporter, parent,
free-text), creating both correctness gaps (negation lost) and a
free-text JQL injection vector. Smaller but coherent themes around the
SKILL-only `--no-render-adf` inversion, page-token validation
strictness, and stale-cache UX appear across multiple lenses and
deserve attention before implementation begins.

### Cross-Cutting Themes

- **JQL post-composition seam in `jira-search-flow.sh`** (flagged by:
  architecture, code-quality, correctness, security) — Skill-only
  flags (`--type`, `--component`, `--reporter`, `--parent`,
  `--free-text`) are appended to the JQL string in the flow helper
  rather than via `jql_compose`. This loses the `~` negation
  convention that works for `--status`/`--label`/`--assignee`,
  bypasses `jql_quote_value` for any future hand-rolled `text ~ "…"`
  composition, and leaks JQL-assembly responsibility into the flow
  layer. The pseudocode array-splat (`"${assignee_vals[@]/#/--assignee
  }"`) further compounds this — it does not produce the paired
  arguments `jql_compose` expects.
- **`--no-render-adf` inversion lives only in SKILL prose**
  (architecture, code-quality, test-coverage, usability) — The plan
  states load-bearing logic must live in bash, but the M5 default-on
  inversion is implemented in SKILL prose. This makes the
  user-visible default untestable, asymmetric vs. helper users, and
  fragile across `skill-creator` regenerations.
- **`--page-token` validation regex** (architecture, code-quality,
  correctness, compatibility) — The proposed
  `^[A-Za-z0-9._~+/=-]+$` rejects characters Atlassian's opaque
  token format may use and admits some it doesn't, with no concrete
  attack mitigated. Loosen to a control-character/whitespace check
  or rely on JSON serialisation.
- **Default `expand=renderedFields` self-contradiction**
  (correctness/critical, compatibility/major) — M4 outline sets it
  as the default; "What We're NOT Doing" states it is not requested.
  These cannot both be true; M4 case 1 codifies the contradiction.
- **Auto-trigger / `disable-model-invocation: false` policy**
  (architecture, compatibility, usability) — Divergence from Phase 1
  precedent without a written policy rule, plus deferred trigger
  evals, leave the central UX surface unverified at ship time.
- **Stale `fields.json` cache UX** (test-coverage, usability) — Users
  upgrading after Phase 2 lands silently miss custom-textarea
  rendering with no in-tool signal pointing at `/init-jira
  --refresh-fields`. The `schema.system == "textarea"` selector also
  appears to be the wrong heuristic — Atlassian carries textarea
  identity on `schema.custom`, not `schema.system`.

### Tradeoff Analysis

- **Library extension vs. skill-only translation in `jql_compose`** —
  The plan keeps `jql_compose` minimal and lets `jira-search-flow.sh`
  translate skill-specific flags. Architecture and Correctness lenses
  argue for extending the library (single source of truth for
  escaping/negation); the plan author argued the library should not
  encode skill-level semantics. **Recommendation**: extend
  `jql_compose` for the multi-value IN/NOT-IN flags (type, component,
  reporter, parent), keep "skill knows the semantics" only for shape
  conversions (free-text → `text ~`, watching → `watcher = …`). The
  multi-value pattern is exactly what the library already models.

### Findings

#### Critical

- 🔴 **Code Quality**: Array-into-flag-pair expansion does not produce paired arguments
  **Location**: Phase 2.M2 §3 jira-search-flow.sh (`jql_compose` call)
  The pseudocode `"${assignee_vals[@]/#/--assignee }"` produces single
  tokens like `--assignee redacted-id`, not the two-token pair
  `jql_compose`'s case statement requires. M2 cases 1, 3, 11, 12 will
  fail in implementation. Replace with explicit accumulation loops
  building an array of paired flags.

- 🔴 **Correctness**: Default `expand=renderedFields` contradicts the explicit out-of-scope statement
  **Location**: Phase 2.M4 §3 outline + "What We're NOT Doing"
  M4 sets `expand="renderedFields,names,schema,transitions"` as default
  while "What We're NOT Doing" states `renderedFields` is not requested.
  M4 case 1 codifies the contradiction. Remove `renderedFields` from
  the default and align test expectations.

- 🔴 **Correctness**: Comments JSON shape after `--comments N` does not match the walker's case-4 path
  **Location**: M4 §3 (comments wrapping) ↔ M1 cases 4 & 12
  M4 wraps the offset response under `{comments: $c}` producing
  `issue.comments.comments[].body`, while M1 case 4 says the walker
  handles `comments.comments[]` from `expand=comments` (which on
  Atlassian is actually `fields.comment.comments[]`). The two cases
  test different paths than M4 produces; comments may not render in
  real responses. Pick one canonical shape and align both sides.

#### Major

- 🟡 **Code Quality**: `schema.system == "textarea"` is the wrong heuristic for ADF-bearing custom fields
  **Location**: M1 §1 case 6 + §2 custom_paths build
  In Atlassian responses, `schema.system` is the system field name and
  is null for custom fields; textarea identity is on `schema.custom ==
  "com.atlassian.jira.plugin.system.customfieldtypes:textarea"`. The
  current selector either over-matches every string custom field or
  never matches the intended branch. Rewrite the selector and the
  fields.sh refresh widening accordingly.

- 🟡 **Architecture**: JQL clause post-composition leaks escaping concerns into the flow helper
  **Location**: M2 §3 + Key Discoveries
  Skill-only flags appended via string concatenation create two paths
  for clause assembly — one in the library, one outside it. Negation
  and escaping correctness now span two modules. Extend `jql_compose`
  with the multi-value flags using the existing `_jql_compose_field`
  pattern; keep skill-side translation only for shape conversion.

- 🟡 **Architecture / Code Quality**: `--no-render-adf` inversion in SKILL prose
  **Location**: M5 §1 step 2; convention notes
  The user-visible default is realised in LLM-interpreted prose, not
  bash. Untestable, drifts on `skill-creator` regeneration, and creates
  helper/SKILL asymmetry. Add `--no-render-adf` (and matching test) to
  `jira-show-flow.sh` itself; let the SKILL forward whichever flag the
  user provides.

- 🟡 **Correctness**: Skill-only flags bypass `jql_compose`'s negation handling
  **Location**: M2 §3
  `--type ~Bug` would produce `issuetype IN ('~Bug')` (literal value
  `~Bug`), not `issuetype NOT IN ('Bug')`. Negation works for
  status/label/assignee but silently breaks for type/component/
  reporter/parent. Either extend `jql_compose` (preferred) or split
  via `jql_split_neg` and call `jql_in`/`jql_not_in` accordingly.

- 🟡 **Security**: Free-text and skill-only fields composed without `jql_quote_value`
  **Location**: M2 §3 (`text ~ "$value"` not yet specified)
  `jql_match`/`jql_text` helpers do not exist; the search-flow author
  will likely write `text ~ "$value"` by hand, opening JQL injection
  via prompt-fed free-text values (e.g. `foo" OR project = SECRET OR
  text ~ "bar`). Add `jql_match` to `jira-jql.sh` with proper double-
  quote escaping; add a test case asserting the value is treated as a
  single literal.

- 🟡 **Security**: `--jql` raw escape hatch trust boundary undocumented
  **Location**: M2 §3 + M3 SKILL frontmatter
  `disable-model-invocation: false` lets the LLM synthesise `--jql`
  from prompt context, which can broaden reads (`OR project = SECRET`)
  via indirect prompt injection. Document the trust boundary
  explicitly: `--jql` is operator-trusted; the LLM must not synthesise
  it from untrusted ticket/file content. Optionally echo composed JQL
  to stderr for visibility.

- 🟡 **Code Quality**: `local x=$(...)` exit-code propagation footgun
  **Location**: M2 §3 + M4 §3
  `local foo=$(cmd) || return $?` silently swallows the inner exit
  because `local` returns 0. The plan currently uses the safe two-line
  form, but a future "tidy-up" refactor would break exit-code
  propagation across both helpers. Add an explanatory comment near the
  first occurrence in each helper, matching the precedent in
  `jira-init-flow.sh`.

- 🟡 **Code Quality**: `_is_known_short_field` referenced but undefined; primitive-obsession smell
  **Location**: M2 §3 `_jira_search_resolve_field`
  Hard-coding "standard fields" via an undefined helper is a
  maintenance hazard. Drop the special case; resolve every
  non-`customfield_NNNNN` token via `jira-fields.sh resolve` and fall
  back to the literal on exit 50.

- 🟡 **Correctness**: `--page-token` regex may reject valid Atlassian tokens
  **Location**: M2 §3 + Compatibility cross-cutting
  Atlassian's `nextPageToken` is opaque; the chosen alphabet is a
  guess. A future Atlassian token-format change silently breaks
  pagination at the validator. Reduce to a control-character /
  whitespace check, or remove validation and rely on JSON
  serialisation.

- 🟡 **Correctness**: Hard-coded `expand: "names,schema"` in search body unmotivated
  **Location**: M2 §3 jq body builder
  The walker reads schema from `fields.json`, not response schema, so
  the search-body expansion is load-bearing for nothing the plan
  describes. Either drop it or document why it's necessary.

- 🟡 **Correctness**: `--free-text` translation and escaping not specified
  **Location**: M2 §3 + M3 argument-hint
  Mentioned as a flag but no JQL composition shown. Specify the
  operator (`text ~ "value"`) and require `jql_match`-style escaping.
  Add a test case asserting `--free-text 'has"quote'` is escaped per
  JQL string-literal rules.

- 🟡 **Correctness**: Field resolver short-circuit ambiguity
  **Location**: M2 case 9 + §3
  Without a defined `_is_known_short_field`, implementer must invent
  semantics. Drop the short-circuit; route everything through resolve
  with literal fallback.

- 🟡 **Correctness**: Null/non-object detection at ADF paths under-specified
  **Location**: M1 §2 idempotency
  The "skip non-objects" rule is too vague for the implementer. State
  the gate explicitly: render only when `(getpath(p) | type) ==
  "object"` and the value has `.type == "doc"`. Document the invariant
  so case 11's idempotency follows from the type predicate.

- 🟡 **Test Coverage**: ADF walker fixtures collapse to a single "hello world" paragraph
  **Location**: M1 cases 1–7
  Every positive case appears to use the same flat ADF doc. Subtree
  extraction bugs would be invisible against this fixture. Add at
  least three richer ADF fixtures (multi-paragraph + heading + list,
  inline marks, code block) and assert structural markers in the
  rendered Markdown.

- 🟡 **Test Coverage**: Search tests miss empty issues, last-page absence-of-token, double-pagination round-trip, free-text JQL injection
  **Location**: M2 cases 1–16
  Several high-value paths missing. Add cases for empty `issues: []`,
  last-page semantics (no `nextPageToken`), two-call pagination
  round-trip in one test, `--free-text` with quote-bearing input, and
  `--jql` combined with structured flags.

- 🟡 **Test Coverage**: Show tests miss empty-comments, mixed-content render, auth failures, `--expand` × `--fields` interaction
  **Location**: M4 cases 1–12
  Add cases for `--comments 5` returning empty array, full mixed
  rendering (description + custom textarea + comments), 401/403
  propagation, and `--render-adf` with `--fields` excluding
  description.

- 🟡 **Test Coverage**: Trigger evals deferred for two auto-invoking skills
  **Location**: M3 + M5 "Eval scaffolding (omitted)"
  Auto-trigger is the central UX of these skills; deferring evals
  leaves it unverified. Add minimal `evals.json` (3–5 phrases per
  skill) at ship time, or commit to a hard date in "What We're NOT
  Doing".

- 🟡 **Test Coverage**: Idempotency test asserts at process boundary, not the per-path short-circuit
  **Location**: M1 case 11
  A walker that re-renders strings via lenient parse would still
  pass. Strengthen with an instrumented variant that asserts the
  second invocation does not spawn `jira-adf-to-md.sh`, or interleave
  already-rendered strings with ADF subtrees in the input.

- 🟡 **Usability**: Error messages on `--limit`/`--page-token` don't surface the constraint
  **Location**: M2 validation
  `E_SEARCH_BAD_LIMIT` alone tells the user nothing about the cap or
  the page-token escape. Specify friendly stderr text and assert it
  in case 8.

- 🟡 **Usability**: 13+ flag argument-hint with no `--help` story
  **Location**: M3 frontmatter
  No usage banner, no `-h`/`--help` output, no in-tool affordance for
  flag discovery. Add `--help` to both flow helpers; print usage on
  E_SEARCH_BAD_FLAG / E_SHOW_BAD_FLAG.

- 🟡 **Usability**: Unknown `--fields` slugs pass through to Jira silently
  **Location**: M2 case 9 + Migration Notes
  Stale `fields.json` produces a Jira 400 with no hint that
  `/init-jira --refresh-fields` would help. Emit a stderr warning on
  resolve fall-through and consider a dedicated exit code.

- 🟡 **Usability**: No in-tool signal that `fields.json` cache is stale post-upgrade
  **Location**: Migration Notes
  Custom textareas silently fail to render with no indication. Emit a
  one-line stderr hint when `fields.json` predates the schema
  widening.

- 🟡 **Compatibility**: Hard-coded `pageToken` regex risks rejecting future Atlassian alphabets
  **Location**: M2 page-token validation
  Same concern as Correctness; framed here as forward-compat. Document
  the assumption or relax validation.

- 🟡 **Compatibility**: Default `expand=renderedFields` couples to a feature Atlassian is steering away from
  **Location**: M4 default expand
  Atlassian's direction is native ADF, not server-rendered HTML. With
  the contradiction resolved, drop `renderedFields` from the default
  to avoid forward-compat exposure.

#### Minor

- 🔵 **Architecture**: Per-path subprocess spawn in walker vs. single jq pass — acceptable for Phase 2 but make path-set data-driven so a future single-pass refactor is non-breaking.
- 🔵 **Architecture**: Auto-invocation policy diverges from Phase 1 without a written rule — add an "Invocation policy" subsection naming the rule (read-only auto-invokes; mutating/setup are slash-only).
- 🔵 **Architecture**: Read-only contract on `site.json`/`fields.json` is conventional, not enforceable — split `jira-common.sh` into read core + writer module, or document the contract in EXIT_CODES.md.
- 🔵 **Architecture**: `fields.json` schema widening lacks an explicit milestone owner — promote to M0 or list as an M1 sub-deliverable with `test-jira-fields.sh` updates.
- 🔵 **Architecture**: `--page-token` validation duplicates trust logic — reduce to JSON-body safety only.
- 🔵 **Code Quality**: `--page-token` regex loose enough to admit `+`/`/` while tight enough to reject `:`/`*` — pick a defined safety property.
- 🔵 **Code Quality**: EXIT_CODES.md edited in both M2 and M4 — sequence strictly, land all blocks in M1, or split into per-helper sections.
- 🔵 **Code Quality**: Bang-prefix preprocessor escape ambiguity in plan prose — reference `init-jira/SKILL.md` lines verbatim instead of describing the syntax.
- 🔵 **Code Quality**: `--no-render-adf` flag inversion in SKILL prose — minor restated for completeness; promote helper-side flag as suggested in major findings.
- 🔵 **Test Coverage**: Worklog and other ADF-bearing fields not covered as named cases — explicitly out-of-scope or add a 13th case.
- 🔵 **Test Coverage**: `--fields` resolution failure paths uncovered — add tests for bogus slug pass-through and missing `fields.json`.
- 🔵 **Test Coverage**: No systematic stderr-cleanliness sweep — add `assert_stderr_empty` to one happy-path test per flow.
- 🔵 **Test Coverage**: No explicit assertion that mock recorded zero unexpected requests for single-call tests — expose the mock's consumed-expectation count.
- 🔵 **Test Coverage**: Scenario fixtures share a flat directory — namespace by helper or add fixture-purpose comment headers.
- 🔵 **Test Coverage**: Skill-level `--no-render-adf` inversion is untested — extract to bash wrapper or accept the gap explicitly.
- 🔵 **Correctness**: `--free-text` translation/escaping unspecified — minor restated; specify the operator and escaping rule.
- 🔵 **Correctness**: `*all` token semantics + URL-encode pass-through unverified — add a test asserting `fields=*all` appears literally in the URL.
- 🔵 **Correctness**: `--limit` reject-vs-clamp choice undocumented — clamp with warning or document the cap in the SKILL description.
- 🔵 **Correctness**: Per-path subprocess fan-out makes 200 ms claim optimistic — update perf section with realistic numbers and add a partial-failure test.
- 🔵 **Correctness**: Walker test-seam `ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST` not shown in pseudocode — surface it explicitly.
- 🔵 **Security**: `accountId` substituted into JQL without value-shape validation — add a single-line regex assertion in `_jira_search_resolve_me` rejecting non-`[A-Za-z0-9:_-]+`.
- 🔵 **Security**: Test-seam env-var gate semantics not specified — pin to strict equality (`= "1"`) and add a negative test.
- 🔵 **Security**: ADF-rendered Markdown carries untrusted link/code content downstream — add a manual-test case for `[click](javascript:…)` and consider rejecting non-`http(s)`/`mailto` link schemes in `jira-adf-render.jq`.
- 🔵 **Security**: `site.json`/`fields.json` on-disk trust model not documented — add an explicit note about VCS posture and umask expectations.
- 🔵 **Security**: Composed JQL surfaces in stderr — note the trade-off for shared CI captures.
- 🔵 **Security**: M4 issue-key path traversal claim is correct but the rationale (regex + `..` check + iterative URL-decode) should be recorded — strengthen the M4 narrative and add a percent-encoded traversal test.
- 🔵 **Usability**: `--fields` is comma-separated while other multi-value flags repeat — accept both forms or pick one.
- 🔵 **Usability**: Pagination teaching is thin — add a worked example to the M3 SKILL body.
- 🔵 **Usability**: Bare-key auto-trigger may over-fire in code-review contexts — soften the M5 description.
- 🔵 **Usability**: Manual smoke checklist tests only happy paths — add steps 11–15 covering credential/network/flag failure paths.
- 🔵 **Usability**: JSON-on-stdout assertions don't cover stderr cleanliness or `set -x` traces — strengthen cases 16/12.
- 🔵 **Usability**: Negation `~` convention not taught in SKILL body — add a one-line note.
- 🔵 **Compatibility**: `*all` and default-fields semantics depend on undocumented Atlassian behaviour — document the assumption.
- 🔵 **Compatibility**: `disable-model-invocation: false` is unverified in this codebase — confirm via smoke test.
- 🔵 **Compatibility**: Field-resolver pass-through couples to Jira's error shape — choose an explicit local error or document pass-through.
- 🔵 **Compatibility**: Walker handles two comment shapes by structural inference — add a fallback warning when neither shape matches.
- 🔵 **Compatibility**: No `cacheVersion` on `fields.json` — add a version sibling for forward-compat probing.
- 🔵 **Compatibility**: case…esac flag parser will grow non-trivially in Phase 3 — document a flag-parsing contract in the helper headers.

### Strengths

- ✅ Clear SKILL → flow-helper → primitives layering with a sound dependency graph (M1 → M2/M4 → M3/M5 → M6).
- ✅ Single-implementation-site rule for `--render-adf` keeps both flow helpers thin and consistent.
- ✅ Phase 1 `jql_quote_value` properly rejects control chars and escapes single quotes — inherited safe quoting whenever flags route through `jql_compose`/`jql_in`.
- ✅ `jira-request.sh` already enforces layered path defence (regex + explicit `..` check + iterative URL-decode), making M4 case 11 sound.
- ✅ Test-seam env vars gated by `ACCELERATOR_TEST_MODE=1`; mock server fails fast on unexpected requests.
- ✅ Exit-code allocation extends an explicit, owned namespace (70–99) without overlap.
- ✅ Pagination uses opaque `nextPageToken` round-tripping; no `total` synthesis.
- ✅ JSON-first stdout / stderr-for-status discipline with at least one explicit test per helper.
- ✅ Schema widening of `fields.json` is genuinely additive with documented graceful degradation.
- ✅ Idempotency identified as an architectural property (M1 case 11) — uncommon and valuable.
- ✅ Decision to fold comments into `show-jira-issue` rather than mirror the REST endpoint is intentional API surface design.
- ✅ Manual smoke checklist (M6) is thorough on happy paths and exercises both auto-invocation triggers.

### Recommended Changes

Ordered by impact:

1. **Resolve the `expand=renderedFields` contradiction** (addresses:
   "Default `expand=renderedFields` contradicts" critical and the
   matching compatibility finding). Remove `renderedFields` from M4's
   default `expand` and update M4 case 1's expected request.

2. **Fix the comments shape mismatch end-to-end** (addresses: comments
   shape critical). Pick one canonical merged shape — recommended:
   emit `{comments: [...], pagination: {total, startAt, maxResults}}`
   from M4 with the array directly under `issue.comments`. Update M1
   walker rules and the desired-end-state §1.2 text accordingly.

3. **Replace pseudocode array splats with explicit accumulation
   loops** (addresses: array-into-flag-pair critical and the
   correctness finding on the same defect). Rewrite the M2 §3
   pseudocode using `for v in "${status_vals[@]}"; do compose_args+=(--status "$v"); done`.

4. **Extend `jql_compose` with the multi-value skill flags**
   (addresses: architecture leak, correctness negation, security
   free-text injection, compatibility flag-parser growth). Add
   `--type`, `--component`, `--reporter`, `--parent` to
   `jql_compose` using `_jql_compose_field`; add a `jql_match` (or
   `jql_text`) helper for free-text with proper double-quote
   escaping; route `--free-text` through it.

5. **Rewrite the custom-textarea selector against `schema.custom`**
   (addresses: schema.system heuristic major). Update both the M1
   walker query and the M1 widening of `jira-fields.sh refresh` to
   capture and select on `schema.custom`. Verify against a real
   `/rest/api/3/field` response in M1 manual verification.

6. **Promote `--no-render-adf` into `jira-show-flow.sh`** (addresses:
   architecture/code-quality/test-coverage/usability cross-cutting
   theme). Helper accepts `--no-render-adf`, defaults render-on; SKILL
   prose forwards whichever flag the user provided. Add an M4 test
   asserting the helper-level default.

7. **Loosen `--page-token` validation** (addresses: architecture,
   code-quality, correctness, compatibility on the same regex).
   Reject only control characters and whitespace; rely on JSON
   serialisation for the rest. Document the change.

8. **Specify the ADF idempotency oracle precisely** (addresses:
   correctness null/non-object major and test-coverage idempotency
   major). Render only when `(getpath(p) | type) == "object"` and
   `.type == "doc"`. Surface the test-seam env var
   (`ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST`) explicitly in the
   pseudocode. Strengthen case 11 with an instrumented "did not
   re-spawn the renderer" assertion.

9. **Add a milestone or M0 sub-deliverable for `jira-fields.sh
   refresh` widening** (addresses: architecture milestone-owner
   minor + test-coverage). Either promote to M0 sequenced before M1,
   or list explicitly under M1 with checkboxes for `test-jira-fields.sh`
   updates.

10. **Document the `--jql` trust boundary** (addresses: security
    major on `--jql` raw passthrough). Add a "Trust boundary" note
    to both SKILL prose and the plan's convention notes: `--jql` is
    operator-trusted; the LLM must not synthesise it from untrusted
    content. Echo composed JQL to stderr at INFO.

11. **Add empty/last-page/round-trip/free-text/auth-failure tests**
    (addresses: test-coverage majors on M2 and M4). Add cases listed
    in those findings.

12. **Add minimal trigger evals for M3/M5** (addresses: test-coverage
    deferred-evals + usability + architecture invocation policy).
    Even 3–5 phrases per skill catches description regressions.

13. **Improve error messages and add `--help`** (addresses:
    usability error-message and discoverability majors). Friendly
    stderr text on validation errors; `--help`/`-h` printing flag
    set on both flow helpers; print usage on bad-flag exits.

14. **Add stale-cache stderr hint** (addresses: usability +
    test-coverage on `fields.json` migration). Walker emits a hint
    when `fields.json` predates the schema widening, pointing at
    `/init-jira --refresh-fields`.

15. **Capture an explicit invocation policy** (addresses:
    architecture + compatibility + usability on
    `disable-model-invocation: false`). Add a short subsection to
    the plan or to a Phase 1 conventions doc: read-only skills
    auto-invoke; mutating/setup skills are slash-only.

16. **Document migration UX, on-disk trust model, and add a
    `cacheVersion` field** (addresses: minor compatibility +
    security). Adds one paragraph to Migration Notes and one
    integer to the cache.

The remaining minor findings can be addressed inline during
implementation without further plan revision.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-aligned with the Phase 1 architecture — thin
SKILL → flow-helper → primitives layering, JSON-first I/O, sourceable+CLI
hybrid scripts, read-only consumption of cached state. Most boundaries are
coherent. However, several architectural choices weaken cohesion: the flow
helper hand-composes JQL clauses outside `jql_compose` (drifting escape
responsibility into search-flow), `--no-render-adf` lives as inversion
logic in SKILL prose contradicting the plan's own load-bearing-logic-in-bash
principle, and the read-only contract on `site.json`/`fields.json` is
asserted in prose but not enforced structurally.

**Findings**: 2 major, 5 minor — captured in main review above.

### Code Quality

**Summary**: The plan reuses Phase 1 conventions cleanly and is well-scoped.
However, the M2 §3 pseudocode contains several concrete bash anti-patterns
that will not work as written (array-into-flag-pair word-splitting,
conditional flag injection via positional command substitution,
`local foo=$(...)` exit-code masking risk), and the M1 fields.json query
relies on a heuristic about `schema.system` that does not match how
Atlassian populates that property. Smaller issues — undefined
`_is_known_short_field`, ambiguous idempotency oracle, EXIT_CODES.md merge
coordination — should be tightened before implementation.

**Findings**: 1 critical, 3 major, 5 minor.

### Test Coverage

**Summary**: The plan adopts strict TDD with per-helper test scripts,
mock HTTP via the existing `mock-jira-server.py`, and an umbrella runner
— well-aligned with Phase 1 conventions. However, the proposed M1
test cases are dangerously thin on ADF node coverage (essentially a
'hello world' single-paragraph fixture), several important error/edge
paths in M2/M4 are uncovered, and the deferral of trigger evals for two
auto-invoking skills leaves a core UX behaviour unverified.

**Findings**: 4 major, 7 minor.

### Correctness

**Summary**: The plan has solid happy-path correctness coverage but
several pseudocode-level concerns indicate logical gaps: a
self-contradiction on `expand=renderedFields`, a comments JSON-shape
mismatch between M1 walker expectations and M4's wrapping logic,
ungated negation for skill-only flags, and ambiguous handling of
`null` ADF values during idempotency. None are unfixable, but enough
are load-bearing that several test cases as written would fail before
the implementation is corrected.

**Findings**: 2 critical, 6 major, 5 minor.

### Security

**Summary**: Phase 2 inherits a strong Phase 1 security foundation:
`jql_quote_value` rejects control characters and properly escapes
single quotes; `jira-request.sh` performs path validation with iterative
URL-decoded `..` traversal checks; the test-mode override is gated. The
most material gaps are around the `--jql` raw escape hatch trust
boundary, the search-flow's planned JQL composition for new fields
(which bypasses the safe `jql_compose` API), and a couple of test-seam
/ cache trust-model details that should be documented before
implementation.

**Findings**: 2 major, 6 minor.

### Usability

**Summary**: The plan delivers a sensible read-side flow with thoughtful
defaults (`--render-adf` on for show, off for search) and an idempotent
ADF walker. However, several DX rough edges remain: a 13+ flag search
surface with no `--help` story, an asymmetric `--no-render-adf` that
exists only at the SKILL layer, silent migration UX for stale
`fields.json` caches, friendly-slug pass-through that defers errors to
Jira, and pagination/limit error messages that don't tell the user the
actual constraint.

**Findings**: 5 major, 6 minor.

### Compatibility

**Summary**: The plan is largely additive and respects Phase 1's
contracts. The most material compatibility risks are external — reliance
on the still-evolving Atlassian `POST /search/jql` token-based pagination
contract, an in-helper regex on opaque page tokens, and a default
`expand=renderedFields` that depends on a feature Atlassian is steering
customers away from. Internal compatibility (skill discovery, frontmatter
conventions, test fixture format, exit-code ranges) is well aligned with
Phase 1 precedent.

**Findings**: 2 major, 6 minor.

---

## Re-Review (Pass 2) — 2026-05-02T13:30:00Z

**Verdict:** REVISE

The plan revision substantively addressed almost every previous
finding: all 3 criticals from pass 1 are resolved, the JQL
composition seam is properly consolidated in `jql_compose`, the
comments shape is unified end-to-end, the `--no-render-adf`
inversion is now a real helper flag, the page-token validator
is loosened, the ADF idempotency oracle has an explicit type
predicate, and both auto-invoking SKILLs ship minimal trigger
evals. Of 68 prior findings, 45 (66%) are fully resolved, 11
partially resolved, and 12 still present. The remaining
still-present items are dominated by test-coverage breadth
issues (richer ADF fixtures, idempotency at process boundary,
worklog/auth-failure/mixed-content cases) and three minor
security/usability items that were judged less critical to
ship-block (accountId regex validation, Markdown link-scheme
filtering, manual smoke failure paths).

The revision introduced 21 new findings — most are minor
documentation-or-test-shape items, but three are majors worth
flagging:
1. **Correctness/MAJOR** — The `fields_array` JSON literal in
   `jira-search-flow.sh` is built by string concatenation
   (`fields_array+="\"$resolved\""`); a stale-cache fall-through
   token containing `\` or `"` produces malformed JSON.
2. **Test Coverage/MAJOR** — The new client-side comment-slice
   logic has only one test case (8→5); edge cases (N>length,
   empty array, missing `fields.comment`, missing `.created`)
   are uncovered.
3. **Test Coverage/MAJOR** — The fields-cache widening has only
   2 test cases; behaviour for non-textarea `schema.custom`
   values (e.g. `:textfield`, `:float`) is unspecified.

The verdict remains REVISE because the new majors are
concretely fixable (one pseudocode change + a handful of test
cases) and ≥3 majors still trip the configured threshold.
Functionally the plan is much closer to ship-ready: a third
revision pass focused only on these three majors (plus optional
adoption of any partial-resolution items the user wants to
upgrade) would land the plan at COMMENT.

### Previously Identified Issues

#### Architecture (7 findings)
- ✅ **Architecture**: JQL clause post-composition leaks escaping concerns into the flow helper — Resolved (M2 §3 extends `jql_compose`; flow helper is a strict translator)
- ✅ **Architecture**: `--no-render-adf` inversion lives in SKILL prose — Resolved (M4 §3 + case 13; M5 prose simplified)
- ✅ **Architecture**: Per-path subprocess spawn vs single jq pass — Resolved (Performance section documents the tradeoff and Phase-3 optimisation path)
- ✅ **Architecture**: Auto-invocation policy diverges without anchor — Resolved (Invocation policy in convention notes; M3/M5 reference it)
- 🟡 **Architecture**: Read-only contract conventional, not enforceable — Partially resolved (acknowledged as conventional; no structural split)
- ✅ **Architecture**: Test-fixture scaffolding for textarea schemas implicit — Resolved (M1 §1 sub-deliverable with explicit test additions)
- ✅ **Architecture**: `--page-token` validation duplicates input-trust logic — Resolved (validator scoped to JSON-body safety only)

#### Code Quality (9 findings)
- ✅ All 9 resolved (array splat, schema heuristic, `local x=$()` trap, `_is_known_short_field`, idempotency oracle, page-token regex, EXIT_CODES coordination, bang-prefix escapes, `--no-render-adf`)

#### Test Coverage (11 findings)
- 🔴 **Test Coverage**: ADF walker fixtures collapse to "hello world" — Still present (no multi-paragraph/list/marks/code-block fixtures added)
- ✅ **Test Coverage**: Search tests miss empty/last-page/round-trip/JQL injection — Resolved
- ✅ **Test Coverage**: Trigger evals deferred — Resolved (5 cases per skill)
- 🟡 **Test Coverage**: Show-flow tests miss empty-comments/auth/mixed-content/`--expand`×`--fields` — Partially resolved (auth, mixed-content, empty-comments, --expand×--fields still uncovered)
- 🔴 **Test Coverage**: Idempotency at process boundary only — Still present (no instrumented "didn't re-spawn" check)
- 🔴 **Test Coverage**: Worklog and other ADF-bearing fields not covered — Still present (no case, no out-of-scope statement)
- 🟡 **Test Coverage**: `--fields` resolution failure path — Partially resolved (cache-miss covered; missing-fields.json untested)
- 🔴 **Test Coverage**: No systematic stderr-cleanliness sweep — Still present
- 🟡 **Test Coverage**: No assertion mock recorded zero unexpected requests — Partially resolved (added to M2 case 1 only)
- 🔴 **Test Coverage**: Scenario fixtures shared across tests — Still present (flat directory unchanged)
- ✅ **Test Coverage**: Skill-level inversion of `--render-adf` untested — Resolved (M4 case 13)

#### Correctness (13 findings)
- ✅ **Correctness**: Default `expand=renderedFields` contradicts out-of-scope — Resolved
- ✅ **Correctness**: Comments JSON shape mismatch — Resolved (single embedded shape; client-side slice)
- ✅ **Correctness**: Skill-only flags bypass `jql_compose` negation — Resolved (extended library)
- ✅ **Correctness**: Array splat export semantics unstated — Resolved (explicit accumulation loops)
- 🔴 **Correctness**: Hard-coded `expand: "names,schema"` in search body unmotivated — Still present
- ✅ **Correctness**: `--page-token` regex may reject valid tokens — Resolved
- ✅ **Correctness**: Null/non-object detection at ADF paths — Resolved (explicit type predicate)
- ✅ **Correctness**: Field resolver short-circuit ambiguity — Resolved (`_is_known_short_field` removed)
- ✅ **Correctness**: `--free-text` translation/escaping unspecified — Resolved (`jql_match` helper with explicit escape order)
- ✅ **Correctness**: `*all` token + URL-encoding unverified — Resolved (M4 case 2 records URL)
- ✅ **Correctness**: 100 vs 5000 reject vs clamp — Resolved (documented + remediation message)
- ✅ **Correctness**: Per-path subprocess fan-out latency — Resolved (perf section updated)
- ✅ **Correctness**: Walker state_dir lookup — Resolved (test seam in pseudocode)

#### Security (8 findings)
- ✅ **Security**: New JQL fields bypass safe quoting — Resolved (`jql_match` + injection test)
- ✅ **Security**: `--jql` raw passthrough trust boundary — Resolved (3-level enforcement)
- 🔴 **Security**: `accountId` substituted into JQL unvalidated — Still present (no regex assertion in `_jira_search_resolve_me`)
- ✅ **Security**: Test-seam env-var gate semantics — Resolved (strict equality + regression test)
- 🔴 **Security**: ADF Markdown carries untrusted link/code content — Still present (no link-scheme filtering)
- 🔴 **Security**: Trust model for site.json/fields.json undocumented — Still present
- 🟡 **Security**: Composed JQL surfaces in stderr (CI logs) — Partially resolved (echo added; redaction tradeoff not noted)
- ✅ **Security**: M4 case 11 traversal rationale — Resolved (layered defence documented; percent-encoded test)

#### Usability (12 findings)
- ✅ **Usability**: `--no-render-adf` only at SKILL layer — Resolved
- ✅ **Usability**: Error messages don't surface constraint — Resolved
- ✅ **Usability**: 13+ flag with no `--help` — Resolved (banner with full flag set)
- ✅ **Usability**: Unknown `--fields` slugs pass silently — Resolved (stderr warning)
- 🟡 **Usability**: No in-tool stale `fields.json` signal — Partially resolved (warning only on unknown-token path)
- ✅ **Usability**: `--fields` CSV vs repeatable inconsistency — Resolved (both forms accepted)
- 🔴 **Usability**: Pagination teaching thin — Still present (no worked round-trip example in M3 SKILL prose)
- 🟡 **Usability**: Bare-key auto-trigger over-fires — Partially resolved (eval case acknowledges; description text unchanged)
- 🔴 **Usability**: Manual smoke checklist only happy paths — Still present
- 🔴 **Usability**: JSON-on-stdout assertion needs debug guard — Still present
- ✅ **Usability**: Trigger evals deferred — Resolved
- 🟡 **Usability**: Negation `~` not clearly taught — Partially resolved (in `--help` banner; not in SKILL body teaching)

#### Compatibility (8 findings)
- ✅ **Compatibility**: Hard-coded pageToken regex — Resolved
- ✅ **Compatibility**: Default `expand=renderedFields` — Resolved
- 🟡 **Compatibility**: Reliance on `*all` and default-fields semantics — Partially resolved (Migration Notes mention schema; `*all` semantics not flagged as upstream-contract dependency)
- 🟡 **Compatibility**: Verify `disable-model-invocation: false` recognised — Partially resolved (manual verification step covers it; no explicit smoke test)
- ✅ **Compatibility**: Field token resolution undocumented — Resolved
- ✅ **Compatibility**: Walker handles two comment shapes by structural inference — Resolved (single shape now)
- ✅ **Compatibility**: No version field on fields.json — Resolved (deferral with Phase-3 plan recorded)
- 🔴 **Compatibility**: case…esac flag-parser growth contract undocumented — Still present

### New Issues Introduced

#### Major

- 🟡 **Correctness**: JSON string concatenation in `fields_array` does not escape special characters
  **Location**: M2 §4 jira-search-flow.sh, fields_array assembly
  When `_jira_search_resolve_field` falls through with the raw
  user-supplied token, embedding it into JSON via
  `fields_array+="\"$resolved\""` produces malformed JSON for
  tokens containing `"` or `\`. Fix by routing through `jq -R`
  or `jq --args`.

- 🟡 **Test Coverage**: Client-side comment slice lacks edge-case coverage
  **Location**: M4 case 5
  The `sort_by(.created) | .[-($n):]` slice is exercised only
  on the populated 8→5 case. Untested: short-array (N>length),
  empty array, missing `fields.comment`, missing `.created` on
  some entries, `--comments` × `--no-render-adf` interaction.
  Add 3-4 sub-cases.

- 🟡 **Test Coverage**: Fields-cache widening covered by only 2 cases
  **Location**: M1 §1 (test-jira-fields.sh refresh widening)
  Behaviour for non-textarea `schema.custom` values (e.g.
  `:textfield`, `:float`) is unspecified — does the cache
  preserve them or filter? Add a third case asserting
  field-type-agnostic preservation.

#### Minor (selected)

- 🔵 **Code Quality**: Near-duplicated `@me` resolution loop across `assignee_vals` and `reporter_vals` — factor into `_jira_search_substitute_me_in <array_name>` helper using nameref.
- 🔵 **Code Quality**: Hand-rolled JSON array assembly for `fields_array` — use `jq` for JSON escaping (overlaps with the major correctness finding above).
- 🔵 **Code Quality**: Performance section prose contradicts the resolved `_jira_search_resolve_field` shape (still claims "standard short forms" short-circuit).
- 🔵 **Code Quality**: M1 walker recursion outline elided with `…` — sketch the recursive paths explicitly.
- 🔵 **Test Coverage**: `--help` banner contents asserted by structural keywords only — drift between banner and `argument-hint` could go undetected.
- 🔵 **Test Coverage**: CSV/repeatable equivalence asserted but trailing whitespace and duplicate-token edge cases not specified.
- 🔵 **Test Coverage**: `jql_match` injection-resistance test asserts shape only, not exact equality of the escaped output.
- 🔵 **Correctness**: Empty CSV tokens (`--fields a,,b`) not filtered — Jira-side error rather than local validation.
- 🔵 **Correctness**: Comment slice undefined behaviour when `.created` is missing/null on some comments.
- 🔵 **Correctness**: `--expand` flag parsing not shown in M4 outline; interaction with `--comments`-driven append unclear.
- 🔵 **Security**: `jql_match` escape order is correct but not documented inline — a future "simplify" refactor could re-introduce the bug.
- 🔵 **Security**: Mandatory INFO stderr echo of composed JQL may leak sensitive `--text` search terms into shared CI logs — consider `ACCELERATOR_JIRA_QUIET_JQL=1` opt-out.
- 🔵 **Security**: `--jql` trust boundary enforced only by SKILL prose with no runtime guard.
- 🔵 **Usability**: Mandatory INFO stderr echo overwhelms scripted/loop callers — add `--quiet`/`-q` flag.
- 🔵 **Usability**: `--order-by` / `--reverse` declared in helper outline but absent from argument-hint, `--help` banner, and tests.
- 🔵 **Usability**: `argument-hint` syntax `[--fields a,b,c|--fields a]...` is non-standard and may confuse first-time readers.
- 🔵 **Compatibility**: Comment slice depends on Atlassian's embedded shape always populating `.created` — falls back to undocumented sort behaviour for null keys.
- 🔵 **Compatibility**: Atlassian's embedded comments page size (`~20`) is asserted but not pinned to a documented value — silent truncation when N exceeds it.

### Assessment

The revision is substantively complete. All criticals resolved,
JQL composition properly factored, comments shape unified,
default-on rendering testable, trigger evals shipped. The
remaining work is one targeted pseudocode fix (JSON
concatenation in `fields_array`) plus ~6-8 additional test
cases (comment-slice edges, fields-cache schema breadth,
auth-failure propagation, ADF richness fixtures). A third
revision pass focused on these would land the plan at
COMMENT/APPROVE; alternatively, the user can accept the
remaining items as implementation-time deltas and proceed,
since the plan provides enough scaffolding for the implementer
to fill in the gaps without further design decisions. The new
findings are concrete and individually small; none represent
unresolved structural questions about the design.

---

## Re-Review (Pass 3) — 2026-05-02T14:00:00Z

**Verdict:** COMMENT

Pass 3 closed every previously-flagged major finding from pass 2.
The plan now carries **0 critical and 0 major findings** across
all 7 lenses; the residual items are minor or suggestion-tier
quality concerns appropriate for implementation-time fixup
rather than further plan revision. The plan is ship-ready in
scope and approach; reviewers should expect to apply the
remaining minors as small in-flight adjustments during
M1-M6 execution.

### Pass-3 fix summary (22 changes)

All three new majors from pass 2 resolved:
- ✅ JSON `fields_array` rebuilt via `printf | jq -R | jq -s` (also drops empty CSV tokens).
- ✅ Comment-slice case 5 expanded to 5a–5e (happy, N>length, empty, missing block, with `--no-render-adf`).
- ✅ Fields-cache widening third case for non-textarea `schema.custom`.

All 12 still-present items from pass 2 resolved:
- ✅ ADF walker fixture richness (case 1 → 1a–1c covering single/multi-paragraph + heading + list + marks + code block).
- ✅ Idempotency at process boundary (case 11 → 11a/11b with instrumented "renderer not re-spawned" stub).
- ✅ Show-flow auth/mixed-content/`--expand`×`--fields` (cases 10a–10c, 15, 16 added).
- ✅ Hard-coded `expand: "names,schema"` removed.
- ✅ Worklog ADF declared out-of-scope with rationale.
- ✅ Manual smoke checklist failure paths (steps 11–16).
- ✅ Pagination round-trip example (M3 Example 3).
- ✅ Negation `~` convention taught explicitly in M3 step 1.
- ✅ `accountId` regex validation in `_jira_search_resolve_me`.
- ✅ Performance prose contradiction corrected.
- ✅ `--order-by`/`--reverse` vestigial declarations dropped.
- ✅ `jql_match` escape order documented inline + adversarial test added; injection test now exact-equality.

Plus pass-2 minors addressed:
- ✅ `@me` resolution duplication → `_jira_search_substitute_me_in <array>` helper.
- ✅ `--expand` flag arm shown in M4 outline; comments append documented.
- ✅ Empty CSV tokens filtered.
- ✅ Comment-slice `.created // ""` fallback.
- ✅ M5 description softened with explicit no-trigger guidance for incidental key mentions.

Plus three judgment-call additions accepted:
- ✅ `--quiet`/`-q` flag suppresses INFO JQL echo for scripted callers.
- ✅ Markdown link-scheme allowlist (http/https/mailto) added to `jira-adf-render.jq` with 7 test cases.
- ✅ Trust model documentation (VCS posture + umask + accountId validation) in Migration Notes.

### New findings (all minor / suggestion)

The pass-3 edits introduced ~18 minor/suggestion-tier findings.
The most actionable cluster across lenses:

#### Cross-cutting (multi-lens)
- 🔵 **Link-scheme allowlist may be too narrow** (Compatibility, Usability) — `tel:`, `sms:`, `xmpp:`, `ftp:`, `git:`, `slack:`/`zoommtg:` and similar legitimate non-HTTP schemes are silently stripped. Consider widening the allowlist or document the policy explicitly with rationale for future maintainers.
- 🔵 **`accountId` regex `[A-Za-z0-9:_-]+` may reject future Atlassian formats** (Security, Code Quality, Compatibility) — Atlassian doesn't promise the alphabet is permanent. Consider loosening to a control-char-only check, OR adding a length cap (e.g. 128) AND noting the assumption in Migration Notes.
- 🔵 **Link-scheme whitespace normalisation** (Security, Test Coverage) — only leading-space tested; tab/newline/NBSP-prefixed schemes could bypass if the implementation uses literal-space stripping. Use `[[:space:]]+` and add cases.
- 🔵 **Walker recursion outline still elided with `…`** (Code Quality, Test Coverage) — the load-bearing recursion shape (bash for-loop vs. jq `walk`) is the most underspecified piece left in M1. Five-line concrete sketch would lock the contract.
- 🔵 **`--expand`/`--comments` interaction** (Correctness, Test Coverage) — `--expand ''` empty value plus `--comments N>0` produces `expand=,comments` (leading comma); `--expand comments,changelog --comments 5` duplicates the `comments` token. Validate empty / dedupe.

#### Single-lens minors

- 🔵 **Architecture**: `_jira_search_substitute_me_in` defined nested rather than at file scope; promote for testability/symmetry.
- 🔵 **Architecture**: Phase 1 jq-library hardening (link-scheme) landing in M1 blurs phase boundary; rename M1 or split into M0.
- 🔵 **Test Coverage**: No systematic `assert_stderr_empty` sweep across happy paths.
- 🔵 **Test Coverage**: Scenario fixtures still in flat directory; consider per-helper subdirs.
- 🔵 **Test Coverage**: `--fields` with no `fields.json` still implicit (case 9 covers cache-miss but not cache-absent).
- 🔵 **Test Coverage**: Mock unexpected-request assertion only on M2 case 1; should be systematic.
- 🔵 **Test Coverage**: `--fields` trailing whitespace and duplicate-token edge cases not tested.
- 🔵 **Test Coverage**: `--help` banner asserted by structural keywords only — drift between banner and parser flags possible.
- 🔵 **Correctness**: `local -n` nameref aliasing risk — comment naming the gotcha.
- 🔵 **Correctness**: `jq -R` line-mode would mis-split tokens containing newlines (defence-in-depth: pre-validate or use `jq --args`).
- 🔵 **Correctness**: Link-scheme detection regex unspecified (`page.html` schemeless vs. `page:html` scheme-bearing distinction).
- 🔵 **Security**: Allowlist excludes `tel:`/`sms:`/`xmpp:`/`ftp:` without explicit rationale.
- 🔵 **Usability**: Argument-hint `[--fields a,b,c|--fields a]...` non-standard; simplify to `[--fields LIST]...`.
- 🔵 **Usability**: No `JIRA_DEBUG` env to relax JSON-on-stdout for troubleshooting (low-priority Phase-3 item).
- 🔵 **Usability**: Stale `fields.json` cache emits warnings only on unknown-token resolve, not as a general state hint.
- 🔵 **Compatibility**: `expand=names,schema` removal couples search-response interpretation to local cache; document for Phase 3.
- 🔵 **Compatibility**: Smoke step 14 "Phase 1 auth-failure code" ambiguous between exit 11 (401) and 12 (403); pin individually.
- 🔵 **Compatibility**: Atlassian `customfieldtypes:textarea` identifier stability and embedded comment shape stability not documented as upstream-contract assumptions.
- 🔵 **Compatibility**: `jql_match` exit 31 referenced as `E_JQL_BAD_VALUE` in plan but registered as `E_JQL_UNSAFE_VALUE` in EXIT_CODES.md; standardise.

### Assessment

The plan is now in a state where the implementer can proceed
with confidence. All design decisions are settled, all
load-bearing contracts are specified, and the test surface is
substantial (M1: ~13 cases with sub-cases; M2: 17 cases plus
13 added jql library cases; M4: 16 cases; plus the 3 new
fields-cache cases in test-jira-fields.sh and the 7 link-scheme
cases in test-jira-adf-to-md.sh).

The remaining minor findings are exactly the kind of items that
surface naturally during implementation — scheme allowlist width,
nameref naming hygiene, fixture organisation — and can be
addressed in PR review without further design discussion. None
require revisiting any of the scoping decisions made in passes
1–3 (comment shape, JQL composition seam, `--no-render-adf`
default, fields-cache widening, invocation policy, `--jql` trust
boundary).

Verdict: **COMMENT** — the plan is acceptable as-is; reviewers
should treat the minor findings as a punch-list for
implementation-time refinement rather than blockers.
