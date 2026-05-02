---
date: "2026-05-03T03:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-03-design-convergence-workflow.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, standards, usability, documentation]
review_pass: 4
status: complete
---

## Plan Review: Design Convergence Workflow Implementation Plan

**Verdict:** REVISE

The plan demonstrates strong architectural alignment with ADR-0001, well-staged TDD phasing, and faithful reuse of existing locator/analyser, template auto-discovery, and skill orchestration patterns. However, the security lens raised five critical-or-major findings centred on credential handling that block implementation as-written: an auth header sent on every navigation with no origin allowlist, secrets reaching screenshots and inventory bodies committed to the repo, and an unpinned `@latest` MCP server are all credential-disclosure or supply-chain risks. The correctness lens additionally identified two critical issues (undefined source-id resolution semantics and a non-atomic supersede protocol) that will produce silently-wrong gap analyses in normal use. Several proposed bash test snippets contain function-signature inversions (`assert_contains` argument order) and reference helpers that don't exist in `test-helpers.sh` (`assert_file_exists`), so the TDD red-then-green cycle won't actually run. Cross-cutting themes around the MCP probe pattern, the cue-phrase contract enforcement, env-var discoverability, and the wildcard `mcp__playwright__*` allowance reinforce that several precedent-setting decisions need pinning down before this lands.

### Cross-Cutting Themes

- **MCP availability probe is the wrong shape** (architecture, code-quality, correctness, usability) — Multiple lenses flagged that probing via `browser_navigate about:blank` conflates "MCP not declared", "server failed to start", "browser binary missing", and "transient navigation error". Each failure mode needs distinct user-visible messaging, and the detection pattern should be lifted into a reusable helper rather than copied per skill.
- **Cue-phrase contract is enforced only by exhortation** (architecture, correctness, test-coverage) — The contract between `analyse-design-gaps` output and `extract-work-items` consumption is the load-bearing integration of the chain, but is enforced only by skill-prose-grep (`assert_contains "we need to"`) plus the round-trip eval. A non-conformant paragraph is silently dropped downstream.
- **Supersede protocol is non-atomic and resolution semantics are undefined** (correctness, code-quality, test-coverage) — Three lenses converged on the supersede mutation lacking ordering, recovery, concurrency, and same-day-collision specs. Combined with `analyse-design-gaps`'s glob-based source-id resolution (no zero-match / multi-match / malformed-frontmatter handling), this produces a class of silent-failure modes likely to dominate production bugs.
- **Auth env-var contract is invented inline without abstraction or documentation** (architecture, code-quality, security, usability, documentation) — Five lenses raised concerns: namespacing (`ACCELERATOR_DESIGN_*` vs `ACCELERATOR_BROWSER_*`), validation gaps for partial credential sets, no precedence rule when both header and form-login are configured, no discovery surface in README/CHANGELOG, and trust-boundary risk if env vars enter the LLM context.
- **`mcp__playwright__*` wildcard is precedent-setting in the wrong direction** (architecture, security, standards, documentation) — The skill's `allowed-tools` wildcard plus the README agents-table wildcard hide the very tool-allocation discipline the plan goes to lengths to enforce in agent frontmatter. A future Playwright MCP release silently expands what the skill is allowed to do.
- **Bash test snippets have concrete defects that block TDD** (code-quality, test-coverage) — `assert_contains` argument order is inverted relative to `test-config.sh`'s local override, and `assert_file_exists` is referenced from new scripts that source only `test-helpers.sh` where it isn't defined.

### Tradeoff Analysis

- **Security (auth-header origin allowlist) vs Usability (auth-walled crawl coverage)**: Restricting the auth header to the original origin will mean OAuth/SSO redirects break crawls. The plan should choose: drop OAuth-style auth from v1 (recommended; document in "What We're NOT Doing"), or implement a navigation hook that re-authenticates on permitted redirect targets.
- **Test rigour (per-paragraph cue-phrase enforcement) vs Skill orchestration simplicity (LLM-driven prose)**: A programmatic post-write check that asserts each non-empty H2 contains a cue-phrase paragraph would catch silent-drop regressions but adds a non-trivial skill step. The alternative (machine-readable structural section consumed by `extract-work-items`) is more robust but rewrites the cross-skill contract.
- **MCP detection precision (lift to shared helper) vs Phase-isolation discipline (this plan ships only the consuming skill)**: The reusable `scripts/config-check-mcp.sh` is the right abstraction but expanding scope risks the "design-convergence plan also rewrites MCP infrastructure" anti-pattern. Recommend punting the helper to a follow-up but documenting the consuming skill's probe contract precisely so it is easy to extract later.

### Findings

#### Critical

- 🔴 **Security**: `ACCELERATOR_DESIGN_AUTH_HEADER` sent on every navigation with no origin allowlist
  **Location**: Phase 3 §2 Auth handling
  Cross-origin token leak — bearer credential will be sent to attacker-controlled hosts via off-site links, OAuth redirects, or javascript: hops. Standard `Authorization` forwarding flaw. Constrain header injection to scheme+host+port of the resolved `[location]`/`LOGIN_URL`, strip on cross-origin redirects, and assert via eval.

- 🔴 **Security**: Secrets reach the LLM and may be written into `inventory.md` / screenshots committed to the repo
  **Location**: Phase 3 §2 Auth handling; "What We're NOT Doing" §3
  Per-screen screenshots are durable in the repo (pruning is deferred). Login-flow values, query-string tokens, and session cookies can be captured in screenshots and inventory body via DOM rendering. Mask password fields, strip URL query strings, scrub secret patterns from generated body, forbid the agent from rendering env-var values, and add an eval that fails on known-secret patterns.

- 🔴 **Security**: Unpinned `npx @playwright/mcp@latest` is a supply-chain risk
  **Location**: Phase 2 §1 .claude-plugin/.mcp.json
  Every invocation may resolve to a new version; a single malicious publish compromises every accelerator user. Pin to a specific version (e.g., `@playwright/mcp@0.0.x`), add a CI check flagging `@latest` in any `.mcp.json`.

- 🔴 **Correctness**: Source-id resolution has undefined semantics for zero-match and multi-match
  **Location**: Phase 4 §1 step 1
  No specified behaviour when no inventory matches, when multiple non-superseded inventories match (likely after a crashed prior run), or when frontmatter is unparseable. Same-day re-runs collide on the date-only directory name. Specify error behaviour explicitly (zero → fail with available list; multi → deterministic tiebreak with warning), and add evals for each branch.

- 🔴 **Correctness**: Supersede mutation is non-atomic with no failure-recovery story
  **Location**: Phase 3 inventory-design Resolve Source / supersede semantics
  Two-step write (new directory → mutate prior frontmatter) has no ordering, atomicity, or concurrency contract. Partial failures leave the resolver unable to disambiguate. Specify ordering (write-new-then-mutate-prior), an idempotent resolver fallback ("if multiple match, treat older as superseded for resolution"), and add an eval for the partial-failure path.

#### Major

- 🟡 **Code Quality**: `assert_contains` argument order is inverted in proposed `test-config.sh` additions
  **Location**: Phase 1 §5 (lines 291-312)
  `test-config.sh` shadows the helper with a local `(name, needle, haystack)` signature; the new calls use `(name, haystack, needle)`. Tests will pass/fail for the wrong reasons. Either flip the new calls or remove the local override and standardise.

- 🟡 **Code Quality**: `assert_file_exists` is not exported by `test-helpers.sh`
  **Location**: Phase 1 §5 (line 322); Phase 2 §6 (line 489); Phase 3 §5 (line 657)
  The new scripts source only `test-helpers.sh` and call `assert_file_exists`, which lives only in `test-config.sh`. Each script will exit immediately under `set -euo pipefail`. Lift the helper into `test-helpers.sh` as a Phase 0 prerequisite.

- 🟡 **Code Quality / Architecture / Correctness**: MCP availability probe via `about:blank` conflates failure modes
  **Location**: Phase 3 §3 Choose Crawl Strategy
  Side-effecting probe; cannot distinguish "tool not declared" from "navigate failed". Default `--crawler hybrid` will silently degrade on transient errors. Distinguish "MCP tool absent from session" (deterministic, safe to fall back) from "MCP tool present but errored" (raise visibly). Lift detection into a reusable `scripts/config-check-mcp.sh` helper.

- 🟡 **Architecture / Correctness / Test Coverage**: Cross-skill cue-phrase contract is enforced only by exhortation
  **Location**: Phase 4 §1 prose-generation; §3 tests
  Failed paragraph is silently dropped by `extract-work-items`. Replace with either a machine-readable structural section consumed by `extract-work-items`, or a programmatic post-write check that asserts each non-empty H2 contains a cue-phrase paragraph. Strengthen eval #5 with a parallel static cue-phrase eval as discriminator.

- 🟡 **Architecture**: Inventory-as-directory diverges from flat-file `meta/` convention without consumer-side adaptation
  **Location**: Phase 1 §1 / Desired End State
  `documents-locator` won't surface inventories; the divergence is shipped without resolving the consumer side. Either bring the locator update into scope or commit a follow-up ticket and add a README note.

- 🟡 **Test Coverage**: MCP probe error path has no automated test
  **Location**: Phase 3 §3
  Only the happy fallback is covered. Add evals/structural assertions distinguishing probe failure modes.

- 🟡 **Test Coverage**: Auth-failure paths underspecified
  **Location**: Phase 3 §2; eval #4
  Bad credentials, header-only silent 401, partial env-var sets all untested. False-positive "removed features" downstream is a high-blast-radius silent failure. Add evals plus a SKILL.md grep for the four env-var names.

- 🟡 **Test Coverage**: Supersede edge cases (already-superseded, multiple priors, same-day) untested
  **Location**: Phase 3 eval #2
  Add same-day re-run, three-run sequence, and malformed-prior-frontmatter evals.

- 🟡 **Test Coverage**: 5 evals × 3 runs is statistically thin against 0.9 floor
  **Location**: Testing Strategy → Eval Tests
  A single intermittent failure swings the mean. Either raise to 5+ runs or document as smoke-test and add an extended benchmark.

- 🟡 **Test Coverage**: End-to-end coverage is manual-only with no committed fixture
  **Location**: Testing Strategy → End-to-End (Manual)
  `examples/design-test-app/` is conditionally created. Commit the fixture and add an e2e shell test running the full chain non-interactively.

- 🟡 **Test Coverage**: New test scripts not actually wired into mise — only described
  **Location**: Phase 1 §5; Phase 2 §6; Testing Strategy
  "Add to existing OR create new task" leaves CI integration ambiguous. Provide an explicit `mise.toml` diff and assert via `mise tasks ls | grep test:integration:design`. Also assert script executable bits.

- 🟡 **Test Coverage**: Round-trip eval doesn't isolate the cue-phrase contract from extract-work-items behaviour
  **Location**: Phase 4 eval #5
  Failures in either skill mask each other. Add a parallel static cue-phrase eval as a discriminator.

- 🟡 **Correctness**: Init count assertion is brittle against future directory additions
  **Location**: Phase 1 tests — `scripts/test-design-foundation.sh`
  Literal "the 14 directories" assertion adds a fourth coupling site without addressing the underlying brittleness. Compute the count programmatically or assert it equals the number of `config-read-path.sh` invocations in the resolution block.

- 🟡 **Correctness**: Auth env-var contract has gap when both `AUTH_HEADER` and `USERNAME`/`PASSWORD` are set
  **Location**: Phase 3 inventory-design Auth handling
  No precedence rule, no validation for partial credential sets, leading to inconsistent supersede behaviour. Define explicit precedence (header wins; partial credentials fail clean) and add an eval.

- 🟡 **Security**: `browser_evaluate` grants arbitrary JS execution against user-supplied URLs
  **Location**: Phase 2 §3 agents/browser-analyser.md tools list
  Drop `browser_evaluate` (state matrix and computed values can largely come from the accessibility-tree snapshot), or restrict via skill instructions to a read-only payload allowlist with eval enforcement.

- 🟡 **Security**: Unvalidated `[location]` enables `file://`, `javascript:`, and SSRF
  **Location**: Phase 3 Resolve Source step
  Validate scheme (require https://, deny file/javascript/data/chrome) and reject RFC1918/loopback/link-local without an explicit `--allow-internal` opt-in. Add an eval for `file:///etc/passwd` exiting non-zero.

- 🟡 **Security**: Wildcard `mcp__playwright__*` admits future and undeclared tools
  **Location**: Phase 2 §6; Phase 3 §1 skill `allowed-tools`
  Replace wildcard with explicit list matching the union of the two agents' frontmatter; add a CI assertion against wildcard usage.

- 🟡 **Security**: Auth env vars accessible to LLM context regardless of crawler mode
  **Location**: Phase 3 §3
  Indirect prompt injection could exfiltrate. Read auth env vars only inside the bash wrapper that drives `browser_type`; never expose values to the LLM as text.

- 🟡 **Security**: Supersede mechanism mutates prior artifacts with no integrity check
  **Location**: Phase 3 §4 eval #2; Phase 4 step 1
  Repo write-access tampering vector. Document the trust assumption explicitly and consider using directory-name date prefixes (immutable) as the resolution source of truth instead of mutable frontmatter.

- 🟡 **Standards**: README agents-table row uses inconsistent wildcard vs explicit listings
  **Location**: Phase 2 §4
  `browser-locator` enumerates tools; `browser-analyser` uses `mcp__playwright__browser_*`. List the seven explicit tools.

- 🟡 **Standards**: `allowed-tools` wildcard scope for `mcp__playwright__*` needs explicit precedent decision
  **Location**: Phase 3 §1
  This is the first MCP-using skill — set the convention now (recommend explicit listing matching agent frontmatter).

- 🟡 **Usability**: Four new auth env vars have no documented discovery path
  **Location**: Phase 3 §2; Phase 5
  Document in README and CHANGELOG; emit a console message when a route appears auth-walled and an env var is unset.

- 🟡 **Usability**: `source-id` has no validation or error path when it does not resolve
  **Location**: Phase 3 §1 (argument-hint); Phase 4 §1 step 1
  Validate `^[a-z0-9][a-z0-9-]*$`; list available source-ids on miss; error on ambiguous match. Mirror `extract-work-items` step 1 behaviour.

- 🟡 **Usability**: `--crawler` flag values are undocumented in the user-facing surface
  **Location**: Phase 3 §1 / §3
  Document each mode in skill frontmatter description and a `## Crawler Modes` SKILL.md section. Make the conditional default explicit. Emit the auto-fallback notice before crawl starts, not only as a post-hoc Crawl Note.

- 🟡 **Documentation**: Four `ACCELERATOR_DESIGN_*` env vars documented only inside skill body
  **Location**: Phase 3 §2; Phase 5
  Surface in README/CHANGELOG mirroring `ACCELERATOR_VISUALISER_*` precedent.

- 🟡 **Documentation**: `examples/design-test-app/` fixture mentioned but not specified as a deliverable
  **Location**: Testing Strategy → End-to-End (Manual)
  Path also drifts (`./examples/test-app` in Phase 3 vs `examples/design-test-app/` in Testing Strategy). Reconcile and add as concrete deliverable.

- 🟡 **Documentation**: No first-time walkthrough or quick-start for the new design-convergence chain
  **Location**: Phase 5 §1
  Mirror Work Item Management section: diagram + paragraph + 3-step example sequence. Keep the research-doc pointer as a "for full rationale" footnote.

#### Minor

- 🔵 **Architecture**: Environment-variable auth contract invented inline rather than as a reusable mechanism
  **Location**: Phase 3 §2
  Rename to `ACCELERATOR_BROWSER_*`, or defer the env-var contract until a second consumer exists (per research §OQ #2).

- 🔵 **Architecture**: Hardcoded directory count in init skill is recurring SPOF
  **Location**: Phase 1 §2 / Current State Analysis
  Out of scope for this plan, but worth a `meta/notes/` deferred-debt entry.

- 🔵 **Architecture**: `inventory-design` orchestrator composes six agents — verify token budget against ADR-0001 ceiling
  **Location**: Phase 3 §1
  Add a synthesis-step note describing how large-app crawls stay within the 120k context envelope, or document a max-screen guidance.

- 🔵 **Architecture**: MCP server pinning to `@latest` couples plugin behaviour to upstream releases (also Critical from Security)

- 🔵 **Code Quality**: Hardcoded path-key list across init/configure/README is shotgun surgery
  **Location**: Current State Analysis; Phase 1 §2
  Add a `meta/notes/` follow-up; at minimum add a CI assertion that init/configure/README list the same set of keys.

- 🔵 **Code Quality**: Hardcoded directory count `12 → 14` is primitive obsession
  **Location**: Phase 1 §2 / §5
  Reword the prose to avoid the count, or surface via a `{count}` token.

- 🔵 **Code Quality / Test Coverage**: Tools-list assertions use single-line `awk` extraction; will silently miss wrapped YAML
  **Location**: Phase 2 §6
  Use a YAML-aware parser (python3 -c) consistent with `test-evals-structure.sh`.

- 🔵 **Code Quality**: Inline negative assertion duplicates assertion-helper boilerplate
  **Location**: Phase 2 §6 (lines 503-509)
  Add `assert_not_contains` to `test-helpers.sh`.

- 🔵 **Code Quality**: Authentication via four parallel env vars is a data clump
  **Location**: Phase 3 §2
  Either add a `scripts/resolve-auth.sh` validator or document precedence/exclusivity rules explicitly.

- 🔵 **Test Coverage**: Structural assertions grep prose substrings, coupling tests to wording
  **Location**: Phase 1 §5; Phase 3 §5
  Use HTML-comment markers (e.g., `<!-- DIR_COUNT:14 -->`) for counts; move cue-phrase contract into a machine-checkable file.

- 🔵 **Test Coverage**: Malformed inventory frontmatter / corrupted state untested
  **Location**: Phase 3 §1; Phase 4 §1
  Add evals with deliberately malformed fixtures.

- 🔵 **Test Coverage**: Mid-flight crash and screenshot accumulation paths untested
  **Location**: Phase 3 §1
  Atomic write via temp dir + rename; assert prior screenshots are byte-identical after re-run.

- 🔵 **Test Coverage**: Agent tool whitelist asserted by substring not parsed YAML
  **Location**: Phase 2 §6

- 🔵 **Test Coverage**: Phase 5 declares "Tests — none needed" for version bump
  **Location**: Phase 5 §4
  Verify cross-file version consistency between `plugin.json` and `mise.toml`.

- 🔵 **Correctness**: Foundation test assertion may match across newlines unintentionally
  **Location**: Phase 1 tests
  Grep for the full canonical line, or call the resolver behaviourally.

- 🔵 **Correctness**: Manual verification uses YYYY-MM-DD as the only directory disambiguator
  **Location**: Phase 3 Manual Verification
  Either add HH-MM-SS to the directory name or specify the same-day collision policy.

- 🔵 **Security**: No mention of shell quoting / variable expansion safety
  **Location**: Phase 3 §2
  Pass secrets via env-var inheritance, not command-line arguments. Add a shellcheck lint and an eval with a metacharacter-rich password.

- 🔵 **Security**: No DoS / runaway-crawl bound documented
  **Location**: Implementation Approach
  Document per-crawl page cap, wall-clock timeout, screenshot byte budget; bail with a clear error and `status: incomplete` frontmatter on cap hit.

- 🔵 **Security**: No security guidance in user-facing docs
  **Location**: Phase 5 §2; Phase 2 §5
  Add a "Security considerations" subsection to the README adjacent to the MCP install note.

- 🔵 **Standards**: Init path-resolution block ordering deviates from existing grouping
  **Location**: Phase 1 §2
  Insert design keys near `notes`; keep `tmp` last.

- 🔵 **Standards**: `mise.toml` does not contain a `version` field — cited as a place version lives
  **Location**: Phase 5 §3
  Replace with the actual version-bearing files used by `invoke version.bump`, or just delegate.

- 🔵 **Standards**: Bash test wiring path conflicts with existing `invoke`-based `test:integration:*` convention
  **Location**: Phase 1 §5
  Add `invoke test.integration.design` mirroring `invoke test.integration.config`, or extend the existing config task.

- 🔵 **Standards**: Wildcard in README agents-table tools column breaks formatting precedent
  **Location**: Phase 2 §4

- 🔵 **Standards**: `argument-hint` mixes positional and optional flag without codified precedent
  **Location**: Phase 3 §1
  Either document the bracket convention in the README skills overview or use `<>` for required positionals.

- 🔵 **Standards**: Leading-dot filename `.mcp.json` deviates from sibling `plugin.json` naming
  **Location**: Phase 2 §1
  Confirm Claude Code requires the dotted form; if not, use `mcp.json`.

- 🔵 **Standards**: Skill description shape — multi-line vs single-line — should be pinned
  **Location**: Phase 3 §1
  Cite a specific accelerator-skill description as the model.

- 🔵 **Usability**: No way to preview a gap analysis before files are written
  **Location**: Phase 4 §1 steps 1-6
  Add a "present summary, confirm before writing" step.

- 🔵 **Usability**: Viewport limitation surfaces as a naming convention with no validation
  **Location**: Phase 3 §1; What We're NOT Doing
  Capture viewport in inventory frontmatter, or document the convention in the skill body.

- 🔵 **Usability**: First-time MCP enable prompt wording is not pinned down
  **Location**: Phase 2 §5
  Distinguish "MCP not configured / declined" vs "browser binaries missing" failure modes in error messaging.

- 🔵 **Usability**: Positional-vs-flag mix is inconsistent with `research-codebase` precedent
  **Location**: Phase 3 §1
  Either validate that `source-id` is kebab-case (so reversed args fail clean) or make `location` a flag.

- 🔵 **Documentation**: Visualiser doc-types list and CHANGELOG entry not updated for two new doc types
  **Location**: Phase 5

- 🔵 **Documentation**: Discoverability — placement of the workflow section is under-specified
  **Location**: Phase 5 §1
  Pin precise placement (e.g., new H2 immediately after `## Work Item Management`).

- 🔵 **Documentation**: Migration Notes does not surface that users may want to opt into new path keys
  **Location**: Migration Notes

- 🔵 **Documentation**: Install-section MCP note placement uses approximate line numbers
  **Location**: Phase 2 §5
  Replace `~line 50-75` with anchor reference (`under `## Installation`, before `### Prerelease Versions``).

- 🔵 **Documentation**: Agents-table tool column uses abbreviation `mcp__playwright__browser_*`
  **Location**: Phase 2 §4

- 🔵 **Documentation**: CHANGELOG bullets do not explain user-visible behaviour
  **Location**: Phase 5 §2
  Lead with a one-paragraph capability framing.

#### Suggestions

- 🔵 **Code Quality**: Test-script granularity is unclear; foundation/agents/skills split is partly arbitrary
  **Location**: Phase 1 (line 358-364); Phase 2 (line 488)
  Recommend a single `scripts/test-design.sh` with section headers per phase.

- 🔵 **Code Quality**: Skill-creator deliverables are underspecified for code review
  **Location**: Phase 3 §1; Phase 4 §1
  Strengthen structural tests to assert presence of explicit instruction-block headings.

- 🔵 **Code Quality**: Supersede frontmatter mutation has no defined ownership
  **Location**: Phase 3 Success Criteria; Phase 4 evals
  Specify which step owns the mutation and the order relative to the new artifact write.

### Strengths

- ✅ Strong architectural alignment with ADR-0001: bounded-context agents, locator/analyser separation, filesystem-as-shared-memory.
- ✅ Agent split (`browser-locator` vs `browser-analyser`) preserves single-modality discipline with explicit negative tests.
- ✅ Naming the runtime pair `browser-*` rather than `design-*` is an explicit evolutionary-fitness choice.
- ✅ Phase ordering follows clean dependency-direction discipline; each phase is independently mergeable.
- ✅ Composition over duplication: reuses `codebase-*` agents rather than absorbing them.
- ✅ Phase 1 separates pure infrastructure from skill orchestration.
- ✅ Workflow chain plugs into existing filesystem-handoff patterns.
- ✅ TDD ordering is explicit per phase.
- ✅ Auto-discovery for templates and arbitrary path-keys honours existing extension seams.
- ✅ Eval and benchmark coverage treated as first-class deliverable per skill.
- ✅ Graceful degradation when Playwright MCP is unavailable is explicitly designed in.
- ✅ Each phase declares both Automated and Manual Verification gates.
- ✅ Locator/analyser tool-allocation contract correctly mirrored for browser-* agents.
- ✅ Path key naming uses snake_case matching existing convention.
- ✅ `config-read-skill-instructions.sh` placement at end of SKILL.md is asserted.
- ✅ Brace-style placeholders chosen for new templates matching newer-template precedent.
- ✅ `{<role> agent}` token convention preserved for user-configured agent name overrides.
- ✅ Negative assertions included for the locator/analyser tool boundary.
- ✅ Workflow chain is explicit and `analyse-design-gaps` commits to suggesting `extract-work-items` next.
- ✅ Phase 5 held to last so docs reflect shipped behaviour.
- ✅ README touch points identified precisely with line numbers.
- ✅ CHANGELOG entry contents enumerated by category.

### Recommended Changes

1. **Pin the Playwright MCP version and add a CI guard against `@latest`** (addresses: Critical Security #3, Architecture/MCP pinning)
   Replace `@playwright/mcp@latest` with a specific version. Add a foundation-test assertion that no `.mcp.json` declares `@latest`.

2. **Restrict `ACCELERATOR_DESIGN_AUTH_HEADER` to the resolved location's origin** (addresses: Critical Security #1)
   Specify in the SKILL.md that the analyser strips the header on any cross-origin navigation. Add an eval that asserts the header is not sent on a redirect to a different host.

3. **Specify secret-handling protections for screenshots and inventory body** (addresses: Critical Security #2)
   Mask password fields in screenshots, strip URL query strings in recorded URLs, run a pre-write secret-scrubber over the generated body that fails the run if env-var literals appear, document in skill body.

4. **Specify source-id resolution semantics for zero-match, multi-match, malformed-frontmatter, and same-day collision** (addresses: Critical Correctness #1, Minor Correctness same-day collision, Major Usability source-id validation)
   Document explicitly in the `analyse-design-gaps` SKILL.md. Use directory mtime or a finer-grained timestamp (HH-MM-SS) in the directory name to disambiguate same-day runs. Add evals for each branch.

5. **Specify supersede ordering, atomicity, and recovery** (addresses: Critical Correctness #2, Minor Code Quality supersede ownership, Minor Test Coverage supersede edge cases)
   Pin write-new-then-mutate-prior ordering. Define the resolver's behaviour when multiple non-superseded inventories match. Add evals for the partial-failure path. Consider treating directory date prefix as the source of truth and frontmatter `status` as advisory only.

6. **Fix bash test-snippet defects before TDD begins** (addresses: Major Code Quality `assert_contains` order, `assert_file_exists` missing)
   Add a Phase 0 step: lift `assert_file_exists` (and any other locally-defined helpers in `test-config.sh`) into `test-helpers.sh`. Reconcile `assert_contains` argument order across `test-config.sh` and `test-helpers.sh` (recommend: one signature, no shadowing).

7. **Replace MCP probe with a tool-presence check and lift to a reusable helper** (addresses: Architecture/Code-Quality/Correctness MCP probe theme, Usability MCP error messaging)
   Detect MCP availability by inspecting whether the tool is registered in the session, not by side-effecting navigation. Distinguish "MCP not declared", "navigate failed mid-crawl", "browser binaries missing" in user-visible error messages. Punt the shared `scripts/config-check-mcp.sh` helper to a follow-up but document the probe contract precisely in the consuming skill.

8. **Strengthen the cue-phrase contract enforcement** (addresses: Architecture/Correctness/Test-Coverage cue-phrase theme)
   Add a programmatic post-write check that scans each non-empty H2 of the gap artifact for at least one cue-phrase paragraph and fails the run if absent. Add a parallel static cue-phrase eval as a discriminator for the round-trip eval.

9. **Replace `mcp__playwright__*` wildcard with explicit listings** (addresses: Major Standards/Security wildcard theme, Documentation agents-table abbreviation)
   In the skill's `allowed-tools`, the README agents-table, and any CHANGELOG mention, enumerate the exact tools matching agent frontmatter. Add a CI assertion against `mcp__*` wildcard usage outside an explicit allowlist.

10. **Add validation and discovery for the four auth env vars** (addresses: Architecture inline auth contract, Code Quality data clump, Major Security/Usability/Documentation env-var theme, Correctness env-var precedence)
    Define precedence (header takes precedence; partial credentials fail clean), document in README and CHANGELOG with the `ACCELERATOR_VISUALISER_*` precedent as the model, emit a console message when an auth-walled route is detected and env vars are unset, and read secrets only inside the bash wrapper that drives `browser_type` (never expose values to LLM context). Consider renaming to `ACCELERATOR_BROWSER_*` for cross-skill reusability.

11. **Drop or constrain `browser_evaluate` and validate `[location]` URLs** (addresses: Major Security `browser_evaluate`, Major Security `[location]` validation, Minor Security DoS bound)
    Drop `browser_evaluate` if accessibility-tree snapshot is sufficient, or restrict to a read-only payload allowlist with eval enforcement. Validate `[location]` (require `https://`, deny `file:` / `javascript:` / `data:` / `chrome:` schemes, deny RFC1918/loopback without explicit opt-in). Document a per-crawl page cap and wall-clock timeout; bail with `status: incomplete` if hit.

12. **Commit the `examples/design-test-app/` fixture and wire an automated end-to-end test** (addresses: Major Test Coverage e2e fixture, Documentation fixture deliverable, Phase-3 path-name drift)
    Reconcile the path name (settle on one). Add the fixture as a Phase 1 or Phase 3 deliverable with a README. Add a `test:e2e:design` task running the full chain non-interactively and asserting expected artifact frontmatter.

13. **Wire CI integration explicitly with a concrete `mise.toml` diff** (addresses: Major Test Coverage CI wiring, Standards `invoke`-based convention)
    Replace the OR with an explicit decision. Recommend adding `invoke test.integration.design` (mirroring `invoke test.integration.config`) so the convention is preserved. Assert the script is discoverable via `mise tasks ls`.

14. **Address brittle structural assertions and consider one combined test script** (addresses: Major/Minor Test Coverage prose-substring asserts, Code Quality test-script granularity, Correctness foundation test assertion)
    Replace literal-string asserts with structured markers (e.g., `<!-- DIR_COUNT:14 -->`) or behavioural calls into the resolver. Recommend a single `scripts/test-design.sh` with section headers rather than three small files. Use YAML-aware parsing for tools-list assertions.

15. **Expand README workflow section into a quick-start walkthrough** (addresses: Major Documentation no walkthrough, Minor Documentation placement, Visualiser doc-types reconciliation)
    Mirror the Work Item Management section: diagram + paragraph + 3-step example invocation. Pin placement (recommend new H2 `## Design Convergence` immediately after `## Work Item Management`). Decide whether the Visualiser surfaces the new doc types and update the doc-types count and CHANGELOG entry accordingly.

16. **Bring `documents-locator` adaptation into scope or commit a follow-up** (addresses: Major Architecture inventory-as-directory divergence)
    Either add a Phase to update `documents-locator` for directory-style artifacts, or open a follow-up issue and add a one-line README note that inventories are intentionally not surfaced via `documents-locator`.

17. **Document the `--crawler` mode semantics user-side** (addresses: Major Usability `--crawler` undocumented)
    Add a `## Crawler Modes` SKILL.md section, make the conditional default explicit in `argument-hint`, and emit the auto-fallback notice before the crawl starts (not only as a post-hoc Crawl Note).

18. **Decide on `mise.toml` version bump and `.mcp.json` filename precedent** (addresses: Minor Standards `mise.toml` version, Minor Standards leading-dot filename)
    Remove the `mise.toml` reference from Phase 5 §3 (or replace with the actual version-bearing files / `invoke version:bump` task). Confirm whether Claude Code accepts `mcp.json` (no leading dot) and document the choice.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Strong alignment with ADR-0001 and reuses existing composition patterns. `browser-*` naming and the new `skills/design/` category are well-justified. Three precedent-setting concerns warrant stronger contractual scaffolding: MCP availability detection is brittle and undefined as a reusable pattern, inventory directory-as-artifact divergence weakens the convention used elsewhere in `meta/`, and the cross-skill prose contract is enforced only by skill instructions.

**Strengths**: agent split preserves single-modality / locator-analyser invariant; `browser-*` naming preserves evolutionary headroom; phase ordering follows dependency-direction discipline; composition over duplication; phase 1 separates infrastructure from orchestration; workflow chain uses filesystem-handoff patterns.

**Findings**: 7 (2 major, 5 minor). Major: MCP probe couples skill orchestration to MCP failure modes; cross-skill prose contract enforced only by exhortation; inventory-as-directory diverges from flat-file convention without consumer-side adaptation.

### Code Quality

**Summary**: Generally well-structured but contains concrete defects in proposed test snippets that will fail to run as written; entrenches existing maintainability smells; the MCP-availability probe is fragile.

**Findings**: 11 (3 major, 5 minor, 3 suggestion). Major: `assert_contains` argument-order inversion; `assert_file_exists` missing from `test-helpers.sh`; MCP probe is side-effecting.

### Test Coverage

**Summary**: Adopts a TDD posture with skill-creator evals and bash structural tests, but skewed toward static structural assertions; under-tests behavioural error paths; 5×3 benchmark is statistically thin; manual-only e2e with no fixture harness.

**Findings**: 12 (7 major, 5 minor). Major: MCP probe error path; auth-failure paths; supersede edge cases; thin benchmark; manual-only e2e; CI not actually wired; round-trip eval doesn't isolate cue-phrase contract.

### Correctness

**Summary**: Coherent for happy path but several load-bearing gaps around supersede protocol, MCP probe semantics, source-id resolution, and the cross-skill cue-phrase contract.

**Findings**: 9 (2 critical, 5 major, 2 minor). Critical: source-id resolution undefined for zero-match/multi-match; supersede mutation non-atomic with no recovery story.

### Security

**Summary**: New attack surface (secrets-bearing env vars, Playwright with browser-evaluate, durable on-disk screenshots) without commensurate controls. Acute risks: credential leakage, unpinned `@latest` MCP, auth header sent on every navigation, browser-evaluate against user-supplied URLs.

**Findings**: 11 (3 critical, 5 major, 3 minor). Critical: `AUTH_HEADER` cross-origin leak; secrets in screenshots/inventory; unpinned `@latest`.

### Standards

**Summary**: Largely consistent with conventions; precedent-setting choices around `mcp__*` wildcards and brace placeholders are reasonable but warrant alignment adjustments.

**Findings**: 9 (2 major, 7 minor). Major: README agents-table wildcard inconsistency; `allowed-tools` wildcard scope precedent.

### Usability

**Summary**: Well-shaped skills integrating into the workflow chain, but several DX concerns unaddressed: env-var auth invisible, source-id semantics conventional rather than validated, `--crawler` flag lacks help-text, error messaging unspecified, no preview before writing.

**Findings**: 7 (3 major, 4 minor). Major: env-var discovery; source-id validation; `--crawler` mode documentation.

### Documentation

**Summary**: Covers the core surfaces and stages user-visible polish to Phase 5; several deliverables under-specified or missing: env vars only in skill bodies, no quick-start, fixture not a clear deliverable, Visualiser doc-types list not updated.

**Findings**: 9 (3 major, 6 minor). Major: env vars not in README/CHANGELOG; fixture not a deliverable; no first-time walkthrough.

## Re-Review (Pass 2) — 2026-05-03

**Verdict:** REVISE (tighter than pass 1 — all 5 criticals resolved; remaining issues are addressable with targeted edits)

All 5 critical findings from pass 1 are fully resolved. The 5 most acute majors (auth-header origin allowlist, secrets in artifacts, MCP pin, source-id resolution, supersede atomicity) all have concrete contracts plus eval/structural test coverage. The TDD foundation defects (Phase 0) are unblocked. The cue-phrase contract is now executable (audit-cue-phrases.sh) with positive/negative/empty-H2 fixture tests.

The verdict remains REVISE because pass 2 surfaced 5 deduplicated NEW major findings — all from the revision itself rather than gaps in the original plan. They are tightly localised and most can be fixed with one-line edits to the plan.

### Previously Identified Issues

#### Critical (5 of 5 resolved)

- 🔴 → ✅ **Security**: AUTH_HEADER cross-origin leak — Resolved (origin allowlist + cross-origin strip + eval #6)
- 🔴 → ✅ **Security**: Secrets reaching artifacts/screenshots — Resolved (mask + URL scrub + pre-write scrubber + LLM isolation + eval #7)
- 🔴 → ✅ **Security**: Unpinned `@latest` MCP — Resolved (pin discipline + assert_not_contains @latest)
- 🔴 → ✅ **Correctness**: Source-id resolution undefined — Resolved (full algorithm Phase 4 §1 step 1 + evals #6-#8)
- 🔴 → ✅ **Correctness**: Non-atomic supersede mutation — Resolved (write-new-then-mutate + atomic tmp+rename + HHMMSS suffix + idempotent fallback + eval #9)

#### Major (25 of 25 addressed; 22 resolved, 3 partially resolved)

Resolved (22): MCP probe coupling; cue-phrase exhortation; assert_contains order; assert_file_exists missing; MCP probe side-effecting; auth-failure paths; supersede edge cases; new scripts wiring; MCP failure-mode conflation; cue-phrase silent-drop; round-trip eval coupling; init count brittleness; auth precedence gap; browser_evaluate arbitrary JS; [location] unvalidated; wildcard mcp__playwright__*; README agents-table wildcard; allowed-tools wildcard precedent; auth env-var discovery; source-id validation; --crawler undocumented; env vars only in skill body; fixture not a deliverable; no walkthrough.

Partially resolved (3): inventory-as-directory divergence (acknowledged + glob pattern documented; consumer adaptation deferred); MCP probe error path (mode 1 covered; modes 2/3 documented but no eval); E2E manual-only (fixture committed; e2e shell task explicitly deferred); round-trip eval not isolated (audit script as discriminator); auth env vars in LLM context (form-login isolated; AUTH_HEADER value still in tool-call surface); supersede tampering (date prefix authoritative; no cryptographic check).

Still present (1): 5×3 benchmark thinness (eval list grew to 15+9 but run count unchanged) — promoted to NEW major below.

#### Minor (35 of 35 addressed; ~28 resolved, ~5 partially resolved, ~2 deferred)

Most minors resolved (DIR_COUNT marker, YAML-aware tools extractor, assert_not_contains, .mcp.json filename precedent, mise.toml version misstatement, init path-key ordering, README placement, install-section anchor, agents-table column enumeration, CHANGELOG capability framing, Migration Notes, etc.). Three explicitly deferred follow-ups (path-key fan-out refactor; Visualiser doc-types graduation; no-preview-before-write UX). Two remain unaddressed (5×3 benchmark — promoted to major; viewport limitation).

### New Issues Introduced

Five deduplicated NEW major findings, plus a cluster of minors. The majors are tightly localised and addressable with one-to-three line edits.

#### Major

- 🟡 **Code Quality / Standards / Usability** (3 lenses converged): **`argument-hint` convention contradicts itself and the cited exemplar**. Phase 3 §1 spec uses `<source-id> <location>` (`<>` for required); Phase 3 §5 + Phase 4 §3 tests + Phase 5 README all use `[source-id] [location]` (`[]`); the exemplar `extract-work-items` itself uses `[]` for required positionals. The TDD-first tests will fail against the spec, OR the implementation matches the test and loses the required-vs-optional distinction. Pick one form everywhere — recommend matching `extract-work-items` (`[]` for both required and optional, with conditional default in inline phrase).

- 🟡 **Code Quality / Correctness** (2 lenses converged): **Cue-phrase audit regex is narrower than the contract it enforces**. Two distinct angles: (a) the regex `(we need to|users? need|the system must|implement [A-Z])` is lowercase-anchored but compliant fixtures use `We need to` / `Users need` (capitalised) — the audit will reject its own compliant fixture; (b) the cue-phrase set documented at `skills/work/extract-work-items/SKILL.md:130-138` includes more variants (user stories, "We need to implement", "must support"), which the audit will silently abort despite extraction accepting them. Fix with case-insensitive matching AND extracting the regex from the canonical extract-work-items list.

- 🟡 **Test Coverage**: **Run count per eval not raised despite expanding eval lists**. Phase 3 grew from 5 to 15 evals, Phase 4 from 5 to 9, but `benchmark.json runs each eval at least 3 times` is unchanged. The probabilistic evals (cross-origin strip, MCP fallback, prompt-injection scrubber) need 5+ runs to discriminate real regressions from flakes. Either raise the run count or document specific evals as smoke-tests.

- 🟡 **Test Coverage**: **Security-critical helper scripts have no isolated behavioural tests**. `resolve-auth.sh`, `scrub-secrets.sh`, `validate-source.sh` are factored out for testability but only `audit-cue-phrases.sh` gets per-fixture behavioural assertions in Phase 4 §3. Add parallel behavioural sections to `scripts/test-design.sh` mirroring the audit-cue-phrases pattern.

- 🟡 **Correctness**: **Resolver tiebreaker by directory-name sort is unsafe under clock skew or manual mv**. Multi-match fallback assumes the system clock moves monotonically and no human/tool ever renames a directory — both break under NTP correction backwards, restoring from backup, manually fixing a typo. Either record an explicit monotonic `sequence: N` in inventory frontmatter, use directory mtime as secondary tiebreaker, or document the assumption explicitly with a warning whenever the tiebreaker fires.

#### Minor

- 🔵 **Code Quality**: Success Criteria still cite stale per-phase test-script filenames (`test-design-foundation.sh`, `test-design-agents.sh`, `test-design-skills.sh`) — replace with `bash scripts/test-design.sh`.
- 🔵 **Code Quality**: Phase 0 §2 hand-waves the audit of existing inverted `assert_contains` call sites — make it concrete with explicit per-call-site review.
- 🔵 **Code Quality**: `browser_evaluate` forbidden-pattern test loops over substrings; passes if patterns appear anywhere in agent body, not necessarily in a Forbidden section. Use a structured marker block.
- 🔵 **Test Coverage**: Audit fixtures only exercise `we need to` and `users need` — add fixtures for `the system must` and `implement [A-Z]` (and a negative for `implement [a-z]`).
- 🔵 **Test Coverage**: Helper scripts under `skills/design/*/scripts/` don't have `assert_file_executable` checks.
- 🔵 **Test Coverage**: MCP failure modes 2 (binaries missing) and 3 (mid-crawl error) have no automated coverage — at minimum a structural assertion that SKILL.md enumerates all three messages.
- 🔵 **Test Coverage**: Atomic `.tmp/` cleanup contract has no test — add an eval that hand-creates a `.YYYY-MM-DD-HHMMSS-source.tmp/` and asserts the resolver ignores it.
- 🔵 **Test Coverage**: Sub-second collision still possible if HHMMSS is the only disambiguator.
- 🔵 **Correctness**: Screenshot-byte-budget exhaustion writes inventory without `status: incomplete` — asymmetric with page-cap and wall-clock handling.
- 🔵 **Correctness**: Three-retry abort in audit loop has no idempotency contract (does revision regress passing sections? frontmatter timestamp drift? orphan files?).
- 🔵 **Correctness**: Atomic `mv` of new directory races with concurrent resolver reads (multi-match window between rename and supersede mutation).
- 🔵 **Security**: Pre-write secret scrubber is literal-substring only; bypassed by URL-encoded, base64, whitespace-split forms.
- 🔵 **Security**: Screenshot mask selectors miss credentials in `type=text`, `<pre>` / `<code>` blocks, post-clipboard reveals.
- 🔵 **Security**: Host allowlist needs to apply to redirects, not just initial location — SSRF via redirect to 169.254.169.254 still possible.
- 🔵 **Security**: No automated check or process for reviewing `@playwright/mcp` version when the pin is bumped.
- 🔵 **Standards**: New `skills/design/` insertion point in `plugin.json` skills array not pinned.
- 🔵 **Standards**: Brace-style placeholder convention asserted but not enforced by tests.
- 🔵 **Usability**: Audit hard-abort leaves no recoverable draft — write rejected body to `.draft.md`.
- 🔵 **Usability**: `claude mcp` error message doesn't name the specific server (`playwright`).
- 🔵 **Usability**: README example uses `https://prototype.example.com` which won't actually crawl — annotate as placeholder.
- 🔵 **Documentation**: New helper scripts have no SKILL.md-level usage documentation requirement.
- 🔵 **Documentation**: Crawler Modes documented in two places (SKILL body + README) without designating one as authoritative.
- 🔵 **Documentation**: CHANGELOG Visualiser-line edit is conditional ("Recommend the rephrase") rather than committed.
- 🔵 **Documentation**: Security considerations subsection placement relative to Authenticated browser crawls and Prerelease Versions not pinned.
- 🔵 **Documentation**: Migration Notes does not mention browser MCP enable prompt or Playwright binary install for upgraders.
- 🔵 **Architecture**: Cue-phrase audit retry loop couples skill termination to LLM stochasticity.
- 🔵 **Architecture**: Reusable browser/auth helpers live under inventory-design — extraction-when-second-consumer-arrives implied but not committed.
- 🔵 **Architecture**: Resolver tiebreaker treats date prefix as authoritative while frontmatter status remains writable — dual-state without a single authoritative model.

### Assessment

The critical risks identified in pass 1 are decisively addressed and the plan is no longer blocked on safety/correctness fundamentals. The new majors are localised regressions or blind-spots from the revision itself rather than architectural gaps:

- 3 of the 5 new majors converge on a single root cause (argument-hint convention contradiction) — a one-pass reconciliation across the spec, the test assertions, and the README example fixes all three.
- The cue-phrase audit regex issue (2 lenses) is a one-line fix (`grep -iE` + extracted regex constant) plus a fixture addition.
- The remaining majors (run count, security helper tests, resolver clock-skew) are each one-pass plan edits.

**Recommendation**: One more focused revision pass addressing the 5 deduplicated majors should bring the plan to APPROVE. The minors can be addressed by the implementer at write time or as follow-ups. The plan is in materially better shape than at the start of the review cycle and ready for a final tightening pass before implementation begins.

## Re-Review (Pass 3) — 2026-05-03

**Verdict:** REVISE (author-response pass — all 5 Pass 2 majors addressed; verdict held pending fresh agent re-review)

This pass is an **author-response** rather than a fresh agent-driven review: it documents the targeted edits made in response to Pass 2's 5 deduplicated majors and 4 selected minors, and marks each Pass 2 finding as Addressed with a citation. A future Pass 4 (if requested) would re-spawn the lens agents to confirm verdict change to APPROVE.

### Pass 2 Major Findings — All Addressed

- 🟡 → ✅ **`argument-hint` convention contradicts itself and the cited exemplar** (Code Quality / Standards / Usability — 3 lenses converged): Reconciled by matching `extract-work-items` precedent. Phase 3 §1 spec now uses `[source-id] [location] [--crawler ...] (default: hybrid for code-repo, runtime otherwise)` with `[]` for both required and optional positionals; tests at Phase 3 §5 and Phase 4 §3 already used `[]`; Phase 5 README skills table already used `[]`. All four locations now consistent.

- 🟡 → ✅ **Cue-phrase audit regex is narrower than the contract it enforces** (Code Quality / Correctness — 2 lenses converged):
  - Regex now sourced from a new `scripts/extract-work-items-cue-phrases.txt` shared source-of-truth file (one ERE alternative per line) — added as a Phase 4 deliverable. The `extract-work-items` SKILL.md is required to remain in agreement, asserted by a structural test.
  - Audit applies the regex case-insensitively (`grep -iE`) so prose written in sentence case matches.
  - Behavioural test fixtures expanded: compliant fixture now exercises all four cue patterns capitalised (`We need to`, `Users need`, `The system must`, `Implement Search`); new negative fixture for `implement <lowercase>`; assert the source-of-truth file exists.
  - Hard-abort path now writes a recoverable `.draft.md` so the user can hand-edit and rerun the audit, addressing the related Pass 2 usability minor in the same edit.
  - Skill diff-asserts that revisions don't regress passing sections; frontmatter timestamp generated once at first attempt and reused across retries (addresses the Pass 2 idempotency minor).

- 🟡 → ✅ **Run count per eval not raised despite expanding eval lists** (Test Coverage): Explicit run-count tiering tables added to both Phase 3 §4 and Phase 4 §2:
  - Phase 3 (15 evals): probabilistic (#1, #2, #3, #4, #6, #7, #15) at ≥5 runs; deterministic structural/validation (#5, #8, #10, #11, #12, #13) at 3 runs; pure structural assertion (#14) at 1 run.
  - Phase 4 (9 evals): probabilistic (#1, #2, #3, #5, #7, #8) at ≥5 runs; deterministic (#4, #6, #9) at 3 runs.
  - Tiering documented at the top of `benchmark.json` so future contributors don't blanket-set to 3.

- 🟡 → ✅ **Security-critical helper scripts have no isolated behavioural tests** (Test Coverage): Three new behavioural test sections added to `scripts/test-design.sh`:
  - `validate-source.sh`: 11 assertions covering scheme allowlist (https accept; file/javascript/data reject), host allowlist (localhost/loopback/RFC1918/link-local reject), and path containment (`..` escape rejected).
  - `resolve-auth.sh`: header-precedence with stderr warning naming ignored vars; all-three-form-vars resolves to `form`; partial credentials fail-fast naming the missing var; no-vars resolves to `none`.
  - `scrub-secrets.sh`: clean body passes; literal env-var value triggers; error names the env var by name (not value).

- 🟡 → ✅ **Resolver tiebreaker by directory-name sort is unsafe under clock skew or manual mv** (Correctness): New monotonic `sequence: N` frontmatter field computed at write time (`max(existing) + 1`). Resolver tiebreaker order (Phase 4 §1 step 1e) updated to:
  1. Primary: highest `sequence` (robust to NTP backwards correction, backups, manual rename).
  2. Secondary: directory mtime, newest first (concurrent writes that read same `max`).
  3. Final: `YYYY-MM-DD-HHMMSS` directory-name prefix.
  - Phase 4 eval #7 expanded to test sequence-based and mtime-based tiebreakers.
  - Phase 3 §1 supersede protocol updated: step 1 computes the sequence; in-progress `.tmp/` directories are explicitly excluded from glob and resolver scans (addresses the Pass 2 atomic-mv-races minor).
  - `screenshots_incomplete: true` frontmatter signal added for screenshot-byte-budget exhaustion (addresses the Pass 2 asymmetric-bound minor).

### Pass 2 Minors — Selected Addressed

In addition to the four minors covered by the major edits above, the following Pass 2 minors were directly addressed:

- 🔵 → ✅ **Stale per-phase test-script filenames in Success Criteria**: All four occurrences (`test-design-foundation.sh`, `test-design-agents.sh`, `test-design-skills.sh`) replaced with `bash scripts/test-design.sh` plus a section reference. The Phase 1 §5 consolidation note is now consistent end-to-end.

- 🔵 → ✅ **Audit fixtures don't exercise all four cue patterns**: Compliant fixture now includes all four (`we need to`, `users? need`, `the system must`, `implement [A-Z]`) at capitalised forms; negative fixture added for `implement <lowercase>`.

- 🔵 → ✅ **Audit hard-abort leaves no recoverable draft**: Rejected body now written to `.draft.md` sibling for hand-edit and re-audit.

- 🔵 → ✅ **Three-retry abort idempotency contract**: Specified that revisions are scoped to failing sections (passing sections must be byte-identical across attempts via diff assertion); frontmatter timestamps generated once at first attempt; abort cleanly removes tmp/partial state.

- 🔵 → ✅ **Atomic mv races with concurrent resolver reads** (multi-match window): Resolver now skips leading-dot directories (`.tmp/` invisible during writes); sequence-based primary tiebreaker eliminates the window's user-visible warning noise.

- 🔵 → ✅ **Screenshot-byte-budget exhaustion writes inventory without `status: incomplete`**: New `screenshots_incomplete: true` frontmatter signal for the byte-budget case (the page-cap and wall-clock cases continue to set `status: incomplete`).

### Pass 2 Minors — Deferred (acceptable for implementer pickup or follow-up)

- 🔵 Phase 0 §2 hand-waves audit of existing inverted `assert_contains` call sites — left as implementer task with the existing CI-greenness gate.
- 🔵 `browser_evaluate` forbidden-pattern test loops over substrings — could be tightened with a structured marker block, deferred.
- 🔵 Helper scripts under `skills/design/*/scripts/` don't have `assert_file_executable` checks — partially addressed (the Pass 3 helper-script test sections include them for `audit-cue-phrases.sh`, `validate-source.sh`, `resolve-auth.sh`, `scrub-secrets.sh`); remaining metadata scripts deferred.
- 🔵 MCP failure modes 2 (binaries missing) and 3 (mid-crawl) have no automated coverage — at minimum the SKILL.md prose enumerates all three with documented messages; eval coverage deferred.
- 🔵 Atomic `.tmp/` cleanup contract has no test — partially addressed (resolver now explicitly skips leading-dot directories); a hand-created `.tmp/` ignore-eval is deferred.
- 🔵 Sub-second collision still possible if HHMMSS is the only disambiguator — now mitigated by sequence-based resolution; sub-second collision becomes equal-sequence which mtime then disambiguates.
- 🔵 Pre-write secret scrubber is literal-substring only (URL-encoded, base64, whitespace-split forms not detected) — documented limitation; encoding-aware variants deferred.
- 🔵 Screenshot mask selectors miss credentials in `type=text`, `<pre>` / `<code>`, post-clipboard — documented in README Security considerations as user responsibility; opt-in no-screenshot mode deferred.
- 🔵 Host allowlist needs to apply to redirects — deferred; current bound is initial-URL only.
- 🔵 No automated check on `@playwright/mcp` version bump — deferred (CI guard against `@latest` in place; pin-bump review process is a CONTRIBUTING.md addition).
- 🔵 New `skills/design/` insertion point in `plugin.json` skills array not pinned — deferred to implementer.
- 🔵 Brace-style placeholder convention not enforced by tests — deferred.
- 🔵 `claude mcp` error message doesn't name the specific server — deferred wording polish.
- 🔵 README example uses `https://prototype.example.com` which won't actually crawl — deferred (acceptable as placeholder pattern).
- 🔵 Helper script SKILL.md-level header docs requirement — deferred to implementer convention.
- 🔵 Crawler Modes drift between SKILL body and README — deferred (cross-reference convention left to implementer).
- 🔵 CHANGELOG Visualiser-line edit conditional — deferred (recommendation stands, exact wording at implementer discretion).
- 🔵 Security considerations subsection placement — deferred (relative ordering left to implementer).
- 🔵 Migration Notes mention browser MCP / Playwright binary install — deferred (covered in CHANGELOG Notes block).
- 🔵 Architecture: cue-phrase audit retry coupling, helpers under skill not shared, resolver dual-state — partially addressed (sequence-based tiebreaker establishes a single primary source of truth); follow-ups deferred.

### Assessment

All 5 Pass 2 deduplicated majors are addressed with concrete, citable edits to the plan. Six minors are also addressed in the same edits. The remaining ~20 minors are deferred to implementer or follow-up — none are blocking.

The plan now incorporates ~55 of the original 65 review findings across all three review passes. It is ready for implementation pending a fresh agent re-review pass to formally confirm verdict change to APPROVE. The author estimates Pass 4 would land at APPROVE based on the Pass 2 majors being decisively addressed and the absence of new architectural surface area in Pass 3 edits.

**Recommendation**: Either (a) run Pass 4 to confirm APPROVE, or (b) accept the plan for implementation now and let any residual concerns surface during implementation as targeted PR feedback rather than as plan revisions.

## Re-Review (Pass 4) — 2026-05-03

**Verdict:** APPROVE

Fresh agent re-review across all 8 lenses confirms the plan is ready for implementation. Every lens returned APPROVE with zero new major+ findings.

### Per-Lens Verdicts

| Lens          | Verdict   | Pass 2 majors resolved | New majors | Notes                                                                  |
|---------------|-----------|------------------------|------------|------------------------------------------------------------------------|
| Architecture  | APPROVE   | n/a (no Pass 2 majors) | 0          | Sequence-based primary tiebreaker resolves the dual-state concern      |
| Code Quality  | APPROVE   | 1/1 (regex case)       | 0          | Shared regex file + grep -iE + expanded fixtures                       |
| Test Coverage | APPROVE   | 2/2 (run count, helper tests) | 0   | Per-eval tiering + 24 new helper-script assertions                     |
| Correctness   | APPROVE   | 2/2 (regex narrowness, tiebreaker) | 0 | Sequence/mtime/date-prefix layered fallback                            |
| Security      | APPROVE   | n/a (no Pass 2 majors) | 0          | No regressions; sequence further mitigates frontmatter tampering       |
| Standards     | APPROVE   | 1/1 (argument-hint)    | 0          | All 5 locations consistent with `[]` per `extract-work-items` precedent |
| Usability     | APPROVE   | 1/1 (argument-hint)    | 0          | Consistent + draft-on-abort + case-insensitive prose matching          |
| Documentation | APPROVE   | n/a (no Pass 2 majors) | 0          | All Pass 3 deltas documented inline; per-phase Success Criteria consistent |

### Key Confirmations

**All 5 Pass 2 deduplicated majors are decisively resolved**:

1. **`argument-hint` convention contradiction** (Code Quality + Standards + Usability) — Standards agent verified all 5 locations now consistently use `[]` brackets matching the cited `extract-work-items` precedent. No reviewer flagged any remaining contradiction.

2. **Cue-phrase audit regex too narrow** (Code Quality + Correctness) — Both reviewers confirmed the shared `scripts/extract-work-items-cue-phrases.txt` source-of-truth file plus `grep -iE` case-insensitive matching plus expanded fixtures (all four cue patterns capitalised + lowercase-implementer negative) is a clean DRY resolution.

3. **Run count per eval not raised** (Test Coverage) — Reviewer confirmed the explicit per-eval tiering tables in Phase 3 §4 and Phase 4 §2 give each eval a justified run-count category (probabilistic ≥5, deterministic 3, structural-only 1) with eval IDs listed against each tier.

4. **Security helper scripts have no isolated tests** (Test Coverage) — Reviewer confirmed the three new behavioural test sections (validate-source.sh: 11 assertions, resolve-auth.sh: 5 with stderr/exit-code coverage, scrub-secrets.sh: 3 including secret-name-not-value verification) make the security-critical contracts executable.

5. **Resolver tiebreaker unsafe under clock skew** (Correctness) — Reviewer confirmed the layered sequence → mtime → directory-name fallback eliminates the entire class of clock-skew/manual-rename failure modes; sequence is the right primary because it is computed at write time as `max(existing) + 1`, monotonic and immune to system-clock manipulation.

### New Issues Introduced

**None.** All 8 lens agents returned empty `new_findings` arrays.

### Cross-Cutting Strengths Identified by Pass 4

Multiple lenses independently flagged the same architectural choices as strong:

- **Phase 0 test-helper reconciliation** (Architecture + Code Quality + Test Coverage): standalone refactor that unblocks subsequent phases without functional change.
- **Sequence-based primary tiebreaker** (Architecture + Correctness + Security): single ordering primitive that simultaneously resolves clock-skew correctness, dual-state architectural ambiguity, and frontmatter-tampering attack surface.
- **Helper script behavioural tests using `env -i`** (Code Quality + Test Coverage + Security): pin-environment discipline prevents test-env leakage and makes security-critical contracts executable in isolation.
- **Shared cue-phrase regex source-of-truth** (Architecture + Code Quality + Correctness + Documentation): eliminates cross-skill drift hazard with a single file consumed by both the audit script and the extract-work-items SKILL.md (verified by structural drift assertion).
- **Leading-dot directory exclusion** (Architecture + Correctness + Security): closes the `.tmp/` race window cleanly and matches the `.draft.md` recovery-path convention.

### Assessment

The plan is **ready for implementation**. Across four review passes the plan has incorporated ~60 of the original 65 review findings; all 5 critical and all majors are resolved or partially-resolved with documented residual; remaining minors are deferred follow-ups appropriate for implementer pickup or post-merge PRs.

**The plan is APPROVED.** Move forward with `/accelerator:implement-plan` when ready.
