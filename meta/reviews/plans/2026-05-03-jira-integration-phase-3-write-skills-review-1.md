---
date: "2026-05-04T00:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-jira-integration-phase-3-write-skills.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, usability, standards]
review_pass: 3
status: complete
---

## Plan Review: Jira Integration Phase 3 — Write Skills

**Verdict:** REVISE

The plan is structurally coherent and demonstrates strong TDD discipline, sensible milestone sequencing, and good safety instincts (the deviation from "VCS revert is the recovery path" is well-justified for irreversible Jira writes). However, five **critical** correctness/code defects and a wide field of cross-cutting major issues will cause tests to fail and behaviour to diverge from the contract as written. The most important issue is that **`jira_die` is hardcoded to exit 1**, which silently invalidates every named numeric exit code in the plan — this single fact is flagged independently by the architecture, code-quality, correctness, and standards lenses.

### Cross-Cutting Themes

- **`jira_die` / numeric exit mechanism** (flagged by: Architecture, Code Quality, Correctness, Standards) — `jira_die` delegates to `log_die` which always `exit 1`. Phase 2 helpers avoid it for flow-specific errors. The plan's `_jira_coerce_custom_value`, the `jira-body-input.sh` callers, and `jira_die "..." 112` (which silently drops the second arg) all rely on numeric codes that cannot actually be emitted. This must be resolved before any test scripts are authored.

- **Custom-field coercion is broken end-to-end** (Architecture, Correctness, Code Quality, Usability) — Phase 2's `jira-fields.sh` does not actually persist `schema.type` (only `schema.custom`), so `_jira_coerce_custom_value` falls into the empty-string branch for every field; the empty-string branch silently coerces as a JSON string instead of erroring; the `@json:42` test for sprint sends the wrong shape (must be `[42]`); and the error message points at `@json:` without an example.

- **Confirmation gate is untestable and ungrounded for delete** (Test Coverage, Safety, Security, Usability) — placing the confirmation in SKILL prose means there is no automated test for the most safety-critical behaviour in the phase. `delete` has no `--print-payload` path, so its preview is synthesised by Claude rather than computed by the helper. `--print-payload` failure handling before the gate is unspecified.

- **Phase 2 convention claims are factually wrong** (Standards, Architecture) — M5 says Phase 2 uses `disable-model-invocation: true`; both Phase 2 SKILLs use `false`. The plan also claims Phase 2 widened the cache to include `schema.type`; it didn't.

- **Pagination loop has correctness, safety, and termination issues** (Correctness, Safety) — uses requested `page_size` rather than actual `comments | length` for the increment; lacks max-page guard; synthesises a `maxResults` field set to `total`.

- **HTTP 400 maps to exit 20** (Architecture, Correctness, Test Coverage) — flagged as "investigate during implementation" but the answer is already on disk; this needs to be resolved before tests are written.

### Tradeoff Analysis

- **Strict y/Y vs revision-friendly UX**: Safety and Security favour the strict gate; Usability points out re-running the whole slash command after spotting a typo is expensive. The `create-work-item` precedent the plan cites actually stays in review on `n`. **Recommendation**: keep `y`/`Y` as the only confirm token but treat `n` as "revise" (matching `create-work-item`), abort only on unrelated input.

- **`--print-payload` body preview vs PII exposure**: Safety wants grounded previews; Security flags context-window exposure for sensitive body content. **Recommendation**: keep grounded previews but truncate body content > ~500 chars in the SKILL display (full payload still sent).

### Findings

#### Critical

- 🔴 **Architecture / Code Quality / Standards**: `jira_die` always exits 1 — named numeric exits are silently discarded
  **Location**: M1 jira-body-input.sh, M2 _jira_coerce_custom_value, M3 jira_die "..." 112
  Every flow-specific exit code reachable via `jira_die` collapses to exit 1; tests asserting exit 102/103/112/114 etc. will fail. Phase 2 used `echo >&2; return N` directly.

- 🔴 **Architecture**: `schema.type` is not persisted by Phase 2 — custom-field coercion will silently fail
  **Location**: Current State Analysis (~line 118)
  The Phase 2 jq filter only captures `schema.custom`. Every `--custom` invocation reads `.schema.type` as `""` and falls through to the string branch — `number`/`date`/`option`/`user` coercion is dead code as shipped.

- 🔴 **Correctness**: `[[ -n "$body" ]]` silently discards `--body ""`, contradicting test case 8
  **Location**: M1 jira_resolve_body
  Empty string is falsy in `[[ -n ]]`, so an explicitly-supplied empty body falls through to `--body-file`/stdin/`$EDITOR`. Test 8 (`Empty body permitted`) cannot pass. Caller pattern compounds this by passing `--body ""` unconditionally.

- 🔴 **Correctness**: Pagination termination uses requested `page_size`, not actual returned count
  **Location**: M4 _jira_comment_list pagination loop
  When server returns fewer records than requested (final page or server cap), `start_at` advances by `page_size`, skipping records or looping. Should use `actual_page_count = jq '.comments | length'`.

- 🔴 **Code Quality / Security**: Tempfile leaks because `RETURN` trap doesn't fire when `jira_die` calls `exit`
  **Location**: M1 jira_resolve_body editor branch; M2/M3 payload tempfiles
  `RETURN` only fires on `return`, not `exit`/signal. Tempfiles containing potentially sensitive body content persist in `/tmp`. Use `trap … RETURN INT TERM EXIT` and avoid `jira_die` inside the function.

#### Major

- 🟡 **Architecture**: Deferred `jira-custom-fields.sh` extraction creates divergence risk between parallel M2/M3 sessions
  **Location**: M3 — "decision made during M3 implementation"
  Make extraction unconditional and put it in M1.5.

- 🟡 **Architecture / Correctness / Test Coverage**: HTTP 400 maps to exit 20 in `jira-request.sh` (the wildcard branch)
  **Location**: M3 test case 24
  Plan defers as "investigate during implementation" but the answer is on disk. Decide before writing tests; consider adding a `400) … exit 16` branch.

- 🟡 **Architecture / Correctness**: Renderer dispatch heuristic is fragile against future API shapes
  **Location**: M4 jira-render-adf-fields.sh
  `has("comments") and not has("fields")`, `has("body") and type == "object"` could match changelog/worklog/transition responses with top-level `body`. Tighten with positive checks (e.g. `has("id") and has("author")`) or add an explicit `--shape` flag.

- 🟡 **Code Quality**: `_jira_coerce_custom_value` has `E_CREATE_BAD_FIELD` hardcoded but is shared with M3
  **Location**: M2/M3 _jira_coerce_custom_value
  Make the error prefix a parameter so M2 passes E_CREATE_BAD_FIELD and M3 passes E_UPDATE_BAD_FIELD.

- 🟡 **Code Quality**: `_jira_coerce_custom_value` reads `fields.json` from disk per call (hidden side effect coupled to file I/O)
  **Location**: M2/M3 _jira_coerce_custom_value
  Pass the resolved schema or path as an argument.

- 🟡 **Test Coverage**: `--print-payload` test cases cannot actually verify the API was not called
  **Location**: M2 case 22, M3 case 21, M4 cases 27–28
  Mock server has no zero-request assertion. Add a `capture_url`-with-`consume:false` guard fixture asserting empty captured-URLs file.

- 🟡 **Test Coverage**: Confirmation prompt has zero automated test coverage
  **Location**: M5 SKILL.md authoring
  Extract the y/Y match into a testable bash function or document the gap explicitly.

- 🟡 **Test Coverage**: Comment-list pagination edge cases under-specified
  **Location**: M4 test case 15
  Missing: `total=0`, `total == page_size`, total changes mid-pagination, mid-pagination deletion.

- 🟡 **Test Coverage**: ADF round-trip body assertions lack a specified comparison fixture
  **Location**: M2 cases 6, 15, 17; M3 cases 3–5, 17
  Pre-compute expected ADF via `jira-md-to-adf.sh` in test setup so assertions verify correct wiring, not just object presence.

- 🟡 **Correctness**: Empty `schema_type` falls through silently to string coercion
  **Location**: M2/M3 _jira_coerce_custom_value
  Separate `""` from `string|date|datetime` — cache miss should produce `E_*_BAD_FIELD` with `Hint: run /init-jira --refresh-fields`.

- 🟡 **Correctness**: `--custom sprint=@json:42` test fixture is wrong
  **Location**: M2/M3 test case 17
  Sprint is array of int, not int. Should be `@json:[42]`. Update tests and example.

- 🟡 **Correctness**: `--assignee ""` unassign mechanism is documented as `assignee=null`
  **Location**: M3 test 14
  Jira historically required `{accountId:null}` or `{accountId:"-1"}`. Specify exact shape and verify against tenant in M6.

- 🟡 **Correctness**: M2 test 3 requires `--type` unconditionally but test 19 says `--issuetype-id` overrides
  **Location**: M2 test cases 3 and 19
  Decide: is `--issuetype-id` alone sufficient? Update both tests to be consistent.

- 🟡 **Security**: `$EDITOR` invoked without validation — arbitrary command execution from environment
  **Location**: M1 jira_resolve_body editor branch
  Add a basename/path-executable check; document trust boundary in banner.

- 🟡 **Security**: `@json:` validates JSON structure but not field-name safety
  **Location**: M2/M3 _jira_coerce_custom_value
  User-supplied JSON merged directly into `fields`. Document trust boundary; log resolved values at INFO.

- 🟡 **Security**: Trust-boundary note ("never synthesise body from upstream context") is prose to the model only
  **Location**: M5 SKILL.md authoring
  Not enforced at bash layer. Add concrete enforcement step in M5 prose; add manual test for prompt-injection scenario.

- 🟡 **Safety**: Delete confirmation preview is ungrounded — no `--print-payload` path
  **Location**: M4 jira-comment-flow.sh; M5 SKILL.md
  Add `--describe`/`--dry-run` to delete subcommand so preview is computed by helper, not synthesised.

- 🟡 **Safety**: `--print-payload` failure handling before confirmation gate is unspecified
  **Location**: M5 SKILL.md authoring
  Required step: abort if `--print-payload` exits non-zero or empty.

- 🟡 **Usability**: `Hint:` line fires for all non-zero exits
  **Location**: M2 jira-create-flow.sh request-exit snippet
  Misleads users debugging non-credential errors. Branch hint message by exit code (11/12/13/19/20/22).

- 🟡 **Usability**: `@json:` escape hatch error message gives no concrete syntax example
  **Location**: M2 _jira_coerce_custom_value `*)` branch
  Expand with `e.g. --custom sprint=@json:[42] or --custom versions=@json:'[{"id":"10001"}]'`.

- 🟡 **Usability**: Strict y/Y gate has no revision path
  **Location**: M5 SKILL.md authoring
  Treat `n` as "stay in review and revise" rather than "abort", matching `create-work-item` precedent.

- 🟡 **Standards**: M5 incorrectly describes Phase 2 frontmatter as `disable-model-invocation: true`
  **Location**: M5 inputs to skill-creator sessions
  Phase 2 SKILLs use `false`. Correct as a Phase 3 departure justified by irreversible side-effects.

- 🟡 **Standards**: M1 caller pattern uses `return 113` for update body failure but 113 = `E_UPDATE_BAD_FLAG`
  **Location**: M1 caller pattern; M3 EXIT_CODES table
  Allocate a dedicated `E_UPDATE_NO_BODY` (e.g. 116).

- 🟡 **Standards / Correctness**: `E_BODY_STDIN_DISALLOWED` is documented but unreachable in implementation
  **Location**: M1 test case 4 vs jira_resolve_body
  Add explicit branch before editor fallback.

#### Minor

- 🔵 **Architecture**: Pagination synthesises `maxResults: total` — internally inconsistent with Jira contract
  **Location**: M4 _jira_comment_list
  Use actual count or omit `maxResults`.

- 🔵 **Architecture**: M5 generative variability — different skill-creator sessions may produce inconsistent SKILL prose
  **Location**: M5
  Provide a verbatim SKILL prose template before M5.

- 🔵 **Code Quality**: `local opt_body=""` not shown in skeletons — risks `set -u` failure
  **Location**: M2/M3/M4 skeletons
  Make explicit.

- 🔵 **Test Coverage**: Mock server `errors` exit not checked after `stop_mock`
  **Location**: Test infrastructure generally
  Capture exit status, fail current test if non-zero.

- 🔵 **Test Coverage**: No test for `--body` value beginning with `--`
  **Location**: M1 test-jira-body-input.sh
  Guard against `shift 1` regressions.

- 🔵 **Test Coverage**: No regression test for single-issue with top-level `comments` key
  **Location**: M4 test-jira-render-adf-fields.sh
  Renderer dispatch collision risk.

- 🔵 **Test Coverage**: No 5xx/timeout fixtures or test cases
  **Location**: M2/M3/M4 test scripts
  Add at least one per flow.

- 🔵 **Test Coverage**: M6 smoke test isn't idempotent and has no cleanup
  **Location**: M6 manual smoke test
  Accumulates stale issues on failure. Add cleanup step.

- 🔵 **Test Coverage**: No test for empty `fields` object exclusion when only `update` ops supplied
  **Location**: M3 test case 5
  Extend to assert `has("fields") == false`.

- 🔵 **Correctness**: `total` overwritten each page — synthetic response inconsistent under concurrent modification
  **Location**: M4 _jira_comment_list
  Compute `maxResults` from accumulated count.

- 🔵 **Correctness**: Caller passes `--body "$opt_body"` unconditionally, defeating any sentinel fix inside the helper
  **Location**: M1 caller pattern
  Conditionally include `--body` only when the flag was supplied.

- 🔵 **Correctness**: `--print-payload` output format for update is unspecified
  **Location**: M3 test case 21
  Define canonical shape (`{method, path, queryParams, body}`) once.

- 🔵 **Security**: URL query string built from Jira response without integer validation
  **Location**: M4 _jira_comment_list pagination loop
  Add `[[ "$page_total" =~ ^[0-9]+$ ]]` guard.

- 🔵 **Security**: `--no-notify` suppresses audit trail with no compensating log
  **Location**: M3/M4 --no-notify flag
  Surface in SKILL preview as "watchers will not be emailed".

- 🔵 **Security**: `--print-payload` echoes full body back to model context
  **Location**: M5 SKILL.md authoring
  Truncate body in SKILL preview display (>500 chars).

- 🔵 **Security**: `site.json` `accountId` tampered-value risk
  **Location**: M2/M3 @me substitution
  Add online cross-check warning against `/myself`.

- 🔵 **Safety**: Empty `--body ""` on update silently erases existing description
  **Location**: M3 test case 4; M1 test case 8
  SKILL preview should call this out explicitly.

- 🔵 **Safety**: Unbounded comment pagination — no max-page guard
  **Location**: M4 _jira_comment_list
  Add hard limit (e.g. 1,000 comments) with truncation warning.

- 🔵 **Usability**: Label-mode conflict error wording is unspecified
  **Location**: M3 test case 8
  Prescribe in plan.

- 🔵 **Usability**: `--no-notify` asymmetry (not on create) is undocumented in create banner
  **Location**: M2 jira-create-flow.sh banner
  Add note.

- 🔵 **Usability**: Default full-pagination on `list` can appear to hang
  **Location**: M4 _jira_comment_list
  Emit per-page INFO line; mention `--first-page-only` in SKILL prose.

- 🔵 **Usability**: `--assignee` accepts three forms with no documented precedence
  **Location**: M2/M3 --assignee
  Specify the contract.

- 🔵 **Standards**: Comment exit codes (91-99) precede create (100-109) numerically but are M4
  **Location**: M2/M6 EXIT_CODES.md
  Add reservation note in M2's EXIT_CODES.md update.

- 🔵 **Standards**: M2/M3/M4 skeleton comment headers are minimal — Phase 2 convention is a full structured banner
  **Location**: M2/M3/M4 skeletons
  Expand skeletons.

#### Suggestions

- 🔵 **Usability**: No top-level definition-of-done aggregating M1–M6
  **Location**: Overview / Desired End State
  Add a Phase 3 acceptance block.

### Strengths

- ✅ TDD-first discipline: every helper has its test script written before implementation, with explicit numbered cases.
- ✅ Milestone sequencing is well-reasoned (M1 unblocks M2/M3/M4; parallel execution explicitly acknowledged).
- ✅ The `BASH_SOURCE` guard, `set -euo pipefail`, `_JIRA_<FLOW>_SCRIPT_DIR` namespacing, and argument-loop pattern are correctly described and consistent with Phase 2.
- ✅ Payload assembly via `jq -n --arg/--argjson` chains correctly avoids string concatenation.
- ✅ `--print-payload` dry-run flag is the right design for grounding the confirmation preview.
- ✅ `disable-model-invocation: true` for write skills is a sound policy.
- ✅ Strict y/Y confirmation rejects "yes"/"sure"/silence — closes the most common accidental-confirmation vector.
- ✅ Renderer extension is purely additive; existing branches and idempotency guard preserved.
- ✅ Trust-boundary inheritance from Phase 1/2 is strong — credentials redacted, `token_cmd` blocked from team config, path/site validation in `jira-request.sh`.
- ✅ Migration note about `/init-jira --refresh-fields` proactively documents the most likely onboarding failure.
- ✅ `@json:` literal escape hatch routes through `jq -e` rejecting syntactically invalid JSON.
- ✅ The deviation from "VCS revert is the recovery path" is explicitly justified with sound reasoning.

### Recommended Changes

Priority-ordered. Items 1–5 unblock test authoring; 6+ are independent.

1. **Replace `jira_die` with `printf >&2; return N` for flow-specific errors** (addresses: jira_die always exits 1; M3 jira_die "..." 112; tempfile leaks; E_BODY_STDIN_DISALLOWED unreachable; M1 caller return code mismatch). Decide the policy before any test script is authored. Update M1, M2, M3, M4 implementation snippets.

2. **Resolve the `schema.type` cache gap as M1 prerequisite** (addresses: schema.type not persisted; empty schema_type fall-through; @json: documentation). Either widen `jira-fields.sh refresh` to persist `schema.type` (recommended) or pivot the coercion strategy. Update Current State Analysis to reflect reality.

3. **Resolve HTTP 400 exit-code mapping in `jira-request.sh` before writing tests** (addresses: 4xx pass-through; M3 test 24; updated `Hint:` per-code branching). Add `400) cat body >&2; exit 16` (or chosen code), update EXIT_CODES.md.

4. **Fix the empty-body sentinel** (addresses: `[[ -n "$body" ]]` empty-string; caller passes --body unconditionally; E_BODY_STDIN_DISALLOWED unreachable). Use `body_set=0` sentinel inside the helper AND change callers to conditionally include `--body` only when supplied (e.g. `${opt_body_set:+--body "$opt_body"}`).

5. **Fix the pagination termination** (addresses: pagination uses page_size; total changes mid-pagination; unbounded loop; synthetic maxResults). Use `actual_page_count = jq '.comments | length'`, terminate on zero or `start_at + actual >= total`, add hard max-page guard, set synthetic `maxResults: ($c | length)`.

6. **Make `jira-custom-fields.sh` extraction unconditional from M1.5** (addresses: deferred extraction; E_CREATE_BAD_FIELD hardcoded; conditional umbrella line). Make `error_prefix` a parameter; remove conditional umbrella line in M6.

7. **Fix the sprint test fixture** (addresses: `@json:42` should be `@json:[42]`). Correct test 17 in M2/M3 and the documented example.

8. **Resolve `--type` vs `--issuetype-id` contradiction** (addresses: M2 test 3 vs test 19). Decide: `--issuetype-id` alone satisfies the requirement; rewrite test 3 accordingly.

9. **Add a delete-preview helper output path** (addresses: ungrounded delete confirmation). `delete --describe` prints the canonical "DELETE /rest/api/3/issue/KEY/comment/ID" line without API call.

10. **Specify `--print-payload` failure handling in M5 SKILL prose** (addresses: --print-payload failure unspecified). Required step: "If --print-payload exits non-zero or emits empty output, abort before the confirmation gate."

11. **Correct the Phase 2 frontmatter claim in M5** (addresses: disable-model-invocation false vs true). Frame as a Phase 3 departure justified by irreversible side-effects.

12. **Branch the `Hint:` line by exit code** (addresses: Hint fires for all non-zero exits). Per-code messages for 11/12/13/19/20/22.

13. **Treat `n` as "revise" not "abort" in confirmation prompt** (addresses: strict y/Y has no revision path). Match `create-work-item` precedent.

14. **Expand the `@json:` error message with a concrete example** (addresses: @json: undocumented in error). `e.g. --custom sprint=@json:[42]`.

15. **Add tests for**: `--print-payload` zero-request guard; body containing `--`-prefixed value; single-issue with top-level `comments`; 5xx response per flow; empty-fields exclusion; comment-list edge cases (total=0, total=page_size, mid-pagination delete). Pre-compute ADF in test setup for round-trip assertions.

16. **Tighten security**: validate `$EDITOR`; document `@json:` trust boundary; add concrete trust-boundary enforcement to SKILL prose; integer-validate Jira response values before URL interpolation; signal-safe trap (`RETURN INT TERM EXIT`); truncate body in SKILL preview.

17. **Tighten safety**: SKILL preview surfaces `--no-notify` and "warning: empty description"; add unbounded-pagination guard with truncation warning; M6 smoke test cleanup step.

18. **Document polish**: `--no-notify` asymmetry in create banner; `--assignee` form precedence; label-mode conflict wording; expand skeleton banners; reservation note for codes 91-99 in M2's EXIT_CODES.md update; Phase 3 definition-of-done block; renderer `--shape` flag (or tightened heuristics with positive checks).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally coherent and builds cleanly on the Phase 1/2 foundation. The layering (body-input helper → flow helpers → SKILL prose) is correct, the renderer-extension strategy is additive and idempotency-preserving, and the confirmation-in-SKILL / execution-in-helper split is a sound separation of concerns. Three structural issues warrant attention before implementation begins: `schema.type` is not actually persisted by Phase 2, `log_die`/`jira_die` is hardcoded to exit 1, and the deferred `jira-custom-fields.sh` extraction creates real risk of divergent coercion logic.

**Findings**: schema.type not persisted by Phase 2 (critical); jira_die hardcoded to exit 1 (critical); deferred jira-custom-fields.sh extraction (major); HTTP 400 maps to exit 20 — answer already on disk (major); renderer dispatch fragility (major); jira_die exits parent script when sourced (minor); M5 generative variability (minor); pagination synthetic maxResults misrepresents page contract (minor).

### Code Quality

**Summary**: The plan is well-structured and consistent with existing Phase 2 conventions. Two concrete defects: `jira_die` calls `exit` (not `return`) which breaks the caller pattern's error-mapping contract, and the `RETURN` trap doesn't fire on `exit`, leaking the editor tempfile. Secondary: `_jira_coerce_custom_value` has `E_CREATE_BAD_FIELD` hardcoded in a function meant to be shared between M2 and M3.

**Findings**: jira_die calls exit, breaking caller's error-mapping contract (critical); tempfile leaks when editor fails — RETURN trap doesn't fire on exit (critical); E_CREATE_BAD_FIELD hardcoded in shared function (major); _jira_coerce_custom_value reads fields.json per call — hidden file I/O (major); jira_die for E_UPDATE_NO_OPS kills shell (major); shared coercion helper extraction deferred to implementation (minor); opt_body unbound variable risk under set -u (minor).

### Test Coverage

**Summary**: Comprehensive TDD-first suite with explicit numbered cases. Coverage gaps: `--print-payload` cases cannot verify zero requests; the confirmation prompt has no automated test coverage; pagination edge cases for comment-list under-specified; ADF round-trip body assertions lack a specified comparison fixture.

**Findings**: --print-payload cannot verify API not called (major); confirmation prompt zero automated coverage (major); comment-list pagination edge cases under-specified (major); ADF round-trip lacks comparison fixture (major); mock server errors not checked after stop_mock (minor); no test for --body containing flag-like value (minor); renderer extension lacks collision test (minor); no 5xx/timeout fixtures (minor); M6 smoke test not idempotent (minor); no test for empty fields object (minor); conditional extraction creates conditional test registration (minor).

### Correctness

**Summary**: Several logic errors that would produce wrong results at runtime. Most critical: `[[ -n "$body" ]]` silently skips empty `--body ""` (contradicting test 8); the pagination termination expression uses `page_size` not actual returned count (infinite/incorrect loops on short pages); `_jira_coerce_custom_value` silently coerces unknown schema types as plain strings.

**Findings**: [[ -n "$body" ]] discards empty body (critical); pagination uses page_size not returned count (critical); empty schema_type silent fall-through (major); @json:42 vs sprint array shape (major); 4xx exit code unresolved (major); --assignee "" mechanism uncertain (major); single-comment dispatch matches non-comments (major); --type vs --issuetype-id contradiction (major); total changes mid-pagination (minor); caller passes --body unconditionally (minor); E_BODY_STDIN_DISALLOWED unreachable (minor); --print-payload format unspecified (minor).

### Security

**Summary**: Strong baseline from Phase 1/2: path validation, site validation, credential redaction, test-mode gating, confirmed y/Y gate. Most significant unaddressed threats: `$EDITOR` invoked without validation, `@json:` validates JSON structure but not field-name safety, and the trust-boundary note for body content is prose to the model only — not enforced at the bash layer.

**Findings**: $EDITOR invoked without validation (major); @json: arbitrary structures (major); trust-boundary note prose only (major); URL query from Jira response no encoding (minor); tempfile RETURN trap not signal-safe (minor); --no-notify silently hides audit trail (minor); --print-payload echoes body to context (minor); site.json accountId tamperability (minor).

### Safety

**Summary**: Safety-conscious for a developer tooling context: the deviation from "VCS revert" is well-reasoned. Two genuine gaps: `delete` has no helper-backed payload preview (Claude synthesises the message); `--print-payload` failure handling before the gate is unspecified.

**Findings**: delete confirmation ungrounded (major); --print-payload failure handling unspecified (major); empty --body silently erases description (minor); unbounded comment pagination (minor); --no-notify undocumented as safety consideration (minor).

### Usability

**Summary**: Strong usability instincts: body-input precedence is explicit, --print-payload provides a dry-run path, SKILL confirmation gate is clear. Main gaps: Hint: logic too aggressive (credentials hint for all errors); @json: discoverability; strict y/Y has no revision path; --no-notify asymmetry undocumented.

**Findings**: Hint: line fires for all non-zero exits (major); @json: error message lacks example (major); strict y/Y has no revision path (major); label-mode conflict message wording unspecified (minor); default full-pagination silent hang (minor); --no-notify asymmetry undocumented (minor); comment delete preview ambiguity (minor); --assignee three forms undocumented (minor); no top-level definition-of-done (suggestion).

### Standards

**Summary**: Generally well-structured and accurately references most Phase 2 conventions. Concrete inconsistencies: `jira_die` always exits 1 (conflicts with the plan's design that flows return specific numeric exits); `disable-model-invocation` is misattributed as Phase 2 convention (Phase 2 actually uses `false`); `E_BODY_STDIN_DISALLOWED` is in tests/docs but unreachable in implementation; M1 caller pattern proposes `return 113` which collides with `E_UPDATE_BAD_FLAG`.

**Findings**: jira_die always exits 1 (critical); E_BODY_STDIN_DISALLOWED unreachable (major); M5 misattributes Phase 2 disable-model-invocation as true (major); M1 caller return 113 vs E_UPDATE_BAD_FLAG (major); comment exit codes 91-99 precede create numerically — no reservation note (minor); skeleton comment headers minimal vs Phase 2 banner convention (minor).

## Re-Review (Pass 2) — 2026-05-03T23:30:00+01:00

**Verdict:** REVISE

### Previously Identified Issues

**All 5 critical findings — Resolved**:
- 🔴 → ✅ jira_die always exits 1 — Resolved (printf >&2; return N throughout)
- 🔴 → ✅ schema.type not persisted — Resolved (M1c widens cache)
- 🔴 → ✅ [[ -n "$body" ]] discards empty body — Resolved (body_set sentinel)
- 🔴 → ✅ Pagination uses page_size — Resolved (uses page_returned)
- 🔴 → ✅ Tempfile RETURN trap leak — Resolved (RETURN INT TERM EXIT)

**Major findings (25+ from pass 1) — All Resolved or substantially addressed**: deferred custom-fields extraction, HTTP 400 mapping, renderer dispatch, sprint shape, --type/--issuetype-id contradiction, --print-payload zero-request guard, n-as-revise pattern, body truncation, --no-notify warning, disable-model-invocation misattribution, all standards items.

**One carryover from pass 1 minors — Still Present**:
- 🔵 site.json accountId tamperability — not addressed in this pass

### New Issues Introduced

#### Major (5)

- 🟡 **Architecture/CodeQuality/Correctness/Standards**: Exit code 99 contradiction
  **Location**: M4 EXIT_CODES.md table vs pagination loop vs M6 namespace summary
  M4 table says "Code 99 reserved for future expansion" but the pagination loop emits `return 99` for E_COMMENT_BAD_RESPONSE, test 33 expects exit 99, and the M6 summary documents 99 as E_COMMENT_BAD_RESPONSE. Flagged by 4 lenses.

- 🟡 **Usability/Standards**: E_*_BAD_ASSIGNEE unallocated
  **Location**: Convention notes vs M2/M3 EXIT_CODES tables
  Convention notes state email-form rejection exits with E_*_BAD_ASSIGNEE, but no numeric code is allocated, no test case exists, and the error message text is unspecified.

- 🟡 **Safety**: MAX_PAGES truncation Warning is stderr-only
  **Location**: M4 pagination loop / M5 SKILL prose
  When list hits the cap, Warning is emitted to stderr, but M5 SKILL prose does not surface it to the user. A user could act on truncated data unaware.

- 🟡 **Usability**: M3 Hint case block omitted from skeleton
  **Location**: M3 skeleton ("mirrors M2 with these differences")
  M2's exit-13 hint ("check the project key") is wrong for M3 (where 13 means "issue not found or not accessible"). An implementer copying M2 verbatim produces misleading error guidance.

- 🟡 **Usability**: M4 Hint blocks entirely unspecified
  **Location**: M4 skeleton (function bodies as `{ … }` placeholders)
  Four subcommands × multiple error codes with no prescribed hint text.

#### Minor (~9)

- 🔵 E_BODY_EDITOR_INVALID shares return code 4 with E_BODY_EDITOR_FAILED
- 🔵 Test 31 description has incorrect arithmetic (code is correct, doc misleads)
- 🔵 print-payload-guard.json fixture missing from M2/M3/M4 inventories
- 🔵 delete --describe missing-key/missing-id tests not specified
- 🔵 MAX_PAGES exact-boundary case (total = MAX_PAGES × page_size) untested
- 🔵 Empty-page-mid-pagination case untested (page_returned == 0 guard unexercised)
- 🔵 M3/M4 skeleton banners not expanded to match M2's
- 🔵 M3 test 14 says "verify in M6" but M6 sequence omits the unassign step
- 🔵 EDITOR error message doesn't list permitted character set

#### Suggestions

- MAX_PAGES as module-level constant (`readonly _JIRA_COMMENT_MAX_PAGES`)
- Generic hint cases extracted to a shared `_jira_emit_request_hint` in jira-common.sh
- @json: security comment in jira-custom-fields.sh
- Trust-boundary re-applied on n-revise iterations (Step 7 to reference Step 3)
- n-revise iteration soft-cap (e.g. "Revision 4 — review carefully")
- Phase-boundary label correction (M1c is a Phase 1 file, not Phase 2)
- --print-payload failure abort message directs user to stderr ("see error above")
- Unify --print-payload / --describe into a single dry-run interface

### Assessment

The plan has moved from "fundamentally unbuildable as written" to "buildable with minor inconsistencies to clean up". All critical defects are resolved; remaining majors are:
- Two **internal contradictions** that will fail the M6 documentation gate (exit 99, E_*_BAD_ASSIGNEE)
- Two **incomplete specifications** in M3/M4 skeletons (Hint blocks)
- One **safety gap** (MAX_PAGES warning visibility through SKILL)

These are tractable in a quick fix-up pass. They are not architectural problems and do not require restructuring milestones. A third review pass after addressing them would likely return APPROVE.

## Re-Review (Pass 3) — 2026-05-04T00:30:00+01:00

**Verdict:** COMMENT (a critical-severity collision was found and fixed inline during this pass; the plan as left after the fix has no critical findings)

### Previously Identified Issues (from pass 2)

**5 majors → all Resolved**:
- 🟡 → ✅ Exit code 99 contradiction — table now lists 99 = E_COMMENT_BAD_RESPONSE
- 🟡 → ✅ E_*_BAD_ASSIGNEE — codes 107/117 allocated with tests + error message
- 🟡 → ✅ MAX_PAGES truncation Warning — `truncated:true` field added; SKILL prose surfaces it
- 🟡 → ✅ M3 Hint case block — added with M3-specific exit-13 phrasing
- 🟡 → ✅ M4 Hint blocks — added; shared `_jira_emit_generic_hint` eliminates duplication

**Pass-2 minors — Resolved or partially resolved**:
- ✅ E_BODY_EDITOR_INVALID code 6, EDITOR error message extended, MAX_PAGES warning value, test 31 arithmetic, fixture inventory, delete --describe arg tests, empty-page test, M3/M4 banner expansion, M6 unassign verification, site.json regex, @json: SECURITY comment, --print-payload stderr direction, n-revise trust-boundary, M1c phase label, --custom missing= validation
- 🔵 Site.json accountId tamperability — Resolved via regex validation
- 🔵 E_BODY_STDIN_DISALLOWED unreachable from Phase 3 flows — Still present (the branch is reachable but only from non-Phase-3 callers; tested in isolation)

### New Issues Introduced (and inline-fixed)

#### Critical — fixed during pass 3

- 🔴 → ✅ **Architecture/Standards**: **Exit code 24 namespace collision** — M1d allocated exit 24 to `E_REQ_BAD_REQUEST` (jira-request.sh) but EXIT_CODES.md already has 24 = `E_NO_TOKEN` (jira-auth.sh). Verified against the on-disk file. **Fixed inline**: changed to exit 34 (next free in the 34-39 gap, since 11-23 = jira-request.sh, 24-29 = jira-auth.sh, 30-33 = jira-jql.sh). Updated all 10 references in the plan.

#### Major

- 🟡 **Architecture**: `_jira_emit_generic_hint` couples `jira-common.sh` to transport-layer exit codes from `jira-request.sh`. Suggestion: move to a request-hints helper or into `jira-request.sh` itself.

- 🟡 **Test Coverage**: `_jira_emit_generic_hint` test section lacks numbered cases — only mentions "covering codes 11/12/13/19/20/21/22/24" as a parenthetical. Add structured per-code cases asserting exact stderr text and return value.

- 🟡 **Test Coverage**: BAD_ASSIGNEE bad-character cases (11b/14b) assert exit code only, not error message text. Mutation-test gap; specifically the value-echo path (security-relevant).

- 🟡 **Test Coverage**: Tampered `site.json` accountId path has no test case. The regex validation specified in the plan is unverifiable by the test suite.

- 🟡 **Security**: BAD_ASSIGNEE and BAD_FIELD error messages echo unsanitised user-supplied values verbatim to stderr (`(got: <value>)`). Terminal-escape and log-injection vector in CI contexts.

- 🟡 **Safety**: 3-revision fatigue counter relies on Claude's in-context counting with no structural enforcement. May silently fail in long sessions or after context compaction.

#### Minor

- 🔵 Architecture: tampered site.json conflated with missing-cache (same exit code, misleading hint)
- 🔵 Code Quality: `_jira_emit_generic_hint` return-1 semantic unconventional under set-e (safe in practice via `if !` pattern, but could mislead a maintainer)
- 🔵 Test Coverage: 20-expectation `comment-list-natural-end-at-cap.json` fixture format unspecified — startAt matching across pages ambiguous
- 🔵 Correctness: E_BODY_STDIN_DISALLOWED still unreachable from Phase 3 flows (carryover; helper-level test exists)
- 🔵 Correctness: `--assignee @ME` (uppercase) rejected with misleading "email" error message; no test case
- 🔵 Correctness: Renderer idempotency test doesn't exercise `truncated` field round-trip
- 🔵 Security: n-revise trust-boundary remains instruction-only (known limitation)
- 🔵 Safety: `accountId:null` conditional update path is in M6 step 4b but not in DoD checklist bullets
- 🔵 Safety: shared hint helper means Phase 4 hint edits affect Phase 3 messages — coupling concern
- 🔵 Safety: 20-expectation fixture silent-pass risk if mock server misroutes
- 🔵 Usability: E_CREATE_BAD_ASSIGNEE (107) absent from M2 banner (allocated but not listed in --help)
- 🔵 Usability: Truncation re-run suggestion in SKILL prose omits issue key placeholder
- 🔵 Standards: Body-input EXIT_CODES table uses 3-column format vs 4-column standard
- 🔵 Standards: BAD_ASSIGNEE error format `(got: <value>)` inconsistent with other E_* messages
- 🔵 Standards: M3/M4 banners list exit codes two-per-line; M2 uses one-per-line

#### Suggestions

- MAX_PAGES as readonly module-level constant (carryover)
- Sentinel boilerplate cross-reference comments at call sites (carryover)
- Define n-revise counter reset boundary explicitly (per-invocation? hard-cap?)
- Strengthen `test-jira-fields.sh exits 0` criterion to require named schema.type assertion
- Reconcile "four lines" / "five lines" inconsistency between Implementation Approach (line 335) and M6 detail
- M5 10 numbered steps — verify against Phase 2 SKILL.md conventions before authoring

### Assessment

The plan continues to improve significantly. Of the 5 critical findings from pass 1, all are resolved. Of the 5 major findings from pass 2, all are resolved. Pass 3 surfaced **one new critical** (exit code 24 collision) — caught by reading EXIT_CODES.md against the plan's claims, **fixed inline**. After the fix, no criticals remain.

The 6 remaining majors fall into three clusters:
- **Test coverage gaps** (3 findings): test-jira-common.sh, BAD_ASSIGNEE message-text, tampered site.json — these are documentation/specification gaps, easily addressed
- **Security stderr injection** (1 finding): legitimately new; user-supplied values echoed without sanitisation
- **Architecture/Safety** (2 findings): jira-common.sh coupling, revision counter enforcement — design questions worth discussing but not blocking

The plan is now **acceptable for implementation** with the understanding that these majors are tracked for cleanup. A pass-4 review after addressing them would likely be APPROVE. Verdict downgraded from REVISE to COMMENT because the inline-fixed critical was the only blocking issue; the remaining majors are bounded improvements rather than foundational defects.
