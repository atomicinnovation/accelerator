---
date: "2026-05-19T12:34:06Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-19-inventory-design-and-browser-agent-fixes"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, documentation, compatibility, usability]
review_pass: 5
status: complete
id: "2026-05-19-inventory-design-and-browser-agent-fixes-review-1"
title: "2026-05-19-inventory-design-and-browser-agent-fixes-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-19T12:34:06Z"
last_updated_by: Toby Clemson
---

## Plan Review: Inventory-Design and Browser-Agent Fixes

**Verdict:** REVISE

The plan is structurally sound — it correctly identifies a single architectural
failure mode (implicit contracts between a skill and its spawned processes/
agents) and applies a consistent remedy that mirrors the established
`documents-locator` / `accelerator:paths` precedent. TDD discipline is good,
the previously-untested `owner-exited` code path is finally exercised, and
phases are independently shippable. However, eight lenses surfaced one
critical correctness defect and a recurring cross-cutting concern that the
plan's contract is enforced by prose rather than mechanism: the
`ACCELERATOR_PLAYWRIGHT_NO_OWNER` env var does not propagate from the
spawning skill into sub-agent processes, so the very PIDs that motivate
the fix still hit the broken `$$` path. Several other issues compound this
(env-var truthiness, `links` returning raw hrefs without scrubbing, missing
PROTOCOL.md update, migration-0004 dependency).

### Cross-Cutting Themes

- **Env-var-based escape hatch is structurally weak** (flagged by:
  correctness, architecture, code-quality, security, usability) — the
  `ACCELERATOR_PLAYWRIGHT_NO_OWNER=1` contract is set in the parent skill's
  bash blocks but does NOT inherit into sub-agent shells, where the agent's
  own `run.sh` invocations re-introduce the failure mode. Several lenses
  independently recommended either making this an automatic detection in
  `run.sh` or moving the contract into the agent bodies.
- **Env-var truthiness ("0" / "false" treated as truthy)** (flagged by:
  correctness, security, code-quality, usability) — `[ -n "${...:-}" ]`
  accepts any non-empty value, including `0` and `false`. Negative-polarity
  naming compounds the trap.
- **`links` command returns raw, unfiltered, unresolved hrefs** (flagged by:
  architecture, code-quality, correctness, security, usability) — `href`
  comes from `getAttribute('href')` not `a.href`, so relative URLs are
  unresolved; no same-origin filter at the daemon boundary; no query-string
  scrubbing (sensitive tokens leak); `role: 'link'` default conflates
  explicit vs implicit ARIA role; `textContent.trim()` leaves embedded
  whitespace.
- **PROTOCOL.md not updated for new `links` command and new env var**
  (flagged by: documentation, compatibility) — the canonical wire-protocol
  reference is silent on the new command, violating its own stability
  commitment.
- **Cross-phase ordering and `<run.sh>` placeholder convention**
  (flagged by: code-quality, usability) — angle-bracket placeholders in
  agent bodies collide with existing `<url>` / `<ref>` placeholders for
  user-supplied values and diverge from the `documents-locator` curly-brace
  convention.
- **Hardcoded executor path in `config-read-browser-executor.sh`**
  (flagged by: architecture, code-quality, correctness) — three string
  copies of the same path now exist (resolver, allowed-tools glob,
  launcher's own `$SCRIPT_DIR` resolution).

### Tradeoff Analysis

- **Security vs Usability on env-var contract**: tightening
  `ACCELERATOR_PLAYWRIGHT_NO_OWNER` to literal `1` (security recommendation)
  reduces accidental disable, but a positively-named alternative
  (usability recommendation, e.g. `ACCELERATOR_PLAYWRIGHT_OWNER_PID=0` or
  `ACCELERATOR_PLAYWRIGHT_OWNER_WATCH=disabled`) solves both at once and
  reuses the existing `--owner-pid 0` idiom. Prefer the positively-named
  alternative.
- **Architecture vs Scope on `links` daemon-side enrichment**: making the
  daemon resolve hrefs, filter cross-origin, and strip query strings
  (architecture + security recommendations) adds daemon complexity but
  removes per-agent normalisation duty. Doing it once at the daemon is
  consistent with the existing trust-boundary posture and worth the
  ~10-line addition.

### Findings

#### Critical

- 🔴 **Correctness**: Env var ACCELERATOR_PLAYWRIGHT_NO_OWNER does not propagate from spawning skill into sub-agent shells
  **Location**: Phase 2, Step 5: Have inventory-design set the env var before agent spawn
  Sub-agents run in isolated processes/shells. A bash `export` in the parent skill's bash block does not flow into a subagent's later `Bash` tool invocations of `run.sh`. The ephemeral shells that browser-locator and browser-analyser themselves spawn will still pass `$$` as `--owner-pid`, re-introducing the failure mode. The headline production bug is only partially mitigated.

#### Major

- 🟡 **Architecture**: Cross-turn daemon persistence is enforced by prose, not by mechanism
  **Location**: Phase 2, Step 5
  Eight bash blocks in the skill plus every agent bash invocation must remember to set the env var; any future edit silently regresses. Make the contract a property of the executor entry point (auto-detect ephemeral parent, or flip default).
- 🟡 **Code Quality**: Hardcoded executor path duplicates source-of-truth
  **Location**: Phase 3, Section 2 (config-read-browser-executor.sh)
  Three string copies of `skills/design/inventory-design/scripts/playwright/run.sh` will silently drift. Consider a `PATH_KEYS` entry or glob-discovery.
- 🟡 **Code Quality**: `links` snippet conflates default anchor role with explicit ARIA role
  **Location**: Phase 4, Section 2 (links command)
  `role: a.getAttribute('role') || 'link'` bakes a semantic inaccuracy into the protocol — callers cannot distinguish 'no role' from 'role=link'. Return verbatim.
- 🟡 **Code Quality**: `textContent.trim()` collapses multi-line link text inconsistently
  **Location**: Phase 4, Section 2 (links command)
  Real anchors with embedded SVG/whitespace yield noisy strings. Normalise once in the executor with `/\s+/g`.
- 🟡 **Test Coverage**: New daemon owner-exited test skips the Playwright-availability guard
  **Location**: Phase 2, Step 1
  Every existing daemon.test.js spawn applies the `resolvePlaywrightNsRoot()` early-return; new test omits it. Will be the first daemon test that diverges from the pattern.
- 🟡 **Test Coverage**: Env-var integration test (test-run.sh) is prose-only — not concrete enough to write first
  **Location**: Phase 2, Step 3
  Unlike every other test step, no bash snippet is provided. Inline the script the same way Phase 1 / Phase 4 do.
- 🟡 **Correctness**: `-n` check treats `ACCELERATOR_PLAYWRIGHT_NO_OWNER=0` and `=false` as 'disable'
  **Location**: Phase 2, Step 4 (run.sh)
  Subtle user-facing surprise. Either tighten to `= "1"` or document explicitly.
- 🟡 **Correctness**: Owner-exited test relies on Node reaping the spawned `sleep` zombie before the next watcher tick
  **Location**: Phase 2, Step 1
  Production failure mode (Bash-tool ephemeral shells reaped by harness) differs from test environment (Node auto-reaps). Possible CI flake.
- 🟡 **Correctness**: `--owner-pid 0 ignores owner death` test never kills any process
  **Location**: Phase 2, Step 2
  Test name is misleading; it asserts only that the daemon does not self-shutdown for an unrelated reason within 600ms. Rename or spawn an owner and kill it.
- 🟡 **Correctness**: Integration test doesn't exercise production failure mode it claims to differentiate
  **Location**: Phase 2, Step 3
  Asserts only `=1` case; without a complementary negative test, regressions in env-var precedence won't be caught.
- 🟡 **Security**: ACCELERATOR_PLAYWRIGHT_NO_OWNER extends auth-bearing browser context lifetime to 30-min idle ceiling
  **Location**: Phase 2 overall
  Cached auth headers and session cookies survive in memory for up to IDLE_MS. Lower IDLE_MS when the escape hatch is set, and require cleanup on error paths.
- 🟡 **Security**: `links` returns raw hrefs (including query strings) without scrubbing
  **Location**: Phase 4, Section 2
  Pages with `?token=...` or `?code=...` URLs surface tokens to agent context even though final inventory.md is scrubbed.
- 🟡 **Documentation**: PROTOCOL.md not updated for new `links` command
  **Location**: Phase 4 overall
  `skills/design/inventory-design/PROTOCOL.md` is the canonical wire reference; every other command has a full subsection.
- 🟡 **Documentation**: New `ACCELERATOR_PLAYWRIGHT_NO_OWNER` env var has no documented home
  **Location**: Phase 2 overall
  Other ACCELERATOR_PLAYWRIGHT_* env vars are also undocumented; this one is now a hard requirement. Add an environment-variable reference.
- 🟡 **Compatibility**: Phase 1 renames bare-key call sites without verifying users have run migration 0004
  **Location**: Phase 1
  Users who upgrade plugin before running `/accelerator:migrate` silently fall back to default paths (worse than today's loud warning). Add a migration check or transitional warning.
- 🟡 **Compatibility**: Reliance on Claude Code subagent `skills:` preload mechanism is undocumented and version-fragile
  **Location**: Phase 3 overall
  No minimum Claude Code version pinned; no integration test covering 'subagent receives preloaded skill body'. A future Claude Code release could silently break the fix.
- 🟡 **Usability**: Angle-bracket `<run.sh>` placeholder diverges from `{key}` convention and is ambiguous
  **Location**: Phase 3, Step 4
  Mixes with existing `<url>`/`<ref>` payload placeholders on the same lines. Use `{run.sh}` or `${BROWSER_EXECUTOR}` for consistency.
- 🟡 **Usability**: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` is a negative-polarity boolean
  **Location**: Phase 2, Step 4
  Double-negative naming + truthy-on-"0" combine for max confusion. Rename to positive-polarity or reuse `--owner-pid 0` via `ACCELERATOR_PLAYWRIGHT_OWNER_PID`.
- 🟡 **Usability**: Cross-turn persistence rationale lives in inventory-design/SKILL.md, not co-located with run.sh toggle
  **Location**: Phase 2, Step 5
  A developer reading run.sh first sees the `if [ -n ... ]` with no comment. Add an inline rationale comment in run.sh.

#### Minor

- 🔵 **Architecture**: New preloaded skill is a single-value variant of `paths` rather than an extension of it
  **Location**: Phase 3, Step 2-3
  Pattern will proliferate; consider folding into `paths` skill or a `resolved-locations` registry.
- 🔵 **Architecture**: `links` command surfaces raw `<a href>` set with no normalisation, same-origin filter, or cap at the daemon boundary
  **Location**: Phase 4, Step 2
  Push filtering responsibility from agent prose to daemon — add `resolved_href`, `same_origin`, `scheme` fields.
- 🔵 **Architecture**: Locator's route-discovery still mixes `navigate` with anchor enumeration, leaving SPA failure partially open
  **Location**: Phase 4, Step 3
  On a same-shell SPA, fabricated paths still return 200. Add server-side reachability fingerprint or restructure the strategy.
- 🔵 **Architecture**: Hardcoded executor path creates hidden coupling to inventory-design directory
  **Location**: Phase 3, Step 1-2
  See Code Quality finding above; this is the architecture-lens framing.
- 🔵 **Code Quality**: Test uses fixed `setTimeout(1500)` rather than polling for outcome
  **Location**: Phase 2, Step 1
  Add a `waitForStopped(stateDir, reason, ms)` helper alongside the existing `waitForInfo`.
- 🔵 **Code Quality**: `--owner-pid 0` test's negative assertion has no time bound that maps to watcher behaviour
  **Location**: Phase 2, Step 2
  Timing rationale is misleading; spawn a short-lived owner *and* set `--owner-pid 0`, then kill the owner.
- 🔵 **Code Quality**: Adding `links` to `BLOCKING_OPS` is implicit; no rationale captured
  **Location**: Phase 4, Step 2
  Add a one-line comment to BLOCKING_OPS and a wall-clock-arming test for `links`.
- 🔵 **Code Quality**: Agent body uses `<run.sh>` placeholder requiring per-invocation substitution
  **Location**: Phase 3, Sections 3-4
  See cross-cutting theme above.
- 🔵 **Code Quality**: `assert_not_contains` against `config-read-path.sh design_inventories` masks substring matches
  **Location**: Phase 1, Step 1
  Use stricter grep with word boundaries or replace with a repo-wide grep.
- 🔵 **Test Coverage**: Test does not cover empty-string vs unset distinction for the escape-hatch env var
  **Location**: Phase 2, Step 4
  Add an unset-case assertion that preserves original `$$` behaviour.
- 🔵 **Test Coverage**: Sleep-based synchronisation in owner-exited test
  **Location**: Phase 2, Step 1
  Same as the Code Quality "fixed setTimeout(1500)" finding; deduplicated.
- 🔵 **Test Coverage**: Two of the four 'red' steps are actually backfill, not TDD
  **Location**: Phase 2, Step 1 / Step 2
  Relabel as 'Coverage backfill (passes immediately on existing code)'.
- 🔵 **Test Coverage**: Fixture test does not cover locator's normalisation/filtering responsibility
  **Location**: Phase 4, Step 1
  Either pin the locator prose with a test-design.sh assertion, or document the gap explicitly.
- 🔵 **Test Coverage**: No test exercises wall-clock arming for the new `links` command
  **Location**: Phase 4, Step 2
  Promote the manual grep to a test-design.sh assertion on BLOCKING_OPS membership.
- 🔵 **Test Coverage**: New assertions iterate over hardcoded skill list with no guard against drift
  **Location**: Phase 1, Step 1
  Replace with `grep -rE 'config-read-path\.sh design_(inventories|gaps)\b'`.
- 🔵 **Correctness**: `links` may be invoked before any navigate; output shape on about:blank not specified
  **Location**: Phase 4, Step 2
  Return `page.url()` alongside `links`; agent body should require `navigate` first.
- 🔵 **Correctness**: Returning raw `getAttribute('href')` does not resolve relative URLs
  **Location**: Phase 4, Step 2
  Use `a.href` (resolved) or emit both `href` and `resolved` fields.
- 🔵 **Correctness**: Phase 4 `links` test relies on Phase 2's fix being landed first
  **Location**: Phase 4, Step 1
  Set `ACCELERATOR_PLAYWRIGHT_NO_OWNER=1` in the Phase 4 test environment to decouple.
- 🔵 **Correctness**: Path emitted by config-read-browser-executor.sh is fixed; no boundary case for missing run.sh
  **Location**: Phase 3, Step 2
  Add `test -x` and fail with a clear error.
- 🔵 **Security**: Escape hatch trigger is any non-empty value
  **Location**: Phase 2, Step 4
  See Correctness finding above.
- 🔵 **Security**: `links` not subject to any per-origin allowlist
  **Location**: Phase 4, Step 2
  See Architecture finding above.
- 🔵 **Documentation**: browser-analyser.md body rewrite is under-specified
  **Location**: Phase 3, Step 4
  Analyser has a richer Tools section (7 commands + evaluate allowlist) than locator. Enumerate which sections are preserved.
- 🔵 **Documentation**: CHANGELOG.md not updated for any of the four fixes
  **Location**: Cross-cutting
  Past similar additive changes received CHANGELOG entries.
- 🔵 **Documentation**: Cross-turn persistence rationale wording deferred to implementation
  **Location**: Phase 2, Step 5
  Drop 'can be tightened' — lock the rationale.
- 🔵 **Documentation**: Step 8 prose update is too terse for a non-obvious contract
  **Location**: Phase 3, Step 5
  Expand to name the failure mode and the precedent.
- 🔵 **Documentation**: Analyser-side `links` documentation rationale is thin
  **Location**: Phase 4, Step 4
  Either add a "when to use" note or drop the entry.
- 🔵 **Compatibility**: Truthiness contract for `ACCELERATOR_PLAYWRIGHT_NO_OWNER` is permissive in a way that may surprise
  **Location**: Phase 2, Step 4
  Same finding as Correctness/Security/Usability above; deduplicated.
- 🔵 **Compatibility**: `links` command not documented in PROTOCOL.md
  **Location**: Phase 4
  Same finding as Documentation above; deduplicated.
- 🔵 **Compatibility**: Visualise SKILL.md assertion lives in test-design.sh
  **Location**: Phase 1, Step 1
  Add a comment explaining why the visualise path is tested alongside design skills, or move to a sibling test.
- 🔵 **Usability**: `role: 'link'` default obscures whether an anchor had an explicit role attribute
  **Location**: Phase 4, Step 2
  Same finding as Code Quality above; deduplicated.
- 🔵 **Usability**: Adding `links` to browser-analyser 'for symmetry' adds cognitive load without a use case
  **Location**: Phase 4, Step 4
  Same finding as Documentation above; deduplicated.
- 🔵 **Usability**: Maintainer note about `user-invocable: false` is duplicated across paths and browser-executor
  **Location**: Phase 3, Step 3
  Extract the rationale into a single doc location.

#### Suggestions

- 🔵 **Code Quality**: Naming inconsistency between `browser-executor` skill and the script `run.sh` (also called "the executor")
  **Location**: Phase 3, cross-cutting
  Consider `browser-executor-path` or fold into `paths`.
- 🔵 **Security**: Preloaded absolute path discloses plugin install location into agent transcripts
  **Location**: Phase 3
  Low risk; defence-in-depth only.
- 🔵 **Security**: Plan does not require Step 12 (daemon-stop) to run on agent-failure paths
  **Location**: Phase 2, Step 5
  Cleanup-on-error contract; add `server-stopped.json` assertion after failure path.
- 🔵 **Usability**: Daemon env-var surface has no documented index
  **Location**: Phase 2, cross-cutting
  Add an Environment Variables section to the playwright/ README or top-of-file comment.
- 🔵 **Usability**: Pattern for 'preloaded executor-path skill' is now established by two examples but never documented as a pattern
  **Location**: Phase 3
  Consider a follow-up ADR or skills/config README.
- 🔵 **Usability**: Five-site rename has no transitional discoverability for developers reading old docs/diffs
  **Location**: Phase 1
  Consider a 'this key was renamed' hint from config-read-path.sh.

### Strengths

- ✅ Correctly identifies a single architectural failure mode (implicit skill/spawn contracts) across four superficially-different defects and applies a consistent remedy
- ✅ Reuses the established preloaded-skill precedent (`accelerator:paths` / documents-locator) rather than inventing a parallel mechanism; carries forward the load-bearing `user-invocable: false` vs `disable-model-invocation: true` maintainer note
- ✅ Strictly follows TDD (red-test first, production change, green) for every phase; the structure makes intent reviewable and the resulting code easier to maintain
- ✅ Closes a real test-architecture gap: every existing daemon.test.js case uses `--owner-pid 0` (the disabled branch), so the production code path was completely untested
- ✅ Phase 1 strengthens existing test-design.sh assertions from 'string contains design_inventories' (which passes vacuously) to assertions that explicitly forbid the bare-key call form
- ✅ `links` is added as an additive switch case under the existing `PROTOCOL = 1` envelope with no protocol version bump and no breaking change
- ✅ Phase 4 fixture covers four distinct anchor shapes (relative href, absolute href, role override, empty text) — a good edge-case spread
- ✅ Explicit non-goals (no Step 8 restructure, no `evaluate` policy change, no daemon idle/wall-clock behaviour change) keep each phase tightly scoped and independently revertible
- ✅ Existing security controls preserved: auth-header origin allowlist, evaluate-payload allowlist, URL scrubbing, screenshot masking, secret-literal scrubber
- ✅ References section provides comprehensive precedent citations with line ranges, enabling self-service verification by future maintainers

### Recommended Changes

Ordered by impact. Each change addresses one or more findings cited in parentheses.

1. **Make the daemon-persistence fix self-enforcing rather than caller-disciplined** (addresses: Critical Correctness "env var doesn't propagate"; Architecture "prose-enforced contract"; Usability "rationale not co-located")
   Replace the env-var contract with auto-detection in `run.sh`. Detect the Claude Code harness via a stable signal (e.g. `CLAUDE_PLUGIN_ROOT` set AND stdin is not a tty AND parent process is short-lived) and pass `--owner-pid 0` unconditionally in that case. Keep the env var as an explicit override for tests/edge cases, but reuse the existing `--owner-pid` CLI shape by renaming to `ACCELERATOR_PLAYWRIGHT_OWNER_PID` (numeric; `0` disables). Drop the per-bash-block prose contract entirely. This also resolves the negative-polarity and truthiness issues.

2. **Harden the `links` command at the daemon boundary** (addresses: Code Quality "role default", "textContent.trim"; Correctness "raw getAttribute('href')", "about:blank shape"; Security "raw hrefs leak query strings", "no same-origin filter"; Architecture "no normalisation at daemon", "javascript: hrefs surface"; Usability "role: 'link' default")
   Update the daemon `links` case to return enriched, normalised, server-scrubbed entries:
   ```javascript
   case 'links': {
     const links = await page.evaluate(() => {
       const pageOrigin = location.origin;
       return Array.from(document.querySelectorAll('a[href]')).map(a => {
         const raw = a.getAttribute('href');
         let resolved = null, sameOrigin = false, scheme = null, pathname = null;
         try {
           const u = new URL(a.href);
           resolved = u.toString();
           sameOrigin = u.origin === pageOrigin;
           scheme = u.protocol.replace(':', '');
           pathname = u.pathname;
         } catch {}
         return {
           text: (a.textContent || '').replace(/\s+/g, ' ').trim(),
           href: raw,
           resolved,
           pathname,
           same_origin: sameOrigin,
           scheme,
           role: a.getAttribute('role'),  // verbatim; null if unset
         };
       });
     });
     return { protocol: PROTOCOL, url: page.url(), links };
   }
   ```
   Update the fixture test and locator agent body to consume the richer shape and to use `pathname` for route enumeration (stripping query strings naturally).

3. **Update PROTOCOL.md and document the env-var surface** (addresses: Documentation "PROTOCOL.md not updated", "env var has no documented home"; Compatibility "links not documented")
   Add a `### links` section to `skills/design/inventory-design/PROTOCOL.md` with request shape, response (including new fields from change #2), blocking-op classification, and error codes. Add an `## Environment Variables` section listing `ACCELERATOR_PLAYWRIGHT_OWNER_PID`, `_OWNER_POLL_MS`, `_IDLE_MS`, `_WALL_CLOCK_MS`, `_CACHE`, `_KEEP_STDIO`, `_NS_ROOT` with defaults, meaning, and who sets them. Add a `test-design.sh` assertion that PROTOCOL.md documents every dispatched command in `daemon.js`.

4. **Replace single-value `browser-executor` resolver with a `paths`-style data-driven entry** (addresses: Architecture "single-value variant", "hidden coupling"; Code Quality "hardcoded path duplicates source-of-truth", "naming inconsistency"; Correctness "no boundary case for missing run.sh"; Usability "maintainer-note duplication", "pattern not documented")
   Add a new path key (e.g. `paths.browser_executor` or, since it is plugin-relative not project-relative, a new `EXECUTORS_KEYS` array) to `config-defaults.sh`. Have the existing `accelerator:paths` skill emit a `## Executors` section alongside `## Configured Paths`, sourced from the new array. Browser agents preload the existing `paths` skill (no new SKILL.md needed). Add `test -x` validation to the resolver. This avoids the proliferation tax of one preloaded skill per resource.

5. **Concretise the Phase 2 tests and align with existing patterns** (addresses: Test Coverage "test skips Playwright guard", "prose-only env-var integration test", "two 'red' steps are backfill", "no unset-case test"; Code Quality "fixed setTimeout vs polling", "negative assertion has no time bound"; Correctness "zombie reap dependency", "tests don't exercise production failure mode")
   Specifically:
   - Add the `resolvePlaywrightNsRoot()` early-return to the new owner-exited test.
   - Add `await new Promise(r => owner.on('exit', r))` after `owner.kill('SIGKILL')` before the assertion window.
   - Introduce a `waitForStopped(stateDir, reason, ms)` helper alongside `waitForInfo`; use it instead of fixed sleeps.
   - For the `--owner-pid 0` test: spawn a real short-lived owner, set `--owner-pid 0`, kill the owner, assert the daemon survives. Rename appropriately.
   - Inline the bash for the `test-run.sh` env-var integration test, including a negative-path complement (unset → original `$$` behaviour preserved).
   - Relabel Phase 2 Steps 1 & 2 as 'Coverage backfill' rather than TDD red.

6. **Tighten Phase 1 against migration-0004 drift** (addresses: Compatibility "Phase 1 renames without verifying migration"; Test Coverage "hardcoded skill list with no guard")
   Replace the four-skill enumeration in `test-design.sh` with a single repo-wide assertion:
   ```bash
   assert_exit_code "no SKILL.md uses bare design_(inventories|gaps)" 1 \
     bash -c "grep -rE 'config-read-path\\.sh design_(inventories|gaps)\\b' \"$PLUGIN_ROOT/skills\" \"$PLUGIN_ROOT/agents\""
   ```
   Additionally, enhance `config-read-path.sh` to emit a migration-aware warning when it sees the bare `design_inventories` / `design_gaps` keys in a user's config: "this key was renamed by migration 0004 to research_design_*; run /accelerator:migrate". This protects users who upgrade the plugin before running the migration.

7. **Tighten the cross-phase test posture** (addresses: Correctness "Phase 4 test depends on Phase 2 fix being landed"; Test Coverage "no wall-clock test for links")
   Set the harness env var (`ACCELERATOR_PLAYWRIGHT_OWNER_PID=0` or whatever the renamed var becomes) in the Phase 4 test environment so the `links` test does not entangle with the owner-watcher timing. Add a wall-clock arming test for `links` (e.g. `ACCELERATOR_PLAYWRIGHT_WALL_CLOCK_MS=1` causes wall-clock-exceeded for `links`). Add a `test-design.sh` assertion that `'links'` appears in the `BLOCKING_OPS` initialiser.

8. **Tighten daemon-cleanup posture for the long-lived case** (addresses: Security "auth-bearing context lives to 30-min idle ceiling", "Step 12 doesn't run on failure paths")
   When the daemon is launched with `--owner-pid 0` (escape hatch on), lower the effective `IDLE_MS` ceiling (e.g. 5 min instead of 30 min). Update `inventory-design/SKILL.md` to require `daemon-stop` in a trap-style cleanup that survives error paths, not only in Step 12. Add a test that asserts `server-stopped.json` exists after a forced-error path.

9. **Resolve the placeholder convention divergence** (addresses: Code Quality "<run.sh> requires per-invocation substitution"; Usability "angle-bracket placeholder")
   Stop using `<run.sh>` as a placeholder in agent bodies. Either (a) use `{run.sh}` or (b) define `${BROWSER_EXECUTOR}` in the preloaded block and instruct agents to use the token literally. Match the `documents-locator` precedent of referencing preloaded values directly without prose-level substitution.

10. **Clarify Phase 3 documentation scope and CHANGELOG** (addresses: Documentation "browser-analyser rewrite under-specified", "CHANGELOG not updated", "Step 8 prose too terse", "rationale wording deferred", "analyser-side links rationale thin"; Usability "links on analyser adds cognitive load")
    - Phase 3, Step 4: enumerate exactly which sections of `browser-analyser.md` are touched and which (Tools list past `snapshot`, the `evaluate` payload allowlist, error handling) are preserved verbatim. Add a `test-design.sh` assertion that the evaluate allowlist section is still present.
    - Drop the `links` entry from `browser-analyser.md` until a concrete use case justifies it (resolves the "for symmetry" concern). Keep symmetry in the executor protocol; do not advertise it to an agent that won't use it.
    - Expand the Phase 3 Step 5 note to two short paragraphs that name the failure mode and the precedent.
    - Pin the cross-turn-persistence rationale paragraph verbatim (drop the "can be tightened" caveat).
    - Add a Phase 5 (or a step in each phase) that appends CHANGELOG.md `[Unreleased]` entries: additive `links` command, daemon owner-pid behaviour change, new env var, renamed call sites.

11. **Pin the Claude Code subagent-skills-preload contract** (addresses: Compatibility "preload mechanism is undocumented and version-fragile")
    Add a minimum Claude Code version to the plugin manifest (or document it in the README), and add an integration test (in `test-design.sh` or a new file) that asserts the preloaded `## Browser Executor` block actually appears in an agent context at spawn time. If a real subagent spawn is not feasible in the test harness, at least add a structured runtime check in the agent body that emits a clear error when the expected block is absent.

---

*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound: each defect is correctly diagnosed as a variant of the same failure mode (implicit contracts between a skill and its spawned processes/agents), and each fix follows a precedent already established in the codebase (preloaded skills for path resolution, additive command-dispatch for daemon protocol, env-var escape hatches for harness-specific behaviour). Coupling is reduced rather than increased. Two architectural concerns are worth surfacing: (1) the cross-turn-persistence contract is enforced by prose rather than mechanism, leaving the door open for a fresh bash block to forget to export the env var and silently kill the daemon, and (2) Phase 4's `links` command adds a new browser-side primitive without an accompanying same-origin or anchor-cap policy at the daemon layer.

**Strengths**:
- Correctly identifies a single architectural failure mode and applies a consistent remedy
- Reuses the established preloaded-skill precedent
- `links` added as an additive switch case under existing PROTOCOL=1
- Phase 2 closes a real test-architecture gap
- Migration boundary respected (no alias keys)
- Explicit non-goals keep each phase tightly scoped

**Findings**:
- 🟡 Major: Cross-turn daemon persistence is enforced by prose, not by mechanism
- 🔵 Minor: New preloaded skill is a single-value variant of `paths` rather than an extension of it
- 🔵 Minor: `links` surfaces raw `<a href>` set with no normalisation, same-origin filter, or cap at the daemon boundary
- 🔵 Minor: Locator's route-discovery still mixes `navigate` with anchor enumeration on SPAs
- 🔵 Minor: Hardcoded executor path in `config-read-browser-executor.sh` creates hidden coupling

### Code Quality

**Summary**: The plan is well-structured, follows TDD, and mirrors an established codebase precedent. Several code-quality concerns warrant attention: hardcoded executor path duplicates source-of-truth, the `links` snippet has subtle issues (whitespace handling, role attribute defaulting), and the env-var escape hatch is negative-named. Test consistency and the new test's assertions also have a few rough edges.

**Strengths**:
- TDD discipline throughout
- Adopts the established `skills:` preloaded-skill precedent
- Each phase is independently shippable
- Explicitly chooses migration shape B (rename call sites)
- Closes the test-coverage gap on owner-exited code path

**Findings**:
- 🟡 Major: Hardcoded executor path duplicates source-of-truth
- 🟡 Major: `links` snippet conflates default anchor role with explicit ARIA role
- 🟡 Major: `textContent.trim()` collapses multi-line link text inconsistently
- 🔵 Minor: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` is a negative-named boolean flag
- 🔵 Minor: Test uses fixed `setTimeout(1500)` rather than polling
- 🔵 Minor: Negative-assertion test has no time bound that maps to watcher behaviour
- 🔵 Minor: Adding `links` to `BLOCKING_OPS` is implicit; rationale not captured
- 🔵 Minor: Agent body uses `<run.sh>` placeholder requiring per-invocation substitution
- 🔵 Minor: `assert_not_contains` masks substring matches
- 🔵 Suggestion: Naming inconsistency between `browser-executor` skill and `run.sh`

### Test Coverage

**Summary**: Strong overall: each phase pairs a production change with an explicit failing test, the previously-untested owner-exited code path is finally exercised, and new assertions pin both canonical key names and SKILL.md preload wiring. However, several tests are underspecified at timing, environment, and assertion strength — Phase 2 step 1 omits the Playwright-NS-root skip pattern, Step 3 is prose-only, and two 'red' steps actually start green because they cover pre-existing production code.

**Strengths**:
- Each phase begins with a concrete failing assertion before the production change
- Phase 2 explicitly fills the largest pre-existing coverage gap
- Phase 1 strengthens existing test-design.sh assertions to forbid the bare-key call form
- Phase 4 fixture covers four distinct anchor shapes
- test-design.sh used to pin contracts across phases

**Findings**:
- 🟡 Major: New daemon owner-exited test skips the Playwright-availability guard
- 🟡 Major: Env-var integration test is prose-only — not concrete enough to write first
- 🔵 Minor: Test does not cover empty-string vs unset distinction
- 🔵 Minor: Sleep-based synchronisation in owner-exited test
- 🔵 Minor: Two of the four 'red' steps are actually backfill, not TDD
- 🔵 Minor: Fixture test does not cover locator's normalisation/filtering responsibility
- 🔵 Minor: No test exercises wall-clock arming for the new `links` command
- 🔵 Minor: New assertions iterate over hardcoded skill list with no guard against drift

### Correctness

**Summary**: The plan is logically coherent and the proposed fixes target real defects, but several correctness concerns remain. Most importantly, the `ACCELERATOR_PLAYWRIGHT_NO_OWNER=1` escape hatch is set in the skill's bash blocks but is not propagated to the sub-agent processes that actually invoke `run.sh` — the very PIDs that motivate the fix. There are also subtler concerns around env-var truthiness, race conditions in the new owner-exited test, and a test claim that doesn't exercise the production failure mode it advertises.

**Strengths**:
- Plan correctly identifies that existing daemon tests opt out of the failing branch
- BLOCKING_OPS membership for `links` is correct
- Phase ordering respects cross-phase dependencies
- Atomic file-write semantics for `server-stopped.json` preserved
- test-design.sh superstring assertions correctly strengthened
- Phase 1 catches the visualise/SKILL.md call sites missed by the research

**Findings**:
- 🔴 Critical: Env var ACCELERATOR_PLAYWRIGHT_NO_OWNER does not propagate from spawning skill into sub-agent shells
- 🟡 Major: `-n` check treats `=0` and `=false` as 'disable'
- 🟡 Major: Owner-exited test relies on Node reaping the spawned `sleep` zombie before the next watcher tick
- 🟡 Major: `--owner-pid 0 ignores owner death` test never kills any process
- 🟡 Major: Integration test does not exercise production failure mode
- 🔵 Minor: `links` may be invoked before any navigate; output shape on about:blank not specified
- 🔵 Minor: Returning raw `getAttribute('href')` does not resolve relative URLs
- 🔵 Minor: Phase 4 `links` test relies on Phase 2's fix being landed first
- 🔵 Minor: No boundary case for missing run.sh in config-read-browser-executor.sh

### Security

**Summary**: The plan is mostly security-neutral and preserves core controls (auth-header origin allowlist, evaluate-payload allowlist, URL scrubbing, screenshot masking, secret-literal scrubber). However, it introduces two changes that broaden the trust surface: the `ACCELERATOR_PLAYWRIGHT_NO_OWNER` escape hatch extends daemon lifetime (and therefore the lifetime of cached auth context), and the new `links` command surfaces raw href values without re-asserting query-string scrubbing for sensitive URLs.

**Strengths**:
- Existing browser-locator prohibitions on evaluate/click/type preserved
- `links` implemented server-side rather than by widening the evaluate allowlist
- `links` added to BLOCKING_OPS so wall-clock arms
- Phase 3 mirrors documents-locator preload precedent exactly
- Phase 2 escape hatch is opt-in (defaults preserve current behaviour)

**Findings**:
- 🟡 Major: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` extends auth-bearing browser context lifetime to 30-min idle ceiling
- 🟡 Major: `links` returns raw hrefs (including query strings) without scrubbing
- 🔵 Minor: Escape hatch trigger is any non-empty value
- 🔵 Minor: `links` not subject to any per-origin allowlist or auth-header scrub
- 🔵 Suggestion: Preloaded absolute path discloses plugin install location into agent transcripts
- 🔵 Suggestion: Plan does not require Step 12 (daemon-stop) to run on agent-failure paths

### Documentation

**Summary**: The plan does a strong job documenting the new preloaded-skill SKILL.md by mirroring the paths/SKILL.md precedent exactly. However, it has a significant gap: the new `links` command is not documented in PROTOCOL.md, and the new `ACCELERATOR_PLAYWRIGHT_NO_OWNER` env var has no documented home. The agent-body rewrites in Phase 3 are also under-specified — particularly for browser-analyser.md, which has a richer Tools section.

**Strengths**:
- browser-executor/SKILL.md mirrors paths/SKILL.md exactly, preserving the maintainer comment
- Cross-turn persistence rationale block focuses on WHY rather than WHAT
- Search Strategy rewrite captures the audience shift well
- 'Routes come from links' guideline directly addresses fabrication
- References section provides comprehensive precedent citations

**Findings**:
- 🟡 Major: PROTOCOL.md not updated for new `links` command
- 🟡 Major: New `ACCELERATOR_PLAYWRIGHT_NO_OWNER` env var has no documented home
- 🔵 Minor: browser-analyser.md body rewrite is under-specified
- 🔵 Minor: CHANGELOG.md not updated for any of the four fixes
- 🔵 Minor: Cross-turn persistence rationale wording deferred to implementation
- 🔵 Minor: Step 8 prose update is too terse for a non-obvious contract
- 🔵 Minor: Analyser-side `links` documentation rationale is thin

### Compatibility

**Summary**: The plan is largely additive and compatibility-safe: the new `links` command is a pure addition to the executor protocol, and the env-var escape hatch and preloaded skill follow established codebase precedents. However, there are real compatibility risks around the dependency on Claude Code subagent `skills:` preload semantics, the dependency on users having run migration 0004 (Phase 1 removes the bare-key call sites unconditionally), and an undocumented contract between the existing `--owner-pid 0` test path and the new env-var escape hatch.

**Strengths**:
- `links` is additive behind existing PROTOCOL=1 with graceful degradation for unknown commands
- Plan explicitly states 'no state-dir or daemon-protocol version bump'
- Plan reuses the documents-locator + accelerator:paths precedent
- New env var is opt-in; composes cleanly with existing ACCELERATOR_PLAYWRIGHT_* envelope

**Findings**:
- 🟡 Major: Phase 1 renames bare-key call sites without verifying users have run migration 0004
- 🟡 Major: Reliance on Claude Code subagent `skills:` preload mechanism is undocumented and version-fragile
- 🔵 Minor: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` truthiness contract is permissive
- 🔵 Minor: `links` command not documented in PROTOCOL.md
- 🔵 Minor: Visualise SKILL.md assertion lives outside test-design.sh scope

### Usability

**Summary**: The plan correctly mirrors a known-good precedent and is broadly ergonomic for LLM consumers and skill authors. However, several DX rough edges deserve attention: the `<run.sh>` angle-bracket placeholder diverges from the existing `{key}` convention, the `ACCELERATOR_PLAYWRIGHT_NO_OWNER` env var uses a negative-polarity name with double-negative confusion, and the cross-turn persistence rationale lives in inventory-design/SKILL.md rather than co-located with the toggle's implementation.

**Strengths**:
- Reusing documents-locator/paths precedent gives skill authors a clear template
- Maintainer comment about `user-invocable: false` is preserved verbatim
- Renaming call sites removes a visible 'warning: unknown key' message
- New `links` command with 'Routes come from links' guideline closes fabrication
- Plan pins contract with test-design.sh assertion

**Findings**:
- 🟡 Major: Angle-bracket `<run.sh>` placeholder diverges from existing `{key}` convention and is ambiguous
- 🟡 Major: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` is a negative-polarity boolean
- 🟡 Major: Cross-turn persistence rationale lives in inventory-design/SKILL.md, not co-located with run.sh toggle
- 🔵 Minor: `role: 'link'` default obscures whether an anchor had an explicit role attribute
- 🔵 Minor: Adding `links` to browser-analyser 'for symmetry' adds cognitive load
- 🔵 Minor: Maintainer note duplication across paths and browser-executor
- 🔵 Suggestion: Daemon env-var surface has no documented index
- 🔵 Suggestion: Pattern for 'preloaded executor-path skill' never documented as a pattern
- 🔵 Suggestion: Five-site rename has no transitional discoverability

---

## Re-Review (Pass 2) — 2026-05-19T10:25:09Z

**Verdict:** REVISE

The revision substantially improves the plan. The critical defect from
pass 1 (env-var propagation to sub-agent shells) is fully eliminated
because the entire watcher mechanism is removed rather than worked
around. Most major findings are resolved or rendered obsolete by the
design change. The remaining issues are smaller in scope but include
several genuine bugs introduced by the new content: one test missed
during the watcher-removal sweep, two tests that don't actually verify
what they advertise, a documentation step that's not concrete enough,
and a residual security-relevant data-leak in the `links` response.

### Previously Identified Issues

#### Critical (pass 1)

- 🔴 **Correctness**: Env var ACCELERATOR_PLAYWRIGHT_NO_OWNER does not propagate from spawning skill into sub-agent shells — **RESOLVED** (watcher removed entirely; env-var contract no longer exists)

#### Major (pass 1)

- 🟡 **Architecture**: Cross-turn daemon persistence is enforced by prose, not by mechanism — **RESOLVED**
- 🟡 **Code Quality**: Hardcoded executor path duplicates source-of-truth — **PARTIAL** (drift now detectable via `test -x`; two copies remain)
- 🟡 **Code Quality**: `links` snippet conflates default anchor role with explicit ARIA role — **RESOLVED**
- 🟡 **Code Quality**: `textContent.trim()` collapses multi-line link text inconsistently — **RESOLVED**
- 🟡 **Test Coverage**: New daemon owner-exited test skips the Playwright-availability guard — **OBSOLETE** (test removed with watcher)
- 🟡 **Test Coverage**: Env-var integration test is prose-only — **OBSOLETE** (test removed with watcher)
- 🟡 **Correctness**: `-n` check treats `=0` and `=false` as 'disable' — **OBSOLETE**
- 🟡 **Correctness**: Owner-exited test relies on Node reaping spawned `sleep` zombie — **OBSOLETE**
- 🟡 **Correctness**: `--owner-pid 0 ignores owner death` test never kills any process — **OBSOLETE**
- 🟡 **Correctness**: Integration test does not exercise production failure mode — **OBSOLETE** (replaced; new launcher-shell-exit test has its own concerns — see new findings)
- 🟡 **Security**: ACCELERATOR_PLAYWRIGHT_NO_OWNER extends auth-bearing browser context lifetime — **PARTIAL** (env var gone; IDLE_MS now 10 min so auth-context lifetime narrowed 3x; user explicitly chose single IDLE_MS for all cases)
- 🟡 **Security**: `links` returns raw hrefs (including query strings) without scrubbing — **PARTIAL** (pathname added; raw `href` and `resolved` still present in response — see new finding)
- 🟡 **Documentation**: PROTOCOL.md not updated for new `links` command — **RESOLVED** (Phase 5 added; concreteness concern in new findings)
- 🟡 **Documentation**: New env var has no documented home — **RESOLVED** (Phase 5 Environment Variables section)
- 🟡 **Compatibility**: Phase 1 renames bare-key call sites without verifying migration 0004 — **PARTIAL** (migration-aware warning added; but warning targets wrong audience — see new finding)
- 🟡 **Compatibility**: Reliance on Claude Code subagent `skills:` preload mechanism is undocumented and version-fragile — **STILL PRESENT**
- 🟡 **Usability**: Angle-bracket `<run.sh>` placeholder diverges from `{key}` convention — **RESOLVED** (now `{browser-executor-script}`, matches resolver key)
- 🟡 **Usability**: `ACCELERATOR_PLAYWRIGHT_NO_OWNER` is a negative-polarity boolean — **OBSOLETE**
- 🟡 **Usability**: Cross-turn persistence rationale lives in inventory-design/SKILL.md — **OBSOLETE**

#### Minor and suggestion (pass 1)

All minors and suggestions from pass 1 are either resolved, rendered obsolete by the watcher removal, or accepted as deliberate design decisions (notably: browser-executor not folded into `paths`; Step 8 not restructured). Full mapping omitted for brevity; see per-lens results below.

### New Issues Introduced

#### Major (new in pass 2)

- 🟡 **Compatibility**: `test-run.js:68` still passes `--owner-pid 0` after Phase 2 removes the flag
  **Location**: Phase 2
  The plan enumerates `daemon.test.js:77,99,120,141,164` and the run.js header comment, but misses `skills/design/inventory-design/scripts/playwright/test-run.js:68`. Also stale header comments at `run.js:4` and `daemon.js:2` reference the removed flag. The `test-design.sh` watcher-removed grep only checks three files.
- 🟡 **Compatibility**: Subagent `skills:` preload mechanism still version-fragile
  **Location**: Phase 3
  Carryover from pass 1. No minimum Claude Code version pinned, no runtime guard, no integration test. The whole Phase-3 fix could silently regress on a Claude Code upgrade.
- 🟡 **Correctness**: Wall-clock test for `links` will fire on prerequisite `navigate` call
  **Location**: Phase 4 Step 3
  Test sets `WALL_CLOCK_MS=1` then sends `navigate` (which is also in BLOCKING_OPS). The 1ms timer fires during navigate and the daemon shuts down before the `links` call. The assertion may match for the wrong reason; a regression dropping `'links'` from BLOCKING_OPS would not be caught.
- 🟡 **Correctness**: Migration-aware warning never fires for its intended audience
  **Location**: Phase 1 Step 3
  Stated purpose: help users who upgraded the plugin before running `/accelerator:migrate`. But after Phase 1 Step 2, no plugin code calls the bare key, so the warning never triggers for these users. Pre-migration users whose `.accelerator/config.md` still has `paths.design_inventories` silently fall back to the canonical default (losing their custom path) with NO warning.
- 🟡 **Test Coverage**: Launcher-shell-exit survival test does not prove the original production failure mode is fixed
  **Location**: Phase 2 Step 5
  Test waits 2 seconds and asserts `kill -0` succeeds. Under the OLD code, watcher default `OWNER_POLL_MS=60000` so the first tick wouldn't fire within 2s either — the test passes on both old and new code. Real regression guard is the source-level grep in `test-design.sh`; this runtime test is just a smoke test.
- 🟡 **Security**: `links` response still includes raw `href` and `resolved` (with query strings)
  **Location**: Phase 4 Step 2
  Locator instructed to use `pathname`, but `href` and `resolved` are physically present in every response. Auth tokens, OAuth codes, signed URLs in query strings leak into agent transcripts even though the final `inventory.md` is scrubbed.
- 🟡 **Documentation**: Phase 5 PROTOCOL.md instructions are not concrete enough to write from
  **Location**: Phase 5 Step 1
  Plan says "matching the shape of the existing entries" but doesn't inline the Request JSON block, Success response JSON envelope, or the Error codes table for `links`. Asymmetric with Phase 4 (which inlines daemon code) and Phase 5 Step 3 (which inlines CHANGELOG entries).

#### Minor (new in pass 2)

- 🔵 **Correctness**: `same_origin` semantics for `file://` URLs rely on Chromium-specific opaque-origin behaviour
  **Location**: Phase 4 Step 1
  Fixture uses `file://` URLs. `location.origin === "null"` for opaque origins in Chromium happens to make the test pass, but `mailto:` is also opaque-origin and would be marked `same_origin: true` (semantically wrong). Production runs on HTTP origins behave differently.
- 🔵 **Correctness**: IDLE_MS test pins source text, not runtime behaviour
  **Location**: Phase 2 Step 1
  Regex over source code; benign refactors (extract constant, use `??`) break the test without changing behaviour.
- 🔵 **Correctness**: `links` envelope shape on `about:blank` still unspecified
  **Location**: Phase 4 Step 2
  No test asserts what `links` returns before any `navigate`. Caller could interpret `links: []` as "no routes" rather than "no page loaded".
- 🔵 **Correctness**: `javascript:` and `data:` hrefs surface meaningless `pathname` values
  **Location**: Phase 4 Step 2
  Fixture doesn't include these. Locator instructed to use `pathname` but a `javascript:void(0)` href yields `pathname === "void(0)"`.
- 🔵 **Correctness**: Orphan daemon can survive up to IDLE_MS (10 min) after launcher death
  **Location**: Phase 2 overall
  Deliberate trade-off and documented; flagged for visibility.
- 🔵 **Code Quality**: IDLE_MS source-string pin is brittle (also flagged under correctness)
- 🔵 **Code Quality**: Maintainer note duplicated verbatim across `paths` and `browser-executor` SKILL.md
  **Location**: Phase 3 Step 3
  Still present from pass 1; not addressed.
- 🔵 **Code Quality**: Tools-section prose for the placeholder rule is verbose
  **Location**: Phase 3 Step 4
  ~5 sentences explaining the curly-brace convention; could be tightened to one sentence per agent body.
- 🔵 **Test Coverage**: Locator's `links`-consumption contract (use pathname, filter same_origin) verified only by prose-string-presence checks
  **Location**: Phase 4 Step 6
  Acceptable given the limits of testing markdown agent bodies; document the gap explicitly.
- 🔵 **Test Coverage**: No test asserts `run.js` behaviour when caller still passes `--owner-pid` after Phase 2 removes the parser
  **Location**: Phase 2 Step 3
  Silent-ignore could mask a stale external caller.
- 🔵 **Documentation**: Env-var table content named but not inlined
  **Location**: Phase 5 Step 1
  Same shape concern as the `links` PROTOCOL.md entry: shape described but not inlined.
- 🔵 **Documentation**: All defect fixes routed to Added/Changed/Removed; no `Fixed` bucket
  **Location**: Phase 5 Step 3
  CHANGELOG existing convention has `Fixed` heading; user-perceived "no more X" framing is more discoverable than mechanism-first.
- 🔵 **Documentation**: Placeholder convention explanation only in one agent body
  **Location**: Phase 3 Step 4
  Browser-locator body explains; browser-analyser body said to "add the same instruction" without specifying that the explanatory prose appears there too.
- 🔵 **Security**: Step 12 cleanup-on-error contract still not required
  **Location**: Phase 2 overall
  Carryover; severity reduced from pass 1 because IDLE_MS is now 10 min.
- 🔵 **Compatibility**: CHANGELOG `--owner-pid` removal not flagged under `### Breaking`
  **Location**: Phase 5 Step 3
  Existing CHANGELOG uses `### Breaking` heading; the removal entry should sit under it.
- 🔵 **Compatibility**: Env-var section in PROTOCOL.md is not pinned by a test assertion
  **Location**: Phase 5 Step 2
  The new in-sync assertion covers commands but not env vars. Future env vars could silently miss PROTOCOL.md.

#### Suggestions (new in pass 2)

- 🔵 **Security**: PROTOCOL.md should document the auth-context lifetime rationale for IDLE_MS
- 🔵 **Architecture**: Idle timer is now the sole architectural ceiling on cross-turn auth-bearing context lifetime — defence-in-depth `MAX_LIFETIME_MS` watchdog could be a future follow-up
- 🔵 **Usability**: Pattern for "preloaded plugin-internal resolver skill" still not documented (two examples now, no ADR / README)

### Assessment

**The plan is much closer to ready than pass 1.** The watcher removal was the right call and cleanly resolves the critical defect plus ~8 majors in one move. The new `{browser-executor-script}` placeholder, the migration-aware warning, the enriched `links` shape, the new Phase 5, and the watcher-removed source-level assertions all address pass-1 concerns substantively.

However, **REVISE is still the right verdict** because seven new major issues appeared in pass 2:

1. **Three new bugs in the revision's own content** (correctness/compatibility): the missed `test-run.js` site, the broken wall-clock test, the audience-mismatched migration warning. These are concrete defects in the new edits, not interpretive disagreements.
2. **Two test-quality issues** where the test doesn't verify what it claims: the launcher-shell-exit test (would pass on old code too) and the wall-clock test (fires on navigate, not links).
3. **One residual data leak**: `links` still returns `href` and `resolved` alongside `pathname`. The locator is instructed to use `pathname`, but the sensitive fields are physically present and will leak into transcripts.
4. **One carryover**: Claude Code subagent `skills:` preload version-fragility — unaddressed in both passes.
5. **One under-specification**: Phase 5 PROTOCOL.md instructions are abstract where every other phase inlines literal content.

Most of these are addressable with focused edits rather than another restructure. After a third pass that:
- Extends the watcher-removal sweep to `test-run.js`, `run.js:4`, `daemon.js:2`, and the `test-design.sh` assertion
- Drops `href` and `resolved` from the `links` response (or moves them behind an opt-in flag)
- Fixes the wall-clock test to actually exercise `links` (or replaces it with a daemon-status probe of BLOCKING_OPS)
- Reframes the migration-aware warning to actually help pre-migration users (detect legacy key in user config, not in the call site)
- Inlines the PROTOCOL.md `### links` and Environment Variables content
- Tightens the launcher-shell-exit test (lower OWNER_POLL_MS in the test env so it would have failed on old code)
- Adds a minimum Claude Code version pin OR a runtime guard in the agent body for missing `## Browser Executor` block

…the plan should reach APPROVE. The remaining minors and suggestions can be accepted or deferred without blocking.

---

## Re-Review (Pass 3) — 2026-05-19T11:00:17Z

**Verdict:** REVISE

The pass-3 revision substantively addresses every pass-2 major. The
`links` response no longer leaks raw URLs (`href` and `resolved`
dropped); the watcher-removal sweep now covers `test-run.js:68`,
`daemon.js:2`, and `run.js:4` with a repo-wide grep assertion; the
migration-aware warning now fires for its intended audience by probing
the user's config for the legacy alias; PROTOCOL.md is inlined verbatim
with full `### links` and `## Environment Variables` content; the broken
wall-clock test is dropped in favour of a source-level `BLOCKING_OPS`
membership assertion; a Preload guard mitigates (though does not fully
resolve) the subagent-skills-preload version fragility; the launcher-
shell-exit test is honestly relabelled as a smoke test. The plan is in
substantially better shape than pass 2.

Four new issues kept the verdict at REVISE — only one is a strict
blocker:

1. **Self-collision** (concrete bug): the broadened `test-design.sh`
   watcher-removal grep matches the literal `owner-exited` substring
   inside the smoke test's own assertion message in `test-run.sh`, so
   the assertion will fail the moment Phase 2 lands. 1-line fix.
2. **CHANGELOG `### Removed` → `#### Breaking` nesting** doesn't match
   the existing CHANGELOG.md convention (which uses `### Breaking` as a
   peer heading or inline `**BREAKING**:` prefix). The
   Success-Criteria `grep -E '^### (Fixed|Breaking)'` would also be
   inconsistent with the proposed structure.
3. **Preload guard relies on LLM self-introspection** — useful
   defence-in-depth but not a strong guarantee. An honest framing in
   the plan (downgrade the strength claim) would resolve this.
4. **No positive Claude Code version-compatibility signal** — the
   Preload guard tells users *that* something is wrong but not *which
   version was verified to work*. A README/manifest entry would close
   this.

Several minors were also surfaced; most are accepted-by-design or
deferrable, but two have concrete cost: the migration-warning probe
forks a subprocess on every canonical-key resolution (consider a cheap
presence-check gate); and the same-origin computation returns `true`
for opaque-origin schemes (`mailto:`, `javascript:`, `data:`) on
opaque-origin pages, which combined with the locator's "filter to
same_origin: true" rule is a (low-probability) XSS injection path on
file:// pages.

### Previously Identified Issues (from pass 2)

#### Major (pass 2)

- 🟡 **Compatibility**: `test-run.js:68` still passes `--owner-pid 0` — **RESOLVED** (sweep extended)
- 🟡 **Compatibility**: Subagent `skills:` preload version-fragility — **PARTIAL** (Preload guard mitigates loud-failure side; no positive version signal — see new major below)
- 🟡 **Correctness**: Wall-clock test for `links` fires on prerequisite navigate — **RESOLVED** (runtime test dropped, source-level BLOCKING_OPS assertion added)
- 🟡 **Correctness**: Migration-aware warning never fires for intended audience — **RESOLVED** (probe added; covers legacy-in-config case)
- 🟡 **Test Coverage**: Launcher-shell-exit test does not prove production failure mode — **ACKNOWLEDGED** (relabelled smoke test; source-level grep designated as the real regression guard)
- 🟡 **Security**: `links` response includes raw `href`/`resolved` — **RESOLVED** (fields dropped; negative fixture assertions added)
- 🟡 **Documentation**: PROTOCOL.md content not concrete — **RESOLVED** (request/response JSON, field table, error-code table, env-var table inlined verbatim)

#### Minor (pass 2) — selected highlights

- 🔵 same_origin file:// quirks — **DOCUMENTED** in PROTOCOL.md Notes (security new-finding still open — see below)
- 🔵 IDLE_MS source-text test brittleness — **STILL PRESENT** (deliberate choice; comment added)
- 🔵 about:blank links shape — **PARTIAL** (test added but tests post-navigate, not pre-navigate path)
- 🔵 javascript: href handling — **DOCUMENTED** in PROTOCOL.md; still no fixture coverage
- 🔵 CHANGELOG `--owner-pid` not under Breaking — **PARTIAL** (now under `#### Breaking` but the nesting doesn't match existing convention — see new major)
- 🔵 Env-var section not pinned — **RESOLVED** (test-design.sh asserts every daemon.js env var appears in PROTOCOL.md)
- 🔵 browser-analyser preload-guard prose duplication — **RESOLVED** (analyser explicitly told to mirror)
- 🔵 Step 12 cleanup-on-error contract — **STILL PRESENT** (severity reduced because IDLE_MS narrowed to 10 min)
- 🔵 Maintainer-note duplication — **STILL PRESENT** (suggestion-level)
- 🔵 Tools-section prose verbosity — **STILL PRESENT** (acknowledged in pass-3 edits as not tightened)

### New Issues Introduced

#### Major (new in pass 3)

- 🟡 **Correctness / Test Coverage**: Watcher-removal grep matches its own smoke test
  **Location**: Phase 2 Step 5 (smoke test) + Phase 2 Step 6 (test-design.sh assertion)
  The repo-wide grep `grep -rnE 'ownerPid|owner-pid|owner-exited|OWNER_POLL_MS'` scans the entire `playwright/` tree, but the new smoke test in `test-run.sh` contains the literal string `owner-exited` inside the assertion message `"daemon stopped with reason daemon-stop (not owner-exited)"`. The assertion `assert_exit_code "no watcher references" 1 grep ...` expects exit 1 (no matches) but grep will return 0 (match found). The test-design suite fails on first run.
  **Fix**: Either exclude test files from the grep (`--include='*.js' --include='*.sh' --exclude='test-run.sh'`), tighten the pattern to drop the generic `owner-exited` alternate and keep the identifier-only forms, or reword the smoke test's assertion message to drop the literal token.
- 🟡 **Compatibility**: CHANGELOG `### Removed` → `#### Breaking` does not match existing convention
  **Location**: Phase 5 Step 3
  Existing CHANGELOG.md uses `### Breaking` as a peer top-level subsection (already present in `[Unreleased]` for an unrelated change) or inline `- **BREAKING**:` prefixes within `### Changed`. The plan's `### Removed` containing a nested `#### Breaking` is a new pattern. The Success Criteria assertion `grep -E '^### (Fixed|Breaking)'` would match the existing `### Breaking` heading rather than the new `#### Breaking`, giving a false-positive sense of coverage.
  **Fix**: Either (a) promote the watcher-removal entry into the existing `[Unreleased] ### Breaking` section, or (b) drop the `#### Breaking` heading and use inline `- **BREAKING**:` within `### Removed`. Update the success-criteria grep accordingly.
- 🟡 **Correctness**: Preload guard relies on LLM self-introspection
  **Location**: Phase 3 Step 4
  The guard asks the agent to verify a `## Browser Executor` block is in its own context before acting. LLMs perform context self-introspection unreliably; the very failure case the guard targets (preload mechanism broken) is the case where the model is most likely to hallucinate the block being present or skip the check.
  **Fix**: Either honestly downgrade the plan's claim ("best-effort defence-in-depth, not a hard guarantee") or move the guard into the executor (`run.sh` fails loudly when invoked without an expected environment signal that only the preloaded skill establishes). The latter is a mechanical guard rather than prompt-engineered.
- 🟡 **Compatibility**: Preload guard gives no positive version-compatibility signal
  **Location**: Phase 3 Step 4
  When the guard fires, the user is told to "verify their Claude Code version" but no version is named — the plugin records no known-good baseline.
  **Fix**: Add a verified-good Claude Code version to the README (or plugin manifest) at plan-implementation time. Update the guard's error to reference the documented baseline so users have a concrete version to compare against.

#### Minor (new in pass 3)

- 🔵 **Correctness / Security**: `same_origin: true` returned for opaque-origin schemes on opaque-origin pages
  Mailto, `javascript:`, `data:` anchors on `file://` or `about:blank` pages return `same_origin: true` because both origins are the string `"null"`. Locator's "filter same_origin: true → navigate" rule combined with this becomes a (low-prob) XSS injection path on file:// fixtures.
  **Fix**: Tighten same-origin to `u.origin === pageOrigin && u.origin !== 'null'`, OR add a server-side scheme denylist for `javascript`/`data`/`vbscript`/`blob`.
- 🔵 **Architecture**: Migration-aware warning forks subprocess on every canonical-key resolution
  **Fix**: Gate the probe behind a cheap `grep -q 'paths\.\(design_inventories\|design_gaps\)' "$CONFIG_PATH"` check before forking config-read-value.sh.
- 🔵 **Test Coverage**: about:blank fixture tests navigate-to-about:blank, not links-before-any-navigate
  **Fix**: Add a third assertion block that calls `links` after a fresh `daemon-stop` with no intervening navigate.
- 🔵 **Test Coverage**: Negative `"resolved"` assertion uses substring match
  Could false-positive on future fixture text containing the word "resolved", or false-negative if a regression renames the field.
- 🔵 **Test Coverage**: Migration-warning matrix omits the cross-cell case (one legacy set, the other queried)
  **Fix**: Add an assertion that with only `paths.design_inventories` in config, querying `research_design_gaps` is silent.
- 🔵 **Code Quality**: `links` no-href rationale duplicated verbatim across 4 locations (daemon.js comment, PROTOCOL.md, CHANGELOG.md, browser-locator.md)
  **Fix**: Pick PROTOCOL.md as canonical; other 3 sites reference it.
- 🔵 **Code Quality**: Agent-body Preload guard prose ~10 lines, duplicated verbatim across both agents
  **Fix**: Compress to one sentence per guard rule; drop the documents-locator-precedent parenthetical.
- 🔵 **Code Quality**: Migration-aware warning script has two warning paths plus subprocess probe — borderline complexity
  **Fix**: Extract a `warn_legacy_key()` helper, OR accept and document the two-audiences design.
- 🔵 **Documentation**: Preload guard error message addresses end-user but mentions `skills:` frontmatter they cannot inspect
  **Fix**: Split into user-facing message (report to maintainer with Claude Code version) and maintainer-facing diagnostic.
- 🔵 **Documentation**: PROTOCOL.md success-response example shows only 2 entries, but field table notes apply to mailto/fragment/query anchors (which the fixture covers)
  **Fix**: Add 2-3 more example rows showing mailto, fragment-only, query-only.
- 🔵 **Documentation**: Env-var table omits `ACCELERATOR_PLAYWRIGHT_SKIP_REAL_INSTALL`; WALL_CLOCK_MS clamp warning unmentioned
  **Fix**: Add row for SKIP_REAL_INSTALL; mention the "clamped to Xms" stderr warning in WALL_CLOCK_MS row.
- 🔵 **Security**: KEEP_STDIO env var newly documented but no warning that it bypasses URL/secret scrubbing in stdio
- 🔵 **Security**: Anchor `text` field still echoes anchor visible content; a page rendering `Click here: example.com/?token=secret` would surface `token=secret` in text
- 🔵 **Usability**: Preload guard error message mixes agent-directed and user-directed framing

### Assessment

The plan is now in mature shape. Of the four new majors:

- **Self-collision** is a concrete bug with a 1-line fix (the grep
  pattern or the test message). Must fix.
- **CHANGELOG convention** is a 5-minute fix once the existing
  `[Unreleased]` block is consulted.
- **Preload guard reliability** is a framing fix (downgrade the
  strength claim) plus an optional architectural improvement (move
  guard into `run.sh`). The framing fix is small; the architectural
  improvement is genuine follow-up.
- **No version-compatibility signal** is a small README/manifest
  addition that closes the pass-1 carryover.

If those four are addressed plus the same-origin opaque-origin fix
(low-prob XSS path), the plan should reach APPROVE. The remaining
minors and suggestions can be accepted, deferred, or addressed
opportunistically; none are blockers.

The plan demonstrates strong iteration. Pass 1 → Pass 2 resolved the
critical defect and ~8 majors. Pass 2 → Pass 3 resolved 7 of 7 pass-2
majors, with only the new self-collision being a strict regression.
The trajectory toward approve is clear.

---

## Re-Review (Pass 4) — 2026-05-19T12:02:17Z

**Verdict:** COMMENT — plan is acceptable but two major findings remain.

All four pass-3 majors are resolved cleanly: the self-collision is gone
(grep tightened to identifier-only forms with word boundaries), the
CHANGELOG now uses the existing peer `### Breaking` convention, the
Preload guard is honestly framed as best-effort defence-in-depth, and
the README baseline gives users a concrete version artefact to compare
against. The plan is below the 3-major REVISE threshold and could
proceed; the two remaining majors are addressable in focused edits.

### Previously Identified Issues (from pass 3)

#### Major (pass 3)

- 🟡 **Correctness/Test-Coverage**: Self-collision in watcher-removal grep — **RESOLVED** (`\bownerPid\b|--owner-pid|\bOWNER_POLL_MS\b`, identifier-only with word boundaries; `owner-exited` deliberately excluded with documented rationale)
- 🟡 **Compatibility**: CHANGELOG `### Removed → #### Breaking` mismatch — **RESOLVED** (peer `### Breaking` heading matches existing `[Unreleased]` convention; upgrade-sequence note added)
- 🟡 **Correctness**: Preload guard LLM self-introspection unreliable — **RESOLVED** (honestly framed as "best-effort defence-in-depth, not a hard guarantee"; README baseline is the mechanical companion)
- 🟡 **Compatibility**: No positive Claude Code version-compatibility signal — **RESOLVED** (Phase 5 Step 4 adds README baseline with literal sentinel)

#### Minor (pass 3) — selected highlights

- 🔵 Opaque-origin same_origin trap — **RESOLVED** (`u.origin === pageOrigin && u.origin !== 'null'` guard; fixture asserts no anchor reports same_origin: true on file://)
- 🔵 Migration-warning subprocess fork — **PARTIAL** (cheap-gate added; but uses `$PWD` — see new major below)
- 🔵 about:blank fixture tests post-navigate not pre-navigate — STILL PRESENT (deferred)
- 🔵 Maintainer-note duplication — STILL PRESENT (deferred)
- 🔵 Tools-section prose verbosity — STILL PRESENT (and grown, per pass-4 finding)
- 🔵 IDLE_MS source-text pin — STILL PRESENT (deliberate; comment justifies)

### New Issues Introduced

#### Major (new in pass 4)

- 🟡 **Correctness / Architecture**: Migration-warning cheap-gate uses `$PWD`-relative config paths
  **Location**: Phase 1 Step 3
  The cheap-gate's `grep -lF "$legacy" "$PWD/.accelerator/config.md" "$PWD/.accelerator/config.local.md"` uses literal `$PWD`, but `config-read-value.sh` resolves the config file via `find_repo_root` from `scripts/vcs-common.sh`. When `config-read-path.sh` is invoked from a subdirectory of the project root (a common case — skills are invoked from arbitrary CWDs), the gate looks at non-existent paths and silently skips the probe, so the pre-migration user's stale legacy override is silently ignored without the load-bearing warning. The Phase 1 promise that "this second case is the one that actually fires for users in production" is silently broken in any subdirectory invocation.
  **Fix**: Either (a) source `config-common.sh` and resolve the project root via `config_project_root` (or `find_repo_root` directly), then grep `$root/.accelerator/...`; or (b) drop the cheap-gate entirely and accept one extra subprocess on every `research_design_*` resolution (this is not a hot path).

- 🟡 **Test Coverage**: Same-origin positive branch (`same_origin: true`) never exercised
  **Location**: Phase 4 Step 1 (fixture test)
  The file:// fixture has an opaque page origin (`"null"`), so the new `u.origin === pageOrigin && u.origin !== 'null'` guard correctly makes every anchor report `same_origin: false`. The fixture asserts the negative case (`no anchor reports same_origin: true on a file:// page`), but no test exercises the positive branch — the load-bearing half of the security-critical computation that the locator's "filter same_origin: true" rule depends on. A mutation that hardcoded `sameOrigin = false`, dropped the assignment, or flipped the comparator polarity would pass every current assertion. The plan acknowledges this gap ("To exercise same_origin: true we would need an HTTP origin fixture; that is out of scope").
  **Fix**: Either (a) add a minimal HTTP fixture via a tiny `http.createServer` in `test-run.sh` so at least one anchor reports same_origin: true and the inverse assertion has bite; (b) document the gap explicitly in the "Known Coverage Gaps" section so reviewers see deliberate absence rather than vacuous coverage; or (c) extract the per-anchor mapping logic into a unit-testable module-level helper.

#### Minor (new in pass 4)

- 🔵 **Documentation**: Stale `### Removed` cross-reference at plan line 1283-1285
  PROTOCOL.md insertion says "see Removed section in CHANGELOG.md" but pass-4 dropped the Removed section in favour of `### Breaking`.
  **Fix**: Change to "see Breaking section in CHANGELOG.md".
- 🔵 **Documentation**: Phase 5 Manual Verification at plan line 1459-1461 lists "Added / Changed / Removed" as CHANGELOG headings
  Same stale reference — the plan no longer creates a Removed section.
  **Fix**: Update to "Breaking / Added / Changed / Fixed" to match the actual heading set.
- 🔵 **Compatibility**: Internal narrative at plan line 529 still cites the old grep pattern (`'ownerPid|owner-pid|owner-exited|OWNER_POLL_MS'`)
  Inconsistent with the actual assertion at line 556-557 (now word-bounded, no `owner-exited`).
  **Fix**: Update the narrative to cite the actual pattern.
- 🔵 **Documentation / Compatibility**: README baseline placeholder `vX.Y.Z` could ship unfilled
  The Phase 5 grep checks for the sentinel `browser-executor verified baseline` but not for the absence of the `vX.Y.Z` placeholder.
  **Fix**: Add a negative success-criterion: `grep -F 'vX.Y.Z' README.md` returns no match.
- 🔵 **Usability**: README baseline version format unspecified
  Implementer is told to record "the version" but `claude --version` output format isn't pinned. Users running it to compare may see a string format that doesn't match.
  **Fix**: Add a worked example showing the verbatim `claude --version` output.
- 🔵 **Code Quality**: Cheap-gate uses two-pipeline form (`grep -lF | grep -q .`) where a single `grep -qF` would suffice
  Cosmetic; either change to one pipeline or add a comment explaining the defensive shape.
- 🔵 **Security**: KEEP_STDIO documented without scrubbing-bypass warning (carryover from pass 3)
- 🔵 **Security**: Anchor `text` field still echoes visible URL content (carryover from pass 3)
- 🔵 **Security**: Plugin install path disclosure in preloaded block (carryover from pass 3)
- 🔵 **Test Coverage**: Migration-warning cheap-gate substring-matches `design_inventories` against `research_design_inventories` (false-positive on the gate, but correctness preserved by the inner check; gate's performance claim weakens)
- 🔵 **Usability**: Tools-section prose has grown to ~4 paragraphs before the first command example
  Pass-3 verbose-prose carryover, now amplified by the Preload guard text.
- 🔵 **Code Quality**: `links` no-href rationale still duplicated across 4 locations (daemon.js, PROTOCOL.md, agent body, CHANGELOG) — carryover from pass 3

### Assessment

The plan is in approve-able shape per the configured threshold (2 majors
< 3-major REVISE bar). The two pass-4 majors are both concrete defects
with small, well-scoped fixes:

- **Cheap-gate `$PWD` fix**: either source `config-common.sh` to use
  `find_repo_root`, or drop the cheap-gate and accept the subprocess
  cost. The simpler "drop it" option preserves correctness and removes
  a class of CWD-dependent edge cases at the cost of one extra
  subprocess per `research_design_*` resolution (not a hot path).
- **Positive same_origin branch coverage**: explicitly document the
  gap in "Known Coverage Gaps" if an HTTP fixture is out of scope, OR
  add an `http.createServer`-based fixture in test-run.sh.

The three small documentation defects (stale CHANGELOG cross-reference,
stale Manual Verification listing, stale grep-pattern narrative) are
trivial one-line fixes that should be batched with the two majors.

If those five are addressed, the plan reaches APPROVE. The remaining
minors and suggestions (KEEP_STDIO warning, anchor text scrubbing,
rationale-duplication consolidation, Tools-section trimming, etc.) are
all deferrable to follow-up work without blocking implementation.

---

## Re-Review (Pass 5) — 2026-05-19T12:34:06Z

**Verdict:** APPROVE

All five pass-4 issues marked for fixing landed cleanly across all 8
lenses. The plan is ready to implement.

### Previously Identified Issues (from pass 4)

#### Major (pass 4)

- 🟡 **Correctness / Architecture**: Migration-warning cheap-gate `$PWD`-relative paths — **RESOLVED** (sourced `vcs-common.sh`, switched to `find_repo_root` resolution; matches downstream `config-read-value.sh` semantics; correctness verified end-to-end including outside-repo fall-through)
- 🟡 **Test Coverage**: Same_origin positive branch not exercised — **RESOLVED** (documented in Known Coverage Gaps with cost-tradeoff rationale; Manual Verification step 6 covers it end-to-end on the HTTP-origin visualiser; follow-up path identified — acceptable test-coverage practice per the lens)

#### Minor (pass 4)

- 🔵 Stale "Removed section" cross-reference in PROTOCOL.md — **RESOLVED**
- 🔵 Phase 5 Manual Verification listing "Added/Changed/Removed" — **RESOLVED**
- 🔵 Internal narrative citing old grep pattern — **RESOLVED**
- 🔵 README `vX.Y.Z` placeholder shipping risk — **RESOLVED** (new negative grep assertion)
- 🔵 Cheap-gate two-pipeline form — **RESOLVED** (consolidated to single `grep -qF`)

### New Issues Introduced

#### Major

None.

#### Minor

- 🔵 **Documentation**: Phase 2 Step 6 comment cites "PROTOCOL.md cross-references" as a false-positive source for `owner-exited`, but PROTOCOL.md does not contain such references. Test assertion messages alone justify the exclusion.
  **Fix**: Drop "or PROTOCOL.md cross-references" from the comment, OR add an `owner-exited` mention to PROTOCOL.md's removed-mechanisms note.

- 🔵 **Documentation**: Phase 5 Step 1 instruction quotes "Stability commitment" (lowercase) but the actual PROTOCOL.md heading is `## Stability Commitment` (title case at line 496).
  **Fix**: Update the plan's instruction to use the verbatim heading `## Stability Commitment` for case-sensitive search reliability.

#### Suggestions

- 🔵 **Usability**: Plan sources `vcs-common.sh` directly instead of using the existing `config_project_root` helper in `config-common.sh` (which other config-* scripts use as the convention). The two routes are semantically equivalent for this use case (`config_project_root` wraps `find_repo_root`), but bypassing `config-common.sh` makes `config-read-path.sh` the lone config-* script that reaches around the established abstraction.
  **Note**: Defensible because `config-common.sh` brings in `config_assert_no_legacy_layout` (the original VCS-cost concern). If `config_project_root` could be made callable without paying that cost, the convention would unify; today the direct-source approach is the pragmatic choice. Either keep as-is with the existing inline comment justifying the divergence, or document the deliberate departure more explicitly.

### Assessment

The plan is ready to implement. All four passes of substantive findings
are addressed:

- **Pass 1 critical** (env-var propagation to sub-agents): eliminated by removing the watcher entirely
- **Pass 2 majors** (×7, including test-run.js missed, wall-clock test broken, migration audience mismatch, links data-leak, PROTOCOL.md not concrete): all resolved
- **Pass 3 majors** (×4, including self-collision, CHANGELOG nesting, preload-guard reliability framing, version baseline): all resolved
- **Pass 4 majors** (×2, including cheap-gate `$PWD` bug and same_origin coverage gap): both resolved

The remaining minor findings (Phase 2 Step 6 comment refinement, PROTOCOL.md
heading case, the vcs-common.sh sourcing convention) are all small
documentation refinements that don't block implementation. They can be
addressed opportunistically during the implementation pass or left as-is.

The plan's evolution across 5 review passes — from 1 critical + 19
majors initially to 0 criticals + 0 majors at approve — reflects strong
iterative discipline. The trajectory was steep and consistent: each
pass reduced the major count substantively, and each fix avoided
introducing regressions of comparable severity.

---

*Review complete. Ready to proceed to `/accelerator:implement-plan`.*
